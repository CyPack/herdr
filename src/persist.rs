//! Session persistence — save/restore workspaces, layouts, and working directories.
//!
//! Stored at `~/.config/herdr/session.json`.
//! Optional pane screen history is stored separately at `session-history.json`.
//! Installed plugins are persisted separately at `plugins.json`.
//! Which chats ran in which workspace is kept in `workspace-chats.json` —
//! deliberately outside the session snapshot, because that snapshot describes
//! the LIVE layout and the restore contract depends on its shape, while the
//! chat ledger is history that outlives the workspaces it describes.

pub mod chat_worklog;
pub mod closed_agents;
pub(crate) mod durable;
mod io;
pub mod plugin_registry;
mod restore;
mod snapshot;
pub mod workspace_chats;

pub use self::io::{clear, clear_history, load, load_history, save};
pub use self::restore::restore;
#[cfg(unix)]
pub use self::restore::{handoff_pane_aliases, restore_handoff};
pub use self::snapshot::{
    capture, capture_history, DirectionSnapshot, FilesTabSnapshot, LayoutSnapshot,
    SessionHistorySnapshot, SessionSnapshot, TabSnapshot, WorkspaceSnapshot,
};
