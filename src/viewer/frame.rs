//! What the viewer draws, computed without touching the terminal.
//!
//! Everything here is pure: the inputs are a path, a page, a cell grid and the
//! host's cell size; the output is a [`ViewerFrame`] describing the picture and
//! the one line of text under it. No escape sequence is written, no terminal is
//! read, no clock is consulted — so the whole layer is testable without a PTY,
//! which is where the interesting cases live.
//!
//! A file herdr cannot decode is **not** an error here. It resolves to a frame
//! carrying the reason, because a tab that exits on a bad file closes instantly
//! and reads as a crash. Only conditions that make drawing meaningless at all —
//! no path, no room — produce `None`.

use std::path::{Path, PathBuf};

use crate::fm::image_preview::{
    read_image_preview, ImagePreviewLimits, ImagePreviewTarget, PreparedImagePreview,
};
use crate::fm::pdf_preview::{read_pdf_page_preview, PdfPreviewLimits};
use crate::kitty_graphics::HostCellSize;

/// The picture and the status line for one draw.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ViewerFrame {
    /// Absolute path the viewer was opened on, for the title.
    pub(crate) source_path: PathBuf,
    /// Ready pixels, when there are any to draw.
    pub(crate) picture: Option<ViewerPicture>,
    /// The line drawn under the picture. Never empty: it names the file, and
    /// carries the failure reason when there is no picture.
    pub(crate) status: String,
    /// Pages in the document, once a render reported it. `None` for images,
    /// which have exactly one.
    pub(crate) total_pages: Option<usize>,
}

/// Placed pixels: what to send and the cell box to send it to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ViewerPicture {
    pub(crate) prepared: PreparedImagePreview,
    /// Column of the picture's left edge, zero-based from the content area.
    pub(crate) col: u16,
    /// Row of the picture's top edge, zero-based from the content area.
    pub(crate) row: u16,
    /// Cell box the picture occupies.
    pub(crate) cols: u16,
    pub(crate) rows: u16,
}

/// Rows the viewer reserves under the picture for its status line.
const STATUS_ROWS: u16 = 1;

/// Compute the frame for `path` at `page`, drawn into a `cols`×`rows` grid.
///
/// `None` when there is no room to draw anything at all. Every other outcome —
/// including a file that cannot be read — is a frame, so the tab stays open and
/// says why.
pub(crate) fn compute_frame(
    path: &Path,
    page: usize,
    cols: u16,
    rows: u16,
    cell_size: HostCellSize,
) -> Option<ViewerFrame> {
    let content_rows = rows.checked_sub(STATUS_ROWS)?;
    if cols == 0 || content_rows == 0 || !cell_size.is_known() {
        return None;
    }
    let target = ImagePreviewTarget {
        width_px: u32::from(cols).checked_mul(cell_size.width_px)?,
        height_px: u32::from(content_rows).checked_mul(cell_size.height_px)?,
    };

    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned());

    if crate::fm::pdf_preview::is_pdf_path(path) {
        return Some(
            match read_pdf_page_preview(path, page, target, PdfPreviewLimits::default()) {
                Ok(rendered) => ViewerFrame {
                    source_path: path.to_path_buf(),
                    picture: place(&rendered.image, cols, content_rows, cell_size),
                    status: format!(
                        "{name}  ·  page {} of {}  ·  PageUp/PageDown  ·  q to close",
                        rendered.page.saturating_add(1),
                        rendered.total_pages
                    ),
                    total_pages: Some(rendered.total_pages),
                },
                Err(error) => unreadable(path, &name, format!("{error}")),
            },
        );
    }

    Some(
        match read_image_preview(path, target, ImagePreviewLimits::default()) {
            Ok(prepared) => ViewerFrame {
                source_path: path.to_path_buf(),
                picture: place(&prepared, cols, content_rows, cell_size),
                status: format!("{name}  ·  q to close"),
                total_pages: None,
            },
            Err(error) => unreadable(path, &name, format!("{error}")),
        },
    )
}

/// A frame with no picture, carrying the reason in its status line.
fn unreadable(path: &Path, name: &str, reason: String) -> ViewerFrame {
    ViewerFrame {
        source_path: path.to_path_buf(),
        picture: None,
        status: format!("{name}  ·  {reason}  ·  q to close"),
        total_pages: None,
    }
}

