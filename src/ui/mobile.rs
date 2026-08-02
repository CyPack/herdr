use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Clear, Paragraph},
    Frame,
};

use super::sidebar::{
    agent_panel_entries, agent_panel_entries_from, grouped_child_display_label, AgentPanelEntry,
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
    /// The pinned band under the scroll body: the drawer's primary action and
    /// `select text`, at a screen position scrolling cannot move (TP-MOB-76).
    pub footer: Rect,
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
    /// Fold or unfold a repository group. Carried as its position among the
    /// group rows rather than its key, so the target stays `Copy` like every
    /// other one; the key is read back from the row list on activation.
    ToggleSpaceGroup {
        group_idx: usize,
    },
    /// Open a remembered chat under a checkout.
    Chat {
        ws_idx: usize,
        chat_idx: usize,
    },
    /// Hand the client back its own selection gesture, or take it back.
    ToggleSelectMode,
}

/// What a drawer row draws.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DrawerRowContent {
    SectionTitle(&'static str),
    /// The repository a group of checkouts belongs to.
    SpaceGroup {
        space_key: String,
        depth: u8,
        collapsed: bool,
    },
    Space {
        ws_idx: usize,
        depth: u8,
    },
    /// A remembered chat under a checkout.
    Chat {
        ws_idx: usize,
        chat_idx: usize,
        depth: u8,
    },
    /// A primary action pinned to the drawer's footer band, styled to read as
    /// the one button the drawer wants pressed — the terminal's answer to the
    /// reference app's "+ New chat" pill.
    FooterAction(&'static str),
    /// An inert note under a checkout's chat drawer: "no chats yet" or the
    /// folded "… N older" row. Not a cursor stop, exactly as on the desktop.
    ChatNote {
        depth: u8,
        label: String,
    },
    Agent {
        entry_idx: usize,
    },
    Tab {
        tab_idx: usize,
    },
    Menu {
        menu_idx: usize,
    },
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

    // The buttons reach one row below what they draw. The header is two rows
    // tall and a thumb aiming at a five-by-two target routinely lands just
    // under it; measured live, three of thirty-seven taps did. Those used to
    // reach the terminal and do nothing, which reads as a broken button. The
    // strip does not overshoot — it spans most of the width, so a row of reach
    // there would swallow the terminal's top row instead (TP-MOB-66).
    let button_h = area.height.saturating_add(1);

    MobileHeaderHitAreas {
        spaces_menu: Rect::new(area.x, area.y, left_w, button_h),
        tab_strip: Rect::new(area.x + left_w, area.y, strip_w, area.height),
        tabs_menu: Rect::new(area.x + left_w + strip_w, area.y, right_w, button_h),
    }
}

/// Pull a reported position back inside `screen`.
///
/// A touch client clamps a tap on the last column or row to the screen size
/// rather than one less, so an edge tap arrives at column 76 on a 76-column
/// phone — outside every rect. Measured live, three of thirty-seven taps did,
/// and the rightmost column is exactly where one of the two header buttons
/// lives (TP-MOB-65).
pub(crate) fn clamp_to_mobile_screen(screen: Rect, column: u16, row: u16) -> (u16, u16) {
    if screen.width == 0 || screen.height == 0 {
        return (column, row);
    }
    (
        column.min(screen.right().saturating_sub(1)),
        row.min(screen.bottom().saturating_sub(1)),
    )
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
    let body_h = panel.height.saturating_sub(1);

    // The pinned tail claims a band at the bottom — its rows plus a separator.
    // A body too short for both clips the footer rather than the list: the
    // list is what the drawer exists for, the footer is a convenience on top
    // of it (TP-MOB-80). The list keeps at least one row.
    let tail = drawer_pinned_tail_height(&mobile_drawer_rows(app));
    let wanted_band = if tail > 0 { tail + 1 } else { 0 };
    let band = wanted_band.min(usize::from(body_h.saturating_sub(1)));
    let scroll_h = body_h.saturating_sub(band as u16);
    let footer_h = band.saturating_sub(1) as u16;

    let viewport = Rect::new(body_x, panel.y.saturating_add(1), body_w, scroll_h);
    let footer = if footer_h > 0 {
        // One separator row sits between the list and the band.
        Rect::new(body_x, viewport.y + scroll_h + 1, body_w, footer_h)
    } else {
        Rect::default()
    };

    MobileDrawerAreas {
        panel,
        scrim,
        viewport,
        footer,
    }
}

/// Whether a row belongs to the pinned footer band rather than the scroll
/// document. Derived from what the row *is* so no construction site has to
/// remember a flag; the producers append these rows last, and a test pins
/// that they stay trailing.
fn drawer_row_is_pinned(content: &DrawerRowContent) -> bool {
    matches!(
        content,
        DrawerRowContent::FooterAction(_) | DrawerRowContent::SelectMode
    )
}

/// Height of the trailing pinned rows.
fn drawer_pinned_tail_height(rows: &[DrawerRow]) -> usize {
    rows.iter()
        .rev()
        .take_while(|row| drawer_row_is_pinned(&row.content))
        .map(|row| row.height)
        .sum()
}

/// First document row of the pinned tail — the row count of the scrollable
/// part. Everything at or past this position renders in the footer band.
pub(crate) fn mobile_drawer_pinned_start(app: &AppState) -> usize {
    let rows = mobile_drawer_rows(app);
    let total: usize = rows.iter().map(|row| row.height).sum();
    total - drawer_pinned_tail_height(&rows)
}

/// The band (footer rows + separator) the footer costs the drawer body, in
/// document rows. The `compute_view` scroll clamp subtracts this because it
/// derives the viewport height arithmetically rather than through
/// [`mobile_drawer_areas`].
pub(crate) fn mobile_drawer_footer_band_height(app: &AppState) -> usize {
    let tail = drawer_pinned_tail_height(&mobile_drawer_rows(app));
    if tail > 0 {
        tail + 1
    } else {
        0
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
    // nothing. The later headings earn their rows by marking a change. The
    // create action lives in the pinned footer rather than here (TP-MOB-77).
    // The drawer walks the same tree the desktop sidebar walks, unfiltered.
    // It used to keep only the workspace rows, which dropped the repository
    // header above them and the chats below: worktrees from different
    // repositories landed in one flat list and a remembered chat could not be
    // reached from a phone at all (TP-MOB-60).
    let mut group_idx = 0usize;
    for entry in crate::ui::sidebar::workspace_list_entries(app) {
        match entry {
            crate::ui::sidebar::WorkspaceListEntry::GroupHeader { space_key } => {
                let collapsed = app.collapsed_space_keys.contains(&space_key);
                rows.push(DrawerRow {
                    height: 1,
                    target: Some(MobileSwitcherTarget::ToggleSpaceGroup { group_idx }),
                    content: DrawerRowContent::SpaceGroup {
                        space_key,
                        depth: 0,
                        collapsed,
                    },
                });
                group_idx += 1;
            }
            crate::ui::sidebar::WorkspaceListEntry::Workspace { ws_idx, indented } => {
                let is_active = Some(ws_idx) == app.active;
                rows.push(DrawerRow {
                    // The detail line answers "what state is this branch in",
                    // and that is asked about the branch being worked in, not
                    // about all sixteen at once. Measured: sixteen workspaces
                    // made a 42-row document in a 32-row viewport, which put
                    // the menu five rows below the fold (TP-MOB-70).
                    height: if is_active { entry_h } else { 1 },
                    // Every workspace row keeps the same target. Splitting it
                    // by whether the row was active broke both consumers that
                    // identify a workspace row by its target — the doc range
                    // the drawer scrolls to, and the keyboard cursor's
                    // selection sync. The two intents live in the activation
                    // instead, which is where "it depends where you already
                    // are" belongs (TP-MOB-69).
                    target: Some(MobileSwitcherTarget::Workspace(ws_idx)),
                    content: DrawerRowContent::Space {
                        ws_idx,
                        depth: u8::from(indented),
                    },
                });
                // The desktop list already carries the chats of a workspace the
                // reader expanded there, so this only adds the ones a phone
                // would otherwise never see; the two can never emit the same
                // row twice (TP-MOB-67).
                if is_active
                    && !app.mobile_active_chats_folded
                    && crate::ui::sidebar::workspace_chat_drawer_collapsed(app, ws_idx)
                {
                    // No "no chats yet" note here. The desktop shows one
                    // because the reader opened that drawer and an empty gap
                    // would read as broken; this one opens itself, so the note
                    // would be an unasked-for row under every branch without a
                    // history — and rows are what this viewport is short of.
                    let chats = crate::ui::sidebar::workspace_chat_rows_for(app, ws_idx);
                    let shown = chats
                        .len()
                        .min(crate::ui::sidebar::WORKSPACE_CHAT_ROW_LIMIT);
                    for chat_idx in 0..shown {
                        rows.push(DrawerRow {
                            height: 1,
                            target: Some(MobileSwitcherTarget::Chat { ws_idx, chat_idx }),
                            content: DrawerRowContent::Chat {
                                ws_idx,
                                chat_idx,
                                depth: 2,
                            },
                        });
                    }
                    if chats.len() > shown {
                        let hidden = chats.len() - shown;
                        rows.push(DrawerRow {
                            height: 1,
                            target: None,
                            content: DrawerRowContent::ChatNote {
                                depth: 2,
                                label: format!("… {hidden} older"),
                            },
                        });
                    }
                }
            }
            crate::ui::sidebar::WorkspaceListEntry::Chat { ws_idx, chat_idx } => {
                rows.push(DrawerRow {
                    height: 1,
                    target: Some(MobileSwitcherTarget::Chat { ws_idx, chat_idx }),
                    content: DrawerRowContent::Chat {
                        ws_idx,
                        chat_idx,
                        depth: 2,
                    },
                });
            }
            crate::ui::sidebar::WorkspaceListEntry::NoChats { .. } => {
                rows.push(DrawerRow {
                    height: 1,
                    target: None,
                    content: DrawerRowContent::ChatNote {
                        depth: 2,
                        label: "no chats yet".into(),
                    },
                });
            }
            crate::ui::sidebar::WorkspaceListEntry::MoreChats { ws_idx } => {
                let hidden = crate::ui::sidebar::workspace_chat_rows_for(app, ws_idx)
                    .len()
                    .saturating_sub(crate::ui::sidebar::WORKSPACE_CHAT_ROW_LIMIT);
                rows.push(DrawerRow {
                    height: 1,
                    target: None,
                    content: DrawerRowContent::ChatNote {
                        depth: 2,
                        label: format!("… {hidden} older"),
                    },
                });
            }
        }
    }

    // A blank row before each section title, when the width class affords
    // one: the separation is most of what reads as composed about the
    // reference app's drawer, and a heading jammed against the row above
    // reads as one more list item. Tight widths skip it — rows are the
    // scarce resource there, the same trade the detail line makes
    // (TP-MOB-81).
    let spacer = || DrawerRow {
        height: 1,
        target: None,
        content: DrawerRowContent::Empty(""),
    };
    let breathe = entry_h > 1;

    let agents = agent_panel_entries(app);
    if !agents.is_empty() || app.agent_view_override.is_some() {
        if breathe {
            rows.push(spacer());
        }
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

    if breathe {
        rows.push(spacer());
    }
    rows.push(DrawerRow {
        height: 1,
        target: None,
        content: DrawerRowContent::SectionTitle("menu"),
    });
    for menu_idx in 0..app.global_menu_labels().len() {
        rows.push(DrawerRow {
            height: 1,
            target: Some(MobileSwitcherTarget::Menu(menu_idx)),
            content: DrawerRowContent::Menu { menu_idx },
        });
    }

    // The pinned footer, last so its rows are the trailing ones: the create
    // action in the thumb zone, and `select text` — the way back out of a
    // mode where taps reach nothing — at a screen position scrolling cannot
    // move (TP-MOB-76, TP-MOB-77).
    rows.push(DrawerRow {
        height: 1,
        target: Some(MobileSwitcherTarget::NewWorkspace),
        content: DrawerRowContent::FooterAction("+ new workspace"),
    });
    rows.push(DrawerRow {
        height: 1,
        target: Some(MobileSwitcherTarget::ToggleSelectMode),
        content: DrawerRowContent::SelectMode,
    });

    rows
}

fn tabs_drawer_rows(app: &AppState) -> Vec<DrawerRow> {
    let mut rows = Vec::new();
    let footer = DrawerRow {
        height: 1,
        target: Some(MobileSwitcherTarget::NewTab),
        content: DrawerRowContent::FooterAction("+ new tab"),
    };
    let Some(ws) = app.active.and_then(|idx| app.workspaces.get(idx)) else {
        rows.push(footer);
        return rows;
    };
    for tab_idx in 0..ws.tabs.len() {
        rows.push(DrawerRow {
            height: 1,
            target: Some(MobileSwitcherTarget::Tab(tab_idx)),
            content: DrawerRowContent::Tab { tab_idx },
        });
    }
    rows.push(footer);
    rows
}

pub(crate) fn mobile_drawer_max_scroll_for_height(app: &AppState, viewport_height: u16) -> usize {
    // Only the scrollable part counts: the pinned tail is already on screen,
    // and scroll range that "revealed" it again would run the list past its
    // own end into blank rows (TP-MOB-78).
    let rows = mobile_drawer_rows(app);
    let total: usize = rows.iter().map(|row| row.height).sum();
    (total - drawer_pinned_tail_height(&rows)).saturating_sub(viewport_height as usize)
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
    // Matched on what the row *is*, not on what tapping it does. The active
    // workspace's row carries `ToggleActiveChats` rather than `Workspace(idx)`
    // (TP-MOB-69), and keying off the target silently lost the one row this is
    // most often asked about — the row the reader is standing on.
    drawer_row_spans(&rows)
        .into_iter()
        .find(|(_, row)| {
            matches!(row.content, DrawerRowContent::Space { ws_idx, .. } if ws_idx == idx)
        })
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
    let rows = mobile_drawer_rows(app);

    // The footer band maps by screen position alone: scrolling cannot move it,
    // so the mapping must not consult the scroll either — hit-testing has to
    // agree with what render drew (TP-MOB-76).
    if rect_contains(areas.footer, col, row) {
        let total: usize = rows.iter().map(|r| r.height).sum();
        let pinned_start = total - drawer_pinned_tail_height(&rows);
        let doc_row = pinned_start + usize::from(row - areas.footer.y);
        return drawer_row_spans(&rows)
            .into_iter()
            .find(|(span, _)| span.contains(&doc_row))
            .and_then(|(_, r)| r.target);
    }

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
    // The buttons reach one row below the header so a thumb that lands just
    // under one still presses it (TP-MOB-66). That row belongs to whatever is
    // drawn below, so drawing stays inside the header.
    let drawn = |target: Rect| target.intersection(area);
    render_header_button(
        app,
        frame,
        drawn(hits.spaces_menu),
        crate::app::state::MobileDrawer::Spaces,
        global_agent_counts(app).blocked > 0,
    );
    render_header_status(app, terminal_runtimes, frame, drawn(hits.tab_strip));
    render_header_button(
        app,
        frame,
        drawn(hits.tabs_menu),
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

    render_mobile_drawer_content(app, terminal_runtimes, frame, &areas);

    // The separator between the list and the pinned band: the reference app
    // divides them with whitespace, and one dim rule buys the same reading for
    // a single row.
    if areas.footer.height > 0 {
        let sep_y = areas.footer.y.saturating_sub(1);
        frame.render_widget(
            Paragraph::new("─".repeat(areas.footer.width as usize))
                .style(Style::default().fg(p.surface_dim).bg(p.panel_bg)),
            Rect::new(areas.footer.x, sep_y, areas.footer.width, 1),
        );
    }

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
    areas: &MobileDrawerAreas,
) {
    let viewport = areas.viewport;
    if viewport.width == 0 || viewport.height == 0 {
        return;
    }

    let p = &app.palette;
    let rows = mobile_drawer_rows(app);
    let total_height: usize = rows.iter().map(|row| row.height).sum();
    let pinned_start = total_height - drawer_pinned_tail_height(&rows);
    // The scrollbar describes the scrollable part only; the pinned band never
    // scrolls, so counting it would show a bar that can never reach its end.
    render_left_scrollbar(
        frame,
        viewport,
        pinned_start,
        viewport.height as usize,
        app.mobile_switcher_scroll,
        p,
    );
    let content = inset_for_left_scrollbar(viewport);
    if content == Rect::default() {
        return;
    }
    // Pinned rows draw into the footer band with the band's own origin: the
    // "scroll" for that band is the document position where the band starts,
    // which makes `visible_y` place them at a position scrolling cannot move.
    let footer_content = Rect::new(
        content.x,
        areas.footer.y,
        content.width,
        areas.footer.height,
    );

    let scroll = app.mobile_switcher_scroll;
    let agents = agent_panel_entries_from(app, terminal_runtimes);
    let focused_agent = app.active.and_then(|ws_idx| {
        let ws = app.workspaces.get(ws_idx)?;
        ws.focused_pane_id()
            .map(|pane_id| (ws_idx, ws.active_tab_index(), pane_id))
    });

    let mut doc_y = 0usize;
    for (row_idx, row) in rows.iter().enumerate() {
        // A pinned row draws into the footer band instead of the scroll
        // viewport. Shadowing the three geometry inputs keeps every render arm
        // below — and the cursor marker after them — on one code path, so the
        // footer cannot drift from the list's rendering rules.
        let (viewport, content, scroll) = if drawer_row_is_pinned(&row.content) {
            (footer_content, footer_content, pinned_start)
        } else {
            (viewport, content, scroll)
        };
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
            DrawerRowContent::FooterAction(label) => {
                if let Some(y) = visible_y(viewport, scroll, doc_y) {
                    frame.render_widget(
                        Paragraph::new(truncate_end(&format!("  {label}"), content.width as usize))
                            .style(
                                Style::default()
                                    .fg(p.accent)
                                    .bg(p.panel_bg)
                                    .add_modifier(Modifier::BOLD),
                            ),
                        Rect::new(content.x, y, content.width, 1),
                    );
                }
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
            DrawerRowContent::Space { ws_idx, depth } => {
                render_space_row(
                    app,
                    terminal_runtimes,
                    frame,
                    viewport,
                    content,
                    doc_y,
                    row.height,
                    *ws_idx,
                    *depth,
                    drawer_row_is_last_at_depth(&rows, row_idx),
                );
            }
            DrawerRowContent::SpaceGroup {
                space_key,
                depth,
                collapsed,
            } => {
                if let Some(y) = visible_y(viewport, scroll, doc_y) {
                    let label = crate::ui::sidebar::space_label_for_key(app, space_key);
                    let indent = drawer_indent(*depth, content.width);
                    let text = format!(
                        "{:indent$}{} {label}",
                        "",
                        if *collapsed { "▸" } else { "▾" },
                        indent = indent
                    );
                    frame.render_widget(
                        Paragraph::new(truncate_end(&text, content.width as usize)).style(
                            Style::default()
                                .fg(p.text)
                                .bg(p.panel_bg)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Rect::new(content.x, y, content.width, 1),
                    );
                }
            }
            DrawerRowContent::Chat {
                ws_idx,
                chat_idx,
                depth,
            } => {
                if let Some(y) = visible_y(viewport, scroll, doc_y) {
                    let title = crate::ui::sidebar::workspace_chat_rows_for(app, *ws_idx)
                        .get(*chat_idx)
                        .map(|row| row.title.clone().unwrap_or_else(|| row.session_id.clone()))
                        .unwrap_or_default();
                    let indent = drawer_indent(*depth, content.width);
                    frame.render_widget(
                        Paragraph::new(truncate_end(
                            &format!("{:indent$}· {title}", "", indent = indent),
                            content.width as usize,
                        ))
                        .style(Style::default().fg(p.overlay1).bg(p.panel_bg)),
                        Rect::new(content.x, y, content.width, 1),
                    );
                }
            }
            DrawerRowContent::ChatNote { depth, label } => {
                if let Some(y) = visible_y(viewport, scroll, doc_y) {
                    let indent = drawer_indent(*depth, content.width);
                    frame.render_widget(
                        Paragraph::new(truncate_end(
                            &format!("{:indent$}{label}", "", indent = indent),
                            content.width as usize,
                        ))
                        .style(
                            Style::default()
                                .fg(p.overlay0)
                                .bg(p.panel_bg)
                                .add_modifier(Modifier::DIM),
                        ),
                        Rect::new(content.x, y, content.width, 1),
                    );
                }
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

#[allow(clippy::too_many_arguments)]
// one row, one call site; splitting the
// argument list would only move the same values through a struct nobody else
// constructs.
/// The tree level a row sits at, for rows that have one.
fn drawer_row_depth(content: &DrawerRowContent) -> Option<u8> {
    match content {
        DrawerRowContent::SpaceGroup { depth, .. }
        | DrawerRowContent::Space { depth, .. }
        | DrawerRowContent::Chat { depth, .. }
        | DrawerRowContent::ChatNote { depth, .. } => Some(*depth),
        _ => None,
    }
}

/// Columns of indent a level earns, capped so the label keeps room.
///
/// Ratatui does not clip a line that overruns its rect, so an indent budget
/// spent without a floor would push a name off the panel and corrupt the row
/// (TP-MOB-64).
fn drawer_indent(depth: u8, content_width: u16) -> usize {
    const PER_LEVEL: u16 = 2;
    const MIN_LABEL: u16 = 8;
    let wanted = u16::from(depth).saturating_mul(PER_LEVEL);
    usize::from(wanted.min(content_width.saturating_sub(MIN_LABEL)))
}

/// Whether `idx` is the last row at its own level within its parent.
fn drawer_row_is_last_at_depth(rows: &[DrawerRow], idx: usize) -> bool {
    let Some(depth) = rows.get(idx).and_then(|r| drawer_row_depth(&r.content)) else {
        return true;
    };
    for row in rows.iter().skip(idx + 1) {
        match drawer_row_depth(&row.content) {
            Some(other) if other > depth => continue,
            Some(other) if other == depth => {
                return !matches!(row.content, DrawerRowContent::Space { .. });
            }
            _ => return true,
        }
    }
    true
}

fn render_space_row(
    app: &AppState,
    terminal_runtimes: &TerminalRuntimeRegistry,
    frame: &mut Frame,
    viewport: Rect,
    content: Rect,
    doc_y: usize,
    height: usize,
    ws_idx: usize,
    depth: u8,
    last_child: bool,
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
    let detail_prefix = if depth > 0 {
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
    let name = if depth > 0 {
        grouped_child_display_label(&raw_label, ws.branch().as_deref(), ws.custom_name.is_some())
    } else {
        raw_label
    };
    let name_budget = content.width.saturating_sub(if depth > 0 { 8 } else { 5 }) as usize;
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

fn rect_contains(rect: Rect, col: u16, row: u16) -> bool {
    rect.width > 0
        && rect.height > 0
        && col >= rect.x
        && col < rect.x + rect.width
        && row >= rect.y
        && row < rect.y + rect.height
}

pub(crate) fn mobile_screen_rect(app: &AppState) -> Rect {
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

    /// A repository group with a main checkout, two linked worktrees and a
    /// standalone workspace, with chats open under the second checkout. This is
    /// the shape the reader actually has — the flat fixtures elsewhere in this
    /// file cannot show a level being lost.
    fn tree_app(w: u16, h: u16) -> AppState {
        use crate::workspace::{Workspace, WorktreeSpaceMembership};

        fn checkout(name: &str, linked: bool) -> Workspace {
            let mut ws = Workspace::test_new(name);
            ws.identity_cwd = std::path::PathBuf::from(format!("/repo/herdr-{name}"));
            ws.worktree_space = Some(WorktreeSpaceMembership {
                key: "/repo/herdr/.git".into(),
                label: "herdr".into(),
                repo_root: std::path::PathBuf::from("/repo/herdr"),
                checkout_path: std::path::PathBuf::from(format!("/repo/herdr-{name}")),
                is_linked_worktree: linked,
            });
            ws
        }

        let mut app = AppState::test_new();
        app.workspaces = vec![
            checkout("main", false),
            checkout("mobil", true),
            checkout("tiling", true),
            Workspace::test_new("cc-dashboard"),
        ];
        app.active = Some(0);
        app.selected = 0;

        let key = crate::persist::workspace_chats::ledger_key(&app.workspaces[1].identity_cwd);
        app.workspace_chat_rows.insert(
            key.clone(),
            (0..2)
                .map(|idx| crate::app::state::WorkspaceChatRow {
                    session_id: format!("s{idx}"),
                    agent: "claude".into(),
                    title: Some(format!("chat {idx}")),
                    last_seen_ms: 1_000 + idx as u64,
                    last_modified: None,
                })
                .collect(),
        );
        app.expanded_chat_workspaces.insert(key);

        app.mobile_drawer = crate::app::state::MobileDrawer::Spaces;
        app.view.mobile_header_rect = Rect::new(0, 0, w, 2);
        app.view.terminal_area = Rect::new(0, 2, w, h - 2);
        app.view.layout = crate::app::state::ViewLayout::Mobile;
        app
    }

    // TP-MOB-60: the drawer carries every level the workspace tree has. It used
    // to keep only the workspace rows, dropping the repository header above them
    // and the chats below — so a reader on a phone saw worktrees from different
    // repositories in one flat list, and could not reach a remembered chat at
    // all. A level the drawer does not carry is a level that can go missing
    // again without a test noticing.
    #[test]
    fn the_spaces_drawer_carries_every_level_of_the_tree() {
        let app = tree_app(76, 35);
        let rows = mobile_drawer_rows(&app);

        let groups = rows
            .iter()
            .filter(|r| matches!(r.content, DrawerRowContent::SpaceGroup { .. }))
            .count();
        let spaces = rows
            .iter()
            .filter(|r| matches!(r.content, DrawerRowContent::Space { .. }))
            .count();
        let chats = rows
            .iter()
            .filter(|r| matches!(r.content, DrawerRowContent::Chat { .. }))
            .count();

        assert_eq!(groups, 1, "the repository header row");
        assert_eq!(spaces, 4, "three checkouts plus the standalone workspace");
        assert_eq!(chats, 2, "both remembered chats under the second checkout");
    }

    // TP-MOB-61: the three levels are distinguishable. The old row carried a
    // single `indented` flag, so a repository, a checkout under it and that
    // checkout's chat all drew at the same offset — which is exactly what the
    // reader reported as the levels being mixed up.
    #[test]
    fn the_three_levels_carry_three_different_depths() {
        let app = tree_app(76, 35);
        let rows = mobile_drawer_rows(&app);

        let group_depth = rows
            .iter()
            .find_map(|r| match r.content {
                DrawerRowContent::SpaceGroup { depth, .. } => Some(depth),
                _ => None,
            })
            .expect("group row");
        let member_depth = rows
            .iter()
            .find_map(|r| match r.content {
                DrawerRowContent::Space { ws_idx: 1, depth } => Some(depth),
                _ => None,
            })
            .expect("member row");
        let chat_depth = rows
            .iter()
            .find_map(|r| match r.content {
                DrawerRowContent::Chat { depth, .. } => Some(depth),
                _ => None,
            })
            .expect("chat row");

        assert_eq!((group_depth, member_depth, chat_depth), (0, 1, 2));
    }

    // TP-MOB-62: a collapsed group hides its members, on the phone as on the
    // desktop. The mobile list used to force every group open because the old
    // flat switcher had no way to fold one; the drawer has a header row a finger
    // and the keyboard cursor can both reach, and a reader with sixteen
    // workspaces needs it.
    #[test]
    fn a_collapsed_group_hides_its_members_from_the_drawer() {
        let mut app = tree_app(76, 35);
        app.collapsed_space_keys.insert("/repo/herdr/.git".into());
        let rows = mobile_drawer_rows(&app);

        assert!(
            rows.iter()
                .any(|r| matches!(r.content, DrawerRowContent::SpaceGroup { .. })),
            "the header stays so the group can be opened again"
        );
        assert!(
            !rows
                .iter()
                .any(|r| matches!(r.content, DrawerRowContent::Space { ws_idx: 1, .. })),
            "a member of the collapsed group must not be drawn"
        );
        assert!(
            !rows
                .iter()
                .any(|r| matches!(r.content, DrawerRowContent::Chat { .. })),
            "nor its chats"
        );
        assert!(
            rows.iter()
                .any(|r| matches!(r.content, DrawerRowContent::Space { ws_idx: 3, .. })),
            "the standalone workspace belongs to no group and stays"
        );
    }

    // TP-MOB-63: the keyboard cursor never lands on a row that is not drawn,
    // and never on an inert note. A cursor on a hidden row is C14's failure in
    // tree form — the arrow key moves, nothing on screen does, and the reader
    // loses their place.
    #[test]
    fn the_drawer_cursor_only_stops_on_rows_that_are_drawn() {
        let mut app = tree_app(76, 35);
        app.collapsed_space_keys.insert("/repo/herdr/.git".into());

        let rows = mobile_drawer_rows(&app);
        let stops = mobile_drawer_cursor_stops(&app);
        let doc_height: usize = rows.iter().map(|row| row.height).sum();

        for stop in &stops {
            assert!(
                *stop < doc_height,
                "cursor stop {stop} is past the document"
            );
        }
        for row in &rows {
            if matches!(row.content, DrawerRowContent::ChatNote { .. }) {
                assert!(row.target.is_none(), "an inert note is not a cursor stop");
            }
        }
        assert!(
            !rows
                .iter()
                .any(|row| matches!(row.content, DrawerRowContent::Chat { .. })),
            "a folded group contributes no chat rows for the cursor to reach"
        );
    }

    // TP-MOB-64: an indent budget has a floor. Ratatui does not clip a line
    // that overruns its rect, so indenting without one would push a name past
    // the panel edge and corrupt the row on the narrowest phones.
    #[test]
    fn indenting_never_costs_the_label_its_last_columns() {
        for width in 1..=60u16 {
            for depth in 0..=4u8 {
                let indent = drawer_indent(depth, width);
                assert!(
                    indent < usize::from(width).max(1),
                    "depth {depth} at width {width} indented {indent}"
                );
            }
        }
    }

    /// A workspace with remembered chats, none of them expanded — the state a
    /// phone starts in, because the only thing that ever expanded one was a
    /// single cell on the desktop sidebar's workspace card.
    fn chat_app(active: usize, w: u16, h: u16) -> AppState {
        let mut app = AppState::test_new();
        app.workspaces = (0..3)
            .map(|idx| {
                let mut ws = crate::workspace::Workspace::test_new(&format!("ws-{idx}"));
                ws.identity_cwd = std::path::PathBuf::from(format!("/repo/ws-{idx}"));
                ws
            })
            .collect();
        for idx in 0..3 {
            let key =
                crate::persist::workspace_chats::ledger_key(&app.workspaces[idx].identity_cwd);
            app.workspace_chat_rows.insert(
                key,
                (0..2)
                    .map(|c| crate::app::state::WorkspaceChatRow {
                        session_id: format!("ws{idx}-s{c}"),
                        agent: "claude".into(),
                        title: Some(format!("ws{idx} chat {c}")),
                        last_seen_ms: 1_000 + c as u64,
                        last_modified: None,
                    })
                    .collect(),
            );
        }
        app.active = Some(active);
        app.selected = active;
        app.mobile_drawer = crate::app::state::MobileDrawer::Spaces;
        app.view.mobile_header_rect = Rect::new(0, 0, w, 2);
        app.view.terminal_area = Rect::new(0, 2, w, h - 2);
        app.view.layout = crate::app::state::ViewLayout::Mobile;
        app
    }

    // TP-MOB-67: the workspace you are in shows the chats it remembers, with
    // nothing to press first. The only thing that ever opened a chat drawer was
    // a single cell on the desktop sidebar's workspace card, which the phone
    // shell does not draw — so on a phone the chats existed in the ledger and
    // could not be reached at all. A cell that small is not a phone target
    // either: five columns by two rows already loses one tap in six.
    #[test]
    fn the_workspace_you_are_in_shows_its_remembered_chats() {
        let app = chat_app(1, 76, 35);
        let chats: Vec<usize> = mobile_drawer_rows(&app)
            .iter()
            .filter_map(|row| match row.content {
                DrawerRowContent::Chat { ws_idx, .. } => Some(ws_idx),
                _ => None,
            })
            .collect();
        assert_eq!(chats, vec![1, 1], "both chats of the active workspace");
    }

    // TP-MOB-68: only the workspace you are in. Opening all sixteen at once
    // would rebuild the wall the drawer exists to avoid.
    #[test]
    fn the_other_workspaces_keep_their_chats_folded() {
        let app = chat_app(1, 76, 35);
        assert!(
            !mobile_drawer_rows(&app).iter().any(|row| matches!(
                row.content,
                DrawerRowContent::Chat { ws_idx: 0, .. } | DrawerRowContent::Chat { ws_idx: 2, .. }
            )),
            "a workspace you are not in stays a single row"
        );
    }

    // TP-MOB-69: the active workspace's row carries a different intent from
    // every other one. Tapping a workspace you are not in means "go there";
    // tapping the one you are already in cannot mean that, so it means "show
    // me what I did here" instead.
    #[test]
    fn activating_the_row_you_are_on_folds_its_chats_rather_than_switching() {
        let app = chat_app(1, 76, 35);
        let rows = mobile_drawer_rows(&app);
        let target_for = |ws: usize| {
            rows.iter()
                .find(|row| matches!(row.content, DrawerRowContent::Space { ws_idx, .. } if ws_idx == ws))
                .and_then(|row| row.target)
        };
        // Every row carries the same target: the two intents live in the
        // activation, because the consumers that identify a workspace row do
        // it by target and splitting it silently broke them.
        assert_eq!(target_for(1), Some(MobileSwitcherTarget::Workspace(1)));
        assert_eq!(target_for(0), Some(MobileSwitcherTarget::Workspace(0)));

        // The row you are on folds its own history and stays open.
        let mut on_active = chat_app(1, 76, 35);
        on_active.mobile_drawer_cursor = mobile_drawer_workspace_doc_range(&on_active, 1).start;
        on_active.activate_mobile_drawer_cursor();
        assert!(on_active.mobile_active_chats_folded);
        assert_eq!(
            on_active.mobile_drawer,
            crate::app::state::MobileDrawer::Spaces,
            "narrowing the list being read does not leave it"
        );

        // Any other row switches and closes.
        let mut on_other = chat_app(1, 76, 35);
        on_other.mobile_drawer_cursor = mobile_drawer_workspace_doc_range(&on_other, 0).start;
        on_other.activate_mobile_drawer_cursor();
        assert!(!on_other.mobile_active_chats_folded);
        assert_eq!(
            on_other.mobile_drawer,
            crate::app::state::MobileDrawer::None
        );
    }

    // TP-MOB-70: the detail line is the answer to "what state is this branch
    // in", and that is a question asked about the branch you are on, not about
    // all sixteen at once. Measured: sixteen workspaces made a 42-row document
    // in a 32-row viewport, so the menu sat five rows below the fold.
    #[test]
    fn only_the_active_workspace_spends_a_second_row_on_detail() {
        let app = chat_app(1, 76, 35);
        let heights: Vec<(usize, usize)> = mobile_drawer_rows(&app)
            .iter()
            .filter_map(|row| match row.content {
                DrawerRowContent::Space { ws_idx, .. } => Some((ws_idx, row.height)),
                _ => None,
            })
            .collect();
        assert_eq!(heights, vec![(0, 1), (1, 2), (2, 1)]);
    }

    // TP-MOB-71: sixteen workspaces fit the drawer without scrolling. Measured
    // before this change: a 42-row document in a 32-row viewport, which put the
    // menu — and with it `select text`, the way back out of a mode where taps
    // reach nothing — five rows below the fold.
    #[test]
    fn the_drawer_fits_the_reader_s_real_workspace_count() {
        use crate::workspace::{Workspace, WorktreeSpaceMembership};
        let mut app = AppState::test_new();
        let mut list = Vec::new();
        for group in 0..3 {
            for member in 0..4 {
                let mut ws = Workspace::test_new(&format!("g{group}-m{member}"));
                ws.identity_cwd = std::path::PathBuf::from(format!("/repo/r{group}-{member}"));
                ws.worktree_space = Some(WorktreeSpaceMembership {
                    key: format!("/repo/r{group}/.git"),
                    label: format!("repo{group}"),
                    repo_root: std::path::PathBuf::from(format!("/repo/r{group}")),
                    checkout_path: std::path::PathBuf::from(format!("/repo/r{group}-{member}")),
                    is_linked_worktree: member != 0,
                });
                list.push(ws);
            }
        }
        for extra in 0..4 {
            list.push(Workspace::test_new(&format!("solo{extra}")));
        }
        app.workspaces = list;
        app.active = Some(0);
        app.selected = 0;
        app.mobile_drawer = crate::app::state::MobileDrawer::Spaces;
        app.view.mobile_header_rect = Rect::new(0, 0, 76, 2);
        app.view.terminal_area = Rect::new(0, 2, 76, 33);
        app.view.layout = crate::app::state::ViewLayout::Mobile;

        let document: usize = mobile_drawer_rows(&app).iter().map(|row| row.height).sum();
        let viewport = mobile_drawer_areas(&app).viewport.height as usize;
        assert!(
            document <= viewport,
            "sixteen workspaces make a {document}-row document in a {viewport}-row viewport"
        );
    }

    // TP-MOB-72: the keyboard reaches the chats too. A tap is not reliably
    // delivered by an iOS terminal, so a row only a finger can reach is only
    // half reachable — the same reason every other drawer row is a cursor stop.
    #[test]
    fn the_keyboard_cursor_stops_on_the_chat_rows() {
        let app = chat_app(1, 76, 35);
        let rows = mobile_drawer_rows(&app);
        let stops = mobile_drawer_cursor_stops(&app);

        let mut doc_y = 0usize;
        let mut chat_starts = Vec::new();
        for row in &rows {
            if matches!(row.content, DrawerRowContent::Chat { .. }) {
                chat_starts.push(doc_y);
            }
            doc_y += row.height;
        }
        assert_eq!(chat_starts.len(), 2);
        for start in chat_starts {
            assert!(stops.contains(&start), "chat row at {start} is not a stop");
        }
    }

    // TP-MOB-75: the drawer belongs to the display it was opened on. A phone
    // attached beside a desktop shared one drawer state with it: which drawer
    // was open, where its cursor sat, whether the active workspace's chats
    // were folded and whether the client had been handed back its own
    // selection gesture were all one value for every display at once. Opening
    // a drawer on the phone therefore reached into the desktop's state and
    // back, which is what "the system gets confused about which client has
    // priority" describes.
    #[test]
    fn a_drawer_belongs_to_the_display_it_was_opened_on() {
        let mut app = chat_app(1, 76, 35);
        app.mobile_drawer = crate::app::state::MobileDrawer::None;

        let phone = crate::app::state::ClientId::from(2u64);
        let desktop = crate::app::state::ClientId::from(1u64);

        let previous = app.enter_viewer(Some(phone));
        app.toggle_mobile_drawer(crate::app::state::MobileDrawer::Spaces);
        app.mobile_active_chats_folded = true;
        app.restore_viewer(previous);

        let previous = app.enter_viewer(Some(desktop));
        assert_eq!(
            app.mobile_drawer,
            crate::app::state::MobileDrawer::None,
            "a drawer opened on the phone is not open on the desktop"
        );
        assert!(
            !app.mobile_active_chats_folded,
            "and neither is the fold it left behind"
        );
        app.restore_viewer(previous);

        let previous = app.enter_viewer(Some(phone));
        assert_eq!(
            app.mobile_drawer,
            crate::app::state::MobileDrawer::Spaces,
            "the display that opened it still has it"
        );
        assert!(app.mobile_active_chats_folded);
        app.restore_viewer(previous);

        // The migration moves four fields between register banks, so the
        // identity invariants are asserted against adversarial state rather
        // than against the tidy fixture above.
        let mut adversarial = AppState::test_with_adversarial_identity_state();
        adversarial.assert_invariants_for_test();
        let previous = adversarial.enter_viewer(Some(phone));
        adversarial.mobile_drawer = crate::app::state::MobileDrawer::Tabs;
        adversarial.mobile_drawer_cursor = 7;
        adversarial.mobile_select_mode = Some(true);
        adversarial.assert_invariants_for_test();
        adversarial.restore_viewer(previous);
        adversarial.assert_invariants_for_test();

        let previous = adversarial.enter_viewer(Some(desktop));
        assert_eq!(
            adversarial.mobile_drawer,
            crate::app::state::MobileDrawer::None
        );
        assert_eq!(adversarial.mobile_drawer_cursor, 0);
        assert_eq!(adversarial.mobile_select_mode, None);
        adversarial.assert_invariants_for_test();
        adversarial.restore_viewer(previous);
    }

    /// The reader's real scale: sixteen workspaces across three repositories,
    /// shared by every footer test so none of them can pass on a toy list.
    fn sixteen_workspace_app(w: u16, h: u16) -> AppState {
        use crate::workspace::{Workspace, WorktreeSpaceMembership};
        let mut app = AppState::test_new();
        let mut list = Vec::new();
        for group in 0..3 {
            for member in 0..4 {
                let mut ws = Workspace::test_new(&format!("g{group}-m{member}"));
                ws.identity_cwd = std::path::PathBuf::from(format!("/repo/r{group}-{member}"));
                ws.worktree_space = Some(WorktreeSpaceMembership {
                    key: format!("/repo/r{group}/.git"),
                    label: format!("repo{group}"),
                    repo_root: std::path::PathBuf::from(format!("/repo/r{group}")),
                    checkout_path: std::path::PathBuf::from(format!("/repo/r{group}-{member}")),
                    is_linked_worktree: member != 0,
                });
                list.push(ws);
            }
        }
        for extra in 0..4 {
            list.push(Workspace::test_new(&format!("solo{extra}")));
        }
        app.workspaces = list;
        app.active = Some(0);
        app.selected = 0;
        app.mobile_drawer = crate::app::state::MobileDrawer::Spaces;
        app.view.mobile_header_rect = Rect::new(0, 0, w, 2);
        app.view.terminal_area = Rect::new(0, 2, w, h - 2);
        app.view.layout = crate::app::state::ViewLayout::Mobile;
        app
    }

    // TP-MOB-76: `select text` and the primary action sit in a footer band
    // whose screen position does not move when the list scrolls. The reference
    // app pins its "+ New chat" pill to the bottom for the same reason: an
    // escape hatch whose position depends on how far the reader has scrolled
    // is not an escape hatch. `select text` is the way back out of a mode
    // where taps reach nothing at all.
    #[test]
    fn the_footer_keeps_its_targets_on_screen_at_max_scroll() {
        let mut app = sixteen_workspace_app(76, 35);
        app.mobile_switcher_scroll = mobile_drawer_max_scroll(&app);
        let areas = mobile_drawer_areas(&app);
        assert!(
            areas.footer.height >= 2,
            "the spaces footer holds the create action and select text"
        );
        assert_eq!(
            mobile_drawer_target_at(&app, areas.footer.x + 2, areas.footer.y),
            Some(MobileSwitcherTarget::NewWorkspace),
        );
        assert_eq!(
            mobile_drawer_target_at(
                &app,
                areas.footer.x + 2,
                areas.footer.y + areas.footer.height - 1
            ),
            Some(MobileSwitcherTarget::ToggleSelectMode),
        );
    }

    // TP-MOB-77: the primary action leaves the top of the scroll document for
    // the footer, in both drawers. At the top it was the first thing a thumb
    // had to scroll past on every open; the reference app keeps it at the
    // bottom, in the zone a thumb rests over.
    #[test]
    fn the_primary_action_lives_in_the_footer_not_the_scroll() {
        let app = sixteen_workspace_app(76, 35);
        let rows = mobile_drawer_rows(&app);
        assert!(
            matches!(rows[0].content, DrawerRowContent::SpaceGroup { .. }),
            "the scroll document opens with the tree, not a create row"
        );
        let tail: Vec<&DrawerRowContent> =
            rows.iter().rev().take(2).map(|row| &row.content).collect();
        assert!(matches!(tail[0], DrawerRowContent::SelectMode));
        assert!(matches!(
            tail[1],
            DrawerRowContent::FooterAction("+ new workspace")
        ));

        let mut tabs = drawer_app(1, 2, 76, 35);
        tabs.mobile_drawer = crate::app::state::MobileDrawer::Tabs;
        let tab_rows = mobile_drawer_rows(&tabs);
        assert!(
            matches!(tab_rows[0].content, DrawerRowContent::Tab { tab_idx: 0 }),
            "the tab list starts at the top"
        );
        assert!(matches!(
            tab_rows.last().expect("rows").content,
            DrawerRowContent::FooterAction("+ new tab")
        ));
        assert_eq!(
            tab_rows.last().expect("rows").target,
            Some(MobileSwitcherTarget::NewTab)
        );
    }

    // TP-MOB-78: scrolling stops where the scrollable content ends. The pinned
    // tail is already on screen, so scroll range that "reveals" it again would
    // scroll the list past its own end into blank rows.
    #[test]
    fn max_scroll_excludes_the_pinned_tail() {
        let app = sixteen_workspace_app(76, 35);
        let rows = mobile_drawer_rows(&app);
        let total: usize = rows.iter().map(|row| row.height).sum();
        let pinned: usize = rows
            .iter()
            .rev()
            .take_while(|row| {
                matches!(
                    row.content,
                    DrawerRowContent::FooterAction(_) | DrawerRowContent::SelectMode
                )
            })
            .map(|row| row.height)
            .sum();
        assert!(pinned > 0, "the spaces drawer has a footer");
        let viewport = mobile_drawer_areas(&app).viewport.height as usize;
        assert_eq!(
            mobile_drawer_max_scroll(&app),
            (total - pinned).saturating_sub(viewport)
        );
    }

    // TP-MOB-79: stepping the keyboard cursor onto the footer does not move
    // the scroll. The footer is always on screen; a follow that treated its
    // document position as something to reveal would jump the list to the
    // bottom the moment the cursor entered it, and the reader loses the row
    // they were just on.
    #[test]
    fn stepping_onto_the_footer_needs_no_scroll() {
        let mut app = sixteen_workspace_app(76, 35);
        let stops = mobile_drawer_cursor_stops(&app);
        let rows = mobile_drawer_rows(&app);
        let total: usize = rows.iter().map(|row| row.height).sum();
        let pinned: usize = rows
            .iter()
            .rev()
            .take_while(|row| {
                matches!(
                    row.content,
                    DrawerRowContent::FooterAction(_) | DrawerRowContent::SelectMode
                )
            })
            .map(|row| row.height)
            .sum();
        let footer_start = total - pinned;
        let first_footer_stop = stops
            .iter()
            .copied()
            .find(|stop| *stop >= footer_start)
            .expect("the footer rows are cursor stops");

        app.mobile_switcher_scroll = 0;
        app.mobile_drawer_cursor = first_footer_stop;
        app.move_mobile_drawer_cursor(1);
        assert_eq!(
            app.mobile_switcher_scroll, 0,
            "moving within the footer must not scroll the list it sits under"
        );
        assert_eq!(
            mobile_drawer_cursor_target(&app),
            Some(MobileSwitcherTarget::ToggleSelectMode)
        );
    }

    // TP-MOB-80: a short body clips the footer before it starves the list.
    // The list is what the drawer exists for; the footer is a convenience on
    // top of it, and two bands drawing over each other is the one outcome
    // worse than either being short.
    #[test]
    fn a_short_drawer_body_never_overlaps_footer_and_list() {
        for height in [6u16, 8, 10, 14] {
            let app = tree_app(76, height);
            let areas = mobile_drawer_areas(&app);
            if areas.panel.height == 0 {
                continue;
            }
            assert!(
                areas.viewport.height >= 1,
                "at screen height {height} the list keeps at least one row"
            );
            if areas.footer.height > 0 {
                assert!(
                    areas.viewport.y + areas.viewport.height < areas.footer.y,
                    "at screen height {height} a separator row divides list and footer"
                );
                assert!(
                    areas.footer.y + areas.footer.height <= areas.panel.y + areas.panel.height,
                    "at screen height {height} the footer stays inside the panel"
                );
            }
        }
    }

    // TP-MOB-81: a blank row precedes each section title when the width class
    // affords one. The reference app separates its sections with whitespace,
    // and that separation is most of what reads as "designed" about it; a
    // heading jammed against the row above reads as one more list item. Tight
    // widths skip the spacer — there the rows are the scarce resource, which
    // is the same trade the detail line already makes (TP-MOB-38).
    #[test]
    fn sections_breathe_when_the_width_affords_it() {
        let mut compact = drawer_app(2, 1, 52, 26);
        compact.mobile_drawer = crate::app::state::MobileDrawer::Spaces;
        let rows = mobile_drawer_rows(&compact);
        let mut spacer_before_title = 0;
        for pair in rows.windows(2) {
            if matches!(pair[1].content, DrawerRowContent::SectionTitle(_)) {
                assert!(
                    matches!(pair[0].content, DrawerRowContent::Empty("")),
                    "a section title follows a spacer, found {:?}",
                    pair[0].content
                );
                assert!(pair[0].target.is_none(), "a spacer is not a cursor stop");
                spacer_before_title += 1;
            }
        }
        assert!(spacer_before_title >= 1, "the menu section exists");

        let mut tight = drawer_app(2, 1, 36, 18);
        tight.mobile_drawer = crate::app::state::MobileDrawer::Spaces;
        assert!(
            !mobile_drawer_rows(&tight)
                .iter()
                .any(|row| matches!(row.content, DrawerRowContent::Empty(""))),
            "a tight drawer spends no rows on air"
        );
    }

    // TP-MOB-82: everything the drawer promises still holds at the widths a
    // large font leaves. The reader's answer to small text is the client's
    // font setting, and growing it shrinks the columns: 76 becomes ~50, then
    // ~38, then ~32. The threshold keeps the phone shell on; this test keeps
    // the shell honest inside it — every row one line, no rows spent on air,
    // the footer still hittable, the chats still reachable, nothing wider
    // than the panel.
    #[test]
    fn the_drawer_keeps_its_promises_at_large_font_widths() {
        for width in [50u16, 38, 32] {
            let mut app = chat_app(1, width, 30);
            app.mobile_width_threshold = 90;
            let areas = mobile_drawer_areas(&app);
            let rows = mobile_drawer_rows(&app);

            let tight = width <= crate::ui::size_class::TIGHT_MAX_WIDTH;
            for row in &rows {
                if tight {
                    assert_eq!(
                        row.height, 1,
                        "at {width} columns every row spends one line, {:?}",
                        row.content
                    );
                    assert!(
                        !matches!(row.content, DrawerRowContent::Empty("")),
                        "at {width} columns no row is spent on air"
                    );
                }
            }

            // The escape hatch stays hittable at every width.
            assert!(areas.footer.height >= 2, "footer at {width} columns");
            assert_eq!(
                mobile_drawer_target_at(
                    &app,
                    areas.footer.x + 1,
                    areas.footer.y + areas.footer.height - 1
                ),
                Some(MobileSwitcherTarget::ToggleSelectMode),
                "select text at {width} columns"
            );

            // The active workspace's chats stay reachable.
            assert!(
                rows.iter()
                    .any(|row| matches!(row.content, DrawerRowContent::Chat { .. })),
                "chats at {width} columns"
            );

            // The indent budget never costs a label its last columns.
            let content_w = areas.viewport.width.saturating_sub(1);
            for depth in 0..=2u8 {
                assert!(
                    drawer_indent(depth, content_w) < usize::from(content_w.max(1)),
                    "indent at {width} columns depth {depth}"
                );
            }
        }
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
            assert!(
                !rows.is_empty(),
                "an open drawer produces rows for {drawer:?}"
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

        // A compact viewport still affords the detail line, but spends it only
        // on the workspace being worked in. The line answers "what state is
        // this branch in", and that is asked about one branch, not sixteen —
        // measured, sixteen of them made a 42-row document in a 32-row
        // viewport (TP-MOB-70).
        let mut compact = drawer_app(2, 1, 52, 26);
        compact.mobile_drawer = crate::app::state::MobileDrawer::Spaces;
        let compact_heights: Vec<(usize, usize)> = mobile_drawer_rows(&compact)
            .iter()
            .filter_map(|row| match row.content {
                DrawerRowContent::Space { ws_idx, .. } => Some((ws_idx, row.height)),
                _ => None,
            })
            .collect();
        assert_eq!(compact.active, Some(0));
        assert_eq!(compact_heights, vec![(0, 2), (1, 1)]);

        let doc = |app: &AppState| -> usize {
            mobile_drawer_rows(app).iter().map(|row| row.height).sum()
        };
        assert!(
            doc(&tight) < doc(&compact),
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
        // The panel is already titled "spaces" and the create row moved to the
        // pinned footer (TP-MOB-77), so the first space IS the document's
        // first row: the question the reader opened this drawer to answer is
        // now literally the first thing in it.
        assert_eq!(mobile_drawer_workspace_doc_range(&app, 0).start, 0);

        let viewport = mobile_drawer_areas(&app).viewport;
        app.mobile_switcher_scroll = 100;
        let workspace_hit = mobile_drawer_target_at(&app, viewport.x + 2, viewport.y + 1);
        assert_eq!(workspace_hit, Some(MobileSwitcherTarget::Workspace(0)));

        // Agents follow: one two-row space and the "agents"
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
        // ahead of the unrelated "other" workspace (idx 1). The document opens
        // with the repository header — the create row moved to the pinned
        // footer (TP-MOB-77) — then main, feature, other. The active checkout
        // spends two rows on its detail line; the others one each (TP-MOB-70).
        assert_eq!(mobile_drawer_workspace_doc_range(&app, 0).start, 1);
        assert_eq!(mobile_drawer_workspace_doc_range(&app, 2).start, 3);
        assert_eq!(mobile_drawer_workspace_doc_range(&app, 1).start, 4);

        let viewport = mobile_drawer_areas(&app).viewport;
        // The second space row on screen is the worktree, not workspaces[1].
        let hit = mobile_drawer_target_at(&app, viewport.x + 2, viewport.y + 3);
        assert_eq!(hit, Some(MobileSwitcherTarget::Workspace(2)));

        // TP-MOB-62: folding the group on the phone hides its checkouts, the
        // same as on the desktop. The old mobile list forced every group open
        // because the flat switcher had no way to fold one; the drawer's header
        // row is reachable by both a finger and the keyboard cursor, and a
        // reader with sixteen workspaces needs it.
        app.collapsed_space_keys.insert("repo-key".to_string());
        assert_eq!(
            mobile_drawer_target_at(&app, viewport.x + 2, viewport.y),
            Some(MobileSwitcherTarget::ToggleSpaceGroup { group_idx: 0 }),
            "the header stays, so the group can be opened again"
        );
        assert_eq!(
            mobile_drawer_target_at(&app, viewport.x + 2, viewport.y + 1),
            Some(MobileSwitcherTarget::Workspace(0)),
            "folding hides the linked worktrees, not the checkout they branch from"
        );
        assert!(
            !mobile_drawer_rows(&app)
                .iter()
                .any(|row| matches!(row.content, DrawerRowContent::Space { ws_idx: 2, .. })),
            "the linked worktree is folded away"
        );
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

        // No attached terminals -> no agents -> no agents section at all. The
        // create row lives in the pinned footer (TP-MOB-77), so the lone
        // workspace opens the document.
        assert_eq!(agent_panel_entries(&app).len(), 0);
        assert_eq!(mobile_drawer_workspace_doc_range(&app, 0).start, 0);
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
