//! Per-cwd git branch tracking for the agent panel (BUG-2c).
//!
//! Every pane's terminal keeps the cwd it was spawned with; this module maps
//! those cwds to their current git branch so the agent panel can show which
//! branch a chat agent works on. Branches come from the HEAD file (a plain
//! read, no subprocess) and refresh on the shared scheduled-tasks cadence
//! behind an mtime fingerprint, so interaction paths never touch the disk and
//! unchanged repos are never re-read.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime};

pub(crate) const TAB_BRANCH_POLL_INTERVAL: Duration = Duration::from_secs(2);

/// Cached branch state for one terminal cwd.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct TabBranchEntry {
    /// Whether this cwd has been looked at at least once. Distinguishes a
    /// fresh entry from a resolved non-git (or reftable) cwd whose
    /// fingerprint is legitimately `None`.
    resolved: bool,
    /// HEAD-file mtime at the last read; `None` for non-git cwds and
    /// reftable repos (see `git_branch_fingerprint`).
    fingerprint: Option<SystemTime>,
    pub(crate) branch: Option<String>,
}

impl TabBranchEntry {
    /// Test seam: a resolved entry carrying `branch`, for seeding the cache
    /// from UI tests without touching the filesystem.
    #[cfg(test)]
    pub(crate) fn test_with_branch(branch: Option<&str>) -> Self {
        Self {
            resolved: true,
            fingerprint: None,
            branch: branch.map(str::to_string),
        }
    }
}

/// Refresh `cache` to cover exactly `cwds`: drop entries whose terminal is
/// gone, resolve new cwds, and re-read a branch only when its HEAD
/// fingerprint moved (including Some→None when a repo disappears). Returns
/// whether any visible branch changed.
pub(crate) fn refresh_tab_branch_cache(
    cache: &mut HashMap<PathBuf, TabBranchEntry>,
    cwds: &HashSet<PathBuf>,
) -> bool {
    cache.retain(|cwd, _| cwds.contains(cwd));
    let mut changed = false;
    for cwd in cwds {
        let entry = cache.entry(cwd.clone()).or_default();
        let fingerprint = crate::workspace::git_branch_fingerprint(cwd);
        if entry.resolved && fingerprint == entry.fingerprint {
            // Covers unchanged HEAD files and settled non-git/reftable cwds
            // (both fingerprints None) — nothing is read again.
            continue;
        }
        let branch = crate::workspace::git_branch(cwd);
        if branch != entry.branch {
            changed = true;
        }
        entry.resolved = true;
        entry.fingerprint = fingerprint;
        entry.branch = branch;
    }
    changed
}

/// The cwds whose branch the agent panel can actually show: terminals of
/// custom-named tabs (project chats). Auto-named tabs render no branch, so
/// polling their repos would be I/O for an invisible surface — widen this set
/// when another surface starts consuming the cache.
pub(crate) fn branch_watch_cwds(state: &super::state::AppState) -> HashSet<PathBuf> {
    state
        .workspaces
        .iter()
        .flat_map(|ws| ws.tabs.iter())
        .filter(|tab| tab.custom_name.is_some())
        .flat_map(|tab| tab.panes.values())
        .filter_map(|pane| state.terminals.get(&pane.attached_terminal_id))
        .map(|terminal| terminal.cwd.clone())
        .collect()
}

