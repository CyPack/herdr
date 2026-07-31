use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use super::size_class::{HeightClass, SizeClass, WidthClass};
use crate::app::state::Palette;

pub(super) fn render_panel_shell(
    frame: &mut Frame,
    area: Rect,
    border_color: Color,
    bg: Color,
) -> Option<Rect> {
    if area.width < 2 || area.height < 2 {
        return None;
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .border_set(ratatui::symbols::border::PLAIN)
        .style(Style::default().bg(bg));
    let inner = block.inner(area);
    frame.render_widget(Clear, area);
    frame.render_widget(block, area);
    Some(inner)
}

pub(super) fn panel_contrast_fg(p: &Palette) -> Color {
    match p.panel_bg {
        Color::Reset => p.surface_dim,
        color => color,
    }
}

/// Columns a floating popup leaves on each side of itself.
///
/// A margin is what makes a popup read as floating above the surface behind
/// it. That reading is worth two columns on a desktop terminal and worth
/// nothing on a phone held upright, where four columns of margin is a tenth of
/// the screen and the popup has stopped looking like a floating box anyway.
/// The tight case is the terminal's version of the compact-viewport rule that
/// turns a side panel into a full-width sheet.
fn popup_margin_x(width: WidthClass) -> u16 {
    match width {
        WidthClass::Tight => 0,
        WidthClass::Compact => 1,
        WidthClass::Regular => 2,
    }
}

/// Rows a floating popup leaves above and below itself.
fn popup_margin_y(height: HeightClass) -> u16 {
    match height {
        HeightClass::Short => 0,
        HeightClass::Regular => 1,
    }
}

/// The width a popup declaring `popup_w` actually gets inside `area`.
///
/// Split out from [`centered_popup_rect`] for popups whose height depends on
/// how their content wraps: the height is an input to `centered_popup_rect`,
/// but the wrapping that determines it needs the width that call would return.
/// Asking for the width first breaks the cycle.
pub(crate) fn popup_width_for(area: Rect, popup_w: u16) -> u16 {
    let size = SizeClass::of_viewport(area);
    popup_w.min(area.width.saturating_sub(2 * popup_margin_x(size.width)))
}

pub(crate) fn centered_popup_rect(area: Rect, popup_w: u16, popup_h: u16) -> Option<Rect> {
    let size = SizeClass::of_viewport(area);
    let popup_w = popup_width_for(area, popup_w);
    let popup_h = popup_h.min(area.height.saturating_sub(2 * popup_margin_y(size.height)));
    if popup_w < 4 || popup_h < 4 {
        return None;
    }

    let popup_x = area.x + (area.width.saturating_sub(popup_w)) / 2;
    let popup_y = area.y + (area.height.saturating_sub(popup_h)) / 2;
    Some(Rect::new(popup_x, popup_y, popup_w, popup_h))
}

/// The widest "this does not fit" line that fits `width`.
///
/// The exit key is in every variant, because the reader of this line is by
/// definition someone who opened something they cannot see and needs to know
/// they are not stuck.
fn too_small_text(title: &str, width: u16) -> String {
    let width = width as usize;
    for candidate in [
        format!(" {title} · too small · esc"),
        format!(" {title} · esc"),
        " esc".to_string(),
    ] {
        if candidate.chars().count() <= width {
            return candidate;
        }
    }
    super::text::truncate_end(" esc", width)
}

/// Draw a one-line notice in place of a modal the viewport cannot hold.
///
/// The alternative — the one this replaces — was to return without drawing
/// anything. The mode stayed active, so the terminal showed the surface behind
/// an overlay that was not there, and every keystroke went to an overlay the
/// reader could not see. An unexplained blank is worse than either an empty
/// state or an error, because it gives the reader nothing to act on.
pub(super) fn render_too_small_notice(frame: &mut Frame, area: Rect, title: &str, p: &Palette) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let strip = Rect::new(area.x, area.y + area.height / 2, area.width, 1);
    frame.render_widget(Clear, strip);
    frame.render_widget(
        Paragraph::new(too_small_text(title, strip.width)).style(
            Style::default()
                .fg(p.text)
                .bg(p.surface0)
                .add_modifier(Modifier::BOLD),
        ),
        strip,
    );
}

