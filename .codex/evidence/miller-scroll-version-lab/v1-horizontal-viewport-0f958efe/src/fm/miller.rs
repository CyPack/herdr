//! Client-local width preferences for the Miller trail.
//!
//! Directory contents and selection identity live exclusively in
//! `TrailState`/`TrailSnapshots`. This module retains only bounded per-column
//! layout preferences and a revision used by typed resize transactions.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};

pub(crate) const MAX_MILLER_HISTORY_DEPTH: usize = 32;
pub(crate) const MILLER_COLUMN_MIN_WIDTH: u16 = 16;
pub(crate) const MILLER_COLUMN_PREFERRED_WIDTH: u16 = 28;
pub(crate) const MILLER_COLUMN_MAX_WIDTH: u16 = 64;
pub(crate) const MILLER_DETAIL_MIN_WIDTH: u16 = 20;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MillerPathSegment {
    pub directory: PathBuf,
    pub preferred_width: u16,
}

impl MillerPathSegment {
    pub(crate) fn new(directory: PathBuf) -> Self {
        Self {
            directory,
            preferred_width: MILLER_COLUMN_PREFERRED_WIDTH,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MillerHorizontalViewport {
    pub first_visible: usize,
    pub follow_active: bool,
}

impl Default for MillerHorizontalViewport {
    fn default() -> Self {
        Self {
            first_visible: 0,
            follow_active: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MillerAdjacentWidthTarget {
    Directory(PathBuf),
    Preview,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MillerState {
    pub chain: VecDeque<MillerPathSegment>,
    pub horizontal: MillerHorizontalViewport,
    pub focused_directory: PathBuf,
    pub preview_preferred_width: u16,
    pub revision: u64,
}

impl MillerState {
    pub(crate) fn seed(cwd: PathBuf) -> Self {
        let mut segments = vec![MillerPathSegment::new(cwd.clone())];
        let mut ancestor = cwd.parent().map(Path::to_path_buf);
        while let Some(directory) = ancestor {
            segments.push(MillerPathSegment::new(directory.clone()));
            ancestor = directory.parent().map(Path::to_path_buf);
        }
        segments.truncate(MAX_MILLER_HISTORY_DEPTH);
        segments.reverse();
        Self {
            chain: VecDeque::from(segments),
            horizontal: MillerHorizontalViewport::default(),
            focused_directory: cwd,
            preview_preferred_width: MILLER_COLUMN_PREFERRED_WIDTH,
            revision: 0,
        }
    }

    pub(crate) fn visit(&mut self, directory: PathBuf) {
        if let Some(existing_index) = self
            .chain
            .iter()
            .position(|segment| segment.directory == directory)
        {
            self.chain.truncate(existing_index.saturating_add(1));
        } else {
            self.chain
                .push_back(MillerPathSegment::new(directory.clone()));
        }
        while self.chain.len() > MAX_MILLER_HISTORY_DEPTH {
            self.chain.pop_front();
        }
        self.horizontal.first_visible = self
            .horizontal
            .first_visible
            .min(self.chain.len().saturating_sub(1));
        self.horizontal.follow_active = true;
        self.focused_directory = directory;
        self.revision = self.revision.saturating_add(1);
    }

    pub(crate) fn sync_trail_directories(&mut self, directories: &[PathBuf]) {
        if directories.is_empty()
            || self.chain.len() == directories.len()
                && self
                    .chain
                    .iter()
                    .zip(directories)
                    .all(|(segment, directory)| segment.directory == *directory)
        {
            return;
        }
        let chain = directories
            .iter()
            .take(MAX_MILLER_HISTORY_DEPTH)
            .map(|directory| {
                self.chain
                    .iter()
                    .find(|segment| segment.directory == *directory)
                    .cloned()
                    .unwrap_or_else(|| MillerPathSegment::new(directory.clone()))
            })
            .collect::<VecDeque<_>>();
        let Some(focused_directory) = chain.back().map(|segment| segment.directory.clone()) else {
            return;
        };
        self.chain = chain;
        self.focused_directory = focused_directory;
        self.horizontal.first_visible = self
            .horizontal
            .first_visible
            .min(self.chain.len().saturating_sub(1));
        self.horizontal.follow_active = true;
        self.revision = self.revision.saturating_add(1);
    }

    pub(crate) fn preferred_widths_for(
        &self,
        directories: impl Iterator<Item = PathBuf>,
    ) -> Vec<u16> {
        directories
            .map(|directory| {
                self.chain
                    .iter()
                    .find(|segment| segment.directory == directory)
                    .map_or(MILLER_COLUMN_PREFERRED_WIDTH, |segment| {
                        segment.preferred_width
                    })
            })
            .collect()
    }

    #[cfg(test)]
    pub(crate) fn commit_column_width(&mut self, chain_index: usize, width: u16) -> bool {
        let Some(segment) = self.chain.get_mut(chain_index) else {
            return false;
        };
        segment.preferred_width = width.clamp(MILLER_COLUMN_MIN_WIDTH, MILLER_COLUMN_MAX_WIDTH);
        self.revision = self.revision.saturating_add(1);
        true
    }

    pub(crate) fn commit_adjacent_column_widths(
        &mut self,
        leading_directory: &Path,
        leading_width: u16,
        trailing: MillerAdjacentWidthTarget,
        trailing_width: u16,
    ) -> bool {
        let Some(leading_chain_index) = self
            .chain
            .iter()
            .position(|segment| segment.directory == leading_directory)
        else {
            return false;
        };
        let trailing_is_live = match trailing {
            MillerAdjacentWidthTarget::Directory(ref directory) => self
                .chain
                .get(leading_chain_index.saturating_add(1))
                .is_some_and(|segment| segment.directory == *directory),
            MillerAdjacentWidthTarget::Preview => self
                .chain
                .get(leading_chain_index)
                .is_some_and(|leading| leading.directory == self.focused_directory),
        };
        if !trailing_is_live {
            return false;
        }
        let leading_width = leading_width.clamp(MILLER_COLUMN_MIN_WIDTH, MILLER_COLUMN_MAX_WIDTH);
        let trailing_min_width = match trailing {
            MillerAdjacentWidthTarget::Directory(_) => MILLER_COLUMN_MIN_WIDTH,
            MillerAdjacentWidthTarget::Preview => MILLER_DETAIL_MIN_WIDTH,
        };
        let trailing_width = trailing_width.clamp(trailing_min_width, MILLER_COLUMN_MAX_WIDTH);
        let Some(leading) = self.chain.get_mut(leading_chain_index) else {
            return false;
        };
        leading.preferred_width = leading_width;
        match trailing {
            MillerAdjacentWidthTarget::Directory(directory) => {
                let Some(trailing) = self
                    .chain
                    .iter_mut()
                    .find(|segment| segment.directory == directory)
                else {
                    return false;
                };
                trailing.preferred_width = trailing_width;
            }
            MillerAdjacentWidthTarget::Preview => {
                self.preview_preferred_width = trailing_width;
            }
        }
        self.revision = self.revision.saturating_add(1);
        true
    }

    #[cfg(test)]
    pub(crate) fn assert_miller_invariants_for_test(&self) {
        assert!(!self.chain.is_empty());
        assert!(self.chain.len() <= MAX_MILLER_HISTORY_DEPTH);
        assert!(self.horizontal.first_visible < self.chain.len());
        assert_eq!(
            self.chain.back().map(|segment| &segment.directory),
            Some(&self.focused_directory)
        );
        assert!(self.chain.iter().all(|segment| {
            (MILLER_COLUMN_MIN_WIDTH..=MILLER_COLUMN_MAX_WIDTH).contains(&segment.preferred_width)
        }));
        assert!((MILLER_COLUMN_MIN_WIDTH..=MILLER_COLUMN_MAX_WIDTH)
            .contains(&self.preview_preferred_width));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trail_layout_history_is_bounded_and_focused() {
        let mut state = MillerState::seed(PathBuf::from("/root"));
        for index in 0..MAX_MILLER_HISTORY_DEPTH + 8 {
            state.visit(PathBuf::from(format!("/root/{index}")));
        }
        state.assert_miller_invariants_for_test();
    }

    #[test]
    fn adjacent_width_commit_is_atomic_and_clamped() {
        let mut state = MillerState::seed(PathBuf::from("/root/a"));
        let before = state.clone();
        assert!(!state.commit_adjacent_column_widths(
            Path::new("/root"),
            40,
            MillerAdjacentWidthTarget::Directory(PathBuf::from("/missing")),
            20,
        ));
        assert_eq!(state, before);
        let last = state.chain.len().saturating_sub(1);
        let focused = state.focused_directory.clone();
        assert!(state.commit_adjacent_column_widths(
            &focused,
            100,
            MillerAdjacentWidthTarget::Preview,
            1,
        ));
        assert_eq!(state.chain[last].preferred_width, MILLER_COLUMN_MAX_WIDTH);
        assert_eq!(state.preview_preferred_width, MILLER_DETAIL_MIN_WIDTH);
    }

    #[test]
    fn trail_sync_preserves_path_widths_and_adopts_selected_directory_columns() {
        let mut state = MillerState::seed(PathBuf::from("/root"));
        state
            .chain
            .iter_mut()
            .find(|segment| segment.directory == Path::new("/root"))
            .expect("seed contains focused directory")
            .preferred_width = 33;
        state.sync_trail_directories(&[PathBuf::from("/root"), PathBuf::from("/root/selected")]);
        assert_eq!(
            state
                .chain
                .iter()
                .map(|segment| (&segment.directory, segment.preferred_width))
                .collect::<Vec<_>>(),
            vec![
                (&PathBuf::from("/root"), 33),
                (
                    &PathBuf::from("/root/selected"),
                    MILLER_COLUMN_PREFERRED_WIDTH
                ),
            ]
        );
        assert_eq!(state.focused_directory, PathBuf::from("/root/selected"));
    }
}
