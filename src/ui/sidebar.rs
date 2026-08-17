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

// TP-CHROME-12/13: framing one half leaves the other alone, and a half with no
// room for a frame keeps its content instead of its decoration.
pub(crate) fn expanded_sidebar_sections(
    area: Rect,
    split_ratio: f32,
    chrome: crate::ui::shell::SidebarChrome,
) -> (Rect, Rect) {
    let content = Rect::new(area.x, area.y, area.width.saturating_sub(1), area.height);
    if content.width == 0 || content.height == 0 {
        return (Rect::default(), Rect::default());
    }

    let (ws_h, detail_h) = sidebar_section_heights(content.height, split_ratio);
    let ws_area = Rect::new(content.x, content.y, content.width, ws_h);
    let detail_area = Rect::new(content.x, content.y + ws_h, content.width, detail_h);
    // The inset lands here, in the one place both section rectangles come from,
    // so what is painted and what a click resolves to can never disagree.
    (
        section_content_rect(ws_area, chrome.spaces),
        section_content_rect(detail_area, chrome.agents),
    )
}

/// What a framed section leaves for its content.
///
/// A section too small to hold a frame keeps its full rectangle instead: losing
/// the border on a short panel is a cosmetic disappointment, losing the panel
/// is not.
pub(crate) fn section_content_rect(outer: Rect, tint: Option<crate::ui::shell::BarTint>) -> Rect {
    if tint.is_none() || outer.width < 3 || outer.height < 3 {
        return outer;
    }
    Rect::new(outer.x + 1, outer.y + 1, outer.width - 2, outer.height - 2)
}

/// The outer rectangles, before any frame took its cells — what the frames
/// themselves are drawn into.
pub(crate) fn expanded_sidebar_section_frames(area: Rect, split_ratio: f32) -> (Rect, Rect) {
    let content = Rect::new(area.x, area.y, area.width.saturating_sub(1), area.height);
    if content.width == 0 || content.height == 0 {
        return (Rect::default(), Rect::default());
    }
    let (ws_h, detail_h) = sidebar_section_heights(content.height, split_ratio);
    (
        Rect::new(content.x, content.y, content.width, ws_h),
        Rect::new(content.x, content.y + ws_h, content.width, detail_h),
    )
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

pub(crate) fn agent_panel_toggle_rect(
    area: Rect,
    sort: AgentPanelSort,
    chrome: crate::ui::shell::SidebarChrome,
) -> Rect {
    agent_panel_header_label_rect(area, agent_panel_sort_label(sort), chrome)
}

/// Where a header control sits, top-right of the agents half.
///
/// The header already reserves [`AGENT_PANEL_HEADER_ROWS`] rows, which is
/// exactly what a chip needs, so asking for chips costs the list nothing here —
/// the frame simply takes the rows the separator and its blank line were using.
// TP-CHROME-20/21: the header's controls become corner chips, hit-tested where
// they were drawn.
fn agent_panel_header_label_rect(
    area: Rect,
    label: &str,
    chrome: crate::ui::shell::SidebarChrome,
) -> Rect {
    if area.width == 0 || area.height < 2 {
        return Rect::default();
    }

    let width = chrome
        .control_width(display_width_u16(label), label)
        .min(area.width);
    let (y, height) = if chrome.chips.is_some() {
        (area.y, crate::ui::widgets::CHIP_ROWS.min(area.height))
    } else {
        (area.y + 1, 1)
    };
    Rect::new(area.x + area.width.saturating_sub(width), y, width, height)
}

/// The mirror of [`agent_panel_header_label_rect`] on the left edge: where the
/// half's own name is written.
fn agent_panel_header_name_rect(
    area: Rect,
    label: &str,
    chrome: crate::ui::shell::SidebarChrome,
) -> Rect {
    if area.width == 0 || area.height < 2 {
        return Rect::default();
    }

    let width = chrome
        .control_width(display_width_u16(label), label)
        .min(area.width);
    let (y, height) = if chrome.chips.is_some() {
        (area.y, crate::ui::widgets::CHIP_ROWS.min(area.height))
    } else {
        (area.y + 1, 1)
    };
    Rect::new(area.x, y, width, height)
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

/// The tree node behind a header row's key, whichever table authored it.
pub(crate) fn node_for_key<'a>(
    app: &'a AppState,
    node_key: &str,
) -> Option<&'a crate::spaces::SpaceNode> {
    app.space_nodes.iter().find(|node| node.key == node_key)
}

/// What a container header row draws: its name, its glyph, and whether it has
/// membership of its own to summarise while folded.
///
/// One header row type carries two kinds of key. `[[spaces.project]]` and
/// `[[spaces.node]]` are the same shape to a reader — an arrow, a glyph, a
/// name, children underneath — and the renderer asks this one question rather
/// than growing a second painter that would drift from the first (K1).
pub(crate) struct HeaderFace<'a> {
    pub(crate) name: &'a str,
    pub(crate) icon: Option<&'a str>,
    /// A project folded over its checkouts answers for them; a module holds
    /// no membership, so a state dot on its row would be a summary of
    /// nothing (K2).
    pub(crate) aggregates_state: bool,
}

/// Resolve a header row's key against both container tables.
///
/// Returning `None` used to be the common case for module rows: the row was
/// laid out from the node forest and looked up against projects only, so it
/// took its line and painted nothing. A blank line in a tree reads as damage,
/// and the module the user had just named was nowhere on screen. The "older
/// chats" row lost a release to the same shape.
pub(crate) fn header_face_for_key<'a>(app: &'a AppState, key: &str) -> Option<HeaderFace<'a>> {
    if let Some(project) = project_for_key(app, key) {
        return Some(HeaderFace {
            name: project.name.as_str(),
            icon: project.icon.as_deref(),
            aggregates_state: true,
        });
    }
    let node = node_for_key(app, key)?;
    Some(HeaderFace {
        name: node.name.as_str(),
        icon: node.icon.as_deref(),
        aggregates_state: false,
    })
}

/// How many indent steps `node_key` sits below the top, capped at six (K10):
/// a deeper tree keeps folding correctly, it just stops eating name columns
/// on a 76-column phone.
pub(crate) fn node_depth(app: &AppState, node_key: &str) -> u16 {
    // The chain may run through buckets (TP-NODE-08): every edge is read
    // through the one tree-edge reader so mixed chains count fully.
    let guard = app.space_nodes.len() + app.space_split_rules.len();
    let mut depth: u16 = 0;
    let mut current =
        crate::spaces::tree_parent_of(&app.space_nodes, &app.space_split_rules, node_key);
    while let Some(key) = current {
        depth += 1;
        if depth as usize > guard {
            break; // The forest is validated; belt and braces.
        }
        current = crate::spaces::tree_parent_of(&app.space_nodes, &app.space_split_rules, key);
    }
    depth.min(6)
}

/// The node a workspace's space hangs under, if any: the claiming rule's own
/// `parent` first, the claiming project second — the same chain the emitter
/// walks, answered per workspace for the indent and hit-test paths.
pub(crate) fn workspace_parent_node_key(app: &AppState, ws_idx: usize) -> Option<String> {
    let space = effective_space(app, ws_idx)?;
    let ws = app.workspaces.get(ws_idx)?;
    let membership = ws.worktree_space()?;
    let rule = crate::spaces::resolve_space_rule(
        &app.space_split_rules,
        &membership.repo_root,
        &membership.checkout_path,
        ws.branch().as_deref(),
    );
    crate::spaces::resolve_space_parent(
        rule,
        &app.space_projects,
        &space.key,
        Some(&membership.repo_root),
    )
    .filter(|parent| {
        node_for_key(app, parent).is_some()
            || app
                .space_projects
                .iter()
                .any(|project| &project.key == parent)
    })
}

/// Indent steps the node chain adds to a workspace row. Yesterday's
/// "inside a project adds one step" is the depth-one case of this.
pub(crate) fn workspace_node_shift(app: &AppState, ws_idx: usize) -> u16 {
    match workspace_parent_node_key(app, ws_idx) {
        Some(parent) => (node_depth(app, &parent) + 1).min(6),
        None => 0,
    }
}

/// Indent steps the node chain adds to a space's own header row.
pub(crate) fn space_node_shift_for_key(app: &AppState, space_key: &str) -> u16 {
    (0..app.workspaces.len())
        .find(|idx| effective_space(app, *idx).is_some_and(|space| space.key == space_key))
        .map(|idx| workspace_node_shift(app, idx))
        .unwrap_or(0)
}

/// The owner a bucket header hangs under: the claiming rule's own parent
/// first, the claiming project second — the single-key form of the walk the
/// emitter does per space (TP-DOTS-12).
pub(crate) fn space_owner_for_key(app: &AppState, space_key: &str) -> Option<String> {
    let ws_idx = (0..app.workspaces.len())
        .find(|idx| effective_space(app, *idx).is_some_and(|space| space.key == space_key))?;
    let ws = app.workspaces.get(ws_idx)?;
    let membership = ws.worktree_space()?;
    let rule = crate::spaces::resolve_space_rule(
        &app.space_split_rules,
        &membership.repo_root,
        &membership.checkout_path,
        ws.branch().as_deref(),
    );
    crate::spaces::resolve_space_parent(
        rule,
        &app.space_projects,
        space_key,
        Some(&membership.repo_root),
    )
}

/// The workspace a module's "New branch..." should branch from: a direct
/// member of the module first, then any workspace whose bucket chain runs
/// through it, then the same search up the module's own ancestor chain — a
/// freshly created (still invisible) module borrows its ancestors' repo
/// (TP-DOTS-14).
/// What a module can start a branch from.
///
/// TP-MOD-37: "New branch..." has been on every module header since TP-DOTS-13,
/// but it only ever looked for a checkout already standing under the module.
/// A module the person gave a directory to — which TP-MOD-33 made possible and
/// TP-MOD-35 gave a picker for — had nothing to offer, so the verb answered
/// "move a branch under it first" and stopped there. That is a dead end wearing
/// the clothes of an explanation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ModuleBranchSource {
    /// A checkout already stands under this module. Today's behaviour, and it
    /// wins over everything below: a checkout under the module is a more
    /// specific answer than the module's own directory, and quietly preferring
    /// the directory would move the branch somewhere else than before.
    Workspace(usize),
    /// The module's own directory IS a git repository root.
    Repository(std::path::PathBuf),
    /// The directory is there but is not a repository root — the one case that
    /// can be fixed on the spot, by initialising it.
    UninitializedDirectory(std::path::PathBuf),
    /// Nothing stated, or what was stated is no longer on disk.
    NoDirectory,
}

/// Whether `dir` is a git repository ROOT.
///
/// ⛔ TP-MOD-37: deliberately NOT `git rev-parse --show-toplevel`, and
/// deliberately no walk up the parents. Measured on the reporting machine:
///
/// ```text
/// /home/user/Marktplaats satis        exists, empty, no .git
/// git -C "/home/user/Marktplaats satis" rev-parse --show-toplevel  →  /home/user
/// ```
///
/// `$HOME` is itself a git repository there. A climbing check would have called
/// that module a repository and opened branches and worktrees in the home
/// directory — over `~/.claude`, `~/.config` and every project in it. Only the
/// directory the person named counts, and only if it is a root.
///
/// `.git` is tested with `exists()` rather than `is_dir()` because a linked
/// worktree and a submodule both carry a `.git` FILE, and both are perfectly
/// good places to branch from.
pub(crate) fn is_git_repository_root(dir: &std::path::Path) -> bool {
    dir.join(".git").exists()
}

/// TP-MOD-37: the four answers, in the order that keeps today's behaviour safe.
pub(crate) fn module_branch_source(app: &AppState, module_key: &str) -> ModuleBranchSource {
    if let Some(ws_idx) = worktree_source_for_module(app, module_key) {
        return ModuleBranchSource::Workspace(ws_idx);
    }
    let Some(dir) = app.module_directory_for_key(module_key) else {
        return ModuleBranchSource::NoDirectory;
    };
    // Checked here and not only when it was written: a directory can be removed
    // afterwards — a worktree pruned, a disk unmounted — and TP-CHAT-MOVE-10
    // (R3) already pays this exact toll on the chat side.
    if !dir.is_dir() {
        return ModuleBranchSource::NoDirectory;
    }
    if is_git_repository_root(&dir) {
        ModuleBranchSource::Repository(dir)
    } else {
        ModuleBranchSource::UninitializedDirectory(dir)
    }
}

pub(crate) fn worktree_source_for_module(app: &AppState, module_key: &str) -> Option<usize> {
    let chain_hits_module = |idx: usize, target: &str| -> bool {
        let Some(space) = effective_space(app, idx) else {
            return false;
        };
        if space.key == target {
            return true;
        }
        let mut current = space_owner_for_key(app, &space.key);
        let mut steps = 0usize;
        while let Some(key) = current {
            if key == target {
                return true;
            }
            steps += 1;
            if steps > app.space_nodes.len() + app.space_split_rules.len() {
                break;
            }
            current = crate::spaces::tree_parent_of(&app.space_nodes, &app.space_split_rules, &key)
                .map(str::to_string);
        }
        false
    };

    let mut target = module_key.to_string();
    let mut climbed = 0usize;
    loop {
        if let Some(idx) = (0..app.workspaces.len()).find(|&idx| chain_hits_module(idx, &target)) {
            return Some(idx);
        }
        let parent =
            crate::spaces::tree_parent_of(&app.space_nodes, &app.space_split_rules, &target)?;
        target = parent.to_string();
        climbed += 1;
        if climbed > app.space_nodes.len() + app.space_split_rules.len() {
            return None;
        }
    }
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

/// The label an indented row wears.
///
/// TP-DAILY-14: `indented` says a row is drawn one level in, and it was
/// carrying a second meaning it never earned — "this row is a checkout under a
/// repository header", which is what makes a branch name the right label
/// there. The daily area's own rows share the flag and nothing else: they have
/// no checkout, so the branch they showed came from whatever repository
/// happened to contain the daily directory.
pub(crate) fn indented_row_label(
    label: &str,
    branch: Option<&str>,
    has_custom_name: bool,
    daily_name: Option<&str>,
) -> String {
    // TP-DAILY-14: a daily row has no checkout, so the branch it used to show
    // came from whatever repository happened to contain the daily directory —
    // on the machine this was reported from, `$HOME`, and seven rows all read
    // `main`. The name arrives already resolved (TP-DAILY-15/16) because
    // telling the rows apart is a question about the whole set, not about one
    // row.
    if let Some(name) = daily_name {
        return name.to_string();
    }
    grouped_child_display_label(label, branch, has_custom_name)
}

/// The name a row takes from what is *inside* it, for a workspace whose
/// directory cannot tell it apart from its neighbours.
///
/// The daily area collects every workspace standing in one directory, so the
/// directory's own name is the one thing they all share — deriving the label
/// from it names them all the same. Seven rows reading `ayaz` say no more than
/// seven rows reading `main` did (#99): a name repeated seven times is not a
/// name, it is a category.
///
/// So the row asks its contents instead, in the order a person would:
///
/// 1. **A tab that has been named.** A named tab is the one place a purpose has
///    already been written down — by hand or by the auto-namer. With a single
///    tab the workspace *is* that tab, so it wears the name outright; with
///    several it wears the first and counts the rest, because a multi-tab
///    workspace cannot honestly be reduced to one of them.
/// 2. **What is running in it.** A workspace of unnamed tabs is still
///    distinguishable by the agent inside it. `pane_details` only answers for
///    panes that report an agent, so a plain shell contributes nothing and
///    never becomes a name.
/// 3. **Nothing.** The caller keeps the directory name — and
///    [`disambiguate_repeated_labels`] makes the repeats addressable.
///
/// Tab numbers are deliberately not a source: `tab_display_name` falls back to
/// the tab's ordinal, so an unnamed tab answers `"1"`. That is a position, not
/// an identity, and hoisting it into the row would name six workspaces `1`.
/// Only `custom_name` counts as named, which is why this takes the raw option
/// rather than the display string.
// TP-DAILY-15
pub(crate) fn content_derived_row_name<'a>(
    tab_names: impl IntoIterator<Item = Option<&'a str>>,
    agent_labels: impl IntoIterator<Item = &'a str>,
) -> Option<String> {
    let mut tab_count = 0usize;
    let mut first_named: Option<&str> = None;
    for name in tab_names {
        tab_count += 1;
        if first_named.is_some() {
            continue;
        }
        let named = name.map(str::trim).filter(|name| !name.is_empty());
        first_named = named;
    }

    if let Some(name) = first_named {
        return Some(if tab_count > 1 {
            format!("{name} +{}", tab_count - 1)
        } else {
            name.to_string()
        });
    }

    agent_labels
        .into_iter()
        .map(str::trim)
        .find(|label| !label.is_empty())
        .map(str::to_string)
}

/// Make repeated labels addressable by numbering the repeats.
///
/// Two rows that read alike are two rows a person cannot choose between. When
/// the derivation above still lands on one name — five workspaces each running
/// nothing but `reviewr` — the repeats take an ordinal so every row can at
/// least be named out loud. The first keeps the bare name: numbering something
/// that appears once invents a series that does not exist.
// TP-DAILY-16
pub(crate) fn disambiguate_repeated_labels(labels: &mut [String]) {
    let mut occurrences: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for label in labels.iter() {
        *occurrences.entry(label.as_str()).or_insert(0) += 1;
    }
    if occurrences.values().all(|count| *count <= 1) {
        return;
    }

    // Every name already on screen is spoken for, including the ones this loop
    // has not reached yet. A row literally named `reviewr 2` must not be
    // shadowed by an ordinal minted for a different row — two rows reading
    // `reviewr 2` is the very defect being fixed, arrived at from the other
    // side.
    let mut taken: std::collections::HashSet<String> = labels.iter().cloned().collect();
    let mut seen: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for label in labels.iter_mut() {
        let count = seen.entry(label.clone()).or_insert(0);
        *count += 1;
        if *count == 1 {
            continue;
        }
        let mut ordinal = *count;
        let numbered = loop {
            let candidate = format!("{label} {ordinal}");
            if !taken.contains(&candidate) {
                break candidate;
            }
            ordinal += 1;
        };
        taken.insert(numbered.clone());
        *label = numbered;
    }
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
    /// A chat that was moved into a declared container, drawn under that
    /// container's header.
    ///
    /// TP-CHAT-MOVE-06: this carries a `node_key` and deliberately no
    /// `ws_idx`. A container is not a workspace — it may have no directory at
    /// all — so folding it into the workspace-indexed area vectors would make
    /// every press on one of these rows resolve as some other checkout's chat,
    /// the same trap `TP-DAILY-03` keeps the daily rows out of.
    ModuleChat {
        node_key: String,
        chat_idx: usize,
    },
    /// Placeholder under an expanded workspace that has no remembered chats.
    /// Shown rather than nothing, because an empty gap reads as a broken
    /// drawer — and an empty drawer is the honest answer for a branch whose
    /// work predates the ledger.
    NoChats {
        ws_idx: usize,
    },
    /// A declared container that draws nothing beneath it, saying so on a row
    /// of its own.
    ///
    /// TP-MOD-03: an empty gap under a header is what damage looks like. Once
    /// a module could be created before any branch existed (TP-MOD-13), the
    /// tree gained rows whose whole content is absence — and absence has to be
    /// stated, or the reader goes hand-editing a tree that was working.
    ///
    /// It carries no `ws_idx`, like the header rows, and nothing behind it
    /// opens: the way in is the "+" on the header directly above.
    EmptyModule {
        node_key: String,
    },
    /// Inert "… N older" row when a workspace has more chats than the drawer
    /// lists.
    MoreChats {
        ws_idx: usize,
        /// Whether this display has already opened the drawer all the way.
        /// The row is the way in AND the way back (TP-DRAW-11) — a switch
        /// with no off position is not a switch.
        expanded: bool,
    },
    /// The header of the daily-chats section, above the whole tree.
    ///
    /// TP-DAILY-02: chats started outside every checkout have no workspace to
    /// hang under — since `effective_cwd` prefers the checkout, nothing claims
    /// `$HOME` and those conversations were reachable from nowhere. The
    /// section is their home, and it sits at the top because that is where a
    /// person looks for what they were just doing.
    ///
    /// Like the other header rows it carries no `ws_idx`: it must never fold
    /// into a workspace-indexed vector.
    DailyHeader,
    /// A chat under the daily section, indexed into `daily_chat_rows`.
    DailyChat {
        chat_idx: usize,
    },
    /// The daily section's own "… N older" switch (TP-DRAW-11's sibling).
    DailyMore {
        expanded: bool,
    },
    /// The switch that reveals the other workspaces standing in the daily
    /// directory.
    ///
    /// TP-DAILY-18: the same shape `DailyMore` uses for chats, for the same
    /// reason — the section is a glance surface. Seven rows for one place read
    /// as spam, which is exactly how it was reported; one row and a switch
    /// reads as one place you can look inside.
    DailyMoreWorkspaces {
        hidden: usize,
        expanded: bool,
    },
}

/// How many chats a workspace's drawer lists before folding the rest into a
/// single "older" row. The sidebar is a glance surface, not an archive.
pub(crate) const WORKSPACE_CHAT_ROW_LIMIT: usize = 5;

/// What the daily section calls itself. One constant so the screen, the tests
/// and the phone drawer can never disagree about the words.
pub(crate) const DAILY_SECTION_TITLE: &str = "daily chats";

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
/// What a container with nothing under it writes about itself. One constant so
/// the screen and the tests can never disagree about the words.
pub(crate) const EMPTY_MODULE_NOTE: &str = "(no branches yet)";
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
    let depth =
        u16::from(workspace_is_group_member(app, ws_idx)) + workspace_node_shift(app, ws_idx);
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

/// The "manage this row" cell: a breathing cell left of the "+", opening the
/// same menu a right-click does (TP-DOTS-04 — one menu source, two roads).
/// Mouse chrome like the "+": drawn only while the mouse owns the sidebar.
pub(crate) fn workspace_menu_cell(card_rect: Rect) -> Rect {
    if card_rect.width < 6 {
        return Rect::default();
    }
    Rect::new(card_rect.x + card_rect.width - 3, card_rect.y, 1, 1)
}

/// The header rows' "manage" cell: a breathing cell left of the header "+"
/// (TP-DOTS-03), mirroring the card's `[⋯] [+]` layout. Same contract as
/// [`workspace_menu_cell`].
pub(crate) fn header_menu_cell(head_rect: Rect) -> Rect {
    if head_rect.width < 6 {
        return Rect::default();
    }
    Rect::new(head_rect.x + head_rect.width - 3, head_rect.y, 1, 1)
}

/// The header rows' "+": the trailing edge of a node or bucket header,
/// starting the module's "New branch..." road (TP-DOTS-17 — the same body
/// the header menu walks, so the two doors can never drift apart).
pub(crate) fn header_new_branch_cell(head_rect: Rect) -> Rect {
    if head_rect.width < 6 {
        return Rect::default();
    }
    Rect::new(head_rect.x + head_rect.width - 1, head_rect.y, 1, 1)
}

/// The daily header's "+": the trailing edge of the section header, opening
/// the same agent menu a workspace card's "+" opens (TP-DAILY-10).
///
/// Geometry deliberately identical to [`header_new_branch_cell`] — one plus
/// sits at one place on this sidebar, wherever it appears. Mouse chrome, so
/// the count underneath it keeps the row whenever the mouse is elsewhere.
pub(crate) fn daily_new_chat_cell(head_rect: Rect) -> Rect {
    if head_rect.width < 6 {
        return Rect::default();
    }
    Rect::new(head_rect.x + head_rect.width - 1, head_rect.y, 1, 1)
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
    // TP-WSID-03: the drawer reads by the directory the row MEANS — the
    // checkout when known — never by the birthplace two rows may share.
    let key = crate::persist::workspace_chats::ledger_key(workspace.effective_cwd());
    app.workspace_chat_rows
        .get(&key)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

/// Whether `ws_idx`'s drawer is folded shut.
///
/// A thin delegate: the mode-aware answer lives on [`AppState`], the single
/// evaluation gate, so the render path, the mobile drawer, and the input
/// layer all read the same verdict.
pub(crate) fn workspace_chat_drawer_collapsed(app: &AppState, ws_idx: usize) -> bool {
    app.chat_drawer_collapsed(ws_idx)
}

/// Whether this display asked to see the whole of `ws_idx`'s drawer.
///
/// TP-DRAW-12: keyed through the same ledger key the drawer's openness is
/// keyed by, and held per display for the same reason — one screen reading
/// an old chat must not stretch the drawer on another.
pub(crate) fn workspace_chat_drawer_expanded(app: &AppState, ws_idx: usize) -> bool {
    app.workspaces
        .get(ws_idx)
        .map(|ws| crate::persist::workspace_chats::ledger_key(ws.effective_cwd()))
        .is_some_and(|key| app.fully_open_chat_drawers.contains(&key))
}

/// The chat row the selection accent belongs to, when there is one.
///
/// TP-FOCUS-01: the accent marks the deepest *visible* focus object. When the
/// active tab resumes a chat and that workspace's drawer is open, the answer
/// to "where am I" is the chat row itself, so the accent descends to it and
/// the workspace card steps back to a quiet active tone. Whenever the drawer
/// is shut or no chat is resumed this is `None` and the card keeps the accent
/// (TP-TREE-11) — an invisible selection would be worse than a coarse one.
pub(crate) fn visible_active_chat(app: &AppState) -> Option<(usize, usize)> {
    let ws_idx = app.active?;
    if workspace_chat_drawer_collapsed(app, ws_idx) {
        return None;
    }
    let active_tab = app.workspaces.get(ws_idx)?.active_tab_index();
    workspace_chat_rows_for(app, ws_idx)
        .iter()
        .position(|chat| {
            app.find_resumed_chat_tab(&chat.session_id)
                .is_some_and(|(w, tab)| w == ws_idx && tab == active_tab)
        })
        .map(|chat_idx| (ws_idx, chat_idx))
}

/// The chats of the daily directory — the ones no checkout claims.
///
/// Empty only when there is no daily directory or nothing has been started
/// there. TP-DAILY-09: a workspace sitting in that same directory does not
/// silence the section. It used to, and on the machine this was built for the
/// silence was total — ten workspaces had been born in `$HOME`, seven outside
/// any checkout, so `effective_cwd` handed back `$HOME` on every render. The
/// duplication that rule guarded against was already on screen seven times
/// over, since each of those workspaces reads this very ledger key; all the
/// rule removed was the one place the chats could be found on purpose.
pub(crate) fn daily_chat_rows(app: &AppState) -> &[crate::app::state::WorkspaceChatRow] {
    let Some(daily) = app.daily_chat_cwd.as_deref() else {
        return &[];
    };
    let key = crate::persist::workspace_chats::ledger_key(daily);
    app.workspace_chat_rows
        .get(&key)
        .map(Vec::as_slice)
        .unwrap_or_default()
}

/// The chats that were moved into a declared container.
///
/// TP-CHAT-MOVE-06: the container builds its own key rather than being handed
/// one, exactly as [`daily_chat_rows`] does for a directory. That is the whole
/// discipline that keeps the ledger's two key spaces from touching — nobody
/// parses a key to find out what it is, because everybody who reads one made
/// it.
pub(crate) fn module_chat_rows<'a>(
    app: &'a AppState,
    node_key: &str,
) -> &'a [crate::app::state::WorkspaceChatRow] {
    let key = crate::persist::workspace_chats::module_ledger_key(node_key);
    app.workspace_chat_rows
        .get(&key)
        .map(Vec::as_slice)
        .unwrap_or_default()
}

/// Whether the daily section draws at all on this display.
///
/// TP-DAILY-06: the focus switch narrows the tree to what this display is
/// working in, and daily chats are not in any tree — so they go quiet with
/// everything else. The exception is a daily chat that is actually running:
/// a filter may narrow what you see, never hide where you are.
fn daily_section_visible(app: &AppState) -> bool {
    if daily_chat_rows(app).is_empty() {
        return false;
    }
    if !app.spaces_focus_only {
        return true;
    }
    daily_chat_rows(app)
        .iter()
        .any(|chat| app.find_resumed_chat_tab(&chat.session_id).is_some())
}

/// The workspaces that stand in the daily directory itself.
///
/// TP-DAILY-13: a workspace whose effective directory *is* the daily directory
/// has no checkout of its own to sit under. The tree drew it anyway, as a
/// sibling of real checkouts, and on the machine this was built for there were
/// seven of them — seven rows that looked like branches, listed the same ledger,
/// and opened and closed together because the drawer's fold state is keyed by
/// that shared ledger key. They belong under the daily area, not beside it.
///
/// Three gates, all of which must hold:
///
/// - the section must be drawn at all, or these rows would land beneath a
///   header that is not there — a row you cannot see is the #88 failure again;
/// - there must be a daily directory to compare against;
/// - the workspace's *effective* directory must resolve to the same ledger key.
///   Effective, not `worktree_space`: reading only the checkout field is what
///   made #88's first measurement wrong, because every one of those seven
///   workspaces has no worktree at all and answers with its birthplace.
///
/// This returns indices rather than filtering the workspace set, because the
/// set feeds the client frame as well as the sidebar. Removing a workspace at
/// the source removes it from every surface that reads it — the regression the
/// first attempt at this shipped and had to take back.
/// TP-DAILY-17: the membership test itself now lives on the state, because the
/// "new workspace" road has to ask the same question and two copies of it would
/// drift. This adds the one thing that is genuinely the sidebar's business —
/// whether the section is drawn at all.
fn daily_owned_workspaces(app: &AppState) -> Vec<usize> {
    if !daily_section_visible(app) {
        return Vec::new();
    }
    app.workspaces_in_daily_directory()
}

/// The name each daily-area row wears, resolved for the whole set at once.
///
/// Telling these rows apart is a question about the set, not about any one row:
/// a name is only useless *because another row has it too*. So the derivation
/// (TP-DAILY-15) and the numbering that follows it (TP-DAILY-16) run together,
/// once, and the render loop reads the answer.
///
/// A workspace the user has named keeps that name untouched — an explicit
/// intent outranks anything derived — but it still takes part in the
/// numbering, because two rows named alike by hand are just as unaddressable
/// as two named alike by accident.
// TP-DAILY-15/16
fn daily_row_names(
    app: &AppState,
    owned: &[usize],
    terminal_runtimes: Option<&TerminalRuntimeRegistry>,
) -> std::collections::HashMap<usize, String> {
    let directory_name = |ws: &crate::workspace::Workspace| match terminal_runtimes {
        Some(runtimes) => ws.display_name_from(&app.terminals, runtimes),
        None => ws.display_name(),
    };

    let mut labels: Vec<String> = owned
        .iter()
        .filter_map(|ws_idx| app.workspaces.get(*ws_idx))
        .map(|ws| {
            if ws.custom_name.is_some() {
                return directory_name(ws);
            }
            let details = ws.pane_details(&app.terminals);
            content_derived_row_name(
                ws.tabs.iter().map(|tab| tab.custom_name.as_deref()),
                details.iter().map(|detail| detail.label.as_str()),
            )
            .unwrap_or_else(|| directory_name(ws))
        })
        .collect();
    disambiguate_repeated_labels(&mut labels);

    owned.iter().copied().zip(labels).collect()
}

