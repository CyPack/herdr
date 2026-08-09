mod tokens;

use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use self::tokens::{ResolvedToken, ResolvedTokenKind, SpaceTokenContext};
use super::scrollbar::{render_scrollbar, should_show_scrollbar};
use super::status::{agent_icon, state_dot, state_label, state_label_color};
use super::text::{display_width, display_width_u16, truncate_end};
use super::widgets::panel_contrast_fg;
use crate::app::state::{AgentPanelSort, Palette, ProjectRowArea, ProjectRowKind};
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
    pub pane_label: Option<String>,
    pub terminal_title: Option<String>,
    pub terminal_title_stripped: Option<String>,
    pub agent_label: Option<String>,
    pub agent_kind_label: Option<String>,
    pub agent: Option<crate::detect::Agent>,
    pub state: AgentState,
    pub seen: bool,
    pub last_agent_state_change_seq: Option<u64>,
    pub state_labels: std::collections::HashMap<String, String>,
    pub tokens: std::collections::HashMap<String, String>,
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
    agent_panel_header_label_rect(area, agent_panel_sort_label(sort))
}

fn agent_panel_header_label_rect(area: Rect, label: &str) -> Rect {
    if area.width == 0 || area.height < 2 {
        return Rect::default();
    }

    let width = display_width_u16(label).min(area.width);
    Rect::new(
        area.x + area.width.saturating_sub(width),
        area.y + 1,
        width,
        1,
    )
}

fn active_agent_view_label(app: &AppState) -> Option<&str> {
    app.agent_view_override
        .as_ref()
        .map(|view| view.label.as_deref().unwrap_or("filtered"))
}

pub(crate) fn agent_panel_entries(app: &AppState) -> Vec<AgentPanelEntry> {
    agent_panel_entries_with_runtimes(app, None)
}

pub(crate) fn all_agent_panel_entries(app: &AppState) -> Vec<AgentPanelEntry> {
    collect_agent_panel_entries_with_runtimes(app, None)
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
    let mut entries = collect_agent_panel_entries_with_runtimes(app, terminal_runtimes);
    crate::app::agent_view::apply_agent_view(app, &mut entries);
    entries
}

fn collect_agent_panel_entries_with_runtimes(
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

    app.workspaces
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
                        pane_label: detail.pane_label,
                        terminal_title: detail.terminal_title,
                        terminal_title_stripped: detail.terminal_title_stripped,
                        agent_label: Some(detail.agent_label),
                        agent_kind_label: detail.agent_kind_label,
                        agent: detail.agent,
                        state: detail.state,
                        seen: detail.seen,
                        last_agent_state_change_seq: detail.last_agent_state_change_seq,
                        state_labels: detail.state_labels,
                        tokens: detail.tokens,
                    }
                })
        })
        .collect()
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

fn workspace_row_height(app: &AppState, ws: &crate::workspace::Workspace, indented: bool) -> u16 {
    let (state, seen) = ws.aggregate_state(&app.terminals);
    let label = if indented {
        grouped_child_display_label(
            &ws.display_name(),
            ws.branch().as_deref(),
            ws.custom_name.is_some(),
        )
    } else {
        ws.display_name()
    };
    let token_values = ws.metadata_tokens.values();
    tokens::space_rows(
        &app.sidebar_spaces,
        SpaceTokenContext {
            workspace: &label,
            branch: ws.branch().as_deref(),
            state_text: state_label(state, seen),
            ahead_behind: ws.git_ahead_behind(),
            tokens: &token_values,
            suppress_git_details: indented,
        },
    )
    .len()
    .max(1)
    .min(u16::MAX as usize) as u16
}

fn workspace_row_height_in_body(
    app: &AppState,
    workspace: &crate::workspace::Workspace,
    indented: bool,
    body_height: u16,
) -> u16 {
    workspace_row_height(app, workspace, indented).min(body_height)
}

/// The configured group gap, emitted only where the next row starts a new
/// top-level unit — a repository header or an ungrouped workspace.
///
/// TP-TREE-06: one rule for every row kind. A repository's header, its
/// checkouts and their open drawers read as one block, and the gap falls
/// between blocks. Before the header row existed the gap landed between a
/// group's parent and its own first child, which drew a separator inside a
/// group instead of around it.
fn workspace_entry_gap(app: &AppState, entries: &[WorkspaceListEntry], entry_idx: usize) -> u16 {
    match entries.get(entry_idx.saturating_add(1)) {
        Some(WorkspaceListEntry::GroupHeader { .. })
        | Some(WorkspaceListEntry::ProjectHeader { .. })
        | Some(WorkspaceListEntry::Workspace {
            indented: false, ..
        }) => app.sidebar_spaces.row_gap,
        _ => 0,
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

/// The space a workspace renders under, after `[[spaces.split]]` rules.
///
/// Resolution happens here, at render time, rather than being baked into the
/// persisted membership. That keeps the stored key the repository's — which is
/// what restore validates against — and makes a config reload re-group every
/// open workspace immediately instead of only newly opened ones.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EffectiveSpace {
    pub key: String,
    pub label: String,
    /// Whether this workspace may render as the group's header row (used by
    /// TP-SPLIT-GROUP-01). A repo
    /// space has exactly one candidate, its main checkout. A config space has
    /// no main checkout of its own, so every member is a candidate and the
    /// first one in workspace order takes the header.
    pub is_parent_candidate: bool,
    /// True when a `[[spaces.split]]` rule claimed this checkout.
    pub is_custom: bool,
}

pub(crate) fn effective_space(app: &AppState, ws_idx: usize) -> Option<EffectiveSpace> {
    let ws = app.workspaces.get(ws_idx)?;
    let space = ws.worktree_space()?;
    match crate::spaces::resolve_space_rule(
        &app.space_split_rules,
        &space.repo_root,
        &space.checkout_path,
        ws.branch().as_deref(),
    ) {
        Some(rule) => Some(EffectiveSpace {
            key: rule.key.clone(),
            label: rule.label.clone(),
            is_parent_candidate: true,
            is_custom: true,
        }),
        None => Some(EffectiveSpace {
            key: space.key.clone(),
            label: space.label.clone(),
            is_parent_candidate: !space.is_linked_worktree,
            is_custom: false,
        }),
    }
}

/// Workspace indices in the given space, in workspace order.
fn space_member_indices(app: &AppState, key: &str) -> Vec<usize> {
    (0..app.workspaces.len())
        .filter(|idx| effective_space(app, *idx).is_some_and(|space| space.key == key))
        .collect()
}

/// The member that renders as the group header, if the group has one.
fn space_parent_index(app: &AppState, key: &str) -> Option<usize> {
    space_member_indices(app, key)
        .into_iter()
        .find(|idx| effective_space(app, *idx).is_some_and(|space| space.is_parent_candidate))
}

/// Header text for a non-indented row: a config space names itself after the
/// rule, so the module — not whichever checkout happens to lead it — is what
/// the collapsed group reads as (TP-SPLIT-HEAD-01).
pub(crate) fn space_header_display_label(
    app: &AppState,
    ws_idx: usize,
    workspace_label: String,
) -> String {
    effective_space(app, ws_idx)
        .filter(|space| space.is_custom)
        .filter(|space| space_parent_index(app, &space.key) == Some(ws_idx))
        .filter(|space| space_member_indices(app, &space.key).len() >= 2)
        .map(|space| space.label)
        .unwrap_or(workspace_label)
}

/// The project a workspace's space belongs to, if any (TP-PROJ-MATCH-02).
pub(crate) fn workspace_project(
    app: &AppState,
    ws_idx: usize,
) -> Option<&crate::spaces::SpaceProject> {
    let space = effective_space(app, ws_idx)?;
    let repo_root = app
        .workspaces
        .get(ws_idx)
        .and_then(|ws| ws.worktree_space())
        .map(|membership| membership.repo_root.clone());
    crate::spaces::resolve_project(&app.space_projects, &space.key, repo_root.as_deref())
}

/// The project claiming `space_key`, resolved through the space's members.
pub(crate) fn project_for_space_key<'a>(
    app: &'a AppState,
    space_key: &str,
) -> Option<&'a crate::spaces::SpaceProject> {
    (0..app.workspaces.len())
        .find(|idx| effective_space(app, *idx).is_some_and(|space| space.key == space_key))
        .and_then(|idx| workspace_project(app, idx))
}

/// The configured project behind a header row's key.
pub(crate) fn project_for_key<'a>(
    app: &'a AppState,
    project_key: &str,
) -> Option<&'a crate::spaces::SpaceProject> {
    app.space_projects
        .iter()
        .find(|project| project.key == project_key)
}

/// The icon of the rule that shaped this space, if the rule set one.
fn space_rule_icon<'a>(app: &'a AppState, space_key: &str) -> Option<&'a str> {
    app.space_split_rules
        .iter()
        .find(|rule| rule.key == space_key)
        .and_then(|rule| rule.icon.as_deref())
}

// TP-PROJ-GROUP-03: folded, the project answers for every workspace it hides.
fn project_aggregate_state(app: &AppState, project_key: &str) -> (AgentState, bool) {
    (0..app.workspaces.len())
        .filter(|idx| {
            workspace_project(app, *idx).is_some_and(|project| project.key == project_key)
        })
        .filter_map(|idx| app.workspaces.get(idx))
        .map(|ws| ws.aggregate_state(&app.terminals))
        .max_by_key(|(state, seen)| workspace_attention_priority(*state, *seen))
        .unwrap_or((AgentState::Unknown, true))
}

fn space_aggregate_state(app: &AppState, key: &str) -> (AgentState, bool) {
    (0..app.workspaces.len())
        .filter(|idx| effective_space(app, *idx).is_some_and(|space| space.key == key))
        .filter_map(|idx| app.workspaces.get(idx))
        .map(|ws| ws.aggregate_state(&app.terminals))
        .max_by_key(|(state, seen)| workspace_attention_priority(*state, *seen))
        .unwrap_or((AgentState::Unknown, true))
}

// TP-SPLIT-HEAD-02: only the header row reports a group, keyed by the rule.
pub(crate) fn workspace_parent_group_state(
    app: &AppState,
    ws_idx: usize,
) -> Option<(String, bool)> {
    let space = effective_space(app, ws_idx)?;
    if !space.is_parent_candidate {
        return None;
    }
    if space_parent_index(app, &space.key) != Some(ws_idx) {
        return None;
    }
    let member_count = space_member_indices(app, &space.key).len();
    (member_count >= 2).then(|| {
        (
            space.key.clone(),
            app.collapsed_space_keys.contains(&space.key),
        )
    })
}

/// Branch-name prefixes worth dropping from a sidebar row.
///
/// TP-DRAW-08: in a column this narrow the prefix is pure cost — every row
/// spends five columns saying "feat/" and the part that tells the branches
/// apart is what gets truncated. Only this closed set is dropped: a namespace
/// the person chose themselves (`codex/…`, a customer name) is information,
/// and guessing at it would delete meaning rather than noise.
const DROPPED_BRANCH_PREFIXES: &[&str] = &[
    "worktree/",
    "feat/",
    "feature/",
    "fix/",
    "hotfix/",
    "bugfix/",
    "chore/",
    "refactor/",
    "docs/",
    "test/",
    "perf/",
    "style/",
    "ci/",
    "build/",
    "release/",
];

