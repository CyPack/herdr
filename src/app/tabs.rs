//! Opening a command in a new tab of the current workspace.
//!
//! One place, because the same seven steps of bookkeeping already exist twice —
//! `projects.rs` opens a project chat this way and `api/plugins/panes.rs` opens
//! a plugin pane this way. Both are correct and neither is wrong enough to turn
//! a test red if the third copy drifted, which is exactly the failure a shared
//! answer prevents.
//!
//! The two existing callers are deliberately not routed through this yet: each
//! does extra work around the common steps (naming the tab and recording a
//! resumed session; registering plugin ownership and honouring a focus flag),
//! and moving them is a refactor that wants characterization tests of its own.

use crate::app::App;

impl App {
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

        let root_pane = self.state.workspaces[ws_idx].tabs[tab_idx].root_pane;
        self.terminal_runtimes.insert(terminal.id.clone(), runtime);
        self.state.remove_alias_shadowed_by_new_pane(root_pane);
        self.state.terminals.insert(terminal.id.clone(), terminal);
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
        Ok(())
    }
}
