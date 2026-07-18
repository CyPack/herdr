use crate::app::agent_reference_picker::AgentReferenceRequest;
use crate::app::state::{FileManagerContextMenuAction, FileManagerOperationState};
use bytes::Bytes;

const MAX_AGENT_ATTACHMENT_PAYLOAD_BYTES: usize = 1024 * 1024;

fn agent_attachment_payload(path: &std::path::Path) -> Result<Vec<u8>, &'static str> {
    let path = path.to_str().ok_or("attachment path is not valid UTF-8")?;
    let payload_len = path
        .len()
        .checked_add(1)
        .filter(|len| *len <= MAX_AGENT_ATTACHMENT_PAYLOAD_BYTES)
        .ok_or("attachment path exceeds 1 MiB")?;
    let mut payload = Vec::with_capacity(payload_len);
    payload.extend_from_slice(path.as_bytes());
    payload.push(b'\r');
    Ok(payload)
}

/// Last-seam filesystem revalidation for one reference path: UTF-8, no
/// control characters, and a still-live regular file or directory target
/// (symlink identity preserved). Vanished, special, and broken targets fail
/// closed (TP-FIP-REF-11/12/13).
pub(super) fn reference_path_is_deliverable(path: &std::path::Path) -> bool {
    let Some(text) = path.to_str() else {
        return false;
    };
    if text.chars().any(char::is_control) {
        return false;
    }
    match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => matches!(
            std::fs::metadata(path),
            Ok(target) if target.is_dir() || target.is_file()
        ),
        Ok(meta) => meta.is_dir() || meta.is_file(),
        Err(_) => false,
    }
}

pub(super) enum TerminalInputSendError {
    RuntimeUnavailable,
    SendFailed { message: String, busy: bool },
}

impl crate::app::App {
    #[cfg(test)]
    pub(super) fn open_file_manager_row_agent_handoff(&mut self, path: std::path::PathBuf) -> bool {
        let selection_is_bulk = self
            .state
            .file_manager
            .as_ref()
            .is_some_and(|file_manager| file_manager.multi_selection_paths().len() > 1);
        if selection_is_bulk {
            self.state.request_file_manager_agent_handoff = None;
            return false;
        }
        self.open_agent_reference_picker(vec![path])
    }

    fn open_file_manager_context_agent_handoff(&mut self, paths: Vec<std::path::PathBuf>) -> bool {
        // C3 already emitted this typed path only after comparing its menu
        // snapshot with the current action model. The picker opener
        // revalidates live entry, operation, and path deliverability; an
        // explicit row activation then snapshots the chosen agent identity
        // (TP-FIP-REF-01/04).
        self.open_agent_reference_picker(paths)
    }

    pub(super) fn sync_file_manager_agent_handoff(&mut self) -> bool {
        let is_send_agent = self
            .state
            .request_file_manager_context_action
            .as_ref()
            .is_some_and(|intent| intent.action == FileManagerContextMenuAction::SendAgent);
        if !is_send_agent {
            return false;
        }
        let Some(intent) = self.state.request_file_manager_context_action.take() else {
            return false;
        };
        let _ = self.open_file_manager_context_agent_handoff(intent.paths);
        true
    }

    pub(super) fn try_send_terminal_input(
        &self,
        terminal_id: &crate::terminal::TerminalId,
        bytes: Bytes,
    ) -> Result<(), TerminalInputSendError> {
        let runtime = self
            .terminal_runtimes
            .get(terminal_id)
            .ok_or(TerminalInputSendError::RuntimeUnavailable)?;
        runtime.try_send_bytes(bytes).map_err(|error| {
            let busy = matches!(error, tokio::sync::mpsc::error::TrySendError::Full(_));
            TerminalInputSendError::SendFailed {
                message: error.to_string(),
                busy,
            }
        })
    }

    pub(super) fn sync_file_manager_agent_handoff_send(&mut self) -> bool {
        let Some(request) = self.state.request_file_manager_agent_handoff.take() else {
            return false;
        };
        if !self.file_manager_agent_handoff_is_current(&request) {
            self.show_file_manager_agent_handoff_failure("agent handoff authority changed");
            return true;
        }
        if !reference_path_is_deliverable(&request.path) {
            self.show_file_manager_agent_handoff_failure("reference path is unavailable");
            return true;
        }
        let Some(path) = request.path.to_str() else {
            self.show_file_manager_agent_handoff_failure("agent handoff authority changed");
            return true;
        };
        // Reference-only contract (TP-FIP-REF-05/07): exactly the UTF-8 path
        // bytes, never a submit byte. The agent decides when to send.
        let payload = Bytes::copy_from_slice(path.as_bytes());

        match self.try_send_terminal_input(&request.terminal_id, payload) {
            Ok(()) => {}
            Err(TerminalInputSendError::RuntimeUnavailable) => {
                self.show_file_manager_agent_handoff_failure("agent runtime is unavailable");
            }
            Err(TerminalInputSendError::SendFailed { busy, .. }) => {
                let context = if busy {
                    "agent input is busy"
                } else {
                    "agent runtime is unavailable"
                };
                self.show_file_manager_agent_handoff_failure(context);
            }
        }
        true
    }

