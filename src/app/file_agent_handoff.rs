use crate::app::state::{
    FileManagerAgentHandoffRequest, FileManagerClaudeSplitRequest, FileManagerContextMenuAction,
    FileManagerOperationState,
};
use bytes::Bytes;

#[derive(Debug)]
struct OwnedFileManagerClaudeSplit {
    workspace_id: String,
    pane_id: crate::layout::PaneId,
    terminal_id: crate::terminal::TerminalId,
}

fn file_manager_claude_argv() -> [String; 1] {
    ["claude".to_string()]
}

pub(super) enum TerminalInputSendError {
    RuntimeUnavailable,
    SendFailed { message: String, busy: bool },
}

impl crate::app::App {
    #[cfg(test)]
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
        self.state.request_file_manager_claude_split = None;
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
        let Some(file_manager) = self.state.file_manager.as_ref() else {
            return false;
        };
        if !file_manager
            .entries
            .iter()
            .any(|entry| entry.operation_supported && entry.path == path)
        {
            return false;
        }
        let cwd = file_manager.cwd.clone();

        let Some(workspace_idx) = self.state.active else {
            return false;
        };
        let Some(workspace) = self.state.workspaces.get(workspace_idx) else {
            return false;
        };
        let workspace_id = workspace.id.clone();
        let Some(pane_id) = workspace.focused_pane_id() else {
            return false;
        };
        let Some(terminal_id) = self.state.terminal_id_for_pane(workspace_idx, pane_id) else {
            return false;
        };
        let is_agent = self
            .state
            .terminals
            .get(&terminal_id)
            .is_some_and(crate::terminal::TerminalState::is_agent_terminal);
        if is_agent {
            self.state.request_file_manager_agent_handoff =
                Some(FileManagerAgentHandoffRequest { path, terminal_id });
        } else if self.state.terminals.contains_key(&terminal_id) {
            self.state.request_file_manager_claude_split = Some(FileManagerClaudeSplitRequest {
                path,
                cwd,
                workspace_id,
                source_pane_id: pane_id,
                source_terminal_id: terminal_id,
            });
        } else {
            return false;
        }
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

    pub(super) fn try_send_terminal_input(
        &self,
        terminal_id: &crate::terminal::TerminalId,
        bytes: Bytes,
    ) -> Result<(), TerminalInputSendError> {
        let runtime = self
            .terminal_runtimes
            .get(terminal_id)
            .ok_or(TerminalInputSendError::RuntimeUnavailable)?;
        runtime.try_send_bytes(bytes).map_err(|error| {
            let busy = matches!(error, tokio::sync::mpsc::error::TrySendError::Full(_));
            TerminalInputSendError::SendFailed {
                message: error.to_string(),
                busy,
            }
        })
    }

    pub(super) fn sync_file_manager_agent_handoff_send(&mut self) -> bool {
        let Some(request) = self.state.request_file_manager_agent_handoff.take() else {
            return false;
        };
        if !self.file_manager_agent_handoff_is_current(&request) {
            self.show_file_manager_agent_handoff_failure("agent handoff authority changed");
            return true;
        }
        let Some(path) = request.path.to_str() else {
            self.show_file_manager_agent_handoff_failure("agent handoff authority changed");
            return true;
        };
        let mut payload = Vec::with_capacity(path.len() + 1);
        payload.extend_from_slice(path.as_bytes());
        payload.push(b'\r');

        match self.try_send_terminal_input(&request.terminal_id, Bytes::from(payload)) {
            Ok(()) => {}
            Err(TerminalInputSendError::RuntimeUnavailable) => {
                self.show_file_manager_agent_handoff_failure("agent runtime is unavailable");
            }
            Err(TerminalInputSendError::SendFailed { busy, .. }) => {
                let context = if busy {
                    "agent input is busy"
                } else {
                    "agent runtime is unavailable"
                };
                self.show_file_manager_agent_handoff_failure(context);
            }
        }
        true
    }

