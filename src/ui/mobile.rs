use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Clear, Paragraph},
    Frame,
};

use super::sidebar::{
    agent_panel_entries, agent_panel_entries_from, grouped_child_display_label,
    mobile_space_entries, AgentPanelEntry,
};
use super::status::{agent_icon, state_dot};
use super::text::{display_width_u16, truncate_end};
use crate::app::state::{Palette, ToastKind, ToastNotification};
use crate::app::AppState;
use crate::detect::AgentState;
use crate::layout::PaneId;
use crate::terminal::TerminalRuntimeRegistry;

/// Columns each header button occupies.
///
/// Three is the floor a touch target can have and still be hit reliably: the
/// glyph plus a column of slack either side. The buttons keep this width even
/// on the narrowest viewport, and the strip between them takes the loss —
/// missing the button is a failed action, missing the strip is a failed
/// shortcut to the same action.
/// The header buttons sit in the two corners a thumb reaches least accurately
/// and are the only tap targets the phone shell always shows. Three columns is
/// roughly half the 44pt floor Apple asks for; five across the header's two
/// rows reads as a square button rather than a glyph with a hitbox drawn round
/// it. It stops there because every column a button takes is one the
/// active-tab strip loses (TP-MOB-58).
const HEADER_BUTTON_WIDTH: u16 = 5;

