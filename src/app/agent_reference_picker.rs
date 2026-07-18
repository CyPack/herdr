//! Client-local "Add Reference to Agent..." picker: choose one live agent
//! terminal as the explicit target for a prepared file reference
//! (TP-FIP-REF-01/02/04). Opening and selecting perform no runtime work;
//! delivery still crosses the existing App-owned send boundary.

use std::path::PathBuf;

/// One selectable live agent target. Identities are snapshotted from the
/// agents projection; `live` is recomputed against current state so a
/// vanished pane renders disabled instead of silently disappearing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentReferencePickerRow {
    pub label: String,
    pub is_current: bool,
    pub workspace_id: String,
    pub pane_id: crate::layout::PaneId,
    pub terminal_id: crate::terminal::TerminalId,
    pub live: bool,
}

/// Blocking client-local picker state. Owns no watcher, worker, process,
/// pane, or server state; closing it discards only presentation data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentReferencePickerState {
    pub source_path: PathBuf,
    pub source_files_generation: u32,
    pub rows: Vec<AgentReferencePickerRow>,
    pub selected: usize,
}

/// Exact reference-delivery authority prepared by an explicit picker
/// activation. Every identity is revalidated at the send boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentReferenceRequest {
    pub path: PathBuf,
    pub source_files_generation: u32,
    pub workspace_id: String,
    pub pane_id: crate::layout::PaneId,
    pub terminal_id: crate::terminal::TerminalId,
}

impl crate::app::state::AppState {
    /// Centered popup rect over the terminal area: bounded width, one row
    /// per target plus the header. Pure presentation geometry.
    pub(crate) fn agent_reference_picker_popup_rect(&self) -> Option<ratatui::layout::Rect> {
        let picker = self.agent_reference_picker.as_ref()?;
        let area = self.view.terminal_area;
        let width = 44u16.min(area.width.saturating_sub(2)).max(4);
        let height = (picker.rows.len() as u16)
            .saturating_add(4)
            .min(area.height.saturating_sub(2))
            .max(4);
        if area.width < 8 || area.height < 6 {
            return None;
        }
        let x = area.x + (area.width.saturating_sub(width)) / 2;
        let y = area.y + (area.height.saturating_sub(height)) / 2;
        Some(ratatui::layout::Rect::new(x, y, width, height))
    }

    /// One hit rect per visible picker row, aligned with the rendered list.
    pub(crate) fn agent_reference_picker_row_hit_areas(&self) -> Vec<ratatui::layout::Rect> {
        let Some(picker) = self.agent_reference_picker.as_ref() else {
            return Vec::new();
        };
        let Some(popup) = self.agent_reference_picker_popup_rect() else {
            return Vec::new();
        };
        let inner = ratatui::layout::Rect::new(
            popup.x + 1,
            popup.y + 3,
            popup.width.saturating_sub(2),
            popup.height.saturating_sub(4),
        );
        picker
            .rows
            .iter()
            .enumerate()
            .take(inner.height as usize)
            .map(|(idx, _)| {
                ratatui::layout::Rect::new(inner.x, inner.y + idx as u16, inner.width, 1)
            })
            .collect()
    }

    /// Row index under one exact cell, or None outside every row.
    pub(crate) fn agent_reference_picker_row_at(&self, column: u16, row: u16) -> Option<usize> {
        self.agent_reference_picker_row_hit_areas()
            .iter()
            .position(|rect| {
                column >= rect.x && column < rect.right() && row >= rect.y && row < rect.bottom()
            })
    }

    /// Close the picker and restore the pre-overlay focus owner. Closing
    /// discards only client-local presentation state and sends zero bytes.
    pub(crate) fn close_agent_reference_picker(&mut self) {
        if self.agent_reference_picker.take().is_some() {
            crate::app::input::leave_modal(self);
        }
    }
}