    pub(super) fn sync_file_manager_claude_split(&mut self) -> bool {
        let argv = file_manager_claude_argv();
        self.sync_file_manager_claude_split_with_argv(&argv)
    }

    fn sync_file_manager_claude_split_with_argv(&mut self, argv: &[String]) -> bool {
        let Some(request) = self.state.request_file_manager_claude_split.take() else {
            return false;
        };
        if !self.file_manager_claude_split_is_current(&request) {
            self.show_file_manager_agent_handoff_failure("agent handoff authority changed");
            return true;
        }

        match self.launch_file_manager_claude_split(&request, argv) {
            Ok(owned) => {
                let _ = self.complete_file_manager_claude_split(&request, owned);
            }
            Err(context) => self.show_file_manager_agent_handoff_failure(&context),
        }
        true
    }

    fn launch_file_manager_claude_split(
        &mut self,
        request: &FileManagerClaudeSplitRequest,
        argv: &[String],
    ) -> Result<OwnedFileManagerClaudeSplit, String> {
        if argv.is_empty() {
            return Err("Claude launch argv is empty".to_string());
        }
        let Some(workspace_idx) = self
            .state
            .workspaces
            .iter()
            .position(|workspace| workspace.id == request.workspace_id)
        else {
            return Err("agent handoff authority changed".to_string());
        };

        let split = self.spawn_agent_split(
            workspace_idx,
            request.source_pane_id,
            crate::api::schema::SplitDirection::Down,
            request.cwd.clone(),
            argv,
            Vec::new(),
            false,
        );
        let (_, _, pane_id) = split.map_err(|error| {
            let body = self.agent_start_error_body(error);
            format!("Claude could not be launched: {}", body.message)
        })?;

        let Some(terminal_id) = self.state.terminal_id_for_pane(workspace_idx, pane_id) else {
            self.rollback_file_manager_claude_split_pane(
                request,
                &request.workspace_id,
                pane_id,
                None,
            );
            return Err("Claude terminal setup failed".to_string());
        };
        let owned = OwnedFileManagerClaudeSplit {
            workspace_id: request.workspace_id.clone(),
            pane_id,
            terminal_id,
        };
        let agent_name = self.next_file_manager_claude_name();
        let Some(terminal) = self.state.terminals.get_mut(&owned.terminal_id) else {
            self.rollback_file_manager_claude_split(request, &owned);
            return Err("Claude terminal setup failed".to_string());
        };
        terminal.set_agent_name(agent_name.clone());
        terminal.set_manual_label(agent_name);
        self.state.mark_session_dirty();
        Ok(owned)
    }

    fn complete_file_manager_claude_split(
        &mut self,
        request: &FileManagerClaudeSplitRequest,
        owned: OwnedFileManagerClaudeSplit,
    ) -> bool {
        if !self.file_manager_claude_split_is_current(request)
            || !self.file_manager_claude_split_is_owned(&owned)
        {
            self.rollback_file_manager_claude_split(request, &owned);
            self.show_file_manager_agent_handoff_failure("agent handoff authority changed");
            return false;
        }
        let Some(path) = request.path.to_str() else {
            self.rollback_file_manager_claude_split(request, &owned);
            self.show_file_manager_agent_handoff_failure("agent handoff authority changed");
            return false;
        };
        let mut payload = Vec::with_capacity(path.len() + 1);
        payload.extend_from_slice(path.as_bytes());
        payload.push(b'\r');

        if let Err(error) = self.try_send_terminal_input(&owned.terminal_id, Bytes::from(payload)) {
            let context = match error {
                TerminalInputSendError::RuntimeUnavailable => "Claude runtime is unavailable",
                TerminalInputSendError::SendFailed { busy: true, .. } => "Claude input is busy",
                TerminalInputSendError::SendFailed { busy: false, .. } => {
                    "Claude runtime is unavailable"
                }
            };
            self.rollback_file_manager_claude_split(request, &owned);
            self.show_file_manager_agent_handoff_failure(context);
            return false;
        }

        let Some(workspace_idx) = self
            .state
            .workspaces
            .iter()
            .position(|workspace| workspace.id == owned.workspace_id)
        else {
            self.rollback_file_manager_claude_split(request, &owned);
            self.show_file_manager_agent_handoff_failure("agent handoff authority changed");
            return false;
        };
        self.state
            .focus_pane_in_workspace(workspace_idx, owned.pane_id);
        self.state.close_file_manager();
        self.state.settle_terminal_mode_after_focus();
        self.schedule_session_save();
        true
    }

