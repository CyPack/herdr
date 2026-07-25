use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::common::AgentStatus;
use super::common::SplitDirection;
use super::panes::PaneInfo;
use super::workspaces::WorkspaceWorktreeInfo;
use crate::popup_size::PopupSize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PluginLinkParams {
    pub path: String,
    #[serde(default = "super::common::default_true")]
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<PluginSourceInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema, Default)]
pub struct PluginListParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PluginUnlinkParams {
    pub plugin_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PluginSetEnabledParams {
    pub plugin_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct InstalledPluginInfo {
    pub plugin_id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub min_herdr_version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub manifest_path: String,
    pub plugin_root: String,
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub platforms: Option<Vec<PluginPlatform>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub build: Vec<PluginManifestBuild>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub startup: Vec<PluginManifestStartup>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<PluginManifestAction>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub events: Vec<PluginManifestEventHook>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub panes: Vec<PluginManifestPane>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub link_handlers: Vec<PluginManifestLinkHandler>,
    #[serde(default)]
    pub source: PluginSourceInfo,
    /// Warnings collected at link time or on registry load (e.g. unknown event names,
    /// missing manifest file). Non-fatal — the entry is kept and surfaced by plugin.list.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PluginSourceInfo {
    #[serde(default)]
    pub kind: PluginSourceKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repo: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subdir: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requested_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved_commit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub managed_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub installed_unix_ms: Option<u64>,
}

impl Default for PluginSourceInfo {
    fn default() -> Self {
        Self {
            kind: PluginSourceKind::Local,
            owner: None,
            repo: None,
            subdir: None,
            requested_ref: None,
            resolved_commit: None,
            managed_path: None,
            installed_unix_ms: None,
        }
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema, Default,
)]
#[serde(rename_all = "snake_case")]
pub enum PluginSourceKind {
    #[default]
    Local,
    Github,
}

pub(crate) fn plugin_managed_path_component(value: &str) -> String {
    let slug = readable_plugin_path_slug(value);
    let hash = short_plugin_id_hash_for_path_component(value);
    format!("{slug}-{hash}")
}

fn readable_plugin_path_slug(value: &str) -> String {
    let mut slug = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>();
    while slug.contains("--") {
        slug = slug.replace("--", "-");
    }
    let slug = slug
        .trim_matches(|ch| matches!(ch, '-' | '_' | '.'))
        .to_string();
    let slug = if slug.is_empty() {
        "plugin".to_string()
    } else {
        slug.chars().take(80).collect()
    };
    if has_windows_reserved_stem_for_path_component(&slug) {
        slug.replace('.', "-")
    } else {
        slug
    }
}

pub(crate) fn short_plugin_id_hash_for_path_component(value: &str) -> String {
    use sha2::{Digest, Sha256};

    let digest = Sha256::digest(value.as_bytes());
    let mut hash = String::with_capacity(12);
    for byte in &digest[..6] {
        use std::fmt::Write as _;
        let _ = write!(hash, "{byte:02x}");
    }
    hash
}

pub(crate) fn has_windows_reserved_stem_for_path_component(value: &str) -> bool {
    let stem = value.split('.').next().unwrap_or(value);
    matches!(
        stem.to_ascii_uppercase().as_str(),
        "CON"
            | "PRN"
            | "AUX"
            | "NUL"
            | "COM1"
            | "COM2"
            | "COM3"
            | "COM4"
            | "COM5"
            | "COM6"
            | "COM7"
            | "COM8"
            | "COM9"
            | "LPT1"
            | "LPT2"
            | "LPT3"
            | "LPT4"
            | "LPT5"
            | "LPT6"
            | "LPT7"
            | "LPT8"
            | "LPT9"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_managed_path_component_is_windows_safe_and_collision_free() {
        let dotdot = plugin_managed_path_component("..");
        assert_ne!(dotdot, ".");
        assert_ne!(dotdot, "..");
        assert!(dotdot.starts_with("plugin-"));
        assert_ne!(
            plugin_managed_path_component("a:b"),
            plugin_managed_path_component("a_b")
        );
        assert_ne!(plugin_managed_path_component("con"), "con");
        assert!(!plugin_managed_path_component("example.").ends_with('.'));
        assert!(plugin_managed_path_component("con.example").starts_with("con-example-"));
        assert!(plugin_managed_path_component("aux.plugin").starts_with("aux-plugin-"));
        assert!(plugin_managed_path_component("nul.x").starts_with("nul-x-"));
        assert!(plugin_managed_path_component("com1.tool").starts_with("com1-tool-"));
    }

    #[test]
    fn plugin_managed_path_component_keeps_readable_slug() {
        let component = plugin_managed_path_component("example.worktree-bootstrap");
        assert!(component.starts_with("example.worktree-bootstrap-"));
        assert!(component.len() <= "example.worktree-bootstrap-".len() + 12);
    }

    #[test]
    fn plugin_managed_path_component_hash_distinguishes_same_slug_shape() {
        assert_ne!(
            plugin_managed_path_component("example:a"),
            plugin_managed_path_component("example/a")
        );
    }

    fn action_for(file_extensions: &[&str]) -> PluginActionInfo {
        PluginActionInfo {
            plugin_id: "example.plugin".to_owned(),
            action_id: "open".to_owned(),
            title: "Open in New Tab".to_owned(),
            description: None,
            contexts: vec![PluginActionContext::File],
            file_extensions: file_extensions
                .iter()
                .map(|extension| (*extension).to_owned())
                .collect(),
            command: vec!["/bin/true".to_owned()],
            platforms: None,
        }
    }

    fn paths(names: &[&str]) -> Vec<std::path::PathBuf> {
        names.iter().map(std::path::PathBuf::from).collect()
    }

    // TP-FOPEN-01: an action that names its extensions is offered only for
    // those files. Without this the spreadsheet plugin was offered on a PDF,
    // and choosing it launched the spreadsheet editor on a file it cannot
    // read — an empty tab with nothing to explain it.
    #[test]
    fn an_action_that_names_extensions_matches_only_those_files() {
        let action = action_for(&["xlsx", "csv"]);
        assert!(action.matches_paths(&paths(["/tmp/report.xlsx"].as_slice())));
        assert!(action.matches_paths(&paths(["/tmp/data.csv"].as_slice())));
        assert!(!action.matches_paths(&paths(["/tmp/manual.pdf"].as_slice())));
        assert!(!action.matches_paths(&paths(["/tmp/photo.png"].as_slice())));
    }

    // TP-FOPEN-02: an action that names no extensions still matches every
    // file. Every manifest installed today omits the field, so reading an
    // empty list as "nothing" would silently disable every existing plugin —
    // a second defect rather than a fix.
    #[test]
    fn an_action_without_extensions_matches_every_file() {
        let action = action_for(&[]);
        assert!(action.matches_paths(&paths(["/tmp/manual.pdf"].as_slice())));
        assert!(action.matches_paths(&paths(["/tmp/Makefile"].as_slice())));
        assert!(action.matches_paths(&paths(["/tmp/a.xlsx", "/tmp/b.png"].as_slice())));
    }

    // TP-FOPEN-03: extension comparison ignores case. Downloaded files arrive
    // named `.XLSX` often enough that a case-sensitive match would look like
    // the plugin randomly failing to appear.
    #[test]
    fn extension_matching_ignores_case() {
        let action = action_for(&["xlsx"]);
        assert!(action.matches_paths(&paths(["/tmp/REPORT.XLSX"].as_slice())));
        assert!(action.matches_paths(&paths(["/tmp/Report.Xlsx"].as_slice())));
    }

    // TP-FOPEN-04: every path in the selection has to match. Offering the
    // action on a partly-matching selection runs the wrong program on the
    // files that did not match, which is the original defect in miniature.
    #[test]
    fn a_partly_matching_selection_does_not_match() {
        let action = action_for(&["xlsx"]);
        assert!(action.matches_paths(&paths(["/tmp/a.xlsx", "/tmp/b.xlsx"].as_slice())));
        assert!(!action.matches_paths(&paths(["/tmp/a.xlsx", "/tmp/b.pdf"].as_slice())));
    }

    // TP-FOPEN-05: a file with no extension matches nothing that named one,
    // and does not panic on the way to that answer.
    #[test]
    fn a_file_without_an_extension_matches_only_an_unrestricted_action() {
        assert!(!action_for(&["xlsx"]).matches_paths(&paths(["/tmp/Makefile"].as_slice())));
        assert!(action_for(&[]).matches_paths(&paths(["/tmp/Makefile"].as_slice())));
    }

    // TP-FOPEN-06: the last dotted segment is the extension, matching
    // `Path::extension`. Recorded rather than assumed: `archive.tar.gz` is a
    // `gz`, and an action naming `tar` is not offered on it.
    #[test]
    fn a_multi_dotted_name_matches_its_last_segment() {
        assert!(action_for(&["gz"]).matches_paths(&paths(["/tmp/archive.tar.gz"].as_slice())));
        assert!(!action_for(&["tar"]).matches_paths(&paths(["/tmp/archive.tar.gz"].as_slice())));
    }

    // TP-FOPEN-07: an empty selection matches nothing. There is no file to
    // run the action on, and "every path matches" is vacuously true for an
    // empty list — the trap this closes.
    #[test]
    fn an_empty_selection_matches_nothing() {
        assert!(!action_for(&["xlsx"]).matches_paths(&[]));
        assert!(!action_for(&[]).matches_paths(&[]));
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PluginManifestBuild {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub platforms: Option<Vec<PluginPlatform>>,
    pub command: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PluginManifestStartup {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub platforms: Option<Vec<PluginPlatform>>,
    pub command: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PluginManifestAction {
    pub id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub contexts: Vec<PluginActionContext>,
    /// Which file extensions this action handles. Empty means every file; see
    /// [`PluginActionInfo::file_extensions`].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub file_extensions: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub platforms: Option<Vec<PluginPlatform>>,
    pub command: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PluginManifestEventHook {
    pub on: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub platforms: Option<Vec<PluginPlatform>>,
    pub command: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PluginManifestPane {
    pub id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub platforms: Option<Vec<PluginPlatform>>,
    #[serde(default)]
    pub placement: PluginPanePlacement,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<PopupSize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<PopupSize>,
    pub command: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PluginManifestLinkHandler {
    pub id: String,
    pub title: String,
    pub pattern: String,
    pub action: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub platforms: Option<Vec<PluginPlatform>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema, Default)]
pub struct PluginActionListParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema, Default)]
pub struct PluginLogListParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PluginActionInvokeParams {
    pub action_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<PluginInvocationContext>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PluginCommandLogInfo {
    pub log_id: String,
    pub plugin_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event: Option<String>,
    pub command: Vec<String>,
    pub status: PluginCommandStatus,
    pub started_unix_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finished_unix_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stdout: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stderr: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PluginCommandStatus {
    Running,
    Succeeded,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PluginPlatform {
    Linux,
    Macos,
    Windows,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PluginActionContext {
    Global,
    Workspace,
    Tab,
    Pane,
    Selection,
    File,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PluginInvocationContext {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_cwd: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worktree: Option<WorkspaceWorktreeInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tab_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tab_label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focused_pane_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focused_pane_cwd: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focused_pane_agent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focused_pane_status: Option<AgentStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invocation_source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clicked_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link_handler_id: Option<String>,
    /// Exact file paths prepared by a file-oriented client surface. Empty for
    /// non-file invocations. Paths that cannot be represented losslessly in
    /// JSON must not be placed here.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub file_paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PluginActionInfo {
    pub plugin_id: String,
    pub action_id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub contexts: Vec<PluginActionContext>,
    /// Which file extensions this action handles, normalized and lowercase.
    ///
    /// Empty means every file. Every manifest written before this field
    /// existed omits it, so reading empty as "nothing" would silently disable
    /// every installed plugin.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub file_extensions: Vec<String>,
    pub command: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub platforms: Option<Vec<PluginPlatform>>,
}

impl PluginActionInfo {
    pub fn qualified_id(&self) -> String {
        format!("{}.{}", self.plugin_id, self.action_id)
    }

    /// Whether this action handles every path in the selection.
    ///
    /// Every path, not any: offering an action on a partly-matching selection
    /// runs the wrong program on the files that did not match, which is the
    /// defect this field exists to prevent. An empty selection matches
    /// nothing — "all paths match" is vacuously true for an empty list, and
    /// there is no file to act on.
    pub fn matches_paths(&self, paths: &[std::path::PathBuf]) -> bool {
        if paths.is_empty() {
            return false;
        }
        if self.file_extensions.is_empty() {
            return true;
        }
        paths.iter().all(|path| {
            path.extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| {
                    let extension = extension.to_ascii_lowercase();
                    self.file_extensions
                        .iter()
                        .any(|handled| handled == &extension)
                })
        })
    }
}

/// Reduce a manifest-written extension to the one form matching compares.
///
/// Authors reasonably write `xlsx`, `.xlsx` and `XLSX`; accepting all three
/// and storing one keeps the match a single rule rather than three. An entry
/// that normalizes to nothing is dropped, so a stray empty string cannot
/// become an extension that matches nothing and quietly hides the action.
pub fn normalize_plugin_file_extension(extension: &str) -> Option<String> {
    let extension = extension.trim().trim_start_matches('.').trim();
    (!extension.is_empty()).then(|| extension.to_ascii_lowercase())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PluginPaneOpenParams {
    pub plugin_id: String,
    pub entrypoint: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placement: Option<PluginPanePlacement>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<PopupSize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<PopupSize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_pane_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub direction: Option<SplitDirection>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(default)]
    pub focus: bool,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub env: HashMap<String, String>,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema, Default,
)]
#[serde(rename_all = "snake_case")]
pub enum PluginPanePlacement {
    #[default]
    Overlay,
    Popup,
    Split,
    Tab,
    Zoomed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PluginPaneFocusParams {
    pub pane_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PluginPaneCloseParams {
    pub pane_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PluginPaneInfo {
    pub plugin_id: String,
    pub entrypoint: String,
    pub pane: PaneInfo,
}
