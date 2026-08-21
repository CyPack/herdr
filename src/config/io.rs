use std::path::{Path, PathBuf};

use tracing::warn;

use super::{model::LoadedConfig, Config, CONFIG_PATH_ENV_VAR};

/// Every top-level key `Config` accepts.
///
/// This list only feeds diagnostics — deserialization happens first and is
/// unaffected — but a section missing here is reported to the reader as
/// "ignoring section", which is a lie that costs them the setting they just
/// wrote. It drifted before: `preview` and `tailscale` shipped without being
/// added, so both told that lie until `shell` arrived and the same trap caught
/// a third one. `scripts/config_reference_check.py` now fails when this list
/// and the config model disagree.
const KNOWN_TOP_LEVEL_CONFIG_KEYS: &[&str] = &[
    "advanced",
    "experimental",
    "keys",
    "onboarding",
    "preview",
    "projects",
    "remote",
    "session",
    "shell",
    "spaces",
    "tailscale",
    "terminal",
    "theme",
    "ui",
    "update",
    "worktrees",
];

pub fn app_dir_name() -> &'static str {
    if cfg!(debug_assertions) {
        "herdr-dev"
    } else {
        "herdr"
    }
}

pub fn config_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("XDG_CONFIG_HOME") {
        return PathBuf::from(dir).join(app_dir_name());
    }
    platform_config_dir()
}

pub fn state_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("XDG_STATE_HOME") {
        return PathBuf::from(dir).join(app_dir_name());
    }
    platform_state_dir()
}

#[cfg(windows)]
fn platform_config_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("APPDATA") {
        return PathBuf::from(dir).join(app_dir_name());
    }
    if let Ok(profile) = std::env::var("USERPROFILE") {
        return PathBuf::from(profile)
            .join("AppData")
            .join("Roaming")
            .join(app_dir_name());
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(format!(".config/{}", app_dir_name()));
    }
    std::env::temp_dir().join(app_dir_name())
}

#[cfg(not(windows))]
fn platform_config_dir() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(format!(".config/{}", app_dir_name()))
    } else {
        std::env::temp_dir().join(app_dir_name())
    }
}

#[cfg(windows)]
fn platform_state_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("LOCALAPPDATA") {
        return PathBuf::from(dir).join(app_dir_name());
    }
    if let Ok(profile) = std::env::var("USERPROFILE") {
        return PathBuf::from(profile)
            .join("AppData")
            .join("Local")
            .join(app_dir_name());
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(format!(".local/state/{}", app_dir_name()));
    }
    std::env::temp_dir().join(format!("{}-state", app_dir_name()))
}

#[cfg(not(windows))]
fn platform_state_dir() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(format!(".local/state/{}", app_dir_name()))
    } else {
        std::env::temp_dir().join(format!("{}-state", app_dir_name()))
    }
}

fn read_optional_config(path: &Path) -> std::io::Result<Option<String>> {
    match std::fs::read_to_string(path) {
        Ok(content) => Ok(Some(content)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err),
    }
}

/// The overlay file `herdr space promote` owns. Loaded after the user's own
/// config so hand-written rules win first-match (TP-RANK-05).
pub fn managed_spaces_path() -> PathBuf {
    config_dir().join("spaces.managed.toml")
}

/// Merge a `spaces.managed.toml` document into an already-loaded config.
/// Managed entries append after user-authored ones; a broken overlay is
/// reported and skipped, never fatal (TP-RANK-05).
///
/// Every collection the overlay can carry is appended here, and
/// `scripts/managed_overlay_check.py` fails the build when one is missed. A
/// forgotten collection is invisible without that gate: the file parses, the
/// value is valid, `herdr config check` answers "ok", and the entry is
/// dropped when this function returns. That is how `[[spaces.node]]` — every
/// module created from the sidebar or by `space move --new-group` — lived on
/// disk and nowhere else (TP-MOVL-01).
pub(crate) fn merge_managed_spaces_str(config: &mut Config, content: &str) -> Vec<String> {
    #[derive(Debug, Default, serde::Deserialize)]
    #[serde(default)]
    struct ManagedSpacesFile {
        spaces: super::model::SpacesConfig,
    }
    match toml::from_str::<ManagedSpacesFile>(content) {
        Ok(managed) => {
            config.spaces.split.extend(managed.spaces.split);
            config.spaces.project.extend(managed.spaces.project);
            config.spaces.node.extend(managed.spaces.node);
            // TP-MOD-34: appended after the user's own, which is what makes
            // `display_name_for`'s last-wins rule mean "the most recent
            // decision" rather than "whichever file was read second".
            config.spaces.display.extend(managed.spaces.display);
            Vec::new()
        }
        Err(err) => vec![format!(
            "spaces.managed.toml parse error: {err}; ignoring the managed overlay"
        )],
    }
}

/// The overlay file the bar config panel owns. Loaded after the user's own
/// config: a field the panel wrote wins over the hand-written value for that
/// edge, and a field it left unwritten keeps following the user's file
/// (TP-CHROME-149). The asymmetry with `spaces.managed.toml` is deliberate —
/// spaces entries are additive rules, so appending after the user keeps the
/// user first; bar fields are scalar overrides, and an override that lost to
/// the file it exists to change would make the panel a silent no-op.
pub fn managed_bars_path() -> PathBuf {
    config_dir().join("bars.managed.toml")
}

/// One edge's overrides as the panel wrote them.
///
/// Only the fields the panel manages are carried; `gradient`, `sections`,
/// `max_sections` and `hide_when_focused` stay hand-written territory on
/// purpose — the panel has no surface for lists or budgets, and a field the
/// overlay cannot carry is a field it can never silently pin. `border` is a
/// word rather than a bool because the panel has three states to persist —
/// auto / on / off — and TOML cannot write "explicitly none".
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Deserialize)]
#[serde(default)]
pub(crate) struct ManagedBarOverride {
    pub(crate) enabled: Option<bool>,
    pub(crate) size: Option<u16>,
    pub(crate) style: Option<String>,
    pub(crate) border: Option<String>,
    pub(crate) color: Option<String>,
    pub(crate) background: Option<String>,
}

impl ManagedBarOverride {
    /// True when the override carries nothing — the empty diff Apply skips.
    pub(crate) fn is_empty(&self) -> bool {
        let Self {
            enabled,
            size,
            style,
            border,
            color,
            background,
        } = self;
        enabled.is_none()
            && size.is_none()
            && style.is_none()
            && border.is_none()
            && color.is_none()
            && background.is_none()
    }
}

/// Merge a `bars.managed.toml` document into an already-loaded config.
/// Field-level last-wins per edge; a broken overlay is reported and skipped,
/// never fatal (TP-CHROME-149).
pub(crate) fn merge_managed_bars_str(config: &mut Config, content: &str) -> Vec<String> {
    #[derive(Debug, Default, serde::Deserialize)]
    #[serde(default)]
    struct ManagedBarsFile {
        shell: ManagedShell,
    }
    #[derive(Debug, Default, serde::Deserialize)]
    #[serde(default)]
    struct ManagedShell {
        bars: ManagedBars,
    }
    #[derive(Debug, Default, serde::Deserialize)]
    #[serde(default)]
    struct ManagedBars {
        top: Option<ManagedBarOverride>,
        bottom: Option<ManagedBarOverride>,
        left: Option<ManagedBarOverride>,
        right: Option<ManagedBarOverride>,
    }
    match toml::from_str::<ManagedBarsFile>(content) {
        Ok(managed) => {
            let mut diagnostics = Vec::new();
            let bars = managed.shell.bars;
            let pairs = [
                (bars.top, &mut config.shell.bars.top, "top"),
                (bars.bottom, &mut config.shell.bars.bottom, "bottom"),
                (bars.left, &mut config.shell.bars.left, "left"),
                (bars.right, &mut config.shell.bars.right, "right"),
            ];
            for (over, target, edge) in pairs {
                if let Some(over) = over {
                    apply_managed_bar_override(target, over, edge, &mut diagnostics);
                }
            }
            diagnostics
        }
        Err(err) => vec![format!(
            "bars.managed.toml parse error: {err}; ignoring the managed overlay"
        )],
    }
}

