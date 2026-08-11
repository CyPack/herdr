use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

use ratatui::layout::Direction;
use serde::{Deserialize, Serialize};

use crate::layout::Node;
use crate::terminal::TerminalRuntimeRegistry;
use crate::ui::shell::{
    shell_persistence_parts, validate_persisted_shell_parts, ComponentPlacement, RegionId,
    ShellNode, ShellPresentationState, ShellTemplateId, TrackPolicy,
};
use crate::workspace::Workspace;

/// Current snapshot format version.
pub(super) const SNAPSHOT_VERSION: u32 = 4;

/// Version 1 wrote a tree the app had never drawn.
///
/// Its writer hardcoded `DockSidebarStage` and that template's root and tracks
/// while production ran the legacy `sidebar | stage` tree with no tracks at
/// all, so every version-1 file claims an AppDock column that was never on
/// screen. Version 2 records whatever the derivation actually produced.
///
/// The number is what makes the two distinguishable: once a tree can be chosen,
/// a file that says `DockSidebarStage` may be telling the truth, and there is no
/// other way to tell that file from today's fabricated one.
const SHELL_SNAPSHOT_VERSION: u16 = 2;
const FABRICATED_TREE_SHELL_SNAPSHOT_VERSION: u16 = 1;
const LEGACY_LEFT_PANEL_MIN_WIDTH: u16 = 4;
const LEGACY_LEFT_PANEL_DEFAULT_WIDTH: u16 = 26;
const LEGACY_LEFT_PANEL_MAX_WIDTH: u16 = 40;

/// Versioned, client-local shell preferences embedded in a session snapshot.
///
/// SF3.3 grows this bounded DTO test-first. It deliberately excludes runtime,
/// focus, hover, capture, computed geometry, and worker state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShellSnapshotV1 {
    pub(crate) schema_version: u16,
    /// The tree the app was presenting. `None` is the legacy desktop tree,
    /// mirroring the derivation's own vocabulary; it is also what a migrated
    /// version-1 file becomes, because that file's template claim was invented
    /// by the writer rather than observed.
    ///
    /// Omitted when absent so a legacy session file stays small and reads as
    /// "no template", not as "template: null".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) template: Option<ShellTemplateId>,
    pub(crate) root: ShellNode,
    #[serde(default)]
    pub(crate) region_constraints: BTreeMap<RegionId, TrackPolicy>,
    #[serde(default)]
    pub(crate) component_placements: Vec<ComponentPlacement>,
    #[serde(default)]
    pub(crate) collapse_restore_widths: BTreeMap<RegionId, u16>,
    #[serde(default)]
    pub(crate) pinned_dock_order: Vec<PinnedBuiltinAppV1>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum PinnedBuiltinAppV1 {
    Terminal,
    Files,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RestoredLeftPanelPreference {
    pub(crate) width: u16,
    pub(crate) collapsed: bool,
}

impl ShellSnapshotV1 {
    fn from_legacy_sidebar_width(sidebar_width: Option<u16>) -> Self {
        Self::from_left_panel_preference(
            sidebar_width.unwrap_or(LEGACY_LEFT_PANEL_DEFAULT_WIDTH),
            false,
        )
    }

    fn from_presentation(sidebar_width: u16, shell_presentation: &ShellPresentationState) -> Self {
        let collapsed = shell_presentation.left_panel_collapsed();
        let width = if collapsed {
            shell_presentation.left_panel_restore_width()
        } else {
            sidebar_width
        };
        Self::from_presented_tree(shell_presentation.shell_template(), width, collapsed)
    }

    fn from_left_panel_preference(width: u16, collapsed: bool) -> Self {
        Self::from_presented_tree(None, width, collapsed)
    }

    /// Record the tree the app is presenting, plus the one preference the user
    /// sets by hand.
    ///
    /// The tree comes from the same derivation the screen came from, so the
    /// file can no longer name a composition nobody drew. The left panel entry
    /// is written even when the presented tree has no such region: it is the
    /// person's preference, and dropping it because today's tree hides the
    /// panel would lose it the moment they switch back.
    fn from_presented_tree(template: Option<ShellTemplateId>, width: u16, collapsed: bool) -> Self {
        let preferred = width.clamp(LEGACY_LEFT_PANEL_MIN_WIDTH, LEGACY_LEFT_PANEL_MAX_WIDTH);
        let parts = shell_persistence_parts(template);
        let mut region_constraints = parts.region_constraints;
        region_constraints.insert(
            RegionId::LeftPanel,
            if collapsed {
                TrackPolicy::Collapsed { restore: preferred }
            } else {
                TrackPolicy::Resizable {
                    min: LEGACY_LEFT_PANEL_MIN_WIDTH,
                    preferred,
                    max: LEGACY_LEFT_PANEL_MAX_WIDTH,
                }
            },
        );
        Self {
            schema_version: SHELL_SNAPSHOT_VERSION,
            template: parts.template,
            root: parts.root,
            region_constraints,
            component_placements: parts.component_placements,
            collapse_restore_widths: BTreeMap::from([(RegionId::LeftPanel, preferred)]),
            pinned_dock_order: vec![PinnedBuiltinAppV1::Terminal, PinnedBuiltinAppV1::Files],
        }
    }

    /// Keep the half of a version-1 file that was ever true.
    ///
    /// The left panel width and collapse state came from the running app. The
    /// template, root, tracks and placements were invented by the writer, so
    /// applying them on upgrade would silently add a column to a composition
    /// the layout lock froze.
    fn migrated_from_fabricated_tree(&self) -> Self {
        let preference = self.restored_left_panel_preference();
        Self::from_presented_tree(
            None,
            preference.map_or(LEGACY_LEFT_PANEL_DEFAULT_WIDTH, |value| value.width),
            preference.is_some_and(|value| value.collapsed),
        )
    }

    fn from_value(value: serde_json::Value) -> Result<Self, String> {
        let snapshot = serde_json::from_value::<Self>(value).map_err(|error| error.to_string())?;
        let snapshot = match snapshot.schema_version {
            SHELL_SNAPSHOT_VERSION => snapshot,
            // A newer herdr wrote this. Refusing is the whole point of the
            // version: guessing at a shape we do not know would put a tree on
            // screen that nobody has drawn. The caller keeps the session and
            // falls back to compatibility preferences.
            version if version > SHELL_SNAPSHOT_VERSION => {
                return Err(format!("shell snapshot version {version} is unsupported"))
            }
            FABRICATED_TREE_SHELL_SNAPSHOT_VERSION => snapshot.migrated_from_fabricated_tree(),
            version => return Err(format!("shell snapshot version {version} is unsupported")),
        };
        validate_persisted_shell_parts(
            &snapshot.root,
            &snapshot.region_constraints,
            &snapshot.component_placements,
        )?;
        Ok(snapshot)
    }

    fn restored_left_panel_preference(&self) -> Option<RestoredLeftPanelPreference> {
        let retained = self
            .collapse_restore_widths
            .get(&RegionId::LeftPanel)
            .copied();
        let (width, collapsed) = match self.region_constraints.get(&RegionId::LeftPanel)? {
            TrackPolicy::Fixed { cells } => (*cells, false),
            TrackPolicy::ContentBounded { min, max } => {
                (retained.unwrap_or(*min).clamp(*min, *max), false)
            }
            TrackPolicy::Resizable {
                min,
                preferred,
                max,
            } => ((*preferred).clamp(*min, *max), false),
            TrackPolicy::Collapsed { restore } => (retained.unwrap_or(*restore), true),
            TrackPolicy::Fill { .. } => (retained?, false),
        };
        Some(RestoredLeftPanelPreference { width, collapsed })
    }
}

/// Serializable snapshot of the entire herdr session.
#[derive(Serialize, Deserialize)]
pub struct SessionSnapshot {
    /// Format version — used to detect incompatible changes.
    #[serde(default)]
    pub version: u32,
    pub workspaces: Vec<WorkspaceSnapshot>,
    pub active: Option<usize>,
    pub selected: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shell: Option<ShellSnapshotV1>,
    #[serde(default)]
    pub sidebar_width: Option<u16>,
    #[serde(default)]
    pub sidebar_section_split: Option<f32>,
    #[serde(default)]
    pub collapsed_space_keys: std::collections::HashSet<String>,
    /// Folded `[[spaces.project]]` headers. Defaulted so session files written
    /// before projects existed keep loading (TP-PROJ-PERS-01).
    #[serde(default)]
    pub collapsed_project_keys: std::collections::HashSet<String>,
    /// The Files tab left open at save time, if any.
    ///
    /// Optional and defaulted so session files written before the tab existed
    /// keep loading, and so a file written by a build that has it stays
    /// readable by one that does not.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub files_tab: Option<FilesTabSnapshot>,
}

/// A Files tab as recorded in the session file.
///
/// Deliberately self-describing rather than a reference into the workspace
/// tree: the tab is a stage app, so it is not owned by any `TabSnapshot`, and
/// keeping it standalone is what let restart survival land without touching
/// `workspace::Tab` or the wire protocol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilesTabSnapshot {
    /// Exact directory the tab was left in.
    pub cwd: std::path::PathBuf,
    /// Whether it owned the stage at save time.
    #[serde(default)]
    pub active: bool,
}

impl SessionSnapshot {
    pub(crate) fn restored_left_panel_preference(&self) -> Option<RestoredLeftPanelPreference> {
        self.shell
            .as_ref()
            .and_then(ShellSnapshotV1::restored_left_panel_preference)
            .or_else(|| {
                self.sidebar_width.map(|width| RestoredLeftPanelPreference {
                    width,
                    collapsed: false,
                })
            })
    }

    /// The shell tree this session was last presenting.
    ///
    /// `None` is the legacy desktop tree — which is what a session file without
    /// a shell block, and every migrated version-1 file, resolves to.
    pub(crate) fn restored_shell_template(&self) -> Option<ShellTemplateId> {
        self.shell.as_ref().and_then(|shell| shell.template)
    }
}

