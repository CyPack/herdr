//! Native file manager — accumulating Miller Trail render.
//!
//! Draws the open [`FmState`](crate::fm::FmState) into a rect: a one-row current
//! directory header followed by the canonical root-to-active Trail and optional
//! detail panel. Pure draw (reads state, never mutates or touches the
//! filesystem), matching herdr's `compute_view`/`render` split. Client-side
//! presentation only.
//!
//! This is the first non-terminal *content* swapped into a named region
//! (`CenterContent`): when `app.file_manager` is open, the base layer draws this
//! here instead of the terminal panes. Text/image previews and row-action
//! geometry build on the same pure client-side projection.

pub(crate) mod locations;
pub(crate) mod miller;
pub(crate) mod trail_view;

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;
use std::collections::BTreeSet;
use unicode_width::UnicodeWidthChar;

use super::text::truncate_end;
use super::widgets::{centered_popup_rect, render_panel_shell};
#[cfg(test)]
use crate::app::state::FileManagerRowAction;
use crate::app::state::{AppState, Palette};
use crate::app::state::{
    FileManagerActionBarModel, FileManagerActionBarSelection, FileManagerActionBarSelectionKind,
    FileManagerActionDisabledReason, FileManagerActionState, FileManagerHeaderAction,
    FileManagerHeaderActionArea, FileManagerOperationKind, FileManagerOperationState,
    FileManagerOperationStatus, FileManagerRowActionArea, FileManagerRowArea,
};
use crate::fm::{
    FileEntry, FmDirectoryStatus, FmFilePreview, FmImagePreviewState, FmPreview, FmState,
    HighlightedTextPreview, ImagePreviewError, PreviewTextLine, PreviewTextSpan, PreviewTextStyle,
    TextPreview, TextPreviewError,
};

const MIN_COLUMN_WIDTH: u16 = 12;
const DIVIDER_WIDTH: u16 = 1;
const THREE_COLUMN_MIN_WIDTH: u16 = MIN_COLUMN_WIDTH * 3 + DIVIDER_WIDTH * 2;
const TWO_COLUMN_MIN_WIDTH: u16 = MIN_COLUMN_WIDTH * 2 + DIVIDER_WIDTH;
const MAX_RENDERED_PREVIEW_LINES: usize = 128;
const HEADER_MIN_IDENTITY_WIDTH: u16 = 12;
const HEADER_ACTION_GAP: u16 = 1;
const ROW_ACTION_WIDTH: u16 = 1;

#[derive(Debug, Clone, Copy)]
struct FileManagerVisualStyles {
    canvas: Style,
    identity: Style,
    panel_title: Style,
    divider: Style,
    enabled_action: Style,
    disabled_action: Style,
    empty: Style,
    cursor: Style,
    multi_selection: Style,
    directory: Style,
    file: Style,
    warning: Style,
    error: Style,
    running: Style,
    completed: Style,
    cancelled: Style,
}

/// Project theme roles into native-FM styles without embedding theme-specific
/// colors or deriving semantic state from painted glyphs.
fn file_manager_visual_styles(palette: &Palette) -> FileManagerVisualStyles {
    FileManagerVisualStyles {
        canvas: Style::default().fg(palette.text).bg(palette.panel_bg),
        identity: Style::default()
            .fg(palette.subtext0)
            .bg(palette.panel_bg)
            .add_modifier(Modifier::BOLD),
        panel_title: Style::default()
            .fg(palette.overlay1)
            .bg(palette.panel_bg)
            .add_modifier(Modifier::BOLD),
        divider: Style::default()
            .fg(palette.surface_dim)
            .bg(palette.panel_bg),
        enabled_action: Style::default().fg(palette.overlay1).bg(palette.panel_bg),
        disabled_action: Style::default()
            .fg(palette.overlay0)
            .bg(palette.panel_bg)
            .add_modifier(Modifier::DIM),
        empty: Style::default().fg(palette.overlay0).bg(palette.panel_bg),
        cursor: Style::default()
            .bg(palette.surface0)
            .fg(palette.text)
            .add_modifier(Modifier::BOLD),
        multi_selection: Style::default()
            .bg(palette.surface1)
            .fg(palette.text)
            .add_modifier(Modifier::BOLD),
        directory: Style::default()
            .fg(palette.blue)
            .bg(palette.panel_bg)
            .add_modifier(Modifier::BOLD),
        file: Style::default().fg(palette.subtext0).bg(palette.panel_bg),
        warning: Style::default().fg(palette.yellow).bg(palette.panel_bg),
        error: Style::default().fg(palette.red).bg(palette.panel_bg),
        running: Style::default().fg(palette.yellow).bg(palette.surface0),
        completed: Style::default().fg(palette.green).bg(palette.surface0),
        cancelled: Style::default().fg(palette.peach).bg(palette.surface0),
    }
}

#[derive(Debug, Clone, Copy)]
struct MillerLayout {
    parent: Option<Rect>,
    current: Rect,
    preview: Option<Rect>,
    dividers: [Option<Rect>; 2],
}

#[derive(Debug, Clone, Copy)]
struct FileManagerAreas {
    header: Rect,
    columns: MillerLayout,
}

fn miller_layout(area: Rect) -> MillerLayout {
    if area.width >= THREE_COLUMN_MIN_WIDTH {
        let [parent, first_divider, current, second_divider, preview] = Layout::horizontal([
            Constraint::Min(MIN_COLUMN_WIDTH),
            Constraint::Length(DIVIDER_WIDTH),
            Constraint::Min(MIN_COLUMN_WIDTH),
            Constraint::Length(DIVIDER_WIDTH),
            Constraint::Min(MIN_COLUMN_WIDTH),
        ])
        .areas(area);
        MillerLayout {
            parent: Some(parent),
            current,
            preview: Some(preview),
            dividers: [Some(first_divider), Some(second_divider)],
        }
    } else if area.width >= TWO_COLUMN_MIN_WIDTH {
        let [current, divider, preview] = Layout::horizontal([
            Constraint::Min(MIN_COLUMN_WIDTH),
            Constraint::Length(DIVIDER_WIDTH),
            Constraint::Min(MIN_COLUMN_WIDTH),
        ])
        .areas(area);
        MillerLayout {
            parent: None,
            current,
            preview: Some(preview),
            dividers: [Some(divider), None],
        }
    } else {
        MillerLayout {
            parent: None,
            current: area,
            preview: None,
            dividers: [None, None],
        }
    }
}

fn file_manager_areas(area: Rect) -> Option<FileManagerAreas> {
    let [header, body, _status] = file_manager_frame_areas(area)?;
    Some(FileManagerAreas {
        header,
        columns: miller_layout(body),
    })
}

fn file_manager_frame_areas(area: Rect) -> Option<[Rect; 3]> {
    if area.width == 0 || area.height == 0 {
        return None;
    }
    Some(
        Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .areas(area),
    )
}

/// Complete body reserved for Miller columns between the one-row header and
/// one-row status line. Production viewport projection, compatibility render,
/// hit geometry, and preview placement derive from this same frame split.
#[cfg(test)]
pub(crate) fn file_manager_miller_viewport_area(area: Rect) -> Rect {
    file_manager_frame_areas(area)
        .map(|[_, body, _]| body)
        .unwrap_or_default()
}

fn panel_areas(area: Rect) -> [Rect; 2] {
    Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(area)
}

/// Pixel graphics and text rendering share this exact PREVIEW content seam.
/// The top-level FM header and the PREVIEW panel title are intentionally
/// excluded so host graphics cannot cover either label.
#[cfg(test)]
pub(crate) fn file_manager_preview_content_area(area: Rect) -> Option<Rect> {
    let preview = file_manager_areas(area)?.columns.preview?;
    let content = panel_areas(preview)[1];
    (content.width > 0 && content.height > 0).then_some(content)
}

/// Lay out visible CURRENT entry rows for both pure rendering and input hit
/// testing. The viewport is defensively clamped so stale state cannot create
/// off-list targets even if this helper is called before the next state sync.
#[derive(Debug, Default)]
pub(crate) struct FileManagerRowGeometry {
    pub(crate) rows: Vec<FileManagerRowArea>,
    pub(crate) actions: Vec<FileManagerRowActionArea>,
}

#[cfg(test)]
pub(crate) fn compute_file_manager_row_geometry(
    area: Rect,
    entries: &[FileEntry],
    viewport_start: usize,
) -> FileManagerRowGeometry {
    let Some(areas) = file_manager_areas(area) else {
        return FileManagerRowGeometry::default();
    };
    compute_file_manager_row_geometry_in_content(
        panel_areas(areas.columns.current)[1],
        entries,
        viewport_start,
    )
}

/// Derive CURRENT row/action compatibility geometry from the exact content
/// rect already owned by the Miller frame snapshot.
#[cfg(test)]
pub(crate) fn compute_file_manager_row_geometry_in_content(
    list: Rect,
    entries: &[FileEntry],
    viewport_start: usize,
) -> FileManagerRowGeometry {
    let visible_rows = list.height as usize;
    let entry_count = entries.len();
    if list.width == 0 || visible_rows == 0 || entry_count == 0 {
        return FileManagerRowGeometry::default();
    }

    let start = viewport_start.min(entry_count.saturating_sub(visible_rows));
    let count = visible_rows.min(entry_count.saturating_sub(start));
    let visible_action_count = usize::from(list.width.saturating_sub(1) / ROW_ACTION_WIDTH)
        .min(FileManagerRowAction::ALL.len());
    let actions_width = visible_action_count as u16 * ROW_ACTION_WIDTH;
    let name_width = list.width.saturating_sub(actions_width);
    let mut rows = Vec::with_capacity(count);
    let mut actions = Vec::with_capacity(count.saturating_mul(visible_action_count));

    for offset in 0..count {
        let entry_idx = start + offset;
        let y = list.y.saturating_add(offset as u16);
        let name_rect = Rect::new(list.x, y, name_width, 1);
        rows.push(FileManagerRowArea {
            rect: name_rect,
            entry_idx,
            entry_path: entries[entry_idx].path.clone(),
        });
        for (action_idx, action) in FileManagerRowAction::ALL
            .iter()
            .copied()
            .take(visible_action_count)
            .enumerate()
        {
            actions.push(FileManagerRowActionArea {
                rect: Rect::new(
                    name_rect
                        .right()
                        .saturating_add(action_idx as u16 * ROW_ACTION_WIDTH),
                    y,
                    ROW_ACTION_WIDTH,
                    1,
                ),
                entry_idx,
                entry_path: entries[entry_idx].path.clone(),
                action,
            });
        }
    }

    FileManagerRowGeometry { rows, actions }
}

/// Build the persistent action-bar content from already-prepared FM state.
/// This is pure client presentation logic: no metadata or directory reads.
pub(crate) fn compute_file_manager_action_bar_model(
    file_manager: &FmState,
    clipboard: &[std::path::PathBuf],
    operation_in_flight: bool,
) -> FileManagerActionBarModel {
    let prepared_selection = prepare_file_manager_action_bar_selection(file_manager);
    let actions = FileManagerHeaderAction::ALL.map(|action| {
        let disabled_reason = if operation_in_flight {
            Some(FileManagerActionDisabledReason::OperationInFlight)
        } else {
            match action {
                FileManagerHeaderAction::Copy => prepared_selection.disabled_reason,
                FileManagerHeaderAction::Paste => {
                    if clipboard.is_empty() {
                        Some(FileManagerActionDisabledReason::EmptyClipboard)
                    } else if !file_manager.cwd_writable {
                        Some(FileManagerActionDisabledReason::ReadOnlyTarget)
                    } else {
                        None
                    }
                }
                FileManagerHeaderAction::NewFolder => {
                    Some(FileManagerActionDisabledReason::UnsupportedAction)
                }
                FileManagerHeaderAction::Delete => {
                    prepared_selection.disabled_reason.or_else(|| {
                        (!file_manager.cwd_writable)
                            .then_some(FileManagerActionDisabledReason::ReadOnlyTarget)
                    })
                }
            }
        };
        FileManagerActionState {
            action,
            enabled: disabled_reason.is_none(),
            disabled_reason,
        }
    });
    FileManagerActionBarModel {
        selection: prepared_selection.selection,
        clipboard_count: clipboard.len(),
        actions,
    }
}

struct PreparedFileManagerActionBarSelection {
    selection: Option<FileManagerActionBarSelection>,
    disabled_reason: Option<FileManagerActionDisabledReason>,
}

/// Resolve explicit path identities against the already-refreshed directory
/// snapshot. Visible entries retain Miller-list order; missing or ambiguous
/// identities stay visible in the prepared model but disable bulk operations.
fn prepare_file_manager_action_bar_selection(
    file_manager: &FmState,
) -> PreparedFileManagerActionBarSelection {
    let selected_paths = file_manager.multi_selection_paths();
    if selected_paths.is_empty() {
        return PreparedFileManagerActionBarSelection {
            selection: None,
            disabled_reason: Some(FileManagerActionDisabledReason::NoSelection),
        };
    }

    let mut seen_paths = BTreeSet::new();
    let mut live_entries = Vec::new();
    let mut ambiguous = false;
    for entry in &file_manager.entries {
        if selected_paths.contains(&entry.path) {
            if seen_paths.insert(entry.path.as_path()) {
                live_entries.push(entry);
            } else {
                ambiguous = true;
            }
        }
    }

    let mut ordered_paths = live_entries
        .iter()
        .map(|entry| entry.path.clone())
        .collect::<Vec<_>>();
    ordered_paths.extend(
        selected_paths
            .iter()
            .filter(|path| !seen_paths.contains(path.as_path()))
            .cloned(),
    );

    let stale = ambiguous || live_entries.len() != selected_paths.len();
    let unsupported = live_entries
        .iter()
        .any(|entry| !entry.operation_supported());
    let disabled_reason = if stale {
        Some(FileManagerActionDisabledReason::StaleSelection)
    } else if unsupported {
        Some(FileManagerActionDisabledReason::UnsupportedSelection)
    } else {
        None
    };

    let (label, kind) = if selected_paths.len() > 1 {
        (
            format!("{} selected", selected_paths.len()),
            FileManagerActionBarSelectionKind::Multiple,
        )
    } else if stale {
        let label = selected_paths
            .first()
            .and_then(|path| path.file_name())
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| String::from("unavailable selection"));
        (label, FileManagerActionBarSelectionKind::Unavailable)
    } else if let Some(entry) = live_entries.first() {
        let kind = if entry.is_dir() {
            FileManagerActionBarSelectionKind::Directory
        } else {
            FileManagerActionBarSelectionKind::File
        };
        (entry.display_name().into_owned(), kind)
    } else {
        // Defensive fallback for a future projection change that violates the
        // live-count invariant; operation authority already fails closed.
        (
            String::from("unavailable selection"),
            FileManagerActionBarSelectionKind::Unavailable,
        )
    };

    PreparedFileManagerActionBarSelection {
        selection: Some(FileManagerActionBarSelection {
            paths: ordered_paths,
            label,
            kind,
        }),
        disabled_reason,
    }
}

