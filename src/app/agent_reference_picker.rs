//! Client-local "Add Reference to Agent..." picker: choose one live agent
//! terminal as the explicit target for a prepared file reference
//! (TP-FIP-REF-01/02/04). Opening and selecting perform no runtime work;
//! delivery still crosses the existing App-owned send boundary.

use std::path::PathBuf;

/// One selectable live agent target. Identities are snapshotted from the
/// agents projection; `live` is recomputed against current state so a
/// vanished pane renders disabled instead of silently disappearing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentReferencePickerRow {
    pub label: String,
    pub is_current: bool,
    pub workspace_id: String,
    pub pane_id: crate::layout::PaneId,
    pub terminal_id: crate::terminal::TerminalId,
    pub live: bool,
}

/// Blocking client-local picker state. Owns no watcher, worker, process,
/// pane, or server state; closing it discards only presentation data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentReferencePickerState {
    pub source_path: PathBuf,
    pub source_files_generation: u32,
    pub rows: Vec<AgentReferencePickerRow>,
    pub selected: usize,
}

/// Exact reference-delivery authority prepared by an explicit picker
/// activation. Every identity is revalidated at the send boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentReferenceRequest {
    pub path: PathBuf,
    pub source_files_generation: u32,
    pub workspace_id: String,
    pub pane_id: crate::layout::PaneId,
    pub terminal_id: crate::terminal::TerminalId,
}

impl crate::app::App {
    /// Explicit picker activation. RED stub: the GREEN seam prepares the
    /// typed AgentReferenceRequest from the selected live row.
    pub(crate) fn activate_agent_reference_picker_selection(&mut self) -> bool {
        false
    }

    /// Blocking-overlay key routing owner for the picker mode. RED stub:
    /// the popup-ownership task wires selection, activation, and close.
    pub(crate) fn handle_agent_reference_picker_key(&mut self, _key: crossterm::event::KeyEvent) {}
}

#[cfg(test)]
mod tests {
    use crate::app::state::{FileManagerContextMenuAction, Mode};

    struct PickerFixture {
        root: std::path::PathBuf,
    }

    impl PickerFixture {
        fn new(tag: &str) -> Self {
            let root = std::env::temp_dir().join(format!(
                "herdr-agent-ref-picker-{}-{tag}",
                std::process::id()
            ));
            let _ = std::fs::remove_dir_all(&root);
            std::fs::create_dir_all(&root).expect("create picker fixture root");
            Self { root }
        }

        fn file(&self, name: &str) -> std::path::PathBuf {
            let path = self.root.join(name);
            std::fs::write(&path, b"reference").expect("write picker fixture");
            path
        }
    }

    impl Drop for PickerFixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    /// One workspace, two panes, BOTH agent terminals; Files stage open.
    /// Returns the app plus the focused and neighbor pane terminal ids.
    fn app_with_two_agents(
        root: &std::path::Path,
    ) -> (
        crate::app::App,
        crate::terminal::TerminalId,
        crate::terminal::TerminalId,
    ) {
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = crate::app::App::new(
            &crate::config::Config::default(),
            true,
            None,
            api_rx,
            crate::api::EventHub::default(),
        );
        let mut workspace = crate::workspace::Workspace::test_new("agent-ref-picker");
        let focused_pane = workspace.tabs[0].root_pane;
        workspace.test_split(ratatui::layout::Direction::Horizontal);
        workspace.tabs[0].layout.focus_pane(focused_pane);
        let neighbor_pane = workspace.tabs[0]
            .layout
            .pane_ids()
            .into_iter()
            .find(|pane_id| *pane_id != focused_pane)
            .expect("neighbor pane");
        let focused_terminal = workspace
            .terminal_id(focused_pane)
            .expect("focused terminal")
            .clone();
        let neighbor_terminal = workspace
            .terminal_id(neighbor_pane)
            .expect("neighbor terminal")
            .clone();
        app.state.workspaces = vec![workspace];
        app.state.ensure_test_terminals();
        for (terminal_id, name) in [
            (&focused_terminal, "agent-focused"),
            (&neighbor_terminal, "agent-neighbor"),
        ] {
            app.state
                .terminals
                .get_mut(terminal_id)
                .expect("agent terminal state")
                .set_agent_name(name.into());
        }
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state
            .try_open_file_manager_with(|_| Some(crate::fm::FmState::new(root)))
            .expect("Files activation");
        (app, focused_terminal, neighbor_terminal)
    }

