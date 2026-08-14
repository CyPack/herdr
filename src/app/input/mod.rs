//! Input handling — translates crossterm key/mouse events into state mutations.

use bytes::Bytes;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use tracing::warn;

use crate::app::PaneClickState;
use crate::input::TerminalKey;
#[cfg(test)]
use ratatui::layout::Direction;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScrollbarClickTarget {
    Thumb { grab_row_offset: u16 },
    Track { offset_from_bottom: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg(test)]
enum WheelRouting {
    HostScroll,
    MouseReport,
    AlternateScroll,
}

const WORKSPACE_DRAG_THRESHOLD: u16 = 1;
const TAB_DRAG_THRESHOLD: u16 = 1;

fn modified_url_click_modifier() -> KeyModifiers {
    KeyModifiers::CONTROL
}

#[cfg(test)]
#[test]
fn modified_url_click_modifier_matches_terminal_mouse_reporting() {
    assert_eq!(modified_url_click_modifier(), KeyModifiers::CONTROL);
}

mod copy_mode;
mod file_manager;
pub(in crate::app) use file_manager::FileManagerVerticalWheelBurstGate;
pub(crate) mod modal;
pub(crate) use modal::leave_modal;
mod mouse;
mod navigate;
mod overlays;
mod selection;
mod settings;
mod shell;
mod sidebar;
mod terminal;

pub(crate) use self::{
    file_manager::{
        handle_preview_viewer_key, handle_tailscale_send_key, open_preview_viewer,
        open_tailscale_send,
    },
    modal::{
        handle_global_menu_key, handle_keybind_help_key, handle_navigator_key,
        insert_keybind_help_query_text, insert_navigator_search_text, insert_rename_input_text,
        open_new_workspace_dialog,
    },
    navigate::{
        terminal_direct_indexed_navigation_action, terminal_direct_non_indexed_navigation_action,
    },
    settings::open_settings_at,
};
use self::{
    modal::{
        modal_action_from_key, ModalAction, ONBOARDING_WELCOME_ACTIONS, RELEASE_NOTES_ACTIONS,
    },
    mouse::MouseAction,
    settings::SettingsAction,
    shell::{BarSectionClick, SectionGesture, ShellInputOwner},
};
use super::state::{AppState, Mode};
use super::App;

// ---------------------------------------------------------------------------
// Key handling
// ---------------------------------------------------------------------------

impl App {
    pub(super) async fn handle_key(
        &mut self,
        key: TerminalKey,
    ) -> Option<super::TerminalInputTarget> {
        if self.state.popup_pane.is_some() {
            return self.handle_terminal_key(key).await;
        }
        let key_event = key.as_key_event();
        if modal_paste_target_active(&self.state) && is_modal_paste_shortcut(&key_event) {
            if let Some(text) = crate::platform::read_clipboard_text() {
                self.paste_into_active_text_input(&text);
            }
            return None;
        }

        // One frozen precedence resolves the keyboard owner: topmost overlay
        // -> active capture -> focused component -> global dispatch. The
        // router is the single ordering authority; each arm below only
        // dispatches within the tier it was granted.
        match self.state.shell_key_input_owner() {
            ShellInputOwner::TopmostOverlay => {
                // A blocking overlay owns keyboard focus ahead of the active
                // capture and the visible surface, even when its native-FM
                // surface remains visible underneath it. Overlay modes
                // without a dedicated route below own their keys through the
                // same mode-guarded global dispatch.
                match self.state.mode {
                    Mode::ContextMenu => {
                        self.handle_context_menu_key_via_api(key_event);
                        None
                    }
                    Mode::ConfirmFileDelete => {
                        self.handle_file_manager_delete_confirmation_key(key_event);
                        None
                    }
                    Mode::RenameFile => {
                        self.handle_rename_key_via_api(key_event);
                        None
                    }
                    Mode::AttachFile => {
                        self.route_agent_attachment_picker_key(key_event);
                        None
                    }
                    _ => self.handle_global_key_dispatch(key).await,
                }
            }
            ShellInputOwner::ActiveCapture => {
                // An active typed divider capture owns keyboard input ahead of
                // the visible surface. Target-specific handlers keep resize
                // keys out of native apps and PTYs while preserving topmost
                // modal ownership above.
                self.handle_active_capture_key(key_event);
                None
            }
            ShellInputOwner::FocusedComponent => {
                // When the native file manager is open it captures all
                // keyboard input, ahead of the mode dispatch, so keys drive
                // its navigation instead of reaching the terminal underneath.
                self.handle_focused_file_manager_key(key_event);
                None
            }
            ShellInputOwner::GlobalShortcut => self.handle_global_key_dispatch(key).await,
            ShellInputOwner::TopmostHit(_) | ShellInputOwner::PageShortcut => {
                // Keyboard routing produces no positional or page owner in
                // v0; consume fail-closed rather than acting for a tier the
                // context builder cannot grant.
                debug_assert!(false, "keyboard routing has no positional or page owner");
                None
            }
            ShellInputOwner::FailClosed => None,
        }
    }

    /// Returns the terminal target the key reached, so the caller can track
    /// press/release pairs. Only the terminal tier yields a target; every
    /// other owner consumes the key inside herdr's own chrome.
    pub(super) fn handle_key_headless(
        &mut self,
        key: TerminalKey,
    ) -> Option<super::TerminalInputTarget> {
        let key_event = key.as_key_event();
        if modal_paste_target_active(&self.state) && is_modal_paste_shortcut(&key_event) {
            if let Some(text) = crate::platform::read_clipboard_text() {
                self.paste_into_active_text_input(&text);
            }
            return None;
        }

        // An open popup owns the keyboard outright, exactly as the monolithic
        // loop's first check does. Without this, Esc pressed over the editor
        // popup routed to the shell owner underneath — the file manager — and
        // cleared ITS selection while the popup sat there unclosed.
        if self.state.popup_pane.is_some() {
            return self.handle_terminal_key_headless(key);
        }

        match self.state.shell_key_input_owner() {
            ShellInputOwner::TopmostOverlay => {
                self.handle_non_terminal_key_headless(key);
                None
            }
            ShellInputOwner::ActiveCapture => {
                self.handle_active_capture_key(key_event);
                None
            }
            ShellInputOwner::FocusedComponent => {
                self.handle_focused_file_manager_key(key_event);
                None
            }
            ShellInputOwner::GlobalShortcut => {
                if self.state.mode == Mode::Terminal {
                    self.handle_terminal_key_headless(key)
                } else {
                    self.handle_non_terminal_key_headless(key);
                    None
                }
            }
            ShellInputOwner::TopmostHit(_) | ShellInputOwner::PageShortcut => {
                debug_assert!(false, "keyboard routing has no positional or page owner");
                None
            }
            ShellInputOwner::FailClosed => None,
        }
    }

    fn handle_active_capture_key(&mut self, key_event: KeyEvent) {
        let handled = if self.state.shell_resize_active() {
            self.state.handle_shell_resize_key(key_event)
        } else {
            self.handle_miller_resize_key(key_event)
        };
        debug_assert!(handled, "an active capture must consume every key");
    }

    fn handle_focused_file_manager_key(&mut self, key_event: KeyEvent) {
        if self.state.file_manager.is_none() {
            debug_assert!(
                false,
                "focused component keyboard ownership requires an open file manager"
            );
            return;
        }

        match file_manager::handle_file_manager_key(&mut self.state, key_event) {
            file_manager::FileManagerKeyDispatch::CancelOperation => {
                let _ = self.cancel_file_manager_operation();
            }
            file_manager::FileManagerKeyDispatch::Refresh(request) => {
                let _ = self.execute_file_manager_current_refresh(*request);
            }
            file_manager::FileManagerKeyDispatch::PreviewDirectory {
                trail_col,
                entry_index,
                expected_path,
            } => {
                let _ = self.queue_file_manager_trail_directory_preview_identity(
                    trail_col,
                    entry_index,
                    &expected_path,
                );
            }
            file_manager::FileManagerKeyDispatch::ActivateDirectory {
                trail_col,
                entry_index,
                expected_path,
            } => {
                let _ = self.queue_file_manager_trail_directory_activation_identity(
                    trail_col,
                    entry_index,
                    &expected_path,
                );
            }
            file_manager::FileManagerKeyDispatch::Inert => {
                self.file_manager_key_render_override = Some(false);
            }
            file_manager::FileManagerKeyDispatch::DeferredLocationNavigation => {
                self.file_manager_key_render_override = Some(false);
            }
            file_manager::FileManagerKeyDispatch::Consumed => {}
        }
    }

    async fn handle_global_key_dispatch(
        &mut self,
        key: TerminalKey,
    ) -> Option<super::TerminalInputTarget> {
        let key_event = key.as_key_event();
        match self.state.mode {
            Mode::Terminal => return self.handle_terminal_key(key).await,
            Mode::Prefix => self.handle_prefix_key(key),
            Mode::Navigate => self.handle_navigate_key(key),
            Mode::Copy => self.handle_copy_mode_key(key),
            _ => match self.state.mode {
                Mode::Onboarding => self.handle_onboarding_key(key_event),
                Mode::ReleaseNotes => self.handle_release_notes_key(key_event),
                Mode::ProductAnnouncement => self.handle_product_announcement_key(key_event),
                Mode::Prefix | Mode::Navigate | Mode::Copy => unreachable!(),
                Mode::RenameWorkspace | Mode::RenameTab | Mode::RenamePane | Mode::RenameFile => {
                    self.handle_rename_key_via_api(key_event)
                }
                Mode::NewLinkedWorktree => self.handle_worktree_create_key(key_event),
                Mode::OpenExistingWorktree => self.handle_worktree_open_key(key_event),
                Mode::ConfirmRemoveWorktree => self.handle_worktree_remove_key(key_event),
                Mode::Resize => self.handle_resize_key_via_api(key),
                Mode::ConfirmClose => self.handle_confirm_close_key_via_api(key_event),
                Mode::ConfirmFileDelete => {
                    self.handle_file_manager_delete_confirmation_key(key_event)
                }
                Mode::ContextMenu => {
                    self.handle_context_menu_key_via_api(key_event);
                }
                Mode::Settings => self.handle_settings_key(key_event),
                Mode::GlobalMenu => handle_global_menu_key(&mut self.state, key_event),
                Mode::KeybindHelp => handle_keybind_help_key(&mut self.state, key),
                Mode::Navigator => {
                    handle_navigator_key(&mut self.state, &self.terminal_runtimes, key_event)
                }
                Mode::AgentReferencePicker => {
                    self.handle_agent_reference_picker_key(key_event);
                }
                Mode::PreviewViewer => {
                    handle_preview_viewer_key(&mut self.state, key_event);
                }
                Mode::TailscaleSend => {
                    if let Some(pinned) = handle_tailscale_send_key(&mut self.state, key_event) {
                        self.save_tailscale_pinned_devices(&pinned);
                    }
                }
                Mode::Terminal => unreachable!(),
                Mode::AttachFile => unreachable!(),
            },
        }
        None
    }

    pub(super) async fn handle_paste(&mut self, text: String) {
        if self.state.popup_pane.is_some() {
            if let Some(runtime) = self.popup_runtime() {
                let _ = runtime.send_paste(text).await;
            } else {
                self.close_popup_pane();
            }
            return;
        }
        if self.state.mode != Mode::Terminal {
            self.paste_into_active_text_input(&text);
            return;
        }
        if !self.visible_terminal_owns_paste() {
            return;
        }

        if let Some(ws_idx) = self.state.active {
            if let Some(rt) = self
                .state
                .focused_runtime_in_workspace(&self.terminal_runtimes, ws_idx)
            {
                let _ = rt.send_paste(text).await;
            }
        }
    }

    pub(super) fn visible_terminal_owns_paste(&self) -> bool {
        self.state.mode == Mode::Terminal
            && self.state.shell_key_input_owner() == ShellInputOwner::GlobalShortcut
    }

    pub(crate) fn paste_into_active_text_input(&mut self, text: &str) -> bool {
        match self.state.mode {
            Mode::RenameWorkspace | Mode::RenameTab | Mode::RenamePane | Mode::RenameFile => {
                insert_rename_input_text(&mut self.state, text);
                true
            }
            Mode::NewLinkedWorktree => {
                self.insert_worktree_create_text(text);
                true
            }
            Mode::OpenExistingWorktree => {
                if !self
                    .state
                    .worktree_open
                    .as_ref()
                    .is_some_and(|open| open.search_focused)
                {
                    return false;
                }
                self.insert_worktree_open_search_text(text);
                true
            }
            Mode::Navigator => {
                if !self.state.navigator.search_focused {
                    return false;
                }
                insert_navigator_search_text(&mut self.state, &self.terminal_runtimes, text);
                true
            }
            Mode::KeybindHelp => {
                if !self.state.keybind_help.search_focused {
                    return false;
                }
                insert_keybind_help_query_text(&mut self.state, text);
                true
            }
            Mode::Copy => {
                let Some(prompt) = self
                    .state
                    .copy_mode
                    .as_mut()
                    .and_then(|copy_mode| copy_mode.search.prompt.as_mut())
                else {
                    return false;
                };
                prompt
                    .query
                    .extend(text.chars().filter(|ch| !ch.is_control()));
                true
            }
            _ => false,
        }
    }

    pub(crate) fn handle_onboarding_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Right | KeyCode::Char('l') => self.open_settings_from_onboarding(),
            _ => {
                if let Some(ModalAction::Continue) =
                    modal_action_from_key(&key, ONBOARDING_WELCOME_ACTIONS)
                {
                    self.open_settings_from_onboarding();
                }
            }
        }
    }

    pub(crate) fn handle_release_notes_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => self.scroll_release_notes(-1),
            KeyCode::Down | KeyCode::Char('j') => self.scroll_release_notes(1),
            KeyCode::PageUp => self.scroll_release_notes(-8),
            KeyCode::PageDown => self.scroll_release_notes(8),
            KeyCode::Home => {
                if let Some(notes) = &mut self.state.release_notes {
                    notes.scroll = 0;
                }
            }
            KeyCode::End => {
                let max_scroll = self.state.release_notes_max_scroll();
                if let Some(notes) = &mut self.state.release_notes {
                    notes.scroll = max_scroll;
                }
            }
            _ => {
                if let Some(ModalAction::Close) = modal_action_from_key(&key, RELEASE_NOTES_ACTIONS)
                {
                    self.dismiss_release_notes();
                }
            }
        }
    }

    pub(crate) fn handle_product_announcement_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => self.scroll_product_announcement(-1),
            KeyCode::Down | KeyCode::Char('j') => self.scroll_product_announcement(1),
            KeyCode::PageUp => self.scroll_product_announcement(-8),
            KeyCode::PageDown => self.scroll_product_announcement(8),
            KeyCode::Home => {
                if let Some(announcement) = &mut self.state.product_announcement {
                    announcement.scroll = 0;
                }
            }
            KeyCode::End => {
                let max_scroll = self.state.product_announcement_max_scroll();
                if let Some(announcement) = &mut self.state.product_announcement {
                    announcement.scroll = max_scroll;
                }
            }
            _ => {
                if let Some(ModalAction::Close) = modal_action_from_key(&key, RELEASE_NOTES_ACTIONS)
                {
                    self.dismiss_product_announcement();
                }
            }
        }
    }

    pub(super) fn handle_mouse(&mut self, mouse: MouseEvent) {
        self.handle_mouse_from_input_source(super::LOCAL_INPUT_SOURCE, mouse);
    }

    pub(super) fn handle_mouse_from_input_source(
        &mut self,
        source_id: super::InputSourceId,
        mouse: MouseEvent,
    ) {
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                self.pending_url_click_sources.remove(&source_id);
            }
            MouseEventKind::Drag(MouseButton::Left)
                if self.pending_url_click_sources.contains(&source_id) =>
            {
                return;
            }
            MouseEventKind::Up(MouseButton::Left)
                if self.pending_url_click_sources.remove(&source_id) =>
            {
                return;
            }
            _ => {}
        }

        if self.state.popup_pane.is_some() {
            self.handle_popup_mouse(mouse);
            return;
        }
        if self.state.mode == Mode::Terminal {
            let action = self.state.view.agent_worktree_action_area.clone();
            if let Some(action) = action.filter(|action| {
                mouse.column >= action.rect.x
                    && mouse.column < action.rect.right()
                    && mouse.row >= action.rect.y
                    && mouse.row < action.rect.bottom()
            }) {
                if !matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left))
                    || !mouse.modifiers.is_empty()
                {
                    return;
                }
                let current = self.state.active.and_then(|workspace_idx| {
                    let workspace = self.state.workspaces.get(workspace_idx)?;
                    if self.state.file_manager.is_some()
                        || workspace.id != action.workspace_id
                        || !workspace.can_open_existing_worktree_from_cache()
                    {
                        return None;
                    }
                    let pane_id = workspace.focused_pane_id()?;
                    let terminal_id = self.state.terminal_id_for_pane(workspace_idx, pane_id)?;
                    self.state
                        .terminals
                        .get(&terminal_id)
                        .is_some_and(crate::terminal::TerminalState::is_agent_terminal)
                        .then_some((workspace_idx, pane_id, terminal_id))
                });
                if current.as_ref().is_some_and(|(_, pane_id, terminal_id)| {
                    *pane_id == action.pane_id && *terminal_id == action.terminal_id
                }) {
                    self.state.request_open_existing_worktree =
                        current.map(|(workspace_idx, _, _)| workspace_idx);
                }
                return;
            }

            if !matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left))
                || !mouse.modifiers.is_empty()
            {
                return self.handle_mouse_without_agent_frame_action(source_id, mouse);
            }
            let action = self.state.view.agent_attachment_action_area.clone();
            if let Some(action) = action.filter(|action| {
                mouse.column >= action.rect.x
                    && mouse.column < action.rect.right()
                    && mouse.row >= action.rect.y
                    && mouse.row < action.rect.bottom()
            }) {
                let current = self.state.active.and_then(|workspace_idx| {
                    let pane_id = self
                        .state
                        .workspaces
                        .get(workspace_idx)?
                        .focused_pane_id()?;
                    let terminal_id = self.state.terminal_id_for_pane(workspace_idx, pane_id)?;
                    Some((pane_id, terminal_id))
                });
                if current == Some((action.pane_id, action.terminal_id)) {
                    self.open_agent_attachment_picker_with_feedback();
                }
                return;
            }
        }

        self.handle_mouse_without_agent_frame_action(source_id, mouse);
    }

    /// Consume every event over live dock terrain: an enabled target
    /// activates on plain left press and opens its name popover on plain
    /// right press; everything else on the dock (disabled targets, modified
    /// presses, moves, wheels) is consumed fail-closed so no event can fall
    /// through to a surface underneath the dock chrome.
    fn handle_app_dock_mouse(&mut self, mouse: MouseEvent) -> bool {
        let Some(target) = self
            .state
            .view
            .app_dock_entry_areas
            .iter()
            .find(|entry| {
                mouse.column >= entry.rect.x
                    && mouse.column < entry.rect.x.saturating_add(entry.rect.width)
                    && mouse.row >= entry.rect.y
                    && mouse.row < entry.rect.y.saturating_add(entry.rect.height)
            })
            .copied()
        else {
            return false;
        };

        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left)
                if mouse.modifiers.is_empty() && target.enabled =>
            {
                self.state.activate_dock_app(target.app);
            }
            MouseEventKind::Down(MouseButton::Right)
                if mouse.modifiers.is_empty() && target.enabled =>
            {
                self.state.context_menu = Some(crate::app::state::ContextMenuState {
                    kind: crate::app::state::ContextMenuKind::AppDock { app: target.app },
                    x: mouse.column,
                    y: mouse.row,
                    list: crate::app::state::MenuListState::new(0),
                });
                self.state.enter_overlay_mode(Mode::ContextMenu);
            }
            _ => {}
        }
        true
    }

    /// Consume every event over a live bar section: a plain left press runs
    /// that section's action, and everything else over the chrome is consumed
    /// so no event can fall through a bar onto the surface behind it.
    ///
    /// Two things are deliberately NOT claimed, both for the same reason the
    /// frozen precedence puts an active capture above the positional hit: a
    /// gesture that began somewhere else already has an owner, and taking its
    /// events away mid-flight would strand it half-finished when the pointer
    /// merely crossed the chrome on its way somewhere.
    // TP-CHROME-43/44: everything over a section stops at the bar, and an
    // action that cannot start says so instead of failing quietly.
    fn handle_bar_section_mouse(&mut self, mouse: MouseEvent) -> bool {
        if self.state.shell_resize_active() || self.state.drag.is_some() {
            return false;
        }
        if matches!(mouse.kind, MouseEventKind::Drag(_)) {
            return false;
        }

        // Left asks for the action; right asks which presentation it opens in.
        // Anything else over a bar is consumed without being acted on, so the
        // gesture is an Option rather than an early return: the bar still has
        // to answer "is this over me at all" for events it will not run.
        let gesture = match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => Some(SectionGesture::Primary),
            MouseEventKind::Down(MouseButton::Right) => Some(SectionGesture::Secondary),
            _ => None,
        };
        // Whether a position is over a section at all is positional, and the
        // same for both gestures — so probing with either answers it. Pinned by
        // a control test rather than left as a reading of this comment.
        let click = self.state.bar_section_click_at(
            ratatui::layout::Position::new(mouse.column, mouse.row),
            gesture.unwrap_or(SectionGesture::Primary),
        );
        if matches!(click, BarSectionClick::Elsewhere) {
            return false;
        }
        if gesture.is_none() {
            return true;
        }
        // A modified press is how the person reaches the terminal underneath
        // other chrome, so it is consumed here rather than acted on: the bar
        // does not own a gesture it was not given.
        if !mouse.modifiers.is_empty() {
            return true;
        }

        match click {
            BarSectionClick::Elsewhere | BarSectionClick::Inert => {}
            BarSectionClick::OpenTab { argv } => {
                if let Err(err) = self.open_argv_in_new_tab(&argv) {
                    self.warn_about_bar_section_action(
                        "bar section action failed",
                        err.to_string(),
                    );
                }
            }
            BarSectionClick::InvokePlugin { action } => {
                if let Err(err) = self.invoke_plugin_action_from_bar_section(action) {
                    self.warn_about_bar_section_action("bar section action failed", err);
                }
            }
            BarSectionClick::PopupAlreadyOpen => {
                self.warn_about_bar_section_action(
                    "a popup is already open",
                    "close it before opening another from the bar",
                );
            }
            BarSectionClick::OpenPopup {
                argv,
                width,
                height,
            } => {
                if let Err(err) = self.spawn_popup_argv_command(
                    &argv,
                    None,
                    Vec::new(),
                    crate::app::popup::PopupGeometry { width, height },
                ) {
                    self.warn_about_bar_section_action(
                        "bar section action failed",
                        err.to_string(),
                    );
                }
            }
        }
        true
    }

    fn warn_about_bar_section_action(&mut self, title: &str, context: impl Into<String>) {
        let previous_toast = self.state.toast.clone();
        self.state.toast = Some(crate::app::state::ToastNotification {
            kind: crate::app::state::ToastKind::NeedsAttention,
            title: title.to_string(),
            context: context.into(),
            position: None,
            target: None,
        });
        self.sync_toast_deadline(previous_toast);
    }

    fn handle_mouse_without_agent_frame_action(
        &mut self,
        source_id: super::InputSourceId,
        mouse: MouseEvent,
    ) {
        if self.handle_overlay_mouse(mouse) {
            return;
        }

        // While a topmost blocking overlay owns the mouse, every background
        // pre-branch (file manager surface, divider gestures, URL clicks)
        // stays inert. The overlay's own interactions live in the
        // mode-guarded state dispatch below.
        let blocking_overlay = self
            .state
            .shell_mouse_input_owner(ratatui::layout::Position::new(mouse.column, mouse.row))
            == ShellInputOwner::TopmostOverlay;

        // An open mobile drawer is painted above every surface, so it owns
        // the mouse the same way: with Files open underneath, the file
        // manager's pre-branch used to eat the drawer's taps and every
        // segment and row fell through dead — paint order and input order
        // must not disagree about what is on top (TP-MOB-96).
        let mobile_drawer_owns_mouse = self.state.view.layout
            == crate::app::state::ViewLayout::Mobile
            && self.state.mobile_drawer.is_open();

        if !blocking_overlay && !mobile_drawer_owns_mouse {
            if self.handle_app_dock_mouse(mouse) {
                return;
            }

            if self.handle_bar_section_mouse(mouse) {
                return;
            }

            match self.handle_file_manager_mouse(mouse) {
                file_manager::FileManagerMouseDispatch::NotHandled => {}
                file_manager::FileManagerMouseDispatch::Consumed => return,
                file_manager::FileManagerMouseDispatch::RowAction { action, entry_path } => {
                    let _ = self.dispatch_file_manager_row_action(action, entry_path);
                    return;
                }
                file_manager::FileManagerMouseDispatch::HeaderAction(action) => {
                    let _ = self.dispatch_file_manager_header_action(action);
                    return;
                }
            }

            if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left))
                && self.state.on_sidebar_divider(mouse.column, mouse.row)
            {
                let now = std::time::Instant::now();
                let is_double_click = self.last_sidebar_divider_click.is_some_and(|last| {
                    now.duration_since(last) <= super::SIDEBAR_DOUBLE_CLICK_WINDOW
                });
                self.last_sidebar_divider_click = Some(now);

                if is_double_click {
                    self.state.reset_sidebar_resize_to_preferred();
                    return;
                }
            }

            if self.handle_modified_url_click(source_id, mouse) {
                return;
            }
        }

        let handled_pane_double_click = self.handle_pane_double_click(mouse);
        if !handled_pane_double_click {
            self.focus_pane_before_mouse_press(mouse);
        }

        let previous_agent_panel_sort = self.state.agent_panel_sort;
        let previous_settings_section = self.state.settings.section;
        if !handled_pane_double_click {
            if let Some(action) = self.state.handle_mouse(&mut self.terminal_runtimes, mouse) {
                match action {
                    MouseAction::NewWorkspace => {
                        self.begin_tui_workspace_create("tui.mouse.workspace.create")
                    }
                    MouseAction::Settings(action) => match action {
                        SettingsAction::SaveTheme(name) => self.save_theme(&name),
                        SettingsAction::SaveSound(enabled) => self.save_sound(enabled),
                        SettingsAction::SaveToastDelivery(delivery) => {
                            self.save_toast_delivery(delivery)
                        }
                        SettingsAction::SavePreviewPlacement(placement) => {
                            self.save_preview_placement(placement)
                        }
                        SettingsAction::SaveAgentBorderLabels(enabled) => {
                            self.save_agent_border_labels(enabled)
                        }
                        SettingsAction::SavePaneHistory(enabled) => {
                            self.save_pane_history_persistence(enabled)
                        }
                        SettingsAction::SaveSwitchAsciiInputSourceInPrefix(enabled) => {
                            self.save_switch_ascii_input_source_in_prefix(enabled)
                        }
                        SettingsAction::InstallRecommendedIntegrations => {
                            self.install_recommended_integrations()
                        }
                    },
                    MouseAction::FocusWorkspace { ws_idx } => {
                        self.focus_workspace_idx_via_api(ws_idx)
                    }
                    MouseAction::FocusTab { tab_idx } => {
                        // TP-FTAB-INPUT-01: focusing a terminal tab returns the
                        // stage to the terminal workspace. Every resident app
                        // instance stays in the strip to switch back to.
                        self.state.show_terminal_workspace();
                        self.focus_tab_idx_via_api(tab_idx)
                    }
                    MouseAction::FocusPane { ws_idx, pane_id } => {
                        self.focus_pane_internal_via_api(ws_idx, pane_id)
                    }
                    MouseAction::FocusToastTarget => self.focus_toast_target_via_api(),
                    MouseAction::MoveWorkspace {
                        source_ws_idx,
                        insert_idx,
                    } => self.move_workspace_via_api(source_ws_idx, insert_idx),
                    MouseAction::MoveTab {
                        ws_idx,
                        source_tab_idx,
                        insert_idx,
                    } => self.move_tab_via_api(ws_idx, source_tab_idx, insert_idx),
                    MouseAction::SetSplitRatio { path, ratio } => {
                        self.set_split_ratio_via_api(path, ratio)
                    }
                    MouseAction::RenameModal(action) => {
                        self.apply_rename_mouse_action_via_api(action)
                    }
                    MouseAction::ConfirmCloseAccept => self.confirm_close_accept_via_api(),
                    MouseAction::AgentReferencePickerActivate => {
                        let _ = self.activate_agent_reference_picker_selection();
                    }
                    MouseAction::TailscaleSendActivate => {
                        let _ = file_manager::send_to_selected_device(&mut self.state);
                    }
                    MouseAction::ContextMenu { menu, idx } => {
                        self.apply_context_menu_action_via_api(menu, idx)
                    }
                    MouseAction::ToggleProjectsActives => {
                        self.state.projects_actives_only = !self.state.projects_actives_only;
                        let enabled = self.state.projects_actives_only;
                        self.save_projects_actives_only(enabled);
                    }
                    MouseAction::ToggleSpacesFocus => {
                        self.state.spaces_focus_only = !self.state.spaces_focus_only;
                        let enabled = self.state.spaces_focus_only;
                        self.save_spaces_focus_only(enabled);
                    }
                }
            }
            if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left))
                && self
                    .state
                    .selection
                    .as_ref()
                    .is_none_or(crate::selection::Selection::is_in_progress)
            {
                self.selection_highlight_clear_deadline = None;
            }
        }
        if previous_settings_section != crate::app::state::SettingsSection::Integrations
            && self.state.settings.section == crate::app::state::SettingsSection::Integrations
        {
            self.refresh_integration_recommendations();
        }
        if self.state.agent_panel_sort != previous_agent_panel_sort {
            self.save_agent_panel_sort(self.state.agent_panel_sort);
        }

        if let Some(content) = self.state.request_clipboard_write.take() {
            if self
                .event_tx
                .try_send(crate::events::AppEvent::ClipboardWrite { content })
                .is_err()
            {
                tracing::warn!("failed to queue clipboard write event");
            }
        }

        // Sync autoscroll deadline with state (mouse handler may have
        // set or cleared selection_autoscroll during handle_mouse).
        if self.state.selection_autoscroll.is_none() {
            self.selection_autoscroll_deadline = None;
        } else if self.selection_autoscroll_deadline.is_none() {
            self.selection_autoscroll_deadline =
                Some(std::time::Instant::now() + super::SELECTION_AUTOSCROLL_INTERVAL);
        }
    }

    fn open_agent_attachment_picker_with_feedback(&mut self) {
        let previous_toast = self.state.toast.clone();
        let _ = self.state.open_agent_attachment_picker();
        self.sync_toast_deadline(previous_toast);
    }

    // TP-CHROME-49/50/51: the first press outside asks, the second closes, the
    // request belongs to one popup, and returning to it cancels the request.
    fn handle_popup_mouse(&mut self, mouse: MouseEvent) {
        let Some((outer, inner)) =
            crate::ui::popup_pane_rects(&self.state, self.state.view.terminal_area)
        else {
            return;
        };
        let outside_popup = mouse.column < outer.x
            || mouse.column >= outer.x.saturating_add(outer.width)
            || mouse.row < outer.y
            || mouse.row >= outer.y.saturating_add(outer.height);
        if outside_popup {
            // Clicking past the popup means "I am done here". The dismissal
            // is delivered as Esc INTO the popup's own app rather than a
            // kill from outside: the editors bind Esc to save-then-quit, so
            // the popup closes with the last state safely on disk — closing
            // the pane out from under them would race their final write.
            //
            // Not every program reads Esc that way. btop and htop quit on `q`
            // and ignore Esc entirely, so for them the press above did nothing
            // visible and the surface looked stuck. The second press closes.
            //
            // An editor never reaches that second press: if Esc quit it, the
            // popup is already gone, and this whole path is unreachable while
            // `popup_pane` is None. So the guarantee above is not weakened —
            // it is exhausted first, and only then overridden.
            if matches!(mouse.kind, MouseEventKind::Down(_)) {
                // Remembered against the popup's own terminal, not as a bare
                // flag: a different popup is a different id, so a dismissal
                // asked of one can never be spent on its successor. Nobody has
                // to remember to reset it.
                let popup_id = self
                    .state
                    .popup_pane
                    .as_ref()
                    .map(|popup| popup.terminal_id.clone());
                let already_asked = popup_id.is_some() && self.popup_dismiss_requested == popup_id;

                if already_asked {
                    self.close_popup_pane();
                    self.popup_dismiss_requested = None;
                } else if let Some(rt) = self.popup_runtime() {
                    let _ = rt.try_send_bytes(bytes::Bytes::from_static(b"\x1b"));
                    self.popup_dismiss_requested = popup_id;
                    // A two-step gesture nobody is told about is a gesture
                    // nobody finds. This is the sentence that turns "it is
                    // stuck" into "click again".
                    //
                    // NeedsAttention rather than a new Info kind: the toast
                    // kinds are a closed enum matched exhaustively in several
                    // places and carried across the API, and a hint does not
                    // earn that. It is also not a misuse — something on screen
                    // is waiting for the person to act on it.
                    let previous_toast = self.state.toast.clone();
                    self.state.toast = Some(crate::app::state::ToastNotification {
                        kind: crate::app::state::ToastKind::NeedsAttention,
                        title: "popup still open".to_string(),
                        context: "click outside again to close it".to_string(),
                        position: None,
                        target: None,
                    });
                    self.sync_toast_deadline(previous_toast);
                } else {
                    self.close_popup_pane();
                }
            }
            return;
        }
        if mouse.column < inner.x
            || mouse.column >= inner.x.saturating_add(inner.width)
            || mouse.row < inner.y
            || mouse.row >= inner.y.saturating_add(inner.height)
        {
            return;
        }
        // Going back into the popup cancels a dismissal that was asked for and
        // not answered. Esc does not always mean quit: an editor may have
        // opened an "unsaved changes?" prompt with it, and the press that
        // answers that prompt lands in here. Without this, the next click
        // outside would close the pane on top of the very question the
        // guarantee above exists to protect.
        if matches!(mouse.kind, MouseEventKind::Down(_)) {
            self.popup_dismiss_requested = None;
        }
        let Some(rt) = self.popup_runtime() else {
            self.close_popup_pane();
            return;
        };
        let column = mouse.column.saturating_sub(inner.x);
        let row = mouse.row.saturating_sub(inner.y);
        let bytes = match mouse.kind {
            MouseEventKind::ScrollUp
            | MouseEventKind::ScrollDown
            | MouseEventKind::ScrollLeft
            | MouseEventKind::ScrollRight => match rt.wheel_routing() {
                Some(crate::pane::WheelRouting::MouseReport) => {
                    rt.encode_mouse_wheel(mouse.kind, column, row, mouse.modifiers)
                }
                Some(crate::pane::WheelRouting::AlternateScroll) => {
                    rt.encode_alternate_scroll(mouse.kind)
                }
                Some(crate::pane::WheelRouting::HostScroll) | None => {
                    let lines_per_notch = self.state.mouse_scroll_lines;
                    match mouse.kind {
                        MouseEventKind::ScrollUp => rt.scroll_up(lines_per_notch),
                        MouseEventKind::ScrollDown => rt.scroll_down(lines_per_notch),
                        _ => {}
                    }
                    return;
                }
            },
            MouseEventKind::Down(_) | MouseEventKind::Up(_) | MouseEventKind::Drag(_) => {
                rt.encode_mouse_button(mouse.kind, column, row, mouse.modifiers)
            }
            MouseEventKind::Moved => {
                rt.encode_mouse_motion(mouse.kind, column, row, mouse.modifiers)
            }
        };
        let Some(bytes) = bytes else {
            return;
        };
        rt.scroll_reset();
        if let Err(err) = rt.try_send_bytes(Bytes::from(bytes)) {
            warn!(err = %err, kind = ?mouse.kind, "failed to forward popup mouse event");
        }
    }

    fn focus_pane_before_mouse_press(&mut self, mouse: MouseEvent) {
        if !matches!(self.state.mode, Mode::Terminal | Mode::Resize)
            || !matches!(
                mouse.kind,
                MouseEventKind::Down(MouseButton::Left | MouseButton::Middle)
            )
        {
            return;
        }

        let Some(pane_id) = self
            .state
            .pane_at(mouse.column, mouse.row)
            .map(|info| info.id)
        else {
            return;
        };
        let Some(ws_idx) = self.state.active else {
            return;
        };

        // Focus through the runtime API before an application can consume its press.
        self.focus_pane_internal_via_api(ws_idx, pane_id);
    }

    fn handle_modified_url_click(
        &mut self,
        source_id: super::InputSourceId,
        mouse: MouseEvent,
    ) -> bool {
        if self.state.mode != Mode::Terminal
            || !matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left))
            || !mouse.modifiers.contains(modified_url_click_modifier())
        {
            return false;
        }

        let Some(info) = self.state.pane_at(mouse.column, mouse.row).cloned() else {
            return false;
        };
        let viewport_row = mouse.row.saturating_sub(info.inner_rect.y);
        let col = mouse.column.saturating_sub(info.inner_rect.x);
        let Some(url) =
            self.state
                .url_at_pane_cell(&self.terminal_runtimes, info.id, viewport_row, col)
        else {
            return false;
        };

        self.last_pane_click = None;
        self.pending_url_click_sources.insert(source_id);
        match self.invoke_plugin_link_handler_for_url(&url, info.id) {
            Ok(true) => return true,
            Ok(false) => {}
            Err(err) => {
                tracing::warn!(err = %err, url = %url, "failed to invoke plugin link handler");
            }
        }
        if let Err(err) = crate::platform::open_url(&url) {
            tracing::warn!(err = %err, url = %url, "failed to open pane URL");
        }
        true
    }

    fn handle_pane_double_click(&mut self, mouse: MouseEvent) -> bool {
        // A pane press stops being a double-click candidate once it becomes
        // a drag or completes as a real text selection.
        match mouse.kind {
            MouseEventKind::Drag(MouseButton::Left) => {
                self.last_pane_click = None;
                return false;
            }
            MouseEventKind::Up(MouseButton::Left)
                if self
                    .state
                    .selection
                    .as_ref()
                    .is_some_and(|selection| selection.is_visible()) =>
            {
                self.last_pane_click = None;
                return false;
            }
            _ => {}
        }

        // Only terminal-pane left-clicks can start this gesture; other clicks
        // should keep their existing mouse behavior and clear stale candidates.
        let Some(click) = self.pane_click_candidate(mouse) else {
            return false;
        };

        // Require the second click to land near the first click in the same pane
        // and within the double-click window so adjacent interactions do not copy.
        if !self.take_pane_double_click(click) {
            return false;
        }

        // Preserve a short highlight after copying so the user gets visible
        // confirmation without leaving a persistent selection behind.
        self.copy_double_clicked_word(click)
    }

    fn pane_click_candidate(&mut self, mouse: MouseEvent) -> Option<PaneClickState> {
        if !matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
            return None;
        }

        if !mouse.modifiers.is_empty() {
            self.last_pane_click = None;
            return None;
        }

        if self.state.mode != Mode::Terminal {
            self.last_pane_click = None;
            return None;
        }

        let Some(info) = self.state.pane_at(mouse.column, mouse.row).cloned() else {
            self.last_pane_click = None;
            return None;
        };

        Some(PaneClickState {
            pane_id: info.id,
            viewport_row: mouse.row - info.inner_rect.y,
            col: mouse.column - info.inner_rect.x,
            at: std::time::Instant::now(),
        })
    }

    fn take_pane_double_click(&mut self, click: PaneClickState) -> bool {
        if !self
            .last_pane_click
            .is_some_and(|last| last.is_double_click_for(click))
        {
            self.last_pane_click = Some(click);
            return false;
        }

        self.last_pane_click = None;
        true
    }

    fn copy_double_clicked_word(&mut self, click: PaneClickState) -> bool {
        let copied = self.state.copy_word_at_pane_cell(
            &self.terminal_runtimes,
            click.pane_id,
            click.viewport_row,
            click.col,
        );
        if copied {
            self.selection_highlight_clear_deadline =
                Some(std::time::Instant::now() + super::PANE_COPY_HIGHLIGHT_DURATION);
        }
        copied
    }
}

