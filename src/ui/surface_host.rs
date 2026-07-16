#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AppSurfaceRef {
    TerminalWorkspace,
    NativeFiles,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct StageState {
    active: AppSurfaceRef,
    previous: Option<AppSurfaceRef>,
}

impl StageState {
    #[allow(dead_code)] // SF4.2/SF4.3 consume the typed surface in input and render projection.
    pub(crate) const fn active_surface(&self) -> AppSurfaceRef {
        self.active
    }

    #[allow(dead_code)] // SF4.1 close/failure restoration consumes this after its RED contracts.
    pub(crate) const fn previous_surface(&self) -> Option<AppSurfaceRef> {
        self.previous
    }

    pub(crate) fn activate_files(&mut self) {
        self.previous = Some(self.active);
        self.active = AppSurfaceRef::NativeFiles;
    }
}

impl Default for StageState {
    fn default() -> Self {
        Self {
            active: AppSurfaceRef::TerminalWorkspace,
            previous: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::app::state::AppState;

    use super::{AppSurfaceRef, StageState};

    #[test]
    fn stage_starts_on_terminal_workspace() {
        let state = AppState::test_new();

        assert_eq!(
            state.stage.active_surface(),
            AppSurfaceRef::TerminalWorkspace
        );
    }

    #[test]
    fn activating_files_records_previous_surface() {
        let mut stage = StageState::default();

        stage.activate_files();

        assert_eq!(
            (stage.active_surface(), stage.previous_surface()),
            (
                AppSurfaceRef::NativeFiles,
                Some(AppSurfaceRef::TerminalWorkspace),
            )
        );
    }

    #[test]
    fn reactivating_singleton_files_keeps_one_surface() {
        let mut stage = StageState::default();
        stage.activate_files();
        let first_activation = stage;

        stage.activate_files();

        assert_eq!(stage, first_activation);
    }
}
