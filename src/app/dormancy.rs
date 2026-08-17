//! Pane dormancy: releasing a retired pane's runtime while keeping its pane.
//!
//! A pane whose child has exited still holds the most expensive things a pane
//! owns — a PTY actor, a reader task, and a libghostty core carrying the whole
//! scrollback in memory. Herdr's reason to exist is agents running for days,
//! so a session accumulates these retired panes by design; dormancy is how
//! they stop costing anything without the pane disappearing from the session.
//!
//! Dormancy is strictly narrower than closing: the pane, its tab position,
//! its terminal identity, labels, and agent metadata all stay. Only the
//! runtime goes, and the scrollback goes to disk first so waking the pane
//! shows the history the user left.
//!
//! Two hard rules, in order of importance:
//!
//! 1. **A pane with a live child is never dormanted.** Killing background
//!    work would break the product's core promise; the guard is
//!    [`crate::pane::PaneRuntime::child_exited`], which asks both herdr's
//!    reaping record and the OS.
//! 2. **A watched pane is never dormanted.** A display is looking at it;
//!    releasing the runtime under a viewer would blank their screen.

use std::path::PathBuf;

use super::App;

/// Why a pane was left alone by [`App::make_pane_dormant`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DormancyRefusal {
    /// The pane id resolved to nothing.
    NoSuchPane,
    /// The pane has no runtime — it is already dormant or never had one.
    NoRuntime,
    /// The child process is still running; dormancy never touches live work.
    ChildStillRunning,
    /// A display is watching the pane's tab.
    Watched,
    /// The scrollback could not be written; the runtime was kept rather than
    /// releasing history that only existed in memory.
    HistoryWriteFailed,
    /// The pane is on the alternate screen, where the non-unix capture path
    /// can only read the active screen; sleeping it would silently discard
    /// the primary history. On unix the patched screen formatter (vendor
    /// patch 0002) reads the primary screen directly, so this refusal no
    /// longer exists there — the pane sleeps with its primary history.
    #[cfg(not(unix))]
    HistoryUnavailable,
}

impl App {
    /// Where dormant scrollback files live. Overridable so tests never touch
    /// the real data directory.
    pub(crate) fn dormant_history_dir(&self) -> PathBuf {
        self.dormant_dir
            .clone()
            .unwrap_or_else(|| crate::session::data_dir().join("dormant"))
    }

    /// Whether anything is looking at this tab right now.
    ///
    /// Server mode counts attached displays; the monolithic TUI (which has an
    /// input channel and no client map) counts its own active tab. Dormancy
    /// and waking share this one definition so a pane cannot be slept by one
    /// rule and immediately woken by another.
    pub(crate) fn tab_effectively_watched(&self, ws_idx: usize, tab_idx: usize) -> bool {
        let Some(workspace) = self.state.workspaces.get(ws_idx) else {
            return false;
        };
        if workspace.tab_is_watched(tab_idx) {
            return true;
        }
        self.input_rx.is_some()
            && self.state.active == Some(ws_idx)
            && workspace.active_tab_index() == tab_idx
    }

    /// Release a retired pane's runtime, writing its scrollback to disk first.
    ///
    /// TP-DORMANT-01 (live child refused) · TP-DORMANT-02 (release + recipe)
    pub(crate) fn make_pane_dormant(
        &mut self,
        pane_id: crate::layout::PaneId,
    ) -> Result<(), DormancyRefusal> {
        let Some((ws_idx, pane_state)) = self.find_pane(pane_id) else {
            return Err(DormancyRefusal::NoSuchPane);
        };
        let terminal_id = pane_state.attached_terminal_id.clone();
        let Some(runtime) = self.terminal_runtimes.get(&terminal_id) else {
            return Err(DormancyRefusal::NoRuntime);
        };
        if !runtime.child_exited() {
            return Err(DormancyRefusal::ChildStillRunning);
        }
        let watched = self.state.workspaces[ws_idx]
            .tabs
            .iter()
            .position(|tab| tab.panes.contains_key(&pane_id))
            .is_some_and(|tab_idx| self.tab_effectively_watched(ws_idx, tab_idx));
        if watched {
            return Err(DormancyRefusal::Watched);
        }

        // A pane on the alternate screen used to be refused here: capture
        // could only read the active screen, so it came back empty and the
        // pane would sleep into exactly the deferred data loss TP-DORMANT-03
        // names. On unix the capture now reads the primary screen directly
        // (vendor patch 0002), so such a pane sleeps with its primary
        // history. The non-unix capture path still reads the active screen
        // only, where the loss is real — the refusal stays. TP-DORMANT-10
        #[cfg(not(unix))]
        if runtime
            .input_state()
            .is_some_and(|input_state| input_state.alternate_screen)
        {
            return Err(DormancyRefusal::HistoryUnavailable);
        }

        // Capture before releasing anything: the runtime is the only holder
        // of this history until the file exists.
        #[cfg(unix)]
        let history = runtime.handoff_history_ansi_full();
        #[cfg(not(unix))]
        let history = runtime.snapshot_history();
        let history_path = match history {
            Some(history) if !history.is_empty() => {
                let dir = self.dormant_history_dir();
                let path = dir.join(format!("{terminal_id}.ansi"));
                // tmp + fsync + rename: fs::write starts with truncate, so a
                // crash mid-write would leave a torn file the wake replays as
                // garbage. The freight writer uses the same shape. TP-DORMANT-09
                let tmp = dir.join(format!("{terminal_id}.ansi.tmp"));
                let written = std::fs::create_dir_all(&dir).and_then(|()| {
                    let mut file = std::fs::File::create(&tmp)?;
                    std::io::Write::write_all(&mut file, history.as_bytes())?;
                    file.sync_all()?;
                    std::fs::rename(&tmp, &path)
                });
                if let Err(err) = written {
                    let _ = std::fs::remove_file(&tmp);
                    tracing::warn!(
                        terminal = %terminal_id,
                        err = %err,
                        "keeping pane runtime: dormant history could not be written"
                    );
                    return Err(DormancyRefusal::HistoryWriteFailed);
                }
                Some(path)
            }
            _ => None,
        };

        if let Some(runtime) = self.terminal_runtimes.remove(&terminal_id) {
            runtime.shutdown();
        }
        if let Some(terminal) = self.state.terminals.get_mut(&terminal_id) {
            terminal.dormant = Some(crate::terminal::DormantTerminal { history_path });
        }
        tracing::info!(terminal = %terminal_id, "pane went dormant");
        Ok(())
    }

