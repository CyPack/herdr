//! Miller trail projection + render (trail program T3): pure geometry over a
//! `TrailState` and its loaded `TrailSnapshots`, per the canonical trail UX
//! contract. Columns run left to right from the trail root; the DEEPEST
//! column is always auto-scrolled into the visible window (LAW 2); widths
//! are per-index (LAW 4); the selected entry stays highlighted in every
//! ancestor column (LAW 1). Render consumes only this snapshot — no
//! filesystem work, no state mutation.

use std::path::PathBuf;

use ratatui::layout::Rect;
use ratatui::widgets::{Block, Borders};
use ratatui::Frame;

use super::miller::miller_viewport_geometry;
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
    let trail_cols = trail.cols();
    let snap_cols = snaps.cols();
    let aligned = trail_cols.len() == snap_cols.len()
        && trail_cols
            .iter()
            .zip(snap_cols.iter())
            .all(|(col, snap)| snap.directory() == col.directory.as_path());
    if trail_cols.is_empty() || !aligned {
        return TrailViewSnapshot::default();
    }

    let widths: Vec<u16> = (0..trail_cols.len())
        .map(|index| trail_column_width(preferred_widths, index))
        .collect();
    // Requesting origin 0 clamps up to the floor that keeps the DEEPEST
    // column inside a complete-column window: auto-scroll right (LAW 2).
    let geometry = miller_viewport_geometry(stage, &widths, trail.deepest(), 0);

    let columns = geometry
        .columns
        .iter()
        .map(|column| {
            let trail_index = column.chain_index;
            let entries = snap_cols[trail_index].entries();
            let selected_entry = trail_cols[trail_index]
                .selected
                .as_deref()
                .and_then(|selected| entries.iter().position(|entry| entry.path == selected));
            let height = usize::from(column.rect.height);
            let viewport_start = selected_entry
                .filter(|&selected| height > 0 && selected >= height)
                .map(|selected| selected + 1 - height)
                .unwrap_or(0);
            let rows = entries
                .iter()
                .enumerate()
                .skip(viewport_start)
                .take(height)
                .map(|(entry_index, entry)| TrailRowView {
                    trail_index,
                    entry_index,
                    entry_path: entry.path.clone(),
                    rect: Rect::new(
                        column.rect.x,
                        column.rect.y + (entry_index - viewport_start) as u16,
                        column.rect.width,
                        1,
                    ),
                })
                .collect();
            TrailColumnView {
                trail_index,
                directory: trail_cols[trail_index].directory.clone(),
                rect: column.rect,
                selected_entry,
                viewport_start,
                rows,
            }
        })
        .collect();
    let dividers = geometry
        .dividers
        .iter()
        .map(|divider| TrailDividerView {
            left_trail_index: divider.left_chain_index,
            right_trail_index: divider.right_chain_index,
            rect: divider.rect,
        })
        .collect();

    TrailViewSnapshot {
        first_visible: geometry.first_visible,
        columns,
        dividers,
    }
}

/// Resolve one screen position against this exact projected frame. The row
/// rects ARE the hit areas — input never recomputes geometry. Positions on
/// dividers, empty column space, or outside the projection resolve to None.
pub(crate) fn trail_row_at(view: &TrailViewSnapshot, x: u16, y: u16) -> Option<&TrailRowView> {
    let position = ratatui::layout::Position::new(x, y);
    view.columns
        .iter()
        .flat_map(|column| column.rows.iter())
        .find(|row| row.rect.contains(position))
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
    let styles = super::file_manager_visual_styles(&app.palette);
    for divider in &view.dividers {
        frame.render_widget(
            Block::default()
                .borders(Borders::LEFT)
                .border_style(styles.divider),
            divider.rect,
        );
    }
    for column in &view.columns {
        let Some(snap) = snaps.cols().get(column.trail_index) else {
            continue;
        };
        for row in &column.rows {
            let Some(entry) = snap.entries().get(row.entry_index) else {
                continue;
            };
            let selected = column.selected_entry == Some(row.entry_index);
            super::render_entry_row(app, frame, row.rect, entry, selected, false);
        }
    }
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
                    assert!(
                        prior.intersection(row.rect).is_empty(),
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

    // Hit resolution: a position inside a row rect resolves to EXACTLY that
    // row — the projection is the single hit authority.
    #[test]
    fn row_hit_resolves_exact_row() {
        let td = TempDir::new("hit");
        let (trail, snaps) = deep_loaded_trail(&td.root, 2, 3);
        let stage = Rect::new(3, 2, 100, 8);
        let view = project_trail_view(stage, &trail, &snaps, &[]);
        for column in &view.columns {
            for row in &column.rows {
                let hit = trail_row_at(&view, row.rect.x, row.rect.y)
                    .expect("a row rect position resolves");
                assert_eq!(hit, row, "hit resolves to exactly the visible row");
                let right_edge = row.rect.right() - 1;
                let hit = trail_row_at(&view, right_edge, row.rect.y)
                    .expect("the row's last cell also resolves");
                assert_eq!(hit, row);
            }
        }
    }

    // Hit resolution: dividers, empty column space below the listing, and
    // positions outside the stage resolve to nothing.
    #[test]
    fn hit_outside_rows_is_none() {
        let td = TempDir::new("hit-none");
        let (trail, snaps) = deep_loaded_trail(&td.root, 1, 1);
        let stage = Rect::new(0, 0, 100, 10);
        let view = project_trail_view(stage, &trail, &snaps, &[]);
        assert!(!view.columns.is_empty());
        for divider in &view.dividers {
            assert!(
                trail_row_at(&view, divider.rect.x, divider.rect.y).is_none(),
                "divider cells never resolve to a row"
            );
        }
        let column = &view.columns[0];
        let below = column.rect.y + column.rows.len() as u16;
        assert!(below < column.rect.bottom(), "fixture has empty space");
        assert!(
            trail_row_at(&view, column.rect.x, below).is_none(),
            "empty column space resolves to nothing"
        );
        assert!(
            trail_row_at(&view, stage.right().saturating_sub(1), stage.bottom() - 1).is_none(),
            "outside every projected column resolves to nothing"
        );
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
