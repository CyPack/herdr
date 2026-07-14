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