    fn file_manager_claude_split_is_current(
        &self,
        request: &FileManagerClaudeSplitRequest,
    ) -> bool {
        if self.file_operation_worker.is_busy()
            || self
                .state
                .file_manager_operation
                .as_ref()
                .is_some_and(FileManagerOperationState::is_running)
            || request.path.to_str().is_none()
        {
            return false;
        }
        let Some(file_manager) = self.state.file_manager.as_ref() else {
            return false;
        };
        if file_manager.cwd != request.cwd
            || !file_manager
                .entries
                .iter()
                .any(|entry| entry.operation_supported && entry.path == request.path)
        {
            return false;
        }
        let Some(workspace_idx) = self.state.active else {
            return false;
        };
        let Some(workspace) = self.state.workspaces.get(workspace_idx) else {
            return false;
        };
        if workspace.id != request.workspace_id
            || workspace.focused_pane_id() != Some(request.source_pane_id)
            || self
                .state
                .terminal_id_for_pane(workspace_idx, request.source_pane_id)
                .as_ref()
                != Some(&request.source_terminal_id)
        {
            return false;
        }
        self.state
            .terminals
            .get(&request.source_terminal_id)
            .is_some_and(|terminal| !terminal.is_agent_terminal())
    }

    fn file_manager_claude_split_is_owned(&self, owned: &OwnedFileManagerClaudeSplit) -> bool {
        let Some(workspace_idx) = self
            .state
            .workspaces
            .iter()
            .position(|workspace| workspace.id == owned.workspace_id)
        else {
            return false;
        };
        self.state
            .terminal_id_for_pane(workspace_idx, owned.pane_id)
            .as_ref()
            == Some(&owned.terminal_id)
            && self
                .state
                .terminals
                .get(&owned.terminal_id)
                .is_some_and(crate::terminal::TerminalState::is_agent_terminal)
    }

    fn rollback_file_manager_claude_split(
        &mut self,
        request: &FileManagerClaudeSplitRequest,
        owned: &OwnedFileManagerClaudeSplit,
    ) {
        self.rollback_file_manager_claude_split_pane(
            request,
            &owned.workspace_id,
            owned.pane_id,
            Some(&owned.terminal_id),
        );
    }

    fn rollback_file_manager_claude_split_pane(
        &mut self,
        request: &FileManagerClaudeSplitRequest,
        workspace_id: &str,
        pane_id: crate::layout::PaneId,
        expected_terminal_id: Option<&crate::terminal::TerminalId>,
    ) {
        let Some(workspace_idx) = self
            .state
            .workspaces
            .iter()
            .position(|workspace| workspace.id == workspace_id)
        else {
            return;
        };
        let terminal_id = self.state.terminal_id_for_pane(workspace_idx, pane_id);
        if expected_terminal_id.is_some_and(|expected| terminal_id.as_ref() != Some(expected)) {
            return;
        }
        let Some(terminal_id) = terminal_id else {
            return;
        };
        let should_close_workspace = self.state.workspaces[workspace_idx].remove_pane(pane_id);
        if should_close_workspace {
            return;
        }
        self.state.remove_plugin_pane_records([pane_id]);
        self.state.remove_unattached_terminal_ids([terminal_id]);
        self.shutdown_detached_terminal_runtimes();
        if self
            .state
            .workspaces
            .get(workspace_idx)
            .and_then(|workspace| workspace.terminal_id(request.source_pane_id))
            == Some(&request.source_terminal_id)
        {
            self.state
                .focus_pane_in_workspace(workspace_idx, request.source_pane_id);
        }
        self.state.mark_session_dirty();
        self.schedule_session_save();
    }

