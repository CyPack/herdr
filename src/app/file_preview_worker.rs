use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::thread::JoinHandle;

use sha2::{Digest, Sha256};
use tokio::sync::Notify;

use crate::fm::{highlight_text_preview, TextPreview, TextPreviewError};

#[derive(Debug, Clone, PartialEq, Eq)]
struct FilePreviewKey {
    files_generation: u32,
    path: PathBuf,
    preview_generation: u64,
    content_sha256: Option<[u8; 32]>,
    truncated: bool,
}

impl FilePreviewKey {
    #[cfg(test)]
    fn new(path: &Path, preview: &TextPreview) -> Self {
        Self::prepared(1, path, 1, preview)
    }

    fn pending(files_generation: u32, path: PathBuf, preview_generation: u64) -> Self {
        Self {
            files_generation,
            path,
            preview_generation,
            content_sha256: None,
            truncated: false,
        }
    }

    fn prepared(
        files_generation: u32,
        path: &Path,
        preview_generation: u64,
        preview: &TextPreview,
    ) -> Self {
        let digest = Sha256::digest(preview.content.as_bytes());
        Self {
            files_generation,
            path: path.to_path_buf(),
            preview_generation,
            content_sha256: Some(digest.into()),
            truncated: preview.truncated,
        }
    }