    pub(super) fn sync_agent_attachment_delivery(&mut self) -> bool {
        let Some(request) = self.state.request_agent_attachment_delivery.take() else {
            return false;
        };
        let picker_matches = self
            .state
            .agent_attachment_picker
            .as_ref()
            .is_some_and(|picker| picker.target == request.target);
        if !picker_matches
            || !self
                .state
                .agent_attachment_target_is_current(&request.target)
        {
            self.show_agent_attachment_delivery_failure("attachment target changed");
            return true;
        }
        if self.state.agent_attachment_selected_file().as_ref() != Some(&request.path) {
            self.show_agent_attachment_delivery_failure("attachment selection changed");
            return true;
        }
        let payload = match agent_attachment_payload(&request.path) {
            Ok(payload) => payload,
            Err(context) => {
                self.show_agent_attachment_delivery_failure(context);
                return true;
            }
        };
        if !std::fs::metadata(&request.path).is_ok_and(|metadata| metadata.is_file()) {
            self.show_agent_attachment_delivery_failure(
                "selected attachment is no longer a regular file",
            );
            return true;
        }

        match self.try_send_terminal_input(&request.target.terminal_id, Bytes::from(payload)) {
            Ok(()) => self.state.close_agent_attachment_picker(),
            Err(TerminalInputSendError::RuntimeUnavailable) => {
                self.show_agent_attachment_delivery_failure("agent runtime is unavailable");
            }
            Err(TerminalInputSendError::SendFailed { busy, .. }) => {
                let context = if busy {
                    "agent input is busy"
                } else {
                    "agent runtime is unavailable"
                };
                self.show_agent_attachment_delivery_failure(context);
            }
        }
        true
    }

    fn file_manager_agent_handoff_is_current(&self, request: &AgentReferenceRequest) -> bool {
        if self.file_operation_worker.is_busy()
            || self
                .state
                .file_manager_operation
                .as_ref()
                .is_some_and(FileManagerOperationState::is_running)
            || request.path.to_str().is_none()
            || !self
                .state
                .file_manager
                .as_ref()
                .is_some_and(|file_manager| {
                    file_manager
                        .entries
                        .iter()
                        .any(|entry| entry.operation_supported() && entry.path == request.path)
                })
        {
            return false;
        }

        // The picker chose an explicit target (TP-FIP-REF-04): validate the
        // CHOSEN workspace/pane/terminal binding, not the focused pane.
        let Some(workspace_idx) = self
            .state
            .workspaces
            .iter()
            .position(|workspace| workspace.id == request.workspace_id)
        else {
            return false;
        };
        if self
            .state
            .terminal_id_for_pane(workspace_idx, request.pane_id)
            .as_ref()
            != Some(&request.terminal_id)
        {
            return false;
        }
        self.state
            .terminals
            .get(&request.terminal_id)
            .is_some_and(crate::terminal::TerminalState::is_agent_terminal)
    }

    pub(super) fn show_file_manager_agent_handoff_failure(&mut self, context: &str) {
        let previous_toast = self.state.toast.clone();
        self.state.toast = Some(crate::app::state::ToastNotification {
            kind: crate::app::state::ToastKind::NeedsAttention,
            title: "send to agent failed".to_string(),
            context: context.to_string(),
            position: None,
            target: None,
        });
        self.sync_toast_deadline(previous_toast);
    }

