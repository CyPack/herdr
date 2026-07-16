use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Position;

use super::super::state::{AppState, DragState, DragTarget, Mode, SidebarWidthSource};
use crate::ui::shell::{
    CollapseDecision, DividerId, RegionId, ResizeBounds, ResizeDecision, ResizeTransaction,
    ResizeUpdate, ShellDirection,
};

/// The single owner the frozen shell input precedence resolves for one event.
///
/// Frozen order (design spec "Focus, Mouse, and Keyboard Routing"):
/// topmost blocking overlay -> active capture -> z-ordered topmost hit ->
/// focused component -> page/template shortcut -> global shortcuts ->
/// fail-closed consumption so hidden background surfaces never act.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ShellInputOwner {
    TopmostOverlay,
    ActiveCapture,
    TopmostHit(RegionId),
    FocusedComponent,
    PageShortcut,
    GlobalShortcut,
    FailClosed,
}

/// Ownership facts one event resolves against. The context is a pure
/// projection of current state: building it performs no mutation, and the
/// positional hit must come from the exact current `ShellView` generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ShellInputRouteContext {
    pub(crate) topmost_overlay: bool,
    pub(crate) active_capture: bool,
    pub(crate) topmost_hit: Option<RegionId>,
    pub(crate) focused_component: bool,
    pub(crate) page_shortcut: bool,
    pub(crate) global_shortcut: bool,
}

/// Resolve exactly one input owner from the frozen precedence. Total by
/// construction: every context maps to one owner and the empty context fails
/// closed instead of leaking to a hidden background surface.
pub(crate) fn route_shell_input(context: ShellInputRouteContext) -> ShellInputOwner {
    if context.topmost_overlay {
        return ShellInputOwner::TopmostOverlay;
    }
    if context.active_capture {
        return ShellInputOwner::ActiveCapture;
    }
    if let Some(target) = context.topmost_hit {
        return ShellInputOwner::TopmostHit(target);
    }
    if context.focused_component {
        return ShellInputOwner::FocusedComponent;
    }
    if context.page_shortcut {
        return ShellInputOwner::PageShortcut;
    }
    if context.global_shortcut {
        return ShellInputOwner::GlobalShortcut;
    }
    ShellInputOwner::FailClosed
}

impl AppState {
    /// Project current keyboard ownership into the frozen router. Keyboard
    /// events carry no position, so the hit tier stays empty; v0 has no
    /// page/template shortcut owner yet, so remaining keys belong to the
    /// global application dispatch.
    pub(crate) fn shell_key_input_owner(&self) -> ShellInputOwner {
        route_shell_input(ShellInputRouteContext {
            topmost_overlay: self.blocking_overlay_active(),
            active_capture: self.shell_resize_active(),
            topmost_hit: None,
            focused_component: self.file_manager.is_some(),
            page_shortcut: false,
            global_shortcut: true,
        })
    }

    /// Project current mouse ownership into the frozen router. v0 populates
    /// the overlay tier only: the capture, positional-hit, and
    /// focused-component tiers arrive with the later SF4.2 capture and
    /// stale-generation slices, so unrouted events stay with the existing
    /// mode-guarded global dispatch.
    pub(crate) fn shell_mouse_input_owner(&self) -> ShellInputOwner {
        route_shell_input(ShellInputRouteContext {
            topmost_overlay: self.blocking_overlay_active(),
            active_capture: false,
            topmost_hit: None,
            focused_component: false,
            page_shortcut: false,
            global_shortcut: true,
        })
    }

    /// Enter a blocking overlay while remembering the current non-default
    /// focus owner (`Resize`/`Copy`) so `leave_modal` can restore it. An
    /// overlay-to-overlay transition preserves the original remembered owner;
    /// entering from a default owner clears any stale value by construction.
    pub(crate) fn enter_overlay_mode(&mut self, overlay: Mode) {
        if !self.blocking_overlay_active() {
            self.overlay_return_mode =
                matches!(self.mode, Mode::Resize | Mode::Copy).then_some(self.mode);
        }
        self.mode = overlay;
    }