    fn is_pending(&self) -> bool {
        self.content_sha256.is_none()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FilePreviewSync {
    Unchanged,
    Started { generation: u64 },
    Stopped,
}

#[derive(Debug, Default)]
struct FilePreviewSlot {
    generation: u64,
    active: Option<FilePreviewKey>,
}

impl FilePreviewSlot {
    fn sync(&mut self, target: Option<FilePreviewKey>) -> FilePreviewSync {
        if self.active == target {
            return FilePreviewSync::Unchanged;
        }

        self.generation = self.generation.wrapping_add(1).max(1);
        self.active = target;
        if self.active.is_some() {
            FilePreviewSync::Started {
                generation: self.generation,
            }
        } else {
            FilePreviewSync::Stopped
        }
    }

    fn accepts(&self, generation: u64, key: &FilePreviewKey) -> bool {
        self.generation == generation && self.active.as_ref() == Some(key)
    }
}

#[derive(Debug)]
enum FilePreviewSource {
    Pending,
    Prepared(TextPreview),
}

#[derive(Debug)]
struct FilePreviewTarget {
    key: FilePreviewKey,
    source: FilePreviewSource,
}

impl FilePreviewTarget {
    fn pending(files_generation: u32, path: PathBuf, preview_generation: u64) -> Self {
        Self {
            key: FilePreviewKey::pending(files_generation, path, preview_generation),
            source: FilePreviewSource::Pending,
        }
    }

    fn prepared(
        files_generation: u32,
        path: PathBuf,
        preview_generation: u64,
        preview: TextPreview,
    ) -> Self {
        Self {
            key: FilePreviewKey::prepared(files_generation, &path, preview_generation, &preview),
            source: FilePreviewSource::Prepared(preview),
        }
    }
}

#[cfg(test)]
impl From<(PathBuf, TextPreview)> for FilePreviewTarget {
    fn from((path, preview): (PathBuf, TextPreview)) -> Self {
        Self::prepared(1, path, 1, preview)
    }
}

#[derive(Debug)]
struct FilePreviewRequest {
    generation: u64,
    key: FilePreviewKey,
    source: FilePreviewSource,
}

#[derive(Debug)]
struct FilePreviewResult {
    generation: u64,
    key: FilePreviewKey,
    prepared: Result<TextPreview, TextPreviewError>,
}

#[derive(Debug, Default)]
struct FilePreviewDrain {
    current: Option<FilePreviewResult>,
    disconnected: bool,
    report_disconnect: bool,
}

#[derive(Debug)]
struct FilePreviewWorkerState {
    pending: Option<FilePreviewRequest>,
    result: Option<FilePreviewResult>,
    alive: bool,
    closed: bool,
}

impl Default for FilePreviewWorkerState {
    fn default() -> Self {
        Self {
            pending: None,
            result: None,
            alive: true,
            closed: false,
        }
    }
}

type SharedWorkerState = Arc<(Mutex<FilePreviewWorkerState>, Condvar)>;

struct WorkerAliveGuard {
    shared: SharedWorkerState,
    wake: Arc<Notify>,
}

impl Drop for WorkerAliveGuard {
    fn drop(&mut self) {
        let (state, _) = &*self.shared;
        lock_state(state).alive = false;
        self.wake.notify_one();
    }
}

pub(super) struct FilePreviewWorker {
    slot: FilePreviewSlot,
    shared: SharedWorkerState,
    handle: Option<JoinHandle<()>>,
    disconnect_reported: bool,
}

impl FilePreviewWorker {
    pub(super) fn new(wake: Arc<Notify>) -> Self {
        Self::with_preview_processor(wake, process_preview)
    }

    fn with_preview_processor<F>(wake: Arc<Notify>, processor: F) -> Self
    where
        F: Fn(&Path, FilePreviewSource) -> Result<TextPreview, TextPreviewError> + Send + 'static,
    {
        let shared = Arc::new((
            Mutex::new(FilePreviewWorkerState::default()),
            Condvar::new(),
        ));
        let worker_shared = shared.clone();
        let handle = std::thread::spawn(move || {
            let _alive_guard = WorkerAliveGuard {
                shared: worker_shared.clone(),
                wake: wake.clone(),
            };
            while let Some(request) = take_next_request(&worker_shared) {
                let prepared = processor(&request.key.path, request.source);
                let result = FilePreviewResult {
                    generation: request.generation,
                    key: request.key,
                    prepared,
                };
                let (state, _) = &*worker_shared;
                let mut state = lock_state(state);
                if state.closed {
                    break;
                }
                state.result = Some(result);
                drop(state);
                wake.notify_one();
            }
        });

        Self {
            slot: FilePreviewSlot::default(),
            shared,
            handle: Some(handle),
            disconnect_reported: false,
        }
    }

    #[cfg(test)]
    fn with_processor<F>(wake: Arc<Notify>, processor: F) -> Self
    where
        F: Fn(&Path, &TextPreview) -> crate::fm::HighlightedTextPreview + Send + 'static,
    {
        Self::with_preview_processor(wake, move |path, source| {
            let mut preview = match source {
                FilePreviewSource::Pending => crate::fm::prepare_default_text_preview(path)?,
                FilePreviewSource::Prepared(preview) => preview,
            };
            preview.highlighted = Some(processor(path, &preview));
            Ok(preview)
        })
    }

    fn sync_target<T>(&mut self, target: Option<T>) -> FilePreviewSync
    where
        T: Into<FilePreviewTarget>,
    {
        let target = target.map(Into::into);
        let target_key = target.as_ref().map(|target| target.key.clone());
        let sync = self.slot.sync(target_key.clone());
        let (state, pending) = &*self.shared;
        let mut state = lock_state(state);
        match sync {
            FilePreviewSync::Started { generation } => {
                if let (Some(key), Some(target)) = (target_key, target) {
                    state.pending = Some(FilePreviewRequest {
                        generation,
                        key,
                        source: target.source,
                    });
                    crate::render_prof::event("fm.text_worker.submitted");
                    pending.notify_one();
                }
            }
            FilePreviewSync::Stopped => {
                state.pending = None;
                state.result = None;
            }
            FilePreviewSync::Unchanged => {}
        }
        sync
    }

    fn drain(&mut self) -> FilePreviewDrain {
        let (state, _) = &*self.shared;
        let mut state = lock_state(state);
        let result = state.result.take();
        let disconnected = !state.alive && !state.closed;
        let report_disconnect = disconnected && !self.disconnect_reported;
        self.disconnect_reported |= report_disconnect;
        drop(state);

        let current = result.and_then(|result| {
            if self.slot.accepts(result.generation, &result.key) {
                crate::render_prof::event("fm.text_worker.completed");
                Some(result)
            } else {
                crate::render_prof::event("fm.text_worker.rejected");
                None
            }
        });
        FilePreviewDrain {
            current,
            disconnected,
            report_disconnect,
        }
    }
}

#[cfg(test)]
type FilePreviewHighlightKey = FilePreviewKey;
#[cfg(test)]
type FilePreviewHighlightSync = FilePreviewSync;
#[cfg(test)]
type FilePreviewHighlightSlot = FilePreviewSlot;
#[cfg(test)]
type FilePreviewHighlightResult = FilePreviewResult;
#[cfg(test)]
type FilePreviewHighlightWorker = FilePreviewWorker;

impl Drop for FilePreviewWorker {
    fn drop(&mut self) {
        let (state, pending) = &*self.shared;
        let mut state = lock_state(state);
        state.closed = true;
        state.pending = None;
        state.result = None;
        drop(state);
        pending.notify_one();

        if let Some(handle) = self.handle.take() {
            // Preview preparation includes filesystem I/O. A stalled mount
            // must not turn App/server teardown into an unbounded join. The
            // closed flag prevents any late result from escaping this private
            // shared state; dropping an unfinished handle safely detaches it.
            if handle.is_finished() {
                let _ = handle.join();
            }
        }
    }
}

fn take_next_request(shared: &SharedWorkerState) -> Option<FilePreviewRequest> {
    let (state, pending) = &**shared;
    let mut state = lock_state(state);
    while state.pending.is_none() && !state.closed {
        state = match pending.wait(state) {
            Ok(state) => state,
            Err(poisoned) => poisoned.into_inner(),
        };
    }
    if state.closed {
        return None;
    }
    state.pending.take()
}

fn process_preview(
    path: &Path,
    source: FilePreviewSource,
) -> Result<TextPreview, TextPreviewError> {
    let mut preview = match source {
        FilePreviewSource::Pending => crate::fm::prepare_default_text_preview(path)?,
        FilePreviewSource::Prepared(preview) => preview,
    };
    preview.highlighted = Some(highlight_text_preview(path, &preview));
    Ok(preview)
}

fn lock_state(state: &Mutex<FilePreviewWorkerState>) -> MutexGuard<'_, FilePreviewWorkerState> {
    match state.lock() {
        Ok(state) => state,
        Err(poisoned) => poisoned.into_inner(),
    }
}

impl super::App {
    #[cfg(test)]
    pub(in crate::app) fn file_preview_worker_generation_for_test(&self) -> u64 {
        self.file_preview_worker.slot.generation
    }

