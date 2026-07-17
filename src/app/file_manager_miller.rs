//! Bounded Miller input authority.
//!
//! Raw terminal coordinates are resolved against the immutable Files frame
//! snapshot, then exact generation/path/index identities are revalidated
//! against `FmState` before any input adapter may mutate state.

#[cfg(test)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct ResolvedMillerRow {
    column_kind: crate::ui::MillerRowColumnKind,
    directory_path: std::path::PathBuf,
    entry_index: usize,
    entry_path: std::path::PathBuf,
}

#[cfg(test)]
fn resolve_live_miller_row(
    _snapshot: &crate::ui::MillerViewSnapshot,
    _file_manager: &crate::fm::FmState,
    _active_files_generation: u32,
    _column: u16,
    _row: u16,
) -> Option<ResolvedMillerRow> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fm::miller::{MillerColumnId, MillerDirectoryProjection, MillerPathSegment};
    use crate::fm::{FileEntry, FmDirectoryStatus, FmParent, FmPreview, FmState};
    use crate::ui::{project_miller_view, MillerRowColumnKind, MillerViewSnapshot};
    use ratatui::layout::Rect;
    use std::path::PathBuf;

    fn entry(path: impl Into<PathBuf>, is_dir: bool) -> FileEntry {
        let path = path.into();
        FileEntry {
            name: path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
            path,
            is_dir,
            operation_supported: true,
        }
    }

    fn prepared_snapshot() -> (FmState, MillerViewSnapshot) {
        let resident = PathBuf::from("/virtual/resident");
        let parent = PathBuf::from("/virtual/parent");
        let current = parent.join("current");
        let mut file_manager = FmState::test_empty(current.clone());
        file_manager.miller.chain = [resident.clone(), parent.clone(), current.clone()]
            .into_iter()
            .map(MillerPathSegment::new)
            .collect();
        file_manager.miller.focused_directory = current.clone();
        file_manager.entries = vec![entry(current.join("current.txt"), false)];
        file_manager.parent = Some(FmParent {
            entries: vec![entry(parent.join("peer"), true)],
            cursor: Some(0),
        });
        file_manager.preview = FmPreview::Directory(vec![entry(
            current.join("current.txt").join("preview-child"),
            true,
        )]);
        file_manager.miller.visit(
            current,
            Some(MillerDirectoryProjection {
                id: MillerColumnId {
                    directory: resident.clone(),
                    generation: 41,
                },
                entries: vec![entry(resident.join("cached-child"), true)],
                status: FmDirectoryStatus::Available,
                writable: true,
            }),
        );
        let snapshot = project_miller_view(Rect::new(0, 0, 144, 8), &file_manager, 7);
        (file_manager, snapshot)
    }

    // TP-FM3-LIVE-ROWS: every rendered row resolves through its exact typed
    // column authority. No caller may infer ownership from the row index or
    // screen rectangle alone.
    #[test]
    fn plain_hit_resolves_exact_live_row_in_each_visible_column() {
        let (file_manager, snapshot) = prepared_snapshot();
        let expected_kinds = [
            MillerRowColumnKind::ResidentDirectory,
            MillerRowColumnKind::PreparedParent,
            MillerRowColumnKind::Current,
            MillerRowColumnKind::Preview,
        ];
        let actionable_columns = snapshot
            .columns
            .iter()
            .filter(|column| !column.rows.is_empty())
            .collect::<Vec<_>>();
        assert_eq!(actionable_columns.len(), expected_kinds.len());

        for (column, expected_kind) in actionable_columns.into_iter().zip(expected_kinds) {
            let target = &column.rows[0];
            assert_eq!(
                resolve_live_miller_row(&snapshot, &file_manager, 7, target.rect.x, target.rect.y,),
                Some(ResolvedMillerRow {
                    column_kind: expected_kind,
                    directory_path: target.directory_path.as_ref().clone(),
                    entry_index: target.entry_index,
                    entry_path: target.entry_path.clone(),
                })
            );
        }
    }
}
