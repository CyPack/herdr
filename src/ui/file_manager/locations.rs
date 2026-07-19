use std::path::PathBuf;

use ratatui::layout::Rect;

use crate::app::state::AppState;
use crate::fm::miller::MILLER_COLUMN_MIN_WIDTH;
use crate::ui::surface_host::StageSurfaceView;

pub(crate) const WIDE_RAIL_TARGET: u16 = 24;
pub(crate) const WIDE_RAIL_MIN: u16 = 18;
pub(crate) const WIDE_RAIL_MAX: u16 = 28;
pub(crate) const STANDARD_RAIL_MIN: u16 = 16;
pub(crate) const STANDARD_RAIL_MAX: u16 = 20;
pub(crate) const LOCATIONS_SEPARATOR_WIDTH: u16 = 1;
pub(crate) const LOCATIONS_ACTION_WIDTH: u16 = 9;
const LOCATIONS_ACTION_GAP: u16 = 1;
const HEADER_IDENTITY_MIN_WIDTH: u16 = 12;
pub(crate) const COMPACT_CONTENT_THRESHOLD: u16 =
    STANDARD_RAIL_MIN + LOCATIONS_SEPARATOR_WIDTH + MILLER_COLUMN_MIN_WIDTH;
const WIDE_CONTENT_THRESHOLD: u16 =
    WIDE_RAIL_TARGET + LOCATIONS_SEPARATOR_WIDTH + MILLER_COLUMN_MIN_WIDTH * 2;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum FileManagerLocationsMode {
    Wide,
    Standard,
    #[default]
    Compact,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct FileManagerContentLayout {
    pub(crate) mode: FileManagerLocationsMode,
    pub(crate) rail: Option<Rect>,
    pub(crate) separator: Option<Rect>,
    pub(crate) trail: Rect,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FileManagerLocationRowArea {
    pub(crate) rect: Rect,
    pub(crate) path: PathBuf,
    pub(crate) files_generation: u32,
    pub(crate) model_revision: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct FileManagerLocationsView {
    pub(crate) files_generation: Option<u32>,
    pub(crate) model_revision: u64,
    pub(crate) layout: FileManagerContentLayout,
    pub(crate) rows: Vec<FileManagerLocationRowArea>,
    pub(crate) locations_action_area: Option<Rect>,
    pub(crate) drawer_area: Option<Rect>,
}

/// Split the exact Native Files body into one locations rail, one inert
/// separator, and the remaining Trail viewport. A complete useful Miller
/// column always wins over decorative rail width.
pub(crate) fn file_manager_content_layout(body: Rect) -> FileManagerContentLayout {
    if body.width < COMPACT_CONTENT_THRESHOLD || body.height == 0 {
        return FileManagerContentLayout {
            mode: FileManagerLocationsMode::Compact,
            rail: None,
            separator: None,
            trail: body,
        };
    }

    let mode = if body.width >= WIDE_CONTENT_THRESHOLD {
        FileManagerLocationsMode::Wide
    } else {
        FileManagerLocationsMode::Standard
    };
    let rail_limits = if mode == FileManagerLocationsMode::Wide {
        (WIDE_RAIL_MIN, WIDE_RAIL_MAX, WIDE_RAIL_TARGET)
    } else {
        (STANDARD_RAIL_MIN, STANDARD_RAIL_MAX, STANDARD_RAIL_MAX)
    };
    let maximum_rail_width = body
        .width
        .saturating_sub(LOCATIONS_SEPARATOR_WIDTH)
        .saturating_sub(MILLER_COLUMN_MIN_WIDTH);
    let rail_width = rail_limits
        .2
        .min(maximum_rail_width)
        .clamp(rail_limits.0, rail_limits.1);
    let rail = Rect::new(body.x, body.y, rail_width, body.height);
    let separator = Rect::new(rail.right(), body.y, LOCATIONS_SEPARATOR_WIDTH, body.height);
    let trail = Rect::new(
        separator.right(),
        body.y,
        body.right().saturating_sub(separator.right()),
        body.height,
    );

    FileManagerContentLayout {
        mode,
        rail: Some(rail),
        separator: Some(separator),
        trail,
    }
}

fn project_location_rows(
    app: &AppState,
    rail: Rect,
    files_generation: u32,
    model_revision: u64,
) -> Vec<FileManagerLocationRowArea> {
    if rail.width == 0 || rail.height == 0 {
        return Vec::new();
    }

    let mut rows = Vec::new();
    let mut line_index = 0_u16;
    for (section_index, section) in app.file_manager_sidebar.sections.iter().enumerate() {
        if section_index > 0 {
            line_index = line_index.saturating_add(1);
        }
        line_index = line_index.saturating_add(1);

        for item in &section.items {
            if line_index >= rail.height {
                return rows;
            }
            rows.push(FileManagerLocationRowArea {
                rect: Rect::new(rail.x, rail.y.saturating_add(line_index), rail.width, 1),
                path: item.path.clone(),
                files_generation,
                model_revision,
            });
            line_index = line_index.saturating_add(1);
        }
    }
    rows
}

fn locations_action_area(header: Rect, mode: FileManagerLocationsMode) -> Option<Rect> {
    if mode != FileManagerLocationsMode::Compact || header.height == 0 {
        return None;
    }
    let required_width = HEADER_IDENTITY_MIN_WIDTH
        .saturating_add(LOCATIONS_ACTION_GAP)
        .saturating_add(LOCATIONS_ACTION_WIDTH);
    (header.width >= required_width).then(|| {
        Rect::new(
            header.right().saturating_sub(LOCATIONS_ACTION_WIDTH),
            header.y,
            LOCATIONS_ACTION_WIDTH,
            1,
        )
    })
}

/// Publish every Files-local navigation target from one current-frame
/// projection. Hidden/closed Files surfaces return the default snapshot so
/// prior-frame identities cannot remain actionable.
pub(crate) fn project_file_manager_locations_view(
    app: &AppState,
    area: Rect,
) -> FileManagerLocationsView {
    if app.stage.surface_view() != StageSurfaceView::NativeFiles || app.file_manager.is_none() {
        return FileManagerLocationsView::default();
    }
    let Some(files_generation) = app.stage.active_instance_generation() else {
        return FileManagerLocationsView::default();
    };
    let Some([header, body, _status]) = super::file_manager_frame_areas(area) else {
        return FileManagerLocationsView::default();
    };

    let layout = file_manager_content_layout(body);
    let model_revision = app.file_manager_sidebar.revision();
    let rows = layout
        .rail
        .map(|rail| project_location_rows(app, rail, files_generation, model_revision))
        .unwrap_or_default();

    FileManagerLocationsView {
        files_generation: Some(files_generation),
        model_revision,
        layout,
        rows,
        locations_action_area: locations_action_area(header, layout.mode),
        drawer_area: None,
    }
}

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
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                project_file_manager_locations_view(&app, area)
            }));
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