/// The share of the screen an open drawer covers.
///
/// The uncovered quarter does two jobs: it is the target that closes the
/// drawer, and it is the reminder that a terminal is still running underneath.
/// A full-width panel would lose both, and the way back would be invisible.
const DRAWER_NUMERATOR: u16 = 3;
const DRAWER_DENOMINATOR: u16 = 4;

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct MobileHeaderHitAreas {
    /// Left button — opens the spaces drawer.
    pub spaces_menu: Rect,
    /// The active-tab strip between the buttons. Dispatches the same action as
    /// `tabs_menu`, because it is the larger target for the same intent.
    pub tab_strip: Rect,
    /// Right button — opens the tabs drawer.
    pub tabs_menu: Rect,
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct MobileDrawerAreas {
    /// The drawer panel.
    pub panel: Rect,
    /// The strip the drawer leaves uncovered. Tapping it closes the drawer.
    pub scrim: Rect,
    /// The scrolling body inside the panel, below its title row.
    pub viewport: Rect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MobileSwitcherTarget {
    NewWorkspace,
    Workspace(usize),
    NewTab,
    Tab(usize),
    Agent {
        ws_idx: usize,
        tab_idx: usize,
        pane_id: PaneId,
    },
    Menu(usize),
    /// Hand the client back its own selection gesture, or take it back.
    ToggleSelectMode,
}

/// What a drawer row draws.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DrawerRowContent {
    SectionTitle(&'static str),
    Action(&'static str),
    Space { ws_idx: usize, indented: bool },
    Agent { entry_idx: usize },
    Tab { tab_idx: usize },
    Menu { menu_idx: usize },
    SelectMode,
    Empty(&'static str),
}

/// One entry in a drawer, in document space.
///
/// Render, hit-testing, height and the keyboard cursor all read this list.
/// They used to derive the same layout three times — the file said as much in
/// a comment asking future readers to keep them in step. A cursor would have
/// been a fourth. One producer makes disagreement impossible rather than
/// discouraged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DrawerRow {
    /// Rows this entry occupies in the drawer document.
    pub height: usize,
    /// What tapping or activating this row does, if anything.
    pub target: Option<MobileSwitcherTarget>,
    pub content: DrawerRowContent,
}

pub(crate) fn compute_mobile_header_hit_areas(_app: &AppState, area: Rect) -> MobileHeaderHitAreas {
    if area.width == 0 || area.height == 0 {
        return MobileHeaderHitAreas::default();
    }

    // The buttons hold their width and the strip absorbs whatever narrowing
    // there is, down to nothing. Overlapping targets would make one of the two
    // intents unreachable without saying which.
    // Narrowing is shared between the two buttons rather than spent entirely on
    // the right one: taking the full width for the left button first leaves the
    // right one a sliver on a viewport that is merely narrow, and the two
    // buttons are equal in importance (TP-MOB-45, TP-MOB-58).
    let button_w = HEADER_BUTTON_WIDTH.min(area.width / 2).max(1);
    let left_w = button_w.min(area.width);
    let right_w = button_w.min(area.width.saturating_sub(left_w));
    let strip_w = area.width.saturating_sub(left_w + right_w);

    MobileHeaderHitAreas {
        spaces_menu: Rect::new(area.x, area.y, left_w, area.height),
        tab_strip: Rect::new(area.x + left_w, area.y, strip_w, area.height),
        tabs_menu: Rect::new(area.x + left_w + strip_w, area.y, right_w, area.height),
    }
}

/// Width an open drawer covers inside `screen`.
fn drawer_width(screen_width: u16) -> u16 {
    let scaled = (u32::from(screen_width) * u32::from(DRAWER_NUMERATOR))
        .div_ceil(u32::from(DRAWER_DENOMINATOR)) as u16;
    scaled.clamp(1, screen_width)
}

pub(crate) fn mobile_drawer_areas(app: &AppState) -> MobileDrawerAreas {
    let screen = mobile_screen_rect(app);
    let header_h = app.view.mobile_header_rect.height;
    let body = Rect::new(
        screen.x,
        screen.y.saturating_add(header_h),
        screen.width,
        screen.height.saturating_sub(header_h),
    );
    if body.width == 0 || body.height == 0 || !app.mobile_drawer.is_open() {
        return MobileDrawerAreas::default();
    }

    let panel_w = drawer_width(body.width);
    let scrim_w = body.width.saturating_sub(panel_w);
    // The left drawer answers a question about what is outside this workspace,
    // so it comes from the left edge; the right one answers a question inside
    // it. Keeping each on its own edge lets the reader tell them apart by
    // position, before reading a word.
    let (panel_x, scrim_x) = match app.mobile_drawer {
        crate::app::state::MobileDrawer::Tabs => (body.x + scrim_w, body.x),
        _ => (body.x, body.x + panel_w),
    };
    let panel = Rect::new(panel_x, body.y, panel_w, body.height);
    let scrim = Rect::new(scrim_x, body.y, scrim_w, body.height);
    // The edge column belongs to the drawer's outer side. On the right-hand
    // drawer that side is its left column, which the body would otherwise
    // start in — so the body begins one column further in.
    let body_x = match app.mobile_drawer {
        crate::app::state::MobileDrawer::Tabs => panel.x.saturating_add(1),
        _ => panel.x,
    };
    let body_w = panel.width.saturating_sub(1);
    // Row 0 of the panel is its title; the body starts under it.
    let viewport = Rect::new(
        body_x,
        panel.y.saturating_add(1),
        body_w,
        panel.height.saturating_sub(1),
    );

    MobileDrawerAreas {
        panel,
        scrim,
        viewport,
    }
}

/// The rows an open drawer contains, in document order.
///
/// This is the one producer. Render walks it, hit-testing maps a document row
/// back through it, the scroll height sums it, and the keyboard cursor steps
/// over the entries in it that have a target.
pub(crate) fn mobile_drawer_rows(app: &AppState) -> Vec<DrawerRow> {
    match app.mobile_drawer {
        crate::app::state::MobileDrawer::None => Vec::new(),
        crate::app::state::MobileDrawer::Spaces => spaces_drawer_rows(app),
        crate::app::state::MobileDrawer::Tabs => tabs_drawer_rows(app),
    }
}

/// How many document rows a space or agent entry takes.
///
/// Two lines carry a name and its detail — branch, tab, agent state. On a
/// phone held upright that detail costs half the list: the measured switcher
/// put a third of its rows off screen with only two workspaces and three
/// agents. There, the name alone is the thing being scanned for.
fn drawer_entry_height(app: &AppState) -> usize {
    let width = app
        .view
        .mobile_header_rect
        .width
        .max(app.view.terminal_area.width);
    match super::size_class::SizeClass::of(Rect::new(0, 0, width, 24), app.mobile_width_threshold)
        .width
    {
        super::size_class::WidthClass::Tight => 1,
        _ => 2,
    }
}

fn spaces_drawer_rows(app: &AppState) -> Vec<DrawerRow> {
    let entry_h = drawer_entry_height(app);
    let mut rows = Vec::new();

    // No "spaces" section title: the panel is titled "spaces" one row above,
    // and a heading that repeats the panel above it spends a row saying
    // nothing. The later headings earn their rows by marking a change.
    rows.push(DrawerRow {
        height: 1,
        target: Some(MobileSwitcherTarget::NewWorkspace),
        content: DrawerRowContent::Action("+ new workspace"),
    });
    for (ws_idx, indented) in mobile_space_entries(app) {
        rows.push(DrawerRow {
            height: entry_h,
            target: Some(MobileSwitcherTarget::Workspace(ws_idx)),
            content: DrawerRowContent::Space { ws_idx, indented },
        });
    }

    let agents = agent_panel_entries(app);
    if !agents.is_empty() || app.agent_view_override.is_some() {
        rows.push(DrawerRow {
            height: 1,
            target: None,
            content: DrawerRowContent::SectionTitle("agents"),
        });
        if agents.is_empty() {
            rows.push(DrawerRow {
                height: 1,
                target: None,
                content: DrawerRowContent::Empty("  no matching agents"),
            });
        }
        for (entry_idx, entry) in agents.iter().enumerate() {
            rows.push(DrawerRow {
                height: entry_h,
                target: Some(MobileSwitcherTarget::Agent {
                    ws_idx: entry.ws_idx,
                    tab_idx: entry.tab_idx,
                    pane_id: entry.pane_id,
                }),
                content: DrawerRowContent::Agent { entry_idx },
            });
        }
    }

    rows.push(DrawerRow {
        height: 1,
        target: None,
        content: DrawerRowContent::SectionTitle("menu"),
    });
    rows.push(DrawerRow {
        height: 1,
        target: Some(MobileSwitcherTarget::ToggleSelectMode),
        content: DrawerRowContent::SelectMode,
    });
    for menu_idx in 0..app.global_menu_labels().len() {
        rows.push(DrawerRow {
            height: 1,
            target: Some(MobileSwitcherTarget::Menu(menu_idx)),
            content: DrawerRowContent::Menu { menu_idx },
        });
    }

    rows
}

fn tabs_drawer_rows(app: &AppState) -> Vec<DrawerRow> {
    let mut rows = vec![DrawerRow {
        height: 1,
        target: Some(MobileSwitcherTarget::NewTab),
        content: DrawerRowContent::Action("+ new tab"),
    }];
    let Some(ws) = app.active.and_then(|idx| app.workspaces.get(idx)) else {
        return rows;
    };
    for tab_idx in 0..ws.tabs.len() {
        rows.push(DrawerRow {
            height: 1,
            target: Some(MobileSwitcherTarget::Tab(tab_idx)),
            content: DrawerRowContent::Tab { tab_idx },
        });
    }
    rows
}

pub(crate) fn mobile_drawer_content_height(app: &AppState) -> usize {
    mobile_drawer_rows(app).iter().map(|row| row.height).sum()
}

pub(crate) fn mobile_drawer_max_scroll_for_height(app: &AppState, viewport_height: u16) -> usize {
    mobile_drawer_content_height(app).saturating_sub(viewport_height as usize)
}

pub(crate) fn mobile_drawer_max_scroll(app: &AppState) -> usize {
    mobile_drawer_max_scroll_for_height(app, mobile_drawer_areas(app).viewport.height)
}

/// The document rows a given row index occupies, and the row itself.
fn drawer_row_spans(rows: &[DrawerRow]) -> Vec<(std::ops::Range<usize>, &DrawerRow)> {
    let mut spans = Vec::with_capacity(rows.len());
    let mut cursor = 0usize;
    for row in rows {
        spans.push((cursor..cursor + row.height, row));
        cursor += row.height;
    }
    spans
}

/// The document rows the workspace `idx` occupies in the open drawer.
pub(crate) fn mobile_drawer_workspace_doc_range(
    app: &AppState,
    idx: usize,
) -> std::ops::Range<usize> {
    let rows = mobile_drawer_rows(app);
    drawer_row_spans(&rows)
        .into_iter()
        .find(|(_, row)| row.target == Some(MobileSwitcherTarget::Workspace(idx)))
        .map(|(span, _)| span)
        .unwrap_or(0..0)
}

/// Document rows that can hold the cursor, in order.
///
/// Section titles and empty-state lines are skipped: a cursor that stops on a
/// heading spends a keypress saying nothing, and every list here is short
/// enough that the extra stop is felt.
pub(crate) fn mobile_drawer_cursor_stops(app: &AppState) -> Vec<usize> {
    let rows = mobile_drawer_rows(app);
    drawer_row_spans(&rows)
        .into_iter()
        .filter(|(_, row)| row.target.is_some())
        .map(|(span, _)| span.start)
        .collect()
}

/// Where the cursor sits when a drawer opens.
///
/// Context, not the top: the spaces drawer opens on the workspace you are in
/// and the tabs drawer on the tab you are looking at, so the first arrow key
/// moves relative to where you already are.
pub(crate) fn mobile_drawer_default_cursor(app: &AppState) -> usize {
    let current = match app.mobile_drawer {
        crate::app::state::MobileDrawer::Tabs => app
            .active
            .and_then(|idx| app.workspaces.get(idx))
            .map(|ws| MobileSwitcherTarget::Tab(ws.active_tab_index())),
        crate::app::state::MobileDrawer::Spaces => app.active.map(MobileSwitcherTarget::Workspace),
        crate::app::state::MobileDrawer::None => None,
    };
    let rows = mobile_drawer_rows(app);
    let spans = drawer_row_spans(&rows);
    current
        .and_then(|target| {
            spans
                .iter()
                .find(|(_, row)| row.target == Some(target))
                .map(|(span, _)| span.start)
        })
        .or_else(|| mobile_drawer_cursor_stops(app).first().copied())
        .unwrap_or(0)
}

/// The target the cursor is on, if the cursor is on one.
pub(crate) fn mobile_drawer_cursor_target(app: &AppState) -> Option<MobileSwitcherTarget> {
    let rows = mobile_drawer_rows(app);
    drawer_row_spans(&rows)
        .into_iter()
        .find(|(span, _)| span.contains(&app.mobile_drawer_cursor))
        .and_then(|(_, row)| row.target)
}

/// The document range the cursor's row occupies.
pub(crate) fn mobile_drawer_cursor_doc_range(app: &AppState) -> std::ops::Range<usize> {
    let rows = mobile_drawer_rows(app);
    drawer_row_spans(&rows)
        .into_iter()
        .find(|(span, _)| span.contains(&app.mobile_drawer_cursor))
        .map(|(span, _)| span)
        .unwrap_or(0..0)
}

pub(crate) fn mobile_drawer_target_at(
    app: &AppState,
    col: u16,
    row: u16,
) -> Option<MobileSwitcherTarget> {
    let areas = mobile_drawer_areas(app);
    let content = inset_for_left_scrollbar(areas.viewport);
    if !rect_contains(content, col, row) {
        return None;
    }

    let scroll = app
        .mobile_switcher_scroll
        .min(mobile_drawer_max_scroll_for_height(
            app,
            areas.viewport.height,
        ));
    let doc_row = scroll.saturating_add(row.saturating_sub(areas.viewport.y) as usize);
    let rows = mobile_drawer_rows(app);
    drawer_row_spans(&rows)
        .into_iter()
        .find(|(span, _)| span.contains(&doc_row))
        .and_then(|(_, row)| row.target)
}

pub(crate) fn render_mobile_header(
    app: &AppState,
    terminal_runtimes: &TerminalRuntimeRegistry,
    frame: &mut Frame,
    area: Rect,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let p = &app.palette;
    fill_rect(frame, area, Style::default().bg(p.panel_bg));

    let hits = app.view.mobile_header_hits;
    render_header_button(
        app,
        frame,
        hits.spaces_menu,
        crate::app::state::MobileDrawer::Spaces,
        global_agent_counts(app).blocked > 0,
    );
    render_header_status(app, terminal_runtimes, frame, hits.tab_strip);
    render_header_button(
        app,
        frame,
        hits.tabs_menu,
        crate::app::state::MobileDrawer::Tabs,
        false,
    );
}

pub(crate) fn mobile_toast_banner_rect(area: Rect, offset_for_warning: bool) -> Rect {
    if area.width == 0 || area.height == 0 {
        return Rect::default();
    }

    let y = area.y
        + area
            .height
            .saturating_sub(1 + if offset_for_warning { 1 } else { 0 });
    Rect::new(area.x, y, area.width, 1)
}

pub(crate) fn render_mobile_toast_banner(
    frame: &mut Frame,
    area: Rect,
    toast: &ToastNotification,
    offset_for_warning: bool,
    p: &Palette,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let dot_color = match toast.kind {
        ToastKind::NeedsAttention => p.red,
        ToastKind::Finished => p.blue,
        ToastKind::UpdateInstalled => p.accent,
    };
    let banner = mobile_toast_banner_rect(area, offset_for_warning);
    let bg = p.surface0;

    frame.render_widget(Clear, banner);
    fill_rect(frame, banner, Style::default().bg(bg));
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(" ", Style::default().bg(bg)),
            Span::styled("●", Style::default().fg(dot_color).bg(bg)),
            Span::styled(" ", Style::default().bg(bg)),
            Span::styled(
                mobile_toast_title(toast),
                Style::default()
                    .fg(p.text)
                    .bg(bg)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" · ", Style::default().fg(p.overlay0).bg(bg)),
            Span::styled(&toast.context, Style::default().fg(p.overlay0).bg(bg)),
        ])),
        banner,
    );
}