fn file_manager_action_bar_identity(cwd: &str, action_bar: &FileManagerActionBarModel) -> String {
    let mut identity = String::from(cwd);
    match action_bar.selection.as_ref() {
        Some(selection) => {
            identity.push_str(" — ");
            identity.push_str(&selection.label);
        }
        None => identity.push_str(" — no selection"),
    }
    if action_bar.clipboard_count > 0 {
        identity.push_str(" · clipboard: ");
        identity.push_str(&action_bar.clipboard_count.to_string());
    }
    identity
}

fn file_manager_operation_kind_label(kind: FileManagerOperationKind) -> &'static str {
    match kind {
        FileManagerOperationKind::Copy => "copy",
        FileManagerOperationKind::Move => "move",
        FileManagerOperationKind::Trash => "trash",
        FileManagerOperationKind::PermanentDelete => "delete",
        FileManagerOperationKind::Rename => "rename",
        FileManagerOperationKind::BulkRename => "bulk rename",
    }
}

fn file_manager_operation_summary(operation: &FileManagerOperationState) -> String {
    let kind = file_manager_operation_kind_label(operation.kind);
    let counts = format!("{}/{}", operation.completed_items, operation.total_items);
    let mut summary = match operation.status {
        FileManagerOperationStatus::Running => format!("{kind} {counts} · Esc cancel"),
        FileManagerOperationStatus::Completed => format!("{kind} completed {counts}"),
        FileManagerOperationStatus::Cancelled => format!("{kind} cancelled {counts}"),
        FileManagerOperationStatus::Partial => format!(
            "{kind} partial {counts} · {} failed",
            operation.failed_items
        ),
        FileManagerOperationStatus::Failed => {
            format!("{kind} failed {counts} · {} failed", operation.failed_items)
        }
    };
    if matches!(
        operation.status,
        FileManagerOperationStatus::Partial | FileManagerOperationStatus::Failed
    ) {
        if let Some(recovery_path) = operation
            .items
            .iter()
            .find_map(|item| item.recovery_path.as_ref())
        {
            summary.push_str(" · recovery: ");
            summary.push_str(&recovery_path.to_string_lossy());
        }
    }
    summary
}

fn file_manager_cwd_status_text(file_manager: &FmState) -> Option<&'static str> {
    match file_manager.cwd_status {
        FmDirectoryStatus::Missing => Some("cwd is missing"),
        FmDirectoryStatus::PermissionDenied => Some("cwd permission denied"),
        FmDirectoryStatus::Unavailable => Some("cwd is unavailable"),
        FmDirectoryStatus::Available if !file_manager.cwd_writable => Some("cwd is read-only"),
        FmDirectoryStatus::Available => None,
    }
}

fn file_manager_status_line(
    file_manager: &FmState,
    operation: Option<&FileManagerOperationState>,
    styles: FileManagerVisualStyles,
) -> Option<(String, Style)> {
    let mut text = operation.map(file_manager_operation_summary);
    if let Some(cwd_status) = file_manager_cwd_status_text(file_manager) {
        match text.as_mut() {
            Some(text) => {
                text.push_str(" · ");
                text.push_str(cwd_status);
            }
            None => text = Some(cwd_status.to_owned()),
        }
    }
    let style = if matches!(
        operation.map(|operation| operation.status),
        Some(FileManagerOperationStatus::Partial | FileManagerOperationStatus::Failed)
    ) || matches!(
        file_manager.cwd_status,
        FmDirectoryStatus::Missing
            | FmDirectoryStatus::PermissionDenied
            | FmDirectoryStatus::Unavailable
    ) {
        styles.error
    } else if !file_manager.cwd_writable {
        styles.warning
    } else {
        match operation.map(|operation| operation.status) {
            Some(FileManagerOperationStatus::Running) => styles.running,
            Some(FileManagerOperationStatus::Completed) => styles.completed,
            Some(FileManagerOperationStatus::Cancelled) => styles.cancelled,
            Some(FileManagerOperationStatus::Partial | FileManagerOperationStatus::Failed) => {
                styles.error
            }
            None => return None,
        }
    };
    text.map(|text| (text, style))
}

fn file_manager_current_empty_state(
    file_manager: &FmState,
    styles: FileManagerVisualStyles,
) -> (&'static str, Style) {
    match file_manager.cwd_status {
        FmDirectoryStatus::Available => ("(empty directory)", styles.empty),
        FmDirectoryStatus::Missing => ("(directory missing)", styles.error),
        FmDirectoryStatus::PermissionDenied => ("(permission denied)", styles.error),
        FmDirectoryStatus::Unavailable => ("(directory unavailable)", styles.error),
    }
}

/// Lay out complete native-FM header buttons from highest to lowest priority.
/// The cwd identity keeps a readable minimum; actions that cannot fit in full
/// are omitted so render/input can never expose a clipped phantom target.
pub(crate) fn compute_file_manager_header_action_areas(
    area: Rect,
) -> Vec<FileManagerHeaderActionArea> {
    if area.width == 0 || area.height == 0 {
        return Vec::new();
    }

    let available = area.width.saturating_sub(HEADER_MIN_IDENTITY_WIDTH);
    let mut selected = Vec::new();
    let mut used = 0_u16;
    for action in FileManagerHeaderAction::ALL {
        let width = action.label().len() as u16;
        let gap = if selected.is_empty() {
            0
        } else {
            HEADER_ACTION_GAP
        };
        let required = gap.saturating_add(width);
        if required > available.saturating_sub(used) {
            break;
        }
        used = used.saturating_add(required);
        selected.push((action, width));
    }

    let mut x = area.right().saturating_sub(used);
    selected
        .into_iter()
        .enumerate()
        .map(|(index, (action, width))| {
            if index > 0 {
                x = x.saturating_add(HEADER_ACTION_GAP);
            }
            let rect = Rect::new(x, area.y, width, 1);
            x = x.saturating_add(width);
            FileManagerHeaderActionArea { rect, action }
        })
        .collect()
}

/// Render the open file manager into `area`. Does nothing when the file manager
/// is closed (`app.file_manager` is `None`) or the area is empty, so callers can
/// invoke it unconditionally.
pub(crate) fn render_file_manager(app: &AppState, frame: &mut Frame, area: Rect) {
    let Some(fm) = app.file_manager.as_ref() else {
        return;
    };
    let Some([header_area, body_area, status_area]) = file_manager_frame_areas(area) else {
        return;
    };
    let p = &app.palette;
    let styles = file_manager_visual_styles(p);

    // Own the exact FM canvas so stale terminal cells cannot show through.
    // Block is paint-only; all layout and input authority stays in the pure
    // geometry helpers shared with compute_view.
    frame.render_widget(Block::default().style(styles.canvas), area);

    let fallback_action_bar;
    let action_bar = if area == app.view.terminal_area {
        app.view.file_manager_action_bar.as_ref()
    } else {
        // Unit/component callers can render without a preceding compute_view.
        fallback_action_bar = compute_file_manager_action_bar_model(
            fm,
            app.file_manager_clipboard.as_slice(),
            app.file_manager_operation
                .as_ref()
                .is_some_and(crate::app::state::FileManagerOperationState::is_running),
        );
        Some(&fallback_action_bar)
    };
    let fallback_header_actions;
    let header_actions = if area == app.view.terminal_area {
        app.view.file_manager_header_action_areas.as_slice()
    } else {
        // Use the exact same pure geometry seam as the full-frame path.
        fallback_header_actions = compute_file_manager_header_action_areas(area);
        fallback_header_actions.as_slice()
    };

    // A one-row identity header stays stable while responsive Miller columns
    // progressively disclose parent and preview context below it.
    let cwd_text = fm.cwd.to_string_lossy();
    let action_bar_identity = action_bar
        .map(|model| file_manager_action_bar_identity(&cwd_text, model))
        .unwrap_or_else(|| cwd_text.into_owned());
    let identity_width = header_actions
        .first()
        .map(|action| action.rect.x.saturating_sub(header_area.x))
        .unwrap_or(header_area.width);
    let identity_area = Rect::new(header_area.x, header_area.y, identity_width, 1);
    let header = truncate_end(&action_bar_identity, identity_area.width as usize);
    frame.render_widget(Paragraph::new(header).style(styles.identity), identity_area);
    for action in header_actions {
        let enabled = action_bar
            .and_then(|model| model.action_state(action.action))
            .is_some_and(|state| state.enabled);
        let style = if enabled {
            styles.enabled_action
        } else {
            styles.disabled_action
        };
        frame.render_widget(
            Paragraph::new(action.action.label()).style(style),
            action.rect,
        );
    }

    let live_files_view = area == app.view.terminal_area
        && app.stage.surface_view() == crate::ui::surface_host::StageSurfaceView::NativeFiles;
    let locations = live_files_view.then_some(&app.view.file_manager_locations);
    if let Some(locations) = locations {
        locations::render_file_manager_locations(app, frame, locations);
    }

    let fallback_trail;
    let trail = if live_files_view {
        &app.view.file_manager_trail
    } else {
        let trail_area = locations.map_or(body_area, |locations| locations.layout.trail);
        fallback_trail =
            trail_view::project_trail_view(trail_area, &fm.trail, &fm.trail_snapshots, &[]);
        &fallback_trail
    };
    trail_view::render_trail_view(app, frame, trail, &fm.trail_snapshots);
    render_file_manager_status(app, frame, status_area, fm, styles);
}

fn render_file_manager_status(
    app: &AppState,
    frame: &mut Frame,
    status_area: Rect,
    fm: &FmState,
    styles: FileManagerVisualStyles,
) {
    if status_area.width == 0 || status_area.height == 0 {
        return;
    }
    if let Some((status, style)) =
        file_manager_status_line(fm, app.file_manager_operation.as_ref(), styles)
    {
        let status = truncate_end(&format!(" {status}"), status_area.width as usize);
        frame.render_widget(Paragraph::new(status).style(style), status_area);
    }
}

pub(crate) fn agent_attachment_picker_rect(area: Rect) -> Option<Rect> {
    if area.width < 18 || area.height < 10 {
        return None;
    }
    centered_popup_rect(area, 96, 30)
}

#[derive(Debug, Clone, Copy)]
struct AgentAttachmentPickerAreas {
    popup: Rect,
    header: Rect,
    content: Rect,
    footer: Rect,
}

fn agent_attachment_picker_areas(area: Rect) -> Option<AgentAttachmentPickerAreas> {
    let popup = agent_attachment_picker_rect(area)?;
    if popup.width < 2 || popup.height < 2 {
        return None;
    }
    let inner = Rect::new(
        popup.x.saturating_add(1),
        popup.y.saturating_add(1),
        popup.width.saturating_sub(2),
        popup.height.saturating_sub(2),
    );
    let [header, content, footer] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(inner);
    Some(AgentAttachmentPickerAreas {
        popup,
        header,
        content,
        footer,
    })
}

pub(crate) fn agent_attachment_picker_visible_rows(area: Rect) -> usize {
    agent_attachment_picker_areas(area)
        .and_then(|areas| file_manager_areas(areas.content))
        .map(|areas| panel_areas(areas.columns.current)[1].height as usize)
        .unwrap_or(0)
}

pub(crate) fn compute_agent_attachment_picker_row_areas(
    area: Rect,
    entries: &[FileEntry],
    viewport_start: usize,
) -> Vec<FileManagerRowArea> {
    let Some(list) = agent_attachment_picker_areas(area)
        .and_then(|areas| file_manager_areas(areas.content))
        .map(|areas| panel_areas(areas.columns.current)[1])
    else {
        return Vec::new();
    };
    let visible_rows = list.height as usize;
    if list.width == 0 || visible_rows == 0 || entries.is_empty() {
        return Vec::new();
    }
    let start = viewport_start.min(entries.len().saturating_sub(visible_rows));
    entries
        .iter()
        .enumerate()
        .skip(start)
        .take(visible_rows)
        .map(|(entry_idx, entry)| FileManagerRowArea {
            rect: Rect::new(
                list.x,
                list.y
                    .saturating_add(entry_idx.saturating_sub(start) as u16),
                list.width,
                1,
            ),
            entry_idx,
            entry_path: entry.path.clone(),
        })
        .collect()
}

/// Paint the blocking picker above the terminal base. The private `FmState`
/// was prepared outside render; this function only consumes cached data.
pub(crate) fn render_agent_attachment_picker(app: &AppState, frame: &mut Frame, area: Rect) {
    let Some(picker) = app.agent_attachment_picker.as_ref() else {
        return;
    };
    let Some(areas) = agent_attachment_picker_areas(area) else {
        return;
    };
    if render_panel_shell(frame, areas.popup, app.palette.accent, app.palette.panel_bg).is_none() {
        return;
    }
    let target_current = app.agent_attachment_target_is_current(&picker.target);
    let (title, color) = if target_current {
        ("Attach file", app.palette.text)
    } else {
        ("Attach file · target unavailable", app.palette.peach)
    };
    frame.render_widget(
        Paragraph::new(title).style(Style::default().fg(color).add_modifier(Modifier::BOLD)),
        areas.header,
    );
    let fallback_rows;
    let rows = if area == app.view.terminal_area {
        app.view.agent_attachment_picker_row_areas.as_slice()
    } else {
        fallback_rows = compute_agent_attachment_picker_row_areas(
            area,
            &picker.file_manager.entries,
            picker.file_manager.viewport_start,
        );
        fallback_rows.as_slice()
    };
    render_agent_attachment_file_manager(app, &picker.file_manager, frame, areas.content, rows);
    frame.render_widget(
        Paragraph::new("Enter attach  Esc cancel").style(Style::default().fg(app.palette.overlay1)),
        areas.footer,
    );
}

fn render_agent_attachment_file_manager(
    app: &AppState,
    fm: &FmState,
    frame: &mut Frame,
    area: Rect,
    current_rows: &[FileManagerRowArea],
) {
    let Some(areas) = file_manager_areas(area) else {
        return;
    };
    let styles = file_manager_visual_styles(&app.palette);
    frame.render_widget(Block::default().style(styles.canvas), area);
    let cwd = truncate_end(&fm.cwd.to_string_lossy(), areas.header.width as usize);
    frame.render_widget(Paragraph::new(cwd).style(styles.identity), areas.header);
    for divider in areas.columns.dividers.into_iter().flatten() {
        frame.render_widget(
            Block::default()
                .borders(Borders::LEFT)
                .border_style(styles.divider),
            divider,
        );
    }
    if let Some(parent_area) = areas.columns.parent {
        if let Some(parent) = fm.parent.as_ref() {
            render_panel(
                app,
                frame,
                parent_area,
                "PARENT",
                &parent.entries,
                parent.cursor,
                "(empty)",
                None,
                None,
                None,
            );
        } else {
            render_panel(
                app,
                frame,
                parent_area,
                "PARENT",
                &[],
                None,
                "(root)",
                None,
                None,
                None,
            );
        }
    }
    let (empty, empty_style) = file_manager_current_empty_state(fm, styles);
    render_panel(
        app,
        frame,
        areas.columns.current,
        "CURRENT",
        &fm.entries,
        (!fm.entries.is_empty()).then_some(fm.cursor),
        empty,
        Some(empty_style),
        Some(current_rows),
        None,
    );
    if let Some(preview_area) = areas.columns.preview {
        match &fm.preview {
            FmPreview::None => render_panel(
                app,
                frame,
                preview_area,
                "PREVIEW",
                &[],
                None,
                "(nothing selected)",
                None,
                None,
                None,
            ),
            FmPreview::File(preview) => render_file_preview(app, frame, preview_area, preview),
            FmPreview::Directory(entries) => render_panel(
                app,
                frame,
                preview_area,
                "PREVIEW",
                entries,
                None,
                "(empty)",
                None,
                None,
                None,
            ),
        }
    }
}

