use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::thread::JoinHandle;

use tokio::sync::Notify;

use crate::fm::operations::{
    execute_file_operation, FileOperationCancellation, FileOperationExecutionResult,
    FileOperationPlan,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FileOperationStartError {
    Busy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FileOperationWorkerError {
    Panicked,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct FileOperationWorkerCompletion {
    pub(super) generation: u64,
    pub(super) result: Result<FileOperationExecutionResult, FileOperationWorkerError>,
}

#[derive(Debug, Default)]
pub(super) struct FileOperationWorkerDrain {
    pub(super) completion: Option<FileOperationWorkerCompletion>,
    pub(super) disconnected: bool,
}

struct FileOperationWorkerRequest {
    generation: u64,
    plan: FileOperationPlan,
    cancellation: FileOperationCancellation,
}

struct FileOperationWorkerState {
    pending: Option<FileOperationWorkerRequest>,
    completion: Option<FileOperationWorkerCompletion>,
    active_generation: Option<u64>,
    active_cancellation: Option<FileOperationCancellation>,
    next_generation: u64,
    alive: bool,
    closed: bool,
}

impl Default for FileOperationWorkerState {
    fn default() -> Self {
        Self {
            pending: None,
            completion: None,
            active_generation: None,
            active_cancellation: None,
            next_generation: 0,
            alive: true,
            closed: false,
        }
    }
}

type SharedWorkerState = Arc<(Mutex<FileOperationWorkerState>, Condvar)>;

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

pub(super) struct FileOperationWorker {
    shared: SharedWorkerState,
    handle: Option<JoinHandle<()>>,
}

impl FileOperationWorker {
    pub(super) fn new(wake: Arc<Notify>) -> Self {
        Self::with_executor(wake, execute_file_operation)
    }

    fn with_executor<F>(wake: Arc<Notify>, executor: F) -> Self
    where
        F: Fn(&FileOperationPlan, &FileOperationCancellation) -> FileOperationExecutionResult
            + Send
            + 'static,
    {
        let shared = Arc::new((
            Mutex::new(FileOperationWorkerState::default()),
            Condvar::new(),
        ));
        let worker_shared = shared.clone();
        let handle = std::thread::Builder::new()
            .name("herdr-fm-operation".into())
            .spawn(move || {
                let _alive_guard = WorkerAliveGuard {
                    shared: worker_shared.clone(),
                    wake: wake.clone(),
                };
                while let Some(request) = take_next_request(&worker_shared) {
                    let result = catch_unwind(AssertUnwindSafe(|| {
                        executor(&request.plan, &request.cancellation)
                    }))
                    .map_err(|_| FileOperationWorkerError::Panicked);
                    let (state, _) = &*worker_shared;
                    let mut state = lock_state(state);
                    if state.closed {
                        break;
                    }
                    state.completion = Some(FileOperationWorkerCompletion {
                        generation: request.generation,
                        result,
                    });
                    drop(state);
                    wake.notify_one();
                }
            })
            .ok();
        if handle.is_none() {
            let (state, _) = &*shared;
            lock_state(state).alive = false;
        }
        Self { shared, handle }
    }

    pub(super) fn start(
        &mut self,
        plan: FileOperationPlan,
    ) -> Result<u64, FileOperationStartError> {
        let (state, pending) = &*self.shared;
        let mut state = lock_state(state);
        if state.closed
            || !state.alive
            || state.active_generation.is_some()
            || state.pending.is_some()
            || state.completion.is_some()
        {
            return Err(FileOperationStartError::Busy);
        }
        state.next_generation = state.next_generation.wrapping_add(1).max(1);
        let generation = state.next_generation;
        let cancellation = FileOperationCancellation::default();
        state.active_generation = Some(generation);
        state.active_cancellation = Some(cancellation.clone());
        state.pending = Some(FileOperationWorkerRequest {
            generation,
            plan,
            cancellation,
        });
        pending.notify_one();
        Ok(generation)
    }

    pub(super) fn is_busy(&self) -> bool {
        let (state, _) = &*self.shared;
        lock_state(state).active_generation.is_some()
    }

    pub(super) fn cancel(&self) -> bool {
        let cancellation = {
            let (state, _) = &*self.shared;
            lock_state(state).active_cancellation.clone()
        };
        if let Some(cancellation) = cancellation {
            cancellation.cancel();
            true
        } else {
            false
        }
    }

    pub(super) fn drain(&mut self) -> FileOperationWorkerDrain {
        let (state, _) = &*self.shared;
        let mut state = lock_state(state);
        let completion = state.completion.take();
        if let Some(completion) = &completion {
            if state.active_generation == Some(completion.generation) {
                state.active_generation = None;
                state.active_cancellation = None;
            }
        }
        FileOperationWorkerDrain {
            disconnected: !state.alive && completion.is_none(),
            completion,
        }
    }
}

impl Drop for FileOperationWorker {
    fn drop(&mut self) {
        let cancellation = {
            let (state, pending) = &*self.shared;
            let mut state = lock_state(state);
            state.closed = true;
            state.pending = None;
            pending.notify_all();
            state.active_cancellation.clone()
        };
        if let Some(cancellation) = cancellation {
            cancellation.cancel();
        }
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn take_next_request(shared: &SharedWorkerState) -> Option<FileOperationWorkerRequest> {
    let (state, pending) = &**shared;
    let mut state = lock_state(state);
    loop {
        if state.closed {
            return None;
        }
        if let Some(request) = state.pending.take() {
            return Some(request);
        }
        state = match pending.wait(state) {
            Ok(state) => state,
            Err(poisoned) => poisoned.into_inner(),
        };
    }
}

fn lock_state(state: &Mutex<FileOperationWorkerState>) -> MutexGuard<'_, FileOperationWorkerState> {
    match state.lock() {
        Ok(state) => state,
        Err(poisoned) => poisoned.into_inner(),
    }
}

#[cfg(test)]
mod tests {
    use super::{FileOperationStartError, FileOperationWorker, FileOperationWorkerError};
    use crate::app::state::{
        FileManagerContextActionIntent, FileManagerContextMenuAction, FileManagerHeaderAction,
        FileManagerOperationStatus,
    };
    use crate::fm::operations::{
        execute_file_operation, FileOperationCancellation, FileOperationExecutionStatus,
        FileOperationKind, FileOperationPlan, FileOperationRequest,
    };
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;
    use std::time::{Duration, Instant};
    use tokio::sync::Notify;

    fn unique() -> u64 {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        COUNTER.fetch_add(1, Ordering::Relaxed)
    }

    struct TempDir {
        root: PathBuf,
    }

    impl TempDir {
        fn new(tag: &str) -> Self {
            let root = std::env::temp_dir().join(format!(
                "herdr-fm-operation-worker-test-{}-{}-{}",
                std::process::id(),
                tag,
                unique()
            ));
            fs::create_dir_all(&root).expect("create isolated worker test root");
            Self { root }
        }

        fn copy_plan(&self, name: &str) -> FileOperationPlan {
            let source = self.root.join(format!("source-{name}.txt"));
            fs::write(&source, name.as_bytes()).expect("write worker source");
            let destination = self.root.join(format!("destination-{name}"));
            fs::create_dir(&destination).expect("create worker destination");
            FileOperationPlan::preflight(FileOperationRequest {
                kind: FileOperationKind::Copy,
                sources: vec![source],
                destination_directory: destination,
                operation_in_flight: false,
            })
            .expect("worker copy plan")
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn wait_for_completion(
        worker: &mut FileOperationWorker,
    ) -> super::FileOperationWorkerCompletion {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let drained = worker.drain();
            if let Some(completion) = drained.completion {
                return completion;
            }
            assert!(
                !drained.disconnected,
                "worker disconnected before completion"
            );
            assert!(Instant::now() < deadline, "worker completion timed out");
            std::thread::sleep(Duration::from_millis(5));
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

    // TP-C4.1-LIFECYCLE: one bounded lane rejects concurrent work and reopens
    // only after the current generation reaches one terminal completion.
    #[test]
    fn file_operation_worker_is_single_lane_and_generation_safe() {
        let td = TempDir::new("single-lane");
        let wake = Arc::new(Notify::new());
        let mut worker = FileOperationWorker::new(wake);
        let first_generation = worker.start(td.copy_plan("first")).expect("start first");

        assert!(worker.is_busy());
        assert_eq!(
            worker.start(td.copy_plan("rejected")),
            Err(FileOperationStartError::Busy)
        );

        let first = wait_for_completion(&mut worker);
        assert_eq!(first.generation, first_generation);
        assert_eq!(
            first.result.expect("first execution").status(),
            FileOperationExecutionStatus::Completed
        );
        assert!(!worker.is_busy());

        let second_generation = worker.start(td.copy_plan("second")).expect("start second");
        assert!(second_generation > first_generation);
        let second = wait_for_completion(&mut worker);
        assert_eq!(second.generation, second_generation);
        assert_eq!(
            second.result.expect("second execution").status(),
            FileOperationExecutionStatus::Completed
        );
    }

    // TP-C4.1-LIFECYCLE: cancellation is idempotent, wakes the worker, and
    // produces a single Cancelled terminal result without filesystem output.
    #[test]
    fn file_operation_worker_cancel_is_idempotent_and_terminal() {
        let td = TempDir::new("cancel");
        let wake = Arc::new(Notify::new());
        let mut worker = FileOperationWorker::with_executor(wake, |plan, cancellation| {
            while !cancellation.is_cancelled() {
                std::thread::yield_now();
            }
            execute_file_operation(plan, cancellation)
        });
        let plan = td.copy_plan("cancelled");
        let destination = plan.destination_directory().to_path_buf();
        let generation = worker.start(plan).expect("start cancellable work");

        assert!(worker.cancel());
        assert!(worker.cancel());
        let completion = wait_for_completion(&mut worker);

        assert_eq!(completion.generation, generation);
        assert_eq!(
            completion.result.expect("cancel result").status(),
            FileOperationExecutionStatus::Cancelled
        );
        assert!(fs::read_dir(destination)
            .expect("read cancelled destination")
            .next()
            .is_none());
        assert!(!worker.is_busy());
        assert!(!worker.cancel(), "idle cancel is a no-op");
    }

    // TP-C4.1-LIFECYCLE: a panic is converted to an explicit terminal error;
    // it cannot strand the lane or poison the next generation.
    #[test]
    fn file_operation_worker_converts_panics_and_accepts_next_generation() {
        let td = TempDir::new("panic");
        let wake = Arc::new(Notify::new());
        let mut worker = FileOperationWorker::with_executor(
            wake,
            |_plan: &FileOperationPlan, _cancellation: &FileOperationCancellation| {
                panic!("injected operation panic")
            },
        );
        let first_generation = worker
            .start(td.copy_plan("panic-one"))
            .expect("start panic");

        let completion = wait_for_completion(&mut worker);
        assert_eq!(completion.generation, first_generation);
        assert_eq!(completion.result, Err(FileOperationWorkerError::Panicked));
        assert!(!worker.is_busy());

        let second_generation = worker
            .start(td.copy_plan("panic-two"))
            .expect("restart lane");
        assert!(second_generation > first_generation);
        let completion = wait_for_completion(&mut worker);
        assert_eq!(completion.generation, second_generation);
        assert_eq!(completion.result, Err(FileOperationWorkerError::Panicked));
    }

    // TP-C4.1-LIFECYCLE: Copy consumes the current live selection identity
    // only. Preparing clipboard content performs no filesystem mutation and
    // does not create an operation generation.
    #[test]
    fn app_copy_action_prepares_exact_selection_without_filesystem_work() {
        let td = TempDir::new("app-copy-action");
        let first = td.root.join("first.txt");
        let second = td.root.join("second.txt");
        fs::write(&first, b"first").expect("write first selection fixture");
        fs::write(&second, b"second").expect("write second selection fixture");
        let mut app = test_app();
        let mut file_manager = crate::fm::FmState::new(&td.root);
        let first_idx = file_manager
            .entries
            .iter()
            .position(|entry| entry.path == first)
            .expect("first selection row");
        let second_idx = file_manager
            .entries
            .iter()
            .position(|entry| entry.path == second)
            .expect("second selection row");
        assert!(file_manager.replace_selection(first_idx));
        assert!(file_manager.toggle_selection(second_idx));
        app.state.file_manager = Some(file_manager);
        let before_first = fs::read(&first).expect("read first before copy action");
        let before_second = fs::read(&second).expect("read second before copy action");

        assert!(app.dispatch_file_manager_header_action(FileManagerHeaderAction::Copy));

        assert_eq!(
            app.state.file_manager_clipboard,
            vec![first.clone(), second.clone()]
        );
        assert!(app.state.file_manager_operation.is_none());
        assert_eq!(
            fs::read(first).expect("read first after copy action"),
            before_first
        );
        assert_eq!(
            fs::read(second).expect("read second after copy action"),
            before_second
        );
    }

    // TP-C4.1-LIFECYCLE/WATCHER: Paste starts one background generation,
    // rejects concurrent work, publishes the copy, reaches one terminal state,
    // and explicitly reloads only the destination currently shown by the FM.
    #[test]
    fn app_paste_is_single_lane_and_completion_reloads_matching_destination() {
        let td = TempDir::new("app-paste-lifecycle");
        let source_root = td.root.join("sources");
        let destination = td.root.join("destination");
        fs::create_dir(&source_root).expect("create source root");
        fs::create_dir(&destination).expect("create destination root");
        let source = source_root.join("payload.txt");
        fs::write(&source, b"payload").expect("write paste source");
        let (started_tx, started_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let worker = FileOperationWorker::with_executor(
            Arc::new(Notify::new()),
            move |plan, cancellation| {
                started_tx.send(()).expect("signal paste started");
                release_rx.recv().expect("release paste");
                execute_file_operation(plan, cancellation)
            },
        );
        let mut app = test_app();
        app.file_operation_worker = worker;
        app.state.file_manager = Some(crate::fm::FmState::new(&destination));
        app.state.file_manager_clipboard = vec![source.clone()];

        assert!(app.dispatch_file_manager_header_action(FileManagerHeaderAction::Paste));
        started_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("paste worker started");
        let generation = app
            .state
            .file_manager_operation
            .as_ref()
            .expect("running operation state")
            .generation;
        assert!(app
            .state
            .file_manager_operation
            .as_ref()
            .is_some_and(|operation| operation.status == FileManagerOperationStatus::Running));
        assert!(
            !app.dispatch_file_manager_header_action(FileManagerHeaderAction::Paste),
            "second paste must fail closed while the lane is occupied"
        );

        release_tx.send(()).expect("release paste worker");
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if app.sync_file_operation_worker() {
                if app
                    .state
                    .file_manager_operation
                    .as_ref()
                    .is_some_and(|operation| {
                        operation.status == FileManagerOperationStatus::Completed
                    })
                {
                    break;
                }
            }
            assert!(Instant::now() < deadline, "paste completion timed out");
            std::thread::sleep(Duration::from_millis(5));
        }

        let operation = app
            .state
            .file_manager_operation
            .as_ref()
            .expect("terminal operation state");
        assert_eq!(operation.generation, generation);
        assert_eq!(operation.completed_items, 1);
        assert_eq!(
            fs::read(destination.join("payload.txt")).expect("copied payload"),
            b"payload"
        );
        assert!(app
            .state
            .file_manager
            .as_ref()
            .expect("open destination")
            .entries
            .iter()
            .any(|entry| entry.path == destination.join("payload.txt")));
    }

    // TP-C4.1-LIFECYCLE: a completion for the prior destination may finish
    // after close/reopen, but it cannot reload or project entries into the new
    // file-manager cwd.
    #[test]
    fn app_reopen_rejects_prior_destination_projection() {
        let td = TempDir::new("app-reopen-stale-completion");
        let source_root = td.root.join("sources");
        let old_destination = td.root.join("old-destination");
        let reopened_destination = td.root.join("reopened-destination");
        fs::create_dir(&source_root).expect("create source root");
        fs::create_dir(&old_destination).expect("create old destination");
        fs::create_dir(&reopened_destination).expect("create reopened destination");
        let source = source_root.join("old-only.txt");
        fs::write(&source, b"old").expect("write stale source");
        fs::write(reopened_destination.join("current.txt"), b"current")
            .expect("write reopened fixture");
        let (started_tx, started_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let worker = FileOperationWorker::with_executor(
            Arc::new(Notify::new()),
            move |plan, cancellation| {
                started_tx.send(()).expect("signal stale operation started");
                release_rx.recv().expect("release stale operation");
                execute_file_operation(plan, cancellation)
            },
        );
        let mut app = test_app();
        app.file_operation_worker = worker;
        app.state.file_manager = Some(crate::fm::FmState::new(&old_destination));
        app.state.file_manager_clipboard = vec![source];
        assert!(app.dispatch_file_manager_header_action(FileManagerHeaderAction::Paste));
        started_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("stale operation started");

        app.state.file_manager = None;
        app.state.file_manager = Some(crate::fm::FmState::new(&reopened_destination));
        release_tx.send(()).expect("release stale operation");
        let deadline = Instant::now() + Duration::from_secs(5);
        while app
            .state
            .file_manager_operation
            .as_ref()
            .is_some_and(|operation| operation.status == FileManagerOperationStatus::Running)
        {
            let _ = app.sync_file_operation_worker();
            assert!(Instant::now() < deadline, "stale completion timed out");
            std::thread::sleep(Duration::from_millis(5));
        }

        let reopened = app.state.file_manager.as_ref().expect("reopened FM");
        assert_eq!(reopened.cwd, reopened_destination);
        assert_eq!(reopened.entries.len(), 1);
        assert_eq!(
            reopened.entries[0].path,
            reopened_destination.join("current.txt")
        );
        assert!(old_destination.join("old-only.txt").exists());
    }

    // TP-C4.1-LIFECYCLE: C3 context-menu Copy converges on the same exact,
    // revalidated clipboard authority as the persistent header action.
    #[test]
    fn app_consumes_revalidated_context_copy_intent() {
        let td = TempDir::new("app-context-copy");
        let source = td.root.join("selected.txt");
        fs::write(&source, b"selected").expect("write context source");
        let mut file_manager = crate::fm::FmState::new(&td.root);
        assert!(file_manager.replace_selection(0));
        let mut app = test_app();
        app.state.file_manager = Some(file_manager);
        app.state.request_file_manager_context_action = Some(FileManagerContextActionIntent {
            action: FileManagerContextMenuAction::Copy,
            paths: vec![source.clone()],
        });

        assert!(app.sync_file_operation_worker());

        assert_eq!(app.state.file_manager_clipboard, vec![source]);
        assert!(app.state.request_file_manager_context_action.is_none());
        assert!(app.state.file_manager_operation.is_none());
    }
}
