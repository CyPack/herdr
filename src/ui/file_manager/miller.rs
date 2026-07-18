//! Horizontal Miller viewport geometry (FM1.3): a bounded window of COMPLETE
//! columns plus divider rects over the logical chain, computed purely from
//! the Stage rectangle and per-segment preferred widths. Render and input
//! consume these rects; no filesystem work happens here. FM2 reuses the
//! divider rects as SF3 resize-transaction targets.
#![allow(dead_code)] // P1 production compute owns this snapshot; P2/P3 will
                     // consume every projected render/input identity.

use std::path::PathBuf;
use std::sync::Arc;

use ratatui::layout::Rect;

use crate::fm::miller::{
    MAX_RESIDENT_MILLER_COLUMNS, MILLER_COLUMN_MAX_WIDTH, MILLER_COLUMN_MIN_WIDTH,
};
use crate::fm::{FmPreview, FmState};

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

/// Prepared source for a non-current directory column. A missing source is an
/// explicit inert state; render must never load or substitute the directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MillerDirectorySource {
    Resident(crate::fm::miller::MillerColumnId),
    PreparedParent,
    Unavailable,
}

/// Typed identity of one visible column. Preview is a first-class snapshot
/// column so render and Kitty placement cannot silently lose or re-lay it out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MillerColumnKind {
    Directory {
        chain_index: usize,
        directory: PathBuf,
        source: MillerDirectorySource,
    },
    Current {
        chain_index: usize,
        directory: PathBuf,
        generation: u64,
    },
    Preview {
        parent_chain_index: usize,
        source_path: Option<PathBuf>,
        generation: u64,
    },
}

impl MillerColumnKind {
    pub(crate) fn chain_index(&self) -> Option<usize> {
        match self {
            Self::Directory { chain_index, .. } | Self::Current { chain_index, .. } => {
                Some(*chain_index)
            }
            Self::Preview { .. } => None,
        }
    }

    pub(crate) fn directory(&self) -> Option<&PathBuf> {
        match self {
            Self::Directory { directory, .. } | Self::Current { directory, .. } => Some(directory),
            Self::Preview { .. } => None,
        }
    }

    pub(crate) fn is_current(&self) -> bool {
        matches!(self, Self::Current { .. })
    }

    pub(crate) fn is_preview(&self) -> bool {
        matches!(self, Self::Preview { .. })
    }
}

/// One bounded visible row projected from already-prepared entries. Only the
/// visible exact path identities are cloned; complete directory vectors remain
/// singly owned by `FmState` or the resident cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MillerRowColumnKind {
    ResidentDirectory,
    PreparedParent,
    Current,
    Preview,
}

/// Generation-safe identity and geometry for one actionable Miller row.
///
/// Directory identity is shared across rows in the same column through an
/// `Arc`; this keeps the snapshot self-authenticating without cloning the same
/// potentially long path once per visible terminal row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MillerRowView {
    pub files_generation: u32,
    pub model_revision: u64,
    pub projection_index: usize,
    pub chain_index: Option<usize>,
    pub source_generation: u64,
    pub column_kind: MillerRowColumnKind,
    pub directory_path: Arc<PathBuf>,
    pub entry_index: usize,
    pub entry_path: PathBuf,
    pub rect: Rect,
}

/// One visible logical Miller column projected from prepared model state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MillerColumnView {
    /// Index in the geometry input. Preview uses the synthetic index directly
    /// after the focused current chain segment.
    pub projection_index: usize,
    pub kind: MillerColumnKind,
    pub rect: Rect,
    pub content_rect: Rect,
    pub cursor: Option<usize>,
    pub viewport_start: usize,
    pub rows: Vec<MillerRowView>,
}

/// One visible divider linked to exact adjacent entries in `columns`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MillerDividerView {
    pub left_column: usize,
    pub right_column: usize,
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
    /// `true` only when this frame applied a live, generation-valid resize
    /// transaction. Background image work must not chase this transient
    /// geometry; the first committed frame clears the flag and resynchronizes
    /// the target exactly once.
    pub resize_preview_active: bool,
    pub focused_chain_index: Option<usize>,
    /// Inclusive horizontal origin bounds for input against this exact frame.
    /// The lower bound is the fullest complete window containing focus; the
    /// upper bound keeps CURRENT (and inline PREVIEW, when present) visible.
    pub first_visible_min: usize,
    pub first_visible_max: usize,
    pub first_visible: usize,
    pub columns: Vec<MillerColumnView>,
    pub dividers: Vec<MillerDividerView>,
}

