//! Native AppDock presentation: a bounded icon-only vertical dock hosting the
//! built-in Terminal and Files entries. SF5.1 owns the pure model, geometry,
//! and render; interaction and the anchored name popover arrive with SF5.2.
//!
//! Size policy is frozen by the typed shell template track
//! (`template::dock_track()`): preferred 5 cells, min 3, max 9. Entry names
//! are available to accessible text/popover models only — the dock itself
//! renders icons, never permanent labels.

use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Span;
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::state::AppState;
use crate::ui::surface_host::{BuiltInAppId, StageSurfaceView};

/// One dock entry projected from typed Stage and domain state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AppDockEntry {
    pub app: BuiltInAppId,
    /// The entry owns the active Stage surface right now.
    pub active: bool,
    /// The entry has live domain state behind it (running terminal
    /// workspaces / an open Files surface) even while not active.
    pub running: bool,
    /// A disabled entry keeps its target visible but consumes activation
    /// fail-closed. Every built-in entry is enabled in v0.
    pub enabled: bool,
}

impl AppDockEntry {
    /// Single-cell icon for the entry. `ascii` selects the ASCII-safe
    /// fallback used by tests and capability-limited terminals.
    pub(crate) fn icon(&self, ascii: bool) -> &'static str {
        match (self.app, ascii) {
            (BuiltInAppId::Terminal, false) => "❯",
            (BuiltInAppId::Terminal, true) => ">",
            (BuiltInAppId::Files, false) => "▤",
            (BuiltInAppId::Files, true) => "#",
        }
    }

    /// Accessible name for popover/text models; never rendered in the dock.
    #[allow(dead_code)] // SF5.2 popover model consumes this.
    pub(crate) fn name(&self) -> &'static str {
        match self.app {
            BuiltInAppId::Terminal => "Terminal",
            BuiltInAppId::Files => "Files",
        }
    }
}

/// Pure bounded projection of the dock content: exactly the built-in
/// Terminal and Files entries, in stable order.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct AppDockModel {
    pub entries: Vec<AppDockEntry>,
}

impl AppDockModel {
    pub(crate) fn for_state(state: &AppState) -> Self {
        let surface = state.stage.surface_view();
        Self {
            entries: vec![
                AppDockEntry {
                    app: BuiltInAppId::Terminal,
                    active: surface == StageSurfaceView::TerminalWorkspace,
                    running: !state.workspaces.is_empty(),
                    enabled: true,
                },
                AppDockEntry {
                    app: BuiltInAppId::Files,
                    active: surface == StageSurfaceView::NativeFiles,
                    running: state.file_manager.is_some(),
                    enabled: true,
                },
            ],
        }
    }
}

/// One complete dock hit rectangle for a specific entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AppDockEntryArea {
    pub app: BuiltInAppId,
    pub rect: Rect,
    /// Copied from the entry so input can consume a disabled target
    /// fail-closed without re-deriving the model.
    pub enabled: bool,
}

/// Complete, disjoint, in-order entry rectangles inside the dock region:
/// one full-width single-row target per model entry, stacked from the top,
/// clipped to the rows the region actually has. Degenerate geometry
/// produces no target at all.
pub(crate) fn app_dock_entry_areas(model: &AppDockModel, area: Rect) -> Vec<AppDockEntryArea> {
    if area.width == 0 || area.height == 0 {
        return Vec::new();
    }
    model
        .entries
        .iter()
        .take(usize::from(area.height))
        .enumerate()
        .map(|(index, entry)| AppDockEntryArea {
            app: entry.app,
            rect: Rect::new(area.x, area.y + index as u16, area.width, 1),
            enabled: entry.enabled,
        })
        .collect()
}

/// Pure dock render: reads the prepared model and the palette, draws each
/// entry's single-cell icon centered in its target row, an ownership bar on
/// the active entry, and dims idle entries. Mutates nothing and performs no
/// I/O.
pub(crate) fn render_app_dock(app: &AppState, model: &AppDockModel, frame: &mut Frame, area: Rect) {
    let palette = &app.palette;
    for target in app_dock_entry_areas(model, area) {
        let Some(entry) = model.entries.iter().find(|entry| entry.app == target.app) else {
            continue;
        };
        let icon_style = if entry.active {
            Style::default().fg(palette.accent)
        } else if entry.running {
            Style::default().fg(palette.text)
        } else {
            Style::default().fg(palette.overlay0)
        };
        let bar = if entry.active {
            Span::styled("▎", Style::default().fg(palette.accent))
        } else {
            Span::raw(" ")
        };
        // Center the single-cell icon in the remaining width after the
        // one-cell ownership bar column.
        let icon_col = 1 + target.rect.width.saturating_sub(1) / 2;
        let padding = usize::from(icon_col.saturating_sub(1));
        let line = ratatui::text::Line::from(vec![
            bar,
            Span::raw(" ".repeat(padding)),
            Span::styled(entry.icon(false), icon_style),
        ]);
        frame.render_widget(Paragraph::new(line), target.rect);
    }
}

