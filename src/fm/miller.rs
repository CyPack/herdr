//! Bounded Miller history and resident-projection model (FM1).
//!
//! The logical path chain and the resident directory cache are BOUNDED by
//! construction: at most `MAX_MILLER_HISTORY_DEPTH` nearest path segments,
//! and at most `MAX_RESIDENT_MILLER_COLUMNS` complete directory projections
//! resident at once (the active current column is never evicted; at most
//! four unique non-current cached projections). Frozen interface source:
//! the FM program plan "Frozen Interfaces and Bounds".
#![allow(dead_code)] // FM1.2 App integration (FmState seeding/visits) and the
                     // FM1.3 horizontal viewport consume this bounded model.

use std::collections::VecDeque;
use std::path::PathBuf;

use super::{FileEntry, FmDirectoryStatus};

pub(crate) const MAX_MILLER_HISTORY_DEPTH: usize = 32;
pub(crate) const MAX_RESIDENT_MILLER_COLUMNS: usize = 5;
pub(crate) const MILLER_COLUMN_MIN_WIDTH: u16 = 16;
pub(crate) const MILLER_COLUMN_PREFERRED_WIDTH: u16 = 28;
pub(crate) const MILLER_COLUMN_MAX_WIDTH: u16 = 64;

/// Exact identity of one projected Miller column: the directory path plus a
/// monotonically increasing generation. A stale generation never resolves.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct MillerColumnId {
    pub directory: PathBuf,
    pub generation: u64,
}

/// One logical path segment in the bounded chain. Segments are cheap
/// bookkeeping (path + cursor + viewport + preferred width), never loaded
/// directory contents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MillerPathSegment {
    pub directory: PathBuf,
    pub focused_child: Option<PathBuf>,
    pub cursor: usize,
    pub viewport_start: usize,
    pub preferred_width: u16,
}

impl MillerPathSegment {
    pub(crate) fn new(directory: PathBuf) -> Self {
        Self {
            directory,
            focused_child: None,
            cursor: 0,
            viewport_start: 0,
            preferred_width: MILLER_COLUMN_PREFERRED_WIDTH,
        }
    }
}

/// One complete cached directory projection for a non-current column.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MillerDirectoryProjection {
    pub id: MillerColumnId,
    pub entries: Vec<FileEntry>,
    pub status: FmDirectoryStatus,
    pub writable: bool,
}

/// Horizontal window over the visible chain suffix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct MillerHorizontalViewport {
    pub first_visible: usize,
}

/// Bounded client-local Miller state. The CURRENT directory's entries stay
/// the operational authority on `FmState`; this model owns only the chain,
/// the evictable non-current cache, and the horizontal window.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MillerState {
    pub chain: VecDeque<MillerPathSegment>,
    pub resident_non_current: VecDeque<MillerDirectoryProjection>,
    pub horizontal: MillerHorizontalViewport,
    pub focused_directory: PathBuf,
    pub revision: u64,
    next_generation: u64,
}

impl MillerState {
    /// Seed the chain from an opening cwd: the cwd plus its nearest
    /// ancestors, WITHOUT canonicalizing through inaccessible or missing
    /// path components, bounded to the nearest
    /// `MAX_MILLER_HISTORY_DEPTH` segments.
    pub(crate) fn seed(cwd: PathBuf) -> Self {
        let mut segments: Vec<MillerPathSegment> = Vec::new();
        segments.push(MillerPathSegment::new(cwd.clone()));
        let mut ancestor = cwd.parent().map(PathBuf::from);
        while let Some(directory) = ancestor {
            segments.push(MillerPathSegment::new(directory.clone()));
            ancestor = directory.parent().map(PathBuf::from);
        }
        // Nearest-first was built child->root; keep the NEAREST segments and
        // restore root->child chain order.
        segments.truncate(MAX_MILLER_HISTORY_DEPTH);
        segments.reverse();

        Self {
            chain: VecDeque::from(segments),
            resident_non_current: VecDeque::new(),
            horizontal: MillerHorizontalViewport::default(),
            focused_directory: cwd,
            revision: 0,
            next_generation: 0,
        }
    }

