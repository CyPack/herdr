use serde::{Deserialize, Serialize};

use super::agents::AgentInfo;
use super::panes::{PaneInfo, PaneLayoutSnapshot};
use super::tabs::TabInfo;
use super::workspaces::WorkspaceInfo;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SessionSnapshot {
    pub version: String,
    pub protocol: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focused_workspace_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focused_tab_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focused_pane_id: Option<String>,
    pub workspaces: Vec<WorkspaceInfo>,
    pub tabs: Vec<TabInfo>,
    pub panes: Vec<PaneInfo>,
    pub layouts: Vec<PaneLayoutSnapshot>,
    pub agents: Vec<AgentInfo>,
    /// Recently closed agents, newest first. Dead records carrying a revival
    /// recipe — no runtime stands behind a row. Default + skip-if-empty keeps
    /// the wire identical for sessions that never closed an agent.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub closed_agents: Vec<ClosedAgentInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ClosedAgentInfo {
    /// Stable identity: what a revival request names, and the dedup key that
    /// keeps one agent from appearing twice.
    pub agent_id: String,
    /// The display name frozen at close time; the live state it came from is
    /// gone, so the record carries it.
    pub label: String,
    /// Where the agent last worked. Absent when the close never knew one — a
    /// revival must refuse to guess rather than fall back to `$HOME`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    /// The workspace the revival returns under.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,
    /// The conversation the revival reattaches to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    /// Close time in milliseconds since the epoch; newest first.
    pub closed_at: u64,
    /// True while a revival spawn is in flight; further requests are inert.
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub reviving: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ClosedAgentReviveParams {
    /// The identity of the closed-agent record to revive.
    pub agent_id: String,
}
