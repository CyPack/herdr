use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use super::scrollbar::{render_scrollbar, should_show_scrollbar};
use super::status::{agent_icon, state_dot, state_label, state_label_color};
use super::text::{display_width, display_width_u16, truncate_end};
use super::widgets::panel_contrast_fg;
use crate::app::state::{
    AgentPanelSort, FileManagerSidebarItem, FileManagerSidebarRowArea, Palette, ProjectRowArea,
    ProjectRowKind,
};
use crate::app::{AppState, Mode};
use crate::detect::AgentState;
use crate::terminal::TerminalRuntimeRegistry;

const WORKSPACE_SECTION_HEADER_ROWS: u16 = 2;
const AGENT_PANEL_HEADER_ROWS: u16 = 3;

pub(crate) struct AgentPanelEntry {
    pub ws_idx: usize,
    pub tab_idx: usize,
    pub pane_id: crate::layout::PaneId,
    pub primary_label: String,
    pub primary_tab_label: Option<String>,
    pub agent_label: Option<String>,
    pub state: AgentState,
    pub seen: bool,
    pub last_agent_state_change_seq: Option<u64>,
    pub custom_status: Option<String>,
    pub state_labels: std::collections::HashMap<String, String>,
}

fn sidebar_section_heights(total_h: u16, split_ratio: f32) -> (u16, u16) {
    if total_h == 0 {
        return (0, 0);
    }

    if total_h < 6 {
        let ws_h = total_h.div_ceil(2);
        return (ws_h, total_h.saturating_sub(ws_h));
    }

    let ratio = split_ratio.clamp(0.1, 0.9);
    let ws_h = ((total_h as f32) * ratio).round() as u16;
    let ws_h = ws_h.clamp(3, total_h.saturating_sub(3));
    let detail_h = total_h.saturating_sub(ws_h);
    (ws_h, detail_h)
}

pub(crate) fn expanded_sidebar_sections(area: Rect, split_ratio: f32) -> (Rect, Rect) {
    let content = Rect::new(area.x, area.y, area.width.saturating_sub(1), area.height);
    if content.width == 0 || content.height == 0 {
        return (Rect::default(), Rect::default());
    }

    let (ws_h, detail_h) = sidebar_section_heights(content.height, split_ratio);
    let ws_area = Rect::new(content.x, content.y, content.width, ws_h);
    let detail_area = Rect::new(content.x, content.y + ws_h, content.width, detail_h);
    (ws_area, detail_area)
}

pub(crate) fn sidebar_section_divider_rect(area: Rect, split_ratio: f32) -> Rect {
    let content = Rect::new(area.x, area.y, area.width.saturating_sub(1), area.height);
    if content.width == 0 || content.height < 6 {
        return Rect::default();
    }

    let (ws_h, _) = sidebar_section_heights(content.height, split_ratio);
    Rect::new(content.x, content.y + ws_h, content.width, 1)
}

fn agent_panel_sort_label(sort: AgentPanelSort) -> &'static str {
    match sort {
        AgentPanelSort::Spaces => "grouped",
        AgentPanelSort::Priority => "priority",
    }
}

pub(crate) fn agent_panel_toggle_rect(area: Rect, sort: AgentPanelSort) -> Rect {
    if area.width == 0 || area.height < 2 {
        return Rect::default();
    }

    let label = agent_panel_sort_label(sort);
    let width = display_width_u16(label);
    Rect::new(
        area.x + area.width.saturating_sub(width),
        area.y + 1,
        width,
        1,
    )
}

pub(crate) fn agent_panel_entries(app: &AppState) -> Vec<AgentPanelEntry> {
    agent_panel_entries_with_runtimes(app, None)
}

pub(crate) fn agent_panel_entries_from(
    app: &AppState,
    terminal_runtimes: &TerminalRuntimeRegistry,
) -> Vec<AgentPanelEntry> {
    agent_panel_entries_with_runtimes(app, Some(terminal_runtimes))
}

fn agent_panel_entries_with_runtimes(
    app: &AppState,
    terminal_runtimes: Option<&TerminalRuntimeRegistry>,
) -> Vec<AgentPanelEntry> {
    let empty_runtimes;
    let terminal_runtimes = match terminal_runtimes {
        Some(terminal_runtimes) => terminal_runtimes,
        None => {
            empty_runtimes = TerminalRuntimeRegistry::new();
            &empty_runtimes
        }
    };

    let mut entries: Vec<_> = app
        .workspaces
        .iter()
        .enumerate()
        .flat_map(|(ws_idx, ws)| {
            let multi_tab = ws.tabs.len() > 1;
            let workspace_label = ws.display_name_from(&app.terminals, terminal_runtimes);
            ws.pane_details(&app.terminals)
                .into_iter()
                .map(move |detail| {
                    // A custom-named tab (project chats name themselves after
                    // their project) leads with its own label, paired with the
                    // git branch its terminal cwd is on (from the runtime's
                    // branch cache) — never the workspace name.
                    let (primary_label, primary_tab_label) =
                        if let Some(custom) = detail.tab_custom_label.clone() {
                            let branch = app
                                .tab_branch_cache
                                .get(&detail.terminal_cwd)
                                .and_then(|entry| entry.branch.clone());
                            (custom, branch)
                        } else {
                            (
                                workspace_label.clone(),
                                multi_tab.then_some(detail.tab_label),
                            )
                        };
                    AgentPanelEntry {
                        ws_idx,
                        tab_idx: detail.tab_idx,
                        pane_id: detail.pane_id,
                        primary_label,
                        primary_tab_label,
                        agent_label: Some(detail.agent_label),
                        state: detail.state,
                        seen: detail.seen,
                        last_agent_state_change_seq: detail.last_agent_state_change_seq,
                        custom_status: detail.custom_status,
                        state_labels: detail.state_labels,
                    }
                })
        })
        .collect();

    if matches!(app.agent_panel_sort, AgentPanelSort::Priority) {
        entries.sort_by_key(|entry| {
            (
                std::cmp::Reverse(workspace_attention_priority(entry.state, entry.seen)),
                std::cmp::Reverse(entry.last_agent_state_change_seq),
            )
        });
    }

    entries
}

pub(super) fn agent_panel_status_key(state: AgentState, seen: bool) -> &'static str {
    match (state, seen) {
        (AgentState::Idle, false) => "done",
        (AgentState::Idle, true) => "idle",
        (AgentState::Working, _) => "working",
        (AgentState::Blocked, _) => "blocked",
        (AgentState::Unknown, _) => "unknown",
    }
}

fn format_agent_panel_primary_label(entry: &AgentPanelEntry, max_width: usize) -> String {
    let Some(tab_label) = entry.primary_tab_label.as_deref() else {
        return truncate_end(&entry.primary_label, max_width);
    };

    let separator = " · ";
    let separator_width = display_width(separator);
    if max_width <= separator_width + 2 {
        return truncate_end(
            &format!("{}{}{}", entry.primary_label, separator, tab_label),
            max_width,
        );
    }

    let available = max_width.saturating_sub(separator_width);
    let min_tab = 4.min(available.saturating_sub(1)).max(1);
    let preferred_workspace = ((available * 2) / 3).max(1);
    let mut workspace_budget = preferred_workspace
        .min(available.saturating_sub(min_tab))
        .max(1);
    let mut tab_budget = available.saturating_sub(workspace_budget);

    let workspace_len = display_width(&entry.primary_label);
    let tab_len = display_width(tab_label);

    if workspace_len < workspace_budget {
        let spare = workspace_budget - workspace_len;
        workspace_budget = workspace_len;
        tab_budget = (tab_budget + spare).min(available.saturating_sub(workspace_budget));
    }
    if tab_len < tab_budget {
        let spare = tab_budget - tab_len;
        tab_budget = tab_len;
        workspace_budget = (workspace_budget + spare).min(available.saturating_sub(tab_budget));
    }

    format!(
        "{}{}{}",
        truncate_end(&entry.primary_label, workspace_budget),
        separator,
        truncate_end(tab_label, tab_budget)
    )
}

fn workspace_row_height(ws: &crate::workspace::Workspace) -> u16 {
    if ws.branch().is_some() {
        2
    } else {
        1
    }
}

fn workspace_attention_priority(state: AgentState, seen: bool) -> u8 {
    match (state, seen) {
        (AgentState::Blocked, _) => 4,
        (AgentState::Idle, false) => 3,
        (AgentState::Working, _) => 2,
        (AgentState::Idle, true) => 1,
        (AgentState::Unknown, _) => 0,
    }
}

fn space_aggregate_state(app: &AppState, key: &str) -> (AgentState, bool) {
    app.workspaces
        .iter()
        .filter(|ws| ws.worktree_space().is_some_and(|space| space.key == key))
        .map(|ws| ws.aggregate_state(&app.terminals))
        .max_by_key(|(state, seen)| workspace_attention_priority(*state, *seen))
        .unwrap_or((AgentState::Unknown, true))
}

pub(crate) fn workspace_parent_group_state(
    app: &AppState,
    ws_idx: usize,
) -> Option<(String, bool)> {
    let space = app.workspaces.get(ws_idx)?.worktree_space()?;
    if space.is_linked_worktree {
        return None;
    }
    let member_count = app
        .workspaces
        .iter()
        .filter(|ws| {
            ws.worktree_space()
                .is_some_and(|member| member.key == space.key)
        })
        .count();
    (member_count >= 2).then(|| {
        (
            space.key.clone(),
            app.collapsed_space_keys.contains(&space.key),
        )
    })
}

pub(crate) fn grouped_child_display_label(
    label: &str,
    branch: Option<&str>,
    has_custom_name: bool,
) -> String {
    if has_custom_name {
        return label.to_string();
    }
    let Some(branch) = branch else {
        return label.to_string();
    };
    branch
        .strip_prefix("worktree/")
        .unwrap_or(branch)
        .to_string()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WorkspaceListEntry {
    Workspace { ws_idx: usize, indented: bool },
}

pub(crate) fn next_entry_is_indented_workspace(entries: &[WorkspaceListEntry], idx: usize) -> bool {
    matches!(
        entries.get(idx.saturating_add(1)),
        Some(WorkspaceListEntry::Workspace { indented: true, .. })
    )
}

pub(crate) fn normalized_workspace_scroll(app: &AppState, area: Rect, requested: usize) -> usize {
    let ws_area = workspace_list_rect(area, app.sidebar_section_split);
    let body = workspace_list_body_rect(ws_area, false);
    if body.height == 0 {
        return requested;
    }

    let entry_count = workspace_list_entries(app).len();
    if entry_count == 0 {
        0
    } else {
        requested.min(entry_count.saturating_sub(1))
    }
}

pub(crate) fn workspace_list_entries(app: &AppState) -> Vec<WorkspaceListEntry> {
    workspace_list_entries_inner(app, false)
}

/// Like [`workspace_list_entries`] but always expands worktree groups, ignoring
/// `collapsed_space_keys`. The mobile switcher has no collapse affordance and
/// always shows the full worktree tree.
pub(crate) fn workspace_list_entries_expanded(app: &AppState) -> Vec<WorkspaceListEntry> {
    workspace_list_entries_inner(app, true)
}

fn workspace_list_entries_inner(app: &AppState, force_expanded: bool) -> Vec<WorkspaceListEntry> {
    let mut members_by_key = std::collections::HashMap::<String, Vec<usize>>::new();
    for (ws_idx, ws) in app.workspaces.iter().enumerate() {
        if let Some(space) = ws.worktree_space() {
            members_by_key
                .entry(space.key.clone())
                .or_default()
                .push(ws_idx);
        }
    }
    let grouped_keys = members_by_key
        .iter()
        .filter(|(_, members)| {
            members.len() >= 2
                && members.iter().any(|idx| {
                    app.workspaces
                        .get(*idx)
                        .and_then(|ws| ws.worktree_space())
                        .is_some_and(|space| !space.is_linked_worktree)
                })
        })
        .map(|(key, _)| key.clone())
        .collect::<std::collections::HashSet<_>>();

    let visible_group_idx = if matches!(app.mode, Mode::Navigate) {
        Some(app.selected)
    } else {
        app.active
    };
    let active_group = visible_group_idx.and_then(|idx| {
        app.workspaces
            .get(idx)
            .and_then(|ws| ws.worktree_space())
            .map(|space| space.key.clone())
    });

    let mut emitted_groups = std::collections::HashSet::<String>::new();
    let mut entries = Vec::new();
    for (ws_idx, ws) in app.workspaces.iter().enumerate() {
        let Some(space) = ws
            .worktree_space()
            .filter(|space| grouped_keys.contains(&space.key))
        else {
            entries.push(WorkspaceListEntry::Workspace {
                ws_idx,
                indented: false,
            });
            continue;
        };

        if !emitted_groups.insert(space.key.clone()) {
            continue;
        }

        let Some(members) = members_by_key.get(&space.key) else {
            continue;
        };
        let Some(parent_idx) = members.iter().copied().find(|idx| {
            app.workspaces
                .get(*idx)
                .and_then(|member| member.worktree_space())
                .is_some_and(|member_space| !member_space.is_linked_worktree)
        }) else {
            entries.push(WorkspaceListEntry::Workspace {
                ws_idx,
                indented: false,
            });
            continue;
        };
        let collapsed = !force_expanded && app.collapsed_space_keys.contains(&space.key);
        entries.push(WorkspaceListEntry::Workspace {
            ws_idx: parent_idx,
            indented: false,
        });

        if collapsed {
            if let Some(active_idx) = visible_group_idx
                .filter(|idx| *idx != parent_idx)
                .filter(|_| active_group.as_deref() == Some(space.key.as_str()))
            {
                entries.push(WorkspaceListEntry::Workspace {
                    ws_idx: active_idx,
                    indented: true,
                });
            }
        } else {
            for member_idx in members {
                if *member_idx == parent_idx {
                    continue;
                }
                entries.push(WorkspaceListEntry::Workspace {
                    ws_idx: *member_idx,
                    indented: true,
                });
            }
        }
    }
    entries
}

pub(crate) fn workspace_list_rect(area: Rect, split_ratio: f32) -> Rect {
    let (ws_area, _) = expanded_sidebar_sections(area, split_ratio);
    ws_area
}

/// Lay out the Spaces/Projects/Files header tabs across the top row of the
/// sidebar's workspace section. Returns one rect per `SidebarTab::ALL` entry,
/// in order: the tabs share the row width left-to-right, and any remainder goes
/// to the last tab. A row too narrow for every tab yields zero-width trailing
/// rects (rendering skips those) instead of panicking; a zero-size area yields
/// all-default rects.
pub(crate) fn compute_sidebar_tab_areas(ws_area: Rect) -> Vec<Rect> {
    let tab_count = crate::app::state::SidebarTab::ALL.len();
    let mut rects = vec![Rect::default(); tab_count];
    if ws_area.width == 0 || ws_area.height == 0 {
        return rects;
    }

    let row_y = ws_area.y;
    let right = ws_area.x + ws_area.width;
    let mut x = ws_area.x;
    for (i, rect) in rects.iter_mut().enumerate() {
        if x >= right {
            break;
        }
        let remaining_tabs = (tab_count - i) as u16;
        let remaining_width = right - x;
        let width = (remaining_width / remaining_tabs)
            .max(1)
            .min(remaining_width);
        *rect = Rect::new(x, row_y, width, 1);
        x = x.saturating_add(width);
    }
    rects
}

pub(crate) fn workspace_list_body_rect(area: Rect, has_scrollbar: bool) -> Rect {
    if area.width == 0 || area.height <= WORKSPACE_SECTION_HEADER_ROWS {
        return Rect::default();
    }

    let body_y = area.y.saturating_add(WORKSPACE_SECTION_HEADER_ROWS);
    let footer_y = area.y + area.height.saturating_sub(1);
    let body_height = footer_y.saturating_sub(body_y);
    let body_width = area.width.saturating_sub(u16::from(has_scrollbar));
    Rect::new(area.x, body_y, body_width, body_height)
}

enum FileManagerSidebarLine<'a> {
    Header(&'a str),
    Blank,
    Item(&'a FileManagerSidebarItem),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FileManagerSidebarMarker {
    Warning,
    Eject,
}

fn file_manager_sidebar_marker(item: &FileManagerSidebarItem) -> Option<FileManagerSidebarMarker> {
    if !item.accessible {
        Some(FileManagerSidebarMarker::Warning)
    } else if item.ejectable {
        Some(FileManagerSidebarMarker::Eject)
    } else {
        None
    }
}

fn file_manager_sidebar_item_is_current(app: &AppState, item: &FileManagerSidebarItem) -> bool {
    app.sidebar_tab == crate::app::state::SidebarTab::Files
        && item.accessible
        && app
            .file_manager
            .as_ref()
            .is_some_and(|file_manager| file_manager.cwd.as_path() == item.path.as_path())
}

fn file_manager_sidebar_item_line(
    app: &AppState,
    item: &FileManagerSidebarItem,
    width: u16,
) -> Line<'static> {
    let width = usize::from(width);
    if width == 0 {
        return Line::default();
    }

    let marker = file_manager_sidebar_marker(item);
    let marker_width = usize::from(marker.is_some());
    let available_left = width.saturating_sub(marker_width);
    // Keep a blank cell between row content and a trailing affordance whenever
    // the row is wide enough to show both.
    let content_limit =
        available_left.saturating_sub(usize::from(marker.is_some() && available_left > 0));
    let mut spans = Vec::new();
    let mut content_width = 0;

    if file_manager_sidebar_item_is_current(app, item) && content_limit >= 4 {
        let body_width = content_limit.saturating_sub(3);
        let full_body = format!("{} {}", item.icon.glyph(), item.label);
        let body = if body_width == 1 {
            truncate_end(item.icon.glyph(), body_width)
        } else {
            truncate_end(&full_body, body_width)
        };
        content_width = 1 + 1 + display_width(&body) + 1;
        spans.extend([
            Span::raw(" "),
            Span::styled("", Style::default().fg(app.palette.accent)),
            Span::styled(
                body,
                Style::default()
                    .fg(panel_contrast_fg(&app.palette))
                    .bg(app.palette.accent),
            ),
            Span::styled("", Style::default().fg(app.palette.accent)),
        ]);
    } else if content_limit > 0 {
        let prefix_width = content_limit.min(2);
        spans.push(Span::raw(" ".repeat(prefix_width)));
        content_width = prefix_width;

        let icon_limit = content_limit.saturating_sub(content_width);
        if icon_limit > 0 {
            let icon = truncate_end(item.icon.glyph(), icon_limit);
            content_width += display_width(&icon);
            spans.push(Span::styled(
                icon,
                Style::default().fg(app.palette.overlay1),
            ));
        }

        if content_width < content_limit {
            spans.push(Span::raw(" "));
            content_width += 1;
        }

        let label_limit = content_limit.saturating_sub(content_width);
        if label_limit > 0 {
            let label = truncate_end(&item.label, label_limit);
            content_width += display_width(&label);
            let style = if item.accessible {
                Style::default().fg(app.palette.subtext0)
            } else {
                Style::default().fg(app.palette.overlay0)
            };
            spans.push(Span::styled(label, style));
        }
    }

    let padding = available_left.saturating_sub(content_width);
    if padding > 0 {
        spans.push(Span::raw(" ".repeat(padding)));
    }
    if let Some(marker) = marker {
        let (symbol, color) = match marker {
            FileManagerSidebarMarker::Warning => ("⚠", app.palette.yellow),
            FileManagerSidebarMarker::Eject => ("⏏", app.palette.blue),
        };
        spans.push(Span::styled(symbol, Style::default().fg(color)));
    }

    Line::from(spans)
}

fn file_manager_sidebar_lines(app: &AppState) -> Vec<FileManagerSidebarLine<'_>> {
    let mut lines = Vec::new();
    for (section_idx, section) in app.file_manager_sidebar.sections.iter().enumerate() {
        if section_idx > 0 {
            lines.push(FileManagerSidebarLine::Blank);
        }
        lines.push(FileManagerSidebarLine::Header(section.kind.label()));
        lines.extend(section.items.iter().map(FileManagerSidebarLine::Item));
    }
    lines
}

