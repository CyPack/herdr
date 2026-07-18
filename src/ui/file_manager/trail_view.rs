//! Miller trail projection + render (trail program T3): pure geometry over a
//! `TrailState` and its loaded `TrailSnapshots`, per the canonical trail UX
//! contract. Columns run left to right from the trail root; the DEEPEST
//! column is always auto-scrolled into the visible window (LAW 2); widths
//! are per-index (LAW 4); the selected entry stays highlighted in every
//! ancestor column (LAW 1). Render consumes only this snapshot — no
//! filesystem work, no state mutation.

use std::path::PathBuf;

use ratatui::layout::Rect;
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use super::miller::{miller_viewport_geometry, MILLER_DIVIDER_WIDTH};
use crate::app::state::AppState;
use crate::fm::miller::{
    MILLER_COLUMN_MAX_WIDTH, MILLER_COLUMN_MIN_WIDTH, MILLER_COLUMN_PREFERRED_WIDTH,
};
use crate::fm::trail::TrailState;
use crate::fm::trail_snapshots::TrailSnapshots;

/// One clickable row inside one trail column. Input consumes these exact
/// rects, so hit-testing and render share a single geometric source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TrailRowView {
    pub trail_index: usize,
    pub entry_index: usize,
    pub entry_path: PathBuf,
    pub rect: Rect,
}

/// One visible trail column with its bounded row window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TrailColumnView {
    pub trail_index: usize,
    pub directory: PathBuf,
    pub rect: Rect,
    /// Entry index of the trail selection inside this column, when visible
    /// in the loaded listing.
    pub selected_entry: Option<usize>,
    pub viewport_start: usize,
    pub rows: Vec<TrailRowView>,
}

/// Divider between two adjacent visible columns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TrailDividerView {
    pub left_trail_index: usize,
    pub right_trail_index: usize,
    pub rect: Rect,
}

/// Immutable trail frame projection: geometry authority for render and input.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct TrailViewSnapshot {
    pub first_visible: usize,
    pub columns: Vec<TrailColumnView>,
    pub dividers: Vec<TrailDividerView>,
}

/// Effective per-index column width: the caller-provided preference when one
/// exists, the shared default otherwise — always clamped to the frozen
/// min/max bounds (LAW 4).
pub(crate) fn trail_column_width(preferred_widths: &[u16], trail_index: usize) -> u16 {
    preferred_widths
        .get(trail_index)
        .copied()
        .unwrap_or(MILLER_COLUMN_PREFERRED_WIDTH)
        .clamp(MILLER_COLUMN_MIN_WIDTH, MILLER_COLUMN_MAX_WIDTH)
}

/// Project the trail into a bounded visible window. The deepest column is
/// always kept inside the window (auto-scroll right); a trail whose
/// snapshots are missing or misaligned projects NOTHING — stale geometry is
/// inert, never a placeholder.
pub(crate) fn project_trail_view(
    stage: Rect,
    trail: &TrailState,
    snaps: &TrailSnapshots,
    preferred_widths: &[u16],
) -> TrailViewSnapshot {
    let _ = (stage, trail, snaps, preferred_widths);
    TrailViewSnapshot::default()
}