impl MillerViewSnapshot {
    /// Resolve one bounded horizontal input step against this frame.
    ///
    /// A stale structural projection is inert. Horizontal position itself
    /// deliberately does not increment `MillerState::revision`, so repeated
    /// wheel events may share the same frame bounds until the next compute.
    pub(crate) fn horizontal_scroll_target(
        &self,
        file_manager: &FmState,
        delta: i8,
    ) -> Option<usize> {
        if self.files_generation.is_none()
            || self.model_revision != file_manager.miller.revision
            || self.focused_chain_index
                != file_manager
                    .miller
                    .chain
                    .iter()
                    .position(|segment| segment.directory == file_manager.miller.focused_directory)
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

    /// Exact prepared preview content rect for this frame. Generation and
    /// selected-path checks make stale snapshots inert for both text and Kitty
    /// consumers; no caller may reconstruct preview geometry independently.
    pub(crate) fn preview_content_rect(&self, file_manager: &FmState) -> Option<Rect> {
        self.columns.iter().find_map(|column| {
            let MillerColumnKind::Preview {
                source_path,
                generation,
                ..
            } = &column.kind
            else {
                return None;
            };
            (*generation == file_manager.preview_generation
                && source_path.as_ref() == file_manager.selected().map(|entry| &entry.path)
                && column.content_rect.width > 0
                && column.content_rect.height > 0)
                .then_some(column.content_rect)
        })
    }
}

/// Project prepared Miller model state into a bounded frame snapshot.
///
/// This function performs no filesystem work. At most five columns (including
/// inline preview), four dividers, and their visible row path identities are
/// projected.
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
    let mut preferred_widths = chain
        .iter()
        .take(focused_chain_index + 1)
        .map(|segment| segment.preferred_width)
        .collect::<Vec<_>>();
    let preview_projection_index = focused_chain_index + 1;
    let project_inline_preview = stage.width
        >= MILLER_COLUMN_MIN_WIDTH
            .saturating_mul(2)
            .saturating_add(MILLER_DIVIDER_WIDTH);
    let focused_projection_index = if project_inline_preview {
        // Reserve the preview minimum before honoring the current column's
        // preferred width. Without this pair budget, the generic geometry's
        // complete-column rule can greedily keep a 28-cell preview and evict
        // current at the exact 33-cell two-column threshold.
        let pair_budget = stage.width.saturating_sub(MILLER_DIVIDER_WIDTH);
        let current_max = pair_budget
            .saturating_sub(MILLER_COLUMN_MIN_WIDTH)
            .min(MILLER_COLUMN_MAX_WIDTH);
        let current_width =
            preferred_widths[focused_chain_index].clamp(MILLER_COLUMN_MIN_WIDTH, current_max);
        let preview_max = pair_budget
            .saturating_sub(current_width)
            .min(MILLER_COLUMN_MAX_WIDTH);
        let preview_width = file_manager
            .miller
            .preview_preferred_width
            .clamp(MILLER_COLUMN_MIN_WIDTH, preview_max);
        preferred_widths[focused_chain_index] = current_width;
        preferred_widths.push(preview_width);
        preview_projection_index
    } else {
        focused_chain_index
    };
    let resize_preview_active = apply_resize_preview_widths(
        &mut preferred_widths,
        file_manager,
        files_generation,
        resize_preview,
    );
    let first_visible_min =
        first_visible_floor(stage.width, &preferred_widths, focused_projection_index);
    let first_visible_max = focused_chain_index;
    let requested_first_visible = file_manager
        .miller
        .horizontal
        .first_visible
        .clamp(first_visible_min, first_visible_max);
    let geometry = miller_viewport_geometry(
        stage,
        &preferred_widths,
        focused_projection_index,
        requested_first_visible,
    );
    let (first_visible_min, first_visible_max) = if geometry.columns.is_empty() {
        (0, 0)
    } else {
        (first_visible_min, first_visible_max)
    };

