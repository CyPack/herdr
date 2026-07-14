//! Native file manager — Miller-capable directory-list render (A2.2).
//!
//! Draws the open [`FmState`](crate::fm::FmState) into a rect: a one-row current
//! directory header followed by responsive parent/current/preview columns. Pure
//! draw (reads state, never mutates or touches the filesystem), matching herdr's
//! `compute_view`/`render` split. Client-side presentation only.
//!
//! This is the first non-terminal *content* swapped into a named region
//! (`CenterContent`): when `app.file_manager` is open, the base layer draws this
//! here instead of the terminal panes. Text/image previews and per-row buttons
//! land in later steps (B1/B2/C2).

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use super::text::truncate_end;
use crate::app::state::AppState;
use crate::app::state::{
    FileManagerActionBarModel, FileManagerActionBarSelection, FileManagerActionBarSelectionKind,
    FileManagerHeaderAction, FileManagerHeaderActionArea, FileManagerRowArea,
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
pub(crate) fn compute_file_manager_row_areas(
    area: Rect,
    entry_count: usize,
    viewport_start: usize,
) -> Vec<FileManagerRowArea> {
    let Some(areas) = file_manager_areas(area) else {
        return Vec::new();
    };
    let list = panel_areas(areas.columns.current)[1];
    let visible_rows = list.height as usize;
    if list.width == 0 || visible_rows == 0 || entry_count == 0 {
        return Vec::new();
    }

    let start = viewport_start.min(entry_count.saturating_sub(visible_rows));
    let count = visible_rows.min(entry_count.saturating_sub(start));
    (0..count)
        .map(|offset| FileManagerRowArea {
            rect: Rect::new(list.x, list.y.saturating_add(offset as u16), list.width, 1),
            entry_idx: start + offset,
        })
        .collect()
}

/// Build the persistent action-bar content from already-prepared FM state.
/// This is pure client presentation logic: no metadata or directory reads.
pub(crate) fn compute_file_manager_action_bar_model(
    file_manager: &FmState,
    clipboard: &[std::path::PathBuf],
) -> FileManagerActionBarModel {
    let selection = file_manager
        .selected()
        .map(|entry| FileManagerActionBarSelection {
            path: entry.path.clone(),
            label: entry.name.clone(),
            kind: if entry.is_dir {
                FileManagerActionBarSelectionKind::Directory
            } else {
                FileManagerActionBarSelectionKind::File
            },
        });
    FileManagerActionBarModel {
        selection,
        clipboard_count: clipboard.len(),
    }
}

fn file_manager_action_bar_identity(cwd: &str, action_bar: &FileManagerActionBarModel) -> String {
    let mut identity = String::from(cwd);
    match action_bar.selection.as_ref() {
        Some(selection) => {
            identity.push_str(" — ");
            identity.push_str(&selection.label);
        }
        None => identity.push_str(" — empty"),
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
        fallback_action_bar =
            compute_file_manager_action_bar_model(fm, app.file_manager_clipboard.as_slice());
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
        frame.render_widget(
            Paragraph::new(action.action.label()).style(Style::default().fg(p.overlay1)),
            action.rect,
        );
    }

    if areas.columns.current.height == 0 {
        return;
    }

    let layout = areas.columns;
    let fallback_rows;
    let current_rows = if area == app.view.terminal_area {
        app.view.file_manager_row_areas.as_slice()
    } else {
        // Unit-level/component callers can render into an arbitrary rect
        // without a preceding full-frame compute_view pass.
        fallback_rows = compute_file_manager_row_areas(area, fm.entries.len(), fm.viewport_start);
        fallback_rows.as_slice()
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
            );
        } else {
            render_panel(app, frame, parent_area, "PARENT", &[], None, "(root)", None);
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
    );

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
        render_entry_row(app, frame, row, entry, selected == Some(idx));
    }
}