    /// Record a visible/focused transition into `directory`, caching the
    /// supplied non-current projection under a fresh generation. The chain
    /// stays bounded to the nearest segments around the new focus; the cache
    /// evicts by least-recent visible/focused transition and NEVER holds the
    /// current directory.
    pub(crate) fn visit(
        &mut self,
        directory: PathBuf,
        previous_current: Option<MillerDirectoryProjection>,
    ) {
        if !self
            .chain
            .iter()
            .any(|segment| segment.directory == directory)
        {
            self.chain
                .push_back(MillerPathSegment::new(directory.clone()));
        }
        while self.chain.len() > MAX_MILLER_HISTORY_DEPTH {
            // Keep the NEAREST segments relative to the new focus: drop from
            // the far (root) side, never the focused tail.
            self.chain.pop_front();
        }

        if let Some(projection) = previous_current {
            self.resident_non_current
                .retain(|resident| resident.id.directory != projection.id.directory);
            self.resident_non_current.push_back(projection);
        }
        self.resident_non_current
            .retain(|resident| resident.id.directory != directory);
        while self.resident_non_current.len() >= MAX_RESIDENT_MILLER_COLUMNS {
            self.resident_non_current.pop_front();
        }

        self.focused_directory = directory;
        self.revision = self.revision.saturating_add(1);
    }

    /// Allocate a fresh column generation for a projection about to enter
    /// the cache or the current column.
    pub(crate) fn next_column_id(&mut self, directory: PathBuf) -> MillerColumnId {
        let generation = self.next_generation;
        self.next_generation = self.next_generation.saturating_add(1);
        MillerColumnId {
            directory,
            generation,
        }
    }

    /// Resolve a resident non-current projection by EXACT column identity.
    /// A stale generation (evicted or replaced) resolves to nothing.
    pub(crate) fn resident_projection(
        &self,
        id: &MillerColumnId,
    ) -> Option<&MillerDirectoryProjection> {
        self.resident_non_current
            .iter()
            .find(|projection| projection.id == *id)
    }