    let mut columns = Vec::with_capacity(geometry.columns.len());
    for column in geometry.columns {
        let content_rect = column_content_rect(column.rect);
        if project_inline_preview && column.chain_index == preview_projection_index {
            let source_path = file_manager.selected().map(|entry| entry.path.clone());
            let (entries, cursor, viewport_start) = match &file_manager.preview {
                FmPreview::Directory(entries) => (
                    entries.as_slice(),
                    None,
                    file_manager.preview_viewport_start,
                ),
                FmPreview::None | FmPreview::File(_) => (&[][..], None, 0),
            };
            let rows = source_path.as_ref().map_or_else(Vec::new, |directory| {
                project_visible_rows(
                    content_rect,
                    entries,
                    viewport_start,
                    MillerRowAuthority {
                        files_generation,
                        model_revision: file_manager.miller.revision,
                        projection_index: column.chain_index,
                        chain_index: None,
                        source_generation: file_manager.preview_generation,
                        column_kind: MillerRowColumnKind::Preview,
                        directory_path: Arc::new(directory.clone()),
                    },
                )
            });
            columns.push(MillerColumnView {
                projection_index: column.chain_index,
                kind: MillerColumnKind::Preview {
                    parent_chain_index: focused_chain_index,
                    source_path,
                    generation: file_manager.preview_generation,
                },
                rect: column.rect,
                content_rect,
                cursor,
                viewport_start,
                rows,
            });
            continue;
        }

        let Some(segment) = chain.get(column.chain_index) else {
            continue;
        };
        let directory = segment.directory.clone();
        let row_directory_path = Arc::new(directory.clone());
        let (kind, entries, cursor, viewport_start, source_generation, row_column_kind) =
            if directory == file_manager.cwd {
                (
                    MillerColumnKind::Current {
                        chain_index: column.chain_index,
                        directory,
                        generation: file_manager.directory_generation,
                    },
                    file_manager.entries.as_slice(),
                    (!file_manager.entries.is_empty()).then_some(file_manager.cursor),
                    file_manager.viewport_start,
                    file_manager.directory_generation,
                    MillerRowColumnKind::Current,
                )
            } else if let Some(resident) = file_manager
                .miller
                .resident_projection_for_directory(&directory)
            {
                (
                    MillerColumnKind::Directory {
                        chain_index: column.chain_index,
                        directory,
                        source: MillerDirectorySource::Resident(resident.id.clone()),
                    },
                    resident.entries.as_slice(),
                    crate::fm::miller::MillerState::resolve_resident_selection(
                        segment,
                        resident.entries.as_slice(),
                    ),
                    resident_viewport_start(
                        segment,
                        resident.entries.as_slice(),
                        content_rect.height,
                    ),
                    resident.id.generation,
                    MillerRowColumnKind::ResidentDirectory,
                )
            } else if file_manager
                .cwd
                .parent()
                .is_some_and(|parent| parent == directory)
            {
                match file_manager.parent.as_ref() {
                    Some(parent) => (
                        MillerColumnKind::Directory {
                            chain_index: column.chain_index,
                            directory,
                            source: MillerDirectorySource::PreparedParent,
                        },
                        parent.entries.as_slice(),
                        parent.cursor,
                        segment.viewport_start,
                        file_manager.directory_generation,
                        MillerRowColumnKind::PreparedParent,
                    ),
                    None => (
                        MillerColumnKind::Directory {
                            chain_index: column.chain_index,
                            directory,
                            source: MillerDirectorySource::Unavailable,
                        },
                        &[][..],
                        None,
                        0,
                        file_manager.directory_generation,
                        MillerRowColumnKind::PreparedParent,
                    ),
                }
            } else {
                (
                    MillerColumnKind::Directory {
                        chain_index: column.chain_index,
                        directory,
                        source: MillerDirectorySource::Unavailable,
                    },
                    &[][..],
                    None,
                    0,
                    0,
                    MillerRowColumnKind::ResidentDirectory,
                )
            };
        let rows = project_visible_rows(
            content_rect,
            entries,
            viewport_start,
            MillerRowAuthority {
                files_generation,
                model_revision: file_manager.miller.revision,
                projection_index: column.chain_index,
                chain_index: Some(column.chain_index),
                source_generation,
                column_kind: row_column_kind,
                directory_path: row_directory_path,
            },
        );
        columns.push(MillerColumnView {
            projection_index: column.chain_index,
            kind,
            rect: column.rect,
            content_rect,
            cursor,
            viewport_start,
            rows,
        });
    }