#[cfg(test)]
mod tests {
    use ratatui::{backend::TestBackend, Terminal};

    use super::*;
    use crate::ui::shell::ShellTemplateId;
    use crate::workspace::Workspace;

    fn dock_state(open_files: bool) -> AppState {
        let mut state = AppState::test_new();
        state.workspaces = vec![Workspace::test_new("dock")];
        state.active = Some(0);
        state.selected = 0;
        if open_files {
            let root = std::env::temp_dir();
            state
                .try_open_file_manager_with(|_| Some(crate::fm::FmState::new(&root)))
                .expect("Files activation");
        }
        state
    }

    fn dock_buffer(state: &AppState, area: Rect) -> ratatui::buffer::Buffer {
        let model = AppDockModel::for_state(state);
        let mut terminal = Terminal::new(TestBackend::new(area.width.max(1), area.height.max(1)))
            .expect("dock test terminal");
        terminal
            .draw(|frame| render_app_dock(state, &model, frame, area))
            .expect("dock render");
        terminal.backend().buffer().clone()
    }

    fn buffer_text(buffer: &ratatui::buffer::Buffer, area: Rect) -> String {
        (area.y..area.bottom())
            .flat_map(|y| (area.x..area.right()).map(move |x| (x, y)))
            .map(|(x, y)| buffer[(x, y)].symbol())
            .collect()
    }

    // SF5.1: the frozen typed template track supplies the size policy —
    // preferred five cells at comfortable widths, bounded 3..=9.
    #[test]
    fn app_dock_defaults_to_five_cells() {
        let layout = ShellTemplateId::DockStage
            .validated_layout()
            .expect("built-in template validates");
        let layout = layout.as_layout();
        let regions = layout.compute_regions(Rect::new(0, 0, 80, 24), |_| 0);
        assert_eq!(regions.get(crate::ui::shell::RegionId::AppDock).width, 5);
        assert_eq!(
            regions
                .get(crate::ui::shell::RegionId::WorkspaceStage)
                .width,
            75
        );
    }

    // SF5.1: the dock is icon-only. Both built-in entries render exactly one
    // single-cell icon each and never a permanent name label.
    #[test]
    fn app_dock_renders_icon_only_terminal_and_files() {
        let state = dock_state(false);
        let model = AppDockModel::for_state(&state);
        assert_eq!(
            model
                .entries
                .iter()
                .map(|entry| entry.app)
                .collect::<Vec<_>>(),
            vec![BuiltInAppId::Terminal, BuiltInAppId::Files],
            "the dock projects exactly Terminal and Files, in stable order"
        );
        for entry in &model.entries {
            for ascii in [false, true] {
                assert_eq!(
                    unicode_width::UnicodeWidthStr::width(entry.icon(ascii)),
                    1,
                    "every dock icon must occupy exactly one cell"
                );
            }
        }

        let area = Rect::new(0, 0, 5, 8);
        let text = buffer_text(&dock_buffer(&state, area), area);
        for entry in &model.entries {
            assert!(
                text.contains(entry.icon(false)) || text.contains(entry.icon(true)),
                "the dock must render the {:?} icon",
                entry.app
            );
            assert!(
                !text.contains(entry.name()),
                "the dock must never render the permanent name {:?}",
                entry.name()
            );
        }
    }