/// Draw the open drawer over the terminal, leaving the scrim uncovered.
///
/// The scrim is deliberately not painted over: what shows through it is the
/// live terminal, which is both the reminder that the session is still there
/// and the target that closes the drawer.
pub(crate) fn render_mobile_drawer(
    app: &AppState,
    terminal_runtimes: &TerminalRuntimeRegistry,
    frame: &mut Frame,
) {
    let areas = mobile_drawer_areas(app);
    if areas.panel.width == 0 || areas.panel.height == 0 {
        return;
    }

    let p = &app.palette;
    frame.render_widget(Clear, areas.panel);
    fill_rect(frame, areas.panel, Style::default().bg(p.panel_bg));

    let title = match app.mobile_drawer {
        crate::app::state::MobileDrawer::Tabs => drawer_tabs_title(app),
        _ => " spaces".to_string(),
    };
    frame.render_widget(
        Paragraph::new(truncate_end(&title, areas.panel.width as usize)).style(
            Style::default()
                .fg(p.text)
                .bg(p.panel_bg)
                .add_modifier(Modifier::BOLD),
        ),
        Rect::new(areas.viewport.x, areas.panel.y, areas.viewport.width, 1),
    );

    render_mobile_drawer_content(app, terminal_runtimes, frame, areas.viewport);

    // A single column of the drawer's edge, drawn against the scrim, tells the
    // eye where the panel stops without spending a whole border row.
    if areas.scrim.width > 0 {
        let edge_x = match app.mobile_drawer {
            crate::app::state::MobileDrawer::Tabs => areas.panel.x,
            _ => areas.panel.x + areas.panel.width.saturating_sub(1),
        };
        for y in areas.panel.y..areas.panel.y + areas.panel.height {
            frame.buffer_mut()[(edge_x, y)]
                .set_symbol("│")
                .set_style(Style::default().fg(p.surface_dim).bg(p.panel_bg));
        }
    }
}

fn drawer_tabs_title(app: &AppState) -> String {
    match app.active.and_then(|idx| app.workspaces.get(idx)) {
        Some(ws) => format!(" tabs · {}", ws.display_name()),
        None => " tabs".to_string(),
    }
}

fn render_header_status(
    app: &AppState,
    terminal_runtimes: &TerminalRuntimeRegistry,
    frame: &mut Frame,
    area: Rect,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let p = &app.palette;
    let Some(ws) = app.active.and_then(|idx| app.workspaces.get(idx)) else {
        frame.render_widget(Paragraph::new(" no workspace"), area);
        return;
    };

    let (state, seen) = ws.aggregate_state(&app.terminals);
    let (dot, dot_style) = if matches!(state, AgentState::Working) {
        (
            super::spinner_frame(app.spinner_tick),
            Style::default().fg(p.yellow),
        )
    } else {
        state_dot(state, seen, p)
    };
    let tab_label = mobile_tab_status(ws);
    let row1 = Rect::new(area.x, area.y, area.width, 1);
    let tab_w = display_width_u16(&tab_label)
        .saturating_add(1)
        .min(area.width);
    let name_w = area.width.saturating_sub(tab_w);

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::raw(" "),
            Span::styled(dot, dot_style.bg(p.panel_bg)),
            Span::raw(" "),
            Span::styled(
                truncate_end(
                    &ws.display_name_from(&app.terminals, terminal_runtimes),
                    name_w.saturating_sub(4) as usize,
                ),
                Style::default()
                    .fg(p.text)
                    .bg(p.panel_bg)
                    .add_modifier(Modifier::BOLD),
            ),
        ])),
        Rect::new(row1.x, row1.y, name_w, 1),
    );
    frame.render_widget(
        Paragraph::new(tab_label)
            .style(Style::default().fg(p.overlay1).bg(p.panel_bg))
            .alignment(Alignment::Right),
        Rect::new(row1.x + name_w, row1.y, tab_w, 1),
    );

    if area.height > 1 {
        let summary_row = Rect::new(area.x, area.y + 1, area.width, 1);
        if app.mobile_select_mode.is_some() {
            // While capture is released, taps do not reach Herdr at all — so
            // the row that would explain that is the one thing that has to say
            // it, and say how to get back.
            frame.render_widget(
                Paragraph::new(truncate_end(
                    " select text · tap off in menu",
                    summary_row.width as usize,
                ))
                .style(
                    Style::default()
                        .fg(p.accent)
                        .bg(p.panel_bg)
                        .add_modifier(Modifier::BOLD),
                ),
                summary_row,
            );
        } else {
            frame.render_widget(
                Paragraph::new(agent_summary_line(app, p, area.width)),
                summary_row,
            );
        }
    }
}

fn mobile_tab_status(ws: &crate::workspace::Workspace) -> String {
    let tab_label = ws
        .tab_display_name(ws.active_tab_index())
        .unwrap_or_else(|| (ws.active_tab_index() + 1).to_string());
    if ws.tabs.len() <= 1 {
        format!("tab {tab_label}")
    } else {
        format!(
            "tab {tab_label} · {}/{}",
            ws.active_tab_index() + 1,
            ws.tabs.len()
        )
    }
}

/// Draw one of the two header buttons.
///
/// Both are the same glyph. What tells them apart is which edge they sit on
/// and what opens when they are pressed — position carries the meaning, so the
/// three columns can go to the target rather than to a label that would not
/// fit anyway.
fn render_header_button(
    app: &AppState,
    frame: &mut Frame,
    area: Rect,
    opens: crate::app::state::MobileDrawer,
    badge: bool,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let p = &app.palette;
    let active = app.mobile_drawer == opens;
    let bg = if active { p.surface_dim } else { p.surface0 };
    fill_rect(frame, area, Style::default().bg(bg));

    let glyph_y = if area.height > 1 { area.y + 1 } else { area.y };
    frame.render_widget(
        Paragraph::new("\u{2630}")
            .style(
                Style::default()
                    .fg(if active { p.accent } else { p.text })
                    .bg(bg)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center),
        Rect::new(area.x, glyph_y, area.width, 1),
    );

    // A blocked agent anywhere makes the spaces button read as "press me"
    // without the reader parsing the summary row first.
    if badge && area.height > 0 {
        let bx = area.x + area.width.saturating_sub(1);
        frame.buffer_mut()[(bx, area.y)]
            .set_symbol("\u{25cf}")
            .set_style(Style::default().fg(p.red).bg(bg));
    }
}

fn render_mobile_drawer_content(
    app: &AppState,
    terminal_runtimes: &TerminalRuntimeRegistry,
    frame: &mut Frame,
    viewport: Rect,
) {
    if viewport.width == 0 || viewport.height == 0 {
        return;
    }

    let p = &app.palette;
    let rows = mobile_drawer_rows(app);
    let total_height: usize = rows.iter().map(|row| row.height).sum();
    render_left_scrollbar(
        frame,
        viewport,
        total_height,
        viewport.height as usize,
        app.mobile_switcher_scroll,
        p,
    );
    let content = inset_for_left_scrollbar(viewport);
    if content == Rect::default() {
        return;
    }

    let scroll = app.mobile_switcher_scroll;
    let agents = agent_panel_entries_from(app, terminal_runtimes);
    let space_entries = mobile_space_entries(app);
    let focused_agent = app.active.and_then(|ws_idx| {
        let ws = app.workspaces.get(ws_idx)?;
        ws.focused_pane_id()
            .map(|pane_id| (ws_idx, ws.active_tab_index(), pane_id))
    });

    let mut doc_y = 0usize;
    for row in &rows {
        match &row.content {
            DrawerRowContent::SectionTitle(title) => {
                let title = if *title == "agents" {
                    app.agent_view_override
                        .as_ref()
                        .map(|view| {
                            format!("agents · {}", view.label.as_deref().unwrap_or("filtered"))
                        })
                        .unwrap_or_else(|| "agents".to_string())
                } else {
                    (*title).to_string()
                };
                render_section_title_at(frame, viewport, content, doc_y, scroll, &title, p);
            }
            DrawerRowContent::Action(label) => {
                render_action_row_at(frame, viewport, content, doc_y, scroll, label, p);
            }
            DrawerRowContent::Empty(label) => {
                render_one_line_item(
                    frame,
                    viewport,
                    content,
                    doc_y,
                    scroll,
                    ratatui::style::Color::Reset,
                    Line::from(Span::styled(
                        *label,
                        Style::default().fg(p.overlay0).add_modifier(Modifier::DIM),
                    )),
                );
            }
            DrawerRowContent::Space { ws_idx, indented } => {
                render_space_row(
                    app,
                    terminal_runtimes,
                    frame,
                    viewport,
                    content,
                    doc_y,
                    row.height,
                    *ws_idx,
                    *indented,
                    &space_entries,
                );
            }
            DrawerRowContent::Agent { entry_idx } => {
                if let Some(entry) = agents.get(*entry_idx) {
                    render_agent_row(
                        app,
                        frame,
                        viewport,
                        content,
                        doc_y,
                        row.height,
                        entry,
                        focused_agent,
                    );
                }
            }
            DrawerRowContent::Tab { tab_idx } => {
                render_tab_row(app, frame, viewport, content, doc_y, *tab_idx);
            }
            DrawerRowContent::SelectMode => {
                if let Some(y) = visible_y(viewport, scroll, doc_y) {
                    let on = app.mobile_select_mode.is_some();
                    frame.render_widget(
                        Paragraph::new(truncate_end(
                            &format!("  select text  [{}]", if on { "on" } else { "off" }),
                            content.width as usize,
                        ))
                        .style(
                            Style::default()
                                .fg(if on { p.accent } else { p.overlay1 })
                                .bg(p.panel_bg),
                        ),
                        Rect::new(content.x, y, content.width, 1),
                    );
                }
            }
            DrawerRowContent::Menu { menu_idx } => {
                if let Some(label) = app.global_menu_labels().get(*menu_idx) {
                    if let Some(y) = visible_y(viewport, scroll, doc_y) {
                        frame.render_widget(
                            Paragraph::new(truncate_end(
                                &format!("  {label}"),
                                content.width as usize,
                            ))
                            .style(Style::default().fg(p.overlay1).bg(p.panel_bg)),
                            Rect::new(content.x, y, content.width, 1),
                        );
                    }
                }
            }
        }
        if row.target.is_some() && drawer_row_has_cursor(app, doc_y, row.height) {
            let bg = match &row.content {
                DrawerRowContent::Space { ws_idx, .. } => {
                    mobile_item_bg(*ws_idx == app.selected, Some(*ws_idx) == app.active, p)
                }
                _ => p.panel_bg,
            };
            render_drawer_cursor_marker(frame, viewport, content, doc_y, scroll, p, bg);
        }
        doc_y += row.height;
    }
}

