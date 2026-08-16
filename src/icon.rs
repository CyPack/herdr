//! Small pictures made of cells.
//!
//! # Why cells and not an image
//!
//! herdr's server→client path is a cell diff: `BlitEncoder` skips every cell
//! that did not change (`src/protocol/render_ansi.rs`). A bar icon is the most
//! static thing on the screen — it is drawn in every frame and changes almost
//! never — so as cells it costs a few hundred bytes once and **zero** on every
//! frame after that.
//!
//! The image path is a different pipe. `FrameData.graphics` carries Kitty
//! protocol bytes "to apply after the text frame", outside the diff, and the
//! frame ceiling rises from 2 MB to 32 MB when graphics are enabled precisely
//! because those payloads are large. Kitty graphics are also behind
//! `config.experimental` and off by default, and this build asks no terminal
//! whether it supports them. An icon drawn that way would be invisible for most
//! people and expensive for the rest.
//!
//! So: cells. The image path is deferred, not refused — it needs a capability
//! probe and region tracking first.
//!
//! # How two pixels fit in one cell
//!
//! `▀` (U+2580) paints the **top** half in the foreground colour and leaves the
//! bottom half showing the background, so one cell carries two vertically
//! stacked pixels, each with its own 24-bit colour. `▄` (U+2584) is the same
//! trick upside down, and it is what a cell whose top pixel is transparent
//! uses, so transparency never needs a fabricated background.
//!
//! Braille would give 2×4 pixels per cell but only one colour, which is no use
//! to a logo. Quadrants give 2×2 with two colours. Sextants need a font this
//! project cannot assume. Half blocks are the widest-supported thing that keeps
//! colour, which is why every protocol-less image viewer falls back to them.

use std::collections::BTreeMap;

/// One cell of art: the pixel above and the pixel below, each an index into the
/// art's own colour table, or `None` for transparent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct HalfCell {
    pub(crate) upper: Option<u8>,
    pub(crate) lower: Option<u8>,
}

/// A picture, in cells, with its colours still written the way the config wrote
/// them.
///
/// The colour specs stay unresolved on purpose. Bar colours are resolved against
/// the live palette at draw time, so switching theme recolours everything
/// without re-deriving any geometry; an icon that baked `Color` values at config
/// time would be the one surface that kept the old theme.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct IconArt {
    width: u16,
    /// Colour specs in the same grammar as `shell.bars.<edge>.color`:
    /// a palette token like `mauve`, or a literal like `#cba6f7`.
    specs: Vec<String>,
    /// Row-major, `width` per row.
    cells: Vec<HalfCell>,
}

impl IconArt {
    pub(crate) const fn width(&self) -> u16 {
        self.width
    }

    pub(crate) fn height(&self) -> u16 {
        if self.width == 0 {
            return 0;
        }
        u16::try_from(self.cells.len() / usize::from(self.width)).unwrap_or(u16::MAX)
    }

    pub(crate) fn spec(&self, index: u8) -> Option<&str> {
        self.specs.get(usize::from(index)).map(String::as_str)
    }

    /// The cell at (column, row), or `None` outside the art.
    pub(crate) fn cell(&self, x: u16, y: u16) -> Option<HalfCell> {
        if x >= self.width {
            return None;
        }
        let index = usize::from(y) * usize::from(self.width) + usize::from(x);
        self.cells.get(index).copied()
    }
}

/// What is wrong with a picture somebody wrote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum IconProblem {
    NoPixels,
    RaggedRow {
        row: usize,
        width: usize,
        first: usize,
    },
    UnknownKey {
        row: usize,
        column: usize,
        key: char,
    },
    MultiCharacterKey {
        key: String,
    },
    TooManyColours,
}

impl std::fmt::Display for IconProblem {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoPixels => write!(formatter, "has no pixel rows, so it would draw nothing"),
            Self::RaggedRow { row, width, first } => write!(
                formatter,
                "pixel row {row} is {width} wide but the first row is {first}; \
                 every row must be the same width"
            ),
            Self::UnknownKey { row, column, key } => write!(
                formatter,
                "pixel row {row} column {column} is {key:?}, which the palette does not name; \
                 use \".\" or a space for transparent"
            ),
            Self::MultiCharacterKey { key } => write!(
                formatter,
                "palette key {key:?} is longer than one character, so a pixel row could be \
                 read more than one way"
            ),
            Self::TooManyColours => write!(
                formatter,
                "uses more than 255 colours, which is far more than a bar icon can show"
            ),
        }
    }
}

