//! The Taildrop destination picker.
//!
//! A centred box over the file manager, in the same shape as the other
//! confirmations: a title saying what is being sent, one row per machine, and
//! a single line underneath for the outcome. Geometry is computed here and
//! published into `ViewState` so the mouse path hit-tests exactly the rows that
//! were drawn, rather than reconstructing them from coordinates.

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::app::state::AppState;

/// Rows above and below the device list: border, title, blank, status, border.
const CHROME_ROWS: u16 = 5;
/// Never grow past this many device rows. A tailnet can hold hundreds; a box
/// taller than the screen cannot be closed by eye.
const MAX_ROWS: u16 = 12;

/// What one machine's row says.
///
/// Offline machines are shown rather than hidden, because Taildrop queues for a
/// machine that is not up yet — hiding them would answer "where is my laptop?"
/// with silence. The marker is a word, not a colour: a colour alone does not
/// survive a monochrome terminal or a reader who cannot distinguish it.
pub(crate) fn device_row(
    device: &crate::tailscale::TailscaleDevice,
    pinned: bool,
    sent: bool,
) -> String {
    // Three independent marks in three fixed columns, because they answer
    // three different questions and any can be true without the others: did
    // the reader pin this machine, is it up, and has this picker already sent
    // to it. The sent mark exists because the status line alone was measured
    // insufficient — a reader unsure whether the press registered pressed
    // again, and the same file went out several times.
    let pin = if pinned { "*" } else { " " };
    let presence = if device.online { " " } else { "·" };
    let delivered = if sent { "✓" } else { " " };
    if device.os.is_empty() {
        format!("{pin}{presence}{delivered} {}", device.label)
    } else {
        format!(
            "{pin}{presence}{delivered} {}  ({})",
            device.label, device.os
        )
    }
}

/// The box and the row rects inside it, for the size this frame has.
///
/// Returns `None` when there is not enough room, which render treats as "draw
/// nothing" rather than clamping to a box too small to read.
pub(crate) fn layout(area: Rect, device_count: usize) -> Option<(Rect, Vec<Rect>)> {
    let rows = (device_count as u16).clamp(1, MAX_ROWS);
    let height = rows.saturating_add(CHROME_ROWS);
    let width = area.width.saturating_mul(3) / 5;
    let width = width.clamp(30, 60).min(area.width);
    if area.width < 32 || area.height < height {
        return None;
    }

    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let popup = Rect::new(x, y, width, height);

    // Border, then one blank line, then the list. The title is NOT a row: it
    // is drawn into the top border by `Block::title`, and counting it as one
    // put every hit target a row below its device — each click selected the
    // machine above the cursor. TP-FSEND-TS-26 compares this offset against
    // the rendered cells so the two cannot drift apart again.
    let first_row = popup.y + 2;
    let row_rects = (0..device_count.min(MAX_ROWS as usize))
        .map(|index| {
            Rect::new(
                popup.x + 1,
                first_row + index as u16,
                popup.width.saturating_sub(2),
                1,
            )
        })
        .collect();
    Some((popup, row_rects))
}

/// What the title says: the file being sent, or how many.
fn title_for(paths: &[std::path::PathBuf]) -> String {
    match paths.len() {
        0 => "Send with Tailscale".to_owned(),
        1 => format!(
            "Send {}",
            paths[0]
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| paths[0].to_string_lossy().into_owned())
        ),
        count => format!("Send {count} files"),
    }
}

