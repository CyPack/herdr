use std::fmt;
use std::io::{self, Read};
use std::path::Path;

const MAX_TEXT_PREVIEW_BYTES: usize = 64 * 1024;
const UTF8_SENTINEL_BYTES: usize = 4;

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
}

/// Stable domain failures from bounded text preparation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextPreviewError {
    Io(io::ErrorKind),
    InvalidUtf8 { valid_up_to: usize },
}

impl fmt::Display for TextPreviewError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(kind) => write!(formatter, "text preview I/O failed: {kind:?}"),
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
    let file = std::fs::File::open(path).map_err(|error| TextPreviewError::Io(error.kind()))?;
    let mut bytes = Vec::with_capacity(read_cap);
    file.take(read_cap as u64)
        .read_to_end(&mut bytes)
        .map_err(|error| TextPreviewError::Io(error.kind()))?;

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

    Ok(TextPreview { content, truncated })
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
