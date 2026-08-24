//! Per-chat work log — which commits and pushes a conversation produced.
//!
//! The transcript is where a chat's commits drown: one session touches ten
//! areas and the evidence is scattered across hundreds of tool calls. An
//! external analyzer (scripts/chat_context.py) distils them into
//! `chat-worklog.json`; this module is the read side. The runtime never
//! writes the file — the analyzer owns it, the way the agent owns its
//! transcript store — so a missing, stale or corrupt file must degrade to
//! "no work log" and never to a startup failure (the TP-WSCHAT-08 posture).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tracing::warn;

/// One commit or push the chat performed, as the analyzer recorded it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkLogEntry {
    /// Repository the commit landed in (absolute path as the tool call named it).
    #[serde(default)]
    pub repo: String,
    /// Branch from the commit's own output line, when one was captured.
    #[serde(default)]
    pub branch: Option<String>,
    /// Conventional-commit type ("feat", "fix", ... or "other").
    #[serde(rename = "type", default)]
    pub kind: String,
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub subject: String,
    /// Short hash from the commit's result; `None` means the attempt left no
    /// confirmed commit behind.
    #[serde(default)]
    pub sha: Option<String>,
    /// Transcript timestamp of the tool call (RFC3339; lexicographic order is
    /// chronological).
    #[serde(default)]
    pub ts: String,
    #[serde(default)]
    pub pushed: bool,
    #[serde(default)]
    pub status: String,
}

/// The whole file: session id → its commits, in transcript order.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatWorkLog {
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub chats: BTreeMap<String, Vec<WorkLogEntry>>,
}

impl ChatWorkLog {
    /// The confirmed entries for one chat — the popup shows work that
    /// happened, not attempts that left nothing behind.
    pub fn confirmed_for(&self, session_id: &str) -> Vec<&WorkLogEntry> {
        self.chats
            .get(session_id)
            .map(|entries| entries.iter().filter(|e| e.sha.is_some()).collect())
            .unwrap_or_default()
    }

    pub fn has_confirmed(&self, session_id: &str) -> bool {
        !self.confirmed_for(session_id).is_empty()
    }
}

pub fn default_worklog_path() -> PathBuf {
    crate::config::config_dir().join("chat-worklog.json")
}

/// Read the analyzer's file; anything short of a well-formed log is an empty
/// one, said once in the logs and never fatal.
pub fn load_from_path(path: &Path) -> ChatWorkLog {
    let raw = match std::fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return ChatWorkLog::default(),
        Err(err) => {
            warn!(path = %path.display(), %err, "failed to read chat work log");
            return ChatWorkLog::default();
        }
    };
    match serde_json::from_str::<ChatWorkLog>(&raw) {
        Ok(log) => log,
        Err(err) => {
            warn!(path = %path.display(), %err, "chat work log is not valid; ignoring it");
            ChatWorkLog::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(name: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|elapsed| elapsed.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!(
            "herdr-worklog-{name}-{}-{nanos}.json",
            std::process::id()
        ))
    }

    // TP-WORKLOG-03
    #[test]
    fn a_missing_or_corrupt_worklog_degrades_to_empty_without_panicking() {
        let missing = temp_path("missing");
        assert_eq!(load_from_path(&missing), ChatWorkLog::default());

        let corrupt = temp_path("corrupt");
        std::fs::write(&corrupt, "not json at all {").unwrap();
        assert_eq!(load_from_path(&corrupt), ChatWorkLog::default());
        let _ = std::fs::remove_file(&corrupt);
    }

    // TP-WORKLOG-03
    #[test]
    fn the_analyzers_file_shape_round_trips() {
        let path = temp_path("shape");
        std::fs::write(
            &path,
            r#"{"version":1,"chats":{"sid-1":[
                {"repo":"/r","branch":"feat/x","type":"feat","scope":"scan",
                 "subject":"add scanner","sha":"abc1234","ts":"2026-08-20T10:00:00Z",
                 "pushed":true,"status":"committed"},
                {"repo":"/r","type":"fix","subject":"attempted only","ts":"2026-08-20T11:00:00Z",
                 "pushed":false,"status":"attempted"}
            ]}}"#,
        )
        .unwrap();
        let log = load_from_path(&path);
        let entries = log.chats.get("sid-1").expect("chat present");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].kind, "feat");
        assert_eq!(entries[0].sha.as_deref(), Some("abc1234"));
        assert!(entries[0].pushed);
        // the popup's own sieve: attempts without a sha are not "work done"
        assert_eq!(log.confirmed_for("sid-1").len(), 1);
        assert!(log.has_confirmed("sid-1"));
        assert!(!log.has_confirmed("sid-none"));
        let _ = std::fs::remove_file(&path);
    }
}
