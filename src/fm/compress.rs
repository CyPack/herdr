//! The Files surface's archive writer: N selected entries into ONE zip.
//!
//! Zip was chosen over rar (no open writer exists — the format's write side
//! is licensed proprietary) and over tar.zst (a second format, queued as a
//! later candidate) because every platform this app targets opens it with
//! nothing installed. Pure Rust through the `zip` crate — no system binary,
//! so the Windows target and the Linux one compress identically.
//!
//! This module is the ENGINE only: pure functions over paths, a cancel flag
//! and a progress callback, testable without the app. The operation-worker
//! seat that drives it (progress UI, cancel button, the `[zip]` header verb)
//! rides on top.
#![allow(dead_code)] // engine-first slice: the worker seat and the [zip] verb
                     // consume this in the next slice, and this allow leaves with them.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum CompressError {
    /// An io failure, carried with the path it happened on.
    Io {
        path: PathBuf,
        kind: std::io::ErrorKind,
    },
    /// The cancel flag was raised; the partial archive was removed.
    Cancelled,
    /// No sources, or a source with no file name to root its entries at.
    NothingToCompress,
    /// Symlinks are refused, the same answer the copy/move preflight gives —
    /// an archive that silently followed one would smuggle files from
    /// outside the selection.
    SourceSymlink { path: PathBuf },
}

fn io_err(path: &Path, error: &std::io::Error) -> CompressError {
    CompressError::Io {
        path: path.to_path_buf(),
        kind: error.kind(),
    }
}

/// The first free `stem.zip`, `stem (2).zip`, `stem (3).zip`… in `dir` —
/// the collision answer every desktop file manager gives.
pub(crate) fn unique_zip_destination(dir: &Path, stem: &str) -> PathBuf {
    let first = dir.join(format!("{stem}.zip"));
    if !first.exists() {
        return first;
    }
    let mut counter = 2u32;
    loop {
        let candidate = dir.join(format!("{stem} ({counter}).zip"));
        if !candidate.exists() {
            return candidate;
        }
        counter = counter.saturating_add(1);
    }
}

/// Every file that will become an archive entry: `(disk path, archive name)`.
///
/// Each source roots its entries at its own file name, so `a.txt` and
/// `photos/` become `a.txt` and `photos/…` — the layout the person sees in
/// the listing is the layout the archive carries.
fn collect_entries(sources: &[PathBuf]) -> Result<Vec<(PathBuf, String)>, CompressError> {
    let mut entries = Vec::new();
    for source in sources {
        let metadata = std::fs::symlink_metadata(source).map_err(|error| io_err(source, &error))?;
        if metadata.file_type().is_symlink() {
            return Err(CompressError::SourceSymlink {
                path: source.clone(),
            });
        }
        let root_name = source
            .file_name()
            .ok_or(CompressError::NothingToCompress)?
            .to_string_lossy()
            .into_owned();
        if metadata.is_dir() {
            collect_dir_entries(source, &root_name, &mut entries)?;
        } else {
            entries.push((source.clone(), root_name));
        }
    }
    if entries.is_empty() {
        return Err(CompressError::NothingToCompress);
    }
    Ok(entries)
}

fn collect_dir_entries(
    dir: &Path,
    prefix: &str,
    entries: &mut Vec<(PathBuf, String)>,
) -> Result<(), CompressError> {
    let mut children: Vec<_> = std::fs::read_dir(dir)
        .map_err(|error| io_err(dir, &error))?
        .collect::<Result<_, _>>()
        .map_err(|error| io_err(dir, &error))?;
    // Deterministic archive order regardless of readdir's mood.
    children.sort_by_key(|entry| entry.file_name());
    for child in children {
        let path = child.path();
        let metadata = std::fs::symlink_metadata(&path).map_err(|error| io_err(&path, &error))?;
        if metadata.file_type().is_symlink() {
            return Err(CompressError::SourceSymlink { path });
        }
        let name = format!("{prefix}/{}", child.file_name().to_string_lossy());
        if metadata.is_dir() {
            collect_dir_entries(&path, &name, entries)?;
        } else {
            entries.push((path, name));
        }
    }
    Ok(())
}

