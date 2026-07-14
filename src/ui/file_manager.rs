//! Native file manager — directory-list render (A2).
//!
//! Draws the open [`FmState`](crate::fm::FmState) into a rect: a one-row header
//! with the current directory, then its entries (directories first, natural
//! order — see `crate::fm`) with the cursor row highlighted. Pure draw (reads
//! state, never mutates), matching herdr's `compute_view`/`render` split, and it
//! reuses the same list-row idiom as `render_modal_choice_list` and the sidebar
//! lists. Client-side presentation only (runtime/client boundary).
//!
//! This is the first non-terminal *content* swapped into a named region
//! (`CenterContent`): when `app.file_manager` is open, the base layer draws this
//! here instead of the terminal panes. Navigation input, scrolling, previews,
//! and per-row buttons land in later steps (A3+).

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use super::text::truncate_end;
use crate::app::state::AppState;

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

    // A one-row header (current directory) above the entry list.
    let [header_area, list_area] =
        Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(area);

    let cwd_text = fm.cwd.to_string_lossy();
    let header = truncate_end(&cwd_text, header_area.width as usize);
    frame.render_widget(
        Paragraph::new(header).style(Style::default().fg(p.subtext0).add_modifier(Modifier::BOLD)),
        header_area,
    );

    if list_area.height == 0 {
        return;
    }
    if fm.entries.is_empty() {
        frame.render_widget(
            Paragraph::new("  (empty)").style(Style::default().fg(p.overlay1)),
            list_area,
        );
        return;
    }

    // Show a window of entries that keeps the cursor visible. Persistent scroll
    // state lands with navigation (A3); here the window is derived from the
    // cursor so a long list still renders the highlighted row.
    let rows = list_area.height as usize;
    let first = if fm.cursor < rows {
        0
    } else {
        fm.cursor - rows + 1
    };

    for (offset, entry) in fm.entries.iter().skip(first).take(rows).enumerate() {
        let idx = first + offset;
        let row = Rect::new(list_area.x, list_area.y + offset as u16, list_area.width, 1);
        let suffix = if entry.is_dir { "/" } else { "" };
        let label = truncate_end(
            &format!("  {}{}", entry.name, suffix),
            list_area.width as usize,
        );
        let style = if idx == fm.cursor {
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
    use crate::fm::FmState;
    use ratatui::backend::TestBackend;
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
        let mut terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
        terminal
            .draw(|frame| render_file_manager(app, frame, Rect::new(0, 0, w, h)))
            .unwrap();
        let buffer = terminal.backend().buffer().clone();
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

        // Header is row 0; entries start at row 1: a.txt (row 1), b.txt (row 2).
        let cursor_row = 2u16;
        let other_row = 1u16;
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