/// Lay out only complete, clickable Files-sidebar item rows. Headers and blank
/// separators consume vertical space but intentionally carry no path identity.
pub(crate) fn compute_file_manager_sidebar_row_areas(
    app: &AppState,
    area: Rect,
) -> Vec<FileManagerSidebarRowArea> {
    if app.sidebar_tab != crate::app::state::SidebarTab::Files {
        return Vec::new();
    }
    let body = workspace_list_body_rect(area, false);
    if body.width == 0 || body.height == 0 {
        return Vec::new();
    }
    let bottom = body.y.saturating_add(body.height);
    let mut rows = Vec::new();
    for (line_idx, line) in file_manager_sidebar_lines(app).into_iter().enumerate() {
        let y = body
            .y
            .saturating_add(u16::try_from(line_idx).unwrap_or(u16::MAX));
        if y >= bottom {
            break;
        }
        if let FileManagerSidebarLine::Item(item) = line {
            rows.push(FileManagerSidebarRowArea {
                rect: Rect::new(body.x, y, body.width, 1),
                path: item.path.clone(),
            });
        }
    }
    rows
}

fn render_file_manager_sidebar(app: &AppState, frame: &mut Frame, area: Rect) {
    let body = workspace_list_body_rect(area, false);
    if body.width == 0 || body.height == 0 {
        return;
    }
    let bottom = body.y.saturating_add(body.height);
    for (line_idx, line) in file_manager_sidebar_lines(app).into_iter().enumerate() {
        let y = body
            .y
            .saturating_add(u16::try_from(line_idx).unwrap_or(u16::MAX));
        if y >= bottom {
            break;
        }
        let row = Rect::new(body.x, y, body.width, 1);
        match line {
            FileManagerSidebarLine::Header(label) => frame.render_widget(
                Paragraph::new(Span::styled(
                    format!(" {label}"),
                    Style::default()
                        .fg(app.palette.overlay0)
                        .add_modifier(Modifier::BOLD),
                )),
                row,
            ),
            FileManagerSidebarLine::Blank => {}
            FileManagerSidebarLine::Item(item) => frame.render_widget(
                Paragraph::new(file_manager_sidebar_item_line(app, item, row.width)),
                row,
            ),
        }
    }
}

fn workspace_list_visible_count(app: &AppState, area: Rect, scroll: usize) -> usize {
    let body = workspace_list_body_rect(area, false);
    if body.width == 0 || body.height == 0 {
        return 0;
    }

    let mut used_rows = 0u16;
    let mut visible = 0usize;
    let entries = workspace_list_entries(app);
    for (entry_idx, entry) in entries.iter().enumerate().skip(scroll) {
        let needed = match entry {
            WorkspaceListEntry::Workspace { ws_idx, indented } => {
                let Some(ws) = app.workspaces.get(*ws_idx) else {
                    continue;
                };
                let row_height = if *indented {
                    1
                } else {
                    workspace_row_height(ws)
                };
                let gap = u16::from(
                    !(*indented && next_entry_is_indented_workspace(&entries, entry_idx)),
                );
                row_height.saturating_add(gap)
            }
        };
        if used_rows.saturating_add(needed) > body.height {
            break;
        }
        used_rows = used_rows.saturating_add(needed);
        visible += 1;
    }
    visible
}

pub(crate) fn workspace_list_scroll_metrics(
    app: &AppState,
    area: Rect,
) -> crate::pane::ScrollMetrics {
    let entries = workspace_list_entries(app);
    let total_rows = entries.len();
    let scroll = app.workspace_scroll.min(total_rows.saturating_sub(1));
    let viewport_rows = workspace_list_visible_count(app, area, scroll);
    let max_offset_from_bottom = total_rows.saturating_sub(viewport_rows);
    let offset_from_bottom = total_rows
        .saturating_sub(scroll)
        .saturating_sub(viewport_rows);

    crate::pane::ScrollMetrics {
        offset_from_bottom,
        max_offset_from_bottom,
        viewport_rows,
    }
}

pub(crate) fn workspace_list_scrollbar_rect(app: &AppState, area: Rect) -> Option<Rect> {
    let metrics = workspace_list_scroll_metrics(app, area);
    let body = workspace_list_body_rect(area, true);
    (should_show_scrollbar(metrics) && body.width > 0 && body.height > 0).then_some(Rect::new(
        area.x + area.width.saturating_sub(1),
        body.y,
        1,
        body.height,
    ))
}

pub(crate) fn agent_panel_body_rect(area: Rect, has_scrollbar: bool) -> Rect {
    if area.width == 0 || area.height <= AGENT_PANEL_HEADER_ROWS {
        return Rect::default();
    }

    let body_y = area.y.saturating_add(AGENT_PANEL_HEADER_ROWS);
    let body_height = (area.y + area.height).saturating_sub(body_y);
    let body_width = area.width.saturating_sub(u16::from(has_scrollbar));
    Rect::new(area.x, body_y, body_width, body_height)
}

fn agent_panel_visible_count(area: Rect) -> usize {
    let body = agent_panel_body_rect(area, false);
    if body.width == 0 || body.height < 2 {
        return 0;
    }

    let mut used_rows = 0u16;
    let mut visible = 0usize;
    while used_rows.saturating_add(2) <= body.height {
        used_rows = used_rows.saturating_add(2);
        visible += 1;
        if used_rows < body.height {
            used_rows = used_rows.saturating_add(1);
        }
    }
    visible
}

pub(crate) fn agent_panel_scroll_metrics(app: &AppState, area: Rect) -> crate::pane::ScrollMetrics {
    let viewport_rows = agent_panel_visible_count(area);
    let total_rows = agent_panel_entries(app).len();
    let max_offset_from_bottom = total_rows.saturating_sub(viewport_rows);
    let offset_from_bottom = total_rows
        .saturating_sub(app.agent_panel_scroll)
        .saturating_sub(viewport_rows);

    crate::pane::ScrollMetrics {
        offset_from_bottom,
        max_offset_from_bottom,
        viewport_rows,
    }
}

pub(crate) fn agent_panel_scrollbar_rect(app: &AppState, area: Rect) -> Option<Rect> {
    let metrics = agent_panel_scroll_metrics(app, area);
    let body = agent_panel_body_rect(area, true);
    (should_show_scrollbar(metrics) && body.width > 0 && body.height > 0).then_some(Rect::new(
        area.x + area.width.saturating_sub(1),
        body.y,
        1,
        body.height,
    ))
}

pub(crate) fn compute_workspace_list_areas(
    app: &AppState,
    area: Rect,
) -> (Vec<crate::app::state::WorkspaceCardArea>, Vec<()>) {
    let ws_area = workspace_list_rect(area, app.sidebar_section_split);
    if ws_area == Rect::default() {
        return (Vec::new(), Vec::new());
    }

    let metrics = workspace_list_scroll_metrics(app, ws_area);
    let body = workspace_list_body_rect(ws_area, should_show_scrollbar(metrics));
    if body.width == 0 || body.height == 0 {
        return (Vec::new(), Vec::new());
    }

    let scroll = app.workspace_scroll;
    let mut row_y = body.y;
    let body_bottom = body.y + body.height;
    let mut cards = Vec::new();
    let headers = Vec::new();

    let entries = workspace_list_entries(app);
    for (entry_idx, entry) in entries.iter().enumerate().skip(scroll) {
        match entry {
            WorkspaceListEntry::Workspace { ws_idx, indented } => {
                let Some(ws) = app.workspaces.get(*ws_idx) else {
                    continue;
                };
                let row_height = if *indented {
                    1
                } else {
                    workspace_row_height(ws)
                };
                let gap = u16::from(
                    !(*indented && next_entry_is_indented_workspace(&entries, entry_idx)),
                );
                if row_y.saturating_add(row_height).saturating_add(gap) > body_bottom {
                    break;
                }
                cards.push(crate::app::state::WorkspaceCardArea {
                    ws_idx: *ws_idx,
                    rect: Rect::new(body.x, row_y, body.width, row_height),
                    indented: *indented,
                });
                row_y = row_y.saturating_add(row_height + gap);
            }
        }
    }

    (cards, headers)
}

pub(crate) fn compute_workspace_card_areas(
    app: &AppState,
    area: Rect,
) -> Vec<crate::app::state::WorkspaceCardArea> {
    compute_workspace_list_areas(app, area).0
}

/// Auto-scale sidebar width based on workspace identity + agent summary.
pub(crate) fn collapsed_sidebar_sections(area: Rect) -> (Rect, Option<u16>, Rect) {
    let content = Rect::new(area.x, area.y, area.width.saturating_sub(1), area.height);
    if content.width == 0 || content.height == 0 {
        return (Rect::default(), None, Rect::default());
    }

    if content.height < 7 {
        return (content, None, Rect::default());
    }

    let total_h = content.height as usize;
    let ws_h = total_h.div_ceil(2);
    let detail_h = total_h.saturating_sub(ws_h + 1);
    if ws_h == 0 || detail_h == 0 {
        return (content, None, Rect::default());
    }

    let divider_y = content.y + ws_h as u16;
    let ws_area = Rect::new(content.x, content.y, content.width, ws_h as u16);
    let detail_area = Rect::new(content.x, divider_y + 1, content.width, detail_h as u16);
    (ws_area, Some(divider_y), detail_area)
}

/// Collapsed sidebar: workspace glance on top, compact agent list below.
pub(super) fn render_sidebar_collapsed(app: &AppState, frame: &mut Frame, area: Rect) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let is_navigating = matches!(app.mode, Mode::Navigate);

    let p = &app.palette;
    let sep_style = if is_navigating {
        Style::default().fg(p.accent)
    } else {
        Style::default().fg(p.surface_dim)
    };
    let sep_x = area.x + area.width.saturating_sub(1);
    let buf = frame.buffer_mut();
    for y in area.y..area.y + area.height {
        buf[(sep_x, y)].set_symbol("│");
        buf[(sep_x, y)].set_style(sep_style);
    }

    let (ws_area, divider_y, detail_area) = collapsed_sidebar_sections(area);
    if ws_area == Rect::default() {
        render_sidebar_toggle(app, frame, area, true, p);
        return;
    }

    for (visible_idx, ws) in app.workspaces.iter().enumerate() {
        let y = ws_area.y + visible_idx as u16;
        if y >= ws_area.y + ws_area.height {
            break;
        }
        let (agg_state, agg_seen) = ws.aggregate_state(&app.terminals);
        let (icon, icon_style) = state_dot(agg_state, agg_seen, p);
        let is_selected = visible_idx == app.selected && is_navigating;
        let is_active = Some(visible_idx) == app.active;
        let row_style = if is_selected {
            Style::default().bg(p.surface0)
        } else if is_active {
            Style::default().bg(p.surface_dim)
        } else {
            Style::default()
        };
        let num_style = if is_selected {
            Style::default().fg(p.overlay1).bg(p.surface0)
        } else if is_active {
            Style::default().fg(p.text).bg(p.surface_dim)
        } else {
            Style::default().fg(p.overlay0)
        };

        if is_selected || is_active {
            let buf = frame.buffer_mut();
            for x in ws_area.x..ws_area.x + ws_area.width {
                buf[(x, y)].set_style(row_style);
            }
        }

        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(format!("{}", visible_idx + 1), num_style),
                Span::styled(" ", row_style),
                Span::styled(icon, icon_style),
            ])),
            Rect::new(ws_area.x, y, ws_area.width, 1),
        );
    }

    if let Some(divider_y) = divider_y {
        let buf = frame.buffer_mut();
        for x in ws_area.x..ws_area.x + ws_area.width {
            buf[(x, divider_y)].set_symbol("─");
            buf[(x, divider_y)].set_style(Style::default().fg(p.surface_dim));
        }
    }

    let detail_content_area = Rect::new(
        detail_area.x,
        detail_area.y,
        detail_area.width,
        detail_area.height.saturating_sub(1),
    );
    if detail_content_area != Rect::default() {
        for (detail_idx, detail) in agent_panel_entries(app).iter().enumerate() {
            let y = detail_content_area.y + detail_idx as u16;
            if y >= detail_content_area.y + detail_content_area.height {
                break;
            }
            let pane_num = app
                .workspaces
                .get(detail.ws_idx)
                .and_then(|ws| ws.public_pane_number(detail.pane_id))
                .unwrap_or(detail_idx + 1);
            let pane_style = Style::default().fg(p.overlay0);
            let (icon, icon_style) = agent_icon(detail.state, detail.seen, app.spinner_tick, p);
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled(format!("{pane_num}"), pane_style),
                    Span::styled(" ", pane_style),
                    Span::styled(icon, icon_style),
                ])),
                Rect::new(detail_content_area.x, y, detail_content_area.width, 1),
            );
        }
    }

    render_sidebar_toggle(app, frame, area, true, p);
}

