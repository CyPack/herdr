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
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use super::text::truncate_end;
use crate::app::state::AppState;
use crate::fm::{FileEntry, FmPreview};

const MIN_COLUMN_WIDTH: u16 = 12;
const DIVIDER_WIDTH: u16 = 1;
const THREE_COLUMN_MIN_WIDTH: u16 = MIN_COLUMN_WIDTH * 3 + DIVIDER_WIDTH * 2;
const TWO_COLUMN_MIN_WIDTH: u16 = MIN_COLUMN_WIDTH * 2 + DIVIDER_WIDTH;

#[derive(Debug, Clone, Copy)]
struct MillerLayout {
    parent: Option<Rect>,
    current: Rect,
    preview: Option<Rect>,
    dividers: [Option<Rect>; 2],
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

/// Render the open file manager into `area`. Does nothing when the file manager
/// is closed (`app.file_manager` is `None`) or the area is empty, so callers can
/// invoke it unconditionally.
pub(crate) fn render_file_manager(app: &AppState, frame: &mut Frame, area: Rect) {
    let Some(fm) = app.file_manager.as_ref() else {
        return;
    };
    if area.width == 0 || area.height == 0 {
        return;
    }
    let p = &app.palette;

    // A one-row identity header stays stable while responsive Miller columns
    // progressively disclose parent and preview context below it.
    let [header_area, body_area] =
        Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(area);

    let cwd_text = fm.cwd.to_string_lossy();
    let header = truncate_end(&cwd_text, header_area.width as usize);
    frame.render_widget(
        Paragraph::new(header).style(Style::default().fg(p.subtext0).add_modifier(Modifier::BOLD)),
        header_area,
    );

    if body_area.height == 0 {
        return;
    }

    let layout = miller_layout(body_area);
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
            );
        } else {
            render_panel(app, frame, parent_area, "PARENT", &[], None, "(root)");
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
            ),
            FmPreview::File(_) => render_panel(
                app,
                frame,
                preview_area,
                "PREVIEW",
                &[],
                None,
                "(file preview later)",
            ),
            FmPreview::Directory(entries) => render_panel(
                app,
                frame,
                preview_area,
                "PREVIEW",
                entries,
                None,
                "(empty)",
            ),
        }
    }
}

fn render_panel(
    app: &AppState,
    frame: &mut Frame,
    area: Rect,
    title: &str,
    entries: &[FileEntry],
    selected: Option<usize>,
    empty_label: &str,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let p = &app.palette;
    let [title_area, list_area] =
        Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(area);
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

    // Show a window of entries that keeps the cursor visible. Persistent scroll
    // state lands later; the window is derived so highlighted context remains
    // visible even in long parent/current directories.
    let rows = list_area.height as usize;
    let cursor = selected.unwrap_or(0).min(entries.len() - 1);
    let first = if cursor < rows { 0 } else { cursor - rows + 1 };

    for (offset, entry) in entries.iter().skip(first).take(rows).enumerate() {
        let idx = first + offset;
        let row = Rect::new(list_area.x, list_area.y + offset as u16, list_area.width, 1);
        let suffix = if entry.is_dir { "/" } else { "" };
        let label = truncate_end(
            &format!("  {}{}", entry.name, suffix),
            list_area.width as usize,
        );
        let style = if selected.is_some_and(|selected| idx == selected) {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fm::{
        FmFilePreview, FmState, HighlightedTextPreview, PreviewTextLine, PreviewTextSpan,
        PreviewTextStyle, TextPreviewError,
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
        let x = row.find("styled").expect("styled preview column") as u16;
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