#[allow(clippy::too_many_arguments)] // one row, one call site; splitting the
                                     // argument list would only move the same values through a struct nobody else
                                     // constructs.
fn render_space_row(
    app: &AppState,
    terminal_runtimes: &TerminalRuntimeRegistry,
    frame: &mut Frame,
    viewport: Rect,
    content: Rect,
    doc_y: usize,
    height: usize,
    ws_idx: usize,
    indented: bool,
    space_entries: &[(usize, bool)],
) {
    let p = &app.palette;
    let Some(ws) = app.workspaces.get(ws_idx) else {
        return;
    };
    let active = Some(ws_idx) == app.active;
    let selected = ws_idx == app.selected;
    let bg = mobile_item_bg(selected, active, p);
    let (state, seen) = ws.aggregate_state(&app.terminals);
    let (dot, dot_style) = state_dot(state, seen, p);

    let mut title_spans = vec![Span::styled("  ", Style::default().bg(bg))];
    // Worktrees of the same space render as branches off their parent, so a
    // child gets an L/T connector on its name row and a matching vertical
    // continuation on its detail row.
    let detail_prefix = if indented {
        let position = space_entries
            .iter()
            .position(|(idx, _)| *idx == ws_idx)
            .unwrap_or(0);
        let last_child = !space_entries
            .get(position + 1)
            .is_some_and(|(_, indented)| *indented);
        title_spans.push(Span::styled(
            if last_child { "└─ " } else { "├─ " },
            Style::default().fg(p.overlay0).bg(bg),
        ));
        if last_child {
            "       "
        } else {
            "  │    "
        }
    } else {
        "  "
    };

    title_spans.push(Span::styled(dot, dot_style.bg(bg)));
    title_spans.push(Span::styled(" ", Style::default().bg(bg)));
    let raw_label = ws.display_name_from(&app.terminals, terminal_runtimes);
    let name = if indented {
        grouped_child_display_label(&raw_label, ws.branch().as_deref(), ws.custom_name.is_some())
    } else {
        raw_label
    };
    let name_budget = content.width.saturating_sub(if indented { 8 } else { 5 }) as usize;
    title_spans.push(Span::styled(
        truncate_end(&name, name_budget),
        Style::default()
            .fg(p.text)
            .bg(bg)
            .add_modifier(Modifier::BOLD),
    ));

    if height == 1 {
        render_one_line_item(
            frame,
            viewport,
            content,
            doc_y,
            app.mobile_switcher_scroll,
            bg,
            Line::from(title_spans),
        );
        return;
    }

    let detail = format!(
        "{detail_prefix}{} · {}",
        ws.branch().unwrap_or_else(|| "shell".into()),
        mobile_tab_status(ws)
    );
    render_two_line_item(
        frame,
        viewport,
        content,
        doc_y,
        app.mobile_switcher_scroll,
        bg,
        Line::from(title_spans),
        truncate_end(&detail, content.width as usize),
        p.overlay0,
    );
}

#[allow(clippy::too_many_arguments)] // same reasoning as `render_space_row`.
fn render_agent_row(
    app: &AppState,
    frame: &mut Frame,
    viewport: Rect,
    content: Rect,
    doc_y: usize,
    height: usize,
    entry: &AgentPanelEntry,
    focused_agent: Option<(usize, usize, PaneId)>,
) {
    let p = &app.palette;
    let active = focused_agent.is_some_and(|(ws_idx, tab_idx, pane_id)| {
        entry.ws_idx == ws_idx && entry.tab_idx == tab_idx && entry.pane_id == pane_id
    });
    let bg = mobile_item_bg(false, active, p);
    let (icon, icon_style) = agent_icon(entry.state, entry.seen, app.spinner_tick, p);
    let title = Line::from(vec![
        Span::styled("  ", Style::default().bg(bg)),
        Span::styled(icon, icon_style.bg(bg)),
        Span::styled(" ", Style::default().bg(bg)),
        Span::styled(
            truncate_end(
                &entry.primary_label,
                content.width.saturating_sub(5) as usize,
            ),
            Style::default()
                .fg(p.text)
                .bg(bg)
                .add_modifier(Modifier::BOLD),
        ),
    ]);

    if height == 1 {
        render_one_line_item(
            frame,
            viewport,
            content,
            doc_y,
            app.mobile_switcher_scroll,
            bg,
            title,
        );
        return;
    }

    render_two_line_item(
        frame,
        viewport,
        content,
        doc_y,
        app.mobile_switcher_scroll,
        bg,
        title,
        truncate_end(&mobile_agent_detail(entry), content.width as usize),
        p.overlay0,
    );
}

fn render_tab_row(
    app: &AppState,
    frame: &mut Frame,
    viewport: Rect,
    content: Rect,
    doc_y: usize,
    tab_idx: usize,
) {
    let p = &app.palette;
    let Some(ws) = app.active.and_then(|idx| app.workspaces.get(idx)) else {
        return;
    };
    let Some(tab) = ws.tabs.get(tab_idx) else {
        return;
    };
    let active = tab_idx == ws.active_tab_index();
    let bg = mobile_item_bg(false, active, p);
    let display_name = ws
        .tab_display_name(tab_idx)
        .unwrap_or_else(|| (tab_idx + 1).to_string());
    let label = if tab.is_auto_named() {
        format!("tab {display_name}")
    } else {
        format!("{} · {display_name}", tab_idx + 1)
    };
    let marker = if active { "▸ " } else { "  " };
    let title = Line::from(vec![
        Span::styled(
            marker,
            Style::default()
                .fg(if active { p.accent } else { p.overlay0 })
                .bg(bg),
        ),
        Span::styled(
            truncate_end(&label, content.width.saturating_sub(3) as usize),
            Style::default()
                .fg(p.text)
                .bg(bg)
                .add_modifier(Modifier::BOLD),
        ),
    ]);
    render_one_line_item(
        frame,
        viewport,
        content,
        doc_y,
        app.mobile_switcher_scroll,
        bg,
        title,
    );
}

fn mobile_agent_detail(entry: &AgentPanelEntry) -> String {
    let mut parts = Vec::new();
    if let Some(tab_label) = entry.primary_tab_label.as_deref() {
        parts.push(tab_label.to_string());
    }
    let status = entry
        .state_labels
        .get(super::sidebar::agent_panel_status_key(
            entry.state,
            entry.seen,
        ))
        .cloned()
        .unwrap_or_else(|| super::status::state_label(entry.state, entry.seen).to_string());
    parts.push(status);
    if let Some(agent_label) = entry.agent_label.as_deref() {
        parts.push(agent_label.to_string());
    }
    format!("  {}", parts.join(" · "))
}

fn render_section_title_at(
    frame: &mut Frame,
    viewport: Rect,
    content: Rect,
    doc_y: usize,
    scroll: usize,
    title: &str,
    p: &Palette,
) {
    let Some(y) = visible_y(viewport, scroll, doc_y) else {
        return;
    };
    render_section_title(
        frame,
        Rect::new(content.x, y, content.width.saturating_sub(1), 1),
        title,
        p,
    );
}

fn render_action_row_at(
    frame: &mut Frame,
    viewport: Rect,
    content: Rect,
    doc_y: usize,
    scroll: usize,
    label: &str,
    p: &Palette,
) {
    let Some(y) = visible_y(viewport, scroll, doc_y) else {
        return;
    };
    render_action_row(frame, Rect::new(content.x, y, content.width, 1), label, p);
}

fn render_one_line_item(
    frame: &mut Frame,
    viewport: Rect,
    content: Rect,
    doc_y: usize,
    scroll: usize,
    bg: ratatui::style::Color,
    title: Line<'_>,
) {
    fill_visible_doc_rect(
        frame,
        viewport,
        content,
        doc_y,
        1,
        Style::default().bg(bg),
        scroll,
    );
    if let Some(y) = visible_y(viewport, scroll, doc_y) {
        frame.render_widget(
            Paragraph::new(title),
            Rect::new(content.x, y, content.width, 1),
        );
    }
}

