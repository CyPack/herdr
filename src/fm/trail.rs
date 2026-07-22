//! Miller TRAIL core — pure client-local navigation state per the canonical
//! trail UX contract (docs/superpowers/specs/2026-07-18-herdr-miller-trail-
//! ux-contract.md, LAW 1): columns accumulate from the root; a folder select
//! truncates and branches; a file select never appends a column; every
//! visible column is by construction a loaded directory. No filesystem,
//! terminal, or server work happens here (AppState pure-data principle).

use std::path::{Path, PathBuf};

/// Hard depth bound. Beyond it the trail slides: the oldest column is
/// dropped so navigation never stalls (deliberate deviation from a
/// "root always visible" reading — the root stays reachable by selecting
/// upward in remaining ancestors).
pub(crate) const MAX_TRAIL_DEPTH: usize = 32;

/// One trail column: the directory it lists and the exact selected child
/// path (exact-path identity, never a row index).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TrailCol {
    pub directory: PathBuf,
    pub selected: Option<PathBuf>,
}

/// The whole trail. Column 0 is the (current) root; the last column is the
/// deepest open directory. `active_col` is the SINGLE focus authority shared
/// by keyboard and mouse (contract LAW 2) and never dangles past the trail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TrailState {
    cols: Vec<TrailCol>,
    active_col: usize,
    /// Ephemeral keyboard/wheel cursor. This is deliberately separate from
    /// `TrailCol::selected`: the latter owns the activated directory chain,
    /// while this exact path may move inside one column without truncating or
    /// extending that chain.
    cursor: Option<(usize, PathBuf)>,
}