/// Draw a modal frame, or a notice saying the viewport cannot hold one.
///
/// `min_inner` is the interior the caller needs before its body is worth
/// drawing; below it the caller used to return silently, which is the same
/// blank screen by a different route.
pub(super) fn render_modal_shell_or_notice(
    frame: &mut Frame,
    area: Rect,
    popup_w: u16,
    popup_h: u16,
    title: &str,
    min_inner: (u16, u16),
    p: &Palette,
) -> Option<Rect> {
    let inner = centered_popup_rect(area, popup_w, popup_h)
        .and_then(|popup| render_panel_shell(frame, popup, p.accent, p.panel_bg))
        .filter(|inner| inner.width >= min_inner.0 && inner.height >= min_inner.1);
    if inner.is_none() {
        render_too_small_notice(frame, area, title, p);
    }
    inner
}

pub(super) fn render_modal_header(frame: &mut Frame, area: Rect, title: &str, p: &Palette) {
    let line = Line::from(vec![Span::styled(
        title,
        Style::default().fg(p.text).add_modifier(Modifier::BOLD),
    )]);
    frame.render_widget(Paragraph::new(line), area);
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ModalStackAreas {
    pub header: Rect,
    pub content: Rect,
    pub footer: Option<Rect>,
    pub actions: Option<Rect>,
}

pub(crate) fn modal_stack_areas(
    inner: Rect,
    header_height: u16,
    footer_height: u16,
    actions_height: u16,
    gap: u16,
) -> ModalStackAreas {
    #[derive(Clone, Copy)]
    enum Slot {
        Header,
        Content,
        Footer,
        Actions,
    }

    let mut constraints = Vec::new();
    let mut slots = Vec::new();
    let mut push = |slot: Slot, constraint: Constraint| {
        if !slots.is_empty() {
            constraints.push(Constraint::Length(gap));
        }
        constraints.push(constraint);
        slots.push(slot);
    };

    push(Slot::Header, Constraint::Length(header_height));
    push(Slot::Content, Constraint::Min(0));
    if footer_height > 0 {
        push(Slot::Footer, Constraint::Length(footer_height));
    }
    if actions_height > 0 {
        push(Slot::Actions, Constraint::Length(actions_height));
    }

    let areas = Layout::vertical(constraints).split(inner);
    let mut header = Rect::default();
    let mut content = Rect::default();
    let mut footer = None;
    let mut actions = None;

    for (slot, area) in slots.into_iter().zip(areas.iter().step_by(2).copied()) {
        match slot {
            Slot::Header => header = area,
            Slot::Content => content = area,
            Slot::Footer => footer = Some(area),
            Slot::Actions => actions = Some(area),
        }
    }

    ModalStackAreas {
        header,
        content,
        footer,
        actions,
    }
}

pub(crate) fn action_button_text(hint: Option<&str>, label: &str) -> String {
    match hint {
        Some(hint) => format!(" {hint} {label} "),
        None => format!(" {label} "),
    }
}

pub(crate) fn action_button_width(hint: Option<&str>, label: &str) -> u16 {
    action_button_text(hint, label).chars().count() as u16
}

pub(crate) struct ActionButtonSpec<'a> {
    pub hint: Option<&'a str>,
    pub label: &'a str,
}

