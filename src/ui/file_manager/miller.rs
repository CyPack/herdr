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
    pub columns: Vec<MillerColumnView>,
    pub dividers: Vec<MillerDividerView>,
}

impl MillerViewSnapshot {
    pub(crate) fn horizontal_scroll_target(
        &self,
        file_manager: &FmState,
        delta: i8,
    ) -> Option<usize> {
        if self.files_generation.is_none()
            || self.model_revision != file_manager.miller.revision
            || self.columns.is_empty()
            || self.first_visible_min > self.first_visible_max
        {
            return None;
        }
        let current = file_manager
            .miller
            .horizontal
            .first_visible
            .clamp(self.first_visible_min, self.first_visible_max);
        Some(if delta < 0 {
            current.saturating_sub(1).max(self.first_visible_min)
        } else if delta > 0 {
            current.saturating_add(1).min(self.first_visible_max)
        } else {
            current
        })
    }
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
    let auto_first_visible =
        first_visible_floor(column_stage.width, &preferred_widths, focused_index);
    let first_visible_min = 0;
    let first_visible_max = focused_index;
    let requested = if file_manager.miller.horizontal.follow_active {
        auto_first_visible
    } else {
        file_manager
            .miller
            .horizontal
            .first_visible
            .clamp(first_visible_min, first_visible_max)
    };
    let geometry =
        miller_viewport_geometry(column_stage, &preferred_widths, focused_index, requested);

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
    let mut columns = Vec::new();
    let mut dividers = Vec::new();
    let mut x = stage.x;
    let mut remaining = stage.width;
    let mut chain_index = first_visible;
    while chain_index < chain_len && columns.len() < MAX_VISIBLE_TRAIL_COLUMNS {
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
        assert!(view
            .columns
            .iter()
            .all(|column| column.rect.width >= MILLER_COLUMN_MIN_WIDTH));
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
