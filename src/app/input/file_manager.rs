//! Native file manager — navigation input (A3).
//!
//! While the file manager is open it captures keyboard input (intercepted in
//! `handle_key` before the mode dispatch), driving the cursor and directory
//! navigation on `AppState.file_manager`. Client-side presentation input; keys
//! that it does not use are swallowed so they never reach the hidden terminal.

use crossterm::event::{KeyCode, KeyEvent};

use crate::app::state::AppState;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::FileManagerRowArea;
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
}
