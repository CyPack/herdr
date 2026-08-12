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
            Vec::new()
        }
        Err(err) => vec![format!(
            "spaces.managed.toml parse error: {err}; ignoring the managed overlay"
        )],
    }
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