    /// Rebuild a dormant pane's runtime, replaying its saved scrollback.
    ///
    /// Returns whether a wake actually happened; a pane that is not dormant
    /// is left alone, so this is safe to call from any touch path.
    ///
    /// TP-DORMANT-03 (history restored on wake)
    pub(crate) fn wake_dormant_pane(&mut self, pane_id: crate::layout::PaneId) -> bool {
        let Some((ws_idx, pane_state)) = self.find_pane(pane_id) else {
            return false;
        };
        let terminal_id = pane_state.attached_terminal_id.clone();
        let Some(dormant) = self
            .state
            .terminals
            .get(&terminal_id)
            .and_then(|terminal| terminal.dormant.clone())
        else {
            return false;
        };
        let Some(cwd) = self
            .state
            .terminals
            .get(&terminal_id)
            .map(|terminal| terminal.cwd.clone())
        else {
            return false;
        };

        // The click protocol's second half: a pane that carried an agent
        // session wakes into the cold-restore resume machinery instead of a
        // bare shell — only the plan is queued here, the machinery owns the
        // launch (theme wait, spawn, cleanup). The saved scrollback is
        // deleted unreplayed, faithful to cold restore: the resumed agent
        // repaints its own transcript, and a replay would double it. An
        // agent the resume table does not know falls through to the shell
        // path below, history intact. TP-DORMANT-12
        if let Some(plan) = self
            .state
            .terminals
            .get(&terminal_id)
            .and_then(|terminal| terminal.persisted_agent_session.as_ref())
            .and_then(|session| {
                crate::agent_resume::plan(&session.source, &session.agent, &session.session_ref)
            })
        {
            if let Some(terminal) = self.state.terminals.get_mut(&terminal_id) {
                terminal.dormant = None;
                terminal.pending_agent_resume_plan = Some(plan);
            }
            if let Some(path) = dormant.history_path.as_deref() {
                let _ = std::fs::remove_file(path);
            }
            tracing::info!(terminal = %terminal_id, "dormant agent pane woke into a resume");
            self.emit_pane_updated(ws_idx, pane_id);
            return true;
        }

        let history = dormant
            .history_path
            .as_deref()
            .and_then(|path| std::fs::read_to_string(path).ok());
        let (rows, cols) = self.state.estimate_pane_size();
        let Some(launch_env) = self.pane_launch_env(ws_idx, pane_id, Vec::new()) else {
            return false;
        };
        let runtime = match crate::terminal::TerminalRuntime::spawn_with_initial_history(
            pane_id,
            rows,
            cols,
            cwd,
            self.state.pane_scrollback_limit_bytes,
            self.state.host_terminal_theme,
            crate::pane::PaneShellConfig::new(&self.state.default_shell, self.state.shell_mode),
            &launch_env,
            history.as_deref(),
            self.event_tx.clone(),
            self.render_notify.clone(),
            self.render_dirty.clone(),
        ) {
            Ok(runtime) => runtime,
            Err(err) => {
                tracing::warn!(
                    terminal = %terminal_id,
                    err = %err,
                    "failed to wake dormant pane; it stays dormant"
                );
                return false;
            }
        };

        self.terminal_runtimes.insert(terminal_id.clone(), runtime);
        if let Some(terminal) = self.state.terminals.get_mut(&terminal_id) {
            terminal.dormant = None;
        }
        if let Some(path) = dormant.history_path.as_deref() {
            let _ = std::fs::remove_file(path);
        }
        tracing::info!(terminal = %terminal_id, "dormant pane woke");
        self.emit_pane_updated(ws_idx, pane_id);
        true
    }

    /// Wake every dormant pane a display is currently watching.
    ///
    /// This is the touch protocol's chokepoint: attach, tab switch, workspace
    /// switch, and focus all end with the pane's tab being watched, so waking
    /// here covers them without instrumenting each path. When nothing is
    /// dormant this is map lookups only — no runtime or ghostty calls.
    ///
    /// TP-DORMANT-04
    pub(crate) fn wake_dormant_panes_on_watched_tabs(&mut self) -> bool {
        let mut to_wake = Vec::new();
        for (ws_idx, workspace) in self.state.workspaces.iter().enumerate() {
            for (tab_idx, tab) in workspace.tabs.iter().enumerate() {
                if !self.tab_effectively_watched(ws_idx, tab_idx) {
                    continue;
                }
                for (pane_id, pane) in &tab.panes {
                    let dormant = self
                        .state
                        .terminals
                        .get(&pane.attached_terminal_id)
                        .is_some_and(|terminal| terminal.dormant.is_some());
                    if dormant {
                        to_wake.push(*pane_id);
                    }
                }
            }
        }
        let mut woke = false;
        for pane_id in to_wake {
            woke |= self.wake_dormant_pane(pane_id);
        }
        woke
    }

    /// Delete history files whose dormant terminals were closed.
    ///
    /// `AppState` stays pure data: the close path only queues the orphaned
    /// paths, and this drain — running every scheduled-task pass in both
    /// loops, flag or no flag — does the file IO. TP-DORMANT-08
    pub(crate) fn sync_dormant_history_removals(&mut self) -> bool {
        if self.state.pending_dormant_history_removals.is_empty() {
            return false;
        }
        for path in std::mem::take(&mut self.state.pending_dormant_history_removals) {
            let _ = std::fs::remove_file(path);
        }
        false
    }

    /// The periodic dormancy sweep: retire-quiet panes go to sleep.
    ///
    /// Policy, in the order the decisions were locked:
    /// - only a pane whose child has exited is ever a candidate,
    /// - a watched pane is never touched,
    /// - memory pressure is the primary trigger; without it a candidate must
    ///   have been retired for [`DORMANCY_QUIET_THRESHOLD`] first.
    ///
    /// Behind `[experimental] pane_dormancy`; the sweep itself runs at most
    /// once per [`DORMANCY_SWEEP_INTERVAL`], and with the flag off it costs
    /// one boolean test per loop tick. TP-DORMANT-05
    pub(crate) fn sync_pane_dormancy_sweep(&mut self, now: std::time::Instant) -> bool {
        if !self.pane_dormancy_enabled {
            return false;
        }
        if self.next_dormancy_sweep_at.is_some_and(|at| now < at) {
            return false;
        }
        self.next_dormancy_sweep_at = Some(now + DORMANCY_SWEEP_INTERVAL);
        self.dormancy_sweep_with_pressure(now, memory_pressure_is_high())
    }

