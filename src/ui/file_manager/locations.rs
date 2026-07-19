#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use ratatui::layout::Rect;

    use super::{
        file_manager_content_layout, project_file_manager_locations_view, FileManagerLocationsMode,
        COMPACT_CONTENT_THRESHOLD, LOCATIONS_ACTION_WIDTH,
    };
    use crate::app::state::{
        AppState, FileManagerSidebarIcon, FileManagerSidebarItem, FileManagerSidebarModel,
    };

    fn item(label: &str, path: &str) -> FileManagerSidebarItem {
        FileManagerSidebarItem {
            label: label.to_string(),
            path: PathBuf::from(path),
            icon: FileManagerSidebarIcon::Pin,
            accessible: true,
            ejectable: false,
        }
    }

    fn prepared_files_app(items: Vec<FileManagerSidebarItem>) -> AppState {
        let mut app = AppState::test_new();
        app.stage.activate_files().expect("Files activation");
        app.file_manager = Some(crate::fm::FmState::new(PathBuf::from("/")));
        app.file_manager_sidebar =
            FileManagerSidebarModel::from_sources(items, Vec::new(), Vec::new());
        app
    }

    fn assert_bounded(rect: Rect, bounds: Rect) {
        assert!(rect.x >= bounds.x);
        assert!(rect.y >= bounds.y);
        assert!(rect.right() <= bounds.right());
        assert!(rect.bottom() <= bounds.bottom());
    }

    // TP-FCL-GEO-01: the content-local rail, separator, and Trail are one
    // bounded projection. Disjointness intentionally uses `is_empty()` because
    // ratatui preserves a nonzero origin for empty intersections.
    #[test]
    fn fcl_geometry_wide_and_standard_regions_are_bounded_and_disjoint() {
        for body in [Rect::new(7, 3, 120, 18), Rect::new(11, 5, 48, 9)] {
            let layout = file_manager_content_layout(body);
            assert_ne!(layout.mode, FileManagerLocationsMode::Compact);
            let rail = layout.rail.expect("persistent rail");
            let separator = layout.separator.expect("persistent separator");

            for rect in [rail, separator, layout.trail] {
                assert_bounded(rect, body);
                assert!(!rect.is_empty());
            }
            assert!(rail.intersection(separator).is_empty());
            assert!(rail.intersection(layout.trail).is_empty());
            assert!(separator.intersection(layout.trail).is_empty());
            assert!(layout.trail.width >= crate::fm::miller::MILLER_COLUMN_MIN_WIDTH);
        }
    }

    // TP-FCL-GEO-02: the responsive boundary is content-derived and stable at
    // the exact cell on either side of the transition.
    #[test]
    fn fcl_geometry_compact_boundary_is_exact_and_deterministic() {
        let below = file_manager_content_layout(Rect::new(0, 0, COMPACT_CONTENT_THRESHOLD - 1, 8));
        let exact = file_manager_content_layout(Rect::new(0, 0, COMPACT_CONTENT_THRESHOLD, 8));
        let above = file_manager_content_layout(Rect::new(0, 0, COMPACT_CONTENT_THRESHOLD + 1, 8));

        assert_eq!(below.mode, FileManagerLocationsMode::Compact);
        assert_eq!(below.rail, None);
        assert_eq!(below.separator, None);
        assert_eq!(below.trail.width, COMPACT_CONTENT_THRESHOLD - 1);
        assert_eq!(exact.mode, FileManagerLocationsMode::Standard);
        assert_eq!(above.mode, FileManagerLocationsMode::Standard);
        assert!(exact.rail.is_some());
        assert!(above.rail.is_some());
    }

    // TP-FCL-GEO-03: degenerate frames and display-wide labels remain
    // panic-free. Published rows and compact actions are always complete.
    #[test]
    fn fcl_geometry_tiny_frames_and_unicode_rows_publish_only_complete_targets() {
        let app = prepared_files_app(vec![
            item("提交 herdr 的反馈", "/wide-one"),
            item("İndirilenler", "/wide-two"),
            item("🗂️ Arşiv", "/wide-three"),
        ]);

        for area in [
            Rect::default(),
            Rect::new(5, 7, 1, 1),
            Rect::new(5, 7, 21, 2),
        ] {
            let result =
                std::panic::catch_unwind(|| project_file_manager_locations_view(&app, area));
            let snapshot = result.expect("degenerate geometry must not panic");
            assert!(snapshot.rows.is_empty());
            assert!(snapshot.locations_action_area.is_none());
        }

        let complete_action = project_file_manager_locations_view(&app, Rect::new(4, 2, 22, 5))
            .locations_action_area
            .expect("complete compact action");
        assert_eq!(complete_action.width, LOCATIONS_ACTION_WIDTH);

        let snapshot = project_file_manager_locations_view(
            &app,
            Rect::new(4, 2, COMPACT_CONTENT_THRESHOLD, 5),
        );
        assert_eq!(snapshot.rows.len(), 2);
        assert_eq!(snapshot.rows[0].path, PathBuf::from("/wide-one"));
        assert_eq!(snapshot.rows[1].path, PathBuf::from("/wide-two"));
        assert!(snapshot.rows.iter().all(|row| row.rect.height == 1));
        assert!(snapshot.rows.iter().all(|row| !row.rect.is_empty()));
        assert!(snapshot.locations_action_area.is_none());
    }
}
