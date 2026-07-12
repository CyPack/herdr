//! Projects-tab chat actions (Task #5): opening a Claude Code chat session as
//! a new tab in the right workspace. The pure launch planning lives here so it
//! is testable without PTYs; the tab spawn itself reuses the proven
//! `Workspace::create_tab_argv_command` path (same as plugin panes).

/// Agent CLIs a NEW project chat can launch with, in menu order. The entries
/// double as the selector menu items and the `[projects] default_chat_agent`
/// config values.
pub(crate) const CHAT_AGENTS: &[&str] = &["claude", "codex", "gemini", "opencode"];

/// Builds the argv + extra launch env for opening a project chat.
///
/// `session_id` `Some` resumes that session with `claude --resume <id>` —
/// ALWAYS claude, whatever `agent` says, because the sidebar's sessions are
/// Claude Code sessions no other CLI can resume. `None` starts a fresh chat
/// with `agent` (falling back to claude for unknown ids so a stale config
/// value can never break the "+" button). The claude launch mirrors the
/// user's fish `cc` alias: permission prompts skipped, background tasks on.
pub(crate) fn project_chat_launch(
    agent: &str,
    session_id: Option<&str>,
) -> (Vec<String>, Vec<(String, String)>) {
    let claude_launch = || {
        (
            vec![
                "claude".to_string(),
                "--dangerously-skip-permissions".to_string(),
            ],
            vec![("ENABLE_BACKGROUND_TASKS".to_string(), "1".to_string())],
        )
    };

    if let Some(session_id) = session_id {
        let (mut argv, env) = claude_launch();
        argv.push("--resume".to_string());
        argv.push(session_id.to_string());
        return (argv, env);
    }

    match agent {
        "claude" => claude_launch(),
        "codex" | "gemini" | "opencode" => (vec![agent.to_string()], Vec::new()),
        unknown => {
            tracing::warn!(agent = unknown, "unknown default_chat_agent; using claude");
            claude_launch()
        }
    }
}

impl super::App {
    /// Consume a queued Projects-tab chat request, if any. Returns whether a
    /// request was handled so callers can trigger a re-render. Shared by the
    /// headless server and the monolithic event loop deferred-request passes.
    pub(crate) fn handle_project_chat_tab_request(&mut self) -> bool {
        let Some(req) = self.state.request_project_chat_tab.take() else {
            return false;
        };
        self.open_project_chat_tab(req);
        true
    }

    /// Open a project chat (resume or new) as a tab in the right workspace.
    /// New chats launch with the configured default agent; resumes are always
    /// claude (see `project_chat_launch`).
    fn open_project_chat_tab(&mut self, req: crate::app::state::ProjectChatTabRequest) {
        let (argv, extra_env) =
            project_chat_launch(&self.state.default_chat_agent, req.session_id.as_deref());
        self.open_project_chat_tab_with_argv(req, &argv, extra_env);
    }

