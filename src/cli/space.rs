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
        Some("move") => space_move(&args[1..]),
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
    eprintln!("       herdr space move <branch> (--under|--beside|--above <node-key> | --top |");
    eprintln!("                           --new-group <name>) [--repo <root>] [--dry-run]");
    eprintln!("       herdr space demote <key|branch> [--dry-run]");
    eprintln!("       herdr space list");
}

/// Where a move sends the checkout: relative to an existing node, to top
/// level, or under a group created on the way (K5).
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum MoveDest {
    Node {
        key: String,
        op: crate::spaces::MoveOp,
    },
    Top,
    NewGroup {
        name: String,
    },
}

/// The `herdr space move` invocation, parsed but not yet resolved.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MoveArgs {
    pub branch: String,
    pub dest: MoveDest,
    pub repo: Option<String>,
    pub dry_run: bool,
}

/// Parse `herdr space move` arguments. Exactly one destination is required —
/// a move with none or two is a mistake to refuse, never to guess at
/// (TP-RANK-11).
pub(crate) fn parse_move_args(args: &[String]) -> Result<MoveArgs, String> {
    let mut branch = None;
    let mut dest: Option<MoveDest> = None;
    let mut repo = None;
    let mut dry_run = false;

    fn claim_dest(slot: &mut Option<MoveDest>, new: MoveDest) -> Result<(), String> {
        if slot.is_some() {
            return Err("pick exactly one destination".to_string());
        }
        *slot = Some(new);
        Ok(())
    }

    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            flag @ ("--under" | "--beside" | "--above" | "--new-group" | "--repo") => {
                let Some(value) = args.get(index + 1) else {
                    return Err(format!("missing value for {flag}"));
                };
                match flag {
                    "--under" | "--beside" | "--above" => {
                        let op = match flag {
                            "--under" => crate::spaces::MoveOp::Under,
                            "--beside" => crate::spaces::MoveOp::Beside,
                            _ => crate::spaces::MoveOp::Above,
                        };
                        claim_dest(
                            &mut dest,
                            MoveDest::Node {
                                key: value.clone(),
                                op,
                            },
                        )?;
                    }
                    "--new-group" => {
                        claim_dest(
                            &mut dest,
                            MoveDest::NewGroup {
                                name: value.clone(),
                            },
                        )?;
                    }
                    _ => repo = Some(value.clone()),
                }
                index += 2;
            }
            "--top" => {
                claim_dest(&mut dest, MoveDest::Top)?;
                index += 1;
            }
            "--dry-run" => {
                dry_run = true;
                index += 1;
            }
            other if other.starts_with("--") => {
                return Err(format!("unknown flag {other}"));
            }
            _ if branch.is_none() => {
                branch = Some(args[index].clone());
                index += 1;
            }
            other => {
                return Err(format!("unexpected argument {other:?}"));
            }
        }
    }

    let branch = branch.ok_or_else(|| "a branch to move is required".to_string())?;
    let dest = dest.ok_or_else(|| {
        "pick a destination: --under/--beside/--above <node-key>, --top, or --new-group <name>"
            .to_string()
    })?;
    Ok(MoveArgs {
        branch,
        dest,
        repo,
        dry_run,
    })
}

/// The label and icon the managed overlay already carries for `key`, so a
/// move re-writes the rule without losing what a promote once chose
/// (TP-RANK-11).
pub(crate) fn existing_rule_style(content: &str, key: &str) -> Option<(String, Option<String>)> {
    let root: toml::Value = content.parse().ok()?;
    let split = root.get("spaces")?.get("split")?.as_array()?;
    let rule = split
        .iter()
        .find(|entry| entry.get("key").and_then(|k| k.as_str()) == Some(key))?;
    let label = rule.get("label")?.as_str()?.to_string();
    let icon = rule
        .get("icon")
        .and_then(|icon| icon.as_str())
        .map(str::to_string);
    Some((label, icon))
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
    /// The node the rule's bucket hangs under; `None` stays top level. A move
    /// is a promote that carries a parent (TP-RANK-08).
    pub parent: Option<String>,
    /// A `[[spaces.node]]` entry written in the same update, so "move under a
    /// new group" lands the group and the membership atomically (TP-RANK-10).
    pub node: Option<NodePlan>,
}

