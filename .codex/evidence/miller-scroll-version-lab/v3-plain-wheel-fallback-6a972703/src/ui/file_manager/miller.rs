//! Pure bounded viewport geometry and typed resize projection for the Trail.
//!
//! Navigation, rows, selection, and directory contents are owned exclusively
//! by `TrailState`/`TrailSnapshots`. This module keeps the proven complete
//! column geometry used by the Trail renderer and exposes only the minimal
//! immutable layout snapshot needed by resize transactions.

use std::path::PathBuf;

use ratatui::layout::Rect;

use crate::fm::miller::{MILLER_COLUMN_MAX_WIDTH, MILLER_COLUMN_MIN_WIDTH};
use crate::fm::FmState;

pub(crate) const MILLER_DIVIDER_WIDTH: u16 = 1;
const MAX_VISIBLE_TRAIL_COLUMNS: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MillerColumnRect {
    pub chain_index: usize,
    pub rect: Rect,
    pub logical_width: u16,
    pub source_x: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MillerDividerRect {
    pub left_chain_index: usize,
    pub right_chain_index: usize,
    pub rect: Rect,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct MillerViewportGeometry {
    pub columns: Vec<MillerColumnRect>,
    pub dividers: Vec<MillerDividerRect>,
    pub offset_cells: u32,
    pub max_offset_cells: u32,
    pub first_visible: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MillerColumnKind {
    Trail {
        trail_index: usize,
        directory: PathBuf,
        generation: u64,
        current: bool,
    },
    Detail {
        parent_trail_index: usize,
        source_path: PathBuf,
        generation: u64,
    },
}

impl MillerColumnKind {
    #[cfg(test)]
    pub(crate) fn chain_index(&self) -> Option<usize> {
        match self {
            Self::Trail { trail_index, .. } => Some(*trail_index),
            Self::Detail { .. } => None,
        }
    }

    pub(crate) fn is_directory(&self, path: &std::path::Path) -> bool {
        matches!(self, Self::Trail { directory, .. } if directory == path)
    }

    #[cfg(test)]
    pub(crate) fn is_current(&self) -> bool {
        matches!(self, Self::Trail { current: true, .. })
    }

    #[cfg(test)]
    pub(crate) fn is_preview(&self) -> bool {
        matches!(self, Self::Detail { .. })
    }
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MillerRowView {
    pub entry_path: PathBuf,
    pub rect: Rect,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MillerColumnView {
    pub projection_index: usize,
    pub kind: MillerColumnKind,
    pub rect: Rect,
    pub content_rect: Rect,
    #[cfg(test)]
    pub rows: Vec<MillerRowView>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MillerDividerView {
    pub left_column: usize,
    pub right_column: usize,
    pub rect: Rect,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct MillerViewSnapshot {
    pub files_generation: Option<u32>,
    pub model_revision: u64,
    pub resize_preview_active: bool,
    pub focused_chain_index: Option<usize>,
    pub first_visible_min: usize,
    pub first_visible_max: usize,
    pub first_visible: usize,
    pub horizontal_offset_cells: u32,
    pub horizontal_max_offset_cells: u32,
    pub columns: Vec<MillerColumnView>,
    pub dividers: Vec<MillerDividerView>,
}

#[cfg(test)]
pub(crate) fn project_miller_view(
    stage: Rect,
    file_manager: &FmState,
    files_generation: u32,
) -> MillerViewSnapshot {
    project_miller_view_with_resize_preview(stage, file_manager, files_generation, None)
}

pub(crate) fn project_miller_view_with_resize_preview(
    stage: Rect,
    file_manager: &FmState,
    files_generation: u32,
    resize_preview: Option<(&crate::ui::shell::MillerDividerId, [u16; 2])>,
) -> MillerViewSnapshot {
    let trail_cols = file_manager.trail.cols();
    if trail_cols.is_empty() {
        return MillerViewSnapshot::default();
    }
    let mut preferred_widths = file_manager
        .miller
        .preferred_widths_for(trail_cols.iter().map(|column| column.directory.clone()));
    let detail = file_manager.trail_snapshots.detail();
    let mut detail_preferred_width = file_manager.miller.preview_preferred_width;
    let resize_preview_active = apply_resize_preview_widths(
        &mut preferred_widths,
        &mut detail_preferred_width,
        file_manager,
        files_generation,
        resize_preview,
    );
    let detail_width = (detail.is_some()
        && stage.width >= super::trail_view::TRAIL_DETAIL_PANEL_MIN_WIDTH.saturating_mul(2))
    .then(|| {
        detail_preferred_width
            .clamp(
                super::trail_view::TRAIL_DETAIL_PANEL_MIN_WIDTH,
                MILLER_COLUMN_MAX_WIDTH,
            )
            .min(stage.width / 2)
    })
    .filter(|width| stage.width >= width.saturating_add(MILLER_COLUMN_MIN_WIDTH));
    let column_stage = detail_width.map_or(stage, |width| {
        Rect::new(stage.x, stage.y, stage.width - width, stage.height)
    });
    let focused_index = file_manager.trail.deepest();
    let first_visible_min = 0;
    let first_visible_max = focused_index;
    let requested_offset = if file_manager.miller.horizontal.follow_active {
        miller_auto_follow_offset(column_stage.width, &preferred_widths, focused_index)
    } else {
        file_manager.miller.horizontal.offset_cells
    };
    let geometry =
        miller_viewport_geometry_at_offset(column_stage, &preferred_widths, requested_offset);

    let mut columns = geometry
        .columns
        .iter()
        .filter_map(|column| {
            let trail = trail_cols.get(column.chain_index)?;
            let kind = MillerColumnKind::Trail {
                trail_index: column.chain_index,
                directory: trail.directory.clone(),
                generation: file_manager.directory_generation,
                current: trail.directory == file_manager.cwd,
            };
            Some(MillerColumnView {
                projection_index: column.chain_index,
                kind,
                rect: column.rect,
                content_rect: Rect::new(
                    column.rect.x,
                    column.rect.y.saturating_add(1),
                    column.rect.width,
                    column.rect.height.saturating_sub(1),
                ),
                #[cfg(test)]
                rows: layout_rows_for_test(
                    file_manager,
                    column.chain_index,
                    column.rect,
                    trail_cols.len(),
                ),
            })
        })
        .collect::<Vec<_>>();
    let mut dividers = geometry
        .dividers
        .iter()
        .filter_map(|divider| {
            let left_column = columns
                .iter()
                .position(|column| column.projection_index == divider.left_chain_index)?;
            let right_column = columns
                .iter()
                .position(|column| column.projection_index == divider.right_chain_index)?;
            Some(MillerDividerView {
                left_column,
                right_column,
                rect: divider.rect,
            })
        })
        .collect::<Vec<_>>();
    if let (Some(width), Some(detail)) = (detail_width, detail) {
        let left_column = columns
            .iter()
            .position(|column| column.projection_index == file_manager.trail.deepest());
        if let Some(left_column) = left_column {
            let right_column = columns.len();
            let panel = Rect::new(
                stage.right().saturating_sub(width),
                stage.y,
                width,
                stage.height,
            );
            columns.push(MillerColumnView {
                projection_index: trail_cols.len(),
                kind: MillerColumnKind::Detail {
                    parent_trail_index: file_manager.trail.deepest(),
                    source_path: detail.path.clone(),
                    generation: file_manager.preview_generation,
                },
                rect: panel,
                content_rect: Rect::new(
                    panel.x.saturating_add(1),
                    panel.y.saturating_add(1),
                    panel.width.saturating_sub(2),
                    panel.height.saturating_sub(2),
                ),
                #[cfg(test)]
                rows: layout_rows_for_test(file_manager, trail_cols.len(), panel, trail_cols.len()),
            });
            dividers.push(MillerDividerView {
                left_column,
                right_column,
                rect: Rect::new(panel.x, panel.y, 1, panel.height),
            });
        }
    }

    MillerViewSnapshot {
        files_generation: Some(files_generation),
        model_revision: file_manager.miller.revision,
        resize_preview_active,
        focused_chain_index: Some(file_manager.trail.active_col()),
        first_visible_min,
        first_visible_max,
        first_visible: geometry.first_visible,
        horizontal_offset_cells: geometry.offset_cells,
        horizontal_max_offset_cells: geometry.max_offset_cells,
        columns,
        dividers,
    }
}

#[cfg(test)]
fn layout_rows_for_test(
    file_manager: &FmState,
    projection_index: usize,
    rect: Rect,
    trail_len: usize,
) -> Vec<MillerRowView> {
    let entries = if projection_index < trail_len {
        file_manager
            .trail_snapshots
            .cols()
            .get(projection_index)
            .map(|snapshot| snapshot.entries())
            .unwrap_or_default()
    } else {
        match &file_manager.preview {
            crate::fm::FmPreview::Directory(entries) => entries.as_slice(),
            crate::fm::FmPreview::None | crate::fm::FmPreview::File(_) => &[],
        }
    };
    entries
        .iter()
        .take(rect.height as usize)
        .enumerate()
        .map(|(index, entry)| MillerRowView {
            entry_path: entry.path.clone(),
            rect: Rect::new(rect.x, rect.y.saturating_add(index as u16), rect.width, 1),
        })
        .collect()
}

fn apply_resize_preview_widths(
    preferred_widths: &mut [u16],
    detail_preferred_width: &mut u16,
    file_manager: &FmState,
    files_generation: u32,
    resize_preview: Option<(&crate::ui::shell::MillerDividerId, [u16; 2])>,
) -> bool {
    let Some((divider, tracks)) = resize_preview else {
        return false;
    };
    if divider.files_generation() != files_generation
        || divider.model_revision() != file_manager.miller.revision
        || divider.axis() != crate::ui::shell::ShellDirection::Horizontal
        || divider.leading().projection_index().saturating_add(1)
            != divider.trailing().projection_index()
        || !miller_resize_column_is_live(divider.leading(), file_manager)
        || !miller_resize_column_is_live(divider.trailing(), file_manager)
    {
        return false;
    }
    let leading_index = divider.leading().projection_index();
    let Some(leading_width) = preferred_widths.get_mut(leading_index) else {
        return false;
    };
    *leading_width = tracks[0].clamp(MILLER_COLUMN_MIN_WIDTH, MILLER_COLUMN_MAX_WIDTH);
    match divider.trailing() {
        crate::ui::shell::MillerResizeColumnId::Directory { chain_index, .. } => {
            let Some(trailing_width) = preferred_widths.get_mut(*chain_index) else {
                return false;
            };
            *trailing_width = tracks[1].clamp(MILLER_COLUMN_MIN_WIDTH, MILLER_COLUMN_MAX_WIDTH);
        }
        crate::ui::shell::MillerResizeColumnId::Preview { .. } => {
            *detail_preferred_width = tracks[1].clamp(
                crate::fm::miller::MILLER_DETAIL_MIN_WIDTH,
                MILLER_COLUMN_MAX_WIDTH,
            );
        }
    }
    true
}

pub(crate) fn miller_resize_column_is_live(
    column: &crate::ui::shell::MillerResizeColumnId,
    file_manager: &FmState,
) -> bool {
    match column {
        crate::ui::shell::MillerResizeColumnId::Directory {
            chain_index,
            directory,
            generation,
        } => {
            *generation == file_manager.directory_generation
                && file_manager
                    .trail
                    .cols()
                    .get(*chain_index)
                    .is_some_and(|column| column.directory == *directory)
        }
        crate::ui::shell::MillerResizeColumnId::Preview {
            parent_chain_index,
            source_path,
            generation,
        } => {
            *parent_chain_index == file_manager.trail.deepest()
                && *generation == file_manager.preview_generation
                && source_path.as_ref()
                    == file_manager
                        .trail_snapshots
                        .detail()
                        .map(|detail| &detail.path)
        }
    }
}

#[cfg(test)]
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
    let focused_index = focused_index.min(chain_len - 1);
    let first_visible = requested_first_visible
        .min(chain_len - 1)
        .min(focused_index);
    let offset = preferred_widths
        .iter()
        .take(first_visible)
        .fold(0_u32, |offset, width| {
            offset
                .saturating_add(u32::from(
                    (*width).clamp(MILLER_COLUMN_MIN_WIDTH, MILLER_COLUMN_MAX_WIDTH),
                ))
                .saturating_add(u32::from(MILLER_DIVIDER_WIDTH))
        });
    miller_viewport_geometry_at_offset(stage, preferred_widths, offset)
}

pub(crate) fn miller_viewport_geometry_at_offset(
    stage: Rect,
    preferred_widths: &[u16],
    requested_offset_cells: u32,
) -> MillerViewportGeometry {
    if stage.width < MILLER_COLUMN_MIN_WIDTH || stage.height == 0 || preferred_widths.is_empty() {
        return MillerViewportGeometry::default();
    }

    let widths = preferred_widths
        .iter()
        .map(|width| (*width).clamp(MILLER_COLUMN_MIN_WIDTH, MILLER_COLUMN_MAX_WIDTH))
        .collect::<Vec<_>>();
    let total_width = widths
        .iter()
        .enumerate()
        .fold(0_u32, |total, (index, width)| {
            total.saturating_add(u32::from(*width)).saturating_add(
                u32::from(index + 1 < widths.len()) * u32::from(MILLER_DIVIDER_WIDTH),
            )
        });
    let max_offset_cells = total_width.saturating_sub(u32::from(stage.width));
    let offset_cells = requested_offset_cells.min(max_offset_cells);
    let viewport_end = offset_cells.saturating_add(u32::from(stage.width));
    let mut logical_x = 0_u32;
    let mut columns = Vec::new();
    let mut divider_intervals = Vec::new();

    for (chain_index, width) in widths.iter().copied().enumerate() {
        let column_start = logical_x;
        let column_end = column_start.saturating_add(u32::from(width));
        let visible_start = column_start.max(offset_cells);
        let visible_end = column_end.min(viewport_end);
        if visible_start < visible_end && columns.len() < MAX_VISIBLE_TRAIL_COLUMNS {
            let destination_x = stage
                .x
                .saturating_add((visible_start - offset_cells) as u16);
            columns.push(MillerColumnRect {
                chain_index,
                rect: Rect::new(
                    destination_x,
                    stage.y,
                    (visible_end - visible_start) as u16,
                    stage.height,
                ),
                logical_width: width,
                source_x: (visible_start - column_start) as u16,
            });
        }
        logical_x = column_end;
        if chain_index + 1 < widths.len() {
            let divider_end = logical_x.saturating_add(u32::from(MILLER_DIVIDER_WIDTH));
            divider_intervals.push((chain_index, chain_index + 1, logical_x, divider_end));
            logical_x = divider_end;
        }
    }

    let dividers = divider_intervals
        .into_iter()
        .filter_map(|(left_chain_index, right_chain_index, start, end)| {
            let visible_start = start.max(offset_cells);
            let visible_end = end.min(viewport_end);
            let both_columns_visible = columns
                .iter()
                .any(|column| column.chain_index == left_chain_index)
                && columns
                    .iter()
                    .any(|column| column.chain_index == right_chain_index);
            (visible_start < visible_end && both_columns_visible).then(|| MillerDividerRect {
                left_chain_index,
                right_chain_index,
                rect: Rect::new(
                    stage
                        .x
                        .saturating_add((visible_start - offset_cells) as u16),
                    stage.y,
                    (visible_end - visible_start) as u16,
                    stage.height,
                ),
            })
        })
        .collect::<Vec<_>>();
    let first_visible = columns.first().map_or(0, |column| column.chain_index);

    MillerViewportGeometry {
        columns,
        dividers,
        offset_cells,
        max_offset_cells,
        first_visible,
    }
}

pub(crate) fn miller_auto_follow_offset(
    stage_width: u16,
    preferred_widths: &[u16],
    focused_index: usize,
) -> u32 {
    if stage_width == 0 || preferred_widths.is_empty() {
        return 0;
    }
    let focused_index = focused_index.min(preferred_widths.len() - 1);
    let start = preferred_widths
        .iter()
        .take(focused_index)
        .fold(0_u32, |offset, width| {
            offset
                .saturating_add(u32::from(
                    (*width).clamp(MILLER_COLUMN_MIN_WIDTH, MILLER_COLUMN_MAX_WIDTH),
                ))
                .saturating_add(u32::from(MILLER_DIVIDER_WIDTH))
        });
    let width =
        preferred_widths[focused_index].clamp(MILLER_COLUMN_MIN_WIDTH, MILLER_COLUMN_MAX_WIDTH);
    let end = start.saturating_add(u32::from(width));
    let requested = if width > stage_width {
        start
    } else {
        end.saturating_sub(u32::from(stage_width))
    };
    let total_width = preferred_widths
        .iter()
        .enumerate()
        .fold(0_u32, |total, (index, width)| {
            total
                .saturating_add(u32::from(
                    (*width).clamp(MILLER_COLUMN_MIN_WIDTH, MILLER_COLUMN_MAX_WIDTH),
                ))
                .saturating_add(
                    u32::from(index + 1 < preferred_widths.len()) * u32::from(MILLER_DIVIDER_WIDTH),
                )
        });
    requested.min(total_width.saturating_sub(u32::from(stage_width)))
}

#[cfg(test)]
pub(crate) fn first_visible_floor(stage_width: u16, widths: &[u16], focused_index: usize) -> usize {
    let mut remaining = stage_width;
    let mut start = focused_index;
    let mut count = 0usize;
    let mut index = focused_index as isize;
    while index >= 0 && count < MAX_VISIBLE_TRAIL_COLUMNS {
        let preferred =
            widths[index as usize].clamp(MILLER_COLUMN_MIN_WIDTH, MILLER_COLUMN_MAX_WIDTH);
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

    #[test]
    fn auto_follow_origin_keeps_focused_column_complete() {
        let widths = vec![28; 8];
        let area = Rect::new(4, 2, 90, 12);
        let auto_origin = first_visible_floor(area.width, &widths, 7);
        let view = miller_viewport_geometry(area, &widths, 7, auto_origin);
        assert_eq!(
            view.columns.last().map(|column| column.chain_index),
            Some(7)
        );
        assert_eq!(
            view.columns
                .iter()
                .find(|column| column.chain_index == 7)
                .map(|column| column.rect.width),
            Some(28),
            "the focused column stays complete while an edge column may be clipped"
        );
        assert!(view.columns.iter().all(|column| {
            !column.rect.is_empty()
                && column.rect.x >= area.x
                && column.rect.right() <= area.right()
        }));
    }

    #[test]
    fn miller_viewport_offset_clips_both_edges_and_clamps_to_content() {
        let area = Rect::new(5, 2, 40, 12);
        let view = miller_viewport_geometry_at_offset(area, &[30, 30, 30], 10);

        assert_eq!(view.offset_cells, 10);
        assert_eq!(view.max_offset_cells, 52);
        assert_eq!(view.first_visible, 0);
        assert_eq!(
            view.columns
                .iter()
                .map(|column| (
                    column.chain_index,
                    column.rect,
                    column.logical_width,
                    column.source_x,
                ))
                .collect::<Vec<_>>(),
            vec![
                (0, Rect::new(5, 2, 20, 12), 30, 10),
                (1, Rect::new(26, 2, 19, 12), 30, 0),
            ]
        );
        assert_eq!(
            view.dividers
                .iter()
                .map(|divider| divider.rect)
                .collect::<Vec<_>>(),
            vec![Rect::new(25, 2, 1, 12)]
        );

        let clamped = miller_viewport_geometry_at_offset(area, &[30, 30, 30], u32::MAX);
        assert_eq!(clamped.offset_cells, clamped.max_offset_cells);
        assert_eq!(
            clamped.columns.last().map(|column| column.chain_index),
            Some(2)
        );
        assert_eq!(
            clamped.columns.last().map(|column| column.rect.right()),
            Some(area.right())
        );
    }

    #[test]
    fn zero_area_has_no_geometry() {
        let view = miller_viewport_geometry(
            Rect::ZERO,
            &[crate::fm::miller::MILLER_COLUMN_PREFERRED_WIDTH],
            0,
            0,
        );
        assert!(view.columns.is_empty());
        assert!(view.dividers.is_empty());
    }
}