/// Centre `prepared` in a `cols`×`rows` grid.
///
/// The readers already fit the picture inside the requested pixel box, so this
/// only converts pixels to whole cells and centres the result. A picture that
/// does not fit in whole cells is refused rather than clipped: Kitty scales to
/// exactly the cell box it is given, so an overhanging box stretches the image
/// instead of cropping it.
fn place(
    prepared: &PreparedImagePreview,
    cols: u16,
    rows: u16,
    cell_size: HostCellSize,
) -> Option<ViewerPicture> {
    if prepared.width == 0 || prepared.height == 0 {
        return None;
    }
    let picture_cols = u16::try_from(prepared.width.div_ceil(cell_size.width_px)).ok()?;
    let picture_rows = u16::try_from(prepared.height.div_ceil(cell_size.height_px)).ok()?;
    if picture_cols == 0 || picture_rows == 0 || picture_cols > cols || picture_rows > rows {
        return None;
    }
    Some(ViewerPicture {
        prepared: prepared.clone(),
        col: (cols - picture_cols) / 2,
        row: (rows - picture_rows) / 2,
        cols: picture_cols,
        rows: picture_rows,
    })
}

/// Move `page` one step, staying inside a document of `total_pages`.
///
/// Neither direction wraps, matching the preview panel's `turn_pdf_page`: the
/// two are the same document behaviour seen from two surfaces, and disagreeing
/// would mean a page turn does different things depending on where the reader
/// opened the file.
pub(crate) fn turn_page(page: usize, total_pages: Option<usize>, forward: bool) -> usize {
    match (forward, total_pages) {
        (false, _) => page.saturating_sub(1),
        (true, Some(total)) => {
            let next = page.saturating_add(1);
            if next < total {
                next
            } else {
                page
            }
        }
        (true, None) => page,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn unique() -> u64 {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        NEXT.fetch_add(1, Ordering::Relaxed)
    }

    struct TempDir {
        root: PathBuf,
    }

    impl TempDir {
        fn new(tag: &str) -> Self {
            let root = std::env::temp_dir().join(format!(
                "herdr-viewer-{}-{tag}-{}",
                std::process::id(),
                unique()
            ));
            fs::create_dir_all(&root).expect("create temp root");
            Self { root }
        }

        fn write(&self, name: &str, bytes: &[u8]) -> PathBuf {
            let path = self.root.join(name);
            fs::write(&path, bytes).expect("write fixture");
            path
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn cells() -> HostCellSize {
        HostCellSize {
            width_px: 8,
            height_px: 16,
        }
    }

    fn png(width: u32, height: u32) -> Vec<u8> {
        let image = image::RgbaImage::from_fn(width, height, |x, y| {
            image::Rgba([
                u8::try_from(x % 256).expect("x channel"),
                u8::try_from(y % 256).expect("y channel"),
                0x40,
                0xff,
            ])
        });
        let mut out = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(image)
            .write_to(&mut out, image::ImageFormat::Png)
            .expect("encode png fixture");
        out.into_inner()
    }

    // TP-FVIEW-TAB-01: an image resolves to a frame with pixels and a status
    // line naming the file. This is the whole feature in one assertion.
    #[test]
    fn an_image_resolves_to_a_drawable_frame() {
        let td = TempDir::new("image-frame");
        let path = td.write("photo.png", &png(32, 32));

        let frame = compute_frame(&path, 0, 80, 24, cells()).expect("frame for an image");
        let picture = frame.picture.expect("an image has pixels");
        assert!(picture.cols > 0 && picture.rows > 0);
        assert!(frame.status.contains("photo.png"));
        assert_eq!(frame.total_pages, None, "an image has no page count");
    }

    // TP-FVIEW-TAB-02: a file that cannot be decoded still produces a frame,
    // with the reason in its status line. Returning `Err` here would close the
    // tab the instant it opened, which reads as a crash rather than an answer.
    #[test]
    fn an_undecodable_file_still_produces_a_frame_with_the_reason() {
        let td = TempDir::new("bad-image");
        let path = td.write("broken.png", b"this is not a png");

        let frame = compute_frame(&path, 0, 80, 24, cells()).expect("frame for a bad file");
        assert!(frame.picture.is_none());
        assert!(frame.status.contains("broken.png"));
        assert!(
            frame.status.len() > "broken.png".len() + 4,
            "the status must carry a reason, got {:?}",
            frame.status
        );
    }

    // TP-FVIEW-TAB-03: a missing file is the same shape of answer — a frame,
    // not an error. The tab explains itself instead of vanishing.
    #[test]
    fn a_missing_file_produces_a_frame_rather_than_an_error() {
        let td = TempDir::new("missing");
        let path = td.root.join("absent.png");

        let frame = compute_frame(&path, 0, 80, 24, cells()).expect("frame for a missing file");
        assert!(frame.picture.is_none());
        assert!(frame.status.contains("absent.png"));
    }

    // TP-FVIEW-TAB-04: a grid with no room for both the picture and its status
    // line yields nothing at all. Half a frame would place a host image over
    // the line meant to label it, and a Kitty placement is not erased by the
    // cells drawn under it.
    #[test]
    fn a_grid_too_small_to_hold_the_status_line_yields_no_frame() {
        let td = TempDir::new("tiny");
        let path = td.write("photo.png", &png(8, 8));

        for (cols, rows) in [(0u16, 24u16), (80, 0), (80, 1), (0, 0)] {
            assert!(
                compute_frame(&path, 0, cols, rows, cells()).is_none(),
                "{cols}x{rows} must not produce a frame"
            );
        }
        assert!(compute_frame(&path, 0, 80, 2, cells()).is_some());
    }

    // TP-FVIEW-TAB-05: an unknown cell size yields nothing. Guessing one would
    // place the picture against a grid that does not exist, and the picture
    // would land in the wrong cells with nothing on screen to explain it.
    #[test]
    fn an_unknown_cell_size_yields_no_frame() {
        let td = TempDir::new("no-cells");
        let path = td.write("photo.png", &png(8, 8));

        let unknown = HostCellSize {
            width_px: 0,
            height_px: 0,
        };
        assert!(compute_frame(&path, 0, 80, 24, unknown).is_none());
    }

    // TP-FVIEW-TAB-06: the picture is centred and never overhangs its grid.
    // Kitty scales an image to exactly the cell box it is handed, so a box
    // wider than the area stretches the picture instead of clipping it.
    #[test]
    fn the_picture_is_centred_and_stays_inside_the_grid() {
        let td = TempDir::new("centred");
        let path = td.write("photo.png", &png(64, 64));

        let (cols, rows) = (80u16, 24u16);
        let frame = compute_frame(&path, 0, cols, rows, cells()).expect("frame");
        let picture = frame.picture.expect("pixels");

        assert!(
            picture.col + picture.cols <= cols,
            "{picture:?} escapes {cols}"
        );
        assert!(
            picture.row + picture.rows <= rows - STATUS_ROWS,
            "{picture:?} overlaps the status row"
        );
        assert_eq!(picture.col, (cols - picture.cols) / 2, "centred across");
        assert_eq!(
            picture.row,
            (rows - STATUS_ROWS - picture.rows) / 2,
            "centred down"
        );
    }

    // TP-FVIEW-TAB-07: the same inputs give the same frame. The layer's whole
    // testability rests on this, so it is asserted rather than assumed.
    #[test]
    fn computing_the_same_frame_twice_gives_the_same_answer() {
        let td = TempDir::new("pure");
        let path = td.write("photo.png", &png(24, 24));

        let first = compute_frame(&path, 0, 80, 24, cells());
        let second = compute_frame(&path, 0, 80, 24, cells());
        assert_eq!(first, second);
    }

    // TP-FVIEW-TAB-08: page turning matches the preview panel's rule exactly —
    // neither direction wraps. The two surfaces show one document; disagreeing
    // would make a page turn mean different things depending on where the file
    // was opened.
    #[test]
    fn turning_pages_clamps_at_both_ends_and_never_wraps() {
        assert_eq!(turn_page(0, Some(10), false), 0, "no wrap backwards");
        assert_eq!(turn_page(9, Some(10), true), 9, "no wrap forwards");
        assert_eq!(turn_page(4, Some(10), true), 5);
        assert_eq!(turn_page(4, Some(10), false), 3);
    }

    // TP-FVIEW-TAB-09: without a known page count, forward is refused rather
    // than guessed. A guess past the end resolves to `PageOutOfRange`, which
    // turns navigation into an error message.
    #[test]
    fn turning_forward_without_a_page_count_is_refused() {
        assert_eq!(turn_page(3, None, true), 3);
        assert_eq!(
            turn_page(3, None, false),
            2,
            "the lower bound is always known"
        );
    }
}