    fn show_agent_attachment_delivery_failure(&mut self, context: &str) {
        let previous_toast = self.state.toast.clone();
        self.state.toast = Some(crate::app::state::ToastNotification {
            kind: crate::app::state::ToastKind::NeedsAttention,
            title: "attach file failed".to_string(),
            context: context.to_string(),
            position: None,
            target: None,
        });
        self.sync_toast_deadline(previous_toast);
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;

    use crate::app::agent_reference_picker::AgentReferenceRequest;
    use crate::app::state::Mode;

    fn reference_request_for(
        app: &crate::app::App,
        path: std::path::PathBuf,
        terminal_id: crate::terminal::TerminalId,
    ) -> AgentReferenceRequest {
        AgentReferenceRequest {
            path,
            source_files_generation: 0,
            workspace_id: app
                .state
                .workspaces
                .first()
                .map(|workspace| workspace.id.clone())
                .unwrap_or_default(),
            pane_id: app.state.workspaces[0]
                .focused_pane_id()
                .expect("focused pane for reference request"),
            terminal_id,
        }
    }

    struct HandoffFixture {
        root: std::path::PathBuf,
    }

    impl HandoffFixture {
        fn new(tag: &str) -> Self {
            let root = std::env::temp_dir().join(format!(
                "herdr-fm-agent-handoff-{}-{tag}",
                std::process::id()
            ));
            let _ = std::fs::remove_dir_all(&root);
            std::fs::create_dir_all(&root).expect("create handoff fixture root");
            Self { root }
        }

        fn file(&self, name: &str) -> std::path::PathBuf {
            let path = self.root.join(name);
            std::fs::write(&path, b"handoff").expect("write handoff fixture");
            path
        }
    }

    impl Drop for HandoffFixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    fn app_with_agent_handoff(
        root: &std::path::Path,
        channel_capacity: usize,
    ) -> (
        crate::app::App,
        crate::terminal::TerminalId,
        tokio::sync::mpsc::Receiver<Bytes>,
    ) {
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = crate::app::App::new(
            &crate::config::Config::default(),
            true,
            None,
            api_rx,
            crate::api::EventHub::default(),
        );
        let workspace = crate::workspace::Workspace::test_new("fm-agent-handoff-send");
        let pane_id = workspace.tabs[0].root_pane;
        let terminal_id = workspace
            .terminal_id(pane_id)
            .expect("handoff terminal id")
            .clone();
        app.state.workspaces = vec![workspace];
        app.state.ensure_test_terminals();
        app.state
            .terminals
            .get_mut(&terminal_id)
            .expect("handoff terminal state")
            .set_agent_name("handoff-target".into());
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state
            .try_open_file_manager_with(|_| Some(crate::fm::FmState::new(root)))
            .expect("Files activation");
        let (runtime, receiver) =
            crate::terminal::TerminalRuntime::test_with_channel_capacity(80, 24, channel_capacity);
        app.terminal_runtimes.insert(terminal_id.clone(), runtime);
        (app, terminal_id, receiver)
    }

    fn app_with_attachment_picker(
        root: &std::path::Path,
        channel_capacity: usize,
    ) -> (
        crate::app::App,
        crate::layout::PaneId,
        crate::terminal::TerminalId,
        tokio::sync::mpsc::Receiver<Bytes>,
    ) {
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = crate::app::App::new(
            &crate::config::Config::default(),
            true,
            None,
            api_rx,
            crate::api::EventHub::default(),
        );
        let mut workspace = crate::workspace::Workspace::test_new("attachment-picker-send");
        workspace.identity_cwd = root.to_path_buf();
        let pane_id = workspace.tabs[0].root_pane;
        let terminal_id = workspace
            .terminal_id(pane_id)
            .expect("attachment terminal id")
            .clone();
        app.state.workspaces = vec![workspace];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;
        app.state.ensure_test_terminals();
        app.state
            .terminals
            .get_mut(&terminal_id)
            .expect("attachment terminal state")
            .set_agent_name("attachment-target".into());
        app.state.view.terminal_area = ratatui::layout::Rect::new(0, 0, 80, 24);
        app.state
            .open_agent_attachment_picker()
            .expect("open attachment picker");
        let (runtime, receiver) =
            crate::terminal::TerminalRuntime::test_with_channel_capacity(80, 24, channel_capacity);
        app.terminal_runtimes.insert(terminal_id.clone(), runtime);
        (app, pane_id, terminal_id, receiver)
    }

    fn prepare_attachment_request(app: &mut crate::app::App, path: &std::path::Path) {
        let picker = app
            .state
            .agent_attachment_picker
            .as_mut()
            .expect("open attachment picker");
        let entry_idx = picker
            .file_manager
            .entries
            .iter()
            .position(|entry| entry.path == path)
            .expect("attachment fixture entry");
        picker.file_manager.select(entry_idx);
        app.route_agent_attachment_picker_key(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Enter,
            crossterm::event::KeyModifiers::NONE,
        ));
    }

    fn app_with_non_agent_handoff(
        root: &std::path::Path,
        with_neighbor: bool,
    ) -> (
        crate::app::App,
        crate::layout::PaneId,
        crate::terminal::TerminalId,
    ) {
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = crate::app::App::new(
            &crate::config::Config::default(),
            true,
            None,
            api_rx,
            crate::api::EventHub::default(),
        );
        let mut workspace = crate::workspace::Workspace::test_new("fm-claude-split");
        let source_pane_id = workspace.tabs[0].root_pane;
        if with_neighbor {
            workspace.test_split(ratatui::layout::Direction::Horizontal);
            workspace.tabs[0].layout.focus_pane(source_pane_id);
        }
        let source_terminal_id = workspace
            .terminal_id(source_pane_id)
            .expect("source terminal identity")
            .clone();
        app.state.workspaces = vec![workspace];
        app.state.ensure_test_terminals();
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state
            .try_open_file_manager_with(|_| Some(crate::fm::FmState::new(root)))
            .expect("Files activation");
        (app, source_pane_id, source_terminal_id)
    }

