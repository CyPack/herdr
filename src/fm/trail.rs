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
/// deepest open directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TrailState {
    cols: Vec<TrailCol>,
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
        }
    }

    pub(crate) fn cols(&self) -> &[TrailCol] {
        &self.cols
    }

    /// Index of the deepest column.
    pub(crate) fn deepest(&self) -> usize {
        self.cols.len().saturating_sub(1)
    }

    /// LAW 1: selecting a FOLDER in column `col_idx` truncates every deeper
    /// column, marks the selection, and appends one new column listing the
    /// selected folder. Returns false (no change) for an out-of-range column.
    pub(crate) fn select_dir(&mut self, _col_idx: usize, _child: &Path) -> bool {
        false
    }

    /// LAW 1: selecting a FILE truncates deeper columns and marks the
    /// selection but NEVER appends a column (the detail panel owns files).
    pub(crate) fn select_file(&mut self, _col_idx: usize, _child: &Path) -> bool {
        false
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
