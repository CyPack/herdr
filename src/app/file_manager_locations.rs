//! Pure client-local authority for the root selected in Native Files.
//!
//! The current directory is deliberately absent from this model. A location
//! remains highlighted because the user selected that exact prepared item,
//! not because a later path happens to share its prefix.

use std::path::{Path, PathBuf};

use super::state::FileManagerSidebarModel;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FileManagerLocationOrigin {
    Location(PathBuf),
    Direct(PathBuf),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct FileManagerLocationsState {
    pub(crate) origin: Option<FileManagerLocationOrigin>,
}

impl FileManagerLocationsState {
    pub(crate) fn activate_location(
        &mut self,
        path: &Path,
        model: &FileManagerSidebarModel,
    ) -> bool {
        if !model
            .item_for_path(path)
            .is_some_and(|item| item.accessible)
        {
            return false;
        }
        self.origin = Some(FileManagerLocationOrigin::Location(path.to_path_buf()));
        true
    }

    pub(crate) fn activate_direct(&mut self, path: PathBuf) {
        self.origin = Some(FileManagerLocationOrigin::Direct(path));
    }

    pub(crate) fn highlighted_path<'a>(
        &'a self,
        model: &FileManagerSidebarModel,
    ) -> Option<&'a Path> {
        let path = match self.origin.as_ref()? {
            FileManagerLocationOrigin::Location(path) | FileManagerLocationOrigin::Direct(path) => {
                path
            }
        };
        model
            .item_for_path(path)
            .is_some_and(|item| item.accessible)
            .then_some(path.as_path())
    }

    pub(crate) fn reconcile_model(&mut self, model: &FileManagerSidebarModel) -> bool {
        let Some(FileManagerLocationOrigin::Location(path)) = self.origin.as_ref() else {
            return false;
        };
        if model
            .item_for_path(path)
            .is_some_and(|item| item.accessible)
        {
            return false;
        }
        self.origin = None;
        true
    }

    pub(crate) fn retire_for_closed_files(&mut self) {
        self.origin = None;
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{FileManagerLocationOrigin, FileManagerLocationsState};
    use crate::app::state::{
        FileManagerSidebarIcon, FileManagerSidebarItem, FileManagerSidebarModel,
    };

    fn item(path: &str, accessible: bool) -> FileManagerSidebarItem {
        FileManagerSidebarItem {
            label: Path::new(path)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or(path)
                .to_string(),
            path: PathBuf::from(path),
            icon: FileManagerSidebarIcon::Pin,
            accessible,
            ejectable: false,
        }
    }

    fn model(home_accessible: bool, include_nested: bool) -> FileManagerSidebarModel {
        let mut favorites = vec![item("/home/ayaz", home_accessible)];
        if include_nested {
            favorites.push(item("/home/ayaz/projects/herdr", true));
        }
        FileManagerSidebarModel::from_sources(favorites, Vec::new(), Vec::new())
    }

    // TP-FCL-AUTH-01: current-directory descent is not highlight authority;
    // the explicit Home origin remains selected at any descendant depth.
    #[test]
    fn fcl_origin_location_survives_deep_descendant_navigation() {
        let model = model(true, true);
        let mut state = FileManagerLocationsState::default();

        assert!(state.activate_location(Path::new("/home/ayaz"), &model));

        assert_eq!(
            state.origin,
            Some(FileManagerLocationOrigin::Location(PathBuf::from(
                "/home/ayaz"
            )))
        );
        assert_eq!(
            state.highlighted_path(&model),
            Some(Path::new("/home/ayaz")),
            "a deeper cwd is intentionally absent from this authority check"
        );
    }

    // TP-FCL-AUTH-02: nested favorites do not compete by prefix. Only another
    // exact explicit activation transfers the single highlight.
    #[test]
    fn fcl_origin_nested_favorite_wins_only_after_explicit_activation() {
        let model = model(true, true);
        let mut state = FileManagerLocationsState::default();
        assert!(state.activate_location(Path::new("/home/ayaz"), &model));
        assert_eq!(
            state.highlighted_path(&model),
            Some(Path::new("/home/ayaz"))
        );

        assert!(state.activate_location(Path::new("/home/ayaz/projects/herdr"), &model));
        assert_eq!(
            state.highlighted_path(&model),
            Some(Path::new("/home/ayaz/projects/herdr"))
        );
    }

    // TP-FCL-AUTH-03: Direct roots do not infer an ancestor favorite. Exact
    // equality with an accessible item is the sole exception.
    #[test]
    fn fcl_origin_direct_path_never_infers_ancestor_highlight() {
        let model = model(true, true);
        let mut state = FileManagerLocationsState::default();

        state.activate_direct(PathBuf::from("/home/ayaz/projects"));
        assert_eq!(state.highlighted_path(&model), None);

        state.activate_direct(PathBuf::from("/home/ayaz"));
        assert_eq!(
            state.highlighted_path(&model),
            Some(Path::new("/home/ayaz"))
        );
    }

    // TP-FCL-AUTH-04: removed/inaccessible items and close/reopen lifecycle
    // retire stale origin instead of selecting an ancestor.
    #[test]
    fn fcl_origin_invalid_model_and_close_retire_authority() {
        let live = model(true, true);
        let inaccessible = model(false, false);
        let mut state = FileManagerLocationsState::default();
        assert!(state.activate_location(Path::new("/home/ayaz"), &live));

        assert!(state.reconcile_model(&inaccessible));
        assert_eq!(state.origin, None);
        assert_eq!(state.highlighted_path(&inaccessible), None);

        assert!(state.activate_location(Path::new("/home/ayaz"), &live));
        state.retire_for_closed_files();
        assert_eq!(state.origin, None);
        assert_eq!(state.highlighted_path(&live), None);
    }
}
