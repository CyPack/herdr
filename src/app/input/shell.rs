use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Position;

use super::super::state::{AppState, DragState, DragTarget, SidebarWidthSource};
use crate::ui::shell::{
    DividerId, RegionId, ResizeBounds, ResizeDecision, ResizeTransaction, ResizeUpdate,
    ShellDirection,
};

impl AppState {
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

    fn state_with_sidebar_capture() -> AppState {
        let mut state = AppState::test_new();
        crate::ui::compute_view(&mut state, Rect::new(0, 0, 106, 40));
        assert!(state.begin_sidebar_resize(Position::new(25, 5)));
        state.session_dirty = false;
        state
    }

    fn handle_shell_resize_key_for_test(state: &mut AppState, key: KeyEvent) -> bool {
        state.handle_shell_resize_key(key)
    }
}
