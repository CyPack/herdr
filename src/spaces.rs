//! Config-driven space partitioning.
//!
//! Stock grouping puts every checkout of a repository into one space, keyed by
//! the repository's git directory. That is the right default: a space is "the
//! repo I am working in". It stops being right once a single repository holds
//! several independent product modules — a panel serving four tenants, say —
//! because then "which repo" is no longer the question the sidebar is being
//! asked. The question is "which module".
//!
//! These rules let the config answer that: a rule claims some of a repository's
//! checkouts by branch or directory name and gives them their own space key and
//! label. Everything a rule does not claim keeps the stock repo space, so the
//! feature is additive — an empty rule list reproduces upstream behaviour
//! exactly.
//!
//! Matching is deliberately a one-wildcard glob rather than a regex: these
//! patterns are written by hand in a config file and read by whoever is
//! debugging their sidebar six months later. `feat/t4f-*` should mean what it
//! looks like it means.

use std::path::{Path, PathBuf};

// TP-SPLIT-MATCH-01/02/03/04/05.
/// One `[[spaces.split]]` rule after validation: the repository it applies to,
/// the patterns claiming checkouts inside it, and the space those checkouts
/// move to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpaceSplitRule {
    /// Repository root the rule applies to, `~` already expanded.
    pub repo_root: PathBuf,
    /// Globs matched against the checkout's branch, then its directory name.
    pub patterns: Vec<String>,
    /// Space key the matched checkouts group under. Must be unique per module.
    pub key: String,
    /// Header label rendered for the group.
    pub label: String,
    /// Glyph drawn before the label on the group's header row. `None` renders
    /// the label alone — existing configs carry emoji inside their labels and
    /// keep rendering unchanged.
    pub icon: Option<String>,
    /// Node key this bucket hangs under. `None` keeps today's placement:
    /// a claiming project when one exists, top level otherwise.
    pub parent: Option<String>,
}

impl SpaceSplitRule {
    /// Whether this rule claims `checkout_path` (on `branch`) of `repo_root`.
    ///
    /// The branch is tried first because it is what the user thinks in; the
    /// directory name is the fallback for detached checkouts, which have no
    /// branch to match on (TP-SPLIT-MATCH-02, TP-SPLIT-MATCH-03).
    pub fn claims(&self, repo_root: &Path, checkout_path: &Path, branch: Option<&str>) -> bool {
        if self.repo_root != repo_root {
            return false;
        }
        let dir_name = checkout_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        self.patterns.iter().any(|pattern| {
            branch.is_some_and(|branch| matches_glob(pattern, branch))
                || matches_glob(pattern, dir_name)
        })
    }
}

/// The first rule claiming this checkout, or `None` to keep the repo space.
///
/// First-match-wins rather than most-specific-wins: config order is visible and
/// editable, "specificity" would have to be guessed (TP-SPLIT-MATCH-04). An
/// empty rule list claims nothing (TP-SPLIT-MATCH-01).
pub fn resolve_space_rule<'a>(
    rules: &'a [SpaceSplitRule],
    repo_root: &Path,
    checkout_path: &Path,
    branch: Option<&str>,
) -> Option<&'a SpaceSplitRule> {
    rules
        .iter()
        .find(|rule| rule.claims(repo_root, checkout_path, branch))
}

// TP-PROJ-MATCH-01/02/03.
/// One `[[spaces.project]]` umbrella after validation: a top-level sidebar
/// group gathering whole repositories and individual spaces under one header,
/// so several repositories serving one product read as one project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpaceProject {
    /// Key project collapse state is stored under. Must be unique per project.
    pub key: String,
    /// Header title.
    pub name: String,
    /// Glyph drawn before the name; `None` uses the configured default.
    pub icon: Option<String>,
    /// Repository roots whose every space — the repo space and any config
    /// spaces split out of it — belongs to this project.
    pub repo_roots: Vec<PathBuf>,
    /// Individual space keys claimed regardless of repository. This is the
    /// promotion path: one branch's space joins (or becomes) a project without
    /// pulling its whole repository along.
    pub space_keys: Vec<String>,
}

