const MAX_BUILT_IN_INSTANCES: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum BuiltInAppId {
    Terminal,
    Files,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum AppSurfaceRef {
    TerminalWorkspace,
    NativeFiles,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct AppInstanceId {
    app: BuiltInAppId,
    generation: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AppInstance {
    id: AppInstanceId,
    surface: AppSurfaceRef,
}

impl AppInstance {
    const fn built_in(id: AppInstanceId) -> Self {
        let surface = match id.app {
            BuiltInAppId::Terminal => AppSurfaceRef::TerminalWorkspace,
            BuiltInAppId::Files => AppSurfaceRef::NativeFiles,
        };
        Self { id, surface }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StageStateError {
    BuiltInInstanceCapacityReached,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct StageState {
    instances: [Option<AppInstance>; MAX_BUILT_IN_INSTANCES],
    instance_count: usize,
    active: AppInstanceId,
    previous: Option<AppInstanceId>,
}

impl StageState {
    pub(crate) fn active_surface(&self) -> Option<AppSurfaceRef> {
        self.instance(self.active).map(|instance| instance.surface)
    }

    #[allow(dead_code)] // SF4.1 close/failure restoration consumes this after its RED contracts.
    pub(crate) fn previous_surface(&self) -> Option<AppSurfaceRef> {
        self.previous
            .and_then(|id| self.instance(id))
            .map(|instance| instance.surface)
    }

    pub(crate) fn activate_files(&mut self) -> Result<(), StageStateError> {
        if self.active_surface() == Some(AppSurfaceRef::NativeFiles) {
            return Ok(());
        }

        let files_id = self
            .instances()
            .find(|instance| instance.id.app == BuiltInAppId::Files)
            .map(|instance| instance.id)
            .unwrap_or(AppInstanceId {
                app: BuiltInAppId::Files,
                generation: 0,
            });
        if self.instance(files_id).is_none() {
            self.insert_instance(AppInstance::built_in(files_id))?;
        }

        self.previous = Some(self.active);
        self.active = files_id;
        Ok(())
    }

    fn instance(&self, id: AppInstanceId) -> Option<&AppInstance> {
        self.instances().find(|instance| instance.id == id)
    }

    fn instances(&self) -> impl Iterator<Item = &AppInstance> {
        self.instances[..self.instance_count]
            .iter()
            .filter_map(Option::as_ref)
    }

    fn insert_instance(&mut self, instance: AppInstance) -> Result<(), StageStateError> {
        if self.instance_count == MAX_BUILT_IN_INSTANCES {
            return Err(StageStateError::BuiltInInstanceCapacityReached);
        }
        self.instances[self.instance_count] = Some(instance);
        self.instance_count += 1;
        Ok(())
    }
}

impl Default for StageState {
    fn default() -> Self {
        let terminal_id = AppInstanceId {
            app: BuiltInAppId::Terminal,
            generation: 0,
        };
        let mut instances = [None; MAX_BUILT_IN_INSTANCES];
        instances[0] = Some(AppInstance::built_in(terminal_id));
        Self {
            instances,
            instance_count: 1,
            active: terminal_id,
            previous: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::app::state::AppState;

    use super::{
        AppInstance, AppInstanceId, AppSurfaceRef, BuiltInAppId, StageState, StageStateError,
        MAX_BUILT_IN_INSTANCES,
    };

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct TestGenerationAllocator {
        last: u32,
    }

    #[test]
    fn stage_starts_on_terminal_workspace() {
        let state = AppState::test_new();

        assert_eq!(
            state.stage.active_surface(),
            Some(AppSurfaceRef::TerminalWorkspace)
        );
    }

    #[test]
    fn activating_files_records_previous_surface() {
        let mut stage = StageState::default();

        stage.activate_files().expect("activate Files");

        assert_eq!(
            (stage.active_surface(), stage.previous_surface()),
            (
                Some(AppSurfaceRef::NativeFiles),
                Some(AppSurfaceRef::TerminalWorkspace),
            )
        );
    }

    #[test]
    fn reactivating_singleton_files_keeps_one_surface() {
        let mut stage = StageState::default();
        stage.activate_files().expect("first Files activation");
        let first_activation = stage;

        stage
            .activate_files()
            .expect("singleton Files reactivation");

        assert_eq!(stage, first_activation);
    }

    #[test]
    fn stage_rejects_more_than_sixteen_builtin_instances() {
        let mut stage = StageState::default();
        for generation in 1..MAX_BUILT_IN_INSTANCES as u32 {
            assert_eq!(
                stage.insert_instance(AppInstance::built_in(AppInstanceId {
                    app: BuiltInAppId::Files,
                    generation,
                })),
                Ok(())
            );
        }
        let retained = stage;

        assert_eq!(
            stage.insert_instance(AppInstance::built_in(AppInstanceId {
                app: BuiltInAppId::Files,
                generation: MAX_BUILT_IN_INSTANCES as u32,
            })),
            Err(StageStateError::BuiltInInstanceCapacityReached)
        );
        assert_eq!(stage, retained);
    }

    #[test]
    fn instance_generation_exhaustion_fails_without_aliasing() {
        let mut allocator = TestGenerationAllocator { last: u32::MAX };
        let retained = allocator;

        assert_eq!(
            allocate_next_generation_for_test(&mut allocator),
            Err("instance generation exhausted")
        );
        assert_eq!(allocator, retained);
    }

    fn allocate_next_generation_for_test(
        allocator: &mut TestGenerationAllocator,
    ) -> Result<u32, &'static str> {
        // RED-only seam: wrapping would alias generation zero after exhaustion.
        // SF4.1 replaces it with checked allocation before any state mutation.
        let generation = allocator.last.wrapping_add(1);
        allocator.last = generation;
        Ok(generation)
    }
}