impl crate::app::App {
    /// Open the blocking agent picker for one supported, still-deliverable
    /// path. Opening performs no runtime work and prepares no delivery
    /// authority; with zero live agents the action stays a no-op.
    pub(super) fn open_agent_reference_picker(&mut self, paths: Vec<PathBuf>) -> bool {
        self.state.request_file_manager_agent_handoff = None;
        let Some(path) = paths.first().filter(|_| paths.len() == 1).cloned() else {
            return false;
        };
        if self.file_operation_worker.is_busy()
            || self
                .state
                .file_manager_operation
                .as_ref()
                .is_some_and(crate::app::state::FileManagerOperationState::is_running)
        {
            return false;
        }
        let Some(file_manager) = self.state.file_manager.as_ref() else {
            return false;
        };
        if !file_manager
            .entries
            .iter()
            .any(|entry| entry.operation_supported() && entry.path == path)
        {
            return false;
        }
        if !crate::app::file_agent_handoff::reference_path_is_deliverable(&path) {
            return false;
        }
        let rows = self.agent_reference_picker_rows();
        if rows.is_empty() {
            return false;
        }
        self.state.agent_reference_picker = Some(AgentReferencePickerState {
            source_path: path,
            source_files_generation: self
                .state
                .stage
                .active_instance_generation()
                .unwrap_or_default(),
            rows,
            selected: 0,
        });
        self.state
            .enter_overlay_mode(crate::app::state::Mode::AgentReferencePicker);
        true
    }

    /// Live agent targets projected from the agents panel: agent-classified
    /// terminals only, deduped per pane, with the focused agent first and
    /// marked current. Bounded `O(live agent panes)`.
    fn agent_reference_picker_rows(&self) -> Vec<AgentReferencePickerRow> {
        let focused = self.state.active.and_then(|workspace_idx| {
            self.state
                .workspaces
                .get(workspace_idx)
                .and_then(crate::workspace::Workspace::focused_pane_id)
                .map(|pane_id| (workspace_idx, pane_id))
        });
        let mut rows: Vec<AgentReferencePickerRow> = crate::ui::agent_panel_entries(&self.state)
            .into_iter()
            .filter_map(|entry| {
                let terminal_id = self
                    .state
                    .terminal_id_for_pane(entry.ws_idx, entry.pane_id)?;
                if !self
                    .state
                    .terminals
                    .get(&terminal_id)
                    .is_some_and(crate::terminal::TerminalState::is_agent_terminal)
                {
                    return None;
                }
                let workspace_id = self.state.workspaces.get(entry.ws_idx)?.id.clone();
                Some(AgentReferencePickerRow {
                    label: entry
                        .agent_label
                        .clone()
                        .unwrap_or_else(|| entry.primary_label.clone()),
                    is_current: focused == Some((entry.ws_idx, entry.pane_id)),
                    workspace_id,
                    pane_id: entry.pane_id,
                    terminal_id,
                    live: true,
                })
            })
            .collect();
        rows.sort_by_key(|row| !row.is_current);
        rows
    }

    /// Explicit picker activation: snapshot the FULL selected target
    /// identity into the typed request and close the picker. Delivery still
    /// happens at the App-owned send boundary on a later frame; a stale or
    /// non-live selection prepares nothing (fail closed).
    pub(crate) fn activate_agent_reference_picker_selection(&mut self) -> bool {
        let Some(picker) = self.state.agent_reference_picker.as_ref() else {
            return false;
        };
        let Some(row) = picker.rows.get(picker.selected).filter(|row| row.live) else {
            return false;
        };
        let row = row.clone();
        let source_path = picker.source_path.clone();
        let source_files_generation = picker.source_files_generation;
        let Some(workspace_idx) = self
            .state
            .workspaces
            .iter()
            .position(|workspace| workspace.id == row.workspace_id)
        else {
            self.fail_agent_reference_activation();
            return false;
        };
        if self
            .state
            .terminal_id_for_pane(workspace_idx, row.pane_id)
            .as_ref()
            != Some(&row.terminal_id)
            || !self
                .state
                .terminals
                .get(&row.terminal_id)
                .is_some_and(crate::terminal::TerminalState::is_agent_terminal)
            || !crate::app::file_agent_handoff::reference_path_is_deliverable(&source_path)
        {
            self.fail_agent_reference_activation();
            return false;
        }
        self.state.request_file_manager_agent_handoff = Some(AgentReferenceRequest {
            path: source_path,
            source_files_generation,
            workspace_id: row.workspace_id,
            pane_id: row.pane_id,
            terminal_id: row.terminal_id,
        });
        self.state.close_agent_reference_picker();
        true
    }