/// A managed `[[spaces.node]]` entry a move may create on the way.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct NodePlan {
    pub key: String,
    pub name: String,
    pub parent: Option<String>,
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

/// Upsert one managed `[[spaces.node]]` entry on its own — the 2-click
/// module road (TP-DOTS-05). No split rule rides along: a module born from
/// a header has no branch yet, and writing an empty rule would steal the
/// first matching branch from whatever rule owns it today.
pub(crate) fn upsert_managed_node(content: &str, node: &NodePlan) -> Result<String, String> {
    let mut root = parse_managed(content)?;
    let mut entry = toml::map::Map::new();
    entry.insert("key".into(), toml::Value::String(node.key.clone()));
    entry.insert("name".into(), toml::Value::String(node.name.clone()));
    if let Some(parent) = &node.parent {
        entry.insert("parent".into(), toml::Value::String(parent.clone()));
    }
    upsert_by_key(
        ensure_spaces_array(&mut root, "node")?,
        &node.key,
        toml::Value::Table(entry),
    );
    serialize_managed(&root)
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
    // TP-RANK-08: the parent is written only when the plan carries one, so a
    // plain promote (and a move back to top level) keeps the field out.
    if let Some(parent) = &plan.parent {
        rule.insert("parent".into(), toml::Value::String(parent.clone()));
    }
    upsert_by_key(
        ensure_spaces_array(&mut root, "split")?,
        &plan.key,
        toml::Value::Table(rule),
    );

    // TP-RANK-10: a move under a new group writes the `[[spaces.node]]` entry
    // in the same document update as the membership that points at it.
    if let Some(node) = &plan.node {
        let mut entry = toml::map::Map::new();
        entry.insert("key".into(), toml::Value::String(node.key.clone()));
        entry.insert("name".into(), toml::Value::String(node.name.clone()));
        if let Some(parent) = &node.parent {
            entry.insert("parent".into(), toml::Value::String(parent.clone()));
        }
        upsert_by_key(
            ensure_spaces_array(&mut root, "node")?,
            &node.key,
            toml::Value::Table(entry),
        );
    }

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
/// Whether the overlay — the file the machine owns — declares this module.
///
/// TP-MOD-26: the answer decides whether a delete verb may be offered at all.
/// A module written by hand into `config.toml` reaches the tree through the
/// same merge and looks identical on screen, but no machine road can take it
/// back; a verb that cannot keep its word is worse than a missing one.
///
/// Only `[[spaces.node]]` counts. A split rule that happens to carry the same
/// key is a bucket, and buckets are taken back by their own verb.
pub(crate) fn managed_has_node(content: &str, key: &str) -> bool {
    let Ok(root) = content.parse::<toml::Value>() else {
        return false;
    };
    root.get("spaces")
        .and_then(|spaces| spaces.get("node"))
        .and_then(|nodes| nodes.as_array())
        .is_some_and(|nodes| {
            nodes
                .iter()
                .any(|entry| entry.get("key").and_then(|k| k.as_str()) == Some(key))
        })
}

/// Take one `[[spaces.node]]` entry out of the overlay.
///
/// TP-MOD-29: deliberately narrower than [`remove_managed`], which hunts a
/// *branch* across the split and project arrays and matches key suffixes to
/// find the rule that claimed it. Here the key is already exact and the target
/// is one array — widening this to "everything that mentions the key" would
/// let deleting a module take out the branch rule standing next to it.
///
/// Children are not touched. A module that loses its parent is re-seated at
/// top level by `validate_node_forest`, with a diagnostic (TP-MOD-30): losing
/// the container must not mean losing what was inside it.
pub(crate) fn remove_managed_node(content: &str, key: &str) -> Result<(String, usize), String> {
    if content.trim().is_empty() {
        return Ok((content.to_string(), 0));
    }
    let mut root = parse_managed(content)?;
    let mut removed = 0usize;
    if let Ok(nodes) = ensure_spaces_array(&mut root, "node") {
        nodes.retain(|entry| {
            let hit = entry.get("key").and_then(|k| k.as_str()) == Some(key);
            if hit {
                removed += 1;
            }
            !hit
        });
    }
    if removed == 0 {
        return Ok((content.to_string(), 0));
    }
    Ok((serialize_managed(&root)?, removed))
}

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
        parent: None,
        node: None,
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

/// `herdr space move` — re-hang a checkout's bucket relative to a node, on
/// the same managed-overlay road a promote takes (TP-RANK-11).
fn space_move(args: &[String]) -> std::io::Result<i32> {
    let parsed = match parse_move_args(args) {
        Ok(parsed) => parsed,
        Err(err) => {
            eprintln!("{err}");
            print_space_help();
            return Ok(2);
        }
    };

    let repo_root = match parsed.repo {
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

    let loaded = crate::config::Config::load();
    let (nodes, _) = crate::spaces::validate_node_forest(
        loaded.config.spaces.nodes(),
        &crate::spaces::split_parent_map(&loaded.config.spaces.rules()),
    );
    let (parent, node) = match parsed.dest {
        MoveDest::Node { key, op } => match crate::spaces::move_parent_for(&nodes, &key, op) {
            Ok(parent) => (parent, None),
            Err(err) => {
                eprintln!("{err}");
                return Ok(1);
            }
        },
        MoveDest::Top => (None, None),
        MoveDest::NewGroup { name } => {
            let key = format!("group:{}", slug_for_branch(&name));
            (
                Some(key.clone()),
                Some(NodePlan {
                    key,
                    name,
                    parent: None,
                }),
            )
        }
    };

    let path = crate::config::managed_spaces_path();
    let current = std::fs::read_to_string(&path).unwrap_or_default();
    let key = managed_key(&repo_root, &parsed.branch);
    // A move is a re-hang, not a re-style: whatever label and icon the rule
    // already carries stay with it.
    let (label, icon) =
        existing_rule_style(&current, &key).unwrap_or_else(|| (parsed.branch.clone(), None));
    let plan = PromotePlan {
        project: None,
        repo_root,
        branch: parsed.branch,
        key,
        label,
        icon,
        parent,
        node,
    };

    let updated = match upsert_managed(&current, &plan) {
        Ok(updated) => updated,
        Err(err) => {
            eprintln!("{err}");
            return Ok(1);
        }
    };

    if parsed.dry_run {
        print!("{updated}");
        return Ok(0);
    }

    std::fs::write(&path, &updated)?;
    match &plan.parent {
        Some(parent) => println!("moved {} under {parent} ({})", plan.branch, plan.key),
        None => println!("moved {} to top level ({})", plan.branch, plan.key),
    }
    if user_config_mentions(&plan.branch) {
        println!("note: config.toml also mentions this branch; hand-written rules win first-match");
    }
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
    let managed = std::fs::read_to_string(crate::config::managed_spaces_path()).unwrap_or_default();
    for line in space_list_lines(&loaded.config, &managed) {
        println!("{line}");
    }
    Ok(0)
}

/// The lines `herdr space list` prints, as data.
///
/// Kept apart from printing so the listing is testable, and because this is
/// the only surface that answers "did the thing I just created land?". A
/// module written into the managed overlay is otherwise invisible until a
/// branch happens to join it: it draws no header of its own, so a listing
/// that skipped containers left the user with no way to tell a module that
/// exists from one that was never written (TP-MOVL-05).
pub(crate) fn space_list_lines(config: &crate::config::Config, managed: &str) -> Vec<String> {
    // `key = "x"` is how both files spell an entry's identity, so a hit in
    // the overlay text is what makes an entry managed rather than authored.
    let source_of = |key: &str| {
        if managed.contains(&format!("key = \"{key}\"")) {
            "managed"
        } else {
            "config"
        }
    };

    let spaces = &config.spaces;
    if spaces.split.is_empty() && spaces.project.is_empty() && spaces.node.is_empty() {
        return vec!["no space rules configured".to_owned()];
    }

    let mut lines = Vec::new();
    for rule in spaces.rules() {
        lines.push(format!(
            "rule    {:<28} {:<20} [{}] {}",
            rule.key,
            rule.label,
            source_of(&rule.key),
            rule.repo_root.display()
        ));
    }
    for project in spaces.projects() {
        lines.push(format!(
            "project {:<28} {:<20} [{}]",
            project.key,
            project.name,
            source_of(&project.key)
        ));
    }
    // Containers last: they are the tree's scaffolding, and reading the rules
    // that claim branches first matches how the sidebar is built.
    for node in &spaces.node {
        let name = if node.name.is_empty() {
            node.key.as_str()
        } else {
            node.name.as_str()
        };
        let parent = if node.parent.is_empty() {
            "top level".to_owned()
        } else {
            format!("under {}", node.parent)
        };
        lines.push(format!(
            "node    {:<28} {:<20} [{}] {parent}",
            node.key,
            name,
            source_of(&node.key)
        ));
    }
    lines
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

    /// A config carrying exactly the spaces entries a test needs.
    fn config_with(spaces_toml: &str) -> crate::config::Config {
        toml::from_str(spaces_toml).expect("the fixture is valid config")
    }

    // TP-MOVL-05: a container the user created is listed. Without this the
    // only way to tell "my module was written" from "my module was lost" is
    // to open a file the user is told never to hand-edit.
    #[test]
    fn space_list_names_the_containers_too() {
        let config = config_with(
            r#"
[[spaces.split]]
repo = "/home/a/panel"
match = ["feat/*"]
key = "panel:user"
label = "User"

[[spaces.node]]
key = "group:remote-audio"
name = "UZAKTAN SES"
parent = "project:herdr"
"#,
        );

        let lines = space_list_lines(&config, "");
        let node_lines: Vec<&String> = lines
            .iter()
            .filter(|line| line.starts_with("node "))
            .collect();
        assert_eq!(node_lines.len(), 1, "{lines:?}");
        assert!(
            node_lines[0].contains("group:remote-audio"),
            "{node_lines:?}"
        );
        assert!(node_lines[0].contains("UZAKTAN SES"), "{node_lines:?}");
        assert!(
            node_lines[0].contains("under project:herdr"),
            "the parent is what tells the user where the module went: {node_lines:?}"
        );
        assert!(
            lines.iter().any(|line| line.starts_with("rule ")),
            "rules still listed: {lines:?}"
        );
    }

    // TP-MOVL-06: the source tag separates the two writing homes. Pointing a
    // user at the wrong one sends them to hand-edit `spaces.managed.toml`,
    // which the promote/move commands own and will overwrite.
    #[test]
    fn space_list_marks_where_each_container_was_written() {
        let config = config_with(
            r#"
[[spaces.node]]
key = "group:by-hand"
name = "By hand"

[[spaces.node]]
key = "group:by-tool"
name = "By tool"
"#,
        );

        let managed = "[[spaces.node]]\nkey = \"group:by-tool\"\n";
        let lines = space_list_lines(&config, managed);
        let by_hand = lines
            .iter()
            .find(|line| line.contains("group:by-hand"))
            .expect("the hand-written node is listed");
        let by_tool = lines
            .iter()
            .find(|line| line.contains("group:by-tool"))
            .expect("the tool-written node is listed");
        assert!(by_hand.contains("[config]"), "{by_hand}");
        assert!(by_tool.contains("[managed]"), "{by_tool}");
    }

    // TP-MOVL-07: a tree made only of containers is a configured tree. The
    // empty check counted rules and projects, so a config holding nothing but
    // modules reported "no space rules configured" — telling the user their
    // work was gone while it sat two lines below in the same file.
    #[test]
    fn space_list_does_not_call_a_container_only_tree_empty() {
        let config = config_with(
            r#"
[[spaces.node]]
key = "group:only"
name = "Only"
"#,
        );

        let lines = space_list_lines(&config, "");
        assert_ne!(
            lines,
            vec!["no space rules configured".to_owned()],
            "containers count as configuration"
        );
        assert!(lines.iter().any(|line| line.contains("group:only")));
    }

    // The empty answer still exists, and still says so.
    #[test]
    fn space_list_reports_an_empty_tree_as_empty() {
        let config = config_with("");
        assert_eq!(
            space_list_lines(&config, ""),
            vec!["no space rules configured".to_owned()]
        );
    }

    // A container with no name falls back to its key rather than printing a
    // blank column the reader cannot match to anything.
    #[test]
    fn space_list_falls_back_to_the_key_when_a_container_has_no_name() {
        let config = config_with("[[spaces.node]]\nkey = \"group:nameless\"\n");
        let line = space_list_lines(&config, "")
            .into_iter()
            .find(|line| line.starts_with("node "))
            .expect("listed");
        assert!(line.contains("group:nameless"), "{line}");
        assert!(line.contains("top level"), "{line}");
    }

    // TP-MOD-29: taking a module back takes the module and nothing else. The
    // branch-facing `remove_managed` walks the split and project arrays and
    // matches on key *suffixes*, which is right for a branch and wrong here: a
    // module named after the branch beside it would drag that branch's rule
    // out with it. The node road matches whole keys and touches one array.
    #[test]
    fn remove_managed_node_takes_the_node_and_leaves_the_neighbours() {
        let with_rule = upsert_managed("", &plan(true)).expect("upsert the branch rule");
        let content = upsert_managed_node(
            &with_rule,
            &NodePlan {
                key: "group:docs".into(),
                name: "Docs".into(),
                parent: None,
            },
        )
        .expect("upsert the module");

        let (updated, removed) =
            remove_managed_node(&content, "group:docs").expect("the overlay parses");
        assert_eq!(removed, 1, "exactly the one module");

        let root: toml::Value = updated.parse().expect("still valid toml");
        let spaces = root.get("spaces").expect("the spaces table survives");
        let count = |name: &str| {
            spaces
                .get(name)
                .and_then(|entries| entries.as_array())
                .map_or(0, Vec::len)
        };
        assert_eq!(count("node"), 0, "the module is gone: {updated}");
        assert_eq!(count("split"), 1, "the branch rule is untouched: {updated}");
        assert_eq!(count("project"), 1, "the project is untouched: {updated}");
    }

    // TP-MOD-29: a key nobody wrote changes nothing, byte for byte. A silent
    // rewrite of the overlay would churn a file the user also edits by hand.
    #[test]
    fn remove_managed_node_without_a_match_changes_nothing() {
        let content = upsert_managed_node(
            "",
            &NodePlan {
                key: "group:docs".into(),
                name: "Docs".into(),
                parent: None,
            },
        )
        .expect("upsert");
        let (updated, removed) = remove_managed_node(&content, "group:yok").expect("parses");
        assert_eq!(removed, 0);
        assert_eq!(updated, content);
    }

    // TP-MOD-27: a project is written as `[[spaces.project]]` and reaches the
    // tree as a parentless node, so it looks like a module on screen. It must
    // not answer to the module verb: deleting a project takes its member rules
    // with it, which is a different and much larger act than dropping an empty
    // container.
    #[test]
    fn managed_has_node_does_not_claim_a_project() {
        let content = upsert_managed("", &plan(true)).expect("upsert a project-bearing rule");
        assert!(
            content.contains("project"),
            "the fixture really writes a project: {content}"
        );
        assert!(!managed_has_node(&content, "project:worktree-tiling"));
    }

    // TP-MOD-26: the menu asks this before offering to delete. A module the
    // person wrote into config.toml by hand is not in the overlay, and the
    // machine cannot take it back — offering the verb anyway would be a button
    // that does nothing, which is the promise #64 was about.
    #[test]
    fn managed_has_node_answers_only_for_the_overlay() {
        let content = upsert_managed_node(
            "",
            &NodePlan {
                key: "group:docs".into(),
                name: "Docs".into(),
                parent: None,
            },
        )
        .expect("upsert");
        assert!(managed_has_node(&content, "group:docs"));
        assert!(!managed_has_node(&content, "group:el-yazmasi"));
        assert!(
            !managed_has_node("", "group:docs"),
            "an empty overlay holds nothing"
        );
        // A split rule with the same key is a bucket, not a module: the node
        // road must not claim it.
        let with_rule = upsert_managed("", &plan(false)).expect("upsert");
        assert!(!managed_has_node(&with_rule, "herdr:worktree-tiling"));
    }

    // TP-DOTS-05: the module road writes one node entry and nothing else —
    // no split rule, no project — and a re-write with the same key updates
    // in place instead of stacking duplicates.
    #[test]
    fn upsert_managed_node_writes_only_the_node_entry() {
        let updated = upsert_managed_node(
            "",
            &NodePlan {
                key: "group:docs".into(),
                name: "Docs".into(),
                parent: Some("project:herdr".into()),
            },
        )
        .expect("a clean overlay accepts the node");
        let root: toml::Value = updated.parse().expect("the overlay stays valid toml");
        let spaces = root.get("spaces").expect("a spaces table is written");
        let nodes = spaces
            .get("node")
            .and_then(|n| n.as_array())
            .expect("a node array is written");
        assert_eq!(nodes.len(), 1);
        assert_eq!(
            nodes[0].get("key").and_then(|v| v.as_str()),
            Some("group:docs")
        );
        assert_eq!(nodes[0].get("name").and_then(|v| v.as_str()), Some("Docs"));
        assert_eq!(
            nodes[0].get("parent").and_then(|v| v.as_str()),
            Some("project:herdr")
        );
        assert!(
            spaces.get("split").is_none(),
            "no split rule rides along with a branchless module"
        );

        let renamed = upsert_managed_node(
            &updated,
            &NodePlan {
                key: "group:docs".into(),
                name: "Docs Render".into(),
                parent: None,
            },
        )
        .expect("a re-write with the same key is an update");
        let root: toml::Value = renamed.parse().expect("still valid toml");
        let nodes = root
            .get("spaces")
            .and_then(|s| s.get("node"))
            .and_then(|n| n.as_array())
            .expect("the node array survives");
        assert_eq!(nodes.len(), 1, "the same key updates in place");
        assert_eq!(
            nodes[0].get("name").and_then(|v| v.as_str()),
            Some("Docs Render")
        );
        assert!(
            nodes[0].get("parent").is_none(),
            "top level drops the parent field"
        );
    }

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
            parent: None,
            node: None,
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

    // TP-RANK-08: a move writes the parent onto the managed rule; a plan
    // without one keeps the field out of the file entirely, so yesterday's
    // promotes stay byte-compatible.
    #[test]
    fn upsert_writes_the_parent_the_plan_carries() {
        let mut moved = plan(false);
        moved.parent = Some("group:ops".into());
        let updated = upsert_managed("", &moved).expect("upsert");
        let value: toml::Value = updated.parse().expect("managed file parses");
        let split = value["spaces"]["split"].as_array().expect("split array");
        assert_eq!(split[0]["parent"].as_str(), Some("group:ops"));

        let plain = upsert_managed("", &plan(false)).expect("upsert");
        let value: toml::Value = plain.parse().expect("managed file parses");
        let split = value["spaces"]["split"].as_array().expect("split array");
        assert!(
            split[0].get("parent").is_none(),
            "no parent on the plan, no parent key in the file"
        );
    }

    // TP-RANK-08: moving the same checkout again re-hangs it in place —
    // one rule, the new parent — and moving to top level drops the field.
    #[test]
    fn moving_again_rehangs_the_same_rule() {
        let mut first = plan(false);
        first.parent = Some("group:ops".into());
        let content = upsert_managed("", &first).expect("first move");

        let mut second = plan(false);
        second.parent = Some("group:infra".into());
        let content = upsert_managed(&content, &second).expect("second move");
        let value: toml::Value = content.parse().expect("managed file parses");
        let split = value["spaces"]["split"].as_array().expect("split array");
        assert_eq!(split.len(), 1, "no duplicate rule");
        assert_eq!(split[0]["parent"].as_str(), Some("group:infra"));

        let back_to_top = upsert_managed(&content, &plan(false)).expect("top-level move");
        let value: toml::Value = back_to_top.parse().expect("managed file parses");
        let split = value["spaces"]["split"].as_array().expect("split array");
        assert_eq!(split.len(), 1);
        assert!(
            split[0].get("parent").is_none(),
            "a top-level move removes the parent"
        );
    }

    // TP-RANK-11: exactly one destination, spelled one of five ways.
    #[test]
    fn move_args_parse_the_five_destinations() {
        let args = |list: &[&str]| -> Vec<String> { list.iter().map(|s| s.to_string()).collect() };

        assert_eq!(
            parse_move_args(&args(&["feat/x", "--under", "group:ops"])),
            Ok(MoveArgs {
                branch: "feat/x".into(),
                dest: MoveDest::Node {
                    key: "group:ops".into(),
                    op: crate::spaces::MoveOp::Under,
                },
                repo: None,
                dry_run: false,
            })
        );
        assert_eq!(
            parse_move_args(&args(&["feat/x", "--beside", "group:ops"]))
                .expect("beside parses")
                .dest,
            MoveDest::Node {
                key: "group:ops".into(),
                op: crate::spaces::MoveOp::Beside,
            }
        );
        assert_eq!(
            parse_move_args(&args(&["feat/x", "--above", "group:ops"]))
                .expect("above parses")
                .dest,
            MoveDest::Node {
                key: "group:ops".into(),
                op: crate::spaces::MoveOp::Above,
            }
        );
        assert_eq!(
            parse_move_args(&args(&["feat/x", "--top"]))
                .expect("top parses")
                .dest,
            MoveDest::Top
        );
        let parsed = parse_move_args(&args(&[
            "feat/x",
            "--new-group",
            "Ops",
            "--repo",
            "/repo/herdr",
            "--dry-run",
        ]))
        .expect("new-group parses");
        assert_eq!(parsed.dest, MoveDest::NewGroup { name: "Ops".into() });
        assert_eq!(parsed.repo.as_deref(), Some("/repo/herdr"));
        assert!(parsed.dry_run);
    }

    // TP-RANK-11: none or two destinations, a missing value, an unknown flag
    // or a missing branch are refused — a move is never guessed at.
    #[test]
    fn move_args_refuse_ambiguous_or_incomplete_invocations() {
        let args = |list: &[&str]| -> Vec<String> { list.iter().map(|s| s.to_string()).collect() };

        assert!(
            parse_move_args(&args(&["feat/x"])).is_err(),
            "no destination"
        );
        assert!(
            parse_move_args(&args(&["feat/x", "--under", "a", "--top"])).is_err(),
            "two destinations"
        );
        assert!(
            parse_move_args(&args(&["feat/x", "--under"])).is_err(),
            "missing value"
        );
        assert!(
            parse_move_args(&args(&["--top"])).is_err(),
            "missing branch"
        );
        assert!(
            parse_move_args(&args(&["feat/x", "--sideways", "a"])).is_err(),
            "unknown flag"
        );
    }

    // TP-RANK-11: a move is a re-hang, not a re-style — the label and icon a
    // promote once wrote survive the rule being re-written.
    #[test]
    fn a_move_keeps_the_promoted_label_and_icon() {
        let content = upsert_managed("", &plan(false)).expect("promote");
        assert_eq!(
            existing_rule_style(&content, "herdr:worktree-tiling"),
            Some(("Tiling arastirmasi".to_string(), Some("🧱".to_string())))
        );
        assert_eq!(
            existing_rule_style(&content, "no-such-key"),
            None,
            "an unknown key carries no style"
        );
        assert_eq!(existing_rule_style("", "any"), None, "an empty overlay too");
    }

    // TP-RANK-10: "move under a new group" lands the group and the
    // membership in one write — a crash between the two cannot leave a
    // parent pointing at a group that was never written.
    #[test]
    fn a_new_group_lands_with_its_member_in_one_write() {
        let mut moved = plan(false);
        moved.parent = Some("group:ops".into());
        moved.node = Some(NodePlan {
            key: "group:ops".into(),
            name: "Ops".into(),
            parent: None,
        });
        let updated = upsert_managed("", &moved).expect("upsert");
        let value: toml::Value = updated.parse().expect("managed file parses");

        let nodes = value["spaces"]["node"].as_array().expect("node array");
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0]["key"].as_str(), Some("group:ops"));
        assert_eq!(nodes[0]["name"].as_str(), Some("Ops"));
        assert!(
            nodes[0].get("parent").is_none(),
            "a new group is born top level"
        );

        let split = value["spaces"]["split"].as_array().expect("split array");
        assert_eq!(split[0]["parent"].as_str(), Some("group:ops"));

        // Writing the same group again updates in place (TP-RANK-03's
        // contract extended to nodes).
        let again = upsert_managed(&updated, &moved).expect("second upsert");
        let value: toml::Value = again.parse().expect("managed file parses");
        assert_eq!(
            value["spaces"]["node"]
                .as_array()
                .expect("node array")
                .len(),
            1,
            "no duplicate node entry"
        );
    }
}
