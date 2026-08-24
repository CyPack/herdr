use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    widgets::Paragraph,
    Frame,
};

use super::surface_host::{AppInstanceId, StageTabHitArea};
use super::text::display_width_u16;
use super::widgets::panel_contrast_fg;
use crate::app::AppState;

const MIN_TAB_WIDTH: u16 = 8;
const NEW_TAB_WIDTH: u16 = 3;
const TAB_SCROLL_BUTTON_WIDTH: u16 = 3;
/// TP-TAB-SPLIT-01: each of the two split buttons beside `+` — right split,
/// then down split — is the same three cells wide the `+` is.
const SPLIT_BUTTON_WIDTH: u16 = 3;
const ZOOM_INDICATOR: &str = "ZOOM";
// The narrowest overflowing tab strip worth keeping interactive: one
// minimum-width tab, both scroll controls, and the new-tab control.
const MIN_TAB_STRIP_WIDTH: u16 =
    MIN_TAB_WIDTH + NEW_TAB_WIDTH + TAB_SCROLL_BUTTON_WIDTH.saturating_mul(2);

#[derive(Debug, Clone, Default)]
pub(crate) struct TabBarView {
    pub scroll: usize,
    pub tab_hit_areas: Vec<Rect>,
    /// Stage app entries sharing the strip with the terminal tabs. Kept in
    /// their own vector because `tab_hit_areas` is index-aligned with
    /// `ws.tabs`; appending here would make a stage click resolve as a
    /// terminal tab index.
    pub stage_tab_hit_areas: Vec<StageTabHitArea>,
    pub scroll_left_hit_area: Rect,
    pub scroll_right_hit_area: Rect,
    pub new_tab_hit_area: Rect,
    /// TP-TAB-SPLIT-01: split-right button, directly right of `+`.
    pub split_right_hit_area: Rect,
    /// TP-TAB-SPLIT-01: split-down button, right of the split-right button.
    pub split_down_hit_area: Rect,
}

fn stage_tab_width(instance: AppInstanceId) -> u16 {
    display_width_u16(instance.app().tab_label())
        .saturating_add(4)
        .max(MIN_TAB_WIDTH)
}

/// Horizontal cells the stage entries take out of the strip, gaps included.
fn stage_tabs_reserved_width(stage_tabs: &[AppInstanceId]) -> u16 {
    stage_tabs.iter().fold(0u16, |total, instance| {
        total.saturating_add(stage_tab_width(*instance).saturating_add(1))
    })
}

fn layout_stage_tab_hit_areas(
    stage_tabs: &[AppInstanceId],
    start_x: u16,
    right: u16,
    y: u16,
) -> Vec<StageTabHitArea> {
    let mut areas = Vec::with_capacity(stage_tabs.len());
    let mut x = start_x;
    for instance in stage_tabs {
        if x >= right {
            areas.push(StageTabHitArea {
                rect: Rect::default(),
                instance: *instance,
            });
            continue;
        }
        let width = stage_tab_width(*instance)
            .min(right.saturating_sub(x))
            .max(1);
        areas.push(StageTabHitArea {
            rect: Rect::new(x, y, width, 1),
            instance: *instance,
        });
        x = x.saturating_add(width.saturating_add(1));
    }
    areas
}

fn tab_width(ws: &crate::workspace::Workspace, tab_idx: usize) -> u16 {
    display_width_u16(&tab_chrome_label(ws, tab_idx))
        .saturating_add(4)
        .max(MIN_TAB_WIDTH)
}

/// The widest a tab's NAME may paint, in display cells. The unseen dot and
/// the zoom suffix ride outside the clamp — they are state channels, and a
/// long name must not be able to swallow them.
const TAB_NAME_MAX_WIDTH: usize = 20;

fn tab_chrome_label(ws: &crate::workspace::Workspace, tab_idx: usize) -> String {
    let name = ws
        .tab_display_name(tab_idx)
        .unwrap_or_else(|| (tab_idx + 1).to_string());
    // TP-TAB-NAME-01: clamp the name before any mark is attached, so the
    // width, the hit areas and the paint all follow one bounded label.
    let name = crate::ui::text::truncate_end(&name, TAB_NAME_MAX_WIDTH);
    // The glyph is the shape channel of the unseen mark — it survives any
    // palette. Going through the label keeps tab_width and the hit areas in
    // step automatically, the same route the zoomed " Z" suffix takes.
    // TP-TAB-UNSEEN-04
    let name = if ws.tabs.get(tab_idx).is_some_and(|tab| tab.unseen) {
        format!("● {name}")
    } else {
        name
    };
    if ws.tabs.get(tab_idx).is_some_and(|tab| tab.zoomed) {
        format!("{name} Z")
    } else {
        name
    }
}

#[derive(Clone, Copy)]
struct VisibleStatusSegment<'a> {
    text: &'a str,
    accent: bool,
}

fn visible_status_segments(app: &AppState) -> Vec<VisibleStatusSegment<'_>> {
    let zoomed = app
        .active
        .and_then(|index| app.workspaces.get(index))
        .is_some_and(|workspace| workspace.zoomed);
    app.tab_bar_right
        .iter()
        .filter_map(|segment| match segment {
            crate::app::state::TabBarStatusSegment::Zoom if zoomed => Some(VisibleStatusSegment {
                text: ZOOM_INDICATOR,
                accent: true,
            }),
            crate::app::state::TabBarStatusSegment::Text(Some(text))
                if display_width_u16(text) > 0 =>
            {
                Some(VisibleStatusSegment {
                    text,
                    accent: false,
                })
            }
            crate::app::state::TabBarStatusSegment::Zoom
            | crate::app::state::TabBarStatusSegment::Text(_) => None,
        })
        .collect()
}

fn tab_bar_status_width(app: &AppState) -> u16 {
    let segments = visible_status_segments(app);
    let content_width = segments.iter().fold(0_u16, |width, segment| {
        width.saturating_add(display_width_u16(segment.text))
    });
    let separators = u16::try_from(segments.len().saturating_sub(1)).unwrap_or(u16::MAX);
    content_width
        .saturating_add(display_width_u16(&app.tab_bar_right_separator).saturating_mul(separators))
}

fn tab_bar_status_area(app: &AppState, area: Rect) -> Option<Rect> {
    let width = tab_bar_status_width(app);
    if width == 0 {
        return None;
    }
    let reserved = width.saturating_add(1);
    (area.width.saturating_sub(reserved) >= MIN_TAB_STRIP_WIDTH)
        .then(|| Rect::new(area.x + area.width.saturating_sub(width), area.y, width, 1))
}

// Tabs win over status decoration on narrow rows. The extra reserved cell is
// the gap between the interactive strip and the right-aligned status entries.
pub(crate) fn tab_bar_content_area(app: &AppState, area: Rect) -> Rect {
    let reserved = tab_bar_status_area(app, area)
        .map(|status| status.width.saturating_add(1))
        .unwrap_or(0);
    Rect {
        width: area.width.saturating_sub(reserved),
        ..area
    }
}

fn layout_tab_hit_areas(ws: &crate::workspace::Workspace, area: Rect, scroll: usize) -> Vec<Rect> {
    let mut rects = vec![Rect::default(); ws.tabs.len()];
    if area.width == 0 || area.height == 0 {
        return rects;
    }

    let mut x = area.x;
    let right = area.x + area.width;
    for (idx, rect) in rects.iter_mut().enumerate().skip(scroll) {
        if x >= right {
            break;
        }
        let desired = tab_width(ws, idx);
        let remaining = right.saturating_sub(x);
        let width = desired.min(remaining).max(1);
        *rect = Rect::new(x, area.y, width, 1);
        x = x.saturating_add(width + 1);
    }
    rects
}

