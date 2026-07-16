const MAX_BUILT_IN_INSTANCES: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum BuiltInAppId {
    Terminal,
    Files,
}

impl BuiltInAppId {
    const fn index(self) -> usize {
        match self {
            Self::Terminal => 0,
            Self::Files => 1,
        }
    }
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
    InstanceGenerationExhausted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct StageState {
    instances: [Option<AppInstance>; MAX_BUILT_IN_INSTANCES],
    instance_count: usize,
    active: AppInstanceId,
    previous: Option<AppInstanceId>,
    last_generations: [Option<u32>; 2],
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

        let files_id = match self
            .instances()
            .find(|instance| instance.id.app == BuiltInAppId::Files)
            .map(|instance| instance.id)
        {
            Some(id) => id,
            None => self.next_instance_id(BuiltInAppId::Files)?,
        };
        if self.instance(files_id).is_none() {
            self.insert_instance(AppInstance::built_in(files_id))?;
        }

        self.previous = Some(self.active);
        self.active = files_id;
        Ok(())
    }

    pub(crate) fn close_files(&mut self) {
        let Some(files_index) = self
            .instances()
            .position(|instance| instance.id.app == BuiltInAppId::Files)
        else {
            return;
        };
        let Some(files_id) = self.instances[files_index].map(|instance| instance.id) else {
            return;
        };

        if self.active == files_id {
            let restore = self
                .previous
                .filter(|id| self.instance(*id).is_some())
                .or_else(|| {
                    self.instances()
                        .find(|instance| instance.id.app == BuiltInAppId::Terminal)
                        .map(|instance| instance.id)
                });
            let Some(restore) = restore else {
                return;
            };
            self.active = restore;
        }

        self.remove_instance_at(files_index);
        self.previous = None;
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
        let last_generation = &mut self.last_generations[instance.id.app.index()];
        *last_generation = Some(
            last_generation
                .map(|last| last.max(instance.id.generation))
                .unwrap_or(instance.id.generation),
        );
        Ok(())
    }

    fn remove_instance_at(&mut self, index: usize) {
        for slot in index..self.instance_count.saturating_sub(1) {
            self.instances[slot] = self.instances[slot + 1];
        }
        self.instance_count = self.instance_count.saturating_sub(1);
        self.instances[self.instance_count] = None;
    }

    fn next_instance_id(&self, app: BuiltInAppId) -> Result<AppInstanceId, StageStateError> {
        let generation = match self.last_generations[app.index()] {
            Some(last) => last
                .checked_add(1)
                .ok_or(StageStateError::InstanceGenerationExhausted)?,
            None => 0,
        };
        Ok(AppInstanceId { app, generation })
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
            last_generations: [Some(0), None],
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        app::{
            actions::FileManagerOpenError,
            state::{AppState, PaneFocusTarget},
        },
        layout::PaneId,
    };

    use super::{
        AppInstance, AppInstanceId, AppSurfaceRef, BuiltInAppId, StageState, StageStateError,
        MAX_BUILT_IN_INSTANCES,
    };

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
        let mut stage = StageState::default();
        stage.last_generations[BuiltInAppId::Files.index()] = Some(u32::MAX);
        let retained = stage;

        assert_eq!(
            stage.next_instance_id(BuiltInAppId::Files),
            Err(StageStateError::InstanceGenerationExhausted)
        );
        assert_eq!(stage, retained);
    }

    #[test]
    fn closing_files_restores_previous_terminal_surface() {
        let mut state = AppState::test_new();
        state.open_file_manager();
        assert_eq!(
            state.stage.active_surface(),
            Some(AppSurfaceRef::NativeFiles)
        );

        state.close_file_manager();

        assert_eq!(
            (state.stage.active_surface(), state.stage.previous_surface()),
            (Some(AppSurfaceRef::TerminalWorkspace), None)
        );
        assert!(
            state
                .stage
                .instances()
                .all(|instance| instance.id.app != BuiltInAppId::Files),
            "closed Files must not leave a resident instance"
        );
    }

    #[test]
    fn failed_files_open_restores_previous_surface_and_focus() {
        let mut state = AppState::test_new();
        state.previous_pane_focus = Some(PaneFocusTarget {
            workspace_id: "prior-workspace".to_string(),
            pane_id: PaneId::alloc(),
        });
        let retained_stage = state.stage;
        let retained_focus = state.previous_pane_focus.clone();

        assert_eq!(
            state.try_open_file_manager_with(|focus| {
                *focus = None;
                None
            }),
            Err(FileManagerOpenError::PreparationFailed)
        );
        assert_eq!(state.stage, retained_stage);
        assert_eq!(state.previous_pane_focus, retained_focus);
        assert!(state.file_manager.is_none());
    }
}
