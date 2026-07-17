//! Native file manager — navigation and tagged action input (A3/C1/C2).
//!
//! While the file manager is open it captures keyboard input (intercepted in
//! `handle_key` before the mode dispatch), driving the cursor and directory
//! navigation on `AppState.file_manager`. Client-side presentation input; keys
//! that it does not use are swallowed so they never reach the hidden terminal.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

use crate::app::state::{
    AppState, ContextMenuKind, ContextMenuState, FileManagerContextMenuModel,
    FileManagerHeaderAction, FileManagerRowAction, MenuListState,
};
use crate::app::{App, FileManagerClickState, Mode};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum FileManagerMouseDispatch {
    NotHandled,
    Consumed,
    HeaderAction(FileManagerHeaderAction),
    RowAction {
        action: FileManagerRowAction,
        entry_path: std::path::PathBuf,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum FileManagerKeyDispatch {
    Consumed,
    CancelOperation,
    Navigate(crate::fm::FmNavigationRequest),
    Refresh(crate::fm::FmCurrentRefreshRequest),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AttachmentPickerKeyDispatch {
    Consumed,
    Navigate(crate::fm::FmNavigationRequest),
    Refresh(crate::fm::FmCurrentRefreshRequest),
}

/// Handle one key while the file manager is open. `Esc` requests cancellation
/// for a running file operation and otherwise closes the file manager; `q`
/// always closes it. The arrow keys and `hjkl` move the cursor or navigate
/// directories; `.` toggles hidden files; Ctrl+A selects all; Ctrl+Shift+A
/// clears the explicit selection. Any other key is a no-op (swallowed).
pub(super) fn handle_file_manager_key(
    state: &mut AppState,
    key: KeyEvent,
) -> FileManagerKeyDispatch {
    if key.code == KeyCode::Esc
        && state
            .file_manager_operation
            .as_ref()
            .is_some_and(crate::app::state::FileManagerOperationState::is_running)
    {
        return FileManagerKeyDispatch::CancelOperation;
    }
    match (key.code, key.modifiers) {
        (KeyCode::Esc | KeyCode::Char('q'), _) => {
            state.close_file_manager();
        }
        (KeyCode::Char('a'), KeyModifiers::CONTROL) => {
            if let Some(fm) = state.file_manager.as_mut() {
                fm.select_all();
            }
        }
        (KeyCode::Char('a') | KeyCode::Char('A'), modifiers)
            if modifiers == KeyModifiers::CONTROL | KeyModifiers::SHIFT =>
        {
            if let Some(fm) = state.file_manager.as_mut() {
                fm.clear_multi_selection();
            }
        }
        (KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J'), KeyModifiers::SHIFT) => {
            if let Some(fm) = state.file_manager.as_mut() {
                let target = fm
                    .cursor
                    .saturating_add(1)
                    .min(fm.entries.len().saturating_sub(1));
                fm.extend_selection(target);
            }
        }
        (KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K'), KeyModifiers::SHIFT) => {
            if let Some(fm) = state.file_manager.as_mut() {
                fm.extend_selection(fm.cursor.saturating_sub(1));
            }
        }
        (KeyCode::Char(' '), KeyModifiers::NONE) => {
            if let Some(fm) = state.file_manager.as_mut() {
                let cursor = fm.cursor;
                fm.toggle_selection(cursor);
            }
        }
        (KeyCode::Down | KeyCode::Char('j'), KeyModifiers::NONE) => {
            if let Some(fm) = state.file_manager.as_mut() {
                fm.move_down();
            }
        }
        (KeyCode::Up | KeyCode::Char('k'), KeyModifiers::NONE) => {
            if let Some(fm) = state.file_manager.as_mut() {
                fm.move_up();
            }
        }
        (KeyCode::Enter | KeyCode::Right | KeyCode::Char('l'), KeyModifiers::NONE) => {
            if let Some(request) = state
                .file_manager
                .as_ref()
                .and_then(crate::fm::FmState::request_enter_navigation)
            {
                return FileManagerKeyDispatch::Navigate(request);
            }
        }
        (KeyCode::Backspace | KeyCode::Left | KeyCode::Char('h'), KeyModifiers::NONE) => {
            if let Some(request) = state
                .file_manager
                .as_ref()
                .and_then(crate::fm::FmState::request_leave_navigation)
            {
                return FileManagerKeyDispatch::Navigate(request);
            }
        }
        (KeyCode::Char('.'), KeyModifiers::NONE) => {
            let files_generation = (state.stage.surface_view()
                == crate::ui::surface_host::StageSurfaceView::NativeFiles)
                .then(|| state.stage.active_instance_generation())
                .flatten();
            if let Some(request) = state.file_manager.as_ref().and_then(|file_manager| {
                files_generation.map(|generation| file_manager.request_hidden_toggle(generation))
            }) {
                return FileManagerKeyDispatch::Refresh(request);
            }
        }
        _ => {}
    }
    FileManagerKeyDispatch::Consumed
}

pub(crate) fn handle_agent_attachment_picker_key(
    state: &mut AppState,
    key: KeyEvent,
) -> AttachmentPickerKeyDispatch {
    match (key.code, key.modifiers) {
        (KeyCode::Esc | KeyCode::Char('q'), _) => {
            state.close_agent_attachment_picker();
            AttachmentPickerKeyDispatch::Consumed
        }
        (KeyCode::Down | KeyCode::Char('j'), KeyModifiers::NONE) => {
            if let Some(picker) = state.agent_attachment_picker.as_mut() {
                picker.file_manager.move_down();
            }
            AttachmentPickerKeyDispatch::Consumed
        }
        (KeyCode::Up | KeyCode::Char('k'), KeyModifiers::NONE) => {
            if let Some(picker) = state.agent_attachment_picker.as_mut() {
                picker.file_manager.move_up();
            }
            AttachmentPickerKeyDispatch::Consumed
        }
        (KeyCode::Enter, KeyModifiers::NONE) => {
            if let Some(path) = state.agent_attachment_selected_file() {
                if let Some(target) = state
                    .agent_attachment_picker
                    .as_ref()
                    .map(|picker| picker.target.clone())
                {
                    state.request_agent_attachment_delivery =
                        Some(crate::app::state::AgentAttachmentDeliveryRequest { path, target });
                }
                AttachmentPickerKeyDispatch::Consumed
            } else {
                state
                    .agent_attachment_picker
                    .as_ref()
                    .and_then(|picker| picker.file_manager.request_enter_navigation())
                    .map_or(
                        AttachmentPickerKeyDispatch::Consumed,
                        AttachmentPickerKeyDispatch::Navigate,
                    )
            }
        }
        (KeyCode::Right | KeyCode::Char('l'), KeyModifiers::NONE) => {
            if state.agent_attachment_selected_file().is_some() {
                AttachmentPickerKeyDispatch::Consumed
            } else {
                state
                    .agent_attachment_picker
                    .as_ref()
                    .and_then(|picker| picker.file_manager.request_enter_navigation())
                    .map_or(
                        AttachmentPickerKeyDispatch::Consumed,
                        AttachmentPickerKeyDispatch::Navigate,
                    )
            }
        }
        (KeyCode::Backspace | KeyCode::Left | KeyCode::Char('h'), KeyModifiers::NONE) => state
            .agent_attachment_picker
            .as_ref()
            .and_then(|picker| picker.file_manager.request_leave_navigation())
            .map_or(
                AttachmentPickerKeyDispatch::Consumed,
                AttachmentPickerKeyDispatch::Navigate,
            ),
        (KeyCode::Char('.'), KeyModifiers::NONE) => state
            .agent_attachment_picker
            .as_ref()
            .map(|picker| picker.file_manager.request_hidden_toggle(0))
            .map_or(
                AttachmentPickerKeyDispatch::Consumed,
                AttachmentPickerKeyDispatch::Refresh,
            ),
        _ => AttachmentPickerKeyDispatch::Consumed,
    }
}

impl App {
    pub(in crate::app) fn route_agent_attachment_picker_key(&mut self, key: KeyEvent) {
        match handle_agent_attachment_picker_key(&mut self.state, key) {
            AttachmentPickerKeyDispatch::Navigate(request) => {
                let Some(prepared) = crate::fm::prepare_navigation_io(request) else {
                    return;
                };
                let _ = self
                    .state
                    .agent_attachment_picker
                    .as_mut()
                    .is_some_and(|picker| picker.file_manager.apply_prepared_navigation(prepared));
            }
            AttachmentPickerKeyDispatch::Refresh(request) => {
                let prepared = crate::fm::prepare_current_refresh_io(request);
                let _ = self
                    .state
                    .agent_attachment_picker
                    .as_mut()
                    .is_some_and(|picker| {
                        picker
                            .file_manager
                            .apply_prepared_current_refresh(prepared, 0)
                    });
            }
            AttachmentPickerKeyDispatch::Consumed => {}
        }
    }

    /// Convert one stable row hit target into the same typed intent consumed by
    /// header/context actions. A cloned projection proves the anchored row is
    /// currently eligible before the real selection changes, so rejected
    /// stale, bulk, read-only, or in-flight actions cannot corrupt focus.
    pub(super) fn dispatch_file_manager_row_action(
        &mut self,
        action: FileManagerRowAction,
        entry_path: std::path::PathBuf,
    ) -> bool {
        let context_action = match action {
            FileManagerRowAction::SendAgent => {
                crate::app::state::FileManagerContextMenuAction::SendAgent
            }
            FileManagerRowAction::Rename => crate::app::state::FileManagerContextMenuAction::Rename,
            FileManagerRowAction::Delete => crate::app::state::FileManagerContextMenuAction::Delete,
        };
        let Some((entry_idx, mut projected)) = self.state.file_manager.as_ref().and_then(|fm| {
            (fm.multi_selection_paths().len() <= 1).then_some(())?;
            let entry_idx = fm
                .entries
                .iter()
                .position(|entry| entry.operation_supported && entry.path == entry_path)?;
            Some((entry_idx, fm.clone()))
        }) else {
            return false;
        };
        if !projected.replace_selection(entry_idx) {
            return false;
        }
        let action_bar = crate::ui::compute_file_manager_action_bar_model(
            &projected,
            &self.state.file_manager_clipboard,
            self.state
                .file_manager_operation
                .as_ref()
                .is_some_and(crate::app::state::FileManagerOperationState::is_running),
        );
        let Some(model) =
            FileManagerContextMenuModel::from_action_bar_with_plugins(&action_bar, &[])
        else {
            return false;
        };
        if model.paths != [entry_path.clone()]
            || !model
                .items
                .iter()
                .any(|item| item.action == context_action && item.enabled)
        {
            return false;
        }
        if !self
            .state
            .file_manager
            .as_mut()
            .is_some_and(|fm| fm.replace_selection(entry_idx))
        {
            return false;
        }
        self.state.request_file_manager_context_action =
            Some(crate::app::state::FileManagerContextActionIntent {
                action: context_action,
                paths: vec![entry_path],
            });
        true
    }

    /// Route native-FM center-content mouse input before the hidden terminal
    /// pane path. Row actions carry stable path identity but remain side-effect
    /// free until their operation modules provide explicit execution authority.
    pub(super) fn handle_file_manager_mouse(
        &mut self,
        mouse: MouseEvent,
    ) -> FileManagerMouseDispatch {
        // The TYPED stage authority owns Files mouse routing: a hidden Files
        // surface (or a divergent legacy boolean) receives nothing.
        if self.state.stage.surface_view() != crate::ui::surface_host::StageSurfaceView::NativeFiles
            || self.state.file_manager.is_none()
        {
            self.last_file_manager_click = None;
            return FileManagerMouseDispatch::NotHandled;
        }
        if self.state.mode == Mode::ContextMenu {
            return FileManagerMouseDispatch::NotHandled;
        }

        if self.state.shell_interaction.miller_resize_active() {
            match mouse.kind {
                MouseEventKind::Drag(MouseButton::Left) => {
                    if let Some(bounds) = crate::ui::shell::ResizeBounds::new(
                        crate::fm::miller::MILLER_COLUMN_MIN_WIDTH,
                        crate::fm::miller::MILLER_COLUMN_MAX_WIDTH,
                        crate::fm::miller::MILLER_COLUMN_MIN_WIDTH,
                        crate::fm::miller::MILLER_COLUMN_MAX_WIDTH,
                    ) {
                        let _ = self.state.shell_interaction.preview_resize(
                            ratatui::layout::Position::new(mouse.column, mouse.row),
                            bounds,
                        );
                    }
                    return FileManagerMouseDispatch::Consumed;
                }
                MouseEventKind::Up(MouseButton::Left) => {
                    let _ = self.commit_miller_resize();
                    return FileManagerMouseDispatch::Consumed;
                }
                _ => {}
            }
        }

        // FM2.2: an ACTIVE divider capture owns move/up everywhere — even
        // outside the Files area — per the SF4.2-04 capture principle, so a
        // fast drag can never escape the gesture.
        if let Some(drag) = self.state.miller_trio_drag {
            match mouse.kind {
                MouseEventKind::Drag(MouseButton::Left) => {
                    let delta = i32::from(mouse.column) - i32::from(drag.origin_x);
                    let desired =
                        (i32::from(drag.original_width) + delta).clamp(0, i32::from(u16::MAX));
                    if let Some(file_manager) = self.state.file_manager.as_mut() {
                        let _ = file_manager.commit_trio_width(drag.slot, desired as u16);
                    }
                    return FileManagerMouseDispatch::Consumed;
                }
                MouseEventKind::Up(MouseButton::Left) => {
                    self.state.miller_trio_drag = None;
                    return FileManagerMouseDispatch::Consumed;
                }
                _ => {}
            }
        }

        let center = self.state.view.terminal_area;
        let in_center = rect_contains(center, mouse.column, mouse.row);
        if !in_center {
            if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
                self.last_file_manager_click = None;
            }
            return FileManagerMouseDispatch::NotHandled;
        }

        if self.handle_miller_horizontal_scroll(mouse.kind, mouse.modifiers) {
            return FileManagerMouseDispatch::Consumed;
        }

        if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left))
            && mouse.modifiers.is_empty()
            && self.begin_miller_resize_capture(mouse.column, mouse.row)
        {
            return FileManagerMouseDispatch::Consumed;
        }

        // FM2.2: trio divider drag-resize. The one-cell divider strips from
        // the CURRENT layout (same pure geometry authority as render) own
        // press/drag/release; commits clamp through `commit_trio_width`.
        let overrides = self
            .state
            .file_manager
            .as_ref()
            .map(|file_manager| file_manager.trio_overrides)
            .unwrap_or_default();
        if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left))
            && mouse.modifiers.is_empty()
        {
            let dividers = crate::ui::file_manager_divider_areas(center, overrides);
            let areas = [
                crate::ui::file_manager_column_widths(center, overrides)[0],
                crate::ui::file_manager_column_widths(center, overrides)[1],
            ];
            for (slot, divider) in dividers.into_iter().enumerate() {
                let Some(divider) = divider else { continue };
                if mouse.column >= divider.x
                    && mouse.column < divider.x.saturating_add(divider.width)
                    && mouse.row >= divider.y
                    && mouse.row < divider.y.saturating_add(divider.height)
                {
                    if let Some(original_width) = areas[slot] {
                        self.state.miller_trio_drag = Some(crate::app::state::MillerTrioDrag {
                            slot,
                            origin_x: mouse.column,
                            original_width,
                        });
                    }
                    return FileManagerMouseDispatch::Consumed;
                }
            }
        }

        let header_action = matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left))
            .then_some(())
            .filter(|_| mouse.modifiers.is_empty())
            .and_then(|()| {
                self.state
                    .view
                    .file_manager_header_action_areas
                    .iter()
                    .find(|area| rect_contains(area.rect, mouse.column, mouse.row))
                    .map(|area| area.action)
            });

        let miller_row_target = self.resolve_miller_mouse_row(mouse.column, mouse.row);
        let entry_target = miller_row_target
            .as_ref()
            .and_then(|row| row.current_entry_target());
        let entry_idx = entry_target.as_ref().map(|(entry_idx, _)| *entry_idx);

        let mut context_entry_target =
            matches!(mouse.kind, MouseEventKind::Down(MouseButton::Right))
                .then(|| {
                    entry_target.clone().or_else(|| {
                        self.state
                            .view
                            .file_manager_row_action_areas
                            .iter()
                            .find(|area| rect_contains(area.rect, mouse.column, mouse.row))
                            .and_then(|area| {
                                let entry =
                                    self.state.file_manager.as_ref().and_then(|file_manager| {
                                        file_manager.entries.get(area.entry_idx)
                                    })?;
                                (entry.path == area.entry_path)
                                    .then(|| (area.entry_idx, area.entry_path.clone()))
                            })
                    })
                })
                .flatten();
        if context_entry_target.is_none()
            && matches!(mouse.kind, MouseEventKind::Down(MouseButton::Right))
            && mouse.modifiers.is_empty()
        {
            if let Some(row) = miller_row_target.as_ref() {
                context_entry_target = self.activate_miller_context_row(row);
            }
        }

        let row_action = matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left))
            .then_some(())
            .filter(|_| mouse.modifiers.is_empty())
            .and_then(|()| {
                self.state
                    .view
                    .file_manager_row_action_areas
                    .iter()
                    .find(|area| rect_contains(area.rect, mouse.column, mouse.row))
            })
            .and_then(|area| {
                let entry = self
                    .state
                    .file_manager
                    .as_ref()
                    .and_then(|file_manager| file_manager.entries.get(area.entry_idx))?;
                (entry.operation_supported && entry.path == area.entry_path).then(|| {
                    FileManagerMouseDispatch::RowAction {
                        action: area.action,
                        entry_path: area.entry_path.clone(),
                    }
                })
            });

        if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left))
            && mouse.modifiers.is_empty()
        {
            if let Some(header_action) = header_action {
                self.last_file_manager_click = None;
                let enabled = self
                    .state
                    .view
                    .file_manager_action_bar
                    .as_ref()
                    .and_then(|model| model.action_state(header_action))
                    .is_some_and(|state| state.enabled);
                return if enabled {
                    FileManagerMouseDispatch::HeaderAction(header_action)
                } else {
                    FileManagerMouseDispatch::Consumed
                };
            }
            if let Some(row_action) = row_action {
                self.last_file_manager_click = None;
                return row_action;
            }
        }

        if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Right)) {
            self.last_file_manager_click = None;
            if mouse.modifiers.is_empty() {
                if let Some((entry_idx, entry_path)) = context_entry_target {
                    let selection_ready =
                        self.state
                            .file_manager
                            .as_mut()
                            .is_some_and(|file_manager| {
                                if file_manager.multi_selection_paths().contains(&entry_path) {
                                    file_manager.select(entry_idx)
                                } else {
                                    file_manager.replace_selection(entry_idx)
                                }
                            });
                    if selection_ready {
                        let action_bar = self.state.file_manager.as_ref().map(|file_manager| {
                            crate::ui::compute_file_manager_action_bar_model(
                                file_manager,
                                &self.state.file_manager_clipboard,
                                self.state
                                    .file_manager_operation
                                    .as_ref()
                                    .is_some_and(|operation| operation.is_running()),
                            )
                        });
                        let plugin_actions = crate::app::api::plugins::file_manifest_actions(
                            &self.state.installed_plugins,
                        );
                        if let Some((action_bar, model)) = action_bar.and_then(|action_bar| {
                            FileManagerContextMenuModel::from_action_bar_with_plugins(
                                &action_bar,
                                &plugin_actions,
                            )
                            .map(|model| (action_bar, model))
                        }) {
                            self.state.view.file_manager_action_bar = Some(action_bar);
                            self.state.context_menu = Some(ContextMenuState {
                                kind: ContextMenuKind::File { model },
                                x: mouse.column,
                                y: mouse.row,
                                list: MenuListState::new(0),
                            });
                            self.state.enter_overlay_mode(Mode::ContextMenu);
                        }
                    }
                }
            }
            return FileManagerMouseDispatch::Consumed;
        }

        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) if mouse.modifiers == KeyModifiers::CONTROL => {
                self.last_file_manager_click = None;
                if let (Some(file_manager), Some(entry_idx)) =
                    (self.state.file_manager.as_mut(), entry_idx)
                {
                    file_manager.toggle_selection(entry_idx);
                }
            }
            MouseEventKind::Down(MouseButton::Left) if mouse.modifiers == KeyModifiers::SHIFT => {
                self.last_file_manager_click = None;
                if let (Some(file_manager), Some(entry_idx)) =
                    (self.state.file_manager.as_mut(), entry_idx)
                {
                    file_manager.extend_selection(entry_idx);
                }
            }
            MouseEventKind::Down(MouseButton::Left) if mouse.modifiers.is_empty() => {
                if miller_row_target
                    .as_ref()
                    .is_some_and(|row| self.handle_miller_non_current_plain_click(row))
                {
                    return FileManagerMouseDispatch::Consumed;
                }
                let Some((entry_idx, entry_path)) = entry_target else {
                    self.last_file_manager_click = None;
                    return FileManagerMouseDispatch::Consumed;
                };

                let click = FileManagerClickState {
                    entry_path,
                    at: std::time::Instant::now(),
                };
                let is_double_click = self
                    .last_file_manager_click
                    .as_ref()
                    .is_some_and(|last| last.is_double_click_for(&click));
                self.last_file_manager_click = if is_double_click { None } else { Some(click) };

                let enter_request = self.state.file_manager.as_mut().and_then(|file_manager| {
                    (file_manager.replace_selection(entry_idx) && is_double_click)
                        .then(|| file_manager.request_enter_navigation())
                        .flatten()
                });
                if let Some(request) = enter_request {
                    let _ = self.execute_file_manager_navigation(request);
                }
            }
            MouseEventKind::ScrollUp if mouse.modifiers.is_empty() => {
                self.last_file_manager_click = None;
                if entry_idx.is_some() {
                    if let Some(file_manager) = self.state.file_manager.as_mut() {
                        file_manager.move_up();
                    }
                } else if let Some(row) = miller_row_target.as_ref() {
                    let _ = self.scroll_miller_non_current_row(row, -1);
                }
            }
            MouseEventKind::ScrollDown if mouse.modifiers.is_empty() => {
                self.last_file_manager_click = None;
                if entry_idx.is_some() {
                    if let Some(file_manager) = self.state.file_manager.as_mut() {
                        file_manager.move_down();
                    }
                } else if let Some(row) = miller_row_target.as_ref() {
                    let _ = self.scroll_miller_non_current_row(row, 1);
                }
            }
            MouseEventKind::Down(MouseButton::Left) => {
                self.last_file_manager_click = None;
            }
            _ => {}
        }

        FileManagerMouseDispatch::Consumed
    }
}