    // SF5.1: active ownership and running state are visibly distinct.
    #[test]
    fn app_dock_marks_active_and_running_states() {
        let state = dock_state(true);
        let model = AppDockModel::for_state(&state);
        let files = model
            .entries
            .iter()
            .find(|entry| entry.app == BuiltInAppId::Files)
            .expect("files entry");
        let terminal_entry = model
            .entries
            .iter()
            .find(|entry| entry.app == BuiltInAppId::Terminal)
            .expect("terminal entry");
        assert!(files.active, "open Files owns the active surface");
        assert!(files.running);
        assert!(!terminal_entry.active);
        assert!(
            terminal_entry.running,
            "a live workspace keeps Terminal running while inactive"
        );

        let area = Rect::new(0, 0, 5, 8);
        let buffer = dock_buffer(&state, area);
        let areas = app_dock_entry_areas(&model, area);
        let rect_of = |app: BuiltInAppId| {
            areas
                .iter()
                .find(|entry| entry.app == app)
                .expect("entry area")
                .rect
        };
        let files_rect = rect_of(BuiltInAppId::Files);
        assert_eq!(
            buffer[(files_rect.x, files_rect.y)].symbol(),
            "▎",
            "the active entry carries the left ownership bar"
        );
        let has_accent = (files_rect.x..files_rect.right())
            .any(|x| buffer[(x, files_rect.y)].fg == state.palette.accent);
        assert!(has_accent, "the active entry is accented");
        let terminal_rect = rect_of(BuiltInAppId::Terminal);
        assert_ne!(
            buffer[(terminal_rect.x, terminal_rect.y)].symbol(),
            "▎",
            "an inactive entry carries no ownership bar"
        );
        let terminal_uses_accent = (terminal_rect.x..terminal_rect.right())
            .any(|x| buffer[(x, terminal_rect.y)].fg == state.palette.accent);
        assert!(
            !terminal_uses_accent,
            "a running-but-inactive entry must not read as active"
        );
    }

    // SF5.1: entry hit rectangles are complete, disjoint, in stable order,
    // inside the dock, and identical across recomputation.
    #[test]
    fn app_dock_hits_are_complete_and_stable() {
        let state = dock_state(false);
        let model = AppDockModel::for_state(&state);
        let area = Rect::new(2, 3, 5, 8);
        let areas = app_dock_entry_areas(&model, area);
        assert_eq!(
            areas.iter().map(|entry| entry.app).collect::<Vec<_>>(),
            vec![BuiltInAppId::Terminal, BuiltInAppId::Files],
            "every model entry owns exactly one target, in order"
        );
        for entry in &areas {
            assert!(!entry.rect.is_empty());
            assert_eq!(entry.rect.intersection(area), entry.rect, "target in dock");
        }
        assert!(
            areas[0].rect.intersection(areas[1].rect).is_empty(),
            "targets are disjoint"
        );
        assert_eq!(
            areas,
            app_dock_entry_areas(&model, area),
            "recomputation is stable"
        );
    }

    // SF5.1: the minimum three-cell dock still exposes two distinct targets
    // and renders without panic.
    #[test]
    fn app_dock_narrow_mode_preserves_distinct_targets() {
        let state = dock_state(false);
        let model = AppDockModel::for_state(&state);
        let area = Rect::new(0, 0, 3, 8);
        let areas = app_dock_entry_areas(&model, area);
        assert_eq!(areas.len(), 2, "narrow mode keeps both targets");
        assert!(areas[0].rect.intersection(areas[1].rect).is_empty());
        let text = buffer_text(&dock_buffer(&state, area), area);
        assert!(!text.trim().is_empty(), "narrow dock still draws icons");
    }

    // SF5.1 characterization: the solver collapses the dock BEFORE starving
    // the stage (frozen degradation ladder).
    #[test]
    fn app_dock_collapses_before_stage() {
        let layout = ShellTemplateId::DockStage
            .validated_layout()
            .expect("built-in template validates");
        let layout = layout.as_layout();
        let narrow = Rect::new(0, 0, 3, 24);
        let regions = layout.compute_regions(narrow, |_| 0);
        assert_eq!(
            regions.get(crate::ui::shell::RegionId::AppDock).width,
            0,
            "below its minimum the dock collapses"
        );
        assert_eq!(
            regions
                .get(crate::ui::shell::RegionId::WorkspaceStage)
                .width,
            3,
            "the stage keeps the full remaining width"
        );
    }

    // SF5.1: degenerate zero-height geometry exposes no target and renders
    // as a panic-free no-op.
    #[test]
    fn zero_height_dock_is_panic_free_and_inert() {
        let state = dock_state(false);
        let model = AppDockModel::for_state(&state);
        for area in [Rect::new(0, 0, 5, 0), Rect::new(0, 0, 0, 8), Rect::ZERO] {
            assert!(
                app_dock_entry_areas(&model, area).is_empty(),
                "degenerate {area:?} must expose no target"
            );
            let _ = dock_buffer(&state, area);
        }
    }
}