    // TP-FIP-REF-05/07 (supersedes TP-C5-SEND): the reference payload is
    // EXACTLY the literal UTF-8 path bytes — no CR, LF, Enter, prefix,
    // suffix, or implicit whitespace, and never a second attempt. Shell
    // metacharacters stay data; this is chat text, not a command.
    #[tokio::test]
    async fn existing_agent_receives_exact_path_bytes_with_no_submit() {
        let fixture = HandoffFixture::new("literal");
        let path = fixture.file("space 'quote' $(touch nope) `echo` ünicode.txt");
        let (mut app, terminal_id, mut receiver) = app_with_agent_handoff(&fixture.root, 4);
        app.state.request_file_manager_agent_handoff =
            Some(reference_request_for(&app, path.clone(), terminal_id));

        assert!(app.sync_file_manager_agent_handoff_send());
        assert!(app.state.request_file_manager_agent_handoff.is_none());
        let expected = path
            .to_str()
            .expect("UTF-8 fixture path")
            .as_bytes()
            .to_vec();
        let sent = receiver.try_recv().expect("one literal reference payload");
        assert_eq!(sent, Bytes::from(expected));
        assert!(
            !sent.contains(&b'\r') && !sent.contains(&b'\n'),
            "the reference payload must contain no submit byte"
        );
        assert!(receiver.try_recv().is_err(), "no second payload");
        assert!(!app.sync_file_manager_agent_handoff_send());
        assert!(receiver.try_recv().is_err(), "frame retry stays silent");
    }

    // TP-FIP-REF-03: a non-agent focused terminal must NOT trigger an
    // implicit Claude split/chat for the reference action. No typed split
    // request, no send request, and no new pane or terminal may appear.
    #[tokio::test]
    async fn non_agent_focus_prepares_no_claude_split_for_reference_action() {
        let fixture = HandoffFixture::new("no-implicit-split");
        let path = fixture.file("selected.txt");
        let (mut app, _, _) = app_with_non_agent_handoff(&fixture.root, false);
        let before_panes = app.state.workspaces[0].tabs[0].layout.pane_ids();
        let before_terminals: std::collections::HashSet<_> =
            app.state.terminals.keys().cloned().collect();

        assert!(
            !app.open_file_manager_row_agent_handoff(path),
            "the reference action must never prepare an implicit Claude split"
        );
        assert!(app.state.request_file_manager_agent_handoff.is_none());
        assert_eq!(
            app.state.workspaces[0].tabs[0].layout.pane_ids(),
            before_panes
        );
        assert_eq!(
            app.state
                .terminals
                .keys()
                .cloned()
                .collect::<std::collections::HashSet<_>>(),
            before_terminals
        );
    }

    // TP-FIP-REF-06: a directory is a first-class reference target; its exact
    // UTF-8 path bytes cross the boundary once with no submit byte.
    #[tokio::test]
    async fn directory_reference_delivers_exact_directory_path() {
        let fixture = HandoffFixture::new("directory-ref");
        let dir = fixture.root.join("docs");
        std::fs::create_dir(&dir).expect("create directory fixture");
        let (mut app, _, mut receiver) = app_with_agent_handoff(&fixture.root, 4);

        assert!(app.open_file_manager_row_agent_handoff(dir.clone()));
        assert!(app.activate_agent_reference_picker_selection());
        assert!(app.sync_file_manager_agent_handoff_send());
        let expected = dir
            .to_str()
            .expect("UTF-8 fixture path")
            .as_bytes()
            .to_vec();
        let sent = receiver
            .try_recv()
            .expect("one directory reference payload");
        assert_eq!(sent, Bytes::from(expected));
        assert!(
            !sent.contains(&b'\r') && !sent.contains(&b'\n'),
            "the directory reference payload must contain no submit byte"
        );
        assert!(receiver.try_recv().is_err(), "no second payload");
    }

    // TP-FIP-REF-11: a path deleted between prepare and send fails closed at
    // the delivery seam — zero bytes and one visible failure.
    #[tokio::test]
    async fn deleted_path_before_send_sends_zero_bytes() {
        let fixture = HandoffFixture::new("deleted-before-send");
        let path = fixture.file("volatile.txt");
        let (mut app, terminal_id, mut receiver) = app_with_agent_handoff(&fixture.root, 4);
        app.state.request_file_manager_agent_handoff =
            Some(reference_request_for(&app, path.clone(), terminal_id));
        std::fs::remove_file(&path).expect("delete between prepare and send");

        assert!(app.sync_file_manager_agent_handoff_send());
        assert!(
            receiver.try_recv().is_err(),
            "a vanished path must deliver zero bytes"
        );
        let toast = app.state.toast.as_ref().expect("visible delivery failure");
        assert_eq!(toast.title, "send to agent failed");
        assert_eq!(toast.context, "reference path is unavailable");
        assert!(!app.sync_file_manager_agent_handoff_send());
        assert!(receiver.try_recv().is_err(), "no retry on later ticks");
    }