/// Lay one edge's overrides over the loaded bar.
///
/// The override is destructured without `..` on purpose: a field added to
/// `ManagedBarOverride` must be routed here or this stops compiling — the
/// structural gate `managed_overlay_check.py` provides for the spaces merge,
/// the compiler provides for this one.
fn apply_managed_bar_override(
    target: &mut super::model::ShellBarConfig,
    over: ManagedBarOverride,
    edge: &str,
    diagnostics: &mut Vec<String>,
) {
    let ManagedBarOverride {
        enabled,
        size,
        style,
        border,
        color,
        background,
    } = over;
    if let Some(value) = enabled {
        target.enabled = value;
    }
    if let Some(value) = size {
        // Refused rather than clamped — the same doctrine `max_sections`
        // follows — so a hand-edited overlay cannot smuggle past the 1-32
        // range the spec promises.
        if (1..=32).contains(&value) {
            target.size = value;
        } else {
            diagnostics.push(format!(
                "bars.managed.toml: {edge}.size {value} is outside 1-32; ignoring that key"
            ));
        }
    }
    if let Some(value) = style {
        target.style = value;
    }
    match border.as_deref() {
        None => {}
        Some("auto") => target.border = None,
        Some("on") => target.border = Some(true),
        Some("off") => target.border = Some(false),
        Some(other) => diagnostics.push(format!(
            "bars.managed.toml: {edge}.border {other:?} is not auto/on/off; ignoring that key"
        )),
    }
    if let Some(value) = color {
        target.color = value;
    }
    if let Some(value) = background {
        target.background = value;
    }
}

/// The banner every written `bars.managed.toml` carries — the file explains
/// itself to the person who finds it.
const MANAGED_BARS_HEADER: &str = "\
# Written by herdr's bar config panel. Fields here override the same fields\n\
# in config.toml, edge by edge; delete a line to hand that field back to\n\
# your own config.\n";

/// Produce the next `bars.managed.toml` document: the existing content with
/// `overrides` laid over its `[shell.bars.<edge>]` tables, field by field.
/// Read-modify-write at the document level, so one Apply never erases the
/// entry an earlier Apply wrote for another edge; an unreadable existing
/// document is refused rather than silently emptied (TP-CHROME-151).
pub(crate) fn upsert_managed_bars_doc(
    existing: &str,
    overrides: &[(&str, ManagedBarOverride)],
) -> Result<String, String> {
    let mut root: toml::Value = if existing.trim().is_empty() {
        toml::Value::Table(toml::map::Map::new())
    } else {
        toml::from_str(existing).map_err(|err| format!("bars.managed.toml parse error: {err}"))?
    };
    let table = root
        .as_table_mut()
        .ok_or_else(|| "bars.managed.toml root is not a table".to_string())?;
    let shell = table
        .entry("shell")
        .or_insert_with(|| toml::Value::Table(toml::map::Map::new()))
        .as_table_mut()
        .ok_or_else(|| "bars.managed.toml [shell] is not a table".to_string())?;
    let bars = shell
        .entry("bars")
        .or_insert_with(|| toml::Value::Table(toml::map::Map::new()))
        .as_table_mut()
        .ok_or_else(|| "bars.managed.toml [shell.bars] is not a table".to_string())?;
    for (edge, over) in overrides {
        let entry = bars
            .entry((*edge).to_string())
            .or_insert_with(|| toml::Value::Table(toml::map::Map::new()))
            .as_table_mut()
            .ok_or_else(|| format!("bars.managed.toml [shell.bars.{edge}] is not a table"))?;
        let ManagedBarOverride {
            enabled,
            size,
            style,
            border,
            color,
            background,
        } = over;
        if let Some(value) = enabled {
            entry.insert("enabled".into(), toml::Value::Boolean(*value));
        }
        if let Some(value) = size {
            entry.insert("size".into(), toml::Value::Integer(i64::from(*value)));
        }
        if let Some(value) = style {
            entry.insert("style".into(), toml::Value::String(value.clone()));
        }
        if let Some(value) = border {
            entry.insert("border".into(), toml::Value::String(value.clone()));
        }
        if let Some(value) = color {
            entry.insert("color".into(), toml::Value::String(value.clone()));
        }
        if let Some(value) = background {
            entry.insert("background".into(), toml::Value::String(value.clone()));
        }
    }
    let body = toml::to_string_pretty(&root)
        .map_err(|err| format!("bars.managed.toml serialize error: {err}"))?;
    Ok(format!("{MANAGED_BARS_HEADER}\n{body}"))
}

/// Write `overrides` into the managed bars file on disk — read, upsert,
/// replace atomically (write-then-rename), so a crash mid-write leaves the
/// old document rather than half of a new one.
pub(crate) fn persist_managed_bar_overrides(
    overrides: &[(&str, ManagedBarOverride)],
) -> Result<(), String> {
    let path = managed_bars_path();
    let existing = match read_optional_config(&path) {
        Ok(Some(content)) => content,
        Ok(None) => String::new(),
        Err(err) => return Err(format!("bars.managed.toml read error: {err}")),
    };
    let next = upsert_managed_bars_doc(&existing, overrides)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("bars.managed.toml mkdir error: {err}"))?;
    }
    let tmp = path.with_extension("toml.tmp");
    std::fs::write(&tmp, next).map_err(|err| format!("bars.managed.toml write error: {err}"))?;
    std::fs::rename(&tmp, &path).map_err(|err| format!("bars.managed.toml rename error: {err}"))
}

/// Apply the managed overlay, if one exists, to a loaded config.
fn finish_with_managed_overlay(mut loaded: LoadedConfig) -> LoadedConfig {
    match read_optional_config(&managed_spaces_path()) {
        Ok(Some(content)) => {
            let diagnostics = merge_managed_spaces_str(&mut loaded.config, &content);
            loaded.diagnostics.extend(diagnostics);
        }
        Ok(None) => {}
        Err(err) => loaded
            .diagnostics
            .push(format!("spaces.managed.toml read error: {err}; ignoring")),
    }
    match read_optional_config(&managed_bars_path()) {
        Ok(Some(content)) => {
            let diagnostics = merge_managed_bars_str(&mut loaded.config, &content);
            loaded.diagnostics.extend(diagnostics);
        }
        Ok(None) => {}
        Err(err) => loaded
            .diagnostics
            .push(format!("bars.managed.toml read error: {err}; ignoring")),
    }
    loaded
}

impl Config {
    pub fn load() -> LoadedConfig {
        finish_with_managed_overlay(Self::load_inner())
    }

