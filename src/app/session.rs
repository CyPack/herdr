use std::time::{Duration, Instant};

use super::{App, SESSION_SAVE_DEBOUNCE};

enum SessionSaveJob {
    Clear,
    Save {
        snapshot: Box<crate::persist::SessionSnapshot>,
        history: Option<crate::persist::SessionHistorySnapshot>,
    },
}

impl App {
    pub(super) fn schedule_session_save(&mut self) {
        if !self.no_session {
            self.session_save_deadline = Some(Instant::now() + SESSION_SAVE_DEBOUNCE);
        }
    }

    pub(crate) fn sync_session_save_schedule(&mut self) {
        if self.state.session_dirty {
            self.state.session_dirty = false;
            self.schedule_session_save();
        }
    }

    fn reap_finished_session_save(&mut self) {
        if self
            .session_save_thread
            .as_ref()
            .is_some_and(std::thread::JoinHandle::is_finished)
        {
            if let Some(thread) = self.session_save_thread.take() {
                let _ = thread.join();
            }
        }
    }

    fn capture_session_save_job(&self) -> SessionSaveJob {
        if self.state.workspaces.is_empty() {
            SessionSaveJob::Clear
        } else {
            let snapshot = crate::persist::capture(
                &self.state.workspaces,
                &self.state.terminals,
                &self.terminal_runtimes,
                self.state.active,
                self.state.selected,
                self.state.sidebar_width,
                &self.state.shell_presentation,
                self.state.sidebar_section_split,
                self.state.collapsed_space_keys.clone(),
                self.state.files_tab_snapshot(),
            );
            let history = self.persist_pane_history.then(|| {
                crate::persist::capture_history(&self.state.workspaces, &self.terminal_runtimes)
            });
            SessionSaveJob::Save {
                snapshot: Box::new(snapshot),
                history,
            }
        }
    }

    pub(crate) fn start_background_session_save(&mut self) {
        if self.no_session {
            self.session_save_deadline = None;
            return;
        }

        self.reap_finished_session_save();
        if self.session_save_thread.is_some() {
            self.session_save_deadline = Some(Instant::now() + Duration::from_millis(250));
            return;
        }

        let job = self.capture_session_save_job();
        self.session_save_deadline = None;
        self.record_workspace_chats_from(&job);
        match std::thread::Builder::new()
            .name("herdr-session-save".into())
            .spawn(move || run_session_save_job(job))
        {
            Ok(thread) => self.session_save_thread = Some(thread),
            Err(err) => {
                tracing::warn!(err = %err, "failed to spawn session save thread; saving inline");
                let job = self.capture_session_save_job();
                self.record_workspace_chats_from(&job);
                run_session_save_job(job);
            }
        }
    }

    /// Fold this save's snapshot into the workspace chat ledger.
    ///
    /// It rides the session save because that is where the snapshot already
    /// exists and where the debounce already lives — the association is only
    /// worth recording as often as the layout itself is. A ledger write only
    /// happens when a chat is new or its sighting went stale, so a live agent
    /// does not turn every save into a second file write.
    fn record_workspace_chats_from(&mut self, job: &SessionSaveJob) {
        let SessionSaveJob::Save { snapshot, .. } = job else {
            return;
        };
        // `--no-session` means this run leaves nothing on disk. The in-memory
        // ledger still tracks the associations so the surface works during the
        // run; only the write is suppressed. This also keeps unit tests from
        // touching a real config directory.
        let persist = !self.no_session;
        let now_ms = crate::persist::workspace_chats::now_ms();
        let observations = crate::persist::workspace_chats::observe_from_snapshot(snapshot);
        if !self
            .workspace_chat_ledger
            .apply_observations(observations, now_ms)
        {
            return;
        }
        self.sync_workspace_chat_rows();
        if !persist {
            return;
        }
        let path = crate::persist::workspace_chats::default_ledger_path();
        if let Err(err) =
            crate::persist::workspace_chats::save_to_path(&path, &self.workspace_chat_ledger)
        {
            tracing::warn!(path = %path.display(), %err, "failed to save workspace chat ledger");
        }
    }

    /// Project the ledger into the presentation rows the sidebar reads.
    ///
    /// Deliberately not a poll: the ledger only changes when a session save
    /// folds new observations into it, so mirroring it there costs nothing and
    /// avoids adding a scheduled task — scheduled work in this codebase has to
    /// answer the per-display question, and a projection of shared session
    /// history has no business being per-display.
    pub(crate) fn sync_workspace_chat_rows(&mut self) {
        self.state.workspace_chat_rows =
            crate::persist::workspace_chats::project_rows(&self.workspace_chat_ledger);
    }

    pub(crate) fn save_session_now(&mut self) {
        if let Some(thread) = self.session_save_thread.take() {
            let _ = thread.join();
        }

        if self.no_session {
            self.session_save_deadline = None;
            return;
        }

        let job = self.capture_session_save_job();
        self.record_workspace_chats_from(&job);
        run_session_save_job(job);
        self.session_save_deadline = None;
    }
}

