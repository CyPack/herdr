//! `herdr view` — one file, one tab, drawn by herdr itself.
//!
//! The viewer runs inside a PTY like any other terminal program: it owns the
//! screen while it is up and hands it back untouched when it leaves. What to
//! draw is decided in [`frame`], which is pure; this module is the thin shell
//! that talks to the terminal.

pub(crate) mod frame;

use std::io::{self, Write};
use std::path::Path;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::{execute, queue};
use ratatui::layout::Rect;

use crate::kitty_graphics::HostCellSize;
use frame::{compute_frame, turn_page, ViewerFrame};

/// Restores the terminal when it goes out of scope.
///
/// Undoing raw mode and the alternate screen by hand is not enough: every `?`
/// and every panic skips those lines, and the reader is left with a terminal
/// that needs `reset` to type in again. Tying it to `Drop` covers the error
/// paths and the unwind alike.
struct TerminalGuard {
    restored: bool,
}

impl TerminalGuard {
    fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen)?;
        Ok(Self { restored: false })
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        if self.restored {
            return;
        }
        self.restored = true;
        // Best effort, and deliberately silent: a failure here would panic
        // during an unwind and replace the real error with this one.
        let _ = clear_pictures(&mut io::stdout());
        let _ = execute!(io::stdout(), crossterm::cursor::Show);
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        let _ = disable_raw_mode();
    }
}

/// Delete every image this process placed.
///
/// Kitty images outlive the alternate screen: leaving without this hands the
/// reader their shell with a picture still hanging over it.
fn clear_pictures(out: &mut impl Write) -> io::Result<()> {
    write!(out, "\x1b_Ga=d,d=A,q=2;\x1b\\")?;
    out.flush()
}

/// Show `path` until the reader closes it. Returns the process exit code.
pub(crate) fn run(path: &Path, start_page: usize) -> io::Result<i32> {
    let _guard = TerminalGuard::enter()?;
    let mut page = start_page;
    let mut drawn: Option<ViewerFrame> = None;
    let mut painted = false;

    loop {
        let (cols, rows) = crossterm::terminal::size()?;
        let area = Rect::new(0, 0, cols, rows);
        let cell_size = HostCellSize::try_from_terminal(area)
            .unwrap_or_else(|| HostCellSize::fallback_for_area(area));
        let next = compute_frame(path, page, cols, rows, cell_size);

        if !painted || next != drawn {
            draw(&mut io::stdout(), next.as_ref(), cols, rows)?;
            drawn = next;
            painted = true;
        }

        // A poll rather than a blocking read: a resize arrives as an event on
        // most hosts, but not all, and the timeout keeps the picture correct on
        // the ones where it does not.
        if !event::poll(Duration::from_millis(250))? {
            continue;
        }
        match event::read()? {
            Event::Key(key) if key.kind != KeyEventKind::Release => match key.code {
                KeyCode::Char('q') | KeyCode::Esc => return Ok(0),
                KeyCode::PageDown => {
                    page = turn_page(page, drawn.as_ref().and_then(|f| f.total_pages), true);
                }
                KeyCode::PageUp => {
                    page = turn_page(page, drawn.as_ref().and_then(|f| f.total_pages), false);
                }
                _ => {}
            },
            Event::Resize(_, _) => painted = false,
            _ => {}
        }
    }
}

/// Paint one frame: clear, place the picture, write the status line.
fn draw(out: &mut impl Write, frame: Option<&ViewerFrame>, cols: u16, rows: u16) -> io::Result<()> {
    clear_pictures(out)?;
    queue!(
        out,
        crossterm::terminal::Clear(crossterm::terminal::ClearType::All),
        crossterm::cursor::Hide,
        crossterm::cursor::MoveTo(0, 0)
    )?;

    let Some(frame) = frame else {
        // No room to draw. Say so rather than leaving a blank screen, which
        // reads as a hang.
        write!(out, "terminal too small")?;
        return out.flush();
    };

    if let Some(picture) = frame.picture.as_ref() {
        let id = 1u32;
        let control = format!(
            "a=t,t=d,f=32,s={},v={},i={id},q=2",
            picture.prepared.width, picture.prepared.height
        );
        crate::kitty_graphics::encode_kitty_data_to(out, &control, &picture.prepared.rgba)?;
        queue!(out, crossterm::cursor::MoveTo(picture.col, picture.row))?;
        write!(
            out,
            "\x1b_Ga=p,i={id},p=1,c={},r={},z=0,C=1,q=2;\x1b\\",
            picture.cols, picture.rows
        )?;
    }

    let status_row = rows.saturating_sub(1);
    queue!(out, crossterm::cursor::MoveTo(0, status_row))?;
    let status: String = frame.status.chars().take(cols as usize).collect();
    write!(out, "{status}")?;
    out.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    // TP-FVIEW-TAB-10: leaving deletes every picture this process placed.
    // Kitty images outlive the alternate screen, so skipping this hands the
    // reader their shell with a picture hanging over it.
    #[test]
    fn leaving_deletes_the_pictures() {
        let mut out = Vec::new();
        clear_pictures(&mut out).expect("clear");
        let text = String::from_utf8(out).expect("utf-8");
        assert!(
            text.contains("a=d"),
            "expected a delete command, got {text:?}"
        );
        assert!(text.contains("d=A"), "expected an all-images delete");
    }

    // TP-FVIEW-TAB-11: a frame with no room still writes something. A blank
    // screen is indistinguishable from a hang.
    #[test]
    fn a_missing_frame_still_says_something() {
        let mut out = Vec::new();
        draw(&mut out, None, 10, 3).expect("draw");
        let text = String::from_utf8_lossy(&out).into_owned();
        assert!(text.contains("too small"), "got {text:?}");
    }

    // TP-FVIEW-TAB-12: the status line is truncated to the terminal width. A
    // longer line wraps, pushing the screen up and scrolling the picture out of
    // the box it was placed in.
    #[test]
    fn the_status_line_is_truncated_to_the_width() {
        let frame = ViewerFrame {
            source_path: std::path::PathBuf::from("/tmp/a-very-long-file-name.png"),
            picture: None,
            status: "a-very-long-file-name.png - q to close".to_owned(),
            total_pages: None,
        };
        let mut out = Vec::new();
        draw(&mut out, Some(&frame), 10, 3).expect("draw");
        let text = String::from_utf8_lossy(&out).into_owned();
        assert!(
            !text.contains("q to close"),
            "the tail must be cut at 10 columns, got {text:?}"
        );
        assert!(
            text.contains("a-very-lon"),
            "the head must survive: {text:?}"
        );
    }
}