    fn load_inner() -> LoadedConfig {
        let path = config_path();
        let content = match read_optional_config(&path) {
            Ok(Some(content)) => content,
            Ok(None) => {
                return LoadedConfig {
                    config: Self::default(),
                    diagnostics: Vec::new(),
                    invalid_sections: Vec::new(),
                };
            }
            Err(err) => {
                warn!(err = %err, "config read error, using defaults");
                return LoadedConfig {
                    config: Self::default(),
                    diagnostics: vec![format!("config read error: {err}; using defaults")],
                    invalid_sections: Vec::new(),
                };
            }
        };

        match deserialize_with_ignored::<Config, _>(toml::Deserializer::new(&content)) {
            Ok((config, ignored_keys)) => {
                let (unknown_sections, mut diagnostics) =
                    unknown_top_level_sections_from_str(&content);
                diagnostics.extend(unknown_config_key_diagnostics(
                    ignored_keys
                        .into_iter()
                        .filter(|path| {
                            !matches!(path.as_slice(), [ConfigKeyPathSegment::Key(key)] if unknown_sections.contains(key))
                        })
                        .collect(),
                    None,
                ));
                diagnostics.extend(config.collect_diagnostics());
                LoadedConfig {
                    config,
                    diagnostics,
                    invalid_sections: Vec::new(),
                }
            }
            Err(err) => {
                warn!(err = %err, "config parse error, using defaults");
                LoadedConfig {
                    config: Self::default(),
                    diagnostics: vec![format!("config parse error: {err}; using defaults")],
                    invalid_sections: Vec::new(),
                }
            }
        }
    }
}

pub(super) fn resolve_config_relative_path(path: &Path) -> PathBuf {
    if path.is_absolute() {
        return path.to_path_buf();
    }

    config_path()
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(path)
}

pub fn config_path() -> PathBuf {
    if let Ok(path) = std::env::var(CONFIG_PATH_ENV_VAR) {
        return PathBuf::from(path);
    }
    config_dir().join("config.toml")
}

pub fn config_diagnostic_summary(diagnostics: &[String]) -> Option<String> {
    if diagnostics.is_empty() {
        return None;
    }

    let target = config_path()
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("config.toml")
        .to_string();
    let read_error = diagnostics
        .iter()
        .any(|diagnostic| diagnostic.starts_with("config read error:"));
    let impact = if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.contains("using defaults"))
    {
        if read_error {
            " unreadable; using defaults"
        } else {
            " invalid; using defaults"
        }
    } else if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.contains("keeping current config"))
    {
        if read_error {
            " unreadable; keeping current config"
        } else {
            " invalid; keeping current config"
        }
    } else if diagnostics
        .iter()
        .all(|diagnostic| diagnostic.starts_with("unknown config key "))
    {
        " has unknown keys"
    } else {
        ""
    };

    Some(format!("{target}{impact}; herdr config check"))
}

pub fn load_live_config() -> Result<LoadedConfig, Vec<String>> {
    load_live_config_inner().map(finish_with_managed_overlay)
}

fn load_live_config_inner() -> Result<LoadedConfig, Vec<String>> {
    let path = config_path();
    let content = match read_optional_config(&path) {
        Ok(Some(content)) => content,
        Ok(None) => {
            return Ok(LoadedConfig {
                config: Config::default(),
                diagnostics: Vec::new(),
                invalid_sections: Vec::new(),
            });
        }
        Err(err) => {
            return Err(vec![format!(
                "config read error: {err}; keeping current config"
            )]);
        }
    };
    load_live_config_from_str(&content)
}

fn load_live_config_from_str(content: &str) -> Result<LoadedConfig, Vec<String>> {
    let value = content
        .parse::<toml::Value>()
        .map_err(|err| vec![format!("config parse error: {err}; keeping current config")])?;
    let table = value.as_table().ok_or_else(|| {
        vec![
            "config parse error: top-level config must be a table; keeping current config"
                .to_string(),
        ]
    })?;

    let mut config = Config::default();
    let mut diagnostics = unknown_top_level_section_diagnostics(table);
    diagnostics.extend(unknown_top_level_config_key_diagnostics(table));
    let mut invalid_sections = Vec::new();

    if let Some(value) = table.get("onboarding") {
        match value.clone().try_into::<Option<bool>>() {
            Ok(onboarding) => config.onboarding = onboarding,
            Err(err) => diagnostics.push(format!(
                "invalid onboarding setting: {err}; keeping current onboarding state"
            )),
        }
    }

    load_live_section(
        table,
        "theme",
        "theme config",
        &mut diagnostics,
        &mut invalid_sections,
        |section| config.theme = section,
    );
    load_live_section(
        table,
        "keys",
        "keybinding config",
        &mut diagnostics,
        &mut invalid_sections,
        |section| config.keys = section,
    );
    load_live_section(
        table,
        "terminal",
        "terminal config",
        &mut diagnostics,
        &mut invalid_sections,
        |section| config.terminal = section,
    );
    load_live_section(
        table,
        "session",
        "session config",
        &mut diagnostics,
        &mut invalid_sections,
        |section| config.session = section,
    );
    load_live_section(
        table,
        "update",
        "update config",
        &mut diagnostics,
        &mut invalid_sections,
        |section| config.update = section,
    );
    load_live_section(
        table,
        "ui",
        "ui config",
        &mut diagnostics,
        &mut invalid_sections,
        |section| config.ui = section,
    );
    load_live_section(
        table,
        "advanced",
        "advanced config",
        &mut diagnostics,
        &mut invalid_sections,
        |section| config.advanced = section,
    );
    load_live_section(
        table,
        "worktrees",
        "worktree config",
        &mut diagnostics,
        &mut invalid_sections,
        |section| config.worktrees = section,
    );
    load_live_section(
        table,
        "experimental",
        "experimental config",
        &mut diagnostics,
        &mut invalid_sections,
        |section| config.experimental = section,
    );
    load_live_section(
        table,
        "remote",
        "remote config",
        &mut diagnostics,
        &mut invalid_sections,
        |section| config.remote = section,
    );
    load_live_section(
        table,
        "projects",
        "projects config",
        &mut diagnostics,
        &mut invalid_sections,
        |section| config.projects = section,
    );
    // TP-SPLIT-CONF-03: a known section, isolated when malformed.
    load_live_section(
        table,
        "spaces",
        "spaces config",
        &mut diagnostics,
        &mut invalid_sections,
        |section| config.spaces = section,
    );
    // Startup reads the whole struct through serde; this path is hand written
    // per section, so a section added to `Config` alone loads at startup and
    // silently vanishes on reload. `[shell]` was in exactly that state, and
    // `[preview]` and `[tailscale]` still are — recorded rather than fixed
    // here, because `tailscale.pinned_devices` is written by the app and
    // changing when it is re-read is a separate measurement.
    // TP-CHROME-32: this one section carries the whole shell subtree, arrays of
    // section tables included, so a bar's division survives a reload intact.
    load_live_section(
        table,
        "shell",
        "shell config",
        &mut diagnostics,
        &mut invalid_sections,
        |section| config.shell = section,
    );

    Ok(LoadedConfig {
        config,
        diagnostics,
        invalid_sections,
    })
}

fn unknown_top_level_sections_from_str(content: &str) -> (Vec<String>, Vec<String>) {
    let Ok(value) = content.parse::<toml::Value>() else {
        return (Vec::new(), Vec::new());
    };
    let Some(table) = value.as_table() else {
        return (Vec::new(), Vec::new());
    };

    let mut keys = Vec::new();
    let mut diagnostics = Vec::new();
    for (key, value) in table {
        if let Some(diagnostic) = unknown_top_level_section_diagnostic(key, value) {
            keys.push(key.clone());
            diagnostics.push(diagnostic);
        }
    }
    (keys, diagnostics)
}

