//! Crash-durable file persistence shared by every state file herdr owns.
//!
//! Why this exists (measured 2026-08-26, btrfs `/home`): the session, the
//! workspace-chat ledger and the closed-agent graveyard were each written as
//! `fs::write(tmp)` + `rename`, with no `fsync`. A system crash landed the
//! renames on disk before the data, the files came back empty, every loader
//! "degraded to empty" as designed — and the very next save overwrote what
//! had been the last good copy. Twenty-eight panes, three hundred and fifty
//! filed chats and every module row were gone with nothing left to recover.
//!
//! Three rules, each pinned by a test (TP-PERSIST-01..03):
//!
//! 1. A save is `tmp → fsync(tmp) → rename → fsync(dir)`. The rename only
//!    becomes visible after the bytes are on the platter.
//! 2. Before the rename replaces a readable previous file, that file is kept
//!    as `<name>.bak` (hard link where the filesystem allows it, copy
//!    otherwise). One save back is always on disk.
//! 3. A file that fails to parse is never left in place to be overwritten:
//!    it is moved aside as `<name>.corrupt-<unix-secs>` and the loader falls
//!    back to `.bak`. A newer-version file is preserved the same way.
//!
//! Callers keep their own JSON types and version checks; this module owns the
//! bytes-on-disk contract and nothing else.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use tracing::warn;

/// Where the loader found its bytes, so callers can log honestly.
#[derive(Debug, PartialEq, Eq)]
pub enum Recovered {
    /// The primary file parsed.
    Primary,
    /// The primary file was corrupt (moved aside to the given path) and the
    /// `.bak` copy parsed instead.
    Backup { quarantined: PathBuf },
}

/// What a load produced.
#[derive(Debug)]
pub enum Loaded<T> {
    /// No primary file and no backup: first run.
    Missing,
    /// A value, plus where it came from.
    Value(T, Recovered),
    /// The primary was corrupt and moved aside, and no backup could stand in.
    /// The path names the quarantined copy so the person can look at it.
    Quarantined(PathBuf),
}

/// TP-PERSIST-01: write `bytes` to `path` so that a crash at any instant
/// leaves either the previous file or the complete new one — never a torn or
/// empty file that the next start would read as corrupt.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    write_atomic_with(path, bytes, &mut sync_file, &mut sync_dir)
}

/// The same write with the two sync points injected, so a test can prove they
/// run — and run before the rename — without a filesystem that can be
/// crashed on demand.
pub(crate) fn write_atomic_with(
    path: &Path,
    bytes: &[u8],
    sync_file: &mut dyn FnMut(&fs::File) -> io::Result<()>,
    sync_dir: &mut dyn FnMut(&Path) -> io::Result<()>,
) -> io::Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    let tmp = sibling(path, "tmp");
    let written = (|| {
        let mut file = fs::File::create(&tmp)?;
        io::Write::write_all(&mut file, bytes)?;
        sync_file(&file)?;
        Ok::<(), io::Error>(())
    })();
    if let Err(err) = written {
        let _ = fs::remove_file(&tmp);
        return Err(err);
    }
    keep_backup(path);
    #[cfg(windows)]
    if path.exists() {
        if let Err(err) = fs::remove_file(path) {
            let _ = fs::remove_file(&tmp);
            return Err(err);
        }
    }
    if let Err(err) = fs::rename(&tmp, path) {
        let _ = fs::remove_file(&tmp);
        return Err(err);
    }
    sync_dir(parent)
}

