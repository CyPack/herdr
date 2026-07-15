//! Native file manager — Miller-capable directory-list render (A2.2).
//!
//! Draws the open [`FmState`](crate::fm::FmState) into a rect: a one-row current
//! directory header followed by responsive parent/current/preview columns. Pure
//! draw (reads state, never mutates or touches the filesystem), matching herdr's
//! `compute_view`/`render` split. Client-side presentation only.
//!
//! This is the first non-terminal *content* swapped into a named region
//! (`CenterContent`): when `app.file_manager` is open, the base layer draws this
//! here instead of the terminal panes. Text/image previews and row-action
//! geometry build on the same pure client-side projection.

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;
use std::collections::BTreeSet;

use super::text::truncate_end;
use crate::app::state::AppState;
use crate::app::state::{
    FileManagerActionBarModel, FileManagerActionBarSelection, FileManagerActionBarSelectionKind,
    FileManagerActionDisabledReason, FileManagerActionState, FileManagerHeaderAction,
    FileManagerHeaderActionArea, FileManagerRowAction, FileManagerRowActionArea,
    FileManagerRowArea,
};
use crate::fm::{
    FileEntry, FmFilePreview, FmImagePreviewState, FmPreview, FmState, HighlightedTextPreview,
    ImagePreviewError, PreviewTextLine, PreviewTextSpan, PreviewTextStyle, TextPreview,
    TextPreviewError,
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
    if area.width == 0 || area.height == 0 {
        return None;
    }
    let [header, body] = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(area);
    Some(FileManagerAreas {
        header,
        columns: miller_layout(body),
    })
}

fn panel_areas(area: Rect) -> [Rect; 2] {
    Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(area)
}

/// Number of rows available to CURRENT entries for this responsive FM area.
/// `compute_view` uses the same pure geometry that render consumes.
pub(crate) fn file_manager_visible_rows(area: Rect) -> usize {
    file_manager_areas(area)
        .map(|areas| panel_areas(areas.columns.current)[1].height as usize)
        .unwrap_or(0)
}

/// Pixel graphics and text rendering share this exact PREVIEW content seam.
/// The top-level FM header and the PREVIEW panel title are intentionally
/// excluded so host graphics cannot cover either label.
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

pub(crate) fn compute_file_manager_row_geometry(
    area: Rect,
    entries: &[FileEntry],
    viewport_start: usize,
) -> FileManagerRowGeometry {
    let Some(areas) = file_manager_areas(area) else {
        return FileManagerRowGeometry::default();
    };
    let list = panel_areas(areas.columns.current)[1];
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
    let unsupported = live_entries.iter().any(|entry| !entry.operation_supported);
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
        let kind = if entry.is_dir {
            FileManagerActionBarSelectionKind::Directory
        } else {
            FileManagerActionBarSelectionKind::File
        };
        (entry.name.clone(), kind)
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
    let Some(areas) = file_manager_areas(area) else {
        return;
    };
    let p = &app.palette;

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
        .map(|action| action.rect.x.saturating_sub(areas.header.x))
        .unwrap_or(areas.header.width);
    let identity_area = Rect::new(areas.header.x, areas.header.y, identity_width, 1);
    let header = truncate_end(&action_bar_identity, identity_area.width as usize);
    frame.render_widget(
        Paragraph::new(header).style(Style::default().fg(p.subtext0).add_modifier(Modifier::BOLD)),
        identity_area,
    );
    for action in header_actions {
        let enabled = action_bar
            .and_then(|model| model.action_state(action.action))
            .is_some_and(|state| state.enabled);
        let style = if enabled {
            Style::default().fg(p.overlay1)
        } else {
            Style::default().fg(p.overlay0).add_modifier(Modifier::DIM)
        };
        frame.render_widget(
            Paragraph::new(action.action.label()).style(style),
            action.rect,
        );
    }

    if areas.columns.current.height == 0 {
        return;
    }

    let layout = areas.columns;
    let fallback_geometry;
    let (current_rows, current_actions) = if area == app.view.terminal_area {
        (
            app.view.file_manager_row_areas.as_slice(),
            app.view.file_manager_row_action_areas.as_slice(),
        )
    } else {
        // Unit-level/component callers can render into an arbitrary rect
        // without a preceding full-frame compute_view pass.
        fallback_geometry = compute_file_manager_row_geometry(area, &fm.entries, fm.viewport_start);
        (
            fallback_geometry.rows.as_slice(),
            fallback_geometry.actions.as_slice(),
        )
    };
    for divider in layout.dividers.into_iter().flatten() {
        frame.render_widget(
            Block::default()
                .borders(Borders::LEFT)
                .border_style(Style::default().fg(p.surface1)),
            divider,
        );
    }

    if let Some(parent_area) = layout.parent {
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
            );
        }
    }

    render_panel(
        app,
        frame,
        layout.current,
        "CURRENT",
        &fm.entries,
        (!fm.entries.is_empty()).then_some(fm.cursor),
        "(empty)",
        Some(current_rows),
        Some(fm.multi_selection_paths()),
    );
    for action_area in current_actions {
        if let Some(entry) = fm.entries.get(action_area.entry_idx) {
            render_row_action(
                app,
                frame,
                action_area,
                fm.cursor == action_area.entry_idx,
                fm.multi_selection_paths().contains(&entry.path),
            );
        }
    }

    if let Some(preview_area) = layout.preview {
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
            ),
            FmPreview::File(preview) => {
                render_file_preview(app, frame, preview_area, preview);
            }
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
            ),
        }
    }
}