fn centered_tab_scroll(ws: &crate::workspace::Workspace, area: Rect) -> usize {
    let mut best_scroll = ws.active_tab_index();
    let mut best_distance = u16::MAX;
    let viewport_center = area.x.saturating_mul(2).saturating_add(area.width);

    for scroll in 0..=ws.active_tab_index() {
        let rects = layout_tab_hit_areas(ws, area, scroll);
        let Some(active_rect) = rects.get(ws.active_tab_index()).copied() else {
            continue;
        };
        if active_rect.width == 0 {
            continue;
        }

        let active_center = active_rect
            .x
            .saturating_mul(2)
            .saturating_add(active_rect.width);
        let distance = active_center.abs_diff(viewport_center);
        if distance <= best_distance {
            best_distance = distance;
            best_scroll = scroll;
        }
    }

    best_scroll
}

fn trailing_tab_controls_x(tab_hit_areas: &[Rect], fallback_x: u16) -> u16 {
    tab_hit_areas
        .iter()
        .rev()
        .find(|rect| rect.width > 0)
        .map(|rect| rect.x + rect.width)
        .unwrap_or(fallback_x)
}

/// The two split buttons, pinned flush to the strip's right edge — the down
/// split ends at the edge, the right split sits just before it. `seats_width`
/// is the free width at the edge; whole seats only (TP-MOD-35's rule: a
/// sliver would paint as blank cells that still answer a press), and a lone
/// seat goes to the right split, the first of the pair.
fn pinned_split_button_hit_areas(seats_width: u16, area_right: u16, y: u16) -> (Rect, Rect) {
    let slot = |end: u16| {
        Rect::new(
            end.saturating_sub(SPLIT_BUTTON_WIDTH),
            y,
            SPLIT_BUTTON_WIDTH,
            1,
        )
    };
    if seats_width >= SPLIT_BUTTON_WIDTH.saturating_mul(2) {
        (
            slot(area_right.saturating_sub(SPLIT_BUTTON_WIDTH)),
            slot(area_right),
        )
    } else if seats_width >= SPLIT_BUTTON_WIDTH {
        (slot(area_right), Rect::default())
    } else {
        (Rect::default(), Rect::default())
    }
}

fn max_tab_scroll(ws: &crate::workspace::Workspace, area: Rect) -> usize {
    (0..ws.tabs.len())
        .find(|&scroll| {
            layout_tab_hit_areas(ws, area, scroll)
                .last()
                .is_some_and(|rect| rect.width > 0)
        })
        .unwrap_or(0)
}

pub(crate) fn compute_tab_bar_view(
    ws: &crate::workspace::Workspace,
    stage_tabs: &[AppInstanceId],
    area: Rect,
    current_scroll: usize,
    follow_active: bool,
    mouse_chrome: bool,
) -> TabBarView {
    if area.width == 0 || area.height == 0 {
        return TabBarView::default();
    }

    let area_right = area.x + area.width;
    // TP-FTAB-ENTRY-05: stage entries are pinned to the leading edge and never
    // scroll, so they are laid out first and the terminal tabs get what remains.
    // Everything downstream — scrolling, overflow, drag-reorder — then keeps
    // working on a narrower area without knowing they exist.
    let stage_tab_hit_areas = layout_stage_tab_hit_areas(stage_tabs, area.x, area_right, area.y);
    let tabs_x = area
        .x
        .saturating_add(stage_tabs_reserved_width(stage_tabs))
        .min(area_right);
    let tabs_width = area_right.saturating_sub(tabs_x);

    if !mouse_chrome {
        let tabs_area = Rect::new(tabs_x, area.y, tabs_width, area.height);
        let max_scroll = max_tab_scroll(ws, tabs_area);
        let scroll = if follow_active {
            centered_tab_scroll(ws, tabs_area).min(max_scroll)
        } else {
            current_scroll.min(max_scroll)
        };
        return TabBarView {
            scroll,
            stage_tab_hit_areas,
            tab_hit_areas: layout_tab_hit_areas(ws, tabs_area, scroll),
            scroll_left_hit_area: Rect::default(),
            scroll_right_hit_area: Rect::default(),
            new_tab_hit_area: Rect::default(),
            split_right_hit_area: Rect::default(),
            split_down_hit_area: Rect::default(),
        };
    }

    let all_tabs_area = Rect::new(
        tabs_x,
        area.y,
        tabs_width.saturating_sub(NEW_TAB_WIDTH),
        area.height,
    );
    let all_tabs = layout_tab_hit_areas(ws, all_tabs_area, 0);
    let overflow = all_tabs.iter().any(|rect| rect.width == 0);
    if !overflow {
        // TP-TAB-SPLIT-01: the split buttons are pinned to the strip's far
        // right the way the stage tabs are pinned to its far left
        // (TP-FTAB-ENTRY-05). A seat is carved out only when the carving
        // leaves the tab layout untouched — the layout is recomputed inside
        // the narrower area and must come back identical, because
        // `layout_tab_hit_areas` squeezes a tab before it drops one, and a
        // squeezed tab is the reservation making the strip worse. Tier by
        // tier: both buttons, the right one alone, none.
        let mut reserved_split_width = 0;
        for candidate in [SPLIT_BUTTON_WIDTH.saturating_mul(2), SPLIT_BUTTON_WIDTH] {
            let candidate_area = Rect::new(
                tabs_x,
                area.y,
                tabs_width
                    .saturating_sub(NEW_TAB_WIDTH)
                    .saturating_sub(candidate),
                area.height,
            );
            if layout_tab_hit_areas(ws, candidate_area, 0) == all_tabs {
                reserved_split_width = candidate;
                break;
            }
        }
        let new_tab_x = trailing_tab_controls_x(&all_tabs, tabs_x);
        let new_tab_hit_area = Rect::new(
            new_tab_x,
            area.y,
            area_right
                .saturating_sub(reserved_split_width)
                .saturating_sub(new_tab_x)
                .min(NEW_TAB_WIDTH),
            1,
        );
        let (split_right_hit_area, split_down_hit_area) =
            pinned_split_button_hit_areas(reserved_split_width, area_right, area.y);
        return TabBarView {
            scroll: 0,
            tab_hit_areas: all_tabs,
            stage_tab_hit_areas,
            scroll_left_hit_area: Rect::default(),
            scroll_right_hit_area: Rect::default(),
            new_tab_hit_area,
            split_right_hit_area,
            split_down_hit_area,
        };
    }

    let left_hit_area = Rect::new(tabs_x, area.y, TAB_SCROLL_BUTTON_WIDTH.min(tabs_width), 1);
    let tab_area_x = left_hit_area.x + left_hit_area.width;
    // TP-TAB-SPLIT-01: in overflow the trailing chrome is `\u{25b8} + \u{2590} \u{2584}` — the
    // split buttons stay reachable however many tabs there are; the tabs
    // scroll, the chrome does not.
    let reserved_trailing_width = NEW_TAB_WIDTH
        .saturating_add(TAB_SCROLL_BUTTON_WIDTH)
        .saturating_add(SPLIT_BUTTON_WIDTH.saturating_mul(2));
    let tab_area_right = area_right.saturating_sub(reserved_trailing_width);
    let tab_area = Rect::new(
        tab_area_x,
        area.y,
        tab_area_right.saturating_sub(tab_area_x),
        area.height,
    );

    let max_scroll = max_tab_scroll(ws, tab_area);
    let scroll = if follow_active {
        centered_tab_scroll(ws, tab_area).min(max_scroll)
    } else {
        current_scroll.min(max_scroll)
    };
    let tab_hit_areas = layout_tab_hit_areas(ws, tab_area, scroll);
    let trailing_x = trailing_tab_controls_x(&tab_hit_areas, tab_area_x).min(tab_area_right);
    let right_hit_area = Rect::new(
        trailing_x,
        area.y,
        area_right
            .saturating_sub(trailing_x)
            .min(TAB_SCROLL_BUTTON_WIDTH),
        1,
    );
    let new_tab_x = right_hit_area.x + right_hit_area.width;
    let new_tab_hit_area = Rect::new(
        new_tab_x,
        area.y,
        area_right.saturating_sub(new_tab_x).min(NEW_TAB_WIDTH),
        1,
    );
    // TP-TAB-SPLIT-01: pinned to the edge here too — the reserved trailing
    // chrome guarantees the seats on all but the narrowest strips.
    let split_seats = area_right.saturating_sub(new_tab_hit_area.x + new_tab_hit_area.width);
    let (split_right_hit_area, split_down_hit_area) =
        pinned_split_button_hit_areas(split_seats, area_right, area.y);

    TabBarView {
        scroll,
        tab_hit_areas,
        stage_tab_hit_areas,
        scroll_left_hit_area: left_hit_area,
        scroll_right_hit_area: right_hit_area,
        new_tab_hit_area,
        split_right_hit_area,
        split_down_hit_area,
    }
}

