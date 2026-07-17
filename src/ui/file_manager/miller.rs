//! Horizontal Miller viewport geometry (FM1.3): a bounded window of COMPLETE
//! columns plus divider rects over the logical chain, computed purely from
//! the Stage rectangle and per-segment preferred widths. Render and input
//! consume these rects; no filesystem work happens here. FM2 reuses the
//! divider rects as SF3 resize-transaction targets.
#![allow(dead_code)] // P1 production compute owns this snapshot; P2/P3 will
                     // consume every projected render/input identity.

use std::path::PathBuf;

use ratatui::layout::Rect;

use crate::fm::miller::{
    MAX_RESIDENT_MILLER_COLUMNS, MILLER_COLUMN_MAX_WIDTH, MILLER_COLUMN_MIN_WIDTH,
};
use crate::fm::FmState;

pub(crate) const MILLER_DIVIDER_WIDTH: u16 = 1;

/// One complete visible column: the chain index it projects plus its rect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MillerColumnRect {
    pub chain_index: usize,
    pub rect: Rect,
}

/// One divider between two adjacent visible columns. FM2 attaches the SF3
/// resize transaction to exactly these rects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MillerDividerRect {
    pub left_chain_index: usize,
    pub right_chain_index: usize,
    pub rect: Rect,
}

/// Complete horizontal viewport projection for one frame.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct MillerViewportGeometry {
    pub columns: Vec<MillerColumnRect>,
    pub dividers: Vec<MillerDividerRect>,
    /// The clamped first visible chain index actually used. Callers persist
    /// this back so shrink/resize can never leave a stale window.
    pub first_visible: usize,
}

/// One visible logical Miller column projected from prepared model state.
/// Directory identity is copied into the frame snapshot so later render/input
/// wiring never has to infer a path from a raw chain index or coordinate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MillerColumnView {
    pub chain_index: usize,
    pub directory: PathBuf,
    pub rect: Rect,
}

/// One visible divider with the exact adjacent logical directory identities.
/// FM2's transaction target will combine this identity with the snapshot's
/// Files generation; P1 deliberately introduces no input behavior.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MillerDividerView {
    pub left_chain_index: usize,
    pub right_chain_index: usize,
    pub left_directory: PathBuf,
    pub right_directory: PathBuf,
    pub rect: Rect,
}

/// Immutable, bounded Miller projection owned by one computed Files frame.
/// Render and input will consume this snapshot in later phases; P1 first
/// establishes production geometry authority without changing visible output.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct MillerViewSnapshot {
    /// Generation of the active Files singleton that owns this projection.
    /// `None` means no Files instance has authority over these rects.
    pub files_generation: Option<u32>,
    pub model_revision: u64,
    pub focused_chain_index: Option<usize>,
    pub first_visible: usize,
    pub columns: Vec<MillerColumnView>,
    pub dividers: Vec<MillerDividerView>,
}

/// Project prepared Miller model state into a bounded frame snapshot.
///
/// This function performs no filesystem work. At most five directory paths
/// and four adjacent divider pairs are cloned, matching the frozen resident
/// and visible-column bound.
pub(crate) fn project_miller_view(
    stage: Rect,
    file_manager: &FmState,
    files_generation: u32,
) -> MillerViewSnapshot {
    let chain = &file_manager.miller.chain;
    let focused_chain_index = chain
        .iter()
        .position(|segment| segment.directory == file_manager.miller.focused_directory);
    let Some(focused_chain_index) = focused_chain_index else {
        return MillerViewSnapshot {
            files_generation: Some(files_generation),
            model_revision: file_manager.miller.revision,
            ..MillerViewSnapshot::default()
        };
    };
    let preferred_widths = chain
        .iter()
        .map(|segment| segment.preferred_width)
        .collect::<Vec<_>>();
    let geometry = miller_viewport_geometry(
        stage,
        &preferred_widths,
        focused_chain_index,
        file_manager.miller.horizontal.first_visible,
    );

    let mut columns = Vec::with_capacity(geometry.columns.len());
    for column in geometry.columns {
        let Some(segment) = chain.get(column.chain_index) else {
            continue;
        };
        columns.push(MillerColumnView {
            chain_index: column.chain_index,
            directory: segment.directory.clone(),
            rect: column.rect,
        });
    }

    let mut dividers = Vec::with_capacity(geometry.dividers.len());
    for divider in geometry.dividers {
        let (Some(left), Some(right)) = (
            chain.get(divider.left_chain_index),
            chain.get(divider.right_chain_index),
        ) else {
            continue;
        };
        dividers.push(MillerDividerView {
            left_chain_index: divider.left_chain_index,
            right_chain_index: divider.right_chain_index,
            left_directory: left.directory.clone(),
            right_directory: right.directory.clone(),
            rect: divider.rect,
        });
    }

    MillerViewSnapshot {
        files_generation: Some(files_generation),
        model_revision: file_manager.miller.revision,
        focused_chain_index: Some(focused_chain_index),
        first_visible: geometry.first_visible,
        columns,
        dividers,
    }
}