/// TP-PERSIST-03: read `path`, parse it with `parse`, and never leave an
/// unreadable file where the next save would overwrite it. `parse` returns
/// `Err` for anything that must not be trusted — bad JSON, a truncated file,
/// or a version this binary does not understand.
pub fn load_or_quarantine<T>(path: &Path, parse: &dyn Fn(&str) -> Result<T, String>) -> Loaded<T> {
    let backup = sibling(path, "bak");
    match read_and_parse(path, parse) {
        ReadOutcome::Missing => match read_and_parse(&backup, parse) {
            ReadOutcome::Parsed(value) => {
                warn!(path = %path.display(), "primary state file missing; restored from backup");
                Loaded::Value(
                    value,
                    Recovered::Backup {
                        quarantined: PathBuf::new(),
                    },
                )
            }
            _ => Loaded::Missing,
        },
        ReadOutcome::Parsed(value) => Loaded::Value(value, Recovered::Primary),
        ReadOutcome::Bad(reason) => {
            let quarantined = quarantine(path, &reason);
            match read_and_parse(&backup, parse) {
                ReadOutcome::Parsed(value) => {
                    warn!(
                        path = %path.display(),
                        quarantined = %quarantined.display(),
                        reason,
                        "state file unreadable; restored from backup"
                    );
                    Loaded::Value(value, Recovered::Backup { quarantined })
                }
                _ => {
                    warn!(
                        path = %path.display(),
                        quarantined = %quarantined.display(),
                        reason,
                        "state file unreadable and no backup parsed; starting empty"
                    );
                    Loaded::Quarantined(quarantined)
                }
            }
        }
    }
}

enum ReadOutcome<T> {
    Missing,
    Parsed(T),
    Bad(String),
}

fn read_and_parse<T>(path: &Path, parse: &dyn Fn(&str) -> Result<T, String>) -> ReadOutcome<T> {
    if !path.exists() {
        return ReadOutcome::Missing;
    }
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(err) => return ReadOutcome::Bad(format!("read failed: {err}")),
    };
    match parse(&content) {
        Ok(value) => ReadOutcome::Parsed(value),
        Err(reason) => ReadOutcome::Bad(reason),
    }
}

/// Move an unreadable file aside. The name carries the wall-clock second so
/// repeated crashes do not overwrite each other's evidence. On any failure the
/// file is left where it is — a quarantine that deletes is worse than none.
fn quarantine(path: &Path, reason: &str) -> PathBuf {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let target = sibling(path, &format!("corrupt-{secs}"));
    match fs::rename(path, &target) {
        Ok(()) => target,
        Err(err) => {
            warn!(path = %path.display(), %err, reason, "could not quarantine state file");
            path.to_path_buf()
        }
    }
}

/// TP-PERSIST-02: keep the file about to be replaced as `<name>.bak`, but
/// only when it is readable in the cheapest sense — non-empty. A hard link
/// costs nothing and shares no bytes with the new file; a copy is the
/// fallback for filesystems that refuse links. Failure is logged, never
/// fatal: a missing backup must not block the save that would create one.
fn keep_backup(path: &Path) {
    let Ok(meta) = fs::metadata(path) else {
        return;
    };
    if !meta.is_file() || meta.len() == 0 {
        return;
    }
    let backup = sibling(path, "bak");
    let _ = fs::remove_file(&backup);
    if fs::hard_link(path, &backup).is_ok() {
        return;
    }
    if let Err(err) = fs::copy(path, &backup) {
        warn!(path = %path.display(), %err, "could not keep a backup of the previous state file");
    }
}

fn sync_file(file: &fs::File) -> io::Result<()> {
    file.sync_all()
}

#[cfg(unix)]
fn sync_dir(dir: &Path) -> io::Result<()> {
    fs::File::open(dir)?.sync_all()
}

#[cfg(not(unix))]
fn sync_dir(_dir: &Path) -> io::Result<()> {
    // Directory handles cannot be fsynced on Windows; the file sync plus the
    // replace-rename is the strongest ordering the platform offers.
    Ok(())
}

