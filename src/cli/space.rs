//! `herdr space` — sidebar space grouping from the command line.
//!
//! Promotion writes to `spaces.managed.toml`, never to the user's own
//! config.toml: the machine edits the machine's file, the human keeps theirs,
//! and user-authored rules always win first-match (TP-RANK-05). A promote is
//! one command for an agent: write the rule, ask the running server to reload,
//! and the sidebar regroups in place.

use std::path::{Path, PathBuf};

use crate::api::schema::{EmptyParams, Method, Request};

const MANAGED_HEADER: &str = "# Managed by `herdr space promote` - do not hand-edit.\n\
# Hand-written rules belong in config.toml; they load first and win\n\
# first-match against everything in this file.\n\n";

pub(super) fn run_space_command(args: &[String]) -> std::io::Result<i32> {
    match args.first().map(|arg| arg.as_str()) {
        Some("promote") => space_promote(&args[1..]),
        Some("demote") => space_demote(&args[1..]),
        Some("list") => space_list(&args[1..]),
        Some("help" | "--help" | "-h") => {
            print_space_help();
            Ok(0)
        }
        _ => {
            print_space_help();
            Ok(2)
        }
    }
}

fn print_space_help() {
    eprintln!("usage: herdr space promote <branch> [--repo <root>] [--as module|project]");
    eprintln!(
        "                           [--label <text>] [--icon <glyph>] [--key <key>] [--dry-run]"
    );
    eprintln!("       herdr space demote <key|branch> [--dry-run]");
    eprintln!("       herdr space list");
}

/// Everything a promote is going to write, decided before any file is touched.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PromotePlan {
    pub repo_root: PathBuf,
    pub branch: String,
    pub key: String,
    pub label: String,
    pub icon: Option<String>,
    /// `Some` promotes past module rank: the space also becomes a top-level
    /// project of its own (TP-RANK-02).
    pub project: Option<ProjectPlan>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ProjectPlan {
    pub key: String,
    pub name: String,
}

/// A branch name reduced to key material: ascii-lowered, every run of
/// non-alphanumerics one dash, never empty.
pub(crate) fn slug_for_branch(branch: &str) -> String {
    let mut slug = String::new();
    let mut pending_dash = false;
    for ch in branch.chars() {
        if ch.is_ascii_alphanumeric() {
            if pending_dash && !slug.is_empty() {
                slug.push('-');
            }
            pending_dash = false;
            slug.push(ch.to_ascii_lowercase());
        } else {
            pending_dash = true;
        }
    }
    if slug.is_empty() {
        "branch".to_string()
    } else {
        slug
    }
}

/// The managed rule key for a checkout: `<repo-dir>:<branch-slug>`.
pub(crate) fn managed_key(repo_root: &Path, branch: &str) -> String {
    let repo = repo_root
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("repo");
    format!("{repo}:{}", slug_for_branch(branch))
}

/// Upsert the plan into the managed overlay document. Replaces the rule (and
/// project) that already carries the plan's key, so promoting twice updates in
/// place instead of stacking duplicates (TP-RANK-03).
pub(crate) fn upsert_managed(content: &str, plan: &PromotePlan) -> Result<String, String> {
    let mut root = parse_managed(content)?;

    let mut rule = toml::map::Map::new();
    rule.insert(
        "repo".into(),
        toml::Value::String(plan.repo_root.display().to_string()),
    );
    rule.insert(
        "match".into(),
        toml::Value::Array(vec![toml::Value::String(plan.branch.clone())]),
    );
    rule.insert("key".into(), toml::Value::String(plan.key.clone()));
    rule.insert("label".into(), toml::Value::String(plan.label.clone()));
    if let Some(icon) = &plan.icon {
        rule.insert("icon".into(), toml::Value::String(icon.clone()));
    }
    upsert_by_key(
        ensure_spaces_array(&mut root, "split")?,
        &plan.key,
        toml::Value::Table(rule),
    );

    if let Some(project) = &plan.project {
        let mut entry = toml::map::Map::new();
        entry.insert("key".into(), toml::Value::String(project.key.clone()));
        entry.insert("name".into(), toml::Value::String(project.name.clone()));
        if let Some(icon) = &plan.icon {
            entry.insert("icon".into(), toml::Value::String(icon.clone()));
        }
        entry.insert(
            "spaces".into(),
            toml::Value::Array(vec![toml::Value::String(plan.key.clone())]),
        );
        upsert_by_key(
            ensure_spaces_array(&mut root, "project")?,
            &project.key,
            toml::Value::Table(entry),
        );
    }

    serialize_managed(&root)
}

fn parse_managed(content: &str) -> Result<toml::Value, String> {
    if content.trim().is_empty() {
        Ok(toml::Value::Table(toml::map::Map::new()))
    } else {
        content
            .parse()
            .map_err(|err| format!("managed overlay is not valid toml: {err}"))
    }
}

