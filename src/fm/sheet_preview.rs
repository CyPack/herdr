//! Bounded spreadsheet preview preparation.
//!
//! Reads a workbook into a small, render-ready snapshot. Like the text and
//! image readers beside it, this runs in the bounded preview worker and never
//! on an input or render thread, and every failure is a typed value rather than
//! a panic — a spreadsheet is user-supplied input and its parser is new attack
//! surface.
//!
//! What this deliberately does not do: recalculate formulas. `calamine` yields
//! the value the writing application cached, which is what the user saw in
//! Excel. Writing an evaluator means a parser, an AST, a dependency graph,
//! cycle detection and a function library — a separate product, not a preview
//! (`docs/patterns/document-rendering.md` DR12/DA9).

use std::fmt;
use std::io;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};

use calamine::{open_workbook_auto, Data, Reader};
use unicode_width::UnicodeWidthStr;

/// A workbook larger than this is refused before any parsing starts.
///
/// The real hazard is not the file on disk but the cell grid it expands to:
/// `calamine` materialises a worksheet range densely, so a sheet declaring a
/// million rows costs a million rows of memory whatever the compressed size
/// was. The encoded ceiling is the cheap outer gate that keeps that arithmetic
/// bounded; the row and column ceilings below are the inner ones.
const DEFAULT_MAX_ENCODED_BYTES: u64 = 16 * 1024 * 1024;
const DEFAULT_MAX_ROWS: usize = 512;
const DEFAULT_MAX_COLUMNS: usize = 64;
const DEFAULT_MAX_CELL_CHARS: usize = 256;

/// Ceiling for one prepared column, in terminal cells.
///
/// A single long cell must not be able to push every other column off the
/// panel; past this width the text is the column's problem, not the layout's.
const MAX_COLUMN_WIDTH: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SheetPreviewLimits {
    pub(crate) max_encoded_bytes: u64,
    pub(crate) max_rows: usize,
    pub(crate) max_columns: usize,
    pub(crate) max_cell_chars: usize,
}

impl Default for SheetPreviewLimits {
    fn default() -> Self {
        Self {
            max_encoded_bytes: DEFAULT_MAX_ENCODED_BYTES,
            max_rows: DEFAULT_MAX_ROWS,
            max_columns: DEFAULT_MAX_COLUMNS,
            max_cell_chars: DEFAULT_MAX_CELL_CHARS,
        }
    }
}

/// One prepared cell: the text to draw, and whether it came from a number.
///
/// `numeric` exists for alignment — numbers read correctly only when they are
/// right-aligned in a column — and is decided here rather than in render, so
/// the decision travels with the data instead of being re-derived per frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SheetCell {
    pub text: String,
    pub numeric: bool,
}

/// A prepared column's display width, in terminal cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SheetColumn {
    pub width: usize,
}

/// A bounded window onto one worksheet, ready to draw.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SheetPreview {
    pub source_path: PathBuf,
    /// Every sheet in the workbook, in file order.
    ///
    /// Only `active` is materialised today, but the names cost one cheap call
    /// and they are the whole input to sheet switching. Reading them now means
    /// that feature adds a selector, not a second pass over the format layer.
    pub sheets: Vec<String>,
    /// Index into `sheets` of the worksheet the rows below came from.
    pub active: usize,
    pub columns: Vec<SheetColumn>,
    pub rows: Vec<Vec<SheetCell>>,
    /// The worksheet's real size, not the size of the window above.
    pub total_rows: u64,
    pub total_columns: u64,
    pub truncated_rows: bool,
    pub truncated_columns: bool,
}

/// Stable domain failures from bounded spreadsheet preparation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SheetPreviewError {
    Io(io::ErrorKind),
    NotRegularFile,
    TooLarge {
        actual: u64,
        limit: u64,
    },
    /// The file is not a workbook this reader understands, or it is damaged.
    Unreadable,
    /// A workbook with no worksheets at all.
    NoSheets,
    ReaderPanicked,
}

