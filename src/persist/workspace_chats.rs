//! Which agent chats have run in which workspace — an append-only ledger.
//!
//! The question "which chat did I work with on this branch" cannot be answered
//! from the agent's own on-disk store: that store is keyed by the directory the
//! agent was launched in, and a branch checkout very often is not that
//! directory. Measured on 2026-07-30 against a live session: 9 of 14 workspaces
//! resolved to no chats at all that way, and 4 of the 9 sessions herdr was
//! actively wired to lived under a different directory than their workspace.
//!
//! The live wiring (`agent_session` on a pane) knows the truth, but only while
//! the pane exists — closing a tab erases it, and the question is asked in the
//! past tense. So the association is recorded here as it happens.
//!
//! Runtime fact, not presentation: names stay surface-neutral (CLAUDE.md
//! runtime/client boundary). The core is pure data with I/O only at the edges,
//! so every rule below is testable without touching a real config directory.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tracing::warn;

/// On-disk schema version. Bumped only for a breaking layout change; an
/// unknown version is treated as unreadable rather than guessed at.
pub const LEDGER_VERSION: u32 = 1;

/// Upper bound on remembered chats per workspace. A ledger that grows without
/// limit turns a convenience into a disk-space bug; the oldest observations are
/// the least useful, so they go first.
pub(crate) const MAX_CHATS_PER_WORKSPACE: usize = 200;

/// One observation of an agent chat running inside a workspace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatRecord {
    /// Session identity as the agent reports it (an id or a transcript path).
    pub session_id: String,
    /// Agent label, e.g. `claude`, `codex`.
    pub agent: String,
    /// Reporting source, e.g. `herdr:claude` — kept so a future consumer can
    /// tell hook-reported sessions from other origins.
    pub source: String,
    /// `id` or `path`, mirroring the live `agent_session` shape.
    pub kind: String,
    /// First time this session was seen in this workspace (unix millis).
    pub first_seen_ms: u64,
    /// Most recent time it was seen (unix millis).
    pub last_seen_ms: u64,
}

/// What the caller observed; the ledger supplies the timestamps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatObservation {
    pub session_id: String,
    pub agent: String,
    pub source: String,
    pub kind: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceChats {
    /// Newest observation first.
    pub chats: Vec<ChatRecord>,
}

/// The ledger itself. `BTreeMap` so the serialized file has a stable key order
/// and a save with no logical change produces a byte-identical file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceChatLedger {
    pub version: u32,
    #[serde(default)]
    pub workspaces: BTreeMap<String, WorkspaceChats>,
}

impl Default for WorkspaceChatLedger {
    fn default() -> Self {
        Self {
            version: LEDGER_VERSION,
            workspaces: BTreeMap::new(),
        }
    }
}

/// Canonical ledger key for a workspace directory.
///
/// Symlinked and non-normalized paths would otherwise split one workspace into
/// several entries, so the key goes through the same canonicalization the rest
/// of the codebase uses for worktree paths, falling back to the original when
/// the path does not exist (a workspace whose directory was removed keeps its
/// history).
pub fn ledger_key(identity_cwd: &Path) -> String {
    crate::worktree::canonical_or_original(identity_cwd)
        .to_string_lossy()
        .into_owned()
}

pub fn default_ledger_path() -> PathBuf {
    crate::config::config_dir().join("workspace-chats.json")
}

/// Wall clock in unix millis, saturating to 0 before the epoch.
///
/// The ledger's own functions all take an explicit timestamp so they stay
/// deterministic under test; this is the one place the real clock is read, and
/// the caller passes the result down.
pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as u64)
        .unwrap_or(0)
}