fn run_session_save_job(job: SessionSaveJob) {
    match job {
        SessionSaveJob::Clear => crate::persist::clear(),
        SessionSaveJob::Save { snapshot, history } => {
            crate::persist::save(snapshot.as_ref(), history.as_ref());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::Workspace;

    fn test_app() -> App {
        App::new(
            &crate::config::Config::default(),
            true,
            None,
            tokio::sync::mpsc::unbounded_channel().1,
            crate::api::EventHub::default(),
        )
    }

    // TP-WSCHAT-13: the association is recorded through the REAL save funnel,
    // not a hand-built snapshot. A ledger that only works in a unit test would
    // leave the drawer permanently empty in production — the failure mode this
    // whole feature exists to avoid.
    #[test]
    fn a_session_save_folds_the_live_wiring_into_the_ledger() {
        let mut app = test_app();
        let mut workspace = Workspace::test_new("ledger-probe");
        workspace.identity_cwd = std::env::temp_dir();
        let pane_id = workspace.tabs[0].root_pane;
        app.state.workspaces = vec![workspace];
        app.state.active = Some(0);
        app.state.ensure_test_terminals();

        let terminal_id = app.state.workspaces[0]
            .terminal_id(pane_id)
            .cloned()
            .expect("probe pane has a terminal");
        app.state
            .terminals
            .get_mut(&terminal_id)
            .expect("probe terminal exists")
            .set_persisted_agent_session(crate::agent_resume::PersistedAgentSession {
                source: "herdr:claude".into(),
                agent: "claude".into(),
                session_ref: crate::agent_resume::AgentSessionRef::id("probe-session-id")
                    .expect("probe session id is valid"),
            });

        let job = app.capture_session_save_job();
        app.record_workspace_chats_from(&job);

        let key = crate::persist::workspace_chats::ledger_key(&std::env::temp_dir());
        let chats = app.workspace_chat_ledger.chats_for(&key);
        assert_eq!(
            chats.len(),
            1,
            "the save funnel must record the pane's session against its workspace"
        );
        assert_eq!(chats[0].session_id, "probe-session-id");
        assert_eq!(chats[0].agent, "claude");
    }

    // TP-WSCHAT-14: a `--no-session` run leaves nothing on disk. The ledger is
    // still tracked in memory so the surface works during the run, but the
    // write is suppressed — without this every unit test that captures a save
    // would write into the real config directory (observed: a test run created
    // ~/.config/herdr-dev/workspace-chats.json).
    #[test]
    fn a_no_session_run_tracks_chats_in_memory_but_writes_nothing() {
        let mut app = test_app();
        assert!(app.no_session, "control: the test app is a no-session run");
        let mut workspace = Workspace::test_new("no-session-probe");
        workspace.identity_cwd = std::env::temp_dir();
        let pane_id = workspace.tabs[0].root_pane;
        app.state.workspaces = vec![workspace];
        app.state.active = Some(0);
        app.state.ensure_test_terminals();
        let terminal_id = app.state.workspaces[0]
            .terminal_id(pane_id)
            .cloned()
            .expect("probe pane has a terminal");
        app.state
            .terminals
            .get_mut(&terminal_id)
            .expect("probe terminal exists")
            .set_persisted_agent_session(crate::agent_resume::PersistedAgentSession {
                source: "herdr:claude".into(),
                agent: "claude".into(),
                session_ref: crate::agent_resume::AgentSessionRef::id("no-session-probe-id")
                    .expect("probe session id is valid"),
            });

        let ledger_path = crate::persist::workspace_chats::default_ledger_path();
        let existed_before = ledger_path.exists();

        let job = app.capture_session_save_job();
        app.record_workspace_chats_from(&job);

        let key = crate::persist::workspace_chats::ledger_key(&std::env::temp_dir());
        assert!(
            app.workspace_chat_ledger
                .chats_for(&key)
                .iter()
                .any(|chat| chat.session_id == "no-session-probe-id"),
            "the in-memory ledger still tracks the association"
        );
        assert_eq!(
            ledger_path.exists(),
            existed_before,
            "a no-session run must not create or touch the ledger file at {}",
            ledger_path.display()
        );
    }

    // TP-WSCHAT-13: a workspace with no agent must not manufacture a record.
    #[test]
    fn a_session_save_without_any_agent_records_nothing() {
        let mut app = test_app();
        let mut workspace = Workspace::test_new("empty-probe");
        workspace.identity_cwd = std::env::temp_dir();
        app.state.workspaces = vec![workspace];
        app.state.active = Some(0);
        app.state.ensure_test_terminals();

        let job = app.capture_session_save_job();
        app.record_workspace_chats_from(&job);

        assert!(app.workspace_chat_ledger.workspaces.is_empty());
    }
}