impl fmt::Display for SheetPreviewError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(kind) => write!(formatter, "spreadsheet preview I/O failed: {kind:?}"),
            Self::NotRegularFile => {
                formatter.write_str("spreadsheet preview source is not a regular file")
            }
            Self::TooLarge { actual, limit } => write!(
                formatter,
                "spreadsheet is too large ({actual} > {limit} bytes)"
            ),
            Self::Unreadable => formatter.write_str("spreadsheet could not be read"),
            Self::NoSheets => formatter.write_str("workbook contains no sheets"),
            Self::ReaderPanicked => formatter.write_str("spreadsheet reader panicked"),
        }
    }
}

impl std::error::Error for SheetPreviewError {}

/// Every workbook extension the preview reads, lowercase.
///
/// One table, for the same reason the image formats have one: the classifier
/// and the reader answering this question separately is how a file ends up
/// carrying a spreadsheet icon and then failing as binary text.
pub(crate) const READABLE_SHEET_EXTENSIONS: &[&str] =
    &["xlsx", "xlsm", "xlam", "xls", "xla", "xlsb", "ods"];

/// Does this path name a workbook the preview can read, judging by extension?
///
/// Extension only: this runs on the input path and must not read the file.
/// Content is authoritative later, inside the reader.
pub(crate) fn is_readable_sheet_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            READABLE_SHEET_EXTENSIONS
                .iter()
                .any(|candidate| extension.eq_ignore_ascii_case(candidate))
        })
}

pub(crate) fn read_sheet_preview(
    path: &Path,
    limits: SheetPreviewLimits,
) -> Result<SheetPreview, SheetPreviewError> {
    let metadata = std::fs::metadata(path).map_err(|error| SheetPreviewError::Io(error.kind()))?;
    if !metadata.is_file() {
        return Err(SheetPreviewError::NotRegularFile);
    }
    if metadata.len() > limits.max_encoded_bytes {
        return Err(SheetPreviewError::TooLarge {
            actual: metadata.len(),
            limit: limits.max_encoded_bytes,
        });
    }

    read_with_panic_boundary(|| read_workbook(path, limits))
}

fn read_workbook(
    path: &Path,
    limits: SheetPreviewLimits,
) -> Result<SheetPreview, SheetPreviewError> {
    let mut workbook = open_workbook_auto(path).map_err(|_| SheetPreviewError::Unreadable)?;
    let sheets = workbook.sheet_names();
    let Some(active_name) = sheets.first().cloned() else {
        return Err(SheetPreviewError::NoSheets);
    };
    let range = workbook
        .worksheet_range(&active_name)
        .map_err(|_| SheetPreviewError::Unreadable)?;

    let total_rows = range.height() as u64;
    let total_columns = range.width() as u64;
    let truncated_rows = range.height() > limits.max_rows;
    let truncated_columns = range.width() > limits.max_columns;

    let mut rows: Vec<Vec<SheetCell>> = Vec::new();
    for row in range.rows().take(limits.max_rows) {
        rows.push(
            row.iter()
                .take(limits.max_columns)
                .map(|cell| prepare_cell(cell, limits.max_cell_chars))
                .collect(),
        );
    }

    Ok(SheetPreview {
        source_path: path.to_path_buf(),
        sheets,
        active: 0,
        columns: measure_columns(&rows),
        rows,
        total_rows,
        total_columns,
        truncated_rows,
        truncated_columns,
    })
}

/// Width per column: the widest cell in it, capped.
///
/// Display width, not character count — a CJK glyph occupies two terminal
/// cells, and counting characters puts every following column one cell out of
/// line for the rest of the row.
fn measure_columns(rows: &[Vec<SheetCell>]) -> Vec<SheetColumn> {
    let column_count = rows.iter().map(Vec::len).max().unwrap_or(0);
    (0..column_count)
        .map(|index| SheetColumn {
            width: rows
                .iter()
                .filter_map(|row| row.get(index))
                .map(|cell| cell.text.width())
                .max()
                .unwrap_or(0)
                .min(MAX_COLUMN_WIDTH),
        })
        .collect()
}