fn unknown_top_level_section_diagnostics(
    table: &toml::map::Map<String, toml::Value>,
) -> Vec<String> {
    table
        .iter()
        .filter_map(|(key, value)| unknown_top_level_section_diagnostic(key, value))
        .collect()
}

fn unknown_top_level_section_diagnostic(key: &str, value: &toml::Value) -> Option<String> {
    if KNOWN_TOP_LEVEL_CONFIG_KEYS.contains(&key) {
        return None;
    }

    let header = if value.is_table() {
        format!("[{key}]")
    } else if value
        .as_array()
        .is_some_and(|items| !items.is_empty() && items.iter().all(toml::Value::is_table))
    {
        format!("[[{key}]]")
    } else {
        return None;
    };

    if key == "toast" {
        Some(format!(
            "unknown config section {header}; did you mean [ui.toast]? ignoring section"
        ))
    } else {
        Some(format!("unknown config section {header}; ignoring section"))
    }
}

fn unknown_top_level_config_key_diagnostics(
    table: &toml::map::Map<String, toml::Value>,
) -> Vec<String> {
    let paths = table
        .iter()
        .filter(|(key, value)| {
            !KNOWN_TOP_LEVEL_CONFIG_KEYS.contains(&key.as_str())
                && unknown_top_level_section_diagnostic(key, value).is_none()
        })
        .map(|(key, _)| vec![ConfigKeyPathSegment::Key(key.clone())])
        .collect();
    unknown_config_key_diagnostics(paths, None)
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum ConfigKeyPathSegment {
    Key(String),
    Index(usize),
}

fn config_key_path(path: &serde_ignored::Path<'_>) -> Vec<ConfigKeyPathSegment> {
    fn visit(path: &serde_ignored::Path<'_>, segments: &mut Vec<ConfigKeyPathSegment>) {
        match path {
            serde_ignored::Path::Root => {}
            serde_ignored::Path::Seq { parent, index } => {
                visit(parent, segments);
                segments.push(ConfigKeyPathSegment::Index(*index));
            }
            serde_ignored::Path::Map { parent, key } => {
                visit(parent, segments);
                segments.push(ConfigKeyPathSegment::Key(key.clone()));
            }
            serde_ignored::Path::Some { parent }
            | serde_ignored::Path::NewtypeStruct { parent }
            | serde_ignored::Path::NewtypeVariant { parent } => visit(parent, segments),
        }
    }

    let mut segments = Vec::new();
    visit(path, &mut segments);
    segments
}

fn format_config_key_path(path: &[ConfigKeyPathSegment]) -> String {
    path.iter()
        .map(|segment| match segment {
            ConfigKeyPathSegment::Key(key)
                if !key.is_empty()
                    && key.bytes().all(|byte| {
                        byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-')
                    }) =>
            {
                key.clone()
            }
            ConfigKeyPathSegment::Key(key) => toml::Value::String(key.clone()).to_string(),
            ConfigKeyPathSegment::Index(index) => index.to_string(),
        })
        .collect::<Vec<_>>()
        .join(".")
}

fn unknown_config_key_diagnostics(
    paths: Vec<Vec<ConfigKeyPathSegment>>,
    section: Option<&str>,
) -> Vec<String> {
    let mut paths: Vec<Vec<ConfigKeyPathSegment>> = paths
        .into_iter()
        .map(|mut path| {
            if let Some(section) = section {
                path.insert(0, ConfigKeyPathSegment::Key(section.to_string()));
            }
            path
        })
        .collect();
    paths.sort();
    paths.dedup();
    paths
        .into_iter()
        .map(|path| {
            format!(
                "unknown config key {}; ignoring key",
                format_config_key_path(&path)
            )
        })
        .collect()
}

fn deserialize_with_ignored<'de, T, D>(
    deserializer: D,
) -> Result<(T, Vec<Vec<ConfigKeyPathSegment>>), D::Error>
where
    T: serde::Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    let mut ignored = Vec::new();
    let value = serde_ignored::deserialize(deserializer, |path| {
        ignored.push(config_key_path(&path));
    })?;
    Ok((value, ignored))
}

fn load_live_section<T>(
    table: &toml::map::Map<String, toml::Value>,
    section: &'static str,
    label: &str,
    diagnostics: &mut Vec<String>,
    invalid_sections: &mut Vec<String>,
    apply: impl FnOnce(T),
) where
    T: serde::de::DeserializeOwned,
{
    let Some(value) = table.get(section) else {
        return;
    };

    match deserialize_with_ignored(value.clone()) {
        Ok((section_config, ignored_keys)) => {
            diagnostics.extend(unknown_config_key_diagnostics(ignored_keys, Some(section)));
            apply(section_config);
        }
        Err(err) => {
            diagnostics.push(format!(
                "invalid {label}: {err}; keeping current {section} settings"
            ));
            invalid_sections.push(section.to_string());
        }
    }
}

pub(crate) fn upsert_top_level_bool(content: &str, key: &str, value: bool) -> String {
    let replacement = format!("{key} = {value}");
    let mut lines: Vec<String> = content.lines().map(|line| line.to_string()).collect();
    let mut in_section = false;

    for line in &mut lines {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_section = true;
            continue;
        }
        if in_section {
            continue;
        }
        if trimmed.starts_with(&format!("{key} ")) || trimmed.starts_with(&format!("{key}=")) {
            *line = replacement.clone();
            return lines.join("\n") + "\n";
        }
    }

    if lines.is_empty() {
        format!("{replacement}\n")
    } else {
        format!("{replacement}\n{}\n", lines.join("\n").trim_end())
    }
}

/// Write a key = value pair in a TOML section (creates section if missing).
pub fn upsert_section_value(content: &str, section: &str, key: &str, value: &str) -> String {
    upsert_section_raw(content, section, key, value)
}

pub fn upsert_section_bool(content: &str, section: &str, key: &str, value: bool) -> String {
    upsert_section_raw(content, section, key, &value.to_string())
}

pub fn remove_section_key(content: &str, section: &str, key: &str) -> String {
    let header = format!("[{section}]");
    let lines: Vec<&str> = content.lines().collect();
    let mut result = Vec::new();
    let mut i = 0;
    let mut in_section = false;

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();

        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_section = trimmed == header;
            result.push(line.to_string());
            i += 1;
            continue;
        }

        if in_section
            && (trimmed.starts_with(&format!("{key} ")) || trimmed.starts_with(&format!("{key}=")))
        {
            i += 1;
            continue;
        }

        result.push(line.to_string());
        i += 1;
    }

    result.join("\n") + "\n"
}

pub fn remove_keybinding_config_sections(content: &str) -> (String, bool) {
    let mut result = Vec::new();
    let mut removed = false;
    let mut skipping_key_section = false;
    let mut in_table = false;

    for line in content.lines() {
        let trimmed = line.trim();

        if let Some(table_name) = toml_table_header_name(trimmed) {
            in_table = true;
            skipping_key_section = is_keys_table_name(table_name);
            if skipping_key_section {
                removed = true;
                continue;
            }
        } else if skipping_key_section || (!in_table && is_top_level_keys_assignment(trimmed)) {
            removed = true;
            continue;
        }

        result.push(line.to_string());
    }

    let mut updated = result.join("\n");
    if content.ends_with('\n') || !updated.is_empty() {
        updated.push('\n');
    }
    (updated, removed)
}

