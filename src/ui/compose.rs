//! Composition scaffolding — a client-only layer stack over ratatui's
//! immediate-mode rendering, modeled on helix's `Compositor` + `Component`.
//!
//! This is the additive foundation for flexible UI composition (named regions,
//! swappable component slots, pages, popups). Today's entire UI becomes layer 0,
//! rendered exactly as before; new composition capabilities are added as further
//! layers/regions on top without rewriting the base.
//!
//! Pure TUI presentation (AGENTS.md runtime/client boundary): none of these
//! types are shared runtime facts, and none appear in `protocol`/`api::schema`.
//! Every [`Component::render`] is pure — it reads state and draws, never mutates
//! it — matching herdr's `compute_view` (geometry+mutation) / `render` (pure
//! draw) split.

use ratatui::{layout::Rect, Frame};

use crate::app::state::AppState;
use crate::terminal::TerminalRuntimeRegistry;

/// Read-only context threaded to every component: the state to read plus the
/// live terminal runtimes needed to paint terminal panes.
pub(crate) struct RenderCtx<'a> {
    pub app: &'a AppState,
    pub terminals: &'a TerminalRuntimeRegistry,
}

/// One composable, pure-render layer (helix's `Component`).
///
/// Deliberately minimal for now — just `render`. The interactive surface
/// (`hit_areas`, `size_hint`, `handle_event`) joins the contract when the first
/// interactive/resizable component needs it, rather than being speculatively
/// added now (the gitui-minimal first step).
pub(crate) trait Component {
    /// Paint into `area`. Pure: reads `ctx`, never mutates state.
    fn render(&self, frame: &mut Frame, area: Rect, ctx: &RenderCtx);
}

/// A back-to-front stack of layers over immediate-mode rendering (helix's
/// `Compositor`). Layer 0 paints first (the base UI); later layers paint on top
/// (overlays/popups). Z-order is strictly paint order — ratatui has no z-index,
/// so a later `render` call overwrites earlier ones in overlapping cells.
pub(crate) struct Compositor {
    layers: Vec<Box<dyn Component>>,
}

impl Compositor {
    pub fn new(layers: Vec<Box<dyn Component>>) -> Self {
        Self { layers }
    }

    /// Render every layer, in order, into `area`.
    pub fn render(&self, frame: &mut Frame, area: Rect, ctx: &RenderCtx) {
        for layer in &self.layers {
            layer.render(frame, area, ctx);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Component, Compositor, RenderCtx};
    use crate::app::state::AppState;
    use crate::terminal::TerminalRuntimeRegistry;
    use ratatui::layout::Rect;
    use ratatui::widgets::Paragraph;
    use ratatui::{backend::TestBackend, Frame, Terminal};

    /// Fills its whole area with 'A'.
    struct FillA;
    impl Component for FillA {
        fn render(&self, frame: &mut Frame, area: Rect, _ctx: &RenderCtx) {
            frame.render_widget(Paragraph::new("AAAA"), area);
        }
    }

    /// Stamps 'BB' into the leftmost two cells of its area.
    struct StampB;
    impl Component for StampB {
        fn render(&self, frame: &mut Frame, area: Rect, _ctx: &RenderCtx) {
            let two = Rect::new(area.x, area.y, 2, area.height.min(1));
            frame.render_widget(Paragraph::new("BB"), two);
        }
    }

    // TP-S1.2: a later layer paints over an earlier one (back-to-front z-order).
    #[test]
    fn later_layer_paints_over_earlier() {
        let app = AppState::test_new();
        let terminals = TerminalRuntimeRegistry::new();
        let ctx = RenderCtx {
            app: &app,
            terminals: &terminals,
        };
        let compositor = Compositor::new(vec![
            Box::new(FillA) as Box<dyn Component>,
            Box::new(StampB),
        ]);

        let mut terminal = Terminal::new(TestBackend::new(4, 1)).unwrap();
        terminal
            .draw(|frame| compositor.render(frame, frame.area(), &ctx))
            .unwrap();
        let buf = terminal.backend().buffer();

        let row: String = (0..4)
            .map(|x| buf[(x, 0)].symbol().chars().next().unwrap_or(' '))
            .collect();
        // StampB (later) overwrites FillA in the first two cells; FillA shows
        // through in the remaining cells it painted first.
        assert_eq!(row, "BBAA");
    }

    // TP-S1.3: routing the real render through the Compositor stays deterministic
    // (pure render — the same state renders to identical buffers).
    #[test]
    fn routed_render_is_deterministic() {
        let mut app = AppState::test_new();
        app.workspaces = vec![crate::workspace::Workspace::test_new("one")];
        app.active = Some(0);
        app.selected = 0;
        app.mode = crate::app::state::Mode::Terminal;
        crate::ui::compute_view(&mut app, Rect::new(0, 0, 80, 24));

        let snapshot = || {
            let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
            terminal
                .draw(|frame| crate::ui::render(&app, frame))
                .unwrap();
            terminal.backend().buffer().clone()
        };
        assert_eq!(snapshot(), snapshot());
    }
}
