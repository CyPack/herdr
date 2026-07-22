//! Snapshot bridge for the Miller trail (trail program T2): pairs every
//! `TrailCol` with one loaded `FmDirectorySnapshot` so a visible column is by
//! construction a loaded directory — an unloaded placeholder column is
//! structurally impossible (canonical trail UX contract, LAW 1). Directory
//! selection is fail-closed: the target is read FIRST and the trail only
//! branches when the read succeeds (same policy as `prepare_navigation_io`).
//! Selection identity stays in `TrailState` as an exact path, so a watcher
//! refresh replaces entries without ever touching what is selected.

use std::path::{Path, PathBuf};

use super::trail::TrailState;
use super::{
    directory_is_writable, read_directory_snapshot, FileEntry, FmDirectorySnapshot,
    FmDirectoryStatus, FmFilePreview, FmPreview,
};
use crate::fm::entry_kind::FileEntryKind;
use crate::fm::preview_capability::{
    preview_capability, PreviewCapability, PreviewFallback, PreviewProviderSet,
};
use crate::fm::TextPreview;

/// Prepared detail-panel content for the selected FILE (contract LAW 3):
/// prepared at selection time, outside render, so the panel never does
/// filesystem work while painting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TrailDetail {
    pub path: PathBuf,
    pub kind: FileEntryKind,
    pub preview: TrailDetailPreview,
}

/// What the detail panel can show for one file. An unpreviewable file is an
/// EXPLICIT state with a reason — never a silent empty panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TrailDetailPreview {
    /// Exact text target is queued in the bounded preview worker.
    PendingText,
    Text(TextPreview),
    /// A recognized image; pixel delivery is the Kitty-graphics track
    /// (FIP-D4) and completes at integration.
    Image,
    MetadataOnly(String),
    Unpreviewable(String),
}

/// One trail column with its loaded directory listing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TrailColSnapshot {
    directory: PathBuf,
    snapshot: FmDirectorySnapshot,
    writable: bool,
}

impl TrailColSnapshot {
    fn prepared(directory: PathBuf, snapshot: FmDirectorySnapshot, writable: bool) -> Self {
        Self {
            directory,
            snapshot,
            writable,
        }
    }

    fn read(directory: &Path, show_hidden: bool) -> Self {
        let snapshot = read_directory_snapshot(directory, show_hidden);
        let writable =
            snapshot.status == FmDirectoryStatus::Available && directory_is_writable(directory);
        Self::prepared(directory.to_path_buf(), snapshot, writable)
    }

    pub(crate) fn directory(&self) -> &Path {
        &self.directory
    }

    pub(crate) fn entries(&self) -> &[FileEntry] {
        &self.snapshot.entries
    }

    pub(crate) fn status(&self) -> FmDirectoryStatus {
        self.snapshot.status
    }

    pub(crate) fn writable(&self) -> bool {
        self.writable
    }

    pub(crate) fn omission_message(&self) -> Option<&'static str> {
        if self.snapshot.omissions.hidden > 0 {
            Some("hidden items omitted")
        } else if self.snapshot.omissions.non_utf8 > 0 {
            Some("unreadable names omitted")
        } else if self.snapshot.omissions.entry_errors > 0 {
            Some("some entries unreadable")
        } else {
            None
        }
    }
}

/// Outcome of one input activation against the loaded trail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TrailActivateOutcome {
    /// A directory was selected: the trail truncated and branched.
    Branched,
    /// A file was selected: the selection moved, no column appeared.
    SelectedFile,
    /// The hit was stale, out of range, or the target was unreadable —
    /// nothing changed.
    Rejected,
}

/// Outcome of one cursor-only vertical movement. Unlike activation, a moved
/// directory never owns Trail branching or active-column transfer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TrailCursorMoveOutcome {
    Moved { entry_index: usize, directory: bool },
    Unchanged { entry_index: usize, directory: bool },
    Rejected,
}

impl TrailCursorMoveOutcome {
    pub(crate) fn is_rejected(self) -> bool {
        self == Self::Rejected
    }

    pub(crate) fn changed(self) -> bool {
        matches!(self, Self::Moved { .. })
    }

    pub(crate) fn entry_index(self) -> Option<usize> {
        match self {
            Self::Moved { entry_index, .. } | Self::Unchanged { entry_index, .. } => {
                Some(entry_index)
            }
            Self::Rejected => None,
        }
    }

    pub(crate) fn is_directory(self) -> bool {
        matches!(
            self,
            Self::Moved {
                directory: true,
                ..
            } | Self::Unchanged {
                directory: true,
                ..
            }
        )
    }
}

/// Loaded snapshots kept index-aligned with a `TrailState`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TrailSnapshots {
    cols: Vec<TrailColSnapshot>,
    show_hidden: bool,
    detail: Option<TrailDetail>,
}

impl TrailSnapshots {
    /// Start a fresh cursor transaction in the exact active column using only
    /// resident snapshots. Deeper destination history and detail authority
    /// are retired before callers choose the first actionable row.
    pub(crate) fn reset_active_column_cursor(&mut self, trail: &mut TrailState) -> bool {
        let active_col = trail.active_col();
        let aligned = self.cols.get(active_col).is_some_and(|snapshot| {
            trail
                .cols()
                .get(active_col)
                .is_some_and(|column| column.directory == snapshot.directory)
        });
        if !aligned {
            return false;
        }
        self.cols.truncate(active_col + 1);
        self.detail = None;
        trail.clear_selection_at(active_col)
    }

    /// Reset to an already loaded root without touching the filesystem.
    pub(crate) fn reset_to_resident_root(
        &mut self,
        trail: &mut TrailState,
        expected_root: &Path,
    ) -> bool {
        if trail.cols().first().map(|col| col.directory.as_path()) != Some(expected_root)
            || self.cols.first().map(|col| col.directory.as_path()) != Some(expected_root)
            || !trail.clear_selection_at(0)
        {
            return false;
        }
        self.cols.truncate(1);
        self.detail = None;
        true
    }