    pub(crate) fn sync_file_preview_worker(&mut self) -> bool {
        let files_generation = (self.state.stage.surface_view()
            == crate::ui::surface_host::StageSurfaceView::NativeFiles)
            .then(|| self.state.stage.active_instance_generation())
            .flatten();
        let target = files_generation.and_then(|files_generation| {
            self.state.file_manager.as_ref().and_then(|file_manager| {
                let selected_path = file_manager.selected()?.path.clone();
                match &file_manager.preview {
                    crate::fm::FmPreview::File(crate::fm::FmFilePreview::PendingText {
                        source_path,
                        generation,
                    }) if source_path == &selected_path
                        && *generation == file_manager.preview_generation =>
                    {
                        Some(FilePreviewTarget::pending(
                            files_generation,
                            selected_path,
                            *generation,
                        ))
                    }
                    crate::fm::FmPreview::File(crate::fm::FmFilePreview::Text(preview))
                        if preview.source_path == selected_path
                            && preview.highlighted.is_none() =>
                    {
                        let mut source = preview.clone();
                        source.highlighted = None;
                        Some(FilePreviewTarget::prepared(
                            files_generation,
                            selected_path,
                            file_manager.preview_generation,
                            source,
                        ))
                    }
                    crate::fm::FmPreview::None
                    | crate::fm::FmPreview::Directory(_)
                    | crate::fm::FmPreview::File(_) => None,
                }
            })
        });

        let _ = self.file_preview_worker.sync_target(target);
        let drained = self.file_preview_worker.drain();
        let mut changed = false;
        if drained.report_disconnect {
            tracing::warn!("fm: text preview worker stopped; pending preview is unavailable");
        }
        if drained.disconnected {
            let pending = self.state.file_manager.as_ref().and_then(|file_manager| {
                match &file_manager.preview {
                    crate::fm::FmPreview::File(crate::fm::FmFilePreview::PendingText {
                        source_path,
                        generation,
                    }) => Some((source_path.clone(), *generation)),
                    crate::fm::FmPreview::None
                    | crate::fm::FmPreview::Directory(_)
                    | crate::fm::FmPreview::File(_) => None,
                }
            });
            if let Some((path, generation)) = pending {
                changed |= self
                    .state
                    .file_manager
                    .as_mut()
                    .is_some_and(|file_manager| {
                        file_manager.apply_prepared_text_preview(
                            &path,
                            generation,
                            Err(TextPreviewError::Io(std::io::ErrorKind::BrokenPipe)),
                        )
                    });
            }
        }
        let Some(result) = drained.current else {
            return changed;
        };
        if files_generation != Some(result.key.files_generation) {
            return changed;
        }
        let Some(file_manager) = self.state.file_manager.as_mut() else {
            return changed;
        };
        if result.key.is_pending() {
            return file_manager.apply_prepared_text_preview(
                &result.key.path,
                result.key.preview_generation,
                result.prepared,
            ) || changed;
        }
        let Ok(prepared) = result.prepared else {
            return changed;
        };
        let crate::fm::FmPreview::File(crate::fm::FmFilePreview::Text(preview)) =
            &mut file_manager.preview
        else {
            return changed;
        };
        if FilePreviewKey::prepared(
            result.key.files_generation,
            &result.key.path,
            file_manager.preview_generation,
            preview,
        ) != result.key
            || prepared.source_path != result.key.path
            || prepared.highlighted.is_none()
            || *preview == prepared
        {
            return changed;
        }
        *preview = prepared;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fm::{
        HighlightedTextPreview, PreviewTextLine, PreviewTextSpan, PreviewTextStyle, TextPreview,
    };
    use std::path::{Path, PathBuf};
    use std::sync::{mpsc, Arc, Mutex};
    use std::time::{Duration, Instant};
    use tokio::sync::Notify;

    struct TempDir {
        root: PathBuf,
    }

    impl TempDir {
        fn new(tag: &str) -> Self {
            static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let root = std::env::temp_dir().join(format!(
                "herdr-fm-highlight-{}-{tag}-{}",
                std::process::id(),
                COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            ));
            std::fs::create_dir_all(&root).expect("create highlight temp directory");
            Self { root }
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    fn test_app() -> crate::app::App {
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        crate::app::App::new(
            &crate::config::Config::default(),
            true,
            None,
            api_rx,
            crate::api::EventHub::default(),
        )
    }

    fn preview(content: &str) -> TextPreview {
        TextPreview {
            source_path: PathBuf::from("sample.rs"),
            content: content.to_owned(),
            truncated: false,
            highlighted: None,
        }
    }

    fn preview_for(path: &Path, content: &str) -> TextPreview {
        TextPreview {
            source_path: path.to_path_buf(),
            content: content.to_owned(),
            truncated: false,
            highlighted: None,
        }
    }

    fn highlighted(content: &str) -> HighlightedTextPreview {
        HighlightedTextPreview {
            lines: vec![PreviewTextLine {
                spans: vec![PreviewTextSpan {
                    content: content.to_owned(),
                    style: PreviewTextStyle::default(),
                }],
            }],
            syntax_name: None,
            truncated_bytes: false,
            truncated_lines: false,
        }
    }

    fn wait_for_current(worker: &mut FilePreviewHighlightWorker) -> FilePreviewHighlightResult {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let drained = worker.drain();
            assert!(!drained.disconnected, "worker disconnected before result");
            if let Some(current) = drained.current {
                return current;
            }
            assert!(Instant::now() < deadline, "timed out waiting for highlight");
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    // TP-B1.4-LIFECYCLE: unchanged content does not enqueue duplicate work,
    // while a content change advances generation and invalidates old results.
    #[test]
    fn highlight_slot_rebinds_and_rejects_stale_generation() {
        let first_key = FilePreviewHighlightKey::new(Path::new("sample.rs"), &preview("first"));
        let second_key = FilePreviewHighlightKey::new(Path::new("sample.rs"), &preview("second"));
        let mut slot = FilePreviewHighlightSlot::default();

        let first_generation = match slot.sync(Some(first_key.clone())) {
            FilePreviewHighlightSync::Started { generation } => generation,
            other => panic!("first target must start, got {other:?}"),
        };
        assert_eq!(
            slot.sync(Some(first_key.clone())),
            FilePreviewHighlightSync::Unchanged
        );

        let second_generation = match slot.sync(Some(second_key.clone())) {
            FilePreviewHighlightSync::Started { generation } => generation,
            other => panic!("changed content must restart, got {other:?}"),
        };

        assert_ne!(first_generation, second_generation);
        assert!(!slot.accepts(first_generation, &first_key));
        assert!(slot.accepts(second_generation, &second_key));
    }

    // TP-B1.4-LIFECYCLE: closing the FM invalidates the current authority even
    // if its background result arrives later.
    #[test]
    fn highlight_slot_close_rejects_prior_generation() {
        let key = FilePreviewHighlightKey::new(Path::new("sample.rs"), &preview("content"));
        let mut slot = FilePreviewHighlightSlot::default();
        let generation = match slot.sync(Some(key.clone())) {
            FilePreviewHighlightSync::Started { generation } => generation,
            other => panic!("target must start, got {other:?}"),
        };

        assert_eq!(slot.sync(None), FilePreviewHighlightSync::Stopped);
        assert!(!slot.accepts(generation, &key));
        assert_eq!(slot.sync(None), FilePreviewHighlightSync::Unchanged);
    }

    #[test]
    fn pending_slot_rejects_same_path_and_preview_generation_after_files_reopen() {
        let path = PathBuf::from("same.rs");
        let first = FilePreviewKey::pending(41, path.clone(), 9);
        let reopened = FilePreviewKey::pending(42, path, 9);
        let mut slot = FilePreviewSlot::default();
        let first_generation = match slot.sync(Some(first.clone())) {
            FilePreviewSync::Started { generation } => generation,
            other => panic!("first pending target must start, got {other:?}"),
        };
        let reopened_generation = match slot.sync(Some(reopened.clone())) {
            FilePreviewSync::Started { generation } => generation,
            other => panic!("reopened pending target must restart, got {other:?}"),
        };

        assert_ne!(first_generation, reopened_generation);
        assert!(!slot.accepts(first_generation, &first));
        assert!(slot.accepts(reopened_generation, &reopened));
    }

    // TP-B1.4-LIFECYCLE: one active job plus one replaceable pending slot is
    // the complete work queue. Rapid navigation never builds an unbounded
    // backlog, and only the latest generation can be applied.
    #[test]
    fn highlight_worker_keeps_only_latest_pending_request() {
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let processed = Arc::new(Mutex::new(Vec::new()));
        let processed_by_worker = processed.clone();
        let mut worker = FilePreviewHighlightWorker::with_processor(
            Arc::new(Notify::new()),
            move |_path, preview| {
                if preview.content == "first" {
                    started_tx.send(()).expect("signal first started");
                    release_rx.recv().expect("release first request");
                }
                processed_by_worker
                    .lock()
                    .expect("processed lock")
                    .push(preview.content.clone());
                highlighted(&preview.content)
            },
        );
        let path = PathBuf::from("sample.rs");

        worker.sync_target(Some((path.clone(), preview("first"))));
        started_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("first request started");
        worker.sync_target(Some((path.clone(), preview("second"))));
        let third = preview("third");
        let third_key = FilePreviewHighlightKey::new(&path, &third);
        worker.sync_target(Some((path, third)));
        release_tx.send(()).expect("release first request");

        let current = wait_for_current(&mut worker);
        assert_eq!(current.key, third_key);
        assert_eq!(
            *processed.lock().expect("processed lock"),
            vec!["first".to_owned(), "third".to_owned()]
        );
    }

    // FM-PERF-TEXT-03: pending file reads share the same hard queue bound as
    // highlighting: one executing request plus one replaceable latest slot.
    #[test]
    fn pending_preview_worker_executes_first_and_latest_only() {
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let processed = Arc::new(Mutex::new(Vec::new()));
        let processed_by_worker = processed.clone();
        let mut worker = FilePreviewWorker::with_preview_processor(
            Arc::new(Notify::new()),
            move |path, source| {
                assert!(matches!(source, FilePreviewSource::Pending));
                let name = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .expect("test path name")
                    .to_owned();
                if name == "first.rs" {
                    started_tx.send(()).expect("signal first read started");
                    release_rx.recv().expect("release first read");
                }
                processed_by_worker
                    .lock()
                    .expect("processed lock")
                    .push(name.clone());
                let mut preview = preview_for(path, &name);
                preview.highlighted = Some(highlighted(&name));
                Ok(preview)
            },
        );
        let first = PathBuf::from("first.rs");
        let second = PathBuf::from("second.rs");
        let third = PathBuf::from("third.rs");

        worker.sync_target(Some(FilePreviewTarget::pending(7, first, 1)));
        started_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("first pending read started");
        worker.sync_target(Some(FilePreviewTarget::pending(7, second, 2)));
        let third_target = FilePreviewTarget::pending(7, third.clone(), 3);
        let third_key = third_target.key.clone();
        worker.sync_target(Some(third_target));
        release_tx.send(()).expect("release first read");

        let current = wait_for_current(&mut worker);
        assert_eq!(current.key, third_key);
        assert_eq!(current.prepared.expect("latest preview").source_path, third);
        assert_eq!(
            *processed.lock().expect("processed lock"),
            vec!["first.rs".to_owned(), "third.rs".to_owned()]
        );
    }

    #[test]
    fn text_worker_profile_counts_submitted_completed_and_rejected() {
        let (started_tx, started_rx) = mpsc::channel::<String>();
        let (release_tx, release_rx) = mpsc::channel::<()>();
        let mut worker = FilePreviewHighlightWorker::with_processor(
            Arc::new(Notify::new()),
            move |_path, preview| {
                started_tx
                    .send(preview.content.clone())
                    .expect("signal started text request");
                release_rx.recv().expect("release text request");
                highlighted(&preview.content)
            },
        );
        let path = PathBuf::from("sample.rs");

        let (_, profile) = crate::render_prof::observe_for_test(|| {
            assert!(matches!(
                worker.sync_target(Some((path.clone(), preview("first")))),
                FilePreviewHighlightSync::Started { .. }
            ));
            assert_eq!(
                started_rx
                    .recv_timeout(Duration::from_secs(2))
                    .expect("first text request starts"),
                "first"
            );
            assert!(matches!(
                worker.sync_target(Some((path, preview("second")))),
                FilePreviewHighlightSync::Started { .. }
            ));

            release_tx.send(()).expect("release stale text request");
            assert_eq!(
                started_rx
                    .recv_timeout(Duration::from_secs(2))
                    .expect("second text request starts"),
                "second"
            );
            let stale = worker.drain();
            assert!(
                stale.current.is_none(),
                "the first completion is rejected after the target changes"
            );
            assert!(!stale.disconnected);

            release_tx.send(()).expect("release current text request");
            let current = wait_for_current(&mut worker);
            assert_eq!(
                current.prepared.expect("prepared preview").highlighted,
                Some(highlighted("second"))
            );
        });

        assert_eq!(profile.counter("fm.text_worker.submitted"), 2);
        assert_eq!(profile.counter("fm.text_worker.rejected"), 1);
        assert_eq!(profile.counter("fm.text_worker.completed"), 1);
    }

    // TP-B1.4-LIFECYCLE: a processor panic closes the result channel. Draining
    // reports degradation instead of panicking or applying partial data.
    #[test]
    fn highlight_worker_reports_processor_disconnect_without_panic() {
        let mut worker = FilePreviewHighlightWorker::with_processor(
            Arc::new(Notify::new()),
            |_path, _preview| -> HighlightedTextPreview {
                panic!("intentional worker failure fixture")
            },
        );
        worker.sync_target(Some((PathBuf::from("panic.rs"), preview("panic"))));

        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let drained = worker.drain();
            assert!(drained.current.is_none());
            if drained.disconnected {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "timed out waiting for disconnect"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    // FM-PERF-TEXT-09: text preparation now includes filesystem I/O, which
    // may block indefinitely on a stalled mount. Dropping App authority must
    // close the queue without joining a still-running filesystem call.
    #[test]
    fn dropping_preview_worker_does_not_wait_for_blocked_processor() {
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let mut worker = FilePreviewWorker::with_preview_processor(
            Arc::new(Notify::new()),
            move |_path, _source| {
                started_tx.send(()).expect("signal blocked processor");
                release_rx.recv().expect("release blocked processor");
                Ok(preview("released"))
            },
        );
        worker.sync_target(Some(FilePreviewTarget::pending(
            1,
            PathBuf::from("blocked.rs"),
            1,
        )));
        started_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("processor started");

        let (dropped_tx, dropped_rx) = mpsc::channel();
        let drop_thread = std::thread::spawn(move || {
            drop(worker);
            dropped_tx.send(()).expect("signal worker dropped");
        });
        let dropped_without_waiting = dropped_rx.recv_timeout(Duration::from_millis(250)).is_ok();
        release_tx.send(()).expect("release blocked processor");
        drop_thread.join().expect("join drop observer");

        assert!(
            dropped_without_waiting,
            "worker drop must detach a blocked filesystem processor"
        );
    }

    // TP-B1.4-LIFECYCLE: App owns the worker and applies one current result to
    // pure FmState. Repeated scheduled syncs do not create a dirty loop.
    #[test]
    fn app_applies_current_text_highlight_once() {
        let td = TempDir::new("app-current");
        std::fs::write(td.root.join("sample.rs"), "pub fn main() {}\n")
            .expect("write Rust preview fixture");
        let mut app = test_app();
        app.state
            .try_open_file_manager_with(|_| Some(crate::fm::FmState::new(&td.root)))
            .expect("Files activation");

        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let changed = app.sync_file_preview_worker();
            let highlighted =
                app.state
                    .file_manager
                    .as_ref()
                    .and_then(|state| match &state.preview {
                        crate::fm::FmPreview::File(crate::fm::FmFilePreview::Text(preview)) => {
                            preview.highlighted.as_ref()
                        }
                        _ => None,
                    });
            if let Some(highlighted) = highlighted {
                assert!(changed, "first applied result must dirty the frame");
                assert_eq!(highlighted.syntax_name.as_deref(), Some("Rust"));
                break;
            }
            assert!(Instant::now() < deadline, "timed out waiting for App apply");
            std::thread::sleep(Duration::from_millis(5));
        }

        assert!(
            !app.sync_file_preview_worker(),
            "unchanged highlighted state must not dirty the frame"
        );
    }

    // FM-PERF-TEXT-04: a resident file click performs only pure selection
    // projection. The worker later installs one exact path/generation result
    // into both legacy preview and Trail detail authorities.
    #[test]
    fn app_resolves_pending_file_selection_off_thread_once() {
        let td = TempDir::new("pending-selection");
        let path = td.root.join("clicked.rs");
        std::fs::write(&path, "fn clicked() {}\n").expect("write click preview fixture");
        let mut app = test_app();
        app.state
            .try_open_file_manager_with(|_| Some(crate::fm::FmState::new(&td.root)))
            .expect("Files activation");

        let file_manager = app.state.file_manager.as_mut().expect("open Files");
        let entry_index = file_manager
            .trail_snapshots
            .cols()
            .first()
            .and_then(|snapshot| {
                snapshot
                    .entries()
                    .iter()
                    .position(|entry| entry.path == path)
            })
            .expect("click row identity");
        assert_eq!(
            file_manager.activate_trail_entry(0, entry_index, &path),
            crate::fm::trail_snapshots::TrailActivateOutcome::SelectedFile
        );
        let generation = file_manager.preview_generation;
        assert!(matches!(
            &file_manager.preview,
            crate::fm::FmPreview::File(crate::fm::FmFilePreview::PendingText {
                source_path,
                generation: pending_generation,
            }) if source_path == &path && *pending_generation == generation
        ));
        assert_eq!(
            file_manager
                .trail_snapshots
                .detail()
                .map(|detail| &detail.preview),
            Some(&crate::fm::trail_snapshots::TrailDetailPreview::PendingText)
        );
        assert!(
            !file_manager.apply_prepared_text_preview(
                &path,
                generation.wrapping_add(1),
                Ok(preview_for(&path, "stale")),
            ),
            "a stale generation cannot replace pending state"
        );

        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if app.sync_file_preview_worker() {
                break;
            }
            assert!(Instant::now() < deadline, "timed out loading clicked file");
            std::thread::yield_now();
        }

        let file_manager = app.state.file_manager.as_ref().expect("open Files");
        match &file_manager.preview {
            crate::fm::FmPreview::File(crate::fm::FmFilePreview::Text(preview)) => {
                assert_eq!(preview.source_path, path);
                assert_eq!(preview.content, "fn clicked() {}\n");
                assert!(preview.highlighted.is_some());
            }
            other => panic!("expected resolved text preview, got {other:?}"),
        }
        match file_manager
            .trail_snapshots
            .detail()
            .map(|detail| &detail.preview)
        {
            Some(crate::fm::trail_snapshots::TrailDetailPreview::Text(preview)) => {
                assert_eq!(preview.source_path, path);
                assert!(preview.highlighted.is_some());
            }
            other => panic!("expected resolved Trail detail, got {other:?}"),
        }
        let generation_after_apply = app.file_preview_worker_generation_for_test();
        assert!(!app.sync_file_preview_worker());
        assert!(!app.sync_file_preview_worker());
        assert_eq!(
            app.file_preview_worker_generation_for_test(),
            generation_after_apply.wrapping_add(1).max(1),
            "resolved content stops the worker target once without resubmitting"
        );
    }

    // FM-PERF-TEXT-05: TOCTOU deletion is an explicit bounded failure, not a
    // retry loop or a permanently loading detail panel.
    #[test]
    fn missing_pending_file_becomes_explicit_unavailable_state() {
        let td = TempDir::new("pending-missing");
        let path = td.root.join("vanished.txt");
        std::fs::write(&path, "gone\n").expect("write disappearing fixture");
        let mut app = test_app();
        app.state
            .try_open_file_manager_with(|_| Some(crate::fm::FmState::new(&td.root)))
            .expect("Files activation");
        let file_manager = app.state.file_manager.as_mut().expect("open Files");
        let entry_index = file_manager.trail_snapshots.cols()[0]
            .entries()
            .iter()
            .position(|entry| entry.path == path)
            .expect("row identity");
        assert_eq!(
            file_manager.activate_trail_entry(0, entry_index, &path),
            crate::fm::trail_snapshots::TrailActivateOutcome::SelectedFile
        );
        std::fs::remove_file(&path).expect("delete before worker read");

        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if app.sync_file_preview_worker() {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "timed out applying missing result"
            );
            std::thread::yield_now();
        }
        let file_manager = app.state.file_manager.as_ref().expect("open Files");
        assert!(matches!(
            file_manager.preview,
            crate::fm::FmPreview::File(crate::fm::FmFilePreview::Unavailable(
                TextPreviewError::Io(std::io::ErrorKind::NotFound)
            ))
        ));
        assert!(matches!(
            file_manager
                .trail_snapshots
                .detail()
                .map(|detail| &detail.preview),
            Some(crate::fm::trail_snapshots::TrailDetailPreview::Unpreviewable(reason))
                if reason.contains("NotFound")
        ));
    }

    // FM-PERF-TEXT-07: byte-classification failures cross the same async seam
    // and terminate in a stable explicit state.
    #[test]
    fn invalid_utf8_pending_file_becomes_explicit_unavailable_state() {
        let td = TempDir::new("pending-invalid-utf8");
        let path = td.root.join("invalid.dat");
        std::fs::write(&path, [159, 146, 150]).expect("write invalid UTF-8 fixture");
        let mut app = test_app();
        app.state
            .try_open_file_manager_with(|_| Some(crate::fm::FmState::new(&td.root)))
            .expect("Files activation");
        let file_manager = app.state.file_manager.as_mut().expect("open Files");
        let entry_index = file_manager.trail_snapshots.cols()[0]
            .entries()
            .iter()
            .position(|entry| entry.path == path)
            .expect("row identity");
        assert_eq!(
            file_manager.activate_trail_entry(0, entry_index, &path),
            crate::fm::trail_snapshots::TrailActivateOutcome::SelectedFile
        );

        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if app.sync_file_preview_worker() {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "timed out applying invalid UTF-8 result"
            );
            std::thread::yield_now();
        }
        let file_manager = app.state.file_manager.as_ref().expect("open Files");
        assert!(matches!(
            file_manager.preview,
            crate::fm::FmPreview::File(crate::fm::FmFilePreview::Unavailable(
                TextPreviewError::InvalidUtf8 { valid_up_to: 0 }
            ))
        ));
        assert!(matches!(
            file_manager
                .trail_snapshots
                .detail()
                .map(|detail| &detail.preview),
            Some(crate::fm::trail_snapshots::TrailDetailPreview::Unpreviewable(reason))
                if reason.contains("not UTF-8")
        ));
    }

    // FM-PERF-TEXT-08: a dead preview processor cannot leave the UI in an
    // eternal loading state. Disconnect is converted once to a typed failure.
    #[test]
    fn pending_preview_worker_disconnect_clears_loading_state() {
        let td = TempDir::new("pending-disconnect");
        let path = td.root.join("panic.rs");
        let retry_path = td.root.join("retry.rs");
        std::fs::write(&path, "fn panic_fixture() {}\n").expect("write panic fixture");
        std::fs::write(&retry_path, "fn retry_fixture() {}\n").expect("write retry fixture");
        let mut app = test_app();
        app.file_preview_worker = FilePreviewWorker::with_preview_processor(
            Arc::new(Notify::new()),
            |_path, _source| -> Result<TextPreview, TextPreviewError> {
                panic!("intentional pending preview processor failure")
            },
        );
        app.state
            .try_open_file_manager_with(|_| Some(crate::fm::FmState::new(&td.root)))
            .expect("Files activation");
        let file_manager = app.state.file_manager.as_mut().expect("open Files");
        let entry_index = file_manager.trail_snapshots.cols()[0]
            .entries()
            .iter()
            .position(|entry| entry.path == path)
            .expect("row identity");
        assert_eq!(
            file_manager.activate_trail_entry(0, entry_index, &path),
            crate::fm::trail_snapshots::TrailActivateOutcome::SelectedFile
        );

        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if app.sync_file_preview_worker() {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "timed out observing preview worker disconnect"
            );
            std::thread::yield_now();
        }
        assert!(matches!(
            app.state
                .file_manager
                .as_ref()
                .map(|file_manager| &file_manager.preview),
            Some(crate::fm::FmPreview::File(
                crate::fm::FmFilePreview::Unavailable(TextPreviewError::Io(
                    std::io::ErrorKind::BrokenPipe
                ))
            ))
        ));

        let file_manager = app.state.file_manager.as_mut().expect("open Files");
        let retry_index = file_manager.trail_snapshots.cols()[0]
            .entries()
            .iter()
            .position(|entry| entry.path == retry_path)
            .expect("retry row identity");
        assert_eq!(
            file_manager.activate_trail_entry(0, retry_index, &retry_path),
            crate::fm::trail_snapshots::TrailActivateOutcome::SelectedFile
        );
        assert!(
            app.sync_file_preview_worker(),
            "every new pending selection after disconnect must terminate instead of loading forever"
        );
        assert!(matches!(
            app.state
                .file_manager
                .as_ref()
                .map(|file_manager| &file_manager.preview),
            Some(crate::fm::FmPreview::File(
                crate::fm::FmFilePreview::Unavailable(TextPreviewError::Io(
                    std::io::ErrorKind::BrokenPipe
                ))
            ))
        ));
    }

    // P5 BRANCH/WORKER RACE: a highlight result from a branch retired by
    // ancestor-then-sibling navigation can arrive while the sibling preview
    // is still being processed. The old branch authority and its result must
    // both be rejected; only the current sibling generation may apply.
    #[test]
    fn branch_truncation_rejects_stale_preview_completion() {
        let td = TempDir::new("branch-truncation-stale");
        let old_branch = td.root.join("old-branch");
        let sibling = td.root.join("sibling");
        std::fs::create_dir_all(&old_branch).expect("create old branch");
        std::fs::create_dir_all(&sibling).expect("create sibling branch");
        std::fs::write(old_branch.join("alpha.rs"), "fn alpha() {}\n")
            .expect("write first preview fixture");
        std::fs::write(sibling.join("beta.py"), "def beta():\n    pass\n")
            .expect("write second preview fixture");
        let (first_started_tx, first_started_rx) = mpsc::channel();
        let (first_release_tx, first_release_rx) = mpsc::channel();
        let (second_started_tx, second_started_rx) = mpsc::channel();
        let (second_release_tx, second_release_rx) = mpsc::channel();
        let worker = FilePreviewHighlightWorker::with_processor(
            Arc::new(Notify::new()),
            move |path, preview| {
                if path.ends_with("alpha.rs") {
                    first_started_tx.send(()).expect("signal first started");
                    first_release_rx.recv().expect("release first result");
                } else {
                    second_started_tx.send(()).expect("signal second started");
                    second_release_rx.recv().expect("release second result");
                }
                highlight_text_preview(path, preview)
            },
        );
        let mut app = test_app();
        app.file_preview_worker = worker;
        app.state
            .try_open_file_manager_with(|_| Some(crate::fm::FmState::new(&old_branch)))
            .expect("Files activation");

        assert!(!app.sync_file_preview_worker());
        first_started_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("first request started");
        let file_manager = app.state.file_manager.as_mut().expect("open file manager");
        file_manager.leave();
        let sibling_index = file_manager
            .entries
            .iter()
            .position(|entry| entry.path == sibling)
            .expect("sibling row after ancestor transition");
        assert!(file_manager.replace_selection(sibling_index));
        file_manager.enter();
        assert_eq!(file_manager.cwd, sibling);
        assert!(
            file_manager
                .miller
                .chain
                .iter()
                .all(|segment| segment.directory != old_branch),
            "the retired branch segment cannot remain addressable"
        );
        assert!(!app.sync_file_preview_worker());

        first_release_tx.send(()).expect("release first result");
        second_started_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("second request started after first result was stored");
        assert!(
            !app.sync_file_preview_worker(),
            "stale first result must not dirty or mutate current preview"
        );
        let current_highlight =
            app.state
                .file_manager
                .as_ref()
                .and_then(|state| match &state.preview {
                    crate::fm::FmPreview::File(crate::fm::FmFilePreview::Text(preview)) => {
                        preview.highlighted.as_ref()
                    }
                    _ => None,
                });
        assert!(current_highlight.is_none());

        second_release_tx.send(()).expect("release second result");
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            if app.sync_file_preview_worker() {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "timed out applying current result"
            );
            std::thread::yield_now();
        }
        let current_highlight = app
            .state
            .file_manager
            .as_ref()
            .and_then(|state| match &state.preview {
                crate::fm::FmPreview::File(crate::fm::FmFilePreview::Text(preview)) => {
                    preview.highlighted.as_ref()
                }
                _ => None,
            })
            .expect("current result applied");
        assert_eq!(current_highlight.syntax_name.as_deref(), Some("Python"));
        assert_eq!(current_highlight.lines.len(), 2);
    }

