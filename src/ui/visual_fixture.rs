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
