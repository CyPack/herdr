//! The graveyard that survives a restart.
//!
//! The closed-agent ledger lived entirely in memory: a `Default::default()` at
//! startup, a ring of eight, and nothing on disk. Every restart emptied it —
//! and on this machine a restart is not rare, because each delivery replaces
//! the running server through `live-handoff`. Measured 2026-08-16 03:10: two
//! ghosts stood in the panel before a delivery and zero after it, including
//! the agent the user had closed minutes earlier.
//!
//! So "let me see the ones that opened and closed in the last month" was never
//! a capacity setting. It was a missing store.
//!
//! Shape follows `workspace_chats`: a versioned document, a disk type kept
//! separate from the runtime one, atomic writes, and pure functions that take
//! an explicit clock so every rule below is testable without touching a real
//! config directory or waiting for time to pass.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tracing::warn;

/// On-disk schema version. Bumped only for a breaking layout change; an
/// unknown version is treated as unreadable rather than guessed at.
pub const CLOSED_AGENTS_VERSION: u32 = 1;

/// How far back the graveyard remembers.
///
/// The user asked for "the last month", and a month is also where the value
/// stops: a ghost older than that is archaeology, and its working directory
/// has usually moved on. Kept as a constant rather than a setting because a
/// knob nobody turns is a knob that only adds a way to be wrong.
pub const RETENTION_MS: u64 = 30 * 24 * 60 * 60 * 1000;

/// Upper bound regardless of age.
///
/// Persistence is not permission to grow without limit: a machine that closes
/// agents all day would otherwise turn a convenience into a disk-space bug —
/// the same reasoning `MAX_CHATS_PER_WORKSPACE` carries next door. Five
/// hundred is far past what any panel shows and far short of what any disk
/// notices.
pub const MAX_RECORDS: usize = 500;

/// The document as it sits on disk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClosedAgentStore {
    pub version: u32,
    #[serde(default)]
    pub records: Vec<StoredClosedAgent>,
}

impl Default for ClosedAgentStore {
    fn default() -> Self {
        Self {
            version: CLOSED_AGENTS_VERSION,
            records: Vec::new(),
        }
    }
}

/// One remembered death.
///
/// Deliberately *not* the runtime `ClosedAgentRecord`: that type carries
/// `RevivalState`, which is a fact about this process and not about the agent.
/// A revival in flight when the server was replaced did not survive the
/// replacement, so persisting it would resurrect a lie — every loaded row
/// starts dormant, which is also what `record_closed` already enforces for a
/// fresh death.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredClosedAgent {
    pub agent_id: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<StoredSession>,
    pub closed_at: u64,
}

/// The resume recipe, frozen at close time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredSession {
    pub source: String,
    pub agent: String,
    pub ref_kind: crate::agent_resume::AgentSessionRefKind,
    pub ref_value: String,
}

pub fn default_store_path() -> PathBuf {
    crate::config::config_dir().join("closed-agents.json")
}

/// Drop what the graveyard should no longer hold, newest first.
///
/// Two bounds, in this order: age first because it is the one the user asked
/// about, then count as the backstop. The clock is a parameter so the rule is
/// testable without waiting a month.
pub fn prune(
    mut records: Vec<StoredClosedAgent>,
    now_ms: u64,
    retention_ms: u64,
) -> Vec<StoredClosedAgent> {
    // Newest first: the panel draws in this order and the truncation below
    // must drop the oldest, not whatever happened to be written last.
    records.sort_by_key(|record| std::cmp::Reverse(record.closed_at));
    // TP-AGPANEL-41: one identity, one headstone. Seeding walks the drawer's
    // rows per directory key, and one conversation can appear under two of
    // them — `$HOME` and a checkout inside it both list it — so the same chat
    // was derived twice and stood in the panel twice. Deduped here rather than
    // at the seeding site because this is the single gate every record passes,
    // whether it was witnessed, derived, or read back from disk. Newest wins:
    // the list is already sorted, so the first sighting of an id is its most
    // recent one.
    let mut seen = std::collections::HashSet::new();
    records.retain(|record| seen.insert(record.agent_id.clone()));
    records.retain(|record| now_ms.saturating_sub(record.closed_at) <= retention_ms);
    records.truncate(MAX_RECORDS);
    records
}

