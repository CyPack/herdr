use crate::app::state::{
    FileManagerAgentHandoffRequest, FileManagerContextMenuAction, FileManagerOperationState,
};

impl crate::app::App {
    pub(super) fn open_file_manager_row_agent_handoff(&mut self, path: std::path::PathBuf) -> bool {
        let selection_is_bulk = self
            .state
            .file_manager
            .as_ref()
            .is_some_and(|file_manager| file_manager.multi_selection_paths().len() > 1);
        if selection_is_bulk {
            self.state.request_file_manager_agent_handoff = None;
            return false;
        }
        self.prepare_file_manager_agent_handoff(vec![path])
    }

    fn open_file_manager_context_agent_handoff(&mut self, paths: Vec<std::path::PathBuf>) -> bool {
        // C3 already emitted this typed path only after comparing its menu
        // snapshot with the current action model. Revalidate live entry,
        // operation, and agent identity below without turning cursor focus
        // into a second authority source.
        self.prepare_file_manager_agent_handoff(paths)
    }

    fn prepare_file_manager_agent_handoff(&mut self, paths: Vec<std::path::PathBuf>) -> bool {
        self.state.request_file_manager_agent_handoff = None;
        let Some(path) = paths.first().filter(|_| paths.len() == 1).cloned() else {
            return false;
        };
        if path.to_str().is_none()
            || self.file_operation_worker.is_busy()
            || self
                .state
                .file_manager_operation
                .as_ref()
                .is_some_and(FileManagerOperationState::is_running)
        {
            return false;
        }
        let path_is_current = self
            .state
            .file_manager
            .as_ref()
            .is_some_and(|file_manager| {
                file_manager
                    .entries
                    .iter()
                    .any(|entry| entry.operation_supported && entry.path == path)
            });
        if !path_is_current {
            return false;
        }

        let Some(workspace_idx) = self.state.active else {
            return false;
        };
        let Some(pane_id) = self
            .state
            .workspaces
            .get(workspace_idx)
            .and_then(crate::workspace::Workspace::focused_pane_id)
        else {
            return false;
        };
        let Some(terminal_id) = self.state.terminal_id_for_pane(workspace_idx, pane_id) else {
            return false;
        };
        if !self
            .state
            .terminals
            .get(&terminal_id)
            .is_some_and(crate::terminal::TerminalState::is_agent_terminal)
        {
            return false;
        }

        self.state.request_file_manager_agent_handoff =
            Some(FileManagerAgentHandoffRequest { path, terminal_id });
        true
    }

    pub(super) fn sync_file_manager_agent_handoff(&mut self) -> bool {
        let is_send_agent = self
            .state
            .request_file_manager_context_action
            .as_ref()
            .is_some_and(|intent| intent.action == FileManagerContextMenuAction::SendAgent);
        if !is_send_agent {
            return false;
        }
        let Some(intent) = self.state.request_file_manager_context_action.take() else {
            return false;
        };
        let _ = self.open_file_manager_context_agent_handoff(intent.paths);
        true
    }
}