/// Transparent pixels. A space is accepted alongside `.` because TOML strings
/// keep leading spaces and somebody drawing a shape will reach for one.
const TRANSPARENT: [char; 2] = ['.', ' '];

/// Turns pixel rows into cells, two pixel rows at a time.
///
/// An odd number of pixel rows is allowed and the last cell row's lower half
/// stays transparent. Filling it with anything — the first colour, the last row
/// repeated — draws a stripe under the picture that its author never wrote.
// TP-ART-04: pixels are read two rows at a time; an odd final row keeps a
// transparent lower half, and an unnamed key is refused.
pub(crate) fn art_from_pixels(
    pixels: &[String],
    palette: &BTreeMap<String, String>,
) -> Result<IconArt, IconProblem> {
    if pixels.is_empty() || pixels.iter().all(|row| row.is_empty()) {
        return Err(IconProblem::NoPixels);
    }

    // One character per key, so a pixel row has exactly one reading.
    let mut keys: BTreeMap<char, u8> = BTreeMap::new();
    let mut specs: Vec<String> = Vec::new();
    for (key, spec) in palette {
        let mut chars = key.chars();
        let (Some(first), None) = (chars.next(), chars.next()) else {
            return Err(IconProblem::MultiCharacterKey { key: key.clone() });
        };
        let index = u8::try_from(specs.len()).map_err(|_| IconProblem::TooManyColours)?;
        specs.push(spec.clone());
        keys.insert(first, index);
    }

    let rows: Vec<Vec<char>> = pixels.iter().map(|row| row.chars().collect()).collect();
    let first = rows[0].len();
    for (index, row) in rows.iter().enumerate() {
        if row.len() != first {
            return Err(IconProblem::RaggedRow {
                row: index,
                width: row.len(),
                first,
            });
        }
    }

    let width = u16::try_from(first).unwrap_or(u16::MAX);
    let mut cells = Vec::with_capacity(first * rows.len().div_ceil(2));

    for pair in rows.chunks(2) {
        for column in 0..first {
            let upper = pixel_at(&keys, pair.first(), column, 0)?;
            let lower = match pair.get(1) {
                // The row index only matters for the error message, and an odd
                // trailing row has no partner to name.
                Some(_) => pixel_at(&keys, pair.get(1), column, 1)?,
                None => None,
            };
            cells.push(HalfCell { upper, lower });
        }
    }

    Ok(IconArt {
        width,
        specs,
        cells,
    })
}

fn pixel_at(
    keys: &BTreeMap<char, u8>,
    row: Option<&Vec<char>>,
    column: usize,
    row_index: usize,
) -> Result<Option<u8>, IconProblem> {
    let Some(row) = row else {
        return Ok(None);
    };
    let Some(key) = row.get(column).copied() else {
        return Ok(None);
    };
    if TRANSPARENT.contains(&key) {
        return Ok(None);
    }
    keys.get(&key).copied().map(Some).ok_or({
        IconProblem::UnknownKey {
            row: row_index,
            column,
            key,
        }
    })
}

/// One bundled picture: what it is called, its pixel rows, and what its letters
/// stand for.
struct BuiltinArt {
    name: &'static str,
    rows: &'static [&'static str],
    palette: &'static [(&'static str, &'static str)],
}

/// The bundled pictures, in the order a refusal offers them.
///
/// A table rather than a match, for the same reason the section kinds became
/// one: a match can be asked whether it knows a name but never asked what names
/// it knows, so the refusal had no way to say what it would have accepted. That
/// left this the one closed set in the bar grammar that turned a config down
/// without telling anybody what to write instead.
const BUILTIN_ART: &[BuiltinArt] = &[
    // Agents converging into one runtime: two chevrons narrowing into a stem.
    // Ten pixels wide by six tall, which is ten cells by three rows — the
    // smallest size where the two halves still read as one mark.
    BuiltinArt {
        name: "herd",
        rows: &[
            "..a....a..",
            "...a..a...",
            "....aa....",
            "....bb....",
            "...bbbb...",
            "..bb..bb..",
        ],
        palette: &[("a", "mauve"), ("b", "teal")],
    },
    // A filled dot, for the one-cell-row case: two pixel rows, four wide. The
    // two lit columns sit in the middle and both their halves are set, so the
    // cell row paints `·██·` — one solid block with a cell of air on each side.
    // A terminal cell is about twice as tall as it is wide, which is what makes
    // two cells by one row read as round rather than as a bar.
    BuiltinArt {
        name: "dot",
        rows: &[".aa.", ".aa."],
        palette: &[("a", "accent")],
    },
];

