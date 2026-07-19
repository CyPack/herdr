//! Native file manager — client-side directory model.
//!
//! This is the L4 "domain" of the native file manager: reading a directory,
//! ordering its entries, and holding the cursor/visibility state for one browsed
//! folder. Following the runtime/client boundary (AGENTS.md), it is pure,
//! PTY-free presentation state that lives entirely on the TUI/client side — it
//! spawns no processes, holds no runtime state, touches no network, and never
//! panics on unreadable directories (mirrors `claude_sessions`). Rendering (A2)
//! and navigation input (A3) consume this model; they do not live here.
//!
//! Design docs: `.local/prd/native-fm/` (A1-fs-reader.md, 00-MODULE-TREE.md).

pub(crate) mod delete;
// Removal condition for the allow: FIP-3.3/3.4 wire `classify_dir_entry` into
// `read_directory_snapshot` and migrate consumers onto `FileEntryKind`.
#[allow(dead_code)]
pub(crate) mod entry_kind;
pub(crate) mod entry_time;
pub(crate) mod image_preview;
pub(crate) mod miller;
mod natsort;
pub(crate) mod operations;
pub(crate) mod preview_capability;
pub(crate) mod rename;
mod text_preview;
#[allow(dead_code)] // consumed from FIP trail program T3 (render) onward
pub(crate) mod trail;
#[allow(dead_code)] // consumed from FIP trail program T3 (render) onward
pub(crate) mod trail_snapshots;
pub(crate) mod watcher;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Hard client-side ceiling for one explicit bulk-selection operation. Bulk
/// commands reject larger sets atomically rather than silently selecting a
/// misleading subset.
pub(crate) const MAX_MULTI_SELECTION_PATHS: usize = 4_096;

pub use image_preview::{ImagePreviewError, ImagePreviewTarget, PreparedImagePreview};
pub(crate) use text_preview::highlight_text_preview;
use text_preview::{read_text_preview, TextPreviewLimits};
pub use text_preview::{
    HighlightedTextPreview, PreviewTextLine, PreviewTextSpan, PreviewTextStyle, TextPreview,
    TextPreviewError,
};

/// One entry in a browsed directory. Pure, cloneable data for rendering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEntry {
    /// File name (the final path component), as shown to the user.
    pub name: String,
    /// Absolute (or `dir`-relative) path to the entry.
    pub path: PathBuf,
    /// Canonical semantic kind prepared at snapshot time (symlink identity
    /// preserved). Visual classification and capability checks derive from
    /// this single source of truth.
    pub kind: entry_kind::FileEntryKind,
    /// Modification time of the listed path itself, prepared once with
    /// symlink-preserving metadata. Failure remains explicit as `None`; it
    /// never hides the entry or triggers render-time filesystem work.
    pub modified: Option<std::time::SystemTime>,
}

impl FileEntry {
    /// Whether this entry resolves to a directory (symlinks are followed).
    pub fn is_dir(&self) -> bool {
        self.kind.is_directory_target()
    }

    /// Whether the target resolves to a regular file or directory supported by
    /// the native operation surface. Special/broken targets fail closed.
    pub fn operation_supported(&self) -> bool {
        self.kind.supports_native_operation()
    }

    /// Presentation-safe name: every control character escaped to a printable
    /// single-cell form (TP-FIP-ICON-13). Classification and path identity
    /// keep using the raw name.
    pub fn display_name(&self) -> std::borrow::Cow<'_, str> {
        entry_kind::escape_control_chars(&self.name)
    }
}

/// Prepared availability of the exact current directory. Keeping the read
/// result beside the entry snapshot lets pure render distinguish a valid empty
/// directory from a path that disappeared or could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FmDirectoryStatus {
    Available,
    Missing,
    PermissionDenied,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FmDirectorySnapshot {
    entries: Vec<FileEntry>,
    status: FmDirectoryStatus,
    omissions: FmDirectoryOmissions,
}

/// Bounded counts for entries omitted from the actionable listing. Paths and
/// names never cross this prepared-state boundary.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct FmDirectoryOmissions {
    hidden: usize,
    non_utf8: usize,
    entry_errors: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FmNavigationReason {
    Enter,
    Leave,
    ActivateSelection,
}

/// Pure, generation-bound request for one directory transition. Preparing
/// this value performs no filesystem work; an App adapter may load the target
/// and later apply it only while every source identity still matches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FmNavigationRequest {
    pub reason: FmNavigationReason,
    pub source_directory: PathBuf,
    pub source_directory_generation: u64,
    pub source_preview_generation: u64,
    pub source_miller_revision: u64,
    pub target_directory: PathBuf,
    pub focus_path: Option<PathBuf>,
    pub fallback_cursor: usize,
    pub show_hidden: bool,
}

/// Complete I/O result for one navigation request. Applying this payload is a
/// pure model transition: every directory/preview/parent read has already
/// happened in the adapter that prepared it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FmPreparedNavigation {
    pub request: FmNavigationRequest,
    pub entries: Vec<FileEntry>,
    pub status: FmDirectoryStatus,
    omissions: FmDirectoryOmissions,
    pub writable: bool,
    pub cursor: usize,
    pub parent: Option<FmParent>,
    pub preview: FmPreview,
    pub preview_generation: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FmCurrentRefreshReason {
    ToggleHidden,
    OperationFallback,
}

/// Pure, generation-bound request for refreshing the current directory.
///
/// The active Files instance generation is explicit because model generations
/// restart when Files is closed and reopened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FmCurrentRefreshRequest {
    pub reason: FmCurrentRefreshReason,
    pub files_generation: u32,
    pub source_directory: PathBuf,
    pub source_directory_generation: u64,
    pub source_preview_generation: u64,
    pub source_miller_revision: u64,
    pub selected_path: Option<PathBuf>,
    pub fallback_cursor: usize,
    pub source_show_hidden: bool,
    pub target_show_hidden: bool,
    pub previous_text_preview: Option<TextPreview>,
}

/// Complete I/O result for one current-directory refresh. Applying this value
/// is a pure model transition and must revalidate every source identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FmPreparedCurrentRefresh {
    pub request: FmCurrentRefreshRequest,
    pub entries: Vec<FileEntry>,
    pub status: FmDirectoryStatus,
    omissions: FmDirectoryOmissions,
    pub writable: bool,
    pub cursor: usize,
    pub parent: Option<FmParent>,
    pub preview: FmPreview,
    pub preview_generation: u64,
}

/// Pure identity for loading a new Native Files root outside the UI thread.
///
/// The Files instance and prepared locations-model revision are captured
/// before the request crosses the worker boundary. Applying a result must
/// revalidate both.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FmRootNavigationRequest {
    pub files_generation: u32,
    pub location_model_revision: u64,
    pub target_root: PathBuf,
    pub show_hidden: bool,
}

/// Stable root-loading failures. Platform-specific error strings never enter
/// client presentation state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FmRootNavigationError {
    Missing,
    PermissionDenied,
    ChangedType,
    Unavailable,
}

/// Complete root projection prepared away from the UI/scheduled thread.
#[derive(Debug, Clone)]
pub(crate) struct FmPreparedRootNavigation {
    pub request: FmRootNavigationRequest,
    pub file_manager: FmState,
}

/// Parent-directory context for the left Miller column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FmParent {
    /// Ordered entries of `cwd.parent()`.
    pub entries: Vec<FileEntry>,
    /// Position of `cwd` in `entries`. This can be `None` when the parent is
    /// unreadable or changes between the directory read and state refresh.
    pub cursor: Option<usize>,
}

/// Cached content for the right Miller column. Keeping this in [`FmState`]
/// preserves the project's pure-render boundary: rendering never reads disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FmPreview {
    /// The current directory has no selected entry.
    None,
    /// The selected entry is a file and its bounded preview preparation result.
    File(FmFilePreview),
    /// The selected entry is a directory and these are its ordered children.
    Directory(Vec<FileEntry>),
}

