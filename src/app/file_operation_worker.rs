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
