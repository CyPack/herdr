use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use super::shell::BarTint;
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

    // Rounded, because everything drawn through this shell floats above the
    // surface behind it, and what floats is drawn as floating — the pattern
    // the reference file managers keep (yazi rounds exactly its overlays and
    // leaves its panes plain), adopted here by the reader's explicit call
    // (TP-SUR-SHELL-01). Static border cells are free in the frame diff.
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .border_set(ratatui::symbols::border::ROUNDED)
        .style(Style::default().bg(bg));
    let inner = block.inner(area);
    frame.render_widget(Clear, area);
    frame.render_widget(block, area);
    Some(inner)
}

/// Draw one edge bar's own shell and hand back what is left inside it.
///
/// Rounded, like every other framed surface here, and BOLD rather than the
/// heavy box-drawing set: Unicode has no thick *rounded* corner, so asking for
/// `┏` would square the corners off. Bold keeps `╭╮╰╯` and lets the terminal
/// render the run with weight, which is the only way to have both.
// TP-CHROME-11: rounded corners; weight comes from bold, because no
// thick-and-rounded glyph exists.
pub(crate) fn render_bar_shell(
    frame: &mut Frame,
    area: Rect,
    tint: BarTint,
    bg: Color,
) -> Option<Rect> {
    if area.width < 2 || area.height < 2 {
        return None;
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(
            Style::default()
                .fg(tint.at(0, 1))
                .add_modifier(Modifier::BOLD),
        )
        .border_set(ratatui::symbols::border::ROUNDED)
        .style(Style::default().bg(bg));
    let inner = block.inner(area);
    frame.render_widget(Clear, area);
    frame.render_widget(block, area);

    // The block draws the glyphs; the fade only re-tints them, so the shape is
    // never a function of the colour. It walks the long axis, because a
    // gradient across the short one would finish inside a single cell.
    let horizontal = area.width >= area.height;
    let span = if horizontal { area.width } else { area.height };
    let buffer = frame.buffer_mut();
    for y in area.y..area.y.saturating_add(area.height) {
        for x in area.x..area.x.saturating_add(area.width) {
            let on_border = x == area.x
                || x + 1 == area.x + area.width
                || y == area.y
                || y + 1 == area.y + area.height;
            if !on_border {
                continue;
            }
            let position = if horizontal { x - area.x } else { y - area.y };
            if let Some(cell) = buffer.cell_mut((x, y)) {
                cell.set_fg(tint.at(position, span));
            }
        }
    }

    Some(inner)
}

/// The smallest framed surface: a control that reads as a button.
///
/// Three rows, because a rounded frame spends one on each side and the label
/// needs the one between them. That cost is why this returns `None` instead of
/// drawing something shorter — a caller that cannot afford a box has to know,
/// so it can fall back to a cheaper style rather than ship half a border.
///
/// The drawn rectangle comes back so the caller registers the same rectangle it
/// painted. Hit testing that recomputes its own is how a button ends up
/// clickable one cell away from where it looks.
// TP-CHROME-22: a chip clips rather than wraps, and refuses rather than
// drawing half a frame.
pub(crate) fn render_chip(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    tint: BarTint,
    label_style: Style,
    bg: Color,
) -> Option<Rect> {
    let width = chip_width(label).min(area.width);
    if area.height < CHIP_ROWS || width < 3 {
        return None;
    }

    let chip = Rect::new(area.x, area.y, width, CHIP_ROWS);
    let inner = render_bar_shell(frame, chip, tint, bg)?;
    // Clipped rather than wrapped: a second row would break the frame this
    // function just promised.
    let text: String = label.chars().take(usize::from(inner.width)).collect();
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(text, label_style))).alignment(Alignment::Center),
        inner,
    );
    Some(chip)
}

/// Draw one bar section's widget into the rectangle that section was given.
///
/// Clipped by DISPLAY width rather than character count: a section is measured
/// in cells and an emoji is two of them, so counting characters would let a
/// label written with icons overrun the rectangle into its neighbour. The fork
/// already learned this once for file columns (TP-FSH-10).
///
/// An empty rectangle draws nothing rather than being a special case anywhere
/// else (CL9), and a widget never changes the rectangle it was handed — the
/// size was decided by the layout solver before this function was called.
// TP-CHROME-52/53: a label is drawn inside its own section, clipped by display
// width, and an empty rectangle is a no-op.
#[allow(clippy::too_many_arguments)] // one parameter per live source, and each
                                     // one is a reading taken elsewhere; folding
                                     // them into a struct would hide which
                                     // widgets need which.