    #[test]
    fn stale_worker_completion_after_scroll_is_rejected() {
        let td = TempDir::new("scroll-stale-completion");
        std::fs::write(td.root.join("alpha.rs"), "fn alpha() {}\n")
            .expect("write first preview fixture");
        std::fs::write(td.root.join("beta.py"), "def beta():\n    pass\n")
            .expect("write second preview fixture");
        let (first_started_tx, first_started_rx) = mpsc::channel();
        let (first_release_tx, first_release_rx) = mpsc::channel();
        let (second_started_tx, second_started_rx) = mpsc::channel();
        let (second_release_tx, second_release_rx) = mpsc::channel();
        let worker = FilePreviewHighlightWorker::with_processor(
            Arc::new(Notify::new()),
            move |path, preview| {
                if path.ends_with("alpha.rs") {
                    first_started_tx.send(()).expect("signal first started");
                    first_release_rx.recv().expect("release first result");
                } else {
                    second_started_tx.send(()).expect("signal second started");
                    second_release_rx.recv().expect("release second result");
                }
                highlight_text_preview(path, preview)
            },
        );
        let mut app = test_app();
        app.file_preview_worker = worker;
        app.state
            .try_open_file_manager_with(|_| Some(crate::fm::FmState::new(&td.root)))
            .expect("Files activation");

        assert!(!app.sync_file_preview_worker());
        first_started_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("first request started");
        let file_manager = app.state.file_manager.as_mut().expect("open Files");
        file_manager.move_down();
        assert_eq!(
            file_manager.selected().map(|entry| entry.name.as_str()),
            Some("beta.py")
        );
        assert!(!app.sync_file_preview_worker());

        first_release_tx.send(()).expect("release stale result");
        second_started_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("current request started");
        assert!(
            !app.sync_file_preview_worker(),
            "the stale result cannot dirty the current preview"
        );
        assert!(
            app.state
                .file_manager
                .as_ref()
                .and_then(|state| match &state.preview {
                    crate::fm::FmPreview::File(crate::fm::FmFilePreview::Text(preview)) => {
                        preview.highlighted.as_ref()
                    }
                    _ => None,
                })
                .is_none(),
            "the stale Rust highlight cannot appear under the Python selection"
        );

        second_release_tx.send(()).expect("release current result");
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            if app.sync_file_preview_worker() {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "timed out applying the current scrolled result"
            );
            std::thread::yield_now();
        }
        let current_highlight = app
            .state
            .file_manager
            .as_ref()
            .and_then(|state| match &state.preview {
                crate::fm::FmPreview::File(crate::fm::FmFilePreview::Text(preview)) => {
                    preview.highlighted.as_ref()
                }
                _ => None,
            })
            .expect("current result applied");
        assert_eq!(current_highlight.syntax_name.as_deref(), Some("Python"));
    }