    let mut dividers = Vec::with_capacity(geometry.dividers.len());
    for divider in geometry.dividers {
        let (Some(left_column), Some(right_column)) = (
            columns
                .iter()
                .position(|column| column.projection_index == divider.left_chain_index),
            columns
                .iter()
                .position(|column| column.projection_index == divider.right_chain_index),
        ) else {
            continue;
        };
        dividers.push(MillerDividerView {
            left_column,
            right_column,
            rect: divider.rect,
        });
    }

    MillerViewSnapshot {
        files_generation: Some(files_generation),
        model_revision: file_manager.miller.revision,
        resize_preview_active,
        focused_chain_index: Some(focused_chain_index),
        first_visible_min,
        first_visible_max,
        first_visible: geometry.first_visible,
        columns,
        dividers,
    }
}

fn apply_resize_preview_widths(
    preferred_widths: &mut [u16],
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
    {
        return false;
    }
    let leading_index = divider.leading().projection_index();
    let trailing_index = divider.trailing().projection_index();
    if leading_index.saturating_add(1) != trailing_index
        || trailing_index >= preferred_widths.len()
        || !miller_resize_column_is_live(divider.leading(), file_manager)
        || !miller_resize_column_is_live(divider.trailing(), file_manager)
    {
        return false;
    }
    preferred_widths[leading_index] =
        tracks[0].clamp(MILLER_COLUMN_MIN_WIDTH, MILLER_COLUMN_MAX_WIDTH);
    preferred_widths[trailing_index] =
        tracks[1].clamp(MILLER_COLUMN_MIN_WIDTH, MILLER_COLUMN_MAX_WIDTH);
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
            let path_matches = file_manager
                .miller
                .chain
                .get(*chain_index)
                .is_some_and(|segment| segment.directory == *directory);
            let generation_matches = if file_manager.cwd == *directory
                || file_manager
                    .cwd
                    .parent()
                    .is_some_and(|parent| parent == directory)
            {
                *generation == file_manager.directory_generation
            } else if let Some(resident) = file_manager
                .miller
                .resident_projection_for_directory(directory)
            {
                *generation == resident.id.generation
            } else {
                *generation == 0
            };
            path_matches && generation_matches
        }
        crate::ui::shell::MillerResizeColumnId::Preview {
            parent_chain_index,
            source_path,
            generation,
        } => {
            file_manager
                .miller
                .chain
                .get(*parent_chain_index)
                .is_some_and(|segment| segment.directory == file_manager.cwd)
                && source_path.as_ref() == file_manager.selected().map(|entry| &entry.path)
                && *generation == file_manager.preview_generation
        }
    }
}

/// Clamp the cached resident viewport so the re-resolved focused row stays
/// visible (TP-FIP-FOCUS-07). Without a resolved selection the cached window
/// is used unchanged.
fn resident_viewport_start(
    segment: &crate::fm::miller::MillerPathSegment,
    entries: &[crate::fm::FileEntry],
    visible_rows: u16,
) -> usize {
    let cached = segment.viewport_start;
    let Some(cursor) = crate::fm::miller::MillerState::resolve_resident_selection(segment, entries)
    else {
        return cached;
    };
    let visible = visible_rows as usize;
    if visible == 0 {
        return cached;
    }
    if cursor < cached {
        cursor
    } else if cursor >= cached + visible {
        cursor + 1 - visible
    } else {
        cached
    }
}

fn column_content_rect(column: Rect) -> Rect {
    Rect::new(
        column.x,
        column.y.saturating_add(1),
        column.width,
        column.height.saturating_sub(1),
    )
}

