use std::fmt;
use std::io::{self, Read};
use std::path::Path;
use std::sync::OnceLock;

use syntect::easy::HighlightLines;
use syntect::highlighting::{FontStyle, ThemeSet};
use syntect::parsing::{SyntaxReference, SyntaxSet};
use syntect::util::LinesWithEndings;

const MAX_TEXT_PREVIEW_BYTES: usize = 64 * 1024;
const UTF8_SENTINEL_BYTES: usize = 4;
const MAX_HIGHLIGHTED_LINES: usize = 128;
const PREVIEW_THEME: &str = "base16-ocean.dark";

/// Byte budget for one prepared text preview.
///
/// Requests are clamped to a hard ceiling so an accidental caller value cannot
/// turn a preview refresh into an unbounded allocation or read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct TextPreviewLimits {
    max_bytes: usize,
}

impl TextPreviewLimits {
    pub(super) fn new(max_bytes: usize) -> Self {
        Self {
            max_bytes: max_bytes.min(MAX_TEXT_PREVIEW_BYTES),
        }
    }
}

impl Default for TextPreviewLimits {
    fn default() -> Self {
        Self::new(MAX_TEXT_PREVIEW_BYTES)
    }
}

/// Prepared UTF-8 file content consumed by later state/render stages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextPreview {
    pub content: String,
    pub truncated: bool,
    pub highlighted: Option<HighlightedTextPreview>,
}

/// Terminal-independent style prepared from a syntax scope.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PreviewTextStyle {
    pub foreground: Option<[u8; 3]>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
}

impl PreviewTextStyle {
    pub fn is_plain(self) -> bool {
        self == Self::default()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewTextSpan {
    pub content: String,
    pub style: PreviewTextStyle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewTextLine {
    pub spans: Vec<PreviewTextSpan>,
}

/// Bounded, render-ready syntax output. The original [`TextPreview`] remains
/// the source of truth so a classifier/highlighter fallback cannot lose text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HighlightedTextPreview {
    pub lines: Vec<PreviewTextLine>,
    pub syntax_name: Option<String>,
    pub truncated_bytes: bool,
    pub truncated_lines: bool,
}

/// Stable domain failures from bounded text preparation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextPreviewError {
    Io(io::ErrorKind),
    NotRegularFile,
    Binary,
    InvalidUtf8 { valid_up_to: usize },
}

impl fmt::Display for TextPreviewError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(kind) => write!(formatter, "text preview I/O failed: {kind:?}"),
            Self::NotRegularFile => formatter.write_str("text preview source is not a file"),
            Self::Binary => formatter.write_str("text preview source is binary"),
            Self::InvalidUtf8 { valid_up_to } => {
                write!(formatter, "text preview is not UTF-8 at byte {valid_up_to}")
            }
        }
    }
}

impl std::error::Error for TextPreviewError {}

/// Read a bounded UTF-8 prefix without splitting a scalar at the byte budget.
///
/// The four-byte sentinel is the maximum UTF-8 scalar width. It lets the
/// boundary check distinguish a valid scalar crossing the display budget from
/// malformed input while keeping both allocation and I/O strictly bounded.
pub(super) fn read_text_preview(
    path: &Path,
    limits: TextPreviewLimits,
) -> Result<TextPreview, TextPreviewError> {
    let read_cap = limits.max_bytes.saturating_add(UTF8_SENTINEL_BYTES);
    let metadata = std::fs::metadata(path).map_err(|error| TextPreviewError::Io(error.kind()))?;
    if !metadata.is_file() {
        return Err(TextPreviewError::NotRegularFile);
    }
    let file = std::fs::File::open(path).map_err(|error| TextPreviewError::Io(error.kind()))?;
    let mut bytes = Vec::with_capacity(read_cap);
    file.take(read_cap as u64)
        .read_to_end(&mut bytes)
        .map_err(|error| TextPreviewError::Io(error.kind()))?;

    if bytes.contains(&0) {
        return Err(TextPreviewError::Binary);
    }

    let truncated = bytes.len() > limits.max_bytes;
    let retained = &bytes[..bytes.len().min(limits.max_bytes)];
    let content = match std::str::from_utf8(retained) {
        Ok(content) => content.to_owned(),
        Err(error) if truncated && error.error_len().is_none() => {
            let valid_up_to = error.valid_up_to();
            if !crossing_scalar_is_valid(&bytes, valid_up_to, limits.max_bytes) {
                return Err(TextPreviewError::InvalidUtf8 { valid_up_to });
            }
            // `valid_up_to` is supplied by `from_utf8`, so this prefix is
            // guaranteed valid without unchecked conversion.
            std::str::from_utf8(&bytes[..valid_up_to])
                .map_err(|error| TextPreviewError::InvalidUtf8 {
                    valid_up_to: error.valid_up_to(),
                })?
                .to_owned()
        }
        Err(error) => {
            return Err(TextPreviewError::InvalidUtf8 {
                valid_up_to: error.valid_up_to(),
            });
        }
    };

    Ok(TextPreview {
        content,
        truncated,
        highlighted: None,
    })
}