    /// Every mode whose surface is a topmost blocking overlay for mouse and
    /// keyboard routing. The match is exhaustive so a new mode must choose a
    /// side explicitly instead of silently leaking background input.
    fn blocking_overlay_active(&self) -> bool {
        match self.mode {
            Mode::Terminal | Mode::Prefix | Mode::Navigate | Mode::Copy | Mode::Resize => false,
            Mode::Onboarding
            | Mode::ReleaseNotes
            | Mode::ProductAnnouncement
            | Mode::AttachFile
            | Mode::RenameWorkspace
            | Mode::RenameTab
            | Mode::RenamePane
            | Mode::RenameFile
            | Mode::NewLinkedWorktree
            | Mode::OpenExistingWorktree
            | Mode::ConfirmRemoveWorktree
            | Mode::ConfirmClose
            | Mode::ConfirmFileDelete
            | Mode::ContextMenu
            | Mode::Settings
            | Mode::GlobalMenu
            | Mode::KeybindHelp
            | Mode::Navigator => true,
        }
    }

    pub(crate) fn begin_sidebar_resize(&mut self, pointer: Position) -> bool {
        let Some(total) = self.current_sidebar_resize_total() else {
            return false;
        };
        let Some(original_tracks) = self.sidebar_resize_tracks(total) else {
            return false;
        };
        let Some(divider) = DividerId::new(
            RegionId::LeftPanel,
            RegionId::WorkspaceStage,
            ShellDirection::Horizontal,
        ) else {
            return false;
        };
        let Some(transaction) = ResizeTransaction::begin(
            divider,
            self.view.shell.generation,
            pointer,
            original_tracks,
        ) else {
            return false;
        };

        self.shell_interaction.begin_resize(transaction);
        self.drag = Some(DragState {
            target: DragTarget::SidebarDivider,
        });
        true
    }

    pub(crate) fn preview_sidebar_resize(&mut self, pointer: Position) -> bool {
        let Some(total) = self.shell_interaction.resize_original_total() else {
            return false;
        };
        let Some(bounds) = self.sidebar_resize_bounds(total) else {
            return false;
        };
        self.shell_interaction.preview_resize(pointer, bounds)
    }

    pub(crate) fn handle_shell_resize_key(&mut self, key: KeyEvent) -> bool {
        if !self.shell_resize_active() {
            return false;
        }

        match key.code {
            KeyCode::Right | KeyCode::Char('l') => self.preview_sidebar_resize_step(1),
            KeyCode::Left | KeyCode::Char('h') => self.preview_sidebar_resize_step(-1),
            KeyCode::Enter => {
                self.commit_sidebar_resize();
                self.clear_sidebar_resize_drag();
            }
            KeyCode::Esc => self.cancel_sidebar_resize(),
            _ => {}
        }
        true
    }

    pub(crate) fn set_sidebar_collapsed(&mut self, collapsed: bool) -> bool {
        let update = if collapsed {
            self.shell_presentation
                .collapse_left_panel(self.sidebar_width)
        } else {
            let Some(total) = self.current_sidebar_resize_total() else {
                return false;
            };
            let Some(bounds) = self.sidebar_resize_bounds(total) else {
                return false;
            };
            self.shell_presentation.expand_left_panel(total, bounds)
        };

        match update.decision() {
            CollapseDecision::Inert => return false,
            CollapseDecision::Collapsed { .. } => {
                self.sidebar_collapsed = true;
            }
            CollapseDecision::Expanded { width } => {
                self.sidebar_collapsed = false;
                self.sidebar_width = width;
                self.sidebar_width_source = SidebarWidthSource::Manual;
                self.sidebar_width_auto = false;
            }
        }
        if update.marks_persistence_dirty() {
            self.mark_session_dirty();
        }
        true
    }

    #[cfg(test)]
    fn sidebar_collapse_snapshot_for_test(&self) -> (u16, u64) {
        (
            self.shell_presentation.left_panel_restore_width(),
            self.shell_presentation.left_panel_collapse_revision(),
        )
    }

    pub(crate) fn commit_sidebar_resize(&mut self) {
        let generation = self
            .shell_interaction
            .resize_generation()
            .unwrap_or(self.view.shell.generation);
        let update = self.shell_interaction.commit_resize(generation);
        self.apply_sidebar_resize_update(update, SidebarWidthSource::Manual);
    }

