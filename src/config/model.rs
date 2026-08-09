use std::{collections::BTreeSet, num::NonZeroUsize, path::Path};

use crossterm::event::KeyModifiers;
use serde::{de, Deserialize, Deserializer, Serialize};

use super::{
    ActionKeybinds, BindingConfig, CommandKeybindConfig, IndexedKeybind, Keybinds, SidebarConfig,
    SoundConfig, ThemeConfig, DEFAULT_MOBILE_WIDTH_THRESHOLD, DEFAULT_MOUSE_SCROLL_LINES,
    DEFAULT_SCROLLBACK_LIMIT_BYTES,
};

pub const MAX_TOAST_DELAY_SECONDS: u64 = 3600;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum UpdateChannelConfig {
    #[default]
    Stable,
    Preview,
}

impl UpdateChannelConfig {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Preview => "preview",
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(default)]
pub struct UpdateConfig {
    pub channel: UpdateChannelConfig,
    pub version_check: bool,
    pub manifest_check: bool,
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self {
            channel: default_update_channel(),
            version_check: true,
            manifest_check: true,
        }
    }
}

fn default_update_channel() -> UpdateChannelConfig {
    if cfg!(windows) {
        UpdateChannelConfig::Preview
    } else {
        UpdateChannelConfig::Stable
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ToastDelivery {
    #[default]
    Off,
    Herdr,
    Terminal,
    System,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, schemars::JsonSchema, Default,
)]
#[serde(rename_all = "kebab-case")]
pub enum ToastHerdrPosition {
    TopLeft,
    TopRight,
    BottomLeft,
    #[default]
    BottomRight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum ToastClipboardPosition {
    TopLeft,
    TopCenter,
    TopRight,
    BottomLeft,
    #[default]
    BottomCenter,
    BottomRight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum AgentPanelSortConfig {
    #[default]
    #[serde(alias = "workspaces")]
    Spaces,
    Priority,
}

impl AgentPanelSortConfig {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Spaces => "spaces",
            Self::Priority => "priority",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum HostCursorModeConfig {
    #[default]
    Auto,
    Native,
    Drawn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SidebarCollapsedModeConfig {
    #[default]
    Compact,
    Hidden,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RightClickPassthroughModifierConfig(Option<KeyModifiers>);

impl RightClickPassthroughModifierConfig {
    pub fn modifiers(self) -> Option<KeyModifiers> {
        self.0
    }
}

impl<'de> Deserialize<'de> for RightClickPassthroughModifierConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        parse_right_click_passthrough_modifier(&value)
            .map(Self)
            .ok_or_else(|| {
                de::Error::custom(
                    "right_click_passthrough_modifier must be empty, off, none, disabled, ctrl/control, alt/option, cmd/command/super, meta, hyper, or a + separated combination without shift",
                )
            })
    }
}

fn parse_right_click_passthrough_modifier(value: &str) -> Option<Option<KeyModifiers>> {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || trimmed.eq_ignore_ascii_case("off")
        || trimmed.eq_ignore_ascii_case("none")
        || trimmed.eq_ignore_ascii_case("disabled")
    {
        return Some(None);
    }

    let mut modifiers = KeyModifiers::empty();
    for token in trimmed.split('+') {
        let token = token.trim().to_ascii_lowercase();
        let modifier = match token.as_str() {
            "ctrl" | "control" => KeyModifiers::CONTROL,
            "alt" | "option" => KeyModifiers::ALT,
            "cmd" | "command" | "super" => KeyModifiers::SUPER,
            "meta" => KeyModifiers::META,
            "hyper" => KeyModifiers::HYPER,
            "shift" => return None,
            _ => return None,
        };
        modifiers |= modifier;
    }

    (!modifiers.is_empty()).then_some(Some(modifiers))
}

#[derive(Debug, Clone)]
pub struct ToastConfig {
    pub delivery: ToastDelivery,
    pub delay_seconds: u64,
    pub herdr: HerdrToastConfig,
    pub clipboard: ClipboardToastConfig,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(default)]
pub struct HerdrToastConfig {
    pub position: ToastHerdrPosition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(default)]
pub struct ClipboardToastConfig {
    pub enabled: bool,
    pub position: ToastClipboardPosition,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum NewTerminalCwdConfig {
    #[default]
    Follow,
    Home,
    Current,
    Path(String),
}

impl<'de> Deserialize<'de> for NewTerminalCwdConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        match value.trim() {
            "" | "follow" => Ok(Self::Follow),
            "home" => Ok(Self::Home),
            "current" => Ok(Self::Current),
            _ => Ok(Self::Path(value)),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ShellModeConfig {
    #[default]
    Auto,
    Login,
    NonLogin,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct TerminalConfig {
    /// Executable used for new interactive panes. Empty means SHELL, then /bin/sh.
    pub default_shell: String,
    /// Startup mode for new interactive pane shells.
    pub shell_mode: ShellModeConfig,
    /// CWD policy for new interactive panes, tabs, and workspaces.
    pub new_cwd: NewTerminalCwdConfig,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct SessionConfig {
    /// Resume supported AI-agent panes into their native conversation sessions
    /// when restoring a Herdr session. Default: true.
    pub resume_agents_on_restore: bool,
    /// Let each attached display hold its own workspace, tab and focused pane
    /// so several monitors can watch different tabs of one session at the same
    /// time. Set to false to mirror one view onto every display, which is the
    /// behaviour for pairing or a demo. Default: true.
    pub per_display_focus: bool,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            resume_agents_on_restore: true,
            per_display_focus: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ConfigReloadStatus {
    Applied,
    Partial,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct ConfigReloadReport {
    pub status: ConfigReloadStatus,
    pub diagnostics: Vec<String>,
}

/// Validate `[ui]` sidebar bound configuration.
///
/// Returns `Some((min, max))` when `min <= max`, `None` otherwise. The two
/// values are funneled through this helper before they reach any
/// `u16::clamp(min, max)` call site (`u16::clamp` panics when `min > max`).
pub fn validated_sidebar_bounds(min: u16, max: u16) -> Option<(u16, u16)> {
    if min <= max {
        Some((min, max))
    } else {
        None
    }
}

/// Projects sidebar tab configuration.
///
/// Holds the directories pinned to the Projects tab. Entries are user-authored,
/// so blank and non-absolute values are tolerated instead of failing the whole
/// config load: they are dropped from the usable set and reported by
/// [`ProjectsConfig::diagnostics`]. The usable-path accessor is added with the
/// Projects render that consumes it.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct ProjectsConfig {
    /// Absolute (or `~`-rooted) project directories pinned to the Projects tab.
    pub pinned: Vec<String>,
    /// Agent CLI id for NEW chats opened from the Projects tab (one of the
    /// built-in chat agents; unknown values fall back to claude at launch, so
    /// a stale entry can never break the "+" button).
    pub default_chat_agent: Option<String>,
    /// Whether the Projects tab lists only chats that are open as tabs
    /// (the footer "actives" toggle). Defaults to on when absent.
    pub actives_only: Option<bool>,
}

impl ProjectsConfig {
    /// Usable pinned directories, in config order: non-blank entries that are
    /// absolute or `~`-rooted, each trimmed of surrounding whitespace. Blank and
    /// non-absolute entries are dropped (they are surfaced by
    /// [`Self::diagnostics`] instead of failing the whole config load). Returned
    /// values may still start with `~` — callers expand it before use.
    pub fn pinned_paths(&self) -> Vec<String> {
        self.pinned
            .iter()
            .filter_map(|raw| {
                let trimmed = raw.trim();
                pinned_entry_is_usable(trimmed).then(|| trimmed.to_string())
            })
            .collect()
    }

    /// Human-readable diagnostics for pinned entries that are unusable (blank,
    /// or a non-absolute path). Usable entries produce no diagnostic.
    pub fn diagnostics(&self) -> Vec<String> {
        self.pinned
            .iter()
            .filter_map(|raw| {
                let trimmed = raw.trim();
                if trimmed.is_empty() {
                    Some(format!("projects.pinned entry {raw:?} is blank; ignoring"))
                } else if !pinned_entry_is_usable(trimmed) {
                    Some(format!(
                        "projects.pinned entry {trimmed:?} is not an absolute path; ignoring"
                    ))
                } else {
                    None
                }
            })
            .collect()
    }
}

/// Space partitioning configuration: the `[[spaces.split]]` rules that move
/// some of a repository's checkouts into their own sidebar space.
///
/// Entries are user-authored, so an unusable one is dropped and reported rather
/// than failing the whole config load — the same tolerance [`ProjectsConfig`]
/// applies to `pinned`. An empty list reproduces stock grouping exactly.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct SpacesConfig {
    pub split: Vec<SpaceSplitEntry>,
    /// `[[spaces.project]]` umbrellas gathering repositories and spaces under
    /// one top-level header.
    pub project: Vec<SpaceProjectEntry>,
    /// `[spaces.icons]` row-kind icon defaults for the Spaces tree.
    pub icons: SpaceIconsConfig,
}

/// One authored `[[spaces.split]]` table, before validation.
#[derive(Debug, Default, Clone, Deserialize)]
#[serde(default)]
pub struct SpaceSplitEntry {
    /// Repository root the rule applies to. Absolute or `~`-rooted.
    pub repo: String,
    /// Globs claiming checkouts, matched against branch then directory name.
    #[serde(rename = "match")]
    pub patterns: Vec<String>,
    /// Space key the claimed checkouts group under.
    pub key: String,
    /// Group header label. Falls back to `key` when blank.
    pub label: String,
    /// Glyph drawn before the label. Blank means no icon of its own.
    pub icon: String,
}

/// One authored `[[spaces.project]]` table, before validation.
#[derive(Debug, Default, Clone, Deserialize)]
#[serde(default)]
pub struct SpaceProjectEntry {
    /// Header title. Falls back to `key` when blank.
    pub name: String,
    /// Collapse-state key. Must be unique per project.
    pub key: String,
    /// Glyph drawn before the name; blank uses `[spaces.icons] project`.
    pub icon: String,
    /// Repository roots whose spaces all join this project. Absolute or
    /// `~`-rooted.
    pub repos: Vec<String>,
    /// Individual space keys joining this project.
    pub spaces: Vec<String>,
}

/// Row-kind icons for the Spaces tree. Every field carries a working default,
/// so the table is only written to change taste, not to enable the feature.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct SpaceIconsConfig {
    /// Project header rows without an `icon` of their own.
    pub project: String,
    /// Workspace (branch/checkout) rows.
    pub branch: String,
    /// Chat drawer rows.
    pub chat: String,
}

impl Default for SpaceIconsConfig {
    fn default() -> Self {
        Self {
            project: "📁".to_string(),
            branch: "".to_string(),
            chat: "💬".to_string(),
        }
    }
}

impl SpacesConfig {
    /// Validated rules in config order, `~` expanded. Unusable entries are
    /// dropped (see [`Self::diagnostics`]).
    pub fn rules(&self) -> Vec<crate::spaces::SpaceSplitRule> {
        self.split
            .iter()
            .filter(|entry| space_split_entry_problem(entry).is_none())
            .map(|entry| {
                let label = entry.label.trim();
                let icon = entry.icon.trim();
                crate::spaces::SpaceSplitRule {
                    repo_root: crate::worktree::expand_tilde_absolute_path(entry.repo.trim()),
                    patterns: entry
                        .patterns
                        .iter()
                        .map(|pattern| pattern.trim().to_string())
                        .filter(|pattern| !pattern.is_empty())
                        .collect(),
                    key: entry.key.trim().to_string(),
                    label: if label.is_empty() {
                        entry.key.trim().to_string()
                    } else {
                        label.to_string()
                    },
                    icon: (!icon.is_empty()).then(|| icon.to_string()),
                }
            })
            .collect()
    }

    /// Validated projects in config order, `~` expanded. Unusable entries are
    /// dropped (see [`Self::diagnostics`]).
    pub fn projects(&self) -> Vec<crate::spaces::SpaceProject> {
        self.project
            .iter()
            .filter(|entry| space_project_entry_problem(entry).is_none())
            .map(|entry| {
                let name = entry.name.trim();
                let icon = entry.icon.trim();
                crate::spaces::SpaceProject {
                    key: entry.key.trim().to_string(),
                    name: if name.is_empty() {
                        entry.key.trim().to_string()
                    } else {
                        name.to_string()
                    },
                    icon: (!icon.is_empty()).then(|| icon.to_string()),
                    repo_roots: entry
                        .repos
                        .iter()
                        .map(|repo| repo.trim())
                        .filter(|repo| !repo.is_empty())
                        .map(crate::worktree::expand_tilde_absolute_path)
                        .collect(),
                    space_keys: entry
                        .spaces
                        .iter()
                        .map(|space| space.trim().to_string())
                        .filter(|space| !space.is_empty())
                        .collect(),
                }
            })
            .collect()
    }

    /// Human-readable diagnostics for dropped entries, plus duplicate keys
    /// (TP-SPLIT-CONF-01, TP-SPLIT-CONF-02, TP-PROJ-CONF-01, TP-PROJ-CONF-02).
    pub fn diagnostics(&self) -> Vec<String> {
        let mut seen_keys = std::collections::HashSet::new();
        let mut diagnostics: Vec<String> = self
            .split
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| {
                space_split_entry_problem(entry)
                    .map(|problem| format!("spaces.split[{index}] {problem}; ignoring"))
            })
            .collect();
        // A duplicate key inside one repository would silently merge two of
        // its modules, which looks like a grouping bug rather than a config
        // typo. Across repositories the same key is deliberate: it gathers
        // several repositories' checkouts into one module (TP-SPLIT-CONF-04).
        for (index, entry) in self.split.iter().enumerate() {
            let key = entry.key.trim();
            let repo = crate::worktree::expand_tilde_absolute_path(entry.repo.trim());
            if !key.is_empty() && !seen_keys.insert((repo, key.to_string())) {
                diagnostics.push(format!(
                    "spaces.split[{index}] key {key:?} is already used by an earlier rule \
                     for the same repository; both rules will share one space"
                ));
            }
        }
        diagnostics.extend(
            self.project
                .iter()
                .enumerate()
                .filter_map(|(index, entry)| {
                    space_project_entry_problem(entry)
                        .map(|problem| format!("spaces.project[{index}] {problem}; ignoring"))
                }),
        );
        // Same reasoning as duplicate rule keys: two projects sharing a key
        // share one collapse state, which reads as a folding bug.
        let mut seen_project_keys = std::collections::HashSet::new();
        for (index, entry) in self.project.iter().enumerate() {
            let key = entry.key.trim();
            if !key.is_empty() && !seen_project_keys.insert(key.to_string()) {
                diagnostics.push(format!(
                    "spaces.project[{index}] key {key:?} is already used by an earlier project; \
                     both will share one collapse state"
                ));
            }
        }
        diagnostics
    }
}

/// Why a project entry cannot be used, or `None` when it is usable
/// (TP-PROJ-CONF-01).
fn space_project_entry_problem(entry: &SpaceProjectEntry) -> Option<&'static str> {
    if entry.key.trim().is_empty() {
        return Some("has no key");
    }
    // A relative repo drops the whole entry rather than being skipped: a
    // half-applied project reads as a grouping bug, not as a config typo.
    if entry.repos.iter().any(|repo| {
        let trimmed = repo.trim();
        !trimmed.is_empty() && !pinned_entry_is_usable(trimmed)
    }) {
        return Some("has a repo that is not an absolute path");
    }
    let has_repo_member = entry
        .repos
        .iter()
        .any(|repo| pinned_entry_is_usable(repo.trim()));
    let has_space_member = entry.spaces.iter().any(|space| !space.trim().is_empty());
    if !has_repo_member && !has_space_member {
        return Some("has no usable repos or spaces member");
    }
    None
}

/// Why an entry cannot be used, or `None` when it is usable
/// (TP-SPLIT-CONF-01).
fn space_split_entry_problem(entry: &SpaceSplitEntry) -> Option<&'static str> {
    let repo = entry.repo.trim();
    if repo.is_empty() {
        return Some("has no repo");
    }
    if !pinned_entry_is_usable(repo) {
        return Some("repo is not an absolute path");
    }
    if entry.key.trim().is_empty() {
        return Some("has no key");
    }
    if entry
        .patterns
        .iter()
        .all(|pattern| pattern.trim().is_empty())
    {
        return Some("has no usable match pattern");
    }
    None
}

/// A pinned entry is usable when it is non-blank and points at an absolute or
/// `~`-rooted location. `~` is accepted because Herdr config paths conventionally
/// use it (e.g. `worktrees.directory` defaults to `~/.herdr/worktrees`).
fn pinned_entry_is_usable(trimmed: &str) -> bool {
    !trimmed.is_empty() && (Path::new(trimmed).is_absolute() || trimmed.starts_with('~'))
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Config {
    pub onboarding: Option<bool>,
    pub theme: ThemeConfig,
    pub terminal: TerminalConfig,
    pub session: SessionConfig,
    pub update: UpdateConfig,
    pub keys: KeysConfig,
    pub ui: UiConfig,
    pub worktrees: WorktreesConfig,
    pub advanced: AdvancedConfig,
    pub experimental: ExperimentalConfig,
    pub remote: RemoteConfig,
    pub projects: ProjectsConfig,
    pub spaces: SpacesConfig,
    pub preview: PreviewConfig,
    pub tailscale: TailscaleConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct TailscaleConfig {
    /// Devices the user put at the top of the send picker, in their order.
    ///
    /// Stored as the tailnet DNS name rather than the host name, because host
    /// names repeat and a pin has to name one machine. Order is the reader's:
    /// the first entry sorts above the second.
    pub pinned_devices: Vec<String>,
}

/// `[preview]` — how the wired browser preview places itself when a chat tab
/// gains focus. Only the default mode is implemented today; the other modes
/// are announced ("soon") so the setting surface is stable before the
/// backends land, and any of them behaves like the default at runtime.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct PreviewConfig {
    pub placement: PreviewPlacement,
}

/// Preview window placement mode. Runtime consumers must check
/// [`PreviewPlacement::is_available`] and fall back to the default for the
/// announced-but-unimplemented modes (a hand-edited config must never change
/// behavior silently or break the app).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PreviewPlacement {
    /// Half screen on the display the window is already on (single- and
    /// multi-monitor). The only implemented mode — the default.
    #[default]
    SameScreenHalf,
    /// Other display, shared browser window with one tab per preview (soon).
    OtherMonitorSharedTabs,
    /// Other display, a dedicated window per chat (soon).
    OtherMonitorPerChat,
    /// User-defined placement matrix (soon).
    CustomMatrix,
}

impl PreviewPlacement {
    pub const ALL: &[Self] = &[
        Self::SameScreenHalf,
        Self::OtherMonitorSharedTabs,
        Self::OtherMonitorPerChat,
        Self::CustomMatrix,
    ];

    /// Whether the mode's backend exists. Everything except the default is
    /// announced-only for now.
    pub fn is_available(self) -> bool {
        matches!(self, Self::SameScreenHalf)
    }

    /// The kebab-case config value (used when persisting the selection).
    pub fn config_value(self) -> &'static str {
        match self {
            Self::SameScreenHalf => "same-screen-half",
            Self::OtherMonitorSharedTabs => "other-monitor-shared-tabs",
            Self::OtherMonitorPerChat => "other-monitor-per-chat",
            Self::CustomMatrix => "custom-matrix",
        }
    }
}

#[derive(Debug)]
pub struct LoadedConfig {
    pub config: Config,
    pub diagnostics: Vec<String>,
    pub invalid_sections: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct KeysConfig {
    /// Prefix key to enter prefix mode (e.g. "ctrl+b", "f12", "esc").
    pub prefix: String,
    /// Open keybinding help. Default: "prefix+?"
    pub help: BindingConfig,
    /// Open settings. Default: "prefix+s"
    pub settings: BindingConfig,
    /// Create a new workspace. Default: "prefix+shift+n"
    pub new_workspace: BindingConfig,
    /// Create a Git worktree from the selected workspace. Default: "prefix+shift+g"
    pub new_worktree: BindingConfig,
    /// Open an existing Git worktree from the selected workspace. Unset by default.
    pub open_worktree: BindingConfig,
    /// Delete the selected managed worktree checkout after confirmation. Unset by default.
    pub remove_worktree: BindingConfig,
    /// Rename the selected workspace. Default: "prefix+shift+w"
    pub rename_workspace: BindingConfig,
    /// Close the selected workspace. Default: "prefix+shift+d"
    pub close_workspace: BindingConfig,
    /// Open the workspace navigation surface. Default: "prefix+w"
    pub workspace_picker: BindingConfig,
    /// Open the session navigator. Default: "prefix+g"
    pub goto: BindingConfig,
    /// Move workspace selection up in navigate mode. Default: "up".
    pub navigate_workspace_up: BindingConfig,
    /// Move workspace selection down in navigate mode. Default: "down".
    pub navigate_workspace_down: BindingConfig,
    /// Focus the pane to the left in navigate mode. Default: "h". Left arrow is always an alias.
    pub navigate_pane_left: BindingConfig,
    /// Focus the pane below in navigate mode. Default: "j".
    pub navigate_pane_down: BindingConfig,
    /// Focus the pane above in navigate mode. Default: "k".
    pub navigate_pane_up: BindingConfig,
    /// Focus the pane to the right in navigate mode. Default: "l". Right arrow is always an alias.
    pub navigate_pane_right: BindingConfig,
    /// Detach from server/client mode, or exit --no-session mode. Default: "prefix+q".
    pub detach: BindingConfig,
    /// Reload config.toml in the running app/server. Default: "prefix+shift+r".
    pub reload_config: BindingConfig,
    /// Focus the currently visible notification target. Default: "prefix+o".
    pub open_notification_target: BindingConfig,
    /// Select the previous workspace. Unset by default.
    pub previous_workspace: BindingConfig,
    /// Select the next workspace. Unset by default.
    pub next_workspace: BindingConfig,
    /// Focus the previous agent shown in the agent panel. Unset by default.
    pub previous_agent: BindingConfig,
    /// Focus the next agent shown in the agent panel. Unset by default.
    pub next_agent: BindingConfig,
    /// Focus an agent by index 1-9. Unset by default.
    pub focus_agent: BindingConfig,
    /// Local-client shortcut that sends a clipboard image to a remote Herdr session. Default: "ctrl+v".
    pub remote_image_paste: String,
    /// Create a new tab in the active workspace. Default: "prefix+c"
    pub new_tab: BindingConfig,
    /// Open a new chat tab in the home directory with the default chat agent. Unset by default.
    pub new_chat_tab: BindingConfig,
    /// Rename the active tab. Default: "prefix+shift+t".
    pub rename_tab: BindingConfig,
    /// Select the previous tab. Default: "prefix+p".
    pub previous_tab: BindingConfig,
    /// Select the next tab. Default: "prefix+n".
    pub next_tab: BindingConfig,
    /// Switch to tab 1-9. Default: "prefix+1..9".
    pub switch_tab: BindingConfig,
    /// Switch to workspace 1-9 from prefix mode. Unset by default.
    pub switch_workspace: BindingConfig,
    /// Close the active tab. Default: "prefix+shift+x".
    pub close_tab: BindingConfig,
    /// Rename the focused pane. Default: "prefix+shift+p".
    pub rename_pane: BindingConfig,
    /// Open the focused pane scrollback in $EDITOR. Default: "prefix+e".
    pub edit_scrollback: BindingConfig,
    /// Enter keyboard copy mode for the focused pane. Default: "prefix+[".
    pub copy_mode: BindingConfig,
    /// Focus the pane to the left. Default: "prefix+h".
    pub focus_pane_left: BindingConfig,
    /// Focus the pane below. Default: "prefix+j".
    pub focus_pane_down: BindingConfig,
    /// Focus the pane above. Default: "prefix+k".
    pub focus_pane_up: BindingConfig,
    /// Focus the pane to the right. Default: "prefix+l".
    pub focus_pane_right: BindingConfig,
    /// Swap the focused pane with the pane to the left. Default: "prefix+shift+h".
    pub swap_pane_left: BindingConfig,
    /// Swap the focused pane with the pane below. Default: "prefix+shift+j".
    pub swap_pane_down: BindingConfig,
    /// Swap the focused pane with the pane above. Default: "prefix+shift+k".
    pub swap_pane_up: BindingConfig,
    /// Swap the focused pane with the pane to the right. Default: "prefix+shift+l".
    pub swap_pane_right: BindingConfig,
    /// Cycle to the next pane. Default: "prefix+tab".
    pub cycle_pane_next: BindingConfig,
    /// Cycle to the previous pane. Default: "prefix+shift+tab".
    pub cycle_pane_previous: BindingConfig,
    /// Focus the last focused pane across workspaces and tabs. Unset by default.
    pub last_pane: BindingConfig,
    /// Split pane vertically (side by side). Default: "prefix+v"
    pub split_vertical: BindingConfig,
    /// Split pane horizontally (stacked). Default: "prefix+minus"
    pub split_horizontal: BindingConfig,
    /// Close the focused pane. Default: "prefix+x"
    pub close_pane: BindingConfig,
    /// Toggle zoom for the focused pane. Default: "prefix+z"
    #[serde(alias = "fullscreen")]
    pub zoom: BindingConfig,
    /// Enter resize mode. Default: "prefix+r"
    pub resize_mode: BindingConfig,
    /// Toggle sidebar collapse. Default: "prefix+b"
    pub toggle_sidebar: BindingConfig,
    /// Toggle the native file manager. Default: "prefix+f"
    pub toggle_file_manager: BindingConfig,
    /// Open the focused-agent attachment picker. Default: "prefix+a"
    pub agent_attachment_picker: BindingConfig,
    /// Optional indexed shortcuts expanded over number keys 1-9.
    pub indexed: IndexedKeysConfig,
    /// Prefix-mode custom command bindings.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub command: Vec<CommandKeybindConfig>,
    #[serde(skip_serializing)]
    pub(crate) user_fields: BTreeSet<&'static str>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub(crate) struct KeysConfigOverlay {
    #[serde(skip_serializing_if = "Option::is_none")]
    prefix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    help: Option<BindingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    settings: Option<BindingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    new_workspace: Option<BindingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    new_worktree: Option<BindingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    open_worktree: Option<BindingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    remove_worktree: Option<BindingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rename_workspace: Option<BindingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    close_workspace: Option<BindingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    workspace_picker: Option<BindingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    goto: Option<BindingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    navigate_workspace_up: Option<BindingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    navigate_workspace_down: Option<BindingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    navigate_pane_left: Option<BindingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    navigate_pane_down: Option<BindingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    navigate_pane_up: Option<BindingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    navigate_pane_right: Option<BindingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    detach: Option<BindingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reload_config: Option<BindingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    open_notification_target: Option<BindingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    previous_workspace: Option<BindingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    next_workspace: Option<BindingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    previous_agent: Option<BindingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    next_agent: Option<BindingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    focus_agent: Option<BindingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    remote_image_paste: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    new_tab: Option<BindingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    new_chat_tab: Option<BindingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rename_tab: Option<BindingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    previous_tab: Option<BindingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    next_tab: Option<BindingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    switch_tab: Option<BindingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    switch_workspace: Option<BindingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    close_tab: Option<BindingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rename_pane: Option<BindingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    edit_scrollback: Option<BindingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    copy_mode: Option<BindingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    focus_pane_left: Option<BindingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    focus_pane_down: Option<BindingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    focus_pane_up: Option<BindingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    focus_pane_right: Option<BindingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    swap_pane_left: Option<BindingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    swap_pane_down: Option<BindingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    swap_pane_up: Option<BindingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    swap_pane_right: Option<BindingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cycle_pane_next: Option<BindingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cycle_pane_previous: Option<BindingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_pane: Option<BindingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    split_vertical: Option<BindingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    split_horizontal: Option<BindingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    close_pane: Option<BindingConfig>,
    #[serde(alias = "fullscreen", skip_serializing_if = "Option::is_none")]
    zoom: Option<BindingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    resize_mode: Option<BindingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    toggle_sidebar: Option<BindingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    toggle_file_manager: Option<BindingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    agent_attachment_picker: Option<BindingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    indexed: Option<IndexedKeysConfig>,
    #[serde(skip_serializing)]
    command: Option<Vec<CommandKeybindConfig>>,
}

impl<'de> Deserialize<'de> for KeysConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let input = KeysConfigOverlay::deserialize(deserializer)?;
        let mut keys = KeysConfig::default();

        macro_rules! apply_field {
            ($field:ident) => {
                if let Some(value) = input.$field {
                    keys.$field = value;
                    keys.user_fields.insert(stringify!($field));
                }
            };
        }

        apply_field!(prefix);
        apply_field!(help);
        apply_field!(settings);
        apply_field!(new_workspace);
        apply_field!(new_worktree);
        apply_field!(open_worktree);
        apply_field!(remove_worktree);
        apply_field!(rename_workspace);
        apply_field!(close_workspace);
        apply_field!(workspace_picker);
        apply_field!(goto);
        apply_field!(navigate_workspace_up);
        apply_field!(navigate_workspace_down);
        apply_field!(navigate_pane_left);
        apply_field!(navigate_pane_down);
        apply_field!(navigate_pane_up);
        apply_field!(navigate_pane_right);
        apply_field!(detach);
        apply_field!(reload_config);
        apply_field!(open_notification_target);
        apply_field!(previous_workspace);
        apply_field!(next_workspace);
        apply_field!(previous_agent);
        apply_field!(next_agent);
        apply_field!(focus_agent);
        apply_field!(remote_image_paste);
        apply_field!(new_tab);
        apply_field!(new_chat_tab);
        apply_field!(rename_tab);
        apply_field!(previous_tab);
        apply_field!(next_tab);
        apply_field!(switch_tab);
        apply_field!(switch_workspace);
        apply_field!(close_tab);
        apply_field!(rename_pane);
        apply_field!(edit_scrollback);
        apply_field!(copy_mode);
        apply_field!(focus_pane_left);
        apply_field!(focus_pane_down);
        apply_field!(focus_pane_up);
        apply_field!(focus_pane_right);
        apply_field!(swap_pane_left);
        apply_field!(swap_pane_down);
        apply_field!(swap_pane_up);
        apply_field!(swap_pane_right);
        apply_field!(cycle_pane_next);
        apply_field!(cycle_pane_previous);
        apply_field!(last_pane);
        apply_field!(split_vertical);
        apply_field!(split_horizontal);
        apply_field!(close_pane);
        apply_field!(zoom);
        apply_field!(resize_mode);
        apply_field!(toggle_sidebar);
        apply_field!(toggle_file_manager);
        apply_field!(agent_attachment_picker);
        apply_field!(indexed);
        apply_field!(command);

        Ok(keys)
    }
}

impl KeysConfig {
    pub(crate) fn key_field_is_user_configured(&self, field: &str) -> bool {
        self.user_fields.contains(field)
    }

    pub(crate) fn local_profile(&self, keybinds: &Keybinds) -> KeysConfigOverlay {
        let mut profile = KeysConfigOverlay::default();

        macro_rules! copy_user_field {
            ($field:ident) => {
                if self.user_fields.contains(stringify!($field)) {
                    profile.$field = Some(self.$field.clone());
                }
            };
        }
        macro_rules! copy_effective_action_field {
            ($field:ident, $target:expr) => {
                if self.user_fields.contains(stringify!($field)) {
                    profile.$field = Some(self.$field.clone());
                } else if binding_config_is_effective(&self.$field, &$target) {
                    profile.$field = Some(self.$field.clone());
                } else if binding_config_has_values(&self.$field) {
                    profile.$field = Some(BindingConfig::empty());
                }
            };
        }
        macro_rules! copy_effective_indexed_field {
            ($field:ident, $target:expr) => {
                if self.user_fields.contains(stringify!($field)) {
                    profile.$field = Some(self.$field.clone());
                } else if let Some(effective) = effective_indexed_config(&self.$field, &$target) {
                    profile.$field = Some(effective);
                } else if binding_config_has_values(&self.$field) {
                    profile.$field = Some(BindingConfig::empty());
                }
            };
        }

        profile.prefix = Some(self.prefix.clone());
        copy_effective_action_field!(help, keybinds.help);
        copy_effective_action_field!(settings, keybinds.settings);
        copy_effective_action_field!(new_workspace, keybinds.new_workspace);
        copy_effective_action_field!(new_worktree, keybinds.new_worktree);
        copy_effective_action_field!(open_worktree, keybinds.open_worktree);
        copy_effective_action_field!(remove_worktree, keybinds.remove_worktree);
        copy_effective_action_field!(rename_workspace, keybinds.rename_workspace);
        copy_effective_action_field!(close_workspace, keybinds.close_workspace);
        copy_effective_action_field!(workspace_picker, keybinds.workspace_picker);
        copy_effective_action_field!(goto, keybinds.goto);
        copy_effective_action_field!(navigate_workspace_up, keybinds.navigate.workspace_up);
        copy_effective_action_field!(navigate_workspace_down, keybinds.navigate.workspace_down);
        copy_effective_action_field!(navigate_pane_left, keybinds.navigate.pane_left);
        copy_effective_action_field!(navigate_pane_down, keybinds.navigate.pane_down);
        copy_effective_action_field!(navigate_pane_up, keybinds.navigate.pane_up);
        copy_effective_action_field!(navigate_pane_right, keybinds.navigate.pane_right);
        copy_effective_action_field!(detach, keybinds.detach);
        copy_effective_action_field!(reload_config, keybinds.reload_config);
        copy_effective_action_field!(open_notification_target, keybinds.open_notification_target);
        copy_effective_action_field!(previous_workspace, keybinds.previous_workspace);
        copy_effective_action_field!(next_workspace, keybinds.next_workspace);
        copy_effective_action_field!(previous_agent, keybinds.previous_agent);
        copy_effective_action_field!(next_agent, keybinds.next_agent);
        copy_effective_indexed_field!(focus_agent, keybinds.focus_agent);
        copy_user_field!(remote_image_paste);
        copy_effective_action_field!(new_tab, keybinds.new_tab);
        copy_effective_action_field!(new_chat_tab, keybinds.new_chat_tab);
        copy_effective_action_field!(rename_tab, keybinds.rename_tab);
        copy_effective_action_field!(previous_tab, keybinds.previous_tab);
        copy_effective_action_field!(next_tab, keybinds.next_tab);
        copy_effective_indexed_field!(switch_tab, keybinds.switch_tab);
        copy_effective_indexed_field!(switch_workspace, keybinds.switch_workspace);
        copy_effective_action_field!(close_tab, keybinds.close_tab);
        copy_effective_action_field!(rename_pane, keybinds.rename_pane);
        copy_effective_action_field!(edit_scrollback, keybinds.edit_scrollback);
        copy_effective_action_field!(copy_mode, keybinds.copy_mode);
        copy_effective_action_field!(focus_pane_left, keybinds.focus_pane_left);
        copy_effective_action_field!(focus_pane_down, keybinds.focus_pane_down);
        copy_effective_action_field!(focus_pane_up, keybinds.focus_pane_up);
        copy_effective_action_field!(focus_pane_right, keybinds.focus_pane_right);
        copy_effective_action_field!(swap_pane_left, keybinds.swap_pane_left);
        copy_effective_action_field!(swap_pane_down, keybinds.swap_pane_down);
        copy_effective_action_field!(swap_pane_up, keybinds.swap_pane_up);
        copy_effective_action_field!(swap_pane_right, keybinds.swap_pane_right);
        copy_effective_action_field!(cycle_pane_next, keybinds.cycle_pane_next);
        copy_effective_action_field!(cycle_pane_previous, keybinds.cycle_pane_previous);
        copy_effective_action_field!(last_pane, keybinds.last_pane);
        copy_effective_action_field!(split_vertical, keybinds.split_vertical);
        copy_effective_action_field!(split_horizontal, keybinds.split_horizontal);
        copy_effective_action_field!(close_pane, keybinds.close_pane);
        copy_effective_action_field!(zoom, keybinds.zoom);
        copy_effective_action_field!(resize_mode, keybinds.resize_mode);
        copy_effective_action_field!(toggle_sidebar, keybinds.toggle_sidebar);
        copy_effective_action_field!(toggle_file_manager, keybinds.toggle_file_manager);
        copy_effective_action_field!(agent_attachment_picker, keybinds.agent_attachment_picker);
        copy_user_field!(indexed);

        profile
    }
}

fn binding_config_has_values(config: &BindingConfig) -> bool {
    config.has_values()
}

fn binding_config_is_effective(config: &BindingConfig, keybinds: &ActionKeybinds) -> bool {
    !binding_config_has_values(config) || !keybinds.bindings.is_empty()
}

fn effective_indexed_config(
    config: &BindingConfig,
    keybinds: &[IndexedKeybind],
) -> Option<BindingConfig> {
    if !binding_config_has_values(config) {
        return Some(config.clone());
    }

    let expected_labels = config.indexed_labels();
    if expected_labels.is_empty() {
        return None;
    }

    let effective_labels: Vec<String> = expected_labels
        .iter()
        .filter(|expected| {
            keybinds
                .iter()
                .any(|binding| binding.label.as_str() == expected.as_str())
        })
        .cloned()
        .collect();

    if effective_labels.is_empty() {
        None
    } else if effective_labels.len() == expected_labels.len() {
        Some(config.clone())
    } else {
        Some(BindingConfig::Many(effective_labels))
    }
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct IndexedKeysConfig {
    /// Modifier combo for tab shortcuts 1-9. Unset by default.
    pub tabs: String,
    /// Modifier combo for workspace shortcuts 1-9. Unset by default.
    pub workspaces: String,
    /// Modifier combo for agent shortcuts 1-9. Unset by default.
    pub agents: String,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct WorktreesConfig {
    /// Root directory under which Herdr creates <repo>/<branch-slug> checkouts.
    pub directory: String,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct UiConfig {
    pub sidebar_width: u16,
    /// Minimum sidebar width (columns) when expanded. Default: 18.
    pub sidebar_min_width: u16,
    /// Maximum sidebar width (columns) when expanded. Default: 36.
    pub sidebar_max_width: u16,
    /// Start with the sidebar collapsed. Default: false.
    pub sidebar_start_collapsed: bool,
    /// Collapsed sidebar presentation. Default: compact.
    pub sidebar_collapsed_mode: SidebarCollapsedModeConfig,
    /// Terminal width at or below which Herdr uses the mobile single-column layout. Default: 64.
    pub mobile_width_threshold: u16,
    /// Capture mouse input for Herdr's mouse UI. Default: true.
    pub mouse_capture: bool,
    /// Copy text selected with the mouse. Default: true.
    pub copy_on_select: bool,
    /// Host cursor policy. Default: auto.
    pub host_cursor: HostCursorModeConfig,
    /// Modifier that lets right-click gestures pass through to pane apps. Empty disables it.
    pub right_click_passthrough_modifier: RightClickPassthroughModifierConfig,
    /// Force a full host-terminal redraw when the outer terminal regains focus. Default: true.
    pub redraw_on_focus_gained: bool,
    /// Lines to scroll per mouse wheel notch. Default: 3.
    pub mouse_scroll_lines: Option<NonZeroUsize>,
    /// Ask for confirmation before closing a workspace. Default: true.
    pub confirm_close: bool,
    /// Ask for a tab name before creating a new tab. Default: true.
    pub prompt_new_tab_name: bool,
    /// Ask for a workspace name before interactive creation. Default: false.
    pub prompt_new_workspace_name: bool,
    /// Draw borders around split panes. Default: true.
    pub pane_borders: bool,
    /// Keep split panes visually separated instead of sharing divider borders. Default: true.
    pub pane_gaps: bool,
    /// Show agent labels in split pane borders when no manual pane label is set. Default: false.
    pub show_agent_labels_on_pane_borders: bool,
    /// Hide the tab row when the workspace has one tab. Default: false.
    pub hide_tab_bar_when_single_tab: bool,
    /// Agent sidebar ordering. Saved values are "spaces" or "priority". Default: "spaces".
    pub agent_panel_sort: AgentPanelSortConfig,
    /// Expanded sidebar row composition.
    pub sidebar: SidebarConfig,
    /// Accent color for highlights, borders, and navigation UI.
    /// Accepts hex (#89b4fa), named colors (cyan, blue), or RGB (rgb(137,180,250)).
    pub accent: String,
    /// Optional visual toast notifications for background workspace events.
    pub toast: ToastConfig,
    /// Play sounds when agents change state in background workspaces.
    pub sound: SoundConfig,
}

/// Cursor shape (DECSCUSR) used for the forced IME anchor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImeCursorShape {
    Block,
    #[default]
    SteadyBlock,
    Underline,
    SteadyUnderline,
    Bar,
    SteadyBar,
}

impl ImeCursorShape {
    /// Convert to DECSCUSR parameter (1–6).
    pub fn to_decscusr(self) -> u8 {
        match self {
            Self::Block => 1,
            Self::SteadyBlock => 2,
            Self::Underline => 3,
            Self::SteadyUnderline => 4,
            Self::Bar => 5,
            Self::SteadyBar => 6,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct AdvancedConfig {
    /// Maximum scrollback buffer size in bytes retained per pane terminal. Default: 10000000.
    #[serde(alias = "scrollback_lines")]
    pub scrollback_limit_bytes: usize,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct RemoteConfig {
    /// Add keepalive fallbacks and private connection reuse for `herdr --remote`.
    /// Set false to run plain ssh unchanged. Default: true.
    pub manage_ssh_config: bool,
}

impl Default for RemoteConfig {
    fn default() -> Self {
        Self {
            manage_ssh_config: true,
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct ExperimentalConfig {
    /// Allow launching herdr inside an existing herdr pane. Default: false.
    pub allow_nested: bool,
    /// Experimental local Kitty graphics rendering for attached clients. Default: false.
    pub kitty_graphics: bool,
    /// Persist pane screen history to session-history.json. Default: false.
    pub pane_history: bool,
    /// Expose the focused pane's cursor anchor to the outer terminal even when
    /// the pane requested `?25l`, so macOS native input methods keep tracking
    /// the candidate window when TUIs paint their own cursor (Claude Code, pi,
    /// codex, etc.). Default: false.
    ///
    /// When the pane reports no cursor position, falls back to the pane's
    /// top-left so a stable IME anchor is always available.
    ///
    /// Trade-off when enabled: an extra hardware cursor will be visible in the
    /// outer terminal for apps that hide the cursor without painting a
    /// replacement (vim normal mode, etc.). See #149.
    pub reveal_hidden_cursor_for_cjk_ime: bool,
    /// Restrict `reveal_hidden_cursor_for_cjk_ime` to focused panes whose
    /// detected agent matches one of these names (case-insensitive). Empty
    /// list means apply to any focused pane. Unknown agent names are ignored;
    /// if the list contains no valid names, the reveal does not apply.
    /// Accepted names: pi, claude, codex, gemini, cursor, devin, cline,
    /// opencode, copilot, kimi, kiro, droid, amp, grok, hermes, kilo,
    /// qodercli, qoder, maki.
    /// Default: empty.
    pub cjk_ime_agents: Vec<String>,
    /// Cursor shape rendered for the IME anchor when
    /// `reveal_hidden_cursor_for_cjk_ime` is enabled. Default: "steady_block".
    pub cjk_ime_cursor_shape: ImeCursorShape,
    /// While prefix mode is active, temporarily switch the macOS host input
    /// source to an ASCII-capable keyboard layout so prefix commands are read
    /// as ASCII even when a CJK IME is active, then restore the previous input
    /// source when prefix mode exits. macOS only; a no-op elsewhere and a
    /// best-effort no-op if the switch fails. Default: false.
    pub switch_ascii_input_source_in_prefix: bool,
}

impl Default for KeysConfig {
    fn default() -> Self {
        Self {
            prefix: "ctrl+b".into(),
            help: BindingConfig::one("prefix+?"),
            settings: BindingConfig::one("prefix+s"),
            new_workspace: BindingConfig::one("prefix+shift+n"),
            new_worktree: BindingConfig::one("prefix+shift+g"),
            open_worktree: BindingConfig::empty(),
            remove_worktree: BindingConfig::empty(),
            rename_workspace: BindingConfig::one("prefix+shift+w"),
            close_workspace: BindingConfig::one("prefix+shift+d"),
            workspace_picker: BindingConfig::one("prefix+w"),
            goto: BindingConfig::one("prefix+g"),
            navigate_workspace_up: BindingConfig::one("up"),
            navigate_workspace_down: BindingConfig::one("down"),
            navigate_pane_left: BindingConfig::one("h"),
            navigate_pane_down: BindingConfig::one("j"),
            navigate_pane_up: BindingConfig::one("k"),
            navigate_pane_right: BindingConfig::one("l"),
            detach: BindingConfig::one("prefix+q"),
            reload_config: BindingConfig::one("prefix+shift+r"),
            open_notification_target: BindingConfig::one("prefix+o"),
            previous_workspace: BindingConfig::empty(),
            next_workspace: BindingConfig::empty(),
            previous_agent: BindingConfig::empty(),
            next_agent: BindingConfig::empty(),
            focus_agent: BindingConfig::empty(),
            remote_image_paste: "ctrl+v".into(),
            new_tab: BindingConfig::one("prefix+c"),
            new_chat_tab: BindingConfig::empty(),
            rename_tab: BindingConfig::one("prefix+shift+t"),
            previous_tab: BindingConfig::one("prefix+p"),
            next_tab: BindingConfig::one("prefix+n"),
            switch_tab: BindingConfig::one("prefix+1..9"),
            switch_workspace: BindingConfig::empty(),
            close_tab: BindingConfig::one("prefix+shift+x"),
            rename_pane: BindingConfig::one("prefix+shift+p"),
            edit_scrollback: BindingConfig::one("prefix+e"),
            copy_mode: BindingConfig::one("prefix+["),
            focus_pane_left: BindingConfig::one("prefix+h"),
            focus_pane_down: BindingConfig::one("prefix+j"),
            focus_pane_up: BindingConfig::one("prefix+k"),
            focus_pane_right: BindingConfig::one("prefix+l"),
            swap_pane_left: BindingConfig::one("prefix+shift+h"),
            swap_pane_down: BindingConfig::one("prefix+shift+j"),
            swap_pane_up: BindingConfig::one("prefix+shift+k"),
            swap_pane_right: BindingConfig::one("prefix+shift+l"),
            cycle_pane_next: BindingConfig::one("prefix+tab"),
            cycle_pane_previous: BindingConfig::one("prefix+shift+tab"),
            last_pane: BindingConfig::empty(),
            split_vertical: BindingConfig::one("prefix+v"),
            split_horizontal: BindingConfig::one("prefix+minus"),
            close_pane: BindingConfig::one("prefix+x"),
            zoom: BindingConfig::one("prefix+z"),
            resize_mode: BindingConfig::one("prefix+r"),
            toggle_sidebar: BindingConfig::one("prefix+b"),
            toggle_file_manager: BindingConfig::one("prefix+f"),
            agent_attachment_picker: BindingConfig::one("prefix+a"),
            indexed: IndexedKeysConfig::default(),
            command: Vec::new(),
            user_fields: BTreeSet::new(),
        }
    }
}

impl Default for WorktreesConfig {
    fn default() -> Self {
        Self {
            directory: "~/.herdr/worktrees".into(),
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            // TP-TREE-13: the Spaces tab is a three-level tree — repository,
            // checkout, chat — and depth is charged in columns. At 26 a branch
            // name and its chat titles were both truncated before they said
            // anything useful. 30 pays for the two levels of indent and leaves
            // the names intact; `sidebar_max_width` still allows 36.
            sidebar_width: 30,
            sidebar_min_width: 18,
            sidebar_max_width: 36,
            sidebar_start_collapsed: false,
            sidebar_collapsed_mode: SidebarCollapsedModeConfig::Compact,
            mobile_width_threshold: DEFAULT_MOBILE_WIDTH_THRESHOLD,
            mouse_capture: true,
            copy_on_select: true,
            host_cursor: HostCursorModeConfig::Auto,
            right_click_passthrough_modifier: RightClickPassthroughModifierConfig::default(),
            redraw_on_focus_gained: true,
            mouse_scroll_lines: None,
            confirm_close: true,
            prompt_new_tab_name: true,
            prompt_new_workspace_name: false,
            pane_borders: true,
            pane_gaps: true,
            show_agent_labels_on_pane_borders: false,
            hide_tab_bar_when_single_tab: false,
            agent_panel_sort: AgentPanelSortConfig::Spaces,
            sidebar: SidebarConfig::default(),
            accent: "cyan".into(),
            toast: ToastConfig::default(),
            sound: SoundConfig::default(),
        }
    }
}

impl UiConfig {
    pub fn mouse_scroll_lines(&self) -> usize {
        self.mouse_scroll_lines
            .map(NonZeroUsize::get)
            .unwrap_or(DEFAULT_MOUSE_SCROLL_LINES)
    }

    pub fn right_click_passthrough_modifiers(&self) -> Option<KeyModifiers> {
        self.right_click_passthrough_modifier.modifiers()
    }
}

impl Default for ToastConfig {
    fn default() -> Self {
        Self {
            delivery: ToastDelivery::Off,
            delay_seconds: 1,
            herdr: HerdrToastConfig::default(),
            clipboard: ClipboardToastConfig::default(),
        }
    }
}

impl Default for HerdrToastConfig {
    fn default() -> Self {
        Self {
            position: ToastHerdrPosition::BottomRight,
        }
    }
}

impl Default for ClipboardToastConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            position: ToastClipboardPosition::BottomCenter,
        }
    }
}

impl<'de> Deserialize<'de> for ToastConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize, Default)]
        #[serde(default)]
        struct RawToastConfig {
            delivery: Option<ToastDelivery>,
            enabled: Option<bool>,
            delay_seconds: Option<u64>,
            herdr: HerdrToastConfig,
            clipboard: ClipboardToastConfig,
        }

        let raw = RawToastConfig::deserialize(deserializer)?;
        let legacy_delivery = match raw.enabled {
            Some(true) => ToastDelivery::Herdr,
            Some(false) | None => ToastDelivery::Off,
        };
        let delivery = raw.delivery.unwrap_or(legacy_delivery);
        let default = Self::default();
        let delay_seconds = raw.delay_seconds.unwrap_or(default.delay_seconds);
        if delay_seconds > MAX_TOAST_DELAY_SECONDS {
            return Err(de::Error::custom(format!(
                "ui.toast.delay_seconds must be between 0 and {MAX_TOAST_DELAY_SECONDS}"
            )));
        }
        Ok(Self {
            delivery,
            delay_seconds,
            herdr: raw.herdr,
            clipboard: raw.clipboard,
        })
    }
}

impl Default for AdvancedConfig {
    fn default() -> Self {
        Self {
            scrollback_limit_bytes: DEFAULT_SCROLLBACK_LIMIT_BYTES,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TP-ACT-DEFAULT: the default keybinds bind prefix+f to the file manager.
    #[test]
    fn default_binds_prefix_f_to_file_manager() {
        assert_eq!(
            KeysConfig::default().toggle_file_manager,
            BindingConfig::one("prefix+f")
        );
    }

    // TP-M1.1-KEYBIND: M1 owns one configurable prefix action and must use the
    // existing user-first conflict registry instead of hard-coding input.
    #[test]
    fn default_binds_prefix_a_to_agent_attachment_picker_without_conflict() {
        let defaults = Config::default();
        assert_eq!(
            defaults.keys.agent_attachment_picker,
            BindingConfig::one("prefix+a")
        );
        assert_eq!(
            defaults
                .keybinds()
                .agent_attachment_picker
                .label()
                .as_deref(),
            Some("prefix+a")
        );

        let displaced: Config = toml::from_str(
            r#"
[keys]
next_tab = "prefix+a"
"#,
        )
        .unwrap();
        let displaced = displaced.keybinds();
        assert_eq!(displaced.next_tab.label().as_deref(), Some("prefix+a"));
        assert!(displaced.agent_attachment_picker.bindings.is_empty());
    }

    #[test]
    fn update_config_defaults_and_parses() {
        let default_config = Config::default();
        assert_eq!(default_config.update.channel, default_update_channel());
        assert!(default_config.update.version_check);
        assert!(default_config.update.manifest_check);

        let toml = r#"
[update]
channel = "preview"
version_check = false
manifest_check = false
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.update.channel, UpdateChannelConfig::Preview);
        assert_eq!(config.update.channel.as_str(), "preview");
        assert!(!config.update.version_check);
        assert!(!config.update.manifest_check);
    }

    #[cfg(windows)]
    #[test]
    fn windows_update_config_defaults_to_preview() {
        let empty: Config = toml::from_str("").unwrap();
        let without_update_channel: Config =
            toml::from_str("[update]\nversion_check = false").unwrap();

        assert_eq!(
            Config::default().update.channel,
            UpdateChannelConfig::Preview
        );
        assert_eq!(empty.update.channel, UpdateChannelConfig::Preview);
        assert_eq!(
            without_update_channel.update.channel,
            UpdateChannelConfig::Preview
        );
    }

    #[test]
    fn terminal_default_shell_defaults_empty_and_parses() {
        let default_config = Config::default();
        assert!(default_config.terminal.default_shell.is_empty());
        assert_eq!(default_config.terminal.shell_mode, ShellModeConfig::Auto);

        let toml = r#"
[terminal]
default_shell = "nu"
shell_mode = "non_login"
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.terminal.default_shell, "nu");
        assert_eq!(config.terminal.shell_mode, ShellModeConfig::NonLogin);
    }

    #[test]
    fn terminal_new_cwd_defaults_follow_and_parses() {
        let default_config = Config::default();
        assert_eq!(
            default_config.terminal.new_cwd,
            NewTerminalCwdConfig::Follow
        );

        let config: Config = toml::from_str(
            r#"
[terminal]
new_cwd = "home"
"#,
        )
        .unwrap();
        assert_eq!(config.terminal.new_cwd, NewTerminalCwdConfig::Home);

        let config: Config = toml::from_str(
            r#"
[terminal]
new_cwd = "~/Projects"
"#,
        )
        .unwrap();
        assert_eq!(
            config.terminal.new_cwd,
            NewTerminalCwdConfig::Path("~/Projects".into())
        );
    }

    #[test]
    fn resume_agents_on_restore_defaults_on_and_parses() {
        let default_config = Config::default();
        assert!(default_config.session.resume_agents_on_restore);

        let toml = r#"
[session]
resume_agents_on_restore = false
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert!(!config.session.resume_agents_on_restore);
    }

    // Preview placement: absent section → implemented default; every declared
    // mode round-trips through its persisted kebab-case value; only the
    // default is available (the rest are announced "soon" and must be treated
    // as fallback by runtime consumers).
    #[test]
    fn preview_placement_defaults_parses_and_roundtrips() {
        let default_config = Config::default();
        assert_eq!(
            default_config.preview.placement,
            PreviewPlacement::SameScreenHalf
        );
        assert!(default_config.preview.placement.is_available());

        let toml_src = r#"
[preview]
placement = "other-monitor-per-chat"
"#;
        let config: Config = toml::from_str(toml_src).unwrap();
        assert_eq!(
            config.preview.placement,
            PreviewPlacement::OtherMonitorPerChat
        );
        assert!(!config.preview.placement.is_available());

        for mode in PreviewPlacement::ALL {
            let parsed: PreviewConfig =
                toml::from_str(&format!("placement = \"{}\"", mode.config_value())).unwrap();
            assert_eq!(parsed.placement, *mode, "config_value must round-trip");
        }
        assert_eq!(
            PreviewPlacement::ALL
                .iter()
                .filter(|mode| mode.is_available())
                .count(),
            1,
            "exactly one implemented mode until a soon backend lands"
        );
    }

    #[test]
    fn agent_panel_sort_config_parses_alias_and_defaults() {
        assert_eq!(
            Config::default().ui.agent_panel_sort,
            AgentPanelSortConfig::Spaces
        );

        let toml = r#"
[ui]
agent_panel_sort = "priority"
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.ui.agent_panel_sort, AgentPanelSortConfig::Priority);

        let toml = r#"
[ui]
agent_panel_sort = "workspaces"
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.ui.agent_panel_sort, AgentPanelSortConfig::Spaces);

        let toml = r#"
[ui]
agent_panel_scope = "current"
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.ui.agent_panel_sort, AgentPanelSortConfig::Spaces);
    }

    #[test]
    fn pane_appearance_defaults_and_parse() {
        let default_config = Config::default();
        assert!(default_config.ui.pane_borders);
        assert!(default_config.ui.pane_gaps);
        assert!(!default_config.ui.show_agent_labels_on_pane_borders);
        assert!(!default_config.ui.hide_tab_bar_when_single_tab);

        let toml = r#"
[ui]
pane_borders = false
pane_gaps = true
show_agent_labels_on_pane_borders = true
hide_tab_bar_when_single_tab = true
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert!(!config.ui.pane_borders);
        assert!(config.ui.pane_gaps);
        assert!(config.ui.show_agent_labels_on_pane_borders);
        assert!(config.ui.hide_tab_bar_when_single_tab);
    }

    #[test]
    fn worktrees_directory_defaults_and_parses() {
        let default_config = Config::default();
        assert_eq!(default_config.worktrees.directory, "~/.herdr/worktrees");

        let toml = r#"
[worktrees]
directory = "~/Projects/herdr-worktrees"
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.worktrees.directory, "~/Projects/herdr-worktrees");
    }

    #[test]
    fn prompt_new_tab_name_defaults_on_and_parses() {
        let default_config = Config::default();
        assert!(default_config.ui.prompt_new_tab_name);

        let toml = r#"
[ui]
prompt_new_tab_name = false
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert!(!config.ui.prompt_new_tab_name);
    }

    #[test]
    fn prompt_new_workspace_name_defaults_off_and_parses() {
        let default_config = Config::default();
        assert!(!default_config.ui.prompt_new_workspace_name);

        let toml = r#"
[ui]
prompt_new_workspace_name = true
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert!(config.ui.prompt_new_workspace_name);
    }

    #[test]
    fn reveal_hidden_cursor_for_cjk_ime_default_off_and_parse() {
        let default_config = Config::default();
        assert!(!default_config.experimental.reveal_hidden_cursor_for_cjk_ime);

        let toml = r#"
[experimental]
reveal_hidden_cursor_for_cjk_ime = true
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert!(config.experimental.reveal_hidden_cursor_for_cjk_ime);
    }

    #[test]
    fn switch_ascii_input_source_in_prefix_default_off_and_parse() {
        let default_config = Config::default();
        assert!(
            !default_config
                .experimental
                .switch_ascii_input_source_in_prefix
        );

        let toml = r#"
[experimental]
switch_ascii_input_source_in_prefix = true
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert!(config.experimental.switch_ascii_input_source_in_prefix);
    }

    #[test]
    fn cjk_ime_cursor_shape_default_steady_block_and_parse() {
        let default_config = Config::default();
        assert_eq!(
            default_config.experimental.cjk_ime_cursor_shape,
            ImeCursorShape::SteadyBlock
        );

        let toml = r#"
[experimental]
cjk_ime_cursor_shape = "bar"
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(
            config.experimental.cjk_ime_cursor_shape,
            ImeCursorShape::Bar
        );
    }

    #[test]
    fn cjk_ime_agents_default_empty_and_parse() {
        let default_config = Config::default();
        assert!(default_config.experimental.cjk_ime_agents.is_empty());

        let toml = r#"
[experimental]
cjk_ime_agents = ["claude", "codex"]
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(
            config.experimental.cjk_ime_agents,
            vec!["claude".to_string(), "codex".to_string()]
        );
    }

    #[test]
    fn sidebar_bounds_default_and_parse() {
        let default_config = Config::default();
        assert_eq!(default_config.ui.sidebar_min_width, 18);
        assert_eq!(default_config.ui.sidebar_max_width, 36);
        assert_eq!(
            default_config.ui.mobile_width_threshold,
            DEFAULT_MOBILE_WIDTH_THRESHOLD
        );

        let toml = r#"
[ui]
sidebar_min_width = 12
sidebar_max_width = 80
mobile_width_threshold = 96
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.ui.sidebar_min_width, 12);
        assert_eq!(config.ui.sidebar_max_width, 80);
        assert_eq!(config.ui.mobile_width_threshold, 96);
    }

    #[test]
    fn sidebar_start_collapsed_defaults_off_and_parses_on() {
        let default_config = Config::default();
        assert!(!default_config.ui.sidebar_start_collapsed);

        let toml = r#"
[ui]
sidebar_start_collapsed = true
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert!(config.ui.sidebar_start_collapsed);
    }

    #[test]
    fn sidebar_collapsed_mode_defaults_compact_and_parses_hidden() {
        let default_config = Config::default();
        assert_eq!(
            default_config.ui.sidebar_collapsed_mode,
            SidebarCollapsedModeConfig::Compact
        );

        let toml = r#"
[ui]
sidebar_collapsed_mode = "hidden"
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(
            config.ui.sidebar_collapsed_mode,
            SidebarCollapsedModeConfig::Hidden
        );
    }

    #[test]
    fn validated_sidebar_bounds_rejects_inverted() {
        assert_eq!(validated_sidebar_bounds(18, 36), Some((18, 36)));
        assert_eq!(validated_sidebar_bounds(20, 20), Some((20, 20)));
        assert_eq!(validated_sidebar_bounds(0, u16::MAX), Some((0, u16::MAX)));
        assert_eq!(validated_sidebar_bounds(50, 30), None);
        assert_eq!(validated_sidebar_bounds(u16::MAX, 0), None);
    }

    #[test]
    fn mouse_capture_default_on_and_parse() {
        let default_config = Config::default();
        assert!(default_config.ui.mouse_capture);

        let toml = r#"
[ui]
mouse_capture = false
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert!(!config.ui.mouse_capture);
    }

    #[test]
    fn copy_on_select_default_on_and_parse() {
        let default_config = Config::default();
        assert!(default_config.ui.copy_on_select);

        let toml = r#"
[ui]
copy_on_select = false
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert!(!config.ui.copy_on_select);
    }

    #[test]
    fn right_click_passthrough_modifier_defaults_off_and_parses() {
        let default_config = Config::default();
        assert_eq!(default_config.ui.right_click_passthrough_modifiers(), None);

        for value in ["", "off", "none", "disabled"] {
            let toml = format!(
                r#"
[ui]
right_click_passthrough_modifier = "{value}"
"#
            );
            let config: Config = toml::from_str(&toml).unwrap();
            assert_eq!(
                config.ui.right_click_passthrough_modifiers(),
                None,
                "value {value:?} should disable passthrough"
            );
        }

        for (value, expected) in [
            ("ctrl", KeyModifiers::CONTROL),
            ("control", KeyModifiers::CONTROL),
            ("alt", KeyModifiers::ALT),
            ("option", KeyModifiers::ALT),
            ("cmd", KeyModifiers::SUPER),
            ("command", KeyModifiers::SUPER),
            ("super", KeyModifiers::SUPER),
            ("meta", KeyModifiers::META),
            ("hyper", KeyModifiers::HYPER),
        ] {
            let toml = format!(
                r#"
[ui]
right_click_passthrough_modifier = "{value}"
"#
            );
            let config: Config = toml::from_str(&toml).unwrap();
            assert_eq!(
                config.ui.right_click_passthrough_modifiers(),
                Some(expected),
                "value {value:?} should parse"
            );
        }

        let toml = r#"
[ui]
right_click_passthrough_modifier = "cmd+alt"
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(
            config.ui.right_click_passthrough_modifiers(),
            Some(KeyModifiers::SUPER | KeyModifiers::ALT)
        );
    }

    #[test]
    fn right_click_passthrough_modifier_rejects_shift() {
        for value in ["shift", "shift+ctrl", "ctrl+", "ctrl++alt", "banana"] {
            let toml = format!(
                r#"
[ui]
right_click_passthrough_modifier = "{value}"
"#
            );
            assert!(
                toml::from_str::<Config>(&toml).is_err(),
                "value {value:?} should be rejected"
            );
        }
    }

    #[test]
    fn redraw_on_focus_gained_default_on_and_parse() {
        let default_config = Config::default();
        assert!(default_config.ui.redraw_on_focus_gained);

        let toml = r#"
[ui]
redraw_on_focus_gained = false
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert!(!config.ui.redraw_on_focus_gained);
    }

    #[test]
    fn mouse_scroll_lines_defaults_to_three_and_parses() {
        let default_config = Config::default();
        assert_eq!(
            default_config.ui.mouse_scroll_lines(),
            DEFAULT_MOUSE_SCROLL_LINES
        );

        let toml = r#"
[ui]
mouse_scroll_lines = 1
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.ui.mouse_scroll_lines(), 1);
    }

    #[test]
    fn mouse_scroll_lines_rejects_zero() {
        let toml = r#"
[ui]
mouse_scroll_lines = 0
"#;
        assert!(toml::from_str::<Config>(toml).is_err());
    }

    #[test]
    fn toast_config_parses() {
        let toml = r#"
[ui.toast]
delivery = "terminal"
delay_seconds = 2

[ui.toast.herdr]
position = "top-left"

[ui.toast.clipboard]
enabled = false
position = "top-center"
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.ui.toast.delivery, ToastDelivery::Terminal);
        assert_eq!(config.ui.toast.delay_seconds, 2);
        assert_eq!(config.ui.toast.herdr.position, ToastHerdrPosition::TopLeft);
        assert!(!config.ui.toast.clipboard.enabled);
        assert_eq!(
            config.ui.toast.clipboard.position,
            ToastClipboardPosition::TopCenter
        );
    }

    #[test]
    fn toast_config_defaults_preserve_existing_behavior_with_delay() {
        let config = Config::default();
        assert_eq!(config.ui.toast.delivery, ToastDelivery::Off);
        assert_eq!(config.ui.toast.delay_seconds, 1);
        assert_eq!(
            config.ui.toast.herdr.position,
            ToastHerdrPosition::BottomRight
        );
        assert!(config.ui.toast.clipboard.enabled);
        assert_eq!(
            config.ui.toast.clipboard.position,
            ToastClipboardPosition::BottomCenter
        );
    }

    #[test]
    fn toast_config_parses_system_delivery() {
        let toml = r#"
[ui.toast]
delivery = "system"
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.ui.toast.delivery, ToastDelivery::System);
    }

    #[test]
    fn toast_config_legacy_enabled_true_maps_to_herdr() {
        let toml = r#"
[ui.toast]
enabled = true
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.ui.toast.delivery, ToastDelivery::Herdr);
    }

    #[test]
    fn toast_config_legacy_enabled_false_maps_to_off() {
        let toml = r#"
[ui.toast]
enabled = false
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.ui.toast.delivery, ToastDelivery::Off);
    }

    #[test]
    fn toast_config_delivery_wins_over_legacy_enabled() {
        let toml = r#"
[ui.toast]
enabled = true
delivery = "terminal"
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.ui.toast.delivery, ToastDelivery::Terminal);
    }

    #[test]
    fn toast_config_rejects_unbounded_delay() {
        let toml = format!(
            r#"
[ui.toast]
delay_seconds = {}
"#,
            MAX_TOAST_DELAY_SECONDS + 1
        );

        let error = toml::from_str::<Config>(&toml).unwrap_err().to_string();

        assert!(error.contains("ui.toast.delay_seconds must be between 0 and 3600"));
    }

    #[test]
    fn missing_onboarding_shows_setup() {
        let config = Config::default();
        assert!(config.should_show_onboarding());
    }

    #[test]
    fn onboarding_false_skips_setup() {
        let config: Config = toml::from_str("onboarding = false").unwrap();
        assert!(!config.should_show_onboarding());
    }

    #[test]
    fn advanced_defaults_include_scrollback_limit_bytes() {
        let config = Config::default();
        assert_eq!(
            config.advanced.scrollback_limit_bytes,
            DEFAULT_SCROLLBACK_LIMIT_BYTES
        );
    }

    #[test]
    fn pane_history_persistence_is_opt_in() {
        assert!(!Config::default().experimental.pane_history);

        let toml = r#"
[experimental]
pane_history = true
"#;
        let config: Config = toml::from_str(toml).unwrap();

        assert!(config.experimental.pane_history);
    }

    #[test]
    fn kitty_graphics_default_off_and_parse() {
        let config = Config::default();
        assert!(!config.experimental.kitty_graphics);

        let toml = r#"
[experimental]
kitty_graphics = true
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert!(config.experimental.kitty_graphics);
    }

    #[test]
    fn experimental_config_parses() {
        let toml = r#"
[experimental]
allow_nested = true
kitty_graphics = true
pane_history = true
switch_ascii_input_source_in_prefix = true
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert!(config.experimental.allow_nested);
        assert!(config.experimental.kitty_graphics);
        assert!(config.experimental.pane_history);
        assert!(config.experimental.switch_ascii_input_source_in_prefix);
    }

    #[test]
    fn advanced_config_parses() {
        let toml = r#"
[advanced]
scrollback_limit_bytes = 12345
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.advanced.scrollback_limit_bytes, 12345);
    }

    #[test]
    fn advanced_legacy_scrollback_lines_alias_parses() {
        let toml = r#"
[advanced]
scrollback_lines = 12345
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.advanced.scrollback_limit_bytes, 12345);
    }

    #[test]
    fn spaces_rules_drop_unusable_entries() {
        let config: Config = toml::from_str(
            r#"
[[spaces.split]]
repo = ""
match = ["a"]
key = "k1"

[[spaces.split]]
repo = "relative/path"
match = ["a"]
key = "k2"

[[spaces.split]]
repo = "/repo/ok"
match = ["a"]
key = ""

[[spaces.split]]
repo = "/repo/ok"
match = ["   "]
key = "k4"

[[spaces.split]]
repo = "/repo/ok"
match = ["feat/*"]
key = "k5"
"#,
        )
        .expect("config parses");

        let rules = config.spaces.rules();
        assert_eq!(
            rules
                .iter()
                .map(|rule| rule.key.as_str())
                .collect::<Vec<_>>(),
            vec!["k5"],
            "only the usable rule survives"
        );
        assert_eq!(
            config.spaces.diagnostics().len(),
            4,
            "each dropped entry is reported: {:?}",
            config.spaces.diagnostics()
        );
    }

    #[test]
    fn space_split_entry_problem_rejects_unusable_entries() {
        let usable = SpaceSplitEntry {
            repo: "~/panel".into(),
            patterns: vec!["feat/*".into()],
            key: "panel:feat".into(),
            label: String::new(),
            icon: String::new(),
        };
        assert_eq!(space_split_entry_problem(&usable), None);

        let mut no_key = usable.clone();
        no_key.key = "  ".into();
        assert_eq!(space_split_entry_problem(&no_key), Some("has no key"));

        let mut relative = usable.clone();
        relative.repo = "panel".into();
        assert_eq!(
            space_split_entry_problem(&relative),
            Some("repo is not an absolute path")
        );

        let mut no_pattern = usable.clone();
        no_pattern.patterns = vec![String::new()];
        assert_eq!(
            space_split_entry_problem(&no_pattern),
            Some("has no usable match pattern")
        );
    }

    #[test]
    fn spaces_rule_label_falls_back_to_key() {
        let config: Config = toml::from_str(
            r#"
[[spaces.split]]
repo = "/repo/ok"
match = ["feat/*"]
key = "panel:feat"
"#,
        )
        .expect("config parses");
        assert_eq!(config.spaces.rules()[0].label, "panel:feat");
    }

    #[test]
    fn spaces_diagnostics_report_duplicate_keys() {
        let config: Config = toml::from_str(
            r#"
[[spaces.split]]
repo = "/repo/ok"
match = ["feat/*"]
key = "panel:dup"

[[spaces.split]]
repo = "/repo/ok"
match = ["fix/*"]
key = "panel:dup"
"#,
        )
        .expect("config parses");

        assert!(
            config
                .spaces
                .diagnostics()
                .iter()
                .any(|d| d.contains("already used by an earlier rule")),
            "a duplicate key must be reported: {:?}",
            config.spaces.diagnostics()
        );
    }

    // TP-SPLIT-CONF-04: the same key in two different repositories is a
    // deliberate cross-repository module, not a typo — the sidebar gathers
    // both repositories' checkouts under one group, so no warning banner.
    #[test]
    fn spaces_diagnostics_ignore_shared_key_across_repos() {
        let config: Config = toml::from_str(
            r#"
[[spaces.split]]
repo = "/repo/fork"
match = ["herdr-web"]
key = "herdr:web"

[[spaces.split]]
repo = "/repo/pwa"
match = ["main"]
key = "herdr:web"
"#,
        )
        .expect("config parses");

        assert!(
            config.spaces.diagnostics().is_empty(),
            "two repositories may share one module key on purpose: {:?}",
            config.spaces.diagnostics()
        );
    }

    // TP-PROJ-CONF-01.
    #[test]
    fn spaces_projects_drop_unusable_entries() {
        let config: Config = toml::from_str(
            r#"
[[spaces.project]]
name = "keysiz"
repos = ["/repo/a"]

[[spaces.project]]
key = "project:rel"
repos = ["relative/path"]

[[spaces.project]]
key = "project:bos"

[[spaces.project]]
key = "project:ok"
name = "Herdr"
icon = " 🚀 "
repos = ["/repo/a", "  "]
spaces = ["a:t4f", "   "]
"#,
        )
        .expect("config parses");

        let projects = config.spaces.projects();
        assert_eq!(
            projects
                .iter()
                .map(|project| project.key.as_str())
                .collect::<Vec<_>>(),
            vec!["project:ok"],
            "only the usable project survives"
        );
        assert_eq!(projects[0].name, "Herdr");
        assert_eq!(projects[0].icon.as_deref(), Some("🚀"));
        assert_eq!(
            projects[0].repo_roots,
            vec![std::path::PathBuf::from("/repo/a")],
            "blank repos entries are dropped, not treated as members"
        );
        assert_eq!(projects[0].space_keys, vec!["a:t4f".to_string()]);
        assert_eq!(
            config.spaces.diagnostics().len(),
            3,
            "each dropped project is reported: {:?}",
            config.spaces.diagnostics()
        );
    }

    // TP-PROJ-CONF-01.
    #[test]
    fn space_project_entry_problem_rejects_unusable_entries() {
        let usable = SpaceProjectEntry {
            name: String::new(),
            key: "project:x".into(),
            icon: String::new(),
            repos: vec!["~/panel".into()],
            spaces: Vec::new(),
        };
        assert_eq!(space_project_entry_problem(&usable), None);

        let mut no_key = usable.clone();
        no_key.key = "  ".into();
        assert_eq!(space_project_entry_problem(&no_key), Some("has no key"));

        let mut relative = usable.clone();
        relative.repos = vec!["panel".into()];
        assert_eq!(
            space_project_entry_problem(&relative),
            Some("has a repo that is not an absolute path")
        );

        let mut no_members = usable.clone();
        no_members.repos = Vec::new();
        no_members.spaces = vec!["   ".into()];
        assert_eq!(
            space_project_entry_problem(&no_members),
            Some("has no usable repos or spaces member")
        );

        let mut spaces_only = usable.clone();
        spaces_only.repos = Vec::new();
        spaces_only.spaces = vec!["a:t4f".into()];
        assert_eq!(
            space_project_entry_problem(&spaces_only),
            None,
            "a project may consist of promoted spaces alone"
        );
    }

    // TP-PROJ-CONF-02.
    #[test]
    fn projects_diagnostics_report_duplicate_project_keys() {
        let config: Config = toml::from_str(
            r#"
[[spaces.project]]
key = "project:dup"
repos = ["/repo/a"]

[[spaces.project]]
key = "project:dup"
repos = ["/repo/b"]
"#,
        )
        .expect("config parses");

        assert!(
            config
                .spaces
                .diagnostics()
                .iter()
                .any(|d| d.contains("already used by an earlier project")),
            "a duplicate project key must be reported: {:?}",
            config.spaces.diagnostics()
        );
    }

    #[test]
    fn spaces_project_name_falls_back_to_key() {
        let config: Config = toml::from_str(
            r#"
[[spaces.project]]
key = "project:herdr"
repos = ["/repo/a"]
"#,
        )
        .expect("config parses");
        assert_eq!(config.spaces.projects()[0].name, "project:herdr");
        assert_eq!(
            config.spaces.projects()[0].icon,
            None,
            "a blank icon means \"use the configured default\", not an empty glyph"
        );
    }

    // TP-ICON-02 (parse half; the render half lives in ui::sidebar).
    #[test]
    fn split_rules_carry_optional_icons() {
        let config: Config = toml::from_str(
            r#"
[[spaces.split]]
repo = "/repo/ok"
match = ["feat/*"]
key = "panel:feat"
icon = " 🌐 "

[[spaces.split]]
repo = "/repo/ok"
match = ["fix/*"]
key = "panel:fix"
"#,
        )
        .expect("config parses");
        let rules = config.spaces.rules();
        assert_eq!(rules[0].icon.as_deref(), Some("🌐"));
        assert_eq!(rules[1].icon, None);
    }

    // TP-ICON-04.
    #[test]
    fn space_icons_defaults_and_partial_override() {
        let defaults = SpacesConfig::default().icons;
        assert_eq!(
            (
                defaults.project.as_str(),
                defaults.branch.as_str(),
                defaults.chat.as_str()
            ),
            ("📁", "", "💬"),
            "row-kind icons work without any config"
        );

        let config: Config = toml::from_str(
            r#"
[spaces.icons]
chat = "✉"
"#,
        )
        .expect("config parses");
        assert_eq!(config.spaces.icons.chat, "✉");
        assert_eq!(
            config.spaces.icons.branch, "\u{e0a0}",
            "an unmentioned icon keeps its default"
        );
    }

    #[test]
    fn projects_pinned_defaults_empty() {
        let config = Config::default();
        assert!(config.projects.pinned.is_empty());
        assert!(config.projects.diagnostics().is_empty());
    }

    #[test]
    fn projects_pinned_parses_absolute_paths() {
        let toml = r#"
[projects]
pinned = ["/home/a/projects/x", "/home/a/work/y"]
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(
            config.projects.pinned,
            vec![
                "/home/a/projects/x".to_string(),
                "/home/a/work/y".to_string(),
            ]
        );
        assert!(config.projects.diagnostics().is_empty());
    }

    #[test]
    fn projects_pinned_accepts_tilde_rooted_entries() {
        let toml = r#"
[projects]
pinned = ["~/projects/herdr"]
"#;
        let config: Config = toml::from_str(toml).unwrap();
        // `~`-rooted entries are treated as usable (no diagnostic).
        assert!(config.projects.diagnostics().is_empty());
    }

    #[test]
    fn projects_pinned_blank_entries_are_reported() {
        let toml = r#"
[projects]
pinned = ["/home/a/x", "", "   ", "/home/a/y"]
"#;
        let config: Config = toml::from_str(toml).unwrap();
        let diagnostics = config.projects.diagnostics();
        assert_eq!(diagnostics.len(), 2, "two blank entries should warn");
        assert!(
            diagnostics.iter().all(|d| d.contains("blank")),
            "blank entries should be reported as blank: {diagnostics:?}"
        );
    }

    #[test]
    fn projects_pinned_relative_entries_are_reported() {
        let toml = r#"
[projects]
pinned = ["/home/a/abs", "relative/dir", "./also/relative"]
"#;
        let config: Config = toml::from_str(toml).unwrap();
        let diagnostics = config.projects.diagnostics();
        assert_eq!(diagnostics.len(), 2);
        assert!(
            diagnostics
                .iter()
                .all(|d| d.contains("not an absolute path")),
            "relative entries should be reported: {diagnostics:?}"
        );
    }

    #[test]
    fn projects_pinned_trims_before_validation() {
        let toml = r#"
[projects]
pinned = ["  /home/a/x  "]
"#;
        let config: Config = toml::from_str(toml).unwrap();
        // Surrounding whitespace is trimmed before the absolute-path check,
        // so this entry is usable and produces no diagnostic.
        assert!(config.projects.diagnostics().is_empty());
    }

    // P1: pinned_paths() keeps usable entries in order (absolute + tilde),
    // trims surrounding whitespace, and drops blank/relative entries.
    #[test]
    fn projects_pinned_paths_keeps_usable_and_drops_unusable() {
        let toml = r#"
[projects]
pinned = ["/a", "", "  ~/z  ", "relative/dir", "./nope", "  /w  "]
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(
            config.projects.pinned_paths(),
            vec!["/a".to_string(), "~/z".to_string(), "/w".to_string(),]
        );
    }

    #[test]
    fn projects_pinned_paths_default_empty() {
        assert!(Config::default().projects.pinned_paths().is_empty());
    }
}
