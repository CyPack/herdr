#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AppSurfaceRef {
    TerminalWorkspace,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct StageState {
    active: AppSurfaceRef,
}

impl StageState {
    #[allow(dead_code)] // SF4.2/SF4.3 consume the typed surface in input and render projection.
    pub(crate) const fn active_surface(&self) -> AppSurfaceRef {
        self.active
    }
}

impl Default for StageState {
    fn default() -> Self {
        Self {
            active: AppSurfaceRef::TerminalWorkspace,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::app::state::AppState;

    use super::{AppSurfaceRef, StageState};

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum TestSurfaceRef {
        TerminalWorkspace,
        NativeFiles,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct TestStageActivation {
        active: TestSurfaceRef,
        previous: Option<TestSurfaceRef>,
    }

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
        let stage = StageState::default();

        let activated = activate_files_for_test(&stage);

        assert_eq!(
            (activated.active, activated.previous),
            (
                TestSurfaceRef::NativeFiles,
                Some(TestSurfaceRef::TerminalWorkspace),
            )
        );
    }

    fn activate_files_for_test(stage: &StageState) -> TestStageActivation {
        let current = match stage.active_surface() {
            AppSurfaceRef::TerminalWorkspace => TestSurfaceRef::TerminalWorkspace,
        };

        // RED-only seam: SF4.1 must atomically retain the displaced surface
        // and activate NativeFiles instead of leaving the current Stage inert.
        TestStageActivation {
            active: current,
            previous: None,
        }
    }
}