fn toml_table_header_name(trimmed: &str) -> Option<&str> {
    if let Some(name) = trimmed
        .strip_prefix("[[")
        .and_then(|value| value.strip_suffix("]]"))
    {
        return Some(name.trim());
    }
    trimmed
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .map(str::trim)
}

fn is_keys_table_name(name: &str) -> bool {
    name == "keys" || name.starts_with("keys.")
}

fn is_top_level_keys_assignment(trimmed: &str) -> bool {
    trimmed.starts_with("keys ") || trimmed.starts_with("keys=") || trimmed.starts_with("keys.")
}

fn upsert_section_raw(content: &str, section: &str, key: &str, value: &str) -> String {
    let header = format!("[{section}]");
    let assignment = format!("{key} = {value}");
    let lines: Vec<&str> = content.lines().collect();
    let mut result = Vec::new();
    let mut i = 0;
    let mut found_section = false;
    let mut inserted = false;

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();

        if trimmed == header {
            found_section = true;
            result.push(line.to_string());
            i += 1;

            while i < lines.len() {
                let current = lines[i];
                let current_trimmed = current.trim();
                if current_trimmed.starts_with('[') && current_trimmed.ends_with(']') {
                    if !inserted {
                        result.push(assignment.clone());
                        inserted = true;
                    }
                    break;
                }

                if current_trimmed.starts_with(&format!("{key} "))
                    || current_trimmed.starts_with(&format!("{key}="))
                {
                    result.push(assignment.clone());
                    inserted = true;
                } else {
                    result.push(current.to_string());
                }
                i += 1;
            }

            continue;
        }

        result.push(line.to_string());
        i += 1;
    }

    if !found_section {
        if !result.is_empty() && !result.last().is_some_and(|line| line.trim().is_empty()) {
            result.push(String::new());
        }
        result.push(header);
        result.push(assignment);
    } else if !inserted {
        result.push(assignment);
    }

    result.join("\n") + "\n"
}

#[cfg(test)]
mod tests {
    use super::*;

    // The layer that added `[shell.bars]` tested the config MODEL and never a
    // config FILE, so nobody noticed the reader would be told the section was
    // being ignored. A person who writes a setting and is told it was ignored
    // stops there — the lie is more expensive than a missing feature.
    #[test]
    fn a_shell_bars_table_is_read_and_not_reported_as_unknown() {
        let content = "\
[shell.bars.top]
enabled = true
size = 3

[shell.bars.right]
enabled = true
size = 14
";
        let loaded = load_live_config_from_str(content).expect("a bars table parses");

        assert!(loaded.config.shell.bars.top.enabled);
        assert_eq!(loaded.config.shell.bars.top.size, 3);
        assert!(loaded.config.shell.bars.right.enabled);
        assert_eq!(loaded.config.shell.bars.right.size, 14);
        assert!(
            !loaded.config.shell.bars.bottom.enabled,
            "an edge nobody named stays off"
        );
        assert!(
            loaded.diagnostics.is_empty(),
            "a valid bars table must not produce diagnostics: {:?}",
            loaded.diagnostics
        );
    }

    // The same trap, one level deeper: sections arrive as an array of tables
    // under an edge, and an array that the live reader silently dropped would
    // leave the bar undivided on reload while the file plainly says otherwise.
    #[test]
    fn a_bars_sections_array_survives_the_live_reader() {
        let content = "\
[shell.bars.top]
enabled = true
size = 3

[[shell.bars.top.sections]]
kind = \"fixed\"
cells = 12

[[shell.bars.top.sections]]
kind = \"fill\"
weight = 3
";
        let loaded = load_live_config_from_str(content).expect("a sections array parses");

        let sections = &loaded.config.shell.bars.top.sections;
        assert_eq!(sections.len(), 2, "both sections must survive the reader");
        assert_eq!(sections[0].kind, "fixed");
        assert_eq!(sections[0].cells, 12);
        assert_eq!(sections[1].kind, "fill");
        assert_eq!(sections[1].weight, 3);
        assert!(
            loaded.config.shell.bars.bottom.sections.is_empty(),
            "an edge nobody divided stays undivided"
        );
        assert!(
            loaded.diagnostics.is_empty(),
            "a valid sections array must not produce diagnostics: {:?}",
            loaded.diagnostics
        );
    }

    // Every section the model accepts has to be in the diagnostic list, or the
    // reader is told their setting was dropped when it was not.
    #[test]
    fn every_documented_section_is_known_to_the_diagnostics() {
        for section in [
            "preview",
            "shell",
            "tailscale",
            "ui",
            "theme",
            "terminal",
            "spaces",
        ] {
            let content = format!("[{section}]\n");
            let loaded = load_live_config_from_str(&content)
                .unwrap_or_else(|error| panic!("[{section}] must parse: {error:?}"));
            assert!(
                loaded.diagnostics.is_empty(),
                "[{section}] is a real section but was reported: {:?}",
                loaded.diagnostics
            );
        }
    }

    #[test]
    fn upsert_top_level_bool_replaces_existing_value() {
        let content = "onboarding = true\n[keys]\nprefix = \"ctrl+b\"\n";
        let updated = upsert_top_level_bool(content, "onboarding", false);
        assert!(updated.contains("onboarding = false"));
        assert!(!updated.contains("onboarding = true"));
    }

    #[test]
    fn upsert_section_bool_adds_missing_section() {
        let updated = upsert_section_bool("", "ui.toast", "enabled", true);
        assert!(updated.contains("[ui.toast]"));
        assert!(updated.contains("enabled = true"));
    }

    #[test]
    fn remove_section_key_removes_matching_key_from_section() {
        let content =
            "[ui.toast]\nenabled = true\ndelivery = \"herdr\"\n[ui.sound]\nenabled = true\n";
        let updated = remove_section_key(content, "ui.toast", "enabled");
        assert!(!updated.contains("[ui.toast]\nenabled = true"));
        assert!(updated.contains("delivery = \"herdr\""));
        assert!(updated.contains("[ui.sound]\nenabled = true"));
    }

    #[test]
    fn config_diagnostic_summary_uses_compact_actionable_banner() {
        let diagnostics = vec![
            "one".to_string(),
            "two".to_string(),
            "three".to_string(),
            "four".to_string(),
            "five".to_string(),
        ];

        assert_eq!(
            config_diagnostic_summary(&diagnostics).as_deref(),
            Some("config.toml; herdr config check")
        );
    }

    #[test]
    fn config_diagnostic_summary_reports_unknown_keys_compactly() {
        let diagnostics = vec![
            "unknown config key ui.mouse_captur; ignoring key".to_string(),
            "unknown config key keys.new_tabb; ignoring key".to_string(),
        ];

        assert_eq!(
            config_diagnostic_summary(&diagnostics).as_deref(),
            Some("config.toml has unknown keys; herdr config check")
        );
    }

    #[test]
    fn config_diagnostic_summary_keeps_mixed_diagnostics_generic() {
        let diagnostics = vec![
            "invalid ui config: invalid type: string; keeping current ui settings".to_string(),
            "unknown config key keys.new_tabb; ignoring key".to_string(),
        ];

        assert_eq!(
            config_diagnostic_summary(&diagnostics).as_deref(),
            Some("config.toml; herdr config check")
        );
    }