fn rect_contains(rect: ratatui::layout::Rect, column: u16, row: u16) -> bool {
    column >= rect.x
        && column < rect.x.saturating_add(rect.width)
        && row >= rect.y
        && row < rect.y.saturating_add(rect.height)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::{
        ContextMenuKind, FileManagerActionBarModel, FileManagerActionDisabledReason,
        FileManagerActionState, FileManagerAgentHandoffRequest, FileManagerContextMenuAction,
        FileManagerContextMenuTargetKind, FileManagerHeaderAction, FileManagerHeaderActionArea,
        FileManagerRowAction, FileManagerRowActionArea, FileManagerRowArea,
    };
    use crate::app::Mode;
    use crate::fm::{FmState, MAX_MULTI_SELECTION_PATHS};
    use crate::ui::compute_view;
    use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
    use ratatui::layout::Rect;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn unique() -> u64 {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        COUNTER.fetch_add(1, Ordering::Relaxed)
    }

    struct TempDir {
        root: PathBuf,
    }

    impl TempDir {
        fn new(tag: &str) -> Self {
            let root = std::env::temp_dir().join(format!(
                "herdr-fminput-{}-{}-{}",
                std::process::id(),
                tag,
                unique()
            ));
            fs::create_dir_all(&root).expect("create temp root");
            Self { root }
        }
        fn file(&self, name: &str) {
            fs::write(self.root.join(name), b"x").expect("write temp file");
        }
        fn dir(&self, name: &str) {
            fs::create_dir_all(self.root.join(name)).expect("create temp dir");
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn key_with_modifiers(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, modifiers)
    }

    fn state_with_attachment_picker(root: &std::path::Path, label: &str) -> AppState {
        let mut state = AppState::test_new();
        let mut workspace = crate::workspace::Workspace::test_new(label);
        workspace.identity_cwd = root.to_path_buf();
        let pane_id = workspace.tabs[0].root_pane;
        let terminal_id = workspace.tabs[0].panes[&pane_id]
            .attached_terminal_id
            .clone();
        state.workspaces = vec![workspace];
        state.active = Some(0);
        state.mode = Mode::Terminal;
        state.ensure_test_terminals();
        state
            .terminals
            .get_mut(&terminal_id)
            .expect("attachment target terminal")
            .set_agent_name("codex".into());
        state.view.terminal_area = Rect::new(0, 0, 80, 24);
        state
            .open_agent_attachment_picker()
            .expect("open attachment picker");
        state
    }

    fn apply_test_navigation(state: &mut AppState, dispatch: FileManagerKeyDispatch) {
        if let FileManagerKeyDispatch::Navigate(request) = dispatch {
            let prepared =
                crate::fm::prepare_navigation_io(request).expect("test navigation preparation");
            assert!(
                state
                    .file_manager
                    .as_mut()
                    .is_some_and(|file_manager| file_manager.apply_prepared_navigation(prepared)),
                "test App adapter must apply the live prepared result"
            );
        }
    }

    // TP-FM4-APP-ADAPTER: attachment-picker input emits navigation intent but
    // performs no filesystem-backed model transition before the App adapter.
    #[test]
    fn attachment_picker_directory_enter_is_pure_until_app_adapter() {
        let td = TempDir::new("attachment-directory-intent");
        td.dir("child");
        let child = td.root.join("child");
        let mut state = state_with_attachment_picker(&td.root, "attachment-directory-intent");
        let picker = state
            .agent_attachment_picker
            .as_mut()
            .expect("open attachment picker");
        let entry_idx = picker
            .file_manager
            .entries
            .iter()
            .position(|entry| entry.path == child)
            .expect("child directory");
        picker.file_manager.select(entry_idx);
        let before = picker.file_manager.clone();

        let dispatch = handle_agent_attachment_picker_key(&mut state, key(KeyCode::Enter));
        let AttachmentPickerKeyDispatch::Navigate(request) = dispatch else {
            panic!("directory Enter must emit one typed navigation request");
        };

        let after = &state
            .agent_attachment_picker
            .as_ref()
            .expect("picker remains open")
            .file_manager;
        assert_eq!(request.reason, crate::fm::FmNavigationReason::Enter);
        assert_eq!(request.source_directory, before.cwd);
        assert_eq!(
            request.source_directory_generation,
            before.directory_generation
        );
        assert_eq!(request.source_preview_generation, before.preview_generation);
        assert_eq!(request.source_miller_revision, before.miller.revision);
        assert_eq!(request.target_directory, child);
        assert_eq!(request.focus_path, None);
        assert_eq!(request.fallback_cursor, 0);
        assert_eq!(request.show_hidden, before.show_hidden);
        assert_eq!(after.cwd, before.cwd);
        assert_eq!(after.directory_generation, before.directory_generation);
        assert_eq!(after.preview_generation, before.preview_generation);
        assert_eq!(after.miller, before.miller);
        assert!(state.request_agent_attachment_delivery.is_none());
    }

    // P5 RED: the blocking attachment overlay owns the key but emits the same
    // disk-free hidden-refresh intent instead of mutating its model directly.
    #[test]
    fn attachment_dot_emits_hidden_refresh_request_without_mutation() {
        let td = TempDir::new("attachment-hidden-intent");
        td.file("shown.txt");
        td.file(".hidden.txt");
        let mut state = state_with_attachment_picker(&td.root, "attachment-hidden-intent");
        let before = state
            .agent_attachment_picker
            .as_ref()
            .expect("open attachment picker")
            .file_manager
            .clone();

        let dispatch = handle_agent_attachment_picker_key(&mut state, key(KeyCode::Char('.')));

        let AttachmentPickerKeyDispatch::Refresh(request) = dispatch else {
            panic!("attachment dot must emit one typed hidden-refresh request");
        };
        assert_eq!(
            request.reason,
            crate::fm::FmCurrentRefreshReason::ToggleHidden
        );
        assert_eq!(request.files_generation, 0);
        assert_eq!(request.source_directory, td.root);
        assert_eq!(request.source_show_hidden, before.show_hidden);
        assert_eq!(request.target_show_hidden, !before.show_hidden);
        let after = &state
            .agent_attachment_picker
            .as_ref()
            .expect("attachment picker remains open")
            .file_manager;
        assert_eq!(after.entries, before.entries);
        assert_eq!(after.show_hidden, before.show_hidden);
        assert_eq!(after.directory_generation, before.directory_generation);
        assert_eq!(after.preview_generation, before.preview_generation);
        assert!(state.request_agent_attachment_delivery.is_none());
    }

    #[test]
    fn attachment_app_route_applies_hidden_refresh_once() {
        let td = TempDir::new("attachment-hidden-app");
        td.file("shown.txt");
        td.file(".hidden.txt");
        let mut app = super::super::app_for_mouse_test();
        app.state = state_with_attachment_picker(&td.root, "attachment-hidden-app");
        let before = app
            .state
            .agent_attachment_picker
            .as_ref()
            .expect("open attachment picker")
            .file_manager
            .clone();

        app.route_agent_attachment_picker_key(key(KeyCode::Char('.')));

        let after = &app
            .state
            .agent_attachment_picker
            .as_ref()
            .expect("attachment picker remains open")
            .file_manager;
        assert!(after.show_hidden);
        assert_eq!(after.entries.len(), 2);
        assert_eq!(after.directory_generation, before.directory_generation + 1);
        assert_eq!(after.preview_generation, before.preview_generation + 1);
        assert!(app.state.request_agent_attachment_delivery.is_none());
    }

    // TP-FM4-APP-ADAPTER: parent navigation is also an intent-only input
    // transition; stale preparation must be rejectable before model mutation.
    #[test]
    fn attachment_picker_leave_is_pure_until_app_adapter() {
        let td = TempDir::new("attachment-leave-intent");
        td.dir("child");
        let child = td.root.join("child");
        let mut state = state_with_attachment_picker(&child, "attachment-leave-intent");
        let before = state
            .agent_attachment_picker
            .as_ref()
            .expect("open attachment picker")
            .file_manager
            .clone();

        let dispatch = handle_agent_attachment_picker_key(&mut state, key(KeyCode::Backspace));
        let AttachmentPickerKeyDispatch::Navigate(request) = dispatch else {
            panic!("Backspace must emit one typed parent navigation request");
        };

        let after = &state
            .agent_attachment_picker
            .as_ref()
            .expect("picker remains open")
            .file_manager;
        assert_eq!(request.reason, crate::fm::FmNavigationReason::Leave);
        assert_eq!(request.source_directory, child);
        assert_eq!(
            request.source_directory_generation,
            before.directory_generation
        );
        assert_eq!(request.source_preview_generation, before.preview_generation);
        assert_eq!(request.source_miller_revision, before.miller.revision);
        assert_eq!(request.target_directory, td.root);
        assert_eq!(request.focus_path, Some(before.cwd.clone()));
        assert_eq!(request.fallback_cursor, 0);
        assert_eq!(request.show_hidden, before.show_hidden);
        assert_eq!(after.cwd, before.cwd);
        assert_eq!(after.directory_generation, before.directory_generation);
        assert_eq!(after.preview_generation, before.preview_generation);
        assert_eq!(after.miller, before.miller);
        assert!(state.request_agent_attachment_delivery.is_none());
    }

    // Characterization: the complete App route still enters the selected
    // directory after the input-only handler is separated from filesystem I/O.
    #[test]
    fn attachment_picker_app_route_enters_directory_once() {
        let td = TempDir::new("attachment-app-navigation");
        td.dir("child");
        let child = td.root.join("child");
        let mut app = super::super::app_for_mouse_test();
        app.state = state_with_attachment_picker(&td.root, "attachment-app-navigation");
        let picker = app
            .state
            .agent_attachment_picker
            .as_mut()
            .expect("open attachment picker");
        let entry_idx = picker
            .file_manager
            .entries
            .iter()
            .position(|entry| entry.path == child)
            .expect("child directory");
        picker.file_manager.select(entry_idx);
        let before_generation = picker.file_manager.directory_generation;

        app.handle_non_terminal_key_headless(crate::input::TerminalKey::new(
            KeyCode::Enter,
            KeyModifiers::NONE,
        ));

        let after = &app
            .state
            .agent_attachment_picker
            .as_ref()
            .expect("picker remains open")
            .file_manager;
        assert_eq!(after.cwd, child);
        assert_eq!(
            after.directory_generation,
            before_generation.wrapping_add(1).max(1)
        );
        assert!(app.state.request_agent_attachment_delivery.is_none());
    }

    // Characterization: the async production key path must use the same App
    // adapter as the headless path and apply the navigation exactly once.
    #[tokio::test]
    async fn attachment_picker_async_app_route_enters_directory_once() {
        let td = TempDir::new("attachment-async-app-navigation");
        td.dir("child");
        let child = td.root.join("child");
        let mut app = super::super::app_for_mouse_test();
        app.state = state_with_attachment_picker(&td.root, "attachment-async-app-navigation");
        let picker = app
            .state
            .agent_attachment_picker
            .as_mut()
            .expect("open attachment picker");
        let entry_idx = picker
            .file_manager
            .entries
            .iter()
            .position(|entry| entry.path == child)
            .expect("child directory");
        picker.file_manager.select(entry_idx);
        let before_generation = picker.file_manager.directory_generation;

        app.handle_key(crate::input::TerminalKey::new(
            KeyCode::Enter,
            KeyModifiers::NONE,
        ))
        .await;

        let after = &app
            .state
            .agent_attachment_picker
            .as_ref()
            .expect("picker remains open")
            .file_manager;
        assert_eq!(after.cwd, child);
        assert_eq!(
            after.directory_generation,
            before_generation.wrapping_add(1).max(1)
        );
        assert!(app.state.request_agent_attachment_delivery.is_none());
    }

    #[test]
    fn attachment_picker_swallowing_unknown_key_preserves_background_terminal() {
        let td = TempDir::new("attachment-key-block");
        td.file("a.txt");
        let mut state = AppState::test_new();
        let mut workspace = crate::workspace::Workspace::test_new("attachment-key-block");
        workspace.identity_cwd = td.root.clone();
        let pane_id = workspace.tabs[0].root_pane;
        let terminal_id = workspace.tabs[0].panes[&pane_id]
            .attached_terminal_id
            .clone();
        state.workspaces = vec![workspace];
        state.active = Some(0);
        state.mode = Mode::Terminal;
        state.ensure_test_terminals();
        state
            .terminals
            .get_mut(&terminal_id)
            .unwrap()
            .set_agent_name("codex".into());
        state.view.terminal_area = Rect::new(0, 0, 80, 24);
        state.open_agent_attachment_picker().unwrap();
        let before = state
            .agent_attachment_picker
            .as_ref()
            .unwrap()
            .file_manager
            .cursor;

        let dispatch = handle_agent_attachment_picker_key(&mut state, key(KeyCode::Char('x')));

        assert_eq!(dispatch, AttachmentPickerKeyDispatch::Consumed);
        assert_eq!(state.mode, Mode::AttachFile);
        assert_eq!(
            state
                .agent_attachment_picker
                .as_ref()
                .unwrap()
                .file_manager
                .cursor,
            before
        );
        assert!(state.request_agent_attachment_delivery.is_none());
    }

    // TP-M1.3-PREPARE: Enter creates one typed request for the exact picker
    // target. Input handling performs no runtime send and keeps the overlay.
    #[test]
    fn attachment_picker_enter_prepares_one_typed_request_without_delivery() {
        let td = TempDir::new("attachment-enter-request");
        let path = td.root.join("literal ünicode.txt");
        td.file("literal ünicode.txt");
        let mut state = AppState::test_new();
        let mut workspace = crate::workspace::Workspace::test_new("attachment-enter-request");
        workspace.identity_cwd = td.root.clone();
        let pane_id = workspace.tabs[0].root_pane;
        let terminal_id = workspace.tabs[0].panes[&pane_id]
            .attached_terminal_id
            .clone();
        state.workspaces = vec![workspace];
        state.active = Some(0);
        state.mode = Mode::Terminal;
        state.ensure_test_terminals();
        state
            .terminals
            .get_mut(&terminal_id)
            .unwrap()
            .set_agent_name("codex".into());
        state.view.terminal_area = Rect::new(0, 0, 80, 24);
        state.open_agent_attachment_picker().unwrap();
        let picker = state.agent_attachment_picker.as_mut().unwrap();
        let entry_idx = picker
            .file_manager
            .entries
            .iter()
            .position(|entry| entry.path == path)
            .unwrap();
        picker.file_manager.select(entry_idx);

        let dispatch = handle_agent_attachment_picker_key(&mut state, key(KeyCode::Enter));

        assert_eq!(dispatch, AttachmentPickerKeyDispatch::Consumed);
        let request = state
            .request_agent_attachment_delivery
            .as_ref()
            .expect("one typed delivery request");
        assert_eq!(request.path, path);
        assert_eq!(request.target.pane_id, pane_id);
        assert_eq!(request.target.terminal_id, terminal_id);
        assert_eq!(state.mode, Mode::AttachFile);
        assert!(state.agent_attachment_picker.is_some());
    }

    #[test]
    fn attachment_frame_click_revalidates_exact_target_before_opening_picker() {
        let td = TempDir::new("attachment-frame-click");
        let mut app = super::super::app_for_mouse_test();
        let mut workspace = crate::workspace::Workspace::test_new("attachment-frame-click");
        workspace.identity_cwd = td.root.clone();
        let pane_id = workspace.test_split(ratatui::layout::Direction::Horizontal);
        let terminal_id = workspace.tabs[0].panes[&pane_id]
            .attached_terminal_id
            .clone();
        app.state.workspaces = vec![workspace];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.ensure_test_terminals();
        app.state
            .terminals
            .get_mut(&terminal_id)
            .unwrap()
            .set_agent_name("codex".into());
        compute_view(&mut app.state, Rect::new(0, 0, 100, 24));
        let action = app
            .state
            .view
            .agent_attachment_action_area
            .clone()
            .expect("agent action");

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            action.rect.x + 1,
            action.rect.y,
        ));

        assert_eq!(app.state.mode, Mode::AttachFile);
        assert_eq!(
            app.state
                .agent_attachment_picker
                .as_ref()
                .unwrap()
                .target
                .terminal_id,
            terminal_id
        );
    }

    // TP-M2.1-INPUT: frame coordinates are presentation only; the exact
    // workspace/pane/terminal and cached Git capability are revalidated.
    #[test]
    fn worktree_action_click_revalidates_exact_workspace_pane_and_terminal() {
        let mut app = super::super::app_for_mouse_test();
        let mut workspace = crate::workspace::Workspace::test_new("worktree-action-click");
        workspace.cached_git_space = Some(crate::workspace::GitSpaceMetadata {
            key: "repo".into(),
            checkout_key: "/repo".into(),
            label: "repo".into(),
            repo_root: "/repo".into(),
            is_linked_worktree: false,
        });
        let pane_id = workspace.test_split(ratatui::layout::Direction::Horizontal);
        let terminal_id = workspace.tabs[0].panes[&pane_id]
            .attached_terminal_id
            .clone();
        app.state.workspaces = vec![workspace];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.ensure_test_terminals();
        app.state
            .terminals
            .get_mut(&terminal_id)
            .unwrap()
            .set_agent_name("codex".into());
        compute_view(&mut app.state, Rect::new(0, 0, 100, 24));
        let action = app
            .state
            .view
            .agent_worktree_action_area
            .clone()
            .expect("worktree action");

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            action.rect.x + 1,
            action.rect.y,
        ));
        assert_eq!(app.state.request_open_existing_worktree, Some(0));
        assert!(app.state.request_new_linked_worktree.is_none());
        assert!(app.state.request_remove_linked_worktree.is_none());

        app.state.request_open_existing_worktree = None;
        app.handle_mouse(mouse_with_modifiers(
            MouseEventKind::Down(MouseButton::Left),
            action.rect.x + 1,
            action.rect.y,
            KeyModifiers::CONTROL,
        ));
        assert!(app.state.request_open_existing_worktree.is_none());

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            action.rect.x.saturating_sub(1),
            action.rect.y,
        ));
        assert!(app.state.request_open_existing_worktree.is_none());

        let original_workspace_id = app.state.workspaces[0].id.clone();
        app.state.workspaces[0].id = "replacement-workspace".into();
        app.state.view.agent_worktree_action_area = Some(action.clone());
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            action.rect.x + 1,
            action.rect.y,
        ));
        assert!(app.state.request_open_existing_worktree.is_none());

        app.state.workspaces[0].id = original_workspace_id;
        let root_pane = app.state.workspaces[0].tabs[0].root_pane;
        app.state.workspaces[0].tabs[0].layout.focus_pane(root_pane);
        app.state.view.agent_worktree_action_area = Some(action.clone());
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            action.rect.x + 1,
            action.rect.y,
        ));
        assert!(app.state.request_open_existing_worktree.is_none());

        app.state.workspaces[0].tabs[0].layout.focus_pane(pane_id);
        app.state
            .terminals
            .get_mut(&terminal_id)
            .unwrap()
            .set_agent_name(String::new());
        app.state.view.agent_worktree_action_area = Some(action.clone());
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            action.rect.x + 1,
            action.rect.y,
        ));
        assert!(app.state.request_open_existing_worktree.is_none());

        app.state
            .terminals
            .get_mut(&terminal_id)
            .unwrap()
            .set_agent_name("codex".into());
        app.state.workspaces[0].cached_git_space = None;
        app.state.view.agent_worktree_action_area = Some(action.clone());
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            action.rect.x + 1,
            action.rect.y,
        ));
        assert!(app.state.request_open_existing_worktree.is_none());
    }

    // TP-M1.2-MOUSE: the blocking picker owns exact current-row hit targets;
    // stale paths, modifiers, and coordinates outside those targets are inert.
    #[test]
    fn attachment_picker_mouse_selects_only_fresh_unmodified_current_row() {
        let td = TempDir::new("attachment-picker-mouse");
        let first = td.root.join("a.txt");
        let second = td.root.join("b.txt");
        td.file("a.txt");
        td.file("b.txt");
        let mut app = super::super::app_for_mouse_test();
        let mut workspace = crate::workspace::Workspace::test_new("attachment-picker-mouse");
        workspace.identity_cwd = td.root.clone();
        let pane_id = workspace.tabs[0].root_pane;
        let terminal_id = workspace.tabs[0].panes[&pane_id]
            .attached_terminal_id
            .clone();
        app.state.workspaces = vec![workspace];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.ensure_test_terminals();
        app.state
            .terminals
            .get_mut(&terminal_id)
            .unwrap()
            .set_agent_name("codex".into());
        app.state.view.terminal_area = Rect::new(0, 0, 120, 30);
        app.state.open_agent_attachment_picker().unwrap();
        compute_view(&mut app.state, Rect::new(0, 0, 120, 30));

        let second_row = app
            .state
            .view
            .agent_attachment_picker_row_areas
            .iter()
            .find(|row| row.entry_path == second)
            .cloned()
            .expect("second picker row");
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            second_row.rect.x,
            second_row.rect.y,
        ));
        assert_eq!(
            app.state.agent_attachment_selected_file(),
            Some(second.clone())
        );

        let first_idx = app
            .state
            .agent_attachment_picker
            .as_ref()
            .unwrap()
            .file_manager
            .entries
            .iter()
            .position(|entry| entry.path == first)
            .unwrap();
        app.state
            .agent_attachment_picker
            .as_mut()
            .unwrap()
            .file_manager
            .select(first_idx);
        let second_row_pos = app
            .state
            .view
            .agent_attachment_picker_row_areas
            .iter()
            .position(|row| row.entry_idx == second_row.entry_idx)
            .unwrap();
        app.state.view.agent_attachment_picker_row_areas[second_row_pos].entry_path =
            td.root.join("stale.txt");
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            second_row.rect.x,
            second_row.rect.y,
        ));
        assert_eq!(
            app.state.agent_attachment_selected_file(),
            Some(first.clone())
        );

        app.state.view.agent_attachment_picker_row_areas[second_row_pos].entry_path =
            second.clone();
        let mut modified = mouse(
            MouseEventKind::Down(MouseButton::Left),
            second_row.rect.x,
            second_row.rect.y,
        );
        modified.modifiers = KeyModifiers::CONTROL;
        app.handle_mouse(modified);
        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 0, 0));
        assert_eq!(app.state.agent_attachment_selected_file(), Some(first));
    }

    fn app_with_fm(fm: FmState) -> AppState {
        let mut app = AppState::test_new();
        app.try_open_file_manager_with(|_| Some(fm))
            .expect("Files activation");
        app
    }

    #[test]
    fn miller_divider_down_starts_typed_capture() {
        let td = TempDir::new("fm3-typed-divider-capture");
        td.file("00.txt");
        let mut app = runtime_app_with_fm(FmState::new(&td.root));
        install_focused_agent(&mut app);
        app.state.mobile_width_threshold = 0;
        app.state.sidebar_collapsed = true;
        compute_view(&mut app.state, Rect::new(0, 0, 86, 16));

        let divider = app
            .state
            .view
            .file_manager_miller
            .dividers
            .first()
            .expect("current Files projection exposes a divider")
            .rect;
        let before_model = {
            let file_manager = app.state.file_manager.as_ref().expect("open FM");
            (file_manager.miller.revision, file_manager.trio_overrides)
        };

        assert_eq!(
            app.handle_file_manager_mouse(mouse(
                MouseEventKind::Down(MouseButton::Left),
                divider.x,
                divider.y,
            )),
            FileManagerMouseDispatch::Consumed
        );
        let after_model = {
            let file_manager = app.state.file_manager.as_ref().expect("open FM");
            (file_manager.miller.revision, file_manager.trio_overrides)
        };
        assert_eq!(
            after_model, before_model,
            "divider down captures input without committing a model width"
        );
        assert_eq!(
            (
                app.state.miller_trio_drag.is_none(),
                app.state.shell_interaction.resize_active(),
            ),
            (true, true),
            "Miller divider down must start only the shared typed resize transaction"
        );
    }

    #[test]
    fn miller_drag_preview_changes_geometry_not_model_preferences() {
        let td = TempDir::new("fm3-divider-preview");
        td.file("00.txt");
        let mut app = runtime_app_with_fm(FmState::new(&td.root));
        install_focused_agent(&mut app);
        app.state.mobile_width_threshold = 0;
        app.state.sidebar_collapsed = true;
        let frame = Rect::new(0, 0, 86, 16);
        compute_view(&mut app.state, frame);

        let divider = app
            .state
            .view
            .file_manager_miller
            .dividers
            .first()
            .expect("current Files projection exposes a divider")
            .clone();
        let original_tracks = [
            app.state.view.file_manager_miller.columns[divider.left_column]
                .rect
                .width,
            app.state.view.file_manager_miller.columns[divider.right_column]
                .rect
                .width,
        ];
        let before_model = {
            let file_manager = app.state.file_manager.as_ref().expect("open FM");
            (
                file_manager.miller.revision,
                file_manager
                    .miller
                    .chain
                    .iter()
                    .map(|segment| segment.preferred_width)
                    .collect::<Vec<_>>(),
                file_manager.trio_overrides,
            )
        };

        app.handle_file_manager_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            divider.rect.x,
            divider.rect.y,
        ));
        assert_eq!(
            app.handle_file_manager_mouse(mouse(
                MouseEventKind::Drag(MouseButton::Left),
                divider.rect.x + 4,
                divider.rect.y,
            )),
            FileManagerMouseDispatch::Consumed
        );
        assert_eq!(
            app.state.shell_interaction.resize_preview_tracks(),
            Some([original_tracks[0] + 4, original_tracks[1] - 4]),
            "drag changes only the transient adjacent track pair"
        );

        compute_view(&mut app.state, frame);
        let preview_tracks = [
            app.state.view.file_manager_miller.columns[divider.left_column]
                .rect
                .width,
            app.state.view.file_manager_miller.columns[divider.right_column]
                .rect
                .width,
        ];
        assert_eq!(
            preview_tracks,
            [original_tracks[0] + 4, original_tracks[1] - 4],
            "fresh projection consumes the transient resize preview"
        );
        let after_model = {
            let file_manager = app.state.file_manager.as_ref().expect("open FM");
            (
                file_manager.miller.revision,
                file_manager
                    .miller
                    .chain
                    .iter()
                    .map(|segment| segment.preferred_width)
                    .collect::<Vec<_>>(),
                file_manager.trio_overrides,
            )
        };
        assert_eq!(
            after_model, before_model,
            "preview cannot commit a width or advance the Miller model"
        );
    }

    #[test]
    fn files_close_cancels_typed_miller_resize_capture() {
        let td = TempDir::new("fm3-divider-close-cancel");
        td.file("00.txt");
        let mut app = runtime_app_with_fm(FmState::new(&td.root));
        install_focused_agent(&mut app);
        app.state.mobile_width_threshold = 0;
        app.state.sidebar_collapsed = true;
        compute_view(&mut app.state, Rect::new(0, 0, 86, 16));

        let divider = app
            .state
            .view
            .file_manager_miller
            .dividers
            .first()
            .expect("current Files projection exposes a divider")
            .rect;
        assert_eq!(
            app.handle_file_manager_mouse(mouse(
                MouseEventKind::Down(MouseButton::Left),
                divider.x,
                divider.y,
            )),
            FileManagerMouseDispatch::Consumed
        );
        assert_eq!(
            app.handle_file_manager_mouse(mouse(
                MouseEventKind::Drag(MouseButton::Left),
                divider.x + 4,
                divider.y,
            )),
            FileManagerMouseDispatch::Consumed
        );
        assert!(
            app.state.shell_interaction.miller_resize_active(),
            "precondition: Files owns a typed Miller capture"
        );

        app.state.close_file_manager();

        assert!(
            !app.state.shell_interaction.resize_active(),
            "closing Files must retire its transient resize authority"
        );
        assert!(
            app.state.file_manager.is_none(),
            "the closed Files model cannot receive a later commit"
        );
        assert!(
            app.state.view.file_manager_miller.dividers.is_empty(),
            "closing Files retires the divider hit generation atomically"
        );
    }

    #[test]
    fn miller_mouse_up_commits_once() {
        let td = TempDir::new("fm3-divider-commit");
        td.file("00.txt");
        let mut app = runtime_app_with_fm(FmState::new(&td.root));
        install_focused_agent(&mut app);
        app.state.mobile_width_threshold = 0;
        app.state.sidebar_collapsed = true;
        let frame = Rect::new(0, 0, 86, 16);
        compute_view(&mut app.state, frame);

        let divider = app
            .state
            .view
            .file_manager_miller
            .dividers
            .first()
            .expect("current Files projection exposes a divider")
            .clone();
        let leading_chain_index = app.state.view.file_manager_miller.columns[divider.left_column]
            .kind
            .chain_index()
            .expect("leading projected column belongs to the Miller chain");
        let original_tracks = [
            app.state.view.file_manager_miller.columns[divider.left_column]
                .rect
                .width,
            app.state.view.file_manager_miller.columns[divider.right_column]
                .rect
                .width,
        ];
        let (before_revision, before_widths, before_overrides) = {
            let file_manager = app.state.file_manager.as_ref().expect("open FM");
            (
                file_manager.miller.revision,
                file_manager
                    .miller
                    .chain
                    .iter()
                    .map(|segment| segment.preferred_width)
                    .collect::<Vec<_>>(),
                file_manager.trio_overrides,
            )
        };

        assert_eq!(
            app.handle_file_manager_mouse(mouse(
                MouseEventKind::Down(MouseButton::Left),
                divider.rect.x,
                divider.rect.y,
            )),
            FileManagerMouseDispatch::Consumed
        );
        for delta in [2, 4, 6] {
            assert_eq!(
                app.handle_file_manager_mouse(mouse(
                    MouseEventKind::Drag(MouseButton::Left),
                    divider.rect.x + delta,
                    divider.rect.y,
                )),
                FileManagerMouseDispatch::Consumed
            );
        }
        let expected_tracks = [original_tracks[0] + 6, original_tracks[1] - 6];
        assert_eq!(
            app.state.shell_interaction.resize_preview_tracks(),
            Some(expected_tracks),
            "the final preview, not an intermediate move, is the commit candidate"
        );

        assert_eq!(
            app.handle_file_manager_mouse(mouse(
                MouseEventKind::Up(MouseButton::Left),
                divider.rect.x + 6,
                divider.rect.y,
            )),
            FileManagerMouseDispatch::Consumed
        );
        assert!(
            !app.state.shell_interaction.miller_resize_active(),
            "mouse-up must retire the typed Miller capture"
        );

        let (after_revision, after_widths, after_overrides) = {
            let file_manager = app.state.file_manager.as_ref().expect("open FM");
            (
                file_manager.miller.revision,
                file_manager
                    .miller
                    .chain
                    .iter()
                    .map(|segment| segment.preferred_width)
                    .collect::<Vec<_>>(),
                file_manager.trio_overrides,
            )
        };
        let mut expected_widths = before_widths;
        expected_widths[leading_chain_index] = expected_tracks[0];
        assert_eq!(
            (after_revision, after_widths, after_overrides),
            (before_revision + 1, expected_widths, before_overrides,),
            "mouse-up commits only the leading Miller preference and one revision"
        );

        let _ = app.handle_file_manager_mouse(mouse(
            MouseEventKind::Up(MouseButton::Left),
            divider.rect.x + 6,
            divider.rect.y,
        ));
        assert_eq!(
            app.state
                .file_manager
                .as_ref()
                .expect("open FM")
                .miller
                .revision,
            after_revision,
            "a repeated mouse-up cannot duplicate the commit"
        );
    }

    #[test]
    fn stale_miller_revision_mouse_up_retires_capture_without_commit() {
        let td = TempDir::new("fm3-divider-stale-revision");
        td.file("00.txt");
        let mut app = runtime_app_with_fm(FmState::new(&td.root));
        install_focused_agent(&mut app);
        app.state.mobile_width_threshold = 0;
        app.state.sidebar_collapsed = true;
        compute_view(&mut app.state, Rect::new(0, 0, 86, 16));

        let divider = app
            .state
            .view
            .file_manager_miller
            .dividers
            .first()
            .expect("current Files projection exposes a divider")
            .clone();
        let leading_chain_index = app.state.view.file_manager_miller.columns[divider.left_column]
            .kind
            .chain_index()
            .expect("leading projected column belongs to the Miller chain");
        let original_width = app
            .state
            .file_manager
            .as_ref()
            .expect("open FM")
            .miller
            .chain[leading_chain_index]
            .preferred_width;

        assert_eq!(
            app.handle_file_manager_mouse(mouse(
                MouseEventKind::Down(MouseButton::Left),
                divider.rect.x,
                divider.rect.y,
            )),
            FileManagerMouseDispatch::Consumed
        );
        assert_eq!(
            app.handle_file_manager_mouse(mouse(
                MouseEventKind::Drag(MouseButton::Left),
                divider.rect.x + 4,
                divider.rect.y,
            )),
            FileManagerMouseDispatch::Consumed
        );

        let authoritative_width = original_width + 1;
        let file_manager = app.state.file_manager.as_mut().expect("open FM");
        assert!(
            file_manager
                .miller
                .commit_column_width(leading_chain_index, authoritative_width),
            "precondition: another authority advances the Miller revision"
        );
        let authoritative_revision = file_manager.miller.revision;

        assert_eq!(
            app.handle_file_manager_mouse(mouse(
                MouseEventKind::Up(MouseButton::Left),
                divider.rect.x + 4,
                divider.rect.y,
            )),
            FileManagerMouseDispatch::Consumed
        );
        assert!(
            !app.state.shell_interaction.miller_resize_active(),
            "stale mouse-up must retire capture authority"
        );
        let file_manager = app.state.file_manager.as_ref().expect("open FM");
        assert_eq!(
            (
                file_manager.miller.revision,
                file_manager.miller.chain[leading_chain_index].preferred_width,
            ),
            (authoritative_revision, authoritative_width),
            "the stale preview cannot overwrite a newer model commit"
        );
    }

    #[test]
    fn terminal_resize_cancels_stale_miller_transaction() {
        let td = TempDir::new("fm3-divider-terminal-resize");
        td.file("00.txt");
        let mut app = runtime_app_with_fm(FmState::new(&td.root));
        install_focused_agent(&mut app);
        app.state.mobile_width_threshold = 0;
        app.state.sidebar_collapsed = true;
        let original_frame = Rect::new(0, 0, 86, 16);
        compute_view(&mut app.state, original_frame);

        let divider = app
            .state
            .view
            .file_manager_miller
            .dividers
            .first()
            .expect("current Files projection exposes a divider")
            .rect;
        let before_model = {
            let file_manager = app.state.file_manager.as_ref().expect("open FM");
            (
                file_manager.miller.revision,
                file_manager
                    .miller
                    .chain
                    .iter()
                    .map(|segment| segment.preferred_width)
                    .collect::<Vec<_>>(),
                file_manager.trio_overrides,
            )
        };
        assert_eq!(
            app.handle_file_manager_mouse(mouse(
                MouseEventKind::Down(MouseButton::Left),
                divider.x,
                divider.y,
            )),
            FileManagerMouseDispatch::Consumed
        );
        assert_eq!(
            app.handle_file_manager_mouse(mouse(
                MouseEventKind::Drag(MouseButton::Left),
                divider.x + 4,
                divider.y,
            )),
            FileManagerMouseDispatch::Consumed
        );
        assert!(
            app.state.shell_interaction.miller_resize_active(),
            "precondition: old frame owns a Miller capture"
        );

        compute_view(&mut app.state, Rect::new(0, 0, 70, 16));

        assert_eq!(
            app.state.view.shell.area.width, 70,
            "precondition: compute applies the terminal resize"
        );
        assert!(
            !app.state.shell_interaction.miller_resize_active(),
            "new terminal geometry must retire the old pointer transaction"
        );
        let file_manager = app.state.file_manager.as_ref().expect("open FM");
        assert_eq!(
            (
                file_manager.miller.revision,
                file_manager
                    .miller
                    .chain
                    .iter()
                    .map(|segment| segment.preferred_width)
                    .collect::<Vec<_>>(),
                file_manager.trio_overrides,
            ),
            before_model,
            "terminal resize cancellation cannot commit model or legacy widths"
        );
    }

    #[tokio::test]
    async fn miller_resize_escape_cancels_preview_without_closing_files() {
        let td = TempDir::new("fm3-divider-escape-cancel");
        td.file("00.txt");
        let mut app = runtime_app_with_fm(FmState::new(&td.root));
        install_focused_agent(&mut app);
        app.state.mobile_width_threshold = 0;
        app.state.sidebar_collapsed = true;
        compute_view(&mut app.state, Rect::new(0, 0, 86, 16));

        let divider = app
            .state
            .view
            .file_manager_miller
            .dividers
            .first()
            .expect("current Files projection exposes a divider")
            .rect;
        let before_model = {
            let file_manager = app.state.file_manager.as_ref().expect("open FM");
            (
                file_manager.miller.revision,
                file_manager
                    .miller
                    .chain
                    .iter()
                    .map(|segment| segment.preferred_width)
                    .collect::<Vec<_>>(),
                file_manager.trio_overrides,
            )
        };
        assert_eq!(
            app.handle_file_manager_mouse(mouse(
                MouseEventKind::Down(MouseButton::Left),
                divider.x,
                divider.y,
            )),
            FileManagerMouseDispatch::Consumed
        );
        assert_eq!(
            app.handle_file_manager_mouse(mouse(
                MouseEventKind::Drag(MouseButton::Left),
                divider.x + 4,
                divider.y,
            )),
            FileManagerMouseDispatch::Consumed
        );

        app.handle_key(crate::input::TerminalKey::new(
            KeyCode::Esc,
            KeyModifiers::NONE,
        ))
        .await;

        assert!(
            app.state.file_manager.is_some(),
            "Escape cancels the gesture without closing Files"
        );
        assert!(
            !app.state.shell_interaction.miller_resize_active(),
            "Escape retires Miller capture authority"
        );
        let file_manager = app.state.file_manager.as_ref().expect("Files remains open");
        assert_eq!(
            (
                file_manager.miller.revision,
                file_manager
                    .miller
                    .chain
                    .iter()
                    .map(|segment| segment.preferred_width)
                    .collect::<Vec<_>>(),
                file_manager.trio_overrides,
            ),
            before_model,
            "Escape cancellation cannot commit model or legacy widths"
        );
    }

    #[tokio::test]
    async fn miller_resize_keyboard_preview_and_enter_commit_once() {
        let td = TempDir::new("fm3-divider-keyboard-commit");
        td.file("00.txt");
        let mut app = runtime_app_with_fm(FmState::new(&td.root));
        install_focused_agent(&mut app);
        app.state.mobile_width_threshold = 0;
        app.state.sidebar_collapsed = true;
        compute_view(&mut app.state, Rect::new(0, 0, 86, 16));

        let divider = app
            .state
            .view
            .file_manager_miller
            .dividers
            .first()
            .expect("current Files projection exposes a divider")
            .clone();
        let leading_chain_index = app.state.view.file_manager_miller.columns[divider.left_column]
            .kind
            .chain_index()
            .expect("leading projected column belongs to the Miller chain");
        let original_tracks = [
            app.state.view.file_manager_miller.columns[divider.left_column]
                .rect
                .width,
            app.state.view.file_manager_miller.columns[divider.right_column]
                .rect
                .width,
        ];
        let (before_revision, before_widths, before_overrides) = {
            let file_manager = app.state.file_manager.as_ref().expect("open FM");
            (
                file_manager.miller.revision,
                file_manager
                    .miller
                    .chain
                    .iter()
                    .map(|segment| segment.preferred_width)
                    .collect::<Vec<_>>(),
                file_manager.trio_overrides,
            )
        };
        assert_eq!(
            app.handle_file_manager_mouse(mouse(
                MouseEventKind::Down(MouseButton::Left),
                divider.rect.x,
                divider.rect.y,
            )),
            FileManagerMouseDispatch::Consumed
        );

        for code in [KeyCode::Right, KeyCode::Char('l')] {
            app.handle_key(crate::input::TerminalKey::new(code, KeyModifiers::NONE))
                .await;
        }
        assert_eq!(
            app.state.shell_interaction.resize_preview_tracks(),
            Some([original_tracks[0] + 2, original_tracks[1] - 2]),
            "keyboard steps share the transient Miller preview reducer"
        );
        let file_manager = app.state.file_manager.as_ref().expect("open FM");
        assert_eq!(
            (
                file_manager.miller.revision,
                file_manager
                    .miller
                    .chain
                    .iter()
                    .map(|segment| segment.preferred_width)
                    .collect::<Vec<_>>(),
                file_manager.trio_overrides,
            ),
            (before_revision, before_widths.clone(), before_overrides),
            "keyboard preview cannot mutate committed model state"
        );

        app.handle_key(crate::input::TerminalKey::new(
            KeyCode::Enter,
            KeyModifiers::NONE,
        ))
        .await;

        assert!(
            app.state.file_manager.is_some(),
            "Enter commits the gesture without closing Files"
        );
        assert!(
            !app.state.shell_interaction.miller_resize_active(),
            "Enter retires Miller capture authority"
        );
        let file_manager = app.state.file_manager.as_ref().expect("Files remains open");
        let mut expected_widths = before_widths;
        expected_widths[leading_chain_index] = original_tracks[0] + 2;
        assert_eq!(
            (
                file_manager.miller.revision,
                file_manager
                    .miller
                    .chain
                    .iter()
                    .map(|segment| segment.preferred_width)
                    .collect::<Vec<_>>(),
                file_manager.trio_overrides,
            ),
            (before_revision + 1, expected_widths, before_overrides),
            "Enter performs exactly one final model commit"
        );
    }

    // FM2.2 end-to-end: pressing a trio divider and dragging resizes the
    // column through the clamped commit seam; release ends the capture; the
    // committed width survives recompute and clamps to the frozen 16..=64
    // bounds. This is the user-visible custom-layout interaction.
    #[test]
    fn divider_drag_resizes_trio_columns_end_to_end() {
        let td = TempDir::new("fm2-divider-drag");
        td.file("00.txt");
        let mut app = runtime_app_with_fm(FmState::new(&td.root));
        install_focused_agent(&mut app);
        app.state.mobile_width_threshold = 0;
        app.state.sidebar_collapsed = true;
        compute_view(&mut app.state, Rect::new(0, 0, 60, 16));

        let center = app.state.view.terminal_area;
        let overrides = app
            .state
            .file_manager
            .as_ref()
            .expect("open FM")
            .trio_overrides;
        assert_eq!(overrides, crate::fm::miller::MillerTrioOverrides::default());
        let dividers = crate::ui::file_manager_divider_areas(center, overrides);
        let divider = dividers[0].expect("three-column layout exposes the first divider");
        let original = crate::ui::file_manager_column_widths(center, overrides)[0]
            .expect("parent column width");

        // Press on the divider begins the capture and consumes the event.
        assert_eq!(
            app.handle_file_manager_mouse(mouse(
                MouseEventKind::Down(MouseButton::Left),
                divider.x,
                divider.y + 2,
            )),
            FileManagerMouseDispatch::Consumed
        );
        assert!(
            app.state.miller_trio_drag.is_some(),
            "capture begins on press"
        );

        // Dragging right widens the parent column through the clamped seam.
        assert_eq!(
            app.handle_file_manager_mouse(mouse(
                MouseEventKind::Drag(MouseButton::Left),
                divider.x + 4,
                divider.y + 2,
            )),
            FileManagerMouseDispatch::Consumed
        );
        let widened = app
            .state
            .file_manager
            .as_ref()
            .expect("open FM")
            .trio_overrides
            .parent
            .expect("committed parent width");
        assert_eq!(
            widened,
            (original + 4).clamp(
                crate::fm::miller::MILLER_COLUMN_MIN_WIDTH,
                crate::fm::miller::MILLER_COLUMN_MAX_WIDTH
            ),
            "drag commits the clamped width"
        );

        // Release ends the capture; the width survives a fresh compute and
        // the layout honors it.
        assert_eq!(
            app.handle_file_manager_mouse(mouse(
                MouseEventKind::Up(MouseButton::Left),
                divider.x + 4,
                divider.y + 2,
            )),
            FileManagerMouseDispatch::Consumed
        );
        assert!(
            app.state.miller_trio_drag.is_none(),
            "release ends the capture"
        );
        compute_view(&mut app.state, Rect::new(0, 0, 60, 16));
        let overrides_after = app
            .state
            .file_manager
            .as_ref()
            .expect("open FM")
            .trio_overrides;
        let new_parent = crate::ui::file_manager_column_widths(center, overrides_after)[0]
            .expect("parent column width after drag");
        assert_eq!(new_parent, widened, "the layout honors the committed width");

        // A far drag clamps to the frozen bounds instead of exploding.
        let dividers = crate::ui::file_manager_divider_areas(center, overrides_after);
        let divider = dividers[0].expect("divider after resize");
        app.handle_file_manager_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            divider.x,
            divider.y + 2,
        ));
        app.handle_file_manager_mouse(mouse(
            MouseEventKind::Drag(MouseButton::Left),
            divider.x.saturating_add(200),
            divider.y + 2,
        ));
        assert_eq!(
            app.state
                .file_manager
                .as_ref()
                .expect("open FM")
                .trio_overrides
                .parent,
            Some(crate::fm::miller::MILLER_COLUMN_MAX_WIDTH),
            "a runaway drag clamps to the 64-cell maximum"
        );
        app.handle_file_manager_mouse(mouse(
            MouseEventKind::Up(MouseButton::Left),
            divider.x,
            divider.y + 2,
        ));
    }

    // P0 characterization: the published legacy-trio adapter exposes two
    // independent divider slots. Pin the second one before the windowed
    // projection and shared transaction replace this compatibility path.
    #[test]
    fn second_legacy_divider_is_interactive() {
        let td = TempDir::new("fm2-second-divider");
        td.file("00.txt");
        let mut app = runtime_app_with_fm(FmState::new(&td.root));
        install_focused_agent(&mut app);
        app.state.mobile_width_threshold = 0;
        app.state.sidebar_collapsed = true;
        compute_view(&mut app.state, Rect::new(0, 0, 60, 16));

        let center = app.state.view.terminal_area;
        let original_overrides = app
            .state
            .file_manager
            .as_ref()
            .expect("open FM")
            .trio_overrides;
        let divider = crate::ui::file_manager_divider_areas(center, original_overrides)[1]
            .expect("three-column layout exposes the second divider");
        let original_current = crate::ui::file_manager_column_widths(center, original_overrides)[1]
            .expect("current column width");

        assert_eq!(
            app.handle_file_manager_mouse(mouse(
                MouseEventKind::Down(MouseButton::Left),
                divider.x,
                divider.y + 2,
            )),
            FileManagerMouseDispatch::Consumed
        );
        assert_eq!(
            app.state.miller_trio_drag.map(|drag| drag.slot),
            Some(1),
            "the second divider owns the current-column slot"
        );
        assert_eq!(
            app.handle_file_manager_mouse(mouse(
                MouseEventKind::Drag(MouseButton::Left),
                divider.x + 3,
                divider.y + 2,
            )),
            FileManagerMouseDispatch::Consumed
        );

        let resized = app
            .state
            .file_manager
            .as_ref()
            .expect("open FM")
            .trio_overrides;
        assert_eq!(
            resized.parent, original_overrides.parent,
            "the second divider must not rewrite the parent slot"
        );
        assert_eq!(
            resized.current,
            Some((original_current + 3).clamp(
                crate::fm::miller::MILLER_COLUMN_MIN_WIDTH,
                crate::fm::miller::MILLER_COLUMN_MAX_WIDTH
            )),
            "the second divider changes only the current width"
        );

        assert_eq!(
            app.handle_file_manager_mouse(mouse(
                MouseEventKind::Up(MouseButton::Left),
                divider.x + 3,
                divider.y + 2,
            )),
            FileManagerMouseDispatch::Consumed
        );
        assert!(
            app.state.miller_trio_drag.is_none(),
            "release clears the second-divider capture"
        );
    }

    // P0 characterization: Files close is an authority boundary. An old
    // divider gesture cannot survive close/reopen and retarget a fresh Files
    // generation even when the new surface occupies the same coordinates.
    #[test]
    fn files_close_retires_legacy_drag_and_hits() {
        let td = TempDir::new("fm2-close-retires-drag");
        td.file("00.txt");
        let mut app = runtime_app_with_fm(FmState::new(&td.root));
        install_focused_agent(&mut app);
        app.state.mobile_width_threshold = 0;
        app.state.sidebar_collapsed = true;
        compute_view(&mut app.state, Rect::new(0, 0, 60, 16));

        let old_center = app.state.view.terminal_area;
        let old_overrides = app
            .state
            .file_manager
            .as_ref()
            .expect("open FM")
            .trio_overrides;
        let old_divider = crate::ui::file_manager_divider_areas(old_center, old_overrides)[0]
            .expect("three-column layout exposes the first divider");

        assert_eq!(
            app.handle_file_manager_mouse(mouse(
                MouseEventKind::Down(MouseButton::Left),
                old_divider.x,
                old_divider.y + 2,
            )),
            FileManagerMouseDispatch::Consumed
        );
        assert!(
            app.state.miller_trio_drag.is_some(),
            "precondition: old Files generation owns a capture"
        );

        app.state.close_file_manager();
        assert!(
            app.state.miller_trio_drag.is_none(),
            "close retires the active Files capture"
        );
        assert!(
            app.state.view.file_manager_row_areas.is_empty()
                && app.state.view.file_manager_row_action_areas.is_empty()
                && app.state.view.file_manager_header_action_areas.is_empty(),
            "close retires projected Files hits in the same transaction"
        );

        app.state
            .try_open_file_manager_with(|_| Some(FmState::new(&td.root)))
            .expect("fresh Files activation");
        compute_view(&mut app.state, Rect::new(0, 0, 60, 16));
        let fresh_before = app
            .state
            .file_manager
            .as_ref()
            .expect("reopened FM")
            .trio_overrides;

        let _ = app.handle_file_manager_mouse(mouse(
            MouseEventKind::Drag(MouseButton::Left),
            old_divider.x.saturating_add(4),
            old_divider.y + 2,
        ));
        let _ = app.handle_file_manager_mouse(mouse(
            MouseEventKind::Up(MouseButton::Left),
            old_divider.x.saturating_add(4),
            old_divider.y + 2,
        ));

        assert!(
            app.state.miller_trio_drag.is_none(),
            "stale move/up cannot recreate a capture"
        );
        assert_eq!(
            app.state
                .file_manager
                .as_ref()
                .expect("reopened FM")
                .trio_overrides,
            fresh_before,
            "stale move/up cannot commit widths into the fresh Files generation"
        );
    }

    // SF6.2: Files input routes from the TYPED stage authority
    // (`AppSurfaceRef::NativeFiles`), not the legacy `file_manager.is_some()`
    // boolean. The adversarial divergent state (Files domain state present
    // while the typed stage says TerminalWorkspace) pins which source owns
    // keyboard and mouse routing.
    #[test]
    fn files_input_routes_from_typed_surface_authority() {
        let td = TempDir::new("files-typed-input-authority");
        td.file("00.txt");
        let mut app = runtime_app_with_fm(FmState::new(&td.root));
        install_focused_agent(&mut app);
        app.state.mobile_width_threshold = 0;
        app.state.sidebar_collapsed = true;
        compute_view(&mut app.state, Rect::new(0, 0, 60, 16));

        // Control: the aligned NativeFiles state owns the keyboard tier and
        // consumes in-center mouse events.
        assert_eq!(
            app.state.stage.surface_view(),
            crate::ui::surface_host::StageSurfaceView::NativeFiles,
            "fixture: Files owns the stage"
        );
        assert_eq!(
            app.state.shell_key_input_owner(),
            crate::app::input::shell::ShellInputOwner::FocusedComponent
        );
        let center = app.state.view.terminal_area;
        let probe = (
            center.x + center.width.saturating_sub(2),
            center.y + center.height.saturating_sub(2),
        );
        assert_eq!(
            app.handle_file_manager_mouse(mouse(
                MouseEventKind::Down(MouseButton::Left),
                probe.0,
                probe.1
            )),
            FileManagerMouseDispatch::Consumed,
            "control: the aligned Files surface consumes in-center mouse input"
        );

        // Adversarial divergence: Files domain state present while the typed
        // stage says TerminalWorkspace. The TYPED authority must own routing
        // on BOTH input paths.
        app.state.stage.close_files();
        assert!(app.state.file_manager.is_some(), "divergent fixture holds");
        assert_ne!(
            app.state.shell_key_input_owner(),
            crate::app::input::shell::ShellInputOwner::FocusedComponent,
            "the typed stage authority must own keyboard routing"
        );
        assert_eq!(
            app.handle_file_manager_mouse(mouse(
                MouseEventKind::Down(MouseButton::Left),
                probe.0,
                probe.1
            )),
            FileManagerMouseDispatch::NotHandled,
            "the typed stage authority must own mouse routing"
        );
    }

    // SF4.2-08: with the Files surface covering the workspace stage, no
    // mouse event inside the covered center may reach the hidden terminal —
    // no selection anchor, no pane focus, no context/terminal side effect —
    // and the keyboard tier belongs to the focused file manager. The control
    // phase proves the SAME press reaches the live terminal once the Files
    // surface closes, so the seal cannot pass vacuously.
    #[test]
    fn files_stage_blocks_hidden_terminal_input() {
        let td = TempDir::new("files-stage-input-seal");
        td.file("00.txt");
        td.file("01.txt");
        let mut app = runtime_app_with_fm(FmState::new(&td.root));
        install_focused_agent(&mut app);
        app.state.mobile_width_threshold = 0;
        app.state.sidebar_collapsed = true;
        compute_view(&mut app.state, Rect::new(0, 0, 60, 16));

        let center = app.state.view.terminal_area;
        let probe = (
            center.x + center.width.saturating_sub(2),
            center.y + center.height.saturating_sub(2),
        );
        let on_fm_control = app
            .state
            .view
            .file_manager_row_areas
            .iter()
            .map(|row| row.rect)
            .chain(
                app.state
                    .view
                    .file_manager_header_action_areas
                    .iter()
                    .map(|area| area.rect),
            )
            .any(|rect| {
                probe.0 >= rect.x
                    && probe.0 < rect.x.saturating_add(rect.width)
                    && probe.1 >= rect.y
                    && probe.1 < rect.y.saturating_add(rect.height)
            });
        assert!(
            !on_fm_control,
            "fixture: the probe must target covered terrain, not an FM control"
        );

        for kind in [
            MouseEventKind::Down(MouseButton::Left),
            MouseEventKind::Drag(MouseButton::Left),
            MouseEventKind::Up(MouseButton::Left),
            MouseEventKind::Moved,
            MouseEventKind::ScrollUp,
            MouseEventKind::ScrollDown,
            MouseEventKind::Down(MouseButton::Middle),
            MouseEventKind::Down(MouseButton::Right),
        ] {
            app.handle_mouse(mouse(kind, probe.0, probe.1));
            assert!(
                app.state.selection.is_none(),
                "{kind:?} must not anchor a hidden terminal selection"
            );
            assert!(
                app.state.file_manager.is_some(),
                "{kind:?} must not close the Files surface"
            );
            assert_eq!(
                app.state.mode,
                Mode::Terminal,
                "{kind:?} must not change the mode"
            );
        }
        assert!(
            app.state.context_menu.is_none(),
            "a non-row right-click must not open a menu"
        );
        assert_eq!(
            app.state.shell_key_input_owner(),
            super::super::shell::ShellInputOwner::FocusedComponent,
            "the open Files surface owns the keyboard tier"
        );

        // Control: once the Files surface closes, the SAME press reaches the
        // live terminal and anchors a selection.
        app.state.close_file_manager();
        compute_view(&mut app.state, Rect::new(0, 0, 60, 16));
        assert!(
            app.state.pane_at(probe.0, probe.1).is_some(),
            "control fixture: the probe must sit on the live pane"
        );
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            probe.0,
            probe.1,
        ));
        assert!(
            app.state.selection.is_some(),
            "control: the same press must reach the live terminal"
        );
    }

    fn runtime_app_with_fm(fm: FmState) -> crate::app::App {
        let mut app = super::super::app_for_mouse_test();
        app.state
            .try_open_file_manager_with(|_| Some(fm))
            .expect("Files activation");
        app.state.view.terminal_area = Rect::new(26, 0, 20, 6);
        let entry_paths = app
            .state
            .file_manager
            .as_ref()
            .map(|file_manager| {
                file_manager
                    .entries
                    .iter()
                    .take(4)
                    .map(|entry| entry.path.clone())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        app.state.view.file_manager_row_areas = entry_paths
            .into_iter()
            .enumerate()
            .map(|(entry_idx, entry_path)| FileManagerRowArea {
                rect: Rect::new(26, 2 + entry_idx as u16, 20, 1),
                entry_idx,
                entry_path,
            })
            .collect();
        let files_generation = app
            .state
            .stage
            .active_instance_generation()
            .expect("active Files generation");
        app.state.view.file_manager_miller = crate::ui::project_miller_view(
            Rect::new(26, 1, 20, 5),
            app.state.file_manager.as_ref().expect("open FM"),
            files_generation,
        );
        app
    }

    fn install_row_actions(app: &mut crate::app::App, entry_idx: usize) -> PathBuf {
        let entry_path = app
            .state
            .file_manager
            .as_ref()
            .and_then(|file_manager| file_manager.entries.get(entry_idx))
            .expect("row-action fixture entry")
            .path
            .clone();
        let row = app
            .state
            .view
            .file_manager_row_areas
            .iter_mut()
            .find(|row| row.entry_idx == entry_idx)
            .expect("row-action fixture row");
        row.rect.width = 17;
        app.state.view.file_manager_row_action_areas = FileManagerRowAction::ALL
            .into_iter()
            .enumerate()
            .map(|(action_idx, action)| FileManagerRowActionArea {
                rect: Rect::new(43 + action_idx as u16, row.rect.y, 1, 1),
                entry_idx,
                entry_path: entry_path.clone(),
                action,
            })
            .collect();
        entry_path
    }

    fn install_focused_agent(app: &mut crate::app::App) -> crate::terminal::TerminalId {
        let workspace = crate::workspace::Workspace::test_new("fm-agent-handoff");
        let pane_id = workspace.tabs[0].root_pane;
        let terminal_id = workspace
            .terminal_id(pane_id)
            .expect("focused agent terminal id")
            .clone();
        app.state.workspaces = vec![workspace];
        app.state.ensure_test_terminals();
        app.state
            .terminals
            .get_mut(&terminal_id)
            .expect("focused agent terminal state")
            .set_agent_name("fm-target".into());
        app.state.active = Some(0);
        app.state.selected = 0;
        terminal_id
    }

    fn mouse(kind: MouseEventKind, col: u16, row: u16) -> MouseEvent {
        MouseEvent {
            kind,
            column: col,
            row,
            modifiers: KeyModifiers::NONE,
        }
    }

    fn mouse_with_modifiers(
        kind: MouseEventKind,
        col: u16,
        row: u16,
        modifiers: KeyModifiers,
    ) -> MouseEvent {
        MouseEvent {
            kind,
            column: col,
            row,
            modifiers,
        }
    }

    fn install_wide_header_actions(app: &mut crate::app::App) {
        app.state.view.terminal_area = Rect::new(26, 0, 60, 6);
        app.state.view.file_manager_header_action_areas = vec![
            FileManagerHeaderActionArea {
                rect: Rect::new(50, 0, 6, 1),
                action: FileManagerHeaderAction::Copy,
            },
            FileManagerHeaderActionArea {
                rect: Rect::new(57, 0, 7, 1),
                action: FileManagerHeaderAction::Paste,
            },
            FileManagerHeaderActionArea {
                rect: Rect::new(65, 0, 12, 1),
                action: FileManagerHeaderAction::NewFolder,
            },
            FileManagerHeaderActionArea {
                rect: Rect::new(78, 0, 8, 1),
                action: FileManagerHeaderAction::Delete,
            },
        ];
        app.state.view.file_manager_action_bar = Some(FileManagerActionBarModel {
            selection: None,
            clipboard_count: 0,
            actions: FileManagerHeaderAction::ALL.map(|action| FileManagerActionState {
                action,
                enabled: true,
                disabled_reason: None,
            }),
        });
    }

    // TP-A3.5: j/k (and arrows) move the cursor within the list.
    #[test]
    fn jk_moves_cursor() {
        let td = TempDir::new("jk");
        td.file("a");
        td.file("b");
        td.file("c");
        let mut app = app_with_fm(FmState::new(&td.root));

        handle_file_manager_key(&mut app, key(KeyCode::Char('j')));
        assert_eq!(app.file_manager.as_ref().unwrap().cursor, 1);
        handle_file_manager_key(&mut app, key(KeyCode::Down));
        assert_eq!(app.file_manager.as_ref().unwrap().cursor, 2);
        handle_file_manager_key(&mut app, key(KeyCode::Char('k')));
        assert_eq!(app.file_manager.as_ref().unwrap().cursor, 1);
    }

    // TP-A3.6: Enter descends into a directory; Backspace returns to the parent.
    #[test]
    fn enter_and_backspace_navigate_directories() {
        let td = TempDir::new("nav");
        td.dir("sub");
        fs::write(td.root.join("sub").join("inner"), b"x").expect("write inner");
        let mut app = app_with_fm(FmState::new(&td.root));

        let dispatch = handle_file_manager_key(&mut app, key(KeyCode::Enter));
        apply_test_navigation(&mut app, dispatch);
        assert_eq!(
            app.file_manager.as_ref().unwrap().cwd,
            td.root.join("sub"),
            "enter descends into the directory"
        );

        let dispatch = handle_file_manager_key(&mut app, key(KeyCode::Backspace));
        apply_test_navigation(&mut app, dispatch);
        assert_eq!(
            app.file_manager.as_ref().unwrap().cwd,
            td.root,
            "backspace returns to the parent"
        );
    }

    // TP-FM4-APP-ADAPTER: input emits a typed request but performs no
    // filesystem preparation or model transition. The App layer owns prepare
    // and generation-safe apply after this pure dispatch step.
    #[test]
    fn keyboard_enter_emits_navigation_request_without_mutating_state() {
        let td = TempDir::new("typed-keyboard-navigation");
        td.dir("child");
        let child = td.root.join("child");
        let mut state = app_with_fm(FmState::new(&td.root));
        let before = state.file_manager.as_ref().expect("open FM").clone();

        let dispatch = handle_file_manager_key(&mut state, key(KeyCode::Enter));

        let FileManagerKeyDispatch::Navigate(request) = dispatch else {
            panic!("directory Enter must emit a typed navigation request");
        };
        assert_eq!(request.reason, crate::fm::FmNavigationReason::Enter);
        assert_eq!(request.source_directory, td.root);
        assert_eq!(request.target_directory, child);
        let after = state.file_manager.as_ref().expect("open FM");
        assert_eq!(after.cwd, before.cwd);
        assert_eq!(after.entries, before.entries);
        assert_eq!(after.cursor, before.cursor);
        assert_eq!(after.directory_generation, before.directory_generation);
        assert_eq!(after.preview_generation, before.preview_generation);
        assert_eq!(after.miller, before.miller);
    }

    // P5 RED: '.' emits one exact hidden-refresh intent without reading disk
    // or mutating the Files model in the input layer.
    #[test]
    fn dot_emits_hidden_refresh_request_without_mutating_files() {
        let td = TempDir::new("hidden");
        td.file("shown");
        td.file(".secret");
        let mut app = app_with_fm(FmState::new(&td.root));
        let before = app.file_manager.as_ref().expect("open FM").clone();

        let dispatch = handle_file_manager_key(&mut app, key(KeyCode::Char('.')));

        let FileManagerKeyDispatch::Refresh(request) = dispatch else {
            panic!("dot must emit one typed hidden-refresh request");
        };
        assert_eq!(
            request.reason,
            crate::fm::FmCurrentRefreshReason::ToggleHidden
        );
        assert_eq!(request.source_directory, td.root);
        assert_eq!(request.source_show_hidden, before.show_hidden);
        assert_eq!(request.target_show_hidden, !before.show_hidden);
        assert_eq!(
            request.files_generation,
            app.stage
                .active_instance_generation()
                .expect("active Files generation")
        );
        let after = app.file_manager.as_ref().expect("open FM");
        assert_eq!(after.entries, before.entries);
        assert_eq!(after.cursor, before.cursor);
        assert_eq!(after.show_hidden, before.show_hidden);
        assert_eq!(after.directory_generation, before.directory_generation);
        assert_eq!(after.preview_generation, before.preview_generation);
        assert_eq!(after.miller, before.miller);
    }

    #[test]
    fn app_hidden_toggle_applies_once_and_rejects_replay() {
        let td = TempDir::new("hidden-app-adapter");
        td.file("shown");
        td.file(".secret");
        let mut app = runtime_app_with_fm(FmState::new(&td.root));
        let before = app.state.file_manager.as_ref().expect("open FM").clone();
        let dispatch = handle_file_manager_key(&mut app.state, key(KeyCode::Char('.')));
        let FileManagerKeyDispatch::Refresh(request) = dispatch else {
            panic!("dot must emit hidden refresh");
        };
        let stale_replay = request.clone();

        assert_eq!(
            app.execute_file_manager_current_refresh(request),
            Some(true)
        );
        let toggled = app.state.file_manager.as_ref().expect("open FM");
        assert!(toggled.show_hidden);
        assert_eq!(toggled.entries.len(), 2);
        assert_eq!(
            toggled.directory_generation,
            before.directory_generation + 1
        );
        assert_eq!(toggled.preview_generation, before.preview_generation + 1);
        let once = toggled.clone();

        assert_eq!(
            app.execute_file_manager_current_refresh(stale_replay),
            None,
            "the first request cannot replay after its generations retire"
        );
        let after_replay = app.state.file_manager.as_ref().expect("open FM");
        assert_eq!(after_replay.entries, once.entries);
        assert_eq!(after_replay.show_hidden, once.show_hidden);
        assert_eq!(after_replay.directory_generation, once.directory_generation);
        assert_eq!(after_replay.preview_generation, once.preview_generation);

        let dispatch = handle_file_manager_key(&mut app.state, key(KeyCode::Char('.')));
        let FileManagerKeyDispatch::Refresh(request) = dispatch else {
            panic!("second dot must emit a fresh hidden refresh");
        };
        assert_eq!(
            request.reason,
            crate::fm::FmCurrentRefreshReason::ToggleHidden
        );
        assert!(request.source_show_hidden);
        assert!(!request.target_show_hidden);
        assert_eq!(
            app.execute_file_manager_current_refresh(request),
            Some(true)
        );
        let restored = app.state.file_manager.as_ref().expect("open FM");
        assert!(!restored.show_hidden);
        assert_eq!(restored.entries.len(), 1);
        assert_eq!(
            restored.directory_generation,
            before.directory_generation + 2
        );
        assert_eq!(restored.preview_generation, before.preview_generation + 2);
    }

    // TP-A3.7: Esc and q both close the file manager.
    #[test]
    fn esc_and_q_close() {
        let td = TempDir::new("close");
        td.file("a");

        let mut app = app_with_fm(FmState::new(&td.root));
        handle_file_manager_key(&mut app, key(KeyCode::Esc));
        assert!(app.file_manager.is_none(), "esc closes the file manager");

        let mut app = app_with_fm(FmState::new(&td.root));
        handle_file_manager_key(&mut app, key(KeyCode::Char('q')));
        assert!(app.file_manager.is_none(), "q closes the file manager");
    }

    // TP-C4.4-CANCEL: Esc is the user cancellation route while an operation
    // is running. It must emit a typed, repeatable intent without closing the
    // file manager; the App/worker boundary owns the cancellation side effect.
    #[test]
    fn esc_emits_repeatable_operation_cancel_intent_while_running() {
        let td = TempDir::new("cancel-running-operation");
        td.file("source.txt");
        let source = td.root.join("source.txt");
        let mut app = app_with_fm(FmState::new(&td.root));
        app.file_manager_operation = Some(crate::app::state::FileManagerOperationState {
            generation: 7,
            kind: crate::app::state::FileManagerOperationKind::Copy,
            destination_directory: td.root.clone(),
            total_items: 1,
            completed_items: 0,
            failed_items: 0,
            status: crate::app::state::FileManagerOperationStatus::Running,
            items: vec![crate::app::state::FileManagerOperationItemState {
                path: source,
                recovery_path: None,
                status: crate::app::state::FileManagerOperationItemStatus::Running,
            }],
        });

        assert_eq!(
            handle_file_manager_key(&mut app, key(KeyCode::Esc)),
            FileManagerKeyDispatch::CancelOperation
        );
        assert_eq!(
            handle_file_manager_key(&mut app, key(KeyCode::Esc)),
            FileManagerKeyDispatch::CancelOperation
        );
        assert!(app.file_manager.is_some());
    }

    // TP-A3.3-DISPATCH: one left press on a visible CURRENT row selects its
    // absolute entry and refreshes the preview cache for that selection.
    #[test]
    fn single_click_selects_current_row_and_refreshes_preview() {
        let td = TempDir::new("mouse-single");
        td.dir("alpha-dir");
        td.file("beta.txt");
        let mut app = runtime_app_with_fm(FmState::new(&td.root));

        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 27, 3));

        let fm = app.state.file_manager.as_ref().expect("file manager open");
        assert_eq!(fm.cursor, 1);
        assert_eq!(
            fm.selected().map(|entry| entry.name.as_str()),
            Some("beta.txt")
        );
        assert!(matches!(fm.preview, crate::fm::FmPreview::File(_)));
    }

    // TP-FM3-CURRENT-CUTOVER: CURRENT plain-click authority comes from the
    // generation-safe Miller row target, not the legacy compatibility row
    // list. Removing the legacy list must not disable an exact live click.
    #[test]
    fn current_plain_click_uses_typed_miller_target_without_legacy_rows() {
        let td = TempDir::new("typed-current-click");
        for index in 0..3 {
            td.file(&format!("{index:02}.txt"));
        }
        let mut app = runtime_app_with_fm(FmState::new(&td.root));
        install_focused_agent(&mut app);
        app.state.mobile_width_threshold = 0;
        app.state.sidebar_collapsed = true;
        compute_view(&mut app.state, Rect::new(0, 0, 80, 16));
        let target = app
            .state
            .view
            .file_manager_miller
            .columns
            .iter()
            .find(|column| column.kind.is_current())
            .and_then(|column| column.rows.get(1))
            .cloned()
            .expect("second typed CURRENT row");
        app.state.view.file_manager_row_areas.clear();

        assert_eq!(
            app.handle_file_manager_mouse(mouse(
                MouseEventKind::Down(MouseButton::Left),
                target.rect.x,
                target.rect.y,
            )),
            FileManagerMouseDispatch::Consumed
        );

        let file_manager = app.state.file_manager.as_ref().expect("open FM");
        assert_eq!(file_manager.cursor, target.entry_index);
        assert_eq!(
            file_manager.selected().map(|entry| &entry.path),
            Some(&target.entry_path)
        );
    }

    // TP-FM3-OVERLAY: the typed Files handler remains fail-closed while a
    // topmost overlay owns input. Exact live row geometry cannot select,
    // activate, scroll, or open a second context surface behind that overlay.
    #[test]
    fn overlay_blocks_every_typed_miller_row_gesture() {
        let td = TempDir::new("typed-row-overlay");
        td.dir("directory");
        td.file("file.txt");
        let mut app = runtime_app_with_fm(FmState::new(&td.root));
        install_focused_agent(&mut app);
        app.state.mobile_width_threshold = 0;
        app.state.sidebar_collapsed = true;
        compute_view(&mut app.state, Rect::new(0, 0, 80, 16));
        let target = app
            .state
            .view
            .file_manager_miller
            .columns
            .iter()
            .find(|column| column.kind.is_current())
            .and_then(|column| column.rows.get(1))
            .cloned()
            .expect("second typed CURRENT row");
        let before = app.state.file_manager.as_ref().expect("open FM").clone();
        app.state.mode = Mode::ContextMenu;

        for kind in [
            MouseEventKind::Down(MouseButton::Left),
            MouseEventKind::Down(MouseButton::Right),
            MouseEventKind::ScrollDown,
        ] {
            assert_eq!(
                app.handle_file_manager_mouse(mouse(kind, target.rect.x, target.rect.y)),
                FileManagerMouseDispatch::NotHandled
            );
        }

        let after = app.state.file_manager.as_ref().expect("open FM");
        assert_eq!(after.cwd, before.cwd);
        assert_eq!(after.entries, before.entries);
        assert_eq!(after.cursor, before.cursor);
        assert_eq!(after.viewport_start, before.viewport_start);
        assert_eq!(after.preview_viewport_start, before.preview_viewport_start);
        assert_eq!(
            after.multi_selection_paths(),
            before.multi_selection_paths()
        );
        assert_eq!(after.parent, before.parent);
        assert_eq!(after.preview, before.preview);
        assert_eq!(after.directory_generation, before.directory_generation);
        assert_eq!(after.preview_generation, before.preview_generation);
        assert_eq!(after.miller, before.miller);
        assert!(app.state.context_menu.is_none());
    }

    // TP-FM3-ALL-COLUMN-PLAIN: one plain click in each actionable visible
    // column focuses the exact live entry under that column. Non-current
    // columns first become the operational directory; a plain click never
    // enters the clicked child itself.
    #[test]
    fn plain_click_focuses_exact_live_row_in_every_visible_column() {
        let td = TempDir::new("typed-all-column-click");
        let a = td.root.join("a");
        let b = a.join("b");
        let current = b.join("current");
        let preview_directory = current.join("preview-directory");
        fs::create_dir_all(&preview_directory).expect("create deep FM fixture");
        fs::write(preview_directory.join("child.txt"), b"x").expect("write preview child");
        fs::write(current.join("peer.txt"), b"x").expect("write current peer");

        let mut file_manager = FmState::new(&td.root);
        for expected in [&a, &b, &current] {
            let entry_index = file_manager
                .entries
                .iter()
                .position(|entry| &entry.path == expected)
                .expect("next directory row");
            assert!(file_manager.select(entry_index));
            file_manager.enter();
        }
        assert_eq!(file_manager.cwd, current);
        assert_eq!(
            file_manager.selected().map(|entry| &entry.path),
            Some(&preview_directory)
        );

        let frame = Rect::new(0, 0, 200, 18);
        let mut template = runtime_app_with_fm(file_manager.clone());
        install_focused_agent(&mut template);
        template.state.mobile_width_threshold = 0;
        template.state.sidebar_collapsed = true;
        compute_view(&mut template.state, frame);
        let targets = template
            .state
            .view
            .file_manager_miller
            .columns
            .iter()
            .flat_map(|column| column.rows.first())
            .cloned()
            .collect::<Vec<_>>();
        assert!(
            targets.len() >= 4,
            "fixture must expose resident/current/preview rows"
        );
        assert!(targets
            .iter()
            .any(|row| row.column_kind == crate::ui::MillerRowColumnKind::Current));
        assert!(targets
            .iter()
            .any(|row| row.column_kind == crate::ui::MillerRowColumnKind::Preview));
        assert!(targets
            .iter()
            .any(|row| { row.column_kind == crate::ui::MillerRowColumnKind::ResidentDirectory }));

        for target in targets {
            let mut app = runtime_app_with_fm(file_manager.clone());
            install_focused_agent(&mut app);
            app.state.mobile_width_threshold = 0;
            app.state.sidebar_collapsed = true;
            compute_view(&mut app.state, frame);

            assert_eq!(
                app.handle_file_manager_mouse(mouse(
                    MouseEventKind::Down(MouseButton::Left),
                    target.rect.x,
                    target.rect.y,
                )),
                FileManagerMouseDispatch::Consumed
            );

            let actual = app.state.file_manager.as_ref().expect("open FM");
            assert_eq!(
                actual.cwd.as_path(),
                target.directory_path.as_path(),
                "plain click first activates the row's owning directory"
            );
            assert_eq!(
                actual.selected().map(|entry| &entry.path),
                Some(&target.entry_path),
                "plain click focuses the exact live target path"
            );
        }
    }

    // TP-FM3-TOCTOU: the prepared snapshot can remain internally fresh while
    // the filesystem changes before dispatch. The second exact directory read
    // rejects the renamed path atomically and the stale event is not replayed.
    #[test]
    fn renamed_non_current_target_is_consumed_without_model_mutation() {
        let td = TempDir::new("typed-row-rename-race");
        let current = td.root.join("current");
        let preview_directory = current.join("preview-directory");
        let old_path = preview_directory.join("old-name.txt");
        let new_path = preview_directory.join("new-name.txt");
        fs::create_dir_all(&preview_directory).expect("create preview directory");
        fs::write(&old_path, b"x").expect("write preview target");

        let mut file_manager = FmState::new(&current);
        let preview_index = file_manager
            .entries
            .iter()
            .position(|entry| entry.path == preview_directory)
            .expect("preview directory row");
        assert!(file_manager.select(preview_index));
        let mut app = runtime_app_with_fm(file_manager);
        install_focused_agent(&mut app);
        app.state.mobile_width_threshold = 0;
        app.state.sidebar_collapsed = true;
        compute_view(&mut app.state, Rect::new(0, 0, 100, 16));
        let target = app
            .state
            .view
            .file_manager_miller
            .columns
            .iter()
            .flat_map(|column| &column.rows)
            .find(|row| {
                row.column_kind == crate::ui::MillerRowColumnKind::Preview
                    && row.entry_path == old_path
            })
            .cloned()
            .expect("prepared preview row");
        let before = app.state.file_manager.as_ref().expect("open FM").clone();

        fs::rename(&old_path, &new_path).expect("rename after projection");
        assert_eq!(
            app.handle_file_manager_mouse(mouse(
                MouseEventKind::Down(MouseButton::Left),
                target.rect.x,
                target.rect.y,
            )),
            FileManagerMouseDispatch::Consumed
        );

        let after = app.state.file_manager.as_ref().expect("open FM");
        assert_eq!(after.cwd, before.cwd);
        assert_eq!(after.entries, before.entries);
        assert_eq!(after.cursor, before.cursor);
        assert_eq!(after.viewport_start, before.viewport_start);
        assert_eq!(
            after.multi_selection_paths(),
            before.multi_selection_paths()
        );
        assert_eq!(after.parent, before.parent);
        assert_eq!(after.preview, before.preview);
        assert_eq!(after.directory_generation, before.directory_generation);
        assert_eq!(after.preview_generation, before.preview_generation);
        assert_eq!(after.miller, before.miller);
    }

    // TP-FM3-NONCURRENT-CONTEXT: a live right-click in a non-current column
    // revalidates and activates its owning directory, then opens the existing
    // typed file menu for the exact new CURRENT selection.
    #[test]
    fn right_click_live_non_current_row_opens_exact_context_menu() {
        let td = TempDir::new("typed-noncurrent-context");
        let current = td.root.join("current");
        let preview_directory = current.join("preview-directory");
        let target_path = preview_directory.join("target.txt");
        fs::create_dir_all(&preview_directory).expect("create preview directory");
        fs::write(&target_path, b"x").expect("write context target");

        let mut file_manager = FmState::new(&current);
        let preview_index = file_manager
            .entries
            .iter()
            .position(|entry| entry.path == preview_directory)
            .expect("preview directory row");
        assert!(file_manager.select(preview_index));
        let mut app = runtime_app_with_fm(file_manager);
        install_focused_agent(&mut app);
        app.state.mobile_width_threshold = 0;
        app.state.sidebar_collapsed = true;
        compute_view(&mut app.state, Rect::new(0, 0, 100, 16));
        let target = app
            .state
            .view
            .file_manager_miller
            .columns
            .iter()
            .flat_map(|column| &column.rows)
            .find(|row| {
                row.column_kind == crate::ui::MillerRowColumnKind::Preview
                    && row.entry_path == target_path
            })
            .cloned()
            .expect("live preview target");

        assert_eq!(
            app.handle_file_manager_mouse(mouse(
                MouseEventKind::Down(MouseButton::Right),
                target.rect.x,
                target.rect.y,
            )),
            FileManagerMouseDispatch::Consumed
        );

        let file_manager = app.state.file_manager.as_ref().expect("open FM");
        assert_eq!(file_manager.cwd, preview_directory);
        assert_eq!(
            file_manager.selected().map(|entry| &entry.path),
            Some(&target_path)
        );
        assert_eq!(app.state.mode, Mode::ContextMenu);
        let ContextMenuKind::File { model } = &app
            .state
            .context_menu
            .as_ref()
            .expect("typed file menu")
            .kind
        else {
            panic!("expected typed file menu");
        };
        assert_eq!(model.paths, vec![target_path]);
    }

    // TP-FM3-CROSS-COLUMN-DOUBLE: a first click may move a preview/resident
    // column into CURRENT. After the required frame recompute, a second click
    // on the same stable entry path preserves the existing directory-enter
    // double-click semantics across that column transition.
    #[test]
    fn double_click_non_current_directory_revalidates_then_enters() {
        let td = TempDir::new("typed-noncurrent-double");
        let current = td.root.join("current");
        let preview_directory = current.join("preview-directory");
        let child_directory = preview_directory.join("child-directory");
        fs::create_dir_all(&child_directory).expect("create nested directory");
        fs::write(child_directory.join("inside.txt"), b"x").expect("write nested fixture");

        let mut file_manager = FmState::new(&current);
        let preview_index = file_manager
            .entries
            .iter()
            .position(|entry| entry.path == preview_directory)
            .expect("preview directory row");
        assert!(file_manager.select(preview_index));
        let mut app = runtime_app_with_fm(file_manager);
        install_focused_agent(&mut app);
        app.state.mobile_width_threshold = 0;
        app.state.sidebar_collapsed = true;
        let frame = Rect::new(0, 0, 100, 16);
        compute_view(&mut app.state, frame);
        let preview_target = app
            .state
            .view
            .file_manager_miller
            .columns
            .iter()
            .flat_map(|column| &column.rows)
            .find(|row| {
                row.column_kind == crate::ui::MillerRowColumnKind::Preview
                    && row.entry_path == child_directory
            })
            .cloned()
            .expect("preview directory target");

        app.handle_file_manager_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            preview_target.rect.x,
            preview_target.rect.y,
        ));
        assert_eq!(
            app.state.file_manager.as_ref().expect("open FM").cwd,
            preview_directory
        );
        compute_view(&mut app.state, frame);
        let current_target = app
            .state
            .view
            .file_manager_miller
            .columns
            .iter()
            .find(|column| column.kind.is_current())
            .and_then(|column| {
                column
                    .rows
                    .iter()
                    .find(|row| row.entry_path == child_directory)
            })
            .cloned()
            .expect("same path moved into CURRENT");

        app.handle_file_manager_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            current_target.rect.x,
            current_target.rect.y,
        ));

        assert_eq!(
            app.state.file_manager.as_ref().expect("open FM").cwd,
            child_directory,
            "second stable-path click enters only after fresh revalidation"
        );
    }

    // TP-A3.3-DISPATCH: the second unmodified press on the same directory row
    // inside the double-click window selects then enters that directory.
    #[test]
    fn directory_double_click_enters_selected_directory() {
        let td = TempDir::new("mouse-double-directory");
        td.dir("alpha-dir");
        fs::write(td.root.join("alpha-dir").join("child.txt"), b"x").expect("write child fixture");
        let mut app = runtime_app_with_fm(FmState::new(&td.root));
        let click = mouse(MouseEventKind::Down(MouseButton::Left), 27, 2);

        app.handle_mouse(click);
        app.handle_mouse(click);

        let fm = app.state.file_manager.as_ref().expect("file manager open");
        assert_eq!(fm.cwd, td.root.join("alpha-dir"));
        assert_eq!(fm.cursor, 0);
    }

    // TP-A3.3-DISPATCH: double-clicking a file keeps it selected and never
    // changes cwd; file opening belongs to a later product module.
    #[test]
    fn file_double_click_stays_selected_without_entering() {
        let td = TempDir::new("mouse-double-file");
        td.dir("alpha-dir");
        td.file("beta.txt");
        let mut app = runtime_app_with_fm(FmState::new(&td.root));
        let click = mouse(MouseEventKind::Down(MouseButton::Left), 27, 3);

        app.handle_mouse(click);
        app.handle_mouse(click);

        let fm = app.state.file_manager.as_ref().expect("file manager open");
        assert_eq!(fm.cwd, td.root);
        assert_eq!(fm.cursor, 1);
        assert_eq!(
            fm.selected().map(|entry| entry.name.as_str()),
            Some("beta.txt")
        );
    }

    // TP-A3.3-DISPATCH: rapid clicks on different absolute entries are two
    // selections, not a directory activation gesture.
    #[test]
    fn rapid_clicks_on_different_rows_do_not_activate_directory() {
        let td = TempDir::new("mouse-different-rows");
        td.dir("alpha-dir");
        td.dir("beta-dir");
        let mut app = runtime_app_with_fm(FmState::new(&td.root));

        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 27, 2));
        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 27, 3));

        let fm = app.state.file_manager.as_ref().expect("file manager open");
        assert_eq!(fm.cwd, td.root);
        assert_eq!(fm.cursor, 1);
        assert_eq!(
            fm.selected().map(|entry| entry.name.as_str()),
            Some("beta-dir")
        );
    }

    // TP-A3.3-DISPATCH: wheel input over CURRENT moves one bounded row per
    // event. The FM header is not a list target and leaves cursor unchanged.
    #[test]
    fn wheel_moves_cursor_within_bounds_only_over_current_rows() {
        let td = TempDir::new("mouse-wheel");
        for index in 0..6 {
            td.file(&format!("{index:02}.txt"));
        }
        let mut app = runtime_app_with_fm(FmState::new(&td.root));

        app.handle_mouse(mouse(MouseEventKind::ScrollUp, 27, 2));
        assert_eq!(app.state.file_manager.as_ref().expect("open fm").cursor, 0);

        for _ in 0..20 {
            app.handle_mouse(mouse(MouseEventKind::ScrollDown, 27, 3));
        }
        assert_eq!(app.state.file_manager.as_ref().expect("open fm").cursor, 5);

        app.handle_mouse(mouse(MouseEventKind::ScrollDown, 27, 0));
        assert_eq!(app.state.file_manager.as_ref().expect("open fm").cursor, 5);

        for _ in 0..20 {
            app.handle_mouse(mouse(MouseEventKind::ScrollUp, 27, 2));
        }
        assert_eq!(app.state.file_manager.as_ref().expect("open fm").cursor, 0);
    }

    // TP-FM3-RESIDENT-WHEEL: plain wheel over a resident ancestor advances
    // only that owning segment's bounded cursor/viewport. CURRENT selection,
    // horizontal window, generations, and other segments remain unchanged.
    #[test]
    fn plain_wheel_moves_only_hovered_resident_column_viewport() {
        let td = TempDir::new("resident-wheel");
        td.file("current-a.txt");
        td.file("current-b.txt");
        let current = td.root.clone();
        let resident = PathBuf::from("/virtual/resident-wheel");
        let mut file_manager = FmState::new(&current);
        file_manager.miller.chain = [resident.clone(), current.clone()]
            .into_iter()
            .map(crate::fm::miller::MillerPathSegment::new)
            .collect();
        file_manager.miller.focused_directory = current;
        file_manager.miller.resident_non_current.push_back(
            crate::fm::miller::MillerDirectoryProjection {
                id: crate::fm::miller::MillerColumnId {
                    directory: resident.clone(),
                    generation: 77,
                },
                entries: (0..12)
                    .map(|index| crate::fm::FileEntry {
                        name: format!("{index:02}.txt"),
                        path: resident.join(format!("{index:02}.txt")),
                        is_dir: false,
                        operation_supported: true,
                    })
                    .collect(),
                status: crate::fm::FmDirectoryStatus::Available,
                writable: true,
            },
        );
        let mut app = runtime_app_with_fm(file_manager);
        install_focused_agent(&mut app);
        app.state.mobile_width_threshold = 0;
        app.state.sidebar_collapsed = true;
        compute_view(&mut app.state, Rect::new(0, 0, 200, 8));
        let resident_column = app
            .state
            .view
            .file_manager_miller
            .columns
            .iter()
            .find(|column| {
                column.rows.first().is_some_and(|row| {
                    row.column_kind == crate::ui::MillerRowColumnKind::ResidentDirectory
                })
            })
            .cloned()
            .expect("visible resident column");
        let probe = resident_column.rows[0].rect;
        let visible_rows = resident_column.content_rect.height as usize;
        let before = app.state.file_manager.as_ref().expect("open FM").clone();

        let wheel_events = visible_rows.saturating_add(2);
        for _ in 0..wheel_events {
            assert_eq!(
                app.handle_file_manager_mouse(mouse(MouseEventKind::ScrollDown, probe.x, probe.y,)),
                FileManagerMouseDispatch::Consumed
            );
        }

        let after = app.state.file_manager.as_ref().expect("open FM");
        let resident_segment = &after.miller.chain[0];
        let expected_cursor = wheel_events.min(11);
        let expected_viewport = expected_cursor
            .saturating_add(1)
            .saturating_sub(visible_rows);
        assert_eq!(resident_segment.cursor, expected_cursor);
        assert_eq!(resident_segment.viewport_start, expected_viewport);
        assert_eq!(after.cursor, before.cursor);
        assert_eq!(after.viewport_start, before.viewport_start);
        assert_eq!(
            after.multi_selection_paths(),
            before.multi_selection_paths()
        );
        assert_eq!(
            after.miller.horizontal, before.miller.horizontal,
            "vertical column wheel cannot pan the horizontal window"
        );
        assert_eq!(after.miller.chain[1], before.miller.chain[1]);
        assert_eq!(after.directory_generation, before.directory_generation);
        assert_eq!(after.preview_generation, before.preview_generation);
        assert_eq!(after.miller.revision, before.miller.revision);
    }

    // TP-FM3-PREVIEW-WHEEL: PREVIEW has no chain segment, so it owns one
    // bounded client-local viewport. Plain wheel scrolls only that directory
    // preview; CURRENT focus, horizontal origin, chain, and generations stay
    // byte-for-byte stable.
    #[test]
    fn plain_wheel_moves_only_hovered_preview_viewport() {
        let td = TempDir::new("preview-wheel");
        let preview_directory = td.root.join("preview-directory");
        fs::create_dir_all(&preview_directory).expect("create preview directory");
        for index in 0..12 {
            fs::write(preview_directory.join(format!("{index:02}.txt")), b"x")
                .expect("write preview entry");
        }
        let mut file_manager = FmState::new(&td.root);
        let preview_index = file_manager
            .entries
            .iter()
            .position(|entry| entry.path == preview_directory)
            .expect("preview directory row");
        assert!(file_manager.select(preview_index));
        let mut app = runtime_app_with_fm(file_manager);
        install_focused_agent(&mut app);
        app.state.mobile_width_threshold = 0;
        app.state.sidebar_collapsed = true;
        let frame = Rect::new(0, 0, 100, 8);
        compute_view(&mut app.state, frame);
        let preview_column = app
            .state
            .view
            .file_manager_miller
            .columns
            .iter()
            .find(|column| column.kind.is_preview())
            .cloned()
            .expect("visible preview column");
        let probe = preview_column.rows[0].rect;
        let before = app.state.file_manager.as_ref().expect("open FM").clone();

        for _ in 0..3 {
            app.handle_file_manager_mouse(mouse(MouseEventKind::ScrollDown, probe.x, probe.y));
        }
        compute_view(&mut app.state, frame);

        let first_preview_entry = app
            .state
            .view
            .file_manager_miller
            .columns
            .iter()
            .find(|column| column.kind.is_preview())
            .and_then(|column| column.rows.first())
            .map(|row| row.entry_index);
        assert_eq!(
            first_preview_entry,
            Some(3),
            "three wheel steps advance only the preview viewport"
        );
        let after = app.state.file_manager.as_ref().expect("open FM");
        assert_eq!(after.preview_viewport_start, 3);
        assert_eq!(after.cwd, before.cwd);
        assert_eq!(after.cursor, before.cursor);
        assert_eq!(after.viewport_start, before.viewport_start);
        assert_eq!(
            after.multi_selection_paths(),
            before.multi_selection_paths()
        );
        assert_eq!(after.miller, before.miller);
        assert_eq!(after.directory_generation, before.directory_generation);
        assert_eq!(after.preview_generation, before.preview_generation);
    }

    // TP-FM3-PARENT-WHEEL: a prepared immediate-parent column uses the same
    // bounded owning-column semantics as a resident ancestor. It may move its
    // local highlight/viewport but never CURRENT or horizontal state.
    #[test]
    fn plain_wheel_moves_only_hovered_prepared_parent_viewport() {
        let td = TempDir::new("prepared-parent-wheel");
        for index in 0..10 {
            td.dir(&format!("{index:02}-sibling"));
        }
        let current = td.root.join("zz-current");
        fs::create_dir(&current).expect("create current directory");
        let file_manager = FmState::new(&current);
        let mut app = runtime_app_with_fm(file_manager);
        install_focused_agent(&mut app);
        app.state.mobile_width_threshold = 0;
        app.state.sidebar_collapsed = true;
        compute_view(&mut app.state, Rect::new(0, 0, 200, 8));
        let parent_column = app
            .state
            .view
            .file_manager_miller
            .columns
            .iter()
            .find(|column| {
                column.rows.first().is_some_and(|row| {
                    row.column_kind == crate::ui::MillerRowColumnKind::PreparedParent
                })
            })
            .cloned()
            .expect("visible prepared parent");
        let target = parent_column.rows[0].clone();
        let visible_rows = parent_column.content_rect.height as usize;
        let before = app.state.file_manager.as_ref().expect("open FM").clone();
        let before_parent_cursor = before
            .parent
            .as_ref()
            .and_then(|parent| parent.cursor)
            .expect("current row in prepared parent");

        for _ in 0..3 {
            app.handle_file_manager_mouse(mouse(
                MouseEventKind::ScrollUp,
                target.rect.x,
                target.rect.y,
            ));
        }

        let after = app.state.file_manager.as_ref().expect("open FM");
        let expected_cursor = before_parent_cursor.saturating_sub(3);
        assert_eq!(
            after.parent.as_ref().and_then(|parent| parent.cursor),
            Some(expected_cursor)
        );
        let segment = &after.miller.chain[target.chain_index.expect("parent chain index")];
        assert_eq!(segment.cursor, expected_cursor);
        assert_eq!(
            segment.viewport_start,
            before_parent_cursor.saturating_sub(visible_rows),
            "first upward step brings the offscreen parent cursor into view; later steps remain within that window"
        );
        assert_eq!(after.cwd, before.cwd);
        assert_eq!(after.cursor, before.cursor);
        assert_eq!(after.viewport_start, before.viewport_start);
        assert_eq!(after.miller.horizontal, before.miller.horizontal);
        assert_eq!(after.directory_generation, before.directory_generation);
        assert_eq!(after.preview_generation, before.preview_generation);
    }

    // TP-FM3-NONCURRENT-MODIFIERS: Ctrl/Shift selection authority is confined
    // to CURRENT. A preview/ancestor target with either modifier is consumed
    // without activating a directory or creating cross-directory selection.
    #[test]
    fn modified_click_outside_current_directory_is_consumed_inert() {
        let td = TempDir::new("noncurrent-modified-click");
        let preview_directory = td.root.join("preview-directory");
        let child = preview_directory.join("child.txt");
        fs::create_dir_all(&preview_directory).expect("create preview directory");
        fs::write(&child, b"x").expect("write preview child");
        let mut file_manager = FmState::new(&td.root);
        let preview_index = file_manager
            .entries
            .iter()
            .position(|entry| entry.path == preview_directory)
            .expect("preview directory row");
        assert!(file_manager.select(preview_index));
        let mut app = runtime_app_with_fm(file_manager);
        install_focused_agent(&mut app);
        app.state.mobile_width_threshold = 0;
        app.state.sidebar_collapsed = true;
        compute_view(&mut app.state, Rect::new(0, 0, 100, 12));
        let target = app
            .state
            .view
            .file_manager_miller
            .columns
            .iter()
            .flat_map(|column| &column.rows)
            .find(|row| row.column_kind == crate::ui::MillerRowColumnKind::Preview)
            .cloned()
            .expect("preview row target");
        let before = app.state.file_manager.as_ref().expect("open FM").clone();

        for modifiers in [KeyModifiers::CONTROL, KeyModifiers::SHIFT] {
            assert_eq!(
                app.handle_file_manager_mouse(mouse_with_modifiers(
                    MouseEventKind::Down(MouseButton::Left),
                    target.rect.x,
                    target.rect.y,
                    modifiers,
                )),
                FileManagerMouseDispatch::Consumed
            );
        }

        let after = app.state.file_manager.as_ref().expect("open FM");
        assert_eq!(after.cwd, before.cwd);
        assert_eq!(after.cursor, before.cursor);
        assert_eq!(
            after.multi_selection_paths(),
            before.multi_selection_paths()
        );
        assert_eq!(after.miller, before.miller);
        assert_eq!(after.directory_generation, before.directory_generation);
        assert_eq!(after.preview_generation, before.preview_generation);
    }

    // TP-FM3-STALE-CONTEXT: a right-click target can become stale on disk
    // after projection. The second revalidation must preserve model state and
    // must not open a destructive context overlay.
    #[test]
    fn renamed_non_current_right_click_does_not_open_context_menu() {
        let td = TempDir::new("noncurrent-context-rename");
        let preview_directory = td.root.join("preview-directory");
        let old_path = preview_directory.join("old.txt");
        let new_path = preview_directory.join("new.txt");
        fs::create_dir_all(&preview_directory).expect("create preview directory");
        fs::write(&old_path, b"x").expect("write preview target");
        let mut file_manager = FmState::new(&td.root);
        let preview_index = file_manager
            .entries
            .iter()
            .position(|entry| entry.path == preview_directory)
            .expect("preview directory row");
        assert!(file_manager.select(preview_index));
        let mut app = runtime_app_with_fm(file_manager);
        install_focused_agent(&mut app);
        app.state.mobile_width_threshold = 0;
        app.state.sidebar_collapsed = true;
        compute_view(&mut app.state, Rect::new(0, 0, 100, 12));
        let target = app
            .state
            .view
            .file_manager_miller
            .columns
            .iter()
            .flat_map(|column| &column.rows)
            .find(|row| row.entry_path == old_path)
            .cloned()
            .expect("preview row target");
        let before = app.state.file_manager.as_ref().expect("open FM").clone();

        fs::rename(&old_path, &new_path).expect("rename after projection");
        assert_eq!(
            app.handle_file_manager_mouse(mouse(
                MouseEventKind::Down(MouseButton::Right),
                target.rect.x,
                target.rect.y,
            )),
            FileManagerMouseDispatch::Consumed
        );

        let after = app.state.file_manager.as_ref().expect("open FM");
        assert_eq!(app.state.mode, Mode::Terminal);
        assert!(app.state.context_menu.is_none());
        assert_eq!(after.cwd, before.cwd);
        assert_eq!(after.cursor, before.cursor);
        assert_eq!(
            after.multi_selection_paths(),
            before.multi_selection_paths()
        );
        assert_eq!(after.miller, before.miller);
        assert_eq!(after.directory_generation, before.directory_generation);
        assert_eq!(after.preview_generation, before.preview_generation);
    }

    // TP-FM1.3-HSCROLL-MODIFIERS: only the exact Shift+wheel gesture changes
    // the horizontal window. Control/Alt and combined modifiers are consumed
    // fail-closed and cannot accidentally become vertical list navigation.
    #[test]
    fn non_shift_modified_wheel_is_consumed_without_moving_any_axis() {
        let td = TempDir::new("miller-wheel-modifiers");
        for index in 0..3 {
            td.file(&format!("{index:02}.txt"));
        }
        let mut app = runtime_app_with_fm(FmState::new(&td.root));
        let before = app.state.file_manager.as_ref().expect("open FM").clone();

        for modifiers in [
            KeyModifiers::CONTROL,
            KeyModifiers::ALT,
            KeyModifiers::CONTROL | KeyModifiers::SHIFT,
        ] {
            assert_eq!(
                app.handle_file_manager_mouse(mouse_with_modifiers(
                    MouseEventKind::ScrollDown,
                    27,
                    3,
                    modifiers,
                )),
                FileManagerMouseDispatch::Consumed
            );
        }

        let after = app.state.file_manager.as_ref().expect("open FM");
        assert_eq!(after.cursor, before.cursor);
        assert_eq!(after.viewport_start, before.viewport_start);
        assert_eq!(
            after.miller.horizontal.first_visible,
            before.miller.horizontal.first_visible
        );
    }

    // TP-FM1.3-HSCROLL: native horizontal wheel events and Shift+wheel move
    // ONLY the bounded Miller window. Current/preview remain visible after
    // every recompute, while vertical cursor/viewport, entries, selection,
    // preview identity, and the structural Miller revision stay unchanged.
    #[test]
    fn horizontal_wheel_changes_only_miller_window_and_preserves_focus() {
        let td = TempDir::new("miller-horizontal-wheel");
        let mut current = td.root.clone();
        for level in 0..8 {
            current.push(format!("level-{level}"));
        }
        fs::create_dir_all(&current).expect("create deep Miller fixture");
        fs::write(current.join("00.txt"), b"x").expect("write selected fixture");

        let mut file_manager = FmState::new(&current);
        for segment in &mut file_manager.miller.chain {
            segment.preferred_width = crate::fm::miller::MILLER_COLUMN_MIN_WIDTH;
        }
        let mut app = runtime_app_with_fm(file_manager);
        install_focused_agent(&mut app);
        app.state.mobile_width_threshold = 0;
        app.state.sidebar_collapsed = true;
        let frame = Rect::new(0, 0, 144, 18);
        compute_view(&mut app.state, frame);

        let before = app.state.file_manager.as_ref().expect("open FM").clone();
        let first_visible = app.state.view.file_manager_miller.first_visible;
        let probe = app
            .state
            .view
            .file_manager_miller
            .columns
            .iter()
            .find(|column| column.kind.is_current())
            .map(|column| (column.content_rect.x, column.content_rect.y))
            .expect("current column probe");

        assert_eq!(
            app.handle_file_manager_mouse(mouse(MouseEventKind::ScrollRight, probe.0, probe.1,)),
            FileManagerMouseDispatch::Consumed
        );
        assert_eq!(
            app.state
                .file_manager
                .as_ref()
                .expect("open FM")
                .miller
                .horizontal
                .first_visible,
            first_visible + 1,
            "native ScrollRight advances the bounded horizontal origin"
        );
        compute_view(&mut app.state, frame);
        assert!(
            app.state
                .view
                .file_manager_miller
                .columns
                .iter()
                .any(|column| column.kind.is_current()),
            "current remains visible after horizontal scroll"
        );
        assert!(
            app.state
                .view
                .file_manager_miller
                .columns
                .iter()
                .any(|column| column.kind.is_preview()),
            "preview remains visible after horizontal scroll"
        );

        app.handle_file_manager_mouse(mouse_with_modifiers(
            MouseEventKind::ScrollUp,
            probe.0,
            probe.1,
            KeyModifiers::SHIFT,
        ));
        assert_eq!(
            app.state
                .file_manager
                .as_ref()
                .expect("open FM")
                .miller
                .horizontal
                .first_visible,
            first_visible,
            "Shift+ScrollUp maps to horizontal left"
        );
        compute_view(&mut app.state, frame);

        app.handle_file_manager_mouse(mouse_with_modifiers(
            MouseEventKind::ScrollDown,
            probe.0,
            probe.1,
            KeyModifiers::SHIFT,
        ));
        assert_eq!(
            app.state
                .file_manager
                .as_ref()
                .expect("open FM")
                .miller
                .horizontal
                .first_visible,
            first_visible + 1,
            "Shift+ScrollDown maps to horizontal right"
        );
        app.handle_file_manager_mouse(mouse(MouseEventKind::ScrollLeft, probe.0, probe.1));
        assert_eq!(
            app.state
                .file_manager
                .as_ref()
                .expect("open FM")
                .miller
                .horizontal
                .first_visible,
            first_visible,
            "native ScrollLeft returns to the bounded origin"
        );

        for _ in 0..64 {
            app.handle_file_manager_mouse(mouse(MouseEventKind::ScrollLeft, probe.0, probe.1));
        }
        assert_eq!(
            app.state
                .file_manager
                .as_ref()
                .expect("open FM")
                .miller
                .horizontal
                .first_visible,
            first_visible,
            "left input clamps at the fullest focused window"
        );

        for _ in 0..64 {
            app.handle_file_manager_mouse(mouse(MouseEventKind::ScrollRight, probe.0, probe.1));
        }
        let focused_chain_index = app
            .state
            .view
            .file_manager_miller
            .focused_chain_index
            .expect("focused chain identity");
        assert_eq!(
            app.state
                .file_manager
                .as_ref()
                .expect("open FM")
                .miller
                .horizontal
                .first_visible,
            focused_chain_index,
            "right input clamps before it can hide current"
        );
        compute_view(&mut app.state, frame);
        assert!(
            app.state
                .view
                .file_manager_miller
                .columns
                .iter()
                .any(|column| column.kind.is_current()),
            "current remains visible at the right clamp"
        );
        assert!(
            app.state
                .view
                .file_manager_miller
                .columns
                .iter()
                .any(|column| column.kind.is_preview()),
            "preview remains visible at the right clamp"
        );

        let after = app.state.file_manager.as_ref().expect("open FM");
        assert_eq!(after.cursor, before.cursor);
        assert_eq!(after.viewport_start, before.viewport_start);
        assert_eq!(after.entries, before.entries);
        assert_eq!(
            after.multi_selection_paths(),
            before.multi_selection_paths()
        );
        assert_eq!(after.preview_generation, before.preview_generation);
        assert_eq!(after.preview, before.preview);
        assert_eq!(after.miller.chain, before.miller.chain);
        assert_eq!(after.miller.revision, before.miller.revision);
    }

    // TP-FM1.3-HSCROLL-AUTHORITY: horizontal input consumes only a fresh
    // active-Files snapshot inside the Files terrain. Stale model/generation
    // identity is inert, overlays retain priority, and outside coordinates
    // remain available to the outer shell router.
    #[test]
    fn horizontal_wheel_fails_closed_without_fresh_files_authority() {
        let td = TempDir::new("miller-horizontal-authority");
        let mut current = td.root.clone();
        for level in 0..8 {
            current.push(format!("level-{level}"));
        }
        fs::create_dir_all(&current).expect("create deep Miller fixture");
        fs::write(current.join("00.txt"), b"x").expect("write selected fixture");

        let mut file_manager = FmState::new(&current);
        for segment in &mut file_manager.miller.chain {
            segment.preferred_width = crate::fm::miller::MILLER_COLUMN_MIN_WIDTH;
        }
        let mut app = runtime_app_with_fm(file_manager);
        install_focused_agent(&mut app);
        app.state.mobile_width_threshold = 0;
        app.state.sidebar_collapsed = true;
        compute_view(&mut app.state, Rect::new(0, 0, 144, 18));

        let center = app.state.view.terminal_area;
        let probe = (center.x, center.y.saturating_add(2));
        let first_visible = app
            .state
            .file_manager
            .as_ref()
            .expect("open FM")
            .miller
            .horizontal
            .first_visible;

        app.state.view.file_manager_miller.model_revision = app
            .state
            .view
            .file_manager_miller
            .model_revision
            .saturating_add(1);
        assert_eq!(
            app.handle_file_manager_mouse(mouse(MouseEventKind::ScrollRight, probe.0, probe.1,)),
            FileManagerMouseDispatch::Consumed
        );
        assert_eq!(
            app.state
                .file_manager
                .as_ref()
                .expect("open FM")
                .miller
                .horizontal
                .first_visible,
            first_visible,
            "stale structural revision cannot move the window"
        );

        compute_view(&mut app.state, Rect::new(0, 0, 144, 18));
        app.state.view.file_manager_miller.files_generation = app
            .state
            .view
            .file_manager_miller
            .files_generation
            .map(|generation| generation.saturating_add(1));
        app.handle_file_manager_mouse(mouse(MouseEventKind::ScrollRight, probe.0, probe.1));
        assert_eq!(
            app.state
                .file_manager
                .as_ref()
                .expect("open FM")
                .miller
                .horizontal
                .first_visible,
            first_visible,
            "wrong Files generation cannot move the window"
        );

        compute_view(&mut app.state, Rect::new(0, 0, 144, 18));
        let previous_mode = app.state.mode;
        app.state.mode = Mode::ContextMenu;
        assert_eq!(
            app.handle_file_manager_mouse(mouse(MouseEventKind::ScrollRight, probe.0, probe.1,)),
            FileManagerMouseDispatch::NotHandled,
            "the topmost overlay retains routing priority"
        );
        app.state.mode = previous_mode;
        assert_eq!(
            app.handle_file_manager_mouse(mouse(
                MouseEventKind::ScrollRight,
                center.right(),
                probe.1,
            )),
            FileManagerMouseDispatch::NotHandled,
            "outside Files terrain remains owned by the outer shell"
        );
        assert_eq!(
            app.state
                .file_manager
                .as_ref()
                .expect("open FM")
                .miller
                .horizontal
                .first_visible,
            first_visible
        );

        assert_eq!(
            app.handle_file_manager_mouse(mouse(MouseEventKind::ScrollRight, probe.0, probe.1,)),
            FileManagerMouseDispatch::Consumed,
            "control: a fresh in-bounds Files snapshot consumes the event"
        );
        assert_eq!(
            app.state
                .file_manager
                .as_ref()
                .expect("open FM")
                .miller
                .horizontal
                .first_visible,
            first_visible + 1,
            "control: the same fresh snapshot can actually move"
        );
    }

    // TP-A3.3-DISPATCH-STALE: a row snapshot can outlive a watcher reload for
    // one frame. An invalid absolute index is consumed but must not clamp to or
    // activate an unrelated live entry.
    #[test]
    fn stale_row_index_is_consumed_without_selecting_another_entry() {
        let td = TempDir::new("mouse-stale-row");
        td.dir("alpha-dir");
        td.file("beta.txt");
        let mut app = runtime_app_with_fm(FmState::new(&td.root));
        app.state.view.file_manager_row_areas[0].entry_idx = usize::MAX;

        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 27, 2));

        let fm = app.state.file_manager.as_ref().expect("file manager open");
        assert_eq!(fm.cwd, td.root);
        assert_eq!(fm.cursor, 0);
        assert_eq!(
            fm.selected().map(|entry| entry.name.as_str()),
            Some("alpha-dir")
        );
    }

    // TP-N4.1-SELECTION-STATE: plain mouse selection establishes one explicit
    // path, normal keyboard navigation moves only cursor focus, and reopen
    // drops the overlay-local selection.
    #[test]
    fn plain_selection_and_cursor_focus_follow_close_reopen_lifecycle() {
        let td = TempDir::new("selection-focus-reopen");
        for index in 0..3 {
            td.file(&format!("{index:02}.txt"));
        }
        let mut app = runtime_app_with_fm(FmState::new(&td.root));
        let selected_path = td.root.join("02.txt");

        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 27, 4));
        let fm = app.state.file_manager.as_ref().expect("open fm");
        assert_eq!(fm.cursor, 2);
        assert_eq!(
            fm.multi_selection_paths().iter().collect::<Vec<_>>(),
            vec![&selected_path]
        );
        assert_eq!(fm.multi_selection_anchor(), Some(selected_path.as_path()));

        handle_file_manager_key(&mut app.state, key(KeyCode::Up));
        let fm = app.state.file_manager.as_ref().expect("open fm");
        assert_eq!(fm.cursor, 1);
        assert_eq!(
            fm.multi_selection_paths().iter().collect::<Vec<_>>(),
            vec![&selected_path]
        );

        handle_file_manager_key(&mut app.state, key(KeyCode::Esc));
        assert!(app.state.file_manager.is_none());
        app.state
            .try_open_file_manager_with(|_| Some(FmState::new(&td.root)))
            .expect("Files activation");
        let fm = app.state.file_manager.as_ref().expect("reopened fm");
        assert_eq!(fm.cursor, 0);
        assert!(fm.multi_selection_paths().is_empty());
        assert!(fm.multi_selection_anchor().is_none());
    }

    // TP-N4.1-SELECTION-STATE: exact mouse modifiers share the pure model
    // semantics; combined modifiers fail closed without changing the set.
    #[test]
    fn mouse_plain_control_shift_and_combined_gestures_are_exact() {
        let td = TempDir::new("multi-selection-mouse-gestures");
        for index in 0..4 {
            td.file(&format!("{index:02}.txt"));
        }
        let mut app = runtime_app_with_fm(FmState::new(&td.root));
        let path = |index| td.root.join(format!("{index:02}.txt"));

        app.handle_file_manager_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 27, 3));
        let fm = app.state.file_manager.as_ref().expect("open fm");
        assert_eq!(fm.multi_selection_paths().len(), 1);
        assert!(fm.multi_selection_paths().contains(&path(1)));

        app.handle_file_manager_mouse(mouse_with_modifiers(
            MouseEventKind::Down(MouseButton::Left),
            27,
            5,
            KeyModifiers::CONTROL,
        ));
        let fm = app.state.file_manager.as_ref().expect("open fm");
        assert_eq!(fm.cursor, 3);
        assert_eq!(fm.multi_selection_paths().len(), 2);
        assert!(fm.multi_selection_paths().contains(&path(1)));
        assert!(fm.multi_selection_paths().contains(&path(3)));
        assert_eq!(fm.multi_selection_anchor(), Some(path(3).as_path()));

        app.handle_file_manager_mouse(mouse_with_modifiers(
            MouseEventKind::Down(MouseButton::Left),
            27,
            4,
            KeyModifiers::SHIFT,
        ));
        let fm = app.state.file_manager.as_ref().expect("open fm");
        assert_eq!(fm.cursor, 2);
        assert_eq!(fm.multi_selection_paths().len(), 2);
        assert!(fm.multi_selection_paths().contains(&path(2)));
        assert!(fm.multi_selection_paths().contains(&path(3)));
        assert!(!fm.multi_selection_paths().contains(&path(1)));
        assert_eq!(fm.multi_selection_anchor(), Some(path(3).as_path()));

        let before_paths = fm.multi_selection_paths().clone();
        let before_anchor = fm.multi_selection_anchor().map(PathBuf::from);
        app.handle_file_manager_mouse(mouse_with_modifiers(
            MouseEventKind::Down(MouseButton::Left),
            27,
            2,
            KeyModifiers::CONTROL | KeyModifiers::SHIFT,
        ));
        let fm = app.state.file_manager.as_ref().expect("open fm");
        assert_eq!(fm.cursor, 2);
        assert_eq!(fm.multi_selection_paths(), &before_paths);
        assert_eq!(fm.multi_selection_anchor(), before_anchor.as_deref());
    }

    // TP-N4.1-SELECTION-STATE: Space toggles the focused identity, Shift plus
    // vertical navigation extends from the stable anchor, and plain movement
    // does not rewrite the explicit set.
    #[test]
    fn keyboard_toggle_range_and_cursor_only_movement_share_selection_model() {
        let td = TempDir::new("multi-selection-keyboard-gestures");
        for index in 0..4 {
            td.file(&format!("{index:02}.txt"));
        }
        let mut app = app_with_fm(FmState::new(&td.root));
        let path = |index| td.root.join(format!("{index:02}.txt"));

        handle_file_manager_key(&mut app, key(KeyCode::Char(' ')));
        handle_file_manager_key(
            &mut app,
            key_with_modifiers(KeyCode::Down, KeyModifiers::SHIFT),
        );
        handle_file_manager_key(
            &mut app,
            key_with_modifiers(KeyCode::Down, KeyModifiers::SHIFT),
        );
        let fm = app.file_manager.as_ref().expect("open fm");
        assert_eq!(fm.cursor, 2);
        assert_eq!(fm.multi_selection_paths().len(), 3);
        assert_eq!(fm.multi_selection_anchor(), Some(path(0).as_path()));

        handle_file_manager_key(
            &mut app,
            key_with_modifiers(KeyCode::Up, KeyModifiers::SHIFT),
        );
        let fm = app.file_manager.as_ref().expect("open fm");
        assert_eq!(fm.cursor, 1);
        assert_eq!(fm.multi_selection_paths().len(), 2);
        assert!(fm.multi_selection_paths().contains(&path(0)));
        assert!(fm.multi_selection_paths().contains(&path(1)));

        handle_file_manager_key(&mut app, key(KeyCode::Down));
        let fm = app.file_manager.as_ref().expect("open fm");
        assert_eq!(fm.cursor, 2);
        assert_eq!(fm.multi_selection_paths().len(), 2);

        handle_file_manager_key(&mut app, key(KeyCode::Char(' ')));
        let fm = app.file_manager.as_ref().expect("open fm");
        assert_eq!(fm.multi_selection_paths().len(), 3);
        assert!(fm.multi_selection_paths().contains(&path(2)));
        assert_eq!(fm.multi_selection_anchor(), Some(path(2).as_path()));
    }

    // TP-N4.2-BULK-AUTHORITY: exact Ctrl+A/Ctrl+Shift+A gestures select all
    // and clear explicitly, refresh prepared authority, and reject extra mods.
    #[test]
    fn keyboard_select_all_and_clear_are_exact_and_refresh_bulk_authority() {
        let td = TempDir::new("multi-selection-keyboard-bulk");
        for index in 0..3 {
            td.file(&format!("{index:02}.txt"));
        }
        let mut app = app_with_fm(FmState::new(&td.root));

        handle_file_manager_key(
            &mut app,
            key_with_modifiers(KeyCode::Char('a'), KeyModifiers::CONTROL),
        );
        assert_eq!(
            app.file_manager
                .as_ref()
                .expect("open fm")
                .multi_selection_paths()
                .len(),
            3
        );
        compute_view(&mut app, Rect::new(0, 0, 100, 6));
        let selected_model = app
            .view
            .file_manager_action_bar
            .as_ref()
            .expect("selected action bar");
        assert_eq!(
            selected_model
                .selection
                .as_ref()
                .map(|selection| selection.label.as_str()),
            Some("3 selected")
        );

        handle_file_manager_key(
            &mut app,
            key_with_modifiers(
                KeyCode::Char('a'),
                KeyModifiers::CONTROL | KeyModifiers::SHIFT,
            ),
        );
        let fm = app.file_manager.as_ref().expect("open fm");
        assert!(fm.multi_selection_paths().is_empty());
        assert!(fm.multi_selection_anchor().is_none());
        compute_view(&mut app, Rect::new(0, 0, 100, 6));
        let cleared_model = app
            .view
            .file_manager_action_bar
            .as_ref()
            .expect("cleared action bar");
        assert!(cleared_model.selection.is_none());
        assert_eq!(
            cleared_model
                .action_state(FileManagerHeaderAction::Copy)
                .expect("copy state")
                .disabled_reason,
            Some(FileManagerActionDisabledReason::NoSelection)
        );

        assert!(app
            .file_manager
            .as_mut()
            .expect("open fm")
            .replace_selection(1));
        let before_paths = app
            .file_manager
            .as_ref()
            .expect("open fm")
            .multi_selection_paths()
            .clone();
        handle_file_manager_key(
            &mut app,
            key_with_modifiers(
                KeyCode::Char('a'),
                KeyModifiers::CONTROL | KeyModifiers::ALT,
            ),
        );
        assert_eq!(
            app.file_manager
                .as_ref()
                .expect("open fm")
                .multi_selection_paths(),
            &before_paths
        );

        let mut oversized = FmState::test_empty("/virtual");
        oversized.entries = (0..=MAX_MULTI_SELECTION_PATHS)
            .map(|index| crate::fm::FileEntry {
                name: format!("{index:05}.txt"),
                path: PathBuf::from(format!("/virtual/{index:05}.txt")),
                is_dir: false,
                operation_supported: true,
            })
            .collect();
        let mut oversized_app = app_with_fm(oversized);
        handle_file_manager_key(
            &mut oversized_app,
            key_with_modifiers(KeyCode::Char('a'), KeyModifiers::CONTROL),
        );
        assert!(oversized_app
            .file_manager
            .as_ref()
            .expect("open oversized fm")
            .multi_selection_paths()
            .is_empty());
    }

    // TP-N4.2-BULK-AUTHORITY: keyboard range growth uses the same atomic
    // ceiling as the state method; rejected growth cannot move focus alone.
    #[test]
    fn keyboard_range_overflow_preserves_cursor_paths_and_anchor() {
        let mut fm = FmState::test_empty("/virtual");
        fm.entries = (0..=MAX_MULTI_SELECTION_PATHS)
            .map(|index| crate::fm::FileEntry {
                name: format!("{index:05}.txt"),
                path: PathBuf::from(format!("/virtual/{index:05}.txt")),
                is_dir: false,
                operation_supported: true,
            })
            .collect();
        assert!(fm.replace_selection(0));
        assert!(fm.extend_selection(MAX_MULTI_SELECTION_PATHS - 1));
        let mut app = app_with_fm(fm);
        let before = app.file_manager.as_ref().expect("open fm");
        let before_cursor = before.cursor;
        let before_paths = before.multi_selection_paths().clone();
        let before_anchor = before.multi_selection_anchor().map(PathBuf::from);

        handle_file_manager_key(
            &mut app,
            key_with_modifiers(KeyCode::Down, KeyModifiers::SHIFT),
        );

        let fm = app.file_manager.as_ref().expect("open fm");
        assert_eq!(fm.cursor, before_cursor);
        assert_eq!(fm.multi_selection_paths(), &before_paths);
        assert_eq!(fm.multi_selection_anchor(), before_anchor.as_deref());
    }

    // TP-N4.1-SELECTION-STATE: a stale typed row target and unrecognized modifier
    // combinations are consumed without mutating cursor, paths, or anchor.
    #[test]
    fn stale_and_unrecognized_selection_gestures_fail_closed() {
        let td = TempDir::new("multi-selection-stale-gesture");
        td.file("00.txt");
        td.file("01.txt");
        let mut app = runtime_app_with_fm(FmState::new(&td.root));
        assert!(app
            .state
            .file_manager
            .as_mut()
            .expect("open fm")
            .replace_selection(0));
        app.state
            .view
            .file_manager_miller
            .columns
            .iter_mut()
            .flat_map(|column| &mut column.rows)
            .find(|row| row.entry_index == 1)
            .expect("second typed current row")
            .entry_index = usize::MAX;
        let before_paths = app
            .state
            .file_manager
            .as_ref()
            .expect("open fm")
            .multi_selection_paths()
            .clone();

        assert_eq!(
            app.handle_file_manager_mouse(mouse_with_modifiers(
                MouseEventKind::Down(MouseButton::Left),
                27,
                3,
                KeyModifiers::CONTROL,
            )),
            FileManagerMouseDispatch::Consumed
        );
        handle_file_manager_key(
            &mut app.state,
            key_with_modifiers(KeyCode::Down, KeyModifiers::CONTROL),
        );

        let fm = app.state.file_manager.as_ref().expect("open fm");
        assert_eq!(fm.cursor, 0);
        assert_eq!(fm.multi_selection_paths(), &before_paths);
        assert_eq!(
            fm.multi_selection_anchor(),
            Some(td.root.join("00.txt").as_path())
        );
    }

    // TP-N4.1-SELECTION-STATE: row hit geometry snapshots stable path identity
    // so a watcher reorder at the same valid index can be rejected on input.
    #[test]
    fn row_selection_snapshot_carries_stable_path_identity() {
        let td = TempDir::new("multi-selection-row-identity");
        td.file("00.txt");
        td.file("01.txt");
        let mut app = runtime_app_with_fm(FmState::new(&td.root));
        let expected = td.root.join("00.txt");

        assert_eq!(
            app.state.view.file_manager_row_areas[0].entry_path,
            expected
        );

        let preserved = td.root.join("01.txt");
        assert!(app
            .state
            .file_manager
            .as_mut()
            .expect("open fm")
            .replace_selection(1));
        app.state
            .file_manager
            .as_mut()
            .expect("open fm")
            .entries
            .swap(0, 1);

        assert_eq!(
            app.handle_file_manager_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 27, 2,)),
            FileManagerMouseDispatch::Consumed
        );
        let fm = app.state.file_manager.as_ref().expect("open fm");
        assert_eq!(fm.cursor, 1);
        assert_eq!(
            fm.multi_selection_paths().iter().collect::<Vec<_>>(),
            vec![&preserved]
        );
    }

    // TP-C1.2-DISPATCH: every complete visible header rectangle resolves to
    // its exact tag, while C1.2 performs no filesystem mutation or selection.
    #[test]
    fn header_left_click_dispatches_exact_tags_without_filesystem_effects() {
        let td = TempDir::new("header-actions");
        td.file("selected.txt");
        let mut app = runtime_app_with_fm(FmState::new(&td.root));
        install_wide_header_actions(&mut app);
        let before_entries = fs::read_dir(&td.root)
            .expect("read fixture before clicks")
            .map(|entry| entry.expect("fixture entry").file_name())
            .collect::<Vec<_>>();

        for (column, action) in [
            (50, FileManagerHeaderAction::Copy),
            (63, FileManagerHeaderAction::Paste),
            (76, FileManagerHeaderAction::NewFolder),
            (85, FileManagerHeaderAction::Delete),
        ] {
            assert_eq!(
                app.handle_file_manager_mouse(mouse(
                    MouseEventKind::Down(MouseButton::Left),
                    column,
                    0,
                )),
                FileManagerMouseDispatch::HeaderAction(action)
            );
        }

        let fm = app.state.file_manager.as_ref().expect("file manager open");
        assert_eq!(fm.cwd, td.root);
        assert_eq!(fm.cursor, 0);
        let after_entries = fs::read_dir(&td.root)
            .expect("read fixture after clicks")
            .map(|entry| entry.expect("fixture entry").file_name())
            .collect::<Vec<_>>();
        assert_eq!(after_entries, before_entries);
    }

    // TP-C4.1-LIFECYCLE: the top-level mouse router must consume the typed
    // header result and dispatch it to the App controller instead of silently
    // dropping an enabled Copy action.
    #[test]
    fn top_level_mouse_dispatches_header_copy_to_clipboard_controller() {
        let td = TempDir::new("header-copy-controller");
        td.file("selected.txt");
        let selected = td.root.join("selected.txt");
        let mut app = runtime_app_with_fm(FmState::new(&td.root));
        assert!(app
            .state
            .file_manager
            .as_mut()
            .expect("open FM")
            .replace_selection(0));
        install_wide_header_actions(&mut app);

        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 50, 0));

        assert!(app.state.file_manager_clipboard.is_empty());
        assert_eq!(
            app.state.request_file_manager_context_action,
            Some(crate::app::state::FileManagerContextActionIntent {
                action: FileManagerContextMenuAction::Copy,
                paths: vec![selected.clone()],
            })
        );
        assert!(app.sync_file_operation_worker());
        assert_eq!(app.state.file_manager_clipboard, vec![selected]);
        assert_eq!(
            fs::read(td.root.join("selected.txt")).expect("copy action preserves source"),
            b"x"
        );
    }

    // TP-C1.2-DISPATCH: identity/gap/outside/hidden/zero/stale/non-left input
    // never invents a header action from coordinates or stale paint state.
    #[test]
    fn header_action_dispatch_fails_closed_for_non_targets() {
        let td = TempDir::new("header-non-targets");
        td.file("selected.txt");
        let mut app = runtime_app_with_fm(FmState::new(&td.root));
        install_wide_header_actions(&mut app);

        for column in [26, 49, 56, 64, 77] {
            assert_eq!(
                app.handle_file_manager_mouse(mouse(
                    MouseEventKind::Down(MouseButton::Left),
                    column,
                    0,
                )),
                FileManagerMouseDispatch::Consumed,
                "non-action header column {column}"
            );
        }
        assert_eq!(
            app.handle_file_manager_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 50, 1,)),
            FileManagerMouseDispatch::Consumed
        );
        assert_eq!(
            app.handle_file_manager_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 25, 0,)),
            FileManagerMouseDispatch::NotHandled
        );

        app.state.view.file_manager_header_action_areas.truncate(1);
        assert_eq!(
            app.handle_file_manager_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 60, 0,)),
            FileManagerMouseDispatch::Consumed,
            "hidden Paste action is not inferred"
        );
        app.state.view.file_manager_header_action_areas.clear();
        assert_eq!(
            app.handle_file_manager_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 50, 0,)),
            FileManagerMouseDispatch::Consumed,
            "zero visible actions fail closed"
        );

        install_wide_header_actions(&mut app);
        for kind in [
            MouseEventKind::Down(MouseButton::Right),
            MouseEventKind::Down(MouseButton::Middle),
        ] {
            assert_eq!(
                app.handle_file_manager_mouse(mouse(kind, 50, 0)),
                FileManagerMouseDispatch::Consumed
            );
        }
        assert_eq!(
            app.handle_file_manager_mouse(mouse_with_modifiers(
                MouseEventKind::Down(MouseButton::Left),
                50,
                0,
                KeyModifiers::CONTROL,
            )),
            FileManagerMouseDispatch::Consumed
        );

        app.state.file_manager = None;
        assert_eq!(
            app.handle_file_manager_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 50, 0,)),
            FileManagerMouseDispatch::NotHandled,
            "stale areas cannot dispatch after FM closes"
        );
    }

    // TP-N3.2-AUTHORITY: a disabled visible action is consumed without tag,
    // selection, clipboard, cwd, or filesystem mutation.
    #[test]
    fn disabled_header_action_is_consumed_without_side_effects() {
        let td = TempDir::new("disabled-header-action");
        td.file("selected.txt");
        let mut app = runtime_app_with_fm(FmState::new(&td.root));
        install_wide_header_actions(&mut app);
        let action_bar = app
            .state
            .view
            .file_manager_action_bar
            .as_mut()
            .expect("action bar model");
        let copy = action_bar
            .actions
            .iter_mut()
            .find(|state| state.action == FileManagerHeaderAction::Copy)
            .expect("copy state");
        copy.enabled = false;
        copy.disabled_reason = Some(FileManagerActionDisabledReason::OperationInFlight);

        let before_cursor = app.state.file_manager.as_ref().expect("open FM").cursor;
        let before_cwd = app
            .state
            .file_manager
            .as_ref()
            .expect("open FM")
            .cwd
            .clone();
        let before_clipboard = app.state.file_manager_clipboard.clone();
        let before_entries = fs::read_dir(&td.root)
            .expect("read fixture before click")
            .count();

        assert_eq!(
            app.handle_file_manager_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 50, 0,)),
            FileManagerMouseDispatch::Consumed
        );
        assert_eq!(
            app.state.file_manager.as_ref().expect("open FM").cursor,
            before_cursor
        );
        assert_eq!(
            app.state.file_manager.as_ref().expect("open FM").cwd,
            before_cwd
        );
        assert_eq!(app.state.file_manager_clipboard, before_clipboard);
        assert_eq!(
            fs::read_dir(&td.root)
                .expect("read fixture after click")
                .count(),
            before_entries
        );
    }

    // TP-C2.2-ROW-DISPATCH: every complete visible row-action rectangle
    // resolves to its exact tag plus stable path identity. C2.2 only routes
    // tags; it must not select the row or mutate clipboard/cwd/filesystem.
    #[test]
    fn row_left_click_dispatches_exact_tags_without_side_effects() {
        let td = TempDir::new("row-actions");
        td.file("alpha.txt");
        td.file("beta.txt");
        let mut app = runtime_app_with_fm(FmState::new(&td.root));
        let entry_path = install_row_actions(&mut app, 1);
        let before_cursor = app.state.file_manager.as_ref().expect("open FM").cursor;
        let before_cwd = app
            .state
            .file_manager
            .as_ref()
            .expect("open FM")
            .cwd
            .clone();
        let before_clipboard = app.state.file_manager_clipboard.clone();
        let before_entries = fs::read_dir(&td.root)
            .expect("read row-action fixture before clicks")
            .map(|entry| entry.expect("fixture entry").file_name())
            .collect::<Vec<_>>();

        for (column, action) in [
            (43, FileManagerRowAction::SendAgent),
            (44, FileManagerRowAction::Rename),
            (45, FileManagerRowAction::Delete),
        ] {
            assert_eq!(
                app.handle_file_manager_mouse(mouse(
                    MouseEventKind::Down(MouseButton::Left),
                    column,
                    3,
                )),
                FileManagerMouseDispatch::RowAction {
                    action,
                    entry_path: entry_path.clone(),
                }
            );
        }

        let fm = app.state.file_manager.as_ref().expect("file manager open");
        assert_eq!(fm.cwd, before_cwd);
        assert_eq!(fm.cursor, before_cursor);
        assert_eq!(app.state.file_manager_clipboard, before_clipboard);
        assert_eq!(
            fs::read_dir(&td.root)
                .expect("read row-action fixture after clicks")
                .map(|entry| entry.expect("fixture entry").file_name())
                .collect::<Vec<_>>(),
            before_entries
        );
    }

    // TP-C5-AUTHORITY: the row SendAgent tag must bind one exact current path
    // to the focused agent terminal identity without sending bytes, spawning a
    // process, mutating the filesystem, or reconstructing authority from text.
    #[test]
    fn row_send_agent_prepares_exact_path_and_focused_terminal_identity() {
        let td = TempDir::new("row-send-agent-authority");
        td.file("alpha.txt");
        td.file("beta.txt");
        let mut app = runtime_app_with_fm(FmState::new(&td.root));
        let terminal_id = install_focused_agent(&mut app);
        let entry_path = install_row_actions(&mut app, 1);
        let before = fs::read(&entry_path).expect("read send-agent source before intent");

        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 43, 3));

        assert_eq!(
            app.state.request_file_manager_context_action,
            Some(crate::app::state::FileManagerContextActionIntent {
                action: FileManagerContextMenuAction::SendAgent,
                paths: vec![entry_path.clone()],
            })
        );
        assert!(app.state.request_file_manager_agent_handoff.is_none());
        assert!(app.sync_file_manager_agent_handoff());
        assert_eq!(
            app.state.request_file_manager_agent_handoff,
            Some(FileManagerAgentHandoffRequest {
                path: entry_path.clone(),
                terminal_id,
            })
        );
        assert_eq!(
            fs::read(entry_path).expect("send-agent source remains unchanged"),
            before
        );
    }

    // TP-C5-AUTHORITY: C3's single-path context intent converges on the same
    // typed current-agent request and is consumed exactly once.
    #[test]
    fn context_send_agent_converges_on_typed_current_authority() {
        let td = TempDir::new("context-send-agent-authority");
        td.file("selected.txt");
        let path = td.root.join("selected.txt");
        let mut app = runtime_app_with_fm(FmState::new(&td.root));
        let terminal_id = install_focused_agent(&mut app);
        app.state.request_file_manager_context_action =
            Some(crate::app::state::FileManagerContextActionIntent {
                action: FileManagerContextMenuAction::SendAgent,
                paths: vec![path.clone()],
            });

        assert!(app.sync_file_manager_agent_handoff());
        assert!(app.state.request_file_manager_context_action.is_none());
        assert_eq!(
            app.state.request_file_manager_agent_handoff,
            Some(FileManagerAgentHandoffRequest { path, terminal_id })
        );
        assert!(!app.sync_file_manager_agent_handoff());
    }

    // TP-C5-SPLIT: a non-agent terminal binds the exact current path, FM cwd,
    // and source pane/terminal identities without spawning during input.
    #[test]
    fn send_agent_on_non_agent_terminal_prepares_exact_split_authority_only() {
        let td = TempDir::new("send-agent-split-authority");
        td.file("alpha.txt");
        td.file("beta.txt");

        let mut app = runtime_app_with_fm(FmState::new(&td.root));
        let workspace = crate::workspace::Workspace::test_new("fm-non-agent");
        let workspace_id = workspace.id.clone();
        let source_pane_id = workspace.tabs[0].root_pane;
        let source_terminal_id = workspace
            .terminal_id(source_pane_id)
            .expect("source terminal identity")
            .clone();
        app.state.workspaces = vec![workspace];
        app.state.ensure_test_terminals();
        app.state.active = Some(0);
        app.state.selected = 0;
        let entry_path = install_row_actions(&mut app, 1);
        let before_panes = app.state.workspaces[0].tabs[0].layout.pane_count();
        let before_terminals = app.state.terminals.len();
        let before_runtimes = app.terminal_runtimes.len();

        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 43, 3));

        assert!(app.state.request_file_manager_agent_handoff.is_none());
        assert!(app.sync_file_manager_agent_handoff());
        assert_eq!(
            app.state.request_file_manager_claude_split,
            Some(crate::app::state::FileManagerClaudeSplitRequest {
                path: entry_path,
                cwd: td.root.clone(),
                workspace_id,
                source_pane_id,
                source_terminal_id,
            })
        );
        assert_eq!(
            app.state.workspaces[0].tabs[0].layout.pane_count(),
            before_panes
        );
        assert_eq!(app.state.terminals.len(), before_terminals);
        assert_eq!(app.terminal_runtimes.len(), before_runtimes);
    }

    // TP-C5-AUTHORITY: bulk row authority or an operation-in-flight snapshot
    // cannot create either existing-agent or split-and-launch authority.
    #[test]
    fn send_agent_authority_fails_closed_without_current_single_path() {
        let td = TempDir::new("send-agent-fail-closed");
        td.file("alpha.txt");
        td.file("beta.txt");

        let mut bulk = runtime_app_with_fm(FmState::new(&td.root));
        install_focused_agent(&mut bulk);
        install_row_actions(&mut bulk, 1);
        let fm = bulk.state.file_manager.as_mut().expect("bulk FM open");
        assert!(fm.replace_selection(0));
        assert!(fm.toggle_selection(1));
        bulk.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 43, 3));
        assert!(bulk.state.request_file_manager_agent_handoff.is_none());
        assert!(bulk.state.request_file_manager_claude_split.is_none());

        let mut busy = runtime_app_with_fm(FmState::new(&td.root));
        install_focused_agent(&mut busy);
        install_row_actions(&mut busy, 1);
        busy.state.file_manager_operation = Some(crate::app::state::FileManagerOperationState {
            generation: 1,
            kind: crate::app::state::FileManagerOperationKind::Copy,
            destination_directory: td.root.clone(),
            total_items: 1,
            completed_items: 0,
            failed_items: 0,
            status: crate::app::state::FileManagerOperationStatus::Running,
            items: Vec::new(),
        });
        busy.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 43, 3));
        assert!(busy.state.request_file_manager_agent_handoff.is_none());
        assert!(busy.state.request_file_manager_claude_split.is_none());
    }

    // TP-C4.3-INTENT: the stable row Rename tag must converge on one typed
    // exact-path file modal. Opening it is pure client-local authority: no
    // worker generation or filesystem mutation exists yet.
    #[test]
    fn row_rename_opens_exact_file_modal_without_filesystem_work() {
        let td = TempDir::new("row-rename-intent");
        td.file("alpha.txt");
        td.file("beta.txt");
        let mut app = runtime_app_with_fm(FmState::new(&td.root));
        let entry_path = install_row_actions(&mut app, 1);
        let before = fs::read(&entry_path).expect("read row rename fixture");

        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 44, 3));

        assert_eq!(
            app.state.request_file_manager_context_action,
            Some(crate::app::state::FileManagerContextActionIntent {
                action: FileManagerContextMenuAction::Rename,
                paths: vec![entry_path.clone()],
            })
        );
        assert_ne!(app.state.mode, Mode::RenameFile);
        assert!(app.sync_file_operation_worker());
        assert_eq!(app.state.mode, Mode::RenameFile);
        assert_eq!(
            app.state
                .file_manager_rename
                .as_ref()
                .expect("typed file rename modal")
                .paths,
            vec![entry_path.clone()]
        );
        assert_eq!(app.state.name_input, "beta.txt");
        assert!(app.state.name_input_replace_on_type);
        assert!(app.state.file_manager_operation.is_none());
        assert_eq!(
            fs::read(&entry_path).expect("row rename target remains untouched"),
            before
        );
    }

    // TP-C6.3-AUTHORITY: the row Delete tag selects its exact anchored row,
    // emits the same typed C3 intent as context/header Delete, and reaches the
    // existing confirmation owner only at the scheduled boundary.
    #[test]
    fn row_delete_converges_on_shared_typed_confirmation_authority() {
        let td = TempDir::new("row-delete-authority");
        td.file("alpha.txt");
        td.file("beta.txt");
        let mut app = runtime_app_with_fm(FmState::new(&td.root));
        let entry_path = install_row_actions(&mut app, 1);
        let before = fs::read(&entry_path).expect("read row delete fixture");

        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 45, 3));

        assert_eq!(
            app.state.request_file_manager_context_action,
            Some(crate::app::state::FileManagerContextActionIntent {
                action: FileManagerContextMenuAction::Delete,
                paths: vec![entry_path.clone()],
            })
        );
        assert!(app.state.file_manager_delete_confirmation.is_none());
        assert!(app.sync_file_operation_worker());
        assert_eq!(
            app.state
                .file_manager_delete_confirmation
                .as_ref()
                .expect("shared delete confirmation")
                .paths,
            vec![entry_path.clone()]
        );
        assert_eq!(
            fs::read(entry_path).expect("row delete remains confirmation-only"),
            before
        );
    }

    // TP-C4.3-INTENT: a row coordinate is not independent authority while a
    // bulk selection or another operation is active. Both cases fail closed
    // before a modal or worker request can exist.
    #[test]
    fn row_rename_rejects_bulk_selection_and_inflight_operation() {
        let td = TempDir::new("row-rename-fail-closed");
        td.file("alpha.txt");
        td.file("beta.txt");
        let mut app = runtime_app_with_fm(FmState::new(&td.root));
        install_row_actions(&mut app, 1);
        let fm = app.state.file_manager.as_mut().expect("open FM");
        assert!(fm.replace_selection(0));
        assert!(fm.toggle_selection(1));

        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 44, 3));
        assert_ne!(app.state.mode, Mode::RenameFile);
        assert!(app.state.file_manager_rename.is_none());

        app.state
            .file_manager
            .as_mut()
            .expect("open FM")
            .clear_multi_selection();
        app.state.file_manager_operation = Some(crate::app::state::FileManagerOperationState {
            generation: 9,
            kind: crate::app::state::FileManagerOperationKind::Copy,
            destination_directory: td.root.clone(),
            total_items: 1,
            completed_items: 0,
            failed_items: 0,
            status: crate::app::state::FileManagerOperationStatus::Running,
            items: Vec::new(),
        });
        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 44, 3));
        assert_ne!(app.state.mode, Mode::RenameFile);
        assert!(app.state.file_manager_rename.is_none());
    }

    // TP-C2.2-NON-TARGETS: the name rectangle preserves selection, while
    // gaps, hidden actions, middle presses, modifiers, and stale closed-FM
    // geometry cannot invent a row action. Right press is owned by C3.2.
    #[test]
    fn row_action_dispatch_preserves_names_and_fails_closed_for_non_targets() {
        let td = TempDir::new("row-action-non-targets");
        td.file("alpha.txt");
        td.file("beta.txt");
        let mut app = runtime_app_with_fm(FmState::new(&td.root));
        install_row_actions(&mut app, 1);

        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 27, 3));
        assert_eq!(app.state.file_manager.as_ref().expect("open FM").cursor, 1);

        for event in [
            mouse(MouseEventKind::Down(MouseButton::Middle), 43, 3),
            mouse_with_modifiers(
                MouseEventKind::Down(MouseButton::Left),
                43,
                3,
                KeyModifiers::CONTROL,
            ),
            mouse(MouseEventKind::Down(MouseButton::Left), 43, 4),
            mouse(MouseEventKind::Down(MouseButton::Left), 43, 1),
        ] {
            assert_eq!(
                app.handle_file_manager_mouse(event),
                FileManagerMouseDispatch::Consumed
            );
        }

        app.state.view.file_manager_row_action_areas.clear();
        assert_eq!(
            app.handle_file_manager_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 43, 3,)),
            FileManagerMouseDispatch::Consumed,
            "hidden action is not inferred from its former coordinates"
        );
        assert_eq!(
            app.handle_file_manager_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 25, 3,)),
            FileManagerMouseDispatch::NotHandled
        );

        app.state.file_manager = None;
        assert_eq!(
            app.handle_file_manager_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 43, 3,)),
            FileManagerMouseDispatch::NotHandled
        );
    }

    // TP-C2.2-STALE-IDENTITY: a watcher reload can reorder entries between
    // compute_view and input. Matching coordinates and index are insufficient;
    // the snapshotted path must still match and the live target must remain
    // supported before a tag can escape.
    #[test]
    fn row_action_dispatch_rejects_reordered_and_unsupported_targets() {
        let td = TempDir::new("row-action-stale");
        td.file("alpha.txt");
        td.file("beta.txt");
        let mut app = runtime_app_with_fm(FmState::new(&td.root));
        install_row_actions(&mut app, 0);
        app.state
            .file_manager
            .as_mut()
            .expect("open FM")
            .entries
            .swap(0, 1);

        assert_eq!(
            app.handle_file_manager_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 43, 2,)),
            FileManagerMouseDispatch::Consumed,
            "same index with a different path is stale"
        );

        let mut app = runtime_app_with_fm(FmState::new(&td.root));
        install_row_actions(&mut app, 0);
        app.state.file_manager.as_mut().expect("open FM").entries[0].operation_supported = false;
        assert_eq!(
            app.handle_file_manager_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 43, 2,)),
            FileManagerMouseDispatch::Consumed,
            "unsupported live target fails closed"
        );
    }

    // TP-C3.2-POPUP-GEOMETRY: right-clicking a member of an explicit bulk
    // selection preserves that set while focusing the clicked row. A row
    // outside the set replaces it with one exact live path before the menu
    // model is prepared.
    #[test]
    fn right_click_applies_exact_selection_policy_before_opening_file_menu() {
        let td = TempDir::new("file-context-selection-policy");
        td.file("00.txt");
        td.file("01.txt");
        td.file("02.txt");
        let mut app = runtime_app_with_fm(FmState::new(&td.root));
        let fm = app.state.file_manager.as_mut().expect("open FM");
        assert!(fm.replace_selection(0));
        assert!(fm.toggle_selection(1));
        let bulk_paths = fm.multi_selection_paths().clone();
        let before_entries = fs::read_dir(&td.root)
            .expect("read context-menu fixture before clicks")
            .map(|entry| entry.expect("fixture entry").file_name())
            .collect::<Vec<_>>();

        assert_eq!(
            app.handle_file_manager_mouse(mouse(MouseEventKind::Down(MouseButton::Right), 27, 3,)),
            FileManagerMouseDispatch::Consumed
        );
        assert_eq!(app.state.mode, Mode::ContextMenu);
        let fm = app.state.file_manager.as_ref().expect("open FM");
        assert_eq!(fm.cursor, 1, "right-click focuses the exact live row");
        assert_eq!(fm.multi_selection_paths(), &bulk_paths);
        let menu = app.state.context_menu.as_ref().expect("file context menu");
        assert_eq!((menu.x, menu.y), (27, 3));
        let ContextMenuKind::File { model } = &menu.kind else {
            panic!("expected file context menu")
        };
        assert_eq!(
            model.target_kind,
            FileManagerContextMenuTargetKind::Multiple
        );
        assert_eq!(model.paths, bulk_paths.iter().cloned().collect::<Vec<_>>());

        app.state.context_menu = None;
        app.state.mode = Mode::Terminal;
        assert_eq!(
            app.handle_file_manager_mouse(mouse(MouseEventKind::Down(MouseButton::Right), 27, 4,)),
            FileManagerMouseDispatch::Consumed
        );
        let fm = app.state.file_manager.as_ref().expect("open FM");
        assert_eq!(fm.cursor, 2);
        assert_eq!(
            fm.multi_selection_paths().iter().collect::<Vec<_>>(),
            vec![&td.root.join("02.txt")]
        );
        let menu = app.state.context_menu.as_ref().expect("replacement menu");
        let ContextMenuKind::File { model } = &menu.kind else {
            panic!("expected replacement file context menu")
        };
        assert_eq!(model.target_kind, FileManagerContextMenuTargetKind::File);
        assert_eq!(model.paths, vec![td.root.join("02.txt")]);
        assert_eq!(
            fs::read_dir(&td.root)
                .expect("read context-menu fixture after clicks")
                .map(|entry| entry.expect("fixture entry").file_name())
                .collect::<Vec<_>>(),
            before_entries,
            "C3 routing performs no filesystem mutation"
        );
    }

    // TP-C3.2-POPUP-GEOMETRY: the same snapshotted current-row geometry used
    // by render/input opens the existing popup at one/two/three-column Miller
    // widths. First, middle, and last visible rows remain reachable and the
    // popup clamps to the complete screen rectangle at every edge.
    #[test]
    fn right_click_popup_is_bounded_at_all_miller_breakpoints() {
        let td = TempDir::new("file-context-breakpoints");
        for index in 0..12 {
            td.file(&format!("{index:02}.txt"));
        }

        for width in [20, 30, 45] {
            let mut app = runtime_app_with_fm(FmState::new(&td.root));
            app.state.mobile_width_threshold = 0;
            app.state.sidebar_collapsed = true;
            compute_view(&mut app.state, Rect::new(0, 0, width, 12));
            let rows = app.state.view.file_manager_row_areas.clone();
            assert!(!rows.is_empty(), "width {width} exposes current rows");
            let row_indices = [0, rows.len() / 2, rows.len() - 1];

            for row_index in row_indices {
                let row = &rows[row_index];
                assert_eq!(
                    app.handle_file_manager_mouse(mouse(
                        MouseEventKind::Down(MouseButton::Right),
                        row.rect.x,
                        row.rect.y,
                    )),
                    FileManagerMouseDispatch::Consumed,
                    "width {width}, visible row {row_index}"
                );
                let menu = app.state.context_menu.as_ref().expect("bounded file menu");
                let ContextMenuKind::File { model } = &menu.kind else {
                    panic!("expected file menu at width {width}")
                };
                assert_eq!(model.paths, vec![row.entry_path.clone()]);

                let popup = app.state.context_menu_rect().expect("popup geometry");
                let screen = app.state.screen_rect();
                assert!(popup.x >= screen.x && popup.y >= screen.y);
                assert!(popup.right() <= screen.right());
                assert!(popup.bottom() <= screen.bottom());
                assert_eq!(popup.height, 8, "six complete rows plus borders");

                app.state.context_menu = None;
                app.state.mode = Mode::Terminal;
            }
        }
    }

    // TP-C3.2-POPUP-GEOMETRY: stale row identity, non-row center regions,
    // modified right-click, and zero geometry are consumed without opening a
    // menu or changing the existing selection.
    #[test]
    fn right_click_file_menu_fails_closed_for_stale_and_non_targets() {
        let td = TempDir::new("file-context-stale");
        td.file("00.txt");
        td.file("01.txt");
        let mut app = runtime_app_with_fm(FmState::new(&td.root));
        assert!(app
            .state
            .file_manager
            .as_mut()
            .expect("open FM")
            .replace_selection(1));
        let before_paths = app
            .state
            .file_manager
            .as_ref()
            .expect("open FM")
            .multi_selection_paths()
            .clone();
        app.state
            .file_manager
            .as_mut()
            .expect("open FM")
            .entries
            .swap(0, 1);

        for event in [
            mouse(MouseEventKind::Down(MouseButton::Right), 27, 2),
            mouse(MouseEventKind::Down(MouseButton::Right), 27, 0),
            mouse(MouseEventKind::Down(MouseButton::Right), 27, 1),
            mouse(MouseEventKind::Down(MouseButton::Right), 45, 5),
            mouse_with_modifiers(
                MouseEventKind::Down(MouseButton::Right),
                27,
                3,
                KeyModifiers::CONTROL,
            ),
        ] {
            assert_eq!(
                app.handle_file_manager_mouse(event),
                FileManagerMouseDispatch::Consumed
            );
            assert!(app.state.context_menu.is_none());
        }
        let fm = app.state.file_manager.as_ref().expect("open FM");
        assert_eq!(fm.cursor, 1);
        assert_eq!(fm.multi_selection_paths(), &before_paths);

        app.state.view.terminal_area = Rect::default();
        assert_eq!(
            app.handle_file_manager_mouse(mouse(MouseEventKind::Down(MouseButton::Right), 27, 3,)),
            FileManagerMouseDispatch::NotHandled
        );
        assert!(app.state.context_menu.is_none());
    }

    // TP-C3.2-POPUP-GEOMETRY: row-local action cells are still part of the
    // exact current-row identity for right-click context, but no row action
    // tag or side effect escapes.
    #[test]
    fn right_click_row_action_cell_opens_the_same_file_context() {
        let td = TempDir::new("file-context-row-action");
        td.file("00.txt");
        td.file("01.txt");
        let mut app = runtime_app_with_fm(FmState::new(&td.root));
        let expected = install_row_actions(&mut app, 1);

        assert_eq!(
            app.handle_file_manager_mouse(mouse(MouseEventKind::Down(MouseButton::Right), 44, 3,)),
            FileManagerMouseDispatch::Consumed
        );
        let menu = app
            .state
            .context_menu
            .as_ref()
            .expect("row action file menu");
        let ContextMenuKind::File { model } = &menu.kind else {
            panic!("expected file menu")
        };
        assert_eq!(model.paths, vec![expected]);
    }

    // TP-C3.2-POPUP-LIFECYCLE: ContextMenu keyboard routing owns focus even
    // while the FM remains open. Down selects Copy without moving the FM
    // cursor; Enter emits one exact client-local intent and no filesystem
    // mutation.
    #[test]
    fn file_context_menu_keyboard_owns_focus_and_emits_exact_intent() {
        let td = TempDir::new("file-context-keyboard-intent");
        td.file("00.txt");
        let mut app = runtime_app_with_fm(FmState::new(&td.root));
        let before_entries = fs::read_dir(&td.root)
            .expect("read keyboard fixture before")
            .count();

        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Right), 27, 2));
        assert_eq!(app.state.mode, Mode::ContextMenu);
        let cursor = app.state.file_manager.as_ref().expect("open FM").cursor;

        app.route_client_input(b"\x1b[B".to_vec());
        assert_eq!(
            app.state
                .context_menu
                .as_ref()
                .expect("menu after Down")
                .list
                .highlighted,
            1
        );
        assert_eq!(
            app.state.file_manager.as_ref().expect("open FM").cursor,
            cursor,
            "menu navigation cannot move the FM cursor"
        );

        app.route_client_input(b"\r".to_vec());
        assert!(app.state.context_menu.is_none());
        assert_ne!(app.state.mode, Mode::ContextMenu);
        let intent = app
            .state
            .request_file_manager_context_action
            .as_ref()
            .expect("exact file action intent");
        assert_eq!(intent.action, FileManagerContextMenuAction::Copy);
        assert_eq!(intent.paths, vec![td.root.join("00.txt")]);
        assert_eq!(
            fs::read_dir(&td.root)
                .expect("read keyboard fixture after")
                .count(),
            before_entries,
            "C3 intent dispatch performs no filesystem mutation"
        );
    }

    // TP-C3.3-PLUGIN-SURFACE: the actual right-click/input pipeline appends an
    // enabled file action, emits only typed public invocation parameters, and
    // revalidates plugin enabled state before activation.
    #[test]
    fn file_context_menu_plugin_action_is_typed_and_disable_race_fails_closed() {
        let td = TempDir::new("file-context-plugin-intent");
        td.file("00.txt");
        let plugin_td = TempDir::new("file-context-plugin-manifest");
        let manifest = plugin_td.root.join("herdr-plugin.toml");
        fs::write(
            &manifest,
            r#"
id = "example.files"
name = "Example Files"
version = "0.1.0"
min_herdr_version = "0.6.10"

[[actions]]
id = "inspect"
title = "Inspect file"
contexts = ["file"]
command = ["inspect"]
"#,
        )
        .expect("write plugin manifest");
        let plugin =
            crate::app::api::plugins::load_plugin_manifest(&manifest.display().to_string(), true)
                .expect("valid plugin manifest");

        let mut app = runtime_app_with_fm(FmState::new(&td.root));
        app.state
            .installed_plugins
            .insert(plugin.plugin_id.clone(), plugin);
        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Right), 27, 2));
        let menu = app.state.context_menu.as_ref().expect("plugin file menu");
        let ContextMenuKind::File { model } = &menu.kind else {
            panic!("expected file menu")
        };
        assert_eq!(model.items.len(), 7);
        assert_eq!(model.items[6].label, "Inspect file");
        assert_eq!(
            model.items[6].action,
            FileManagerContextMenuAction::Plugin {
                plugin_id: "example.files".into(),
                action_id: "inspect".into(),
            }
        );

        for _ in 0..6 {
            app.route_client_input(b"\x1b[B".to_vec());
        }
        app.route_client_input(b"\r".to_vec());
        let intent = app
            .state
            .request_file_manager_context_action
            .as_ref()
            .expect("typed plugin file intent");
        let params = intent
            .plugin_invocation_params()
            .expect("public plugin invocation params");
        assert_eq!(params.plugin_id.as_deref(), Some("example.files"));
        assert_eq!(params.action_id, "inspect");
        assert_eq!(
            params.context.expect("file context").file_paths,
            vec![td.root.join("00.txt").display().to_string()]
        );
        assert!(app.state.plugin_command_logs.is_empty());

        app.state.request_file_manager_context_action = None;
        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Right), 27, 2));
        app.state
            .installed_plugins
            .get_mut("example.files")
            .expect("installed plugin")
            .enabled = false;
        for _ in 0..6 {
            app.route_client_input(b"\x1b[B".to_vec());
        }
        app.route_client_input(b"\r".to_vec());
        assert!(app.state.request_file_manager_context_action.is_none());
        assert!(app.state.plugin_command_logs.is_empty());
    }

    // TP-C3.2-POPUP-LIFECYCLE: disabled activation stays open and emits
    // nothing. Reorder, delete, and operation-in-flight changes after menu
    // creation invalidate the snapshot before any intent can escape.
    #[test]
    fn disabled_and_stale_file_context_actions_fail_closed() {
        let td = TempDir::new("file-context-disabled-stale");
        td.file("00.txt");
        td.file("01.txt");
        let mut app = runtime_app_with_fm(FmState::new(&td.root));
        let fm = app.state.file_manager.as_mut().expect("open FM");
        assert!(fm.replace_selection(0));
        assert!(fm.toggle_selection(1));

        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Right), 27, 2));
        app.route_client_input(b"\r".to_vec());
        assert!(app.state.context_menu.is_some(), "disabled Open stays open");
        assert!(app.state.request_file_manager_context_action.is_none());

        app.route_client_input(b"\x1b[B".to_vec());
        app.state
            .file_manager
            .as_mut()
            .expect("open FM")
            .entries
            .swap(0, 1);
        app.route_client_input(b"\r".to_vec());
        assert!(app.state.context_menu.is_none(), "stale menu closes");
        assert!(app.state.request_file_manager_context_action.is_none());

        let mut app = runtime_app_with_fm(FmState::new(&td.root));
        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Right), 27, 2));
        app.state.file_manager_operation = Some(crate::app::state::FileManagerOperationState {
            generation: 1,
            kind: crate::app::state::FileManagerOperationKind::Copy,
            destination_directory: td.root.clone(),
            total_items: 1,
            completed_items: 0,
            failed_items: 0,
            status: crate::app::state::FileManagerOperationStatus::Running,
            items: Vec::new(),
        });
        app.route_client_input(b"\r".to_vec());
        assert!(app.state.request_file_manager_context_action.is_none());

        let mut app = runtime_app_with_fm(FmState::new(&td.root));
        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Right), 27, 2));
        fs::remove_file(td.root.join("00.txt")).expect("delete selected fixture path");
        app.state.file_manager.as_mut().expect("open FM").reload();
        app.route_client_input(b"\r".to_vec());
        assert!(app.state.context_menu.is_none());
        assert!(app.state.request_file_manager_context_action.is_none());
    }

    // TP-C3.2-POPUP-LIFECYCLE: mouse hover uses the existing global menu hit
    // geometry. Disabled click stays open, enabled click emits exact intent,
    // outside click closes, and closing FM clears its owned popup.
    #[test]
    fn file_context_menu_mouse_hover_click_outside_and_close_lifecycle() {
        let td = TempDir::new("file-context-mouse-lifecycle");
        td.file("00.txt");
        td.file("01.txt");
        let mut app = runtime_app_with_fm(FmState::new(&td.root));
        let fm = app.state.file_manager.as_mut().expect("open FM");
        assert!(fm.replace_selection(0));
        assert!(fm.toggle_selection(1));
        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Right), 27, 2));
        let popup = app.state.context_menu_rect().expect("popup");
        let item_x = popup.x + 1;
        let open_y = popup.y + 1;
        let copy_y = popup.y + 2;

        app.handle_mouse(mouse(MouseEventKind::Moved, item_x, copy_y));
        assert_eq!(
            app.state
                .context_menu
                .as_ref()
                .expect("hovered menu")
                .list
                .highlighted,
            1
        );
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            item_x,
            open_y,
        ));
        assert!(
            app.state.context_menu.is_some(),
            "disabled mouse row stays open"
        );
        assert!(app.state.request_file_manager_context_action.is_none());

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            item_x,
            copy_y,
        ));
        let intent = app
            .state
            .request_file_manager_context_action
            .take()
            .expect("enabled mouse intent");
        assert_eq!(intent.action, FileManagerContextMenuAction::Copy);
        assert_eq!(intent.paths.len(), 2);

        app.state.mode = Mode::Terminal;
        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Right), 27, 2));
        let popup = app.state.context_menu_rect().expect("reopened popup");
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            popup.right(),
            popup.bottom(),
        ));
        assert!(app.state.context_menu.is_none(), "outside click closes");

        app.state.mode = Mode::Terminal;
        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Right), 27, 2));
        assert!(app.state.context_menu.is_some());
        app.state.close_file_manager();
        assert!(app.state.context_menu.is_none());
        assert_ne!(app.state.mode, Mode::ContextMenu);
    }
}