    /// Per-frame liveness recompute for open picker rows: `live` is
    /// re-derived from the current workspace/pane/terminal bindings, bounded
    /// `O(rows)`. A vanished pane renders disabled instead of silently
    /// working; identities and ordering never change after open.
    pub(super) fn sync_agent_reference_picker(&mut self) -> bool {
        let Some(picker) = self.state.agent_reference_picker.take() else {
            return false;
        };
        let mut picker = picker;
        let mut changed = false;
        for row in &mut picker.rows {
            let live = self
                .state
                .workspaces
                .iter()
                .position(|workspace| workspace.id == row.workspace_id)
                .is_some_and(|workspace_idx| {
                    self.state
                        .terminal_id_for_pane(workspace_idx, row.pane_id)
                        .as_ref()
                        == Some(&row.terminal_id)
                })
                && self
                    .state
                    .terminals
                    .get(&row.terminal_id)
                    .is_some_and(crate::terminal::TerminalState::is_agent_terminal);
            if row.live != live {
                row.live = live;
                changed = true;
            }
        }
        self.state.agent_reference_picker = Some(picker);
        changed
    }

    /// A stale activation is consumed loudly: the picker closes with zero
    /// bytes prepared and the failure stays visible (TP-FIP-5.5).
    fn fail_agent_reference_activation(&mut self) {
        self.state.close_agent_reference_picker();
        self.show_file_manager_agent_handoff_failure("agent handoff authority changed");
    }