    // TP-FIP-REF-12: a path whose kind changed to an unsupported special
    // target between prepare and send fails closed with zero bytes.
    #[cfg(unix)]
    #[tokio::test]
    async fn path_kind_change_to_special_before_send_sends_zero_bytes() {
        let fixture = HandoffFixture::new("kind-change-before-send");
        let path = fixture.file("mutating.txt");
        let (mut app, terminal_id, mut receiver) = app_with_agent_handoff(&fixture.root, 4);
        app.state.request_file_manager_agent_handoff =
            Some(reference_request_for(&app, path.clone(), terminal_id));
        std::fs::remove_file(&path).expect("remove before kind change");
        let status = std::process::Command::new("mkfifo")
            .arg(&path)
            .status()
            .expect("mkfifo runs");
        assert!(status.success(), "fifo fixture must exist");

        assert!(app.sync_file_manager_agent_handoff_send());
        assert!(
            receiver.try_recv().is_err(),
            "a special-kind path must deliver zero bytes"
        );
        assert_eq!(
            app.state.toast.as_ref().map(|toast| toast.context.as_str()),
            Some("reference path is unavailable")
        );
    }

    // TP-FIP-REF-13: a control character anywhere in the path disables the
    // reference action at prepare AND independently at the delivery seam.
    #[cfg(unix)]
    #[tokio::test]
    async fn control_character_path_disables_reference_action() {
        let fixture = HandoffFixture::new("control-char");
        let path = fixture.file("a\nb.txt");
        let (mut app, terminal_id, mut receiver) = app_with_agent_handoff(&fixture.root, 4);

        assert!(
            !app.open_file_manager_row_agent_handoff(path.clone()),
            "prepare must refuse a control-character path"
        );
        assert!(app.state.request_file_manager_agent_handoff.is_none());

        app.state.request_file_manager_agent_handoff =
            Some(reference_request_for(&app, path, terminal_id));
        assert!(app.sync_file_manager_agent_handoff_send());
        assert!(
            receiver.try_recv().is_err(),
            "the send seam must independently reject control characters"
        );
        assert_eq!(
            app.state.toast.as_ref().map(|toast| toast.context.as_str()),
            Some("reference path is unavailable")
        );
    }

    // TP-FIP-REF-13: a non-UTF-8 path never becomes a typed request.
    #[cfg(unix)]
    #[tokio::test]
    async fn non_utf8_path_rejects_at_prepare() {
        use std::os::unix::ffi::OsStringExt;

        let fixture = HandoffFixture::new("non-utf8-prepare");
        let _ = fixture.file("decoy.txt");
        let (mut app, _, _receiver) = app_with_agent_handoff(&fixture.root, 4);
        let path = std::path::PathBuf::from(std::ffi::OsString::from_vec(vec![0xff]));

        assert!(!app.open_file_manager_row_agent_handoff(path));
        assert!(app.state.request_file_manager_agent_handoff.is_none());
    }

    // TP-FIP-REF-18: spaces and punctuation survive prepare, revalidation,
    // and delivery byte-for-byte — the validator must never reject them.
    #[tokio::test]
    async fn spaces_and_punctuation_paths_preserved_byte_for_byte() {
        let fixture = HandoffFixture::new("punctuation-ref");
        let path = fixture.file("notes (v2), [draft] & final; 100%.md");
        let (mut app, _, mut receiver) = app_with_agent_handoff(&fixture.root, 4);

        assert!(app.open_file_manager_row_agent_handoff(path.clone()));
        assert!(app.activate_agent_reference_picker_selection());
        assert!(app.sync_file_manager_agent_handoff_send());
        let expected = path
            .to_str()
            .expect("UTF-8 fixture path")
            .as_bytes()
            .to_vec();
        assert_eq!(
            receiver.try_recv().expect("one punctuation payload"),
            Bytes::from(expected)
        );
        assert!(receiver.try_recv().is_err(), "no second payload");
    }