/// Compute the bounded horizontal viewport: starting at `first_visible`
/// (clamped so the FOCUSED chain tail stays reachable and the window never
/// runs past the chain), lay out consecutive COMPLETE columns — each clamped
/// to `MILLER_COLUMN_MIN_WIDTH..=MILLER_COLUMN_MAX_WIDTH` — separated by
/// one-cell dividers, until the Stage width is exhausted or
/// `MAX_RESIDENT_MILLER_COLUMNS` columns are visible. A column that cannot
/// fit COMPLETELY (minimum width) is not shown at all; degenerate stage
/// geometry produces no rect.
pub(crate) fn miller_viewport_geometry(
    stage: Rect,
    preferred_widths: &[u16],
    focused_index: usize,
    requested_first_visible: usize,
) -> MillerViewportGeometry {
    let chain_len = preferred_widths.len();
    if stage.width < MILLER_COLUMN_MIN_WIDTH || stage.height == 0 || chain_len == 0 {
        return MillerViewportGeometry::default();
    }

    // Clamp the window origin: never past the chain, and never so far left
    // that the focused column falls out of the complete-column window.
    let focused_index = focused_index.min(chain_len - 1);
    let floor = first_visible_floor(stage.width, preferred_widths, focused_index);
    let first_visible = requested_first_visible
        .min(chain_len - 1)
        .max(floor)
        .min(focused_index);

    let mut columns = Vec::new();
    let mut dividers = Vec::new();
    let mut x = stage.x;
    let mut remaining = stage.width;
    let mut chain_index = first_visible;
    while chain_index < chain_len && columns.len() < MAX_RESIDENT_MILLER_COLUMNS {
        let preferred =
            preferred_widths[chain_index].clamp(MILLER_COLUMN_MIN_WIDTH, MILLER_COLUMN_MAX_WIDTH);
        let divider_cost = u16::from(!columns.is_empty()) * MILLER_DIVIDER_WIDTH;
        let Some(after_divider) = remaining.checked_sub(divider_cost) else {
            break;
        };
        if after_divider < MILLER_COLUMN_MIN_WIDTH {
            break;
        }
        let width = preferred.min(after_divider);
        if divider_cost > 0 {
            dividers.push(MillerDividerRect {
                left_chain_index: chain_index - 1,
                right_chain_index: chain_index,
                rect: Rect::new(x, stage.y, MILLER_DIVIDER_WIDTH, stage.height),
            });
            x += MILLER_DIVIDER_WIDTH;
        }
        columns.push(MillerColumnRect {
            chain_index,
            rect: Rect::new(x, stage.y, width, stage.height),
        });
        x += width;
        remaining = stage.width - (x - stage.x);
        chain_index += 1;
    }

    MillerViewportGeometry {
        columns,
        dividers,
        first_visible,
    }
}

/// The lowest window origin that still keeps the focused column inside a
/// complete-column window: walk BACKWARD from the focused column, taking
/// each complete clamped column while it fits.
fn first_visible_floor(stage_width: u16, preferred_widths: &[u16], focused_index: usize) -> usize {
    let mut remaining = stage_width;
    let mut start = focused_index;
    let mut count = 0usize;
    let mut index = focused_index as isize;
    while index >= 0 && count < MAX_RESIDENT_MILLER_COLUMNS {
        let preferred = preferred_widths[index as usize]
            .clamp(MILLER_COLUMN_MIN_WIDTH, MILLER_COLUMN_MAX_WIDTH);
        let cost = preferred + u16::from(count > 0) * MILLER_DIVIDER_WIDTH;
        if remaining < cost {
            break;
        }
        remaining -= cost;
        start = index as usize;
        count += 1;
        index -= 1;
    }
    start
}

#[cfg(test)]
mod tests {
    use super::*;

    fn widths(count: usize) -> Vec<u16> {
        vec![crate::fm::miller::MILLER_COLUMN_PREFERRED_WIDTH; count]
    }