    /// Spawn `argv` as a new focused tab for `req`: prefer the workspace whose
    /// identity matches the project directory, else the active workspace, else
    /// (empty session) a fresh workspace in that directory. The tab's cwd is
    /// always the project directory, and the tab is wired to the resumed
    /// session id so later clicks can find it. Spawn failures are logged, not
    /// fatal: the app must survive e.g. a deleted project directory.
    fn open_project_chat_tab_with_argv(
        &mut self,
        req: crate::app::state::ProjectChatTabRequest,
        argv: &[String],
        extra_env: Vec<(String, String)>,
    ) {
        // Consume-side spam guard: if a second click queued this session while
        // the first spawn was still in flight (or the tab already exists for
        // any other reason), focus the wired tab instead of duplicating it.
        if let Some(session_id) = req.session_id.as_deref() {
            if let Some((ws_idx, tab_idx)) = self.state.find_resumed_chat_tab(session_id) {
                self.state.switch_workspace_tab(ws_idx, tab_idx);
                self.state.mode = crate::app::Mode::Terminal;
                return;
            }
        }

        let (rows, cols) = self.state.estimate_pane_size();
        let target_ws = self
            .state
            .workspaces
            .iter()
            .position(|ws| ws.identity_cwd == req.project_path)
            .or(self.state.active)
            .filter(|ws_idx| *ws_idx < self.state.workspaces.len());

        let Some(ws_idx) = target_ws else {
            // No workspace to attach the tab to: open the chat in a fresh
            // workspace rooted at the project directory instead.
            match self.spawn_agent_workspace(
                req.project_path.clone(),
                rows,
                cols,
                argv,
                extra_env,
                true,
            ) {
                Ok((ws_idx, tab_idx, _pane_id)) => {
                    self.state.workspaces[ws_idx].tabs[tab_idx].resumed_session_id = req.session_id;
                }
                Err(err) => {
                    tracing::warn!(
                        project = %req.project_path.display(),
                        err = ?err,
                        "failed to open project chat in a new workspace"
                    );
                }
            }
            return;
        };

        let Some(ws) = self.state.workspaces.get_mut(ws_idx) else {
            return;
        };
        let created = ws.create_tab_argv_command(
            rows.max(4),
            cols.max(10),
            req.project_path.clone(),
            argv,
            extra_env,
            self.state.pane_scrollback_limit_bytes,
            self.state.host_terminal_theme,
        );
        let (tab_idx, terminal, runtime) = match created {
            Ok(result) => result,
            Err(err) => {
                tracing::warn!(
                    project = %req.project_path.display(),
                    err = %err,
                    "failed to open project chat tab"
                );
                return;
            }
        };
        let root_pane = self.state.workspaces[ws_idx].tabs[tab_idx].root_pane;
        self.terminal_runtimes.insert(terminal.id.clone(), runtime);
        self.state.remove_alias_shadowed_by_new_pane(root_pane);
        self.state.terminals.insert(terminal.id.clone(), terminal);
        self.state.workspaces[ws_idx].tabs[tab_idx].resumed_session_id = req.session_id;
        self.state.switch_workspace_tab(ws_idx, tab_idx);
        self.state.mode = crate::app::Mode::Terminal;
        self.schedule_session_save();
        if let Some(tab) = self.tab_info(ws_idx, tab_idx) {
            self.emit_event(crate::api::schema::EventEnvelope {
                event: crate::api::schema::EventKind::TabCreated,
                data: crate::api::schema::EventData::TabCreated { tab },
            });
        }
        if let Some(pane) = self.pane_info(ws_idx, root_pane) {
            self.emit_event(crate::api::schema::EventEnvelope {
                event: crate::api::schema::EventKind::PaneCreated,
                data: crate::api::schema::EventData::PaneCreated { pane },
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::path::PathBuf;

    use crate::app::state::ProjectChatTabRequest;
    use crate::workspace::Workspace;

    fn test_app() -> crate::app::App {
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        crate::app::App::new(
            &crate::config::Config::default(),
            true,
            None,
            api_rx,
            crate::api::EventHub::default(),
        )
    }

    /// A real, unique, empty directory: the spawned pane needs an existing cwd.
    fn unique_project_dir(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("herdr-projects-test-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create test project dir");
        dir
    }

    /// A harmless real argv so tab-spawn tests never launch an actual agent.
    fn sh_argv() -> Vec<String> {
        vec!["/bin/sh".into(), "-c".into(), "exit 0".into()]
    }

    fn req(project_path: PathBuf, session_id: Option<&str>) -> ProjectChatTabRequest {
        ProjectChatTabRequest {
            project_path,
            session_id: session_id.map(str::to_string),
        }
    }

    // T5c: closing a resumed-chat tab must release its terminal runtime via
    // the standard detached-runtime shutdown queue — the path that kills the
    // pane's PTY session (claude + its MCP children) and frees the wiring.
    // Phase 1 proves the close only QUEUES the shutdown; phase 2 proves the
    // event-loop drain actually unregisters and shuts the runtime down.
    #[tokio::test]
    async fn closing_resumed_chat_tab_shuts_down_its_terminal_runtime() {
        let dir = unique_project_dir("close");
        let mut app = test_app();
        let mut ws = Workspace::test_new("proj");
        ws.identity_cwd = dir.clone();
        app.state.workspaces = vec![ws];
        app.state.active = Some(0);
        app.state.selected = 0;

        app.open_project_chat_tab_with_argv(
            req(dir.clone(), Some("sess-7")),
            &sh_argv(),
            Vec::new(),
        );
        let ws = &app.state.workspaces[0];
        assert_eq!(ws.tabs.len(), 2, "resume tab opened");
        let tab = &ws.tabs[1];
        let terminal_id = tab.panes[&tab.root_pane].attached_terminal_id.clone();
        assert!(app.terminal_runtimes.get(&terminal_id).is_some());

        // TUI-path close of the focused (resume) tab: queues the shutdown.
        app.state.close_tab();
        assert!(
            app.state.terminal_runtime_shutdowns.contains(&terminal_id),
            "close queues the detached terminal for shutdown"
        );
        assert!(
            app.terminal_runtimes.get(&terminal_id).is_some(),
            "runtime lives until the event loop drains the queue"
        );

        // The pass runtime.rs runs after every input event / API request.
        app.shutdown_detached_terminal_runtimes();

        assert!(
            app.terminal_runtimes.get(&terminal_id).is_none(),
            "runtime shut down and unregistered"
        );
        assert!(
            !app.state.terminals.contains_key(&terminal_id),
            "terminal state removed with the tab"
        );
        assert_eq!(
            app.state.find_resumed_chat_tab("sess-7"),
            None,
            "chat wiring released by the close"
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    // T5c (live kill proof, linux): closing the tab must terminate the WHOLE
    // pane session — the shell and its background child (a stand-in for MCP
    // servers a chat spawns). The unique sleep durations act as markers that
    // survive shell `exec`, so the tree is observable from outside via pgrep.
    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn closing_resumed_chat_tab_kills_the_pane_process_tree() {
        fn pgrep(pattern: &str) -> Vec<u32> {
            let out = std::process::Command::new("pgrep")
                .args(["-f", pattern])
                .output()
                .expect("pgrep runs");
            String::from_utf8_lossy(&out.stdout)
                .lines()
                .filter_map(|line| line.trim().parse().ok())
                .collect()
        }

        let dir = unique_project_dir("kill");
        let tag = format!("9{:05}", std::process::id() % 100_000);
        let script = format!("sleep {tag}1 & sleep {tag}2");
        let pattern = format!("sleep {tag}");
        let argv = vec!["/bin/sh".to_string(), "-c".to_string(), script];

        let mut app = test_app();
        let mut ws = Workspace::test_new("proj");
        ws.identity_cwd = dir.clone();
        app.state.workspaces = vec![ws];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.open_project_chat_tab_with_argv(req(dir.clone(), Some("sess-k")), &argv, Vec::new());

        // Both the background and foreground sleeps must come up before we
        // can prove they die (the PTY spawn is asynchronous).
        let mut alive = Vec::new();
        for _ in 0..100 {
            alive = pgrep(&pattern);
            if alive.len() >= 2 {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        assert!(
            alive.len() >= 2,
            "pane process tree (shell child + background child) started: {alive:?}"
        );

        app.state.close_tab();
        app.shutdown_detached_terminal_runtimes();

        // shutdown_pane_processes escalates HUP→TERM→KILL with short graces;
        // give the kernel a beat to reap before asserting extinction.
        let mut remaining = pgrep(&pattern);
        for _ in 0..100 {
            if remaining.is_empty() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            remaining = pgrep(&pattern);
        }
        assert!(
            remaining.is_empty(),
            "all pane session processes must be dead after tab close: {remaining:?}"
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    // T5b (race guard): consuming a request whose session is already wired
    // (e.g. a second click landed before the first spawn finished) focuses
    // the wired tab instead of spawning a duplicate.
    #[tokio::test]
    async fn open_chat_tab_focuses_existing_wired_tab_instead_of_duplicating() {
        let dir = unique_project_dir("dedup");
        let mut app = test_app();
        let mut ws = Workspace::test_new("proj");
        ws.identity_cwd = dir.clone();
        let tab_idx = ws.test_add_tab(Some("chat"));
        ws.tabs[tab_idx].resumed_session_id = Some("sess-9".to_string());
        app.state.workspaces = vec![ws];
        app.state.active = Some(0);
        app.state.selected = 0;

        app.open_project_chat_tab_with_argv(
            req(dir.clone(), Some("sess-9")),
            &sh_argv(),
            Vec::new(),
        );

        assert_eq!(
            app.state.workspaces[0].tabs.len(),
            2,
            "no duplicate tab spawned"
        );
        assert_eq!(
            app.state.workspaces[0].active_tab, tab_idx,
            "wired tab focused"
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    // T5a-7: the tab must land in the workspace whose identity matches the
    // chat's project directory (not the active one), get the project as cwd,
    // be wired to the session id, and take focus.
    #[tokio::test]
    async fn open_chat_tab_targets_workspace_matching_project_dir() {
        let dir = unique_project_dir("match");
        let mut app = test_app();
        let mut other = Workspace::test_new("other");
        other.identity_cwd = PathBuf::from("/");
        let mut matching = Workspace::test_new("proj");
        matching.identity_cwd = dir.clone();
        app.state.workspaces = vec![other, matching];
        app.state.active = Some(0);
        app.state.selected = 0;

        app.open_project_chat_tab_with_argv(
            req(dir.clone(), Some("sess-9")),
            &sh_argv(),
            Vec::new(),
        );

        assert_eq!(
            app.state.workspaces[0].tabs.len(),
            1,
            "active-but-unrelated workspace must stay untouched"
        );
        let ws = &app.state.workspaces[1];
        assert_eq!(ws.tabs.len(), 2, "tab added to the matching workspace");
        let tab = &ws.tabs[1];
        assert_eq!(tab.resumed_session_id.as_deref(), Some("sess-9"));
        assert_eq!(app.state.active, Some(1), "focus follows the new tab");
        assert_eq!(ws.active_tab, 1);
        let terminal_id = tab.panes[&tab.root_pane].attached_terminal_id.clone();
        let terminal = app
            .state
            .terminals
            .get(&terminal_id)
            .expect("terminal registered");
        assert_eq!(terminal.cwd, dir, "tab cwd is the chat's project dir");
        assert!(
            app.terminal_runtimes.get(&terminal_id).is_some(),
            "runtime registered"
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    // T5a-8: with no identity match the tab opens in the ACTIVE workspace —
    // but its cwd must still be the project directory, and a new chat
    // (session_id None) must not wire a session id.
    #[tokio::test]
    async fn open_chat_tab_falls_back_to_active_workspace() {
        let dir = unique_project_dir("fallback");
        let mut app = test_app();
        let mut ws = Workspace::test_new("elsewhere");
        ws.identity_cwd = PathBuf::from("/");
        app.state.workspaces = vec![ws];
        app.state.active = Some(0);
        app.state.selected = 0;

        app.open_project_chat_tab_with_argv(req(dir.clone(), None), &sh_argv(), Vec::new());

        let ws = &app.state.workspaces[0];
        assert_eq!(ws.tabs.len(), 2, "tab added to the active workspace");
        let tab = &ws.tabs[1];
        assert_eq!(tab.resumed_session_id, None, "new chat is not wired");
        let terminal_id = tab.panes[&tab.root_pane].attached_terminal_id.clone();
        let terminal = app
            .state
            .terminals
            .get(&terminal_id)
            .expect("terminal registered");
        assert_eq!(terminal.cwd, dir);
        let _ = std::fs::remove_dir_all(dir);
    }

    // T5a-9: with no workspaces at all a fresh one is created in the project
    // directory (a tab cannot exist outside a workspace).
    #[tokio::test]
    async fn open_chat_tab_spawns_workspace_when_none_exists() {
        let dir = unique_project_dir("spawn");
        let mut app = test_app();
        assert!(app.state.workspaces.is_empty());

        app.open_project_chat_tab_with_argv(
            req(dir.clone(), Some("sess-3")),
            &sh_argv(),
            Vec::new(),
        );

        assert_eq!(app.state.workspaces.len(), 1, "workspace created");
        let ws = &app.state.workspaces[0];
        assert_eq!(ws.tabs[0].resumed_session_id.as_deref(), Some("sess-3"));
        assert_eq!(app.state.active, Some(0), "new workspace focused");
        let _ = std::fs::remove_dir_all(dir);
    }

    // T5a-10: the deferred request is consumed exactly once, and a spawn
    // failure (nonexistent project dir) must not panic or leave the request
    // stuck — the app survives e.g. a project deleted after pinning.
    #[tokio::test]
    async fn handle_project_chat_tab_request_consumes_queued_request() {
        let mut app = test_app();
        app.state.request_project_chat_tab =
            Some(req(PathBuf::from("/nonexistent/herdr-projects-test"), None));

        assert!(app.handle_project_chat_tab_request());
        assert_eq!(app.state.request_project_chat_tab, None);
        assert!(
            !app.handle_project_chat_tab_request(),
            "second pass has nothing to consume"
        );
    }

    // C1: a new chat launches with the selected agent's plain command and no
    // claude-specific env; each catalog entry must map to a real CLI name.
    #[test]
    fn project_chat_launch_new_chat_uses_selected_agent() {
        for agent in ["codex", "gemini", "opencode"] {
            let (argv, env) = project_chat_launch(agent, None);
            assert_eq!(argv, vec![agent.to_string()], "argv for {agent}");
            assert!(env.is_empty(), "no claude env for {agent}");
        }
    }

    // C1 (no-happy-path): a stale/unknown config value must never break the
    // "+" button — it falls back to the claude launch.
    #[test]
    fn project_chat_launch_unknown_agent_falls_back_to_claude() {
        let (argv, _env) = project_chat_launch("not-a-real-agent", None);
        assert_eq!(argv[0], "claude");
        assert_eq!(argv[1], "--dangerously-skip-permissions");
    }

    // C1 (guard): resuming ALWAYS uses claude even when another agent is the
    // default — sidebar sessions are Claude Code sessions.
    #[test]
    fn project_chat_launch_resume_ignores_selected_agent() {
        let (argv, _env) = project_chat_launch("codex", Some("sess-1"));
        assert_eq!(
            argv,
            vec![
                "claude".to_string(),
                "--dangerously-skip-permissions".to_string(),
                "--resume".to_string(),
                "sess-1".to_string(),
            ]
        );
    }

    // T5a-1: resuming a chat must produce the exact fish-`cc` argv shape;
    // a wrong flag order or missing id opens the wrong (or no) session.
    #[test]
    fn project_chat_launch_builds_resume_argv() {
        let (argv, env) =
            project_chat_launch("claude", Some("0d55b02e-aaaa-bbbb-cccc-111111111111"));
        assert_eq!(
            argv,
            vec![
                "claude".to_string(),
                "--dangerously-skip-permissions".to_string(),
                "--resume".to_string(),
                "0d55b02e-aaaa-bbbb-cccc-111111111111".to_string(),
            ]
        );
        assert_eq!(
            env,
            vec![("ENABLE_BACKGROUND_TASKS".to_string(), "1".to_string())]
        );
    }

    // T5a-2: a new chat is the same launch without `--resume`; passing an
    // empty `--resume` would make claude error out instead of starting fresh.
    #[test]
    fn project_chat_launch_builds_new_chat_argv() {
        let (argv, env) = project_chat_launch("claude", None);
        assert_eq!(
            argv,
            vec![
                "claude".to_string(),
                "--dangerously-skip-permissions".to_string(),
            ]
        );
        assert_eq!(
            env,
            vec![("ENABLE_BACKGROUND_TASKS".to_string(), "1".to_string())]
        );
    }
}