    // TP-C5-SEND: authority is revalidated at send time. Lost agent identity
    // consumes the stale request without writing any bytes.
    #[tokio::test]
    async fn existing_agent_handoff_fails_closed_after_agent_identity_is_lost() {
        let fixture = HandoffFixture::new("lost-agent");
        let path = fixture.file("selected.txt");
        let (mut app, terminal_id, mut receiver) = app_with_agent_handoff(&fixture.root, 2);
        app.state.request_file_manager_agent_handoff =
            Some(reference_request_for(&app, path, terminal_id.clone()));
        app.state
            .terminals
            .get_mut(&terminal_id)
            .expect("handoff terminal state")
            .clear_agent_name();

        assert!(app.sync_file_manager_agent_handoff_send());
        assert!(app.state.request_file_manager_agent_handoff.is_none());
        assert!(receiver.try_recv().is_err());
        let toast = app.state.toast.as_ref().expect("visible authority failure");
        assert_eq!(toast.kind, crate::app::state::ToastKind::NeedsAttention);
        assert_eq!(toast.title, "send to agent failed");
        assert_eq!(toast.context, "agent handoff authority changed");
        // TP-FIP-REF-10: exactly one failure — later ticks stay silent.
        assert!(!app.sync_file_manager_agent_handoff_send());
        assert!(receiver.try_recv().is_err(), "no retry on later ticks");
    }

    // TP-FIP-REF-08: a workspace that vanished between prepare and send
    // fails closed with zero bytes and one visible failure.
    #[tokio::test]
    async fn vanished_workspace_or_pane_sends_zero_bytes() {
        let fixture = HandoffFixture::new("vanished-workspace");
        let path = fixture.file("selected.txt");
        let (mut app, terminal_id, mut receiver) = app_with_agent_handoff(&fixture.root, 2);
        app.state.request_file_manager_agent_handoff =
            Some(reference_request_for(&app, path.clone(), terminal_id));
        app.state.workspaces.clear();
        app.state.active = None;

        assert!(app.sync_file_manager_agent_handoff_send());
        assert!(
            receiver.try_recv().is_err(),
            "a vanished workspace must deliver zero bytes"
        );
        assert_eq!(
            app.state.toast.as_ref().map(|toast| toast.context.as_str()),
            Some("agent handoff authority changed")
        );
        assert!(!app.sync_file_manager_agent_handoff_send());
        assert!(receiver.try_recv().is_err(), "no retry on later ticks");
    }

    // TP-FIP-REF-09: the request is bound to the CHOSEN pane identity.
    // When that pane's terminal no longer matches the snapshot, no bytes may
    // cross to the terminal that now lives there.
    #[tokio::test]
    async fn changed_terminal_identity_sends_zero_bytes() {
        let fixture = HandoffFixture::new("changed-terminal");
        let path = fixture.file("selected.txt");
        let (mut app, _, mut receiver) = app_with_agent_handoff(&fixture.root, 2);
        let pane_a = app.state.workspaces[0]
            .focused_pane_id()
            .expect("focused pane");
        app.state.workspaces[0].test_split(ratatui::layout::Direction::Horizontal);
        app.state.ensure_test_terminals();
        let pane_b = app.state.workspaces[0].tabs[0]
            .layout
            .pane_ids()
            .into_iter()
            .find(|pane_id| *pane_id != pane_a)
            .expect("neighbor pane");
        let terminal_b = app
            .state
            .terminal_id_for_pane(0, pane_b)
            .expect("neighbor terminal");
        // Snapshot claims pane_a delivers to terminal_b — a binding that no
        // longer exists. Delivery must fail closed.
        app.state.request_file_manager_agent_handoff = Some(AgentReferenceRequest {
            path,
            source_files_generation: 0,
            workspace_id: app.state.workspaces[0].id.clone(),
            pane_id: pane_a,
            terminal_id: terminal_b,
        });

        assert!(app.sync_file_manager_agent_handoff_send());
        assert!(
            receiver.try_recv().is_err(),
            "a changed terminal identity must deliver zero bytes"
        );
        assert_eq!(
            app.state.toast.as_ref().map(|toast| toast.context.as_str()),
            Some("agent handoff authority changed")
        );
        assert!(!app.sync_file_manager_agent_handoff_send());
        assert!(receiver.try_recv().is_err(), "no retry on later ticks");
    }

    // TP-C5-SEND: a watcher-reconciled stale entry and a vanished runtime are
    // independently fail-closed and visible; neither can route to another pane.
    #[tokio::test]
    async fn existing_agent_handoff_rejects_stale_path_and_missing_runtime() {
        let fixture = HandoffFixture::new("stale-path");
        let path = fixture.file("selected.txt");
        let (mut stale, terminal_id, mut stale_receiver) = app_with_agent_handoff(&fixture.root, 2);
        stale.state.request_file_manager_agent_handoff =
            Some(reference_request_for(&stale, path.clone(), terminal_id));
        stale
            .state
            .file_manager
            .as_mut()
            .expect("open handoff FM")
            .entries
            .clear();

        assert!(stale.sync_file_manager_agent_handoff_send());
        assert!(stale_receiver.try_recv().is_err());
        assert_eq!(
            stale
                .state
                .toast
                .as_ref()
                .map(|toast| toast.context.as_str()),
            Some("agent handoff authority changed")
        );

        let (mut missing, terminal_id, mut missing_receiver) =
            app_with_agent_handoff(&fixture.root, 2);
        missing.state.request_file_manager_agent_handoff =
            Some(reference_request_for(&missing, path, terminal_id.clone()));
        let runtime = missing
            .terminal_runtimes
            .remove(&terminal_id)
            .expect("remove handoff runtime");

        assert!(missing.sync_file_manager_agent_handoff_send());
        assert!(missing_receiver.try_recv().is_err());
        assert_eq!(
            missing
                .state
                .toast
                .as_ref()
                .map(|toast| toast.context.as_str()),
            Some("agent runtime is unavailable")
        );
        runtime.shutdown();
    }