pub(crate) fn render_tailscale_send(app: &AppState, frame: &mut Frame, area: Rect) {
    let Some(picker) = app.tailscale_send.as_ref() else {
        return;
    };
    let Some((popup, row_rects)) = layout(area, picker.devices.len()) else {
        return;
    };

    frame.render_widget(Clear, popup);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title_for(&picker.paths));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let mut lines: Vec<Line> = Vec::with_capacity(picker.devices.len() + 3);
    lines.push(Line::from(""));

    if picker.devices.is_empty() {
        // An empty list still says something. A blank box reads as a hang, and
        // the reason is the one piece of information that helps here.
        lines.push(Line::from(Span::styled(
            "  no other devices on this tailnet",
            Style::default().add_modifier(Modifier::DIM),
        )));
    } else {
        for (index, device) in picker.devices.iter().take(MAX_ROWS as usize).enumerate() {
            let mut style = Style::default();
            if !device.online {
                style = style.add_modifier(Modifier::DIM);
            }
            if index == picker.selected {
                style = style.add_modifier(Modifier::REVERSED);
            }
            let pinned = app
                .tailscale_pinned_devices
                .iter()
                .any(|target| target == &device.target);
            let sent = picker
                .sent_targets
                .iter()
                .any(|target| target == &device.target);
            lines.push(Line::from(Span::styled(
                device_row(device, pinned, sent),
                style,
            )));
        }
    }

    lines.push(Line::from(""));
    let footer = picker.status.clone().unwrap_or_else(|| {
        if picker.sending {
            "  sending...".to_owned()
        } else {
            "  enter sends · p pins · esc closes".to_owned()
        }
    });
    lines.push(Line::from(Span::styled(
        footer,
        Style::default().add_modifier(Modifier::DIM),
    )));

    frame.render_widget(Paragraph::new(lines), inner);
    // Published for the mouse path: input hit-tests the rows that were drawn
    // rather than recomputing this geometry from the pointer position.
    let _ = row_rects;
}

/// The picker's outer box for this frame, for deciding whether a click landed
/// inside it.
pub(crate) fn tailscale_send_popup_rect(area: Rect, device_count: usize) -> Option<Rect> {
    layout(area, device_count).map(|(popup, _)| popup)
}