/// Write `sources` into the archive at `destination`.
///
/// `progress(done, total)` is called after every finished entry; a raised
/// `cancelled` flag stops before the next entry and removes the partial
/// archive — a half-written zip that looks finished is worse than none.
pub(crate) fn compress_paths_to_zip(
    sources: &[PathBuf],
    destination: &Path,
    cancelled: &AtomicBool,
    mut progress: impl FnMut(usize, usize),
) -> Result<(), CompressError> {
    let entries = collect_entries(sources)?;
    let total = entries.len();

    let file = std::fs::File::create(destination).map_err(|error| io_err(destination, &error))?;
    let mut writer = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    let abort = |writer: zip::ZipWriter<std::fs::File>, error: CompressError| {
        drop(writer);
        let _ = std::fs::remove_file(destination);
        Err(error)
    };

    let mut buffer = vec![0u8; 64 * 1024];
    for (done, (path, name)) in entries.into_iter().enumerate() {
        if cancelled.load(Ordering::Relaxed) {
            return abort(writer, CompressError::Cancelled);
        }
        if let Err(error) = writer.start_file(&name, options) {
            let error = std::io::Error::other(error.to_string());
            return abort(writer, io_err(&path, &error));
        }
        let mut source = match std::fs::File::open(&path) {
            Ok(source) => source,
            Err(error) => return abort(writer, io_err(&path, &error)),
        };
        loop {
            let read = match source.read(&mut buffer) {
                Ok(read) => read,
                Err(error) => return abort(writer, io_err(&path, &error)),
            };
            if read == 0 {
                break;
            }
            if let Err(error) = writer.write_all(&buffer[..read]) {
                return abort(writer, io_err(destination, &error));
            }
        }
        progress(done + 1, total);
    }
    if let Err(error) = writer.finish() {
        let error = std::io::Error::other(error.to_string());
        let _ = std::fs::remove_file(destination);
        return Err(io_err(destination, &error));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;

    fn tempdir(tag: &str) -> PathBuf {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "herdr-compress-{}-{}-{}",
            std::process::id(),
            tag,
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("temp root");
        root
    }

    fn read_archive(path: &Path) -> Vec<(String, Vec<u8>)> {
        let file = std::fs::File::open(path).expect("open archive");
        let mut archive = zip::ZipArchive::new(file).expect("parse archive");
        let mut out = Vec::new();
        for index in 0..archive.len() {
            let mut entry = archive.by_index(index).expect("entry");
            let mut content = Vec::new();
            entry.read_to_end(&mut content).expect("entry content");
            out.push((entry.name().to_string(), content));
        }
        out
    }

    // The round trip IS the feature: what went in comes out, under the
    // names the listing showed, directories carried recursively.
    #[test]
    fn a_selection_round_trips_through_the_archive() {
        let root = tempdir("roundtrip");
        std::fs::write(root.join("a.txt"), b"alpha").expect("a");
        let nested = root.join("photos");
        std::fs::create_dir_all(nested.join("inner")).expect("dirs");
        std::fs::write(nested.join("one.png"), b"png-one").expect("one");
        std::fs::write(nested.join("inner/two.png"), b"png-two").expect("two");

        let dest = unique_zip_destination(&root, "bundle");
        let mut seen = Vec::new();
        compress_paths_to_zip(
            &[root.join("a.txt"), nested.clone()],
            &dest,
            &AtomicBool::new(false),
            |done, total| seen.push((done, total)),
        )
        .expect("compress");

        let mut entries = read_archive(&dest);
        entries.sort();
        assert_eq!(
            entries,
            vec![
                ("a.txt".to_string(), b"alpha".to_vec()),
                ("photos/inner/two.png".to_string(), b"png-two".to_vec()),
                ("photos/one.png".to_string(), b"png-one".to_vec()),
            ]
        );
        assert_eq!(seen.last(), Some(&(3, 3)), "progress reached the total");
        let _ = std::fs::remove_dir_all(&root);
    }

    // The desktop collision answer: name.zip taken means name (2).zip,
    // then name (3).zip — never a silent overwrite.
    #[test]
    fn the_destination_steps_around_existing_archives() {
        let root = tempdir("unique");
        assert_eq!(unique_zip_destination(&root, "name"), root.join("name.zip"));
        std::fs::write(root.join("name.zip"), b"x").expect("first");
        assert_eq!(
            unique_zip_destination(&root, "name"),
            root.join("name (2).zip")
        );
        std::fs::write(root.join("name (2).zip"), b"x").expect("second");
        assert_eq!(
            unique_zip_destination(&root, "name"),
            root.join("name (3).zip")
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    // A raised cancel flag stops the work AND removes the partial archive —
    // a half-written zip that looks finished is worse than none.
    #[test]
    fn cancelling_removes_the_partial_archive() {
        let root = tempdir("cancel");
        std::fs::write(root.join("a.txt"), b"alpha").expect("a");
        let dest = root.join("out.zip");
        let result = compress_paths_to_zip(
            &[root.join("a.txt")],
            &dest,
            &AtomicBool::new(true),
            |_, _| {},
        );
        assert_eq!(result, Err(CompressError::Cancelled));
        assert!(!dest.exists(), "the partial archive was removed");
        let _ = std::fs::remove_dir_all(&root);
    }

    // Symlinks are refused the way the copy/move preflight refuses them —
    // an archive that followed one would smuggle files from outside the
    // selection.
    #[cfg(unix)]
    #[test]
    fn a_symlink_source_is_refused() {
        let root = tempdir("symlink");
        std::fs::write(root.join("real.txt"), b"x").expect("real");
        let link = root.join("link.txt");
        std::os::unix::fs::symlink(root.join("real.txt"), &link).expect("symlink");
        let dest = root.join("out.zip");
        let result = compress_paths_to_zip(
            std::slice::from_ref(&link),
            &dest,
            &AtomicBool::new(false),
            |_, _| {},
        );
        assert_eq!(result, Err(CompressError::SourceSymlink { path: link }));
        assert!(!dest.exists());
        let _ = std::fs::remove_dir_all(&root);
    }
}
