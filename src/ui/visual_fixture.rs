//! Test-only Ratatui cell-fixture exporter for the Playwright Chromium visual
//! oracle. Serializes an exact `TestBackend` buffer (symbol, fg, bg,
//! modifiers, position per cell) so the browser harness renders the real
//! prepared projection instead of reinventing product layout.

use ratatui::buffer::Buffer;
use ratatui::style::{Color, Modifier};

#[derive(Debug, serde::Serialize)]
pub(crate) struct FixtureCell {
    pub symbol: String,
    pub fg: String,
    pub bg: String,
    pub modifiers: Vec<String>,
    pub x: u16,
    pub y: u16,
}

#[derive(Debug, serde::Serialize)]
pub(crate) struct CellFixture {
    pub name: String,
    pub width: u16,
    pub height: u16,
    pub cells: Vec<FixtureCell>,
}

pub(crate) fn export_cell_fixture(name: &str, buffer: &Buffer) -> CellFixture {
    let area = *buffer.area();
    let mut cells = Vec::with_capacity(usize::from(area.width) * usize::from(area.height));
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            let cell = &buffer[(x, y)];
            cells.push(FixtureCell {
                symbol: cell.symbol().to_string(),
                fg: format_color(cell.fg),
                bg: format_color(cell.bg),
                modifiers: format_modifiers(cell.modifier),
                x,
                y,
            });
        }
    }
    CellFixture {
        name: name.to_string(),
        width: area.width,
        height: area.height,
        cells,
    }
}

fn format_color(color: Color) -> String {
    match color {
        Color::Reset => "reset".to_string(),
        Color::Black => "black".to_string(),
        Color::Red => "red".to_string(),
        Color::Green => "green".to_string(),
        Color::Yellow => "yellow".to_string(),
        Color::Blue => "blue".to_string(),
        Color::Magenta => "magenta".to_string(),
        Color::Cyan => "cyan".to_string(),
        Color::Gray => "gray".to_string(),
        Color::DarkGray => "darkgray".to_string(),
        Color::LightRed => "lightred".to_string(),
        Color::LightGreen => "lightgreen".to_string(),
        Color::LightYellow => "lightyellow".to_string(),
        Color::LightBlue => "lightblue".to_string(),
        Color::LightMagenta => "lightmagenta".to_string(),
        Color::LightCyan => "lightcyan".to_string(),
        Color::White => "white".to_string(),
        Color::Rgb(r, g, b) => format!("rgb({r},{g},{b})"),
        Color::Indexed(index) => format!("indexed({index})"),
    }
}