    // FM1.3: the nine plan widths — at most five columns, every visible
    // column COMPLETE (>= min width), dividers disjoint one-cell strips,
    // the focused column visible, and every rect inside the Stage.
    #[test]
    fn miller_geometry_holds_across_plan_stage_widths() {
        for stage_width in [0u16, 15, 16, 31, 32, 56, 84, 140, 400] {
            let stage = Rect::new(2, 1, stage_width, 20);
            let geometry = miller_viewport_geometry(stage, &widths(8), 7, 0);

            if stage_width < MILLER_COLUMN_MIN_WIDTH {
                assert_eq!(
                    geometry,
                    MillerViewportGeometry::default(),
                    "width {stage_width}: no complete column can exist"
                );
                continue;
            }
            assert!(
                (1..=MAX_RESIDENT_MILLER_COLUMNS).contains(&geometry.columns.len()),
                "width {stage_width}: bounded non-empty column count"
            );
            for column in &geometry.columns {
                assert!(column.rect.width >= MILLER_COLUMN_MIN_WIDTH);
                assert!(column.rect.width <= MILLER_COLUMN_MAX_WIDTH);
                assert_eq!(column.rect.intersection(stage), column.rect);
            }
            for divider in &geometry.dividers {
                assert_eq!(divider.rect.width, MILLER_DIVIDER_WIDTH);
                assert_eq!(divider.rect.intersection(stage), divider.rect);
            }
            let mut rects: Vec<Rect> = geometry
                .columns
                .iter()
                .map(|column| column.rect)
                .chain(geometry.dividers.iter().map(|divider| divider.rect))
                .collect();
            rects.sort_by_key(|rect| rect.x);
            for pair in rects.windows(2) {
                assert!(
                    pair[0].intersection(pair[1]).is_empty(),
                    "width {stage_width}: rects must be disjoint"
                );
            }
            assert!(
                geometry
                    .columns
                    .iter()
                    .any(|column| column.chain_index == 7),
                "width {stage_width}: the focused column stays visible"
            );
        }
    }

    // FM1.3: shrinking the chain clamps a stale window instead of pointing
    // past the end.
    #[test]
    fn horizontal_viewport_clamps_after_path_shrink() {
        let stage = Rect::new(0, 0, 120, 20);
        let geometry = miller_viewport_geometry(stage, &widths(3), 2, 30);
        assert!(
            geometry.first_visible < 3,
            "a stale window clamps into the chain"
        );
        assert!(geometry.columns.iter().all(|column| column.chain_index < 3));
    }

    // FM1.3: shrinking the terminal clamps the window so the focused column
    // remains reachable.
    #[test]
    fn horizontal_viewport_clamps_after_terminal_resize() {
        let wide = miller_viewport_geometry(Rect::new(0, 0, 200, 20), &widths(6), 5, 0);
        assert!(wide.columns.len() > 1);
        let narrow = miller_viewport_geometry(Rect::new(0, 0, 20, 20), &widths(6), 5, 0);
        assert_eq!(narrow.columns.len(), 1, "one complete column fits");
        assert_eq!(
            narrow.columns[0].chain_index, 5,
            "the focused column wins the narrow window"
        );
    }

    // FM1.3: horizontal scrolling changes ONLY the window origin — column
    // count and stage rects stay bounded and inside the stage.
    #[test]
    fn horizontal_scroll_changes_only_miller_window() {
        let stage = Rect::new(0, 0, 90, 20);
        let narrow = vec![MILLER_COLUMN_MIN_WIDTH; 8];
        let at_zero = miller_viewport_geometry(stage, &narrow, 5, 1);
        let scrolled = miller_viewport_geometry(stage, &narrow, 5, 2);
        assert_ne!(at_zero.first_visible, scrolled.first_visible);
        assert_eq!(at_zero.columns.len(), scrolled.columns.len());
        assert_eq!(
            at_zero.columns[0].rect, scrolled.columns[0].rect,
            "geometry rects are window-independent; only chain indices shift"
        );
    }

    // FM1.3: zero-area stage geometry exposes no column or divider target.
    #[test]
    fn zero_area_clears_column_and_divider_hits() {
        for stage in [Rect::new(0, 0, 0, 20), Rect::new(0, 0, 120, 0), Rect::ZERO] {
            let geometry = miller_viewport_geometry(stage, &widths(4), 3, 0);
            assert!(geometry.columns.is_empty());
            assert!(geometry.dividers.is_empty());
        }
    }
}