    /// Blocking-overlay key routing owner for the picker mode. The
    /// popup-ownership task wires full selection movement; Esc always
    /// closes with zero bytes.
    pub(crate) fn handle_agent_reference_picker_key(&mut self, key: crossterm::event::KeyEvent) {
        match key.code {
            crossterm::event::KeyCode::Esc => self.state.close_agent_reference_picker(),
            crossterm::event::KeyCode::Enter => {
                let _ = self.activate_agent_reference_picker_selection();
            }
            crossterm::event::KeyCode::Up | crossterm::event::KeyCode::Char('k') => {
                if let Some(picker) = self.state.agent_reference_picker.as_mut() {
                    picker.selected = picker.selected.saturating_sub(1);
                }
            }
            crossterm::event::KeyCode::Down | crossterm::event::KeyCode::Char('j') => {
                if let Some(picker) = self.state.agent_reference_picker.as_mut() {
                    if picker.selected.saturating_add(1) < picker.rows.len() {
                        picker.selected += 1;
                    }
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::app::state::{FileManagerContextMenuAction, Mode};

    struct PickerFixture {
        root: std::path::PathBuf,
    }

    impl PickerFixture {
        fn new(tag: &str) -> Self {
            let root = std::env::temp_dir().join(format!(
                "herdr-agent-ref-picker-{}-{tag}",
                std::process::id()
            ));
            let _ = std::fs::remove_dir_all(&root);
            std::fs::create_dir_all(&root).expect("create picker fixture root");
            Self { root }
        }

        fn file(&self, name: &str) -> std::path::PathBuf {
            let path = self.root.join(name);
            std::fs::write(&path, b"reference").expect("write picker fixture");
            path
        }
    }

    impl Drop for PickerFixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    /// One workspace, two panes, BOTH agent terminals; Files stage open.
    /// Returns the app plus the focused and neighbor pane terminal ids.
    fn app_with_two_agents(
        root: &std::path::Path,
    ) -> (
        crate::app::App,
        crate::terminal::TerminalId,
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
        let mut workspace = crate::workspace::Workspace::test_new("agent-ref-picker");
        let focused_pane = workspace.tabs[0].root_pane;
        workspace.test_split(ratatui::layout::Direction::Horizontal);
        workspace.tabs[0].layout.focus_pane(focused_pane);
        let neighbor_pane = workspace.tabs[0]
            .layout
            .pane_ids()
            .into_iter()
            .find(|pane_id| *pane_id != focused_pane)
            .expect("neighbor pane");
        let focused_terminal = workspace
            .terminal_id(focused_pane)
            .expect("focused terminal")
            .clone();
        let neighbor_terminal = workspace
            .terminal_id(neighbor_pane)
            .expect("neighbor terminal")
            .clone();
        app.state.workspaces = vec![workspace];
        app.state.ensure_test_terminals();
        for (terminal_id, name) in [
            (&focused_terminal, "agent-focused"),
            (&neighbor_terminal, "agent-neighbor"),
        ] {
            app.state
                .terminals
                .get_mut(terminal_id)
                .expect("agent terminal state")
                .set_agent_name(name.into());
        }
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state
            .try_open_file_manager_with(|_| Some(crate::fm::FmState::new(root)))
            .expect("Files activation");
        (app, focused_terminal, neighbor_terminal)
    }

    fn dispatch_reference_action(app: &mut crate::app::App, path: std::path::PathBuf) {
        app.state.request_file_manager_context_action =
            Some(crate::app::state::FileManagerContextActionIntent {
                action: FileManagerContextMenuAction::SendAgent,
                paths: vec![path],
            });
        assert!(app.sync_file_manager_agent_handoff());
    }

    // Kullanıcı kanıtı (2026-07-18 canlı ekran): hiç canlı agent yokken
    // "Add Reference to Agent..." SESSİZ no-op idi. Fail-visible kontrat:
    // sıfır byte + tek görünür hata, picker açılmaz.
    #[tokio::test]
    async fn reference_action_with_no_live_agent_shows_visible_failure() {
        let fixture = PickerFixture::new("no-live-agent");
        let path = fixture.file("selected.txt");
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = crate::app::App::new(
            &crate::config::Config::default(),
            true,
            None,
            api_rx,
            crate::api::EventHub::default(),
        );
        // Workspace with a PLAIN (non-agent) terminal only.
        app.state.workspaces = vec![crate::workspace::Workspace::test_new("no-agent")];
        app.state.ensure_test_terminals();
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state
            .try_open_file_manager_with(|_| Some(crate::fm::FmState::new(&fixture.root)))
            .expect("Files activation");

        app.state.request_file_manager_context_action =
            Some(crate::app::state::FileManagerContextActionIntent {
                action: FileManagerContextMenuAction::SendAgent,
                paths: vec![path],
            });
        assert!(app.sync_file_manager_agent_handoff());

        assert!(app.state.agent_reference_picker.is_none());
        assert!(app.state.request_file_manager_agent_handoff.is_none());
        let toast = app
            .state
            .toast
            .as_ref()
            .expect("a dead-end action must be visibly explained, never silent");
        assert_eq!(toast.context, "no live agent to receive references");
    }

    // TP-FIP-5.5: a target pane closed while the picker is open renders
    // disabled on the next recompute instead of silently disappearing.
    #[tokio::test]
    async fn target_pane_closed_while_picker_open_disables_row_on_recompute() {
        let fixture = PickerFixture::new("pane-closed-recompute");
        let path = fixture.file("selected.txt");
        let (mut app, _, neighbor_terminal) = app_with_two_agents(&fixture.root);
        dispatch_reference_action(&mut app, path);
        let neighbor_row = app
            .state
            .agent_reference_picker
            .as_ref()
            .expect("open picker")
            .rows
            .iter()
            .position(|row| row.terminal_id == neighbor_terminal)
            .expect("neighbor row");
        let neighbor_pane = app
            .state
            .agent_reference_picker
            .as_ref()
            .expect("open picker")
            .rows[neighbor_row]
            .pane_id;

        let _ = app.state.workspaces[0].remove_pane(neighbor_pane);
        assert!(app.sync_agent_reference_picker());

        let picker = app
            .state
            .agent_reference_picker
            .as_ref()
            .expect("picker stays open");
        assert!(
            !picker.rows[neighbor_row].live,
            "a closed pane must render disabled on recompute"
        );
    }

    // TP-FIP-5.5: activating a target that disappeared after open fails
    // closed with zero bytes and one visible failure.
    #[tokio::test]
    async fn activation_of_disappeared_target_fails_closed_with_visible_failure() {
        let fixture = PickerFixture::new("vanished-activation");
        let path = fixture.file("selected.txt");
        let (mut app, focused_terminal, _) = app_with_two_agents(&fixture.root);
        dispatch_reference_action(&mut app, path);
        let focused_pane = app
            .state
            .agent_reference_picker
            .as_ref()
            .expect("open picker")
            .rows[0]
            .pane_id;
        assert_eq!(
            app.state
                .agent_reference_picker
                .as_ref()
                .expect("open picker")
                .rows[0]
                .terminal_id,
            focused_terminal
        );
        let _ = app.state.workspaces[0].remove_pane(focused_pane);

        assert!(!app.activate_agent_reference_picker_selection());
        assert!(app.state.request_file_manager_agent_handoff.is_none());
        assert!(
            app.state.agent_reference_picker.is_none(),
            "a vanished-target activation closes the picker"
        );
        assert_eq!(
            app.state.toast.as_ref().map(|toast| toast.context.as_str()),
            Some("agent handoff authority changed"),
            "the failure must be visible"
        );
    }

    // TP-FIP-5.5: a terminal that stopped being an agent between open and
    // activation prepares nothing — zero bytes ever cross.
    #[tokio::test]
    async fn terminal_identity_change_between_open_and_activation_sends_zero_bytes() {
        let fixture = PickerFixture::new("identity-change-activation");
        let path = fixture.file("selected.txt");
        let (mut app, focused_terminal, _) = app_with_two_agents(&fixture.root);
        dispatch_reference_action(&mut app, path);
        app.state
            .terminals
            .get_mut(&focused_terminal)
            .expect("focused terminal state")
            .clear_agent_name();

        assert!(!app.activate_agent_reference_picker_selection());
        assert!(app.state.request_file_manager_agent_handoff.is_none());
        assert_eq!(
            app.state.toast.as_ref().map(|toast| toast.context.as_str()),
            Some("agent handoff authority changed")
        );
    }

    // TP-FIP-REF-15: the picker is a blocking overlay — background gestures
    // are consumed fail-closed while it is open.
    #[tokio::test]
    async fn picker_enters_overlay_mode_and_blocks_background_input() {
        let fixture = PickerFixture::new("blocks-background");
        let path = fixture.file("selected.txt");
        let (mut app, _, _) = app_with_two_agents(&fixture.root);
        app.state.view.terminal_area = ratatui::layout::Rect::new(0, 0, 120, 40);
        dispatch_reference_action(&mut app, path);
        assert_eq!(app.state.mode, Mode::AgentReferencePicker);
        let fm_cursor_before = app.state.file_manager.as_ref().expect("open FM").cursor;

        app.handle_mouse(crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::ScrollDown,
            column: 5,
            row: 5,
            modifiers: crossterm::event::KeyModifiers::NONE,
        });

        assert!(
            app.state.agent_reference_picker.is_some(),
            "a background wheel gesture must not close the picker"
        );
        assert_eq!(
            app.state.file_manager.as_ref().expect("open FM").cursor,
            fm_cursor_before,
            "background surfaces must not act while the picker is open"
        );
    }

    // TP-FIP-REF-15: Escape and an outside click both close the picker with
    // zero bytes and restore the pre-overlay focus owner.
    #[tokio::test]
    async fn escape_and_outside_click_close_picker_with_zero_bytes() {
        let fixture = PickerFixture::new("close-paths");
        let path = fixture.file("selected.txt");
        let (mut app, _, _) = app_with_two_agents(&fixture.root);
        app.state.view.terminal_area = ratatui::layout::Rect::new(0, 0, 120, 40);
        dispatch_reference_action(&mut app, path.clone());
        app.handle_agent_reference_picker_key(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Esc,
            crossterm::event::KeyModifiers::NONE,
        ));
        assert!(app.state.agent_reference_picker.is_none());
        assert_ne!(app.state.mode, Mode::AgentReferencePicker);
        assert!(app.state.request_file_manager_agent_handoff.is_none());

        dispatch_reference_action(&mut app, path);
        let popup = app
            .state
            .agent_reference_picker_popup_rect()
            .expect("picker popup rect");
        let outside = (popup.x.saturating_sub(2), popup.y.saturating_sub(2));
        app.handle_mouse(crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
            column: outside.0,
            row: outside.1,
            modifiers: crossterm::event::KeyModifiers::NONE,
        });
        assert!(
            app.state.agent_reference_picker.is_none(),
            "an outside click must close the picker"
        );
        assert!(app.state.request_file_manager_agent_handoff.is_none());
    }

    // TP-FIP-REF-15/04: keyboard movement and mouse clicks share ONE
    // selection authority; Enter and a row click activate the same target.
    #[tokio::test]
    async fn keyboard_up_down_enter_and_mouse_click_share_selection() {
        let fixture = PickerFixture::new("shared-selection");
        let path = fixture.file("selected.txt");
        let (mut app, _, neighbor_terminal) = app_with_two_agents(&fixture.root);
        app.state.view.terminal_area = ratatui::layout::Rect::new(0, 0, 120, 40);
        dispatch_reference_action(&mut app, path.clone());
        let key =
            |code| crossterm::event::KeyEvent::new(code, crossterm::event::KeyModifiers::NONE);
        app.handle_agent_reference_picker_key(key(crossterm::event::KeyCode::Down));
        assert_eq!(
            app.state
                .agent_reference_picker
                .as_ref()
                .expect("open picker")
                .selected,
            1,
            "Down must move the shared selection"
        );
        app.handle_agent_reference_picker_key(key(crossterm::event::KeyCode::Up));
        assert_eq!(
            app.state
                .agent_reference_picker
                .as_ref()
                .expect("open picker")
                .selected,
            0
        );
        app.handle_agent_reference_picker_key(key(crossterm::event::KeyCode::Down));
        app.handle_agent_reference_picker_key(key(crossterm::event::KeyCode::Enter));
        let request = app
            .state
            .request_file_manager_agent_handoff
            .take()
            .expect("Enter activates the selected row");
        assert_eq!(request.terminal_id, neighbor_terminal);

        dispatch_reference_action(&mut app, path);
        let row0 = app.state.agent_reference_picker_row_hit_areas()[0];
        app.handle_mouse(crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
            column: row0.x + 1,
            row: row0.y,
            modifiers: crossterm::event::KeyModifiers::NONE,
        });
        let request = app
            .state
            .request_file_manager_agent_handoff
            .as_ref()
            .expect("a row click activates that exact row");
        assert_ne!(request.terminal_id, neighbor_terminal);
        assert!(app.state.agent_reference_picker.is_none());
    }

