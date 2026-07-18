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
    /// Open the blocking agent picker for one supported, still-deliverable
    /// path. Opening performs no runtime work and prepares no delivery
    /// authority; with zero live agents the action stays a no-op.
    pub(super) fn open_agent_reference_picker(&mut self, paths: Vec<PathBuf>) -> bool {
        self.state.request_file_manager_agent_handoff = None;
        let Some(path) = paths.first().filter(|_| paths.len() == 1).cloned() else {
            return false;
        };
        if self.file_operation_worker.is_busy()
            || self
                .state
                .file_manager_operation
                .as_ref()
                .is_some_and(crate::app::state::FileManagerOperationState::is_running)
        {
            return false;
        }
        let Some(file_manager) = self.state.file_manager.as_ref() else {
            return false;
        };
        if !file_manager
            .entries
            .iter()
            .any(|entry| entry.operation_supported() && entry.path == path)
        {
            return false;
        }
        if !crate::app::file_agent_handoff::reference_path_is_deliverable(&path) {
            return false;
        }
        let rows = self.agent_reference_picker_rows();
        if rows.is_empty() {
            return false;
        }
        self.state.agent_reference_picker = Some(AgentReferencePickerState {
            source_path: path,
            source_files_generation: self
                .state
                .stage
                .active_instance_generation()
                .unwrap_or_default(),
            rows,
            selected: 0,
        });
        self.state
            .enter_overlay_mode(crate::app::state::Mode::AgentReferencePicker);
        true
    }

    /// Live agent targets projected from the agents panel: agent-classified
    /// terminals only, deduped per pane, with the focused agent first and
    /// marked current. Bounded `O(live agent panes)`.
    fn agent_reference_picker_rows(&self) -> Vec<AgentReferencePickerRow> {
        let focused = self.state.active.and_then(|workspace_idx| {
            self.state
                .workspaces
                .get(workspace_idx)
                .and_then(crate::workspace::Workspace::focused_pane_id)
                .map(|pane_id| (workspace_idx, pane_id))
        });
        let mut rows: Vec<AgentReferencePickerRow> = crate::ui::agent_panel_entries(&self.state)
            .into_iter()
            .filter_map(|entry| {
                let terminal_id = self
                    .state
                    .terminal_id_for_pane(entry.ws_idx, entry.pane_id)?;
                if !self
                    .state
                    .terminals
                    .get(&terminal_id)
                    .is_some_and(crate::terminal::TerminalState::is_agent_terminal)
                {
                    return None;
                }
                let workspace_id = self.state.workspaces.get(entry.ws_idx)?.id.clone();
                Some(AgentReferencePickerRow {
                    label: entry
                        .agent_label
                        .clone()
                        .unwrap_or_else(|| entry.primary_label.clone()),
                    is_current: focused == Some((entry.ws_idx, entry.pane_id)),
                    workspace_id,
                    pane_id: entry.pane_id,
                    terminal_id,
                    live: true,
                })
            })
            .collect();
        rows.sort_by_key(|row| !row.is_current);
        rows
    }

    /// Explicit picker activation: snapshot the FULL selected target
    /// identity into the typed request and close the picker. Delivery still
    /// happens at the App-owned send boundary on a later frame; a stale or
    /// non-live selection prepares nothing (fail closed).
    pub(crate) fn activate_agent_reference_picker_selection(&mut self) -> bool {
        let Some(picker) = self.state.agent_reference_picker.as_ref() else {
            return false;
        };
        let Some(row) = picker.rows.get(picker.selected).filter(|row| row.live) else {
            return false;
        };
        let row = row.clone();
        let source_path = picker.source_path.clone();
        let source_files_generation = picker.source_files_generation;
        let Some(workspace_idx) = self
            .state
            .workspaces
            .iter()
            .position(|workspace| workspace.id == row.workspace_id)
        else {
            return false;
        };
        if self
            .state
            .terminal_id_for_pane(workspace_idx, row.pane_id)
            .as_ref()
            != Some(&row.terminal_id)
            || !self
                .state
                .terminals
                .get(&row.terminal_id)
                .is_some_and(crate::terminal::TerminalState::is_agent_terminal)
            || !crate::app::file_agent_handoff::reference_path_is_deliverable(&source_path)
        {
            return false;
        }
        self.state.request_file_manager_agent_handoff = Some(AgentReferenceRequest {
            path: source_path,
            source_files_generation,
            workspace_id: row.workspace_id,
            pane_id: row.pane_id,
            terminal_id: row.terminal_id,
        });
        self.close_agent_reference_picker();
        true
    }

    /// Close the picker and restore the pre-overlay focus owner. Closing
    /// discards only client-local presentation state.
    pub(crate) fn close_agent_reference_picker(&mut self) {
        if self.state.agent_reference_picker.take().is_some() {
            crate::app::input::leave_modal(&mut self.state);
        }
    }

    /// Blocking-overlay key routing owner for the picker mode. The
    /// popup-ownership task wires full selection movement; Esc always
    /// closes with zero bytes.
    pub(crate) fn handle_agent_reference_picker_key(&mut self, key: crossterm::event::KeyEvent) {
        match key.code {
            crossterm::event::KeyCode::Esc => self.close_agent_reference_picker(),
            crossterm::event::KeyCode::Enter => {
                let _ = self.activate_agent_reference_picker_selection();
            }
            _ => {}
        }
    }
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
