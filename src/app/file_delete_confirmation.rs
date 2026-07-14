#[cfg(test)]
mod tests {
    use crate::app::state::{
        FileManagerContextActionIntent, FileManagerContextMenuAction,
        FileManagerDeleteConfirmationStage, FileManagerDeleteKind, FileManagerHeaderAction,
        FileManagerOperationKind, FileManagerOperationState, FileManagerOperationStatus,
    };
    use crate::app::Mode;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    struct TempDir {
        root: PathBuf,
    }

    impl TempDir {
        fn new(tag: &str) -> Self {
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            let root = std::env::temp_dir().join(format!(
                "herdr-fm-delete-confirmation-test-{}-{tag}-{}",
                std::process::id(),
                COUNTER.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&root).expect("create delete confirmation fixture root");
            Self { root }
        }

        fn file(&self, name: &str, content: &[u8]) -> PathBuf {
            let path = self.root.join(name);
            fs::write(&path, content).expect("write delete confirmation fixture");
            path
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn test_app(root: &std::path::Path) -> crate::app::App {
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = crate::app::App::new(
            &crate::config::Config::default(),
            true,
            None,
            api_rx,
            crate::api::EventHub::default(),
        );
        app.state.file_manager = Some(crate::fm::FmState::new(root));
        app
    }

    fn select_all(app: &mut crate::app::App) -> Vec<PathBuf> {
        let file_manager = app.state.file_manager.as_mut().expect("open FM");
        assert!(file_manager.select_all());
        file_manager
            .multi_selection_paths()
            .iter()
            .cloned()
            .collect()
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    // TP-C4.2-CONFIRM: header Delete snapshots current prepared path order into
    // a typed confirmation only. Opening the modal performs no filesystem work.
    #[test]
    fn header_delete_opens_exact_confirmation_without_mutation() {
        let td = TempDir::new("header-exact");
        let alpha = td.file("alpha.txt", b"alpha");
        let beta = td.file("beta.txt", b"beta");
        let mut app = test_app(&td.root);
        let paths = select_all(&mut app);

        assert!(app.dispatch_file_manager_header_action(FileManagerHeaderAction::Delete));

        assert_eq!(app.state.mode, Mode::ConfirmFileDelete);
        let confirmation = app
            .state
            .file_manager_delete_confirmation
            .as_ref()
            .expect("delete confirmation");
        assert_eq!(confirmation.paths, paths);
        assert_eq!(
            confirmation.stage,
            FileManagerDeleteConfirmationStage::ChooseAction
        );
        assert!(app.state.request_file_manager_delete.is_none());
        assert_eq!(fs::read(alpha).expect("alpha preserved"), b"alpha");
        assert_eq!(fs::read(beta).expect("beta preserved"), b"beta");
    }

    // TP-C4.2-CONFIRM: no selection and an operation that becomes in-flight
    // both fail closed before a destructive confirmation can be installed.
    #[test]
    fn delete_confirmation_rejects_empty_and_inflight_authority() {
        let td = TempDir::new("fail-closed");
        td.file("selected.txt", b"selected");
        let mut app = test_app(&td.root);

        assert!(!app.dispatch_file_manager_header_action(FileManagerHeaderAction::Delete));
        assert!(app.state.file_manager_delete_confirmation.is_none());

        let paths = select_all(&mut app);
        app.state.file_manager_operation = Some(FileManagerOperationState {
            generation: 7,
            kind: FileManagerOperationKind::Copy,
            destination_directory: td.root.clone(),
            total_items: 1,
            completed_items: 0,
            failed_items: 0,
            status: FileManagerOperationStatus::Running,
        });
        assert!(!app.dispatch_file_manager_header_action(FileManagerHeaderAction::Delete));
        assert!(app.state.file_manager_delete_confirmation.is_none());
        assert!(paths[0].exists());
    }

    // TP-C4.2-CONFIRM: context-menu Delete converges on the same exact typed
    // modal instead of creating a second or label-derived authority path.
    #[test]
    fn context_delete_opens_the_same_confirmation_model() {
        let td = TempDir::new("context-delete");
        td.file("selected.txt", b"selected");
        let mut app = test_app(&td.root);
        let paths = select_all(&mut app);
        app.state.request_file_manager_context_action = Some(FileManagerContextActionIntent {
            action: FileManagerContextMenuAction::Delete,
            paths: paths.clone(),
        });

        assert!(app.sync_file_operation_worker());

        assert_eq!(app.state.mode, Mode::ConfirmFileDelete);
        assert_eq!(
            app.state
                .file_manager_delete_confirmation
                .as_ref()
                .expect("context delete confirmation")
                .paths,
            paths
        );
        assert!(app.state.request_file_manager_context_action.is_none());
    }

    // TP-C4.2-CONFIRM: Trash is an explicit confirmed request and still does
    // not touch disk on the UI thread. Cancellation clears all modal authority.
    #[test]
    fn trash_confirmation_emits_request_while_cancel_is_side_effect_free() {
        let td = TempDir::new("trash-and-cancel");
        let source = td.file("selected.txt", b"selected");
        let mut app = test_app(&td.root);
        let paths = select_all(&mut app);
        assert!(app.dispatch_file_manager_header_action(FileManagerHeaderAction::Delete));

        app.handle_file_manager_delete_confirmation_key(key(KeyCode::Esc));
        assert_eq!(app.state.mode, Mode::Navigate);
        assert!(app.state.file_manager_delete_confirmation.is_none());
        assert!(app.state.request_file_manager_delete.is_none());
        assert_eq!(
            fs::read(&source).expect("cancel preserves source"),
            b"selected"
        );

        assert!(app.dispatch_file_manager_header_action(FileManagerHeaderAction::Delete));
        app.handle_file_manager_delete_confirmation_key(key(KeyCode::Char('t')));
        let request = app
            .state
            .request_file_manager_delete
            .as_ref()
            .expect("confirmed trash request");
        assert_eq!(request.kind, FileManagerDeleteKind::Trash);
        assert_eq!(request.paths, paths);
        assert_eq!(
            fs::read(source).expect("request preserves source"),
            b"selected"
        );
    }

    // TP-C4.2-CONFIRM: Permanent delete requires a separate second explicit
    // confirmation; the first choice cannot emit irreversible authority.
    #[test]
    fn permanent_delete_requires_second_confirmation() {
        let td = TempDir::new("permanent-two-step");
        let source = td.file("selected.txt", b"selected");
        let mut app = test_app(&td.root);
        let paths = select_all(&mut app);
        assert!(app.dispatch_file_manager_header_action(FileManagerHeaderAction::Delete));

        app.handle_file_manager_delete_confirmation_key(key(KeyCode::Char('d')));
        assert!(app.state.request_file_manager_delete.is_none());
        assert_eq!(
            app.state
                .file_manager_delete_confirmation
                .as_ref()
                .expect("second confirmation")
                .stage,
            FileManagerDeleteConfirmationStage::ConfirmPermanent
        );

        app.handle_file_manager_delete_confirmation_key(key(KeyCode::Enter));
        let request = app
            .state
            .request_file_manager_delete
            .as_ref()
            .expect("permanent delete request");
        assert_eq!(request.kind, FileManagerDeleteKind::Permanent);
        assert_eq!(request.paths, paths);
        assert_eq!(
            fs::read(source).expect("request preserves source"),
            b"selected"
        );
    }

    // TP-C4.2-CONFIRM: closing/reopening or changing the selected identity
    // invalidates the old modal. A late key cannot authorize another target.
    #[test]
    fn stale_or_reopened_confirmation_cannot_emit_delete_request() {
        let first = TempDir::new("stale-first");
        let second = TempDir::new("stale-second");
        let first_path = first.file("first.txt", b"first");
        let second_path = second.file("second.txt", b"second");
        let mut app = test_app(&first.root);
        select_all(&mut app);
        assert!(app.dispatch_file_manager_header_action(FileManagerHeaderAction::Delete));

        app.state.file_manager = None;
        app.state.file_manager = Some(crate::fm::FmState::new(&second.root));
        select_all(&mut app);
        app.handle_file_manager_delete_confirmation_key(key(KeyCode::Char('t')));

        assert!(app.state.request_file_manager_delete.is_none());
        assert!(app.state.file_manager_delete_confirmation.is_none());
        assert_eq!(app.state.mode, Mode::Navigate);
        assert_eq!(fs::read(first_path).expect("first preserved"), b"first");
        assert_eq!(fs::read(second_path).expect("second preserved"), b"second");
    }
}