    /// Assert every frozen bound and identity invariant (test builds).
    #[cfg(test)]
    pub(crate) fn assert_miller_invariants_for_test(&self) {
        assert!(self.chain.len() <= MAX_MILLER_HISTORY_DEPTH);
        assert!(self.resident_non_current.len() < MAX_RESIDENT_MILLER_COLUMNS);
        assert!(
            self.chain
                .iter()
                .any(|segment| segment.directory == self.focused_directory),
            "the focused directory must be a chain member"
        );
        let mut seen = std::collections::HashSet::new();
        for segment in &self.chain {
            assert!(
                seen.insert(&segment.directory),
                "chain path identity must be unique"
            );
        }
        for projection in &self.resident_non_current {
            assert_ne!(
                projection.id.directory, self.focused_directory,
                "the current directory never sits in the evictable cache"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn deep_path(levels: usize) -> PathBuf {
        let mut path = PathBuf::from("/");
        for level in 0..levels {
            path.push(format!("level-{level:03}"));
        }
        path
    }

    fn projection(state: &mut MillerState, directory: &str) -> MillerDirectoryProjection {
        let directory = PathBuf::from(directory);
        MillerDirectoryProjection {
            id: state.next_column_id(directory),
            entries: Vec::new(),
            status: FmDirectoryStatus::Available,
            writable: true,
        }
    }

    // FM1.1: opening a 100-level cwd retains the NEAREST at most 32
    // segments plus the current directory itself.
    #[test]
    fn miller_history_keeps_nearest_thirty_two_segments() {
        let cwd = deep_path(100);
        let state = MillerState::seed(cwd.clone());
        assert!(state.chain.len() <= MAX_MILLER_HISTORY_DEPTH);
        assert_eq!(
            state.chain.back().map(|segment| segment.directory.clone()),
            Some(cwd.clone()),
            "the current directory is the chain tail"
        );
        assert_eq!(
            state.chain.front().map(|segment| segment.directory.clone()),
            Some(deep_path(100 - MAX_MILLER_HISTORY_DEPTH + 1)),
            "the retained ancestors are the NEAREST ones"
        );
        state.assert_miller_invariants_for_test();
    }

    // FM1.1: chain trimming never drops the focused current segment.
    #[test]
    fn miller_history_never_drops_focused_current_segment() {
        let mut state = MillerState::seed(deep_path(31));
        for extra in 0..40 {
            let directory = deep_path(31).join(format!("extra-{extra:02}"));
            state.visit(directory.clone(), None);
            assert_eq!(state.focused_directory, directory);
            assert!(
                state
                    .chain
                    .iter()
                    .any(|segment| segment.directory == directory),
                "trimming must never drop the focused segment"
            );
            state.assert_miller_invariants_for_test();
        }
    }

    // FM1.1: at most five complete directory projections are resident at
    // once — the current plus at most four unique non-current entries.
    #[test]
    fn resident_cache_plus_current_keeps_at_most_five_directories() {
        let mut state = MillerState::seed(PathBuf::from("/tmp/miller"));
        for step in 0..20 {
            let previous = projection(&mut state, &format!("/tmp/miller/dir-{step:02}"));
            state.visit(
                PathBuf::from(format!("/tmp/miller/dir-{:02}", step + 1)),
                Some(previous),
            );
            assert!(
                state.resident_non_current.len() < MAX_RESIDENT_MILLER_COLUMNS,
                "at most four unique non-current projections may be resident"
            );
        }
        state.assert_miller_invariants_for_test();
    }

    // FM1.1: eviction invalidates the evicted column's exact generation.
    #[test]
    fn resident_eviction_invalidates_old_generation() {
        let mut state = MillerState::seed(PathBuf::from("/tmp/miller"));
        let first = projection(&mut state, "/tmp/miller/first");
        let first_id = first.id.clone();
        state.visit(PathBuf::from("/tmp/miller/next-0"), Some(first));
        assert!(
            state.resident_projection(&first_id).is_some(),
            "fixture: the projection is resident before eviction"
        );

        for step in 0..MAX_RESIDENT_MILLER_COLUMNS {
            let previous = projection(&mut state, &format!("/tmp/miller/next-{step}"));
            state.visit(
                PathBuf::from(format!("/tmp/miller/next-{}", step + 1)),
                Some(previous),
            );
        }
        assert!(
            state.resident_projection(&first_id).is_none(),
            "an evicted generation must never resolve"
        );
        state.assert_miller_invariants_for_test();
    }

    // FM1.1: the current directory's projection is operational authority on
    // `FmState`, never a member of the evictable cache.
    #[test]
    fn current_projection_is_separate_from_evictable_cache() {
        let mut state = MillerState::seed(PathBuf::from("/tmp/miller"));
        let stale_current = projection(&mut state, "/tmp/miller/target");
        state.visit(PathBuf::from("/tmp/miller/target"), Some(stale_current));
        assert!(
            state
                .resident_non_current
                .iter()
                .all(|resident| resident.id.directory != state.focused_directory),
            "the current directory never sits in the evictable cache"
        );
        state.assert_miller_invariants_for_test();
    }

    // FM1.1: close/reopen rebuilds fresh state — new revision baseline and
    // no leaked cache or generations from the previous Files lifecycle.
    #[test]
    fn close_reopen_rebuilds_fresh_miller_state() {
        let mut state = MillerState::seed(PathBuf::from("/tmp/miller"));
        for step in 0..6 {
            let previous = projection(&mut state, &format!("/tmp/miller/dir-{step}"));
            state.visit(
                PathBuf::from(format!("/tmp/miller/dir-{}", step + 1)),
                Some(previous),
            );
        }
        let old_id = state.next_column_id(PathBuf::from("/tmp/miller/dir-1"));
        assert!(old_id.generation > 0);

        let reopened = MillerState::seed(PathBuf::from("/tmp/miller"));
        assert_eq!(reopened.revision, 0);
        assert!(reopened.resident_non_current.is_empty());
        assert!(
            reopened.resident_projection(&old_id).is_none(),
            "a previous lifecycle's generation must not resolve after reopen"
        );
        reopened.assert_miller_invariants_for_test();
    }
}
