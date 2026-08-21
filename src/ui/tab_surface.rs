use ratatui::{layout::Rect, Frame};

use super::panes::{render_panes, resize_tab_panes};
use crate::app::state::ViewState;
use crate::app::{AppState, Mode};
use crate::layout::{PaneInfo, SplitBorder};
use crate::protocol::CursorState;
use crate::terminal::TerminalRuntimeRegistry;

pub(crate) struct TabSurfaceLayout {
    pub(crate) pane_infos: Vec<PaneInfo>,
    pub(crate) split_borders: Vec<SplitBorder>,
}

#[derive(Clone, Copy)]
pub(crate) struct TabSurfaceView<'a> {
    pub(crate) pane_infos: &'a [PaneInfo],
    pub(crate) split_borders: &'a [SplitBorder],
}

impl ViewState {
    pub(crate) fn tab_surface(&self) -> TabSurfaceView<'_> {
        TabSurfaceView {
            pane_infos: &self.pane_infos,
            split_borders: &self.split_borders,
        }
    }
}

pub(crate) fn compute_tab_surface(
    app: &AppState,
    terminal_runtimes: &TerminalRuntimeRegistry,
    area: Rect,
    resize_panes: bool,
    cell_size: crate::kitty_graphics::HostCellSize,
) -> TabSurfaceLayout {
    match app.active {
        Some(ws_idx) => compute_tab_surface_for(
            app,
            terminal_runtimes,
            ws_idx,
            area,
            resize_panes,
            cell_size,
        ),
        None => TabSurfaceLayout {
            pane_infos: Vec::new(),
            split_borders: Vec::new(),
        },
    }
}

/// The same layout for ONE NAMED workspace (TP-STAGE-SBS-01).
pub(crate) fn compute_tab_surface_for(
    app: &AppState,
    terminal_runtimes: &TerminalRuntimeRegistry,
    ws_idx: usize,
    area: Rect,
    resize_panes: bool,
    cell_size: crate::kitty_graphics::HostCellSize,
) -> TabSurfaceLayout {
    let split_borders = app
        .workspaces
        .get(ws_idx)
        .map(|ws| {
            if ws.zoomed {
                Vec::new()
            } else {
                ws.layout.splits(area)
            }
        })
        .unwrap_or_default();
    let pane_infos = super::panes::compute_pane_infos_for(
        app,
        terminal_runtimes,
        ws_idx,
        area,
        resize_panes,
        cell_size,
    );

    TabSurfaceLayout {
        pane_infos,
        split_borders,
    }
}

pub(crate) fn resize_tab_surface(
    app: &AppState,
    terminal_runtimes: &TerminalRuntimeRegistry,
    tab: &crate::workspace::Tab,
    area: Rect,
    cell_size: crate::kitty_graphics::HostCellSize,
) {
    resize_tab_panes(app, terminal_runtimes, tab, area, cell_size);
}

pub(crate) fn render_tab_surface(
    app: &AppState,
    terminal_runtimes: &TerminalRuntimeRegistry,
    surface: TabSurfaceView<'_>,
    frame: &mut Frame,
) {
    render_panes(
        app,
        terminal_runtimes,
        frame,
        surface.pane_infos,
        surface.split_borders,
    );
}

pub(crate) fn tab_surface_hyperlinks(
    app: &AppState,
    terminal_runtimes: &TerminalRuntimeRegistry,
    surface: TabSurfaceView<'_>,
) -> Vec<((u16, u16), String, String)> {
    let Some(ws_idx) = app.active else {
        return Vec::new();
    };
    if app.workspaces.get(ws_idx).is_none() {
        return Vec::new();
    }

    let mut links = Vec::new();
    for info in surface.pane_infos {
        if let Some(runtime) = app.runtime_for_pane_in_workspace(terminal_runtimes, ws_idx, info.id)
        {
            links.extend(runtime.visible_hyperlinks(info.inner_rect));
        }
    }
    links
}