    pub(crate) fn new(show_hidden: bool) -> Self {
        Self {
            cols: Vec::new(),
            show_hidden,
            detail: None,
        }
    }

    pub(crate) fn cols(&self) -> &[TrailColSnapshot] {
        &self.cols
    }

    /// The prepared detail panel for the currently selected FILE, when one
    /// is selected (LAW 3). A directory selection has no detail.
    pub(crate) fn detail(&self) -> Option<&TrailDetail> {
        self.detail.as_ref()
    }

    /// Resolve one exact pending text detail from prepared worker output.
    /// A newer selection or non-text detail remains untouched.
    pub(crate) fn apply_prepared_text_detail(
        &mut self,
        expected_path: &Path,
        prepared: &Result<TextPreview, super::TextPreviewError>,
    ) -> bool {
        let Some(detail) = self.detail.as_mut() else {
            return false;
        };
        if detail.path != expected_path
            || detail.preview != TrailDetailPreview::PendingText
            || prepared
                .as_ref()
                .is_ok_and(|preview| preview.source_path != expected_path)
        {
            return false;
        }
        detail.preview = match prepared {
            Ok(preview) => TrailDetailPreview::Text(preview.clone()),
            Err(error) => TrailDetailPreview::Unpreviewable(error.to_string()),
        };
        true
    }