fn project_visible_rows(
    content: Rect,
    entries: &[crate::fm::FileEntry],
    viewport_start: usize,
    authority: MillerRowAuthority,
) -> Vec<MillerRowView> {
    let visible_rows = content.height as usize;
    if content.width == 0 || visible_rows == 0 || entries.is_empty() {
        return Vec::new();
    }
    let start = viewport_start.min(entries.len().saturating_sub(visible_rows));
    let count = visible_rows.min(entries.len().saturating_sub(start));
    (0..count)
        .map(|offset| {
            let entry_index = start + offset;
            MillerRowView {
                files_generation: authority.files_generation,
                model_revision: authority.model_revision,
                projection_index: authority.projection_index,
                chain_index: authority.chain_index,
                source_generation: authority.source_generation,
                column_kind: authority.column_kind,
                directory_path: Arc::clone(&authority.directory_path),
                entry_index,
                entry_path: entries[entry_index].path.clone(),
                rect: Rect::new(
                    content.x,
                    content.y.saturating_add(offset as u16),
                    content.width,
                    1,
                ),
            }
        })
        .collect()
}

#[derive(Debug, Clone)]
struct MillerRowAuthority {
    files_generation: u32,
    model_revision: u64,
    projection_index: usize,
    chain_index: Option<usize>,
    source_generation: u64,
    column_kind: MillerRowColumnKind,
    directory_path: Arc<PathBuf>,
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

    fn entry(path: impl Into<PathBuf>, is_dir: bool) -> crate::fm::FileEntry {
        let path = path.into();
        crate::fm::FileEntry {
            name: path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
            path,
            kind: if is_dir {
                crate::fm::entry_kind::FileEntryKind::Directory
            } else {
                crate::fm::entry_kind::FileEntryKind::RegularFile
            },
        }
    }

