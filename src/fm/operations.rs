use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
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

#[cfg(test)]
mod tests {
    use super::{
        FileOperationKind, FileOperationPlan, FileOperationPreflightError, FileOperationRequest,
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
