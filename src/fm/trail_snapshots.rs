//! Snapshot bridge for the Miller trail (trail program T2): pairs every
//! `TrailCol` with one loaded `FmDirectorySnapshot` so a visible column is by
//! construction a loaded directory — a "(unavailable)" placeholder column is
//! structurally impossible (canonical trail UX contract, LAW 1). Directory
//! selection is fail-closed: the target is read FIRST and the trail only
//! branches when the read succeeds (same policy as `prepare_navigation_io`).
//! Selection identity stays in `TrailState` as an exact path, so a watcher
//! refresh replaces entries without ever touching what is selected.

use std::path::{Path, PathBuf};

use super::trail::TrailState;
use super::{read_directory_snapshot, FileEntry, FmDirectorySnapshot, FmDirectoryStatus};

/// One trail column with its loaded directory listing.
pub(crate) struct TrailColSnapshot {
    directory: PathBuf,
    snapshot: FmDirectorySnapshot,
}

impl TrailColSnapshot {
    pub(crate) fn directory(&self) -> &Path {
        &self.directory
    }

    pub(crate) fn entries(&self) -> &[FileEntry] {
        &self.snapshot.entries
    }

    pub(crate) fn status(&self) -> FmDirectoryStatus {
        self.snapshot.status
    }
}

/// Loaded snapshots kept index-aligned with a `TrailState`.
pub(crate) struct TrailSnapshots {
    cols: Vec<TrailColSnapshot>,
    show_hidden: bool,
}

impl TrailSnapshots {
    pub(crate) fn new(show_hidden: bool) -> Self {
        Self {
            cols: Vec::new(),
            show_hidden,
        }
    }

    pub(crate) fn cols(&self) -> &[TrailColSnapshot] {
        &self.cols
    }

    /// Realign with the trail: keep a cached snapshot only when the same
    /// index still lists the same directory, otherwise load it fresh.
    pub(crate) fn sync(&mut self, trail: &TrailState) {
        let _ = trail;
    }

    /// Fail-closed folder selection: read the target first; branch the trail
    /// and append the loaded column only when the directory is `Available`.
    /// Out-of-range columns change nothing and report `Unavailable`.
    pub(crate) fn select_dir(
        &mut self,
        trail: &mut TrailState,
        col_idx: usize,
        child: &Path,
    ) -> FmDirectoryStatus {
        let _ = (trail, col_idx, child);
        FmDirectoryStatus::Unavailable
    }

    /// Re-read one column from disk (watcher refresh path). Selection lives
    /// in the trail as an exact path and is never touched here.
    pub(crate) fn refresh_col(&mut self, col_idx: usize) -> bool {
        let _ = col_idx;
        false
    }
}

#[cfg(test)]
mod tests {
    use super::super::trail::MAX_TRAIL_DEPTH;
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn unique() -> u64 {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        NEXT.fetch_add(1, Ordering::Relaxed)
    }

    /// Isolated temp directory, recursively removed on drop. Never touches
    /// any real user directory.
    struct TempDir {
        root: PathBuf,
    }