fn render_file_preview(app: &AppState, frame: &mut Frame, area: Rect, preview: &FmFilePreview) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let p = &app.palette;
    let styles = file_manager_visual_styles(p);
    let [title_area, content_area] =
        Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(area);
    let title = truncate_end(" PREVIEW", title_area.width as usize);
    frame.render_widget(Paragraph::new(title).style(styles.panel_title), title_area);
    if content_area.height == 0 {
        return;
    }

    match preview {
        FmFilePreview::PendingText { .. } => {
            let label = truncate_end("  loading preview...", content_area.width as usize);
            frame.render_widget(Paragraph::new(label).style(styles.empty), content_area);
        }
        FmFilePreview::Unavailable(error) => {
            let label = truncate_end(
                &format!("  {}", text_preview_error_label(*error)),
                content_area.width as usize,
            );
            frame.render_widget(
                Paragraph::new(label).style(text_preview_error_style(*error, styles)),
                content_area,
            );
        }
        FmFilePreview::Text(preview) => {
            let (mut lines, truncated) = preview_lines(preview, p.subtext0);
            let available_rows = content_area.height as usize;
            let content_rows = if truncated {
                available_rows.saturating_sub(1)
            } else {
                available_rows
            };
            lines.truncate(content_rows);
            if truncated {
                lines.push(Line::styled(
                    "  (preview truncated)",
                    styles.warning.add_modifier(Modifier::ITALIC),
                ));
            }
            frame.render_widget(Paragraph::new(lines), content_area);
        }
        FmFilePreview::Image(preview) => {
            render_image_preview_status(app, frame, content_area, preview);
        }
    }
}

pub(super) fn render_image_preview_status(
    app: &AppState,
    frame: &mut Frame,
    area: Rect,
    preview: &crate::fm::FmImagePreview,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let styles = file_manager_visual_styles(&app.palette);
    let label_and_style = if !app.kitty_graphics_enabled {
        Some(("(Kitty graphics req.)", styles.warning))
    } else {
        match &preview.state {
            FmImagePreviewState::Pending => {
                Some(("(image preview pending)", styles.enabled_action))
            }
            FmImagePreviewState::Loading { .. } => {
                Some(("(loading image...)", styles.enabled_action))
            }
            FmImagePreviewState::Ready { .. } => None,
            FmImagePreviewState::Unavailable { error, .. } => Some((
                image_preview_error_label(*error),
                image_preview_error_style(*error, styles),
            )),
        }
    };
    let Some((label, style)) = label_and_style else {
        return;
    };
    let label = truncate_end(&format!("  {label}"), area.width as usize);
    frame.render_widget(Paragraph::new(label).style(style), area);
}

fn image_preview_error_style(error: ImagePreviewError, styles: FileManagerVisualStyles) -> Style {
    match error {
        ImagePreviewError::EncodedTooLarge { .. }
        | ImagePreviewError::DimensionsTooLarge { .. }
        | ImagePreviewError::PixelCountTooLarge { .. }
        | ImagePreviewError::DecodedBytesTooLarge { .. }
        | ImagePreviewError::OutputTooLarge { .. }
        | ImagePreviewError::UnsupportedFormat => styles.warning,
        ImagePreviewError::Io(_)
        | ImagePreviewError::NotRegularFile
        | ImagePreviewError::EmptyTarget
        | ImagePreviewError::ArithmeticOverflow
        | ImagePreviewError::DecodeFailed
        | ImagePreviewError::DecoderPanicked => styles.error,
    }
}

fn image_preview_error_label(error: ImagePreviewError) -> &'static str {
    match error {
        ImagePreviewError::Io(std::io::ErrorKind::PermissionDenied) => "(permission denied)",
        ImagePreviewError::EncodedTooLarge { .. }
        | ImagePreviewError::DimensionsTooLarge { .. }
        | ImagePreviewError::PixelCountTooLarge { .. }
        | ImagePreviewError::DecodedBytesTooLarge { .. }
        | ImagePreviewError::OutputTooLarge { .. } => "(image too large)",
        ImagePreviewError::UnsupportedFormat => "(unsupported image)",
        ImagePreviewError::DecodeFailed | ImagePreviewError::DecoderPanicked => {
            "(image decode failed)"
        }
        ImagePreviewError::Io(_)
        | ImagePreviewError::NotRegularFile
        | ImagePreviewError::EmptyTarget
        | ImagePreviewError::ArithmeticOverflow => "(image preview unavailable)",
    }
}

fn text_preview_error_label(error: TextPreviewError) -> &'static str {
    match error {
        TextPreviewError::Binary => "(binary file)",
        TextPreviewError::InvalidUtf8 { .. } => "(not UTF-8)",
        TextPreviewError::Io(std::io::ErrorKind::PermissionDenied) => "(permission denied)",
        TextPreviewError::Io(_) => "(preview unavailable)",
        TextPreviewError::NotRegularFile => "(not a regular file)",
    }
}

fn text_preview_error_style(error: TextPreviewError, styles: FileManagerVisualStyles) -> Style {
    match error {
        TextPreviewError::Binary | TextPreviewError::InvalidUtf8 { .. } => styles.warning,
        TextPreviewError::Io(_) | TextPreviewError::NotRegularFile => styles.error,
    }
}

fn preview_lines(preview: &TextPreview, fallback: Color) -> (Vec<Line<'static>>, bool) {
    if let Some(highlighted) = preview.highlighted.as_ref() {
        return highlighted_preview_lines(highlighted, fallback);
    }

    let mut source_lines = preview.content.lines();
    let mut lines = Vec::new();
    for _ in 0..MAX_RENDERED_PREVIEW_LINES {
        let Some(content) = source_lines.next() else {
            break;
        };
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(content.to_owned(), Style::default().fg(fallback)),
        ]));
    }
    let truncated_lines = source_lines.next().is_some();
    if lines.is_empty() {
        lines.push(Line::styled(
            "  (empty file)",
            Style::default().fg(fallback),
        ));
    }
    (lines, preview.truncated || truncated_lines)
}

fn highlighted_preview_lines(
    preview: &HighlightedTextPreview,
    fallback: Color,
) -> (Vec<Line<'static>>, bool) {
    let mut lines: Vec<Line<'static>> = preview
        .lines
        .iter()
        .take(MAX_RENDERED_PREVIEW_LINES)
        .map(|line| highlighted_preview_line(line, fallback))
        .collect();
    if lines.is_empty() {
        lines.push(Line::styled(
            "  (empty file)",
            Style::default().fg(fallback),
        ));
    }
    (
        lines,
        preview.truncated_bytes
            || preview.truncated_lines
            || preview.lines.len() > MAX_RENDERED_PREVIEW_LINES,
    )
}

fn highlighted_preview_line(line: &PreviewTextLine, fallback: Color) -> Line<'static> {
    let mut spans = Vec::with_capacity(line.spans.len().saturating_add(1));
    spans.push(Span::raw("  "));
    spans.extend(
        line.spans
            .iter()
            .map(|span| highlighted_preview_span(span, fallback)),
    );
    Line::from(spans)
}

fn highlighted_preview_span(span: &PreviewTextSpan, fallback: Color) -> Span<'static> {
    Span::styled(
        span.content.clone(),
        preview_text_style(span.style, fallback),
    )
}

fn preview_text_style(source: PreviewTextStyle, fallback: Color) -> Style {
    if source.is_plain() {
        return Style::default().fg(fallback);
    }
    let mut style = Style::default().fg(match source.foreground {
        Some([red, green, blue]) => Color::Rgb(red, green, blue),
        None => fallback,
    });
    let mut modifiers = Modifier::empty();
    if source.bold {
        modifiers |= Modifier::BOLD;
    }
    if source.italic {
        modifiers |= Modifier::ITALIC;
    }
    if source.underline {
        modifiers |= Modifier::UNDERLINED;
    }
    style = style.add_modifier(modifiers);
    style
}

fn render_panel(
    app: &AppState,
    frame: &mut Frame,
    area: Rect,
    title: &str,
    entries: &[FileEntry],
    selected: Option<usize>,
    empty_label: &str,
    empty_style: Option<Style>,
    row_areas: Option<&[FileManagerRowArea]>,
    multi_selected_paths: Option<&std::collections::BTreeSet<std::path::PathBuf>>,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let p = &app.palette;
    let styles = file_manager_visual_styles(p);
    let [title_area, list_area] = panel_areas(area);
    let title = truncate_end(&format!(" {title}"), title_area.width as usize);
    frame.render_widget(Paragraph::new(title).style(styles.panel_title), title_area);

    if list_area.height == 0 {
        return;
    }
    if entries.is_empty() {
        let label = truncate_end(&format!("  {empty_label}"), list_area.width as usize);
        frame.render_widget(
            Paragraph::new(label).style(empty_style.unwrap_or(styles.empty)),
            list_area,
        );
        return;
    }

    if let Some(row_areas) = row_areas {
        for row_area in row_areas {
            if let Some(entry) = entries.get(row_area.entry_idx) {
                render_entry_row(
                    app,
                    frame,
                    row_area.rect,
                    entry,
                    selected == Some(row_area.entry_idx),
                    multi_selected_paths.is_some_and(|paths| paths.contains(&entry.path)),
                );
            }
        }
        return;
    }

    // Context panels derive a cursor-following window because they do not own
    // independent scroll state.
    let rows = list_area.height as usize;
    let cursor = selected.unwrap_or(0).min(entries.len() - 1);
    let first = if cursor < rows { 0 } else { cursor - rows + 1 };

    for (offset, entry) in entries.iter().skip(first).take(rows).enumerate() {
        let idx = first + offset;
        let row = Rect::new(list_area.x, list_area.y + offset as u16, list_area.width, 1);
        render_entry_row(
            app,
            frame,
            row,
            entry,
            selected == Some(idx),
            multi_selected_paths.is_some_and(|paths| paths.contains(&entry.path)),
        );
    }
}

fn render_entry_row(
    app: &AppState,
    frame: &mut Frame,
    row: Rect,
    entry: &FileEntry,
    cursor_focused: bool,
    multi_selected: bool,
) {
    render_entry_row_clipped(
        app,
        frame,
        row,
        row.width,
        0,
        entry,
        cursor_focused,
        multi_selected,
    );
}

pub(crate) fn render_entry_row_clipped(
    app: &AppState,
    frame: &mut Frame,
    row: Rect,
    logical_width: u16,
    source_x: u16,
    entry: &FileEntry,
    cursor_focused: bool,
    multi_selected: bool,
) {
    if row.is_empty() {
        return;
    }
    let p = &app.palette;
    let styles = file_manager_visual_styles(p);
    let suffix = if entry.is_dir() { "/" } else { "" };
    let icon =
        crate::fm::entry_kind::visual_class(entry.kind, &entry.name).glyph(app.file_icon_profile);
    let full_label = truncate_end(
        &format!(" {icon} {}{}", entry.display_name(), suffix),
        logical_width as usize,
    );
    let label = display_cell_slice(&full_label, source_x, row.width);
    let style = if cursor_focused {
        styles.cursor
    } else if multi_selected {
        styles.multi_selection
    } else if entry.is_dir() {
        styles.directory
    } else {
        styles.file
    };
    frame.render_widget(Paragraph::new(label).style(style), row);
}

fn display_cell_slice(text: &str, source_x: u16, width: u16) -> String {
    let start = usize::from(source_x);
    let end = start.saturating_add(usize::from(width));
    let mut cell = 0usize;
    let mut visible = String::new();
    for ch in text.chars() {
        let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);
        let ch_start = cell;
        let ch_end = cell.saturating_add(ch_width);
        cell = ch_end;
        if ch_width == 0 {
            if ch_start >= start && ch_start < end {
                visible.push(ch);
            }
            continue;
        }
        if ch_end <= start {
            continue;
        }
        if ch_start >= end {
            break;
        }
        if ch_start < start || ch_end > end {
            visible.push(' ');
        } else {
            visible.push(ch);
        }
    }
    visible
}

