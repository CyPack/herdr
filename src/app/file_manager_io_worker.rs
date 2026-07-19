use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::thread::JoinHandle;

use tokio::sync::Notify;

pub(super) use crate::fm::{
    classify_root_navigation_error, prepare_root_navigation_io, FmPreparedRootNavigation,
    FmRootNavigationError, FmRootNavigationRequest,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct FileManagerIoSource {
    pub(super) directory: PathBuf,
    pub(super) directory_generation: u64,
    pub(super) preview_generation: u64,
    pub(super) miller_revision: u64,
    pub(super) show_hidden: bool,
}

impl FileManagerIoSource {
    pub(super) fn from_file_manager(file_manager: &crate::fm::FmState) -> Self {
        Self {
            directory: file_manager.cwd.clone(),
            directory_generation: file_manager.directory_generation,
            preview_generation: file_manager.preview_generation,
            miller_revision: file_manager.miller.revision,
            show_hidden: file_manager.show_hidden,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum FileManagerIoIdentity {
    Root {
        files_generation: u32,
        location_model_revision: u64,
        target_root: PathBuf,
    },
    Navigation {
        files_generation: u32,
        source: FileManagerIoSource,
        target_directory: PathBuf,
    },
    Refresh {
        files_generation: u32,
        source: FileManagerIoSource,
    },
}

impl FileManagerIoIdentity {
    pub(super) fn target_path(&self) -> Option<&Path> {
        match self {
            Self::Root { target_root, .. } => Some(target_root),
            Self::Navigation {
                target_directory, ..
            } => Some(target_directory),
            Self::Refresh { source, .. } => Some(&source.directory),
        }
    }

    pub(super) fn is_current(
        &self,
        files_generation: u32,
        location_model_revision: u64,
        source: Option<&FileManagerIoSource>,
    ) -> bool {
        match self {
            Self::Root {
                files_generation: expected_files_generation,
                location_model_revision: expected_model_revision,
                ..
            } => {
                *expected_files_generation == files_generation
                    && *expected_model_revision == location_model_revision
            }
            Self::Navigation {
                files_generation: expected_files_generation,
                source: expected_source,
                ..
            }
            | Self::Refresh {
                files_generation: expected_files_generation,
                source: expected_source,
            } => *expected_files_generation == files_generation && source == Some(expected_source),
        }
    }
}

#[derive(Debug, Clone)]
pub(super) enum FileManagerIoRequest {
    Root(FmRootNavigationRequest),
    Navigate {
        files_generation: u32,
        request: crate::fm::FmNavigationRequest,
    },
    Refresh(crate::fm::FmCurrentRefreshRequest),
}

impl FileManagerIoRequest {
    pub(super) fn identity(&self) -> FileManagerIoIdentity {
        match self {
            Self::Root(request) => FileManagerIoIdentity::Root {
                files_generation: request.files_generation,
                location_model_revision: request.location_model_revision,
                target_root: request.target_root.clone(),
            },
            Self::Navigate {
                files_generation,
                request,
            } => FileManagerIoIdentity::Navigation {
                files_generation: *files_generation,
                source: FileManagerIoSource {
                    directory: request.source_directory.clone(),
                    directory_generation: request.source_directory_generation,
                    preview_generation: request.source_preview_generation,
                    miller_revision: request.source_miller_revision,
                    show_hidden: request.show_hidden,
                },
                target_directory: request.target_directory.clone(),
            },
            Self::Refresh(request) => FileManagerIoIdentity::Refresh {
                files_generation: request.files_generation,
                source: FileManagerIoSource {
                    directory: request.source_directory.clone(),
                    directory_generation: request.source_directory_generation,
                    preview_generation: request.source_preview_generation,
                    miller_revision: request.source_miller_revision,
                    show_hidden: request.source_show_hidden,
                },
            },
        }
    }
}

#[derive(Debug, Clone)]
pub(super) enum FileManagerIoOutcome {
    Root(Result<FmPreparedRootNavigation, FmRootNavigationError>),
    Navigate {
        files_generation: u32,
        prepared: Option<crate::fm::FmPreparedNavigation>,
    },
    Refresh(crate::fm::FmPreparedCurrentRefresh),
    Panicked(FileManagerIoIdentity),
}

impl FileManagerIoOutcome {
    #[cfg(test)]
    fn for_request(self, _request: &FileManagerIoRequest) -> Self {
        self
    }
}

#[derive(Debug, Clone)]
pub(super) struct FileManagerIoResult {
    pub(super) generation: u64,
    pub(super) identity: FileManagerIoIdentity,
    pub(super) outcome: FileManagerIoOutcome,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FileManagerIoSubmit {
    Accepted {
        generation: u64,
        replaced_pending: bool,
    },
    Disconnected,
}

#[derive(Debug, Default)]
pub(super) struct FileManagerIoDrain {
    pub(super) current: Option<FileManagerIoResult>,
    pub(super) disconnected: bool,
}

#[derive(Debug)]
struct FileManagerIoWork {
    generation: u64,
    request: FileManagerIoRequest,
}

#[derive(Debug)]
struct FileManagerIoWorkerState {
    pending: Option<FileManagerIoWork>,
    result: Option<FileManagerIoResult>,
    alive: bool,
    closed: bool,
}

impl Default for FileManagerIoWorkerState {
    fn default() -> Self {
        Self {
            pending: None,
            result: None,
            alive: true,
            closed: false,
        }
    }
}

type SharedWorkerState = Arc<(Mutex<FileManagerIoWorkerState>, Condvar)>;

struct WorkerAliveGuard {
    shared: SharedWorkerState,
    wake: Arc<Notify>,
}

impl Drop for WorkerAliveGuard {
    fn drop(&mut self) {
        let (state, changed) = &*self.shared;
        lock_state(state).alive = false;
        changed.notify_all();
        self.wake.notify_one();
    }
}

pub(super) struct FileManagerIoWorker {
    shared: SharedWorkerState,
    handle: Option<JoinHandle<()>>,
    latest_generation: u64,
    disconnect_reported: bool,
}

impl FileManagerIoWorker {
    pub(super) fn new(wake: Arc<Notify>) -> Self {
        Self::with_processor(wake, process_request)
    }

    fn with_processor<F>(wake: Arc<Notify>, processor: F) -> Self
    where
        F: Fn(FileManagerIoRequest) -> FileManagerIoOutcome + Send + 'static,
    {
        let shared = Arc::new((
            Mutex::new(FileManagerIoWorkerState::default()),
            Condvar::new(),
        ));
        let worker_shared = shared.clone();
        let handle = std::thread::spawn(move || {
            let _alive_guard = WorkerAliveGuard {
                shared: worker_shared.clone(),
                wake: wake.clone(),
            };
            while let Some(work) = take_next_request(&worker_shared) {
                let identity = work.request.identity();
                let outcome = catch_unwind(AssertUnwindSafe(|| processor(work.request)))
                    .unwrap_or_else(|_| FileManagerIoOutcome::Panicked(identity.clone()));
                let result = FileManagerIoResult {
                    generation: work.generation,
                    identity,
                    outcome,
                };
                let (state, changed) = &*worker_shared;
                let mut state = lock_state(state);
                if state.closed {
                    break;
                }
                state.result = Some(result);
                drop(state);
                changed.notify_all();
                wake.notify_one();
            }
        });

        Self {
            shared,
            handle: Some(handle),
            latest_generation: 0,
            disconnect_reported: false,
        }
    }

    pub(super) fn submit(&mut self, request: FileManagerIoRequest) -> FileManagerIoSubmit {
        let (state, changed) = &*self.shared;
        let mut state = lock_state(state);
        if state.closed || !state.alive {
            return FileManagerIoSubmit::Disconnected;
        }
        self.latest_generation = self.latest_generation.wrapping_add(1).max(1);
        let replaced_pending = state.pending.replace(FileManagerIoWork {
            generation: self.latest_generation,
            request,
        });
        changed.notify_one();
        FileManagerIoSubmit::Accepted {
            generation: self.latest_generation,
            replaced_pending: replaced_pending.is_some(),
        }
    }

    pub(super) fn drain(&mut self) -> FileManagerIoDrain {
        let (state, _) = &*self.shared;
        let mut state = lock_state(state);
        let result = state.result.take();
        let disconnected = !state.alive && !state.closed && !self.disconnect_reported;
        self.disconnect_reported |= disconnected;
        drop(state);

        FileManagerIoDrain {
            current: result.filter(|result| result.generation == self.latest_generation),
            disconnected,
        }
    }

    #[cfg(test)]
    fn wait_for_result_for_test(&self) {
        let (state, changed) = &*self.shared;
        let mut state = lock_state(state);
        while state.result.is_none() && state.alive && !state.closed {
            state = match changed.wait(state) {
                Ok(state) => state,
                Err(poisoned) => poisoned.into_inner(),
            };
        }
    }
}

impl Drop for FileManagerIoWorker {
    fn drop(&mut self) {
        let (state, changed) = &*self.shared;
        let mut state = lock_state(state);
        state.closed = true;
        state.pending = None;
        state.result = None;
        drop(state);
        changed.notify_all();

        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn take_next_request(shared: &SharedWorkerState) -> Option<FileManagerIoWork> {
    let (state, changed) = &**shared;
    let mut state = lock_state(state);
    while state.pending.is_none() && !state.closed {
        state = match changed.wait(state) {
            Ok(state) => state,
            Err(poisoned) => poisoned.into_inner(),
        };
    }
    if state.closed {
        return None;
    }
    state.pending.take()
}

fn lock_state(state: &Mutex<FileManagerIoWorkerState>) -> MutexGuard<'_, FileManagerIoWorkerState> {
    match state.lock() {
        Ok(state) => state,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn process_request(request: FileManagerIoRequest) -> FileManagerIoOutcome {
    match request {
        FileManagerIoRequest::Root(request) => {
            FileManagerIoOutcome::Root(prepare_root_navigation_io(request))
        }
        FileManagerIoRequest::Navigate {
            files_generation,
            request,
        } => FileManagerIoOutcome::Navigate {
            files_generation,
            prepared: crate::fm::prepare_navigation_io(request),
        },
        FileManagerIoRequest::Refresh(request) => {
            FileManagerIoOutcome::Refresh(crate::fm::prepare_current_refresh_io(request))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Condvar, Mutex};

    use tokio::sync::Notify;

    use super::{
        classify_root_navigation_error, prepare_root_navigation_io, process_request,
        FileManagerIoIdentity, FileManagerIoOutcome, FileManagerIoRequest, FileManagerIoSource,
        FileManagerIoSubmit, FileManagerIoWorker, FmRootNavigationError, FmRootNavigationRequest,
    };

    #[derive(Default)]
    struct Gate {
        open: Mutex<bool>,
        changed: Condvar,
    }

    impl Gate {
        fn wait(&self) {
            let mut open = self.open.lock().unwrap();
            while !*open {
                open = self.changed.wait(open).unwrap();
            }
        }

        fn release(&self) {
            *self.open.lock().unwrap() = true;
            self.changed.notify_all();
        }
    }

    fn root_request(
        files_generation: u32,
        location_model_revision: u64,
        target_root: &str,
    ) -> FileManagerIoRequest {
        FileManagerIoRequest::Root(FmRootNavigationRequest {
            files_generation,
            location_model_revision,
            target_root: PathBuf::from(target_root),
            show_hidden: false,
        })
    }

    struct TempDir {
        root: PathBuf,
    }

    impl TempDir {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "herdr-fcl-io-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            std::fs::create_dir_all(&root).unwrap();
            Self { root }
        }

        fn dir(&self, name: &str) -> PathBuf {
            let path = self.root.join(name);
            std::fs::create_dir_all(&path).unwrap();
            path
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    // TP-FCL-IO-01: submission cannot run the directory processor on the
    // caller. A deterministically blocked processor leaves existing model
    // data readable and submit returns before the gate opens.
    #[test]
    fn fcl_io_submit_is_non_blocking_while_processor_is_blocked() {
        let gate = Arc::new(Gate::default());
        let worker_gate = gate.clone();
        let (started_tx, started_rx) = std::sync::mpsc::channel();
        let mut worker =
            FileManagerIoWorker::with_processor(Arc::new(Notify::new()), move |request| {
                started_tx.send(()).unwrap();
                worker_gate.wait();
                FileManagerIoOutcome::Root(Err(FmRootNavigationError::Unavailable))
                    .for_request(&request)
            });
        let old_trail = vec![PathBuf::from("/already/rendered")];

        assert_eq!(
            worker.submit(root_request(7, 11, "/blocked")),
            FileManagerIoSubmit::Accepted {
                generation: 1,
                replaced_pending: false,
            }
        );
        started_rx.recv().unwrap();
        assert_eq!(old_trail, vec![PathBuf::from("/already/rendered")]);

        gate.release();
        worker.wait_for_result_for_test();
        assert!(worker.drain().current.is_some());
    }

    // TP-FCL-IO-02: while request 1 executes, request 3 replaces request 2.
    // The queue never grows beyond one executing plus one latest pending.
    #[test]
    fn fcl_io_worker_keeps_only_latest_pending_request() {
        let gate = Arc::new(Gate::default());
        let worker_gate = gate.clone();
        let (started_tx, started_rx) = std::sync::mpsc::channel();
        let mut worker =
            FileManagerIoWorker::with_processor(Arc::new(Notify::new()), move |request| {
                let target = request.identity().target_path().unwrap().to_path_buf();
                started_tx.send(target.clone()).unwrap();
                if target == Path::new("/one") {
                    worker_gate.wait();
                }
                FileManagerIoOutcome::Root(Err(FmRootNavigationError::Unavailable))
                    .for_request(&request)
            });

        assert_eq!(
            worker.submit(root_request(1, 1, "/one")),
            FileManagerIoSubmit::Accepted {
                generation: 1,
                replaced_pending: false,
            }
        );
        assert_eq!(started_rx.recv().unwrap(), PathBuf::from("/one"));
        assert_eq!(
            worker.submit(root_request(1, 1, "/two")),
            FileManagerIoSubmit::Accepted {
                generation: 2,
                replaced_pending: false,
            }
        );
        assert_eq!(
            worker.submit(root_request(1, 1, "/three")),
            FileManagerIoSubmit::Accepted {
                generation: 3,
                replaced_pending: true,
            }
        );

        gate.release();
        assert_eq!(started_rx.recv().unwrap(), PathBuf::from("/three"));
        worker.wait_for_result_for_test();
        let result = worker.drain().current.unwrap();
        assert_eq!(result.generation, 3);
        assert_eq!(result.identity.target_path(), Some(Path::new("/three")));
    }

    // TP-FCL-IO-03: application authority compares the latest worker ticket,
    // Files instance, location-model revision, and exact source identities.
    #[test]
    fn fcl_io_result_rejects_every_stale_identity_axis() {
        let source = FileManagerIoSource {
            directory: PathBuf::from("/root/current"),
            directory_generation: 13,
            preview_generation: 17,
            miller_revision: 19,
            show_hidden: false,
        };
        let identity = FileManagerIoIdentity::Root {
            files_generation: 5,
            location_model_revision: 23,
            target_root: PathBuf::from("/root"),
        };

        assert!(identity.is_current(5, 23, Some(&source)));
        assert!(!identity.is_current(6, 23, Some(&source)));
        assert!(!identity.is_current(5, 24, Some(&source)));

        let navigation = FileManagerIoIdentity::Navigation {
            files_generation: 5,
            source: source.clone(),
            target_directory: PathBuf::from("/root/current/child"),
        };
        assert!(navigation.is_current(5, 23, Some(&source)));
        let mut changed_source = source;
        changed_source.miller_revision += 1;
        assert!(!navigation.is_current(5, 23, Some(&changed_source)));
    }

    // TP-FCL-IO-04: a processor panic is converted to a typed result and the
    // same lane remains usable by a later generation.
    #[test]
    fn fcl_io_worker_recovers_after_processor_panic() {
        let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let worker_calls = calls.clone();
        let mut worker =
            FileManagerIoWorker::with_processor(Arc::new(Notify::new()), move |request| {
                if worker_calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst) == 0 {
                    panic!("injected file-manager processor panic");
                }
                FileManagerIoOutcome::Root(Err(FmRootNavigationError::Missing))
                    .for_request(&request)
            });

        assert!(matches!(
            worker.submit(root_request(2, 3, "/panic")),
            FileManagerIoSubmit::Accepted { generation: 1, .. }
        ));
        worker.wait_for_result_for_test();
        let first = worker.drain().current.unwrap();
        assert!(matches!(
            first.outcome,
            FileManagerIoOutcome::Panicked(FileManagerIoIdentity::Root { .. })
        ));

        assert!(matches!(
            worker.submit(root_request(2, 3, "/recovered")),
            FileManagerIoSubmit::Accepted { generation: 2, .. }
        ));
        worker.wait_for_result_for_test();
        let second = worker.drain().current.unwrap();
        assert!(matches!(
            second.outcome,
            FileManagerIoOutcome::Root(Err(FmRootNavigationError::Missing))
        ));
    }

    #[test]
    fn prepare_root_navigation_classifies_success_missing_changed_type_and_permission() {
        let td = TempDir::new();
        let request = FmRootNavigationRequest {
            files_generation: 4,
            location_model_revision: 9,
            target_root: td.root.clone(),
            show_hidden: false,
        };
        let prepared = prepare_root_navigation_io(request.clone()).unwrap();
        assert_eq!(prepared.request, request);
        assert_eq!(prepared.file_manager.cwd, td.root);

        let missing = FmRootNavigationRequest {
            target_root: td.root.join("missing"),
            ..request.clone()
        };
        assert!(matches!(
            prepare_root_navigation_io(missing),
            Err(FmRootNavigationError::Missing)
        ));

        let file = td.root.join("plain-file");
        std::fs::write(&file, b"not a directory").unwrap();
        let changed_type = FmRootNavigationRequest {
            target_root: file,
            ..request
        };
        assert!(matches!(
            prepare_root_navigation_io(changed_type),
            Err(FmRootNavigationError::ChangedType)
        ));
        assert_eq!(
            classify_root_navigation_error(std::io::ErrorKind::PermissionDenied),
            FmRootNavigationError::PermissionDenied
        );
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

    fn location_model(path: &Path) -> crate::app::state::FileManagerSidebarModel {
        crate::app::state::FileManagerSidebarModel::from_sources(
            Vec::new(),
            vec![crate::app::state::FileManagerSidebarItem {
                label: "Target".into(),
                path: path.to_path_buf(),
                icon: crate::app::state::FileManagerSidebarIcon::Pin,
                accessible: true,
                ejectable: false,
            }],
            Vec::new(),
        )
    }

    // TP-FCL-IO-01/03: App root submission preserves the rendered Trail while
    // blocked, then applies only to the still-current Files/model identity.
    #[test]
    fn fcl_io_location_request_is_async_and_generation_safe() {
        let td = TempDir::new();
        let initial = td.dir("initial");
        let target = td.dir("target");
        std::fs::write(target.join("visible.txt"), b"visible").unwrap();
        let mut app = test_app();
        app.state
            .try_open_file_manager_with(|_| Some(crate::fm::FmState::new(&initial)))
            .unwrap();
        app.state.file_manager_sidebar = location_model(&target);
        app.state.request_file_manager_sidebar_navigation = Some(target.clone());

        let gate = Arc::new(Gate::default());
        let worker_gate = gate.clone();
        let (started_tx, started_rx) = std::sync::mpsc::channel();
        app.file_manager_io_worker =
            FileManagerIoWorker::with_processor(Arc::new(Notify::new()), move |request| {
                started_tx.send(()).unwrap();
                worker_gate.wait();
                process_request(request)
            });

        assert!(app.sync_file_manager_location_request());
        started_rx.recv().unwrap();
        assert_eq!(
            app.state.file_manager.as_ref().unwrap().cwd,
            initial,
            "the old Trail remains available while root I/O is blocked"
        );

        gate.release();
        app.file_manager_io_worker.wait_for_result_for_test();
        assert!(app.sync_file_manager_io_results());
        assert_eq!(app.state.file_manager.as_ref().unwrap().cwd, target);
    }

    #[test]
    fn fcl_io_root_completion_after_close_reopen_is_rejected() {
        let td = TempDir::new();
        let initial = td.dir("initial");
        let target = td.dir("target");
        let reopened = td.dir("reopened");
        let mut app = test_app();
        app.state
            .try_open_file_manager_with(|_| Some(crate::fm::FmState::new(&initial)))
            .unwrap();
        app.state.file_manager_sidebar = location_model(&target);
        app.state.request_file_manager_sidebar_navigation = Some(target);

        let gate = Arc::new(Gate::default());
        let worker_gate = gate.clone();
        let (started_tx, started_rx) = std::sync::mpsc::channel();
        app.file_manager_io_worker =
            FileManagerIoWorker::with_processor(Arc::new(Notify::new()), move |request| {
                started_tx.send(()).unwrap();
                worker_gate.wait();
                process_request(request)
            });
        assert!(app.sync_file_manager_location_request());
        started_rx.recv().unwrap();

        app.state.close_file_manager();
        app.state
            .try_open_file_manager_with(|_| Some(crate::fm::FmState::new(&reopened)))
            .unwrap();
        gate.release();
        app.file_manager_io_worker.wait_for_result_for_test();
        assert!(!app.sync_file_manager_io_results());
        assert_eq!(app.state.file_manager.as_ref().unwrap().cwd, reopened);
    }

    // TP-FCL-IO-05: clicking the already resident Trail root resets it from
    // prepared snapshots and never invokes the directory processor.
    #[test]
    fn fcl_io_resident_root_activation_performs_zero_worker_reads() {
        let td = TempDir::new();
        let child = td.dir("child");
        let file_manager = crate::fm::FmState::open_trail_to(&td.root, &child, false).unwrap();
        let mut app = test_app();
        app.state
            .try_open_file_manager_with(|_| Some(file_manager))
            .unwrap();
        app.state.file_manager_sidebar = location_model(&td.root);
        app.state.request_file_manager_sidebar_navigation = Some(td.root.clone());

        let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let worker_calls = calls.clone();
        app.file_manager_io_worker =
            FileManagerIoWorker::with_processor(Arc::new(Notify::new()), move |request| {
                worker_calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                process_request(request)
            });

        assert!(app.sync_file_manager_location_request());
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 0);
        let file_manager = app.state.file_manager.as_ref().unwrap();
        assert_eq!(file_manager.cwd, td.root);
        assert_eq!(file_manager.trail.cols().len(), 1);
    }

    // TP-FCL-IO-06: Miller navigation and current refresh both cross the
    // worker boundary; their processor never runs on the caller thread.
    #[test]
    fn fcl_io_miller_and_current_refresh_share_worker_lane() {
        let td = TempDir::new();
        td.dir("child");
        let mut app = test_app();
        app.state
            .try_open_file_manager_with(|_| Some(crate::fm::FmState::new(&td.root)))
            .unwrap();
        let files_generation = app.state.stage.active_instance_generation().unwrap();
        let caller = std::thread::current().id();
        let (processor_tx, processor_rx) = std::sync::mpsc::channel();
        app.file_manager_io_worker =
            FileManagerIoWorker::with_processor(Arc::new(Notify::new()), move |request| {
                processor_tx.send(std::thread::current().id()).unwrap();
                process_request(request)
            });

        let navigation = app
            .state
            .file_manager
            .as_ref()
            .and_then(crate::fm::FmState::request_enter_navigation)
            .unwrap();
        assert!(app.execute_file_manager_navigation(navigation));
        assert_ne!(processor_rx.recv().unwrap(), caller);
        app.file_manager_io_worker.wait_for_result_for_test();
        assert!(app.sync_file_manager_io_results());

        let refresh = app
            .state
            .file_manager
            .as_ref()
            .unwrap()
            .request_operation_refresh(files_generation);
        assert_eq!(
            app.execute_file_manager_current_refresh(refresh),
            Some(false)
        );
        assert_ne!(processor_rx.recv().unwrap(), caller);
        app.file_manager_io_worker.wait_for_result_for_test();
        let _ = app.sync_file_manager_io_results();
    }

    #[test]
    fn fcl_io_watcher_adapter_has_no_synchronous_directory_refresh() {
        let source = include_str!("file_manager_watcher.rs");
        let start = source
            .find("pub(super) fn sync_file_manager_watcher_at")
            .expect("watcher scheduled adapter");
        let end = source[start..]
            .find("\n}\n\n#[cfg(test)]")
            .map(|offset| start + offset)
            .expect("watcher production impl boundary");
        let production = &source[start..end];

        assert!(
            !production.contains(".refresh_active_trail_col("),
            "watcher scheduled sync must not enumerate a Trail column inline"
        );
        assert!(
            production.contains("FileManagerIoRequest::TrailRefresh"),
            "watcher refresh must submit the shared bounded I/O lane"
        );
    }
}
