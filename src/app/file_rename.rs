use crate::app::state::{
    FileManagerContextMenuAction, FileManagerContextMenuModel, FileManagerOperationState,
    FileManagerRenameRequest, FileManagerRenameState, FileManagerRenameValidationError, Mode,
};
use crate::fm::rename::{
    validate_rename_name_component, RenameNameIssue,
    RenameNamePlatform as FileManagerRenamePlatform,
};

#[derive(Debug, Clone, PartialEq, Eq)]
enum FileManagerRenameName {
    Unchanged,
    Changed(String),
}

fn validate_file_manager_name(
    original_name: &str,
    input: &str,
    platform: FileManagerRenamePlatform,
) -> Result<FileManagerRenameName, FileManagerRenameValidationError> {
    validate_rename_name_component(input, platform).map_err(map_rename_name_issue)?;

    if input == original_name {
        Ok(FileManagerRenameName::Unchanged)
    } else {
        Ok(FileManagerRenameName::Changed(input.to_string()))
    }
}

const fn map_rename_name_issue(issue: RenameNameIssue) -> FileManagerRenameValidationError {
    match issue {
        RenameNameIssue::Empty => FileManagerRenameValidationError::Empty,
        RenameNameIssue::CurrentDirectory => FileManagerRenameValidationError::CurrentDirectory,
        RenameNameIssue::ParentDirectory => FileManagerRenameValidationError::ParentDirectory,
        RenameNameIssue::Absolute => FileManagerRenameValidationError::Absolute,
        RenameNameIssue::Separator => FileManagerRenameValidationError::Separator,
        RenameNameIssue::ContainsNul => FileManagerRenameValidationError::ContainsNul,
        RenameNameIssue::NameTooLong => FileManagerRenameValidationError::NameTooLong,
        RenameNameIssue::WindowsReservedName => {
            FileManagerRenameValidationError::WindowsReservedName
        }
        RenameNameIssue::WindowsReservedCharacter => {
            FileManagerRenameValidationError::WindowsReservedCharacter
        }
        RenameNameIssue::WindowsTrailingDotOrSpace => {
            FileManagerRenameValidationError::WindowsTrailingDotOrSpace
        }
    }
}

impl crate::app::App {
    #[cfg(test)]
    pub(super) fn open_file_manager_row_rename(&mut self, path: std::path::PathBuf) -> bool {
        let selection_is_bulk = self
            .state
            .file_manager
            .as_ref()
            .is_some_and(|file_manager| file_manager.multi_selection_paths().len() > 1);
        if selection_is_bulk {
            return false;
        }
        self.open_file_manager_rename(vec![path])
    }

    pub(super) fn open_file_manager_context_rename(
        &mut self,
        paths: Vec<std::path::PathBuf>,
    ) -> bool {
        let Some(file_manager) = self.state.file_manager.as_ref() else {
            return false;
        };
        let action_bar = crate::ui::compute_file_manager_action_bar_model(
            file_manager,
            &self.state.file_manager_clipboard,
            self.state
                .file_manager_operation
                .as_ref()
                .is_some_and(FileManagerOperationState::is_running),
        );
        let Some(model) =
            FileManagerContextMenuModel::from_action_bar_with_plugins(&action_bar, &[])
        else {
            return false;
        };
        let rename_is_current = model.paths == paths
            && model
                .items
                .iter()
                .any(|item| item.action == FileManagerContextMenuAction::Rename && item.enabled);
        if !rename_is_current {
            return false;
        }
        self.open_file_manager_rename(paths)
    }

    fn open_file_manager_rename(&mut self, paths: Vec<std::path::PathBuf>) -> bool {
        if paths.len() != 1
            || self.file_operation_worker.is_busy()
            || self
                .state
                .file_manager_operation
                .as_ref()
                .is_some_and(FileManagerOperationState::is_running)
        {
            return false;
        }
        let Some(path) = paths.first() else {
            return false;
        };
        let Some(file_manager) = self.state.file_manager.as_ref() else {
            return false;
        };
        let Some(entry) = file_manager
            .entries
            .iter()
            .find(|entry| entry.path == *path)
        else {
            return false;
        };
        if !entry.operation_supported {
            return false;
        }
        let Some(name) = path
            .file_name()
            .and_then(|name| name.to_str())
            .map(str::to_string)
        else {
            return false;
        };

        self.state.request_file_manager_rename = None;
        self.state.file_manager_rename = Some(FileManagerRenameState {
            paths,
            validation_error: None,
        });
        self.state.name_input = name;
        self.state.name_input_replace_on_type = true;
        self.state.mode = Mode::RenameFile;
        true
    }