    // TP-FIP-REF-16: a non-live row can be activated by neither keyboard nor
    // mouse; the picker stays open with zero bytes prepared.
    #[tokio::test]
    async fn disabled_row_cannot_be_activated_by_keyboard_or_mouse() {
        let fixture = PickerFixture::new("disabled-row");
        let path = fixture.file("selected.txt");
        let (mut app, _, _) = app_with_two_agents(&fixture.root);
        app.state.view.terminal_area = ratatui::layout::Rect::new(0, 0, 120, 40);
        dispatch_reference_action(&mut app, path);
        app.state
            .agent_reference_picker
            .as_mut()
            .expect("open picker")
            .rows[0]
            .live = false;

        app.handle_agent_reference_picker_key(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Enter,
            crossterm::event::KeyModifiers::NONE,
        ));
        assert!(app.state.request_file_manager_agent_handoff.is_none());
        assert!(app.state.agent_reference_picker.is_some());

        let row0 = app.state.agent_reference_picker_row_hit_areas()[0];
        app.handle_mouse(crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
            column: row0.x + 1,
            row: row0.y,
            modifiers: crossterm::event::KeyModifiers::NONE,
        });
        assert!(app.state.request_file_manager_agent_handoff.is_none());
        assert!(app.state.agent_reference_picker.is_some());
    }

    // TP-FIP-REF-16: a stale source path opens nothing — the intent is
    // consumed without a picker or any prepared authority.
    #[tokio::test]
    async fn stale_source_row_or_context_does_not_open_picker() {
        let fixture = PickerFixture::new("stale-source");
        let path = fixture.file("selected.txt");
        let (mut app, _, _) = app_with_two_agents(&fixture.root);
        std::fs::remove_file(&path).expect("delete before dispatch");
        app.state
            .file_manager
            .as_mut()
            .expect("open FM")
            .entries
            .clear();

        app.state.request_file_manager_context_action =
            Some(crate::app::state::FileManagerContextActionIntent {
                action: FileManagerContextMenuAction::SendAgent,
                paths: vec![path],
            });
        assert!(app.sync_file_manager_agent_handoff());
        assert!(app.state.agent_reference_picker.is_none());
        assert!(app.state.request_file_manager_agent_handoff.is_none());
        assert_ne!(app.state.mode, Mode::AgentReferencePicker);
    }

    // TP-FIP-REF-17: an explicit multi-selection disables the reference
    // action — the intent carries multiple paths and opens nothing.
    #[tokio::test]
    async fn multi_selection_disables_reference_action() {
        let fixture = PickerFixture::new("multi-selection");
        let first = fixture.file("first.txt");
        let second = fixture.file("second.txt");
        let (mut app, _, _) = app_with_two_agents(&fixture.root);

        app.state.request_file_manager_context_action =
            Some(crate::app::state::FileManagerContextActionIntent {
                action: FileManagerContextMenuAction::SendAgent,
                paths: vec![first, second],
            });
        assert!(app.sync_file_manager_agent_handoff());
        assert!(app.state.agent_reference_picker.is_none());
        assert!(app.state.request_file_manager_agent_handoff.is_none());
    }

    // TP-FIP-REF-01: the reference action opens a blocking picker built from
    // the live agents projection — it never sends bytes on open.
    #[tokio::test]
    async fn reference_action_opens_picker_from_live_agents_projection() {
        let fixture = PickerFixture::new("opens-picker");
        let path = fixture.file("selected.txt");
        let (mut app, focused_terminal, neighbor_terminal) = app_with_two_agents(&fixture.root);

        dispatch_reference_action(&mut app, path.clone());

        let picker = app
            .state
            .agent_reference_picker
            .as_ref()
            .expect("the reference action must open the agent picker");
        assert_eq!(app.state.mode, Mode::AgentReferencePicker);
        assert_eq!(picker.source_path, path);
        let picker_terminals: Vec<_> = picker
            .rows
            .iter()
            .map(|row| row.terminal_id.clone())
            .collect();
        assert!(picker_terminals.contains(&focused_terminal));
        assert!(picker_terminals.contains(&neighbor_terminal));
        assert_eq!(picker.rows.len(), 2, "exactly the live agent entries");
        assert!(
            app.state.request_file_manager_agent_handoff.is_none(),
            "opening the picker must not prepare delivery authority"
        );
    }

    // TP-FIP-REF-02: the focused agent is the first row, marked current,
    // and preselected.
    #[tokio::test]
    async fn current_focused_agent_is_first_and_preselected() {
        let fixture = PickerFixture::new("current-first");
        let path = fixture.file("selected.txt");
        let (mut app, focused_terminal, _) = app_with_two_agents(&fixture.root);

        dispatch_reference_action(&mut app, path);

        let picker = app
            .state
            .agent_reference_picker
            .as_ref()
            .expect("the reference action must open the agent picker");
        assert_eq!(picker.selected, 0, "the current chat is preselected");
        assert!(picker.rows[0].is_current);
        assert_eq!(picker.rows[0].terminal_id, focused_terminal);
        assert!(!picker.rows[1].is_current);
    }

    // TP-FIP-REF-04: activating a row snapshots the FULL target identity
    // (path, files generation, workspace, pane, terminal) and sends nothing.
    #[tokio::test]
    async fn picker_selection_snapshots_full_target_identity() {
        let fixture = PickerFixture::new("snapshot-identity");
        let path = fixture.file("selected.txt");
        let (mut app, focused_terminal, _) = app_with_two_agents(&fixture.root);
        let workspace_id = app.state.workspaces[0].id.clone();
        let focused_pane = app.state.workspaces[0]
            .focused_pane_id()
            .expect("focused pane");

        dispatch_reference_action(&mut app, path.clone());
        let generation = app
            .state
            .agent_reference_picker
            .as_ref()
            .expect("open picker")
            .source_files_generation;

        assert!(app.activate_agent_reference_picker_selection());
        let request = app
            .state
            .request_file_manager_agent_handoff
            .as_ref()
            .expect("activation must prepare the typed reference request");
        assert_eq!(request.path, path);
        assert_eq!(request.source_files_generation, generation);
        assert_eq!(request.workspace_id, workspace_id);
        assert_eq!(request.pane_id, focused_pane);
        assert_eq!(request.terminal_id, focused_terminal);
        assert!(
            app.state.agent_reference_picker.is_none(),
            "activation closes the picker"
        );
    }
}