    fn next_file_manager_claude_name(&self) -> String {
        let agents = self.collect_agent_infos();
        let base = "fm-claude";
        if agents
            .iter()
            .all(|agent| agent.name.as_deref() != Some(base))
        {
            return base.to_string();
        }
        for suffix in 2..=agents.len().saturating_add(2) {
            let candidate = format!("{base}-{suffix}");
            if agents
                .iter()
                .all(|agent| agent.name.as_deref() != Some(candidate.as_str()))
            {
                return candidate;
            }
        }
        format!("{base}-{}", agents.len().saturating_add(3))
    }

    fn file_manager_agent_handoff_is_current(
        &self,
        request: &FileManagerAgentHandoffRequest,
    ) -> bool {
        if self.file_operation_worker.is_busy()
            || self
                .state
                .file_manager_operation
                .as_ref()
                .is_some_and(FileManagerOperationState::is_running)
            || request.path.to_str().is_none()
            || !self
                .state
                .file_manager
                .as_ref()
                .is_some_and(|file_manager| {
                    file_manager
                        .entries
                        .iter()
                        .any(|entry| entry.operation_supported && entry.path == request.path)
                })
        {
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
        if self
            .state
            .terminal_id_for_pane(workspace_idx, pane_id)
            .as_ref()
            != Some(&request.terminal_id)
        {
            return false;
        }
        self.state
            .terminals
            .get(&request.terminal_id)
            .is_some_and(crate::terminal::TerminalState::is_agent_terminal)
    }

    fn show_file_manager_agent_handoff_failure(&mut self, context: &str) {
        let previous_toast = self.state.toast.clone();
        self.state.toast = Some(crate::app::state::ToastNotification {
            kind: crate::app::state::ToastKind::NeedsAttention,
            title: "send to agent failed".to_string(),
            context: context.to_string(),
            position: None,
            target: None,
        });
        self.sync_toast_deadline(previous_toast);
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;

    use crate::app::state::{FileManagerAgentHandoffRequest, FileManagerClaudeSplitRequest};

    struct HandoffFixture {
        root: std::path::PathBuf,
    }

    impl HandoffFixture {
        fn new(tag: &str) -> Self {
            let root = std::env::temp_dir().join(format!(
                "herdr-fm-agent-handoff-{}-{tag}",
                std::process::id()
            ));
            let _ = std::fs::remove_dir_all(&root);
            std::fs::create_dir_all(&root).expect("create handoff fixture root");
            Self { root }
        }

        fn file(&self, name: &str) -> std::path::PathBuf {
            let path = self.root.join(name);
            std::fs::write(&path, b"handoff").expect("write handoff fixture");
            path
        }
    }

    impl Drop for HandoffFixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    fn app_with_agent_handoff(
        root: &std::path::Path,
        channel_capacity: usize,
    ) -> (
        crate::app::App,
        crate::terminal::TerminalId,
        tokio::sync::mpsc::Receiver<Bytes>,
    ) {
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = crate::app::App::new(
            &crate::config::Config::default(),
            true,
            None,
            api_rx,
            crate::api::EventHub::default(),
        );
        let workspace = crate::workspace::Workspace::test_new("fm-agent-handoff-send");
        let pane_id = workspace.tabs[0].root_pane;
        let terminal_id = workspace
            .terminal_id(pane_id)
            .expect("handoff terminal id")
            .clone();
        app.state.workspaces = vec![workspace];
        app.state.ensure_test_terminals();
        app.state
            .terminals
            .get_mut(&terminal_id)
            .expect("handoff terminal state")
            .set_agent_name("handoff-target".into());
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.file_manager = Some(crate::fm::FmState::new(root));
        let (runtime, receiver) =
            crate::terminal::TerminalRuntime::test_with_channel_capacity(80, 24, channel_capacity);
        app.terminal_runtimes.insert(terminal_id.clone(), runtime);
        (app, terminal_id, receiver)
    }

    fn app_with_non_agent_handoff(
        root: &std::path::Path,
        with_neighbor: bool,
    ) -> (
        crate::app::App,
        crate::layout::PaneId,
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
        let mut workspace = crate::workspace::Workspace::test_new("fm-claude-split");
        let source_pane_id = workspace.tabs[0].root_pane;
        if with_neighbor {
            workspace.test_split(ratatui::layout::Direction::Horizontal);
            workspace.tabs[0].layout.focus_pane(source_pane_id);
        }
        let source_terminal_id = workspace
            .terminal_id(source_pane_id)
            .expect("source terminal identity")
            .clone();
        app.state.workspaces = vec![workspace];
        app.state.ensure_test_terminals();
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.file_manager = Some(crate::fm::FmState::new(root));
        (app, source_pane_id, source_terminal_id)
    }

    #[cfg(windows)]
    fn running_test_command() -> &'static str {
        "C:\\Windows\\System32\\more.com"
    }

    #[cfg(not(windows))]
    fn running_test_command() -> &'static str {
        "/bin/cat"
    }