impl SpaceProject {
    /// Whether this project claims the space `space_key`, whose members live
    /// in `repo_root`. Explicit space keys are the sharper claim and are tried
    /// first; repository membership is the broad one.
    pub fn claims(&self, space_key: &str, repo_root: Option<&Path>) -> bool {
        self.space_keys.iter().any(|key| key == space_key)
            || repo_root.is_some_and(|root| self.repo_roots.iter().any(|repo| repo == root))
    }
}

/// The first project claiming this space, or `None` to stay top-level.
///
/// First-match-wins in config order, exactly like [`resolve_space_rule`]:
/// which project a space lands in stays readable from the config file
/// (TP-PROJ-MATCH-03). An empty project list claims nothing, so a sidebar
/// without `[[spaces.project]]` renders as it always has (TP-PROJ-MATCH-01).
pub fn resolve_project<'a>(
    projects: &'a [SpaceProject],
    space_key: &str,
    repo_root: Option<&Path>,
) -> Option<&'a SpaceProject> {
    projects
        .iter()
        .find(|project| project.claims(space_key, repo_root))
}

/// One `[[spaces.node]]` container after validation: a named tree node that
/// hangs under another node by `parent`, giving the sidebar its N-level shape.
///
/// A node carries no membership of its own — children point up at it, the
/// way `parent` on a rule or another node does. That single direction is what
/// keeps "move this under X" a one-line change wherever it is written
/// (TP-NODE-*).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpaceNode {
    /// Tree identity and collapse-state key. Unique across the forest.
    pub key: String,
    /// Header title.
    pub name: String,
    /// Glyph drawn before the name; `None` uses the configured default.
    pub icon: Option<String>,
    /// The node this one hangs under, or `None` for top level.
    pub parent: Option<String>,
}

/// One edge reader for the whole tree the sidebar walks: a node's configured
/// parent, or — when the key belongs to a split rule (a module bucket) —
/// that rule's own parent. Modules and buckets are one forest to the person
/// using them (TP-NODE-08); every chain walker reads its edges here so none
/// of them can stop halfway up a mixed chain.
pub fn tree_parent_of<'a>(
    nodes: &'a [SpaceNode],
    rules: &'a [SpaceSplitRule],
    key: &str,
) -> Option<&'a str> {
    if let Some(node) = nodes.iter().find(|node| node.key == key) {
        return node.parent.as_deref();
    }
    rules
        .iter()
        .find(|rule| rule.key == key)
        .and_then(|rule| rule.parent.as_deref())
}

/// The bucket-side edges of the tree, in the shape the forest validator
/// wants them: split-rule key → that rule's own parent.
pub fn split_parent_map(
    rules: &[SpaceSplitRule],
) -> std::collections::HashMap<String, Option<String>> {
    rules
        .iter()
        .map(|rule| (rule.key.clone(), rule.parent.clone()))
        .collect()
}