pub(crate) fn action_button_row_rects(
    area: Rect,
    buttons: &[ActionButtonSpec<'_>],
    gap: u16,
    row_offset: u16,
) -> Vec<Rect> {
    let widths: Vec<u16> = buttons
        .iter()
        .map(|button| action_button_width(button.hint, button.label))
        .collect();
    centered_button_row(area, &widths, gap, row_offset)
}

pub(super) fn render_action_button(
    frame: &mut Frame,
    rect: Rect,
    hint: Option<&str>,
    label: &str,
    style: Style,
) {
    frame.render_widget(
        Paragraph::new(action_button_text(hint, label))
            .style(style)
            .alignment(Alignment::Center),
        rect,
    );
}

pub(crate) fn render_modal_description(frame: &mut Frame, area: Rect, text: &str, style: Style) {
    frame.render_widget(
        Paragraph::new(format!(" {text}"))
            .style(style)
            .wrap(Wrap { trim: false }),
        area,
    );
}

pub(crate) fn modal_choice_rows(area: Rect, count: usize, row_height: u16) -> Vec<Rect> {
    let mut rows = Vec::with_capacity(count);
    let mut y = area.y;
    for _ in 0..count {
        if y >= area.y + area.height {
            break;
        }
        let remaining = area.y + area.height - y;
        let height = row_height.min(remaining);
        rows.push(Rect::new(area.x, y, area.width, height));
        y = y.saturating_add(row_height);
    }
    rows
}

pub(crate) fn render_modal_choice_list<T>(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    description: &str,
    options: &[(&str, T)],
    current_value: T,
    selected_idx: usize,
    p: &Palette,
    row_height: u16,
) where
    T: Copy + PartialEq,
{
    let [desc_area, _, list_area] = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(1),
        Constraint::Min(2),
    ])
    .areas::<3>(area);

    render_modal_description(
        frame,
        desc_area,
        description,
        Style::default().fg(p.overlay1),
    );

    let rows = modal_choice_rows(list_area, options.len(), row_height);
    for (idx, ((label, value), row)) in options.iter().zip(rows.iter()).enumerate() {
        let is_active = *value == current_value;
        let is_selected = idx == selected_idx;
        let marker = if is_active { " ✓" } else { "" };
        let style = if is_selected {
            Style::default()
                .bg(p.surface0)
                .fg(p.text)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(p.subtext0)
        };
        frame.render_widget(
            Paragraph::new(format!(" {title}: {label}{marker}"))
                .style(style)
                .wrap(Wrap { trim: false }),
            *row,
        );
    }
}