pub(crate) fn workspace_drop_indicator_row(
    cards: &[crate::app::state::WorkspaceCardArea],
    area: Rect,
    insert_idx: usize,
) -> Option<u16> {
    if area.height == 0 {
        return None;
    }
    let list_bottom = area.y + area.height.saturating_sub(1);

    let first = cards.first()?;
    if insert_idx == first.ws_idx {
        return first.rect.y.checked_sub(1).filter(|y| *y < list_bottom);
    }

    if let Some(row) = cards
        .last()
        .filter(|card| insert_idx == card.ws_idx.saturating_add(1))
        .map(|card| card.rect.y.saturating_add(card.rect.height))
        .filter(|y| *y < list_bottom)
    {
        return Some(row);
    }

    if let Some(card) = cards.iter().find(|card| card.ws_idx == insert_idx) {
        return card.rect.y.checked_sub(1).filter(|y| *y < list_bottom);
    }

    None
}

pub(super) fn render_sidebar(
    app: &AppState,
    terminal_runtimes: &TerminalRuntimeRegistry,
    frame: &mut Frame,
    area: Rect,
) {
    let p = &app.palette;
    let is_navigating = matches!(app.mode, Mode::Navigate);
    let sep_style = if is_navigating {
        Style::default().fg(p.accent)
    } else {
        Style::default().fg(p.surface_dim)
    };

    let sep_x = area.x + area.width.saturating_sub(1);
    let buf = frame.buffer_mut();
    for y in area.y..area.y + area.height {
        buf[(sep_x, y)].set_symbol("│");
        buf[(sep_x, y)].set_style(sep_style);
    }

    let (ws_area, detail_area) = expanded_sidebar_sections(area, app.sidebar_section_split);

    render_workspace_list(app, terminal_runtimes, frame, ws_area, is_navigating);
    render_agent_detail(app, terminal_runtimes, frame, detail_area);
    render_sidebar_toggle(app, frame, area, false, p);
}

/// Render the Spaces/Projects/Files header tabs on the top row of the sidebar
/// workspace section. Reads `app.view.sidebar_tab_hit_areas` (computed in
/// `compute_view`) and highlights the active tab. Zero-width tabs (too-narrow
/// sidebar) are skipped.
fn render_sidebar_tabs(app: &AppState, frame: &mut Frame, ws_area: Rect) {
    if ws_area.width == 0 || ws_area.height == 0 {
        return;
    }
    let p = &app.palette;
    // Paint the header row background first so gaps between tabs stay clean.
    frame.render_widget(
        Paragraph::new(" ".repeat(ws_area.width as usize)).style(Style::default().bg(p.panel_bg)),
        Rect::new(ws_area.x, ws_area.y, ws_area.width, 1),
    );

    for (i, tab) in crate::app::state::SidebarTab::ALL.iter().enumerate() {
        let Some(rect) = app.view.sidebar_tab_hit_areas.get(i).copied() else {
            break;
        };
        if rect.width == 0 {
            continue;
        }
        let active = *tab == app.sidebar_tab;
        let style = if active {
            Style::default()
                .fg(panel_contrast_fg(p))
                .bg(p.accent)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(p.overlay1).bg(p.surface0)
        };
        let width = rect.width as usize;
        let label = tab.label();
        let text = if display_width(label) > width {
            truncate_end(label, width)
        } else {
            format!("{label:^width$}")
        };
        frame.render_widget(Paragraph::new(text).style(style), rect);
    }
}

fn render_workspace_list(
    app: &AppState,
    terminal_runtimes: &TerminalRuntimeRegistry,
    frame: &mut Frame,
    area: Rect,
    is_navigating: bool,
) {
    let p = &app.palette;
    let dragged_ws_idx = match app.drag.as_ref().map(|drag| &drag.target) {
        Some(crate::app::state::DragTarget::WorkspaceReorder { source_ws_idx, .. }) => {
            Some(*source_ws_idx)
        }
        _ => None,
    };
    let insertion_row = match app.drag.as_ref().map(|drag| &drag.target) {
        Some(crate::app::state::DragTarget::WorkspaceReorder {
            insert_idx: Some(insert_idx),
            ..
        }) => workspace_drop_indicator_row(&app.view.workspace_card_areas, area, *insert_idx),
        _ => None,
    };

    let list_bottom = area.y + area.height.saturating_sub(1);
    render_sidebar_tabs(app, frame, area);

    // Projects/Files own their content; the workspace list is the Spaces tab.
    match app.sidebar_tab {
        crate::app::state::SidebarTab::Spaces => {}
        crate::app::state::SidebarTab::Projects => {
            render_projects_list(app, frame, area);
            return;
        }
        crate::app::state::SidebarTab::Files => {
            render_file_manager_sidebar(app, frame, area);
            return;
        }
    }

    let metrics = workspace_list_scroll_metrics(app, area);
    let scrollbar_rect = workspace_list_scrollbar_rect(app, area);
    let cards = &app.view.workspace_card_areas;

    for card in cards {
        let i = card.ws_idx;
        let ws = &app.workspaces[i];
        let row_y = card.rect.y;
        let row_height = card.rect.height;
        let selected = i == app.selected && is_navigating;
        let is_active = Some(i) == app.active;
        let is_dragged = dragged_ws_idx == Some(i);
        let highlighted = selected || is_active || is_dragged;
        let (agg_state, agg_seen) = ws.aggregate_state(&app.terminals);

        if highlighted {
            let bg = if selected {
                p.surface0
            } else if is_dragged {
                p.surface1
            } else {
                p.surface_dim
            };
            let buf = frame.buffer_mut();
            for y in row_y..row_y + row_height {
                if y >= list_bottom {
                    break;
                }
                for x in card.rect.x..card.rect.x + card.rect.width {
                    buf[(x, y)].set_style(Style::default().bg(bg));
                }
            }
        }

        let name_style = if selected || is_active || is_dragged {
            Style::default().fg(p.text).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(p.subtext0)
        };

        let (icon, icon_style) = state_dot(agg_state, agg_seen, p);
        let label = ws.display_name_from(&app.terminals, terminal_runtimes);
        let mut line1 = Vec::new();
        let mut show_workspace_icon = true;
        if card.indented {
            line1.push(Span::styled("   ", Style::default()));
        } else if let Some((key, collapsed)) = workspace_parent_group_state(app, i) {
            let icon = if collapsed { "▸" } else { "▾" };
            let (state_icon, state_style) = if collapsed {
                let (state, seen) = space_aggregate_state(app, &key);
                state_dot(state, seen, p)
            } else {
                (icon, Style::default().fg(p.accent))
            };
            line1.push(Span::styled(icon, Style::default().fg(p.accent)));
            if collapsed {
                line1.push(Span::styled(" ", Style::default()));
                line1.push(Span::styled(state_icon, state_style));
                show_workspace_icon = false;
            }
            line1.push(Span::styled(" ", Style::default()));
        } else {
            line1.push(Span::styled(" ", Style::default()));
        }
        if show_workspace_icon {
            line1.push(Span::styled(icon, icon_style));
            line1.push(Span::styled(" ", Style::default()));
        }
        if card.indented {
            let display_label = grouped_child_display_label(
                &label,
                ws.branch().as_deref(),
                ws.custom_name.is_some(),
            );
            line1.push(Span::styled(display_label, name_style));
        } else {
            line1.push(Span::styled(label, name_style));
        }

        frame.render_widget(
            Paragraph::new(Line::from(line1)),
            Rect::new(card.rect.x, row_y, card.rect.width, 1),
        );

        if row_height > 1 && row_y + 1 < list_bottom {
            if let Some(branch) = ws.branch() {
                let upstream_label = ws.git_ahead_behind().and_then(|(ahead, behind)| {
                    let mut parts = Vec::new();
                    if ahead > 0 {
                        parts.push((format!("↑{}", ahead), p.green));
                    }
                    if behind > 0 {
                        parts.push((format!("↓{}", behind), p.red));
                    }
                    (!parts.is_empty()).then_some(parts)
                });
                let reserved = upstream_label
                    .as_ref()
                    .map(|parts| {
                        parts.iter().map(|(label, _)| label.len()).sum::<usize>() + parts.len()
                    })
                    .unwrap_or(0);
                let max_branch_len = (card.rect.width as usize).saturating_sub(5 + reserved);
                let branch_display = truncate_end(&branch, max_branch_len);
                let branch_color = if selected || is_active {
                    p.mauve
                } else {
                    p.overlay0
                };
                let branch_indent = if card.indented { "     " } else { "   " };
                let mut spans = vec![
                    Span::styled(branch_indent, Style::default()),
                    Span::styled(branch_display, Style::default().fg(branch_color)),
                ];
                if let Some(parts) = upstream_label {
                    spans.push(Span::styled(" ", Style::default()));
                    for (idx, (label, color)) in parts.into_iter().enumerate() {
                        if idx > 0 {
                            spans.push(Span::styled(" ", Style::default()));
                        }
                        spans.push(Span::styled(label, Style::default().fg(color)));
                    }
                }
                frame.render_widget(
                    Paragraph::new(Line::from(spans)),
                    Rect::new(card.rect.x, row_y + 1, card.rect.width, 1),
                );
            }
        }
    }

    if let Some(y) = insertion_row.filter(|y| *y < list_bottom) {
        let indicator_right = scrollbar_rect
            .map(|rect| rect.x)
            .unwrap_or(area.x + area.width);
        let buf = frame.buffer_mut();
        for x in area.x..indicator_right {
            buf[(x, y)].set_symbol("─");
            buf[(x, y)].set_style(Style::default().fg(p.accent));
        }
    }

    if let Some(track) = scrollbar_rect {
        render_scrollbar(frame, metrics, track, p.surface_dim, p.overlay0, "▕");
    }

    render_sidebar_footer_buttons(app, frame, area, " new");
}

/// Draw the shared sidebar footer: a left-aligned action button and the
/// right-aligned global "menu" launcher. Reused by both the Spaces and Projects
/// tabs so the footer chrome stays identical. `new_label` names the left button
/// (" new" workspace on Spaces, "new chat" on Projects). No-op when the mouse UI
/// is disabled or the area has no footer row.
fn render_sidebar_footer_buttons(app: &AppState, frame: &mut Frame, area: Rect, new_label: &str) {
    let p = &app.palette;
    let list_bottom = area.y + area.height.saturating_sub(1);
    if !(app.mouse_capture && list_bottom > area.y) {
        return;
    }

    let new_rect = app.sidebar_new_button_rect();
    frame.render_widget(
        Paragraph::new(Span::styled(new_label, Style::default().fg(p.overlay0))),
        new_rect,
    );

    let menu_rect = app.global_launcher_rect();
    let menu_line = if app.global_menu_attention_badge_visible() {
        Line::from(vec![
            Span::styled(
                "● ",
                Style::default().fg(p.accent).add_modifier(Modifier::BOLD),
            ),
            Span::styled("menu", Style::default().fg(p.overlay0)),
        ])
    } else {
        Line::from(vec![Span::styled("menu", Style::default().fg(p.overlay0))])
    };
    frame.render_widget(
        Paragraph::new(menu_line).alignment(Alignment::Right),
        menu_rect,
    );
}

/// Lay out the Projects-tab rows (geometry only) within `area` — the workspace
/// list section rect. Pinned projects render as collapse/expand headers; every
/// expanded project contributes one row per chat session, or a single "(no
/// chats)" row when it has none. Reads the `projects_sessions` cache only; never
/// touches the filesystem (that is `refresh_project_sessions*`'s job). Rows are
/// clipped to the body height (between the tab header and the footer button row).
/// Chats listed per expanded project; older ones fold into a "… N older" row.
pub(crate) const PROJECT_CHAT_ROW_LIMIT: usize = 5;

/// One logical Projects-tab row, before scroll and viewport clipping. A
/// project header counts as a single line even though it lays out as two
/// disjoint hit rects (name + " +" button), so scrolling can never split the
/// pair. The future Files tab reuses this lines→skip→layout scroll pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProjectRowLine {
    Header { proj_idx: usize },
    Empty { proj_idx: usize },
    Chat { proj_idx: usize, chat_idx: usize },
    More { proj_idx: usize },
}

/// The full logical row list for the Projects tab, unscrolled and unclipped —
/// the single source the scroll metrics and the layout both derive from.
pub(crate) fn project_row_lines(app: &AppState) -> Vec<ProjectRowLine> {
    let mut lines = Vec::new();
    for (proj_idx, project) in app.projects_sessions.iter().enumerate() {
        lines.push(ProjectRowLine::Header { proj_idx });
        if app.collapsed_project_paths.contains(&project.path) {
            continue;
        }
        if project.sessions.is_empty() {
            lines.push(ProjectRowLine::Empty { proj_idx });
        } else if app.projects_actives_only {
            // Actives mode: only chats currently open as tabs, with their
            // ORIGINAL session indices (clicks resume by index). No cap and
            // no "older" row — hidden chats are filtered, not folded.
            let before = lines.len();
            for chat_idx in 0..project.sessions.len() {
                if app
                    .find_resumed_chat_tab(&project.sessions[chat_idx].id)
                    .is_some()
                {
                    lines.push(ProjectRowLine::Chat { proj_idx, chat_idx });
                }
            }
            if lines.len() == before {
                lines.push(ProjectRowLine::Empty { proj_idx });
            }
        } else {
            let visible = project.sessions.len().min(PROJECT_CHAT_ROW_LIMIT);
            for chat_idx in 0..visible {
                lines.push(ProjectRowLine::Chat { proj_idx, chat_idx });
            }
            if project.total_count > PROJECT_CHAT_ROW_LIMIT {
                lines.push(ProjectRowLine::More { proj_idx });
            }
        }
    }
    lines
}

pub(crate) fn projects_scroll_metrics(app: &AppState, area: Rect) -> crate::pane::ScrollMetrics {
    let viewport_rows = workspace_list_body_rect(area, false).height as usize;
    let total_rows = project_row_lines(app).len();
    let max_offset_from_bottom = total_rows.saturating_sub(viewport_rows);
    let offset_from_bottom = total_rows
        .saturating_sub(app.projects_scroll)
        .saturating_sub(viewport_rows);

    crate::pane::ScrollMetrics {
        offset_from_bottom,
        max_offset_from_bottom,
        viewport_rows,
    }
}