    /// Resolve the trail's single exact-path selection through the
    /// index-aligned loaded snapshots. The deepest marked column wins, while
    /// the unselected child column after a directory branch is skipped.
    pub(crate) fn selected_entry<'a>(&'a self, trail: &TrailState) -> Option<&'a FileEntry> {
        if let Some((col_idx, cursor_path)) = trail.cursor_override() {
            return self
                .cols
                .get(col_idx)?
                .entries()
                .iter()
                .find(|entry| entry.path == cursor_path);
        }
        trail
            .cols()
            .iter()
            .zip(&self.cols)
            .rev()
            .find_map(|(trail_col, snapshot)| {
                let selected = trail_col.selected.as_deref()?;
                snapshot
                    .entries()
                    .iter()
                    .find(|entry| entry.path == selected)
            })
    }

    /// Rebuild the transitional FmState bridge from already-prepared current
    /// directory and preview data. This is intentionally disk-free so typed
    /// refresh/navigation apply remains a pure model transition.
    pub(super) fn integrate_current(
        &mut self,
        trail: &mut TrailState,
        root: &Path,
        root_snapshot: FmDirectorySnapshot,
        root_writable: bool,
        selected: Option<&FileEntry>,
        preview: &FmPreview,
        show_hidden: bool,
    ) {
        self.show_hidden = show_hidden;
        self.detail = None;

        let mut col_idx = trail.cols().iter().rposition(|col| col.directory == root);
        let prefix_is_aligned = col_idx.is_some_and(|idx| {
            self.cols
                .get(idx)
                .is_some_and(|snapshot| snapshot.directory == root)
                || (idx == 0 && self.cols.is_empty())
        });
        if !prefix_is_aligned {
            *trail = TrailState::new(root);
            self.cols.clear();
            col_idx = Some(0);
        }
        let mut col_idx = col_idx.unwrap_or(0);
        if !trail.clear_selection_at(col_idx) {
            *trail = TrailState::new(root);
            self.cols.clear();
            col_idx = 0;
        }
        self.cols.truncate(col_idx + 1);
        let current = TrailColSnapshot::prepared(root.to_path_buf(), root_snapshot, root_writable);
        if col_idx < self.cols.len() {
            self.cols[col_idx] = current;
        } else {
            self.cols.push(current);
        }
        let Some(selected) = selected else {
            return;
        };

        match preview {
            FmPreview::Directory(entries) if selected.is_dir() => {
                if trail.select_dir(col_idx, &selected.path) {
                    self.cols.push(TrailColSnapshot::prepared(
                        selected.path.clone(),
                        FmDirectorySnapshot {
                            entries: entries.clone(),
                            status: FmDirectoryStatus::Available,
                            omissions: Default::default(),
                        },
                        false,
                    ));
                    // The prepared child is a Miller preview, not an
                    // implicit Right/Enter/click. Keep keyboard/wheel focus
                    // on the directory column that owns the cursor.
                    let moved_to_owner = trail.move_active_left();
                    debug_assert!(moved_to_owner, "preview child must have an owner column");
                }
            }
            FmPreview::File(file_preview) if !selected.is_dir() => {
                if trail.select_file(col_idx, &selected.path) {
                    self.detail = Some(TrailDetail {
                        path: selected.path.clone(),
                        kind: selected.kind,
                        preview: match file_preview {
                            FmFilePreview::PendingText { .. } => TrailDetailPreview::PendingText,
                            FmFilePreview::Text(preview) => {
                                TrailDetailPreview::Text(preview.clone())
                            }
                            FmFilePreview::Image(_) => TrailDetailPreview::Image,
                            FmFilePreview::Unavailable(error) => {
                                TrailDetailPreview::Unpreviewable(error.to_string())
                            }
                        },
                    });
                }
            }
            FmPreview::None | FmPreview::Directory(_) | FmPreview::File(_) => {
                // A mismatched/failed prepared preview must not invent a
                // visible child column. Retain only the exact selected path.
                let _ = trail.select_file(col_idx, &selected.path);
            }
        }
    }

    /// Realign with the trail: keep a cached snapshot only when the same
    /// index still lists the same directory, otherwise load it fresh. This
    /// single pass covers append, truncate, and the sliding-window drop.
    pub(crate) fn sync(&mut self, trail: &TrailState) {
        let cols = trail.cols();
        self.cols.truncate(cols.len());
        for (idx, col) in cols.iter().enumerate() {
            let cached = self
                .cols
                .get(idx)
                .is_some_and(|snap| snap.directory == col.directory);
            if cached {
                continue;
            }
            let loaded = TrailColSnapshot::read(&col.directory, self.show_hidden);
            if idx < self.cols.len() {
                self.cols[idx] = loaded;
            } else {
                self.cols.push(loaded);
            }
        }
    }

    /// Fail-closed folder selection: read the target first; branch the trail
    /// and append the loaded column only when the directory is `Available`.
    /// Out-of-range columns change nothing and report `Unavailable`.
    pub(crate) fn select_dir(
        &mut self,
        trail: &mut TrailState,
        col_idx: usize,
        child: &Path,
    ) -> FmDirectoryStatus {
        if col_idx >= trail.cols().len() {
            return FmDirectoryStatus::Unavailable;
        }
        let loaded = self
            .cols
            .get(col_idx + 1)
            .filter(|snapshot| snapshot.directory == child)
            .cloned()
            .unwrap_or_else(|| TrailColSnapshot::read(child, self.show_hidden));
        if loaded.status() != FmDirectoryStatus::Available {
            return loaded.status();
        }
        if !trail.select_dir(col_idx, child) {
            return FmDirectoryStatus::Unavailable;
        }
        // Mirror the trail transition exactly: truncate to the branch point,
        // append the pre-read column, then realign for the sliding window.
        self.cols.truncate(col_idx + 1);
        self.cols.push(loaded);
        if self.cols.len() > trail.cols().len() {
            self.cols.remove(0);
        }
        self.sync(trail);
        FmDirectoryStatus::Available
    }

    /// Activate one entry (mouse click or keyboard Enter) against the CURRENT
    /// loaded listing. `expected_path` is the path the input frame saw; a
    /// mismatch with the live snapshot means the hit is stale and is
    /// rejected without touching any state (generation-safe hits).
    /// Directories branch the trail (fail-closed via `select_dir`); files
    /// only mark the selection (LAW 1/3).
    pub(crate) fn activate_entry(
        &mut self,
        trail: &mut TrailState,
        col_idx: usize,
        entry_index: usize,
        expected_path: &Path,
    ) -> TrailActivateOutcome {
        let Some(entry) = self
            .cols
            .get(col_idx)
            .and_then(|col| col.snapshot.entries.get(entry_index))
        else {
            return TrailActivateOutcome::Rejected;
        };
        if entry.path != expected_path {
            return TrailActivateOutcome::Rejected;
        }
        if entry.kind.is_directory_target() {
            let target = entry.path.clone();
            if self.select_dir(trail, col_idx, &target) == FmDirectoryStatus::Available {
                // A directory selection owns the stage again: the file
                // panel can never dangle across a branch (LAW 3).
                self.detail = None;
                TrailActivateOutcome::Branched
            } else {
                TrailActivateOutcome::Rejected
            }
        } else {
            let target = entry.path.clone();
            let kind = entry.kind;
            if trail.select_file(col_idx, &target) {
                self.sync(trail);
                self.detail = Some(prepare_trail_detail(&target, kind));
                TrailActivateOutcome::SelectedFile
            } else {
                TrailActivateOutcome::Rejected
            }
        }
    }

    /// Move only the cursor inside the ACTIVE column. Directory activation and
    /// optional preview scheduling are separate commands owned by callers.
    pub(crate) fn move_cursor(
        &mut self,
        trail: &mut TrailState,
        delta: isize,
    ) -> TrailCursorMoveOutcome {
        let col_idx = trail.active_col();
        self.move_cursor_in_column(trail, col_idx, delta)
    }

    /// Step the cursor inside one exact loaded column without touching the
    /// activated branch. Header/row wheel input uses this seam so geometry can
    /// never fall back to a different active or horizontally visible column.
    pub(crate) fn move_cursor_in_column(
        &mut self,
        trail: &mut TrailState,
        col_idx: usize,
        delta: isize,
    ) -> TrailCursorMoveOutcome {
        let Some(col) = self.cols.get(col_idx) else {
            return TrailCursorMoveOutcome::Rejected;
        };
        let entries = &col.snapshot.entries;
        if entries.is_empty() {
            return TrailCursorMoveOutcome::Rejected;
        }
        let current = trail
            .cursor_path_in_col(col_idx)
            .and_then(|selected| entries.iter().position(|entry| entry.path == selected));
        let landed = match current {
            Some(index) => index
                .saturating_add_signed(delta)
                .min(entries.len().saturating_sub(1)),
            // No selection yet: the first step lands on the edge row.
            None if delta >= 0 => 0,
            None => entries.len() - 1,
        };
        let entry = &entries[landed];
        let directory = entry.kind.is_directory_target();
        if current == Some(landed) {
            return TrailCursorMoveOutcome::Unchanged {
                entry_index: landed,
                directory,
            };
        }
        let target = entry.path.clone();
        let kind = entry.kind;
        if !trail.move_cursor_to(col_idx, &target) {
            return TrailCursorMoveOutcome::Rejected;
        }
        self.detail = (!directory).then(|| prepare_trail_detail(&target, kind));
        TrailCursorMoveOutcome::Moved {
            entry_index: landed,
            directory,
        }
    }

    /// LAW 5 sidebar/deep-link entry: build a FRESH trail rooted at `root`,
    /// then descend the ancestor chain toward `target`, selecting every
    /// directory on the way; a file target ends as the selected file with
    /// its detail prepared. Fail-closed: an unreadable root returns None; a
    /// target outside the root or an unreadable middle component stops the
    /// descent honestly at the last loadable column.
    pub(crate) fn open_trail_to(&mut self, root: &Path, target: &Path) -> Option<TrailState> {
        let root_snapshot = read_directory_snapshot(root, self.show_hidden);
        if root_snapshot.status != FmDirectoryStatus::Available {
            return None;
        }
        self.cols.clear();
        self.detail = None;
        let root_writable = directory_is_writable(root);
        self.cols.push(TrailColSnapshot::prepared(
            root.to_path_buf(),
            root_snapshot,
            root_writable,
        ));
        let mut trail = TrailState::new(root);

        let Ok(relative) = target.strip_prefix(root) else {
            // A target outside the root never descends anywhere.
            return Some(trail);
        };
        let mut current = root.to_path_buf();
        for component in relative.components() {
            current = current.join(component);
            let deepest = trail.deepest();
            let Some(entry_index) = self.cols[deepest]
                .entries()
                .iter()
                .position(|entry| entry.path == current)
            else {
                break;
            };
            let expected = current.clone();
            if self.activate_entry(&mut trail, deepest, entry_index, &expected)
                == TrailActivateOutcome::Rejected
            {
                break;
            }
        }
        Some(trail)
    }

    /// Re-read one column from disk (watcher refresh path). Selection lives
    /// in the trail as an exact path and is never touched here. The column
    /// keeps its explicit status even when the directory disappeared —
    /// honest state, never a silent placeholder.
    pub(crate) fn refresh_col(&mut self, col_idx: usize) -> bool {
        let Some(directory) = self
            .cols
            .get(col_idx)
            .map(|col| col.directory.to_path_buf())
        else {
            return false;
        };
        self.cols[col_idx] = TrailColSnapshot::read(&directory, self.show_hidden);
        true
    }

    /// Reconcile exact-path Trail authority after one loaded column changed.
    /// A still-live directory branch is preserved, a still-live file detail
    /// is freshly prepared, and a missing/type-changed selection truncates
    /// atomically at the refreshed column.
    pub(crate) fn reconcile_refreshed_col(
        &mut self,
        trail: &mut TrailState,
        col_idx: usize,
    ) -> bool {
        let Some(trail_col) = trail.cols().get(col_idx) else {
            return false;
        };
        let Some(snapshot) = self.cols.get(col_idx) else {
            return false;
        };
        if snapshot.directory != trail_col.directory {
            return false;
        }

        let Some(selected_path) = trail_col.selected.clone() else {
            if col_idx < trail.deepest() {
                let _ = trail.clear_selection_at(col_idx);
                self.cols.truncate(col_idx + 1);
                self.detail = None;
            }
            return true;
        };
        let selected = snapshot
            .entries()
            .iter()
            .find(|entry| entry.path == selected_path)
            .cloned();
        let Some(selected) = selected else {
            let _ = trail.clear_selection_at(col_idx);
            self.cols.truncate(col_idx + 1);
            self.detail = None;
            return true;
        };

        if selected.is_dir() {
            let branch_is_aligned = trail
                .cols()
                .get(col_idx + 1)
                .is_some_and(|next| next.directory == selected.path)
                && self
                    .cols
                    .get(col_idx + 1)
                    .is_some_and(|next| next.directory == selected.path);
            if !branch_is_aligned {
                let _ = trail.clear_selection_at(col_idx);
                self.cols.truncate(col_idx + 1);
                self.detail = None;
            }
            return true;
        }

        if !trail.select_file(col_idx, &selected.path) {
            return false;
        }
        self.cols.truncate(col_idx + 1);
        self.detail = Some(prepare_trail_detail(&selected.path, selected.kind));
        true
    }
}

