use crate::app::state::{
    FileManagerContextMenuAction, FileManagerContextMenuModel, FileManagerOperationState,
    FileManagerRenameState, Mode,
};

impl crate::app::App {
    pub(super) fn open_file_manager_row_rename(&mut self, path: std::path::PathBuf) -> bool {
        let selection_is_bulk = self
            .state
            .file_manager
            .as_ref()
            .is_some_and(|file_manager| file_manager.multi_selection_paths().len() > 1);
        if selection_is_bulk {
            return false;
        }
        self.open_file_manager_rename(vec![path])
    }

    pub(super) fn open_file_manager_context_rename(
        &mut self,
        paths: Vec<std::path::PathBuf>,
    ) -> bool {
        let Some(file_manager) = self.state.file_manager.as_ref() else {
            return false;
        };
        let action_bar = crate::ui::compute_file_manager_action_bar_model(
            file_manager,
            &self.state.file_manager_clipboard,
            self.state
                .file_manager_operation
                .as_ref()
                .is_some_and(FileManagerOperationState::is_running),
        );
        let Some(model) =
            FileManagerContextMenuModel::from_action_bar_with_plugins(&action_bar, &[])
        else {
            return false;
        };
        let rename_is_current = model.paths == paths
            && model
                .items
                .iter()
                .any(|item| item.action == FileManagerContextMenuAction::Rename && item.enabled);
        if !rename_is_current {
            return false;
        }
        self.open_file_manager_rename(paths)
    }

    fn open_file_manager_rename(&mut self, paths: Vec<std::path::PathBuf>) -> bool {
        if paths.len() != 1
            || self.file_operation_worker.is_busy()
            || self
                .state
                .file_manager_operation
                .as_ref()
                .is_some_and(FileManagerOperationState::is_running)
        {
            return false;
        }
        let Some(path) = paths.first() else {
            return false;
        };
        let Some(file_manager) = self.state.file_manager.as_ref() else {
            return false;
        };
        let Some(entry) = file_manager
            .entries
            .iter()
            .find(|entry| entry.path == *path)
        else {
            return false;
        };
        if !entry.operation_supported {
            return false;
        }
        let Some(name) = path
            .file_name()
            .and_then(|name| name.to_str())
            .map(str::to_string)
        else {
            return false;
        };

        self.state.file_manager_rename = Some(FileManagerRenameState { paths });
        self.state.name_input = name;
        self.state.name_input_replace_on_type = true;
        self.state.mode = Mode::RenameFile;
        true
    }
}
