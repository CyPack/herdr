use serde::{Deserialize, Serialize};

/// One chat re-home in a bulk seat request: the ledger key is either a
/// checkout directory or a `module:<node-key>` seat — the same vocabulary
/// the TUI's "Move to branch/module..." menu writes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ChatSeatEntry {
    pub session_id: String,
    pub target_key: String,
}

/// Bulk chat seating (`chat.seat`). `source` stamps who decided: a plan
/// applier names itself (e.g. "seat-plan") so its moves can be told apart
/// from — and never override — a person's own menu moves.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ChatSeatParams {
    pub seats: Vec<ChatSeatEntry>,
    #[serde(default)]
    pub source: Option<String>,
}

/// Withdraw every move a given source wrote (`chat.unseat`) — the undo gate
/// for automated seating. A person's moves are source "user" and can only be
/// withdrawn by naming that source explicitly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ChatUnseatParams {
    pub source: String,
}
