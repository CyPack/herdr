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
use std::path::{Path, PathBuf};

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

/// Exact non-current column authority for one vertical wheel transition.
/// Input obtains this only after generation-safe row resolution; the model
/// revalidates the identity again before changing bounded presentation state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MillerColumnScrollTarget {
    Resident {
        chain_index: usize,
        directory: PathBuf,
        generation: u64,
    },
    PreparedParent {
        chain_index: usize,
        directory: PathBuf,
        generation: u64,
    },
    Preview {
        directory: PathBuf,
        generation: u64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MillerAdjacentWidthTarget {
    Chain(usize),
    Preview,
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
    pub preview_preferred_width: u16,
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
            preview_preferred_width: MILLER_COLUMN_PREFERRED_WIDTH,
            revision: 0,
            next_generation: 0,
        }
    }

    /// Record a visible/focused transition into `directory`, caching the
    /// supplied non-current projection under a fresh generation. The chain
    /// stays bounded to the nearest segments around the new focus; the cache
    /// evicts by least-recent visible/focused transition and NEVER holds the
    /// current directory.
    /// Bind the exact child path a navigation departed through to its source
    /// segment. `cursor` stays a derived cache value; this path identity is
    /// the resident-selection authority (TP-FIP-FOCUS-01).
    pub(crate) fn bind_focused_child(&mut self, directory: &Path, child: &Path) {
        if let Some(segment) = self
            .chain
            .iter_mut()
            .find(|segment| segment.directory == directory)
        {
            segment.focused_child = Some(child.to_path_buf());
        }
    }

    pub(crate) fn visit(
        &mut self,
        directory: PathBuf,
        previous_current: Option<MillerDirectoryProjection>,
    ) {
        let mut retired_directories = Vec::new();
        if let Some(existing_index) = self
            .chain
            .iter()
            .position(|segment| segment.directory == directory)
        {
            retired_directories.extend(
                self.chain
                    .iter()
                    .skip(existing_index.saturating_add(1))
                    .map(|segment| segment.directory.clone()),
            );
            self.chain.truncate(existing_index.saturating_add(1));
        } else {
            self.chain
                .push_back(MillerPathSegment::new(directory.clone()));
        }
        while self.chain.len() > MAX_MILLER_HISTORY_DEPTH {
            // Keep the NEAREST segments relative to the new focus: drop from
            // the far (root) side, never the focused tail.
            self.chain.pop_front();
        }
        self.horizontal.first_visible = self
            .horizontal
            .first_visible
            .min(self.chain.len().saturating_sub(1));

        // Branch retirement is atomic with the chain transition: no cached
        // projection may outlive the segment that gave it authority.
        self.resident_non_current.retain(|resident| {
            resident.id.directory != directory
                && !retired_directories.contains(&resident.id.directory)
        });

        if let Some(projection) = previous_current.filter(|projection| {
            projection.id.directory != directory
                && !retired_directories.contains(&projection.id.directory)
        }) {
            self.resident_non_current
                .retain(|resident| resident.id.directory != projection.id.directory);
            self.resident_non_current.push_back(projection);
        }
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

    /// Apply a committed divider resize to one chain column's preferred
    /// width, clamped to the frozen `MILLER_COLUMN_MIN_WIDTH..=MAX` bounds
    /// (FM2.1). The SF3 `ResizeTransaction` supplies the committed leading
    /// track; this is the single write-back seam, so pointer and keyboard
    /// resize can never drift. Returns false for a stale chain index.
    pub(crate) fn commit_column_width(&mut self, chain_index: usize, width: u16) -> bool {
        let Some(segment) = self.chain.get_mut(chain_index) else {
            return false;
        };
        segment.preferred_width = width.clamp(MILLER_COLUMN_MIN_WIDTH, MILLER_COLUMN_MAX_WIDTH);
        self.revision = self.revision.saturating_add(1);
        true
    }

    /// Atomically commit the two adjacent tracks of one typed Miller divider.
    /// Both identities are validated before either preference changes, and
    /// the model revision advances exactly once for the complete gesture.
    pub(crate) fn commit_adjacent_column_widths(
        &mut self,
        leading_chain_index: usize,
        leading_width: u16,
        trailing: MillerAdjacentWidthTarget,
        trailing_width: u16,
    ) -> bool {
        let Some(leading) = self.chain.get(leading_chain_index) else {
            return false;
        };
        let trailing_is_live = match trailing {
            MillerAdjacentWidthTarget::Chain(trailing_chain_index) => {
                leading_chain_index.saturating_add(1) == trailing_chain_index
                    && self.chain.get(trailing_chain_index).is_some()
            }
            MillerAdjacentWidthTarget::Preview => leading.directory == self.focused_directory,
        };
        if !trailing_is_live {
            return false;
        }

        let leading_width = leading_width.clamp(MILLER_COLUMN_MIN_WIDTH, MILLER_COLUMN_MAX_WIDTH);
        let trailing_width = trailing_width.clamp(MILLER_COLUMN_MIN_WIDTH, MILLER_COLUMN_MAX_WIDTH);
        let Some(leading) = self.chain.get_mut(leading_chain_index) else {
            return false;
        };
        leading.preferred_width = leading_width;
        match trailing {
            MillerAdjacentWidthTarget::Chain(trailing_chain_index) => {
                let Some(trailing) = self.chain.get_mut(trailing_chain_index) else {
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

    /// Resolve the single live non-current projection for a directory.
    ///
    /// `visit` removes an older projection for the same path before inserting
    /// a new generation, so a path lookup is unambiguous. Render/projection
    /// callers use this prepared cache only; they never load the filesystem.
    pub(crate) fn resident_projection_for_directory(
        &self,
        directory: &Path,
    ) -> Option<&MillerDirectoryProjection> {
        self.resident_non_current
            .iter()
            .find(|projection| projection.id.directory == directory)
    }

    /// Assert every frozen bound and identity invariant (test builds).
    #[cfg(test)]
    pub(crate) fn assert_miller_invariants_for_test(&self) {
        assert!(self.chain.len() <= MAX_MILLER_HISTORY_DEPTH);
        assert!(self.resident_non_current.len() < MAX_RESIDENT_MILLER_COLUMNS);
        assert!(
            self.horizontal.first_visible < self.chain.len(),
            "the horizontal window must address a live chain segment"
        );
        assert!(
            self.chain
                .iter()
                .any(|segment| segment.directory == self.focused_directory),
            "the focused directory must be a chain member"
        );
        assert_eq!(
            self.chain.back().map(|segment| &segment.directory),
            Some(&self.focused_directory),
            "the focused current directory must be the chain tail"
        );
        assert!(
            (MILLER_COLUMN_MIN_WIDTH..=MILLER_COLUMN_MAX_WIDTH)
                .contains(&self.preview_preferred_width),
            "the inline preview preference stays inside frozen bounds"
        );
        let mut seen = std::collections::HashSet::new();
        for segment in &self.chain {
            assert!(
                seen.insert(&segment.directory),
                "chain path identity must be unique"
            );
            assert!(
                (MILLER_COLUMN_MIN_WIDTH..=MILLER_COLUMN_MAX_WIDTH)
                    .contains(&segment.preferred_width),
                "every chain width must stay inside frozen bounds"
            );
        }
        let mut resident_directories = std::collections::HashSet::new();
        let mut resident_ids = std::collections::HashSet::new();
        for projection in &self.resident_non_current {
            assert_ne!(
                projection.id.directory, self.focused_directory,
                "the current directory never sits in the evictable cache"
            );
            assert!(
                resident_directories.insert(&projection.id.directory),
                "only one resident projection may own a directory"
            );
            assert!(
                resident_ids.insert(&projection.id),
                "resident column identity must be unique"
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
    fn history_eviction_preserves_focused_visibility() {
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
            assert_eq!(
                state.chain.back().map(|segment| &segment.directory),
                Some(&directory),
                "the focused segment remains the visible chain tail"
            );
            state.assert_miller_invariants_for_test();
        }
    }

    // FM1.1: at most five complete directory projections are resident at
    // once — the current plus at most four unique non-current entries.
    #[test]
    fn resident_projection_lru_never_exceeds_five() {
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
        assert_eq!(
            state
                .resident_non_current
                .iter()
                .map(|projection| projection.id.directory.clone())
                .collect::<Vec<_>>(),
            (16..20)
                .map(|step| PathBuf::from(format!("/tmp/miller/dir-{step:02}")))
                .collect::<Vec<_>>(),
            "the four most recently departed projections survive in LRU order"
        );
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

    // TP-FM4-BRANCH-TRUNCATE: revisiting an ancestor is one atomic branch
    // transition. Descendant segments and every resident generation owned by
    // that retired tail disappear before a sibling branch can be appended.
    #[test]
    fn revisiting_ancestor_truncates_descendants_and_retires_projections() {
        let ancestor = PathBuf::from("/virtual/root/ancestor");
        let child = ancestor.join("child");
        let grandchild = child.join("grandchild");
        let mut state = MillerState::seed(ancestor.clone());

        let ancestor_projection = projection(&mut state, "/virtual/root/ancestor");
        let ancestor_id = ancestor_projection.id.clone();
        state.visit(child.clone(), Some(ancestor_projection));
        let child_projection = projection(&mut state, "/virtual/root/ancestor/child");
        let child_id = child_projection.id.clone();
        state.visit(grandchild.clone(), Some(child_projection));
        let grandchild_projection =
            projection(&mut state, "/virtual/root/ancestor/child/grandchild");

        assert!(state.resident_projection(&ancestor_id).is_some());
        assert!(state.resident_projection(&child_id).is_some());
        state.visit(ancestor.clone(), Some(grandchild_projection));

        assert_eq!(
            state.chain.back().map(|segment| &segment.directory),
            Some(&ancestor),
            "the revisited ancestor becomes the chain tail"
        );
        assert!(
            state
                .chain
                .iter()
                .all(|segment| segment.directory != child && segment.directory != grandchild),
            "the retired branch cannot remain addressable"
        );
        assert!(
            state.resident_projection(&child_id).is_none(),
            "a retired descendant generation cannot resolve"
        );
        assert_eq!(state.focused_directory, ancestor);
        state.assert_miller_invariants_for_test();
    }

    // P5 BRANCH ATOMICITY: choosing a sibling is a two-step
    // ancestor-then-child transition. The ancestor step must retire the old
    // branch's segments, generations, and horizontal window authority
    // together; projection must not be required to repair stale model state
    // on a later frame.
    #[test]
    fn selecting_sibling_truncates_old_branch_atomically() {
        let ancestor = PathBuf::from("/virtual/root/ancestor");
        let old_child = ancestor.join("old-child");
        let old_grandchild = old_child.join("grandchild");
        let sibling = ancestor.join("sibling");
        let mut state = MillerState::seed(ancestor.clone());

        let ancestor_projection = projection(&mut state, "/virtual/root/ancestor");
        state.visit(old_child.clone(), Some(ancestor_projection));
        let old_child_projection = projection(&mut state, "/virtual/root/ancestor/old-child");
        let old_child_id = old_child_projection.id.clone();
        state.visit(old_grandchild.clone(), Some(old_child_projection));
        state.horizontal.first_visible = state.chain.len().saturating_sub(1);

        let old_grandchild_projection =
            projection(&mut state, "/virtual/root/ancestor/old-child/grandchild");
        state.visit(ancestor.clone(), Some(old_grandchild_projection));

        assert_eq!(
            state.chain.back().map(|segment| &segment.directory),
            Some(&ancestor)
        );
        assert!(
            state.chain.iter().all(
                |segment| segment.directory != old_child && segment.directory != old_grandchild
            ),
            "the old branch tail must be retired in the ancestor transition"
        );
        assert!(
            state.resident_projection(&old_child_id).is_none(),
            "the old branch generation must retire with its segment"
        );
        assert!(
            state.horizontal.first_visible < state.chain.len(),
            "the horizontal window must be live before the next frame"
        );

        let ancestor_projection = projection(&mut state, "/virtual/root/ancestor");
        state.visit(sibling.clone(), Some(ancestor_projection));
        assert_eq!(state.focused_directory, sibling);
        state.assert_miller_invariants_for_test();
    }

    // FM1.1: the current directory's projection is operational authority on
    // `FmState`, never a member of the evictable cache.
    #[test]
    fn current_segment_is_never_evicted() {
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

    // FM2.1: a committed divider resize writes back through ONE clamped
    // seam — widths clamp to the frozen 16..=64 bounds, the revision
    // advances (geometry recomputes), and a stale chain index is refused.
    #[test]
    fn column_resize_commits_clamped_width_through_single_seam() {
        let mut state = MillerState::seed(deep_path(4));
        let before = state.revision;

        assert!(state.commit_column_width(1, 200));
        assert_eq!(state.chain[1].preferred_width, MILLER_COLUMN_MAX_WIDTH);
        assert!(state.commit_column_width(1, 2));
        assert_eq!(state.chain[1].preferred_width, MILLER_COLUMN_MIN_WIDTH);
        assert!(state.commit_column_width(1, 40));
        assert_eq!(state.chain[1].preferred_width, 40);
        assert_eq!(
            state.revision,
            before + 3,
            "every commit recomputes geometry"
        );

        assert!(
            !state.commit_column_width(99, 40),
            "a stale chain index must be refused"
        );
        assert_eq!(state.revision, before + 3);
        state.assert_miller_invariants_for_test();
    }

    #[test]
    fn adjacent_column_resize_commits_atomically_once() {
        let mut state = MillerState::seed(deep_path(4));
        let before = state.clone();

        assert!(
            !state.commit_adjacent_column_widths(1, 40, MillerAdjacentWidthTarget::Chain(3), 20,),
            "non-adjacent chain identities are stale"
        );
        assert_eq!(state, before, "failed validation is mutation-free");

        assert!(state.commit_adjacent_column_widths(
            1,
            40,
            MillerAdjacentWidthTarget::Chain(2),
            20,
        ));
        assert_eq!(
            (
                state.chain[1].preferred_width,
                state.chain[2].preferred_width,
                state.revision,
            ),
            (40, 20, before.revision + 1),
            "one adjacent pair advances one revision"
        );

        let focused_chain_index = state.chain.len() - 1;
        assert!(state.commit_adjacent_column_widths(
            focused_chain_index,
            50,
            MillerAdjacentWidthTarget::Preview,
            2,
        ));
        assert_eq!(
            (
                state.chain[focused_chain_index].preferred_width,
                state.preview_preferred_width,
                state.revision,
            ),
            (50, MILLER_COLUMN_MIN_WIDTH, before.revision + 2),
            "focused/preview commit clamps both tracks and advances once"
        );

        let before_stale_preview = state.clone();
        assert!(
            !state.commit_adjacent_column_widths(0, 32, MillerAdjacentWidthTarget::Preview, 24,),
            "a non-focused chain column cannot authorize inline preview width"
        );
        assert_eq!(
            state, before_stale_preview,
            "stale preview identity is mutation-free"
        );
        state.assert_miller_invariants_for_test();
    }

    // FM1.1: close/reopen rebuilds fresh state — new revision baseline and
    // no leaked cache or generations from the previous Files lifecycle.
    #[test]
    fn close_reopen_resets_ephemeral_width_and_view_state() {
        let mut state = MillerState::seed(PathBuf::from("/tmp/miller"));
        for step in 0..6 {
            let previous = projection(&mut state, &format!("/tmp/miller/dir-{step}"));
            state.visit(
                PathBuf::from(format!("/tmp/miller/dir-{}", step + 1)),
                Some(previous),
            );
        }
        let focused_chain_index = state.chain.len().saturating_sub(1);
        assert!(state.commit_adjacent_column_widths(
            focused_chain_index,
            51,
            MillerAdjacentWidthTarget::Preview,
            37,
        ));
        state.horizontal.first_visible = focused_chain_index;
        let old_id = state.next_column_id(PathBuf::from("/tmp/miller/dir-1"));
        assert!(old_id.generation > 0);

        let reopened = MillerState::seed(PathBuf::from("/tmp/miller"));
        assert_eq!(reopened.revision, 0);
        assert!(reopened.resident_non_current.is_empty());
        assert_eq!(reopened.horizontal.first_visible, 0);
        assert_eq!(
            reopened.preview_preferred_width,
            MILLER_COLUMN_PREFERRED_WIDTH
        );
        assert!(
            reopened
                .chain
                .iter()
                .all(|segment| segment.preferred_width == MILLER_COLUMN_PREFERRED_WIDTH),
            "column preferences are client-local to one Files lifecycle"
        );
        assert!(
            reopened.resident_projection(&old_id).is_none(),
            "a previous lifecycle's generation must not resolve after reopen"
        );
        reopened.assert_miller_invariants_for_test();
    }
}

#[cfg(test)]
mod integration_tests {
    use crate::fm::FmState;

    // FM1.2 integration: real enter/leave navigation seeds, visits, and
    // caches through the bounded model — the departed directory's complete
    // projection moves into the cache (ownership transfer) and the focused
    // directory always tracks the live cwd.
    #[test]
    fn navigation_visits_keep_bounded_state_and_cache_departed_projection() {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "herdr-miller-integration-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let child = root.join("child");
        std::fs::create_dir_all(&child).expect("create integration tree");
        std::fs::write(root.join("aaa-marker.txt"), b"x").expect("root marker");
        std::fs::write(child.join("inner.txt"), b"x").expect("child marker");

        let mut state = FmState::new(&root);
        assert_eq!(state.miller.focused_directory, root);
        state.miller.assert_miller_invariants_for_test();

        let child_index = state
            .entries
            .iter()
            .position(|entry| entry.path == child)
            .expect("child entry visible");
        state.cursor = child_index;
        state.enter();
        assert_eq!(state.cwd, child);
        assert_eq!(state.miller.focused_directory, child);
        let cached_root = state
            .miller
            .resident_non_current
            .iter()
            .find(|projection| projection.id.directory == root)
            .expect("the departed root projection is cached");
        assert!(
            cached_root
                .entries
                .iter()
                .any(|entry| entry.path == root.join("aaa-marker.txt")),
            "the cached projection carries the moved complete entry vector"
        );
        state.miller.assert_miller_invariants_for_test();

        state.leave();
        assert_eq!(state.cwd, root);
        assert_eq!(state.miller.focused_directory, root);
        assert!(
            state
                .miller
                .resident_non_current
                .iter()
                .all(|projection| projection.id.directory != root),
            "returning to a directory removes it from the evictable cache"
        );
        state.miller.assert_miller_invariants_for_test();

        let _ = std::fs::remove_dir_all(&root);
    }
}