/// `session.json` → `session.json.<suffix>` (the whole name, not the extension:
/// `with_extension` would turn `session.json` into `session.bak`).
fn sibling(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path
        .file_name()
        .map(|n| n.to_os_string())
        .unwrap_or_default();
    name.push(format!(".{suffix}"));
    path.with_file_name(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    fn temp_path(name: &str) -> PathBuf {
        let unique = format!(
            "herdr-durable-tests-{}-{}-{}",
            name,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        std::env::temp_dir().join(unique).join("state.json")
    }

    fn parse_json_with_version(content: &str) -> Result<serde_json::Value, String> {
        let value: serde_json::Value = serde_json::from_str(content).map_err(|e| e.to_string())?;
        match value.get("version").and_then(|v| v.as_u64()) {
            Some(1) => Ok(value),
            Some(other) => Err(format!("unsupported version {other}")),
            None => Err("no version".into()),
        }
    }

    // TP-PERSIST-01
    #[test]
    fn a_save_syncs_the_bytes_before_the_rename_and_the_directory_after() {
        let path = temp_path("sync-order");
        let events = RefCell::new(Vec::<String>::new());
        let path_for_file = path.clone();
        let mut sync_file = |_: &fs::File| {
            // At this instant the new bytes must not yet be visible at `path`.
            let visible = path_for_file.exists();
            events.borrow_mut().push(format!("file(visible={visible})"));
            Ok(())
        };
        let mut sync_dir = |dir: &Path| {
            assert_eq!(dir, path.parent().unwrap());
            events.borrow_mut().push("dir".into());
            Ok(())
        };
        write_atomic_with(&path, b"{\"version\":1}", &mut sync_file, &mut sync_dir).unwrap();
        assert_eq!(
            events.into_inner(),
            vec!["file(visible=false)".to_string(), "dir".to_string()],
            "file sync happens before the rename makes the bytes visible; dir sync after"
        );
        assert_eq!(fs::read_to_string(&path).unwrap(), "{\"version\":1}");
        assert!(
            !sibling(&path, "tmp").exists(),
            "no temp file survives a save"
        );
    }

    // TP-PERSIST-01
    #[test]
    fn a_failed_file_sync_leaves_the_previous_file_untouched() {
        let path = temp_path("sync-fails");
        write_atomic(&path, b"{\"version\":1,\"n\":1}").unwrap();
        let mut failing = |_: &fs::File| Err(io::Error::other("disk full"));
        let mut sync_dir = |_: &Path| Ok(());
        let err = write_atomic_with(
            &path,
            b"{\"version\":1,\"n\":2}",
            &mut failing,
            &mut sync_dir,
        )
        .unwrap_err();
        assert_eq!(err.to_string(), "disk full");
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "{\"version\":1,\"n\":1}"
        );
        assert!(!sibling(&path, "tmp").exists());
    }

    // TP-PERSIST-02
    #[test]
    fn a_save_keeps_the_previous_readable_file_as_a_backup() {
        let path = temp_path("backup");
        write_atomic(&path, b"{\"version\":1,\"n\":1}").unwrap();
        assert!(
            !sibling(&path, "bak").exists(),
            "the first save has nothing to back up"
        );
        write_atomic(&path, b"{\"version\":1,\"n\":2}").unwrap();
        assert_eq!(
            fs::read_to_string(sibling(&path, "bak")).unwrap(),
            "{\"version\":1,\"n\":1}"
        );
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "{\"version\":1,\"n\":2}"
        );
        write_atomic(&path, b"{\"version\":1,\"n\":3}").unwrap();
        assert_eq!(
            fs::read_to_string(sibling(&path, "bak")).unwrap(),
            "{\"version\":1,\"n\":2}"
        );
    }

    // TP-PERSIST-02
    #[test]
    fn an_empty_previous_file_is_not_promoted_to_backup() {
        let path = temp_path("empty-no-backup");
        write_atomic(&path, b"{\"version\":1,\"n\":1}").unwrap();
        write_atomic(&path, b"{\"version\":1,\"n\":2}").unwrap();
        // Simulate the crash outcome: the primary is now zero bytes.
        fs::write(&path, b"").unwrap();
        write_atomic(&path, b"{\"version\":1,\"n\":3}").unwrap();
        assert_eq!(
            fs::read_to_string(sibling(&path, "bak")).unwrap(),
            "{\"version\":1,\"n\":1}",
            "the zero-byte file must not replace the last good backup"
        );
    }

    // TP-PERSIST-03
    #[test]
    fn a_corrupt_file_is_quarantined_and_the_backup_is_restored() {
        let path = temp_path("quarantine-restore");
        write_atomic(&path, b"{\"version\":1,\"n\":1}").unwrap();
        write_atomic(&path, b"{\"version\":1,\"n\":2}").unwrap();
        fs::write(&path, b"").unwrap();
        match load_or_quarantine(&path, &parse_json_with_version) {
            Loaded::Value(value, Recovered::Backup { quarantined }) => {
                assert_eq!(value["n"], 1);
                assert!(
                    quarantined.exists(),
                    "the corrupt file is kept, not deleted"
                );
                assert!(quarantined
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("state.json.corrupt-"));
                assert_eq!(fs::read_to_string(&quarantined).unwrap(), "");
            }
            other => panic!("expected a backup restore, got {other:?}"),
        }
        assert!(
            !path.exists(),
            "the corrupt primary no longer sits where a save would overwrite it"
        );
    }

    // TP-PERSIST-03
    #[test]
    fn a_corrupt_file_without_backup_is_quarantined_not_overwritten() {
        let path = temp_path("quarantine-only");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"{not json").unwrap();
        match load_or_quarantine(&path, &parse_json_with_version) {
            Loaded::Quarantined(quarantined) => {
                assert_eq!(fs::read_to_string(&quarantined).unwrap(), "{not json");
            }
            other => panic!("expected quarantine, got {other:?}"),
        }
        assert!(!path.exists());
        // A later save creates a fresh primary but the evidence survives.
        write_atomic(&path, b"{\"version\":1}").unwrap();
        assert!(fs::read_dir(path.parent().unwrap())
            .unwrap()
            .flatten()
            .any(|e| e
                .file_name()
                .to_string_lossy()
                .starts_with("state.json.corrupt-")));
    }

    // TP-PERSIST-03
    #[test]
    fn a_newer_version_file_is_preserved_the_same_way() {
        let path = temp_path("newer-version");
        write_atomic(&path, b"{\"version\":1,\"n\":1}").unwrap();
        write_atomic(&path, b"{\"version\":99,\"n\":2}").unwrap();
        match load_or_quarantine(&path, &parse_json_with_version) {
            Loaded::Value(value, Recovered::Backup { quarantined }) => {
                assert_eq!(value["n"], 1);
                assert!(fs::read_to_string(&quarantined)
                    .unwrap()
                    .contains("\"version\":99"));
            }
            other => panic!("expected the older backup, got {other:?}"),
        }
    }

    #[test]
    fn a_missing_file_with_no_backup_is_a_first_run() {
        let path = temp_path("missing");
        assert!(matches!(
            load_or_quarantine(&path, &parse_json_with_version),
            Loaded::Missing
        ));
        assert!(!path.parent().unwrap().exists(), "loading creates nothing");
    }

    #[test]
    fn a_missing_primary_with_a_backup_restores_the_backup() {
        let path = temp_path("missing-with-backup");
        write_atomic(&path, b"{\"version\":1,\"n\":1}").unwrap();
        write_atomic(&path, b"{\"version\":1,\"n\":2}").unwrap();
        fs::remove_file(&path).unwrap();
        match load_or_quarantine(&path, &parse_json_with_version) {
            Loaded::Value(value, Recovered::Backup { .. }) => assert_eq!(value["n"], 1),
            other => panic!("expected the backup, got {other:?}"),
        }
    }

    #[test]
    fn sibling_appends_to_the_whole_file_name() {
        assert_eq!(
            sibling(Path::new("/x/session.json"), "bak"),
            PathBuf::from("/x/session.json.bak")
        );
    }
}