/// Drop a known conventional-commit prefix from a branch name.
///
/// TP-DRAW-09: a prefix is only dropped when something is left. `feat/` on its
/// own is the whole name, and a row reading as blank is worse than a row
/// reading as noisy.
pub(crate) fn strip_branch_prefix(branch: &str) -> &str {
    for prefix in DROPPED_BRANCH_PREFIXES {
        if let Some(rest) = branch.strip_prefix(prefix) {
            if !rest.is_empty() {
                return rest;
            }
        }
    }
    branch
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
    strip_branch_prefix(branch).to_string()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WorkspaceListEntry {
    /// The repository a worktree group belongs to, on a row of its own.
    ///
    /// TP-TREE-01: this row exists so that the two disclosures the Spaces tab
    /// owns — "show the sibling checkouts" and "show this checkout's chats" —
    /// live on different rows. While the group's parent checkout doubled as the
    /// group header, both arrows landed in the same gutter one column apart and
    /// a reader could not tell which arrow did what.
    ///
    /// It is deliberately not a workspace: it carries no `ws_idx`, so it can
    /// never be folded into the workspace-indexed area vector.
    GroupHeader {
        space_key: String,
    },
    /// A `[[spaces.project]]` umbrella, on a row of its own above the spaces
    /// it gathers.
    ///
    /// TP-PROJ-GROUP-01: like the repository header it carries no `ws_idx`, so
    /// it can never be folded into the workspace-indexed area vector; folding
    /// state is keyed by the project's config `key`.
    ProjectHeader {
        project_key: String,
    },
    Workspace {
        ws_idx: usize,
        indented: bool,
    },
    /// A remembered chat under an expanded workspace's drawer.
    Chat {
        ws_idx: usize,
        chat_idx: usize,
    },
    /// Placeholder under an expanded workspace that has no remembered chats.
    /// Shown rather than nothing, because an empty gap reads as a broken
    /// drawer — and an empty drawer is the honest answer for a branch whose
    /// work predates the ledger.
    NoChats {
        ws_idx: usize,
    },
    /// Inert "… N older" row when a workspace has more chats than the drawer
    /// lists.
    MoreChats {
        ws_idx: usize,
    },
}

/// How many chats a workspace's drawer lists before folding the rest into a
/// single "older" row. The sidebar is a glance surface, not an archive.
pub(crate) const WORKSPACE_CHAT_ROW_LIMIT: usize = 5;

impl WorkspaceListEntry {
    /// `Some(indented)` only for a workspace row.
    pub(crate) fn as_workspace(&self) -> Option<(usize, bool)> {
        match self {
            WorkspaceListEntry::Workspace { ws_idx, indented } => Some((*ws_idx, *indented)),
            _ => None,
        }
    }
}

/// The cell that opens and closes a workspace's chat drawer.
///
/// The right edge, because the left one already belongs to the worktree-group
/// chevron — two toggles sharing a cell would make one of them unreachable.
/// A workspace with no remembered chats gets no affordance at all: an arrow
/// that only ever reveals "(no chats)" is noise on every row.
/// Columns one tree level is worth.
pub(crate) const ROW_INDENT_STEP: u16 = 2;
/// Columns a disclosure arrow occupies: the glyph plus the breathing space
/// that keeps it from touching the name. A one-column control read as part of
/// the word next to it.
pub(crate) const DISCLOSURE_WIDTH: u16 = 2;
/// The arrow an open row wears.
pub(crate) const DISCLOSURE_OPEN: &str = "▾";
/// The arrow a closed row wears.
pub(crate) const DISCLOSURE_CLOSED: &str = "▸";
/// The rule that ties a drawer's rows to the checkout above them.
pub(crate) const DRAWER_GUIDE: &str = "│";

/// The disclosure cell of a workspace row: leading, at the row's own depth.
///
/// TP-TREE-10: a checkout's arrow sits at the checkout's depth, never in the
/// repository's column. The repository owns column 0 on its own header row, so
/// the two disclosures can no longer be confused for one another — which is
/// the whole reason the header row exists.
pub(crate) fn workspace_chat_toggle_cell(app: &AppState, card_rect: Rect, ws_idx: usize) -> Rect {
    if card_rect.width < 4 || workspace_chat_rows_for(app, ws_idx).is_empty() {
        return Rect::default();
    }
    let depth = u16::from(workspace_is_group_member(app, ws_idx))
        + u16::from(workspace_project(app, ws_idx).is_some());
    Rect::new(
        card_rect.x + depth * ROW_INDENT_STEP,
        card_rect.y,
        DISCLOSURE_WIDTH,
        1,
    )
}

/// Whether this workspace is drawn as a child of a repository header row.
pub(crate) fn workspace_is_group_member(app: &AppState, ws_idx: usize) -> bool {
    app.view
        .workspace_card_areas
        .iter()
        .find(|card| card.ws_idx == ws_idx)
        .map(|card| card.indented)
        .unwrap_or_else(|| {
            workspace_list_entries(app).iter().any(|entry| {
                matches!(
                    entry,
                    WorkspaceListEntry::Workspace {
                        ws_idx: idx,
                        indented: true,
                    } if *idx == ws_idx
                )
            })
        })
}

/// The "start a chat here" cell: the row's trailing edge, mirroring the "+" the
/// Projects tab puts on every project header.
///
/// Trailing rather than leading because the leading edge is where disclosure
/// lives; a create action sharing that space would be pressed by someone
/// meaning to expand. Offered on every workspace row, including ones with no
/// history — starting the first chat somewhere is exactly when the affordance
/// matters most.
pub(crate) fn workspace_new_chat_cell(card_rect: Rect) -> Rect {
    if card_rect.width < 6 {
        return Rect::default();
    }
    Rect::new(card_rect.x + card_rect.width - 1, card_rect.y, 1, 1)
}

/// The Spaces rows the mobile switcher lays out: workspaces only.
///
/// Its geometry is a strict two rows per workspace and it is a switcher rather
/// than a browser, so the chat drawer — which belongs to the desktop sidebar —
/// is filtered out here instead of being special-cased at three call sites.
/// The chats to show under `ws_idx`, or an empty slice when there are none.
pub(crate) fn workspace_chat_rows_for(
    app: &AppState,
    ws_idx: usize,
) -> &[crate::app::state::WorkspaceChatRow] {
    let Some(workspace) = app.workspaces.get(ws_idx) else {
        return &[];
    };
    let key = crate::persist::workspace_chats::ledger_key(&workspace.identity_cwd);
    app.workspace_chat_rows
        .get(&key)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

/// Whether `ws_idx`'s drawer is folded shut.
///
/// Closed is the default: opening every workspace at once would bury the
/// workspace list the tab exists for.
pub(crate) fn workspace_chat_drawer_collapsed(app: &AppState, ws_idx: usize) -> bool {
    let Some(workspace) = app.workspaces.get(ws_idx) else {
        return true;
    };
    let key = crate::persist::workspace_chats::ledger_key(&workspace.identity_cwd);
    !app.expanded_chat_workspaces.contains(&key)
}

/// Append a workspace's chat drawer rows, if it is open.
fn push_chat_drawer(app: &AppState, entries: &mut Vec<WorkspaceListEntry>, ws_idx: usize) {
    if workspace_chat_drawer_collapsed(app, ws_idx) {
        return;
    }
    let chats = workspace_chat_rows_for(app, ws_idx);
    if chats.is_empty() {
        entries.push(WorkspaceListEntry::NoChats { ws_idx });
        return;
    }
    for chat_idx in 0..chats.len().min(WORKSPACE_CHAT_ROW_LIMIT) {
        entries.push(WorkspaceListEntry::Chat { ws_idx, chat_idx });
    }
    if chats.len() > WORKSPACE_CHAT_ROW_LIMIT {
        entries.push(WorkspaceListEntry::MoreChats { ws_idx });
    }
}

pub(crate) fn normalized_workspace_scroll(app: &AppState, area: Rect, requested: usize) -> usize {
    let ws_area = workspace_list_rect(area, app.sidebar_section_split);
    let body = workspace_list_body_rect(ws_area, false);
    if body.height == 0 {
        return requested;
    }

    if workspace_list_entries(app).is_empty() {
        0
    } else {
        requested.min(workspace_list_bottom_start(app, ws_area))
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
    // TP-SPLIT-GROUP-01/02: a config space groups without a main checkout, and
    // several rules over one repository produce several sibling groups.
    // TP-SPLIT-GROUP-03/04: the two-member threshold and the untouched repo
    // group are shared with upstream's behaviour.
    let mut members_by_key = std::collections::HashMap::<String, Vec<usize>>::new();
    for ws_idx in 0..app.workspaces.len() {
        if let Some(space) = effective_space(app, ws_idx) {
            members_by_key.entry(space.key).or_default().push(ws_idx);
        }
    }
    let grouped_keys = members_by_key
        .iter()
        .filter(|(_, members)| {
            members.len() >= 2
                && members.iter().any(|idx| {
                    effective_space(app, *idx).is_some_and(|space| space.is_parent_candidate)
                })
        })
        .map(|(key, _)| key.clone())
        .collect::<std::collections::HashSet<_>>();

    let visible_group_idx = if matches!(app.mode, Mode::Navigate) {
        Some(app.selected)
    } else {
        app.active
    };
    let active_group =
        visible_group_idx.and_then(|idx| effective_space(app, idx).map(|space| space.key));

    // TP-PROJ-MATCH-02: a space's project is resolved once, through the
    // repository its members live in — so a config space split out of a repo
    // follows that repo into its project, and a promoted space key matches
    // regardless of repository.
    let project_of_space: std::collections::HashMap<String, String> = members_by_key
        .iter()
        .filter_map(|(key, members)| {
            let repo_root = members
                .first()
                .and_then(|idx| app.workspaces.get(*idx))
                .and_then(|ws| ws.worktree_space())
                .map(|space| space.repo_root.clone());
            crate::spaces::resolve_project(&app.space_projects, key, repo_root.as_deref())
                .map(|project| (key.clone(), project.key.clone()))
        })
        .collect();

    let mut emitted_groups = std::collections::HashSet::<String>::new();
    let mut emitted_projects = std::collections::HashSet::<String>::new();
    let mut entries = Vec::new();
    for ws_idx in 0..app.workspaces.len() {
        let space = effective_space(app, ws_idx);
        let project_key = space
            .as_ref()
            .and_then(|space| project_of_space.get(&space.key));

        let Some(project_key) = project_key else {
            push_space_block(
                app,
                &mut entries,
                ws_idx,
                &members_by_key,
                &grouped_keys,
                &mut emitted_groups,
                force_expanded,
                visible_group_idx,
                active_group.as_deref(),
            );
            continue;
        };

        if !emitted_projects.insert(project_key.clone()) {
            continue;
        }
        // TP-PROJ-GROUP-01: the project takes a row of its own above the
        // spaces it gathers, emitted where its first member sits so
        // workspace order keeps meaning what it meant.
        entries.push(WorkspaceListEntry::ProjectHeader {
            project_key: project_key.clone(),
        });

        if !force_expanded && app.collapsed_project_keys.contains(project_key) {
            // TP-PROJ-GROUP-02: like a folded module (TP-TREE-03), a folded
            // project keeps the checkout the user is standing in.
            if let Some(active_idx) = visible_group_idx.filter(|idx| {
                effective_space(app, *idx)
                    .is_some_and(|space| project_of_space.get(&space.key) == Some(project_key))
            }) {
                entries.push(WorkspaceListEntry::Workspace {
                    ws_idx: active_idx,
                    indented: true,
                });
                push_chat_drawer(app, &mut entries, active_idx);
            }
            continue;
        }

        // Every space the project claims, in workspace order, each laid out
        // exactly as it would be top-level.
        for member_idx in ws_idx..app.workspaces.len() {
            let claimed = effective_space(app, member_idx)
                .is_some_and(|space| project_of_space.get(&space.key) == Some(project_key));
            if !claimed {
                continue;
            }
            push_space_block(
                app,
                &mut entries,
                member_idx,
                &members_by_key,
                &grouped_keys,
                &mut emitted_groups,
                force_expanded,
                visible_group_idx,
                active_group.as_deref(),
            );
        }
    }
    entries
}

/// Lay out one workspace's contribution to the list: a plain row for an
/// ungrouped checkout, or — on first sight of its space — the whole group
/// block. Shared between the top-level walk and a project's member walk so
/// a space renders identically inside and outside a project.
#[allow(clippy::too_many_arguments)] // internal seam of one function, split for reuse not for API
fn push_space_block(
    app: &AppState,
    entries: &mut Vec<WorkspaceListEntry>,
    ws_idx: usize,
    members_by_key: &std::collections::HashMap<String, Vec<usize>>,
    grouped_keys: &std::collections::HashSet<String>,
    emitted_groups: &mut std::collections::HashSet<String>,
    force_expanded: bool,
    visible_group_idx: Option<usize>,
    active_group: Option<&str>,
) {
    let Some(space) =
        effective_space(app, ws_idx).filter(|space| grouped_keys.contains(&space.key))
    else {
        entries.push(WorkspaceListEntry::Workspace {
            ws_idx,
            indented: false,
        });
        push_chat_drawer(app, entries, ws_idx);
        return;
    };

    if !emitted_groups.insert(space.key.clone()) {
        return;
    }

    let Some(members) = members_by_key.get(&space.key) else {
        return;
    };
    let Some(parent_idx) = members
        .iter()
        .copied()
        .find(|idx| effective_space(app, *idx).is_some_and(|member| member.is_parent_candidate))
    else {
        entries.push(WorkspaceListEntry::Workspace {
            ws_idx,
            indented: false,
        });
        push_chat_drawer(app, entries, ws_idx);
        return;
    };
    let collapsed = !force_expanded && app.collapsed_space_keys.contains(&space.key);
    // TP-TREE-01: the repository takes a row of its own and owns the
    // "show the sibling checkouts" arrow. TP-TREE-04: every checkout, the
    // main one included, is then a child — so the arrow a checkout carries
    // can only ever mean "show my chats".
    entries.push(WorkspaceListEntry::GroupHeader {
        space_key: space.key.clone(),
    });

    if collapsed {
        // TP-TREE-03: a folded group keeps the checkout the user is
        // standing in, so folding never hides where you are.
        if let Some(active_idx) =
            visible_group_idx.filter(|_| active_group == Some(space.key.as_str()))
        {
            entries.push(WorkspaceListEntry::Workspace {
                ws_idx: active_idx,
                indented: true,
            });
            push_chat_drawer(app, entries, active_idx);
        }
    } else {
        // The main checkout leads; the linked worktrees follow in session
        // order, the order they already had as siblings.
        for member_idx in
            std::iter::once(&parent_idx).chain(members.iter().filter(|idx| **idx != parent_idx))
        {
            entries.push(WorkspaceListEntry::Workspace {
                ws_idx: *member_idx,
                indented: true,
            });
            push_chat_drawer(app, entries, *member_idx);
        }
    }
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

fn workspace_list_visible_count(app: &AppState, area: Rect, scroll: usize) -> usize {
    let body = workspace_list_body_rect(area, false);
    if body.width == 0 || body.height == 0 {
        return 0;
    }

    let mut used_rows = 0u16;
    let mut visible = 0usize;
    let entries = workspace_list_entries(app);
    for (entry_idx, entry) in entries.iter().enumerate().skip(scroll) {
        let Some((row_height, gap)) = entry_row_metrics(app, &entries, entry_idx, body.height)
        else {
            continue;
        };
        let _ = entry;
        if used_rows.saturating_add(row_height) > body.height {
            break;
        }
        used_rows = used_rows.saturating_add(row_height);
        visible += 1;
        used_rows = used_rows.saturating_add(gap).min(body.height);
    }
    visible
}

/// Height and trailing gap of one list entry, or `None` when it points at a
/// workspace that no longer exists.
///
/// Every consumer — visible-count, bottom-anchored scrolling and layout — asks
/// this one function, so a row can never be measured one way and drawn another.
/// Drawer rows are single-line and gapless on purpose: a gap between a
/// workspace and its own chats would read as a separator between groups.
fn entry_row_metrics(
    app: &AppState,
    entries: &[WorkspaceListEntry],
    entry_idx: usize,
    body_height: u16,
) -> Option<(u16, u16)> {
    match entries.get(entry_idx)? {
        // TP-TREE-06: one line. The header has to be measured here or the list
        // would scroll past a row it did draw. It never carries a gap: it must
        // hug the checkouts it introduces.
        WorkspaceListEntry::GroupHeader { .. } | WorkspaceListEntry::ProjectHeader { .. } => {
            Some((1, 0))
        }
        WorkspaceListEntry::Workspace { ws_idx, indented } => {
            let workspace = app.workspaces.get(*ws_idx)?;
            Some((
                workspace_row_height_in_body(app, workspace, *indented, body_height),
                workspace_entry_gap(app, entries, entry_idx),
            ))
        }
        WorkspaceListEntry::Chat { ws_idx, .. }
        | WorkspaceListEntry::NoChats { ws_idx }
        | WorkspaceListEntry::MoreChats { ws_idx } => {
            app.workspaces.get(*ws_idx)?;
            // A drawer's last row still has to separate its block from the
            // next one, or two repositories run together while a drawer is open.
            Some((1, workspace_entry_gap(app, entries, entry_idx)))
        }
    }
}

fn workspace_list_bottom_start(app: &AppState, area: Rect) -> usize {
    let body = workspace_list_body_rect(area, false);
    let entries = workspace_list_entries(app);
    let mut used_rows = 0u16;
    let mut start = entries.len();
    for entry_idx in (0..entries.len()).rev() {
        let Some((row_height, gap)) = entry_row_metrics(app, &entries, entry_idx, body.height)
        else {
            continue;
        };
        let needed = row_height.saturating_add(gap);
        if used_rows.saturating_add(needed) > body.height {
            break;
        }
        used_rows = used_rows.saturating_add(needed);
        start = entry_idx;
    }
    start.min(entries.len().saturating_sub(1))
}

pub(crate) fn workspace_list_scroll_metrics(
    app: &AppState,
    area: Rect,
) -> crate::pane::ScrollMetrics {
    let max_scroll = workspace_list_bottom_start(app, area);
    let scroll = app.workspace_scroll.min(max_scroll);
    let viewport_rows = workspace_list_visible_count(app, area, scroll);

    crate::pane::ScrollMetrics {
        offset_from_bottom: max_scroll.saturating_sub(scroll),
        max_offset_from_bottom: max_scroll,
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

fn resolved_agent_rows(app: &AppState, entry: &AgentPanelEntry) -> Vec<Vec<ResolvedToken>> {
    let label = entry
        .state_labels
        .get(agent_panel_status_key(entry.state, entry.seen))
        .map(String::as_str)
        .unwrap_or_else(|| state_label(entry.state, entry.seen));
    tokens::agent_rows(&app.sidebar_agents, entry, label)
}

pub(crate) fn agent_entry_height_in_body(
    app: &AppState,
    entry: &AgentPanelEntry,
    body_height: u16,
) -> u16 {
    (resolved_agent_rows(app, entry)
        .len()
        .max(1)
        .min(u16::MAX as usize) as u16)
        .min(body_height)
}

pub(crate) fn agent_entry_gap(app: &AppState, entry_idx: usize, entry_count: usize) -> u16 {
    if entry_idx + 1 < entry_count {
        app.sidebar_agents.row_gap
    } else {
        0
    }
}

fn agent_panel_visible_count_from(app: &AppState, area: Rect, scroll: usize) -> usize {
    let body = agent_panel_body_rect(area, false);
    if body.width == 0 || body.height == 0 {
        return 0;
    }

    let mut used_rows = 0u16;
    let mut visible = 0usize;
    let entries = agent_panel_entries(app);
    for (index, entry) in entries.iter().enumerate().skip(scroll) {
        let height = agent_entry_height_in_body(app, entry, body.height);
        if used_rows.saturating_add(height) > body.height {
            break;
        }
        used_rows = used_rows.saturating_add(height);
        visible += 1;
        used_rows = used_rows
            .saturating_add(agent_entry_gap(app, index, entries.len()))
            .min(body.height);
    }
    visible
}

fn agent_panel_bottom_start(app: &AppState, area: Rect) -> usize {
    let body = agent_panel_body_rect(area, false);
    let entries = agent_panel_entries(app);
    let mut used_rows = 0u16;
    let mut start = entries.len();
    for (index, entry) in entries.iter().enumerate().rev() {
        let gap = agent_entry_gap(app, index, entries.len());
        let needed = agent_entry_height_in_body(app, entry, body.height).saturating_add(gap);
        if used_rows.saturating_add(needed) > body.height {
            break;
        }
        used_rows = used_rows.saturating_add(needed);
        start = index;
    }
    start.min(entries.len().saturating_sub(1))
}

pub(crate) fn agent_panel_scroll_for_target(
    app: &AppState,
    area: Rect,
    current_scroll: usize,
    target: usize,
) -> usize {
    let max_scroll = agent_panel_bottom_start(app, area);
    if target < current_scroll {
        return target.min(max_scroll);
    }
    let mut scroll = current_scroll.min(max_scroll);
    while scroll < target {
        let visible = agent_panel_visible_count_from(app, area, scroll);
        if visible > 0 && target < scroll.saturating_add(visible) {
            break;
        }
        scroll += 1;
    }
    scroll.min(max_scroll)
}

pub(crate) fn agent_panel_scroll_metrics(app: &AppState, area: Rect) -> crate::pane::ScrollMetrics {
    let max_scroll = agent_panel_bottom_start(app, area);
    let scroll = app.agent_panel_scroll.min(max_scroll);
    let viewport_rows = agent_panel_visible_count_from(app, area, scroll);

    crate::pane::ScrollMetrics {
        offset_from_bottom: max_scroll.saturating_sub(scroll),
        max_offset_from_bottom: max_scroll,
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

/// Lay out the Spaces list: workspace cards and, separately, the group header
/// rows and the chat rows of any open drawer.
///
/// The three vectors stay apart because `WorkspaceCardArea` is
/// workspace-indexed — folding a chat or a header into it would make that
/// click resolve as a workspace, the same trap the tab strip already documents
/// for its stage entries (TP-FTAB-ENTRY-05). TP-TREE-05.
pub(crate) fn compute_workspace_list_areas(
    app: &AppState,
    area: Rect,
) -> (
    Vec<crate::app::state::WorkspaceCardArea>,
    Vec<crate::app::state::WorkspaceChatRowArea>,
    Vec<crate::app::state::WorkspaceGroupHeaderArea>,
    Vec<crate::app::state::WorkspaceProjectHeaderArea>,
) {
    let ws_area = workspace_list_rect(area, app.sidebar_section_split);
    if ws_area == Rect::default() {
        return (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    }

    let metrics = workspace_list_scroll_metrics(app, ws_area);
    let body = workspace_list_body_rect(ws_area, should_show_scrollbar(metrics));
    if body.width == 0 || body.height == 0 {
        return (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    }

    let scroll = app.workspace_scroll;
    let mut row_y = body.y;
    let body_bottom = body.y + body.height;
    let mut cards = Vec::new();
    let mut chat_rows = Vec::new();
    let mut group_headers = Vec::new();
    let mut project_headers = Vec::new();

    let entries = workspace_list_entries(app);
    for (entry_idx, entry) in entries.iter().enumerate().skip(scroll) {
        let Some((row_height, gap)) = entry_row_metrics(app, &entries, entry_idx, body.height)
        else {
            continue;
        };
        if row_y.saturating_add(row_height) > body_bottom {
            break;
        }
        let rect = Rect::new(body.x, row_y, body.width, row_height);
        match entry {
            WorkspaceListEntry::GroupHeader { space_key } => {
                group_headers.push(crate::app::state::WorkspaceGroupHeaderArea {
                    rect,
                    space_key: space_key.clone(),
                });
            }
            WorkspaceListEntry::ProjectHeader { project_key } => {
                project_headers.push(crate::app::state::WorkspaceProjectHeaderArea {
                    rect,
                    project_key: project_key.clone(),
                });
            }
            WorkspaceListEntry::Workspace { ws_idx, indented } => {
                cards.push(crate::app::state::WorkspaceCardArea {
                    ws_idx: *ws_idx,
                    rect,
                    indented: *indented,
                });
            }
            WorkspaceListEntry::Chat { ws_idx, chat_idx } => {
                chat_rows.push(crate::app::state::WorkspaceChatRowArea {
                    rect,
                    ws_idx: *ws_idx,
                    chat_idx: *chat_idx,
                });
            }
            // Placeholders occupy a row but are inert: nothing to click.
            WorkspaceListEntry::NoChats { .. } | WorkspaceListEntry::MoreChats { .. } => {}
        }
        row_y = row_y
            .saturating_add(row_height)
            .saturating_add(gap)
            .min(body_bottom);
    }

    (cards, chat_rows, group_headers, project_headers)
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
        let divider_color = if app.agent_view_override.is_some() {
            p.accent
        } else {
            p.surface_dim
        };
        for x in ws_area.x..ws_area.x + ws_area.width {
            buf[(x, divider_y)].set_symbol("─");
            buf[(x, divider_y)].set_style(Style::default().fg(divider_color));
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
            let position = detail_idx + 1;
            let position_style = Style::default().fg(p.overlay0);
            let (icon, icon_style) = agent_icon(detail.state, detail.seen, app.spinner_tick, p);
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled(format!("{position:<2}"), position_style),
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

fn resolved_token_spans(
    resolved: &[ResolvedToken],
    state_icon: (&str, Style),
    state_text_style: Style,
    workspace_style: Style,
    secondary_style: Style,
    custom_style: Style,
    p: &Palette,
    max_width: usize,
) -> Vec<Span<'static>> {
    let fixed_widths = resolved
        .iter()
        .map(|token| match &token.kind {
            ResolvedTokenKind::StateIcon => display_width(state_icon.0),
            ResolvedTokenKind::GitStatus { ahead, behind } => {
                usize::from(*ahead > 0) * display_width(&format!("↑{ahead}"))
                    + usize::from(*behind > 0) * display_width(&format!("↓{behind}"))
                    + usize::from(*ahead > 0 && *behind > 0)
            }
            _ => 0,
        })
        .collect::<Vec<_>>();
    let flexible_widths = resolved
        .iter()
        .map(|token| match &token.kind {
            ResolvedTokenKind::StateText(text)
            | ResolvedTokenKind::Workspace(text)
            | ResolvedTokenKind::Tab(text)
            | ResolvedTokenKind::Pane(text)
            | ResolvedTokenKind::Agent(text)
            | ResolvedTokenKind::TerminalTitle(text)
            | ResolvedTokenKind::Branch(text)
            | ResolvedTokenKind::Custom(text) => display_width(text),
            _ => 0,
        })
        .collect::<Vec<_>>();
    let minimum_width = |active: &[bool]| {
        let indices = active
            .iter()
            .enumerate()
            .filter_map(|(index, active)| active.then_some(index))
            .collect::<Vec<_>>();
        let content = indices
            .iter()
            .map(|index| fixed_widths[*index] + usize::from(flexible_widths[*index] > 0))
            .sum::<usize>();
        let separators = indices
            .windows(2)
            .map(|pair| display_width(tokens::separator(&resolved[pair[0]], &resolved[pair[1]])))
            .sum::<usize>();
        content + separators
    };
    let mut active = resolved.iter().map(|_| true).collect::<Vec<_>>();
    if minimum_width(&active) > max_width {
        for (index, width) in flexible_widths.iter().enumerate() {
            if *width > 0 {
                active[index] = false;
            }
        }
        for index in (0..resolved.len()).rev() {
            if flexible_widths[index] == 0 {
                continue;
            }
            active[index] = true;
            if minimum_width(&active) > max_width {
                active[index] = false;
            }
        }
    }
    let visible_indices = active
        .iter()
        .enumerate()
        .filter_map(|(index, active)| active.then_some(index))
        .collect::<Vec<_>>();
    let separator_width = visible_indices
        .windows(2)
        .map(|pair| display_width(tokens::separator(&resolved[pair[0]], &resolved[pair[1]])))
        .sum::<usize>();
    let fixed_width = visible_indices
        .iter()
        .map(|index| fixed_widths[*index])
        .sum::<usize>();
    let mut budgets = flexible_widths
        .iter()
        .enumerate()
        .map(|(index, width)| usize::from(active[index] && *width > 0))
        .collect::<Vec<_>>();
    let minimum = budgets.iter().sum::<usize>();
    let mut remaining = max_width
        .saturating_sub(separator_width + fixed_width)
        .saturating_sub(minimum);
    while remaining > 0 {
        let mut grew = false;
        for (budget, width) in budgets.iter_mut().zip(&flexible_widths) {
            if *budget > 0 && *budget < *width {
                *budget += 1;
                remaining -= 1;
                grew = true;
                if remaining == 0 {
                    break;
                }
            }
        }
        if !grew {
            break;
        }
    }
    let mut spans = Vec::new();
    for (position, index) in visible_indices.iter().copied().enumerate() {
        let token = &resolved[index];
        if position > 0 {
            let previous = &resolved[visible_indices[position - 1]];
            spans.push(Span::styled(
                tokens::separator(previous, token),
                Style::default().fg(p.overlay0).add_modifier(Modifier::DIM),
            ));
        }
        match &token.kind {
            ResolvedTokenKind::StateIcon => {
                spans.push(Span::styled(
                    state_icon.0.to_string(),
                    apply_token_style(state_icon.1, token.style),
                ));
            }
            ResolvedTokenKind::StateText(text) => {
                spans.push(Span::styled(
                    truncate_end(text, budgets[index]),
                    apply_token_style(state_text_style, token.style),
                ));
            }
            ResolvedTokenKind::Workspace(text) => {
                spans.push(Span::styled(
                    truncate_end(text, budgets[index]),
                    apply_token_style(workspace_style, token.style),
                ));
            }
            ResolvedTokenKind::Tab(text)
            | ResolvedTokenKind::Pane(text)
            | ResolvedTokenKind::Agent(text)
            | ResolvedTokenKind::Branch(text) => {
                spans.push(Span::styled(
                    truncate_end(text, budgets[index]),
                    apply_token_style(secondary_style, token.style),
                ));
            }
            ResolvedTokenKind::GitStatus { ahead, behind } => {
                if *ahead > 0 {
                    spans.push(Span::styled(
                        format!("↑{ahead}"),
                        apply_token_style(Style::default().fg(p.green), token.style),
                    ));
                }
                if *ahead > 0 && *behind > 0 {
                    spans.push(Span::styled(
                        " ",
                        apply_token_style(Style::default(), token.style),
                    ));
                }
                if *behind > 0 {
                    spans.push(Span::styled(
                        format!("↓{behind}"),
                        apply_token_style(Style::default().fg(p.red), token.style),
                    ));
                }
            }
            ResolvedTokenKind::TerminalTitle(text) | ResolvedTokenKind::Custom(text) => {
                spans.push(Span::styled(
                    truncate_end(text, budgets[index]),
                    apply_token_style(custom_style, token.style),
                ));
            }
        }
    }
    spans
}

fn apply_token_style(mut style: Style, patch: crate::config::SidebarTokenStyle) -> Style {
    if let Some(fg) = patch.fg {
        style = style.fg(fg.ratatui());
    }
    if let Some(bold) = patch.bold {
        style = if bold {
            style.add_modifier(Modifier::BOLD)
        } else {
            style.remove_modifier(Modifier::BOLD)
        };
    }
    if let Some(dim) = patch.dim {
        style = if dim {
            style.add_modifier(Modifier::DIM)
        } else {
            style.remove_modifier(Modifier::DIM)
        };
    }
    style
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

    // Projects alone owns an alternate global-sidebar body. Files is hosted in
    // CenterContent, so a legacy Files tab value keeps the Spaces tracker.
    match app.sidebar_tab {
        crate::app::state::SidebarTab::Spaces | crate::app::state::SidebarTab::Files => {}
        crate::app::state::SidebarTab::Projects => {
            render_projects_list(app, frame, area);
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
            // TP-TREE-11: the workspace you are in wears the accent outright,
            // the same sentence the active agent card and the active tab
            // already speak. Selection while navigating stays a tone apart, so
            // "where I am" and "what I am pointing at" never read alike.
            let bg = if is_active {
                p.accent
            } else if is_dragged {
                p.surface1
            } else {
                p.surface0
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

        let name_style = if is_active {
            Style::default()
                .fg(panel_contrast_fg(p))
                .add_modifier(Modifier::BOLD)
        } else if selected || is_dragged {
            Style::default().fg(p.text).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(p.subtext0)
        };

        let label = ws.display_name_from(&app.terminals, terminal_runtimes);
        let display_label = if card.indented {
            grouped_child_display_label(&label, ws.branch().as_deref(), ws.custom_name.is_some())
        } else {
            space_header_display_label(app, i, label)
        };
        // TP-TREE-08: a checkout row no longer carries a group chevron. The
        // repository owns that arrow on its own header row, so the only arrow
        // that can appear here means "show my chats".
        let (display_state, display_seen) = (agg_state, agg_seen);
        let state_icon = state_dot(display_state, display_seen, p);
        let state_text_style = Style::default()
            .fg(state_label_color(display_state, display_seen, p))
            .add_modifier(Modifier::DIM);
        let branch_style = Style::default().fg(if selected || is_active {
            p.mauve
        } else {
            p.overlay0
        });
        // TP-ICON-01: the branch glyph rides the workspace label itself, so it
        // renders after the state dot (the pinned state-before-name order),
        // truncates with the name, and an empty configured glyph turns it off.
        let branch_icon = app.space_icons.branch.trim();
        let display_label = if branch_icon.is_empty() {
            display_label
        } else {
            format!("{branch_icon} {display_label}")
        };
        let token_values = ws.metadata_tokens.values();
        let rows = tokens::space_rows(
            &app.sidebar_spaces,
            SpaceTokenContext {
                workspace: &display_label,
                branch: ws.branch().as_deref(),
                state_text: state_label(display_state, display_seen),
                ahead_behind: ws.git_ahead_behind(),
                tokens: &token_values,
                suppress_git_details: card.indented,
            },
        );

        // Depth is spent once, on indentation, and the disclosure column is
        // reserved on every row whether or not this row has an arrow — so the
        // names of sibling checkouts line up instead of stepping in and out.
        // A project adds one more step for everything it gathers
        // (TP-PROJ-GROUP-01).
        let project_shift = u16::from(workspace_project(app, i).is_some());
        let name_col =
            (project_shift + u16::from(card.indented)) * ROW_INDENT_STEP + DISCLOSURE_WIDTH;

        // TP-TREE-12: the count is information ("how much history is in here"),
        // the plus is an action, and every checkout offers it — starting a
        // chat on a branch is the point of the row. It stays quiet until the
        // row is the one you are on, and it is never bound to hover, which
        // would make a pointer move repaint the sidebar (TP-REPAINT-2B).
        let chat_count = workspace_chat_rows_for(app, i).len();
        let show_plus = app.mouse_capture;
        let badge = (chat_count > 0).then(|| chat_count.to_string());
        // The trailing chrome is reserved, not overdrawn: before the tree the
        // "+" was painted over whatever the name had already written there, so
        // a long enough workspace name simply lost its last character to it.
        // A row with no history pays one column, not four.
        let trailing = u16::from(show_plus)
            + badge
                .as_ref()
                .map(|text| text.len() as u16 + 2)
                .unwrap_or(0);

        for (row_index, resolved) in rows.iter().enumerate() {
            if row_index as u16 >= row_height || row_y + row_index as u16 >= list_bottom {
                break;
            }
            // Continuation rows align under the name, past the state column.
            let prefix_width = if row_index == 0 {
                name_col
            } else {
                name_col.saturating_add(2)
            };
            let mut spans = vec![Span::raw(" ".repeat(prefix_width as usize))];
            spans.extend(resolved_token_spans(
                resolved,
                state_icon,
                state_text_style,
                name_style,
                branch_style,
                branch_style,
                p,
                card.rect
                    .width
                    .saturating_sub(prefix_width)
                    .saturating_sub(if row_index == 0 { trailing } else { 0 })
                    as usize,
            ));
            frame.render_widget(
                Paragraph::new(Line::from(spans)),
                Rect::new(card.rect.x, row_y + row_index as u16, card.rect.width, 1),
            );
        }

        if row_y < list_bottom {
            let right = card.rect.x + card.rect.width;
            // TP-WSCHAT-23: the create affordance, mirroring the Projects tab's
            // per-project "+". Mouse chrome only, like every other button here.
            if show_plus {
                let plus = workspace_new_chat_cell(card.rect);
                if plus.width > 0 {
                    frame.buffer_mut()[(plus.x, plus.y)]
                        .set_symbol("+")
                        .set_style(Style::default().fg(if is_active {
                            panel_contrast_fg(p)
                        } else if selected {
                            p.accent
                        } else {
                            p.overlay0
                        }));
                }
            }
            if let Some(text) = badge.as_ref() {
                let width = text.len() as u16;
                let x = right
                    .saturating_sub(if show_plus { 2 } else { 0 })
                    .saturating_sub(width);
                let x = x.max(card.rect.x);
                if x > card.rect.x {
                    frame.render_widget(
                        Paragraph::new(Span::styled(
                            text.clone(),
                            Style::default().fg(if is_active {
                                panel_contrast_fg(p)
                            } else {
                                p.overlay1
                            }),
                        )),
                        Rect::new(x, row_y, width, 1),
                    );
                }
            }
        }

        // TP-WSCHAT-19 + TP-TREE-10: the drawer affordance, at this row's own
        // depth. Drawn last so it sits on top of the row's own text, and only
        // where there is history to reveal.
        let toggle = workspace_chat_toggle_cell(app, card.rect, i);
        if toggle.width > 0 && toggle.y < list_bottom {
            let open = !workspace_chat_drawer_collapsed(app, i);
            frame.buffer_mut()[(toggle.x, toggle.y)]
                .set_symbol(if open {
                    DISCLOSURE_OPEN
                } else {
                    DISCLOSURE_CLOSED
                })
                .set_style(Style::default().fg(if is_active {
                    panel_contrast_fg(p)
                } else if open {
                    p.accent
                } else {
                    p.overlay1
                }));
        }
    }

    render_workspace_project_headers(app, frame, list_bottom);
    render_workspace_group_headers(app, frame, list_bottom);

    render_workspace_chat_rows(app, frame, list_bottom);

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

/// The label a worktree space is known by, or the key itself as a last resort.
pub(crate) fn space_label_for_key(app: &AppState, key: &str) -> String {
    (0..app.workspaces.len())
        .find_map(|ws_idx| effective_space(app, ws_idx).filter(|space| space.key == key))
        .map(|space| space.label)
        .unwrap_or_else(|| key.to_string())
}

/// Draw the repository header of every worktree group.
///
/// TP-TREE-08: this row is the only place a group chevron may appear. It is
/// drawn from its own area vector, so it can never be mistaken for — or hit
/// as — one of the checkouts beneath it.
/// Draw the project umbrella header of every `[[spaces.project]]` block.
///
/// TP-PROJ-GROUP-01/03: the row wears the project's own icon and name; folded,
/// it answers for everything it hides with one aggregate state dot, exactly as
/// a folded module header does.
fn render_workspace_project_headers(app: &AppState, frame: &mut Frame, list_bottom: u16) {
    let p = &app.palette;
    for head in &app.view.workspace_project_header_areas {
        if head.rect.width == 0 || head.rect.y >= list_bottom {
            continue;
        }
        let Some(project) = project_for_key(app, &head.project_key) else {
            continue;
        };
        let collapsed = app.collapsed_project_keys.contains(&head.project_key);
        let mut spans = vec![
            Span::styled(
                if collapsed {
                    DISCLOSURE_CLOSED
                } else {
                    DISCLOSURE_OPEN
                },
                Style::default().fg(p.accent),
            ),
            Span::raw(" "),
        ];
        // TP-ICON-02: the project's own icon wins; the configured default
        // fills in; an empty string means no glyph at all.
        let icon = project
            .icon
            .as_deref()
            .unwrap_or(app.space_icons.project.as_str())
            .trim();
        if !icon.is_empty() {
            spans.push(Span::raw(format!("{icon} ")));
        }
        if collapsed {
            let (state, seen) = project_aggregate_state(app, &head.project_key);
            let (glyph, glyph_style) = state_dot(state, seen, p);
            spans.push(Span::styled(glyph, glyph_style));
            spans.push(Span::raw(" "));
        }
        let used = spans
            .iter()
            .map(|span| super::text::display_width(span.content.as_ref()))
            .sum::<usize>();
        spans.push(Span::styled(
            super::text::truncate_end(
                &project.name,
                (head.rect.width as usize).saturating_sub(used),
            ),
            Style::default().fg(p.text).add_modifier(Modifier::BOLD),
        ));
        frame.render_widget(Paragraph::new(Line::from(spans)), head.rect);
    }
}

fn render_workspace_group_headers(app: &AppState, frame: &mut Frame, list_bottom: u16) {
    let p = &app.palette;
    for head in &app.view.workspace_group_header_areas {
        if head.rect.width == 0 || head.rect.y >= list_bottom {
            continue;
        }
        let collapsed = app.collapsed_space_keys.contains(&head.space_key);
        let mut spans = Vec::new();
        // TP-PROJ-GROUP-01: inside a project the module header steps in once,
        // so the umbrella, its modules and their checkouts read as one tree.
        if project_for_space_key(app, &head.space_key).is_some() {
            spans.push(Span::raw(" ".repeat(ROW_INDENT_STEP as usize)));
        }
        spans.push(Span::styled(
            if collapsed {
                DISCLOSURE_CLOSED
            } else {
                DISCLOSURE_OPEN
            },
            Style::default().fg(p.accent),
        ));
        spans.push(Span::raw(" "));
        // TP-ICON-02: a rule's own icon, when it set one.
        if let Some(icon) = space_rule_icon(app, &head.space_key) {
            let icon = icon.trim();
            if !icon.is_empty() {
                spans.push(Span::raw(format!("{icon} ")));
            }
        }
        // Folded, the header answers for the checkouts it hides; open, each
        // checkout answers for itself and a second dot here would be noise.
        if collapsed {
            let (state, seen) = space_aggregate_state(app, &head.space_key);
            let (glyph, glyph_style) = state_dot(state, seen, p);
            spans.push(Span::styled(glyph, glyph_style));
            spans.push(Span::raw(" "));
        }
        let used = spans
            .iter()
            .map(|span| super::text::display_width(span.content.as_ref()))
            .sum::<usize>();
        spans.push(Span::styled(
            super::text::truncate_end(
                &space_label_for_key(app, &head.space_key),
                (head.rect.width as usize).saturating_sub(used),
            ),
            Style::default().fg(p.text).add_modifier(Modifier::BOLD),
        ));
        frame.render_widget(Paragraph::new(Line::from(spans)), head.rect);
    }
}

/// Draw the chat rows of every open drawer.
///
/// TP-WSCHAT-20: a chat row must read as belonging to its workspace and never
/// as a workspace itself — it is indented past the branch children, carries a
/// chat glyph rather than a state dot, and never takes the accent BACKGROUND
/// that marks the active workspace and the active agent card.
fn render_workspace_chat_rows(app: &AppState, frame: &mut Frame, list_bottom: u16) {
    let p = &app.palette;
    let now = std::time::SystemTime::now();
    for row in &app.view.workspace_chat_row_areas {
        if row.rect.width == 0 || row.rect.y >= list_bottom {
            continue;
        }
        let Some(chat) = workspace_chat_rows_for(app, row.ws_idx).get(row.chat_idx) else {
            continue;
        };
        // TP-TREE-08 resolves the old three-way overload of "▸": it now means
        // disclosure and nothing else. A chat that IS the focused tab says so
        // the way every other focused thing in this sidebar says it — with the
        // accent — and "●" keeps its single meaning, open in another tab.
        let wired = app.find_resumed_chat_tab(&chat.session_id);
        let focused = wired.is_some_and(|(ws_idx, tab_idx)| {
            app.active == Some(ws_idx)
                && app
                    .workspaces
                    .get(ws_idx)
                    .is_some_and(|ws| ws.active_tab_index() == tab_idx)
        });
        let marker = if wired.is_some() { "● " } else { "  " };
        let (title_style, marker_style) = if focused {
            (
                Style::default().fg(p.accent).add_modifier(Modifier::BOLD),
                Style::default().fg(p.accent),
            )
        } else if wired.is_some() {
            (Style::default().fg(p.text), Style::default().fg(p.accent))
        } else {
            (Style::default().fg(p.overlay1), Style::default())
        };

        // TP-TREE-09: the drawer hangs off its checkout's disclosure column,
        // and a rule is drawn down that column so the rows read as contained
        // by the checkout above them rather than floating under it. Inside a
        // project the whole block sits one step deeper (TP-PROJ-GROUP-01).
        let guide_indent = (u16::from(workspace_is_group_member(app, row.ws_idx))
            + u16::from(workspace_project(app, row.ws_idx).is_some()))
            * ROW_INDENT_STEP;
        let width = row.rect.width as usize;
        // TP-DRAW-05: every row carries an age. When the transcript itself
        // could not be located the ledger's own sighting answers the same
        // question — a drawer where only some rows are dated reads as broken
        // rather than partial.
        let age = chat
            .last_modified
            .map(|seen| format_relative_time(seen, now))
            .unwrap_or_else(|| {
                std::time::UNIX_EPOCH
                    .checked_add(std::time::Duration::from_millis(chat.last_seen_ms))
                    .map(|seen| format_relative_time(seen, now))
                    .unwrap_or_default()
            });
        let age_width = super::text::display_width(&age);
        // TP-ICON-01/03: the chat glyph, never a state dot — an empty
        // configured glyph turns the column off.
        let chat_icon = app.space_icons.chat.trim();
        let chat_icon_span = if chat_icon.is_empty() {
            String::new()
        } else {
            format!("{chat_icon} ")
        };
        let prefix_width =
            guide_indent as usize + 2 + marker.len() + super::text::display_width(&chat_icon_span);
        let title_budget = width
            .saturating_sub(prefix_width)
            .saturating_sub(if age_width > 0 { age_width + 1 } else { 0 });
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::raw(" ".repeat(guide_indent as usize)),
                Span::styled(DRAWER_GUIDE, Style::default().fg(p.surface1)),
                Span::raw(" "),
                Span::styled(marker, marker_style),
                Span::raw(chat_icon_span),
                Span::styled(
                    super::text::truncate_end(&chat.display_label(), title_budget),
                    title_style,
                ),
            ])),
            row.rect,
        );
        if age_width > 0 && age_width < width {
            frame.render_widget(
                Paragraph::new(Span::styled(age, Style::default().fg(p.overlay0)))
                    .alignment(Alignment::Right),
                row.rect,
            );
        }
    }
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
                            .is_some_and(|ws| ws.active_tab_index() == tab_idx)
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
pub(crate) fn project_display_name(path: &std::path::Path) -> String {
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
    let control_label = active_agent_view_label(app)
        .unwrap_or_else(|| agent_panel_sort_label(app.agent_panel_sort));
    let toggle_rect = agent_panel_header_label_rect(area, control_label);
    if toggle_rect != Rect::default() {
        let color = if app.agent_view_override.is_some() {
            p.accent
        } else {
            p.overlay0
        };
        frame.render_widget(
            Paragraph::new(Span::styled(
                control_label,
                Style::default().fg(color).add_modifier(Modifier::BOLD),
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
    if details.is_empty() && app.agent_view_override.is_some() {
        frame.render_widget(
            Paragraph::new(" no matching agents")
                .style(Style::default().fg(p.overlay0).add_modifier(Modifier::DIM)),
            Rect::new(body.x, body.y, body.width, 1),
        );
        return;
    }

    let scroll = app.agent_panel_scroll.min(metrics.max_offset_from_bottom);
    let mut row_y = body.y;
    let body_bottom = body.y + body.height;
    for (index, detail) in details.iter().enumerate().skip(scroll) {
        let label_color = state_label_color(detail.state, detail.seen, p);
        let rows = resolved_agent_rows(app, detail);
        let height = (rows.len().max(1) as u16).min(body.height);
        if row_y.saturating_add(height) > body_bottom {
            break;
        }

        let is_active = app.is_active_pane(detail.ws_idx, detail.tab_idx, detail.pane_id);
        // TP-AGPANEL-01: the active row speaks the active tab's language —
        // accent background, contrast text — so "which agent am I on" reads
        // the same way on both surfaces. Status color would drown on the
        // accent; the icon's shape still carries the state.
        let row_style = if is_active {
            Style::default().bg(p.accent)
        } else {
            Style::default()
        };
        let name_style = if is_active {
            Style::default()
                .fg(panel_contrast_fg(p))
                .add_modifier(Modifier::BOLD)
        } else {
            // TP-AGPANEL-02: passive rows give up their bold so the eye lands
            // on the active one.
            Style::default().fg(p.subtext0)
        };
        let status_style = if is_active {
            Style::default().fg(panel_contrast_fg(p))
        } else {
            Style::default().fg(label_color).add_modifier(Modifier::DIM)
        };
        let agent_style = if is_active {
            Style::default()
                .fg(panel_contrast_fg(p))
                .add_modifier(Modifier::DIM)
        } else {
            Style::default().fg(p.overlay0).add_modifier(Modifier::DIM)
        };
        let state_icon = agent_icon(detail.state, detail.seen, app.spinner_tick, p);

        for (row_index, resolved) in rows.iter().take(height as usize).enumerate() {
            let mut spans = vec![Span::raw(if row_index == 0 { " " } else { "   " })];
            spans.extend(resolved_token_spans(
                resolved,
                state_icon,
                status_style,
                name_style,
                agent_style,
                agent_style,
                p,
                body.width
                    .saturating_sub(if row_index == 0 { 1 } else { 3 }) as usize,
            ));
            frame.render_widget(
                Paragraph::new(Line::from(spans)).style(row_style),
                Rect::new(body.x, row_y + row_index as u16, body.width, 1),
            );
        }
        row_y = row_y
            .saturating_add(height)
            .saturating_add(agent_entry_gap(app, index, details.len()))
            .min(body_bottom);
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

    fn row_text(buffer: &ratatui::buffer::Buffer, row: u16, width: u16) -> String {
        (0..width)
            .map(|x| buffer[(x, row)].symbol())
            .collect::<String>()
            .trim_end()
            .to_string()
    }

    fn find_symbol_x(buffer: &ratatui::buffer::Buffer, row: u16, width: u16, symbol: &str) -> u16 {
        (0..width)
            .find(|x| buffer[(*x, row)].symbol() == symbol)
            .unwrap_or_else(|| {
                panic!(
                    "missing symbol {symbol:?} in row {}",
                    row_text(buffer, row, width)
                )
            })
    }

    #[test]
    fn default_agent_rows_remove_redundant_state_text() {
        let mut app = crate::app::state::AppState::test_new();
        let workspace = Workspace::test_new("one");
        let pane_id = workspace.tabs[0].root_pane;
        app.workspaces = vec![workspace];
        app.ensure_test_terminals();
        app.active = Some(0);
        let terminal_id = app.workspaces[0].tabs[0].panes[&pane_id]
            .attached_terminal_id
            .clone();
        let terminal_state = app.terminals.get_mut(&terminal_id).unwrap();
        terminal_state.detected_agent = Some(Agent::Pi);
        terminal_state.state = AgentState::Working;

        let area = Rect::new(0, 0, 26, 20);
        let mut terminal = Terminal::new(TestBackend::new(26, 20)).unwrap();
        terminal
            .draw(|frame| render_sidebar(&app, &TerminalRuntimeRegistry::new(), frame, area))
            .unwrap();
        let buffer = terminal.backend().buffer();
        let (_, agent_area) = expanded_sidebar_sections(area, app.sidebar_section_split);
        let body = agent_panel_body_rect(agent_area, false);

        let first = row_text(buffer, body.y, 25);
        let second = row_text(buffer, body.y + 1, 25);
        assert!(first.contains("one"));
        assert_eq!(second, "   pi");
        assert!(!first.contains("working"));
        assert!(!second.contains("working"));

        // TP-AGPANEL-01: this is the ACTIVE card, so it wears the accent
        // background with contrast text (updated 2026-07-29 with the model
        // the user chose; the old faint surface_dim style is gone).
        let workspace_x = find_symbol_x(buffer, body.y, body.width, "o");
        let workspace_style = buffer[(workspace_x, body.y)].style();
        assert_eq!(workspace_style.fg, Some(panel_contrast_fg(&app.palette)));
        assert!(workspace_style.add_modifier.contains(Modifier::BOLD));
        assert!(!workspace_style.add_modifier.contains(Modifier::DIM));
        assert_eq!(workspace_style.bg, Some(app.palette.accent));

        let agent_x = find_symbol_x(buffer, body.y + 1, body.width, "p");
        let agent_style = buffer[(agent_x, body.y + 1)].style();
        assert_eq!(agent_style.fg, Some(panel_contrast_fg(&app.palette)));
        assert!(agent_style.add_modifier.contains(Modifier::DIM));
        assert!(!agent_style.add_modifier.contains(Modifier::BOLD));
        assert_eq!(agent_style.bg, Some(app.palette.accent));
    }

    #[test]
    fn occurrence_false_removes_default_workspace_bold_and_agent_dim() {
        let config: crate::config::Config = toml::from_str(
            r##"
[ui.sidebar.agents]
rows = [[{ token = "workspace", bold = false }, { token = "agent", dim = false }]]
"##,
        )
        .unwrap();
        let mut app = crate::app::state::AppState::test_new();
        app.sidebar_agents = config.ui.sidebar.agents;
        let workspace = Workspace::test_new("one");
        let pane_id = workspace.tabs[0].root_pane;
        app.workspaces = vec![workspace];
        app.ensure_test_terminals();
        app.active = Some(0);
        let terminal_id = app.workspaces[0].tabs[0].panes[&pane_id]
            .attached_terminal_id
            .clone();
        app.terminals.get_mut(&terminal_id).unwrap().detected_agent = Some(Agent::Pi);

        let area = Rect::new(0, 0, 26, 20);
        let mut terminal = Terminal::new(TestBackend::new(26, 20)).unwrap();
        terminal
            .draw(|frame| render_sidebar(&app, &TerminalRuntimeRegistry::new(), frame, area))
            .unwrap();
        let (_, agent_area) = expanded_sidebar_sections(area, app.sidebar_section_split);
        let body = agent_panel_body_rect(agent_area, false);
        let buffer = terminal.backend().buffer();
        let workspace = buffer[(find_symbol_x(buffer, body.y, body.width, "o"), body.y)].style();
        let agent = buffer[(find_symbol_x(buffer, body.y, body.width, "p"), body.y)].style();

        // The base colors follow the active-card accent model (TP-AGPANEL-01);
        // what this test pins is that the config overrides still strip the
        // BOLD and DIM attributes from those bases.
        assert_eq!(workspace.fg, Some(panel_contrast_fg(&app.palette)));
        assert!(!workspace.add_modifier.contains(Modifier::BOLD));
        assert_eq!(agent.fg, Some(panel_contrast_fg(&app.palette)));
        assert!(!agent.add_modifier.contains(Modifier::DIM));
    }

    #[test]
    fn default_space_workspace_style_tracks_active_state() {
        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![Workspace::test_new("one"), Workspace::test_new("two")];
        app.active = Some(0);
        app.mode = Mode::Terminal;
        let area = Rect::new(0, 0, 26, 20);
        app.view.workspace_card_areas = compute_workspace_card_areas(&app, area);
        let first_row = app.view.workspace_card_areas[0].rect.y;
        let second_row = app.view.workspace_card_areas[1].rect.y;
        let mut terminal = Terminal::new(TestBackend::new(26, 20)).unwrap();
        terminal
            .draw(|frame| render_sidebar(&app, &TerminalRuntimeRegistry::new(), frame, area))
            .unwrap();
        let buffer = terminal.backend().buffer();

        // TP-TREE-11 moved the active row's base from a tone to the accent
        // itself; this test's subject is that active and inactive still differ,
        // which it still asserts below.
        let active = buffer[(find_symbol_x(buffer, first_row, 25, "o"), first_row)].style();
        assert_eq!(active.fg, Some(panel_contrast_fg(&app.palette)));
        assert!(active.add_modifier.contains(Modifier::BOLD));
        assert!(!active.add_modifier.contains(Modifier::DIM));
        assert_eq!(active.bg, Some(app.palette.accent));

        let inactive = buffer[(find_symbol_x(buffer, second_row, 25, "t"), second_row)].style();
        assert_eq!(inactive.fg, Some(app.palette.subtext0));
        assert!(!inactive
            .add_modifier
            .intersects(Modifier::BOLD | Modifier::DIM));
        assert_eq!(inactive.bg, Some(ratatui::style::Color::Reset));
    }

    #[test]
    fn space_occurrence_style_applies_without_styling_separator() {
        let config: crate::config::Config = toml::from_str(
            r##"
[ui.sidebar.spaces]
rows = [[{ token = "$hype", fg = "#abcdef", bold = true, dim = false }, "workspace"]]
"##,
        )
        .unwrap();
        let mut app = crate::app::state::AppState::test_new();
        app.sidebar_spaces = config.ui.sidebar.spaces;
        app.workspaces = vec![Workspace::test_new("one")];
        app.active = Some(0);
        app.mode = Mode::Terminal;
        app.workspaces[0].metadata_tokens.patch(
            std::collections::HashMap::from([("hype".into(), Some("HI".into()))]),
            None,
            std::time::Instant::now(),
        );

        let area = Rect::new(0, 0, 26, 20);
        app.view.workspace_card_areas = compute_workspace_card_areas(&app, area);
        let row = app.view.workspace_card_areas[0].rect.y;
        let mut terminal = Terminal::new(TestBackend::new(26, 20)).unwrap();
        terminal
            .draw(|frame| render_sidebar(&app, &TerminalRuntimeRegistry::new(), frame, area))
            .unwrap();
        let buffer = terminal.backend().buffer();
        let h = buffer[(find_symbol_x(buffer, row, 25, "H"), row)].style();
        let i = buffer[(find_symbol_x(buffer, row, 25, "I"), row)].style();
        let separator = buffer[(find_symbol_x(buffer, row, 25, "·"), row)].style();

        // The subject here is that the configured token style reaches the name
        // and not the separator. Only the row's background base moved
        // (TP-TREE-11).
        for style in [h, i] {
            assert_eq!(style.fg, Some(ratatui::style::Color::Rgb(0xab, 0xcd, 0xef)));
            assert!(style.add_modifier.contains(Modifier::BOLD));
            assert!(!style.add_modifier.contains(Modifier::DIM));
            assert_eq!(style.bg, Some(app.palette.accent));
        }
        assert_eq!(separator.fg, Some(app.palette.overlay0));
        assert!(separator.add_modifier.contains(Modifier::DIM));
        assert!(!separator.add_modifier.contains(Modifier::BOLD));
        assert_eq!(separator.bg, Some(app.palette.accent));
    }

    #[test]
    fn occurrence_foreground_flattens_composite_git_status_colors() {
        let config: crate::config::Config = toml::from_str(
            r##"[ui.sidebar.spaces]
rows = [[{ token = "git_status", fg = "#123456" }]]
"##,
        )
        .unwrap();
        let spans = resolved_token_spans(
            &[ResolvedToken {
                kind: ResolvedTokenKind::GitStatus {
                    ahead: 2,
                    behind: 1,
                },
                style: config.ui.sidebar.spaces.rows[0][0].parts().1,
            }],
            ("", Style::default()),
            Style::default(),
            Style::default(),
            Style::default(),
            Style::default(),
            &crate::app::state::AppState::test_new().palette,
            20,
        );

        assert_eq!(
            spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect::<String>(),
            "↑2 ↓1"
        );
        assert!(spans
            .iter()
            .all(|span| { span.style.fg == Some(ratatui::style::Color::Rgb(0x12, 0x34, 0x56)) }));
    }

    #[test]
    fn default_agent_row_gap_packs_rendering_and_scroll_geometry() {
        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![Workspace::test_new("one"), Workspace::test_new("two")];
        app.ensure_test_terminals();
        for (workspace, agent) in app.workspaces.iter().zip([Agent::Pi, Agent::Claude]) {
            let pane_id = workspace.tabs[0].root_pane;
            let terminal_id = workspace.tabs[0].panes[&pane_id]
                .attached_terminal_id
                .clone();
            app.terminals.get_mut(&terminal_id).unwrap().detected_agent = Some(agent);
        }
        app.sidebar_agents.rows = vec![vec![crate::config::AgentSidebarToken::Agent]];
        assert_eq!(app.sidebar_agents.row_gap, 0);

        let area = Rect::new(0, 0, 20, 5);
        let metrics = agent_panel_scroll_metrics(&app, area);
        let body = agent_panel_body_rect(area, false);
        let mut terminal = Terminal::new(TestBackend::new(20, 5)).unwrap();
        terminal
            .draw(|frame| render_agent_detail(&app, &TerminalRuntimeRegistry::new(), frame, area))
            .unwrap();
        let buffer = terminal.backend().buffer();

        assert_eq!(metrics.viewport_rows, 2);
        assert_eq!(metrics.max_offset_from_bottom, 0);
        assert_eq!(row_text(buffer, body.y, body.width), " pi");
        assert_eq!(row_text(buffer, body.y + 1, body.width), " claude");
    }

    #[test]
    fn narrow_agent_rows_preserve_later_tab_tokens() {
        let mut app = crate::app::state::AppState::test_new();
        let mut workspace = Workspace::test_new("very-long-workspace-name");
        let tab_idx = workspace.test_add_tab(Some("logs"));
        let pane_id = workspace.tabs[tab_idx].root_pane;
        app.workspaces = vec![workspace];
        app.ensure_test_terminals();
        let terminal_id = app.workspaces[0].tabs[tab_idx].panes[&pane_id]
            .attached_terminal_id
            .clone();
        app.terminals.get_mut(&terminal_id).unwrap().detected_agent = Some(Agent::Pi);
        // A custom-named tab leads with its own label and pairs with the git
        // branch of its terminal cwd, so the second token here is the branch
        // rather than the workspace name. Seed the branch cache to produce the
        // two-token row this test narrows.
        let cwd = std::path::PathBuf::from("/proj/logs");
        app.terminals.get_mut(&terminal_id).unwrap().cwd = cwd.clone();
        app.tab_branch_cache.insert(
            cwd,
            crate::app::tab_branches::TabBranchEntry::test_with_branch(Some(
                "very-long-branch-name",
            )),
        );

        let area = Rect::new(0, 0, 18, 20);
        let mut terminal = Terminal::new(TestBackend::new(18, 20)).unwrap();
        terminal
            .draw(|frame| render_sidebar(&app, &TerminalRuntimeRegistry::new(), frame, area))
            .unwrap();
        let buffer = terminal.backend().buffer();
        let (_, agent_area) = expanded_sidebar_sections(area, app.sidebar_section_split);
        let body = agent_panel_body_rect(agent_area, false);
        let first = row_text(buffer, body.y, 17);

        assert!(first.contains("logs"), "rendered row: {first:?}");
        assert!(first.contains('·'), "rendered row: {first:?}");
    }

    #[test]
    fn stripped_terminal_title_renders_with_unicode_width_truncation() {
        let mut app = crate::app::state::AppState::test_new();
        let workspace = Workspace::test_new("one");
        let pane_id = workspace.tabs[0].root_pane;
        app.workspaces = vec![workspace];
        app.ensure_test_terminals();
        let terminal_id = app.workspaces[0].tabs[0].panes[&pane_id]
            .attached_terminal_id
            .clone();
        let terminal = app.terminals.get_mut(&terminal_id).unwrap();
        terminal.detected_agent = Some(Agent::Claude);
        terminal.set_terminal_title(Some("⠋ 修复🙂标题很长".into()));
        app.sidebar_agents.rows = vec![vec![
            crate::config::AgentSidebarToken::TerminalTitleStripped,
        ]];

        let area = Rect::new(0, 0, 10, 12);
        let mut renderer = Terminal::new(TestBackend::new(10, 12)).unwrap();
        renderer
            .draw(|frame| render_sidebar(&app, &TerminalRuntimeRegistry::new(), frame, area))
            .unwrap();
        let (_, agent_area) = expanded_sidebar_sections(area, app.sidebar_section_split);
        let body = agent_panel_body_rect(agent_area, false);
        let rendered = row_text(renderer.backend().buffer(), body.y, 9);

        assert!(!rendered.contains('⠋'));
        assert!(rendered.contains('修') && rendered.contains('复'));

        let spans = resolved_token_spans(
            &[ResolvedToken::unstyled(ResolvedTokenKind::TerminalTitle(
                "修复🙂标题很长".into(),
            ))],
            ("", Style::default()),
            Style::default(),
            Style::default(),
            Style::default(),
            Style::default(),
            &app.palette,
            8,
        );
        let text = spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<String>();
        assert!(display_width(&text) <= 8, "resolved title: {text:?}");
    }

    #[test]
    fn variable_agent_heights_pack_the_bottom_and_reveal_targets() {
        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![
            Workspace::test_new("one"),
            Workspace::test_new("two"),
            Workspace::test_new("three"),
        ];
        app.ensure_test_terminals();
        for workspace in &app.workspaces {
            let pane_id = workspace.tabs[0].root_pane;
            let terminal_id = workspace.tabs[0].panes[&pane_id]
                .attached_terminal_id
                .clone();
            app.terminals.get_mut(&terminal_id).unwrap().detected_agent = Some(Agent::Pi);
        }
        let first_pane = app.workspaces[0].tabs[0].root_pane;
        let first_terminal = app.workspaces[0].tabs[0].panes[&first_pane]
            .attached_terminal_id
            .clone();
        app.terminals
            .get_mut(&first_terminal)
            .unwrap()
            .metadata_tokens
            .patch(
                std::collections::HashMap::from([
                    ("a".into(), Some("a".into())),
                    ("b".into(), Some("b".into())),
                ]),
                None,
                std::time::Instant::now(),
            );
        app.sidebar_agents.rows = vec![
            vec![crate::config::AgentSidebarToken::Agent],
            vec![crate::config::AgentSidebarToken::Custom("a".into())],
            vec![crate::config::AgentSidebarToken::Custom("b".into())],
        ];
        let area = Rect::new(0, 0, 20, 6);

        let metrics = agent_panel_scroll_metrics(&app, area);
        assert_eq!(metrics.max_offset_from_bottom, 1);
        assert_eq!(agent_panel_scroll_for_target(&app, area, 0, 2), 1);
    }

    #[test]
    fn oversized_space_layout_is_clipped_to_the_section_body() {
        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![Workspace::test_new("one"), Workspace::test_new("two")];
        app.sidebar_spaces.rows = vec![vec![crate::config::SpaceSidebarToken::Workspace]; 6];
        let area = Rect::new(0, 0, 20, 10);
        let workspace_area = workspace_list_rect(area, app.sidebar_section_split);
        let body = workspace_list_body_rect(workspace_area, false);

        let metrics = workspace_list_scroll_metrics(&app, workspace_area);
        let (cards, _, _headers, _) = compute_workspace_list_areas(&app, area);

        assert_eq!(metrics.viewport_rows, 1);
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].ws_idx, 0);
        assert_eq!(cards[0].rect.height, body.height);
    }

    #[test]
    fn oversized_agent_override_is_clipped_to_the_panel_body() {
        let mut app = crate::app::state::AppState::test_new();
        let workspace = Workspace::test_new("one");
        let pane_id = workspace.tabs[0].root_pane;
        app.workspaces = vec![workspace];
        app.ensure_test_terminals();
        let terminal_id = app.workspaces[0].tabs[0].panes[&pane_id]
            .attached_terminal_id
            .clone();
        app.terminals.get_mut(&terminal_id).unwrap().detected_agent = Some(Agent::Claude);
        app.sidebar_agents.rows_by_agent.insert(
            "claude".into(),
            vec![vec![crate::config::AgentSidebarToken::Agent]; 6],
        );
        let panel = Rect::new(0, 0, 20, 5);

        let metrics = agent_panel_scroll_metrics(&app, panel);

        assert_eq!(metrics.viewport_rows, 1);
        assert_eq!(metrics.max_offset_from_bottom, 0);
        let entry = agent_panel_entries(&app).pop().unwrap();
        assert_eq!(
            agent_entry_height_in_body(&app, &entry, agent_panel_body_rect(panel, false).height),
            agent_panel_body_rect(panel, false).height
        );
    }

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
    fn agent_panel_tab_label_visibility_tracks_tab_identity() {
        let mut app = crate::app::state::AppState::test_new();
        let single_auto = Workspace::test_new("auto");
        let mut single_custom = Workspace::test_new("custom");
        single_custom.tabs[0].set_custom_name("focus".into());
        let mut multi = Workspace::test_new("multi");
        multi.test_add_tab(Some("logs"));

        app.workspaces = vec![single_auto, single_custom, multi];
        app.ensure_test_terminals();
        for (ws_idx, tab_idx, agent) in [
            (0, 0, Agent::Pi),
            (1, 0, Agent::Claude),
            (2, 0, Agent::Codex),
            (2, 1, Agent::Pi),
        ] {
            let pane_id = app.workspaces[ws_idx].tabs[tab_idx].root_pane;
            let terminal_id = app.workspaces[ws_idx].tabs[tab_idx].panes[&pane_id]
                .attached_terminal_id
                .clone();
            app.terminals.get_mut(&terminal_id).unwrap().detected_agent = Some(agent);
        }

        let entries = agent_panel_entries(&app);
        let labels: Vec<_> = entries
            .iter()
            .map(|entry| {
                (
                    entry.primary_label.as_str(),
                    entry.primary_tab_label.as_deref(),
                )
            })
            .collect();

        // Custom-named tabs lead with their own label; the secondary slot
        // carries the git branch (none cached here), never the workspace
        // name (BUG-2b/2c behavior). Auto-named tabs keep workspace-first.
        assert_eq!(
            labels,
            [
                ("auto", None),
                ("focus", None),
                ("multi", Some("1")),
                ("logs", None),
            ]
        );
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
    fn collapsed_sidebar_numbers_grouped_agents_by_list_position() {
        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![Workspace::test_new("one"), Workspace::test_new("two")];
        app.ensure_test_terminals();

        for ws_idx in 0..app.workspaces.len() {
            let pane = app.workspaces[ws_idx].tabs[0].root_pane;
            let terminal_id = app.workspaces[ws_idx].tabs[0].panes[&pane]
                .attached_terminal_id
                .clone();
            app.terminals.get_mut(&terminal_id).unwrap().detected_agent = Some(Agent::Claude);
        }

        let area = Rect::new(0, 0, 4, 12);
        let (_, _, detail_area) = collapsed_sidebar_sections(area);
        let mut terminal = Terminal::new(TestBackend::new(area.width, area.height))
            .expect("test terminal should initialize");

        terminal
            .draw(|frame| render_sidebar_collapsed(&app, frame, area))
            .expect("collapsed sidebar should render");

        let buffer = terminal.backend().buffer();
        assert_eq!(buffer[(detail_area.x, detail_area.y)].symbol(), "1");
        assert_eq!(buffer[(detail_area.x, detail_area.y + 1)].symbol(), "2");
    }

    #[test]
    fn collapsed_sidebar_keeps_status_visible_for_two_digit_positions() {
        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = (1..=10)
            .map(|idx| Workspace::test_new(&format!("workspace-{idx}")))
            .collect();
        app.ensure_test_terminals();

        for ws_idx in 0..app.workspaces.len() {
            let pane = app.workspaces[ws_idx].tabs[0].root_pane;
            let terminal_id = app.workspaces[ws_idx].tabs[0].panes[&pane]
                .attached_terminal_id
                .clone();
            app.terminals.get_mut(&terminal_id).unwrap().detected_agent = Some(Agent::Claude);
        }

        let area = Rect::new(0, 0, 4, 25);
        let (_, _, detail_area) = collapsed_sidebar_sections(area);
        let mut terminal = Terminal::new(TestBackend::new(area.width, area.height))
            .expect("test terminal should initialize");

        terminal
            .draw(|frame| render_sidebar_collapsed(&app, frame, area))
            .expect("collapsed sidebar should render");

        let tenth_row = detail_area.y + 9;
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer[(detail_area.x, tenth_row)].symbol(), "1");
        assert_eq!(buffer[(detail_area.x + 1, tenth_row)].symbol(), "0");
        assert_eq!(buffer[(detail_area.x + 2, tenth_row)].symbol(), "○");
    }

    #[test]
    fn collapsed_sidebar_numbers_priority_agents_by_list_position() {
        let first = Workspace::test_new("one");
        let first_pane = first.tabs[0].root_pane;
        let mut second = Workspace::test_new("two");
        let second_pane = second.tabs[0].root_pane;
        let urgent_pane = second.test_split(ratatui::layout::Direction::Horizontal);

        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![first, second];
        app.ensure_test_terminals();
        app.agent_panel_sort = crate::app::state::AgentPanelSort::Priority;

        let set_state = |app: &mut crate::app::state::AppState, ws_idx: usize, pane_id, state| {
            let terminal_id = app.workspaces[ws_idx].tabs[0].panes[&pane_id]
                .attached_terminal_id
                .clone();
            let terminal = app.terminals.get_mut(&terminal_id).unwrap();
            terminal.detected_agent = Some(Agent::Claude);
            terminal.state = state;
        };
        set_state(&mut app, 0, first_pane, AgentState::Working);
        set_state(&mut app, 1, second_pane, AgentState::Working);
        set_state(&mut app, 1, urgent_pane, AgentState::Blocked);

        assert_eq!(app.workspaces[1].public_pane_number(urgent_pane), Some(2));
        assert_eq!(agent_panel_entries(&app)[0].pane_id, urgent_pane);

        let area = Rect::new(0, 0, 4, 16);
        let (_, _, detail_area) = collapsed_sidebar_sections(area);
        let mut terminal = Terminal::new(TestBackend::new(area.width, area.height))
            .expect("test terminal should initialize");

        terminal
            .draw(|frame| render_sidebar_collapsed(&app, frame, area))
            .expect("collapsed sidebar should render");

        let buffer = terminal.backend().buffer();
        assert_eq!(buffer[(detail_area.x, detail_area.y)].symbol(), "1");
        assert_eq!(buffer[(detail_area.x, detail_area.y + 1)].symbol(), "2");
        assert_eq!(buffer[(detail_area.x, detail_area.y + 2)].symbol(), "3");
        assert_eq!(buffer[(detail_area.x + 2, detail_area.y)].symbol(), "◉");
        assert_eq!(
            buffer[(detail_area.x + 2, detail_area.y)].style().fg,
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
        icon: crate::app::state::FileManagerLocationIcon,
        accessible: bool,
        ejectable: bool,
    ) -> crate::app::state::FileManagerLocationItem {
        crate::app::state::FileManagerLocationItem {
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
    fn file_locations_model_orders_sections_and_deduplicates_path_authority() {
        use crate::app::state::{
            FileManagerLocationIcon, FileManagerLocationSectionKind, FileManagerLocationsModel,
        };
        let model = FileManagerLocationsModel::from_sources(
            vec![
                file_sidebar_item(
                    "Home",
                    "/home/a",
                    FileManagerLocationIcon::Home,
                    true,
                    false,
                ),
                file_sidebar_item(
                    "Downloads",
                    "/home/a/Downloads",
                    FileManagerLocationIcon::Downloads,
                    true,
                    false,
                ),
            ],
            vec![
                file_sidebar_item(
                    "duplicate",
                    "/home/a",
                    FileManagerLocationIcon::Pin,
                    true,
                    false,
                ),
                file_sidebar_item(
                    "Missing",
                    "/missing",
                    FileManagerLocationIcon::Pin,
                    false,
                    false,
                ),
            ],
            vec![
                file_sidebar_item("Root", "/", FileManagerLocationIcon::Disk, true, false),
                file_sidebar_item(
                    "USB",
                    "/media/usb",
                    FileManagerLocationIcon::Disk,
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
                FileManagerLocationSectionKind::Favorites,
                FileManagerLocationSectionKind::Pinned,
                FileManagerLocationSectionKind::Locations,
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

        let without_pins = FileManagerLocationsModel::from_sources(
            vec![file_sidebar_item(
                "Home",
                "/home/a",
                FileManagerLocationIcon::Home,
                true,
                false,
            )],
            Vec::new(),
            vec![file_sidebar_item(
                "Root",
                "/",
                FileManagerLocationIcon::Disk,
                true,
                false,
            )],
        );
        assert_eq!(without_pins.sections.len(), 2);
        assert!(without_pins
            .sections
            .iter()
            .all(|section| section.kind != FileManagerLocationSectionKind::Pinned));
    }

    // TP-FCL-SHELL-01: even a legacy Files tab value renders the global
    // workspace tracker; Favorites/Locations belong exclusively to the
    // Native Files content rail.
    #[test]
    fn legacy_files_tab_value_renders_spaces_tracker_not_locations() {
        let mut app = crate::app::state::AppState::test_new();
        app.sidebar_tab = crate::app::state::SidebarTab::Files;
        app.mouse_capture = false; // skip new/menu chrome for a focused test
        app.workspaces = vec![crate::workspace::Workspace::test_new("Tracked Space")];
        app.active = Some(0);
        app.selected = 0;
        let area = Rect::new(0, 0, 24, 14);
        app.view.sidebar_tab_hit_areas = compute_sidebar_tab_areas(area);
        app.view.workspace_card_areas = compute_workspace_card_areas(&app, area);

        let runtimes = TerminalRuntimeRegistry::new();
        let mut terminal = Terminal::new(TestBackend::new(24, 14)).unwrap();
        terminal
            .draw(|frame| render_workspace_list(&app, &runtimes, frame, area, false))
            .unwrap();

        let text: String = (0..14)
            .flat_map(|y| (0..24).map(move |x| (x, y)))
            .map(|(x, y)| terminal.backend().buffer()[(x, y)].symbol())
            .collect();
        assert!(text.contains("Tracked Space"), "missing tracker: {text:?}");
        assert!(!text.contains("FAVORITES"), "locations leaked: {text:?}");
        assert!(!text.contains("LOCATIONS"), "locations leaked: {text:?}");
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
        ws.set_active_tab(0);
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

    fn app_with_chat_drawer(chat_count: usize) -> (AppState, String) {
        let mut app = AppState::test_new();
        let mut workspace = Workspace::test_new("drawer-probe");
        workspace.identity_cwd = std::env::temp_dir();
        app.workspaces = vec![workspace];
        app.active = Some(0);
        app.selected = 0;
        // Without this a narrow test area is treated as a phone and the desktop
        // sidebar is never laid out at all.
        app.mobile_width_threshold = 0;

        let key = crate::persist::workspace_chats::ledger_key(&std::env::temp_dir());
        let rows = (0..chat_count)
            .map(|idx| crate::app::state::WorkspaceChatRow {
                session_id: format!("session-{idx}"),
                agent: "claude".to_string(),
                title: Some(format!("chat {idx}")),
                last_seen_ms: 1_000 + idx as u64,
                last_modified: None,
            })
            .collect::<Vec<_>>();
        app.workspace_chat_rows.insert(key.clone(), rows);
        (app, key)
    }

    fn entry_kinds(app: &AppState) -> Vec<&'static str> {
        workspace_list_entries(app)
            .iter()
            .map(|entry| match entry {
                WorkspaceListEntry::GroupHeader { .. } => "group",
                WorkspaceListEntry::ProjectHeader { .. } => "project",
                WorkspaceListEntry::Workspace { .. } => "workspace",
                WorkspaceListEntry::Chat { .. } => "chat",
                WorkspaceListEntry::NoChats { .. } => "no-chats",
                WorkspaceListEntry::MoreChats { .. } => "more",
            })
            .collect()
    }

    // TP-WSCHAT-15: the drawer is closed until asked for. With a dozen-plus
    // workspaces, opening every drawer at once would bury the workspace list
    // the tab exists for.
    #[test]
    fn a_workspaces_chat_drawer_is_closed_until_it_is_opened() {
        let (mut app, key) = app_with_chat_drawer(2);
        assert_eq!(entry_kinds(&app), vec!["workspace"]);

        app.expanded_chat_workspaces.insert(key);
        assert_eq!(entry_kinds(&app), vec!["workspace", "chat", "chat"]);
    }

    // TP-WSCHAT-16: an open drawer with nothing to show says so. An empty gap
    // reads as a broken drawer, and an empty drawer is the honest answer for a
    // branch whose work predates the ledger.
    #[test]
    fn an_open_drawer_with_no_chats_shows_a_placeholder() {
        let (mut app, key) = app_with_chat_drawer(0);
        app.expanded_chat_workspaces.insert(key);

        assert_eq!(entry_kinds(&app), vec!["workspace", "no-chats"]);
    }

    // TP-WSCHAT-16: the sidebar is a glance surface, not an archive — a busy
    // workspace folds its tail into one inert row instead of pushing every
    // other workspace off the screen.
    #[test]
    fn a_busy_drawer_lists_a_capped_number_of_chats_and_an_older_row() {
        let (mut app, key) = app_with_chat_drawer(WORKSPACE_CHAT_ROW_LIMIT + 3);
        app.expanded_chat_workspaces.insert(key);

        let kinds = entry_kinds(&app);
        assert_eq!(
            kinds.iter().filter(|kind| **kind == "chat").count(),
            WORKSPACE_CHAT_ROW_LIMIT
        );
        assert_eq!(kinds.last(), Some(&"more"));
    }

    // TP-WSCHAT-17: the row list is the single source. If the scroll metrics
    // counted a different number of rows than the layout produced, the list
    // would scroll past rows it never drew — the failure the Projects tab
    // already solved by deriving both from one function.
    #[test]
    fn the_scroll_metrics_and_the_layout_agree_on_the_drawer_rows() {
        let (mut app, key) = app_with_chat_drawer(2);
        app.expanded_chat_workspaces.insert(key);
        let area = Rect::new(0, 0, 30, 24);

        let entries = workspace_list_entries(&app);
        let (cards, chat_rows, _headers, _) = compute_workspace_list_areas(&app, area);

        assert_eq!(entries.len(), 3, "one workspace plus its two chats");
        assert_eq!(cards.len(), 1);
        assert_eq!(chat_rows.len(), 2, "chat rows are laid out separately");
        assert_eq!(chat_rows[0].chat_idx, 0);
        assert_eq!(chat_rows[1].chat_idx, 1);
        assert!(
            chat_rows[0].rect.y < chat_rows[1].rect.y,
            "drawer rows keep the ledger's newest-first order"
        );
        assert!(
            cards[0].rect.y < chat_rows[0].rect.y,
            "the drawer sits under its workspace"
        );
    }

    // TP-WSCHAT-17: a chat row must never be mistaken for a workspace. The
    // card vector is workspace-indexed, so folding chat rows into it would make
    // a chat click resolve as a workspace switch.
    #[test]
    fn chat_rows_stay_out_of_the_workspace_indexed_card_vector() {
        let (mut app, key) = app_with_chat_drawer(3);
        app.expanded_chat_workspaces.insert(key);

        let (cards, chat_rows, _headers, _) =
            compute_workspace_list_areas(&app, Rect::new(0, 0, 30, 24));

        assert_eq!(cards.len(), 1, "only real workspaces become cards");
        assert!(cards.iter().all(|card| card.ws_idx == 0));
        assert!(!chat_rows.is_empty());
        assert!(
            chat_rows.iter().all(|row| row.ws_idx == 0),
            "every chat row names the workspace it belongs to"
        );
    }

    // TP-WSCHAT-19: the affordance only exists where it does something. An
    // arrow on every row that mostly reveals "(no chats)" is noise, and the
    // right edge is used because the left one already carries the worktree
    // group chevron — two toggles sharing a cell makes one unreachable.
    #[test]
    fn only_a_workspace_with_history_offers_a_drawer_toggle() {
        let (app, _) = app_with_chat_drawer(2);
        let card = Rect::new(0, 3, 20, 1);

        let cell = workspace_chat_toggle_cell(&app, card, 0);
        assert_eq!(
            cell.x, card.x,
            "the disclosure arrow LEADS the row, like every other one in this \
             sidebar and in the Projects tab — a trailing arrow reads as an \
             unrelated control at the far edge"
        );
        assert_eq!(cell.y, card.y);

        let (empty_app, _) = app_with_chat_drawer(0);
        assert_eq!(
            workspace_chat_toggle_cell(&empty_app, card, 0),
            Rect::default(),
            "a workspace with no remembered chats gets no arrow"
        );
    }

    // TP-WSCHAT-20: the drawer has to be visible to exist. A row whose state is
    // right but whose cells are empty is the failure this family already paid
    // for once, so this asserts on the drawn buffer, not on the row list.
    #[test]
    fn an_open_drawer_draws_its_chats_below_the_workspace() {
        let (mut app, key) = app_with_chat_drawer(2);
        app.expanded_chat_workspaces.insert(key);
        let area = Rect::new(0, 0, 90, 20);

        crate::ui::compute_view(&mut app, area);
        let backend = ratatui::backend::TestBackend::new(90, 20);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let registry = TerminalRuntimeRegistry::new();
        terminal
            .draw(|frame| render_sidebar(&app, &registry, frame, app.view.sidebar_rect))
            .unwrap();
        let buffer = terminal.backend().buffer();

        let rows: Vec<String> = (0..area.height)
            .map(|y| {
                (0..area.width)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect();
        let joined = rows.join("\n");
        assert!(
            joined.contains("chat 0") && joined.contains("chat 1"),
            "both remembered chats must be drawn:\n{joined}"
        );
        assert!(
            joined.contains('▾'),
            "an open drawer shows the open arrow:\n{joined}"
        );
        // TP-WSCHAT-22: the same wired-state vocabulary the Projects tab uses.
        // A chat that is not open carries a blank marker, not a glyph of its
        // own — two surfaces inventing different alphabets for one fact makes
        // the sidebar unreadable.
        assert!(
            !joined.contains('○'),
            "a closed chat has no glyph of its own; only ▸/● mean something:\n{joined}"
        );

        let chat_row_y = rows
            .iter()
            .position(|row| row.contains("chat 0"))
            .expect("chat row is on screen") as u16;
        let accent = app.palette.accent;
        assert!(
            (0..area.width).all(|x| buffer[(x, chat_row_y)].bg != accent),
            "a chat row must never take the accent background — that marks the \
             active workspace and the active agent card"
        );
    }

    // TP-WSCHAT-18: the phone drawer carries the chat rows of an open drawer.
    //
    // This row used to say the opposite. The flat mobile switcher derived a
    // workspace's position arithmetically — two rows each — so any row that was
    // not a workspace shifted every position after it and the switcher selected
    // the wrong one. The drawer that replaced it derives every position from
    // one row list (`mobile_drawer_rows`), which render, hit-testing, height and
    // the keyboard cursor all read, so an extra row kind cannot desynchronise
    // them. With the hazard gone, the exclusion only hid the chats from anyone
    // on a phone.
    #[test]
    fn the_mobile_drawer_sees_chat_rows() {
        let (mut app, key) = app_with_chat_drawer(4);
        app.expanded_chat_workspaces.insert(key);
        app.mobile_drawer = crate::app::state::MobileDrawer::Spaces;

        let chats = crate::ui::mobile_drawer_rows(&app)
            .iter()
            .filter(|row| matches!(row.content, crate::ui::DrawerRowContent::Chat { .. }))
            .count();
        assert_eq!(
            chats, 4,
            "every remembered chat is reachable from the phone"
        );
    }

    // TP-AGPANEL-01 + TP-AGPANEL-02: the panel answers "which agent am I on"
    // at a glance. The active row speaks the same language as the active tab —
    // accent background with contrast text — and exactly one row may say it.
    // The passive rows give up their bold so the eye lands on the active one;
    // status still reads through the icon's shape.
    #[test]
    fn the_active_agent_row_wears_the_accent_background_and_the_rest_stay_muted() {
        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![Workspace::test_new("alpha"), Workspace::test_new("beta")];
        app.ensure_test_terminals();
        for ws_idx in 0..2 {
            let pane = app.workspaces[ws_idx].tabs[0].root_pane;
            let terminal_id = app.workspaces[ws_idx].tabs[0].panes[&pane]
                .attached_terminal_id
                .clone();
            let terminal = app.terminals.get_mut(&terminal_id).unwrap();
            terminal.detected_agent = Some(Agent::Pi);
        }
        app.active = Some(0);
        app.selected = 0;

        let accent = app.palette.accent;
        let muted = app.palette.subtext0;
        let area = Rect::new(0, 0, 34, 8);
        let backend = ratatui::backend::TestBackend::new(34, 8);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let registry = TerminalRuntimeRegistry::new();
        terminal
            .draw(|frame| render_agent_detail(&app, &registry, frame, area))
            .unwrap();
        let buffer = terminal.backend().buffer();

        let row_text = |y: u16| -> String {
            (0..area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
                .trim_end()
                .to_string()
        };

        let accent_rows: Vec<u16> = (0..area.height)
            .filter(|&y| (0..area.width).any(|x| buffer[(x, y)].bg == accent))
            .collect();
        assert!(
            !accent_rows.is_empty(),
            "the active agent's card must claim the accent background"
        );
        let contiguous = accent_rows.windows(2).all(|w| w[1] == w[0] + 1);
        assert!(
            contiguous,
            "only ONE card may claim the accent — scattered accent rows mean \
             a second entry took it too: {accent_rows:?}"
        );
        assert!(
            accent_rows.iter().any(|&y| row_text(y).contains("alpha")),
            "the accent card must be the active workspace's agent"
        );

        let passive_y = (0..area.height)
            .find(|&y| row_text(y).contains("beta"))
            .expect("the passive agent must still be listed");
        assert!(
            !accent_rows.contains(&passive_y),
            "a passive agent's card must not wear the accent background"
        );
        let passive_name_is_bold = (0..area.width).any(|x| {
            let cell = &buffer[(x, passive_y)];
            cell.fg == muted && cell.modifier.contains(Modifier::BOLD)
        });
        assert!(
            !passive_name_is_bold,
            "a passive agent's name must give up its bold so the active one stands out"
        );
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

    /// A checkout of `/repo/herdr` on `branch`, living in its own directory.
    /// Everything here is a linked worktree: a config space's whole point is
    /// that it has no main checkout of its own.
    fn worktree_on_branch(name: &str, branch: &str) -> crate::workspace::Workspace {
        let mut ws = crate::workspace::Workspace::test_new(name);
        ws.cached_git_branch = Some(branch.into());
        ws.worktree_space = Some(crate::workspace::WorktreeSpaceMembership {
            key: "/repo/herdr/.git".into(),
            label: "herdr".into(),
            repo_root: std::path::PathBuf::from("/repo/herdr"),
            checkout_path: std::path::PathBuf::from(format!("/repo/herdr-{name}")),
            is_linked_worktree: true,
        });
        ws
    }

    fn split_rule(patterns: &[&str], key: &str, label: &str) -> crate::spaces::SpaceSplitRule {
        crate::spaces::SpaceSplitRule {
            repo_root: std::path::PathBuf::from("/repo/herdr"),
            patterns: patterns.iter().map(|p| (*p).to_string()).collect(),
            key: key.to_string(),
            label: label.to_string(),
            icon: None,
        }
    }

    #[test]
    fn config_space_groups_worktrees_that_have_no_main_checkout() {
        let mut app = AppState::test_new();
        app.workspaces = vec![
            worktree_on_branch("alpha", "feat/t4f-alpha"),
            worktree_on_branch("beta", "feat/t4f-beta"),
        ];
        app.space_split_rules = vec![split_rule(&["feat/t4f-*"], "herdr:t4f", "T4F")];

        let entries = workspace_list_entries(&app);
        assert_eq!(
            entries,
            vec![
                WorkspaceListEntry::GroupHeader {
                    space_key: "herdr:t4f".into()
                },
                WorkspaceListEntry::Workspace {
                    ws_idx: 0,
                    indented: true
                },
                WorkspaceListEntry::Workspace {
                    ws_idx: 1,
                    indented: true
                },
            ],
            "a config space must group even though every member is a linked worktree"
        );
    }

    #[test]
    fn config_space_header_row_shows_the_rule_label() {
        let mut app = AppState::test_new();
        app.workspaces = vec![
            worktree_on_branch("alpha", "feat/t4f-alpha"),
            worktree_on_branch("beta", "feat/t4f-beta"),
        ];
        app.space_split_rules = vec![split_rule(&["feat/t4f-*"], "herdr:t4f", "T4F BAM")];

        assert_eq!(
            space_header_display_label(&app, 0, "alpha".into()),
            "T4F BAM",
            "the collapsed group must read as the module, not as whichever checkout leads it"
        );
        assert_eq!(
            space_header_display_label(&app, 1, "beta".into()),
            "beta",
            "only the header row is renamed"
        );
    }

    #[test]
    fn config_space_splits_one_repository_into_several_groups() {
        let mut app = AppState::test_new();
        app.workspaces = vec![
            worktree_on_branch("alpha", "feat/t4f-alpha"),
            worktree_on_branch("beta", "feat/t4f-beta"),
            worktree_on_branch("gamma", "feat/circet-gamma"),
            worktree_on_branch("delta", "feat/circet-delta"),
        ];
        app.space_split_rules = vec![
            split_rule(&["feat/t4f-*"], "herdr:t4f", "T4F"),
            split_rule(&["feat/circet-*"], "herdr:circet", "Circet"),
        ];

        let entries = workspace_list_entries(&app);
        assert_eq!(
            entries,
            vec![
                WorkspaceListEntry::GroupHeader {
                    space_key: "herdr:t4f".into()
                },
                WorkspaceListEntry::Workspace {
                    ws_idx: 0,
                    indented: true
                },
                WorkspaceListEntry::Workspace {
                    ws_idx: 1,
                    indented: true
                },
                WorkspaceListEntry::GroupHeader {
                    space_key: "herdr:circet".into()
                },
                WorkspaceListEntry::Workspace {
                    ws_idx: 2,
                    indented: true
                },
                WorkspaceListEntry::Workspace {
                    ws_idx: 3,
                    indented: true
                },
            ],
            "two rules over one repository must produce two sibling groups"
        );
    }

    #[test]
    fn config_space_with_a_single_member_stays_flat() {
        let mut app = AppState::test_new();
        app.workspaces = vec![worktree_on_branch("alpha", "feat/t4f-alpha")];
        app.space_split_rules = vec![split_rule(&["feat/t4f-*"], "herdr:t4f", "T4F")];

        assert_eq!(
            workspace_list_entries(&app),
            vec![WorkspaceListEntry::Workspace {
                ws_idx: 0,
                indented: false
            }],
            "one member is a row, not a group — same rule the repo space follows"
        );
        assert_eq!(
            space_header_display_label(&app, 0, "alpha".into()),
            "alpha",
            "a row that heads no group keeps its own name"
        );
    }

    #[test]
    fn unclaimed_worktrees_keep_the_repository_group() {
        let mut app = AppState::test_new();
        app.workspaces = vec![
            workspace_with_worktree_space("main", Some("repo-key"), "/repo/herdr"),
            workspace_with_worktree_space("issue", Some("repo-key"), "/repo/herdr-issue"),
        ];
        app.space_split_rules = vec![split_rule(&["feat/t4f-*"], "herdr:t4f", "T4F")];

        assert_eq!(
            workspace_list_entries(&app),
            vec![
                WorkspaceListEntry::GroupHeader {
                    space_key: "repo-key".into()
                },
                WorkspaceListEntry::Workspace {
                    ws_idx: 0,
                    indented: true
                },
                WorkspaceListEntry::Workspace {
                    ws_idx: 1,
                    indented: true
                },
            ],
            "rules must not disturb checkouts they do not claim"
        );
        assert_eq!(
            space_header_display_label(&app, 0, "main".into()),
            "main",
            "a repo group keeps naming itself after its main checkout"
        );
    }

    #[test]
    fn config_space_collapse_state_is_keyed_by_the_rule_key() {
        let mut app = AppState::test_new();
        app.workspaces = vec![
            worktree_on_branch("alpha", "feat/t4f-alpha"),
            worktree_on_branch("beta", "feat/t4f-beta"),
        ];
        app.space_split_rules = vec![split_rule(&["feat/t4f-*"], "herdr:t4f", "T4F")];

        assert_eq!(
            workspace_parent_group_state(&app, 0),
            Some(("herdr:t4f".to_string(), false)),
            "the header row must report the rule key so collapse targets the module"
        );
        assert_eq!(
            workspace_parent_group_state(&app, 1),
            None,
            "a non-header member heads no group"
        );

        app.collapsed_space_keys.insert("herdr:t4f".into());
        assert_eq!(
            workspace_list_entries(&app),
            vec![
                WorkspaceListEntry::GroupHeader {
                    space_key: "herdr:t4f".into()
                },
                WorkspaceListEntry::Workspace {
                    ws_idx: 0,
                    indented: true
                },
            ],
            "collapsing the module keeps its header and the checkout in use, and \
             hides the rest"
        );
    }

    fn project_over(key: &str, repos: &[&str], spaces: &[&str]) -> crate::spaces::SpaceProject {
        crate::spaces::SpaceProject {
            key: key.to_string(),
            name: key.to_string(),
            icon: None,
            repo_roots: repos.iter().map(std::path::PathBuf::from).collect(),
            space_keys: spaces.iter().map(|s| (*s).to_string()).collect(),
        }
    }

    fn foreign_repo_workspace(name: &str, repo: &str) -> crate::workspace::Workspace {
        let mut ws = Workspace::test_new(name);
        ws.worktree_space = Some(crate::workspace::WorktreeSpaceMembership {
            key: format!("{repo}-key"),
            label: name.into(),
            repo_root: std::path::PathBuf::from(repo),
            checkout_path: std::path::PathBuf::from(repo),
            is_linked_worktree: false,
        });
        ws.identity_cwd = std::path::PathBuf::from(repo);
        ws
    }

    // TP-PROJ-MATCH-01 (render half): no projects configured, no project rows.
    #[test]
    fn entries_without_projects_have_no_project_header() {
        let app = app_with_worktree_tree(30);
        assert_eq!(
            entry_kinds(&app),
            vec!["group", "workspace", "chat", "workspace", "chat"],
            "without [[spaces.project]] the tree renders exactly as before"
        );
    }

    // TP-PROJ-GROUP-01.
    #[test]
    fn a_project_gathers_its_spaces_under_one_top_level_header() {
        let mut app = app_with_worktree_tree(30);
        app.space_projects = vec![project_over("project:herdr", &["/repo/herdr"], &[])];
        assert_eq!(
            entry_kinds(&app),
            vec!["project", "group", "workspace", "chat", "workspace", "chat"],
            "the four levels stack: project, module, checkout, chat"
        );
    }

    // TP-PROJ-GROUP-02.
    #[test]
    fn a_collapsed_project_keeps_the_active_checkout_visible() {
        let mut app = app_with_worktree_tree(30);
        app.space_projects = vec![project_over("project:herdr", &["/repo/herdr"], &[])];
        app.collapsed_project_keys
            .insert("project:herdr".to_string());
        app.active = Some(1);
        app.selected = 1;

        let entries = workspace_list_entries(&app);
        assert_eq!(
            entry_kinds(&app),
            vec!["project", "workspace", "chat"],
            "folding a project hides everything but the checkout in use"
        );
        assert!(
            matches!(
                entries[1],
                WorkspaceListEntry::Workspace {
                    ws_idx: 1,
                    indented: true,
                }
            ),
            "the surviving row is the active checkout, drawn as a child"
        );
    }

    // TP-PROJ-GROUP-04.
    #[test]
    fn spaces_outside_a_project_render_after_it_unchanged() {
        let mut app = app_with_worktree_tree(30);
        app.workspaces
            .push(foreign_repo_workspace("other", "/repo/other"));
        app.space_projects = vec![project_over("project:herdr", &["/repo/herdr"], &[])];

        let entries = workspace_list_entries(&app);
        assert_eq!(
            entry_kinds(&app),
            vec![
                "project",
                "group",
                "workspace",
                "chat",
                "workspace",
                "chat",
                "workspace"
            ],
            "an unclaimed repository keeps its plain row after the project block"
        );
        assert!(
            matches!(
                entries[6],
                WorkspaceListEntry::Workspace {
                    ws_idx: 2,
                    indented: false,
                }
            ),
            "the foreign checkout is neither indented nor claimed"
        );
    }

    /// Buffer text with runs of spaces squeezed to one. A wide glyph's
    /// continuation cell scrapes as its own space, so "🚀 x" reads back as
    /// "🚀  x"; comparing squeezed keeps the asserts about order, not about
    /// terminal width bookkeeping.
    fn squeezed_row_text(buffer: &ratatui::buffer::Buffer, row: u16, width: u16) -> String {
        row_text(buffer, row, width)
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn draw_tree(app: &mut AppState, area: Rect) -> ratatui::buffer::Buffer {
        crate::ui::compute_view(app, area);
        let mut terminal =
            Terminal::new(TestBackend::new(area.width, area.height)).expect("test backend");
        terminal
            .draw(|frame| render_sidebar(app, &TerminalRuntimeRegistry::new(), frame, area))
            .expect("draw");
        terminal.backend().buffer().clone()
    }

    // TP-PROJ-GROUP-01 (render): the umbrella row wears chevron, icon, name.
    #[test]
    fn project_header_row_draws_chevron_icon_and_name() {
        let mut app = app_with_worktree_tree(40);
        let mut project = project_over("project:herdr", &["/repo/herdr"], &[]);
        project.name = "herdr".into();
        project.icon = Some("🚀".into());
        app.space_projects = vec![project];

        let area = Rect::new(0, 0, 40, 24);
        let buffer = draw_tree(&mut app, area);
        let head = app.view.workspace_project_header_areas[0].rect;
        let text = squeezed_row_text(&buffer, head.y, area.width);
        assert!(
            text.contains("▾ 🚀 herdr"),
            "chevron, project icon and name in order; got {text:?}"
        );
    }

    // TP-PROJ-GROUP-03: folded, the header answers for everything it hides.
    #[test]
    fn collapsed_project_header_carries_an_aggregate_state_dot() {
        let mut app = app_with_worktree_tree(40);
        app.space_projects = vec![project_over("project:herdr", &["/repo/herdr"], &[])];
        app.collapsed_project_keys.insert("project:herdr".into());

        let area = Rect::new(0, 0, 40, 24);
        let buffer = draw_tree(&mut app, area);
        let head = app.view.workspace_project_header_areas[0].rect;
        let text = row_text(&buffer, head.y, area.width);
        assert!(
            text.contains("▸") && text.contains("·"),
            "a folded project shows its chevron and one aggregate dot; got {text:?}"
        );
    }

    // TP-ICON-01/02: the module header wears its rule's icon, one step in
    // under a project.
    #[test]
    fn group_header_under_a_project_is_indented_and_shows_its_rule_icon() {
        let mut app = AppState::test_new();
        // Below the threshold the desktop sidebar is never laid out and every
        // area vector stays empty — the narrow-area-counts-as-mobile trap.
        app.mobile_width_threshold = 0;
        app.workspaces = vec![
            worktree_on_branch("alpha", "feat/t4f-alpha"),
            worktree_on_branch("beta", "feat/t4f-beta"),
        ];
        let mut rule = split_rule(&["feat/t4f-*"], "herdr:t4f", "T4F BAM");
        rule.icon = Some("🌐".into());
        app.space_split_rules = vec![rule];
        app.space_projects = vec![project_over("project:herdr", &["/repo/herdr"], &[])];

        let area = Rect::new(0, 0, 40, 24);
        let buffer = draw_tree(&mut app, area);
        let head = app.view.workspace_group_header_areas[0].rect;
        let text = squeezed_row_text(&buffer, head.y, area.width);
        assert!(
            text.contains("▾ 🌐 T4F BAM"),
            "module chevron, icon, label in order; got {text:?}"
        );
        assert_eq!(
            find_symbol_x(&buffer, head.y, area.width, "▾"),
            head.x + ROW_INDENT_STEP,
            "under a project the module header steps in once"
        );
    }

    // TP-ICON-01: workspace rows carry the branch glyph, chats the chat glyph.
    #[test]
    fn workspace_and_chat_rows_carry_their_kind_icons() {
        let (mut app, key) = app_with_chat_drawer(1);
        app.expanded_chat_workspaces.insert(key);

        let area = Rect::new(0, 0, 40, 24);
        let buffer = draw_tree(&mut app, area);
        let card = app.view.workspace_card_areas[0];
        let chat = app.view.workspace_chat_row_areas[0].clone();
        assert!(
            row_text(&buffer, card.rect.y, area.width).contains(''),
            "a checkout row carries the branch glyph"
        );
        assert!(
            row_text(&buffer, chat.rect.y, area.width).contains('💬'),
            "a chat row carries the chat glyph"
        );
    }

    // TP-PROJ-GROUP-01: everything under a project steps in by one level.
    #[test]
    fn workspace_rows_shift_one_step_under_a_project() {
        let area = Rect::new(0, 0, 40, 24);

        let mut plain = app_with_worktree_tree(40);
        let plain_buffer = draw_tree(&mut plain, area);
        let plain_card = plain.view.workspace_card_areas[0];
        let plain_x = find_symbol_x(&plain_buffer, plain_card.rect.y, area.width, "");

        let mut nested = app_with_worktree_tree(40);
        nested.space_projects = vec![project_over("project:herdr", &["/repo/herdr"], &[])];
        let nested_buffer = draw_tree(&mut nested, area);
        let nested_card = nested.view.workspace_card_areas[0];
        let nested_x = find_symbol_x(&nested_buffer, nested_card.rect.y, area.width, "");

        assert_eq!(
            nested_x,
            plain_x + ROW_INDENT_STEP,
            "the same checkout renders one step deeper inside a project"
        );
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

    /// A repository with two checkouts, the main one active, each with an open
    /// chat drawer — the shape the whole tree exists to render.
    fn app_with_worktree_tree(width: u16) -> AppState {
        let mut app = AppState::test_new();
        let mut main = Workspace::test_new("herdr");
        main.worktree_space = Some(crate::workspace::WorktreeSpaceMembership {
            key: "repo-key".into(),
            label: "herdr".into(),
            repo_root: std::path::PathBuf::from("/repo/herdr"),
            checkout_path: std::path::PathBuf::from("/repo/herdr"),
            is_linked_worktree: false,
        });
        main.custom_name = None;
        main.identity_cwd = std::path::PathBuf::from("/repo/herdr");
        main.cached_git_branch = Some("master".into());
        let mut child = Workspace::test_new("fix");
        child.worktree_space = Some(crate::workspace::WorktreeSpaceMembership {
            key: "repo-key".into(),
            label: "herdr".into(),
            repo_root: std::path::PathBuf::from("/repo/herdr"),
            checkout_path: std::path::PathBuf::from("/repo/herdr-fix"),
            is_linked_worktree: true,
        });
        child.custom_name = None;
        child.identity_cwd = std::path::PathBuf::from("/repo/herdr-fix");
        child.cached_git_branch = Some("fix/clipboard".into());
        app.workspaces = vec![main, child];
        app.active = Some(0);
        app.selected = 0;
        app.mobile_width_threshold = 0;
        app.mouse_capture = true;
        app.sidebar_width = width;
        app.sidebar_min_width = 10;

        for path in ["/repo/herdr", "/repo/herdr-fix"] {
            let key = crate::persist::workspace_chats::ledger_key(&std::path::PathBuf::from(path));
            app.workspace_chat_rows.insert(
                key.clone(),
                vec![crate::app::state::WorkspaceChatRow {
                    session_id: format!("session-{path}"),
                    agent: "claude".to_string(),
                    title: Some("remembered chat".to_string()),
                    last_seen_ms: 1_000,
                    last_modified: None,
                }],
            );
            app.expanded_chat_workspaces.insert(key);
        }
        app
    }

    /// Draw the sidebar and hand back its rows as plain strings.
    fn drawn_sidebar_rows(app: &mut AppState, height: u16) -> Vec<String> {
        let area = Rect::new(0, 0, 100, height);
        crate::ui::compute_view(app, area);
        let backend = ratatui::backend::TestBackend::new(100, height);
        let mut terminal = ratatui::Terminal::new(backend).expect("test terminal");
        let registry = TerminalRuntimeRegistry::new();
        terminal
            .draw(|frame| render_sidebar(app, &registry, frame, app.view.sidebar_rect))
            .expect("draw");
        let buffer = terminal.backend().buffer();
        let width = app.view.sidebar_rect.width;
        (0..height)
            .map(|y| (0..width).map(|x| buffer[(x, y)].symbol()).collect())
            .collect()
    }

    // TP-TREE-08: the failure that started this work, pinned. Two disclosure
    // arrows side by side in one row means two different controls one column
    // apart, and no reader can tell which one folds the repository and which
    // one opens the chats. The tree makes it structurally impossible; this
    // asserts it stays impossible.
    #[test]
    fn no_row_ever_carries_two_disclosure_arrows_side_by_side() {
        for width in [18u16, 26, 32] {
            let mut app = app_with_worktree_tree(width);
            let rows = drawn_sidebar_rows(&mut app, 26);
            for row in &rows {
                for pair in [
                    format!("{DISCLOSURE_OPEN}{DISCLOSURE_OPEN}"),
                    format!("{DISCLOSURE_OPEN}{DISCLOSURE_CLOSED}"),
                    format!("{DISCLOSURE_CLOSED}{DISCLOSURE_OPEN}"),
                    format!("{DISCLOSURE_CLOSED}{DISCLOSURE_CLOSED}"),
                ] {
                    assert!(
                        !row.contains(&pair),
                        "width {width}: two disclosures collided in {row:?}"
                    );
                }
            }
        }
    }

    // TP-TREE-09 + TP-TREE-10: the repository owns column 0; a checkout's own
    // arrow sits one level in; and the drawer hangs off that arrow's column on
    // a rule, so the chats read as contained rather than floating.
    #[test]
    fn the_tree_puts_the_repository_the_checkouts_and_the_chats_on_their_own_depths() {
        let mut app = app_with_worktree_tree(32);
        let rows = drawn_sidebar_rows(&mut app, 26);

        let header = rows
            .iter()
            .find(|row| row.contains("herdr") && !row.contains('·'))
            .expect("the repository header is drawn");
        assert!(
            header.starts_with(DISCLOSURE_OPEN),
            "the repository owns column 0: {header:?}"
        );

        let checkout = rows
            .iter()
            .find(|row| row.contains("master"))
            .expect("the checkout is drawn");
        assert_eq!(
            checkout.find(DISCLOSURE_OPEN),
            Some(ROW_INDENT_STEP as usize),
            "a checkout's arrow sits at the checkout's depth, never in the \
             repository's column: {checkout:?}"
        );

        let chat = rows
            .iter()
            .find(|row| row.contains("remembered chat"))
            .expect("the drawer row is drawn");
        assert_eq!(
            chat.find(DRAWER_GUIDE),
            Some(ROW_INDENT_STEP as usize),
            "the drawer's rule runs down its checkout's arrow column: {chat:?}"
        );
    }

    // TP-TREE-11: the workspace you are in wears the accent outright — the
    // same sentence the active tab and the active agent card speak. A chat row
    // never takes it, so "which workspace am I in" stays a single answer.
    #[test]
    fn the_active_workspace_row_wears_the_accent_and_a_chat_row_never_does() {
        let mut app = app_with_worktree_tree(32);
        let area = Rect::new(0, 0, 100, 26);
        crate::ui::compute_view(&mut app, area);
        let backend = ratatui::backend::TestBackend::new(100, 26);
        let mut terminal = ratatui::Terminal::new(backend).expect("test terminal");
        let registry = TerminalRuntimeRegistry::new();
        terminal
            .draw(|frame| render_sidebar(&app, &registry, frame, app.view.sidebar_rect))
            .expect("draw");
        let buffer = terminal.backend().buffer();
        let accent = app.palette.accent;
        let width = app.view.sidebar_rect.width;

        let active = app
            .view
            .workspace_card_areas
            .iter()
            .find(|card| Some(card.ws_idx) == app.active)
            .expect("the active workspace is laid out");
        assert!(
            (active.rect.x..active.rect.x + active.rect.width)
                .all(|x| buffer[(x, active.rect.y)].bg == accent),
            "the active row is filled with the accent"
        );
        for row in &app.view.workspace_chat_row_areas {
            assert!(
                (0..width).all(|x| buffer[(x, row.rect.y)].bg != accent),
                "a chat row must never take the accent background"
            );
        }
    }

    // TP-TREE-12: every checkout offers "start a chat here" — that is what the
    // row is for — and a checkout with history says how much of it there is.
    #[test]
    fn every_checkout_offers_a_plus_and_reports_how_many_chats_it_remembers() {
        let mut app = app_with_worktree_tree(32);
        let rows = drawn_sidebar_rows(&mut app, 26);

        // The branch prefix is dropped on the way to the screen (TP-DRAW-08),
        // so "fix/clipboard" is drawn as "clipboard".
        let checkout_rows: Vec<&String> = rows
            .iter()
            .filter(|row| row.contains("master") || row.contains("clipboard"))
            .collect();
        assert_eq!(checkout_rows.len(), 2, "both checkouts are drawn");
        for row in checkout_rows {
            assert!(row.contains('+'), "a checkout offers a new chat: {row:?}");
            assert!(
                row.contains('1'),
                "a checkout with one remembered chat says so: {row:?}"
            );
        }
    }

    // TP-TREE-13: the narrowest configurable sidebar still lays out. Depth is
    // charged in columns, so a width the design never tried is exactly where a
    // prefix wider than the row would panic or wrap.
    #[test]
    fn the_narrowest_sidebar_still_draws_the_tree() {
        let mut app = app_with_worktree_tree(10);
        let rows = drawn_sidebar_rows(&mut app, 26);
        let width = app.view.sidebar_rect.width as usize;

        assert!(
            width >= 1,
            "a sidebar is still laid out at the minimum width"
        );
        assert!(
            rows.iter()
                .all(|row| crate::ui::text::display_width(row) <= width),
            "no row may overflow the sidebar at the minimum width"
        );
    }

    // TP-DRAW-08 + TP-DRAW-09: in a column this narrow the conventional-commit
    // prefix is pure cost — every row spends five columns saying "feat/" and
    // the part that tells the branches apart is what gets truncated. Only a
    // closed set is dropped, and only when something is left.
    #[test]
    fn a_known_branch_prefix_is_dropped_and_a_chosen_namespace_is_kept() {
        for (branch, expected) in [
            ("feat/t4f-reactive-refresh", "t4f-reactive-refresh"),
            ("fix/clipboard-nonblocking", "clipboard-nonblocking"),
            ("chore/bump-deps", "bump-deps"),
            ("worktree/Tiling", "Tiling"),
            ("release/0.8.0", "0.8.0"),
        ] {
            assert_eq!(strip_branch_prefix(branch), expected, "branch {branch:?}");
        }

        for kept in [
            // A namespace the person chose themselves is information.
            "codex/user-task-isolation",
            "cypack/mnmveldops",
            // Nothing would be left, and a blank row is worse than a noisy one.
            "feat/",
            "fix/",
            // No prefix at all.
            "master",
        ] {
            assert_eq!(
                strip_branch_prefix(kept),
                kept,
                "branch {kept:?} must survive untouched"
            );
        }
    }

    // TP-TREE-01: the repository takes a row of its own. This is the whole
    // point of the tree: while the group's parent checkout doubled as the
    // header, the "show the siblings" arrow and the "show my chats" arrow both
    // landed in the same gutter one column apart, and no reader could tell
    // which arrow did what.
    #[test]
    fn a_repository_with_two_checkouts_gets_a_header_row_of_its_own() {
        let mut app = AppState::test_new();
        app.workspaces = vec![
            workspace_with_worktree_space("main", Some("repo-key"), "/repo/herdr"),
            workspace_with_worktree_space("issue", Some("repo-key"), "/repo/herdr-issue"),
        ];

        let entries = workspace_list_entries(&app);
        assert!(
            matches!(entries.first(), Some(WorkspaceListEntry::GroupHeader { space_key }) if space_key == "repo-key"),
            "the repository leads its own block: {entries:?}"
        );
        // TP-TREE-04: and every checkout, the main one included, is a child.
        assert!(
            entries
                .iter()
                .skip(1)
                .all(|entry| matches!(entry, WorkspaceListEntry::Workspace { indented: true, .. })),
            "no checkout may stay at the header's depth: {entries:?}"
        );
    }

    // TP-TREE-02: a repository with a single checkout gets no header. With a
    // dozen-plus workspaces, a header each would double the vertical cost of
    // the list for no information — there is no group to fold.
    #[test]
    fn a_lone_checkout_gets_no_header_row() {
        let mut app = AppState::test_new();
        app.workspaces = vec![
            workspace_with_worktree_space("main", Some("repo-key"), "/repo/herdr"),
            Workspace::test_new("notes"),
        ];

        let entries = workspace_list_entries(&app);
        assert!(
            !entries
                .iter()
                .any(|entry| matches!(entry, WorkspaceListEntry::GroupHeader { .. })),
            "a single checkout is not a group: {entries:?}"
        );
    }

    // TP-TREE-06: the header row has to be measured like any other row. A row
    // that is drawn but not counted makes the list scroll past itself — the
    // failure the drawer rows already paid for once (TP-WSCHAT-17).
    #[test]
    fn the_scroll_metrics_count_the_header_row() {
        let mut app = AppState::test_new();
        app.workspaces = vec![
            workspace_with_worktree_space("main", Some("repo-key"), "/repo/herdr"),
            workspace_with_worktree_space("issue", Some("repo-key"), "/repo/herdr-issue"),
        ];
        app.sidebar_spaces.rows = vec![vec![crate::config::SpaceSidebarToken::Workspace]];

        let area = Rect::new(0, 0, 30, 20);
        let (cards, _, group_headers, _) = compute_workspace_list_areas(&app, area);
        let drawn_rows: u16 = cards.iter().map(|card| card.rect.height).sum::<u16>()
            + group_headers
                .iter()
                .map(|head| head.rect.height)
                .sum::<u16>();
        let counted_rows: u16 = (0..workspace_list_entries(&app).len())
            .filter_map(|idx| {
                entry_row_metrics(&app, &workspace_list_entries(&app), idx, area.height)
            })
            .map(|(height, _)| height)
            .sum();

        assert_eq!(
            drawn_rows, counted_rows,
            "every drawn row must also be a counted row"
        );
    }

    // TP-TREE-18: the mobile switcher lays out a fixed number of rows per
    // workspace. A header leaking into that list would shift every position
    // after it, so the switcher would select the wrong workspace.
    #[test]
    fn the_mobile_drawer_sees_the_group_header() {
        let mut app = AppState::test_new();
        app.workspaces = vec![
            workspace_with_worktree_space("main", Some("repo-key"), "/repo/herdr"),
            workspace_with_worktree_space("issue", Some("repo-key"), "/repo/herdr-issue"),
        ];
        app.mobile_drawer = crate::app::state::MobileDrawer::Spaces;

        let rows = crate::ui::mobile_drawer_rows(&app);
        assert_eq!(
            rows.iter()
                .filter(|row| matches!(row.content, crate::ui::DrawerRowContent::SpaceGroup { .. }))
                .count(),
            1,
            "the repository row names what the checkouts under it belong to"
        );
        assert_eq!(
            rows.iter()
                .filter(|row| matches!(row.content, crate::ui::DrawerRowContent::Space { .. }))
                .count(),
            2,
            "and the checkouts are still there"
        );
    }

    #[test]
    fn parent_workspace_row_stays_clickable_when_grouped() {
        let mut app = AppState::test_new();
        app.workspaces = vec![
            workspace_with_worktree_space("main", Some("repo-key"), "/repo/herdr"),
            workspace_with_worktree_space("issue", Some("repo-key"), "/repo/herdr-issue"),
        ];
        app.sidebar_spaces.row_gap = 1;

        let (cards, chat_rows, group_headers, _) =
            compute_workspace_list_areas(&app, Rect::new(0, 0, 30, 20));

        assert!(chat_rows.is_empty());
        // TP-TREE-04: the main checkout is now a child of the repository row,
        // but it is still a card of its own — the click target survived the
        // move. Losing it would strand the main checkout behind the header.
        assert_eq!(cards[0].ws_idx, 0);
        assert!(
            cards[0].indented,
            "every checkout sits under the repository"
        );
        assert_eq!(cards[1].ws_idx, 1);
        assert!(cards[1].indented);
        // TP-TREE-06: siblings stay compact; the configured gap separates
        // blocks, not the members of one block.
        assert_eq!(cards[1].rect.y, cards[0].rect.y + cards[0].rect.height);
        // TP-TREE-05/07: the header is laid out, in its own vector, above the
        // checkouts it introduces.
        assert_eq!(group_headers.len(), 1);
        assert_eq!(group_headers[0].space_key, "repo-key");
        assert!(group_headers[0].rect.y < cards[0].rect.y);
    }

    #[test]
    fn space_row_gap_preserves_compact_worktree_children() {
        let mut app = AppState::test_new();
        app.workspaces = vec![
            workspace_with_worktree_space("main", Some("repo-key"), "/repo/herdr"),
            workspace_with_worktree_space("issue", Some("repo-key"), "/repo/herdr-issue"),
            workspace_with_worktree_space("review", Some("repo-key"), "/repo/herdr-review"),
            Workspace::test_new("notes"),
        ];
        app.sidebar_spaces.rows = vec![vec![crate::config::SpaceSidebarToken::Workspace]];
        app.sidebar_spaces.row_gap = 2;

        let (spacious, _, _headers, _) =
            compute_workspace_list_areas(&app, Rect::new(0, 0, 30, 30));
        // TP-TREE-06: the three checkouts of one repository are one block and
        // stay compact; the gap falls where the next top-level unit begins.
        assert_eq!(
            spacious[1].rect.y,
            spacious[0].rect.y + spacious[0].rect.height
        );
        assert_eq!(
            spacious[2].rect.y,
            spacious[1].rect.y + spacious[1].rect.height
        );
        assert_eq!(
            spacious[3].rect.y,
            spacious[2].rect.y + spacious[2].rect.height + 2
        );
        // The three checkouts no longer pay a gap each, so more of the block
        // fits in the same height.
        let spacious_metrics = workspace_list_scroll_metrics(&app, Rect::new(0, 0, 30, 7));
        assert_eq!(spacious_metrics.viewport_rows, 4);
        assert_eq!(spacious_metrics.max_offset_from_bottom, 3);

        app.sidebar_spaces.row_gap = 0;
        let (packed, _, _headers, _) = compute_workspace_list_areas(&app, Rect::new(0, 0, 30, 30));
        assert!(packed
            .windows(2)
            .all(|pair| pair[1].rect.y == pair[0].rect.y + pair[0].rect.height));
        let packed_metrics = workspace_list_scroll_metrics(&app, Rect::new(0, 0, 30, 7));
        assert_eq!(packed_metrics.viewport_rows, 4);
        // The repository header costs the one row that used to make the whole
        // list fit; scrolling by one now reaches the top.
        assert_eq!(packed_metrics.max_offset_from_bottom, 1);
    }

    #[test]
    fn packed_workspace_drag_indicator_overlays_an_internal_boundary() {
        let mut app = AppState::test_new();
        app.workspaces = vec![
            Workspace::test_new("a"),
            Workspace::test_new("b"),
            Workspace::test_new("c"),
        ];
        app.sidebar_spaces.rows = vec![vec![crate::config::SpaceSidebarToken::Workspace]];
        app.sidebar_spaces.row_gap = 0;
        let area = Rect::new(0, 0, 30, 20);
        app.view.workspace_card_areas = compute_workspace_card_areas(&app, area);
        let list_area = workspace_list_rect(area, app.sidebar_section_split);
        let indicator_row =
            workspace_drop_indicator_row(&app.view.workspace_card_areas, list_area, 2).unwrap();
        assert_eq!(indicator_row, app.view.workspace_card_areas[1].rect.y);
        app.drag = Some(crate::app::state::DragState {
            target: crate::app::state::DragTarget::WorkspaceReorder {
                source_ws_idx: 0,
                insert_idx: Some(2),
            },
        });

        let mut terminal = Terminal::new(TestBackend::new(area.width, area.height)).unwrap();
        terminal
            .draw(|frame| {
                render_workspace_list(
                    &app,
                    &TerminalRuntimeRegistry::new(),
                    frame,
                    list_area,
                    false,
                )
            })
            .unwrap();

        assert_eq!(
            terminal.backend().buffer()[(list_area.x, indicator_row)].symbol(),
            "─"
        );
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
    fn compact_space_group_scroll_clamps_when_all_entries_fit() {
        let mut app = AppState::test_new();
        app.workspaces = vec![
            workspace_with_worktree_space("main", Some("repo-key"), "/repo/herdr"),
            workspace_with_worktree_space("one", Some("repo-key"), "/repo/herdr-one"),
            workspace_with_worktree_space("two", Some("repo-key"), "/repo/herdr-two"),
        ];
        let area = Rect::new(0, 0, 30, 20);
        app.workspace_scroll = normalized_workspace_scroll(&app, area, 2);

        let (cards, headers, _headers, _) = compute_workspace_list_areas(&app, area);

        assert!(headers.is_empty());
        assert_eq!(app.workspace_scroll, 0);
        assert_eq!(cards.len(), 3);
        assert_eq!(cards[2].ws_idx, 2);
    }

    #[test]
    fn workspace_scroll_metrics_count_display_entries_not_raw_workspaces() {
        let mut app = AppState::test_new();
        app.workspaces = vec![
            workspace_with_worktree_space("main", Some("repo-key"), "/repo/herdr"),
            workspace_with_worktree_space("issue", Some("repo-key"), "/repo/herdr-issue"),
            Workspace::test_new("notes"),
        ];
        for workspace in &mut app.workspaces {
            workspace.cached_git_branch = Some("main".into());
        }
        app.collapsed_space_keys.insert("repo-key".into());
        app.active = None;
        app.mode = Mode::Terminal;

        let ws_area = Rect::new(0, 0, 30, 6);
        let metrics = workspace_list_scroll_metrics(&app, ws_area);

        // TP-TREE-06: the folded group is a one-line header, so it and the
        // ungrouped workspace both fit — the metric still counts display
        // entries rather than raw workspaces, which is this test's subject.
        assert_eq!(metrics.viewport_rows, 2);
        assert_eq!(metrics.max_offset_from_bottom, 0);
        assert_eq!(metrics.offset_from_bottom, 0);
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

        let (cards, headers, _headers, _) =
            compute_workspace_list_areas(&app, Rect::new(0, 0, 30, 12));

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
                WorkspaceListEntry::GroupHeader {
                    space_key: "repo-key".into(),
                },
                WorkspaceListEntry::Workspace {
                    ws_idx: 0,
                    indented: true,
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
                WorkspaceListEntry::GroupHeader {
                    space_key: "repo-key".into(),
                },
                WorkspaceListEntry::Workspace {
                    ws_idx: 0,
                    indented: true,
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
                WorkspaceListEntry::GroupHeader {
                    space_key: "repo-key".into(),
                },
                WorkspaceListEntry::Workspace {
                    ws_idx: 0,
                    indented: true,
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
                WorkspaceListEntry::GroupHeader {
                    space_key: "repo-key".into(),
                },
                WorkspaceListEntry::Workspace {
                    ws_idx: 1,
                    indented: true,
                },
            ]
        );

        app.active = None;
        app.mode = Mode::Terminal;
        // TP-TREE-03: with nothing in the group active, a folded group is one
        // row — its repository header.
        assert_eq!(
            workspace_list_entries(&app),
            vec![WorkspaceListEntry::GroupHeader {
                space_key: "repo-key".into(),
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
                WorkspaceListEntry::GroupHeader {
                    space_key: "repo-key".into(),
                },
                WorkspaceListEntry::Workspace {
                    ws_idx: 1,
                    indented: true,
                },
            ]
        );
    }
}
