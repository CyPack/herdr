//! Bounded Miller input authority.
//!
//! Raw terminal coordinates are resolved against the immutable Files frame
//! snapshot, then exact generation/path/index identities are revalidated
//! against `FmState` before any input adapter may mutate state.
#![allow(dead_code)] // FM3 lands and verifies the resolver before the next
                     // atomic commit cuts mouse routing over to this module.

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ResolvedMillerRow {
    column_kind: crate::ui::MillerRowColumnKind,
    directory_path: std::path::PathBuf,
    entry_index: usize,
    entry_path: std::path::PathBuf,
}

pub(super) fn resolve_live_miller_row(
    snapshot: &crate::ui::MillerViewSnapshot,
    file_manager: &crate::fm::FmState,
    active_files_generation: u32,
    column: u16,
    row: u16,
) -> Option<ResolvedMillerRow> {
    if snapshot.files_generation != Some(active_files_generation)
        || snapshot.model_revision != file_manager.miller.revision
    {
        return None;
    }

    let mut hits = snapshot.columns.iter().flat_map(|column_view| {
        column_view
            .rows
            .iter()
            .filter(move |target| rect_contains(target.rect, column, row))
            .map(move |target| (column_view, target))
    });
    let (column_view, target) = hits.next()?;
    if hits.next().is_some()
        || target.files_generation != active_files_generation
        || target.model_revision != snapshot.model_revision
        || target.projection_index != column_view.projection_index
    {
        return None;
    }

    let entry_is_live = match target.column_kind {
        crate::ui::MillerRowColumnKind::Current => {
            let chain_index = target.chain_index?;
            file_manager.cwd == *target.directory_path
                && file_manager.preview_generation == target.source_generation
                && file_manager
                    .miller
                    .chain
                    .get(chain_index)
                    .is_some_and(|segment| segment.directory == *target.directory_path)
                && exact_entry(
                    &file_manager.entries,
                    target.entry_index,
                    &target.entry_path,
                )
        }
        crate::ui::MillerRowColumnKind::ResidentDirectory => {
            let chain_index = target.chain_index?;
            file_manager
                .miller
                .chain
                .get(chain_index)
                .is_some_and(|segment| segment.directory == *target.directory_path)
                && file_manager
                    .miller
                    .resident_non_current
                    .iter()
                    .find(|projection| {
                        projection.id.directory == *target.directory_path
                            && projection.id.generation == target.source_generation
                    })
                    .is_some_and(|projection| {
                        exact_entry(&projection.entries, target.entry_index, &target.entry_path)
                    })
        }
        crate::ui::MillerRowColumnKind::PreparedParent => {
            let chain_index = target.chain_index?;
            file_manager.preview_generation == target.source_generation
                && file_manager.cwd.parent() == Some(target.directory_path.as_path())
                && file_manager
                    .miller
                    .chain
                    .get(chain_index)
                    .is_some_and(|segment| segment.directory == *target.directory_path)
                && file_manager.parent.as_ref().is_some_and(|parent| {
                    exact_entry(&parent.entries, target.entry_index, &target.entry_path)
                })
        }
        crate::ui::MillerRowColumnKind::Preview => {
            target.chain_index.is_none()
                && file_manager.preview_generation == target.source_generation
                && file_manager.selected().map(|entry| &entry.path)
                    == Some(target.directory_path.as_ref())
                && match &file_manager.preview {
                    crate::fm::FmPreview::Directory(entries) => {
                        exact_entry(entries, target.entry_index, &target.entry_path)
                    }
                    crate::fm::FmPreview::None | crate::fm::FmPreview::File(_) => false,
                }
        }
    };
    entry_is_live.then(|| ResolvedMillerRow {
        column_kind: target.column_kind,
        directory_path: target.directory_path.as_ref().clone(),
        entry_index: target.entry_index,
        entry_path: target.entry_path.clone(),
    })
}

fn rect_contains(rect: ratatui::layout::Rect, column: u16, row: u16) -> bool {
    column >= rect.x
        && column < rect.x.saturating_add(rect.width)
        && row >= rect.y
        && row < rect.y.saturating_add(rect.height)
}