pub(crate) fn is_modal_paste_shortcut(key: &KeyEvent) -> bool {
    if !matches!(key.code, KeyCode::Char('v' | 'V')) {
        return false;
    }

    #[cfg(target_os = "macos")]
    {
        key.modifiers.contains(KeyModifiers::SUPER) || key.modifiers.contains(KeyModifiers::CONTROL)
    }

    #[cfg(not(target_os = "macos"))]
    {
        key.modifiers.contains(KeyModifiers::CONTROL)
    }
}

pub(crate) fn modal_paste_target_active(state: &AppState) -> bool {
    match state.mode {
        Mode::RenameWorkspace
        | Mode::RenameTab
        | Mode::RenamePane
        | Mode::RenameFile
        | Mode::NewLinkedWorktree => true,
        Mode::OpenExistingWorktree => state
            .worktree_open
            .as_ref()
            .is_some_and(|open| open.search_focused),
        Mode::Navigator => state.navigator.search_focused,
        Mode::KeybindHelp => state.keybind_help.search_focused,
        Mode::Copy => state
            .copy_mode
            .as_ref()
            .is_some_and(|copy_mode| copy_mode.search.prompt.is_some()),
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// Mouse handling
// ---------------------------------------------------------------------------

// Note: split_pane needs runtime (event_tx for PTY spawn), so it lives on App
impl AppState {
    #[cfg(test)]
    pub(crate) fn split_pane(
        &mut self,
        terminal_runtimes: &mut crate::terminal::TerminalRuntimeRegistry,
        direction: Direction,
    ) {
        // Actual PTY spawning happens in Workspace::split_focused
        // which needs events channel — this is called from navigate_key
        // where we don't have async context, so the workspace handles it
        let (rows, cols) = self.estimate_pane_size();
        let new_rows = (rows / 2).max(4);
        let new_cols = (cols / 2).max(10);

        let follow_cwd = self
            .active
            .and_then(|i| self.workspaces.get(i))
            .and_then(|ws| {
                let tab = ws.active_tab()?;
                let terminal_id = tab.terminal_id(tab.layout.focused())?;
                super::creation::launch_cwd_for_terminal(
                    terminal_id,
                    &self.terminals,
                    terminal_runtimes,
                )
            });
        let cwd = Some(super::creation::resolve_new_terminal_cwd(
            &self.new_terminal_cwd,
            follow_cwd,
        ));

        let previous_focus = self.current_pane_focus_target();
        if let Some(ws_idx) = self.active {
            let Some(ws) = self.workspaces.get_mut(ws_idx) else {
                return;
            };
            if let Ok(new_pane) = ws.split_focused(
                direction,
                new_rows,
                new_cols,
                cwd,
                self.pane_scrollback_limit_bytes,
                self.host_terminal_theme,
                crate::pane::PaneShellConfig::new(&self.default_shell, self.shell_mode),
                Vec::new(),
            ) {
                let new_id = new_pane.pane_id;
                terminal_runtimes.insert(new_pane.terminal.id.clone(), new_pane.runtime);
                self.remove_alias_shadowed_by_new_pane(new_id);
                self.terminals
                    .insert(new_pane.terminal.id.clone(), new_pane.terminal);
                self.record_pane_focus_change(previous_focus, ws_idx, new_id);
                self.mark_session_dirty();
                self.mode = Mode::Terminal;
            }
        }
    }
}

#[cfg(test)]
fn state_with_workspaces(names: &[&str]) -> AppState {
    let mut state = AppState::test_new();
    state.workspaces = names
        .iter()
        .map(|name| crate::workspace::Workspace::test_new(name))
        .collect();
    if !state.workspaces.is_empty() {
        state.active = Some(0);
        state.selected = 0;
        state.mode = Mode::Navigate;
    }
    state
}

#[cfg(test)]
fn app_for_mouse_test() -> App {
    let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
    let mut app = App::new(
        &crate::config::Config::default(),
        true,
        None,
        api_rx,
        crate::api::EventHub::default(),
    );
    app.state.mode = Mode::Terminal;
    app.state.update_available = None;
    app.state.latest_release_notes_available = false;
    // The divider and shell-resize tests are about drag/commit mechanics, not
    // about whatever width the product happens to ship. Pinning the width here
    // keeps their coordinates meaningful when the default moves (it moved to
    // 30 for the Spaces tree, TP-TREE-13).
    app.state.sidebar_width = 26;
    app.state.default_sidebar_width = 26;
    app.state.view.sidebar_rect = ratatui::layout::Rect::new(0, 0, 26, 20);
    app.state.view.terminal_area = ratatui::layout::Rect::new(26, 0, 80, 20);
    app
}

#[cfg(test)]
fn mouse(
    kind: crossterm::event::MouseEventKind,
    col: u16,
    row: u16,
) -> crossterm::event::MouseEvent {
    crossterm::event::MouseEvent {
        kind,
        column: col,
        row,
        modifiers: crossterm::event::KeyModifiers::empty(),
    }
}

#[cfg(test)]
fn numbered_lines_bytes(count: usize) -> Vec<u8> {
    (0..count)
        .map(|i| format!("{i:06}\r\n"))
        .collect::<String>()
        .into_bytes()
}

#[cfg(test)]
fn capture_snapshot(state: &AppState) -> crate::persist::SessionSnapshot {
    let terminal_runtimes = crate::terminal::TerminalRuntimeRegistry::new();
    crate::persist::capture(
        &state.workspaces,
        &state.terminals,
        &terminal_runtimes,
        state.active,
        state.selected,
        state.sidebar_width,
        &state.shell_presentation,
        state.sidebar_section_split,
        state.collapsed_space_keys.clone(),
        state.collapsed_project_keys.clone(),
        state.files_tab_snapshot(),
    )
}

#[cfg(test)]
fn root_layout_ratio(snapshot: &crate::persist::SessionSnapshot) -> Option<f32> {
    match &snapshot.workspaces.first()?.tabs.first()?.layout {
        crate::persist::LayoutSnapshot::Split { ratio, .. } => Some(*ratio),
        crate::persist::LayoutSnapshot::Pane(_) => None,
    }
}

#[cfg(test)]
fn unique_temp_path(name: &str) -> std::path::PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!("herdr-{name}-{}-{nanos}", std::process::id()))
}