fn format_modifiers(modifier: Modifier) -> Vec<String> {
    const NAMED: [(Modifier, &str); 9] = [
        (Modifier::BOLD, "bold"),
        (Modifier::DIM, "dim"),
        (Modifier::ITALIC, "italic"),
        (Modifier::UNDERLINED, "underlined"),
        (Modifier::SLOW_BLINK, "slow_blink"),
        (Modifier::RAPID_BLINK, "rapid_blink"),
        (Modifier::REVERSED, "reversed"),
        (Modifier::HIDDEN, "hidden"),
        (Modifier::CROSSED_OUT, "crossed_out"),
    ];
    NAMED
        .iter()
        .filter(|(flag, _)| modifier.contains(*flag))
        .map(|(_, label)| (*label).to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::layout::Rect;
    use ratatui::style::Style;
    use ratatui::widgets::Paragraph;
    use ratatui::Terminal;

    fn write_fixture(dir: &std::path::Path, fixture: CellFixture) {
        let path = dir.join(format!("{}.json", fixture.name));
        let json = serde_json::to_string(&fixture).expect("serialize fixture");
        std::fs::write(path, json).expect("write fixture");
    }

    fn mtime_fixture_system_time(
        year: i32,
        month: time::Month,
        day: u8,
        hour: u8,
        minute: u8,
    ) -> std::time::SystemTime {
        let date =
            time::Date::from_calendar_date(year, month, day).expect("valid mtime fixture date");
        let time = time::Time::from_hms(hour, minute, 0).expect("valid mtime fixture time");
        time::PrimitiveDateTime::new(date, time)
            .assume_offset(time::UtcOffset::UTC)
            .into()
    }

    fn set_mtime_fixture_modified(path: &std::path::Path, modified: std::time::SystemTime) {
        let file = std::fs::File::open(path).expect("open mtime visual fixture");
        file.set_times(std::fs::FileTimes::new().set_modified(modified))
            .expect("set mtime visual fixture time");
    }

    fn mtime_fixture_anchor() -> crate::fm::entry_time::LocalCalendarAnchor {
        crate::fm::entry_time::LocalCalendarAnchor::from_system_time_at_offset(
            mtime_fixture_system_time(2026, time::Month::January, 10, 12, 0),
            time::UtcOffset::UTC,
        )
    }

    fn fcl_location_item(
        label: &str,
        path: std::path::PathBuf,
        icon: crate::app::state::FileManagerLocationIcon,
    ) -> crate::app::state::FileManagerLocationItem {
        crate::app::state::FileManagerLocationItem {
            label: label.to_string(),
            path,
            icon,
            accessible: true,
            ejectable: false,
        }
    }

    fn fcl_visual_app(
        trail_root: &std::path::Path,
        target: &std::path::Path,
        explicit_origin: &std::path::Path,
        locations_model: crate::app::state::FileManagerLocationsModel,
    ) -> crate::app::state::AppState {
        let file_manager = crate::fm::FmState::open_trail_to(trail_root, target, false)
            .expect("FCL visual Trail target must resolve");
        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![crate::workspace::Workspace::test_new("fcl-agents")];
        app.active = Some(0);
        app.selected = 0;
        app.ensure_test_terminals();
        let root_pane = app.workspaces[0].tabs[0].root_pane;
        let terminal_id = app.workspaces[0].tabs[0].panes[&root_pane]
            .attached_terminal_id
            .clone();
        let terminal = app
            .terminals
            .get_mut(&terminal_id)
            .expect("FCL visual agent terminal");
        terminal.detected_agent = Some(crate::detect::Agent::Claude);
        terminal.state = crate::detect::AgentState::Working;
        app.mode = crate::app::state::Mode::Terminal;
        app.palette = crate::app::state::Palette::catppuccin();
        app.sidebar_tab = crate::app::state::SidebarTab::Files;
        app.mobile_width_threshold = 0;
        app.file_icon_profile = crate::fm::entry_kind::IconProfile::Ascii;
        app.file_manager_locations_model = locations_model;
        app.try_open_file_manager_with(|_| Some(file_manager))
            .expect("FCL visual Files activation");
        assert!(
            app.file_manager_locations
                .activate_location(explicit_origin, &app.file_manager_locations_model),
            "FCL visual explicit origin must be accessible"
        );
        app
    }

    fn render_fcl_visual(
        app: &mut crate::app::state::AppState,
        width: u16,
        height: u16,
        expected_mode: crate::ui::file_manager::locations::FileManagerLocationsMode,
        expected_rail_width: Option<u16>,
        minimum_visible_columns: usize,
        expect_detail: bool,
    ) -> Buffer {
        let area = Rect::new(0, 0, width, height);
        crate::ui::compute_view(app, area);
        let locations = &app.view.file_manager_locations;
        assert_eq!(locations.layout.mode, expected_mode);
        assert_eq!(
            locations.layout.rail.map(|rail| rail.width),
            expected_rail_width
        );
        if let (Some(rail), Some(separator)) = (locations.layout.rail, locations.layout.separator) {
            assert!(
                rail.intersection(separator).is_empty(),
                "FCL visual rail and separator must be disjoint"
            );
            assert!(
                separator.intersection(locations.layout.trail).is_empty(),
                "FCL visual separator and Trail must be disjoint"
            );
        }
        assert!(
            app.view.file_manager_trail.columns.len() >= minimum_visible_columns,
            "FCL visual needs at least {minimum_visible_columns} visible Trail columns"
        );
        assert_eq!(
            app.view.file_manager_trail.detail_panel.is_some(),
            expect_detail,
            "FCL visual detail-panel contract"
        );
        assert!(app.view.file_manager_trail.columns.iter().all(|column| {
            column.rect.x >= locations.layout.trail.x
                && column.rect.right() <= locations.layout.trail.right()
        }));

        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).expect("FCL visual terminal");
        terminal
            .draw(|frame| crate::ui::render(app, frame))
            .expect("render FCL visual frame");
        terminal.backend().buffer().clone()
    }

    fn write_fcl_visual_fixtures(out_dir: &std::path::Path) {
        use crate::app::state::{FileManagerLocationIcon, FileManagerLocationsModel};
        use crate::ui::file_manager::locations::FileManagerLocationsMode;

        let base = std::path::PathBuf::from("/tmp/herdr-vis18-fcl-root");
        let _ = std::fs::remove_dir_all(&base);
        let home = base.join("home");
        let desktop = home.join("Desktop");
        let downloads = home.join("Downloads");
        let documents = home.join("Documents");
        let pictures = home.join("Pictures");
        let screenshots = pictures.join("Screenshots");
        let project = home.join("projects").join("herdr");
        let src = project.join("src");
        let core = src.join("core");
        let mount = base.join("data-disk");
        for directory in [
            &desktop,
            &downloads,
            &documents,
            &screenshots,
            &core,
            &project.join("docs"),
            &mount,
        ] {
            std::fs::create_dir_all(directory).expect("create FCL visual directory");
        }
        let detail_target = core.join("state.rs");
        for (path, contents) in [
            (home.join("readme.txt"), "home"),
            (desktop.join("roadmap.md"), "desktop"),
            (downloads.join("archive.zip"), "download"),
            (documents.join("notes.md"), "documents"),
            (screenshots.join("capture.png"), "image"),
            (project.join("Cargo.toml"), "[package]"),
            (project.join("docs").join("guide.md"), "guide"),
            (src.join("lib.rs"), "pub mod core;"),
            (core.join("engine.rs"), "pub fn run() {}"),
            (detail_target.clone(), "pub struct FilesLocations;"),
            (mount.join("backup.txt"), "backup"),
        ] {
            std::fs::write(path, contents).expect("write FCL visual file");
        }
        for path in [
            home.join("readme.txt"),
            desktop.join("roadmap.md"),
            downloads.join("archive.zip"),
            documents.join("notes.md"),
            screenshots.join("capture.png"),
            project.join("Cargo.toml"),
            project.join("docs").join("guide.md"),
            src.join("lib.rs"),
            core.join("engine.rs"),
            detail_target.clone(),
            mount.join("backup.txt"),
            core.clone(),
            src.clone(),
            project.join("docs"),
            project.clone(),
            home.join("projects"),
            desktop.clone(),
            downloads.clone(),
            documents.clone(),
            screenshots.clone(),
            pictures.clone(),
            mount.clone(),
            home.clone(),
        ] {
            set_mtime_fixture_modified(
                &path,
                mtime_fixture_system_time(2026, time::Month::January, 10, 10, 0),
            );
        }

        let locations_model = FileManagerLocationsModel::from_sources(
            vec![
                fcl_location_item("Home", home.clone(), FileManagerLocationIcon::Home),
                fcl_location_item("Desktop", desktop.clone(), FileManagerLocationIcon::Desktop),
                fcl_location_item(
                    "Downloads",
                    downloads.clone(),
                    FileManagerLocationIcon::Downloads,
                ),
                fcl_location_item(
                    "Documents",
                    documents.clone(),
                    FileManagerLocationIcon::Documents,
                ),
                fcl_location_item(
                    "Pictures",
                    pictures.clone(),
                    FileManagerLocationIcon::Pictures,
                ),
                fcl_location_item(
                    "Herdr Project",
                    project.clone(),
                    FileManagerLocationIcon::Pin,
                ),
            ],
            vec![fcl_location_item(
                "Screenshots",
                screenshots,
                FileManagerLocationIcon::Pin,
            )],
            vec![fcl_location_item(
                "Data Disk",
                mount,
                FileManagerLocationIcon::Disk,
            )],
        );

        let mut wide = fcl_visual_app(&home, &detail_target, &home, locations_model.clone());
        let wide_buffer = render_fcl_visual(
            &mut wide,
            220,
            40,
            FileManagerLocationsMode::Wide,
            Some(24),
            4,
            true,
        );
        let wide_text = wide_buffer
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(wide_text.contains("fcl-agents"));
        assert!(wide_text.contains("claude"));
        write_fixture(
            out_dir,
            export_cell_fixture("vis-18-files-locations-wide", &wide_buffer),
        );

        let mut home_origin = fcl_visual_app(&home, &detail_target, &home, locations_model.clone());
        write_fixture(
            out_dir,
            export_cell_fixture(
                "vis-19-files-locations-home-origin",
                &render_fcl_visual(
                    &mut home_origin,
                    120,
                    28,
                    FileManagerLocationsMode::Wide,
                    Some(24),
                    2,
                    true,
                ),
            ),
        );

        let mut nested_origin =
            fcl_visual_app(&project, &detail_target, &project, locations_model.clone());
        nested_origin.sidebar_collapsed = true;
        nested_origin.sidebar_collapsed_mode = crate::config::SidebarCollapsedModeConfig::Hidden;
        write_fixture(
            out_dir,
            export_cell_fixture(
                "vis-20-files-locations-nested-origin",
                &render_fcl_visual(
                    &mut nested_origin,
                    44,
                    24,
                    FileManagerLocationsMode::Standard,
                    Some(20),
                    1,
                    false,
                ),
            ),
        );

        let mut standard = fcl_visual_app(&home, &detail_target, &home, locations_model.clone());
        standard.sidebar_collapsed = true;
        standard.sidebar_collapsed_mode = crate::config::SidebarCollapsedModeConfig::Hidden;
        let standard_fm = standard.file_manager.as_mut().expect("standard FCL Files");
        standard_fm.miller.horizontal.offset_cells = 10;
        standard_fm.miller.horizontal.follow_active = false;
        let standard_buffer = render_fcl_visual(
            &mut standard,
            44,
            24,
            FileManagerLocationsMode::Standard,
            Some(20),
            1,
            false,
        );
        assert!(
            standard.view.file_manager_trail.offset_cells > 0,
            "standard FCL fixture must preserve a fractional Trail origin"
        );
        write_fixture(
            out_dir,
            export_cell_fixture("vis-21-files-locations-standard", &standard_buffer),
        );

        let mut compact =
            fcl_visual_app(&project, &detail_target, &project, locations_model.clone());
        compact.sidebar_collapsed = true;
        compact.sidebar_collapsed_mode = crate::config::SidebarCollapsedModeConfig::Hidden;
        write_fixture(
            out_dir,
            export_cell_fixture(
                "vis-22-files-locations-compact-closed",
                &render_fcl_visual(
                    &mut compact,
                    30,
                    24,
                    FileManagerLocationsMode::Compact,
                    None,
                    1,
                    false,
                ),
            ),
        );
        assert!(compact.file_manager_locations.open_drawer());
        let compact_open = render_fcl_visual(
            &mut compact,
            30,
            24,
            FileManagerLocationsMode::Compact,
            None,
            1,
            false,
        );
        assert!(compact.view.file_manager_locations.drawer_area.is_some());
        write_fixture(
            out_dir,
            export_cell_fixture("vis-23-files-locations-compact-open", &compact_open),
        );

        let mut transition = fcl_visual_app(&home, &detail_target, &home, locations_model);
        transition.sidebar_collapsed = true;
        transition.sidebar_collapsed_mode = crate::config::SidebarCollapsedModeConfig::Hidden;
        let files_generation = transition
            .stage
            .active_instance_generation()
            .expect("pending FCL Files generation");
        transition.file_manager_locations.begin_load(
            downloads.clone(),
            files_generation,
            transition.file_manager_locations_model.revision(),
            91,
        );
        let pending = render_fcl_visual(
            &mut transition,
            96,
            24,
            FileManagerLocationsMode::Wide,
            Some(24),
            2,
            true,
        );
        let pending_directories: Vec<_> = transition
            .view
            .file_manager_trail
            .columns
            .iter()
            .map(|column| column.directory.clone())
            .collect();
        write_fixture(
            out_dir,
            export_cell_fixture("vis-24-files-locations-pending", &pending),
        );

        transition.file_manager_locations.fail_load(
            downloads,
            crate::app::FileManagerLocationLoadError::PermissionDenied,
        );
        let failed = render_fcl_visual(
            &mut transition,
            96,
            24,
            FileManagerLocationsMode::Wide,
            Some(24),
            2,
            true,
        );
        assert_eq!(
            transition
                .view
                .file_manager_trail
                .columns
                .iter()
                .map(|column| column.directory.clone())
                .collect::<Vec<_>>(),
            pending_directories,
            "pending-to-failure transition must retain the prior Trail"
        );
        write_fixture(
            out_dir,
            export_cell_fixture("vis-25-files-locations-failure", &failed),
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    #[ignore = "exports FCL visual fixtures; set HERDR_VISUAL_FIXTURE_DIR explicitly"]
    fn write_files_locations_visual_fixtures() {
        let out_dir = std::path::PathBuf::from(
            std::env::var("HERDR_VISUAL_FIXTURE_DIR")
                .expect("HERDR_VISUAL_FIXTURE_DIR must point at the fixture output directory"),
        );
        std::fs::create_dir_all(&out_dir).expect("create FCL fixture output dir");
        write_fcl_visual_fixtures(&out_dir);
    }

    // Exports the real UI states consumed by the Playwright Chromium specs.
    // Only an explicit run writes fixtures, and only into the caller-provided
    // directory; ordinary unit runs never touch the filesystem.
    #[test]
    #[ignore = "exports visual fixtures; set HERDR_VISUAL_FIXTURE_DIR and run explicitly"]
    fn write_visual_fixtures() {
        let out_dir = std::path::PathBuf::from(
            std::env::var("HERDR_VISUAL_FIXTURE_DIR")
                .expect("HERDR_VISUAL_FIXTURE_DIR must point at the fixture output directory"),
        );
        std::fs::create_dir_all(&out_dir).expect("create fixture output dir");

        let render_state = |app: &crate::app::state::AppState, width: u16, height: u16| {
            let backend = TestBackend::new(width, height);
            let mut terminal = Terminal::new(backend).expect("test terminal");
            terminal
                .draw(|frame| crate::ui::render(app, frame))
                .expect("render frame");
            terminal.backend().buffer().clone()
        };

        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![crate::workspace::Workspace::test_new("vis")];
        app.active = Some(0);
        app.selected = 0;
        app.mode = crate::app::state::Mode::Terminal;
        app.mobile_width_threshold = 0;
        crate::ui::compute_view(&mut app, Rect::new(0, 0, 120, 40));
        write_fixture(
            &out_dir,
            export_cell_fixture("vis-01-terminal", &render_state(&app, 120, 40)),
        );

        // Fixed filesystem base so cwd labels stay identical across exports.
        // The FM opens the INNER directory so the rendered parent column shows
        // only the controlled base content, never the live /tmp listing.
        let base = std::path::PathBuf::from("/tmp/herdr-vis01-root");
        let _ = std::fs::remove_dir_all(&base);
        let root = base.join("inner");
        for dir in ["alpha", "beta"] {
            std::fs::create_dir_all(root.join(dir)).expect("fixture dir");
        }
        for file in ["gamma.rs", "notes.txt"] {
            std::fs::write(root.join(file), b"x").expect("fixture file");
        }
        std::fs::create_dir_all(root.join("beta").join("deep")).expect("fixture deep dir");
        std::fs::write(root.join("beta").join("inner.txt"), b"x").expect("fixture inner file");
        let vis01_modified = mtime_fixture_system_time(2026, time::Month::January, 10, 11, 55);
        for path in [
            root.join("alpha"),
            root.join("beta"),
            root.join("gamma.rs"),
            root.join("notes.txt"),
            root.join("beta").join("deep"),
            root.join("beta").join("inner.txt"),
        ] {
            set_mtime_fixture_modified(&path, vis01_modified);
        }
        // Browser fonts do not carry Nerd PUA glyphs. The visual oracle uses
        // the deterministic ASCII profile so every semantic entry icon is
        // visible in Chromium.
        app.file_icon_profile = crate::fm::entry_kind::IconProfile::Ascii;
        app.try_open_file_manager_with(|_| Some(crate::fm::FmState::new(root.clone())))
            .expect("files stage must open for the fixture");
        crate::ui::compute_view(&mut app, Rect::new(0, 0, 120, 40));
        write_fixture(
            &out_dir,
            export_cell_fixture("vis-01-files", &render_state(&app, 120, 40)),
        );

        // TP-FIP-VIS-02 / TP-TRAIL-T7-RENDER-05: descend through the NONZERO
        // child `beta` (index 1) and then into `deep`; the accumulating Trail
        // must retain exact ancestor highlights, never substitute row zero.
        {
            let fm = app.file_manager.as_mut().expect("open file manager");
            let beta = root.join("beta");
            let beta_index = fm
                .entries
                .iter()
                .position(|entry| entry.path == beta)
                .expect("beta row");
            assert!(beta_index > 0, "fixture requires a nonzero child index");
            assert!(fm.select(beta_index));
            fm.enter();
            let deep = beta.join("deep");
            let deep_index = fm
                .entries
                .iter()
                .position(|entry| entry.path == deep)
                .expect("deep row");
            assert!(fm.select(deep_index));
            fm.enter();
            assert_eq!(fm.cwd, deep);
        }
        // 160 cells wide so the loaded `inner → beta → deep` Trail stays
        // visible as complete columns in the bounded window.
        crate::ui::compute_view(&mut app, Rect::new(0, 0, 160, 40));
        write_fixture(
            &out_dir,
            export_cell_fixture("vis-02-resident-focus", &render_state(&app, 160, 40)),
        );
        let _ = std::fs::remove_dir_all(&base);

        // TP-FIP-VIS-03/04: a mixed-kind directory rendered with the
        // deterministic ASCII icon profile (Nerd private-use glyphs render
        // empty in the browser font). Same base/inner isolation pattern.
        let icon_base = std::path::PathBuf::from("/tmp/herdr-vis03-root");
        let _ = std::fs::remove_dir_all(&icon_base);
        let icon_root = icon_base.join("inner");
        std::fs::create_dir_all(icon_root.join("alpha")).expect("fixture dir");
        for file in [
            "main.rs",
            "config.toml",
            "notes.md",
            "photo.png",
            "pack.zip",
        ] {
            std::fs::write(icon_root.join(file), b"x").expect("fixture file");
        }
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(icon_root.join("alpha"), icon_root.join("link-dir"))
                .expect("link-dir");
            std::os::unix::fs::symlink(icon_root.join("main.rs"), icon_root.join("link-file"))
                .expect("link-file");
            std::os::unix::fs::symlink(icon_root.join("missing"), icon_root.join("broken"))
                .expect("broken");
            let status = std::process::Command::new("mkfifo")
                .arg(icon_root.join("fifo"))
                .status()
                .expect("mkfifo runs");
            assert!(status.success(), "fifo fixture must exist");
        }

        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![crate::workspace::Workspace::test_new("vis")];
        app.active = Some(0);
        app.selected = 0;
        app.mode = crate::app::state::Mode::Terminal;
        app.mobile_width_threshold = 0;
        app.file_icon_profile = crate::fm::entry_kind::IconProfile::Ascii;
        app.try_open_file_manager_with(|_| Some(crate::fm::FmState::new(icon_root.clone())))
            .expect("files stage must open for the icon fixture");
        crate::ui::compute_view(&mut app, Rect::new(0, 0, 120, 40));
        write_fixture(
            &out_dir,
            export_cell_fixture("vis-03-icons-ascii", &render_state(&app, 120, 40)),
        );
        {
            let fm = app.file_manager.as_mut().expect("open icon file manager");
            let main_index = fm
                .entries
                .iter()
                .position(|entry| entry.path == icon_root.join("main.rs"))
                .expect("main.rs row");
            assert!(fm.select(main_index));
        }
        crate::ui::compute_view(&mut app, Rect::new(0, 0, 60, 16));
        assert!(
            !app.view.file_manager_trail.columns.is_empty(),
            "tiny icon fixture must exercise one complete Trail column"
        );
        write_fixture(
            &out_dir,
            export_cell_fixture("vis-04-icons-tiny", &render_state(&app, 60, 16)),
        );

        // TP-FIP-VIS-05/06: the blocking agent picker over the Files stage —
        // current agent first and preselected, then a disabled (vanished)
        // second row on a tiny screen. Rows are constructed deterministically
        // from the fixture workspace's real pane/terminal identities.
        let workspace_id = app.workspaces[0].id.clone();
        let pane_id = app.workspaces[0].tabs[0].root_pane;
        let terminal_id = app.workspaces[0]
            .terminal_id(pane_id)
            .expect("fixture terminal identity")
            .clone();
        let row = |label: &str, is_current: bool, live: bool| {
            crate::app::agent_reference_picker::AgentReferencePickerRow {
                label: label.to_string(),
                is_current,
                workspace_id: workspace_id.clone(),
                pane_id,
                terminal_id: terminal_id.clone(),
                live,
            }
        };
        app.agent_reference_picker = Some(
            crate::app::agent_reference_picker::AgentReferencePickerState {
                source_path: icon_root.join("main.rs"),
                source_files_generation: 0,
                rows: vec![
                    row("claude — vis", true, true),
                    row("codex — build", false, true),
                ],
                selected: 0,
            },
        );
        app.mode = crate::app::state::Mode::AgentReferencePicker;
        crate::ui::compute_view(&mut app, Rect::new(0, 0, 120, 40));
        write_fixture(
            &out_dir,
            export_cell_fixture("vis-05-picker", &render_state(&app, 120, 40)),
        );

        if let Some(picker) = app.agent_reference_picker.as_mut() {
            picker.rows[1].live = false;
        }
        crate::ui::compute_view(&mut app, Rect::new(0, 0, 60, 20));
        write_fixture(
            &out_dir,
            export_cell_fixture("vis-06-picker-disabled-tiny", &render_state(&app, 60, 20)),
        );
        let _ = std::fs::remove_dir_all(&icon_base);

        // VIS-07/08 (trail program T3): the standalone Miller trail render —
        // four accumulated columns with the selection emphasized in every
        // ancestor (LAW 1/2), then the same trail after an ancestor-sibling
        // reselect cut the old branch (rebranch proof). Deterministic fixed
        // tree; ASCII icon profile for browser-visible glyphs.
        let trail_base = std::path::PathBuf::from("/tmp/herdr-vis07-root");
        let _ = std::fs::remove_dir_all(&trail_base);
        let trail_root = trail_base.join("inner");
        let src = trail_root.join("src");
        let core = src.join("core");
        let detail = core.join("detail");
        std::fs::create_dir_all(&detail).expect("trail tree");
        std::fs::create_dir_all(trail_root.join("docs")).expect("docs dir");
        std::fs::create_dir_all(trail_root.join("assets")).expect("assets dir");
        for (dir, files) in [
            (&trail_root, &["notes.md", "readme.md"][..]),
            (&src, &["lib.rs", "main.rs"][..]),
            (&core, &["engine.rs", "state.rs"][..]),
            (&detail, &["alpha.rs", "beta.rs"][..]),
            (&trail_root.join("docs"), &["guide.md", "spec.md"][..]),
        ] {
            for file in files {
                std::fs::write(dir.join(file), b"x").expect("trail file");
            }
        }

        let mut trail_app = crate::app::state::AppState::test_new();
        trail_app.file_icon_profile = crate::fm::entry_kind::IconProfile::Ascii;
        let mut trail = crate::fm::trail::TrailState::new(&trail_root);
        let mut snaps = crate::fm::trail_snapshots::TrailSnapshots::new(false);
        snaps.sync(&trail);
        for (col, dir) in [(0usize, &src), (1, &core), (2, &detail)] {
            assert_eq!(
                snaps.select_dir(&mut trail, col, dir),
                crate::fm::FmDirectoryStatus::Available,
                "trail fixture descent must load"
            );
        }
        let fractional_trail = trail.clone();
        let fractional_snaps = snaps.clone();
        assert!(trail.select_file(3, &detail.join("alpha.rs")));
        let render_trail = |trail: &crate::fm::trail::TrailState,
                            snaps: &crate::fm::trail_snapshots::TrailSnapshots,
                            width: u16,
                            height: u16| {
            let backend = TestBackend::new(width, height);
            let mut terminal = Terminal::new(backend).expect("trail terminal");
            let stage = Rect::new(0, 0, width, height);
            let view =
                crate::ui::file_manager::trail_view::project_trail_view(stage, trail, snaps, &[]);
            assert!(!view.columns.is_empty(), "trail fixture must project");
            terminal
                .draw(|frame| {
                    crate::ui::file_manager::trail_view::render_trail_view(
                        &trail_app, frame, &view, snaps,
                    );
                })
                .expect("render trail frame");
            terminal.backend().buffer().clone()
        };
        write_fixture(
            &out_dir,
            export_cell_fixture("vis-07-trail-depth", &render_trail(&trail, &snaps, 120, 40)),
        );

        // VIS-11 (scrollable Trail viewport): after auto-following the active
        // end, a user-selected origin exposes the still-live root ancestors
        // in a narrow viewport instead of snapping back to the deepest column.
        let render_scrolled_trail = |requested_first_visible: usize| {
            let width = 60;
            let height = 20;
            let backend = TestBackend::new(width, height);
            let mut terminal = Terminal::new(backend).expect("scrolled trail terminal");
            let stage = Rect::new(0, 0, width, height);
            let view = crate::ui::file_manager::trail_view::project_trail_view_with_origin(
                stage,
                &trail,
                &snaps,
                &[],
                crate::ui::file_manager::trail_view::TRAIL_DETAIL_PANEL_DEFAULT_WIDTH,
                requested_first_visible as u32,
            );
            assert_eq!(
                view.first_visible, requested_first_visible,
                "fixture must preserve the explicit horizontal origin"
            );
            terminal
                .draw(|frame| {
                    crate::ui::file_manager::trail_view::render_trail_view(
                        &trail_app, frame, &view, &snaps,
                    );
                })
                .expect("render scrolled trail frame");
            terminal.backend().buffer().clone()
        };
        write_fixture(
            &out_dir,
            export_cell_fixture("vis-11-trail-horizontal-scroll", &render_scrolled_trail(0)),
        );

        // VIS-12 (fractional Trail viewport): the viewport begins ten cells
        // inside the 30-cell second column, so the leading column is clipped
        // to 20 cells and the 48-cell trailing column begins at the right
        // edge. The fixture is the real Ratatui buffer, not an HTML layout
        // reconstruction.
        {
            let width = 60;
            let height = 20;
            let preferred_widths = [18_u16, 30, 48, 24];
            let offset_cells = 18_u32 + 1 + 10;
            let stage = Rect::new(0, 0, width, height);
            let view = crate::ui::file_manager::trail_view::project_trail_view_with_origin(
                stage,
                &fractional_trail,
                &fractional_snaps,
                &preferred_widths,
                crate::ui::file_manager::trail_view::TRAIL_DETAIL_PANEL_DEFAULT_WIDTH,
                offset_cells,
            );
            let leading = view.columns.first().expect("fractional leading column");
            let trailing = view.columns.last().expect("fractional trailing column");
            assert_eq!(
                (leading.trail_index, leading.source_x, leading.rect.width),
                (1, 10, 20)
            );
            assert_eq!(trailing.trail_index, 2);
            assert!(
                trailing.rect.width > 0 && trailing.rect.width < trailing.logical_width,
                "fixture must expose the beginning of a clipped trailing column"
            );
            let backend = TestBackend::new(width, height);
            let mut terminal = Terminal::new(backend).expect("fractional trail terminal");
            terminal
                .draw(|frame| {
                    crate::ui::file_manager::trail_view::render_trail_view(
                        &trail_app,
                        frame,
                        &view,
                        &fractional_snaps,
                    );
                })
                .expect("render fractional trail frame");
            write_fixture(
                &out_dir,
                export_cell_fixture(
                    "vis-12-fractional-miller-scroll",
                    terminal.backend().buffer(),
                ),
            );
        }

        // Rebranch: reselect the sibling `docs` at the ROOT column — the old
        // src/core/detail branch is discarded and the trail regrows.
        assert_eq!(
            snaps.select_dir(&mut trail, 0, &trail_root.join("docs")),
            crate::fm::FmDirectoryStatus::Available
        );
        assert!(trail.select_file(1, &trail_root.join("docs").join("guide.md")));
        snaps.sync(&trail);
        write_fixture(
            &out_dir,
            export_cell_fixture(
                "vis-08-trail-rebranch",
                &render_trail(&trail, &snaps, 120, 40),
            ),
        );

        // VIS-09 (trail LAW 3): activating a FILE through the input seam
        // opens the resizable right-side detail panel with the prepared text
        // preview while the sibling columns stay visible.
        std::fs::write(
            trail_root.join("docs").join("guide.md"),
            b"# Guide\n\ntrail detail panel preview",
        )
        .expect("guide content");
        let docs_col = &snaps.cols()[1];
        let guide = trail_root.join("docs").join("guide.md");
        let guide_index = docs_col
            .entries()
            .iter()
            .position(|entry| entry.path == guide)
            .expect("guide row");
        assert_eq!(
            snaps.activate_entry(&mut trail, 1, guide_index, &guide),
            crate::fm::trail_snapshots::TrailActivateOutcome::SelectedFile
        );
        write_fixture(
            &out_dir,
            export_cell_fixture(
                "vis-09-trail-detail-panel",
                &render_trail(&trail, &snaps, 120, 40),
            ),
        );

        // VIS-14 (FMR-3): heavyweight document types stay explicit and
        // bounded in native Files instead of being misread as text.
        let manual = trail_root.join("docs").join("manual.pdf");
        std::fs::write(&manual, b"%PDF fixture").expect("metadata fixture");
        let mut metadata_snaps = crate::fm::trail_snapshots::TrailSnapshots::new(false);
        let metadata_trail = metadata_snaps
            .open_trail_to(&trail_root, &manual)
            .expect("metadata detail fixture resolves");
        write_fixture(
            &out_dir,
            export_cell_fixture(
                "vis-14-trail-metadata-preview",
                &render_trail(&metadata_trail, &metadata_snaps, 120, 40),
            ),
        );

        // VIS-10 (trail LAW 5 / FIP-D1): a sidebar deep-link builds the whole
        // trail from scratch — fresh snapshots, ancestor chain resolved down
        // to the file, detail panel open. This is the acceptance visual for
        // "favorites click constructs the trail correctly".
        let mut deep_snaps = crate::fm::trail_snapshots::TrailSnapshots::new(false);
        let deep_trail = deep_snaps
            .open_trail_to(&trail_root, &core.join("state.rs"))
            .expect("deep link fixture resolves");
        write_fixture(
            &out_dir,
            export_cell_fixture(
                "vis-10-trail-deep-link",
                &render_trail(&deep_trail, &deep_snaps, 120, 40),
            ),
        );

        // VIS-13 (FMR-1): a mixed directory keeps its actionable file row
        // while a separate non-actionable status row explains hidden items.
        let omission_root = trail_base.join("omissions");
        std::fs::create_dir_all(&omission_root).expect("omission fixture dir");
        std::fs::write(omission_root.join("visible.txt"), b"x").expect("visible fixture");
        std::fs::write(omission_root.join(".secret"), b"x").expect("hidden fixture");
        let omission_trail = crate::fm::trail::TrailState::new(&omission_root);
        let mut omission_snaps = crate::fm::trail_snapshots::TrailSnapshots::new(false);
        omission_snaps.sync(&omission_trail);
        write_fixture(
            &out_dir,
            export_cell_fixture(
                "vis-13-trail-directory-omissions",
                &render_trail(&omission_trail, &omission_snaps, 80, 20),
            ),
        );
        let _ = std::fs::remove_dir_all(&trail_base);

        // VIS-15..17 (MTIME-1..5): a deterministic UTC calendar tree proves
        // mixed directory/file ordering, responsive timestamp omission, and
        // exact-path selection after an mtime-driven reorder. Build every
        // node first, then set FileTimes last so directory creation cannot
        // perturb the approved order.
        let mtime_base = std::path::PathBuf::from("/tmp/herdr-vis15-mtime-root");
        let _ = std::fs::remove_dir_all(&mtime_base);
        let mtime_root = mtime_base.join("inner");
        let active_dir = mtime_root.join("active-project");
        let yesterday_dir = mtime_root.join("yesterday-folder");
        let archive_dir = mtime_root.join("archive");
        std::fs::create_dir_all(&active_dir).expect("active project fixture");
        std::fs::create_dir_all(&yesterday_dir).expect("yesterday fixture");
        std::fs::create_dir_all(&archive_dir).expect("archive fixture");
        for path in [
            mtime_root.join("today-notes.txt"),
            mtime_root.join("week-report.md"),
            mtime_root.join("promoted.txt"),
            active_dir.join("latest.rs"),
            active_dir.join("yesterday.log"),
            active_dir.join("week-plan.md"),
            active_dir.join("legacy.txt"),
        ] {
            std::fs::write(path, b"mtime visual fixture").expect("mtime fixture file");
        }

        for (path, day, hour, minute) in [
            (active_dir.clone(), 10, 11, 45),
            (mtime_root.join("today-notes.txt"), 10, 10, 15),
            (yesterday_dir.clone(), 9, 16, 30),
            (mtime_root.join("week-report.md"), 7, 14, 5),
            (archive_dir, 1, 8, 0),
            (mtime_root.join("promoted.txt"), 1, 7, 30),
            (active_dir.join("latest.rs"), 10, 11, 20),
            (active_dir.join("yesterday.log"), 9, 9, 10),
            (active_dir.join("week-plan.md"), 6, 13, 40),
            (active_dir.join("legacy.txt"), 1, 6, 5),
        ] {
            set_mtime_fixture_modified(
                &path,
                mtime_fixture_system_time(2026, time::Month::January, day, hour, minute),
            );
        }

        let mut mtime_trail = crate::fm::trail::TrailState::new(&mtime_root);
        let mut mtime_snaps = crate::fm::trail_snapshots::TrailSnapshots::new(false);
        mtime_snaps.sync(&mtime_trail);
        assert_eq!(
            mtime_snaps.select_dir(&mut mtime_trail, 0, &active_dir),
            crate::fm::FmDirectoryStatus::Available,
            "mtime fixture descent must load"
        );
        let render_mtime_trail = |name: &str,
                                  trail: &crate::fm::trail::TrailState,
                                  snaps: &crate::fm::trail_snapshots::TrailSnapshots,
                                  width: u16,
                                  height: u16,
                                  offset_cells: Option<u32>| {
            let backend = TestBackend::new(width, height);
            let mut terminal = Terminal::new(backend).expect("mtime fixture terminal");
            let stage = Rect::new(0, 0, width, height);
            let view = match offset_cells {
                Some(offset) => {
                    crate::ui::file_manager::trail_view::project_trail_view_at_with_origin(
                        stage,
                        trail,
                        snaps,
                        &[48, 48],
                        offset,
                        mtime_fixture_anchor(),
                    )
                }
                None => crate::ui::file_manager::trail_view::project_trail_view_at(
                    stage,
                    trail,
                    snaps,
                    &[48, 48],
                    mtime_fixture_anchor(),
                ),
            };
            assert!(!view.columns.is_empty(), "mtime fixture must project");
            if name == "vis-16-mtime-groups-narrow" {
                assert!(
                    view.columns.iter().any(|column| {
                        column.source_x > 0 && column.rows.iter().all(|row| row.timestamp.is_none())
                    }),
                    "narrow fixture must include a clipped timestamp-free column"
                );
                assert!(
                    view.columns
                        .iter()
                        .flat_map(|column| &column.rows)
                        .all(|row| {
                            row.actions
                                .iter()
                                .all(|action| action.rect.intersection(row.rect) == action.rect)
                        }),
                    "narrow fixture actions must remain complete"
                );
            }
            terminal
                .draw(|frame| {
                    crate::ui::file_manager::trail_view::render_trail_view(
                        &trail_app, frame, &view, snaps,
                    );
                })
                .expect("render mtime fixture");
            write_fixture(
                &out_dir,
                export_cell_fixture(name, terminal.backend().buffer()),
            );
        };
        render_mtime_trail(
            "vis-15-mtime-groups",
            &mtime_trail,
            &mtime_snaps,
            96,
            24,
            None,
        );
        render_mtime_trail(
            "vis-16-mtime-groups-narrow",
            &mtime_trail,
            &mtime_snaps,
            44,
            16,
            Some(42),
        );

        let promoted = mtime_root.join("promoted.txt");
        assert!(mtime_trail.select_file(0, &promoted));
        set_mtime_fixture_modified(
            &promoted,
            mtime_fixture_system_time(2026, time::Month::January, 10, 11, 55),
        );
        assert!(
            mtime_snaps.refresh_col(0),
            "mtime reorder fixture must refresh the owning column"
        );
        assert!(
            mtime_snaps.reconcile_refreshed_col(&mut mtime_trail, 0),
            "mtime reorder fixture must reconcile exact-path Trail authority"
        );
        let selected_path = mtime_trail.cols()[0]
            .selected
            .as_deref()
            .expect("mtime reorder selection");
        assert_eq!(selected_path, promoted);
        render_mtime_trail(
            "vis-17-mtime-reorder-selection",
            &mtime_trail,
            &mtime_snaps,
            72,
            18,
            None,
        );
        let _ = std::fs::remove_dir_all(&mtime_base);

        // VIS-18..25 (FCL-6): the full Native Files composition proves the
        // content-owned locations rail, explicit-origin authority, responsive
        // drawer, and fail-closed loading states in deterministic ASCII/UTC.
        write_fcl_visual_fixtures(&out_dir);
    }

    #[test]
    fn exported_fixture_serializes_every_cell_with_style() {
        let backend = TestBackend::new(4, 2);
        let mut terminal = Terminal::new(backend).expect("test terminal");
        terminal
            .draw(|frame| {
                let paragraph = Paragraph::new("ab")
                    .style(Style::default().fg(Color::Rgb(1, 2, 3)).bg(Color::Black));
                frame.render_widget(paragraph, Rect::new(0, 0, 2, 1));
            })
            .expect("draw");
        let fixture = export_cell_fixture("nav-01", terminal.backend().buffer());
        assert_eq!(fixture.width, 4);
        assert_eq!(fixture.height, 2);
        assert_eq!(fixture.name, "nav-01");
        assert_eq!(fixture.cells.len(), 8);
        assert_eq!(fixture.cells[0].symbol, "a");
        assert_eq!(fixture.cells[0].fg, "rgb(1,2,3)");
        assert_eq!(fixture.cells[0].bg, "black");
        assert_eq!((fixture.cells[0].x, fixture.cells[0].y), (0, 0));
        assert_eq!(fixture.cells[1].symbol, "b");
        assert_eq!((fixture.cells[5].x, fixture.cells[5].y), (1, 1));
    }

    #[test]
    fn exported_fixture_round_trips_modifiers_and_serializes_to_json() {
        let backend = TestBackend::new(2, 1);
        let mut terminal = Terminal::new(backend).expect("test terminal");
        terminal
            .draw(|frame| {
                let paragraph = Paragraph::new("x").style(
                    Style::default()
                        .add_modifier(Modifier::BOLD)
                        .add_modifier(Modifier::UNDERLINED),
                );
                frame.render_widget(paragraph, Rect::new(0, 0, 1, 1));
            })
            .expect("draw");
        let fixture = export_cell_fixture("mod-check", terminal.backend().buffer());
        assert_eq!(
            fixture.cells[0].modifiers,
            vec!["bold".to_string(), "underlined".to_string()]
        );
        let json = serde_json::to_string(&fixture).expect("serialize fixture");
        assert!(json.contains("\"name\":\"mod-check\""));
        assert!(json.contains("\"symbol\":\"x\""));
    }
}