impl WorkspaceChatLedger {
    /// Record that `observation` was seen in `workspace_key` at `now_ms`.
    ///
    /// Returns whether the ledger changed, so a caller can skip a save. The
    /// same session seen again updates `last_seen_ms` and moves to the front
    /// but never duplicates and never rewrites `first_seen_ms` — "when did I
    /// start working with this chat here" must survive every later sighting.
    pub fn record_at(
        &mut self,
        workspace_key: &str,
        observation: ChatObservation,
        now_ms: u64,
    ) -> bool {
        if workspace_key.is_empty() || observation.session_id.is_empty() {
            return false;
        }
        let entry = self
            .workspaces
            .entry(workspace_key.to_string())
            .or_default();

        if let Some(position) = entry
            .chats
            .iter()
            .position(|chat| chat.session_id == observation.session_id)
        {
            let mut existing = entry.chats.remove(position);
            let unchanged = existing.last_seen_ms == now_ms
                && position == 0
                && existing.agent == observation.agent
                && existing.source == observation.source
                && existing.kind == observation.kind;
            existing.last_seen_ms = now_ms.max(existing.last_seen_ms);
            existing.agent = observation.agent;
            existing.source = observation.source;
            existing.kind = observation.kind;
            entry.chats.insert(0, existing);
            return !unchanged;
        }

        entry.chats.insert(
            0,
            ChatRecord {
                session_id: observation.session_id,
                agent: observation.agent,
                source: observation.source,
                kind: observation.kind,
                first_seen_ms: now_ms,
                last_seen_ms: now_ms,
            },
        );
        entry.chats.truncate(MAX_CHATS_PER_WORKSPACE);
        true
    }

    /// Chats remembered for a workspace, newest sighting first.
    pub fn chats_for(&self, workspace_key: &str) -> &[ChatRecord] {
        self.workspaces
            .get(workspace_key)
            .map(|entry| entry.chats.as_slice())
            .unwrap_or(&[])
    }
}

/// How long a known sighting stays fresh before it is worth re-recording.
///
/// The observer runs on every debounced session save, so without this the
/// `last_seen_ms` of every live chat would advance on each pass and the ledger
/// would be rewritten continuously. Policy lives here rather than inside
/// [`WorkspaceChatLedger::record_at`], which stays a faithful recorder.
pub(crate) const SIGHTING_FRESH_FOR_MS: u64 = 60_000;

/// Derive chat observations from a session snapshot.
///
/// Reading the SNAPSHOT rather than the live state is deliberate: the snapshot
/// already resolved which session a pane belongs to (hook authority first, then
/// the persisted session). Re-deriving that precedence here would let the
/// ledger and the session file disagree about the same pane — the snapshot is
/// the single answer, so the ledger asks it instead of guessing again.
pub fn observe_from_snapshot(snapshot: &super::SessionSnapshot) -> Vec<(String, ChatObservation)> {
    let mut observations = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for workspace in &snapshot.workspaces {
        let key = ledger_key(&workspace.identity_cwd);
        if key.is_empty() {
            continue;
        }
        for tab in &workspace.tabs {
            for pane in tab.panes.values() {
                let Some(session) = pane.agent_session.as_ref() else {
                    continue;
                };
                if session.value.is_empty() {
                    continue;
                }
                // One workspace can host the same chat in several panes (a
                // split showing the same agent); it is one association.
                if !seen.insert((key.clone(), session.value.clone())) {
                    continue;
                }
                observations.push((
                    key.clone(),
                    ChatObservation {
                        session_id: session.value.clone(),
                        agent: session.agent.clone(),
                        source: session.source.clone(),
                        kind: match session.kind {
                            crate::agent_resume::AgentSessionRefKind::Id => "id".to_string(),
                            crate::agent_resume::AgentSessionRefKind::Path => "path".to_string(),
                        },
                    },
                ));
            }
        }
    }
    observations
}

impl WorkspaceChatLedger {
    /// Whether an observation is worth recording right now.
    ///
    /// An unknown session always is. A known one only once it has gone stale,
    /// so a live chat does not rewrite the ledger on every save pass.
    pub(crate) fn sighting_is_worth_recording(
        &self,
        workspace_key: &str,
        session_id: &str,
        now_ms: u64,
    ) -> bool {
        self.chats_for(workspace_key)
            .iter()
            .find(|chat| chat.session_id == session_id)
            .is_none_or(|chat| now_ms.saturating_sub(chat.last_seen_ms) >= SIGHTING_FRESH_FOR_MS)
    }

    /// Apply a batch of observations, returning whether anything changed.
    pub fn apply_observations(
        &mut self,
        observations: Vec<(String, ChatObservation)>,
        now_ms: u64,
    ) -> bool {
        let mut changed = false;
        for (key, observation) in observations {
            if !self.sighting_is_worth_recording(&key, &observation.session_id, now_ms) {
                continue;
            }
            changed |= self.record_at(&key, observation, now_ms);
        }
        changed
    }
}