/// Prepared selected-file state for the right Miller column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FmFilePreview {
    /// Valid bounded UTF-8 content, prepared outside render.
    Text(TextPreview),
    /// Common image format prepared asynchronously outside render.
    Image(FmImagePreview),
    /// Stable preparation failure; TP-B1.2 defines the complete UI mapping.
    Unavailable(TextPreviewError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FmImagePreview {
    pub source_path: PathBuf,
    pub generation: u64,
    pub state: FmImagePreviewState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FmImagePreviewState {
    /// No drawable preview geometry/cell size has been published yet.
    Pending,
    /// The current path/generation is being decoded for this exact pixel box.
    Loading { target: ImagePreviewTarget },
    /// Current generation pixels, ready for client-local Kitty placement.
    Ready {
        target: ImagePreviewTarget,
        prepared: PreparedImagePreview,
    },
    /// Stable failure for the current generation and target.
    Unavailable {
        target: ImagePreviewTarget,
        error: ImagePreviewError,
    },
}

/// Read the immediate children of `dir` in strict mixed modification-time
/// descending order. When `show_hidden` is false, dot-prefixed names are
/// dropped.
///
/// Never panics. A missing or unreadable directory yields an empty `Vec`, and
/// individually unreadable entries (or non-UTF-8 names) are skipped.
pub fn read_dir_entries(dir: &Path, show_hidden: bool) -> Vec<FileEntry> {
    read_directory_snapshot(dir, show_hidden).entries
}

fn read_directory_snapshot(dir: &Path, show_hidden: bool) -> FmDirectorySnapshot {
    crate::render_prof::event("fm.filesystem.read");
    let read = match std::fs::read_dir(dir) {
        Ok(read) => {
            crate::render_prof::event("fm.filesystem.read_success");
            read
        }
        Err(err) => {
            crate::render_prof::event("fm.filesystem.read_failure");
            // A directory that does not exist yet is a normal state; only log the
            // genuinely unexpected failures, and never spam.
            if err.kind() != std::io::ErrorKind::NotFound {
                tracing::debug!(?dir, %err, "fm: read_dir failed");
            }
            return FmDirectorySnapshot {
                entries: Vec::new(),
                status: classify_directory_error(err.kind()),
                omissions: FmDirectoryOmissions::default(),
            };
        }
    };

    let (mut entries, mut omissions, entry_errors) = collect_directory_entries(read, show_hidden);
    omissions.entry_errors = entry_errors;
    sort_entries(&mut entries);
    FmDirectorySnapshot {
        entries,
        status: FmDirectoryStatus::Available,
        omissions,
    }
}

/// Convert one directory iterator into presentation-safe actionable entries.
/// The third result is reserved for explicit iterator-error accounting; it is
/// deliberately zero until the FMR-1 error-classification RED is installed.
fn collect_directory_entries(
    read: impl IntoIterator<Item = std::io::Result<std::fs::DirEntry>>,
    show_hidden: bool,
) -> (Vec<FileEntry>, FmDirectoryOmissions, usize) {
    let mut entries = Vec::new();
    let mut omissions = FmDirectoryOmissions::default();
    let mut entry_errors = 0;
    for result in read {
        let Ok(entry) = result else {
            entry_errors += 1;
            continue;
        };
        // Non-UTF-8 names cannot be rendered as a `str`; skip them in v1.
        let file_name = entry.file_name();
        let Some(name) = file_name.to_str() else {
            omissions.non_utf8 += 1;
            continue;
        };
        if !show_hidden && name.starts_with('.') {
            omissions.hidden += 1;
            continue;
        }
        let path = entry.path();
        let modified = std::fs::symlink_metadata(&path)
            .and_then(|metadata| metadata.modified())
            .ok();
        entries.push(FileEntry {
            kind: entry_kind::classify_dir_entry(&entry),
            path,
            name: name.to_string(),
            modified,
        });
    }
    (entries, omissions, entry_errors)
}

fn classify_directory_error(kind: std::io::ErrorKind) -> FmDirectoryStatus {
    match kind {
        std::io::ErrorKind::NotFound => FmDirectoryStatus::Missing,
        std::io::ErrorKind::PermissionDenied => FmDirectoryStatus::PermissionDenied,
        _ => FmDirectoryStatus::Unavailable,
    }
}

fn directory_is_writable(path: &Path) -> bool {
    std::fs::metadata(path)
        .map(|metadata| metadata.is_dir() && !metadata.permissions().readonly())
        .unwrap_or(false)
}

/// Perform every filesystem read needed by one typed navigation request.
/// Applying the result later still requires exact source-generation matches.
pub(crate) fn prepare_navigation_io(request: FmNavigationRequest) -> Option<FmPreparedNavigation> {
    let snapshot = read_directory_snapshot(&request.target_directory, request.show_hidden);
    if snapshot.status != FmDirectoryStatus::Available
        || !std::fs::metadata(&request.target_directory).is_ok_and(|metadata| metadata.is_dir())
    {
        return None;
    }

    let cursor = match request.reason {
        FmNavigationReason::Enter => request.fallback_cursor,
        FmNavigationReason::Leave => request
            .focus_path
            .as_ref()
            .and_then(|path| {
                snapshot
                    .entries
                    .iter()
                    .position(|entry| &entry.path == path)
            })
            .unwrap_or(request.fallback_cursor),
        FmNavigationReason::ActivateSelection => {
            unique_entry_index(&snapshot.entries, request.focus_path.as_deref()?)?
        }
    };
    let cursor = if snapshot.entries.is_empty() {
        0
    } else {
        cursor.min(snapshot.entries.len() - 1)
    };
    let preview_generation = request.source_preview_generation.wrapping_add(1).max(1);
    let selected = snapshot
        .entries
        .get(cursor)
        .map(|entry| (entry.path.clone(), entry.is_dir()));
    let parent = read_parent_context_for(&request.target_directory, request.show_hidden);
    let preview = prepare_preview(selected, request.show_hidden, preview_generation, None);

    Some(FmPreparedNavigation {
        writable: directory_is_writable(&request.target_directory),
        request,
        entries: snapshot.entries,
        status: snapshot.status,
        omissions: snapshot.omissions,
        cursor,
        parent,
        preview,
        preview_generation,
    })
}

/// Perform every filesystem read needed by one current-directory refresh.
/// Applying the result later remains generation-bound and disk-free.
pub(crate) fn prepare_current_refresh_io(
    request: FmCurrentRefreshRequest,
) -> FmPreparedCurrentRefresh {
    let snapshot = read_directory_snapshot(&request.source_directory, request.target_show_hidden);
    let cursor = current_refresh_cursor(&request, &snapshot.entries);
    let preview_generation = request.source_preview_generation.wrapping_add(1).max(1);
    let selected = snapshot
        .entries
        .get(cursor)
        .map(|entry| (entry.path.clone(), entry.is_dir()));
    let parent = read_parent_context_for(&request.source_directory, request.target_show_hidden);
    let preview = prepare_preview(
        selected,
        request.target_show_hidden,
        preview_generation,
        request.previous_text_preview.clone(),
    );

    FmPreparedCurrentRefresh {
        writable: snapshot.status == FmDirectoryStatus::Available
            && directory_is_writable(&request.source_directory),
        request,
        entries: snapshot.entries,
        status: snapshot.status,
        omissions: snapshot.omissions,
        cursor,
        parent,
        preview,
        preview_generation,
    }
}

pub(crate) fn classify_root_navigation_error(kind: std::io::ErrorKind) -> FmRootNavigationError {
    match kind {
        std::io::ErrorKind::NotFound => FmRootNavigationError::Missing,
        std::io::ErrorKind::PermissionDenied => FmRootNavigationError::PermissionDenied,
        _ => FmRootNavigationError::Unavailable,
    }
}

/// Prepare a complete Trail root without touching App state. The target is
/// checked before and after enumeration so a missing or changed-type race
/// fails closed instead of installing a partial projection.
pub(crate) fn prepare_root_navigation_io(
    request: FmRootNavigationRequest,
) -> Result<FmPreparedRootNavigation, FmRootNavigationError> {
    let metadata = std::fs::metadata(&request.target_root)
        .map_err(|error| classify_root_navigation_error(error.kind()))?;
    if !metadata.is_dir() {
        return Err(FmRootNavigationError::ChangedType);
    }
    std::fs::read_dir(&request.target_root)
        .map_err(|error| classify_root_navigation_error(error.kind()))?;

    let file_manager = FmState::open_trail_to(
        &request.target_root,
        &request.target_root,
        request.show_hidden,
    )
    .ok_or(FmRootNavigationError::Unavailable)?;

    let metadata = std::fs::metadata(&request.target_root)
        .map_err(|error| classify_root_navigation_error(error.kind()))?;
    if !metadata.is_dir() {
        return Err(FmRootNavigationError::ChangedType);
    }

    Ok(FmPreparedRootNavigation {
        request,
        file_manager,
    })
}

/// Order every entry kind by known modification time descending. Unknown
/// times sort last; natural, raw-name, and exact-path ties keep refreshes and
/// visual fixtures deterministic.
fn sort_entries(entries: &mut [FileEntry]) {
    entries.sort_by(|a, b| {
        b.modified
            .cmp(&a.modified)
            .then_with(|| natsort::natsort(a.name.as_bytes(), b.name.as_bytes(), true))
            .then_with(|| a.name.cmp(&b.name))
            .then_with(|| a.path.cmp(&b.path))
    });
}

fn read_parent_context_for(cwd: &Path, show_hidden: bool) -> Option<FmParent> {
    let parent_path = cwd.parent()?;
    let mut entries = read_dir_entries(parent_path, show_hidden);
    let current_name = cwd.file_name().and_then(|name| name.to_str());
    let mut cursor = entries
        .iter()
        .position(|entry| entry.path == cwd || current_name.is_some_and(|name| entry.name == name));

    // Preserve a browsed dot-directory in parent context without exposing
    // unrelated hidden siblings.
    if cursor.is_none() && current_name.is_some_and(|name| name.starts_with('.')) && !show_hidden {
        if let Some(current) = read_dir_entries(parent_path, true)
            .into_iter()
            .find(|entry| entry.path == cwd || current_name.is_some_and(|name| entry.name == name))
        {
            entries.push(current);
            sort_entries(&mut entries);
            cursor = entries.iter().position(|entry| {
                entry.path == cwd || current_name.is_some_and(|name| entry.name == name)
            });
        }
    }

    Some(FmParent { entries, cursor })
}

fn prepare_preview(
    selected: Option<(PathBuf, bool)>,
    show_hidden: bool,
    generation: u64,
    previous_text: Option<TextPreview>,
) -> FmPreview {
    match selected {
        None => FmPreview::None,
        Some((path, true)) => FmPreview::Directory(read_dir_entries(&path, show_hidden)),
        Some((path, false)) if is_image_preview_path(&path) => {
            FmPreview::File(FmFilePreview::Image(FmImagePreview {
                source_path: path,
                generation,
                state: FmImagePreviewState::Pending,
            }))
        }
        Some((path, false)) => match read_text_preview(&path, TextPreviewLimits::default()) {
            Ok(mut preview) => {
                if let Some(previous) = previous_text.filter(|previous| {
                    previous.source_path == preview.source_path
                        && previous.content == preview.content
                        && previous.truncated == preview.truncated
                }) {
                    preview.highlighted = previous.highlighted;
                }
                FmPreview::File(FmFilePreview::Text(preview))
            }
            Err(error) => FmPreview::File(FmFilePreview::Unavailable(error)),
        },
    }
}

#[derive(Debug, Clone, Default)]
struct FmMultiSelection {
    paths: BTreeSet<PathBuf>,
    anchor: Option<PathBuf>,
}

/// Pure, TUI-only browsing state for one directory pane: the current directory,
/// its ordered entries, cursor focus, path-identity multi-selection, and hidden
/// file visibility. The cursor remains preview/focus authority; the separate
/// multi-selection set does not grant filesystem-operation authority. No PTY,
/// runtime, or async state is held here, so the model remains constructible and
/// assertable without a terminal.
#[derive(Debug, Clone)]
pub struct FmState {
    /// The directory currently being browsed.
    pub cwd: PathBuf,
    /// Ordered entries of `cwd` (see [`read_dir_entries`]).
    pub entries: Vec<FileEntry>,
    /// Index of the highlighted row; always within `[0, entries.len())`, or 0
    /// when the directory is empty.
    pub cursor: usize,
    /// First entry rendered in the current-directory list. Geometry-dependent
    /// normalization happens in `compute_view`; render only consumes it.
    pub viewport_start: usize,
    /// Whether dot-prefixed entries are shown.
    pub show_hidden: bool,
    /// Prepared conservative writability hint for cwd. Actual C4 operations
    /// must still handle TOCTOU and platform permission failures.
    pub cwd_writable: bool,
    /// Prepared result of reading cwd; render must not repeat this I/O.
    pub cwd_status: FmDirectoryStatus,
    /// Bounded explanation for entries omitted from the current listing.
    cwd_omissions: FmDirectoryOmissions,
    /// Monotonic identity of the prepared current-directory and parent entry
    /// snapshots. Cursor/preview changes do not advance this generation;
    /// every filesystem reload does, so rapid double-click remains valid
    /// while watcher/content refresh retires prior row targets.
    pub(crate) directory_generation: u64,
    /// Cached parent-directory context for the left Miller column.
    pub parent: Option<FmParent>,
    /// Cached selected-entry context for the right Miller column.
    pub preview: FmPreview,
    /// First visible entry of a directory PREVIEW column. This is ephemeral
    /// client presentation state: every preview identity refresh resets it.
    pub(crate) preview_viewport_start: usize,
    /// Monotonic client-local identity for preview work. Every context refresh
    /// invalidates in-flight image results even when the path is unchanged.
    pub(crate) preview_generation: u64,
    /// Explicit path identities selected for future bulk actions. This is
    /// intentionally separate from cursor focus and private to the FM model.
    multi_selection: FmMultiSelection,
    /// Canonical accumulating path trail used by the live Files surface.
    pub(crate) trail: trail::TrailState,
    /// Loaded snapshots index-aligned with `trail`; selected entry resolution
    /// never falls back to the legacy cursor.
    pub(crate) trail_snapshots: trail_snapshots::TrailSnapshots,
    /// Bounded Miller chain and resident non-current projections (FM1). The
    /// current directory's `entries` above stay the operational authority.
    pub(crate) miller: miller::MillerState,
}

impl FmState {
    pub(crate) fn request_hidden_toggle(&self, files_generation: u32) -> FmCurrentRefreshRequest {
        self.current_refresh_request(
            files_generation,
            FmCurrentRefreshReason::ToggleHidden,
            !self.show_hidden,
        )
    }

    pub(crate) fn request_operation_refresh(
        &self,
        files_generation: u32,
    ) -> FmCurrentRefreshRequest {
        self.current_refresh_request(
            files_generation,
            FmCurrentRefreshReason::OperationFallback,
            self.show_hidden,
        )
    }

    fn current_refresh_request(
        &self,
        files_generation: u32,
        reason: FmCurrentRefreshReason,
        target_show_hidden: bool,
    ) -> FmCurrentRefreshRequest {
        let previous_text_preview = match &self.preview {
            FmPreview::File(FmFilePreview::Text(preview)) => Some(preview.clone()),
            FmPreview::None
            | FmPreview::Directory(_)
            | FmPreview::File(FmFilePreview::Image(_) | FmFilePreview::Unavailable(_)) => None,
        };
        FmCurrentRefreshRequest {
            reason,
            files_generation,
            source_directory: self.cwd.clone(),
            source_directory_generation: self.directory_generation,
            source_preview_generation: self.preview_generation,
            source_miller_revision: self.miller.revision,
            selected_path: self.selected().map(|entry| entry.path.clone()),
            fallback_cursor: self.cursor,
            source_show_hidden: self.show_hidden,
            target_show_hidden,
            previous_text_preview,
        }
    }

    pub(crate) fn apply_prepared_current_refresh(
        &mut self,
        prepared: FmPreparedCurrentRefresh,
        current_files_generation: u32,
    ) -> bool {
        let expected_preview_generation = prepared
            .request
            .source_preview_generation
            .wrapping_add(1)
            .max(1);
        let status_payload_is_valid = prepared.status == FmDirectoryStatus::Available
            || (prepared.entries.is_empty() && !prepared.writable);
        if !self.current_refresh_request_is_current(&prepared.request, current_files_generation)
            || prepared.preview_generation != expected_preview_generation
            || prepared.cursor != current_refresh_cursor(&prepared.request, &prepared.entries)
            || !status_payload_is_valid
        {
            return false;
        }

        self.entries = prepared.entries;
        self.show_hidden = prepared.request.target_show_hidden;
        self.cwd_status = prepared.status;
        self.cwd_omissions = prepared.omissions;
        self.cwd_writable = prepared.writable;
        self.directory_generation = self.directory_generation.wrapping_add(1).max(1);
        self.reconcile_multi_selection();
        self.cursor = prepared.cursor;
        self.clamp_cursor();
        self.viewport_start = 0;
        self.parent = prepared.parent;
        self.preview = prepared.preview;
        self.preview_viewport_start = 0;
        self.preview_generation = prepared.preview_generation;
        self.rebuild_trail_bridge();
        true
    }

    pub(crate) fn apply_prepared_navigation(&mut self, prepared: FmPreparedNavigation) -> bool {
        let expected_preview_generation = prepared
            .request
            .source_preview_generation
            .wrapping_add(1)
            .max(1);
        let cursor_is_valid = if prepared.entries.is_empty() {
            prepared.cursor == 0
        } else {
            prepared.cursor < prepared.entries.len()
        };
        let exact_activation = match prepared.request.reason {
            FmNavigationReason::ActivateSelection => {
                prepared
                    .request
                    .focus_path
                    .as_deref()
                    .and_then(|path| unique_entry_index(&prepared.entries, path))
                    == Some(prepared.cursor)
            }
            FmNavigationReason::Enter => prepared.request.focus_path.is_none(),
            FmNavigationReason::Leave => true,
        };
        if !self.navigation_request_is_current(&prepared.request)
            || prepared.status != FmDirectoryStatus::Available
            || prepared.preview_generation != expected_preview_generation
            || !cursor_is_valid
            || !exact_activation
        {
            return false;
        }

        self.clear_multi_selection();
        self.cwd = prepared.request.target_directory.clone();
        self.entries = prepared.entries;
        self.cursor = prepared.cursor;
        self.viewport_start = 0;
        self.cwd_status = prepared.status;
        self.cwd_omissions = prepared.omissions;
        self.cwd_writable = prepared.writable;
        self.directory_generation = self.directory_generation.wrapping_add(1).max(1);
        self.parent = prepared.parent;
        self.preview = prepared.preview;
        self.preview_viewport_start = 0;
        self.preview_generation = prepared.preview_generation;
        self.miller.visit(prepared.request.target_directory);

        if prepared.request.reason == FmNavigationReason::ActivateSelection {
            let Some(path) = self
                .entries
                .get(self.cursor)
                .map(|entry| entry.path.clone())
            else {
                return false;
            };
            self.multi_selection.paths.insert(path.clone());
            self.multi_selection.anchor = Some(path);
        }
        self.rebuild_trail_bridge();
        true
    }

    pub(crate) fn request_enter_navigation(&self) -> Option<FmNavigationRequest> {
        let target_directory = self
            .selected()
            .filter(|entry| entry.is_dir())
            .map(|entry| entry.path.clone())?;
        Some(self.navigation_request(FmNavigationReason::Enter, target_directory, None, 0))
    }

    pub(crate) fn request_leave_navigation(&self) -> Option<FmNavigationRequest> {
        let departed = self.cwd.clone();
        let target_directory = departed.parent().map(Path::to_path_buf)?;
        Some(self.navigation_request(
            FmNavigationReason::Leave,
            target_directory,
            Some(departed),
            0,
        ))
    }

    fn navigation_request(
        &self,
        reason: FmNavigationReason,
        target_directory: PathBuf,
        focus_path: Option<PathBuf>,
        fallback_cursor: usize,
    ) -> FmNavigationRequest {
        FmNavigationRequest {
            reason,
            source_directory: self.cwd.clone(),
            source_directory_generation: self.directory_generation,
            source_preview_generation: self.preview_generation,
            source_miller_revision: self.miller.revision,
            target_directory,
            focus_path,
            fallback_cursor,
            show_hidden: self.show_hidden,
        }
    }

    fn navigation_request_is_current(&self, request: &FmNavigationRequest) -> bool {
        self.cwd == request.source_directory
            && self.directory_generation == request.source_directory_generation
            && self.preview_generation == request.source_preview_generation
            && self.miller.revision == request.source_miller_revision
            && self.show_hidden == request.show_hidden
    }

    fn current_refresh_request_is_current(
        &self,
        request: &FmCurrentRefreshRequest,
        current_files_generation: u32,
    ) -> bool {
        current_files_generation == request.files_generation
            && self.cwd == request.source_directory
            && self.directory_generation == request.source_directory_generation
            && self.preview_generation == request.source_preview_generation
            && self.miller.revision == request.source_miller_revision
            && self.show_hidden == request.source_show_hidden
    }

    /// Open `cwd` (hidden files off) and read its entries, cursor at the top.
    pub fn new(cwd: impl Into<PathBuf>) -> Self {
        Self::with_hidden(cwd, false)
    }

    /// Open `cwd` with an explicit hidden-file setting.
    pub fn with_hidden(cwd: impl Into<PathBuf>, show_hidden: bool) -> Self {
        let cwd = cwd.into();
        let snapshot = read_directory_snapshot(&cwd, show_hidden);
        let cwd_writable =
            snapshot.status == FmDirectoryStatus::Available && directory_is_writable(&cwd);
        let miller = miller::MillerState::seed(cwd.clone());
        let trail = trail::TrailState::new(&cwd);
        let mut state = Self {
            cwd,
            entries: snapshot.entries,
            cursor: 0,
            viewport_start: 0,
            show_hidden,
            cwd_writable,
            cwd_status: snapshot.status,
            cwd_omissions: snapshot.omissions,
            directory_generation: 1,
            parent: None,
            preview: FmPreview::None,
            preview_viewport_start: 0,
            preview_generation: 0,
            multi_selection: FmMultiSelection::default(),
            trail,
            trail_snapshots: trail_snapshots::TrailSnapshots::new(show_hidden),
            miller,
        };
        state.refresh_context();
        state
    }

    /// Disk-free baseline for unit tests that need to install prepared state.
    #[cfg(test)]
    pub(crate) fn test_empty(cwd: impl Into<PathBuf>) -> Self {
        let cwd = cwd.into();
        Self {
            miller: miller::MillerState::seed(cwd.clone()),
            trail: trail::TrailState::new(&cwd),
            trail_snapshots: trail_snapshots::TrailSnapshots::new(false),
            cwd,
            entries: Vec::new(),
            cursor: 0,
            viewport_start: 0,
            show_hidden: false,
            cwd_writable: false,
            cwd_status: FmDirectoryStatus::Available,
            cwd_omissions: FmDirectoryOmissions::default(),
            directory_generation: 1,
            parent: None,
            preview: FmPreview::None,
            preview_viewport_start: 0,
            preview_generation: 0,
            multi_selection: FmMultiSelection::default(),
        }
    }

    /// Re-read the current directory, keeping `show_hidden` and preserving the
    /// selected path when it still exists. If it disappeared, retain the old
    /// row when possible and clamp it into the new entry range.
    #[cfg(test)]
    pub fn reload(&mut self) {
        let selected_path = self.selected().map(|entry| entry.path.clone());
        let previous_cursor = self.cursor;
        self.reload_focusing_path(selected_path.as_deref(), previous_cursor);
    }

    /// Re-read cwd once, preferring one exact visible path and otherwise using
    /// the supplied cursor fallback. Directory navigation uses this seam to
    /// transfer path identity without retaining per-directory history.
    #[cfg(test)]
    fn reload_focusing_path(&mut self, selected_path: Option<&Path>, fallback_cursor: usize) {
        let snapshot = read_directory_snapshot(&self.cwd, self.show_hidden);
        self.entries = snapshot.entries;
        self.cwd_status = snapshot.status;
        self.cwd_omissions = snapshot.omissions;
        self.directory_generation = self.directory_generation.wrapping_add(1).max(1);
        self.cwd_writable =
            self.cwd_status == FmDirectoryStatus::Available && directory_is_writable(&self.cwd);
        self.reconcile_multi_selection();
        self.cursor = selected_path
            .and_then(|path| self.entries.iter().position(|entry| entry.path == path))
            .unwrap_or(fallback_cursor);
        self.clamp_cursor();
        self.refresh_context();
    }

    /// Toggle hidden-file visibility and re-read the directory.
    #[cfg(test)]
    pub fn toggle_hidden(&mut self) {
        self.show_hidden = !self.show_hidden;
        self.reload();
    }

    /// The currently highlighted entry, if any.
    pub fn selected(&self) -> Option<&FileEntry> {
        self.trail_snapshots.selected_entry(&self.trail)
    }

    /// Activate one exact row from the immutable Trail frame. The Trail
    /// snapshot revalidates column/index/path before mutation; accepted hits
    /// then refresh the temporary legacy operation projection from the owning
    /// Trail column without deriving any Trail state back from the cursor.
    pub(crate) fn activate_trail_entry(
        &mut self,
        col_idx: usize,
        entry_index: usize,
        expected_path: &Path,
    ) -> trail_snapshots::TrailActivateOutcome {
        let outcome = self.trail_snapshots.activate_entry(
            &mut self.trail,
            col_idx,
            entry_index,
            expected_path,
        );
        if outcome != trail_snapshots::TrailActivateOutcome::Rejected {
            let projection_installed = if outcome == trail_snapshots::TrailActivateOutcome::Branched
            {
                self.install_resident_directory_operation_projection(
                    col_idx,
                    entry_index,
                    expected_path,
                )
            } else {
                self.install_trail_operation_projection(col_idx, entry_index, expected_path)
            };
            if !projection_installed {
                return trail_snapshots::TrailActivateOutcome::Rejected;
            }
        }
        if outcome == trail_snapshots::TrailActivateOutcome::Branched {
            self.miller.horizontal.follow_active = true;
        }
        outcome
    }

    /// Move selection inside the one active Trail column with the same
    /// directory-branch/file-detail semantics as mouse activation.
    pub(crate) fn move_trail_selection(
        &mut self,
        delta: isize,
    ) -> trail_snapshots::TrailActivateOutcome {
        let owner_col = self.trail.active_col();
        self.move_trail_selection_in_column(owner_col, delta)
    }

    /// Move selection in one exact loaded Trail column. This is the typed
    /// non-row header input seam; the accepted entry still installs the same
    /// path-validated operation projection as ordinary row activation.
    pub(crate) fn move_trail_selection_in_column(
        &mut self,
        owner_col: usize,
        delta: isize,
    ) -> trail_snapshots::TrailActivateOutcome {
        let outcome =
            self.trail_snapshots
                .move_selection_in_column(&mut self.trail, owner_col, delta);
        if outcome == trail_snapshots::TrailActivateOutcome::Rejected {
            return outcome;
        }
        let Some(expected_path) = self
            .trail
            .cols()
            .get(owner_col)
            .and_then(|col| col.selected.clone())
        else {
            return trail_snapshots::TrailActivateOutcome::Rejected;
        };
        let Some(entry_index) = self
            .trail_snapshots
            .cols()
            .get(owner_col)
            .and_then(|snapshot| {
                snapshot
                    .entries()
                    .iter()
                    .position(|entry| entry.path == expected_path)
            })
        else {
            return trail_snapshots::TrailActivateOutcome::Rejected;
        };
        if !self.install_trail_operation_projection(owner_col, entry_index, &expected_path) {
            return trail_snapshots::TrailActivateOutcome::Rejected;
        }
        if outcome == trail_snapshots::TrailActivateOutcome::Branched {
            self.miller.horizontal.follow_active = true;
        }
        outcome
    }

    /// Re-activate the selected row in the active Trail column (Enter).
    pub(crate) fn activate_selected_trail_entry(
        &mut self,
    ) -> trail_snapshots::TrailActivateOutcome {
        let col_idx = self.trail.active_col();
        let Some(expected_path) = self
            .trail
            .cols()
            .get(col_idx)
            .and_then(|col| col.selected.clone())
        else {
            return trail_snapshots::TrailActivateOutcome::Rejected;
        };
        let Some(entry_index) = self
            .trail_snapshots
            .cols()
            .get(col_idx)
            .and_then(|snapshot| {
                snapshot
                    .entries()
                    .iter()
                    .position(|entry| entry.path == expected_path)
            })
        else {
            return trail_snapshots::TrailActivateOutcome::Rejected;
        };
        self.activate_trail_entry(col_idx, entry_index, &expected_path)
    }

    /// Move the shared keyboard/mouse focus to one exact live Trail column.
    pub(crate) fn focus_trail_col(&mut self, col_idx: usize) -> bool {
        self.trail.focus_col(col_idx)
    }

    /// Resolve one exact operation-supported path through the loaded Trail
    /// snapshots. Ambiguous duplicate identities fail closed.
    pub(crate) fn trail_entry_identity(&self, expected_path: &Path) -> Option<(usize, usize)> {
        let mut matches =
            self.trail_snapshots
                .cols()
                .iter()
                .enumerate()
                .flat_map(|(col_idx, snapshot)| {
                    snapshot
                        .entries()
                        .iter()
                        .enumerate()
                        .filter(move |(_, entry)| {
                            entry.operation_supported() && entry.path == expected_path
                        })
                        .map(move |(entry_idx, _)| (col_idx, entry_idx))
                });
        let matched = matches.next()?;
        matches.next().is_none().then_some(matched)
    }

    /// Revalidate one immutable Trail row and report whether it resolves to a
    /// directory target. This is pure prepared-state inspection: callers use
    /// it before deciding whether activation must enter the bounded I/O lane.
    pub(crate) fn trail_entry_is_directory(
        &self,
        col_idx: usize,
        entry_index: usize,
        expected_path: &Path,
    ) -> Option<bool> {
        self.trail_snapshots
            .cols()
            .get(col_idx)?
            .entries()
            .get(entry_index)
            .filter(|entry| entry.path == expected_path)
            .map(FileEntry::is_dir)
    }

    /// Build a fresh LAW-5 trail for a sidebar/deep-link target. No first row
    /// is selected implicitly; the caller receives None when the root itself
    /// cannot be read.
    pub(crate) fn open_trail_to(root: &Path, target: &Path, show_hidden: bool) -> Option<Self> {
        let mut state = Self::with_hidden(root, show_hidden);
        let mut snapshots = trail_snapshots::TrailSnapshots::new(show_hidden);
        let trail = snapshots.open_trail_to(root, target)?;
        state.trail = trail;
        state.trail_snapshots = snapshots;
        state.clear_multi_selection();
        let selected = state.trail.selected_path().map(Path::to_path_buf);
        if let Some(selected) = selected {
            let col_idx = state
                .trail
                .cols()
                .iter()
                .rposition(|col| col.selected.as_deref() == Some(selected.as_path()))?;
            let entry_index = state
                .trail_snapshots
                .cols()
                .get(col_idx)?
                .entries()
                .iter()
                .position(|entry| entry.path == selected)?;
            if !state.install_trail_operation_projection(col_idx, entry_index, &selected) {
                return None;
            }
        } else {
            state.install_trail_projection_without_selection(0)?;
        }
        Some(state)
    }

    /// Re-focus the existing root from resident snapshots. This is the
    /// zero-filesystem fast path for activating an already loaded location.
    pub(crate) fn reset_to_resident_trail_root(&mut self, expected_root: &Path) -> bool {
        if !self
            .trail_snapshots
            .reset_to_resident_root(&mut self.trail, expected_root)
        {
            return false;
        }
        self.clear_multi_selection();
        self.install_trail_projection_without_selection(0).is_some()
    }

    /// Exact directory owned by the single Trail focus authority.
    pub(crate) fn active_trail_directory(&self) -> Option<&Path> {
        self.trail
            .cols()
            .get(self.trail.active_col())
            .map(|col| col.directory.as_path())
    }

    /// Refresh the active Trail snapshot from disk and reconcile its
    /// exact-path selection. The transitional operation projection follows a
    /// deepest active column, while ancestor-only focus refreshes never
    /// overwrite a deeper file preview.
    pub(crate) fn refresh_active_trail_col(&mut self, expected_directory: &Path) -> Option<bool> {
        let col_idx = self.trail.active_col();
        if self.active_trail_directory()? != expected_directory {
            return None;
        }
        let previous_snapshot = self.trail_snapshots.cols().get(col_idx)?.clone();
        let previous_trail = self.trail.clone();
        let previous_detail = self.trail_snapshots.detail().cloned();
        let previous_projection = (
            self.cwd.clone(),
            self.entries.clone(),
            self.cursor,
            self.cwd_status,
            self.cwd_writable,
            self.preview.clone(),
        );

        if !self.trail_snapshots.refresh_col(col_idx)
            || !self
                .trail_snapshots
                .reconcile_refreshed_col(&mut self.trail, col_idx)
        {
            return None;
        }

        if col_idx == self.trail.deepest() {
            let selected = self
                .trail
                .cols()
                .get(col_idx)
                .and_then(|col| col.selected.as_deref());
            let selected_index = selected.and_then(|path| {
                self.trail_snapshots
                    .cols()
                    .get(col_idx)?
                    .entries()
                    .iter()
                    .position(|entry| entry.path == path)
            });
            if let Some(selected_index) = selected_index {
                let selected_path = self
                    .trail_snapshots
                    .cols()
                    .get(col_idx)?
                    .entries()
                    .get(selected_index)?
                    .path
                    .clone();
                if !self.install_trail_operation_projection(col_idx, selected_index, &selected_path)
                {
                    return None;
                }
            } else {
                self.install_trail_projection_without_selection(col_idx)?;
            }
        }

        let refreshed_snapshot = self.trail_snapshots.cols().get(col_idx)?;
        let current_projection = (
            self.cwd.clone(),
            self.entries.clone(),
            self.cursor,
            self.cwd_status,
            self.cwd_writable,
            self.preview.clone(),
        );
        Some(
            previous_snapshot != *refreshed_snapshot
                || previous_trail != self.trail
                || previous_detail != self.trail_snapshots.detail().cloned()
                || previous_projection != current_projection,
        )
    }

    fn install_trail_operation_projection(
        &mut self,
        col_idx: usize,
        entry_index: usize,
        expected_path: &Path,
    ) -> bool {
        let Some(snapshot) = self.trail_snapshots.cols().get(col_idx) else {
            return false;
        };
        if snapshot
            .entries()
            .get(entry_index)
            .is_none_or(|entry| entry.path != expected_path)
        {
            return false;
        }
        let directory = snapshot.directory().to_path_buf();
        let entries = snapshot.entries().to_vec();
        let status = snapshot.status();
        self.install_trail_projection(directory, entries, status, Some(entry_index));
        true
    }

    fn install_resident_directory_operation_projection(
        &mut self,
        col_idx: usize,
        entry_index: usize,
        expected_path: &Path,
    ) -> bool {
        let Some(owner) = self.trail_snapshots.cols().get(col_idx) else {
            return false;
        };
        if owner
            .entries()
            .get(entry_index)
            .is_none_or(|entry| entry.path != expected_path || !entry.is_dir())
        {
            return false;
        }
        let Some(child) = self.trail_snapshots.cols().get(col_idx + 1) else {
            return false;
        };
        if child.directory() != expected_path || child.status() != FmDirectoryStatus::Available {
            return false;
        }

        let directory = owner.directory().to_path_buf();
        let entries = owner.entries().to_vec();
        let status = owner.status();
        let writable = owner.writable();
        let parent = col_idx.checked_sub(1).and_then(|parent_idx| {
            let snapshot = self.trail_snapshots.cols().get(parent_idx)?;
            let entries = snapshot.entries().to_vec();
            let cursor = entries.iter().position(|entry| entry.path == directory);
            Some(FmParent { entries, cursor })
        });
        let preview = FmPreview::Directory(child.entries().to_vec());

        self.cwd = directory;
        self.entries = entries;
        self.cwd_status = status;
        self.cwd_writable = writable;
        self.directory_generation = self.directory_generation.wrapping_add(1).max(1);
        self.reconcile_multi_selection();
        self.cursor = entry_index;
        self.clamp_cursor();
        self.viewport_start = 0;
        self.parent = parent;
        self.preview_viewport_start = 0;
        self.preview_generation = self.preview_generation.wrapping_add(1).max(1);
        self.preview = preview;
        true
    }

    fn install_trail_projection_without_selection(&mut self, col_idx: usize) -> Option<()> {
        let snapshot = self.trail_snapshots.cols().get(col_idx)?;
        let directory = snapshot.directory().to_path_buf();
        let entries = snapshot.entries().to_vec();
        let status = snapshot.status();
        self.install_trail_projection(directory, entries, status, None);
        Some(())
    }

    fn install_trail_projection(
        &mut self,
        directory: PathBuf,
        entries: Vec<FileEntry>,
        status: FmDirectoryStatus,
        selected_index: Option<usize>,
    ) {
        self.cwd = directory;
        self.entries = entries;
        self.cwd_status = status;
        self.cwd_writable =
            status == FmDirectoryStatus::Available && directory_is_writable(&self.cwd);
        self.directory_generation = self.directory_generation.wrapping_add(1).max(1);
        self.reconcile_multi_selection();
        self.cursor = selected_index.unwrap_or(0);
        self.clamp_cursor();
        self.viewport_start = 0;
        self.parent = self.read_parent_context();
        self.preview_viewport_start = 0;
        self.preview_generation = self.preview_generation.wrapping_add(1).max(1);
        let generation = self.preview_generation;
        self.preview = selected_index
            .and_then(|index| self.entries.get(index))
            .map(|entry| (entry.path.clone(), entry.is_dir()))
            .map_or(FmPreview::None, |selected| {
                prepare_preview(Some(selected), self.show_hidden, generation, None)
            });
    }

    /// Move the cursor down one row, stopping at the last entry.
    pub fn move_down(&mut self) {
        if self.cursor + 1 < self.entries.len() {
            self.cursor += 1;
            self.refresh_preview();
        }
    }

    /// Move the cursor up one row, stopping at the top.
    pub fn move_up(&mut self) {
        let previous = self.cursor;
        self.cursor = self.cursor.saturating_sub(1);
        if self.cursor != previous {
            self.refresh_preview();
        }
    }

    /// Select an absolute entry row and refresh its cached preview. Invalid
    /// indices are rejected so stale hit geometry cannot select another row.
    pub(crate) fn select(&mut self, index: usize) -> bool {
        if index >= self.entries.len() {
            return false;
        }
        if self.cursor != index {
            self.cursor = index;
            self.refresh_preview();
        }
        true
    }

    /// Path identities in the explicit multi-selection set. Cursor movement
    /// alone never changes this set.
    pub fn multi_selection_paths(&self) -> &BTreeSet<PathBuf> {
        &self.multi_selection.paths
    }

    /// Stable path identity used as the origin for the next range selection.
    #[cfg(test)]
    pub fn multi_selection_anchor(&self) -> Option<&Path> {
        self.multi_selection.anchor.as_deref()
    }

    /// Replace the explicit selection with one live entry and focus it.
    pub fn replace_selection(&mut self, index: usize) -> bool {
        let Some(path) = self.entries.get(index).map(|entry| entry.path.clone()) else {
            return false;
        };
        if !self.select(index) {
            return false;
        }

        self.multi_selection.paths.clear();
        self.multi_selection.paths.insert(path.clone());
        self.multi_selection.anchor = Some(path);
        true
    }

    /// Toggle one live path in the explicit set and focus it. The last live
    /// target remains the range anchor even when this toggle removes it.
    pub fn toggle_selection(&mut self, index: usize) -> bool {
        let Some(path) = self.entries.get(index).map(|entry| entry.path.clone()) else {
            return false;
        };
        if !self.select(index) {
            return false;
        }

        if !self.multi_selection.paths.insert(path.clone()) {
            self.multi_selection.paths.remove(&path);
        }
        self.multi_selection.anchor = Some(path);
        true
    }

    /// Replace the explicit set with the inclusive current-list range from the
    /// anchor to `index`. No anchor falls back to a one-entry selection. A
    /// stale/ambiguous anchor, ambiguous selected identity, or oversized range
    /// is rejected before cursor or selection state changes.
    pub fn extend_selection(&mut self, index: usize) -> bool {
        let Some(target_path) = self.entries.get(index).map(|entry| entry.path.clone()) else {
            return false;
        };

        let anchor_index = match self.multi_selection.anchor.as_ref() {
            None => None,
            Some(anchor) => {
                let mut matches = self
                    .entries
                    .iter()
                    .enumerate()
                    .filter(|(_, entry)| &entry.path == anchor);
                let Some((anchor_index, _)) = matches.next() else {
                    return false;
                };
                if matches.next().is_some() {
                    return false;
                }
                Some(anchor_index)
            }
        };
        let paths = if let Some(anchor_index) = anchor_index {
            let start = anchor_index.min(index);
            let end = anchor_index.max(index);
            let range_len = end.saturating_sub(start).saturating_add(1);
            if range_len > MAX_MULTI_SELECTION_PATHS {
                return false;
            }
            let paths = self.entries[start..=end]
                .iter()
                .map(|entry| entry.path.clone())
                .collect::<BTreeSet<_>>();
            if paths.len() != range_len {
                return false;
            }
            paths
        } else {
            BTreeSet::from([target_path.clone()])
        };
        let mut identified_paths = BTreeSet::new();
        for entry in &self.entries {
            if paths.contains(&entry.path) && !identified_paths.insert(entry.path.as_path()) {
                return false;
            }
        }
        if !self.select(index) {
            return false;
        }

        self.multi_selection.paths = paths;
        if anchor_index.is_none() {
            self.multi_selection.anchor = Some(target_path);
        }
        true
    }

    /// Select every current visible entry when the complete set fits the bulk
    /// ceiling and each path identity is unique. Failure is atomic; empty state
    /// clears any stale explicit selection.
    pub fn select_all(&mut self) -> bool {
        if self.entries.len() > MAX_MULTI_SELECTION_PATHS {
            return false;
        }
        let paths = self
            .entries
            .iter()
            .map(|entry| entry.path.clone())
            .collect::<BTreeSet<_>>();
        if paths.len() != self.entries.len() {
            return false;
        }
        if paths.is_empty() {
            self.clear_multi_selection();
            return true;
        }

        let anchor = self
            .entries
            .get(self.cursor)
            .or_else(|| self.entries.first())
            .map(|entry| entry.path.clone());
        self.multi_selection.paths = paths;
        self.multi_selection.anchor = anchor;
        true
    }

    /// Clear all explicit path identities and their range anchor.
    pub fn clear_multi_selection(&mut self) {
        self.multi_selection.paths.clear();
        self.multi_selection.anchor = None;
    }

    /// Keep the cursor inside a viewport with `visible_rows` entries and clamp
    /// the first rendered row to the current list length. A zero-height or
    /// empty viewport has the canonical start offset 0.
    pub(crate) fn sync_viewport(&mut self, visible_rows: usize) {
        self.clamp_cursor();
        if visible_rows == 0 || self.entries.is_empty() {
            self.viewport_start = 0;
            return;
        }

        let max_start = self.entries.len().saturating_sub(visible_rows);
        self.viewport_start = self.viewport_start.min(max_start);
        if self.cursor < self.viewport_start {
            self.viewport_start = self.cursor;
        } else if self.cursor >= self.viewport_start.saturating_add(visible_rows) {
            self.viewport_start = self.cursor.saturating_add(1).saturating_sub(visible_rows);
        }
        self.viewport_start = self.viewport_start.min(max_start);
    }

    /// Descend into the selected entry when it is a directory, re-reading its
    /// contents with the cursor back at the top. A no-op when the selection is a
    /// file (or the directory is empty).
    #[cfg(test)]
    pub fn enter(&mut self) {
        if let Some(request) = self.request_enter_navigation() {
            if let Some(prepared) = prepare_navigation_io(request) {
                let _ = self.apply_prepared_navigation(prepared);
            }
        }
    }

    /// Go to the parent directory, re-reading its contents and focusing the
    /// exact departed child when it remains visible. Missing or filtered paths
    /// use the deterministic top fallback. A no-op at the filesystem root.
    #[cfg(test)]
    pub fn leave(&mut self) {
        if let Some(request) = self.request_leave_navigation() {
            if let Some(prepared) = prepare_navigation_io(request) {
                let _ = self.apply_prepared_navigation(prepared);
            }
        }
    }

    /// Force the cursor back into `[0, entries.len())` (0 when empty).
    fn clamp_cursor(&mut self) {
        if self.entries.is_empty() {
            self.cursor = 0;
            self.viewport_start = 0;
        } else if self.cursor >= self.entries.len() {
            self.cursor = self.entries.len() - 1;
        }
        self.viewport_start = self
            .viewport_start
            .min(self.entries.len().saturating_sub(1));
    }

    /// Keep only identities present in the current visible list. The anchor is
    /// independent from membership but must still name a live visible entry.
    fn reconcile_multi_selection(&mut self) {
        let entries = &self.entries;
        self.multi_selection
            .paths
            .retain(|path| entries.iter().any(|entry| &entry.path == path));
        if self
            .multi_selection
            .anchor
            .as_ref()
            .is_some_and(|anchor| !entries.iter().any(|entry| &entry.path == anchor))
        {
            self.multi_selection.anchor = None;
        }
    }

    /// Refresh parent and preview caches after the browsed directory or its
    /// entries change. Filesystem I/O stays here, outside the render pass.
    fn refresh_context(&mut self) {
        self.parent = self.read_parent_context();
        self.refresh_preview();
    }

    fn read_parent_context(&self) -> Option<FmParent> {
        read_parent_context_for(&self.cwd, self.show_hidden)
    }

    fn refresh_preview(&mut self) {
        self.preview_viewport_start = 0;
        self.preview_generation = self.preview_generation.wrapping_add(1).max(1);
        let generation = self.preview_generation;
        let previous_text = match std::mem::replace(&mut self.preview, FmPreview::None) {
            FmPreview::File(FmFilePreview::Text(preview)) => Some(preview),
            _ => None,
        };
        let selected = self
            .cursor_entry()
            .map(|entry| (entry.path.clone(), entry.is_dir()));
        self.preview = prepare_preview(selected, self.show_hidden, generation, previous_text);
        self.rebuild_trail_bridge();
    }

    /// Transitional T7.2 adapter: mirror the legacy prepared current model
    /// into the canonical trail without filesystem work. This method is the
    /// only place allowed to derive trail selection from the legacy cursor.
    fn rebuild_trail_bridge(&mut self) {
        let selected = self.cursor_entry().cloned();
        let root_snapshot = FmDirectorySnapshot {
            entries: self.entries.clone(),
            status: self.cwd_status,
            omissions: self.cwd_omissions,
        };
        self.trail_snapshots.integrate_current(
            &mut self.trail,
            &self.cwd,
            root_snapshot,
            self.cwd_writable,
            selected.as_ref(),
            &self.preview,
            self.show_hidden,
        );
    }

    /// Install direct synthetic fixture fields into the canonical trail.
    #[cfg(test)]
    pub(crate) fn sync_trail_bridge_for_test(&mut self) {
        self.rebuild_trail_bridge();
    }

    /// Legacy row lookup retained only as the T7.2 migration input. Public
    /// selection authority is `selected()` above and resolves through trail.
    fn cursor_entry(&self) -> Option<&FileEntry> {
        self.entries.get(self.cursor)
    }
}

fn is_image_preview_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            ["png", "jpg", "jpeg", "gif", "webp"]
                .iter()
                .any(|candidate| extension.eq_ignore_ascii_case(candidate))
        })
}