#[derive(Serialize, Deserialize)]
pub struct SessionHistorySnapshot {
    /// Format version follows the matching session snapshot version.
    #[serde(default)]
    pub version: u32,
    pub workspaces: Vec<WorkspaceHistorySnapshot>,
}

#[derive(Serialize, Deserialize)]
pub struct WorkspaceHistorySnapshot {
    pub tabs: Vec<TabHistorySnapshot>,
}

#[derive(Serialize, Deserialize)]
pub struct TabHistorySnapshot {
    pub panes: HashMap<u32, PaneHistorySnapshot>,
}

#[derive(Serialize, Deserialize)]
pub struct WorkspaceSnapshot {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub custom_name: Option<String>,
    pub identity_cwd: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worktree_space: Option<crate::workspace::WorktreeSpaceMembership>,
    #[serde(default)]
    pub public_pane_numbers: HashMap<u32, usize>,
    #[serde(default)]
    pub next_public_pane_number: usize,
    #[serde(default)]
    pub public_tab_numbers: Vec<usize>,
    #[serde(default)]
    pub next_public_tab_number: usize,
    pub tabs: Vec<TabSnapshot>,
    #[serde(default)]
    pub active_tab: usize,
}

#[derive(Deserialize)]
struct LegacyWorkspaceSnapshot {
    #[serde(default)]
    custom_name: Option<String>,
    layout: LayoutSnapshot,
    panes: HashMap<u32, PaneSnapshot>,
    zoomed: bool,
    #[serde(default)]
    focused: Option<u32>,
    #[serde(default)]
    root_pane: Option<u32>,
}

#[derive(Serialize, Deserialize)]
pub struct TabSnapshot {
    #[serde(default)]
    pub custom_name: Option<String>,
    pub layout: LayoutSnapshot,
    pub panes: HashMap<u32, PaneSnapshot>,
    pub zoomed: bool,
    #[serde(default)]
    pub focused: Option<u32>,
    #[serde(default)]
    pub root_pane: Option<u32>,
}

#[derive(Serialize, Deserialize)]
pub struct PaneSnapshot {
    pub cwd: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub managed_agent_kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_session: Option<PaneAgentSessionSnapshot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub launch_argv: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaneAgentSessionSnapshot {
    pub source: String,
    pub agent: String,
    pub kind: crate::agent_resume::AgentSessionRefKind,
    pub value: String,
}

#[derive(Serialize, Deserialize)]
pub struct PaneHistorySnapshot {
    pub ansi: String,
    pub lines: usize,
}

/// Serializable BSP tree.
#[derive(Serialize, Deserialize)]
pub enum LayoutSnapshot {
    Pane(u32),
    Split {
        direction: DirectionSnapshot,
        ratio: f32,
        first: Box<LayoutSnapshot>,
        second: Box<LayoutSnapshot>,
    },
}

#[derive(Serialize, Deserialize)]
pub enum DirectionSnapshot {
    Horizontal,
    Vertical,
}

impl From<LegacyWorkspaceSnapshot> for WorkspaceSnapshot {
    fn from(snap: LegacyWorkspaceSnapshot) -> Self {
        let identity_cwd = legacy_identity_cwd(&snap);
        let tab = TabSnapshot {
            custom_name: None,
            layout: snap.layout,
            panes: snap.panes,
            zoomed: snap.zoomed,
            focused: snap.focused,
            root_pane: snap.root_pane,
        };

        Self {
            id: None,
            custom_name: snap.custom_name,
            identity_cwd,
            worktree_space: None,
            public_pane_numbers: HashMap::new(),
            next_public_pane_number: 0,
            public_tab_numbers: Vec::new(),
            next_public_tab_number: 0,
            tabs: vec![tab],
            active_tab: 0,
        }
    }
}

#[derive(Deserialize)]
struct RawSessionSnapshot {
    #[serde(default)]
    version: u32,
    #[serde(default)]
    workspaces: Vec<serde_json::Value>,
    #[serde(default)]
    active: Option<usize>,
    #[serde(default)]
    selected: usize,
    #[serde(default)]
    shell: Option<serde_json::Value>,
    #[serde(default)]
    sidebar_width: Option<u16>,
    #[serde(default)]
    sidebar_section_split: Option<f32>,
    #[serde(default)]
    collapsed_space_keys: std::collections::HashSet<String>,
    #[serde(default)]
    collapsed_project_keys: std::collections::HashSet<String>,
    #[serde(default)]
    files_tab: Option<FilesTabSnapshot>,
}

fn migrate_snapshot(raw: RawSessionSnapshot) -> Result<SessionSnapshot, String> {
    let shell = if raw.version == SNAPSHOT_VERSION {
        match raw.shell {
            Some(value) => Some(match ShellSnapshotV1::from_value(value) {
                Ok(snapshot) => snapshot,
                Err(error) => {
                    tracing::warn!(
                        error = %error,
                        "invalid shell snapshot; using compatibility preferences"
                    );
                    ShellSnapshotV1::from_legacy_sidebar_width(raw.sidebar_width)
                }
            }),
            None => Some(ShellSnapshotV1::from_legacy_sidebar_width(
                raw.sidebar_width,
            )),
        }
    } else {
        Some(ShellSnapshotV1::from_legacy_sidebar_width(
            raw.sidebar_width,
        ))
    };
    Ok(SessionSnapshot {
        version: raw.version,
        workspaces: raw
            .workspaces
            .into_iter()
            .map(migrate_workspace)
            .collect::<Result<Vec<_>, _>>()?,
        active: raw.active,
        selected: raw.selected,
        shell,
        sidebar_width: raw.sidebar_width,
        sidebar_section_split: raw.sidebar_section_split,
        collapsed_space_keys: raw.collapsed_space_keys,
        collapsed_project_keys: raw.collapsed_project_keys,
        files_tab: raw.files_tab,
    })
}

fn migrate_workspace(raw: serde_json::Value) -> Result<WorkspaceSnapshot, String> {
    if raw.get("identity_cwd").is_some() {
        return serde_json::from_value(raw).map_err(|e| e.to_string());
    }

    if raw.get("layout").is_some() {
        let legacy =
            serde_json::from_value::<LegacyWorkspaceSnapshot>(raw).map_err(|e| e.to_string())?;
        return Ok(legacy.into());
    }

    Err("workspace snapshot is neither current nor legacy format".to_string())
}

fn legacy_identity_cwd(snap: &LegacyWorkspaceSnapshot) -> PathBuf {
    let root_pane = snap
        .root_pane
        .or_else(|| first_pane_id_in_layout(&snap.layout));

    root_pane
        .and_then(|pane_id| snap.panes.get(&pane_id))
        .map(|pane| pane.cwd.clone())
        .or_else(|| {
            first_pane_id_in_layout(&snap.layout)
                .and_then(|pane_id| snap.panes.get(&pane_id))
                .map(|pane| pane.cwd.clone())
        })
        .or_else(|| {
            snap.panes
                .keys()
                .min()
                .and_then(|pane_id| snap.panes.get(pane_id))
                .map(|pane| pane.cwd.clone())
        })
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| "/".into()))
}

fn first_pane_id_in_layout(layout: &LayoutSnapshot) -> Option<u32> {
    match layout {
        LayoutSnapshot::Pane(id) => Some(*id),
        LayoutSnapshot::Split { first, second, .. } => {
            first_pane_id_in_layout(first).or_else(|| first_pane_id_in_layout(second))
        }
    }
}

/// Capture the current app state into a serializable snapshot.
pub fn capture(
    workspaces: &[Workspace],
    terminals: &std::collections::HashMap<
        crate::terminal::TerminalId,
        crate::terminal::TerminalState,
    >,
    terminal_runtimes: &TerminalRuntimeRegistry,
    active: Option<usize>,
    selected: usize,
    sidebar_width: u16,
    shell_presentation: &ShellPresentationState,
    sidebar_section_split: f32,
    collapsed_space_keys: std::collections::HashSet<String>,
    collapsed_project_keys: std::collections::HashSet<String>,
    files_tab: Option<FilesTabSnapshot>,
) -> SessionSnapshot {
    SessionSnapshot {
        version: SNAPSHOT_VERSION,
        workspaces: workspaces
            .iter()
            .map(|workspace| capture_workspace(workspace, terminals, terminal_runtimes))
            .collect(),
        active,
        selected,
        shell: Some(ShellSnapshotV1::from_presentation(
            sidebar_width,
            shell_presentation,
        )),
        sidebar_width: Some(sidebar_width),
        sidebar_section_split: Some(sidebar_section_split),
        collapsed_space_keys,
        collapsed_project_keys,
        files_tab,
    }
}

fn capture_workspace(
    ws: &Workspace,
    terminals: &std::collections::HashMap<
        crate::terminal::TerminalId,
        crate::terminal::TerminalState,
    >,
    terminal_runtimes: &TerminalRuntimeRegistry,
) -> WorkspaceSnapshot {
    WorkspaceSnapshot {
        id: Some(ws.id.clone()),
        custom_name: ws.custom_name.clone(),
        identity_cwd: ws
            .resolved_identity_cwd_from(terminals, terminal_runtimes)
            .unwrap_or_else(|| ws.identity_cwd.clone()),
        worktree_space: ws.worktree_space.clone(),
        public_pane_numbers: ws
            .public_pane_numbers
            .iter()
            .map(|(pane_id, number)| (pane_id.raw(), *number))
            .collect(),
        next_public_pane_number: ws.next_public_pane_number,
        public_tab_numbers: ws.tabs.iter().map(|tab| tab.number).collect(),
        next_public_tab_number: ws.next_public_tab_number,
        tabs: ws
            .tabs
            .iter()
            .map(|tab| capture_tab(tab, terminals, terminal_runtimes))
            .collect(),
        active_tab: ws.default_tab(),
    }
}