    fn cancel_sidebar_resize(&mut self) {
        let update = self.shell_interaction.cancel_resize();
        debug_assert!(!update.marks_persistence_dirty());
        debug_assert!(!update.requests_pty_resize());
        self.clear_sidebar_resize_drag();
    }

    pub(crate) fn reset_sidebar_resize_to_preferred(&mut self) {
        let _ = self.shell_interaction.cancel_resize();
        self.clear_sidebar_resize_drag();

        let Some(total) = self.current_sidebar_resize_total() else {
            return;
        };
        let Some(current) = self.sidebar_resize_tracks(total) else {
            return;
        };
        let Some(bounds) = self.sidebar_resize_bounds(total) else {
            return;
        };
        let update =
            ResizeTransaction::reset_preferred(current, self.default_sidebar_width, bounds);
        self.apply_sidebar_resize_update(update, SidebarWidthSource::ConfigDefault);
    }

    pub(crate) fn cancel_sidebar_resize_for_terminal_area(&mut self, new_total: u16) {
        let update = if let Some(bounds) = self.sidebar_resize_bounds(new_total) {
            self.shell_interaction.terminal_resize(new_total, bounds)
        } else {
            self.shell_interaction.cancel_resize()
        };
        debug_assert!(!update.marks_persistence_dirty());
        debug_assert!(!update.requests_pty_resize());
        self.clear_sidebar_resize_drag();
    }

    pub(crate) fn rebase_sidebar_resize_generation(&mut self, generation: u64) {
        self.shell_interaction.rebase_resize_generation(generation);
    }

    pub(crate) fn shell_resize_active(&self) -> bool {
        self.shell_interaction.resize_active()
    }

    pub(crate) fn shell_resize_preview_width(&self) -> Option<u16> {
        self.shell_interaction
            .resize_preview_tracks()
            .map(|tracks| tracks[0])
    }

    pub(crate) fn shell_resize_original_total(&self) -> Option<u16> {
        self.shell_interaction.resize_original_total()
    }

    fn current_sidebar_resize_total(&self) -> Option<u16> {
        if self.view.shell.area.width > 0 {
            Some(self.view.shell.area.width)
        } else {
            self.view
                .sidebar_rect
                .width
                .checked_add(self.view.terminal_area.width)
        }
    }

    fn sidebar_resize_tracks(&self, total: u16) -> Option<[u16; 2]> {
        let leading = self
            .sidebar_width
            .clamp(self.sidebar_min_width, self.sidebar_max_width);
        let trailing = total.checked_sub(leading)?;
        Some([leading, trailing])
    }

    fn sidebar_resize_bounds(&self, total: u16) -> Option<ResizeBounds> {
        ResizeBounds::new(self.sidebar_min_width, self.sidebar_max_width, 1, total)
    }

    fn preview_sidebar_resize_step(&mut self, step: i16) {
        let Some(total) = self.shell_interaction.resize_original_total() else {
            return;
        };
        let Some(bounds) = self.sidebar_resize_bounds(total) else {
            return;
        };
        self.shell_interaction
            .preview_keyboard_resize_step(step, bounds);
    }

    fn apply_sidebar_resize_update(&mut self, update: ResizeUpdate, source: SidebarWidthSource) {
        if let ResizeDecision::Committed([leading, _]) = update.decision() {
            self.sidebar_width = leading;
            self.sidebar_width_source = source;
            self.sidebar_width_auto = false;
        }
        if update.marks_persistence_dirty() {
            self.mark_session_dirty();
        }
        // Clearing preview capture makes the next committed compute_view frame
        // the single high-level resize request represented by this flag.
        debug_assert_eq!(
            update.requests_pty_resize(),
            matches!(update.decision(), ResizeDecision::Committed(_))
        );
    }