fn exact_entry(
    entries: &[crate::fm::FileEntry],
    entry_index: usize,
    entry_path: &std::path::Path,
) -> bool {
    entries
        .get(entry_index)
        .is_some_and(|entry| entry.path == entry_path)
        && entries
            .iter()
            .filter(|entry| entry.path == entry_path)
            .take(2)
            .count()
            == 1
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

    // TP-FM3-STALE-ROWS: exact path, source generation, resident identity,
    // Files generation, and unambiguous geometry all remain mandatory. A stale
    // click is consumed by the eventual router but never replayed against the
    // replacement state.
    #[test]
    fn stale_reordered_deleted_evicted_and_reopened_targets_are_inert() {
        let (file_manager, snapshot) = prepared_snapshot();
        let row_for = |kind| {
            snapshot
                .columns
                .iter()
                .flat_map(|column| &column.rows)
                .find(|row| row.column_kind == kind)
                .cloned()
                .expect("typed row fixture")
        };
        let resident = row_for(MillerRowColumnKind::ResidentDirectory);
        let parent = row_for(MillerRowColumnKind::PreparedParent);
        let current = row_for(MillerRowColumnKind::Current);
        let preview = row_for(MillerRowColumnKind::Preview);
        let resolve = |state: &FmState, target: &crate::ui::MillerRowColumnKind, x, y| {
            let resolved = resolve_live_miller_row(&snapshot, state, 7, x, y);
            assert!(
                resolved
                    .as_ref()
                    .is_none_or(|row| &row.column_kind == target),
                "a hit must never resolve as another column kind"
            );
            resolved
        };

        let mut replaced_same_index = file_manager.clone();
        replaced_same_index.entries[0] = entry(
            replaced_same_index
                .cwd
                .join("replacement-at-index-zero.txt"),
            false,
        );
        assert!(
            resolve(
                &replaced_same_index,
                &MillerRowColumnKind::Current,
                current.rect.x,
                current.rect.y,
            )
            .is_none(),
            "row index alone never authorizes a replacement path"
        );

        let mut duplicate_path = file_manager.clone();
        duplicate_path
            .entries
            .push(duplicate_path.entries[0].clone());
        assert!(
            resolve(
                &duplicate_path,
                &MillerRowColumnKind::Current,
                current.rect.x,
                current.rect.y,
            )
            .is_none(),
            "ambiguous duplicate path identity is rejected"
        );

        let mut deleted_parent = file_manager.clone();
        deleted_parent
            .parent
            .as_mut()
            .expect("prepared parent")
            .entries
            .clear();
        assert!(
            resolve(
                &deleted_parent,
                &MillerRowColumnKind::PreparedParent,
                parent.rect.x,
                parent.rect.y,
            )
            .is_none(),
            "deleted prepared-parent row is inert"
        );

        let mut renamed_preview = file_manager.clone();
        if let FmPreview::Directory(entries) = &mut renamed_preview.preview {
            entries[0].path.set_file_name("renamed-preview-child");
        }
        assert!(
            resolve(
                &renamed_preview,
                &MillerRowColumnKind::Preview,
                preview.rect.x,
                preview.rect.y,
            )
            .is_none(),
            "renamed preview row is not replayed by old path"
        );

        let mut evicted_resident = file_manager.clone();
        evicted_resident.miller.resident_non_current.clear();
        assert!(
            resolve(
                &evicted_resident,
                &MillerRowColumnKind::ResidentDirectory,
                resident.rect.x,
                resident.rect.y,
            )
            .is_none(),
            "evicted resident generation cannot resolve"
        );

        let mut refreshed_current = file_manager.clone();
        refreshed_current.preview_generation =
            refreshed_current.preview_generation.saturating_add(1);
        assert!(
            resolve(
                &refreshed_current,
                &MillerRowColumnKind::Current,
                current.rect.x,
                current.rect.y,
            )
            .is_none(),
            "a refreshed current generation retires prior row targets"
        );

        assert!(
            resolve_live_miller_row(&snapshot, &file_manager, 8, current.rect.x, current.rect.y)
                .is_none(),
            "close/reopen Files generation prevents same-path ABA"
        );

        let mut overlapping = snapshot.clone();
        let resident_rect = resident.rect;
        overlapping
            .columns
            .iter_mut()
            .flat_map(|column| &mut column.rows)
            .find(|row| row.column_kind == MillerRowColumnKind::PreparedParent)
            .expect("second row")
            .rect = resident_rect;
        assert!(
            resolve_live_miller_row(
                &overlapping,
                &file_manager,
                7,
                resident_rect.x,
                resident_rect.y,
            )
            .is_none(),
            "ambiguous overlapping row geometry fails closed"
        );
    }
}