fn render_two_line_item(
    frame: &mut Frame,
    viewport: Rect,
    content: Rect,
    doc_y: usize,
    scroll: usize,
    bg: ratatui::style::Color,
    title: Line<'_>,
    detail: String,
    detail_fg: ratatui::style::Color,
) {
    fill_visible_doc_rect(
        frame,
        viewport,
        content,
        doc_y,
        2,
        Style::default().bg(bg),
        scroll,
    );
    if let Some(y) = visible_y(viewport, scroll, doc_y) {
        frame.render_widget(
            Paragraph::new(title),
            Rect::new(content.x, y, content.width, 1),
        );
    }
    if let Some(y) = visible_y(viewport, scroll, doc_y + 1) {
        frame.render_widget(
            Paragraph::new(detail).style(Style::default().fg(detail_fg).bg(bg)),
            Rect::new(content.x, y, content.width, 1),
        );
    }
}

fn visible_y(viewport: Rect, scroll: usize, doc_y: usize) -> Option<u16> {
    let offset = doc_y.checked_sub(scroll)?;
    (offset < viewport.height as usize).then_some(viewport.y + offset as u16)
}

fn fill_visible_doc_rect(
    frame: &mut Frame,
    viewport: Rect,
    content: Rect,
    doc_y: usize,
    height: usize,
    style: Style,
    scroll: usize,
) {
    for offset in 0..height {
        if let Some(y) = visible_y(viewport, scroll, doc_y + offset) {
            fill_rect(frame, Rect::new(content.x, y, content.width, 1), style);
        }
    }
}

fn mobile_item_bg(selected: bool, active: bool, p: &Palette) -> ratatui::style::Color {
    if selected {
        p.surface0
    } else if active {
        p.surface_dim
    } else {
        p.panel_bg
    }
}

/// Whether the drawer cursor is on the row starting at `doc_y`.
fn drawer_row_has_cursor(app: &AppState, doc_y: usize, height: usize) -> bool {
    app.mobile_drawer.is_open()
        && app.mobile_drawer_cursor >= doc_y
        && app.mobile_drawer_cursor < doc_y + height
}

/// Paint the cursor marker in the row's first column.
///
/// A marker rather than a background: the row background already carries two
/// meanings — selected and active — and a third would be indistinguishable
/// from them on a terminal with a small palette.
fn render_drawer_cursor_marker(
    frame: &mut Frame,
    viewport: Rect,
    content: Rect,
    doc_y: usize,
    scroll: usize,
    p: &Palette,
    bg: ratatui::style::Color,
) {
    let Some(y) = visible_y(viewport, scroll, doc_y) else {
        return;
    };
    if content.width == 0 {
        return;
    }
    frame.buffer_mut()[(content.x, y)]
        .set_symbol("\u{25b8}")
        .set_style(
            Style::default()
                .fg(p.accent)
                .bg(bg)
                .add_modifier(Modifier::BOLD),
        );
}

fn inset_for_left_scrollbar(area: Rect) -> Rect {
    if area.width <= 1 {
        return Rect::default();
    }
    Rect::new(area.x + 1, area.y, area.width - 1, area.height)
}

fn render_left_scrollbar(
    frame: &mut Frame,
    area: Rect,
    total_rows: usize,
    visible_rows: usize,
    scroll: usize,
    p: &Palette,
) {
    if area.width == 0 || area.height == 0 || visible_rows == 0 || total_rows <= visible_rows {
        return;
    }

    let track = Rect::new(area.x, area.y, 1, area.height);
    let max_scroll = total_rows.saturating_sub(visible_rows);
    let thumb_len = ((track.height as usize * visible_rows).div_ceil(total_rows))
        .max(1)
        .min(track.height as usize) as u16;
    let travel = track.height.saturating_sub(thumb_len);
    let thumb_top = track.y + ((travel as usize * scroll.min(max_scroll)) / max_scroll) as u16;

    for y in track.y..track.y + track.height {
        let is_thumb = y >= thumb_top && y < thumb_top + thumb_len;
        frame.buffer_mut()[(track.x, y)]
            .set_symbol(if is_thumb { "▌" } else { "│" })
            .set_style(
                Style::default()
                    .fg(if is_thumb { p.accent } else { p.surface_dim })
                    .bg(p.panel_bg),
            );
    }
}

fn render_section_title(frame: &mut Frame, area: Rect, title: &str, p: &Palette) {
    frame.render_widget(
        Paragraph::new(format!(" {title} ")).style(
            Style::default()
                .fg(p.overlay1)
                .bg(p.panel_bg)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        ),
        Rect::new(area.x, area.y, area.width, 1),
    );
}

fn render_action_row(frame: &mut Frame, area: Rect, label: &str, p: &Palette) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    frame.render_widget(
        Paragraph::new(format!("  {label}")).style(
            Style::default()
                .fg(p.accent)
                .bg(p.panel_bg)
                .add_modifier(Modifier::BOLD),
        ),
        area,
    );
}

fn rect_contains(rect: Rect, col: u16, row: u16) -> bool {
    rect.width > 0
        && rect.height > 0
        && col >= rect.x
        && col < rect.x + rect.width
        && row >= rect.y
        && row < rect.y + rect.height
}

fn mobile_screen_rect(app: &AppState) -> Rect {
    let header = app.view.mobile_header_rect;
    let terminal = app.view.terminal_area;
    let x = header.x.min(terminal.x);
    let y = header.y.min(terminal.y);
    let right = (header.x + header.width).max(terminal.x + terminal.width);
    let bottom = (header.y + header.height).max(terminal.y + terminal.height);
    Rect::new(x, y, right.saturating_sub(x), bottom.saturating_sub(y))
}

/// Agent state counts across every workspace. The mobile header is global on
/// purpose: while you stare at one terminal, a blocked agent anywhere should
/// still surface.
#[derive(Debug, Default, Clone, Copy)]
struct GlobalAgentCounts {
    blocked: usize,
    done: usize,
    working: usize,
    idle: usize,
}

impl GlobalAgentCounts {
    fn total(&self) -> usize {
        self.blocked + self.done + self.working + self.idle
    }

    fn any_pending(&self) -> bool {
        self.blocked > 0 || self.done > 0 || self.working > 0
    }
}

fn global_agent_counts(app: &AppState) -> GlobalAgentCounts {
    let mut counts = GlobalAgentCounts::default();
    for entry in crate::ui::all_agent_panel_entries(app) {
        match (entry.state, entry.seen) {
            (AgentState::Blocked, _) => counts.blocked += 1,
            (AgentState::Idle, false) => counts.done += 1,
            (AgentState::Working, _) => counts.working += 1,
            (AgentState::Idle, true) => counts.idle += 1,
            (AgentState::Unknown, _) => {}
        }
    }
    counts
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SummaryTone {
    Blocked,
    Done,
    Working,
    Idle,
    Muted,
}

/// Ordered, non-zero breakdown for the header roll-up: attention states lead
/// (blocked → done → working → idle). Pure so it can be unit-tested.
fn agent_summary_segments(counts: GlobalAgentCounts) -> Vec<(String, SummaryTone)> {
    if counts.total() == 0 {
        return vec![("no agents".to_string(), SummaryTone::Muted)];
    }
    if !counts.any_pending() {
        return vec![("all idle".to_string(), SummaryTone::Muted)];
    }
    let mut segments = Vec::new();
    if counts.blocked > 0 {
        segments.push((
            format!("◉ {} blocked", counts.blocked),
            SummaryTone::Blocked,
        ));
    }
    if counts.done > 0 {
        segments.push((format!("● {} done", counts.done), SummaryTone::Done));
    }
    if counts.working > 0 {
        segments.push((format!("{} working", counts.working), SummaryTone::Working));
    }
    if counts.idle > 0 {
        segments.push((format!("{} idle", counts.idle), SummaryTone::Idle));
    }
    segments
}

/// Greedily keep the most-urgent segments that fit `max_width` (counting the
/// leading space and " · " separators) and report whether any were dropped.
/// Segments are ordered by urgency, so the dropped tail is always the least
/// important state.
fn fit_summary_segments(
    segments: Vec<(String, SummaryTone)>,
    max_width: usize,
) -> (Vec<(String, SummaryTone)>, bool) {
    let mut shown = Vec::new();
    let mut used = 1usize; // leading space
    for (idx, segment) in segments.iter().enumerate() {
        let sep = if idx > 0 { 3 } else { 0 }; // " · "
        let seg_w = segment.0.chars().count();
        if used + sep + seg_w > max_width {
            break;
        }
        used += sep + seg_w;
        shown.push(segment.clone());
    }
    let truncated = shown.len() < segments.len();
    (shown, truncated)
}

fn agent_summary_line(app: &AppState, p: &Palette, max_width: u16) -> Line<'static> {
    let segments = agent_summary_segments(global_agent_counts(app));
    let (shown, truncated) = fit_summary_segments(segments, max_width as usize);

    let mut spans = vec![Span::styled(" ", Style::default().bg(p.panel_bg))];
    let mut used = 1usize;
    for (idx, (text, tone)) in shown.into_iter().enumerate() {
        if idx > 0 {
            spans.push(Span::styled(
                " · ",
                Style::default().fg(p.overlay0).bg(p.panel_bg),
            ));
            used += 3;
        }
        // Only the leading (most urgent) segment keeps its state color; the
        // rest stay dim so the urgent count is the loud thing.
        let style = if idx == 0 {
            let color = match tone {
                SummaryTone::Blocked => p.red,
                SummaryTone::Done => p.blue,
                SummaryTone::Working => p.yellow,
                SummaryTone::Idle | SummaryTone::Muted => p.overlay1,
            };
            let style = Style::default().fg(color).bg(p.panel_bg);
            if tone == SummaryTone::Muted {
                style
            } else {
                style.add_modifier(Modifier::BOLD)
            }
        } else {
            Style::default().fg(p.overlay1).bg(p.panel_bg)
        };
        used += text.chars().count();
        spans.push(Span::styled(text, style));
    }
    if truncated && used + 2 <= max_width as usize {
        spans.push(Span::styled(
            " …",
            Style::default().fg(p.overlay0).bg(p.panel_bg),
        ));
    }
    Line::from(spans)
}