fn serialize_managed(root: &toml::Value) -> Result<String, String> {
    let body = toml::to_string_pretty(root)
        .map_err(|err| format!("managed overlay serialize error: {err}"))?;
    Ok(format!("{MANAGED_HEADER}{body}"))
}

fn ensure_spaces_array<'a>(
    root: &'a mut toml::Value,
    name: &str,
) -> Result<&'a mut Vec<toml::Value>, String> {
    let table = root
        .as_table_mut()
        .ok_or_else(|| "managed overlay top level must be a table".to_string())?;
    let spaces = table
        .entry("spaces")
        .or_insert_with(|| toml::Value::Table(toml::map::Map::new()));
    let spaces = spaces
        .as_table_mut()
        .ok_or_else(|| "`spaces` must be a table".to_string())?;
    spaces
        .entry(name)
        .or_insert_with(|| toml::Value::Array(Vec::new()))
        .as_array_mut()
        .ok_or_else(|| format!("`spaces.{name}` must be an array"))
}

fn upsert_by_key(entries: &mut Vec<toml::Value>, key: &str, value: toml::Value) {
    let existing = entries
        .iter_mut()
        .find(|entry| entry.get("key").and_then(|k| k.as_str()) == Some(key));
    match existing {
        Some(entry) => *entry = value,
        None => entries.push(value),
    }
}

/// Remove managed entries matching `target` — a rule key, a branch name, or a
/// branch slug. Projects that lose every member leave with them. Returns the
/// new document and how many entries were removed; user-authored config is
/// never touched (TP-RANK-04).
pub(crate) fn remove_managed(content: &str, target: &str) -> Result<(String, usize), String> {
    if content.trim().is_empty() {
        return Ok((content.to_string(), 0));
    }
    let mut root = parse_managed(content)?;
    let slug_suffix = format!(":{}", slug_for_branch(target));
    let mut removed = 0usize;
    let mut removed_keys: Vec<String> = Vec::new();

    if let Ok(split) = ensure_spaces_array(&mut root, "split") {
        split.retain(|entry| {
            let key = entry
                .get("key")
                .and_then(|k| k.as_str())
                .unwrap_or_default();
            let hit = key == target
                || key.ends_with(&slug_suffix)
                || entry
                    .get("match")
                    .and_then(|patterns| patterns.as_array())
                    .is_some_and(|patterns| {
                        patterns
                            .iter()
                            .any(|pattern| pattern.as_str() == Some(target))
                    });
            if hit {
                removed += 1;
                removed_keys.push(key.to_string());
            }
            !hit
        });
    }

    if let Ok(projects) = ensure_spaces_array(&mut root, "project") {
        projects.retain_mut(|entry| {
            let key = entry
                .get("key")
                .and_then(|k| k.as_str())
                .unwrap_or_default()
                .to_string();
            if key == target {
                removed += 1;
                return false;
            }
            let Some(spaces) = entry.get_mut("spaces").and_then(|s| s.as_array_mut()) else {
                return true;
            };
            spaces.retain(|space| {
                space
                    .as_str()
                    .map(|space| !removed_keys.iter().any(|removed| removed == space))
                    .unwrap_or(true)
            });
            // A project that lost every member has nothing left to head.
            if spaces.is_empty() {
                removed += 1;
                false
            } else {
                true
            }
        });
    }

    if removed == 0 {
        return Ok((content.to_string(), 0));
    }
    Ok((serialize_managed(&root)?, removed))
}

/// Walk up from `start` to the checkout root — the first directory carrying a
/// `.git` entry (a directory in a main checkout, a file in a linked worktree).
pub(crate) fn discover_repo_root(start: &Path) -> Option<PathBuf> {
    let mut current = Some(start);
    while let Some(dir) = current {
        if dir.join(".git").exists() {
            return Some(dir.to_path_buf());
        }
        current = dir.parent();
    }
    None
}