fn prepare_cell(data: &Data, max_chars: usize) -> SheetCell {
    let numeric = matches!(data, Data::Int(_) | Data::Float(_));
    let text = match data {
        Data::Empty => String::new(),
        // The workbook formats these as words, and a reader comparing the
        // preview against Excel should see the same two words.
        Data::Bool(value) => if *value { "TRUE" } else { "FALSE" }.to_owned(),
        other => other.to_string(),
    };
    SheetCell {
        text: clamp_cell_text(text, max_chars),
        numeric,
    }
}

/// Reduce one cell to a single bounded line.
///
/// A cell may legitimately contain newlines and tabs. Drawn as-is they break
/// the row into pieces and every column after it loses its alignment, so
/// control characters collapse to spaces rather than being trusted.
fn clamp_cell_text(text: String, max_chars: usize) -> String {
    let mut flattened: String = text
        .chars()
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .take(max_chars)
        .collect();
    flattened.truncate(flattened.trim_end().len());
    flattened
}

fn read_with_panic_boundary<F>(read: F) -> Result<SheetPreview, SheetPreviewError>
where
    F: FnOnce() -> Result<SheetPreview, SheetPreviewError>,
{
    catch_unwind(AssertUnwindSafe(read)).map_err(|_| SheetPreviewError::ReaderPanicked)?
}

/// Real `.xlsx` bytes, generated rather than committed as a binary blob.
///
/// A workbook is a zip of XML parts, so building one here makes the row and
/// column counts parameters — which is what lets a test construct a sheet far
/// larger than any checked-in sample would be. Values are written as
/// `inlineStr`, so there is no shared-string table to keep consistent.
///
/// Lives outside the test module because the preview chain is tested from
/// several layers, and each of them needs a workbook that a real reader
/// accepts.
#[cfg(test)]
pub(crate) fn xlsx_fixture(sheets: &[(&str, &[&[&str]])]) -> Vec<u8> {
    use std::io::{Cursor, Write};

    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options: zip::write::FileOptions<'_, ()> =
        zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);

    let mut content_types = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>"#,
    );
    for index in 1..=sheets.len() {
        content_types.push_str(&format!(
            r#"<Override PartName="/xl/worksheets/sheet{index}.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>"#
        ));
    }
    content_types.push_str("</Types>");

    let mut sheet_entries = String::new();
    let mut sheet_rels = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#,
    );
    for (index, (name, _)) in sheets.iter().enumerate() {
        let number = index + 1;
        sheet_entries.push_str(&format!(
            r#"<sheet name="{name}" sheetId="{number}" r:id="rId{number}"/>"#
        ));
        sheet_rels.push_str(&format!(
            r#"<Relationship Id="rId{number}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet{number}.xml"/>"#
        ));
    }
    sheet_rels.push_str("</Relationships>");

    let workbook = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?><workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets>{sheet_entries}</sheets></workbook>"#
    );

    let parts: Vec<(String, String)> = vec![
        ("[Content_Types].xml".to_owned(), content_types),
        (
            "_rels/.rels".to_owned(),
            r#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#.to_owned(),
        ),
        ("xl/workbook.xml".to_owned(), workbook),
        ("xl/_rels/workbook.xml.rels".to_owned(), sheet_rels),
    ];
    for (name, body) in parts {
        writer.start_file(name, options).expect("start part");
        writer.write_all(body.as_bytes()).expect("write part");
    }

    for (index, (_, rows)) in sheets.iter().enumerate() {
        writer
            .start_file(format!("xl/worksheets/sheet{}.xml", index + 1), options)
            .expect("start sheet");
        writer
            .write_all(worksheet_fixture_xml(rows).as_bytes())
            .expect("write sheet");
    }

    writer.finish().expect("finish zip").into_inner()
}

