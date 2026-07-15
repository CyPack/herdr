#[cfg(test)]
mod tests {
    use super::{
        execute_rename_operation, execute_rename_operation_with_host, PlannedRenamePathKind,
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
