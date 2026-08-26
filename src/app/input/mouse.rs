use bytes::Bytes;
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::{Direction, Position, Rect};
use tracing::warn;

use crate::{
    app::state::{
        AgentPanelSort, AppState, ContextMenuKind, ContextMenuState, DragState, DragTarget,
        MenuListState, Mode, RightClickPassthroughGesture, TabPressState, ViewLayout,
        WorkspacePressState,
    },
    layout::{PaneInfo, SplitBorder},
    selection::Selection,
    terminal::TerminalRuntimeRegistry,
};

#[cfg(test)]
use super::WheelRouting;
use super::{
    modal::{
        apply_global_menu_action, confirm_close_cancel, global_menu_actions, leave_modal,
        modal_action_from_buttons, open_global_menu, open_new_tab_dialog, ModalAction,
    },
    settings::SettingsAction,
    ScrollbarClickTarget, TAB_DRAG_THRESHOLD, WORKSPACE_DRAG_THRESHOLD,
};

pub(super) enum MouseAction {
    AgentReferencePickerActivate,
    AgentColleaguePickerActivate,
    /// A bar config panel row was clicked: it is selected, and the press is
    /// resolved on the App road (verbs act, knobs adjust + preview).
    BarConfigPanelPress,
    /// A click outside the panel: cancel — restore the preview and close.
    BarConfigPanelCancel,
    /// A device row in the Taildrop picker was clicked: highlight it and send.
    TailscaleSendActivate,
    NewWorkspace,
    Settings(SettingsAction),
    FocusWorkspace {
        ws_idx: usize,
    },
    FocusTab {
        tab_idx: usize,
    },
    FocusPane {
        ws_idx: usize,
        pane_id: crate::layout::PaneId,
    },
    ReviveClosedAgent {
        agent_id: String,
    },
    /// TP-TAB-SPLIT-02: a split-button press, resolved to its direction.
    SplitFocusedPane {
        direction: crate::api::schema::SplitDirection,
    },
    FocusToastTarget,
    ToggleProjectsActives,
    ToggleSpacesFocus,
    MoveWorkspace {
        source_ws_idx: usize,
        insert_idx: usize,
    },
    MoveWorkspaceBlock {
        params: crate::api::schema::WorkspaceMoveBlockParams,
    },
    MoveTab {
        ws_idx: usize,
        source_tab_idx: usize,
        insert_idx: usize,
    },
    SetSplitRatio {
        path: Vec<bool>,
        ratio: f32,
    },
    RenameModal(ModalAction),
    ConfirmCloseAccept,
    ContextMenu {
        menu: ContextMenuState,
        idx: usize,
    },
}

enum MobileMouseResult {
    Ignored,
    Consumed,
    Action(MouseAction),
}

impl AppState {
    pub(crate) fn handle_pane_mouse_only(
        &mut self,
        terminal_runtimes: &TerminalRuntimeRegistry,
        mouse: MouseEvent,
    ) {
        if self.mode != Mode::Terminal {
            return;
        }
        let Some(info) = self.pane_at(mouse.column, mouse.row).cloned() else {
            return;
        };

        match mouse.kind {
            MouseEventKind::ScrollUp
            | MouseEventKind::ScrollDown
            | MouseEventKind::ScrollLeft
            | MouseEventKind::ScrollRight => {
                self.forward_pane_reported_wheel(terminal_runtimes, &info, mouse);
            }
            MouseEventKind::Down(_) | MouseEventKind::Up(_) | MouseEventKind::Drag(_) => {
                self.forward_pane_mouse_button(terminal_runtimes, &info, mouse);
            }
            MouseEventKind::Moved => {
                self.forward_pane_mouse_motion(terminal_runtimes, &info, mouse);
            }
        }
    }

