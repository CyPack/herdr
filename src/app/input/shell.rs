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
    /// global application dispatch. The focused component derives from the
    /// TYPED stage authority AND live Files domain state, so a divergent
    /// legacy boolean can never grant keyboard focus to a hidden surface.
    pub(crate) fn shell_key_input_owner(&self) -> ShellInputOwner {
        let files_surface_focused = self.stage.surface_view()
            == crate::ui::surface_host::StageSurfaceView::NativeFiles
            && self.file_manager.is_some();
        route_shell_input(ShellInputRouteContext {
            topmost_overlay: self.blocking_overlay_active(),
            active_capture: self.shell_interaction.resize_active(),
            topmost_hit: None,
            focused_component: files_surface_focused,
            page_shortcut: false,
            global_shortcut: true,
        })
    }

    /// Project current mouse ownership into the frozen router. The overlay
    /// tier and the positional-hit tier are live: the hit resolves ONLY
    /// against the exact current `ShellView` generation, so coordinates from
    /// vanished geometry re-resolve to their current owner. Mouse captures
    /// stay routed through `DragState` (frozen by the SF4.2-04
    /// characterization), and the focused-component tier arrives with the
    /// SF4.2-08 hidden-terminal slice, so unrouted events remain with the
    /// existing mode-guarded global dispatch.
    pub(crate) fn shell_mouse_input_owner(&self, position: Position) -> ShellInputOwner {
        route_shell_input(ShellInputRouteContext {
            topmost_overlay: self.blocking_overlay_active(),
            active_capture: false,
            topmost_hit: self
                .view
                .shell
                .region_hit_at(self.view.shell.generation, position),
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
    pub(crate) fn blocking_overlay_active(&self) -> bool {
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
            | Mode::Navigator
            | Mode::AgentReferencePicker => true,
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

    pub(crate) fn cancel_miller_resize_for_terminal_area(&mut self, new_total: u16) {
        let previous_total = self.view.shell.area.width;
        if previous_total == 0
            || previous_total == new_total
            || !self.shell_interaction.miller_resize_active()
        {
            return;
        }
        let update = self.shell_interaction.cancel_resize();
        debug_assert!(!update.marks_persistence_dirty());
        debug_assert!(!update.requests_pty_resize());
    }

    pub(crate) fn rebase_sidebar_resize_generation(&mut self, generation: u64) {
        self.shell_interaction.rebase_resize_generation(generation);
    }

    pub(crate) fn shell_resize_active(&self) -> bool {
        self.shell_interaction.shell_resize_active()
    }

    pub(crate) fn shell_resize_preview_width(&self) -> Option<u16> {
        self.shell_interaction
            .shell_resize_preview_tracks()
            .map(|tracks| tracks[0])
    }

    pub(crate) fn shell_resize_original_total(&self) -> Option<u16> {
        self.shell_interaction.shell_resize_original_total()
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
            crate::render_prof::event("shell.persistence_write");
            self.mark_session_dirty();
        }
        if update.requests_pty_resize() {
            crate::render_prof::event("shell.pty_resize_request");
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

    // SF6.3 contract: one hundred pointer preview moves inside a single
    // resize transaction produce ZERO persistence writes and leave the
    // committed width untouched; exactly the commit marks persistence once.
    // PTY resize purity is structural and separately frozen: preview returns
    // no `ResizeUpdate` by type, and `resize_panes_during_shell_preview`
    // suppresses pane resizing for the whole preview window.
    #[test]
    fn hundred_preview_moves_produce_no_persistence_or_pty_effects() {
        let mut state = state_with_sidebar_capture();

        for step in 0..100u16 {
            state.preview_sidebar_resize(Position::new(20 + (step % 10), 5));
        }
        assert!(
            !state.session_dirty,
            "one hundred preview moves must write no persistence"
        );
        assert_eq!(
            state.sidebar_width, 26,
            "the committed width is untouched during preview"
        );
        assert!(
            state.shell_resize_preview_width().is_some(),
            "the transaction is still live after one hundred moves"
        );

        let handled = handle_shell_resize_key_for_test(
            &mut state,
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
        );
        assert!(handled);
        assert!(
            state.session_dirty,
            "exactly the commit marks persistence dirty"
        );
        assert!(state.shell_resize_preview_width().is_none());
    }

    #[test]
    fn resize_profile_counts_only_committed_persistence_and_pty_requests() {
        let mut state = state_with_sidebar_capture();

        let (_, profile) = crate::render_prof::observe_for_test(|| {
            for step in 0..100_u16 {
                state.preview_sidebar_resize(Position::new(20 + (step % 10), 5));
            }
            state.commit_sidebar_resize();
        });

        assert_eq!(profile.counter("shell.persistence_write"), 1);
        assert_eq!(profile.counter("shell.pty_resize_request"), 1);
        assert!(
            state.session_dirty,
            "the persistence counter corresponds to the debounced dirty request"
        );
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

        // A ContextMenu opened from Copy mode (the right-click arms carry no
        // mode guard) must restore the live copy session instead of stranding
        // `copy_mode: Some` under `Mode::Terminal`.
        state.active = Some(0);
        state.selected = 0;
        let pane_id = state.workspaces[0]
            .focused_pane_id()
            .expect("test workspace exposes a focused pane");
        state.copy_mode = Some(crate::app::state::CopyModeState {
            pane_id,
            cursor_row: 0,
            cursor_col: 0,
            entry_offset_from_bottom: 0,
            selection: None,
            search: crate::app::state::CopyModeSearchState::default(),
        });
        state.mode = Mode::Copy;
        state.open_project_new_chat_menu(0, 4, 4);
        assert_eq!(state.mode, Mode::ContextMenu);
        let mut terminal_runtimes = crate::terminal::TerminalRuntimeRegistry::new();
        super::super::modal::handle_context_menu_key(&mut state, &mut terminal_runtimes, esc);
        assert_eq!(
            state.mode,
            Mode::Copy,
            "closing a context menu must restore the live copy session"
        );
        assert!(
            state.copy_mode.is_some(),
            "the copy session itself must survive the overlay episode"
        );
    }

    // SF4.2-07: the mouse context builder must resolve the z-ordered topmost
    // hit from the exact current `ShellView` generation, so a position is
    // authority only against live geometry — old coordinates re-resolve to
    // their CURRENT owner after every geometry change and never grant a
    // vanished region's authority.
    #[test]
    fn stale_hit_generation_fails_closed() {
        let layout = crate::ui::shell::ShellLayout::default();
        let area = ratatui::layout::Rect::new(0, 0, 80, 24);
        let sidebar_position = Position::new(5, 5);

        let mut state = AppState::test_new();
        state.view.shell = crate::ui::shell::compute_shell_view(
            &layout,
            crate::ui::shell::ShellGeometryKey::new(area, 0, 26, 0),
            crate::ui::shell::ShellView::default(),
            &|region| u16::from(region == RegionId::LeftPanel) * 26,
        );

        // A current-generation position inside the sidebar is owned by the
        // hit tier.
        assert_eq!(
            shell_mouse_owner_at(&state, sidebar_position),
            ShellInputOwner::TopmostHit(RegionId::LeftPanel),
            "a live sidebar position must resolve through the hit tier"
        );

        // A blocking overlay outranks every positional hit.
        state.mode = Mode::GlobalMenu;
        assert_eq!(
            shell_mouse_owner_at(&state, sidebar_position),
            ShellInputOwner::TopmostOverlay,
            "the hit tier must never outrank a blocking overlay"
        );
        state.mode = Mode::Terminal;

        // After a geometry change the SAME coordinates belong to the current
        // owner; the vanished sidebar authority is gone with its generation.
        let previous = std::mem::take(&mut state.view.shell);
        state.view.shell = crate::ui::shell::compute_shell_view(
            &layout,
            crate::ui::shell::ShellGeometryKey::new(area, 0, 4, 0),
            previous,
            &|region| u16::from(region == RegionId::LeftPanel) * 4,
        );
        assert_eq!(
            shell_mouse_owner_at(&state, sidebar_position),
            ShellInputOwner::TopmostHit(RegionId::WorkspaceStage),
            "old coordinates must re-resolve to their current owner"
        );

        // Outside every live region the hit tier stays silent.
        assert_eq!(
            shell_mouse_owner_at(&state, Position::new(100, 100)),
            ShellInputOwner::GlobalShortcut,
            "a positionless miss must fall through, never invent a hit"
        );
    }

    fn shell_mouse_owner_at(state: &AppState, position: Position) -> ShellInputOwner {
        state.shell_mouse_input_owner(position)
    }

    // SF5.2 characterization: dock resize reuses the SAME region-generic SF3
    // `ResizeTransaction` with the dock's frozen 3..=9 track bounds — no
    // dock-specific drag state exists. Valid RED was refuted by source: the
    // reducer is generic over `DividerId` region pairs by construction.
    #[test]
    fn dock_resize_and_collapse_use_shared_transaction() {
        let divider = DividerId::new(
            RegionId::AppDock,
            RegionId::WorkspaceStage,
            ShellDirection::Horizontal,
        )
        .expect("dock divider");
        let bounds = ResizeBounds::new(3, 9, 1, 80).expect("dock bounds");

        // Growing far beyond the maximum clamps to the frozen 9-cell cap.
        let mut transaction = ResizeTransaction::begin(divider, 7, Position::new(5, 3), [5, 75]);
        let tx = transaction.as_mut().expect("dock transaction");
        assert!(tx.preview(Position::new(200, 3), bounds));
        let update = ResizeTransaction::commit(&mut transaction, 7);
        assert_eq!(
            update.decision(),
            ResizeDecision::Committed([9, 71]),
            "the shared transaction clamps the dock to its maximum"
        );

        // Shrinking below the minimum clamps to the frozen 3-cell floor.
        let mut transaction = ResizeTransaction::begin(divider, 7, Position::new(5, 3), [5, 75]);
        let tx = transaction.as_mut().expect("dock transaction");
        assert!(tx.preview(Position::new(0, 3), bounds));
        let update = ResizeTransaction::commit(&mut transaction, 7);
        assert_eq!(update.decision(), ResizeDecision::Committed([3, 77]));

        // A stale view generation stays inert — the same guard every shell
        // divider already obeys.
        let mut transaction = ResizeTransaction::begin(divider, 7, Position::new(5, 3), [5, 75]);
        let tx = transaction.as_mut().expect("dock transaction");
        assert!(tx.preview(Position::new(200, 3), bounds));
        let update = ResizeTransaction::commit(&mut transaction, 8);
        assert_eq!(update.decision(), ResizeDecision::Inert);
    }

    // SF4.2-06 companion characterization: the collapsed-sidebar guard inside
    // `on_sidebar_divider` is load-bearing but was previously unpinned. The
    // adversarial fixture keeps a stale non-zero sidebar rect in the view so
    // ONLY the collapse guard stands between hidden geometry and a resize
    // capture; deleting that guard must fail this test.
    #[test]
    fn collapsed_sidebar_exposes_no_divider_capture() {
        let mut state = AppState::test_new();
        state.view.sidebar_rect = ratatui::layout::Rect::new(0, 0, 26, 24);
        let divider_col = 25;

        state.sidebar_collapsed = false;
        assert!(
            state.on_sidebar_divider(divider_col, 5),
            "control: the probe must hit the live divider column"
        );

        state.sidebar_collapsed = true;
        assert!(
            !state.on_sidebar_divider(divider_col, 5),
            "a collapsed sidebar must never expose divider capture authority"
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