    fn clear_sidebar_resize_drag(&mut self) {
        if matches!(
            self.drag.as_ref().map(|drag| &drag.target),
            Some(DragTarget::SidebarDivider)
        ) {
            self.drag = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ratatui::layout::{Position, Rect};

    use super::*;

    #[test]
    fn active_sidebar_capture_right_arrow_previews_one_cell() {
        let mut state = state_with_sidebar_capture();

        let handled = handle_shell_resize_key_for_test(
            &mut state,
            KeyEvent::new(KeyCode::Right, KeyModifiers::NONE),
        );

        assert_eq!(
            (
                handled,
                state.shell_resize_preview_width(),
                state.sidebar_width,
                state.session_dirty,
            ),
            (true, Some(27), 26, false)
        );
    }

    #[test]
    fn repeated_keyboard_resize_accumulates_through_same_preview_path() {
        let mut state = state_with_sidebar_capture();
        let right = KeyEvent::new(KeyCode::Right, KeyModifiers::NONE);

        handle_shell_resize_key_for_test(&mut state, right);
        handle_shell_resize_key_for_test(&mut state, right);

        assert_eq!(state.shell_resize_preview_width(), Some(28));
        assert_eq!(state.sidebar_width, 26);
        assert!(!state.session_dirty);
    }

    #[test]
    fn active_sidebar_capture_enter_commits_preview() {
        let mut state = state_with_sidebar_capture();
        handle_shell_resize_key_for_test(
            &mut state,
            KeyEvent::new(KeyCode::Right, KeyModifiers::NONE),
        );
        state.session_dirty = false;

        let handled = handle_shell_resize_key_for_test(
            &mut state,
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
        );

        assert_eq!(
            (
                handled,
                state.sidebar_width,
                state.shell_resize_active(),
                state.session_dirty,
            ),
            (true, 27, false, true)
        );
    }

    #[test]
    fn active_sidebar_capture_escape_restores_original() {
        let mut state = state_with_sidebar_capture();
        handle_shell_resize_key_for_test(
            &mut state,
            KeyEvent::new(KeyCode::Right, KeyModifiers::NONE),
        );
        state.session_dirty = false;

        let handled = handle_shell_resize_key_for_test(
            &mut state,
            KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
        );

        assert_eq!(
            (
                handled,
                state.sidebar_width,
                state.shell_resize_active(),
                state.session_dirty,
            ),
            (true, 26, false, false)
        );
    }

    #[test]
    fn active_sidebar_capture_consumes_non_axis_key_inert() {
        let mut state = state_with_sidebar_capture();

        let handled = handle_shell_resize_key_for_test(
            &mut state,
            KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE),
        );

        assert_eq!(
            (
                handled,
                state.shell_resize_preview_width(),
                state.sidebar_width,
                state.session_dirty,
            ),
            (true, Some(26), 26, false)
        );
    }

    #[test]
    fn sidebar_collapse_adapter_remembers_committed_width_and_marks_dirty_once() {
        let mut state = state_for_sidebar_collapse();
        state.mode = super::super::super::state::Mode::Navigate;

        let changed = set_sidebar_collapsed_for_test(&mut state, true);

        assert_eq!(
            (
                changed,
                state.sidebar_collapsed,
                state.sidebar_width,
                sidebar_collapse_snapshot_for_test(&state),
                state.session_dirty,
                state.mode,
            ),
            (
                true,
                true,
                32,
                (32, 1),
                true,
                super::super::super::state::Mode::Navigate,
            )
        );
    }

    #[test]
    fn repeated_sidebar_collapse_intent_is_inert() {
        let mut state = state_for_sidebar_collapse();
        assert!(set_sidebar_collapsed_for_test(&mut state, true));
        state.session_dirty = false;

        let changed = set_sidebar_collapsed_for_test(&mut state, true);

        assert_eq!(
            (
                changed,
                sidebar_collapse_snapshot_for_test(&state),
                state.session_dirty,
            ),
            (false, (32, 1), false)
        );
    }

    #[test]
    fn sidebar_expand_adapter_clamps_restore_after_terminal_shrink() {
        let mut state = state_for_sidebar_collapse();
        assert!(set_sidebar_collapsed_for_test(&mut state, true));
        state.session_dirty = false;
        state.view.shell.area.width = 27;

        let changed = set_sidebar_collapsed_for_test(&mut state, false);

        assert_eq!(
            (
                changed,
                state.sidebar_collapsed,
                state.sidebar_width,
                sidebar_collapse_snapshot_for_test(&state),
                state.session_dirty,
            ),
            (true, false, 26, (26, 2), true)
        );
    }

    fn state_with_sidebar_capture() -> AppState {
        let mut state = AppState::test_new();
        crate::ui::compute_view(&mut state, Rect::new(0, 0, 106, 40));
        assert!(state.begin_sidebar_resize(Position::new(25, 5)));
        state.session_dirty = false;
        state
    }

    fn state_for_sidebar_collapse() -> AppState {
        let mut state = AppState::test_new();
        state.sidebar_width = 32;
        crate::ui::compute_view(&mut state, Rect::new(0, 0, 106, 40));
        state.session_dirty = false;
        state
    }

    fn set_sidebar_collapsed_for_test(state: &mut AppState, collapsed: bool) -> bool {
        state.set_sidebar_collapsed(collapsed)
    }

    fn sidebar_collapse_snapshot_for_test(state: &AppState) -> (u16, u64) {
        state.sidebar_collapse_snapshot_for_test()
    }

    fn handle_shell_resize_key_for_test(state: &mut AppState, key: KeyEvent) -> bool {
        state.handle_shell_resize_key(key)
    }

    // SF4.2-05: closing an overlay restores the previous VALID focus owner,
    // not blindly the Terminal/Navigate template fallback. The launcher is
    // explicitly enabled in Resize mode, so GlobalMenu-from-Resize is a real
    // user path whose close must return to the resize session.
    #[test]
    fn focus_restores_after_overlay_close() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let esc = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);

