use std::collections::BTreeSet;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::Path;
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::thread::JoinHandle;

use tokio::sync::Notify;

use crate::fm::delete::{
    execute_delete_operation_with_observer, DeleteOperationExecutionResult,
    DeleteOperationExecutionStatus, DeleteOperationItemOutcome, DeleteOperationKind,
    DeleteOperationPlan, DeleteOperationRequest,
};
use crate::fm::operations::{
    execute_file_operation_with_observer, FileOperationCancellation, FileOperationExecutionResult,
    FileOperationExecutionStatus, FileOperationItemOutcome, FileOperationKind, FileOperationPlan,
    FileOperationRequest,
};
use crate::fm::rename::{
    execute_bulk_rename_operation_with_observer, execute_rename_operation_with_observer,
    BulkRenameItemOutcome, BulkRenameOperationExecutionResult, BulkRenameOperationExecutionStatus,
    BulkRenameOperationPlan, BulkRenameOperationRequest, RenameOperationExecutionResult,
    RenameOperationExecutionStatus, RenameOperationOutcome, RenameOperationPlan,
    RenameOperationRequest,
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
    pub(super) result: Result<FileOperationWorkerResult, FileOperationWorkerError>,
}

pub(super) struct FileOperationReconcileBaseline {
    operation_generation: u64,
    destination_directory: std::path::PathBuf,
    watcher_generation: u64,
    watcher_revision: u64,
    affected_paths: BTreeSet<std::path::PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct FileOperationWorkerProgress {
    pub(super) generation: u64,
    pub(super) active_item_index: usize,
    pub(super) started_items: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum FileOperationWorkerResult {
    Transfer(FileOperationExecutionResult),
    Delete(DeleteOperationExecutionResult),
    Rename(RenameOperationExecutionResult),
    BulkRename(BulkRenameOperationExecutionResult),
}

#[derive(Debug, Default)]
pub(super) struct FileOperationWorkerDrain {
    pub(super) progress: Option<FileOperationWorkerProgress>,
    pub(super) completion: Option<FileOperationWorkerCompletion>,
    pub(super) disconnected: bool,
    pub(super) generation_floor: u64,
}

struct FileOperationWorkerRequest {
    generation: u64,
    task: FileOperationWorkerTask,
    cancellation: FileOperationCancellation,
}

enum FileOperationWorkerTask {
    Transfer(FileOperationPlan),
    Delete(DeleteOperationPlan),
    Rename(RenameOperationPlan),
    BulkRename(BulkRenameOperationPlan),
}

struct FileOperationWorkerState {
    pending: Option<FileOperationWorkerRequest>,
    progress: Option<FileOperationWorkerProgress>,
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
            progress: None,
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
        Self::with_progress_task_executor(wake, execute_worker_task_with_progress)
    }

    fn new_after_generation(wake: Arc<Notify>, generation_floor: u64) -> Self {
        let worker = Self::new(wake);
        let (state, _) = &*worker.shared;
        lock_state(state).next_generation = generation_floor;
        worker
    }

    #[cfg(test)]
    pub(super) fn with_executor<F>(wake: Arc<Notify>, executor: F) -> Self
    where
        F: Fn(&FileOperationPlan, &FileOperationCancellation) -> FileOperationExecutionResult
            + Send
            + 'static,
    {
        Self::with_task_executor(wake, move |task, cancellation| match task {
            FileOperationWorkerTask::Transfer(plan) => {
                FileOperationWorkerResult::Transfer(executor(plan, cancellation))
            }
            FileOperationWorkerTask::Delete(plan) => FileOperationWorkerResult::Delete(
                crate::fm::delete::execute_delete_operation(plan, cancellation),
            ),
            FileOperationWorkerTask::Rename(plan) => FileOperationWorkerResult::Rename(
                crate::fm::rename::execute_rename_operation(plan, cancellation),
            ),
            FileOperationWorkerTask::BulkRename(plan) => FileOperationWorkerResult::BulkRename(
                crate::fm::rename::execute_bulk_rename_operation(plan, cancellation),
            ),
        })
    }

    #[cfg(test)]
    fn with_task_executor<F>(wake: Arc<Notify>, executor: F) -> Self
    where
        F: Fn(&FileOperationWorkerTask, &FileOperationCancellation) -> FileOperationWorkerResult
            + Send
            + 'static,
    {
        Self::with_progress_task_executor(wake, move |task, cancellation, _report_progress| {
            executor(task, cancellation)
        })
    }

    fn with_progress_task_executor<F>(wake: Arc<Notify>, executor: F) -> Self
    where
        F: Fn(
                &FileOperationWorkerTask,
                &FileOperationCancellation,
                &mut dyn FnMut(usize),
            ) -> FileOperationWorkerResult
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
                    let mut report_progress = |active_item_index: usize| {
                        let (state, _) = &*worker_shared;
                        let mut state = lock_state(state);
                        if state.closed
                            || state.active_generation != Some(request.generation)
                            || state.completion.is_some()
                        {
                            return;
                        }
                        let started_items = state
                            .progress
                            .filter(|progress| progress.generation == request.generation)
                            .map_or(active_item_index.saturating_add(1), |progress| {
                                progress
                                    .started_items
                                    .max(active_item_index.saturating_add(1))
                            });
                        state.progress = Some(FileOperationWorkerProgress {
                            generation: request.generation,
                            active_item_index,
                            started_items,
                        });
                        drop(state);
                        wake.notify_one();
                    };
                    let result = catch_unwind(AssertUnwindSafe(|| {
                        executor(&request.task, &request.cancellation, &mut report_progress)
                    }))
                    .map_err(|_| FileOperationWorkerError::Panicked);
                    let (state, _) = &*worker_shared;
                    let mut state = lock_state(state);
                    if state.closed {
                        break;
                    }
                    state.progress = None;
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
        self.start_task(FileOperationWorkerTask::Transfer(plan))
    }

    pub(super) fn start_delete(
        &mut self,
        plan: DeleteOperationPlan,
    ) -> Result<u64, FileOperationStartError> {
        self.start_task(FileOperationWorkerTask::Delete(plan))
    }

    pub(super) fn start_rename(
        &mut self,
        plan: RenameOperationPlan,
    ) -> Result<u64, FileOperationStartError> {
        self.start_task(FileOperationWorkerTask::Rename(plan))
    }

    pub(super) fn start_bulk_rename(
        &mut self,
        plan: BulkRenameOperationPlan,
    ) -> Result<u64, FileOperationStartError> {
        self.start_task(FileOperationWorkerTask::BulkRename(plan))
    }

    fn start_task(
        &mut self,
        task: FileOperationWorkerTask,
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
        state.progress = None;
        state.pending = Some(FileOperationWorkerRequest {
            generation,
            task,
            cancellation,
        });
        pending.notify_one();
        Ok(generation)
    }

    pub(super) fn is_busy(&self) -> bool {
        let (state, _) = &*self.shared;
        lock_state(state).active_generation.is_some()
    }

    #[cfg(test)]
    pub(super) fn has_buffered_completion(&self) -> bool {
        let (state, _) = &*self.shared;
        lock_state(state).completion.is_some()
    }