    // P1.4: every rendered directory source is explicit prepared state. The
    // snapshot also carries only bounded visible rows, never whole entry
    // vectors or an implicit filesystem fallback.
    #[test]
    fn typed_projection_carries_all_sources_and_bounded_visible_rows() {
        let unavailable = PathBuf::from("/virtual/unavailable");
        let resident = PathBuf::from("/virtual/resident");
        let parent = PathBuf::from("/virtual/parent");
        let current = parent.join("current");
        let chain = [
            unavailable.clone(),
            resident.clone(),
            parent.clone(),
            current.clone(),
        ];
        let mut file_manager = FmState::test_empty(current.clone());
        file_manager.miller.chain = chain
            .iter()
            .cloned()
            .map(crate::fm::miller::MillerPathSegment::new)
            .collect();
        file_manager.miller.focused_directory = current.clone();
        file_manager.entries = (0..9)
            .map(|index| entry(current.join(format!("current-{index}.txt")), false))
            .collect();
        file_manager.cursor = 7;
        file_manager.viewport_start = 5;
        file_manager.parent = Some(crate::fm::FmParent {
            entries: vec![entry(&current, true), entry(parent.join("peer"), true)],
            cursor: Some(0),
        });
        file_manager.preview = FmPreview::Directory(
            (0..7)
                .map(|index| {
                    entry(
                        current.join("selected").join(format!("child-{index}")),
                        true,
                    )
                })
                .collect(),
        );
        let resident_id = crate::fm::miller::MillerColumnId {
            directory: resident.clone(),
            generation: 42,
        };
        file_manager.miller.visit(
            current.clone(),
            Some(crate::fm::miller::MillerDirectoryProjection {
                id: resident_id.clone(),
                entries: vec![entry(resident.join("cached.txt"), false)],
                status: crate::fm::FmDirectoryStatus::Available,
                writable: true,
            }),
        );

        let snapshot = project_miller_view(Rect::new(3, 2, 144, 5), &file_manager, 9);

        assert_eq!(snapshot.columns.len(), 5);
        assert_eq!(snapshot.dividers.len(), 4);
        assert!(matches!(
            &snapshot.columns[0].kind,
            MillerColumnKind::Directory {
                directory,
                source: MillerDirectorySource::Unavailable,
                ..
            } if directory == &unavailable
        ));
        assert!(matches!(
            &snapshot.columns[1].kind,
            MillerColumnKind::Directory {
                directory,
                source: MillerDirectorySource::Resident(id),
                ..
            } if directory == &resident && id == &resident_id
        ));
        assert!(matches!(
            &snapshot.columns[2].kind,
            MillerColumnKind::Directory {
                directory,
                source: MillerDirectorySource::PreparedParent,
                ..
            } if directory == &parent
        ));
        assert!(matches!(
            &snapshot.columns[3].kind,
            MillerColumnKind::Current {
                directory,
                generation: 1,
                ..
            } if directory == &current
        ));
        assert!(matches!(
            &snapshot.columns[4].kind,
            MillerColumnKind::Preview {
                parent_chain_index: 3,
                generation: 0,
                ..
            }
        ));
        for column in &snapshot.columns {
            assert!(
                column.rows.len() <= column.content_rect.height as usize,
                "visible row identities stay bounded by the projected content rect"
            );
            assert!(column
                .rows
                .iter()
                .all(|row| row.rect.intersection(column.content_rect) == row.rect));
        }
        assert_eq!(
            snapshot.columns[3]
                .rows
                .iter()
                .map(|row| row.entry_index)
                .collect::<Vec<_>>(),
            vec![5, 6, 7, 8],
            "current rows preserve the prepared viewport window"
        );
        assert_eq!(
            snapshot.columns[4].rows.len(),
            4,
            "preview directory rows are clipped to the same bounded content height"
        );

        // FM3: every actionable row carries its own complete
        // generation-safe column authority. A consumer may not authorize a
        // click by combining a bare row index with whichever column happens
        // to occupy the same rectangle in a later frame.
        for column in snapshot
            .columns
            .iter()
            .filter(|column| !column.rows.is_empty())
        {
            for row in &column.rows {
                assert_eq!(row.files_generation, 9);
                assert_eq!(row.model_revision, 1);
                assert_eq!(row.projection_index, column.projection_index);
                assert_eq!(row.rect.intersection(column.content_rect), row.rect);
            }
        }
        let row_rects = snapshot
            .columns
            .iter()
            .flat_map(|column| column.rows.iter().map(|row| row.rect))
            .collect::<Vec<_>>();
        for (index, rect) in row_rects.iter().enumerate() {
            assert!(!rect.is_empty(), "every actionable row has a hit area");
            for other in row_rects.iter().skip(index + 1) {
                assert!(
                    rect.intersection(*other).is_empty(),
                    "generation-safe row hit areas are globally disjoint"
                );
            }
        }
        let resident_row = &snapshot.columns[1].rows[0];
        assert_eq!(
            (
                resident_row.chain_index,
                resident_row.source_generation,
                resident_row.column_kind,
                resident_row.directory_path.as_ref(),
            ),
            (
                Some(1),
                42,
                MillerRowColumnKind::ResidentDirectory,
                &resident,
            )
        );
        let parent_row = &snapshot.columns[2].rows[0];
        assert_eq!(
            (
                parent_row.chain_index,
                parent_row.source_generation,
                parent_row.column_kind,
                parent_row.directory_path.as_ref(),
            ),
            (
                Some(2),
                file_manager.directory_generation,
                MillerRowColumnKind::PreparedParent,
                &parent,
            )
        );
        let current_row = &snapshot.columns[3].rows[0];
        assert_eq!(
            (
                current_row.chain_index,
                current_row.source_generation,
                current_row.column_kind,
                current_row.directory_path.as_ref(),
            ),
            (
                Some(3),
                file_manager.directory_generation,
                MillerRowColumnKind::Current,
                &current,
            )
        );
        let preview_row = &snapshot.columns[4].rows[0];
        assert_eq!(
            (
                preview_row.chain_index,
                preview_row.source_generation,
                preview_row.column_kind,
                preview_row.directory_path.as_ref(),
            ),
            (
                None,
                file_manager.preview_generation,
                MillerRowColumnKind::Preview,
                &current.join("current-7.txt"),
            )
        );
    }