fn render_row_action(
    app: &AppState,
    frame: &mut Frame,
    action_area: &FileManagerRowActionArea,
    cursor_focused: bool,
    multi_selected: bool,
) {
    let p = &app.palette;
    let style = if cursor_focused {
        Style::default().bg(p.surface0).fg(p.overlay1)
    } else if multi_selected {
        Style::default().bg(p.surface1).fg(p.overlay1)
    } else {
        Style::default().fg(p.overlay1)
    };
    frame.render_widget(
        Paragraph::new(action_area.action.label()).style(style),
        action_area.rect,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::FileManagerActionBarSelectionKind;
    use crate::fm::{
        FmDirectoryStatus, FmFilePreview, FmImagePreviewState, FmState, ImagePreviewTarget,
        PreparedImagePreview,
    };
    use ratatui::backend::TestBackend;
    use ratatui::buffer::Buffer;
    use ratatui::Terminal;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn unique() -> u64 {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        COUNTER.fetch_add(1, Ordering::Relaxed)
    }

    /// Isolated temp directory, recursively removed on drop.
    struct TempDir {
        root: PathBuf,
    }

    impl TempDir {
        fn new(tag: &str) -> Self {
            let root = std::env::temp_dir().join(format!(
                "herdr-fmrender-{}-{}-{}",
                std::process::id(),
                tag,
                unique()
            ));
            fs::create_dir_all(&root).expect("create temp root");
            Self { root }
        }
        fn file(&self, name: &str) {
            let path = self.root.join(name);
            fs::write(&path, b"x").expect("write temp file");
            Self::set_modified(&path);
        }
        fn dir(&self, name: &str) {
            let path = self.root.join(name);
            fs::create_dir_all(&path).expect("create temp dir");
            Self::set_modified(&path);
        }
        fn set_modified(path: &std::path::Path) {
            let modified = std::time::UNIX_EPOCH + std::time::Duration::from_secs(10);
            let entry = fs::File::open(path).expect("open temp entry");
            entry
                .set_times(fs::FileTimes::new().set_modified(modified))
                .expect("set temp entry mtime");
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn app_with_fm(fm: FmState) -> AppState {
        let mut app = AppState::test_new();
        app.file_manager = Some(fm);
        app
    }

    /// Render into a (w, h) TestBackend and return each row as a right-trimmed
    /// string.
    fn render_rows(app: &AppState, w: u16, h: u16) -> Vec<String> {
        let buffer = render_buffer(app, w, h);
        (0..h)
            .map(|y| {
                (0..w)
                    .map(|x| buffer[(x, y)].symbol().chars().next().unwrap_or(' '))
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            })
            .collect()
    }

    fn render_buffer(app: &AppState, w: u16, h: u16) -> Buffer {
        let mut terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
        terminal
            .draw(|frame| render_file_manager(app, frame, Rect::new(0, 0, w, h)))
            .unwrap();
        terminal.backend().buffer().clone()
    }

    fn find_rendered_text(buffer: &Buffer, w: u16, h: u16, needle: &str) -> (u16, u16) {
        for y in 0..h {
            let row = (0..w).map(|x| buffer[(x, y)].symbol()).collect::<String>();
            if let Some(byte_offset) = row.find(needle) {
                let x = row[..byte_offset].chars().count() as u16;
                return (x, y);
            }
        }
        panic!("rendered text {needle:?} not found");
    }

    fn geometry_entries(count: usize) -> Vec<FileEntry> {
        (0..count)
            .map(|entry_idx| FileEntry {
                name: format!("{entry_idx:02}.txt"),
                path: PathBuf::from("geometry-fixture").join(format!("{entry_idx:02}.txt")),
                kind: if false {
                    crate::fm::entry_kind::FileEntryKind::Directory
                } else {
                    crate::fm::entry_kind::FileEntryKind::RegularFile
                },
                modified: None,
            })
            .collect()
    }

    // P2.1: production render must consume the exact bounded Miller snapshot,
    // not independently reconstruct the legacy parent/current/preview trio.

    // TP-TRAIL-T7-RENDER-02: the live production-area render must consume the
    // exact Trail snapshot published by compute_view. Legacy component
    // characterization remains reachable only through the non-production
    // fallback until the T7.6 teardown.
    #[test]
    fn production_render_consumes_exact_trail_snapshot_without_legacy_placeholders() {
        let td = TempDir::new("production-trail-render");
        td.dir("alpha");
        td.file("notes.txt");
        let mut app = AppState::test_new();
        app.try_open_file_manager_with(|_| Some(FmState::new(&td.root)))
            .expect("Files activation");
        let frame = Rect::new(0, 0, 86, 8);
        let body = file_manager_miller_viewport_area(frame);
        app.view.terminal_area = frame;
        let fm = app.file_manager.as_ref().expect("open FM");
        app.view.file_manager_trail =
            trail_view::project_trail_view(body, &fm.trail, &fm.trail_snapshots, &[]);

        let rendered = render_rows(&app, frame.width, frame.height).join("\n");

        assert!(rendered.contains("alpha/"));
        assert!(rendered.contains("notes.txt"));
        assert!(!rendered.contains("CURRENT"));
        assert!(!rendered.contains("PREVIEW"));
    }

    // TP-TRAIL-T7-RENDER-04: production Trail paint is deterministic and
    // cannot mutate either FmState or the frame geometry it consumes.
    #[test]
    fn production_trail_render_is_byte_identical_and_state_pure() {
        let td = TempDir::new("production-trail-purity");
        td.dir("alpha");
        td.file("notes.txt");
        let mut app = AppState::test_new();
        app.try_open_file_manager_with(|_| Some(FmState::new(&td.root)))
            .expect("Files activation");
        let frame = Rect::new(0, 0, 86, 8);
        let body = file_manager_miller_viewport_area(frame);
        app.view.terminal_area = frame;
        let fm = app.file_manager.as_ref().expect("open FM");
        app.view.file_manager_trail =
            trail_view::project_trail_view(body, &fm.trail, &fm.trail_snapshots, &[]);
        let before_fm = {
            let fm = app.file_manager.as_ref().expect("open FM");
            (
                fm.cwd.clone(),
                fm.entries.clone(),
                fm.cursor,
                fm.viewport_start,
                fm.trail.clone(),
                fm.trail_snapshots.clone(),
            )
        };
        let before_view = app.view.file_manager_trail.clone();

        let first = render_buffer(&app, frame.width, frame.height);
        let second = render_buffer(&app, frame.width, frame.height);

        assert_eq!(first, second);
        let after_fm = {
            let fm = app.file_manager.as_ref().expect("open FM");
            (
                fm.cwd.clone(),
                fm.entries.clone(),
                fm.cursor,
                fm.viewport_start,
                fm.trail.clone(),
                fm.trail_snapshots.clone(),
            )
        };
        assert_eq!(after_fm, before_fm);
        assert_eq!(app.view.file_manager_trail, before_view);
    }

    // Component rendering consumes only loaded Trail snapshots.

    #[test]
    fn windowed_render_ignores_unprojected_chain_segments() {
        let current = PathBuf::from("/virtual/current");
        let far = PathBuf::from("/virtual/forbidden-far");
        let mut fm = FmState::test_empty(current.clone());
        fm.miller.chain = [
            far.clone(),
            PathBuf::from("/virtual/a"),
            PathBuf::from("/virtual/b"),
            PathBuf::from("/virtual/c"),
            PathBuf::from("/virtual/d"),
            current.clone(),
        ]
        .into_iter()
        .map(crate::fm::miller::MillerPathSegment::new)
        .collect();
        fm.miller.focused_directory = current.clone();
        fm.miller.visit(current);

        let rendered = render_rows(&app_with_fm(fm), 57, 6).join("\n");

        assert!(!rendered.contains("forbidden-far"));
        assert!(!rendered.contains("FORBIDDEN_FAR_MARKER"));
    }

    #[test]
    fn windowed_render_tiny_body_is_inert_and_target_free() {
        let app = app_with_fm(FmState::test_empty("/virtual/current"));
        for (width, height) in [(15, 6), (40, 2)] {
            let buffer = render_buffer(&app, width, height);
            let rendered = (0..height)
                .map(|y| {
                    (0..width)
                        .map(|x| buffer[(x, y)].symbol())
                        .collect::<String>()
                })
                .collect::<Vec<_>>()
                .join("\n");
            assert!(!rendered.contains("CURRENT"));
            assert!(!rendered.contains("PREVIEW"));
            assert!(!rendered.contains("PARENT"));
            assert!(
                (0..height).all(|y| (0..width).all(|x| buffer[(x, y)].symbol() != "│")),
                "degenerate body must expose no divider-like paint target"
            );
        }
    }

    #[test]
    fn windowed_render_is_byte_identical_and_state_pure() {
        let td = TempDir::new("windowed-purity");
        td.file("a.txt");
        td.dir("child");
        let app = app_with_fm(FmState::new(&td.root));
        let before_fm = {
            let fm = app.file_manager.as_ref().expect("open FM");
            (
                fm.cwd.clone(),
                fm.entries.clone(),
                fm.cursor,
                fm.viewport_start,
                fm.parent.clone(),
                fm.preview.clone(),
                fm.directory_generation,
                fm.preview_generation,
                fm.miller.clone(),
            )
        };
        let before_snapshot = app.view.file_manager_miller.clone();

        let first = render_buffer(&app, 86, 8);
        let second = render_buffer(&app, 86, 8);

        assert_eq!(first, second, "identical state must paint identical bytes");
        let after_fm = {
            let fm = app.file_manager.as_ref().expect("open FM");
            (
                fm.cwd.clone(),
                fm.entries.clone(),
                fm.cursor,
                fm.viewport_start,
                fm.parent.clone(),
                fm.preview.clone(),
                fm.directory_generation,
                fm.preview_generation,
                fm.miller.clone(),
            )
        };
        assert_eq!(after_fm, before_fm, "render cannot mutate FM state");
        assert_eq!(
            app.view.file_manager_miller, before_snapshot,
            "component fallback cannot publish or mutate ViewState"
        );
    }

    // Stale detail generations cannot leak prepared content.
    #[test]
    fn windowed_render_rejects_stale_current_and_preview_generation() {
        let td = TempDir::new("windowed-stale");
        fs::write(td.root.join("selected.txt"), "SECRET_PREVIEW")
            .expect("write stale preview fixture");
        let mut app = app_with_fm(FmState::new(&td.root));
        let frame = Rect::new(0, 0, 57, 6);
        app.view.terminal_area = frame;
        app.view.file_manager_miller = miller::project_miller_view(
            file_manager_miller_viewport_area(frame),
            app.file_manager.as_ref().expect("open FM"),
            1,
        );
        app.file_manager
            .as_mut()
            .expect("open FM")
            .preview_generation += 1;

        let rendered = render_rows(&app, frame.width, frame.height).join("\n");

        assert!(!rendered.contains("SECRET_PREVIEW"));
    }

    // Loaded Trail columns render root-to-active context without legacy titles.
    // TP-A2.2.1/2/3: a directory selection renders parent, current, and child
    // context side by side. Both the cwd in its parent and the selected child
    // in the current directory are visibly highlighted.

    // TP-B1.5-PLAIN-FALLBACK: prepared text remains visible before asynchronous
    // highlighting arrives. Highlighting is enhancement, never availability
    // authority.

    // TP-B1.5-STYLES: render-ready foreground and font modifiers map exactly
    // to Ratatui cells. Syntax preparation does not leak into render.

    // TP-B1.5-FAILURES: preparation failures have stable, distinct user-facing
    // states; none are confused with an empty directory or pending highlight.

    // TP-C6.4-THEME/EMPTY-ERROR: unsupported content/capability states are
    // warnings, while actual preview I/O/decode failures use the stronger error
    // role. Pending work remains muted and does not invent failure authority.

    #[test]
    fn image_preview_has_explicit_non_kitty_fallback_and_ready_content_is_clear() {
        let td = TempDir::new("image-fallback");
        fs::write(td.root.join("selected.png"), b"image candidate").expect("write image candidate");
        let mut fm = FmState::new(&td.root);

        let fallback = render_rows(&app_with_fm(fm.clone()), 80, 6).join("\n");
        assert!(fallback.contains("(Kitty graphics req.)"));

        match &mut fm.preview {
            FmPreview::File(FmFilePreview::Image(preview)) => {
                preview.state = FmImagePreviewState::Ready {
                    target: ImagePreviewTarget {
                        width_px: 96,
                        height_px: 64,
                    },
                    prepared: PreparedImagePreview {
                        width: 2,
                        height: 2,
                        data_fingerprint: 42,
                        rgba: vec![0xff; 16],
                    },
                };
            }
            other => panic!("expected image preview state, got {other:?}"),
        }
        let mut app = app_with_fm(fm);
        app.kitty_graphics_enabled = true;
        let ready = render_rows(&app, 80, 6).join("\n");
        assert!(!ready.contains("image preview"));
        assert!(!ready.contains("loading image"));
    }

    // TP-B1.5-TRUNCATION: both reader-byte and highlighter-line limits produce
    // an explicit marker inside the preview viewport.

    // TP-B1.5-LINE-LIMIT: a highlighter that stops at its independent line
    // budget exposes the same stable truncation marker as the byte reader.

    // TP-B1.5-COLUMN-BOUND: Paragraph clipping keeps an adversarial one-line
    // preview inside its Miller column; it does not wrap into extra rows or
    // write beyond the frame width.

    // TP-A2.2.4/N1: at the frozen preferred-width breakpoint, the two
    // one-cell dividers leave all three complete content columns readable.

    // TP-A2.2.4: current wins a one-column frame; the inline preview appears
    // only when both frozen 16-cell minima plus one divider fit.

    // TP-A3.2-VIEWPORT: CURRENT consumes the persistent viewport anchor rather
    // than deriving a new window from the cursor during the pure render pass.

    // TP-C2.1-MILLER-GEOMETRY: at each responsive breakpoint, every CURRENT
    // entry exposes one bounded name rect followed by the complete, ordered,
    // and pairwise-disjoint row-action set. Render and input must consume this
    // one geometry snapshot rather than independently recreating the split.
    #[test]
    fn current_row_actions_follow_miller_geometry_at_all_breakpoints() {
        use crate::app::state::FileManagerRowAction;

        for width in [20, 30, 40] {
            let area = Rect::new(5, 7, width, 6);
            let current_list = panel_areas(
                file_manager_areas(area)
                    .expect("non-empty FM geometry")
                    .columns
                    .current,
            )[1];

            let entries = geometry_entries(3);
            let geometry = compute_file_manager_row_geometry(area, &entries, 0);
            assert_eq!(geometry.rows.len(), 3, "width {width}");
            assert_eq!(geometry.actions.len(), 9, "width {width}");
            for (index, row) in geometry.rows.iter().enumerate() {
                assert_eq!(row.entry_idx, index, "width {width}");
                assert_eq!(row.rect.x, current_list.x, "width {width}");
                assert_eq!(row.rect.y, current_list.y + index as u16, "width {width}");
                assert_eq!(row.rect.height, 1, "width {width}");
                assert!(row.rect.width >= 1, "width {width}");

                let actions = geometry
                    .actions
                    .iter()
                    .filter(|action| action.entry_idx == index)
                    .collect::<Vec<_>>();
                assert_eq!(
                    actions.iter().map(|area| area.action).collect::<Vec<_>>(),
                    FileManagerRowAction::ALL,
                    "width {width}",
                );
                assert_eq!(row.rect.right(), actions[0].rect.x, "width {width}");
                assert_eq!(
                    actions.last().expect("delete action").rect.right(),
                    current_list.right()
                );
                for (action_index, action) in actions.iter().enumerate() {
                    assert_eq!(&action.entry_path, &entries[index].path, "width {width}");
                    assert_eq!(action.rect.width, ROW_ACTION_WIDTH, "width {width}");
                    assert_eq!(action.rect.height, 1, "width {width}");
                    assert_eq!(action.rect.y, row.rect.y, "width {width}");
                    assert_eq!(
                        action.rect.x,
                        row.rect.right() + action_index as u16 * ROW_ACTION_WIDTH,
                        "width {width}",
                    );
                    assert!(action.rect.x >= current_list.x, "width {width}");
                    assert!(action.rect.right() <= current_list.right(), "width {width}");
                }
                for (left_index, left) in actions.iter().enumerate() {
                    assert!(row.rect.intersection(left.rect).is_empty(), "width {width}");
                    for right in actions.iter().skip(left_index + 1) {
                        assert!(
                            left.rect.intersection(right.rect).is_empty(),
                            "width {width}"
                        );
                    }
                }
            }
        }
    }

    // TP-C2.1-VIEWPORT: viewport offsets map both name and action rects to
    // absolute entry indices, and adversarial offsets clamp to the last full
    // visible window instead of exposing stale paths.
    #[test]
    fn current_row_actions_apply_viewport_and_clamp_to_list_end() {
        use crate::app::state::FileManagerRowAction;

        let area = Rect::new(10, 20, 20, 6); // three CURRENT list rows + status

        let entries = geometry_entries(10);
        let geometry = compute_file_manager_row_geometry(area, &entries, 6);
        assert_eq!(
            geometry
                .rows
                .iter()
                .map(|row| row.entry_idx)
                .collect::<Vec<_>>(),
            vec![6, 7, 8]
        );
        let expected_name_width = area.width - FileManagerRowAction::ALL.len() as u16;
        assert_eq!(
            geometry.rows[0].rect,
            Rect::new(10, 22, expected_name_width, 1)
        );
        assert_eq!(
            geometry.rows[2].rect,
            Rect::new(10, 24, expected_name_width, 1)
        );
        assert_eq!(
            geometry
                .actions
                .chunks_exact(3)
                .map(|actions| actions[0].entry_idx)
                .collect::<Vec<_>>(),
            vec![6, 7, 8]
        );

        let clamped = compute_file_manager_row_geometry(area, &entries, usize::MAX);
        assert_eq!(
            clamped
                .rows
                .iter()
                .map(|row| row.entry_idx)
                .collect::<Vec<_>>(),
            vec![7, 8, 9]
        );
        assert!(clamped
            .actions
            .iter()
            .all(|action| (7..=9).contains(&action.entry_idx)));
    }

    // TP-C2.1-NARROW: actions disappear as complete one-cell units in priority
    // order while preserving at least one name cell. This prevents clipped
    // labels from leaving phantom hit targets at narrow widths.
    #[test]
    fn current_row_actions_progressively_hide_and_preserve_name_cell() {
        use crate::app::state::FileManagerRowAction;

        let cases = [(1, 0), (2, 1), (3, 2), (4, 3), (10, 3)];
        for (width, expected_action_count) in cases {
            let area = Rect::new(4, 8, width, 4);
            let entries = geometry_entries(1);
            let geometry = compute_file_manager_row_geometry(area, &entries, 0);
            assert_eq!(geometry.rows.len(), 1, "width {width}");
            assert_eq!(
                geometry.actions.len(),
                expected_action_count,
                "width {width}"
            );
            assert_eq!(
                geometry
                    .actions
                    .iter()
                    .map(|area| area.action)
                    .collect::<Vec<_>>(),
                FileManagerRowAction::ALL[..expected_action_count],
                "width {width}",
            );
            assert_eq!(
                geometry.rows[0].rect.width,
                width - expected_action_count as u16 * ROW_ACTION_WIDTH,
                "width {width}",
            );
            assert!(geometry.rows[0].rect.width >= 1, "width {width}");
        }
    }

    // TP-C2.1-DEGENERATE: headers, dividers, empty lists, and zero-sized
    // content expose neither name nor action targets.
    #[test]
    fn current_row_actions_are_empty_for_degenerate_geometry_or_list() {
        for area in [
            Rect::new(0, 0, 0, 6),
            Rect::new(0, 0, 20, 0),
            Rect::new(0, 0, 20, 1),
            Rect::new(0, 0, 20, 2),
        ] {
            let entries = geometry_entries(10);
            let geometry = compute_file_manager_row_geometry(area, &entries, 0);
            assert!(geometry.rows.is_empty());
            assert!(geometry.actions.is_empty());
        }
        let empty = compute_file_manager_row_geometry(
            Rect::new(0, 0, 20, 6),
            &geometry_entries(0),
            usize::MAX,
        );
        assert!(empty.rows.is_empty());
        assert!(empty.actions.is_empty());
    }

    // TP-C1.1-GEOMETRY: header actions are named, ordered, disjoint, and
    // derived from one responsive geometry seam shared by render and input.
    #[test]
    fn header_action_areas_are_tagged_disjoint_and_right_aligned() {
        use crate::app::state::FileManagerHeaderAction;

        let area = Rect::new(10, 4, 60, 1);
        let actions = compute_file_manager_header_action_areas(area);
        assert_eq!(
            actions.iter().map(|area| area.action).collect::<Vec<_>>(),
            vec![
                FileManagerHeaderAction::Copy,
                FileManagerHeaderAction::Paste,
                FileManagerHeaderAction::NewFolder,
                FileManagerHeaderAction::Delete,
            ]
        );
        assert_eq!(
            actions.last().expect("delete action").rect.right(),
            area.right()
        );
        assert!(actions.iter().all(|action| {
            action.rect.y == area.y
                && action.rect.height == 1
                && action.rect.width > 0
                && action.rect.x >= area.x
                && action.rect.right() <= area.right()
        }));
        for (index, left) in actions.iter().enumerate() {
            for right in actions.iter().skip(index + 1) {
                assert!(left.rect.intersection(right.rect).is_empty());
            }
        }
    }

    // TP-C1.1-RESPONSIVE: narrow and degenerate areas expose only complete,
    // highest-priority buttons and never leave clipped phantom hit targets.
    #[test]
    fn header_action_areas_progressively_hide_and_fail_closed() {
        use crate::app::state::FileManagerHeaderAction;

        let cases = [
            (
                60,
                vec![
                    FileManagerHeaderAction::Copy,
                    FileManagerHeaderAction::Paste,
                    FileManagerHeaderAction::NewFolder,
                    FileManagerHeaderAction::Delete,
                ],
            ),
            (
                30,
                vec![
                    FileManagerHeaderAction::Copy,
                    FileManagerHeaderAction::Paste,
                ],
            ),
            (18, vec![FileManagerHeaderAction::Copy]),
            (17, vec![]),
        ];
        for (width, expected) in cases {
            let actions = compute_file_manager_header_action_areas(Rect::new(3, 2, width, 1));
            assert_eq!(
                actions.iter().map(|area| area.action).collect::<Vec<_>>(),
                expected,
                "width {width}"
            );
        }

        assert!(compute_file_manager_header_action_areas(Rect::new(0, 0, 60, 0)).is_empty());
        assert!(
            compute_file_manager_header_action_areas(Rect::new(u16::MAX - 3, 2, 3, 1)).is_empty()
        );
    }

    // TP-C1.1-RENDER-SEAM: rendering consumes the same complete tagged rects
    // that compute_view snapshots for future input hit-testing.
    #[test]
    fn header_actions_render_from_shared_responsive_geometry() {
        let td = TempDir::new("header-actions");
        td.file("selected.txt");
        let app = app_with_fm(FmState::new(&td.root));

        let wide = render_rows(&app, 60, 5)[0].clone();
        for label in ["[copy]", "[paste]", "[new folder]", "[delete]"] {
            assert!(wide.contains(label), "wide header shows {label}: {wide:?}");
        }

        let narrow = render_rows(&app, 30, 5)[0].clone();
        assert!(narrow.contains("[copy]"), "narrow header: {narrow:?}");
        assert!(narrow.contains("[paste]"), "narrow header: {narrow:?}");
        assert!(
            !narrow.contains("[new folder]"),
            "narrow header: {narrow:?}"
        );
        assert!(!narrow.contains("[delete]"), "narrow header: {narrow:?}");
    }

    // TP-N4.2-BULK-AUTHORITY: cursor focus alone is not bulk authority; the
    // model carries explicit one/many paths in current visible order.
    #[test]
    fn action_bar_model_tracks_selection_kind_and_clipboard_content() {
        let td = TempDir::new("action-bar-model");
        td.dir("alpha-dir");
        td.file("beta.txt");
        let mut fm = FmState::new(&td.root);

        let cursor_only = compute_file_manager_action_bar_model(&fm, &[], false);
        assert!(cursor_only.selection.is_none());
        assert_eq!(cursor_only.clipboard_count, 0);

        assert!(fm.replace_selection(0));
        let directory = compute_file_manager_action_bar_model(&fm, &[], false);
        let selection = directory.selection.as_ref().expect("directory selection");
        assert_eq!(selection.label, "alpha-dir");
        assert_eq!(selection.paths, vec![td.root.join("alpha-dir")]);
        assert_eq!(selection.kind, FileManagerActionBarSelectionKind::Directory);
        assert_eq!(directory.clipboard_count, 0);

        assert!(fm.toggle_selection(1));
        let clipboard = vec![td.root.join("copied-one"), td.root.join("copied-two")];
        let multiple = compute_file_manager_action_bar_model(&fm, &clipboard, false);
        let selection = multiple.selection.as_ref().expect("multiple selection");
        assert_eq!(selection.label, "2 selected");
        assert_eq!(
            selection.paths,
            vec![td.root.join("alpha-dir"), td.root.join("beta.txt")]
        );
        assert_eq!(selection.kind, FileManagerActionBarSelectionKind::Multiple);
        assert_eq!(multiple.clipboard_count, 2);

        fm.clear_multi_selection();
        assert!(fm.replace_selection(1));
        let file = compute_file_manager_action_bar_model(&fm, &clipboard, false);
        let selection = file.selection.as_ref().expect("file selection");
        assert_eq!(selection.label, "beta.txt");
        assert_eq!(selection.paths, vec![td.root.join("beta.txt")]);
        assert_eq!(selection.kind, FileManagerActionBarSelectionKind::File);

        let empty = TempDir::new("action-bar-empty");
        let empty_model =
            compute_file_manager_action_bar_model(&FmState::new(&empty.root), &[], false);
        assert!(empty_model.selection.is_none());
        assert_eq!(empty_model.clipboard_count, 0);
    }

    // TP-C6.3-CATALOG: the durable cross-surface matrix is typed rather than
    // inferred from rendered labels. Actions without a v1 execution owner are
    // visible but disabled instead of advertising a silent no-op.
    #[test]
    fn file_manager_action_catalog_matches_supported_dispatch_seams() {
        use crate::app::state::{FileManagerContextMenuAction, FileManagerContextMenuModel};

        assert_eq!(
            FileManagerHeaderAction::ALL.map(FileManagerHeaderAction::label),
            ["[copy]", "[paste]", "[new folder]", "[delete]"]
        );
        assert_eq!(
            FileManagerRowAction::ALL.map(FileManagerRowAction::label),
            [">", "r", "x"]
        );
        assert_eq!(
            FileManagerContextMenuAction::ALL.map(|action| action.label().to_string()),
            [
                "Open".to_string(),
                "Copy".to_string(),
                "Rename".to_string(),
                "Delete".to_string(),
                "Compress".to_string(),
                "Add Reference to Agent...".to_string(),
            ]
        );

        let td = TempDir::new("action-catalog");
        td.file("selected.txt");
        let mut fm = FmState::new(&td.root);
        assert!(fm.replace_selection(0));
        let action_bar = compute_file_manager_action_bar_model(&fm, &[], false);
        assert!(
            !action_bar
                .action_state(FileManagerHeaderAction::NewFolder)
                .expect("new-folder catalog entry")
                .enabled,
            "New Folder has no v1 operation owner"
        );

        let context = FileManagerContextMenuModel::from_action_bar(&action_bar)
            .expect("single-selection context catalog");
        assert!(
            !context
                .items
                .iter()
                .find(|item| item.action == FileManagerContextMenuAction::Compress)
                .expect("compress catalog entry")
                .enabled,
            "Compress has no v1 operation owner"
        );
    }

    // TP-N4.2-BULK-AUTHORITY: prepared bulk paths follow current natural list
    // order rather than the BTreeSet's lexical path order.
    #[test]
    fn bulk_selection_paths_follow_current_visible_order() {
        let td = TempDir::new("bulk-selection-visible-order");
        td.file("file2.txt");
        td.file("file10.txt");
        let mut fm = FmState::new(&td.root);
        assert_eq!(
            fm.entries
                .iter()
                .map(|entry| entry.name.as_str())
                .collect::<Vec<_>>(),
            vec!["file2.txt", "file10.txt"]
        );
        assert!(fm.replace_selection(0));
        assert!(fm.toggle_selection(1));

        let model = compute_file_manager_action_bar_model(&fm, &[], false);
        assert_eq!(
            model.selection.expect("bulk selection").paths,
            vec![td.root.join("file2.txt"), td.root.join("file10.txt")]
        );
    }

    // TP-N4.2-BULK-AUTHORITY: the persistent header distinguishes no explicit
    // selection, one selected identity, and a many-selection count.
    #[test]
    fn action_bar_renders_selected_name_clipboard_count_and_empty_state() {
        let td = TempDir::new("action-bar-render");
        td.file("selected.txt");
        td.file("second.txt");
        let mut fm = FmState::new(&td.root);
        let mut app = app_with_fm(fm.clone());
        app.file_manager_clipboard = vec![td.root.join("copied.txt")];

        let cursor_only_header = render_rows(&app, 160, 5)[0].clone();
        assert!(
            cursor_only_header.contains("no selection"),
            "cursor-only header: {cursor_only_header:?}"
        );
        assert!(
            cursor_only_header.contains("clipboard: 1"),
            "cursor-only header: {cursor_only_header:?}"
        );

        let selected_index = fm
            .entries
            .iter()
            .position(|entry| entry.name == "selected.txt")
            .expect("selected fixture index");
        assert!(fm.replace_selection(selected_index));
        app.file_manager = Some(fm.clone());
        let selected_header = render_rows(&app, 160, 5)[0].clone();
        assert!(selected_header.contains("selected.txt"));

        let second_index = fm
            .entries
            .iter()
            .position(|entry| entry.name == "second.txt")
            .expect("second fixture index");
        assert!(fm.toggle_selection(second_index));
        app.file_manager = Some(fm);
        let multiple_header = render_rows(&app, 160, 5)[0].clone();
        assert!(multiple_header.contains("2 selected"));

        let empty = TempDir::new("action-bar-render-empty");
        let empty_app = app_with_fm(FmState::new(&empty.root));
        let empty_header = render_rows(&empty_app, 160, 5)[0].clone();
        assert!(
            empty_header.contains("no selection"),
            "empty header: {empty_header:?}"
        );
        assert!(!empty_header.contains("selected.txt"));
    }

    // TP-N4.2-BULK-AUTHORITY: every selected path must be live and supported;
    // target writability and in-flight precedence remain explicit.
    #[test]
    fn action_bar_authority_is_explicit_and_fail_closed() {
        use crate::app::state::FileManagerActionDisabledReason;

        let td = TempDir::new("action-authority");
        td.file("selected.txt");
        td.file("unsupported.txt");
        let mut fm = FmState::new(&td.root);

        let base = compute_file_manager_action_bar_model(&fm, &[], false);
        for action in [
            FileManagerHeaderAction::Copy,
            FileManagerHeaderAction::Delete,
        ] {
            assert_eq!(
                base.action_state(action)
                    .expect("selection action state")
                    .disabled_reason,
                Some(FileManagerActionDisabledReason::NoSelection)
            );
        }
        assert_eq!(
            base.action_state(FileManagerHeaderAction::Paste)
                .expect("paste state")
                .disabled_reason,
            Some(FileManagerActionDisabledReason::EmptyClipboard)
        );
        assert_eq!(
            base.action_state(FileManagerHeaderAction::NewFolder)
                .expect("new-folder state")
                .disabled_reason,
            Some(FileManagerActionDisabledReason::UnsupportedAction)
        );
        assert!(fm.replace_selection(0));
        let selected = compute_file_manager_action_bar_model(&fm, &[], false);
        assert!(
            selected
                .action_state(FileManagerHeaderAction::Copy)
                .expect("copy state")
                .enabled
        );
        assert!(
            selected
                .action_state(FileManagerHeaderAction::Delete)
                .expect("delete state")
                .enabled
        );

        let clipboard = vec![td.root.join("copied.txt")];
        let with_clipboard = compute_file_manager_action_bar_model(&fm, &clipboard, false);
        assert!(
            with_clipboard
                .action_state(FileManagerHeaderAction::Paste)
                .expect("paste state")
                .enabled
        );

        fm.cwd_writable = false;
        let read_only = compute_file_manager_action_bar_model(&fm, &clipboard, false);
        assert!(
            read_only
                .action_state(FileManagerHeaderAction::Copy)
                .expect("copy state")
                .enabled
        );
        for action in [
            FileManagerHeaderAction::Paste,
            FileManagerHeaderAction::Delete,
        ] {
            assert_eq!(
                read_only
                    .action_state(action)
                    .expect("write action state")
                    .disabled_reason,
                Some(FileManagerActionDisabledReason::ReadOnlyTarget)
            );
        }
        assert_eq!(
            read_only
                .action_state(FileManagerHeaderAction::NewFolder)
                .expect("new-folder state")
                .disabled_reason,
            Some(FileManagerActionDisabledReason::UnsupportedAction)
        );

        fm.cwd_writable = true;
        assert!(fm.toggle_selection(1));
        fm.entries[1].kind = crate::fm::entry_kind::FileEntryKind::UnsupportedSpecial;
        let unsupported = compute_file_manager_action_bar_model(&fm, &clipboard, false);
        for action in [
            FileManagerHeaderAction::Copy,
            FileManagerHeaderAction::Delete,
        ] {
            assert_eq!(
                unsupported
                    .action_state(action)
                    .expect("selection action state")
                    .disabled_reason,
                Some(FileManagerActionDisabledReason::UnsupportedSelection)
            );
        }

        fm.entries[1].kind = crate::fm::entry_kind::FileEntryKind::RegularFile;
        fm.entries.clear();
        let stale = compute_file_manager_action_bar_model(&fm, &clipboard, false);
        for action in [
            FileManagerHeaderAction::Copy,
            FileManagerHeaderAction::Delete,
        ] {
            assert_eq!(
                stale
                    .action_state(action)
                    .expect("stale selection action state")
                    .disabled_reason,
                Some(FileManagerActionDisabledReason::StaleSelection)
            );
        }

        let in_flight = compute_file_manager_action_bar_model(&fm, &clipboard, true);
        for action in FileManagerHeaderAction::ALL {
            assert_eq!(
                in_flight
                    .action_state(action)
                    .expect("in-flight action state")
                    .disabled_reason,
                Some(FileManagerActionDisabledReason::OperationInFlight)
            );
        }
    }

    // TP-N3.2-RENDER: disabled authority is visibly distinct and is sourced
    // from prepared action state rather than label presence or paint output.
    #[test]
    fn disabled_header_action_uses_distinct_style() {
        let td = TempDir::new("disabled-action-style");
        td.file("selected.txt");
        let mut fm = FmState::new(&td.root);
        assert!(fm.replace_selection(0));
        let enabled_app = app_with_fm(fm.clone());
        let enabled = render_buffer(&enabled_app, 160, 5);

        let mut disabled_app = app_with_fm(fm);
        disabled_app.file_manager_operation = Some(crate::app::state::FileManagerOperationState {
            generation: 1,
            kind: crate::app::state::FileManagerOperationKind::Copy,
            destination_directory: std::path::PathBuf::from("/tmp"),
            total_items: 1,
            completed_items: 0,
            failed_items: 0,
            status: crate::app::state::FileManagerOperationStatus::Running,
            items: Vec::new(),
        });
        let disabled = render_buffer(&disabled_app, 160, 5);

        let copy_x = compute_file_manager_header_action_areas(Rect::new(0, 0, 160, 5))[0]
            .rect
            .x;
        assert_ne!(enabled[(copy_x, 0)].fg, disabled[(copy_x, 0)].fg);
    }

    // Filesystem root does not synthesize an ancestor Trail column.
    // TP-A2.2.5: the filesystem root has no logical ancestor. The bounded
    // snapshot must not synthesize a fake parent column.
    #[test]
    fn filesystem_root_does_not_synthesize_parent_column() {
        let app = app_with_fm(FmState::new("/"));
        let rows = render_rows(&app, 40, 5);
        let joined = rows.join("\n");
        assert!(
            !joined.contains("PARENT")
                && !joined.contains("CURRENT")
                && !joined.contains("PREVIEW")
                && !joined.contains("(root)"),
            "no ancestor identity means no synthesized parent target: {rows:?}"
        );
    }

    // Cursor movement refreshes the selected directory's loaded Trail column.
    // TP-A2.2.3: moving the cursor refreshes the directory preview; stale child
    // contents from the previous selection must not survive.
    #[test]
    fn cursor_movement_refreshes_directory_preview() {
        let td = TempDir::new("preview-cursor");
        td.dir("alpha");
        td.dir("beta");
        fs::write(td.root.join("alpha").join("alpha-only.txt"), b"x").expect("write alpha preview");
        fs::write(td.root.join("beta").join("beta-only.txt"), b"x").expect("write beta preview");
        TempDir::set_modified(&td.root.join("alpha"));
        TempDir::set_modified(&td.root.join("beta"));
        let mut fm = FmState::new(&td.root);

        let alpha = render_rows(&app_with_fm(fm.clone()), 80, 6).join("\n");
        assert!(alpha.contains("alpha-only.txt"), "alpha preview: {alpha:?}");
        assert!(
            !alpha.contains("beta-only.txt"),
            "beta is not stale: {alpha:?}"
        );

        fm.move_down();
        let beta = render_rows(&app_with_fm(fm), 80, 6).join("\n");
        assert!(beta.contains("beta-only.txt"), "beta preview: {beta:?}");
        assert!(
            !beta.contains("alpha-only.txt"),
            "alpha is not stale: {beta:?}"
        );
    }

    // TP-A2.2: an open file manager renders its entries, directories first, each
    // group in natural order, directories marked with a trailing slash.

    // TP-A2.3: the cursor row is highlighted (surface0 background) while other
    // rows are not.

    // TP-N4.1-SELECTION-STATE: explicit multi-selection uses a distinct row
    // background, while cursor focus remains the unique stronger projection.
    #[test]
    fn multi_selection_rows_are_distinct_from_cursor_focus() {
        let td = TempDir::new("multi-selection-style");
        td.file("a.txt");
        td.file("b.txt");
        td.file("c.txt");
        let mut fm = FmState::new(&td.root);
        assert!(fm.replace_selection(0));
        assert!(fm.toggle_selection(2));
        assert!(fm.select(1));
        let app = app_with_fm(fm);
        let buffer = render_buffer(&app, 20, 6);
        let rows = (0..6)
            .map(|y| {
                (0..20)
                    .map(|x| buffer[(x, y)].symbol().chars().next().unwrap_or(' '))
                    .collect::<String>()
            })
            .collect::<Vec<_>>();
        let row_for = |name: &str| {
            rows.iter()
                .position(|row| row.contains(name))
                .expect("rendered entry row") as u16
        };

        assert_ne!(app.palette.surface0, app.palette.surface1);
        assert_eq!(buffer[(2, row_for("a.txt"))].bg, app.palette.surface1);
        assert_eq!(buffer[(2, row_for("c.txt"))].bg, app.palette.surface1);
        assert_eq!(buffer[(2, row_for("b.txt"))].bg, app.palette.surface0);
    }

    // TP-C6.4-THEME: the native FM owns its complete canvas background but
    // must not repaint cells outside the exact client rect supplied by layout.
    #[test]
    fn open_file_manager_fills_only_its_canvas_with_palette_background() {
        let td = TempDir::new("canvas-background");
        td.file("selected.txt");
        let app = app_with_fm(FmState::new(&td.root));
        let mut terminal = Terminal::new(TestBackend::new(26, 7)).unwrap();
        let area = Rect::new(2, 1, 22, 5);

        terminal
            .draw(|frame| render_file_manager(&app, frame, area))
            .unwrap();
        let buffer = terminal.backend().buffer();

        for y in area.y..area.bottom() {
            for x in area.x..area.right() {
                assert!(
                    [
                        app.palette.panel_bg,
                        app.palette.surface0,
                        app.palette.surface1,
                    ]
                    .contains(&buffer[(x, y)].bg),
                    "FM canvas cell ({x}, {y}) has an owned semantic background, got {:?}",
                    buffer[(x, y)].bg
                );
            }
        }
        for (x, y) in [(0, 0), (1, 1), (24, 1), (25, 6)] {
            assert_eq!(
                buffer[(x, y)].bg,
                Color::Reset,
                "outside cell ({x}, {y}) remains untouched"
            );
        }
    }

    // TP-C6.4-THEME: alternate palettes map state to semantic roles. Error or
    // selection authority is not inferred from literal RGB values or glyphs.

    // TP-C6.4-EMPTY-ERROR: an empty readable directory is not conflated with
    // missing, permission-denied, or otherwise unavailable cwd snapshots.

    // TP-C6.4-EMPTY-ERROR: read-only is prepared independently from directory
    // availability and remains visible without turning cursor focus into write
    // authority.
    #[test]
    fn read_only_current_directory_renders_warning_status_line() {
        let mut fm = FmState::test_empty("/virtual/locked");
        fm.cwd_writable = false;
        fm.cwd_status = FmDirectoryStatus::Available;
        let app = app_with_fm(fm);
        let buffer = render_buffer(&app, 80, 6);
        let (x, y) = find_rendered_text(&buffer, 80, 6, "cwd is read-only");

        assert_eq!(y, 5, "status owns the final FM row");
        assert_eq!(buffer[(x, y)].fg, app.palette.yellow);
    }

    fn operation_state(
        status: crate::app::state::FileManagerOperationStatus,
    ) -> crate::app::state::FileManagerOperationState {
        use crate::app::state::{
            FileManagerOperationItemState, FileManagerOperationItemStatus,
            FileManagerOperationKind, FileManagerOperationState,
        };

        FileManagerOperationState {
            generation: 7,
            kind: FileManagerOperationKind::Copy,
            destination_directory: PathBuf::from("/destination"),
            total_items: 4,
            completed_items: 2,
            failed_items: usize::from(matches!(
                status,
                crate::app::state::FileManagerOperationStatus::Partial
                    | crate::app::state::FileManagerOperationStatus::Failed
            )),
            status,
            items: vec![FileManagerOperationItemState {
                path: PathBuf::from("/source/alpha.txt"),
                recovery_path: matches!(
                    status,
                    crate::app::state::FileManagerOperationStatus::Partial
                        | crate::app::state::FileManagerOperationStatus::Failed
                )
                .then(|| PathBuf::from("/exact/recovery/alpha.txt")),
                status: FileManagerOperationItemStatus::Retained,
            }],
        }
    }

    // TP-C6.4-EMPTY-ERROR: operation lifecycle and recovery evidence map to
    // stable semantic colors and bounded copy on the dedicated status row.
    #[test]
    fn operation_status_line_renders_lifecycle_counts_and_exact_recovery_path() {
        use crate::app::state::FileManagerOperationStatus;

        let cases = [
            (
                FileManagerOperationStatus::Running,
                "copy 2/4",
                "Esc cancel",
            ),
            (
                FileManagerOperationStatus::Completed,
                "copy completed 2/4",
                "copy completed",
            ),
            (
                FileManagerOperationStatus::Cancelled,
                "copy cancelled 2/4",
                "copy cancelled",
            ),
            (
                FileManagerOperationStatus::Partial,
                "copy partial 2/4 · 1 failed",
                "recovery: /exact/recovery/alpha.txt",
            ),
            (
                FileManagerOperationStatus::Failed,
                "copy failed 2/4 · 1 failed",
                "recovery: /exact/recovery/alpha.txt",
            ),
        ];

        for (status, summary, evidence) in cases {
            let mut fm = FmState::test_empty("/virtual/operation");
            fm.cwd_writable = true;
            let mut app = app_with_fm(fm);
            app.file_manager_operation = Some(operation_state(status));
            let buffer = render_buffer(&app, 180, 7);
            let (x, y) = find_rendered_text(&buffer, 180, 7, summary);
            let expected_color = match status {
                FileManagerOperationStatus::Running => app.palette.yellow,
                FileManagerOperationStatus::Completed => app.palette.green,
                FileManagerOperationStatus::Cancelled => app.palette.peach,
                FileManagerOperationStatus::Partial | FileManagerOperationStatus::Failed => {
                    app.palette.red
                }
            };

            assert_eq!(y, 6, "operation status owns the final FM row");
            assert_eq!(buffer[(x, y)].fg, expected_color, "status: {status:?}");
            find_rendered_text(&buffer, 180, 7, evidence);
        }
    }

    // TP-N4.1-SELECTION-STATE: CURRENT paints exactly one cursor-focus style,
    // independently from any explicit multi-selection background.

    // TP-A2.4/C6.4: a valid empty directory renders its distinct placeholder
    // without being conflated with an unreadable cwd.

    // TP-A2.5: a name wider than the area is truncated with an ellipsis and never
    // overflows the row width.
    #[test]
    fn long_name_is_truncated_to_width() {
        let td = TempDir::new("long");
        td.file("this-is-a-very-long-file-name-that-exceeds-the-width.txt");
        let app = app_with_fm(FmState::new(&td.root));

        let width = 20u16;
        let rows = render_rows(&app, width, 4);
        for row in &rows {
            assert!(
                row.chars().count() <= width as usize,
                "row within width: {row:?}"
            );
        }
        // The entry row is ellipsized.
        assert!(
            rows.iter().any(|r| r.contains('…')),
            "long name ellipsized: {rows:?}"
        );
    }

    // TP-C2.1-UNICODE-RENDER: row geometry is display-cell based and remains
    // independent from UTF-8 byte length. A long Unicode name is truncated
    // inside the name rect while all complete action labels remain visible.

    // TP-A2.6 (render side): a closed file manager draws nothing, so the base
    // layer's `else` branch (the panes) is what shows.
    #[test]
    fn closed_file_manager_renders_nothing() {
        let app = AppState::test_new(); // file_manager is None
        let rows = render_rows(&app, 20, 4);
        assert!(
            rows.iter().all(|r| r.is_empty()),
            "closed FM leaves the area untouched: {rows:?}"
        );
    }

    // TP-M1.2-OVERLAY: R014 Clear-first ownership replaces background styling
    // inside a bounded responsive surface while keeping the base pane outside.
    #[test]
    fn attachment_picker_clear_overlay_is_responsive_and_blocks_background_input() {
        let td = TempDir::new("attachment-overlay");
        td.file("document.txt");
        let mut app = AppState::test_new();
        let mut workspace = crate::workspace::Workspace::test_new("attachment-overlay");
        workspace.identity_cwd = td.root.clone();
        let pane_id = workspace.tabs[0].root_pane;
        let terminal_id = workspace.tabs[0].panes[&pane_id]
            .attached_terminal_id
            .clone();
        app.workspaces = vec![workspace];
        app.active = Some(0);
        app.mode = crate::app::Mode::Terminal;
        app.ensure_test_terminals();
        app.terminals
            .get_mut(&terminal_id)
            .unwrap()
            .set_agent_name("codex".into());
        app.view.terminal_area = Rect::new(0, 0, 80, 24);
        app.open_agent_attachment_picker().unwrap();

        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        terminal
            .draw(|frame| {
                frame.render_widget(
                    Block::default().style(Style::default().bg(Color::Red)),
                    frame.area(),
                );
                render_agent_attachment_picker(&app, frame, frame.area());
            })
            .unwrap();

        let popup = agent_attachment_picker_rect(Rect::new(0, 0, 80, 24)).unwrap();
        let buffer = terminal.backend().buffer();
        assert_ne!(
            buffer[(popup.x + 1, popup.y + 1)].style().bg,
            Some(Color::Red)
        );
        let rendered = (popup.x..popup.right())
            .map(|x| buffer[(x, popup.y + 1)].symbol())
            .collect::<String>();
        assert!(rendered.contains("Attach file"), "header: {rendered:?}");

        assert!(agent_attachment_picker_rect(Rect::new(0, 0, 17, 10)).is_none());
        assert!(agent_attachment_picker_rect(Rect::new(0, 0, 18, 10)).is_some());
    }

    // Degenerate area must not panic.
    #[test]
    fn zero_area_is_panic_free() {
        let td = TempDir::new("zero");
        td.file("a.txt");
        let app = app_with_fm(FmState::new(&td.root));
        let mut terminal = Terminal::new(TestBackend::new(10, 3)).unwrap();
        terminal
            .draw(|frame| render_file_manager(&app, frame, Rect::new(0, 0, 0, 0)))
            .unwrap();
        // A one-row area has no room for the list body; still must not panic.
        terminal
            .draw(|frame| render_file_manager(&app, frame, Rect::new(0, 0, 10, 1)))
            .unwrap();
    }

    // TP-FIP-ICON-01/02: every entry row renders its prepared semantic icon
    // in one leading display cell before the name.
    #[test]
    fn entry_row_renders_semantic_icon_before_name() {
        use crate::fm::entry_kind::{FileEntryKind, IconProfile, VisualClass};
        let app = crate::app::state::AppState::test_new();
        let cases = [
            (
                "main.rs",
                FileEntryKind::RegularFile,
                VisualClass::SourceCode,
            ),
            ("assets", FileEntryKind::Directory, VisualClass::Directory),
            ("broken", FileEntryKind::BrokenSymlink, VisualClass::Broken),
        ];
        for (name, kind, class) in cases {
            let entry = crate::fm::FileEntry {
                name: name.to_string(),
                path: std::path::PathBuf::from("/x").join(name),
                kind,
                modified: None,
            };
            let backend = ratatui::backend::TestBackend::new(24, 1);
            let mut terminal = ratatui::Terminal::new(backend).expect("test terminal");
            terminal
                .draw(|frame| {
                    render_entry_row(&app, frame, Rect::new(0, 0, 24, 1), &entry, false, false);
                })
                .expect("draw entry row");
            let buffer = terminal.backend().buffer();
            assert_eq!(
                buffer[(1u16, 0u16)].symbol(),
                class.glyph(IconProfile::Nerd),
                "{name}: the icon cell must carry the semantic glyph"
            );
            assert_eq!(
                buffer[(3u16, 0u16)].symbol(),
                &name[0..1],
                "{name}: the name must follow icon + separator"
            );
        }
    }

    // TP-FIP-ICON-10 / VIS-03 seam: entry rows honor the client-local icon
    // profile so the deterministic ASCII fallback drives cross-machine
    // visual fixtures (Nerd private-use glyphs render empty in the browser).
    #[test]
    fn entry_row_honors_ascii_icon_profile() {
        use crate::fm::entry_kind::{FileEntryKind, IconProfile, VisualClass};
        let mut app = crate::app::state::AppState::test_new();
        app.file_icon_profile = IconProfile::Ascii;
        let entry = crate::fm::FileEntry {
            name: "main.rs".to_string(),
            path: std::path::PathBuf::from("/x/main.rs"),
            kind: FileEntryKind::RegularFile,
            modified: None,
        };
        let backend = TestBackend::new(24, 1);
        let mut terminal = Terminal::new(backend).expect("test terminal");
        terminal
            .draw(|frame| {
                render_entry_row(&app, frame, Rect::new(0, 0, 24, 1), &entry, false, false);
            })
            .expect("draw entry row");
        let buffer = terminal.backend().buffer();
        assert_eq!(
            buffer[(1u16, 0u16)].symbol(),
            VisualClass::SourceCode.glyph(IconProfile::Ascii),
            "the icon cell must follow the app icon profile"
        );
    }

    #[test]
    fn partial_leading_entry_row_renders_visible_label_suffix() {
        use crate::fm::entry_kind::{FileEntryKind, IconProfile};
        let mut app = crate::app::state::AppState::test_new();
        app.file_icon_profile = IconProfile::Ascii;
        let entry = crate::fm::FileEntry {
            name: "main.rs".to_string(),
            path: std::path::PathBuf::from("/x/main.rs"),
            kind: FileEntryKind::RegularFile,
            modified: None,
        };
        let backend = TestBackend::new(6, 1);
        let mut terminal = Terminal::new(backend).expect("test terminal");
        terminal
            .draw(|frame| {
                render_entry_row_clipped(
                    &app,
                    frame,
                    Rect::new(0, 0, 6, 1),
                    20,
                    4,
                    &entry,
                    false,
                    false,
                );
            })
            .expect("draw clipped entry row");
        let rendered = (0..6)
            .map(|x| terminal.backend().buffer()[(x, 0)].symbol())
            .collect::<String>();
        assert_eq!(rendered, "ain.rs");
    }

    // TP-FIP-ICON-08: a narrow column keeps the complete icon glyph and
    // truncates the name by display cells, never by bytes. Wide CJK cells
    // are asserted at glyph-start positions only (continuation cells skipped).
    #[test]
    fn narrow_column_keeps_complete_icon_and_truncates_name_by_display_cells() {
        use crate::fm::entry_kind::{FileEntryKind, IconProfile, VisualClass};
        let app = crate::app::state::AppState::test_new();
        let entry = crate::fm::FileEntry {
            name: "配置文件清单.toml".to_string(),
            path: std::path::PathBuf::from("/x/config-list.toml"),
            kind: FileEntryKind::RegularFile,
            modified: None,
        };
        let width = 10u16;
        let backend = TestBackend::new(width, 1);
        let mut terminal = Terminal::new(backend).expect("test terminal");
        terminal
            .draw(|frame| {
                render_entry_row(&app, frame, Rect::new(0, 0, width, 1), &entry, false, false);
            })
            .expect("draw entry row");
        let buffer = terminal.backend().buffer();
        assert_eq!(
            buffer[(1u16, 0u16)].symbol(),
            VisualClass::ConfigData.glyph(IconProfile::Nerd),
            "the icon must survive truncation complete"
        );
        assert_eq!(
            buffer[(3u16, 0u16)].symbol(),
            "配",
            "the name must start right after icon + separator"
        );
        assert_eq!(
            buffer[(9u16, 0u16)].symbol(),
            "…",
            "display-cell truncation must end the row with an ellipsis"
        );
    }

    // TP-FIP-ICON-08: row-action cells and the name/icon rect stay disjoint;
    // painting a long label never bleeds into the first action cell.
    #[test]
    fn icon_never_overlaps_row_action_cells() {
        let entries: Vec<crate::fm::FileEntry> = (0..3)
            .map(|i| crate::fm::FileEntry {
                name: format!("very-long-entry-name-that-overflows-{i}.rs"),
                path: std::path::PathBuf::from(format!("/x/very-long-{i}.rs")),
                kind: crate::fm::entry_kind::FileEntryKind::RegularFile,
                modified: None,
            })
            .collect();
        let list = Rect::new(0, 0, 30, 3);
        let geometry = compute_file_manager_row_geometry_in_content(list, &entries, 0);
        assert!(
            !geometry.actions.is_empty(),
            "fixture must expose at least one row action"
        );
        for row in &geometry.rows {
            assert!(
                row.rect.width > 2,
                "name rect must keep the icon column and separator"
            );
            for action in geometry
                .actions
                .iter()
                .filter(|action| action.rect.y == row.rect.y)
            {
                assert!(
                    row.rect.right() <= action.rect.x,
                    "name/icon cells and action cells must stay disjoint"
                );
            }
        }

        let app = crate::app::state::AppState::test_new();
        let name_rect = geometry.rows[0].rect;
        let first_action_x = geometry
            .actions
            .iter()
            .filter(|action| action.rect.y == name_rect.y)
            .map(|action| action.rect.x)
            .min()
            .expect("row 0 must own actions");
        let backend = TestBackend::new(30, 3);
        let mut terminal = Terminal::new(backend).expect("test terminal");
        terminal
            .draw(|frame| {
                render_entry_row(&app, frame, name_rect, &entries[0], false, false);
            })
            .expect("draw entry row");
        let buffer = terminal.backend().buffer();
        assert_eq!(
            buffer[(first_action_x, name_rect.y)].symbol(),
            " ",
            "a long label must never paint into an action cell"
        );
    }

    // TP-FIP-ICON-09: the cursor style owns the whole row including the icon
    // cell, and multi-selection stays visually distinct from the cursor.
    #[test]
    fn cursor_style_wins_over_icon_class_and_multi_select_stays_distinct() {
        let app = crate::app::state::AppState::test_new();
        let styles = file_manager_visual_styles(&app.palette);
        assert_ne!(
            styles.cursor, styles.multi_selection,
            "cursor and multi-selection must stay distinguishable"
        );
        let entry = crate::fm::FileEntry {
            name: "main.rs".to_string(),
            path: std::path::PathBuf::from("/x/main.rs"),
            kind: crate::fm::entry_kind::FileEntryKind::RegularFile,
            modified: None,
        };
        let render = |cursor_focused: bool, multi_selected: bool| {
            let backend = TestBackend::new(24, 1);
            let mut terminal = Terminal::new(backend).expect("test terminal");
            terminal
                .draw(|frame| {
                    render_entry_row(
                        &app,
                        frame,
                        Rect::new(0, 0, 24, 1),
                        &entry,
                        cursor_focused,
                        multi_selected,
                    );
                })
                .expect("draw entry row");
            terminal.backend().buffer().clone()
        };

        let focused = render(true, false);
        assert_eq!(focused[(1u16, 0u16)].style().bg, styles.cursor.bg);
        assert_eq!(focused[(1u16, 0u16)].style().fg, styles.cursor.fg);

        // Cursor wins even when the row is also inside the multi-selection.
        let focused_in_selection = render(true, true);
        assert_eq!(
            focused_in_selection[(1u16, 0u16)].style().bg,
            styles.cursor.bg
        );

        let multi = render(false, true);
        assert_eq!(multi[(1u16, 0u16)].style().bg, styles.multi_selection.bg);
        assert_eq!(multi[(1u16, 0u16)].style().fg, styles.multi_selection.fg);
    }

    // TP-FIP-ICON-13: a hostile file name containing control characters
    // renders as printable escapes and never shifts or clips row content.
    #[test]
    fn control_characters_in_name_render_escaped_and_do_not_shift_rows() {
        let app = crate::app::state::AppState::test_new();
        let hostile = crate::fm::FileEntry {
            name: "a\nb.rs".to_string(),
            path: std::path::PathBuf::from("/x/hostile"),
            kind: crate::fm::entry_kind::FileEntryKind::RegularFile,
            modified: None,
        };
        let below = crate::fm::FileEntry {
            name: "z.txt".to_string(),
            path: std::path::PathBuf::from("/x/z.txt"),
            kind: crate::fm::entry_kind::FileEntryKind::RegularFile,
            modified: None,
        };
        let backend = TestBackend::new(24, 2);
        let mut terminal = Terminal::new(backend).expect("test terminal");
        terminal
            .draw(|frame| {
                render_entry_row(&app, frame, Rect::new(0, 0, 24, 1), &hostile, false, false);
                render_entry_row(&app, frame, Rect::new(0, 1, 24, 1), &below, false, false);
            })
            .expect("draw entry rows");
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer[(3u16, 0u16)].symbol(), "a");
        assert_eq!(
            buffer[(4u16, 0u16)].symbol(),
            "\u{240a}",
            "a newline in the name must render as the printable ␊ escape"
        );
        assert_eq!(
            buffer[(5u16, 0u16)].symbol(),
            "b",
            "content after the control character must stay on the same row"
        );
        assert_eq!(
            buffer[(3u16, 1u16)].symbol(),
            "z",
            "the following row must stay unshifted"
        );
    }

    // TP-FIP-ICON-11: render consumes prepared entry data only. A path that
    // vanished after snapshot renders byte-identically to a live one.
    #[test]
    fn render_entry_row_performs_no_filesystem_io() {
        let app = crate::app::state::AppState::test_new();
        let td = TempDir::new("render-purity");
        let live_path = td.root.join("report.md");
        fs::write(&live_path, b"x").expect("write fixture");
        let live = crate::fm::FileEntry {
            name: "report.md".to_string(),
            path: live_path,
            kind: crate::fm::entry_kind::FileEntryKind::RegularFile,
            modified: None,
        };
        let gone = crate::fm::FileEntry {
            name: "report.md".to_string(),
            path: td.root.join("deleted-under-us").join("report.md"),
            kind: crate::fm::entry_kind::FileEntryKind::RegularFile,
            modified: None,
        };
        let render = |entry: &crate::fm::FileEntry| {
            let backend = TestBackend::new(24, 1);
            let mut terminal = Terminal::new(backend).expect("test terminal");
            terminal
                .draw(|frame| {
                    render_entry_row(&app, frame, Rect::new(0, 0, 24, 1), entry, false, false);
                })
                .expect("draw entry row");
            terminal.backend().buffer().clone()
        };
        assert_eq!(
            render(&live),
            render(&gone),
            "render output must not depend on filesystem state"
        );
    }

    fn fcl_render_item(
        label: &str,
        path: PathBuf,
        icon: crate::app::state::FileManagerLocationIcon,
    ) -> crate::app::state::FileManagerLocationItem {
        crate::app::state::FileManagerLocationItem {
            label: label.to_string(),
            path,
            icon,
            accessible: true,
            ejectable: false,
        }
    }

    fn flf_render_app(td: &TempDir) -> (AppState, PathBuf, PathBuf) {
        let home = td.root.join("home");
        let downloads = td.root.join("downloads");
        for path in [&home, &downloads] {
            fs::create_dir_all(path).expect("create FLF rendered location");
        }
        let mut app = AppState::test_new();
        app.try_open_file_manager_with(|_| Some(FmState::new(&td.root)))
            .expect("Files activation");
        app.mobile_width_threshold = 0;
        app.sidebar_collapsed = true;
        app.sidebar_collapsed_mode = crate::config::SidebarCollapsedModeConfig::Hidden;
        app.file_manager_locations_model =
            crate::app::state::FileManagerLocationsModel::from_sources(
                vec![
                    fcl_render_item(
                        "Home",
                        home.clone(),
                        crate::app::state::FileManagerLocationIcon::Home,
                    ),
                    fcl_render_item(
                        "Downloads",
                        downloads.clone(),
                        crate::app::state::FileManagerLocationIcon::Downloads,
                    ),
                ],
                Vec::new(),
                Vec::new(),
            );
        assert!(app
            .file_manager_locations
            .activate_location(&home, &app.file_manager_locations_model));
        assert!(app
            .file_manager_locations
            .select_cursor(&downloads, &app.file_manager_locations_model));
        (app, home, downloads)
    }

    fn flf_location_row(
        app: &AppState,
        path: &std::path::Path,
    ) -> crate::ui::file_manager::locations::FileManagerLocationRowArea {
        app.view
            .file_manager_locations
            .rows
            .iter()
            .find(|row| row.path == path)
            .cloned()
            .expect("projected location row")
    }

    // TP-FCL-RENDER-01/02: Native Files renders the prepared Favorites and
    // Locations model inside its content-local rail. Explicit origin,
    // in-flight root, and typed failure are visible presentation states and
    // never require render-time filesystem access.
    #[test]
    fn fcl_render_prepared_locations_with_origin_pending_and_failure_states() {
        let td = TempDir::new("fcl-locations-render");
        let home = td.root.join("home");
        let downloads = td.root.join("downloads");
        let root = td.root.join("root");
        for path in [&home, &downloads, &root] {
            fs::create_dir_all(path).expect("create rendered location");
        }
        let mut app = crate::app::state::AppState::test_new();
        app.try_open_file_manager_with(|_| Some(FmState::new(&td.root)))
            .expect("Files activation");
        app.mobile_width_threshold = 0;
        app.sidebar_collapsed = true;
        app.sidebar_collapsed_mode = crate::config::SidebarCollapsedModeConfig::Hidden;
        app.file_manager_locations_model =
            crate::app::state::FileManagerLocationsModel::from_sources(
                vec![
                    fcl_render_item(
                        "Home",
                        home.clone(),
                        crate::app::state::FileManagerLocationIcon::Home,
                    ),
                    fcl_render_item(
                        "Downloads",
                        downloads.clone(),
                        crate::app::state::FileManagerLocationIcon::Downloads,
                    ),
                ],
                Vec::new(),
                vec![fcl_render_item(
                    "Root",
                    root.clone(),
                    crate::app::state::FileManagerLocationIcon::Disk,
                )],
            );
        assert!(app
            .file_manager_locations
            .activate_location(&home, &app.file_manager_locations_model));
        let files_generation = app
            .stage
            .active_instance_generation()
            .expect("active Files generation");
        app.file_manager_locations.begin_load(
            downloads.clone(),
            files_generation,
            app.file_manager_locations_model.revision(),
            41,
            crate::app::state::FileManagerLocationNavigationIntent::FollowPreview,
        );
        let frame = Rect::new(0, 0, 90, 10);
        crate::ui::compute_view(&mut app, frame);

        let pending = render_buffer(&app, frame.width, frame.height);
        let pending_text = (0..frame.height)
            .map(|y| {
                (0..frame.width)
                    .map(|x| pending[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(pending_text.contains("FAVORITES"));
        assert!(pending_text.contains("LOCATIONS"));
        assert!(pending_text.contains("Home"));
        assert!(pending_text.contains("Downloads"));
        assert!(pending_text.contains("Root"));
        let pending_row = app
            .view
            .file_manager_locations
            .rows
            .iter()
            .find(|row| row.path == downloads)
            .expect("pending row geometry");
        assert_eq!(
            pending[(
                pending_row.rect.right().saturating_sub(1),
                pending_row.rect.y
            )]
                .symbol(),
            "…",
            "the exact pending root needs a visible row marker"
        );
        let home_pos = find_rendered_text(&pending, frame.width, frame.height, "Home");
        assert_eq!(
            pending[home_pos].fg, app.palette.accent,
            "the accepted origin keeps subdued accent identity"
        );
        assert!(pending[home_pos]
            .modifier
            .contains(Modifier::BOLD | Modifier::UNDERLINED));
        assert!(!pending[home_pos].modifier.contains(Modifier::REVERSED));

        app.file_manager_locations.fail_load(
            root.clone(),
            files_generation,
            app.file_manager_locations_model.revision(),
            crate::app::FileManagerLocationLoadError::PermissionDenied,
        );
        crate::ui::compute_view(&mut app, frame);
        let failed = render_buffer(&app, frame.width, frame.height);
        let failed_row = app
            .view
            .file_manager_locations
            .rows
            .iter()
            .find(|row| row.path == root)
            .expect("failed row geometry");
        assert_eq!(
            failed[(failed_row.rect.right().saturating_sub(1), failed_row.rect.y)].symbol(),
            "!",
            "the exact failed root needs a visible row marker"
        );
        let downloads_row = app
            .view
            .file_manager_locations
            .rows
            .iter()
            .find(|row| row.path == downloads)
            .expect("former pending row geometry");
        assert_ne!(
            failed[(
                downloads_row.rect.right().saturating_sub(1),
                downloads_row.rect.y
            )]
                .symbol(),
            "…",
            "failure transition retires the prior pending marker"
        );
    }

    // TP-FLF-VIS-01: the keyboard target is the strongest Rail identity;
    // accepted origin remains visible without impersonating current focus.
    #[test]
    fn flf_render_rail_cursor_wins_and_origin_remains_subdued() {
        let td = TempDir::new("flf-rail-cursor-origin");
        let (mut app, home, downloads) = flf_render_app(&td);
        let frame = Rect::new(0, 0, 90, 10);
        crate::ui::compute_view(&mut app, frame);

        let home_row = flf_location_row(&app, &home);
        let downloads_row = flf_location_row(&app, &downloads);
        let buffer = render_buffer(&app, frame.width, frame.height);
        let origin = &buffer[(home_row.rect.x, home_row.rect.y)];
        let cursor = &buffer[(downloads_row.rect.x, downloads_row.rect.y)];

        assert_eq!(cursor.bg, app.palette.accent);
        assert!(cursor
            .modifier
            .contains(Modifier::BOLD | Modifier::REVERSED));
        assert_eq!(origin.fg, app.palette.accent);
        assert!(origin
            .modifier
            .contains(Modifier::BOLD | Modifier::UNDERLINED));
        assert!(!origin.modifier.contains(Modifier::REVERSED));
    }

    // TP-FLF-NO-HIGHLIGHT-01: Trail identity stays resident while Rail owns
    // focus, but only the Rail cursor may receive the painted focus style.
    #[test]
    fn flf_render_rail_focus_suppresses_trail_cursor_style() {
        let td = TempDir::new("flf-suppress-trail-focus");
        td.file("only.txt");
        let mut app = AppState::test_new();
        app.try_open_file_manager_with(|_| Some(FmState::new(&td.root)))
            .expect("Files activation");
        app.mobile_width_threshold = 0;
        app.sidebar_collapsed = true;
        app.sidebar_collapsed_mode = crate::config::SidebarCollapsedModeConfig::Hidden;
        app.file_manager_locations.focus = crate::app::FileManagerLocationsFocus::Rail;
        let frame = Rect::new(0, 0, 90, 8);
        crate::ui::compute_view(&mut app, frame);

        let selected_rect = app
            .view
            .file_manager_trail
            .columns
            .iter()
            .find_map(|column| {
                let selected = column.selected_entry?;
                column
                    .rows
                    .iter()
                    .find(|row| row.entry_index == selected)
                    .map(|row| row.rect)
            })
            .expect("resident selected Trail row");
        let before_identity = app
            .file_manager
            .as_ref()
            .expect("open FM")
            .active_trail_entry_identity();
        let buffer = render_buffer(&app, frame.width, frame.height);
        let painted = &buffer[(selected_rect.x, selected_rect.y)];

        assert_eq!(painted.bg, app.palette.panel_bg);
        assert!(!painted.modifier.contains(Modifier::BOLD));
        assert_eq!(
            app.file_manager
                .as_ref()
                .expect("open FM")
                .active_trail_entry_identity(),
            before_identity,
            "paint suppression cannot erase Trail selection authority"
        );
    }

    // TP-FLF-VIS-01: ANSI/no-color users must distinguish cursor and origin
    // through modifiers even when every relevant foreground/background agrees.
    #[test]
    fn flf_render_no_color_distinguishes_cursor_from_origin_by_modifiers() {
        let td = TempDir::new("flf-no-color");
        let (mut app, home, downloads) = flf_render_app(&td);
        let monochrome = Color::Indexed(7);
        app.palette.accent = monochrome;
        app.palette.panel_bg = monochrome;
        app.palette.subtext0 = monochrome;
        let frame = Rect::new(0, 0, 90, 10);
        crate::ui::compute_view(&mut app, frame);

        let home_row = flf_location_row(&app, &home);
        let downloads_row = flf_location_row(&app, &downloads);
        let buffer = render_buffer(&app, frame.width, frame.height);
        let origin = &buffer[(home_row.rect.x, home_row.rect.y)];
        let cursor = &buffer[(downloads_row.rect.x, downloads_row.rect.y)];

        assert_eq!((cursor.fg, cursor.bg), (origin.fg, origin.bg));
        assert!(cursor
            .modifier
            .contains(Modifier::BOLD | Modifier::REVERSED));
        assert!(!cursor.modifier.contains(Modifier::UNDERLINED));
        assert!(origin
            .modifier
            .contains(Modifier::BOLD | Modifier::UNDERLINED));
        assert!(!origin.modifier.contains(Modifier::REVERSED));
    }

    // TP-FLF-FAIL-01/STALE-01: markers require current cursor, Files
    // generation, and model revision authority. Matching only a path is stale.
    #[test]
    fn flf_render_pending_failure_apply_only_to_current_cursor() {
        let td = TempDir::new("flf-marker-authority");
        let (mut app, home, downloads) = flf_render_app(&td);
        let frame = Rect::new(0, 0, 90, 10);
        crate::ui::compute_view(&mut app, frame);
        let home_row = flf_location_row(&app, &home);
        let downloads_row = flf_location_row(&app, &downloads);
        let files_generation = app
            .stage
            .active_instance_generation()
            .expect("active Files generation");
        let model_revision = app.file_manager_locations_model.revision();
        let marker =
            |app: &AppState,
             row: &crate::ui::file_manager::locations::FileManagerLocationRowArea| {
                let buffer = render_buffer(app, frame.width, frame.height);
                buffer[(row.rect.right().saturating_sub(1), row.rect.y)]
                    .symbol()
                    .to_string()
            };

        app.file_manager_locations.begin_load(
            home.clone(),
            files_generation,
            model_revision,
            1,
            crate::app::state::FileManagerLocationNavigationIntent::FollowPreview,
        );
        assert_ne!(marker(&app, &home_row), "…", "non-cursor pending is stale");
        app.file_manager_locations.begin_load(
            downloads.clone(),
            files_generation.wrapping_add(1),
            model_revision,
            2,
            crate::app::state::FileManagerLocationNavigationIntent::FollowPreview,
        );
        assert_ne!(
            marker(&app, &downloads_row),
            "…",
            "wrong Files generation cannot render pending"
        );
        app.file_manager_locations.begin_load(
            downloads.clone(),
            files_generation,
            model_revision,
            3,
            crate::app::state::FileManagerLocationNavigationIntent::FollowPreview,
        );
        assert_eq!(marker(&app, &downloads_row), "…");

        app.file_manager_locations.fail_load(
            home,
            files_generation,
            model_revision,
            crate::app::FileManagerLocationLoadError::PermissionDenied,
        );
        assert_ne!(marker(&app, &home_row), "!", "non-cursor failure is stale");
        app.file_manager_locations.fail_load(
            downloads.clone(),
            files_generation,
            model_revision.wrapping_add(1),
            crate::app::FileManagerLocationLoadError::PermissionDenied,
        );
        assert_ne!(
            marker(&app, &downloads_row),
            "!",
            "wrong model revision cannot render failure"
        );
        app.file_manager_locations.fail_load(
            downloads,
            files_generation,
            model_revision,
            crate::app::FileManagerLocationLoadError::PermissionDenied,
        );
        assert_eq!(marker(&app, &downloads_row), "!");
    }

    // TP-FLF-RENDER-01: focus changes may alter paint only. They cannot alter
    // state, row hit targets, column projection, or repeated output bytes.
    #[test]
    fn flf_render_is_state_pure_and_geometry_identical() {
        let td = TempDir::new("flf-render-purity");
        let (mut app, _home, downloads) = flf_render_app(&td);
        let frame = Rect::new(0, 0, 90, 10);
        app.file_manager_locations.focus_trail();
        crate::ui::compute_view(&mut app, frame);
        let trail_focus_locations = app.view.file_manager_locations.clone();
        let trail_focus_trail = app.view.file_manager_trail.clone();

        assert!(app
            .file_manager_locations
            .select_cursor(&downloads, &app.file_manager_locations_model));
        crate::ui::compute_view(&mut app, frame);
        assert_eq!(app.view.file_manager_locations, trail_focus_locations);
        assert_eq!(app.view.file_manager_trail, trail_focus_trail);
        let before_locations = app.file_manager_locations.clone();
        let before_file_manager = {
            let file_manager = app.file_manager.as_ref().expect("open FM");
            (
                file_manager.cwd.clone(),
                file_manager.entries.clone(),
                file_manager.cursor,
                file_manager.viewport_start,
                file_manager.directory_generation,
                file_manager.preview_generation,
                file_manager.trail.clone(),
                file_manager.trail_snapshots.clone(),
            )
        };
        let before_view = app.view.file_manager_locations.clone();

        let first = render_buffer(&app, frame.width, frame.height);
        let second = render_buffer(&app, frame.width, frame.height);

        assert_eq!(first, second, "identical state must paint identical cells");
        assert_eq!(app.file_manager_locations, before_locations);
        let after_file_manager = {
            let file_manager = app.file_manager.as_ref().expect("open FM");
            (
                file_manager.cwd.clone(),
                file_manager.entries.clone(),
                file_manager.cursor,
                file_manager.viewport_start,
                file_manager.directory_generation,
                file_manager.preview_generation,
                file_manager.trail.clone(),
                file_manager.trail_snapshots.clone(),
            )
        };
        assert_eq!(after_file_manager, before_file_manager);
        assert_eq!(app.view.file_manager_locations, before_view);
    }

    // TP-FLF-COMPACT-01/VIS-01: the drawer is the compact Rail owner and
    // paints the same strong cursor semantics as the persistent wide Rail.
    #[test]
    fn flf_compact_drawer_focus_matches_wide_rail() {
        let td = TempDir::new("flf-compact-focus");
        let (mut app, _home, downloads) = flf_render_app(&td);
        let wide_frame = Rect::new(0, 0, 90, 10);
        crate::ui::compute_view(&mut app, wide_frame);
        let wide_row = flf_location_row(&app, &downloads);
        let wide = render_buffer(&app, wide_frame.width, wide_frame.height);
        let wide_style = (
            wide[(wide_row.rect.x, wide_row.rect.y)].fg,
            wide[(wide_row.rect.x, wide_row.rect.y)].bg,
            wide[(wide_row.rect.x, wide_row.rect.y)].modifier,
        );

        assert!(app.file_manager_locations.open_drawer());
        let compact_frame = Rect::new(
            0,
            0,
            locations::COMPACT_CONTENT_THRESHOLD.saturating_sub(1),
            10,
        );
        crate::ui::compute_view(&mut app, compact_frame);
        assert_eq!(
            app.view.file_manager_locations.layout.mode,
            locations::FileManagerLocationsMode::Compact
        );
        assert!(app.view.file_manager_locations.drawer_area.is_some());
        let compact_row = flf_location_row(&app, &downloads);
        let compact = render_buffer(&app, compact_frame.width, compact_frame.height);
        let compact_style = (
            compact[(compact_row.rect.x, compact_row.rect.y)].fg,
            compact[(compact_row.rect.x, compact_row.rect.y)].bg,
            compact[(compact_row.rect.x, compact_row.rect.y)].modifier,
        );

        assert_eq!(compact_style, wide_style);
        assert!(compact_style
            .2
            .contains(Modifier::BOLD | Modifier::REVERSED));
    }
}