/// Paint the projected trail: rows via the shared entry-row renderer (icons,
/// truncation, selection emphasis) and one-cell dividers between columns.
/// The selected row stays emphasized in EVERY visible column (LAW 1).
pub(crate) fn render_trail_view(
    app: &AppState,
    frame: &mut Frame,
    view: &TrailViewSnapshot,
    snaps: &TrailSnapshots,
) {
    let _ = (app, frame, view, snaps);
    let _ = (MILLER_DIVIDER_WIDTH, miller_viewport_geometry);
    #[allow(clippy::no_effect_underscore_binding)]
    let _unused: Option<Paragraph> = None;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fm::trail::MAX_TRAIL_DEPTH;
    use crate::fm::FmDirectoryStatus;
    use ratatui::backend::TestBackend;
    use ratatui::style::Style;
    use ratatui::Terminal;
    use std::fs;
    use std::path::Path;
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
                "herdr-trailview-test-{}-{}-{}",
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

    /// Build a loaded trail `depth` directories deep under `root`; every
    /// level also carries `extra_files` plain files for row content.
    fn deep_loaded_trail(
        root: &Path,
        depth: usize,
        extra_files: usize,
    ) -> (TrailState, TrailSnapshots) {
        let mut dir = root.to_path_buf();
        for level in 0..depth {
            dir = dir.join(format!("d{level}"));
            fs::create_dir_all(&dir).expect("create level dir");
        }
        let mut fill = root.to_path_buf();
        for level in 0..=depth {
            for file in 0..extra_files {
                fs::write(fill.join(format!("f{file}.txt")), b"x").expect("fill file");
            }
            if level < depth {
                fill = fill.join(format!("d{level}"));
            }
        }
        let mut trail = TrailState::new(root);
        let mut snaps = TrailSnapshots::new(false);
        snaps.sync(&trail);
        let mut next = root.to_path_buf();
        for level in 0..depth {
            next = next.join(format!("d{level}"));
            let deepest = trail.deepest();
            assert_eq!(
                snaps.select_dir(&mut trail, deepest, &next),
                FmDirectoryStatus::Available,
                "fixture descent must load"
            );
        }
        (trail, snaps)
    }

    // LAW 2: the deepest column is auto-scrolled into the visible window —
    // a narrow stage never hides the active end of the trail.
    #[test]
    fn deepest_column_scrolls_into_view() {
        let td = TempDir::new("deepest");
        let (trail, snaps) = deep_loaded_trail(&td.root, 6, 1);
        let stage = Rect::new(0, 0, 70, 12);
        let view = project_trail_view(stage, &trail, &snaps, &[]);
        assert!(
            !view.columns.is_empty(),
            "a loaded trail projects visible columns"
        );
        let deepest = trail.deepest();
        assert!(
            view.columns
                .iter()
                .any(|column| column.trail_index == deepest),
            "the deepest trail column is inside the visible window"
        );
        assert!(
            view.first_visible > 0,
            "a 7-column trail on a 70-cell stage must scroll ancestors left"
        );
    }

    // LAW 4: widths are per-index — each visible column takes its own
    // clamped preference, not one shared width.
    #[test]
    fn per_index_widths_shape_column_rects() {
        let td = TempDir::new("widths");
        let (trail, snaps) = deep_loaded_trail(&td.root, 2, 1);
        let stage = Rect::new(0, 0, 120, 12);
        let preferred = [20u16, 34, 25];
        let view = project_trail_view(stage, &trail, &snaps, &preferred);
        assert_eq!(view.columns.len(), 3, "all three columns fit");
        for column in &view.columns {
            assert_eq!(
                column.rect.width, preferred[column.trail_index],
                "column {} takes its own preferred width",
                column.trail_index
            );
        }
    }

    // Selection visibility: a selected entry deep in a long listing pulls
    // the vertical viewport down until the selected row is visible.
    #[test]
    fn selection_scrolls_vertically_into_view() {
        let td = TempDir::new("vscroll");
        for file in 0..30 {
            fs::write(td.root.join(format!("f{file:02}.txt")), b"x").expect("file");
        }
        let mut trail = TrailState::new(&td.root);
        let mut snaps = TrailSnapshots::new(false);
        snaps.sync(&trail);
        let target = td.root.join("f25.txt");
        assert!(trail.select_file(0, &target));

        let stage = Rect::new(0, 0, 40, 10);
        let view = project_trail_view(stage, &trail, &snaps, &[]);
        let column = &view.columns[0];
        assert_eq!(column.selected_entry, Some(25));
        assert!(
            column
                .rows
                .iter()
                .any(|row| row.entry_index == 25 && row.entry_path == target),
            "the selected row is inside the visible vertical window"
        );
    }

    // Single geometric source: every row rect lives inside its column rect
    // and no two rows overlap — input can trust these rects blindly.
    #[test]
    fn row_rects_stay_within_their_column() {
        let td = TempDir::new("rows");
        let (trail, snaps) = deep_loaded_trail(&td.root, 2, 4);
        let stage = Rect::new(2, 1, 100, 8);
        let view = project_trail_view(stage, &trail, &snaps, &[]);
        assert!(!view.columns.is_empty());
        for column in &view.columns {
            assert!(
                !column.rows.is_empty(),
                "a loaded column projects visible rows"
            );
            let mut seen: Vec<Rect> = Vec::new();
            for row in &column.rows {
                assert_eq!(row.trail_index, column.trail_index);
                assert!(
                    column
                        .rect
                        .contains(ratatui::layout::Position::new(row.rect.x, row.rect.y)),
                    "row origin stays inside its column"
                );
                assert!(row.rect.right() <= column.rect.right());
                assert!(row.rect.bottom() <= column.rect.bottom());
                for prior in &seen {
                    assert_eq!(
                        prior.intersection(row.rect),
                        Rect::default(),
                        "row rects never overlap"
                    );
                }
                seen.push(row.rect);
            }
        }
    }

    // Fail-closed: a trail whose snapshots are misaligned (stale) projects
    // nothing — geometry never invents placeholder columns.
    #[test]
    fn misaligned_snapshots_project_nothing() {
        let td = TempDir::new("stale");
        let (mut trail, snaps) = deep_loaded_trail(&td.root, 2, 1);
        // Rebranch the trail WITHOUT resyncing the snapshots: stale pair.
        let z = td.root.join("d0");
        assert!(trail.select_dir(0, &z));
        let stage = Rect::new(0, 0, 120, 12);
        let view = project_trail_view(stage, &trail, &snaps, &[]);
        assert_eq!(
            view,
            TrailViewSnapshot::default(),
            "stale trail/snapshot pairs are inert"
        );
    }

    // LAW 1: the selected entry keeps its visual emphasis in EVERY visible
    // column, so the whole path reads at a glance.
    #[test]
    fn selected_rows_highlight_in_every_ancestor_column() {
        let td = TempDir::new("highlight");
        let (trail, snaps) = deep_loaded_trail(&td.root, 2, 2);
        let stage = Rect::new(0, 0, 100, 8);
        let view = project_trail_view(stage, &trail, &snaps, &[]);
        assert!(view.columns.len() >= 2);

        let app = crate::app::state::AppState::test_new();
        let backend = TestBackend::new(100, 8);
        let mut terminal = Terminal::new(backend).expect("test terminal");
        terminal
            .draw(|frame| render_trail_view(&app, frame, &view, &snaps))
            .expect("render trail");
        let buffer = terminal.backend().buffer().clone();

        let selected_style = |column: &TrailColumnView| -> Style {
            let selected = column.selected_entry.expect("ancestor has a selection");
            let row = column
                .rows
                .iter()
                .find(|row| row.entry_index == selected)
                .expect("selected row is visible");
            buffer[(row.rect.x, row.rect.y)].style()
        };
        let plain_style = |column: &TrailColumnView| -> Style {
            let selected = column.selected_entry;
            let row = column
                .rows
                .iter()
                .find(|row| Some(row.entry_index) != selected)
                .expect("an unselected row exists");
            buffer[(row.rect.x, row.rect.y)].style()
        };
        for column in view.columns.iter().take(view.columns.len() - 1) {
            assert_ne!(
                selected_style(column),
                plain_style(column),
                "ancestor column {} keeps its selection emphasized",
                column.trail_index
            );
        }
    }

    // Bounded sanity: even an over-deep trail projects only complete
    // columns and never exceeds the stage.
    #[test]
    fn projection_stays_inside_the_stage() {
        let td = TempDir::new("bounds");
        let (trail, snaps) = deep_loaded_trail(&td.root, MAX_TRAIL_DEPTH - 1, 1);
        let stage = Rect::new(4, 2, 90, 10);
        let view = project_trail_view(stage, &trail, &snaps, &[]);
        for column in &view.columns {
            assert!(column.rect.x >= stage.x);
            assert!(column.rect.right() <= stage.right());
            assert!(column.rect.y >= stage.y);
            assert!(column.rect.bottom() <= stage.bottom());
        }
        for divider in &view.dividers {
            assert!(divider.rect.x >= stage.x);
            assert!(divider.rect.right() <= stage.right());
        }
    }
}