fn mobile_toast_title(toast: &ToastNotification) -> String {
    match toast.kind {
        ToastKind::NeedsAttention => toast
            .title
            .strip_suffix(" needs attention")
            .map(|agent| format!("{agent} waiting"))
            .unwrap_or_else(|| toast.title.clone()),
        ToastKind::Finished => toast
            .title
            .strip_suffix(" finished")
            .map(|agent| format!("{agent} done"))
            .unwrap_or_else(|| toast.title.clone()),
        ToastKind::UpdateInstalled => "update ready".to_string(),
    }
}

fn fill_rect(frame: &mut Frame, area: Rect, style: Style) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let buf = frame.buffer_mut();
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            buf[(x, y)].set_symbol(" ");
            buf[(x, y)].set_style(style);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn agent_entry(primary_tab_label: Option<&str>, agent_label: Option<&str>) -> AgentPanelEntry {
        AgentPanelEntry {
            ws_idx: 0,
            tab_idx: 0,
            pane_id: PaneId::from_raw(1),
            primary_label: "herdr".into(),
            primary_tab_label: primary_tab_label.map(str::to_string),
            pane_label: None,
            terminal_title: None,
            terminal_title_stripped: None,
            agent_label: agent_label.map(str::to_string),
            agent_kind_label: agent_label.map(str::to_string),
            agent: agent_label.and_then(crate::detect::parse_agent_label),
            state: AgentState::Idle,
            seen: true,
            last_agent_state_change_seq: None,
            state_labels: std::collections::HashMap::new(),
            tokens: std::collections::HashMap::new(),
        }
    }

    /// A mobile app with `spaces` workspaces, each carrying an agent.
    fn drawer_app(spaces: usize, tabs: usize, w: u16, h: u16) -> AppState {
        let mut app = AppState::test_new();
        app.workspaces = (0..spaces)
            .map(|idx| crate::workspace::Workspace::test_new(&format!("ws-{idx}")))
            .collect();
        for _ in 1..tabs {
            app.workspaces[0].test_add_tab(None);
        }
        app.active = Some(0);
        app.selected = 0;
        app.ensure_test_terminals();
        for terminal in app.terminals.values_mut() {
            terminal.agent_name = Some("claude".to_string());
            terminal.state = AgentState::Working;
        }
        app.view.mobile_header_rect = Rect::new(0, 0, w, 2);
        app.view.terminal_area = Rect::new(0, 2, w, h - 2);
        app
    }

    // TP-MOB-32: an open drawer covers three quarters of the width and leaves
    // the rest showing. The uncovered strip is both the way out and the
    // reminder that a session is running under it.
    #[test]
    fn a_drawer_covers_three_quarters_and_leaves_the_rest() {
        let mut app = drawer_app(2, 1, 44, 22);
        app.mobile_drawer = crate::app::state::MobileDrawer::Spaces;
        let areas = mobile_drawer_areas(&app);
        assert_eq!(areas.panel.width, 33, "ceil(3/4 of 44)");
        assert_eq!(
            areas.panel.x, 0,
            "the spaces drawer hangs off the left edge"
        );
        assert_eq!(areas.scrim.width, 11);
        assert_eq!(areas.scrim.x, 33);
        assert_eq!(
            areas.panel.width + areas.scrim.width,
            44,
            "the two together account for the whole width"
        );
    }

    // TP-MOB-33: the tabs drawer hangs off the opposite edge, so the reader
    // tells the two apart by where they came from before reading a word.
    #[test]
    fn the_tabs_drawer_hangs_off_the_right_edge() {
        let mut app = drawer_app(2, 3, 44, 22);
        app.mobile_drawer = crate::app::state::MobileDrawer::Tabs;
        let areas = mobile_drawer_areas(&app);
        assert_eq!(areas.panel.width, 33);
        assert_eq!(areas.panel.x, 11);
        assert_eq!(areas.scrim.x, 0);
        assert_eq!(areas.scrim.width, 11);
    }

    // TP-MOB-34: a closed drawer projects no geometry at all, so nothing
    // downstream can hit-test or paint a panel that is not open.
    #[test]
    fn a_closed_drawer_projects_no_geometry() {
        let app = drawer_app(2, 1, 44, 22);
        let areas = mobile_drawer_areas(&app);
        assert_eq!(areas.panel, Rect::default());
        assert_eq!(areas.scrim, Rect::default());
        assert_eq!(mobile_drawer_rows(&app), Vec::new());
    }

    // TP-MOB-35: the drawer sits under the header, which stays visible so its
    // buttons keep working as toggles and the active tab stays readable.
    #[test]
    fn a_drawer_starts_below_the_header() {
        let mut app = drawer_app(2, 1, 44, 22);
        app.mobile_drawer = crate::app::state::MobileDrawer::Spaces;
        let areas = mobile_drawer_areas(&app);
        assert_eq!(areas.panel.y, app.view.mobile_header_rect.height);
        assert_eq!(
            areas.panel.y + areas.panel.height,
            app.view.terminal_area.y + app.view.terminal_area.height
        );
    }

    // TP-MOB-36: every row the producer emits hit-tests back to its own
    // target, at every document position it occupies. This is the guarantee
    // that replaced three independent derivations of the same layout.
    #[test]
    fn every_drawer_row_hit_tests_back_to_itself() {
        for drawer in [
            crate::app::state::MobileDrawer::Spaces,
            crate::app::state::MobileDrawer::Tabs,
        ] {
            let mut app = drawer_app(3, 4, 44, 40);
            app.mobile_drawer = drawer;
            let areas = mobile_drawer_areas(&app);
            let rows = mobile_drawer_rows(&app);
            assert!(!rows.is_empty());

            let mut doc_y = 0usize;
            for row in &rows {
                for offset in 0..row.height {
                    let screen_y = areas.viewport.y + (doc_y + offset) as u16;
                    if screen_y >= areas.viewport.y + areas.viewport.height {
                        continue;
                    }
                    let hit = mobile_drawer_target_at(&app, areas.viewport.x + 2, screen_y);
                    assert_eq!(
                        hit,
                        row.target,
                        "{drawer:?} doc row {} must hit-test to its own target",
                        doc_y + offset
                    );
                }
                doc_y += row.height;
            }
        }
    }

    // TP-MOB-37: the scroll height is the sum of the rows the producer emits.
    // Computing it separately is what let the height drift from the render.
    #[test]
    fn the_drawer_height_is_the_sum_of_its_rows() {
        let mut app = drawer_app(5, 3, 44, 22);
        for drawer in [
            crate::app::state::MobileDrawer::Spaces,
            crate::app::state::MobileDrawer::Tabs,
        ] {
            app.mobile_drawer = drawer;
            let rows = mobile_drawer_rows(&app);
            assert_eq!(
                mobile_drawer_content_height(&app),
                rows.iter().map(|row| row.height).sum::<usize>()
            );
        }
    }

    // TP-MOB-38: on a phone held upright each entry takes one row instead of
    // two. The measured switcher put a third of its rows off screen with two
    // workspaces and three agents; the detail line is what paid for that.
    #[test]
    fn a_tight_drawer_gives_each_entry_a_single_row() {
        let mut tight = drawer_app(2, 1, 36, 18);
        tight.mobile_drawer = crate::app::state::MobileDrawer::Spaces;
        let tight_rows = mobile_drawer_rows(&tight);
        assert!(
            tight_rows
                .iter()
                .filter(|row| matches!(row.content, DrawerRowContent::Space { .. }))
                .all(|row| row.height == 1),
            "tight spaces take one row"
        );

        let mut compact = drawer_app(2, 1, 52, 26);
        compact.mobile_drawer = crate::app::state::MobileDrawer::Spaces;
        assert!(
            mobile_drawer_rows(&compact)
                .iter()
                .filter(|row| matches!(row.content, DrawerRowContent::Space { .. }))
                .all(|row| row.height == 2),
            "a compact viewport keeps the detail line"
        );

        assert!(
            mobile_drawer_content_height(&tight) < mobile_drawer_content_height(&compact),
            "the tight drawer is the shorter document"
        );
    }

    // TP-MOB-39: a drawer whose content overflows can be scrolled to its end,
    // and one that fits reports no scroll at all.
    #[test]
    fn a_drawer_scrolls_only_when_its_content_overflows() {
        let mut crowded = drawer_app(12, 1, 44, 22);
        crowded.mobile_drawer = crate::app::state::MobileDrawer::Spaces;
        assert!(
            mobile_drawer_max_scroll(&crowded) > 0,
            "twelve spaces and their agents overflow a twenty-row body"
        );

        let mut small = drawer_app(1, 2, 44, 22);
        small.mobile_drawer = crate::app::state::MobileDrawer::Tabs;
        assert_eq!(
            mobile_drawer_max_scroll(&small),
            0,
            "two tabs and a create row fit without scrolling"
        );
    }

    // TP-MOB-52: turning select text on releases mouse capture, so the
    // client's own press-and-hold selection works again. With reporting on,
    // an iOS terminal suppresses its selection handles entirely.
    #[test]
    fn select_text_releases_mouse_capture_and_restores_it() {
        let mut app = drawer_app(1, 1, 44, 22);
        app.mouse_capture = true;

        app.toggle_mobile_select_mode();
        assert!(!app.mouse_capture, "capture is released");
        assert!(app.mobile_select_mode.is_some());

        app.toggle_mobile_select_mode();
        assert!(app.mouse_capture, "the previous setting comes back");
        assert!(app.mobile_select_mode.is_none());
    }

    // TP-MOB-53: a reader who had capture off keeps it off afterwards. The
    // toggle restores what was there, not a hardcoded default.
    #[test]
    fn select_text_restores_the_setting_it_found() {
        let mut app = drawer_app(1, 1, 44, 22);
        app.mouse_capture = false;
        app.toggle_mobile_select_mode();
        app.toggle_mobile_select_mode();
        assert!(!app.mouse_capture);
    }

    // TP-MOB-54: the spaces drawer offers the toggle, and it is a row the
    // cursor can reach — while capture is released, a tap reaches nothing, so
    // the keyboard is the only way back.
    #[test]
    fn the_spaces_drawer_offers_a_reachable_select_text_row() {
        let mut app = drawer_app(2, 1, 44, 22);
        app.mobile_drawer = crate::app::state::MobileDrawer::Spaces;
        let rows = mobile_drawer_rows(&app);
        assert!(
            rows.iter()
                .any(|row| row.target == Some(MobileSwitcherTarget::ToggleSelectMode)),
            "the drawer carries the toggle"
        );
        let stops = mobile_drawer_cursor_stops(&app);
        let toggle_start = drawer_row_spans(&rows)
            .into_iter()
            .find(|(_, row)| row.target == Some(MobileSwitcherTarget::ToggleSelectMode))
            .map(|(span, _)| span.start)
            .expect("the toggle has a document row");
        assert!(
            stops.contains(&toggle_start),
            "the cursor can stop on the toggle"
        );
    }

    // TP-MOB-58: the header buttons are the only always-present tap targets in
    // the phone shell, and they sit in the two corners a thumb reaches least
    // accurately. Apple's own guidance puts the floor at 44pt; a 3-column
    // button on a phone is roughly half that across. They stay square-ish
    // rather than growing without limit, because every column they take is one
    // the active-tab strip loses.
    #[test]
    fn the_header_buttons_are_wide_enough_for_a_thumb() {
        let app = drawer_app(1, 1, 76, 35);
        let hits = compute_mobile_header_hit_areas(&app, Rect::new(0, 0, 76, 2));

        assert!(
            hits.spaces_menu.width >= 5,
            "spaces button is {} columns wide",
            hits.spaces_menu.width
        );
        assert_eq!(hits.spaces_menu.width, hits.tabs_menu.width);
        assert!(
            hits.tab_strip.width > 0,
            "the strip still has to name the active tab"
        );
        assert_eq!(
            hits.spaces_menu.right(),
            hits.tab_strip.x,
            "targets must not overlap or leave a dead gap"
        );
        assert_eq!(hits.tab_strip.right(), hits.tabs_menu.x);
        assert_eq!(hits.tabs_menu.right(), 76);
    }

    // TP-MOB-59: a viewport too narrow for two full buttons degrades by
    // shrinking them rather than by overlapping them, because two targets that
    // share a cell make one of the two intents unreachable without saying so.
    #[test]
    fn the_header_buttons_shrink_before_they_overlap() {
        let app = drawer_app(1, 1, 8, 20);
        for width in 1..=12u16 {
            let hits = compute_mobile_header_hit_areas(&app, Rect::new(0, 0, width, 2));
            assert!(
                hits.spaces_menu.right() <= hits.tab_strip.x,
                "width {width}"
            );
            assert!(hits.tab_strip.right() <= hits.tabs_menu.x, "width {width}");
            assert!(hits.tabs_menu.right() <= width, "width {width}");
        }
    }

    // TP-MOB-55: while select text is on the header says so, and says how to
    // turn it off. A mode with no indicator is one the reader cannot trust,
    // and this one changes whether their taps do anything at all.
    #[test]
    fn the_header_says_when_select_text_is_on() {
        use ratatui::{backend::TestBackend, Terminal};

        let mut app = drawer_app(1, 1, 44, 22);
        app.view.mobile_header_hits = compute_mobile_header_hit_areas(&app, Rect::new(0, 0, 44, 2));
        app.toggle_mobile_select_mode();

        let mut terminal = Terminal::new(TestBackend::new(44, 2)).expect("terminal");
        terminal
            .draw(|frame| {
                render_mobile_header(
                    &app,
                    &TerminalRuntimeRegistry::new(),
                    frame,
                    Rect::new(0, 0, 44, 2),
                )
            })
            .expect("draw");
        let rendered = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(rendered.contains("select text"), "header: {rendered:?}");
        assert!(rendered.contains("menu"), "header names the way back");
    }

    #[test]
    fn global_agent_counts_ignore_active_agent_view_filter() {
        let mut app = AppState::test_new();
        app.workspaces = vec![
            crate::workspace::Workspace::test_new("blocked"),
            crate::workspace::Workspace::test_new("working"),
        ];
        app.ensure_test_terminals();
        for (ws_idx, state) in [(0, AgentState::Blocked), (1, AgentState::Working)] {
            let pane_id = app.workspaces[ws_idx].tabs[0].root_pane;
            let terminal_id = app.workspaces[ws_idx].tabs[0].panes[&pane_id]
                .attached_terminal_id
                .clone();
            let terminal = app.terminals.get_mut(&terminal_id).unwrap();
            terminal.detected_agent = Some(crate::detect::Agent::Claude);
            terminal.state = state;
        }
        app.agent_view_override = Some(crate::api::schema::AgentViewSetParams {
            source: "example.views".to_string(),
            label: None,
            filter: Some(crate::api::schema::AgentViewFilter::Eq {
                field: crate::api::schema::AgentViewField::Builtin(
                    crate::api::schema::AgentViewBuiltinField::Status,
                ),
                value: crate::api::schema::AgentViewValue::String("working".to_string()),
            }),
            sort: Vec::new(),
        });

        let counts = global_agent_counts(&app);
        assert_eq!(counts.blocked, 1);
        assert_eq!(counts.working, 1);
    }

    #[test]
    fn agent_summary_leads_with_attention_states_in_priority_order() {
        let counts = GlobalAgentCounts {
            blocked: 2,
            done: 1,
            working: 2,
            idle: 1,
        };
        let segments = agent_summary_segments(counts);
        let labels: Vec<&str> = segments.iter().map(|(text, _)| text.as_str()).collect();
        assert_eq!(
            labels,
            vec!["◉ 2 blocked", "● 1 done", "2 working", "1 idle"]
        );
        assert_eq!(segments[0].1, SummaryTone::Blocked);
    }

    #[test]
    fn agent_summary_hides_empty_categories() {
        let counts = GlobalAgentCounts {
            done: 1,
            working: 2,
            ..Default::default()
        };
        let labels: Vec<String> = agent_summary_segments(counts)
            .into_iter()
            .map(|(text, _)| text)
            .collect();
        assert_eq!(
            labels,
            vec!["● 1 done".to_string(), "2 working".to_string()]
        );
    }

    #[test]
    fn agent_summary_collapses_to_all_idle_without_attention() {
        let counts = GlobalAgentCounts {
            idle: 3,
            ..Default::default()
        };
        assert_eq!(
            agent_summary_segments(counts),
            vec![("all idle".to_string(), SummaryTone::Muted)]
        );
    }

    #[test]
    fn agent_summary_drops_least_urgent_segments_when_narrow() {
        let counts = GlobalAgentCounts {
            blocked: 2,
            done: 1,
            working: 2,
            idle: 1,
        };
        let (shown, truncated) = fit_summary_segments(agent_summary_segments(counts), 24);
        let labels: Vec<&str> = shown.iter().map(|(text, _)| text.as_str()).collect();
        assert_eq!(labels, vec!["◉ 2 blocked", "● 1 done"]);
        assert!(truncated);
    }

    #[test]
    fn agent_summary_keeps_all_segments_when_wide_enough() {
        let counts = GlobalAgentCounts {
            blocked: 2,
            done: 1,
            working: 2,
            idle: 1,
        };
        let (shown, truncated) = fit_summary_segments(agent_summary_segments(counts), 60);
        assert_eq!(shown.len(), 4);
        assert!(!truncated);
    }

    #[test]
    fn agent_summary_reports_no_agents_when_empty() {
        assert_eq!(
            agent_summary_segments(GlobalAgentCounts::default()),
            vec![("no agents".to_string(), SummaryTone::Muted)]
        );
    }

    #[test]
    fn the_spaces_drawer_leads_with_spaces_and_puts_agents_below() {
        let mut app = crate::app::state::AppState::test_new();
        let mut workspace = crate::workspace::Workspace::test_new("spaces-first");
        workspace.test_add_tab(None); // two tabs -> two agent panes
        app.workspaces = vec![workspace];
        app.ensure_test_terminals();
        for terminal in app.terminals.values_mut() {
            terminal.agent_name = Some("pi".to_string());
            terminal.state = AgentState::Working;
        }
        app.active = Some(0);
        app.selected = 0;
        app.view.mobile_header_rect = Rect::new(0, 0, 44, 2);
        app.view.terminal_area = Rect::new(0, 2, 44, 18);
        app.mobile_drawer = crate::app::state::MobileDrawer::Spaces;

        assert_eq!(agent_panel_entries(&app).len(), 2);
        // The panel is already titled "spaces", so the body opens with the
        // create row and the first space sits at doc row 1: the question the
        // reader opened this drawer to answer is the first thing in it.
        assert_eq!(mobile_drawer_workspace_doc_range(&app, 0).start, 1);

        let viewport = mobile_drawer_areas(&app).viewport;
        app.mobile_switcher_scroll = 100;
        let workspace_hit = mobile_drawer_target_at(&app, viewport.x + 2, viewport.y + 1);
        assert_eq!(workspace_hit, Some(MobileSwitcherTarget::Workspace(0)));

        // Agents follow: the create row, one two-row space and the "agents"
        // title put the first agent at doc row 4.
        let agent_hit = mobile_drawer_target_at(&app, viewport.x + 2, viewport.y + 4);
        assert!(matches!(
            agent_hit,
            Some(MobileSwitcherTarget::Agent { .. })
        ));
    }

    fn worktree_workspace(name: &str, key: &str, linked: bool) -> crate::workspace::Workspace {
        let mut ws = crate::workspace::Workspace::test_new(name);
        ws.worktree_space = Some(crate::workspace::WorktreeSpaceMembership {
            key: key.into(),
            label: "herdr".into(),
            repo_root: std::path::PathBuf::from("/repo/herdr"),
            checkout_path: std::path::PathBuf::from(format!("/repo/{name}")),
            is_linked_worktree: linked,
        });
        ws
    }

    #[test]
    fn the_spaces_drawer_follows_grouped_worktree_order() {
        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![
            worktree_workspace("main", "repo-key", false),
            crate::workspace::Workspace::test_new("other"),
            worktree_workspace("feature", "repo-key", true),
        ];
        app.active = Some(0);
        app.selected = 0;
        app.view.mobile_header_rect = Rect::new(0, 0, 44, 2);
        app.view.terminal_area = Rect::new(0, 2, 44, 18);
        app.mobile_drawer = crate::app::state::MobileDrawer::Spaces;

        // Grouped order pulls the worktree (idx 2) up under its parent (idx 0),
        // ahead of the unrelated "other" workspace (idx 1): rows are main,
        // feature, other, starting after the create row.
        assert_eq!(mobile_drawer_workspace_doc_range(&app, 2).start, 3);
        assert_eq!(mobile_drawer_workspace_doc_range(&app, 1).start, 5);

        let viewport = mobile_drawer_areas(&app).viewport;
        // The second space row on screen is the worktree, not workspaces[1].
        let hit = mobile_drawer_target_at(&app, viewport.x + 2, viewport.y + 3);
        assert_eq!(hit, Some(MobileSwitcherTarget::Workspace(2)));

        // Mobile ignores collapse: even with the space folded on desktop, the
        // worktree child still renders in the same position.
        app.collapsed_space_keys.insert("repo-key".to_string());
        assert_eq!(mobile_drawer_workspace_doc_range(&app, 2).start, 3);
        let hit = mobile_drawer_target_at(&app, viewport.x + 2, viewport.y + 3);
        assert_eq!(hit, Some(MobileSwitcherTarget::Workspace(2)));
    }

    #[test]
    fn the_spaces_drawer_without_agents_has_no_agents_section() {
        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![crate::workspace::Workspace::test_new("shell-only")];
        app.active = Some(0);
        app.selected = 0;

        app.view.mobile_header_rect = Rect::new(0, 0, 44, 2);
        app.view.terminal_area = Rect::new(0, 2, 44, 18);
        app.mobile_drawer = crate::app::state::MobileDrawer::Spaces;

        // No attached terminals -> no agents -> no agents section at all.
        assert_eq!(agent_panel_entries(&app).len(), 0);
        assert_eq!(mobile_drawer_workspace_doc_range(&app, 0).start, 1);
    }

    #[test]
    fn mobile_agent_detail_includes_tab_context_when_available() {
        let entry = agent_entry(Some("mobile-state"), Some("pi"));

        assert_eq!(mobile_agent_detail(&entry), "  mobile-state · idle · pi");
    }

    #[test]
    fn mobile_agent_detail_keeps_existing_compact_detail_without_tab_context() {
        let entry = agent_entry(None, Some("pi"));

        assert_eq!(mobile_agent_detail(&entry), "  idle · pi");
    }

    #[test]
    fn mobile_tab_status_uses_compact_tab_label_and_position() {
        let mut workspace = crate::workspace::Workspace::test_new("mobile-tabs");
        let removed_tab = workspace.test_add_tab(None);
        workspace.test_add_tab(None);
        assert!(workspace.close_tab(removed_tab));
        workspace.set_active_tab(1);

        assert_eq!(mobile_tab_status(&workspace), "tab 2 · 2/2");
    }

    #[test]
    fn the_tabs_drawer_uses_compact_tab_labels_for_auto_named_tabs() {
        let mut app = crate::app::state::AppState::test_new();
        let mut workspace = crate::workspace::Workspace::test_new("mobile-tabs");
        let removed_tab = workspace.test_add_tab(None);
        workspace.test_add_tab(None);
        assert!(workspace.close_tab(removed_tab));
        app.workspaces = vec![workspace];
        app.ensure_test_terminals();
        app.active = Some(0);
        app.selected = 0;
        app.view.mobile_header_rect = Rect::new(0, 0, 40, 2);
        app.view.terminal_area = Rect::new(0, 2, 40, 18);
        app.mobile_drawer = crate::app::state::MobileDrawer::Tabs;

        let backend = ratatui::backend::TestBackend::new(40, 20);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| render_mobile_drawer(&app, &TerminalRuntimeRegistry::new(), frame))
            .unwrap();

        let rendered = (0..20)
            .map(|y| {
                (0..40)
                    .map(|x| terminal.backend().buffer()[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");

        assert!(rendered.contains("tab 2"), "tabs drawer: {rendered:?}");
        assert!(!rendered.contains("tab 3"), "tabs drawer: {rendered:?}");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn mobile_header_uses_live_root_runtime_cwd_for_workspace_label() {
        let unique = format!(
            "herdr-mobile-header-runtime-cwd-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let root = std::env::temp_dir().join(unique);
        let stale_cwd = root.join("issue-264-nix-support");
        let live_cwd = root.join("herdr");
        std::fs::create_dir_all(stale_cwd.join(".git")).unwrap();
        std::fs::create_dir_all(live_cwd.join(".git")).unwrap();

        let mut app = crate::app::state::AppState::test_new();
        let mut workspace = crate::workspace::Workspace::test_new("stale-name");
        workspace.custom_name = None;
        workspace.identity_cwd = stale_cwd.clone();
        let pane = workspace.tabs[0].root_pane;

        app.workspaces = vec![workspace];
        app.ensure_test_terminals();
        let terminal_id = app.workspaces[0].tabs[0].panes[&pane]
            .attached_terminal_id
            .clone();
        app.terminals.get_mut(&terminal_id).unwrap().cwd = stale_cwd;
        app.active = Some(0);
        app.selected = 0;
        app.view.mobile_header_hits = compute_mobile_header_hit_areas(&app, Rect::new(0, 0, 40, 2));

        let (events, _) = tokio::sync::mpsc::channel(4);
        let runtime = crate::terminal::TerminalRuntime::spawn(
            pane,
            24,
            80,
            live_cwd.clone(),
            0,
            crate::terminal_theme::TerminalTheme::default(),
            crate::pane::PaneShellConfig::new("/bin/sh", crate::config::ShellModeConfig::NonLogin),
            &crate::pane::PaneLaunchEnv::default(),
            events,
            std::sync::Arc::new(tokio::sync::Notify::new()),
            std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        )
        .unwrap();

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        while runtime.cwd() != Some(live_cwd.clone()) && std::time::Instant::now() < deadline {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }

        let mut runtime_registry = TerminalRuntimeRegistry::new();
        runtime_registry.insert(terminal_id, runtime);
        let backend = ratatui::backend::TestBackend::new(40, 2);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                render_mobile_header(&app, &runtime_registry, frame, Rect::new(0, 0, 40, 2))
            })
            .unwrap();
        let row = (0..40)
            .map(|x| terminal.backend().buffer()[(x, 0)].symbol())
            .collect::<String>();

        for (_, runtime) in runtime_registry.drain() {
            runtime.shutdown();
        }
        let _ = std::fs::remove_dir_all(root);

        assert!(row.contains("herdr"), "header row: {row:?}");
        assert!(
            !row.contains("issue-264-nix-support"),
            "header row: {row:?}"
        );
    }
}