    /// The sweep body with the pressure signal injected, so the policy is
    /// testable without a loaded machine.
    pub(crate) fn dormancy_sweep_with_pressure(
        &mut self,
        now: std::time::Instant,
        memory_pressure_high: bool,
    ) -> bool {
        // Observe first, mutate after: candidates are collected against the
        // current state, then each dormancy re-checks its own guards.
        let mut candidates = Vec::new();
        let mut retired_now = std::collections::HashSet::new();
        for (ws_idx, workspace) in self.state.workspaces.iter().enumerate() {
            for (tab_idx, tab) in workspace.tabs.iter().enumerate() {
                let watched = self.tab_effectively_watched(ws_idx, tab_idx);
                for (pane_id, pane) in &tab.panes {
                    let Some(runtime) = self.terminal_runtimes.get(&pane.attached_terminal_id)
                    else {
                        continue;
                    };
                    if !runtime.child_exited() {
                        continue;
                    }
                    retired_now.insert(pane.attached_terminal_id.clone());
                    let since = *self
                        .pane_retired_since
                        .entry(pane.attached_terminal_id.clone())
                        .or_insert(now);
                    let quiet_enough =
                        now.saturating_duration_since(since) >= DORMANCY_QUIET_THRESHOLD;
                    if !watched && (memory_pressure_high || quiet_enough) {
                        candidates.push((ws_idx, *pane_id));
                    }
                }
            }
        }
        // A pane that came back to life (respawned shell) leaves the ledger,
        // so a later retirement starts its quiet clock from zero.
        self.pane_retired_since
            .retain(|terminal_id, _| retired_now.contains(terminal_id));

        let mut changed = false;
        for (ws_idx, pane_id) in candidates {
            if self.make_pane_dormant(pane_id).is_ok() {
                self.emit_pane_updated(ws_idx, pane_id);
                changed = true;
            }
        }
        changed
    }

    /// Whether an idle server has earned retirement.
    ///
    /// True only when the whole session has been clientless and childless for
    /// [`IDLE_SERVER_EXIT_GRACE`]. The clock starts at the first idle sighting
    /// and resets the moment a client attaches or any child is alive again;
    /// a freshly handed-off server therefore starts a fresh clock. Checks run
    /// at most once per [`DORMANCY_SWEEP_INTERVAL`] because each one asks the
    /// OS about every pane's child. TP-SRV-RETIRE-01/02/03
    pub(crate) fn server_retirement_due(
        &mut self,
        now: std::time::Instant,
        clients_attached: bool,
    ) -> bool {
        if !self.idle_server_exit_enabled {
            return false;
        }
        if self.next_retirement_check_at.is_some_and(|at| now < at) {
            return false;
        }
        self.next_retirement_check_at = Some(now + DORMANCY_SWEEP_INTERVAL);

        let any_live_child = self
            .terminal_runtimes
            .values()
            .any(|runtime| !runtime.child_exited());
        if clients_attached || any_live_child {
            self.server_idle_since = None;
            return false;
        }
        let since = *self.server_idle_since.get_or_insert(now);
        now.saturating_duration_since(since) >= IDLE_SERVER_EXIT_GRACE
    }

    /// Write every retired pane's scrollback to disk before the server exits.
    ///
    /// Retirement without this loses exactly the state retirement claims to
    /// preserve: a runtime's scrollback lives only in this process until a
    /// dormant file exists.
    pub(crate) fn dormant_all_retired_panes(&mut self) {
        let mut pane_ids = Vec::new();
        for workspace in &self.state.workspaces {
            for tab in &workspace.tabs {
                pane_ids.extend(tab.panes.keys().copied());
            }
        }
        for pane_id in pane_ids {
            let _ = self.make_pane_dormant(pane_id);
        }
    }
}

/// How long a server must be clientless and childless before it may exit.
pub(crate) const IDLE_SERVER_EXIT_GRACE: std::time::Duration =
    std::time::Duration::from_secs(30 * 60);

/// How often the dormancy sweep looks at the session at all.
pub(crate) const DORMANCY_SWEEP_INTERVAL: std::time::Duration = std::time::Duration::from_secs(60);

/// How long a pane must have been retired before quiet time alone puts it to
/// sleep. Memory pressure bypasses this, never the child-exited guard.
pub(crate) const DORMANCY_QUIET_THRESHOLD: std::time::Duration =
    std::time::Duration::from_secs(24 * 60 * 60);

/// Whether the machine is short enough on memory to sleep candidates early.
///
/// Reads the kernel's PSI accounting; on platforms without it the answer is
/// simply "no pressure", leaving the quiet threshold as the only trigger.
fn memory_pressure_is_high() -> bool {
    #[cfg(target_os = "linux")]
    {
        std::fs::read_to_string("/proc/pressure/memory")
            .ok()
            .and_then(|psi| parse_psi_some_avg60(&psi))
            .is_some_and(|avg60| avg60 > 10.0)
    }
    #[cfg(not(target_os = "linux"))]
    false
}