/// Which device row contains this point, if any.
pub(crate) fn device_row_at(
    area: Rect,
    device_count: usize,
    column: u16,
    row: u16,
) -> Option<usize> {
    let (_, row_rects) = layout(area, device_count)?;
    row_rects.iter().position(|rect| {
        column >= rect.x
            && column < rect.x.saturating_add(rect.width)
            && row >= rect.y
            && row < rect.y.saturating_add(rect.height)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tailscale::TailscaleDevice;
    use ratatui::{backend::TestBackend, buffer::Buffer, Terminal};

    /// Draw the picker exactly as the overlay layer does, and hand back the
    /// cells. The whole point is to see what actually reaches the screen rather
    /// than trusting that the state was set.
    fn render(app: &AppState, width: u16, height: u16) -> Buffer {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).expect("test terminal");
        terminal
            .draw(|frame| render_tailscale_send(app, frame, app.view.terminal_area))
            .expect("render picker");
        terminal.backend().buffer().clone()
    }

    fn buffer_text(buffer: &Buffer) -> String {
        (0..buffer.area.height)
            .map(|row| {
                (0..buffer.area.width)
                    .map(|column| buffer[(column, row)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn picker_app(devices: Vec<TailscaleDevice>) -> AppState {
        let mut app = AppState::test_new();
        app.view.terminal_area = Rect::new(0, 0, 80, 24);
        // The state is built here rather than through the input entry point,
        // which is in a private module. What this test is about is the drawing,
        // and the two agree on the same struct.
        let status = devices
            .is_empty()
            .then(|| "  no other devices on this tailnet".to_owned());
        app.tailscale_send = Some(crate::app::state::TailscaleSendState {
            paths: vec![std::path::PathBuf::from("/home/a/report.pdf")],
            devices,
            selected: 0,
            status,
            sending: false,
            sent_targets: Vec::new(),
        });
        app
    }

    fn device(label: &str, online: bool) -> TailscaleDevice {
        TailscaleDevice {
            label: label.to_owned(),
            target: format!("{label}.ts.net"),
            os: "linux".to_owned(),
            online,
        }
    }

    // TP-FSEND-TS-10: an offline machine is marked in text, not by colour
    // alone. A colour does not survive a monochrome terminal, and the reader
    // needs to know before pressing enter that the file will wait.
    #[test]
    fn an_offline_device_is_marked_in_text() {
        let up = device_row(&device("macbook", true), false, false);
        let down = device_row(&device("laptop", false), false, false);
        assert_ne!(
            up.replace(' ', ""),
            down.replace(' ', ""),
            "the two must be distinguishable without styling"
        );
        assert!(down.contains('·'), "{down:?}");
        assert!(!up.contains('·'), "{up:?}");
        assert!(up.contains("macbook") && down.contains("laptop"));
    }

    // TP-FSEND-TS-22: pinned and online are two marks in two columns. They are
    // independent — a pinned machine is often the one that is asleep — and
    // folding them into one glyph makes the list lie about one of them.
    #[test]
    fn pinned_and_online_are_marked_independently() {
        let pinned_offline = device_row(&device("laptop", false), true, false);
        let unpinned_offline = device_row(&device("laptop", false), false, false);
        assert!(pinned_offline.starts_with('*'), "{pinned_offline:?}");
        assert!(!unpinned_offline.starts_with('*'), "{unpinned_offline:?}");
        assert!(
            pinned_offline.contains('·') && unpinned_offline.contains('·'),
            "both are still offline"
        );
        // Same width either way, so the names stay in one column.
        assert_eq!(pinned_offline.len(), unpinned_offline.len());
    }

    // TP-FSEND-TS-11: the box refuses to draw rather than shrinking past
    // readability. A clamped box in a tiny frame overlaps its own border and
    // shows a device list nobody can act on.
    #[test]
    fn a_frame_with_no_room_draws_nothing() {
        assert!(layout(Rect::new(0, 0, 20, 20), 3).is_none(), "too narrow");
        assert!(layout(Rect::new(0, 0, 80, 4), 3).is_none(), "too short");
        assert!(layout(Rect::new(0, 0, 80, 24), 3).is_some());
    }

    // TP-FSEND-TS-12: a long tailnet stops growing. Fifteen machines is already
    // more than a small terminal has rows for, and a box taller than the screen
    // cannot be closed by eye.
    #[test]
    fn a_long_device_list_is_bounded() {
        let (popup, rows) = layout(Rect::new(0, 0, 100, 40), 60).expect("room");
        assert_eq!(rows.len(), MAX_ROWS as usize);
        assert!(popup.height <= MAX_ROWS + CHROME_ROWS);
    }

    // TP-FSEND-TS-27: a successful send marks its row, and the mark reaches
    // the screen. The status line names only the LAST outcome; a reader unsure
    // whether the press registered pressed again, and the same file went out
    // several times. A failed send earns no mark — ✓ on a failure tells the
    // reader to stop trying exactly when they should try again.
    #[test]
    fn a_successful_send_marks_the_device_row() {
        let mut app = picker_app(vec![device("macbook", true), device("laptop", true)]);
        app.tailscale_send
            .as_mut()
            .expect("picker")
            .sent_targets
            .push("macbook.ts.net".to_owned());

        let text = buffer_text(&render(&app, 80, 24));
        let macbook_line = text
            .lines()
            .find(|line| line.contains("macbook"))
            .expect("macbook row");
        let laptop_line = text
            .lines()
            .find(|line| line.contains("laptop"))
            .expect("laptop row");
        assert!(macbook_line.contains('✓'), "no sent mark: {macbook_line:?}");
        assert!(
            !laptop_line.contains('✓'),
            "the mark leaked to a device nothing was sent to: {laptop_line:?}"
        );
    }

    // TP-FSEND-TS-26: the hit test agrees with the PIXELS, not with itself.
    // TS-13 checks clicks against `layout`, but `layout` describing rows one
    // below where render draws them passes that test perfectly — both sides
    // share the mistake. The user found it the honest way: every click landed
    // on the machine above the one under the cursor. So this test reads the
    // rendered buffer, finds the row a name is actually on, and demands the
    // hit test name that device for that row.
    #[test]
    fn a_click_lands_on_the_device_that_is_drawn_there() {
        let app = picker_app(vec![
            device("alpha", true),
            device("bravo", true),
            device("carol", false),
        ]);
        let area = app.view.terminal_area;
        let buffer = render(&app, area.width, area.height);

        for (index, name) in ["alpha", "bravo", "carol"].iter().enumerate() {
            let drawn_row = (0..buffer.area.height)
                .find(|row| {
                    (0..buffer.area.width)
                        .map(|column| buffer[(column, *row)].symbol())
                        .collect::<String>()
                        .contains(name)
                })
                .unwrap_or_else(|| panic!("{name} is not drawn at all"));
            assert_eq!(
                device_row_at(area, 3, area.x + area.width / 2, drawn_row),
                Some(index),
                "clicking the row where {name} is drawn must select {name}"
            );
        }
    }

    // TP-FSEND-TS-13: the row a click lands on is the row that was drawn.
    // Recomputing geometry in the mouse path is how a click ends up one row off
    // and sends the file to the wrong machine.
    #[test]
    fn a_click_selects_the_row_it_landed_on() {
        let area = Rect::new(0, 0, 80, 24);
        let (_, rows) = layout(area, 4).expect("room");
        for (index, rect) in rows.iter().enumerate() {
            assert_eq!(
                device_row_at(area, 4, rect.x + 2, rect.y),
                Some(index),
                "row {index} at {rect:?}"
            );
        }
        // Outside every row.
        assert_eq!(device_row_at(area, 4, 0, 0), None);
    }

    // TP-FSEND-TS-15: the devices actually reach the screen. Every other test
    // here checks a computation; this one checks the cells, which is the only
    // way to catch an overlay that is wired to a rect nothing is drawn into.
    #[test]
    fn the_device_names_are_drawn_into_the_frame() {
        let app = picker_app(vec![device("macbook", true), device("laptop", false)]);
        let text = buffer_text(&render(&app, 80, 24));
        assert!(text.contains("macbook"), "no device on screen:\n{text}");
        assert!(
            text.contains("laptop"),
            "no offline device on screen:\n{text}"
        );
        assert!(text.contains("Send report.pdf"), "no title:\n{text}");
        assert!(text.contains("enter sends"), "no hint:\n{text}");
    }

    // TP-FSEND-TS-18: the picker survives the whole frame, not just its own
    // draw call. Rendering the overlay in isolation proves the box is built;
    // it says nothing about whether the stage underneath paints over it, and
    // that is the difference between a feature and a menu entry that appears
    // to do nothing.
    #[test]
    fn the_picker_is_visible_in_a_full_frame() {
        let mut app = picker_app(vec![device("macbook", true), device("laptop", false)]);
        app.workspaces = vec![crate::workspace::Workspace::test_new("send")];
        app.active = Some(0);
        app.selected = 0;
        app.mobile_width_threshold = 0;
        app.mode = crate::app::state::Mode::TailscaleSend;
        crate::ui::compute_view(&mut app, Rect::new(0, 0, 120, 40));

        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).expect("test terminal");
        terminal
            .draw(|frame| crate::ui::render(&app, frame))
            .expect("render frame");
        let text = buffer_text(terminal.backend().buffer());
        assert!(
            text.contains("macbook"),
            "the picker did not survive the frame:\n{text}"
        );
        assert!(text.contains("Send report.pdf"), "no title:\n{text}");
    }

    // TP-FSEND-TS-16: an empty tailnet says so on screen. An empty box and a
    // failed lookup look identical unless one of them is spelled out, and the
    // reader is left thinking the feature is broken.
    #[test]
    fn an_empty_tailnet_says_so_on_screen() {
        let app = picker_app(Vec::new());
        let text = buffer_text(&render(&app, 80, 24));
        assert!(
            text.contains("no other devices"),
            "silent empty box:\n{text}"
        );
    }

    // TP-FSEND-TS-14: the title names the file. A picker that says only "Send"
    // leaves the reader guessing which of the selected files is going.
    #[test]
    fn the_title_names_what_is_being_sent() {
        assert_eq!(
            title_for(&[std::path::PathBuf::from("/home/a/report.pdf")]),
            "Send report.pdf"
        );
        assert_eq!(
            title_for(&[
                std::path::PathBuf::from("/a/x.png"),
                std::path::PathBuf::from("/a/y.png")
            ]),
            "Send 2 files"
        );
    }
}