/// Project the ledger into the presentation rows the sidebar reads.
///
/// One function so startup and every later refresh agree: two projections that
/// drifted would make the drawer show different history before and after the
/// first save. Titles are resolved separately — a row without one still carries
/// the association, which is the information the drawer exists for.
pub fn project_rows(
    ledger: &WorkspaceChatLedger,
) -> std::collections::HashMap<String, Vec<crate::app::state::WorkspaceChatRow>> {
    ledger
        .workspaces
        .iter()
        .map(|(key, entry)| {
            let rows = entry
                .chats
                .iter()
                .map(|chat| crate::app::state::WorkspaceChatRow {
                    session_id: chat.session_id.clone(),
                    agent: chat.agent.clone(),
                    title: None,
                    last_seen_ms: chat.last_seen_ms,
                })
                .collect();
            (key.clone(), rows)
        })
        .collect()
}

/// Read the ledger, degrading to an empty one.
///
/// A missing file is the normal first-run state and a corrupt one must never
/// stop the server from starting: the ledger is a convenience, not a
/// dependency. Both cases return an empty ledger; only the corrupt case warns.
pub fn load_from_path(path: &Path) -> WorkspaceChatLedger {
    if !path.exists() {
        return WorkspaceChatLedger::default();
    }
    let content = match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(err) => {
            warn!(path = %path.display(), %err, "failed to read workspace chat ledger");
            return WorkspaceChatLedger::default();
        }
    };
    match serde_json::from_str::<WorkspaceChatLedger>(&content) {
        Ok(ledger) if ledger.version == LEDGER_VERSION => ledger,
        Ok(ledger) => {
            warn!(
                path = %path.display(),
                found = ledger.version,
                expected = LEDGER_VERSION,
                "unsupported workspace chat ledger version; starting empty"
            );
            WorkspaceChatLedger::default()
        }
        Err(err) => {
            warn!(path = %path.display(), %err, "failed to parse workspace chat ledger");
            WorkspaceChatLedger::default()
        }
    }
}