fn space_promote(args: &[String]) -> std::io::Result<i32> {
    let mut target = None;
    let mut repo = None;
    let mut as_project = false;
    let mut label = None;
    let mut icon = None;
    let mut key = None;
    let mut dry_run = false;

    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--repo" | "--as" | "--label" | "--icon" | "--key" => {
                let Some(value) = args.get(index + 1) else {
                    eprintln!("missing value for {}", args[index]);
                    return Ok(2);
                };
                match args[index].as_str() {
                    "--repo" => repo = Some(value.clone()),
                    "--as" => match value.as_str() {
                        "module" => as_project = false,
                        "project" => as_project = true,
                        other => {
                            eprintln!("--as takes `module` or `project`, got {other:?}");
                            return Ok(2);
                        }
                    },
                    "--label" => label = Some(value.clone()),
                    "--icon" => icon = Some(value.clone()),
                    _ => key = Some(value.clone()),
                }
                index += 2;
            }
            "--dry-run" => {
                dry_run = true;
                index += 1;
            }
            other if other.starts_with("--") => {
                eprintln!("unknown flag {other}");
                print_space_help();
                return Ok(2);
            }
            _ if target.is_none() => {
                target = Some(args[index].clone());
                index += 1;
            }
            other => {
                eprintln!("unexpected argument {other}");
                print_space_help();
                return Ok(2);
            }
        }
    }

    let Some(branch) = target else {
        print_space_help();
        return Ok(2);
    };

    let repo_root = match repo {
        Some(repo) => crate::worktree::expand_tilde_absolute_path(repo.trim()),
        None => {
            let cwd = std::env::current_dir()?;
            match discover_repo_root(&cwd) {
                Some(root) => root,
                None => {
                    eprintln!(
                        "not inside a git checkout; pass --repo <root> to say which repository \
                         the branch belongs to"
                    );
                    return Ok(1);
                }
            }
        }
    };

    let key = key.unwrap_or_else(|| managed_key(&repo_root, &branch));
    let label = label.unwrap_or_else(|| branch.clone());
    let plan = PromotePlan {
        project: as_project.then(|| ProjectPlan {
            key: format!("project:{}", slug_for_branch(&branch)),
            name: label.clone(),
        }),
        repo_root,
        branch,
        key,
        label,
        icon,
    };

    let path = crate::config::managed_spaces_path();
    let current = std::fs::read_to_string(&path).unwrap_or_default();
    let updated = match upsert_managed(&current, &plan) {
        Ok(updated) => updated,
        Err(err) => {
            eprintln!("{err}");
            return Ok(1);
        }
    };

    if dry_run {
        print!("{updated}");
        return Ok(0);
    }

    std::fs::write(&path, &updated)?;
    println!(
        "promoted {} to {} ({})",
        plan.branch,
        if plan.project.is_some() {
            "a project of its own"
        } else {
            "its own module space"
        },
        plan.key
    );
    report_reload();
    Ok(0)
}

fn space_demote(args: &[String]) -> std::io::Result<i32> {
    let mut target = None;
    let mut dry_run = false;
    for arg in args {
        match arg.as_str() {
            "--dry-run" => dry_run = true,
            other if other.starts_with("--") => {
                eprintln!("unknown flag {other}");
                return Ok(2);
            }
            _ if target.is_none() => target = Some(arg.clone()),
            other => {
                eprintln!("unexpected argument {other}");
                return Ok(2);
            }
        }
    }
    let Some(target) = target else {
        print_space_help();
        return Ok(2);
    };

    let path = crate::config::managed_spaces_path();
    let current = std::fs::read_to_string(&path).unwrap_or_default();
    let (updated, removed) = match remove_managed(&current, &target) {
        Ok(result) => result,
        Err(err) => {
            eprintln!("{err}");
            return Ok(1);
        }
    };

    if removed == 0 {
        if user_config_mentions(&target) {
            println!(
                "no managed rule matches {target:?}, but config.toml has one — demote never \
                 touches your own file; hand-edit it there"
            );
            return Ok(0);
        }
        println!("no managed rule matches {target:?}");
        return Ok(1);
    }

    if dry_run {
        print!("{updated}");
        return Ok(0);
    }

    std::fs::write(&path, &updated)?;
    println!(
        "removed {removed} managed entr{}",
        if removed == 1 { "y" } else { "ies" }
    );
    report_reload();
    Ok(0)
}

fn space_list(args: &[String]) -> std::io::Result<i32> {
    if !args.is_empty() {
        eprintln!("usage: herdr space list");
        return Ok(2);
    }
    let loaded = crate::config::Config::load();
    if loaded.config.spaces.split.is_empty() && loaded.config.spaces.project.is_empty() {
        println!("no space rules configured");
        return Ok(0);
    }
    let managed = std::fs::read_to_string(crate::config::managed_spaces_path()).unwrap_or_default();
    for rule in loaded.config.spaces.rules() {
        let source = if managed.contains(&format!("key = \"{}\"", rule.key)) {
            "managed"
        } else {
            "config"
        };
        println!(
            "rule    {:<28} {:<20} [{source}] {}",
            rule.key,
            rule.label,
            rule.repo_root.display()
        );
    }
    for project in loaded.config.spaces.projects() {
        let source = if managed.contains(&format!("key = \"{}\"", project.key)) {
            "managed"
        } else {
            "config"
        };
        println!(
            "project {:<28} {:<20} [{source}]",
            project.key, project.name
        );
    }
    Ok(0)
}

/// Does the user's own config.toml (not the merged view) mention this target?
fn user_config_mentions(target: &str) -> bool {
    let Ok(content) = std::fs::read_to_string(crate::config::config_path()) else {
        return false;
    };
    content.contains(target)
}