/// The bundled pictures, by name.
///
/// Closed on purpose. A name that resolves to nothing draws an empty section,
/// which is indistinguishable from a section meant to be empty, so an unknown
/// name is refused where it is written rather than discovered on screen.
pub(crate) fn builtin(name: &str) -> Option<(Vec<String>, BTreeMap<String, String>)> {
    BUILTIN_ART.iter().find(|art| art.name == name).map(|art| {
        (
            art.rows.iter().map(|row| (*row).to_string()).collect(),
            art.palette
                .iter()
                .map(|(key, colour)| ((*key).to_string(), (*colour).to_string()))
                .collect(),
        )
    })
}

/// The names of the bundled pictures, in the order a refusal offers them.
pub(crate) fn builtin_names() -> Vec<&'static str> {
    BUILTIN_ART.iter().map(|art| art.name).collect()
}

/// Every bundled picture with the size it draws at, in cells.
///
/// A picture that cannot be read is left out rather than panicked over: the
/// table is compiled in, so an unreadable entry is a mistake made here and a
/// test says so, while a person running the CLI should not be the one who finds
/// out. `every_bundled_picture_is_one_that_can_be_drawn`, in `ui::shell::spec`,
/// is that test: it sits with the spec because the spec is what publishes this
/// catalogue, and an entry silently missing from it is the shape a reader is
/// handed.
pub(crate) fn builtin_catalogue() -> Vec<(&'static str, u16, u16)> {
    BUILTIN_ART
        .iter()
        .filter_map(|entry| {
            let rows = entry
                .rows
                .iter()
                .map(|row| (*row).to_string())
                .collect::<Vec<_>>();
            let palette = entry
                .palette
                .iter()
                .map(|(key, colour)| ((*key).to_string(), (*colour).to_string()))
                .collect::<BTreeMap<_, _>>();
            let art = art_from_pixels(&rows, &palette).ok()?;
            Some((entry.name, art.width(), art.height()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn palette() -> BTreeMap<String, String> {
        [
            ("a".to_string(), "mauve".to_string()),
            ("b".to_string(), "teal".to_string()),
        ]
        .into_iter()
        .collect()
    }

    #[test]
    fn two_pixel_rows_become_one_cell_row_with_the_upper_pixel_first() {
        let pixels = vec!["ab".to_string(), "ba".to_string()];
        let art = art_from_pixels(&pixels, &palette()).expect("a square of four pixels parses");

        assert_eq!(art.width(), 2);
        assert_eq!(art.height(), 1, "two pixel rows are one cell row");

        // Column 0 is `a` over `b`; column 1 is `b` over `a`. Swapping upper
        // and lower would draw the picture upside down and look deliberate.
        let left = art.cell(0, 0).expect("cell exists");
        assert_eq!(art.spec(left.upper.expect("upper set")), Some("mauve"));
        assert_eq!(art.spec(left.lower.expect("lower set")), Some("teal"));
        let right = art.cell(1, 0).expect("cell exists");
        assert_eq!(art.spec(right.upper.expect("upper set")), Some("teal"));
        assert_eq!(art.spec(right.lower.expect("lower set")), Some("mauve"));
    }

    #[test]
    fn an_odd_number_of_pixel_rows_leaves_the_last_lower_half_transparent() {
        let pixels = vec!["a".to_string(), "b".to_string(), "a".to_string()];
        let art = art_from_pixels(&pixels, &palette()).expect("three rows parse");

        assert_eq!(art.height(), 2, "three pixel rows need two cell rows");
        let last = art.cell(0, 1).expect("second cell row exists");
        assert!(
            last.upper.is_some(),
            "the third pixel row is the upper half"
        );
        assert_eq!(
            last.lower, None,
            "an invented lower pixel would draw a stripe the author never wrote"
        );
    }

    #[test]
    fn a_dot_or_a_space_is_transparent_and_an_unnamed_key_is_refused() {
        let pixels = vec!["a. b".to_string(), "aaaa".to_string()];
        let art = art_from_pixels(&pixels, &palette()).expect("dots and spaces parse");
        assert_eq!(art.cell(1, 0).expect("cell").upper, None, "dot");
        assert_eq!(art.cell(2, 0).expect("cell").upper, None, "space");

        let typo = vec!["ax".to_string()];
        assert_eq!(
            art_from_pixels(&typo, &palette()),
            Err(IconProblem::UnknownKey {
                row: 0,
                column: 1,
                key: 'x'
            }),
            "an unnamed key looks exactly like a transparent one on screen"
        );
    }

    #[test]
    fn rows_of_different_widths_are_refused_rather_than_padded() {
        let ragged = vec!["aa".to_string(), "a".to_string()];
        assert_eq!(
            art_from_pixels(&ragged, &palette()),
            Err(IconProblem::RaggedRow {
                row: 1,
                width: 1,
                first: 2
            })
        );
    }

    #[test]
    fn a_picture_with_no_pixels_is_refused() {
        assert_eq!(art_from_pixels(&[], &palette()), Err(IconProblem::NoPixels));
        assert_eq!(
            art_from_pixels(&[String::new()], &palette()),
            Err(IconProblem::NoPixels)
        );
    }

    #[test]
    fn a_palette_key_longer_than_one_character_is_refused() {
        let wide: BTreeMap<String, String> = [("ab".to_string(), "mauve".to_string())]
            .into_iter()
            .collect();
        assert_eq!(
            art_from_pixels(&["ab".to_string()], &wide),
            Err(IconProblem::MultiCharacterKey {
                key: "ab".to_string()
            }),
            "a two-character key makes a pixel row readable two ways"
        );
    }

    /// A bundled picture as the characters it will paint, one string per cell
    /// row: `█` where both halves are set, `▀` upper only, `▄` lower only, `·`
    /// transparent.
    ///
    /// Written as a picture because that is the only form in which a wrong
    /// shape is obvious. Every other spelling of the same fact — a cell count,
    /// a pixel index, a width — is a number that looks exactly as plausible
    /// wrong as right, which is how `dot` came to draw a valley while its own
    /// description, and the shipped guide, both called it filled.
    fn drawn(name: &str) -> Vec<String> {
        let (pixels, palette) = builtin(name).unwrap_or_else(|| panic!("{name} is bundled"));
        let art = art_from_pixels(&pixels, &palette).expect("a bundled picture parses");
        (0..art.height())
            .map(|row| {
                (0..art.width())
                    .map(|column| match art.cell(column, row).unwrap_or_default() {
                        HalfCell {
                            upper: Some(_),
                            lower: Some(_),
                        } => '█',
                        HalfCell {
                            upper: Some(_),
                            lower: None,
                        } => '▀',
                        HalfCell {
                            upper: None,
                            lower: Some(_),
                        } => '▄',
                        HalfCell {
                            upper: None,
                            lower: None,
                        } => '·',
                    })
                    .collect()
            })
            .collect()
    }

    // TP-ART-08: a bundled picture is held to the cells it paints, not only to
    // the size it occupies.
    #[test]
    fn the_bundled_pictures_draw_the_shapes_their_descriptions_promise() {
        assert_eq!(
            drawn("dot"),
            vec!["·██·"],
            "a picture called `dot`, and documented as filled, has to be filled"
        );
        assert_eq!(
            drawn("herd"),
            vec!["··▀▄··▄▀··", "····██····", "··▄█▀▀█▄··"],
            "two marks converging into a stem that opens again"
        );
    }

    #[test]
    fn the_bundled_pictures_parse_and_report_the_size_they_will_occupy() {
        let (pixels, palette) = builtin("herd").expect("herd is bundled");
        let art = art_from_pixels(&pixels, &palette).expect("a bundled picture must parse");
        assert_eq!(art.width(), 10);
        assert_eq!(art.height(), 3, "six pixel rows are three cell rows");

        let (pixels, palette) = builtin("dot").expect("dot is bundled");
        let art = art_from_pixels(&pixels, &palette).expect("a bundled picture must parse");
        assert_eq!(art.width(), 4);
        assert_eq!(art.height(), 1);

        assert_eq!(builtin("nonexistent"), None);
    }
}