    #[test]
    fn config_diagnostic_summary_reports_default_fallback() {
        let diagnostics = vec![
            "config parse error: TOML parse error at line 33, column 8\n   |\n33 | type = \"popup\"\n   |        ^^^^^^^\nunknown variant `popup`; using defaults"
                .to_string(),
        ];

        assert_eq!(
            config_diagnostic_summary(&diagnostics).as_deref(),
            Some("config.toml invalid; using defaults; herdr config check")
        );
    }

    #[test]
    fn config_diagnostic_summary_reports_unreadable_config_impact() {
        let startup = vec!["config read error: permission denied; using defaults".to_string()];
        assert_eq!(
            config_diagnostic_summary(&startup).as_deref(),
            Some("config.toml unreadable; using defaults; herdr config check")
        );

        let reload =
            vec!["config read error: permission denied; keeping current config".to_string()];
        assert_eq!(
            config_diagnostic_summary(&reload).as_deref(),
            Some("config.toml unreadable; keeping current config; herdr config check")
        );
    }

    #[test]
    fn config_diagnostic_summary_reports_retained_live_config() {
        let diagnostics = vec![
            "config parse error: TOML parse error at line 7, column 4; keeping current config"
                .to_string(),
        ];

        assert_eq!(
            config_diagnostic_summary(&diagnostics).as_deref(),
            Some("config.toml invalid; keeping current config; herdr config check")
        );
    }

    #[test]
    fn config_loaders_report_unreadable_path() {
        let _guard = crate::config::test_config_env_lock().lock().unwrap();
        let path =
            std::env::temp_dir().join(format!("herdr-config-unreadable-{}", std::process::id()));
        std::fs::create_dir_all(&path).unwrap();
        std::env::set_var(CONFIG_PATH_ENV_VAR, &path);

        let startup = Config::load();
        assert!(startup
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("config read error")
                && diagnostic.contains("using defaults")));

        let reload = load_live_config().unwrap_err();
        assert!(reload.iter().any(|diagnostic| {
            diagnostic.contains("config read error")
                && diagnostic.contains("keeping current config")
        }));