pub(crate) fn render_section_widget(
    frame: &mut Frame,
    widget: &crate::ui::shell::SectionWidget,
    resources: &crate::resource::ResourceSample,
    history: &crate::resource::ResourceHistory,
    now: Option<time::OffsetDateTime>,
    palette: &Palette,
    area: Rect,
    style: Style,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let text = match widget {
        crate::ui::shell::SectionWidget::None => return,
        // A picture is the one widget that wants more than a line, and the
        // rectangle it needs was always there — `section_rects` hands every
        // section the bar's full inner height. Only this function was throwing
        // the rest away.
        crate::ui::shell::SectionWidget::Art { art } => {
            render_icon_art(frame, art, palette, area);
            return;
        }
        crate::ui::shell::SectionWidget::Meter { metric } => {
            render_meter(frame, resources, *metric, palette, area);
            return;
        }
        crate::ui::shell::SectionWidget::Sparkline { metric } => {
            render_sparkline(frame, history, *metric, palette, area);
            return;
        }
        crate::ui::shell::SectionWidget::Icon { glyph } => {
            std::borrow::Cow::Borrowed(glyph.as_str())
        }
        // The reading arrives already taken, like the sample below it. A clock
        // that called `now()` here would draw correctly and repaint its cells on
        // every frame, which is the failure `resource` was shaped to avoid.
        //
        // An unreadable local zone draws nothing rather than falling back to
        // UTC: a clock quietly showing another country's time is worse than an
        // empty section, because nothing about it looks wrong.
        crate::ui::shell::SectionWidget::Clock { format } => {
            let Some(now) = now else {
                return;
            };
            std::borrow::Cow::Owned(format.render(now))
        }
        crate::ui::shell::SectionWidget::Label { text } => {
            if text.is_empty() {
                return;
            }
            std::borrow::Cow::Borrowed(text.as_str())
        }
        // The sample arrives already taken. This arm formats it and nothing
        // else — no reading, no clock, no cache — which is what makes "the
        // renderer never samples" a property of the code rather than a promise
        // in a comment.
        crate::ui::shell::SectionWidget::Resource { metric } => {
            std::borrow::Cow::Owned(crate::resource::metric_text(resources, *metric))
        }
    };

    let clipped = super::text::truncate_end(&text, usize::from(area.width));
    let line = Rect::new(area.x, area.y, area.width, 1);
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(clipped, style))),
        line,
    );
}

/// Paints a picture, two pixels to a cell.
///
/// `▀` puts the upper pixel in the foreground and lets the background show
/// through below it; `▄` is the same the other way up and is what a cell with a
/// transparent top uses, so a missing pixel never needs an invented colour. A
/// cell with neither pixel is skipped entirely, which is what keeps the bar's
/// own surface visible behind the shape.
///
/// Colours resolve here rather than at config time, against the live palette,
/// so a theme change recolours the picture without re-deriving any geometry.
///
/// Anything past the rectangle is dropped. That is the same thing a label does
/// when the window narrows: a config declaring a width too small is refused
/// where it is written, but a terminal that shrinks at runtime cannot be
/// refused, only survived.
// TP-ART-01/02: the upper pixel is the foreground, the picture paints every
// row it was given and none outside it, and the same picture drawn twice
// produces identical cells so the diff sends nothing.
fn render_icon_art(frame: &mut Frame, art: &crate::icon::IconArt, palette: &Palette, area: Rect) {
    const UPPER_HALF: &str = "▀";
    const LOWER_HALF: &str = "▄";

    let rows = art.height().min(area.height);
    let columns = art.width().min(area.width);
    let buffer = frame.buffer_mut();

    for row in 0..rows {
        for column in 0..columns {
            let Some(half) = art.cell(column, row) else {
                continue;
            };
            let upper = half.upper.and_then(|index| art.spec(index));
            let lower = half.lower.and_then(|index| art.spec(index));
            let Some(cell) = buffer.cell_mut((area.x + column, area.y + row)) else {
                continue;
            };
            match (upper, lower) {
                (None, None) => {}
                (Some(spec), None) => {
                    cell.set_symbol(UPPER_HALF);
                    cell.set_fg(super::shell::bar_color(spec, palette));
                }
                (None, Some(spec)) => {
                    cell.set_symbol(LOWER_HALF);
                    cell.set_fg(super::shell::bar_color(spec, palette));
                }
                (Some(top), Some(bottom)) => {
                    cell.set_symbol(UPPER_HALF);
                    cell.set_fg(super::shell::bar_color(top, palette));
                    cell.set_bg(super::shell::bar_color(bottom, palette));
                }
            }
        }
    }
}