    fn impossible_test_command() -> String {
        std::env::temp_dir()
            .join(format!(
                "herdr-command-that-does-not-exist-{}",
                std::process::id()
            ))
            .to_string_lossy()
            .into_owned()
    }

    fn prepare_claude_split(
        app: &mut crate::app::App,
        path: &std::path::Path,
    ) -> FileManagerClaudeSplitRequest {
        assert!(app.open_file_manager_row_agent_handoff(path.to_path_buf()));
        app.state
            .request_file_manager_claude_split
            .clone()
            .expect("typed Claude split authority")
    }

    fn shutdown_test_runtimes(app: &mut crate::app::App) {
        for (_, runtime) in app.terminal_runtimes.drain() {
            runtime.shutdown();
        }
    }

    // TP-C5-SPLIT: the production plan is one direct argv launch, never a
    // shell command. The safe test argv still proves Down placement, FM cwd,
    // exact launch metadata, focus transfer, and early-exit ownership cleanup.
    #[tokio::test]
    async fn claude_split_launches_one_owned_down_pane_and_early_exit_removes_only_it() {
        let fixture = HandoffFixture::new("claude-split-success");
        let path = fixture.file("selected.txt");
        let (mut app, source_pane_id, source_terminal_id) =
            app_with_non_agent_handoff(&fixture.root, false);
        let before_terminal_ids = app
            .state
            .terminals
            .keys()
            .cloned()
            .collect::<std::collections::HashSet<_>>();
        prepare_claude_split(&mut app, &path);

        assert_eq!(super::file_manager_claude_argv(), ["claude".to_string()]);
        assert!(
            app.sync_file_manager_claude_split_with_argv(&[running_test_command().to_string(),])
        );
        assert!(app.state.request_file_manager_claude_split.is_none());
        assert_eq!(app.state.workspaces[0].tabs[0].layout.pane_count(), 2);
        let new_pane_id = app.state.workspaces[0].tabs[0]
            .layout
            .pane_ids()
            .into_iter()
            .find(|pane_id| *pane_id != source_pane_id)
            .expect("one newly owned pane");
        let split = app.state.workspaces[0].tabs[0]
            .layout
            .splits(ratatui::layout::Rect::new(0, 0, 120, 40));
        assert_eq!(split.len(), 1);
        assert_eq!(split[0].direction, ratatui::layout::Direction::Vertical);
        assert_eq!(app.state.workspaces[0].focused_pane_id(), Some(new_pane_id));
        assert!(app.state.file_manager.is_none());

        let new_terminal_id = app.state.workspaces[0]
            .terminal_id(new_pane_id)
            .expect("new terminal identity")
            .clone();
        let new_terminal = app
            .state
            .terminals
            .get(&new_terminal_id)
            .expect("new terminal state");
        assert_eq!(new_terminal.cwd, fixture.root);
        assert_eq!(
            new_terminal.launch_argv,
            Some(vec![running_test_command().to_string()])
        );
        assert!(new_terminal.is_agent_terminal());

        app.handle_internal_event(crate::events::AppEvent::PaneDied {
            pane_id: new_pane_id,
        });

        assert_eq!(app.state.workspaces[0].tabs[0].layout.pane_count(), 1);
        assert_eq!(
            app.state.workspaces[0].focused_pane_id(),
            Some(source_pane_id)
        );
        assert_eq!(
            app.state.terminal_id_for_pane(0, source_pane_id).as_ref(),
            Some(&source_terminal_id)
        );
        assert_eq!(
            app.state
                .terminals
                .keys()
                .cloned()
                .collect::<std::collections::HashSet<_>>(),
            before_terminal_ids
        );
        assert!(app.terminal_runtimes.get(&new_terminal_id).is_none());
        shutdown_test_runtimes(&mut app);
    }

