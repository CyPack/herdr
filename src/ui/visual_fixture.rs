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
        app.try_open_file_manager_with(|_| Some(crate::fm::FmState::new(root.clone())))
            .expect("files stage must open for the fixture");
        crate::ui::compute_view(&mut app, Rect::new(0, 0, 120, 40));
        write_fixture(
            &out_dir,
            export_cell_fixture("vis-01-files", &render_state(&app, 120, 40)),
        );

        // TP-FIP-VIS-02: descend through the NONZERO child `beta` (index 1)
        // and then into `deep`; the resident `inner` column must highlight
        // `beta`, never row zero.
        {
            let fm = app.file_manager.as_mut().expect("open file manager");
            let beta = root.join("beta");
            let beta_index = fm
                .entries
                .iter()
                .position(|entry| entry.path == beta)
                .expect("beta row");
            assert!(beta_index > 0, "fixture requires a nonzero child index");
            fm.cursor = beta_index;
            fm.enter();
            let deep = beta.join("deep");
            let deep_index = fm
                .entries
                .iter()
                .position(|entry| entry.path == deep)
                .expect("deep row");
            fm.cursor = deep_index;
            fm.enter();
            assert_eq!(fm.cwd, deep);
        }
        // 160 cells wide so the resident `inner` column stays inside the
        // bounded window next to parent/current/preview.
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
        crate::ui::compute_view(&mut app, Rect::new(0, 0, 60, 16));
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
        let _ = std::fs::remove_dir_all(&trail_base);
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