/// The `avg60` field of the `some` line of a PSI file.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn parse_psi_some_avg60(psi: &str) -> Option<f64> {
    psi.lines()
        .find(|line| line.starts_with("some "))?
        .split_whitespace()
        .find_map(|field| field.strip_prefix("avg60="))
        .and_then(|value| value.parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_app() -> App {
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = App::new(
            &crate::config::Config::default(),
            true,
            None,
            api_rx,
            crate::api::EventHub::default(),
        );
        app.dormant_dir = Some(unique_dormant_dir());
        app
    }

    fn unique_dormant_dir() -> PathBuf {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        std::env::temp_dir().join(format!("herdr-dormant-test-{}-{stamp}", std::process::id()))
    }

    fn app_with_scrollback_pane(bytes: &[u8], child_exited: bool) -> (App, crate::layout::PaneId) {
        let mut app = test_app();
        let workspace = crate::workspace::Workspace::test_new("dormancy");
        let pane_id = workspace.tabs[0].root_pane;
        let terminal_id = workspace.terminal_id(pane_id).cloned().unwrap();
        app.state.workspaces = vec![workspace];
        app.state.ensure_test_terminals();
        let mut runtime =
            crate::terminal::TerminalRuntime::test_with_scrollback_bytes(80, 24, 1 << 20, bytes);
        if child_exited {
            runtime.test_mark_child_exited();
        }
        app.terminal_runtimes.insert(terminal_id, runtime);
        (app, pane_id)
    }

    fn terminal_id_of(app: &App, pane_id: crate::layout::PaneId) -> crate::terminal::TerminalId {
        app.state.workspaces[0]
            .terminal_id(pane_id)
            .cloned()
            .unwrap()
    }

    #[tokio::test]
    async fn a_pane_with_a_live_child_is_refused_dormancy() {
        // TP-DORMANT-01: herdr exists so agents can run for days unattended.
        // Dormancy that can reach a live child is not a resource optimisation,
        // it is the product's core promise breaking.
        let (mut app, pane_id) = app_with_scrollback_pane(b"still-working\r\n", false);
        let terminal_id = terminal_id_of(&app, pane_id);
        app.terminal_runtimes
            .get(&terminal_id)
            .unwrap()
            .test_set_child_pid(std::process::id());

        let refused = app.make_pane_dormant(pane_id);

        assert_eq!(refused, Err(DormancyRefusal::ChildStillRunning));
        assert!(
            app.terminal_runtimes.get(&terminal_id).is_some(),
            "a refused pane keeps its runtime"
        );
        assert!(app
            .state
            .terminals
            .get(&terminal_id)
            .unwrap()
            .dormant
            .is_none());
    }

    #[tokio::test]
    async fn a_retired_unwatched_pane_goes_dormant_and_releases_its_runtime() {
        // TP-DORMANT-02: the release is the whole point — the runtime leaves
        // the registry (PTY actor, reader, scrollback memory go with it) while
        // the pane, its terminal identity, and a wake recipe stay.
        let (mut app, pane_id) = app_with_scrollback_pane(b"dormant-history-marker\r\n", true);
        let terminal_id = terminal_id_of(&app, pane_id);

        app.make_pane_dormant(pane_id).expect("dormancy accepted");

        assert!(
            app.terminal_runtimes.get(&terminal_id).is_none(),
            "the runtime has to leave the registry"
        );
        assert!(
            app.find_pane(pane_id).is_some(),
            "the pane itself stays in the session"
        );
        let dormant = app
            .state
            .terminals
            .get(&terminal_id)
            .unwrap()
            .dormant
            .clone()
            .expect("the terminal carries the dormant recipe");
        let history_path = dormant.history_path.expect("scrollback went to disk");
        let saved = std::fs::read_to_string(&history_path).unwrap();
        assert!(
            saved.contains("dormant-history-marker"),
            "the file holds the scrollback the runtime was the only holder of"
        );
        let _ = std::fs::remove_dir_all(app.dormant_history_dir());
    }

    #[tokio::test]
    async fn a_watched_pane_is_refused_dormancy() {
        // A display is looking at the pane; releasing the runtime under a
        // viewer blanks their screen no matter how retired the child is.
        let (mut app, pane_id) = app_with_scrollback_pane(b"watched\r\n", true);
        let terminal_id = terminal_id_of(&app, pane_id);
        app.state.workspaces[0].active_tab_by_client.insert(7, 0);

        let refused = app.make_pane_dormant(pane_id);

        assert_eq!(refused, Err(DormancyRefusal::Watched));
        assert!(app.terminal_runtimes.get(&terminal_id).is_some());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn closing_a_dormant_pane_deletes_its_history_file() {
        // TP-DORMANT-08 (GAP-A): the wake path and the restore path both
        // consume the file; the close path used to drop the TerminalState and
        // leave the file forever. A days-open laptop accumulates exactly these.
        // Two panes: a workspace refuses to close its last pane, so the
        // dormant one under test is the second.
        let (mut app, _root) = app_with_scrollback_pane(b"root\r\n", false);
        let pane_id = app.state.workspaces[0].test_split(ratatui::layout::Direction::Horizontal);
        let terminal_id = terminal_id_of(&app, pane_id);
        app.state.ensure_test_terminals();
        let mut runtime = crate::terminal::TerminalRuntime::test_with_scrollback_bytes(
            80,
            24,
            1 << 20,
            b"leak-check\r\n",
        );
        runtime.test_mark_child_exited();
        app.terminal_runtimes.insert(terminal_id.clone(), runtime);
        app.make_pane_dormant(pane_id).expect("dormancy accepted");
        let history_path = app
            .state
            .terminals
            .get(&terminal_id)
            .unwrap()
            .dormant
            .clone()
            .unwrap()
            .history_path
            .unwrap();
        assert!(history_path.exists());

        app.state.workspaces[0].close_pane(pane_id);
        app.state
            .remove_unattached_terminal_ids([terminal_id.clone()]);
        app.sync_dormant_history_removals();

        assert!(
            !app.state.terminals.contains_key(&terminal_id),
            "the terminal state is gone with the pane"
        );
        assert!(
            !history_path.exists(),
            "a closed dormant pane's history file has to go with it"
        );
        let _ = std::fs::remove_dir_all(app.dormant_history_dir());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_failed_dormant_write_leaves_no_partial_file() {
        // TP-DORMANT-09 (GAP-B): fs::write starts with truncate, so a crash
        // mid-write leaves a torn file the wake replays as garbage. The write
        // goes through tmp + fsync + rename; a failure leaves the target
        // untouched and the refusal keeps the runtime.
        let (mut app, pane_id) = app_with_scrollback_pane(b"atomic\r\n", true);
        let terminal_id = terminal_id_of(&app, pane_id);
        let dir = app.dormant_history_dir();
        std::fs::create_dir_all(&dir).unwrap();
        let mut perms = std::fs::metadata(&dir).unwrap().permissions();
        std::os::unix::fs::PermissionsExt::set_mode(&mut perms, 0o555);
        std::fs::set_permissions(&dir, perms.clone()).unwrap();

        let refused = app.make_pane_dormant(pane_id);

        assert_eq!(refused, Err(DormancyRefusal::HistoryWriteFailed));
        assert!(
            app.terminal_runtimes.get(&terminal_id).is_some(),
            "a refused pane keeps its runtime"
        );
        std::os::unix::fs::PermissionsExt::set_mode(&mut perms, 0o755);
        std::fs::set_permissions(&dir, perms).unwrap();
        let leftovers: Vec<_> = std::fs::read_dir(&dir)
            .map(|entries| entries.flatten().map(|e| e.file_name()).collect())
            .unwrap_or_default();
        assert!(
            leftovers.is_empty(),
            "no partial or temp file may survive a failed write, got {leftovers:?}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn an_alt_screen_retired_pane_sleeps_with_its_primary_history() {
        // TP-DORMANT-10: this used to be a HistoryUnavailable refusal —
        // capture could only read the active screen. With vendor patch 0002
        // the capture reads the primary screen directly, so the pane sleeps
        // carrying exactly the history the user would scroll back to, and the
        // alternate frame must not leak into the file.
        let (mut app, pane_id) =
            app_with_scrollback_pane(b"primary-history\r\n\x1b[?1049halt-screen", true);
        let terminal_id = terminal_id_of(&app, pane_id);

        app.make_pane_dormant(pane_id)
            .expect("an alt-screen pane sleeps with its primary history");

        let dormant = app
            .state
            .terminals
            .get(&terminal_id)
            .unwrap()
            .dormant
            .clone()
            .expect("dormant marker set");
        let path = dormant.history_path.expect("primary history written");
        let written = std::fs::read_to_string(&path).expect("history file readable");
        assert!(written.contains("primary-history"));
        assert!(!written.contains("alt-screen"));
        assert!(app.terminal_runtimes.get(&terminal_id).is_none());
    }

    fn public_pane_id_of(app: &App, pane_id: crate::layout::PaneId) -> String {
        app.public_pane_id(0, pane_id).expect("public pane id")
    }

    fn sleep_response(app: &mut App, public_id: &str) -> serde_json::Value {
        let encoded = app.handle_pane_sleep(
            "req_sleep".into(),
            crate::api::schema::PaneTarget {
                pane_id: public_id.to_string(),
            },
        );
        serde_json::from_str(&encoded).expect("valid response json")
    }

    #[tokio::test]
    async fn the_pane_sleep_verb_puts_a_retired_unwatched_pane_to_sleep() {
        // TP-DORMANT-13: the API verb is the same policy gate as the sweep —
        // it goes through make_pane_dormant, so every refusal the sweep obeys
        // binds an API caller too, and success returns the pane's fresh info
        // with the dormant field set.
        let (mut app, pane_id) = app_with_scrollback_pane(b"verb-history\r\n", true);
        let terminal_id = terminal_id_of(&app, pane_id);
        let public_id = public_pane_id_of(&app, pane_id);

        let response = sleep_response(&mut app, &public_id);

        assert_eq!(
            response["result"]["pane"]["dormant"],
            serde_json::Value::Bool(true),
            "success returns the pane info carrying dormant: {response}"
        );
        assert!(
            app.terminal_runtimes.get(&terminal_id).is_none(),
            "the runtime left the registry"
        );
        assert!(app
            .state
            .terminals
            .get(&terminal_id)
            .unwrap()
            .dormant
            .is_some());
        let _ = std::fs::remove_dir_all(app.dormant_history_dir());
    }

    #[tokio::test]
    async fn the_pane_sleep_verb_never_reaches_a_live_child() {
        // TP-DORMANT-13: TP-DORMANT-01's unbendable rule holds on the API
        // path — a caller cannot sleep a pane whose child still runs.
        let (mut app, pane_id) = app_with_scrollback_pane(b"still-working\r\n", false);
        let terminal_id = terminal_id_of(&app, pane_id);
        app.terminal_runtimes
            .get(&terminal_id)
            .unwrap()
            .test_set_child_pid(std::process::id());
        let public_id = public_pane_id_of(&app, pane_id);

        let response = sleep_response(&mut app, &public_id);

        assert_eq!(
            response["error"]["code"],
            serde_json::Value::String("child_still_running".into()),
            "the refusal surfaces as an API error: {response}"
        );
        assert!(
            app.terminal_runtimes.get(&terminal_id).is_some(),
            "a refused pane keeps its runtime"
        );
    }

    #[tokio::test]
    async fn the_pane_sleep_verb_refuses_a_watched_pane() {
        // TP-DORMANT-13: the second unbendable rule — a watched pane cannot
        // be blanked under its viewer, not even by an explicit API request.
        let (mut app, pane_id) = app_with_scrollback_pane(b"watched\r\n", true);
        app.state.workspaces[0].active_tab_by_client.insert(7, 0);
        let public_id = public_pane_id_of(&app, pane_id);

        let response = sleep_response(&mut app, &public_id);

        assert_eq!(
            response["error"]["code"],
            serde_json::Value::String("pane_watched".into()),
            "the refusal surfaces as an API error: {response}"
        );
    }

    #[tokio::test]
    async fn the_pane_sleep_verb_is_idempotent_on_a_dormant_pane() {
        // TP-DORMANT-13: a retried sleep is not an error and not a second
        // transition — the pane's state, its history file, and the event
        // stream all stay exactly as the first sleep left them.
        let (mut app, pane_id) = app_with_scrollback_pane(b"idempotent\r\n", true);
        let terminal_id = terminal_id_of(&app, pane_id);
        let public_id = public_pane_id_of(&app, pane_id);

        let first = sleep_response(&mut app, &public_id);
        assert_eq!(first["result"]["pane"]["dormant"], true, "{first}");
        let history_path = app
            .state
            .terminals
            .get(&terminal_id)
            .unwrap()
            .dormant
            .clone()
            .unwrap()
            .history_path
            .expect("history written");
        let saved = std::fs::read_to_string(&history_path).unwrap();
        let events_after_first = app.event_hub.current_sequence();

        let second = sleep_response(&mut app, &public_id);

        assert_eq!(
            second["result"]["pane"]["dormant"],
            serde_json::Value::Bool(true),
            "a repeated sleep is a success, not an error: {second}"
        );
        assert_eq!(
            std::fs::read_to_string(&history_path).unwrap(),
            saved,
            "the history file is untouched"
        );
        assert_eq!(
            app.event_hub.current_sequence(),
            events_after_first,
            "no transition happened, so no event is emitted"
        );
        let _ = std::fs::remove_dir_all(app.dormant_history_dir());
    }

    #[tokio::test]
    async fn the_pane_sleep_verb_reports_an_unknown_pane() {
        // The error contract stays uniform with every other pane verb.
        let mut app = test_app();
        let response = sleep_response(&mut app, "p_missing");
        assert_eq!(
            response["error"]["code"],
            serde_json::Value::String("pane_not_found".into()),
            "{response}"
        );
    }

    fn pane_updated_after(
        app: &App,
        sequence: u64,
    ) -> Option<crate::api::schema::PaneInfo> {
        app.event_hub
            .events_after(sequence)
            .into_iter()
            .find_map(|(_, envelope)| match envelope.data {
                crate::api::schema::EventData::PaneUpdated { pane } => Some(pane),
                _ => None,
            })
    }

    #[tokio::test]
    async fn the_pane_sleep_verb_announces_the_transition() {
        // TP-DORMANT-14: the field is only half the observability — a watcher
        // subscribed to pane.updated must hear the transition, not discover
        // it on its next poll.
        let (mut app, pane_id) = app_with_scrollback_pane(b"announce\r\n", true);
        let public_id = public_pane_id_of(&app, pane_id);
        let seq_before = app.event_hub.current_sequence();

        let response = sleep_response(&mut app, &public_id);

        assert_eq!(response["result"]["pane"]["dormant"], true, "{response}");
        let announced = pane_updated_after(&app, seq_before)
            .expect("the sleep transition emits pane.updated");
        assert!(announced.dormant, "the announced pane carries the new state");
        let _ = std::fs::remove_dir_all(app.dormant_history_dir());
    }

    #[tokio::test]
    async fn the_dormancy_sweep_announces_each_pane_it_sleeps() {
        // TP-DORMANT-14: the sweep is the transition nobody asked for — if it
        // stays silent, an API watcher's picture of the session quietly rots.
        let (mut app, _pane_id) = app_with_scrollback_pane(b"swept\r\n", true);
        let seq_before = app.event_hub.current_sequence();

        let changed = app.dormancy_sweep_with_pressure(std::time::Instant::now(), true);

        assert!(changed, "the sweep slept the retired pane");
        let announced = pane_updated_after(&app, seq_before)
            .expect("the sweep transition emits pane.updated");
        assert!(announced.dormant);
        let _ = std::fs::remove_dir_all(app.dormant_history_dir());
    }

    #[tokio::test]
    async fn waking_a_dormant_pane_announces_it() {
        // TP-DORMANT-14: the wake is the other direction of the same promise;
        // the announced pane is awake again, dormant field gone.
        let (mut app, pane_id) = app_with_scrollback_pane(b"wake-announce\r\n", true);
        app.make_pane_dormant(pane_id).expect("dormancy accepted");
        let seq_before = app.event_hub.current_sequence();

        assert!(app.wake_dormant_pane(pane_id), "the pane woke");

        let announced = pane_updated_after(&app, seq_before)
            .expect("the wake transition emits pane.updated");
        assert!(!announced.dormant, "the announced pane is awake again");
        let _ = std::fs::remove_dir_all(app.dormant_history_dir());
    }

    #[tokio::test]
    async fn a_dormant_panes_api_info_says_so_and_an_awake_panes_does_not() {
        // TP-DORMANT-14: without this field dormancy is invisible to API
        // clients — a watcher polling the snapshot cannot tell a sleeping
        // pane from an awake one, and the lifecycle runs unobserved. The
        // false case stays unserialized so an awake pane's JSON is unchanged.
        let (mut app, pane_id) = app_with_scrollback_pane(b"sleepy\r\n", true);

        let awake = app.pane_info(0, pane_id).expect("awake pane info");
        assert!(!awake.dormant, "an awake pane is not dormant");
        let awake_json = serde_json::to_value(&awake).unwrap();
        assert!(
            awake_json.get("dormant").is_none(),
            "false is not serialized, keeping the awake pane's JSON unchanged"
        );

        app.make_pane_dormant(pane_id).expect("dormancy accepted");

        let dormant = app.pane_info(0, pane_id).expect("dormant pane info");
        assert!(dormant.dormant, "a dormant pane says so");
        let dormant_json = serde_json::to_value(&dormant).unwrap();
        assert_eq!(
            dormant_json.get("dormant"),
            Some(&serde_json::Value::Bool(true))
        );
        let _ = std::fs::remove_dir_all(app.dormant_history_dir());
    }

    #[tokio::test]
    async fn an_alt_screen_panes_api_info_says_so_and_a_primary_panes_does_not() {
        // TP-DORMANT-14: the alternate_screen field lets an API watcher tell
        // "no scrollback yet" from "a fullscreen app owns the viewport" — the
        // intent form of the scroll guard, readable without a screen capture.
        let (app, pane_id) = app_with_scrollback_pane(b"primary\r\n\x1b[?1049halt", true);
        let info = app.pane_info(0, pane_id).expect("alt-screen pane info");
        assert!(info.alternate_screen, "the alternate screen is visible");
        let json = serde_json::to_value(&info).unwrap();
        assert_eq!(
            json.get("alternate_screen"),
            Some(&serde_json::Value::Bool(true))
        );

        let (app, pane_id) = app_with_scrollback_pane(b"primary-only\r\n", true);
        let info = app.pane_info(0, pane_id).expect("primary pane info");
        assert!(!info.alternate_screen);
        let json = serde_json::to_value(&info).unwrap();
        assert!(
            json.get("alternate_screen").is_none(),
            "false is not serialized"
        );
    }

    #[tokio::test]
    async fn an_alt_screen_pane_with_an_empty_primary_sleeps_without_a_file() {
        // The empty-pane contract survives the TP-DORMANT-10 semantics
        // change: a pane that went fullscreen without ever writing to the
        // primary screen has nothing to carry, so it keeps its fileless sleep.
        let (mut app, pane_id) = app_with_scrollback_pane(b"\x1b[?1049halt-screen", true);
        let terminal_id = terminal_id_of(&app, pane_id);

        app.make_pane_dormant(pane_id)
            .expect("empty-primary alt-screen pane sleeps");

        let dormant = app
            .state
            .terminals
            .get(&terminal_id)
            .unwrap()
            .dormant
            .clone()
            .expect("dormant marker set");
        assert!(dormant.history_path.is_none(), "nothing to write, no file");
        assert!(app.terminal_runtimes.get(&terminal_id).is_none());
    }

    #[tokio::test]
    async fn an_empty_retired_pane_still_sleeps_without_a_file() {
        // The alt-screen refusal must not catch the empty pane: nothing to
        // lose means fileless sleep stays allowed — this is the whole
        // empty-pane goal.
        let (mut app, pane_id) = app_with_scrollback_pane(b"", true);
        let terminal_id = terminal_id_of(&app, pane_id);

        app.make_pane_dormant(pane_id).expect("empty pane sleeps");

        let dormant = app
            .state
            .terminals
            .get(&terminal_id)
            .unwrap()
            .dormant
            .clone()
            .expect("dormant marker set");
        assert!(dormant.history_path.is_none(), "nothing to write, no file");
        assert!(app.terminal_runtimes.get(&terminal_id).is_none());
    }

    #[tokio::test]
    async fn a_popup_pane_cannot_become_dormant() {
        // GAP-A's popup half, resolved by test: popup panes live outside the
        // workspaces, so find_pane never resolves them and no popup can carry
        // a dormant marker — there is no popup file to leak.
        let (mut app, _pane_id) = app_with_scrollback_pane(b"popup\r\n", true);
        let popup_pane = crate::layout::PaneId::from_raw(999_999);
        app.state.popup_pane = Some(crate::app::state::PopupPaneState {
            pane_id: popup_pane,
            terminal_id: crate::terminal::TerminalId::alloc(),
            width: None,
            height: None,
        });

        assert_eq!(
            app.make_pane_dormant(popup_pane),
            Err(DormancyRefusal::NoSuchPane)
        );
    }

    #[tokio::test]
    async fn the_sweep_waits_out_the_quiet_threshold_without_memory_pressure() {
        // TP-DORMANT-05: quiet time is a candidate filter, not a race. A pane
        // that exited a minute ago may be mid-workflow (a respawn, a user
        // about to click); only a day of silence or real memory pressure
        // justifies taking its runtime.
        let (mut app, pane_id) = app_with_scrollback_pane(b"policy\r\n", true);
        let terminal_id = terminal_id_of(&app, pane_id);
        app.pane_dormancy_enabled = true;
        let start = std::time::Instant::now();

        assert!(!app.dormancy_sweep_with_pressure(start, false));
        assert!(
            app.terminal_runtimes.get(&terminal_id).is_some(),
            "a freshly retired pane is only marked, not slept"
        );

        let later = start + DORMANCY_QUIET_THRESHOLD + std::time::Duration::from_secs(1);
        assert!(app.dormancy_sweep_with_pressure(later, false));
        assert!(
            app.terminal_runtimes.get(&terminal_id).is_none(),
            "a day-quiet retired pane sleeps"
        );
        let _ = std::fs::remove_dir_all(app.dormant_history_dir());
    }

    #[tokio::test]
    async fn memory_pressure_sleeps_a_retired_pane_without_waiting() {
        // K3-Q1: pressure is the primary trigger — the machine needs the
        // memory now, and a retired pane is the one thing that can give it
        // back without breaking anything.
        let (mut app, pane_id) = app_with_scrollback_pane(b"pressure\r\n", true);
        let terminal_id = terminal_id_of(&app, pane_id);
        app.pane_dormancy_enabled = true;

        assert!(app.dormancy_sweep_with_pressure(std::time::Instant::now(), true));
        assert!(app.terminal_runtimes.get(&terminal_id).is_none());
        let _ = std::fs::remove_dir_all(app.dormant_history_dir());
    }

    #[tokio::test]
    async fn the_sweep_never_reaches_a_live_pane_even_under_pressure() {
        // The one rule pressure cannot bend: a live child is live work.
        let (mut app, pane_id) = app_with_scrollback_pane(b"alive\r\n", false);
        let terminal_id = terminal_id_of(&app, pane_id);
        app.terminal_runtimes
            .get(&terminal_id)
            .unwrap()
            .test_set_child_pid(std::process::id());
        app.pane_dormancy_enabled = true;

        let far_future = std::time::Instant::now() + DORMANCY_QUIET_THRESHOLD * 2;
        assert!(!app.dormancy_sweep_with_pressure(far_future, true));
        assert!(app.terminal_runtimes.get(&terminal_id).is_some());
        let _ = pane_id;
    }

    #[tokio::test]
    async fn a_server_with_a_live_child_never_retires() {
        // TP-SRV-RETIRE-01: the refuted 2026-08-15 draft ("server lives iff a
        // direct child exists") would have killed a live session whose panes
        // were reparented by handoff. The correct signal is the pane ledger:
        // any runtime whose child is alive keeps the whole server alive.
        let (mut app, _pane_id) = app_with_scrollback_pane(b"alive\r\n", false);
        let terminal_id = terminal_id_of(&app, _pane_id);
        app.terminal_runtimes
            .get(&terminal_id)
            .unwrap()
            .test_set_child_pid(std::process::id());
        app.idle_server_exit_enabled = true;

        let start = std::time::Instant::now();
        assert!(!app.server_retirement_due(start, false));
        let far = start + IDLE_SERVER_EXIT_GRACE * 3;
        assert!(
            !app.server_retirement_due(far, false),
            "a live child blocks retirement no matter how long the clock runs"
        );
    }

    #[tokio::test]
    async fn an_attached_client_resets_the_retirement_clock() {
        // TP-SRV-RETIRE-02: attaching is the strongest possible signal the
        // session is wanted; the idle clock starts over from zero.
        let (mut app, _pane_id) = app_with_scrollback_pane(b"quiet\r\n", true);
        app.idle_server_exit_enabled = true;

        let start = std::time::Instant::now();
        assert!(!app.server_retirement_due(start, false), "clock starts");
        let mid = start + IDLE_SERVER_EXIT_GRACE / 2;
        app.next_retirement_check_at = None;
        assert!(!app.server_retirement_due(mid, true), "a client attaches");
        let after = mid + IDLE_SERVER_EXIT_GRACE / 2 + std::time::Duration::from_secs(61);
        app.next_retirement_check_at = None;
        assert!(
            !app.server_retirement_due(after, false),
            "the old idle time does not count; the clock restarted at detach"
        );
    }

    #[tokio::test]
    async fn a_childless_clientless_server_retires_after_the_grace() {
        // TP-SRV-RETIRE-03: 45 orphan servers were measured burning 30.6
        // hours of CPU drawing frames nobody saw. A session whose every child
        // has exited and that nobody has attached to for the grace period has
        // nothing left that disk cannot hold.
        let (mut app, _pane_id) = app_with_scrollback_pane(b"done\r\n", true);
        app.idle_server_exit_enabled = true;

        let start = std::time::Instant::now();
        assert!(
            !app.server_retirement_due(start, false),
            "grace not yet served"
        );
        let due = start + IDLE_SERVER_EXIT_GRACE + std::time::Duration::from_secs(1);
        app.next_retirement_check_at = None;
        assert!(app.server_retirement_due(due, false));
    }

    #[test]
    fn psi_some_avg60_parses_the_kernel_format() {
        let psi = "some avg10=0.00 avg60=12.50 avg300=1.32 total=123456\n\
                   full avg10=0.00 avg60=0.00 avg300=0.00 total=0\n";
        assert_eq!(parse_psi_some_avg60(psi), Some(12.5));
        assert_eq!(parse_psi_some_avg60(""), None);
    }

    fn persisted_claude_session() -> crate::agent_resume::PersistedAgentSession {
        crate::agent_resume::PersistedAgentSession {
            source: "herdr:claude".into(),
            agent: "claude".into(),
            session_ref: crate::agent_resume::AgentSessionRef::id(
                "f6774263-51c5-460c-9c0d-b6fc9c38c756",
            )
            .expect("valid session id"),
        }
    }

    #[tokio::test]
    async fn waking_a_dormant_agent_pane_queues_its_resume_and_drops_the_replay() {
        // TP-DORMANT-12: the click protocol's second half. A pane that carried
        // an agent session wakes into the cold-restore resume machinery — the
        // plan is queued, no bare shell is spawned here — and the saved
        // scrollback is deleted unreplayed, faithful to cold restore: the
        // resumed agent repaints its own transcript, and a replay would
        // double it.
        let (mut app, pane_id) = app_with_scrollback_pane(b"agent transcript\r\n", true);
        let terminal_id = terminal_id_of(&app, pane_id);
        app.state
            .terminals
            .get_mut(&terminal_id)
            .unwrap()
            .persisted_agent_session = Some(persisted_claude_session());
        app.make_pane_dormant(pane_id).expect("dormancy accepted");
        let history_path = app
            .state
            .terminals
            .get(&terminal_id)
            .unwrap()
            .dormant
            .clone()
            .unwrap()
            .history_path
            .expect("history written at sleep");

        assert!(app.wake_dormant_pane(pane_id), "the pane wakes");

        let terminal = app.state.terminals.get(&terminal_id).unwrap();
        assert!(terminal.dormant.is_none(), "waking clears the recipe");
        let plan = terminal
            .pending_agent_resume_plan
            .clone()
            .expect("the resume plan is queued for the restore machinery");
        assert_eq!(
            plan.argv,
            vec![
                "claude".to_string(),
                "--resume".to_string(),
                "f6774263-51c5-460c-9c0d-b6fc9c38c756".to_string()
            ]
        );
        assert!(
            app.terminal_runtimes.get(&terminal_id).is_none(),
            "the resume path spawns nothing itself; the resume machinery owns the launch"
        );
        assert!(
            !history_path.exists(),
            "the resume path deletes the saved scrollback unreplayed"
        );
    }

    #[tokio::test]
    async fn an_agent_session_without_a_resume_plan_wakes_into_the_shell_path() {
        // Fail-safe: an agent source the resume table does not know cannot be
        // relaunched, so the pane falls back to the shell path with its
        // history replayed — neither the process class nor the scrollback is
        // silently dropped.
        let (mut app, pane_id) = app_with_scrollback_pane(b"unknown-agent-history\r\n", true);
        let terminal_id = terminal_id_of(&app, pane_id);
        app.state
            .terminals
            .get_mut(&terminal_id)
            .unwrap()
            .persisted_agent_session = Some(crate::agent_resume::PersistedAgentSession {
            source: "herdr:claude".into(),
            // The source/agent pair does not match any resume recipe.
            agent: "mystery".into(),
            session_ref: crate::agent_resume::AgentSessionRef::id(
                "f6774263-51c5-460c-9c0d-b6fc9c38c756",
            )
            .expect("valid session id"),
        });
        app.make_pane_dormant(pane_id).expect("dormancy accepted");

        assert!(app.wake_dormant_pane(pane_id), "the pane wakes");

        let runtime = app
            .terminal_runtimes
            .get(&terminal_id)
            .expect("the shell path rebuilds the runtime");
        let replayed = runtime.recent_unwrapped_ansi_snapshot(10_000).text;
        assert!(replayed.contains("unknown-agent-history"));
        assert!(app
            .state
            .terminals
            .get(&terminal_id)
            .unwrap()
            .pending_agent_resume_plan
            .is_none());
    }

    #[tokio::test]
    async fn a_watched_tab_wake_uses_the_resume_path_for_an_agent_pane() {
        // The two wake funnels must not diverge: looking at a tab and
        // clicking a pane are the same touch protocol, so both end in the
        // resume path for an agent pane. TP-DORMANT-12
        let (mut app, pane_id) = app_with_scrollback_pane(b"agent transcript\r\n", true);
        let terminal_id = terminal_id_of(&app, pane_id);
        app.state
            .terminals
            .get_mut(&terminal_id)
            .unwrap()
            .persisted_agent_session = Some(persisted_claude_session());
        app.make_pane_dormant(pane_id).expect("dormancy accepted");
        app.state.workspaces[0].active_tab_by_client.insert(7, 0);

        assert!(app.wake_dormant_panes_on_watched_tabs());

        let terminal = app.state.terminals.get(&terminal_id).unwrap();
        assert!(terminal.dormant.is_none());
        assert!(terminal.pending_agent_resume_plan.is_some());
        assert!(app.terminal_runtimes.get(&terminal_id).is_none());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn an_alt_screen_agent_pane_sleeps_with_history_and_wakes_into_a_resume() {
        // The intersection of the two sleep paths, owned by name: an agent
        // pane caught on the alternate screen sleeps carrying its primary
        // history (TP-DORMANT-10), and its wake still takes the resume path —
        // the file is deleted unreplayed like every resume wake, faithful to
        // cold restore, which drops even the pre-agent scrollback.
        // TP-DORMANT-12
        let (mut app, pane_id) =
            app_with_scrollback_pane(b"pre-agent output\r\n\x1b[?1049halt-frame", true);
        let terminal_id = terminal_id_of(&app, pane_id);
        app.state
            .terminals
            .get_mut(&terminal_id)
            .unwrap()
            .persisted_agent_session = Some(persisted_claude_session());
        app.make_pane_dormant(pane_id)
            .expect("an alt-screen pane sleeps with its primary history");
        let history_path = app
            .state
            .terminals
            .get(&terminal_id)
            .unwrap()
            .dormant
            .clone()
            .unwrap()
            .history_path
            .expect("primary history written at sleep");
        assert!(history_path.exists());

        assert!(app.wake_dormant_pane(pane_id), "the pane wakes");

        let terminal = app.state.terminals.get(&terminal_id).unwrap();
        assert!(terminal.pending_agent_resume_plan.is_some());
        assert!(app.terminal_runtimes.get(&terminal_id).is_none());
        assert!(
            !history_path.exists(),
            "the resume wake deletes the file unreplayed"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_watched_tab_wakes_its_dormant_panes() {
        // TP-DORMANT-04: the touch protocol's chokepoint. Attach, tab switch,
        // workspace switch, and focus all end with the tab being watched;
        // waking there covers every path without instrumenting each one.
        let (mut app, pane_id) = app_with_scrollback_pane(b"dormant-history-marker\r\n", true);
        let terminal_id = terminal_id_of(&app, pane_id);
        app.make_pane_dormant(pane_id).expect("dormancy accepted");
        assert!(
            !app.wake_dormant_panes_on_watched_tabs(),
            "an unwatched dormant pane stays dormant"
        );

        app.state.workspaces[0].active_tab_by_client.insert(7, 0);

        assert!(
            app.wake_dormant_panes_on_watched_tabs(),
            "the watched pane wakes"
        );
        assert!(app.terminal_runtimes.get(&terminal_id).is_some());
        assert!(app
            .state
            .terminals
            .get(&terminal_id)
            .unwrap()
            .dormant
            .is_none());
        let _ = std::fs::remove_dir_all(app.dormant_history_dir());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn waking_a_dormant_pane_restores_its_history() {
        // TP-DORMANT-03: dormancy without faithful waking is deferred data
        // loss. The woken pane is a fresh shell, but the scrollback above it
        // is the one the user left.
        let (mut app, pane_id) = app_with_scrollback_pane(b"dormant-history-marker\r\n", true);
        let terminal_id = terminal_id_of(&app, pane_id);
        app.make_pane_dormant(pane_id).expect("dormancy accepted");
        let history_path = app
            .state
            .terminals
            .get(&terminal_id)
            .unwrap()
            .dormant
            .clone()
            .unwrap()
            .history_path
            .unwrap();

        assert!(app.wake_dormant_pane(pane_id), "the pane wakes");

        let runtime = app
            .terminal_runtimes
            .get(&terminal_id)
            .expect("waking rebuilds the runtime");
        let replayed = runtime.recent_unwrapped_ansi_snapshot(10_000).text;
        assert!(
            replayed.contains("dormant-history-marker"),
            "the woken pane replays the saved scrollback, got {replayed:?}"
        );
        assert!(
            app.state
                .terminals
                .get(&terminal_id)
                .unwrap()
                .dormant
                .is_none(),
            "waking clears the recipe"
        );
        assert!(
            !history_path.exists(),
            "the history file is consumed by the wake"
        );
        assert!(
            !app.wake_dormant_pane(pane_id),
            "waking an awake pane is a no-op"
        );
        let _ = std::fs::remove_dir_all(app.dormant_history_dir());
    }
}
