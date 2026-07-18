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

/// Outcome of one input activation against the loaded trail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TrailActivateOutcome {
    /// A directory was selected: the trail truncated and branched.
    Branched,
    /// A file was selected: the selection moved, no column appeared.
    SelectedFile,
    /// The hit was stale, out of range, or the target was unreadable —
    /// nothing changed.
    Rejected,
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
    /// index still lists the same directory, otherwise load it fresh. This
    /// single pass covers append, truncate, and the sliding-window drop.
    pub(crate) fn sync(&mut self, trail: &TrailState) {
        let cols = trail.cols();
        self.cols.truncate(cols.len());
        for (idx, col) in cols.iter().enumerate() {
            let cached = self
                .cols
                .get(idx)
                .is_some_and(|snap| snap.directory == col.directory);
            if cached {
                continue;
            }
            let loaded = TrailColSnapshot {
                snapshot: read_directory_snapshot(&col.directory, self.show_hidden),
                directory: col.directory.clone(),
            };
            if idx < self.cols.len() {
                self.cols[idx] = loaded;
            } else {
                self.cols.push(loaded);
            }
        }
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
        if col_idx >= trail.cols().len() {
            return FmDirectoryStatus::Unavailable;
        }
        let snapshot = read_directory_snapshot(child, self.show_hidden);
        if snapshot.status != FmDirectoryStatus::Available {
            return snapshot.status;
        }
        if !trail.select_dir(col_idx, child) {
            return FmDirectoryStatus::Unavailable;
        }
        // Mirror the trail transition exactly: truncate to the branch point,
        // append the pre-read column, then realign for the sliding window.
        self.cols.truncate(col_idx + 1);
        self.cols.push(TrailColSnapshot {
            snapshot,
            directory: child.to_path_buf(),
        });
        if self.cols.len() > trail.cols().len() {
            self.cols.remove(0);
        }
        self.sync(trail);
        FmDirectoryStatus::Available
    }

    /// Activate one entry (mouse click or keyboard Enter) against the CURRENT
    /// loaded listing. `expected_path` is the path the input frame saw; a
    /// mismatch with the live snapshot means the hit is stale and is
    /// rejected without touching any state (generation-safe hits).
    /// Directories branch the trail (fail-closed via `select_dir`); files
    /// only mark the selection (LAW 1/3).
    pub(crate) fn activate_entry(
        &mut self,
        trail: &mut TrailState,
        col_idx: usize,
        entry_index: usize,
        expected_path: &Path,
    ) -> TrailActivateOutcome {
        let Some(entry) = self
            .cols
            .get(col_idx)
            .and_then(|col| col.snapshot.entries.get(entry_index))
        else {
            return TrailActivateOutcome::Rejected;
        };
        if entry.path != expected_path {
            return TrailActivateOutcome::Rejected;
        }
        if entry.kind.is_directory_target() {
            let target = entry.path.clone();
            if self.select_dir(trail, col_idx, &target) == FmDirectoryStatus::Available {
                TrailActivateOutcome::Branched
            } else {
                TrailActivateOutcome::Rejected
            }
        } else {
            let target = entry.path.clone();
            if trail.select_file(col_idx, &target) {
                self.sync(trail);
                TrailActivateOutcome::SelectedFile
            } else {
                TrailActivateOutcome::Rejected
            }
        }
    }

    /// Keyboard selection move inside the ACTIVE column: step the selection
    /// by `delta` rows (clamped) and activate the landed entry with the same
    /// semantics as a click — directories branch, files mark.
    pub(crate) fn move_selection(
        &mut self,
        trail: &mut TrailState,
        delta: isize,
    ) -> TrailActivateOutcome {
        let col_idx = trail.active_col();
        let Some(col) = self.cols.get(col_idx) else {
            return TrailActivateOutcome::Rejected;
        };
        let entries = &col.snapshot.entries;
        if entries.is_empty() {
            return TrailActivateOutcome::Rejected;
        }
        let current = trail.cols()[col_idx]
            .selected
            .as_deref()
            .and_then(|selected| entries.iter().position(|entry| entry.path == selected));
        let landed = match current {
            Some(index) => index
                .saturating_add_signed(delta)
                .min(entries.len().saturating_sub(1)),
            // No selection yet: the first step lands on the edge row.
            None if delta >= 0 => 0,
            None => entries.len() - 1,
        };
        let expected = entries[landed].path.clone();
        self.activate_entry(trail, col_idx, landed, &expected)
    }

    /// Re-read one column from disk (watcher refresh path). Selection lives
    /// in the trail as an exact path and is never touched here. The column
    /// keeps its explicit status even when the directory disappeared —
    /// honest state, never a silent placeholder.
    pub(crate) fn refresh_col(&mut self, col_idx: usize) -> bool {
        let Some(col) = self.cols.get_mut(col_idx) else {
            return false;
        };
        col.snapshot = read_directory_snapshot(&col.directory, self.show_hidden);
        true
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

    // LAW 1 via input: activating a FOLDER row branches the trail at the
    // hit column with click semantics.
    #[test]
    fn folder_activation_branches_at_hit_column() {
        let td = TempDir::new("act-dir");
        let sub = td.root.join("sub");
        fs::create_dir_all(&sub).expect("sub dir");
        let mut trail = TrailState::new(&td.root);
        let mut snaps = TrailSnapshots::new(false);
        snaps.sync(&trail);

        let entry_index = snaps.cols()[0]
            .entries()
            .iter()
            .position(|entry| entry.path == sub)
            .expect("sub row");
        assert_eq!(
            snaps.activate_entry(&mut trail, 0, entry_index, &sub),
            TrailActivateOutcome::Branched
        );
        assert_eq!(trail.cols().len(), 2);
        assert_eq!(trail.cols()[1].directory, sub);
        assert_eq!(snaps.cols().len(), 2, "the branched column is loaded");
    }

    // LAW 1 via input: activating a sibling in an ANCESTOR column discards
    // the old branch and regrows from that point.
    #[test]
    fn ancestor_sibling_activation_rebranches() {
        let td = TempDir::new("act-sibling");
        let a = td.root.join("a");
        let b = a.join("b");
        let z = td.root.join("z");
        fs::create_dir_all(&b).expect("nested");
        fs::create_dir_all(&z).expect("sibling");
        let mut trail = TrailState::new(&td.root);
        let mut snaps = TrailSnapshots::new(false);
        snaps.sync(&trail);
        assert_eq!(
            snaps.select_dir(&mut trail, 0, &a),
            FmDirectoryStatus::Available
        );
        assert_eq!(
            snaps.select_dir(&mut trail, 1, &b),
            FmDirectoryStatus::Available
        );

        let z_index = snaps.cols()[0]
            .entries()
            .iter()
            .position(|entry| entry.path == z)
            .expect("z row");
        assert_eq!(
            snaps.activate_entry(&mut trail, 0, z_index, &z),
            TrailActivateOutcome::Branched
        );
        assert_eq!(trail.cols().len(), 2, "old sub-branch discarded");
        assert_eq!(trail.cols()[1].directory, z);
        assert_eq!(snaps.cols()[1].directory(), z.as_path());
    }

    // LAW 1/3 via input: activating a FILE marks the selection and never
    // opens a column.
    #[test]
    fn file_activation_marks_selection_only() {
        let td = TempDir::new("act-file");
        fs::write(td.root.join("doc.md"), b"x").expect("file");
        let mut trail = TrailState::new(&td.root);
        let mut snaps = TrailSnapshots::new(false);
        snaps.sync(&trail);

        let doc = td.root.join("doc.md");
        assert_eq!(
            snaps.activate_entry(&mut trail, 0, 0, &doc),
            TrailActivateOutcome::SelectedFile
        );
        assert_eq!(trail.cols().len(), 1, "a file never opens a column");
        assert_eq!(trail.cols()[0].selected.as_deref(), Some(doc.as_path()));
    }

    // Generation-safe hits: a hit whose remembered path no longer matches
    // the live snapshot at that index is stale and rejected outright.
    #[test]
    fn stale_hit_with_mismatched_path_is_rejected() {
        let td = TempDir::new("act-stale");
        fs::write(td.root.join("old.txt"), b"x").expect("file");
        let mut trail = TrailState::new(&td.root);
        let mut snaps = TrailSnapshots::new(false);
        snaps.sync(&trail);
        let before = trail.clone();

        // The input frame remembered a path that is not what row 0 lists now.
        let remembered = td.root.join("vanished.txt");
        assert_eq!(
            snaps.activate_entry(&mut trail, 0, 0, &remembered),
            TrailActivateOutcome::Rejected
        );
        assert_eq!(trail, before, "stale hits change nothing");
        // Out-of-range indexes are equally inert.
        assert_eq!(
            snaps.activate_entry(&mut trail, 0, 99, &remembered),
            TrailActivateOutcome::Rejected
        );
        assert_eq!(
            snaps.activate_entry(&mut trail, 7, 0, &remembered),
            TrailActivateOutcome::Rejected
        );
        assert_eq!(trail, before);
    }

    // LAW 2 keyboard: Down/Up move the selection inside the ACTIVE column
    // with click semantics — a directory branches, a file only marks.
    #[test]
    fn keyboard_selection_moves_within_active_column() {
        let td = TempDir::new("kbd");
        let alpha = td.root.join("alpha");
        fs::create_dir_all(&alpha).expect("alpha");
        fs::write(td.root.join("beta.txt"), b"x").expect("beta");
        let mut trail = TrailState::new(&td.root);
        let mut snaps = TrailSnapshots::new(false);
        snaps.sync(&trail);

        // First Down lands on row 0: the directory `alpha` → branch.
        assert_eq!(
            snaps.move_selection(&mut trail, 1),
            TrailActivateOutcome::Branched
        );
        assert_eq!(trail.cols().len(), 2);
        assert_eq!(trail.cols()[0].selected.as_deref(), Some(alpha.as_path()));

        // Focus back on the root column, step down to the file row.
        assert!(trail.move_active_left());
        assert_eq!(
            snaps.move_selection(&mut trail, 1),
            TrailActivateOutcome::SelectedFile
        );
        assert_eq!(
            trail.cols()[0].selected.as_deref(),
            Some(td.root.join("beta.txt").as_path())
        );
        assert_eq!(trail.cols().len(), 1, "file selection cut the branch");

        // Clamped at the listing edge: stepping past the end stays put.
        assert_eq!(
            snaps.move_selection(&mut trail, 5),
            TrailActivateOutcome::SelectedFile
        );
        assert_eq!(
            trail.cols()[0].selected.as_deref(),
            Some(td.root.join("beta.txt").as_path())
        );
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