    pub(super) fn handle_mouse(
        &mut self,
        terminal_runtimes: &mut TerminalRuntimeRegistry,
        source_id: crate::app::InputSourceId,
        mouse: MouseEvent,
    ) -> Option<MouseAction> {
        if self.mode == Mode::Onboarding {
            self.handle_onboarding_mouse(mouse);
            return None;
        }

        if self.mode == Mode::Terminal
            && self.clickable_toast_at(mouse.column, mouse.row)
            && matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left))
        {
            return Some(MouseAction::FocusToastTarget);
        }

        if self.mode == Mode::Terminal
            && self.clickable_toast_at(mouse.column, mouse.row)
            && matches!(mouse.kind, MouseEventKind::Up(MouseButton::Left))
        {
            return None;
        }

        if self.mode == Mode::Settings {
            return self.handle_settings_mouse(mouse).map(MouseAction::Settings);
        }

        // TP-SBS-DRAG-01: the one empty column between the side-by-side
        // halves is a grab handle. An active drag owns the pointer outright;
        // a Down on the named column arms it. Handled before every other
        // surface so a mid-drag pointer crossing a pane cannot leak into it.
        if self.handle_sbs_divider_mouse(&mouse) {
            return None;
        }

        // TP-AGDND-01: an armed agent-row candidate follows the pointer.
        if self.handle_agent_panel_drag_mouse(&mouse) {
            return None;
        }

        let launcher_enabled = self.view.layout != ViewLayout::Mobile
            && !self.sidebar_collapsed
            && matches!(
                self.mode,
                Mode::Terminal
                    | Mode::Navigate
                    | Mode::Resize
                    | Mode::GlobalMenu
                    | Mode::KeybindHelp
            );
        let launcher = self.global_launcher_rect();
        let launcher_hit = launcher_enabled
            && mouse.column >= launcher.x
            && mouse.column < launcher.x + launcher.width
            && mouse.row >= launcher.y
            && mouse.row < launcher.y + launcher.height;

        if matches!(mouse.kind, MouseEventKind::Moved) && self.mode == Mode::GlobalMenu {
            let actions = global_menu_actions(self);
            let hovered = self
                .global_menu_item_at(mouse.column, mouse.row)
                .and_then(|action| actions.iter().position(|item| *item == action));
            self.global_menu.hover(hovered);
            return None;
        }

        if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) && launcher_hit {
            if self.mode == Mode::GlobalMenu {
                leave_modal(self);
            } else {
                open_global_menu(self);
            }
            return None;
        }

        if self.mode == Mode::GlobalMenu {
            if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
                if let Some(action) = self.global_menu_item_at(mouse.column, mouse.row) {
                    apply_global_menu_action(self, action);
                } else {
                    leave_modal(self);
                }
            }
            return None;
        }

        if self.mode == Mode::KeybindHelp {
            return None;
        }

        if self.view.layout == ViewLayout::Mobile {
            match self.handle_mobile_mouse(mouse) {
                MobileMouseResult::Ignored => {}
                MobileMouseResult::Consumed => return None,
                MobileMouseResult::Action(action) => return Some(action),
            }
        }

        let sidebar = self.view.sidebar_rect;
        let in_sidebar = mouse.column >= sidebar.x
            && mouse.column < sidebar.x + sidebar.width
            && mouse.row >= sidebar.y
            && mouse.row < sidebar.y + sidebar.height;

        if self.handle_right_click_passthrough(terminal_runtimes, source_id, mouse, in_sidebar) {
            return None;
        }

        // The agent reference picker is a topmost blocking overlay: a row
        // click selects and activates that exact row, an outside click
        // closes with zero bytes, and every other gesture is consumed
        // fail-closed (TP-FIP-REF-15).
        // TP-AGPANEL-48: the colleague picker is the same kind of topmost
        // blocking overlay the reference picker is: a live row click selects
        // and activates, an outside click closes, everything else is consumed.
        // TP-CHROME-150: the bar config panel is the same kind of topmost
        // blocking overlay the pickers are — a row click selects and presses,
        // an outside click cancels (restoring the preview), everything else
        // over the surface is consumed.
        if self.mode == Mode::BarConfigPanel {
            if let MouseEventKind::Down(MouseButton::Left) = mouse.kind {
                // TP-CHROME-153: the strip's two tab labels switch faces in
                // place — a state-only turn, nothing to dispatch.
                if let Some(tab) = self.bar_config_panel_tab_at(mouse.column, mouse.row) {
                    if let Some(panel) = self.bar_config_panel.as_mut() {
                        panel.switch_tab(tab);
                    }
                    return None;
                }
                if let Some(idx) = self.bar_config_panel_row_at(mouse.column, mouse.row) {
                    if let Some(panel) = self.bar_config_panel.as_mut() {
                        panel.selected = idx;
                    }
                    return Some(MouseAction::BarConfigPanelPress);
                }
                let inside_popup = self.bar_config_panel_popup_rect().is_some_and(|popup| {
                    mouse.column >= popup.x
                        && mouse.column < popup.right()
                        && mouse.row >= popup.y
                        && mouse.row < popup.bottom()
                });
                if !inside_popup {
                    return Some(MouseAction::BarConfigPanelCancel);
                }
            }
            return None;
        }
        if self.mode == Mode::AgentColleaguePicker {
            if let MouseEventKind::Down(MouseButton::Left) = mouse.kind {
                if let Some(idx) = self.agent_colleague_picker_row_at(mouse.column, mouse.row) {
                    let enabled = self
                        .agent_colleague_picker
                        .as_ref()
                        .is_some_and(|picker| picker.rows.get(idx).is_some_and(|row| row.live));
                    if enabled {
                        if let Some(picker) = self.agent_colleague_picker.as_mut() {
                            picker.selected = idx;
                        }
                        return Some(MouseAction::AgentColleaguePickerActivate);
                    }
                    return None;
                }
                let inside_popup = self
                    .agent_colleague_picker_popup_rect()
                    .is_some_and(|popup| {
                        mouse.column >= popup.x
                            && mouse.column < popup.right()
                            && mouse.row >= popup.y
                            && mouse.row < popup.bottom()
                    });
                if !inside_popup {
                    self.close_agent_colleague_picker();
                }
            }
            return None;
        }
        if self.mode == Mode::AgentReferencePicker {
            if let MouseEventKind::Down(MouseButton::Left) = mouse.kind {
                if let Some(idx) = self.agent_reference_picker_row_at(mouse.column, mouse.row) {
                    let enabled = self
                        .agent_reference_picker
                        .as_ref()
                        .is_some_and(|picker| picker.rows.get(idx).is_some_and(|row| row.live));
                    if enabled {
                        if let Some(picker) = self.agent_reference_picker.as_mut() {
                            picker.selected = idx;
                        }
                        return Some(MouseAction::AgentReferencePickerActivate);
                    }
                    return None;
                }
                let inside_popup = self
                    .agent_reference_picker_popup_rect()
                    .is_some_and(|popup| {
                        mouse.column >= popup.x
                            && mouse.column < popup.right()
                            && mouse.row >= popup.y
                            && mouse.row < popup.bottom()
                    });
                if !inside_popup {
                    self.close_agent_reference_picker();
                }
            }
            return None;
        }

        // The Taildrop picker is a topmost blocking overlay, and follows the
        // reference picker beside it: a row click highlights that machine and
        // sends to it, a click outside the box closes without sending, and
        // every other gesture is consumed rather than reaching the file manager
        // underneath.
        //
        // One deliberate difference: a click selects *and* sends, matching the
        // Enter key. Selecting on the first click and sending on a second would
        // make the two paths disagree, and the row highlight already says which
        // machine the click landed on.
        if self.mode == Mode::TailscaleSend {
            if let MouseEventKind::Down(MouseButton::Left) = mouse.kind {
                let area = self.view.terminal_area;
                let count = self
                    .tailscale_send
                    .as_ref()
                    .map_or(0, |picker| picker.devices.len());
                if let Some(index) = crate::ui::device_row_at(area, count, mouse.column, mouse.row)
                {
                    if let Some(picker) = self.tailscale_send.as_mut() {
                        picker.selected = index;
                    }
                    return Some(MouseAction::TailscaleSendActivate);
                }
                let inside =
                    crate::ui::tailscale_send_popup_rect(area, count).is_some_and(|popup| {
                        mouse.column >= popup.x
                            && mouse.column < popup.right()
                            && mouse.row >= popup.y
                            && mouse.row < popup.bottom()
                    });
                if !inside {
                    let _ = super::file_manager::close_tailscale_send(self);
                }
            }
            return None;
        }

        // An open context menu owns every mouse event except a re-targeting
        // right-click, which falls through to the shared open arms below.
        // Wheel, drag, and every other background gesture is consumed
        // fail-closed so hidden surfaces never act while the menu is open.
        if self.mode == Mode::ContextMenu
            && !matches!(mouse.kind, MouseEventKind::Down(MouseButton::Right))
        {
            match mouse.kind {
                MouseEventKind::Moved => {
                    let hovered = self.context_menu_item_at(mouse.column, mouse.row);
                    if let Some(menu) = &mut self.context_menu {
                        menu.list.hover(hovered);
                    }
                }
                MouseEventKind::Down(MouseButton::Left) => {
                    let item_idx = self.context_menu_item_at(mouse.column, mouse.row);
                    if item_idx.is_some_and(|idx| {
                        self.context_menu
                            .as_ref()
                            .is_some_and(|menu| !menu.item_enabled(idx))
                    }) {
                        return None;
                    }
                    if let Some(menu) = self.context_menu.take() {
                        if let Some(idx) = item_idx {
                            return Some(MouseAction::ContextMenu { menu, idx });
                        }
                        leave_modal(self);
                    }
                }
                _ => {}
            }
            return None;
        }

        if self.mode == Mode::OpenExistingWorktree {
            match mouse.kind {
                MouseEventKind::ScrollUp => {
                    if let Some(open) = &mut self.worktree_open {
                        open.select_previous_filtered();
                    }
                    return None;
                }
                MouseEventKind::ScrollDown => {
                    if let Some(open) = &mut self.worktree_open {
                        open.select_next_filtered();
                    }
                    return None;
                }
                _ => {}
            }
        }

        if matches!(
            self.mode,
            Mode::NewLinkedWorktree | Mode::OpenExistingWorktree | Mode::ConfirmRemoveWorktree
        ) && !matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left))
        {
            return None;
        }

        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                self.selection = None;
                self.selection_autoscroll = None;
                self.clear_chrome_press(source_id);

                if self.mode == Mode::ConfirmClose {
                    let popup = self.confirm_close_rect();
                    let inner = Rect::new(
                        popup.x + 1,
                        popup.y + 1,
                        popup.width.saturating_sub(2),
                        popup.height.saturating_sub(2),
                    );
                    let (confirm, cancel) = crate::ui::confirm_close_button_rects(inner);
                    match modal_action_from_buttons(
                        mouse.column,
                        mouse.row,
                        &[
                            (confirm, ModalAction::Confirm),
                            (cancel, ModalAction::Cancel),
                        ],
                    ) {
                        Some(ModalAction::Confirm) => {
                            return Some(MouseAction::ConfirmCloseAccept);
                        }
                        Some(ModalAction::Cancel) | None => confirm_close_cancel(self),
                        _ => {}
                    }
                    return None;
                }

                if self.mode == Mode::NewLinkedWorktree {
                    if let Some(inner) =
                        crate::ui::new_linked_worktree_inner_rect(self.screen_rect())
                    {
                        let (create, cancel) = crate::ui::new_linked_worktree_button_rects(inner);
                        match modal_action_from_buttons(
                            mouse.column,
                            mouse.row,
                            &[
                                (create, ModalAction::Confirm),
                                (cancel, ModalAction::Cancel),
                            ],
                        ) {
                            Some(ModalAction::Confirm) => {
                                self.request_submit_worktree_create = true;
                            }
                            Some(ModalAction::Cancel)
                                if !self
                                    .worktree_create
                                    .as_ref()
                                    .is_some_and(|create| create.creating) =>
                            {
                                self.worktree_create = None;
                                self.name_input.clear();
                                self.name_input_replace_on_type = false;
                                leave_modal(self);
                            }
                            _ => {}
                        }
                    }
                    return None;
                }

                if self.mode == Mode::OpenExistingWorktree {
                    if let Some(open) = self.worktree_open.as_ref() {
                        if let Some(inner) = crate::ui::open_existing_worktree_inner_rect(
                            self.screen_rect(),
                            open.entries.len(),
                        ) {
                            let filtered = open.filtered_indices();
                            let max_rows =
                                crate::ui::open_existing_worktree_max_visible_rows(inner);
                            let start =
                                crate::ui::open_existing_worktree_visible_start(open, max_rows);
                            if mouse.row == inner.y.saturating_add(1)
                                && mouse.column >= inner.x
                                && mouse.column < inner.x.saturating_add(inner.width)
                            {
                                if let Some(open) = &mut self.worktree_open {
                                    open.search_focused = true;
                                }
                                return None;
                            }
                            let row_idx = if rect_contains(inner, mouse.column, mouse.row) {
                                mouse
                                    .row
                                    .checked_sub(inner.y.saturating_add(3))
                                    .map(usize::from)
                                    .map(|row| row / 2)
                                    .filter(|row| *row < max_rows)
                                    .and_then(|row| filtered.get(start + row).copied())
                            } else {
                                None
                            };
                            if let Some(entry_idx) = row_idx {
                                if let Some(open) = &mut self.worktree_open {
                                    open.selected = entry_idx;
                                }
                                self.request_submit_worktree_open = true;
                                return None;
                            }

                            let (open_button, cancel) =
                                crate::ui::open_existing_worktree_button_rects(inner);
                            match modal_action_from_buttons(
                                mouse.column,
                                mouse.row,
                                &[
                                    (open_button, ModalAction::Confirm),
                                    (cancel, ModalAction::Cancel),
                                ],
                            ) {
                                Some(ModalAction::Confirm) => {
                                    self.request_submit_worktree_open = true;
                                }
                                Some(ModalAction::Cancel) => {
                                    self.worktree_open = None;
                                    leave_modal(self);
                                }
                                _ => {}
                            }
                        }
                    }
                    return None;
                }

                // TP-MOD-43: the dialog's three buttons, resolved against
                // the SAME pure geometry the renderer draws with.
                if self.mode == Mode::ConfirmDeleteModule {
                    if let Some(popup) = crate::ui::delete_module_popup_rect(self.screen_rect()) {
                        let inner = Rect::new(
                            popup.x + 1,
                            popup.y + 1,
                            popup.width.saturating_sub(2),
                            popup.height.saturating_sub(2),
                        );
                        let (delete, rename, cancel) = crate::ui::delete_module_button_rects(inner);
                        let hit = |rect: Rect| {
                            mouse.column >= rect.x
                                && mouse.column < rect.right()
                                && mouse.row >= rect.y
                                && mouse.row < rect.bottom()
                        };
                        if let Some(confirm) = self.module_delete.clone() {
                            if hit(delete) {
                                self.delete_confirmed_module();
                                leave_modal(self);
                            } else if hit(rename) {
                                self.module_delete = None;
                                let key = match confirm.target {
                                    crate::app::state::ModuleDeleteTarget::Node { key }
                                    | crate::app::state::ModuleDeleteTarget::Split { key } => key,
                                };
                                super::modal::open_rename_module_input(self, key, None);
                            } else if hit(cancel) {
                                self.module_delete = None;
                                leave_modal(self);
                            }
                        }
                    }
                    return None;
                }

                if self.mode == Mode::ConfirmRemoveWorktree {
                    if let Some(popup) = crate::ui::remove_worktree_popup_rect(self.screen_rect()) {
                        let inner = Rect::new(
                            popup.x + 1,
                            popup.y + 1,
                            popup.width.saturating_sub(2),
                            popup.height.saturating_sub(2),
                        );
                        let force_confirmation = self
                            .worktree_remove
                            .as_ref()
                            .is_some_and(|remove| remove.force_confirmation);
                        let (remove, cancel) =
                            crate::ui::remove_worktree_button_rects(inner, force_confirmation);
                        match modal_action_from_buttons(
                            mouse.column,
                            mouse.row,
                            &[
                                (remove, ModalAction::Confirm),
                                (cancel, ModalAction::Cancel),
                            ],
                        ) {
                            Some(ModalAction::Confirm) => {
                                self.request_submit_worktree_remove = true;
                            }
                            Some(ModalAction::Cancel)
                                if !self
                                    .worktree_remove
                                    .as_ref()
                                    .is_some_and(|remove| remove.removing) =>
                            {
                                self.worktree_remove = None;
                                leave_modal(self);
                            }
                            _ => {}
                        }
                    }
                    return None;
                }

                if matches!(
                    self.mode,
                    Mode::RenameWorkspace | Mode::RenameTab | Mode::RenamePane | Mode::RenameFile
                ) {
                    let action = self
                        .rename_modal_inner()
                        .map(crate::ui::rename_button_rects)
                        .and_then(|(save, clear, cancel)| {
                            modal_action_from_buttons(
                                mouse.column,
                                mouse.row,
                                &[
                                    (save, ModalAction::Save),
                                    (clear, ModalAction::Clear),
                                    (cancel, ModalAction::Cancel),
                                ],
                            )
                        })
                        .unwrap_or(ModalAction::Cancel);
                    return Some(MouseAction::RenameModal(action));
                }

                if self.on_sidebar_divider(mouse.column, mouse.row) {
                    self.begin_sidebar_resize(Position::new(mouse.column, mouse.row));
                    return None;
                }

                if self.on_sidebar_section_divider(mouse.column, mouse.row) {
                    self.drag = Some(DragState {
                        target: DragTarget::SidebarSectionDivider,
                    });
                    self.set_sidebar_section_split(mouse.row);
                    return None;
                }

                if !in_sidebar {
                    if let Some(border) = self.find_border_at(mouse.column, mouse.row) {
                        let grab_offset = match border.direction {
                            Direction::Horizontal => border.pos.saturating_sub(mouse.column),
                            Direction::Vertical => border.pos.saturating_sub(mouse.row),
                        };
                        self.drag = Some(DragState {
                            target: DragTarget::PaneSplit {
                                path: border.path.clone(),
                                direction: border.direction,
                                area: border.area,
                                grab_offset,
                            },
                        });
                        return None;
                    }

                    if let Some((pane_id, target)) =
                        self.scrollbar_target_at(terminal_runtimes, mouse.column, mouse.row)
                    {
                        self.focus_pane(pane_id);
                        match target {
                            ScrollbarClickTarget::Thumb { grab_row_offset } => {
                                self.drag = Some(DragState {
                                    target: DragTarget::PaneScrollbar {
                                        pane_id,
                                        grab_row_offset,
                                    },
                                });
                            }
                            ScrollbarClickTarget::Track { offset_from_bottom } => {
                                self.set_pane_scroll_offset(
                                    terminal_runtimes,
                                    pane_id,
                                    offset_from_bottom,
                                );
                            }
                        }
                        if self.mode != Mode::Terminal {
                            self.mode = Mode::Terminal;
                        }
                        return None;
                    }
                }

                if self.mode_bar_covers_tab_row(mouse.column, mouse.row) {
                    return None;
                }

                if self.on_tab_scroll_left_button(mouse.column, mouse.row) {
                    self.scroll_tabs_left();
                    return None;
                }
                if self.on_tab_scroll_right_button(mouse.column, mouse.row) {
                    self.scroll_tabs_right();
                    return None;
                }
                // TP-FTAB-INPUT-03/04: a strip entry activates the exact
                // instance its geometry names. A retired identity is inert and
                // is consumed without touching the terminal surface.
                if let Some(instance) = self.stage_tab_at(mouse.column, mouse.row) {
                    self.activate_stage_instance(instance);
                    self.mode = Mode::Terminal;
                    return None;
                }
                if let (Some(ws_idx), Some(tab_idx)) =
                    (self.active, self.tab_at(mouse.column, mouse.row))
                {
                    self.tab_presses.insert(
                        source_id,
                        TabPressState {
                            ws_idx,
                            tab_idx,
                            start_col: mouse.column,
                            start_row: mouse.row,
                        },
                    );
                    return None;
                }
                if self.on_new_tab_button(mouse.column, mouse.row) {
                    if self.prompt_new_tab_name {
                        open_new_tab_dialog(self);
                    } else {
                        self.request_new_tab = true;
                        self.mode = Mode::Terminal;
                    }
                    return None;
                }
                // TP-TAB-SPLIT-02: the buttons beside + split the focused
                // pane, riding the same api road the keyboard split takes.
                if self.on_split_right_button(mouse.column, mouse.row) {
                    self.mode = Mode::Terminal;
                    return Some(MouseAction::SplitFocusedPane {
                        direction: crate::api::schema::SplitDirection::Right,
                    });
                }
                if self.on_split_down_button(mouse.column, mouse.row) {
                    self.mode = Mode::Terminal;
                    return Some(MouseAction::SplitFocusedPane {
                        direction: crate::api::schema::SplitDirection::Down,
                    });
                }

                if in_sidebar {
                    if self.on_sidebar_toggle(mouse.column, mouse.row) {
                        self.set_sidebar_collapsed(!self.sidebar_collapsed);
                        return None;
                    }

                    if self.sidebar_collapsed {
                        if let Some(idx) = self.collapsed_workspace_at_row(mouse.row) {
                            self.mode = Mode::Terminal;
                            return Some(MouseAction::FocusWorkspace { ws_idx: idx });
                        }

                        if let Some((ws_idx, _tab_idx, pane_id)) =
                            self.collapsed_agent_detail_target_at(mouse.row)
                        {
                            self.mode = Mode::Terminal;
                            return Some(MouseAction::FocusPane { ws_idx, pane_id });
                        }
                        return None;
                    }

                    if let Some(tab) = self
                        .sidebar_tab_at(mouse.column, mouse.row)
                        // Only an unmodified primary click owns tab switching
                        // and Stage activation; modified clicks stay inert.
                        .filter(|_| mouse.modifiers.is_empty())
                    {
                        // Switching must stay I/O-free: the tab renders from
                        // the cache instantly and the scheduled poll (fast
                        // fingerprint check) refreshes it moments later.
                        // Synchronously re-reading sessions here blocked the
                        // click for the whole store read (~70MB observed).
                        if tab == crate::app::state::SidebarTab::Files {
                            // Converge on the shared dock authority so the
                            // launcher, dock click, and popover all open or
                            // reuse the same singleton Files Stage. Files owns
                            // the center only; the selected Spaces/Projects
                            // body remains the global runtime tracker.
                            self.activate_dock_app(crate::ui::surface_host::BuiltInAppId::Files);
                        } else {
                            self.sidebar_tab = tab;
                            self.request_file_manager_location_navigation = None;
                            if self.stage.surface_view()
                                == crate::ui::surface_host::StageSurfaceView::NativeFiles
                            {
                                // Symmetric client-local exit through the same
                                // shared authority; terminal runtimes are not
                                // touched.
                                self.activate_dock_app(
                                    crate::ui::surface_host::BuiltInAppId::Terminal,
                                );
                            }
                        }
                        return None;
                    }

                    // The Projects tab owns only the workspace-list BODY rows.
                    // Footer and agent-panel clicks must keep flowing to the
                    // shared handlers below (menu launcher, agent focus) —
                    // swallowing the whole sidebar here broke "click an agent
                    // to jump back to its chat".
                    if self.sidebar_tab == crate::app::state::SidebarTab::Projects {
                        // Scrollbar clicks win over row hit-tests: the track
                        // column is excluded from the row rects, but a press
                        // there must grab/jump the thumb, not toggle a row.
                        if let Some(target) =
                            self.projects_scrollbar_target_at(mouse.column, mouse.row)
                        {
                            match target {
                                ScrollbarClickTarget::Thumb { grab_row_offset } => {
                                    self.drag = Some(DragState {
                                        target: DragTarget::ProjectsScrollbar { grab_row_offset },
                                    });
                                }
                                ScrollbarClickTarget::Track { offset_from_bottom } => {
                                    self.set_projects_offset_from_bottom(offset_from_bottom);
                                }
                            }
                            return None;
                        }
                        let footer_y = self.sidebar_footer_rect().y;
                        if mouse.row < footer_y {
                            self.toggle_projects_row_at(mouse.column, mouse.row, mouse.modifiers);
                            return None;
                        }
                        let actives = self.sidebar_actives_toggle_rect();
                        if actives.width > 0
                            && mouse.row >= actives.y
                            && mouse.row < actives.y + actives.height
                            && mouse.column >= actives.x
                            && mouse.column < actives.x + actives.width
                        {
                            return Some(MouseAction::ToggleProjectsActives);
                        }
                        let new_button = self.sidebar_new_button_rect();
                        let on_new_button = mouse.row >= new_button.y
                            && mouse.row < new_button.y + new_button.height
                            && mouse.column >= new_button.x
                            && mouse.column < new_button.x + new_button.width;
                        if on_new_button {
                            // The footer " chat" button is wired in Task #10e;
                            // it must never create a workspace like Spaces'
                            // " new" underneath it would.
                            return None;
                        }
                    }

                    // TP-FOCUS-SW-04: the Spaces tab's own filter toggle sits
                    // in the footer slot the Projects tab keeps "actives" in,
                    // and only this tab reads it — one slot, two owners, and
                    // neither ever flips the other's filter.
                    if self.sidebar_tab == crate::app::state::SidebarTab::Spaces {
                        let focus = self.sidebar_focus_toggle_rect();
                        if focus.width > 0
                            && mouse.row >= focus.y
                            && mouse.row < focus.y + focus.height
                            && mouse.column >= focus.x
                            && mouse.column < focus.x + focus.width
                        {
                            return Some(MouseAction::ToggleSpacesFocus);
                        }
                    }

                    let new_button = self.sidebar_new_button_rect();
                    let on_new_button = mouse.row >= new_button.y
                        && mouse.row < new_button.y + new_button.height
                        && mouse.column >= new_button.x
                        && mouse.column < new_button.x + new_button.width;
                    if on_new_button {
                        return Some(MouseAction::NewWorkspace);
                    }

                    if let Some(target) =
                        self.workspace_list_scrollbar_target_at(mouse.column, mouse.row)
                    {
                        match target {
                            ScrollbarClickTarget::Thumb { grab_row_offset } => {
                                self.drag = Some(DragState {
                                    target: DragTarget::WorkspaceListScrollbar { grab_row_offset },
                                });
                            }
                            ScrollbarClickTarget::Track { offset_from_bottom } => {
                                self.set_workspace_list_offset_from_bottom(offset_from_bottom);
                            }
                        }
                        return None;
                    }

                    let cards = if self.view.workspace_card_areas.is_empty() {
                        crate::ui::compute_workspace_card_areas(self, self.view.sidebar_rect)
                    } else {
                        self.view.workspace_card_areas.clone()
                    };
                    // TP-DAILY-03/07: the daily section answers three separate
                    // gestures from three separate vectors — fold, open the
                    // rest, resume a chat — and none of them carries a
                    // `ws_idx`. Matched first because the section sits above
                    // the tree, and matched apart because a press on a header
                    // that resolved through a workspace index would fold
                    // whichever checkout happened to share the row.
                    if let Some(rect) = self.view.daily_header_area {
                        if mouse.row == rect.y
                            && mouse.column >= rect.x
                            && mouse.column < rect.x + rect.width
                        {
                            // TP-DAILY-10: the "+" is matched BEFORE the fold.
                            // Both live on this one row, and a fold that ran
                            // first would eat every press aimed at the plus —
                            // leaving the section able to show old chats and
                            // unable to start a new one, which is the half of
                            // the surface a person actually reaches for.
                            let plus = crate::ui::daily_new_chat_cell(rect);
                            if self.mouse_capture && plus.width > 0 && mouse.column == plus.x {
                                self.open_daily_new_chat_menu(mouse.column, mouse.row);
                                return None;
                            }
                            // TP-DAILY-12 / TP-DOTS-04: the "⋯" opens the very
                            // menu the right-click opens, matched before the
                            // fold for the same reason the plus is.
                            let dots = crate::ui::header_menu_cell(rect);
                            if self.mouse_capture && dots.width > 0 && mouse.column == dots.x {
                                self.open_daily_header_menu(mouse.column, mouse.row);
                                return None;
                            }
                            self.toggle_daily_section();
                            return None;
                        }
                    }
                    if let Some(rect) = self.view.daily_more_area {
                        if mouse.row == rect.y
                            && mouse.column >= rect.x
                            && mouse.column < rect.x + rect.width
                        {
                            self.toggle_full_daily_drawer();
                            return None;
                        }
                    }
                    // TP-DAILY-18: its own hit test, beside the chat switch's.
                    // Two switches sharing one rect would each fire the other's
                    // verb — the confusion TP-TREE-01 split rows to prevent.
                    if let Some(rect) = self.view.daily_more_workspaces_area {
                        if mouse.row == rect.y
                            && mouse.column >= rect.x
                            && mouse.column < rect.x + rect.width
                        {
                            self.daily_workspaces_expanded = !self.daily_workspaces_expanded;
                            return None;
                        }
                    }
                    if let Some(hit) = self
                        .view
                        .daily_chat_row_areas
                        .iter()
                        .find(|row| {
                            mouse.row == row.rect.y
                                && mouse.column >= row.rect.x
                                && mouse.column < row.rect.x + row.rect.width
                        })
                        .cloned()
                    {
                        self.open_daily_chat(&hit.session_id);
                        return None;
                    }
                    // TP-CHAT-MOVE-07: a container's chat row is pressable
                    // like any other, matched from its own vector because it
                    // carries a container key rather than a workspace index.
                    if let Some(hit) = self
                        .view
                        .module_chat_row_areas
                        .iter()
                        .find(|row| {
                            mouse.row == row.rect.y
                                && mouse.column >= row.rect.x
                                && mouse.column < row.rect.x + row.rect.width
                        })
                        .cloned()
                    {
                        self.open_module_chat(&hit.node_key, &hit.session_id);
                        return None;
                    }
                    // TP-DRAW-11: the "older chats" row opens the rest of
                    // this drawer and closes it again. Matched from its own
                    // vector, before the chat rows, for the same reason the
                    // chat rows are matched before the cards.
                    if let Some(hit) = self
                        .view
                        .workspace_more_chats_areas
                        .iter()
                        .find(|row| {
                            mouse.row == row.rect.y
                                && mouse.column >= row.rect.x
                                && mouse.column < row.rect.x + row.rect.width
                        })
                        .cloned()
                    {
                        self.toggle_full_chat_drawer(hit.ws_idx);
                        return None;
                    }
                    // A chat row resumes its session. Checked before the
                    // workspace cards because the two vectors describe
                    // different rows and only one of them can own a click.
                    if let Some(hit) = self
                        .view
                        .workspace_chat_row_areas
                        .iter()
                        .find(|row| {
                            mouse.row == row.rect.y
                                && mouse.column >= row.rect.x
                                && mouse.column < row.rect.x + row.rect.width
                        })
                        .cloned()
                    {
                        self.open_workspace_chat(hit.ws_idx, &hit.session_id);
                        return None;
                    }

                    // TP-TREE-14: a repository header folds and unfolds its
                    // group, and does nothing else — it is not a workspace, so
                    // pressing it must never switch to one. It is matched from
                    // its own vector for the same reason: resolving a header
                    // through a ws_idx would fold whichever workspace happened
                    // to share its position.
                    if let Some(head) = self
                        .view
                        .workspace_group_header_areas
                        .iter()
                        .find(|head| {
                            mouse.row == head.rect.y
                                && mouse.column >= head.rect.x
                                && mouse.column < head.rect.x + head.rect.width
                        })
                        .cloned()
                    {
                        // TP-DOTS-04: the header's "⋯" opens the menu the
                        // right-click already owns; the rest still folds.
                        let dots = crate::ui::header_menu_cell(head.rect);
                        if self.mouse_capture && dots.width > 0 && mouse.column == dots.x {
                            let collapsed = self.collapsed_space_keys.contains(&head.space_key);
                            self.context_menu = Some(ContextMenuState {
                                kind: ContextMenuKind::SpaceHeader {
                                    space_key: head.space_key,
                                    collapsed,
                                },
                                x: mouse.column,
                                y: mouse.row,
                                list: MenuListState::new(0),
                            });
                            self.enter_overlay_mode(Mode::ContextMenu);
                            return None;
                        }
                        // TP-DOTS-17: the header's "+" starts the module's
                        // "New branch..." — the same body the menu walks.
                        let plus = crate::ui::header_new_branch_cell(head.rect);
                        if self.mouse_capture && plus.width > 0 && mouse.column == plus.x {
                            super::modal::start_branch_from_module(self, head.space_key);
                            return None;
                        }
                        if !self.collapsed_space_keys.remove(&head.space_key) {
                            self.collapsed_space_keys.insert(head.space_key);
                        }
                        self.mark_session_dirty();
                        return None;
                    }

                    // TP-PROJ-GROUP-02: the project header folds and unfolds
                    // its project, and does nothing else — TP-TREE-14's rule,
                    // one level up, matched from its own vector for the same
                    // reason.
                    if let Some(head) = self
                        .view
                        .workspace_project_header_areas
                        .iter()
                        .find(|head| {
                            mouse.row == head.rect.y
                                && mouse.column >= head.rect.x
                                && mouse.column < head.rect.x + head.rect.width
                        })
                        .cloned()
                    {
                        // TP-DOTS-04: the node header's "⋯", same contract.
                        let dots = crate::ui::header_menu_cell(head.rect);
                        if self.mouse_capture && dots.width > 0 && mouse.column == dots.x {
                            let collapsed = self.node_folded(&head.project_key);
                            // TP-MOD-38: measured when the menu opens, like
                            // `deletable` beside it.
                            let needs_git_init = matches!(
                                crate::ui::module_branch_source(self, &head.project_key),
                                crate::ui::ModuleBranchSource::UninitializedDirectory(_)
                            );
                            self.context_menu = Some(ContextMenuState {
                                kind: ContextMenuKind::NodeHeader {
                                    deletable: self.managed_node_keys.contains(&head.project_key),
                                    node_key: head.project_key,
                                    collapsed,
                                    needs_git_init,
                                },
                                x: mouse.column,
                                y: mouse.row,
                                list: MenuListState::new(0),
                            });
                            self.enter_overlay_mode(Mode::ContextMenu);
                            return None;
                        }
                        // TP-DOTS-17: the node header's "+", same body.
                        let plus = crate::ui::header_new_branch_cell(head.rect);
                        if self.mouse_capture && plus.width > 0 && mouse.column == plus.x {
                            super::modal::start_branch_from_module(self, head.project_key);
                            return None;
                        }
                        if !self.unfold_node(&head.project_key) {
                            self.fold_node(head.project_key);
                        }
                        self.mark_session_dirty();
                        return None;
                    }

                    // TP-DOTS-04: the card's "⋯" is the visible door to the
                    // menu the right-click already opens — same builder, so
                    // the two roads can never drift apart.
                    if self.mouse_capture {
                        if let Some(card) = cards
                            .iter()
                            .find(|card| {
                                let cell = crate::ui::workspace_menu_cell(card.rect);
                                cell.width > 0 && mouse.row == cell.y && mouse.column == cell.x
                            })
                            .cloned()
                        {
                            self.open_workspace_row_menu(
                                terminal_runtimes,
                                card.ws_idx,
                                mouse.column,
                                mouse.row,
                            );
                            return None;
                        }
                    }

                    // "+" on the trailing edge starts a chat in that workspace.
                    if let Some(card) = cards
                        .iter()
                        .find(|card| {
                            let cell = crate::ui::workspace_new_chat_cell(card.rect);
                            cell.width > 0 && mouse.row == cell.y && mouse.column == cell.x
                        })
                        .cloned()
                    {
                        // A repository root can start two kinds of thing, so
                        // it asks; a linked worktree can only start a chat, so
                        // it just does it. Asking a question with one possible
                        // answer is a click the person did not need to make.
                        let offers_worktree = self
                            .workspaces
                            .get(card.ws_idx)
                            .is_some_and(|ws| ws.worktree_space.is_none());
                        if offers_worktree {
                            let cell = crate::ui::workspace_new_chat_cell(card.rect);
                            self.open_workspace_new_chat_menu(card.ws_idx, cell.x, cell.y);
                        } else {
                            self.request_workspace_chat(card.ws_idx);
                        }
                        return None;
                    }

                    // The drawer toggle leads the row, checked after the
                    // trailing "+" so neither can swallow the other's cell.
                    if let Some(card) = cards.iter().find(|card| {
                        let cell =
                            crate::ui::workspace_chat_toggle_cell(self, card.rect, card.ws_idx);
                        cell.width > 0 && mouse.row == cell.y && mouse.column == cell.x
                    }) {
                        self.toggle_chat_drawer(card.ws_idx);
                        return None;
                    }

                    // The group chevron used to live in column 0 of the group's
                    // parent checkout. TP-TREE-14 moved it to the repository's
                    // own row, so column 0 of a checkout row is now plain
                    // indentation and must fall through to the workspace press
                    // like any other part of the row.
                    if let Some(idx) = self.workspace_at_row(mouse.row) {
                        self.workspace_presses.insert(
                            source_id,
                            WorkspacePressState {
                                ws_idx: idx,
                                start_col: mouse.column,
                                start_row: mouse.row,
                            },
                        );
                        return None;
                    }

                    if self.on_agent_panel_sort_toggle(mouse.column, mouse.row) {
                        self.agent_panel_sort = match self.agent_panel_sort {
                            AgentPanelSort::Spaces => AgentPanelSort::Priority,
                            AgentPanelSort::Priority => AgentPanelSort::Spaces,
                        };
                        self.agent_panel_scroll = 0;
                        self.mark_session_dirty();
                        return None;
                    }

                    if let Some(target) =
                        self.agent_panel_scrollbar_target_at(mouse.column, mouse.row)
                    {
                        match target {
                            ScrollbarClickTarget::Thumb { grab_row_offset } => {
                                self.drag = Some(DragState {
                                    target: DragTarget::AgentPanelScrollbar { grab_row_offset },
                                });
                            }
                            ScrollbarClickTarget::Track { offset_from_bottom } => {
                                self.set_agent_panel_offset_from_bottom(offset_from_bottom);
                            }
                        }
                        return None;
                    }

                    if let Some((ws_idx, _tab_idx, pane_id)) =
                        self.agent_detail_target_at(mouse.row)
                    {
                        // TP-AGDND-01: the press is also a drag CANDIDATE —
                        // it becomes a drag only when the pointer leaves this
                        // row, so the focus below keeps being a plain click.
                        self.agent_panel_drag = Some(crate::app::state::AgentPanelDrag {
                            pane_id,
                            pressed_row: mouse.row,
                            current_row: mouse.row,
                            dragging: false,
                        });
                        self.mode = Mode::Terminal;
                        return Some(MouseAction::FocusPane { ws_idx, pane_id });
                    }

                    // TP-AGPANEL-23: a click on a grey row asks the server to
                    // revive that ghost; the server's state machine answers.
                    if let Some(agent_id) = self.closed_agent_target_at(mouse.row) {
                        return Some(MouseAction::ReviveClosedAgent { agent_id });
                    }
                } else if let Some(info) = self.pane_at(mouse.column, mouse.row).cloned() {
                    if self.mode != Mode::Terminal {
                        self.mode = Mode::Terminal;
                    }
                    // TP-SBS-FOCUS-01: clicking back into the terminal half
                    // returns the keyboard to it.
                    if let Some(sbs) = self.side_by_side.as_mut() {
                        sbs.focus = crate::app::state::SideBySideFocus::Left;
                    }

                    if self.forward_pane_mouse_button(terminal_runtimes, &info, mouse) {
                        self.selection = None;
                        self.selection_autoscroll = None;
                        return self.mouse_pane_focus_action(info.id);
                    }

                    let (row, col) = (
                        mouse.row - info.inner_rect.y,
                        mouse.column - info.inner_rect.x,
                    );
                    self.selection = Some(Selection::anchor(
                        info.id,
                        row,
                        col,
                        self.pane_scroll_metrics(terminal_runtimes, info.id),
                    ));
                    return self.mouse_pane_focus_action(info.id);
                } else if let Some(info) = self.view.pane_infos.iter().find(|p| {
                    mouse.column >= p.rect.x
                        && mouse.column < p.rect.x + p.rect.width
                        && mouse.row >= p.rect.y
                        && mouse.row < p.rect.y + p.rect.height
                }) {
                    let id = info.id;
                    if self.mode != Mode::Terminal {
                        self.mode = Mode::Terminal;
                    }
                    return self.mouse_pane_focus_action(id);
                }
            }

            MouseEventKind::Drag(MouseButton::Left) => {
                if self.selection.is_some() {
                    self.update_selection_drag(terminal_runtimes, mouse.column, mouse.row);
                    return None;
                }

                if (self.drag.is_none() || self.chrome_drag_owned_by_other(source_id))
                    && !self.chrome_press_pending(source_id)
                {
                    if let Some(info) = self.pane_mouse_target(mouse.column, mouse.row).cloned() {
                        if self.forward_pane_mouse_button(terminal_runtimes, &info, mouse) {
                            self.selection = None;
                            self.selection_autoscroll = None;
                            return None;
                        }
                    }
                }

                let workspace_drop_target = self.workspace_drop_target_at_row(mouse.row);
                let tab_drop_index = self.tab_drop_index_at(mouse.column, mouse.row);
                if self.drag.is_none() {
                    if let Some(press) = self.workspace_presses.get(&source_id) {
                        let delta_col = mouse.column.abs_diff(press.start_col);
                        let delta_row = mouse.row.abs_diff(press.start_row);
                        // TP-TREE-19: only a visible block root opens a
                        // reorder drag — the folded group's single card does,
                        // a group's second expanded member does not.
                        let can_reorder =
                            crate::ui::workspace_block_roots(self).contains(&press.ws_idx);
                        if workspace_drop_target.is_some()
                            && can_reorder
                            && delta_col.max(delta_row) >= WORKSPACE_DRAG_THRESHOLD
                        {
                            self.drag = Some(DragState {
                                target: DragTarget::WorkspaceReorder {
                                    source_id,
                                    source_ws_idx: press.ws_idx,
                                    drop_target: workspace_drop_target,
                                },
                            });
                        }
                    } else if let Some(press) = self.tab_presses.get(&source_id) {
                        let delta_col = mouse.column.abs_diff(press.start_col);
                        let delta_row = mouse.row.abs_diff(press.start_row);
                        // Require a real drop target before opening a reorder,
                        // so a report from off the tab bar cannot start a drag
                        // that has nowhere to land.
                        if tab_drop_index.is_some()
                            && delta_col.max(delta_row) >= TAB_DRAG_THRESHOLD
                        {
                            self.drag = Some(DragState {
                                target: DragTarget::TabReorder {
                                    source_id,
                                    ws_idx: press.ws_idx,
                                    source_tab_idx: press.tab_idx,
                                    insert_idx: tab_drop_index,
                                },
                            });
                        }
                    }
                }

                if let Some(DragState {
                    target:
                        DragTarget::WorkspaceReorder {
                            source_id: drag_source_id,
                            drop_target,
                            ..
                        },
                }) = &mut self.drag
                {
                    if *drag_source_id == source_id {
                        *drop_target = workspace_drop_target;
                    }
                } else if let Some(DragState {
                    target:
                        DragTarget::TabReorder {
                            source_id: drag_source_id,
                            ws_idx,
                            insert_idx,
                            ..
                        },
                }) = &mut self.drag
                {
                    if *drag_source_id == source_id && self.active == Some(*ws_idx) {
                        *insert_idx = tab_drop_index;
                    }
                } else if let Some(drag) = &self.drag {
                    match &drag.target {
                        DragTarget::WorkspaceReorder { .. } | DragTarget::TabReorder { .. } => {}
                        DragTarget::WorkspaceListScrollbar { grab_row_offset } => {
                            if let Some(offset_from_bottom) =
                                self.workspace_list_offset_for_drag_row(mouse.row, *grab_row_offset)
                            {
                                self.set_workspace_list_offset_from_bottom(offset_from_bottom);
                            }
                        }
                        DragTarget::AgentPanelScrollbar { grab_row_offset } => {
                            if let Some(offset_from_bottom) =
                                self.agent_panel_offset_for_drag_row(mouse.row, *grab_row_offset)
                            {
                                self.set_agent_panel_offset_from_bottom(offset_from_bottom);
                            }
                        }
                        DragTarget::ProjectsScrollbar { grab_row_offset } => {
                            if let Some(offset_from_bottom) =
                                self.projects_offset_for_drag_row(mouse.row, *grab_row_offset)
                            {
                                self.set_projects_offset_from_bottom(offset_from_bottom);
                            }
                        }
                        DragTarget::PaneSplit {
                            path,
                            direction,
                            area,
                            grab_offset,
                        } => {
                            let ratio = match direction {
                                Direction::Horizontal => {
                                    (mouse
                                        .column
                                        .saturating_add(*grab_offset)
                                        .saturating_sub(area.x))
                                        as f32
                                        / area.width.max(1) as f32
                                }
                                Direction::Vertical => {
                                    (mouse
                                        .row
                                        .saturating_add(*grab_offset)
                                        .saturating_sub(area.y))
                                        as f32
                                        / area.height.max(1) as f32
                                }
                            };
                            let ratio = ratio.clamp(0.1, 0.9);
                            let path = path.clone();
                            return Some(MouseAction::SetSplitRatio { path, ratio });
                        }
                        DragTarget::PaneScrollbar {
                            pane_id,
                            grab_row_offset,
                        } => {
                            if let Some(offset_from_bottom) = self.scrollbar_offset_for_pane_row(
                                terminal_runtimes,
                                *pane_id,
                                mouse.row,
                                *grab_row_offset,
                            ) {
                                self.set_pane_scroll_offset(
                                    terminal_runtimes,
                                    *pane_id,
                                    offset_from_bottom,
                                );
                            }
                        }
                        DragTarget::SidebarDivider => {
                            self.preview_sidebar_resize(Position::new(mouse.column, mouse.row));
                        }
                        DragTarget::SidebarSectionDivider => {
                            self.set_sidebar_section_split(mouse.row);
                        }
                        DragTarget::ReleaseNotesScrollbar { .. }
                        | DragTarget::ProductAnnouncementScrollbar { .. }
                        | DragTarget::KeybindHelpScrollbar { .. }
                        | DragTarget::ChatWorkLogScrollbar { .. } => {}
                    }
                }
            }

            MouseEventKind::Up(MouseButton::Left) => {
                // Mouse-up either finishes a drag selection or releases after a
                // double-click word selection; the latter is already finalized.
                if let Some(selection) = self.selection.as_ref() {
                    let was_click = selection.was_just_click();
                    let was_finalized = selection.is_finalized();

                    self.clear_chrome_press(source_id);
                    self.drag = None;
                    self.selection_autoscroll = None;
                    if was_click {
                        self.selection = None;
                    } else if was_finalized {
                        // Double-click already finalized this word selection.
                    } else if self.copy_on_select {
                        self.copy_selection(terminal_runtimes);
                    } else if let Some(selection) = self.selection.as_mut() {
                        selection.finish();
                    }
                    return None;
                }

                let foreign_chrome_drag = self.chrome_drag_owned_by_other(source_id);
                if (self.drag.is_none() || foreign_chrome_drag)
                    && !self.chrome_press_pending(source_id)
                {
                    if let Some(info) = self.pane_mouse_target(mouse.column, mouse.row).cloned() {
                        if self.forward_pane_mouse_button(terminal_runtimes, &info, mouse) {
                            self.selection = None;
                            self.selection_autoscroll = None;
                            return None;
                        }
                    }
                }

                let workspace_press = self.workspace_presses.remove(&source_id);
                let tab_press = self.tab_presses.remove(&source_id);
                if foreign_chrome_drag {
                    return self.chrome_press_action(workspace_press, tab_press);
                }

                match self.drag.take() {
                    Some(DragState {
                        target:
                            DragTarget::WorkspaceReorder {
                                source_ws_idx,
                                drop_target: Some(drop_target),
                                ..
                            },
                    }) => {
                        if let Some(params) =
                            self.workspace_move_block_params(source_ws_idx, drop_target)
                        {
                            if self
                                .workspaces
                                .get(source_ws_idx)
                                .is_some_and(|workspace| workspace.worktree_space().is_some())
                            {
                                return Some(MouseAction::MoveWorkspaceBlock { params });
                            }
                            let insert_idx = params
                                .before_workspace_id
                                .as_ref()
                                .and_then(|id| {
                                    self.workspaces
                                        .iter()
                                        .position(|workspace| workspace.id == *id)
                                })
                                .unwrap_or(self.workspaces.len());
                            return Some(MouseAction::MoveWorkspace {
                                source_ws_idx,
                                insert_idx,
                            });
                        }
                    }
                    Some(DragState {
                        target:
                            DragTarget::TabReorder {
                                ws_idx,
                                source_tab_idx,
                                insert_idx: Some(insert_idx),
                                ..
                            },
                    }) => {
                        if self.active == Some(ws_idx) {
                            self.mode = Mode::Terminal;
                            return Some(MouseAction::MoveTab {
                                ws_idx,
                                source_tab_idx,
                                insert_idx,
                            });
                        }
                    }
                    Some(DragState {
                        target: DragTarget::SidebarDivider,
                    }) => {
                        self.commit_sidebar_resize();
                    }
                    Some(_) => {}
                    None => return self.chrome_press_action(workspace_press, tab_press),
                }
            }

            MouseEventKind::Up(MouseButton::Middle) | MouseEventKind::Drag(MouseButton::Middle)
                if !in_sidebar =>
            {
                if let Some(info) = self.pane_mouse_target(mouse.column, mouse.row).cloned() {
                    let _ = self.forward_pane_mouse_button(terminal_runtimes, &info, mouse);
                }
            }

            MouseEventKind::ScrollUp | MouseEventKind::ScrollDown
                if self.mode_bar_covers_tab_row(mouse.column, mouse.row) => {}

            MouseEventKind::ScrollUp | MouseEventKind::ScrollDown
                if self.on_tab_bar(mouse.column, mouse.row) =>
            {
                match mouse.kind {
                    MouseEventKind::ScrollUp => {
                        if let Some(ws) = self.active.and_then(|i| self.workspaces.get(i)) {
                            if !ws.tabs.is_empty() {
                                let prev = if ws.active_tab_index() == 0 {
                                    ws.tabs.len() - 1
                                } else {
                                    ws.active_tab_index() - 1
                                };
                                return Some(MouseAction::FocusTab { tab_idx: prev });
                            }
                        }
                    }
                    MouseEventKind::ScrollDown => {
                        if let Some(ws) = self.active.and_then(|i| self.workspaces.get(i)) {
                            if !ws.tabs.is_empty() {
                                let next = (ws.active_tab_index() + 1) % ws.tabs.len();
                                return Some(MouseAction::FocusTab { tab_idx: next });
                            }
                        }
                    }
                    _ => {}
                }
            }

            MouseEventKind::ScrollUp | MouseEventKind::ScrollDown
                if !in_sidebar && self.scroll_selection_with_wheel(terminal_runtimes, mouse) => {}

            MouseEventKind::ScrollUp | MouseEventKind::ScrollDown if !in_sidebar => {
                self.selection = None;
                self.selection_autoscroll = None;
                self.handle_terminal_wheel(terminal_runtimes, mouse);
            }

            MouseEventKind::ScrollLeft | MouseEventKind::ScrollRight
                if self.mode == Mode::Terminal && !in_sidebar =>
            {
                if let Some(info) = self.pane_at(mouse.column, mouse.row).cloned() {
                    self.forward_pane_reported_wheel(terminal_runtimes, &info, mouse);
                }
            }

            MouseEventKind::ScrollUp if in_sidebar => {
                let agent_area = self.agent_panel_rect();
                let over_agent_panel = agent_area != Rect::default()
                    && mouse.row >= agent_area.y
                    && mouse.row < agent_area.y + agent_area.height;
                if over_agent_panel {
                    if crate::ui::should_show_scrollbar(crate::ui::agent_panel_scroll_metrics(
                        self, agent_area,
                    )) {
                        self.scroll_agent_panel(-1);
                    }
                } else if self.sidebar_tab == crate::app::state::SidebarTab::Projects {
                    // The Projects tab owns the top section: the wheel scrolls
                    // its rows and must never move the hidden Spaces selection.
                    if crate::ui::should_show_scrollbar(crate::ui::projects_scroll_metrics(
                        self,
                        self.workspace_list_rect(),
                    )) {
                        self.scroll_projects_list(-1);
                    }
                } else if crate::ui::should_show_scrollbar(
                    crate::ui::workspace_list_scroll_metrics(self, self.workspace_list_rect()),
                ) {
                    self.scroll_workspace_list(-1);
                } else {
                    self.move_selected_workspace_by_visible_delta(-1);
                }
            }
            MouseEventKind::ScrollDown if in_sidebar => {
                let agent_area = self.agent_panel_rect();
                let over_agent_panel = agent_area != Rect::default()
                    && mouse.row >= agent_area.y
                    && mouse.row < agent_area.y + agent_area.height;
                if over_agent_panel {
                    if crate::ui::should_show_scrollbar(crate::ui::agent_panel_scroll_metrics(
                        self, agent_area,
                    )) {
                        self.scroll_agent_panel(1);
                    }
                } else if self.sidebar_tab == crate::app::state::SidebarTab::Projects {
                    if crate::ui::should_show_scrollbar(crate::ui::projects_scroll_metrics(
                        self,
                        self.workspace_list_rect(),
                    )) {
                        self.scroll_projects_list(1);
                    }
                } else if crate::ui::should_show_scrollbar(
                    crate::ui::workspace_list_scroll_metrics(self, self.workspace_list_rect()),
                ) {
                    self.scroll_workspace_list(1);
                } else {
                    self.move_selected_workspace_by_visible_delta(1);
                }
            }

            MouseEventKind::Moved if self.mode == Mode::Terminal && !in_sidebar => {
                if let Some(info) = self.pane_at(mouse.column, mouse.row).cloned() {
                    let _ = self.forward_pane_mouse_motion(terminal_runtimes, &info, mouse);
                }
            }

            MouseEventKind::Down(MouseButton::Right) if in_sidebar && !self.sidebar_collapsed => {
                self.clear_chrome_press(source_id);
                if self
                    .workspace_list_scrollbar_target_at(mouse.column, mouse.row)
                    .is_some()
                {
                    return None;
                }
                // TP-AGPANEL-03: the agents panel is the sidebar's lower half
                // on every tab, so its rows are matched before the per-tab
                // roads below — and matched through their own resolver, which
                // is bounded to the panel body and cannot claim a row that
                // belongs to the list above it.
                // TP-AGPANEL-45: a headstone answers a right-click with its
                // own menu. Matched before the living rows below for the same
                // reason the living rows are matched before the tree: one
                // vector may own a row, and the graveyard's resolver is the
                // only one bounded to the ghost cards.
                if let Some(agent_id) = self.closed_agent_target_at(mouse.row) {
                    self.context_menu = Some(ContextMenuState {
                        kind: ContextMenuKind::ClosedAgent { agent_id },
                        x: mouse.column,
                        y: mouse.row,
                        list: MenuListState::new(0),
                    });
                    self.enter_overlay_mode(Mode::ContextMenu);
                    return None;
                }
                if let Some((ws_idx, tab_idx, pane_id)) = self.agent_detail_target_at(mouse.row) {
                    // TP-AGPANEL-28: the chat identity is read HERE, while the
                    // row under the cursor is still the row the menu is for.
                    let session_id = self
                        .workspaces
                        .get(ws_idx)
                        .and_then(|ws| ws.tabs.get(tab_idx))
                        .and_then(|tab| tab.resumed_session_id.clone());
                    self.context_menu = Some(ContextMenuState {
                        kind: ContextMenuKind::AgentEntry {
                            ws_idx,
                            tab_idx,
                            pane_id,
                            session_id,
                        },
                        x: mouse.column,
                        y: mouse.row,
                        list: MenuListState::new(0),
                    });
                    self.enter_overlay_mode(Mode::ContextMenu);
                    return None;
                }
                // The Projects tab owns its rows: right-click on a project
                // header or its "+" button opens the agent selector; other
                // rows are inert. Never fall through to the workspace-card
                // menu — those cards are not visible on this tab.
                if self.sidebar_tab == crate::app::state::SidebarTab::Projects {
                    if let Some(
                        crate::app::state::ProjectRowKind::Project { proj_idx }
                        | crate::app::state::ProjectRowKind::NewChat { proj_idx },
                    ) = self.project_row_kind_at(mouse.column, mouse.row)
                    {
                        self.open_project_new_chat_menu(proj_idx, mouse.column, mouse.row);
                    }
                    return None;
                }
                if self.sidebar_tab == crate::app::state::SidebarTab::Files {
                    let list = self.workspace_list_rect();
                    if mouse.column >= list.x
                        && mouse.column < list.x.saturating_add(list.width)
                        && mouse.row >= list.y
                        && mouse.row < list.y.saturating_add(list.height)
                    {
                        return None;
                    }
                }
                // TP-CHAT-MOVE-08: a daily row owns a menu too, matched from
                // its own vector and before the drawer rows — the same
                // one-vector-owns-the-row rule the left press already keeps.
                // Without this the daily section is the one place a chat can
                // be seen and not filed, which is backwards: those are exactly
                // the chats that belong to no checkout yet.
                if let Some(hit) = self
                    .view
                    .daily_chat_row_areas
                    .iter()
                    .find(|row| {
                        mouse.row == row.rect.y
                            && mouse.column >= row.rect.x
                            && mouse.column < row.rect.x + row.rect.width
                    })
                    .cloned()
                {
                    {
                        // The drawn row carries its identity now.
                        let session_id = hit.session_id.clone();
                        let has_move = self.chat_move_overrides.contains_key(&session_id);
                        let has_live = self.find_resumed_chat_tab(&session_id).is_some();
                        // TP-CHAT-MOVE-11: resolved when the menu opens, like
                        // its siblings, so a config reload underneath an open
                        // menu cannot turn the offer into a no-op.
                        let has_modules = !self.module_move_target_entries().is_empty();
                        // TP-WORKLOG-02: resolved at menu-open like its siblings.
                        let has_worklog = self.chat_worklog.has_confirmed(&session_id);
                        self.context_menu = Some(ContextMenuState {
                            kind: ContextMenuKind::WorkspaceChat {
                                ws_idx: None,
                                session_id,
                                has_move,
                                has_live,
                                has_modules,
                                has_worklog,
                            },
                            x: mouse.column,
                            y: mouse.row,
                            list: MenuListState::new(0),
                        });
                        self.enter_overlay_mode(Mode::ContextMenu);
                    }
                    return None;
                }
                // TP-CHAT-MOVE-04: a chat row owns its own menu — checked
                // before the workspace cards for the same reason the click
                // road is: only one vector may own the row.
                if let Some(hit) = self
                    .view
                    .workspace_chat_row_areas
                    .iter()
                    .find(|row| {
                        mouse.row == row.rect.y
                            && mouse.column >= row.rect.x
                            && mouse.column < row.rect.x + row.rect.width
                    })
                    .cloned()
                {
                    {
                        // The drawn row carries its identity now.
                        let session_id = hit.session_id.clone();
                        let has_move = self.chat_move_overrides.contains_key(&session_id);
                        // TP-AGPANEL-05: the close verb is offered only when
                        // this chat still has a tab running behind it.
                        let has_live = self.find_resumed_chat_tab(&session_id).is_some();
                        // TP-CHAT-MOVE-11
                        let has_modules = !self.module_move_target_entries().is_empty();
                        // TP-WORKLOG-02: resolved at menu-open like its siblings.
                        let has_worklog = self.chat_worklog.has_confirmed(&session_id);
                        self.context_menu = Some(ContextMenuState {
                            kind: ContextMenuKind::WorkspaceChat {
                                ws_idx: Some(hit.ws_idx),
                                session_id,
                                has_move,
                                has_live,
                                has_modules,
                                has_worklog,
                            },
                            x: mouse.column,
                            y: mouse.row,
                            list: MenuListState::new(0),
                        });
                        self.enter_overlay_mode(Mode::ContextMenu);
                    }
                    return None;
                }
                // TP-DAILY-12 / TP-DOTS-04: the area's header answers a
                // right-click with the same menu its "⋯" opens. Matched with
                // the other headers, from its own rect, because it belongs to
                // no workspace and would otherwise resolve through one.
                if let Some(rect) = self.view.daily_header_area {
                    if mouse.row == rect.y
                        && mouse.column >= rect.x
                        && mouse.column < rect.x + rect.width
                    {
                        self.open_daily_header_menu(mouse.column, mouse.row);
                        return None;
                    }
                }
                // TP-DOTS-02: the tree's header rows own menus of their own —
                // matched from their own vectors, like the click road, so a
                // header can never resolve through a workspace sharing its
                // row. The bucket header folds; the node header also creates.
                if let Some(head) = self
                    .view
                    .workspace_group_header_areas
                    .iter()
                    .find(|head| {
                        mouse.row == head.rect.y
                            && mouse.column >= head.rect.x
                            && mouse.column < head.rect.x + head.rect.width
                    })
                    .cloned()
                {
                    let collapsed = self.collapsed_space_keys.contains(&head.space_key);
                    self.context_menu = Some(ContextMenuState {
                        kind: ContextMenuKind::SpaceHeader {
                            space_key: head.space_key,
                            collapsed,
                        },
                        x: mouse.column,
                        y: mouse.row,
                        list: MenuListState::new(0),
                    });
                    self.enter_overlay_mode(Mode::ContextMenu);
                    return None;
                }
                if let Some(head) = self
                    .view
                    .workspace_project_header_areas
                    .iter()
                    .find(|head| {
                        mouse.row == head.rect.y
                            && mouse.column >= head.rect.x
                            && mouse.column < head.rect.x + head.rect.width
                    })
                    .cloned()
                {
                    let collapsed = self.node_folded(&head.project_key);
                    // TP-MOD-38
                    let needs_git_init = matches!(
                        crate::ui::module_branch_source(self, &head.project_key),
                        crate::ui::ModuleBranchSource::UninitializedDirectory(_)
                    );
                    self.context_menu = Some(ContextMenuState {
                        kind: ContextMenuKind::NodeHeader {
                            deletable: self.managed_node_keys.contains(&head.project_key),
                            node_key: head.project_key,
                            collapsed,
                            needs_git_init,
                        },
                        x: mouse.column,
                        y: mouse.row,
                        list: MenuListState::new(0),
                    });
                    self.enter_overlay_mode(Mode::ContextMenu);
                    return None;
                }
                if let Some(idx) = self.workspace_at_row(mouse.row) {
                    self.open_workspace_row_menu(terminal_runtimes, idx, mouse.column, mouse.row);
                    return None;
                }
                // TP-MOD-31: the blank below the tree. Matched LAST, after
                // every row vector has had its say, because a blank menu that
                // ran first would shadow every right-click this sidebar has.
                // Until now the empty space answered nothing at all — and it
                // is the only surface from which a container can be made
                // without already standing inside one.
                self.open_sidebar_blank_menu(mouse.column, mouse.row);
            }

            MouseEventKind::Down(MouseButton::Right)
                if !self.mode_bar_covers_tab_row(mouse.column, mouse.row)
                    && self.tab_at(mouse.column, mouse.row).is_some() =>
            {
                if let (Some(ws_idx), Some(tab_idx)) =
                    (self.active, self.tab_at(mouse.column, mouse.row))
                {
                    self.context_menu = Some(ContextMenuState {
                        kind: ContextMenuKind::Tab { ws_idx, tab_idx },
                        x: mouse.column,
                        y: mouse.row,
                        list: MenuListState::new(0),
                    });
                    self.enter_overlay_mode(Mode::ContextMenu);
                }
            }

            MouseEventKind::Down(MouseButton::Right) if !in_sidebar => {
                if let Some(info) = self.pane_mouse_target(mouse.column, mouse.row).cloned() {
                    let ws_idx = self.active?;
                    let tab_idx = self
                        .workspaces
                        .get(ws_idx)
                        .map(|ws| ws.active_tab_index())?;
                    let previous_focused_pane_id = self
                        .workspaces
                        .get(ws_idx)
                        .and_then(|ws| ws.focused_pane_id());
                    let source_pane_id =
                        previous_focused_pane_id.filter(|pane_id| *pane_id != info.id);
                    let pane_state = self
                        .workspaces
                        .get(ws_idx)
                        .and_then(|ws| ws.pane_state(info.id));
                    let has_manual_label = pane_state
                        .and_then(|pane| self.terminals.get(&pane.attached_terminal_id))
                        .and_then(|terminal| terminal.manual_label.as_ref())
                        .is_some();
                    let right_click_passthrough =
                        pane_state.is_some_and(|pane| pane.right_click_passthrough);
                    self.context_menu = Some(ContextMenuState {
                        kind: ContextMenuKind::Pane {
                            ws_idx,
                            tab_idx,
                            pane_id: info.id,
                            source_pane_id,
                            has_manual_label,
                            right_click_passthrough,
                        },
                        x: mouse.column,
                        y: mouse.row,
                        list: MenuListState::new(0),
                    });
                    self.enter_overlay_mode(Mode::ContextMenu);
                }
            }

            _ => {}
        }

        None
    }

    fn handle_mobile_mouse(&mut self, mouse: MouseEvent) -> MobileMouseResult {
        // Every mobile hit test below reads these coordinates, so the edge
        // correction belongs here rather than at each rect (TP-MOB-65).
        let screen = crate::ui::mobile_screen_rect(self);
        let (column, row) = crate::ui::clamp_to_mobile_screen(screen, mouse.column, mouse.row);
        let mouse = MouseEvent {
            column,
            row,
            ..mouse
        };
        if self.mode == Mode::Navigate {
            match mouse.kind {
                MouseEventKind::ScrollUp => {
                    self.scroll_mobile_switcher_at(mouse.column, mouse.row, -1);
                    return MobileMouseResult::Consumed;
                }
                MouseEventKind::ScrollDown => {
                    self.scroll_mobile_switcher_at(mouse.column, mouse.row, 1);
                    return MobileMouseResult::Consumed;
                }
                MouseEventKind::Down(MouseButton::Left) => {}
                _ => return MobileMouseResult::Consumed,
            }
        } else if !matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
            return MobileMouseResult::Ignored;
        }

        // The header stays visible with a drawer open, so its buttons keep
        // working: pressing the open drawer's own button closes it, which is
        // how a toggle is expected to behave.
        let hits = self.view.mobile_header_hits;
        if matches!(self.mode, Mode::Terminal | Mode::Resize | Mode::Navigate) {
            let pressed = if rect_contains(hits.spaces_menu, mouse.column, mouse.row) {
                Some(crate::app::state::MobileDrawer::Spaces)
            } else if rect_contains(hits.tabs_menu, mouse.column, mouse.row)
                || rect_contains(hits.tab_strip, mouse.column, mouse.row)
            {
                // The strip is the same intent as the button beside it, and the
                // larger target for it.
                Some(crate::app::state::MobileDrawer::Tabs)
            } else {
                None
            };
            if let Some(drawer) = pressed {
                self.toggle_mobile_drawer(drawer);
                return MobileMouseResult::Consumed;
            }
        }

        if self.mode != Mode::Navigate {
            return MobileMouseResult::Ignored;
        }

        // The uncovered strip is the drawer's way out: tapping it closes the
        // drawer and does not reach the terminal underneath.
        let areas = crate::ui::mobile_drawer_areas(self);
        if rect_contains(areas.scrim, mouse.column, mouse.row) {
            self.close_mobile_drawer();
            return MobileMouseResult::Consumed;
        }

        match crate::ui::mobile_drawer_target_at(self, mouse.column, mouse.row) {
            Some(crate::ui::MobileSwitcherTarget::NewWorkspace) => {
                return MobileMouseResult::Action(MouseAction::NewWorkspace);
            }
            Some(crate::ui::MobileSwitcherTarget::Workspace(ws_idx)) => {
                // The row you are already on folds its own history instead of
                // switching, and the drawer stays open (TP-MOB-69).
                if Some(ws_idx) == self.active {
                    self.toggle_mobile_active_chats();
                    return MobileMouseResult::Consumed;
                }
                self.close_mobile_drawer();
                return MobileMouseResult::Action(MouseAction::FocusWorkspace { ws_idx });
            }
            Some(crate::ui::MobileSwitcherTarget::NewTab) => {
                if self.prompt_new_tab_name {
                    open_new_tab_dialog(self);
                } else {
                    self.request_new_tab = true;
                    self.close_mobile_drawer();
                }
            }
            Some(crate::ui::MobileSwitcherTarget::Tab(tab_idx)) => {
                self.close_mobile_drawer();
                return MobileMouseResult::Action(MouseAction::FocusTab { tab_idx });
            }
            Some(crate::ui::MobileSwitcherTarget::Agent {
                ws_idx,
                tab_idx: _,
                pane_id,
            }) => {
                self.close_mobile_drawer();
                return MobileMouseResult::Action(MouseAction::FocusPane { ws_idx, pane_id });
            }
            Some(crate::ui::MobileSwitcherTarget::ToggleSpaceGroup { group_idx }) => {
                self.toggle_mobile_space_group(group_idx);
            }
            Some(crate::ui::MobileSwitcherTarget::ToggleProjectGroup { project_group_idx }) => {
                self.toggle_mobile_project_group(project_group_idx);
            }
            Some(
                target @ (crate::ui::MobileSwitcherTarget::DrawerSegment(_)
                | crate::ui::MobileSwitcherTarget::ToggleProject { .. }
                | crate::ui::MobileSwitcherTarget::ProjectChat { .. }
                | crate::ui::MobileSwitcherTarget::NewChatInProject { .. }
                | crate::ui::MobileSwitcherTarget::RowMenu { .. }),
            ) => {
                // The projects segment's targets behave identically from a
                // tap and from the keyboard cursor, so both roads go through
                // the one application point (TP-MOB-91).
                self.apply_mobile_switcher_target(target);
            }
            Some(crate::ui::MobileSwitcherTarget::Chat { ws_idx, chat_idx }) => {
                self.close_mobile_drawer();
                // Bridge until the mobile targets carry identity themselves:
                // resolve the id at the press site. The drawer is modal, so
                // the list rarely moves under it — the desktop rows, which do
                // race the polls, already resolve by identity.
                if let Some(session_id) = self
                    .workspaces
                    .get(ws_idx)
                    .map(|ws| crate::persist::workspace_chats::ledger_key(&ws.identity_cwd))
                    .and_then(|key| self.workspace_chat_rows.get(&key))
                    .and_then(|rows| rows.get(chat_idx))
                    .map(|row| row.session_id.clone())
                {
                    self.open_workspace_chat(ws_idx, &session_id);
                }
            }
            Some(crate::ui::MobileSwitcherTarget::DailyChat { chat_idx }) => {
                self.close_mobile_drawer();
                if let Some(session_id) = self
                    .daily_chat_cwd
                    .as_ref()
                    .map(|cwd| crate::persist::workspace_chats::ledger_key(cwd))
                    .and_then(|key| self.workspace_chat_rows.get(&key))
                    .and_then(|rows| rows.get(chat_idx))
                    .map(|row| row.session_id.clone())
                {
                    self.open_daily_chat(&session_id);
                }
            }
            Some(crate::ui::MobileSwitcherTarget::ModuleChat { node_key, chat_idx }) => {
                self.close_mobile_drawer();
                let key = crate::persist::workspace_chats::module_ledger_key(&node_key);
                if let Some(session_id) = self
                    .workspace_chat_rows
                    .get(&key)
                    .and_then(|rows| rows.get(chat_idx))
                    .map(|row| row.session_id.clone())
                {
                    self.open_module_chat(&node_key, &session_id);
                }
            }
            Some(crate::ui::MobileSwitcherTarget::ToggleBranchChats { ws_idx }) => {
                self.toggle_mobile_branch_chats(ws_idx);
            }
            Some(crate::ui::MobileSwitcherTarget::NewChatIn { ws_idx }) => {
                self.close_mobile_drawer();
                self.request_workspace_chat(ws_idx);
            }
            Some(crate::ui::MobileSwitcherTarget::ToggleSelectMode) => {
                self.toggle_mobile_select_mode();
            }
            Some(crate::ui::MobileSwitcherTarget::Menu(action_idx)) => {
                let actions = global_menu_actions(self);
                if let Some(action) = actions.get(action_idx).copied() {
                    apply_global_menu_action(self, action);
                }
            }
            None => {}
        }

        MobileMouseResult::Consumed
    }

    fn scroll_mobile_switcher_at(&mut self, _col: u16, _row: u16, delta: i16) {
        let max_scroll = crate::ui::mobile_drawer_max_scroll(self);
        apply_scroll(
            &mut self.mobile_switcher_scroll,
            delta.saturating_mul(2),
            max_scroll,
        );
    }

    pub(super) fn screen_rect(&self) -> Rect {
        let sidebar = self.view.sidebar_rect;
        let terminal = self.view.terminal_area;
        let x = sidebar.x.min(terminal.x);
        let y = sidebar.y.min(terminal.y);
        let right = (sidebar.x + sidebar.width).max(terminal.x + terminal.width);
        let bottom = (sidebar.y + sidebar.height).max(terminal.y + terminal.height);
        Rect::new(x, y, right.saturating_sub(x), bottom.saturating_sub(y))
    }

    pub(crate) fn context_menu_rect(&self) -> Option<Rect> {
        let menu = self.context_menu.as_ref()?;
        let screen = self.screen_rect();
        let max_item_w = menu
            .items()
            .iter()
            .map(|item| crate::ui::display_width_u16(item))
            .max()
            .unwrap_or(0);
        let menu_w = (max_item_w + 4).max(14).min(screen.width.max(1));
        let menu_h = (menu.items().len() as u16 + 2).min(screen.height.max(1));
        let x = menu.x.min(screen.x + screen.width.saturating_sub(menu_w));
        let y = menu.y.min(screen.y + screen.height.saturating_sub(menu_h));
        Some(Rect::new(x, y, menu_w, menu_h))
    }

    pub(crate) fn confirm_close_rect(&self) -> Rect {
        crate::ui::confirm_close_popup_rect(self.view.terminal_area).unwrap_or_default()
    }

    fn context_menu_item_at(&self, col: u16, row: u16) -> Option<usize> {
        let menu_rect = self.context_menu_rect()?;
        let inner_x = menu_rect.x + 1;
        let inner_y = menu_rect.y + 1;
        let inner_w = menu_rect.width.saturating_sub(2);
        let inner_h = menu_rect.height.saturating_sub(2);
        let item_count = self
            .context_menu
            .as_ref()
            .map(|menu| menu.items().len() as u16)
            .unwrap_or(0);
        if col >= inner_x
            && col < inner_x + inner_w
            && row >= inner_y
            && row < inner_y + inner_h.min(item_count)
        {
            Some((row - inner_y) as usize)
        } else {
            None
        }
    }

    pub(super) fn tab_at(&self, col: u16, row: u16) -> Option<usize> {
        self.view
            .tab_hit_areas
            .iter()
            .enumerate()
            .find_map(|(idx, area)| {
                (area.width > 0
                    && row >= area.y
                    && row < area.y + area.height
                    && col >= area.x
                    && col < area.x + area.width)
                    .then_some(idx)
            })
    }

    /// The stage instance whose strip entry covers this cell.
    ///
    /// Returns the instance identity rather than a position, so a caller can
    /// only ever act on the instance the geometry was published for.
    pub(super) fn stage_tab_at(
        &self,
        col: u16,
        row: u16,
    ) -> Option<crate::ui::surface_host::AppInstanceId> {
        self.view.stage_tab_hit_areas.iter().find_map(|entry| {
            let area = entry.rect;
            (area.width > 0
                && row >= area.y
                && row < area.y + area.height
                && col >= area.x
                && col < area.x + area.width)
                .then_some(entry.instance)
        })
    }

    fn mode_bar_covers_tab_row(&self, col: u16, row: u16) -> bool {
        self.tab_bar_position == crate::config::TabBarPositionConfig::Bottom
            && matches!(
                self.mode,
                Mode::Navigate | Mode::Prefix | Mode::Copy | Mode::Resize
            )
            && self.on_tab_bar(col, row)
    }

    pub(super) fn on_tab_bar(&self, col: u16, row: u16) -> bool {
        let area = self.view.tab_bar_rect;
        area.width > 0
            && row >= area.y
            && row < area.y + area.height
            && col >= area.x
            && col < area.x + area.width
    }

    pub(super) fn on_tab_scroll_left_button(&self, col: u16, row: u16) -> bool {
        let area = self.view.tab_scroll_left_hit_area;
        area.width > 0
            && row >= area.y
            && row < area.y + area.height
            && col >= area.x
            && col < area.x + area.width
    }

    pub(super) fn on_tab_scroll_right_button(&self, col: u16, row: u16) -> bool {
        let area = self.view.tab_scroll_right_hit_area;
        area.width > 0
            && row >= area.y
            && row < area.y + area.height
            && col >= area.x
            && col < area.x + area.width
    }

    pub(super) fn tab_drop_index_at(&self, col: u16, row: u16) -> Option<usize> {
        if !self.on_tab_bar(col, row) {
            return None;
        }

        let visible_tabs: Vec<_> = self
            .view
            .tab_hit_areas
            .iter()
            .enumerate()
            .filter(|(_, rect)| rect.width > 0)
            .collect();
        let (first_idx, first_rect) = *visible_tabs.first()?;
        let (last_idx, last_rect) = *visible_tabs.last()?;

        if self.on_tab_scroll_left_button(col, row) {
            return Some(0);
        }
        if self.on_tab_scroll_right_button(col, row) {
            return self
                .active
                .and_then(|idx| self.workspaces.get(idx))
                .map(|ws| ws.tabs.len());
        }

        let left_edge = if first_idx == 0 {
            first_rect.x
        } else {
            self.view.tab_scroll_left_hit_area.x + self.view.tab_scroll_left_hit_area.width
        };
        let right_edge = if self
            .active
            .and_then(|idx| self.workspaces.get(idx))
            .is_some_and(|ws| last_idx + 1 >= ws.tabs.len())
        {
            last_rect.x + last_rect.width
        } else {
            self.view.tab_scroll_right_hit_area.x.saturating_sub(1)
        };

        if col <= left_edge {
            return Some(first_idx);
        }
        if col >= right_edge {
            return Some(last_idx + 1);
        }

        for (idx, rect) in visible_tabs {
            let midpoint = rect.x + rect.width / 2;
            if col < midpoint {
                return Some(idx);
            }
            if col < rect.x + rect.width {
                return Some(idx + 1);
            }
        }

        Some(last_idx + 1)
    }

    pub(super) fn on_split_right_button(&self, col: u16, row: u16) -> bool {
        rect_contains(self.view.split_right_hit_area, col, row)
    }

    pub(super) fn on_split_down_button(&self, col: u16, row: u16) -> bool {
        rect_contains(self.view.split_down_hit_area, col, row)
    }

    pub(super) fn on_new_tab_button(&self, col: u16, row: u16) -> bool {
        let area = self.view.new_tab_hit_area;
        area.width > 0
            && row >= area.y
            && row < area.y + area.height
            && col >= area.x
            && col < area.x + area.width
    }

    pub(super) fn find_border_at(&self, col: u16, row: u16) -> Option<&SplitBorder> {
        self.view.split_borders.iter().find(|b| match b.direction {
            Direction::Horizontal if self.pane_borders && !self.pane_gaps => {
                col == b.pos && row >= b.area.y && row < b.area.y + b.area.height
            }
            Direction::Horizontal if self.pane_borders && self.pane_gaps => {
                row >= b.area.y
                    && row < b.area.y + b.area.height
                    && col >= b.pos.saturating_sub(1)
                    && col <= b.pos
            }
            Direction::Horizontal if !self.pane_borders && self.pane_gaps => {
                row >= b.area.y
                    && row < b.area.y + b.area.height
                    && b.pos.checked_sub(1).is_some_and(|gap_col| {
                        col == gap_col && self.pane_frame_at(col, row).is_none()
                    })
            }
            Direction::Vertical if self.pane_borders && !self.pane_gaps => {
                row == b.pos && col >= b.area.x && col < b.area.x + b.area.width
            }
            Direction::Vertical if self.pane_borders && self.pane_gaps => {
                col >= b.area.x
                    && col < b.area.x + b.area.width
                    && row >= b.pos.saturating_sub(1)
                    && row <= b.pos
            }
            Direction::Vertical if !self.pane_borders && self.pane_gaps => {
                col >= b.area.x
                    && col < b.area.x + b.area.width
                    && b.pos.checked_sub(1).is_some_and(|gap_row| {
                        row == gap_row && self.pane_frame_at(col, row).is_none()
                    })
            }
            _ => false,
        })
    }

    pub(super) fn pane_at(&self, col: u16, row: u16) -> Option<&PaneInfo> {
        self.view.pane_infos.iter().find(|p| {
            col >= p.inner_rect.x
                && col < p.inner_rect.x + p.inner_rect.width
                && row >= p.inner_rect.y
                && row < p.inner_rect.y + p.inner_rect.height
        })
    }

    pub(super) fn pane_mouse_target(&self, col: u16, row: u16) -> Option<&PaneInfo> {
        self.pane_at(col, row)
            .or_else(|| self.pane_frame_at(col, row))
    }

    fn chrome_press_pending(&self, source_id: crate::app::InputSourceId) -> bool {
        self.tab_presses.contains_key(&source_id) || self.workspace_presses.contains_key(&source_id)
    }

    fn chrome_drag_owned_by_other(&self, source_id: crate::app::InputSourceId) -> bool {
        self.drag.as_ref().is_some_and(|drag| {
            matches!(
                drag.target,
                DragTarget::WorkspaceReorder {
                    source_id: drag_source_id,
                    ..
                } | DragTarget::TabReorder {
                    source_id: drag_source_id,
                    ..
                } if drag_source_id != source_id
            )
        })
    }

    fn chrome_press_action(
        &mut self,
        workspace_press: Option<WorkspacePressState>,
        tab_press: Option<TabPressState>,
    ) -> Option<MouseAction> {
        if let Some(press) = workspace_press {
            self.mode = Mode::Terminal;
            return Some(MouseAction::FocusWorkspace {
                ws_idx: press.ws_idx,
            });
        }
        if let Some(press) = tab_press {
            if self.active == Some(press.ws_idx) {
                self.mode = Mode::Terminal;
                return Some(MouseAction::FocusTab {
                    tab_idx: press.tab_idx,
                });
            }
        }
        None
    }

    pub(crate) fn clear_chrome_gesture(&mut self, source_id: crate::app::InputSourceId) {
        if self.drag.as_ref().is_some_and(|drag| {
            matches!(
                drag.target,
                DragTarget::WorkspaceReorder {
                    source_id: drag_source_id,
                    ..
                } | DragTarget::TabReorder {
                    source_id: drag_source_id,
                    ..
                } if drag_source_id == source_id
            )
        }) {
            self.drag = None;
        }
        self.clear_chrome_press(source_id);
    }

    fn clear_chrome_press(&mut self, source_id: crate::app::InputSourceId) {
        self.tab_presses.remove(&source_id);
        self.workspace_presses.remove(&source_id);
    }

    fn mouse_pane_focus_action(&self, pane_id: crate::layout::PaneId) -> Option<MouseAction> {
        let ws_idx = self.active?;
        (self
            .workspaces
            .get(ws_idx)
            .and_then(|workspace| workspace.focused_pane_id())
            != Some(pane_id))
        .then_some(MouseAction::FocusPane { ws_idx, pane_id })
    }

    pub(crate) fn pane_info_by_id(&self, pane_id: crate::layout::PaneId) -> Option<&PaneInfo> {
        self.view.pane_infos.iter().find(|info| info.id == pane_id)
    }

    pub(super) fn pane_frame_at(&self, col: u16, row: u16) -> Option<&PaneInfo> {
        self.view.pane_infos.iter().find(|p| {
            col >= p.rect.x
                && col < p.rect.x + p.rect.width
                && row >= p.rect.y
                && row < p.rect.y + p.rect.height
        })
    }

    pub(super) fn focus_pane(&mut self, pane_id: crate::layout::PaneId) {
        let _ = pane_id;
    }

    fn clickable_toast_at(&self, col: u16, row: u16) -> bool {
        self.toast
            .as_ref()
            .is_some_and(|toast| toast.target.is_some())
            && rect_contains(self.view.toast_hit_area, col, row)
    }

    #[cfg(test)]
    pub(crate) fn focus_toast_target(&mut self) {
        let Some(target) = self.toast.as_ref().and_then(|toast| toast.target.clone()) else {
            return;
        };
        let Some(ws_idx) = self
            .workspaces
            .iter()
            .position(|workspace| workspace.id == target.workspace_id)
        else {
            return;
        };
        let Some(_tab_idx) = self.workspaces[ws_idx].find_tab_index_for_pane(target.pane_id) else {
            return;
        };

        self.focus_pane_in_workspace(ws_idx, target.pane_id);
        self.toast = None;
        self.settle_terminal_mode_after_focus();
    }

    pub(crate) fn scroll_pane_up(
        &self,
        terminal_runtimes: &TerminalRuntimeRegistry,
        pane_id: crate::layout::PaneId,
        lines: usize,
    ) {
        if let Some(ws_idx) = self.active {
            if let Some(rt) = self.runtime_for_pane_in_workspace(terminal_runtimes, ws_idx, pane_id)
            {
                rt.scroll_up(lines);
            }
        }
    }

    pub(crate) fn scroll_pane_down(
        &self,
        terminal_runtimes: &TerminalRuntimeRegistry,
        pane_id: crate::layout::PaneId,
        lines: usize,
    ) {
        if let Some(ws_idx) = self.active {
            if let Some(rt) = self.runtime_for_pane_in_workspace(terminal_runtimes, ws_idx, pane_id)
            {
                rt.scroll_down(lines);
            }
        }
    }

    pub(crate) fn pane_scroll_metrics(
        &self,
        terminal_runtimes: &TerminalRuntimeRegistry,
        pane_id: crate::layout::PaneId,
    ) -> Option<crate::pane::ScrollMetrics> {
        self.active
            .and_then(|i| self.runtime_for_pane_in_workspace(terminal_runtimes, i, pane_id))
            .and_then(crate::terminal::TerminalRuntime::scroll_metrics)
    }

    /// The branch/workspace row's menu, shared by the right-click and the
    /// "⋯" cell so the two roads can never drift apart (TP-DOTS-04).
    fn open_workspace_row_menu(
        &mut self,
        terminal_runtimes: &TerminalRuntimeRegistry,
        idx: usize,
        x: u16,
        y: u16,
    ) {
        self.selected = idx;
        let kind = self
            .workspaces
            .get(idx)
            .and_then(|ws| {
                let group_state = crate::ui::workspace_parent_group_state(self, idx);
                let git_space = ws.git_space().cloned().or_else(|| {
                    ws.resolved_identity_cwd_from(&self.terminals, terminal_runtimes)
                        .as_deref()
                        .and_then(crate::workspace::git_space_metadata)
                });
                let is_linked_worktree = ws.worktree_space().map_or_else(
                    || {
                        git_space
                            .as_ref()
                            .is_some_and(|space| space.is_linked_worktree)
                    },
                    |space| space.is_linked_worktree,
                );
                let show_git_menu = ws.worktree_space().is_some()
                    || git_space
                        .as_ref()
                        .is_some_and(|space| !space.is_linked_worktree);
                show_git_menu.then_some(ContextMenuKind::GitWorkspace {
                    ws_idx: idx,
                    is_linked_worktree,
                    has_worktree_children: group_state.is_some(),
                    collapsed: group_state
                        .as_ref()
                        .is_some_and(|(_, collapsed)| *collapsed),
                    space_is_custom: crate::ui::effective_space(self, idx)
                        .is_some_and(|space| space.is_custom),
                })
            })
            .unwrap_or(ContextMenuKind::Workspace { ws_idx: idx });
        self.context_menu = Some(ContextMenuState {
            kind,
            x,
            y,
            list: MenuListState::new(0),
        });
        self.enter_overlay_mode(Mode::ContextMenu);
    }

    /// The committed ratio for a pair of half widths. The preview bounds
    /// already hold the drag inside the band, but integer floor on both the
    /// bound (`total*20/100`) and this division can still land one percent
    /// outside it — total 101 puts the minimum track at 20 cells, and
    /// 20*100/101 floors to 19. The clamp is that last percent, not
    /// decoration.
    pub(super) fn sbs_ratio_from_tracks_impl(left: u16, right: u16) -> u8 {
        let total = u32::from(left).saturating_add(u32::from(right)).max(1);
        let ratio = ((u32::from(left) * 100) / total) as u8;
        ratio.clamp(20, 80)
    }

    /// The agent-panel row drag: a pressed row is only a CANDIDATE until the
    /// pointer leaves it (a plain click stays a click); from then on Drag
    /// moves the insertion target and Up commits the reorder into
    /// `agent_panel_order`. Returns true when the event belongs to the drag.
    fn handle_agent_panel_drag_mouse(&mut self, mouse: &MouseEvent) -> bool {
        let Some(mut drag) = self.agent_panel_drag else {
            return false;
        };
        match mouse.kind {
            MouseEventKind::Drag(MouseButton::Left) => {
                drag.current_row = mouse.row;
                if mouse.row != drag.pressed_row {
                    drag.dragging = true;
                }
                self.agent_panel_drag = Some(drag);
                drag.dragging
            }
            MouseEventKind::Up(MouseButton::Left) => {
                self.agent_panel_drag = None;
                if !drag.dragging {
                    return false;
                }
                self.commit_agent_panel_reorder(drag);
                true
            }
            _ => false,
        }
    }

    /// Write the drag's outcome: the panel's current visible order with the
    /// dragged pane moved to the drop row's position becomes the remembered
    /// order. Seeding from the visible order means the first drag reorders
    /// exactly what the user sees.
    fn commit_agent_panel_reorder(&mut self, drag: crate::app::state::AgentPanelDrag) {
        let entries = crate::ui::agent_panel_entries(self);
        let mut order: Vec<crate::layout::PaneId> =
            entries.iter().map(|entry| entry.pane_id).collect();
        let Some(from) = order.iter().position(|id| *id == drag.pane_id) else {
            return;
        };
        let to = self
            .agent_detail_target_at(drag.current_row)
            .and_then(|(_, _, target)| order.iter().position(|id| *id == target))
            .unwrap_or(from);
        let moved = order.remove(from);
        order.insert(to.min(order.len()), moved);
        self.agent_panel_order = order;
    }

    /// The side-by-side divider's whole gesture: arm on the named column,
    /// preview while dragging, commit the clamped ratio on release. Returns
    /// true when the event belongs to the divider.
    fn handle_sbs_divider_mouse(&mut self, mouse: &MouseEvent) -> bool {
        use crate::ui::shell::{ResizeBounds, ResizeDecision, ResizeTransaction};

        if self.shell_interaction.side_by_side_resize_active() {
            match mouse.kind {
                MouseEventKind::Drag(MouseButton::Left) => {
                    let total = self.shell_interaction.resize_original_total().unwrap_or(0);
                    // The same 20-80 band the committed ratio promises, held
                    // live so the preview cannot starve a half either.
                    let min = (((u32::from(total)) * 20) / 100).max(1) as u16;
                    if let Some(bounds) = ResizeBounds::new(min, total, min, total) {
                        let _ = self.shell_interaction.preview_resize(
                            ratatui::layout::Position::new(mouse.column, mouse.row),
                            bounds,
                        );
                    }
                    true
                }
                MouseEventKind::Up(MouseButton::Left) => {
                    let update = self.shell_interaction.commit_resize(0);
                    if let ResizeDecision::Committed([left, right]) = update.decision() {
                        if let Some(sbs) = self.side_by_side.as_mut() {
                            sbs.ratio_percent = Self::sbs_ratio_from_tracks_impl(left, right);
                        }
                    }
                    true
                }
                // While armed, every other pointer event stays with the
                // divider — leaking a mid-drag Move into a pane would scroll
                // whatever the pointer happens to cross.
                _ => true,
            }
        } else {
            if !matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
                return false;
            }
            let Some(divider) = self.view.sbs_divider_rect else {
                return false;
            };
            let on_divider = mouse.column == divider.x
                && mouse.row >= divider.y
                && mouse.row < divider.y.saturating_add(divider.height);
            if !on_divider {
                return false;
            }
            let left_w = divider.x.saturating_sub(self.view.terminal_area.x);
            let right_w = self
                .view
                .right_surface
                .as_ref()
                .map(|surface| surface.area.width)
                .unwrap_or(0);
            let Some(transaction) = ResizeTransaction::begin_side_by_side(
                0,
                ratatui::layout::Position::new(mouse.column, mouse.row),
                [left_w, right_w],
            ) else {
                return false;
            };
            self.shell_interaction.begin_resize(transaction);
            true
        }
    }

    fn handle_right_click_passthrough(
        &mut self,
        terminal_runtimes: &TerminalRuntimeRegistry,
        source_id: crate::app::InputSourceId,
        mouse: MouseEvent,
        in_sidebar: bool,
    ) -> bool {
        if let Some(gesture) = self.right_click_passthrough.clone() {
            match mouse.kind {
                MouseEventKind::Drag(MouseButton::Right)
                | MouseEventKind::Up(MouseButton::Right) => {
                    let forwarded_mouse =
                        self.strip_right_click_passthrough_modifiers(mouse, gesture.modifiers);
                    let _ = self.forward_pane_mouse_button(
                        terminal_runtimes,
                        &gesture.pane_info,
                        forwarded_mouse,
                    );
                    if matches!(mouse.kind, MouseEventKind::Up(MouseButton::Right)) {
                        self.right_click_passthrough = None;
                    }
                    return true;
                }
                _ => {
                    self.right_click_passthrough = None;
                }
            }
        }

        if self.mode != Mode::Terminal
            || in_sidebar
            || !matches!(mouse.kind, MouseEventKind::Down(MouseButton::Right))
        {
            return false;
        }

        let Some(info) = self.pane_at(mouse.column, mouse.row).cloned() else {
            return false;
        };
        let configured_modifiers = self
            .right_click_passthrough_modifiers
            .filter(|modifiers| mouse.modifiers == *modifiers);
        let pane_passthrough = mouse.modifiers.is_empty()
            && self.active.is_some_and(|ws_idx| {
                self.workspaces
                    .get(ws_idx)
                    .and_then(|workspace| workspace.pane_state(info.id))
                    .is_some_and(|pane| pane.right_click_passthrough)
            });
        let Some(modifiers) = configured_modifiers
            .or_else(|| pane_passthrough.then(crossterm::event::KeyModifiers::empty))
        else {
            return false;
        };

        self.focus_pane(info.id);
        let forwarded_mouse = self.strip_right_click_passthrough_modifiers(mouse, modifiers);
        if !self.forward_pane_mouse_button(terminal_runtimes, &info, forwarded_mouse) {
            return false;
        }

        self.selection = None;
        self.selection_autoscroll = None;
        self.clear_chrome_press(source_id);
        self.drag = None;
        self.context_menu = None;
        self.right_click_passthrough = Some(RightClickPassthroughGesture {
            pane_info: info,
            modifiers,
        });
        true
    }

    fn strip_right_click_passthrough_modifiers(
        &self,
        mouse: MouseEvent,
        modifiers: crossterm::event::KeyModifiers,
    ) -> MouseEvent {
        MouseEvent {
            modifiers: mouse.modifiers.difference(modifiers),
            ..mouse
        }
    }

    pub(super) fn handle_terminal_wheel(
        &mut self,
        terminal_runtimes: &TerminalRuntimeRegistry,
        mouse: MouseEvent,
    ) {
        let lines_per_notch = self.mouse_scroll_lines;

        if let Some(info) = self.pane_at(mouse.column, mouse.row).cloned() {
            self.focus_pane(info.id);
            if self.forward_pane_wheel(terminal_runtimes, &info, mouse) {
                return;
            }
            match mouse.kind {
                MouseEventKind::ScrollUp => {
                    self.scroll_pane_up(terminal_runtimes, info.id, lines_per_notch)
                }
                MouseEventKind::ScrollDown => {
                    self.scroll_pane_down(terminal_runtimes, info.id, lines_per_notch)
                }
                _ => {}
            }
            return;
        }

        if let Some(info) = self.pane_frame_at(mouse.column, mouse.row).cloned() {
            self.focus_pane(info.id);
            match mouse.kind {
                MouseEventKind::ScrollUp => {
                    self.scroll_pane_up(terminal_runtimes, info.id, lines_per_notch)
                }
                MouseEventKind::ScrollDown => {
                    self.scroll_pane_down(terminal_runtimes, info.id, lines_per_notch)
                }
                _ => {}
            }
            return;
        }

        if let Some(ws_idx) = self.active {
            if let Some(rt) = self.focused_runtime_in_workspace(terminal_runtimes, ws_idx) {
                match mouse.kind {
                    MouseEventKind::ScrollUp => rt.scroll_up(lines_per_notch),
                    MouseEventKind::ScrollDown => rt.scroll_down(lines_per_notch),
                    _ => {}
                }
            }
        }
    }

    fn pane_mouse_position(
        &self,
        runtime: &crate::terminal::TerminalRuntime,
        inner: Rect,
        mouse: MouseEvent,
    ) -> Option<crate::input::mouse::Position> {
        let column = mouse.column.saturating_sub(inner.x);
        let row = mouse.row.saturating_sub(inner.y);
        let cell = crate::input::mouse::Position::Cell { column, row };
        let Some(host) = self.host_mouse_pixels else {
            return Some(cell);
        };
        let wants_pixels = runtime.sgr_pixel_mouse_enabled();
        if !wants_pixels {
            return Some(cell);
        }
        let Some((width_px, height_px)) = runtime.pixel_size() else {
            return Some(cell);
        };
        Some(
            host.pane_position(inner, width_px, height_px)
                .unwrap_or(cell),
        )
    }

    pub(super) fn forward_pane_mouse_button(
        &mut self,
        terminal_runtimes: &TerminalRuntimeRegistry,
        info: &PaneInfo,
        mouse: MouseEvent,
    ) -> bool {
        let Some(ws_idx) = self.active else {
            return false;
        };
        let Some(rt) = self.runtime_for_pane_in_workspace(terminal_runtimes, ws_idx, info.id)
        else {
            return false;
        };
        let Some(position) = self.pane_mouse_position(rt, info.inner_rect, mouse) else {
            return false;
        };
        let Some(bytes) = rt.encode_mouse_button(mouse.kind, position, mouse.modifiers) else {
            // The program never asked for the mouse; queue the press so the
            // App layer can publish it for graphics panes. TP-INP-MOUSE-01
            self.queue_unclaimed_pane_pointer(info.id, position, mouse);
            return false;
        };
        rt.scroll_reset();
        if let Err(err) = rt.try_send_bytes(Bytes::from(bytes)) {
            warn!(pane = info.id.raw(), err = %err, kind = ?mouse.kind, "failed to forward mouse button event");
        }
        true
    }

    fn queue_unclaimed_pane_pointer(
        &mut self,
        pane_id: crate::layout::PaneId,
        position: crate::input::mouse::Position,
        mouse: MouseEvent,
    ) {
        let (kind, button) = match mouse.kind {
            MouseEventKind::Down(button) => (crate::api::schema::PanePointerKind::Down, button),
            MouseEventKind::Up(button) => (crate::api::schema::PanePointerKind::Up, button),
            _ => return,
        };
        let button = match button {
            MouseButton::Left => 0,
            MouseButton::Middle => 1,
            MouseButton::Right => 2,
        };
        let (column, row, x_px, y_px) = match position {
            crate::input::mouse::Position::Cell { column, row } => (column, row, None, None),
            crate::input::mouse::Position::Pixels { x, y } => (0, 0, Some(x), Some(y)),
        };
        self.pending_pane_pointer
            .push(crate::app::PanePointerPress {
                pane_id,
                kind,
                button,
                column,
                row,
                x_px,
                y_px,
            });
    }

    pub(super) fn forward_pane_mouse_motion(
        &self,
        terminal_runtimes: &TerminalRuntimeRegistry,
        info: &PaneInfo,
        mouse: MouseEvent,
    ) -> bool {
        let Some(ws_idx) = self.active else {
            return false;
        };
        let Some(rt) = self.runtime_for_pane_in_workspace(terminal_runtimes, ws_idx, info.id)
        else {
            return false;
        };
        let Some(position) = self.pane_mouse_position(rt, info.inner_rect, mouse) else {
            return false;
        };
        let Some(bytes) = rt.encode_mouse_motion(mouse.kind, position, mouse.modifiers) else {
            return false;
        };
        if let Err(err) = rt.try_send_bytes(Bytes::from(bytes)) {
            warn!(pane = info.id.raw(), err = %err, kind = ?mouse.kind, "failed to forward mouse motion event");
        }
        true
    }

    fn forward_pane_reported_wheel(
        &self,
        terminal_runtimes: &TerminalRuntimeRegistry,
        info: &PaneInfo,
        mouse: MouseEvent,
    ) -> bool {
        let Some(ws_idx) = self.active else {
            return false;
        };
        let Some(rt) = self.runtime_for_pane_in_workspace(terminal_runtimes, ws_idx, info.id)
        else {
            return false;
        };
        if rt.wheel_routing() != Some(crate::pane::WheelRouting::MouseReport) {
            return false;
        }
        rt.scroll_reset();
        let Some(position) = self.pane_mouse_position(rt, info.inner_rect, mouse) else {
            return false;
        };
        let Some(bytes) = rt.encode_mouse_wheel(mouse.kind, position, mouse.modifiers) else {
            warn!(pane = info.id.raw(), kind = ?mouse.kind, "failed to encode mouse wheel event");
            return true;
        };
        if let Err(err) = rt.try_send_bytes(Bytes::from(bytes)) {
            warn!(pane = info.id.raw(), err = %err, "failed to forward mouse wheel event");
        }
        true
    }

    /// TP-MOB-101: whether the host may claim the vertical wheel from a pane
    /// that asked for mouse reporting. This answers *policy* only — whether
    /// the host can actually honour the claim is a separate question the
    /// caller still asks of the pane's scrollback (TP-MOB-97), and no mode
    /// here overrides that.
    fn host_scroll_claims_wheel(&self) -> bool {
        match self.mouse_wheel_host_scroll {
            crate::config::MouseWheelHostScrollConfig::Never => false,
            crate::config::MouseWheelHostScrollConfig::Mobile => {
                self.view.layout == ViewLayout::Mobile
            }
            crate::config::MouseWheelHostScrollConfig::Scrollable => true,
        }
    }

    pub(super) fn forward_pane_wheel(
        &self,
        terminal_runtimes: &TerminalRuntimeRegistry,
        info: &PaneInfo,
        mouse: MouseEvent,
    ) -> bool {
        let Some(ws_idx) = self.active else {
            return false;
        };
        let Some(rt) = self.runtime_for_pane_in_workspace(terminal_runtimes, ws_idx, info.id)
        else {
            return false;
        };
        match rt.wheel_routing() {
            Some(crate::pane::WheelRouting::HostScroll) | None => false,
            Some(crate::pane::WheelRouting::MouseReport)
                if self.host_scroll_claims_wheel()
                    && matches!(
                        mouse.kind,
                        MouseEventKind::ScrollUp | MouseEventKind::ScrollDown
                    )
                    && rt
                        .scroll_metrics()
                        .is_some_and(|metrics| metrics.max_offset_from_bottom > 0) =>
            {
                // An agent that asked for mouse reporting typically does
                // nothing with the wheel, so the gesture lands nowhere and the
                // content area looks frozen. On a phone that spends the only
                // scroll gesture there is: no wheel, no keyboard shortcut in
                // reach, and a scrollbar too thin for a finger (TP-MOB-56), so
                // the phone shell claims it by default. On a desktop the same
                // dead gesture is opt-in (`scrollable`) rather than automatic,
                // because a pane there may genuinely act on the wheel without
                // an alternate screen — and unlike the phone, the desktop has
                // the pane edge, the scrollbar and copy mode to fall back on.
                // Which surfaces claim it is policy (TP-MOB-101); whether the
                // claim can be honoured is not. Alternate-scroll panes are not
                // covered because their arrow keys already scroll on touch —
                // and the claim only holds while the host has scrollback to
                // give: an alternate-screen program has none, and a gesture
                // spent on an empty scrollback is a gesture that does nothing
                // at all (TP-MOB-97), so it falls through to the program.
                false
            }
            Some(crate::pane::WheelRouting::MouseReport) => {
                self.forward_pane_reported_wheel(terminal_runtimes, info, mouse)
            }
            Some(crate::pane::WheelRouting::AlternateScroll) => {
                rt.scroll_reset();
                let Some(bytes) = rt.encode_alternate_scroll(mouse.kind) else {
                    return true;
                };
                if let Err(err) = rt.try_send_bytes(Bytes::from(bytes)) {
                    warn!(pane = info.id.raw(), err = %err, "failed to forward alternate-scroll key");
                }
                true
            }
        }
    }

    pub(super) fn set_pane_scroll_offset(
        &self,
        terminal_runtimes: &TerminalRuntimeRegistry,
        pane_id: crate::layout::PaneId,
        offset_from_bottom: usize,
    ) {
        for ws_idx in 0..self.workspaces.len() {
            let Some(rt) = self.runtime_for_pane_in_workspace(terminal_runtimes, ws_idx, pane_id)
            else {
                continue;
            };
            rt.set_scroll_offset_from_bottom(offset_from_bottom);
            return;
        }
    }

    pub(super) fn scrollbar_target_at(
        &self,
        terminal_runtimes: &TerminalRuntimeRegistry,
        col: u16,
        row: u16,
    ) -> Option<(crate::layout::PaneId, ScrollbarClickTarget)> {
        let ws_idx = self.active?;
        let info = self.view.pane_infos.iter().find(|info| {
            crate::ui::pane_scrollbar_rect(info).is_some_and(|track| {
                col >= track.x
                    && col < track.x + track.width
                    && row >= track.y
                    && row < track.y + track.height
            })
        })?;
        let rt = self.runtime_for_pane_in_workspace(terminal_runtimes, ws_idx, info.id)?;
        let metrics = rt.scroll_metrics()?;
        if metrics.max_offset_from_bottom == 0 {
            return None;
        }
        let track = crate::ui::pane_scrollbar_rect(info)?;
        if let Some(grab_row_offset) = crate::ui::scrollbar_thumb_grab_offset(metrics, track, row) {
            Some((info.id, ScrollbarClickTarget::Thumb { grab_row_offset }))
        } else {
            Some((
                info.id,
                ScrollbarClickTarget::Track {
                    offset_from_bottom: crate::ui::scrollbar_offset_from_row(metrics, track, row),
                },
            ))
        }
    }

    pub(super) fn scrollbar_offset_for_pane_row(
        &self,
        terminal_runtimes: &TerminalRuntimeRegistry,
        pane_id: crate::layout::PaneId,
        row: u16,
        grab_row_offset: u16,
    ) -> Option<usize> {
        let ws_idx = self.active?;
        let info = self
            .view
            .pane_infos
            .iter()
            .find(|info| info.id == pane_id)?;
        let track = crate::ui::pane_scrollbar_rect(info)?;
        let rt = self.runtime_for_pane_in_workspace(terminal_runtimes, ws_idx, pane_id)?;
        let metrics = rt.scroll_metrics()?;
        if metrics.max_offset_from_bottom == 0 {
            return None;
        }
        Some(crate::ui::scrollbar_offset_from_drag_row(
            metrics,
            track,
            row,
            grab_row_offset,
        ))
    }
}