impl TrailState {
    /// A fresh trail rooted at one directory: exactly one column, nothing
    /// selected.
    pub(crate) fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            cols: vec![TrailCol {
                directory: root.into(),
                selected: None,
            }],
            active_col: 0,
            cursor: None,
        }
    }

    /// The single active-column authority (LAW 2): keyboard and mouse share
    /// this focus; every trail transition keeps it inside the trail.
    pub(crate) fn active_col(&self) -> usize {
        self.active_col
    }

    /// Move focus one column toward the root. Returns false at the root.
    pub(crate) fn move_active_left(&mut self) -> bool {
        if self.active_col == 0 {
            return false;
        }
        self.active_col -= 1;
        self.cursor = None;
        true
    }

    /// Move focus one column deeper. Returns false when no deeper column
    /// exists.
    pub(crate) fn move_active_right(&mut self) -> bool {
        if self.active_col >= self.deepest() {
            return false;
        }
        self.active_col += 1;
        self.cursor = None;
        true
    }

    /// Focus one exact live column. Mouse wheel/bulk modifiers use this before
    /// applying their row-local action; stale projected indices are inert.
    pub(crate) fn focus_col(&mut self, col_idx: usize) -> bool {
        if col_idx >= self.cols.len() {
            return false;
        }
        if self.active_col != col_idx {
            self.cursor = None;
        }
        self.active_col = col_idx;
        true
    }

    /// Exact cursor identity for one column. A vertical cursor override wins;
    /// otherwise the activated branch/file selection remains the fallback.
    pub(crate) fn cursor_path_in_col(&self, col_idx: usize) -> Option<&Path> {
        self.cursor
            .as_ref()
            .filter(|(cursor_col, _)| *cursor_col == col_idx)
            .map(|(_, path)| path.as_path())
            .or_else(|| self.cols.get(col_idx)?.selected.as_deref())
    }

    /// Active ephemeral cursor, used by render and exact-entry resolution.
    pub(crate) fn cursor_override(&self) -> Option<(usize, &Path)> {
        self.cursor
            .as_ref()
            .map(|(col_idx, path)| (*col_idx, path.as_path()))
    }

    /// Move only the row cursor in one exact live column. The activated Trail
    /// chain is intentionally untouched; callers may schedule a discardable
    /// preview separately.
    pub(crate) fn move_cursor_to(&mut self, col_idx: usize, path: &Path) -> bool {
        if col_idx >= self.cols.len() || self.cursor_path_in_col(col_idx) == Some(path) {
            return false;
        }
        self.active_col = col_idx;
        self.cursor = Some((col_idx, path.to_path_buf()));
        true
    }

    pub(crate) fn cols(&self) -> &[TrailCol] {
        &self.cols
    }

    /// Exact selected path from the deepest marked column. A directory branch
    /// ends with one unselected child column, so reading only `last()` would
    /// lose the directory that opened it.
    pub(crate) fn selected_path(&self) -> Option<&Path> {
        self.cols
            .iter()
            .rev()
            .find_map(|col| col.selected.as_deref())
    }

    /// Clear one column's selection and every deeper branch while preserving
    /// all ancestors. Used by the T7 bridge before applying a freshly prepared
    /// current-directory selection.
    pub(crate) fn clear_selection_at(&mut self, col_idx: usize) -> bool {
        if col_idx >= self.cols.len() {
            return false;
        }
        self.cols.truncate(col_idx + 1);
        self.cols[col_idx].selected = None;
        self.active_col = col_idx;
        self.cursor = None;
        true
    }

    /// Index of the deepest column.
    pub(crate) fn deepest(&self) -> usize {
        self.cols.len().saturating_sub(1)
    }

    /// LAW 1: selecting a FOLDER in column `col_idx` truncates every deeper
    /// column, marks the selection, and appends one new column listing the
    /// selected folder. Returns false (no change) for an out-of-range column.
    pub(crate) fn select_dir(&mut self, col_idx: usize, child: &Path) -> bool {
        if !self.mark_selection(col_idx, child) {
            return false;
        }
        self.cols.push(TrailCol {
            directory: child.to_path_buf(),
            selected: None,
        });
        if self.cols.len() > MAX_TRAIL_DEPTH {
            self.cols.remove(0);
        }
        // A folder select focuses the NEW column (LAW 2).
        self.active_col = self.deepest();
        true
    }

    /// LAW 1: selecting a FILE truncates deeper columns and marks the
    /// selection but NEVER appends a column (the detail panel owns files).
    pub(crate) fn select_file(&mut self, col_idx: usize, child: &Path) -> bool {
        self.mark_selection(col_idx, child)
    }

    /// Shared truncate-and-mark step: cut every column deeper than
    /// `col_idx`, then record the exact selected child path. Focus follows
    /// the selection and truncation can never leave it dangling.
    fn mark_selection(&mut self, col_idx: usize, child: &Path) -> bool {
        if col_idx >= self.cols.len() {
            return false;
        }
        self.cols.truncate(col_idx + 1);
        self.cols[col_idx].selected = Some(child.to_path_buf());
        self.active_col = col_idx;
        self.cursor = None;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> PathBuf {
        PathBuf::from(s)
    }

    // LAW 1: folder select branches — trail truncates to the clicked column
    // and grows exactly one new column listing the selected folder.
    #[test]
    fn folder_select_truncates_and_branches_trail() {
        let mut trail = TrailState::new("/root");
        assert!(trail.select_dir(0, &p("/root/a")));
        assert!(trail.select_dir(1, &p("/root/a/b")));
        assert_eq!(trail.cols().len(), 3);
        assert_eq!(
            trail.cols()[0].selected.as_deref(),
            Some(p("/root/a").as_path())
        );
        assert_eq!(trail.cols()[1].directory, p("/root/a"));
        assert_eq!(
            trail.cols()[1].selected.as_deref(),
            Some(p("/root/a/b").as_path())
        );
        assert_eq!(trail.cols()[2].directory, p("/root/a/b"));
        assert_eq!(trail.cols()[2].selected, None);
        assert_eq!(trail.deepest(), 2);
    }

    // LAW 1: a file select marks the selection but appends NO column.
    #[test]
    fn file_select_never_appends_a_column() {
        let mut trail = TrailState::new("/root");
        assert!(trail.select_dir(0, &p("/root/a")));
        assert!(trail.select_file(1, &p("/root/a/notes.md")));
        assert_eq!(trail.cols().len(), 2, "a file never opens a column");
        assert_eq!(
            trail.cols()[1].selected.as_deref(),
            Some(p("/root/a/notes.md").as_path())
        );
    }

    // TP-TRAIL-T7-BRIDGE-01: exact-path selection is read from the deepest
    // marked column, not from the final column (which is empty after a folder
    // branch). This is the single path authority consumed by FmState.
    #[test]
    fn selected_path_uses_deepest_marked_column() {
        let mut trail = TrailState::new("/root");
        assert_eq!(trail.selected_path(), None);

        assert!(trail.select_dir(0, &p("/root/a")));
        assert_eq!(trail.selected_path(), Some(p("/root/a").as_path()));

        assert!(trail.select_file(1, &p("/root/a/notes.md")));
        assert_eq!(trail.selected_path(), Some(p("/root/a/notes.md").as_path()));
    }

    // LAW 1: selecting a SIBLING in an ancestor column cuts the old branch
    // and regrows from that point — there is no "back", only the trail.
    #[test]
    fn ancestor_sibling_select_rebranches() {
        let mut trail = TrailState::new("/root");
        assert!(trail.select_dir(0, &p("/root/a")));
        assert!(trail.select_dir(1, &p("/root/a/b")));
        assert!(trail.select_dir(0, &p("/root/z")));
        assert_eq!(trail.cols().len(), 2, "old sub-branch is discarded");
        assert_eq!(
            trail.cols()[0].selected.as_deref(),
            Some(p("/root/z").as_path())
        );
        assert_eq!(trail.cols()[1].directory, p("/root/z"));
        assert_eq!(trail.cols()[1].selected, None);
    }

    // Bounded: past MAX_TRAIL_DEPTH the trail slides (oldest column drops),
    // so the newest column is always the just-entered directory.
    #[test]
    fn trail_depth_stays_bounded() {
        let mut trail = TrailState::new("/d0");
        for i in 0..MAX_TRAIL_DEPTH + 5 {
            let deepest = trail.deepest();
            let child = trail.cols()[deepest].directory.join(format!("c{i}"));
            assert!(trail.select_dir(deepest, &child));
            assert!(trail.cols().len() <= MAX_TRAIL_DEPTH, "depth stays bounded");
            assert_eq!(
                trail.cols()[trail.deepest()].directory,
                child,
                "the newest column is always the just-entered directory"
            );
        }
    }

    // LAW 2: the active column follows every trail transition — a folder
    // select focuses the NEW column, a file select focuses ITS column, and
    // truncation can never leave the focus dangling.
    #[test]
    fn active_column_follows_trail_transitions() {
        let mut trail = TrailState::new("/root");
        assert_eq!(trail.active_col(), 0);
        assert!(trail.select_dir(0, &p("/root/a")));
        assert_eq!(
            trail.active_col(),
            1,
            "folder select focuses the new column"
        );
        assert!(trail.select_dir(1, &p("/root/a/b")));
        assert_eq!(trail.active_col(), 2);
        assert!(trail.select_file(1, &p("/root/a/x.txt")));
        assert_eq!(
            trail.active_col(),
            1,
            "file select focuses its own column and truncation clamps"
        );
    }

    // LAW 2: keyboard column moves walk the trail without changing it —
    // left toward the root, right toward the deepest column, both clamped.
    #[test]
    fn active_column_moves_left_and_right_within_the_trail() {
        let mut trail = TrailState::new("/root");
        assert!(trail.select_dir(0, &p("/root/a")));
        assert!(trail.select_dir(1, &p("/root/a/b")));
        assert_eq!(trail.active_col(), 2);
        assert!(trail.move_active_left());
        assert_eq!(trail.active_col(), 1);
        assert!(trail.move_active_left());
        assert_eq!(trail.active_col(), 0);
        assert!(!trail.move_active_left(), "root is the left boundary");
        assert!(trail.move_active_right());
        assert!(trail.move_active_right());
        assert_eq!(trail.active_col(), 2);
        assert!(!trail.move_active_right(), "deepest is the right boundary");
        assert_eq!(trail.cols().len(), 3, "focus moves never mutate the trail");
    }

    // Out-of-range column indexes change nothing (stale-hit safety).
    #[test]
    fn out_of_range_column_is_a_no_op() {
        let mut trail = TrailState::new("/root");
        let before = trail.clone();
        assert!(!trail.select_dir(5, &p("/root/a")));
        assert!(!trail.select_file(5, &p("/root/a.txt")));
        assert_eq!(trail, before);
    }
}