fn render_entry_row(
    app: &AppState,
    frame: &mut Frame,
    row: Rect,
    entry: &FileEntry,
    selected: bool,
) {
    let p = &app.palette;
    let suffix = if entry.is_dir { "/" } else { "" };
    let label = truncate_end(&format!("  {}{}", entry.name, suffix), row.width as usize);
    let style = if selected {
        Style::default()
            .bg(p.surface0)
            .fg(p.text)
            .add_modifier(Modifier::BOLD)
    } else if entry.is_dir {
        Style::default().fg(p.blue).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(p.subtext0)
    };
    frame.render_widget(Paragraph::new(label).style(style), row);
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

    // TP-A3.3-HIT-GEOMETRY: at each responsive breakpoint, CURRENT row hit
    // rects occupy exactly the same list geometry that the pure renderer uses.
    #[test]
    fn current_row_areas_follow_miller_geometry_at_all_breakpoints() {
        for width in [20, 30, 40] {
            let area = Rect::new(5, 7, width, 6);
            let body = Rect::new(area.x, area.y + 1, area.width, area.height - 1);
            let current_list = panel_areas(miller_layout(body).current)[1];

            let rows = compute_file_manager_row_areas(area, 3, 0);
            assert_eq!(rows.len(), 3, "width {width}");
            for (index, row) in rows.iter().enumerate() {
                assert_eq!(row.entry_idx, index, "width {width}");
                assert_eq!(
                    row.rect,
                    Rect::new(
                        current_list.x,
                        current_list.y + index as u16,
                        current_list.width,
                        1,
                    ),
                    "width {width}",
                );
            }
        }
    }

    // TP-A3.3-HIT-GEOMETRY: viewport offsets map screen rows to absolute entry
    // indices, and adversarial offsets clamp to the last full visible window.
    #[test]
    fn current_row_areas_apply_viewport_and_clamp_to_list_end() {
        let area = Rect::new(10, 20, 20, 5); // three CURRENT list rows

        let rows = compute_file_manager_row_areas(area, 10, 6);
        assert_eq!(
            rows.iter().map(|row| row.entry_idx).collect::<Vec<_>>(),
            vec![6, 7, 8]
        );
        assert_eq!(rows[0].rect, Rect::new(10, 22, 20, 1));
        assert_eq!(rows[2].rect, Rect::new(10, 24, 20, 1));

        let clamped = compute_file_manager_row_areas(area, 10, usize::MAX);
        assert_eq!(
            clamped.iter().map(|row| row.entry_idx).collect::<Vec<_>>(),
            vec![7, 8, 9]
        );
    }

    // TP-A3.3-HIT-GEOMETRY: no content cells means no clickable rows. All
    // degenerate rectangles and an empty directory remain panic-free.
    #[test]
    fn current_row_areas_are_empty_for_degenerate_geometry_or_list() {
        for area in [
            Rect::new(0, 0, 0, 6),
            Rect::new(0, 0, 20, 0),
            Rect::new(0, 0, 20, 1),
            Rect::new(0, 0, 20, 2),
        ] {
            assert!(compute_file_manager_row_areas(area, 10, 0).is_empty());
        }
        assert!(compute_file_manager_row_areas(Rect::new(0, 0, 20, 6), 0, usize::MAX).is_empty());
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

    // TP-N3.1-CONTENT: the pure persistent action-bar model carries the live
    // directory/file/empty selection identity and client-local clipboard
    // summary without reading the filesystem during render.
    #[test]
    fn action_bar_model_tracks_selection_kind_and_clipboard_content() {
        let td = TempDir::new("action-bar-model");
        td.dir("alpha-dir");
        td.file("beta.txt");
        let mut fm = FmState::new(&td.root);

        let directory = compute_file_manager_action_bar_model(&fm, &[]);
        let selection = directory.selection.as_ref().expect("directory selection");
        assert_eq!(selection.label, "alpha-dir");
        assert_eq!(selection.path, td.root.join("alpha-dir"));
        assert_eq!(selection.kind, FileManagerActionBarSelectionKind::Directory);
        assert_eq!(directory.clipboard_count, 0);

        fm.move_down();
        let clipboard = vec![td.root.join("copied-one"), td.root.join("copied-two")];
        let file = compute_file_manager_action_bar_model(&fm, &clipboard);
        let selection = file.selection.as_ref().expect("file selection");
        assert_eq!(selection.label, "beta.txt");
        assert_eq!(selection.path, td.root.join("beta.txt"));
        assert_eq!(selection.kind, FileManagerActionBarSelectionKind::File);
        assert_eq!(file.clipboard_count, 2);

        let empty = TempDir::new("action-bar-empty");
        let empty_model = compute_file_manager_action_bar_model(&FmState::new(&empty.root), &[]);
        assert!(empty_model.selection.is_none());
        assert_eq!(empty_model.clipboard_count, 0);
    }

    // TP-N3.1-RENDER: the persistent header visibly follows prepared model
    // content; the component fallback uses AppState only and performs no I/O.
    #[test]
    fn action_bar_renders_selected_name_clipboard_count_and_empty_state() {
        let td = TempDir::new("action-bar-render");
        td.file("selected.txt");
        let mut app = app_with_fm(FmState::new(&td.root));
        app.file_manager_clipboard = vec![td.root.join("copied.txt")];

        let selected_header = render_rows(&app, 160, 5)[0].clone();
        assert!(
            selected_header.contains("selected.txt"),
            "selected header: {selected_header:?}"
        );
        assert!(
            selected_header.contains("clipboard: 1"),
            "selected header: {selected_header:?}"
        );

        let empty = TempDir::new("action-bar-render-empty");
        let empty_app = app_with_fm(FmState::new(&empty.root));
        let empty_header = render_rows(&empty_app, 160, 5)[0].clone();
        assert!(
            empty_header.contains("empty"),
            "empty header: {empty_header:?}"
        );
        assert!(!empty_header.contains("selected.txt"));
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

    // TP-A3.4-SCOPE: CURRENT paints exactly one cursor-owned selection. Bulk
    // selection visuals remain deferred with N4/C2 semantics.
    #[test]
    fn current_panel_has_exactly_one_visual_selection() {
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