/// Prepare the detail-panel content for one selected file, outside render:
/// image paths take the image track (Kitty delivery is FIP-D4), everything
/// else is a bounded text preview or an EXPLICIT unpreviewable reason.
fn prepare_trail_detail(path: &Path, kind: FileEntryKind) -> TrailDetail {
    let preview = match preview_capability(path, kind, &PreviewProviderSet::default()) {
        PreviewCapability::NativeImage => TrailDetailPreview::Image,
        PreviewCapability::NativeText => TrailDetailPreview::PendingText,
        PreviewCapability::MetadataOnly { reason } => {
            TrailDetailPreview::MetadataOnly(reason.label().to_owned())
        }
        PreviewCapability::OptionalPlugin { fallback, .. } => match fallback {
            PreviewFallback::NativeText => TrailDetailPreview::PendingText,
            PreviewFallback::MetadataOnly(reason) => {
                TrailDetailPreview::MetadataOnly(reason.label().to_owned())
            }
        },
        PreviewCapability::Unsupported { reason } => {
            TrailDetailPreview::Unpreviewable(reason.label().to_owned())
        }
    };
    TrailDetail {
        path: path.to_path_buf(),
        kind,
        preview,
    }
}

#[cfg(test)]
mod tests {
    use super::super::trail::MAX_TRAIL_DEPTH;
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn unique() -> u64 {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        NEXT.fetch_add(1, Ordering::Relaxed)
    }

    /// Isolated temp directory, recursively removed on drop. Never touches
    /// any real user directory.
    struct TempDir {
        root: PathBuf,
    }

    impl TempDir {
        fn new(tag: &str) -> Self {
            let root = std::env::temp_dir().join(format!(
                "herdr-trail-test-{}-{}-{}",
                std::process::id(),
                tag,
                unique()
            ));
            fs::create_dir_all(&root).expect("create temp root");
            Self { root }
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn entry_names(col: &TrailColSnapshot) -> Vec<&str> {
        col.entries().iter().map(|e| e.name.as_str()).collect()
    }

    // LAW 1: after sync + folder selects, every visible column carries a
    // loaded Available snapshot whose directory matches the trail exactly —
    // a placeholder column cannot exist.
    #[test]
    fn every_visible_column_is_loaded() {
        let td = TempDir::new("loaded");
        let a = td.root.join("a");
        let b = a.join("b");
        fs::create_dir_all(&b).expect("create nested dirs");
        fs::write(a.join("x.txt"), b"x").expect("write file");
        for path in [&b, &a.join("x.txt")] {
            fs::File::open(path)
                .expect("open loaded-column mtime fixture")
                .set_times(
                    fs::FileTimes::new()
                        .set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(10)),
                )
                .expect("set loaded-column fixture mtime");
        }

        let mut trail = TrailState::new(&td.root);
        let mut snaps = TrailSnapshots::new(false);
        snaps.sync(&trail);
        assert_eq!(snaps.cols().len(), 1, "root column is loaded on sync");

        assert_eq!(
            snaps.select_dir(&mut trail, 0, &a),
            FmDirectoryStatus::Available
        );
        assert_eq!(
            snaps.select_dir(&mut trail, 1, &b),
            FmDirectoryStatus::Available
        );

        assert_eq!(trail.cols().len(), 3);
        assert_eq!(snaps.cols().len(), 3, "one snapshot per trail column");
        for (idx, col) in snaps.cols().iter().enumerate() {
            assert_eq!(
                col.directory(),
                trail.cols()[idx].directory.as_path(),
                "column {idx} snapshot lists the trail directory"
            );
            assert_eq!(
                col.status(),
                FmDirectoryStatus::Available,
                "column {idx} is loaded"
            );
        }
        assert_eq!(entry_names(&snaps.cols()[0]), vec!["a"]);
        assert_eq!(entry_names(&snaps.cols()[1]), vec!["b", "x.txt"]);
    }

