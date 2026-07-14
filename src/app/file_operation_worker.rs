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
}