/// Write the ledger atomically: a crash mid-write must never leave a truncated
/// file behind, because the next start would read it as corrupt and drop the
/// entire history.
pub fn save_to_path(path: &Path, ledger: &WorkspaceChatLedger) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(ledger)?;
    let tmp_path = path.with_extension("json.tmp");
    std::fs::write(&tmp_path, json)?;
    #[cfg(windows)]
    if path.exists() {
        if let Err(err) = std::fs::remove_file(path) {
            let _ = std::fs::remove_file(&tmp_path);
            return Err(err);
        }
    }
    if let Err(err) = std::fs::rename(&tmp_path, path) {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(err);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observation(session_id: &str) -> ChatObservation {
        ChatObservation {
            session_id: session_id.to_string(),
            agent: "claude".to_string(),
            source: "herdr:claude".to_string(),
            kind: "id".to_string(),
        }
    }

    fn temp_ledger_path(name: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|elapsed| elapsed.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!(
            "herdr-ws-chats-{name}-{}-{nanos}.json",
            std::process::id()
        ))
    }

    struct TempPath(PathBuf);

    impl Drop for TempPath {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
            let _ = std::fs::remove_file(self.0.with_extension("json.tmp"));
        }
    }

    // TP-WSCHAT-01: an observation becomes a record. This is the whole premise
    // — the live wiring is the only place that knows a chat ran in a workspace
    // whose directory the agent never used as its cwd.
    #[test]
    fn a_first_sighting_becomes_a_record() {
        let mut ledger = WorkspaceChatLedger::default();

        assert!(ledger.record_at("/repo", observation("s1"), 1_000));

        let chats = ledger.chats_for("/repo");
        assert_eq!(chats.len(), 1);
        assert_eq!(chats[0].session_id, "s1");
        assert_eq!(chats[0].first_seen_ms, 1_000);
        assert_eq!(chats[0].last_seen_ms, 1_000);
    }

    // TP-WSCHAT-02: hook reports repeat constantly. Without upsert the ledger
    // would grow one row per report, and "when did I start working with this
    // chat here" would be overwritten by the most recent sighting.
    #[test]
    fn a_repeat_sighting_updates_the_time_without_duplicating_or_rewriting_the_first() {
        let mut ledger = WorkspaceChatLedger::default();
        ledger.record_at("/repo", observation("s1"), 1_000);

        assert!(ledger.record_at("/repo", observation("s1"), 5_000));

        let chats = ledger.chats_for("/repo");
        assert_eq!(chats.len(), 1, "a repeat must not duplicate the row");
        assert_eq!(chats[0].first_seen_ms, 1_000, "the first sighting is fixed");
        assert_eq!(chats[0].last_seen_ms, 5_000);
    }

    // TP-WSCHAT-02: an identical report that changes nothing must say so, or
    // every hook report would schedule a disk write.
    #[test]
    fn an_identical_repeat_reports_no_change() {
        let mut ledger = WorkspaceChatLedger::default();
        ledger.record_at("/repo", observation("s1"), 1_000);

        assert!(
            !ledger.record_at("/repo", observation("s1"), 1_000),
            "nothing changed, so nothing should be written"
        );
    }

    // TP-WSCHAT-03: the newest sighting leads, because that is the order the
    // drawer shows and re-sorting at render time would be state work in the
    // render path.
    #[test]
    fn the_most_recent_sighting_leads_the_list() {
        let mut ledger = WorkspaceChatLedger::default();
        ledger.record_at("/repo", observation("old"), 1_000);
        ledger.record_at("/repo", observation("new"), 2_000);
        ledger.record_at("/repo", observation("old"), 3_000);

        let ids: Vec<_> = ledger
            .chats_for("/repo")
            .iter()
            .map(|chat| chat.session_id.as_str())
            .collect();
        assert_eq!(ids, vec!["old", "new"]);
    }

    // TP-WSCHAT-04: workspaces do not bleed into each other. The whole point is
    // per-branch attribution; a shared list would answer the wrong question.
    #[test]
    fn each_workspace_keeps_its_own_chats() {
        let mut ledger = WorkspaceChatLedger::default();
        ledger.record_at("/repo/main", observation("a"), 1_000);
        ledger.record_at("/repo/branch", observation("b"), 1_000);

        assert_eq!(ledger.chats_for("/repo/main").len(), 1);
        assert_eq!(ledger.chats_for("/repo/branch")[0].session_id, "b");
        assert!(ledger.chats_for("/unknown").is_empty());
    }

    // TP-WSCHAT-05: an unbounded ledger is a disk-space bug. The oldest
    // sightings are the least useful, so the cap drops them.
    #[test]
    fn the_ledger_caps_a_workspaces_history_and_drops_the_oldest() {
        let mut ledger = WorkspaceChatLedger::default();
        for index in 0..(MAX_CHATS_PER_WORKSPACE + 10) {
            ledger.record_at(
                "/repo",
                observation(&format!("s{index}")),
                1_000 + index as u64,
            );
        }

        let chats = ledger.chats_for("/repo");
        assert_eq!(chats.len(), MAX_CHATS_PER_WORKSPACE);
        assert_eq!(
            chats[0].session_id,
            format!("s{}", MAX_CHATS_PER_WORKSPACE + 9),
            "the newest sighting survives"
        );
        assert!(
            !chats.iter().any(|chat| chat.session_id == "s0"),
            "the oldest sighting is the one dropped"
        );
    }

    // TP-WSCHAT-06: garbage in must not become garbage out. An empty key or
    // session id means the caller could not resolve the association; recording
    // it would attribute a chat to nowhere.
    #[test]
    fn an_unresolvable_observation_is_refused() {
        let mut ledger = WorkspaceChatLedger::default();

        assert!(!ledger.record_at("", observation("s1"), 1_000));
        assert!(!ledger.record_at("/repo", observation(""), 1_000));
        assert!(ledger.workspaces.is_empty());
    }

    // TP-WSCHAT-07: the ledger survives a restart — that is the difference
    // between it and the live wiring it exists to outlast.
    #[test]
    fn the_ledger_round_trips_through_disk() {
        let path = TempPath(temp_ledger_path("roundtrip"));
        let mut ledger = WorkspaceChatLedger::default();
        ledger.record_at("/repo", observation("s1"), 1_000);
        ledger.record_at("/repo", observation("s2"), 2_000);

        save_to_path(&path.0, &ledger).expect("ledger should save");
        let loaded = load_from_path(&path.0);

        assert_eq!(loaded, ledger);
        assert_eq!(loaded.chats_for("/repo").len(), 2);
    }

    // TP-WSCHAT-08: a corrupt or missing file must never stop the server. The
    // ledger is a convenience; losing it costs history, not availability.
    #[test]
    fn a_corrupt_or_missing_ledger_degrades_to_empty_without_panicking() {
        let missing = temp_ledger_path("missing");
        assert_eq!(load_from_path(&missing), WorkspaceChatLedger::default());

        let corrupt = TempPath(temp_ledger_path("corrupt"));
        std::fs::write(&corrupt.0, b"{not json at all").expect("write corrupt fixture");
        assert_eq!(load_from_path(&corrupt.0), WorkspaceChatLedger::default());

        let future = TempPath(temp_ledger_path("future"));
        std::fs::write(&future.0, br#"{"version":99999,"workspaces":{}}"#)
            .expect("write future-version fixture");
        assert_eq!(
            load_from_path(&future.0),
            WorkspaceChatLedger::default(),
            "an unknown schema version is not guessed at"
        );
    }

    // TP-WSCHAT-09: a crash mid-write must not truncate the history. The write
    // goes to a temp file first, so the real path only ever holds a complete
    // ledger and no stray temp file is left behind.
    #[test]
    fn saving_leaves_no_temp_file_and_replaces_the_previous_ledger() {
        let path = TempPath(temp_ledger_path("atomic"));
        let mut first = WorkspaceChatLedger::default();
        first.record_at("/repo", observation("s1"), 1_000);
        save_to_path(&path.0, &first).expect("first save");

        let mut second = WorkspaceChatLedger::default();
        second.record_at("/repo", observation("s2"), 2_000);
        save_to_path(&path.0, &second).expect("second save");

        assert!(
            !path.0.with_extension("json.tmp").exists(),
            "the temp file must not survive a successful save"
        );
        let loaded = load_from_path(&path.0);
        assert_eq!(loaded.chats_for("/repo").len(), 1);
        assert_eq!(loaded.chats_for("/repo")[0].session_id, "s2");
    }

    fn snapshot_pane(session: Option<&str>) -> crate::persist::snapshot::PaneSnapshot {
        crate::persist::snapshot::PaneSnapshot {
            cwd: PathBuf::from("/repo"),
            label: None,
            agent_name: None,
            managed_agent_kind: None,
            agent_session: session.map(|value| {
                crate::persist::snapshot::PaneAgentSessionSnapshot {
                    source: "herdr:claude".to_string(),
                    agent: "claude".to_string(),
                    kind: crate::agent_resume::AgentSessionRefKind::Id,
                    value: value.to_string(),
                }
            }),
            launch_argv: None,
        }
    }

    fn snapshot_with(
        workspaces: Vec<(&str, Vec<Vec<Option<&str>>>)>,
    ) -> crate::persist::SessionSnapshot {
        crate::persist::SessionSnapshot {
            version: 1,
            workspaces: workspaces
                .into_iter()
                .map(|(cwd, tabs)| crate::persist::WorkspaceSnapshot {
                    id: None,
                    custom_name: None,
                    identity_cwd: PathBuf::from(cwd),
                    worktree_space: None,
                    public_pane_numbers: Default::default(),
                    next_public_pane_number: 1,
                    public_tab_numbers: Vec::new(),
                    next_public_tab_number: 1,
                    tabs: tabs
                        .into_iter()
                        .map(|panes| crate::persist::TabSnapshot {
                            custom_name: None,
                            layout: crate::persist::LayoutSnapshot::Pane(1),
                            panes: panes
                                .into_iter()
                                .enumerate()
                                .map(|(idx, session)| (idx as u32, snapshot_pane(session)))
                                .collect(),
                            zoomed: false,
                            focused: None,
                            root_pane: None,
                        })
                        .collect(),
                    active_tab: 0,
                })
                .collect(),
            active: None,
            selected: 0,
            shell: None,
            sidebar_width: None,
            sidebar_section_split: None,
            collapsed_space_keys: Default::default(),
            files_tab: None,
        }
    }

    // TP-WSCHAT-11: the observer reads the snapshot, which already resolved
    // hook-authority-over-persisted precedence. Deriving that again here would
    // let the ledger and the session file disagree about the same pane.
    #[test]
    fn the_observer_collects_one_association_per_workspace_and_session() {
        let snapshot = snapshot_with(vec![
            ("/repo/main", vec![vec![Some("s1"), None], vec![Some("s2")]]),
            ("/repo/branch", vec![vec![Some("s3")]]),
        ]);

        let observations = observe_from_snapshot(&snapshot);

        let pairs: Vec<_> = observations
            .iter()
            .map(|(key, obs)| (key.as_str(), obs.session_id.as_str()))
            .collect();
        assert_eq!(
            pairs,
            vec![
                ("/repo/main", "s1"),
                ("/repo/main", "s2"),
                ("/repo/branch", "s3"),
            ]
        );
    }

    // TP-WSCHAT-11: a split showing the same agent twice is still one
    // association — otherwise the drawer would list the chat once per pane.
    #[test]
    fn the_observer_deduplicates_the_same_chat_across_panes() {
        let snapshot = snapshot_with(vec![(
            "/repo/main",
            vec![vec![Some("s1"), Some("s1")], vec![Some("s1")]],
        )]);

        assert_eq!(observe_from_snapshot(&snapshot).len(), 1);
    }

    // TP-WSCHAT-11: a pane with no agent contributes nothing. Recording an
    // empty association would attribute a chat to nowhere.
    #[test]
    fn the_observer_ignores_panes_without_a_session() {
        let snapshot = snapshot_with(vec![("/repo/main", vec![vec![None, None]])]);

        assert!(observe_from_snapshot(&snapshot).is_empty());
    }

    // TP-WSCHAT-12: the observer runs on every debounced save. Without the
    // freshness policy each pass would advance last_seen and rewrite the whole
    // ledger — a live chat would mean a permanent write loop.
    #[test]
    fn a_fresh_sighting_is_not_re_recorded_but_a_stale_one_is() {
        let mut ledger = WorkspaceChatLedger::default();
        let observations = || vec![("/repo".to_string(), observation("s1"))];

        assert!(
            ledger.apply_observations(observations(), 1_000),
            "the first sighting must be recorded"
        );
        assert!(
            !ledger.apply_observations(observations(), 1_000 + SIGHTING_FRESH_FOR_MS - 1),
            "a still-fresh sighting must not schedule a write"
        );
        assert!(
            ledger.apply_observations(observations(), 1_000 + SIGHTING_FRESH_FOR_MS),
            "once stale, the sighting is recorded again"
        );
        assert_eq!(
            ledger.chats_for("/repo")[0].last_seen_ms,
            1_000 + SIGHTING_FRESH_FOR_MS
        );
        assert_eq!(
            ledger.chats_for("/repo")[0].first_seen_ms,
            1_000,
            "refreshing never moves the first sighting"
        );
    }

    // TP-WSCHAT-10: one workspace, one key. A non-normalized path would
    // otherwise split a branch's history across two entries. Canonicalization
    // only applies to paths that exist, so the second half pins the documented
    // fallback: a workspace whose directory is gone keeps its raw-path key and
    // therefore keeps its history instead of silently starting over.
    #[test]
    fn the_ledger_key_is_canonical_and_falls_back_for_missing_paths() {
        let root = std::env::temp_dir().join(format!("herdr-key-probe-{}", std::process::id()));
        let nested = root.join("nested");
        std::fs::create_dir_all(&nested).expect("create key probe dirs");

        let messy = nested.join("..");
        assert_eq!(
            ledger_key(&messy),
            ledger_key(&root),
            "two spellings of one existing directory must share a key"
        );

        let _ = std::fs::remove_dir_all(&root);

        let gone = std::path::Path::new("/definitely/not/a/real/path");
        assert_eq!(
            ledger_key(gone),
            gone.to_string_lossy(),
            "a missing directory keeps its raw key so its history survives"
        );
    }
}