    // Fail-closed: an unreadable target reports its status and neither the
    // trail nor the snapshots change — the broken column never appears.
    #[test]
    fn unreadable_target_never_becomes_a_column() {
        let td = TempDir::new("missing");
        let mut trail = TrailState::new(&td.root);
        let mut snaps = TrailSnapshots::new(false);
        snaps.sync(&trail);

        let missing = td.root.join("does-not-exist");
        assert_eq!(
            snaps.select_dir(&mut trail, 0, &missing),
            FmDirectoryStatus::Missing
        );
        assert_eq!(trail.cols().len(), 1, "trail did not branch");
        assert_eq!(trail.cols()[0].selected, None, "nothing was selected");
        assert_eq!(snaps.cols().len(), 1, "no snapshot column appeared");
    }

    // Exact-path selection identity: a watcher-style refresh reloads the
    // entries but the selected path in the trail is untouched.
    #[test]
    fn watcher_refresh_keeps_selection_by_path() {
        let td = TempDir::new("refresh");
        fs::write(td.root.join("keep.txt"), b"x").expect("write file");
        fs::File::open(td.root.join("keep.txt"))
            .expect("open keep mtime fixture")
            .set_times(
                fs::FileTimes::new()
                    .set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(10)),
            )
            .expect("set keep mtime");

        let mut trail = TrailState::new(&td.root);
        let mut snaps = TrailSnapshots::new(false);
        snaps.sync(&trail);

        let keep = td.root.join("keep.txt");
        assert!(trail.select_file(0, &keep));

        fs::write(td.root.join("new.txt"), b"x").expect("write new file");
        fs::File::open(td.root.join("new.txt"))
            .expect("open new mtime fixture")
            .set_times(
                fs::FileTimes::new()
                    .set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(20)),
            )
            .expect("set new mtime");
        assert!(snaps.refresh_col(0), "refresh reloads the column");