fn capture_tab(
    tab: &crate::workspace::Tab,
    terminals: &std::collections::HashMap<
        crate::terminal::TerminalId,
        crate::terminal::TerminalState,
    >,
    terminal_runtimes: &TerminalRuntimeRegistry,
) -> TabSnapshot {
    let mut panes = HashMap::new();
    for id in tab.panes.keys() {
        let cwd = tab
            .cwd_for_pane(*id, terminals, terminal_runtimes)
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| "/".into()));
        let terminal = tab
            .panes
            .get(id)
            .and_then(|pane| terminals.get(&pane.attached_terminal_id));
        let label = terminal.and_then(|terminal| terminal.manual_label.clone());
        let (agent_name, managed_agent_kind) = terminal
            .filter(|terminal| !terminal.managed_agent_launch_pending())
            .map(|terminal| {
                (
                    terminal.agent_name.clone(),
                    terminal
                        .managed_agent_kind()
                        .map(|agent| crate::detect::agent_label(agent).to_string()),
                )
            })
            .unwrap_or_default();
        let launch_argv = terminal.and_then(|terminal| terminal.launch_argv.clone());
        let agent_session = terminal.and_then(|terminal| {
            if let Some(authority) = terminal.hook_authority.as_ref() {
                if let Some(session_ref) = authority.session_ref.as_ref() {
                    return Some(PaneAgentSessionSnapshot {
                        source: authority.source.clone(),
                        agent: authority.agent_label.clone(),
                        kind: session_ref.kind,
                        value: session_ref.value.clone(),
                    });
                }
            }
            terminal
                .persisted_agent_session
                .as_ref()
                .map(|session| PaneAgentSessionSnapshot {
                    source: session.source.clone(),
                    agent: session.agent.clone(),
                    kind: session.session_ref.kind,
                    value: session.session_ref.value.clone(),
                })
        });
        panes.insert(
            id.raw(),
            PaneSnapshot {
                cwd,
                label,
                agent_name,
                managed_agent_kind,
                agent_session,
                launch_argv,
            },
        );
    }
    TabSnapshot {
        custom_name: tab.custom_name.clone(),
        layout: capture_node(tab.layout.root()),
        panes,
        zoomed: tab.zoomed,
        focused: Some(tab.layout.focused().raw()),
        root_pane: Some(tab.root_pane.raw()),
    }
}

/// Capture pane screen history separately from the structural session snapshot.
pub fn capture_history(
    workspaces: &[Workspace],
    terminal_runtimes: &TerminalRuntimeRegistry,
) -> SessionHistorySnapshot {
    SessionHistorySnapshot {
        version: SNAPSHOT_VERSION,
        workspaces: workspaces
            .iter()
            .map(|workspace| WorkspaceHistorySnapshot {
                tabs: workspace
                    .tabs
                    .iter()
                    .map(|tab| TabHistorySnapshot {
                        panes: capture_tab_history(tab, terminal_runtimes),
                    })
                    .collect(),
            })
            .collect(),
    }
}

fn capture_tab_history(
    tab: &crate::workspace::Tab,
    terminal_runtimes: &TerminalRuntimeRegistry,
) -> HashMap<u32, PaneHistorySnapshot> {
    let mut panes = HashMap::new();
    for (id, pane) in &tab.panes {
        if let Some(history) = capture_pane_history(Some(pane), terminal_runtimes) {
            panes.insert(id.raw(), history);
        }
    }
    panes
}

fn capture_pane_history(
    pane: Option<&crate::pane::PaneState>,
    terminal_runtimes: &TerminalRuntimeRegistry,
) -> Option<PaneHistorySnapshot> {
    let ansi = terminal_runtimes
        .get(&pane?.attached_terminal_id)?
        .snapshot_history()?;
    let lines = ansi.lines().count();
    Some(PaneHistorySnapshot { ansi, lines })
}

pub(super) fn capture_node(node: &Node) -> LayoutSnapshot {
    match node {
        Node::Pane(id) => LayoutSnapshot::Pane(id.raw()),
        Node::Split {
            direction,
            ratio,
            first,
            second,
        } => LayoutSnapshot::Split {
            direction: match direction {
                Direction::Horizontal => DirectionSnapshot::Horizontal,
                Direction::Vertical => DirectionSnapshot::Vertical,
            },
            ratio: *ratio,
            first: Box::new(capture_node(first)),
            second: Box::new(capture_node(second)),
        },
    }
}

pub(super) fn parse_snapshot(content: &str) -> Result<SessionSnapshot, String> {
    let raw = serde_json::from_str::<RawSessionSnapshot>(content).map_err(|e| e.to_string())?;
    if raw.version > SNAPSHOT_VERSION {
        return Err(format!(
            "snapshot version {} is newer than supported {}",
            raw.version, SNAPSHOT_VERSION
        ));
    }
    migrate_snapshot(raw)
}

pub(super) fn parse_history_snapshot(content: &str) -> Result<SessionHistorySnapshot, String> {
    let snapshot =
        serde_json::from_str::<SessionHistorySnapshot>(content).map_err(|e| e.to_string())?;
    if snapshot.version > SNAPSHOT_VERSION {
        return Err(format!(
            "history snapshot version {} is newer than supported {}",
            snapshot.version, SNAPSHOT_VERSION
        ));
    }
    Ok(snapshot)
}