    fn dispatch_reference_action(app: &mut crate::app::App, path: std::path::PathBuf) {
        app.state.request_file_manager_context_action =
            Some(crate::app::state::FileManagerContextActionIntent {
                action: FileManagerContextMenuAction::SendAgent,
                paths: vec![path],
            });
        assert!(app.sync_file_manager_agent_handoff());
    }

    // TP-FIP-REF-01: the reference action opens a blocking picker built from
    // the live agents projection — it never sends bytes on open.
    #[tokio::test]
    async fn reference_action_opens_picker_from_live_agents_projection() {
        let fixture = PickerFixture::new("opens-picker");
        let path = fixture.file("selected.txt");
        let (mut app, focused_terminal, neighbor_terminal) = app_with_two_agents(&fixture.root);

        dispatch_reference_action(&mut app, path.clone());

        let picker = app
            .state
            .agent_reference_picker
            .as_ref()
            .expect("the reference action must open the agent picker");
        assert_eq!(app.state.mode, Mode::AgentReferencePicker);
        assert_eq!(picker.source_path, path);
        let picker_terminals: Vec<_> = picker
            .rows
            .iter()
            .map(|row| row.terminal_id.clone())
            .collect();
        assert!(picker_terminals.contains(&focused_terminal));
        assert!(picker_terminals.contains(&neighbor_terminal));
        assert_eq!(picker.rows.len(), 2, "exactly the live agent entries");
        assert!(
            app.state.request_file_manager_agent_handoff.is_none(),
            "opening the picker must not prepare delivery authority"
        );
    }

    // TP-FIP-REF-02: the focused agent is the first row, marked current,
    // and preselected.
    #[tokio::test]
    async fn current_focused_agent_is_first_and_preselected() {
        let fixture = PickerFixture::new("current-first");
        let path = fixture.file("selected.txt");
        let (mut app, focused_terminal, _) = app_with_two_agents(&fixture.root);

        dispatch_reference_action(&mut app, path);

        let picker = app
            .state
            .agent_reference_picker
            .as_ref()
            .expect("the reference action must open the agent picker");
        assert_eq!(picker.selected, 0, "the current chat is preselected");
        assert!(picker.rows[0].is_current);
        assert_eq!(picker.rows[0].terminal_id, focused_terminal);
        assert!(!picker.rows[1].is_current);
    }

    // TP-FIP-REF-04: activating a row snapshots the FULL target identity
    // (path, files generation, workspace, pane, terminal) and sends nothing.
    #[tokio::test]
    async fn picker_selection_snapshots_full_target_identity() {
        let fixture = PickerFixture::new("snapshot-identity");
        let path = fixture.file("selected.txt");
        let (mut app, focused_terminal, _) = app_with_two_agents(&fixture.root);
        let workspace_id = app.state.workspaces[0].id.clone();
        let focused_pane = app.state.workspaces[0]
            .focused_pane_id()
            .expect("focused pane");

        dispatch_reference_action(&mut app, path.clone());
        let generation = app
            .state
            .agent_reference_picker
            .as_ref()
            .expect("open picker")
            .source_files_generation;

        assert!(app.activate_agent_reference_picker_selection());
        let request = app
            .state
            .request_file_manager_agent_handoff
            .as_ref()
            .expect("activation must prepare the typed reference request");
        assert_eq!(request.path, path);
        assert_eq!(request.source_files_generation, generation);
        assert_eq!(request.workspace_id, workspace_id);
        assert_eq!(request.pane_id, focused_pane);
        assert_eq!(request.terminal_id, focused_terminal);
        assert!(
            app.state.agent_reference_picker.is_none(),
            "activation closes the picker"
        );
    }
}