    // TP-C5-SEND: a full terminal input lane is a terminal failure for this
    // one-shot request. It is not retried on later frame ticks.
    #[tokio::test]
    async fn existing_agent_handoff_backpressure_is_consumed_without_hot_retry() {
        let fixture = HandoffFixture::new("backpressure");
        let path = fixture.file("selected.txt");
        let (mut app, terminal_id, mut receiver) = app_with_agent_handoff(&fixture.root, 1);
        app.terminal_runtimes
            .get(&terminal_id)
            .expect("handoff runtime")
            .try_send_bytes(Bytes::from_static(b"occupied"))
            .expect("fill handoff input lane");
        app.state.request_file_manager_agent_handoff =
            Some(reference_request_for(&app, path, terminal_id));

        assert!(app.sync_file_manager_agent_handoff_send());
        assert!(app.state.request_file_manager_agent_handoff.is_none());
        assert_eq!(
            receiver.try_recv().expect("pre-existing lane payload"),
            Bytes::from_static(b"occupied")
        );
        assert!(!app.sync_file_manager_agent_handoff_send());
        assert!(
            receiver.try_recv().is_err(),
            "failed handoff is not retried"
        );
        let toast = app.state.toast.as_ref().expect("visible send failure");
        assert_eq!(toast.kind, crate::app::state::ToastKind::NeedsAttention);
        assert_eq!(toast.title, "send to agent failed");
        assert_eq!(toast.context, "agent input is busy");
    }

    // TP-M1.3-SEND: the exact selected UTF-8 path plus one CR crosses the
    // runtime boundary once; success closes only the client-local picker.
    #[tokio::test]
    async fn attachment_delivery_sends_one_literal_path_and_closes_on_success() {
        let fixture = HandoffFixture::new("attachment-literal");
        let path = fixture.file("space 'quote' $(touch nope) `echo` ünicode.txt");
        let (mut app, _, terminal_id, mut receiver) = app_with_attachment_picker(&fixture.root, 4);
        prepare_attachment_request(&mut app, &path);

        assert!(app.sync_agent_attachment_delivery());
        assert!(app.state.request_agent_attachment_delivery.is_none());
        let mut expected = path.to_str().unwrap().as_bytes().to_vec();
        expected.push(b'\r');
        assert_eq!(receiver.try_recv().unwrap(), Bytes::from(expected));
        assert!(receiver.try_recv().is_err());
        assert!(app.state.agent_attachment_picker.is_none());
        assert_eq!(app.state.mode, Mode::Terminal);
        assert!(!app.sync_agent_attachment_delivery());
        assert!(receiver.try_recv().is_err());
        app.terminal_runtimes
            .remove(&terminal_id)
            .unwrap()
            .shutdown();
    }

    // TP-M1.3-STALE: exact target identity and selected filesystem authority
    // are revalidated after input and before any runtime byte is sent.
    #[tokio::test]
    async fn attachment_delivery_rejects_lost_agent_and_vanished_file() {
        let fixture = HandoffFixture::new("attachment-stale");
        let path = fixture.file("selected.txt");
        let (mut lost_agent, _, terminal_id, mut lost_receiver) =
            app_with_attachment_picker(&fixture.root, 2);
        prepare_attachment_request(&mut lost_agent, &path);
        lost_agent
            .state
            .terminals
            .get_mut(&terminal_id)
            .unwrap()
            .clear_agent_name();

        assert!(lost_agent.sync_agent_attachment_delivery());
        assert!(lost_receiver.try_recv().is_err());
        assert!(lost_agent.state.agent_attachment_picker.is_some());
        assert_eq!(
            lost_agent
                .state
                .toast
                .as_ref()
                .map(|toast| toast.context.as_str()),
            Some("attachment target changed")
        );
        lost_agent
            .terminal_runtimes
            .remove(&terminal_id)
            .unwrap()
            .shutdown();

        let (mut vanished, _, terminal_id, mut vanished_receiver) =
            app_with_attachment_picker(&fixture.root, 2);
        prepare_attachment_request(&mut vanished, &path);
        std::fs::remove_file(&path).unwrap();

        assert!(vanished.sync_agent_attachment_delivery());
        assert!(vanished_receiver.try_recv().is_err());
        assert!(vanished.state.agent_attachment_picker.is_some());
        assert_eq!(
            vanished
                .state
                .toast
                .as_ref()
                .map(|toast| toast.context.as_str()),
            Some("selected attachment is no longer a regular file")
        );
        vanished
            .terminal_runtimes
            .remove(&terminal_id)
            .unwrap()
            .shutdown();
    }

