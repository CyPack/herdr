//! Pure client-local authority for the root selected in Native Files.
//!
//! The current directory is deliberately absent from this model. A location
//! remains highlighted because the user selected that exact prepared item,
//! not because a later path happens to share its prefix.

use std::path::{Path, PathBuf};

use super::state::FileManagerLocationsModel;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FileManagerLocationOrigin {
    Location(PathBuf),
    Direct(PathBuf),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FileManagerLocationLoadError {
    Missing,
    PermissionDenied,
    ChangedType,
    Unavailable,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum FileManagerLocationsFocus {
    #[default]
    Trail,
    Rail,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FileManagerLocationPending {
    pub(crate) path: PathBuf,
    pub(crate) files_generation: u32,
    pub(crate) model_revision: u64,
    pub(crate) io_generation: u64,
}

// Task 2 wires this cursor outcome into the keyboard owner; keep this atomic
// model commit independently warning-free until that consumer lands.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FileManagerLocationCursorMove {
    Inert,
    Moved(PathBuf),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct FileManagerLocationsState {
    pub(crate) origin: Option<FileManagerLocationOrigin>,
    pub(crate) cursor: Option<PathBuf>,
    pub(crate) pending: Option<FileManagerLocationPending>,
    pub(crate) failure: Option<(PathBuf, FileManagerLocationLoadError)>,
    pub(crate) scroll: usize,
    pub(crate) focus: FileManagerLocationsFocus,
    drawer_open: bool,
    drawer_restore_focus: FileManagerLocationsFocus,
}

impl FileManagerLocationsState {
    pub(crate) fn activate_location(
        &mut self,
        path: &Path,
        model: &FileManagerLocationsModel,
    ) -> bool {
        if !model
            .item_for_path(path)
            .is_some_and(|item| item.accessible)
        {
            return false;
        }
        self.origin = Some(FileManagerLocationOrigin::Location(path.to_path_buf()));
        self.set_cursor(Some(path.to_path_buf()));
        self.pending = None;
        self.failure = None;
        self.focus = FileManagerLocationsFocus::Rail;
        true
    }

    pub(crate) fn activate_direct(&mut self, path: PathBuf) {
        self.origin = Some(FileManagerLocationOrigin::Direct(path));
        self.pending = None;
        self.failure = None;
        self.focus = FileManagerLocationsFocus::Trail;
    }

    pub(crate) fn highlighted_path<'a>(
        &'a self,
        model: &FileManagerLocationsModel,
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

    pub(crate) fn reconcile_model(&mut self, model: &FileManagerLocationsModel) -> bool {
        let mut changed = false;
        if let Some(FileManagerLocationOrigin::Location(path)) = self.origin.as_ref() {
            if !model
                .item_for_path(path)
                .is_some_and(|item| item.accessible)
            {
                self.origin = None;
                changed = true;
            }
        }
        self.normalize_cursor_for_rail(model) || changed
    }

    pub(crate) fn retire_for_closed_files(&mut self) {
        self.origin = None;
        self.retire_navigation_authority();
        self.scroll = 0;
        self.focus = FileManagerLocationsFocus::Trail;
        self.drawer_open = false;
        self.drawer_restore_focus = FileManagerLocationsFocus::Trail;
    }

    pub(crate) fn begin_load(
        &mut self,
        path: PathBuf,
        files_generation: u32,
        model_revision: u64,
        io_generation: u64,
    ) {
        self.pending = Some(FileManagerLocationPending {
            path,
            files_generation,
            model_revision,
            io_generation,
        });
        self.failure = None;
    }

    pub(crate) fn fail_load(&mut self, path: PathBuf, error: FileManagerLocationLoadError) {
        self.pending = None;
        self.failure = Some((path, error));
    }

    pub(crate) fn is_pending_root(
        &self,
        path: &Path,
        files_generation: u32,
        model_revision: u64,
        io_generation: u64,
    ) -> bool {
        self.pending.as_ref().is_some_and(|pending| {
            pending.path == path
                && pending.files_generation == files_generation
                && pending.model_revision == model_revision
                && pending.io_generation == io_generation
        })
    }

    pub(crate) fn scroll_rail(
        &mut self,
        delta: isize,
        content_line_count: usize,
        viewport_height: u16,
    ) -> bool {
        let maximum = content_line_count.saturating_sub(usize::from(viewport_height));
        let next = if delta < 0 {
            self.scroll.saturating_sub(delta.unsigned_abs())
        } else {
            self.scroll.saturating_add(delta as usize).min(maximum)
        }
        .min(maximum);
        let changed = next != self.scroll;
        self.scroll = next;
        self.focus = FileManagerLocationsFocus::Rail;
        changed
    }

    pub(crate) fn focus_trail(&mut self) {
        self.focus = FileManagerLocationsFocus::Trail;
    }

    pub(crate) fn drawer_is_open(&self) -> bool {
        self.drawer_open
    }

    pub(crate) fn open_drawer(&mut self) -> bool {
        if self.drawer_open {
            return false;
        }
        self.drawer_restore_focus = self.focus;
        self.drawer_open = true;
        self.focus = FileManagerLocationsFocus::Rail;
        true
    }

    pub(crate) fn close_drawer(&mut self) -> bool {
        if !self.drawer_open {
            return false;
        }
        self.drawer_open = false;
        self.focus = self.drawer_restore_focus;
        true
    }

    pub(crate) fn cursor_path<'a>(&'a self, model: &FileManagerLocationsModel) -> Option<&'a Path> {
        let path = self.cursor.as_deref()?;
        model
            .item_for_path(path)
            .is_some_and(|item| item.accessible)
            .then_some(path)
    }

    pub(crate) fn normalize_cursor_for_rail(&mut self, model: &FileManagerLocationsModel) -> bool {
        let preferred = match self.origin.as_ref() {
            Some(FileManagerLocationOrigin::Location(path)) => model
                .item_for_path(path)
                .filter(|item| item.accessible)
                .map(|item| item.path.clone()),
            Some(FileManagerLocationOrigin::Direct(_)) => {
                self.cursor_path(model).map(Path::to_path_buf)
            }
            None => None,
        };
        let next = preferred.or_else(|| {
            model
                .accessible_items()
                .next()
                .map(|item| item.path.clone())
        });
        self.set_cursor(next)
    }

    // Task 2 consumes this from the Rail input owner.
    #[allow(dead_code)]
    pub(crate) fn move_cursor(
        &mut self,
        model: &FileManagerLocationsModel,
        delta: isize,
    ) -> FileManagerLocationCursorMove {
        let current = self
            .cursor_path(model)
            .and_then(|path| model.accessible_items().position(|item| item.path == path));
        let item_count = model.accessible_items().count();
        if item_count == 0 {
            self.set_cursor(None);
            return FileManagerLocationCursorMove::Inert;
        }
        let current = current.unwrap_or(0);
        let next = current
            .saturating_add_signed(delta)
            .min(item_count.saturating_sub(1));
        if next == current && self.cursor_path(model).is_some() {
            return FileManagerLocationCursorMove::Inert;
        }
        let Some(path) = model
            .accessible_items()
            .nth(next)
            .map(|item| item.path.clone())
        else {
            return FileManagerLocationCursorMove::Inert;
        };
        self.set_cursor(Some(path.clone()));
        FileManagerLocationCursorMove::Moved(path)
    }

    // Task 2 consumes this after Rail cursor movement.
    #[allow(dead_code)]
    pub(crate) fn ensure_cursor_visible(
        &mut self,
        model: &FileManagerLocationsModel,
        viewport_height: u16,
    ) -> bool {
        let Some(path) = self.cursor_path(model) else {
            return false;
        };
        let Some(line_index) = model.line_index_for_path(path) else {
            return false;
        };
        let height = usize::from(viewport_height);
        if height == 0 {
            return false;
        }
        let maximum = model.content_line_count().saturating_sub(height);
        let next = if line_index < self.scroll {
            line_index
        } else if line_index >= self.scroll.saturating_add(height) {
            line_index.saturating_add(1).saturating_sub(height)
        } else {
            self.scroll
        }
        .min(maximum);
        let changed = next != self.scroll;
        self.scroll = next;
        changed
    }

    pub(crate) fn retire_navigation_authority(&mut self) {
        self.cursor = None;
        self.pending = None;
        self.failure = None;
    }

    fn set_cursor(&mut self, next: Option<PathBuf>) -> bool {
        if self.cursor == next {
            return false;
        }
        self.cursor = next;
        self.pending = None;
        self.failure = None;
        true
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{
        FileManagerLocationCursorMove, FileManagerLocationLoadError, FileManagerLocationOrigin,
        FileManagerLocationsState,
    };
    use crate::app::state::{
        FileManagerLocationIcon, FileManagerLocationItem, FileManagerLocationsModel,
    };

    fn item(path: &str, accessible: bool) -> FileManagerLocationItem {
        FileManagerLocationItem {
            label: Path::new(path)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or(path)
                .to_string(),
            path: PathBuf::from(path),
            icon: FileManagerLocationIcon::Pin,
            accessible,
            ejectable: false,
        }
    }

    fn model(home_accessible: bool, include_nested: bool) -> FileManagerLocationsModel {
        let mut favorites = vec![item("/home/ayaz", home_accessible)];
        if include_nested {
            favorites.push(item("/home/ayaz/projects/herdr", true));
        }
        FileManagerLocationsModel::from_sources(favorites, Vec::new(), Vec::new())
    }

    fn flf_model(include_home: bool) -> FileManagerLocationsModel {
        let mut favorites = vec![item("/workspace", true), item("/missing", false)];
        if include_home {
            favorites.push(item("/home/ayaz", true));
        }
        FileManagerLocationsModel::from_sources(
            favorites,
            vec![item("/pinned", true)],
            vec![item("/", true)],
        )
    }

    // TP-FLF-FOCUS-01: exact location authority seeds the exact cursor, while
    // a Direct descendant never invents an ancestor match.
    #[test]
    fn flf_cursor_normalizes_exact_location_without_inferred_direct_ancestor() {
        let model = flf_model(true);
        let mut exact = FileManagerLocationsState::default();
        assert!(exact.activate_location(Path::new("/home/ayaz"), &model));
        assert!(!exact.normalize_cursor_for_rail(&model));
        assert_eq!(exact.cursor_path(&model), Some(Path::new("/home/ayaz")));

        let mut direct = FileManagerLocationsState::default();
        direct.activate_direct(PathBuf::from("/home/ayaz/projects/herdr"));
        assert!(direct.normalize_cursor_for_rail(&model));
        assert_eq!(
            direct.cursor_path(&model),
            Some(Path::new("/workspace")),
            "Direct roots choose the first accessible item, not an inferred ancestor"
        );

        direct.activate_direct(PathBuf::from("/elsewhere"));
        assert!(!direct.normalize_cursor_for_rail(&model));
        assert_eq!(direct.cursor_path(&model), Some(Path::new("/workspace")));
    }

    // TP-FLF-STEP-01: cursor motion is one accessible item per event and
    // clamps without manufacturing a visible mutation at either boundary.
    #[test]
    fn flf_cursor_steps_accessible_items_one_at_a_time_and_clamps() {
        let model = flf_model(true);
        let mut state = FileManagerLocationsState::default();
        assert!(state.normalize_cursor_for_rail(&model));
        assert_eq!(state.cursor_path(&model), Some(Path::new("/workspace")));

        assert_eq!(
            state.move_cursor(&model, 1),
            FileManagerLocationCursorMove::Moved(PathBuf::from("/home/ayaz"))
        );
        assert_eq!(
            state.move_cursor(&model, 1),
            FileManagerLocationCursorMove::Moved(PathBuf::from("/pinned"))
        );
        assert_eq!(
            state.move_cursor(&model, -1),
            FileManagerLocationCursorMove::Moved(PathBuf::from("/home/ayaz"))
        );
        assert_eq!(
            state.move_cursor(&model, -99),
            FileManagerLocationCursorMove::Moved(PathBuf::from("/workspace"))
        );
        assert_eq!(
            state.move_cursor(&model, -1),
            FileManagerLocationCursorMove::Inert
        );
    }

    // TP-FLF-STALE-01: replacing the model retires request/error authority
    // tied to an obsolete cursor before selecting the new first accessible row.
    #[test]
    fn flf_cursor_reconcile_retires_obsolete_pending_and_failure() {
        let live = flf_model(true);
        let replacement = flf_model(false);
        let mut pending = FileManagerLocationsState::default();
        assert!(pending.activate_location(Path::new("/home/ayaz"), &live));
        pending.begin_load(PathBuf::from("/home/ayaz"), 7, live.revision(), 11);
        assert!(pending.reconcile_model(&replacement));
        assert_eq!(
            pending.cursor_path(&replacement),
            Some(Path::new("/workspace"))
        );
        assert_eq!(pending.pending, None);

        let mut failed = FileManagerLocationsState::default();
        assert!(failed.activate_location(Path::new("/home/ayaz"), &live));
        failed.fail_load(
            PathBuf::from("/home/ayaz"),
            FileManagerLocationLoadError::Unavailable,
        );
        assert!(failed.reconcile_model(&replacement));
        assert_eq!(
            failed.cursor_path(&replacement),
            Some(Path::new("/workspace"))
        );
        assert_eq!(failed.failure, None);
    }

    // TP-FLF-STEP-01: revealing a cursor uses the same content-line identity
    // as render, including section headers and blank separators.
    #[test]
    fn flf_cursor_scroll_reveals_exact_model_line() {
        let model = flf_model(true);
        let mut state = FileManagerLocationsState::default();
        assert!(state.activate_location(Path::new("/"), &model));
        assert!(!state.normalize_cursor_for_rail(&model));
        assert!(state.ensure_cursor_visible(&model, 2));
        assert_eq!(state.scroll, 8);
        assert!(!state.ensure_cursor_visible(&model, 2));
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