pub(crate) fn tab_surface_cursor(
    app: &AppState,
    terminal_runtimes: &TerminalRuntimeRegistry,
    surface: TabSurfaceView<'_>,
) -> Option<CursorState> {
    if app.mode != Mode::Terminal {
        return None;
    }

    let ws_idx = app.active?;
    let info = surface.pane_infos.iter().find(|info| info.is_focused)?;
    if !app.pane_exposes_host_cursor(ws_idx, info.id) {
        return None;
    }
    let runtime = app.runtime_for_pane_in_workspace(terminal_runtimes, ws_idx, info.id)?;
    if runtime.synchronized_output_active() {
        return None;
    }
    let scrolled_back = super::panes::pane_is_scrolled_back(runtime);
    let reveal = app.reveal_hidden_cursor_for_cjk_ime
        && (!app.cjk_ime_agent_filter_configured || {
            let detected = app
                .workspaces
                .get(ws_idx)
                .and_then(|ws| ws.terminal_id(info.id))
                .and_then(|terminal_id| app.terminals.get(terminal_id))
                .and_then(|terminal| terminal.detected_agent);
            detected.is_some_and(|agent| app.cjk_ime_agents.contains(&agent))
        });

    if let Some(cursor) = runtime.cursor_state(info.inner_rect, true) {
        let visible = if reveal {
            !scrolled_back
        } else {
            cursor.visible && !scrolled_back
        };
        Some(CursorState {
            x: cursor.x,
            y: cursor.y,
            visible,
            shape: if reveal && visible {
                app.cjk_ime_cursor_shape
            } else {
                cursor.shape
            },
        })
    } else if reveal && !scrolled_back {
        Some(CursorState {
            x: info.inner_rect.x,
            y: info.inner_rect.y,
            visible: true,
            shape: app.cjk_ime_cursor_shape,
        })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::Workspace;
    use ratatui::backend::TestBackend;
    use ratatui::layout::Direction;
    use ratatui::Terminal;

    #[tokio::test]
    async fn explicit_surface_layout_drives_render_cursor_and_hyperlinks() {
        let uri = "https://example.com/surface";
        let mut workspace = Workspace::test_new("shell-workspace");
        let left = workspace.tabs[0].root_pane;
        let right = workspace.test_split(Direction::Horizontal);
        workspace.insert_test_runtime(
            left,
            crate::terminal::TerminalRuntime::test_with_screen_bytes(
                20,
                8,
                format!("\x1b]8;;{uri}\x1b\\LEFT\x1b]8;;\x1b\\").as_bytes(),
            ),
        );
        workspace.insert_test_runtime(
            right,
            crate::terminal::TerminalRuntime::test_with_screen_bytes(20, 8, b"RIGHT"),
        );

        let mut app = AppState::test_new();
        app.workspaces = vec![workspace];
        app.active = Some(0);
        app.selected = 0;
        app.mode = Mode::Terminal;

        let full_area = Rect::new(0, 0, 106, 20);
        crate::ui::compute_view(&mut app, full_area);
        let area = app.view.terminal_area;
        assert_eq!(area, Rect::new(26, 1, 80, 19));
        let surface = compute_tab_surface(
            &app,
            &TerminalRuntimeRegistry::new(),
            area,
            false,
            crate::kitty_graphics::HostCellSize::default(),
        );
        assert_eq!(surface.pane_infos.len(), 2);
        assert!(!surface.split_borders.is_empty());

        app.view.terminal_area = Rect::new(9, 8, 7, 6);
        app.view.pane_infos.clear();
        app.view.split_borders.clear();

        let surface_view = TabSurfaceView {
            pane_infos: &surface.pane_infos,
            split_borders: &surface.split_borders,
        };
        let mut terminal =
            Terminal::new(TestBackend::new(full_area.width, full_area.height)).unwrap();
        terminal
            .draw(|frame| {
                render_tab_surface(&app, &TerminalRuntimeRegistry::new(), surface_view, frame)
            })
            .unwrap();

        let rendered = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(rendered.contains("LEFT"), "surface: {rendered:?}");
        assert!(rendered.contains("RIGHT"), "surface: {rendered:?}");
        assert!(!rendered.contains("shell-workspace"));

        let links = tab_surface_hyperlinks(&app, &TerminalRuntimeRegistry::new(), surface_view);
        assert!(links
            .iter()
            .any(|(_, symbol, link)| { symbol == "L" && link == uri }));
        assert!(tab_surface_cursor(&app, &TerminalRuntimeRegistry::new(), surface_view,).is_some());
    }

    fn full_app_frame(app: &mut AppState, area: Rect) -> crate::protocol::FrameData {
        let (buffer, cursor) = crate::server::render_stream::render_virtual(app, area, true);
        let hyperlinks =
            crate::server::render_stream::visible_hyperlinks(app, &TerminalRuntimeRegistry::new());
        crate::protocol::FrameData::from_ratatui_buffer_with_hyperlinks(
            &buffer,
            cursor,
            &hyperlinks,
        )
    }

    fn frame_digest(frame: &crate::protocol::FrameData) -> String {
        use sha2::{Digest, Sha256};

        let encoded = bincode::serde::encode_to_vec(frame, bincode::config::standard()).unwrap();
        format!("{:x}", Sha256::digest(encoded))
    }

    fn full_app_characterization_state(uri: &str) -> AppState {
        let mut workspace = Workspace::test_new("characterization");
        workspace.identity_cwd = std::path::PathBuf::from("characterization");
        workspace.cached_git_branch = None;
        workspace.cached_git_ahead_behind = None;
        workspace.cached_git_space = None;
        workspace.test_add_tab(Some("logs"));
        workspace.switch_tab(0);
        let left = workspace.tabs[0].root_pane;
        let right = workspace.test_split(Direction::Horizontal);
        workspace.insert_test_runtime(
            left,
            crate::terminal::TerminalRuntime::test_with_screen_bytes(
                40,
                10,
                format!("\x1b]8;;{uri}\x1b\\LINK\x1b]8;;\x1b\\").as_bytes(),
            ),
        );
        workspace.insert_test_runtime(
            right,
            crate::terminal::TerminalRuntime::test_with_screen_bytes(40, 10, b"RIGHT\r\nPANE"),
        );

        let mut app = AppState::test_new();
        app.workspaces = vec![workspace];
        app.active = Some(0);
        app.selected = 0;
        app.mode = Mode::Terminal;
        app
    }

    #[tokio::test]
    async fn desktop_full_app_semantic_frame_is_characterized() {
        let uri = "https://example.com/full-app";
        let mut app = full_app_characterization_state(uri);
        let frame = full_app_frame(&mut app, Rect::new(0, 0, 106, 20));

        assert_eq!((frame.width, frame.height), (106, 20));
        assert_eq!(app.view.sidebar_rect, Rect::new(0, 0, 26, 20));
        assert_eq!(app.view.tab_bar_rect, Rect::new(26, 0, 80, 1));
        assert_eq!(app.view.terminal_area, Rect::new(26, 1, 80, 19));
        assert_eq!(app.view.pane_infos.len(), 2);
        assert!(!app.view.split_borders.is_empty());
        assert!(frame.cursor.is_some());
        assert_eq!(frame.hyperlinks, vec![uri.to_owned()]);
        // Re-baselined for this fork: every structural assertion above still
        // holds (same sidebar/tab-bar/terminal rects, same pane and split
        // counts, same cursor and hyperlink), but the sidebar paints the
        // fork's Spaces/Projects/Files header tabs and its own section rows,
        // so the byte-level digest necessarily differs from upstream's.
        //
        // Re-baselined again 2026-07-30: every workspace row now carries a
        // trailing "+" that starts a chat there (TP-WSCHAT-23). The structural
        // assertions above are unchanged — this digest tracks pixels, and one
        // more painted glyph per row is exactly the kind of change it is meant
        // to surface rather than hide.
        //
        // Re-baselined a third time, same day: the Spaces tab became a
        // three-level tree (TP-TREE-08..13). Rows are indented by depth, the
        // active row is filled with the accent, and drawers hang off a rule.
        // Every structural assertion above still holds — the rects, the pane
        // and split counts, the cursor and the hyperlink are untouched.
        //
        // Re-baselined 2026-08-09 (twice): checkout rows carry the branch glyph,
        // now the powerline git-branch (U+E0A0) after the keyboard-symbol ⎇
        // failed the eye test
        // (TP-ICON-01), one more painted glyph per row — again exactly the
        // change this digest exists to surface. The structural assertions
        // above are untouched.
        //
        // Re-baselined 2026-08-11 (twice): every workspace row now carries a
        // "⋯" left of the "+" (TP-DOTS-03), the visible door to the row
        // menu — then a breathing cell went between the two glyphs. The
        // structural assertions above are untouched both times.
        //
        // Re-baselined 2026-08-12: the Spaces footer gained its "focus"
        // toggle (TP-FOCUS-SW-04), the switch that narrows the tree to what
        // is being worked in. One more painted control in a footer whose
        // rects are unchanged — again the kind of change this digest exists
        // to surface rather than hide, and again every structural assertion
        // above still holds.
        //
        // Re-baselined 2026-08-20: the tab strip seats two split buttons
        // right of "+" (TP-TAB-SPLIT-01) — two more painted glyphs on the
        // strip, rects and structure untouched. Exactly the change this
        // digest exists to surface.
        //
        // Re-baselined 2026-08-21: the split pair moved from the "+"'s side
        // to a pin at the strip's far right (TP-TAB-SPLIT-01 revised) — the
        // same two glyphs, new cells. Structure untouched.
        assert_eq!(
            frame_digest(&frame),
            "cc1a134554f919959ccda8f64ca99291e1eb3c540a2e08dd479594ab15e2bade"
        );
    }

    #[tokio::test]
    async fn mobile_full_app_semantic_frame_is_characterized() {
        let mut app = full_app_characterization_state("https://example.com/mobile");
        app.mode = Mode::Navigate;
        let frame = full_app_frame(&mut app, Rect::new(0, 0, 44, 20));

        assert_eq!((frame.width, frame.height), (44, 20));
        assert_eq!(app.view.layout, crate::app::state::ViewLayout::Mobile);
        assert_eq!(app.view.mobile_header_rect, Rect::new(0, 0, 44, 4));
        assert_eq!(app.view.terminal_area, Rect::new(0, 4, 44, 16));
        assert_eq!(frame.cursor, None);
        // Updated when the single mobile switcher became two drawers: the
        // header grew a button at each edge, navigate mode paints the spaces
        // drawer over three quarters of the width instead of a full-width
        // panel, and the drawer carries a keyboard cursor marker. The
        // structural assertions above are what hold the shape; this digest
        // holds everything else steady. It moved again when the drawer gained
        // its select-text row, again when the header buttons widened from
        // three columns to five (TP-MOB-58), and again when the create action
        // and select text moved into the pinned footer band (TP-MOB-76/77) —
        // verified by dumping the frame first: the list opens with the
        // workspace itself, a dim rule divides it from the band, and the last
        // two rows are "+ new workspace" and "select text [off]" at the
        // bottom of the panel. It moved once more when section titles gained
        // a breathing row above them (TP-MOB-81) — verified by dumping the
        // frame first: a blank row now sits between the workspace's detail
        // line and the "menu" heading. It moved again when workspace rows
        // grew their chat-disclosure head and their trailing `+` (TP-MOB-84)
        // — verified by dumping the frame: the row reads "▸ · name … +". It
        // moved again when every tappable entry grew to the three-row touch
        // height (TP-MOB-87) — verified by dumping the frame: the workspace
        // spans rows 3-5 (title, detail, breath) and each menu item spans
        // three rows of its own, with the pinned band unchanged at the
        // bottom. It moved once more when the band itself became touch-sized
        // (TP-MOB-88) — verified by dumping the frame: the create action
        // spans four rows with its label one row in, select text spans
        // three, and the panel's final row is an empty guard. It moved
        // once more when the header grew to the four-row touch height with
        // nine-column buttons (TP-MOB-89) — verified by dumping the frame:
        // the strip's two lines sit centred on header rows 1-2, each menu
        // glyph on row 2, and the drawer panel starts on row 4. It moved once
        // more when readable dim text stepped up from overlay0 to overlay1
        // (TP-MOB-90) — a colour-only delta over the same layout, so the
        // frame text is unchanged from the previous pin. It moved once more
        // when the drawer's title row became the two-row segment band
        // (TP-MOB-91) — verified by dumping the frame: "spaces" and
        // "projects" split the band's width on panel rows 0-1 and the list
        // starts one row lower. It moved once more when the drawer
        // learned to draw its structure (TP-MOB-92) — verified by dumping
        // the frame: the create action sits in a rounded accent pill and
        // the active segment is a tab-shaped box open toward its list. It
        // moved once more when the band grew its third zone (TP-MOB-93) —
        // verified by dumping the band rows: spaces boxed, projects and
        // files as words in their thirds. It moved once more when
        // workspace rows grew their menu dots (TP-MOB-94) — verified by
        // dumping the row: it reads "▸ · name … ⋯ +". And again on
        // 2026-08-09 when the drawer's branch rows took the branch glyph
        // and its chat rows the chat glyph (TP-MOB-99), the phone half of
        // the desktop's kind icons — verified by the glyph parity test.
        assert_eq!(
            frame_digest(&frame),
            "44d55581cf10e3336420d8bfe268bb58d2c0a2fbf6d73bc8f525ff32239e5583"
        );
    }
}