    // TP-C5-SPLIT: spawn failure is an exact no-op for all pre-existing pane,
    // terminal, runtime, focus, and FM state. The consumed request may be
    // explicitly prepared again and retried without duplicate panes.
    #[tokio::test]
    async fn claude_split_spawn_failure_rolls_back_exactly_and_retry_owns_one_pane() {
        let fixture = HandoffFixture::new("claude-split-retry");
        let path = fixture.file("selected.txt");
        let (mut app, source_pane_id, _) = app_with_non_agent_handoff(&fixture.root, true);
        let before_panes = app.state.workspaces[0].tabs[0].layout.pane_ids();
        let before_terminal_ids = app
            .state
            .terminals
            .keys()
            .cloned()
            .collect::<std::collections::HashSet<_>>();
        let before_runtime_count = app.terminal_runtimes.len();
        prepare_claude_split(&mut app, &path);

        assert!(app.sync_file_manager_claude_split_with_argv(&[impossible_test_command(),]));

        assert_eq!(
            app.state.workspaces[0].tabs[0].layout.pane_ids(),
            before_panes
        );
        assert_eq!(
            app.state.workspaces[0].focused_pane_id(),
            Some(source_pane_id)
        );
        assert_eq!(
            app.state
                .terminals
                .keys()
                .cloned()
                .collect::<std::collections::HashSet<_>>(),
            before_terminal_ids
        );
        assert_eq!(app.terminal_runtimes.len(), before_runtime_count);
        assert!(app.state.file_manager.is_some());
        assert_eq!(
            app.state.toast.as_ref().map(|toast| toast.title.as_str()),
            Some("send to agent failed")
        );

        prepare_claude_split(&mut app, &path);
        assert!(
            app.sync_file_manager_claude_split_with_argv(&[running_test_command().to_string(),])
        );
        assert_eq!(
            app.state.workspaces[0].tabs[0].layout.pane_count(),
            before_panes.len() + 1
        );
        shutdown_test_runtimes(&mut app);
    }