/// One worksheet part.
///
/// A value prefixed with `=` becomes a formula cell carrying both the formula
/// and the value the writing application cached, which is how a real workbook
/// stores one — and the only way to test that the preview shows the cached
/// value rather than the expression.
#[cfg(test)]
fn worksheet_fixture_xml(rows: &[&[&str]]) -> String {
    let mut body = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData>"#,
    );
    for (row_index, row) in rows.iter().enumerate() {
        let row_number = row_index + 1;
        body.push_str(&format!(r#"<row r="{row_number}">"#));
        for (column_index, value) in row.iter().enumerate() {
            let reference = format!("{}{row_number}", fixture_column_name(column_index));
            if value.is_empty() {
                continue;
            }
            if let Some((formula, cached)) = value.strip_prefix('=').and_then(|rest| {
                rest.split_once('#')
                    .map(|(formula, cached)| (formula.to_owned(), cached.to_owned()))
            }) {
                body.push_str(&format!(
                    r#"<c r="{reference}"><f>{formula}</f><v>{cached}</v></c>"#
                ));
            } else if value.parse::<f64>().is_ok() {
                body.push_str(&format!(r#"<c r="{reference}"><v>{value}</v></c>"#));
            } else {
                let escaped = value.replace('&', "&amp;").replace('<', "&lt;");
                body.push_str(&format!(
                    r#"<c r="{reference}" t="inlineStr"><is><t>{escaped}</t></is></c>"#
                ));
            }
        }
        body.push_str("</row>");
    }
    body.push_str("</sheetData></worksheet>");
    body
}

#[cfg(test)]
fn fixture_column_name(mut index: usize) -> String {
    let mut name = Vec::new();
    loop {
        name.push(b'A' + (index % 26) as u8);
        if index < 26 {
            break;
        }
        index = index / 26 - 1;
    }
    name.reverse();
    String::from_utf8(name).expect("column name is ASCII")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn unique() -> u64 {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        NEXT.fetch_add(1, Ordering::Relaxed)
    }

    struct TempDir {
        root: PathBuf,
    }

    impl TempDir {
        fn new(label: &str) -> Self {
            let root = std::env::temp_dir().join(format!(
                "herdr-sheet-{label}-{}-{}",
                std::process::id(),
                unique()
            ));
            std::fs::create_dir_all(&root).expect("create temp dir");
            Self { root }
        }

        fn write(&self, name: &str, bytes: &[u8]) -> PathBuf {
            let path = self.root.join(name);
            std::fs::write(&path, bytes).expect("write fixture");
            path
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }
    use super::xlsx_fixture as xlsx;

    // TP-FSH-03: the reader returns the workbook's actual values and sheet names.
    #[test]
    fn reads_cell_values_and_sheet_names() {
        let td = TempDir::new("values");
        let path = td.write(
            "book.xlsx",
            &xlsx(&[("Budget", &[&["Item", "Cost"], &["cable", "42"]])]),
        );

        let preview = read_sheet_preview(&path, SheetPreviewLimits::default())
            .expect("a valid workbook is readable");

        assert_eq!(preview.sheets, vec!["Budget".to_owned()]);
        assert_eq!(preview.active, 0);
        assert_eq!(preview.rows.len(), 2);
        assert_eq!(preview.rows[0][0].text, "Item");
        assert_eq!(preview.rows[1][0].text, "cable");
        assert_eq!(preview.rows[1][1].text, "42");
        assert!(
            preview.rows[1][1].numeric,
            "a number must be marked numeric so render can right-align it"
        );
        assert!(!preview.rows[1][0].numeric);
        assert!(!preview.truncated_rows);
        assert!(!preview.truncated_columns);
    }

    // TP-FSH-08: a formula cell shows the value the writing application cached,
    // not the expression. herdr does not evaluate formulas and must not look
    // like it tried and failed — showing `=B2*C2` where Excel showed `84` would
    // read as a broken preview rather than a deliberate limit.
    #[test]
    fn formula_cells_show_the_cached_value_not_the_expression() {
        let td = TempDir::new("formula");
        let path = td.write(
            "totals.xlsx",
            &xlsx(&[("Sheet1", &[&["qty", "price", "=B1*A1#84"]])]),
        );

        let preview = read_sheet_preview(&path, SheetPreviewLimits::default()).expect("workbook");

        let cell = &preview.rows[0][2];
        assert_eq!(cell.text, "84", "the cached value is what Excel displayed");
        assert!(
            !cell.text.contains('='),
            "the formula expression must not reach the panel"
        );
        assert!(cell.numeric, "a cached number aligns like any other number");
    }

    // TP-FSH-04: every sheet name is read, not just the one materialised. Sheet
    // switching consumes this list, so it must be right before that feature
    // exists rather than after it.
    #[test]
    fn reads_every_sheet_name_while_materialising_only_the_first() {
        let td = TempDir::new("multi");
        let path = td.write(
            "multi.xlsx",
            &xlsx(&[
                ("Summary", &[&["first"]]),
                ("Detail", &[&["second"]]),
                ("Notes", &[&["third"]]),
            ]),
        );

        let preview =
            read_sheet_preview(&path, SheetPreviewLimits::default()).expect("multi-sheet workbook");

        assert_eq!(preview.sheets, vec!["Summary", "Detail", "Notes"]);
        assert_eq!(preview.active, 0);
        assert_eq!(preview.rows[0][0].text, "first");
    }

    // TP-FSH-05: a sheet larger than the window is truncated, and reports its real
    // size. Materialising every row of a large workbook is the OOM path (DA8).
    #[test]
    fn large_sheet_is_windowed_and_reports_its_real_size() {
        let td = TempDir::new("large");
        let owned: Vec<Vec<String>> = (0..40)
            .map(|row| (0..12).map(|col| format!("r{row}c{col}")).collect())
            .collect();
        let borrowed: Vec<Vec<&str>> = owned
            .iter()
            .map(|row| row.iter().map(String::as_str).collect())
            .collect();
        let rows: Vec<&[&str]> = borrowed.iter().map(Vec::as_slice).collect();
        let path = td.write("large.xlsx", &xlsx(&[("Big", &rows)]));

        let limits = SheetPreviewLimits {
            max_rows: 5,
            max_columns: 3,
            ..SheetPreviewLimits::default()
        };
        let preview = read_sheet_preview(&path, limits).expect("large workbook");

        assert_eq!(preview.rows.len(), 5, "the window is the row ceiling");
        assert!(preview.rows.iter().all(|row| row.len() <= 3));
        assert_eq!(preview.total_rows, 40, "the real height is still reported");
        assert_eq!(preview.total_columns, 12);
        assert!(preview.truncated_rows);
        assert!(preview.truncated_columns);
    }

    // TP-FSH-06: damaged input and a file whose extension lies are typed
    // failures. A new parser is new panic surface, and the preview must not be
    // the place a malformed download takes the process down.
    #[test]
    fn damaged_and_misnamed_workbooks_fail_without_panicking() {
        let td = TempDir::new("damaged");

        let truncated = {
            let mut bytes = xlsx(&[("Sheet1", &[&["a"]])]);
            bytes.truncate(bytes.len() / 2);
            td.write("truncated.xlsx", &bytes)
        };
        assert_eq!(
            read_sheet_preview(&truncated, SheetPreviewLimits::default()),
            Err(SheetPreviewError::Unreadable)
        );

        let lying = td.write("notreally.xlsx", b"this is plain text, not a workbook\n");
        assert_eq!(
            read_sheet_preview(&lying, SheetPreviewLimits::default()),
            Err(SheetPreviewError::Unreadable),
            "content is authoritative; the extension is only a hint"
        );

        let empty = td.write("empty.xlsx", b"");
        assert_eq!(
            read_sheet_preview(&empty, SheetPreviewLimits::default()),
            Err(SheetPreviewError::Unreadable)
        );
    }

    // TP-FSH-07: an empty sheet is a valid preview, not a failure. Reporting
    // "unavailable" for a workbook that simply has nothing in it would read as
    // a bug in herdr rather than a fact about the file.
    #[test]
    fn empty_sheet_is_a_valid_preview() {
        let td = TempDir::new("empty-sheet");
        let path = td.write("blank.xlsx", &xlsx(&[("Sheet1", &[])]));

        let preview = read_sheet_preview(&path, SheetPreviewLimits::default())
            .expect("an empty sheet is still a sheet");

        assert_eq!(preview.sheets, vec!["Sheet1".to_owned()]);
        assert!(preview.rows.is_empty());
        assert_eq!(preview.total_rows, 0);
        assert!(!preview.truncated_rows);
    }

    // TP-FSH-13: the outer size gate refuses the file before the parser sees it.
    #[test]
    fn oversized_workbook_is_refused_before_parsing() {
        let td = TempDir::new("oversized");
        let path = td.write("book.xlsx", &xlsx(&[("Sheet1", &[&["a"]])]));
        let actual = std::fs::metadata(&path).expect("fixture metadata").len();

        let limits = SheetPreviewLimits {
            max_encoded_bytes: 8,
            ..SheetPreviewLimits::default()
        };
        assert_eq!(
            read_sheet_preview(&path, limits),
            Err(SheetPreviewError::TooLarge { actual, limit: 8 })
        );
    }

    #[test]
    fn missing_and_non_file_paths_are_typed_failures() {
        let td = TempDir::new("missing");
        assert_eq!(
            read_sheet_preview(&td.root.join("nope.xlsx"), SheetPreviewLimits::default()),
            Err(SheetPreviewError::Io(io::ErrorKind::NotFound))
        );
        assert_eq!(
            read_sheet_preview(&td.root, SheetPreviewLimits::default()),
            Err(SheetPreviewError::NotRegularFile)
        );
    }

    // TP-FSH-09: a cell may carry newlines and tabs; drawn as-is they break the row apart
    // and every column after it loses alignment.
    #[test]
    fn control_characters_in_a_cell_collapse_to_one_line() {
        let td = TempDir::new("control");
        let path = td.write(
            "wrapped.xlsx",
            &xlsx(&[("Sheet1", &[&["line one\nline two\tend"]])]),
        );

        let preview = read_sheet_preview(&path, SheetPreviewLimits::default()).expect("workbook");

        let text = &preview.rows[0][0].text;
        assert!(
            !text.contains('\n') && !text.contains('\t'),
            "cell text must stay on one line, got {text:?}"
        );
        assert_eq!(text, "line one line two end");
    }

    #[test]
    fn long_cell_text_is_clamped_to_the_character_ceiling() {
        let td = TempDir::new("long-cell");
        let long = "x".repeat(400);
        let path = td.write("long.xlsx", &xlsx(&[("Sheet1", &[&[long.as_str()]])]));

        let limits = SheetPreviewLimits {
            max_cell_chars: 10,
            ..SheetPreviewLimits::default()
        };
        let preview = read_sheet_preview(&path, limits).expect("workbook");

        assert_eq!(preview.rows[0][0].text.chars().count(), 10);
    }

    // Display width, not character count: a CJK glyph is two terminal cells,
    // and counting characters puts every following column out of line.
    #[test]
    fn column_width_uses_display_width_and_is_capped() {
        let cells = |texts: &[&str]| -> Vec<SheetCell> {
            texts
                .iter()
                .map(|text| SheetCell {
                    text: (*text).to_owned(),
                    numeric: false,
                })
                .collect()
        };
        let rows = vec![cells(&["ab", "x"]), cells(&["日本語", "y"])];

        let columns = measure_columns(&rows);

        assert_eq!(columns[0].width, 6, "three CJK glyphs occupy six cells");
        assert_eq!(columns[1].width, 1);

        let wide = vec![cells(&["y".repeat(MAX_COLUMN_WIDTH + 40).as_str()])];
        assert_eq!(measure_columns(&wide)[0].width, MAX_COLUMN_WIDTH);
    }

    #[test]
    fn extension_classifier_matches_the_one_table() {
        for extension in READABLE_SHEET_EXTENSIONS {
            let path = PathBuf::from(format!("book.{extension}"));
            assert!(is_readable_sheet_path(&path), "{extension} must be routed");
            let upper = PathBuf::from(format!("book.{}", extension.to_uppercase()));
            assert!(
                is_readable_sheet_path(&upper),
                "{extension} must match case-insensitively"
            );
        }
        for other in ["txt", "png", "pdf", "docx", "csv"] {
            assert!(
                !is_readable_sheet_path(&PathBuf::from(format!("file.{other}"))),
                "{other} is not a workbook this reader claims"
            );
        }
    }
}
