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

    pub(crate) const fn definition(self) -> AppDefinition {
        AppDefinition {
            id: self,
            launch: LaunchPolicy::Singleton,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LaunchPolicy {
    Singleton,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AppDefinition {
    pub(crate) id: BuiltInAppId,
    pub(crate) launch: LaunchPolicy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum AppSurfaceRef {
    TerminalWorkspace,
    NativeFiles,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StageSurfaceView {
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

    /// Project the active Stage surface for render dispatch. The projection is
    /// a pure read of typed Stage state: it can never own, create, resize, or
    /// destroy terminal runtime state.
    #[allow(dead_code)] // SF6 Files Stage rendering migration consumes this typed projection.
    pub(crate) fn surface_view(&self) -> StageSurfaceView {
        match self.active_surface() {
            Some(AppSurfaceRef::NativeFiles) => StageSurfaceView::NativeFiles,
            Some(AppSurfaceRef::TerminalWorkspace) | None => StageSurfaceView::TerminalWorkspace,
        }
    }

    pub(crate) fn activate_files(&mut self) -> Result<(), StageStateError> {
        let definition = BuiltInAppId::Files.definition();
        match definition.launch {
            LaunchPolicy::Singleton => {
                if self.active_surface() == Some(AppSurfaceRef::NativeFiles) {
                    return Ok(());
                }
            }
        }

        let files_id = match self
            .instances()
            .find(|instance| instance.id.app == definition.id)
            .map(|instance| instance.id)
        {
            Some(id) => id,
            None => self.next_instance_id(definition.id)?,
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
        terminal::{TerminalRuntime, TerminalRuntimeRegistry},
        workspace::Workspace,
    };

    use super::{
        AppInstance, AppInstanceId, AppSurfaceRef, BuiltInAppId, LaunchPolicy, StageState,
        StageStateError, StageSurfaceView, MAX_BUILT_IN_INSTANCES,
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

    // SF4.3-01: exactly ONE stage surface owns projected hit geometry per
    // frame. Terminal pane/split geometry exists only while the
    // TerminalWorkspace surface is active; Files geometry exists only while
    // NativeFiles is active. Today `compute_pane_infos` carries no surface
    // guard, so a hidden terminal keeps projecting pane hit rectangles (and
    // runtime resize side effects) underneath the Files surface.
    #[test]
    fn active_surface_alone_populates_stage_hits() {
        struct FixtureRoot(std::path::PathBuf);

        impl Drop for FixtureRoot {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }

        let root = std::env::temp_dir().join(format!(
            "herdr-stage-hits-{}",
            std::process::id(),
        ));
        let _fixture_root = FixtureRoot(root.clone());
        std::fs::create_dir_all(&root).expect("create stage hits fixture root");
        std::fs::write(root.join("00.txt"), b"x").expect("fixture entry");

        let mut state = AppState::test_new();
        let mut workspace = Workspace::test_new("stage-hits");
        workspace.test_split(ratatui::layout::Direction::Horizontal);
        state.workspaces = vec![workspace];
        state.active = Some(0);
        state.selected = 0;
        state.mobile_width_threshold = 0;
        let area = ratatui::layout::Rect::new(0, 0, 80, 24);

        // Control: the active TerminalWorkspace surface owns pane and split
        // geometry, and no Files geometry exists.
        crate::ui::compute_view(&mut state, area);
        assert_eq!(
            state.stage.surface_view(),
            StageSurfaceView::TerminalWorkspace
        );
        assert!(
            !state.view.pane_infos.is_empty(),
            "control: the active terminal surface must project pane hits"
        );
        assert!(
            !state.view.split_borders.is_empty(),
            "control: the split workspace must project split borders"
        );
        assert!(state.view.file_manager_row_areas.is_empty());
        assert!(state.view.file_manager_header_action_areas.is_empty());

        // With NativeFiles active, ONLY Files geometry may exist: the hidden
        // terminal projects no pane hits, no split borders.
        state
            .try_open_file_manager_with(|_| Some(crate::fm::FmState::new(&root)))
            .expect("Files activation");
        crate::ui::compute_view(&mut state, area);
        assert_eq!(state.stage.surface_view(), StageSurfaceView::NativeFiles);
        assert!(
            !state.view.file_manager_row_areas.is_empty(),
            "the active Files surface must project its row geometry"
        );
        assert!(
            !state.view.file_manager_header_action_areas.is_empty(),
            "the active Files surface must project its header actions"
        );
        assert!(
            state.view.pane_infos.is_empty(),
            "a hidden terminal surface must project no pane hit geometry"
        );
        assert!(
            state.view.split_borders.is_empty(),
            "a hidden terminal surface must project no split borders"
        );

        // Returning to the terminal surface restores its geometry and clears
        // the Files geometry in the same frame.
        state.close_file_manager();
        crate::ui::compute_view(&mut state, area);
        assert_eq!(
            state.stage.surface_view(),
            StageSurfaceView::TerminalWorkspace
        );
        assert!(!state.view.pane_infos.is_empty());
        assert!(!state.view.split_borders.is_empty());
        assert!(state.view.file_manager_row_areas.is_empty());
        assert!(state.view.file_manager_header_action_areas.is_empty());
    }

    #[tokio::test]
    async fn stage_surface_switch_does_not_destroy_terminal_runtime() {
        use std::sync::atomic::{AtomicU64, Ordering};

        struct FixtureRoot(std::path::PathBuf);

        impl Drop for FixtureRoot {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }

        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "herdr-stage-runtime-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let _fixture_root = FixtureRoot(root.clone());
        std::fs::create_dir_all(&root).expect("create stage runtime fixture root");

        let mut state = AppState::test_new();
        let workspace = Workspace::test_new("one");
        let pane_id = workspace.tabs[0].root_pane;
        let terminal_id = workspace
            .terminal_id(pane_id)
            .expect("root pane terminal identity")
            .clone();
        state.workspaces = vec![workspace];
        state.active = Some(0);
        state.selected = 0;

        let mut terminal_runtimes = TerminalRuntimeRegistry::new();
        assert!(
            terminal_runtimes
                .insert(
                    terminal_id.clone(),
                    TerminalRuntime::test_with_screen_bytes(100, 30, b"RUNTIME_BEHIND_STAGE"),
                )
                .is_none(),
            "fixture inserts exactly one runtime"
        );
        let runtime_count = terminal_runtimes.len();

        assert_eq!(
            state.stage.surface_view(),
            StageSurfaceView::TerminalWorkspace
        );
        assert_eq!(
            BuiltInAppId::Terminal.definition().launch,
            LaunchPolicy::Singleton
        );
        assert_eq!(
            BuiltInAppId::Files.definition().launch,
            LaunchPolicy::Singleton
        );

        state
            .try_open_file_manager_with(|_| Some(crate::fm::FmState::new(&root)))
            .expect("Files activation");
        let stage_after_open = state.stage;
        assert_eq!(state.stage.surface_view(), StageSurfaceView::NativeFiles);
        assert_eq!(terminal_runtimes.len(), runtime_count);
        assert!(terminal_runtimes.get(&terminal_id).is_some());

        state
            .try_open_file_manager_with(|_| Some(crate::fm::FmState::new(&root)))
            .expect("singleton Files reactivation");
        assert_eq!(state.stage, stage_after_open);
        assert_eq!(terminal_runtimes.len(), runtime_count);
        assert!(terminal_runtimes.get(&terminal_id).is_some());

        state.close_file_manager();
        assert_eq!(
            state.stage.surface_view(),
            StageSurfaceView::TerminalWorkspace
        );
        assert_eq!(terminal_runtimes.len(), runtime_count);
        terminal_runtimes
            .get(&terminal_id)
            .expect("closing Files must keep the original terminal runtime")
            .test_process_pty_bytes(b"still-usable");

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
        assert_eq!(
            state.stage.surface_view(),
            StageSurfaceView::TerminalWorkspace
        );
        assert_eq!(terminal_runtimes.len(), runtime_count);
        assert!(terminal_runtimes.get(&terminal_id).is_some());

        assert_eq!(state.workspaces.len(), 1);
        assert_eq!(state.workspaces[0].tabs[0].root_pane, pane_id);
        assert_eq!(
            state.workspaces[0].terminal_id(pane_id),
            Some(&terminal_id),
            "stage lifecycle must not rebind pane/terminal identity"
        );
    }
}
