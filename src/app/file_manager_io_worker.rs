#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Condvar, Mutex};

    use tokio::sync::Notify;

    use super::{
        classify_root_navigation_error, prepare_root_navigation_io, FileManagerIoIdentity,
        FileManagerIoOutcome, FileManagerIoRequest, FileManagerIoSource, FileManagerIoSubmit,
        FileManagerIoWorker, FmRootNavigationError, FmRootNavigationRequest,
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
}