/// Derive ghosts from the transcript store for conversations the graveyard
/// never saw.
///
/// TP-AGPANEL-39: persistence only remembers deaths it witnessed, and it
/// started witnessing them today. The month the user asked to see already
/// happened — its only surviving record is the transcript store, which keeps a
/// file per conversation with the directory it ran in and the time it last
/// moved.
///
/// Derived rows are marked by nothing: a ghost is a ghost. What matters is
/// that they carry the same two things a witnessed one does — the directory
/// and the session ref — so a revival resolves instead of refusing.
///
/// `live` decides which conversations are still open; those are not deaths and
/// must not appear as such. Callers pass the same predicate the sidebar uses,
/// so a chat cannot be both a live tab and a headstone.
///
/// TP-DAILY-25: `hidden` decides which conversations are chores. A chore that
/// ends is not a death worth marking, and deriving one anyway is how the daily
/// filter's work walked straight back onto the screen — under the user's own
/// words for what they did not want, "agents that ran and closed".
///
/// It gates DERIVING only. A death the graveyard witnessed is already in
/// `existing` and stays there: the filter is allowed to decline inventing a
/// headstone, never to forget one, because a witnessed record is the only
/// evidence that agent ran at all.
/// Reads the rows the drawer already loaded rather than re-walking the store:
/// `App::new` has just parsed that directory, and reading the same files twice
/// on a startup path buys nothing.
pub fn derive_from_chat_rows(
    existing: &[StoredClosedAgent],
    cwd: &str,
    rows: &[crate::app::state::WorkspaceChatRow],
    live: &dyn Fn(&str) -> bool,
    hidden: &dyn Fn(&str) -> bool,
) -> Vec<StoredClosedAgent> {
    rows.iter()
        .filter(|row| !live(&row.session_id))
        .filter(|row| !hidden(&row.session_id))
        .filter(|row| !existing.iter().any(|kept| kept.agent_id == row.session_id))
        .map(|row| StoredClosedAgent {
            agent_id: row.session_id.clone(),
            label: row.display_label(),
            cwd: Some(cwd.to_string()),
            workspace_key: None,
            session: Some(StoredSession {
                source: "herdr:claude".into(),
                agent: row.agent.clone(),
                ref_kind: crate::agent_resume::AgentSessionRefKind::Id,
                ref_value: row.session_id.clone(),
            }),
            closed_at: row.last_activity_ms(),
        })
        .collect()
}

/// Read the store; any failure is a normal empty start.
///
/// A graveyard that refuses to load is worse than an empty one: it would take
/// the panel down with it on a path that runs at every startup.
pub fn load_from_path(path: &Path) -> ClosedAgentStore {
    if !path.exists() {
        return ClosedAgentStore::default();
    }
    let content = match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(err) => {
            warn!(path = %path.display(), %err, "failed to read closed agent store");
            return ClosedAgentStore::default();
        }
    };
    match serde_json::from_str::<ClosedAgentStore>(&content) {
        Ok(store) if store.version == CLOSED_AGENTS_VERSION => store,
        Ok(store) => {
            warn!(
                path = %path.display(),
                found = store.version,
                expected = CLOSED_AGENTS_VERSION,
                "closed agent store version mismatch; starting empty"
            );
            ClosedAgentStore::default()
        }
        Err(err) => {
            warn!(path = %path.display(), %err, "closed agent store is unreadable; starting empty");
            ClosedAgentStore::default()
        }
    }
}

