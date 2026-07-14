use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::thread::JoinHandle;

use sha2::{Digest, Sha256};
use tokio::sync::Notify;

use crate::fm::{highlight_text_preview, HighlightedTextPreview, TextPreview};

#[derive(Debug, Clone, PartialEq, Eq)]
struct FilePreviewHighlightKey {
    path: PathBuf,
    content_sha256: [u8; 32],
    truncated: bool,
}

impl FilePreviewHighlightKey {
    fn new(path: &Path, preview: &TextPreview) -> Self {
        let digest = Sha256::digest(preview.content.as_bytes());
        Self {
            path: path.to_path_buf(),
            content_sha256: digest.into(),
            truncated: preview.truncated,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FilePreviewHighlightSync {
    Unchanged,
    Started { generation: u64 },
    Stopped,
}

#[derive(Debug, Default)]
struct FilePreviewHighlightSlot {
    generation: u64,
    active: Option<FilePreviewHighlightKey>,
}

impl FilePreviewHighlightSlot {
    fn sync(&mut self, target: Option<FilePreviewHighlightKey>) -> FilePreviewHighlightSync {
        if self.active == target {
            return FilePreviewHighlightSync::Unchanged;
        }

        self.generation = self.generation.wrapping_add(1).max(1);
        self.active = target;
        if self.active.is_some() {
            FilePreviewHighlightSync::Started {
                generation: self.generation,
            }
        } else {
            FilePreviewHighlightSync::Stopped
        }
    }

    fn accepts(&self, generation: u64, key: &FilePreviewHighlightKey) -> bool {
        self.generation == generation && self.active.as_ref() == Some(key)
    }
}

#[derive(Debug)]
struct FilePreviewHighlightRequest {
    generation: u64,
    key: FilePreviewHighlightKey,
    path: PathBuf,
    preview: TextPreview,
}

#[derive(Debug)]
struct FilePreviewHighlightResult {
    generation: u64,
    key: FilePreviewHighlightKey,
    highlighted: HighlightedTextPreview,
}

#[derive(Debug, Default)]
struct FilePreviewHighlightDrain {
    current: Option<FilePreviewHighlightResult>,
    disconnected: bool,
}

#[derive(Debug)]
struct FilePreviewWorkerState {
    pending: Option<FilePreviewHighlightRequest>,
    result: Option<FilePreviewHighlightResult>,
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

pub(super) struct FilePreviewHighlightWorker {
    slot: FilePreviewHighlightSlot,
    shared: SharedWorkerState,
    handle: Option<JoinHandle<()>>,
    disconnect_reported: bool,
}

impl FilePreviewHighlightWorker {
    pub(super) fn new(wake: Arc<Notify>) -> Self {
        Self::with_processor(wake, highlight_text_preview)
    }

    fn with_processor<F>(wake: Arc<Notify>, processor: F) -> Self
    where
        F: Fn(&Path, &TextPreview) -> HighlightedTextPreview + Send + 'static,
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
            loop {
                let Some(request) = take_next_request(&worker_shared) else {
                    break;
                };
                let highlighted = processor(&request.path, &request.preview);
                let result = FilePreviewHighlightResult {
                    generation: request.generation,
                    key: request.key,
                    highlighted,
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
            slot: FilePreviewHighlightSlot::default(),
            shared,
            handle: Some(handle),
            disconnect_reported: false,
        }
    }

    fn sync_target(&mut self, target: Option<(PathBuf, TextPreview)>) -> FilePreviewHighlightSync {
        let target_key = target
            .as_ref()
            .map(|(path, preview)| FilePreviewHighlightKey::new(path, preview));
        let sync = self.slot.sync(target_key.clone());
        let (state, pending) = &*self.shared;
        let mut state = lock_state(state);
        match sync {
            FilePreviewHighlightSync::Started { generation } => {
                if let (Some(key), Some((path, preview))) = (target_key, target) {
                    state.pending = Some(FilePreviewHighlightRequest {
                        generation,
                        key,
                        path,
                        preview,
                    });
                    pending.notify_one();
                }
            }
            FilePreviewHighlightSync::Stopped => {
                state.pending = None;
                state.result = None;
            }
            FilePreviewHighlightSync::Unchanged => {}
        }
        sync
    }

    fn drain(&mut self) -> FilePreviewHighlightDrain {
        let (state, _) = &*self.shared;
        let mut state = lock_state(state);
        let result = state.result.take();
        let disconnected = !state.alive && !state.closed && !self.disconnect_reported;
        self.disconnect_reported |= disconnected;
        drop(state);

        let current = result.filter(|result| self.slot.accepts(result.generation, &result.key));
        FilePreviewHighlightDrain {
            current,
            disconnected,
        }
    }
}

impl Drop for FilePreviewHighlightWorker {
    fn drop(&mut self) {
        let (state, pending) = &*self.shared;
        let mut state = lock_state(state);
        state.closed = true;
        state.pending = None;
        state.result = None;
        drop(state);
        pending.notify_one();

        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn take_next_request(shared: &SharedWorkerState) -> Option<FilePreviewHighlightRequest> {
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

fn lock_state(state: &Mutex<FilePreviewWorkerState>) -> MutexGuard<'_, FilePreviewWorkerState> {
    match state.lock() {
        Ok(state) => state,
        Err(poisoned) => poisoned.into_inner(),
    }
}

impl super::App {
    pub(super) fn sync_file_preview_worker(&mut self) -> bool {
        let target = self.state.file_manager.as_ref().and_then(|file_manager| {
            let selected_path = file_manager.selected()?.path.clone();
            let crate::fm::FmPreview::File(crate::fm::FmFilePreview::Text(preview)) =
                &file_manager.preview
            else {
                return None;
            };
            let mut source = preview.clone();
            source.highlighted = None;
            Some((selected_path, source))
        });

        let _ = self.file_preview_worker.sync_target(target);
        let drained = self.file_preview_worker.drain();
        if drained.disconnected {
            tracing::warn!("fm: text preview highlight worker stopped; using plain-text fallback");
        }
        let Some(result) = drained.current else {
            return false;
        };
        let Some(file_manager) = self.state.file_manager.as_mut() else {
            return false;
        };
        let Some(selected_path) = file_manager.selected().map(|entry| entry.path.clone()) else {
            return false;
        };
        let crate::fm::FmPreview::File(crate::fm::FmFilePreview::Text(preview)) =
            &mut file_manager.preview
        else {
            return false;
        };
        if FilePreviewHighlightKey::new(&selected_path, preview) != result.key {
            return false;
        }
        if preview.highlighted.as_ref() == Some(&result.highlighted) {
            return false;
        }
        preview.highlighted = Some(result.highlighted);
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

    // TP-B1.4-LIFECYCLE: App owns the worker and applies one current result to
    // pure FmState. Repeated scheduled syncs do not create a dirty loop.
    #[test]
    fn app_applies_current_text_highlight_once() {
        let td = TempDir::new("app-current");
        std::fs::write(td.root.join("sample.rs"), "pub fn main() {}\n")
            .expect("write Rust preview fixture");
        let mut app = test_app();
        app.state.file_manager = Some(crate::fm::FmState::new(&td.root));

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
}