    impl TempDir {
        fn new(tag: &str) -> Self {
            let root = std::env::temp_dir().join(format!(
                "herdr-trail-test-{}-{}-{}",
                std::process::id(),
                tag,
                unique()
            ));
            fs::create_dir_all(&root).expect("create temp root");
            Self { root }
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn entry_names(col: &TrailColSnapshot) -> Vec<&str> {
        col.entries().iter().map(|e| e.name.as_str()).collect()
    }

    // LAW 1: after sync + folder selects, every visible column carries a
    // loaded Available snapshot whose directory matches the trail exactly —
    // a placeholder column cannot exist.
    #[test]
    fn every_visible_column_is_loaded() {
        let td = TempDir::new("loaded");
        let a = td.root.join("a");
        let b = a.join("b");
        fs::create_dir_all(&b).expect("create nested dirs");
        fs::write(a.join("x.txt"), b"x").expect("write file");

        let mut trail = TrailState::new(&td.root);
        let mut snaps = TrailSnapshots::new(false);
        snaps.sync(&trail);
        assert_eq!(snaps.cols().len(), 1, "root column is loaded on sync");

        assert_eq!(
            snaps.select_dir(&mut trail, 0, &a),
            FmDirectoryStatus::Available
        );
        assert_eq!(
            snaps.select_dir(&mut trail, 1, &b),
            FmDirectoryStatus::Available
        );

        assert_eq!(trail.cols().len(), 3);
        assert_eq!(snaps.cols().len(), 3, "one snapshot per trail column");
        for (idx, col) in snaps.cols().iter().enumerate() {
            assert_eq!(
                col.directory(),
                trail.cols()[idx].directory.as_path(),
                "column {idx} snapshot lists the trail directory"
            );
            assert_eq!(
                col.status(),
                FmDirectoryStatus::Available,
                "column {idx} is loaded"
            );
        }
        assert_eq!(entry_names(&snaps.cols()[0]), vec!["a"]);
        assert_eq!(entry_names(&snaps.cols()[1]), vec!["b", "x.txt"]);
    }

    // Fail-closed: an unreadable target reports its status and neither the
    // trail nor the snapshots change — the broken column never appears.
    #[test]
    fn unreadable_target_never_becomes_a_column() {
        let td = TempDir::new("missing");
        let mut trail = TrailState::new(&td.root);
        let mut snaps = TrailSnapshots::new(false);
        snaps.sync(&trail);

        let missing = td.root.join("does-not-exist");
        assert_eq!(
            snaps.select_dir(&mut trail, 0, &missing),
            FmDirectoryStatus::Missing
        );
        assert_eq!(trail.cols().len(), 1, "trail did not branch");
        assert_eq!(trail.cols()[0].selected, None, "nothing was selected");
        assert_eq!(snaps.cols().len(), 1, "no snapshot column appeared");
    }

    // Exact-path selection identity: a watcher-style refresh reloads the
    // entries but the selected path in the trail is untouched.
    #[test]
    fn watcher_refresh_keeps_selection_by_path() {
        let td = TempDir::new("refresh");
        fs::write(td.root.join("keep.txt"), b"x").expect("write file");

        let mut trail = TrailState::new(&td.root);
        let mut snaps = TrailSnapshots::new(false);
        snaps.sync(&trail);

        let keep = td.root.join("keep.txt");
        assert!(trail.select_file(0, &keep));

        fs::write(td.root.join("new.txt"), b"x").expect("write new file");
        assert!(snaps.refresh_col(0), "refresh reloads the column");

        assert_eq!(
            trail.cols()[0].selected.as_deref(),
            Some(keep.as_path()),
            "selection is exact-path and survives the refresh"
        );
        assert_eq!(entry_names(&snaps.cols()[0]), vec!["keep.txt", "new.txt"]);
    }

    // Bounded: past MAX_TRAIL_DEPTH the trail slides and the snapshots stay
    // index-aligned with it at every step.
    #[test]
    fn sliding_window_keeps_snapshots_aligned() {
        let td = TempDir::new("bounded");
        let mut deepest_dir = td.root.clone();
        for i in 0..MAX_TRAIL_DEPTH + 5 {
            deepest_dir = deepest_dir.join(format!("d{i}"));
        }
        fs::create_dir_all(&deepest_dir).expect("create deep tree");

        let mut trail = TrailState::new(&td.root);
        let mut snaps = TrailSnapshots::new(false);
        snaps.sync(&trail);

        let mut next = td.root.clone();
        for i in 0..MAX_TRAIL_DEPTH + 5 {
            next = next.join(format!("d{i}"));
            let deepest = trail.deepest();
            assert_eq!(
                snaps.select_dir(&mut trail, deepest, &next),
                FmDirectoryStatus::Available
            );
            assert!(snaps.cols().len() <= MAX_TRAIL_DEPTH);
            assert_eq!(snaps.cols().len(), trail.cols().len());
            for (idx, col) in snaps.cols().iter().enumerate() {
                assert_eq!(
                    col.directory(),
                    trail.cols()[idx].directory.as_path(),
                    "column {idx} stays aligned after sliding"
                );
            }
        }
    }

    // Stale-hit safety: an out-of-range column index changes nothing.
    #[test]
    fn out_of_range_select_is_a_no_op() {
        let td = TempDir::new("range");
        let sub = td.root.join("sub");
        fs::create_dir_all(&sub).expect("create sub dir");

        let mut trail = TrailState::new(&td.root);
        let mut snaps = TrailSnapshots::new(false);
        snaps.sync(&trail);

        let before = trail.clone();
        assert_eq!(
            snaps.select_dir(&mut trail, 5, &sub),
            FmDirectoryStatus::Unavailable
        );
        assert_eq!(trail, before, "trail unchanged");
        assert_eq!(snaps.cols().len(), 1, "snapshots unchanged");
    }
}