/// Write the store atomically: a crash mid-write must never leave a truncated
/// file behind, because the next start would read it as corrupt and drop the
/// whole graveyard.
pub fn save_to_path(path: &Path, store: &ClosedAgentStore) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(store)?;
    let tmp_path = path.with_extension("json.tmp");
    std::fs::write(&tmp_path, json)?;
    #[cfg(windows)]
    if path.exists() {
        if let Err(err) = std::fs::remove_file(path) {
            let _ = std::fs::remove_file(&tmp_path);
            return Err(err);
        }
    }
    std::fs::rename(&tmp_path, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_resume::AgentSessionRefKind;

    fn record(id: &str, closed_at: u64) -> StoredClosedAgent {
        StoredClosedAgent {
            agent_id: id.into(),
            label: format!("{id} did something"),
            cwd: Some("/home/tester/project".into()),
            workspace_key: Some("w1".into()),
            session: Some(StoredSession {
                source: "herdr:claude".into(),
                agent: "claude".into(),
                ref_kind: AgentSessionRefKind::Id,
                ref_value: format!("session-{id}"),
            }),
            closed_at,
        }
    }

    fn temp_path(name: &str) -> PathBuf {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!(
            "herdr-closed-agents-{name}-{}-{stamp}.json",
            std::process::id()
        ))
    }

    fn chat_row(id: &str, title: &str, ms: u64) -> crate::app::state::WorkspaceChatRow {
        crate::app::state::WorkspaceChatRow {
            session_id: id.into(),
            agent: "claude".into(),
            title: Some(title.into()),
            last_seen_ms: ms,
            last_modified: None,
            last_message_at: None,
        }
    }

    // TP-AGPANEL-39 (S1/S4): the month the user asked for already happened,
    // and its only surviving record is the transcript store. A derived ghost
    // must carry the directory and the session ref, or it is a headstone that
    // can be read and never revived.
    #[test]
    fn a_conversation_with_no_headstone_gets_one_from_the_store() {
        let derived = derive_from_chat_rows(
            &[],
            "/home/tester/project",
            &[chat_row("s1", "fixing the parser", 5_000)],
            &|_| false,
            &|_| false,
        );

        assert_eq!(derived.len(), 1);
        let row = &derived[0];
        assert_eq!(row.agent_id, "s1");
        assert_eq!(row.label, "fixing the parser");
        assert_eq!(row.cwd.as_deref(), Some("/home/tester/project"));
        assert_eq!(row.closed_at, 5_000);
        assert_eq!(
            row.session.as_ref().map(|s| s.ref_value.as_str()),
            Some("s1"),
            "a derived ghost resumes like any other"
        );
    }

    // TP-AGPANEL-39 (S2): an open conversation is not a death. Without this
    // the same chat would stand in the panel twice — once as a live agent and
    // once as its own headstone.
    #[test]
    fn a_live_conversation_is_not_given_a_headstone() {
        let derived = derive_from_chat_rows(
            &[],
            "/home/tester/project",
            &[chat_row("open", "still going", 5_000)],
            &|id| id == "open",
            &|_| false,
        );

        assert!(derived.is_empty());
    }

    // TP-AGPANEL-39 (S3): a witnessed death outranks a derived one. Two ghosts
    // with one identity leave the user guessing which revives, and the answer
    // would be neither reliably — the same reasoning `record_closed` dedups on.
    #[test]
    fn a_conversation_already_remembered_is_not_derived_twice() {
        let existing = vec![record("s1", 9_000)];

        let derived = derive_from_chat_rows(
            &existing,
            "/home/tester/project",
            &[chat_row("s1", "a different title", 5_000)],
            &|_| false,
            &|_| false,
        );

        assert!(derived.is_empty());
    }

    /// TP-DAILY-25 (G1): a chat the rules call routine gets no headstone.
    ///
    /// This is the half of the request the daily filter could not reach: the
    /// panel's ghosts are DERIVED from the same chat rows, so a chore that was
    /// taken out of the daily section walked straight back in as "an agent
    /// that closed" — which is the user's own wording for what they did not
    /// want to see.
    #[test]
    fn a_routine_conversation_gets_no_headstone() {
        let derived = derive_from_chat_rows(
            &[],
            "/home/tester/project",
            &[chat_row("chore", "nightly sweep", 5_000)],
            &|_| false,
            &|id| id == "chore",
        );

        assert!(derived.is_empty(), "a chore is not a death worth marking");
    }

    /// TP-DAILY-25 (G3): a chat nothing hid is derived as before. The filter
    /// adds a reason to skip; it must not become a reason to skip everything.
    #[test]
    fn a_chat_nothing_hides_still_gets_its_headstone() {
        let derived = derive_from_chat_rows(
            &[],
            "/home/tester/project",
            &[chat_row("real", "fixing the parser", 5_000)],
            &|_| false,
            &|_| false,
        );

        assert_eq!(derived.len(), 1);
        assert_eq!(derived[0].agent_id, "real");
    }

    /// TP-DAILY-25 (G5): a death the graveyard WITNESSED stays, whatever the
    /// rules make of that conversation.
    ///
    /// Deriving and remembering are different acts. The filter says "do not
    /// invent a headstone for this"; it must never say "forget the one you
    /// already have", because a witnessed death is the only record that an
    /// agent ran at all — and losing it is losing history rather than tidying
    /// a list.
    #[test]
    fn a_witnessed_death_survives_the_hidden_filter() {
        let existing = vec![record("chore", 9_000)];
        let kept = prune(existing.clone(), 9_500, RETENTION_MS);
        assert_eq!(kept.len(), 1, "the store still holds it");

        let derived = derive_from_chat_rows(
            &existing,
            "/home/tester/project",
            &[chat_row("chore", "nightly sweep", 5_000)],
            &|_| false,
            &|id| id == "chore",
        );
        assert!(
            derived.is_empty(),
            "nothing new is derived, and the witnessed record above is untouched"
        );
    }

    // TP-AGPANEL-30: the graveyard survives the process that wrote it.
    // Without this the panel forgets every death on restart — and on this
    // machine each delivery replaces the server, so "restart" means "roughly
    // every time anything ships".
    #[test]
    fn a_saved_graveyard_reads_back_unchanged() {
        let path = temp_path("roundtrip");
        let store = ClosedAgentStore {
            version: CLOSED_AGENTS_VERSION,
            records: vec![record("a", 3_000), record("b", 2_000)],
        };

        save_to_path(&path, &store).expect("save");
        let loaded = load_from_path(&path);
        let _ = std::fs::remove_file(&path);

        assert_eq!(loaded, store, "what was written is what comes back");
    }

    // TP-AGPANEL-31: age is the bound the user asked for — "the ones that
    // opened and closed in the last month". A ghost older than the window is
    // archaeology, and its directory has usually moved on.
    #[test]
    fn a_ghost_older_than_the_window_is_dropped() {
        let now = 100 * 24 * 60 * 60 * 1000;
        let fresh = record("fresh", now - 1_000);
        let stale = record("stale", now - RETENTION_MS - 1);

        let kept = prune(vec![fresh.clone(), stale], now, RETENTION_MS);

        assert_eq!(kept, vec![fresh], "only what falls inside the window stays");
    }

    // TP-AGPANEL-32: newest first, and the count bound drops the oldest —
    // not whatever happened to be appended last. The panel reads this order
    // directly, so an unsorted store would draw a graveyard in write order.
    #[test]
    fn pruning_keeps_the_newest_first_and_evicts_the_oldest() {
        let now = RETENTION_MS * 2;
        // Built OLDEST first on purpose. Generated newest-first, this test
        // passed with the sort removed — the input was already in the order
        // the assertion wanted, so it proved nothing. Mutation caught that.
        let mut records: Vec<_> = (0..MAX_RECORDS + 5)
            .map(|i| record(&format!("g{i}"), now - i as u64))
            .collect();
        records.reverse();

        let kept = prune(records, now, RETENTION_MS);

        assert_eq!(kept.len(), MAX_RECORDS, "the count bound holds");
        assert_eq!(
            kept.first().map(|r| r.agent_id.as_str()),
            Some("g0"),
            "newest first"
        );
        assert!(
            !kept
                .iter()
                .any(|r| r.agent_id == format!("g{}", MAX_RECORDS + 4)),
            "the oldest is the one evicted"
        );
    }

    // TP-AGPANEL-41: one identity, one headstone — and the surviving row is
    // the most recent sighting. Seeding walks the drawer per directory key and
    // one conversation can appear under two of them, which is how the live
    // panel came back with the same title twice.
    #[test]
    fn one_identity_leaves_one_headstone() {
        let now = RETENTION_MS * 2;

        let kept = prune(
            vec![record("twin", now - 5_000), record("twin", now - 1_000)],
            now,
            RETENTION_MS,
        );

        assert_eq!(kept.len(), 1, "the same chat cannot stand twice");
        assert_eq!(
            kept[0].closed_at,
            now - 1_000,
            "the most recent sighting is the one that survives"
        );
    }

    // TP-AGPANEL-33: this file is read on a path that runs at every startup,
    // so a broken one must not take the panel down with it. An empty
    // graveyard is a normal state; a failing start is not.
    #[test]
    fn an_unreadable_store_starts_empty_instead_of_failing() {
        let path = temp_path("corrupt");
        std::fs::write(&path, "{ this is not json").expect("write");

        let loaded = load_from_path(&path);
        let _ = std::fs::remove_file(&path);

        assert_eq!(loaded, ClosedAgentStore::default());
    }

    #[test]
    fn a_store_from_another_version_starts_empty() {
        let path = temp_path("version");
        std::fs::write(
            &path,
            r#"{"version":99,"records":[{"agent_id":"x","label":"x","closed_at":1}]}"#,
        )
        .expect("write");

        let loaded = load_from_path(&path);
        let _ = std::fs::remove_file(&path);

        assert!(
            loaded.records.is_empty(),
            "an unknown layout is unreadable, not guessed at"
        );
    }

    #[test]
    fn a_missing_store_is_an_empty_one() {
        let path = temp_path("absent");
        assert_eq!(load_from_path(&path), ClosedAgentStore::default());
    }
}
