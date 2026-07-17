//! Canonical semantic classification for native file-manager entries
//! (TP-FIP-ICON-01..05). Prepared once during directory snapshot; render and
//! capability checks derive from this single source of truth.

/// Canonical filesystem kind of one visible entry. Client-local presentation
/// state: never persisted and never added to the server protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileEntryKind {
    Directory,
    RegularFile,
    SymlinkDirectory,
    SymlinkFile,
    BrokenSymlink,
    UnsupportedSpecial,
}

impl FileEntryKind {
    pub fn is_directory_target(self) -> bool {
        matches!(self, Self::Directory | Self::SymlinkDirectory)
    }

    pub fn supports_native_operation(self) -> bool {
        !matches!(self, Self::BrokenSymlink | Self::UnsupportedSpecial)
    }

    pub fn supports_agent_reference(self) -> bool {
        self.supports_native_operation()
    }
}

/// Classify one directory entry from symlink-aware metadata. A symlink keeps
/// its link identity and resolves its target kind; a broken link and every
/// special (FIFO/socket/device) or unprovable target fail closed.
pub(crate) fn classify_dir_entry(entry: &std::fs::DirEntry) -> FileEntryKind {
    match entry.file_type() {
        Ok(ft) if ft.is_symlink() => match std::fs::metadata(entry.path()) {
            Ok(target) if target.is_dir() => FileEntryKind::SymlinkDirectory,
            Ok(target) if target.is_file() => FileEntryKind::SymlinkFile,
            Ok(_) => FileEntryKind::UnsupportedSpecial,
            Err(_) => FileEntryKind::BrokenSymlink,
        },
        Ok(ft) if ft.is_dir() => FileEntryKind::Directory,
        Ok(ft) if ft.is_file() => FileEntryKind::RegularFile,
        Ok(_) | Err(_) => FileEntryKind::UnsupportedSpecial,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    // TP-FIP-ICON-01..05: every filesystem observation maps to its canonical
    // kind; broken/special targets carry no operation or reference authority.
    #[test]
    fn classify_covers_all_six_entry_kinds() {
        let root = std::env::temp_dir().join(format!(
            "herdr-entry-kind-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        std::fs::create_dir_all(root.join("dir")).expect("dir");
        std::fs::write(root.join("file.txt"), b"x").expect("file");
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(root.join("dir"), root.join("link-dir")).expect("link-dir");
            std::os::unix::fs::symlink(root.join("file.txt"), root.join("link-file"))
                .expect("link-file");
            std::os::unix::fs::symlink(root.join("missing"), root.join("broken")).expect("broken");
            let status = std::process::Command::new("mkfifo")
                .arg(root.join("fifo"))
                .status()
                .expect("mkfifo runs");
            assert!(status.success(), "fifo fixture must exist");
        }

        let kinds: HashMap<String, FileEntryKind> = std::fs::read_dir(&root)
            .expect("read fixture root")
            .map(|entry| {
                let entry = entry.expect("dir entry");
                (
                    entry.file_name().to_string_lossy().into_owned(),
                    classify_dir_entry(&entry),
                )
            })
            .collect();
        let _ = std::fs::remove_dir_all(&root);

        assert_eq!(kinds["dir"], FileEntryKind::Directory);
        assert_eq!(kinds["file.txt"], FileEntryKind::RegularFile);
        #[cfg(unix)]
        {
            assert_eq!(kinds["link-dir"], FileEntryKind::SymlinkDirectory);
            assert_eq!(kinds["link-file"], FileEntryKind::SymlinkFile);
            assert_eq!(kinds["broken"], FileEntryKind::BrokenSymlink);
            assert_eq!(kinds["fifo"], FileEntryKind::UnsupportedSpecial);
        }
    }

    #[test]
    fn broken_and_special_kinds_disable_operations_and_reference() {
        for kind in [
            FileEntryKind::BrokenSymlink,
            FileEntryKind::UnsupportedSpecial,
        ] {
            assert!(!kind.supports_native_operation());
            assert!(!kind.supports_agent_reference());
            assert!(!kind.is_directory_target());
        }
        for kind in [
            FileEntryKind::Directory,
            FileEntryKind::RegularFile,
            FileEntryKind::SymlinkDirectory,
            FileEntryKind::SymlinkFile,
        ] {
            assert!(kind.supports_native_operation());
            assert!(kind.supports_agent_reference());
        }
        assert!(FileEntryKind::SymlinkDirectory.is_directory_target());
        assert!(!FileEntryKind::SymlinkFile.is_directory_target());
    }
}
