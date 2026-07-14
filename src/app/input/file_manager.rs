//! Native file manager — navigation input (A3).
//!
//! While the file manager is open it captures keyboard input (intercepted in
//! `handle_key` before the mode dispatch), driving the cursor and directory
//! navigation on `AppState.file_manager`. Client-side presentation input; keys
//! that it does not use are swallowed so they never reach the hidden terminal.

use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};

use crate::app::state::AppState;
use crate::app::{App, FileManagerClickState};

/// Handle one key while the file manager is open. `Esc`/`q` close it; the arrow
/// keys and `hjkl` move the cursor or navigate directories; `.` toggles hidden
/// files. Any other key is a no-op (swallowed).
pub(super) fn handle_file_manager_key(state: &mut AppState, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => {
            state.file_manager = None;
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if let Some(fm) = state.file_manager.as_mut() {
                fm.move_down();
            }
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if let Some(fm) = state.file_manager.as_mut() {
                fm.move_up();
            }
        }
        KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
            if let Some(fm) = state.file_manager.as_mut() {
                fm.enter();
            }
        }
        KeyCode::Backspace | KeyCode::Left | KeyCode::Char('h') => {
            if let Some(fm) = state.file_manager.as_mut() {
                fm.leave();
            }
        }
        KeyCode::Char('.') => {
            if let Some(fm) = state.file_manager.as_mut() {
                fm.toggle_hidden();
            }
        }
        _ => {}
    }
}

impl App {
    /// Route native-FM center-content mouse input before the hidden terminal
    /// pane path. Returns true whenever the FM owns the event's screen area.
    pub(super) fn handle_file_manager_mouse(&mut self, mouse: MouseEvent) -> bool {
        if self.state.file_manager.is_none() {
            self.last_file_manager_click = None;
            return false;
        }

        let center = self.state.view.terminal_area;
        let in_center = rect_contains(center, mouse.column, mouse.row);
        if !in_center {
            if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
                self.last_file_manager_click = None;
            }
            return false;
        }

        let entry_idx = self
            .state
            .view
            .file_manager_row_areas
            .iter()
            .find(|row| rect_contains(row.rect, mouse.column, mouse.row))
            .map(|row| row.entry_idx);

        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) if mouse.modifiers.is_empty() => {
                let Some(entry_idx) = entry_idx else {
                    self.last_file_manager_click = None;
                    return true;
                };
                let entry_path = self
                    .state
                    .file_manager
                    .as_ref()
                    .and_then(|file_manager| file_manager.entries.get(entry_idx))
                    .map(|entry| entry.path.clone());
                let Some(entry_path) = entry_path else {
                    self.last_file_manager_click = None;
                    return true;
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

                if let Some(file_manager) = self.state.file_manager.as_mut() {
                    if file_manager.select(entry_idx) && is_double_click {
                        file_manager.enter();
                    }
                }
            }
            MouseEventKind::ScrollUp if entry_idx.is_some() => {
                self.last_file_manager_click = None;
                if let Some(file_manager) = self.state.file_manager.as_mut() {
                    file_manager.move_up();
                }
            }
            MouseEventKind::ScrollDown if entry_idx.is_some() => {
                self.last_file_manager_click = None;
                if let Some(file_manager) = self.state.file_manager.as_mut() {
                    file_manager.move_down();
                }
            }
            MouseEventKind::Down(MouseButton::Left) => {
                self.last_file_manager_click = None;
            }
            _ => {}
        }

        true
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
        FileManagerHeaderAction, FileManagerHeaderActionArea, FileManagerRowArea,
    };
    use crate::fm::FmState;
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

    fn app_with_fm(fm: FmState) -> AppState {
        let mut app = AppState::test_new();
        app.file_manager = Some(fm);
        app
    }

    fn runtime_app_with_fm(fm: FmState) -> crate::app::App {
        let mut app = super::super::app_for_mouse_test();
        app.state.file_manager = Some(fm);
        app.state.view.terminal_area = Rect::new(26, 0, 20, 6);
        let entry_count = app
            .state
            .file_manager
            .as_ref()
            .map_or(0, |file_manager| file_manager.entries.len());
        app.state.view.file_manager_row_areas = (0..entry_count.min(4))
            .map(|entry_idx| FileManagerRowArea {
                rect: Rect::new(26, 2 + entry_idx as u16, 20, 1),
                entry_idx,
            })
            .collect();
        app
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

        handle_file_manager_key(&mut app, key(KeyCode::Enter));
        assert_eq!(
            app.file_manager.as_ref().unwrap().cwd,
            td.root.join("sub"),
            "enter descends into the directory"
        );

        handle_file_manager_key(&mut app, key(KeyCode::Backspace));
        assert_eq!(
            app.file_manager.as_ref().unwrap().cwd,
            td.root,
            "backspace returns to the parent"
        );
    }

    // TP-A3.6b: '.' toggles hidden-file visibility.
    #[test]
    fn dot_toggles_hidden() {
        let td = TempDir::new("hidden");
        td.file("shown");
        td.file(".secret");
        let mut app = app_with_fm(FmState::new(&td.root));
        assert_eq!(app.file_manager.as_ref().unwrap().entries.len(), 1);

        handle_file_manager_key(&mut app, key(KeyCode::Char('.')));
        assert_eq!(app.file_manager.as_ref().unwrap().entries.len(), 2);
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

    // TP-A3.4-SCOPE: v1 mouse and keyboard navigation move one cursor-owned
    // visual selection. Closing and reopening starts a fresh cursor at row 0;
    // no multi-select collection survives or is speculatively introduced.
    #[test]
    fn cursor_only_selection_follows_mouse_keyboard_and_reopen() {
        let td = TempDir::new("cursor-only-scope");
        for index in 0..3 {
            td.file(&format!("{index:02}.txt"));
        }
        let mut app = runtime_app_with_fm(FmState::new(&td.root));

        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 27, 4));
        assert_eq!(app.state.file_manager.as_ref().expect("open fm").cursor, 2);

        handle_file_manager_key(&mut app.state, key(KeyCode::Up));
        assert_eq!(app.state.file_manager.as_ref().expect("open fm").cursor, 1);

        handle_file_manager_key(&mut app.state, key(KeyCode::Esc));
        assert!(app.state.file_manager.is_none());
        app.state.file_manager = Some(FmState::new(&td.root));
        assert_eq!(
            app.state.file_manager.as_ref().expect("reopened fm").cursor,
            0
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
}