    // TP-FIP-FOCUS-03/04/10: the resident column selection re-resolves the
    // bound focused-child PATH against the exact resident entries. A stale
    // cached index, a deleted child, or a duplicate identity can never
    // highlight an unrelated row.
    #[test]
    fn resident_column_reresolves_focused_child_by_unique_path() {
        let resident_dir = PathBuf::from("/virtual/resident");
        let current = resident_dir.join("beta");
        let beta = current.clone();
        let build = |entries: Vec<crate::fm::FileEntry>,
                     focused_child: Option<PathBuf>|
         -> MillerViewSnapshot {
            let mut file_manager = FmState::test_empty(current.clone());
            file_manager.miller.chain = vec![
                crate::fm::miller::MillerPathSegment::new(resident_dir.clone()),
                crate::fm::miller::MillerPathSegment::new(current.clone()),
            ]
            .into();
            file_manager.miller.chain[0].focused_child = focused_child;
            file_manager.miller.chain[0].cursor = 1; // stale cached index
            file_manager.miller.focused_directory = current.clone();
            file_manager.miller.visit(
                current.clone(),
                Some(crate::fm::miller::MillerDirectoryProjection {
                    id: crate::fm::miller::MillerColumnId {
                        directory: resident_dir.clone(),
                        generation: 7,
                    },
                    entries,
                    status: crate::fm::FmDirectoryStatus::Available,
                    writable: true,
                }),
            );
            project_miller_view(Rect::new(0, 0, 144, 8), &file_manager, 3)
        };
        let resident_cursor = |snapshot: &MillerViewSnapshot| {
            snapshot
                .columns
                .iter()
                .find(|column| {
                    matches!(
                        &column.kind,
                        MillerColumnKind::Directory {
                            source: MillerDirectorySource::Resident(_),
                            ..
                        }
                    )
                })
                .expect("resident column visible")
                .cursor
        };

        // Reorder: beta moved from cached index 1 to index 2 — path wins.
        let reordered = build(
            vec![
                entry(resident_dir.join("alpha"), true),
                entry(resident_dir.join("gamma"), true),
                entry(beta.clone(), true),
            ],
            Some(beta.clone()),
        );
        assert_eq!(resident_cursor(&reordered), Some(2));

        // Deleted focused child: no unrelated row may be highlighted.
        let deleted = build(
            vec![
                entry(resident_dir.join("alpha"), true),
                entry(resident_dir.join("gamma"), true),
            ],
            Some(beta.clone()),
        );
        assert_eq!(resident_cursor(&deleted), None);

        // Duplicate identity: ambiguous authority resolves to no selection.
        let duplicated = build(
            vec![
                entry(resident_dir.join("alpha"), true),
                entry(beta.clone(), true),
                entry(beta.clone(), true),
            ],
            Some(beta),
        );
        assert_eq!(resident_cursor(&duplicated), None);
    }

    // TP-FIP-FOCUS-07: when the re-resolved focused row falls outside the
    // cached viewport window, the resident column clamps its viewport so the
    // exact focused row is visible.
    #[test]
    fn resident_viewport_clamps_resolved_focus_visible() {
        let resident_dir = PathBuf::from("/virtual/resident");
        let current = resident_dir.join("child-25");
        let entries: Vec<crate::fm::FileEntry> = (0..30)
            .map(|index| entry(resident_dir.join(format!("child-{index:02}")), true))
            .collect();
        let focused = entries[25].path.clone();
        let mut file_manager = FmState::test_empty(current.clone());
        file_manager.miller.chain = vec![
            crate::fm::miller::MillerPathSegment::new(resident_dir.clone()),
            crate::fm::miller::MillerPathSegment::new(current.clone()),
        ]
        .into();
        file_manager.miller.chain[0].focused_child = Some(focused.clone());
        file_manager.miller.chain[0].viewport_start = 0; // stale window
        file_manager.miller.focused_directory = current.clone();
        file_manager.miller.visit(
            current.clone(),
            Some(crate::fm::miller::MillerDirectoryProjection {
                id: crate::fm::miller::MillerColumnId {
                    directory: resident_dir.clone(),
                    generation: 7,
                },
                entries,
                status: crate::fm::FmDirectoryStatus::Available,
                writable: true,
            }),
        );

        let snapshot = project_miller_view(Rect::new(0, 0, 144, 8), &file_manager, 3);
        let column = snapshot
            .columns
            .iter()
            .find(|column| {
                matches!(
                    &column.kind,
                    MillerColumnKind::Directory {
                        source: MillerDirectorySource::Resident(_),
                        ..
                    }
                )
            })
            .expect("resident column visible");
        assert_eq!(column.cursor, Some(25));
        assert!(
            column.rows.iter().any(|row| row.entry_index == 25),
            "the focused row must be inside the visible window \
             (viewport_start={})",
            column.viewport_start
        );
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
