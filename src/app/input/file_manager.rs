//! Native file manager — navigation and tagged action input (A3/C1/C2).
//!
//! While the file manager is open it captures keyboard input (intercepted in
//! `handle_key` before the mode dispatch), driving the cursor and directory
//! navigation on `AppState.file_manager`. Client-side presentation input; keys
//! that it does not use are swallowed so they never reach the hidden terminal.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

use crate::app::state::{AppState, FileManagerHeaderAction, FileManagerRowAction};
use crate::app::{App, FileManagerClickState};

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

/// Handle one key while the file manager is open. `Esc`/`q` close it; the arrow
/// keys and `hjkl` move the cursor or navigate directories; `.` toggles hidden
/// files. Any other key is a no-op (swallowed).
pub(super) fn handle_file_manager_key(state: &mut AppState, key: KeyEvent) {
    match (key.code, key.modifiers) {
        (KeyCode::Esc | KeyCode::Char('q'), _) => {
            state.file_manager = None;
        }
        (KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J'), KeyModifiers::SHIFT) => {
            if let Some(fm) = state.file_manager.as_mut() {
                fm.move_down();
                let cursor = fm.cursor;
                fm.extend_selection(cursor);
            }
        }
        (KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K'), KeyModifiers::SHIFT) => {
            if let Some(fm) = state.file_manager.as_mut() {
                fm.move_up();
                let cursor = fm.cursor;
                fm.extend_selection(cursor);
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
            if let Some(fm) = state.file_manager.as_mut() {
                fm.enter();
            }
        }
        (KeyCode::Backspace | KeyCode::Left | KeyCode::Char('h'), KeyModifiers::NONE) => {
            if let Some(fm) = state.file_manager.as_mut() {
                fm.leave();
            }
        }
        (KeyCode::Char('.'), KeyModifiers::NONE) => {
            if let Some(fm) = state.file_manager.as_mut() {
                fm.toggle_hidden();
            }
        }
        _ => {}
    }
}

impl App {
    /// Route native-FM center-content mouse input before the hidden terminal
    /// pane path. Row actions carry stable path identity but remain side-effect
    /// free until their operation modules provide explicit execution authority.
    pub(super) fn handle_file_manager_mouse(
        &mut self,
        mouse: MouseEvent,
    ) -> FileManagerMouseDispatch {
        if self.state.file_manager.is_none() {
            self.last_file_manager_click = None;
            return FileManagerMouseDispatch::NotHandled;
        }

        let center = self.state.view.terminal_area;
        let in_center = rect_contains(center, mouse.column, mouse.row);
        if !in_center {
            if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
                self.last_file_manager_click = None;
            }
            return FileManagerMouseDispatch::NotHandled;
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

        let entry_target = self
            .state
            .view
            .file_manager_row_areas
            .iter()
            .find(|row| rect_contains(row.rect, mouse.column, mouse.row))
            .and_then(|row| {
                let entry = self
                    .state
                    .file_manager
                    .as_ref()
                    .and_then(|file_manager| file_manager.entries.get(row.entry_idx))?;
                (entry.path == row.entry_path).then(|| (row.entry_idx, row.entry_path.clone()))
            });
        let entry_idx = entry_target.as_ref().map(|(entry_idx, _)| *entry_idx);

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

                if let Some(file_manager) = self.state.file_manager.as_mut() {
                    if file_manager.replace_selection(entry_idx) && is_double_click {
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
        FileManagerActionBarModel, FileManagerActionDisabledReason, FileManagerActionState,
        FileManagerHeaderAction, FileManagerHeaderActionArea, FileManagerRowAction,
        FileManagerRowActionArea, FileManagerRowArea,
    };
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

    fn app_with_fm(fm: FmState) -> AppState {
        let mut app = AppState::test_new();
        app.file_manager = Some(fm);
        app
    }

    fn runtime_app_with_fm(fm: FmState) -> crate::app::App {
        let mut app = super::super::app_for_mouse_test();
        app.state.file_manager = Some(fm);
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
        app.state.file_manager = Some(FmState::new(&td.root));
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

    // TP-N4.1-SELECTION-STATE: a stale row snapshot and unrecognized modifier
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
        app.state.view.file_manager_row_areas[1].entry_idx = usize::MAX;
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

    // TP-C2.2-NON-TARGETS: the name rectangle preserves selection, while
    // gaps, hidden actions, non-left presses, modifiers, and stale closed-FM
    // geometry cannot invent a row action.
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
            mouse(MouseEventKind::Down(MouseButton::Right), 43, 3),
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
}