impl super::App {
    /// Scheduled-tasks poll (both loops): keep the tab branch cache fresh for
    /// every branch-rendering terminal cwd. Returns whether a rendered branch
    /// changed.
    pub(crate) fn refresh_tab_branches_if_due(&mut self, now: Instant) -> bool {
        if self.next_tab_branch_poll.is_some_and(|next| now < next) {
            return false;
        }
        self.next_tab_branch_poll = Some(now + TAB_BRANCH_POLL_INTERVAL);
        let cwds = branch_watch_cwds(&self.state);
        refresh_tab_branch_cache(&mut self.state.tab_branch_cache, &cwds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn temp_repo(name: &str, branch: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("herdr-tab-branch-{name}-{}", std::process::id()));
        std::fs::create_dir_all(root.join(".git")).expect("test repo dir should create");
        write_head(&root, branch);
        root
    }

    fn write_head(root: &Path, branch: &str) {
        std::fs::write(
            root.join(".git/HEAD"),
            format!("ref: refs/heads/{branch}\n"),
        )
        .expect("test HEAD should write");
    }

    fn cleanup(root: &Path) {
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn branch_cache_fills_on_first_refresh_and_reports_change() {
        let repo = temp_repo("fill", "main");
        let mut cache = HashMap::new();
        let cwds = HashSet::from([repo.clone()]);

        let changed = refresh_tab_branch_cache(&mut cache, &cwds);

        assert!(changed, "first fill is a visible change");
        assert_eq!(
            cache.get(&repo).and_then(|entry| entry.branch.as_deref()),
            Some("main")
        );
        cleanup(&repo);
    }

    #[test]
    fn branch_cache_is_quiet_while_head_is_unchanged() {
        let repo = temp_repo("quiet", "main");
        let mut cache = HashMap::new();
        let cwds = HashSet::from([repo.clone()]);
        refresh_tab_branch_cache(&mut cache, &cwds);

        let changed = refresh_tab_branch_cache(&mut cache, &cwds);

        assert!(!changed, "unchanged HEAD must not dirty the frame");
        cleanup(&repo);
    }

    #[test]
    fn branch_cache_follows_a_branch_switch() {
        let repo = temp_repo("switch", "main");
        let mut cache = HashMap::new();
        let cwds = HashSet::from([repo.clone()]);
        refresh_tab_branch_cache(&mut cache, &cwds);

        // A tiny pause guarantees a distinct HEAD mtime even on coarse
        // filesystem clocks.
        std::thread::sleep(Duration::from_millis(10));
        write_head(&repo, "feature/x");
        let changed = refresh_tab_branch_cache(&mut cache, &cwds);

        assert!(changed);
        assert_eq!(
            cache.get(&repo).and_then(|entry| entry.branch.as_deref()),
            Some("feature/x")
        );
        cleanup(&repo);
    }

    #[test]
    fn branch_cache_holds_none_for_non_git_cwds_without_crashing() {
        let dir =
            std::env::temp_dir().join(format!("herdr-tab-branch-plain-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("plain dir should create");
        let mut cache = HashMap::new();
        let cwds = HashSet::from([dir.clone()]);

        refresh_tab_branch_cache(&mut cache, &cwds);

        assert_eq!(
            cache.get(&dir).map(|entry| entry.branch.clone()),
            Some(None)
        );
        cleanup(&dir);
    }

    #[test]
    fn branch_watch_covers_only_custom_named_tabs() {
        let mut app = crate::app::state::AppState::test_new();
        let mut ws = crate::workspace::Workspace::test_new("space");
        let named_tab = ws.test_add_tab(Some("herdr"));
        let named_pane = ws.tabs[named_tab].root_pane;
        let auto_pane = ws.tabs[0].root_pane;
        app.workspaces = vec![ws];
        app.ensure_test_terminals();

        let cwd_of = |app: &crate::app::state::AppState, tab: usize, pane| {
            let id = app.workspaces[0].tabs[tab].panes[&pane]
                .attached_terminal_id
                .clone();
            app.terminals[&id].cwd.clone()
        };
        let set_cwd = |app: &mut crate::app::state::AppState, tab: usize, pane, cwd: &str| {
            let id = app.workspaces[0].tabs[tab].panes[&pane]
                .attached_terminal_id
                .clone();
            app.terminals.get_mut(&id).expect("terminal").cwd = PathBuf::from(cwd);
        };
        set_cwd(&mut app, named_tab, named_pane, "/proj/named");
        set_cwd(&mut app, 0, auto_pane, "/proj/auto");

        let watched = branch_watch_cwds(&app);

        assert!(watched.contains(&cwd_of(&app, named_tab, named_pane)));
        assert!(
            !watched.contains(Path::new("/proj/auto")),
            "auto-named tabs render no branch, so their repos must not be polled"
        );
    }

    #[test]
    fn branch_cache_drops_cwds_whose_terminals_are_gone() {
        let repo = temp_repo("drop", "main");
        let mut cache = HashMap::new();
        refresh_tab_branch_cache(&mut cache, &HashSet::from([repo.clone()]));
        assert!(cache.contains_key(&repo));

        refresh_tab_branch_cache(&mut cache, &HashSet::new());

        assert!(cache.is_empty(), "stale cwds must not leak in the cache");
        cleanup(&repo);
    }
}