/// Append the daily section — header first, then its chats — above the tree.
///
/// `owned` is [`daily_owned_workspaces`], computed once by the caller so the
/// tree walk and this function cannot disagree about which rows belong here.
fn push_daily_section(app: &AppState, entries: &mut Vec<WorkspaceListEntry>, owned: &[usize]) {
    if !daily_section_visible(app) {
        return;
    }
    entries.push(WorkspaceListEntry::DailyHeader);
    if app.daily_section_collapsed {
        return;
    }
    let chats = daily_chat_rows(app);
    let expanded = app.daily_section_expanded;
    let shown = if expanded {
        chats.len()
    } else {
        chats.len().min(WORKSPACE_CHAT_ROW_LIMIT)
    };
    for chat_idx in 0..shown {
        entries.push(WorkspaceListEntry::DailyChat { chat_idx });
    }
    if chats.len() > WORKSPACE_CHAT_ROW_LIMIT {
        entries.push(WorkspaceListEntry::DailyMore { expanded });
    }
    // TP-DAILY-13: the workspaces standing in this very directory, drawn as
    // the area's own rows rather than as branches in the tree.
    //
    // No chat drawer is opened under them, deliberately. Such a workspace
    // reads the daily ledger key — the same one the rows above came from — so
    // its drawer would repeat this section's chat list verbatim. That repeat
    // is what #96 was: seven rows listing the same twelve chats, opening and
    // closing together because one shared key holds their fold state. The row
    // is here to reach the workspace and its panes; the chats are already
    // above it.
    //
    // TP-DAILY-18: and only ONE of them at a glance. The daily directory is a
    // single place — its chats are keyed by that directory, which is why these
    // rows have no drawer of their own — so several rows for it say the same
    // thing several times. Seven of them was the reported defect, and naming
    // them apart (TP-DAILY-15/16) made the repetition legible without making it
    // any less repetitive: "hepsinin içinde aynı chatler var, fark ne ki?".
    //
    // The rest fold behind a switch rather than disappearing. Those six held
    // fifteen panes and one blocked agent on the machine this came from, and a
    // row nobody can reach is the #88 failure, not a fix for it.
    for (position, ws_idx) in daily_workspace_order(app, owned).into_iter().enumerate() {
        if position > 0 && !app.daily_workspaces_expanded {
            break;
        }
        entries.push(WorkspaceListEntry::Workspace {
            ws_idx,
            indented: true,
        });
    }
    if owned.len() > 1 {
        entries.push(WorkspaceListEntry::DailyMoreWorkspaces {
            hidden: owned.len() - 1,
            expanded: app.daily_workspaces_expanded,
        });
    }
}

/// The daily workspaces with the one that should always be visible first.
///
/// TP-DAILY-18: the workspace you are IN leads, when it is one of these.
/// A fixed "always the first in list order" would hide the row you are working
/// in behind the switch — the row most worth seeing, folded away by the rule
/// meant to reduce noise.
pub(crate) fn daily_workspace_order(app: &AppState, owned: &[usize]) -> Vec<usize> {
    let mut order: Vec<usize> = owned.to_vec();
    if let Some(active) = app.active {
        if let Some(position) = order.iter().position(|ws_idx| *ws_idx == active) {
            order.swap(0, position);
        }
    }
    order
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
    // TP-DRAW-10: a drawer this display asked to see whole shows every chat
    // it holds; otherwise the glance surface keeps its five.
    let expanded = workspace_chat_drawer_expanded(app, ws_idx);
    let shown = if expanded {
        chats.len()
    } else {
        chats.len().min(WORKSPACE_CHAT_ROW_LIMIT)
    };
    for chat_idx in 0..shown {
        entries.push(WorkspaceListEntry::Chat { ws_idx, chat_idx });
    }
    if chats.len() > WORKSPACE_CHAT_ROW_LIMIT {
        entries.push(WorkspaceListEntry::MoreChats { ws_idx, expanded });
    }
}

pub(crate) fn normalized_workspace_scroll(app: &AppState, area: Rect, requested: usize) -> usize {
    let ws_area = workspace_list_rect(area, app.sidebar_section_split, app.sidebar_chrome);
    let body = workspace_list_body_rect(ws_area, false, app.sidebar_chrome);
    if body.height == 0 {
        return requested;
    }

    if workspace_list_entries(app).is_empty() {
        0
    } else {
        requested.min(workspace_list_bottom_start(app, ws_area))
    }
}

/// The checkouts a focused tree keeps, or `None` when the whole tree shows.
///
/// TP-FOCUS-SW-02: focus answers "what am I working in right now" — the
/// active checkout (the selected one while navigating, since that is what the
/// screen is pointing at) plus every checkout running an agent. The module
/// chain above them survives on its own, because headers are drawn from their
/// members: filter the members and the empty modules go quiet by themselves.
///
/// TP-FOCUS-SW-03: a filter that would empty the tree keeps its hands off it.
/// With nothing active and nothing running there is no noise to remove, and a
/// blank list reads as a broken sidebar rather than a focused one.
pub(crate) fn focus_visible_workspaces(app: &AppState) -> Option<std::collections::HashSet<usize>> {
    if !app.spaces_focus_only {
        return None;
    }
    let mut visible = std::collections::HashSet::new();
    // While navigating, the selection is what the screen is pointing at; in
    // every other mode the active checkout is. This is the same pair the
    // tree already uses to decide which group counts as the visible one.
    let pointed = if matches!(app.mode, Mode::Navigate) {
        Some(app.selected)
    } else {
        app.active
    };
    if let Some(idx) = pointed.filter(|idx| *idx < app.workspaces.len()) {
        visible.insert(idx);
    }
    // "Running" is borrowed from the agents panel rather than defined again:
    // two surfaces answering the same question from two definitions drift,
    // and the tree would start disagreeing with the list right below it.
    for entry in agent_panel_entries(app) {
        visible.insert(entry.ws_idx);
    }
    (!visible.is_empty()).then_some(visible)
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
    // TP-FOCUS-SW-01/02: the focus filter is applied at the source, not in
    // the renderer. Everything downstream — group membership, the two-member
    // threshold, the header rows — is derived from this set, so a module
    // whose checkouts are all filtered out loses its header the same way an
    // empty module never gets one.
    let focused = focus_visible_workspaces(app);
    let shows = |ws_idx: usize| focused.as_ref().is_none_or(|set| set.contains(&ws_idx));
    // TP-DAILY-13: the daily area emits its own workspaces, so the tree must
    // not emit them a second time. Computed once, here, and handed to both
    // sides — two independent answers to "is this row the area's own?" is how
    // a row ends up drawn twice or not at all.
    //
    // This skips the row in the *walk*; it does not remove the workspace from
    // any set. `app.workspaces` is untouched and the row is re-emitted below
    // by `push_daily_section`, so every surface reading this list still sees
    // it. Filtering at the source is what the first attempt did, and it took
    // API-created workspaces off the client frame as well.
    let daily_owned = daily_owned_workspaces(app);
    let mut members_by_key = std::collections::HashMap::<String, Vec<usize>>::new();
    for ws_idx in 0..app.workspaces.len() {
        if !shows(ws_idx) {
            continue;
        }
        if daily_owned.contains(&ws_idx) {
            continue;
        }
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

    // The node forest, prepared once per emit. TP-PROJ-MATCH-02 survives as
    // the depth-one case: a space's owner is resolved once, through the
    // claiming rule's own `parent` first and the claiming project second, so
    // a config space split out of a repo still follows that repo's project
    // and a promoted space key still matches regardless of repository.
    let node_parent: std::collections::HashMap<&str, Option<&str>> = app
        .space_nodes
        .iter()
        .map(|node| (node.key.as_str(), node.parent.as_deref()))
        .collect();
    let mut node_children: std::collections::HashMap<&str, Vec<&str>> =
        std::collections::HashMap::new();
    for node in &app.space_nodes {
        if let Some(parent) = node.parent.as_deref() {
            node_children
                .entry(parent)
                .or_default()
                .push(node.key.as_str());
        }
    }
    let parent_of_space: std::collections::HashMap<String, Option<String>> = members_by_key
        .iter()
        .map(|(key, members)| {
            let first = members.first().and_then(|idx| app.workspaces.get(*idx));
            let repo_root = first
                .and_then(|ws| ws.worktree_space())
                .map(|space| space.repo_root.clone());
            let rule = first.and_then(|ws| {
                let membership = ws.worktree_space()?;
                crate::spaces::resolve_space_rule(
                    &app.space_split_rules,
                    &membership.repo_root,
                    &membership.checkout_path,
                    ws.branch().as_deref(),
                )
            });
            let owner = crate::spaces::resolve_space_parent(
                rule,
                &app.space_projects,
                key,
                repo_root.as_deref(),
            )
            // A parent nobody defines keeps the space at top level — the
            // forest is validated, but a rule can still name a ghost. A
            // claiming project counts as defined even when the node list was
            // set by hand: a project IS a node (TP-NODE-02).
            .filter(|parent| {
                node_parent.contains_key(parent.as_str())
                    || app
                        .space_projects
                        .iter()
                        .any(|project| &project.key == parent)
                    // A defined split rule is a real parent too: a bucket can
                    // hang under a bucket once modules hang under buckets
                    // (TP-NODE-08); only the truly undefined stays a ghost.
                    || app
                        .space_split_rules
                        .iter()
                        .any(|rule| &rule.key == parent)
            });
            (key.clone(), owner)
        })
        .collect();
    let first_ws_of_space: std::collections::HashMap<String, usize> = members_by_key
        .iter()
        .map(|(key, members)| {
            (
                key.clone(),
                members.iter().copied().min().unwrap_or(usize::MAX),
            )
        })
        .collect();
    let mut buckets_of_node: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for (key, owner) in &parent_of_space {
        if let Some(owner) = owner {
            buckets_of_node
                .entry(owner.clone())
                .or_default()
                .push(key.clone());
        }
    }

    // Whether ws_idx's space hangs anywhere under `target` — the folded-node
    // active-checkout promise needs the whole chain, not just the owner.
    let chain_hits = |space_key: &str, target: &str| -> bool {
        let mut current = parent_of_space.get(space_key).cloned().flatten();
        let mut steps = 0usize;
        while let Some(key) = current {
            if key == target {
                return true;
            }
            steps += 1;
            if steps > app.space_nodes.len() {
                break;
            }
            current = crate::spaces::tree_parent_of(&app.space_nodes, &app.space_split_rules, &key)
                .map(str::to_string);
        }
        false
    };

    // TP-MOD-13/16: the tree has two sources, and the second one is why a
    // module can exist before any branch does.
    //
    // The walk below is seeded from the checkouts and climbs to their roots,
    // so a container nothing hangs under is never reached. That is the first
    // thing a person does: name the module, then make the branch — and until
    // the branch existed the module was nowhere on screen. Every declared
    // container therefore contributes its own root as a seed, after the
    // checkouts (TP-MOD-14: scaffolding must never push the work down).
    //
    // Duplication needs no guard of its own: a root already emitted from a
    // climb is stopped by `emitted_nodes` inside the walk (TP-MOD-17).
    let declared_roots: Vec<String> = {
        let mut roots = Vec::new();
        let mut seen = std::collections::HashSet::<String>::new();
        let guard = app.space_nodes.len() + app.space_split_rules.len();
        for node in &app.space_nodes {
            let mut root = node.key.clone();
            let mut climbed = 0usize;
            while let Some(parent) =
                crate::spaces::tree_parent_of(&app.space_nodes, &app.space_split_rules, &root)
            {
                root = parent.to_string();
                climbed += 1;
                if climbed > guard {
                    break; // The forest is validated; belt and braces.
                }
            }
            if seen.insert(root.clone()) {
                roots.push(root);
            }
        }
        roots
    };

    /// What starts one walk of the tree.
    enum Seed {
        /// A checkout, which climbs to whatever owns it.
        Checkout(usize),
        /// The top of a declared container chain, which owns itself.
        DeclaredRoot(String),
    }

    let mut emitted_groups = std::collections::HashSet::<String>::new();
    let mut emitted_nodes = std::collections::HashSet::<String>::new();
    let mut entries = Vec::new();
    // TP-DAILY-02: above the tree, before anything the walk emits. These
    // chats belong to no checkout, so there is no branch of the tree they
    // could be placed under without lying about where they came from.
    push_daily_section(app, &mut entries, &daily_owned);
    let seeds = (0..app.workspaces.len())
        .map(Seed::Checkout)
        .chain(declared_roots.into_iter().map(Seed::DeclaredRoot));
    for seed in seeds {
        let ws_idx = match seed {
            Seed::Checkout(ws_idx) => ws_idx,
            Seed::DeclaredRoot(root) => {
                // A declared root is already the top of its chain, so it skips
                // the climb the checkout path needs.
                let mut stack = vec![if members_by_key.contains_key(&root) {
                    Job::Bucket(root, false)
                } else {
                    Job::Node(root)
                }];
                walk_tree(
                    app,
                    &mut stack,
                    &mut entries,
                    &mut TreeWalkState {
                        emitted_groups: &mut emitted_groups,
                        emitted_nodes: &mut emitted_nodes,
                    },
                    &TreeWalkMaps {
                        members_by_key: &members_by_key,
                        grouped_keys: &grouped_keys,
                        node_children: &node_children,
                        buckets_of_node: &buckets_of_node,
                        first_ws_of_space: &first_ws_of_space,
                    },
                    force_expanded,
                    visible_group_idx,
                    active_group.as_deref(),
                    &chain_hits,
                );
                continue;
            }
        };
        if !shows(ws_idx) {
            continue;
        }
        // TP-DAILY-13: already emitted under the daily area. Both tree exits
        // below — the space block and the stack walk — are downstream of this
        // gate, so one skip covers the walk however the row would have found
        // its way in.
        if daily_owned.contains(&ws_idx) {
            continue;
        }
        let space = effective_space(app, ws_idx);
        let owner = space
            .as_ref()
            .and_then(|space| parent_of_space.get(&space.key).cloned().flatten());

        // TP-TREE-16: a bucket with no owner may still carry child modules —
        // it rides the same stack machine so its subtree follows its block.
        let root_seed = match (&space, owner) {
            (_, Some(owner)) => owner,
            (Some(space), None) if node_children.contains_key(space.key.as_str()) => {
                space.key.clone()
            }
            _ => {
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
                    false,
                );
                continue;
            }
        };

        // TP-NODE-04, generalising TP-PROJ-GROUP-01: the chain's root takes
        // its row where its first descendant sits, and the whole subtree
        // follows pre-order — every ancestor header before its children,
        // children in their own first-appearance order. The climb crosses
        // bucket edges too (TP-NODE-08), so a mixed chain finds its true top.
        let mut root = root_seed;
        let mut climbed = 0usize;
        while let Some(parent) =
            crate::spaces::tree_parent_of(&app.space_nodes, &app.space_split_rules, &root)
        {
            root = parent.to_string();
            climbed += 1;
            if climbed > app.space_nodes.len() + app.space_split_rules.len() {
                break; // The forest is validated; belt and braces.
            }
        }

        // The top of a mixed chain can itself be a bucket (TP-TREE-16).
        let mut stack = vec![if members_by_key.contains_key(&root) {
            Job::Bucket(root, false)
        } else {
            Job::Node(root)
        }];
        walk_tree(
            app,
            &mut stack,
            &mut entries,
            &mut TreeWalkState {
                emitted_groups: &mut emitted_groups,
                emitted_nodes: &mut emitted_nodes,
            },
            &TreeWalkMaps {
                members_by_key: &members_by_key,
                grouped_keys: &grouped_keys,
                node_children: &node_children,
                buckets_of_node: &buckets_of_node,
                first_ws_of_space: &first_ws_of_space,
            },
            force_expanded,
            visible_group_idx,
            active_group.as_deref(),
            &chain_hits,
        );
    }
    entries
}

/// The maps one tree walk reads, gathered so the walk can be driven from more
/// than one seed without threading a dozen arguments through each call.
struct TreeWalkMaps<'a> {
    members_by_key: &'a std::collections::HashMap<String, Vec<usize>>,
    grouped_keys: &'a std::collections::HashSet<String>,
    node_children: &'a std::collections::HashMap<&'a str, Vec<&'a str>>,
    buckets_of_node: &'a std::collections::HashMap<String, Vec<String>>,
    first_ws_of_space: &'a std::collections::HashMap<String, usize>,
}

/// What the walk has already drawn. Shared across seeds: a container reached
/// from a checkout's climb must not be drawn again when the declared forest
/// reaches it too (TP-MOD-17).
struct TreeWalkState<'a> {
    emitted_groups: &'a mut std::collections::HashSet<String>,
    emitted_nodes: &'a mut std::collections::HashSet<String>,
}

/// One job of the tree walk.
enum Job {
    Node(String),
    /// A bucket block plus, when it is open, the modules hanging under it.
    /// The flag says whether an ancestor indents it.
    Bucket(String, bool),
}

/// Drain `stack`, appending the rows each job produces to `entries`.
///
/// Pre-order: every ancestor header before its children, children in their
/// own first-appearance order (TP-NODE-04).
#[allow(clippy::too_many_arguments)]
fn walk_tree(
    app: &AppState,
    stack: &mut Vec<Job>,
    entries: &mut Vec<WorkspaceListEntry>,
    state: &mut TreeWalkState<'_>,
    maps: &TreeWalkMaps<'_>,
    force_expanded: bool,
    visible_group_idx: Option<usize>,
    active_group: Option<&str>,
    chain_hits: &dyn Fn(&str, &str) -> bool,
) {
    while let Some(job) = stack.pop() {
        match job {
            Job::Bucket(space_key, parented) => {
                // TP-MOD-15: a rule claiming nothing draws no header — a
                // header for a bucket with no members is a ghost, and a
                // ghost header is a false alarm every time a module is
                // created before its branch.
                //
                // It still walks its children. A module the user declared
                // under that rule is theirs, not the rule's, and tying its
                // fate to whether a branch happens to match today would lose
                // scaffolding for a reason that has nothing to do with it.
                match maps
                    .members_by_key
                    .get(&space_key)
                    .and_then(|members| members.first().copied())
                {
                    Some(first) => push_space_block(
                        app,
                        entries,
                        first,
                        maps.members_by_key,
                        maps.grouped_keys,
                        state.emitted_groups,
                        force_expanded,
                        visible_group_idx,
                        active_group,
                        parented,
                    ),
                    None => {
                        let carries_modules = maps
                            .node_children
                            .get(space_key.as_str())
                            .is_some_and(|kids| !kids.is_empty());
                        if !carries_modules {
                            continue;
                        }
                    }
                }
                // TP-TREE-16/17: an open bucket walks the modules (and
                // through them, buckets) hanging under it; a folded one
                // hides its whole subtree, exactly like its members.
                if !force_expanded && app.collapsed_space_keys.contains(&space_key) {
                    continue;
                }
                let mut kids: Vec<(usize, Job)> = Vec::new();
                for bucket in maps.buckets_of_node.get(&space_key).into_iter().flatten() {
                    kids.push((
                        maps.first_ws_of_space
                            .get(bucket)
                            .copied()
                            .unwrap_or(usize::MAX),
                        Job::Bucket(bucket.clone(), true),
                    ));
                }
                for child in maps
                    .node_children
                    .get(space_key.as_str())
                    .into_iter()
                    .flatten()
                {
                    kids.push((
                        subtree_first_ws(
                            child,
                            maps.node_children,
                            maps.buckets_of_node,
                            maps.first_ws_of_space,
                            app.space_nodes.len() + 1,
                        ),
                        Job::Node((*child).to_string()),
                    ));
                }
                kids.sort_by_key(|(first, _)| *first);
                for (_, kid) in kids.into_iter().rev() {
                    stack.push(kid);
                }
            }
            Job::Node(node_key) => {
                // Several member workspaces climb to the same top; the
                // subtree is emitted once and later climbs skip here.
                if !state.emitted_nodes.insert(node_key.clone()) {
                    continue;
                }
                entries.push(WorkspaceListEntry::ProjectHeader {
                    project_key: node_key.clone(),
                });

                if !force_expanded && app.node_folded(&node_key) {
                    // TP-PROJ-GROUP-02, generalised: a folded ancestor
                    // keeps the checkout the user is standing in.
                    if let Some(active_idx) = visible_group_idx.filter(|idx| {
                        effective_space(app, *idx)
                            .is_some_and(|space| chain_hits(&space.key, &node_key))
                    }) {
                        entries.push(WorkspaceListEntry::Workspace {
                            ws_idx: active_idx,
                            indented: true,
                        });
                        push_chat_drawer(app, entries, active_idx);
                    }
                    continue;
                }

                // TP-CHAT-MOVE-06: the chats someone moved into this
                // container, drawn under its own header.
                //
                // This sits after the fold's `continue` on purpose: a folded
                // container shows none of them, which is the same contract
                // every other container on this sidebar keeps.
                let module_chats = module_chat_rows(app, &node_key);
                let shown = module_chats.len().min(WORKSPACE_CHAT_ROW_LIMIT);
                for chat_idx in 0..shown {
                    entries.push(WorkspaceListEntry::ModuleChat {
                        node_key: node_key.clone(),
                        chat_idx,
                    });
                }

                // TP-MOD-03/24: an open container with nothing under it says
                // so, right below its own header. Folded ones stay quiet —
                // describing the inside of a closed box undoes closing it.
                //
                // TP-CHAT-MOVE-06: a container holding moved chats is not
                // empty. Saying "nothing here" directly above a list of chats
                // is the opposite of the readability TP-MOD-03 exists for.
                if !subtree_draws_rows(&node_key, maps) && module_chats.is_empty() {
                    entries.push(WorkspaceListEntry::EmptyModule {
                        node_key: node_key.clone(),
                    });
                }

                let mut kids: Vec<(usize, Job)> = Vec::new();
                for bucket in maps.buckets_of_node.get(&node_key).into_iter().flatten() {
                    kids.push((
                        maps.first_ws_of_space
                            .get(bucket)
                            .copied()
                            .unwrap_or(usize::MAX),
                        Job::Bucket(bucket.clone(), true),
                    ));
                }
                for child in maps
                    .node_children
                    .get(node_key.as_str())
                    .into_iter()
                    .flatten()
                {
                    kids.push((
                        subtree_first_ws(
                            child,
                            maps.node_children,
                            maps.buckets_of_node,
                            maps.first_ws_of_space,
                            app.space_nodes.len() + 1,
                        ),
                        Job::Node((*child).to_string()),
                    ));
                }
                kids.sort_by_key(|(first, _)| *first);
                for (_, kid) in kids.into_iter().rev() {
                    stack.push(kid);
                }
            }
        }
    }
}

/// Whether anything under `node_key` reaches the screen.
///
/// TP-MOD-21: the question is what gets *drawn*, and the two child maps answer
/// it directly once you know what can be in them. A declared module always
/// paints its own header, so any entry in `node_children` means "not empty".
/// `buckets_of_node` is built from `parent_of_space`, which is built from
/// `members_by_key` — so every bucket that can appear there already has a
/// member and already draws a block. A bucket nobody joined is not in the map
/// at all, which is why a module carrying only such a rule is genuinely empty
/// and says so.
///
/// This was first written as a recursive walk that asked each bucket whether
/// it drew anything. The mutation gate found the walk could never answer "no":
/// unreachable logic that documented a guard the code did not have.
fn subtree_draws_rows(node_key: &str, maps: &TreeWalkMaps<'_>) -> bool {
    let has = |present: Option<bool>| present.unwrap_or(false);
    has(maps
        .node_children
        .get(node_key)
        .map(|kids| !kids.is_empty()))
        || has(maps
            .buckets_of_node
            .get(node_key)
            .map(|buckets| !buckets.is_empty()))
}

