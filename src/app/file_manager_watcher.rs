//! Runtime ownership for native file-manager filesystem watching.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender, TryRecvError, TrySendError};
use std::sync::Arc;
use std::time::Duration;

use notify_debouncer_full::{
    new_debouncer,
    notify::{RecommendedWatcher, RecursiveMode},
    DebounceEventResult, Debouncer, RecommendedCache,
};
use tokio::sync::Notify;

use crate::fm::watcher::{
    drain_watch_messages, watch_message_from_result, FmWatchDrain, FmWatchMessage, FmWatcherSlot,
    FmWatcherSync,
};

const WATCH_DEBOUNCE: Duration = Duration::from_millis(250);
const WATCH_CHANNEL_CAPACITY: usize = 64;
pub(super) const WATCH_DRAIN_LIMIT: usize = 32;
const WATCH_RECONCILE_INTERVAL: Duration = Duration::from_secs(2);

struct OwnedOperationBurst {
    watcher_generation: u64,
    paths: BTreeSet<PathBuf>,
    expires_at: std::time::Instant,
}

type NativeDebouncer = Debouncer<RecommendedWatcher, RecommendedCache>;

enum NativeFmBackend {
    Native { _debouncer: NativeDebouncer },
    PollingFallback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FileManagerWatcherMode {
    Inactive,
    Native,
    PollingFallback,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(super) struct FileManagerWatchDrain {
    pub(super) events: FmWatchDrain,
    pub(super) disconnected: bool,
    pub(super) overflowed: bool,
    pub(super) limit_reached: bool,
}

fn enqueue_message(
    sender: &SyncSender<FmWatchMessage>,
    overflowed: &AtomicBool,
    wake: &Notify,
    message: FmWatchMessage,
) {
    match sender.try_send(message) {
        Ok(()) => wake.notify_one(),
        Err(TrySendError::Full(_)) => {
            overflowed.store(true, Ordering::Release);
            wake.notify_one();
        }
        Err(TrySendError::Disconnected(_)) => {}
    }
}

fn drain_receiver<B>(
    slot: &FmWatcherSlot<B>,
    receiver: &Receiver<FmWatchMessage>,
    overflowed: &AtomicBool,
    limit: usize,
) -> FileManagerWatchDrain {
    let mut messages = Vec::with_capacity(limit);
    let mut disconnected = false;

    for _ in 0..limit {
        match receiver.try_recv() {
            Ok(message) => messages.push(message),
            Err(TryRecvError::Empty) => break,
            Err(TryRecvError::Disconnected) => {
                disconnected = true;
                break;
            }
        }
    }

    let limit_reached = messages.len() == limit && limit != 0;
    let did_overflow = overflowed.swap(false, Ordering::AcqRel);
    let mut events = drain_watch_messages(slot, messages);
    if did_overflow && slot.has_backend() {
        events.refresh = true;
    }

    FileManagerWatchDrain {
        events,
        disconnected,
        overflowed: did_overflow,
        limit_reached,
    }
}

fn start_native_backend(
    path: &Path,
    generation: u64,
    sender: SyncSender<FmWatchMessage>,
    overflowed: Arc<AtomicBool>,
    wake: Arc<Notify>,
) -> notify_debouncer_full::notify::Result<NativeFmBackend> {
    let mut debouncer = new_debouncer(WATCH_DEBOUNCE, None, move |result: DebounceEventResult| {
        enqueue_message(
            &sender,
            &overflowed,
            &wake,
            watch_message_from_result(generation, result),
        );
    })?;
    debouncer.watch(path, RecursiveMode::NonRecursive)?;
    Ok(NativeFmBackend::Native {
        _debouncer: debouncer,
    })
}

pub(super) struct NativeFileManagerWatcher {
    slot: FmWatcherSlot<NativeFmBackend>,
    receiver: Option<Receiver<FmWatchMessage>>,
    overflowed: Arc<AtomicBool>,
    wake: Arc<Notify>,
    mode: FileManagerWatcherMode,
    last_native_error: Option<String>,
    next_reconcile_at: Option<std::time::Instant>,
    requested_reconcile_generation: Option<u64>,
    reconcile_revision: u64,
    owned_operation_burst: Option<OwnedOperationBurst>,
}

impl NativeFileManagerWatcher {
    pub(super) fn new(wake: Arc<Notify>) -> Self {
        Self {
            slot: FmWatcherSlot::default(),
            receiver: None,
            overflowed: Arc::new(AtomicBool::new(false)),
            wake,
            mode: FileManagerWatcherMode::Inactive,
            last_native_error: None,
            next_reconcile_at: None,
            requested_reconcile_generation: None,
            reconcile_revision: 0,
            owned_operation_burst: None,
        }
    }

    pub(super) fn sync(&mut self, target: Option<&Path>, now: std::time::Instant) -> FmWatcherSync {
        if self.slot.watched_dir() != target {
            self.receiver = None;
        }

        let next_overflowed = Arc::new(AtomicBool::new(false));
        let backend_overflowed = next_overflowed.clone();
        let wake = self.wake.clone();
        let mut next_receiver = None;
        let mut next_mode = FileManagerWatcherMode::Inactive;
        let mut next_native_error = None;
        let result = self.slot.sync_with(target, |path, generation| {
            let (sender, receiver) = mpsc::sync_channel(WATCH_CHANNEL_CAPACITY);
            let backend =
                match start_native_backend(path, generation, sender, backend_overflowed, wake) {
                    Ok(backend) => {
                        next_receiver = Some(receiver);
                        next_mode = FileManagerWatcherMode::Native;
                        backend
                    }
                    Err(error) => {
                        next_mode = FileManagerWatcherMode::PollingFallback;
                        next_native_error = Some(error.to_string());
                        NativeFmBackend::PollingFallback
                    }
                };
            Ok::<_, std::convert::Infallible>(backend)
        });
        let result = match result {
            Ok(result) => result,
            Err(never) => match never {},
        };

        match result {
            FmWatcherSync::Unchanged => {}
            FmWatcherSync::Started { .. } => {
                crate::render_prof::event("fm.watcher.rebound");
                self.receiver = next_receiver;
                self.overflowed = next_overflowed;
                self.mode = next_mode;
                self.last_native_error = next_native_error;
                self.next_reconcile_at = Some(now + WATCH_RECONCILE_INTERVAL);
                self.requested_reconcile_generation = None;
                self.owned_operation_burst = None;
            }
            FmWatcherSync::Stopped => {
                self.receiver = None;
                self.overflowed = next_overflowed;
                self.mode = FileManagerWatcherMode::Inactive;
                self.last_native_error = None;
                self.next_reconcile_at = None;
                self.requested_reconcile_generation = None;
                self.owned_operation_burst = None;
            }
        }
        result
    }

    pub(super) fn drain(&mut self) -> FileManagerWatchDrain {
        let drained = self
            .receiver
            .as_ref()
            .map_or_else(FileManagerWatchDrain::default, |receiver| {
                drain_receiver(&self.slot, receiver, &self.overflowed, WATCH_DRAIN_LIMIT)
            });
        if drained.disconnected {
            self.receiver = None;
        }
        if drained.disconnected || !drained.events.errors.is_empty() {
            self.mode = FileManagerWatcherMode::PollingFallback;
        }
        drained
    }

    pub(super) fn watched_dir(&self) -> Option<&Path> {
        self.slot.watched_dir()
    }

    fn mode(&self) -> FileManagerWatcherMode {
        self.mode
    }

    fn last_native_error(&self) -> Option<&str> {
        self.last_native_error.as_deref()
    }

    fn take_reconcile_due(&mut self, now: std::time::Instant) -> bool {
        let Some(deadline) = self.next_reconcile_at else {
            return false;
        };
        if now < deadline {
            return false;
        }
        self.next_reconcile_at = Some(now + WATCH_RECONCILE_INTERVAL);
        true
    }

    pub(super) fn request_reconcile(&mut self, directory: &Path) -> bool {
        if self.slot.watched_dir() != Some(directory) || !self.slot.has_backend() {
            return false;
        }
        self.requested_reconcile_generation = Some(self.slot.generation());
        true
    }

    pub(super) fn own_operation_reconcile(
        &mut self,
        directory: &Path,
        watcher_generation: u64,
        paths: BTreeSet<PathBuf>,
        request_reconcile: bool,
    ) -> bool {
        if self.slot.watched_dir() != Some(directory)
            || !self.slot.accepts_generation(watcher_generation)
            || paths.is_empty()
        {
            return false;
        }
        self.owned_operation_burst = Some(OwnedOperationBurst {
            watcher_generation,
            paths,
            expires_at: std::time::Instant::now() + WATCH_RECONCILE_INTERVAL,
        });
        if request_reconcile {
            self.requested_reconcile_generation = Some(watcher_generation);
        }
        true
    }

    fn owns_drained_operation_burst(
        &mut self,
        paths: &BTreeSet<PathBuf>,
        now: std::time::Instant,
    ) -> bool {
        let Some(owned) = self.owned_operation_burst.as_ref() else {
            return false;
        };
        if now > owned.expires_at || !self.slot.accepts_generation(owned.watcher_generation) {
            self.owned_operation_burst = None;
            return false;
        }
        if paths.is_empty() {
            return false;
        }
        let owned_only = paths.iter().all(|path| owned.paths.contains(path));
        if !owned_only {
            self.owned_operation_burst = None;
        }
        owned_only
    }

    fn take_requested_reconcile(&mut self) -> bool {
        self.requested_reconcile_generation
            .take()
            .is_some_and(|generation| self.slot.accepts_generation(generation))
    }

    pub(super) fn reconcile_snapshot(&self, directory: &Path) -> Option<(u64, u64)> {
        (self.slot.watched_dir() == Some(directory) && self.slot.has_backend())
            .then_some((self.slot.generation(), self.reconcile_revision))
    }

    pub(super) fn reconciled_since(
        &self,
        directory: &Path,
        generation: u64,
        revision: u64,
    ) -> bool {
        self.slot.watched_dir() == Some(directory)
            && self.slot.accepts_generation(generation)
            && self.reconcile_revision != revision
    }

    #[cfg(test)]
    fn next_reconcile_at(&self) -> Option<std::time::Instant> {
        self.next_reconcile_at
    }

    #[cfg(test)]
    fn has_backend(&self) -> bool {
        self.slot.has_backend()
    }

    #[cfg(test)]
    fn generation(&self) -> u64 {
        self.slot.generation()
    }
}

impl super::App {
    #[cfg(test)]
    pub(in crate::app) fn file_manager_watcher_reconcile_snapshot_for_test(
        &self,
        directory: &Path,
    ) -> Option<(u64, u64)> {
        self.file_manager_watcher.reconcile_snapshot(directory)
    }

    fn active_files_generation(&self) -> Option<u32> {
        (self.state.stage.surface_view() == crate::ui::surface_host::StageSurfaceView::NativeFiles)
            .then(|| self.state.stage.active_instance_generation())
            .flatten()
    }

    fn apply_prepared_file_manager_refresh(
        &mut self,
        prepared: crate::fm::FmPreparedCurrentRefresh,
    ) -> bool {
        let Some(files_generation) = self.active_files_generation() else {
            return false;
        };
        self.state
            .file_manager
            .as_mut()
            .is_some_and(|file_manager| {
                file_manager.apply_prepared_current_refresh(prepared, files_generation)
            })
    }

    pub(super) fn execute_file_manager_current_refresh(
        &mut self,
        request: crate::fm::FmCurrentRefreshRequest,
    ) -> Option<bool> {
        let previous = self.state.file_manager.as_ref().map(|file_manager| {
            (
                file_manager.entries.clone(),
                file_manager.cursor,
                file_manager.cwd_status,
                file_manager.cwd_writable,
                file_manager.parent.clone(),
                file_manager.preview.clone(),
                file_manager.show_hidden,
            )
        })?;
        let prepared = crate::fm::prepare_current_refresh_io(request);
        if !self.apply_prepared_file_manager_refresh(prepared) {
            return None;
        }
        self.state.file_manager.as_ref().map(|file_manager| {
            previous.0 != file_manager.entries
                || previous.1 != file_manager.cursor
                || previous.2 != file_manager.cwd_status
                || previous.3 != file_manager.cwd_writable
                || previous.4 != file_manager.parent
                || previous.5 != file_manager.preview
                || previous.6 != file_manager.show_hidden
        })
    }

    pub(super) fn refresh_file_manager_after_operation(&mut self, directory: &Path) -> bool {
        let Some(files_generation) = self.active_files_generation() else {
            return false;
        };
        let Some(request) = self.state.file_manager.as_ref().and_then(|file_manager| {
            (file_manager.cwd == directory)
                .then(|| file_manager.request_operation_refresh(files_generation))
        }) else {
            return false;
        };
        self.execute_file_manager_current_refresh(request).is_some()
    }

    /// Consume one Files-sidebar navigation intent at the App-owned filesystem
    /// boundary. Both model authority and live directory type are revalidated;
    /// invalid or stale requests preserve the currently open FM projection.
    pub(super) fn sync_file_manager_sidebar_navigation(&mut self) -> bool {
        let Some(path) = self.state.request_file_manager_sidebar_navigation.take() else {
            return false;
        };

        if self.state.sidebar_tab != crate::app::state::SidebarTab::Files {
            return true;
        }
        let authorized = self
            .state
            .file_manager_sidebar
            .item_for_path(&path)
            .is_some_and(|item| item.accessible);
        if !authorized || !std::fs::metadata(&path).is_ok_and(|metadata| metadata.is_dir()) {
            return true;
        }

        let next = crate::fm::FmState::new(&path);
        // Close the inexpensive metadata/read race as far as a path-based API
        // can: if the target disappeared or changed type during preparation,
        // keep the prior projection instead of opening an invalid cwd.
        if !std::fs::metadata(&path).is_ok_and(|metadata| metadata.is_dir()) {
            return true;
        }
        self.state.file_manager = Some(next);
        true
    }

    #[cfg(test)]
    pub(super) fn sync_file_manager_watcher(&mut self) -> bool {
        self.sync_file_manager_watcher_at(std::time::Instant::now())
    }

    pub(super) fn sync_file_manager_watcher_at(&mut self, now: std::time::Instant) -> bool {
        let target = self
            .state
            .file_manager
            .as_ref()
            .map(|file_manager| file_manager.cwd.clone());

        let watcher_sync = self.file_manager_watcher.sync(target.as_deref(), now);
        if matches!(watcher_sync, FmWatcherSync::Started { .. })
            && self.file_manager_watcher.mode() == FileManagerWatcherMode::PollingFallback
        {
            tracing::warn!(
                ?target,
                error = self.file_manager_watcher.last_native_error(),
                "fm: native filesystem watcher unavailable; using bounded polling fallback"
            );
        }

        let watched_dir = self
            .file_manager_watcher
            .watched_dir()
            .map(Path::to_path_buf);
        let drained = self.file_manager_watcher.drain();

        if drained.disconnected {
            tracing::warn!(?watched_dir, "fm: filesystem watcher channel disconnected");
        }
        if drained.overflowed {
            tracing::warn!(
                ?watched_dir,
                "fm: filesystem watcher channel overflowed; forcing refresh"
            );
        }
        for error in &drained.events.errors {
            tracing::warn!(?watched_dir, %error, "fm: filesystem watcher runtime error");
        }

        let reconcile_due = self.file_manager_watcher.take_reconcile_due(now);
        let requested_reconcile = self.file_manager_watcher.take_requested_reconcile();
        let owned_operation_burst = self
            .file_manager_watcher
            .owns_drained_operation_burst(&drained.events.paths, now);
        if (!drained.events.refresh || owned_operation_burst)
            && !reconcile_due
            && !requested_reconcile
        {
            return false;
        }

        let Some(files_generation) = self.active_files_generation() else {
            return false;
        };
        let Some(request) = self.state.file_manager.as_ref().and_then(|file_manager| {
            (watched_dir.as_deref() == Some(file_manager.cwd.as_path()))
                .then(|| file_manager.request_current_refresh(files_generation))
        }) else {
            return false;
        };

        let Some(changed) = self.execute_file_manager_current_refresh(request) else {
            return false;
        };
        self.file_manager_watcher.reconcile_revision = self
            .file_manager_watcher
            .reconcile_revision
            .wrapping_add(1)
            .max(1);

        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::FileManagerHeaderAction;
    use crate::fm::operations::execute_file_operation;
    use crate::fm::watcher::{watch_message_from_result, FmWatcherSlot};
    use notify_debouncer_full::{
        notify::{event::CreateKind, Event, EventKind},
        DebouncedEvent,
    };
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering as AtomicOrdering};
    use std::sync::mpsc::sync_channel;
    use std::sync::Arc;
    use std::time::Instant;
    use tokio::sync::Notify;

    struct TempDir {
        root: PathBuf,
    }

    impl TempDir {
        fn new(tag: &str) -> Self {
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            let root = std::env::temp_dir().join(format!(
                "herdr-fmwatch-runtime-{}-{tag}-{}",
                std::process::id(),
                COUNTER.fetch_add(1, AtomicOrdering::Relaxed)
            ));
            std::fs::create_dir_all(&root).expect("create watcher temp directory");
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

    #[test]
    fn watcher_profile_counts_only_real_target_rebinds() {
        let td = TempDir::new("watcher-profile");
        let second = td.root.join("second");
        std::fs::create_dir_all(&second).expect("create second watched directory");
        let mut watcher = NativeFileManagerWatcher::new(Arc::new(Notify::new()));
        let now = Instant::now();

        let (_, profile) = crate::render_prof::observe_for_test(|| {
            assert!(matches!(
                watcher.sync(Some(&td.root), now),
                FmWatcherSync::Started { .. }
            ));
            assert_eq!(
                watcher.sync(Some(&td.root), now),
                FmWatcherSync::Unchanged,
                "an unchanged target cannot rebind the backend"
            );
            assert!(matches!(
                watcher.sync(Some(&second), now),
                FmWatcherSync::Started { .. }
            ));
            assert_eq!(watcher.sync(None, now), FmWatcherSync::Stopped);
        });

        assert_eq!(
            profile.counter("fm.watcher.rebound"),
            2,
            "only the two Started generations count as watcher rebinds"
        );
    }

    // TP-C6.1-NAV/LIFECYCLE: scheduled navigation revalidates exact current
    // model authority and directory type, consumes once, and never replaces a
    // live FM projection for stale/missing/file targets.
    #[test]
    fn sidebar_navigation_opens_exact_directory_and_rejects_stale_targets() {
        use crate::app::state::{
            FileManagerSidebarIcon, FileManagerSidebarItem, FileManagerSidebarModel,
        };
        let td = TempDir::new("sidebar-navigation");
        let target = td.root.join("target");
        std::fs::create_dir_all(&target).expect("create sidebar target");
        std::fs::write(target.join("visible.txt"), b"visible").expect("write target entry");
        let regular_file = td.root.join("not-a-directory.txt");
        std::fs::write(&regular_file, b"file").expect("write non-directory target");
        let item = |label: &str, path: PathBuf| FileManagerSidebarItem {
            label: label.to_string(),
            path,
            icon: FileManagerSidebarIcon::Pin,
            accessible: true,
            ejectable: false,
        };
        let mut app = test_app();
        app.state.sidebar_tab = crate::app::state::SidebarTab::Files;
        app.state.file_manager_sidebar = FileManagerSidebarModel::from_sources(
            Vec::new(),
            vec![
                item("Target", target.clone()),
                item("File", regular_file.clone()),
            ],
            Vec::new(),
        );
        app.state.request_file_manager_sidebar_navigation = Some(target.clone());

        assert!(app.sync_file_manager_sidebar_navigation());
        let file_manager = app
            .state
            .file_manager
            .as_ref()
            .expect("opened exact FM target");
        assert_eq!(file_manager.cwd, target);
        assert!(file_manager
            .entries
            .iter()
            .any(|entry| entry.name == "visible.txt"));
        assert!(app.state.request_file_manager_sidebar_navigation.is_none());
        assert!(
            !app.sync_file_manager_sidebar_navigation(),
            "one-shot request"
        );
        let _ = app.sync_file_manager_watcher();
        assert_eq!(
            app.file_manager_watcher.watched_dir(),
            Some(
                app.state
                    .file_manager
                    .as_ref()
                    .expect("open FM")
                    .cwd
                    .as_path()
            )
        );

        let before_cwd = app
            .state
            .file_manager
            .as_ref()
            .expect("open FM")
            .cwd
            .clone();
        app.state.request_file_manager_sidebar_navigation = Some(regular_file);
        assert!(app.sync_file_manager_sidebar_navigation());
        assert_eq!(
            app.state.file_manager.as_ref().expect("FM preserved").cwd,
            before_cwd
        );

        let missing = td.root.join("missing");
        app.state.request_file_manager_sidebar_navigation = Some(missing);
        assert!(app.sync_file_manager_sidebar_navigation());
        assert_eq!(
            app.state.file_manager.as_ref().expect("FM preserved").cwd,
            before_cwd
        );

        app.state.request_file_manager_sidebar_navigation = Some(target.clone());
        app.state.file_manager_sidebar = FileManagerSidebarModel::default();
        assert!(app.sync_file_manager_sidebar_navigation());
        assert_eq!(
            app.state.file_manager.as_ref().expect("FM preserved").cwd,
            before_cwd
        );

        let other = td.root.join("other");
        std::fs::create_dir_all(&other).expect("create alternate sidebar target");
        app.state.file_manager_sidebar = FileManagerSidebarModel::from_sources(
            Vec::new(),
            vec![item("Other", other.clone())],
            Vec::new(),
        );
        app.state.request_file_manager_sidebar_navigation = Some(other);
        app.state.sidebar_tab = crate::app::state::SidebarTab::Projects;
        assert!(app.sync_file_manager_sidebar_navigation());
        assert_eq!(
            app.state.file_manager.as_ref().expect("FM preserved").cwd,
            before_cwd,
            "leaving Files invalidates the queued navigation"
        );

        app.state.sidebar_tab = crate::app::state::SidebarTab::Files;
        app.state.request_file_manager_sidebar_navigation = Some(target.clone());
        app.state.close_file_manager();
        assert!(app.state.request_file_manager_sidebar_navigation.is_none());
        app.state.request_file_manager_sidebar_navigation = Some(target);
        app.state.open_file_manager();
        assert!(app.state.request_file_manager_sidebar_navigation.is_none());
    }

    fn wait_for_file_manager_state(
        app: &mut crate::app::App,
        description: &str,
        predicate: impl Fn(&crate::fm::FmState) -> bool,
    ) {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let _ = app.sync_file_manager_watcher();
            if app.state.file_manager.as_ref().is_some_and(&predicate) {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "timed out waiting for {description}; entries={:?}",
                app.state.file_manager.as_ref().map(|state| state
                    .entries
                    .iter()
                    .map(|entry| entry.name.as_str())
                    .collect::<Vec<_>>())
            );
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    fn message(generation: u64, path: &str) -> crate::fm::watcher::FmWatchMessage {
        let event = Event::new(EventKind::Create(CreateKind::File)).add_path(PathBuf::from(path));
        watch_message_from_result(
            generation,
            Ok(vec![DebouncedEvent::new(event, Instant::now())]),
        )
    }

    fn active_slot() -> FmWatcherSlot<()> {
        let mut slot = FmWatcherSlot::default();
        slot.sync_with(Some(Path::new("/work")), |_, _| Ok::<_, ()>(()))
            .expect("activate test watcher");
        slot
    }

    #[test]
    fn closed_callback_channel_is_reported_without_refresh_or_panic() {
        let slot = active_slot();
        let (tx, rx) = sync_channel(1);
        drop(tx);

        let drained = drain_receiver(&slot, &rx, &AtomicBool::new(false), 8);
        assert!(drained.disconnected);
        assert!(!drained.events.refresh);
        assert_eq!(drained.events.accepted_messages, 0);
    }

    #[test]
    fn full_callback_channel_marks_overflow_and_forces_safe_refresh() {
        let slot = active_slot();
        let (tx, rx) = sync_channel(1);
        let overflowed = Arc::new(AtomicBool::new(false));
        let wake = Arc::new(Notify::new());

        enqueue_message(&tx, &overflowed, &wake, message(1, "/work/first"));
        enqueue_message(&tx, &overflowed, &wake, message(1, "/work/second"));

        let drained = drain_receiver(&slot, &rx, &overflowed, 8);
        assert!(drained.overflowed);
        assert!(drained.events.refresh);
        assert_eq!(drained.events.accepted_messages, 1);
    }

    #[test]
    fn receiver_drain_is_bounded_per_iteration() {
        let slot = active_slot();
        let (tx, rx) = sync_channel(4);
        let overflowed = Arc::new(AtomicBool::new(false));
        let wake = Arc::new(Notify::new());

        for index in 0..3 {
            enqueue_message(
                &tx,
                &overflowed,
                &wake,
                message(1, &format!("/work/{index}")),
            );
        }

        let first = drain_receiver(&slot, &rx, &overflowed, 2);
        assert_eq!(first.events.accepted_messages, 2);
        assert!(first.limit_reached);

        let second = drain_receiver(&slot, &rx, &overflowed, 2);
        assert_eq!(second.events.accepted_messages, 1);
        assert!(!second.limit_reached);
    }

    // TP-C4.4-RECONCILE: a matching worker completion and its already queued
    // native watcher event share one reconciliation owner. The selected
    // preview generation is an observable reload counter: one scheduler turn
    // must advance it once, not once per producer.
    #[test]
    fn worker_completion_and_queued_watcher_event_reconcile_matching_cwd_once() {
        let td = TempDir::new("worker-watcher-coalesce");
        let source_dir = td.root.join("source");
        let destination = td.root.join("destination");
        std::fs::create_dir(&source_dir).expect("create reconcile source directory");
        std::fs::create_dir(&destination).expect("create reconcile destination directory");
        let source = source_dir.join("payload.txt");
        std::fs::write(&source, b"payload").expect("write reconcile source");

        let mut app = test_app();
        app.state
            .try_open_file_manager_with(|_| Some(crate::fm::FmState::new(&destination)))
            .expect("Files activation");
        let now = Instant::now();
        assert!(!app.sync_file_manager_watcher_at(now));
        let watcher_generation = app.file_manager_watcher.generation();

        app.state.file_manager_clipboard = vec![source];
        assert!(app.dispatch_file_manager_header_action(FileManagerHeaderAction::Paste));
        let deadline = Instant::now() + Duration::from_secs(5);
        while !app.file_operation_worker.has_buffered_completion() {
            assert!(Instant::now() < deadline, "reconcile worker timed out");
            std::thread::sleep(Duration::from_millis(5));
        }

        let (watch_tx, watch_rx) = sync_channel(1);
        watch_tx
            .send(message(
                watcher_generation,
                destination
                    .join("payload.txt")
                    .to_str()
                    .expect("UTF-8 temp path"),
            ))
            .expect("queue matching watcher event");
        app.file_manager_watcher.receiver = Some(watch_rx);

        let before_generation = app
            .state
            .file_manager
            .as_ref()
            .expect("file manager open")
            .preview_generation;
        assert!(app.sync_file_operation_worker());
        let _ = app.sync_file_manager_watcher_at(now);

        let file_manager = app.state.file_manager.as_ref().expect("file manager open");
        assert_eq!(
            file_manager.preview_generation,
            before_generation + 1,
            "worker completion and its watcher event must coalesce into one reload"
        );
        assert_eq!(
            file_manager
                .entries
                .iter()
                .filter(|entry| entry.name == "payload.txt")
                .count(),
            1
        );

        let external = destination.join("external.txt");
        std::fs::write(&external, b"external").expect("write external change");
        watch_tx
            .send(message(
                watcher_generation,
                external.to_str().expect("UTF-8 external temp path"),
            ))
            .expect("queue external watcher event");
        assert!(app.sync_file_manager_watcher_at(now));
        let file_manager = app.state.file_manager.as_ref().expect("file manager open");
        assert_eq!(file_manager.preview_generation, before_generation + 2);
        assert!(file_manager
            .entries
            .iter()
            .any(|entry| entry.path == external));
    }

    // P5 WATCHER/APPLY: one accepted event prepares and applies exactly one
    // stable-path refresh; the drained second sync cannot churn generations.
    #[test]
    fn current_watcher_refresh_reconciles_by_stable_path() {
        let td = TempDir::new("typed-refresh-once");
        let selected = td.root.join("b.txt");
        std::fs::write(td.root.join("a.txt"), b"a").expect("write a");
        std::fs::write(&selected, b"b").expect("write b");
        let mut file_manager = crate::fm::FmState::new(&td.root);
        let selected_index = file_manager
            .entries
            .iter()
            .position(|entry| entry.path == selected)
            .expect("selected path");
        assert!(file_manager.select(selected_index));

        let mut app = test_app();
        app.state
            .try_open_file_manager_with(|_| Some(file_manager))
            .expect("Files activation");
        let now = Instant::now();
        assert!(!app.sync_file_manager_watcher_at(now));
        let watcher_generation = app.file_manager_watcher.generation();
        let before = app
            .state
            .file_manager
            .as_ref()
            .map(|file_manager| {
                (
                    file_manager.directory_generation,
                    file_manager.preview_generation,
                    app.file_manager_watcher.reconcile_revision,
                )
            })
            .expect("file manager open");

        let inserted = td.root.join("aa.txt");
        std::fs::write(&inserted, b"aa").expect("write inserted path");
        let (watch_tx, watch_rx) = sync_channel(1);
        watch_tx
            .send(message(
                watcher_generation,
                inserted.to_str().expect("UTF-8 inserted temp path"),
            ))
            .expect("queue watcher event");
        app.file_manager_watcher.receiver = Some(watch_rx);

        assert!(app.sync_file_manager_watcher_at(now));
        let refreshed = app.state.file_manager.as_ref().expect("file manager open");
        assert_eq!(
            refreshed.selected().map(|entry| entry.path.as_path()),
            Some(selected.as_path())
        );
        assert_eq!(refreshed.directory_generation, before.0 + 1);
        assert_eq!(refreshed.preview_generation, before.1 + 1);
        assert_eq!(app.file_manager_watcher.reconcile_revision, before.2 + 1);
        assert!(refreshed.entries.iter().any(|entry| entry.path == inserted));

        assert!(!app.sync_file_manager_watcher_at(now));
        let stable = app.state.file_manager.as_ref().expect("file manager open");
        assert_eq!(stable.directory_generation, before.0 + 1);
        assert_eq!(stable.preview_generation, before.1 + 1);
        assert_eq!(app.file_manager_watcher.reconcile_revision, before.2 + 1);
        drop(watch_tx);
    }

    // TP-TRAIL-T7-CHAR-01 / P5 LIFECYCLE: model generations can repeat after
    // reopen, so the typed request must also carry the Stage-owned Files
    // instance generation.
    #[test]
    fn prepared_refresh_cannot_apply_after_files_close_reopen() {
        let td = TempDir::new("refresh-close-reopen");
        std::fs::write(td.root.join("before.txt"), b"before").expect("write initial entry");
        let mut app = test_app();
        app.state
            .try_open_file_manager_with(|_| Some(crate::fm::FmState::new(&td.root)))
            .expect("Files activation");
        let files_generation = app
            .active_files_generation()
            .expect("active Files generation");
        let request = app
            .state
            .file_manager
            .as_ref()
            .expect("file manager open")
            .request_current_refresh(files_generation);
        std::fs::write(td.root.join("stale.txt"), b"stale").expect("write stale payload entry");
        let prepared = crate::fm::prepare_current_refresh_io(request);

        app.state.close_file_manager();
        app.state
            .try_open_file_manager_with(|_| Some(crate::fm::FmState::new(&td.root)))
            .expect("Files reopen");
        let reopened_generation = app
            .active_files_generation()
            .expect("reopened Files generation");
        assert_ne!(reopened_generation, files_generation);
        let before = app
            .state
            .file_manager
            .as_ref()
            .map(|file_manager| {
                (
                    file_manager.entries.clone(),
                    file_manager.directory_generation,
                    file_manager.preview_generation,
                    file_manager.miller.clone(),
                )
            })
            .expect("reopened file manager");

        assert!(!app.apply_prepared_file_manager_refresh(prepared));
        let reopened = app
            .state
            .file_manager
            .as_ref()
            .expect("reopened file manager");
        assert_eq!(reopened.entries, before.0);
        assert_eq!(reopened.directory_generation, before.1);
        assert_eq!(reopened.preview_generation, before.2);
        assert_eq!(reopened.miller, before.3);
    }

    // TP-C4.4-RECONCILE: when the watcher has already reconciled the exact
    // published operation before its worker returns, terminal completion must
    // not invalidate the same current FM generation a second time.
    #[test]
    fn watcher_reconcile_after_publish_owns_later_worker_completion() {
        let td = TempDir::new("watcher-before-completion");
        let source_dir = td.root.join("source");
        let destination = td.root.join("destination");
        std::fs::create_dir(&source_dir).expect("create source directory");
        std::fs::create_dir(&destination).expect("create destination directory");
        let source = source_dir.join("payload.txt");
        std::fs::write(&source, b"payload").expect("write source");
        let (published_tx, published_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let worker = super::super::file_operation_worker::FileOperationWorker::with_executor(
            Arc::new(Notify::new()),
            move |plan, cancellation| {
                let result = execute_file_operation(plan, cancellation);
                published_tx.send(()).expect("signal published operation");
                release_rx.recv().expect("release completed operation");
                result
            },
        );

        let mut app = test_app();
        app.file_operation_worker = worker;
        app.state
            .try_open_file_manager_with(|_| Some(crate::fm::FmState::new(&destination)))
            .expect("Files activation");
        let now = Instant::now();
        assert!(!app.sync_file_manager_watcher_at(now));
        let watcher_generation = app.file_manager_watcher.generation();
        app.state.file_manager_clipboard = vec![source];
        assert!(app.dispatch_file_manager_header_action(FileManagerHeaderAction::Paste));
        published_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("operation published before completion");

        let before_generation = app
            .state
            .file_manager
            .as_ref()
            .expect("file manager open")
            .preview_generation;
        let (watch_tx, watch_rx) = sync_channel(1);
        watch_tx
            .send(message(
                watcher_generation,
                destination
                    .join("payload.txt")
                    .to_str()
                    .expect("UTF-8 temp path"),
            ))
            .expect("queue post-publish watcher event");
        app.file_manager_watcher.receiver = Some(watch_rx);
        assert!(app.sync_file_manager_watcher_at(now));

        release_tx.send(()).expect("release worker completion");
        let deadline = Instant::now() + Duration::from_secs(5);
        while !app.file_operation_worker.has_buffered_completion() {
            assert!(Instant::now() < deadline, "completion buffering timed out");
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(app.sync_file_operation_worker());
        let _ = app.sync_file_manager_watcher_at(now);

        let file_manager = app.state.file_manager.as_ref().expect("file manager open");
        assert_eq!(
            file_manager.preview_generation,
            before_generation + 1,
            "post-publish watcher reconciliation must own later completion"
        );
        assert_eq!(
            file_manager
                .entries
                .iter()
                .filter(|entry| entry.name == "payload.txt")
                .count(),
            1
        );
    }

    // TP-C4.4-RECONCILE: native backends may deliver the operation's own
    // debounced event after terminal completion already forced convergence.
    // That exact delayed burst must not invalidate preview state twice.
    #[test]
    fn delayed_own_operation_watcher_event_does_not_reload_twice() {
        let td = TempDir::new("delayed-own-event");
        let source_dir = td.root.join("source");
        let destination = td.root.join("destination");
        std::fs::create_dir(&source_dir).expect("create source directory");
        std::fs::create_dir(&destination).expect("create destination directory");
        let source = source_dir.join("payload.txt");
        std::fs::write(&source, b"payload").expect("write source");

        let mut app = test_app();
        app.state
            .try_open_file_manager_with(|_| Some(crate::fm::FmState::new(&destination)))
            .expect("Files activation");
        let now = Instant::now();
        assert!(!app.sync_file_manager_watcher_at(now));
        let watcher_generation = app.file_manager_watcher.generation();
        app.state.file_manager_clipboard = vec![source];
        assert!(app.dispatch_file_manager_header_action(FileManagerHeaderAction::Paste));
        let deadline = Instant::now() + Duration::from_secs(5);
        while !app.file_operation_worker.has_buffered_completion() {
            assert!(Instant::now() < deadline, "completion buffering timed out");
            std::thread::sleep(Duration::from_millis(5));
        }

        let before_generation = app
            .state
            .file_manager
            .as_ref()
            .expect("file manager open")
            .preview_generation;
        assert!(app.sync_file_operation_worker());
        let _ = app.sync_file_manager_watcher_at(now);

        let (watch_tx, watch_rx) = sync_channel(1);
        watch_tx
            .send(message(
                watcher_generation,
                destination
                    .join("payload.txt")
                    .to_str()
                    .expect("UTF-8 temp path"),
            ))
            .expect("queue delayed own-operation event");
        app.file_manager_watcher.receiver = Some(watch_rx);
        let _ = app.sync_file_manager_watcher_at(now);

        let file_manager = app.state.file_manager.as_ref().expect("file manager open");
        assert_eq!(
            file_manager.preview_generation,
            before_generation + 1,
            "delayed own-operation watcher event must be absorbed"
        );
        assert_eq!(
            file_manager
                .entries
                .iter()
                .filter(|entry| entry.name == "payload.txt")
                .count(),
            1
        );
    }

    // TP-C4.4-RECONCILE: closing and reopening the same cwd creates a new
    // watcher generation and FM instance. A completion bound to the prior
    // generation may terminalize its operation, but cannot reload/project the
    // newly opened state merely because the path string matches again.
    #[test]
    fn prior_generation_completion_cannot_reload_reopened_same_cwd() {
        let td = TempDir::new("same-cwd-reopen");
        let source_dir = td.root.join("source");
        let destination = td.root.join("destination");
        std::fs::create_dir(&source_dir).expect("create source directory");
        std::fs::create_dir(&destination).expect("create destination directory");
        let source = source_dir.join("payload.txt");
        std::fs::write(&source, b"payload").expect("write source");
        let (started_tx, started_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let worker = super::super::file_operation_worker::FileOperationWorker::with_executor(
            Arc::new(Notify::new()),
            move |plan, cancellation| {
                started_tx.send(()).expect("signal operation started");
                release_rx.recv().expect("release stale generation");
                execute_file_operation(plan, cancellation)
            },
        );

        let mut app = test_app();
        app.file_operation_worker = worker;
        app.state
            .try_open_file_manager_with(|_| Some(crate::fm::FmState::new(&destination)))
            .expect("Files activation");
        let now = Instant::now();
        assert!(!app.sync_file_manager_watcher_at(now));
        let prior_watcher_generation = app.file_manager_watcher.generation();
        app.state.file_manager_clipboard = vec![source];
        assert!(app.dispatch_file_manager_header_action(FileManagerHeaderAction::Paste));
        started_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("operation started");

        app.state.file_manager = None;
        assert!(!app.sync_file_manager_watcher_at(now));
        app.state
            .try_open_file_manager_with(|_| Some(crate::fm::FmState::new(&destination)))
            .expect("Files activation");
        assert!(!app.sync_file_manager_watcher_at(now));
        assert!(app.file_manager_watcher.generation() > prior_watcher_generation);
        let reopened_generation = app
            .state
            .file_manager
            .as_ref()
            .expect("file manager reopened")
            .preview_generation;

        release_tx.send(()).expect("release prior generation");
        let deadline = Instant::now() + Duration::from_secs(5);
        while !app.file_operation_worker.has_buffered_completion() {
            assert!(Instant::now() < deadline, "stale completion timed out");
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(app.sync_file_operation_worker());
        let _ = app.sync_file_manager_watcher_at(now);

        let reopened = app
            .state
            .file_manager
            .as_ref()
            .expect("file manager reopened");
        assert_eq!(
            reopened.preview_generation, reopened_generation,
            "prior generation completion must not reload reopened same cwd"
        );
        assert!(
            reopened.entries.is_empty(),
            "prior generation completion must not project its published entry"
        );
        assert!(destination.join("payload.txt").exists());
    }

    // TP-C4.4-RECONCILE: degraded polling mode uses the same immediate
    // completion ownership instead of waiting for the periodic safety net or
    // starting a second scheduler.
    #[test]
    fn polling_fallback_completion_reconciles_once_without_early_retry() {
        let td = TempDir::new("operation-polling-fallback");
        let source_dir = td.root.join("source");
        let destination = td.root.join("destination");
        std::fs::create_dir(&source_dir).expect("create source directory");
        std::fs::create_dir(&destination).expect("create destination directory");
        let source = source_dir.join("payload.txt");
        std::fs::write(&source, b"payload").expect("write source");

        let mut app = test_app();
        app.state
            .try_open_file_manager_with(|_| Some(crate::fm::FmState::new(&destination)))
            .expect("Files activation");
        let now = Instant::now();
        assert!(!app.sync_file_manager_watcher_at(now));
        app.file_manager_watcher.mode = FileManagerWatcherMode::PollingFallback;
        let before_generation = app
            .state
            .file_manager
            .as_ref()
            .expect("file manager open")
            .preview_generation;
        app.state.file_manager_clipboard = vec![source];
        assert!(app.dispatch_file_manager_header_action(FileManagerHeaderAction::Paste));
        let deadline = Instant::now() + Duration::from_secs(5);
        while !app.file_operation_worker.has_buffered_completion() {
            assert!(Instant::now() < deadline, "polling completion timed out");
            std::thread::sleep(Duration::from_millis(5));
        }

        assert!(app.sync_file_operation_worker());
        assert!(app.sync_file_manager_watcher_at(now));
        assert!(!app.sync_file_manager_watcher_at(now));
        let file_manager = app.state.file_manager.as_ref().expect("file manager open");
        assert_eq!(file_manager.preview_generation, before_generation + 1);
        assert!(file_manager
            .entries
            .iter()
            .any(|entry| entry.name == "payload.txt"));
        assert_eq!(
            app.file_manager_watcher.mode(),
            FileManagerWatcherMode::PollingFallback
        );
    }

    // TP-TRAIL-T7-CHAR-04 / TP-C4.4-RECONCILE: watcher-driven rename removes
    // the exact selected path from both cursor and multi-selection authority,
    // then falls back to the old safe row without retaining a stale anchor.
    #[test]
    fn watcher_rename_prunes_selected_path_and_keeps_cursor_safe() {
        let td = TempDir::new("watcher-selection-rename");
        for name in ["a.txt", "b.txt", "c.txt"] {
            std::fs::write(td.root.join(name), name).expect("write selection fixture");
        }
        let old_path = td.root.join("b.txt");
        let new_path = td.root.join("z.txt");
        let mut file_manager = crate::fm::FmState::new(&td.root);
        let old_index = file_manager
            .entries
            .iter()
            .position(|entry| entry.path == old_path)
            .expect("selected path index");
        assert!(file_manager.select(old_index));
        assert!(file_manager.replace_selection(old_index));

        let mut app = test_app();
        app.state
            .try_open_file_manager_with(|_| Some(file_manager))
            .expect("Files activation");
        let now = Instant::now();
        assert!(!app.sync_file_manager_watcher_at(now));
        let watcher_generation = app.file_manager_watcher.generation();
        std::fs::rename(&old_path, &new_path).expect("rename selected path");
        let (watch_tx, watch_rx) = sync_channel(1);
        watch_tx
            .send(message(
                watcher_generation,
                new_path.to_str().expect("UTF-8 renamed temp path"),
            ))
            .expect("queue rename watcher event");
        app.file_manager_watcher.receiver = Some(watch_rx);

        assert!(app.sync_file_manager_watcher_at(now));
        let file_manager = app.state.file_manager.as_ref().expect("file manager open");
        assert_eq!(file_manager.cursor, old_index);
        assert_eq!(
            file_manager.selected().map(|entry| entry.name.as_str()),
            Some("c.txt")
        );
        assert!(!file_manager.multi_selection_paths().contains(&old_path));
        assert!(file_manager.multi_selection_paths().is_empty());
        assert!(file_manager.multi_selection_anchor().is_none());
    }

    #[test]
    fn ancestor_revalidation_does_not_create_persistent_watcher() {
        let tree = TempDir::new("ancestor-watcher");
        let child = tree.root.join("child");
        std::fs::create_dir(&child).expect("create watched child");
        let mut app = test_app();

        assert!(!app.sync_file_manager_watcher());
        assert!(!app.file_manager_watcher.has_backend());

        app.state
            .try_open_file_manager_with(|_| Some(crate::fm::FmState::new(&child)))
            .expect("Files activation");
        assert!(!app.sync_file_manager_watcher());
        assert_eq!(
            app.file_manager_watcher.watched_dir(),
            Some(child.as_path())
        );
        assert!(app.file_manager_watcher.has_backend());
        let child_generation = app.file_manager_watcher.generation();

        assert!(!app.sync_file_manager_watcher());
        assert_eq!(app.file_manager_watcher.generation(), child_generation);

        app.state
            .file_manager
            .as_mut()
            .expect("file manager open")
            .leave();
        assert_eq!(
            app.state
                .file_manager
                .as_ref()
                .map(|file_manager| file_manager.cwd.as_path()),
            Some(tree.root.as_path())
        );
        assert!(!app.sync_file_manager_watcher());
        assert_eq!(
            app.file_manager_watcher.watched_dir(),
            Some(tree.root.as_path())
        );
        assert!(app.file_manager_watcher.has_backend());
        assert!(app.file_manager_watcher.generation() > child_generation);
        let ancestor_generation = app.file_manager_watcher.generation();

        assert!(
            !app.sync_file_manager_watcher(),
            "ancestor projection revalidation keeps the single current watcher stable"
        );
        assert_eq!(
            app.file_manager_watcher.generation(),
            ancestor_generation,
            "no second persistent ancestor watcher may be created"
        );

        app.state.file_manager = None;
        assert!(!app.sync_file_manager_watcher());
        assert_eq!(app.file_manager_watcher.watched_dir(), None);
        assert!(!app.file_manager_watcher.has_backend());
    }

    #[test]
    fn app_sync_uses_latched_polling_fallback_without_panicking_or_hot_retrying() {
        let td = TempDir::new("missing");
        let missing = td.root.join("does-not-exist");
        let mut app = test_app();
        app.state
            .try_open_file_manager_with(|_| Some(crate::fm::FmState::new(&missing)))
            .expect("Files activation");

        assert!(!app.sync_file_manager_watcher());
        assert_eq!(
            app.file_manager_watcher.watched_dir(),
            Some(missing.as_path())
        );
        assert!(app.file_manager_watcher.has_backend());
        assert_eq!(
            app.file_manager_watcher.mode(),
            FileManagerWatcherMode::PollingFallback
        );
        let failed_generation = app.file_manager_watcher.generation();

        assert!(!app.sync_file_manager_watcher());
        assert_eq!(app.file_manager_watcher.generation(), failed_generation);
        assert!(app.file_manager_watcher.has_backend());
        assert_eq!(
            app.file_manager_watcher.mode(),
            FileManagerWatcherMode::PollingFallback
        );
    }

    #[test]
    fn reconciliation_deadline_is_bounded_and_repeats_without_early_polling() {
        let td = TempDir::new("poll-deadline");
        let missing = td.root.join("does-not-exist");
        let wake = Arc::new(Notify::new());
        let mut watcher = NativeFileManagerWatcher::new(wake);
        let started_at = Instant::now();

        assert!(matches!(
            watcher.sync(Some(&missing), started_at),
            FmWatcherSync::Started { .. }
        ));
        assert_eq!(watcher.mode(), FileManagerWatcherMode::PollingFallback);
        let first_deadline = watcher
            .next_reconcile_at()
            .expect("active watcher has reconciliation deadline");
        assert!(first_deadline >= started_at + WATCH_RECONCILE_INTERVAL);
        assert!(!watcher.take_reconcile_due(
            first_deadline
                .checked_sub(Duration::from_nanos(1))
                .expect("deadline after instant origin")
        ));
        assert!(watcher.take_reconcile_due(first_deadline));

        let second_deadline = watcher
            .next_reconcile_at()
            .expect("reconciliation deadline repeats");
        assert!(second_deadline > first_deadline);
        assert!(!watcher.take_reconcile_due(first_deadline));
    }

    #[test]
    fn polling_fallback_converges_when_missing_directory_appears_without_dirty_loop() {
        let td = TempDir::new("poll-converge");
        let missing = td.root.join("appears-later");
        let mut app = test_app();
        app.state
            .try_open_file_manager_with(|_| Some(crate::fm::FmState::new(&missing)))
            .expect("Files activation");
        let started_at = Instant::now();

        assert!(!app.sync_file_manager_watcher_at(started_at));
        assert_eq!(
            app.file_manager_watcher.mode(),
            FileManagerWatcherMode::PollingFallback
        );
        std::fs::create_dir_all(&missing).expect("create previously missing directory");
        std::fs::write(missing.join("arrived.txt"), b"arrived").expect("write fallback file");

        let first_deadline = app
            .file_manager_watcher
            .next_reconcile_at()
            .expect("fallback deadline");
        assert!(app.sync_file_manager_watcher_at(first_deadline));
        assert!(app
            .state
            .file_manager
            .as_ref()
            .expect("file manager open")
            .entries
            .iter()
            .any(|entry| entry.name == "arrived.txt"));

        let unchanged_deadline = app
            .file_manager_watcher
            .next_reconcile_at()
            .expect("next fallback deadline");
        assert!(
            !app.sync_file_manager_watcher_at(unchanged_deadline),
            "unchanged reconciliation must not dirty the render loop"
        );
    }

    #[test]
    fn runtime_error_and_channel_disconnect_degrade_native_mode_to_polling() {
        let wake = Arc::new(Notify::new());
        let mut watcher = NativeFileManagerWatcher::new(wake);
        watcher
            .slot
            .sync_with(Some(Path::new("/work")), |_, _| {
                Ok::<_, ()>(NativeFmBackend::PollingFallback)
            })
            .expect("activate deterministic watcher slot");
        watcher.mode = FileManagerWatcherMode::Native;
        assert_eq!(watcher.mode(), FileManagerWatcherMode::Native);

        let (error_tx, error_rx) = sync_channel(1);
        error_tx
            .send(watch_message_from_result(
                watcher.generation(),
                Err(vec![notify_debouncer_full::notify::Error::generic(
                    "runtime failure",
                )]),
            ))
            .expect("inject watcher runtime error");
        watcher.receiver = Some(error_rx);
        let error_drain = watcher.drain();
        assert_eq!(
            error_drain.events.errors,
            vec!["runtime failure".to_string()]
        );
        assert_eq!(watcher.mode(), FileManagerWatcherMode::PollingFallback);

        let (closed_tx, closed_rx) = sync_channel(1);
        drop(closed_tx);
        watcher.mode = FileManagerWatcherMode::Native;
        watcher.receiver = Some(closed_rx);
        let closed_drain = watcher.drain();
        assert!(closed_drain.disconnected);
        assert_eq!(watcher.mode(), FileManagerWatcherMode::PollingFallback);
    }

    #[test]
    fn native_watcher_converges_create_rename_delete_and_burst_changes() {
        let td = TempDir::new("real-fs");
        std::fs::write(td.root.join("anchor.txt"), b"anchor").expect("write anchor");
        let mut app = test_app();
        app.state
            .try_open_file_manager_with(|_| Some(crate::fm::FmState::new(&td.root)))
            .expect("Files activation");
        assert!(!app.sync_file_manager_watcher());
        assert!(app.file_manager_watcher.has_backend());

        std::fs::write(td.root.join("old.txt"), b"old").expect("create watched file");
        wait_for_file_manager_state(&mut app, "created file", |state| {
            state.entries.iter().any(|entry| entry.name == "old.txt")
        });

        std::fs::rename(td.root.join("old.txt"), td.root.join("new.txt"))
            .expect("rename watched file");
        wait_for_file_manager_state(&mut app, "renamed file", |state| {
            !state.entries.iter().any(|entry| entry.name == "old.txt")
                && state
                    .entries
                    .iter()
                    .filter(|entry| entry.name == "new.txt")
                    .count()
                    == 1
        });

        std::fs::remove_file(td.root.join("new.txt")).expect("delete watched file");
        wait_for_file_manager_state(&mut app, "deleted file", |state| {
            !state.entries.iter().any(|entry| entry.name == "new.txt")
        });

        let burst_names = (0..16)
            .map(|index| format!("{index:02}.txt"))
            .collect::<Vec<_>>();
        for name in &burst_names {
            std::fs::write(td.root.join(name), b"burst").expect("write burst file");
        }
        wait_for_file_manager_state(&mut app, "burst files", |state| {
            burst_names.iter().all(|name| {
                state
                    .entries
                    .iter()
                    .any(|entry| entry.name.as_str() == name)
            })
        });

        let state = app.state.file_manager.as_ref().expect("file manager open");
        assert_eq!(
            state.selected().map(|entry| entry.name.as_str()),
            Some("anchor.txt"),
            "real watcher refreshes preserve the selected path"
        );
        assert!(state.cursor < state.entries.len());
    }
}