/// Paints a filled bar across the section, coloured by how full it is.
///
/// Every row of the rectangle is filled, so the bar reads as a block of colour
/// rather than a line — which is what makes a glance enough. Whole cells are
/// `\u{2588}`; the cell after them carries the remainder as an eighth-block, so the
/// bar moves smoothly instead of jumping a whole cell at a time.
///
/// A metric with no ratio — an unreadable counter, or a pool the machine does
/// not have — draws NOTHING. An empty bar would say "plenty free" about
/// something that is absent or unknown, which is the same lie a fabricated 0%
/// would be.
///
/// The cost story is unchanged from every other widget here: this reads a
/// sample that was already taken, so the bar only changes when the sample does,
/// and an unchanged bar costs nothing in the frame diff.
// TP-METER-01/02: every row is filled, the bar never overruns its rectangle,
// and a metric with no ratio draws nothing at all.
/// One column per reading, newest on the right, growing from the bottom.
///
/// Right-aligned rather than left, and that is the whole reading order: the
/// newest sample sits where the eye lands, and a herdr just opened grows its
/// history leftward into the empty half instead of shunting the newest column
/// sideways on every tick.
///
/// The arithmetic is `meter_cells`, unchanged — a bar filled upward is the same
/// division as one filled sideways, with the section's height where its width
/// would be. Only the glyph table differs.
// TP-SPARK-03/04/05: newest at the right, short histories right-aligned, and
// every column grows from the bottom of the section.
fn render_sparkline(
    frame: &mut Frame,
    history: &crate::resource::ResourceHistory,
    metric: crate::resource::ResourceMetric,
    palette: &Palette,
    area: Rect,
) {
    let series = history.series(metric);
    let width = usize::from(area.width);
    // Only what fits, and the newest end of it.
    let shown = series.len().min(width);
    let skipped = series.len() - shown;
    let left_pad = u16::try_from(width - shown).unwrap_or(0);

    for (offset, ratio) in series.iter().skip(skipped).enumerate() {
        // A reading that could not be taken draws nothing at all. A reading of
        // zero draws the thinnest mark there is, because "idle" and "no idea"
        // must not look the same.
        let Some(ratio) = ratio else {
            continue;
        };
        let column = area.x + left_pad + u16::try_from(offset).unwrap_or(0);
        let colour = super::shell::bar_color(crate::resource::meter_colour(*ratio), palette);
        let (full, eighths) = crate::resource::meter_cells(*ratio, area.height);
        let partial = crate::resource::lower_eighth_block(eighths);

        let buffer = frame.buffer_mut();
        // Full cells first, counted up from the bottom row.
        for filled in 0..full {
            let row = area.y + area.height - 1 - filled;
            if let Some(cell) = buffer.cell_mut((column, row)) {
                cell.set_symbol("\u{2588}");
                cell.set_fg(colour);
            }
        }
        // Then the partial cell sitting on top of them. A value of exactly zero
        // has no full cells and no eighths, so it lands here as the first
        // eighth rather than as nothing.
        let symbol = partial.unwrap_or(if full == 0 { "\u{2581}" } else { "" });
        if !symbol.is_empty() && full < area.height {
            let row = area.y + area.height - 1 - full;
            if let Some(cell) = buffer.cell_mut((column, row)) {
                cell.set_symbol(symbol);
                cell.set_fg(colour);
            }
        }
    }
}

fn render_meter(
    frame: &mut Frame,
    resources: &crate::resource::ResourceSample,
    metric: crate::resource::ResourceMetric,
    palette: &Palette,
    area: Rect,
) {
    const FULL: &str = "\u{2588}";

    let Some(ratio) = crate::resource::meter_ratio(resources, metric) else {
        return;
    };
    let (full, eighths) = crate::resource::meter_cells(ratio, area.width);
    let colour = super::shell::bar_color(crate::resource::meter_colour(ratio), palette);
    let partial = crate::resource::eighth_block(eighths);

    let buffer = frame.buffer_mut();
    for row in 0..area.height {
        for column in 0..full {
            if let Some(cell) = buffer.cell_mut((area.x + column, area.y + row)) {
                cell.set_symbol(FULL);
                cell.set_fg(colour);
            }
        }
        if let Some(symbol) = partial {
            if full < area.width {
                if let Some(cell) = buffer.cell_mut((area.x + full, area.y + row)) {
                    cell.set_symbol(symbol);
                    cell.set_fg(colour);
                }
            }
        }
    }
}

