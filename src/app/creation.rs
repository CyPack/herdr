use std::path::PathBuf;

use crate::api::schema::{EventData, EventEnvelope, EventKind};
#[cfg(test)]
use tracing::error;

use super::{
    api_helpers::{pane_agent_status, tab_attention_priority},
    App, Mode,
};
use crate::{config::NewTerminalCwdConfig, workspace::Workspace};

pub(crate) fn resolve_new_terminal_cwd(
    policy: &NewTerminalCwdConfig,
    follow_cwd: Option<PathBuf>,
) -> PathBuf {
    match policy {
        NewTerminalCwdConfig::Follow => follow_cwd
            .or_else(|| std::env::var_os("HOME").map(PathBuf::from))
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_else(|| PathBuf::from("/")),
        NewTerminalCwdConfig::Home => std::env::var_os("HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_else(|| PathBuf::from("/")),
        NewTerminalCwdConfig::Current => {
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"))
        }
        NewTerminalCwdConfig::Path(path) => crate::worktree::expand_tilde_path(path),
    }
}

pub(super) fn launch_cwd_for_terminal(
    terminal_id: &crate::terminal::TerminalId,
    terminals: &std::collections::HashMap<
        crate::terminal::TerminalId,
        crate::terminal::TerminalState,
    >,
    terminal_runtimes: &crate::terminal::TerminalRuntimeRegistry,
) -> Option<PathBuf> {
    terminal_runtimes
        .get(terminal_id)
        .and_then(|runtime| runtime.follow_cwd())
        .or_else(|| {
            terminals
                .get(terminal_id)
                .map(|terminal| terminal.cwd.clone())
        })
}

impl App {
    pub(super) fn seed_cwd_from_workspace(&self, ws_idx: usize) -> Option<PathBuf> {
        self.state
            .workspaces
            .get(ws_idx)?
            .resolved_identity_cwd_from(&self.state.terminals, &self.terminal_runtimes)
    }

    pub(super) fn launch_cwd_for_pane_in_workspace(
        &self,
        ws_idx: usize,
        pane_id: crate::layout::PaneId,
    ) -> Option<PathBuf> {
        let workspace = self.state.workspaces.get(ws_idx)?;
        let tab = workspace
            .tabs
            .get(workspace.find_tab_index_for_pane(pane_id)?)?;
        launch_cwd_for_terminal(
            tab.terminal_id(pane_id)?,
            &self.state.terminals,
            &self.terminal_runtimes,
        )
    }

    pub(super) fn focused_pane_cwd_in_workspace(&self, ws_idx: usize) -> Option<PathBuf> {
        let pane_id = self.state.workspaces.get(ws_idx)?.focused_pane_id()?;
        self.launch_cwd_for_pane_in_workspace(ws_idx, pane_id)
    }

    pub(super) fn resolve_new_terminal_cwd(&self, follow_cwd: Option<PathBuf>) -> PathBuf {
        resolve_new_terminal_cwd(&self.state.new_terminal_cwd, follow_cwd)
    }

    pub(super) fn workspace_creation_source(&self) -> Option<usize> {
        if self.state.mode == Mode::Navigate
            && self.state.workspaces.get(self.state.selected).is_some()
        {
            return Some(self.state.selected);
        }

        self.state.active.or_else(|| {
            self.state
                .workspaces
                .get(self.state.selected)
                .map(|_| self.state.selected)
        })
    }

    pub(super) fn begin_tui_workspace_create(&mut self, request_id: &'static str) {
        if self.state.prompt_new_workspace_name {
            let follow_cwd = self.workspace_creation_source().and_then(|ws_idx| {
                self.focused_pane_cwd_in_workspace(ws_idx)
                    .or_else(|| self.seed_cwd_from_workspace(ws_idx))
            });
            let cwd = self.resolve_new_terminal_cwd(follow_cwd);
            super::input::open_new_workspace_dialog(&mut self.state, cwd);
            return;
        }

        self.tui_new_workspace(request_id);
        self.state.mode = if self.state.active.is_some() {
            Mode::Terminal
        } else {
            Mode::Navigate
        };
    }

    /// The TUI's own "new workspace" intent — every road to it ends here.
    ///
    /// TP-DAILY-17: the rule that a second unnamed workspace in the daily
    /// directory becomes a TAB rather than a workspace belongs to this layer
    /// and NOT to `workspace.create`. A plugin or script that asks the API for
    /// a workspace must get a workspace; handing it a tab back would break the
    /// contract silently, and the response type says `workspace_created`. What
    /// the person pressed, on the other hand, was "give me somewhere new to
    /// work" — and in a directory that already has an unnamed workspace, a new
    /// tab in it IS somewhere new to work.
    ///
    /// Both TUI roads (the mouse/key affordance and the `request_new_workspace`
    /// loop) call this one function. Menu verbs with two bodies are how #91
    /// shipped an affordance that worked in every test and did nothing in the
    /// product; one body cannot drift from itself.
    pub(crate) fn tui_new_workspace(&mut self, request_id: &'static str) {
        // Only when the new workspace would have landed in the daily directory
        // anyway. A "new workspace" pressed inside a repository must still make
        // a workspace in that repository — breaking that would be a far larger
        // defect than the one being fixed.
        let target_cwd = self.resolve_new_terminal_cwd(
            self.workspace_creation_source()
                .and_then(|source| self.focused_pane_cwd_in_workspace(source)),
        );
        if let Some(ws_idx) = self.state.daily_adoption_target(&target_cwd) {
            let workspace_id = self.state.workspaces.get(ws_idx).map(|ws| ws.id.clone());
            if let Some(workspace_id) = workspace_id {
                self.runtime_tab_create(
                    request_id,
                    crate::api::schema::TabCreateParams {
                        workspace_id: Some(workspace_id),
                        cwd: None,
                        // Focused: the person asked for somewhere new to work,
                        // so landing them nowhere is the one outcome worse than
                        // the duplicate row this replaces.
                        focus: true,
                        label: None,
                        env: Default::default(),
                    },
                );
                return;
            }
        }

        self.runtime_workspace_create(
            request_id,
            crate::api::schema::WorkspaceCreateParams {
                cwd: None,
                focus: true,
                label: None,
                env: Default::default(),
            },
        );
    }

    /// Fold every interchangeable daily workspace into one.
    ///
    /// TP-DAILY-19: TP-DAILY-17 stopped new ones being born and TP-DAILY-18
    /// stopped the existing ones filling the sidebar, but neither removed them
    /// — folding a row away is hiding it, and the person asked how to get rid
    /// of them. This is the verb that does.
    ///
    /// Nothing is closed and nothing is killed. Each pane is moved into a new
    /// tab of the target workspace by `pane.move`, which already keeps the
    /// terminal alive, aliases the pane's public id so a plugin holding the old
    /// one still resolves it, and drops the source workspace by itself once its
    /// last pane leaves. So the work survives and the empty shells go, without
    /// this function ever closing anything.
    ///
    /// The public pane ids are collected BEFORE the first move on purpose:
    /// `handle_pane_move` removes an emptied workspace from the list, which
    /// shifts every index after it. Indices gathered up front would address the
    /// wrong workspace by the second move; public ids do not move.
    pub(crate) fn merge_daily_workspaces(&mut self, request_id: &'static str) {
        let mergeable = self.state.mergeable_daily_workspaces();
        if mergeable.len() < 2 {
            return;
        }
        let Some(target_ws_idx) = self.state.daily_merge_target() else {
            return;
        };
        let Some(target_workspace_id) = self
            .state
            .workspaces
            .get(target_ws_idx)
            .map(|ws| ws.id.clone())
        else {
            return;
        };

        let mut sources: Vec<String> = Vec::new();
        for ws_idx in mergeable {
            if ws_idx == target_ws_idx {
                continue;
            }
            let Some(workspace) = self.state.workspaces.get(ws_idx) else {
                continue;
            };
            let pane_ids: Vec<crate::layout::PaneId> = workspace
                .tabs
                .iter()
                .flat_map(|tab| tab.layout.pane_ids())
                .collect();
            for pane_id in pane_ids {
                if let Some(public_id) = self.public_pane_id(ws_idx, pane_id) {
                    sources.push(public_id);
                }
            }
        }

        for pane_id in sources {
            self.runtime_pane_move(
                request_id,
                crate::api::schema::PaneMoveParams {
                    pane_id,
                    destination: crate::api::schema::PaneMoveDestination::NewTab {
                        workspace_id: Some(target_workspace_id.clone()),
                        label: None,
                    },
                    // Not focused: a merge of fifteen panes that stole focus
                    // fifteen times would throw the screen around once per
                    // move. The person stays where they already were, which
                    // `daily_merge_target` has already made the destination.
                    focus: false,
                },
            );
        }
    }

    /// Create a workspace with a real PTY (needs event_tx).
    #[cfg(test)]
    pub(crate) fn create_workspace(&mut self) {
        let follow_cwd = self.workspace_creation_source().and_then(|ws_idx| {
            self.focused_pane_cwd_in_workspace(ws_idx)
                .or_else(|| self.seed_cwd_from_workspace(ws_idx))
        });
        let initial_cwd = self.resolve_new_terminal_cwd(follow_cwd);
        if let Err(e) = self.create_workspace_with_events(initial_cwd, true) {
            error!(err = %e, "failed to create workspace");
            self.state.mode = Mode::Navigate;
        }
    }

    #[cfg(test)]
    pub(crate) fn create_tab(&mut self) {
        let custom_name = self.state.requested_new_tab_name.take();
        let active_before = self.state.active;
        let follow_cwd = self.state.active.and_then(|ws_idx| {
            self.focused_pane_cwd_in_workspace(ws_idx)
                .or_else(|| self.seed_cwd_from_workspace(ws_idx))
        });
        let initial_cwd = self.resolve_new_terminal_cwd(follow_cwd);
        match self.create_tab_with_options(initial_cwd, true) {
            Ok(created_idx) => {
                let created_workspace = active_before.is_none();
                let ws_idx = if created_workspace {
                    Some(created_idx)
                } else {
                    self.state.active
                };
                let tab_idx = if created_workspace { 0 } else { created_idx };
                if let Some(name) = custom_name {
                    if let Some(ws) =
                        ws_idx.and_then(|ws_idx| self.state.workspaces.get_mut(ws_idx))
                    {
                        if let Some(tab) = ws.tabs.get_mut(tab_idx) {
                            tab.set_custom_name(name);
                        }
                        self.schedule_session_save();
                    }
                }
                if let Some(ws_idx) = ws_idx {
                    if created_workspace {
                        self.emit_workspace_open_events(ws_idx);
                    } else {
                        self.emit_tab_created_events(ws_idx, tab_idx);
                    }
                }
            }
            Err(e) => {
                error!(err = %e, "failed to create tab");
            }
        }
    }

    #[cfg(test)]
    pub(super) fn create_tab_with_options(
        &mut self,
        initial_cwd: PathBuf,
        focus: bool,
    ) -> std::io::Result<usize> {
        let Some(ws_idx) = self.state.active else {
            return self.create_workspace_with_options(initial_cwd, focus);
        };
        let (rows, cols) = self.state.estimate_pane_size();
        let ws = &mut self.state.workspaces[ws_idx];
        let (idx, terminal, runtime) = ws.create_tab(
            rows,
            cols,
            initial_cwd,
            self.state.pane_scrollback_limit_bytes,
            self.state.host_terminal_theme,
            self.state.host_terminal_appearance,
            crate::pane::PaneShellConfig::new(&self.state.default_shell, self.state.shell_mode),
            Vec::new(),
        )?;
        let root_pane = ws.tabs[idx].root_pane;
        self.terminal_runtimes.insert(terminal.id.clone(), runtime);
        self.state.terminals.insert(terminal.id.clone(), terminal);
        self.state.remove_alias_shadowed_by_new_pane(root_pane);
        if focus {
            self.state.switch_workspace_tab(ws_idx, idx);
            self.state.mode = Mode::Terminal;
        }
        let workspace_id = self.state.workspaces[ws_idx].id.clone();
        let tab_id = self
            .public_tab_id(ws_idx, idx)
            .unwrap_or_else(|| crate::workspace::public_tab_id_for_number(&workspace_id, idx + 1));
        let root_pane = self.state.workspaces[ws_idx].tabs[idx].root_pane.raw();
        crate::logging::tab_created(&workspace_id, &tab_id, root_pane);
        self.schedule_session_save();
        Ok(idx)
    }

    pub(crate) fn create_workspace_with_options(
        &mut self,
        initial_cwd: PathBuf,
        focus: bool,
    ) -> std::io::Result<usize> {
        self.create_workspace_with_launch_env(initial_cwd, focus, Vec::new())
    }

    #[cfg(test)]
    pub(crate) fn create_workspace_with_events(
        &mut self,
        initial_cwd: PathBuf,
        focus: bool,
    ) -> std::io::Result<()> {
        let ws_idx = self.create_workspace_with_options(initial_cwd, focus)?;
        self.emit_workspace_open_events(ws_idx);
        Ok(())
    }

    pub(crate) fn create_workspace_with_launch_env(
        &mut self,
        initial_cwd: PathBuf,
        focus: bool,
        extra_env: Vec<(String, String)>,
    ) -> std::io::Result<usize> {
        let (rows, cols) = self.state.estimate_pane_size();
        let (mut ws, terminal, runtime) = Workspace::new_with_extra_env(
            initial_cwd,
            rows,
            cols,
            self.state.pane_scrollback_limit_bytes,
            self.state.host_terminal_theme,
            self.state.host_terminal_appearance,
            crate::pane::PaneShellConfig::new(&self.state.default_shell, self.state.shell_mode),
            self.event_tx.clone(),
            self.render_notify.clone(),
            self.render_dirty.clone(),
            extra_env,
        )?;
        // TP-WSID-07: a workspace born at a main checkout claims it at birth,
        // so the sidebar groups it under its repo without a worktree attach.
        if ws.worktree_space.is_none() {
            ws.worktree_space =
                crate::workspace::derive_initial_worktree_membership(&ws.identity_cwd);
        }
        self.terminal_runtimes.insert(terminal.id.clone(), runtime);
        self.state.terminals.insert(terminal.id.clone(), terminal);
        self.state.workspaces.push(ws);
        let idx = self.state.workspaces.len() - 1;
        self.state
            .remove_alias_shadowed_by_new_pane(self.state.workspaces[idx].tabs[0].root_pane);
        let workspace_id = self.state.workspaces[idx].id.clone();
        let root_pane = self.state.workspaces[idx].tabs[0].root_pane.raw();
        crate::logging::workspace_created(&workspace_id, root_pane);
        if focus || self.state.active.is_none() {
            self.state.switch_workspace(idx);
            self.state.mode = Mode::Terminal;
        }
        self.schedule_session_save();
        Ok(idx)
    }

    pub(super) fn collect_panes_for_workspace(
        &self,
        workspace_id: Option<&str>,
    ) -> Result<Vec<crate::api::schema::PaneInfo>, (String, String)> {
        if let Some(workspace_id) = workspace_id {
            let Some(ws_idx) = self.parse_workspace_id(workspace_id) else {
                return Err((
                    "workspace_not_found".into(),
                    format!("workspace {workspace_id} not found"),
                ));
            };
            let Some(ws) = self.state.workspaces.get(ws_idx) else {
                return Err((
                    "workspace_not_found".into(),
                    format!("workspace {workspace_id} not found"),
                ));
            };
            Ok(ws
                .tabs
                .iter()
                .flat_map(|tab| tab.layout.pane_ids().into_iter())
                .filter_map(|pane_id| self.pane_info(ws_idx, pane_id))
                .collect())
        } else {
            Ok(self
                .state
                .workspaces
                .iter()
                .enumerate()
                .flat_map(|(ws_idx, ws)| {
                    ws.tabs
                        .iter()
                        .flat_map(|tab| tab.layout.pane_ids().into_iter())
                        .filter_map(move |pane_id| self.pane_info(ws_idx, pane_id))
                })
                .collect())
        }
    }

    pub(super) fn tab_info(
        &self,
        ws_idx: usize,
        tab_idx: usize,
    ) -> Option<crate::api::schema::TabInfo> {
        let ws = self.state.workspaces.get(ws_idx)?;
        let tab = ws.tabs.get(tab_idx)?;
        let (agg_state, seen) = tab
            .panes
            .values()
            .filter_map(|pane| {
                self.state
                    .terminals
                    .get(&pane.attached_terminal_id)
                    .map(|terminal| (terminal.state, pane.seen))
            })
            .max_by_key(|(state, seen)| tab_attention_priority(*state, *seen))
            .unwrap_or((crate::detect::AgentState::Unknown, true));
        Some(crate::api::schema::TabInfo {
            tab_id: self.public_tab_id(ws_idx, tab_idx)?,
            workspace_id: self.public_workspace_id(ws_idx),
            number: tab.number,
            label: ws.tab_display_name(tab_idx)?,
            focused: self.state.active == Some(ws_idx) && ws.active_tab_index() == tab_idx,
            pane_count: tab.panes.len(),
            agent_status: pane_agent_status(agg_state, seen),
        })
    }

    pub(crate) fn emit_workspace_open_events(&mut self, ws_idx: usize) {
        let workspace_info = self.workspace_info(ws_idx);
        let Some(tab) = self.tab_info(ws_idx, 0) else {
            return;
        };
        let Some(root_pane) = self.root_pane_info(ws_idx, 0) else {
            return;
        };
        self.emit_event(EventEnvelope {
            event: EventKind::WorkspaceCreated,
            data: EventData::WorkspaceCreated {
                workspace: workspace_info,
            },
        });
        self.emit_tab_and_pane_created_events(tab, root_pane);
        self.emit_layout_updated_event(ws_idx, 0);
    }

    pub(crate) fn emit_tab_created_events(&mut self, ws_idx: usize, tab_idx: usize) {
        let Some(tab) = self.tab_info(ws_idx, tab_idx) else {
            return;
        };
        let Some(root_pane) = self.root_pane_info(ws_idx, tab_idx) else {
            return;
        };
        self.emit_tab_and_pane_created_events(tab, root_pane);
        self.emit_layout_updated_event(ws_idx, tab_idx);
    }

    fn emit_tab_and_pane_created_events(
        &mut self,
        tab: crate::api::schema::TabInfo,
        root_pane: crate::api::schema::PaneInfo,
    ) {
        self.emit_event(EventEnvelope {
            event: EventKind::TabCreated,
            data: EventData::TabCreated { tab },
        });
        self.emit_event(EventEnvelope {
            event: EventKind::PaneCreated,
            data: EventData::PaneCreated { pane: root_pane },
        });
    }

    pub(super) fn workspace_created_result(
        &self,
        ws_idx: usize,
    ) -> Option<crate::api::schema::ResponseResult> {
        Some(crate::api::schema::ResponseResult::WorkspaceCreated {
            workspace: self.workspace_info(ws_idx),
            tab: self.tab_info(ws_idx, 0)?,
            root_pane: self.root_pane_info(ws_idx, 0)?,
        })
    }

    pub(super) fn tab_created_result(
        &self,
        ws_idx: usize,
        tab_idx: usize,
    ) -> Option<crate::api::schema::ResponseResult> {
        Some(crate::api::schema::ResponseResult::TabCreated {
            tab: self.tab_info(ws_idx, tab_idx)?,
            root_pane: self.root_pane_info(ws_idx, tab_idx)?,
        })
    }

    pub(super) fn root_pane_info(
        &self,
        ws_idx: usize,
        tab_idx: usize,
    ) -> Option<crate::api::schema::PaneInfo> {
        let ws = self.state.workspaces.get(ws_idx)?;
        let tab = ws.tabs.get(tab_idx)?;
        self.pane_info(ws_idx, tab.root_pane)
    }

    pub(super) fn pane_info(
        &self,
        ws_idx: usize,
        pane_id: crate::layout::PaneId,
    ) -> Option<crate::api::schema::PaneInfo> {
        let ws = self.state.workspaces.get(ws_idx)?;
        let pane = ws.pane_state(pane_id)?;
        let terminal = self.state.terminals.get(&pane.attached_terminal_id)?;
        let tab_idx = ws.find_tab_index_for_pane(pane_id)?;
        let runtime =
            self.state
                .runtime_for_pane_in_workspace(&self.terminal_runtimes, ws_idx, pane_id);
        let scroll = runtime
            .and_then(|runtime| runtime.scroll_metrics())
            .map(|metrics| crate::api::schema::PaneScrollInfo {
                offset_from_bottom: metrics.offset_from_bottom as u64,
                max_offset_from_bottom: metrics.max_offset_from_bottom as u64,
                viewport_rows: metrics.viewport_rows as u64,
            });
        let alternate_screen =
            runtime.is_some_and(crate::terminal::TerminalRuntime::alternate_screen_active);
        let focused = self.state.active == Some(ws_idx)
            && ws.active_tab_index() == tab_idx
            && ws
                .focused_pane_id()
                .is_some_and(|focused| focused == pane_id);
        let presentation = terminal.effective_presentation();
        Some(crate::api::schema::PaneInfo {
            pane_id: self.public_pane_id(ws_idx, pane_id)?,
            terminal_id: terminal.id.to_string(),
            workspace_id: self.public_workspace_id(ws_idx),
            tab_id: self.public_tab_id(ws_idx, tab_idx)?,
            focused,
            cwd: ws.tabs[tab_idx]
                .cwd_for_pane(pane_id, &self.state.terminals, &self.terminal_runtimes)
                .map(|cwd| cwd.display().to_string()),
            foreground_cwd: ws.tabs[tab_idx]
                .foreground_cwd_for_pane(pane_id, &self.terminal_runtimes)
                .map(|cwd| cwd.display().to_string()),
            label: terminal.manual_label.clone(),
            agent: terminal.effective_agent_label().map(str::to_string),
            title: presentation.title,
            terminal_title: terminal.terminal_title.clone(),
            terminal_title_stripped: terminal.terminal_title_stripped(),
            display_agent: presentation.display_agent,
            agent_status: pane_agent_status(terminal.state, pane.seen),
            state_labels: presentation.state_labels,
            tokens: terminal.metadata_tokens.values(),
            agent_session: terminal_agent_session_info(terminal),
            scroll,
            dormant: terminal.dormant.is_some(),
            alternate_screen,
            revision: terminal.revision,
        })
    }

    pub(super) fn lookup_runtime(
        &self,
        ws_idx: usize,
        pane_id: crate::layout::PaneId,
    ) -> Option<(&crate::terminal::TerminalRuntime, String)> {
        let runtime =
            self.state
                .runtime_for_pane_in_workspace(&self.terminal_runtimes, ws_idx, pane_id)?;
        Some((runtime, self.public_workspace_id(ws_idx)))
    }

    pub(super) fn lookup_runtime_sender(
        &self,
        ws_idx: usize,
        pane_id: crate::layout::PaneId,
    ) -> Option<&crate::terminal::TerminalRuntime> {
        self.state
            .runtime_for_pane_in_workspace(&self.terminal_runtimes, ws_idx, pane_id)
    }

    pub(super) fn workspace_info(&self, index: usize) -> crate::api::schema::WorkspaceInfo {
        let ws = &self.state.workspaces[index];
        let (agg_state, seen) = ws.aggregate_state(&self.state.terminals);
        crate::api::schema::WorkspaceInfo {
            workspace_id: self.public_workspace_id(index),
            number: index + 1,
            label: ws.display_name_from(&self.state.terminals, &self.terminal_runtimes),
            focused: self.state.active == Some(index),
            pane_count: ws.public_pane_numbers.len(),
            tab_count: ws.tabs.len(),
            active_tab_id: self
                .public_tab_id(index, ws.active_tab_index())
                .unwrap_or_else(|| {
                    crate::workspace::public_tab_id_for_number(&ws.id, ws.active_tab_index() + 1)
                }),
            agent_status: pane_agent_status(agg_state, seen),
            tokens: ws.metadata_tokens.values(),
            worktree: ws
                .worktree_space()
                .map(|space| crate::api::schema::WorkspaceWorktreeInfo {
                    repo_key: space.key.clone(),
                    repo_name: space.label.clone(),
                    repo_root: space.repo_root.display().to_string(),
                    checkout_path: space.checkout_path.display().to_string(),
                    is_linked_worktree: space.is_linked_worktree,
                }),
        }
    }
}

fn terminal_agent_session_info(
    terminal: &crate::terminal::TerminalState,
) -> Option<crate::api::schema::AgentSessionInfo> {
    if let Some(authority) = terminal.hook_authority.as_ref() {
        if let Some(session_ref) = authority.session_ref.as_ref() {
            return Some(crate::api::schema::AgentSessionInfo {
                source: authority.source.clone(),
                agent: authority.agent_label.clone(),
                kind: session_ref.kind,
                value: session_ref.value.clone(),
            });
        }
    }

    terminal
        .persisted_agent_session
        .as_ref()
        .map(|session| crate::api::schema::AgentSessionInfo {
            source: session.source.clone(),
            agent: session.agent.clone(),
            kind: session.session_ref.kind,
            value: session.session_ref.value.clone(),
        })
}

#[cfg(test)]
mod merge_tests {
    use super::App;
    use crate::workspace::Workspace;

    /// The reported shape: `n` workspaces standing in one directory, none of
    /// them named.
    ///
    /// `Workspace::test_new` hands back a NAMED workspace — it stores its
    /// argument as `custom_name` — and a named workspace is never mergeable, so
    /// leaving the fixture's name in place would make every assertion below
    /// pass without the merge ever running. The precondition is asserted rather
    /// than assumed for that reason.
    fn app_with_unnamed_daily_workspaces(n: usize) -> App {
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = App::new(
            &crate::config::Config::default(),
            true,
            None,
            api_rx,
            crate::api::EventHub::default(),
        );
        let daily = std::path::PathBuf::from("/herdr-test-daily-merge");
        app.state.daily_chat_cwd = Some(daily.clone());
        app.state.workspaces = (0..n)
            .map(|_| {
                let mut ws = Workspace::test_new("user");
                ws.custom_name = None;
                ws.identity_cwd = daily.clone();
                ws
            })
            .collect();
        app.state.ensure_test_terminals();
        app.state.active = (n > 0).then_some(0);
        app.state.selected = 0;
        assert_eq!(
            app.state.mergeable_daily_workspaces().len(),
            n,
            "precondition: all {n} fixture workspaces are unnamed and stand in the daily directory"
        );
        app
    }

    fn total_panes(app: &App) -> usize {
        app.state
            .workspaces
            .iter()
            .flat_map(|ws| ws.tabs.iter())
            .map(|tab| tab.layout.pane_count())
            .sum()
    }

    fn live_terminals(app: &App) -> std::collections::BTreeSet<String> {
        app.state
            .workspaces
            .iter()
            .flat_map(|ws| ws.tabs.iter())
            .flat_map(|tab| tab.panes.values())
            .map(|pane| format!("{:?}", pane.attached_terminal_id))
            .collect()
    }

    // P2.3 + P2.5 / TP-DAILY-19: the whole point. Three copies of one place
    // become one, and NOTHING is lost on the way — every pane arrives and every
    // terminal id is still alive. Those six workspaces on the reported machine
    // held fifteen panes and one blocked agent; a cleanup that killed them
    // would be far worse than the rows it removed.
    #[test]
    fn merging_leaves_one_workspace_and_carries_every_pane_into_it() {
        let mut app = app_with_unnamed_daily_workspaces(3);
        let panes_before = total_panes(&app);
        let terminals_before = live_terminals(&app);

        app.merge_daily_workspaces("test.daily.merge");

        assert_eq!(
            app.state.mergeable_daily_workspaces().len(),
            1,
            "one place gets one workspace"
        );
        assert_eq!(
            total_panes(&app),
            panes_before,
            "every pane survived the move"
        );
        assert_eq!(
            live_terminals(&app),
            terminals_before,
            "no terminal was killed to tidy the sidebar"
        );
    }

    // P2.6 / TP-DAILY-19: the survivor is the one the person was standing in.
    #[test]
    fn the_workspace_you_are_in_is_the_one_that_survives() {
        let mut app = app_with_unnamed_daily_workspaces(3);
        app.state.active = Some(2);
        let survivor = app.state.workspaces[2].id.clone();

        app.merge_daily_workspaces("test.daily.merge");

        let remaining = app.state.mergeable_daily_workspaces();
        assert_eq!(remaining.len(), 1);
        assert_eq!(
            app.state.workspaces[remaining[0]].id, survivor,
            "a cleanup must not carry the person out of where they were working"
        );
    }

    // P2.4 / TP-DAILY-19: a named workspace is left exactly where it stands.
    #[test]
    fn merging_leaves_a_named_workspace_alone() {
        let mut app = app_with_unnamed_daily_workspaces(3);
        app.state.workspaces[2].custom_name = Some("log tail".to_string());
        let named_id = app.state.workspaces[2].id.clone();
        let panes_before = total_panes(&app);

        app.merge_daily_workspaces("test.daily.merge");

        assert!(
            app.state.workspaces.iter().any(|ws| ws.id == named_id),
            "the named workspace is still its own place"
        );
        assert_eq!(total_panes(&app), panes_before, "and it kept its panes");
    }

    // P2.2 / TP-DAILY-19: with one workspace there is no work, and the verb
    // must not invent any. Called directly rather than through the menu because
    // the menu already refuses to offer it — this pins the body itself, so a
    // future caller cannot make it act on a set it should leave alone.
    #[test]
    fn merging_a_single_workspace_changes_nothing() {
        let mut app = app_with_unnamed_daily_workspaces(1);
        let workspaces_before = app.state.workspaces.len();
        let panes_before = total_panes(&app);

        app.merge_daily_workspaces("test.daily.merge");

        assert_eq!(app.state.workspaces.len(), workspaces_before);
        assert_eq!(total_panes(&app), panes_before);
    }

    // P2.8 / TP-DAILY-19: a plugin or a CLI caller holding a pane's public id
    // from before the merge still resolves it afterwards. `pane.move` aliases
    // the id precisely so this holds; if it stopped holding, `pane send-keys`
    // would quietly reach the wrong pane rather than fail.
    #[test]
    fn a_merged_pane_still_answers_to_the_public_id_it_had() {
        let mut app = app_with_unnamed_daily_workspaces(2);
        let source_ws = app.state.mergeable_daily_workspaces()[1];
        let pane = app.state.workspaces[source_ws].tabs[0].root_pane;
        let old_public_id = app
            .public_pane_id(source_ws, pane)
            .expect("the fixture pane has a public id");

        app.merge_daily_workspaces("test.daily.merge");

        assert!(
            app.parse_pane_id(&old_public_id).is_some(),
            "the id a plugin already holds must keep resolving after the move"
        );
    }

    // P2.9 / TP-DAILY-19: a workspace in a repository is never in the set, so
    // the merge cannot reach it. This is the guard that keeps a tidy-up of the
    // daily area from ever touching real work elsewhere.
    #[test]
    fn a_workspace_in_a_repository_is_untouched_by_the_merge() {
        let mut app = app_with_unnamed_daily_workspaces(2);
        let mut repo = Workspace::test_new("herdr");
        repo.custom_name = None;
        repo.identity_cwd = std::env::temp_dir().join("herdr-merge-test-repo");
        let repo_id = repo.id.clone();
        app.state.workspaces.push(repo);
        app.state.ensure_test_terminals();

        app.merge_daily_workspaces("test.daily.merge");

        assert!(
            app.state.workspaces.iter().any(|ws| ws.id == repo_id),
            "the rule is about one directory, not about workspaces in general"
        );
    }
}