pub(super) fn snapshot_file_version(content: &str) -> Option<u32> {
    serde_json::from_str::<RawSessionSnapshot>(content)
        .ok()
        .map(|raw| raw.version)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::PathBuf;

    use ratatui::layout::{Direction, Position, Rect};

    use super::*;
    use crate::app::{AppState, Mode};
    use crate::layout::NavDirection;
    use crate::workspace::Workspace;

    // TP-FTAB-PERSIST-09: the promise the user actually made — close herdr with
    // a Files tab open, reopen it, and the tab is there in the same directory.
    // Capture and restore are covered separately elsewhere; only this one fails
    // if the two halves disagree about the file that passes between them.
    #[test]
    fn a_files_tab_survives_a_full_save_and_load_cycle() {
        struct FixtureRoot(PathBuf);
        impl Drop for FixtureRoot {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }

        let root = std::env::temp_dir().join(format!("herdr-persist-e2e-{}", std::process::id()));
        let _fixture = FixtureRoot(root.clone());
        std::fs::create_dir_all(&root).expect("fixture root");
        std::fs::write(root.join("00.txt"), b"x").expect("fixture entry");

        for left_active in [true, false] {
            let mut saved = AppState::test_new();
            saved.workspaces = vec![Workspace::test_new("persisted")];
            saved.active = Some(0);
            saved
                .try_open_file_manager_with(|_| Some(crate::fm::FmState::new(&root)))
                .expect("Files activation");
            if !left_active {
                saved.show_terminal_workspace();
            }

            let encoded =
                serde_json::to_string(&capture_from_state(&saved)).expect("serialize session");
            let reloaded = parse_snapshot(&encoded).expect("load session");

            let mut restored = AppState::test_new();
            restored.workspaces = vec![Workspace::test_new("persisted")];
            restored.active = Some(0);
            restored.restore_files_tab(
                reloaded
                    .files_tab
                    .as_ref()
                    .expect("the cycle must carry the Files tab"),
            );

            assert_eq!(
                restored
                    .file_manager
                    .as_ref()
                    .expect("restored Files tab")
                    .cwd,
                root,
                "left_active={left_active}"
            );
            assert_eq!(
                restored.stage.surface_view()
                    == crate::ui::surface_host::StageSurfaceView::NativeFiles,
                left_active,
                "left_active={left_active}: surface ownership must survive the cycle"
            );
        }
    }

    // TP-FTAB-PERSIST-04: session files written before the Files tab existed
    // must keep loading. A required field here would make every existing
    // session unreadable on the first run after an update.
    #[test]
    fn snapshots_written_before_the_files_tab_load_without_one() {
        for name in ["current-herdr", "current-herdr-dev", "legacy-pre-tabs-v2"] {
            let snapshot = parse_snapshot(session_fixture(name))
                .unwrap_or_else(|err| panic!("{name} must still load: {err}"));
            assert_eq!(
                snapshot.files_tab, None,
                "{name}: an absent field must read as no Files tab"
            );
        }
    }

    // TP-FTAB-PERSIST-05: the recorded directory and surface ownership survive
    // a serialization round trip. This pins the serde attributes themselves,
    // which are the part most easily broken by an unrelated edit.
    #[test]
    fn files_tab_snapshot_round_trips_through_json() {
        let mut snapshot = parse_snapshot(session_fixture("current-herdr")).expect("fixture");
        snapshot.files_tab = Some(FilesTabSnapshot {
            cwd: PathBuf::from("/tmp/herdr-persist-round-trip"),
            active: false,
        });

        let encoded = serde_json::to_string(&snapshot).expect("serialize");
        let decoded = parse_snapshot(&encoded).expect("deserialize");

        assert_eq!(decoded.files_tab, snapshot.files_tab);
    }

    fn session_fixture(name: &str) -> &'static str {
        match name {
            "current-herdr" => {
                include_str!("../../tests/fixtures/session/current-herdr-session.json")
            }
            "current-herdr-dev" => {
                include_str!("../../tests/fixtures/session/current-herdr-dev-session.json")
            }
            "legacy-pre-tabs-v2" => {
                include_str!("../../tests/fixtures/session/legacy-pre-tabs-v2.json")
            }
            other => panic!("unknown session fixture: {other}"),
        }
    }

    fn test_session_path(name: &str) -> String {
        std::env::current_dir()
            .unwrap()
            .join(name)
            .display()
            .to_string()
    }

    fn state_with_workspaces(names: &[&str]) -> AppState {
        let mut state = AppState::test_new();
        state.workspaces = names.iter().map(|name| Workspace::test_new(name)).collect();
        state.ensure_test_terminals();
        if !state.workspaces.is_empty() {
            state.active = Some(0);
            state.selected = 0;
            state.mode = Mode::Terminal;
        }
        state
    }

    fn current_workspace_json(id: &str) -> serde_json::Value {
        serde_json::json!({
            "id": id,
            "custom_name": "preserved",
            "identity_cwd": "/tmp/preserved",
            "tabs": [{
                "layout": { "Pane": 0 },
                "panes": {
                    "0": { "cwd": "/tmp/preserved" }
                },
                "zoomed": false,
                "focused": 0,
                "root_pane": 0
            }],
            "active_tab": 0
        })
    }

    /// A shell block exactly as the version-1 writer produced it: a
    /// `DockSidebarStage` tree, written by a build whose screen was the legacy
    /// tree. Kept verbatim because it is what is on every real disk today.
    fn valid_v4_shell_json() -> serde_json::Value {
        serde_json::json!({
            "schema_version": 1,
            "template": "DockSidebarStage",
            "root": {
                "Split": {
                    "direction": "Horizontal",
                    "children": [
                        {
                            "size": "Dynamic",
                            "node": { "Slot": { "region": "AppDock" } }
                        },
                        {
                            "size": "Dynamic",
                            "node": { "Slot": { "region": "LeftPanel" } }
                        },
                        {
                            "size": "Fill",
                            "node": { "Slot": { "region": "WorkspaceStage" } }
                        }
                    ]
                }
            },
            "region_constraints": {
                "AppDock": {
                    "Resizable": { "min": 3, "preferred": 5, "max": 9 }
                },
                "LeftPanel": {
                    "Resizable": { "min": 4, "preferred": 31, "max": 40 }
                },
                "WorkspaceStage": { "Fill": { "weight": 1 } }
            },
            "component_placements": [
                { "component": "AppDock", "region": "AppDock" },
                { "component": "AgentSidebar", "region": "LeftPanel" },
                { "component": "WorkspaceStage", "region": "WorkspaceStage" }
            ],
            "collapse_restore_widths": {
                "AppDock": 5,
                "LeftPanel": 31
            },
            "pinned_dock_order": ["Terminal", "Files"]
        })
    }

    /// The same block claimed at the current schema, i.e. a file whose tree the
    /// reader is meant to believe. Tree-shaped gates have to be exercised here:
    /// a version-1 tree never reaches validation any more, because it is
    /// discarded before it can.
    fn shell_json_at_current_schema() -> serde_json::Value {
        let mut shell = valid_v4_shell_json();
        shell["schema_version"] = serde_json::json!(SHELL_SNAPSHOT_VERSION);
        shell
    }

    fn v4_session_with_shell_json(
        shell: serde_json::Value,
        workspace_id: &str,
        sidebar_width: u16,
    ) -> serde_json::Value {
        serde_json::json!({
            "version": 4,
            "workspaces": [current_workspace_json(workspace_id)],
            "active": 0,
            "selected": 0,
            "shell": shell,
            "sidebar_width": sidebar_width,
            "sidebar_section_split": 0.4
        })
    }

    fn assert_compatible_shell_fallback(
        restored: &SessionSnapshot,
        workspace_id: &str,
        sidebar_width: u16,
    ) {
        assert_eq!(restored.workspaces.len(), 1);
        assert_eq!(restored.workspaces[0].id.as_deref(), Some(workspace_id));
        assert_eq!(restored.active, Some(0));
        assert_eq!(restored.selected, 0);
        assert_eq!(restored.sidebar_width, Some(sidebar_width));
        assert_eq!(restored.sidebar_section_split, Some(0.4));

        let encoded = serde_json::to_value(restored).unwrap();
        let expected_width = Some(u64::from(sidebar_width));
        assert_eq!(
            encoded
                .pointer("/shell/region_constraints/LeftPanel/Resizable/preferred")
                .and_then(serde_json::Value::as_u64),
            expected_width
        );
        assert_eq!(
            encoded
                .pointer("/shell/collapse_restore_widths/LeftPanel")
                .and_then(serde_json::Value::as_u64),
            expected_width
        );
    }

    fn restored_left_panel_width_for_test(snapshot: &SessionSnapshot) -> Option<u16> {
        snapshot
            .restored_left_panel_preference()
            .map(|preference| preference.width)
    }

    fn restored_left_panel_preference_for_test(snapshot: &SessionSnapshot) -> (Option<u16>, bool) {
        snapshot
            .restored_left_panel_preference()
            .map_or((None, false), |preference| {
                (Some(preference.width), preference.collapsed)
            })
    }

    fn capture_from_state(state: &AppState) -> SessionSnapshot {
        let terminal_runtimes = TerminalRuntimeRegistry::new();
        capture_from_state_with_runtimes(state, &terminal_runtimes)
    }

    fn capture_from_state_with_runtimes(
        state: &AppState,
        terminal_runtimes: &TerminalRuntimeRegistry,
    ) -> SessionSnapshot {
        capture(
            &state.workspaces,
            &state.terminals,
            terminal_runtimes,
            state.active,
            state.selected,
            state.sidebar_width,
            &state.shell_presentation,
            state.sidebar_section_split,
            state.collapsed_space_keys.clone(),
            state.collapsed_project_keys.clone(),
            state.files_tab_snapshot(),
        )
    }

    fn capture_history_from_state_with_runtimes(
        state: &AppState,
        terminal_runtimes: &TerminalRuntimeRegistry,
    ) -> SessionHistorySnapshot {
        capture_history(&state.workspaces, terminal_runtimes)
    }

    fn root_split_ratio(tab: &TabSnapshot) -> Option<f32> {
        match &tab.layout {
            LayoutSnapshot::Split { ratio, .. } => Some(*ratio),
            LayoutSnapshot::Pane(_) => None,
        }
    }

    #[test]
    fn managed_agent_snapshot_omits_pending_and_persists_active_ownership() {
        let mut state = state_with_workspaces(&["managed-snapshot"]);
        let root = state.workspaces[0].tabs[0].root_pane;
        let terminal_id = state.workspaces[0].tabs[0].panes[&root]
            .attached_terminal_id
            .clone();
        let now = std::time::Instant::now();
        state
            .terminals
            .get_mut(&terminal_id)
            .unwrap()
            .begin_managed_agent(
                "reviewer".into(),
                crate::detect::Agent::Pi,
                now,
                std::time::Duration::ZERO,
                std::time::Duration::from_secs(1),
            );

        let pending = capture_from_state(&state);
        let pending_pane = &pending.workspaces[0].tabs[0].panes[&root.raw()];
        assert_eq!(pending_pane.agent_name, None);
        assert_eq!(pending_pane.managed_agent_kind, None);

        let terminal = state.terminals.get_mut(&terminal_id).unwrap();
        terminal.set_detected_state(
            Some(crate::detect::Agent::Pi),
            crate::detect::AgentState::Idle,
        );
        assert!(terminal.reconcile_managed_agent_at(now, false));
        let active = capture_from_state(&state);
        let active_pane = &active.workspaces[0].tabs[0].panes[&root.raw()];
        assert_eq!(active_pane.agent_name.as_deref(), Some("reviewer"));
        assert_eq!(active_pane.managed_agent_kind.as_deref(), Some("pi"));
    }

    #[test]
    fn round_trip_empty_session() {
        let snap = SessionSnapshot {
            version: SNAPSHOT_VERSION,
            workspaces: vec![],
            active: None,
            selected: 0,
            shell: None,
            sidebar_width: Some(26),
            sidebar_section_split: Some(0.5),
            collapsed_space_keys: std::collections::HashSet::new(),
            collapsed_project_keys: std::collections::HashSet::new(),
            files_tab: None,
        };
        let json = serde_json::to_string(&snap).unwrap();
        let restored = parse_snapshot(&json).unwrap();
        assert!(restored.workspaces.is_empty());
        assert_eq!(restored.active, None);
        assert_eq!(restored.sidebar_width, Some(26));
        assert_eq!(restored.sidebar_section_split, Some(0.5));
    }

    #[test]
    fn round_trip_layout_snapshot() {
        let layout = LayoutSnapshot::Split {
            direction: DirectionSnapshot::Horizontal,
            ratio: 0.6,
            first: Box::new(LayoutSnapshot::Pane(0)),
            second: Box::new(LayoutSnapshot::Split {
                direction: DirectionSnapshot::Vertical,
                ratio: 0.5,
                first: Box::new(LayoutSnapshot::Pane(1)),
                second: Box::new(LayoutSnapshot::Pane(2)),
            }),
        };
        let json = serde_json::to_string(&layout).unwrap();
        let restored: LayoutSnapshot = serde_json::from_str(&json).unwrap();

        match restored {
            LayoutSnapshot::Split { ratio, .. } => assert!((ratio - 0.6).abs() < 0.01),
            _ => panic!("expected split"),
        }
    }

    #[test]
    fn round_trip_full_workspace_snapshot() {
        let mut panes = HashMap::new();
        panes.insert(
            0,
            PaneSnapshot {
                cwd: PathBuf::from("/home/can/Projects/herdr"),
                label: None,
                agent_name: None,
                managed_agent_kind: None,
                agent_session: None,
                launch_argv: None,
            },
        );
        panes.insert(
            1,
            PaneSnapshot {
                cwd: PathBuf::from("/home/can/Projects/website"),
                label: Some("website".into()),
                agent_name: None,
                managed_agent_kind: None,
                agent_session: None,
                launch_argv: None,
            },
        );

        let snap = SessionSnapshot {
            workspaces: vec![WorkspaceSnapshot {
                id: Some("wproj".to_string()),
                custom_name: Some("pi-mono".to_string()),
                identity_cwd: PathBuf::from("/home/can/Projects/herdr"),
                worktree_space: None,
                public_pane_numbers: HashMap::from([(0, 1), (1, 2)]),
                next_public_pane_number: 3,
                public_tab_numbers: vec![1],
                next_public_tab_number: 2,
                tabs: vec![TabSnapshot {
                    custom_name: Some("api".to_string()),
                    layout: LayoutSnapshot::Split {
                        direction: DirectionSnapshot::Horizontal,
                        ratio: 0.5,
                        first: Box::new(LayoutSnapshot::Pane(0)),
                        second: Box::new(LayoutSnapshot::Pane(1)),
                    },
                    panes,
                    zoomed: false,
                    focused: Some(0),
                    root_pane: Some(0),
                }],
                active_tab: 0,
            }],
            active: Some(0),
            selected: 0,
            shell: None,
            sidebar_width: Some(26),
            sidebar_section_split: Some(0.5),
            collapsed_space_keys: std::collections::HashSet::new(),
            collapsed_project_keys: std::collections::HashSet::new(),
            files_tab: None,
            version: SNAPSHOT_VERSION,
        };

        let json = serde_json::to_string_pretty(&snap).unwrap();
        let restored = parse_snapshot(&json).unwrap();

        assert_eq!(restored.workspaces.len(), 1);
        assert_eq!(restored.workspaces[0].id.as_deref(), Some("wproj"));
        assert_eq!(
            restored.workspaces[0].custom_name.as_deref(),
            Some("pi-mono")
        );
        assert_eq!(restored.workspaces[0].tabs.len(), 1);
        assert_eq!(restored.workspaces[0].tabs[0].panes.len(), 2);
        assert_eq!(
            restored.workspaces[0].tabs[0].panes[&0].cwd,
            PathBuf::from("/home/can/Projects/herdr")
        );
        assert_eq!(
            restored.workspaces[0].tabs[0].panes[&1].label.as_deref(),
            Some("website")
        );
        assert_eq!(restored.sidebar_width, Some(26));
        assert_eq!(restored.sidebar_section_split, Some(0.5));
    }

    #[test]
    fn current_session_fixture_parses() {
        let snap = parse_snapshot(session_fixture("current-herdr")).unwrap();

        assert_eq!(snap.version, 3);
        assert_eq!(snap.workspaces.len(), 2);
        assert_eq!(snap.active, Some(0));
        assert_eq!(snap.selected, 0);
        assert_eq!(snap.sidebar_width, None);
        assert_eq!(snap.sidebar_section_split, None);
        assert_eq!(snap.workspaces[0].tabs.len(), 2);
        assert_eq!(
            snap.workspaces[1].identity_cwd,
            PathBuf::from("/home/test/projects/project-b")
        );
    }

    #[test]
    fn current_dev_session_fixture_parses_additive_fields() {
        let snap = parse_snapshot(session_fixture("current-herdr-dev")).unwrap();

        assert_eq!(snap.version, 3);
        assert_eq!(snap.workspaces.len(), 2);
        assert_eq!(snap.sidebar_section_split, Some(0.4));
        assert_eq!(snap.workspaces[0].active_tab, 1);
        assert_eq!(snap.workspaces[1].tabs[0].panes.len(), 2);
    }

    #[test]
    fn v3_snapshot_migrates_sidebar_width_into_left_panel() {
        let json = serde_json::json!({
            "version": 3,
            "workspaces": [],
            "active": null,
            "selected": 0,
            "sidebar_width": 34,
            "sidebar_section_split": 0.4
        })
        .to_string();

        let restored = parse_snapshot(&json).unwrap();
        let encoded = serde_json::to_value(&restored).unwrap();
        let shell = encoded
            .get("shell")
            .expect("v3 migration must derive a bounded shell snapshot");

        assert_eq!(
            shell
                .pointer("/region_constraints/LeftPanel/Resizable/preferred")
                .and_then(serde_json::Value::as_u64),
            Some(34)
        );
        assert_eq!(
            shell
                .pointer("/collapse_restore_widths/LeftPanel")
                .and_then(serde_json::Value::as_u64),
            Some(34)
        );
    }

    #[test]
    fn v3_sidebar_section_split_remains_sidebar_owned() {
        let json = serde_json::json!({
            "version": 3,
            "workspaces": [],
            "active": null,
            "selected": 0,
            "sidebar_width": 34,
            "sidebar_section_split": 0.4
        })
        .to_string();

        let restored = parse_snapshot(&json).unwrap();
        assert_eq!(restored.sidebar_section_split, Some(0.4));

        let encoded = serde_json::to_value(&restored).unwrap();
        let shell = encoded
            .get("shell")
            .expect("v3 migration must derive a bounded shell snapshot");
        assert!(shell.get("sidebar_section_split").is_none());
    }

    #[test]
    fn v4_shell_round_trip_is_idempotent() {
        let shell = valid_v4_shell_json();
        let input = serde_json::json!({
            "version": 4,
            "workspaces": [],
            "active": null,
            "selected": 0,
            "shell": shell.clone(),
            "sidebar_width": 19,
            "sidebar_section_split": 0.4
        });

        let first = parse_snapshot(&input.to_string())
            .expect("snapshot v4 with a bounded shell must be supported");
        let first_value = serde_json::to_value(&first).unwrap();
        // A version-1 block is migrated on the way in, so the file that comes
        // back is at the current schema with the fabricated tree dropped. What
        // makes the round trip stable is the reader, not the file's own claim.
        assert_eq!(
            first_value["shell"]["schema_version"],
            serde_json::json!(SHELL_SNAPSHOT_VERSION)
        );
        assert_eq!(first_value["shell"].get("template"), None);
        assert_eq!(first.sidebar_width, Some(19));
        assert_eq!(first.sidebar_section_split, Some(0.4));

        let second = parse_snapshot(&first_value.to_string()).unwrap();
        let second_value = serde_json::to_value(&second).unwrap();
        assert_eq!(second_value, first_value);
    }

    // The other half of idempotency: a file already at the current schema comes
    // back byte for byte, because nothing rewrites a tree the reader believes.
    #[test]
    fn a_current_schema_shell_block_round_trips_verbatim() {
        let shell = shell_json_at_current_schema();
        let input = serde_json::json!({
            "version": 4,
            "workspaces": [],
            "active": null,
            "selected": 0,
            "shell": shell.clone(),
            "sidebar_width": 19,
            "sidebar_section_split": 0.4
        });

        let restored =
            parse_snapshot(&input.to_string()).expect("a current-schema shell must be supported");
        let value = serde_json::to_value(&restored).unwrap();

        assert_eq!(value["shell"], shell);
        assert_eq!(
            restored.restored_shell_template(),
            Some(ShellTemplateId::DockSidebarStage),
            "a believable file gets believed"
        );
    }

    // A corrupt version-1 tree is no longer a reason to lose the panel width:
    // the tree was going to be discarded either way, so the migration keeps the
    // half that was real instead of falling back over the half that was not.
    #[test]
    fn a_corrupt_version_one_tree_still_yields_its_panel_width() {
        let mut shell = valid_v4_shell_json();
        shell["root"] = serde_json::json!({
            "Split": {
                "direction": "Horizontal",
                "children": (0..9)
                    .map(|_| serde_json::json!({
                        "size": "Fill",
                        "node": { "Slot": { "region": "WorkspaceStage" } }
                    }))
                    .collect::<Vec<_>>()
            }
        });
        let input = v4_session_with_shell_json(shell, "w-corrupt-v1", 27);

        let restored = parse_snapshot(&input.to_string())
            .expect("a corrupt v1 tree must not discard valid session data");

        assert_eq!(restored.restored_shell_template(), None);
        assert_eq!(
            restored.restored_left_panel_preference(),
            Some(RestoredLeftPanelPreference {
                width: 31,
                collapsed: false
            })
        );
    }

    #[test]
    fn v4_shell_preference_is_authoritative_over_legacy_width() {
        let input = serde_json::json!({
            "version": 4,
            "workspaces": [],
            "active": null,
            "selected": 0,
            "shell": valid_v4_shell_json(),
            "sidebar_width": 19,
            "sidebar_section_split": 0.4
        });

        let restored = parse_snapshot(&input.to_string()).unwrap();

        assert_eq!(restored_left_panel_width_for_test(&restored), Some(31));
        assert_eq!(restored.sidebar_width, Some(19));
    }

    #[test]
    fn resize_preview_is_not_captured() {
        let mut state = state_with_workspaces(&["one"]);
        crate::ui::compute_view(&mut state, Rect::new(0, 0, 106, 40));
        assert!(state.begin_sidebar_resize(Position::new(25, 5)));
        assert!(state.preview_sidebar_resize(Position::new(31, 5)));
        assert_eq!(state.shell_resize_preview_width(), Some(32));

        let snapshot = capture_from_state(&state);
        let encoded = serde_json::to_value(&snapshot).unwrap();

        assert_eq!(snapshot.sidebar_width, Some(26));
        assert_eq!(
            encoded
                .pointer("/shell/region_constraints/LeftPanel/Resizable/preferred")
                .and_then(serde_json::Value::as_u64),
            Some(26)
        );
        assert!(encoded.pointer("/shell/resize").is_none());
        assert!(encoded.pointer("/shell/view_generation").is_none());
        assert!(encoded.pointer("/shell/active_capture").is_none());
    }

    #[test]
    fn resize_commit_is_captured_once() {
        let mut state = state_with_workspaces(&["one"]);
        crate::ui::compute_view(&mut state, Rect::new(0, 0, 106, 40));
        assert!(state.begin_sidebar_resize(Position::new(25, 5)));
        assert!(state.preview_sidebar_resize(Position::new(31, 5)));
        state.session_dirty = false;

        state.commit_sidebar_resize();
        let committed = serde_json::to_value(capture_from_state(&state)).unwrap();
        assert_eq!(state.sidebar_width, 32);
        assert!(state.session_dirty);
        assert_eq!(
            committed
                .pointer("/shell/region_constraints/LeftPanel/Resizable/preferred")
                .and_then(serde_json::Value::as_u64),
            Some(32)
        );

        state.session_dirty = false;
        state.commit_sidebar_resize();
        let repeated = serde_json::to_value(capture_from_state(&state)).unwrap();
        assert!(!state.session_dirty);
        assert_eq!(repeated["shell"], committed["shell"]);
    }

    #[test]
    fn collapsed_left_panel_round_trip_restores_visibility_and_width() {
        let mut state = state_with_workspaces(&["one"]);
        state.sidebar_width = 32;
        crate::ui::compute_view(&mut state, Rect::new(0, 0, 106, 40));
        assert!(state.set_sidebar_collapsed(true));

        let captured = capture_from_state(&state);
        let encoded = serde_json::to_value(&captured).unwrap();
        assert_eq!(
            encoded
                .pointer("/shell/region_constraints/LeftPanel/Collapsed/restore")
                .and_then(serde_json::Value::as_u64),
            Some(32)
        );
        assert_eq!(
            encoded
                .pointer("/shell/collapse_restore_widths/LeftPanel")
                .and_then(serde_json::Value::as_u64),
            Some(32)
        );

        let restored = parse_snapshot(&encoded.to_string()).unwrap();
        assert_eq!(
            restored_left_panel_preference_for_test(&restored),
            (Some(32), true)
        );
    }

    #[test]
    fn invalid_v4_shell_falls_back_without_losing_workspaces() {
        let input = v4_session_with_shell_json(
            serde_json::json!({
                "schema_version": 1,
                "template": "DockSidebarStage",
                "root": { "Slot": { "region": "NotARegion" } }
            }),
            "w-preserved",
            29,
        );

        let restored = parse_snapshot(&input.to_string())
            .expect("invalid shell preferences must not discard valid session data");
        assert_compatible_shell_fallback(&restored, "w-preserved", 29);
    }

    #[test]
    fn over_limit_v4_shell_falls_back_safely() {
        let children = (0..9)
            .map(|_| {
                serde_json::json!({
                    "size": "Fill",
                    "node": { "Slot": { "region": "WorkspaceStage" } }
                })
            })
            .collect::<Vec<_>>();
        let mut shell = shell_json_at_current_schema();
        shell["root"] = serde_json::json!({
            "Split": {
                "direction": "Horizontal",
                "children": children
            }
        });
        let input = v4_session_with_shell_json(shell, "w-over-limit", 27);

        let restored = parse_snapshot(&input.to_string())
            .expect("over-limit shell must fall back without losing the session");
        assert_compatible_shell_fallback(&restored, "w-over-limit", 27);
    }

    #[test]
    fn duplicate_or_unknown_component_placement_falls_back_safely() {
        let duplicate = serde_json::json!([
            { "component": "AppDock", "region": "AppDock" },
            { "component": "AppDock", "region": "LeftPanel" }
        ]);
        let unknown = serde_json::json!([
            { "component": "FutureComponent", "region": "AppDock" }
        ]);

        for (label, placements, width) in [("duplicate", duplicate, 30), ("unknown", unknown, 31)] {
            let mut shell = shell_json_at_current_schema();
            shell["component_placements"] = placements;
            let workspace_id = format!("w-{label}");
            let input = v4_session_with_shell_json(shell, &workspace_id, width);

            let restored = parse_snapshot(&input.to_string()).unwrap_or_else(|error| {
                panic!("{label} placement must fall back without session loss: {error}")
            });
            assert_compatible_shell_fallback(&restored, &workspace_id, width);
        }
    }

    #[test]
    fn unknown_template_falls_back_safely() {
        let mut shell = valid_v4_shell_json();
        shell["template"] = serde_json::json!("FutureWorkspace");
        let input = v4_session_with_shell_json(shell, "w-unknown-template", 28);

        let restored = parse_snapshot(&input.to_string())
            .expect("unknown shell template must not discard valid session data");
        assert_compatible_shell_fallback(&restored, "w-unknown-template", 28);
    }

    #[test]
    fn old_snapshot_defaults_sidebar_fields() {
        let json = serde_json::json!({
            "version": SNAPSHOT_VERSION,
            "workspaces": [],
            "active": null,
            "selected": 0
        })
        .to_string();

        let restored = parse_snapshot(&json).unwrap();

        assert_eq!(restored.sidebar_width, None);
        assert_eq!(restored.sidebar_section_split, None);
    }

    #[test]
    fn old_pane_snapshot_with_embedded_history_is_ignored() {
        let json = serde_json::json!({
            "version": SNAPSHOT_VERSION,
            "workspaces": [{
                "id": "wtest",
                "identity_cwd": "/tmp",
                "tabs": [{
                    "layout": { "Pane": 0 },
                    "panes": {
                        "0": {
                            "cwd": "/tmp",
                            "history": {
                                "ansi": "legacy-secret",
                                "lines": 1
                            }
                        }
                    },
                    "zoomed": false,
                    "focused": 0,
                    "root_pane": 0
                }],
                "active_tab": 0
            }],
            "active": 0,
            "selected": 0
        })
        .to_string();

        let restored = parse_snapshot(&json).unwrap();

        let encoded = serde_json::to_string(&restored).unwrap();
        assert!(!encoded.contains("legacy-secret"));
        assert!(!encoded.contains("\"history\""));
    }

    #[test]
    fn legacy_workspace_snapshot_migrates_to_single_tab() {
        let snap = parse_snapshot(session_fixture("legacy-pre-tabs-v2")).unwrap();
        let ws = &snap.workspaces[0];

        assert_eq!(snap.version, 2);
        assert_eq!(snap.workspaces.len(), 1);
        assert_eq!(ws.custom_name.as_deref(), Some("legacy"));
        assert_eq!(ws.identity_cwd, PathBuf::from("/tmp/pion"));
        assert_eq!(ws.active_tab, 0);
        assert_eq!(ws.tabs.len(), 1);
        assert_eq!(ws.tabs[0].focused, Some(1));
        assert_eq!(ws.tabs[0].root_pane, Some(0));
        assert_eq!(ws.tabs[0].panes[&0].cwd, PathBuf::from("/tmp/pion"));
        assert_eq!(ws.tabs[0].panes[&1].cwd, PathBuf::from("/tmp/herdr"));
    }

    #[test]
    fn capture_contract_tracks_workspace_order_active_and_selected() {
        let mut state = state_with_workspaces(&["a", "b", "c"]);
        state.active = Some(1);
        state.selected = 2;

        state.move_workspace(1, 0);

        let snapshot = capture_from_state(&state);
        let ids: Vec<_> = state.workspaces.iter().map(|ws| ws.id.clone()).collect();
        let captured_ids: Vec<_> = snapshot
            .workspaces
            .iter()
            .map(|ws| ws.id.clone().unwrap())
            .collect();
        assert_eq!(captured_ids, ids);
        assert_eq!(snapshot.active, state.active);
        assert_eq!(snapshot.selected, state.selected);
    }

    #[test]
    fn capture_contract_tracks_workspace_and_tab_names_and_active_tab() {
        let mut state = state_with_workspaces(&["one"]);
        state.workspaces[0].set_custom_name("renamed-workspace".into());
        let second_tab = state.workspaces[0].test_add_tab(Some("logs"));
        state.workspaces[0].switch_tab(second_tab);
        state.workspaces[0].tabs[0].set_custom_name("main".into());

        let snapshot = capture_from_state(&state);
        let workspace = &snapshot.workspaces[0];
        assert_eq!(workspace.custom_name.as_deref(), Some("renamed-workspace"));
        assert_eq!(workspace.active_tab, second_tab);
        assert_eq!(workspace.tabs[0].custom_name.as_deref(), Some("main"));
        assert_eq!(workspace.tabs[1].custom_name.as_deref(), Some("logs"));
    }

    #[test]
    fn capture_contract_tracks_workspace_closure() {
        let mut state = state_with_workspaces(&["one", "two"]);
        state.selected = 1;
        state.active = Some(1);

        state.close_selected_workspace();

        let snapshot = capture_from_state(&state);
        assert_eq!(snapshot.workspaces.len(), 1);
        assert_eq!(snapshot.workspaces[0].custom_name.as_deref(), Some("one"));
        assert_eq!(snapshot.active, Some(0));
        assert_eq!(snapshot.selected, 0);
    }

    #[test]
    fn capture_contract_tracks_sidebar_state() {
        let mut state = state_with_workspaces(&["one"]);
        state.sidebar_width = 31;
        state.sidebar_section_split = 0.4;
        state.collapsed_space_keys.insert("repo-key".into());

        let snapshot = capture_from_state(&state);
        assert_eq!(snapshot.sidebar_width, Some(31));
        assert_eq!(snapshot.sidebar_section_split, Some(0.4));
        assert!(snapshot.collapsed_space_keys.contains("repo-key"));
    }

    // TP-PROJ-PERS-01: project folds ride the session file like space folds,
    // and a session written before projects existed still loads.
    #[test]
    fn capture_contract_tracks_project_folds() {
        let mut state = state_with_workspaces(&["one"]);
        state.collapsed_project_keys.insert("project:herdr".into());

        let snapshot = capture_from_state(&state);
        assert!(snapshot.collapsed_project_keys.contains("project:herdr"));

        let json = serde_json::to_string(&snapshot).expect("snapshot serializes");
        let read_back: SessionSnapshot = serde_json::from_str(&json).expect("snapshot reads back");
        assert!(read_back.collapsed_project_keys.contains("project:herdr"));

        let mut value: serde_json::Value =
            serde_json::from_str(&json).expect("snapshot json parses");
        value
            .as_object_mut()
            .expect("snapshot is an object")
            .remove("collapsed_project_keys")
            .expect("the field was serialized");
        let legacy: SessionSnapshot =
            serde_json::from_value(value).expect("a session file without the field still loads");
        assert!(legacy.collapsed_project_keys.is_empty());
    }

    #[test]
    fn capture_contract_tracks_worktree_space_membership() {
        let mut state = state_with_workspaces(&["main"]);
        state.workspaces[0].worktree_space = Some(crate::workspace::WorktreeSpaceMembership {
            key: "repo-key".into(),
            label: "herdr".into(),
            repo_root: PathBuf::from("/repo/herdr"),
            checkout_path: PathBuf::from("/repo/herdr/worktree-a"),
            is_linked_worktree: true,
        });

        let snapshot = capture_from_state(&state);

        assert_eq!(
            snapshot.workspaces[0].worktree_space,
            state.workspaces[0].worktree_space
        );
    }

    #[test]
    fn capture_contract_tracks_layout_focus_zoom_and_root_pane() {
        let mut state = state_with_workspaces(&["one"]);
        let root = state.workspaces[0].tabs[0].root_pane;
        let second = state.workspaces[0].test_split(Direction::Horizontal);
        state.workspaces[0].tabs[0].layout.focus_pane(second);
        state.toggle_zoom();

        let snapshot = capture_from_state(&state);
        let tab = &snapshot.workspaces[0].tabs[0];
        assert!(matches!(tab.layout, LayoutSnapshot::Split { .. }));
        assert_eq!(tab.focused, Some(second.raw()));
        assert_eq!(tab.root_pane, Some(root.raw()));
        assert!(tab.zoomed);
        assert_eq!(tab.panes.len(), 2);
    }

    #[test]
    fn capture_contract_tracks_focus_navigation() {
        let mut state = state_with_workspaces(&["one"]);
        let root = state.workspaces[0].tabs[0].root_pane;
        let second = state.workspaces[0].test_split(Direction::Horizontal);
        crate::ui::compute_view(&mut state, Rect::new(0, 0, 106, 20));

        state.navigate_pane(NavDirection::Right);

        let snapshot = capture_from_state(&state);
        assert_eq!(snapshot.workspaces[0].tabs[0].focused, Some(second.raw()));
        assert_ne!(snapshot.workspaces[0].tabs[0].focused, Some(root.raw()));
    }

    #[test]
    fn capture_contract_tracks_resize_ratio_changes() {
        let mut state = state_with_workspaces(&["one"]);
        let root = state.workspaces[0].tabs[0].root_pane;
        state.workspaces[0].test_split(Direction::Horizontal);
        state.workspaces[0].layout.focus_pane(root);
        crate::ui::compute_view(&mut state, Rect::new(0, 0, 106, 20));
        let before = capture_from_state(&state);

        state.resize_pane(NavDirection::Right);

        let after = capture_from_state(&state);
        let before_ratio = root_split_ratio(&before.workspaces[0].tabs[0]).unwrap();
        let after_ratio = root_split_ratio(&after.workspaces[0].tabs[0]).unwrap();
        assert_ne!(before_ratio, after_ratio);
    }

    #[test]
    fn capture_contract_tracks_tab_closure() {
        let mut state = state_with_workspaces(&["one"]);
        let second_tab = state.workspaces[0].test_add_tab(Some("logs"));
        state.switch_tab(second_tab);

        state.close_tab();

        let snapshot = capture_from_state(&state);
        let workspace = &snapshot.workspaces[0];
        assert_eq!(workspace.tabs.len(), 1);
        assert_eq!(workspace.active_tab, 0);
        assert!(workspace.tabs[0].custom_name.is_none());
    }

    #[test]
    fn capture_contract_tracks_pane_closure() {
        let mut state = state_with_workspaces(&["one"]);
        state.workspaces[0].test_split(Direction::Horizontal);

        state.close_pane();

        let snapshot = capture_from_state(&state);
        let tab = &snapshot.workspaces[0].tabs[0];
        assert_eq!(tab.panes.len(), 1);
        assert!(matches!(tab.layout, LayoutSnapshot::Pane(_)));
        assert!(!tab.zoomed);
    }

    #[test]
    fn capture_contract_tracks_public_id_counters() {
        let mut state = state_with_workspaces(&["one"]);
        let second = state.workspaces[0].test_split(Direction::Horizontal);
        let third = state.workspaces[0].test_split(Direction::Vertical);
        let second_tab = state.workspaces[0].test_add_tab(None);

        state.workspaces[0].close_pane(second);

        let snapshot = capture_from_state(&state);
        let workspace = &snapshot.workspaces[0];
        assert_eq!(
            workspace.public_pane_numbers,
            HashMap::from([
                (state.workspaces[0].tabs[0].root_pane.raw(), 1),
                (third.raw(), 3),
                (state.workspaces[0].tabs[second_tab].root_pane.raw(), 4),
            ])
        );
        assert_eq!(workspace.next_public_pane_number, 5);
        assert_eq!(workspace.public_tab_numbers, vec![1, 2]);
        assert_eq!(workspace.next_public_tab_number, 3);
    }

    #[test]
    fn capture_contract_tracks_workspace_identity_and_pane_cwds() {
        let mut state = state_with_workspaces(&["one"]);
        let root = state.workspaces[0].tabs[0].root_pane;
        state.workspaces[0].identity_cwd = PathBuf::from("/tmp/pion");
        let second = state.workspaces[0].test_split(Direction::Horizontal);
        state.ensure_test_terminals();
        let root_terminal_id = state.workspaces[0].tabs[0].panes[&root]
            .attached_terminal_id
            .clone();
        state.terminals.get_mut(&root_terminal_id).unwrap().cwd = PathBuf::from("/tmp/pion");
        let second_terminal_id = state.workspaces[0].tabs[0].panes[&second]
            .attached_terminal_id
            .clone();
        state.terminals.get_mut(&second_terminal_id).unwrap().cwd = PathBuf::from("/tmp/herdr");

        let snapshot = capture_from_state(&state);
        let workspace = &snapshot.workspaces[0];
        let tab = &workspace.tabs[0];
        assert_eq!(workspace.identity_cwd, PathBuf::from("/tmp/pion"));
        assert_eq!(tab.panes[&root.raw()].cwd, PathBuf::from("/tmp/pion"));
        assert_eq!(tab.panes[&second.raw()].cwd, PathBuf::from("/tmp/herdr"));
    }

    #[tokio::test]
    async fn capture_contract_tracks_pane_history_from_runtime() {
        let state = state_with_workspaces(&["one"]);
        let root = state.workspaces[0].tabs[0].root_pane;
        let terminal_id = state.workspaces[0].tabs[0].panes[&root]
            .attached_terminal_id
            .clone();
        let mut terminal_runtimes = TerminalRuntimeRegistry::new();
        terminal_runtimes.insert(
            terminal_id,
            crate::terminal::TerminalRuntime::test_with_scrollback_bytes(
                20,
                3,
                4096,
                b"alpha\r\nbeta\r\ngamma\r\n",
            ),
        );

        let snapshot = capture_from_state_with_runtimes(&state, &terminal_runtimes);
        let encoded = serde_json::to_string(&snapshot).unwrap();
        assert!(!encoded.contains("alpha"));
        assert!(!encoded.contains("\"history\""));

        let history_snapshot = capture_history_from_state_with_runtimes(&state, &terminal_runtimes);
        let history = &history_snapshot.workspaces[0].tabs[0].panes[&root.raw()];

        assert!(history.ansi.contains("alpha"));
        assert!(history.ansi.contains("gamma"));
        assert!(history.lines >= 3);
    }

    #[tokio::test]
    async fn capture_contract_tracks_history_for_each_pane() {
        let mut state = state_with_workspaces(&["one"]);
        let first = state.workspaces[0].tabs[0].root_pane;
        let second = state.workspaces[0].test_split(Direction::Horizontal);
        let first_terminal_id = state.workspaces[0].tabs[0].panes[&first]
            .attached_terminal_id
            .clone();
        let second_terminal_id = state.workspaces[0].tabs[0].panes[&second]
            .attached_terminal_id
            .clone();
        let mut terminal_runtimes = TerminalRuntimeRegistry::new();
        terminal_runtimes.insert(
            first_terminal_id,
            crate::terminal::TerminalRuntime::test_with_scrollback_bytes(
                20,
                3,
                4096,
                b"first-pane-history\r\n",
            ),
        );
        terminal_runtimes.insert(
            second_terminal_id,
            crate::terminal::TerminalRuntime::test_with_scrollback_bytes(
                20,
                3,
                4096,
                b"second-pane-history\r\n",
            ),
        );

        let snapshot = capture_from_state_with_runtimes(&state, &terminal_runtimes);
        let encoded = serde_json::to_string(&snapshot).unwrap();
        assert!(!encoded.contains("first-pane-history"));
        assert!(!encoded.contains("second-pane-history"));

        let history_snapshot = capture_history_from_state_with_runtimes(&state, &terminal_runtimes);
        let tab = &history_snapshot.workspaces[0].tabs[0];
        let first_history = &tab.panes[&first.raw()];
        let second_history = &tab.panes[&second.raw()];

        assert!(first_history.ansi.contains("first-pane-history"));
        assert!(second_history.ansi.contains("second-pane-history"));
    }

    #[test]
    fn capture_contract_tracks_hook_authority_agent_session() {
        let mut state = state_with_workspaces(&["one"]);
        let session_path = test_session_path("pi-session.jsonl");
        let root = state.workspaces[0].tabs[0].root_pane;
        state.ensure_test_terminals();
        let terminal_id = state.workspaces[0].tabs[0].panes[&root]
            .attached_terminal_id
            .clone();
        let terminal = state.terminals.get_mut(&terminal_id).unwrap();
        terminal.set_detected_state(
            Some(crate::detect::Agent::Pi),
            crate::detect::AgentState::Idle,
        );
        terminal.set_persisted_agent_session(crate::agent_resume::PersistedAgentSession {
            source: "herdr:pi".into(),
            agent: "pi".into(),
            session_ref: crate::agent_resume::AgentSessionRef::path(session_path.clone()).unwrap(),
        });
        terminal.set_hook_authority_with_session_ref(
            "herdr:pi".into(),
            "pi".into(),
            crate::detect::AgentState::Working,
            None,
            crate::agent_resume::AgentSessionRef::path(session_path.clone()),
            Some(20),
        );

        let snapshot = capture_from_state(&state);
        let agent_session = snapshot.workspaces[0].tabs[0].panes[&root.raw()]
            .agent_session
            .as_ref()
            .expect("agent session should be captured");

        assert_eq!(agent_session.source, "herdr:pi");
        assert_eq!(agent_session.agent, "pi");
        assert_eq!(
            agent_session.kind,
            crate::agent_resume::AgentSessionRefKind::Path
        );
        assert_eq!(agent_session.value, session_path);
    }

    #[test]
    fn capture_contract_preserves_restored_agent_session() {
        let mut state = state_with_workspaces(&["one"]);
        let root = state.workspaces[0].tabs[0].root_pane;
        state.ensure_test_terminals();
        let terminal_id = state.workspaces[0].tabs[0].panes[&root]
            .attached_terminal_id
            .clone();
        state
            .terminals
            .get_mut(&terminal_id)
            .unwrap()
            .set_persisted_agent_session(crate::agent_resume::PersistedAgentSession {
                source: "herdr:opencode".into(),
                agent: "opencode".into(),
                session_ref: crate::agent_resume::AgentSessionRef::id("opencode-session").unwrap(),
            });

        let snapshot = capture_from_state(&state);
        let agent_session = snapshot.workspaces[0].tabs[0].panes[&root.raw()]
            .agent_session
            .as_ref()
            .expect("persisted agent session should be captured");

        assert_eq!(agent_session.source, "herdr:opencode");
        assert_eq!(agent_session.agent, "opencode");
        assert_eq!(
            agent_session.kind,
            crate::agent_resume::AgentSessionRefKind::Id
        );
        assert_eq!(agent_session.value, "opencode-session");
    }

    #[test]
    fn old_unversioned_snapshot_loads_as_version_0() {
        let json = r#"{"workspaces":[],"active":null,"selected":0}"#;
        let snap = parse_snapshot(json).unwrap();
        assert_eq!(snap.version, 0);
    }

    #[test]
    fn future_snapshot_version_is_still_rejected() {
        let json = r#"{"version":999,"workspaces":[],"active":null,"selected":0}"#;
        assert!(parse_snapshot(json).is_err());
    }

    #[test]
    fn active_tab_default_is_zero() {
        let json = r#"{"custom_name":"test","identity_cwd":"/tmp","tabs":[]}"#;
        let ws: WorkspaceSnapshot = serde_json::from_str(json).unwrap();
        assert_eq!(ws.active_tab, 0);
    }

    #[test]
    fn restore_falls_back_to_home_when_cwd_missing() {
        let mut panes = HashMap::new();
        panes.insert(
            0,
            PaneSnapshot {
                cwd: PathBuf::from("/tmp/this-directory-does-not-exist-for-herdr-test"),
                label: None,
                agent_name: None,
                managed_agent_kind: None,
                agent_session: None,
                launch_argv: None,
            },
        );
        panes.insert(
            1,
            PaneSnapshot {
                cwd: std::env::var("HOME")
                    .map(PathBuf::from)
                    .unwrap_or_else(|_| PathBuf::from("/tmp")),
                label: None,
                agent_name: None,
                managed_agent_kind: None,
                agent_session: None,
                launch_argv: None,
            },
        );

        let snap = SessionSnapshot {
            version: SNAPSHOT_VERSION,
            workspaces: vec![WorkspaceSnapshot {
                id: Some("test-ws".to_string()),
                custom_name: Some("fallback test".to_string()),
                identity_cwd: PathBuf::from("/tmp"),
                worktree_space: None,
                public_pane_numbers: HashMap::new(),
                next_public_pane_number: 0,
                public_tab_numbers: Vec::new(),
                next_public_tab_number: 0,
                tabs: vec![TabSnapshot {
                    custom_name: None,
                    layout: LayoutSnapshot::Split {
                        direction: DirectionSnapshot::Horizontal,
                        ratio: 0.5,
                        first: Box::new(LayoutSnapshot::Pane(0)),
                        second: Box::new(LayoutSnapshot::Pane(1)),
                    },
                    panes,
                    zoomed: false,
                    focused: Some(0),
                    root_pane: Some(0),
                }],
                active_tab: 0,
            }],
            active: Some(0),
            selected: 0,
            shell: None,
            sidebar_width: Some(26),
            sidebar_section_split: Some(0.5),
            collapsed_space_keys: std::collections::HashSet::new(),
            collapsed_project_keys: std::collections::HashSet::new(),
            files_tab: None,
        };

        let json = serde_json::to_string(&snap).unwrap();
        let restored = parse_snapshot(&json).unwrap();
        assert_eq!(restored.workspaces.len(), 1);
        assert_eq!(
            restored.workspaces[0].tabs[0].panes[&0].cwd,
            PathBuf::from("/tmp/this-directory-does-not-exist-for-herdr-test")
        );
    }

    // T10 · the file stops claiming a composition nobody drew.
    #[test]
    fn a_captured_shell_records_the_tree_the_app_is_actually_presenting() {
        // Version 1 hardcoded `DockSidebarStage` and that template's root while
        // production drew the legacy `sidebar | stage` tree. Nothing went red:
        // the writer was internally consistent and the reader ignored the tree,
        // so the disagreement could only be found by reading both sides.
        let presentation = ShellPresentationState::new(26);
        let shell = ShellSnapshotV1::from_presentation(26, &presentation);

        assert_eq!(shell.template, None, "production presents the legacy tree");
        assert_eq!(shell.schema_version, SHELL_SNAPSHOT_VERSION);
        assert_eq!(shell.root, shell_persistence_parts(None).root);
        assert!(
            !shell.region_constraints.contains_key(&RegionId::AppDock),
            "the legacy tree has no dock, so the file must not size one"
        );
        assert!(
            !shell
                .collapse_restore_widths
                .contains_key(&RegionId::AppDock),
            "nor remember a restore width for a region that is not on screen"
        );
    }

    // T10c · a fail-closed fallback must not write a fresh lie.
    #[test]
    fn a_template_that_cannot_be_composed_is_recorded_as_the_legacy_tree() {
        // The derivation falls back when a template does not validate. If the
        // snapshot recorded the *request* rather than the result, the file
        // would name a tree the app had just refused to draw — the same class
        // of untruth this layer exists to end.
        let requested = ShellTemplateId::DesktopWorkspace;
        let parts = shell_persistence_parts(Some(requested));
        let presented = ShellPresentationState::from_restored(26, false, parts.template);
        let shell = ShellSnapshotV1::from_presentation(26, &presented);

        assert_eq!(shell.template, parts.template);
        assert_eq!(shell.root, parts.root);
    }

    // T11 · the write half is worthless without the read half.
    #[test]
    fn a_shell_snapshot_survives_a_round_trip_as_the_same_derivation() {
        for template in [
            None,
            Some(ShellTemplateId::StageOnly),
            Some(ShellTemplateId::DockSidebarStage),
            Some(ShellTemplateId::InspectorWorkspace),
        ] {
            let presented = ShellPresentationState::from_restored(31, false, template);
            let written = ShellSnapshotV1::from_presentation(31, &presented);
            let value = serde_json::to_value(&written).expect("a shell snapshot serializes");
            let read = ShellSnapshotV1::from_value(value).expect("and reads back");

            assert_eq!(read.template, written.template, "{template:?}");
            assert_eq!(
                shell_persistence_parts(read.template).root,
                shell_persistence_parts(written.template).root,
                "{template:?} derives a different tree after a round trip"
            );
            assert_eq!(
                read.restored_left_panel_preference(),
                Some(RestoredLeftPanelPreference {
                    width: 31,
                    collapsed: false
                }),
                "{template:?} lost the one preference the person set by hand"
            );
        }
    }

    // T12 · the upgrade must not believe the old file's tree, and must not
    // throw away the part of it that was true.
    #[test]
    fn a_fabricated_version_one_tree_is_dropped_but_its_panel_width_survives() {
        // Every session file written before this layer says `DockSidebarStage`
        // and carries that template's AppDock column. Applying it on upgrade
        // would add a column to a composition the layout lock froze — a V2
        // change arriving as a silent side effect of a bug fix.
        let value = valid_v4_shell_json();
        assert_eq!(value["schema_version"], 1);
        assert_eq!(value["template"], "DockSidebarStage");

        let migrated = ShellSnapshotV1::from_value(value).expect("a v1 file still loads");

        assert_eq!(migrated.template, None, "the claim was never observed");
        assert_eq!(migrated.schema_version, SHELL_SNAPSHOT_VERSION);
        assert_eq!(migrated.root, shell_persistence_parts(None).root);
        assert_eq!(
            migrated.restored_left_panel_preference(),
            Some(RestoredLeftPanelPreference {
                width: 31,
                collapsed: false
            }),
            "the width came from the running app and is the person's own setting"
        );
    }

    // T13 · a file from a newer herdr is refused, not guessed at.
    #[test]
    fn a_future_shell_schema_is_refused_without_destroying_the_session() {
        let mut shell = valid_v4_shell_json();
        shell["schema_version"] = serde_json::json!(SHELL_SNAPSHOT_VERSION + 1);
        let input = v4_session_with_shell_json(shell, "w-future-schema", 28);

        let restored = parse_snapshot(&input.to_string())
            .expect("a future shell schema must not discard valid session data");

        assert_compatible_shell_fallback(&restored, "w-future-schema", 28);
    }

    // T13b · the disk is untrusted input, including at version 2.
    #[test]
    fn a_version_two_tree_without_a_stage_is_refused() {
        let mut shell = valid_v4_shell_json();
        shell["schema_version"] = serde_json::json!(SHELL_SNAPSHOT_VERSION);
        shell["root"] = serde_json::json!({ "Slot": { "region": "LeftPanel" } });
        let input = v4_session_with_shell_json(shell, "w-no-stage", 28);

        let restored = parse_snapshot(&input.to_string())
            .expect("an unusable tree must not discard valid session data");

        assert_compatible_shell_fallback(&restored, "w-no-stage", 28);
    }
}