#[cfg(test)]
pub(super) fn wheel_routing(input_state: crate::pane::InputState) -> WheelRouting {
    if input_state.mouse_protocol_mode.reporting_enabled() {
        WheelRouting::MouseReport
    } else if input_state.alternate_screen && input_state.mouse_alternate_scroll {
        WheelRouting::AlternateScroll
    } else {
        WheelRouting::HostScroll
    }
}

fn rect_contains(rect: Rect, col: u16, row: u16) -> bool {
    rect.width > 0
        && rect.height > 0
        && col >= rect.x
        && col < rect.x + rect.width
        && row >= rect.y
        && row < rect.y + rect.height
}

fn apply_scroll(scroll: &mut usize, delta: i16, max_scroll: usize) {
    if delta.is_negative() {
        *scroll = scroll.saturating_sub(delta.unsigned_abs() as usize);
    } else {
        *scroll = scroll.saturating_add(delta as usize).min(max_scroll);
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEventKind};
    use ratatui::layout::{Direction, Rect};

    use super::super::{
        app_for_mouse_test, capture_snapshot, mouse, numbered_lines_bytes, root_layout_ratio,
    };
    use super::*;
    use crate::app::input::modal::handle_context_menu_key;
    use crate::{
        app::state::{ContextMenuKind, ContextMenuState, MenuListState, Mode, ViewLayout},
        detect::{Agent, AgentState},
        workspace::Workspace,
    };

    fn pane_pointer_fixture() -> (
        crate::app::App,
        crate::terminal::TerminalRuntimeRegistry,
        crate::layout::PaneInfo,
    ) {
        let mut app = app_for_mouse_test();
        let mut workspace = Workspace::test_new("pointer");
        let pane_id = workspace.focused_pane_id().expect("pane");
        workspace.insert_test_runtime(
            pane_id,
            crate::terminal::TerminalRuntime::test_with_screen_bytes(80, 24, b""),
        );
        app.state.workspaces = vec![workspace];
        app.state.active = Some(0);
        app.state.selected = 0;
        let info = crate::layout::PaneInfo {
            id: pane_id,
            rect: Rect::new(0, 0, 80, 24),
            inner_rect: Rect::new(0, 0, 80, 24),
            scrollbar_rect: None,
            borders: ratatui::widgets::Borders::NONE,
            is_focused: true,
        };
        let registry = crate::terminal::TerminalRuntimeRegistry::default();
        (app, registry, info)
    }

    fn pointer_events(app: &crate::app::App) -> Vec<crate::api::schema::EventEnvelope> {
        app.event_hub
            .events_after(0)
            .into_iter()
            .map(|(_, envelope)| envelope)
            .filter(|envelope| envelope.event == crate::api::schema::EventKind::PanePointer)
            .collect()
    }

    // A pane that paints graphics but never enabled a terminal mouse mode
    // would swallow the user's click; the press is published instead so an
    // out-of-band bridge can deliver it. TP-INP-MOUSE-01
    #[tokio::test]
    async fn an_unclaimed_click_on_a_graphics_pane_is_published() {
        let (mut app, registry, info) = pane_pointer_fixture();
        app.pane_graphics.slots.insert(
            (info.id, "L".to_string()),
            crate::app::pane_graphics::Slot::test(77, None),
        );
        let handled = app.state.forward_pane_mouse_button(
            &registry,
            &info,
            mouse(MouseEventKind::Down(MouseButton::Left), 5, 5),
        );
        assert!(!handled, "an unclaimed press still falls through to the UI");
        app.drain_pane_pointer_events();
        let events = pointer_events(&app);
        assert_eq!(events.len(), 1, "exactly one pane.pointer event");
        match &events[0].data {
            crate::api::schema::EventData::PanePointer {
                kind,
                button,
                column,
                row,
                ..
            } => {
                assert_eq!(*kind, crate::api::schema::PanePointerKind::Down);
                assert_eq!(*button, 0);
                assert_eq!((*column, *row), (5, 5));
            }
            other => panic!("expected PanePointer data, got {other:?}"),
        }
    }

    // TP-INP-MOUSE-01
    #[tokio::test]
    async fn a_program_that_asked_for_the_mouse_keeps_it_exclusively() {
        let (mut app, registry, info) = pane_pointer_fixture();
        app.pane_graphics.slots.insert(
            (info.id, "L".to_string()),
            crate::app::pane_graphics::Slot::test(77, None),
        );
        let runtime = app
            .state
            .runtime_for_pane_in_workspace(&registry, 0, info.id)
            .expect("runtime");
        runtime.test_process_pty_bytes(b"\x1b[?1002;1006h");
        let handled = app.state.forward_pane_mouse_button(
            &registry,
            &info,
            mouse(MouseEventKind::Down(MouseButton::Left), 5, 5),
        );
        assert!(handled, "an SGR-mode pane receives the press directly");
        app.drain_pane_pointer_events();
        assert!(
            pointer_events(&app).is_empty(),
            "no side-channel event when the program already got the press"
        );
    }

    // TP-INP-MOUSE-01
    #[tokio::test]
    async fn a_plain_text_pane_does_not_publish_pointer_events() {
        let (mut app, registry, info) = pane_pointer_fixture();
        let handled = app.state.forward_pane_mouse_button(
            &registry,
            &info,
            mouse(MouseEventKind::Down(MouseButton::Left), 5, 5),
        );
        assert!(!handled);
        app.drain_pane_pointer_events();
        assert!(
            pointer_events(&app).is_empty(),
            "a pane with no graphics layer keeps the quiet path"
        );
    }

    // The browser case: terminal-browser paints through the terminal with
    // kitty graphics, not the pane.graphics API, so the gate must accept a
    // pane whose own program is drawing kitty images even with no API layer.
    // Caught by the end-to-end proof after the API-slot-only gate shipped
    // green in unit tests but dropped every real click. TP-INP-MOUSE-01
    #[tokio::test]
    async fn an_unclaimed_click_on_a_terminal_graphics_pane_is_published() {
        crate::kitty_graphics::set_enabled(true);
        let (mut app, registry, info) = pane_pointer_fixture();
        // No API layer: the pane's own program transmits a kitty image, exactly
        // as an in-pane browser would. Generation goes non-zero on transmit,
        // independent of a known cell pixel size or a live placement.
        let runtime = app
            .state
            .runtime_for_pane_in_workspace(&registry, 0, info.id)
            .expect("runtime");
        runtime.test_process_pty_bytes(b"\x1b_Ga=t,f=32,t=d,i=7,s=1,v=1,q=2;/wAA/w==\x1b\\");
        assert!(
            runtime.kitty_graphics_generation() > 0,
            "the pane's terminal has painted kitty graphics",
        );
        assert!(
            !app.pane_graphics
                .slots
                .contains_key(&(info.id, "L".to_string())),
            "and it owns no pane.graphics API layer",
        );
        let handled = app.state.forward_pane_mouse_button(
            &registry,
            &info,
            mouse(MouseEventKind::Down(MouseButton::Left), 5, 5),
        );
        assert!(!handled, "the program never asked for the mouse");
        app.drain_pane_pointer_events();
        assert_eq!(
            pointer_events(&app).len(),
            1,
            "a click on terminal-native graphics is published",
        );
    }

    // A press is queued against whichever workspace the viewer had at input
    // time; by the time the server loop drains it another display may have
    // moved `active`, so the pane is found where it lives, not where the
    // cursor is now. TP-INP-MOUSE-01
    #[tokio::test]
    async fn a_queued_press_is_published_even_when_its_workspace_is_no_longer_active() {
        let (mut app, _registry, info) = pane_pointer_fixture();
        app.state.workspaces.push(Workspace::test_new("elsewhere"));
        app.state.active = Some(1);
        app.state.selected = 1;
        app.pane_graphics.slots.insert(
            (info.id, "L".to_string()),
            crate::app::pane_graphics::Slot::test(77, None),
        );
        app.state
            .pending_pane_pointer
            .push(crate::app::PanePointerPress {
                pane_id: info.id,
                kind: crate::api::schema::PanePointerKind::Up,
                button: 2,
                column: 3,
                row: 1,
                x_px: None,
                y_px: None,
            });
        app.drain_pane_pointer_events();
        let events = pointer_events(&app);
        assert_eq!(events.len(), 1, "the press is still published");
        let expected = app.public_pane_id(0, info.id).expect("public pane id");
        match &events[0].data {
            crate::api::schema::EventData::PanePointer {
                pane_id,
                kind,
                button,
                ..
            } => {
                assert_eq!(pane_id, &expected);
                assert_eq!(*kind, crate::api::schema::PanePointerKind::Up);
                assert_eq!(*button, 2);
            }
            other => panic!("expected PanePointer data, got {other:?}"),
        }
    }

    fn app_with_dock_targets() -> (crate::app::App, Rect, Rect) {
        let mut app = app_for_mouse_test();
        let mut workspace = Workspace::test_new("dock-input");
        workspace.identity_cwd = std::env::temp_dir();
        app.state.workspaces = vec![workspace];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.ensure_test_terminals();
        let model = crate::ui::app_dock::AppDockModel::for_state(&app.state);
        let dock = Rect::new(110, 0, 5, 8);
        app.state.view.app_dock_entry_areas =
            crate::ui::app_dock::app_dock_entry_areas(&model, dock);
        let rect_of = |app_id: crate::ui::surface_host::BuiltInAppId| {
            app.state
                .view
                .app_dock_entry_areas
                .iter()
                .find(|entry| entry.app == app_id)
                .expect("dock target")
                .rect
        };
        let terminal_rect = rect_of(crate::ui::surface_host::BuiltInAppId::Terminal);
        let files_rect = rect_of(crate::ui::surface_host::BuiltInAppId::Files);
        (app, terminal_rect, files_rect)
    }

    // SF5.2: a left click on the enabled Files dock target activates the
    // existing Files singleton or opens one; a second click cannot create a
    // second instance.
    #[test]
    fn left_click_files_activates_existing_singleton_or_opens_one() {
        let (mut app, _terminal_rect, files_rect) = app_with_dock_targets();
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            files_rect.x,
            files_rect.y,
        ));
        assert!(
            app.state.file_manager.is_some(),
            "the Files dock target must open the Files surface"
        );
        assert_eq!(
            app.state.stage.surface_view(),
            crate::ui::surface_host::StageSurfaceView::NativeFiles
        );

        let stage_after_open = app.state.stage;
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            files_rect.x,
            files_rect.y,
        ));
        assert_eq!(
            app.state.stage, stage_after_open,
            "the singleton policy must reuse the existing Files instance"
        );
        assert!(app.state.file_manager.is_some());
    }

    // SF5.2: a left click on the Terminal dock target restores the terminal
    // stage from Files.
    #[test]
    fn left_click_terminal_restores_terminal_stage() {
        let (mut app, terminal_rect, files_rect) = app_with_dock_targets();
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            files_rect.x,
            files_rect.y,
        ));
        assert_eq!(
            app.state.stage.surface_view(),
            crate::ui::surface_host::StageSurfaceView::NativeFiles,
            "fixture: Files owns the stage before the terminal click"
        );

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            terminal_rect.x,
            terminal_rect.y,
        ));
        assert_eq!(
            app.state.stage.surface_view(),
            crate::ui::surface_host::StageSurfaceView::TerminalWorkspace,
            "the Terminal dock target must restore the terminal stage"
        );
    }

    // SF5.2: a right click on a dock target opens the bounded app-name
    // popover as a topmost overlay clamped to the screen.
    #[test]
    fn right_click_opens_bounded_name_popover() {
        let (mut app, terminal_rect, _files_rect) = app_with_dock_targets();
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Right),
            terminal_rect.x,
            terminal_rect.y,
        ));
        assert_eq!(app.state.mode, Mode::ContextMenu);
        let menu = app.state.context_menu.as_ref().expect("dock popover");
        assert!(
            matches!(
                menu.kind,
                ContextMenuKind::AppDock {
                    app: crate::ui::surface_host::BuiltInAppId::Terminal
                }
            ),
            "the popover targets the exact dock entry"
        );
        assert_eq!(menu.items(), vec!["Terminal"]);
        let popup = app.state.context_menu_rect().expect("popover rect");
        let screen = app.state.screen_rect();
        assert_eq!(
            popup.intersection(screen),
            popup,
            "the popover must stay inside the screen"
        );
    }

    // SF5.2: while the dock popover is open, background input is consumed by
    // the topmost overlay (the SF4.2 ContextMenu machinery).
    #[test]
    fn popover_blocks_background_input() {
        let (mut app, terminal_rect, _files_rect) = app_with_dock_targets();
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Right),
            terminal_rect.x,
            terminal_rect.y,
        ));
        assert_eq!(app.state.mode, Mode::ContextMenu, "fixture: popover open");

        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 40, 10));
        assert!(
            app.state.selection.is_none(),
            "a background press under the popover must not reach the terminal"
        );
    }

    // SF5.2 characterization: after the terminal shrinks, the popover rect
    // re-clamps fully inside the new screen (C3.2 clamping precedent).
    #[test]
    fn popover_reanchors_or_closes_after_terminal_resize() {
        let (mut app, terminal_rect, _files_rect) = app_with_dock_targets();
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Right),
            terminal_rect.x,
            terminal_rect.y,
        ));
        assert_eq!(app.state.mode, Mode::ContextMenu, "fixture: popover open");

        app.state.view.terminal_area = Rect::new(0, 0, 40, 8);
        app.state.view.sidebar_rect = Rect::new(0, 0, 0, 8);
        let popup = app.state.context_menu_rect().expect("popover rect");
        let screen = app.state.screen_rect();
        assert_eq!(
            popup.intersection(screen),
            popup,
            "the popover must reanchor inside the shrunken screen"
        );
    }

    // SF5.2: a disabled dock target consumes the click fail-closed without
    // activating anything.
    #[test]
    fn disabled_app_target_is_consumed_without_activation() {
        let (mut app, _terminal_rect, files_rect) = app_with_dock_targets();
        for entry in &mut app.state.view.app_dock_entry_areas {
            entry.enabled = false;
        }
        let stage_before = app.state.stage;

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            files_rect.x,
            files_rect.y,
        ));
        assert!(
            app.state.file_manager.is_none(),
            "a disabled Files target must not open the Files surface"
        );
        assert_eq!(app.state.stage, stage_before);

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Right),
            files_rect.x,
            files_rect.y,
        ));
        assert_ne!(
            app.state.mode,
            Mode::ContextMenu,
            "a disabled target must not open the popover"
        );
    }

    #[test]
    fn tab_click_survives_stray_drag_report_off_the_tab_bar() {
        let mut app = app_for_mouse_test();
        let mut ws = Workspace::test_new("test");
        ws.test_add_tab(None);
        ws.set_active_tab(1);
        app.state.workspaces = vec![ws];
        app.state.active = Some(0);
        app.state.selected = 0;
        let area = Rect::new(0, 0, 106, 20);
        crate::ui::compute_view(&mut app.state, area);

        let first_tab = app.state.view.tab_hit_areas[0];
        let press_col = first_tab.x + 1;
        let stray_row = area.height - 1;

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            press_col,
            first_tab.y,
        ));
        app.handle_mouse(mouse(
            MouseEventKind::Drag(MouseButton::Left),
            press_col,
            stray_row,
        ));
        app.handle_mouse(mouse(
            MouseEventKind::Up(MouseButton::Left),
            press_col,
            stray_row,
        ));

        assert_eq!(app.state.workspaces[0].active_tab_index(), 0);
    }

    #[test]
    fn workspace_click_survives_stray_drag_report_off_the_workspace_list() {
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![Workspace::test_new("first"), Workspace::test_new("second")];
        app.state.active = Some(1);
        app.state.selected = 1;
        let area = Rect::new(0, 0, 106, 20);
        crate::ui::compute_view(&mut app.state, area);

        let first_workspace = app.state.view.workspace_card_areas[0];
        let press_col = first_workspace.rect.x + 1;
        let press_row = first_workspace.rect.y;
        let stray_row = area.height - 1;
        assert!(app.state.workspace_drop_target_at_row(stray_row).is_none());

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            press_col,
            press_row,
        ));
        app.handle_mouse(mouse(
            MouseEventKind::Drag(MouseButton::Left),
            press_col,
            stray_row,
        ));
        app.handle_mouse(mouse(
            MouseEventKind::Up(MouseButton::Left),
            press_col,
            stray_row,
        ));

        assert_eq!(app.state.active, Some(0));
    }

    #[test]
    fn concurrent_input_sources_keep_their_tab_clicks() {
        let mut app = app_for_mouse_test();
        let mut ws = Workspace::test_new("test");
        ws.test_add_tab(None);
        ws.test_add_tab(None);
        ws.set_active_tab(2);
        app.state.workspaces = vec![ws];
        app.state.active = Some(0);
        app.state.selected = 0;
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));

        let first_tab = app.state.view.tab_hit_areas[0];
        let second_tab = app.state.view.tab_hit_areas[1];
        app.handle_mouse_from_input_source(
            41,
            mouse(
                MouseEventKind::Down(MouseButton::Left),
                first_tab.x + 1,
                first_tab.y,
            ),
        );
        app.handle_mouse_from_input_source(
            42,
            mouse(
                MouseEventKind::Down(MouseButton::Left),
                second_tab.x + 1,
                second_tab.y,
            ),
        );

        app.handle_mouse_from_input_source(
            41,
            mouse(
                MouseEventKind::Up(MouseButton::Left),
                first_tab.x + 1,
                first_tab.y,
            ),
        );
        assert_eq!(app.state.workspaces[0].active_tab_index(), 0);

        app.handle_mouse_from_input_source(
            42,
            mouse(
                MouseEventKind::Up(MouseButton::Left),
                second_tab.x + 1,
                second_tab.y,
            ),
        );
        assert_eq!(app.state.workspaces[0].active_tab_index(), 1);
    }

    #[test]
    fn concurrent_input_sources_keep_their_workspace_clicks() {
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![
            Workspace::test_new("first"),
            Workspace::test_new("second"),
            Workspace::test_new("third"),
        ];
        app.state.active = Some(2);
        app.state.selected = 2;
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));

        let first = app.state.view.workspace_card_areas[0].rect;
        let second = app.state.view.workspace_card_areas[1].rect;
        app.handle_mouse_from_input_source(
            41,
            mouse(
                MouseEventKind::Down(MouseButton::Left),
                first.x + 1,
                first.y,
            ),
        );
        app.handle_mouse_from_input_source(
            42,
            mouse(
                MouseEventKind::Down(MouseButton::Left),
                second.x + 1,
                second.y,
            ),
        );

        app.handle_mouse_from_input_source(
            41,
            mouse(MouseEventKind::Up(MouseButton::Left), first.x + 1, first.y),
        );
        assert_eq!(app.state.active, Some(0));

        app.handle_mouse_from_input_source(
            42,
            mouse(
                MouseEventKind::Up(MouseButton::Left),
                second.x + 1,
                second.y,
            ),
        );
        assert_eq!(app.state.active, Some(1));
    }

    #[test]
    fn tab_click_completes_while_other_source_reorders() {
        let mut app = app_for_mouse_test();
        let mut ws = Workspace::test_new("test");
        ws.test_add_tab(None);
        ws.test_add_tab(None);
        ws.set_active_tab(2);
        app.state.workspaces = vec![ws];
        app.state.active = Some(0);
        app.state.selected = 0;
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));

        let first_tab = app.state.view.tab_hit_areas[0];
        let second_tab = app.state.view.tab_hit_areas[1];
        let last_tab = app.state.view.tab_hit_areas[2];
        let drop_col = last_tab.x + last_tab.width;
        app.handle_mouse_from_input_source(
            41,
            mouse(
                MouseEventKind::Down(MouseButton::Left),
                first_tab.x + 1,
                first_tab.y,
            ),
        );
        app.handle_mouse_from_input_source(
            41,
            mouse(
                MouseEventKind::Drag(MouseButton::Left),
                drop_col,
                first_tab.y,
            ),
        );
        app.handle_mouse_from_input_source(
            42,
            mouse(
                MouseEventKind::Down(MouseButton::Left),
                second_tab.x + 1,
                second_tab.y,
            ),
        );

        app.handle_mouse_from_input_source(
            42,
            mouse(
                MouseEventKind::Up(MouseButton::Left),
                second_tab.x + 1,
                second_tab.y,
            ),
        );
        assert_eq!(app.state.workspaces[0].active_tab_index(), 1);
        assert!(matches!(
            app.state.drag.as_ref().map(|drag| &drag.target),
            Some(DragTarget::TabReorder { source_id: 41, .. })
        ));

        app.handle_mouse_from_input_source(
            41,
            mouse(MouseEventKind::Up(MouseButton::Left), drop_col, first_tab.y),
        );
        assert!(app.state.drag.is_none());
    }

    #[test]
    fn releasing_input_source_clears_only_its_pending_tab_click() {
        let mut app = app_for_mouse_test();
        let mut ws = Workspace::test_new("test");
        ws.test_add_tab(None);
        app.state.workspaces = vec![ws];
        app.state.active = Some(0);
        app.state.selected = 0;
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));

        let first_tab = app.state.view.tab_hit_areas[0];
        let second_tab = app.state.view.tab_hit_areas[1];
        app.handle_mouse_from_input_source(
            41,
            mouse(
                MouseEventKind::Down(MouseButton::Left),
                first_tab.x + 1,
                first_tab.y,
            ),
        );
        app.handle_mouse_from_input_source(
            42,
            mouse(
                MouseEventKind::Down(MouseButton::Left),
                second_tab.x + 1,
                second_tab.y,
            ),
        );

        app.clear_input_source(41);

        assert!(!app.state.tab_presses.contains_key(&41));
        assert!(app.state.tab_presses.contains_key(&42));
    }

    #[tokio::test]
    async fn other_input_source_pane_gesture_is_not_swallowed_by_chrome_press() {
        let mut app = app_for_mouse_test();
        let mut ws = Workspace::test_new("test");
        ws.test_add_tab(None);
        app.state.workspaces = vec![ws];
        app.state.active = Some(0);
        app.state.selected = 0;
        let area = Rect::new(0, 0, 106, 20);
        crate::ui::compute_view(&mut app.state, area);

        let info = app.state.view.pane_infos[0].clone();
        let pane_id = info.id;
        let (runtime, mut input_rx) =
            crate::terminal::TerminalRuntime::test_with_channel_and_scrollback_bytes(
                info.inner_rect.width,
                info.inner_rect.height,
                0,
                b"\x1b[?1000h\x1b[?1006h",
                4,
            );
        app.state.insert_test_runtime(pane_id, runtime);
        crate::ui::compute_view(&mut app.state, area);

        let first_tab = app.state.view.tab_hit_areas[0];
        let pane_col = info.inner_rect.x;
        let pane_row = info.inner_rect.y;
        app.handle_mouse_from_input_source(
            41,
            mouse(
                MouseEventKind::Down(MouseButton::Left),
                first_tab.x + 1,
                first_tab.y,
            ),
        );
        app.handle_mouse_from_input_source(
            42,
            mouse(MouseEventKind::Down(MouseButton::Left), pane_col, pane_row),
        );
        app.handle_mouse_from_input_source(
            42,
            mouse(MouseEventKind::Up(MouseButton::Left), pane_col, pane_row),
        );

        assert_eq!(
            input_rx.try_recv().expect("other source mouse down"),
            Bytes::from_static(b"\x1b[<0;1;1M")
        );
        assert_eq!(
            input_rx.try_recv().expect("other source mouse up"),
            Bytes::from_static(b"\x1b[<0;1;1m")
        );
    }

    #[test]
    fn other_input_source_cannot_release_tab_reorder() {
        let mut app = app_for_mouse_test();
        let mut ws = Workspace::test_new("test");
        ws.test_add_tab(Some("second"));
        ws.test_add_tab(Some("third"));
        app.state.workspaces = vec![ws];
        app.state.active = Some(0);
        app.state.selected = 0;
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));

        let source = app.state.view.tab_hit_areas[0];
        let target = app.state.view.tab_hit_areas[2];
        let drop_col = target.x + target.width;
        app.handle_mouse_from_input_source(
            41,
            mouse(
                MouseEventKind::Down(MouseButton::Left),
                source.x + 1,
                source.y,
            ),
        );
        app.handle_mouse_from_input_source(
            41,
            mouse(MouseEventKind::Drag(MouseButton::Left), drop_col, source.y),
        );
        app.handle_mouse_from_input_source(
            42,
            mouse(MouseEventKind::Up(MouseButton::Left), drop_col, source.y),
        );

        assert!(app.state.drag.is_some());
        assert_eq!(app.state.workspaces[0].tabs[0].custom_name, None);

        app.handle_mouse_from_input_source(
            41,
            mouse(MouseEventKind::Up(MouseButton::Left), drop_col, source.y),
        );

        assert!(app.state.drag.is_none());
        assert_eq!(app.state.workspaces[0].tabs[2].custom_name.as_deref(), None);
    }

    #[tokio::test]
    async fn tab_click_survives_stray_drag_report_into_a_mouse_reporting_pane() {
        let mut app = app_for_mouse_test();
        let mut ws = Workspace::test_new("test");
        ws.test_add_tab(None);
        ws.set_active_tab(1);
        app.state.workspaces = vec![ws];
        app.state.active = Some(0);
        app.state.selected = 0;
        let area = Rect::new(0, 0, 106, 20);
        crate::ui::compute_view(&mut app.state, area);

        let info = app
            .state
            .view
            .pane_infos
            .first()
            .cloned()
            .expect("visible pane");
        app.state.insert_test_runtime(
            info.id,
            crate::terminal::TerminalRuntime::test_with_screen_bytes(
                info.inner_rect.width.max(1),
                info.inner_rect.height.max(1),
                b"\x1b[?1002h",
            ),
        );
        crate::ui::compute_view(&mut app.state, area);

        let first_tab = app.state.view.tab_hit_areas[0];
        let press_col = first_tab.x + 1;
        let stray_row = info.inner_rect.bottom().saturating_sub(1);
        assert!(
            app.state.pane_mouse_target(press_col, stray_row).is_some(),
            "stray coordinates must land on the pane for this to test anything"
        );

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            press_col,
            first_tab.y,
        ));
        app.handle_mouse(mouse(
            MouseEventKind::Drag(MouseButton::Left),
            press_col,
            stray_row,
        ));
        app.handle_mouse(mouse(
            MouseEventKind::Up(MouseButton::Left),
            press_col,
            stray_row,
        ));

        assert_eq!(app.state.workspaces[0].active_tab_index(), 0);
    }

    fn mark_worktree_space_member(workspace: &mut Workspace, ws_idx: usize, key: &str) {
        workspace.worktree_space = Some(crate::workspace::WorktreeSpaceMembership {
            key: key.into(),
            label: "herdr".into(),
            repo_root: "/repo/herdr".into(),
            checkout_path: format!("/repo/worktree-{ws_idx}").into(),
            is_linked_worktree: ws_idx != 0,
        });
    }

    #[tokio::test]
    async fn terminal_wheel_uses_configured_mouse_scroll_lines() {
        let mut app = app_for_mouse_test();
        let mut ws = Workspace::test_new("test");
        let pane_id = ws.tabs[0].root_pane;
        let pane_infos = ws.tabs[0].layout.panes(Rect::new(26, 2, 80, 18));
        let info = pane_infos[0].clone();
        ws.tabs[0].runtimes.insert(
            pane_id,
            crate::terminal::TerminalRuntime::test_with_scrollback_bytes(
                info.inner_rect.width,
                info.inner_rect.height,
                16 * 1024,
                &numbered_lines_bytes(64),
            ),
        );

        app.state.workspaces = vec![ws];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;
        app.state.view.pane_infos = pane_infos;
        app.state.mouse_scroll_lines = 7;

        app.handle_mouse(mouse(
            MouseEventKind::ScrollUp,
            info.inner_rect.x + 1,
            info.inner_rect.y + 1,
        ));

        let metrics = app
            .state
            .runtime_for_pane_in_workspace(&app.terminal_runtimes, 0, pane_id)
            .and_then(crate::terminal::TerminalRuntime::scroll_metrics)
            .expect("scroll metrics after wheel");
        assert_eq!(metrics.offset_from_bottom, 7);
    }

    fn mouse_reporting_pane_with_scrollback(
        info: &PaneInfo,
    ) -> (
        crate::terminal::TerminalRuntime,
        tokio::sync::mpsc::Receiver<Bytes>,
    ) {
        let mut bytes = b"\x1b[?1000h\x1b[?1006h".to_vec();
        bytes.extend_from_slice(&numbered_lines_bytes(64));
        crate::terminal::TerminalRuntime::test_with_channel_and_scrollback_bytes(
            info.inner_rect.width,
            info.inner_rect.height,
            16 * 1024,
            &bytes,
            4,
        )
    }

    /// TP-MOB-56: a phone has no wheel and no finger-reachable scrollbar, so a
    /// two-finger swipe is the only way to reach a pane's scrollback. Termius
    /// reports that swipe as an SGR wheel event, and handing it to a
    /// mouse-reporting agent swallows it — the content area looks frozen. In
    /// the mobile shell the vertical wheel drives Herdr's own viewport instead.
    #[tokio::test]
    async fn mobile_shell_vertical_wheel_scrolls_the_pane_instead_of_reporting_it() {
        let mut app = app_for_mouse_test();
        let mut ws = Workspace::test_new("test");
        let pane_id = ws.tabs[0].root_pane;
        let pane_infos = ws.tabs[0].layout.panes(Rect::new(0, 2, 40, 18));
        let info = pane_infos[0].clone();
        let (runtime, mut input_rx) = mouse_reporting_pane_with_scrollback(&info);
        ws.insert_test_runtime(pane_id, runtime);

        app.state.workspaces = vec![ws];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;
        app.state.view.layout = ViewLayout::Mobile;
        // The phone shell has no sidebar; the shared fixture's desktop sidebar
        // rect would otherwise sit under the full-width mobile pane.
        app.state.view.sidebar_rect = Rect::default();
        app.state.view.pane_infos = pane_infos;
        app.state.mouse_scroll_lines = 3;

        app.handle_mouse(mouse(
            MouseEventKind::ScrollUp,
            info.inner_rect.x + 1,
            info.inner_rect.y + 1,
        ));

        let metrics = app
            .state
            .runtime_for_pane_in_workspace(&app.terminal_runtimes, 0, pane_id)
            .and_then(crate::terminal::TerminalRuntime::scroll_metrics)
            .expect("scroll metrics after wheel");
        assert_eq!(
            metrics.offset_from_bottom, 3,
            "the mobile shell owns the vertical wheel over pane content"
        );
        assert!(
            input_rx.try_recv().is_err(),
            "a swipe the phone cannot repeat elsewhere must not be spent on the agent"
        );
    }

    /// TP-MOB-97: the phone shell only claims the wheel when it can honour
    /// it. An alternate-screen program (an agent TUI, a pager) has no
    /// scrollback for the host to move — reported live: a swipe inside the
    /// agent did nothing at all, sometimes, depending on which pane was up.
    /// When the host has nothing to scroll, the swipe belongs to the
    /// program, which scrolls its own content with it.
    #[tokio::test]
    async fn mobile_wheel_reaches_an_alt_screen_program_the_host_cannot_scroll_for() {
        let mut app = app_for_mouse_test();
        let mut ws = Workspace::test_new("test");
        let pane_id = ws.tabs[0].root_pane;
        let pane_infos = ws.tabs[0].layout.panes(Rect::new(0, 2, 40, 18));
        let info = pane_infos[0].clone();
        // Mouse reporting on, then the alternate screen: the shape of every
        // full-screen agent TUI. No scrollback exists behind it.
        let bytes = b"\x1b[?1000h\x1b[?1006h\x1b[?1049hAGENT".to_vec();
        let (runtime, mut input_rx) =
            crate::terminal::TerminalRuntime::test_with_channel_and_scrollback_bytes(
                info.inner_rect.width,
                info.inner_rect.height,
                16 * 1024,
                &bytes,
                4,
            );
        ws.insert_test_runtime(pane_id, runtime);

        app.state.workspaces = vec![ws];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;
        app.state.view.layout = ViewLayout::Mobile;
        app.state.view.sidebar_rect = Rect::default();
        app.state.view.pane_infos = pane_infos;
        app.state.mouse_scroll_lines = 3;

        app.handle_mouse(mouse(
            MouseEventKind::ScrollUp,
            info.inner_rect.x + 1,
            info.inner_rect.y + 1,
        ));

        assert!(
            input_rx.try_recv().is_ok(),
            "with nothing for the host to scroll, the swipe reaches the program"
        );
    }

    /// TP-MOB-57: out of the box the override is scoped to the phone shell. On
    /// a desktop layout the wheel still belongs to the program in the pane,
    /// which is what makes scroll work inside its own lists and viewers.
    /// TP-MOB-101 made that scope configurable; this test pins the default, so
    /// it asserts the setting explicitly rather than inheriting it — a default
    /// that drifts must fail here, not in a reader's session.
    #[tokio::test]
    async fn desktop_shell_vertical_wheel_still_reports_to_the_pane() {
        let mut app = app_for_mouse_test();
        let mut ws = Workspace::test_new("test");
        let pane_id = ws.tabs[0].root_pane;
        let pane_infos = ws.tabs[0].layout.panes(Rect::new(26, 2, 80, 18));
        let info = pane_infos[0].clone();
        let (runtime, mut input_rx) = mouse_reporting_pane_with_scrollback(&info);
        ws.insert_test_runtime(pane_id, runtime);

        app.state.workspaces = vec![ws];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;
        app.state.view.pane_infos = pane_infos;
        app.state.mouse_scroll_lines = 3;
        assert_ne!(app.state.view.layout, ViewLayout::Mobile);
        assert_eq!(
            app.state.mouse_wheel_host_scroll,
            crate::config::MouseWheelHostScrollConfig::Mobile,
            "this test describes the shipped default; state it instead of assuming it"
        );

        app.handle_mouse(mouse(
            MouseEventKind::ScrollUp,
            info.inner_rect.x + 1,
            info.inner_rect.y + 1,
        ));

        assert!(
            input_rx.try_recv().is_ok(),
            "the desktop wheel still reaches a mouse-reporting pane"
        );
        let metrics = app
            .state
            .runtime_for_pane_in_workspace(&app.terminal_runtimes, 0, pane_id)
            .and_then(crate::terminal::TerminalRuntime::scroll_metrics)
            .expect("scroll metrics after wheel");
        assert_eq!(
            metrics.offset_from_bottom, 0,
            "reporting the wheel must not also move Herdr's viewport"
        );
    }

    /// TP-MOB-101: the phone shell's claim is a policy, not a shape of the
    /// screen. Reported live on a desktop: the wheel over an agent pane did
    /// nothing at all, because the agent asks for mouse reporting and then
    /// discards the wheel, and the desktop had no way to say otherwise —
    /// the modifier route is not available either, since the host terminal
    /// captures shift+wheel for itself before Herdr ever sees it. Under
    /// `scrollable` the same claim the phone makes applies here.
    #[tokio::test]
    async fn scrollable_mode_gives_the_desktop_wheel_to_host_scrollback() {
        let mut app = app_for_mouse_test();
        let mut ws = Workspace::test_new("test");
        let pane_id = ws.tabs[0].root_pane;
        let pane_infos = ws.tabs[0].layout.panes(Rect::new(26, 2, 80, 18));
        let info = pane_infos[0].clone();
        let (runtime, mut input_rx) = mouse_reporting_pane_with_scrollback(&info);
        ws.insert_test_runtime(pane_id, runtime);

        app.state.workspaces = vec![ws];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;
        app.state.view.pane_infos = pane_infos;
        app.state.mouse_scroll_lines = 3;
        assert_ne!(app.state.view.layout, ViewLayout::Mobile);
        app.state.mouse_wheel_host_scroll = crate::config::MouseWheelHostScrollConfig::Scrollable;

        app.handle_mouse(mouse(
            MouseEventKind::ScrollUp,
            info.inner_rect.x + 1,
            info.inner_rect.y + 1,
        ));

        let metrics = app
            .state
            .runtime_for_pane_in_workspace(&app.terminal_runtimes, 0, pane_id)
            .and_then(crate::terminal::TerminalRuntime::scroll_metrics)
            .expect("scroll metrics after wheel");
        assert_eq!(
            metrics.offset_from_bottom, 3,
            "under scrollable the desktop wheel moves Herdr's viewport"
        );
        // Both halves are asserted on purpose: a wheel that scrolls the host
        // *and* reaches the program would satisfy the first assert alone while
        // leaving the pane's own list scrolled out from under the reader.
        assert!(
            input_rx.try_recv().is_err(),
            "a wheel the host claimed must not also be spent on the pane"
        );
    }

    /// TP-MOB-101: `scrollable` widens who may claim the wheel, never what the
    /// host is able to honour. An alternate-screen program has no scrollback
    /// behind it, so the claim does not apply and the wheel stays the
    /// program's — the same guard TP-MOB-97 put on the phone shell, measured
    /// again: entering the alternate screen reports max_offset 0, leaving it
    /// restores the previous scrollback intact.
    #[tokio::test]
    async fn scrollable_mode_still_leaves_an_alt_screen_program_its_wheel() {
        let mut app = app_for_mouse_test();
        let mut ws = Workspace::test_new("test");
        let pane_id = ws.tabs[0].root_pane;
        let pane_infos = ws.tabs[0].layout.panes(Rect::new(26, 2, 80, 18));
        let info = pane_infos[0].clone();
        let bytes = b"\x1b[?1000h\x1b[?1006h\x1b[?1049hAGENT".to_vec();
        let (runtime, mut input_rx) =
            crate::terminal::TerminalRuntime::test_with_channel_and_scrollback_bytes(
                info.inner_rect.width,
                info.inner_rect.height,
                16 * 1024,
                &bytes,
                4,
            );
        ws.insert_test_runtime(pane_id, runtime);

        app.state.workspaces = vec![ws];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;
        app.state.view.pane_infos = pane_infos;
        app.state.mouse_scroll_lines = 3;
        app.state.mouse_wheel_host_scroll = crate::config::MouseWheelHostScrollConfig::Scrollable;

        app.handle_mouse(mouse(
            MouseEventKind::ScrollUp,
            info.inner_rect.x + 1,
            info.inner_rect.y + 1,
        ));

        assert!(
            input_rx.try_recv().is_ok(),
            "with nothing for the host to scroll, the wheel reaches the program"
        );
    }

    /// TP-MOB-101: `never` is the reverse direction of the same setting, and
    /// it is the only way a phone user has ever been able to opt out of
    /// TP-MOB-56. Without it the mobile claim would be the one policy in this
    /// setting with no off switch.
    #[tokio::test]
    async fn never_mode_takes_the_wheel_back_from_the_phone_shell() {
        let mut app = app_for_mouse_test();
        let mut ws = Workspace::test_new("test");
        let pane_id = ws.tabs[0].root_pane;
        let pane_infos = ws.tabs[0].layout.panes(Rect::new(0, 2, 40, 18));
        let info = pane_infos[0].clone();
        let (runtime, mut input_rx) = mouse_reporting_pane_with_scrollback(&info);
        ws.insert_test_runtime(pane_id, runtime);

        app.state.workspaces = vec![ws];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;
        app.state.view.layout = ViewLayout::Mobile;
        app.state.view.sidebar_rect = Rect::default();
        app.state.view.pane_infos = pane_infos;
        app.state.mouse_scroll_lines = 3;
        app.state.mouse_wheel_host_scroll = crate::config::MouseWheelHostScrollConfig::Never;

        app.handle_mouse(mouse(
            MouseEventKind::ScrollUp,
            info.inner_rect.x + 1,
            info.inner_rect.y + 1,
        ));

        assert!(
            input_rx.try_recv().is_ok(),
            "never hands the wheel to the pane even where the phone would claim it"
        );
        let metrics = app
            .state
            .runtime_for_pane_in_workspace(&app.terminal_runtimes, 0, pane_id)
            .and_then(crate::terminal::TerminalRuntime::scroll_metrics)
            .expect("scroll metrics after wheel");
        assert_eq!(
            metrics.offset_from_bottom, 0,
            "never must not move Herdr's viewport"
        );
    }

    /// TP-MOB-101: the setting is about the vertical wheel only. A horizontal
    /// wheel carries no scrollback meaning for the host, so claiming it would
    /// take a gesture away from panes that do use it and give nothing back.
    #[tokio::test]
    async fn scrollable_mode_does_not_claim_the_horizontal_wheel() {
        let mut app = app_for_mouse_test();
        let mut ws = Workspace::test_new("test");
        let pane_id = ws.tabs[0].root_pane;
        let pane_infos = ws.tabs[0].layout.panes(Rect::new(26, 2, 80, 18));
        let info = pane_infos[0].clone();
        let (runtime, mut input_rx) = mouse_reporting_pane_with_scrollback(&info);
        ws.insert_test_runtime(pane_id, runtime);

        app.state.workspaces = vec![ws];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;
        app.state.view.pane_infos = pane_infos;
        app.state.mouse_scroll_lines = 3;
        app.state.mouse_wheel_host_scroll = crate::config::MouseWheelHostScrollConfig::Scrollable;

        app.handle_mouse(mouse(
            MouseEventKind::ScrollLeft,
            info.inner_rect.x + 1,
            info.inner_rect.y + 1,
        ));

        assert!(
            input_rx.try_recv().is_ok(),
            "the horizontal wheel stays the pane's under every mode"
        );
    }

    #[tokio::test]
    async fn mouse_dispatcher_forwards_horizontal_wheel_to_mouse_reporting_pane() {
        let mut app = app_for_mouse_test();
        let mut ws = Workspace::test_new("test");
        let pane_id = ws.tabs[0].root_pane;
        let pane_infos = ws.tabs[0].layout.panes(Rect::new(26, 2, 80, 18));
        let info = pane_infos[0].clone();
        let (runtime, mut input_rx) =
            crate::terminal::TerminalRuntime::test_with_channel_and_scrollback_bytes(
                info.inner_rect.width,
                info.inner_rect.height,
                0,
                b"\x1b[?1000h\x1b[?1006h",
                4,
            );
        ws.insert_test_runtime(pane_id, runtime);

        app.state.workspaces = vec![ws];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;
        app.state.view.pane_infos = pane_infos;
        assert!(
            app.state.mouse_capture,
            "reproduction must use the default Herdr mouse dispatcher"
        );

        let outer_column = info.inner_rect.x + 2;
        let outer_row = info.inner_rect.y + 3;
        for (button, expected_kind, ingress) in [
            (66, MouseEventKind::ScrollLeft, "monolithic"),
            (67, MouseEventKind::ScrollRight, "headless"),
        ] {
            let input = format!("\x1b[<{button};{};{}M", outer_column + 1, outer_row + 1);
            let mut events = crate::raw_input::parse_raw_input_bytes_sync(input.as_bytes());
            let event = events
                .pop()
                .expect("horizontal SGR wheel input should parse");
            let crate::raw_input::RawInputEvent::Mouse(mouse) = &event else {
                panic!("expected parsed mouse event");
            };
            assert!(events.is_empty(), "expected one parsed mouse event");
            assert_eq!(mouse.kind, expected_kind);

            if ingress == "monolithic" {
                assert!(app.handle_raw_input_event(event).await);
            } else {
                app.route_client_events(vec![event], false);
            }

            assert_eq!(
                input_rx
                    .try_recv()
                    .expect("horizontal wheel should reach pane"),
                Bytes::from(format!("\x1b[<{button};3;4M"))
            );
        }
        assert!(input_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn horizontal_wheel_stays_inert_for_non_mouse_reporting_pane() {
        let mut app = app_for_mouse_test();
        let mut ws = Workspace::test_new("test");
        let pane_id = ws.tabs[0].root_pane;
        let pane_infos = ws.tabs[0].layout.panes(Rect::new(26, 2, 80, 18));
        let info = pane_infos[0].clone();
        let (runtime, mut input_rx) =
            crate::terminal::TerminalRuntime::test_with_channel_and_scrollback_bytes(
                info.inner_rect.width,
                info.inner_rect.height,
                0,
                b"",
                1,
            );
        ws.insert_test_runtime(pane_id, runtime);

        app.state.workspaces = vec![ws];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;
        app.state.view.pane_infos = pane_infos;

        let input = format!(
            "\x1b[<66;{};{}M",
            info.inner_rect.x + 3,
            info.inner_rect.y + 4
        );
        let event = crate::raw_input::parse_raw_input_bytes_sync(input.as_bytes())
            .pop()
            .expect("horizontal SGR wheel input should parse");

        assert!(app.handle_raw_input_event(event).await);

        assert!(input_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn pane_right_click_passthrough_is_isolated() {
        let mut app = app_for_mouse_test();
        let mut ws = Workspace::test_new("test");
        let passthrough_pane = ws.tabs[0].root_pane;
        let default_pane = ws.test_split(Direction::Horizontal);
        ws.pane_state_mut(passthrough_pane)
            .unwrap()
            .right_click_passthrough = true;
        app.state.workspaces = vec![ws];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));

        let passthrough_info = app.state.pane_info_by_id(passthrough_pane).unwrap().clone();
        let default_info = app.state.pane_info_by_id(default_pane).unwrap().clone();
        let (passthrough_runtime, mut passthrough_input) =
            crate::terminal::TerminalRuntime::test_with_channel_and_scrollback_bytes(
                passthrough_info.inner_rect.width,
                passthrough_info.inner_rect.height,
                0,
                b"\x1b[?1002h\x1b[?1006h",
                4,
            );
        let (default_runtime, mut default_input) =
            crate::terminal::TerminalRuntime::test_with_channel_and_scrollback_bytes(
                default_info.inner_rect.width,
                default_info.inner_rect.height,
                0,
                b"\x1b[?1002h\x1b[?1006h",
                4,
            );
        app.state
            .insert_test_runtime(passthrough_pane, passthrough_runtime);
        app.state.insert_test_runtime(default_pane, default_runtime);

        let col = passthrough_info.inner_rect.x + 2;
        let row = passthrough_info.inner_rect.y + 3;
        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Right), col, row));

        assert_eq!(app.state.mode, Mode::Terminal);
        assert!(app.state.context_menu.is_none());
        assert_eq!(
            passthrough_input.try_recv().unwrap(),
            Bytes::from_static(b"\x1b[<2;3;4M")
        );

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Right),
            default_info.inner_rect.x + 2,
            default_info.inner_rect.y + 3,
        ));

        assert!(default_input.try_recv().is_err());
        assert!(matches!(
            app.state.context_menu.as_ref().map(|menu| &menu.kind),
            Some(ContextMenuKind::Pane { pane_id, .. }) if *pane_id == default_pane
        ));
    }

    #[tokio::test]
    async fn pane_right_click_passthrough_falls_back_when_mouse_reporting_is_off() {
        let mut app = app_for_mouse_test();
        let mut ws = Workspace::test_new("test");
        let pane_id = ws.tabs[0].root_pane;
        ws.pane_state_mut(pane_id).unwrap().right_click_passthrough = true;
        app.state.workspaces = vec![ws];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));
        let info = app.state.pane_info_by_id(pane_id).unwrap().clone();
        app.state.insert_test_runtime(
            pane_id,
            crate::terminal::TerminalRuntime::test_with_screen_bytes(
                info.inner_rect.width,
                info.inner_rect.height,
                b"",
            ),
        );

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Right),
            info.inner_rect.x + 2,
            info.inner_rect.y + 3,
        ));

        assert_eq!(app.state.mode, Mode::ContextMenu);
        assert!(app.state.context_menu.is_some());
    }

    #[tokio::test]
    async fn configured_right_click_passthrough_forwards_gesture_outside_pane() {
        let mut app = app_for_mouse_test();
        let mut ws = Workspace::test_new("test");
        let pane_id = ws.tabs[0].root_pane;
        let pane_infos = ws.tabs[0].layout.panes(Rect::new(26, 2, 80, 18));
        let info = pane_infos[0].clone();
        let (runtime, mut input_rx) =
            crate::terminal::TerminalRuntime::test_with_channel_and_scrollback_bytes(
                info.inner_rect.width,
                info.inner_rect.height,
                0,
                b"\x1b[?1002h\x1b[?1006h",
                4,
            );
        ws.insert_test_runtime(pane_id, runtime);

        app.state.workspaces = vec![ws];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;
        app.state.view.pane_infos = pane_infos;
        app.state.right_click_passthrough_modifiers = Some(KeyModifiers::CONTROL);

        let col = info.inner_rect.x + 2;
        let row = info.inner_rect.y + 3;
        app.handle_mouse(MouseEvent {
            modifiers: KeyModifiers::CONTROL,
            ..mouse(MouseEventKind::Down(MouseButton::Right), col, row)
        });
        app.handle_mouse(MouseEvent {
            modifiers: KeyModifiers::CONTROL,
            ..mouse(MouseEventKind::Drag(MouseButton::Right), 0, 0)
        });
        app.handle_mouse(MouseEvent {
            modifiers: KeyModifiers::CONTROL,
            ..mouse(MouseEventKind::Up(MouseButton::Right), 0, 0)
        });

        assert_eq!(app.state.mode, Mode::Terminal);
        assert!(app.state.context_menu.is_none());
        assert!(app.state.right_click_passthrough.is_none());
        assert_eq!(
            input_rx.try_recv().expect("forwarded right mouse down"),
            Bytes::from_static(b"\x1b[<2;3;4M")
        );
        assert_eq!(
            input_rx.try_recv().expect("forwarded right mouse drag"),
            Bytes::from_static(b"\x1b[<34;1;1M")
        );
        assert_eq!(
            input_rx.try_recv().expect("forwarded right mouse up"),
            Bytes::from_static(b"\x1b[<2;1;1m")
        );
        assert!(input_rx.try_recv().is_err());
    }

    // TP-DOTS-02: right-click on the tree's header rows opens their own
    // menus — the node header carries creation, the bucket header carries
    // the fold verbs. Before this road existed the press fell through to
    // nothing, so the tree could only be managed from branch rows.
    #[test]
    fn right_click_on_headers_opens_their_menus() {
        let mut app = app_for_mouse_test();
        let mut main = crate::workspace::Workspace::test_new("main");
        main.worktree_space = Some(crate::workspace::WorktreeSpaceMembership {
            key: "repo-key".into(),
            label: "herdr".into(),
            repo_root: std::path::PathBuf::from("/repo/herdr"),
            checkout_path: std::path::PathBuf::from("/repo/herdr"),
            is_linked_worktree: false,
        });
        main.identity_cwd = std::env::temp_dir();
        let mut child = crate::workspace::Workspace::test_new("issue");
        child.worktree_space = Some(crate::workspace::WorktreeSpaceMembership {
            key: "repo-key".into(),
            label: "herdr".into(),
            repo_root: std::path::PathBuf::from("/repo/herdr"),
            checkout_path: std::path::PathBuf::from("/repo/herdr-issue"),
            is_linked_worktree: true,
        });
        app.state.workspaces = vec![main, child];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mobile_width_threshold = 0;
        app.state.space_projects = vec![crate::spaces::SpaceProject {
            key: "project:herdr".into(),
            name: "herdr".into(),
            icon: None,
            repo_roots: vec![std::path::PathBuf::from("/repo/herdr")],
            space_keys: Vec::new(),
        }];

        let area = ratatui::layout::Rect::new(0, 0, 106, 20);
        crate::ui::compute_view(&mut app.state, area);
        let project_head = app.state.view.workspace_project_header_areas[0].clone();
        let group_head = app.state.view.workspace_group_header_areas[0].clone();

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Right),
            project_head.rect.x,
            project_head.rect.y,
        ));
        assert!(
            matches!(
                app.state.context_menu.as_ref().map(|menu| &menu.kind),
                Some(crate::app::state::ContextMenuKind::NodeHeader { node_key, .. })
                    if node_key == "project:herdr"
            ),
            "a node header owns its own menu; got {:?}",
            app.state.context_menu.as_ref().map(|menu| &menu.kind)
        );

        app.state.context_menu = None;
        app.state.mode = crate::app::state::Mode::Terminal;
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Right),
            group_head.rect.x,
            group_head.rect.y,
        ));
        assert!(
            matches!(
                app.state.context_menu.as_ref().map(|menu| &menu.kind),
                Some(crate::app::state::ContextMenuKind::SpaceHeader { space_key, .. })
                    if space_key == "repo-key"
            ),
            "a bucket header owns its fold menu; got {:?}",
            app.state.context_menu.as_ref().map(|menu| &menu.kind)
        );
    }

    // TP-DOTS-04: the "⋯" is a second road to the SAME menu the right-click
    // opens — a left press on it must never fold, switch, or invent a menu
    // of its own; and a left press on the rest of the header row still folds.
    #[test]
    fn a_left_press_on_the_dots_opens_the_row_menu_and_the_rest_still_folds() {
        let mut app = app_for_mouse_test();
        let mut main = crate::workspace::Workspace::test_new("main");
        main.worktree_space = Some(crate::workspace::WorktreeSpaceMembership {
            key: "repo-key".into(),
            label: "herdr".into(),
            repo_root: std::path::PathBuf::from("/repo/herdr"),
            checkout_path: std::path::PathBuf::from("/repo/herdr"),
            is_linked_worktree: false,
        });
        main.identity_cwd = std::env::temp_dir();
        let mut child = crate::workspace::Workspace::test_new("issue");
        child.worktree_space = Some(crate::workspace::WorktreeSpaceMembership {
            key: "repo-key".into(),
            label: "herdr".into(),
            repo_root: std::path::PathBuf::from("/repo/herdr"),
            checkout_path: std::path::PathBuf::from("/repo/herdr-issue"),
            is_linked_worktree: true,
        });
        app.state.workspaces = vec![main, child];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mobile_width_threshold = 0;
        app.state.mouse_capture = true;

        let area = ratatui::layout::Rect::new(0, 0, 106, 20);
        crate::ui::compute_view(&mut app.state, area);
        let group_head = app.state.view.workspace_group_header_areas[0].clone();
        let card = app.state.view.workspace_card_areas[0];

        // The header's dots open the bucket menu instead of folding.
        let head_dots = crate::ui::header_menu_cell(group_head.rect);
        assert!(head_dots.width > 0, "the header reserves a manage cell");
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            head_dots.x,
            head_dots.y,
        ));
        assert!(
            matches!(
                app.state.context_menu.as_ref().map(|menu| &menu.kind),
                Some(crate::app::state::ContextMenuKind::SpaceHeader { space_key, .. })
                    if space_key == "repo-key"
            ),
            "the dots open the same menu the right-click does; got {:?}",
            app.state.context_menu.as_ref().map(|menu| &menu.kind)
        );
        assert!(
            !app.state.collapsed_space_keys.contains("repo-key"),
            "the dots never fold the group"
        );

        // The rest of the header row keeps its fold behavior.
        app.state.context_menu = None;
        app.state.mode = crate::app::state::Mode::Terminal;
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            group_head.rect.x,
            group_head.rect.y,
        ));
        assert!(
            app.state.collapsed_space_keys.contains("repo-key"),
            "a press outside the dots still folds"
        );
        app.state.collapsed_space_keys.remove("repo-key");

        // The card's dots open the branch row's own (git) menu.
        crate::ui::compute_view(&mut app.state, area);
        let card_dots = crate::ui::workspace_menu_cell(card.rect);
        assert!(card_dots.width > 0, "the card reserves a manage cell");
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            card_dots.x,
            card_dots.y,
        ));
        assert!(
            matches!(
                app.state.context_menu.as_ref().map(|menu| &menu.kind),
                Some(crate::app::state::ContextMenuKind::GitWorkspace { .. })
            ),
            "the card dots open the branch menu; got {:?}",
            app.state.context_menu.as_ref().map(|menu| &menu.kind)
        );
    }

    // TP-DRAW-11: a press on the "older chats" row opens the drawer the rest
    // of the way and a second press folds it back. It is matched from its own
    // vector, so it can never resume the chat drawn above it.
    #[test]
    fn a_press_on_the_older_chats_row_opens_the_drawer_and_folds_it_back() {
        let mut app = app_for_mouse_test();
        let mut main = crate::workspace::Workspace::test_new("main");
        main.worktree_space = Some(crate::workspace::WorktreeSpaceMembership {
            key: "repo-key".into(),
            label: "herdr".into(),
            repo_root: std::path::PathBuf::from("/repo/herdr"),
            checkout_path: std::path::PathBuf::from("/repo/herdr"),
            is_linked_worktree: false,
        });
        main.identity_cwd = std::path::PathBuf::from("/repo/herdr");
        app.state.workspaces = vec![main];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mobile_width_threshold = 0;
        let key =
            crate::persist::workspace_chats::ledger_key(&std::path::PathBuf::from("/repo/herdr"));
        app.state.workspace_chat_rows.insert(
            key.clone(),
            (0..9)
                .map(|i| crate::app::state::WorkspaceChatRow {
                    session_id: format!("session-{i}"),
                    agent: "claude".to_string(),
                    title: Some(format!("chat {i}")),
                    last_seen_ms: 1_000 + i as u64,
                    last_modified: None,
                    last_message_at: None,
                })
                .collect(),
        );
        app.state.expanded_chat_workspaces.insert(key);

        let area = ratatui::layout::Rect::new(0, 0, 106, 30);
        crate::ui::compute_view(&mut app.state, area);
        let row = app
            .state
            .view
            .workspace_more_chats_areas
            .first()
            .cloned()
            .expect("the deep drawer lays out its older-chats row");
        let ledger =
            crate::persist::workspace_chats::ledger_key(&std::path::PathBuf::from("/repo/herdr"));
        assert!(!app.state.fully_open_chat_drawers.contains(&ledger));

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            row.rect.x + 4,
            row.rect.y,
        ));
        assert!(
            app.state.fully_open_chat_drawers.contains(&ledger),
            "the press opens the drawer the rest of the way"
        );
        assert_eq!(
            app.state.request_project_chat_tab, None,
            "it resumes nothing — the row above it owns that click"
        );

        crate::ui::compute_view(&mut app.state, area);
        let row = app
            .state
            .view
            .workspace_more_chats_areas
            .first()
            .cloned()
            .expect("the row stays as the way back");
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            row.rect.x + 4,
            row.rect.y,
        ));
        assert!(
            !app.state.fully_open_chat_drawers.contains(&ledger),
            "a second press folds it back"
        );
    }

    // TP-AGPANEL-03: a right-click on an agents-panel row opens that row's
    // own menu carrying its pane; the empty space below the last row opens
    // nothing, because there is no agent there to act on. Before this road
    // existed the press fell through to the workspace list's resolver, which
    // reads by row and answers for a different section entirely.
    // TP-CHAT-MOVE-08 (P1+P2): a daily row owns a menu, and that menu carries
    // THAT row's chat. The daily section was the one place a chat could be
    // seen and not filed — backwards, because those are exactly the chats that
    // belong to no checkout yet and most need somewhere to go.
    #[test]
    fn right_click_on_a_daily_row_opens_a_menu_carrying_that_chat() {
        let mut app = app_for_mouse_test();
        let workspace = crate::workspace::Workspace::test_new("one");
        app.state.workspaces = vec![workspace];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mobile_width_threshold = 0;

        let daily = std::env::temp_dir().join("herdr-daily-menu-fixture");
        app.state.daily_chat_cwd = Some(daily.clone());
        let key = crate::persist::workspace_chats::ledger_key(&daily);
        app.state.workspace_chat_rows.insert(
            key,
            vec![
                crate::app::state::WorkspaceChatRow {
                    session_id: "daily-a".to_string(),
                    agent: "claude".to_string(),
                    title: Some("first".to_string()),
                    last_seen_ms: 2_000,
                    last_modified: None,
                    last_message_at: None,
                },
                crate::app::state::WorkspaceChatRow {
                    session_id: "daily-b".to_string(),
                    agent: "claude".to_string(),
                    title: Some("second".to_string()),
                    last_seen_ms: 1_000,
                    last_modified: None,
                    last_message_at: None,
                },
            ],
        );
        // The row the press will land on — laid out the way render lays it out.
        app.state.view.daily_chat_row_areas = vec![crate::app::state::DailyChatRowArea {
            rect: ratatui::layout::Rect::new(0, 7, 20, 1),
            session_id: "daily-b".to_string(),
        }];

        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Right), 3, 7));

        match app.state.context_menu.as_ref().map(|menu| &menu.kind) {
            Some(crate::app::state::ContextMenuKind::WorkspaceChat {
                ws_idx, session_id, ..
            }) => {
                assert_eq!(
                    session_id, "daily-b",
                    "the menu carries the row that was pressed, not the first in the list"
                );
                assert_eq!(
                    *ws_idx, None,
                    "a daily chat belongs to no drawer, so it names none"
                );
            }
            other => panic!("a daily row opens its own menu; got {other:?}"),
        }
    }

    #[test]
    fn right_click_on_an_agent_row_opens_its_menu_and_the_panel_gap_stays_inert() {
        let mut app = app_for_mouse_test();
        let workspace = crate::workspace::Workspace::test_new("one");
        let root = workspace.tabs[0].root_pane;
        app.state.workspaces = vec![workspace];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mobile_width_threshold = 0;
        app.state.ensure_test_terminals();
        let terminal_id = app.state.workspaces[0].tabs[0]
            .panes
            .get(&root)
            .map(|pane| pane.attached_terminal_id.clone())
            .expect("the root pane has a terminal");
        if let Some(terminal) = app.state.terminals.get_mut(&terminal_id) {
            terminal.set_detected_state(
                Some(crate::detect::Agent::Pi),
                crate::detect::AgentState::Idle,
            );
        }

        let area = ratatui::layout::Rect::new(0, 0, 106, 20);
        crate::ui::compute_view(&mut app.state, area);
        let entries = crate::ui::agent_panel_entries(&app.state);
        assert_eq!(entries.len(), 1, "the fixture lists one agent");

        let panel = app.state.agent_panel_rect();
        let row = (panel.y..panel.y + panel.height)
            .find(|row| app.state.agent_detail_target_at(*row) == Some((0, 0, root)))
            .expect("the agent row is laid out");

        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Right), 2, row));
        assert!(
            matches!(
                app.state.context_menu.as_ref().map(|menu| &menu.kind),
                Some(crate::app::state::ContextMenuKind::AgentEntry { ws_idx: 0, tab_idx: 0, pane_id, .. })
                    if *pane_id == root
            ),
            "the agent row owns a menu carrying its own pane; got {:?}",
            app.state.context_menu.as_ref().map(|menu| &menu.kind)
        );

        // The empty space under the last row belongs to no agent.
        app.state.context_menu = None;
        app.state.mode = crate::app::state::Mode::Terminal;
        if let Some(gap) = (panel.y..panel.y + panel.height)
            .find(|row| app.state.agent_detail_target_at(*row).is_none())
        {
            app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Right), 2, gap));
            assert!(
                !matches!(
                    app.state.context_menu.as_ref().map(|menu| &menu.kind),
                    Some(crate::app::state::ContextMenuKind::AgentEntry { .. })
                ),
                "the panel's empty space invents no agent menu; got {:?}",
                app.state.context_menu.as_ref().map(|menu| &menu.kind)
            );
        }
    }

    // TP-FM-DISMISS-01: clicking an agent's row asks to SEE that pane.
    // With the Files stage covering the center, clicking the ALREADY-focused
    // agent used to be a silent no-op — focus did not change, so the screen
    // stayed on Files. The click now dismisses Files first, uniformly for the
    // same agent, a different agent, and the collapsed rail.
    // TP-SBS-DRAG-01: integer floor on both the bound and the division can
    // land one percent outside the band — total 101 floors 20 cells to 19%
    // and 81 cells to 80%+floor artefacts. The clamp is that last percent.
    #[test]
    fn the_committed_ratio_never_leaves_the_band() {
        assert_eq!(
            crate::app::state::AppState::sbs_ratio_from_tracks_impl(20, 81),
            20,
            "total 101: the floor lands at 19 without the clamp"
        );
        assert_eq!(
            crate::app::state::AppState::sbs_ratio_from_tracks_impl(81, 20),
            80
        );
        assert_eq!(
            crate::app::state::AppState::sbs_ratio_from_tracks_impl(0, 0),
            20
        );
        assert_eq!(
            crate::app::state::AppState::sbs_ratio_from_tracks_impl(60, 60),
            50
        );
    }

    fn agent_dnd_app() -> (
        crate::app::App,
        crate::layout::PaneId,
        crate::layout::PaneId,
    ) {
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![
            crate::workspace::Workspace::test_new("alpha"),
            crate::workspace::Workspace::test_new("beta"),
        ];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = crate::app::state::Mode::Terminal;
        app.state.mobile_width_threshold = 0;
        app.state.ensure_test_terminals();
        let p0 = app.state.workspaces[0].tabs[0].root_pane;
        let p1 = app.state.workspaces[1].tabs[0].root_pane;
        // The panel lists detected agents — undetected shells own no row.
        for ws in 0..2 {
            let terminal_id = app.state.workspaces[ws].tabs[0]
                .panes
                .values()
                .next()
                .map(|pane| pane.attached_terminal_id.clone())
                .expect("root pane has a terminal");
            if let Some(terminal) = app.state.terminals.get_mut(&terminal_id) {
                terminal.set_detected_state(
                    Some(crate::detect::Agent::Pi),
                    crate::detect::AgentState::Idle,
                );
            }
        }
        crate::ui::compute_view(&mut app.state, ratatui::layout::Rect::new(0, 0, 120, 40));
        (app, p0, p1)
    }

    fn agent_row_of(app: &crate::app::App, pane: crate::layout::PaneId) -> u16 {
        let panel = app.state.agent_panel_rect();
        (panel.y..panel.y + panel.height)
            .find(|row| {
                app.state
                    .agent_detail_target_at(*row)
                    .is_some_and(|(_, _, id)| id == pane)
            })
            .expect("the pane owns a panel row")
    }

    // TP-AGDND-01: a press that never leaves its row is a click — it focuses
    // the pane exactly as before and writes no order.
    #[test]
    fn a_plain_agent_row_click_stays_a_click() {
        let (mut app, p0, _) = agent_dnd_app();
        let row = agent_row_of(&app, p0);

        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 3, row));
        // A real press wobbles: the terminal reports a Drag on the SAME row.
        // The one-row threshold is exactly what keeps this a click.
        app.handle_mouse(mouse(MouseEventKind::Drag(MouseButton::Left), 5, row));
        app.handle_mouse(mouse(MouseEventKind::Up(MouseButton::Left), 5, row));

        assert!(
            app.state.agent_panel_order.is_empty(),
            "a wobbling click writes no hand-made order"
        );
        assert!(
            app.state.agent_panel_drag.is_none(),
            "the candidate is released"
        );
    }

    // TP-AGDND-01: crossing a row boundary converts the press into a drag;
    // Up commits the dragged pane into the drop row's position, and the next
    // computed panel draws that order.
    #[test]
    fn dragging_an_agent_row_reorders_the_panel() {
        let (mut app, p0, p1) = agent_dnd_app();
        let row0 = agent_row_of(&app, p0);
        let row1 = agent_row_of(&app, p1);
        assert!(row0 < row1, "natural order draws alpha first");

        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 3, row0));
        app.handle_mouse(mouse(MouseEventKind::Drag(MouseButton::Left), 3, row1));
        app.handle_mouse(mouse(MouseEventKind::Up(MouseButton::Left), 3, row1));

        let order: Vec<_> = crate::ui::agent_panel_entries(&app.state)
            .iter()
            .map(|entry| entry.pane_id)
            .collect();
        assert_eq!(
            order,
            vec![p1, p0],
            "the dragged row landed in the drop position"
        );

        // The order is remembered, not a one-frame illusion.
        crate::ui::compute_view(&mut app.state, ratatui::layout::Rect::new(0, 0, 120, 40));
        let again: Vec<_> = crate::ui::agent_panel_entries(&app.state)
            .iter()
            .map(|entry| entry.pane_id)
            .collect();
        assert_eq!(again, vec![p1, p0], "the order survives the next compute");
    }

    fn sbs_drag_app() -> crate::app::App {
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![
            crate::workspace::Workspace::test_new("left"),
            crate::workspace::Workspace::test_new("right"),
        ];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = crate::app::state::Mode::Terminal;
        app.state.mobile_width_threshold = 0;
        app.state.ensure_test_terminals();
        app.state.side_by_side = Some(crate::app::state::SideBySideView {
            right: crate::app::state::SideBySideRight::Workspace(1),
            ratio_percent: 50,
            focus: Default::default(),
        });
        crate::ui::compute_view(&mut app.state, ratatui::layout::Rect::new(0, 0, 120, 30));
        app
    }

    // TP-SBS-DRAG-01: the one empty column between the halves is a grab
    // handle — Down on it arms the shared resize transaction, Drag moves the
    // split live, Up commits the new ratio (clamped to the 20-80 the state
    // already promises), and the released pointer stops owning anything.
    #[test]
    fn dragging_the_sbs_divider_commits_a_new_ratio() {
        let mut app = sbs_drag_app();
        let divider = app
            .state
            .view
            .sbs_divider_rect
            .expect("a split names its divider");

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            divider.x,
            divider.y + 2,
        ));
        assert!(
            app.state.shell_interaction.side_by_side_resize_active(),
            "a press on the divider arms the transaction"
        );

        app.handle_mouse(mouse(
            MouseEventKind::Drag(MouseButton::Left),
            divider.x + 24,
            divider.y + 2,
        ));
        app.handle_mouse(mouse(
            MouseEventKind::Up(MouseButton::Left),
            divider.x + 24,
            divider.y + 2,
        ));

        let ratio = app
            .state
            .side_by_side
            .expect("the split survives")
            .ratio_percent;
        assert!(
            ratio > 50,
            "dragging right grows the left half; ratio stayed {ratio}"
        );
        assert!(
            !app.state.shell_interaction.side_by_side_resize_active(),
            "Up releases the transaction"
        );

        // A drag after release owns nothing — the capture does not leak.
        app.handle_mouse(mouse(
            MouseEventKind::Drag(MouseButton::Left),
            divider.x + 40,
            divider.y + 2,
        ));
        assert_eq!(
            app.state.side_by_side.expect("still split").ratio_percent,
            ratio,
            "a released divider ignores stray drags"
        );
    }

    // TP-SBS-DRAG-01: the commit clamps to the same 20-80 band the state
    // promises everywhere else — a drag to the far edge cannot starve a half.
    #[test]
    fn an_sbs_drag_to_the_edge_clamps_the_ratio() {
        let mut app = sbs_drag_app();
        let divider = app
            .state
            .view
            .sbs_divider_rect
            .expect("a split names its divider");

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            divider.x,
            divider.y,
        ));
        app.handle_mouse(mouse(
            MouseEventKind::Drag(MouseButton::Left),
            119,
            divider.y,
        ));
        app.handle_mouse(mouse(MouseEventKind::Up(MouseButton::Left), 119, divider.y));

        let ratio = app
            .state
            .side_by_side
            .expect("the split survives")
            .ratio_percent;
        assert!(
            (20..=80).contains(&ratio) && ratio >= 70,
            "an edge drag lands at the clamp band's edge; got {ratio}"
        );
    }

    // TP-SBS-DRAG-01: a press anywhere else arms nothing — the drag must not
    // eat ordinary clicks.
    #[test]
    fn a_press_off_the_divider_arms_no_sbs_drag() {
        let mut app = sbs_drag_app();
        let divider = app
            .state
            .view
            .sbs_divider_rect
            .expect("a split names its divider");

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            divider.x + 10,
            divider.y + 2,
        ));
        assert!(
            !app.state.shell_interaction.side_by_side_resize_active(),
            "a press off the divider must not arm the drag"
        );
    }

    #[test]
    fn an_agent_row_click_dismisses_the_files_stage() {
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![crate::workspace::Workspace::test_new("one")];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = crate::app::state::Mode::Terminal;
        app.state.mobile_width_threshold = 0;
        app.state.ensure_test_terminals();
        let root = app.state.workspaces[0].tabs[0].root_pane;
        let terminal_id = app.state.workspaces[0].tabs[0]
            .panes
            .get(&root)
            .map(|pane| pane.attached_terminal_id.clone())
            .expect("root pane has a terminal");
        if let Some(terminal) = app.state.terminals.get_mut(&terminal_id) {
            terminal.set_detected_state(
                Some(crate::detect::Agent::Pi),
                crate::detect::AgentState::Idle,
            );
        }

        let file_manager =
            crate::fm::FmState::new(std::env::current_dir().expect("current directory"));
        app.state
            .try_open_file_manager_with(|_| Some(file_manager))
            .expect("Files activation");
        assert!(app.state.file_manager.is_some(), "fixture: Files is open");

        let area = ratatui::layout::Rect::new(0, 0, 106, 20);
        crate::ui::compute_view(&mut app.state, area);
        let panel = app.state.agent_panel_rect();
        let row = (panel.y..panel.y + panel.height)
            .find(|row| app.state.agent_detail_target_at(*row) == Some((0, 0, root)))
            .expect("the agent row is laid out");

        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 2, row));

        assert!(
            app.state.file_manager.is_none(),
            "the click on the (already focused) agent dismisses Files"
        );
        assert!(
            app.state.stage.active_surface()
                != Some(crate::ui::surface_host::AppSurfaceRef::NativeFiles),
            "the stage returned to the terminal surface"
        );
    }

    // TP-DAILY-12 / TP-DOTS-04: the area's header is managed like every other
    // container header — its "⋯" and a right-click on it open the SAME menu.
    // A header with no menu at all was the one row on this sidebar a person
    // could not act on, which is what made it read as a caption rather than
    // as a place.
    #[test]
    fn the_daily_header_answers_the_dots_and_the_right_click_with_one_menu() {
        let mut app = app_for_mouse_test();
        let daily = std::path::PathBuf::from("/home/tester");
        let mut elsewhere = crate::workspace::Workspace::test_new("elsewhere");
        elsewhere.identity_cwd = std::path::PathBuf::from("/repo/checkout");
        app.state.workspaces = vec![elsewhere];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mobile_width_threshold = 0;
        app.state.daily_chat_cwd = Some(daily.clone());
        let key = crate::persist::workspace_chats::ledger_key(&daily);
        app.state.workspace_chat_rows.insert(
            key,
            vec![crate::app::state::WorkspaceChatRow {
                session_id: "daily-0".to_string(),
                agent: "claude".to_string(),
                title: Some("daily chat".to_string()),
                last_seen_ms: 10,
                last_modified: None,
                last_message_at: None,
            }],
        );
        app.state.mouse_capture = true;

        let area = ratatui::layout::Rect::new(0, 0, 106, 20);
        crate::ui::compute_view(&mut app.state, area);
        let header = app
            .state
            .view
            .daily_header_area
            .expect("the area draws its header");

        // Right-click anywhere on the row.
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Right),
            header.x + 2,
            header.y,
        ));
        assert!(
            matches!(
                app.state.context_menu.as_ref().map(|menu| &menu.kind),
                Some(crate::app::state::ContextMenuKind::DailyHeader {
                    collapsed: false,
                    ..
                })
            ),
            "the right-click opens the area menu; got {:?}",
            app.state.context_menu.as_ref().map(|menu| &menu.kind)
        );

        // The "⋯" opens the very same one, and does not fold on the way.
        app.state.context_menu = None;
        app.state.mode = Mode::Terminal;
        crate::ui::compute_view(&mut app.state, area);
        let dots = crate::ui::header_menu_cell(header);
        assert!(dots.width > 0, "a header this wide reserves a '⋯' cell");
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            dots.x,
            dots.y,
        ));
        assert!(
            matches!(
                app.state.context_menu.as_ref().map(|menu| &menu.kind),
                Some(crate::app::state::ContextMenuKind::DailyHeader { .. })
            ),
            "the dots open the same menu; got {:?}",
            app.state.context_menu.as_ref().map(|menu| &menu.kind)
        );
        assert!(
            !app.state.daily_section_collapsed,
            "opening the menu never folds the area"
        );
    }

    // TP-MOD-31: the tree's empty space answers a right-click with the one
    // verb that needs no row — "New module...". It is matched last, so a
    // right-click that lands on an actual row still gets that row's menu; a
    // blank matched first would shadow every context menu this sidebar has.
    #[test]
    fn a_right_press_on_the_trees_blank_offers_to_make_a_container() {
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![crate::workspace::Workspace::test_new("only")];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mobile_width_threshold = 0;

        let area = ratatui::layout::Rect::new(0, 0, 106, 20);
        crate::ui::compute_view(&mut app.state, area);
        let card = *app
            .state
            .view
            .workspace_card_areas
            .last()
            .expect("the one workspace is laid out");
        let footer_top = app.state.sidebar_footer_rect().y;
        let blank_row = card.rect.y + card.rect.height + 1;
        assert!(
            blank_row < footer_top,
            "the fixture must leave a blank row between the tree and the footer"
        );
        assert!(
            app.state.workspace_at_row(blank_row).is_none(),
            "the chosen row is genuinely empty"
        );

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Right),
            2,
            blank_row,
        ));
        assert!(
            matches!(
                app.state.context_menu.as_ref().map(|menu| &menu.kind),
                Some(crate::app::state::ContextMenuKind::SidebarBlank)
            ),
            "the blank offers the container menu; got {:?}",
            app.state.context_menu.as_ref().map(|menu| &menu.kind)
        );

        // A right-click that lands on a row still belongs to that row.
        app.state.context_menu = None;
        app.state.mode = Mode::Terminal;
        crate::ui::compute_view(&mut app.state, area);
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Right),
            card.rect.x + 1,
            card.rect.y,
        ));
        assert!(
            !matches!(
                app.state.context_menu.as_ref().map(|menu| &menu.kind),
                Some(crate::app::state::ContextMenuKind::SidebarBlank)
            ),
            "a row keeps its own menu; got {:?}",
            app.state.context_menu.as_ref().map(|menu| &menu.kind)
        );
    }

    // TP-DAILY-10: the daily header carries two gestures on one row, and the
    // order between them is the whole behavior. The "+" is matched first, so
    // a press there opens the agent menu and leaves the fold alone; every
    // other column on the row still folds. Without mouse capture the plus is
    // not drawn, so that column folds like the rest — chrome that is invisible
    // must not be clickable, or the section becomes unfoldable by accident.
    #[test]
    fn a_left_press_on_the_daily_plus_opens_the_menu_instead_of_folding() {
        let mut app = app_for_mouse_test();
        let daily = std::path::PathBuf::from("/home/tester");
        let mut elsewhere = crate::workspace::Workspace::test_new("elsewhere");
        elsewhere.identity_cwd = std::path::PathBuf::from("/repo/checkout");
        app.state.workspaces = vec![elsewhere];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mobile_width_threshold = 0;
        app.state.daily_chat_cwd = Some(daily.clone());
        let key = crate::persist::workspace_chats::ledger_key(&daily);
        app.state.workspace_chat_rows.insert(
            key,
            (0..3)
                .map(|idx| crate::app::state::WorkspaceChatRow {
                    session_id: format!("daily-{idx}"),
                    agent: "claude".to_string(),
                    title: Some(format!("daily chat {idx}")),
                    last_seen_ms: 10 + idx as u64,
                    last_modified: None,
                    last_message_at: None,
                })
                .collect(),
        );
        app.state.mouse_capture = true;

        let area = ratatui::layout::Rect::new(0, 0, 106, 20);
        crate::ui::compute_view(&mut app.state, area);
        let header = app
            .state
            .view
            .daily_header_area
            .expect("the section draws its header");
        let plus = crate::ui::daily_new_chat_cell(header);
        assert!(plus.width > 0, "a header this wide reserves a '+' cell");

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            plus.x,
            plus.y,
        ));
        assert!(
            matches!(
                app.state.context_menu.as_ref().map(|menu| &menu.kind),
                Some(crate::app::state::ContextMenuKind::DailyNewChat)
            ),
            "the plus opens the agent menu; got {:?}",
            app.state.context_menu.as_ref().map(|menu| &menu.kind)
        );
        assert!(
            !app.state.daily_section_collapsed,
            "the plus never folds the section"
        );

        // Any other column on the row is still the fold.
        app.state.context_menu = None;
        app.state.mode = Mode::Terminal;
        crate::ui::compute_view(&mut app.state, area);
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            header.x + 1,
            header.y,
        ));
        assert!(
            app.state.daily_section_collapsed,
            "the rest of the header still folds"
        );
        assert!(app.state.context_menu.is_none(), "the fold opens no menu");

        // Mouse chrome: with the panel unowned the same column folds back.
        app.state.mouse_capture = false;
        crate::ui::compute_view(&mut app.state, area);
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            plus.x,
            plus.y,
        ));
        assert!(
            !app.state.daily_section_collapsed,
            "without capture the plus column is just header, and unfolds"
        );
        assert!(
            app.state.context_menu.is_none(),
            "a plus that is not drawn opens nothing"
        );
    }

    // TP-DOTS-17: the header's "+" is a second door to the module's
    // "New branch..." — the press walks the same body the menu item walks
    // (source workspace resolved, module armed, worktree dialog requested),
    // never folds, and stays mouse chrome: without capture the press on
    // that column folds like the rest of the row.
    #[test]
    fn a_left_press_on_the_header_plus_starts_the_branch_road() {
        let mut app = app_for_mouse_test();
        let mut main = crate::workspace::Workspace::test_new("main");
        main.worktree_space = Some(crate::workspace::WorktreeSpaceMembership {
            key: "repo-key".into(),
            label: "herdr".into(),
            repo_root: std::path::PathBuf::from("/repo/herdr"),
            checkout_path: std::path::PathBuf::from("/repo/herdr"),
            is_linked_worktree: false,
        });
        main.identity_cwd = std::env::temp_dir();
        // A second member so both header rows are born (a single-member
        // bucket folds into its row — recorded behavior, TP-NODE-05).
        let mut child = crate::workspace::Workspace::test_new("issue");
        child.worktree_space = Some(crate::workspace::WorktreeSpaceMembership {
            key: "repo-key".into(),
            label: "herdr".into(),
            repo_root: std::path::PathBuf::from("/repo/herdr"),
            checkout_path: std::path::PathBuf::from("/repo/herdr-issue"),
            is_linked_worktree: true,
        });
        app.state.workspaces = vec![main, child];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mobile_width_threshold = 0;
        app.state.space_projects = vec![crate::spaces::SpaceProject {
            key: "project:herdr".into(),
            name: "herdr".into(),
            icon: None,
            repo_roots: vec![std::path::PathBuf::from("/repo/herdr")],
            space_keys: Vec::new(),
        }];
        app.state.mouse_capture = true;

        let area = ratatui::layout::Rect::new(0, 0, 106, 20);
        crate::ui::compute_view(&mut app.state, area);
        let project_head = app.state.view.workspace_project_header_areas[0].clone();

        // The node header's "+" arms the module and requests the dialog.
        let head_plus = crate::ui::header_new_branch_cell(project_head.rect);
        assert!(head_plus.width > 0, "the node header reserves a '+' cell");
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            head_plus.x,
            head_plus.y,
        ));
        assert_eq!(
            app.state.pending_branch_module,
            Some("project:herdr".to_string()),
            "the plus arms the module exactly like the menu item"
        );
        assert_eq!(
            app.state.request_new_linked_worktree,
            Some(0),
            "the plus resolves the same source workspace the menu resolves"
        );
        assert!(
            !app.state.node_folded("project:herdr"),
            "the plus never folds the node"
        );

        // The bucket header's "+" walks the same road for its own key.
        app.state.pending_branch_module = None;
        app.state.request_new_linked_worktree = None;
        crate::ui::compute_view(&mut app.state, area);
        let group_head = app.state.view.workspace_group_header_areas[0].clone();
        let group_plus = crate::ui::header_new_branch_cell(group_head.rect);
        assert!(
            group_plus.width > 0,
            "the bucket header reserves a '+' cell"
        );
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            group_plus.x,
            group_plus.y,
        ));
        assert_eq!(
            app.state.pending_branch_module,
            Some("repo-key".to_string()),
            "the bucket plus arms the bucket as the module"
        );
        assert_eq!(app.state.request_new_linked_worktree, Some(0));
        assert!(
            !app.state.collapsed_space_keys.contains("repo-key"),
            "the plus never folds the group"
        );

        // Without the mouse the chrome is gone and the column folds again.
        app.state.pending_branch_module = None;
        app.state.request_new_linked_worktree = None;
        app.state.mouse_capture = false;
        crate::ui::compute_view(&mut app.state, area);
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            group_plus.x,
            group_plus.y,
        ));
        assert!(
            app.state.collapsed_space_keys.contains("repo-key"),
            "without capture the '+' column is plain header and folds"
        );
        assert_eq!(
            app.state.pending_branch_module, None,
            "without capture nothing arms"
        );
    }

    // TP-TREE-14 + TP-TREE-15: the two disclosures the Spaces tab owns now sit
    // on different rows, and each press does exactly one thing. Pressing the
    // repository folds its group without switching workspace; pressing a
    // checkout's own arrow opens that checkout's drawer without switching to
    // it. While both arrows shared one row, one press could plausibly mean
    // either — which is the complaint that produced this tree.
    #[test]
    fn a_repository_press_folds_its_group_and_a_checkout_arrow_opens_its_drawer() {
        let mut app = app_for_mouse_test();
        let mut main = crate::workspace::Workspace::test_new("main");
        main.worktree_space = Some(crate::workspace::WorktreeSpaceMembership {
            key: "repo-key".into(),
            label: "herdr".into(),
            repo_root: std::path::PathBuf::from("/repo/herdr"),
            checkout_path: std::path::PathBuf::from("/repo/herdr"),
            is_linked_worktree: false,
        });
        main.identity_cwd = std::env::temp_dir();
        let mut child = crate::workspace::Workspace::test_new("issue");
        child.worktree_space = Some(crate::workspace::WorktreeSpaceMembership {
            key: "repo-key".into(),
            label: "herdr".into(),
            repo_root: std::path::PathBuf::from("/repo/herdr"),
            checkout_path: std::path::PathBuf::from("/repo/herdr-issue"),
            is_linked_worktree: true,
        });
        app.state.workspaces = vec![main, child];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mobile_width_threshold = 0;
        // TP-WSID-03 re-base: the drawer keys by the checkout the row means,
        // so the remembered history lives under it, not the birthplace.
        let key = crate::persist::workspace_chats::ledger_key(std::path::Path::new("/repo/herdr"));
        app.state.workspace_chat_rows.insert(
            key.clone(),
            vec![crate::app::state::WorkspaceChatRow {
                session_id: "remembered".into(),
                agent: "claude".into(),
                title: Some("remembered chat".into()),
                last_seen_ms: 1,
                last_modified: None,
                last_message_at: None,
            }],
        );

        let area = ratatui::layout::Rect::new(0, 0, 106, 20);
        crate::ui::compute_view(&mut app.state, area);
        let header = app.state.view.workspace_group_header_areas[0].clone();
        let active_before = app.state.active;

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            header.rect.x,
            header.rect.y,
        ));
        assert!(
            app.state.collapsed_space_keys.contains("repo-key"),
            "pressing the repository folds its group"
        );
        assert_eq!(
            app.state.active, active_before,
            "the repository row is not a workspace: pressing it must not switch"
        );

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            header.rect.x,
            header.rect.y,
        ));
        assert!(
            !app.state.collapsed_space_keys.contains("repo-key"),
            "pressing it again unfolds the group"
        );

        // TP-TREE-15: the checkout's own arrow, at the checkout's depth.
        crate::ui::compute_view(&mut app.state, area);
        let card = app.state.view.workspace_card_areas[0];
        let toggle = crate::ui::workspace_chat_toggle_cell(&app.state, card.rect, card.ws_idx);
        assert!(toggle.width > 0, "a checkout with history offers an arrow");
        assert!(
            toggle.x > card.rect.x,
            "TP-TREE-10: a checkout's arrow never sits in the repository's column"
        );
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            toggle.x,
            toggle.y,
        ));
        assert!(
            app.state.expanded_chat_workspaces.contains(&key),
            "the checkout's arrow opens the checkout's drawer"
        );
        assert!(
            !app.state.collapsed_space_keys.contains("repo-key"),
            "and leaves the group alone"
        );
    }

    // TP-WSCHAT-24: a chat row that cannot be clicked is decoration. Clicking
    // one asks for that session; a chat already wired to a live tab is FOCUSED
    // instead, because resuming it twice spawns a second process against one
    // transcript. The trailing "+" starts a fresh chat and must not be
    // swallowed by the row it sits on.
    #[test]
    fn clicking_a_drawer_row_asks_for_that_chat_and_plus_starts_a_new_one() {
        let mut app = app_for_mouse_test();
        let mut workspace = crate::workspace::Workspace::test_new("chat-click");
        workspace.identity_cwd = std::env::temp_dir();
        app.state.workspaces = vec![workspace];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mobile_width_threshold = 0;
        // No checkout on this workspace, so the birthplace IS the effective
        // directory (TP-WSID-05): the drawer keys by it unchanged.
        let key = crate::persist::workspace_chats::ledger_key(&std::env::temp_dir());
        app.state.workspace_chat_rows.insert(
            key.clone(),
            vec![crate::app::state::WorkspaceChatRow {
                session_id: "resume-me".into(),
                agent: "claude".into(),
                title: Some("remembered chat".into()),
                last_seen_ms: 1,
                last_modified: None,
                last_message_at: None,
            }],
        );
        app.state.expanded_chat_workspaces.insert(key);

        let area = ratatui::layout::Rect::new(0, 0, 106, 20);
        crate::ui::compute_view(&mut app.state, area);
        let row = app.state.view.workspace_chat_row_areas[0].clone();
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            row.rect.x + 5,
            row.rect.y,
        ));
        let request = app
            .state
            .request_project_chat_tab
            .take()
            .expect("clicking a drawer row must ask for that chat");
        assert_eq!(request.session_id.as_deref(), Some("resume-me"));
        assert_eq!(request.project_path, std::env::temp_dir());

        crate::ui::compute_view(&mut app.state, area);
        let card = app.state.view.workspace_card_areas[0];
        let plus = crate::ui::workspace_new_chat_cell(card.rect);
        assert!(plus.width > 0, "every workspace row offers a create button");
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            plus.x,
            plus.y,
        ));
        // TP-WSCHAT-25: a repository root can start two different things, so
        // the button asks instead of guessing.
        let menu = app
            .state
            .context_menu
            .as_ref()
            .expect("the plus on a repository root opens the choice menu");
        assert!(matches!(
            menu.kind,
            crate::app::state::ContextMenuKind::WorkspaceNewChat {
                ws_idx: 0,
                offers_worktree: true
            }
        ));
        let labels = menu.items();
        assert!(
            labels.contains(&"New worktree"),
            "the choice includes a worktree: {labels:?}"
        );
        assert!(
            labels.contains(&"claude"),
            "and the chat agents: {labels:?}"
        );
        assert!(
            app.state.request_project_chat_tab.is_none(),
            "opening the menu must not already commit to a chat"
        );

        // TP-WSCHAT-25: and the answer routes. Picking the worktree entry must
        // reach the worktree request, not fall through to the agent catch-all
        // and quietly persist "New worktree" as the default chat agent.
        let worktree_idx = app
            .state
            .context_menu
            .as_ref()
            .and_then(|menu| menu.items().iter().position(|l| *l == "New worktree"))
            .expect("the menu offers a worktree");
        let menu = app
            .state
            .context_menu
            .take()
            .expect("the menu is still open");
        crate::app::input::modal::apply_context_menu_action(
            &mut app.state,
            &mut app.terminal_runtimes,
            menu,
            worktree_idx,
        );
        assert_eq!(
            app.state.request_new_linked_worktree,
            Some(0),
            "choosing the worktree entry asks for a worktree"
        );
        assert_ne!(
            app.state.default_chat_agent, "New worktree",
            "a worktree choice must never be persisted as a chat agent"
        );
        assert!(app.state.request_project_chat_tab.is_none());
    }

    // TP-WSCHAT-19: the drawer toggle must open AND close, and it must not
    // steal the click that switches to a workspace. A toggle that only opens
    // leaves the list permanently expanded; one that swallows the row makes the
    // workspace unreachable by mouse.
    #[test]
    fn the_drawer_toggle_opens_and_closes_without_stealing_the_workspace_click() {
        let mut app = app_for_mouse_test();
        let mut workspace = crate::workspace::Workspace::test_new("drawer-click");
        workspace.identity_cwd = std::env::temp_dir();
        app.state.workspaces = vec![workspace];
        app.state.active = Some(0);
        app.state.selected = 0;
        let key = crate::persist::workspace_chats::ledger_key(&std::env::temp_dir());
        app.state.workspace_chat_rows.insert(
            key.clone(),
            vec![crate::app::state::WorkspaceChatRow {
                session_id: "click-probe".into(),
                agent: "claude".into(),
                title: Some("probe chat".into()),
                last_seen_ms: 1,
                last_modified: None,
                last_message_at: None,
            }],
        );

        let area = ratatui::layout::Rect::new(0, 0, 106, 20);
        crate::ui::compute_view(&mut app.state, area);
        let card = app.state.view.workspace_card_areas[0];
        let cell = crate::ui::workspace_chat_toggle_cell(&app.state, card.rect, 0);
        assert!(cell.width > 0, "the probe workspace offers a toggle");

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            cell.x,
            cell.y,
        ));
        assert!(
            app.state.expanded_chat_workspaces.contains(&key),
            "the first click opens the drawer"
        );

        crate::ui::compute_view(&mut app.state, area);
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            cell.x,
            cell.y,
        ));
        assert!(
            !app.state.expanded_chat_workspaces.contains(&key),
            "a second click closes it again"
        );

        // A press elsewhere on the row still starts the workspace press that
        // selects it, so the toggle has not taken the row over.
        crate::ui::compute_view(&mut app.state, area);
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            card.rect.x + card.rect.width / 2,
            card.rect.y,
        ));
        assert!(
            app.state
                .workspace_presses
                .contains_key(&crate::app::LOCAL_INPUT_SOURCE),
            "the rest of the row still selects the workspace"
        );
    }

    #[tokio::test]
    async fn captured_left_press_focuses_target_before_forwarding() {
        let mut app = app_for_mouse_test();
        let mut ws = Workspace::test_new("test");
        let source = ws.tabs[0].root_pane;
        let target = ws.test_split(Direction::Horizontal);
        ws.tabs[0].layout.focus_pane(source);
        app.state.workspaces = vec![ws];
        app.state.active = Some(0);
        app.state.selected = 0;
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));
        let info = app
            .state
            .pane_info_by_id(target)
            .expect("target pane info")
            .clone();
        let (runtime, mut input_rx) =
            crate::terminal::TerminalRuntime::test_with_channel_and_scrollback_bytes(
                info.inner_rect.width,
                info.inner_rect.height,
                0,
                b"\x1b[?1002h\x1b[?1006h",
                4,
            );
        app.state.insert_test_runtime(target, runtime);

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            info.inner_rect.x + 1,
            info.inner_rect.y + 1,
        ));

        assert_eq!(app.state.workspaces[0].focused_pane_id(), Some(target));
        assert_eq!(
            input_rx.try_recv().expect("forwarded captured left press"),
            Bytes::from_static(b"\x1b[<0;2;2M")
        );
    }

    #[tokio::test]
    async fn pane_mouse_only_forwards_moved_events_for_any_motion_apps() {
        let mut app = app_for_mouse_test();
        let mut ws = Workspace::test_new("test");
        let pane_id = ws.tabs[0].root_pane;
        let pane_infos = ws.tabs[0].layout.panes(Rect::new(26, 2, 80, 18));
        let info = pane_infos[0].clone();
        let (runtime, mut input_rx) =
            crate::terminal::TerminalRuntime::test_with_channel_and_scrollback_bytes(
                info.inner_rect.width,
                info.inner_rect.height,
                0,
                b"\x1b[?1003h\x1b[?1006h",
                4,
            );
        ws.insert_test_runtime(pane_id, runtime);

        app.state.workspaces = vec![ws];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;
        app.state.view.pane_infos = pane_infos;

        app.state.handle_pane_mouse_only(
            &app.terminal_runtimes,
            mouse(
                MouseEventKind::Moved,
                info.inner_rect.x + 2,
                info.inner_rect.y + 3,
            ),
        );

        assert_eq!(
            input_rx.try_recv().expect("forwarded mouse motion"),
            Bytes::from_static(b"\x1b[<35;3;4M")
        );
        assert!(input_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn pane_mouse_motion_uses_computed_inner_rect_offsets() {
        let mut app = app_for_mouse_test();
        let mut ws = Workspace::test_new("test");
        let pane_id = ws.tabs[0].root_pane;
        let (runtime, mut input_rx) =
            crate::terminal::TerminalRuntime::test_with_channel_and_scrollback_bytes(
                80,
                18,
                0,
                b"\x1b[?1003h\x1b[?1006h",
                4,
            );
        ws.insert_test_runtime(pane_id, runtime);

        app.state.workspaces = vec![ws];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));
        let info = app.state.view.pane_infos[0].clone();
        assert!(info.inner_rect.x > 0, "sidebar offset should be present");
        assert!(info.inner_rect.y > 0, "tab bar offset should be present");

        app.state.handle_pane_mouse_only(
            &app.terminal_runtimes,
            mouse(
                MouseEventKind::Moved,
                info.inner_rect.x + 2,
                info.inner_rect.y + 3,
            ),
        );

        assert_eq!(
            input_rx.try_recv().expect("forwarded mouse motion"),
            Bytes::from_static(b"\x1b[<35;3;4M")
        );
        assert!(input_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn ordinary_cell_mouse_converts_pixel_mode_to_cell_centre_pixels() {
        let mut app = app_for_mouse_test();
        let mut ws = Workspace::test_new("test");
        let pane_id = ws.tabs[0].root_pane;
        let (runtime, mut input_rx) =
            crate::terminal::TerminalRuntime::test_with_channel_and_scrollback_bytes(
                80,
                18,
                0,
                b"\x1b[?1003h\x1b[?1006h\x1b[?1016h",
                4,
            );
        ws.insert_test_runtime(pane_id, runtime);

        app.state.workspaces = vec![ws];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;
        app.state.host_cell_size = crate::kitty_graphics::HostCellSize {
            width_px: 10,
            height_px: 20,
        };
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));
        let info = app.state.view.pane_infos[0].clone();
        app.state
            .runtime_for_pane_in_workspace(&app.terminal_runtimes, 0, pane_id)
            .unwrap()
            .resize(info.inner_rect.height, info.inner_rect.width, 10, 20);
        assert!(info.inner_rect.x > 0, "sidebar offset should be present");
        assert!(info.inner_rect.y > 0, "tab bar offset should be present");

        app.handle_mouse(mouse(
            MouseEventKind::Moved,
            info.inner_rect.x + 2,
            info.inner_rect.y + 3,
        ));

        assert_eq!(
            input_rx.try_recv().expect("forwarded mouse motion"),
            Bytes::from_static(b"\x1b[<35;26;71M")
        );
        assert!(input_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn dedicated_client_pixel_mouse_preserves_subcell_position() {
        let mut app = app_for_mouse_test();
        let mut ws = Workspace::test_new("test");
        let pane_id = ws.tabs[0].root_pane;
        let (runtime, mut input_rx) =
            crate::terminal::TerminalRuntime::test_with_channel_and_scrollback_bytes(
                80,
                18,
                0,
                b"\x1b[?1003h\x1b[?1006h\x1b[?1016h",
                4,
            );
        ws.insert_test_runtime(pane_id, runtime);
        app.state.workspaces = vec![ws];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;
        app.state.mouse_capture = false;
        app.state.host_cell_size = crate::kitty_graphics::HostCellSize {
            width_px: 10,
            height_px: 20,
        };
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));
        let inner = app.state.view.pane_infos[0].inner_rect;
        let geometry = crate::input::mouse::HostGeometry::new(106, 20, 1_060, 400).unwrap();
        let x = u32::from(inner.x + 2) * 10 + 8;
        let y = u32::from(inner.y + 3) * 20 + 9;
        let report = format!("\x1b[<35;{x};{y}M");
        app.state.host_mouse_pixels = Some(crate::input::mouse::HostPixels { x, y, geometry });
        let runtime = app
            .state
            .runtime_for_pane_in_workspace(&app.terminal_runtimes, 0, pane_id)
            .unwrap();
        assert_eq!(runtime.pixel_size(), None);
        assert_eq!(
            app.state.pane_mouse_position(
                runtime,
                inner,
                mouse(MouseEventKind::Moved, inner.x + 2, inner.y + 3),
            ),
            Some(crate::input::mouse::Position::Cell { column: 2, row: 3 })
        );
        runtime.resize(inner.height, inner.width, 10, 20);
        let runtime = app
            .state
            .runtime_for_pane_in_workspace(&app.terminal_runtimes, 0, pane_id)
            .unwrap();
        assert_eq!(
            runtime.pixel_size(),
            Some((u32::from(inner.width) * 10, u32::from(inner.height) * 20))
        );
        assert_eq!(
            app.state.pane_mouse_position(
                runtime,
                inner,
                mouse(MouseEventKind::Moved, inner.x + 2, inner.y + 3),
            ),
            Some(crate::input::mouse::Position::Pixels { x: 28, y: 69 })
        );
        app.state.host_mouse_pixels = None;

        assert!(app.route_client_pixel_mouse(7, report.as_bytes(), geometry));
        assert_eq!(
            input_rx.try_recv().expect("forwarded exact mouse motion"),
            Bytes::from_static(b"\x1b[<35;28;69M")
        );
        assert!(input_rx.try_recv().is_err());
        assert!(app.state.host_mouse_pixels.is_none());
    }

    #[tokio::test]
    async fn mouse_dispatcher_does_not_forward_motion_behind_herdr_modes() {
        let mut app = app_for_mouse_test();
        let mut ws = Workspace::test_new("test");
        let pane_id = ws.tabs[0].root_pane;
        let (runtime, mut input_rx) =
            crate::terminal::TerminalRuntime::test_with_channel_and_scrollback_bytes(
                80,
                18,
                0,
                b"\x1b[?1003h\x1b[?1006h",
                4,
            );
        ws.insert_test_runtime(pane_id, runtime);

        app.state.workspaces = vec![ws];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Navigate;
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));
        let info = app.state.view.pane_infos[0].clone();

        app.handle_mouse(mouse(
            MouseEventKind::Moved,
            info.inner_rect.x + 2,
            info.inner_rect.y + 3,
        ));

        assert!(input_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn unset_right_click_passthrough_keeps_modified_right_click_as_herdr_menu() {
        let mut app = app_for_mouse_test();
        let mut ws = Workspace::test_new("test");
        let pane_id = ws.tabs[0].root_pane;
        let pane_infos = ws.tabs[0].layout.panes(Rect::new(26, 2, 80, 18));
        let info = pane_infos[0].clone();
        let (runtime, mut input_rx) =
            crate::terminal::TerminalRuntime::test_with_channel_and_scrollback_bytes(
                info.inner_rect.width,
                info.inner_rect.height,
                0,
                b"\x1b[?1002h\x1b[?1006h",
                4,
            );
        ws.insert_test_runtime(pane_id, runtime);

        app.state.workspaces = vec![ws];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;
        app.state.view.pane_infos = pane_infos;
        app.state.right_click_passthrough_modifiers = None;

        app.handle_mouse(MouseEvent {
            modifiers: KeyModifiers::CONTROL,
            ..mouse(
                MouseEventKind::Down(MouseButton::Right),
                info.inner_rect.x + 2,
                info.inner_rect.y + 3,
            )
        });

        assert_eq!(app.state.mode, Mode::ContextMenu);
        assert!(app.state.context_menu.is_some());
        assert!(app.state.right_click_passthrough.is_none());
        assert!(input_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn pane_right_click_keeps_focus_and_swap_menu_swaps_with_focused_pane() {
        let mut app = app_for_mouse_test();
        let mut ws = Workspace::test_new("test");
        let source = ws.tabs[0].root_pane;
        let target = ws.test_split(Direction::Horizontal);
        ws.tabs[0].layout.focus_pane(source);
        app.state.workspaces = vec![ws];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 100, 20));
        let target_info = app
            .state
            .view
            .pane_infos
            .iter()
            .find(|info| info.id == target)
            .expect("target pane info")
            .clone();
        let source_rect_before = app
            .state
            .view
            .pane_infos
            .iter()
            .find(|info| info.id == source)
            .expect("source pane info")
            .rect;
        let target_rect_before = target_info.rect;

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Right),
            target_info.inner_rect.x,
            target_info.inner_rect.y,
        ));

        assert_eq!(app.state.workspaces[0].focused_pane_id(), Some(source));
        let menu = app.state.context_menu.as_mut().expect("pane context menu");
        assert!(matches!(
            menu.kind,
            ContextMenuKind::Pane {
                pane_id,
                source_pane_id: Some(source_pane_id),
                ..
            } if pane_id == target && source_pane_id == source
        ));
        let swap_idx = menu
            .items()
            .iter()
            .position(|item| *item == "Swap with focused pane")
            .expect("swap item");
        menu.list.highlighted = swap_idx;

        handle_context_menu_key(
            &mut app.state,
            &mut app.terminal_runtimes,
            KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()),
        );
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 100, 20));

        assert_eq!(app.state.mode, Mode::Terminal);
        assert_eq!(app.state.workspaces[0].focused_pane_id(), Some(source));
        assert_eq!(
            app.state
                .view
                .pane_infos
                .iter()
                .find(|info| info.id == source)
                .unwrap()
                .rect,
            target_rect_before
        );
        assert_eq!(
            app.state
                .view
                .pane_infos
                .iter()
                .find(|info| info.id == target)
                .unwrap()
                .rect,
            source_rect_before
        );
    }

    #[tokio::test]
    async fn normal_right_click_keeps_focus_and_exposes_swap_for_reporting_pane() {
        let mut app = app_for_mouse_test();
        let mut ws = Workspace::test_new("test");
        let source = ws.tabs[0].root_pane;
        let target = ws.test_split(Direction::Horizontal);
        ws.tabs[0].layout.focus_pane(source);
        app.state.workspaces = vec![ws];
        app.state.active = Some(0);
        app.state.selected = 0;
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 100, 20));
        let target_info = app
            .state
            .pane_info_by_id(target)
            .expect("target pane info")
            .clone();
        let (runtime, mut input_rx) =
            crate::terminal::TerminalRuntime::test_with_channel_and_scrollback_bytes(
                target_info.inner_rect.width,
                target_info.inner_rect.height,
                0,
                b"\x1b[?1002h\x1b[?1006h",
                4,
            );
        app.state.insert_test_runtime(target, runtime);

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Right),
            target_info.inner_rect.x,
            target_info.inner_rect.y,
        ));

        assert!(input_rx.try_recv().is_err());
        assert_eq!(app.state.workspaces[0].focused_pane_id(), Some(source));
        let menu = app.state.context_menu.as_mut().expect("pane context menu");
        assert!(matches!(
            menu.kind,
            ContextMenuKind::Pane {
                pane_id,
                source_pane_id: Some(source_pane_id),
                ..
            } if pane_id == target && source_pane_id == source
        ));
        assert!(menu.items().contains(&"Swap with focused pane"));
    }

    #[tokio::test]
    async fn right_click_passthrough_requires_exact_modifier_match() {
        let mut app = app_for_mouse_test();
        let mut ws = Workspace::test_new("test");
        let pane_id = ws.tabs[0].root_pane;
        let pane_infos = ws.tabs[0].layout.panes(Rect::new(26, 2, 80, 18));
        let info = pane_infos[0].clone();
        let (runtime, mut input_rx) =
            crate::terminal::TerminalRuntime::test_with_channel_and_scrollback_bytes(
                info.inner_rect.width,
                info.inner_rect.height,
                0,
                b"\x1b[?1002h\x1b[?1006h",
                4,
            );
        ws.insert_test_runtime(pane_id, runtime);

        app.state.workspaces = vec![ws];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;
        app.state.view.pane_infos = pane_infos;

        app.state.right_click_passthrough_modifiers = Some(KeyModifiers::CONTROL);

        let col = info.inner_rect.x + 2;
        let row = info.inner_rect.y + 3;
        app.handle_mouse(MouseEvent {
            modifiers: KeyModifiers::CONTROL | KeyModifiers::SHIFT,
            ..mouse(MouseEventKind::Down(MouseButton::Right), col, row)
        });

        assert_eq!(app.state.mode, Mode::ContextMenu);
        assert!(app.state.context_menu.is_some());
        assert!(app.state.right_click_passthrough.is_none());
        assert!(input_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn right_click_passthrough_does_not_forward_pane_frame_clicks() {
        let mut app = app_for_mouse_test();
        let mut ws = Workspace::test_new("test");
        let pane_id = ws.tabs[0].root_pane;
        let other_pane = ws.test_split(Direction::Vertical);
        app.state.workspaces = vec![ws];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;
        app.state.right_click_passthrough_modifiers = Some(KeyModifiers::CONTROL);
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));

        let info = app
            .state
            .view
            .pane_infos
            .iter()
            .find(|info| info.id == pane_id)
            .expect("pane info")
            .clone();
        let (runtime, mut input_rx) =
            crate::terminal::TerminalRuntime::test_with_channel_and_scrollback_bytes(
                info.inner_rect.width,
                info.inner_rect.height,
                0,
                b"\x1b[?1002h\x1b[?1006h",
                4,
            );
        app.state.insert_test_runtime(pane_id, runtime);
        app.state.insert_test_runtime(
            other_pane,
            crate::terminal::TerminalRuntime::test_with_screen_bytes(10, 5, b""),
        );

        assert!(app.state.pane_at(info.rect.x, info.rect.y).is_none());
        assert!(app
            .state
            .pane_mouse_target(info.rect.x, info.rect.y)
            .is_some());
        app.handle_mouse(MouseEvent {
            modifiers: KeyModifiers::CONTROL,
            ..mouse(
                MouseEventKind::Down(MouseButton::Right),
                info.rect.x,
                info.rect.y,
            )
        });

        assert_eq!(app.state.mode, Mode::ContextMenu);
        assert!(app.state.context_menu.is_some());
        assert!(app.state.right_click_passthrough.is_none());
        assert!(input_rx.try_recv().is_err());
    }

    fn sample_worktree_open_state() -> crate::app::state::WorktreeOpenState {
        crate::app::state::WorktreeOpenState {
            source_workspace_id: "source".into(),
            source_existing_membership: None,
            source_checkout_path: "/repo/herdr".into(),
            source_repo_root: "/repo/herdr".into(),
            repo_key: "repo-key".into(),
            repo_name: "herdr".into(),
            entries: vec![
                crate::app::state::WorktreeOpenEntry {
                    path: "/repo/herdr".into(),
                    branch: Some("main".into()),
                    is_linked_worktree: false,
                    already_open_ws_idx: Some(0),
                },
                crate::app::state::WorktreeOpenEntry {
                    path: "/repo/herdr-issue".into(),
                    branch: Some("worktree/issue".into()),
                    is_linked_worktree: true,
                    already_open_ws_idx: None,
                },
            ],
            selected: 0,
            query: String::new(),
            search_focused: false,
            error: None,
        }
    }

    #[test]
    fn hovering_context_menu_updates_highlight() {
        let mut app = app_for_mouse_test();
        app.state.context_menu = Some(ContextMenuState {
            kind: ContextMenuKind::Workspace { ws_idx: 0 },
            x: 2,
            y: 2,
            list: MenuListState::new(0),
        });
        app.state.mode = Mode::ContextMenu;

        let menu = app.state.context_menu_rect().unwrap();
        app.handle_mouse(mouse(MouseEventKind::Moved, menu.x + 2, menu.y + 2));

        assert_eq!(app.state.context_menu.unwrap().list.highlighted, 1);
    }

    // Fix #2 — mouse-move render change-gate. The serial render loop repaints
    // whenever `route_client_events` reports a change, so an inert hover move
    // must report `false`; otherwise rapid pointer motion saturates the loop
    // with full virtual frames (the freeze felt while clicking fast between
    // Miller columns). These characterization tests pin the gate: inert moves
    // decline a render, every other input still repaints.

    #[test]
    fn inert_mouse_move_declines_render() {
        // TP-REPAINT-2B: a hover move with no blocking overlay (plain terminal / native
        // file-manager surface) changes nothing herdr draws, so the router must
        // not request a render for it.
        let mut app = app_for_mouse_test();
        assert!(
            !app.state.blocking_overlay_active(),
            "fixture must start with no blocking overlay"
        );
        let rendered = app.route_client_events(
            vec![crate::raw_input::RawInputEvent::Mouse(mouse(
                MouseEventKind::Moved,
                40,
                5,
            ))],
            true,
        );
        assert!(
            !rendered,
            "an inert hover move must not request a render (loop-saturation guard)"
        );
    }

    #[test]
    fn mouse_move_over_blocking_overlay_requests_render() {
        // TP-REPAINT-2D: while a hover-sensitive overlay owns the pointer a move can
        // change its highlight, so the router must keep requesting a render.
        let mut app = app_for_mouse_test();
        app.state.context_menu = Some(ContextMenuState {
            kind: ContextMenuKind::Workspace { ws_idx: 0 },
            x: 2,
            y: 2,
            list: MenuListState::new(0),
        });
        app.state.mode = Mode::ContextMenu;
        assert!(
            app.state.blocking_overlay_active(),
            "fixture must have a blocking overlay up"
        );
        let menu = app.state.context_menu_rect().unwrap();
        let rendered = app.route_client_events(
            vec![crate::raw_input::RawInputEvent::Mouse(mouse(
                MouseEventKind::Moved,
                menu.x + 2,
                menu.y + 2,
            ))],
            true,
        );
        assert!(
            rendered,
            "a move over a blocking overlay must still request a render"
        );
    }

    #[test]
    fn non_move_mouse_events_always_request_render() {
        // TP-REPAINT-2C / TP-REPAINT-2F: generic press, release, drag, and wheel input repaint.
        // Native-FM vertical wheel duplicates have their own exact typed
        // override; this fixture deliberately has no live Files row target.
        for kind in [
            MouseEventKind::Down(MouseButton::Left),
            MouseEventKind::Up(MouseButton::Left),
            MouseEventKind::Drag(MouseButton::Left),
            MouseEventKind::ScrollUp,
            MouseEventKind::ScrollDown,
        ] {
            let mut app = app_for_mouse_test();
            let rendered = app.route_client_events(
                vec![crate::raw_input::RawInputEvent::Mouse(mouse(kind, 40, 5))],
                true,
            );
            assert!(rendered, "mouse {kind:?} must request a render");
        }
    }

    #[test]
    fn keyboard_and_paste_always_request_render() {
        // TP-REPAINT-2E: non-mouse interaction (keys, paste) is low-frequency and always
        // repaints, unchanged by the move gate.
        let mut app = app_for_mouse_test();
        let key_rendered =
            app.route_client_events(crate::raw_input::parse_raw_input_bytes_sync(b"a"), true);
        assert!(key_rendered, "a key press must request a render");

        let paste_rendered = app.route_client_events(
            vec![crate::raw_input::RawInputEvent::Paste("x".to_string())],
            true,
        );
        assert!(paste_rendered, "a paste must request a render");
    }

    // SF4.2-02: a topmost blocking overlay owns every mouse event. Background
    // routes (sidebar wheel selection, divider double-click gestures) must not
    // act while the overlay is open, and the outside click that closes the
    // overlay must not prime a background gesture for the next click.
    #[test]
    fn overlay_blocks_every_background_mouse_action() {
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![
            crate::workspace::Workspace::test_new("one"),
            crate::workspace::Workspace::test_new("two"),
            crate::workspace::Workspace::test_new("three"),
        ];
        app.state.active = Some(1);
        app.state.selected = 1;
        app.state.sidebar_width = 32;
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 40));
        let divider_col = app.state.view.sidebar_rect.x + app.state.view.sidebar_rect.width - 1;
        assert!(
            app.state.on_sidebar_divider(divider_col, 10),
            "fixture must target the live divider column"
        );

        // Control: without an overlay the sidebar wheel moves the selection.
        app.handle_mouse(mouse(MouseEventKind::ScrollUp, 5, 3));
        assert_eq!(
            app.state.selected, 0,
            "control: sidebar wheel must reach the workspace list without an overlay"
        );
        app.state.selected = 1;

        app.state.context_menu = Some(ContextMenuState {
            kind: ContextMenuKind::Workspace { ws_idx: 0 },
            x: 60,
            y: 20,
            list: MenuListState::new(0),
        });
        app.state.mode = Mode::ContextMenu;

        // The open menu owns a background sidebar wheel fail-closed.
        app.handle_mouse(mouse(MouseEventKind::ScrollUp, 5, 3));
        assert_eq!(
            (
                app.state.selected,
                app.state.mode,
                app.state.context_menu.is_some(),
            ),
            (1, Mode::ContextMenu, true),
            "an open context menu must consume a background sidebar wheel"
        );

        // The outside click on the divider closes the menu (its contract) but
        // must not prime a divider double-click gesture.
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            divider_col,
            10,
        ));
        assert_ne!(app.state.mode, Mode::ContextMenu);
        assert!(app.state.context_menu.is_none());
        assert_eq!(app.state.sidebar_width, 32);

        // The immediate next divider click is a first click, never the second
        // half of a double-click primed while the overlay owned input.
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            divider_col,
            10,
        ));
        assert_eq!(
            app.state.sidebar_width, 32,
            "the overlay-consumed click must not pair into a divider double-click reset"
        );

        // Control: a fresh non-overlay double-click still resets the width.
        app.last_sidebar_divider_click = None;
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            divider_col,
            10,
        ));
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            divider_col,
            10,
        ));
        assert_eq!(
            app.state.sidebar_width, app.state.default_sidebar_width,
            "control: the divider double-click reset must survive overlay blocking"
        );
    }

    // TP-C3.3-PLUGIN-SURFACE: dynamic plugin titles size the shared popup by
    // terminal display cells, not UTF-8 byte length.
    #[test]
    fn plugin_file_context_menu_uses_display_width_for_unicode_title() {
        let mut app = app_for_mouse_test();
        app.state.view.sidebar_rect = Rect::new(0, 0, 1, 20);
        app.state.view.terminal_area = Rect::new(1, 0, 79, 20);
        let label = "界".repeat(8);
        app.state.context_menu = Some(ContextMenuState {
            kind: ContextMenuKind::File {
                model: crate::app::state::FileManagerContextMenuModel {
                    target_kind: crate::app::state::FileManagerContextMenuTargetKind::File,
                    paths: vec![std::path::PathBuf::from("/prepared/file.txt")],
                    items: vec![crate::app::state::FileManagerContextMenuItem {
                        action: crate::app::state::FileManagerContextMenuAction::Plugin {
                            plugin_id: "example.files".into(),
                            action_id: "inspect".into(),
                        },
                        label,
                        enabled: true,
                        disabled_reason: None,
                    }],
                },
            },
            x: 2,
            y: 2,
            list: MenuListState::new(0),
        });

        assert_eq!(app.state.context_menu_rect().expect("menu rect").width, 20);
    }

    #[test]
    fn clicking_agent_toast_focuses_target_pane() {
        let mut app = app_for_mouse_test();
        let active = Workspace::test_new("active");
        let mut background = Workspace::test_new("background");
        let first_pane = background.tabs[0].root_pane;
        let target_pane = background.test_split(Direction::Horizontal);
        background.tabs[0].layout.focus_pane(first_pane);

        app.state.workspaces = vec![active, background];
        app.state.ensure_test_terminals();
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.toast_config.delivery = crate::config::ToastDelivery::Herdr;
        app.state.toast_config.delay_seconds = 0;
        let target_terminal_id = app.state.workspaces[1]
            .panes
            .get(&target_pane)
            .unwrap()
            .attached_terminal_id
            .clone();
        app.state
            .terminals
            .get_mut(&target_terminal_id)
            .unwrap()
            .state = AgentState::Working;

        app.state
            .handle_app_event(crate::events::AppEvent::StateChanged {
                pane_id: target_pane,
                agent: Some(Agent::Pi),
                state: AgentState::Idle,
                visible_blocker: false,
                visible_working: false,
                process_exited: false,
                observed_at: std::time::Instant::now(),
            });
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));

        let hit = app.state.view.toast_hit_area;
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            hit.x + 1,
            hit.y + 1,
        ));

        assert_eq!(app.state.active, Some(1));
        assert_eq!(app.state.workspaces[1].focused_pane_id(), Some(target_pane));
        assert!(app.state.toast.is_none());
        assert_eq!(app.state.mode, Mode::Terminal);

        app.state.last_pane();

        assert_eq!(app.state.active, Some(0));
        assert_eq!(
            app.state.workspaces[0].focused_pane_id(),
            Some(app.state.workspaces[0].tabs[0].root_pane)
        );
    }

    #[test]
    fn toast_click_does_not_steal_mouse_from_settings_overlay() {
        let mut app = app_for_mouse_test();
        let active = Workspace::test_new("active");
        let background = Workspace::test_new("background");
        let target_pane = background.tabs[0].root_pane;
        let workspace_id = background.id.clone();

        app.state.workspaces = vec![active, background];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.toast = Some(crate::app::state::ToastNotification {
            kind: crate::app::state::ToastKind::Finished,
            title: "pi finished".into(),
            context: "background · 2".into(),
            position: None,
            target: Some(crate::app::state::ToastTarget {
                workspace_id,
                pane_id: target_pane,
            }),
        });
        app.state.mode = Mode::Settings;
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));

        let hit = app.state.view.toast_hit_area;
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            hit.x + 1,
            hit.y + 1,
        ));

        assert_eq!(app.state.active, Some(0));
        assert!(app.state.toast.is_some());
    }

    #[test]
    fn clicking_confirm_close_accepts_workspace_close() {
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![Workspace::test_new("a"), Workspace::test_new("b")];
        app.state.active = Some(0);
        app.state.selected = 1;
        app.state.mode = Mode::ConfirmClose;

        let popup = app.state.confirm_close_rect();
        let inner = Rect::new(
            popup.x + 1,
            popup.y + 1,
            popup.width.saturating_sub(2),
            popup.height.saturating_sub(2),
        );
        let (confirm, _) = crate::ui::confirm_close_button_rects(inner);

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            confirm.x,
            confirm.y,
        ));

        assert_eq!(app.state.workspaces.len(), 1);
        assert_eq!(app.state.mode, Mode::Terminal);
    }

    #[test]
    fn clicking_rename_save_submits_workspace_rename_through_api_path() {
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![Workspace::test_new("old")];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::RenameWorkspace;
        app.state.name_input = "new".into();

        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 24));
        let inner = app.state.rename_modal_inner().unwrap();
        let (save, _, _) = crate::ui::rename_button_rects(inner);

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            save.x,
            save.y,
        ));

        assert_eq!(app.state.workspaces[0].custom_name.as_deref(), Some("new"));
        assert!(app.event_hub.events_after(0).iter().any(|(_, event)| {
            matches!(event.event, crate::api::schema::EventKind::WorkspaceRenamed)
        }));
    }

    #[test]
    fn clicking_open_worktree_row_selects_and_requests_open() {
        let mut app = app_for_mouse_test();
        app.state.mode = Mode::OpenExistingWorktree;
        app.state.worktree_open = Some(sample_worktree_open_state());
        let inner =
            crate::ui::open_existing_worktree_inner_rect(app.state.screen_rect(), 2).unwrap();

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            inner.x + 1,
            inner.y + 5,
        ));

        assert_eq!(app.state.worktree_open.as_ref().unwrap().selected, 1);
        assert!(app.state.request_submit_worktree_open);
    }

    #[test]
    fn clicking_open_worktree_buttons_requests_open_or_cancels() {
        let mut app = app_for_mouse_test();
        app.state.mode = Mode::OpenExistingWorktree;
        app.state.worktree_open = Some(sample_worktree_open_state());
        let inner =
            crate::ui::open_existing_worktree_inner_rect(app.state.screen_rect(), 2).unwrap();
        let (open, _) = crate::ui::open_existing_worktree_button_rects(inner);

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            open.x,
            open.y,
        ));

        assert!(app.state.worktree_open.is_some());
        assert!(app.state.request_submit_worktree_open);

        let mut app = app_for_mouse_test();
        app.state.mode = Mode::OpenExistingWorktree;
        app.state.worktree_open = Some(sample_worktree_open_state());
        let inner =
            crate::ui::open_existing_worktree_inner_rect(app.state.screen_rect(), 2).unwrap();
        let (_, cancel) = crate::ui::open_existing_worktree_button_rects(inner);

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            cancel.x,
            cancel.y,
        ));

        assert!(app.state.worktree_open.is_none());
        assert_eq!(app.state.mode, Mode::Navigate);
    }

    #[test]
    fn scrolling_open_worktree_picker_moves_selection() {
        let mut app = app_for_mouse_test();
        app.state.mode = Mode::OpenExistingWorktree;
        app.state.worktree_open = Some(sample_worktree_open_state());

        app.handle_mouse(mouse(MouseEventKind::ScrollDown, 1, 1));
        assert_eq!(app.state.worktree_open.as_ref().unwrap().selected, 1);

        app.handle_mouse(mouse(MouseEventKind::ScrollUp, 1, 1));
        assert_eq!(app.state.worktree_open.as_ref().unwrap().selected, 0);
    }

    // TP-MOD-43: the node heading's delete verb OPENS the confirmation —
    // the old road deleted on the spot with no question asked.
    #[test]
    fn node_delete_verb_opens_the_dialog_instead_of_deleting() {
        let mut app = app_for_mouse_test();
        app.state
            .open_module_delete(crate::app::state::ModuleDeleteTarget::Node {
                key: "node-x".to_string(),
            });
        assert_eq!(app.state.mode, Mode::ConfirmDeleteModule);
        assert!(app
            .state
            .module_delete
            .as_ref()
            .is_some_and(|confirm| matches!(
                &confirm.target,
                crate::app::state::ModuleDeleteTarget::Node { key } if key == "node-x"
            )));
    }

    // TP-MOD-43: every button of the module-delete dialog answers a click —
    // and only a button does.
    #[test]
    fn module_delete_dialog_buttons_delete_rename_and_cancel() {
        use crate::app::state::ModuleDeleteConfirmation;
        use crate::app::state::ModuleDeleteTarget;
        let confirm = || ModuleDeleteConfirmation {
            // A key nothing on this machine carries: the delete road reads
            // the managed overlay and, matching nothing, writes nothing.
            target: ModuleDeleteTarget::Split {
                key: "herdr-test-nonexistent-module-x7".to_string(),
            },
            label: "ghost".to_string(),
        };
        let rects = |app: &crate::app::App| {
            let popup =
                crate::ui::delete_module_popup_rect(app.state.screen_rect()).expect("popup");
            let inner = Rect::new(
                popup.x + 1,
                popup.y + 1,
                popup.width.saturating_sub(2),
                popup.height.saturating_sub(2),
            );
            crate::ui::delete_module_button_rects(inner)
        };

        // Cancel: the dialog closes and nothing else happens.
        let mut app = app_for_mouse_test();
        app.state.mode = Mode::ConfirmDeleteModule;
        app.state.module_delete = Some(confirm());
        let (_, _, cancel) = rects(&app);
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            cancel.x,
            cancel.y,
        ));
        assert!(app.state.module_delete.is_none());
        assert_ne!(app.state.mode, Mode::ConfirmDeleteModule);

        // Delete: the confirmation is consumed and the mode is left. (The
        // rule removal itself is pinned by the `_at` seam test in actions.)
        let mut app = app_for_mouse_test();
        app.state.mode = Mode::ConfirmDeleteModule;
        app.state.module_delete = Some(confirm());
        let (delete, _, _) = rects(&app);
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            delete.x,
            delete.y,
        ));
        assert!(app.state.module_delete.is_none());
        assert_ne!(app.state.mode, Mode::ConfirmDeleteModule);

        // Rename: hands over to the module name input instead.
        let mut app = app_for_mouse_test();
        app.state.mode = Mode::ConfirmDeleteModule;
        app.state.module_delete = Some(confirm());
        let (_, rename, _) = rects(&app);
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            rename.x,
            rename.y,
        ));
        assert!(app.state.module_delete.is_none());
        assert_eq!(app.state.mode, Mode::RenameWorkspace);
        assert!(app
            .state
            .pending_new_module
            .as_ref()
            .is_some_and(
                |pending| pending.rename_key.as_deref() == Some("herdr-test-nonexistent-module-x7")
            ));

        // A click outside every button decides nothing.
        let mut app = app_for_mouse_test();
        app.state.mode = Mode::ConfirmDeleteModule;
        app.state.module_delete = Some(confirm());
        let (delete, rename, cancel) = rects(&app);
        assert!(delete.right() <= rename.x && rename.right() <= cancel.x);
        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 0, 0));
        assert!(app.state.module_delete.is_some());
        assert_eq!(app.state.mode, Mode::ConfirmDeleteModule);
    }

    #[test]
    fn clicking_remove_worktree_buttons_requests_remove_or_cancels() {
        let mut app = app_for_mouse_test();
        app.state.mode = Mode::ConfirmRemoveWorktree;
        app.state.worktree_remove = Some(crate::app::state::WorktreeRemoveState {
            workspace_id: "issue".into(),
            repo_root: "/repo/herdr".into(),
            path: "/repo/herdr-issue".into(),
            error: None,
            removing: false,
            force_confirmation: false,
        });
        let popup = crate::ui::remove_worktree_popup_rect(app.state.screen_rect()).unwrap();
        let inner = Rect::new(
            popup.x + 1,
            popup.y + 1,
            popup.width.saturating_sub(2),
            popup.height.saturating_sub(2),
        );
        let (remove, _) = crate::ui::remove_worktree_button_rects(inner, false);

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            remove.x,
            remove.y,
        ));

        assert!(app.state.worktree_remove.is_some());
        assert!(app.state.request_submit_worktree_remove);

        let mut app = app_for_mouse_test();
        app.state.mode = Mode::ConfirmRemoveWorktree;
        app.state.worktree_remove = Some(crate::app::state::WorktreeRemoveState {
            workspace_id: "issue".into(),
            repo_root: "/repo/herdr".into(),
            path: "/repo/herdr-issue".into(),
            error: None,
            removing: false,
            force_confirmation: false,
        });
        let popup = crate::ui::remove_worktree_popup_rect(app.state.screen_rect()).unwrap();
        let inner = Rect::new(
            popup.x + 1,
            popup.y + 1,
            popup.width.saturating_sub(2),
            popup.height.saturating_sub(2),
        );
        let (_, cancel) = crate::ui::remove_worktree_button_rects(inner, false);

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            cancel.x,
            cancel.y,
        ));

        assert!(app.state.worktree_remove.is_none());
        assert_eq!(app.state.mode, Mode::Navigate);
    }

    #[test]
    fn clicking_confirm_close_accepts_after_workspace_context_menu_close() {
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![Workspace::test_new("a"), Workspace::test_new("b")];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;

        app.state.context_menu = Some(ContextMenuState {
            kind: ContextMenuKind::Workspace { ws_idx: 1 },
            x: 2,
            y: 2,
            list: MenuListState::new(1),
        });
        app.state.mode = Mode::ContextMenu;
        handle_context_menu_key(
            &mut app.state,
            &mut app.terminal_runtimes,
            KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()),
        );
        assert_eq!(app.state.mode, Mode::ConfirmClose);
        assert_eq!(app.state.selected, 1);

        let popup = app.state.confirm_close_rect();
        let inner = Rect::new(
            popup.x + 1,
            popup.y + 1,
            popup.width.saturating_sub(2),
            popup.height.saturating_sub(2),
        );
        let (confirm, _) = crate::ui::confirm_close_button_rects(inner);
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            confirm.x + 1,
            confirm.y,
        ));

        assert_eq!(app.state.workspaces.len(), 1);
        assert_eq!(app.state.workspaces[0].display_name(), "a");
    }

    #[test]
    fn clicking_context_menu_close_routes_through_api_path() {
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![Workspace::test_new("a"), Workspace::test_new("b")];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.confirm_close = false;
        app.state.context_menu = Some(ContextMenuState {
            kind: ContextMenuKind::Workspace { ws_idx: 1 },
            x: 2,
            y: 2,
            list: MenuListState::new(1),
        });
        app.state.mode = Mode::ContextMenu;

        let menu = app.state.context_menu_rect().unwrap();
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            menu.x + 2,
            menu.y + 2,
        ));

        assert_eq!(app.state.workspaces.len(), 1);
        assert_eq!(app.state.workspaces[0].display_name(), "a");
        assert!(app.event_hub.events_after(0).iter().any(|(_, event)| {
            matches!(event.event, crate::api::schema::EventKind::WorkspaceClosed)
        }));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn keyboard_context_menu_split_keeps_new_runtime() {
        let mut app = app_for_mouse_test();
        app.state.default_shell = "/usr/bin/true".into();
        let (workspace, terminal, runtime) = Workspace::new(
            std::env::current_dir().unwrap_or_else(|_| "/".into()),
            24,
            80,
            app.state.pane_scrollback_limit_bytes,
            app.state.host_terminal_theme,
            app.state.host_terminal_appearance,
            crate::pane::PaneShellConfig::new(&app.state.default_shell, app.state.shell_mode),
            app.event_tx.clone(),
            app.render_notify.clone(),
            app.render_dirty.clone(),
        )
        .expect("workspace should spawn");
        app.state.workspaces = vec![workspace];
        app.terminal_runtimes.insert(terminal.id.clone(), runtime);
        app.state.terminals.insert(terminal.id.clone(), terminal);
        app.state.active = Some(0);
        app.state.selected = 0;
        let pane_id = app.state.workspaces[0].tabs[0].root_pane;
        let runtime_count = app.terminal_runtimes.len();
        app.state.context_menu = Some(ContextMenuState {
            kind: ContextMenuKind::Pane {
                ws_idx: 0,
                tab_idx: 0,
                pane_id,
                source_pane_id: None,
                has_manual_label: false,
                right_click_passthrough: false,
            },
            x: 2,
            y: 2,
            list: MenuListState::new(1),
        });
        app.state.mode = Mode::ContextMenu;

        handle_context_menu_key(
            &mut app.state,
            &mut app.terminal_runtimes,
            KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()),
        );

        assert_eq!(app.state.mode, Mode::Terminal);
        assert_eq!(app.state.workspaces[0].tabs[0].layout.pane_count(), 2);
        assert_eq!(app.terminal_runtimes.len(), runtime_count + 1);

        let runtimes: Vec<_> = app.terminal_runtimes.drain().collect();
        for (_terminal_id, runtime) in runtimes {
            runtime.shutdown();
        }
    }

    #[test]
    fn dragging_pane_split_updates_captured_layout_ratio() {
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![Workspace::test_new("test")];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.workspaces[0].test_split(Direction::Horizontal);
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));
        let border = app.state.view.split_borders[0].clone();
        let before = capture_snapshot(&app.state);
        let drag_row = border.area.y.saturating_add(1);

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            border.pos,
            drag_row,
        ));
        app.handle_mouse(mouse(
            MouseEventKind::Drag(MouseButton::Left),
            border.pos.saturating_add(6),
            drag_row,
        ));

        let after = capture_snapshot(&app.state);
        assert_ne!(root_layout_ratio(&before), root_layout_ratio(&after));
    }

    #[test]
    fn pane_split_hitbox_does_not_overlap_right_pane_content() {
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![Workspace::test_new("test")];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.pane_gaps = false;
        app.state.workspaces[0].test_split(Direction::Horizontal);
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));
        let border = app.state.view.split_borders[0].clone();
        let row = border.area.y.saturating_add(1);

        assert!(app
            .state
            .find_border_at(border.pos.saturating_sub(1), row)
            .is_none());
        assert!(app.state.find_border_at(border.pos, row).is_some());
        assert!(app
            .state
            .find_border_at(border.pos.saturating_add(1), row)
            .is_none());
    }

    #[test]
    fn pane_split_hitbox_does_not_overlap_bottom_pane_content() {
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![Workspace::test_new("test")];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.pane_gaps = false;
        app.state.workspaces[0].test_split(Direction::Vertical);
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));
        let border = app.state.view.split_borders[0].clone();
        let col = border.area.x.saturating_add(1);

        assert!(app
            .state
            .find_border_at(col, border.pos.saturating_sub(1))
            .is_none());
        assert!(app.state.find_border_at(col, border.pos).is_some());
        assert!(app
            .state
            .find_border_at(col, border.pos.saturating_add(1))
            .is_none());
    }

    #[test]
    fn borderless_no_gap_split_has_no_mouse_hitbox_over_content() {
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![Workspace::test_new("test")];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.pane_borders = false;
        app.state.workspaces[0].test_split(Direction::Horizontal);
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));
        let border = app.state.view.split_borders[0].clone();
        let row = border.area.y.saturating_add(1);

        assert!(app.state.find_border_at(border.pos, row).is_none());
    }

    #[test]
    fn bordered_pane_gaps_keep_both_split_borders_draggable() {
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![Workspace::test_new("test")];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.pane_gaps = true;
        app.state.workspaces[0].test_split(Direction::Horizontal);
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));
        let border = app.state.view.split_borders[0].clone();
        let row = border.area.y.saturating_add(1);

        assert!(app
            .state
            .find_border_at(border.pos.saturating_sub(1), row)
            .is_some());
        assert!(app.state.find_border_at(border.pos, row).is_some());
        assert!(app
            .state
            .find_border_at(border.pos.saturating_add(1), row)
            .is_none());
    }

    #[test]
    fn borderless_pane_gap_is_not_a_pane_but_remains_split_draggable() {
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![Workspace::test_new("test")];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.pane_borders = false;
        app.state.pane_gaps = true;
        app.state.workspaces[0].test_split(Direction::Horizontal);
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));
        let border = app.state.view.split_borders[0].clone();
        let row = border.area.y.saturating_add(1);
        let gap_col = border.pos.saturating_sub(1);

        assert!(app.state.pane_at(gap_col, row).is_none());
        assert!(app.state.find_border_at(gap_col, row).is_some());
        assert!(app.state.find_border_at(border.pos, row).is_none());
    }

    #[test]
    fn borderless_gap_hitbox_is_empty_when_first_split_side_has_one_cell() {
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![Workspace::test_new("test")];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.pane_borders = false;
        app.state.pane_gaps = true;
        app.state.workspaces[0].test_split(Direction::Horizontal);
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 2, 4));
        let border = app.state.view.split_borders[0].clone();
        let row = border.area.y.saturating_add(1);
        let candidate_gap_col = border.pos.saturating_sub(1);

        assert!(app.state.pane_frame_at(candidate_gap_col, row).is_some());
        assert!(app.state.find_border_at(candidate_gap_col, row).is_none());
    }

    #[test]
    fn borderless_gap_hitbox_is_empty_when_first_split_side_has_zero_width() {
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![Workspace::test_new("test")];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.pane_borders = false;
        app.state.pane_gaps = true;
        app.state.workspaces[0].test_split(Direction::Horizontal);
        app.state.workspaces[0].tabs[0]
            .layout
            .set_ratio_at(&[], 0.1);
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 1, 4));
        let border = app.state.view.split_borders[0].clone();
        let row = border.area.y.saturating_add(1);

        assert_eq!(border.pos, 0);
        assert!(app.state.find_border_at(0, row).is_none());
    }

    #[test]
    fn selecting_from_right_pane_first_content_column_starts_selection() {
        let mut app = app_for_mouse_test();
        let mut ws = Workspace::test_new("test");
        let second_pane = ws.test_split(Direction::Horizontal);
        app.state.workspaces = vec![ws];
        app.state.active = Some(0);
        app.state.selected = 0;
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));

        let second_info = app
            .state
            .view
            .pane_infos
            .iter()
            .find(|info| info.id == second_pane)
            .expect("second pane info")
            .clone();
        let col = second_info.inner_rect.x;
        let row = second_info.inner_rect.y;

        assert!(app.state.find_border_at(col, row).is_none());
        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), col, row));

        assert!(app.state.drag.is_none());
        assert_eq!(
            app.state
                .selection
                .as_ref()
                .map(|selection| selection.pane_id),
            Some(second_pane)
        );
    }

    #[test]
    fn selecting_from_bottom_pane_first_content_row_starts_selection() {
        let mut app = app_for_mouse_test();
        let mut ws = Workspace::test_new("test");
        let second_pane = ws.test_split(Direction::Vertical);
        app.state.workspaces = vec![ws];
        app.state.active = Some(0);
        app.state.selected = 0;
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));

        let second_info = app
            .state
            .view
            .pane_infos
            .iter()
            .find(|info| info.id == second_pane)
            .expect("second pane info")
            .clone();
        let col = second_info.inner_rect.x;
        let row = second_info.inner_rect.y;

        assert!(app.state.find_border_at(col, row).is_none());
        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), col, row));

        assert!(app.state.drag.is_none());
        assert_eq!(
            app.state
                .selection
                .as_ref()
                .map(|selection| selection.pane_id),
            Some(second_pane)
        );
    }

    #[tokio::test]
    async fn dragging_vertical_pane_split_still_resizes_when_pane_mouse_reporting_is_enabled() {
        let mut app = app_for_mouse_test();
        let mut ws = Workspace::test_new("test");
        let first_pane = ws.tabs[0].root_pane;
        let second_pane = ws.test_split(Direction::Vertical);

        app.state.workspaces = vec![ws];
        app.state.active = Some(0);
        app.state.selected = 0;

        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));

        let pane_infos = app.state.view.pane_infos.clone();
        let first_info = pane_infos
            .iter()
            .find(|info| info.id == first_pane)
            .expect("first pane info")
            .clone();
        let second_info = pane_infos
            .iter()
            .find(|info| info.id == second_pane)
            .expect("second pane info")
            .clone();

        app.state.insert_test_runtime(
            first_pane,
            crate::terminal::TerminalRuntime::test_with_screen_bytes(
                first_info.inner_rect.width.max(1),
                first_info.inner_rect.height.max(1),
                b"\x1b[?1002h",
            ),
        );
        app.state.insert_test_runtime(
            second_pane,
            crate::terminal::TerminalRuntime::test_with_screen_bytes(
                second_info.inner_rect.width.max(1),
                second_info.inner_rect.height.max(1),
                b"\x1b[?1002h",
            ),
        );

        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));
        let border = app
            .state
            .view
            .split_borders
            .iter()
            .find(|border| border.direction == Direction::Vertical)
            .expect("vertical split border")
            .clone();
        let before = capture_snapshot(&app.state);
        let drag_col = border.area.x.saturating_add(1);

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            drag_col,
            border.pos,
        ));
        app.handle_mouse(mouse(
            MouseEventKind::Drag(MouseButton::Left),
            drag_col,
            border.pos.saturating_add(4),
        ));

        let after = capture_snapshot(&app.state);
        assert_ne!(root_layout_ratio(&before), root_layout_ratio(&after));
    }

    #[tokio::test]
    async fn dragging_horizontal_pane_split_still_resizes_when_pane_mouse_reporting_is_enabled() {
        let mut app = app_for_mouse_test();
        let mut ws = Workspace::test_new("test");
        let first_pane = ws.tabs[0].root_pane;
        let second_pane = ws.test_split(Direction::Horizontal);

        app.state.workspaces = vec![ws];
        app.state.active = Some(0);
        app.state.selected = 0;

        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));

        let pane_infos = app.state.view.pane_infos.clone();
        let first_info = pane_infos
            .iter()
            .find(|info| info.id == first_pane)
            .expect("first pane info")
            .clone();
        let second_info = pane_infos
            .iter()
            .find(|info| info.id == second_pane)
            .expect("second pane info")
            .clone();

        app.state.insert_test_runtime(
            first_pane,
            crate::terminal::TerminalRuntime::test_with_screen_bytes(
                first_info.inner_rect.width.max(1),
                first_info.inner_rect.height.max(1),
                b"\x1b[?1002h",
            ),
        );
        app.state.insert_test_runtime(
            second_pane,
            crate::terminal::TerminalRuntime::test_with_screen_bytes(
                second_info.inner_rect.width.max(1),
                second_info.inner_rect.height.max(1),
                b"\x1b[?1002h",
            ),
        );

        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));
        let border = app
            .state
            .view
            .split_borders
            .iter()
            .find(|border| border.direction == Direction::Horizontal)
            .expect("horizontal split border")
            .clone();
        let before = capture_snapshot(&app.state);
        let drag_row = border.area.y.saturating_add(1);

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            border.pos,
            drag_row,
        ));
        app.handle_mouse(mouse(
            MouseEventKind::Drag(MouseButton::Left),
            border.pos.saturating_add(6),
            drag_row,
        ));

        let after = capture_snapshot(&app.state);
        assert_ne!(root_layout_ratio(&before), root_layout_ratio(&after));
    }

    #[test]
    fn wheel_routing_prefers_mouse_reporting() {
        let input_state = crate::pane::InputState {
            alternate_screen: true,
            application_cursor: false,
            bracketed_paste: false,
            focus_reporting: false,
            mouse_protocol_mode: crate::input::MouseProtocolMode::ButtonMotion,
            mouse_protocol_encoding: crate::input::MouseProtocolEncoding::Sgr,
            mouse_alternate_scroll: true,
            modify_other_keys: false,
            color_scheme_reporting: false,
        };

        assert_eq!(wheel_routing(input_state), WheelRouting::MouseReport);
    }

    #[test]
    fn wheel_over_tab_bar_switches_tabs() {
        let mut app = app_for_mouse_test();
        let mut ws = Workspace::test_new("one");
        ws.test_add_tab(Some("two"));
        ws.test_add_tab(Some("three"));
        app.state.workspaces = vec![ws];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;

        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));
        let tab_bar = app.state.view.tab_bar_rect;

        app.handle_mouse(mouse(MouseEventKind::ScrollDown, tab_bar.x + 1, tab_bar.y));
        assert_eq!(app.state.workspaces[0].active_tab_index(), 1);

        app.handle_mouse(mouse(MouseEventKind::ScrollUp, tab_bar.x + 1, tab_bar.y));
        assert_eq!(app.state.workspaces[0].active_tab_index(), 0);

        app.handle_mouse(mouse(MouseEventKind::ScrollUp, tab_bar.x + 1, tab_bar.y));
        assert_eq!(app.state.workspaces[0].active_tab_index(), 2);

        app.handle_mouse(mouse(
            MouseEventKind::ScrollDown,
            tab_bar.x + tab_bar.width.saturating_sub(1),
            tab_bar.y,
        ));
        assert_eq!(app.state.workspaces[0].active_tab_index(), 0);
    }

    struct StripFixtureRoot(std::path::PathBuf);

    impl Drop for StripFixtureRoot {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn files_strip_fixture(name: &str) -> (crate::app::App, StripFixtureRoot) {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "herdr-strip-{}-{}-{}",
            name,
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&root).expect("create strip fixture root");
        std::fs::write(root.join("00.txt"), b"x").expect("strip fixture entry");

        let mut app = app_for_mouse_test();
        let mut ws = Workspace::test_new("one");
        ws.test_add_tab(Some("two"));
        app.state.workspaces = vec![ws];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;
        (app, StripFixtureRoot(root))
    }

    fn open_files_surface(app: &mut crate::app::App, root: &StripFixtureRoot) {
        app.state
            .try_open_file_manager_with(|_| Some(crate::fm::FmState::new(&root.0)))
            .expect("Files activation");
    }

    const STRIP_AREA: Rect = Rect {
        x: 0,
        y: 0,
        width: 106,
        height: 20,
    };

    // TP-FTAB-INPUT-01: clicking a terminal tab while Files owns the stage
    // switches surfaces; it must NOT close Files. A click that destroyed the
    // other tab would not be tab behavior at all.
    #[test]
    fn clicking_a_terminal_tab_leaves_files_open_as_an_inactive_entry() {
        let (mut app, root) = files_strip_fixture("switch-back");
        open_files_surface(&mut app, &root);
        crate::ui::compute_view(&mut app.state, STRIP_AREA);
        assert_eq!(
            app.state.stage.surface_view(),
            crate::ui::surface_host::StageSurfaceView::NativeFiles,
            "control: Files owns the stage"
        );

        let second_tab = app.state.view.tab_hit_areas[1];
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            second_tab.x + 1,
            second_tab.y,
        ));
        app.handle_mouse(mouse(
            MouseEventKind::Up(MouseButton::Left),
            second_tab.x + 1,
            second_tab.y,
        ));

        assert_eq!(
            app.state.stage.surface_view(),
            crate::ui::surface_host::StageSurfaceView::TerminalWorkspace,
            "the clicked terminal tab must own the stage"
        );
        assert_eq!(app.state.workspaces[0].active_tab_index(), 1);
        assert!(
            app.state.file_manager.is_some(),
            "switching tabs must not close Files"
        );

        crate::ui::compute_view(&mut app.state, STRIP_AREA);
        assert_eq!(
            app.state.view.stage_tab_hit_areas.len(),
            1,
            "the Files entry stays in the strip while inactive"
        );
    }

    // TP-FTAB-INPUT-02: an inactive Files tab owns no projected geometry. The
    // surface guard has to be the active surface, not "is a file manager open",
    // or a hidden Files tab keeps rows clickable under the terminal.
    #[test]
    fn inactive_files_tab_projects_no_stage_geometry() {
        let (mut app, root) = files_strip_fixture("hidden-geometry");
        open_files_surface(&mut app, &root);
        crate::ui::compute_view(&mut app.state, STRIP_AREA);
        assert!(
            !app.state.view.file_manager_row_areas.is_empty(),
            "control: the active Files surface projects row geometry"
        );

        let first_tab = app.state.view.tab_hit_areas[0];
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            first_tab.x + 1,
            first_tab.y,
        ));
        app.handle_mouse(mouse(
            MouseEventKind::Up(MouseButton::Left),
            first_tab.x + 1,
            first_tab.y,
        ));
        crate::ui::compute_view(&mut app.state, STRIP_AREA);

        assert!(
            app.state.view.file_manager_row_areas.is_empty(),
            "a hidden Files tab must project no row geometry"
        );
        assert!(app.state.view.file_manager_row_action_areas.is_empty());
        assert!(app.state.view.file_manager_header_action_areas.is_empty());
        assert!(
            !app.state.view.pane_infos.is_empty(),
            "the terminal surface reclaims its pane geometry in the same frame"
        );
    }

    // TP-FTAB-INPUT-03: the Files entry activates its own instance, and the
    // switch itself retires the terminal projection — the same contract the
    // launcher path already carries, now reachable from the strip.
    #[test]
    fn clicking_the_files_entry_activates_it_and_retires_terminal_geometry() {
        let (mut app, root) = files_strip_fixture("activate");
        open_files_surface(&mut app, &root);
        crate::ui::compute_view(&mut app.state, STRIP_AREA);

        let first_tab = app.state.view.tab_hit_areas[0];
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            first_tab.x + 1,
            first_tab.y,
        ));
        app.handle_mouse(mouse(
            MouseEventKind::Up(MouseButton::Left),
            first_tab.x + 1,
            first_tab.y,
        ));
        crate::ui::compute_view(&mut app.state, STRIP_AREA);
        assert!(!app.state.view.pane_infos.is_empty(), "control: terminal");

        let files_entry = app.state.view.stage_tab_hit_areas[0].rect;
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            files_entry.x + 1,
            files_entry.y,
        ));
        app.handle_mouse(mouse(
            MouseEventKind::Up(MouseButton::Left),
            files_entry.x + 1,
            files_entry.y,
        ));

        assert_eq!(
            app.state.stage.surface_view(),
            crate::ui::surface_host::StageSurfaceView::NativeFiles
        );
        assert!(
            app.state.view.pane_infos.is_empty(),
            "the switch itself must retire stale pane hit geometry"
        );
    }

    // TP-FTAB-INPUT-04: strip geometry is inert once its instance is gone. A
    // rect retained across a close must not activate whatever now occupies it.
    #[test]
    fn stage_entry_geometry_is_inert_after_its_instance_closes() {
        let (mut app, root) = files_strip_fixture("stale");
        open_files_surface(&mut app, &root);
        crate::ui::compute_view(&mut app.state, STRIP_AREA);
        let stale_entry = app.state.view.stage_tab_hit_areas[0].rect;

        app.state.close_file_manager();
        crate::ui::compute_view(&mut app.state, STRIP_AREA);
        let retained_tab = app.state.workspaces[0].active_tab_index();

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            stale_entry.x + 1,
            stale_entry.y,
        ));
        app.handle_mouse(mouse(
            MouseEventKind::Up(MouseButton::Left),
            stale_entry.x + 1,
            stale_entry.y,
        ));

        assert_eq!(
            app.state.stage.surface_view(),
            crate::ui::surface_host::StageSurfaceView::TerminalWorkspace,
            "a retired entry cannot bring its surface back"
        );
        assert!(app.state.file_manager.is_none());
        assert_eq!(app.state.workspaces[0].active_tab_index(), retained_tab);
    }

    #[test]
    fn bottom_mode_bar_consumes_hidden_tab_mouse_actions() {
        let mut app = app_for_mouse_test();
        let mut ws = Workspace::test_new("one");
        ws.test_add_tab(Some("two"));
        app.state.workspaces = vec![ws];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Prefix;
        app.state.tab_bar_position = crate::config::TabBarPositionConfig::Bottom;

        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));
        let second_tab = app.state.view.tab_hit_areas[1];
        let new_tab = app.state.view.new_tab_hit_area;

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            second_tab.x,
            second_tab.y,
        ));
        app.handle_mouse(mouse(
            MouseEventKind::Up(MouseButton::Left),
            second_tab.x,
            second_tab.y,
        ));
        app.handle_mouse(mouse(
            MouseEventKind::ScrollDown,
            second_tab.x,
            second_tab.y,
        ));
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Right),
            second_tab.x,
            second_tab.y,
        ));
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            new_tab.x,
            new_tab.y,
        ));

        app.state.drag = Some(DragState {
            target: DragTarget::SidebarDivider,
        });
        app.handle_mouse(mouse(
            MouseEventKind::Up(MouseButton::Left),
            second_tab.x,
            second_tab.y,
        ));

        assert_eq!(app.state.workspaces[0].active_tab_index(), 0);
        assert_eq!(app.state.workspaces[0].tabs.len(), 2);
        assert!(app.state.context_menu.is_none());
        assert!(app.state.tab_presses.is_empty());
        assert!(app.state.drag.is_none());
    }

    #[test]
    fn right_click_inactive_tab_opens_menu_without_switching_tabs() {
        let mut app = app_for_mouse_test();
        let mut ws = Workspace::test_new("one");
        ws.test_add_tab(Some("two"));
        app.state.workspaces = vec![ws];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;

        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));
        let second_tab = app.state.view.tab_hit_areas[1];

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Right),
            second_tab.x + 1,
            second_tab.y,
        ));

        assert_eq!(app.state.workspaces[0].active_tab_index(), 0);
        let menu = app.state.context_menu.as_ref().expect("tab context menu");
        assert_eq!(
            menu.kind,
            ContextMenuKind::Tab {
                ws_idx: 0,
                tab_idx: 1
            }
        );
        assert_eq!(app.state.mode, Mode::ContextMenu);
    }

    #[test]
    fn clicking_tab_context_menu_close_leaves_context_menu_mode() {
        let mut app = app_for_mouse_test();
        let mut ws = Workspace::test_new("one");
        ws.test_add_tab(Some("two"));
        app.state.workspaces = vec![ws];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;

        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));
        let second_tab = app.state.view.tab_hit_areas[1];

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Right),
            second_tab.x + 1,
            second_tab.y,
        ));

        let menu = app
            .state
            .context_menu_rect()
            .expect("tab context menu rect");
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            menu.x + 2,
            menu.y + 3,
        ));

        assert_eq!(app.state.workspaces[0].tabs.len(), 1);
        assert_eq!(app.state.workspaces[0].display_name(), "one");
        assert!(app.state.context_menu.is_none());
        assert_eq!(app.state.mode, Mode::Terminal);
        assert!(app
            .event_hub
            .events_after(0)
            .iter()
            .any(|(_, event)| { matches!(event.event, crate::api::schema::EventKind::TabClosed) }));
    }

    #[test]
    fn clicking_pane_context_menu_close_leaves_context_menu_mode() {
        let mut app = app_for_mouse_test();
        let mut ws = Workspace::test_new("one");
        let first_pane = ws.tabs[0].root_pane;
        let second_pane = ws.test_split(Direction::Horizontal);
        ws.tabs[0].layout.focus_pane(second_pane);
        app.state.workspaces = vec![ws];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;

        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));
        let first_info = app
            .state
            .view
            .pane_infos
            .iter()
            .find(|info| info.id == first_pane)
            .expect("first pane info")
            .clone();

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Right),
            first_info.inner_rect.x + 1,
            first_info.inner_rect.y + 1,
        ));

        let menu_state = app.state.context_menu.as_ref().expect("pane context menu");
        let close_idx = menu_state
            .items()
            .iter()
            .position(|item| *item == "Close pane")
            .expect("close pane menu item");
        let menu = app
            .state
            .context_menu_rect()
            .expect("pane context menu rect");
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            menu.x + 2,
            menu.y + 1 + close_idx as u16,
        ));

        assert_eq!(app.state.workspaces[0].tabs[0].layout.pane_count(), 1);
        assert!(app.state.context_menu.is_none());
        assert_eq!(app.state.mode, Mode::Terminal);
        assert!(app.event_hub.events_after(0).iter().any(|(_, event)| {
            matches!(event.event, crate::api::schema::EventKind::PaneClosed)
        }));
    }

    #[test]
    fn clicking_pane_context_menu_close_last_parent_group_pane_keeps_confirmation_mode() {
        let mut app = app_for_mouse_test();
        let mut parent = Workspace::test_new("main");
        let pane_id = parent.tabs[0].root_pane;
        mark_worktree_space_member(&mut parent, 0, "repo-key");
        let mut child = Workspace::test_new("issue");
        mark_worktree_space_member(&mut child, 1, "repo-key");
        app.state.workspaces = vec![parent, child];
        app.state.active = Some(0);
        app.state.selected = 1;
        app.state.mode = Mode::Terminal;

        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));
        let pane_info = app
            .state
            .view
            .pane_infos
            .iter()
            .find(|info| info.id == pane_id)
            .expect("pane info")
            .clone();

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Right),
            pane_info.inner_rect.x + 1,
            pane_info.inner_rect.y + 1,
        ));

        let menu_state = app.state.context_menu.as_ref().expect("pane context menu");
        let close_idx = menu_state
            .items()
            .iter()
            .position(|item| *item == "Close pane")
            .expect("close pane menu item");
        let menu = app
            .state
            .context_menu_rect()
            .expect("pane context menu rect");
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            menu.x + 2,
            menu.y + 1 + close_idx as u16,
        ));

        assert_eq!(app.state.selected, 0);
        assert_eq!(app.state.mode, Mode::ConfirmClose);
        assert_eq!(app.state.workspaces.len(), 2);
        assert!(app.state.context_menu.is_none());
    }

    #[test]
    fn wheel_over_overflowing_tab_bar_switches_tabs() {
        let mut app = app_for_mouse_test();
        let mut ws = Workspace::test_new("one");
        ws.tabs[0].set_custom_name("very-long-one".into());
        ws.test_add_tab(Some("very-long-two"));
        ws.test_add_tab(Some("very-long-three"));
        app.state.workspaces = vec![ws];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;

        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 65, 20));
        assert!(app.state.view.tab_scroll_right_hit_area.width > 0);
        let tab_bar = app.state.view.tab_bar_rect;

        app.handle_mouse(mouse(
            MouseEventKind::ScrollDown,
            tab_bar.x + tab_bar.width.saturating_sub(2),
            tab_bar.y,
        ));
        assert_eq!(app.state.workspaces[0].active_tab_index(), 1);

        app.handle_mouse(mouse(
            MouseEventKind::ScrollDown,
            tab_bar.x + tab_bar.width.saturating_sub(2),
            tab_bar.y,
        ));
        assert_eq!(app.state.workspaces[0].active_tab_index(), 2);
    }

    #[test]
    fn wheel_outside_tab_bar_does_not_switch_tabs() {
        let mut app = app_for_mouse_test();
        let mut ws = Workspace::test_new("one");
        ws.test_add_tab(Some("two"));
        app.state.workspaces = vec![ws];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;

        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));
        let terminal = app.state.view.terminal_area;

        app.handle_mouse(mouse(
            MouseEventKind::ScrollDown,
            terminal.x + 1,
            terminal.y + 1,
        ));

        assert_eq!(app.state.workspaces[0].active_tab_index(), 0);
    }

    #[test]
    fn mobile_switch_button_opens_switcher_and_workspace_row_switches_workspace() {
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![Workspace::test_new("one"), Workspace::test_new("two")];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;

        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 44, 20));
        assert_eq!(app.state.view.layout, ViewLayout::Mobile);

        let switch = app.state.view.mobile_header_hits.spaces_menu;
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            switch.x + 1,
            switch.y + 1,
        ));

        assert_eq!(app.state.mode, Mode::Navigate);

        // Each workspace over its three touch rows (TP-MOB-87), the second
        // one spanning document rows 3..6. The create row that used to lead
        // the document moved to the pinned footer (TP-MOB-77), so the list
        // starts with the workspaces themselves.
        let viewport = crate::ui::mobile_drawer_areas(&app.state).viewport;
        // Mid-row: the head cells are the chat disclosure and the tail cells
        // start a chat since TP-MOB-84, so "switch to it" is the middle.
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            viewport.x + viewport.width / 2,
            viewport.y + 4,
        ));

        assert_eq!(app.state.active, Some(1));
        assert_eq!(app.state.mode, Mode::Terminal);
    }

    #[test]
    fn mobile_workspace_panel_scroll_reaches_extra_workspaces() {
        let mut app = app_for_mouse_test();
        app.state.workspaces = (0..12)
            .map(|idx| Workspace::test_new(&format!("ws-{idx}")))
            .collect();
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;

        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 44, 20));
        let switch = app.state.view.mobile_header_hits.spaces_menu;
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            switch.x + 1,
            switch.y + 1,
        ));
        assert_eq!(app.state.mode, Mode::Navigate);

        let viewport = crate::ui::mobile_drawer_areas(&app.state).viewport;
        app.handle_mouse(mouse(
            MouseEventKind::ScrollDown,
            viewport.x + 2,
            viewport.y,
        ));
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 44, 20));
        assert_eq!(app.state.mobile_switcher_scroll, 2);

        // Scrolled two rows, the third viewport row is document row four,
        // inside ws-1's three-row span (3..6, TP-MOB-87) — the point being
        // that a scrolled row is reachable at all.
        // Mid-row: the head cells are the chat disclosure and the tail cells
        // start a chat since TP-MOB-84, so "switch to it" is the middle.
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            viewport.x + viewport.width / 2,
            viewport.y + 2,
        ));

        assert_eq!(app.state.active, Some(1));
        assert_eq!(app.state.mode, Mode::Terminal);
    }

    /// A mobile app already through `compute_view`, ready for header taps.
    fn mobile_app_for_drawers(w: u16, h: u16) -> crate::app::App {
        let mut app = app_for_mouse_test();
        let mut first = Workspace::test_new("one");
        first.test_add_tab(Some("logs"));
        app.state.workspaces = vec![first, Workspace::test_new("two")];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;
        // The reader's phone reports 76 columns, above the 64-column default;
        // the fixture has to be able to describe that viewport as a phone.
        app.state.mobile_width_threshold = app.state.mobile_width_threshold.max(w);
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, w, h));
        app
    }

    fn tap(app: &mut crate::app::App, x: u16, y: u16) {
        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), x, y));
    }

    // TP-MOB-96: the drawer answers taps over the file browser too. Reported
    // live: with Files open, the header button still opened the drawer but
    // every tap inside it — segments, rows — fell through dead, leaving no
    // way to leave Files by touch.
    #[test]
    fn the_drawer_stays_responsive_over_the_file_browser() {
        let mut app = mobile_app_for_drawers(76, 35);
        if let Some(ws) = app
            .state
            .active
            .and_then(|i| app.state.workspaces.get_mut(i))
        {
            ws.identity_cwd = std::env::temp_dir();
        }
        app.state
            .apply_mobile_switcher_target(crate::ui::MobileSwitcherTarget::DrawerSegment(
                crate::app::state::SidebarTab::Files,
            ));
        assert_eq!(
            app.state.stage.surface_view(),
            crate::ui::surface_host::StageSurfaceView::NativeFiles,
            "files is open"
        );
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 76, 35));

        // The header button opens the drawer over the browser…
        let hits = app.state.view.mobile_header_hits;
        tap(&mut app, hits.spaces_menu.x + 1, hits.spaces_menu.y + 1);
        assert_eq!(
            app.state.mobile_drawer,
            crate::app::state::MobileDrawer::Spaces,
            "the drawer opens over files"
        );

        // …and the drawer's own targets keep answering.
        let areas = crate::ui::mobile_drawer_areas(&app.state);
        tap(
            &mut app,
            areas.title.x + areas.title.width / 2,
            areas.title.y,
        );
        assert_eq!(
            app.state.sidebar_tab,
            crate::app::state::SidebarTab::Projects,
            "a segment tap answers over the file browser"
        );
    }

    // TP-MOB-40: each header button opens its own drawer, and opening one
    // closes the other. Two booleans would have let both be open at once; the
    // enum makes that unrepresentable.
    #[test]
    fn each_header_button_opens_its_own_drawer_and_closes_the_other() {
        let mut app = mobile_app_for_drawers(44, 20);
        let hits = app.state.view.mobile_header_hits;

        tap(&mut app, hits.spaces_menu.x + 1, hits.spaces_menu.y);
        assert_eq!(
            app.state.mobile_drawer,
            crate::app::state::MobileDrawer::Spaces
        );
        assert_eq!(app.state.mode, Mode::Navigate);

        tap(&mut app, hits.tabs_menu.x + 1, hits.tabs_menu.y);
        assert_eq!(
            app.state.mobile_drawer,
            crate::app::state::MobileDrawer::Tabs,
            "the second button takes over"
        );
        assert_eq!(app.state.mode, Mode::Navigate);
    }

    // TP-MOB-41: pressing the open drawer's own button closes it. A control
    // that opens but cannot close is half a toggle, and on a phone the button
    // is the nearest thing to hand.
    #[test]
    fn pressing_an_open_drawers_own_button_closes_it() {
        let mut app = mobile_app_for_drawers(44, 20);
        let hits = app.state.view.mobile_header_hits;

        tap(&mut app, hits.spaces_menu.x + 1, hits.spaces_menu.y);
        tap(&mut app, hits.spaces_menu.x + 1, hits.spaces_menu.y);

        assert_eq!(
            app.state.mobile_drawer,
            crate::app::state::MobileDrawer::None
        );
        assert_eq!(app.state.mode, Mode::Terminal);
    }

    // TP-MOB-42: the active-tab strip dispatches the same action as the button
    // beside it. It is the larger target for the same intent, so a tap that
    // misses the three-column button still lands somewhere useful.
    #[test]
    fn the_tab_strip_opens_the_same_drawer_as_the_button_beside_it() {
        let mut by_strip = mobile_app_for_drawers(44, 20);
        let strip = by_strip.state.view.mobile_header_hits.tab_strip;
        tap(&mut by_strip, strip.x + strip.width / 2, strip.y);

        let mut by_button = mobile_app_for_drawers(44, 20);
        let button = by_button.state.view.mobile_header_hits.tabs_menu;
        tap(&mut by_button, button.x + 1, button.y);

        assert_eq!(
            by_strip.state.mobile_drawer,
            crate::app::state::MobileDrawer::Tabs
        );
        assert_eq!(by_strip.state.mobile_drawer, by_button.state.mobile_drawer);
        assert_eq!(by_strip.state.mode, by_button.state.mode);
    }

    // TP-MOB-43: tapping the uncovered strip closes the drawer and does not
    // reach the terminal under it. A scrim that leaks its tap would focus a
    // pane the reader was only trying to dismiss a panel from.
    #[test]
    fn tapping_the_scrim_closes_the_drawer_without_reaching_the_terminal() {
        let mut app = mobile_app_for_drawers(44, 20);
        let hits = app.state.view.mobile_header_hits;
        tap(&mut app, hits.spaces_menu.x + 1, hits.spaces_menu.y);

        let before = app.state.active;
        let scrim = crate::ui::mobile_drawer_areas(&app.state).scrim;
        tap(
            &mut app,
            scrim.x + scrim.width / 2,
            scrim.y + scrim.height / 2,
        );

        assert_eq!(
            app.state.mobile_drawer,
            crate::app::state::MobileDrawer::None
        );
        assert_eq!(app.state.mode, Mode::Terminal);
        assert_eq!(app.state.active, before, "the tap did not focus anything");
    }

    // TP-MOB-44: a tap inside the drawer that lands on no row leaves it open.
    // Closing on any miss would make every scroll and every near-miss in a
    // list dismiss the thing being read.
    #[test]
    fn tapping_empty_space_inside_the_drawer_leaves_it_open() {
        let mut app = mobile_app_for_drawers(44, 20);
        let hits = app.state.view.mobile_header_hits;
        tap(&mut app, hits.tabs_menu.x + 1, hits.tabs_menu.y);

        // The tabs drawer holds a create row and two tabs; anything below that
        // is empty panel.
        let areas = crate::ui::mobile_drawer_areas(&app.state);
        tap(
            &mut app,
            areas.viewport.x + 2,
            areas.viewport.y + areas.viewport.height - 1,
        );

        assert_eq!(
            app.state.mobile_drawer,
            crate::app::state::MobileDrawer::Tabs,
            "an empty row inside the drawer is not a dismissal"
        );
    }

    // TP-MOB-65: a tap reported one column past the last cell still reaches the
    // target under the last cell. A touch client clamps an edge tap to the
    // screen width rather than width - 1, so on a 76-column phone taps arrive
    // at column 76 — outside every rect, and the rightmost column is exactly
    // where one of the two header buttons lives. Measured live: 3 of 37 taps.
    #[test]
    fn a_tap_past_the_last_column_still_reaches_the_button_there() {
        let mut app = mobile_app_for_drawers(76, 35);
        assert_eq!(app.state.view.mobile_header_hits.tabs_menu.right(), 76);

        tap(&mut app, 76, 0);
        assert_eq!(
            app.state.mobile_drawer,
            crate::app::state::MobileDrawer::Tabs,
            "an edge tap the client clamped one column too far is still a tap on the button"
        );
    }

    // TP-MOB-66: the header buttons accept a tap one row below the header. The
    // header is two rows tall and a thumb aiming at a 5x2 target routinely
    // lands just under it; those taps used to reach the terminal and do
    // nothing, which reads as the button being broken. Measured live: 3 of 37
    // taps landed on rows 2 and 4.
    #[test]
    fn the_header_buttons_accept_a_tap_just_below_them() {
        let mut app = mobile_app_for_drawers(76, 35);
        let hits = app.state.view.mobile_header_hits;
        assert_eq!(
            hits.spaces_menu.height, 5,
            "four drawn rows plus one of reach (TP-MOB-89)"
        );
        assert_eq!(
            hits.tab_strip.height, 4,
            "the strip does not overshoot: it spans most of the width and would \
             swallow the terminal's top row"
        );

        tap(&mut app, hits.spaces_menu.x + 2, 4);
        assert_eq!(
            app.state.mobile_drawer,
            crate::app::state::MobileDrawer::Spaces
        );
    }

    // TP-MOB-69 (tap path): tapping the workspace you are already in folds its chats and
    // leaves the drawer open; tapping any other one switches and closes. The
    // meaning of the tap follows from where the reader already is, which is
    // what lets the row carry a second intent without a second target.
    #[tokio::test]
    async fn tapping_the_active_workspace_folds_its_chats_without_closing_the_drawer() {
        let mut app = mobile_app_for_drawers(76, 35);
        let hits = app.state.view.mobile_header_hits;
        tap(&mut app, hits.spaces_menu.x + 1, hits.spaces_menu.y);
        assert_eq!(
            app.state.mobile_drawer,
            crate::app::state::MobileDrawer::Spaces
        );

        let active = app.state.active.expect("an active workspace");
        let range = crate::ui::mobile_drawer_workspace_doc_range(&app.state, active);
        let viewport = crate::ui::mobile_drawer_areas(&app.state).viewport;
        tap(&mut app, viewport.x + 2, viewport.y + range.start as u16);

        assert!(
            app.state.mobile_active_chats_folded,
            "the row you are on folds its own history"
        );
        assert_eq!(
            app.state.mobile_drawer,
            crate::app::state::MobileDrawer::Spaces,
            "folding narrows the list being read; it does not leave it"
        );
    }

    // TP-MOB-45: the header keeps two separate targets even on a viewport
    // barely wide enough for them. Overlapping targets would make one intent
    // unreachable without saying which.
    #[test]
    fn the_header_targets_never_overlap_however_narrow_the_viewport() {
        for width in 6..=64u16 {
            let app = mobile_app_for_drawers(width, 20);
            let hits = app.state.view.mobile_header_hits;
            assert!(
                hits.spaces_menu.width >= 3 && hits.tabs_menu.width >= 3,
                "at {width} columns both buttons keep a reachable width"
            );
            assert!(
                hits.spaces_menu.x + hits.spaces_menu.width <= hits.tabs_menu.x,
                "at {width} columns the buttons must not overlap"
            );
            assert!(
                hits.tab_strip.x >= hits.spaces_menu.x + hits.spaces_menu.width
                    && hits.tab_strip.x + hits.tab_strip.width <= hits.tabs_menu.x,
                "at {width} columns the strip stays between the buttons"
            );
        }
    }

    #[test]
    fn the_tabs_drawer_reaches_every_tab_without_scrolling() {
        let mut app = app_for_mouse_test();
        let mut ws = Workspace::test_new("one");
        ws.test_add_tab(Some("two"));
        ws.test_add_tab(Some("three"));
        ws.test_add_tab(Some("four"));
        app.state.workspaces = vec![ws];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;

        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 44, 24));
        let tabs_button = app.state.view.mobile_header_hits.tabs_menu;
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            tabs_button.x + 1,
            tabs_button.y + 1,
        ));
        assert_eq!(app.state.mode, Mode::Navigate);
        assert_eq!(
            app.state.mobile_drawer,
            crate::app::state::MobileDrawer::Tabs
        );

        // Four touch-height tabs fit a twenty-four-row phone without scrolling —
        // the create row moved to the pinned footer (TP-MOB-77), so the list
        // is the tabs alone. Reaching the fourth tab used to mean scrolling
        // past every workspace and every agent first, because tabs shared one
        // list with them.
        assert_eq!(crate::ui::mobile_drawer_max_scroll(&app.state), 0);

        // The fourth tab's span is document rows 9..12 (TP-MOB-87).
        let viewport = crate::ui::mobile_drawer_areas(&app.state).viewport;
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            viewport.x + 2,
            viewport.y + 10,
        ));

        assert_eq!(app.state.workspaces[0].active_tab_index(), 3);
        assert_eq!(app.state.mode, Mode::Terminal);
        assert_eq!(
            app.state.mobile_drawer,
            crate::app::state::MobileDrawer::None
        );
    }

    #[test]
    fn the_spaces_drawer_new_workspace_opens_the_prompt_when_enabled() {
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![Workspace::test_new("one")];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;
        app.state.prompt_new_workspace_name = true;

        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 44, 20));
        let switch = app.state.view.mobile_header_hits.spaces_menu;
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            switch.x + 1,
            switch.y + 1,
        ));
        // "+ new workspace" lives in the pinned footer band now (TP-MOB-77).
        let areas = crate::ui::mobile_drawer_areas(&app.state);
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            areas.footer.x + 2,
            areas.footer.y,
        ));

        assert_eq!(app.state.mode, Mode::RenameWorkspace);
        assert!(app.state.pending_workspace_create_cwd.is_some());
        assert!(app.state.name_input_replace_on_type);
        assert_eq!(app.state.workspaces.len(), 1);
    }

    #[test]
    fn desktop_new_workspace_opens_prompt_when_enabled() {
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![Workspace::test_new("one")];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;
        app.state.prompt_new_workspace_name = true;

        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 120, 40));
        let new_workspace = app.state.sidebar_new_button_rect();
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            new_workspace.x + 1,
            new_workspace.y,
        ));

        assert_eq!(app.state.mode, Mode::RenameWorkspace);
        assert!(app.state.pending_workspace_create_cwd.is_some());
        assert!(app.state.name_input_replace_on_type);
        assert_eq!(app.state.workspaces.len(), 1);
    }

    #[tokio::test]
    async fn desktop_new_workspace_creates_immediately_by_default() {
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![Workspace::test_new("one")];
        app.state.ensure_test_terminals();
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;

        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 120, 40));
        let new_workspace = app.state.sidebar_new_button_rect();
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            new_workspace.x + 1,
            new_workspace.y,
        ));

        assert_eq!(app.state.workspaces.len(), 2);
        assert_eq!(app.state.mode, Mode::Terminal);
        assert!(app.state.pending_workspace_create_cwd.is_none());
        crate::app::api::test_support::shutdown_test_runtimes(&mut app);
    }

    #[test]
    fn the_tabs_drawer_new_tab_opens_dialog_when_enabled() {
        let mut app = app_for_mouse_test();
        let mut ws = Workspace::test_new("one");
        ws.test_add_tab(Some("logs"));
        app.state.workspaces = vec![ws];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;

        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 44, 20));
        let switch = app.state.view.mobile_header_hits.tabs_menu;
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            switch.x + 1,
            switch.y + 1,
        ));
        let areas = crate::ui::mobile_drawer_areas(&app.state);
        // "+ new tab" lives in the pinned footer band now (TP-MOB-77).
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            areas.footer.x + 2,
            areas.footer.y,
        ));

        assert_eq!(app.state.mode, Mode::RenameTab);
        assert!(app.state.creating_new_tab);
    }

    // 4c: clicking a project header row on the Projects tab toggles its collapse
    // state, and does not leak into Spaces-only actions (new workspace).
    #[test]
    fn projects_tab_click_toggles_project_collapse() {
        let mut app = app_for_mouse_test();
        app.state.mode = Mode::Navigate;
        app.state.sidebar_tab = crate::app::state::SidebarTab::Projects;
        app.state.projects_sessions = vec![crate::app::state::ProjectSessions {
            path: std::path::PathBuf::from("/home/x/proj"),
            sessions: vec![crate::claude_sessions::ClaudeSession {
                id: "s1".to_string(),
                title: "a chat".to_string(),
                last_modified: std::time::SystemTime::UNIX_EPOCH,
                last_message_at: None,
                msg_count: 3,
                opening: None,
            }],
            total_count: 1,
        }];
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));

        let header = app.state.view.project_row_areas[0];
        assert!(matches!(
            header.kind,
            crate::app::state::ProjectRowKind::Project { proj_idx: 0 }
        ));
        let path = std::path::PathBuf::from("/home/x/proj");

        // First click collapses the project.
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            header.rect.x,
            header.rect.y,
        ));
        assert!(app.state.collapsed_project_paths.contains(&path));
        // A Spaces-only action must not fire on the Projects tab.
        assert!(!app.state.request_new_workspace);

        // Recompute (chats now hidden) and click again to expand.
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));
        let header = app.state.view.project_row_areas[0];
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            header.rect.x,
            header.rect.y,
        ));
        assert!(!app.state.collapsed_project_paths.contains(&path));
    }

    #[test]
    fn the_tabs_drawer_new_tab_skips_dialog_when_prompt_disabled() {
        let mut app = app_for_mouse_test();
        let mut ws = Workspace::test_new("one");
        ws.test_add_tab(Some("logs"));
        app.state.workspaces = vec![ws];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;
        app.state.prompt_new_tab_name = false;

        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 44, 20));
        let switch = app.state.view.mobile_header_hits.tabs_menu;
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            switch.x + 1,
            switch.y + 1,
        ));
        let areas = crate::ui::mobile_drawer_areas(&app.state);

        // "+ new tab" lives in the pinned footer band now (TP-MOB-77).
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            areas.footer.x + 2,
            areas.footer.y,
        ));
        assert_eq!(app.state.mode, Mode::Terminal);
        assert!(!app.state.creating_new_tab);
        assert!(app.state.request_new_tab);
        assert!(app.state.requested_new_tab_name.is_none());
    }

    // TP-TAB-SPLIT-02: the split buttons act on the focused pane through the
    // same api road the keyboard split takes — the press resolves to the
    // action, right button rightward, down button downward.
    #[test]
    fn a_click_on_a_split_button_asks_for_that_direction() {
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![Workspace::test_new("one")];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;

        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 120, 40));
        let right_area = app.state.view.split_right_hit_area;
        let down_area = app.state.view.split_down_hit_area;
        assert!(right_area.width > 0, "precondition: the buttons are drawn");

        let mut registry = TerminalRuntimeRegistry::new();
        let action = app.state.handle_mouse(
            &mut registry,
            crate::app::LOCAL_INPUT_SOURCE,
            mouse(
                MouseEventKind::Down(MouseButton::Left),
                right_area.x + 1,
                right_area.y,
            ),
        );
        assert!(
            matches!(
                action,
                Some(MouseAction::SplitFocusedPane {
                    direction: crate::api::schema::SplitDirection::Right
                })
            ),
            "the right button splits rightward"
        );

        let action = app.state.handle_mouse(
            &mut registry,
            crate::app::LOCAL_INPUT_SOURCE,
            mouse(
                MouseEventKind::Down(MouseButton::Left),
                down_area.x + 1,
                down_area.y,
            ),
        );
        assert!(
            matches!(
                action,
                Some(MouseAction::SplitFocusedPane {
                    direction: crate::api::schema::SplitDirection::Down
                })
            ),
            "the down button splits downward"
        );
    }

    // TP-TAB-SPLIT-02: the new-tab press still means new tab — the new
    // neighbours must not swallow it.
    #[test]
    fn the_new_tab_button_still_makes_a_tab_beside_the_split_buttons() {
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![Workspace::test_new("one")];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;
        app.state.prompt_new_tab_name = false;

        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 120, 40));
        let new_tab_area = app.state.view.new_tab_hit_area;
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            new_tab_area.x + 1,
            new_tab_area.y,
        ));

        assert!(app.state.request_new_tab);
    }

    #[test]
    fn desktop_new_tab_button_skips_dialog_when_prompt_disabled() {
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![Workspace::test_new("one")];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;
        app.state.prompt_new_tab_name = false;

        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 120, 40));
        let new_tab_area = app.state.view.new_tab_hit_area;
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            new_tab_area.x + 1,
            new_tab_area.y,
        ));

        assert_eq!(app.state.mode, Mode::Terminal);
        assert!(!app.state.creating_new_tab);
        assert!(app.state.request_new_tab);
        assert!(app.state.requested_new_tab_name.is_none());
    }

    #[test]
    fn mobile_switcher_swallows_non_left_mouse_events() {
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![Workspace::test_new("one")];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;

        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 44, 20));
        let switch = app.state.view.mobile_header_hits.spaces_menu;
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            switch.x + 1,
            switch.y + 1,
        ));
        assert_eq!(app.state.mode, Mode::Navigate);

        let viewport = crate::ui::mobile_drawer_areas(&app.state).viewport;
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Right),
            viewport.x + 2,
            viewport.y + 2,
        ));

        assert_eq!(app.state.mode, Mode::Navigate);
        assert!(app.state.context_menu.is_none());
    }

    #[test]
    fn mobile_switch_button_does_not_bypass_rename_modal() {
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![Workspace::test_new("one")];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::RenameTab;
        app.state.creating_new_tab = true;
        app.state.name_input = "new tab".into();

        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 44, 20));
        let switch = app.state.view.mobile_header_hits.spaces_menu;
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            switch.x + 1,
            switch.y + 1,
        ));

        assert_eq!(app.state.mode, Mode::Terminal);
        assert!(!app.state.creating_new_tab);
        assert!(!app.state.request_new_tab);
    }

    #[test]
    fn mobile_switcher_close_returns_to_terminal() {
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![Workspace::test_new("one")];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;

        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 44, 20));
        let switch = app.state.view.mobile_header_hits.spaces_menu;
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            switch.x + 1,
            switch.y + 1,
        ));
        assert_eq!(app.state.mode, Mode::Navigate);

        // The uncovered strip beside the drawer is what closes it now.
        let scrim = crate::ui::mobile_drawer_areas(&app.state).scrim;
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            scrim.x,
            scrim.y,
        ));

        assert_eq!(app.state.mode, Mode::Terminal);
    }

    #[test]
    fn wheel_routing_uses_alternate_scroll_in_fullscreen_without_mouse_reporting() {
        let input_state = crate::pane::InputState {
            alternate_screen: true,
            application_cursor: false,
            bracketed_paste: false,
            focus_reporting: false,
            mouse_protocol_mode: crate::input::MouseProtocolMode::None,
            mouse_protocol_encoding: crate::input::MouseProtocolEncoding::Default,
            mouse_alternate_scroll: true,
            modify_other_keys: false,
            color_scheme_reporting: false,
        };

        assert_eq!(wheel_routing(input_state), WheelRouting::AlternateScroll);
    }

    #[test]
    fn wheel_routing_falls_back_to_host_scrollback() {
        let input_state = crate::pane::InputState {
            alternate_screen: false,
            application_cursor: false,
            bracketed_paste: false,
            focus_reporting: false,
            mouse_protocol_mode: crate::input::MouseProtocolMode::None,
            mouse_protocol_encoding: crate::input::MouseProtocolEncoding::Default,
            mouse_alternate_scroll: true,
            modify_other_keys: false,
            color_scheme_reporting: false,
        };

        assert_eq!(wheel_routing(input_state), WheelRouting::HostScroll);
    }
}