        assert_eq!(
            trail.cols()[0].selected.as_deref(),
            Some(keep.as_path()),
            "selection is exact-path and survives the refresh"
        );
        assert_eq!(entry_names(&snaps.cols()[0]), vec!["new.txt", "keep.txt"]);
    }

    // Bounded: past MAX_TRAIL_DEPTH the trail slides and the snapshots stay
    // index-aligned with it at every step.
    #[test]
    fn sliding_window_keeps_snapshots_aligned() {
        let td = TempDir::new("bounded");
        let mut deepest_dir = td.root.clone();
        for i in 0..MAX_TRAIL_DEPTH + 5 {
            deepest_dir = deepest_dir.join(format!("d{i}"));
        }
        fs::create_dir_all(&deepest_dir).expect("create deep tree");

        let mut trail = TrailState::new(&td.root);
        let mut snaps = TrailSnapshots::new(false);
        snaps.sync(&trail);

        let mut next = td.root.clone();
        for i in 0..MAX_TRAIL_DEPTH + 5 {
            next = next.join(format!("d{i}"));
            let deepest = trail.deepest();
            assert_eq!(
                snaps.select_dir(&mut trail, deepest, &next),
                FmDirectoryStatus::Available
            );
            assert!(snaps.cols().len() <= MAX_TRAIL_DEPTH);
            assert_eq!(snaps.cols().len(), trail.cols().len());
            for (idx, col) in snaps.cols().iter().enumerate() {
                assert_eq!(
                    col.directory(),
                    trail.cols()[idx].directory.as_path(),
                    "column {idx} stays aligned after sliding"
                );
            }
        }
    }

    // LAW 1 via input: activating a FOLDER row branches the trail at the
    // hit column with click semantics.
    #[test]
    fn folder_activation_branches_at_hit_column() {
        let td = TempDir::new("act-dir");
        let sub = td.root.join("sub");
        fs::create_dir_all(&sub).expect("sub dir");
        let mut trail = TrailState::new(&td.root);
        let mut snaps = TrailSnapshots::new(false);
        snaps.sync(&trail);

        let entry_index = snaps.cols()[0]
            .entries()
            .iter()
            .position(|entry| entry.path == sub)
            .expect("sub row");
        assert_eq!(
            snaps.activate_entry(&mut trail, 0, entry_index, &sub),
            TrailActivateOutcome::Branched
        );
        assert_eq!(trail.cols().len(), 2);
        assert_eq!(trail.cols()[1].directory, sub);
        assert_eq!(snaps.cols().len(), 2, "the branched column is loaded");
    }

    // LAW 1 via input: activating a sibling in an ANCESTOR column discards
    // the old branch and regrows from that point.
    #[test]
    fn ancestor_sibling_activation_rebranches() {
        let td = TempDir::new("act-sibling");
        let a = td.root.join("a");
        let b = a.join("b");
        let z = td.root.join("z");
        fs::create_dir_all(&b).expect("nested");
        fs::create_dir_all(&z).expect("sibling");
        let mut trail = TrailState::new(&td.root);
        let mut snaps = TrailSnapshots::new(false);
        snaps.sync(&trail);
        assert_eq!(
            snaps.select_dir(&mut trail, 0, &a),
            FmDirectoryStatus::Available
        );
        assert_eq!(
            snaps.select_dir(&mut trail, 1, &b),
            FmDirectoryStatus::Available
        );

        let z_index = snaps.cols()[0]
            .entries()
            .iter()
            .position(|entry| entry.path == z)
            .expect("z row");
        assert_eq!(
            snaps.activate_entry(&mut trail, 0, z_index, &z),
            TrailActivateOutcome::Branched
        );
        assert_eq!(trail.cols().len(), 2, "old sub-branch discarded");
        assert_eq!(trail.cols()[1].directory, z);
        assert_eq!(snaps.cols()[1].directory(), z.as_path());
    }

    // LAW 1/3 via input: activating a FILE marks the selection and never
    // opens a column.
    #[test]
    fn file_activation_marks_selection_only() {
        let td = TempDir::new("act-file");
        fs::write(td.root.join("doc.md"), b"x").expect("file");
        let mut trail = TrailState::new(&td.root);
        let mut snaps = TrailSnapshots::new(false);
        snaps.sync(&trail);

        let doc = td.root.join("doc.md");
        assert_eq!(
            snaps.activate_entry(&mut trail, 0, 0, &doc),
            TrailActivateOutcome::SelectedFile
        );
        assert_eq!(trail.cols().len(), 1, "a file never opens a column");
        assert_eq!(trail.cols()[0].selected.as_deref(), Some(doc.as_path()));
    }

    // Generation-safe hits: a hit whose remembered path no longer matches
    // the live snapshot at that index is stale and rejected outright.
    #[test]
    fn stale_hit_with_mismatched_path_is_rejected() {
        let td = TempDir::new("act-stale");
        fs::write(td.root.join("old.txt"), b"x").expect("file");
        let mut trail = TrailState::new(&td.root);
        let mut snaps = TrailSnapshots::new(false);
        snaps.sync(&trail);
        let before = trail.clone();

        // The input frame remembered a path that is not what row 0 lists now.
        let remembered = td.root.join("vanished.txt");
        assert_eq!(
            snaps.activate_entry(&mut trail, 0, 0, &remembered),
            TrailActivateOutcome::Rejected
        );
        assert_eq!(trail, before, "stale hits change nothing");
        // Out-of-range indexes are equally inert.
        assert_eq!(
            snaps.activate_entry(&mut trail, 0, 99, &remembered),
            TrailActivateOutcome::Rejected
        );
        assert_eq!(
            snaps.activate_entry(&mut trail, 7, 0, &remembered),
            TrailActivateOutcome::Rejected
        );
        assert_eq!(trail, before);
    }

    // FMN cursor law: Down/Up move inside the ACTIVE column without activating
    // either a directory or a file.
    #[test]
    fn keyboard_selection_moves_within_active_column() {
        let td = TempDir::new("kbd");
        let alpha = td.root.join("alpha");
        fs::create_dir_all(&alpha).expect("alpha");
        let beta = td.root.join("beta.txt");
        fs::write(&beta, b"x").expect("beta");
        for path in [&alpha, &beta] {
            fs::File::open(path)
                .expect("open keyboard-selection mtime fixture")
                .set_times(
                    fs::FileTimes::new()
                        .set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(10)),
                )
                .expect("set keyboard-selection fixture mtime");
        }
        let mut trail = TrailState::new(&td.root);
        let mut snaps = TrailSnapshots::new(false);
        snaps.sync(&trail);

        // First Down lands on row 0 without entering the directory.
        assert_eq!(
            snaps.move_cursor(&mut trail, 1),
            TrailCursorMoveOutcome::Moved {
                entry_index: 0,
                directory: true,
            }
        );
        assert_eq!(trail.cols().len(), 1);
        assert_eq!(trail.cols()[0].selected, None);
        assert_eq!(
            snaps
                .selected_entry(&trail)
                .map(|entry| entry.path.as_path()),
            Some(alpha.as_path())
        );

        // The next Down remains in the root column and lands on the file.
        assert_eq!(
            snaps.move_cursor(&mut trail, 1),
            TrailCursorMoveOutcome::Moved {
                entry_index: 1,
                directory: false,
            }
        );
        assert_eq!(
            snaps
                .selected_entry(&trail)
                .map(|entry| entry.path.as_path()),
            Some(td.root.join("beta.txt").as_path())
        );
        assert_eq!(trail.cols().len(), 1);
        assert_eq!(trail.active_col(), 0);

        // Clamped at the listing edge: stepping past the end stays put.
        assert_eq!(
            snaps.move_cursor(&mut trail, 5),
            TrailCursorMoveOutcome::Unchanged {
                entry_index: 1,
                directory: false,
            }
        );
        assert_eq!(
            snaps
                .selected_entry(&trail)
                .map(|entry| entry.path.as_path()),
            Some(td.root.join("beta.txt").as_path())
        );
    }

    // TP-FMN-NAV-01/02 + TP-FMN-IO-01 RED: vertical movement is a cursor
    // command, not directory activation. Landing on a sibling directory must
    // leave the already-open branch and the owner-column focus untouched while
    // exposing the landed exact path as the current entry.
    #[test]
    fn vertical_directory_landing_preserves_owner_focus_and_open_branch() {
        let td = TempDir::new("cursor-only-directory-landing");
        for name in ["00-alpha", "01-bravo", "02-current"] {
            fs::create_dir(td.root.join(name)).expect("create directory fixture");
        }
        let modified = std::time::UNIX_EPOCH + std::time::Duration::from_secs(10);
        for name in ["00-alpha", "01-bravo", "02-current"] {
            fs::File::open(td.root.join(name))
                .expect("open directory fixture")
                .set_times(fs::FileTimes::new().set_modified(modified))
                .expect("set deterministic directory mtime");
        }

        let current = td.root.join("02-current");
        let mut trail = TrailState::new(&td.root);
        let mut snaps = TrailSnapshots::new(false);
        snaps.sync(&trail);
        let current_index = snaps.cols()[0]
            .entries()
            .iter()
            .position(|entry| entry.path == current)
            .expect("current directory row");
        assert_eq!(
            snaps.activate_entry(&mut trail, 0, current_index, &current),
            TrailActivateOutcome::Branched
        );
        assert!(trail.move_active_left());
        let before_columns = trail.cols().to_vec();
        let before_snapshot_directories = snaps
            .cols()
            .iter()
            .map(|snapshot| snapshot.directory().to_path_buf())
            .collect::<Vec<_>>();

        let _ = snaps.move_cursor(&mut trail, -1);

        assert_eq!(trail.active_col(), 0, "vertical movement keeps owner focus");
        assert_eq!(
            trail.cols(),
            before_columns,
            "vertical movement neither truncates nor replaces the open branch"
        );
        assert_eq!(
            snaps
                .cols()
                .iter()
                .map(|snapshot| snapshot.directory().to_path_buf())
                .collect::<Vec<_>>(),
            before_snapshot_directories,
            "cursor movement performs no directory activation/read"
        );
        assert_eq!(
            snaps
                .selected_entry(&trail)
                .map(|entry| entry.path.as_path()),
            Some(td.root.join("01-bravo").as_path()),
            "the landed exact path becomes the current cursor entry"
        );
    }

    // LAW 3 / FM-PERF-TEXT-02: activating a FILE prepares exact path and kind
    // immediately, but byte loading remains an explicit worker-owned state.
    #[test]
    fn file_selection_prepares_detail() {
        let td = TempDir::new("detail");
        let doc = td.root.join("doc.md");
        fs::write(&doc, b"hello trail").expect("file");
        let mut trail = TrailState::new(&td.root);
        let mut snaps = TrailSnapshots::new(false);
        snaps.sync(&trail);

        assert_eq!(
            snaps.activate_entry(&mut trail, 0, 0, &doc),
            TrailActivateOutcome::SelectedFile
        );
        let detail = snaps.detail().expect("file selection prepares a detail");
        assert_eq!(detail.path, doc);
        assert_eq!(
            detail.kind,
            crate::fm::entry_kind::FileEntryKind::RegularFile
        );
        assert_eq!(detail.preview, TrailDetailPreview::PendingText);
    }

    // LAW 3: a later DIRECTORY selection closes the panel — the detail can
    // never dangle across a branch.
    #[test]
    fn dir_selection_clears_detail() {
        let td = TempDir::new("detail-clear");
        let sub = td.root.join("sub");
        fs::create_dir_all(&sub).expect("sub");
        let doc = td.root.join("doc.md");
        fs::write(&doc, b"x").expect("file");
        let mut trail = TrailState::new(&td.root);
        let mut snaps = TrailSnapshots::new(false);
        snaps.sync(&trail);

        let doc_index = snaps.cols()[0]
            .entries()
            .iter()
            .position(|entry| entry.path == doc)
            .expect("doc row");
        assert_eq!(
            snaps.activate_entry(&mut trail, 0, doc_index, &doc),
            TrailActivateOutcome::SelectedFile
        );
        assert!(snaps.detail().is_some());

        let sub_index = snaps.cols()[0]
            .entries()
            .iter()
            .position(|entry| entry.path == sub)
            .expect("sub row");
        assert_eq!(
            snaps.activate_entry(&mut trail, 0, sub_index, &sub),
            TrailActivateOutcome::Branched
        );
        assert!(
            snaps.detail().is_none(),
            "a directory selection closes the panel"
        );
    }

    // Binary types are classified before any text read, so native detail does
    // not misrepresent them as corrupt text.
    #[test]
    fn binary_file_detail_is_metadata_only() {
        let td = TempDir::new("detail-bad");
        let blob = td.root.join("blob.bin");
        fs::write(&blob, [0u8, 159, 146, 150]).expect("binary file");
        let mut trail = TrailState::new(&td.root);
        let mut snaps = TrailSnapshots::new(false);
        snaps.sync(&trail);

        assert_eq!(
            snaps.activate_entry(&mut trail, 0, 0, &blob),
            TrailActivateOutcome::SelectedFile
        );
        let detail = snaps.detail().expect("selection still prepares a detail");
        match &detail.preview {
            TrailDetailPreview::MetadataOnly(reason) => {
                assert_eq!(reason, "binary file");
            }
            other => panic!("expected explicit binary metadata, got {other:?}"),
        }
    }

    // FM-PERF-TEXT-06: even invalid text-like bytes are not inspected during
    // selection. The worker-owned apply seam publishes the explicit failure.
    #[test]
    fn unreadable_text_file_detail_defers_byte_inspection() {
        let td = TempDir::new("detail-invalid-text");
        let blob = td.root.join("blob.dat");
        fs::write(&blob, [159, 146, 150]).expect("invalid UTF-8 file");
        let mut trail = TrailState::new(&td.root);
        let mut snaps = TrailSnapshots::new(false);
        snaps.sync(&trail);

        assert_eq!(
            snaps.activate_entry(&mut trail, 0, 0, &blob),
            TrailActivateOutcome::SelectedFile
        );
        let detail = snaps.detail().expect("selection still prepares a detail");
        assert_eq!(detail.preview, TrailDetailPreview::PendingText);
    }

    // Image files are recognized as the image track (Kitty delivery is the
    // FIP-D4 integration lane) instead of being misread as text.
    #[test]
    fn image_file_detail_reports_image_kind() {
        let td = TempDir::new("detail-img");
        let photo = td.root.join("photo.png");
        fs::write(&photo, b"not-really-png").expect("image file");
        let mut trail = TrailState::new(&td.root);
        let mut snaps = TrailSnapshots::new(false);
        snaps.sync(&trail);

        assert_eq!(
            snaps.activate_entry(&mut trail, 0, 0, &photo),
            TrailActivateOutcome::SelectedFile
        );
        let detail = snaps.detail().expect("image selection prepares a detail");
        assert_eq!(detail.preview, TrailDetailPreview::Image);
    }

    // LAW 5: a sidebar favorite click builds a fresh single-column trail
    // rooted at the favorite — loaded, nothing selected, no dangling panel.
    #[test]
    fn favorite_click_builds_root_trail() {
        let td = TempDir::new("fav-root");
        fs::write(td.root.join("a.txt"), b"x").expect("file");
        let mut snaps = TrailSnapshots::new(false);
        let trail = snaps
            .open_trail_to(&td.root, &td.root)
            .expect("a readable favorite opens");
        assert_eq!(trail.cols().len(), 1);
        assert_eq!(trail.cols()[0].directory, td.root);
        assert_eq!(trail.cols()[0].selected, None);
        assert_eq!(snaps.cols().len(), 1);
        assert_eq!(snaps.cols()[0].status(), FmDirectoryStatus::Available);
        assert!(snaps.detail().is_none());
    }

    // LAW 5 deep-link: a FILE target resolves its whole ancestor chain —
    // every ancestor column open and selected, the file selected at the end
    // with its detail prepared.
    #[test]
    fn deep_link_builds_ancestor_chain() {
        let td = TempDir::new("deeplink");
        let a = td.root.join("a");
        let b = a.join("b");
        fs::create_dir_all(&b).expect("nested");
        let file = b.join("file.md");
        fs::write(&file, b"deep link body").expect("file");

        let mut snaps = TrailSnapshots::new(false);
        let trail = snaps
            .open_trail_to(&td.root, &file)
            .expect("deep link resolves");
        assert_eq!(trail.cols().len(), 3, "root → a → b");
        assert_eq!(trail.cols()[0].selected.as_deref(), Some(a.as_path()));
        assert_eq!(trail.cols()[1].selected.as_deref(), Some(b.as_path()));
        assert_eq!(trail.cols()[2].selected.as_deref(), Some(file.as_path()));
        assert_eq!(snaps.cols().len(), 3);
        let detail = snaps.detail().expect("file target prepares the panel");
        assert_eq!(detail.path, file);
    }

    // TP-TRAIL-T7-BRIDGE-02: the bridge resolves the trail's deepest exact
    // path back to the loaded FileEntry without consulting a legacy cursor.
    #[test]
    fn selected_entry_resolves_deepest_exact_path() {
        let td = TempDir::new("selected-entry");
        let nested = td.root.join("nested");
        fs::create_dir_all(&nested).expect("nested");
        let file = nested.join("chosen.md");
        fs::write(&file, b"chosen").expect("file");

        let mut snaps = TrailSnapshots::new(false);
        let trail = snaps
            .open_trail_to(&td.root, &file)
            .expect("deep link resolves");
        let selected = snaps
            .selected_entry(&trail)
            .expect("trail selection resolves to its loaded entry");
        assert_eq!(selected.path, file);
        assert_eq!(selected.kind, FileEntryKind::RegularFile);
    }

    // LAW 5 deep-link: a DIRECTORY target ends with its own open column.
    #[test]
    fn deep_link_to_directory_opens_its_column() {
        let td = TempDir::new("deeplink-dir");
        let a = td.root.join("a");
        let b = a.join("b");
        fs::create_dir_all(&b).expect("nested");

        let mut snaps = TrailSnapshots::new(false);
        let trail = snaps
            .open_trail_to(&td.root, &b)
            .expect("directory deep link resolves");
        assert_eq!(trail.cols().len(), 3, "root → a → b, b's column open");
        assert_eq!(trail.cols()[2].directory, b);
        assert_eq!(trail.cols()[2].selected, None);
        assert!(snaps.detail().is_none(), "a directory opens no panel");
    }

    // Fail-closed: a target OUTSIDE the root never descends anywhere — the
    // trail opens at the root only.
    #[test]
    fn target_outside_root_falls_back_to_root() {
        let td = TempDir::new("outside");
        let elsewhere = TempDir::new("elsewhere");
        let mut snaps = TrailSnapshots::new(false);
        let trail = snaps
            .open_trail_to(&td.root, &elsewhere.root)
            .expect("the root itself still opens");
        assert_eq!(trail.cols().len(), 1, "no descent outside the root");
        assert_eq!(trail.cols()[0].directory, td.root);
    }

    // Honest partial descent: a vanished middle component stops the chain
    // at the last loadable column instead of inventing anything.
    #[test]
    fn unreadable_component_stops_descent_honestly() {
        let td = TempDir::new("partial");
        let a = td.root.join("a");
        fs::create_dir_all(&a).expect("a");
        let ghost_file = a.join("ghost").join("file.md");

        let mut snaps = TrailSnapshots::new(false);
        let trail = snaps
            .open_trail_to(&td.root, &ghost_file)
            .expect("descent still opens what exists");
        assert_eq!(trail.cols().len(), 2, "root → a, ghost never appears");
        assert_eq!(trail.cols()[1].directory, a);
        assert!(snaps.detail().is_none());
    }

    // Fail-closed: an unreadable ROOT opens nothing at all.
    #[test]
    fn unreadable_root_is_rejected() {
        let td = TempDir::new("bad-root");
        let missing = td.root.join("missing-root");
        let mut snaps = TrailSnapshots::new(false);
        assert!(
            snaps.open_trail_to(&missing, &missing).is_none(),
            "an unreadable root is rejected outright"
        );
        assert!(snaps.cols().is_empty(), "no columns were built");
    }

    // Stale-hit safety: an out-of-range column index changes nothing.
    #[test]
    fn out_of_range_select_is_a_no_op() {
        let td = TempDir::new("range");
        let sub = td.root.join("sub");
        fs::create_dir_all(&sub).expect("create sub dir");

        let mut trail = TrailState::new(&td.root);
        let mut snaps = TrailSnapshots::new(false);
        snaps.sync(&trail);

        let before = trail.clone();
        assert_eq!(
            snaps.select_dir(&mut trail, 5, &sub),
            FmDirectoryStatus::Unavailable
        );
        assert_eq!(trail, before, "trail unchanged");
        assert_eq!(snaps.cols().len(), 1, "snapshots unchanged");
    }
}