fn tab_drop_indicator_x(
    app: &AppState,
    ws: &crate::workspace::Workspace,
    insert_idx: usize,
) -> Option<u16> {
    let mut visible_tabs = app
        .view
        .tab_hit_areas
        .iter()
        .enumerate()
        .filter(|(_, rect)| rect.width > 0);
    let first_visible = visible_tabs.clone().next()?;
    let last_visible = visible_tabs.next_back().unwrap_or(first_visible);

    if insert_idx == 0 {
        return Some(if first_visible.0 == 0 {
            first_visible.1.x
        } else {
            app.view.tab_scroll_left_hit_area.x + app.view.tab_scroll_left_hit_area.width
        });
    }

    if let Some((_, rect)) = app
        .view
        .tab_hit_areas
        .iter()
        .enumerate()
        .find(|(idx, rect)| *idx == insert_idx && rect.width > 0)
    {
        return Some(rect.x.saturating_sub(1));
    }

    if insert_idx >= ws.tabs.len() {
        return Some(if last_visible.0 + 1 >= ws.tabs.len() {
            last_visible.1.x + last_visible.1.width
        } else {
            app.view.tab_scroll_right_hit_area.x.saturating_sub(1)
        });
    }

    None
}

pub(super) fn render_tab_bar(app: &AppState, frame: &mut Frame, area: Rect) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let Some(active_ws_idx) = app.active else {
        return;
    };
    let Some(ws) = app.workspaces.get(active_ws_idx) else {
        return;
    };
    let p = &app.palette;

    frame.render_widget(
        Paragraph::new(" ".repeat(area.width as usize)).style(Style::default().bg(p.panel_bg)),
        area,
    );

    let first_visible_idx = app
        .view
        .tab_hit_areas
        .iter()
        .enumerate()
        .find(|(_, rect)| rect.width > 0)
        .map(|(idx, _)| idx);
    let last_visible_idx = app
        .view
        .tab_hit_areas
        .iter()
        .enumerate()
        .rev()
        .find(|(_, rect)| rect.width > 0)
        .map(|(idx, _)| idx);
    let can_scroll_left = app.view.tab_scroll_left_hit_area.width > 0 && app.tab_scroll > 0;
    let can_scroll_right = app.view.tab_scroll_right_hit_area.width > 0
        && last_visible_idx.is_some_and(|idx| idx + 1 < ws.tabs.len());

    if app.mouse_capture && app.view.tab_scroll_left_hit_area.width > 0 {
        let style = if can_scroll_left {
            Style::default().fg(p.overlay1).bg(p.surface0)
        } else {
            Style::default()
                .fg(p.overlay0)
                .bg(p.surface0)
                .add_modifier(Modifier::DIM)
        };
        frame.render_widget(
            Paragraph::new(" < ").style(style),
            app.view.tab_scroll_left_hit_area,
        );
    }

    if app.mouse_capture && app.view.tab_scroll_right_hit_area.width > 0 {
        let style = if can_scroll_right {
            Style::default().fg(p.overlay1).bg(p.surface0)
        } else {
            Style::default()
                .fg(p.overlay0)
                .bg(p.surface0)
                .add_modifier(Modifier::DIM)
        };
        frame.render_widget(
            Paragraph::new(" > ").style(style),
            app.view.tab_scroll_right_hit_area,
        );
    }

    // TP-FTAB-ENTRY-03: the strip has exactly one active entry. A terminal tab
    // painting as active while a stage app owns the content would tell the user
    // their keystrokes go somewhere they do not.
    let terminal_surface_active =
        app.stage.surface_view() == super::surface_host::StageSurfaceView::TerminalWorkspace;

    let now = std::time::Instant::now();
    for (idx, tab) in ws.tabs.iter().enumerate() {
        let Some(rect) = app.view.tab_hit_areas.get(idx).copied() else {
            break;
        };
        if rect.width == 0 {
            continue;
        }
        let active = terminal_surface_active && idx == ws.active_tab_index();
        let style = if active {
            let base = Style::default().fg(panel_contrast_fg(p)).bg(p.accent);
            if tab.is_auto_named() {
                base
            } else {
                base.add_modifier(Modifier::BOLD)
            }
        } else if tab.unseen {
            // The color channel of the unseen mark: accent FOREGROUND on the
            // inactive background. The active tab owns the accent BACKGROUND,
            // so the two states stay distinguishable side by side. Checked
            // before auto-naming so DIM cannot mute a tab that exists to be
            // noticed. TP-TAB-UNSEEN-04
            Style::default()
                .fg(p.accent)
                .bg(p.surface0)
                .add_modifier(Modifier::BOLD)
        } else if tab.is_auto_named() {
            Style::default()
                .fg(p.overlay0)
                .bg(p.surface0)
                .add_modifier(Modifier::DIM)
        } else {
            Style::default().fg(p.overlay1).bg(p.surface0)
        };
        // TP-TAB-FLASH-02: the spawn flash REVERSES whatever style the tab
        // already earned — active, unseen or plain — so it reads on any theme.
        let style = if tab.flash_phase(now) == Some(true) {
            style.add_modifier(Modifier::REVERSED)
        } else {
            style
        };
        let width = rect.width as usize;
        let name = tab_chrome_label(ws, idx);
        // Pad by terminal columns, not chars, so wide glyphs stay centered.
        let padding = width.saturating_sub(display_width_u16(&name) as usize);
        let left = padding / 2;
        let text = format!(
            "{empty:left$}{name}{empty:right$}",
            empty = "",
            right = padding - left
        );
        frame.render_widget(Paragraph::new(text).style(style), rect);
    }

    for entry in &app.view.stage_tab_hit_areas {
        if entry.rect.width == 0 {
            continue;
        }
        let style = if app.stage.is_active_instance(entry.instance) {
            Style::default()
                .fg(panel_contrast_fg(p))
                .bg(p.accent)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(p.overlay1).bg(p.surface0)
        };
        let width = entry.rect.width as usize;
        let text = format!(
            " {:width$}",
            entry.instance.app().tab_label(),
            width = width.saturating_sub(1)
        );
        frame.render_widget(Paragraph::new(text).style(style), entry.rect);
    }

    if let Some(crate::app::state::DragState {
        target:
            crate::app::state::DragTarget::TabReorder {
                ws_idx,
                insert_idx: Some(insert_idx),
                ..
            },
    }) = &app.drag
    {
        if *ws_idx == active_ws_idx {
            if let Some(x) = tab_drop_indicator_x(app, ws, *insert_idx) {
                frame.buffer_mut()[(x.min(area.x + area.width.saturating_sub(1)), area.y)]
                    .set_symbol("│")
                    .set_style(Style::default().fg(p.accent));
            }
        }
    }

    if app.mouse_capture && app.view.new_tab_hit_area.width > 0 {
        frame.render_widget(
            Paragraph::new(" + ").style(Style::default().fg(p.overlay1)),
            app.view.new_tab_hit_area,
        );
    }
    // TP-TAB-SPLIT-01: right-half block reads "new pane on the right", lower
    // block reads "new pane below". Drawn only when their cells exist.
    if app.mouse_capture && app.view.split_right_hit_area.width > 0 {
        frame.render_widget(
            Paragraph::new(" \u{2590} ").style(Style::default().fg(p.overlay1)),
            app.view.split_right_hit_area,
        );
    }
    if app.mouse_capture && app.view.split_down_hit_area.width > 0 {
        frame.render_widget(
            Paragraph::new(" \u{2584} ").style(Style::default().fg(p.overlay1)),
            app.view.split_down_hit_area,
        );
    }

    if first_visible_idx.is_some_and(|idx| idx > 0) {
        let x = if app.mouse_capture && app.view.tab_scroll_left_hit_area.width > 0 {
            app.view.tab_scroll_left_hit_area.x + app.view.tab_scroll_left_hit_area.width
        } else {
            area.x
        };
        if x < area.x + area.width {
            frame.buffer_mut()[(x, area.y)]
                .set_symbol("…")
                .set_style(Style::default().fg(p.overlay0));
        }
    }
    if last_visible_idx.is_some_and(|idx| idx + 1 < ws.tabs.len()) {
        let content = tab_bar_content_area(app, area);
        let content_right = content.x + content.width;
        let x = if app.mouse_capture && app.view.tab_scroll_right_hit_area.width > 0 {
            app.view.tab_scroll_right_hit_area.x.saturating_sub(1)
        } else {
            content_right.saturating_sub(1)
        };
        if x >= area.x && x < area.x + area.width {
            frame.buffer_mut()[(x, area.y)]
                .set_symbol("…")
                .set_style(Style::default().fg(p.overlay0));
        }
    }

    if let Some(status_area) = tab_bar_status_area(app, area) {
        let segments = visible_status_segments(app);
        let separator_width = display_width_u16(&app.tab_bar_right_separator);
        let mut x = status_area.x;
        for (index, segment) in segments.iter().enumerate() {
            if index > 0 && separator_width > 0 {
                let rect = Rect::new(x, area.y, separator_width, 1);
                frame.render_widget(
                    Paragraph::new(app.tab_bar_right_separator.as_str())
                        .style(Style::default().fg(p.overlay0).bg(p.panel_bg)),
                    rect,
                );
                x = x.saturating_add(separator_width);
            }

            let width = display_width_u16(segment.text);
            let rect = Rect::new(x, area.y, width, 1);
            let style = if segment.accent {
                Style::default()
                    .fg(panel_contrast_fg(p))
                    .bg(p.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(p.overlay1).bg(p.panel_bg)
            };
            frame.render_widget(Paragraph::new(segment.text).style(style), rect);
            x = x.saturating_add(width);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::AppState;
    use crate::workspace::Workspace;
    use ratatui::{backend::TestBackend, Terminal};

    fn buffer_row_text(buffer: &ratatui::buffer::Buffer, area: Rect, row: u16) -> String {
        (area.x..area.x + area.width)
            .map(|x| buffer[(x, row)].symbol())
            .collect::<String>()
            .trim_end()
            .to_string()
    }

    struct FixtureRoot(std::path::PathBuf);

    impl Drop for FixtureRoot {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// A workspace plus a populated directory, ready to open Files over.
    fn stage_fixture(name: &str) -> (AppState, FixtureRoot) {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "herdr-files-tab-{}-{}-{}",
            name,
            std::process::id(),
            COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&root).expect("create files tab fixture root");
        std::fs::write(root.join("00.txt"), b"x").expect("fixture entry");

        let mut app = AppState::test_new();
        app.workspaces = vec![Workspace::test_new(name)];
        app.active = Some(0);
        app.selected = 0;
        app.mobile_width_threshold = 0;
        (app, FixtureRoot(root))
    }

    fn open_files(app: &mut AppState, root: &FixtureRoot) {
        app.try_open_file_manager_with(|_| Some(crate::fm::FmState::new(&root.0)))
            .expect("Files activation");
    }

    // TP-FTAB-CHROME-01: the tab strip is shell chrome, not terminal-app
    // chrome. Files opens as a peer in the same content area, so hiding the
    // strip would make it read as leaving the workspace rather than switching
    // to another tab.
    #[test]
    fn tab_strip_stays_present_while_the_files_surface_owns_the_stage() {
        let (mut app, root) = stage_fixture("chrome");
        let area = Rect::new(0, 0, 80, 24);

        crate::ui::compute_view(&mut app, area);
        let terminal_surface_strip = app.view.tab_bar_rect;
        assert_eq!(
            terminal_surface_strip.height, 1,
            "control: the terminal surface carves out a one-row tab strip"
        );

        open_files(&mut app, &root);
        crate::ui::compute_view(&mut app, area);

        assert_eq!(
            app.view.tab_bar_rect, terminal_surface_strip,
            "the Files surface must keep the identical tab strip"
        );
    }

    // TP-FTAB-CHROME-02: both surfaces receive the same content rect. An
    // off-by-one here hides the last Files row, which is invisible in a
    // screenshot and only shows up as "the bottom entry is unreachable".
    #[test]
    fn both_surfaces_receive_the_same_content_area() {
        let (mut app, root) = stage_fixture("content");
        let area = Rect::new(0, 0, 80, 24);

        crate::ui::compute_view(&mut app, area);
        let terminal_content = app.view.terminal_area;
        assert!(
            terminal_content.height > 0,
            "control: terminal content area"
        );

        open_files(&mut app, &root);
        crate::ui::compute_view(&mut app, area);

        assert_eq!(
            app.view.terminal_area, terminal_content,
            "Files must own exactly the same content rect the terminal surface owns"
        );
    }

    // TP-FTAB-ENTRY-01: Files occupies its own entry in the strip, disjoint
    // from every terminal tab rect, and `tab_hit_areas` stays index-aligned
    // with `ws.tabs`. Appending the entry to that vector instead would make a
    // stage click resolve as a terminal tab index.
    #[test]
    fn files_appears_as_a_peer_entry_disjoint_from_terminal_tabs() {
        let (mut app, root) = stage_fixture("entry");
        app.workspaces[0].test_add_tab(None);
        let area = Rect::new(0, 0, 80, 24);

        crate::ui::compute_view(&mut app, area);
        assert!(
            app.view.stage_tab_hit_areas.is_empty(),
            "control: no stage app is open, so the strip carries terminal tabs only"
        );

        open_files(&mut app, &root);
        crate::ui::compute_view(&mut app, area);

        assert_eq!(
            app.view.tab_hit_areas.len(),
            app.workspaces[0].tabs.len(),
            "terminal hit areas must stay index-aligned with ws.tabs"
        );
        assert_eq!(
            app.view.stage_tab_hit_areas.len(),
            1,
            "one open Files instance is one strip entry"
        );
        let files_rect = app.view.stage_tab_hit_areas[0].rect;
        assert!(files_rect.width > 0 && files_rect.height == 1);
        for (idx, tab_rect) in app.view.tab_hit_areas.iter().enumerate() {
            assert!(
                tab_rect.width == 0
                    || files_rect.x >= tab_rect.x + tab_rect.width
                    || tab_rect.x >= files_rect.x + files_rect.width,
                "the Files entry overlaps terminal tab {idx}: {tab_rect:?} vs {files_rect:?}"
            );
        }
    }

    // TP-FTAB-ENTRY-05: stage entries are pinned to the left edge, ahead of
    // every terminal tab, and stay there when the terminal tabs overflow and
    // scroll. An entry that scrolls out of reach is not pinned.
    #[test]
    fn files_entry_is_pinned_left_of_every_terminal_tab() {
        for mouse_chrome in [false, true] {
            let (mut app, root) = stage_fixture("pinned-left");
            app.mouse_capture = mouse_chrome;
            for _ in 0..12 {
                app.workspaces[0].test_add_tab(None);
            }
            let last = app.workspaces[0].tabs.len() - 1;
            app.workspaces[0].set_active_tab(last);
            let area = Rect::new(0, 0, 80, 24);
            open_files(&mut app, &root);
            crate::ui::compute_view(&mut app, area);

            let files = app.view.stage_tab_hit_areas[0].rect;
            assert!(
                files.width > 0,
                "mouse_chrome={mouse_chrome}: the pinned entry is always visible"
            );
            assert_eq!(
                files.x, app.view.tab_bar_rect.x,
                "mouse_chrome={mouse_chrome}: the pinned entry starts at the strip's left edge"
            );
            assert!(
                app.view
                    .tab_hit_areas
                    .iter()
                    .any(|rect: &Rect| rect.width == 0),
                "control: twelve tabs in an 80-cell strip must overflow"
            );
            for (idx, tab) in app.view.tab_hit_areas.iter().enumerate() {
                if tab.width == 0 {
                    continue;
                }
                assert!(
                    files.x + files.width <= tab.x,
                    "mouse_chrome={mouse_chrome}: terminal tab {idx} must start right of the pinned entry: {tab:?} vs {files:?}"
                );
            }
        }
    }

    // TP-FTAB-ENTRY-02: the entry carries the instance's stable lifecycle
    // identity, not a position. A rect retained across close and reopen must
    // not authorize the new instance, which is why the generation must change.
    #[test]
    fn reopened_files_entry_carries_a_new_instance_generation() {
        let (mut app, root) = stage_fixture("generation");
        let area = Rect::new(0, 0, 80, 24);

        open_files(&mut app, &root);
        crate::ui::compute_view(&mut app, area);
        let first = app.view.stage_tab_hit_areas[0].instance;

        app.close_file_manager();
        crate::ui::compute_view(&mut app, area);
        assert!(
            app.view.stage_tab_hit_areas.is_empty(),
            "closing Files retires its strip entry in the same frame"
        );

        open_files(&mut app, &root);
        crate::ui::compute_view(&mut app, area);
        let second = app.view.stage_tab_hit_areas[0].instance;

        assert_ne!(
            first, second,
            "a reopened instance must not reuse the closed instance's identity"
        );
    }

    // TP-FTAB-ENTRY-03: the strip has exactly one active entry. While Files
    // owns the stage, a terminal tab that still paints as active would tell
    // the user their keystrokes go somewhere they do not.
    #[test]
    fn only_the_owning_surface_paints_an_active_strip_entry() {
        let (mut app, root) = stage_fixture("active");
        let area = Rect::new(0, 0, 80, 24);
        open_files(&mut app, &root);
        crate::ui::compute_view(&mut app, area);

        let backend = TestBackend::new(area.width, area.height);
        let mut terminal = Terminal::new(backend).expect("strip terminal");
        terminal
            .draw(|frame| render_tab_bar(&app, frame, app.view.tab_bar_rect))
            .expect("strip render");
        let buffer = terminal.backend().buffer();

        let terminal_tab = app.view.tab_hit_areas[0];
        assert_ne!(
            buffer[(terminal_tab.x + 1, terminal_tab.y)].style().bg,
            Some(app.palette.accent),
            "no terminal tab may paint as active while Files owns the stage"
        );
        let files_entry = app.view.stage_tab_hit_areas[0].rect;
        assert_eq!(
            buffer[(files_entry.x + 1, files_entry.y)].style().bg,
            Some(app.palette.accent),
            "the Files entry paints as the one active strip entry"
        );
    }

    // TP-FTAB-ENTRY-04: `hide_tab_bar_when_single_tab` hides chrome that shows
    // one entry. With Files open there are two, so hiding the strip would make
    // the Files tab unreachable by mouse.
    #[test]
    fn single_terminal_tab_plus_files_keeps_the_strip_visible() {
        let (mut app, root) = stage_fixture("hide-rule");
        app.hide_tab_bar_when_single_tab = true;
        let area = Rect::new(0, 0, 80, 24);

        crate::ui::compute_view(&mut app, area);
        assert_eq!(
            app.view.tab_bar_rect,
            Rect::default(),
            "control: one terminal tab and no stage app hides the strip"
        );

        open_files(&mut app, &root);
        crate::ui::compute_view(&mut app, area);

        assert_eq!(
            app.view.tab_bar_rect.height, 1,
            "a second strip entry must bring the strip back"
        );
        assert_eq!(app.view.stage_tab_hit_areas.len(), 1);
    }

    #[test]
    fn tab_bar_marks_zoomed_tabs_without_renaming_them() {
        let mut app = AppState::test_new();
        let mut ws = Workspace::test_new("test");
        ws.tabs[0].zoomed = true;
        let custom_tab = ws.test_add_tab(Some("test"));
        ws.tabs[custom_tab].zoomed = true;

        app.workspaces = vec![ws];
        app.active = Some(0);
        app.view.tab_bar_rect = Rect::new(0, 0, 30, 1);
        let view = compute_tab_bar_view(
            &app.workspaces[0],
            &[],
            app.view.tab_bar_rect,
            0,
            true,
            false,
        );
        app.view.tab_hit_areas = view.tab_hit_areas;

        let backend = TestBackend::new(30, 1);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| render_tab_bar(&app, frame, app.view.tab_bar_rect))
            .unwrap();

        let row = buffer_row_text(terminal.backend().buffer(), app.view.tab_bar_rect, 0);
        assert!(row.contains(" 1 Z"), "tab row: {row:?}");
        assert!(row.contains(" test Z"), "tab row: {row:?}");
        assert_eq!(app.workspaces[0].tab_display_name(0).as_deref(), Some("1"));
        assert_eq!(
            app.workspaces[0].tab_display_name(custom_tab).as_deref(),
            Some("test")
        );
    }

    #[test]
    fn tab_bar_renders_ordered_status_entries_with_separator() {
        let mut app = AppState::test_new();
        let mut ws = Workspace::test_new("test");
        ws.tabs[0].zoomed = true;
        app.tab_bar_right = vec![
            crate::app::state::TabBarStatusSegment::Zoom,
            crate::app::state::TabBarStatusSegment::Text(Some("wintermute".into())),
            crate::app::state::TabBarStatusSegment::Text(Some("14:30".into())),
        ];
        app.tab_bar_right_separator = " · ".into();

        app.workspaces = vec![ws];
        app.active = Some(0);
        app.view.tab_bar_rect = Rect::new(0, 0, 60, 1);
        let content = tab_bar_content_area(&app, app.view.tab_bar_rect);
        let view = compute_tab_bar_view(&app.workspaces[0], &[], content, 0, true, false);
        app.view.tab_hit_areas = view.tab_hit_areas.clone();

        let backend = TestBackend::new(60, 1);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| render_tab_bar(&app, frame, app.view.tab_bar_rect))
            .unwrap();

        let buffer = terminal.backend().buffer();
        let row = buffer_row_text(buffer, app.view.tab_bar_rect, 0);
        assert!(
            row.ends_with("ZOOM · wintermute · 14:30"),
            "tab row: {row:?}"
        );
        let status_x = 60 - display_width_u16("ZOOM · wintermute · 14:30");
        assert_eq!(buffer[(status_x, 0)].style().bg, Some(app.palette.accent));
        for rect in &view.tab_hit_areas {
            assert!(rect.x + rect.width <= content.x + content.width);
        }
    }

    #[test]
    fn hidden_status_entries_do_not_leave_dangling_separators() {
        let mut app = AppState::test_new();
        app.tab_bar_right = vec![
            crate::app::state::TabBarStatusSegment::Zoom,
            crate::app::state::TabBarStatusSegment::Text(None),
            crate::app::state::TabBarStatusSegment::Text(Some("wintermute".into())),
        ];
        app.tab_bar_right_separator = " | ".into();
        app.workspaces = vec![Workspace::test_new("test")];
        app.active = Some(0);
        app.view.tab_bar_rect = Rect::new(0, 0, 40, 1);
        let content = tab_bar_content_area(&app, app.view.tab_bar_rect);
        let view = compute_tab_bar_view(&app.workspaces[0], &[], content, 0, true, false);
        app.view.tab_hit_areas = view.tab_hit_areas;

        let backend = TestBackend::new(40, 1);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| render_tab_bar(&app, frame, app.view.tab_bar_rect))
            .unwrap();

        let row = buffer_row_text(terminal.backend().buffer(), app.view.tab_bar_rect, 0);
        assert!(row.ends_with("wintermute"), "tab row: {row:?}");
        assert!(!row.contains(" | "), "tab row: {row:?}");
    }

    #[test]
    fn status_reservation_keeps_a_minimum_width_tab_between_scroll_controls() {
        let mut app = AppState::test_new();
        app.tab_bar_right = vec![crate::app::state::TabBarStatusSegment::Text(Some(
            "x".into(),
        ))];
        let mut workspace = Workspace::test_new("test");
        workspace.test_add_tab(None);
        workspace.test_add_tab(None);
        app.workspaces = vec![workspace];
        app.active = Some(0);

        let too_narrow = Rect::new(0, 0, MIN_TAB_STRIP_WIDTH + 1, 1);
        assert_eq!(tab_bar_content_area(&app, too_narrow), too_narrow);

        let wide_enough = Rect::new(0, 0, MIN_TAB_STRIP_WIDTH + 2, 1);
        let content = tab_bar_content_area(&app, wide_enough);
        assert_eq!(content.width, MIN_TAB_STRIP_WIDTH);
        let view = compute_tab_bar_view(&app.workspaces[0], &[], content, 0, true, true);
        assert!(view.tab_hit_areas[0].width >= MIN_TAB_WIDTH);
    }

    #[test]
    fn combined_status_entries_yield_to_tab_controls_on_narrow_rows() {
        let mut app = AppState::test_new();
        app.tab_bar_right = vec![
            crate::app::state::TabBarStatusSegment::Text(Some(
                "a-hostname-wider-than-the-whole-bar".into(),
            )),
            crate::app::state::TabBarStatusSegment::Text(Some("14:30".into())),
        ];
        app.workspaces = vec![Workspace::test_new("test")];
        app.active = Some(0);
        app.view.tab_bar_rect = Rect::new(0, 0, 30, 1);

        assert_eq!(
            tab_bar_content_area(&app, app.view.tab_bar_rect),
            app.view.tab_bar_rect
        );
        assert_eq!(tab_bar_status_area(&app, app.view.tab_bar_rect), None);

        let view = compute_tab_bar_view(
            &app.workspaces[0],
            &[],
            tab_bar_content_area(&app, app.view.tab_bar_rect),
            0,
            true,
            true,
        );
        assert!(view.tab_hit_areas[0].width > 0);
        assert!(view.new_tab_hit_area.width > 0);
    }

    #[test]
    fn cjk_tab_labels_are_centered_by_display_width() {
        let mut app = AppState::test_new();
        let mut ws = Workspace::test_new("test");
        ws.tabs[0].set_custom_name("提交 herdr 的反馈".into());

        app.workspaces = vec![ws];
        app.active = Some(0);
        app.view.tab_bar_rect = Rect::new(0, 0, 30, 1);
        let view = compute_tab_bar_view(
            &app.workspaces[0],
            &[],
            app.view.tab_bar_rect,
            0,
            true,
            false,
        );
        app.view.tab_hit_areas = view.tab_hit_areas;

        let backend = TestBackend::new(30, 1);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| render_tab_bar(&app, frame, app.view.tab_bar_rect))
            .unwrap();

        // 17 display columns + 4 padding: two columns each side, wide glyphs
        // starting right after the left padding.
        let rect = app.view.tab_hit_areas[0];
        assert_eq!(rect.width, 21);
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer[(rect.x, rect.y)].symbol(), " ");
        assert_eq!(buffer[(rect.x + 1, rect.y)].symbol(), " ");
        assert_eq!(buffer[(rect.x + 2, rect.y)].symbol(), "提");
        assert_eq!(buffer[(rect.x + rect.width - 2, rect.y)].symbol(), " ");
        assert_eq!(buffer[(rect.x + rect.width - 1, rect.y)].symbol(), " ");
    }

    #[test]
    fn tab_labels_are_centered_in_their_cells() {
        let mut app = AppState::test_new();
        let mut ws = Workspace::test_new("test");
        ws.tabs[0].set_custom_name("omarchy".into());

        app.workspaces = vec![ws];
        app.active = Some(0);
        app.view.tab_bar_rect = Rect::new(0, 0, 30, 1);
        let view = compute_tab_bar_view(
            &app.workspaces[0],
            &[],
            app.view.tab_bar_rect,
            0,
            true,
            false,
        );
        app.view.tab_hit_areas = view.tab_hit_areas;

        let backend = TestBackend::new(30, 1);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| render_tab_bar(&app, frame, app.view.tab_bar_rect))
            .unwrap();

        let rect = app.view.tab_hit_areas[0];
        let buffer = terminal.backend().buffer();
        let cell: String = (rect.x..rect.x + rect.width)
            .map(|x| buffer[(x, rect.y)].symbol())
            .collect();
        assert_eq!(cell, "  omarchy  ");
    }

    #[test]
    fn active_auto_named_tab_keeps_readable_weight() {
        let mut app = AppState::test_new();
        let ws = Workspace::test_new("test");

        app.workspaces = vec![ws];
        app.active = Some(0);
        app.view.tab_bar_rect = Rect::new(0, 0, 30, 1);
        let view = compute_tab_bar_view(
            &app.workspaces[0],
            &[],
            app.view.tab_bar_rect,
            0,
            true,
            false,
        );
        app.view.tab_hit_areas = view.tab_hit_areas;

        let backend = TestBackend::new(30, 1);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| render_tab_bar(&app, frame, app.view.tab_bar_rect))
            .unwrap();

        let tab_rect = app.view.tab_hit_areas[0];
        let style = terminal.backend().buffer()[(tab_rect.x + 1, tab_rect.y)].style();

        assert_eq!(style.bg, Some(app.palette.accent));
        assert!(!style.add_modifier.contains(Modifier::DIM));
        assert!(!style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn zoom_marker_counts_toward_tab_width() {
        let mut ws = Workspace::test_new("test");
        ws.tabs[0].set_custom_name("abcdefgh".into());
        ws.tabs[0].zoomed = true;

        assert_eq!(tab_width(&ws, 0), 14);
    }

    #[test]
    fn tab_width_uses_display_width_for_cjk_labels() {
        let mut ws = Workspace::test_new("test");
        ws.tabs[0].set_custom_name("提交 herdr 的反馈".into());

        assert_eq!(
            tab_width(&ws, 0),
            display_width_u16("提交 herdr 的反馈") + 4
        );
    }

    // TP-TAB-SPLIT-01: the split pair is pinned flush to the strip's far
    // right — right split, then down split ending at the edge — while the
    // `+` keeps trailing the tabs.
    #[test]
    fn the_split_buttons_stand_pinned_at_the_strip_far_right() {
        let ws = Workspace::test_new("test");
        let view = compute_tab_bar_view(&ws, &[], Rect::new(0, 0, 80, 1), 0, true, true);

        assert!(view.new_tab_hit_area.width > 0, "precondition: + is drawn");
        assert_eq!(view.split_right_hit_area, Rect::new(74, 0, 3, 1));
        assert_eq!(view.split_down_hit_area, Rect::new(77, 0, 3, 1));
        assert_eq!(
            view.new_tab_hit_area.x,
            view.tab_hit_areas[0].x + view.tab_hit_areas[0].width,
            "only the split pair is pinned — the + still trails the tabs"
        );
    }

    // TP-TAB-SPLIT-01: the pin is static — adding a tab moves the `+`, not
    // the split pair. This is the asked-for behaviour: the pair used to
    // trail the `+` and drift with every new tab.
    #[test]
    fn adding_a_tab_moves_the_plus_but_not_the_split_buttons() {
        let mut ws = Workspace::test_new("test");
        let one = compute_tab_bar_view(&ws, &[], Rect::new(0, 0, 80, 1), 0, true, true);
        ws.test_add_tab(None);
        let two = compute_tab_bar_view(&ws, &[], Rect::new(0, 0, 80, 1), 0, true, true);

        assert!(
            two.new_tab_hit_area.x > one.new_tab_hit_area.x,
            "precondition: the + drifted with the new tab"
        );
        assert_eq!(two.split_right_hit_area, one.split_right_hit_area);
        assert_eq!(two.split_down_hit_area, one.split_down_hit_area);
    }

    // TP-TAB-SPLIT-01 boundary: exactly enough spare width for both seats —
    // everything sits flat, nothing overflows.
    #[test]
    fn a_strip_with_exactly_six_spare_cells_seats_both_buttons() {
        let ws = Workspace::test_new("test");
        let width = tab_width(&ws, 0) + NEW_TAB_WIDTH + SPLIT_BUTTON_WIDTH * 2;
        let view = compute_tab_bar_view(&ws, &[], Rect::new(0, 0, width, 1), 0, true, true);

        assert_eq!(view.split_right_hit_area, Rect::new(width - 6, 0, 3, 1));
        assert_eq!(view.split_down_hit_area, Rect::new(width - 3, 0, 3, 1));
        assert_eq!(
            view.scroll_left_hit_area,
            Rect::default(),
            "the reservation did not push the strip into overflow"
        );
    }

    // TP-TAB-SPLIT-01: in overflow the pair still hugs the right edge — the
    // tabs scroll behind the reserved chrome, the pin does not move.
    #[test]
    fn in_overflow_the_split_buttons_hug_the_right_edge() {
        let mut ws = Workspace::test_new("test");
        for _ in 0..8 {
            ws.test_add_tab(None);
        }
        let view = compute_tab_bar_view(&ws, &[], Rect::new(0, 0, 40, 1), 0, true, true);

        assert!(
            view.scroll_left_hit_area.width > 0,
            "precondition: the strip is in overflow"
        );
        assert_eq!(view.split_right_hit_area, Rect::new(34, 0, 3, 1));
        assert_eq!(view.split_down_hit_area, Rect::new(37, 0, 3, 1));
    }

    // TP-TAB-SPLIT-01: keyboard-driven strips carry no button chrome at all.
    #[test]
    fn without_mouse_chrome_the_split_buttons_claim_nothing() {
        let ws = Workspace::test_new("test");
        let view = compute_tab_bar_view(&ws, &[], Rect::new(0, 0, 80, 1), 0, true, false);

        assert_eq!(view.split_right_hit_area, Rect::default());
        assert_eq!(view.split_down_hit_area, Rect::default());
    }

    // TP-TAB-SPLIT-01: the boundary sits AT three cells — a single seat goes
    // whole to the right split, pinned at the edge, and the down button
    // claims nothing.
    #[test]
    fn a_strip_with_exactly_three_spare_cells_seats_the_right_button_alone() {
        let ws = Workspace::test_new("test");
        // The + button sits flush at the tab's end, so the spare cells at
        // the edge are exactly SPLIT_BUTTON_WIDTH here.
        let width = tab_width(&ws, 0) + NEW_TAB_WIDTH + SPLIT_BUTTON_WIDTH;
        let view = compute_tab_bar_view(&ws, &[], Rect::new(0, 0, width, 1), 0, true, true);

        assert_eq!(view.split_right_hit_area, Rect::new(width - 3, 0, 3, 1));
        assert_eq!(view.split_down_hit_area.width, 0);
    }

    // TP-TAB-SPLIT-01: a strip too narrow to seat the buttons claims no hit
    // cells for them — a button that cannot paint must not be pressable.
    #[test]
    fn a_strip_too_narrow_for_the_split_buttons_claims_nothing() {
        let ws = Workspace::test_new("test");
        // Room for the tab and the + button only.
        let width = tab_width(&ws, 0) + 1 + NEW_TAB_WIDTH;
        let view = compute_tab_bar_view(&ws, &[], Rect::new(0, 0, width, 1), 0, true, true);

        assert_eq!(view.split_right_hit_area.width, 0);
        assert_eq!(view.split_down_hit_area.width, 0);
        assert_eq!(
            view.scroll_left_hit_area,
            Rect::default(),
            "giving the buttons up must not push the strip into overflow"
        );
    }

    // TP-TAB-NAME-01: the strip shows at most twenty cells of a name, so a
    // long name cannot squeeze its neighbours out of reach.
    #[test]
    fn a_long_tab_name_is_clamped_to_twenty_cells() {
        let mut ws = Workspace::test_new("test");
        ws.tabs[0].set_custom_name("abcdefghijklmnopqrstuvwxyz".into());

        let label = tab_chrome_label(&ws, 0);
        assert_eq!(display_width_u16(&label), 20);
        assert!(label.ends_with('…'), "the cut is announced: {label:?}");
        assert_eq!(tab_width(&ws, 0), 24, "width follows the clamped label");
    }

    // TP-TAB-NAME-01: the boundary — a name exactly at the limit is whole,
    // no ellipsis pretending something was cut.
    #[test]
    fn a_name_exactly_at_the_limit_is_untouched() {
        let mut ws = Workspace::test_new("test");
        ws.tabs[0].set_custom_name("abcdefghijklmnopqrst".into());

        assert_eq!(tab_chrome_label(&ws, 0), "abcdefghijklmnopqrst");
    }

    #[test]
    fn a_name_under_the_limit_is_untouched() {
        let mut ws = Workspace::test_new("test");
        ws.tabs[0].set_custom_name("abcdefghijklmnopqrs".into());

        assert_eq!(tab_chrome_label(&ws, 0), "abcdefghijklmnopqrs");
    }

    // TP-TAB-NAME-01: the unseen dot and the zoom suffix are state channels;
    // the clamp runs on the name alone so neither can be swallowed by it.
    #[test]
    fn the_unseen_and_zoom_marks_survive_the_clamp() {
        let mut ws = Workspace::test_new("test");
        ws.tabs[0].set_custom_name("abcdefghijklmnopqrstuvwxyz".into());
        ws.tabs[0].unseen = true;
        ws.tabs[0].zoomed = true;

        let label = tab_chrome_label(&ws, 0);
        assert!(
            label.starts_with("\u{25cf} "),
            "unseen dot leads: {label:?}"
        );
        assert!(label.ends_with(" Z"), "zoom suffix trails: {label:?}");
        assert!(label.contains('…'), "the name itself is still clamped");
    }

    // TP-TAB-NAME-01: cells, not chars — a wide script hits the limit sooner.
    #[test]
    fn the_clamp_measures_display_cells_not_chars() {
        let mut ws = Workspace::test_new("test");
        ws.tabs[0].set_custom_name("反馈反馈反馈反馈反馈反馈".into());

        let label = tab_chrome_label(&ws, 0);
        assert!(
            (19..=20).contains(&display_width_u16(&label)),
            "clamped by display width: {label:?}"
        );
        assert!(label.ends_with('…'));
    }

    #[test]
    fn tab_bar_renders_trailing_cjk_character() {
        let mut app = AppState::test_new();
        let mut ws = Workspace::test_new("test");
        ws.tabs[0].set_custom_name("提交 herdr 的反馈".into());

        app.active = Some(0);
        app.workspaces = vec![ws];
        app.view.tab_bar_rect = Rect::new(0, 0, 30, 1);
        let view = compute_tab_bar_view(
            &app.workspaces[0],
            &[],
            app.view.tab_bar_rect,
            0,
            true,
            false,
        );
        app.view.tab_hit_areas = view.tab_hit_areas;

        let backend = TestBackend::new(30, 1);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| render_tab_bar(&app, frame, app.view.tab_bar_rect))
            .unwrap();

        let row = buffer_row_text(terminal.backend().buffer(), app.view.tab_bar_rect, 0);
        assert!(row.contains('馈'), "tab row: {row:?}");
    }

    // TP-TAB-UNSEEN-04: an unseen background tab must be unmissable on the
    // strip AND distinguishable from the active tab. Two channels carry the
    // signal — the `●` glyph (shape, survives any palette) and accent
    // foreground + bold (color). The active tab keeps its accent BACKGROUND,
    // so the two states cannot be confused side by side. Visiting the tab
    // drops both channels: state clearing alone is not enough, the person
    // judges the strip by what is drawn (the FM-preview lesson: a green suite
    // said nothing about the frame).
    #[test]
    fn an_unseen_background_tab_is_highlighted_until_visited() {
        let mut app = AppState::test_new();
        let mut ws = Workspace::test_new("test");
        let fresh = ws.test_add_tab(Some("fresh"));
        ws.tabs[fresh].unseen = true;

        app.active = Some(0);
        app.workspaces = vec![ws];
        app.view.tab_bar_rect = Rect::new(0, 0, 40, 1);

        let render = |app: &mut AppState| {
            let view = compute_tab_bar_view(
                &app.workspaces[0],
                &[],
                app.view.tab_bar_rect,
                0,
                true,
                false,
            );
            app.view.tab_hit_areas = view.tab_hit_areas.clone();
            let backend = TestBackend::new(40, 1);
            let mut terminal = Terminal::new(backend).unwrap();
            terminal
                .draw(|frame| render_tab_bar(app, frame, app.view.tab_bar_rect))
                .unwrap();
            (
                buffer_row_text(terminal.backend().buffer(), app.view.tab_bar_rect, 0),
                terminal.backend().buffer().clone(),
                view.tab_hit_areas,
            )
        };

        let accent = app.palette.accent;
        let surface0 = app.palette.surface0;

        let (row, buffer, hit_areas) = render(&mut app);
        assert!(
            row.contains("● fresh"),
            "the glyph channel must mark the unseen tab: {row:?}"
        );
        let unseen_cell = &buffer[(hit_areas[fresh].x + 1, 0)];
        assert_eq!(unseen_cell.fg, accent, "color channel: accent foreground");
        assert!(
            unseen_cell.modifier.contains(Modifier::BOLD),
            "color channel: bold"
        );
        assert_eq!(
            unseen_cell.bg, surface0,
            "an unseen tab keeps the inactive background — the accent \
             BACKGROUND belongs to the active tab alone"
        );
        let active_cell = &buffer[(hit_areas[0].x + 1, 0)];
        assert_eq!(
            active_cell.bg, accent,
            "control: the active tab is styled by background, so the two \
             states stay distinguishable side by side"
        );

        app.workspaces[0].switch_tab(fresh);
        let (row_after, buffer_after, hit_areas_after) = render(&mut app);
        assert!(
            !row_after.contains('●'),
            "visiting the tab must drop the glyph: {row_after:?}"
        );
        let visited_cell = &buffer_after[(hit_areas_after[fresh].x + 1, 0)];
        assert_eq!(
            visited_cell.bg, accent,
            "the visited tab is now simply the active tab"
        );
    }

    // TP-TAB-FLASH-02 at the output level: a freshly spawned tab REVERSES its
    // style inside the flash window (a render in the first half-period always
    // catches the bright phase), no other tab does, and once the window closes
    // the strip carries no trace — a flash that leaves residue is a bug, not
    // an effect.
    #[test]
    fn a_freshly_spawned_tab_flashes_and_the_flash_leaves_no_trace() {
        let mut app = AppState::test_new();
        let mut ws = Workspace::test_new("test");
        let fresh = ws.test_add_tab(Some("fresh"));
        ws.tabs[fresh].spawned_at = Some(std::time::Instant::now());
        app.active = Some(0);
        app.workspaces = vec![ws];
        app.view.tab_bar_rect = Rect::new(0, 0, 40, 1);

        let render = |app: &mut AppState| {
            let view = compute_tab_bar_view(
                &app.workspaces[0],
                &[],
                app.view.tab_bar_rect,
                0,
                true,
                false,
            );
            app.view.tab_hit_areas = view.tab_hit_areas.clone();
            let backend = TestBackend::new(40, 1);
            let mut terminal = Terminal::new(backend).unwrap();
            terminal
                .draw(|frame| render_tab_bar(app, frame, app.view.tab_bar_rect))
                .unwrap();
            (terminal.backend().buffer().clone(), view.tab_hit_areas)
        };
        let reversed_in = |buffer: &ratatui::buffer::Buffer, rect: Rect| {
            (rect.x..rect.x + rect.width)
                .any(|x| buffer[(x, 0)].modifier.contains(Modifier::REVERSED))
        };

        let (buffer, hit_areas) = render(&mut app);
        assert!(
            reversed_in(&buffer, hit_areas[fresh]),
            "the fresh tab must flash inside its window"
        );
        assert!(
            !reversed_in(&buffer, hit_areas[0]),
            "a tab with no spawn window must not flash"
        );

        app.workspaces[0].tabs[fresh].spawned_at =
            std::time::Instant::now().checked_sub(std::time::Duration::from_secs(3));
        let (buffer, hit_areas) = render(&mut app);
        assert!(
            !reversed_in(&buffer, hit_areas[fresh]),
            "a closed flash window must leave no trace on the strip"
        );
    }
}