#[cfg(test)]
mod per_client_tab_routing_tests {
    use super::super::{app_for_mouse_test, mouse};
    use crate::app::Mode;
    use crate::workspace::Workspace;
    use crossterm::event::MouseEventKind;
    use ratatui::layout::Rect;

    fn scroll_tab_bar(app: &mut crate::app::App, client: u64, kind: MouseEventKind) {
        let tab_bar = app.state.view.tab_bar_rect;
        app.route_client_events_from(
            client,
            vec![crate::raw_input::RawInputEvent::Mouse(mouse(
                kind,
                tab_bar.x + 1,
                tab_bar.y,
            ))],
            false,
        );
    }

    fn tab_seen_by(app: &mut crate::app::App, client: u64) -> usize {
        let previous = app.state.enter_viewer(Some(client));
        let index = app.state.workspaces[0].active_tab_index();
        app.state.restore_viewer(previous);
        index
    }

    // TP-MCF-TAB-05
    #[test]
    fn one_display_switching_tabs_leaves_the_other_display_where_it_was() {
        let mut app = app_for_mouse_test();
        let mut workspace = Workspace::test_new("fixture");
        workspace.test_add_tab(Some("two"));
        workspace.test_add_tab(Some("three"));
        app.state.workspaces = vec![workspace];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));

        // Both displays are attached and looking at the first tab.
        assert_eq!(tab_seen_by(&mut app, 1), 0);
        assert_eq!(tab_seen_by(&mut app, 2), 0);

        // The second display walks forward twice. The first display is not
        // touched at all — this is the whole point of the feature.
        scroll_tab_bar(&mut app, 2, MouseEventKind::ScrollDown);
        scroll_tab_bar(&mut app, 2, MouseEventKind::ScrollDown);

        assert_eq!(
            tab_seen_by(&mut app, 2),
            2,
            "the display that scrolled moves"
        );
        assert_eq!(
            tab_seen_by(&mut app, 1),
            0,
            "the display nobody touched must not follow the one that moved"
        );

        // And it works in both directions: the untouched display can still
        // drive itself afterwards.
        scroll_tab_bar(&mut app, 1, MouseEventKind::ScrollDown);
        assert_eq!(tab_seen_by(&mut app, 1), 1);
        assert_eq!(tab_seen_by(&mut app, 2), 2, "still independent");
    }

    // TP-MCF-TAB-07
    #[test]
    fn a_display_that_detaches_does_not_move_the_one_that_stays() {
        let mut app = app_for_mouse_test();
        let mut workspace = Workspace::test_new("fixture");
        workspace.test_add_tab(Some("two"));
        app.state.workspaces = vec![workspace];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));

        assert_eq!(tab_seen_by(&mut app, 1), 0);
        scroll_tab_bar(&mut app, 2, MouseEventKind::ScrollDown);
        assert_eq!(tab_seen_by(&mut app, 2), 1);

        app.state.forget_client(2);

        assert_eq!(
            tab_seen_by(&mut app, 1),
            0,
            "closing one terminal must not move the remaining one"
        );
    }
}