fn render_file_preview(app: &AppState, frame: &mut Frame, area: Rect, preview: &FmFilePreview) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let p = &app.palette;
    let [title_area, content_area] =
        Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(area);
    let title = truncate_end(" PREVIEW", title_area.width as usize);
    frame.render_widget(
        Paragraph::new(title).style(Style::default().fg(p.overlay1).add_modifier(Modifier::BOLD)),
        title_area,
    );
    if content_area.height == 0 {
        return;
    }

    match preview {
        FmFilePreview::Unavailable(error) => {
            let label = truncate_end(
                &format!("  {}", text_preview_error_label(*error)),
                content_area.width as usize,
            );
            frame.render_widget(
                Paragraph::new(label).style(Style::default().fg(p.overlay1)),
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
                    Style::default()
                        .fg(p.overlay1)
                        .add_modifier(Modifier::ITALIC),
                ));
            }
            frame.render_widget(Paragraph::new(lines), content_area);
        }
        FmFilePreview::Image(preview) => {
            let label = if !app.kitty_graphics_enabled {
                Some("(Kitty graphics req.)")
            } else {
                match &preview.state {
                    FmImagePreviewState::Pending => Some("(image preview pending)"),
                    FmImagePreviewState::Loading { .. } => Some("(loading image...)"),
                    FmImagePreviewState::Ready { .. } => None,
                    FmImagePreviewState::Unavailable { error, .. } => {
                        Some(image_preview_error_label(*error))
                    }
                }
            };
            let Some(label) = label else {
                return;
            };
            let label = truncate_end(&format!("  {label}"), content_area.width as usize);
            frame.render_widget(
                Paragraph::new(label).style(Style::default().fg(p.overlay1)),
                content_area,
            );
        }
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
    row_areas: Option<&[FileManagerRowArea]>,
    multi_selected_paths: Option<&std::collections::BTreeSet<std::path::PathBuf>>,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let p = &app.palette;
    let [title_area, list_area] = panel_areas(area);
    let title = truncate_end(&format!(" {title}"), title_area.width as usize);
    frame.render_widget(
        Paragraph::new(title).style(Style::default().fg(p.overlay1).add_modifier(Modifier::BOLD)),
        title_area,
    );

    if list_area.height == 0 {
        return;
    }
    if entries.is_empty() {
        let label = truncate_end(&format!("  {empty_label}"), list_area.width as usize);
        frame.render_widget(
            Paragraph::new(label).style(Style::default().fg(p.overlay1)),
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
    let p = &app.palette;
    let suffix = if entry.is_dir { "/" } else { "" };
    let label = truncate_end(&format!("  {}{}", entry.name, suffix), row.width as usize);
    let style = if cursor_focused {
        Style::default()
            .bg(p.surface0)
            .fg(p.text)
            .add_modifier(Modifier::BOLD)
    } else if multi_selected {
        Style::default()
            .bg(p.surface1)
            .fg(p.text)
            .add_modifier(Modifier::BOLD)
    } else if entry.is_dir {
        Style::default().fg(p.blue).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(p.subtext0)
    };
    frame.render_widget(Paragraph::new(label).style(style), row);
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
        FmFilePreview, FmImagePreviewState, FmState, HighlightedTextPreview, ImagePreviewTarget,
        PreparedImagePreview, PreviewTextLine, PreviewTextSpan, PreviewTextStyle, TextPreviewError,
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
            fs::write(self.root.join(name), b"x").expect("write temp file");
        }
        fn dir(&self, name: &str) {
            fs::create_dir_all(self.root.join(name)).expect("create temp dir");
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
                is_dir: false,
                operation_supported: true,
            })
            .collect()
    }

    // TP-A2.2.1/2/3: a directory selection renders parent, current, and child
    // context side by side. Both the cwd in its parent and the selected child
    // in the current directory are visibly highlighted.
    #[test]
    fn miller_columns_render_parent_current_and_directory_preview() {
        let td = TempDir::new("miller");
        td.dir("work");
        td.file("parent-peer.txt");
        fs::create_dir_all(td.root.join("work").join("child")).expect("create child dir");
        fs::write(td.root.join("work").join("current.txt"), b"x").expect("write current file");
        fs::write(td.root.join("work").join("child").join("preview.txt"), b"x")
            .expect("write preview file");

        let app = app_with_fm(FmState::new(td.root.join("work")));
        let rows = render_rows(&app, 80, 8);
        let joined = rows.join("\n");

        assert!(joined.contains("PARENT"), "parent title: {rows:?}");
        assert!(joined.contains("CURRENT"), "current title: {rows:?}");
        assert!(joined.contains("PREVIEW"), "preview title: {rows:?}");
        assert!(joined.contains("work/"), "cwd shown in parent: {rows:?}");
        assert!(
            joined.contains("current.txt"),
            "current entries shown: {rows:?}"
        );
        assert!(
            joined.contains("preview.txt"),
            "selected directory contents shown: {rows:?}"
        );

        let buffer = render_buffer(&app, 80, 8);
        assert_eq!(
            buffer[(2, 2)].bg,
            app.palette.surface0,
            "cwd row is highlighted in the parent column"
        );
        let first_divider = (0..80)
            .find(|&x| buffer[(x, 2)].symbol() == "│")
            .expect("first Miller divider");
        assert_eq!(
            buffer[(first_divider + 3, 2)].bg,
            app.palette.surface0,
            "selected row is highlighted in the current column"
        );
    }

    // TP-B1.5-PLAIN-FALLBACK: prepared text remains visible before asynchronous
    // highlighting arrives. Highlighting is enhancement, never availability
    // authority.
    #[test]
    fn file_selection_renders_prepared_plain_text() {
        let td = TempDir::new("file-preview");
        fs::write(
            td.root.join("selected.txt"),
            "plain fallback\nsecond line\n",
        )
        .expect("write plain preview fixture");
        let app = app_with_fm(FmState::new(&td.root));

        let rows = render_rows(&app, 80, 6);
        assert!(
            rows.iter().any(|row| row.contains("plain fallback")),
            "prepared text is visible while highlighting is pending: {rows:?}"
        );
        assert!(rows.iter().any(|row| row.contains("second line")));
    }

    // TP-B1.5-STYLES: render-ready foreground and font modifiers map exactly
    // to Ratatui cells. Syntax preparation does not leak into render.
    #[test]
    fn highlighted_preview_maps_rgb_and_font_modifiers() {
        let td = TempDir::new("file-preview-style");
        fs::write(td.root.join("selected.rs"), "styled\n").expect("write styled fixture");
        let mut fm = FmState::new(&td.root);
        match &mut fm.preview {
            FmPreview::File(FmFilePreview::Text(preview)) => {
                preview.highlighted = Some(HighlightedTextPreview {
                    lines: vec![PreviewTextLine {
                        spans: vec![PreviewTextSpan {
                            content: "styled".to_owned(),
                            style: PreviewTextStyle {
                                foreground: Some([12, 34, 56]),
                                bold: true,
                                italic: true,
                                underline: true,
                            },
                        }],
                    }],
                    syntax_name: Some("Rust".to_owned()),
                    truncated_bytes: false,
                    truncated_lines: false,
                });
            }
            other => panic!("selected text file needs preview state, got {other:?}"),
        }
        let app = app_with_fm(fm);

        let rows = render_rows(&app, 80, 6);
        let (y, row) = rows
            .iter()
            .enumerate()
            .find(|(_, row)| row.contains("styled"))
            .expect("styled preview row");
        let styled_byte = row.find("styled").expect("styled preview column");
        let x = row[..styled_byte].chars().count() as u16;
        let buffer = render_buffer(&app, 80, 6);
        let cell = &buffer[(x, y as u16)];

        assert_eq!(cell.fg, ratatui::style::Color::Rgb(12, 34, 56));
        assert!(cell.modifier.contains(Modifier::BOLD));
        assert!(cell.modifier.contains(Modifier::ITALIC));
        assert!(cell.modifier.contains(Modifier::UNDERLINED));
    }

    // TP-B1.5-FAILURES: preparation failures have stable, distinct user-facing
    // states; none are confused with an empty directory or pending highlight.
    #[test]
    fn text_preview_failures_render_explicit_placeholders() {
        let td = TempDir::new("file-preview-failures");
        td.file("selected.txt");
        let cases = [
            (TextPreviewError::Binary, "(binary file)"),
            (
                TextPreviewError::InvalidUtf8 { valid_up_to: 3 },
                "(not UTF-8)",
            ),
            (
                TextPreviewError::Io(std::io::ErrorKind::PermissionDenied),
                "(permission denied)",
            ),
            (
                TextPreviewError::Io(std::io::ErrorKind::UnexpectedEof),
                "(preview unavailable)",
            ),
            (TextPreviewError::NotRegularFile, "(not a regular file)"),
        ];

        for (error, expected) in cases {
            let mut fm = FmState::new(&td.root);
            fm.preview = FmPreview::File(FmFilePreview::Unavailable(error));
            let rows = render_rows(&app_with_fm(fm), 80, 5);
            assert!(
                rows.iter().any(|row| row.contains(expected)),
                "{error:?} renders {expected:?}: {rows:?}"
            );
        }
    }

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
    #[test]
    fn truncated_text_preview_renders_marker() {
        let td = TempDir::new("file-preview-truncated");
        fs::write(td.root.join("selected.txt"), "visible prefix\n")
            .expect("write truncated fixture");
        let mut fm = FmState::new(&td.root);
        match &mut fm.preview {
            FmPreview::File(FmFilePreview::Text(preview)) => preview.truncated = true,
            other => panic!("selected text file needs preview state, got {other:?}"),
        }

        let rows = render_rows(&app_with_fm(fm), 80, 6);
        assert!(rows.iter().any(|row| row.contains("visible prefix")));
        assert!(
            rows.iter().any(|row| row.contains("(preview truncated)")),
            "truncation is explicit: {rows:?}"
        );
    }

    // TP-B1.5-LINE-LIMIT: a highlighter that stops at its independent line
    // budget exposes the same stable truncation marker as the byte reader.
    #[test]
    fn highlighted_line_limit_renders_truncation_marker() {
        let td = TempDir::new("file-preview-line-limit");
        fs::write(td.root.join("selected.rs"), "line\n").expect("write line-limit fixture");
        let mut fm = FmState::new(&td.root);
        match &mut fm.preview {
            FmPreview::File(FmFilePreview::Text(preview)) => {
                preview.highlighted = Some(HighlightedTextPreview {
                    lines: vec![PreviewTextLine {
                        spans: vec![PreviewTextSpan {
                            content: "bounded line".to_owned(),
                            style: PreviewTextStyle::default(),
                        }],
                    }],
                    syntax_name: Some("Rust".to_owned()),
                    truncated_bytes: false,
                    truncated_lines: true,
                });
            }
            other => panic!("selected text file needs preview state, got {other:?}"),
        }

        let rows = render_rows(&app_with_fm(fm), 80, 6);
        assert!(rows.iter().any(|row| row.contains("bounded line")));
        assert!(rows.iter().any(|row| row.contains("(preview truncated)")));
    }

    // TP-B1.5-COLUMN-BOUND: Paragraph clipping keeps an adversarial one-line
    // preview inside its Miller column; it does not wrap into extra rows or
    // write beyond the frame width.
    #[test]
    fn long_text_preview_line_is_clipped_to_column_width() {
        let td = TempDir::new("file-preview-long-line");
        fs::write(td.root.join("selected.txt"), "x".repeat(512)).expect("write long-line fixture");
        let app = app_with_fm(FmState::new(&td.root));

        let rows = render_rows(&app, 30, 5);
        assert!(rows.iter().all(|row| row.chars().count() <= 30));
        assert_eq!(
            rows.iter().filter(|row| row.contains("xxxxxxxx")).count(),
            1,
            "one logical preview line must not wrap: {rows:?}"
        );
    }

    // TP-A2.2.4/N1: at forty columns the two one-cell dividers leave all three
    // content columns at least twelve cells wide.
    #[test]
    fn forty_columns_preserve_three_readable_miller_columns() {
        let td = TempDir::new("forty-columns");
        td.dir("child");
        let app = app_with_fm(FmState::new(&td.root));
        let buffer = render_buffer(&app, 40, 6);
        let dividers: Vec<u16> = (0..40)
            .filter(|&x| buffer[(x, 2)].symbol() == "│")
            .collect();

        assert_eq!(dividers.len(), 2, "three columns need two dividers");
        let widths = [
            dividers[0],
            dividers[1] - dividers[0] - 1,
            40 - dividers[1] - 1,
        ];
        assert!(
            widths.iter().all(|&width| width >= 12),
            "all Miller columns remain readable: {widths:?}"
        );
    }

    // TP-A2.2.4: when three minimum-width columns cannot fit, parent context is
    // progressively disclosed first; current and preview remain readable.
    #[test]
    fn narrower_areas_degrade_to_two_then_one_column() {
        let td = TempDir::new("responsive-columns");
        td.dir("child");
        let app = app_with_fm(FmState::new(&td.root));

        let two = render_rows(&app, 30, 6).join("\n");
        assert!(!two.contains("PARENT"), "parent is hidden first: {two:?}");
        assert!(two.contains("CURRENT"), "current remains: {two:?}");
        assert!(two.contains("PREVIEW"), "preview remains: {two:?}");

        let one = render_rows(&app, 20, 6).join("\n");
        assert!(!one.contains("PARENT"), "parent stays hidden: {one:?}");
        assert!(one.contains("CURRENT"), "current remains: {one:?}");
        assert!(!one.contains("PREVIEW"), "preview hides second: {one:?}");
    }

    // TP-A3.2-VIEWPORT: CURRENT consumes the persistent viewport anchor rather
    // than deriving a new window from the cursor during the pure render pass.
    #[test]
    fn current_panel_renders_from_persisted_viewport() {
        let td = TempDir::new("persisted-viewport");
        for index in 0..6 {
            td.file(&format!("{index:02}.txt"));
        }
        let mut fm = FmState::new(&td.root);
        fm.cursor = 4;
        fm.viewport_start = 3;

        let rows = render_rows(&app_with_fm(fm), 20, 5).join("\n");
        assert!(!rows.contains("02.txt"), "stale derived window: {rows:?}");
        assert!(rows.contains("03.txt"), "viewport first row: {rows:?}");
        assert!(rows.contains("04.txt"), "cursor remains visible: {rows:?}");
        assert!(rows.contains("05.txt"), "viewport last row: {rows:?}");
    }

    // TP-C2.1-MILLER-GEOMETRY: at each responsive breakpoint, every CURRENT
    // entry exposes one bounded name rect followed by the complete, ordered,
    // and pairwise-disjoint row-action set. Render and input must consume this
    // one geometry snapshot rather than independently recreating the split.
    #[test]
    fn current_row_actions_follow_miller_geometry_at_all_breakpoints() {
        use crate::app::state::FileManagerRowAction;

        for width in [20, 30, 40] {
            let area = Rect::new(5, 7, width, 6);
            let body = Rect::new(area.x, area.y + 1, area.width, area.height - 1);
            let current_list = panel_areas(miller_layout(body).current)[1];

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

        let area = Rect::new(10, 20, 20, 5); // three CURRENT list rows

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
                "Send to Agent".to_string(),
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
        fm.entries[1].operation_supported = false;
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

        fm.entries[1].operation_supported = true;
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

    // TP-A2.2.5: the filesystem root has no parent but still renders a stable,
    // explicit parent-column state without panicking.
    #[test]
    fn filesystem_root_renders_no_parent_state() {
        let app = app_with_fm(FmState::new("/"));
        let rows = render_rows(&app, 40, 5);
        assert!(
            rows.iter().any(|row| row.contains("(root)")),
            "root parent state is explicit: {rows:?}"
        );
    }

    // TP-A2.2.3: moving the cursor refreshes the directory preview; stale child
    // contents from the previous selection must not survive.
    #[test]
    fn cursor_movement_refreshes_directory_preview() {
        let td = TempDir::new("preview-cursor");
        td.dir("alpha");
        td.dir("beta");
        fs::write(td.root.join("alpha").join("alpha-only.txt"), b"x").expect("write alpha preview");
        fs::write(td.root.join("beta").join("beta-only.txt"), b"x").expect("write beta preview");
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
    #[test]
    fn renders_entries_directories_first() {
        let td = TempDir::new("list");
        td.file("banana.txt");
        td.dir("apples");
        td.file("cherry.txt");
        let app = app_with_fm(FmState::new(&td.root));

        let rows = render_rows(&app, 32, 6);
        let joined = rows.join("\n");

        assert!(joined.contains("apples/"), "dir with slash: {rows:?}");
        assert!(joined.contains("banana.txt"), "file shown: {rows:?}");
        assert!(joined.contains("cherry.txt"), "file shown: {rows:?}");

        let dir_row = rows.iter().position(|r| r.contains("apples/")).unwrap();
        let file_row = rows.iter().position(|r| r.contains("banana.txt")).unwrap();
        assert!(dir_row < file_row, "directory precedes files: {rows:?}");
    }

    // TP-A2.3: the cursor row is highlighted (surface0 background) while other
    // rows are not.
    #[test]
    fn cursor_row_is_highlighted() {
        let td = TempDir::new("cursor");
        td.file("a.txt");
        td.file("b.txt");
        let mut fm = FmState::new(&td.root);
        fm.cursor = 1; // second entry (b.txt)
        let app = app_with_fm(fm);

        let mut terminal = Terminal::new(TestBackend::new(20, 4)).unwrap();
        terminal
            .draw(|frame| render_file_manager(&app, frame, Rect::new(0, 0, 20, 4)))
            .unwrap();
        let buffer = terminal.backend().buffer().clone();

        let rows: Vec<String> = (0..4)
            .map(|y| {
                (0..20)
                    .map(|x| buffer[(x, y)].symbol().chars().next().unwrap_or(' '))
                    .collect()
            })
            .collect();
        let cursor_row = rows
            .iter()
            .position(|row| row.contains("b.txt"))
            .expect("b.txt row") as u16;
        let other_row = rows
            .iter()
            .position(|row| row.contains("a.txt"))
            .expect("a.txt row") as u16;
        assert_eq!(
            buffer[(2, cursor_row)].bg,
            app.palette.surface0,
            "cursor row uses the highlight background"
        );
        assert_ne!(
            buffer[(2, other_row)].bg,
            app.palette.surface0,
            "non-cursor row is not highlighted"
        );
    }

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
        let buffer = render_buffer(&app, 20, 5);
        let rows = (0..5)
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
                assert_eq!(
                    buffer[(x, y)].bg,
                    app.palette.panel_bg,
                    "FM canvas cell ({x}, {y}) uses panel background"
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
    #[test]
    fn alternate_palette_maps_file_manager_semantic_roles() {
        let td = TempDir::new("alternate-palette");
        td.file("a.txt");
        td.file("b.txt");
        td.file("c.txt");
        let mut fm = FmState::new(&td.root);
        assert!(fm.replace_selection(0));
        assert!(fm.select(1));
        let mut app = app_with_fm(fm);
        app.palette = crate::app::state::Palette::catppuccin_latte();

        let width = 80;
        let height = 6;
        let buffer = render_buffer(&app, width, height);
        let (header_x, header_y) = find_rendered_text(&buffer, width, height, "herdr-fmrender");
        assert_eq!(buffer[(header_x, header_y)].fg, app.palette.subtext0);

        let layout = file_manager_areas(Rect::new(0, 0, width, height))
            .expect("non-empty FM geometry")
            .columns;
        for divider in layout.dividers.into_iter().flatten() {
            assert_eq!(
                buffer[(divider.x, divider.y + 1)].fg,
                app.palette.surface_dim
            );
        }

        let (selected_x, selected_y) = find_rendered_text(&buffer, width, height, "a.txt");
        assert_eq!(buffer[(selected_x, selected_y)].bg, app.palette.surface1);
        let (cursor_x, cursor_y) = find_rendered_text(&buffer, width, height, "b.txt");
        assert_eq!(buffer[(cursor_x, cursor_y)].bg, app.palette.surface0);

        let new_folder = compute_file_manager_header_action_areas(Rect::new(0, 0, width, height))
            .into_iter()
            .find(|area| area.action == FileManagerHeaderAction::NewFolder)
            .expect("wide layout exposes complete new-folder action");
        let disabled_cell = &buffer[(new_folder.rect.x, new_folder.rect.y)];
        assert_eq!(disabled_cell.fg, app.palette.overlay0);
        assert!(disabled_cell.modifier.contains(Modifier::DIM));

        let empty = TempDir::new("alternate-palette-empty");
        let mut empty_app = app_with_fm(FmState::new(&empty.root));
        empty_app.palette = crate::app::state::Palette::catppuccin_latte();
        let empty_buffer = render_buffer(&empty_app, width, height);
        let (empty_x, empty_y) = find_rendered_text(&empty_buffer, width, height, "(empty)");
        assert_eq!(
            empty_buffer[(empty_x, empty_y)].fg,
            empty_app.palette.overlay0
        );
    }

    // TP-N4.1-SELECTION-STATE: CURRENT paints exactly one cursor-focus style,
    // independently from any explicit multi-selection background.
    #[test]
    fn current_panel_has_exactly_one_cursor_focus_style() {
        let td = TempDir::new("single-visual-selection");
        td.file("a.txt");
        td.file("b.txt");
        td.file("c.txt");
        let mut fm = FmState::new(&td.root);
        fm.cursor = 1;
        let app = app_with_fm(fm);
        let buffer = render_buffer(&app, 20, 5);

        let selected_rows = (2..5)
            .filter(|&row| buffer[(2, row)].bg == app.palette.surface0)
            .count();
        assert_eq!(selected_rows, 1);
    }

    // TP-A2.4: an empty (or unreadable) directory renders a placeholder without
    // panicking.
    #[test]
    fn empty_directory_renders_placeholder() {
        let td = TempDir::new("empty");
        let app = app_with_fm(FmState::new(&td.root));

        let rows = render_rows(&app, 24, 5);
        assert!(
            rows.iter().any(|r| r.contains("(empty)")),
            "empty placeholder shown: {rows:?}"
        );
    }

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
    #[test]
    fn unicode_name_does_not_overwrite_row_action_cells() {
        use crate::app::state::FileManagerRowAction;

        let td = TempDir::new("unicode-row-actions");
        td.file("çalışma-🚀-uzun-dosya-adı.txt");
        let app = app_with_fm(FmState::new(&td.root));
        let buffer = render_buffer(&app, 20, 4);
        let entries = geometry_entries(1);
        let geometry = compute_file_manager_row_geometry(Rect::new(0, 0, 20, 4), &entries, 0);

        assert_eq!(geometry.actions.len(), FileManagerRowAction::ALL.len());
        for action in &geometry.actions {
            let rendered = (action.rect.x..action.rect.right())
                .map(|x| buffer[(x, action.rect.y)].symbol())
                .collect::<String>();
            assert_eq!(rendered, action.action.label());
        }
    }

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
}