pub(super) fn centered_button_row(
    inner: Rect,
    widths: &[u16],
    gap: u16,
    row_offset: u16,
) -> Vec<Rect> {
    let total_w = widths
        .iter()
        .copied()
        .sum::<u16>()
        .saturating_add(gap.saturating_mul(widths.len().saturating_sub(1) as u16));
    let mut x = inner.x + inner.width.saturating_sub(total_w) / 2;
    let y = inner.y + row_offset.min(inner.height.saturating_sub(1));
    widths
        .iter()
        .map(|w| {
            let rect = Rect::new(
                x,
                y,
                (*w).min(inner.width.saturating_sub(x.saturating_sub(inner.x))),
                1,
            );
            x = x.saturating_add(*w).saturating_add(gap);
            rect
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The geometry `centered_popup_rect` produced before it became size-class
    /// aware. Kept here, spelled out, so the desktop characterization test
    /// compares against a fixed rule rather than against the code under test.
    fn legacy_centered_popup_rect(area: Rect, popup_w: u16, popup_h: u16) -> Option<Rect> {
        let popup_w = popup_w.min(area.width.saturating_sub(4));
        let popup_h = popup_h.min(area.height.saturating_sub(2));
        if popup_w < 4 || popup_h < 4 {
            return None;
        }
        let popup_x = area.x + (area.width.saturating_sub(popup_w)) / 2;
        let popup_y = area.y + (area.height.saturating_sub(popup_h)) / 2;
        Some(Rect::new(popup_x, popup_y, popup_w, popup_h))
    }

    // TP-MOB-06: every viewport that already took the desktop shell keeps the
    // exact popup geometry it had. Narrow-screen work must not move a single
    // desktop cell. The compact band (41..=64) is deliberately excluded: it is
    // already the mobile shell, and halving its margin is the point of
    // TP-MOB-08.
    #[test]
    fn regular_viewport_popup_geometry_is_unchanged() {
        let declared = [(76u16, 22u16), (96, 30), (68, 12), (56, 7), (64, 6)];
        for width in (crate::config::DEFAULT_MOBILE_WIDTH_THRESHOLD + 1)..=200u16 {
            for height in (crate::ui::size_class::SHORT_MAX_HEIGHT + 1)..=60u16 {
                let area = Rect::new(0, 0, width, height);
                for (w, h) in declared {
                    assert_eq!(
                        centered_popup_rect(area, w, h),
                        legacy_centered_popup_rect(area, w, h),
                        "declared {w}x{h} in a {width}x{height} viewport"
                    );
                }
            }
        }
    }

    // TP-MOB-07: on a phone held upright the popup stops pretending to float
    // and takes the whole width, because four columns of margin there is a
    // tenth of the screen.
    #[test]
    fn tight_viewport_popup_spans_the_full_width() {
        let area = Rect::new(0, 0, 36, 18);
        let rect = centered_popup_rect(area, 76, 22).expect("tight popup");
        assert_eq!(rect.x, 0);
        assert_eq!(rect.width, 36);
    }

    // TP-MOB-08: a compact viewport keeps one column of margin — enough to
    // read as a panel, half the cost of the desktop margin.
    #[test]
    fn compact_viewport_popup_keeps_one_column_of_margin() {
        let area = Rect::new(0, 0, 44, 22);
        let rect = centered_popup_rect(area, 76, 22).expect("compact popup");
        assert_eq!(rect.width, 42);
        assert_eq!(rect.x, 1);
    }

    // TP-MOB-09: a phone held sideways is wide but very short, so the popup
    // spends no rows on vertical margin.
    #[test]
    fn short_viewport_popup_drops_the_vertical_margin() {
        let area = Rect::new(0, 0, 90, 14);
        let rect = centered_popup_rect(area, 76, 22).expect("short popup");
        assert_eq!(rect.height, 14);
        assert_eq!(rect.y, 0);
        assert_eq!(rect.width, 76, "a short viewport is not a narrow one");
    }

    // TP-MOB-10: a viewport too small for a bordered body still refuses,
    // rather than returning a rect nothing can be drawn into.
    #[test]
    fn impossible_viewport_still_returns_none() {
        assert_eq!(centered_popup_rect(Rect::new(0, 0, 3, 3), 76, 22), None);
        assert_eq!(centered_popup_rect(Rect::new(0, 0, 0, 0), 76, 22), None);
        assert_eq!(centered_popup_rect(Rect::new(0, 0, 36, 3), 76, 22), None);
    }

    // TP-MOB-22: a viewport too small for a modal draws a notice naming the
    // overlay and its exit key, never a blank. Returning without drawing left
    // the mode active over a surface with no overlay on it, so every keystroke
    // went somewhere the reader could not see.
    #[test]
    fn a_modal_too_small_to_draw_leaves_a_notice() {
        use ratatui::{backend::TestBackend, Terminal};

        let palette = crate::app::state::Palette::catppuccin();
        // Narrow enough that even a full-width popup leaves less interior than
        // the help body needs. A 24x10 viewport now *does* fit it, which is
        // itself what giving tight viewports the full width bought.
        let area = Rect::new(0, 0, 16, 8);
        let mut terminal = Terminal::new(TestBackend::new(area.width, area.height))
            .expect("test terminal should initialize");
        terminal
            .draw(|frame| {
                assert!(
                    render_modal_shell_or_notice(
                        frame,
                        area,
                        76,
                        22,
                        "keybinds",
                        (20, 6),
                        &palette
                    )
                    .is_none(),
                    "this viewport cannot hold the modal"
                );
            })
            .expect("draw");

        let rendered: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect();
        assert!(
            rendered.contains("keybinds"),
            "the notice names the overlay that could not be drawn: {rendered:?}"
        );
        assert!(
            rendered.contains("esc"),
            "the notice names the key that gets the reader out: {rendered:?}"
        );
    }

    // TP-MOB-23: the notice keeps the exit key at every width and never
    // overruns its strip.
    #[test]
    fn the_too_small_notice_always_names_the_exit_and_fits() {
        for width in 1..=60u16 {
            let text = too_small_text("keybinds", width);
            assert!(
                text.chars().count() <= width as usize,
                "notice {text:?} overruns its {width}-column strip"
            );
            if width >= 4 {
                assert!(
                    text.contains("esc"),
                    "notice {text:?} at width {width} drops the exit key"
                );
            }
        }
    }

    // TP-MOB-24: drawing the notice never panics, down to a one-cell frame.
    #[test]
    fn the_too_small_notice_survives_a_one_cell_frame() {
        use ratatui::{backend::TestBackend, Terminal};

        let palette = crate::app::state::Palette::catppuccin();
        for (w, h) in [(1u16, 1u16), (2, 1), (1, 2), (5, 3), (24, 10)] {
            let area = Rect::new(0, 0, w, h);
            let mut terminal =
                Terminal::new(TestBackend::new(w, h)).expect("test terminal should initialize");
            terminal
                .draw(|frame| render_too_small_notice(frame, area, "settings", &palette))
                .expect("draw");
        }
    }
}