fn crossing_scalar_is_valid(bytes: &[u8], valid_up_to: usize, max_bytes: usize) -> bool {
    let first_end = max_bytes.saturating_add(1);
    let last_end = bytes
        .len()
        .min(max_bytes.saturating_add(UTF8_SENTINEL_BYTES));
    if first_end > last_end {
        return false;
    }

    (first_end..=last_end).any(|end| std::str::from_utf8(&bytes[valid_up_to..end]).is_ok())
}

struct HighlightAssets {
    syntaxes: SyntaxSet,
    themes: ThemeSet,
}

static HIGHLIGHT_ASSETS: OnceLock<HighlightAssets> = OnceLock::new();

/// Prepare a bounded syntax-highlighted view without filesystem or render I/O.
///
/// This function is intentionally synchronous and pure over prepared text; B1
/// runtime wiring executes it outside the input/render path and rejects stale
/// generations. Any classifier, theme, or parser failure falls back to plain
/// text so highlighting never becomes preview availability authority.
pub fn highlight_text_preview(path: &Path, preview: &TextPreview) -> HighlightedTextPreview {
    let assets = HIGHLIGHT_ASSETS.get_or_init(|| HighlightAssets {
        syntaxes: SyntaxSet::load_defaults_newlines(),
        themes: ThemeSet::load_defaults(),
    });
    let Some(syntax) = select_syntax(path, &preview.content, &assets.syntaxes) else {
        return plain_text_preview(preview);
    };
    let Some(theme) = assets
        .themes
        .themes
        .get(PREVIEW_THEME)
        .or_else(|| assets.themes.themes.values().next())
    else {
        return plain_text_preview(preview);
    };

    let mut highlighter = HighlightLines::new(syntax, theme);
    let mut source_lines = LinesWithEndings::from(&preview.content);
    let mut lines = Vec::new();
    for _ in 0..MAX_HIGHLIGHTED_LINES {
        let Some(source_line) = source_lines.next() else {
            break;
        };
        let Ok(styled) = highlighter.highlight_line(source_line, &assets.syntaxes) else {
            return plain_text_preview(preview);
        };
        lines.push(PreviewTextLine {
            spans: styled_line_spans(source_line, &styled),
        });
    }

    HighlightedTextPreview {
        lines,
        syntax_name: Some(syntax.name.clone()),
        truncated_bytes: preview.truncated,
        truncated_lines: source_lines.next().is_some(),
    }
}

fn select_syntax<'a>(
    path: &Path,
    content: &str,
    syntaxes: &'a SyntaxSet,
) -> Option<&'a SyntaxReference> {
    let extension = path.extension().and_then(|extension| extension.to_str());
    extension
        .and_then(|extension| {
            syntaxes.find_syntax_by_extension(extension).or_else(|| {
                let lowercase = extension.to_ascii_lowercase();
                syntaxes.find_syntax_by_extension(&lowercase)
            })
        })
        .or_else(|| {
            content
                .lines()
                .next()
                .and_then(|line| syntaxes.find_syntax_by_first_line(line))
        })
}

fn plain_text_preview(preview: &TextPreview) -> HighlightedTextPreview {
    let mut source_lines = LinesWithEndings::from(&preview.content);
    let mut lines = Vec::new();
    for _ in 0..MAX_HIGHLIGHTED_LINES {
        let Some(source_line) = source_lines.next() else {
            break;
        };
        lines.push(PreviewTextLine {
            spans: vec![PreviewTextSpan {
                content: without_line_ending(source_line).to_owned(),
                style: PreviewTextStyle::default(),
            }],
        });
    }
    HighlightedTextPreview {
        lines,
        syntax_name: None,
        truncated_bytes: preview.truncated,
        truncated_lines: source_lines.next().is_some(),
    }
}

fn styled_line_spans(
    source_line: &str,
    styled: &[(syntect::highlighting::Style, &str)],
) -> Vec<PreviewTextSpan> {
    let mut remaining = without_line_ending(source_line).len();
    let mut spans = Vec::new();
    for (style, content) in styled {
        if remaining == 0 {
            break;
        }
        let take = remaining.min(content.len());
        let Some(content) = content.get(..take) else {
            return vec![PreviewTextSpan {
                content: without_line_ending(source_line).to_owned(),
                style: PreviewTextStyle::default(),
            }];
        };
        spans.push(PreviewTextSpan {
            content: content.to_owned(),
            style: PreviewTextStyle {
                foreground: Some([style.foreground.r, style.foreground.g, style.foreground.b]),
                bold: style.font_style.contains(FontStyle::BOLD),
                italic: style.font_style.contains(FontStyle::ITALIC),
                underline: style.font_style.contains(FontStyle::UNDERLINE),
            },
        });
        remaining -= take;
    }
    spans
}

fn without_line_ending(line: &str) -> &str {
    line.strip_suffix("\r\n")
        .or_else(|| line.strip_suffix('\n'))
        .unwrap_or(line)
}