/// The forest, made safe to walk: duplicates dropped, unknown parents and
/// cycles cut loose, excessive depth flagged — all loudly, none fatally.
///
/// TP-NODE-03: config problems in the tree shape degrade to a flatter tree
/// plus a diagnostic, never to a failed load and never to a hung walker.
/// `split_parents` names the split-rule keys (with their own parents): a
/// node may hang under a bucket (TP-NODE-08), and a cycle that runs through
/// a bucket is still a cycle (TP-NODE-09).
pub fn validate_node_forest(
    nodes: Vec<SpaceNode>,
    split_parents: &std::collections::HashMap<String, Option<String>>,
) -> (Vec<SpaceNode>, Vec<String>) {
    let mut diagnostics = Vec::new();

    // A duplicate key would make two rows share one identity and one fold.
    let mut seen = std::collections::HashSet::new();
    let mut forest: Vec<SpaceNode> = Vec::with_capacity(nodes.len());
    for node in nodes {
        if !seen.insert(node.key.clone()) {
            diagnostics.push(format!(
                "spaces node {:?} is already defined; keeping the first definition",
                node.key
            ));
            continue;
        }
        forest.push(node);
    }

    // A parent nobody defines is a typo, not a tree. A bucket (split-rule
    // key) is defined too: modules hang under buckets (TP-NODE-08).
    let keys: std::collections::HashSet<String> =
        forest.iter().map(|node| node.key.clone()).collect();
    for node in &mut forest {
        if let Some(parent) = node.parent.as_deref() {
            if !keys.contains(parent) && !split_parents.contains_key(parent) {
                diagnostics.push(format!(
                    "spaces node {:?} names an unknown parent {parent:?}; \
                     keeping it at top level",
                    node.key
                ));
                node.parent = None;
            }
        }
    }

    // A cycle can never take the walker down: every member drops to top
    // level. Walked per node with a seen-set; the forest is config-sized.
    // The walk crosses bucket edges too — a loop that runs through a split
    // rule is still a loop (TP-NODE-09); node entries override on key clash.
    let mut parent_of: std::collections::HashMap<String, Option<String>> = split_parents.clone();
    parent_of.extend(
        forest
            .iter()
            .map(|node| (node.key.clone(), node.parent.clone())),
    );
    let mut in_cycle = std::collections::HashSet::new();
    for node in &forest {
        let mut walked = std::collections::HashSet::new();
        let mut current = Some(node.key.clone());
        while let Some(key) = current {
            if !walked.insert(key.clone()) {
                // The walk came back around: everything walked from here on
                // is in or above the cycle — mark only the true cycle by
                // walking once more from the repeated key.
                let mut member = key.clone();
                loop {
                    if !in_cycle.insert(member.clone()) {
                        break;
                    }
                    match parent_of.get(&member).and_then(|p| p.clone()) {
                        Some(next) => member = next,
                        None => break,
                    }
                }
                break;
            }
            current = parent_of.get(&key).and_then(|p| p.clone());
        }
    }
    if !in_cycle.is_empty() {
        let mut members: Vec<&str> = in_cycle.iter().map(String::as_str).collect();
        members.sort_unstable();
        diagnostics.push(format!(
            "spaces nodes {} form a parent cycle; all of them move to top level",
            members.join(", ")
        ));
        for node in &mut forest {
            if in_cycle.contains(&node.key) {
                node.parent = None;
            }
        }
    }

    // Depth eight is a warning, not a wall (K6): the chain keeps rendering,
    // the author is told the tree has outgrown what a sidebar can indent.
    // Mixed chains count bucket steps too.
    let mut parent_of: std::collections::HashMap<String, Option<String>> = split_parents.clone();
    parent_of.extend(
        forest
            .iter()
            .map(|node| (node.key.clone(), node.parent.clone())),
    );
    let guard = forest.len() + split_parents.len();
    let mut deepest = 0usize;
    for node in &forest {
        let mut depth = 0usize;
        let mut current = parent_of.get(&node.key).and_then(|p| p.clone());
        while let Some(key) = current {
            depth += 1;
            if depth > guard {
                break; // Unreachable after the cycle pass; belt and braces.
            }
            current = parent_of.get(&key).and_then(|p| p.clone());
        }
        deepest = deepest.max(depth + 1);
    }
    if deepest > 8 {
        diagnostics.push(format!(
            "spaces nodes reach depth {deepest}; beyond 8 the sidebar's \
             indentation stops growing"
        ));
    }

    (forest, diagnostics)
}

/// The node key a space hangs under, if any.
///
/// The rule's own `parent` is the sharper claim and wins; a project claiming
/// the space is the broad one (projects are parentless nodes, so this is how
/// yesterday's two-level grouping becomes a plain tree edge); neither means
/// top level.
pub fn resolve_space_parent(
    rule: Option<&SpaceSplitRule>,
    projects: &[SpaceProject],
    space_key: &str,
    repo_root: Option<&Path>,
) -> Option<String> {
    rule.and_then(|rule| rule.parent.clone()).or_else(|| {
        resolve_project(projects, space_key, repo_root).map(|project| project.key.clone())
    })
}

/// The three ways a checkout can be re-hung relative to a node (K5): as its
/// child, as its sibling, or one level above it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveOp {
    Under,
    Beside,
    Above,
}

/// The parent a move lands on, resolved against the validated forest.
///
/// `Ok(None)` means top level. An unknown target is an error, never a silent
/// drop to top level: a typo in a CLI key must not quietly flatten the tree.
pub fn move_parent_for(
    nodes: &[SpaceNode],
    target_key: &str,
    op: MoveOp,
) -> Result<Option<String>, String> {
    let target = nodes
        .iter()
        .find(|node| node.key == target_key)
        .ok_or_else(|| format!("no spaces node has the key {target_key:?}"))?;
    let parent_of = |key: &str| -> Option<String> {
        nodes
            .iter()
            .find(|node| node.key == key)
            .and_then(|node| node.parent.clone())
    };
    Ok(match op {
        MoveOp::Under => Some(target.key.clone()),
        MoveOp::Beside => target.parent.clone(),
        MoveOp::Above => target.parent.as_deref().and_then(parent_of),
    })
}

