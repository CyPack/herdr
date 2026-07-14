use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::fm::operations::FileOperationCancellation;
use crate::fm::MAX_MULTI_SELECTION_PATHS;
use crate::platform::FileIdentity;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DeleteOperationKind {
    Trash,
    Permanent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DeleteOperationRequest {
    pub(crate) kind: DeleteOperationKind,
    pub(crate) paths: Vec<PathBuf>,
    pub(crate) operation_in_flight: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DeleteOperationPreflightError {
    NoPaths,
    TooManyPaths { count: usize, limit: usize },
    OperationInFlight,
    DuplicatePath { path: PathBuf },
    NonUtf8Path { path: PathBuf },
    SourceMissing { path: PathBuf },
    SourceUnavailable { path: PathBuf, kind: io::ErrorKind },
    SourceUnsupported { path: PathBuf },
    FileIdentityUnavailable { path: PathBuf, kind: io::ErrorKind },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlannedDeletePathKind {
    File,
    Directory,
    Symlink,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DeleteSourceSnapshot {
    identity: FileIdentity,
    path_kind: PlannedDeletePathKind,
    len: u64,
    modified: Option<SystemTime>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PlannedDeleteItem {
    path: PathBuf,
    snapshot: DeleteSourceSnapshot,
}

impl PlannedDeleteItem {
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    #[cfg(test)]
    pub(crate) fn path_kind(&self) -> PlannedDeletePathKind {
        self.snapshot.path_kind
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DeleteOperationPlan {
    kind: DeleteOperationKind,
    items: Vec<PlannedDeleteItem>,
}

impl DeleteOperationPlan {
    pub(crate) fn preflight(
        request: DeleteOperationRequest,
    ) -> Result<Self, DeleteOperationPreflightError> {
        if request.operation_in_flight {
            return Err(DeleteOperationPreflightError::OperationInFlight);
        }
        if request.paths.is_empty() {
            return Err(DeleteOperationPreflightError::NoPaths);
        }
        if request.paths.len() > MAX_MULTI_SELECTION_PATHS {
            return Err(DeleteOperationPreflightError::TooManyPaths {
                count: request.paths.len(),
                limit: MAX_MULTI_SELECTION_PATHS,
            });
        }

        let mut exact_paths = HashSet::with_capacity(request.paths.len());
        let mut items = Vec::with_capacity(request.paths.len());
        for path in request.paths {
            if path.to_str().is_none() {
                return Err(DeleteOperationPreflightError::NonUtf8Path { path });
            }
            if !exact_paths.insert(path.clone()) {
                return Err(DeleteOperationPreflightError::DuplicatePath { path });
            }
            items.push(PlannedDeleteItem {
                snapshot: snapshot_delete_source(&path)?,
                path,
            });
        }
        Ok(Self {
            kind: request.kind,
            items,
        })
    }

    #[cfg(test)]
    pub(crate) fn kind(&self) -> DeleteOperationKind {
        self.kind
    }

    pub(crate) fn items(&self) -> &[PlannedDeleteItem] {
        &self.items
    }
}

fn snapshot_delete_source(
    path: &Path,
) -> Result<DeleteSourceSnapshot, DeleteOperationPreflightError> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        if error.kind() == io::ErrorKind::NotFound {
            DeleteOperationPreflightError::SourceMissing {
                path: path.to_path_buf(),
            }
        } else {
            DeleteOperationPreflightError::SourceUnavailable {
                path: path.to_path_buf(),
                kind: error.kind(),
            }
        }
    })?;
    let path_kind = if metadata.file_type().is_symlink() {
        PlannedDeletePathKind::Symlink
    } else if metadata.is_file() {
        PlannedDeletePathKind::File
    } else if metadata.is_dir() {
        PlannedDeletePathKind::Directory
    } else {
        return Err(DeleteOperationPreflightError::SourceUnsupported {
            path: path.to_path_buf(),
        });
    };
    let identity = crate::platform::file_identity(path, &metadata).map_err(|error| {
        DeleteOperationPreflightError::FileIdentityUnavailable {
            path: path.to_path_buf(),
            kind: error.kind(),
        }
    })?;
    Ok(DeleteSourceSnapshot {
        identity,
        path_kind,
        len: metadata.len(),
        modified: metadata.modified().ok(),
    })
}

fn snapshot_for_revalidation(path: &Path) -> Option<DeleteSourceSnapshot> {
    let metadata = fs::symlink_metadata(path).ok()?;
    let path_kind = if metadata.file_type().is_symlink() {
        PlannedDeletePathKind::Symlink
    } else if metadata.is_file() {
        PlannedDeletePathKind::File
    } else if metadata.is_dir() {
        PlannedDeletePathKind::Directory
    } else {
        return None;
    };
    let identity = crate::platform::file_identity(path, &metadata).ok()?;
    Some(DeleteSourceSnapshot {
        identity,
        path_kind,
        len: metadata.len(),
        modified: metadata.modified().ok(),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DeleteBackendError {
    SourceChanged,
    Io(io::ErrorKind),
    Trash(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DeleteOperationExecutionStatus {
    Completed,
    Cancelled,
    Partial,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DeleteOperationItemOutcome {
    NotStarted,
    Deleted,
    Retained(DeleteBackendError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DeleteOperationItemResult {
    path: PathBuf,
    outcome: DeleteOperationItemOutcome,
}

impl DeleteOperationItemResult {
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn outcome(&self) -> &DeleteOperationItemOutcome {
        &self.outcome
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DeleteOperationExecutionResult {
    status: DeleteOperationExecutionStatus,
    items: Vec<DeleteOperationItemResult>,
}

impl DeleteOperationExecutionResult {
    pub(crate) fn status(&self) -> DeleteOperationExecutionStatus {
        self.status
    }

    pub(crate) fn items(&self) -> &[DeleteOperationItemResult] {
        &self.items
    }
}

pub(crate) trait DeleteOperationHost {
    fn delete_path(
        &mut self,
        operation: DeleteOperationKind,
        path_kind: PlannedDeletePathKind,
        path: &Path,
    ) -> Result<(), DeleteBackendError>;
}

struct RealDeleteOperationHost;

impl DeleteOperationHost for RealDeleteOperationHost {
    fn delete_path(
        &mut self,
        operation: DeleteOperationKind,
        path_kind: PlannedDeletePathKind,
        path: &Path,
    ) -> Result<(), DeleteBackendError> {
        match operation {
            // Intentionally one call per item: the App runs this host on its
            // single operation lane and needs exact partial-failure evidence.
            DeleteOperationKind::Trash => trash::delete(path).map_err(map_trash_error),
            DeleteOperationKind::Permanent => match path_kind {
                PlannedDeletePathKind::File | PlannedDeletePathKind::Symlink => {
                    fs::remove_file(path)
                }
                PlannedDeletePathKind::Directory => fs::remove_dir_all(path),
            }
            .map_err(|error| DeleteBackendError::Io(error.kind())),
        }
    }
}

fn map_trash_error(error: trash::Error) -> DeleteBackendError {
    match error {
        trash::Error::Os { code, .. } => {
            DeleteBackendError::Io(io::Error::from_raw_os_error(code).kind())
        }
        #[cfg(all(
            unix,
            not(target_os = "macos"),
            not(target_os = "ios"),
            not(target_os = "android")
        ))]
        trash::Error::FileSystem { source, .. } => DeleteBackendError::Io(source.kind()),
        other => DeleteBackendError::Trash(other.to_string()),
    }
}

pub(crate) fn execute_delete_operation(
    plan: &DeleteOperationPlan,
    cancellation: &FileOperationCancellation,
) -> DeleteOperationExecutionResult {
    let mut host = RealDeleteOperationHost;
    execute_delete_operation_with_host(plan, cancellation, &mut host)
}

pub(crate) fn execute_delete_operation_with_host<H: DeleteOperationHost>(
    plan: &DeleteOperationPlan,
    cancellation: &FileOperationCancellation,
    host: &mut H,
) -> DeleteOperationExecutionResult {
    let mut result = DeleteOperationExecutionResult {
        status: DeleteOperationExecutionStatus::Failed,
        items: plan
            .items
            .iter()
            .map(|item| DeleteOperationItemResult {
                path: item.path.clone(),
                outcome: DeleteOperationItemOutcome::NotStarted,
            })
            .collect(),
    };

    for (index, item) in plan.items.iter().enumerate() {
        if cancellation.is_cancelled() {
            result.status = DeleteOperationExecutionStatus::Cancelled;
            return result;
        }
        if snapshot_for_revalidation(&item.path).as_ref() != Some(&item.snapshot) {
            result.items[index].outcome =
                DeleteOperationItemOutcome::Retained(DeleteBackendError::SourceChanged);
            continue;
        }
        result.items[index].outcome =
            match host.delete_path(plan.kind, item.snapshot.path_kind, &item.path) {
                Ok(()) => DeleteOperationItemOutcome::Deleted,
                Err(error) => DeleteOperationItemOutcome::Retained(error),
            };
    }

    let deleted = result
        .items
        .iter()
        .filter(|item| matches!(item.outcome, DeleteOperationItemOutcome::Deleted))
        .count();
    let retained = result
        .items
        .iter()
        .filter(|item| matches!(item.outcome, DeleteOperationItemOutcome::Retained(_)))
        .count();
    result.status = match (deleted, retained) {
        (_, 0) => DeleteOperationExecutionStatus::Completed,
        (0, _) => DeleteOperationExecutionStatus::Failed,
        _ => DeleteOperationExecutionStatus::Partial,
    };
    result
}

#[cfg(test)]
mod tests {
    use super::{
        execute_delete_operation_with_host, DeleteBackendError, DeleteOperationExecutionStatus,
        DeleteOperationHost, DeleteOperationItemOutcome, DeleteOperationKind, DeleteOperationPlan,
        DeleteOperationPreflightError, DeleteOperationRequest, PlannedDeletePathKind,
    };
    use crate::fm::operations::FileOperationCancellation;
    use crate::fm::MAX_MULTI_SELECTION_PATHS;
    use std::fs;
    use std::io;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    struct TempDir {
        root: PathBuf,
    }

    impl TempDir {
        fn new(tag: &str) -> Self {
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            let root = std::env::temp_dir().join(format!(
                "herdr-fm-delete-core-test-{}-{tag}-{}",
                std::process::id(),
                COUNTER.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&root).expect("create delete fixture root");
            Self { root }
        }

        fn file(&self, name: &str, content: &[u8]) -> PathBuf {
            let path = self.root.join(name);
            fs::write(&path, content).expect("write delete fixture");
            path
        }

        fn directory(&self, name: &str) -> PathBuf {
            let path = self.root.join(name);
            fs::create_dir(&path).expect("create delete fixture directory");
            path
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[derive(Default)]
    struct RecordingHost {
        calls: Vec<(DeleteOperationKind, PlannedDeletePathKind, PathBuf)>,
        fail_path: Option<PathBuf>,
        cancel_after_success: Option<FileOperationCancellation>,
    }

    impl DeleteOperationHost for RecordingHost {
        fn delete_path(
            &mut self,
            operation: DeleteOperationKind,
            path_kind: PlannedDeletePathKind,
            path: &Path,
        ) -> Result<(), DeleteBackendError> {
            self.calls.push((operation, path_kind, path.to_path_buf()));
            if self.fail_path.as_deref() == Some(path) {
                return Err(DeleteBackendError::Io(io::ErrorKind::PermissionDenied));
            }
            match path_kind {
                PlannedDeletePathKind::File | PlannedDeletePathKind::Symlink => {
                    fs::remove_file(path)
                }
                PlannedDeletePathKind::Directory => fs::remove_dir_all(path),
            }
            .map_err(|error| DeleteBackendError::Io(error.kind()))?;
            if let Some(cancellation) = &self.cancel_after_success {
                cancellation.cancel();
            }
            Ok(())
        }
    }

    fn request(
        kind: DeleteOperationKind,
        paths: Vec<PathBuf>,
        operation_in_flight: bool,
    ) -> DeleteOperationRequest {
        DeleteOperationRequest {
            kind,
            paths,
            operation_in_flight,
        }
    }

    // TP-C4.2-TRASH: one immutable plan preserves current path order and
    // captures file, directory, and symlink identities without following links.
    #[cfg(unix)]
    #[test]
    fn delete_preflight_snapshots_ordered_exact_path_kinds() {
        use std::os::unix::fs::symlink;

        let td = TempDir::new("preflight-order");
        let file = td.file("file.txt", b"file");
        let directory = td.directory("directory");
        let target = td.file("target.txt", b"target");
        let link = td.root.join("link.txt");
        symlink(&target, &link).expect("create delete fixture symlink");

        let plan = DeleteOperationPlan::preflight(request(
            DeleteOperationKind::Trash,
            vec![file.clone(), directory.clone(), link.clone()],
            false,
        ))
        .expect("valid delete plan");

        assert_eq!(plan.kind(), DeleteOperationKind::Trash);
        assert_eq!(
            plan.items()
                .iter()
                .map(|item| (item.path().to_path_buf(), item.path_kind()))
                .collect::<Vec<_>>(),
            vec![
                (file, PlannedDeletePathKind::File),
                (directory, PlannedDeletePathKind::Directory),
                (link, PlannedDeletePathKind::Symlink),
            ]
        );
        assert_eq!(
            fs::read(target).expect("symlink target preserved"),
            b"target"
        );
    }

    // TP-C4.2-TRASH: invalid bounds and missing identities fail before the
    // host can observe any destructive request.
    #[test]
    fn delete_preflight_rejects_empty_duplicate_missing_too_many_and_inflight() {
        let td = TempDir::new("preflight-reject");
        let source = td.file("source.txt", b"source");

        assert_eq!(
            DeleteOperationPlan::preflight(request(DeleteOperationKind::Trash, vec![], false)),
            Err(DeleteOperationPreflightError::NoPaths)
        );
        assert_eq!(
            DeleteOperationPlan::preflight(request(
                DeleteOperationKind::Trash,
                vec![source.clone(), source.clone()],
                false,
            )),
            Err(DeleteOperationPreflightError::DuplicatePath {
                path: source.clone()
            })
        );
        let missing = td.root.join("missing.txt");
        assert_eq!(
            DeleteOperationPlan::preflight(request(
                DeleteOperationKind::Trash,
                vec![missing.clone()],
                false,
            )),
            Err(DeleteOperationPreflightError::SourceMissing { path: missing })
        );
        assert_eq!(
            DeleteOperationPlan::preflight(request(
                DeleteOperationKind::Trash,
                vec![source.clone(); MAX_MULTI_SELECTION_PATHS + 1],
                false,
            )),
            Err(DeleteOperationPreflightError::TooManyPaths {
                count: MAX_MULTI_SELECTION_PATHS + 1,
                limit: MAX_MULTI_SELECTION_PATHS,
            })
        );
        assert_eq!(
            DeleteOperationPlan::preflight(
                request(DeleteOperationKind::Trash, vec![source], true,)
            ),
            Err(DeleteOperationPreflightError::OperationInFlight)
        );
    }

    // TP-C4.2-TRASH: backend failure is per-item evidence; later exact paths
    // still run and the failed source remains recoverable.
    #[test]
    fn trash_execution_reports_partial_failure_per_item() {
        let td = TempDir::new("trash-partial");
        let first = td.file("first.txt", b"first");
        let retained = td.file("retained.txt", b"retained");
        let last = td.file("last.txt", b"last");
        let plan = DeleteOperationPlan::preflight(request(
            DeleteOperationKind::Trash,
            vec![first.clone(), retained.clone(), last.clone()],
            false,
        ))
        .expect("valid trash plan");
        let mut host = RecordingHost {
            fail_path: Some(retained.clone()),
            ..RecordingHost::default()
        };

        let result = execute_delete_operation_with_host(
            &plan,
            &FileOperationCancellation::default(),
            &mut host,
        );

        assert_eq!(result.status(), DeleteOperationExecutionStatus::Partial);
        assert!(matches!(
            result.items()[0].outcome(),
            DeleteOperationItemOutcome::Deleted
        ));
        assert!(matches!(
            result.items()[1].outcome(),
            DeleteOperationItemOutcome::Retained(DeleteBackendError::Io(
                io::ErrorKind::PermissionDenied
            ))
        ));
        assert!(matches!(
            result.items()[2].outcome(),
            DeleteOperationItemOutcome::Deleted
        ));
        assert!(!first.exists());
        assert_eq!(
            fs::read(retained).expect("failed trash source retained"),
            b"retained"
        );
        assert!(!last.exists());
    }

    // TP-C4.2-DELETE: replacement after confirmation is not deleted; captured
    // identity must still match immediately before the host call.
    #[test]
    fn delete_execution_rejects_replaced_path_before_mutation() {
        let td = TempDir::new("replacement");
        let source = td.file("source.txt", b"old");
        let plan = DeleteOperationPlan::preflight(request(
            DeleteOperationKind::Permanent,
            vec![source.clone()],
            false,
        ))
        .expect("valid permanent plan");
        fs::remove_file(&source).expect("remove captured source");
        fs::write(&source, b"replacement with different identity").expect("replace source");
        let mut host = RecordingHost::default();

        let result = execute_delete_operation_with_host(
            &plan,
            &FileOperationCancellation::default(),
            &mut host,
        );

        assert_eq!(result.status(), DeleteOperationExecutionStatus::Failed);
        assert!(matches!(
            result.items()[0].outcome(),
            DeleteOperationItemOutcome::Retained(DeleteBackendError::SourceChanged)
        ));
        assert!(host.calls.is_empty());
        assert_eq!(
            fs::read(source).expect("replacement retained"),
            b"replacement with different identity"
        );
    }

    // TP-C4.2-DELETE: permanent delete chooses file/directory primitives and
    // removes a symlink as a link while preserving its target.
    #[cfg(unix)]
    #[test]
    fn permanent_delete_removes_files_trees_and_symlinks_without_following() {
        use std::os::unix::fs::symlink;

        let td = TempDir::new("permanent-kinds");
        let file = td.file("file.txt", b"file");
        let directory = td.directory("tree");
        fs::write(directory.join("nested.txt"), b"nested").expect("write nested fixture");
        let target = td.file("target.txt", b"target");
        let link = td.root.join("link.txt");
        symlink(&target, &link).expect("create delete fixture symlink");
        let plan = DeleteOperationPlan::preflight(request(
            DeleteOperationKind::Permanent,
            vec![file.clone(), directory.clone(), link.clone()],
            false,
        ))
        .expect("valid permanent plan");
        let mut host = RecordingHost::default();

        let result = execute_delete_operation_with_host(
            &plan,
            &FileOperationCancellation::default(),
            &mut host,
        );

        assert_eq!(result.status(), DeleteOperationExecutionStatus::Completed);
        assert_eq!(
            host.calls,
            vec![
                (
                    DeleteOperationKind::Permanent,
                    PlannedDeletePathKind::File,
                    file.clone()
                ),
                (
                    DeleteOperationKind::Permanent,
                    PlannedDeletePathKind::Directory,
                    directory.clone(),
                ),
                (
                    DeleteOperationKind::Permanent,
                    PlannedDeletePathKind::Symlink,
                    link.clone(),
                ),
            ]
        );
        assert!(!file.exists());
        assert!(!directory.exists());
        assert!(!link.exists());
        assert_eq!(fs::read(target).expect("link target preserved"), b"target");
    }

    // TP-C4.2-RECOVERY: cancellation after one irreversible terminal result
    // preserves it and marks every untouched item as NotStarted.
    #[test]
    fn delete_cancellation_preserves_completed_and_not_started_evidence() {
        let td = TempDir::new("cancel-boundary");
        let first = td.file("first.txt", b"first");
        let second = td.file("second.txt", b"second");
        let plan = DeleteOperationPlan::preflight(request(
            DeleteOperationKind::Permanent,
            vec![first.clone(), second.clone()],
            false,
        ))
        .expect("valid permanent plan");
        let cancellation = FileOperationCancellation::default();
        let mut host = RecordingHost {
            cancel_after_success: Some(cancellation.clone()),
            ..RecordingHost::default()
        };

        let result = execute_delete_operation_with_host(&plan, &cancellation, &mut host);

        assert_eq!(result.status(), DeleteOperationExecutionStatus::Cancelled);
        assert!(matches!(
            result.items()[0].outcome(),
            DeleteOperationItemOutcome::Deleted
        ));
        assert!(matches!(
            result.items()[1].outcome(),
            DeleteOperationItemOutcome::NotStarted
        ));
        assert!(!first.exists());
        assert_eq!(
            fs::read(second).expect("cancelled source retained"),
            b"second"
        );
    }
}
