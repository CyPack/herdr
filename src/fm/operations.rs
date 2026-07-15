use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::SystemTime;

use crate::fm::MAX_MULTI_SELECTION_PATHS;
use crate::platform::FileIdentity;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FileOperationKind {
    Copy,
    Move,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FileOperationRequest {
    pub(crate) kind: FileOperationKind,
    pub(crate) sources: Vec<PathBuf>,
    pub(crate) destination_directory: PathBuf,
    pub(crate) operation_in_flight: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FileOperationPreflightError {
    NoSources,
    TooManySources {
        count: usize,
        limit: usize,
    },
    OperationInFlight,
    DuplicateSource {
        path: PathBuf,
    },
    NonUtf8Path {
        path: PathBuf,
    },
    SourceMissing {
        path: PathBuf,
    },
    SourceUnavailable {
        path: PathBuf,
        kind: io::ErrorKind,
    },
    SourceSymlink {
        path: PathBuf,
    },
    SourceUnsupported {
        path: PathBuf,
    },
    SourceHasNoFileName {
        path: PathBuf,
    },
    DestinationMissing {
        path: PathBuf,
    },
    DestinationUnavailable {
        path: PathBuf,
        kind: io::ErrorKind,
    },
    DestinationSymlink {
        path: PathBuf,
    },
    DestinationNotDirectory {
        path: PathBuf,
    },
    DestinationReadOnly {
        path: PathBuf,
    },
    PathResolutionFailed {
        path: PathBuf,
        kind: io::ErrorKind,
    },
    FileIdentityUnavailable {
        path: PathBuf,
        kind: io::ErrorKind,
    },
    SourceEqualsDestination {
        path: PathBuf,
    },
    DestinationInsideSource {
        source: PathBuf,
        destination_directory: PathBuf,
    },
    DestinationCollision {
        source: PathBuf,
        destination: PathBuf,
    },
    SourceChanged {
        path: PathBuf,
    },
    DestinationChanged {
        path: PathBuf,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlannedSourceKind {
    File,
    Directory,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceSnapshot {
    identity: FileIdentity,
    kind: PlannedSourceKind,
    len: u64,
    modified: Option<SystemTime>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PlannedFileTransfer {
    source: PathBuf,
    destination: PathBuf,
    canonical_source: PathBuf,
    source_snapshot: SourceSnapshot,
}

impl PlannedFileTransfer {
    pub(crate) fn source(&self) -> &Path {
        &self.source
    }

    #[cfg(test)]
    pub(crate) fn destination(&self) -> &Path {
        &self.destination
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FileOperationPlan {
    kind: FileOperationKind,
    destination_directory: PathBuf,
    canonical_destination_directory: PathBuf,
    destination_identity: FileIdentity,
    transfers: Vec<PlannedFileTransfer>,
}

impl FileOperationPlan {
    pub(crate) fn preflight(
        request: FileOperationRequest,
    ) -> Result<Self, FileOperationPreflightError> {
        validate_request_bounds(&request)?;
        validate_utf8_path(&request.destination_directory)?;
        let destination_identity = destination_snapshot(&request.destination_directory)?;
        let canonical_destination_directory = canonicalize(&request.destination_directory)?;

        let mut source_paths = HashSet::with_capacity(request.sources.len());
        let mut source_identities = HashSet::with_capacity(request.sources.len());
        let mut transfers = Vec::with_capacity(request.sources.len());
        for source in request.sources {
            validate_utf8_path(&source)?;
            if !source_paths.insert(source.clone()) {
                return Err(FileOperationPreflightError::DuplicateSource { path: source });
            }
            let source_snapshot = source_snapshot(&source)?;
            if !source_identities.insert(source_snapshot.identity) {
                return Err(FileOperationPreflightError::DuplicateSource { path: source });
            }
            let canonical_source = canonicalize(&source)?;
            let file_name = source.file_name().ok_or_else(|| {
                FileOperationPreflightError::SourceHasNoFileName {
                    path: source.clone(),
                }
            })?;
            let destination = request.destination_directory.join(file_name);
            let canonical_destination = canonical_destination_directory.join(file_name);

            if canonical_source == canonical_destination {
                return Err(FileOperationPreflightError::SourceEqualsDestination { path: source });
            }
            if source_snapshot.kind == PlannedSourceKind::Directory
                && canonical_destination_directory.starts_with(&canonical_source)
            {
                return Err(FileOperationPreflightError::DestinationInsideSource {
                    source,
                    destination_directory: request.destination_directory,
                });
            }
            reject_destination_collision(&source, &destination)?;
            transfers.push(PlannedFileTransfer {
                source,
                destination,
                canonical_source,
                source_snapshot,
            });
        }

        Ok(Self {
            kind: request.kind,
            destination_directory: request.destination_directory,
            canonical_destination_directory,
            destination_identity,
            transfers,
        })
    }

    pub(crate) fn kind(&self) -> FileOperationKind {
        self.kind
    }

    pub(crate) fn destination_directory(&self) -> &Path {
        &self.destination_directory
    }

    pub(crate) fn transfers(&self) -> &[PlannedFileTransfer] {
        &self.transfers
    }

    /// Revalidate all captured authority immediately before future COPY/MOVE
    /// code performs its first write. This method is read-only.
    pub(crate) fn revalidate(&self) -> Result<(), FileOperationPreflightError> {
        let metadata = fs::symlink_metadata(&self.destination_directory).map_err(|_| {
            FileOperationPreflightError::DestinationChanged {
                path: self.destination_directory.clone(),
            }
        })?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(FileOperationPreflightError::DestinationChanged {
                path: self.destination_directory.clone(),
            });
        }
        if metadata.permissions().readonly() {
            return Err(FileOperationPreflightError::DestinationReadOnly {
                path: self.destination_directory.clone(),
            });
        }
        let identity = crate::platform::file_identity(&self.destination_directory, &metadata)
            .map_err(|_| FileOperationPreflightError::DestinationChanged {
                path: self.destination_directory.clone(),
            })?;
        if identity != self.destination_identity
            || canonicalize_for_revalidation(&self.destination_directory)
                != Some(self.canonical_destination_directory.clone())
        {
            return Err(FileOperationPreflightError::DestinationChanged {
                path: self.destination_directory.clone(),
            });
        }

        for transfer in &self.transfers {
            let snapshot = source_snapshot_for_revalidation(&transfer.source).ok_or_else(|| {
                FileOperationPreflightError::SourceChanged {
                    path: transfer.source.clone(),
                }
            })?;
            if snapshot != transfer.source_snapshot
                || canonicalize_for_revalidation(&transfer.source)
                    != Some(transfer.canonical_source.clone())
            {
                return Err(FileOperationPreflightError::SourceChanged {
                    path: transfer.source.clone(),
                });
            }
            reject_destination_collision(&transfer.source, &transfer.destination)?;
        }
        Ok(())
    }
}

fn validate_request_bounds(
    request: &FileOperationRequest,
) -> Result<(), FileOperationPreflightError> {
    if request.operation_in_flight {
        return Err(FileOperationPreflightError::OperationInFlight);
    }
    if request.sources.is_empty() {
        return Err(FileOperationPreflightError::NoSources);
    }
    if request.sources.len() > MAX_MULTI_SELECTION_PATHS {
        return Err(FileOperationPreflightError::TooManySources {
            count: request.sources.len(),
            limit: MAX_MULTI_SELECTION_PATHS,
        });
    }
    Ok(())
}

fn validate_utf8_path(path: &Path) -> Result<(), FileOperationPreflightError> {
    if path.to_str().is_none() {
        return Err(FileOperationPreflightError::NonUtf8Path {
            path: path.to_path_buf(),
        });
    }
    Ok(())
}

fn destination_snapshot(path: &Path) -> Result<FileIdentity, FileOperationPreflightError> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        if error.kind() == io::ErrorKind::NotFound {
            FileOperationPreflightError::DestinationMissing {
                path: path.to_path_buf(),
            }
        } else {
            FileOperationPreflightError::DestinationUnavailable {
                path: path.to_path_buf(),
                kind: error.kind(),
            }
        }
    })?;
    if metadata.file_type().is_symlink() {
        return Err(FileOperationPreflightError::DestinationSymlink {
            path: path.to_path_buf(),
        });
    }
    if !metadata.is_dir() {
        return Err(FileOperationPreflightError::DestinationNotDirectory {
            path: path.to_path_buf(),
        });
    }
    if metadata.permissions().readonly() {
        return Err(FileOperationPreflightError::DestinationReadOnly {
            path: path.to_path_buf(),
        });
    }
    let identity = crate::platform::file_identity(path, &metadata).map_err(|error| {
        FileOperationPreflightError::FileIdentityUnavailable {
            path: path.to_path_buf(),
            kind: error.kind(),
        }
    })?;
    Ok(identity)
}

fn source_snapshot(path: &Path) -> Result<SourceSnapshot, FileOperationPreflightError> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        if error.kind() == io::ErrorKind::NotFound {
            FileOperationPreflightError::SourceMissing {
                path: path.to_path_buf(),
            }
        } else {
            FileOperationPreflightError::SourceUnavailable {
                path: path.to_path_buf(),
                kind: error.kind(),
            }
        }
    })?;
    if metadata.file_type().is_symlink() {
        return Err(FileOperationPreflightError::SourceSymlink {
            path: path.to_path_buf(),
        });
    }
    let kind = if metadata.is_file() {
        PlannedSourceKind::File
    } else if metadata.is_dir() {
        PlannedSourceKind::Directory
    } else {
        return Err(FileOperationPreflightError::SourceUnsupported {
            path: path.to_path_buf(),
        });
    };
    let identity = crate::platform::file_identity(path, &metadata).map_err(|error| {
        FileOperationPreflightError::FileIdentityUnavailable {
            path: path.to_path_buf(),
            kind: error.kind(),
        }
    })?;
    Ok(SourceSnapshot {
        identity,
        kind,
        len: metadata.len(),
        modified: metadata.modified().ok(),
    })
}

fn source_snapshot_for_revalidation(path: &Path) -> Option<SourceSnapshot> {
    let metadata = fs::symlink_metadata(path).ok()?;
    if metadata.file_type().is_symlink() {
        return None;
    }
    let kind = if metadata.is_file() {
        PlannedSourceKind::File
    } else if metadata.is_dir() {
        PlannedSourceKind::Directory
    } else {
        return None;
    };
    let identity = crate::platform::file_identity(path, &metadata).ok()?;
    Some(SourceSnapshot {
        identity,
        kind,
        len: metadata.len(),
        modified: metadata.modified().ok(),
    })
}

fn canonicalize(path: &Path) -> Result<PathBuf, FileOperationPreflightError> {
    fs::canonicalize(path).map_err(|error| FileOperationPreflightError::PathResolutionFailed {
        path: path.to_path_buf(),
        kind: error.kind(),
    })
}

fn canonicalize_for_revalidation(path: &Path) -> Option<PathBuf> {
    fs::canonicalize(path).ok()
}

fn reject_destination_collision(
    source: &Path,
    destination: &Path,
) -> Result<(), FileOperationPreflightError> {
    match fs::symlink_metadata(destination) {
        Ok(_) => Err(FileOperationPreflightError::DestinationCollision {
            source: source.to_path_buf(),
            destination: destination.to_path_buf(),
        }),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(FileOperationPreflightError::DestinationUnavailable {
            path: destination.to_path_buf(),
            kind: error.kind(),
        }),
    }
}

const MAX_OPERATION_TREE_ENTRIES: usize = 1_000_000;

#[derive(Debug, Clone, Default)]
pub(crate) struct FileOperationCancellation {
    cancelled: Arc<AtomicBool>,
}

impl FileOperationCancellation {
    pub(crate) fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    pub(crate) fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FileOperationExecutionStatus {
    Completed,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FileOperationIoAction {
    ReadSource,
    CreateStaging,
    CopyData,
    SetPermissions,
    Publish,
    RemoveSource,
    Cleanup,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FileOperationExecutionError {
    Preflight(FileOperationPreflightError),
    SourceSymlink {
        path: PathBuf,
    },
    SourceUnsupported {
        path: PathBuf,
    },
    TreeEntryLimitExceeded {
        source: PathBuf,
        limit: usize,
    },
    Io {
        path: PathBuf,
        action: FileOperationIoAction,
        kind: io::ErrorKind,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FileOperationItemOutcome {
    NotStarted,
    Committed,
    RolledBack,
    SourceRetained(FileOperationExecutionError),
    Failed(FileOperationExecutionError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FileOperationItemResult {
    source: PathBuf,
    destination: PathBuf,
    outcome: FileOperationItemOutcome,
}

impl FileOperationItemResult {
    pub(crate) fn source(&self) -> &Path {
        &self.source
    }

    #[cfg(test)]
    pub(crate) fn destination(&self) -> &Path {
        &self.destination
    }

    pub(crate) fn outcome(&self) -> &FileOperationItemOutcome {
        &self.outcome
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FileOperationExecutionResult {
    status: FileOperationExecutionStatus,
    items: Vec<FileOperationItemResult>,
}

impl FileOperationExecutionResult {
    pub(crate) fn status(&self) -> FileOperationExecutionStatus {
        self.status
    }

    pub(crate) fn items(&self) -> &[FileOperationItemResult] {
        &self.items
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FileOperationPhase {
    StagingEntry,
    Committing,
    RemovingSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FileOperationProgressEvent {
    item_index: usize,
    phase: FileOperationPhase,
    source: PathBuf,
    destination: PathBuf,
}

impl FileOperationProgressEvent {
    pub(crate) fn item_index(&self) -> usize {
        self.item_index
    }

    #[cfg(test)]
    pub(crate) fn phase(&self) -> FileOperationPhase {
        self.phase
    }
}

struct StagedTransfer {
    item_index: usize,
    container: PathBuf,
    payload: PathBuf,
}

enum CopyWorkItem {
    Copy {
        source: PathBuf,
        destination: PathBuf,
    },
    FinishDirectory {
        source: PathBuf,
        destination: PathBuf,
    },
}

pub(crate) trait FileOperationHost {
    fn publish_no_replace(&mut self, source: &Path, destination: &Path) -> io::Result<()>;
    fn remove_source(&mut self, source: &Path) -> io::Result<()>;
}

struct RealFileOperationHost;

impl FileOperationHost for RealFileOperationHost {
    fn publish_no_replace(&mut self, source: &Path, destination: &Path) -> io::Result<()> {
        crate::platform::publish_staged_path_no_replace(source, destination)
    }

    fn remove_source(&mut self, source: &Path) -> io::Result<()> {
        remove_path(source)
    }
}

#[cfg(test)]
pub(crate) fn execute_file_operation(
    plan: &FileOperationPlan,
    cancellation: &FileOperationCancellation,
) -> FileOperationExecutionResult {
    let mut host = RealFileOperationHost;
    execute_file_operation_with_host(plan, cancellation, &mut host, |_| {})
}

pub(crate) fn execute_file_operation_with_observer<F>(
    plan: &FileOperationPlan,
    cancellation: &FileOperationCancellation,
    observer: F,
) -> FileOperationExecutionResult
where
    F: FnMut(&FileOperationProgressEvent),
{
    let mut host = RealFileOperationHost;
    execute_file_operation_with_host(plan, cancellation, &mut host, observer)
}

pub(crate) fn execute_file_operation_with_host<H, F>(
    plan: &FileOperationPlan,
    cancellation: &FileOperationCancellation,
    host: &mut H,
    mut observer: F,
) -> FileOperationExecutionResult
where
    H: FileOperationHost,
    F: FnMut(&FileOperationProgressEvent),
{
    let mut result = FileOperationExecutionResult {
        status: FileOperationExecutionStatus::Failed,
        items: plan
            .transfers
            .iter()
            .map(|transfer| FileOperationItemResult {
                source: transfer.source.clone(),
                destination: transfer.destination.clone(),
                outcome: FileOperationItemOutcome::NotStarted,
            })
            .collect(),
    };

    if cancellation.is_cancelled() {
        result.status = FileOperationExecutionStatus::Cancelled;
        return result;
    }
    if let Err(error) = plan.revalidate() {
        fail_first_unstarted(&mut result, FileOperationExecutionError::Preflight(error));
        return result;
    }

    if plan.kind == FileOperationKind::Move {
        return execute_move_after_preflight(plan, cancellation, host, &mut observer, result);
    }

    let operation_id = next_operation_id();
    let mut staged = Vec::with_capacity(plan.transfers.len());
    for (item_index, transfer) in plan.transfers.iter().enumerate() {
        if cancellation.is_cancelled() {
            cleanup_staged(&staged);
            result.status = FileOperationExecutionStatus::Cancelled;
            return result;
        }
        match stage_transfer(
            plan,
            transfer,
            item_index,
            operation_id,
            cancellation,
            &mut observer,
        ) {
            Ok(staged_transfer) => staged.push(staged_transfer),
            Err(StageFailure::Cancelled { container }) => {
                cleanup_path(&container);
                cleanup_staged(&staged);
                result.status = FileOperationExecutionStatus::Cancelled;
                return result;
            }
            Err(StageFailure::Failed { container, error }) => {
                cleanup_path(&container);
                cleanup_staged(&staged);
                if let Some(item) = result.items.get_mut(item_index) {
                    item.outcome = FileOperationItemOutcome::Failed(error);
                }
                return result;
            }
        }
    }

    if let Err(error) = plan.revalidate() {
        cleanup_staged(&staged);
        fail_first_unstarted(&mut result, FileOperationExecutionError::Preflight(error));
        return result;
    }

    let mut committed = Vec::new();
    for staged_transfer in &staged {
        let item_index = staged_transfer.item_index;
        let Some(item) = result.items.get(item_index) else {
            cleanup_staged(&staged);
            return result;
        };
        observer(&FileOperationProgressEvent {
            item_index,
            phase: FileOperationPhase::Committing,
            source: item.source.clone(),
            destination: item.destination.clone(),
        });
        if cancellation.is_cancelled() {
            rollback_committed(&mut result, &committed);
            cleanup_staged(&staged);
            result.status = FileOperationExecutionStatus::Cancelled;
            return result;
        }
        if let Err(error) = host.publish_no_replace(&staged_transfer.payload, &item.destination) {
            let execution_error = FileOperationExecutionError::Io {
                path: item.destination.clone(),
                action: FileOperationIoAction::Publish,
                kind: error.kind(),
            };
            rollback_committed(&mut result, &committed);
            cleanup_staged(&staged);
            if let Some(item) = result.items.get_mut(item_index) {
                item.outcome = FileOperationItemOutcome::Failed(execution_error);
            }
            return result;
        }
        cleanup_path(&staged_transfer.container);
        if let Some(item) = result.items.get_mut(item_index) {
            item.outcome = FileOperationItemOutcome::Committed;
        }
        committed.push(item_index);
    }

    result.status = FileOperationExecutionStatus::Completed;
    result
}

fn execute_move_after_preflight<H, F>(
    plan: &FileOperationPlan,
    cancellation: &FileOperationCancellation,
    host: &mut H,
    observer: &mut F,
    mut result: FileOperationExecutionResult,
) -> FileOperationExecutionResult
where
    H: FileOperationHost,
    F: FnMut(&FileOperationProgressEvent),
{
    let operation_id = next_operation_id();
    for (item_index, transfer) in plan.transfers.iter().enumerate() {
        if cancellation.is_cancelled() {
            result.status = FileOperationExecutionStatus::Cancelled;
            return result;
        }
        observer(&FileOperationProgressEvent {
            item_index,
            phase: FileOperationPhase::Committing,
            source: transfer.source.clone(),
            destination: transfer.destination.clone(),
        });
        if cancellation.is_cancelled() {
            result.status = FileOperationExecutionStatus::Cancelled;
            return result;
        }

        match host.publish_no_replace(&transfer.source, &transfer.destination) {
            Ok(()) => {
                if let Some(item) = result.items.get_mut(item_index) {
                    item.outcome = FileOperationItemOutcome::Committed;
                }
            }
            Err(error) if error.kind() == io::ErrorKind::CrossesDevices => {
                let staged_transfer = match stage_transfer(
                    plan,
                    transfer,
                    item_index,
                    operation_id,
                    cancellation,
                    observer,
                ) {
                    Ok(staged_transfer) => staged_transfer,
                    Err(StageFailure::Cancelled { container }) => {
                        cleanup_path(&container);
                        result.status = FileOperationExecutionStatus::Cancelled;
                        return result;
                    }
                    Err(StageFailure::Failed { container, error }) => {
                        cleanup_path(&container);
                        if let Some(item) = result.items.get_mut(item_index) {
                            item.outcome = FileOperationItemOutcome::Failed(error);
                        }
                        return result;
                    }
                };

                if cancellation.is_cancelled() {
                    cleanup_path(&staged_transfer.container);
                    result.status = FileOperationExecutionStatus::Cancelled;
                    return result;
                }
                observer(&FileOperationProgressEvent {
                    item_index,
                    phase: FileOperationPhase::Committing,
                    source: transfer.source.clone(),
                    destination: transfer.destination.clone(),
                });
                if cancellation.is_cancelled() {
                    cleanup_path(&staged_transfer.container);
                    result.status = FileOperationExecutionStatus::Cancelled;
                    return result;
                }
                if let Err(error) =
                    host.publish_no_replace(&staged_transfer.payload, &transfer.destination)
                {
                    cleanup_path(&staged_transfer.container);
                    if let Some(item) = result.items.get_mut(item_index) {
                        item.outcome =
                            FileOperationItemOutcome::Failed(FileOperationExecutionError::Io {
                                path: transfer.destination.clone(),
                                action: FileOperationIoAction::Publish,
                                kind: error.kind(),
                            });
                    }
                    return result;
                }
                cleanup_path(&staged_transfer.container);

                observer(&FileOperationProgressEvent {
                    item_index,
                    phase: FileOperationPhase::RemovingSource,
                    source: transfer.source.clone(),
                    destination: transfer.destination.clone(),
                });
                if cancellation.is_cancelled() {
                    let rollback_error = remove_path(&transfer.destination).err();
                    if let Some(item) = result.items.get_mut(item_index) {
                        item.outcome = match rollback_error {
                            None => FileOperationItemOutcome::RolledBack,
                            Some(error) => FileOperationItemOutcome::SourceRetained(
                                FileOperationExecutionError::Io {
                                    path: transfer.destination.clone(),
                                    action: FileOperationIoAction::Cleanup,
                                    kind: error.kind(),
                                },
                            ),
                        };
                    }
                    result.status = FileOperationExecutionStatus::Cancelled;
                    return result;
                }
                if let Err(error) = host.remove_source(&transfer.source) {
                    if let Some(item) = result.items.get_mut(item_index) {
                        item.outcome = FileOperationItemOutcome::SourceRetained(
                            FileOperationExecutionError::Io {
                                path: transfer.source.clone(),
                                action: FileOperationIoAction::RemoveSource,
                                kind: error.kind(),
                            },
                        );
                    }
                    return result;
                }
                if let Some(item) = result.items.get_mut(item_index) {
                    item.outcome = FileOperationItemOutcome::Committed;
                }
            }
            Err(error) => {
                if let Some(item) = result.items.get_mut(item_index) {
                    item.outcome =
                        FileOperationItemOutcome::Failed(FileOperationExecutionError::Io {
                            path: transfer.destination.clone(),
                            action: FileOperationIoAction::Publish,
                            kind: error.kind(),
                        });
                }
                return result;
            }
        }
    }

    result.status = FileOperationExecutionStatus::Completed;
    result
}

enum StageFailure {
    Cancelled {
        container: PathBuf,
    },
    Failed {
        container: PathBuf,
        error: FileOperationExecutionError,
    },
}

fn stage_transfer<F>(
    plan: &FileOperationPlan,
    transfer: &PlannedFileTransfer,
    item_index: usize,
    operation_id: u64,
    cancellation: &FileOperationCancellation,
    observer: &mut F,
) -> Result<StagedTransfer, StageFailure>
where
    F: FnMut(&FileOperationProgressEvent),
{
    let container = plan.destination_directory.join(format!(
        ".herdr-operation-{}-{operation_id}-{item_index}",
        std::process::id()
    ));
    if let Err(error) = fs::create_dir(&container) {
        return Err(StageFailure::Failed {
            container: container.clone(),
            error: FileOperationExecutionError::Io {
                path: container,
                action: FileOperationIoAction::CreateStaging,
                kind: error.kind(),
            },
        });
    }
    let payload = container.join("payload");
    match copy_path_bounded(
        &transfer.source,
        &payload,
        item_index,
        cancellation,
        observer,
    ) {
        Ok(()) => Ok(StagedTransfer {
            item_index,
            container,
            payload,
        }),
        Err(CopyFailure::Cancelled) => Err(StageFailure::Cancelled { container }),
        Err(CopyFailure::Failed(error)) => Err(StageFailure::Failed { container, error }),
    }
}

enum CopyFailure {
    Cancelled,
    Failed(FileOperationExecutionError),
}

fn copy_path_bounded<F>(
    source: &Path,
    destination: &Path,
    item_index: usize,
    cancellation: &FileOperationCancellation,
    observer: &mut F,
) -> Result<(), CopyFailure>
where
    F: FnMut(&FileOperationProgressEvent),
{
    let root_source = source.to_path_buf();
    let mut processed = 0_usize;
    let mut work = vec![CopyWorkItem::Copy {
        source: source.to_path_buf(),
        destination: destination.to_path_buf(),
    }];
    while let Some(item) = work.pop() {
        if cancellation.is_cancelled() {
            return Err(CopyFailure::Cancelled);
        }
        processed = processed.saturating_add(1);
        if processed > MAX_OPERATION_TREE_ENTRIES {
            return Err(CopyFailure::Failed(
                FileOperationExecutionError::TreeEntryLimitExceeded {
                    source: root_source,
                    limit: MAX_OPERATION_TREE_ENTRIES,
                },
            ));
        }
        match item {
            CopyWorkItem::Copy {
                source,
                destination,
            } => {
                observer(&FileOperationProgressEvent {
                    item_index,
                    phase: FileOperationPhase::StagingEntry,
                    source: source.clone(),
                    destination: destination.clone(),
                });
                if cancellation.is_cancelled() {
                    return Err(CopyFailure::Cancelled);
                }
                let metadata = fs::symlink_metadata(&source).map_err(|error| {
                    CopyFailure::Failed(FileOperationExecutionError::Io {
                        path: source.clone(),
                        action: FileOperationIoAction::ReadSource,
                        kind: error.kind(),
                    })
                })?;
                if metadata.file_type().is_symlink() {
                    return Err(CopyFailure::Failed(
                        FileOperationExecutionError::SourceSymlink { path: source },
                    ));
                }
                if metadata.is_file() {
                    fs::copy(&source, &destination).map_err(|error| {
                        CopyFailure::Failed(FileOperationExecutionError::Io {
                            path: destination,
                            action: FileOperationIoAction::CopyData,
                            kind: error.kind(),
                        })
                    })?;
                } else if metadata.is_dir() {
                    fs::create_dir(&destination).map_err(|error| {
                        CopyFailure::Failed(FileOperationExecutionError::Io {
                            path: destination.clone(),
                            action: FileOperationIoAction::CopyData,
                            kind: error.kind(),
                        })
                    })?;
                    let mut children = Vec::new();
                    let read_dir = fs::read_dir(&source).map_err(|error| {
                        CopyFailure::Failed(FileOperationExecutionError::Io {
                            path: source.clone(),
                            action: FileOperationIoAction::ReadSource,
                            kind: error.kind(),
                        })
                    })?;
                    for entry in read_dir {
                        let entry = entry.map_err(|error| {
                            CopyFailure::Failed(FileOperationExecutionError::Io {
                                path: source.clone(),
                                action: FileOperationIoAction::ReadSource,
                                kind: error.kind(),
                            })
                        })?;
                        children.push((entry.path(), destination.join(entry.file_name())));
                        if processed.saturating_add(children.len()) > MAX_OPERATION_TREE_ENTRIES {
                            return Err(CopyFailure::Failed(
                                FileOperationExecutionError::TreeEntryLimitExceeded {
                                    source: root_source,
                                    limit: MAX_OPERATION_TREE_ENTRIES,
                                },
                            ));
                        }
                    }
                    children.sort_by(|left, right| left.0.cmp(&right.0));
                    work.push(CopyWorkItem::FinishDirectory {
                        source: source.clone(),
                        destination: destination.clone(),
                    });
                    work.extend(children.into_iter().rev().map(|(source, destination)| {
                        CopyWorkItem::Copy {
                            source,
                            destination,
                        }
                    }));
                } else {
                    return Err(CopyFailure::Failed(
                        FileOperationExecutionError::SourceUnsupported { path: source },
                    ));
                }
            }
            CopyWorkItem::FinishDirectory {
                source,
                destination,
            } => {
                let permissions = fs::symlink_metadata(&source)
                    .map_err(|error| {
                        CopyFailure::Failed(FileOperationExecutionError::Io {
                            path: source,
                            action: FileOperationIoAction::ReadSource,
                            kind: error.kind(),
                        })
                    })?
                    .permissions();
                fs::set_permissions(&destination, permissions).map_err(|error| {
                    CopyFailure::Failed(FileOperationExecutionError::Io {
                        path: destination,
                        action: FileOperationIoAction::SetPermissions,
                        kind: error.kind(),
                    })
                })?;
            }
        }
    }
    Ok(())
}

fn next_operation_id() -> u64 {
    static NEXT_OPERATION_ID: AtomicU64 = AtomicU64::new(1);
    NEXT_OPERATION_ID.fetch_add(1, Ordering::Relaxed)
}

fn cleanup_staged(staged: &[StagedTransfer]) {
    for transfer in staged {
        cleanup_path(&transfer.container);
    }
}

fn cleanup_path(path: &Path) {
    if let Err(error) = remove_path(path) {
        if error.kind() == io::ErrorKind::NotFound {
            return;
        }
        tracing::warn!(?path, %error, "fm: failed to clean file-operation staging path");
    }
}

fn remove_path(path: &Path) -> io::Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.is_dir() && !metadata.file_type().is_symlink() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}

fn rollback_committed(result: &mut FileOperationExecutionResult, committed: &[usize]) {
    for item_index in committed.iter().rev().copied() {
        let Some(item) = result.items.get_mut(item_index) else {
            continue;
        };
        let path = item.destination.clone();
        let existed = fs::symlink_metadata(&path).is_ok();
        cleanup_path(&path);
        if existed && fs::symlink_metadata(&path).is_err() {
            item.outcome = FileOperationItemOutcome::RolledBack;
        } else {
            item.outcome = FileOperationItemOutcome::Failed(FileOperationExecutionError::Io {
                path,
                action: FileOperationIoAction::Cleanup,
                kind: io::ErrorKind::Other,
            });
        }
    }
}

fn fail_first_unstarted(
    result: &mut FileOperationExecutionResult,
    error: FileOperationExecutionError,
) {
    if let Some(item) = result.items.first_mut() {
        item.outcome = FileOperationItemOutcome::Failed(error);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        execute_file_operation, execute_file_operation_with_host,
        execute_file_operation_with_observer, FileOperationCancellation,
        FileOperationExecutionError, FileOperationExecutionStatus, FileOperationHost,
        FileOperationItemOutcome, FileOperationKind, FileOperationPhase, FileOperationPlan,
        FileOperationPreflightError, FileOperationRequest,
    };
    use crate::fm::MAX_MULTI_SELECTION_PATHS;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

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
                "herdr-fm-operation-test-{}-{}-{}",
                std::process::id(),
                tag,
                unique()
            ));
            fs::create_dir_all(&root).expect("create isolated operation test root");
            Self { root }
        }

        fn dir(&self, relative: &str) -> PathBuf {
            let path = self.root.join(relative);
            fs::create_dir_all(&path).expect("create isolated operation test directory");
            path
        }

        fn file(&self, relative: &str, contents: &[u8]) -> PathBuf {
            let path = self.root.join(relative);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("create operation test file parent");
            }
            fs::write(&path, contents).expect("write isolated operation test file");
            path
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn request(
        kind: FileOperationKind,
        sources: Vec<PathBuf>,
        destination_directory: PathBuf,
    ) -> FileOperationRequest {
        FileOperationRequest {
            kind,
            sources,
            destination_directory,
            operation_in_flight: false,
        }
    }

    fn operation_artifacts(directory: &PathBuf) -> Vec<PathBuf> {
        fs::read_dir(directory)
            .expect("read operation destination")
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().starts_with(".herdr-"))
            .map(|entry| entry.path())
            .collect()
    }

    struct CrossDeviceHost {
        direct_source: PathBuf,
        fail_remove: bool,
    }

    impl FileOperationHost for CrossDeviceHost {
        fn publish_no_replace(
            &mut self,
            source: &std::path::Path,
            destination: &std::path::Path,
        ) -> std::io::Result<()> {
            if source == self.direct_source {
                return Err(std::io::Error::from(std::io::ErrorKind::CrossesDevices));
            }
            crate::platform::publish_staged_path_no_replace(source, destination)
        }

        fn remove_source(&mut self, source: &std::path::Path) -> std::io::Result<()> {
            if self.fail_remove {
                return Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied));
            }
            let metadata = fs::symlink_metadata(source)?;
            if metadata.is_dir() && !metadata.file_type().is_symlink() {
                fs::remove_dir_all(source)
            } else {
                fs::remove_file(source)
            }
        }
    }

    // TP-C4.1-MOVE: a same-filesystem move uses direct atomic no-replace
    // publish and creates exactly one destination without staging residue.
    #[test]
    fn file_operation_move_same_filesystem_renames_without_copy_residue() {
        let td = TempDir::new("move-same-filesystem");
        let source = td.file("source.txt", b"move contents");
        let destination = td.dir("destination");
        let plan = FileOperationPlan::preflight(request(
            FileOperationKind::Move,
            vec![source.clone()],
            destination.clone(),
        ))
        .expect("move plan");

        let result = execute_file_operation(&plan, &FileOperationCancellation::default());

        assert_eq!(result.status(), FileOperationExecutionStatus::Completed);
        assert_eq!(
            result.items()[0].outcome(),
            &FileOperationItemOutcome::Committed
        );
        assert!(!source.exists());
        assert_eq!(
            fs::read(destination.join("source.txt")).expect("read moved file"),
            b"move contents"
        );
        assert!(operation_artifacts(&destination).is_empty());
    }

    // TP-C4.1-MOVE: EXDEV falls back to staged copy, publishes the verified
    // destination, and only then removes the original source.
    #[test]
    fn file_operation_move_cross_filesystem_commits_copy_before_source_removal() {
        let td = TempDir::new("move-cross-filesystem");
        let source = td.dir("source-tree");
        td.file("source-tree/nested.txt", b"cross-device contents");
        let destination = td.dir("destination");
        let plan = FileOperationPlan::preflight(request(
            FileOperationKind::Move,
            vec![source.clone()],
            destination.clone(),
        ))
        .expect("move plan");
        let mut host = CrossDeviceHost {
            direct_source: source.clone(),
            fail_remove: false,
        };

        let result = execute_file_operation_with_host(
            &plan,
            &FileOperationCancellation::default(),
            &mut host,
            |_| {},
        );

        assert_eq!(result.status(), FileOperationExecutionStatus::Completed);
        assert_eq!(
            result.items()[0].outcome(),
            &FileOperationItemOutcome::Committed
        );
        assert!(!source.exists());
        assert_eq!(
            fs::read(destination.join("source-tree/nested.txt"))
                .expect("read fallback move output"),
            b"cross-device contents"
        );
        assert!(operation_artifacts(&destination).is_empty());
    }

    // TP-C4.1-MOVE: if destructive source removal fails after the copied
    // destination commits, the explicit partial result preserves both paths.
    #[test]
    fn file_operation_move_source_removal_failure_is_explicit_and_recoverable() {
        let td = TempDir::new("move-source-retained");
        let source = td.file("source.txt", b"retained source");
        let destination = td.dir("destination");
        let plan = FileOperationPlan::preflight(request(
            FileOperationKind::Move,
            vec![source.clone()],
            destination.clone(),
        ))
        .expect("move plan");
        let mut host = CrossDeviceHost {
            direct_source: source.clone(),
            fail_remove: true,
        };

        let result = execute_file_operation_with_host(
            &plan,
            &FileOperationCancellation::default(),
            &mut host,
            |_| {},
        );

        assert_eq!(result.status(), FileOperationExecutionStatus::Failed);
        assert!(matches!(
            result.items()[0].outcome(),
            FileOperationItemOutcome::SourceRetained(FileOperationExecutionError::Io {
                action: super::FileOperationIoAction::RemoveSource,
                kind: std::io::ErrorKind::PermissionDenied,
                ..
            })
        ));
        assert_eq!(
            fs::read(&source).expect("source retained"),
            b"retained source"
        );
        assert_eq!(
            fs::read(destination.join("source.txt")).expect("destination committed"),
            b"retained source"
        );
        assert!(operation_artifacts(&destination).is_empty());
    }

    // TP-C4.1-MOVE: fallback traversal failure never reaches the destructive
    // phase, so the complete source remains and no destination is published.
    #[cfg(unix)]
    #[test]
    fn file_operation_move_fallback_failure_never_removes_source() {
        let td = TempDir::new("move-fallback-failure");
        let source = td.dir("source-tree");
        td.file("source-tree/00-file.txt", b"source");
        std::os::unix::fs::symlink(td.root.join("outside"), td.root.join("source-tree/99-link"))
            .expect("create unsupported nested symlink");
        let destination = td.dir("destination");
        let plan = FileOperationPlan::preflight(request(
            FileOperationKind::Move,
            vec![source.clone()],
            destination.clone(),
        ))
        .expect("move plan");
        let mut host = CrossDeviceHost {
            direct_source: source.clone(),
            fail_remove: false,
        };

        let result = execute_file_operation_with_host(
            &plan,
            &FileOperationCancellation::default(),
            &mut host,
            |_| {},
        );

        assert_eq!(result.status(), FileOperationExecutionStatus::Failed);
        assert!(source.exists());
        assert!(source.join("00-file.txt").exists());
        assert!(!destination.join("source-tree").exists());
        assert!(operation_artifacts(&destination).is_empty());
    }

    // TP-C4.1-COPY: file, directory, and multi-source copy stage completely,
    // publish in prepared order, preserve source data, and leave no temp data.
    #[test]
    fn file_operation_copy_commits_files_directories_and_metadata() {
        let td = TempDir::new("copy-success");
        let first = td.file("source/first.txt", b"first contents");
        let directory = td.dir("source/tree");
        td.file("source/tree/nested/child.txt", b"nested contents");
        let destination = td.dir("destination");

        #[cfg(unix)]
        {
            use std::os::unix::fs::{MetadataExt, PermissionsExt};

            fs::set_permissions(&first, fs::Permissions::from_mode(0o640))
                .expect("set source mode");
            assert_eq!(
                fs::metadata(&first).expect("source metadata").mode() & 0o777,
                0o640
            );
        }

        let plan = FileOperationPlan::preflight(request(
            FileOperationKind::Copy,
            vec![first.clone(), directory.clone()],
            destination.clone(),
        ))
        .expect("copy plan");
        let result = execute_file_operation(&plan, &FileOperationCancellation::default());

        assert_eq!(result.status(), FileOperationExecutionStatus::Completed);
        assert_eq!(result.items().len(), 2);
        assert_eq!(result.items()[0].source(), first);
        assert_eq!(result.items()[1].source(), directory);
        assert!(result
            .items()
            .iter()
            .all(|item| item.outcome() == &FileOperationItemOutcome::Committed));
        assert_eq!(
            fs::read(destination.join("first.txt")).expect("read copied file"),
            b"first contents"
        );
        assert_eq!(
            fs::read(destination.join("tree/nested/child.txt")).expect("read copied nested file"),
            b"nested contents"
        );
        assert_eq!(
            fs::read(&first).expect("source file remains"),
            b"first contents"
        );
        assert!(directory.exists());
        assert!(operation_artifacts(&destination).is_empty());

        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;

            assert_eq!(
                fs::metadata(destination.join("first.txt"))
                    .expect("copied metadata")
                    .mode()
                    & 0o777,
                0o640
            );
        }
    }

    // TP-C4.1-COPY: an unsupported nested entry fails the item explicitly and
    // removes every staged byte before any destination becomes visible.
    #[cfg(unix)]
    #[test]
    fn file_operation_copy_failure_cleans_staging_without_partial_publish() {
        let td = TempDir::new("copy-failure-cleanup");
        let directory = td.dir("source/tree");
        td.file("source/tree/00-before.txt", b"would have been staged");
        std::os::unix::fs::symlink(td.root.join("outside"), td.root.join("source/tree/99-link"))
            .expect("create unsupported nested symlink");
        let destination = td.dir("destination");
        let plan = FileOperationPlan::preflight(request(
            FileOperationKind::Copy,
            vec![directory.clone()],
            destination.clone(),
        ))
        .expect("top-level directory plan");

        let result = execute_file_operation(&plan, &FileOperationCancellation::default());

        assert_eq!(result.status(), FileOperationExecutionStatus::Failed);
        assert_eq!(result.items().len(), 1);
        assert!(matches!(
            result.items()[0].outcome(),
            FileOperationItemOutcome::Failed(FileOperationExecutionError::SourceSymlink {
                path
            }) if path.ends_with("99-link")
        ));
        assert!(!destination.join("tree").exists());
        assert!(operation_artifacts(&destination).is_empty());
        assert!(directory.exists());
    }

    // TP-C4.1-COPY: cancellation is idempotent both before start and after a
    // staging event, and neither path publishes or leaks temp artifacts.
    #[test]
    fn file_operation_copy_cancellation_is_idempotent_and_cleans_staging() {
        let td = TempDir::new("copy-cancel");
        let source = td.file("source.txt", b"source contents");
        let destination = td.dir("destination");
        let plan = FileOperationPlan::preflight(request(
            FileOperationKind::Copy,
            vec![source.clone()],
            destination.clone(),
        ))
        .expect("copy plan");

        let cancelled = FileOperationCancellation::default();
        cancelled.cancel();
        cancelled.cancel();
        let before_start = execute_file_operation(&plan, &cancelled);
        assert_eq!(
            before_start.status(),
            FileOperationExecutionStatus::Cancelled
        );
        assert_eq!(
            before_start.items()[0].outcome(),
            &FileOperationItemOutcome::NotStarted
        );
        assert!(!destination.join("source.txt").exists());
        assert!(operation_artifacts(&destination).is_empty());

        let during_staging = FileOperationCancellation::default();
        let result = execute_file_operation_with_observer(&plan, &during_staging, |event| {
            if event.phase() == FileOperationPhase::StagingEntry {
                during_staging.cancel();
            }
        });
        assert_eq!(result.status(), FileOperationExecutionStatus::Cancelled);
        assert_eq!(
            result.items()[0].outcome(),
            &FileOperationItemOutcome::NotStarted
        );
        assert!(!destination.join("source.txt").exists());
        assert!(operation_artifacts(&destination).is_empty());
        assert!(source.exists());
    }

    // TP-C4.1-COPY: a target created after the final revalidation but before
    // publish is never replaced; the racing writer's bytes remain authoritative.
    #[test]
    fn file_operation_copy_publish_is_atomic_no_replace() {
        let td = TempDir::new("copy-no-replace");
        let source = td.file("source.txt", b"copy bytes");
        let destination = td.dir("destination");
        let published = destination.join("source.txt");
        let plan = FileOperationPlan::preflight(request(
            FileOperationKind::Copy,
            vec![source.clone()],
            destination.clone(),
        ))
        .expect("copy plan");

        let result = execute_file_operation_with_observer(
            &plan,
            &FileOperationCancellation::default(),
            |event| {
                if event.phase() == FileOperationPhase::Committing {
                    fs::write(&published, b"racing writer").expect("create late collision");
                }
            },
        );

        assert_eq!(result.status(), FileOperationExecutionStatus::Failed);
        assert!(matches!(
            result.items()[0].outcome(),
            FileOperationItemOutcome::Failed(FileOperationExecutionError::Io {
                action: super::FileOperationIoAction::Publish,
                ..
            })
        ));
        assert_eq!(
            fs::read(&published).expect("read racing target"),
            b"racing writer"
        );
        assert_eq!(fs::read(&source).expect("source remains"), b"copy bytes");
        assert!(operation_artifacts(&destination).is_empty());
    }

    // TP-C4.1-COPY: cancellation between item commits rolls back already
    // published copy outputs and reports the distinction from untouched items.
    #[test]
    fn file_operation_copy_cancel_between_commits_rolls_back_published_items() {
        let td = TempDir::new("copy-commit-cancel");
        let first = td.file("source/first.txt", b"first");
        let second = td.file("source/second.txt", b"second");
        let destination = td.dir("destination");
        let plan = FileOperationPlan::preflight(request(
            FileOperationKind::Copy,
            vec![first.clone(), second.clone()],
            destination.clone(),
        ))
        .expect("multi-copy plan");
        let cancellation = FileOperationCancellation::default();
        let mut commit_count = 0;

        let result = execute_file_operation_with_observer(&plan, &cancellation, |event| {
            if event.phase() == FileOperationPhase::Committing {
                commit_count += 1;
                if commit_count == 2 {
                    cancellation.cancel();
                }
            }
        });

        assert_eq!(result.status(), FileOperationExecutionStatus::Cancelled);
        assert_eq!(
            result.items()[0].outcome(),
            &FileOperationItemOutcome::RolledBack
        );
        assert_eq!(
            result.items()[1].outcome(),
            &FileOperationItemOutcome::NotStarted
        );
        assert_eq!(
            result.items()[0].destination(),
            destination.join("first.txt")
        );
        assert!(!destination.join("first.txt").exists());
        assert!(!destination.join("second.txt").exists());
        assert!(operation_artifacts(&destination).is_empty());
        assert!(first.exists());
        assert!(second.exists());
    }

    // TP-C4.1-PREFLIGHT-PLAN: exact prepared path order survives planning and
    // every destination is derived once without performing a write.
    #[test]
    fn file_operation_preflight_builds_ordered_immutable_plan() {
        let td = TempDir::new("ordered-plan");
        let first = td.file("source/b.txt", b"second");
        let second = td.file("source/a.txt", b"first");
        let destination = td.dir("destination");

        let plan = FileOperationPlan::preflight(request(
            FileOperationKind::Copy,
            vec![first.clone(), second.clone()],
            destination.clone(),
        ))
        .expect("valid exact paths produce a plan");

        assert_eq!(plan.kind(), FileOperationKind::Copy);
        assert_eq!(plan.destination_directory(), destination);
        assert_eq!(plan.transfers().len(), 2);
        assert_eq!(plan.transfers()[0].source(), first);
        assert_eq!(plan.transfers()[0].destination(), destination.join("b.txt"));
        assert_eq!(plan.transfers()[1].source(), second);
        assert_eq!(plan.transfers()[1].destination(), destination.join("a.txt"));
        assert_eq!(plan.revalidate(), Ok(()));
        assert!(!destination.join("b.txt").exists());
        assert!(!destination.join("a.txt").exists());
    }

    // TP-C4.1-PREFLIGHT-BOUNDS: empty, oversized, duplicate, and concurrent
    // requests fail before metadata traversal or any filesystem mutation.
    #[test]
    fn file_operation_preflight_rejects_invalid_bounds_and_in_flight_state() {
        let td = TempDir::new("bounds");
        let source = td.file("source.txt", b"source");
        let destination = td.dir("destination");

        assert_eq!(
            FileOperationPlan::preflight(request(
                FileOperationKind::Copy,
                Vec::new(),
                destination.clone(),
            )),
            Err(FileOperationPreflightError::NoSources)
        );

        let oversized = (0..=MAX_MULTI_SELECTION_PATHS)
            .map(|index| td.root.join(format!("not-read-{index}")))
            .collect::<Vec<_>>();
        assert_eq!(
            FileOperationPlan::preflight(request(
                FileOperationKind::Copy,
                oversized,
                destination.clone(),
            )),
            Err(FileOperationPreflightError::TooManySources {
                count: MAX_MULTI_SELECTION_PATHS + 1,
                limit: MAX_MULTI_SELECTION_PATHS,
            })
        );

        assert_eq!(
            FileOperationPlan::preflight(request(
                FileOperationKind::Move,
                vec![source.clone(), source.clone()],
                destination.clone(),
            )),
            Err(FileOperationPreflightError::DuplicateSource {
                path: source.clone()
            })
        );

        let mut in_flight = request(FileOperationKind::Copy, vec![source], destination.clone());
        in_flight.operation_in_flight = true;
        assert_eq!(
            FileOperationPlan::preflight(in_flight),
            Err(FileOperationPreflightError::OperationInFlight)
        );
        assert!(fs::read_dir(destination)
            .expect("read untouched destination")
            .next()
            .is_none());
    }

    // TP-C4.1-PREFLIGHT-SOURCE: missing, symlink, and special sources are not
    // followed or coerced into a supported operation.
    #[cfg(unix)]
    #[test]
    fn file_operation_preflight_rejects_untrusted_source_types() {
        use std::os::unix::ffi::OsStringExt;

        let td = TempDir::new("source-types");
        let destination = td.dir("destination");
        let missing = td.root.join("missing.txt");
        assert_eq!(
            FileOperationPlan::preflight(request(
                FileOperationKind::Copy,
                vec![missing.clone()],
                destination.clone(),
            )),
            Err(FileOperationPreflightError::SourceMissing { path: missing })
        );

        let target = td.file("target.txt", b"target");
        let link = td.root.join("link.txt");
        std::os::unix::fs::symlink(&target, &link).expect("create isolated source symlink");
        assert_eq!(
            FileOperationPlan::preflight(request(
                FileOperationKind::Copy,
                vec![link.clone()],
                destination.clone(),
            )),
            Err(FileOperationPreflightError::SourceSymlink { path: link })
        );

        let socket = td.root.join("agent.sock");
        let _listener =
            std::os::unix::net::UnixListener::bind(&socket).expect("bind isolated source socket");
        assert_eq!(
            FileOperationPlan::preflight(request(
                FileOperationKind::Copy,
                vec![socket.clone()],
                destination.clone(),
            )),
            Err(FileOperationPreflightError::SourceUnsupported { path: socket })
        );

        let non_utf8 = td.root.join(std::ffi::OsString::from_vec(vec![b'b', 0xff]));
        fs::write(&non_utf8, b"opaque").expect("write isolated non-UTF-8 source");
        assert_eq!(
            FileOperationPlan::preflight(request(
                FileOperationKind::Copy,
                vec![non_utf8.clone()],
                destination,
            )),
            Err(FileOperationPreflightError::NonUtf8Path { path: non_utf8 })
        );
    }

    // TP-C4.1-PREFLIGHT-DEST: the destination authority must be one existing,
    // real, writable directory; file and symlink aliases fail closed.
    #[cfg(unix)]
    #[test]
    fn file_operation_preflight_rejects_invalid_destination_authority() {
        use std::os::unix::fs::PermissionsExt;

        let td = TempDir::new("destination-authority");
        let source = td.file("source.txt", b"source");
        let missing = td.root.join("missing-destination");
        assert_eq!(
            FileOperationPlan::preflight(request(
                FileOperationKind::Copy,
                vec![source.clone()],
                missing.clone(),
            )),
            Err(FileOperationPreflightError::DestinationMissing { path: missing })
        );

        let file = td.file("destination-file", b"not a directory");
        assert_eq!(
            FileOperationPlan::preflight(request(
                FileOperationKind::Copy,
                vec![source.clone()],
                file.clone(),
            )),
            Err(FileOperationPreflightError::DestinationNotDirectory { path: file })
        );

        let real_directory = td.dir("real-destination");
        let link = td.root.join("destination-link");
        std::os::unix::fs::symlink(&real_directory, &link)
            .expect("create isolated destination symlink");
        assert_eq!(
            FileOperationPlan::preflight(request(
                FileOperationKind::Copy,
                vec![source.clone()],
                link.clone(),
            )),
            Err(FileOperationPreflightError::DestinationSymlink { path: link })
        );

        let read_only = td.dir("read-only-destination");
        let original_mode = fs::metadata(&read_only)
            .expect("read destination permissions")
            .permissions()
            .mode();
        fs::set_permissions(
            &read_only,
            fs::Permissions::from_mode(original_mode & !0o222),
        )
        .expect("make isolated destination read-only");
        assert_eq!(
            FileOperationPlan::preflight(request(
                FileOperationKind::Copy,
                vec![source],
                read_only.clone(),
            )),
            Err(FileOperationPreflightError::DestinationReadOnly {
                path: read_only.clone(),
            })
        );
        fs::set_permissions(&read_only, fs::Permissions::from_mode(original_mode))
            .expect("restore isolated destination permissions");
    }

    // TP-C4.1-PREFLIGHT-RELATION: no-overwrite is the default, and a source
    // may never resolve to itself or contain its own destination directory.
    #[test]
    fn file_operation_preflight_rejects_collision_same_path_and_descendant() {
        let td = TempDir::new("path-relations");
        let source = td.file("source/file.txt", b"source");
        let destination = td.dir("destination");
        let collision = td.file("destination/file.txt", b"existing");
        assert_eq!(
            FileOperationPlan::preflight(request(
                FileOperationKind::Copy,
                vec![source.clone()],
                destination,
            )),
            Err(FileOperationPreflightError::DestinationCollision {
                source: source.clone(),
                destination: collision,
            })
        );

        let source_parent = source.parent().expect("source parent").to_path_buf();
        assert_eq!(
            FileOperationPlan::preflight(request(
                FileOperationKind::Move,
                vec![source.clone()],
                source_parent,
            )),
            Err(FileOperationPreflightError::SourceEqualsDestination {
                path: source.clone(),
            })
        );

        let directory_source = td.dir("tree");
        let nested_destination = td.dir("tree/nested");
        assert_eq!(
            FileOperationPlan::preflight(request(
                FileOperationKind::Copy,
                vec![directory_source.clone()],
                nested_destination.clone(),
            )),
            Err(FileOperationPreflightError::DestinationInsideSource {
                source: directory_source,
                destination_directory: nested_destination,
            })
        );
    }

    // TP-C4.1-PREFLIGHT-TOCTOU: the plan is not execution authority forever;
    // source/destination identity and collision absence are checked again at
    // the last boundary before future COPY/MOVE writes.
    #[test]
    fn file_operation_plan_revalidation_rejects_replacement_and_new_collision() {
        let td = TempDir::new("revalidation");
        let source = td.file("source.txt", b"first identity");
        let destination = td.dir("destination");
        let plan = FileOperationPlan::preflight(request(
            FileOperationKind::Copy,
            vec![source.clone()],
            destination.clone(),
        ))
        .expect("initial plan");

        fs::remove_file(&source).expect("remove planned source");
        fs::write(&source, b"replacement identity with different length")
            .expect("replace planned source");
        assert_eq!(
            plan.revalidate(),
            Err(FileOperationPreflightError::SourceChanged {
                path: source.clone(),
            })
        );

        let collision_source = td.file("second.txt", b"second");
        let collision_plan = FileOperationPlan::preflight(request(
            FileOperationKind::Move,
            vec![collision_source.clone()],
            destination.clone(),
        ))
        .expect("collision-free plan");
        let late_destination = td.file("destination/second.txt", b"late collision");
        assert_eq!(
            collision_plan.revalidate(),
            Err(FileOperationPreflightError::DestinationCollision {
                source: collision_source,
                destination: late_destination,
            })
        );

        let stable_source = td.file("stable.txt", b"stable");
        let destination_plan = FileOperationPlan::preflight(request(
            FileOperationKind::Copy,
            vec![stable_source],
            destination.clone(),
        ))
        .expect("stable destination plan");
        let moved_destination = td.root.join("destination-old");
        fs::rename(&destination, &moved_destination).expect("replace destination directory");
        fs::create_dir(&destination).expect("recreate destination path with new identity");
        assert_eq!(
            destination_plan.revalidate(),
            Err(FileOperationPreflightError::DestinationChanged { path: destination })
        );
    }
}
