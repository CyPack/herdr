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
// Consumed by the sidebar tree entries in the next commit of this branch; the
// allow dies there (bin crate: test-only callers count as dead).
#[allow(dead_code)]
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
    // Consumed by the sidebar tree entries in the next commit of this branch.
    #[allow(dead_code)]
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
// Consumed by the sidebar tree entries in the next commit of this branch.
#[allow(dead_code)]
pub fn resolve_project<'a>(
    projects: &'a [SpaceProject],
    space_key: &str,
    repo_root: Option<&Path>,
) -> Option<&'a SpaceProject> {
    projects
        .iter()
        .find(|project| project.claims(space_key, repo_root))
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
        }
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
}
