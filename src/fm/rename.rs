use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};
use std::time::SystemTime;

use crate::fm::operations::FileOperationCancellation;
use crate::platform::FileIdentity;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RenameOperationRequest {
    pub(crate) source_path: PathBuf,
    pub(crate) new_name: String,
    pub(crate) operation_in_flight: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RenameOperationPreflightError {
    OperationInFlight,
    InvalidNewName,
    NonUtf8Path { path: PathBuf },
    SourceMissing { path: PathBuf },
    SourceUnavailable { path: PathBuf, kind: io::ErrorKind },
    SourceUnsupported { path: PathBuf },
    SourceHasNoFileName { path: PathBuf },
    UnchangedName { path: PathBuf },
    FileIdentityUnavailable { path: PathBuf, kind: io::ErrorKind },
    DestinationCollision { path: PathBuf },
    DestinationUnavailable { path: PathBuf, kind: io::ErrorKind },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlannedRenamePathKind {
    File,
    Directory,
    Symlink,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RenameSourceSnapshot {
    identity: FileIdentity,
    path_kind: PlannedRenamePathKind,
    len: u64,
    modified: Option<SystemTime>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RenameOperationPlan {
    source: PathBuf,
    destination: PathBuf,
    snapshot: RenameSourceSnapshot,
}

impl RenameOperationPlan {
    pub(crate) fn preflight(
        request: RenameOperationRequest,
    ) -> Result<Self, RenameOperationPreflightError> {
        if request.operation_in_flight {
            return Err(RenameOperationPreflightError::OperationInFlight);
        }
        if request.source_path.to_str().is_none() {
            return Err(RenameOperationPreflightError::NonUtf8Path {
                path: request.source_path,
            });
        }
        if !is_single_file_name(&request.new_name) {
            return Err(RenameOperationPreflightError::InvalidNewName);
        }
        let Some(source_name) = request.source_path.file_name() else {
            return Err(RenameOperationPreflightError::SourceHasNoFileName {
                path: request.source_path,
            });
        };
        if source_name == request.new_name.as_str() {
            return Err(RenameOperationPreflightError::UnchangedName {
                path: request.source_path,
            });
        }
        let Some(parent) = request.source_path.parent() else {
            return Err(RenameOperationPreflightError::SourceHasNoFileName {
                path: request.source_path,
            });
        };
        let destination = parent.join(&request.new_name);
        reject_destination_collision(&destination)?;
        let snapshot = snapshot_source(&request.source_path)?;
        Ok(Self {
            source: request.source_path,
            destination,
            snapshot,
        })
    }

    pub(crate) fn source(&self) -> &Path {
        &self.source
    }

    pub(crate) fn destination(&self) -> &Path {
        &self.destination
    }

    pub(crate) fn path_kind(&self) -> PlannedRenamePathKind {
        self.snapshot.path_kind
    }
}

fn is_single_file_name(name: &str) -> bool {
    if name.is_empty() || name.contains('\0') {
        return false;
    }
    let mut components = Path::new(name).components();
    matches!(components.next(), Some(Component::Normal(_))) && components.next().is_none()
}

fn reject_destination_collision(path: &Path) -> Result<(), RenameOperationPreflightError> {
    match fs::symlink_metadata(path) {
        Ok(_) => Err(RenameOperationPreflightError::DestinationCollision {
            path: path.to_path_buf(),
        }),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(RenameOperationPreflightError::DestinationUnavailable {
            path: path.to_path_buf(),
            kind: error.kind(),
        }),
    }
}

fn snapshot_source(path: &Path) -> Result<RenameSourceSnapshot, RenameOperationPreflightError> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        if error.kind() == io::ErrorKind::NotFound {
            RenameOperationPreflightError::SourceMissing {
                path: path.to_path_buf(),
            }
        } else {
            RenameOperationPreflightError::SourceUnavailable {
                path: path.to_path_buf(),
                kind: error.kind(),
            }
        }
    })?;
    let path_kind = if metadata.file_type().is_symlink() {
        PlannedRenamePathKind::Symlink
    } else if metadata.is_file() {
        PlannedRenamePathKind::File
    } else if metadata.is_dir() {
        PlannedRenamePathKind::Directory
    } else {
        return Err(RenameOperationPreflightError::SourceUnsupported {
            path: path.to_path_buf(),
        });
    };
    let identity = crate::platform::file_identity(path, &metadata).map_err(|error| {
        RenameOperationPreflightError::FileIdentityUnavailable {
            path: path.to_path_buf(),
            kind: error.kind(),
        }
    })?;
    Ok(RenameSourceSnapshot {
        identity,
        path_kind,
        len: metadata.len(),
        modified: metadata.modified().ok(),
    })
}

fn snapshot_for_revalidation(path: &Path) -> Option<RenameSourceSnapshot> {
    let metadata = fs::symlink_metadata(path).ok()?;
    let path_kind = if metadata.file_type().is_symlink() {
        PlannedRenamePathKind::Symlink
    } else if metadata.is_file() {
        PlannedRenamePathKind::File
    } else if metadata.is_dir() {
        PlannedRenamePathKind::Directory
    } else {
        return None;
    };
    let identity = crate::platform::file_identity(path, &metadata).ok()?;
    Some(RenameSourceSnapshot {
        identity,
        path_kind,
        len: metadata.len(),
        modified: metadata.modified().ok(),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RenameOperationError {
    SourceChanged,
    DestinationCollision,
    Io(io::ErrorKind),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RenameOperationExecutionStatus {
    Completed,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RenameOperationOutcome {
    NotStarted,
    Renamed,
    Retained(RenameOperationError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RenameOperationExecutionResult {
    status: RenameOperationExecutionStatus,
    outcome: RenameOperationOutcome,
}

impl RenameOperationExecutionResult {
    pub(crate) fn status(&self) -> RenameOperationExecutionStatus {
        self.status
    }

    pub(crate) fn outcome(&self) -> &RenameOperationOutcome {
        &self.outcome
    }
}

pub(crate) trait RenameOperationHost {
    /// Test point immediately before the final source/destination read. The
    /// production host is a no-op; injected hosts use this seam to prove stale
    /// plans fail closed without widening the filesystem API surface.
    fn before_revalidation(&mut self) -> io::Result<()>;

    fn publish_no_replace(&mut self, source: &Path, destination: &Path) -> io::Result<()>;
}

struct RealRenameOperationHost;

impl RenameOperationHost for RealRenameOperationHost {
    fn before_revalidation(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn publish_no_replace(&mut self, source: &Path, destination: &Path) -> io::Result<()> {
        crate::platform::publish_staged_path_no_replace(source, destination)
    }
}

pub(crate) fn execute_rename_operation(
    plan: &RenameOperationPlan,
    cancellation: &FileOperationCancellation,
) -> RenameOperationExecutionResult {
    execute_rename_operation_with_host(plan, cancellation, &mut RealRenameOperationHost)
}

pub(crate) fn execute_rename_operation_with_host<H: RenameOperationHost>(
    plan: &RenameOperationPlan,
    cancellation: &FileOperationCancellation,
    host: &mut H,
) -> RenameOperationExecutionResult {
    if cancellation.is_cancelled() {
        return RenameOperationExecutionResult {
            status: RenameOperationExecutionStatus::Cancelled,
            outcome: RenameOperationOutcome::NotStarted,
        };
    }
    if let Err(error) = host.before_revalidation() {
        return retained_result(RenameOperationError::Io(error.kind()));
    }
    if snapshot_for_revalidation(&plan.source).as_ref() != Some(&plan.snapshot) {
        return retained_result(RenameOperationError::SourceChanged);
    }
    if fs::symlink_metadata(&plan.destination).is_ok() {
        return retained_result(RenameOperationError::DestinationCollision);
    }

    match host.publish_no_replace(&plan.source, &plan.destination) {
        Ok(()) => RenameOperationExecutionResult {
            status: RenameOperationExecutionStatus::Completed,
            outcome: RenameOperationOutcome::Renamed,
        },
        Err(error)
            if error.kind() == io::ErrorKind::AlreadyExists
                || fs::symlink_metadata(&plan.destination).is_ok() =>
        {
            retained_result(RenameOperationError::DestinationCollision)
        }
        Err(error) => retained_result(RenameOperationError::Io(error.kind())),
    }
}

fn retained_result(error: RenameOperationError) -> RenameOperationExecutionResult {
    RenameOperationExecutionResult {
        status: RenameOperationExecutionStatus::Failed,
        outcome: RenameOperationOutcome::Retained(error),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        execute_bulk_rename_operation, execute_bulk_rename_operation_with_host,
        execute_rename_operation, execute_rename_operation_with_host, BulkRenameItemOutcome,
        BulkRenameOperationExecutionStatus, BulkRenameOperationPlan,
        BulkRenameOperationPreflightError, BulkRenameOperationRequest, PlannedRenamePathKind,
        RenameOperationError, RenameOperationExecutionStatus, RenameOperationHost,
        RenameOperationOutcome, RenameOperationPlan, RenameOperationPreflightError,
        RenameOperationRequest,
    };
    use crate::fm::operations::FileOperationCancellation;
    use std::fs;
    use std::io;
    use std::path::{Path, PathBuf};
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
                "herdr-fm-rename-operation-test-{}-{}-{}",
                std::process::id(),
                tag,
                unique()
            ));
            fs::create_dir_all(&root).expect("create rename-operation test root");
            Self { root }
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn request(source_path: PathBuf, new_name: &str) -> RenameOperationRequest {
        RenameOperationRequest {
            source_path,
            new_name: new_name.to_string(),
            operation_in_flight: false,
        }
    }

    fn bulk_request(mappings: Vec<(PathBuf, &str)>) -> BulkRenameOperationRequest {
        BulkRenameOperationRequest {
            mappings: mappings
                .into_iter()
                .map(|(source_path, new_name)| (source_path, new_name.to_string()))
                .collect(),
            operation_in_flight: false,
        }
    }

    // TP-C4.3-BULK: the complete ordered mapping validates before mutation.
    // Outputs must be unique, external destinations must be absent, and source
    // count remains bounded; occupied destinations are allowed only when they
    // are exact members of the same rename graph.
    #[test]
    fn bulk_rename_preflight_validates_complete_mapping_atomically() {
        let td = TempDir::new("bulk-preflight");
        let alpha = td.root.join("alpha.txt");
        let beta = td.root.join("beta.txt");
        fs::write(&alpha, b"alpha").expect("write alpha");
        fs::write(&beta, b"beta").expect("write beta");

        let plan = BulkRenameOperationPlan::preflight(bulk_request(vec![
            (alpha.clone(), "beta.txt"),
            (beta.clone(), "gamma.txt"),
        ]))
        .expect("chain destinations may name exact sources");
        assert_eq!(
            plan.mappings(),
            &[
                (alpha.clone(), td.root.join("beta.txt")),
                (beta.clone(), td.root.join("gamma.txt")),
            ]
        );
        assert_eq!(fs::read(&alpha).expect("alpha remains"), b"alpha");
        assert_eq!(fs::read(&beta).expect("beta remains"), b"beta");

        assert_eq!(
            BulkRenameOperationPlan::preflight(bulk_request(vec![])),
            Err(BulkRenameOperationPreflightError::NoMappings)
        );
        assert_eq!(
            BulkRenameOperationPlan::preflight(BulkRenameOperationRequest {
                mappings: vec![(alpha.clone(), "renamed.txt".to_string())],
                operation_in_flight: true,
            }),
            Err(BulkRenameOperationPreflightError::OperationInFlight)
        );
        assert_eq!(
            BulkRenameOperationPlan::preflight(bulk_request(vec![
                (alpha.clone(), "same.txt"),
                (beta.clone(), "same.txt"),
            ])),
            Err(BulkRenameOperationPreflightError::DuplicateDestination {
                path: td.root.join("same.txt")
            })
        );
        assert_eq!(
            BulkRenameOperationPlan::preflight(bulk_request(vec![
                (alpha.clone(), "one.txt"),
                (alpha.clone(), "two.txt"),
            ])),
            Err(BulkRenameOperationPreflightError::DuplicateSource {
                path: alpha.clone()
            })
        );

        let occupied = td.root.join("occupied.txt");
        fs::write(&occupied, b"occupied").expect("write occupied target");
        assert_eq!(
            BulkRenameOperationPlan::preflight(bulk_request(vec![(alpha.clone(), "occupied.txt")])),
            Err(BulkRenameOperationPreflightError::DestinationCollision {
                path: occupied.clone()
            })
        );
        assert_eq!(fs::read(alpha).expect("source retained"), b"alpha");
        assert_eq!(fs::read(occupied).expect("occupied retained"), b"occupied");
    }

    // TP-C4.3-BULK: staging every source before publishing outputs must make
    // chains, swaps, and cycles independent from input order while preserving
    // the payload associated with each original source.
    #[test]
    fn bulk_rename_executes_chains_swaps_and_cycles_without_corruption() {
        let td = TempDir::new("bulk-cycles");
        let alpha = td.root.join("alpha.txt");
        let beta = td.root.join("beta.txt");
        let gamma = td.root.join("gamma.txt");
        fs::write(&alpha, b"alpha").expect("write cycle alpha");
        fs::write(&beta, b"beta").expect("write cycle beta");
        fs::write(&gamma, b"gamma").expect("write cycle gamma");

        let plan = BulkRenameOperationPlan::preflight(bulk_request(vec![
            (gamma.clone(), "alpha.txt"),
            (alpha.clone(), "beta.txt"),
            (beta.clone(), "gamma.txt"),
        ]))
        .expect("cycle plan");
        let staging_paths = plan.staging_paths().to_vec();
        let result = execute_bulk_rename_operation(&plan, &FileOperationCancellation::default());

        assert_eq!(
            result.status(),
            BulkRenameOperationExecutionStatus::Completed
        );
        assert!(result
            .items()
            .iter()
            .all(|item| item.outcome() == &BulkRenameItemOutcome::Renamed));
        assert_eq!(fs::read(&alpha).expect("cycle alpha output"), b"gamma");
        assert_eq!(fs::read(&beta).expect("cycle beta output"), b"alpha");
        assert_eq!(fs::read(&gamma).expect("cycle gamma output"), b"beta");
        assert!(staging_paths.iter().all(|path| !path.exists()));

        let swap = BulkRenameOperationPlan::preflight(bulk_request(vec![
            (alpha.clone(), "beta.txt"),
            (beta.clone(), "alpha.txt"),
        ]))
        .expect("swap plan");
        let result = execute_bulk_rename_operation(&swap, &FileOperationCancellation::default());
        assert_eq!(
            result.status(),
            BulkRenameOperationExecutionStatus::Completed
        );
        assert_eq!(fs::read(alpha).expect("swap alpha output"), b"alpha");
        assert_eq!(fs::read(beta).expect("swap beta output"), b"gamma");
    }

    struct OneShotFailingRenameHost {
        fail_on_call: usize,
        calls: usize,
    }

    impl RenameOperationHost for OneShotFailingRenameHost {
        fn before_revalidation(&mut self) -> io::Result<()> {
            Ok(())
        }

        fn publish_no_replace(&mut self, source: &Path, destination: &Path) -> io::Result<()> {
            self.calls += 1;
            if self.calls == self.fail_on_call {
                Err(io::Error::new(
                    io::ErrorKind::Other,
                    "injected one-shot rename failure",
                ))
            } else {
                crate::platform::publish_staged_path_no_replace(source, destination)
            }
        }
    }

    // TP-C4.3-BULK: a staging failure rolls every already-staged source back to
    // its exact original name, reports a rolled-back terminal state, and leaves
    // neither requested outputs nor private artifacts behind.
    #[test]
    fn bulk_rename_staging_failure_restores_all_sources() {
        let td = TempDir::new("bulk-stage-failure");
        let alpha = td.root.join("alpha.txt");
        let beta = td.root.join("beta.txt");
        fs::write(&alpha, b"alpha").expect("write stage alpha");
        fs::write(&beta, b"beta").expect("write stage beta");
        let plan = BulkRenameOperationPlan::preflight(bulk_request(vec![
            (alpha.clone(), "renamed-alpha.txt"),
            (beta.clone(), "renamed-beta.txt"),
        ]))
        .expect("stage failure plan");
        let staging_paths = plan.staging_paths().to_vec();
        let result = execute_bulk_rename_operation_with_host(
            &plan,
            &FileOperationCancellation::default(),
            &mut OneShotFailingRenameHost {
                fail_on_call: 2,
                calls: 0,
            },
        );

        assert_eq!(
            result.status(),
            BulkRenameOperationExecutionStatus::RolledBack
        );
        assert_eq!(fs::read(&alpha).expect("restored stage alpha"), b"alpha");
        assert_eq!(fs::read(&beta).expect("retained stage beta"), b"beta");
        assert!(!td.root.join("renamed-alpha.txt").exists());
        assert!(!td.root.join("renamed-beta.txt").exists());
        assert!(staging_paths.iter().all(|path| !path.exists()));
        assert!(result
            .items()
            .iter()
            .any(|item| matches!(item.outcome(), BulkRenameItemOutcome::Restored(_))));
        assert!(result
            .items()
            .iter()
            .any(|item| matches!(item.outcome(), BulkRenameItemOutcome::Retained(_))));
    }

    // TP-C4.3-BULK: a publish failure after an earlier output committed first
    // reverses committed outputs back to private staging, then restores every
    // source. Recovery is explicit and no partial success is reported.
    #[test]
    fn bulk_rename_publish_failure_rolls_back_committed_outputs() {
        let td = TempDir::new("bulk-publish-failure");
        let alpha = td.root.join("alpha.txt");
        let beta = td.root.join("beta.txt");
        fs::write(&alpha, b"alpha").expect("write publish alpha");
        fs::write(&beta, b"beta").expect("write publish beta");
        let plan = BulkRenameOperationPlan::preflight(bulk_request(vec![
            (alpha.clone(), "renamed-alpha.txt"),
            (beta.clone(), "renamed-beta.txt"),
        ]))
        .expect("publish failure plan");
        let staging_paths = plan.staging_paths().to_vec();
        let result = execute_bulk_rename_operation_with_host(
            &plan,
            &FileOperationCancellation::default(),
            &mut OneShotFailingRenameHost {
                // Calls 1-2 stage both sources, 3 publishes alpha, 4 fails beta.
                fail_on_call: 4,
                calls: 0,
            },
        );

        assert_eq!(
            result.status(),
            BulkRenameOperationExecutionStatus::RolledBack
        );
        assert_eq!(fs::read(&alpha).expect("restored publish alpha"), b"alpha");
        assert_eq!(fs::read(&beta).expect("restored publish beta"), b"beta");
        assert!(!td.root.join("renamed-alpha.txt").exists());
        assert!(!td.root.join("renamed-beta.txt").exists());
        assert!(staging_paths.iter().all(|path| !path.exists()));
        assert!(result
            .items()
            .iter()
            .all(|item| matches!(item.outcome(), BulkRenameItemOutcome::Restored(_))));
    }

    // TP-C4.3-COLLISION: preflight snapshots the exact source object and one
    // same-parent destination while rejecting stale, unsupported, root,
    // unchanged, malformed, in-flight, and already-colliding intents before
    // any filesystem mutation.
    #[test]
    fn rename_preflight_is_single_source_same_parent_and_fail_closed() {
        let td = TempDir::new("preflight");
        let source = td.root.join("source.txt");
        fs::write(&source, b"source").expect("write preflight source");

        let plan = RenameOperationPlan::preflight(request(source.clone(), "renamed.txt"))
            .expect("valid rename preflight");
        assert_eq!(plan.source(), source);
        assert_eq!(plan.destination(), td.root.join("renamed.txt"));
        assert_eq!(plan.path_kind(), PlannedRenamePathKind::File);
        assert_eq!(
            fs::read(&source).expect("preflight source remains"),
            b"source"
        );

        let missing = td.root.join("missing.txt");
        assert_eq!(
            RenameOperationPlan::preflight(request(missing.clone(), "other.txt")),
            Err(RenameOperationPreflightError::SourceMissing { path: missing })
        );
        assert_eq!(
            RenameOperationPlan::preflight(RenameOperationRequest {
                source_path: source.clone(),
                new_name: "other.txt".to_string(),
                operation_in_flight: true,
            }),
            Err(RenameOperationPreflightError::OperationInFlight)
        );
        assert_eq!(
            RenameOperationPlan::preflight(request(source.clone(), "source.txt")),
            Err(RenameOperationPreflightError::UnchangedName {
                path: source.clone()
            })
        );
        for invalid_name in ["", ".", "..", "nested/name"] {
            assert_eq!(
                RenameOperationPlan::preflight(request(source.clone(), invalid_name)),
                Err(RenameOperationPreflightError::InvalidNewName)
            );
        }
        assert_eq!(
            RenameOperationPlan::preflight(request(PathBuf::from("/"), "root")),
            Err(RenameOperationPreflightError::SourceHasNoFileName {
                path: PathBuf::from("/")
            })
        );

        let collision = td.root.join("occupied.txt");
        fs::write(&collision, b"occupied").expect("write collision target");
        assert_eq!(
            RenameOperationPlan::preflight(request(source.clone(), "occupied.txt")),
            Err(RenameOperationPreflightError::DestinationCollision {
                path: collision.clone()
            })
        );
        assert_eq!(fs::read(source).expect("source retained"), b"source");
        assert_eq!(
            fs::read(collision).expect("collision retained"),
            b"occupied"
        );
    }

    // TP-C4.3-ATOMIC: the final boundary revalidates both source identity and
    // destination absence. A replacement or newly occupied target must retain
    // every unrelated object and must not call the commit primitive.
    #[test]
    fn rename_execution_revalidates_source_and_destination_before_commit() {
        struct BeforeCommitHost {
            replacement: Option<(PathBuf, Vec<u8>)>,
            collision: Option<(PathBuf, Vec<u8>)>,
            publishes: usize,
        }

        impl RenameOperationHost for BeforeCommitHost {
            fn before_revalidation(&mut self) -> io::Result<()> {
                if let Some((path, contents)) = self.replacement.take() {
                    fs::remove_file(&path)?;
                    fs::write(path, contents)?;
                }
                if let Some((path, contents)) = self.collision.take() {
                    fs::write(path, contents)?;
                }
                Ok(())
            }

            fn publish_no_replace(
                &mut self,
                _source: &Path,
                _destination: &Path,
            ) -> io::Result<()> {
                self.publishes += 1;
                Ok(())
            }
        }

        let td = TempDir::new("revalidation");
        let source = td.root.join("source.txt");
        let destination = td.root.join("renamed.txt");
        fs::write(&source, b"original").expect("write original source");
        let plan = RenameOperationPlan::preflight(request(source.clone(), "renamed.txt"))
            .expect("source replacement plan");
        let mut host = BeforeCommitHost {
            replacement: Some((source.clone(), b"replacement".to_vec())),
            collision: None,
            publishes: 0,
        };
        let result = execute_rename_operation_with_host(
            &plan,
            &FileOperationCancellation::default(),
            &mut host,
        );
        assert_eq!(result.status(), RenameOperationExecutionStatus::Failed);
        assert_eq!(
            result.outcome(),
            &RenameOperationOutcome::Retained(RenameOperationError::SourceChanged)
        );
        assert_eq!(host.publishes, 0);
        assert_eq!(
            fs::read(&source).expect("replacement retained"),
            b"replacement"
        );
        assert!(!destination.exists());

        let plan = RenameOperationPlan::preflight(request(source.clone(), "renamed.txt"))
            .expect("destination collision plan");
        let mut host = BeforeCommitHost {
            replacement: None,
            collision: Some((destination.clone(), b"racer".to_vec())),
            publishes: 0,
        };
        let result = execute_rename_operation_with_host(
            &plan,
            &FileOperationCancellation::default(),
            &mut host,
        );
        assert_eq!(
            result.outcome(),
            &RenameOperationOutcome::Retained(RenameOperationError::DestinationCollision)
        );
        assert_eq!(host.publishes, 0);
        assert_eq!(fs::read(&source).expect("source retained"), b"replacement");
        assert_eq!(fs::read(&destination).expect("racer retained"), b"racer");
    }

    // TP-C4.3-ATOMIC: even when a destination appears after the explicit
    // absence check, the platform no-replace primitive is the commit authority
    // and must preserve both source and racer contents.
    #[test]
    fn rename_publish_race_never_replaces_destination() {
        struct TargetRaceHost;

        impl RenameOperationHost for TargetRaceHost {
            fn before_revalidation(&mut self) -> io::Result<()> {
                Ok(())
            }

            fn publish_no_replace(&mut self, source: &Path, destination: &Path) -> io::Result<()> {
                fs::write(destination, b"racer")?;
                crate::platform::publish_staged_path_no_replace(source, destination)
            }
        }

        let td = TempDir::new("publish-race");
        let source = td.root.join("source.txt");
        let destination = td.root.join("renamed.txt");
        fs::write(&source, b"source").expect("write race source");
        let plan = RenameOperationPlan::preflight(request(source.clone(), "renamed.txt"))
            .expect("race plan");
        let result = execute_rename_operation_with_host(
            &plan,
            &FileOperationCancellation::default(),
            &mut TargetRaceHost,
        );

        assert_eq!(result.status(), RenameOperationExecutionStatus::Failed);
        assert_eq!(
            result.outcome(),
            &RenameOperationOutcome::Retained(RenameOperationError::DestinationCollision)
        );
        assert_eq!(fs::read(source).expect("race source retained"), b"source");
        assert_eq!(
            fs::read(destination).expect("race destination retained"),
            b"racer"
        );
    }

    // TP-C4.3-ATOMIC: real file, directory, and symlink renames publish the
    // exact requested path without changing file bytes, directory children, or
    // symlink targets.
    #[test]
    fn rename_real_filesystem_preserves_file_directory_and_symlink_payloads() {
        let td = TempDir::new("real-filesystem");

        let file = td.root.join("file.txt");
        fs::write(&file, b"file payload").expect("write file payload");
        let file_plan = RenameOperationPlan::preflight(request(file.clone(), "renamed.txt"))
            .expect("file plan");
        assert_eq!(file_plan.path_kind(), PlannedRenamePathKind::File);
        let result = execute_rename_operation(&file_plan, &FileOperationCancellation::default());
        assert_eq!(result.status(), RenameOperationExecutionStatus::Completed);
        assert_eq!(result.outcome(), &RenameOperationOutcome::Renamed);
        assert!(!file.exists());
        assert_eq!(
            fs::read(td.root.join("renamed.txt")).expect("read renamed file"),
            b"file payload"
        );

        let directory = td.root.join("directory");
        fs::create_dir(&directory).expect("create directory source");
        fs::write(directory.join("child.txt"), b"child").expect("write child");
        let directory_plan =
            RenameOperationPlan::preflight(request(directory.clone(), "renamed-directory"))
                .expect("directory plan");
        assert_eq!(directory_plan.path_kind(), PlannedRenamePathKind::Directory);
        let result =
            execute_rename_operation(&directory_plan, &FileOperationCancellation::default());
        assert_eq!(result.outcome(), &RenameOperationOutcome::Renamed);
        assert!(!directory.exists());
        assert_eq!(
            fs::read(td.root.join("renamed-directory/child.txt")).expect("read renamed child"),
            b"child"
        );

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;

            let target = td.root.join("target.txt");
            let link = td.root.join("link.txt");
            fs::write(&target, b"target").expect("write symlink target");
            symlink(&target, &link).expect("create symlink source");
            let link_plan = RenameOperationPlan::preflight(request(link.clone(), "renamed-link"))
                .expect("symlink plan");
            assert_eq!(link_plan.path_kind(), PlannedRenamePathKind::Symlink);
            let result =
                execute_rename_operation(&link_plan, &FileOperationCancellation::default());
            assert_eq!(result.outcome(), &RenameOperationOutcome::Renamed);
            assert!(fs::symlink_metadata(&link).is_err());
            assert_eq!(
                fs::read_link(td.root.join("renamed-link")).expect("read renamed symlink"),
                target
            );
        }
    }
}