pub(crate) fn projects_scrollbar_rect(app: &AppState, area: Rect) -> Option<Rect> {
    let metrics = projects_scroll_metrics(app, area);
    let body = workspace_list_body_rect(area, true);
    (should_show_scrollbar(metrics) && body.width > 0 && body.height > 0).then_some(Rect::new(
        area.x + area.width.saturating_sub(1),
        body.y,
        1,
        body.height,
    ))
}

/// Clamp a Projects scroll offset to the current list; the list length moves
/// underneath the offset via the session polls, so `compute_view` re-normalizes
/// every frame (same contract as `normalized_workspace_scroll`).
pub(crate) fn normalized_projects_scroll(app: &AppState, area: Rect, scroll: usize) -> usize {
    scroll.min(projects_scroll_metrics(app, area).max_offset_from_bottom)
}

pub(crate) fn compute_project_row_areas(app: &AppState, area: Rect) -> Vec<ProjectRowArea> {
    let has_scrollbar = should_show_scrollbar(projects_scroll_metrics(app, area));
    let body = workspace_list_body_rect(area, has_scrollbar);
    if body.width == 0 || body.height == 0 {
        return Vec::new();
    }
    let body_bottom = body.y + body.height;
    let mut areas: Vec<ProjectRowArea> = Vec::new();

    for (row_idx, line) in project_row_lines(app)
        .into_iter()
        .skip(app.projects_scroll)
        .enumerate()
    {
        let y = body
            .y
            .saturating_add(u16::try_from(row_idx).unwrap_or(u16::MAX));
        if y >= body_bottom {
            break;
        }
        match line {
            ProjectRowLine::Header { proj_idx } => {
                // The header row splits into the collapse/name area and a
                // fixed-width " +" new-chat button at the right edge. Disjoint
                // rects keep the hit-test unambiguous; the button is dropped on
                // very narrow sidebars so the header itself stays clickable.
                let button_w: u16 = if body.width >= 8 { 3 } else { 0 };
                areas.push(ProjectRowArea {
                    rect: Rect::new(body.x, y, body.width - button_w, 1),
                    kind: ProjectRowKind::Project { proj_idx },
                });
                if button_w > 0 {
                    areas.push(ProjectRowArea {
                        rect: Rect::new(body.x + body.width - button_w, y, button_w, 1),
                        kind: ProjectRowKind::NewChat { proj_idx },
                    });
                }
            }
            ProjectRowLine::Empty { proj_idx } => {
                areas.push(ProjectRowArea {
                    rect: Rect::new(body.x, y, body.width, 1),
                    kind: ProjectRowKind::Empty { proj_idx },
                });
            }
            ProjectRowLine::Chat { proj_idx, chat_idx } => {
                areas.push(ProjectRowArea {
                    rect: Rect::new(body.x, y, body.width, 1),
                    kind: ProjectRowKind::Chat { proj_idx, chat_idx },
                });
            }
            ProjectRowLine::More { proj_idx } => {
                areas.push(ProjectRowArea {
                    rect: Rect::new(body.x, y, body.width, 1),
                    kind: ProjectRowKind::More { proj_idx },
                });
            }
        }
    }
    areas
}

/// Pure render for the Projects tab. Draws the rows laid out by
/// [`compute_project_row_areas`] (stored in `app.view.project_row_areas`) and
/// the shared footer. Resolves every row's content from the `projects_sessions`
/// cache; never mutates state or reads the disk (CLAUDE.md render purity).
fn render_projects_list(app: &AppState, frame: &mut Frame, area: Rect) {
    let p = &app.palette;
    let now = std::time::SystemTime::now();

    for row in &app.view.project_row_areas {
        let rect = row.rect;
        if rect.width == 0 || rect.height == 0 {
            continue;
        }
        match row.kind {
            ProjectRowKind::Project { proj_idx } => {
                let Some(project) = app.projects_sessions.get(proj_idx) else {
                    continue;
                };
                let collapsed = app.collapsed_project_paths.contains(&project.path);
                let chevron = if collapsed { "▸" } else { "▾" };
                let name = project_display_name(&project.path);
                let name = truncate_end(&name, (rect.width as usize).saturating_sub(2));
                frame.render_widget(
                    Paragraph::new(Line::from(vec![
                        Span::styled(chevron, Style::default().fg(p.accent)),
                        Span::styled(" ", Style::default()),
                        Span::styled(
                            name,
                            Style::default().fg(p.subtext0).add_modifier(Modifier::BOLD),
                        ),
                    ])),
                    rect,
                );
            }
            ProjectRowKind::Chat { proj_idx, chat_idx } => {
                let Some(session) = app
                    .projects_sessions
                    .get(proj_idx)
                    .and_then(|project| project.sessions.get(chat_idx))
                else {
                    continue;
                };
                let width = rect.width as usize;
                let rel = format_relative_time(session.last_modified, now);
                let rel_width = display_width(&rel);
                // Wired-state marker in the 3-column indent, synced with the
                // tab bar: "▸" = this chat IS the focused tab, "●" = open in
                // another tab, spaces = not open. Plain-text markers keep the
                // state readable without color support (and testable).
                let wired = app.find_resumed_chat_tab(&session.id);
                let focused = wired.is_some_and(|(ws_idx, tab_idx)| {
                    app.active == Some(ws_idx)
                        && app
                            .workspaces
                            .get(ws_idx)
                            .is_some_and(|ws| ws.active_tab == tab_idx)
                });
                let indent = if focused {
                    " ▸ "
                } else if wired.is_some() {
                    " ● "
                } else {
                    "   "
                };
                // The marker glyphs are multi-byte but all render 3 cells wide.
                let indent_width = 3usize;
                let title_budget = width
                    .saturating_sub(indent_width)
                    .saturating_sub(rel_width + 1);
                let title = truncate_end(&session.title, title_budget);
                // The focused chat reads as the primary row; open chats keep
                // normal text; chats with no recorded turns stay dimmed.
                let (title_style, indent_style) = if focused {
                    (
                        Style::default().fg(p.accent).add_modifier(Modifier::BOLD),
                        Style::default().fg(p.accent),
                    )
                } else if wired.is_some() {
                    (Style::default().fg(p.text), Style::default().fg(p.accent))
                } else if session.msg_count == 0 {
                    (Style::default().fg(p.overlay0), Style::default())
                } else {
                    (Style::default().fg(p.text), Style::default())
                };
                frame.render_widget(
                    Paragraph::new(Line::from(vec![
                        Span::styled(indent, indent_style),
                        Span::styled(title, title_style),
                    ])),
                    rect,
                );
                if rel_width > 0 && rel_width < width {
                    frame.render_widget(
                        Paragraph::new(Span::styled(rel, Style::default().fg(p.overlay0)))
                            .alignment(Alignment::Right),
                        rect,
                    );
                }
            }
            ProjectRowKind::Empty { proj_idx } => {
                // In actives mode a project can have chats that are just not
                // open; "(no chats)" would be misleading there.
                let has_hidden_chats = app.projects_actives_only
                    && app
                        .projects_sessions
                        .get(proj_idx)
                        .is_some_and(|project| !project.sessions.is_empty());
                let label = if has_hidden_chats {
                    "   (no active chats)"
                } else {
                    "   (no chats)"
                };
                frame.render_widget(
                    Paragraph::new(Span::styled(label, Style::default().fg(p.overlay0))),
                    rect,
                );
            }
            ProjectRowKind::NewChat { .. } => {
                frame.render_widget(
                    Paragraph::new(Span::styled(
                        " +",
                        Style::default().fg(p.accent).add_modifier(Modifier::BOLD),
                    )),
                    rect,
                );
            }
            ProjectRowKind::More { proj_idx } => {
                let hidden = app
                    .projects_sessions
                    .get(proj_idx)
                    .map(|project| {
                        project
                            .sessions
                            .len()
                            .saturating_sub(PROJECT_CHAT_ROW_LIMIT)
                    })
                    .unwrap_or(0);
                frame.render_widget(
                    Paragraph::new(Span::styled(
                        format!("   … {hidden} older"),
                        Style::default().fg(p.overlay0),
                    )),
                    rect,
                );
            }
        }
    }

    if let Some(track) = projects_scrollbar_rect(app, area) {
        let metrics = projects_scroll_metrics(app, area);
        render_scrollbar(frame, metrics, track, p.surface_dim, p.overlay0, "▕");
    }

    render_sidebar_footer_buttons(app, frame, area, " chat");

    // Projects-only footer toggle between the shared chat/menu buttons:
    // highlighted while the actives filter is on, dimmed when off.
    if app.mouse_capture {
        let toggle = app.sidebar_actives_toggle_rect();
        if toggle.width > 0 {
            let style = if app.projects_actives_only {
                Style::default().fg(p.accent).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(p.overlay0)
            };
            frame.render_widget(Paragraph::new(Span::styled("actives", style)), toggle);
        }
    }
}

/// Short, human-friendly label for a pinned project: its final path component
/// (e.g. `herdr`), falling back to the full path when there is none.
fn project_display_name(path: &std::path::Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

/// Compact relative age of a chat session ("now", "5m", "3h", "2d", "4w").
/// Clock skew or a future mtime collapses to "now" (never panics).
fn format_relative_time(
    last_modified: std::time::SystemTime,
    now: std::time::SystemTime,
) -> String {
    let secs = now
        .duration_since(last_modified)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    if secs < 60 {
        "now".to_string()
    } else if secs < 3_600 {
        format!("{}m", secs / 60)
    } else if secs < 86_400 {
        format!("{}h", secs / 3_600)
    } else if secs < 604_800 {
        format!("{}d", secs / 86_400)
    } else {
        format!("{}w", secs / 604_800)
    }
}

fn render_agent_detail(
    app: &AppState,
    terminal_runtimes: &TerminalRuntimeRegistry,
    frame: &mut Frame,
    area: Rect,
) {
    let p = &app.palette;

    if area.height < 3 {
        return;
    }

    let sep_line = "─".repeat(area.width as usize);
    frame.render_widget(
        Paragraph::new(Span::styled(&sep_line, Style::default().fg(p.surface_dim))),
        Rect::new(area.x, area.y, area.width, 1),
    );

    frame.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            " agents",
            Style::default().fg(p.overlay0).add_modifier(Modifier::BOLD),
        )])),
        Rect::new(area.x, area.y + 1, area.width, 1),
    );
    let toggle_rect = agent_panel_toggle_rect(area, app.agent_panel_sort);
    if toggle_rect != Rect::default() {
        frame.render_widget(
            Paragraph::new(Span::styled(
                agent_panel_sort_label(app.agent_panel_sort),
                Style::default().fg(p.overlay0).add_modifier(Modifier::BOLD),
            ))
            .alignment(Alignment::Right),
            toggle_rect,
        );
    }

    let details = agent_panel_entries_from(app, terminal_runtimes);
    let metrics = agent_panel_scroll_metrics(app, area);
    let scrollbar_rect = agent_panel_scrollbar_rect(app, area);
    let body = agent_panel_body_rect(area, should_show_scrollbar(metrics));
    if body == Rect::default() {
        return;
    }

    let mut row_y = body.y;
    let body_bottom = body.y + body.height;
    for detail in details.iter().skip(app.agent_panel_scroll) {
        if row_y.saturating_add(1) >= body_bottom {
            break;
        }

        // Check if this agent entry corresponds to the active session
        let is_active = app.is_active_pane(detail.ws_idx, detail.tab_idx, detail.pane_id);

        let (icon, icon_style) = agent_icon(detail.state, detail.seen, app.spinner_tick, p);
        let label_color = state_label_color(detail.state, detail.seen, p);
        let label = detail
            .state_labels
            .get(agent_panel_status_key(detail.state, detail.seen))
            .map(String::as_str)
            .unwrap_or_else(|| state_label(detail.state, detail.seen));

        let row_style = if is_active {
            Style::default().bg(p.surface_dim)
        } else {
            Style::default()
        };

        let name_style = if is_active {
            Style::default().fg(p.text).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(p.subtext0).add_modifier(Modifier::BOLD)
        };
        let status_style = if is_active {
            Style::default().fg(label_color)
        } else {
            Style::default().fg(label_color).add_modifier(Modifier::DIM)
        };
        let agent_style = Style::default().fg(p.overlay0).add_modifier(Modifier::DIM);

        let primary_label =
            format_agent_panel_primary_label(detail, body.width.saturating_sub(3) as usize);
        let name_line = Line::from(vec![
            Span::styled(" ", Style::default()),
            Span::styled(icon, icon_style),
            Span::styled(" ", Style::default()),
            Span::styled(primary_label, name_style),
        ]);
        frame.render_widget(
            Paragraph::new(name_line).style(row_style),
            Rect::new(body.x, row_y, body.width, 1),
        );
        row_y += 1;

        let mut status_spans = vec![
            Span::styled("   ", Style::default()),
            Span::styled(label, status_style),
        ];
        if let Some(agent_label) = &detail.agent_label {
            status_spans.push(Span::styled(" · ", agent_style));
            status_spans.push(Span::styled(agent_label, agent_style));
        }
        if let Some(custom_status) = &detail.custom_status {
            status_spans.push(Span::styled(" · ", agent_style));
            status_spans.push(Span::styled(custom_status.clone(), agent_style));
        }
        frame.render_widget(
            Paragraph::new(Line::from(status_spans)).style(row_style),
            Rect::new(body.x, row_y, body.width, 1),
        );
        row_y += 1;

        if row_y < body_bottom {
            row_y += 1;
        }
    }

    if let Some(track) = scrollbar_rect {
        render_scrollbar(frame, metrics, track, p.surface_dim, p.overlay0, "▕");
    }
}

pub(crate) fn collapsed_sidebar_toggle_rect(area: Rect) -> Rect {
    let bottom_y = area.y + area.height.saturating_sub(1);
    let content_w = area.width.saturating_sub(1);
    if content_w == 0 || area.height == 0 {
        return Rect::default();
    }
    let x = area.x + content_w / 2;
    Rect::new(x, bottom_y, 1, 1)
}

pub(crate) fn expanded_sidebar_toggle_rect(area: Rect) -> Rect {
    if area.width <= 1 || area.height == 0 {
        return Rect::default();
    }
    Rect::new(
        area.x + area.width.saturating_sub(2),
        area.y + area.height.saturating_sub(1),
        1,
        1,
    )
}