/// Rows a boxed chip occupies: border, label, border.
pub(crate) const CHIP_ROWS: u16 = 3;

/// Columns a chip adds around its label: the frame's two, plus a space either
/// side so the text never touches the border.
pub(crate) const CHIP_SIDE_CELLS: u16 = 4;

/// Cells a boxed chip wants for a label.
pub(crate) fn chip_width(label: &str) -> u16 {
    let label_cells = u16::try_from(label.chars().count()).unwrap_or(u16::MAX);
    label_cells.saturating_add(CHIP_SIDE_CELLS)
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

    fn draw<F: FnOnce(&mut Frame)>(width: u16, height: u16, f: F) -> ratatui::buffer::Buffer {
        use ratatui::{backend::TestBackend, Terminal};
        let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("test backend");
        terminal.draw(|frame| f(frame)).expect("draw");
        terminal.backend().buffer().clone()
    }

    fn peach() -> BarTint {
        BarTint::solid(Color::Rgb(250, 179, 135))
    }

    /// A history whose readings are the ratios given, oldest first.
    fn history_of(ratios: &[Option<f32>]) -> crate::resource::ResourceHistory {
        let mut history = crate::resource::ResourceHistory::default();
        for ratio in ratios {
            history.push(&crate::resource::ResourceSample {
                cpu: ratio.map(|ratio| ratio * 100.0),
                ..Default::default()
            });
        }
        history
    }

    /// One row of a drawn sparkline, as text.
    fn sparkline_row(ratios: &[Option<f32>], width: u16, height: u16, row: u16) -> String {
        let history = history_of(ratios);
        let buffer = draw(width, height, |frame| {
            render_sparkline(
                frame,
                &history,
                crate::resource::ResourceMetric::Cpu,
                &crate::app::state::Palette::catppuccin(),
                Rect::new(0, 0, width, height),
            );
        });
        (0..width)
            .filter_map(|x| buffer.cell((x, row)).map(|c| c.symbol().to_string()))
            .collect()
    }

    // TP-SPARK-02: a reading that could not be taken and a reading of zero must
    // not draw the same thing.
    //
    // One pixel apart and opposite in meaning: "the machine was idle" against
    // "we have no idea". If they collapsed, a bar would report an idle machine
    // it had never measured, and nothing on screen would say so.
    #[test]
    fn an_unread_column_is_blank_and_a_zero_column_is_the_thinnest_mark() {
        let row = sparkline_row(&[None, Some(0.0)], 2, 1, 0);
        assert_eq!(row, " ▁", "unread and zero drew the same thing: {row:?}");
    }

    // TP-SPARK-03: with more readings than columns, the newest survive.
    //
    // A sparkline answers "what has it been doing lately". Keeping the oldest
    // readings would answer a question nobody asked, and would look identical.
    #[test]
    fn a_history_longer_than_the_section_keeps_its_newest_readings() {
        // Eight readings climbing to full, in a section three columns wide.
        let ratios: Vec<Option<f32>> = (1..=8)
            .map(|step| Some(f32::from(step as u8) / 8.0))
            .collect();
        let row = sparkline_row(&ratios, 3, 1, 0);
        assert_eq!(
            row, "▆▇█",
            "the oldest readings were drawn instead of the newest: {row:?}"
        );
    }

    // TP-SPARK-04: with fewer readings than columns, they sit on the right.
    //
    // A herdr just opened grows its history leftward into the empty half. Left
    // alignment would shunt the newest column sideways on every reading, which
    // reads as the whole graph sliding rather than as one new sample.
    #[test]
    fn a_history_shorter_than_the_section_is_right_aligned() {
        let row = sparkline_row(&[Some(1.0), Some(1.0)], 5, 1, 0);
        assert_eq!(
            row, "   ██",
            "a short history was not right-aligned: {row:?}"
        );
    }

    // TP-SPARK-05: columns grow from the bottom.
    //
    // Gravity. A graph filled from the top is not a graph anybody reads, and the
    // mistake is invisible in a one-row bar — which is exactly where this would
    // otherwise have been tested.
    #[test]
    fn a_column_fills_upward_from_the_bottom_row() {
        // Half full in a four-row section: two full cells at the bottom.
        let ratios = [Some(0.5)];
        assert_eq!(sparkline_row(&ratios, 1, 4, 3), "█", "bottom row unfilled");
        assert_eq!(sparkline_row(&ratios, 1, 4, 2), "█", "second row unfilled");
        assert_eq!(sparkline_row(&ratios, 1, 4, 1), " ", "third row filled");
        assert_eq!(sparkline_row(&ratios, 1, 4, 0), " ", "top row filled");
    }

    // T51/T52 · a control has to stay readable inside the frame that makes it
    // look like a control.
    #[test]
    fn a_chip_keeps_its_label_inside_its_own_frame() {
        let mut drawn = None;
        let buffer = draw(20, 5, |frame| {
            drawn = render_chip(
                frame,
                Rect::new(0, 0, 20, 5),
                "menu",
                peach(),
                Style::default(),
                Color::Reset,
            );
        });
        let chip = drawn.expect("a chip fits in twenty by five");
        assert_eq!(chip.height, CHIP_ROWS);
        assert_eq!(chip.width, chip_width("menu"));

        let row: String = (chip.x..chip.x + chip.width)
            .filter_map(|x| buffer.cell((x, chip.y + 1)).map(|c| c.symbol().to_string()))
            .collect();
        assert!(
            row.contains("menu"),
            "the label survived its own border: {row:?}"
        );
        assert!(
            row.starts_with('│') && row.ends_with('│'),
            "and stayed inside it"
        );

        // The frame is a shape, not a function of the text.
        assert_eq!(buffer.cell((chip.x, chip.y)).unwrap().symbol(), "╭");
        assert_eq!(
            buffer
                .cell((chip.x + chip.width - 1, chip.y + CHIP_ROWS - 1))
                .unwrap()
                .symbol(),
            "╯"
        );
    }

    // T52 · too narrow clips the label; the corners are not negotiable.
    #[test]
    fn a_narrow_chip_clips_its_label_rather_than_its_frame() {
        let mut drawn = None;
        let buffer = draw(8, 3, |frame| {
            drawn = render_chip(
                frame,
                Rect::new(0, 0, 8, 3),
                "a very long control name",
                peach(),
                Style::default(),
                Color::Reset,
            );
        });
        let chip = drawn.expect("eight columns still hold a chip");
        assert_eq!(chip.width, 8, "the chip takes what it is given, no more");
        assert_eq!(buffer.cell((chip.x, chip.y)).unwrap().symbol(), "╭");
        assert_eq!(
            buffer.cell((chip.x + 7, chip.y + 2)).unwrap().symbol(),
            "╯",
            "a clipped label must not eat the corner"
        );
    }

    // T53 · a caller that cannot afford a box is told, not handed half of one.
    #[test]
    fn a_chip_refuses_a_space_too_short_for_its_frame() {
        for (w, h) in [(20, 2), (20, 1), (2, 3)] {
            let mut drawn = Some(Rect::default());
            let _ = draw(20.max(w), 3.max(h), |frame| {
                drawn = render_chip(
                    frame,
                    Rect::new(0, 0, w, h),
                    "new",
                    peach(),
                    Style::default(),
                    Color::Reset,
                );
            });
            assert!(
                drawn.is_none(),
                "{w}x{h} cannot hold a boxed chip, and saying so is what lets the \
                 caller choose a cheaper style"
            );
        }
    }

    // TP-SUR-SHELL-01: every floating panel shell wears rounded corners.
    // What floats above the surface is drawn as floating — the pattern the
    // reference file managers keep (yazi rounds exactly its overlays:
    // confirm, notify, pick — and leaves its panes plain), and the reader
    // asked for it. One shared shell means one voice for every popup.
    #[test]
    fn floating_shells_wear_rounded_corners() {
        use ratatui::{backend::TestBackend, Terminal};
        let mut term = Terminal::new(TestBackend::new(10, 5)).expect("terminal");
        term.draw(|frame| {
            render_panel_shell(frame, Rect::new(0, 0, 10, 5), Color::Blue, Color::Reset);
        })
        .expect("draw");
        let buffer = term.backend().buffer().clone();
        assert_eq!(buffer[(0u16, 0u16)].symbol(), "╭");
        assert_eq!(buffer[(9u16, 0u16)].symbol(), "╮");
        assert_eq!(buffer[(0u16, 4u16)].symbol(), "╰");
        assert_eq!(buffer[(9u16, 4u16)].symbol(), "╯");
    }

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
