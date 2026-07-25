//! The file manager's raster preview, opened to fill the frame.
//!
//! The viewer owns no pixels. It changes which rect the one raster preview is
//! decoded and placed into, so enlarging produces a bigger decode rather than
//! an upscale of the panel-sized one. Everything downstream — the bounded
//! worker, the Kitty placement, the stale-result rejection — is the path that
//! was already there.

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::app::state::AppState;
use crate::ui::text::truncate_end;

/// Rows reserved above and below the picture for the title and the status line.
const VIEWER_CHROME_ROWS: u16 = 2;

/// The rect the enlarged picture may draw into, for a frame of this size.
///
/// `None` when the frame cannot hold the chrome and a picture at once: half a
/// viewer is worse than none, because the picture would be placed over the
/// title it is supposed to be labelled by.
pub(crate) fn preview_viewer_content_area(frame: Rect) -> Option<Rect> {
    let inner_width = frame.width.checked_sub(2)?;
    let inner_height = frame.height.checked_sub(2)?;
    let content_height = inner_height.checked_sub(VIEWER_CHROME_ROWS)?;
    if inner_width == 0 || content_height == 0 {
        return None;
    }
    Some(Rect {
        x: frame.x.saturating_add(1),
        y: frame.y.saturating_add(2),
        width: inner_width,
        height: content_height,
    })
}

/// Draw the viewer's chrome. The picture itself is a host image placed by the
/// Kitty path into [`preview_viewer_content_area`], so nothing here draws over
/// the content rect.
pub(crate) fn render_preview_viewer(app: &AppState, frame: &mut Frame, area: Rect) {
    let Some(viewer) = app.preview_viewer.as_ref() else {
        return;
    };
    if area.width == 0 || area.height == 0 {
        return;
    }
    frame.render_widget(Clear, area);
    frame.render_widget(Block::default().borders(Borders::ALL), area);

    let Some(content) = preview_viewer_content_area(area) else {
        return;
    };
    let name = viewer
        .source_path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| viewer.source_path.to_string_lossy().into_owned());
    let title = Line::from(vec![Span::styled(
        truncate_end(&name, content.width as usize),
        Style::default().add_modifier(Modifier::BOLD),
    )]);
    frame.render_widget(
        Paragraph::new(title),
        Rect {
            x: content.x,
            y: area.y.saturating_add(1),
            width: content.width,
            height: 1,
        },
    );

    let status_row = Rect {
        x: content.x,
        y: content.y.saturating_add(content.height),
        width: content.width,
        height: 1,
    };
    let status = match app.file_manager.as_ref().map(|fm| &fm.preview) {
        Some(crate::fm::FmPreview::File(crate::fm::FmFilePreview::Pdf(preview)))
            if preview.source_path == viewer.source_path =>
        {
            match preview.total_pages {
                Some(total) => format!(
                    "page {} of {total}  ·  PageUp/PageDown  ·  Esc to close",
                    preview.page.saturating_add(1)
                ),
                None => "Esc to close".to_owned(),
            }
        }
        _ => "Esc to close".to_owned(),
    };
    frame.render_widget(
        Paragraph::new(truncate_end(&status, status_row.width as usize)),
        status_row,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    // TP-FVIEW-01: the picture never overlaps the chrome that labels it.
    #[test]
    fn viewer_content_leaves_room_for_its_title_and_status() {
        let frame = Rect::new(0, 0, 80, 24);
        let content = preview_viewer_content_area(frame).expect("content area");
        assert!(
            content.y >= frame.y + 2,
            "the border and title rows stay above the picture"
        );
        assert_eq!(
            content.y + content.height + 1,
            frame.y + frame.height - 1,
            "the status row sits between the picture and the bottom border"
        );
        assert!(content.x > frame.x, "the left border stays clear");
        assert!(
            content.x + content.width < frame.x + frame.width,
            "the right border stays clear"
        );
    }

    // TP-FVIEW-02: a frame too small for both chrome and picture yields no
    // content rect at all. Returning a zero-height rect instead would place a
    // host image over the title, and a Kitty placement is not erased by the
    // cells drawn under it.
    #[test]
    fn a_frame_too_small_for_the_chrome_has_no_content_area() {
        for height in 0..5u16 {
            assert!(
                preview_viewer_content_area(Rect::new(0, 0, 40, height)).is_none(),
                "height {height} must not produce a content rect"
            );
        }
        for width in 0..3u16 {
            assert!(preview_viewer_content_area(Rect::new(0, 0, width, 24)).is_none());
        }
        assert!(preview_viewer_content_area(Rect::new(0, 0, 3, 5)).is_some());
    }
}