    // TP-C5-SPLIT: stale or cancelled authority produces zero spawn. A
    // post-spawn first-send failure removes only the split transaction's exact
    // pane/terminal identities and keeps unrelated existing panes usable.
    #[tokio::test]
    async fn claude_split_stale_cancel_and_first_send_failure_leave_no_partial_setup() {
        let fixture = HandoffFixture::new("claude-split-partial");
        let path = fixture.file("selected.txt");
        let (mut app, source_pane_id, _) = app_with_non_agent_handoff(&fixture.root, true);
        let before_panes = app.state.workspaces[0].tabs[0].layout.pane_ids();
        let before_terminal_ids = app
            .state
            .terminals
            .keys()
            .cloned()
            .collect::<std::collections::HashSet<_>>();

        prepare_claude_split(&mut app, &path);
        app.state.request_file_manager_claude_split = None;
        assert!(
            !app.sync_file_manager_claude_split_with_argv(&[running_test_command().to_string(),])
        );
        assert_eq!(
            app.state.workspaces[0].tabs[0].layout.pane_ids(),
            before_panes
        );

        prepare_claude_split(&mut app, &path);
        app.state
            .file_manager
            .as_mut()
            .expect("open FM")
            .entries
            .clear();
        assert!(
            app.sync_file_manager_claude_split_with_argv(&[running_test_command().to_string(),])
        );
        assert_eq!(
            app.state.workspaces[0].tabs[0].layout.pane_ids(),
            before_panes
        );

        app.state.file_manager = Some(crate::fm::FmState::new(&fixture.root));
        let request = prepare_claude_split(&mut app, &path);
        let owned = app
            .launch_file_manager_claude_split(&request, &[running_test_command().to_string()])
            .expect("safe split launch");
        let runtime = app
            .terminal_runtimes
            .remove(&owned.terminal_id)
            .expect("remove new runtime to force first-send failure");

        assert!(!app.complete_file_manager_claude_split(&request, owned));
        runtime.shutdown();
        assert_eq!(
            app.state.workspaces[0].tabs[0].layout.pane_ids(),
            before_panes
        );
        assert_eq!(
            app.state.workspaces[0].focused_pane_id(),
            Some(source_pane_id)
        );
        assert_eq!(
            app.state
                .terminals
                .keys()
                .cloned()
                .collect::<std::collections::HashSet<_>>(),
            before_terminal_ids
        );
        assert!(app.state.file_manager.is_some());
        shutdown_test_runtimes(&mut app);
    }

    // TP-C5-SEND: the handoff is one literal UTF-8 path followed by exactly
    // one terminal Enter. Shell metacharacters are data, never command syntax.
    #[tokio::test]
    async fn existing_agent_receives_one_literal_path_and_enter_exactly_once() {
        let fixture = HandoffFixture::new("literal");
        let path = fixture.file("space 'quote' $(touch nope) `echo` ünicode.txt");
        let (mut app, terminal_id, mut receiver) = app_with_agent_handoff(&fixture.root, 4);
        app.state.request_file_manager_agent_handoff = Some(FileManagerAgentHandoffRequest {
            path: path.clone(),
            terminal_id,
        });

        assert!(app.sync_file_manager_agent_handoff_send());
        assert!(app.state.request_file_manager_agent_handoff.is_none());
        let mut expected = path
            .to_str()
            .expect("UTF-8 fixture path")
            .as_bytes()
            .to_vec();
        expected.push(b'\r');
        assert_eq!(
            receiver.try_recv().expect("one literal handoff payload"),
            Bytes::from(expected)
        );
        assert!(receiver.try_recv().is_err(), "no second payload");
        assert!(!app.sync_file_manager_agent_handoff_send());
        assert!(receiver.try_recv().is_err(), "frame retry stays silent");
    }