        // Control: a Terminal-origin overlay still falls back to Terminal.
        let mut state = AppState::test_new();
        state.workspaces = vec![crate::workspace::Workspace::test_new("one")];
        state.active = Some(0);
        state.selected = 0;
        state.mode = Mode::Terminal;
        super::super::modal::open_global_menu(&mut state);
        assert_eq!(state.mode, Mode::GlobalMenu);
        super::super::modal::handle_global_menu_key(&mut state, esc);
        assert_eq!(
            state.mode,
            Mode::Terminal,
            "control: a Terminal-origin overlay close keeps the fallback"
        );

        // A Resize-origin overlay close must restore the resize session.
        state.mode = Mode::Resize;
        super::super::modal::open_global_menu(&mut state);
        assert_eq!(state.mode, Mode::GlobalMenu);
        super::super::modal::handle_global_menu_key(&mut state, esc);
        assert_eq!(
            state.mode,
            Mode::Resize,
            "closing an overlay must restore the previous valid focus owner"
        );

        // A remembered owner that is no longer valid falls back instead.
        state.mode = Mode::Resize;
        super::super::modal::open_global_menu(&mut state);
        state.active = None;
        super::super::modal::handle_global_menu_key(&mut state, esc);
        assert_eq!(
            state.mode,
            Mode::Navigate,
            "an invalid remembered owner must fall back, never restore blindly"
        );
    }

    // SF4.2-04 characterization: an active divider capture already owns every
    // move/up event through `DragState`, independent of coordinates. This is
    // GREEN by intent (SF1 precedent): drag routing never re-resolves rects,
    // and a left-down clears any lingering selection before a capture can
    // begin, so no competing owner is reachable mid-gesture.
    #[test]
    fn capture_owns_move_and_up_outside_original_rect() {
        use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

        fn mouse_event(kind: MouseEventKind, column: u16, row: u16) -> MouseEvent {
            MouseEvent {
                kind,
                column,
                row,
                modifiers: crossterm::event::KeyModifiers::empty(),
            }
        }

        let mut state = AppState::test_new();
        state.workspaces = vec![
            crate::workspace::Workspace::test_new("one"),
            crate::workspace::Workspace::test_new("two"),
        ];
        state.active = Some(0);
        state.selected = 1;
        crate::ui::compute_view(&mut state, Rect::new(0, 0, 106, 40));
        state.session_dirty = false;
        let mut terminal_runtimes = crate::terminal::TerminalRuntimeRegistry::new();

        let _ = state.handle_mouse(
            &mut terminal_runtimes,
            mouse_event(MouseEventKind::Down(MouseButton::Left), 25, 5),
        );
        assert!(
            state.shell_resize_active(),
            "a divider left-down must begin the resize capture"
        );

        // Drag over the pane area, far outside the divider rect.
        let _ = state.handle_mouse(
            &mut terminal_runtimes,
            mouse_event(MouseEventKind::Drag(MouseButton::Left), 30, 15),
        );
        assert_eq!(state.shell_resize_preview_width(), Some(31));
        assert!(state.selection.is_none(), "capture drag must not select");

        // Drag over the sidebar workspace rows: no press, reorder, or
        // selection movement may start under the active capture.
        let _ = state.handle_mouse(
            &mut terminal_runtimes,
            mouse_event(MouseEventKind::Drag(MouseButton::Left), 5, 3),
        );
        assert_eq!(
            (state.shell_resize_preview_width(), state.selected),
            (Some(state.sidebar_min_width), 1)
        );

        // Drag to the far corner clamps at the bound and keeps ownership.
        let _ = state.handle_mouse(
            &mut terminal_runtimes,
            mouse_event(MouseEventKind::Drag(MouseButton::Left), 100, 35),
        );
        assert_eq!(
            state.shell_resize_preview_width(),
            Some(state.sidebar_max_width)
        );

        // Releasing outside the original rect commits exactly once.
        let _ = state.handle_mouse(
            &mut terminal_runtimes,
            mouse_event(MouseEventKind::Up(MouseButton::Left), 100, 35),
        );
        assert_eq!(
            (
                state.sidebar_width,
                state.shell_resize_active(),
                state.session_dirty,
                state.selected,
            ),
            (state.sidebar_max_width, false, true, 1)
        );
    }

    #[test]
    fn shell_input_router_follows_frozen_precedence() {
        struct PrecedenceRow {
            name: &'static str,
            context: ShellInputRouteContext,
            expected: ShellInputOwner,
        }

        let rows = [
            PrecedenceRow {
                name: "topmost blocking overlay owns input ahead of every lower tier",
                context: ShellInputRouteContext {
                    topmost_overlay: true,
                    active_capture: true,
                    topmost_hit: Some(RegionId::WorkspaceStage),
                    focused_component: true,
                    page_shortcut: true,
                    global_shortcut: true,
                },
                expected: ShellInputOwner::TopmostOverlay,
            },
            PrecedenceRow {
                name: "active capture owns input under an absent overlay",
                context: ShellInputRouteContext {
                    topmost_overlay: false,
                    active_capture: true,
                    topmost_hit: Some(RegionId::WorkspaceStage),
                    focused_component: true,
                    page_shortcut: true,
                    global_shortcut: true,
                },
                expected: ShellInputOwner::ActiveCapture,
            },
            PrecedenceRow {
                name: "resolved z-ordered topmost hit owns input under overlay and capture",
                context: ShellInputRouteContext {
                    topmost_overlay: false,
                    active_capture: false,
                    topmost_hit: Some(RegionId::LeftPanel),
                    focused_component: true,
                    page_shortcut: true,
                    global_shortcut: true,
                },
                expected: ShellInputOwner::TopmostHit(RegionId::LeftPanel),
            },
            PrecedenceRow {
                name: "focused component owns non-positional input without a hit",
                context: ShellInputRouteContext {
                    topmost_overlay: false,
                    active_capture: false,
                    topmost_hit: None,
                    focused_component: true,
                    page_shortcut: true,
                    global_shortcut: true,
                },
                expected: ShellInputOwner::FocusedComponent,
            },
            PrecedenceRow {
                name: "page shortcut owner precedes global shortcuts",
                context: ShellInputRouteContext {
                    topmost_overlay: false,
                    active_capture: false,
                    topmost_hit: None,
                    focused_component: false,
                    page_shortcut: true,
                    global_shortcut: true,
                },
                expected: ShellInputOwner::PageShortcut,
            },
            PrecedenceRow {
                name: "global application shortcuts are the last acting tier",
                context: ShellInputRouteContext {
                    topmost_overlay: false,
                    active_capture: false,
                    topmost_hit: None,
                    focused_component: false,
                    page_shortcut: false,
                    global_shortcut: true,
                },
                expected: ShellInputOwner::GlobalShortcut,
            },
            PrecedenceRow {
                name: "input with no owner fails closed instead of reaching hidden surfaces",
                context: ShellInputRouteContext {
                    topmost_overlay: false,
                    active_capture: false,
                    topmost_hit: None,
                    focused_component: false,
                    page_shortcut: false,
                    global_shortcut: false,
                },
                expected: ShellInputOwner::FailClosed,
            },
        ];

        for row in rows {
            assert_eq!(
                route_shell_input(row.context),
                row.expected,
                "frozen precedence row: {}",
                row.name
            );
        }
    }
}
