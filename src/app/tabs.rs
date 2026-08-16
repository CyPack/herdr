//! Opening a command in a new tab of the current workspace.
//!
//! One place, because the same seven steps of bookkeeping already exist twice —
//! `projects.rs` opens a project chat this way and `api/plugins/panes.rs` opens
//! a plugin pane this way. Both are correct and neither is wrong enough to turn
//! a test red if the third copy drifted, which is exactly the failure a shared
//! answer prevents.
//!
//! The bookkeeping is now two functions rather than one, because the callers do
//! not all do the same thing between its halves: a project chat names its tab
//! and records a resumed session before anything takes focus. Registering and
//! announcing are therefore separate, and what falls between them belongs to
//! whoever is calling.
//!
//! The plugin path stays outside both for now. It reports failure as a wire
//! error rather than a `Result`, registers plugin ownership of its own, and can
//! open a tab without focusing it; folding it in would put three unrelated
//! error vocabularies in one function. When it is folded in, that is when the
//! focus question becomes a real choice worth a parameter — inventing the
//! parameter today would only add an arm nothing calls.

use crate::app::App;
use crate::layout::PaneId;
use crate::terminal::{TerminalRuntime, TerminalState};

impl App {
    /// Put a freshly created tab's terminal and runtime into state, and say
    /// which pane is the tab's root.
    ///
    /// The half every caller performs before its own bookkeeping. The root pane
    /// is returned because the announcement needs it and because otherwise each
    /// call site indexes three levels deep to find a value the workspace has
    /// just handed back.
    ///
    /// `None` means the workspace or tab is gone, which is a caller's problem to
    /// report in its own vocabulary rather than this function's to guess at.
    pub(crate) fn register_new_tab_pane(
        &mut self,
        ws_idx: usize,
        tab_idx: usize,
        terminal: TerminalState,
        runtime: TerminalRuntime,
    ) -> Option<PaneId> {
        let root_pane = self
            .state
            .workspaces
            .get(ws_idx)?
            .tabs
            .get(tab_idx)?
            .root_pane;
        self.terminal_runtimes.insert(terminal.id.clone(), runtime);
        // Before the terminal goes in, because an alias that outlived its pane
        // would otherwise be live for the moment in between.
        self.state.remove_alias_shadowed_by_new_pane(root_pane);
        self.state.terminals.insert(terminal.id.clone(), terminal);
        Some(root_pane)
    }

    /// Go to a freshly registered tab, persist the session, and announce both
    /// the tab and its pane.
    ///
    /// Announced in that order: a subscriber that learns about a pane before the
    /// tab holding it has to hold the pane somewhere until the tab arrives, and
    /// nothing about the order costs anything to get right here.
    pub(crate) fn announce_new_tab(&mut self, ws_idx: usize, tab_idx: usize, root_pane: PaneId) {
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
    /// Run `argv` in a new tab of the active workspace, and go there.
    ///
    /// "Full size" needs no zoom: the tab's root pane is the only pane in it, so
    /// it already occupies the whole tab.
    ///
    /// The new tab takes focus. A person who asked to see something *bigger*
    /// and was left where they were would read the gesture as having done
    /// nothing, which is the failure TP-CHROME-44 exists to prevent.
    // TP-CHROME-61: a command opened in a new tab is registered, focused and
    // announced, so every surface that draws tabs sees the same one.
    pub(crate) fn open_argv_in_new_tab(&mut self, argv: &[String]) -> std::io::Result<()> {
        let Some(ws_idx) = self.state.active else {
            return Err(std::io::Error::other("no active workspace"));
        };

        // The same directory the popup presentation would have used, so the two
        // gestures on one section land in the same place. A section that opened
        // btop in the focused pane's directory on the left and in herdr's own
        // on the right would be one control with two meanings.
        let cwd = self
            .state
            .workspaces
            .get(ws_idx)
            .and_then(|workspace| {
                let focused = workspace.focused_pane_id()?;
                let tab = workspace.active_tab()?;
                tab.cwd_for_pane(focused, &self.state.terminals, &self.terminal_runtimes)
            })
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| "/".into()));

        let (rows, cols) = self.state.estimate_pane_size();
        let Some(workspace) = self.state.workspaces.get_mut(ws_idx) else {
            return Err(std::io::Error::other("active workspace disappeared"));
        };
        // The same floors the other two callers use. A pane smaller than this
        // is one no terminal program can draw in.
        let (tab_idx, terminal, runtime) = workspace.create_tab_argv_command(
            rows.max(4),
            cols.max(10),
            cwd,
            argv,
            Vec::new(),
            self.state.pane_scrollback_limit_bytes,
            self.state.host_terminal_theme,
        )?;

        let Some(root_pane) = self.register_new_tab_pane(ws_idx, tab_idx, terminal, runtime) else {
            return Err(std::io::Error::other("the new tab disappeared"));
        };
        self.announce_new_tab(ws_idx, tab_idx, root_pane);
        Ok(())
    }
}