    // TP-B1.4-CLOSE: closing the FM invalidates a running request. Even when
    // that request later publishes a result, scheduled sync must discard it
    // without dirtying the frame or recreating file-manager state.
    #[test]
    fn app_discards_inflight_highlight_after_file_manager_close() {
        let td = TempDir::new("app-close-stale");
        std::fs::write(td.root.join("sample.rs"), "fn sample() {}\n")
            .expect("write preview fixture");
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let worker = FilePreviewHighlightWorker::with_processor(
            Arc::new(Notify::new()),
            move |path, preview| {
                started_tx.send(()).expect("signal request started");
                release_rx.recv().expect("release result");
                highlight_text_preview(path, preview)
            },
        );
        let shared = worker.shared.clone();
        let mut app = test_app();
        app.file_preview_worker = worker;
        app.state
            .try_open_file_manager_with(|_| Some(crate::fm::FmState::new(&td.root)))
            .expect("Files activation");

        assert!(!app.sync_file_preview_worker());
        started_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("request started");
        app.state.file_manager = None;
        assert!(!app.sync_file_preview_worker());
        release_tx.send(()).expect("release stale result");

        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            if lock_state(&shared.0).result.is_some() {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "timed out waiting for stale result"
            );
            std::thread::yield_now();
        }
        assert!(!app.sync_file_preview_worker());
        assert!(app.state.file_manager.is_none());
        assert!(!app.sync_file_preview_worker());
    }

    // TP-B1.4-REOPEN: an App-owned worker can stop with the closed FM and then
    // bind a fresh generation after reopen. The prior path/syntax cannot leak
    // into the reopened preview.
    #[test]
    fn app_reopen_highlights_only_the_new_file_manager_selection() {
        let first = TempDir::new("app-reopen-first");
        let second = TempDir::new("app-reopen-second");
        std::fs::write(first.root.join("first.rs"), "fn first() {}\n")
            .expect("write first fixture");
        std::fs::write(second.root.join("second.py"), "def second():\n    pass\n")
            .expect("write second fixture");
        let mut app = test_app();
        app.state
            .try_open_file_manager_with(|_| Some(crate::fm::FmState::new(&first.root)))
            .expect("Files activation");

        let first_deadline = Instant::now() + Duration::from_secs(5);
        loop {
            app.sync_file_preview_worker();
            let syntax = app
                .state
                .file_manager
                .as_ref()
                .and_then(|state| match &state.preview {
                    crate::fm::FmPreview::File(crate::fm::FmFilePreview::Text(preview)) => preview
                        .highlighted
                        .as_ref()
                        .and_then(|highlighted| highlighted.syntax_name.as_deref()),
                    _ => None,
                });
            if syntax == Some("Rust") {
                break;
            }
            assert!(
                Instant::now() < first_deadline,
                "timed out highlighting first FM"
            );
            std::thread::yield_now();
        }

        app.state.file_manager = None;
        assert!(!app.sync_file_preview_worker());
        app.state
            .try_open_file_manager_with(|_| Some(crate::fm::FmState::new(&second.root)))
            .expect("Files activation");

        let second_deadline = Instant::now() + Duration::from_secs(5);
        loop {
            app.sync_file_preview_worker();
            let syntax = app
                .state
                .file_manager
                .as_ref()
                .and_then(|state| match &state.preview {
                    crate::fm::FmPreview::File(crate::fm::FmFilePreview::Text(preview)) => {
                        assert!(preview.source_path.ends_with("second.py"));
                        preview
                            .highlighted
                            .as_ref()
                            .and_then(|highlighted| highlighted.syntax_name.as_deref())
                    }
                    _ => None,
                });
            if syntax == Some("Python") {
                break;
            }
            assert!(
                syntax != Some("Rust"),
                "closed file-manager syntax leaked after reopen"
            );
            assert!(
                Instant::now() < second_deadline,
                "timed out highlighting reopened FM"
            );
            std::thread::yield_now();
        }
    }
}