    // TP-C5-SEND: authority is revalidated at send time. Lost agent identity
    // consumes the stale request without writing any bytes.
    #[tokio::test]
    async fn existing_agent_handoff_fails_closed_after_agent_identity_is_lost() {
        let fixture = HandoffFixture::new("lost-agent");
        let path = fixture.file("selected.txt");
        let (mut app, terminal_id, mut receiver) = app_with_agent_handoff(&fixture.root, 2);
        app.state.request_file_manager_agent_handoff = Some(FileManagerAgentHandoffRequest {
            path,
            terminal_id: terminal_id.clone(),
        });
        app.state
            .terminals
            .get_mut(&terminal_id)
            .expect("handoff terminal state")
            .clear_agent_name();

        assert!(app.sync_file_manager_agent_handoff_send());
        assert!(app.state.request_file_manager_agent_handoff.is_none());
        assert!(receiver.try_recv().is_err());
        let toast = app.state.toast.as_ref().expect("visible authority failure");
        assert_eq!(toast.kind, crate::app::state::ToastKind::NeedsAttention);
        assert_eq!(toast.title, "send to agent failed");
        assert_eq!(toast.context, "agent handoff authority changed");
    }

    // TP-C5-SEND: a watcher-reconciled stale entry and a vanished runtime are
    // independently fail-closed and visible; neither can route to another pane.
    #[tokio::test]
    async fn existing_agent_handoff_rejects_stale_path_and_missing_runtime() {
        let fixture = HandoffFixture::new("stale-path");
        let path = fixture.file("selected.txt");
        let (mut stale, terminal_id, mut stale_receiver) = app_with_agent_handoff(&fixture.root, 2);
        stale.state.request_file_manager_agent_handoff = Some(FileManagerAgentHandoffRequest {
            path: path.clone(),
            terminal_id,
        });
        stale
            .state
            .file_manager
            .as_mut()
            .expect("open handoff FM")
            .entries
            .clear();

        assert!(stale.sync_file_manager_agent_handoff_send());
        assert!(stale_receiver.try_recv().is_err());
        assert_eq!(
            stale
                .state
                .toast
                .as_ref()
                .map(|toast| toast.context.as_str()),
            Some("agent handoff authority changed")
        );

        let (mut missing, terminal_id, mut missing_receiver) =
            app_with_agent_handoff(&fixture.root, 2);
        missing.state.request_file_manager_agent_handoff = Some(FileManagerAgentHandoffRequest {
            path,
            terminal_id: terminal_id.clone(),
        });
        let runtime = missing
            .terminal_runtimes
            .remove(&terminal_id)
            .expect("remove handoff runtime");

        assert!(missing.sync_file_manager_agent_handoff_send());
        assert!(missing_receiver.try_recv().is_err());
        assert_eq!(
            missing
                .state
                .toast
                .as_ref()
                .map(|toast| toast.context.as_str()),
            Some("agent runtime is unavailable")
        );
        runtime.shutdown();
    }

    // TP-C5-SEND: a full terminal input lane is a terminal failure for this
    // one-shot request. It is not retried on later frame ticks.
    #[tokio::test]
    async fn existing_agent_handoff_backpressure_is_consumed_without_hot_retry() {
        let fixture = HandoffFixture::new("backpressure");
        let path = fixture.file("selected.txt");
        let (mut app, terminal_id, mut receiver) = app_with_agent_handoff(&fixture.root, 1);
        app.terminal_runtimes
            .get(&terminal_id)
            .expect("handoff runtime")
            .try_send_bytes(Bytes::from_static(b"occupied"))
            .expect("fill handoff input lane");
        app.state.request_file_manager_agent_handoff =
            Some(FileManagerAgentHandoffRequest { path, terminal_id });

        assert!(app.sync_file_manager_agent_handoff_send());
        assert!(app.state.request_file_manager_agent_handoff.is_none());
        assert_eq!(
            receiver.try_recv().expect("pre-existing lane payload"),
            Bytes::from_static(b"occupied")
        );
        assert!(!app.sync_file_manager_agent_handoff_send());
        assert!(
            receiver.try_recv().is_err(),
            "failed handoff is not retried"
        );
        let toast = app.state.toast.as_ref().expect("visible send failure");
        assert_eq!(toast.kind, crate::app::state::ToastKind::NeedsAttention);
        assert_eq!(toast.title, "send to agent failed");
        assert_eq!(toast.context, "agent input is busy");
    }
}