/// Glob match supporting `*` (any run of characters, including empty). No `?`,
/// no character classes, no escaping: every other character is literal
/// (TP-SPLIT-MATCH-05).
pub fn matches_glob(pattern: &str, value: &str) -> bool {
    let mut segments = pattern.split('*');
    let Some(first) = segments.next() else {
        return pattern == value;
    };
    if !value.starts_with(first) {
        return false;
    }
    // No wildcard at all: the pattern must be the whole value, not a prefix.
    let Some(mut rest) = value.get(first.len()..) else {
        return false;
    };
    if !pattern.contains('*') {
        return rest.is_empty();
    }

    let segments: Vec<&str> = segments.collect();
    let last_index = segments.len().saturating_sub(1);
    for (index, segment) in segments.iter().enumerate() {
        if segment.is_empty() {
            // A trailing `*` accepts whatever is left.
            if index == last_index {
                return true;
            }
            continue;
        }
        if index == last_index {
            return rest.ends_with(segment);
        }
        let Some(found) = rest.find(segment) else {
            return false;
        };
        rest = &rest[found + segment.len()..];
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(repo: &str, patterns: &[&str], key: &str, label: &str) -> SpaceSplitRule {
        SpaceSplitRule {
            repo_root: PathBuf::from(repo),
            patterns: patterns.iter().map(|p| (*p).to_string()).collect(),
            key: key.to_string(),
            label: label.to_string(),
            icon: None,
            parent: None,
        }
    }

    // TP-MOD-30: deleting a module must not delete what was inside it. The
    // children name a parent nobody defines any more, and the forest re-seats
    // them at top level and says so — a silent disappearance would read as
    // data loss to the person who only meant to rename a container.
    #[test]
    fn a_module_whose_parent_was_deleted_is_re_seated_at_top_level() {
        let node = |key: &str, parent: Option<&str>| SpaceNode {
            key: key.into(),
            name: key.into(),
            icon: None,
            parent: parent.map(str::to_string),
        };
        // "group:gone" is not in the list: this is the tree one moment after
        // the delete wrote the overlay back.
        let (forest, diagnostics) = validate_node_forest(
            vec![
                node("group:child", Some("group:gone")),
                node("group:grandchild", Some("group:child")),
            ],
            &std::collections::HashMap::new(),
        );
        assert_eq!(forest.len(), 2, "nothing is dropped: {forest:?}");
        let child = forest
            .iter()
            .find(|n| n.key == "group:child")
            .expect("the orphan survives");
        assert_eq!(child.parent, None, "it is re-seated at top level");
        let grandchild = forest
            .iter()
            .find(|n| n.key == "group:grandchild")
            .expect("its own child survives");
        assert_eq!(
            grandchild.parent.as_deref(),
            Some("group:child"),
            "a chain below the orphan is left intact"
        );
        assert!(
            diagnostics.iter().any(|d| d.contains("group:gone")),
            "the re-seating is stated, not silent: {diagnostics:?}"
        );
    }

    fn project(key: &str, repos: &[&str], spaces: &[&str]) -> SpaceProject {
        SpaceProject {
            key: key.to_string(),
            name: key.to_string(),
            icon: None,
            repo_roots: repos.iter().map(PathBuf::from).collect(),
            space_keys: spaces.iter().map(|s| (*s).to_string()).collect(),
        }
    }

    #[test]
    fn glob_without_wildcard_requires_exact_value() {
        assert!(matches_glob("main", "main"));
        assert!(!matches_glob("main", "main-2"));
        assert!(!matches_glob("main", "ma"));
    }

    #[test]
    fn glob_matches_prefix_suffix_and_interior_wildcards() {
        assert!(matches_glob("feat/t4f-*", "feat/t4f-reactive-refresh"));
        assert!(matches_glob(
            "*bamcheck*",
            "feat/bamcheck-multiprofile-layout"
        ));
        assert!(matches_glob("*-qc", "feat/has-qc"));
        assert!(matches_glob("feat/*-window", "feat/voorspan-window"));
        assert!(!matches_glob("feat/t4f-*", "fix/t4f-opmerking"));
    }

    #[test]
    fn glob_star_matches_everything_including_empty() {
        assert!(matches_glob("*", ""));
        assert!(matches_glob("*", "anything"));
        assert!(matches_glob("feat/*", "feat/"));
    }

    #[test]
    fn rule_ignores_other_repositories() {
        let rules = [rule("/repo/a", &["feat/*"], "a:feat", "A")];
        assert!(
            resolve_space_rule(
                &rules,
                Path::new("/repo/b"),
                Path::new("/repo/b-feature"),
                Some("feat/x")
            )
            .is_none(),
            "a rule must not claim a checkout of a different repository"
        );
    }

    #[test]
    fn rule_claims_by_branch() {
        let rules = [rule("/repo/a", &["feat/t4f-*"], "a:t4f", "T4F")];
        let hit = resolve_space_rule(
            &rules,
            Path::new("/repo/a"),
            Path::new("/repo/a-source-status"),
            Some("feat/t4f-reactive-refresh"),
        );
        assert_eq!(hit.map(|rule| rule.key.as_str()), Some("a:t4f"));
        assert_eq!(hit.map(|rule| rule.label.as_str()), Some("T4F"));
    }

    #[test]
    fn rule_falls_back_to_directory_name_for_detached_checkouts() {
        let rules = [rule("/repo/a", &["*circet*"], "a:circet", "Circet")];
        let hit = resolve_space_rule(
            &rules,
            Path::new("/repo/a"),
            Path::new("/repo/a-circet-data-platform"),
            None,
        );
        assert_eq!(hit.map(|rule| rule.key.as_str()), Some("a:circet"));
    }

    #[test]
    fn first_matching_rule_wins() {
        let rules = [
            rule("/repo/a", &["*has-*"], "a:circet", "Circet"),
            rule("/repo/a", &["feat/*"], "a:feat", "Feat"),
        ];
        let hit = resolve_space_rule(
            &rules,
            Path::new("/repo/a"),
            Path::new("/repo/a-has-qc"),
            Some("feat/has-qc"),
        );
        assert_eq!(
            hit.map(|rule| rule.key.as_str()),
            Some("a:circet"),
            "config order decides, not pattern specificity"
        );
    }

    #[test]
    fn unclaimed_checkout_keeps_the_repo_space() {
        let rules = [rule("/repo/a", &["feat/t4f-*"], "a:t4f", "T4F")];
        assert!(
            resolve_space_rule(
                &rules,
                Path::new("/repo/a"),
                Path::new("/repo/a"),
                Some("main")
            )
            .is_none(),
            "an unmatched checkout must fall through to the stock repo space"
        );
    }

    #[test]
    fn empty_rule_list_never_claims_anything() {
        assert!(
            resolve_space_rule(
                &[],
                Path::new("/repo/a"),
                Path::new("/repo/a"),
                Some("main")
            )
            .is_none(),
            "no rules configured must reproduce upstream grouping exactly"
        );
    }

    // TP-PROJ-MATCH-01.
    #[test]
    fn empty_project_list_claims_nothing() {
        assert!(
            resolve_project(&[], "repo-key", Some(Path::new("/repo/a"))).is_none(),
            "no projects configured must leave every space top-level"
        );
    }

    // TP-PROJ-MATCH-02.
    #[test]
    fn project_claims_by_space_key() {
        let projects = [project("project:x", &[], &["a:t4f"])];
        let hit = resolve_project(&projects, "a:t4f", None);
        assert_eq!(hit.map(|p| p.key.as_str()), Some("project:x"));
        assert!(
            resolve_project(&projects, "a:other", None).is_none(),
            "a project must not claim spaces it never names"
        );
    }

    // TP-PROJ-MATCH-02.
    #[test]
    fn project_claims_by_repo_root() {
        let projects = [project("project:x", &["/repo/a"], &[])];
        let hit = resolve_project(&projects, "any-space", Some(Path::new("/repo/a")));
        assert_eq!(hit.map(|p| p.key.as_str()), Some("project:x"));
        assert!(
            resolve_project(&projects, "any-space", Some(Path::new("/repo/b"))).is_none(),
            "repository membership must not leak across repositories"
        );
    }

    // TP-PROJ-MATCH-03.
    #[test]
    fn first_matching_project_wins() {
        let projects = [
            project("project:first", &["/repo/a"], &[]),
            project("project:second", &[], &["a:t4f"]),
        ];
        let hit = resolve_project(&projects, "a:t4f", Some(Path::new("/repo/a")));
        assert_eq!(
            hit.map(|p| p.key.as_str()),
            Some("project:first"),
            "config order decides, not claim kind"
        );
    }

    fn node(key: &str, parent: Option<&str>) -> SpaceNode {
        SpaceNode {
            key: key.to_string(),
            name: key.to_string(),
            icon: None,
            parent: parent.map(str::to_string),
        }
    }

    // TP-NODE-03: a parent cycle can never take the load down. The nodes in
    // the cycle drop to top level, the config author is told, and everything
    // outside the cycle keeps its place.
    #[test]
    fn a_cycle_is_reported_and_its_nodes_go_top_level() {
        let (forest, diagnostics) = validate_node_forest(
            vec![
                node("a", Some("b")),
                node("b", Some("a")),
                node("c", Some("a")),
            ],
            &Default::default(),
        );

        let parent_of = |key: &str| {
            forest
                .iter()
                .find(|n| n.key == key)
                .and_then(|n| n.parent.clone())
        };
        assert_eq!(parent_of("a"), None, "a cycle member goes top level");
        assert_eq!(parent_of("b"), None, "both cycle members go top level");
        assert_eq!(
            parent_of("c"),
            Some("a".to_string()),
            "a child hanging off the cycle keeps its now-valid parent"
        );
        assert!(
            diagnostics.iter().any(|d| d.contains("cycle")),
            "the author is told about the cycle: {diagnostics:?}"
        );
    }

    // A parent nobody defines is a typo, not a tree: report it, keep the node
    // at top level, and never drop the node itself.
    #[test]
    fn an_unknown_parent_is_reported_and_the_node_stays_top_level() {
        let (forest, diagnostics) = validate_node_forest(
            vec![node("a", Some("ghost")), node("b", None)],
            &Default::default(),
        );

        assert_eq!(forest.len(), 2, "no node is dropped over a bad parent");
        assert_eq!(
            forest
                .iter()
                .find(|n| n.key == "a")
                .and_then(|n| n.parent.clone()),
            None
        );
        assert!(
            diagnostics.iter().any(|d| d.contains("ghost")),
            "the missing key is named: {diagnostics:?}"
        );
    }

    // TP-NODE-08: a module bucket is a parent like any other — a node
    // hanging under a split rule's key is a valid tree, not a typo.
    #[test]
    fn a_node_may_hang_under_a_bucket() {
        let split_parents =
            std::collections::HashMap::from([("herdr:tiling".to_string(), None::<String>)]);
        let (forest, diagnostics) = validate_node_forest(
            vec![node("group:probe", Some("herdr:tiling"))],
            &split_parents,
        );

        assert!(
            diagnostics.is_empty(),
            "a bucket parent is defined, not a ghost: {diagnostics:?}"
        );
        assert_eq!(
            forest[0].parent.as_deref(),
            Some("herdr:tiling"),
            "the bucket parent survives validation"
        );
    }

    // TP-NODE-09: a cycle that runs through a bucket is still a cycle — the
    // walker can never be handed a loop just because one edge is a rule's.
    #[test]
    fn a_cycle_through_a_bucket_is_still_caught() {
        let split_parents =
            std::collections::HashMap::from([("bucket".to_string(), Some("group:a".to_string()))]);
        let (forest, diagnostics) =
            validate_node_forest(vec![node("group:a", Some("bucket"))], &split_parents);

        assert_eq!(
            forest[0].parent, None,
            "the node side of the loop drops to top level"
        );
        assert!(
            diagnostics.iter().any(|d| d.contains("cycle")),
            "the author is told about the cycle: {diagnostics:?}"
        );
    }

    // K6: depth eight is a warning, not a wall — the chain keeps rendering.
    #[test]
    fn depth_beyond_eight_warns_but_still_chains() {
        let mut nodes = vec![node("n0", None)];
        for i in 1..=9 {
            nodes.push(node(&format!("n{i}"), Some(&format!("n{}", i - 1))));
        }
        let (forest, diagnostics) = validate_node_forest(nodes, &Default::default());

        assert_eq!(
            forest
                .iter()
                .find(|n| n.key == "n9")
                .and_then(|n| n.parent.clone()),
            Some("n8".to_string()),
            "the deep chain is kept intact"
        );
        assert!(
            diagnostics.iter().any(|d| d.contains("depth")),
            "the author is warned about the depth: {diagnostics:?}"
        );
    }

    // A duplicate node key would make two rows share one identity and one
    // fold. The first definition wins, the second is dropped loudly.
    #[test]
    fn a_duplicate_node_key_keeps_the_first_and_drops_the_second() {
        let (forest, diagnostics) = validate_node_forest(
            vec![
                node("a", None),
                SpaceNode {
                    key: "a".to_string(),
                    name: "impostor".to_string(),
                    icon: None,
                    parent: None,
                },
            ],
            &Default::default(),
        );

        assert_eq!(forest.len(), 1);
        assert_eq!(forest[0].name, "a", "the first definition wins");
        assert!(diagnostics.iter().any(|d| d.contains("already")));
    }

    // The parent chain a space hangs under: the rule's own parent is the
    // sharper claim and wins; a project claiming the space is the broad one;
    // neither means top level.
    #[test]
    fn resolve_space_parent_prefers_the_rules_own_parent() {
        let mut with_parent = rule("/repo/a", &["feat/*"], "a:feat", "Feat");
        with_parent.parent = Some("node:ui".to_string());
        let projects = [project("project:x", &[], &["a:feat"])];

        assert_eq!(
            resolve_space_parent(Some(&with_parent), &projects, "a:feat", None),
            Some("node:ui".to_string()),
            "the rule's own parent outranks the project claim"
        );

        let without_parent = rule("/repo/a", &["feat/*"], "a:feat", "Feat");
        assert_eq!(
            resolve_space_parent(Some(&without_parent), &projects, "a:feat", None),
            Some("project:x".to_string()),
            "with no rule parent the claiming project answers"
        );
        assert_eq!(
            resolve_space_parent(None, &[], "a:feat", None),
            None,
            "nothing claims it: top level"
        );
    }

    // TP-RANK-09: the three move operations resolve against the forest —
    // under is the target itself, beside is the target's parent, above is
    // the grandparent — and the top of the tree answers with top level.
    #[test]
    fn move_ops_resolve_under_beside_and_above() {
        let forest = [
            node("root", None),
            node("mid", Some("root")),
            node("leaf", Some("mid")),
        ];

        assert_eq!(
            move_parent_for(&forest, "mid", MoveOp::Under),
            Ok(Some("mid".to_string())),
            "under X hangs the checkout on X itself"
        );
        assert_eq!(
            move_parent_for(&forest, "leaf", MoveOp::Beside),
            Ok(Some("mid".to_string())),
            "beside X shares X's parent"
        );
        assert_eq!(
            move_parent_for(&forest, "leaf", MoveOp::Above),
            Ok(Some("root".to_string())),
            "above X hangs on X's grandparent"
        );
    }

    // TP-RANK-09: at the top of the tree the answers degrade to top level,
    // never to an error — only a missing target is refused.
    #[test]
    fn move_ops_at_the_top_answer_top_level() {
        let forest = [node("root", None), node("mid", Some("root"))];

        assert_eq!(
            move_parent_for(&forest, "root", MoveOp::Beside),
            Ok(None),
            "beside a top-level node is top level"
        );
        assert_eq!(
            move_parent_for(&forest, "root", MoveOp::Above),
            Ok(None),
            "above a top-level node is still top level"
        );
        assert_eq!(
            move_parent_for(&forest, "mid", MoveOp::Above),
            Ok(None),
            "above a child of a top-level node is top level"
        );
    }

    // TP-RANK-09: a typo in the target key is an error, never a silent drop
    // to top level — a CLI mistake must not quietly flatten the tree.
    #[test]
    fn an_unknown_move_target_is_refused() {
        let forest = [node("root", None)];
        let err = move_parent_for(&forest, "no-such-node", MoveOp::Under)
            .expect_err("unknown target must refuse");
        assert!(err.contains("no-such-node"), "{err}");
    }
}