fn unique_entry_index(entries: &[FileEntry], entry_path: &Path) -> Option<usize> {
    let mut matches = entries
        .iter()
        .enumerate()
        .filter(|(_, entry)| entry.path == entry_path);
    let (entry_index, _) = matches.next()?;
    matches.next().is_none().then_some(entry_index)
}

fn current_refresh_cursor(request: &FmCurrentRefreshRequest, entries: &[FileEntry]) -> usize {
    if entries.is_empty() {
        return 0;
    }
    request
        .selected_path
        .as_deref()
        .and_then(|path| unique_entry_index(entries, path))
        .unwrap_or(request.fallback_cursor)
        .min(entries.len() - 1)
}

#[cfg(test)]
mod tests {
    use super::text_preview::{highlight_text_preview, HighlightedTextPreview};
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};

    fn unique() -> u64 {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        COUNTER.fetch_add(1, AtomicOrdering::Relaxed)
    }

    /// Isolated temp directory, recursively removed on drop. Never touches any
    /// real user directory.
    struct TempDir {
        root: PathBuf,
    }

    impl TempDir {
        fn new(tag: &str) -> Self {
            let root = std::env::temp_dir().join(format!(
                "herdr-fm-test-{}-{}-{}",
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

    fn names(entries: &[FileEntry]) -> Vec<&str> {
        entries.iter().map(|e| e.name.as_str()).collect()
    }

    fn fixed_time(seconds: u64) -> std::time::SystemTime {
        std::time::UNIX_EPOCH + std::time::Duration::from_secs(seconds)
    }

    fn set_modified(path: &Path, modified: std::time::SystemTime) {
        let file = std::fs::File::open(path).expect("open fixture for mtime");
        file.set_times(std::fs::FileTimes::new().set_modified(modified))
            .expect("set fixture mtime");
    }

    fn set_equal_modified(td: &TempDir, names: &[&str]) {
        for name in names {
            set_modified(&td.root.join(name), fixed_time(10));
        }
    }

    fn synthetic_state(entry_count: usize) -> FmState {
        let mut state = FmState::test_empty("/virtual");
        state.cwd_writable = true;
        state.entries = (0..entry_count)
            .map(|index| FileEntry {
                name: format!("{index:05}.txt"),
                path: PathBuf::from(format!("/virtual/{index:05}.txt")),
                kind: if false {
                    crate::fm::entry_kind::FileEntryKind::Directory
                } else {
                    crate::fm::entry_kind::FileEntryKind::RegularFile
                },
                modified: None,
            })
            .collect();
        state
    }

    // TP-MTIME-01/02: file kind must not outrank fresher filesystem evidence.
    // Compile-valid RED against the current directory-first comparator.
    #[test]
    fn newer_file_sorts_before_older_directory() {
        let td = TempDir::new("mtime-newer-file");
        let directory = td.root.join("aaa-directory");
        let file = td.root.join("zzz-file.txt");
        fs::create_dir(&directory).expect("create directory");
        fs::write(&file, b"x").expect("write file");
        set_modified(&directory, fixed_time(10));
        set_modified(&file, fixed_time(20));

        assert_eq!(
            names(&read_dir_entries(&td.root, false)),
            vec!["zzz-file.txt", "aaa-directory"]
        );
    }

    // TP-MTIME-02: the inverse ordering is equally strict and must not depend
    // on a directory-first exception.
    #[test]
    fn newer_directory_sorts_before_older_file() {
        let td = TempDir::new("mtime-newer-directory");
        let file = td.root.join("aaa-file.txt");
        let directory = td.root.join("zzz-directory");
        fs::write(&file, b"x").expect("write file");
        fs::create_dir(&directory).expect("create directory");
        set_modified(&file, fixed_time(10));
        set_modified(&directory, fixed_time(20));

        assert_eq!(
            names(&read_dir_entries(&td.root, false)),
            vec!["zzz-directory", "aaa-file.txt"]
        );
    }

    // TP-MTIME-01/03: an entry can disappear after read_dir yielded it but
    // before metadata preparation. It remains visible and unknown sorts last.
    #[test]
    fn deleted_dir_entry_stays_visible_and_sorts_as_unknown() {
        let td = TempDir::new("mtime-deleted-dir-entry");
        let gone = td.root.join("aaa-gone.txt");
        let live = td.root.join("zzz-live.txt");
        fs::write(&gone, b"x").expect("write disappearing file");
        fs::write(&live, b"x").expect("write live file");
        set_modified(&live, fixed_time(20));
        let dir_entries = fs::read_dir(&td.root)
            .expect("read fixture directory")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect fixture dir entries");
        fs::remove_file(&gone).expect("remove entry after iterator yielded it");

        let (mut entries, omissions, entry_errors) =
            collect_directory_entries(dir_entries.into_iter().map(Ok), false);
        sort_entries(&mut entries);

        assert_eq!(names(&entries), vec!["zzz-live.txt", "aaa-gone.txt"]);
        assert_eq!(omissions, FmDirectoryOmissions::default());
        assert_eq!(entry_errors, 0);
    }

    // TP-MTIME-01: link identity owns the displayed entry timestamp. Following
    // the old target mtime would put the freshly-created link after middle.
    #[cfg(unix)]
    #[test]
    fn symlink_uses_its_own_modification_time() {
        let td = TempDir::new("mtime-symlink");
        let target = td.root.join("aaa-target.txt");
        let middle = td.root.join("middle.txt");
        let link = td.root.join("zzz-link.txt");
        fs::write(&target, b"x").expect("write symlink target");
        fs::write(&middle, b"x").expect("write middle file");
        set_modified(&target, fixed_time(10));
        set_modified(&middle, fixed_time(20));
        std::os::unix::fs::symlink(&target, &link).expect("create symlink");

        assert_eq!(
            names(&read_dir_entries(&td.root, false)),
            vec!["zzz-link.txt", "middle.txt", "aaa-target.txt"]
        );
    }

    // TP-MTIME-01: special filesystem entries are not hidden by metadata
    // preparation and participate in the same chronological ordering.
    #[cfg(unix)]
    #[test]
    fn special_entry_participates_in_mtime_ordering() {
        let td = TempDir::new("mtime-special");
        let old = td.root.join("aaa-old.txt");
        let fifo = td.root.join("zzz-fifo");
        fs::write(&old, b"x").expect("write old file");
        set_modified(&old, fixed_time(10));
        let status = std::process::Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .expect("mkfifo runs");
        assert!(status.success(), "fifo fixture must exist");

        assert_eq!(
            names(&read_dir_entries(&td.root, false)),
            vec!["zzz-fifo", "aaa-old.txt"]
        );
    }

    // TP-MTIME-03: equal mtimes retain deterministic natural/case/raw order.
    #[test]
    fn equal_mtimes_use_natural_then_raw_name_order() {
        let td = TempDir::new("mtime-ties");
        for name in ["file10.txt", "file2.txt", "a2.txt", "A2.txt"] {
            let path = td.root.join(name);
            fs::write(&path, b"x").expect("write tie fixture");
            set_modified(&path, fixed_time(10));
        }

        assert_eq!(
            names(&read_dir_entries(&td.root, false)),
            vec!["A2.txt", "a2.txt", "file2.txt", "file10.txt"]
        );
    }

    #[test]
    fn directory_read_profile_counts_success_and_failure_without_path_labels() {
        let td = TempDir::new("directory-read-profile");
        td.file("visible.txt");
        let missing = td.root.join("missing");

        let ((available, unavailable), profile) = crate::render_prof::observe_for_test(|| {
            (
                read_directory_snapshot(&td.root, false),
                read_directory_snapshot(&missing, false),
            )
        });

        assert_eq!(available.status, FmDirectoryStatus::Available);
        assert_eq!(available.entries.len(), 1);
        assert_eq!(unavailable.status, FmDirectoryStatus::Missing);
        assert!(unavailable.entries.is_empty());
        assert_eq!(profile.counter("fm.filesystem.read"), 2);
        assert_eq!(profile.counter("fm.filesystem.read_success"), 1);
        assert_eq!(profile.counter("fm.filesystem.read_failure"), 1);
    }

    // TP-FMP-SCALE-01/02: host-specific calibration for an extreme flat
    // directory. This stays ignored in routine suites because fixture
    // creation consumes 100k inodes. Run explicitly and record the filesystem,
    // build profile, sample count, and host conditions with the result.
    #[test]
    #[ignore = "explicit 100k-entry file-manager scale calibration"]
    fn fmp_scale_100k_directory_snapshot_meets_reference_budget() {
        const ENTRY_COUNT: usize = 100_000;
        const SAMPLE_COUNT: usize = 5;
        const FINAL_P95_BUDGET_MS: u128 = 2_000;
        const RETAINED_METADATA_BUDGET_BYTES: usize = 64 * 1024 * 1024;
        const LARGE_FILE_COUNT: usize = 4;
        const LARGE_FILE_BYTES: u64 = 256 * 1024 * 1024 * 1024;

        let td = TempDir::new("fmp-scale-100k");
        let fixture_started = std::time::Instant::now();
        for index in 0..ENTRY_COUNT {
            let path = td.root.join(format!("entry-{index:06}.dat"));
            let file = std::fs::File::create(&path).expect("create scale fixture entry");
            if index < LARGE_FILE_COUNT {
                file.set_len(LARGE_FILE_BYTES)
                    .expect("create sparse large-file fixture");
            }
        }
        let fixture_ms = fixture_started.elapsed().as_millis();

        let warmup = read_directory_snapshot(&td.root, false);
        assert_eq!(warmup.entries.len(), ENTRY_COUNT);
        drop(warmup);

        let mut samples_ms = Vec::with_capacity(SAMPLE_COUNT);
        let mut final_snapshot = None;
        for _ in 0..SAMPLE_COUNT {
            let started = std::time::Instant::now();
            let snapshot = read_directory_snapshot(&td.root, false);
            samples_ms.push(started.elapsed().as_millis());
            final_snapshot = Some(snapshot);
        }
        samples_ms.sort_unstable();
        let p95_ms = samples_ms[SAMPLE_COUNT - 1];
        let snapshot = final_snapshot.expect("at least one calibrated sample");
        assert_eq!(snapshot.entries.len(), ENTRY_COUNT);

        let retained_metadata_bytes = std::mem::size_of_val(snapshot.entries.as_slice())
            + snapshot
                .entries
                .iter()
                .map(|entry| entry.name.len() + entry.path.as_os_str().len())
                .sum::<usize>();
        eprintln!(
            "FMP_SCALE_100K fixture_ms={fixture_ms} samples_ms={samples_ms:?} \
             p95_ms={p95_ms} entries={} logical_large_file_bytes={} \
             retained_metadata_lower_bound_bytes={retained_metadata_bytes}",
            snapshot.entries.len(),
            LARGE_FILE_COUNT as u64 * LARGE_FILE_BYTES,
        );

        assert!(
            p95_ms <= FINAL_P95_BUDGET_MS,
            "100k-entry final sorted snapshot p95 {p95_ms}ms exceeds \
             {FINAL_P95_BUDGET_MS}ms and authorizes partial listing work"
        );
        assert!(
            retained_metadata_bytes <= RETAINED_METADATA_BUDGET_BYTES,
            "100k-entry retained metadata lower bound {retained_metadata_bytes} exceeds \
             {RETAINED_METADATA_BUDGET_BYTES} bytes and authorizes cache/paging work"
        );
    }

    #[test]
    fn directory_visibility_counts_iterator_entry_errors() {
        let read = std::iter::once(Err::<std::fs::DirEntry, _>(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "synthetic entry failure",
        )));

        let (entries, omissions, entry_errors) = collect_directory_entries(read, false);

        assert!(entries.is_empty());
        assert_eq!(omissions, FmDirectoryOmissions::default());
        assert_eq!(
            entry_errors, 1,
            "an individual iterator failure must not disappear into flattening"
        );
    }

    // TP-FM3-DIRECTORY-GENERATION: cursor/preview movement must not retire a
    // second click from the same prepared directory snapshot, while any
    // filesystem reload must retire every prior current/parent row target.
    #[test]
    fn directory_generation_changes_on_reload_not_cursor_selection() {
        let td = TempDir::new("directory-generation");
        td.file("a.txt");
        td.file("b.txt");
        let mut state = FmState::new(&td.root);
        let opened_generation = state.directory_generation;

        assert!(state.select(1));
        assert_eq!(
            state.directory_generation, opened_generation,
            "cursor/preview selection is not a directory content generation"
        );

        state.reload();
        assert!(
            state.directory_generation > opened_generation,
            "every prepared directory reload retires prior row targets"
        );
    }

    // TP-MTIME-03: equal modification times use one natural order across
    // kinds; a directory receives no tie-breaking priority.
    #[test]
    fn equal_mtime_entries_use_natural_order_across_kinds() {
        let td = TempDir::new("order");
        td.file("file10.txt");
        td.file("file2.txt");
        td.dir("zeta");
        td.dir("alpha10");
        td.dir("alpha2");
        for name in ["file10.txt", "file2.txt", "zeta", "alpha10", "alpha2"] {
            set_modified(&td.root.join(name), fixed_time(10));
        }
        let entries = read_dir_entries(&td.root, false);
        assert_eq!(
            names(&entries),
            vec!["alpha2", "alpha10", "file2.txt", "file10.txt", "zeta"]
        );
    }

    // T-A1.2b: hidden entries are dropped unless requested.
    #[test]
    fn hidden_entries_toggle() {
        let td = TempDir::new("hidden");
        td.file("visible.txt");
        td.file(".secret");
        td.dir(".git");
        td.dir("src");
        for name in ["visible.txt", ".secret", ".git", "src"] {
            set_modified(&td.root.join(name), fixed_time(10));
        }
        assert_eq!(
            names(&read_dir_entries(&td.root, false)),
            vec!["src", "visible.txt"]
        );
        assert_eq!(
            names(&read_dir_entries(&td.root, true)),
            vec![".git", ".secret", "src", "visible.txt"]
        );
    }

    // T-A1.2c: a missing directory yields an empty list, no panic.
    #[test]
    fn missing_directory_is_empty_and_panic_free() {
        let td = TempDir::new("missing");
        let missing = td.root.join("does-not-exist");
        let entries = read_dir_entries(&missing, false);
        assert!(entries.is_empty());

        let state = FmState::new(missing);
        assert!(!state.cwd_writable);
    }

    // TP-C6.4-EMPTY-ERROR: current-directory read failures are preserved as
    // prepared state instead of being rendered as a valid empty directory.
    #[test]
    fn current_directory_status_distinguishes_available_missing_and_unavailable() {
        let td = TempDir::new("directory-status");
        let available = FmState::new(&td.root);
        assert_eq!(available.cwd_status, FmDirectoryStatus::Available);

        let missing_path = td.root.join("missing");
        let missing = FmState::new(&missing_path);
        assert_eq!(missing.cwd_status, FmDirectoryStatus::Missing);
        assert!(missing.entries.is_empty());

        let file_path = td.root.join("not-a-directory.txt");
        fs::write(&file_path, b"file").expect("write non-directory fixture");
        let unavailable = FmState::new(&file_path);
        assert_eq!(unavailable.cwd_status, FmDirectoryStatus::Unavailable);
        assert!(unavailable.entries.is_empty());
    }

    // TP-C6.4-EMPTY-ERROR: permission classification is deterministic even
    // when the test process itself has elevated filesystem privileges.
    #[test]
    fn directory_error_kind_classification_is_platform_independent() {
        assert_eq!(
            classify_directory_error(std::io::ErrorKind::NotFound),
            FmDirectoryStatus::Missing
        );
        assert_eq!(
            classify_directory_error(std::io::ErrorKind::PermissionDenied),
            FmDirectoryStatus::PermissionDenied
        );
        assert_eq!(
            classify_directory_error(std::io::ErrorKind::NotADirectory),
            FmDirectoryStatus::Unavailable
        );
    }

    // TP-C6.4-EMPTY-ERROR: reload refreshes failure state and can recover when
    // the same exact cwd path becomes a readable directory again.
    #[test]
    fn missing_current_path_has_defined_recovery() {
        let td = TempDir::new("directory-status-reload");
        let cwd = td.root.join("cwd");
        fs::create_dir(&cwd).expect("create cwd fixture");
        let mut state = FmState::new(&cwd);
        assert_eq!(state.cwd_status, FmDirectoryStatus::Available);
        let initial_directory_generation = state.directory_generation;

        fs::remove_dir(&cwd).expect("remove cwd fixture");
        state.reload();
        assert_eq!(state.cwd_status, FmDirectoryStatus::Missing);
        assert!(state.entries.is_empty());
        assert_eq!(state.cursor, 0);
        assert_eq!(state.cwd, cwd);
        assert_eq!(state.miller.focused_directory, cwd);
        assert!(state.directory_generation > initial_directory_generation);
        state.miller.assert_miller_invariants_for_test();

        fs::create_dir(&cwd).expect("recreate cwd fixture");
        state.reload();
        assert_eq!(state.cwd_status, FmDirectoryStatus::Available);
        assert_eq!(state.cwd, cwd);
        assert_eq!(state.miller.focused_directory, cwd);
        state.miller.assert_miller_invariants_for_test();
    }

    // T-A1.2d: a symlink to a directory is listed and sorted as a directory.
    #[cfg(unix)]
    #[test]
    fn symlink_to_directory_counts_as_directory() {
        let td = TempDir::new("symlink");
        td.dir("target");
        td.file("zzz.txt");
        std::os::unix::fs::symlink(td.root.join("target"), td.root.join("link"))
            .expect("create symlink");
        let entries = read_dir_entries(&td.root, false);
        assert_eq!(names(&entries), vec!["link", "target", "zzz.txt"]);
        let link = entries
            .iter()
            .find(|e| e.name == "link")
            .expect("link entry");
        assert!(link.is_dir());
        assert!(link.operation_supported());
    }

    // T-N3.2h: unsupported Unix special files remain visible but fail closed
    // for copy/delete action authority.
    #[cfg(unix)]
    #[test]
    fn special_entry_is_visible_but_operation_unsupported() {
        let td = TempDir::new("special-entry");
        let socket_path = td.root.join("agent.sock");
        let _listener = std::os::unix::net::UnixListener::bind(&socket_path)
            .expect("bind isolated Unix socket");

        let entries = read_dir_entries(&td.root, false);
        let socket = entries
            .iter()
            .find(|entry| entry.path == socket_path)
            .expect("special entry remains visible");
        assert!(!socket.is_dir());
        assert!(!socket.operation_supported());
    }

    // T-N3.2i: cwd writability is prepared outside render and refreshed when
    // filesystem permissions change.
    #[cfg(unix)]
    #[test]
    fn reload_refreshes_cwd_writability() {
        use std::os::unix::fs::PermissionsExt;

        let td = TempDir::new("cwd-writability");
        let original_mode = fs::metadata(&td.root)
            .expect("read temp directory metadata")
            .permissions()
            .mode();
        let mut state = FmState::new(&td.root);
        assert!(state.cwd_writable);

        fs::set_permissions(&td.root, fs::Permissions::from_mode(original_mode & !0o222))
            .expect("make temp directory read-only");
        state.reload();
        assert!(!state.cwd_writable);

        fs::set_permissions(&td.root, fs::Permissions::from_mode(original_mode))
            .expect("restore temp directory permissions");
        state.reload();
        assert!(state.cwd_writable);
    }

    // T-A1.2d: unicode / emoji names survive intact.
    #[test]
    fn unicode_and_emoji_names_survive() {
        let td = TempDir::new("unicode");
        td.file("café.txt");
        td.file("naïve.md");
        td.dir("smile-dir");
        let entries = read_dir_entries(&td.root, false);
        assert!(entries.iter().any(|e| e.name == "café.txt"));
        assert!(entries.iter().any(|e| e.name == "naïve.md"));
        assert!(entries.iter().any(|e| e.name == "smile-dir" && e.is_dir()));
    }

    // T-A1.3a / TP-MTIME-03: opening a directory puts the cursor at the top
    // of the canonical mixed order.
    #[test]
    fn fmstate_opens_with_cursor_at_top() {
        let td = TempDir::new("state-open");
        td.file("a.txt");
        td.dir("d");
        set_modified(&td.root.join("a.txt"), fixed_time(10));
        set_modified(&td.root.join("d"), fixed_time(10));
        let st = FmState::new(&td.root);
        assert_eq!(st.cursor, 0);
        assert_eq!(st.entries.len(), 2);
        assert_eq!(st.selected().map(|e| e.name.as_str()), Some("a.txt"));
    }

    // TP-TRAIL-T7-BRIDGE-03: FmState owns one index-aligned trail/snapshot
    // bridge immediately after opening; selected() is the trail's exact path.
    #[test]
    fn fmstate_owns_aligned_trail_bridge_on_open() {
        let td = TempDir::new("trail-bridge-open");
        td.dir("alpha");
        td.file("zeta.txt");

        let state = FmState::new(&td.root);
        let selected = state.selected().expect("cursor selects alpha");
        assert_eq!(
            state.trail.selected_path(),
            Some(selected.path.as_path()),
            "trail is the selected-path authority"
        );
        assert_eq!(
            state
                .trail_snapshots
                .selected_entry(&state.trail)
                .map(|entry| entry.path.as_path()),
            Some(selected.path.as_path())
        );
        assert_eq!(state.trail.cols().len(), state.trail_snapshots.cols().len());
        for (trail_col, snapshot) in state.trail.cols().iter().zip(state.trail_snapshots.cols()) {
            assert_eq!(trail_col.directory, snapshot.directory());
        }
    }

    // TP-TRAIL-T7-BRIDGE-04: cursor movement rebuilds exact-path trail
    // selection while explicit bulk authority remains independent.
    #[test]
    fn cursor_move_rebuilds_trail_selection_without_bulk_authority() {
        let td = TempDir::new("trail-bridge-cursor");
        td.file("00.txt");
        td.file("01.txt");
        set_modified(&td.root.join("00.txt"), fixed_time(10));
        set_modified(&td.root.join("01.txt"), fixed_time(10));
        let mut state = FmState::new(&td.root);
        assert!(state.multi_selection_paths().is_empty());

        state.move_down();

        let expected = td.root.join("01.txt");
        assert_eq!(state.selected().map(|entry| &entry.path), Some(&expected));
        assert_eq!(state.trail.selected_path(), Some(expected.as_path()));
        assert_eq!(
            state
                .trail_snapshots
                .selected_entry(&state.trail)
                .map(|entry| &entry.path),
            Some(&expected)
        );
        assert_eq!(state.trail.cols().len(), state.trail_snapshots.cols().len());
        assert!(state.multi_selection_paths().is_empty());
        assert!(state.multi_selection_anchor().is_none());
    }

    // TP-N4.1-SELECTION-STATE: cursor focus remains independent from the
    // explicit path-identity set that will later authorize bulk operations.
    #[test]
    fn multi_selection_starts_empty_and_cursor_moves_independently() {
        let td = TempDir::new("multi-selection-cursor-independent");
        td.file("00.txt");
        td.file("01.txt");
        set_equal_modified(&td, &["00.txt", "01.txt"]);
        let mut state = FmState::new(&td.root);

        assert!(state.multi_selection_paths().is_empty());
        assert!(state.multi_selection_anchor().is_none());

        state.move_down();

        assert_eq!(state.cursor, 1);
        assert!(state.multi_selection_paths().is_empty());
        assert!(state.multi_selection_anchor().is_none());
    }

    // TP-N4.1-SELECTION-STATE: an unmodified selection replaces any previous
    // set with exactly the live target and establishes a deterministic anchor.
    #[test]
    fn plain_selection_replaces_paths_and_establishes_anchor() {
        let td = TempDir::new("multi-selection-plain");
        td.file("00.txt");
        td.file("01.txt");
        td.file("02.txt");
        set_equal_modified(&td, &["00.txt", "01.txt", "02.txt"]);
        let mut state = FmState::new(&td.root);
        let first = td.root.join("00.txt");
        let last = td.root.join("02.txt");

        assert!(state.replace_selection(0));
        assert!(state.toggle_selection(1));
        assert!(state.replace_selection(2));

        assert_eq!(state.cursor, 2);
        assert_eq!(
            state.multi_selection_paths().iter().collect::<Vec<_>>(),
            vec![&last]
        );
        assert_eq!(state.multi_selection_anchor(), Some(last.as_path()));
        assert!(!state.multi_selection_paths().contains(&first));
    }

    // TP-N4.1-SELECTION-STATE: Ctrl-style toggle is path-deduplicated, removes
    // an already selected path, and leaves the last live target as the anchor.
    #[test]
    fn toggle_selection_deduplicates_removes_and_updates_anchor() {
        let td = TempDir::new("multi-selection-toggle");
        td.file("00.txt");
        td.file("01.txt");
        set_equal_modified(&td, &["00.txt", "01.txt"]);
        let mut state = FmState::new(&td.root);
        let first = td.root.join("00.txt");
        let second = td.root.join("01.txt");

        assert!(state.toggle_selection(0));
        assert!(state.toggle_selection(1));
        assert_eq!(state.multi_selection_paths().len(), 2);

        assert!(state.toggle_selection(0));

        assert_eq!(state.cursor, 0);
        assert_eq!(
            state.multi_selection_paths().iter().collect::<Vec<_>>(),
            vec![&second]
        );
        assert_eq!(state.multi_selection_anchor(), Some(first.as_path()));
    }

    // TP-N4.1-SELECTION-STATE: Shift-style range is inclusive and derives its
    // order from the current visible list, not from stale stored row indices.
    #[test]
    fn range_selection_is_inclusive_and_direction_independent() {
        let td = TempDir::new("multi-selection-range");
        for index in 0..5 {
            td.file(&format!("{index:02}.txt"));
        }
        set_equal_modified(&td, &["00.txt", "01.txt", "02.txt", "03.txt", "04.txt"]);
        let mut state = FmState::new(&td.root);
        let expected = ["01.txt", "02.txt", "03.txt"]
            .into_iter()
            .map(|name| td.root.join(name))
            .collect::<std::collections::BTreeSet<_>>();
        let backward_anchor = td.root.join("03.txt");
        let forward_anchor = td.root.join("01.txt");

        assert!(state.replace_selection(3));
        assert!(state.extend_selection(1));
        assert_eq!(state.multi_selection_paths(), &expected);
        assert_eq!(
            state.multi_selection_anchor(),
            Some(backward_anchor.as_path())
        );

        assert!(state.replace_selection(1));
        assert!(state.extend_selection(3));
        assert_eq!(state.multi_selection_paths(), &expected);
        assert_eq!(
            state.multi_selection_anchor(),
            Some(forward_anchor.as_path())
        );
    }

    // TP-N4.1-SELECTION-STATE: missing anchors fall back to the live target,
    // stale targets are rejected without mutation, and duplicate visible path
    // identities cannot inflate the selection set or panic range selection.
    #[test]
    fn range_selection_fails_closed_for_missing_stale_and_duplicate_identity() {
        let td = TempDir::new("multi-selection-adversarial");
        td.file("00.txt");
        td.file("01.txt");
        td.file("02.txt");
        set_equal_modified(&td, &["00.txt", "01.txt", "02.txt"]);
        let mut state = FmState::new(&td.root);
        let first = td.root.join("00.txt");
        let second = td.root.join("01.txt");

        assert!(state.replace_selection(2));
        state.entries.remove(2);
        state.entries.insert(1, state.entries[1].clone());

        let stale_paths = state.multi_selection_paths().clone();
        let stale_anchor = state.multi_selection_anchor().map(Path::to_path_buf);
        let stale_cursor = state.cursor;
        assert!(!state.extend_selection(0));
        assert_eq!(
            state.multi_selection_paths(),
            &stale_paths,
            "stale anchor is rejected atomically"
        );
        assert_eq!(state.multi_selection_anchor(), stale_anchor.as_deref());
        assert_eq!(state.cursor, stale_cursor);

        let before_paths = state.multi_selection_paths().clone();
        let before_anchor = state.multi_selection_anchor().map(Path::to_path_buf);
        assert!(!state.extend_selection(usize::MAX));
        assert_eq!(state.multi_selection_paths(), &before_paths);
        assert_eq!(state.multi_selection_anchor(), before_anchor.as_deref());

        assert!(state.replace_selection(0));
        let duplicate_paths = state.multi_selection_paths().clone();
        let duplicate_anchor = state.multi_selection_anchor().map(Path::to_path_buf);
        let duplicate_cursor = state.cursor;
        assert!(!state.extend_selection(2));
        assert_eq!(state.multi_selection_paths(), &duplicate_paths);
        assert_eq!(state.multi_selection_anchor(), duplicate_anchor.as_deref());
        assert_eq!(state.cursor, duplicate_cursor);
        assert!(state.multi_selection_paths().contains(&first));
        assert!(!state.multi_selection_paths().contains(&second));
    }

    // TP-N4.2-BULK-AUTHORITY: select-all is atomic. Exact-limit unique state
    // succeeds, while overflow and duplicate identities preserve prior state.
    #[test]
    fn select_all_is_atomic_bounded_and_deterministic() {
        let mut exact = synthetic_state(MAX_MULTI_SELECTION_PATHS);
        exact.cursor = MAX_MULTI_SELECTION_PATHS - 1;
        let cursor_path = exact.entries[exact.cursor].path.clone();

        assert!(exact.select_all());
        assert_eq!(
            exact.multi_selection_paths().len(),
            MAX_MULTI_SELECTION_PATHS
        );
        assert_eq!(exact.multi_selection_anchor(), Some(cursor_path.as_path()));
        assert_eq!(exact.cursor, MAX_MULTI_SELECTION_PATHS - 1);

        exact.clear_multi_selection();
        assert!(exact.multi_selection_paths().is_empty());
        assert!(exact.multi_selection_anchor().is_none());

        let mut overflow = synthetic_state(MAX_MULTI_SELECTION_PATHS + 1);
        assert!(overflow.replace_selection(7));
        let overflow_paths = overflow.multi_selection_paths().clone();
        let overflow_anchor = overflow.multi_selection_anchor().map(Path::to_path_buf);
        let overflow_cursor = overflow.cursor;
        assert!(!overflow.select_all());
        assert_eq!(overflow.multi_selection_paths(), &overflow_paths);
        assert_eq!(
            overflow.multi_selection_anchor(),
            overflow_anchor.as_deref()
        );
        assert_eq!(overflow.cursor, overflow_cursor);

        let mut duplicate = synthetic_state(2);
        duplicate.entries[1].path = duplicate.entries[0].path.clone();
        assert!(duplicate.replace_selection(0));
        let duplicate_paths = duplicate.multi_selection_paths().clone();
        assert!(!duplicate.select_all());
        assert_eq!(duplicate.multi_selection_paths(), &duplicate_paths);

        duplicate.entries.clear();
        assert!(duplicate.select_all());
        assert!(duplicate.multi_selection_paths().is_empty());
        assert!(duplicate.multi_selection_anchor().is_none());
    }

    // TP-N4.2-BULK-AUTHORITY: inclusive ranges accept the exact bound but
    // reject overflow atomically instead of silently selecting a partial set.
    #[test]
    fn range_selection_is_atomic_at_the_bulk_limit() {
        let mut exact = synthetic_state(MAX_MULTI_SELECTION_PATHS);
        assert!(exact.replace_selection(0));
        assert!(exact.extend_selection(MAX_MULTI_SELECTION_PATHS - 1));
        assert_eq!(
            exact.multi_selection_paths().len(),
            MAX_MULTI_SELECTION_PATHS
        );
        assert_eq!(exact.cursor, MAX_MULTI_SELECTION_PATHS - 1);

        let mut overflow = synthetic_state(MAX_MULTI_SELECTION_PATHS + 1);
        assert!(overflow.replace_selection(0));
        let before_paths = overflow.multi_selection_paths().clone();
        let before_anchor = overflow.multi_selection_anchor().map(Path::to_path_buf);
        let before_cursor = overflow.cursor;
        assert!(!overflow.extend_selection(MAX_MULTI_SELECTION_PATHS));
        assert_eq!(overflow.multi_selection_paths(), &before_paths);
        assert_eq!(overflow.multi_selection_anchor(), before_anchor.as_deref());
        assert_eq!(overflow.cursor, before_cursor);
    }

    // TP-N4.1-SELECTION-STATE: watcher-style reload follows live path identity
    // across reordering, prunes deleted members, and drops a missing anchor.
    #[test]
    fn reload_reconciles_multi_selection_by_live_path_identity() {
        let td = TempDir::new("multi-selection-reload-lifecycle");
        td.file("b.txt");
        td.file("c.txt");
        td.file("d.txt");
        set_equal_modified(&td, &["b.txt", "c.txt", "d.txt"]);
        let mut state = FmState::new(&td.root);
        let b = td.root.join("b.txt");
        let d = td.root.join("d.txt");

        assert!(state.replace_selection(0));
        assert!(state.toggle_selection(2));
        assert_eq!(state.multi_selection_anchor(), Some(d.as_path()));

        td.file("a.txt");
        state.reload();
        assert_eq!(
            state.multi_selection_paths().iter().collect::<Vec<_>>(),
            vec![&b, &d]
        );
        assert_eq!(state.multi_selection_anchor(), Some(d.as_path()));

        fs::remove_file(&d).expect("delete anchored selection");
        state.reload();
        assert_eq!(
            state.multi_selection_paths().iter().collect::<Vec<_>>(),
            vec![&b]
        );
        assert!(state.multi_selection_anchor().is_none());

        fs::remove_file(&b).expect("delete final selection");
        state.reload();
        assert!(state.multi_selection_paths().is_empty());
        assert!(state.multi_selection_anchor().is_none());
    }

    // TP-N4.1-SELECTION-STATE: hiding a selected dotfile removes only that
    // invisible identity and clears its anchor while preserving live members.
    #[test]
    fn hidden_toggle_prunes_invisible_selection_and_anchor() {
        let td = TempDir::new("multi-selection-hidden-lifecycle");
        td.file("visible.txt");
        td.file(".hidden.txt");
        let visible = td.root.join("visible.txt");
        let hidden = td.root.join(".hidden.txt");
        let mut state = FmState::with_hidden(&td.root, true);
        let visible_index = state
            .entries
            .iter()
            .position(|entry| entry.path == visible)
            .expect("visible entry index");
        let hidden_index = state
            .entries
            .iter()
            .position(|entry| entry.path == hidden)
            .expect("hidden entry index");

        assert!(state.replace_selection(visible_index));
        assert!(state.toggle_selection(hidden_index));
        assert_eq!(state.multi_selection_anchor(), Some(hidden.as_path()));

        state.toggle_hidden();

        assert_eq!(
            state.multi_selection_paths().iter().collect::<Vec<_>>(),
            vec![&visible]
        );
        assert!(state.multi_selection_anchor().is_none());
    }

    // TP-N4.1-SELECTION-STATE: successful directory changes clear identities
    // from the old listing, while an enter attempt on a file remains a no-op.
    #[test]
    fn directory_navigation_clears_selection_but_file_enter_preserves_it() {
        let td = TempDir::new("multi-selection-directory-lifecycle");
        td.dir("child");
        td.file("sibling.txt");
        td.file("child/inside.txt");
        let child = td.root.join("child");
        let sibling = td.root.join("sibling.txt");
        let mut state = FmState::new(&td.root);
        let child_index = state
            .entries
            .iter()
            .position(|entry| entry.path == child)
            .expect("child entry index");
        let sibling_index = state
            .entries
            .iter()
            .position(|entry| entry.path == sibling)
            .expect("sibling entry index");

        assert!(state.replace_selection(sibling_index));
        state.enter();
        assert_eq!(state.cwd, td.root);
        assert_eq!(
            state.multi_selection_paths().iter().collect::<Vec<_>>(),
            vec![&sibling]
        );

        assert!(state.toggle_selection(child_index));
        assert!(state.select(child_index));
        state.enter();
        assert_eq!(state.cwd, child);
        assert!(state.multi_selection_paths().is_empty());
        assert!(state.multi_selection_anchor().is_none());

        assert!(state.replace_selection(0));
        state.leave();
        assert_eq!(state.cwd, td.root);
        assert!(state.multi_selection_paths().is_empty());
        assert!(state.multi_selection_anchor().is_none());
    }

    // TP-N4.1-SELECTION-STATE: closing destroys FmState, and a fresh instance
    // cannot inherit selection identity or anchor from the previous overlay.
    #[test]
    fn reopened_file_manager_starts_with_empty_multi_selection() {
        let td = TempDir::new("multi-selection-reopen-lifecycle");
        td.file("selected.txt");
        let mut closed = FmState::new(&td.root);
        assert!(closed.replace_selection(0));
        assert_eq!(closed.multi_selection_paths().len(), 1);

        drop(closed);
        let reopened = FmState::new(&td.root);
        assert!(reopened.multi_selection_paths().is_empty());
        assert!(reopened.multi_selection_anchor().is_none());
    }

    // T-A1.3b: the cursor is always clamped into range (empty, end, shrink).
    #[test]
    fn cursor_stays_in_range() {
        let td = TempDir::new("state-clamp");

        // Empty directory: cursor pinned at 0, moving down is a no-op.
        let mut st = FmState::new(&td.root);
        assert_eq!(st.cursor, 0);
        st.move_down();
        assert_eq!(st.cursor, 0);
        assert!(st.selected().is_none());

        // Populate and drive the cursor to the last entry.
        td.file("a");
        td.file("b");
        td.file("c");
        st.reload();
        assert_eq!(st.entries.len(), 3);
        st.move_down();
        st.move_down();
        st.move_down();
        assert_eq!(st.cursor, 2);

        // Shrinking the directory re-clamps the cursor on reload.
        fs::remove_file(td.root.join("b")).expect("remove b");
        fs::remove_file(td.root.join("c")).expect("remove c");
        st.reload();
        assert_eq!(st.entries.len(), 1);
        assert_eq!(st.cursor, 0);
    }

    // TP-A3.2-VIEWPORT: a three-row viewport follows repeated cursor movement
    // in both directions, only scrolling when the cursor would leave the
    // visible window and clamping exactly at the final valid start row.
    #[test]
    fn viewport_follows_cursor_and_clamps_at_both_edges() {
        let td = TempDir::new("viewport-follow");
        for index in 0..10 {
            td.file(&format!("{index:02}.txt"));
        }
        let mut st = FmState::new(&td.root);

        st.sync_viewport(3);
        assert_eq!((st.cursor, st.viewport_start), (0, 0));

        for _ in 0..3 {
            st.move_down();
            st.sync_viewport(3);
        }
        assert_eq!((st.cursor, st.viewport_start), (3, 1));

        for _ in 0..20 {
            st.move_down();
            st.sync_viewport(3);
        }
        assert_eq!((st.cursor, st.viewport_start), (9, 7));

        for _ in 0..3 {
            st.move_up();
            st.sync_viewport(3);
        }
        assert_eq!((st.cursor, st.viewport_start), (6, 6));

        for _ in 0..20 {
            st.move_up();
            st.sync_viewport(3);
        }
        assert_eq!((st.cursor, st.viewport_start), (0, 0));
    }

    // TP-A3.2-VIEWPORT: degenerate height and asynchronous directory shrink
    // cannot leave a stale offset beyond the new list; an empty list has the
    // canonical cursor/viewport pair (0, 0).
    #[test]
    fn viewport_handles_zero_rows_reload_shrink_and_empty_list() {
        let td = TempDir::new("viewport-shrink");
        for index in 0..5 {
            td.file(&format!("{index:02}.txt"));
        }
        let mut st = FmState::new(&td.root);
        for _ in 0..4 {
            st.move_down();
        }
        st.sync_viewport(2);
        assert_eq!((st.cursor, st.viewport_start), (4, 3));

        fs::remove_file(td.root.join("03.txt")).expect("remove penultimate file");
        fs::remove_file(td.root.join("04.txt")).expect("remove selected file");
        st.reload();
        st.sync_viewport(2);
        assert_eq!((st.cursor, st.viewport_start), (2, 1));

        st.sync_viewport(0);
        assert_eq!(st.viewport_start, 0);

        for index in 0..3 {
            fs::remove_file(td.root.join(format!("{index:02}.txt")))
                .expect("empty directory fixture");
        }
        st.reload();
        st.sync_viewport(2);
        assert_eq!((st.cursor, st.viewport_start), (0, 0));
    }

    // TP-A3.2-VIEWPORT: changing directory must not carry an unrelated scroll
    // anchor into the child or parent listing.
    #[test]
    fn enter_and_leave_normalize_viewport_for_new_directory() {
        let td = TempDir::new("viewport-directory-change");
        td.dir("sub");
        td.file("sibling.txt");
        for index in 0..4 {
            fs::write(td.root.join("sub").join(format!("{index:02}.txt")), b"x")
                .expect("write child entry");
        }
        let mut st = FmState::new(&td.root);
        st.viewport_start = usize::MAX;
        let sub = td.root.join("sub");
        let sub_index = st
            .entries
            .iter()
            .position(|entry| entry.path == sub)
            .expect("sub directory row");
        assert!(st.select(sub_index));

        st.enter();
        st.sync_viewport(2);
        assert_eq!(st.cwd, sub);
        assert_eq!((st.cursor, st.viewport_start), (0, 0));

        st.viewport_start = usize::MAX;
        st.leave();
        st.sync_viewport(2);
        assert_eq!(st.cwd, td.root);
        assert_eq!(
            st.selected().map(|entry| entry.path.as_path()),
            Some(sub.as_path())
        );
        assert_eq!(st.viewport_start, 0);
    }

    // T-A1.3c: toggling hidden re-reads and changes what is visible.
    #[test]
    fn toggle_hidden_reveals_and_hides_dotfiles() {
        let td = TempDir::new("state-hidden");
        td.file("shown");
        td.file(".hidden");
        let mut st = FmState::new(&td.root);
        assert_eq!(st.entries.len(), 1);
        st.toggle_hidden();
        assert!(st.show_hidden);
        assert_eq!(st.entries.len(), 2);
        st.toggle_hidden();
        assert!(!st.show_hidden);
        assert_eq!(st.entries.len(), 1);
    }

    // TP-TRAIL-T7-BRIDGE-05: rebuilding after a hidden-toggle must update the
    // snapshot bridge's future directory-read policy, not only its current
    // cloned entries.
    #[test]
    fn hidden_toggle_propagates_to_future_trail_branches() {
        let td = TempDir::new("trail-hidden-policy");
        let alpha = td.root.join("alpha");
        let nested = alpha.join("nested");
        fs::create_dir_all(&nested).expect("create nested fixture");
        fs::write(nested.join(".secret"), b"x").expect("write hidden fixture");
        fs::write(nested.join("visible"), b"x").expect("write visible fixture");
        set_modified(&nested.join(".secret"), fixed_time(10));
        set_modified(&nested.join("visible"), fixed_time(10));
        let mut state = FmState::new(&td.root);

        state.toggle_hidden();
        assert!(state.show_hidden);
        assert_eq!(
            state
                .trail_snapshots
                .select_dir(&mut state.trail, 1, &nested),
            FmDirectoryStatus::Available
        );
        assert_eq!(
            names(state.trail_snapshots.cols()[2].entries()),
            vec![".secret", "visible"],
            "future trail branches inherit the active hidden-file policy"
        );
    }

    // TP-FMP-RES-01 / TP-FMP-TRAIL-01: re-activating the exact child whose
    // loaded snapshot is already the next Trail column must be a disk-free
    // state transition. This covers both the child listing and the temporary
    // legacy operation projection (parent/preview included).
    #[test]
    fn resident_ancestor_activation_rebranches_without_filesystem_reads() {
        let td = TempDir::new("fmp-resident-ancestor");
        let alpha = td.root.join("alpha");
        let beta = alpha.join("beta");
        fs::create_dir_all(&beta).expect("create resident ancestor fixture");
        fs::write(beta.join("leaf.txt"), b"leaf").expect("write resident leaf");
        let mut state =
            FmState::open_trail_to(&td.root, &beta, false).expect("open resident deep Trail");
        let alpha_index = state.trail_snapshots.cols()[0]
            .entries()
            .iter()
            .position(|entry| entry.path == alpha)
            .expect("alpha row in root snapshot");

        let (outcome, profile) = crate::render_prof::observe_for_test(|| {
            state.activate_trail_entry(0, alpha_index, &alpha)
        });

        assert_eq!(outcome, trail_snapshots::TrailActivateOutcome::Branched);
        assert_eq!(
            state
                .trail
                .cols()
                .iter()
                .map(|col| col.directory.as_path())
                .collect::<Vec<_>>(),
            vec![td.root.as_path(), alpha.as_path()]
        );
        assert_eq!(state.trail.active_col(), 1);
        assert_eq!(
            profile.counter("fm.filesystem.read"),
            0,
            "resident child, parent, and preview projections must reuse prepared snapshots"
        );
    }

    // TP-A3.1: entering a selected directory reads its contents, cursor at top.
    #[test]
    fn enter_directory_appends_segment_and_focuses_child() {
        let td = TempDir::new("enter");
        td.dir("sub");
        td.file("top.txt");
        fs::write(td.root.join("sub").join("child.txt"), b"x").expect("write child");
        let mut st = FmState::new(&td.root);
        // Directories sort first, so "sub" is selected at the top.
        assert_eq!(st.selected().map(|e| e.name.as_str()), Some("sub"));
        let chain_len = st.miller.chain.len();

        st.enter();

        assert_eq!(st.cwd, td.root.join("sub"));
        assert_eq!(st.cursor, 0);
        assert!(st.entries.iter().any(|e| e.name == "child.txt"));
        assert_eq!(st.miller.chain.len(), chain_len + 1);
        assert_eq!(st.miller.focused_directory, st.cwd);
        assert_eq!(
            st.miller.chain.back().map(|segment| &segment.directory),
            Some(&st.cwd)
        );
        st.miller.assert_miller_invariants_for_test();
    }

    // TP-TRAIL-T7-BRIDGE-06: legacy directory navigation may update cwd, but
    // the canonical trail keeps its root and accumulates the exact ancestor
    // chain across enter/leave transitions.
    #[test]
    fn trail_bridge_accumulates_across_enter_and_leave() {
        let td = TempDir::new("trail-navigation");
        let alpha = td.root.join("alpha");
        let beta = alpha.join("beta");
        fs::create_dir_all(&beta).expect("create nested directories");
        fs::write(beta.join("leaf.txt"), b"x").expect("write leaf");
        let mut state = FmState::new(&td.root);

        assert_eq!(
            state
                .trail
                .cols()
                .iter()
                .map(|col| col.directory.as_path())
                .collect::<Vec<_>>(),
            vec![td.root.as_path(), alpha.as_path()]
        );

        state.enter();
        assert_eq!(state.cwd, alpha);
        assert_eq!(state.trail.selected_path(), Some(beta.as_path()));
        assert_eq!(
            state
                .trail
                .cols()
                .iter()
                .map(|col| col.directory.as_path())
                .collect::<Vec<_>>(),
            vec![td.root.as_path(), alpha.as_path(), beta.as_path()]
        );

        state.enter();
        assert_eq!(state.cwd, beta);
        assert_eq!(
            state.trail.selected_path(),
            Some(beta.join("leaf.txt").as_path())
        );
        assert_eq!(
            state.trail.cols()[0].directory,
            td.root,
            "enter never rebases the trail root"
        );

        state.leave();
        assert_eq!(state.cwd, alpha);
        assert_eq!(state.trail.selected_path(), Some(beta.as_path()));
        assert_eq!(
            state
                .trail
                .cols()
                .iter()
                .map(|col| col.directory.as_path())
                .collect::<Vec<_>>(),
            vec![td.root.as_path(), alpha.as_path(), beta.as_path()]
        );
    }

    // TP-FM4-PURE-REQUEST: input first creates an immutable, generation-bound
    // transition request. Request construction cannot read disk or mutate
    // selection, preview, chain, cache, cursor, or any generation.
    #[test]
    fn enter_navigation_request_is_exact_and_state_pure() {
        let mut state = FmState::test_empty("/virtual/current");
        let child = PathBuf::from("/virtual/current/child");
        state.entries = vec![FileEntry {
            name: "child".into(),
            path: child.clone(),
            kind: if true {
                crate::fm::entry_kind::FileEntryKind::Directory
            } else {
                crate::fm::entry_kind::FileEntryKind::RegularFile
            },
            modified: None,
        }];
        state.rebuild_trail_bridge();
        state.directory_generation = 17;
        state.preview_generation = 23;
        state.miller.revision = 31;
        let before = state.clone();

        assert_eq!(
            state.request_enter_navigation(),
            Some(FmNavigationRequest {
                reason: FmNavigationReason::Enter,
                source_directory: PathBuf::from("/virtual/current"),
                source_directory_generation: 17,
                source_preview_generation: 23,
                source_miller_revision: 31,
                target_directory: child,
                focus_path: None,
                fallback_cursor: 0,
                show_hidden: false,
            })
        );
        assert_eq!(state.cwd, before.cwd);
        assert_eq!(state.entries, before.entries);
        assert_eq!(state.cursor, before.cursor);
        assert_eq!(state.viewport_start, before.viewport_start);
        assert_eq!(state.directory_generation, before.directory_generation);
        assert_eq!(state.preview, before.preview);
        assert_eq!(state.preview_generation, before.preview_generation);
        assert_eq!(state.miller, before.miller);
    }

    // TP-FM4-PURE-APPLY: the apply half consumes only a prepared payload.
    // Virtual paths deliberately do not exist on disk; success proves this
    // transition does not fall back to filesystem reads.
    #[test]
    fn prepared_navigation_applies_virtual_payload_without_filesystem_access() {
        let current = PathBuf::from("/virtual/current");
        let child = current.join("child");
        let inside = child.join("inside.txt");
        let mut state = FmState::test_empty(current.clone());
        state.entries = vec![FileEntry {
            name: "child".into(),
            path: child.clone(),
            kind: if true {
                crate::fm::entry_kind::FileEntryKind::Directory
            } else {
                crate::fm::entry_kind::FileEntryKind::RegularFile
            },
            modified: None,
        }];
        state.rebuild_trail_bridge();
        let request = state
            .request_enter_navigation()
            .expect("pure enter request");
        let preview_generation = request.source_preview_generation + 1;
        let prepared = FmPreparedNavigation {
            request,
            entries: vec![FileEntry {
                name: "inside.txt".into(),
                path: inside.clone(),
                kind: if false {
                    crate::fm::entry_kind::FileEntryKind::Directory
                } else {
                    crate::fm::entry_kind::FileEntryKind::RegularFile
                },
                modified: None,
            }],
            status: FmDirectoryStatus::Available,
            omissions: FmDirectoryOmissions::default(),
            writable: true,
            cursor: 0,
            parent: Some(FmParent {
                entries: vec![FileEntry {
                    name: "child".into(),
                    path: child.clone(),
                    kind: if true {
                        crate::fm::entry_kind::FileEntryKind::Directory
                    } else {
                        crate::fm::entry_kind::FileEntryKind::RegularFile
                    },
                    modified: None,
                }],
                cursor: Some(0),
            }),
            preview: FmPreview::File(FmFilePreview::Unavailable(TextPreviewError::Io(
                std::io::ErrorKind::NotFound,
            ))),
            preview_generation,
        };

        assert!(state.apply_prepared_navigation(prepared.clone()));
        assert_eq!(state.cwd, child);
        assert_eq!(state.entries, prepared.entries);
        assert_eq!(state.cursor, 0);
        assert_eq!(state.viewport_start, 0);
        assert_eq!(state.cwd_status, FmDirectoryStatus::Available);
        assert!(state.cwd_writable);
        assert_eq!(state.directory_generation, 2);
        assert_eq!(state.parent, prepared.parent);
        assert_eq!(state.preview, prepared.preview);
        assert_eq!(state.preview_generation, preview_generation);
        assert_eq!(state.miller.focused_directory, state.cwd);
        assert_eq!(
            state.miller.chain.back().map(|segment| &segment.directory),
            Some(&state.cwd)
        );
        assert!(state.multi_selection_paths().is_empty());
        state.miller.assert_miller_invariants_for_test();
    }

    // P5 RED: a prepared watcher/current refresh must be a disk-free model
    // apply that reconciles cursor authority by stable path identity.
    #[test]
    fn prepared_current_refresh_applies_by_stable_path_without_io() {
        let root = PathBuf::from("/virtual/current-refresh");
        let selected_path = root.join("b.txt");
        let mut state = FmState::test_empty(&root);
        state.entries = vec![
            FileEntry {
                name: "a.txt".to_string(),
                path: root.join("a.txt"),
                kind: if false {
                    crate::fm::entry_kind::FileEntryKind::Directory
                } else {
                    crate::fm::entry_kind::FileEntryKind::RegularFile
                },
                modified: None,
            },
            FileEntry {
                name: "b.txt".to_string(),
                path: selected_path.clone(),
                kind: if false {
                    crate::fm::entry_kind::FileEntryKind::Directory
                } else {
                    crate::fm::entry_kind::FileEntryKind::RegularFile
                },
                modified: None,
            },
        ];
        state.cursor = 1;
        state.viewport_start = 1;
        state.directory_generation = 11;
        state.preview_generation = 17;
        state.multi_selection.paths = BTreeSet::from([root.join("a.txt"), selected_path.clone()]);
        state.multi_selection.anchor = Some(selected_path.clone());
        state.rebuild_trail_bridge();

        let request = state.request_hidden_toggle(29);
        assert_eq!(request.reason, FmCurrentRefreshReason::ToggleHidden);
        assert_eq!(request.files_generation, 29);
        assert_eq!(request.source_directory, root);
        assert_eq!(request.source_directory_generation, 11);
        assert_eq!(request.source_preview_generation, 17);
        assert_eq!(request.source_miller_revision, state.miller.revision);
        assert_eq!(
            request.selected_path.as_deref(),
            Some(selected_path.as_path())
        );
        assert_eq!(request.fallback_cursor, 1);
        assert!(!request.source_show_hidden);
        assert!(request.target_show_hidden);

        let prepared = FmPreparedCurrentRefresh {
            request,
            entries: vec![
                FileEntry {
                    name: "b.txt".to_string(),
                    path: selected_path.clone(),
                    kind: if false {
                        crate::fm::entry_kind::FileEntryKind::Directory
                    } else {
                        crate::fm::entry_kind::FileEntryKind::RegularFile
                    },
                    modified: None,
                },
                FileEntry {
                    name: "c.txt".to_string(),
                    path: root.join("c.txt"),
                    kind: if false {
                        crate::fm::entry_kind::FileEntryKind::Directory
                    } else {
                        crate::fm::entry_kind::FileEntryKind::RegularFile
                    },
                    modified: None,
                },
            ],
            status: FmDirectoryStatus::Available,
            omissions: FmDirectoryOmissions::default(),
            writable: true,
            cursor: 0,
            parent: None,
            preview: FmPreview::None,
            preview_generation: 18,
        };

        assert!(
            state.apply_prepared_current_refresh(prepared, 29),
            "current generation-bound refresh must apply"
        );
        assert_eq!(state.cursor, 0);
        assert_eq!(
            state.selected().map(|entry| entry.path.as_path()),
            Some(selected_path.as_path())
        );
        assert_eq!(state.directory_generation, 12);
        assert_eq!(state.preview_generation, 18);
        assert_eq!(state.viewport_start, 0);
        assert!(state.cwd_writable);
        assert_eq!(
            state.multi_selection_paths(),
            &BTreeSet::from([selected_path.clone()])
        );
        assert_eq!(
            state.multi_selection_anchor(),
            Some(selected_path.as_path())
        );
        state.miller.assert_miller_invariants_for_test();
    }

    // P5 AUTHORITY: each request identity and each structural payload oracle
    // independently fails closed without a partial model mutation.
    #[test]
    fn prepared_current_refresh_rejects_each_stale_authority() {
        let root = PathBuf::from("/virtual/current-refresh-stale");
        let selected_path = root.join("b.txt");
        let mut base = FmState::test_empty(&root);
        base.entries = vec![FileEntry {
            name: "b.txt".to_string(),
            path: selected_path.clone(),
            kind: if false {
                crate::fm::entry_kind::FileEntryKind::Directory
            } else {
                crate::fm::entry_kind::FileEntryKind::RegularFile
            },
            modified: None,
        }];
        base.directory_generation = 11;
        base.preview_generation = 17;
        base.miller.revision = 23;
        let request = base.request_hidden_toggle(29);
        let prepared = FmPreparedCurrentRefresh {
            request,
            entries: vec![
                FileEntry {
                    name: "b.txt".to_string(),
                    path: selected_path,
                    kind: if false {
                        crate::fm::entry_kind::FileEntryKind::Directory
                    } else {
                        crate::fm::entry_kind::FileEntryKind::RegularFile
                    },
                    modified: None,
                },
                FileEntry {
                    name: "c.txt".to_string(),
                    path: root.join("c.txt"),
                    kind: if false {
                        crate::fm::entry_kind::FileEntryKind::Directory
                    } else {
                        crate::fm::entry_kind::FileEntryKind::RegularFile
                    },
                    modified: None,
                },
            ],
            status: FmDirectoryStatus::Available,
            omissions: FmDirectoryOmissions::default(),
            writable: true,
            cursor: 0,
            parent: None,
            preview: FmPreview::None,
            preview_generation: 18,
        };

        for case in 0..9 {
            let mut state = base.clone();
            let mut candidate = prepared.clone();
            let mut current_files_generation = 29;
            let label = match case {
                0 => {
                    current_files_generation += 1;
                    "files generation"
                }
                1 => {
                    state.cwd = PathBuf::from("/virtual/other");
                    "cwd"
                }
                2 => {
                    state.directory_generation += 1;
                    "directory generation"
                }
                3 => {
                    state.preview_generation += 1;
                    "preview generation"
                }
                4 => {
                    state.miller.revision += 1;
                    "Miller revision"
                }
                5 => {
                    state.show_hidden = !state.show_hidden;
                    "hidden preference"
                }
                6 => {
                    candidate.preview_generation += 1;
                    "prepared preview generation"
                }
                7 => {
                    candidate.cursor = 1;
                    "prepared cursor"
                }
                8 => {
                    candidate.status = FmDirectoryStatus::Missing;
                    "status payload"
                }
                _ => unreachable!(),
            };
            let before = state.clone();

            assert!(
                !state.apply_prepared_current_refresh(candidate, current_files_generation),
                "{label} drift must reject the prepared payload"
            );
            assert_eq!(state.cwd, before.cwd, "{label}");
            assert_eq!(state.entries, before.entries, "{label}");
            assert_eq!(state.cursor, before.cursor, "{label}");
            assert_eq!(
                state.directory_generation, before.directory_generation,
                "{label}"
            );
            assert_eq!(state.parent, before.parent, "{label}");
            assert_eq!(state.preview, before.preview, "{label}");
            assert_eq!(
                state.preview_generation, before.preview_generation,
                "{label}"
            );
            assert_eq!(state.miller, before.miller, "{label}");
        }
    }

    // TP-FM4-STALE-APPLY: preparation can complete after a watcher refresh or
    // another navigation. Any source-generation drift retires the payload
    // without partially moving entries, cache ownership, or focus.
    #[test]
    fn stale_prepared_navigation_is_rejected_without_state_mutation() {
        let current = PathBuf::from("/virtual/current");
        let child = current.join("child");
        let mut state = FmState::test_empty(current);
        state.entries = vec![FileEntry {
            name: "child".into(),
            path: child.clone(),
            kind: if true {
                crate::fm::entry_kind::FileEntryKind::Directory
            } else {
                crate::fm::entry_kind::FileEntryKind::RegularFile
            },
            modified: None,
        }];
        state.rebuild_trail_bridge();
        let request = state
            .request_enter_navigation()
            .expect("pure enter request");
        let prepared = FmPreparedNavigation {
            preview_generation: request.source_preview_generation + 1,
            request,
            entries: Vec::new(),
            status: FmDirectoryStatus::Available,
            omissions: FmDirectoryOmissions::default(),
            writable: true,
            cursor: 0,
            parent: None,
            preview: FmPreview::None,
        };
        state.directory_generation = state.directory_generation.saturating_add(1);
        let before = state.clone();

        assert!(!state.apply_prepared_navigation(prepared));
        assert_eq!(state.cwd, before.cwd);
        assert_eq!(state.entries, before.entries);
        assert_eq!(state.cursor, before.cursor);
        assert_eq!(state.directory_generation, before.directory_generation);
        assert_eq!(state.parent, before.parent);
        assert_eq!(state.preview, before.preview);
        assert_eq!(state.preview_generation, before.preview_generation);
        assert_eq!(state.miller, before.miller);
    }

    // TP-FM4-ENTER-FAILURE: directory identity can disappear between the
    // prepared row snapshot and activation. Failed preparation must leave the
    // last valid current directory and every generation/branch authority
    // unchanged instead of publishing a missing ghost current.
    #[test]
    fn missing_selected_child_does_not_corrupt_current_chain() {
        let td = TempDir::new("enter-missing-child");
        td.dir("child");
        td.file("sibling.txt");
        let child = td.root.join("child");
        let mut state = FmState::new(&td.root);
        let child_index = state
            .entries
            .iter()
            .position(|entry| entry.path == child)
            .expect("prepared child");
        assert!(state.replace_selection(child_index));
        let before = state.clone();

        fs::remove_dir(&child).expect("remove child after projection");
        state.enter();

        assert_eq!(state.cwd, before.cwd);
        assert_eq!(state.entries, before.entries);
        assert_eq!(state.cursor, before.cursor);
        assert_eq!(state.viewport_start, before.viewport_start);
        assert_eq!(state.cwd_status, before.cwd_status);
        assert_eq!(state.cwd_writable, before.cwd_writable);
        assert_eq!(state.directory_generation, before.directory_generation);
        assert_eq!(state.parent, before.parent);
        assert_eq!(state.preview, before.preview);
        assert_eq!(state.preview_viewport_start, before.preview_viewport_start);
        assert_eq!(state.preview_generation, before.preview_generation);
        assert_eq!(
            state.multi_selection_paths(),
            before.multi_selection_paths()
        );
        assert_eq!(
            state.multi_selection_anchor(),
            before.multi_selection_anchor()
        );
        assert_eq!(state.miller, before.miller);
    }

    // P5 REAL I/O FAILURE: the selected directory remains a valid projected
    // row while its permissions change before preparation. A denied open must
    // preserve the last valid current directory and every branch/generation
    // authority. The permission is restored before assertions/cleanup.
    #[cfg(unix)]
    #[test]
    fn permission_denied_child_projects_error_without_chain_corruption() {
        use std::os::unix::fs::PermissionsExt;

        let td = TempDir::new("enter-permission-denied-child");
        td.dir("child");
        td.file("child/inside.txt");
        td.file("sibling.txt");
        let child = td.root.join("child");
        let mut state = FmState::new(&td.root);
        let child_index = state
            .entries
            .iter()
            .position(|entry| entry.path == child)
            .expect("prepared child");
        assert!(state.replace_selection(child_index));
        let before = state.clone();

        fs::set_permissions(&child, fs::Permissions::from_mode(0o000))
            .expect("deny child directory access");
        state.enter();
        fs::set_permissions(&child, fs::Permissions::from_mode(0o700))
            .expect("restore child directory access");

        assert_eq!(state.cwd, before.cwd);
        assert_eq!(state.entries, before.entries);
        assert_eq!(state.cursor, before.cursor);
        assert_eq!(state.viewport_start, before.viewport_start);
        assert_eq!(state.cwd_status, before.cwd_status);
        assert_eq!(state.cwd_writable, before.cwd_writable);
        assert_eq!(state.directory_generation, before.directory_generation);
        assert_eq!(state.parent, before.parent);
        assert_eq!(state.preview, before.preview);
        assert_eq!(state.preview_viewport_start, before.preview_viewport_start);
        assert_eq!(state.preview_generation, before.preview_generation);
        assert_eq!(
            state.multi_selection_paths(),
            before.multi_selection_paths()
        );
        assert_eq!(
            state.multi_selection_anchor(),
            before.multi_selection_anchor()
        );
        assert_eq!(state.miller, before.miller);
        state.miller.assert_miller_invariants_for_test();
    }

    // TP-A3.2: entering a file selection is a no-op.
    #[test]
    fn enter_on_file_is_noop() {
        let td = TempDir::new("enterfile");
        td.file("only.txt");
        let mut st = FmState::new(&td.root);
        assert_eq!(st.selected().map(|e| e.name.as_str()), Some("only.txt"));

        let before = st.cwd.clone();
        st.enter();

        assert_eq!(st.cwd, before, "entering a file does not change directory");
    }

    // TP-A3.3: leaving goes to the parent and focuses the departed child.
    #[test]
    fn leave_ascends_to_parent() {
        let td = TempDir::new("leave");
        td.dir("sub");
        let mut st = FmState::new(td.root.join("sub"));

        st.leave();

        assert_eq!(st.cwd, td.root);
        assert_eq!(st.cursor, 0);
    }

    // TP-FM4-LEAVE-FAILURE: an external rename can retire the prepared parent
    // path while the current directory is still displayed. Failed parent
    // preparation must not replace the last valid model with a missing branch.
    #[test]
    fn missing_parent_does_not_corrupt_current_chain_on_leave() {
        let td = TempDir::new("leave-missing-parent");
        td.dir("parent/child");
        td.file("parent/child/inside.txt");
        let parent = td.root.join("parent");
        let child = parent.join("child");
        let moved_parent = td.root.join("moved-parent");
        let mut state = FmState::new(&child);
        let before = state.clone();

        fs::rename(&parent, &moved_parent).expect("retire prepared parent path");
        state.leave();

        assert_eq!(state.cwd, before.cwd);
        assert_eq!(state.entries, before.entries);
        assert_eq!(state.cursor, before.cursor);
        assert_eq!(state.viewport_start, before.viewport_start);
        assert_eq!(state.cwd_status, before.cwd_status);
        assert_eq!(state.cwd_writable, before.cwd_writable);
        assert_eq!(state.directory_generation, before.directory_generation);
        assert_eq!(state.parent, before.parent);
        assert_eq!(state.preview, before.preview);
        assert_eq!(state.preview_viewport_start, before.preview_viewport_start);
        assert_eq!(state.preview_generation, before.preview_generation);
        assert_eq!(
            state.multi_selection_paths(),
            before.multi_selection_paths()
        );
        assert_eq!(
            state.multi_selection_anchor(),
            before.multi_selection_anchor()
        );
        assert_eq!(state.miller, before.miller);
    }

    // TP-A3.4: leaving at the filesystem root is a no-op (no panic).
    #[test]
    fn leave_at_root_is_noop() {
        let mut st = FmState::new("/");
        st.leave();
        assert_eq!(st.cwd, PathBuf::from("/"));
    }

    // TP-N2.1-PATH: leaving must transfer the exact departed child identity
    // into the new current list instead of reusing the unrelated row zero.
    #[test]
    fn leave_focuses_exact_parent_and_preserves_valid_child_focus() {
        let td = TempDir::new("leave-path-focus");
        td.dir("alpha");
        td.dir("target");
        td.dir("zulu");
        let target = td.root.join("target");
        let mut state = FmState::new(&target);

        state.leave();

        assert_eq!(state.cwd, td.root);
        assert_eq!(
            state.selected().map(|entry| &entry.path),
            Some(&target),
            "leave follows the departed path rather than its former row index"
        );
        assert_eq!(state.miller.focused_directory, td.root);
        assert_eq!(
            state.miller.chain.back().map(|segment| &segment.directory),
            Some(&td.root)
        );
        state.miller.assert_miller_invariants_for_test();
    }

    // TP-N2.1-PATH: directory-boundary selection cleanup and preview
    // generation must converge on the same departed-child focus authority.
    #[test]
    fn leave_focuses_child_preview_and_clears_selection() {
        let td = TempDir::new("leave-preview-selection");
        td.dir("alpha");
        td.dir("target");
        td.file("target/inside.txt");
        let target = td.root.join("target");
        let mut state = FmState::new(&target);
        assert!(state.replace_selection(0));
        let generation_before = state.preview_generation;

        state.leave();

        assert!(state.multi_selection_paths().is_empty());
        assert!(state.multi_selection_anchor().is_none());
        assert_eq!(state.selected().map(|entry| &entry.path), Some(&target));
        assert!(state.preview_generation > generation_before);
        match &state.preview {
            FmPreview::Directory(entries) => assert_eq!(names(entries), vec!["inside.txt"]),
            other => panic!("departed directory needs its own preview, got {other:?}"),
        }
    }

    // TP-N2.1-PATH: watcher-style reorder follows exact path identity; a later
    // delete falls back to a live bounded row without retaining the old path.
    #[test]
    fn leave_focus_survives_parent_reorder_and_deletion() {
        let td = TempDir::new("leave-reorder-delete");
        td.dir("alpha");
        td.dir("target");
        td.dir("zulu");
        let target = td.root.join("target");
        let mut state = FmState::new(&target);

        state.leave();
        assert_eq!(state.selected().map(|entry| &entry.path), Some(&target));

        td.dir("bravo");
        state.reload();
        assert_eq!(state.selected().map(|entry| &entry.path), Some(&target));

        fs::remove_dir(&target).expect("remove departed child");
        state.reload();
        assert!(state.cursor < state.entries.len());
        assert_ne!(state.selected().map(|entry| &entry.path), Some(&target));
        assert!(
            state.entries.iter().all(|entry| entry.path != target),
            "deleted departed identity is not retained"
        );
    }

    // TP-N2.1-PATH: geometry normalization owns scrolling after navigation;
    // leave only restores focus and the bounded viewport contains that cursor.
    #[test]
    fn leave_focus_scrolls_into_bounded_viewport() {
        let td = TempDir::new("leave-viewport-focus");
        for name in ["alpha", "bravo", "charlie", "delta", "target"] {
            td.dir(name);
        }
        let target = td.root.join("target");
        let mut state = FmState::new(&target);

        state.leave();
        state.sync_viewport(2);

        assert_eq!(state.selected().map(|entry| &entry.path), Some(&target));
        assert!(state.viewport_start <= state.cursor);
        assert!(state.cursor < state.viewport_start + 2);
        assert!(state.viewport_start <= state.entries.len().saturating_sub(2));
    }

    // TP-N2.1-PATH: a raced-away or filtered departed child does not create a
    // synthetic row. Navigation reaches the parent with the existing fallback.
    #[test]
    fn leave_missing_or_hidden_child_uses_top_fallback() {
        let td = TempDir::new("leave-missing-hidden");
        td.dir("visible");

        let missing = td.root.join("missing");
        let mut missing_state = FmState::new(&missing);
        missing_state.leave();
        assert_eq!(missing_state.cwd, td.root);
        assert_eq!(missing_state.cursor, 0);
        assert!(missing_state
            .entries
            .iter()
            .all(|entry| entry.path != missing));

        td.dir(".hidden");
        let hidden = td.root.join(".hidden");
        let mut hidden_state = FmState::with_hidden(&hidden, false);
        hidden_state.leave();
        assert_eq!(hidden_state.cwd, td.root);
        assert_eq!(hidden_state.cursor, 0);
        assert!(hidden_state
            .entries
            .iter()
            .all(|entry| entry.path != hidden));
        assert_eq!(
            hidden_state.selected().map(|entry| entry.name.as_str()),
            Some("visible")
        );
    }

    // TP-N2.1-PATH: root leave is a complete state no-op, including selection,
    // prepared context, viewport, and preview generation lifecycle.
    #[test]
    fn leave_at_root_preserves_complete_state() {
        let mut state = synthetic_state(1);
        state.cwd = PathBuf::from("/");
        state.viewport_start = 0;
        state.preview_generation = 41;
        state.preview = FmPreview::Directory(state.entries.clone());
        assert!(state.replace_selection(0));

        let cwd = state.cwd.clone();
        let entries = state.entries.clone();
        let cursor = state.cursor;
        let viewport_start = state.viewport_start;
        let show_hidden = state.show_hidden;
        let cwd_writable = state.cwd_writable;
        let cwd_status = state.cwd_status;
        let parent = state.parent.clone();
        let preview = state.preview.clone();
        let preview_generation = state.preview_generation;
        let selected_paths = state.multi_selection_paths().clone();
        let selection_anchor = state.multi_selection_anchor().map(Path::to_path_buf);

        state.leave();

        assert_eq!(state.cwd, cwd);
        assert_eq!(state.entries, entries);
        assert_eq!(state.cursor, cursor);
        assert_eq!(state.viewport_start, viewport_start);
        assert_eq!(state.show_hidden, show_hidden);
        assert_eq!(state.cwd_writable, cwd_writable);
        assert_eq!(state.cwd_status, cwd_status);
        assert_eq!(state.parent, parent);
        assert_eq!(state.preview, preview);
        assert_eq!(state.preview_generation, preview_generation);
        assert_eq!(state.multi_selection_paths(), &selected_paths);
        assert_eq!(state.multi_selection_anchor(), selection_anchor.as_deref());
    }

    // Prepared context remains render-independent for operations and detail.
    // TP-A2.2.2/3: Miller context is loaded into pure state before render. The
    // parent cursor identifies cwd and a selected directory exposes its child
    // entries without filesystem access from the renderer.
    #[test]
    fn miller_context_loads_parent_cursor_and_directory_preview() {
        let td = TempDir::new("miller-context");
        td.dir("work");
        fs::create_dir_all(td.root.join("work").join("child")).expect("create child");
        fs::write(td.root.join("work").join("child").join("inside.txt"), b"x")
            .expect("write preview file");
        let st = FmState::new(td.root.join("work"));

        let parent = st.parent.as_ref().expect("parent context");
        let parent_cursor = parent.cursor.expect("cwd in parent entries");
        assert_eq!(parent.entries[parent_cursor].name, "work");
        match &st.preview {
            FmPreview::Directory(entries) => {
                assert!(entries.iter().any(|entry| entry.name == "inside.txt"));
            }
            other => panic!("directory selection needs directory preview, got {other:?}"),
        }
    }

    // Selected-file detail classification remains explicit.
    // TP-A2.2.3: a selected file is explicitly classified; it is not confused
    // with an empty directory preview.
    #[test]
    fn miller_context_classifies_file_preview() {
        let td = TempDir::new("file-context");
        td.file("only.txt");
        let st = FmState::new(&td.root);
        assert!(matches!(st.preview, FmPreview::File(_)));
    }

    // TP-B1.1-BOUNDED-READ: selected-file content is prepared in FmState,
    // before render. The renderer receives immutable text/truncation data and
    // therefore has no reason to touch the filesystem.
    #[test]
    fn fmstate_prepares_selected_text_file_outside_render() {
        let td = TempDir::new("text-preview-state");
        let path = td.root.join("sample.txt");
        fs::write(&path, "state-prepared\r\ncafé\n").expect("write state preview fixture");

        let state = FmState::new(&td.root);

        match &state.preview {
            FmPreview::File(FmFilePreview::Text(preview)) => {
                assert_eq!(preview.content, "state-prepared\r\ncafé\n");
                assert!(!preview.truncated);
            }
            other => panic!("selected text file needs prepared preview, got {other:?}"),
        }
    }

    // TP-B1.4-RELOAD: a watcher refresh that re-reads the same selected path
    // and unchanged content preserves the already prepared highlight. This
    // prevents plain/highlight flicker and duplicate CPU work after harmless
    // filesystem notifications.
    #[test]
    fn reload_preserves_highlight_for_unchanged_selected_text() {
        let td = TempDir::new("text-preview-highlight-reload");
        let path = td.root.join("sample.rs");
        fs::write(&path, "fn main() {}\n").expect("write Rust preview fixture");
        let mut state = FmState::new(&td.root);
        let prepared = match &mut state.preview {
            FmPreview::File(FmFilePreview::Text(preview)) => {
                assert_eq!(preview.source_path, path);
                let highlighted = highlight_text_preview(&path, preview);
                preview.highlighted = Some(highlighted.clone());
                highlighted
            }
            other => panic!("selected text file needs prepared preview, got {other:?}"),
        };

        state.reload();

        match &state.preview {
            FmPreview::File(FmFilePreview::Text(preview)) => {
                assert_eq!(preview.source_path, path);
                assert_eq!(preview.highlighted.as_ref(), Some(&prepared));
            }
            other => panic!("reloaded text file needs prepared preview, got {other:?}"),
        }
    }

    // TP-B1.4-PATH-IDENTITY: equal bytes are not sufficient identity. Moving
    // to a different path, especially one with a different extension, must
    // discard the old syntax result and bind the new preview to the new path.
    #[test]
    fn cursor_move_does_not_reuse_highlight_for_equal_content_at_new_path() {
        let td = TempDir::new("text-preview-highlight-path");
        let first = td.root.join("alpha.rs");
        let second = td.root.join("beta.py");
        fs::write(&first, "same bytes\n").expect("write first preview fixture");
        fs::write(&second, "same bytes\n").expect("write second preview fixture");
        let mut state = FmState::new(&td.root);
        match &mut state.preview {
            FmPreview::File(FmFilePreview::Text(preview)) => {
                preview.highlighted = Some(highlight_text_preview(&first, preview));
            }
            other => panic!("first text file needs prepared preview, got {other:?}"),
        }

        state.move_down();

        match &state.preview {
            FmPreview::File(FmFilePreview::Text(preview)) => {
                assert_eq!(preview.source_path, second);
                assert!(preview.highlighted.is_none());
            }
            other => panic!("second text file needs prepared preview, got {other:?}"),
        }
    }

    // TP-B1.4-REPLACE: replacement at the same path is still a new preview
    // generation when visible bytes change. The old highlight must not survive
    // into the new content.
    #[test]
    fn reload_replaced_selected_text_discards_old_highlight() {
        let td = TempDir::new("text-preview-highlight-replace");
        let path = td.root.join("sample.rs");
        fs::write(&path, "fn old() {}\n").expect("write original fixture");
        let mut state = FmState::new(&td.root);
        match &mut state.preview {
            FmPreview::File(FmFilePreview::Text(preview)) => {
                preview.highlighted = Some(highlight_text_preview(&path, preview));
            }
            other => panic!("original file needs prepared preview, got {other:?}"),
        }

        fs::write(&path, "fn replacement() {}\n").expect("replace selected fixture");
        state.reload();

        match &state.preview {
            FmPreview::File(FmFilePreview::Text(preview)) => {
                assert_eq!(preview.source_path, path);
                assert_eq!(preview.content, "fn replacement() {}\n");
                assert!(preview.highlighted.is_none());
            }
            other => panic!("replacement needs prepared preview, got {other:?}"),
        }
    }

    // TP-B1.4-DELETE: deleting the selected file falls forward to the nearest
    // valid row and prepares that row's content; stale source identity and
    // highlight data are both discarded.
    #[test]
    fn reload_deleted_selected_text_previews_new_selection() {
        let td = TempDir::new("text-preview-highlight-delete");
        let deleted = td.root.join("alpha.rs");
        let remaining = td.root.join("beta.py");
        fs::write(&deleted, "fn alpha() {}\n").expect("write selected fixture");
        fs::write(&remaining, "def beta():\n    pass\n").expect("write remaining fixture");
        let mut state = FmState::new(&td.root);
        match &mut state.preview {
            FmPreview::File(FmFilePreview::Text(preview)) => {
                preview.highlighted = Some(highlight_text_preview(&deleted, preview));
            }
            other => panic!("selected file needs prepared preview, got {other:?}"),
        }

        fs::remove_file(&deleted).expect("delete selected fixture");
        state.reload();

        assert_eq!(state.selected().map(|entry| &entry.path), Some(&remaining));
        match &state.preview {
            FmPreview::File(FmFilePreview::Text(preview)) => {
                assert_eq!(preview.source_path, remaining);
                assert_eq!(preview.content, "def beta():\n    pass\n");
                assert!(preview.highlighted.is_none());
            }
            other => panic!("remaining file needs prepared preview, got {other:?}"),
        }
    }

    // TP-B1.4-HIDDEN: filter reloads preserve an unchanged visible selection,
    // but hiding the selected dotfile must rebind preview identity to the next
    // visible file.
    #[test]
    fn hidden_toggle_preserves_or_rebinds_text_highlight_by_path() {
        let td = TempDir::new("text-preview-highlight-hidden");
        let hidden = td.root.join(".hidden.rs");
        let visible = td.root.join("visible.rs");
        fs::write(&hidden, "fn hidden() {}\n").expect("write hidden fixture");
        fs::write(&visible, "fn visible() {}\n").expect("write visible fixture");
        let mut state = FmState::with_hidden(&td.root, true);
        state.cursor = state
            .entries
            .iter()
            .position(|entry| entry.path == visible)
            .expect("visible selection");
        state.refresh_preview();
        let visible_highlight = match &mut state.preview {
            FmPreview::File(FmFilePreview::Text(preview)) => {
                let highlighted = highlight_text_preview(&visible, preview);
                preview.highlighted = Some(highlighted.clone());
                highlighted
            }
            other => panic!("visible file needs prepared preview, got {other:?}"),
        };

        state.toggle_hidden();
        match &state.preview {
            FmPreview::File(FmFilePreview::Text(preview)) => {
                assert_eq!(preview.source_path, visible);
                assert_eq!(preview.highlighted.as_ref(), Some(&visible_highlight));
            }
            other => panic!("visible file remains selected, got {other:?}"),
        }

        state.toggle_hidden();
        state.cursor = state
            .entries
            .iter()
            .position(|entry| entry.path == hidden)
            .expect("hidden selection");
        state.refresh_preview();
        match &mut state.preview {
            FmPreview::File(FmFilePreview::Text(preview)) => {
                preview.highlighted = Some(highlight_text_preview(&hidden, preview));
            }
            other => panic!("hidden file needs prepared preview, got {other:?}"),
        }

        state.toggle_hidden();
        match &state.preview {
            FmPreview::File(FmFilePreview::Text(preview)) => {
                assert_eq!(preview.source_path, visible);
                assert!(preview.highlighted.is_none());
            }
            other => panic!("visible fallback needs prepared preview, got {other:?}"),
        }
    }

    // Filesystem root has no parent operation context.
    // TP-A2.2.5: filesystem root has no parent context.
    #[test]
    fn miller_context_at_root_has_no_parent() {
        let st = FmState::new("/");
        assert!(st.parent.is_none());
    }

    // Hidden directory parent operation context remains exact.
    // No-happy-path: entering a dot-directory while hidden files are disabled
    // must not erase cwd from its own parent context.
    #[test]
    fn hidden_cwd_remains_visible_in_parent_context() {
        let td = TempDir::new("hidden-cwd");
        td.dir(".work");
        td.dir("visible-peer");
        let st = FmState::new(td.root.join(".work"));

        let parent = st.parent.as_ref().expect("parent context");
        let parent_cursor = parent.cursor.expect("hidden cwd in parent entries");
        assert_eq!(parent.entries[parent_cursor].name, ".work");
        assert!(parent
            .entries
            .iter()
            .any(|entry| entry.name == "visible-peer"));
    }

    // Detail refresh follows the exact selected path.
    // TP-A4.3: a refresh follows the selected path across re-sorting and
    // rebuilds the right Miller column from the resulting selection.
    #[test]
    fn reload_preserves_selected_path_and_refreshes_preview_context() {
        let td = TempDir::new("watch-selection-preserve");
        td.dir("selected");
        fs::write(td.root.join("selected").join("inside.txt"), b"x")
            .expect("write selected directory child");
        td.file("z.txt");
        set_equal_modified(&td, &["selected", "z.txt"]);
        let mut state = FmState::new(&td.root);
        let selected_path = td.root.join("selected");
        assert_eq!(
            state.selected().map(|entry| &entry.path),
            Some(&selected_path)
        );

        td.dir("ahead");
        state.reload();

        assert_eq!(
            state.selected().map(|entry| &entry.path),
            Some(&selected_path)
        );
        match &state.preview {
            FmPreview::Directory(entries) => {
                assert_eq!(names(entries), vec!["inside.txt"]);
            }
            other => panic!("preserved directory needs refreshed preview, got {other:?}"),
        }
    }

    // TP-A4.3: when the selected path disappears, retain the nearest valid row;
    // when all rows disappear, clamp to zero and clear preview state.
    #[test]
    fn reload_deleted_selection_uses_nearest_row_then_handles_empty_directory() {
        let td = TempDir::new("watch-selection-delete");
        td.file("a.txt");
        td.file("b.txt");
        td.file("c.txt");
        set_equal_modified(&td, &["a.txt", "b.txt", "c.txt"]);
        let mut state = FmState::new(&td.root);
        assert!(state.select(1));
        assert_eq!(
            state.selected().map(|entry| entry.name.as_str()),
            Some("b.txt")
        );

        fs::remove_file(td.root.join("b.txt")).expect("remove selected file");
        state.reload();
        assert_eq!(state.cursor, 1);
        assert_eq!(
            state.selected().map(|entry| entry.name.as_str()),
            Some("c.txt")
        );
        assert!(matches!(state.preview, FmPreview::File(_)));

        fs::remove_file(td.root.join("a.txt")).expect("remove first file");
        fs::remove_file(td.root.join("c.txt")).expect("remove last file");
        state.reload();
        assert_eq!(state.cursor, 0);
        assert!(state.selected().is_none());
        assert!(matches!(state.preview, FmPreview::None));
    }

    // TP-A4.3: a rename removes the exact old path, so fallback is the old row
    // index (or its clamped predecessor), never an out-of-range cursor.
    #[test]
    fn reload_renamed_selection_falls_back_to_safe_row() {
        let td = TempDir::new("watch-selection-rename");
        td.file("a.txt");
        td.file("b.txt");
        td.file("c.txt");
        set_equal_modified(&td, &["a.txt", "b.txt", "c.txt"]);
        let mut state = FmState::new(&td.root);
        assert!(state.select(1));

        fs::rename(td.root.join("b.txt"), td.root.join("z.txt")).expect("rename selected file");
        state.reload();

        assert_eq!(state.cursor, 1);
        assert_eq!(
            state.selected().map(|entry| entry.name.as_str()),
            Some("c.txt")
        );
    }

    // TP-A4.3: changing the hidden filter must preserve a still-visible path,
    // even when removing a dotfile changes its row index.
    #[test]
    fn toggle_hidden_preserves_selection_that_remains_visible() {
        let td = TempDir::new("watch-selection-hidden");
        td.file(".hidden");
        td.file("a.txt");
        td.file("z.txt");
        let mut state = FmState::with_hidden(&td.root, true);
        let selected_index = state
            .entries
            .iter()
            .position(|entry| entry.name == "a.txt")
            .expect("visible selection");
        assert!(state.select(selected_index));
        let selected_path = td.root.join("a.txt");

        state.toggle_hidden();

        assert!(!state.show_hidden);
        assert_eq!(
            state.selected().map(|entry| &entry.path),
            Some(&selected_path)
        );
    }

    // TP-B1.1-BOUNDED-READ: an in-limit text file remains byte-for-byte UTF-8,
    // including CRLF. The preview model records that no truncation occurred.
    #[test]
    fn bounded_text_preview_preserves_exact_utf8_and_crlf() {
        let td = TempDir::new("text-preview-exact");
        let path = td.root.join("sample.txt");
        let content = "alpha\r\ncafé\n";
        fs::write(&path, content).expect("write UTF-8 preview fixture");

        let preview = read_text_preview(&path, TextPreviewLimits::new(content.len()))
            .expect("bounded UTF-8 preview");

        assert_eq!(preview.content, content);
        assert!(!preview.truncated);
    }

    // TP-B1.1-BOUNDED-READ: a file exactly at the byte budget is complete;
    // one additional byte flips the explicit truncation marker.
    #[test]
    fn bounded_text_preview_distinguishes_exact_limit_from_overflow() {
        let td = TempDir::new("text-preview-boundary");
        let path = td.root.join("boundary.txt");

        fs::write(&path, "1234").expect("write exact-limit fixture");
        let exact =
            read_text_preview(&path, TextPreviewLimits::new(4)).expect("read exact-limit preview");
        assert_eq!(exact.content, "1234");
        assert!(!exact.truncated);

        fs::write(&path, "12345").expect("write over-limit fixture");
        let overflow =
            read_text_preview(&path, TextPreviewLimits::new(4)).expect("read over-limit preview");
        assert_eq!(overflow.content, "1234");
        assert!(overflow.truncated);
    }

    // TP-B1.1-BOUNDED-READ: a byte limit may land inside a multi-byte scalar.
    // The bounded preview retreats to a valid boundary instead of emitting
    // lossy or invalid UTF-8.
    #[test]
    fn bounded_text_preview_never_splits_utf8_scalar() {
        let td = TempDir::new("text-preview-utf8-boundary");
        let path = td.root.join("unicode.txt");
        fs::write(&path, "abcé-tail").expect("write multi-byte fixture");

        let preview = read_text_preview(&path, TextPreviewLimits::new(4))
            .expect("read UTF-8-boundary preview");

        assert_eq!(preview.content, "abc");
        assert!(preview.truncated);
        assert!(preview.content.len() <= 4);
    }

    // TP-B1.1-BOUNDED-READ: newline-free input is still bounded by bytes; the
    // implementation must not scan or allocate through the rest of the file.
    #[test]
    fn bounded_text_preview_caps_one_long_line() {
        let td = TempDir::new("text-preview-long-line");
        let path = td.root.join("long.txt");
        fs::write(&path, "x".repeat(64)).expect("write long-line fixture");

        let preview = read_text_preview(&path, TextPreviewLimits::new(8))
            .expect("read bounded long-line preview");

        assert_eq!(preview.content, "xxxxxxxx");
        assert!(preview.truncated);
    }

    // TP-B1.2-FAILURES: selection can disappear between directory refresh and
    // preview preparation; the race is explicit and panic-free.
    #[test]
    fn bounded_text_preview_reports_missing_path() {
        let td = TempDir::new("text-preview-missing");
        let error = read_text_preview(
            &td.root.join("removed-before-preview.txt"),
            TextPreviewLimits::default(),
        )
        .expect_err("missing file must not produce text");

        assert_eq!(error, TextPreviewError::Io(std::io::ErrorKind::NotFound));
    }

    // TP-B1.2-FAILURES: directories and other non-regular inputs are not read
    // as text, regardless of the host-specific error returned by `read`.
    #[test]
    fn bounded_text_preview_rejects_directory_as_non_regular() {
        let td = TempDir::new("text-preview-directory");
        let error = read_text_preview(&td.root, TextPreviewLimits::default())
            .expect_err("directory must not produce text");

        assert_eq!(error, TextPreviewError::NotRegularFile);
    }

    // TP-B1.2-FAILURES: a NUL in the bounded sample classifies the selected
    // file as binary instead of sending control bytes into the text renderer.
    #[test]
    fn bounded_text_preview_rejects_binary_nul() {
        let td = TempDir::new("text-preview-binary");
        let path = td.root.join("binary.dat");
        fs::write(&path, b"plain\0binary").expect("write binary fixture");

        let error = read_text_preview(&path, TextPreviewLimits::default())
            .expect_err("binary file must not produce text");

        assert_eq!(error, TextPreviewError::Binary);
    }

    // TP-B1.2-FAILURES: invalid encoding remains an explicit error; no lossy
    // replacement characters are invented.
    #[test]
    fn bounded_text_preview_rejects_invalid_utf8() {
        let td = TempDir::new("text-preview-invalid-utf8");
        let path = td.root.join("invalid.txt");
        fs::write(&path, [b'a', 0xff, b'b']).expect("write invalid UTF-8 fixture");

        let error = read_text_preview(&path, TextPreviewLimits::default())
            .expect_err("invalid UTF-8 must not produce text");

        assert_eq!(error, TextPreviewError::InvalidUtf8 { valid_up_to: 1 });
    }

    // TP-B1.2-FAILURES: permission errors are stable domain data. Unix mode
    // bits give this test a deterministic local fixture; Windows coverage is
    // provided by the shared I/O error mapping and MSVC compile gate.
    #[cfg(unix)]
    #[test]
    fn bounded_text_preview_reports_permission_denied() {
        use std::os::unix::fs::PermissionsExt;

        let td = TempDir::new("text-preview-permission");
        let path = td.root.join("denied.txt");
        fs::write(&path, "secret").expect("write permission fixture");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o000))
            .expect("remove read permission");

        let error = read_text_preview(&path, TextPreviewLimits::default())
            .expect_err("unreadable file must not produce text");

        assert_eq!(
            error,
            TextPreviewError::Io(std::io::ErrorKind::PermissionDenied)
        );
    }

    fn highlighted_line_text(preview: &HighlightedTextPreview, line: usize) -> String {
        preview.lines[line]
            .spans
            .iter()
            .map(|span| span.content.as_str())
            .collect()
    }

    // TP-B1.3-CLASSIFY: a known extension selects a bundled syntax. Styling
    // may evolve with the bundled theme, but text preservation and the
    // presence of semantic styling are stable contracts.
    #[test]
    fn text_highlight_selects_rust_and_preserves_content() {
        let source = "pub fn main() { println!(\"hi\"); }\n";
        let preview = TextPreview {
            source_path: PathBuf::from("main.rs"),
            content: source.to_owned(),
            truncated: false,
            highlighted: None,
        };

        let highlighted = highlight_text_preview(Path::new("main.rs"), &preview);

        assert_eq!(highlighted.syntax_name.as_deref(), Some("Rust"));
        assert_eq!(highlighted_line_text(&highlighted, 0), source.trim_end());
        assert!(highlighted.lines[0]
            .spans
            .iter()
            .any(|span| !span.style.is_plain()));
    }

    // TP-B1.3-CLASSIFY: extension-less scripts use first-line evidence.
    #[test]
    fn text_highlight_uses_python_shebang() {
        let preview = TextPreview {
            source_path: PathBuf::from("script"),
            content: "#!/usr/bin/env python3\nprint('hi')\n".to_owned(),
            truncated: false,
            highlighted: None,
        };

        let highlighted = highlight_text_preview(Path::new("script"), &preview);

        assert_eq!(highlighted.syntax_name.as_deref(), Some("Python"));
        assert_eq!(highlighted_line_text(&highlighted, 1), "print('hi')");
    }

    // TP-B1.3-CLASSIFY: unsupported types remain readable plain text. A
    // classifier miss must never become a preview availability failure.
    #[test]
    fn text_highlight_unknown_extension_falls_back_to_plain_text() {
        let preview = TextPreview {
            source_path: PathBuf::from("sample.mystery"),
            content: "unknown but readable\n".to_owned(),
            truncated: false,
            highlighted: None,
        };

        let highlighted = highlight_text_preview(Path::new("sample.mystery"), &preview);

        assert!(highlighted.syntax_name.is_none());
        assert_eq!(
            highlighted_line_text(&highlighted, 0),
            "unknown but readable"
        );
        assert!(highlighted.lines[0]
            .spans
            .iter()
            .all(|span| span.style.is_plain()));
    }

    // TP-B1.3-CLASSIFY: highlighting cost is bounded independently from the
    // reader byte cap. Remaining text stays available in the underlying model.
    #[test]
    fn text_highlight_caps_prepared_line_count() {
        let preview = TextPreview {
            source_path: PathBuf::from("many.rs"),
            content: "let value = 1;\n".repeat(130),
            truncated: false,
            highlighted: None,
        };

        let highlighted = highlight_text_preview(Path::new("many.rs"), &preview);

        assert_eq!(highlighted.lines.len(), 128);
        assert!(highlighted.truncated_lines);
    }

    #[test]
    fn image_preview_generation_rebinds_on_reload_and_navigation() {
        let td = TempDir::new("image-generation");
        fs::write(td.root.join("alpha.PNG"), b"first generation")
            .expect("write first image candidate");
        fs::write(td.root.join("beta.webp"), b"second generation")
            .expect("write second image candidate");
        set_equal_modified(&td, &["alpha.PNG", "beta.webp"]);

        let mut state = FmState::new(&td.root);
        let first_generation = match &state.preview {
            FmPreview::File(FmFilePreview::Image(preview)) => {
                assert!(preview.source_path.ends_with("alpha.PNG"));
                assert_eq!(preview.state, FmImagePreviewState::Pending);
                preview.generation
            }
            other => panic!("expected first image candidate, got {other:?}"),
        };

        state.reload();
        let reloaded_generation = match &state.preview {
            FmPreview::File(FmFilePreview::Image(preview)) => preview.generation,
            other => panic!("expected reloaded image candidate, got {other:?}"),
        };
        assert_ne!(reloaded_generation, first_generation);

        state.move_down();
        match &state.preview {
            FmPreview::File(FmFilePreview::Image(preview)) => {
                assert!(preview.source_path.ends_with("beta.webp"));
                assert_eq!(preview.state, FmImagePreviewState::Pending);
                assert_ne!(preview.generation, reloaded_generation);
            }
            other => panic!("expected navigated image candidate, got {other:?}"),
        }
    }

    // TP-FIP-FOCUS-01: entering a child at a nonzero index must bind that
    // exact child path to the departing segment, never leave it unset for a
    // row-zero fallback.
    #[test]
    fn entering_nonzero_child_binds_exact_focused_child_in_departing_segment() {
        let td = TempDir::new("focus-bind");
        for name in ["alpha", "beta", "gamma"] {
            fs::create_dir_all(td.root.join(name)).expect("fixture dir");
        }
        set_equal_modified(&td, &["alpha", "beta", "gamma"]);
        let mut state = FmState::new(&td.root);
        let beta = td.root.join("beta");
        let beta_index = state
            .entries
            .iter()
            .position(|entry| entry.path == beta)
            .expect("beta row");
        assert!(beta_index > 0, "test requires a nonzero child index");
        assert!(state.select(beta_index));

        state.enter();

        assert_eq!(state.cwd, beta);
        let column = state
            .trail
            .cols()
            .iter()
            .find(|column| column.directory == td.root)
            .expect("departing column stays in Trail");
        assert_eq!(column.selected.as_deref(), Some(beta.as_path()));
    }

    // Deep Trail ancestors retain exact selected-child identity.
    // TP-FIP-FOCUS-02: descending four levels binds every resident ancestor
    // to its exact next path segment, not just the immediate parent.
    #[test]
    fn four_level_descent_binds_every_resident_ancestor_focus() {
        let td = TempDir::new("focus-chain");
        let l1 = td.root.join("b1");
        let l2 = l1.join("b2");
        let l3 = l2.join("b3");
        // A sibling before each target keeps every entered child at a
        // nonzero index.
        for sibling in [td.root.join("a0"), l1.join("a0"), l2.join("a0"), l3.clone()] {
            fs::create_dir_all(sibling).expect("fixture dir");
        }
        let mut state = FmState::new(&td.root);
        for target in [&l1, &l2, &l3] {
            let index = state
                .entries
                .iter()
                .position(|entry| entry.path == **target)
                .expect("target row");
            assert!(index > 0, "each entered child must be nonzero");
            assert!(state.select(index));
            state.enter();
        }
        assert_eq!(state.cwd, l3);
        for (directory, child) in [(&td.root, &l1), (&l1, &l2), (&l2, &l3)] {
            let column = state
                .trail
                .cols()
                .iter()
                .find(|column| column.directory == **directory)
                .expect("ancestor column stays in Trail");
            assert_eq!(
                column.selected.as_deref(),
                Some(child.as_path()),
                "ancestor {directory:?} must bind its exact next Trail segment"
            );
        }
    }

    // TP-FIP-FOCUS-05/06: changing branch through an ancestor retires the
    // descendant focus with its segments, and the re-entered ancestor binds
    // the NEW child.
    #[test]
    fn branch_change_retires_descendant_focus_and_rebinds_ancestor() {
        let td = TempDir::new("focus-branch");
        let alpha = td.root.join("alpha");
        let beta = td.root.join("beta");
        let alpha_child = alpha.join("inner");
        fs::create_dir_all(&alpha_child).expect("fixture dir");
        fs::create_dir_all(&beta).expect("fixture dir");
        let mut state = FmState::new(&td.root);

        let alpha_index = state
            .entries
            .iter()
            .position(|entry| entry.path == alpha)
            .expect("alpha row");
        assert!(state.select(alpha_index));
        state.enter();
        state.leave();
        // Root now binds alpha; switch the branch to beta.
        let beta_index = state
            .entries
            .iter()
            .position(|entry| entry.path == beta)
            .expect("beta row");
        assert!(beta_index > 0, "beta must sit at a nonzero index");
        assert!(state.select(beta_index));
        state.enter();

        assert_eq!(state.cwd, beta);
        let root_column = state
            .trail
            .cols()
            .iter()
            .find(|column| column.directory == td.root)
            .expect("root Trail column");
        assert_eq!(
            root_column.selected.as_deref(),
            Some(beta.as_path()),
            "the re-entered ancestor binds the NEW branch child"
        );
        assert!(
            !state
                .trail
                .cols()
                .iter()
                .any(|column| column.directory == alpha),
            "the retired branch column leaves the Trail"
        );
    }

    // FIP-3.1 / TP-MTIME-01: kind identity is independent from chronological
    // position. Look up exact names instead of coupling this characterization
    // to the retired directory-first order or filesystem creation timing.
    #[test]
    fn snapshot_symlink_classification_is_independent_of_mtime_sort() {
        let td = TempDir::new("icon-baseline");
        fs::create_dir_all(td.root.join("bdir")).expect("dir");
        fs::create_dir_all(td.root.join("adir")).expect("dir");
        fs::write(td.root.join("afile.txt"), b"x").expect("file");
        fs::write(td.root.join("zfile.txt"), b"x").expect("file");
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(td.root.join("adir"), td.root.join("link-dir"))
                .expect("link-dir");
            std::os::unix::fs::symlink(td.root.join("afile.txt"), td.root.join("link-file"))
                .expect("link-file");
            std::os::unix::fs::symlink(td.root.join("missing"), td.root.join("broken"))
                .expect("broken");
        }
        let snapshot = read_directory_snapshot(&td.root, false);
        #[cfg(unix)]
        assert_eq!(snapshot.entries.len(), 7);
        let by_name = |name: &str| {
            snapshot
                .entries
                .iter()
                .find(|entry| entry.name == name)
                .expect("entry present")
        };
        assert!(by_name("adir").is_dir() && by_name("adir").operation_supported());
        assert!(!by_name("afile.txt").is_dir() && by_name("afile.txt").operation_supported());
        #[cfg(unix)]
        {
            assert!(by_name("link-dir").is_dir() && by_name("link-dir").operation_supported());
            assert!(!by_name("link-file").is_dir() && by_name("link-file").operation_supported());
            assert!(
                !by_name("broken").is_dir() && !by_name("broken").operation_supported(),
                "a broken symlink lists as an unsupported file"
            );
        }
    }

    // TP-FIP-ICON-01..05 at the snapshot seam: the prepared canonical kind
    // preserves symlink identity, and the bridge fields stay consistent with
    // the kind-derived capabilities for every listed entry.
    #[test]
    fn snapshot_prepares_canonical_entry_kinds() {
        use super::entry_kind::FileEntryKind;
        let td = TempDir::new("kind-snapshot");
        fs::create_dir_all(td.root.join("dir")).expect("dir");
        fs::write(td.root.join("file.txt"), b"x").expect("file");
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(td.root.join("dir"), td.root.join("link-dir"))
                .expect("link-dir");
            std::os::unix::fs::symlink(td.root.join("file.txt"), td.root.join("link-file"))
                .expect("link-file");
            std::os::unix::fs::symlink(td.root.join("missing"), td.root.join("broken"))
                .expect("broken");
        }
        let snapshot = read_directory_snapshot(&td.root, false);
        let kind_of = |name: &str| {
            snapshot
                .entries
                .iter()
                .find(|entry| entry.name == name)
                .expect("entry present")
                .kind
        };
        assert_eq!(kind_of("dir"), FileEntryKind::Directory);
        assert_eq!(kind_of("file.txt"), FileEntryKind::RegularFile);
        #[cfg(unix)]
        {
            assert_eq!(kind_of("link-dir"), FileEntryKind::SymlinkDirectory);
            assert_eq!(kind_of("link-file"), FileEntryKind::SymlinkFile);
            assert_eq!(kind_of("broken"), FileEntryKind::BrokenSymlink);
        }
        for entry in &snapshot.entries {
            assert_eq!(entry.is_dir(), entry.kind.is_directory_target());
            assert_eq!(
                entry.operation_supported(),
                entry.kind.supports_native_operation()
            );
        }
    }
}