    pub(super) fn submit_file_manager_rename(&mut self) -> bool {
        let Some(rename) = self.state.file_manager_rename.as_ref() else {
            return false;
        };
        let Some(source_path) = rename
            .paths
            .first()
            .filter(|_| rename.paths.len() == 1)
            .cloned()
        else {
            return self
                .reject_file_manager_rename(FileManagerRenameValidationError::SourceUnavailable);
        };
        let source_is_current = !self.file_operation_worker.is_busy()
            && !self
                .state
                .file_manager_operation
                .as_ref()
                .is_some_and(FileManagerOperationState::is_running)
            && self
                .state
                .file_manager
                .as_ref()
                .is_some_and(|file_manager| {
                    file_manager
                        .entries
                        .iter()
                        .any(|entry| entry.operation_supported && entry.path == source_path)
                });
        if !source_is_current {
            return self
                .reject_file_manager_rename(FileManagerRenameValidationError::SourceUnavailable);
        }
        let Some(original_name) = source_path.file_name().and_then(|name| name.to_str()) else {
            return self
                .reject_file_manager_rename(FileManagerRenameValidationError::SourceUnavailable);
        };

        match validate_file_manager_name(
            original_name,
            &self.state.name_input,
            FileManagerRenamePlatform::current(),
        ) {
            Ok(FileManagerRenameName::Unchanged) => {
                self.state.request_file_manager_rename = None;
                true
            }
            Ok(FileManagerRenameName::Changed(new_name)) => {
                self.state.request_file_manager_rename = Some(FileManagerRenameRequest {
                    source_path,
                    new_name,
                });
                true
            }
            Err(error) => self.reject_file_manager_rename(error),
        }
    }

    fn reject_file_manager_rename(&mut self, error: FileManagerRenameValidationError) -> bool {
        if let Some(rename) = self.state.file_manager_rename.as_mut() {
            rename.validation_error = Some(error);
        }
        self.state.request_file_manager_rename = None;
        false
    }
}

