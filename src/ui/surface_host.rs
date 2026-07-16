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
        if self.active == AppSurfaceRef::NativeFiles {
            return;
        }
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

    #[derive(Default)]
    struct TestBuiltInInstances {
        generations: Vec<u32>,
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

    #[test]
    fn stage_rejects_more_than_sixteen_builtin_instances() {
        let mut instances = TestBuiltInInstances::default();
        for generation in 0..16 {
            assert_eq!(insert_instance_for_test(&mut instances, generation), Ok(()));
        }
        let retained = instances.generations.clone();

        assert_eq!(
            insert_instance_for_test(&mut instances, 16),
            Err("built-in instance capacity reached")
        );
        assert_eq!(instances.generations, retained);
    }

    fn insert_instance_for_test(
        instances: &mut TestBuiltInInstances,
        generation: u32,
    ) -> Result<(), &'static str> {
        // RED-only seam: SF4.1 replaces this unbounded push with the production
        // typed instance store and its fail-closed sixteen-instance limit.
        instances.generations.push(generation);
        Ok(())
    }
}