        std::env::remove_var(CONFIG_PATH_ENV_VAR);
        let _ = std::fs::remove_dir_all(path);
    }

    #[test]
    fn load_live_config_parses_session_section() {
        let loaded = load_live_config_from_str(
            r#"
[session]
resume_agents_on_restore = true
"#,
        )
        .unwrap();

        assert!(loaded.config.session.resume_agents_on_restore);
        assert!(loaded.diagnostics.is_empty());
        assert!(loaded.invalid_sections.is_empty());
    }

    #[test]
    fn load_live_config_warns_about_unknown_top_level_sections() {
        let loaded = load_live_config_from_str(
            r#"
[toast]
delivery = "system"

[ui.toast]
delivery = "herdr"
"#,
        )
        .unwrap();

        assert_eq!(
            loaded.diagnostics,
            vec!["unknown config section [toast]; did you mean [ui.toast]? ignoring section"]
        );
        assert!(loaded.invalid_sections.is_empty());
        assert_eq!(
            loaded.config.ui.toast.delivery,
            super::super::ToastDelivery::Herdr
        );
    }

    #[test]
    fn load_live_config_warns_about_unknown_keys_and_applies_known_siblings() {
        let loaded = load_live_config_from_str(
            r##"
plugin = []

[theme.custom]
accentt = "#ffffff"

[advanced]
scrollback_lines = 42

[keys]
fullscreen = "prefix+z"
new_tabb = "prefix+t"

[[keys.command]]
key = "prefix+g"
command = "git status"
descrption = "status"

[ui]
mouse_capture = false
mouse_captur = true
"foo.bar" = true
"foo.?.bar" = false

[ui.toast]
enabled = true
delivry = "system"

[ui.sidebar.agents.rows_by_agent]
claude = [["terminal_title"]]
"##,
        )
        .unwrap();

        assert_eq!(
            loaded.diagnostics,
            vec![
                "unknown config key plugin; ignoring key",
                "unknown config key theme.custom.accentt; ignoring key",
                "unknown config key keys.command.0.descrption; ignoring key",
                "unknown config key keys.new_tabb; ignoring key",
                "unknown config key ui.\"foo.?.bar\"; ignoring key",
                "unknown config key ui.\"foo.bar\"; ignoring key",
                "unknown config key ui.mouse_captur; ignoring key",
                "unknown config key ui.toast.delivry; ignoring key",
            ]
        );
        assert!(loaded.invalid_sections.is_empty());
        assert_eq!(loaded.config.advanced.scrollback_limit_bytes, 42);
        assert!(!loaded.config.ui.mouse_capture);
        assert_eq!(
            loaded.config.ui.toast.delivery,
            super::super::ToastDelivery::Herdr
        );
        assert!(loaded
            .config
            .keybinds()
            .zoom
            .bindings
            .iter()
            .any(|binding| binding.label == "prefix+z"));
    }

    #[test]
    fn load_live_config_discards_ignored_keys_from_an_invalid_section() {
        let loaded = load_live_config_from_str(
            r#"
[ui]
mouse_capture = "yes"
mouse_captur = true
"#,
        )
        .unwrap();

        assert_eq!(loaded.diagnostics.len(), 1);
        assert!(loaded.diagnostics[0].contains("invalid ui config"));
        assert!(!loaded.diagnostics[0].starts_with("unknown config key"));
        assert_eq!(loaded.invalid_sections, vec!["ui"]);
    }

    #[test]
    fn startup_config_load_warns_about_unknown_top_level_sections() {
        let _guard = crate::config::test_config_env_lock().lock().unwrap();
        let path = std::env::temp_dir().join(format!(
            "herdr-config-unknown-section-{}.toml",
            std::process::id()
        ));
        std::fs::write(
            &path,
            r#"
[[plugin]]
id = "example"

[ui.toast]
delivery = "system"
"#,
        )
        .unwrap();
        std::env::set_var(CONFIG_PATH_ENV_VAR, &path);

        let loaded = Config::load();

        assert_eq!(
            loaded.diagnostics,
            vec!["unknown config section [[plugin]]; ignoring section"]
        );
        assert_eq!(
            loaded.config.ui.toast.delivery,
            super::super::ToastDelivery::System
        );

        std::env::remove_var(CONFIG_PATH_ENV_VAR);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn remove_keybinding_config_sections_removes_keys_tables_only() {
        let content = r#"onboarding = false

[theme]
name = "catppuccin"

[keys]
prefix = "ctrl+a"
new_tab = "c"

[[keys.command]]
key = "g"
command = "lazygit"

[keys.indexed]
tabs = "ctrl"

[ui]
mouse_capture = false
"#;

        let (updated, removed) = remove_keybinding_config_sections(content);

        assert!(removed);
        assert!(updated.contains("onboarding = false"));
        assert!(updated.contains("[theme]\nname = \"catppuccin\""));
        assert!(updated.contains("[ui]\nmouse_capture = false"));
        assert!(!updated.contains("[keys]"));
        assert!(!updated.contains("[[keys.command]]"));
        assert!(!updated.contains("[keys.indexed]"));
        assert!(toml::from_str::<toml::Value>(&updated).is_ok());
    }

    #[test]
    fn remove_keybinding_config_sections_reports_noop_without_keys() {
        let content = "[ui]\nmouse_capture = true\n";
        let (updated, removed) = remove_keybinding_config_sections(content);
        assert!(!removed);
        assert_eq!(updated, content);
    }

    #[test]
    fn load_live_config_recognizes_projects_section() {
        let loaded = load_live_config_from_str(
            r#"
[projects]
pinned = ["/home/a/x"]
"#,
        )
        .unwrap();

        assert_eq!(loaded.config.projects.pinned, vec!["/home/a/x".to_string()]);
        assert!(
            loaded.diagnostics.is_empty(),
            "known [projects] section must not warn: {:?}",
            loaded.diagnostics
        );
        assert!(loaded.invalid_sections.is_empty());
    }

    #[test]
    fn load_live_config_recognizes_spaces_section() {
        let loaded = load_live_config_from_str(
            r#"
[[spaces.split]]
repo = "/home/a/panel"
match = ["feat/t4f-*"]
key = "panel:t4f"
label = "T4F"
"#,
        )
        .unwrap();

        let rules = loaded.config.spaces.rules();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].key, "panel:t4f");
        assert_eq!(rules[0].label, "T4F");
        assert!(
            loaded.diagnostics.is_empty(),
            "known [spaces] section must not warn: {:?}",
            loaded.diagnostics
        );
    }

    // TP-RANK-05: the managed overlay loads after the user's own config, so a
    // hand-written rule wins first-match against anything promotion wrote.
    #[test]
    fn managed_spaces_overlay_merges_after_user_rules() {
        let mut loaded = load_live_config_from_str(
            r#"
[[spaces.split]]
repo = "/home/a/panel"
match = ["feat/*"]
key = "panel:user"
label = "User"
"#,
        )
        .unwrap();

        let diagnostics = merge_managed_spaces_str(
            &mut loaded.config,
            r#"
[[spaces.split]]
repo = "/home/a/panel"
match = ["feat/x-*"]
key = "panel:managed"
label = "Managed"

[[spaces.project]]
key = "project:x"
spaces = ["panel:managed"]
"#,
        );

        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let rules = loaded.config.spaces.rules();
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].key, "panel:user", "user rules stay first");
        assert_eq!(rules[1].key, "panel:managed");
        assert_eq!(loaded.config.spaces.projects()[0].key, "project:x");
    }

    // TP-MOVL-01/02/03: the overlay carries containers too, and every
    // collection it can hold reaches the live config in one merge.
    //
    // A `[[spaces.node]]` written by `herdr space move --new-group`, by the
    // header's two-click module road, or by hand parsed perfectly well and was
    // then dropped on the floor: the merge copied `split` and `project` and
    // never touched `node`. Nothing reported it, because nothing was wrong —
    // the value was valid and simply unread. The module the user had just
    // created existed on disk and nowhere else.
    #[test]
    fn managed_spaces_overlay_merges_containers_after_user_ones() {
        let mut loaded = load_live_config_from_str(
            r#"
[[spaces.node]]
key = "hand"
name = "Hand written"
"#,
        )
        .unwrap();

        let diagnostics = merge_managed_spaces_str(
            &mut loaded.config,
            r#"
[[spaces.split]]
repo = "/home/a/panel"
match = ["feat/*"]
key = "panel:managed"
label = "Managed"

[[spaces.project]]
key = "project:x"
spaces = ["panel:managed"]

[[spaces.node]]
key = "group:remote-audio"
name = "UZAKTAN SES"
parent = "project:x"
"#,
        );

        assert!(diagnostics.is_empty(), "{diagnostics:?}");

        // TP-MOVL-01: all three collections travel together. A merge that
        // copies some of them is exactly the shape of the bug this test owns,
        // and a single-collection assertion cannot see it.
        assert_eq!(loaded.config.spaces.rules().len(), 1, "split merged");
        assert_eq!(loaded.config.spaces.projects().len(), 1, "project merged");

        let nodes = &loaded.config.spaces.node;
        assert_eq!(nodes.len(), 2, "node merged: {nodes:?}");

        // TP-MOVL-02: hand-written entries keep their place at the front, so
        // first-match still favours what the user wrote by hand (TP-RANK-05).
        assert_eq!(nodes[0].key, "hand", "hand-written node stays first");
        assert_eq!(nodes[1].key, "group:remote-audio");
        assert_eq!(nodes[1].name, "UZAKTAN SES", "the name survives the merge");
        assert_eq!(
            nodes[1].parent, "project:x",
            "the parent survives the merge — without it the module lands at \
             top level instead of under the project the user chose"
        );
    }

    // TP-MOVL-04: an overlay with no `spaces` table at all adds nothing and
    // panics on nothing. The `#[serde(default)]` path regresses silently.
    #[test]
    fn managed_spaces_overlay_accepts_an_empty_document() {
        let mut loaded = load_live_config_from_str("").unwrap();
        let diagnostics = merge_managed_spaces_str(&mut loaded.config, "");
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert!(loaded.config.spaces.rules().is_empty());
        assert!(loaded.config.spaces.projects().is_empty());
        assert!(loaded.config.spaces.node.is_empty());
    }

    // TP-RANK-05's failure half: a broken overlay is reported, never fatal.
    #[test]
    fn managed_spaces_overlay_tolerates_a_broken_file() {
        let mut loaded = load_live_config_from_str("").unwrap();
        let diagnostics = merge_managed_spaces_str(&mut loaded.config, "not toml [");
        assert_eq!(diagnostics.len(), 1);
        assert!(
            diagnostics[0].contains("spaces.managed.toml"),
            "{diagnostics:?}"
        );
        assert!(loaded.config.spaces.rules().is_empty());
        // TP-MOVL-04: the failure half covers the containers too. A merge that
        // grew a new collection must not start half-applying a broken file.
        assert!(loaded.config.spaces.node.is_empty());
        assert!(loaded.invalid_sections.is_empty());
    }

    // TP-CHROME-149: the bars overlay is field-level last-wins — a field the
    // panel wrote wins over the hand-written value for that edge, and every
    // field it left unwritten keeps following the user's own file.
    #[test]
    fn bars_managed_overlay_overrides_written_fields_and_leaves_the_rest() {
        let mut loaded = load_live_config_from_str(
            r#"
[shell.bars.top]
enabled = true
style = "islands"
color = "mauve"
"#,
        )
        .unwrap();

        let diagnostics = merge_managed_bars_str(
            &mut loaded.config,
            r#"
[shell.bars.top]
style = "pills"
size = 1
border = "off"
"#,
        );

        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let top = &loaded.config.shell.bars.top;
        assert_eq!(top.style, "pills", "a written field wins over the user's");
        assert_eq!(top.size, 1);
        assert_eq!(top.border, Some(false));
        assert_eq!(
            top.color, "mauve",
            "an unwritten field keeps the user's value"
        );
        assert!(top.enabled, "an unwritten field keeps the user's value");
    }

    // TP-CHROME-149: the overlay is per-edge — an edge it never mentions is
    // an edge it never touches.
    #[test]
    fn bars_managed_overlay_leaves_unmentioned_edges_alone() {
        let mut loaded = load_live_config_from_str(
            r#"
[shell.bars.bottom]
enabled = true
style = "plain"
"#,
        )
        .unwrap();

        let diagnostics =
            merge_managed_bars_str(&mut loaded.config, "[shell.bars.top]\nstyle = \"pills\"\n");

        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(loaded.config.shell.bars.bottom.style, "plain");
        assert!(loaded.config.shell.bars.bottom.enabled);
        assert_eq!(loaded.config.shell.bars.top.style, "pills");
    }

    // TP-CHROME-149: an empty overlay is a no-op, not a complaint — parity
    // with the spaces overlay.
    #[test]
    fn bars_managed_overlay_accepts_an_empty_document() {
        let mut loaded = load_live_config_from_str("").unwrap();
        let before = loaded.config.shell.bars.clone();
        let diagnostics = merge_managed_bars_str(&mut loaded.config, "");
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(loaded.config.shell.bars, before);
    }

    // TP-CHROME-149: a machine file must never kill the user's config — a
    // broken overlay is reported and skipped, and the bars stay what the
    // user wrote.
    #[test]
    fn bars_managed_overlay_tolerates_a_broken_file() {
        let mut loaded = load_live_config_from_str("[shell.bars.top]\nenabled = true\n").unwrap();
        let diagnostics = merge_managed_bars_str(&mut loaded.config, "not toml [");
        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        assert!(
            diagnostics[0].contains("bars.managed.toml"),
            "{diagnostics:?}"
        );
        assert!(
            loaded.config.shell.bars.top.enabled,
            "config stays untouched"
        );
    }

    // TP-CHROME-149: `border` in the overlay is a word, not a bool, because
    // the panel has three states to persist — auto (the style decides), on,
    // off — and TOML has no way to write "explicitly none". Anything else is
    // refused by name and the user's own border survives.
    #[test]
    fn bars_managed_border_speaks_auto_on_off_and_refuses_the_rest() {
        let mut loaded = load_live_config_from_str("[shell.bars.top]\nborder = true\n").unwrap();

        let diagnostics =
            merge_managed_bars_str(&mut loaded.config, "[shell.bars.top]\nborder = \"auto\"\n");
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(
            loaded.config.shell.bars.top.border, None,
            "auto clears an explicit border back to the style's choice"
        );

        let mut loaded = load_live_config_from_str("[shell.bars.top]\nborder = true\n").unwrap();
        let diagnostics =
            merge_managed_bars_str(&mut loaded.config, "[shell.bars.top]\nborder = \"weird\"\n");
        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        assert!(diagnostics[0].contains("weird"), "{diagnostics:?}");
        assert_eq!(
            loaded.config.shell.bars.top.border,
            Some(true),
            "a refused word leaves the user's border standing"
        );
    }

    // TP-CHROME-149: an out-of-range size is refused rather than clamped —
    // the same doctrine `max_sections` follows — so a hand-edited overlay
    // cannot smuggle past the range the spec promises.
    #[test]
    fn bars_managed_size_out_of_range_is_refused_by_name() {
        let mut loaded = load_live_config_from_str("[shell.bars.top]\nsize = 4\n").unwrap();
        let diagnostics =
            merge_managed_bars_str(&mut loaded.config, "[shell.bars.top]\nsize = 99\n");
        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        assert!(diagnostics[0].contains("99"), "{diagnostics:?}");
        assert_eq!(
            loaded.config.shell.bars.top.size, 4,
            "the user's size survives a refused override"
        );
    }

    // TP-CHROME-151: the writer is read-modify-write — a second Apply on
    // another edge keeps what the first one wrote.
    #[test]
    fn upserting_bar_overrides_builds_and_updates_the_document() {
        let first = upsert_managed_bars_doc(
            "",
            &[(
                "top",
                ManagedBarOverride {
                    style: Some("pills".to_string()),
                    ..Default::default()
                },
            )],
        )
        .unwrap();
        assert!(first.contains("[shell.bars.top]"), "{first}");
        assert!(first.contains("style = \"pills\""), "{first}");

        let second = upsert_managed_bars_doc(
            &first,
            &[(
                "bottom",
                ManagedBarOverride {
                    enabled: Some(true),
                    ..Default::default()
                },
            )],
        )
        .unwrap();
        assert!(
            second.contains("style = \"pills\""),
            "the first edge survives"
        );
        assert!(second.contains("[shell.bars.bottom]"), "{second}");
        assert!(second.contains("enabled = true"), "{second}");
    }

    // TP-CHROME-151: a document the writer cannot read is refused, never
    // silently replaced — refusing loses one Apply, replacing loses every
    // earlier one.
    #[test]
    fn upserting_refuses_a_broken_existing_document() {
        let result =
            upsert_managed_bars_doc("not toml [", &[("top", ManagedBarOverride::default())]);
        assert!(result.is_err());
    }

    // TP-CHROME-149/151: what the writer writes, the merge reads back — the
    // two halves of the overlay meet in the middle.
    #[test]
    fn the_written_document_reads_back_through_the_merge() {
        let doc = upsert_managed_bars_doc(
            "",
            &[(
                "left",
                ManagedBarOverride {
                    enabled: Some(true),
                    size: Some(2),
                    style: Some("plain".to_string()),
                    border: Some("auto".to_string()),
                    color: Some("teal".to_string()),
                    background: Some("bg".to_string()),
                },
            )],
        )
        .unwrap();
        let mut loaded = load_live_config_from_str("").unwrap();
        loaded.config.shell.bars.left.border = Some(true);
        let diagnostics = merge_managed_bars_str(&mut loaded.config, &doc);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let left = &loaded.config.shell.bars.left;
        assert!(left.enabled);
        assert_eq!(left.size, 2);
        assert_eq!(left.style, "plain");
        assert_eq!(left.border, None, "auto travelled through the file");
        assert_eq!(left.color, "teal");
        assert_eq!(left.background, "bg");
    }

    #[test]
    fn load_live_config_isolates_invalid_spaces_section() {
        let loaded = load_live_config_from_str(
            r#"
[spaces]
split = 5

[ui]
mouse_capture = false
"#,
        )
        .unwrap();

        // A malformed [spaces] is isolated: other sections still apply.
        assert!(loaded.config.spaces.rules().is_empty());
        assert!(!loaded.config.ui.mouse_capture);
        assert!(loaded.invalid_sections.contains(&"spaces".to_string()));
        assert!(
            loaded.diagnostics.iter().any(|d| d.contains("spaces")),
            "invalid spaces section should produce a diagnostic: {:?}",
            loaded.diagnostics
        );
    }

    #[test]
    fn load_live_config_isolates_invalid_chat_drawer_mode() {
        let loaded = load_live_config_from_str(
            r#"
[ui]
chat_drawer_mode = "bogus"

[update]
version_check = false
"#,
        )
        .unwrap();

        // An unknown drawer mode follows the section-isolation contract:
        // [ui] falls back to defaults, the failure is reported, and the
        // rest of the config still applies.
        assert_eq!(
            loaded.config.ui.chat_drawer_mode,
            crate::config::ChatDrawerModeConfig::AllActive
        );
        assert!(!loaded.config.update.version_check);
        assert!(loaded.invalid_sections.contains(&"ui".to_string()));
        assert!(
            loaded.diagnostics.iter().any(|d| d.contains("ui config")),
            "invalid chat_drawer_mode should surface a ui diagnostic: {:?}",
            loaded.diagnostics
        );
    }

    #[test]
    fn load_live_config_isolates_invalid_projects_section() {
        let loaded = load_live_config_from_str(
            r#"
[projects]
pinned = 5

[ui]
mouse_capture = false
"#,
        )
        .unwrap();

        // A malformed [projects] is isolated: other sections still apply,
        // the bad section is recorded, and nothing panics.
        assert!(loaded.invalid_sections.contains(&"projects".to_string()));
        assert!(!loaded.config.ui.mouse_capture);
        assert!(
            loaded.diagnostics.iter().any(|d| d.contains("projects")),
            "invalid projects section should produce a diagnostic: {:?}",
            loaded.diagnostics
        );
    }
}
