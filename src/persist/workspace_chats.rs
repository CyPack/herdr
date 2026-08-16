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
    /// User-decided re-homes: session id → the ledger key whose drawer the
    /// chat belongs to now (TP-CHAT-MOVE-01). Deliberately additive on the
    /// version-1 schema: an older binary still reads the file and only loses
    /// the moves if it saves — the observed history itself is never at risk.
    #[serde(default)]
    pub moves: BTreeMap<String, String>,
    /// User-chosen names: session id → the name that chat's row wears
    /// (TP-CHAT-NAME-01). Additive on the version-1 schema for the same reason
    /// `moves` is: an older binary still reads the file and only loses the
    /// names if it saves, while the observed history itself is never at risk.
    ///
    /// Kept beside the observations rather than inside `ChatRecord` because a
    /// name belongs to the conversation, not to the workspace that happened to
    /// see it — the same session observed in two directories must not be able
    /// to wear two different names.
    #[serde(default)]
    pub names: BTreeMap<String, String>,
}

impl Default for WorkspaceChatLedger {
    fn default() -> Self {
        Self {
            version: LEDGER_VERSION,
            workspaces: BTreeMap::new(),
            moves: BTreeMap::new(),
            names: BTreeMap::new(),
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

/// The prefix that separates container identities from directory keys.
pub const MODULE_KEY_PREFIX: &str = "module:";

/// Ledger identity of a declared container.
///
/// TP-CHAT-MOVE-05: a container is a label first and a directory maybe never —
/// it can be declared before any checkout joins it, and the person who declared
/// it may never give it one. So it cannot be keyed by a path, and this is the
/// one place that decides what it is keyed by instead.
///
/// The ledger's key space now holds two kinds of string, and they must not be
/// confused: `/home/ayaz/projects/herdr` is somewhere on disk, `module:docs`
/// is not. Every reader builds its own key rather than parsing someone else's
/// — [`ledger_key`] for directories, this for containers — so nothing
/// downstream has to tell them apart by shape. A function that takes this
/// string for a path produces the silent class of defect #88 was: no error, no
/// crash, just a lookup that never matches.
///
/// A predicate that asks "is this key a container?" deliberately does not
/// exist yet. Nothing in the product needs to ask — every reader knows which
/// kind it built — and an unused one would be scaffolding the lint rejects.
/// It arrives with the first caller that genuinely holds an unknown key.
pub fn module_ledger_key(node_key: &str) -> String {
    format!("{MODULE_KEY_PREFIX}{node_key}")
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

    /// Re-home a chat: from now on it belongs to `target_key`'s drawer, no
    /// matter where it was observed (TP-CHAT-MOVE-01). Returns whether the
    /// ledger changed, so an identical decision never schedules a write.
    pub fn set_move(&mut self, session_id: &str, target_key: &str) -> bool {
        if session_id.is_empty() || target_key.is_empty() {
            return false;
        }
        if self.moves.get(session_id).map(String::as_str) == Some(target_key) {
            return false;
        }
        self.moves
            .insert(session_id.to_string(), target_key.to_string());
        true
    }

    /// Withdraw a re-home: the chat returns to wherever it was observed
    /// (TP-CHAT-MOVE-03).
    pub fn clear_move(&mut self, session_id: &str) -> bool {
        self.moves.remove(session_id).is_some()
    }

    /// Name a chat. Returns whether the ledger changed, so re-submitting the
    /// name already on screen never schedules a write.
    ///
    /// TP-CHAT-NAME-01: a blank name is not stored as a blank — it withdraws
    /// the name instead, which is the only sensible reading of "clear the box
    /// and press enter" and leaves the row with the title it would have had.
    pub fn set_name(&mut self, session_id: &str, name: &str) -> bool {
        let name = name.trim();
        if session_id.is_empty() {
            return false;
        }
        if name.is_empty() {
            return self.clear_name(session_id);
        }
        if self.names.get(session_id).map(String::as_str) == Some(name) {
            return false;
        }
        self.names.insert(session_id.to_string(), name.to_string());
        true
    }

    /// Withdraw a name: the row goes back to whatever the transcript says.
    pub fn clear_name(&mut self, session_id: &str) -> bool {
        self.names.remove(session_id).is_some()
    }
}

/// Lay the user's chat names over the assembled presentation rows.
///
/// TP-CHAT-NAME-01: this runs LAST, beside `apply_chat_moves` and for the
/// identical reason. The agent's own store re-answers every refresh with the
/// title it derived from the transcript, so a name written any earlier is
/// overwritten within one sync — the chat would appear to rename itself back.
pub fn apply_chat_names(
    rows: &mut std::collections::HashMap<String, Vec<crate::app::state::WorkspaceChatRow>>,
    names: &BTreeMap<String, String>,
) {
    if names.is_empty() {
        return;
    }
    for chats in rows.values_mut() {
        for chat in chats.iter_mut() {
            if let Some(name) = names.get(&chat.session_id) {
                chat.title = Some(name.clone());
            }
        }
    }
}

/// Apply the user's re-homes to the assembled presentation rows.
///
/// This runs as the LAST step of the sync, after the ledger projection AND the
/// agent-store merge: applying it inside `project_rows` alone would let the
/// agent's own cwd-keyed store re-leak a moved chat back into its source
/// drawer on the very next refresh (TP-CHAT-MOVE-01).
pub fn apply_chat_moves(
    rows: &mut std::collections::HashMap<String, Vec<crate::app::state::WorkspaceChatRow>>,
    moves: &BTreeMap<String, String>,
) {
    for (session_id, target_key) in moves {
        // Pull the chat out of every drawer, keeping its freshest copy —
        // the target's own copy counts too, so re-inserting cannot duplicate.
        let mut best: Option<crate::app::state::WorkspaceChatRow> = None;
        for list in rows.values_mut() {
            list.retain(|row| {
                if row.session_id == *session_id {
                    if best
                        .as_ref()
                        .is_none_or(|kept| row.last_seen_ms > kept.last_seen_ms)
                    {
                        best = Some(row.clone());
                    }
                    false
                } else {
                    true
                }
            });
        }
        // A session nobody shows moves nothing; the map never learns keys
        // for ghosts.
        let Some(moved) = best else {
            continue;
        };
        let list = rows.entry(target_key.clone()).or_default();
        let position = list
            .iter()
            .position(|row| row.last_seen_ms < moved.last_seen_ms)
            .unwrap_or(list.len());
        list.insert(position, moved);
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
        // TP-WSID-04: a sighting is attributed to the directory the row
        // MEANS — the checkout when the workspace carries one.
        let key = ledger_key(
            workspace
                .worktree_space
                .as_ref()
                .map(|space| space.checkout_path.as_path())
                .unwrap_or(&workspace.identity_cwd),
        );
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
                    last_modified: None,
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

    fn row(session_id: &str, last_seen_ms: u64) -> crate::app::state::WorkspaceChatRow {
        crate::app::state::WorkspaceChatRow {
            session_id: session_id.to_string(),
            agent: "claude".to_string(),
            title: None,
            last_seen_ms,
            last_modified: None,
        }
    }

    // T3.6 / TP-CHAT-NAME-01: the name the user typed is what the row reads,
    // wherever that row was assembled from.
    #[test]
    fn a_named_chat_wears_its_name_in_every_drawer_it_appears_in() {
        let mut rows = std::collections::HashMap::from([
            ("/repo/main".to_string(), vec![row("s1", 5_000)]),
            // The agent-store merge attributed the same chat here too, with
            // whatever title it derived from the transcript.
            ("/repo/other".to_string(), vec![row("s1", 3_000)]),
        ]);
        let names = BTreeMap::from([("s1".to_string(), "gece nöbeti".to_string())]);

        apply_chat_names(&mut rows, &names);

        assert_eq!(rows["/repo/main"][0].title.as_deref(), Some("gece nöbeti"));
        assert_eq!(rows["/repo/other"][0].title.as_deref(), Some("gece nöbeti"));
    }

    // TP-CHAT-NAME-01: an unnamed chat is left exactly as the sources built
    // it. The overlay names what it was told to and nothing else.
    #[test]
    fn an_unnamed_chat_keeps_whatever_title_it_had() {
        let mut rows =
            std::collections::HashMap::from([("/repo/main".to_string(), vec![row("s1", 5_000)])]);
        let before = rows["/repo/main"][0].title.clone();

        apply_chat_names(&mut rows, &BTreeMap::new());

        assert_eq!(rows["/repo/main"][0].title, before);
    }

    // TP-CHAT-NAME-01: a name is stored once, re-submitting it changes
    // nothing, and clearing the box withdraws the name rather than storing an
    // empty one — a blank row would be worse than the title it replaced.
    #[test]
    fn naming_a_chat_is_idempotent_and_a_blank_withdraws_it() {
        let mut ledger = WorkspaceChatLedger::default();

        assert!(ledger.set_name("s1", "gece nöbeti"));
        assert!(
            !ledger.set_name("s1", "  gece nöbeti  "),
            "the same decision must not schedule a second write"
        );
        assert_eq!(
            ledger.names.get("s1").map(String::as_str),
            Some("gece nöbeti")
        );

        assert!(ledger.set_name("s1", "   "));
        assert!(
            !ledger.names.contains_key("s1"),
            "a cleared box withdraws the name instead of storing a blank"
        );
        assert!(
            !ledger.set_name("s1", ""),
            "withdrawing twice changes nothing"
        );
    }

    // TP-CHAT-MOVE-01: a re-home wins over every source — the ledger's own
    // projection and the agent-store merge alike — and the chat appears in
    // exactly one drawer afterwards, in recency order.
    #[test]
    fn a_move_relocates_the_chat_out_of_every_source_drawer() {
        let mut rows = std::collections::HashMap::from([
            (
                "/repo/main".to_string(),
                vec![row("s1", 5_000), row("s2", 4_000)],
            ),
            // The agent-store merge attributed the same chat here too.
            ("/repo/other".to_string(), vec![row("s1", 3_000)]),
            (
                "/repo/target".to_string(),
                vec![row("t-new", 9_000), row("t-old", 1_000)],
            ),
        ]);
        let moves = BTreeMap::from([("s1".to_string(), "/repo/target".to_string())]);

        apply_chat_moves(&mut rows, &moves);

        assert!(
            !rows["/repo/main"].iter().any(|r| r.session_id == "s1"),
            "the ledger-projected source lets go"
        );
        assert!(
            !rows["/repo/other"].iter().any(|r| r.session_id == "s1"),
            "the merge-attributed source lets go too"
        );
        let target_ids: Vec<_> = rows["/repo/target"]
            .iter()
            .map(|r| r.session_id.as_str())
            .collect();
        assert_eq!(
            target_ids,
            vec!["t-new", "s1", "t-old"],
            "the freshest sighting (5000) lands in recency order"
        );
        assert_eq!(
            rows["/repo/main"],
            vec![row("s2", 4_000)],
            "unmoved chats stay put"
        );
    }

    // TP-CHAT-MOVE-01: a move to a drawer nobody has observed yet still
    // lands — the target key may not exist in the map at all.
    #[test]
    fn a_move_to_an_unobserved_drawer_creates_it() {
        let mut rows =
            std::collections::HashMap::from([("/repo/main".to_string(), vec![row("s1", 5_000)])]);
        let moves = BTreeMap::from([("s1".to_string(), "/repo/fresh".to_string())]);

        apply_chat_moves(&mut rows, &moves);

        assert!(rows["/repo/main"].is_empty());
        assert_eq!(rows["/repo/fresh"], vec![row("s1", 5_000)]);
    }

    // TP-CHAT-MOVE-01: a chat already shown in its target must not double up,
    // and a move naming a session nobody shows is a no-op.
    #[test]
    fn a_move_never_duplicates_and_tolerates_the_unknown() {
        let mut rows =
            std::collections::HashMap::from([("/repo/target".to_string(), vec![row("s1", 5_000)])]);
        let moves = BTreeMap::from([
            ("s1".to_string(), "/repo/target".to_string()),
            ("ghost".to_string(), "/repo/target".to_string()),
        ]);

        apply_chat_moves(&mut rows, &moves);

        assert_eq!(
            rows["/repo/target"],
            vec![row("s1", 5_000)],
            "already home: one row, unchanged"
        );
    }

    // TP-CHAT-MOVE-02: the decision survives a restart, and yesterday's file
    // (no moves field) still loads — the additive-schema promise.
    #[test]
    fn moves_round_trip_and_an_old_file_loads_without_them() {
        let path = TempPath(temp_ledger_path("moves"));
        let mut ledger = WorkspaceChatLedger::default();
        ledger.record_at("/repo", observation("s1"), 1_000);
        assert!(ledger.set_move("s1", "/repo/target"));

        save_to_path(&path.0, &ledger).expect("ledger should save");
        let loaded = load_from_path(&path.0);
        assert_eq!(loaded, ledger);
        assert_eq!(
            loaded.moves.get("s1").map(String::as_str),
            Some("/repo/target")
        );

        let old = TempPath(temp_ledger_path("old-schema"));
        std::fs::write(&old.0, br#"{"version":1,"workspaces":{}}"#).expect("write old fixture");
        assert_eq!(load_from_path(&old.0).moves.len(), 0);
    }

    // TP-CHAT-MOVE-03: set and clear answer honestly — an identical decision
    // or an unknown withdrawal must never schedule a disk write.
    #[test]
    fn set_and_clear_move_report_change_honestly() {
        let mut ledger = WorkspaceChatLedger::default();

        assert!(ledger.set_move("s1", "/repo/target"), "a new decision");
        assert!(
            !ledger.set_move("s1", "/repo/target"),
            "the same decision again changes nothing"
        );
        assert!(
            ledger.set_move("s1", "/repo/elsewhere"),
            "a different target is a new decision"
        );
        assert!(ledger.clear_move("s1"), "withdrawing an existing move");
        assert!(!ledger.clear_move("s1"), "withdrawing nothing");
        assert!(!ledger.set_move("", "/repo/x"), "an empty session id");
        assert!(!ledger.set_move("s1", ""), "an empty target");
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
            collapsed_project_keys: Default::default(),
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

    // TP-WSID-04: a workspace that carries a checkout attributes its
    // sightings to the checkout — never to the shared birthplace directory
    // that let one chat list appear under several branch rows.
    #[test]
    fn the_observer_attributes_by_the_checkout_when_one_is_carried() {
        let mut snapshot = snapshot_with(vec![("/home/user", vec![vec![Some("s1")]])]);
        snapshot.workspaces[0].worktree_space = Some(crate::workspace::WorktreeSpaceMembership {
            key: "repo-key".into(),
            label: "herdr".into(),
            repo_root: PathBuf::from("/repo/herdr"),
            checkout_path: PathBuf::from("/repo/herdr-branch"),
            is_linked_worktree: true,
        });

        let observations = observe_from_snapshot(&snapshot);

        assert_eq!(observations.len(), 1);
        assert_eq!(
            observations[0].0, "/repo/herdr-branch",
            "the sighting lands under the checkout, not the birthplace"
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