/// Ask the running server to reload; promotion still lands without one.
fn report_reload() {
    match super::send_request(&Request {
        id: "cli:space:reload-config".into(),
        method: Method::ServerReloadConfig(EmptyParams::default()),
    }) {
        Ok(response) if response.get("error").is_none() => {
            println!("server reloaded; the sidebar regrouped in place");
        }
        Ok(response) => eprintln!("server reload answered with an error: {response}"),
        Err(_) => println!("no running server; the grouping applies on next start"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan(as_project: bool) -> PromotePlan {
        PromotePlan {
            repo_root: PathBuf::from("/repo/herdr"),
            branch: "worktree/Tiling".into(),
            key: "herdr:worktree-tiling".into(),
            label: "Tiling arastirmasi".into(),
            icon: Some("🧱".into()),
            project: as_project.then(|| ProjectPlan {
                key: "project:worktree-tiling".into(),
                name: "Tiling arastirmasi".into(),
            }),
        }
    }

    #[test]
    fn slug_reduces_branches_to_key_material() {
        assert_eq!(slug_for_branch("feat/T4F-Alpha"), "feat-t4f-alpha");
        assert_eq!(slug_for_branch("worktree/Tiling"), "worktree-tiling");
        assert_eq!(slug_for_branch("***"), "branch", "never empty");
    }

    #[test]
    fn managed_key_is_repo_dir_and_slug() {
        assert_eq!(
            managed_key(Path::new("/repo/herdr"), "worktree/Tiling"),
            "herdr:worktree-tiling"
        );
    }

    // TP-RANK-01: promotion writes a managed rule the loader will pick up.
    #[test]
    fn upsert_writes_a_managed_rule_with_the_header() {
        let updated = upsert_managed("", &plan(false)).expect("upsert");
        assert!(updated.starts_with("# Managed by"));
        let value: toml::Value = updated.parse().expect("managed file parses");
        let split = value["spaces"]["split"].as_array().expect("split array");
        assert_eq!(split.len(), 1);
        assert_eq!(split[0]["key"].as_str(), Some("herdr:worktree-tiling"));
        assert_eq!(split[0]["repo"].as_str(), Some("/repo/herdr"));
        assert_eq!(split[0]["match"][0].as_str(), Some("worktree/Tiling"));
        assert_eq!(split[0]["icon"].as_str(), Some("🧱"));
        assert!(
            value["spaces"].get("project").is_none(),
            "module rank writes no project"
        );
    }

    // TP-RANK-03: promoting the same target twice updates in place.
    #[test]
    fn upsert_is_idempotent_per_key() {
        let first = upsert_managed("", &plan(false)).expect("first upsert");
        let mut second_plan = plan(false);
        second_plan.label = "Tiling v2".into();
        let second = upsert_managed(&first, &second_plan).expect("second upsert");
        let value: toml::Value = second.parse().expect("managed file parses");
        let split = value["spaces"]["split"].as_array().expect("split array");
        assert_eq!(split.len(), 1, "no duplicate rule");
        assert_eq!(split[0]["label"].as_str(), Some("Tiling v2"));
    }

    // TP-RANK-02: project rank writes the rule and the umbrella together.
    #[test]
    fn upsert_as_project_writes_the_umbrella_too() {
        let updated = upsert_managed("", &plan(true)).expect("upsert");
        let value: toml::Value = updated.parse().expect("managed file parses");
        let projects = value["spaces"]["project"]
            .as_array()
            .expect("project array");
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0]["key"].as_str(), Some("project:worktree-tiling"));
        assert_eq!(
            projects[0]["spaces"][0].as_str(),
            Some("herdr:worktree-tiling"),
            "the umbrella claims exactly the promoted space"
        );
    }

    // TP-RANK-04: demote removes managed entries — and only managed entries.
    #[test]
    fn remove_takes_the_rule_and_its_orphaned_project() {
        let content = upsert_managed("", &plan(true)).expect("upsert");
        let (updated, removed) = remove_managed(&content, "worktree/Tiling").expect("remove");
        assert_eq!(removed, 2, "the rule and its emptied project");
        let value: toml::Value = updated.parse().expect("managed file parses");
        let empty = |name: &str| {
            value
                .get("spaces")
                .and_then(|spaces| spaces.get(name))
                .and_then(|entries| entries.as_array())
                .is_none_or(|entries| entries.is_empty())
        };
        assert!(empty("split"), "{updated}");
        assert!(empty("project"), "{updated}");
    }

    #[test]
    fn remove_without_a_match_changes_nothing() {
        let content = upsert_managed("", &plan(false)).expect("upsert");
        let (updated, removed) = remove_managed(&content, "no-such-branch").expect("remove");
        assert_eq!(removed, 0);
        assert_eq!(updated, content);
    }
}
