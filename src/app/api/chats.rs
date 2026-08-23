//! Chat seating over the socket — the bulk sibling of the TUI's
//! "Move to branch/module..." menu (TP-CHAT-MOVE-13/14).
//!
//! The menu path parks a request and lets the App loop call
//! `apply_chat_move` once per chat; that is right for a person moving one
//! chat and wrong for a plan moving hundreds — each call would sync and
//! write the ledger to disk. This handler runs on the same `&mut App` the
//! API runtime already holds, folds every entry into the ledger first, and
//! then syncs ONCE and saves ONCE.

use super::responses::encode_success;
use crate::api::schema::response::ResponseResult;
use crate::api::schema::{ChatSeatParams, ChatUnseatParams};
use crate::persist::workspace_chats::{SeatOutcome, USER_MOVE_SOURCE};

impl super::App {
    pub(super) fn handle_chat_seat(&mut self, id: String, params: ChatSeatParams) -> String {
        let source = params
            .source
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or(USER_MOVE_SOURCE);
        let (mut applied, mut unchanged, mut refused) = (0usize, 0usize, 0usize);
        for entry in &params.seats {
            match self.workspace_chat_ledger.set_move_from(
                &entry.session_id,
                &entry.target_key,
                source,
            ) {
                SeatOutcome::Applied => applied += 1,
                SeatOutcome::Unchanged => unchanged += 1,
                SeatOutcome::Refused => refused += 1,
            }
        }
        if applied > 0 {
            self.finish_chat_ledger_mutation();
        }
        encode_success(
            id,
            ResponseResult::ChatSeatReport {
                applied,
                unchanged,
                refused,
            },
        )
    }

    pub(super) fn handle_chat_unseat(&mut self, id: String, params: ChatUnseatParams) -> String {
        let cleared = self
            .workspace_chat_ledger
            .clear_moves_by_source(&params.source);
        if cleared > 0 {
            self.finish_chat_ledger_mutation();
        }
        encode_success(id, ResponseResult::ChatUnseatReport { cleared })
    }

    /// The single sync + single save every ledger mutation road ends in —
    /// the same shape `apply_chat_move` uses, shared so a bulk road cannot
    /// forget the `no_session` guard and write a fixture's moves into the
    /// machine's real ledger.
    fn finish_chat_ledger_mutation(&mut self) {
        self.sync_workspace_chat_rows();
        if self.no_session {
            return;
        }
        let path = crate::persist::workspace_chats::default_ledger_path();
        if let Err(err) =
            crate::persist::workspace_chats::save_to_path(&path, &self.workspace_chat_ledger)
        {
            tracing::warn!(path = %path.display(), %err, "failed to save workspace chat ledger");
        }
    }
}