    #[cfg(test)]
    fn disconnected_after_progress_for_test(
        generation: u64,
        active_item_index: usize,
        started_items: usize,
    ) -> Self {
        let mut state = FileOperationWorkerState::default();
        state.alive = false;
        state.next_generation = generation;
        state.active_generation = Some(generation);
        state.active_cancellation = Some(FileOperationCancellation::default());
        state.progress = Some(FileOperationWorkerProgress {
            generation,
            active_item_index,
            started_items,
        });
        Self {
            shared: Arc::new((Mutex::new(state), Condvar::new())),
            handle: None,
        }
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

    fn cancel_generation(&self, generation: u64) -> bool {
        let cancellation = {
            let (state, _) = &*self.shared;
            let state = lock_state(state);
            if state.active_generation != Some(generation)
                || state
                    .completion
                    .as_ref()
                    .is_some_and(|completion| completion.generation == generation)
            {
                return false;
            }
            state.active_cancellation.clone()
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
        let progress = state.progress.take();
        let completion = state.completion.take();
        if let Some(completion) = &completion {
            if state.active_generation == Some(completion.generation) {
                state.active_generation = None;
                state.active_cancellation = None;
            }
        }
        FileOperationWorkerDrain {
            progress,
            disconnected: !state.alive && completion.is_none(),
            completion,
            generation_floor: state.next_generation,
        }
    }
}

fn execute_worker_task_with_progress(
    task: &FileOperationWorkerTask,
    cancellation: &FileOperationCancellation,
    report_progress: &mut dyn FnMut(usize),
) -> FileOperationWorkerResult {
    match task {
        FileOperationWorkerTask::Transfer(plan) => FileOperationWorkerResult::Transfer(
            execute_file_operation_with_observer(plan, cancellation, |event| {
                report_progress(event.item_index());
            }),
        ),
        FileOperationWorkerTask::Delete(plan) => FileOperationWorkerResult::Delete(
            execute_delete_operation_with_observer(plan, cancellation, report_progress),
        ),
        FileOperationWorkerTask::Rename(plan) => FileOperationWorkerResult::Rename(
            execute_rename_operation_with_observer(plan, cancellation, report_progress),
        ),
        FileOperationWorkerTask::BulkRename(plan) => FileOperationWorkerResult::BulkRename(
            execute_bulk_rename_operation_with_observer(plan, cancellation, report_progress),
        ),
    }
}

impl Drop for FileOperationWorker {
    fn drop(&mut self) {
        {
            let (state, pending) = &*self.shared;
            let mut state = lock_state(state);
            state.closed = true;
            state.pending = None;
            pending.notify_all();
        }
        let _ = self.cancel();
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

impl crate::app::App {
    pub(super) fn cancel_file_manager_operation(&self) -> bool {
        let Some(generation) = self
            .state
            .file_manager_operation
            .as_ref()
            .filter(|operation| operation.is_running())
            .map(|operation| operation.generation)
        else {
            return false;
        };
        self.file_operation_worker.cancel_generation(generation)
    }

    /// Dispatch one currently enabled native-FM header action. Copy only
    /// prepares exact path identities; Paste performs immutable preflight and
    /// hands the plan to the bounded worker before returning to the UI loop.
    pub(super) fn dispatch_file_manager_header_action(
        &mut self,
        action: crate::app::state::FileManagerHeaderAction,
    ) -> bool {
        use crate::app::state::FileManagerHeaderAction;

        match action {
            FileManagerHeaderAction::Copy => {
                let Some(paths) = current_action_paths(&self.state, action) else {
                    return false;
                };
                self.state.file_manager_clipboard = paths;
                true
            }
            FileManagerHeaderAction::Paste => self.start_file_manager_paste(),
            FileManagerHeaderAction::Delete => {
                let Some(paths) = current_action_paths(&self.state, action) else {
                    return false;
                };
                self.open_file_manager_delete_confirmation(paths)
            }
            FileManagerHeaderAction::NewFolder => false,
        }
    }

    fn start_file_manager_paste(&mut self) -> bool {
        use crate::app::state::{
            FileManagerHeaderAction, FileManagerOperationState, FileManagerOperationStatus,
        };

        if current_action_paths(&self.state, FileManagerHeaderAction::Paste).is_none() {
            return false;
        }
        let Some(destination_directory) = self
            .state
            .file_manager
            .as_ref()
            .map(|file_manager| file_manager.cwd.clone())
        else {
            return false;
        };
        let plan = match FileOperationPlan::preflight(FileOperationRequest {
            kind: FileOperationKind::Copy,
            sources: self.state.file_manager_clipboard.clone(),
            destination_directory: destination_directory.clone(),
            operation_in_flight: self.file_operation_worker.is_busy()
                || self
                    .state
                    .file_manager_operation
                    .as_ref()
                    .is_some_and(FileManagerOperationState::is_running),
        }) {
            Ok(plan) => plan,
            Err(error) => {
                tracing::warn!(?error, "fm: file operation preflight rejected paste");
                return false;
            }
        };
        let operation_kind = file_manager_operation_kind(plan.kind());
        let destination_directory = plan.destination_directory().to_path_buf();
        let affected_paths = plan
            .transfers()
            .iter()
            .map(|transfer| transfer.destination().to_path_buf())
            .collect();
        let total_items = plan.transfers().len();
        let items = plan
            .transfers()
            .iter()
            .map(
                |transfer| crate::app::state::FileManagerOperationItemState {
                    path: transfer.source().to_path_buf(),
                    recovery_path: None,
                    status: crate::app::state::FileManagerOperationItemStatus::Pending,
                },
            )
            .collect();
        let generation = match self.file_operation_worker.start(plan) {
            Ok(generation) => generation,
            Err(FileOperationStartError::Busy) => return false,
        };
        self.record_file_operation_reconcile_baseline(
            generation,
            &destination_directory,
            affected_paths,
        );
        self.state.file_manager_operation = Some(FileManagerOperationState {
            generation,
            kind: operation_kind,
            destination_directory,
            total_items,
            completed_items: 0,
            failed_items: 0,
            status: FileManagerOperationStatus::Running,
            items,
        });
        true
    }

    /// Consume C3's revalidated C4 intents and reconcile one worker terminal
    /// result. C5 context actions remain queued for their owning module
    /// instead of being silently discarded.
    pub(super) fn sync_file_operation_worker(&mut self) -> bool {
        let mut changed = self.consume_file_manager_delete_request();
        changed |= self.consume_file_manager_rename_request();
        changed |= self.consume_file_manager_bulk_rename_request();
        changed |= self.consume_file_manager_context_rename();
        changed |= self.consume_file_manager_context_delete();
        changed |= self.consume_file_manager_context_copy();
        let drained = self.file_operation_worker.drain();
        if let Some(progress) = drained.progress {
            if let Some(operation) = self.state.file_manager_operation.as_mut() {
                changed |= apply_worker_progress(operation, progress);
            }
        }
        if drained.disconnected {
            let operation_failed =
                self.state
                    .file_manager_operation
                    .as_mut()
                    .is_some_and(|operation| {
                        if !operation.is_running() {
                            return false;
                        }
                        mark_operation_worker_failure(operation);
                        true
                    });
            self.file_operation_reconcile_baseline = None;
            self.file_operation_worker = FileOperationWorker::new_after_generation(
                self.render_notify.clone(),
                drained.generation_floor,
            );
            if operation_failed {
                tracing::warn!("fm: file operation worker stopped before completion");
                return true;
            }
            return changed;
        }
        let Some(completion) = drained.completion else {
            return changed;
        };
        let Some(operation) = self.state.file_manager_operation.as_mut() else {
            return changed;
        };
        if !operation.is_running() || operation.generation != completion.generation {
            return changed;
        }

        let destination_directory = operation.destination_directory.clone();
        let reconcile_baseline = self
            .file_operation_reconcile_baseline
            .take()
            .filter(|baseline| {
                baseline.operation_generation == completion.generation
                    && baseline.destination_directory == destination_directory
            });
        match completion.result {
            Ok(FileOperationWorkerResult::Transfer(result)) => {
                apply_execution_result(operation, &result)
            }
            Ok(FileOperationWorkerResult::Delete(result)) => {
                apply_delete_execution_result(operation, &result)
            }
            Ok(FileOperationWorkerResult::Rename(result)) => {
                apply_rename_execution_result(operation, &result)
            }
            Ok(FileOperationWorkerResult::BulkRename(result)) => {
                apply_bulk_rename_execution_result(operation, &result)
            }
            Err(FileOperationWorkerError::Panicked) => {
                mark_operation_worker_failure(operation);
                tracing::error!(
                    generation = completion.generation,
                    "fm: file operation worker converted panic to terminal failure"
                );
            }
        }
        let had_reconcile_baseline = reconcile_baseline.is_some();
        let watcher_already_reconciled = reconcile_baseline.as_ref().is_some_and(|baseline| {
            self.file_manager_watcher.reconciled_since(
                &destination_directory,
                baseline.watcher_generation,
                baseline.watcher_revision,
            )
        });
        let owned_by_watcher = reconcile_baseline.as_ref().is_some_and(|baseline| {
            self.file_manager_watcher.own_operation_reconcile(
                &destination_directory,
                baseline.watcher_generation,
                baseline.affected_paths.clone(),
                !watcher_already_reconciled,
            )
        });
        let reconcile_with_watcher = owned_by_watcher
            || had_reconcile_baseline
            || self
                .state
                .file_manager
                .as_ref()
                .is_some_and(|file_manager| file_manager.cwd == destination_directory)
                && self
                    .file_manager_watcher
                    .request_reconcile(&destination_directory);
        if !reconcile_with_watcher {
            if let Some(file_manager) = self.state.file_manager.as_mut() {
                if file_manager.cwd == destination_directory {
                    file_manager.reload();
                }
            }
        }
        changed = true;
        changed
    }

    fn record_file_operation_reconcile_baseline(
        &mut self,
        operation_generation: u64,
        destination_directory: &Path,
        affected_paths: BTreeSet<std::path::PathBuf>,
    ) {
        self.file_operation_reconcile_baseline = self
            .file_manager_watcher
            .reconcile_snapshot(destination_directory)
            .map(
                |(watcher_generation, watcher_revision)| FileOperationReconcileBaseline {
                    operation_generation,
                    destination_directory: destination_directory.to_path_buf(),
                    watcher_generation,
                    watcher_revision,
                    affected_paths,
                },
            );
    }

    fn consume_file_manager_delete_request(&mut self) -> bool {
        let Some(request) = self.state.request_file_manager_delete.take() else {
            return false;
        };
        let _ = self.start_file_manager_delete(request);
        true
    }

    fn consume_file_manager_rename_request(&mut self) -> bool {
        let Some(request) = self.state.request_file_manager_rename.take() else {
            return false;
        };
        let _ = self.start_file_manager_rename(request);
        true
    }

    fn consume_file_manager_bulk_rename_request(&mut self) -> bool {
        let Some(request) = self.state.request_file_manager_bulk_rename.take() else {
            return false;
        };
        let _ = self.start_file_manager_bulk_rename(request);
        true
    }

    fn start_file_manager_rename(
        &mut self,
        request: crate::app::state::FileManagerRenameRequest,
    ) -> bool {
        use crate::app::state::{
            FileManagerOperationItemState, FileManagerOperationItemStatus,
            FileManagerOperationKind, FileManagerOperationState, FileManagerOperationStatus,
        };

        let Some(file_manager) = self.state.file_manager.as_ref() else {
            return false;
        };
        let affected_directory = file_manager.cwd.clone();
        if request.source_path.parent() != Some(affected_directory.as_path())
            || !file_manager
                .entries
                .iter()
                .any(|entry| entry.operation_supported && entry.path == request.source_path)
        {
            return false;
        }
        let operation_in_flight = self.file_operation_worker.is_busy()
            || self
                .state
                .file_manager_operation
                .as_ref()
                .is_some_and(FileManagerOperationState::is_running);
        let plan = match RenameOperationPlan::preflight(RenameOperationRequest {
            source_path: request.source_path,
            new_name: request.new_name,
            operation_in_flight,
        }) {
            Ok(plan) => plan,
            Err(error) => {
                tracing::warn!(?error, "fm: rename operation preflight rejected request");
                return false;
            }
        };
        let source = plan.source().to_path_buf();
        let affected_paths = [source.clone(), plan.destination().to_path_buf()]
            .into_iter()
            .collect();
        let generation = match self.file_operation_worker.start_rename(plan) {
            Ok(generation) => generation,
            Err(FileOperationStartError::Busy) => return false,
        };
        self.record_file_operation_reconcile_baseline(
            generation,
            &affected_directory,
            affected_paths,
        );
        self.state.file_manager_operation = Some(FileManagerOperationState {
            generation,
            kind: FileManagerOperationKind::Rename,
            destination_directory: affected_directory,
            total_items: 1,
            completed_items: 0,
            failed_items: 0,
            status: FileManagerOperationStatus::Running,
            items: vec![FileManagerOperationItemState {
                path: source,
                recovery_path: None,
                status: FileManagerOperationItemStatus::Pending,
            }],
        });
        true
    }

    fn start_file_manager_bulk_rename(
        &mut self,
        request: crate::app::state::FileManagerBulkRenameRequest,
    ) -> bool {
        use crate::app::state::{
            FileManagerOperationItemState, FileManagerOperationItemStatus,
            FileManagerOperationKind, FileManagerOperationState, FileManagerOperationStatus,
        };

        let Some(file_manager) = self.state.file_manager.as_ref() else {
            return false;
        };
        let affected_directory = file_manager.cwd.clone();
        let requested_sources = request
            .mappings
            .iter()
            .map(|(source, _)| source.clone())
            .collect::<Vec<_>>();
        let current_sources = file_manager
            .entries
            .iter()
            .filter(|entry| file_manager.multi_selection_paths().contains(&entry.path))
            .map(|entry| entry.path.clone())
            .collect::<Vec<_>>();
        if current_sources != requested_sources
            || file_manager.entries.iter().any(|entry| {
                file_manager.multi_selection_paths().contains(&entry.path)
                    && !entry.operation_supported
            })
        {
            return false;
        }
        let operation_in_flight = self.file_operation_worker.is_busy()
            || self
                .state
                .file_manager_operation
                .as_ref()
                .is_some_and(FileManagerOperationState::is_running);
        let plan = match BulkRenameOperationPlan::preflight(BulkRenameOperationRequest {
            mappings: request.mappings,
            operation_in_flight,
        }) {
            Ok(plan) => plan,
            Err(error) => {
                tracing::warn!(?error, "fm: bulk rename preflight rejected request");
                return false;
            }
        };
        let items = plan
            .mappings()
            .iter()
            .map(|(source, _)| FileManagerOperationItemState {
                path: source.clone(),
                recovery_path: None,
                status: FileManagerOperationItemStatus::Pending,
            })
            .collect::<Vec<_>>();
        let affected_paths = plan
            .mappings()
            .iter()
            .flat_map(|(source, destination)| [source.clone(), destination.clone()])
            .collect();
        let total_items = items.len();
        let generation = match self.file_operation_worker.start_bulk_rename(plan) {
            Ok(generation) => generation,
            Err(FileOperationStartError::Busy) => return false,
        };
        self.record_file_operation_reconcile_baseline(
            generation,
            &affected_directory,
            affected_paths,
        );
        self.state.file_manager_operation = Some(FileManagerOperationState {
            generation,
            kind: FileManagerOperationKind::BulkRename,
            destination_directory: affected_directory,
            total_items,
            completed_items: 0,
            failed_items: 0,
            status: FileManagerOperationStatus::Running,
            items,
        });
        true
    }

    fn start_file_manager_delete(
        &mut self,
        request: crate::app::state::FileManagerDeleteRequest,
    ) -> bool {
        use crate::app::state::{
            FileManagerDeleteKind, FileManagerHeaderAction, FileManagerOperationKind,
            FileManagerOperationState, FileManagerOperationStatus,
        };

        let Some(affected_directory) = self
            .state
            .file_manager
            .as_ref()
            .map(|file_manager| file_manager.cwd.clone())
        else {
            return false;
        };
        if current_action_paths(&self.state, FileManagerHeaderAction::Delete).as_ref()
            != Some(&request.paths)
        {
            return false;
        }
        let operation_in_flight = self.file_operation_worker.is_busy()
            || self
                .state
                .file_manager_operation
                .as_ref()
                .is_some_and(FileManagerOperationState::is_running);
        let (delete_kind, operation_kind) = match request.kind {
            FileManagerDeleteKind::Trash => {
                (DeleteOperationKind::Trash, FileManagerOperationKind::Trash)
            }
            FileManagerDeleteKind::Permanent => (
                DeleteOperationKind::Permanent,
                FileManagerOperationKind::PermanentDelete,
            ),
        };
        let plan = match DeleteOperationPlan::preflight(DeleteOperationRequest {
            kind: delete_kind,
            paths: request.paths,
            operation_in_flight,
        }) {
            Ok(plan) => plan,
            Err(error) => {
                tracing::warn!(?error, "fm: delete operation preflight rejected request");
                return false;
            }
        };
        let total_items = plan.items().len();
        let affected_paths = plan
            .items()
            .iter()
            .map(|item| item.path().to_path_buf())
            .collect();
        let items = plan
            .items()
            .iter()
            .map(|item| crate::app::state::FileManagerOperationItemState {
                path: item.path().to_path_buf(),
                recovery_path: None,
                status: crate::app::state::FileManagerOperationItemStatus::Pending,
            })
            .collect();
        let generation = match self.file_operation_worker.start_delete(plan) {
            Ok(generation) => generation,
            Err(FileOperationStartError::Busy) => return false,
        };
        self.record_file_operation_reconcile_baseline(
            generation,
            &affected_directory,
            affected_paths,
        );
        self.state.file_manager_operation = Some(FileManagerOperationState {
            generation,
            kind: operation_kind,
            destination_directory: affected_directory,
            total_items,
            completed_items: 0,
            failed_items: 0,
            status: FileManagerOperationStatus::Running,
            items,
        });
        true
    }

    fn consume_file_manager_context_copy(&mut self) -> bool {
        use crate::app::state::{FileManagerContextMenuAction, FileManagerHeaderAction};

        let is_copy = self
            .state
            .request_file_manager_context_action
            .as_ref()
            .is_some_and(|intent| intent.action == FileManagerContextMenuAction::Copy);
        if !is_copy {
            return false;
        }
        let Some(intent) = self.state.request_file_manager_context_action.take() else {
            return false;
        };
        if current_action_paths(&self.state, FileManagerHeaderAction::Copy)
            .is_some_and(|paths| paths == intent.paths)
        {
            self.state.file_manager_clipboard = intent.paths;
        }
        true
    }

    fn consume_file_manager_context_delete(&mut self) -> bool {
        use crate::app::state::FileManagerContextMenuAction;

        let is_delete = self
            .state
            .request_file_manager_context_action
            .as_ref()
            .is_some_and(|intent| intent.action == FileManagerContextMenuAction::Delete);
        if !is_delete {
            return false;
        }
        let Some(intent) = self.state.request_file_manager_context_action.take() else {
            return false;
        };
        let _ = self.open_file_manager_delete_confirmation(intent.paths);
        true
    }

    fn consume_file_manager_context_rename(&mut self) -> bool {
        use crate::app::state::FileManagerContextMenuAction;

        let is_rename = self
            .state
            .request_file_manager_context_action
            .as_ref()
            .is_some_and(|intent| intent.action == FileManagerContextMenuAction::Rename);
        if !is_rename {
            return false;
        }
        let Some(intent) = self.state.request_file_manager_context_action.take() else {
            return false;
        };
        let _ = self.open_file_manager_context_rename(intent.paths);
        true
    }
}

pub(super) fn current_action_paths(
    state: &crate::app::state::AppState,
    action: crate::app::state::FileManagerHeaderAction,
) -> Option<Vec<std::path::PathBuf>> {
    let file_manager = state.file_manager.as_ref()?;
    let action_bar = crate::ui::compute_file_manager_action_bar_model(
        file_manager,
        &state.file_manager_clipboard,
        state
            .file_manager_operation
            .as_ref()
            .is_some_and(crate::app::state::FileManagerOperationState::is_running),
    );
    action_bar
        .action_state(action)
        .is_some_and(|action_state| action_state.enabled)
        .then(|| {
            action_bar
                .selection
                .map(|selection| selection.paths)
                .unwrap_or_default()
        })
}

fn file_manager_operation_kind(
    kind: FileOperationKind,
) -> crate::app::state::FileManagerOperationKind {
    match kind {
        FileOperationKind::Copy => crate::app::state::FileManagerOperationKind::Copy,
        FileOperationKind::Move => crate::app::state::FileManagerOperationKind::Move,
    }
}

fn apply_worker_progress(
    operation: &mut crate::app::state::FileManagerOperationState,
    progress: FileOperationWorkerProgress,
) -> bool {
    use crate::app::state::FileManagerOperationItemStatus;

    if !operation.is_running() || operation.generation != progress.generation {
        return false;
    }
    let started_items = progress
        .started_items
        .min(operation.total_items)
        .min(operation.items.len());
    if started_items == 0 || progress.active_item_index >= started_items {
        return false;
    }

    let mut changed = false;
    for item in operation.items.iter_mut().take(started_items) {
        if item.status == FileManagerOperationItemStatus::Pending {
            item.status = FileManagerOperationItemStatus::Running;
            changed = true;
        }
    }
    changed
}

fn apply_execution_result(
    operation: &mut crate::app::state::FileManagerOperationState,
    result: &FileOperationExecutionResult,
) {
    use crate::app::state::FileManagerOperationStatus;

    let completed_items = result
        .items()
        .iter()
        .filter(|item| matches!(item.outcome(), FileOperationItemOutcome::Committed))
        .count();
    let source_retained = result
        .items()
        .iter()
        .filter(|item| matches!(item.outcome(), FileOperationItemOutcome::SourceRetained(_)))
        .count();
    let failed_items = result
        .items()
        .iter()
        .filter(|item| {
            matches!(
                item.outcome(),
                FileOperationItemOutcome::Failed(_) | FileOperationItemOutcome::SourceRetained(_)
            )
        })
        .count();
    operation.completed_items = completed_items;
    operation.failed_items = failed_items;
    operation.items = result
        .items()
        .iter()
        .map(|item| crate::app::state::FileManagerOperationItemState {
            path: item.source().to_path_buf(),
            recovery_path: None,
            status: match item.outcome() {
                FileOperationItemOutcome::NotStarted => {
                    crate::app::state::FileManagerOperationItemStatus::Pending
                }
                FileOperationItemOutcome::Committed => {
                    crate::app::state::FileManagerOperationItemStatus::Completed
                }
                FileOperationItemOutcome::SourceRetained(_) => {
                    crate::app::state::FileManagerOperationItemStatus::Retained
                }
                FileOperationItemOutcome::RolledBack | FileOperationItemOutcome::Failed(_) => {
                    crate::app::state::FileManagerOperationItemStatus::Failed
                }
            },
        })
        .collect();
    operation.status = match result.status() {
        FileOperationExecutionStatus::Completed => FileManagerOperationStatus::Completed,
        FileOperationExecutionStatus::Cancelled => FileManagerOperationStatus::Cancelled,
        FileOperationExecutionStatus::Failed if completed_items > 0 || source_retained > 0 => {
            FileManagerOperationStatus::Partial
        }
        FileOperationExecutionStatus::Failed => FileManagerOperationStatus::Failed,
    };
}

fn mark_operation_worker_failure(operation: &mut crate::app::state::FileManagerOperationState) {
    operation.status = crate::app::state::FileManagerOperationStatus::Failed;
    operation.completed_items = 0;
    operation.failed_items = operation.total_items;
    for item in &mut operation.items {
        item.status = crate::app::state::FileManagerOperationItemStatus::Failed;
    }
}

fn apply_delete_execution_result(
    operation: &mut crate::app::state::FileManagerOperationState,
    result: &DeleteOperationExecutionResult,
) {
    use crate::app::state::FileManagerOperationStatus;

    operation.completed_items = result
        .items()
        .iter()
        .filter(|item| matches!(item.outcome(), DeleteOperationItemOutcome::Deleted))
        .count();
    operation.failed_items = result
        .items()
        .iter()
        .filter(|item| matches!(item.outcome(), DeleteOperationItemOutcome::Retained(_)))
        .count();
    operation.items = result
        .items()
        .iter()
        .map(|item| crate::app::state::FileManagerOperationItemState {
            path: item.path().to_path_buf(),
            recovery_path: None,
            status: match item.outcome() {
                DeleteOperationItemOutcome::NotStarted => {
                    crate::app::state::FileManagerOperationItemStatus::Pending
                }
                DeleteOperationItemOutcome::Deleted => {
                    crate::app::state::FileManagerOperationItemStatus::Completed
                }
                DeleteOperationItemOutcome::Retained(_) => {
                    crate::app::state::FileManagerOperationItemStatus::Retained
                }
            },
        })
        .collect();
    for item in result.items() {
        if let DeleteOperationItemOutcome::Retained(error) = item.outcome() {
            tracing::warn!(path = %item.path().display(), ?error, "fm: delete source retained");
        }
    }
    operation.status = match result.status() {
        DeleteOperationExecutionStatus::Completed => FileManagerOperationStatus::Completed,
        DeleteOperationExecutionStatus::Cancelled => FileManagerOperationStatus::Cancelled,
        DeleteOperationExecutionStatus::Partial => FileManagerOperationStatus::Partial,
        DeleteOperationExecutionStatus::Failed => FileManagerOperationStatus::Failed,
    };
}

fn apply_rename_execution_result(
    operation: &mut crate::app::state::FileManagerOperationState,
    result: &RenameOperationExecutionResult,
) {
    use crate::app::state::{
        FileManagerOperationItemState, FileManagerOperationItemStatus, FileManagerOperationStatus,
    };

    let Some(source) = operation.items.first().map(|item| item.path.clone()) else {
        mark_operation_worker_failure(operation);
        return;
    };
    let (completed_items, failed_items, item_status) = match result.outcome() {
        RenameOperationOutcome::NotStarted => (0, 0, FileManagerOperationItemStatus::Pending),
        RenameOperationOutcome::Renamed => (1, 0, FileManagerOperationItemStatus::Completed),
        RenameOperationOutcome::Retained(error) => {
            tracing::warn!(path = %source.display(), ?error, "fm: rename source retained");
            (0, 1, FileManagerOperationItemStatus::Retained)
        }
    };
    operation.completed_items = completed_items;
    operation.failed_items = failed_items;
    operation.items = vec![FileManagerOperationItemState {
        path: source,
        recovery_path: None,
        status: item_status,
    }];
    operation.status = match result.status() {
        RenameOperationExecutionStatus::Completed => FileManagerOperationStatus::Completed,
        RenameOperationExecutionStatus::Cancelled => FileManagerOperationStatus::Cancelled,
        RenameOperationExecutionStatus::Failed => FileManagerOperationStatus::Failed,
    };
}

fn apply_bulk_rename_execution_result(
    operation: &mut crate::app::state::FileManagerOperationState,
    result: &BulkRenameOperationExecutionResult,
) {
    use crate::app::state::{
        FileManagerOperationItemState, FileManagerOperationItemStatus, FileManagerOperationStatus,
    };

    operation.completed_items = result
        .items()
        .iter()
        .filter(|item| {
            matches!(
                item.outcome(),
                BulkRenameItemOutcome::Renamed | BulkRenameItemOutcome::Unchanged
            )
        })
        .count();
    operation.failed_items = result
        .items()
        .iter()
        .filter(|item| {
            matches!(
                item.outcome(),
                BulkRenameItemOutcome::Restored(_)
                    | BulkRenameItemOutcome::Retained(_)
                    | BulkRenameItemOutcome::Uncertain(_)
            )
        })
        .count();
    operation.items = result
        .items()
        .iter()
        .map(|item| FileManagerOperationItemState {
            path: item.source().to_path_buf(),
            recovery_path: item.recovery_path().map(Path::to_path_buf),
            status: match item.outcome() {
                BulkRenameItemOutcome::NotStarted => FileManagerOperationItemStatus::Pending,
                BulkRenameItemOutcome::Renamed | BulkRenameItemOutcome::Unchanged => {
                    FileManagerOperationItemStatus::Completed
                }
                BulkRenameItemOutcome::Restored(_) | BulkRenameItemOutcome::Retained(_) => {
                    FileManagerOperationItemStatus::Retained
                }
                BulkRenameItemOutcome::Uncertain(_) => FileManagerOperationItemStatus::Failed,
            },
        })
        .collect();
    for item in result.items() {
        if let BulkRenameItemOutcome::Uncertain(error) = item.outcome() {
            tracing::error!(
                source = %item.source().display(),
                destination = %item.destination().display(),
                recovery_path = ?item.recovery_path(),
                ?error,
                "fm: bulk rename recovery is uncertain"
            );
        }
    }
    operation.status = match result.status() {
        BulkRenameOperationExecutionStatus::Completed => FileManagerOperationStatus::Completed,
        BulkRenameOperationExecutionStatus::Cancelled => FileManagerOperationStatus::Cancelled,
        BulkRenameOperationExecutionStatus::RolledBack
        | BulkRenameOperationExecutionStatus::Failed => FileManagerOperationStatus::Failed,
        BulkRenameOperationExecutionStatus::RecoveryFailed => FileManagerOperationStatus::Partial,
    };
}

#[cfg(test)]
mod tests {
    use super::{
        FileOperationStartError, FileOperationWorker, FileOperationWorkerError,
        FileOperationWorkerResult, FileOperationWorkerTask,
    };
    use crate::app::state::{
        FileManagerContextActionIntent, FileManagerContextMenuAction, FileManagerDeleteKind,
        FileManagerDeleteRequest, FileManagerHeaderAction, FileManagerOperationItemStatus,
        FileManagerOperationKind, FileManagerOperationState, FileManagerOperationStatus,
    };
    use crate::fm::delete::{
        execute_delete_operation, execute_delete_operation_with_host, DeleteBackendError,
        DeleteOperationHost, DeleteOperationKind, DeleteOperationPlan, DeleteOperationRequest,
        PlannedDeletePathKind,
    };
    use crate::fm::operations::{
        execute_file_operation, FileOperationCancellation, FileOperationExecutionStatus,
        FileOperationKind, FileOperationPlan, FileOperationRequest,
    };
    use crate::fm::rename::{
        BulkRenameOperationExecutionStatus, BulkRenameOperationPlan, BulkRenameOperationRequest,
        RenameOperationExecutionStatus, RenameOperationPlan, RenameOperationRequest,
    };
    use crate::input::TerminalKey;
    use crossterm::event::{KeyCode, KeyModifiers};
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{mpsc, Arc};
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

    fn expect_transfer_result(
        result: FileOperationWorkerResult,
    ) -> crate::fm::operations::FileOperationExecutionResult {
        match result {
            FileOperationWorkerResult::Transfer(result) => result,
            _ => panic!("expected transfer worker result"),
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
            expect_transfer_result(first.result.expect("first execution")).status(),
            FileOperationExecutionStatus::Completed
        );
        assert!(!worker.is_busy());

        let second_generation = worker.start(td.copy_plan("second")).expect("start second");
        assert!(second_generation > first_generation);
        let second = wait_for_completion(&mut worker);
        assert_eq!(second.generation, second_generation);
        assert_eq!(
            expect_transfer_result(second.result.expect("second execution")).status(),
            FileOperationExecutionStatus::Completed
        );
    }

    // TP-C4.4-PROGRESS: rapid worker updates occupy one latest-value slot.
    // The UI sees the newest same-generation item without an unbounded queue,
    // and terminal completion cannot leave stale progress behind.
    #[test]
    fn file_operation_worker_progress_is_bounded_coalesced_and_generation_safe() {
        let td = TempDir::new("coalesced-progress");
        let first = td.root.join("first.txt");
        let second = td.root.join("second.txt");
        fs::write(&first, b"first").expect("write first progress source");
        fs::write(&second, b"second").expect("write second progress source");
        let destination = td.root.join("destination-progress");
        fs::create_dir(&destination).expect("create progress destination");
        let plan = FileOperationPlan::preflight(FileOperationRequest {
            kind: FileOperationKind::Copy,
            sources: vec![first, second],
            destination_directory: destination,
            operation_in_flight: false,
        })
        .expect("progress plan");
        let (reported_tx, reported_rx) = mpsc::sync_channel(1);
        let (release_tx, release_rx) = mpsc::sync_channel(1);
        let mut worker = FileOperationWorker::with_progress_task_executor(
            Arc::new(Notify::new()),
            move |task, cancellation, report_progress| {
                report_progress(0);
                report_progress(1);
                reported_tx.send(()).expect("signal progress reported");
                release_rx.recv().expect("release progress executor");
                match task {
                    FileOperationWorkerTask::Transfer(plan) => FileOperationWorkerResult::Transfer(
                        execute_file_operation(plan, cancellation),
                    ),
                    _ => panic!("expected transfer progress task"),
                }
            },
        );
        let generation = worker.start(plan).expect("start progress operation");
        reported_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("worker reported progress");

        let drained = worker.drain();
        assert_eq!(
            drained.progress,
            Some(super::FileOperationWorkerProgress {
                generation,
                active_item_index: 1,
                started_items: 2,
            })
        );
        assert!(drained.completion.is_none());

        release_tx.send(()).expect("release progress operation");
        let completion = wait_for_completion(&mut worker);
        assert_eq!(completion.generation, generation);
        assert!(worker.drain().progress.is_none());
    }

    // TP-C4.4-PROGRESS: App projection accepts only the current generation and
    // exposes bounded per-item Running state before terminal completion.
    #[test]
    fn app_projects_current_worker_progress_without_waiting_for_completion() {
        let td = TempDir::new("app-progress-projection");
        let first = td.root.join("first.txt");
        let second = td.root.join("second.txt");
        fs::write(&first, b"first").expect("write first App progress source");
        fs::write(&second, b"second").expect("write second App progress source");
        let destination = td.root.join("destination-progress");
        fs::create_dir(&destination).expect("create App progress destination");
        let plan = FileOperationPlan::preflight(FileOperationRequest {
            kind: FileOperationKind::Copy,
            sources: vec![first.clone(), second.clone()],
            destination_directory: destination.clone(),
            operation_in_flight: false,
        })
        .expect("App progress plan");
        let (reported_tx, reported_rx) = mpsc::sync_channel(1);
        let (release_tx, release_rx) = mpsc::sync_channel(1);
        let mut worker = FileOperationWorker::with_progress_task_executor(
            Arc::new(Notify::new()),
            move |task, cancellation, report_progress| {
                report_progress(0);
                report_progress(1);
                reported_tx.send(()).expect("signal App progress reported");
                release_rx.recv().expect("release App progress executor");
                match task {
                    FileOperationWorkerTask::Transfer(plan) => FileOperationWorkerResult::Transfer(
                        execute_file_operation(plan, cancellation),
                    ),
                    _ => panic!("expected App transfer progress task"),
                }
            },
        );
        let generation = worker.start(plan).expect("start App progress operation");
        let mut app = test_app();
        app.file_operation_worker = worker;
        app.state.file_manager_operation = Some(FileManagerOperationState {
            generation,
            kind: FileManagerOperationKind::Copy,
            destination_directory: destination,
            total_items: 2,
            completed_items: 0,
            failed_items: 0,
            status: FileManagerOperationStatus::Running,
            items: vec![
                crate::app::state::FileManagerOperationItemState {
                    path: first,
                    recovery_path: None,
                    status: FileManagerOperationItemStatus::Pending,
                },
                crate::app::state::FileManagerOperationItemState {
                    path: second,
                    recovery_path: None,
                    status: FileManagerOperationItemStatus::Pending,
                },
            ],
        });
        reported_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("App worker reported progress");

        assert!(app.sync_file_operation_worker());
        let operation = app
            .state
            .file_manager_operation
            .as_ref()
            .expect("running App progress state");
        assert_eq!(operation.status, FileManagerOperationStatus::Running);
        assert_eq!(
            operation
                .items
                .iter()
                .map(|item| item.status)
                .collect::<Vec<_>>(),
            vec![
                FileManagerOperationItemStatus::Running,
                FileManagerOperationItemStatus::Running,
            ]
        );

        release_tx.send(()).expect("release App progress operation");
        let deadline = Instant::now() + Duration::from_secs(5);
        while app
            .state
            .file_manager_operation
            .as_ref()
            .is_some_and(FileManagerOperationState::is_running)
        {
            let _ = app.sync_file_operation_worker();
            assert!(
                Instant::now() < deadline,
                "App progress completion timed out"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    // TP-C4.4-PROGRESS: the production task adapter forwards real transfer
    // item indices instead of leaving the bounded worker slot test-only.
    #[test]
    fn default_worker_task_executor_reports_transfer_item_indices() {
        let td = TempDir::new("default-transfer-progress");
        let first = td.root.join("first.txt");
        let second = td.root.join("second.txt");
        fs::write(&first, b"first").expect("write first transfer progress source");
        fs::write(&second, b"second").expect("write second transfer progress source");
        let destination = td.root.join("destination-progress");
        fs::create_dir(&destination).expect("create transfer progress destination");
        let plan = FileOperationPlan::preflight(FileOperationRequest {
            kind: FileOperationKind::Copy,
            sources: vec![first, second],
            destination_directory: destination,
            operation_in_flight: false,
        })
        .expect("default transfer progress plan");
        let mut reported = Vec::new();

        let result = super::execute_worker_task_with_progress(
            &FileOperationWorkerTask::Transfer(plan),
            &FileOperationCancellation::default(),
            &mut |item_index| reported.push(item_index),
        );

        assert!(matches!(result, FileOperationWorkerResult::Transfer(_)));
        reported.sort_unstable();
        reported.dedup();
        assert_eq!(reported, vec![0, 1]);
    }

    // TP-C4.4-PROGRESS: delete reports each exact temporary item through the
    // same bounded slot; this test never touches the platform Trash backend.
    #[test]
    fn default_worker_task_executor_reports_delete_item_indices() {
        let td = TempDir::new("default-delete-progress");
        let first = td.root.join("first.txt");
        let second = td.root.join("second.txt");
        fs::write(&first, b"first").expect("write first delete progress source");
        fs::write(&second, b"second").expect("write second delete progress source");
        let plan = DeleteOperationPlan::preflight(DeleteOperationRequest {
            kind: DeleteOperationKind::Permanent,
            paths: vec![first, second],
            operation_in_flight: false,
        })
        .expect("default delete progress plan");
        let mut reported = Vec::new();

        let result = super::execute_worker_task_with_progress(
            &FileOperationWorkerTask::Delete(plan),
            &FileOperationCancellation::default(),
            &mut |item_index| reported.push(item_index),
        );

        assert!(matches!(result, FileOperationWorkerResult::Delete(_)));
        assert_eq!(reported, vec![0, 1]);
    }

    // TP-C4.4-PROGRESS: single Rename reports its exact item before the
    // no-replace commit and still uses the same bounded worker slot.
    #[test]
    fn default_worker_task_executor_reports_single_rename_item() {
        let td = TempDir::new("default-single-rename-progress");
        let source = td.root.join("source.txt");
        fs::write(&source, b"source").expect("write rename progress source");
        let plan = RenameOperationPlan::preflight(RenameOperationRequest {
            source_path: source,
            new_name: "renamed.txt".into(),
            operation_in_flight: false,
        })
        .expect("default single rename progress plan");
        let mut reported = Vec::new();

        let result = super::execute_worker_task_with_progress(
            &FileOperationWorkerTask::Rename(plan),
            &FileOperationCancellation::default(),
            &mut |item_index| reported.push(item_index),
        );

        assert!(matches!(result, FileOperationWorkerResult::Rename(_)));
        assert_eq!(reported, vec![0]);
    }

    // TP-C4.4-PROGRESS: a cycle-safe bulk plan reports every mapping once as
    // it enters staging, independent of later publish or recovery phases.
    #[test]
    fn default_worker_task_executor_reports_bulk_rename_item_indices() {
        let td = TempDir::new("default-bulk-rename-progress");
        let alpha = td.root.join("alpha.txt");
        let beta = td.root.join("beta.txt");
        fs::write(&alpha, b"alpha").expect("write bulk alpha progress source");
        fs::write(&beta, b"beta").expect("write bulk beta progress source");
        let plan = BulkRenameOperationPlan::preflight(BulkRenameOperationRequest {
            mappings: vec![(alpha, "beta.txt".into()), (beta, "alpha.txt".into())],
            operation_in_flight: false,
        })
        .expect("default bulk rename progress plan");
        let mut reported = Vec::new();

        let result = super::execute_worker_task_with_progress(
            &FileOperationWorkerTask::BulkRename(plan),
            &FileOperationCancellation::default(),
            &mut |item_index| reported.push(item_index),
        );

        assert!(matches!(result, FileOperationWorkerResult::BulkRename(_)));
        assert_eq!(reported, vec![0, 1]);
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
            expect_transfer_result(completion.result.expect("cancel result")).status(),
            FileOperationExecutionStatus::Cancelled
        );
        assert!(fs::read_dir(destination)
            .expect("read cancelled destination")
            .next()
            .is_none());
        assert!(!worker.is_busy());
        assert!(!worker.cancel(), "idle cancel is a no-op");
    }

    // TP-C4.4-CANCEL: once the worker has buffered completion, cancellation
    // has lost the race even before the UI drains that result. The cancel API
    // must not claim acceptance or rewrite the sole terminal outcome.
    #[test]
    fn file_operation_worker_rejects_cancel_after_buffered_completion() {
        let td = TempDir::new("cancel-after-completion");
        let wake = Arc::new(Notify::new());
        let mut worker = FileOperationWorker::with_executor(wake, execute_file_operation);
        let plan = td.copy_plan("already-completed");
        let destination = plan.destination_directory().to_path_buf();
        let generation = worker.start(plan).expect("start completion race work");

        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            let completion_is_buffered = {
                let (state, _) = &*worker.shared;
                super::lock_state(state)
                    .completion
                    .as_ref()
                    .is_some_and(|completion| completion.generation == generation)
            };
            if completion_is_buffered {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "worker completion was not buffered"
            );
            std::thread::sleep(Duration::from_millis(5));
        }

        assert!(
            !worker.cancel_generation(generation),
            "buffered terminal completion cannot accept cancellation"
        );
        let completion = worker.drain().completion.expect("buffered completion");
        assert_eq!(completion.generation, generation);
        assert_eq!(
            expect_transfer_result(completion.result.expect("completion race result")).status(),
            FileOperationExecutionStatus::Completed
        );
        assert!(destination.join("source-already-completed.txt").exists());
        assert!(!worker.is_busy());
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
            if app.sync_file_operation_worker()
                && app
                    .state
                    .file_manager_operation
                    .as_ref()
                    .is_some_and(|operation| {
                        operation.status == FileManagerOperationStatus::Completed
                    })
            {
                break;
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

    // TP-C4.4-CANCEL: the typed Esc intent must reach the exact active worker
    // generation, remain idempotent, keep the FM open, and terminalize the
    // operation without publishing a destination entry.
    #[tokio::test]
    async fn app_esc_cancellation_is_generation_safe_and_lane_reusable() {
        let td = TempDir::new("app-esc-cancel");
        let source_root = td.root.join("sources");
        let destination = td.root.join("destination");
        fs::create_dir(&source_root).expect("create cancellable source root");
        fs::create_dir(&destination).expect("create cancellable destination");
        let source = source_root.join("payload.txt");
        let next_source = source_root.join("next.txt");
        fs::write(&source, b"payload").expect("write cancellable source");
        fs::write(&next_source, b"next").expect("write next-generation source");
        let (started_tx, started_rx) = mpsc::channel();
        let calls = Arc::new(AtomicU64::new(0));
        let executor_calls = calls.clone();
        let worker = FileOperationWorker::with_executor(
            Arc::new(Notify::new()),
            move |plan, cancellation| {
                if executor_calls.fetch_add(1, Ordering::Relaxed) == 0 {
                    started_tx
                        .send(())
                        .expect("signal cancellable worker started");
                    while !cancellation.is_cancelled() {
                        std::thread::yield_now();
                    }
                }
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
            .expect("cancellable worker started");
        let generation = app
            .state
            .file_manager_operation
            .as_ref()
            .expect("running cancellable operation")
            .generation;

        app.state
            .file_manager_operation
            .as_mut()
            .expect("stale cancellation fixture")
            .generation = generation.saturating_add(1);
        app.handle_key(TerminalKey::new(KeyCode::Esc, KeyModifiers::NONE))
            .await;
        std::thread::sleep(Duration::from_millis(20));
        assert!(
            app.file_operation_worker.is_busy(),
            "stale App generation cannot cancel the active worker"
        );
        app.state
            .file_manager_operation
            .as_mut()
            .expect("restore cancellation fixture")
            .generation = generation;

        app.handle_key(TerminalKey::new(KeyCode::Esc, KeyModifiers::NONE))
            .await;
        app.handle_key(TerminalKey::new(KeyCode::Esc, KeyModifiers::NONE))
            .await;
        assert!(app.state.file_manager.is_some());

        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            if app.sync_file_operation_worker()
                && app
                    .state
                    .file_manager_operation
                    .as_ref()
                    .is_some_and(|operation| {
                        operation.status == FileManagerOperationStatus::Cancelled
                    })
            {
                break;
            }
            assert!(Instant::now() < deadline, "Esc cancellation timed out");
            std::thread::sleep(Duration::from_millis(5));
        }

        let operation = app
            .state
            .file_manager_operation
            .as_ref()
            .expect("cancelled operation state");
        assert_eq!(operation.generation, generation);
        assert_eq!(operation.status, FileManagerOperationStatus::Cancelled);
        assert_eq!(operation.completed_items, 0);
        assert_eq!(operation.failed_items, 0);
        assert!(operation
            .items
            .iter()
            .all(|item| item.status == FileManagerOperationItemStatus::Pending));
        assert_eq!(
            fs::read(source).expect("cancelled source retained"),
            b"payload"
        );
        assert!(!destination.join("payload.txt").exists());
        assert!(!app.file_operation_worker.is_busy());
        assert!(!app.file_operation_worker.cancel());

        app.state.file_manager_clipboard = vec![next_source];
        assert!(app.dispatch_file_manager_header_action(FileManagerHeaderAction::Paste));
        let next_generation = app
            .state
            .file_manager_operation
            .as_ref()
            .expect("next operation running")
            .generation;
        assert!(next_generation > generation);
        assert!(
            !app.file_operation_worker.cancel_generation(generation),
            "stale generation cannot cancel the reused lane"
        );

        let deadline = Instant::now() + Duration::from_secs(5);
        while app
            .state
            .file_manager_operation
            .as_ref()
            .is_some_and(FileManagerOperationState::is_running)
        {
            let _ = app.sync_file_operation_worker();
            assert!(Instant::now() < deadline, "next generation timed out");
            std::thread::sleep(Duration::from_millis(5));
        }
        let next_operation = app
            .state
            .file_manager_operation
            .as_ref()
            .expect("next operation terminal");
        assert_eq!(next_operation.generation, next_generation);
        assert_eq!(next_operation.status, FileManagerOperationStatus::Completed);
        assert_eq!(
            fs::read(destination.join("next.txt")).expect("read next-generation output"),
            b"next"
        );
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

    // TP-C4.3-INTENT: C3's revalidated context Rename and C2's row Rename
    // converge on the same typed exact-path modal. Multi-path, stale, and
    // closed-FM intents are consumed but cannot install authority.
    #[test]
    fn app_context_rename_converges_and_invalid_intents_fail_closed() {
        let td = TempDir::new("context-rename-intent");
        let source = td.root.join("selected.txt");
        fs::write(&source, b"selected").expect("write context rename fixture");
        let mut app = test_app();
        let mut file_manager = crate::fm::FmState::new(&td.root);
        assert!(file_manager.replace_selection(0));
        app.state.file_manager = Some(file_manager);
        app.state.request_file_manager_context_action = Some(FileManagerContextActionIntent {
            action: FileManagerContextMenuAction::Rename,
            paths: vec![source.clone()],
        });

        assert!(app.sync_file_operation_worker());
        assert_eq!(app.state.mode, crate::app::Mode::RenameFile);
        assert_eq!(
            app.state
                .file_manager_rename
                .as_ref()
                .expect("context file rename modal")
                .paths,
            vec![source.clone()]
        );
        assert!(app.state.request_file_manager_context_action.is_none());
        assert!(app.state.file_manager_operation.is_none());
        assert_eq!(fs::read(&source).expect("source remains"), b"selected");

        for paths in [
            vec![source.clone(), td.root.join("other.txt")],
            vec![td.root.join("stale.txt")],
        ] {
            app.state.mode = crate::app::Mode::Terminal;
            app.state.file_manager_rename = None;
            app.state.request_file_manager_context_action = Some(FileManagerContextActionIntent {
                action: FileManagerContextMenuAction::Rename,
                paths,
            });
            assert!(app.sync_file_operation_worker());
            assert_ne!(app.state.mode, crate::app::Mode::RenameFile);
            assert!(app.state.file_manager_rename.is_none());
            assert!(app.state.request_file_manager_context_action.is_none());
        }

        app.state.file_manager = None;
        app.state.request_file_manager_context_action = Some(FileManagerContextActionIntent {
            action: FileManagerContextMenuAction::Rename,
            paths: vec![source.clone()],
        });
        assert!(app.sync_file_operation_worker());
        assert_ne!(app.state.mode, crate::app::Mode::RenameFile);
        assert!(app.state.file_manager_rename.is_none());
        assert_eq!(
            fs::read(&source).expect("closed FM source remains"),
            b"selected"
        );
    }

    // TP-C4.2-DELETE/RECOVERY: a separately confirmed permanent request enters
    // the same bounded worker lane, reaches terminal per-item state, and
    // reloads the matching native-FM directory after completion.
    #[test]
    fn app_permanent_delete_runs_in_worker_and_reloads_matching_directory() {
        let td = TempDir::new("app-permanent-delete");
        let source = td.root.join("selected.txt");
        fs::write(&source, b"selected").expect("write permanent delete fixture");
        let mut app = test_app();
        let mut file_manager = crate::fm::FmState::new(&td.root);
        assert!(file_manager.replace_selection(0));
        app.state.file_manager = Some(file_manager);
        app.state.request_file_manager_delete = Some(FileManagerDeleteRequest {
            kind: FileManagerDeleteKind::Permanent,
            paths: vec![source.clone()],
        });

        assert!(app.sync_file_operation_worker());
        assert!(app.state.request_file_manager_delete.is_none());
        let generation = app
            .state
            .file_manager_operation
            .as_ref()
            .expect("running permanent delete")
            .generation;

        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let _ = app.sync_file_operation_worker();
            if app
                .state
                .file_manager_operation
                .as_ref()
                .is_some_and(|operation| !operation.is_running())
            {
                break;
            }
            assert!(Instant::now() < deadline, "permanent delete timed out");
            std::thread::sleep(Duration::from_millis(5));
        }

        let operation = app
            .state
            .file_manager_operation
            .as_ref()
            .expect("terminal permanent delete");
        assert_eq!(operation.generation, generation);
        assert_eq!(operation.kind, FileManagerOperationKind::PermanentDelete);
        assert_eq!(operation.status, FileManagerOperationStatus::Completed);
        assert_eq!(operation.completed_items, 1);
        assert_eq!(operation.failed_items, 0);
        assert!(!source.exists());
        assert!(app
            .state
            .file_manager
            .as_ref()
            .expect("open FM")
            .entries
            .is_empty());
    }

    // TP-C4.2-TRASH: a confirmed trash request is consumed but fails closed
    // before preflight while another operation is projected in flight. This
    // test deliberately never invokes the host trash backend.
    #[test]
    fn app_trash_request_fails_closed_while_operation_is_inflight() {
        let td = TempDir::new("app-trash-busy");
        let source = td.root.join("selected.txt");
        fs::write(&source, b"selected").expect("write trash busy fixture");
        let mut app = test_app();
        app.state.file_manager = Some(crate::fm::FmState::new(&td.root));
        app.state.file_manager_operation = Some(FileManagerOperationState {
            generation: 41,
            kind: FileManagerOperationKind::Copy,
            destination_directory: td.root.clone(),
            total_items: 1,
            completed_items: 0,
            failed_items: 0,
            status: FileManagerOperationStatus::Running,
            items: Vec::new(),
        });
        app.state.request_file_manager_delete = Some(FileManagerDeleteRequest {
            kind: FileManagerDeleteKind::Trash,
            paths: vec![source.clone()],
        });

        assert!(app.sync_file_operation_worker());

        assert!(app.state.request_file_manager_delete.is_none());
        assert_eq!(
            app.state
                .file_manager_operation
                .as_ref()
                .expect("existing operation retained")
                .generation,
            41
        );
        assert_eq!(
            fs::read(source).expect("busy source preserved"),
            b"selected"
        );
    }

    // TP-C4.2-RECOVERY: partial destructive completion remains exact path
    // evidence in pure AppState instead of collapsing to aggregate counts.
    #[test]
    fn delete_partial_result_projects_ordered_per_item_recovery_state() {
        struct PartialHost {
            retained: PathBuf,
        }

        impl DeleteOperationHost for PartialHost {
            fn delete_path(
                &mut self,
                _operation: DeleteOperationKind,
                _path_kind: PlannedDeletePathKind,
                path: &std::path::Path,
            ) -> Result<(), DeleteBackendError> {
                if path == self.retained {
                    return Err(DeleteBackendError::Io(std::io::ErrorKind::PermissionDenied));
                }
                fs::remove_file(path).map_err(|error| DeleteBackendError::Io(error.kind()))
            }
        }

        let td = TempDir::new("delete-partial-projection");
        let first = td.root.join("first.txt");
        let retained = td.root.join("retained.txt");
        let last = td.root.join("last.txt");
        fs::write(&first, b"first").expect("write first delete projection fixture");
        fs::write(&retained, b"retained").expect("write retained delete projection fixture");
        fs::write(&last, b"last").expect("write last delete projection fixture");
        let plan = DeleteOperationPlan::preflight(DeleteOperationRequest {
            kind: DeleteOperationKind::Permanent,
            paths: vec![first.clone(), retained.clone(), last.clone()],
            operation_in_flight: false,
        })
        .expect("valid partial delete plan");
        let result = execute_delete_operation_with_host(
            &plan,
            &FileOperationCancellation::default(),
            &mut PartialHost {
                retained: retained.clone(),
            },
        );
        let mut operation = FileManagerOperationState {
            generation: 1,
            kind: FileManagerOperationKind::PermanentDelete,
            destination_directory: td.root.clone(),
            total_items: 3,
            completed_items: 0,
            failed_items: 0,
            status: FileManagerOperationStatus::Running,
            items: Vec::new(),
        };

        super::apply_delete_execution_result(&mut operation, &result);

        assert_eq!(operation.status, FileManagerOperationStatus::Partial);
        assert_eq!(operation.completed_items, 2);
        assert_eq!(operation.failed_items, 1);
        assert_eq!(
            operation
                .items
                .iter()
                .map(|item| (item.path.clone(), item.status))
                .collect::<Vec<_>>(),
            vec![
                (first, FileManagerOperationItemStatus::Completed),
                (retained, FileManagerOperationItemStatus::Retained),
                (last, FileManagerOperationItemStatus::Completed),
            ]
        );
    }

    // TP-C4.2-TRASH: the confirmed UI kind maps to a real Trash worker task.
    // Cancellation is injected before execution so the test never touches a
    // platform trash backend or the user's Trash.
    #[test]
    fn app_trash_request_maps_to_trash_task_without_ui_thread_mutation() {
        let td = TempDir::new("app-trash-task-mapping");
        let source = td.root.join("selected.txt");
        fs::write(&source, b"selected").expect("write trash task fixture");
        let worker = FileOperationWorker::with_task_executor(
            Arc::new(Notify::new()),
            |task, cancellation| match task {
                FileOperationWorkerTask::Delete(plan) => {
                    assert_eq!(plan.kind(), DeleteOperationKind::Trash);
                    cancellation.cancel();
                    FileOperationWorkerResult::Delete(execute_delete_operation(plan, cancellation))
                }
                _ => panic!("expected trash delete task"),
            },
        );
        let mut app = test_app();
        app.file_operation_worker = worker;
        let mut file_manager = crate::fm::FmState::new(&td.root);
        assert!(file_manager.replace_selection(0));
        app.state.file_manager = Some(file_manager);
        app.state.request_file_manager_delete = Some(FileManagerDeleteRequest {
            kind: FileManagerDeleteKind::Trash,
            paths: vec![source.clone()],
        });

        assert!(app.sync_file_operation_worker());
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let _ = app.sync_file_operation_worker();
            if app
                .state
                .file_manager_operation
                .as_ref()
                .is_some_and(|operation| !operation.is_running())
            {
                break;
            }
            assert!(Instant::now() < deadline, "trash cancellation timed out");
            std::thread::sleep(Duration::from_millis(5));
        }

        let operation = app
            .state
            .file_manager_operation
            .as_ref()
            .expect("trash state");
        assert_eq!(operation.kind, FileManagerOperationKind::Trash);
        assert_eq!(operation.status, FileManagerOperationStatus::Cancelled);
        assert_eq!(
            operation.items[0].status,
            FileManagerOperationItemStatus::Pending
        );
        assert_eq!(
            fs::read(source).expect("cancelled trash source retained"),
            b"selected"
        );
    }

    // TP-C4.2-RECOVERY: a worker panic is terminal for every exact item. No
    // item may remain visually Pending after the lane has failed.
    #[test]
    fn app_delete_worker_panic_marks_every_item_failed() {
        let td = TempDir::new("app-delete-panic-items");
        let source = td.root.join("selected.txt");
        fs::write(&source, b"selected").expect("write delete panic fixture");
        let worker = FileOperationWorker::with_task_executor(
            Arc::new(Notify::new()),
            |_task, _cancellation| panic!("injected delete task panic"),
        );
        let mut app = test_app();
        app.file_operation_worker = worker;
        let mut file_manager = crate::fm::FmState::new(&td.root);
        assert!(file_manager.replace_selection(0));
        app.state.file_manager = Some(file_manager);
        app.state.request_file_manager_delete = Some(FileManagerDeleteRequest {
            kind: FileManagerDeleteKind::Permanent,
            paths: vec![source.clone()],
        });

        assert!(app.sync_file_operation_worker());
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let _ = app.sync_file_operation_worker();
            if app
                .state
                .file_manager_operation
                .as_ref()
                .is_some_and(|operation| !operation.is_running())
            {
                break;
            }
            assert!(Instant::now() < deadline, "delete panic timed out");
            std::thread::sleep(Duration::from_millis(5));
        }

        let operation = app
            .state
            .file_manager_operation
            .as_ref()
            .expect("panic terminal state");
        assert_eq!(operation.status, FileManagerOperationStatus::Failed);
        assert_eq!(operation.failed_items, 1);
        assert_eq!(operation.items.len(), 1);
        assert_eq!(operation.items[0].path, source);
        assert_eq!(
            operation.items[0].status,
            FileManagerOperationItemStatus::Failed
        );
        assert_eq!(
            fs::read(&operation.items[0].path).expect("panic source retained"),
            b"selected"
        );
    }

    // TP-C4.4-RECOVERY: progress followed by a caught worker panic must
    // terminalize every item, release reconciliation ownership, and leave the
    // same single lane reusable at a strictly newer generation.
    #[test]
    fn app_recovers_from_progress_then_panic_and_reuses_lane() {
        let td = TempDir::new("progress-panic-recovery");
        let source_dir = td.root.join("source");
        let destination = td.root.join("destination");
        fs::create_dir(&source_dir).expect("create panic recovery source directory");
        fs::create_dir(&destination).expect("create panic recovery destination");
        let first = source_dir.join("first.txt");
        let second = source_dir.join("second.txt");
        let next = source_dir.join("next.txt");
        fs::write(&first, b"first").expect("write first panic recovery source");
        fs::write(&second, b"second").expect("write second panic recovery source");
        fs::write(&next, b"next").expect("write next panic recovery source");

        let calls = Arc::new(AtomicU64::new(0));
        let executor_calls = calls.clone();
        let worker = FileOperationWorker::with_progress_task_executor(
            Arc::new(Notify::new()),
            move |task, cancellation, report_progress| {
                if executor_calls.fetch_add(1, Ordering::Relaxed) == 0 {
                    report_progress(0);
                    panic!("injected panic after progress");
                }
                match task {
                    FileOperationWorkerTask::Transfer(plan) => FileOperationWorkerResult::Transfer(
                        execute_file_operation(plan, cancellation),
                    ),
                    _ => panic!("expected transfer recovery task"),
                }
            },
        );
        let mut app = test_app();
        app.file_operation_worker = worker;
        app.state.file_manager = Some(crate::fm::FmState::new(&destination));
        app.state.file_manager_clipboard = vec![first.clone(), second.clone()];

        assert!(app.dispatch_file_manager_header_action(FileManagerHeaderAction::Paste));
        let failed_generation = app
            .state
            .file_manager_operation
            .as_ref()
            .expect("panic operation running")
            .generation;
        app.file_operation_reconcile_baseline = Some(super::FileOperationReconcileBaseline {
            operation_generation: failed_generation,
            destination_directory: destination.clone(),
            watcher_generation: 7,
            watcher_revision: 11,
            affected_paths: [
                destination.join("first.txt"),
                destination.join("second.txt"),
            ]
            .into_iter()
            .collect(),
        });

        let deadline = Instant::now() + Duration::from_secs(5);
        while app
            .state
            .file_manager_operation
            .as_ref()
            .is_some_and(FileManagerOperationState::is_running)
        {
            let _ = app.sync_file_operation_worker();
            assert!(
                Instant::now() < deadline,
                "progress panic recovery timed out"
            );
            std::thread::sleep(Duration::from_millis(5));
        }

        let failed = app
            .state
            .file_manager_operation
            .as_ref()
            .expect("panic operation terminal");
        assert_eq!(failed.status, FileManagerOperationStatus::Failed);
        assert_eq!(failed.completed_items, 0);
        assert_eq!(failed.failed_items, 2);
        assert!(failed
            .items
            .iter()
            .all(|item| item.status == FileManagerOperationItemStatus::Failed));
        assert!(app.file_operation_reconcile_baseline.is_none());
        assert!(!app.file_operation_worker.is_busy());

        app.state.file_manager_clipboard = vec![next];
        assert!(app.dispatch_file_manager_header_action(FileManagerHeaderAction::Paste));
        let next_generation = app
            .state
            .file_manager_operation
            .as_ref()
            .expect("next panic recovery operation running")
            .generation;
        assert!(next_generation > failed_generation);
        let deadline = Instant::now() + Duration::from_secs(5);
        while app
            .state
            .file_manager_operation
            .as_ref()
            .is_some_and(FileManagerOperationState::is_running)
        {
            let _ = app.sync_file_operation_worker();
            assert!(Instant::now() < deadline, "reused panic lane timed out");
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(
            app.state
                .file_manager_operation
                .as_ref()
                .expect("next panic recovery operation terminal")
                .status,
            FileManagerOperationStatus::Completed
        );
        assert_eq!(
            fs::read(destination.join("next.txt")).expect("read copied recovery file"),
            b"next"
        );
    }

    // TP-C4.4-RECOVERY: a dead worker observed after bounded progress must
    // terminalize every item, discard stale reconciliation ownership, and
    // reopen the same single lane at a strictly newer generation.
    #[test]
    fn app_recovers_disconnected_worker_after_progress_and_reuses_lane() {
        let td = TempDir::new("disconnect-recovery");
        let destination = td.root.join("destination");
        let source_dir = td.root.join("source");
        fs::create_dir(&destination).expect("create recovery destination");
        fs::create_dir(&source_dir).expect("create recovery source directory");
        let failed_first = destination.join("failed-first.txt");
        let failed_second = destination.join("failed-second.txt");
        fs::write(&failed_first, b"first").expect("write first failed fixture");
        fs::write(&failed_second, b"second").expect("write second failed fixture");
        let next_source = source_dir.join("next.txt");
        fs::write(&next_source, b"next").expect("write next generation source");
        let failed_generation = 41;

        let mut app = test_app();
        app.file_operation_worker =
            FileOperationWorker::disconnected_after_progress_for_test(failed_generation, 0, 1);
        app.state.file_manager = Some(crate::fm::FmState::new(&destination));
        app.state.file_manager_operation = Some(FileManagerOperationState {
            generation: failed_generation,
            kind: FileManagerOperationKind::Copy,
            destination_directory: destination.clone(),
            total_items: 2,
            completed_items: 0,
            failed_items: 0,
            status: FileManagerOperationStatus::Running,
            items: vec![
                crate::app::state::FileManagerOperationItemState {
                    path: failed_first,
                    recovery_path: None,
                    status: FileManagerOperationItemStatus::Pending,
                },
                crate::app::state::FileManagerOperationItemState {
                    path: failed_second,
                    recovery_path: None,
                    status: FileManagerOperationItemStatus::Pending,
                },
            ],
        });

        assert!(app.sync_file_operation_worker());
        let failed = app
            .state
            .file_manager_operation
            .as_ref()
            .expect("failed operation retained");
        assert_eq!(failed.status, FileManagerOperationStatus::Failed);
        assert_eq!(failed.completed_items, 0);
        assert_eq!(failed.failed_items, 2);
        assert!(failed
            .items
            .iter()
            .all(|item| item.status == FileManagerOperationItemStatus::Failed));
        assert!(app.file_operation_reconcile_baseline.is_none());
        assert!(!app.file_operation_worker.is_busy());

        app.state.file_manager_clipboard = vec![next_source];
        assert!(app.dispatch_file_manager_header_action(FileManagerHeaderAction::Paste));
        let next_generation = app
            .state
            .file_manager_operation
            .as_ref()
            .expect("next operation running")
            .generation;
        assert!(next_generation > failed_generation);
        let deadline = Instant::now() + Duration::from_secs(5);
        while !app.file_operation_worker.has_buffered_completion() {
            assert!(Instant::now() < deadline, "recovered lane timed out");
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(app.sync_file_operation_worker());
        assert_eq!(
            app.state
                .file_manager_operation
                .as_ref()
                .expect("next operation terminal")
                .status,
            FileManagerOperationStatus::Completed
        );
        assert_eq!(
            fs::read(destination.join("next.txt")).expect("read recovered copy"),
            b"next"
        );
    }

    // TP-C4.3-LIFECYCLE: one valid immutable single-rename request enters the
    // existing operation lane, reaches an exact terminal projection, and
    // reloads only the matching current directory after the worker commits.
    #[test]
    fn app_file_rename_runs_in_existing_lane_and_reloads_matching_directory() {
        let td = TempDir::new("app-rename-lifecycle");
        let source = td.root.join("selected.txt");
        let destination = td.root.join("renamed.txt");
        fs::write(&source, b"selected").expect("write rename lifecycle source");
        let mut app = test_app();
        let mut file_manager = crate::fm::FmState::new(&td.root);
        assert!(file_manager.replace_selection(0));
        app.state.file_manager = Some(file_manager);
        app.state.request_file_manager_rename = Some(crate::app::state::FileManagerRenameRequest {
            source_path: source.clone(),
            new_name: "renamed.txt".to_string(),
        });

        assert!(app.sync_file_operation_worker());
        assert!(app.state.request_file_manager_rename.is_none());
        let generation = app
            .state
            .file_manager_operation
            .as_ref()
            .expect("rename operation state")
            .generation;

        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let _ = app.sync_file_operation_worker();
            if app
                .state
                .file_manager_operation
                .as_ref()
                .is_some_and(|operation| !operation.is_running())
            {
                break;
            }
            assert!(Instant::now() < deadline, "rename completion timed out");
            std::thread::sleep(Duration::from_millis(5));
        }

        let operation = app
            .state
            .file_manager_operation
            .as_ref()
            .expect("terminal rename operation");
        assert_eq!(operation.generation, generation);
        assert_eq!(operation.kind, FileManagerOperationKind::Rename);
        assert_eq!(operation.status, FileManagerOperationStatus::Completed);
        assert_eq!(operation.completed_items, 1);
        assert_eq!(operation.failed_items, 0);
        assert_eq!(operation.items[0].path, source);
        assert_eq!(
            operation.items[0].status,
            FileManagerOperationItemStatus::Completed
        );
        assert!(!source.exists());
        assert_eq!(
            fs::read(&destination).expect("renamed payload"),
            b"selected"
        );
        let file_manager = app.state.file_manager.as_ref().expect("matching FM open");
        assert!(file_manager
            .entries
            .iter()
            .any(|entry| entry.path == destination));
        assert!(file_manager
            .entries
            .iter()
            .all(|entry| entry.path != source));
    }

    // TP-C4.3-LIFECYCLE: stale, closed, or busy App authority is consumed but
    // cannot replace the active operation generation or mutate either path.
    #[test]
    fn app_file_rename_request_fails_closed_when_stale_closed_or_busy() {
        let td = TempDir::new("app-rename-fail-closed");
        let source = td.root.join("selected.txt");
        fs::write(&source, b"selected").expect("write fail-closed source");
        let mut app = test_app();
        app.state.file_manager = Some(crate::fm::FmState::new(&td.root));

        app.state.file_manager_operation = Some(FileManagerOperationState {
            generation: 91,
            kind: FileManagerOperationKind::Copy,
            destination_directory: td.root.clone(),
            total_items: 1,
            completed_items: 0,
            failed_items: 0,
            status: FileManagerOperationStatus::Running,
            items: Vec::new(),
        });
        app.state.request_file_manager_rename = Some(crate::app::state::FileManagerRenameRequest {
            source_path: source.clone(),
            new_name: "busy.txt".to_string(),
        });
        assert!(app.sync_file_operation_worker());
        assert_eq!(
            app.state
                .file_manager_operation
                .as_ref()
                .expect("busy generation retained")
                .generation,
            91
        );
        assert!(source.exists());
        assert!(!td.root.join("busy.txt").exists());

        app.state.file_manager_operation = None;
        app.state.request_file_manager_rename = Some(crate::app::state::FileManagerRenameRequest {
            source_path: td.root.join("stale.txt"),
            new_name: "ignored.txt".to_string(),
        });
        assert!(app.sync_file_operation_worker());
        assert!(app.state.file_manager_operation.is_none());

        app.state.file_manager = None;
        app.state.request_file_manager_rename = Some(crate::app::state::FileManagerRenameRequest {
            source_path: source.clone(),
            new_name: "closed.txt".to_string(),
        });
        assert!(app.sync_file_operation_worker());
        assert!(app.state.file_manager_operation.is_none());
        assert_eq!(
            fs::read(source).expect("closed source retained"),
            b"selected"
        );
    }

    // TP-C4.3-LIFECYCLE: a rename may finish after FM close/reopen, but its
    // completion cannot project the old directory into the new file-manager
    // generation even though the exact filesystem operation itself completes.
    #[test]
    fn app_file_rename_completion_rejects_reopened_directory_projection() {
        let td = TempDir::new("app-rename-reopen");
        let old_directory = td.root.join("old");
        let reopened_directory = td.root.join("reopened");
        fs::create_dir(&old_directory).expect("create old rename directory");
        fs::create_dir(&reopened_directory).expect("create reopened rename directory");
        let source = old_directory.join("selected.txt");
        fs::write(&source, b"selected").expect("write reopened rename source");
        fs::write(reopened_directory.join("current.txt"), b"current")
            .expect("write reopened current fixture");
        let (started_tx, started_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let worker = FileOperationWorker::with_task_executor(
            Arc::new(Notify::new()),
            move |task, cancellation| match task {
                FileOperationWorkerTask::Rename(plan) => {
                    started_tx.send(()).expect("signal rename started");
                    release_rx.recv().expect("release rename");
                    FileOperationWorkerResult::Rename(crate::fm::rename::execute_rename_operation(
                        plan,
                        cancellation,
                    ))
                }
                _ => panic!("expected rename worker task"),
            },
        );
        let mut app = test_app();
        app.file_operation_worker = worker;
        app.state.file_manager = Some(crate::fm::FmState::new(&old_directory));
        app.state.request_file_manager_rename = Some(crate::app::state::FileManagerRenameRequest {
            source_path: source.clone(),
            new_name: "renamed.txt".to_string(),
        });
        assert!(app.sync_file_operation_worker());
        started_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("rename worker started");

        app.state.file_manager = None;
        app.state.file_manager = Some(crate::fm::FmState::new(&reopened_directory));
        release_tx.send(()).expect("release rename worker");
        let deadline = Instant::now() + Duration::from_secs(5);
        while app
            .state
            .file_manager_operation
            .as_ref()
            .is_some_and(FileManagerOperationState::is_running)
        {
            let _ = app.sync_file_operation_worker();
            assert!(Instant::now() < deadline, "reopened rename timed out");
            std::thread::sleep(Duration::from_millis(5));
        }

        let reopened = app.state.file_manager.as_ref().expect("reopened FM");
        assert_eq!(reopened.cwd, reopened_directory);
        assert_eq!(reopened.entries.len(), 1);
        assert_eq!(
            reopened.entries[0].path,
            reopened_directory.join("current.txt")
        );
        assert!(old_directory.join("renamed.txt").exists());
    }

    // TP-C4.3-BULK/LIFECYCLE: bulk plans use the same bounded generation lane
    // and cancellation token; they do not create a second scheduler.
    #[test]
    fn bulk_rename_plan_uses_existing_worker_lane_and_cancellation() {
        let td = TempDir::new("bulk-rename-worker");
        let alpha = td.root.join("alpha.txt");
        let beta = td.root.join("beta.txt");
        fs::write(&alpha, b"alpha").expect("write worker alpha");
        fs::write(&beta, b"beta").expect("write worker beta");
        let plan = BulkRenameOperationPlan::preflight(BulkRenameOperationRequest {
            mappings: vec![
                (alpha.clone(), "beta.txt".to_string()),
                (beta.clone(), "alpha.txt".to_string()),
            ],
            operation_in_flight: false,
        })
        .expect("worker bulk plan");
        let wake = Arc::new(Notify::new());
        let mut worker =
            FileOperationWorker::with_task_executor(wake, |task, cancellation| match task {
                FileOperationWorkerTask::BulkRename(plan) => {
                    cancellation.cancel();
                    FileOperationWorkerResult::BulkRename(
                        crate::fm::rename::execute_bulk_rename_operation(plan, cancellation),
                    )
                }
                _ => panic!("expected bulk rename task"),
            });

        let generation = worker
            .start_bulk_rename(plan)
            .expect("start bulk rename lane");
        assert_eq!(
            worker.start(td.copy_plan("busy")),
            Err(FileOperationStartError::Busy)
        );
        let completion = wait_for_completion(&mut worker);
        assert_eq!(completion.generation, generation);
        let result = match completion.result.expect("bulk worker result") {
            FileOperationWorkerResult::BulkRename(result) => result,
            _ => panic!("expected bulk rename result"),
        };
        assert_eq!(
            result.status(),
            BulkRenameOperationExecutionStatus::Cancelled
        );
        assert_eq!(fs::read(alpha).expect("cancelled alpha retained"), b"alpha");
        assert_eq!(fs::read(beta).expect("cancelled beta retained"), b"beta");
    }

    // TP-C4.3-BULK/LIFECYCLE: one complete mapping must match the current
    // visible selection in its stable row order before App hands it to the
    // shared lane. Completion projects exact item state and reloads the same
    // directory without a second scheduler or render-time filesystem work.
    #[test]
    fn app_bulk_rename_consumes_current_mapping_and_projects_terminal_items() {
        let td = TempDir::new("app-bulk-rename");
        let alpha = td.root.join("alpha.txt");
        let beta = td.root.join("beta.txt");
        fs::write(&alpha, b"alpha").expect("write App bulk alpha");
        fs::write(&beta, b"beta").expect("write App bulk beta");
        let mut file_manager = crate::fm::FmState::new(&td.root);
        let alpha_index = file_manager
            .entries
            .iter()
            .position(|entry| entry.path == alpha)
            .expect("alpha row");
        let beta_index = file_manager
            .entries
            .iter()
            .position(|entry| entry.path == beta)
            .expect("beta row");
        assert!(file_manager.replace_selection(alpha_index));
        assert!(file_manager.toggle_selection(beta_index));
        let mut app = test_app();
        app.state.file_manager = Some(file_manager);
        app.state.request_file_manager_bulk_rename =
            Some(crate::app::state::FileManagerBulkRenameRequest {
                mappings: vec![
                    (alpha.clone(), "beta.txt".to_string()),
                    (beta.clone(), "alpha.txt".to_string()),
                ],
            });

        assert!(app.sync_file_operation_worker());
        assert!(app.state.request_file_manager_bulk_rename.is_none());
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let _ = app.sync_file_operation_worker();
            if app
                .state
                .file_manager_operation
                .as_ref()
                .is_some_and(|operation| !operation.is_running())
            {
                break;
            }
            assert!(Instant::now() < deadline, "App bulk rename timed out");
            std::thread::sleep(Duration::from_millis(5));
        }

        let operation = app
            .state
            .file_manager_operation
            .as_ref()
            .expect("terminal App bulk rename");
        assert_eq!(operation.kind, FileManagerOperationKind::BulkRename);
        assert_eq!(operation.status, FileManagerOperationStatus::Completed);
        assert_eq!(operation.completed_items, 2);
        assert_eq!(operation.failed_items, 0);
        assert_eq!(
            operation
                .items
                .iter()
                .map(|item| (item.path.clone(), item.status, item.recovery_path.clone()))
                .collect::<Vec<_>>(),
            vec![
                (
                    alpha.clone(),
                    FileManagerOperationItemStatus::Completed,
                    None
                ),
                (
                    beta.clone(),
                    FileManagerOperationItemStatus::Completed,
                    None
                ),
            ]
        );
        assert_eq!(fs::read(alpha).expect("App bulk alpha output"), b"beta");
        assert_eq!(fs::read(beta).expect("App bulk beta output"), b"alpha");
    }

    // TP-C4.4-RECOVERY: if bulk staging and its rollback both fail, App must
    // retain the exact private recovery path as terminal evidence. The worker
    // lane still reopens for a strictly newer operation without hot retry or
    // a second scheduler.
    #[test]
    fn app_bulk_rename_exposes_private_recovery_path_and_reuses_lane() {
        struct StagingRecoveryFailingHost {
            calls: usize,
        }

        impl crate::fm::rename::RenameOperationHost for StagingRecoveryFailingHost {
            fn before_revalidation(&mut self) -> std::io::Result<()> {
                Ok(())
            }

            fn publish_no_replace(
                &mut self,
                source: &std::path::Path,
                destination: &std::path::Path,
            ) -> std::io::Result<()> {
                self.calls += 1;
                if matches!(self.calls, 2 | 3) {
                    Err(std::io::Error::other(
                        "injected staging and rollback failure",
                    ))
                } else {
                    crate::platform::publish_staged_path_no_replace(source, destination)
                }
            }
        }

        let td = TempDir::new("app-bulk-private-recovery");
        let source_dir = td.root.join("next-source");
        fs::create_dir(&source_dir).expect("create next-operation source directory");
        let alpha = td.root.join("alpha.txt");
        let beta = td.root.join("beta.txt");
        let next = source_dir.join("next.txt");
        fs::write(&alpha, b"alpha").expect("write private recovery alpha");
        fs::write(&beta, b"beta").expect("write private recovery beta");
        fs::write(&next, b"next").expect("write recovery lane source");
        let worker = FileOperationWorker::with_task_executor(
            Arc::new(Notify::new()),
            move |task, cancellation| match task {
                FileOperationWorkerTask::BulkRename(plan) => FileOperationWorkerResult::BulkRename(
                    crate::fm::rename::execute_bulk_rename_operation_with_host(
                        plan,
                        cancellation,
                        &mut StagingRecoveryFailingHost { calls: 0 },
                    ),
                ),
                FileOperationWorkerTask::Transfer(plan) => {
                    FileOperationWorkerResult::Transfer(execute_file_operation(plan, cancellation))
                }
                _ => panic!("unexpected private recovery task"),
            },
        );
        let mut file_manager = crate::fm::FmState::new(&td.root);
        let alpha_index = file_manager
            .entries
            .iter()
            .position(|entry| entry.path == alpha)
            .expect("private recovery alpha row");
        let beta_index = file_manager
            .entries
            .iter()
            .position(|entry| entry.path == beta)
            .expect("private recovery beta row");
        assert!(file_manager.replace_selection(alpha_index));
        assert!(file_manager.toggle_selection(beta_index));
        let mut app = test_app();
        app.file_operation_worker = worker;
        app.state.file_manager = Some(file_manager);
        app.state.request_file_manager_bulk_rename =
            Some(crate::app::state::FileManagerBulkRenameRequest {
                mappings: vec![
                    (alpha.clone(), "renamed-alpha.txt".to_string()),
                    (beta.clone(), "renamed-beta.txt".to_string()),
                ],
            });

        assert!(app.sync_file_operation_worker());
        let failed_generation = app
            .state
            .file_manager_operation
            .as_ref()
            .expect("private recovery operation running")
            .generation;
        let deadline = Instant::now() + Duration::from_secs(5);
        while app
            .state
            .file_manager_operation
            .as_ref()
            .is_some_and(FileManagerOperationState::is_running)
        {
            let _ = app.sync_file_operation_worker();
            assert!(Instant::now() < deadline, "private recovery timed out");
            std::thread::sleep(Duration::from_millis(5));
        }

        let operation = app
            .state
            .file_manager_operation
            .as_ref()
            .expect("private recovery operation terminal");
        assert_eq!(operation.status, FileManagerOperationStatus::Partial);
        assert_eq!(operation.completed_items, 0);
        assert_eq!(operation.failed_items, 2);
        let uncertain = operation
            .items
            .iter()
            .find(|item| item.path == alpha)
            .expect("uncertain alpha projection");
        assert_eq!(uncertain.status, FileManagerOperationItemStatus::Failed);
        let recovery_path = uncertain
            .recovery_path
            .as_ref()
            .expect("private recovery path remains visible");
        assert!(recovery_path.starts_with(&td.root));
        assert!(recovery_path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with(".herdr-rename-stage-")));
        assert_eq!(
            fs::read(recovery_path).expect("read exact private recovery artifact"),
            b"alpha"
        );
        let private_artifacts = fs::read_dir(&td.root)
            .expect("scan private recovery artifacts")
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with(".herdr-rename-stage-"))
            })
            .collect::<Vec<_>>();
        assert_eq!(private_artifacts, vec![recovery_path.clone()]);
        assert!(!app.file_operation_worker.is_busy());

        app.state.file_manager_clipboard = vec![next];
        assert!(app.dispatch_file_manager_header_action(FileManagerHeaderAction::Paste));
        let next_generation = app
            .state
            .file_manager_operation
            .as_ref()
            .expect("post-recovery operation running")
            .generation;
        assert!(next_generation > failed_generation);
        let deadline = Instant::now() + Duration::from_secs(5);
        while app
            .state
            .file_manager_operation
            .as_ref()
            .is_some_and(FileManagerOperationState::is_running)
        {
            let _ = app.sync_file_operation_worker();
            assert!(Instant::now() < deadline, "post-recovery paste timed out");
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(
            app.state
                .file_manager_operation
                .as_ref()
                .expect("post-recovery operation terminal")
                .status,
            FileManagerOperationStatus::Completed
        );
        assert_eq!(
            fs::read(td.root.join("next.txt")).expect("read post-recovery copy"),
            b"next"
        );
    }

    // Keep the single-rename core status imports exercised at the worker
    // boundary so accidental result-type drift is a compile-time failure.
    #[test]
    fn rename_worker_result_status_contract_is_typed() {
        let td = TempDir::new("rename-worker-result-type");
        let source = td.root.join("source.txt");
        fs::write(&source, b"source").expect("write typed rename source");
        let plan = RenameOperationPlan::preflight(RenameOperationRequest {
            source_path: source,
            new_name: "renamed.txt".to_string(),
            operation_in_flight: false,
        })
        .expect("typed rename plan");
        let result = crate::fm::rename::execute_rename_operation(
            &plan,
            &FileOperationCancellation::default(),
        );
        assert_eq!(result.status(), RenameOperationExecutionStatus::Completed);
    }
}