fn render_sidebar_toggle(
    app: &AppState,
    frame: &mut Frame,
    area: Rect,
    collapsed: bool,
    p: &Palette,
) {
    let toggle_area = if collapsed {
        collapsed_sidebar_toggle_rect(area)
    } else {
        expanded_sidebar_toggle_rect(area)
    };
    if toggle_area == Rect::default() {
        return;
    }
    let icon = if collapsed { "»" } else { "«" };
    let icon_style = if collapsed && app.global_menu_attention_badge_visible() {
        Style::default().fg(p.accent).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(p.overlay0)
    };
    frame.render_widget(Paragraph::new(Span::styled(icon, icon_style)), toggle_area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{detect::Agent, workspace::Workspace};
    use ratatui::{backend::TestBackend, Terminal};

    #[test]
    fn render_sidebar_toggle_draws_expanded_collapse_icon() {
        let app = crate::app::state::AppState::test_new();
        let area = Rect::new(0, 0, 26, 20);
        let mut terminal =
            Terminal::new(TestBackend::new(26, 20)).expect("test terminal should initialize");

        terminal
            .draw(|frame| render_sidebar_toggle(&app, frame, area, false, &app.palette))
            .expect("sidebar toggle should render");

        let toggle = expanded_sidebar_toggle_rect(area);
        assert_eq!(
            terminal.backend().buffer()[(toggle.x, toggle.y)].symbol(),
            "«"
        );
    }

    #[test]
    fn expanded_sidebar_toggle_sits_inside_sidebar_content() {
        let area = Rect::new(0, 0, 26, 20);
        let toggle = expanded_sidebar_toggle_rect(area);

        assert_eq!(toggle.x, area.x + area.width - 2);
        assert_eq!(toggle.y, area.y + area.height - 1);
    }

    #[test]
    fn all_workspaces_agent_panel_entries_use_workspace_and_optional_tab_labels() {
        let mut app = crate::app::state::AppState::test_new();
        let first = Workspace::test_new("one");
        let first_pane = first.tabs[0].root_pane;
        let mut second = Workspace::test_new("two");
        let second_tab = second.test_add_tab(Some("logs"));
        let second_pane = second.tabs[second_tab].root_pane;

        app.workspaces = vec![first, second];
        app.ensure_test_terminals();
        let first_terminal_id = app.workspaces[0].tabs[0].panes[&first_pane]
            .attached_terminal_id
            .clone();
        app.terminals
            .get_mut(&first_terminal_id)
            .unwrap()
            .detected_agent = Some(Agent::Pi);
        let second_terminal_id = app.workspaces[1].tabs[second_tab].panes[&second_pane]
            .attached_terminal_id
            .clone();
        app.terminals
            .get_mut(&second_terminal_id)
            .unwrap()
            .detected_agent = Some(Agent::Claude);
        app.active = Some(0);
        app.selected = 0;

        let entries = agent_panel_entries(&app);
        assert_eq!(entries[0].primary_label, "one");
        assert!(entries[0].primary_tab_label.is_none());
        assert_eq!(entries[0].agent_label.as_deref(), Some("pi"));
        // The custom-named "logs" tab leads with its own label; the secondary
        // slot carries its git branch (none cached here), never the workspace
        // name (BUG-2c behavior).
        assert_eq!(entries[1].primary_label, "logs");
        assert!(entries[1].primary_tab_label.is_none());
        assert_eq!(entries[1].agent_label.as_deref(), Some("claude"));
    }

    // ---- BUG-2b: custom-named tabs lead with their own label (project chats) ----

    fn single_agent_workspace_app(
        ws_name: &str,
        tab_name: Option<&str>,
    ) -> (
        crate::app::state::AppState,
        usize,
        crate::terminal::TerminalId,
    ) {
        let mut app = crate::app::state::AppState::test_new();
        let mut ws = Workspace::test_new(ws_name);
        let tab_idx = match tab_name {
            Some(name) => ws.test_add_tab(Some(name)),
            None => ws.test_add_tab(None),
        };
        let pane = ws.tabs[tab_idx].root_pane;
        app.workspaces = vec![ws];
        app.ensure_test_terminals();
        let terminal_id = app.workspaces[0].tabs[tab_idx].panes[&pane]
            .attached_terminal_id
            .clone();
        app.terminals
            .get_mut(&terminal_id)
            .expect("test terminal should exist")
            .detected_agent = Some(Agent::Claude);
        app.active = Some(0);
        app.selected = 0;
        (app, tab_idx, terminal_id)
    }

    #[test]
    fn agent_panel_pairs_custom_tab_label_with_its_git_branch() {
        let (mut app, tab_idx, terminal_id) = single_agent_workspace_app("space", Some("herdr"));
        let cwd = std::path::PathBuf::from("/proj/herdr");
        app.terminals
            .get_mut(&terminal_id)
            .expect("test terminal should exist")
            .cwd = cwd.clone();
        app.tab_branch_cache.insert(
            cwd,
            crate::app::tab_branches::TabBranchEntry::test_with_branch(Some("master")),
        );

        let entries = agent_panel_entries(&app);
        let chat_entry = entries
            .iter()
            .find(|entry| entry.tab_idx == tab_idx)
            .expect("chat tab should be listed");

        assert_eq!(chat_entry.primary_label, "herdr");
        assert_eq!(chat_entry.primary_tab_label.as_deref(), Some("master"));
    }

    #[test]
    fn agent_panel_omits_secondary_label_when_no_branch_is_known() {
        let (app, tab_idx, _) = single_agent_workspace_app("space", Some("herdr"));

        let entries = agent_panel_entries(&app);
        let chat_entry = entries
            .iter()
            .find(|entry| entry.tab_idx == tab_idx)
            .expect("chat tab should be listed");

        assert_eq!(chat_entry.primary_label, "herdr");
        assert!(
            chat_entry.primary_tab_label.is_none(),
            "the workspace name must never leak into a custom-named row"
        );
    }

    #[test]
    fn agent_panel_keeps_workspace_label_for_auto_named_tabs() {
        let (app, tab_idx, _) = single_agent_workspace_app("space", None);

        let entries = agent_panel_entries(&app);
        let entry = entries
            .iter()
            .find(|entry| entry.tab_idx == tab_idx)
            .expect("auto-named tab should be listed");

        assert_eq!(entry.primary_label, "space");
        assert_eq!(entry.primary_tab_label.as_deref(), Some("2"));
    }

    #[test]
    fn priority_agent_panel_sort_uses_attention_then_space_order() {
        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![
            Workspace::test_new("one"),
            Workspace::test_new("two"),
            Workspace::test_new("three"),
            Workspace::test_new("four"),
        ];
        app.ensure_test_terminals();
        app.active = Some(0);
        app.selected = 0;
        app.agent_panel_sort = crate::app::state::AgentPanelSort::Priority;

        let set_state = |app: &mut crate::app::state::AppState, ws_idx: usize, state| {
            let pane = app.workspaces[ws_idx].tabs[0].root_pane;
            let terminal_id = app.workspaces[ws_idx].tabs[0].panes[&pane]
                .attached_terminal_id
                .clone();
            let terminal = app.terminals.get_mut(&terminal_id).unwrap();
            terminal.detected_agent = Some(Agent::Claude);
            terminal.state = state;
        };
        set_state(&mut app, 0, AgentState::Working);
        set_state(&mut app, 1, AgentState::Idle);
        set_state(&mut app, 2, AgentState::Working);
        set_state(&mut app, 3, AgentState::Blocked);

        let done_pane = app.workspaces[1].tabs[0].root_pane;
        app.workspaces[1].tabs[0]
            .panes
            .get_mut(&done_pane)
            .unwrap()
            .seen = false;

        let labels: Vec<String> = agent_panel_entries(&app)
            .into_iter()
            .map(|entry| entry.primary_label)
            .collect();

        assert_eq!(labels, ["four", "two", "one", "three"]);
    }

    #[test]
    fn collapsed_sidebar_uses_all_workspaces_agent_panel_order() {
        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![Workspace::test_new("one"), Workspace::test_new("two")];
        app.ensure_test_terminals();
        app.active = Some(0);
        app.selected = 0;
        app.agent_panel_sort = crate::app::state::AgentPanelSort::Priority;

        let set_state = |app: &mut crate::app::state::AppState, ws_idx: usize, state| {
            let pane = app.workspaces[ws_idx].tabs[0].root_pane;
            let terminal_id = app.workspaces[ws_idx].tabs[0].panes[&pane]
                .attached_terminal_id
                .clone();
            let terminal = app.terminals.get_mut(&terminal_id).unwrap();
            terminal.detected_agent = Some(Agent::Claude);
            terminal.state = state;
        };
        set_state(&mut app, 0, AgentState::Working);
        set_state(&mut app, 1, AgentState::Blocked);

        let area = Rect::new(0, 0, 5, 12);
        let (_, _, detail_area) = collapsed_sidebar_sections(area);
        let first_detail_y = detail_area.y;
        let mut terminal = Terminal::new(TestBackend::new(area.width, area.height))
            .expect("test terminal should initialize");

        terminal
            .draw(|frame| render_sidebar_collapsed(&app, frame, area))
            .expect("collapsed sidebar should render");

        let buffer = terminal.backend().buffer();
        assert_eq!(buffer[(detail_area.x + 2, first_detail_y)].symbol(), "◉");
        assert_eq!(
            buffer[(detail_area.x + 2, first_detail_y)].style().fg,
            Some(app.palette.red)
        );
    }

    // ---- Sidebar header tabs (Spaces | Projects | Files) — Task #3 ----

    #[test]
    fn sidebar_tab_defaults_to_spaces() {
        use crate::app::state::{AppState, SidebarTab};
        assert_eq!(SidebarTab::default(), SidebarTab::Spaces);
        assert_eq!(AppState::test_new().sidebar_tab, SidebarTab::Spaces);
    }

    #[test]
    fn compute_sidebar_tab_areas_lays_out_three_tabs_side_by_side() {
        let ws_area = Rect::new(0, 0, 24, 10);
        let rects = compute_sidebar_tab_areas(ws_area);
        assert_eq!(rects.len(), 3, "one rect per Spaces/Projects/Files");
        for r in &rects {
            assert!(
                r.width > 0,
                "each tab gets width on a 24-wide sidebar: {rects:?}"
            );
            assert_eq!(r.height, 1, "tabs live on a single header row");
            assert_eq!(r.y, ws_area.y, "tabs sit on the top row of the section");
        }
        // Contiguous, left-to-right, spanning the full width.
        assert_eq!(rects[0].x, ws_area.x);
        assert_eq!(rects[1].x, rects[0].x + rects[0].width);
        assert_eq!(rects[2].x, rects[1].x + rects[1].width);
        assert_eq!(rects[2].x + rects[2].width, ws_area.x + ws_area.width);
    }

    #[test]
    fn compute_sidebar_tab_areas_does_not_panic_on_tiny_or_empty_area() {
        for area in [
            Rect::new(0, 0, 0, 10),
            Rect::new(0, 0, 24, 0),
            Rect::new(0, 0, 2, 10), // too narrow for three tabs
            Rect::new(0, 0, 1, 1),
        ] {
            let rects = compute_sidebar_tab_areas(area);
            assert_eq!(rects.len(), 3, "always one slot per tab, area={area:?}");
            for r in &rects {
                assert!(
                    r.x + r.width <= area.x + area.width,
                    "rect {r:?} overflows area {area:?}"
                );
            }
        }
    }

    #[test]
    fn render_sidebar_tabs_shows_all_three_labels() {
        let mut app = crate::app::state::AppState::test_new();
        app.sidebar_tab = crate::app::state::SidebarTab::Projects;
        let ws_area = Rect::new(0, 0, 24, 10);
        app.view.sidebar_tab_hit_areas = compute_sidebar_tab_areas(ws_area);

        let mut terminal = Terminal::new(TestBackend::new(24, 10)).unwrap();
        terminal
            .draw(|frame| render_sidebar_tabs(&app, frame, ws_area))
            .unwrap();

        let row: String = (0..24)
            .map(|x| terminal.backend().buffer()[(x, 0)].symbol())
            .collect();
        assert!(row.contains("Spaces"), "row: {row:?}");
        assert!(row.contains("Projects"), "row: {row:?}");
        assert!(row.contains("Files"), "row: {row:?}");
    }

    #[test]
    fn render_sidebar_tabs_highlights_active_tab_only() {
        let mut app = crate::app::state::AppState::test_new();
        app.sidebar_tab = crate::app::state::SidebarTab::Projects;
        let ws_area = Rect::new(0, 0, 24, 10);
        app.view.sidebar_tab_hit_areas = compute_sidebar_tab_areas(ws_area);

        let mut terminal = Terminal::new(TestBackend::new(24, 10)).unwrap();
        terminal
            .draw(|frame| render_sidebar_tabs(&app, frame, ws_area))
            .unwrap();
        let buffer = terminal.backend().buffer();

        let accent = app.palette.accent;
        let rect_has_accent_bg = |rect: Rect| {
            (rect.x..rect.x + rect.width).any(|x| buffer[(x, rect.y)].style().bg == Some(accent))
        };
        let rects = &app.view.sidebar_tab_hit_areas;
        assert!(
            rect_has_accent_bg(rects[1]),
            "active Projects tab should have accent bg"
        );
        assert!(
            !rect_has_accent_bg(rects[0]),
            "inactive Spaces tab should not have accent bg"
        );
        assert!(
            !rect_has_accent_bg(rects[2]),
            "inactive Files tab should not have accent bg"
        );
    }

    fn file_sidebar_item(
        label: &str,
        path: &str,
        icon: crate::app::state::FileManagerSidebarIcon,
        accessible: bool,
        ejectable: bool,
    ) -> crate::app::state::FileManagerSidebarItem {
        crate::app::state::FileManagerSidebarItem {
            label: label.to_string(),
            path: std::path::PathBuf::from(path),
            icon,
            accessible,
            ejectable,
        }
    }

    // TP-C6.1-MODEL: source order is stable, optional PINNED disappears when
    // empty, and a path repeated across sections grants only the first row.
    #[test]
    fn file_sidebar_model_orders_sections_and_deduplicates_path_authority() {
        use crate::app::state::{
            FileManagerSidebarIcon, FileManagerSidebarModel, FileManagerSidebarSectionKind,
        };
        let model = FileManagerSidebarModel::from_sources(
            vec![
                file_sidebar_item("Home", "/home/a", FileManagerSidebarIcon::Home, true, false),
                file_sidebar_item(
                    "Downloads",
                    "/home/a/Downloads",
                    FileManagerSidebarIcon::Downloads,
                    true,
                    false,
                ),
            ],
            vec![
                file_sidebar_item(
                    "duplicate",
                    "/home/a",
                    FileManagerSidebarIcon::Pin,
                    true,
                    false,
                ),
                file_sidebar_item(
                    "Missing",
                    "/missing",
                    FileManagerSidebarIcon::Pin,
                    false,
                    false,
                ),
            ],
            vec![
                file_sidebar_item("Root", "/", FileManagerSidebarIcon::Disk, true, false),
                file_sidebar_item(
                    "USB",
                    "/media/usb",
                    FileManagerSidebarIcon::Disk,
                    true,
                    true,
                ),
            ],
        );

        assert_eq!(
            model
                .sections
                .iter()
                .map(|section| section.kind)
                .collect::<Vec<_>>(),
            [
                FileManagerSidebarSectionKind::Favorites,
                FileManagerSidebarSectionKind::Pinned,
                FileManagerSidebarSectionKind::Locations,
            ]
        );
        assert_eq!(model.sections[0].items.len(), 2);
        assert_eq!(
            model.sections[1]
                .items
                .iter()
                .map(|item| item.path.as_path())
                .collect::<Vec<_>>(),
            [std::path::Path::new("/missing")]
        );
        assert!(!model.sections[1].items[0].accessible);
        assert!(model.sections[2].items[1].ejectable);

        let without_pins = FileManagerSidebarModel::from_sources(
            vec![file_sidebar_item(
                "Home",
                "/home/a",
                FileManagerSidebarIcon::Home,
                true,
                false,
            )],
            Vec::new(),
            vec![file_sidebar_item(
                "Root",
                "/",
                FileManagerSidebarIcon::Disk,
                true,
                false,
            )],
        );
        assert_eq!(without_pins.sections.len(), 2);
        assert!(without_pins
            .sections
            .iter()
            .all(|section| section.kind != FileManagerSidebarSectionKind::Pinned));
    }

    // TP-C6.1-GEOMETRY: only complete visible item rows receive exact path
    // rectangles; headers/blanks are inert and height clipping is atomic.
    #[test]
    fn file_sidebar_geometry_addresses_items_only_and_clips_complete_rows() {
        use crate::app::state::{FileManagerSidebarIcon, FileManagerSidebarModel, SidebarTab};
        let mut app = crate::app::state::AppState::test_new();
        app.sidebar_tab = SidebarTab::Files;
        app.file_manager_sidebar = FileManagerSidebarModel::from_sources(
            vec![file_sidebar_item(
                "Home",
                "/home/a",
                FileManagerSidebarIcon::Home,
                true,
                false,
            )],
            vec![file_sidebar_item(
                "Pinned",
                "/work",
                FileManagerSidebarIcon::Pin,
                true,
                false,
            )],
            vec![file_sidebar_item(
                "Root",
                "/",
                FileManagerSidebarIcon::Disk,
                true,
                false,
            )],
        );
        let area = Rect::new(3, 4, 20, 9);
        let rows = compute_file_manager_sidebar_row_areas(&app, area);

        assert_eq!(rows.len(), 2, "third section item is clipped as one row");
        assert_eq!(rows[0].path, std::path::PathBuf::from("/home/a"));
        assert_eq!(rows[1].path, std::path::PathBuf::from("/work"));
        assert!(rows.iter().all(|row| row.rect.width == area.width));
        assert!(rows[0].rect.y > area.y, "section header has no hit area");
        assert!(rows[0].rect.y < rows[1].rect.y, "section gap stays inert");

        app.sidebar_tab = SidebarTab::Projects;
        assert!(compute_file_manager_sidebar_row_areas(&app, area).is_empty());

        app.sidebar_tab = SidebarTab::Files;
        assert!(compute_file_manager_sidebar_row_areas(&app, Rect::new(0, 0, 0, 2)).is_empty());
        app.sidebar_collapsed = true;
        crate::ui::compute_view(&mut app, Rect::new(0, 0, 106, 20));
        assert!(app.view.file_manager_sidebar_row_areas.is_empty());
    }

    // TP-C6.1-RENDER: Files renders the prepared section model and removes the
    // placeholder. Rendering consumes no filesystem or environment source.
    #[test]
    fn render_workspace_list_shows_native_file_sidebar_sections() {
        use crate::app::state::{FileManagerSidebarIcon, FileManagerSidebarModel};
        let mut app = crate::app::state::AppState::test_new();
        app.sidebar_tab = crate::app::state::SidebarTab::Files;
        app.mouse_capture = false; // skip new/menu chrome for a focused test
        app.file_manager_sidebar = FileManagerSidebarModel::from_sources(
            vec![file_sidebar_item(
                "Home",
                "/home/a",
                FileManagerSidebarIcon::Home,
                true,
                false,
            )],
            vec![file_sidebar_item(
                "Herdr",
                "/work/herdr",
                FileManagerSidebarIcon::Pin,
                true,
                false,
            )],
            vec![file_sidebar_item(
                "Root",
                "/",
                FileManagerSidebarIcon::Disk,
                true,
                false,
            )],
        );
        let area = Rect::new(0, 0, 24, 14);
        app.view.sidebar_tab_hit_areas = compute_sidebar_tab_areas(area);
        app.view.file_manager_sidebar_row_areas =
            compute_file_manager_sidebar_row_areas(&app, area);
        app.view.workspace_card_areas = Vec::new();

        let runtimes = TerminalRuntimeRegistry::new();
        let mut terminal = Terminal::new(TestBackend::new(24, 14)).unwrap();
        terminal
            .draw(|frame| render_workspace_list(&app, &runtimes, frame, area, false))
            .unwrap();

        let text: String = (0..14)
            .flat_map(|y| (0..24).map(move |x| (x, y)))
            .map(|(x, y)| terminal.backend().buffer()[(x, y)].symbol())
            .collect();
        for expected in ["FAVORITES", "Home", "PINNED", "Herdr", "LOCATIONS", "Root"] {
            assert!(text.contains(expected), "missing {expected:?}: {text:?}");
        }
        assert!(
            !text.contains("soon"),
            "placeholder must be removed: {text:?}"
        );
    }

    fn render_file_sidebar_for_test(
        app: &crate::app::state::AppState,
        width: u16,
        height: u16,
    ) -> Terminal<TestBackend> {
        let mut terminal = Terminal::new(TestBackend::new(width.max(1), height.max(1)))
            .expect("file sidebar test terminal should initialize");
        terminal
            .draw(|frame| {
                render_file_manager_sidebar(app, frame, Rect::new(0, 0, width, height));
            })
            .expect("file sidebar should render");
        terminal
    }

    // TP-C6.2-CURRENT/LIFECYCLE: exact prepared path identity is the only
    // current-location authority. Cwd transitions move the pill immediately;
    // inaccessible, absent, and closed-FM states cannot retain it.
    #[test]
    fn file_sidebar_current_pill_tracks_exact_accessible_open_cwd() {
        use crate::app::state::{FileManagerSidebarIcon, FileManagerSidebarModel, SidebarTab};
        let mut app = crate::app::state::AppState::test_new();
        app.sidebar_tab = SidebarTab::Files;
        app.file_manager_sidebar = FileManagerSidebarModel::from_sources(
            vec![
                file_sidebar_item(
                    "Home",
                    "/virtual/home",
                    FileManagerSidebarIcon::Home,
                    true,
                    false,
                ),
                file_sidebar_item(
                    "Downloads",
                    "/virtual/downloads",
                    FileManagerSidebarIcon::Downloads,
                    true,
                    false,
                ),
                file_sidebar_item(
                    "Missing",
                    "/virtual/missing",
                    FileManagerSidebarIcon::Pin,
                    false,
                    false,
                ),
            ],
            Vec::new(),
            Vec::new(),
        );
        let area = Rect::new(0, 0, 24, 8);
        let rows: Vec<_> = compute_file_manager_sidebar_row_areas(&app, area)
            .into_iter()
            .map(|row| row.rect)
            .collect();
        assert_eq!(rows.len(), 3);

        let row_has_accent = |terminal: &Terminal<TestBackend>, rect: Rect| {
            (rect.x..rect.x.saturating_add(rect.width)).any(|x| {
                terminal.backend().buffer()[(x, rect.y)].style().bg == Some(app.palette.accent)
            })
        };
        let row_symbols = |terminal: &Terminal<TestBackend>, rect: Rect| -> String {
            (rect.x..rect.x.saturating_add(rect.width))
                .map(|x| terminal.backend().buffer()[(x, rect.y)].symbol())
                .collect()
        };

        app.file_manager = Some(crate::fm::FmState::new("/virtual/home"));
        let home = render_file_sidebar_for_test(&app, area.width, area.height);
        assert!(row_has_accent(&home, rows[0]));
        assert!(row_symbols(&home, rows[0]).contains(''));
        assert!(row_symbols(&home, rows[0]).contains(''));
        assert!(!row_has_accent(&home, rows[1]));
        assert!(!row_has_accent(&home, rows[2]));

        app.file_manager = Some(crate::fm::FmState::new("/virtual/downloads"));
        let downloads = render_file_sidebar_for_test(&app, area.width, area.height);
        assert!(!row_has_accent(&downloads, rows[0]));
        assert!(row_has_accent(&downloads, rows[1]));
        assert!(!row_has_accent(&downloads, rows[2]));

        app.file_manager = Some(crate::fm::FmState::new("/virtual/missing"));
        let inaccessible = render_file_sidebar_for_test(&app, area.width, area.height);
        assert!(rows.iter().all(|row| !row_has_accent(&inaccessible, *row)));

        app.file_manager = Some(crate::fm::FmState::new("/virtual/not-in-model"));
        let absent = render_file_sidebar_for_test(&app, area.width, area.height);
        assert!(rows.iter().all(|row| !row_has_accent(&absent, *row)));

        app.file_manager = Some(crate::fm::FmState::new("/virtual/home"));
        app.sidebar_tab = SidebarTab::Spaces;
        let hidden = render_file_sidebar_for_test(&app, area.width, area.height);
        assert!(rows.iter().all(|row| !row_has_accent(&hidden, *row)));

        app.sidebar_tab = SidebarTab::Files;
        app.file_manager = None;
        let closed = render_file_sidebar_for_test(&app, area.width, area.height);
        assert!(rows.iter().all(|row| !row_has_accent(&closed, *row)));

        app.file_manager = Some(crate::fm::FmState::new("/virtual/home"));
        let reopened = render_file_sidebar_for_test(&app, area.width, area.height);
        assert!(row_has_accent(&reopened, rows[0]));
        assert!(rows[1..].iter().all(|row| !row_has_accent(&reopened, *row)));
    }

    // TP-C6.2-MARKER: warning is stronger than eject, markers are pinned to
    // the final drawable cell, and a plain item cannot invent an affordance.
    #[test]
    fn file_sidebar_markers_are_right_aligned_and_warning_precedes_eject() {
        use crate::app::state::{FileManagerSidebarIcon, FileManagerSidebarModel, SidebarTab};
        let mut app = crate::app::state::AppState::test_new();
        app.sidebar_tab = SidebarTab::Files;
        app.file_manager_sidebar = FileManagerSidebarModel::from_sources(
            Vec::new(),
            Vec::new(),
            vec![
                file_sidebar_item(
                    "Unavailable disk",
                    "/media/broken",
                    FileManagerSidebarIcon::Disk,
                    false,
                    true,
                ),
                file_sidebar_item(
                    "USB",
                    "/media/usb",
                    FileManagerSidebarIcon::Disk,
                    true,
                    true,
                ),
                file_sidebar_item("Root", "/", FileManagerSidebarIcon::Disk, true, false),
            ],
        );
        let area = Rect::new(0, 0, 20, 8);
        let rows: Vec<_> = compute_file_manager_sidebar_row_areas(&app, area)
            .into_iter()
            .map(|row| row.rect)
            .collect();
        assert_eq!(rows.len(), 3);
        let terminal = render_file_sidebar_for_test(&app, area.width, area.height);
        let buffer = terminal.backend().buffer();

        let last_cell =
            |rect: Rect| &buffer[(rect.x.saturating_add(rect.width).saturating_sub(1), rect.y)];
        assert_eq!(last_cell(rows[0]).symbol(), "⚠");
        assert_eq!(last_cell(rows[0]).style().fg, Some(app.palette.yellow));
        assert_eq!(last_cell(rows[1]).symbol(), "⏏");
        assert_eq!(last_cell(rows[1]).style().fg, Some(app.palette.blue));
        assert_eq!(last_cell(rows[2]).symbol(), " ");

        let narrow = render_file_sidebar_for_test(&app, 1, area.height);
        let narrow_rows = compute_file_manager_sidebar_row_areas(&app, Rect::new(0, 0, 1, 8));
        assert_eq!(
            narrow.backend().buffer()[(0, narrow_rows[0].rect.y)].symbol(),
            "⚠"
        );
        assert_eq!(
            narrow.backend().buffer()[(0, narrow_rows[1].rect.y)].symbol(),
            "⏏"
        );

        let _zero = render_file_sidebar_for_test(&app, 0, area.height);
    }

    // TP-C6.2-PILL: Unicode labels use cell-width truncation, a reserved
    // trailing marker never overlaps the current pill, and narrow rows omit
    // both caps together instead of exposing a clipped current indicator.
    #[test]
    fn file_sidebar_current_pill_is_complete_unicode_and_width_safe() {
        use crate::app::state::{FileManagerSidebarIcon, FileManagerSidebarModel, SidebarTab};
        let mut app = crate::app::state::AppState::test_new();
        app.sidebar_tab = SidebarTab::Files;
        app.file_manager_sidebar = FileManagerSidebarModel::from_sources(
            Vec::new(),
            Vec::new(),
            vec![file_sidebar_item(
                "資料Downloads",
                "/media/usb",
                FileManagerSidebarIcon::Disk,
                true,
                true,
            )],
        );
        app.file_manager = Some(crate::fm::FmState::new("/media/usb"));

        let area = Rect::new(0, 0, 14, 5);
        let row = compute_file_manager_sidebar_row_areas(&app, area)[0].rect;
        let terminal = render_file_sidebar_for_test(&app, area.width, area.height);
        let symbols: Vec<_> = (row.x..row.x.saturating_add(row.width))
            .map(|x| terminal.backend().buffer()[(x, row.y)].symbol())
            .collect();
        let left_cap = symbols
            .iter()
            .position(|symbol| *symbol == "")
            .expect("left cap");
        let right_cap = symbols
            .iter()
            .position(|symbol| *symbol == "")
            .expect("right cap");
        let marker = symbols
            .iter()
            .position(|symbol| *symbol == "⏏")
            .expect("eject marker");
        assert!(
            left_cap < right_cap && right_cap < marker,
            "symbols: {symbols:?}"
        );
        assert_eq!(marker, usize::from(row.width) - 1);
        let mut continuation_cells = 0;
        for index in left_cap + 1..right_cap {
            if continuation_cells > 0 {
                continuation_cells -= 1;
                continue;
            }
            let cell = &terminal.backend().buffer()[(row.x + index as u16, row.y)];
            assert_eq!(
                cell.style().bg,
                Some(app.palette.accent),
                "pill glyph start at cell {index} must retain the accent background"
            );
            // TestBackend represents hidden cells covered by a wide glyph as
            // reset blanks. The glyph-start style is the terminal-visible
            // style, so do not mistake those internal continuation cells for
            // holes in the pill background.
            continuation_cells = display_width(cell.symbol()).saturating_sub(1);
        }

        let narrow = render_file_sidebar_for_test(&app, 3, area.height);
        let narrow_row =
            compute_file_manager_sidebar_row_areas(&app, Rect::new(0, 0, 3, 5))[0].rect;
        let narrow_symbols: Vec<_> = (narrow_row.x..narrow_row.x.saturating_add(narrow_row.width))
            .map(|x| narrow.backend().buffer()[(x, narrow_row.y)].symbol())
            .collect();
        assert!(!narrow_symbols.contains(&""));
        assert!(!narrow_symbols.contains(&""));
        assert_eq!(narrow_symbols.last(), Some(&"⏏"));
    }

    // ---- Projects tab render + layout helpers --------------------------------

    fn test_chat(id: &str, title: &str, msg_count: usize) -> crate::claude_sessions::ClaudeSession {
        crate::claude_sessions::ClaudeSession {
            id: id.to_string(),
            title: title.to_string(),
            last_modified: std::time::SystemTime::UNIX_EPOCH,
            msg_count,
        }
    }

    fn project_sessions(
        path: &str,
        sessions: Vec<crate::claude_sessions::ClaudeSession>,
    ) -> crate::app::state::ProjectSessions {
        let total_count = sessions.len();
        crate::app::state::ProjectSessions {
            path: std::path::PathBuf::from(path),
            sessions,
            total_count,
        }
    }

    fn render_projects_to_text(app: &AppState, area: Rect) -> String {
        let runtimes = TerminalRuntimeRegistry::new();
        let mut terminal = Terminal::new(TestBackend::new(area.width, area.height)).unwrap();
        terminal
            .draw(|frame| render_workspace_list(app, &runtimes, frame, area, false))
            .unwrap();
        (0..area.height)
            .flat_map(|y| (0..area.width).map(move |x| (x, y)))
            .map(|(x, y)| terminal.backend().buffer()[(x, y)].symbol())
            .collect()
    }

    // T1.4a: an expanded project shows the ▾ chevron, its name, and every chat.
    #[test]
    fn render_projects_list_shows_project_and_chats() {
        let mut app = crate::app::state::AppState::test_new();
        app.sidebar_tab = crate::app::state::SidebarTab::Projects;
        app.mouse_capture = false;
        app.projects_sessions = vec![project_sessions(
            "/home/ayaz/projects/herdr",
            vec![
                test_chat("a", "first chat", 4),
                test_chat("b", "second chat", 2),
            ],
        )];
        let area = Rect::new(0, 0, 24, 12);
        app.view.sidebar_tab_hit_areas = compute_sidebar_tab_areas(area);
        app.view.project_row_areas = compute_project_row_areas(&app, area);

        let text = render_projects_to_text(&app, area);
        assert!(text.contains('▾'), "expanded chevron expected: {text:?}");
        assert!(text.contains("herdr"), "project name expected: {text:?}");
        assert!(text.contains("first chat"), "chat 1 expected: {text:?}");
        assert!(text.contains("second chat"), "chat 2 expected: {text:?}");
    }

    // T11b: wired-state markers stay in sync with the tab bar — the focused
    // tab's chat shows "▸", chats open in other tabs "●", closed chats none.
    #[test]
    fn render_projects_marks_focused_and_open_chats() {
        let mut app = crate::app::state::AppState::test_new();
        app.sidebar_tab = crate::app::state::SidebarTab::Projects;
        app.mouse_capture = false;
        app.projects_sessions = vec![project_sessions(
            "/p",
            vec![
                test_chat("sess-a", "alpha", 3),
                test_chat("sess-b", "beta", 3),
                test_chat("sess-c", "gamma", 3),
            ],
        )];
        let mut ws = crate::workspace::Workspace::test_new("p");
        let tab_b = ws.test_add_tab(Some("beta"));
        ws.tabs[0].resumed_session_id = Some("sess-a".to_string());
        ws.tabs[tab_b].resumed_session_id = Some("sess-b".to_string());
        ws.active_tab = 0;
        app.workspaces = vec![ws];
        app.active = Some(0);

        let area = Rect::new(0, 0, 24, 12);
        app.view.sidebar_tab_hit_areas = compute_sidebar_tab_areas(area);
        app.view.project_row_areas = compute_project_row_areas(&app, area);

        let text = render_projects_to_text(&app, area);
        assert!(
            text.contains("▸ alpha"),
            "focused marker expected: {text:?}"
        );
        assert!(text.contains("● beta"), "open marker expected: {text:?}");
        assert!(
            !text.contains("▸ gamma") && !text.contains("● gamma"),
            "closed chat must stay unmarked: {text:?}"
        );
    }

    // T1.4b: a collapsed project shows the ▸ chevron and hides its chats.
    #[test]
    fn render_projects_list_collapsed_hides_chats() {
        let mut app = crate::app::state::AppState::test_new();
        app.sidebar_tab = crate::app::state::SidebarTab::Projects;
        app.mouse_capture = false;
        app.projects_sessions = vec![project_sessions(
            "/home/ayaz/projects/herdr",
            vec![test_chat("a", "hidden chat", 4)],
        )];
        app.collapsed_project_paths
            .insert(std::path::PathBuf::from("/home/ayaz/projects/herdr"));
        let area = Rect::new(0, 0, 24, 12);
        app.view.sidebar_tab_hit_areas = compute_sidebar_tab_areas(area);
        app.view.project_row_areas = compute_project_row_areas(&app, area);

        let text = render_projects_to_text(&app, area);
        assert!(text.contains('▸'), "collapsed chevron expected: {text:?}");
        assert!(text.contains("herdr"), "project name expected: {text:?}");
        assert!(
            !text.contains("hidden chat"),
            "collapsed project must hide chats: {text:?}"
        );
    }

    // T1.4c: an expanded project with no chats shows the "(no chats)" row.
    #[test]
    fn render_projects_list_empty_project_shows_no_chats() {
        let mut app = crate::app::state::AppState::test_new();
        app.sidebar_tab = crate::app::state::SidebarTab::Projects;
        app.mouse_capture = false;
        app.projects_sessions = vec![project_sessions("/home/ayaz/projects/empty", Vec::new())];
        let area = Rect::new(0, 0, 24, 12);
        app.view.sidebar_tab_hit_areas = compute_sidebar_tab_areas(area);
        app.view.project_row_areas = compute_project_row_areas(&app, area);

        let text = render_projects_to_text(&app, area);
        assert!(text.contains("empty"), "project name expected: {text:?}");
        assert!(
            text.contains("(no chats)"),
            "empty project placeholder expected: {text:?}"
        );
    }

    #[test]
    fn compute_project_row_areas_expanded_lists_one_row_per_chat() {
        let mut app = crate::app::state::AppState::test_new();
        app.projects_sessions = vec![
            project_sessions(
                "/a",
                vec![test_chat("x", "one", 1), test_chat("y", "two", 1)],
            ),
            project_sessions("/b", Vec::new()),
        ];
        let area = Rect::new(0, 0, 24, 20);
        let rows = compute_project_row_areas(&app, area);
        // project /a (header + "+" + 2 chats) + project /b (header + "+" +
        // "(no chats)") = 7 areas; each header row contributes two disjoint
        // hit areas on the same line.
        assert_eq!(rows.len(), 7);
        assert!(matches!(
            rows[0].kind,
            ProjectRowKind::Project { proj_idx: 0 }
        ));
        assert!(matches!(
            rows[1].kind,
            ProjectRowKind::NewChat { proj_idx: 0 }
        ));
        assert!(matches!(
            rows[2].kind,
            ProjectRowKind::Chat {
                proj_idx: 0,
                chat_idx: 0
            }
        ));
        assert!(matches!(
            rows[3].kind,
            ProjectRowKind::Chat {
                proj_idx: 0,
                chat_idx: 1
            }
        ));
        assert!(matches!(
            rows[4].kind,
            ProjectRowKind::Project { proj_idx: 1 }
        ));
        assert!(matches!(
            rows[5].kind,
            ProjectRowKind::NewChat { proj_idx: 1 }
        ));
        assert!(matches!(
            rows[6].kind,
            ProjectRowKind::Empty { proj_idx: 1 }
        ));
        // The "+" button shares the header line but never overlaps the name
        // area — an ambiguous hit would fire the wrong action.
        assert_eq!(rows[1].rect.y, rows[0].rect.y);
        assert_eq!(rows[1].rect.x, rows[0].rect.x + rows[0].rect.width);
        assert_eq!(
            rows[1].rect.x + rows[1].rect.width,
            rows[0].rect.x + area.width
        );
        // Rows stack one per line inside the body (below the 2-row header).
        assert_eq!(rows[0].rect.y, area.y + WORKSPACE_SECTION_HEADER_ROWS);
        assert_eq!(rows[2].rect.y, rows[0].rect.y + 1);
    }

    // ---- Projects scroll (agent panel pattern; Files tab reuses it) ----

    /// 7-chat project + empty project → Header, Chat×5, More, Header, Empty
    /// (9 logical lines).
    fn scroll_fixture_app() -> crate::app::state::AppState {
        let mut app = crate::app::state::AppState::test_new();
        let many: Vec<_> = (0..7)
            .map(|i| test_chat(&format!("s{i}"), "t", 1))
            .collect();
        app.projects_sessions = vec![
            project_sessions("/a", many),
            project_sessions("/b", Vec::new()),
        ];
        app
    }

    // ---- FEAT-B: footer "actives" filter ----

    /// 3 chats, only sessions[1] open as a tab.
    fn actives_fixture_app() -> crate::app::state::AppState {
        let mut app = crate::app::state::AppState::test_new();
        app.projects_sessions = vec![project_sessions(
            "/a",
            vec![
                test_chat("s0", "t", 1),
                test_chat("s1", "t", 1),
                test_chat("s2", "t", 1),
            ],
        )];
        let mut ws = Workspace::test_new("space");
        let tab = ws.test_add_tab(Some("chat"));
        ws.tabs[tab].resumed_session_id = Some("s1".to_string());
        app.workspaces = vec![ws];
        app.projects_actives_only = true;
        app
    }

    #[test]
    fn actives_mode_lists_only_open_chats_with_original_indices() {
        let app = actives_fixture_app();
        let lines = project_row_lines(&app);
        assert_eq!(
            lines,
            vec![
                ProjectRowLine::Header { proj_idx: 0 },
                ProjectRowLine::Chat {
                    proj_idx: 0,
                    chat_idx: 1
                },
            ],
            "only the open chat is listed, keeping its original session index"
        );
    }

    #[test]
    fn actives_mode_shows_empty_row_when_no_chat_is_open() {
        let mut app = actives_fixture_app();
        app.workspaces[0]
            .tabs
            .iter_mut()
            .for_each(|tab| tab.resumed_session_id = None);
        let lines = project_row_lines(&app);
        assert_eq!(
            lines,
            vec![
                ProjectRowLine::Header { proj_idx: 0 },
                ProjectRowLine::Empty { proj_idx: 0 },
            ]
        );
    }

    #[test]
    fn actives_toggle_rect_stays_clear_of_the_other_footer_buttons() {
        let mut app = crate::app::state::AppState::test_new();
        crate::ui::compute_view(&mut app, Rect::new(0, 0, 106, 20));
        let actives = app.sidebar_actives_toggle_rect();
        if actives.width > 0 {
            let chat = app.sidebar_new_button_rect();
            let menu = app.global_launcher_rect();
            assert!(chat.x + chat.width <= actives.x, "overlaps the chat button");
            assert!(
                actives.x + actives.width <= menu.x,
                "overlaps the menu button"
            );
        }
    }

    #[test]
    fn project_row_lines_list_headers_chats_more_and_empty_in_order() {
        let app = scroll_fixture_app();
        let lines = project_row_lines(&app);
        assert_eq!(
            lines,
            vec![
                ProjectRowLine::Header { proj_idx: 0 },
                ProjectRowLine::Chat {
                    proj_idx: 0,
                    chat_idx: 0
                },
                ProjectRowLine::Chat {
                    proj_idx: 0,
                    chat_idx: 1
                },
                ProjectRowLine::Chat {
                    proj_idx: 0,
                    chat_idx: 2
                },
                ProjectRowLine::Chat {
                    proj_idx: 0,
                    chat_idx: 3
                },
                ProjectRowLine::Chat {
                    proj_idx: 0,
                    chat_idx: 4
                },
                ProjectRowLine::More { proj_idx: 0 },
                ProjectRowLine::Header { proj_idx: 1 },
                ProjectRowLine::Empty { proj_idx: 1 },
            ]
        );
    }

    #[test]
    fn projects_scroll_skips_leading_lines_and_relayouts_from_body_top() {
        let mut app = scroll_fixture_app();
        app.projects_scroll = 2;
        let area = Rect::new(0, 0, 24, 20);
        let rows = compute_project_row_areas(&app, area);
        // Lines 0 (header) and 1 (chat 0) scrolled away: the viewport now
        // starts at chat 1, laid out at the body's first row.
        assert!(matches!(
            rows[0].kind,
            ProjectRowKind::Chat {
                proj_idx: 0,
                chat_idx: 1
            }
        ));
        assert_eq!(rows[0].rect.y, area.y + WORKSPACE_SECTION_HEADER_ROWS);
    }

    #[test]
    fn projects_scroll_never_splits_a_header_from_its_new_chat_button() {
        let mut app = scroll_fixture_app();
        app.projects_scroll = 7; // first visible line = Header { proj_idx: 1 }
        let rows = compute_project_row_areas(&app, Rect::new(0, 0, 24, 20));
        assert!(matches!(
            rows[0].kind,
            ProjectRowKind::Project { proj_idx: 1 }
        ));
        assert!(matches!(
            rows[1].kind,
            ProjectRowKind::NewChat { proj_idx: 1 }
        ));
        assert_eq!(rows[0].rect.y, rows[1].rect.y);
    }

    #[test]
    fn projects_scrollbar_appears_only_when_the_list_overflows() {
        let app = scroll_fixture_app();
        // 9 logical lines; a 6-row area leaves a 3-row body → overflow.
        assert!(projects_scrollbar_rect(&app, Rect::new(0, 0, 24, 6)).is_some());
        // A 20-row area (17-row body) fits all 9 lines → no scrollbar.
        assert!(projects_scrollbar_rect(&app, Rect::new(0, 0, 24, 20)).is_none());
    }

    #[test]
    fn projects_rows_shrink_for_the_scrollbar_column() {
        let app = scroll_fixture_app();
        let area = Rect::new(0, 0, 24, 6);
        let rows = compute_project_row_areas(&app, area);
        let track =
            projects_scrollbar_rect(&app, area).expect("overflowing list shows a scrollbar");
        assert!(!rows.is_empty());
        for row in &rows {
            assert!(
                row.rect.x + row.rect.width <= track.x,
                "row overlaps the scrollbar column"
            );
        }
    }

    #[test]
    fn normalized_projects_scroll_clamps_to_the_list_end() {
        let app = scroll_fixture_app();
        let area = Rect::new(0, 0, 24, 6);
        // 9 lines, 3-row body → max scroll 6.
        assert_eq!(normalized_projects_scroll(&app, area, 99), 6);
        assert_eq!(normalized_projects_scroll(&app, area, 3), 3);
    }

    #[test]
    fn compute_project_row_areas_collapsed_emits_only_the_header() {
        let mut app = crate::app::state::AppState::test_new();
        app.projects_sessions = vec![project_sessions("/a", vec![test_chat("x", "one", 1)])];
        app.collapsed_project_paths
            .insert(std::path::PathBuf::from("/a"));
        let rows = compute_project_row_areas(&app, Rect::new(0, 0, 24, 20));
        assert_eq!(
            rows.len(),
            2,
            "header keeps its \"+\" button when collapsed"
        );
        assert!(matches!(
            rows[0].kind,
            ProjectRowKind::Project { proj_idx: 0 }
        ));
        assert!(matches!(
            rows[1].kind,
            ProjectRowKind::NewChat { proj_idx: 0 }
        ));
    }

    // T12c: a busy project lists only the newest 5 chats plus an inert
    // "… N older" row (the reader already sorts newest-first).
    #[test]
    fn compute_project_row_areas_caps_chats_and_adds_more_row() {
        let mut app = crate::app::state::AppState::test_new();
        let chats = (0..7)
            .map(|i| test_chat(&format!("s{i}"), &format!("c{i}"), 1))
            .collect();
        app.projects_sessions = vec![project_sessions("/a", chats)];
        let rows = compute_project_row_areas(&app, Rect::new(0, 0, 24, 20));
        // header + "+" + 5 chats + "… older" = 8 areas.
        assert_eq!(rows.len(), 8);
        assert!(matches!(
            rows[6].kind,
            ProjectRowKind::Chat {
                proj_idx: 0,
                chat_idx: 4
            }
        ));
        assert!(matches!(rows[7].kind, ProjectRowKind::More { proj_idx: 0 }));
    }

    #[test]
    fn compute_project_row_areas_clips_to_body_height() {
        let mut app = crate::app::state::AppState::test_new();
        app.projects_sessions = vec![project_sessions(
            "/a",
            vec![
                test_chat("x", "one", 1),
                test_chat("y", "two", 1),
                test_chat("z", "three", 1),
            ],
        )];
        // Height 4: 2 header rows + 1 footer row leaves exactly 1 body row, so
        // only the project header line (name area + "+" button) fits.
        let rows = compute_project_row_areas(&app, Rect::new(0, 0, 24, 4));
        assert_eq!(rows.len(), 2);
        assert!(matches!(
            rows[0].kind,
            ProjectRowKind::Project { proj_idx: 0 }
        ));
        assert!(matches!(
            rows[1].kind,
            ProjectRowKind::NewChat { proj_idx: 0 }
        ));
    }

    #[test]
    fn compute_project_row_areas_empty_without_projects() {
        let app = crate::app::state::AppState::test_new();
        assert!(compute_project_row_areas(&app, Rect::new(0, 0, 24, 20)).is_empty());
    }

    #[test]
    fn project_display_name_uses_final_component() {
        assert_eq!(
            project_display_name(std::path::Path::new("/home/ayaz/projects/herdr")),
            "herdr"
        );
        assert_eq!(project_display_name(std::path::Path::new("/")), "/");
    }

    #[test]
    fn format_relative_time_buckets_by_magnitude() {
        use std::time::{Duration, SystemTime};
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(10_000_000);
        let ago = |secs: u64| now - Duration::from_secs(secs);
        assert_eq!(format_relative_time(ago(5), now), "now");
        assert_eq!(format_relative_time(ago(300), now), "5m");
        assert_eq!(format_relative_time(ago(7_200), now), "2h");
        assert_eq!(format_relative_time(ago(172_800), now), "2d");
        assert_eq!(format_relative_time(ago(1_209_600), now), "2w");
        // A future mtime (clock skew) collapses to "now" instead of panicking.
        assert_eq!(
            format_relative_time(now + Duration::from_secs(60), now),
            "now"
        );
    }

    #[test]
    fn render_workspace_list_renders_workspace_cards_for_spaces_tab() {
        let mut app = crate::app::state::AppState::test_new();
        app.sidebar_tab = crate::app::state::SidebarTab::Spaces;
        app.mouse_capture = false;
        app.workspaces = vec![Workspace::test_new("myproj")];
        app.ensure_test_terminals();
        app.active = Some(0);
        app.selected = 0;
        let area = Rect::new(0, 0, 24, 12);
        app.view.sidebar_tab_hit_areas = compute_sidebar_tab_areas(area);
        app.view.workspace_card_areas = compute_workspace_card_areas(&app, area);

        let runtimes = TerminalRuntimeRegistry::new();
        let mut terminal = Terminal::new(TestBackend::new(24, 12)).unwrap();
        terminal
            .draw(|frame| render_workspace_list(&app, &runtimes, frame, area, true))
            .unwrap();

        let text: String = (0..12)
            .flat_map(|y| (0..24).map(move |x| (x, y)))
            .map(|(x, y)| terminal.backend().buffer()[(x, y)].symbol())
            .collect();
        assert!(
            text.contains("myproj"),
            "spaces tab should render workspace cards: {text:?}"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn all_workspaces_agent_panel_entries_use_live_root_runtime_cwd_for_workspace_label() {
        let unique = format!(
            "herdr-agent-panel-runtime-cwd-{}-{}",
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
        let mut workspace = Workspace::test_new("stale-name");
        workspace.custom_name = None;
        workspace.identity_cwd = stale_cwd.clone();
        let pane = workspace.tabs[0].root_pane;

        app.workspaces = vec![workspace];
        app.ensure_test_terminals();
        let terminal_id = app.workspaces[0].tabs[0].panes[&pane]
            .attached_terminal_id
            .clone();
        let terminal = app.terminals.get_mut(&terminal_id).unwrap();
        terminal.cwd = stale_cwd;
        terminal.detected_agent = Some(Agent::Pi);
        app.active = Some(0);
        app.selected = 0;

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
        let entries = agent_panel_entries_from(&app, &runtime_registry);
        let primary_label = entries[0].primary_label.clone();

        for (_, runtime) in runtime_registry.drain() {
            runtime.shutdown();
        }
        let _ = std::fs::remove_dir_all(root);

        assert_eq!(primary_label, "herdr");
    }

    #[test]
    fn all_workspaces_agent_panel_entries_prefer_agent_names_for_agent_identity() {
        let mut app = crate::app::state::AppState::test_new();
        let workspace = Workspace::test_new("bridge");
        let first_pane = workspace.tabs[0].root_pane;

        app.workspaces = vec![workspace];
        app.ensure_test_terminals();
        let first_terminal_id = app.workspaces[0].tabs[0].panes[&first_pane]
            .attached_terminal_id
            .clone();
        app.terminals
            .get_mut(&first_terminal_id)
            .unwrap()
            .detected_agent = Some(Agent::Pi);
        app.terminals
            .get_mut(&first_terminal_id)
            .unwrap()
            .set_agent_name("planner".into());
        app.active = Some(0);
        app.selected = 0;

        let entries = agent_panel_entries(&app);
        assert_eq!(entries[0].primary_label, "bridge");
        assert_eq!(entries[0].agent_label.as_deref(), Some("planner"));
    }

    #[test]
    fn all_workspaces_primary_label_truncates_workspace_and_tab() {
        let entry = AgentPanelEntry {
            ws_idx: 0,
            tab_idx: 0,
            pane_id: crate::layout::PaneId::from_raw(1),
            primary_label: "agent-browser".into(),
            primary_tab_label: Some("test-escalation".into()),
            agent_label: Some("claude".into()),
            state: AgentState::Idle,
            seen: true,
            last_agent_state_change_seq: None,
            custom_status: None,
            state_labels: std::collections::HashMap::new(),
        };

        let label = format_agent_panel_primary_label(&entry, 18);

        assert_eq!(label, "agent-bro… · test…");
    }

    #[test]
    fn expanded_sidebar_sections_handle_tiny_heights() {
        let (ws_area, detail_area) = expanded_sidebar_sections(Rect::new(0, 0, 20, 5), 0.9);

        assert_eq!(ws_area, Rect::new(0, 0, 19, 3));
        assert_eq!(detail_area, Rect::new(0, 3, 19, 2));
    }

    #[test]
    fn sidebar_section_divider_is_hidden_for_tiny_heights() {
        let divider = sidebar_section_divider_rect(Rect::new(0, 0, 20, 5), 0.5);

        assert_eq!(divider, Rect::default());
    }

    #[test]
    fn grouped_child_label_keeps_custom_workspace_name() {
        assert_eq!(
            grouped_child_display_label("renamed issue", Some("worktree/issue-137"), true),
            "renamed issue"
        );
    }

    #[test]
    fn grouped_child_label_uses_short_branch_for_auto_named_workspace() {
        assert_eq!(
            grouped_child_display_label("herdr-issue", Some("worktree/issue-137"), false),
            "issue-137"
        );
    }

    #[test]
    fn workspace_list_truncates_cjk_branch_without_panic() {
        let mut app = crate::app::state::AppState::test_new();
        let mut ws = Workspace::test_new("repo");
        ws.cached_git_branch = Some("feature/中文-分支-644".into());
        app.workspaces = vec![ws];
        app.active = Some(0);
        app.selected = 0;
        app.mode = Mode::Terminal;
        app.view.workspace_card_areas = vec![crate::app::state::WorkspaceCardArea {
            ws_idx: 0,
            rect: Rect::new(0, 1, 15, 2),
            indented: false,
        }];

        let mut terminal = Terminal::new(TestBackend::new(15, 6)).expect("test terminal");
        let runtimes = crate::terminal::TerminalRuntimeRegistry::new();

        terminal
            .draw(|frame| {
                render_workspace_list(&app, &runtimes, frame, Rect::new(0, 0, 15, 6), false)
            })
            .expect("workspace list should render");
    }

    fn workspace_with_worktree_space(
        name: &str,
        key: Option<&str>,
        checkout_key: &str,
    ) -> crate::workspace::Workspace {
        let mut ws = crate::workspace::Workspace::test_new(name);
        if let Some(key) = key {
            ws.worktree_space = Some(crate::workspace::WorktreeSpaceMembership {
                key: key.into(),
                label: "herdr".into(),
                repo_root: std::path::PathBuf::from("/repo/herdr"),
                checkout_path: std::path::PathBuf::from(checkout_key),
                is_linked_worktree: name != "main",
            });
        }
        ws
    }

    fn workspace_with_git_space(name: &str, key: &str) -> crate::workspace::Workspace {
        let mut ws = crate::workspace::Workspace::test_new(name);
        ws.cached_git_space = Some(crate::workspace::GitSpaceMetadata {
            key: key.into(),
            checkout_key: format!("/repo/{name}"),
            label: "herdr".into(),
            repo_root: std::path::PathBuf::from(format!("/repo/{name}")),
            is_linked_worktree: false,
        });
        ws
    }

    #[test]
    fn parent_workspace_row_stays_clickable_when_grouped() {
        let mut app = AppState::test_new();
        app.workspaces = vec![
            workspace_with_worktree_space("main", Some("repo-key"), "/repo/herdr"),
            workspace_with_worktree_space("issue", Some("repo-key"), "/repo/herdr-issue"),
        ];

        let (cards, headers) = compute_workspace_list_areas(&app, Rect::new(0, 0, 30, 20));

        assert!(headers.is_empty());
        assert_eq!(cards[0].ws_idx, 0);
        assert!(!cards[0].indented);
        assert_eq!(cards[1].ws_idx, 1);
        assert!(cards[1].indented);
        assert_eq!(cards[1].rect.y, cards[0].rect.y + cards[0].rect.height + 1);
    }

    #[test]
    fn linked_only_worktree_members_do_not_form_parentless_group() {
        let mut app = AppState::test_new();
        app.workspaces = vec![
            workspace_with_worktree_space("issue", Some("repo-key"), "/repo/herdr-issue"),
            workspace_with_worktree_space("review", Some("repo-key"), "/repo/herdr-review"),
        ];

        let entries = workspace_list_entries(&app);

        assert_eq!(
            entries,
            vec![
                WorkspaceListEntry::Workspace {
                    ws_idx: 0,
                    indented: false
                },
                WorkspaceListEntry::Workspace {
                    ws_idx: 1,
                    indented: false
                },
            ]
        );
    }

    #[test]
    fn compact_space_group_scroll_offset_can_start_inside_group() {
        let mut app = AppState::test_new();
        app.workspaces = vec![
            workspace_with_worktree_space("main", Some("repo-key"), "/repo/herdr"),
            workspace_with_worktree_space("one", Some("repo-key"), "/repo/herdr-one"),
            workspace_with_worktree_space("two", Some("repo-key"), "/repo/herdr-two"),
        ];
        let area = Rect::new(0, 0, 30, 20);
        app.workspace_scroll = normalized_workspace_scroll(&app, area, 2);

        let (cards, headers) = compute_workspace_list_areas(&app, area);

        assert!(headers.is_empty());
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].ws_idx, 2);
    }

    #[test]
    fn workspace_scroll_metrics_count_display_entries_not_raw_workspaces() {
        let mut app = AppState::test_new();
        app.workspaces = vec![
            workspace_with_worktree_space("main", Some("repo-key"), "/repo/herdr"),
            workspace_with_worktree_space("issue", Some("repo-key"), "/repo/herdr-issue"),
            Workspace::test_new("notes"),
        ];
        app.collapsed_space_keys.insert("repo-key".into());
        app.active = None;
        app.mode = Mode::Terminal;

        let ws_area = Rect::new(0, 0, 30, 6);
        let metrics = workspace_list_scroll_metrics(&app, ws_area);

        assert_eq!(metrics.viewport_rows, 1);
        assert_eq!(metrics.max_offset_from_bottom, 1);
        assert_eq!(metrics.offset_from_bottom, 1);
    }

    #[test]
    fn workspace_scroll_offset_applies_to_group_children() {
        let mut app = AppState::test_new();
        app.workspaces = vec![
            workspace_with_worktree_space("main", Some("repo-key"), "/repo/herdr"),
            workspace_with_worktree_space("issue", Some("repo-key"), "/repo/herdr-issue"),
            Workspace::test_new("notes"),
        ];
        app.collapsed_space_keys.insert("repo-key".into());
        app.active = None;
        app.mode = Mode::Terminal;
        app.workspace_scroll = 1;

        let (cards, headers) = compute_workspace_list_areas(&app, Rect::new(0, 0, 30, 12));

        assert!(headers.is_empty());
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].ws_idx, 2);
    }

    #[test]
    fn workspace_list_entries_group_multiple_workspaces_in_same_git_space() {
        let mut app = AppState::test_new();
        app.workspaces = vec![
            workspace_with_worktree_space("main", Some("repo-key"), "/repo/herdr"),
            workspace_with_worktree_space("issue", Some("repo-key"), "/repo/herdr-issue"),
        ];

        assert_eq!(
            workspace_list_entries(&app),
            vec![
                WorkspaceListEntry::Workspace {
                    ws_idx: 0,
                    indented: false,
                },
                WorkspaceListEntry::Workspace {
                    ws_idx: 1,
                    indented: true,
                },
            ]
        );
    }

    #[test]
    fn workspace_list_entries_group_non_contiguous_explicit_members() {
        let mut app = AppState::test_new();
        app.workspaces = vec![
            workspace_with_worktree_space("main", Some("repo-key"), "/repo/herdr"),
            workspace_with_git_space("normal", "other-key"),
            workspace_with_worktree_space("issue", Some("repo-key"), "/repo/herdr-issue"),
        ];

        assert_eq!(
            workspace_list_entries(&app),
            vec![
                WorkspaceListEntry::Workspace {
                    ws_idx: 0,
                    indented: false,
                },
                WorkspaceListEntry::Workspace {
                    ws_idx: 2,
                    indented: true,
                },
                WorkspaceListEntry::Workspace {
                    ws_idx: 1,
                    indented: false,
                },
            ]
        );
    }

    #[test]
    fn workspace_list_entries_do_not_group_normal_git_workspaces() {
        let mut app = AppState::test_new();
        app.workspaces = vec![
            workspace_with_git_space("one", "repo-key"),
            workspace_with_git_space("two", "repo-key"),
        ];

        assert_eq!(
            workspace_list_entries(&app),
            vec![
                WorkspaceListEntry::Workspace {
                    ws_idx: 0,
                    indented: false,
                },
                WorkspaceListEntry::Workspace {
                    ws_idx: 1,
                    indented: false,
                },
            ]
        );
    }

    #[test]
    fn workspace_list_entries_do_not_auto_attach_normal_git_workspace_to_group() {
        let mut app = AppState::test_new();
        app.workspaces = vec![
            workspace_with_worktree_space("main", Some("repo-key"), "/repo/herdr"),
            workspace_with_git_space("scratch", "repo-key"),
            workspace_with_worktree_space("issue", Some("repo-key"), "/repo/herdr-issue"),
        ];

        assert_eq!(
            workspace_list_entries(&app),
            vec![
                WorkspaceListEntry::Workspace {
                    ws_idx: 0,
                    indented: false,
                },
                WorkspaceListEntry::Workspace {
                    ws_idx: 2,
                    indented: true,
                },
                WorkspaceListEntry::Workspace {
                    ws_idx: 1,
                    indented: false,
                },
            ]
        );
    }

    #[test]
    fn workspace_list_entries_leave_single_git_and_non_git_workspaces_flat() {
        let mut app = AppState::test_new();
        app.workspaces = vec![
            workspace_with_git_space("one", "repo-key"),
            workspace_with_worktree_space("notes", None, "/notes"),
        ];

        assert_eq!(
            workspace_list_entries(&app),
            vec![
                WorkspaceListEntry::Workspace {
                    ws_idx: 0,
                    indented: false,
                },
                WorkspaceListEntry::Workspace {
                    ws_idx: 1,
                    indented: false,
                },
            ]
        );
    }

    #[test]
    fn collapsed_group_hides_inactive_children_but_keeps_active_visible() {
        let mut app = AppState::test_new();
        app.workspaces = vec![
            workspace_with_worktree_space("main", Some("repo-key"), "/repo/herdr"),
            workspace_with_worktree_space("issue", Some("repo-key"), "/repo/herdr-issue"),
        ];
        app.active = Some(1);
        app.mode = Mode::Terminal;
        app.collapsed_space_keys.insert("repo-key".into());

        assert_eq!(
            workspace_list_entries(&app),
            vec![
                WorkspaceListEntry::Workspace {
                    ws_idx: 0,
                    indented: false,
                },
                WorkspaceListEntry::Workspace {
                    ws_idx: 1,
                    indented: true,
                },
            ]
        );

        app.active = None;
        app.mode = Mode::Terminal;
        assert_eq!(
            workspace_list_entries(&app),
            vec![WorkspaceListEntry::Workspace {
                ws_idx: 0,
                indented: false,
            }]
        );
    }

    #[test]
    fn collapsed_group_keeps_selected_child_visible_in_navigate_mode() {
        let mut app = AppState::test_new();
        app.workspaces = vec![
            workspace_with_worktree_space("main", Some("repo-key"), "/repo/herdr"),
            workspace_with_worktree_space("issue", Some("repo-key"), "/repo/herdr-issue"),
        ];
        app.mode = Mode::Navigate;
        app.selected = 1;
        app.active = Some(1);
        app.collapsed_space_keys.insert("repo-key".into());

        assert_eq!(
            workspace_list_entries(&app),
            vec![
                WorkspaceListEntry::Workspace {
                    ws_idx: 0,
                    indented: false,
                },
                WorkspaceListEntry::Workspace {
                    ws_idx: 1,
                    indented: true,
                },
            ]
        );
    }
}
