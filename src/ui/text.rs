use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

pub(crate) fn display_width(text: &str) -> usize {
    UnicodeWidthStr::width(text)
}

pub(crate) fn display_width_u16(text: &str) -> u16 {
    display_width(text).min(u16::MAX as usize) as u16
}

pub(crate) fn truncate_end(text: &str, max_width: usize) -> String {
    if display_width(text) <= max_width {
        return text.to_string();
    }
    if max_width == 0 {
        return String::new();
    }
    if max_width == 1 {
        return "…".to_string();
    }

    let prefix = take_prefix_width(text, max_width.saturating_sub(1));
    format!("{prefix}…")
}

/// TP-FM-EXT-01: end-truncation that refuses to eat the extension — the part
/// that tells the reader what a file IS. `report-2026.xlsx` in 12 cells is
/// `report…xlsx`, never `report-20…`. Falls back to [`truncate_end`] when
/// there is no extension to save (directories, dotfiles, bare names) or when
/// the extension itself would not leave a single cell for the name.
pub(crate) fn truncate_keep_extension(text: &str, max_width: usize) -> String {
    if display_width(text) <= max_width {
        return text.to_string();
    }
    let Some((stem, ext)) = text.rsplit_once('.') else {
        return truncate_end(text, max_width);
    };
    // A slash-carrying or empty tail is not an extension. (A dotfile's
    // whole name lands in `ext` here, but a dotfile long enough to truncate
    // always has `ext_width >= max_width`, so the budget guard below already
    // sends it down the plain road — proven by the corners test.)
    if ext.is_empty() || ext.contains('/') {
        return truncate_end(text, max_width);
    }
    let ext_width = display_width(ext);
    // One cell for the ellipsis, one at least for the name.
    if max_width < ext_width + 2 {
        return truncate_end(text, max_width);
    }
    let prefix = take_prefix_width(stem, max_width - ext_width - 1);
    format!("{prefix}…{ext}")
}

pub(crate) fn middle_elide(text: &str, max_width: usize) -> String {
    if display_width(text) <= max_width {
        return text.to_string();
    }
    if max_width <= 1 {
        return "…".to_string();
    }

    let content_width = max_width.saturating_sub(1);
    let left_width = content_width / 2;
    let right_width = content_width.saturating_sub(left_width);
    let prefix = take_prefix_width(text, left_width);
    let suffix = take_suffix_width(text, right_width);
    format!("{prefix}…{suffix}")
}

fn take_prefix_width(text: &str, max_width: usize) -> String {
    let mut output = String::new();
    let mut width = 0usize;
    for ch in text.chars() {
        let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);
        if width + ch_width > max_width {
            break;
        }
        output.push(ch);
        width += ch_width;
    }
    output
}

fn take_suffix_width(text: &str, max_width: usize) -> String {
    let mut output = Vec::new();
    let mut width = 0usize;
    for ch in text.chars().rev() {
        let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);
        if width + ch_width > max_width {
            break;
        }
        output.push(ch);
        width += ch_width;
    }
    output.into_iter().rev().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // TP-FM-EXT-01: the corner cases that make extension-keeping honest.
    #[test]
    fn truncate_keep_extension_covers_its_corners() {
        // Short names pass through untouched.
        assert_eq!(truncate_keep_extension("a.rs", 10), "a.rs");
        // A long name gives up its middle, never its extension — the stem
        // spends every cell the ellipsis and the extension leave behind.
        assert_eq!(
            truncate_keep_extension("report-2026.xlsx", 12),
            "report-…xlsx"
        );
        // No extension: plain end truncation, no stray dots.
        assert_eq!(truncate_keep_extension("READMEFILE", 7), "README…");
        // A dotfile's dot is identity, not an extension.
        assert_eq!(truncate_keep_extension(".bashrc_backup", 8), ".bashrc…");
        // An extension wider than the budget cannot be saved.
        assert_eq!(truncate_keep_extension("a.verylongext", 5), "a.ve…");
    }

    #[test]
    fn truncate_end_uses_display_width() {
        let text = truncate_end("提交 herdr 的反馈", 16);

        assert_eq!(text, "提交 herdr 的反…");
        assert!(display_width(&text) <= 16);
    }

    #[test]
    fn middle_elide_uses_display_width() {
        let text = middle_elide("重构用户认证模块并迁移到统一登录服务", 12);

        assert!(text.contains('…'));
        assert!(display_width(&text) <= 12);
    }
}