    // TP-M1.3-IDENTITY: moving focus to another pane or losing the exact
    // runtime registry entry consumes the request without cross-routing bytes.
    #[tokio::test]
    async fn attachment_delivery_rejects_changed_focus_and_missing_runtime() {
        let fixture = HandoffFixture::new("attachment-identity");
        let path = fixture.file("selected.txt");
        let (mut moved, _, terminal_id, mut moved_receiver) =
            app_with_attachment_picker(&fixture.root, 2);
        prepare_attachment_request(&mut moved, &path);
        moved.state.workspaces[0].test_split(ratatui::layout::Direction::Horizontal);
        moved.state.ensure_test_terminals();

        assert!(moved.sync_agent_attachment_delivery());
        assert!(moved_receiver.try_recv().is_err());
        assert!(moved.state.agent_attachment_picker.is_some());
        assert_eq!(
            moved
                .state
                .toast
                .as_ref()
                .map(|toast| toast.context.as_str()),
            Some("attachment target changed")
        );
        moved
            .terminal_runtimes
            .remove(&terminal_id)
            .unwrap()
            .shutdown();

        let (mut missing, _, terminal_id, mut missing_receiver) =
            app_with_attachment_picker(&fixture.root, 2);
        prepare_attachment_request(&mut missing, &path);
        let runtime = missing
            .terminal_runtimes
            .remove(&terminal_id)
            .expect("remove exact attachment runtime");

        assert!(missing.sync_agent_attachment_delivery());
        assert!(missing_receiver.try_recv().is_err());
        assert!(missing.state.agent_attachment_picker.is_some());
        assert_eq!(
            missing
                .state
                .toast
                .as_ref()
                .map(|toast| toast.context.as_str()),
            Some("agent runtime is unavailable")
        );
        runtime.shutdown();
    }

    // TP-M1.3-BUSY: a full input lane consumes the one-shot request, keeps the
    // picker available for an explicit retry, and never retries on later ticks.
    #[tokio::test]
    async fn attachment_delivery_backpressure_is_visible_without_hot_retry() {
        let fixture = HandoffFixture::new("attachment-busy");
        let path = fixture.file("selected.txt");
        let (mut app, _, terminal_id, mut receiver) = app_with_attachment_picker(&fixture.root, 1);
        app.terminal_runtimes
            .get(&terminal_id)
            .unwrap()
            .try_send_bytes(Bytes::from_static(b"occupied"))
            .unwrap();
        prepare_attachment_request(&mut app, &path);

        assert!(app.sync_agent_attachment_delivery());
        assert!(app.state.request_agent_attachment_delivery.is_none());
        assert_eq!(
            receiver.try_recv().unwrap(),
            Bytes::from_static(b"occupied")
        );
        assert!(receiver.try_recv().is_err());
        assert!(app.state.agent_attachment_picker.is_some());
        assert_eq!(
            app.state.toast.as_ref().map(|toast| toast.context.as_str()),
            Some("agent input is busy")
        );
        assert!(!app.sync_agent_attachment_delivery());
        assert!(receiver.try_recv().is_err());
        app.terminal_runtimes
            .remove(&terminal_id)
            .unwrap()
            .shutdown();
    }

    // TP-M1.3-LIMIT: payload bounds are checked with the final CR included,
    // independently of platform filesystem path-length limits.
    #[test]
    fn attachment_payload_rejects_more_than_one_mib_including_enter() {
        let at_limit = std::path::PathBuf::from("a".repeat(1024 * 1024 - 1));
        assert_eq!(
            super::agent_attachment_payload(&at_limit).unwrap().len(),
            1024 * 1024
        );

        let too_large = std::path::PathBuf::from("a".repeat(1024 * 1024));
        assert_eq!(
            super::agent_attachment_payload(&too_large),
            Err("attachment path exceeds 1 MiB")
        );
    }

    #[cfg(unix)]
    #[test]
    fn attachment_payload_rejects_non_utf8_path() {
        use std::os::unix::ffi::OsStringExt;

        let path = std::path::PathBuf::from(std::ffi::OsString::from_vec(vec![0xff]));
        assert_eq!(
            super::agent_attachment_payload(&path),
            Err("attachment path is not valid UTF-8")
        );
    }
}