/// Where a node's subtree first appears in workspace order: the minimum over
/// its own buckets and every descendant's — the recursive half of
/// order-by-first-member (TP-NODE-04).
fn subtree_first_ws(
    node: &str,
    node_children: &std::collections::HashMap<&str, Vec<&str>>,
    buckets_of_node: &std::collections::HashMap<String, Vec<String>>,
    first_ws_of_space: &std::collections::HashMap<String, usize>,
    guard: usize,
) -> usize {
    if guard == 0 {
        return usize::MAX;
    }
    let own = buckets_of_node
        .get(node)
        .into_iter()
        .flatten()
        .filter_map(|bucket| first_ws_of_space.get(bucket))
        .copied()
        .min();
    let descendants = node_children
        .get(node)
        .into_iter()
        .flatten()
        .map(|child| {
            subtree_first_ws(
                child,
                node_children,
                buckets_of_node,
                first_ws_of_space,
                guard - 1,
            )
        })
        .min();
    own.into_iter()
        .chain(descendants)
        .min()
        .unwrap_or(usize::MAX)
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
    parented: bool,
) {
    let Some(space) =
        effective_space(app, ws_idx).filter(|space| grouped_keys.contains(&space.key))
    else {
        // TP-NODE-05: under a node, a bucket too small for a header of its
        // own hangs its member straight below the node, indented — "move
        // this branch under X" reads as exactly that.
        entries.push(WorkspaceListEntry::Workspace {
            ws_idx,
            indented: parented,
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

pub(crate) fn workspace_list_rect(
    area: Rect,
    split_ratio: f32,
    chrome: crate::ui::shell::SidebarChrome,
) -> Rect {
    let (ws_area, _) = expanded_sidebar_sections(area, split_ratio, chrome);
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

pub(crate) fn workspace_list_body_rect(
    area: Rect,
    has_scrollbar: bool,
    chrome: crate::ui::shell::SidebarChrome,
) -> Rect {
    if area.width == 0 || area.height <= WORKSPACE_SECTION_HEADER_ROWS {
        return Rect::default();
    }

    let body_y = area.y.saturating_add(WORKSPACE_SECTION_HEADER_ROWS);
    let footer_y = area.y + area.height.saturating_sub(chrome.footer_rows());
    let body_height = footer_y.saturating_sub(body_y);
    let body_width = area.width.saturating_sub(u16::from(has_scrollbar));
    Rect::new(area.x, body_y, body_width, body_height)
}

fn workspace_list_visible_count(app: &AppState, area: Rect, scroll: usize) -> usize {
    let body = workspace_list_body_rect(area, false, app.sidebar_chrome);
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
        // TP-DAILY-02: one line each, and the section's last row carries the
        // usual gap so the tree below starts as its own block rather than
        // running on from the daily chats.
        WorkspaceListEntry::DailyHeader => Some((1, 0)),
        WorkspaceListEntry::DailyChat { .. }
        | WorkspaceListEntry::DailyMore { .. }
        // TP-DAILY-18: the workspace switch measures like the chat switch —
        // it is the same kind of row in the same section.
        | WorkspaceListEntry::DailyMoreWorkspaces { .. }
        // TP-CHAT-MOVE-06: a container's chat measures like every other
        // chat row; it is the same kind of thing in a different home.
        | WorkspaceListEntry::ModuleChat { .. } => {
            Some((1, workspace_entry_gap(app, entries, entry_idx)))
        }
        WorkspaceListEntry::Workspace { ws_idx, indented } => {
            let workspace = app.workspaces.get(*ws_idx)?;
            Some((
                workspace_row_height_in_body(app, workspace, *indented, body_height),
                workspace_entry_gap(app, entries, entry_idx),
            ))
        }
        // The note stands on its own: no workspace to look up, and the gap
        // rule the drawer rows follow applies here too.
        WorkspaceListEntry::EmptyModule { .. } => {
            Some((1, workspace_entry_gap(app, entries, entry_idx)))
        }
        WorkspaceListEntry::Chat { ws_idx, .. }
        | WorkspaceListEntry::NoChats { ws_idx }
        | WorkspaceListEntry::MoreChats { ws_idx, .. } => {
            app.workspaces.get(*ws_idx)?;
            // A drawer's last row still has to separate its block from the
            // next one, or two repositories run together while a drawer is open.
            Some((1, workspace_entry_gap(app, entries, entry_idx)))
        }
    }
}

fn workspace_list_bottom_start(app: &AppState, area: Rect) -> usize {
    let body = workspace_list_body_rect(area, false, app.sidebar_chrome);
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
    let body = workspace_list_body_rect(area, true, app.sidebar_chrome);
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
    tokens::agent_rows(&app.sidebar_agents, entry.into(), label)
}

/// A ghost's rows, laid out by the very function the living use.
///
/// TP-AGPANEL-42: the user asked for the closed agents to be drawn "in the
/// same format as the open ones, the only difference being that they are a
/// faded grey". Calling the same layout is how that is kept true — a second
/// drawing path would be free to drift, and the one it replaces had already
/// drifted all the way to a bare `Paragraph` of one line.
///
/// Where a living row says what it is doing, a ghost says when it stopped. The
/// state slot cannot stay empty (the layout would collapse) and cannot carry a
/// live state (it has none), and the age is the one fact a graveyard exists to
/// answer — "let me see the ones opened and closed in the last month" was the
/// request that produced this section.
// TP-AGPANEL-44
fn resolved_ghost_rows(
    app: &AppState,
    record: &crate::app::closed_agents::ClosedAgentRecord,
    now: std::time::SystemTime,
) -> Vec<Vec<ResolvedToken>> {
    let empty = std::collections::HashMap::new();
    let name = ghost_display_label(record);
    tokens::agent_rows(
        &app.sidebar_agents,
        tokens::AgentTokenContext {
            agent: None,
            primary_label: &name,
            primary_tab_label: None,
            pane_label: None,
            agent_label: None,
            terminal_title: None,
            terminal_title_stripped: None,
            tokens: &empty,
        },
        &ghost_age_label(record, now),
    )
}

/// The name a headstone wears, with the static ellipsis a revival adds.
///
/// TP-AGPANEL-22 put the ellipsis on the name and it stays there: state, not
/// animation — nothing ticks for a ghost. It rides the name rather than the
/// age because the name is the one thing a headstone always has, so a row
/// layout that asks for no state text still shows that a revival is under way.
fn ghost_display_label(record: &crate::app::closed_agents::ClosedAgentRecord) -> String {
    if record.revival == crate::app::closed_agents::RevivalState::Reviving {
        format!("{} …", record.label)
    } else {
        record.label.clone()
    }
}

/// How long ago a ghost closed, in the panel's own relative vocabulary.
///
/// TP-AGPANEL-44: where a living row says what it is doing, a headstone says
/// when it stopped. The state slot cannot stay empty — the layout would
/// collapse — and cannot carry a live state, because a ghost has none. "let me
/// see the ones opened and closed in the last month" is the request this
/// section exists to answer, and the age is the half of it the row can show.
fn ghost_age_label(
    record: &crate::app::closed_agents::ClosedAgentRecord,
    now: std::time::SystemTime,
) -> String {
    let closed_at = std::time::UNIX_EPOCH + std::time::Duration::from_millis(record.closed_at);
    format_relative_time(closed_at, now)
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

    let entries = agent_panel_entries(app);
    let ghosts = ghost_row_layouts(app);
    let rows = agent_panel_rows(app, entries.len());
    let mut used_rows = 0u16;
    let mut visible = 0usize;
    for index in scroll..rows.len() {
        let height = agent_panel_row_height(app, rows[index], &entries, &ghosts, body.height);
        if used_rows.saturating_add(height) > body.height {
            break;
        }
        used_rows = used_rows.saturating_add(height);
        visible += 1;
        used_rows = used_rows
            .saturating_add(agent_panel_row_gap(app, &rows, index))
            .min(body.height);
    }
    visible
}

/// Where every row of the panel is painted, and how tall it is.
///
/// TP-AGPANEL-43: one walk feeds the painter AND both hit tests, so a row's
/// click box can never sit beside its paint — the rule TP-CHROME-15/16 pinned
/// for the collapse control, applied to a list.
pub(crate) fn agent_panel_placements(app: &AppState, area: Rect) -> Vec<(AgentPanelRow, u16, u16)> {
    let metrics = agent_panel_scroll_metrics(app, area);
    let body = agent_panel_body_rect(area, should_show_scrollbar(metrics));
    if body == Rect::default() || body.height == 0 {
        return Vec::new();
    }
    let entries = agent_panel_entries(app);
    let ghosts = ghost_row_layouts(app);
    let rows = agent_panel_rows(app, entries.len());
    let scroll = app.agent_panel_scroll.min(metrics.max_offset_from_bottom);

    let mut placements = Vec::new();
    let mut row_y = body.y;
    let body_bottom = body.y + body.height;
    for index in scroll..rows.len() {
        let height = agent_panel_row_height(app, rows[index], &entries, &ghosts, body.height);
        if row_y.saturating_add(height) > body_bottom {
            break;
        }
        placements.push((rows[index], row_y, height));
        row_y = row_y
            .saturating_add(height)
            .saturating_add(agent_panel_row_gap(app, &rows, index))
            .min(body_bottom);
    }
    placements
}

/// One thing the agents panel scrolls past: a running agent, the divider, or a
/// closed one.
///
/// TP-AGPANEL-43: the graveyard used to be drawn into whatever space the
/// living left over — it took no part in the scroll metrics, the ceiling
/// reserved two rows for it, and the painter filled the remainder newest-first
/// and clipped the rest. Measured on the reported machine that meant 62 ghosts
/// of which at most a handful could ever be seen, and NO scroll position
/// reached the others: the user asked to see them "one by one, in a scrollable
/// area", and the panel was structurally incapable of it.
///
/// Making them rows of the same list is what makes them reachable. Scrolling,
/// visible-count, hit testing and painting then all read one sequence, so a
/// ghost cannot be reachable by one and invisible to another.
#[derive(Clone, Copy)]
pub(crate) enum AgentPanelRow {
    Live(usize),
    Separator,
    Ghost(usize),
}

/// The panel's whole scrollable sequence: the living, then — only if there are
/// any ghosts — a divider and the ghosts.
///
/// TP-AGPANEL-29 survives here rather than as a reserved-rows constant: the
/// separator is emitted only alongside at least one ghost, so a divider
/// dividing nothing remains impossible by construction.
pub(crate) fn agent_panel_rows(app: &AppState, live_count: usize) -> Vec<AgentPanelRow> {
    let mut rows: Vec<AgentPanelRow> = (0..live_count).map(AgentPanelRow::Live).collect();
    let ghosts = app.closed_agents.entries().count();
    if ghosts > 0 {
        rows.push(AgentPanelRow::Separator);
        rows.extend((0..ghosts).map(AgentPanelRow::Ghost));
    }
    rows
}

/// How tall one row of that sequence is, in the body it is drawn into.
fn agent_panel_row_height(
    app: &AppState,
    row: AgentPanelRow,
    entries: &[AgentPanelEntry],
    ghost_rows: &[Vec<Vec<ResolvedToken>>],
    body_height: u16,
) -> u16 {
    match row {
        AgentPanelRow::Live(idx) => entries
            .get(idx)
            .map(|entry| agent_entry_height_in_body(app, entry, body_height))
            .unwrap_or(0),
        AgentPanelRow::Separator => 1.min(body_height),
        AgentPanelRow::Ghost(idx) => ghost_rows
            .get(idx)
            .map(|rows| (rows.len().max(1) as u16).min(body_height))
            .unwrap_or(0),
    }
}

/// The gap under a row. Only the living carry one: the divider is already the
/// separation the graveyard needs, and spacing the ghosts apart would spend
/// the panel's scarcest resource — rows — on air.
fn agent_panel_row_gap(app: &AppState, rows: &[AgentPanelRow], index: usize) -> u16 {
    match rows.get(index) {
        Some(AgentPanelRow::Live(_)) if index + 1 < rows.len() => app.sidebar_agents.row_gap,
        _ => 0,
    }
}

/// Every ghost's laid-out rows, computed once per frame.
///
/// `SystemTime::now()` is read here rather than inside the layout so the whole
/// panel agrees on one instant; two ghosts closed a second apart must not be
/// able to disagree about what "now" was.
fn ghost_row_layouts(app: &AppState) -> Vec<Vec<Vec<ResolvedToken>>> {
    let now = std::time::SystemTime::now();
    app.closed_agents
        .entries()
        .map(|record| resolved_ghost_rows(app, record, now))
        .collect()
}

fn agent_panel_bottom_start(app: &AppState, area: Rect) -> usize {
    let body = agent_panel_body_rect(area, false);
    let entries = agent_panel_entries(app);
    let ghosts = ghost_row_layouts(app);
    let rows = agent_panel_rows(app, entries.len());
    if rows.is_empty() {
        return 0;
    }
    let mut used_rows = 0u16;
    let mut start = rows.len();
    for index in (0..rows.len()).rev() {
        let gap = agent_panel_row_gap(app, &rows, index);
        let needed = agent_panel_row_height(app, rows[index], &entries, &ghosts, body.height)
            .saturating_add(gap);
        if used_rows.saturating_add(needed) > body.height {
            break;
        }
        used_rows = used_rows.saturating_add(needed);
        start = index;
    }
    start.min(rows.len().saturating_sub(1))
}

/// Where the graveyard paints: the separator row's y and each ghost row's y,
/// walked with the same math the live entries use. One function feeds both
/// the painter and the hit test so the two can never drift (the rule
/// TP-CHROME-15/16 pinned for the collapse control). Ghosts fill whatever
/// space is left under the live entries, newest first, and clip from the
/// oldest end; a separator with no room for a single row under it is not
/// drawn at all — a divider dividing nothing reads as a rendering bug.
/// The graveyard's placements in the shape the tests that predate the unified
/// list assert on: the separator's row, then each headstone's first row.
///
/// Test-facing on purpose. Production reads `agent_panel_placements` directly —
/// this is the older, narrower view of the same walk, kept so the
/// characterization tests written against the leftover-space graveyard keep
/// guarding the behaviour they were written for.
#[cfg(test)]
pub(crate) fn closed_agent_row_slots(app: &AppState, area: Rect) -> Option<(u16, Vec<u16>)> {
    let placements = agent_panel_placements(app, area);
    let separator_y = placements
        .iter()
        .find_map(|(row, y, _)| matches!(row, AgentPanelRow::Separator).then_some(*y))?;
    let ghosts: Vec<u16> = placements
        .iter()
        .filter_map(|(row, y, _)| matches!(row, AgentPanelRow::Ghost(_)).then_some(*y))
        .collect();
    (!ghosts.is_empty()).then_some((separator_y, ghosts))
}

/// The ghost whose card covers `row`, by index.
///
/// TP-AGPANEL-43: a ghost is a card now, not a line, so a press anywhere on it
/// counts. Matching only its first row would make the lower half of a two-row
/// headstone silently dead — the class of defect the shared layout exists to
/// prevent.
pub(crate) fn closed_agent_index_at(app: &AppState, area: Rect, row: u16) -> Option<usize> {
    agent_panel_placements(app, area)
        .into_iter()
        .find_map(|(kind, y, height)| match kind {
            AgentPanelRow::Ghost(idx) if row >= y && row < y.saturating_add(height.max(1)) => {
                Some(idx)
            }
            _ => None,
        })
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
/// Every laid-out row of the Spaces list, split by what a press there means:
/// cards switch, chat rows resume, headers fold, and the "older chats" rows
/// open a drawer the rest of the way. One vector per meaning, so a press can
/// never resolve as the wrong kind of row.
pub(crate) type WorkspaceListAreas = (
    Vec<crate::app::state::WorkspaceCardArea>,
    Vec<crate::app::state::WorkspaceChatRowArea>,
    Vec<crate::app::state::WorkspaceGroupHeaderArea>,
    Vec<crate::app::state::WorkspaceProjectHeaderArea>,
    Vec<crate::app::state::WorkspaceMoreChatsArea>,
    // The sixth carries no meaning for a press: it exists so the note gets
    // painted (TP-MOD-25).
    Vec<crate::app::state::WorkspaceEmptyModuleArea>,
    // The daily section's rows, gathered rather than spread across three more
    // tuple slots: they belong to one surface and are read together.
    DailySectionAreas,
    // TP-CHAT-MOVE-06: chat rows under declared containers. These must be
    // laid out, not merely emitted: the sidebar draws chat rows from the
    // laid-out areas, so a row with no area here is a row nobody ever sees
    // however green the emission tests are.
    Vec<crate::app::state::ModuleChatRowArea>,
);

/// The daily section's laid-out rows.
///
/// TP-DAILY-03/07: three gestures — fold, open the rest, resume a chat — and
/// so three separate targets. None of them carries a `ws_idx`, which is the
/// whole point: the section belongs to no workspace.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct DailySectionAreas {
    pub header: Option<Rect>,
    pub chats: Vec<crate::app::state::DailyChatRowArea>,
    pub more: Option<Rect>,
    /// TP-DAILY-18: the workspace switch's own target.
    pub more_workspaces: Option<Rect>,
}

pub(crate) fn compute_workspace_list_areas(app: &AppState, area: Rect) -> WorkspaceListAreas {
    let ws_area = workspace_list_rect(area, app.sidebar_section_split, app.sidebar_chrome);
    if ws_area == Rect::default() {
        return (
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            DailySectionAreas::default(),
            Vec::new(),
        );
    }

    let metrics = workspace_list_scroll_metrics(app, ws_area);
    let body =
        workspace_list_body_rect(ws_area, should_show_scrollbar(metrics), app.sidebar_chrome);
    if body.width == 0 || body.height == 0 {
        return (
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            DailySectionAreas::default(),
            Vec::new(),
        );
    }

    let scroll = app.workspace_scroll;
    let mut row_y = body.y;
    let body_bottom = body.y + body.height;
    let mut cards = Vec::new();
    let mut chat_rows = Vec::new();
    let mut module_chats = Vec::new();
    let mut group_headers = Vec::new();
    let mut project_headers = Vec::new();
    let mut more_chats = Vec::new();
    let mut empty_modules = Vec::new();
    let mut daily = DailySectionAreas::default();

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
            // TP-DRAW-11: the "older" row is the way into the rest of the
            // drawer and back out again, so it earns a rect of its own — in
            // its own vector, because a click here must never resolve as the
            // chat row above it.
            WorkspaceListEntry::MoreChats { ws_idx, .. } => {
                more_chats.push(crate::app::state::WorkspaceMoreChatsArea {
                    rect,
                    ws_idx: *ws_idx,
                });
            }
            // TP-DAILY-03/07: three gestures, three vectors, no `ws_idx` in
            // any of them — the section belongs to no workspace, and folding
            // it into a workspace-indexed vector is how a press would land on
            // whichever checkout happened to share the row.
            WorkspaceListEntry::DailyHeader => {
                daily.header = Some(rect);
            }
            WorkspaceListEntry::DailyChat { chat_idx } => {
                daily.chats.push(crate::app::state::DailyChatRowArea {
                    rect,
                    chat_idx: *chat_idx,
                });
            }
            WorkspaceListEntry::DailyMore { .. } => {
                daily.more = Some(rect);
            }
            // TP-DAILY-18: its own rect, not the chat switch's. Sharing one
            // would make a press on either toggle both — two switches one
            // column apart doing each other's job is the confusion TP-TREE-01
            // split rows to prevent.
            WorkspaceListEntry::DailyMoreWorkspaces { .. } => {
                daily.more_workspaces = Some(rect);
            }
            // TP-MOD-25: laid out to be drawn, not to be pressed.
            WorkspaceListEntry::EmptyModule { node_key } => {
                empty_modules.push(crate::app::state::WorkspaceEmptyModuleArea {
                    rect,
                    node_key: node_key.clone(),
                });
            }
            // TP-CHAT-MOVE-06: a container's chat row is laid out like any
            // other chat row. This is not optional bookkeeping — the sidebar
            // paints chat rows from these areas, so skipping the arm would
            // leave the row emitted, measured, tested green, and invisible.
            WorkspaceListEntry::ModuleChat { node_key, chat_idx } => {
                module_chats.push(crate::app::state::ModuleChatRowArea {
                    rect,
                    node_key: node_key.clone(),
                    chat_idx: *chat_idx,
                });
            }
            // The empty-drawer placeholder occupies a row but stays inert:
            // there is nothing behind it to open.
            WorkspaceListEntry::NoChats { .. } => {}
        }
        row_y = row_y
            .saturating_add(row_height)
            .saturating_add(gap)
            .min(body_bottom);
    }

    (
        cards,
        chat_rows,
        group_headers,
        project_headers,
        more_chats,
        empty_modules,
        daily,
        module_chats,
    )
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
    chrome: crate::ui::shell::SidebarChrome,
) -> Option<u16> {
    if area.height == 0 {
        return None;
    }
    let list_bottom = area.y + area.height.saturating_sub(chrome.footer_rows());

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

    let (ws_area, detail_area) =
        expanded_sidebar_sections(area, app.sidebar_section_split, app.sidebar_chrome);

    // Each half wears its frame before its content draws, into the rectangle
    // the content was already inset out of — the same inset the hit areas came
    // from, because both read `expanded_sidebar_sections`.
    let (ws_frame, detail_frame) = expanded_sidebar_section_frames(area, app.sidebar_section_split);
    for (frame_area, tint) in [
        (ws_frame, app.sidebar_chrome.spaces),
        (detail_frame, app.sidebar_chrome.agents),
    ] {
        let Some(tint) = tint else { continue };
        if frame_area.width < 3 || frame_area.height < 3 {
            continue;
        }
        crate::ui::widgets::render_bar_shell(frame, frame_area, tint, app.palette.panel_bg);
    }

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
        }) => workspace_drop_indicator_row(
            &app.view.workspace_card_areas,
            area,
            *insert_idx,
            app.sidebar_chrome,
        ),
        _ => None,
    };

    let list_bottom = area.y + area.height.saturating_sub(app.sidebar_chrome.footer_rows());
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
    // TP-DAILY-14: computed once, outside the loop. The answer is needed per
    // card but the question is about the whole list, and asking it per card
    // would walk every workspace for every row.
    let daily_owned = daily_owned_workspaces(app);
    // TP-DAILY-15/16: resolved for the whole set here, for the same reason
    // `daily_owned` is — the question is about the list, and asking it per card
    // would both walk every workspace per row and lose the one fact that makes
    // the answer useful: what the *other* rows are called.
    let daily_names = daily_row_names(app, &daily_owned, Some(terminal_runtimes));

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
        let chat_carries_accent =
            is_active && visible_active_chat(app).is_some_and(|(w, _)| w == i);

        if highlighted {
            // TP-TREE-11: the accent lives on exactly one focus carrier. The
            // workspace you are in wears it outright — the same sentence the
            // active agent card and the active tab speak — unless the drawer
            // below shows the very chat the active tab resumes; the accent
            // then descends to that row (TP-FOCUS-01) and the card keeps a
            // quiet active tone so the two never wear it at once. Selection
            // while navigating stays a tone apart, so "where I am" and "what
            // I am pointing at" never read alike.
            let bg = if is_active && !chat_carries_accent {
                p.accent
            } else if is_active {
                p.surface_dim
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

        let name_style = if is_active && !chat_carries_accent {
            Style::default()
                .fg(panel_contrast_fg(p))
                .add_modifier(Modifier::BOLD)
        } else if is_active || selected || is_dragged {
            // Active with the accent down on its chat row: bold carries the
            // activeness, the contrast ink would be unreadable off-accent.
            Style::default().fg(p.text).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(p.subtext0)
        };

        let label = ws.display_name_from(&app.terminals, terminal_runtimes);
        let display_label = if card.indented {
            // TP-DAILY-14: `indented` says a row is drawn one level in, and it
            // was carrying a second meaning it never earned — "this row is a
            // checkout under a repository header", which is what makes a
            // branch name the right label there. The daily area's own rows
            // share the flag and nothing else: they have no checkout, so the
            // branch they showed came from whatever repository happened to
            // contain the daily directory. On the machine this was reported
            // from that was `$HOME`, and seven rows all read `main`.
            //
            // #94 moved those rows under the daily header and, without meaning
            // to, moved their labels too — before it they read `ayaz`, after it
            // `main`. Withholding the branch was the first half of the fix; the
            // second is that `ayaz` seven times says no more than `main` seven
            // times did, so the name arrives resolved (TP-DAILY-15/16).
            indented_row_label(
                &label,
                ws.branch().as_deref(),
                ws.custom_name.is_some(),
                daily_names.get(&i).map(String::as_str),
            )
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
        let project_shift = workspace_node_shift(app, i);
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
        // The "⋯" rides with the "+" with one breathing cell between them
        // (TP-DOTS-03/09), so mouse chrome costs three columns.
        let trailing = u16::from(show_plus) * 3
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
                // TP-FOCUS-04: the chrome speaks the name's sentence —
                // contrast ink on the accent, text ink on the quiet
                // active tone the card steps back to.
                let chrome_fg = if is_active && !chat_carries_accent {
                    panel_contrast_fg(p)
                } else if is_active {
                    p.text
                } else if selected {
                    p.accent
                } else {
                    p.overlay0
                };
                let plus = workspace_new_chat_cell(card.rect);
                if plus.width > 0 {
                    frame.buffer_mut()[(plus.x, plus.y)]
                        .set_symbol("+")
                        .set_style(Style::default().fg(chrome_fg));
                }
                // TP-DOTS-03: the manage road, one column in — the second
                // door to the menu the right-click already opens.
                let dots = workspace_menu_cell(card.rect);
                if dots.width > 0 {
                    frame.buffer_mut()[(dots.x, dots.y)]
                        .set_symbol("⋯")
                        .set_style(Style::default().fg(chrome_fg));
                }
            }
            if let Some(text) = badge.as_ref() {
                let width = text.len() as u16;
                let x = right
                    .saturating_sub(if show_plus { 4 } else { 0 })
                    .saturating_sub(width);
                let x = x.max(card.rect.x);
                if x > card.rect.x {
                    frame.render_widget(
                        Paragraph::new(Span::styled(
                            text.clone(),
                            Style::default().fg(if is_active && !chat_carries_accent {
                                panel_contrast_fg(p)
                            } else if is_active {
                                p.text
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
    render_daily_section(app, frame, list_bottom);
    // TP-CHAT-MOVE-06: container chat rows draw from their own laid-out
    // areas, alongside the daily section's.
    render_module_chat_rows(app, frame, list_bottom);
    render_workspace_empty_module_rows(app, frame, list_bottom);
    render_workspace_more_chats_rows(app, frame, list_bottom);

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

    // TP-FOCUS-SW-04: the Spaces tab's filter toggle wears the same clothes
    // as the Projects tab's "actives" in the same slot — accent while the
    // tree is narrowed, dim while it is whole, and mouse chrome either way.
    if app.mouse_capture {
        let toggle = app.sidebar_focus_toggle_rect();
        if toggle.width > 0 {
            let style = if app.spaces_focus_only {
                Style::default().fg(p.accent).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(p.overlay0)
            };
            let framed = app.sidebar_chrome.chips.and_then(|tint| {
                crate::ui::widgets::render_chip(frame, toggle, "focus", tint, style, p.panel_bg)
            });
            if framed.is_none() {
                frame.render_widget(Paragraph::new(Span::styled("focus", style)), toggle);
            }
        }
    }
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
        // TP-MOD-09: the row carries a project key or a module key, and both
        // are drawn. Looking the key up against projects alone left every
        // module row painted with nothing at all.
        let Some(face) = header_face_for_key(app, &head.project_key) else {
            continue;
        };
        let collapsed = app.node_folded(&head.project_key);
        let mut spans = Vec::new();
        // TP-MOD-10: one step per ancestor, the same measure the module
        // headers and the checkouts below already use, so a sub-module reads
        // as under its parent and a parallel one reads as beside it. The
        // depth comes from the node chain rather than from a member, because
        // a module the user just created has no member to ask.
        let shift = node_depth(app, &head.project_key);
        if shift > 0 {
            spans.push(Span::raw(" ".repeat((shift * ROW_INDENT_STEP) as usize)));
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
        // TP-ICON-02: the container's own icon wins; the configured default
        // fills in; an empty string means no glyph at all.
        let icon = face.icon.unwrap_or(app.space_icons.project.as_str()).trim();
        if !icon.is_empty() {
            spans.push(Span::raw(format!("{icon} ")));
        }
        // TP-MOD-11: only a project answers for the checkouts it hides.
        if collapsed && face.aggregates_state {
            let (state, seen) = project_aggregate_state(app, &head.project_key);
            let (glyph, glyph_style) = state_dot(state, seen, p);
            spans.push(Span::styled(glyph, glyph_style));
            spans.push(Span::raw(" "));
        }
        let used = spans
            .iter()
            .map(|span| super::text::display_width(span.content.as_ref()))
            .sum::<usize>();
        // TP-DOTS-09: the manage chrome `[⋯] [+]` is reserved, so a long
        // name truncates short of it instead of bleeding underneath.
        let reserved = if app.mouse_capture { 4 } else { 0 };
        spans.push(Span::styled(
            super::text::truncate_end(
                face.name,
                (head.rect.width as usize).saturating_sub(used + reserved),
            ),
            Style::default().fg(p.text).add_modifier(Modifier::BOLD),
        ));
        frame.render_widget(Paragraph::new(Line::from(spans)), head.rect);
        draw_header_menu_dots(app, frame, head.rect);
    }
}

/// TP-DOTS-03: the "⋯" every header row wears while the mouse owns the
/// sidebar — the visible door to the menu right-click already opens.
/// TP-DOTS-17: the "+" beside it, one breathing cell to the right — the
/// visible door to the module's "New branch..." road.
fn draw_header_menu_dots(app: &AppState, frame: &mut Frame, head_rect: Rect) {
    if !app.mouse_capture {
        return;
    }
    let dots = header_menu_cell(head_rect);
    if dots.width == 0 {
        return;
    }
    frame.buffer_mut()[(dots.x, dots.y)]
        .set_symbol("⋯")
        .set_style(Style::default().fg(app.palette.overlay0));
    let plus = header_new_branch_cell(head_rect);
    if plus.width == 0 {
        return;
    }
    frame.buffer_mut()[(plus.x, plus.y)]
        .set_symbol("+")
        .set_style(Style::default().fg(app.palette.overlay0));
}

fn render_workspace_group_headers(app: &AppState, frame: &mut Frame, list_bottom: u16) {
    let p = &app.palette;
    for head in &app.view.workspace_group_header_areas {
        if head.rect.width == 0 || head.rect.y >= list_bottom {
            continue;
        }
        let collapsed = app.collapsed_space_keys.contains(&head.space_key);
        let mut spans = Vec::new();
        // TP-PROJ-GROUP-01, generalised: inside a node chain the module
        // header steps in once per ancestor, so the umbrella, its nodes,
        // their modules and the checkouts read as one tree.
        let shift = space_node_shift_for_key(app, &head.space_key);
        if shift > 0 {
            spans.push(Span::raw(" ".repeat((shift * ROW_INDENT_STEP) as usize)));
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
        // TP-DOTS-09: same reservation as the project header above.
        let reserved = if app.mouse_capture { 4 } else { 0 };
        spans.push(Span::styled(
            super::text::truncate_end(
                &space_label_for_key(app, &head.space_key),
                (head.rect.width as usize).saturating_sub(used + reserved),
            ),
            Style::default().fg(p.text).add_modifier(Modifier::BOLD),
        ));
        frame.render_widget(Paragraph::new(Line::from(spans)), head.rect);
        draw_header_menu_dots(app, frame, head.rect);
    }
}

/// Draw the chat rows of every open drawer.
///
/// TP-WSCHAT-20: a chat row must read as belonging to its workspace and never
/// as a workspace itself — it is indented past the branch children, carries a
/// chat glyph rather than a state dot, and never takes the accent BACKGROUND
/// that marks the active workspace and the active agent card.
/// Draw the "older chats" row of every drawer that has one.
///
/// TP-DRAW-11: the row says which way it goes — how many chats are still
/// hidden, or that the drawer can be folded back. Before this it was laid
/// out but never painted, so the desktop drawer ended in a blank line the
/// reader could neither understand nor act on.
fn render_workspace_more_chats_rows(app: &AppState, frame: &mut Frame, list_bottom: u16) {
    let p = &app.palette;
    for row in &app.view.workspace_more_chats_areas {
        if row.rect.width == 0 || row.rect.y >= list_bottom {
            continue;
        }
        let total = workspace_chat_rows_for(app, row.ws_idx).len();
        let label = if workspace_chat_drawer_expanded(app, row.ws_idx) {
            "   … fewer".to_string()
        } else {
            format!(
                "   … {} older",
                total.saturating_sub(WORKSPACE_CHAT_ROW_LIMIT)
            )
        };
        frame.render_widget(
            Paragraph::new(Span::styled(
                super::text::truncate_end(&label, row.rect.width as usize),
                Style::default().fg(p.overlay0),
            )),
            row.rect,
        );
    }
}

/// The daily section's own three row kinds.
///
/// TP-DAILY-02/03/04: the header states what the section is and whether it is
/// open; the chats speak the same visual language as every other chat row, so
/// a reader learns one dialect rather than two; the switch says how many are
/// hidden and how to get back. Drawn from the section's own area vectors, so
/// a row here can never be painted over a workspace's.
/// Draw one chat row: the live marker, the icon, the title, and the age.
///
/// TP-CHAT-MOVE-06: the daily section and declared containers both draw chat
/// rows, and they have to look identical — a conversation that changes
/// appearance because of where it was filed reads as a different kind of
/// thing. `indent` is the only difference either surface may have.
fn render_chat_row(
    app: &AppState,
    frame: &mut Frame,
    rect: Rect,
    chat: &crate::app::state::WorkspaceChatRow,
    now: std::time::SystemTime,
    indent: u16,
) {
    let p = &app.palette;
    let wired = app.find_resumed_chat_tab(&chat.session_id);
    let focused = wired.is_some_and(|(ws_idx, tab_idx)| {
        app.active == Some(ws_idx)
            && app
                .workspaces
                .get(ws_idx)
                .is_some_and(|ws| ws.active_tab_index() == tab_idx)
    });
    let marker = if wired.is_some() { "\u{25cf} " } else { "  " };
    let (title_style, marker_style) = if focused {
        (
            Style::default()
                .fg(panel_contrast_fg(p))
                .bg(p.accent)
                .add_modifier(Modifier::BOLD),
            Style::default().fg(panel_contrast_fg(p)).bg(p.accent),
        )
    } else if wired.is_some() {
        (Style::default().fg(p.text), Style::default().fg(p.accent))
    } else {
        (Style::default().fg(p.overlay1), Style::default())
    };
    let icon = app.space_icons.chat.trim();
    let icon_span = if icon.is_empty() {
        String::new()
    } else {
        format!("{icon} ")
    };
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
    let width = rect.width as usize;
    let prefix_width = usize::from(indent) + marker.len() + super::text::display_width(&icon_span);
    let title_budget = width
        .saturating_sub(prefix_width)
        .saturating_sub(if age_width > 0 { age_width + 1 } else { 0 });
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::raw(" ".repeat(usize::from(indent))),
            Span::styled(marker, marker_style),
            Span::raw(icon_span),
            Span::styled(
                super::text::truncate_end(&chat.display_label(), title_budget),
                title_style,
            ),
        ])),
        rect,
    );
    if age_width > 0 && age_width < width {
        frame.render_widget(
            Paragraph::new(Span::styled(age, Style::default().fg(p.overlay0)))
                .alignment(Alignment::Right),
            rect,
        );
    }
}

/// Draw the chat rows filed under declared containers.
///
/// TP-CHAT-MOVE-06: these are laid out by `compute_workspace_list_areas` like
/// every other chat row, and drawn from those areas — which is why the area
/// arm is not optional bookkeeping. They sit one step further in than a daily
/// row, because they *do* hang off a header.
fn render_module_chat_rows(app: &AppState, frame: &mut Frame, list_bottom: u16) {
    let now = std::time::SystemTime::now();
    for row in &app.view.module_chat_row_areas {
        if row.rect.width == 0 || row.rect.y >= list_bottom {
            continue;
        }
        let chats = module_chat_rows(app, &row.node_key);
        let Some(chat) = chats.get(row.chat_idx) else {
            continue;
        };
        render_chat_row(app, frame, row.rect, chat, now, DISCLOSURE_WIDTH + 2);
    }
}

fn render_daily_section(app: &AppState, frame: &mut Frame, list_bottom: u16) {
    let p = &app.palette;
    let now = std::time::SystemTime::now();
    let chats = daily_chat_rows(app);

    if let Some(rect) = app.view.daily_header_area {
        if rect.width > 0 && rect.y < list_bottom {
            let arrow = if app.daily_section_collapsed {
                DISCLOSURE_CLOSED
            } else {
                DISCLOSURE_OPEN
            };
            // TP-DAILY-12: the section is drawn in the containers' dialect,
            // not in a section-title's. It sits at the top of the same tree
            // the projects sit in, so reading it as a heading rather than as
            // a place — a coloured arrow, an icon, a bold name and the manage
            // chrome every container header wears — was the difference
            // between "a label above the tree" and "the area my day lives in".
            let mut spans = vec![
                Span::styled(arrow, Style::default().fg(p.accent)),
                Span::raw(" "),
            ];
            // TP-DAILY-12: its own icon, not the chat glyph. This row names a
            // place — the one that is always yours — and a speech bubble made
            // it read as one more conversation rather than as where they live.
            let icon = app.space_icons.daily.trim();
            if !icon.is_empty() {
                spans.push(Span::raw(format!("{icon} ")));
            }
            let used = spans
                .iter()
                .map(|span| super::text::display_width(span.content.as_ref()))
                .sum::<usize>();
            // TP-DOTS-09's measure: the trailing chrome is reserved so a name
            // truncates short of it instead of bleeding underneath. Here the
            // count lives in that lane too, so it reserves one column more.
            let reserved = if app.mouse_capture { 5 } else { 2 };
            spans.push(Span::styled(
                super::text::truncate_end(
                    DAILY_SECTION_TITLE,
                    (rect.width as usize).saturating_sub(used + reserved),
                ),
                Style::default().fg(p.text).add_modifier(Modifier::BOLD),
            ));
            frame.render_widget(Paragraph::new(Line::from(spans)), rect);
            // TP-DAILY-10: the "+" is mouse chrome, like every other plus on
            // this sidebar — drawn only while the mouse owns the panel. The
            // count steps left of it for exactly that long, so the two never
            // share a cell: information when the mouse is away, the action
            // when it is here.
            let plus = daily_new_chat_cell(rect);
            let plus_drawn = app.mouse_capture && plus.width > 0;
            if plus_drawn {
                frame.render_widget(
                    Paragraph::new(Span::styled("+", Style::default().fg(p.overlay1))),
                    plus,
                );
            }
            // TP-DAILY-12: the "⋯" every container header wears — the visible
            // door to the menu a right-click opens. Without it this row is the
            // only header on the sidebar a person cannot manage.
            draw_header_menu_dots(app, frame, rect);
            // The count answers "how much is in here" while the section is
            // folded — the one question a closed container cannot otherwise
            // answer, and the reason a fold is safe to leave closed.
            let count = chats.len().to_string();
            let count_room = if plus_drawn {
                rect.width.saturating_sub(4)
            } else {
                rect.width
            };
            if super::text::display_width(&count) < count_room as usize {
                frame.render_widget(
                    Paragraph::new(Span::styled(count, Style::default().fg(p.overlay0)))
                        .alignment(Alignment::Right),
                    Rect {
                        width: count_room,
                        ..rect
                    },
                );
            }
        }
    }

    for row in &app.view.daily_chat_row_areas {
        if row.rect.width == 0 || row.rect.y >= list_bottom {
            continue;
        }
        let Some(chat) = chats.get(row.chat_idx) else {
            continue;
        };
        // The section hangs off nothing, so its rows are indented by the
        // disclosure column alone — there is no checkout above them to draw a
        // guide down from.
        render_chat_row(app, frame, row.rect, chat, now, DISCLOSURE_WIDTH);
    }

    if let Some(rect) = app.view.daily_more_area {
        if rect.width > 0 && rect.y < list_bottom {
            let label = if app.daily_section_expanded {
                "   … fewer".to_string()
            } else {
                format!(
                    "   … {} older",
                    chats.len().saturating_sub(WORKSPACE_CHAT_ROW_LIMIT)
                )
            };
            frame.render_widget(
                Paragraph::new(Span::styled(
                    super::text::truncate_end(&label, rect.width as usize),
                    Style::default().fg(p.overlay0),
                )),
                rect,
            );
        }
    }

    // TP-DAILY-18: the workspace switch, in the chat switch's dialect. One
    // place, one row, and a way to look at the rest of what stands in it.
    if let Some(rect) = app.view.daily_more_workspaces_area {
        if rect.width > 0 && rect.y < list_bottom {
            let hidden = daily_owned_workspaces(app).len().saturating_sub(1);
            let label = if app.daily_workspaces_expanded {
                "   … fewer workspaces".to_string()
            } else {
                format!("   … {hidden} more here")
            };
            frame.render_widget(
                Paragraph::new(Span::styled(
                    super::text::truncate_end(&label, rect.width as usize),
                    Style::default().fg(p.overlay0),
                )),
                rect,
            );
        }
    }
}

/// The note an empty container writes under its own header.
///
/// TP-MOD-03: it is indented one step past the header so it reads as being
/// *inside* the container, and dimmed like the drawer's other stated absences
/// — it is a fact about the tree, not a row of the tree.
fn render_workspace_empty_module_rows(app: &AppState, frame: &mut Frame, list_bottom: u16) {
    let p = &app.palette;
    for row in &app.view.workspace_empty_module_areas {
        if row.rect.width == 0 || row.rect.y >= list_bottom {
            continue;
        }
        let indent = usize::from(ROW_INDENT_STEP).saturating_mul(usize::from(
            node_depth(app, &row.node_key).saturating_add(1),
        ));
        let label = format!("{}{EMPTY_MODULE_NOTE}", " ".repeat(indent));
        frame.render_widget(
            Paragraph::new(Span::styled(
                super::text::truncate_end(&label, row.rect.width as usize),
                Style::default().fg(p.overlay0),
            )),
            row.rect,
        );
    }
}

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
        // TP-FOCUS-01: the focused chat now wears the accent background the
        // workspace card gave up for it — one carrier, one answer to "where
        // am I", read the same way the active agent card reads.
        let (title_style, marker_style) = if focused {
            (
                Style::default()
                    .fg(panel_contrast_fg(p))
                    .bg(p.accent)
                    .add_modifier(Modifier::BOLD),
                Style::default().fg(panel_contrast_fg(p)).bg(p.accent),
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
            + workspace_node_shift(app, row.ws_idx))
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
// TP-CHROME-18/19: framed buttons when they fit, plain labels when they do not.
fn render_sidebar_footer_buttons(app: &AppState, frame: &mut Frame, area: Rect, new_label: &str) {
    let p = &app.palette;
    let list_bottom = area.y + area.height.saturating_sub(app.sidebar_chrome.footer_rows());
    if !(app.mouse_capture && list_bottom > area.y) {
        return;
    }

    let new_rect = app.sidebar_new_button_rect();
    let menu_rect = app.global_launcher_rect();
    let label_style = Style::default().fg(p.overlay0);

    // A chip is asked for, not assumed. Both controls are checked before either
    // is drawn: a half-applied decision would leave one framed button and one
    // bare label, and two frames that overlap interleave into something that
    // reads as neither. Whenever the answer is no, the footer keeps the labels
    // it has always drawn.
    if let Some(tint) = app.sidebar_chrome.chips {
        let disjoint = new_rect.x + new_rect.width <= menu_rect.x;
        let tall_enough = new_rect.height >= crate::ui::widgets::CHIP_ROWS;
        if disjoint && tall_enough {
            let drew_new = crate::ui::widgets::render_chip(
                frame,
                new_rect,
                new_label.trim(),
                tint,
                label_style,
                p.panel_bg,
            );
            let drew_menu = crate::ui::widgets::render_chip(
                frame,
                menu_rect,
                "menu",
                tint,
                label_style,
                p.panel_bg,
            );
            if drew_new.is_some() && drew_menu.is_some() {
                return;
            }
        }
    }

    frame.render_widget(
        Paragraph::new(Span::styled(new_label, label_style)),
        new_rect,
    );

    let menu_line = if app.global_menu_attention_badge_visible() {
        Line::from(vec![
            Span::styled(
                "● ",
                Style::default().fg(p.accent).add_modifier(Modifier::BOLD),
            ),
            Span::styled("menu", label_style),
        ])
    } else {
        Line::from(vec![Span::styled("menu", label_style)])
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
    let viewport_rows = workspace_list_body_rect(area, false, app.sidebar_chrome).height as usize;
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
    let body = workspace_list_body_rect(area, true, app.sidebar_chrome);
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
    let body = workspace_list_body_rect(area, has_scrollbar, app.sidebar_chrome);
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
            // This control shares the footer with the two beside it, so it
            // wears the same clothes: a rectangle already sized for a chip and
            // then drawn as a bare label would leave the frame's cells empty
            // and still clickable.
            let framed = app.sidebar_chrome.chips.and_then(|tint| {
                crate::ui::widgets::render_chip(frame, toggle, "actives", tint, style, p.panel_bg)
            });
            if framed.is_none() {
                frame.render_widget(Paragraph::new(Span::styled("actives", style)), toggle);
            }
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

    let control_label = active_agent_view_label(app)
        .unwrap_or_else(|| agent_panel_sort_label(app.agent_panel_sort));
    let name_style = Style::default().fg(p.overlay0).add_modifier(Modifier::BOLD);
    let control_style = Style::default()
        .fg(if app.agent_view_override.is_some() {
            p.accent
        } else {
            p.overlay0
        })
        .add_modifier(Modifier::BOLD);
    let name_rect = agent_panel_header_name_rect(area, "agents", app.sidebar_chrome);
    let toggle_rect = agent_panel_header_label_rect(area, control_label, app.sidebar_chrome);

    // With chips, the two labels become buttons in the header's own top
    // corners, and the frames replace the separator that used to divide the
    // halves -- so the list keeps every row it had. Either both are framed or
    // neither is: one boxed label beside one bare one reads as a rendering bug.
    let framed = app.sidebar_chrome.chips.filter(|_| {
        name_rect.x + name_rect.width <= toggle_rect.x && toggle_rect != Rect::default()
    });
    let drew_chips = framed.is_some_and(|tint| {
        let name = crate::ui::widgets::render_chip(
            frame, name_rect, "agents", tint, name_style, p.panel_bg,
        );
        let control = crate::ui::widgets::render_chip(
            frame,
            toggle_rect,
            control_label,
            tint,
            control_style,
            p.panel_bg,
        );
        name.is_some() && control.is_some()
    });

    if !drew_chips {
        let sep_line = "─".repeat(area.width as usize);
        frame.render_widget(
            Paragraph::new(Span::styled(&sep_line, Style::default().fg(p.surface_dim))),
            Rect::new(area.x, area.y, area.width, 1),
        );

        frame.render_widget(
            Paragraph::new(Line::from(vec![Span::styled(" agents", name_style)])),
            Rect::new(area.x, area.y + 1, area.width, 1),
        );
        if toggle_rect != Rect::default() {
            frame.render_widget(
                Paragraph::new(Span::styled(control_label, control_style))
                    .alignment(Alignment::Right),
                toggle_rect,
            );
        }
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

    // TP-AGPANEL-43: one walk, one sequence. The graveyard is no longer drawn
    // into leftover space after this loop — it IS part of this loop, which is
    // what makes every ghost reachable by scrolling.
    let ghost_layouts = ghost_row_layouts(app);
    let ghost_records: Vec<_> = app.closed_agents.entries().collect();
    for (kind, row_y, height) in agent_panel_placements(app, area) {
        let (index, detail) = match kind {
            AgentPanelRow::Live(index) => match details.get(index) {
                Some(detail) => (index, detail),
                None => continue,
            },
            AgentPanelRow::Separator => {
                let sep_line = "─".repeat(body.width as usize);
                frame.render_widget(
                    Paragraph::new(Span::styled(&sep_line, Style::default().fg(p.surface_dim))),
                    Rect::new(body.x, row_y, body.width, 1),
                );
                continue;
            }
            AgentPanelRow::Ghost(idx) => {
                render_ghost_card(
                    app,
                    frame,
                    body,
                    row_y,
                    height,
                    ghost_layouts.get(idx).map(Vec::as_slice).unwrap_or(&[]),
                    ghost_records.get(idx).copied(),
                );
                continue;
            }
        };
        let label_color = state_label_color(detail.state, detail.seen, p);
        let rows = resolved_agent_rows(app, detail);

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
        let _ = index;
    }

    if let Some(track) = scrollbar_rect {
        render_scrollbar(frame, metrics, track, p.surface_dim, p.overlay0, "▕");
    }
}

/// Draw one headstone, through the very span builder the living rows use.
///
/// TP-AGPANEL-22 keeps its third visual class here: the active card wears the
/// accent, passive rows drop bold, and a ghost dims *and* leans — dim alone
/// would collide with the passive agent token, which already dims. What
/// changed is only that the dimming is applied to a card rather than to a
/// bare line of text.
// TP-AGPANEL-42/44
fn render_ghost_card(
    app: &AppState,
    frame: &mut Frame,
    body: Rect,
    row_y: u16,
    height: u16,
    rows: &[Vec<ResolvedToken>],
    record: Option<&crate::app::closed_agents::ClosedAgentRecord>,
) {
    let p = &app.palette;
    let ghost_style = Style::default()
        .fg(p.overlay0)
        .add_modifier(Modifier::DIM | Modifier::ITALIC);
    // Nothing ticks for a ghost: the icon is asked for a static glyph, never
    // for a spinner frame.
    let state_icon = ("·", ghost_style);

    if rows.is_empty() {
        let label = record.map(ghost_display_label).unwrap_or_default();
        frame.render_widget(
            Paragraph::new(Span::styled(format!(" {label}"), ghost_style)),
            Rect::new(body.x, row_y, body.width, 1),
        );
        return;
    }

    for (row_index, resolved) in rows.iter().take(height.max(1) as usize).enumerate() {
        let mut spans = vec![Span::raw(if row_index == 0 { " " } else { "   " })];
        spans.extend(resolved_token_spans(
            resolved,
            state_icon,
            ghost_style,
            ghost_style,
            ghost_style,
            ghost_style,
            p,
            body.width
                .saturating_sub(if row_index == 0 { 1 } else { 3 }) as usize,
        ));
        frame.render_widget(
            Paragraph::new(Line::from(spans)),
            Rect::new(body.x, row_y + row_index as u16, body.width, 1),
        );
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

/// The collapse control's cell, in the bottom-right of the sidebar's own
/// content.
///
/// The control belongs *inside* the agents half; it is not part of that half's
/// decoration. So when the agents half wears a frame, the last row and column
/// are the frame's own and the control steps one cell inwards — otherwise the
/// icon lands on the corner glyph and the border reads as broken. Both the
/// drawing and the hit test call this one function, so the inset can never
/// drift between what is painted and what is clickable.
// TP-CHROME-15/16: the control steps inside the frame, and the click follows it.
pub(crate) fn expanded_sidebar_toggle_rect(
    area: Rect,
    chrome: crate::ui::shell::SidebarChrome,
) -> Rect {
    if area.width <= 1 || area.height == 0 {
        return Rect::default();
    }
    let inset = u16::from(chrome.agents.is_some());
    if area.width <= 2 + inset || area.height <= inset {
        return Rect::default();
    }
    Rect::new(
        area.x + area.width.saturating_sub(2 + inset),
        area.y + area.height.saturating_sub(1 + inset),
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
        expanded_sidebar_toggle_rect(area, app.sidebar_chrome)
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
    use crate::app::test_wait::LoadAwareDeadline;
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
        let (_, agent_area) =
            expanded_sidebar_sections(area, app.sidebar_section_split, app.sidebar_chrome);
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
        let (_, agent_area) =
            expanded_sidebar_sections(area, app.sidebar_section_split, app.sidebar_chrome);
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

    fn test_ghost(
        id: &str,
        label: &str,
        closed_at: u64,
    ) -> crate::app::closed_agents::ClosedAgentRecord {
        crate::app::closed_agents::ClosedAgentRecord {
            agent_id: id.into(),
            label: label.into(),
            cwd: Some(std::path::PathBuf::from("/tmp")),
            workspace_key: None,
            session: None,
            closed_at,
            revival: crate::app::closed_agents::RevivalState::Dormant,
        }
    }

    /// An app whose agents panel is filled several times over by living rows,
    /// with `ghosts` closed agents remembered behind them.
    fn app_with_a_full_panel_and_ghosts(living: usize, ghosts: usize) -> AppState {
        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = (0..living)
            .map(|idx| Workspace::test_new(&format!("ws{idx}")))
            .collect();
        app.active = Some(0);
        app.selected = 0;
        app.mobile_width_threshold = 0;
        app.ensure_test_terminals();
        // The panel lists a pane only when its terminal has a detected agent
        // (`pane_details` drops the rest), so the fixture has to give each one
        // an agent or the panel is empty and the test measures nothing.
        for ws in &app.workspaces {
            for tab in &ws.tabs {
                for pane in tab.panes.values() {
                    if let Some(terminal) = app.terminals.get_mut(&pane.attached_terminal_id) {
                        terminal.detected_agent = Some(Agent::Pi);
                    }
                }
            }
        }
        app.sidebar_agents.rows = vec![vec![crate::config::AgentSidebarToken::Agent]];
        for idx in 0..ghosts {
            app.closed_agents.record_closed(test_ghost(
                &format!("ghost-{idx}"),
                "Claude",
                idx as u64,
            ));
        }
        app
    }

    // T2.1 / TP-AGPANEL-43: the reported defect, stated as a property. Sixty-two
    // headstones were on disk and at most a handful could ever be seen, because
    // the graveyard was painted into whatever space the living left over — the
    // rest were not merely below the fold, they were unreachable at every
    // scroll position there was. "I want to see them one by one, in a
    // scrollable area" is not satisfiable while that is true, whatever the rows
    // look like.
    #[test]
    fn every_headstone_is_reachable_at_some_scroll_position() {
        let ghosts = 20;
        let mut app = app_with_a_full_panel_and_ghosts(24, ghosts);
        let area = Rect::new(0, 0, 30, 12);

        assert!(
            agent_panel_bottom_start(&app, area) > 0,
            "precondition: the panel scrolls at all"
        );

        let ceiling = agent_panel_bottom_start(&app, area);
        let mut reached = std::collections::HashSet::new();
        for scroll in 0..=ceiling {
            app.agent_panel_scroll = scroll;
            for (row, _, _) in agent_panel_placements(&app, area) {
                if let AgentPanelRow::Ghost(idx) = row {
                    reached.insert(idx);
                }
            }
        }

        assert_eq!(
            reached.len(),
            ghosts,
            "every headstone must be reachable; reached {} of {ghosts}",
            reached.len()
        );
    }

    // T2.2 / TP-AGPANEL-42: a headstone is laid out by the living rows' own
    // layout, so a two-row configuration gives it two rows. Drawn by a second
    // path it would be free to drift — and the path it replaced had drifted all
    // the way to one bare line of text.
    #[test]
    fn a_headstone_is_as_tall_as_the_row_layout_says() {
        let mut app = app_with_a_full_panel_and_ghosts(1, 1);
        app.sidebar_agents.rows = vec![
            vec![crate::config::AgentSidebarToken::Workspace],
            vec![crate::config::AgentSidebarToken::StateText],
        ];
        let area = Rect::new(0, 0, 30, 12);

        let placements = agent_panel_placements(&app, area);
        let ghost_height = placements
            .iter()
            .find_map(|(row, _, height)| matches!(row, AgentPanelRow::Ghost(_)).then_some(*height))
            .expect("the graveyard is drawn");
        let live_height = placements
            .iter()
            .find_map(|(row, _, height)| matches!(row, AgentPanelRow::Live(_)).then_some(*height))
            .expect("a living row is drawn");

        assert_eq!(
            ghost_height, live_height,
            "the only difference between the two must be colour"
        );
        assert_eq!(ghost_height, 2, "a two-row layout gives two rows");
    }

    // T2.4 / TP-AGPANEL-43: a headstone is a card, so a press on its lower half
    // counts. Matching only its first row would leave the rest of the card
    // silently dead — the class of defect one shared layout exists to prevent.
    #[test]
    fn a_press_on_the_lower_half_of_a_headstone_still_hits_it() {
        let mut app = app_with_a_full_panel_and_ghosts(1, 1);
        app.sidebar_agents.rows = vec![
            vec![crate::config::AgentSidebarToken::Workspace],
            vec![crate::config::AgentSidebarToken::StateText],
        ];
        let area = Rect::new(0, 0, 30, 12);
        let (_, y, height) = agent_panel_placements(&app, area)
            .into_iter()
            .find(|(row, _, _)| matches!(row, AgentPanelRow::Ghost(_)))
            .expect("the graveyard is drawn");
        assert_eq!(height, 2, "precondition: the headstone is two rows tall");

        assert_eq!(closed_agent_index_at(&app, area, y), Some(0));
        assert_eq!(
            closed_agent_index_at(&app, area, y + 1),
            Some(0),
            "the second row of the card belongs to the same headstone"
        );
    }

    // T2.3 / TP-AGPANEL-22: the third visual class holds. A headstone dims and
    // leans and never wears the accent — the accent means "this is where you
    // are", and nowhere is where a closed agent is.
    #[test]
    fn a_headstone_never_wears_the_active_accent() {
        let app = app_with_a_full_panel_and_ghosts(1, 1);
        let area = Rect::new(0, 0, 30, 12);
        let (_, ghost_y, _) = agent_panel_placements(&app, area)
            .into_iter()
            .find(|(row, _, _)| matches!(row, AgentPanelRow::Ghost(_)))
            .expect("the graveyard is drawn");

        let mut terminal = Terminal::new(TestBackend::new(30, 12)).unwrap();
        terminal
            .draw(|frame| render_agent_detail(&app, &TerminalRuntimeRegistry::new(), frame, area))
            .unwrap();
        let buffer = terminal.backend().buffer();
        let body = agent_panel_body_rect(area, false);

        for x in body.x..body.x + body.width {
            assert_ne!(
                buffer[(x, ghost_y)].style().bg,
                Some(app.palette.accent),
                "a headstone must never read as the row you are on"
            );
        }
    }

    // TP-AGPANEL-29 (R1): a panel the living rows fill can still be scrolled
    // down to its graveyard.
    //
    // The scroll ceiling used to be computed from the living rows alone, and
    // the graveyard was drawn only into whatever space they left over. On a
    // machine with fifty-nine panes those two rules meet badly: the panel is
    // always full, the leftover space never appears, and no scroll position
    // reaches the ghosts. A section that cannot be reached is a section that
    // is not there — which is exactly how it was reported.
    #[test]
    fn a_full_panel_can_still_be_scrolled_down_to_its_graveyard() {
        let mut app = app_with_a_full_panel_and_ghosts(24, 3);
        let area = Rect::new(0, 0, 30, 24);

        // Precondition: the living rows really do overflow the panel.
        assert!(
            agent_panel_bottom_start(&app, area) > 0,
            "precondition: the panel scrolls at all; entries={} body={:?} ceiling={}",
            agent_panel_entries(&app).len(),
            agent_panel_body_rect(area, false),
            agent_panel_bottom_start(&app, area)
        );

        app.agent_panel_scroll = agent_panel_bottom_start(&app, area);
        let slots = closed_agent_row_slots(&app, area);
        assert!(
            slots.is_some(),
            "scrolled to the bottom, the graveyard must have somewhere to paint"
        );
        let (_separator_y, rows) = slots.expect("slots");
        assert!(
            !rows.is_empty(),
            "a separator with no ghost row under it is the rendering bug the contract forbids"
        );
    }

    // TP-AGPANEL-29 (R2): with no ghosts the ceiling is untouched. Growing it
    // for a graveyard that does not exist opens empty space under the panel —
    // an "there is more below" that is a lie.
    #[test]
    fn with_no_ghosts_the_panel_ceiling_is_unchanged() {
        let area = Rect::new(0, 0, 30, 24);
        let with_ghosts = app_with_a_full_panel_and_ghosts(24, 0);
        let ceiling = agent_panel_bottom_start(&with_ghosts, area);

        let mut bare = app_with_a_full_panel_and_ghosts(24, 0);
        bare.closed_agents = Default::default();
        assert_eq!(
            agent_panel_bottom_start(&bare, area),
            ceiling,
            "an empty graveyard changes nothing about how far the panel scrolls"
        );
    }

    // TP-AGPANEL-29 (R4): a panel the living rows do not fill behaves exactly
    // as before — the graveyard was already visible there.
    #[test]
    fn a_short_panel_still_shows_its_graveyard_without_scrolling() {
        let app = app_with_a_full_panel_and_ghosts(1, 2);
        let area = Rect::new(0, 0, 30, 24);
        assert!(
            closed_agent_row_slots(&app, area).is_some(),
            "with room to spare the graveyard paints without any scrolling"
        );
    }

    // TP-AGPANEL-22: after the living rows, a separator and the recently
    // closed agents — newest first, plain text. The third visual class holds
    // on one axis: the active card wears the accent, passive rows drop bold,
    // and a ghost dims AND leans (italic) — dim alone collides with the
    // passive agent token, which already dims. A reviving ghost carries a
    // static ellipsis; nothing animates for a dead row.
    #[test]
    fn closed_agents_render_dimmed_under_a_separator() {
        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![Workspace::test_new("one")];
        app.ensure_test_terminals();
        let pane_id = app.workspaces[0].tabs[0].root_pane;
        let terminal_id = app.workspaces[0].tabs[0].panes[&pane_id]
            .attached_terminal_id
            .clone();
        app.terminals.get_mut(&terminal_id).unwrap().detected_agent = Some(Agent::Pi);
        app.sidebar_agents.rows = vec![vec![crate::config::AgentSidebarToken::Agent]];

        app.closed_agents
            .record_closed(test_ghost("old-friend", "Claude", 1));
        app.closed_agents
            .record_closed(test_ghost("second", "Codex", 2));
        assert!(app.closed_agents.try_begin_revival("second"));

        let area = Rect::new(0, 0, 20, 8);
        let body = agent_panel_body_rect(area, false);
        let mut terminal = Terminal::new(TestBackend::new(20, 8)).unwrap();
        terminal
            .draw(|frame| render_agent_detail(&app, &TerminalRuntimeRegistry::new(), frame, area))
            .unwrap();
        let buffer = terminal.backend().buffer();

        assert_eq!(row_text(buffer, body.y, body.width), " pi");
        assert!(
            row_text(buffer, body.y + 1, body.width).starts_with('─'),
            "a separator divides the living from the graveyard: {:?}",
            row_text(buffer, body.y + 1, body.width)
        );
        assert_eq!(row_text(buffer, body.y + 2, body.width), " Codex …");
        assert_eq!(row_text(buffer, body.y + 3, body.width), " Claude");

        // The class is measurable in the buffer, not just claimed: ghost
        // cells lean, live cells never do.
        let ghost_cell = &buffer[(body.x + 1, body.y + 3)];
        let live_cell = &buffer[(body.x + 1, body.y)];
        assert!(ghost_cell
            .style()
            .add_modifier
            .contains(ratatui::style::Modifier::ITALIC));
        assert!(ghost_cell
            .style()
            .add_modifier
            .contains(ratatui::style::Modifier::DIM));
        assert!(!live_cell
            .style()
            .add_modifier
            .contains(ratatui::style::Modifier::ITALIC));
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
        let (_, agent_area) =
            expanded_sidebar_sections(area, app.sidebar_section_split, app.sidebar_chrome);
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
        let (_, agent_area) =
            expanded_sidebar_sections(area, app.sidebar_section_split, app.sidebar_chrome);
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
        let workspace_area =
            workspace_list_rect(area, app.sidebar_section_split, app.sidebar_chrome);
        let body = workspace_list_body_rect(workspace_area, false, app.sidebar_chrome);

        let metrics = workspace_list_scroll_metrics(&app, workspace_area);
        let (cards, _, _headers, _, _, _, _, _) = compute_workspace_list_areas(&app, area);

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

        let toggle = expanded_sidebar_toggle_rect(area, app.sidebar_chrome);
        assert_eq!(
            terminal.backend().buffer()[(toggle.x, toggle.y)].symbol(),
            "«"
        );
    }

    #[test]
    fn expanded_sidebar_toggle_sits_inside_sidebar_content() {
        let area = Rect::new(0, 0, 26, 20);
        let bare = crate::ui::shell::SidebarChrome {
            spaces: None,
            agents: None,
            chips: None,
        };
        let toggle = expanded_sidebar_toggle_rect(area, bare);

        // T56 · the unframed path is the one every user sees today, and it does
        // not move.
        assert_eq!(toggle.x, area.x + area.width - 2);
        assert_eq!(toggle.y, area.y + area.height - 1);

        let framed = crate::ui::shell::SidebarChrome {
            spaces: None,
            agents: Some(crate::ui::shell::BarTint::solid(
                ratatui::style::Color::Rgb(1, 2, 3),
            )),
            chips: None,
        };
        let inside = expanded_sidebar_toggle_rect(area, framed);
        assert_eq!(inside.x, toggle.x - 1, "the frame owns the last column");
        assert_eq!(inside.y, toggle.y - 1, "the frame owns the last row");
    }

    fn app_with_footer_chips(area: Rect, chips: bool) -> crate::app::state::AppState {
        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![Workspace::test_new("one")];
        app.ensure_test_terminals();
        app.active = Some(0);
        app.mouse_capture = true;
        app.view.sidebar_rect = area;
        app.sidebar_chrome = crate::ui::shell::SidebarChrome {
            spaces: None,
            agents: None,
            chips: chips.then(|| {
                crate::ui::shell::BarTint::solid(ratatui::style::Color::Rgb(250, 179, 135))
            }),
        };
        app
    }

    // T60 · with chips asked for, the footer's controls are drawn as framed
    // buttons — the frame is the whole point of the request, so its corners are
    // what the test looks for, not the label.
    #[test]
    fn footer_controls_wear_their_own_frames_when_chips_are_on() {
        let area = Rect::new(0, 0, 30, 24);
        let app = app_with_footer_chips(area, true);

        let mut terminal = Terminal::new(TestBackend::new(area.width, area.height)).unwrap();
        terminal
            .draw(|frame| render_sidebar(&app, &TerminalRuntimeRegistry::new(), frame, area))
            .unwrap();
        let buffer = terminal.backend().buffer();

        for (name, rect) in [
            ("new", app.sidebar_new_button_rect()),
            ("menu", app.global_launcher_rect()),
        ] {
            assert_eq!(
                buffer[(rect.x, rect.y)].symbol(),
                "╭",
                "{name} has no top-left corner at {rect:?}"
            );
            assert_eq!(
                buffer[(rect.x + rect.width - 1, rect.y + rect.height - 1)].symbol(),
                "╯",
                "{name} has no bottom-right corner at {rect:?}"
            );
            // The corners alone would still be satisfied by a rectangle too
            // narrow to say anything -- a chip sized independently of its label
            // clips to a frame around nothing.
            let label_row: String = (rect.x..rect.x + rect.width)
                .map(|x| buffer[(x, rect.y + 1)].symbol())
                .collect();
            assert!(
                label_row.contains(name),
                "{name} lost its label inside its own frame: {label_row:?}"
            );
        }
    }

    // TP-FOCUS-SW-04 (render): the Spaces footer draws its focus toggle,
    // accented while the tree is narrowed and dim while it is whole — the
    // switch has to say which way it is thrown. Like every other control in
    // this footer it is mouse chrome: no mouse, no button.
    #[test]
    fn the_spaces_footer_draws_its_focus_toggle_and_says_which_way_it_is_thrown() {
        let area = Rect::new(0, 0, 30, 24);
        let mut app = app_with_footer_chips(area, false);
        app.sidebar_tab = crate::app::state::SidebarTab::Spaces;

        let draw = |app: &crate::app::state::AppState| {
            let mut terminal = Terminal::new(TestBackend::new(area.width, area.height)).unwrap();
            terminal
                .draw(|frame| render_sidebar(app, &TerminalRuntimeRegistry::new(), frame, area))
                .unwrap();
            let rect = app.sidebar_focus_toggle_rect();
            assert!(rect.width > 0, "the footer keeps room for the toggle");
            let buffer = terminal.backend().buffer();
            let label: String = (rect.x..rect.x + rect.width)
                .map(|x| buffer[(x, rect.y)].symbol())
                .collect();
            (label, buffer[(rect.x, rect.y)].style().fg)
        };

        let (off_label, off_fg) = draw(&app);
        assert!(off_label.contains("focus"), "got {off_label:?}");
        assert_eq!(
            off_fg,
            Some(app.palette.overlay0),
            "an unfocused tree keeps its switch dim"
        );

        app.spaces_focus_only = true;
        let (on_label, on_fg) = draw(&app);
        assert!(on_label.contains("focus"), "got {on_label:?}");
        assert_eq!(
            on_fg,
            Some(app.palette.accent),
            "a narrowed tree wears its switch in the accent"
        );

        // Without the mouse the footer chrome is gone, like every control
        // beside it (TP-REPAINT-2B).
        app.mouse_capture = false;
        let mut terminal = Terminal::new(TestBackend::new(area.width, area.height)).unwrap();
        terminal
            .draw(|frame| render_sidebar(&app, &TerminalRuntimeRegistry::new(), frame, area))
            .unwrap();
        let rect = app.sidebar_focus_toggle_rect();
        let label: String = (rect.x..rect.x + rect.width)
            .map(|x| terminal.backend().buffer()[(x, rect.y)].symbol())
            .collect();
        assert!(
            !label.contains("focus"),
            "without the mouse the toggle is not drawn: {label:?}"
        );
    }

    // T62 · a control that cannot fit a frame keeps its label. Half a border is
    // worse than no border, and a vanished button is worse than both.
    #[test]
    fn a_footer_too_narrow_for_chips_still_draws_its_labels() {
        let area = Rect::new(0, 0, 12, 24);
        let app = app_with_footer_chips(area, true);

        let mut terminal = Terminal::new(TestBackend::new(area.width, area.height)).unwrap();
        terminal
            .draw(|frame| render_sidebar(&app, &TerminalRuntimeRegistry::new(), frame, area))
            .unwrap();
        let buffer = terminal.backend().buffer();

        let footer = app.sidebar_new_button_rect();
        let rows: Vec<String> = (footer.y..footer.y + footer.height)
            .map(|y| (0..area.width).map(|x| buffer[(x, y)].symbol()).collect())
            .collect();
        let footer_text = rows.join(" / ");
        assert!(
            footer_text.contains("new") || footer_text.contains("menu"),
            "the footer lost its controls entirely: {footer_text:?}"
        );
        assert!(
            !footer_text.contains('╭'),
            "two frames cannot share this width, so neither should have been drawn: {footer_text:?}"
        );
    }

    // T63 · the agents half's own two labels become buttons in its top corners,
    // and they cost the list nothing: the header already reserved the rows a
    // frame needs, so the frames simply take the ones the separator was using.
    #[test]
    fn the_agents_header_wears_its_labels_as_corner_chips() {
        let area = Rect::new(0, 0, 34, 26);
        let app = app_with_footer_chips(area, true);

        let mut terminal = Terminal::new(TestBackend::new(area.width, area.height)).unwrap();
        terminal
            .draw(|frame| render_sidebar(&app, &TerminalRuntimeRegistry::new(), frame, area))
            .unwrap();
        let buffer = terminal.backend().buffer();

        let (_, detail) =
            expanded_sidebar_sections(area, app.sidebar_section_split, app.sidebar_chrome);
        let name = agent_panel_header_name_rect(detail, "agents", app.sidebar_chrome);
        let sort = agent_panel_toggle_rect(detail, app.agent_panel_sort, app.sidebar_chrome);

        assert_eq!(name.x, detail.x, "the name sits in the left corner");
        assert_eq!(
            sort.x + sort.width,
            detail.x + detail.width,
            "the sort control sits in the right corner"
        );
        assert_eq!(name.y, detail.y, "and both sit at the top");
        assert_eq!(sort.y, detail.y);
        assert!(
            name.x + name.width <= sort.x,
            "the two chips do not overlap: {name:?} {sort:?}"
        );

        for (label, rect) in [("agents", name), ("grouped", sort)] {
            assert_eq!(
                buffer[(rect.x, rect.y)].symbol(),
                "╭",
                "{label} has no frame at {rect:?}"
            );
            let row: String = (rect.x..rect.x + rect.width)
                .map(|x| buffer[(x, rect.y + 1)].symbol())
                .collect();
            assert!(
                row.contains(label),
                "{label} lost its text inside its frame: {row:?}"
            );
        }

        // AGENT_PANEL_HEADER_ROWS already covered these rows, so the list below
        // must start exactly where it always did.
        let bare = app_with_footer_chips(area, false);
        let (_, bare_detail) =
            expanded_sidebar_sections(area, bare.sidebar_section_split, bare.sidebar_chrome);
        assert_eq!(
            agent_panel_body_rect(detail, false).height,
            agent_panel_body_rect(bare_detail, false).height,
            "chips in the header cost the agent list nothing"
        );
    }

    // T57 · a framed panel owns its own corner. The collapse icon is a control
    // inside the panel, not a glyph that overwrites the frame it sits in — an
    // icon on the corner reads as a broken border, not as a button.
    #[test]
    fn the_collapse_icon_keeps_off_a_framed_panels_corner() {
        let area = Rect::new(0, 0, 30, 24);
        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![Workspace::test_new("one")];
        app.ensure_test_terminals();
        app.active = Some(0);
        app.sidebar_chrome = crate::ui::shell::SidebarChrome {
            spaces: None,
            agents: Some(crate::ui::shell::BarTint::solid(
                ratatui::style::Color::Rgb(250, 179, 135),
            )),
            chips: None,
        };

        let mut terminal = Terminal::new(TestBackend::new(area.width, area.height)).unwrap();
        terminal
            .draw(|frame| render_sidebar(&app, &TerminalRuntimeRegistry::new(), frame, area))
            .unwrap();
        let buffer = terminal.backend().buffer();

        let (_, detail_frame) = expanded_sidebar_section_frames(area, app.sidebar_section_split);
        let corner_x = detail_frame.x + detail_frame.width - 1;
        let corner_y = detail_frame.y + detail_frame.height - 1;
        assert_eq!(
            buffer[(corner_x, corner_y)].symbol(),
            "╯",
            "the frame's own corner survived the collapse icon"
        );
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
            "/home/user/projects/herdr",
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
            "/home/user/projects/herdr",
            vec![test_chat("a", "hidden chat", 4)],
        )];
        app.collapsed_project_paths
            .insert(std::path::PathBuf::from("/home/user/projects/herdr"));
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
        app.projects_sessions = vec![project_sessions("/home/user/projects/empty", Vec::new())];
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
            project_display_name(std::path::Path::new("/home/user/projects/herdr")),
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

    // TP-MOD-09: a module's row carries its name. The row is emitted by the
    // tree walk and painted by the header renderer, and those are two
    // different questions: a row that is emitted but never painted still
    // takes its line, so the tree grows a blank gap and the module the user
    // named is nowhere on screen.
    #[test]
    fn a_module_row_paints_the_name_the_user_gave_it() {
        let mut app = crate::app::state::AppState::test_new();
        app.sidebar_tab = crate::app::state::SidebarTab::Spaces;
        app.mouse_capture = false;
        app.mobile_width_threshold = 0;
        app.workspaces = vec![
            worktree_on_branch("alpha", "feat/tui-alpha"),
            worktree_on_branch("beta", "feat/tui-beta"),
        ];
        app.space_split_rules = vec![split_rule(&["feat/tui-*"], "herdr:tui", "TUI")];
        // Parented under a drawn project, so the row is definitely emitted
        // (`an_empty_module_under_a_drawn_project_takes_a_row_of_its_own`).
        // This test is only about whether the emitted row reaches the screen.
        app.space_projects = vec![project_over("project:herdr", &["/repo/herdr"], &[])];
        app.space_nodes = vec![crate::spaces::SpaceNode {
            key: "group:remote-audio".into(),
            name: "UZAKTANSES".into(),
            icon: None,
            parent: Some("project:herdr".into()),
            dir: None,
        }];
        app.ensure_test_terminals();
        app.active = Some(0);
        app.selected = 0;

        let area = Rect::new(0, 0, 28, 16);
        app.view.sidebar_tab_hit_areas = compute_sidebar_tab_areas(area);
        let (cards, chats, groups, projects, more, empty_modules, _, _) =
            compute_workspace_list_areas(&app, area);
        app.view.workspace_card_areas = cards;
        app.view.workspace_chat_row_areas = chats;
        app.view.workspace_group_header_areas = groups;
        app.view.workspace_project_header_areas = projects;
        app.view.workspace_more_chats_areas = more;
        app.view.workspace_empty_module_areas = empty_modules;

        let runtimes = TerminalRuntimeRegistry::new();
        let mut terminal = Terminal::new(TestBackend::new(28, 16)).unwrap();
        terminal
            .draw(|frame| render_workspace_list(&app, &runtimes, frame, area, true))
            .unwrap();

        let rows: Vec<String> = (0..16)
            .map(|y| {
                (0..28)
                    .map(|x| terminal.backend().buffer()[(x, y)].symbol())
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            })
            .filter(|row| !row.is_empty())
            .collect();
        assert!(
            rows.iter().any(|row| row.contains("UZAKTANSES")),
            "the module's name must reach the screen:\n{}",
            rows.join("\n")
        );
    }

    /// A tree with one populated bucket and one declared, genuinely empty
    /// module hanging under a drawn project.
    fn app_with_an_empty_module() -> AppState {
        let mut app = AppState::test_new();
        app.sidebar_tab = crate::app::state::SidebarTab::Spaces;
        app.mouse_capture = false;
        app.mobile_width_threshold = 0;
        app.workspaces = vec![
            worktree_on_branch("alpha", "feat/tui-alpha"),
            worktree_on_branch("beta", "feat/tui-beta"),
        ];
        app.space_split_rules = vec![split_rule(&["feat/tui-*"], "herdr:tui", "TUI")];
        app.space_projects = vec![project_over("project:herdr", &["/repo/herdr"], &[])];
        app.space_nodes = vec![crate::spaces::SpaceNode {
            key: "group:remote-audio".into(),
            name: "UZAKTANSES".into(),
            icon: None,
            parent: Some("project:herdr".into()),
            dir: None,
        }];
        app.ensure_test_terminals();
        app.active = Some(0);
        app.selected = 0;
        app
    }

    /// Every screen row, index kept equal to `y` — `spaces_rows` drops blanks,
    /// which is exactly what a test about *where* a row landed cannot afford.
    fn spaces_screen(app: &mut AppState, width: u16, height: u16) -> Vec<String> {
        let area = Rect::new(0, 0, width, height);
        app.view.sidebar_tab_hit_areas = compute_sidebar_tab_areas(area);
        let (cards, chats, groups, projects, more, empty_modules, _, _) =
            compute_workspace_list_areas(app, area);
        app.view.workspace_card_areas = cards;
        app.view.workspace_chat_row_areas = chats;
        app.view.workspace_group_header_areas = groups;
        app.view.workspace_project_header_areas = projects;
        app.view.workspace_more_chats_areas = more;
        app.view.workspace_empty_module_areas = empty_modules;

        let runtimes = TerminalRuntimeRegistry::new();
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|frame| render_workspace_list(app, &runtimes, frame, area, true))
            .unwrap();
        (0..height)
            .map(|y| {
                (0..width)
                    .map(|x| terminal.backend().buffer()[(x, y)].symbol())
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            })
            .collect()
    }

    fn note_rows(screen: &[String]) -> Vec<usize> {
        screen
            .iter()
            .enumerate()
            .filter(|(_, row)| row.contains(EMPTY_MODULE_NOTE))
            .map(|(y, _)| y)
            .collect()
    }

    // TP-MOD-03: a container that draws nothing beneath it says so on a row of
    // its own. Without the note the module is a name over a gap, and a gap is
    // what damage looks like — the reader cannot tell "empty" from "broken"
    // and goes hand-editing a tree that was working.
    #[test]
    fn an_empty_module_says_it_is_empty() {
        let mut app = app_with_an_empty_module();
        let screen = spaces_screen(&mut app, 40, 20);
        assert_eq!(
            note_rows(&screen).len(),
            1,
            "the empty module writes exactly one note:\n{}",
            screen.join("\n")
        );
    }

    // TP-MOD-21: a bucket that claims nothing draws no header (TP-MOD-15), so
    // a module whose only child is such a bucket still shows an empty screen.
    // Asking "does it have children" instead of "does anything get drawn"
    // makes this case answer "not empty" while the reader sees a gap — the
    // emit-versus-paint split this fork has already paid for twice.
    #[test]
    fn a_module_over_a_bucket_that_claims_nothing_is_still_empty() {
        let mut app = app_with_an_empty_module();
        app.space_split_rules
            .push(split_rule(&["hicbir-dal-eslesmez/*"], "herdr:bos", "Bos"));
        app.space_nodes[0].parent = Some("project:herdr".into());
        // The empty bucket hangs under the module, so the module has a child
        // in the map and no row on the screen.
        app.space_split_rules[1].parent = Some("group:remote-audio".to_string());

        let screen = spaces_screen(&mut app, 40, 20);
        assert_eq!(
            note_rows(&screen).len(),
            1,
            "a module over a bucket nobody joined is still empty:\n{}",
            screen.join("\n")
        );
    }

    // TP-MOD-22: a module that carries another module is not empty — the child
    // always draws its own header. Only the child, which has nothing under it,
    // gets the note. A note on the parent too would call a populated container
    // empty and teach the reader to distrust the note.
    #[test]
    fn a_module_that_carries_a_module_is_not_empty_but_its_child_is() {
        let mut app = app_with_an_empty_module();
        app.space_nodes.push(crate::spaces::SpaceNode {
            key: "group:remote-audio-sub".into(),
            name: "ALTMODUL".into(),
            icon: None,
            parent: Some("group:remote-audio".into()),
            dir: None,
        });

        let screen = spaces_screen(&mut app, 40, 20);
        let notes = note_rows(&screen);
        assert_eq!(
            notes.len(),
            1,
            "only the childless module writes a note:\n{}",
            screen.join("\n")
        );
        let child_row = screen
            .iter()
            .position(|row| row.contains("ALTMODUL"))
            .expect("the sub-module is drawn");
        assert!(
            notes[0] > child_row,
            "the note belongs to the child, so it sits below it:\n{}",
            screen.join("\n")
        );
    }

    // TP-MOD-23: a module over a bucket with members is not empty. This is the
    // ordinary populated case, and a false note here would hang a line of
    // noise under every working module in the tree.
    #[test]
    fn a_module_over_a_populated_bucket_writes_no_note() {
        let mut app = app_with_an_empty_module();
        app.space_split_rules[0].parent = Some("group:remote-audio".to_string());

        let screen = spaces_screen(&mut app, 40, 20);
        assert!(
            note_rows(&screen).is_empty(),
            "a module holding two checkouts is not empty:\n{}",
            screen.join("\n")
        );
    }

    // TP-MOD-24: a folded module keeps its subtree hidden, and a note about
    // what is inside a closed container contradicts closing it. It would also
    // make folding change the row count in the wrong direction.
    #[test]
    fn a_folded_empty_module_writes_no_note() {
        let mut app = app_with_an_empty_module();
        app.fold_node("group:remote-audio".to_string());

        let screen = spaces_screen(&mut app, 40, 20);
        assert!(
            note_rows(&screen).is_empty(),
            "a closed module says nothing about its inside:\n{}",
            screen.join("\n")
        );
    }

    // TP-MOD-25: the note is painted and inert. Painted, because a row that is
    // emitted and never drawn takes its line and leaves the gap it was meant
    // to explain. Inert, because there is nothing behind it to open — and a
    // row that answers a click by doing nothing is worse than one that never
    // invited it.
    #[test]
    fn the_empty_module_note_is_painted_and_carries_no_hit_area() {
        let mut app = app_with_an_empty_module();
        let screen = spaces_screen(&mut app, 40, 20);
        let note_y = u16::try_from(
            *note_rows(&screen)
                .first()
                .unwrap_or_else(|| panic!("the note is painted:\n{}", screen.join("\n"))),
        )
        .expect("the test screen is small");

        let claimed = app
            .view
            .workspace_card_areas
            .iter()
            .map(|area| area.rect)
            .chain(
                app.view
                    .workspace_chat_row_areas
                    .iter()
                    .map(|area| area.rect),
            )
            .chain(
                app.view
                    .workspace_group_header_areas
                    .iter()
                    .map(|area| area.rect),
            )
            .chain(
                app.view
                    .workspace_project_header_areas
                    .iter()
                    .map(|area| area.rect),
            )
            .chain(
                app.view
                    .workspace_more_chats_areas
                    .iter()
                    .map(|area| area.rect),
            )
            .any(|rect| note_y >= rect.y && note_y < rect.y.saturating_add(rect.height));
        assert!(
            !claimed,
            "no clickable area may cover the note row {note_y}:\n{}",
            screen.join("\n")
        );
    }

    /// Render the Spaces list and return its non-empty rows, trailing space
    /// trimmed but leading indent kept — the indent is the thing under test.
    fn spaces_rows(app: &mut AppState, width: u16, height: u16) -> Vec<String> {
        let area = Rect::new(0, 0, width, height);
        app.view.sidebar_tab_hit_areas = compute_sidebar_tab_areas(area);
        let (cards, chats, groups, projects, more, empty_modules, _, _) =
            compute_workspace_list_areas(app, area);
        app.view.workspace_card_areas = cards;
        app.view.workspace_chat_row_areas = chats;
        app.view.workspace_group_header_areas = groups;
        app.view.workspace_project_header_areas = projects;
        app.view.workspace_more_chats_areas = more;
        app.view.workspace_empty_module_areas = empty_modules;

        let runtimes = TerminalRuntimeRegistry::new();
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|frame| render_workspace_list(app, &runtimes, frame, area, true))
            .unwrap();
        (0..height)
            .map(|y| {
                (0..width)
                    .map(|x| terminal.backend().buffer()[(x, y)].symbol())
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            })
            .filter(|row| !row.is_empty())
            .collect()
    }

    /// Leading spaces of the row that carries `needle`.
    fn indent_of(rows: &[String], needle: &str) -> usize {
        let row = rows
            .iter()
            .find(|row| row.contains(needle))
            .unwrap_or_else(|| panic!("no row carries {needle:?}: {rows:#?}"));
        row.len() - row.trim_start().len()
    }

    // TP-MOD-10: the row says where the module was created. A sub-module
    // steps in one place under its parent; a parallel module sits at the
    // parent's own level. Without the step the two creation roads produce
    // rows a reader cannot tell apart, and "make this a sub-module" becomes
    // an action with no visible result.
    #[test]
    fn a_sub_module_steps_in_and_a_parallel_module_does_not() {
        let mut app = crate::app::state::AppState::test_new();
        app.sidebar_tab = crate::app::state::SidebarTab::Spaces;
        app.mouse_capture = false;
        app.mobile_width_threshold = 0;
        app.workspaces = vec![
            worktree_on_branch("alpha", "feat/tui-alpha"),
            worktree_on_branch("beta", "feat/tui-beta"),
        ];
        app.space_split_rules = vec![split_rule(&["feat/tui-*"], "herdr:tui", "TUI")];
        app.space_projects = vec![project_over("project:herdr", &["/repo/herdr"], &[])];
        app.space_nodes = vec![
            // Parallel to the project's other children: parent is the project.
            crate::spaces::SpaceNode {
                key: "group:paralel".into(),
                name: "PARALEL".into(),
                icon: None,
                parent: Some("project:herdr".into()),
                dir: None,
            },
            // A sub-module of the one above.
            crate::spaces::SpaceNode {
                key: "group:altmodul".into(),
                name: "ALTMODUL".into(),
                icon: None,
                parent: Some("group:paralel".into()),
                dir: None,
            },
        ];
        app.ensure_test_terminals();
        app.active = Some(0);
        app.selected = 0;

        let rows = spaces_rows(&mut app, 30, 18);
        let project = indent_of(&rows, "project:herdr");
        let parallel = indent_of(&rows, "PARALEL");
        let sub = indent_of(&rows, "ALTMODUL");
        assert_eq!(
            parallel,
            project + ROW_INDENT_STEP as usize,
            "a module under the project steps in exactly once: {rows:#?}"
        );
        assert_eq!(
            sub,
            parallel + ROW_INDENT_STEP as usize,
            "a sub-module steps in once more than its parent: {rows:#?}"
        );
    }

    // TP-MOD-11: a module carries the same disclosure arrow every header
    // wears, and carries no aggregate state dot. A project folded over its
    // checkouts can answer for them; a module holds no membership of its
    // own, so a dot there would be a number invented out of nothing.
    #[test]
    fn a_folded_module_shows_the_arrow_and_no_invented_state_dot() {
        let mut app = crate::app::state::AppState::test_new();
        app.sidebar_tab = crate::app::state::SidebarTab::Spaces;
        app.mouse_capture = false;
        app.mobile_width_threshold = 0;
        app.workspaces = vec![
            worktree_on_branch("alpha", "feat/tui-alpha"),
            worktree_on_branch("beta", "feat/tui-beta"),
        ];
        app.space_split_rules = vec![split_rule(&["feat/tui-*"], "herdr:tui", "TUI")];
        app.space_projects = vec![project_over("project:herdr", &["/repo/herdr"], &[])];
        app.space_nodes = vec![crate::spaces::SpaceNode {
            key: "group:kapali".into(),
            name: "KAPALI".into(),
            icon: None,
            parent: Some("project:herdr".into()),
            dir: None,
        }];
        app.fold_node("group:kapali".to_string());
        app.ensure_test_terminals();
        app.active = Some(0);
        app.selected = 0;

        let rows = spaces_rows(&mut app, 30, 18);
        let row = rows
            .iter()
            .find(|row| row.contains("KAPALI"))
            .unwrap_or_else(|| panic!("the folded module is still drawn: {rows:#?}"));
        assert!(
            row.contains(DISCLOSURE_CLOSED),
            "a folded module wears the closed arrow: {row:?}"
        );
        assert!(
            !row.contains('●') && !row.contains('○'),
            "a module has no membership to summarise, so it prints no dot: {row:?}"
        );
    }

    // TP-MOD-12: opening the header renderer to a second identity must not
    // change what a project header has always drawn — arrow, icon, name, and
    // the aggregate dot it earns while folded.
    #[test]
    fn a_project_header_still_draws_its_arrow_icon_and_name() {
        let mut app = crate::app::state::AppState::test_new();
        app.sidebar_tab = crate::app::state::SidebarTab::Spaces;
        app.mouse_capture = false;
        app.mobile_width_threshold = 0;
        app.workspaces = vec![
            worktree_on_branch("alpha", "feat/tui-alpha"),
            worktree_on_branch("beta", "feat/tui-beta"),
        ];
        app.space_split_rules = vec![split_rule(&["feat/tui-*"], "herdr:tui", "TUI")];
        let mut project = project_over("project:herdr", &["/repo/herdr"], &[]);
        project.name = "HERDRPROJE".into();
        project.icon = Some("🚀".into());
        app.space_projects = vec![project];
        app.ensure_test_terminals();
        app.active = Some(0);
        app.selected = 0;

        let rows = spaces_rows(&mut app, 30, 18);
        let row = rows
            .iter()
            .find(|row| row.contains("HERDRPROJE"))
            .unwrap_or_else(|| panic!("the project header is drawn: {rows:#?}"));
        assert!(row.contains(DISCLOSURE_OPEN), "arrow: {row:?}");
        assert!(row.contains('🚀'), "the project's own icon wins: {row:?}");
        assert_eq!(
            indent_of(&rows, "HERDRPROJE"),
            0,
            "a top-level project sits at the left edge"
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

        // Was a bare deadline the loop simply gave up on: if the cwd never
        // arrived the test carried on and failed later, complaining about
        // whatever it checked next rather than about the wait that never
        // finished. Everything below needs this cwd, so saying so here is both
        // the honest message and the load-aware budget.
        let wait = LoadAwareDeadline::new(2, "the runtime to report its working directory");
        while runtime.cwd() != Some(live_cwd.clone()) {
            wait.check();
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
                WorkspaceListEntry::EmptyModule { .. } => "empty-module",
                WorkspaceListEntry::MoreChats { .. } => "more",
                WorkspaceListEntry::DailyHeader => "daily-header",
                WorkspaceListEntry::DailyChat { .. } => "daily-chat",
                WorkspaceListEntry::DailyMore { .. } => "daily-more",
                WorkspaceListEntry::DailyMoreWorkspaces { .. } => "daily-more-workspaces",
                WorkspaceListEntry::ModuleChat { .. } => "module-chat",
            })
            .collect()
    }

    /// A declared container holding `chat_count` moved chats, and one
    /// workspace living somewhere else — the shape the move exists for: the
    /// container has no directory and never needed one.
    fn app_with_a_module_holding_moved_chats(chat_count: usize) -> AppState {
        let mut app = crate::app::state::AppState::test_new();
        let mut ws = Workspace::test_new("elsewhere");
        ws.identity_cwd = std::env::temp_dir().join("herdr-module-elsewhere");
        app.workspaces = vec![ws];
        app.active = Some(0);
        app.selected = 0;
        app.mobile_width_threshold = 0;
        app.space_nodes = vec![crate::spaces::SpaceNode {
            key: "docs".to_string(),
            name: "Docs".to_string(),
            icon: None,
            parent: None,
            dir: None,
        }];
        let key = crate::persist::workspace_chats::module_ledger_key("docs");
        let rows = (0..chat_count)
            .map(|idx| crate::app::state::WorkspaceChatRow {
                session_id: format!("moved-session-{idx}"),
                agent: "claude".to_string(),
                title: Some(format!("moved chat {idx}")),
                last_seen_ms: 3_000 + idx as u64,
                last_modified: None,
            })
            .collect::<Vec<_>>();
        app.workspace_chat_rows.insert(key, rows);
        app
    }

    fn module_chat_positions(app: &AppState) -> Vec<usize> {
        workspace_list_entries(app)
            .iter()
            .enumerate()
            .filter(|(_, entry)| matches!(entry, WorkspaceListEntry::ModuleChat { .. }))
            .map(|(pos, _)| pos)
            .collect()
    }

    // TP-CHAT-MOVE-06 (M6): a chat moved into a container is drawn under that
    // container's header. Writing the move and never drawing it is the #91
    // shape of defect: the plumbing works, the product is dead.
    #[test]
    fn a_chat_moved_into_a_module_is_drawn_under_its_header() {
        let app = app_with_a_module_holding_moved_chats(2);
        let entries = workspace_list_entries(&app);

        let header = entries
            .iter()
            .position(|entry| {
                matches!(entry, WorkspaceListEntry::ProjectHeader { project_key } if project_key == "docs")
            })
            .expect("the declared container gets a header even with no checkout under it");
        let chats = module_chat_positions(&app);

        assert_eq!(
            chats.len(),
            2,
            "both moved chats are drawn; got {entries:?}"
        );
        assert!(
            chats.iter().all(|pos| *pos > header),
            "they belong under the header that owns them"
        );
    }

    // TP-CHAT-MOVE-06 (M7): a container holding chats is not empty. "Nothing
    // here" printed directly above a list of chats is the opposite of the
    // readability TP-MOD-03 exists for.
    #[test]
    fn a_module_holding_moved_chats_does_not_claim_to_be_empty() {
        let app = app_with_a_module_holding_moved_chats(1);
        assert!(
            !workspace_list_entries(&app)
                .iter()
                .any(|entry| matches!(entry, WorkspaceListEntry::EmptyModule { .. })),
            "a container with chats in it must not print the empty-module row"
        );

        // And the row is still there when the container really is empty.
        let mut bare = app_with_a_module_holding_moved_chats(0);
        bare.workspace_chat_rows.clear();
        assert!(
            workspace_list_entries(&bare)
                .iter()
                .any(|entry| matches!(entry, WorkspaceListEntry::EmptyModule { .. })),
            "an actually empty container still says so (TP-MOD-03 is not weakened)"
        );
    }

    // TP-CHAT-MOVE-06 (M8): the row carries a container key and no `ws_idx`.
    // Folded into the workspace-indexed vectors it would resolve as some other
    // checkout's chat on every press — the trap TP-DAILY-03 keeps daily rows
    // out of.
    #[test]
    fn a_module_chat_row_carries_no_workspace_index() {
        let app = app_with_a_module_holding_moved_chats(2);
        for entry in workspace_list_entries(&app) {
            if let WorkspaceListEntry::ModuleChat { node_key, .. } = &entry {
                assert_eq!(node_key, "docs", "the row names its container");
            }
            assert!(
                !matches!(entry, WorkspaceListEntry::Chat { .. }),
                "a moved chat is never emitted as a workspace-indexed chat row"
            );
        }
    }

    // TP-CHAT-MOVE-06 (M9): folding the container takes its chats with it.
    // Describing the inside of a closed box undoes closing it.
    #[test]
    fn folding_a_module_hides_the_chats_moved_into_it() {
        let mut app = app_with_a_module_holding_moved_chats(2);
        assert_eq!(module_chat_positions(&app).len(), 2, "precondition");
        // Fold through the state's own verb rather than by writing the set it
        // keeps: the fold key is derived, not the bare node key, and a test
        // that imitates the internal spelling passes or fails for reasons the
        // product does not have.
        app.fold_node("docs".to_string());
        assert!(
            app.node_folded("docs"),
            "precondition: the container really is folded"
        );
        assert!(
            module_chat_positions(&app).is_empty(),
            "a folded container shows none of its chats"
        );
    }

    // TP-CHAT-MOVE-06 (M11): the row is laid out, not merely emitted.
    //
    // This is the test that matters most in this family. The sidebar paints
    // chat rows from the laid-out areas, not from the entry list, so a row
    // that is emitted and measured but never given an area is a row nobody
    // ever sees — with every emission test green. That failure has a history
    // in this repository and this assertion is what keeps it from repeating.
    #[test]
    fn a_module_chat_row_is_laid_out_so_it_can_be_drawn_and_pressed() {
        let app = app_with_a_module_holding_moved_chats(2);
        let (_, _, _, _, _, _, _, module_chats) =
            compute_workspace_list_areas(&app, Rect::new(0, 0, 30, 40));

        assert_eq!(
            module_chats.len(),
            2,
            "both container chat rows get an area of their own"
        );
        assert!(
            module_chats.iter().all(|row| row.node_key == "docs"),
            "each area names the container it belongs to"
        );
        assert!(
            module_chats.iter().all(|row| row.rect.width > 0),
            "an area with no width draws nothing"
        );
    }

    // TP-CHAT-MOVE-06 (M10): the glance budget is the same five every other
    // drawer keeps. A container that collected a thousand transcripts must not
    // bury the tree under its own history.
    #[test]
    fn a_module_shows_the_same_five_chats_every_drawer_does() {
        let app = app_with_a_module_holding_moved_chats(9);
        assert_eq!(
            module_chat_positions(&app).len(),
            WORKSPACE_CHAT_ROW_LIMIT,
            "the glance budget bounds the container the same way it bounds a workspace"
        );
    }

    /// An app whose daily directory holds `chat_count` chats and whose one
    /// workspace lives somewhere else entirely — the shape the section exists
    /// for (nothing claims `$HOME`).
    fn app_with_daily_chats(chat_count: usize) -> (AppState, std::path::PathBuf) {
        let mut app = crate::app::state::AppState::test_new();
        let daily = std::env::temp_dir().join("herdr-daily-fixture");
        let mut workspace = Workspace::test_new("elsewhere");
        workspace.identity_cwd = std::env::temp_dir().join("herdr-daily-elsewhere");
        app.workspaces = vec![workspace];
        app.active = Some(0);
        app.selected = 0;
        app.mobile_width_threshold = 0;
        app.daily_chat_cwd = Some(daily.clone());

        let key = crate::persist::workspace_chats::ledger_key(&daily);
        let rows = (0..chat_count)
            .map(|idx| crate::app::state::WorkspaceChatRow {
                session_id: format!("daily-session-{idx}"),
                agent: "claude".to_string(),
                title: Some(format!("daily chat {idx}")),
                last_seen_ms: 2_000 + idx as u64,
                last_modified: None,
            })
            .collect::<Vec<_>>();
        app.workspace_chat_rows.insert(key, rows);
        (app, daily)
    }

    /// The shape the machine actually had: a workspace born in the daily
    /// directory itself, with no worktree, so `effective_cwd` answers with its
    /// birthplace and it shares the daily area's ledger key.
    fn app_with_a_workspace_standing_in_the_daily_directory(
        chat_count: usize,
    ) -> (AppState, std::path::PathBuf) {
        let (mut app, daily) = app_with_daily_chats(chat_count);
        let mut home = Workspace::test_new("ayaz");
        home.identity_cwd = daily.clone();
        app.workspaces.push(home);
        (app, daily)
    }

    // TP-DAILY-13: the workspace that stands in the daily directory is the
    // area's own. Without this the tree draws it as a branch beside real
    // checkouts — which is exactly what was on screen, seven times over.
    #[test]
    fn a_workspace_standing_in_the_daily_directory_belongs_to_the_area() {
        let (app, _) = app_with_a_workspace_standing_in_the_daily_directory(2);
        assert_eq!(
            daily_owned_workspaces(&app),
            vec![1],
            "the workspace whose effective directory is the daily directory is the area's own"
        );
    }

    // TP-DAILY-13: `effective_cwd` has two branches and both of them count.
    // Reading only `worktree_space` is what made #88's first measurement say
    // "nothing claims $HOME" while seven workspaces did.
    #[test]
    fn a_checkout_that_points_at_the_daily_directory_also_belongs_to_the_area() {
        let (mut app, daily) = app_with_daily_chats(2);
        let mut adopted = Workspace::test_new("adopted");
        adopted.identity_cwd = std::env::temp_dir().join("herdr-daily-birthplace");
        adopted.worktree_space = Some(crate::workspace::WorktreeSpaceMembership {
            key: "adopted-space".to_string(),
            label: "adopted".to_string(),
            repo_root: daily.clone(),
            checkout_path: daily.clone(),
            is_linked_worktree: false,
        });
        app.workspaces.push(adopted);
        assert_eq!(
            daily_owned_workspaces(&app),
            vec![1],
            "the checkout branch of effective_cwd counts the same as the birthplace branch"
        );
    }

    // TP-DAILY-13: no daily directory, nothing to belong to. Without this gate
    // the move has no destination and the rows would vanish instead.
    #[test]
    fn without_a_daily_directory_no_workspace_belongs_to_the_area() {
        let (mut app, _) = app_with_a_workspace_standing_in_the_daily_directory(2);
        app.daily_chat_cwd = None;
        assert!(
            daily_owned_workspaces(&app).is_empty(),
            "with no daily directory there is nothing for a workspace to belong to"
        );
    }

    /// The reported shape: `n` workspaces all standing in the daily directory,
    /// none of them named, exactly as `workspace list` answered on the machine.
    ///
    /// `Workspace::test_new` hands back a workspace that IS named — it stores
    /// its argument as `custom_name` — and that is the opposite of what was
    /// measured: those seven carried no name at all and read `ayaz` only
    /// because that is what `$HOME` is called. Leaving the fixture's name in
    /// place makes every derivation test pass without ever running the
    /// derivation, which is how the first version of these tests went green
    /// while proving nothing.
    fn app_with_n_workspaces_in_the_daily_directory(n: usize) -> (AppState, std::path::PathBuf) {
        let (mut app, daily) = app_with_daily_chats(2);
        for _ in 0..n {
            let mut home = Workspace::test_new("ayaz");
            home.custom_name = None;
            home.identity_cwd = daily.clone();
            app.workspaces.push(home);
        }
        assert!(
            app.workspaces
                .iter()
                .skip(1)
                .all(|ws| ws.custom_name.is_none()),
            "precondition: the measured workspaces carry no name of their own"
        );
        (app, daily)
    }

    // P1.1 / TP-DAILY-17: the root cause. `new_cwd = "follow"` means a new
    // workspace inherits the pane's directory, so a `$HOME` workspace makes the
    // next `$HOME` workspace — five of them within 32 minutes on the machine
    // this was reported from. The second one becomes a TAB instead.
    #[test]
    fn a_second_workspace_in_the_daily_directory_is_adopted_instead() {
        let (app, daily) = app_with_n_workspaces_in_the_daily_directory(1);
        let owned = daily_owned_workspaces(&app);

        assert_eq!(
            app.daily_adoption_target(&daily),
            owned.first().copied(),
            "the place already has an unnamed workspace; a second one is the same place twice"
        );
    }

    // P1.2 / TP-DAILY-17: a NAMED workspace is never adopted. A name is a
    // deliberate identity — if someone said "this one is the log tail", a
    // second one standing beside it is deliberate too, and folding into the
    // first would overrule a decision the machine has no business overruling.
    #[test]
    fn a_named_workspace_in_the_daily_directory_is_not_adopted() {
        let (mut app, daily) = app_with_n_workspaces_in_the_daily_directory(1);
        if let Some(ws) = app.workspaces.last_mut() {
            ws.custom_name = Some("log tail".to_string());
        }

        assert_eq!(
            app.daily_adoption_target(&daily),
            None,
            "an explicit name outranks the tidy-up"
        );
    }

    // P1.3 / TP-DAILY-17: the first one is legitimate. The rule says "no
    // second", not "none".
    #[test]
    fn the_first_workspace_in_the_daily_directory_is_still_created() {
        let (app, daily) = app_with_daily_chats(2);

        assert_eq!(app.daily_adoption_target(&daily), None);
    }

    // P1.4 / TP-DAILY-17: the load-bearing guard. A "new workspace" pressed
    // inside a repository must still make a workspace there — breaking that
    // would be a far larger defect than the duplicate rows being removed.
    #[test]
    fn a_new_workspace_outside_the_daily_directory_is_untouched() {
        let (app, _) = app_with_n_workspaces_in_the_daily_directory(3);
        let elsewhere = std::env::temp_dir().join("herdr-some-repo");

        assert_eq!(
            app.daily_adoption_target(&elsewhere),
            None,
            "the rule is about one directory, not about new workspaces"
        );
    }

    // P2.1 / TP-DAILY-19: the set the merge verb folds. Two or more unnamed
    // workspaces standing in one directory are copies of that directory, and
    // the verb exists to turn them back into one.
    #[test]
    fn every_unnamed_workspace_in_the_daily_directory_is_mergeable() {
        let (app, _) = app_with_n_workspaces_in_the_daily_directory(3);

        assert_eq!(
            app.mergeable_daily_workspaces().len(),
            3,
            "all three stand in the same place and none of them is named"
        );
    }

    // P2.2 / TP-DAILY-19: with one workspace there is nothing to fold, and a
    // verb with no work to do is a button that does nothing. The menu must not
    // promise what the section cannot keep.
    #[test]
    fn a_single_daily_workspace_is_not_a_merge() {
        let (app, _) = app_with_n_workspaces_in_the_daily_directory(1);

        assert!(
            app.mergeable_daily_workspaces().len() < 2,
            "one workspace is already the merged state"
        );
    }

    // P2.4 / TP-DAILY-19: a named workspace is excluded for exactly the reason
    // adoption excludes it (P1.2) — the name is a decision, and a tidy-up must
    // not overrule it.
    #[test]
    fn a_named_workspace_is_never_merged_away() {
        let (mut app, _) = app_with_n_workspaces_in_the_daily_directory(3);
        if let Some(ws) = app.workspaces.last_mut() {
            ws.custom_name = Some("log tail".to_string());
        }

        let mergeable = app.mergeable_daily_workspaces();
        assert_eq!(mergeable.len(), 2, "the named one drops out of the set");
        assert!(
            !mergeable.contains(&(app.workspaces.len() - 1)),
            "and it is specifically the named one that is spared"
        );
    }

    // P2.6 / TP-DAILY-19: the target is the workspace the person is standing
    // in, when that is one of them. A cleanup someone asked for must not carry
    // them out of where they were working — the core-side counterpart of the
    // row order the section already draws (TP-DAILY-18).
    #[test]
    fn the_merge_target_is_the_active_workspace_when_it_is_one_of_them() {
        let (mut app, _) = app_with_n_workspaces_in_the_daily_directory(3);
        let mergeable = app.mergeable_daily_workspaces();
        let last = *mergeable.last().expect("three were pushed");
        assert_ne!(
            last, mergeable[0],
            "precondition: the active one below is NOT the first, or this proves nothing"
        );
        app.active = Some(last);

        assert_eq!(
            app.daily_merge_target(),
            Some(last),
            "the person stays where they already were"
        );
    }

    // P2.6b / TP-DAILY-19: standing somewhere else, the first is the target.
    // Without this the verb would have no destination at all whenever the
    // person pressed it from a repository workspace.
    #[test]
    fn the_merge_target_falls_back_to_the_first_when_the_active_is_elsewhere() {
        let (mut app, _) = app_with_n_workspaces_in_the_daily_directory(3);
        app.active = Some(0); // the fixture's own non-daily workspace

        let mergeable = app.mergeable_daily_workspaces();
        assert!(
            !mergeable.contains(&0),
            "precondition: workspace 0 is not one of the daily ones"
        );
        assert_eq!(app.daily_merge_target(), mergeable.first().copied());
    }

    // P2.9 / TP-DAILY-19: the rule is about ONE directory. A workspace standing
    // in a repository is never in the set, so merging can never reach it —
    // the same boundary P1.4 draws for adoption.
    #[test]
    fn workspaces_outside_the_daily_directory_are_never_mergeable() {
        let (mut app, _) = app_with_n_workspaces_in_the_daily_directory(2);
        let mut repo = Workspace::test_new("herdr");
        repo.custom_name = None;
        repo.identity_cwd = std::env::temp_dir().join("herdr-some-repo");
        app.workspaces.push(repo);
        let repo_idx = app.workspaces.len() - 1;

        assert!(
            !app.mergeable_daily_workspaces().contains(&repo_idx),
            "an unnamed workspace in a repository is still that repository's, not the daily area's"
        );
    }

    // P3.2 / TP-DAILY-18: the reported defect, answered. Seven rows for one
    // place read as spam — "hepsinin içinde aynı chatler var, fark ne ki?" —
    // so the section shows ONE and offers the rest.
    #[test]
    fn the_daily_area_shows_one_workspace_and_a_switch_for_the_rest() {
        let (app, _) = app_with_n_workspaces_in_the_daily_directory(7);
        let entries = workspace_list_entries(&app);

        // Only the daily area's own rows: the fixture also has a workspace
        // living elsewhere, and it is drawn in the tree where it belongs.
        let owned = daily_owned_workspaces(&app);
        let rows = entries
            .iter()
            .filter(|entry| {
                matches!(entry, WorkspaceListEntry::Workspace { ws_idx, .. } if owned.contains(ws_idx))
            })
            .count();
        let switch = entries
            .iter()
            .find_map(|entry| match entry {
                WorkspaceListEntry::DailyMoreWorkspaces { hidden, expanded } => {
                    Some((*hidden, *expanded))
                }
                _ => None,
            })
            .expect("seven workspaces in one place offer a switch");

        assert_eq!(rows, 1, "one place, one row");
        assert_eq!(switch, (6, false), "and six more, folded");
    }

    // P3.3 / TP-DAILY-18: the switch goes both ways. A fold with no way back
    // is not a fold, it is a hiding place — and those six held fifteen panes.
    #[test]
    fn expanding_the_switch_reveals_every_workspace_standing_there() {
        let (mut app, _) = app_with_n_workspaces_in_the_daily_directory(7);
        app.daily_workspaces_expanded = true;

        let owned = daily_owned_workspaces(&app);
        let rows = workspace_list_entries(&app)
            .iter()
            .filter(|entry| {
                matches!(entry, WorkspaceListEntry::Workspace { ws_idx, .. } if owned.contains(ws_idx))
            })
            .count();

        assert_eq!(rows, 7, "every workspace is reachable once asked for");
    }

    // P3.1 / TP-DAILY-18: one workspace offers no switch. "More here" with
    // nothing more here is a control that lies.
    #[test]
    fn a_single_daily_workspace_offers_no_switch() {
        let (app, _) = app_with_n_workspaces_in_the_daily_directory(1);

        assert!(
            !workspace_list_entries(&app)
                .iter()
                .any(|entry| matches!(entry, WorkspaceListEntry::DailyMoreWorkspaces { .. })),
            "a lone workspace has nothing folded behind it"
        );
    }

    // P3.4 / TP-DAILY-18: the workspace you are IN leads. Folding the row you
    // are working in behind the switch is the #88 failure wearing the costume
    // of a tidy-up.
    #[test]
    fn the_workspace_you_are_in_is_the_one_left_visible() {
        let (mut app, _) = app_with_n_workspaces_in_the_daily_directory(4);
        let owned = daily_owned_workspaces(&app);
        let last = *owned.last().expect("four workspaces");
        app.active = Some(last);

        let drawn = workspace_list_entries(&app)
            .iter()
            .find_map(|entry| match entry {
                WorkspaceListEntry::Workspace { ws_idx, .. } if owned.contains(ws_idx) => {
                    Some(*ws_idx)
                }
                _ => None,
            })
            .expect("one daily row is drawn");

        assert_eq!(
            drawn, last,
            "the row you are working in must not be the one folded away"
        );
    }

    // T1.3 / TP-DAILY-16: the defect end to end. Seven workspaces share one
    // directory, so the directory's name names them all — and a name repeated
    // seven times is a category, not a name. Every row must be addressable.
    #[test]
    fn seven_rows_in_one_directory_do_not_all_read_alike() {
        let (mut app, _) = app_with_n_workspaces_in_the_daily_directory(7);
        // One of the seven held a named tab on the machine; the other six did
        // not. Both halves belong in the same assertion: derivation names the
        // one it can, numbering rescues the ones it cannot, and neither alone
        // makes all seven addressable.
        if let Some(ws) = app.workspaces.last_mut() {
            if let Some(tab) = ws.tabs.first_mut() {
                tab.custom_name = Some("HERDR SERVER".to_string());
            }
        }
        let owned = daily_owned_workspaces(&app);
        assert_eq!(owned.len(), 7, "precondition: seven rows share a directory");

        let names = daily_row_names(&app, &owned, None);
        let unique: std::collections::HashSet<&String> = names.values().collect();

        assert_eq!(
            unique.len(),
            7,
            "every row must be tellable from its neighbours: {names:?}"
        );
        assert!(
            names.values().any(|name| name == "HERDR SERVER"),
            "the row that had something to say must say it: {names:?}"
        );
    }

    // T1.1 / TP-DAILY-15: the row takes the name of the tab inside it, so a
    // workspace whose directory says nothing still says something.
    #[test]
    fn a_daily_row_wears_the_name_of_the_tab_inside_it() {
        let (mut app, _) = app_with_n_workspaces_in_the_daily_directory(1);
        if let Some(ws) = app.workspaces.last_mut() {
            if let Some(tab) = ws.tabs.first_mut() {
                tab.custom_name = Some("HERDR SERVER".to_string());
            }
        }
        let owned = daily_owned_workspaces(&app);
        let names = daily_row_names(&app, &owned, None);

        assert_eq!(
            names.values().next().map(String::as_str),
            Some("HERDR SERVER"),
            "a named tab is the one place this workspace's purpose is written down"
        );
    }

    // T1.4 / TP-DAILY-15: an explicit name outranks anything derived. The
    // derivation exists because there was nothing better; here there is.
    #[test]
    fn a_named_daily_workspace_keeps_the_name_its_owner_gave_it() {
        let (mut app, _) = app_with_n_workspaces_in_the_daily_directory(1);
        if let Some(ws) = app.workspaces.last_mut() {
            ws.custom_name = Some("gece nöbeti".to_string());
            if let Some(tab) = ws.tabs.first_mut() {
                tab.custom_name = Some("HERDR SERVER".to_string());
            }
        }
        let owned = daily_owned_workspaces(&app);
        let names = daily_row_names(&app, &owned, None);

        assert_eq!(
            names.values().next().map(String::as_str),
            Some("gece nöbeti"),
            "the derivation must not overrule a name the user typed"
        );
    }

    // TP-DAILY-13: only a real collision moves. If this gate goes, every
    // workspace in the tree falls into the daily area.
    #[test]
    fn a_workspace_that_lives_elsewhere_stays_out_of_the_area() {
        let (app, _) = app_with_daily_chats(2);
        assert!(
            daily_owned_workspaces(&app).is_empty(),
            "the fixture's only workspace lives elsewhere and must not be claimed"
        );
    }

    /// Where a workspace row sits in the emitted list, by index.
    fn workspace_row_position(app: &AppState, ws_idx: usize) -> Option<usize> {
        workspace_list_entries(app).iter().position(
            |entry| matches!(entry, WorkspaceListEntry::Workspace { ws_idx: idx, .. } if *idx == ws_idx),
        )
    }

    fn workspace_row_count(app: &AppState) -> usize {
        workspace_list_entries(app)
            .iter()
            .filter(|entry| matches!(entry, WorkspaceListEntry::Workspace { .. }))
            .count()
    }

    // TP-DAILY-13 (H1): the row moves under the daily area and leaves the tree.
    // Both halves are asserted in one test on purpose — emitting it in both
    // places is real duplication, emitting it in neither loses the way in to
    // fifteen live panes.
    #[test]
    fn the_daily_area_s_own_workspace_sits_under_it_and_not_in_the_tree() {
        let (app, _) = app_with_a_workspace_standing_in_the_daily_directory(2);
        let entries = workspace_list_entries(&app);

        let home = workspace_row_position(&app, 1).expect("the area's own workspace is emitted");
        let elsewhere = workspace_row_position(&app, 0).expect("the tree's workspace is emitted");
        let last_daily_chat = entries
            .iter()
            .rposition(|entry| matches!(entry, WorkspaceListEntry::DailyChat { .. }))
            .expect("the section has chats");

        assert!(
            home > last_daily_chat,
            "the area's own workspace belongs after the section's chats"
        );
        assert!(
            home < elsewhere,
            "it belongs under the daily area, above everything the tree walk emits"
        );
        assert!(
            matches!(
                entries[home],
                WorkspaceListEntry::Workspace { indented: true, .. }
            ),
            "a row inside a container is drawn as one"
        );
    }

    // TP-DAILY-13 (H2): moving a row is not closing it. The workspace, its
    // tabs and its panes are untouched — on the machine this was built for
    // those seven rows held nine tabs and fifteen panes, one of them a blocked
    // agent still waiting.
    #[test]
    fn moving_the_row_does_not_touch_the_workspace_itself() {
        let (app, _) = app_with_a_workspace_standing_in_the_daily_directory(2);
        assert_eq!(app.workspaces.len(), 2, "both workspaces are still there");
        assert!(
            workspace_row_position(&app, 1).is_some(),
            "the moved workspace is still reachable from the list"
        );
    }

    // TP-DAILY-13 (H3): the container takes its contents with it. A section
    // that folds its chats but keeps its workspace rows reads as damage.
    #[test]
    fn folding_the_daily_area_folds_the_workspace_it_owns() {
        let (mut app, _) = app_with_a_workspace_standing_in_the_daily_directory(2);
        assert!(workspace_row_position(&app, 1).is_some(), "precondition");
        app.daily_section_collapsed = true;
        assert_eq!(
            workspace_row_position(&app, 1),
            None,
            "a folded container shows none of its rows"
        );
        assert!(
            workspace_row_position(&app, 0).is_some(),
            "the tree is unaffected by the section's fold"
        );
    }

    // TP-DAILY-13 (H7): the move is a move, never a removal. This set feeds
    // the client frame as well as the sidebar, and the first attempt at this
    // filtered at the source — which took API-created workspaces off every
    // surface at once. Count preserved means no surface lost a row.
    #[test]
    fn moving_the_row_never_removes_it_from_the_shared_set() {
        let (mut app, _) = app_with_a_workspace_standing_in_the_daily_directory(2);
        let moved = workspace_row_count(&app);

        // Same state, with nothing to move to: the tree keeps both rows.
        app.daily_chat_cwd = None;
        let untouched = workspace_row_count(&app);

        assert_eq!(
            moved, untouched,
            "every workspace that had a row still has one; only its place changed"
        );
        assert_eq!(moved, 2, "and both workspaces are that count");
    }

    // TP-DAILY-13 (H8): the moved row is a row, not a picture of one. Hit
    // testing reads the same entry list, so the workspace under the daily area
    // gets a card area and stays clickable — which is the whole reason this is
    // a move and not a hide. Fifteen live panes are reached through it.
    #[test]
    fn the_row_under_the_daily_area_is_still_clickable() {
        let (app, _) = app_with_a_workspace_standing_in_the_daily_directory(2);
        let areas = compute_workspace_card_areas(&app, Rect::new(0, 0, 30, 40));
        assert!(
            areas.iter().any(|card| card.ws_idx == 1),
            "the area's own workspace gets a card area like any other row"
        );
        assert!(
            areas.iter().any(|card| card.ws_idx == 0),
            "and the tree's workspace keeps its own"
        );
    }

    // TP-DAILY-13 (H4 at the emit layer): with no daily directory the rows
    // stay exactly where the tree put them.
    #[test]
    fn without_a_daily_directory_the_tree_keeps_its_rows() {
        let (mut app, _) = app_with_a_workspace_standing_in_the_daily_directory(2);
        app.daily_chat_cwd = None;
        assert!(
            workspace_row_position(&app, 1).is_some(),
            "the workspace is still emitted by the tree walk"
        );
    }

    // TP-DAILY-13: an undrawn section owns nothing. Rows placed under a header
    // that was never emitted are invisible — the #88 class of silent defect.
    #[test]
    fn an_undrawn_daily_area_owns_nothing() {
        let (mut app, _) = app_with_a_workspace_standing_in_the_daily_directory(2);
        assert_eq!(daily_owned_workspaces(&app), vec![1], "precondition: drawn");
        // TP-DAILY-06's filter: focus-only with no resumed daily chat silences
        // the section.
        app.spaces_focus_only = true;
        assert!(
            !daily_section_visible(&app),
            "precondition: the section is now silent"
        );
        assert!(
            daily_owned_workspaces(&app).is_empty(),
            "a section that is not drawn cannot own rows"
        );
    }

    // TP-DAILY-02: the section is born at the very top, above the tree —
    // that is where a person looks for what they were just doing. A chat
    // outside every checkout has no workspace to hang under, so if this row
    // is not first it is nowhere.
    #[test]
    fn the_daily_section_is_emitted_above_the_tree() {
        let (app, _) = app_with_daily_chats(2);
        assert_eq!(
            entry_kinds(&app),
            vec!["daily-header", "daily-chat", "daily-chat", "workspace"]
        );
    }

    // TP-DAILY-02: an empty section is never drawn. A header promising
    // content that is not there reads as a broken surface, and this section
    // is empty on every machine that has never started a chat outside a
    // checkout.
    #[test]
    fn a_daily_section_with_no_chats_is_not_drawn_at_all() {
        let (app, _) = app_with_daily_chats(0);
        assert_eq!(entry_kinds(&app), vec!["workspace"]);

        // Nor when there is no daily directory to read in the first place.
        let (mut homeless, _) = app_with_daily_chats(3);
        homeless.daily_chat_cwd = None;
        assert_eq!(entry_kinds(&homeless), vec!["workspace"]);
    }

    // TP-DAILY-03: folding takes the whole section with it, header excepted —
    // a half-folded section reads as damage rather than as a fold.
    #[test]
    fn folding_the_daily_section_takes_every_row_below_it() {
        let (mut app, _) = app_with_daily_chats(3);
        app.daily_section_collapsed = true;
        assert_eq!(entry_kinds(&app), vec!["daily-header", "workspace"]);
    }

    // TP-DAILY-04: the glance contract, the same one every drawer keeps —
    // five rows and a switch. Without the bound a machine with a thousand
    // home transcripts buries the tree under its own history.
    #[test]
    fn the_daily_section_lists_five_chats_and_offers_the_rest() {
        let (mut app, _) = app_with_daily_chats(7);
        assert_eq!(
            entry_kinds(&app),
            vec![
                "daily-header",
                "daily-chat",
                "daily-chat",
                "daily-chat",
                "daily-chat",
                "daily-chat",
                "daily-more",
                "workspace"
            ]
        );
        assert!(workspace_list_entries(&app)
            .iter()
            .any(|entry| matches!(entry, WorkspaceListEntry::DailyMore { expanded: false })));

        // TP-DRAW-11's sibling: the row is the way in AND the way back, so it
        // stays on screen once opened — a switch with no off position is not
        // a switch.
        app.daily_section_expanded = true;
        let opened = entry_kinds(&app);
        assert_eq!(
            opened.iter().filter(|kind| **kind == "daily-chat").count(),
            7
        );
        assert!(workspace_list_entries(&app)
            .iter()
            .any(|entry| matches!(entry, WorkspaceListEntry::DailyMore { expanded: true })));
    }

    // TP-DAILY-09: a workspace sitting in the daily directory does NOT silence
    // the section. The silence used to be the contract, and on the machine
    // this feature was built for it turned the whole surface off: ten of that
    // session's workspaces had been born in `$HOME`, seven of them outside any
    // checkout, so `effective_cwd` handed back `$HOME` and the claim test fired
    // on every render. Worse, the duplication the old rule guarded against was
    // already there — each of those workspaces reads the same ledger key, so
    // the list was on screen seven times while its one canonical home was not.
    #[test]
    fn a_workspace_sitting_in_the_daily_directory_keeps_the_section() {
        let (mut app, daily) = app_with_daily_chats(3);
        app.workspaces[0].identity_cwd = daily;
        assert_eq!(
            entry_kinds(&app),
            vec![
                "daily-header",
                "daily-chat",
                "daily-chat",
                "daily-chat",
                "workspace"
            ]
        );
    }

    // TP-DAILY-12: the header is drawn in the containers' dialect — the same
    // accent-coloured disclosure arrow every project and module header wears,
    // and the same "⋯" door to its menu. Drawn as a plain heading instead, the
    // area reads as a label ABOVE the tree rather than as the first place IN
    // it, which is precisely the complaint that produced this behaviour.
    #[test]
    fn the_daily_header_is_drawn_in_the_container_dialect() {
        let (mut app, _) = app_with_daily_chats(2);
        let area = Rect::new(0, 0, 30, 20);
        app.view.sidebar_rect = area;
        let (cards, chats, groups, projects, more, empty, daily, _) =
            compute_workspace_list_areas(&app, area);
        app.view.workspace_card_areas = cards;
        app.view.workspace_chat_row_areas = chats;
        app.view.workspace_group_header_areas = groups;
        app.view.workspace_project_header_areas = projects;
        app.view.workspace_more_chats_areas = more;
        app.view.workspace_empty_module_areas = empty;
        app.view.daily_header_area = daily.header;
        app.view.daily_chat_row_areas = daily.chats;
        app.view.daily_more_area = daily.more;
        let header_rect = app.view.daily_header_area.expect("the section is drawn");

        let mut terminal = Terminal::new(TestBackend::new(30, 20)).unwrap();
        terminal
            .draw(|frame| render_sidebar(&app, &TerminalRuntimeRegistry::new(), frame, area))
            .unwrap();
        let buffer = terminal.backend().buffer();

        // The arrow carries the accent every container header uses, and is a
        // span of its own rather than a character inside the label.
        let arrow_x = find_symbol_x(buffer, header_rect.y, header_rect.width, DISCLOSURE_OPEN);
        assert_eq!(
            buffer[(arrow_x, header_rect.y)].style().fg,
            Some(app.palette.accent),
            "the disclosure arrow wears the container accent"
        );

        // The "⋯" door is present while the mouse owns the panel, and it does
        // not sit on top of the "+".
        let dots = header_menu_cell(header_rect);
        let plus = daily_new_chat_cell(header_rect);
        assert!(dots.width > 0 && plus.width > 0);
        assert_ne!(dots.x, plus.x, "the two doors keep separate cells");
        let row = row_text(buffer, header_rect.y, header_rect.width);
        assert!(
            row.contains('⋯'),
            "the header wears the manage chrome: {row:?}"
        );
        assert!(row.contains('+'), "and its plus: {row:?}");
        assert!(
            row.contains(DAILY_SECTION_TITLE),
            "without losing its name: {row:?}"
        );
    }

    // TP-DAILY-10: a header too narrow to hold a "+" reserves no cell at all.
    // A half-drawn plus would look pressable and do nothing — the same reason
    // every other cell on this sidebar refuses below six columns.
    #[test]
    fn a_header_too_narrow_reserves_no_daily_plus() {
        assert_eq!(
            daily_new_chat_cell(Rect::new(0, 0, 5, 1)),
            Rect::default(),
            "five columns is too narrow for a plus"
        );
        let cell = daily_new_chat_cell(Rect::new(0, 3, 20, 1));
        assert_eq!(
            (cell.x, cell.y, cell.width),
            (19, 3, 1),
            "the plus sits on the header's trailing column, like every other plus"
        );
    }

    // TP-DAILY-02/03: the section says what it is, whether it is open, and
    // how much it holds — the last one is the question a folded container
    // cannot otherwise answer, and the reason a fold is safe to leave closed.
    // Its chats speak the same visual dialect as every other chat row.
    #[test]
    fn the_daily_section_draws_a_header_its_count_and_its_chats() {
        let (mut app, _) = app_with_daily_chats(2);
        let area = Rect::new(0, 0, 30, 20);
        app.view.sidebar_rect = area;
        let (cards, chats, groups, projects, more, empty, daily, _) =
            compute_workspace_list_areas(&app, area);
        app.view.workspace_card_areas = cards;
        app.view.workspace_chat_row_areas = chats;
        app.view.workspace_group_header_areas = groups;
        app.view.workspace_project_header_areas = projects;
        app.view.workspace_more_chats_areas = more;
        app.view.workspace_empty_module_areas = empty;
        app.view.daily_header_area = daily.header;
        app.view.daily_chat_row_areas = daily.chats;
        app.view.daily_more_area = daily.more;

        let header_rect = app.view.daily_header_area.expect("the section is drawn");
        assert_eq!(
            app.view.daily_chat_row_areas.len(),
            2,
            "both chats are laid out"
        );

        let mut terminal = Terminal::new(TestBackend::new(30, 20)).unwrap();
        terminal
            .draw(|frame| render_sidebar(&app, &TerminalRuntimeRegistry::new(), frame, area))
            .unwrap();
        let buffer = terminal.backend().buffer();

        let header = row_text(buffer, header_rect.y, header_rect.width);
        assert!(header.contains(DAILY_SECTION_TITLE), "header: {header:?}");
        assert!(header.contains(DISCLOSURE_OPEN), "open arrow: {header:?}");
        // TP-DAILY-10/12: with the mouse on the panel the container chrome
        // owns the trailing columns — the count, then the "⋯", then the "+" —
        // and none of the three is painted over another.
        assert!(
            header.trim_end().ends_with("2 ⋯ +"),
            "count, dots and plus: {header:?}"
        );

        // TP-DAILY-10/12: with the mouse away the chrome is gone and the
        // count takes the trailing column back — no cell is left reserved
        // for chrome that is not drawn.
        app.mouse_capture = false;
        let mut plain = Terminal::new(TestBackend::new(30, 20)).unwrap();
        plain
            .draw(|frame| render_sidebar(&app, &TerminalRuntimeRegistry::new(), frame, area))
            .unwrap();
        let quiet = row_text(plain.backend().buffer(), header_rect.y, header_rect.width);
        assert!(quiet.trim_end().ends_with('2'), "count alone: {quiet:?}");
        assert!(!quiet.contains('+'), "no plus without capture: {quiet:?}");
        assert!(
            !quiet.contains('⋯'),
            "no dots without capture either: {quiet:?}"
        );
        app.mouse_capture = true;

        let first = row_text(
            buffer,
            app.view.daily_chat_row_areas[0].rect.y,
            header_rect.width,
        );
        assert!(first.contains("daily chat 0"), "chat row: {first:?}");

        // Folded, the header stays and states the same count — and the rows
        // below it are gone from the layout, not merely unpainted.
        app.daily_section_collapsed = true;
        let (_, _, _, _, _, _, folded, _) = compute_workspace_list_areas(&app, area);
        assert!(folded.header.is_some());
        assert!(folded.chats.is_empty(), "a fold takes the rows with it");
    }

    // TP-DAILY-03/07: the section's rows keep vectors of their own. Folded
    // into the workspace-indexed ones, every press here would resolve as some
    // other checkout's chat — the failure TP-DRAW-11 pinned for the "older"
    // row, one surface over.
    #[test]
    fn daily_rows_stay_out_of_the_workspace_indexed_vectors() {
        let (mut app, _) = app_with_daily_chats(7);
        app.daily_section_expanded = true;
        // Tall enough that both the opened section and the tree below it are
        // laid out: an opened section CAN push the tree past the viewport,
        // which the list's own scroll answers — but that is a different
        // question from the one this test asks.
        let area = Rect::new(0, 0, 30, 40);
        let (cards, chats, _, _, more, _, daily, _) = compute_workspace_list_areas(&app, area);

        assert_eq!(daily.chats.len(), 7);
        assert!(daily.more.is_some(), "the switch is laid out");
        assert!(
            chats.is_empty() && more.is_empty(),
            "no daily row leaked into the workspace vectors"
        );
        assert_eq!(cards.len(), 1, "the tree still lays out its own checkout");

        // Every daily rect is distinct from every workspace rect: two rows
        // sharing a rect is a click landing on the wrong one.
        for row in &daily.chats {
            assert!(
                !cards.iter().any(|card| card.rect == row.rect),
                "daily rect collided with a workspace card"
            );
        }
    }

    // A sidebar can be dragged narrow; a section that panics or overflows
    // there takes the whole frame with it.
    #[test]
    fn a_narrow_sidebar_still_draws_the_daily_section() {
        let (mut app, _) = app_with_daily_chats(3);
        let area = Rect::new(0, 0, 8, 10);
        app.view.sidebar_rect = area;
        let (_, _, _, _, _, _, daily, _) = compute_workspace_list_areas(&app, area);
        app.view.daily_header_area = daily.header;
        app.view.daily_chat_row_areas = daily.chats;
        app.view.daily_more_area = daily.more;

        let mut terminal = Terminal::new(TestBackend::new(8, 10)).unwrap();
        terminal
            .draw(|frame| render_sidebar(&app, &TerminalRuntimeRegistry::new(), frame, area))
            .unwrap();
    }

    // TP-DAILY-06: focus narrows the tree to what this display is working in,
    // and daily chats are in no tree — so they go quiet with everything else.
    // The exception is a daily chat that is actually running: a filter may
    // narrow what you see, never hide where you are.
    #[test]
    fn focus_hides_the_daily_section_unless_one_of_its_chats_is_running() {
        let (mut app, _) = app_with_daily_chats(2);
        app.spaces_focus_only = true;
        assert!(!entry_kinds(&app).contains(&"daily-header"));

        app.workspaces[0].tabs[0].resumed_session_id = Some("daily-session-1".into());
        assert!(
            entry_kinds(&app).contains(&"daily-header"),
            "a running daily chat keeps its section visible under focus"
        );
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
        let (cards, chat_rows, _headers, _, _, _, _, _) = compute_workspace_list_areas(&app, area);

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

        let (cards, chat_rows, _headers, _, _, _, _, _) =
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

    // T46 · nobody who did not ask for a frame pays for one.
    #[test]
    fn without_a_frame_the_two_sections_keep_exactly_their_old_rectangles() {
        let area = Rect::new(0, 0, 30, 24);
        let bare = expanded_sidebar_sections(area, 0.5, crate::ui::shell::SidebarChrome::NONE);
        let raw = expanded_sidebar_section_frames(area, 0.5);
        assert_eq!(bare, raw, "an unframed section is its own frame");
    }

    // T47 · the two halves are independent surfaces, not one decision.
    #[test]
    fn framing_one_section_leaves_the_other_untouched() {
        let area = Rect::new(0, 0, 30, 24);
        let tint = Some(crate::ui::shell::BarTint::solid(
            ratatui::style::Color::Rgb(250, 179, 135),
        ));
        let (raw_ws, raw_detail) = expanded_sidebar_section_frames(area, 0.5);

        let spaces_only = crate::ui::shell::SidebarChrome {
            spaces: tint,
            agents: None,
            chips: None,
        };
        let (ws, detail) = expanded_sidebar_sections(area, 0.5, spaces_only);
        assert_eq!(
            ws.width,
            raw_ws.width - 2,
            "the frame takes a column each side"
        );
        assert_eq!(ws.height, raw_ws.height - 2);
        assert_eq!(ws.x, raw_ws.x + 1);
        assert_eq!(detail, raw_detail, "the other half did not move");

        let agents_only = crate::ui::shell::SidebarChrome {
            spaces: None,
            agents: tint,
            chips: None,
        };
        let (ws2, detail2) = expanded_sidebar_sections(area, 0.5, agents_only);
        assert_eq!(ws2, raw_ws);
        assert_eq!(detail2.height, raw_detail.height - 2);
    }

    // T49 · a panel too short for a frame keeps its content, not its decoration.
    #[test]
    fn a_section_too_small_for_a_frame_keeps_its_whole_rectangle() {
        let tint = Some(crate::ui::shell::BarTint::solid(
            ratatui::style::Color::Rgb(1, 2, 3),
        ));
        for outer in [
            Rect::new(0, 0, 2, 10),
            Rect::new(0, 0, 10, 2),
            Rect::new(0, 0, 1, 1),
        ] {
            assert_eq!(
                section_content_rect(outer, tint),
                outer,
                "losing the border is cosmetic; losing the panel is not"
            );
        }
    }

    #[test]
    fn expanded_sidebar_sections_handle_tiny_heights() {
        let (ws_area, detail_area) = expanded_sidebar_sections(
            Rect::new(0, 0, 20, 5),
            0.9,
            crate::ui::shell::SidebarChrome::NONE,
        );

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

    // TP-DAILY-14 (H1): the daily area's own row wears its own name. It has no
    // checkout, so the branch it used to show came from whatever repository
    // contained the daily directory — on the machine this was reported from,
    // `$HOME`, and seven rows all read `main`.
    #[test]
    fn a_daily_area_row_keeps_its_own_name_instead_of_a_branch() {
        assert_eq!(
            indented_row_label("ayaz", Some("main"), false, Some("reviewr 2")),
            "reviewr 2",
            "the branch belongs to the directory, not to this row"
        );
    }

    // TP-DAILY-14 (H2): a checkout under a repository header still reads as
    // its branch. That is what tells sibling checkouts apart, and the fix must
    // not reach it.
    #[test]
    fn a_group_child_row_still_reads_as_its_branch() {
        assert_eq!(
            indented_row_label("herdr", Some("worktree/issue-137"), false, None),
            "issue-137"
        );
    }

    // T1.1 / TP-DAILY-15: a single named tab IS the workspace. Carrying both
    // the directory name and the tab name would say one thing twice, and the
    // directory half is the half that repeats.
    #[test]
    fn a_lone_named_tab_names_the_row_outright() {
        assert_eq!(
            content_derived_row_name([Some("HERDR SERVER")], []),
            Some("HERDR SERVER".to_string())
        );
    }

    // T1.2 / TP-DAILY-15: an unnamed tab answers `tab_display_name` with its
    // ordinal — `"1"`. That is a position, not an identity; six workspaces
    // would all become `1`. Only `custom_name` counts, so the row falls
    // through to what is actually running in it.
    #[test]
    fn an_unnamed_tab_yields_to_the_agent_running_in_it() {
        assert_eq!(
            content_derived_row_name([None], ["reviewr"]),
            Some("reviewr".to_string()),
            "a tab number is a position, not a name"
        );
    }

    // T1.5 / TP-DAILY-15: a workspace of several tabs cannot honestly be
    // reduced to one of them, so the row names the first and counts the rest.
    #[test]
    fn a_multi_tab_row_names_the_first_and_counts_the_rest() {
        assert_eq!(
            content_derived_row_name(
                [
                    None,
                    Some("HERDR SERVER"),
                    Some("Jellyfin"),
                    Some("temizlik")
                ],
                []
            ),
            Some("HERDR SERVER +3".to_string())
        );
    }

    // TP-DAILY-15: a blank custom name is not a name. Whitespace would
    // otherwise win over a perfectly good agent label and produce an empty row.
    #[test]
    fn a_blank_tab_name_does_not_count_as_named() {
        assert_eq!(
            content_derived_row_name([Some("   ")], ["reviewr"]),
            Some("reviewr".to_string())
        );
    }

    // TP-DAILY-15: with nothing named and nothing running, there is no content
    // to name the row after; the caller keeps the directory name rather than
    // inventing one.
    #[test]
    fn a_row_with_no_content_derives_no_name() {
        assert_eq!(content_derived_row_name([None], []), None);
    }

    // T1.3 / TP-DAILY-16: the reported defect in one assertion — rows that all
    // resolve alike must still be addressable one by one.
    #[test]
    fn repeated_labels_are_numbered_so_every_row_is_addressable() {
        let mut labels = vec![
            "reviewr".to_string(),
            "reviewr".to_string(),
            "reviewr".to_string(),
        ];
        disambiguate_repeated_labels(&mut labels);

        assert_eq!(labels, vec!["reviewr", "reviewr 2", "reviewr 3"]);
        let unique: std::collections::HashSet<&String> = labels.iter().collect();
        assert_eq!(unique.len(), labels.len(), "no two rows may read alike");
    }

    // TP-DAILY-16: a name that appears once keeps its bare form. Numbering it
    // would announce a series of one.
    #[test]
    fn a_label_that_appears_once_is_left_alone() {
        let mut labels = vec!["HERDR SERVER".to_string(), "reviewr".to_string()];
        disambiguate_repeated_labels(&mut labels);

        assert_eq!(labels, vec!["HERDR SERVER", "reviewr"]);
    }

    // TP-DAILY-16: the ordinal must not collide with a name that already ends
    // in one. Appending blindly would produce two rows reading `reviewr 2`.
    #[test]
    fn numbering_steps_over_a_name_that_already_reads_like_an_ordinal() {
        let mut labels = vec![
            "reviewr".to_string(),
            "reviewr 2".to_string(),
            "reviewr".to_string(),
        ];
        disambiguate_repeated_labels(&mut labels);

        let unique: std::collections::HashSet<&String> = labels.iter().collect();
        assert_eq!(
            unique.len(),
            labels.len(),
            "an ordinal that lands on an existing label defeats its own purpose: {labels:?}"
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
            parent: None,
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

    // TP-SPLIT-CONF-04: one module key spanning two repositories gathers both
    // repositories' checkouts under a single group header — the render half of
    // the "shared key across repos is deliberate" contract.
    #[test]
    fn two_repositories_sharing_a_space_key_render_one_module_group() {
        let mut app = AppState::test_new();
        let mut pwa = crate::workspace::Workspace::test_new("pwa");
        pwa.cached_git_branch = Some("main".into());
        pwa.worktree_space = Some(crate::workspace::WorktreeSpaceMembership {
            key: "/repo/pwa/.git".into(),
            label: "pwa".into(),
            repo_root: std::path::PathBuf::from("/repo/pwa"),
            checkout_path: std::path::PathBuf::from("/repo/pwa"),
            is_linked_worktree: false,
        });
        app.workspaces = vec![worktree_on_branch("alpha", "herdr-web"), pwa];
        app.space_split_rules = vec![
            split_rule(&["herdr-web"], "herdr:web", "Web"),
            crate::spaces::SpaceSplitRule {
                repo_root: std::path::PathBuf::from("/repo/pwa"),
                patterns: vec!["main".to_string()],
                key: "herdr:web".to_string(),
                label: "Web".to_string(),
                icon: None,
                parent: None,
            },
        ];

        let entries = workspace_list_entries(&app);
        assert_eq!(
            entries,
            vec![
                WorkspaceListEntry::GroupHeader {
                    space_key: "herdr:web".into()
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
            "a shared key must merge checkouts from both repositories into one module group"
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

    /// The rows a tree emits, flattened to strings a person can read.
    fn tree_rows(app: &AppState) -> Vec<String> {
        workspace_list_entries(app)
            .into_iter()
            .filter_map(|entry| match entry {
                WorkspaceListEntry::GroupHeader { space_key } => Some(format!("group:{space_key}")),
                WorkspaceListEntry::ProjectHeader { project_key } => {
                    Some(format!("node:{project_key}"))
                }
                WorkspaceListEntry::Workspace { ws_idx, .. } => Some(format!("ws:{ws_idx}")),
                _ => None,
            })
            .collect()
    }

    /// A workspace/rule pair that gives the tree something populated to draw,
    /// so a test about declared containers is never also a test about an
    /// empty list.
    fn app_with_one_populated_bucket() -> AppState {
        let mut app = AppState::test_new();
        app.mobile_width_threshold = 0;
        app.workspaces = vec![
            worktree_on_branch("alpha", "feat/tui-alpha"),
            worktree_on_branch("beta", "feat/tui-beta"),
        ];
        app.space_split_rules = vec![split_rule(&["feat/tui-*"], "herdr:tui", "TUI")];
        app
    }

    // TP-MOD-13: a module declared at top level takes a row even though no
    // checkout climbs to it. The tree is walked from the workspaces up, so a
    // container nothing hangs under is never reached — which is exactly the
    // first thing a person does: name the module, then make the branch.
    #[test]
    fn a_top_level_module_with_no_members_still_takes_a_row() {
        let mut app = app_with_one_populated_bucket();
        app.space_nodes = vec![crate::spaces::SpaceNode {
            key: "group:remote-audio".into(),
            name: "UZAKTAN SES".into(),
            icon: None,
            parent: None,
            dir: None,
        }];

        let rows = tree_rows(&app);
        assert!(
            rows.contains(&"node:group:remote-audio".to_string()),
            "a declared top-level module is drawn: {rows:?}"
        );
    }

    // TP-MOD-14: declared containers come after the rows the workspaces
    // produced. Creating a module must never push the work in progress down
    // the list.
    #[test]
    fn declared_containers_follow_the_rows_the_workspaces_produced() {
        let mut app = app_with_one_populated_bucket();
        app.space_nodes = vec![crate::spaces::SpaceNode {
            key: "group:remote-audio".into(),
            name: "UZAKTAN SES".into(),
            icon: None,
            parent: None,
            dir: None,
        }];

        let rows = tree_rows(&app);
        let last_workspace = rows
            .iter()
            .rposition(|row| row.starts_with("ws:"))
            .expect("the populated bucket drew its checkouts");
        let module = rows
            .iter()
            .position(|row| row == "node:group:remote-audio")
            .expect("the module is drawn");
        assert!(
            module > last_workspace,
            "declared scaffolding sits below the work: {rows:?}"
        );
    }

    // TP-MOD-15: a rule that currently claims nothing draws no header — a
    // header for an empty bucket is a ghost — but a module the
    // user declared under it is theirs, not the rule's, and survives.
    #[test]
    fn a_module_under_an_empty_bucket_survives_while_the_bucket_stays_hidden() {
        let mut app = app_with_one_populated_bucket();
        app.space_split_rules
            .push(split_rule(&["asla-eslesmeyecek/*"], "herdr:bos", "Bos"));
        app.space_nodes = vec![crate::spaces::SpaceNode {
            key: "group:altinda".into(),
            name: "ALTINDA".into(),
            icon: None,
            parent: Some("herdr:bos".into()),
            dir: None,
        }];

        let rows = tree_rows(&app);
        assert!(
            rows.contains(&"node:group:altinda".to_string()),
            "the declared module survives its empty parent: {rows:?}"
        );
        assert!(
            !rows.contains(&"group:herdr:bos".to_string()),
            "a rule claiming nothing still draws no header: {rows:?}"
        );
    }

    // TP-MOD-16: an all-empty chain is drawn whole, parent before child, so
    // scaffolding can be built top-down before any branch exists.
    #[test]
    fn a_chain_of_empty_modules_is_drawn_in_order() {
        let mut app = app_with_one_populated_bucket();
        let node = |key: &str, parent: Option<&str>| crate::spaces::SpaceNode {
            key: key.into(),
            name: key.into(),
            icon: None,
            parent: parent.map(str::to_string),
            dir: None,
        };
        app.space_nodes = vec![
            node("group:a", None),
            node("group:b", Some("group:a")),
            node("group:c", Some("group:b")),
        ];

        let rows = tree_rows(&app);
        let at = |key: &str| {
            rows.iter()
                .position(|row| row == &format!("node:{key}"))
                .unwrap_or_else(|| panic!("{key} is drawn: {rows:?}"))
        };
        assert!(at("group:a") < at("group:b"), "{rows:?}");
        assert!(at("group:b") < at("group:c"), "{rows:?}");
    }

    // TP-MOD-17: a container reachable both from a checkout's climb and from
    // the declared forest is drawn once. Two sources feeding one list is the
    // obvious way to draw everything twice.
    #[test]
    fn a_container_reachable_from_both_sources_is_drawn_once() {
        let mut app = app_with_one_populated_bucket();
        app.space_projects = vec![project_over("project:herdr", &["/repo/herdr"], &[])];
        app.space_nodes = vec![crate::spaces::SpaceNode {
            key: "group:remote-audio".into(),
            name: "UZAKTAN SES".into(),
            icon: None,
            parent: Some("project:herdr".into()),
            dir: None,
        }];

        let rows = tree_rows(&app);
        assert_eq!(
            rows.iter()
                .filter(|row| *row == "node:group:remote-audio")
                .count(),
            1,
            "one row per container: {rows:?}"
        );
        assert_eq!(
            rows.iter()
                .filter(|row| *row == "node:project:herdr")
                .count(),
            1,
            "the project is not re-emitted either: {rows:?}"
        );
    }

    // TP-MOD-18: a tree with no declared containers is byte-for-byte the tree
    // it was before the second source existed. Everyone who does not use
    // modules must see no change at all.
    #[test]
    fn a_tree_without_containers_is_unchanged() {
        let mut app = AppState::test_new();
        app.mobile_width_threshold = 0;
        app.workspaces = vec![
            worktree_on_branch("alpha", "feat/tui-alpha"),
            worktree_on_branch("beta", "feat/tui-beta"),
            Workspace::test_new("solo"),
        ];
        app.space_split_rules = vec![split_rule(&["feat/tui-*"], "herdr:tui", "TUI")];

        assert_eq!(
            tree_rows(&app),
            vec!["group:herdr:tui", "ws:0", "ws:1", "ws:2"],
            "no containers declared, so nothing new is emitted"
        );
    }

    // TP-MOD-19: folding a declared container hides its declared children,
    // exactly as folding hides members (TP-TREE-17). A second source must
    // obey the same fold contract as the first.
    #[test]
    fn folding_a_declared_module_hides_its_declared_children() {
        let mut app = app_with_one_populated_bucket();
        app.space_nodes = vec![
            crate::spaces::SpaceNode {
                key: "group:ust".into(),
                name: "UST".into(),
                icon: None,
                parent: None,
                dir: None,
            },
            crate::spaces::SpaceNode {
                key: "group:alt".into(),
                name: "ALT".into(),
                icon: None,
                parent: Some("group:ust".into()),
                dir: None,
            },
        ];
        app.fold_node("group:ust".to_string());

        let rows = tree_rows(&app);
        assert!(
            rows.contains(&"node:group:ust".to_string()),
            "the folded module keeps its own row: {rows:?}"
        );
        assert!(
            !rows.contains(&"node:group:alt".to_string()),
            "a folded container hides what it holds: {rows:?}"
        );
    }

    // TP-MOD-01: a module the user created takes a row even with nothing in
    // it yet, so the scaffolding can be built before the branches exist.
    //
    // This is the shape a person actually hits: they open the header menu,
    // name a module, and expect to see it. It reached the tree only after the
    // managed overlay learned to carry containers (TP-MOVL-01); before that
    // the entry never left the file. The row is pinned here because the
    // emission that draws it is incidental — an empty node is walked as a
    // child of an emitted parent, and nothing else says it must be.
    #[test]
    fn an_empty_module_under_a_drawn_project_takes_a_row_of_its_own() {
        let mut app = AppState::test_new();
        app.mobile_width_threshold = 0;
        // Two members: a single-member parented bucket folds its header into
        // the row (TP-NODE-05), and this test is about where headers sit.
        app.workspaces = vec![
            worktree_on_branch("alpha", "feat/tui-alpha"),
            worktree_on_branch("beta", "feat/tui-beta"),
        ];
        app.space_split_rules = vec![split_rule(&["feat/tui-*"], "herdr:tui", "TUI")];
        app.space_projects = vec![project_over("project:herdr", &["/repo/herdr"], &[])];
        app.space_nodes = vec![
            crate::spaces::SpaceNode {
                key: "group:remote-audio".into(),
                name: "UZAKTAN SES".into(),
                icon: None,
                parent: Some("project:herdr".into()),
                dir: None,
            },
            crate::spaces::SpaceNode {
                key: "group:remote-video".into(),
                name: "UZAKTAN FILM".into(),
                icon: None,
                parent: Some("project:herdr".into()),
                dir: None,
            },
        ];

        let rows = tree_rows(&app);
        assert!(
            rows.contains(&"node:group:remote-audio".to_string()),
            "an empty module the user created must be visible: {rows:?}"
        );
        assert!(
            rows.contains(&"node:group:remote-video".to_string()),
            "a second empty module is visible too: {rows:?}"
        );
        // TP-NODE-04's ordering half: what has members comes first, so making
        // a module never pushes the work the user is doing down the list.
        let bucket = rows
            .iter()
            .position(|row| row == "group:herdr:tui")
            .expect("the populated bucket is drawn");
        let empty = rows
            .iter()
            .position(|row| row == "node:group:remote-audio")
            .expect("the empty module is drawn");
        assert!(bucket < empty, "populated before empty: {rows:?}");
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

    // TP-TREE-16: a module bucket can carry modules of its own — a node
    // whose parent is a split rule's key is drawn under that bucket,
    // pre-order, with its own buckets following. Modules and buckets are
    // one tree to the person using them (TP-NODE-08).
    #[test]
    fn a_node_under_a_bucket_is_drawn_inside_that_bucket() {
        let mut app = AppState::test_new();
        app.mobile_width_threshold = 0;
        app.workspaces = vec![
            worktree_on_branch("alpha", "feat/t4f-alpha"),
            worktree_on_branch("beta", "feat/t4f-beta"),
            // Two members, so the child bucket keeps its own header —
            // a single-member parented bucket folds into the row (TP-NODE-05).
            worktree_on_branch("probe", "probe/one"),
            worktree_on_branch("probe2", "probe/two"),
        ];
        let mut inner_rule = split_rule(&["probe/*"], "herdr:probe", "Probe");
        inner_rule.parent = Some("group:inner".into());
        app.space_split_rules = vec![split_rule(&["feat/t4f-*"], "herdr:t4f", "T4F"), inner_rule];
        app.space_nodes = vec![crate::spaces::SpaceNode {
            key: "group:inner".into(),
            name: "Inner".into(),
            icon: None,
            parent: Some("herdr:t4f".into()),
            dir: None,
        }];

        let entries: Vec<String> = workspace_list_entries(&app)
            .into_iter()
            .filter_map(|entry| match entry {
                WorkspaceListEntry::GroupHeader { space_key } => Some(format!("group:{space_key}")),
                WorkspaceListEntry::ProjectHeader { project_key } => {
                    Some(format!("node:{project_key}"))
                }
                WorkspaceListEntry::Workspace { ws_idx, .. } => Some(format!("ws:{ws_idx}")),
                _ => None,
            })
            .collect();
        assert_eq!(
            entries,
            vec![
                "group:herdr:t4f",
                "ws:0",
                "ws:1",
                "node:group:inner",
                "group:herdr:probe",
                "ws:2",
                "ws:3",
            ],
            "the bucket's own block leads, then its child module and that module's bucket"
        );
    }

    // TP-TREE-17: folding the bucket hides the modules hanging under it,
    // exactly as it hides its member checkouts.
    #[test]
    fn a_folded_bucket_hides_the_modules_under_it() {
        let mut app = AppState::test_new();
        app.mobile_width_threshold = 0;
        app.workspaces = vec![
            worktree_on_branch("alpha", "feat/t4f-alpha"),
            worktree_on_branch("probe", "probe/one"),
        ];
        let mut inner_rule = split_rule(&["probe/*"], "herdr:probe", "Probe");
        inner_rule.parent = Some("group:inner".into());
        app.space_split_rules = vec![split_rule(&["feat/t4f-*"], "herdr:t4f", "T4F"), inner_rule];
        app.space_nodes = vec![crate::spaces::SpaceNode {
            key: "group:inner".into(),
            name: "Inner".into(),
            icon: None,
            parent: Some("herdr:t4f".into()),
            dir: None,
        }];
        app.collapsed_space_keys.insert("herdr:t4f".into());

        let entries = workspace_list_entries(&app);
        assert!(
            !entries.iter().any(|entry| matches!(
                entry,
                WorkspaceListEntry::ProjectHeader { project_key } if project_key == "group:inner"
            )),
            "a folded bucket hides its child modules"
        );
        assert!(
            !entries.iter().any(|entry| matches!(
                entry,
                WorkspaceListEntry::GroupHeader { space_key } if space_key == "herdr:probe"
            )),
            "and the buckets hanging under those modules"
        );
    }

    // TP-TREE-16 companion: depth is counted across mixed chains — a node
    // under a bucket under a node is two steps in, not one.
    #[test]
    fn node_depth_walks_through_buckets() {
        let mut app = AppState::test_new();
        let mut mid = split_rule(&["x*"], "bucket:mid", "Mid");
        mid.parent = Some("group:top".into());
        app.space_split_rules = vec![mid];
        app.space_nodes = vec![
            crate::spaces::SpaceNode {
                key: "group:top".into(),
                name: "Top".into(),
                icon: None,
                parent: None,
                dir: None,
            },
            crate::spaces::SpaceNode {
                key: "group:leaf".into(),
                name: "Leaf".into(),
                icon: None,
                parent: Some("bucket:mid".into()),
                dir: None,
            },
        ];

        assert_eq!(
            node_depth(&app, "group:leaf"),
            2,
            "leaf hangs under the bucket, the bucket under top"
        );
    }

    /// A third checkout under the same repository, so the fixture has a row
    /// that focus can legitimately hide.
    fn app_with_three_checkouts() -> AppState {
        let mut app = app_with_worktree_tree(40);
        let mut idle = Workspace::test_new("docs");
        idle.worktree_space = Some(crate::workspace::WorktreeSpaceMembership {
            key: "repo-key".into(),
            label: "herdr".into(),
            repo_root: std::path::PathBuf::from("/repo/herdr"),
            checkout_path: std::path::PathBuf::from("/repo/herdr-docs"),
            is_linked_worktree: true,
        });
        idle.custom_name = None;
        idle.identity_cwd = std::path::PathBuf::from("/repo/herdr-docs");
        idle.cached_git_branch = Some("docs/readme".into());
        app.workspaces.push(idle);
        app
    }

    fn workspace_indices(entries: &[WorkspaceListEntry]) -> Vec<usize> {
        entries
            .iter()
            .filter_map(|entry| match entry {
                WorkspaceListEntry::Workspace { ws_idx, .. } => Some(*ws_idx),
                _ => None,
            })
            .collect()
    }

    fn mark_agent_in(app: &mut AppState, ws_idx: usize) {
        app.ensure_test_terminals();
        let pane_id = app.workspaces[ws_idx].tabs[0].root_pane;
        let terminal_id = app.workspaces[ws_idx].tabs[0]
            .panes
            .get(&pane_id)
            .map(|pane| pane.attached_terminal_id.clone())
            .expect("the root pane has a terminal");
        if let Some(terminal) = app.terminals.get_mut(&terminal_id) {
            terminal.set_detected_state(
                Some(crate::detect::Agent::Pi),
                crate::detect::AgentState::Idle,
            );
        }
    }

    /// A workspace whose drawer holds more chats than the glance limit.
    fn app_with_a_deep_drawer(chat_count: usize) -> AppState {
        let mut app = app_with_worktree_tree(40);
        let key =
            crate::persist::workspace_chats::ledger_key(&std::path::PathBuf::from("/repo/herdr"));
        let rows = (0..chat_count)
            .map(|i| crate::app::state::WorkspaceChatRow {
                session_id: format!("session-{i}"),
                agent: "claude".to_string(),
                title: Some(format!("chat {i}")),
                last_seen_ms: 1_000 + i as u64,
                last_modified: None,
            })
            .collect();
        app.workspace_chat_rows.insert(key.clone(), rows);
        app.expanded_chat_workspaces.insert(key);
        app
    }

    fn drawer_rows(app: &AppState, ws_idx: usize) -> (usize, Option<bool>) {
        let entries = workspace_list_entries(app);
        let chats = entries
            .iter()
            .filter(
                |entry| matches!(entry, WorkspaceListEntry::Chat { ws_idx: w, .. } if *w == ws_idx),
            )
            .count();
        let more = entries.iter().find_map(|entry| match entry {
            WorkspaceListEntry::MoreChats {
                ws_idx: w,
                expanded,
            } if *w == ws_idx => Some(*expanded),
            _ => None,
        });
        (chats, more)
    }

    // TP-DRAW-10: the drawer is a glance surface until asked otherwise — five
    // rows and a way to see the rest. Opened all the way it shows every chat
    // it holds, and the row that opened it stays as the way back.
    #[test]
    fn a_deep_drawer_shows_five_until_it_is_opened_all_the_way() {
        let mut app = app_with_a_deep_drawer(9);

        assert_eq!(
            drawer_rows(&app, 0),
            (WORKSPACE_CHAT_ROW_LIMIT, Some(false)),
            "the glance surface keeps its five and offers the rest"
        );

        app.toggle_full_chat_drawer(0);
        assert_eq!(
            drawer_rows(&app, 0),
            (9, Some(true)),
            "opened all the way, every chat is drawn and the row stays as the way back"
        );

        app.toggle_full_chat_drawer(0);
        assert_eq!(
            drawer_rows(&app, 0),
            (WORKSPACE_CHAT_ROW_LIMIT, Some(false)),
            "the same row folds it back — a switch with no off position is not a switch"
        );
    }

    // TP-DRAW-11: the row is drawn, and it says which way it goes. It was laid
    // out but never painted before, so the drawer ended in a blank line.
    #[test]
    fn the_older_chats_row_is_painted_and_says_which_way_it_goes() {
        let mut app = app_with_a_deep_drawer(9);
        let rows = drawn_sidebar_rows(&mut app, 24);
        assert!(
            rows.iter().any(|row| row.contains("… 4 older")),
            "the folded drawer names how many chats it is hiding: {rows:?}"
        );

        // Opened, the drawer is nine rows deeper, so the assertion needs a
        // panel tall enough to reach its last row — the row's existence is
        // proven by the entries test; this one is about what it says.
        app.toggle_full_chat_drawer(0);
        let rows = drawn_sidebar_rows(&mut app, 34);
        assert!(
            rows.iter().any(|row| row.contains("… fewer")),
            "the opened drawer offers the way back: {rows:?}"
        );
    }

    // TP-DRAW-12: how deep a drawer is opened belongs to the screen doing the
    // reading — one display digging through old chats must not stretch the
    // drawer on another.
    #[test]
    fn opening_a_drawer_all_the_way_stays_on_this_display() {
        let mut here = app_with_a_deep_drawer(9);
        let there = app_with_a_deep_drawer(9);

        here.toggle_full_chat_drawer(0);

        assert_eq!(drawer_rows(&here, 0).0, 9);
        assert_eq!(
            drawer_rows(&there, 0).0,
            WORKSPACE_CHAT_ROW_LIMIT,
            "the other screen keeps the drawer it was reading"
        );
    }

    // TP-FOCUS-SW-01: focus is opt-in. While it is off the tree is exactly
    // the tree it has always been — the filter may not change what an
    // unfocused screen shows, or every reader would have to check a toggle
    // before trusting the list.
    #[test]
    fn an_unfocused_tree_is_the_whole_tree() {
        let mut app = app_with_three_checkouts();
        app.spaces_focus_only = false;

        assert_eq!(focus_visible_workspaces(&app), None);
        assert_eq!(
            workspace_indices(&workspace_list_entries(&app)),
            vec![0, 1, 2]
        );
    }

    // TP-FOCUS-SW-02: focus keeps what is being worked in — the active
    // checkout and every checkout running an agent — and drops the rest.
    // This is the complaint the switch exists for: a tree that grows past
    // the screen while the work happens in two of its rows.
    #[test]
    fn a_focused_tree_keeps_the_active_checkout_and_the_running_ones() {
        let mut app = app_with_three_checkouts();
        mark_agent_in(&mut app, 2);
        app.active = Some(0);
        app.selected = 0;
        app.spaces_focus_only = true;

        let visible = focus_visible_workspaces(&app).expect("focus narrows the tree");
        assert!(visible.contains(&0), "the active checkout always stays");
        assert!(visible.contains(&2), "a checkout running an agent stays");
        assert!(
            !visible.contains(&1),
            "an idle checkout nobody is in is exactly the noise focus removes"
        );

        let entries = workspace_list_entries(&app);
        assert_eq!(workspace_indices(&entries), vec![0, 2]);
    }

    // TP-FOCUS-SW-02 (headers): a module survives on its members. With every
    // member of a group hidden the group header goes with them — a header
    // over nothing is the noise the switch was asked to remove.
    #[test]
    fn a_focused_tree_drops_the_headers_left_without_members() {
        let mut app = app_with_three_checkouts();
        // A module earns a header at two members (TP-SPLIT-GROUP-03), so the
        // fixture gives docs a second checkout — otherwise the test would be
        // asserting against a header the tree never draws.
        let mut second_doc = Workspace::test_new("api-docs");
        second_doc.worktree_space = Some(crate::workspace::WorktreeSpaceMembership {
            key: "repo-key".into(),
            label: "herdr".into(),
            repo_root: std::path::PathBuf::from("/repo/herdr"),
            checkout_path: std::path::PathBuf::from("/repo/herdr-docs-api"),
            is_linked_worktree: true,
        });
        second_doc.custom_name = None;
        second_doc.identity_cwd = std::path::PathBuf::from("/repo/herdr-docs-api");
        second_doc.cached_git_branch = Some("docs/api".into());
        app.workspaces.push(second_doc);
        app.space_split_rules = vec![crate::spaces::SpaceSplitRule {
            repo_root: std::path::PathBuf::from("/repo/herdr"),
            patterns: vec!["docs/*".into()],
            key: "herdr:docs".into(),
            label: "Docs".into(),
            icon: None,
            parent: None,
        }];
        app.active = Some(0);
        app.selected = 0;

        app.spaces_focus_only = false;
        let open = workspace_list_entries(&app);
        assert!(
            open.iter().any(|entry| matches!(
                entry,
                WorkspaceListEntry::GroupHeader { space_key } if space_key == "herdr:docs"
            )),
            "the docs module is drawn while the tree is open"
        );

        app.spaces_focus_only = true;
        let focused = workspace_list_entries(&app);
        assert!(
            !focused.iter().any(|entry| matches!(
                entry,
                WorkspaceListEntry::GroupHeader { space_key } if space_key == "herdr:docs"
            )),
            "with its only member hidden the module header goes quiet too"
        );
    }

    // TP-FOCUS-SW-03: a filter that would empty the tree keeps its hands off
    // it. With nothing active and nothing running there is no noise to
    // remove, and a blank sidebar reads as broken rather than focused.
    #[test]
    fn a_focus_with_nothing_to_show_shows_everything() {
        let mut app = app_with_three_checkouts();
        app.active = None;
        app.mode = crate::app::state::Mode::Terminal;
        app.spaces_focus_only = true;

        assert_eq!(
            focus_visible_workspaces(&app),
            None,
            "no candidates means no filter, not an empty tree"
        );
        assert_eq!(
            workspace_indices(&workspace_list_entries(&app)),
            vec![0, 1, 2]
        );
    }

    // TP-FOCUS-SW-05: focus is a property of the screen doing the looking.
    // Two clients share the same workspaces; narrowing one must leave the
    // other's tree exactly as it was.
    #[test]
    fn focusing_one_display_leaves_the_other_tree_alone() {
        let mut focused = app_with_three_checkouts();
        let mut wide = app_with_three_checkouts();
        mark_agent_in(&mut focused, 2);
        mark_agent_in(&mut wide, 2);

        focused.spaces_focus_only = true;
        // The second display never touched the toggle, so its own state
        // still answers "show everything".
        assert!(!wide.spaces_focus_only);

        assert_eq!(
            workspace_indices(&workspace_list_entries(&focused)),
            vec![0, 2]
        );
        assert_eq!(
            workspace_indices(&workspace_list_entries(&wide)),
            vec![0, 1, 2],
            "the other screen keeps the tree it was reading"
        );
    }

    // TP-DOTS-03: the manage chrome is mouse chrome — "⋯" appears on the
    // workspace card (one column left of the "+") and on both header rows
    // while the mouse owns the sidebar, and on none of them otherwise.
    // TP-DOTS-09: the chrome is reserved, not overdrawn — a long name is
    // truncated short of it instead of bleeding into its cells.
    #[test]
    fn the_manage_dots_are_mouse_chrome_on_every_tree_level() {
        let mut app = app_with_worktree_tree(40);
        app.space_projects = vec![project_over("project:herdr", &["/repo/herdr"], &[])];
        app.workspaces[0]
            .set_custom_name("a very long workspace name that would bleed into the chrome".into());
        app.mouse_capture = true;

        let area = Rect::new(0, 0, 40, 24);
        let buffer = draw_tree(&mut app, area);

        let card = app
            .view
            .workspace_card_areas
            .first()
            .expect("a card is laid out");
        let dots = workspace_menu_cell(card.rect);
        assert!(dots.width > 0, "the card reserves a manage cell");
        assert_eq!(
            buffer[(dots.x, dots.y)].symbol(),
            "⋯",
            "the card draws the manage dots while the mouse owns the sidebar"
        );
        let plus = workspace_new_chat_cell(card.rect);
        assert_eq!(
            buffer[(plus.x, plus.y)].symbol(),
            "+",
            "the long name stays short of the plus (TP-DOTS-09)"
        );
        assert_eq!(
            buffer[(dots.x + 1, dots.y)].symbol(),
            " ",
            "one breathing cell separates the dots from the plus"
        );

        let project_head = app.view.workspace_project_header_areas[0].rect;
        let head_dots = header_menu_cell(project_head);
        assert!(head_dots.width > 0, "the project header reserves the cell");
        assert_eq!(
            buffer[(head_dots.x, head_dots.y)].symbol(),
            "⋯",
            "the project header draws the manage dots"
        );
        // TP-DOTS-17: the header carries a "+" on its trailing edge — the
        // card layout `[⋯] [+]`, one level up — and the dots sit a breathing
        // cell to its left.
        let head_plus = header_new_branch_cell(project_head);
        assert!(
            head_plus.width > 0,
            "the project header reserves a '+' cell"
        );
        assert_eq!(
            buffer[(head_plus.x, head_plus.y)].symbol(),
            "+",
            "the project header draws the new-branch plus"
        );
        assert_eq!(
            head_dots.x,
            head_plus.x - 2,
            "one breathing cell separates the header dots from the plus"
        );

        let group_head = app.view.workspace_group_header_areas[0].rect;
        let group_dots = header_menu_cell(group_head);
        assert_eq!(
            buffer[(group_dots.x, group_dots.y)].symbol(),
            "⋯",
            "the bucket header draws the manage dots"
        );
        let group_plus = header_new_branch_cell(group_head);
        assert_eq!(
            buffer[(group_plus.x, group_plus.y)].symbol(),
            "+",
            "the bucket header draws the new-branch plus"
        );

        app.mouse_capture = false;
        let buffer = draw_tree(&mut app, area);
        assert_ne!(
            buffer[(dots.x, dots.y)].symbol(),
            "⋯",
            "without the mouse the card cell holds no dots"
        );
        assert_ne!(
            buffer[(head_dots.x, head_dots.y)].symbol(),
            "⋯",
            "without the mouse the header cell holds no dots"
        );
        assert_ne!(
            buffer[(head_plus.x, head_plus.y)].symbol(),
            "+",
            "without the mouse the header cell holds no plus"
        );
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
    fn node_over(key: &str, parent: Option<&str>) -> crate::spaces::SpaceNode {
        crate::spaces::SpaceNode {
            key: key.to_string(),
            name: key.to_string(),
            icon: None,
            parent: parent.map(str::to_string),
            dir: None,
        }
    }

    /// Two checkouts of one repo, claimed by one parented rule — the smallest
    /// world where a node chain has something to hang.
    fn app_with_node_chain() -> AppState {
        let mut app = app_with_worktree_tree(60);
        app.space_nodes = vec![
            node_over("node:root", None),
            node_over("node:ui", Some("node:root")),
        ];
        let mut rule = split_rule(&["master", "fix/*"], "herdr:all", "All");
        rule.parent = Some("node:ui".to_string());
        app.space_split_rules = vec![rule];
        app
    }

    // TP-NODE-04: the chain reads top-down — every ancestor header appears
    // before the child, and the bucket sits under its own node.
    #[test]
    fn nested_nodes_emit_parent_before_child_before_the_bucket() {
        let app = app_with_node_chain();

        let entries = workspace_list_entries(&app);
        let shape: Vec<String> = entries
            .iter()
            .map(|entry| match entry {
                WorkspaceListEntry::ProjectHeader { project_key } => {
                    format!("node({project_key})")
                }
                WorkspaceListEntry::GroupHeader { space_key } => format!("bucket({space_key})"),
                WorkspaceListEntry::Workspace { ws_idx, indented } => {
                    format!("ws({ws_idx},{indented})")
                }
                other => format!("{other:?}"),
            })
            .collect();

        assert_eq!(
            shape[..4].to_vec(),
            vec![
                "node(node:root)".to_string(),
                "node(node:ui)".to_string(),
                "bucket(herdr:all)".to_string(),
                "ws(0,true)".to_string(),
            ],
            "root before child before bucket before member: {shape:?}"
        );
    }

    // TP-NODE-05: a parented bucket with one member needs no header of its
    // own — the checkout hangs straight under the node, indented, so "move
    // this branch under X" reads as exactly that.
    #[test]
    fn a_single_member_parented_bucket_hangs_its_member_under_the_node() {
        let mut app = app_with_node_chain();
        app.workspaces.truncate(1);

        let entries = workspace_list_entries(&app);
        let shape: Vec<String> = entries
            .iter()
            .map(|entry| match entry {
                WorkspaceListEntry::ProjectHeader { project_key } => {
                    format!("node({project_key})")
                }
                WorkspaceListEntry::GroupHeader { space_key } => format!("bucket({space_key})"),
                WorkspaceListEntry::Workspace { ws_idx, indented } => {
                    format!("ws({ws_idx},{indented})")
                }
                other => format!("{other:?}"),
            })
            .collect();

        assert!(
            shape.contains(&"node(node:ui)".to_string())
                && shape.contains(&"ws(0,true)".to_string())
                && !shape.iter().any(|s| s.starts_with("bucket(")),
            "one member, no bucket header, member indented under the node: {shape:?}"
        );
    }

    // TP-WSID-03: the drawer reads by the directory the row MEANS. A
    // workspace born in a shared directory but carrying a checkout shows the
    // checkout's chats — the shared birthplace list never bleeds in.
    #[test]
    fn the_drawer_reads_by_the_checkout_not_the_birthplace() {
        let mut app = crate::app::state::AppState::test_new();
        let mut ws = crate::workspace::Workspace::test_new("adopted");
        ws.identity_cwd = std::path::PathBuf::from("/home/user");
        ws.worktree_space = Some(crate::workspace::WorktreeSpaceMembership {
            key: "repo-key".into(),
            label: "herdr".into(),
            repo_root: std::path::PathBuf::from("/repo/herdr"),
            checkout_path: std::path::PathBuf::from("/repo/herdr-branch"),
            is_linked_worktree: true,
        });
        app.workspaces = vec![ws];
        let row = |id: &str| crate::app::state::WorkspaceChatRow {
            session_id: id.to_string(),
            agent: "claude".to_string(),
            title: None,
            last_seen_ms: 1,
            last_modified: None,
        };
        app.workspace_chat_rows
            .insert("/home/user".to_string(), vec![row("home-chat")]);
        app.workspace_chat_rows
            .insert("/repo/herdr-branch".to_string(), vec![row("branch-chat")]);

        let ids: Vec<_> = workspace_chat_rows_for(&app, 0)
            .iter()
            .map(|r| r.session_id.as_str())
            .collect();
        assert_eq!(ids, vec!["branch-chat"], "only the checkout's list shows");
    }

    // TP-NODE-06: folding a node is a statement about one screen.
    #[test]
    fn one_displays_node_fold_never_moves_anothers_tree() {
        let mut app = app_with_node_chain();

        // Both displays exist before either acts (TP-SUR-DEFAULT-01).
        app.enter_viewer(Some(2));
        app.restore_viewer(None);
        app.enter_viewer(Some(3));
        app.restore_viewer(None);

        app.enter_viewer(Some(2));
        app.fold_node("node:root".to_string());
        let folded_len = workspace_list_entries(&app).len();
        app.restore_viewer(None);

        app.enter_viewer(Some(3));
        let other = workspace_list_entries(&app);
        assert!(
            other.len() > folded_len,
            "display 3 still sees the open subtree"
        );
        app.restore_viewer(None);

        app.enter_viewer(Some(2));
        assert_eq!(
            workspace_list_entries(&app).len(),
            folded_len,
            "display 2 keeps the fold it made"
        );
        app.restore_viewer(None);
    }

    // TP-NODE-07: a fold recorded by the old session-wide project set still
    // reads as folded, and unfolding it moves the truth into the per-display
    // set — the one-way door of the migration.
    #[test]
    fn a_folded_legacy_project_key_still_reads_folded_and_unfolds_forward() {
        let mut app = app_with_node_chain();
        app.collapsed_project_keys.insert("node:root".to_string());

        assert!(app.node_folded("node:root"), "the legacy fold is honoured");

        assert!(app.unfold_node("node:root"), "unfolding reports the change");
        assert!(!app.node_folded("node:root"));
        assert!(
            !app.collapsed_project_keys.contains("node:root"),
            "the legacy record is withdrawn so it cannot re-fold every screen"
        );

        app.fold_node("node:root".to_string());
        assert!(app.node_folded("node:root"), "the new fold lives forward");
        assert!(
            !app.collapsed_project_keys.contains("node:root"),
            "and never writes the session-wide set again"
        );
    }

    // Folding an ancestor keeps the checkout the user is standing in — the
    // same promise a folded module and a folded project already make.
    #[test]
    fn folding_a_node_keeps_the_active_checkout_visible() {
        let mut app = app_with_node_chain();
        app.active = Some(1);
        app.selected = 1;
        app.fold_node("node:root".to_string());

        let entries = workspace_list_entries(&app);
        assert!(
            entries
                .iter()
                .any(|entry| matches!(entry, WorkspaceListEntry::Workspace { ws_idx: 1, .. })),
            "the active checkout survives the fold: {entries:?}"
        );
        assert!(
            !entries
                .iter()
                .any(|entry| matches!(entry, WorkspaceListEntry::Workspace { ws_idx: 0, .. })),
            "its siblings do not: {entries:?}"
        );
    }

    // K10: indentation follows the chain and stops growing at six, so a deep
    // tree cannot starve a 76-column phone of its name space.
    #[test]
    fn node_depth_walks_the_chain_and_stops_growing_at_six() {
        let mut app = app_with_node_chain();
        let mut nodes = vec![node_over("n0", None)];
        for i in 1..=8 {
            nodes.push(node_over(&format!("n{i}"), Some(&format!("n{}", i - 1))));
        }
        app.space_nodes = nodes;

        assert_eq!(node_depth(&app, "n0"), 0);
        assert_eq!(node_depth(&app, "n3"), 3);
        assert_eq!(node_depth(&app, "n8"), 6, "the visual depth is capped");
    }

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
    // same sentence the active tab and the active agent card speak. In this
    // fixture no chat is resumed by the active tab, so the accent stays on
    // the card and no chat row takes it; when one is, the accent descends to
    // that single row instead (TP-FOCUS-01) — one carrier either way.
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

    // TP-FOCUS-01: the accent marks the deepest visible focus object. With
    // the active tab's chat shown in the open drawer, the chat row wears the
    // accent and the card steps back to the quiet active tone.
    #[test]
    fn the_accent_descends_to_the_visible_active_chat_row() {
        let (mut app, key) = app_with_chat_drawer(2);
        app.expanded_chat_workspaces.insert(key);
        app.workspaces[0].tabs[0].resumed_session_id = Some("session-0".to_string());

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

        let card = app
            .view
            .workspace_card_areas
            .first()
            .expect("the workspace card is laid out");
        assert!(
            (card.rect.x..card.rect.x + card.rect.width)
                .all(|x| buffer[(x, card.rect.y)].bg != accent),
            "the card gives the accent up while its chat carries it"
        );
        let chat = app
            .view
            .workspace_chat_row_areas
            .iter()
            .find(|row| row.chat_idx == 0)
            .expect("the resumed chat row is laid out");
        assert!(
            (chat.rect.x..chat.rect.x + chat.rect.width)
                .any(|x| buffer[(x, chat.rect.y)].bg == accent),
            "the focused chat row wears the accent"
        );
    }

    // TP-FOCUS-04: when the accent descends to the chat row, the card's
    // trailing chrome — the "+" and the chat count — steps back with the
    // name. Contrast ink belongs on the accent; left behind on the quiet
    // active tone it reads as a broken highlight in the row's corner.
    #[test]
    fn the_plus_and_count_follow_the_card_off_the_accent() {
        let (mut app, key) = app_with_chat_drawer(2);
        app.expanded_chat_workspaces.insert(key);
        app.workspaces[0].tabs[0].resumed_session_id = Some("session-0".to_string());
        app.mouse_capture = true;

        let area = Rect::new(0, 0, 100, 26);
        crate::ui::compute_view(&mut app, area);
        let backend = ratatui::backend::TestBackend::new(100, 26);
        let mut terminal = ratatui::Terminal::new(backend).expect("test terminal");
        let registry = TerminalRuntimeRegistry::new();
        terminal
            .draw(|frame| render_sidebar(&app, &registry, frame, app.view.sidebar_rect))
            .expect("draw");
        let buffer = terminal.backend().buffer();

        let card = app
            .view
            .workspace_card_areas
            .first()
            .expect("the workspace card is laid out");
        let plus = workspace_new_chat_cell(card.rect);
        let plus_cell = &buffer[(plus.x, plus.y)];
        assert_eq!(plus_cell.symbol(), "+", "the plus is drawn in its cell");
        assert_eq!(
            plus_cell.fg, app.palette.text,
            "off the accent the plus wears the text ink, not the contrast ink"
        );

        // The count sits one column past the chrome trio ("⋯", a breathing
        // cell, then "+").
        let badge_x = card.rect.x + card.rect.width - 5;
        let badge_cell = &buffer[(badge_x, card.rect.y)];
        assert_eq!(badge_cell.symbol(), "2", "the chat count is drawn");
        assert_eq!(
            badge_cell.fg, app.palette.text,
            "off the accent the count wears the text ink too"
        );
    }

    // The companion invariant: while the card itself wears the accent, the
    // plus keeps the contrast ink — that pairing was always right.
    #[test]
    fn the_plus_keeps_the_contrast_ink_on_the_accent() {
        let (mut app, _key) = app_with_chat_drawer(2);
        app.mouse_capture = true;

        let area = Rect::new(0, 0, 100, 26);
        crate::ui::compute_view(&mut app, area);
        let backend = ratatui::backend::TestBackend::new(100, 26);
        let mut terminal = ratatui::Terminal::new(backend).expect("test terminal");
        let registry = TerminalRuntimeRegistry::new();
        terminal
            .draw(|frame| render_sidebar(&app, &registry, frame, app.view.sidebar_rect))
            .expect("draw");
        let buffer = terminal.backend().buffer();

        let card = app
            .view
            .workspace_card_areas
            .first()
            .expect("the workspace card is laid out");
        let plus = workspace_new_chat_cell(card.rect);
        let plus_cell = &buffer[(plus.x, plus.y)];
        assert_eq!(plus_cell.symbol(), "+", "the plus is drawn in its cell");
        assert_eq!(
            plus_cell.fg,
            panel_contrast_fg(&app.palette),
            "on the accent the plus keeps the contrast ink"
        );
    }

    // TP-FOCUS-02: with the drawer shut that chat row does not exist on
    // screen, so the card keeps the accent — an invisible selection would be
    // worse than a coarse one.
    #[test]
    fn the_card_keeps_the_accent_while_its_chat_is_hidden() {
        let (mut app, _key) = app_with_chat_drawer(2);
        app.workspaces[0].tabs[0].resumed_session_id = Some("session-0".to_string());

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

        let card = app
            .view
            .workspace_card_areas
            .first()
            .expect("the workspace card is laid out");
        assert!(
            (card.rect.x..card.rect.x + card.rect.width)
                .all(|x| buffer[(x, card.rect.y)].bg == accent),
            "the card keeps the accent while no chat row is visible"
        );
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
        let (cards, _, group_headers, _, _, _, _, _) = compute_workspace_list_areas(&app, area);
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

        let (cards, chat_rows, group_headers, _, _, _, _, _) =
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

        let (spacious, _, _headers, _, _, _, _, _) =
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
        let (packed, _, _headers, _, _, _, _, _) =
            compute_workspace_list_areas(&app, Rect::new(0, 0, 30, 30));
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
        let list_area = workspace_list_rect(area, app.sidebar_section_split, app.sidebar_chrome);
        let indicator_row = workspace_drop_indicator_row(
            &app.view.workspace_card_areas,
            list_area,
            2,
            app.sidebar_chrome,
        )
        .unwrap();
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

        let (cards, headers, _headers, _, _, _, _, _) = compute_workspace_list_areas(&app, area);

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

        let (cards, headers, _headers, _, _, _, _, _) =
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

#[cfg(test)]
mod module_gap_tests {
    use super::{module_branch_source, ModuleBranchSource};
    use crate::app::state::AppState;

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("herdr-modgap-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch directory");
        dir
    }

    fn state_with_node(key: &str, dir: Option<std::path::PathBuf>) -> AppState {
        let mut app = AppState::test_new();
        app.space_nodes = vec![crate::spaces::SpaceNode {
            key: key.to_string(),
            name: key.to_string(),
            icon: None,
            parent: None,
            dir,
        }];
        app
    }

    fn state_with_bucket(key: &str, repo_root: std::path::PathBuf) -> AppState {
        let mut app = AppState::test_new();
        app.space_split_rules = vec![crate::spaces::SpaceSplitRule {
            repo_root,
            patterns: vec!["*".to_string()],
            key: key.to_string(),
            label: format!("{key} label"),
            icon: None,
            parent: None,
        }];
        app
    }

    // M1.8 / TP-MOD-36: a node states its directory and that is the answer.
    #[test]
    fn a_node_with_a_directory_answers_with_it() {
        let dir = scratch("node-dir");
        let app = state_with_node("mod", Some(dir.clone()));

        assert_eq!(app.module_directory_for_key("mod"), Some(dir));
    }

    // M1.9 / TP-MOD-36: a bucket's directory is its repo root — also a fact the
    // person wrote down, in the rule itself. Twenty of the twenty-four modules
    // on the reporting machine are buckets, so a definition that skipped them
    // would answer "no directory" for most of that tree.
    #[test]
    fn a_bucket_answers_with_its_repository_root() {
        let repo = scratch("bucket-repo");
        let app = state_with_bucket("bucket", repo.clone());

        assert_eq!(app.module_directory_for_key("bucket"), Some(repo));
    }

    // M1.10 / TP-MOD-36: nothing stated, nothing invented. #46 measured where
    // guessed directories land.
    #[test]
    fn a_module_with_nothing_stated_has_no_directory() {
        let app = state_with_node("mod", None);

        assert_eq!(app.module_directory_for_key("mod"), None);
        assert_eq!(app.module_directory_for_key("nobody"), None);
    }

    // M2.2 / TP-MOD-37: the module's own directory is a repository root, so it
    // is somewhere to branch from.
    #[test]
    fn a_module_standing_in_a_repository_root_can_branch() {
        let repo = scratch("repo-root");
        std::fs::create_dir_all(repo.join(".git")).expect("repo marker");
        let app = state_with_node("mod", Some(repo.clone()));

        assert_eq!(
            module_branch_source(&app, "mod"),
            ModuleBranchSource::Repository(repo)
        );
    }

    // M2.3 🔴 / TP-MOD-37: THE guard. Measured on the reporting machine:
    //
    //   /home/user/Marktplaats satis          exists, empty, no .git
    //   git -C "..." rev-parse --show-toplevel  →  /home/user
    //
    // `$HOME` is itself a git repository there. If this answered `Repository`,
    // "New branch..." on that module would open branches and worktrees over the
    // whole home directory — `~/.claude`, `~/.config`, every project in it.
    // This test failing is that defect.
    #[test]
    fn a_directory_inside_a_repository_is_not_itself_a_branch_source() {
        let root = scratch("outer-repo");
        std::fs::create_dir_all(root.join(".git")).expect("repo marker");
        let inner = root.join("inner");
        std::fs::create_dir_all(&inner).expect("inner directory");
        let app = state_with_node("mod", Some(inner.clone()));

        assert_eq!(
            module_branch_source(&app, "mod"),
            ModuleBranchSource::UninitializedDirectory(inner),
            "only the directory the person named counts, and only if it is a root"
        );
    }

    // M2.4 / TP-MOD-37: nothing stated — the menu needs to know so it can point
    // at "Set directory..." instead of at a dead end.
    #[test]
    fn a_module_with_no_directory_has_no_branch_source() {
        let app = state_with_node("mod", None);

        assert_eq!(
            module_branch_source(&app, "mod"),
            ModuleBranchSource::NoDirectory
        );
    }

    // M2.5 / TP-MOD-37: stated once, gone since. Re-measured at use, the same
    // toll TP-CHAT-MOVE-10 (R3) already pays on the chat side.
    #[test]
    fn a_module_whose_directory_has_gone_has_no_branch_source() {
        let gone = std::env::temp_dir().join("herdr-modgap-never-existed");
        let app = state_with_node("mod", Some(gone));

        assert_eq!(
            module_branch_source(&app, "mod"),
            ModuleBranchSource::NoDirectory
        );
    }

    // M2.10 / TP-MOD-37: a bucket branches from the repository its rule names.
    #[test]
    fn a_bucket_branches_from_the_repository_its_rule_names() {
        let repo = scratch("bucket-branch");
        std::fs::create_dir_all(repo.join(".git")).expect("repo marker");
        let app = state_with_bucket("bucket", repo.clone());

        assert_eq!(
            module_branch_source(&app, "bucket"),
            ModuleBranchSource::Repository(repo)
        );
    }

    // M1.1 + M1.2 / TP-CHAT-MOVE-11: nodes AND buckets are destinations, each
    // keyed by `module:<key>`.
    #[test]
    fn nodes_and_buckets_are_both_module_destinations() {
        let repo = scratch("targets-repo");
        let dir = scratch("targets-node");
        let mut app = state_with_node("node-mod", Some(dir));
        app.space_split_rules = vec![crate::spaces::SpaceSplitRule {
            repo_root: repo,
            patterns: vec!["*".to_string()],
            key: "bucket-mod".to_string(),
            label: "Bucket Mod".to_string(),
            icon: None,
            parent: None,
        }];

        let keys: Vec<String> = app
            .module_move_target_entries()
            .into_iter()
            .map(|(key, _)| key)
            .collect();

        assert!(keys.contains(&"module:node-mod".to_string()));
        assert!(
            keys.contains(&"module:bucket-mod".to_string()),
            "buckets are twenty of this person's twenty-four modules"
        );
    }

    // M1.3 / TP-CHAT-MOVE-11: the module list holds modules only. Two verbs
    // that offer the same list are one verb wearing two names.
    #[test]
    fn the_module_list_never_offers_a_workspace() {
        let dir = scratch("modules-only");
        let app = state_with_node("mod", Some(dir));

        assert!(
            app.module_move_target_entries()
                .iter()
                .all(|(key, _)| key.starts_with("module:")),
            "every entry is a module identity, never a directory key"
        );
    }

    // M1.4 / TP-CHAT-MOVE-11: a module with a directory can actually receive a
    // chat and reopen it; one without cannot (TP-CHAT-MOVE-07). Offering the
    // dead end first would point at the option that does not work.
    #[test]
    fn modules_with_a_directory_are_offered_first() {
        let dir = scratch("ordered");
        let mut app = AppState::test_new();
        app.space_nodes = vec![
            crate::spaces::SpaceNode {
                key: "no-dir".to_string(),
                name: "No Dir".to_string(),
                icon: None,
                parent: None,
                dir: None,
            },
            crate::spaces::SpaceNode {
                key: "has-dir".to_string(),
                name: "Has Dir".to_string(),
                icon: None,
                parent: None,
                dir: Some(dir),
            },
        ];

        let keys: Vec<String> = app
            .module_move_target_entries()
            .into_iter()
            .map(|(key, _)| key)
            .collect();

        assert_eq!(
            keys.first().map(String::as_str),
            Some("module:has-dir"),
            "the first option must be one that works"
        );
    }

    // TP-CHAT-MOVE-11: one row per module, not per rule. The reporting config
    // gives `herdr:web` two `[[spaces.split]]` rules; listing it twice would
    // read as two modules with the same name.
    #[test]
    fn a_module_claimed_by_two_rules_is_listed_once() {
        let repo = scratch("dup-rules");
        let mut app = AppState::test_new();
        app.space_nodes.clear();
        app.space_split_rules = vec![
            crate::spaces::SpaceSplitRule {
                repo_root: repo.clone(),
                patterns: vec!["a*".to_string()],
                key: "same".to_string(),
                label: "Same".to_string(),
                icon: None,
                parent: None,
            },
            crate::spaces::SpaceSplitRule {
                repo_root: repo,
                patterns: vec!["b*".to_string()],
                key: "same".to_string(),
                label: "Same".to_string(),
                icon: None,
                parent: None,
            },
        ];

        let entries = app.module_move_target_entries();
        assert_eq!(entries.len(), 1, "one module, one row");
    }
}