#[cfg(test)]
mod tests {
    use super::{validate_file_manager_name, FileManagerRenameName, FileManagerRenamePlatform};
    use crate::app::state::{FileManagerRenameRequest, FileManagerRenameValidationError, Mode};
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
                "herdr-fm-rename-name-test-{}-{}-{}",
                std::process::id(),
                tag,
                unique()
            ));
            fs::create_dir_all(&root).expect("create rename-name test root");
            Self { root }
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn app_with_source(tag: &str) -> (TempDir, crate::app::App, PathBuf) {
        let td = TempDir::new(tag);
        let source = td.root.join("selected.txt");
        fs::write(&source, b"selected").expect("write rename-name source");
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = crate::app::App::new(
            &crate::config::Config::default(),
            true,
            None,
            api_rx,
            crate::api::EventHub::default(),
        );
        app.state.file_manager = Some(crate::fm::FmState::new(&td.root));
        (td, app, source)
    }

    // TP-C4.3-NAME: untrusted text must remain one bounded component under
    // both Unix and Windows policies; unchanged input is an explicit no-op.
    #[test]
    fn file_rename_name_validation_is_bounded_and_platform_explicit() {
        use FileManagerRenameValidationError as Error;

        for (input, expected) in [
            ("", Error::Empty),
            (".", Error::CurrentDirectory),
            ("..", Error::ParentDirectory),
            ("/tmp", Error::Absolute),
            ("dir/name", Error::Separator),
            ("nul\0byte", Error::ContainsNul),
        ] {
            assert_eq!(
                validate_file_manager_name("selected.txt", input, FileManagerRenamePlatform::Unix),
                Err(expected),
                "Unix input {input:?}"
            );
        }
        assert_eq!(
            validate_file_manager_name(
                "selected.txt",
                &"é".repeat(128),
                FileManagerRenamePlatform::Unix,
            ),
            Err(Error::NameTooLong)
        );
        assert_eq!(
            validate_file_manager_name(
                "selected.txt",
                "selected.txt",
                FileManagerRenamePlatform::Unix,
            ),
            Ok(FileManagerRenameName::Unchanged)
        );
        assert_eq!(
            validate_file_manager_name(
                "selected.txt",
                " leading and trailing ",
                FileManagerRenamePlatform::Unix,
            ),
            Ok(FileManagerRenameName::Changed(
                " leading and trailing ".to_string()
            ))
        );

        for (input, expected) in [
            ("C:\\temp", Error::Absolute),
            ("dir\\name", Error::Separator),
            ("CON", Error::WindowsReservedName),
            ("con.txt", Error::WindowsReservedName),
            ("LPT9.log", Error::WindowsReservedName),
            ("bad:name", Error::WindowsReservedCharacter),
            ("trailing.", Error::WindowsTrailingDotOrSpace),
            ("trailing ", Error::WindowsTrailingDotOrSpace),
        ] {
            assert_eq!(
                validate_file_manager_name(
                    "selected.txt",
                    input,
                    FileManagerRenamePlatform::Windows,
                ),
                Err(expected),
                "Windows input {input:?}"
            );
        }
        assert_eq!(
            validate_file_manager_name(
                "selected.txt",
                &"😀".repeat(128),
                FileManagerRenamePlatform::Windows,
            ),
            Err(Error::NameTooLong)
        );
        assert_eq!(
            validate_file_manager_name(
                "selected.txt",
                "LPT10.txt",
                FileManagerRenamePlatform::Windows,
            ),
            Ok(FileManagerRenameName::Changed("LPT10.txt".to_string()))
        );
    }

    // TP-C4.3-NAME: invalid input remains in the modal with a typed reason;
    // unchanged closes as a no-op; valid input emits only an immutable request.
    #[test]
    fn file_rename_submission_is_fail_closed_noop_or_request_only() {
        let (_td, mut app, source) = app_with_source("submission");
        assert!(app.open_file_manager_row_rename(source.clone()));
        app.state.name_input = "../escape".to_string();
        app.route_client_input(b"\r".to_vec());

        assert_eq!(app.state.mode, Mode::RenameFile);
        assert_eq!(
            app.state
                .file_manager_rename
                .as_ref()
                .expect("invalid rename remains open")
                .validation_error,
            Some(FileManagerRenameValidationError::Separator)
        );
        assert!(app.state.request_file_manager_rename.is_none());
        assert_eq!(
            fs::read(&source).expect("invalid source remains"),
            b"selected"
        );

        app.state.name_input = "selected.txt".to_string();
        app.route_client_input(b"\r".to_vec());
        assert_ne!(app.state.mode, Mode::RenameFile);
        assert!(app.state.file_manager_rename.is_none());
        assert!(app.state.request_file_manager_rename.is_none());
        assert_eq!(
            fs::read(&source).expect("unchanged source remains"),
            b"selected"
        );

        assert!(app.open_file_manager_row_rename(source.clone()));
        app.state.name_input = "renamed.txt".to_string();
        app.route_client_input(b"\r".to_vec());
        assert_eq!(
            app.state.request_file_manager_rename,
            Some(FileManagerRenameRequest {
                source_path: source.clone(),
                new_name: "renamed.txt".to_string(),
            })
        );
        assert_ne!(app.state.mode, Mode::RenameFile);
        assert!(app.state.file_manager_operation.is_none());
        assert!(source.exists());
        assert!(!source.with_file_name("renamed.txt").exists());
    }

    // TP-C4.3-NAME: a non-UTF-8 source name cannot enter the text modal and
    // cannot be normalized into a lossy rename request.
    #[cfg(unix)]
    #[test]
    fn file_rename_rejects_non_utf8_source_name_before_modal() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        let td = TempDir::new("non-utf8-source");
        let source = td.root.join(OsString::from_vec(vec![b'f', 0x80]));
        fs::write(&source, b"selected").expect("write non-UTF-8 source");
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = crate::app::App::new(
            &crate::config::Config::default(),
            true,
            None,
            api_rx,
            crate::api::EventHub::default(),
        );
        app.state.file_manager = Some(crate::fm::FmState::new(&td.root));

        assert!(!app.open_file_manager_row_rename(source.clone()));
        assert_ne!(app.state.mode, Mode::RenameFile);
        assert!(app.state.file_manager_rename.is_none());
        assert!(app.state.request_file_manager_rename.is_none());
        assert_eq!(
            fs::read(source).expect("non-UTF-8 source remains"),
            b"selected"
        );
    }
}