#[cfg(test)]
#[cfg(unix)]
fn wait_for_file(path: &std::path::Path) -> String {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    while std::time::Instant::now() < deadline {
        if let Ok(content) = std::fs::read_to_string(path) {
            if !content.is_empty() {
                return content;
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    panic!("timed out waiting for {}", path.display());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn active_shell_resize_capture_owns_key_before_terminal_dispatch() {
        let mut app = app_for_mouse_test();
        crate::ui::compute_view(&mut app.state, ratatui::layout::Rect::new(0, 0, 106, 40));
        assert!(app
            .state
            .begin_sidebar_resize(ratatui::layout::Position::new(25, 5)));
        app.state.session_dirty = false;

        app.handle_key(TerminalKey::new(KeyCode::Right, KeyModifiers::NONE))
            .await;

        assert_eq!(app.state.shell_resize_preview_width(), Some(27));
        assert_eq!(app.state.sidebar_width, 26);
        assert!(!app.state.session_dirty);
    }

    #[test]
    fn shell_resize_keyboard_supports_both_directions_and_vim_aliases() {
        let mut state = AppState::test_new();
        crate::ui::compute_view(&mut state, ratatui::layout::Rect::new(0, 0, 106, 40));
        assert!(state.begin_sidebar_resize(ratatui::layout::Position::new(25, 5)));
        state.session_dirty = false;

        for (code, expected) in [
            (KeyCode::Left, 25),
            (KeyCode::Char('h'), 24),
            (KeyCode::Char('l'), 25),
            (KeyCode::Right, 26),
        ] {
            assert!(state.handle_shell_resize_key(KeyEvent::new(code, KeyModifiers::NONE)));
            assert_eq!(state.shell_resize_preview_width(), Some(expected));
        }
        assert_eq!(state.sidebar_width, 26);
        assert!(!state.session_dirty);
    }

    // SF4.2-03: a topmost blocking overlay owns keyboard input ahead of an
    // active background capture. A launcher click can open the global menu
    // while a sidebar resize capture stays active, so overlay keys must never
    // adjust or cancel the capture underneath.
    #[tokio::test]
    async fn overlay_blocks_background_keyboard_shortcut() {
        let mut app = app_for_mouse_test();
        crate::ui::compute_view(&mut app.state, ratatui::layout::Rect::new(0, 0, 106, 40));
        assert!(app
            .state
            .begin_sidebar_resize(ratatui::layout::Position::new(25, 5)));

        // Control: without an overlay the active capture consumes the key.
        app.handle_key(TerminalKey::new(KeyCode::Right, KeyModifiers::NONE))
            .await;
        assert_eq!(app.state.shell_resize_preview_width(), Some(27));

        modal::open_global_menu(&mut app.state);
        assert_eq!(app.state.mode, Mode::GlobalMenu);

        app.handle_key(TerminalKey::new(KeyCode::Right, KeyModifiers::NONE))
            .await;
        assert_eq!(
            (app.state.shell_resize_preview_width(), app.state.mode),
            (Some(27), Mode::GlobalMenu),
            "an open global menu must consume background keys ahead of the capture"
        );

        app.handle_key(TerminalKey::new(KeyCode::Esc, KeyModifiers::NONE))
            .await;
        assert_ne!(
            app.state.mode,
            Mode::GlobalMenu,
            "escape must close the overlay"
        );
        assert!(
            app.state.shell_resize_active(),
            "escape must never cancel the background capture through the overlay"
        );

        // With the overlay gone the capture owns keys again.
        app.handle_key(TerminalKey::new(KeyCode::Right, KeyModifiers::NONE))
            .await;
        assert_eq!(app.state.shell_resize_preview_width(), Some(28));
    }

    #[tokio::test]
    async fn context_menu_escape_owns_key_before_active_shell_resize_capture() {
        let mut app = app_for_mouse_test();
        crate::ui::compute_view(&mut app.state, ratatui::layout::Rect::new(0, 0, 106, 40));
        assert!(app
            .state
            .begin_sidebar_resize(ratatui::layout::Position::new(25, 5)));
        app.state.mode = Mode::ContextMenu;

        app.handle_key(TerminalKey::new(KeyCode::Esc, KeyModifiers::NONE))
            .await;

        assert!(app.state.shell_resize_active());
        assert_ne!(app.state.mode, Mode::ContextMenu);
    }

    fn test_app() -> App {
        App::new(
            &crate::config::Config::default(),
            true,
            None,
            tokio::sync::mpsc::unbounded_channel().1,
            crate::api::EventHub::default(),
        )
    }

    /// An app whose top bar is one cell tall and divided into a single ten-cell
    /// section that opens a popup, with the geometry computed.
    fn app_with_a_clickable_top_bar_section() -> App {
        let mut section = crate::config::ShellBarSectionConfig {
            kind: "fixed".to_string(),
            cells: 10,
            ..Default::default()
        };
        section.action.kind = "popup".to_string();
        section.action.argv = vec!["btop".to_string()];

        let bars = crate::config::ShellBarsConfig {
            top: crate::config::ShellBarConfig {
                enabled: true,
                size: 1,
                border: false,
                color: String::new(),
                gradient: Vec::new(),
                sections: vec![section],
                ..Default::default()
            },
            ..Default::default()
        };

        let mut app = test_app();
        app.state.shell_presentation = crate::ui::shell::ShellPresentationState::from_restored(
            26,
            false,
            None,
            crate::ui::shell::ShellBars::from_config(&bars),
        );
        app.state.shell_bar_chrome = crate::ui::shell::ShellBarChrome::from_config(&bars);
        crate::ui::compute_view(&mut app.state, ratatui::layout::Rect::new(0, 0, 106, 40));
        app
    }

    fn plugin_section_chrome(command: &str) -> crate::ui::shell::ShellBarChrome {
        let mut section = crate::config::ShellBarSectionConfig {
            kind: "fixed".to_string(),
            cells: 10,
            ..Default::default()
        };
        section.action.kind = "plugin".to_string();
        section.action.command = command.to_string();
        crate::ui::shell::ShellBarChrome::from_config(&crate::config::ShellBarsConfig {
            top: crate::config::ShellBarConfig {
                enabled: true,
                size: 1,
                border: false,
                color: String::new(),
                gradient: Vec::new(),
                sections: vec![section],
                ..Default::default()
            },
            ..Default::default()
        })
    }

    // TC-66-15 · an icon that dies silently under the finger is the worst thing
    // this surface can do: the person presses, nothing happens, and there is no
    // way to learn why. Reached through the real resolver with an id no
    // installed plugin declares — which is exactly what a config written before
    // the plugin was installed, or after it was removed, looks like.
    // TP-CHROME-85: a plugin action that cannot be resolved says so and opens
    // nothing.
    #[test]
    fn a_plugin_section_that_names_an_unknown_action_says_so_and_opens_nothing() {
        let mut app = app_with_a_clickable_top_bar_section();
        app.state.shell_bar_chrome = plugin_section_chrome("nobody.installed.this");

        let consumed = app.handle_bar_section_mouse(bar_mouse(
            MouseEventKind::Down(MouseButton::Left),
            4,
            0,
            KeyModifiers::NONE,
        ));

        assert!(
            consumed,
            "the bar owns the press whether or not the action behind it ran"
        );
        assert!(
            app.state.popup_pane.is_none(),
            "a failed plugin invocation must not leave a popup behind"
        );
        let toast = app
            .state
            .toast
            .as_ref()
            .expect("a failure the person caused by clicking must be said out loud");
        assert_eq!(toast.kind, crate::app::state::ToastKind::NeedsAttention);
        assert!(
            toast.context.contains("not found"),
            "the message must carry the resolver's own reason rather than a \
             generic one: {:?}",
            toast.context
        );
    }

    // TC-66-17 · the second gesture reaches the same handler and must stop
    // there. Asserted by absence — no toast — because a right press that
    // silently invoked the plugin would look identical to a working left press,
    // and the two would only diverge in the plugin's own logs.
    // TP-CHROME-86: a secondary press on a plugin section is consumed without
    // invoking anything.
    #[test]
    fn a_secondary_press_on_a_plugin_section_invokes_nothing() {
        let mut app = app_with_a_clickable_top_bar_section();
        app.state.shell_bar_chrome = plugin_section_chrome("nobody.installed.this");

        let consumed = app.handle_bar_section_mouse(bar_mouse(
            MouseEventKind::Down(MouseButton::Right),
            4,
            0,
            KeyModifiers::NONE,
        ));

        assert!(consumed, "the bar still owns an event it will not act on");
        assert!(
            app.state.toast.is_none(),
            "nothing was attempted, so there is nothing to report: a complaint \
             here would mean the invocation was tried and failed"
        );
    }

    fn bar_mouse(
        kind: MouseEventKind,
        column: u16,
        row: u16,
        modifiers: KeyModifiers,
    ) -> MouseEvent {
        MouseEvent {
            kind,
            column,
            row,
            modifiers,
        }
    }

    // The last hop, and the one a mutation caught nobody was watching: the
    // size reaching the click intent proves nothing about it reaching the call
    // that opens the popup. Dropping it there is invisible — the popup simply
    // opens at the default — so this observes the size where it lands, in the
    // popup's own state.
    //
    // Spawns for real, following `direct_custom_popup_command_closes_after_exit`:
    // the geometry is decided inside the spawn, so a fake runtime installed
    // afterwards would skip the very step under test. `/bin/true` exits at once
    // and the runtimes are shut down before the test returns.
    #[cfg(unix)]
    #[tokio::test]
    async fn a_sized_section_opens_its_popup_at_the_size_it_asked_for() {
        let mut section = crate::config::ShellBarSectionConfig {
            kind: "fixed".to_string(),
            cells: 10,
            ..Default::default()
        };
        section.action.kind = "popup".to_string();
        section.action.argv = vec!["/bin/true".to_string()];
        section.action.width = Some(crate::popup_size::PopupSize::Percent(80));
        section.action.height = Some(crate::popup_size::PopupSize::Cells(20));

        let bars = crate::config::ShellBarsConfig {
            top: crate::config::ShellBarConfig {
                enabled: true,
                size: 1,
                border: false,
                color: String::new(),
                gradient: Vec::new(),
                sections: vec![section],
                ..Default::default()
            },
            ..Default::default()
        };

        let mut app = test_app();
        app.state.default_shell = "/bin/sh".into();
        let (workspace, terminal, runtime) = crate::workspace::Workspace::new(
            std::env::current_dir().unwrap_or_else(|_| "/".into()),
            24,
            80,
            app.state.pane_scrollback_limit_bytes,
            app.state.host_terminal_theme,
            crate::pane::PaneShellConfig::new(&app.state.default_shell, app.state.shell_mode),
            app.event_tx.clone(),
            app.render_notify.clone(),
            app.render_dirty.clone(),
        )
        .expect("test workspace spawns");
        app.terminal_runtimes.insert(terminal.id.clone(), runtime);
        app.state.terminals.insert(terminal.id.clone(), terminal);
        app.state.workspaces = vec![workspace];
        app.state.active = Some(0);
        app.state.selected = 0;

        app.state.shell_presentation = crate::ui::shell::ShellPresentationState::from_restored(
            26,
            false,
            None,
            crate::ui::shell::ShellBars::from_config(&bars),
        );
        app.state.shell_bar_chrome = crate::ui::shell::ShellBarChrome::from_config(&bars);
        crate::ui::compute_view(&mut app.state, ratatui::layout::Rect::new(0, 0, 106, 40));

        let consumed = app.handle_bar_section_mouse(bar_mouse(
            MouseEventKind::Down(MouseButton::Left),
            4,
            0,
            KeyModifiers::NONE,
        ));

        assert!(consumed, "the press belongs to the bar");
        let popup = app
            .state
            .popup_pane
            .as_ref()
            .expect("the section's action must have opened a popup");
        assert_eq!(
            (popup.width, popup.height),
            (
                Some(crate::popup_size::PopupSize::Percent(80)),
                Some(crate::popup_size::PopupSize::Cells(20))
            ),
            "the popup must open at the size the section asked for, not the default"
        );

        for (_, runtime) in app.terminal_runtimes.drain() {
            runtime.shutdown();
        }
    }

    /// The same shape as `app_with_a_clickable_top_bar_section`, with a real
    /// workspace under it and a section that answers both gestures.
    ///
    /// Spawns for real, following `a_sized_section_opens_its_popup_at_the_size_it_asked_for`:
    /// the tab is created inside the handler, so a fake runtime installed
    /// afterwards would skip the step under test. Every runtime it starts is
    /// shut down by the caller before the test returns (C76's rule, in unit form).
    #[cfg(unix)]
    fn app_with_a_two_gesture_section(argv: &[&str]) -> App {
        let mut section = crate::config::ShellBarSectionConfig {
            kind: "fixed".to_string(),
            cells: 10,
            ..Default::default()
        };
        section.action.kind = "popup".to_string();
        section.action.argv = argv.iter().map(|argument| argument.to_string()).collect();
        section.action.secondary = "tab".to_string();

        let bars = crate::config::ShellBarsConfig {
            top: crate::config::ShellBarConfig {
                enabled: true,
                size: 1,
                border: false,
                color: String::new(),
                gradient: Vec::new(),
                sections: vec![section],
                ..Default::default()
            },
            ..Default::default()
        };

        let mut app = test_app();
        app.state.default_shell = "/bin/sh".into();
        let (workspace, terminal, runtime) = crate::workspace::Workspace::new(
            std::env::current_dir().unwrap_or_else(|_| "/".into()),
            24,
            80,
            app.state.pane_scrollback_limit_bytes,
            app.state.host_terminal_theme,
            crate::pane::PaneShellConfig::new(&app.state.default_shell, app.state.shell_mode),
            app.event_tx.clone(),
            app.render_notify.clone(),
            app.render_dirty.clone(),
        )
        .expect("test workspace spawns");
        app.terminal_runtimes.insert(terminal.id.clone(), runtime);
        app.state.terminals.insert(terminal.id.clone(), terminal);
        app.state.workspaces = vec![workspace];
        app.state.active = Some(0);
        app.state.selected = 0;

        app.state.shell_presentation = crate::ui::shell::ShellPresentationState::from_restored(
            26,
            false,
            None,
            crate::ui::shell::ShellBars::from_config(&bars),
        );
        app.state.shell_bar_chrome = crate::ui::shell::ShellBarChrome::from_config(&bars);
        crate::ui::compute_view(&mut app.state, ratatui::layout::Rect::new(0, 0, 106, 40));
        app
    }

    // TC-67-12/TC-67-13/TC-67-15 · THE WHOLE CHAIN, in the product's own terms.
    // A right press over the section creates a tab, registers its pane so every
    // surface that draws tabs can find it, and goes there.
    //
    // This is the only test that would have caught the single line the layer
    // started as: `handle_bar_section_mouse` accepted nothing but a left press,
    // so the pure decision below it could have been perfect and the gesture
    // would still have done nothing.
    //
    // `/bin/true` exits at once; the runtimes are shut down before returning.
    // TP-CHROME-67: a right press over a two-gesture section opens its command
    // in a new focused tab of the active workspace.
    #[cfg(unix)]
    #[tokio::test]
    async fn a_right_press_over_a_section_opens_its_command_in_a_new_tab() {
        let mut app = app_with_a_two_gesture_section(&["/bin/true"]);
        let tabs_before = app.state.workspaces[0].tabs.len();
        let terminals_before = app.state.terminals.len();

        let consumed = app.handle_bar_section_mouse(bar_mouse(
            MouseEventKind::Down(MouseButton::Right),
            4,
            0,
            KeyModifiers::NONE,
        ));

        assert!(consumed, "the press belongs to the bar");
        assert_eq!(
            app.state.workspaces[0].tabs.len(),
            tabs_before + 1,
            "the secondary gesture must have created exactly one tab"
        );
        assert_eq!(
            app.state.terminals.len(),
            terminals_before + 1,
            "an unregistered pane is one no surface can draw"
        );
        let new_tab = app.state.workspaces[0].tabs.len() - 1;
        assert_eq!(
            app.state.workspaces[0].active_tab_index(),
            new_tab,
            "the person asked to see this bigger; leaving them where they were \
             would read as the gesture having done nothing"
        );
        assert_eq!(
            app.state.mode,
            Mode::Terminal,
            "focus without the mode to use it is focus in name only"
        );
        assert!(
            app.state.popup_pane.is_none(),
            "the secondary gesture must not also have opened the primary one"
        );

        for (_, runtime) in app.terminal_runtimes.drain() {
            runtime.shutdown();
        }
    }

    // TC-67-16/TC-67-17 · the gesture is a plain press and nothing else. A
    // modified press is how the person reaches what is under the chrome, and a
    // release must not fire the action a second time.
    //
    // Both are consumed, and both must leave the workspace exactly as it was —
    // asserted on the tab count rather than on a flag, because the failure this
    // guards against is a tab that really did get created.
    // TP-CHROME-68: only a plain right press opens the secondary presentation.
    #[cfg(unix)]
    #[tokio::test]
    async fn only_a_plain_right_press_opens_the_second_presentation() {
        let mut app = app_with_a_two_gesture_section(&["/bin/true"]);
        let tabs_before = app.state.workspaces[0].tabs.len();

        for (name, kind, modifiers) in [
            (
                "modified right press",
                MouseEventKind::Down(MouseButton::Right),
                KeyModifiers::SHIFT,
            ),
            (
                "right release",
                MouseEventKind::Up(MouseButton::Right),
                KeyModifiers::NONE,
            ),
            (
                "right drag",
                MouseEventKind::Drag(MouseButton::Right),
                KeyModifiers::NONE,
            ),
            ("wheel", MouseEventKind::ScrollDown, KeyModifiers::NONE),
        ] {
            let consumed = app.handle_bar_section_mouse(bar_mouse(kind, 4, 0, modifiers));
            // A drag is the one gesture a bar deliberately does not claim: it
            // began somewhere else and already has an owner.
            if !matches!(kind, MouseEventKind::Drag(_)) {
                assert!(
                    consumed,
                    "{name} over a section must be consumed by the bar"
                );
            }
            assert_eq!(
                app.state.workspaces[0].tabs.len(),
                tabs_before,
                "{name} must not be mistaken for the secondary gesture"
            );
        }

        for (_, runtime) in app.terminal_runtimes.drain() {
            runtime.shutdown();
        }
    }

    // TC-67-14 · the failure is a message, never a crash and never a silence.
    // Reached through the real refusal path: with no active workspace there is
    // nothing for a tab to belong to, and this is reachable from a mouse click
    // on a bar that is drawn before any workspace exists.
    // TP-CHROME-69: a secondary gesture that cannot run says so and opens
    // nothing.
    #[test]
    fn a_secondary_gesture_that_cannot_run_says_so_and_opens_nothing() {
        let mut app = app_with_a_clickable_top_bar_section();
        app.state.shell_bar_chrome = {
            let mut section = crate::config::ShellBarSectionConfig {
                kind: "fixed".to_string(),
                cells: 10,
                ..Default::default()
            };
            section.action.kind = "popup".to_string();
            section.action.argv = vec!["btop".to_string()];
            section.action.secondary = "tab".to_string();
            crate::ui::shell::ShellBarChrome::from_config(&crate::config::ShellBarsConfig {
                top: crate::config::ShellBarConfig {
                    enabled: true,
                    size: 1,
                    border: false,
                    color: String::new(),
                    gradient: Vec::new(),
                    sections: vec![section],
                    ..Default::default()
                },
                ..Default::default()
            })
        };
        assert!(
            app.state.active.is_none(),
            "control: this fixture has no workspace for a tab to belong to"
        );

        let consumed = app.handle_bar_section_mouse(bar_mouse(
            MouseEventKind::Down(MouseButton::Right),
            4,
            0,
            KeyModifiers::NONE,
        ));

        assert!(consumed, "the press still belongs to the bar");
        let toast = app
            .state
            .toast
            .as_ref()
            .expect("a refusal the person cannot see is a silent failure");
        assert_eq!(toast.title, "bar section action failed");
        assert!(
            app.state.workspaces.is_empty(),
            "nothing may have been created on the way to failing"
        );
    }

    /// An app holding an open popup whose runtime writes into `rx`, with the
    /// view computed so the popup has real rectangles to be inside and outside
    /// of.
    fn app_with_open_popup() -> (
        App,
        tokio::sync::mpsc::Receiver<bytes::Bytes>,
        ratatui::layout::Rect,
    ) {
        let mut app = test_app();
        app.state.workspaces = vec![crate::workspace::Workspace::test_new("one")];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;
        crate::ui::compute_view(&mut app.state, ratatui::layout::Rect::new(0, 0, 106, 40));

        let (runtime, rx) = crate::terminal::TerminalRuntime::test_with_channel(40, 12);
        app.install_test_popup_runtime(runtime);
        let (outer, _inner) = crate::ui::popup_pane_rects(&app.state, app.state.view.terminal_area)
            .expect("the fixture popup must have rectangles");
        (app, rx, outer)
    }

    fn outside_of(outer: ratatui::layout::Rect) -> (u16, u16) {
        // Above and left of the popup, which the centred geometry always
        // leaves room for in a 106x40 view.
        (outer.x.saturating_sub(1), outer.y.saturating_sub(1))
    }

    // TC-A1 · THE GUARANTEE THIS LAYER MUST NOT EAT. A press outside the popup
    // delivers Esc into the program and does NOT close the pane. Editors bind
    // Esc to save-then-quit, and taking the pane away would race their final
    // write. Until now this behaviour had no test and no registry entry at
    // all, so an escape hatch built on top of it could have removed it without
    // anything going red.
    #[tokio::test]
    async fn a_first_press_outside_a_popup_asks_it_to_quit_rather_than_killing_it() {
        let (mut app, mut rx, outer) = app_with_open_popup();
        let (column, row) = outside_of(outer);

        app.handle_popup_mouse(bar_mouse(
            MouseEventKind::Down(MouseButton::Left),
            column,
            row,
            KeyModifiers::NONE,
        ));

        assert_eq!(
            rx.try_recv()
                .expect("the popup must receive something")
                .as_ref(),
            b"\x1b",
            "the dismissal is delivered as Esc into the program, not as a kill"
        );
        assert!(
            app.state.popup_pane.is_some(),
            "the pane must survive the first press so the program can save and quit itself"
        );
        assert!(
            app.state.toast.is_some(),
            "a two-step gesture nobody is told about is one nobody finds"
        );

        for (_, runtime) in app.terminal_runtimes.drain() {
            runtime.shutdown();
        }
    }

    // TC-A2/TC-A6 · a program that ignores Esc still has a way out, and the
    // request cannot be inherited by a later popup.
    #[tokio::test]
    async fn a_second_press_outside_closes_a_popup_that_ignored_the_first() {
        let (mut app, mut rx, outer) = app_with_open_popup();
        let (column, row) = outside_of(outer);
        let press = bar_mouse(
            MouseEventKind::Down(MouseButton::Left),
            column,
            row,
            KeyModifiers::NONE,
        );

        app.handle_popup_mouse(press);
        assert!(app.state.popup_pane.is_some(), "control: still open");
        let _ = rx.try_recv();

        app.handle_popup_mouse(press);
        assert!(
            app.state.popup_pane.is_none(),
            "a program that ignores Esc must not be able to hold the surface"
        );
        assert_eq!(
            app.popup_dismiss_requested, None,
            "the spent request must not survive the popup it was made of"
        );

        // TC-A6: a NEW popup starts over. The request is remembered against a
        // terminal id, so this is true by construction rather than by anyone
        // remembering to reset it — which is the point of storing it that way.
        let (runtime, _rx2) = crate::terminal::TerminalRuntime::test_with_channel(40, 12);
        app.install_test_popup_runtime(runtime);
        app.handle_popup_mouse(press);
        assert!(
            app.state.popup_pane.is_some(),
            "the first press on a new popup must ask, not close"
        );

        for (_, runtime) in app.terminal_runtimes.drain() {
            runtime.shutdown();
        }
    }

    // TC-A4 · moving over or scrolling past a popup is not a statement of
    // intent, so it must not advance the escalation. Otherwise crossing the
    // background with the pointer would arm a close nobody asked for.
    #[tokio::test]
    async fn moving_past_a_popup_does_not_arm_its_dismissal() {
        let (mut app, _rx, outer) = app_with_open_popup();
        let (column, row) = outside_of(outer);

        for kind in [
            MouseEventKind::Moved,
            MouseEventKind::ScrollDown,
            MouseEventKind::ScrollUp,
            MouseEventKind::Up(MouseButton::Left),
        ] {
            app.handle_popup_mouse(bar_mouse(kind, column, row, KeyModifiers::NONE));
        }

        assert_eq!(
            app.popup_dismiss_requested, None,
            "only a press states an intent to dismiss"
        );
        assert!(app.state.popup_pane.is_some());

        for (_, runtime) in app.terminal_runtimes.drain() {
            runtime.shutdown();
        }
    }

    // TC-A8 · going back into the popup cancels a dismissal that was asked for
    // and not answered. Esc does not always mean quit: an editor may have
    // opened an "unsaved changes?" prompt with it, and the press answering that
    // prompt lands inside. Without this, the next press outside would close the
    // pane on top of the very question the guarantee exists to protect.
    #[tokio::test]
    async fn returning_to_the_popup_cancels_a_dismissal_it_did_not_answer() {
        let (mut app, mut rx, outer) = app_with_open_popup();
        let (column, row) = outside_of(outer);
        let outside_press = bar_mouse(
            MouseEventKind::Down(MouseButton::Left),
            column,
            row,
            KeyModifiers::NONE,
        );

        app.handle_popup_mouse(outside_press);
        assert!(app.popup_dismiss_requested.is_some(), "control: armed");
        let _ = rx.try_recv();

        // A press inside the popup: the person went back to using it.
        let inside = bar_mouse(
            MouseEventKind::Down(MouseButton::Left),
            outer.x.saturating_add(outer.width / 2),
            outer.y.saturating_add(outer.height / 2),
            KeyModifiers::NONE,
        );
        app.handle_popup_mouse(inside);
        assert_eq!(
            app.popup_dismiss_requested, None,
            "using the popup again must cancel a dismissal it never answered"
        );

        app.handle_popup_mouse(outside_press);
        assert!(
            app.state.popup_pane.is_some(),
            "the next press outside must ask again rather than close"
        );

        for (_, runtime) in app.terminal_runtimes.drain() {
            runtime.shutdown();
        }
    }

    // TA-11 · a bar is chrome, so every event over one of its sections stops
    // there. An event that fell through would act on the surface behind the
    // bar, which is not the surface the person was pointing at (CL12).
    #[test]
    fn every_event_over_a_section_stops_at_the_bar() {
        let mut app = app_with_a_clickable_top_bar_section();
        let inside = (4, 0);

        for (name, kind, modifiers) in [
            (
                "right press",
                MouseEventKind::Down(MouseButton::Right),
                KeyModifiers::NONE,
            ),
            ("wheel", MouseEventKind::ScrollDown, KeyModifiers::NONE),
            (
                "modified left press",
                MouseEventKind::Down(MouseButton::Left),
                KeyModifiers::SHIFT,
            ),
            (
                "release",
                MouseEventKind::Up(MouseButton::Left),
                KeyModifiers::NONE,
            ),
        ] {
            assert!(
                app.handle_bar_section_mouse(bar_mouse(kind, inside.0, inside.1, modifiers)),
                "{name} over a section must be consumed by the bar"
            );
            assert!(
                app.state.popup_pane.is_none(),
                "{name} must not be mistaken for the section's action"
            );
        }

        // Away from the bar the handler claims nothing, so the surfaces
        // underneath keep every event they already owned.
        assert!(!app.handle_bar_section_mouse(bar_mouse(
            MouseEventKind::Down(MouseButton::Left),
            60,
            20,
            KeyModifiers::NONE,
        )));
        // Inside the bar but outside the section is the bar's own terrain, not
        // a section's: this handler must not claim it either.
        assert!(!app.handle_bar_section_mouse(bar_mouse(
            MouseEventKind::Down(MouseButton::Left),
            60,
            0,
            KeyModifiers::NONE,
        )));
    }

    // TA-6 · the action's failure is a message, never a crash and never a
    // silence. Reached through the real refusal path: with no active workspace
    // the popup machinery declines, which is the same road a terminal area too
    // small for a popup takes.
    #[test]
    fn a_section_action_that_cannot_run_says_so_and_opens_nothing() {
        let mut app = app_with_a_clickable_top_bar_section();
        assert!(
            app.state.active.is_none(),
            "control: this fixture has no workspace for a popup to belong to"
        );

        let consumed = app.handle_bar_section_mouse(bar_mouse(
            MouseEventKind::Down(MouseButton::Left),
            4,
            0,
            KeyModifiers::NONE,
        ));

        assert!(consumed, "a failed action still belongs to the bar");
        assert!(
            app.state.popup_pane.is_none(),
            "a refused spawn must leave no popup behind"
        );
        let toast = app.state.toast.as_ref().expect("the refusal must be said");
        assert_eq!(
            toast.kind,
            crate::app::state::ToastKind::NeedsAttention,
            "a silent failure reads as a bar that does nothing"
        );
    }

    // TA-3 at the wiring layer: the pure resolution refuses, and the wiring
    // must neither reach the spawn nor disturb the popup already on screen.
    // Async only because the popup fixture spawns a detection task; the
    // behaviour under test is synchronous.
    #[tokio::test]
    async fn a_second_popup_is_refused_without_touching_the_first() {
        let mut app = app_with_a_clickable_top_bar_section();
        let (runtime, _rx) = crate::terminal::TerminalRuntime::test_with_channel(40, 12);
        let (_, terminal_id) = app.install_test_popup_runtime(runtime);

        let consumed = app.handle_bar_section_mouse(bar_mouse(
            MouseEventKind::Down(MouseButton::Left),
            4,
            0,
            KeyModifiers::NONE,
        ));

        assert!(consumed);
        assert_eq!(
            app.state
                .popup_pane
                .as_ref()
                .map(|popup| &popup.terminal_id),
            Some(&terminal_id),
            "the popup already open must survive a bar click untouched"
        );
        assert!(app.state.toast.is_some(), "the refusal is worth saying");
    }

    #[tokio::test]
    async fn paste_routes_to_rename_modal_input() {
        let mut app = test_app();
        app.state.workspaces = vec![crate::workspace::Workspace::test_new("test")];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::RenameTab;
        app.state.name_input = "2".into();
        app.state.name_input_replace_on_type = true;

        app.handle_paste("feature/logs".into()).await;

        assert_eq!(app.state.name_input, "feature/logs");
        assert!(!app.state.name_input_replace_on_type);
    }

    #[tokio::test]
    async fn paste_routes_to_keybind_help_query_only_when_searching() {
        let mut app = test_app();
        app.state.mode = Mode::KeybindHelp;
        app.handle_paste("ignored".into()).await;
        assert!(app.state.keybind_help.query.is_empty());

        app.state.keybind_help.search_focused = true;
        app.state.keybind_help.scroll = 3;
        app.handle_paste("work\nspace".into()).await;

        assert_eq!(app.state.keybind_help.query, "workspace");
        assert_eq!(app.state.keybind_help.scroll, 0);
    }

    #[tokio::test]
    async fn paste_routes_to_new_linked_worktree_input() {
        let mut app = test_app();
        app.state.mode = Mode::NewLinkedWorktree;
        app.state.name_input = "generated-branch".into();
        app.state.name_input_replace_on_type = true;
        app.state.worktree_create = Some(crate::app::state::WorktreeCreateState {
            source_workspace_id: "source".into(),
            source_checkout_path: "/repo/herdr".into(),
            source_existing_membership: None,
            source_repo_root: "/repo/herdr".into(),
            repo_key: "repo-key".into(),
            repo_name: "herdr".into(),
            branch: "generated-branch".into(),
            checkout_path: "/repo/herdr-generated-branch".into(),
            error: None,
            creating: false,
        });

        app.handle_paste("feature/linear-302".into()).await;

        assert_eq!(app.state.name_input, "feature/linear-302");
        assert_eq!(
            app.state
                .worktree_create
                .as_ref()
                .map(|create| create.branch.as_str()),
            Some("feature/linear-302")
        );
    }

    #[tokio::test]
    async fn files_surface_blocks_paste_from_hidden_terminal() {
        let root = unique_temp_path("files-block-hidden-paste");
        std::fs::create_dir_all(&root).unwrap();

        let mut app = test_app();
        let mut workspace = crate::workspace::Workspace::test_new("test");
        let focused = workspace.focused_pane_id().unwrap();
        let (runtime, mut rx) = crate::terminal::TerminalRuntime::test_with_channel(80, 24);
        workspace.insert_test_runtime(focused, runtime);
        app.state.workspaces = vec![workspace];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;
        app.state
            .try_open_file_manager_with(|_| Some(crate::fm::FmState::new(&root)))
            .unwrap();

        app.handle_paste("must-not-reach-hidden-pty".into()).await;

        assert!(app.state.file_manager.is_some());
        assert!(
            rx.try_recv().is_err(),
            "Files-focused paste must not reach the hidden PTY"
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn visible_terminal_surface_still_receives_paste() {
        let mut app = test_app();
        let mut workspace = crate::workspace::Workspace::test_new("test");
        let focused = workspace.focused_pane_id().unwrap();
        let (runtime, mut rx) = crate::terminal::TerminalRuntime::test_with_channel(80, 24);
        workspace.insert_test_runtime(focused, runtime);
        app.state.workspaces = vec![workspace];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;

        app.handle_paste("visible-terminal-paste".into()).await;

        assert_eq!(
            rx.recv().await.unwrap(),
            bytes::Bytes::from_static(b"visible-terminal-paste")
        );
    }

    #[test]
    fn modal_paste_shortcut_matches_platform_primary_v() {
        #[cfg(target_os = "macos")]
        let modifiers = KeyModifiers::SUPER;
        #[cfg(not(target_os = "macos"))]
        let modifiers = KeyModifiers::CONTROL;

        assert!(is_modal_paste_shortcut(&KeyEvent::new(
            KeyCode::Char('v'),
            modifiers
        )));
        assert!(is_modal_paste_shortcut(&KeyEvent::new(
            KeyCode::Char('V'),
            modifiers | KeyModifiers::SHIFT
        )));
        assert!(!is_modal_paste_shortcut(&KeyEvent::new(
            KeyCode::Char('v'),
            KeyModifiers::ALT
        )));
    }

    #[test]
    fn modal_paste_target_is_active_only_for_text_inputs() {
        let mut state = AppState::test_new();

        state.mode = Mode::RenameTab;
        assert!(modal_paste_target_active(&state));

        state.mode = Mode::Navigator;
        state.navigator.search_focused = false;
        assert!(!modal_paste_target_active(&state));
        state.navigator.search_focused = true;
        assert!(modal_paste_target_active(&state));

        state.mode = Mode::KeybindHelp;
        state.keybind_help.search_focused = false;
        assert!(!modal_paste_target_active(&state));
        state.keybind_help.search_focused = true;
        assert!(modal_paste_target_active(&state));

        state.mode = Mode::ConfirmClose;
        assert!(!modal_paste_target_active(&state));
    }
}
