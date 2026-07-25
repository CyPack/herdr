//! Bounded PDF page rasterisation.
//!
//! One page at a time, scaled to the panel it will be drawn into, returned as
//! the same `PreparedImagePreview` the image path already produces — so the
//! whole Kitty delivery chain applies unchanged and a page is, from the
//! renderer's point of view, just another picture.
//!
//! Rendering is pure Rust (`hayro`): no external binary to find, no library to
//! ship alongside the executable, and identical behaviour on all three
//! platforms. That is why the project's earlier rejection of PDF preview no
//! longer applies — it was a rejection of shipping `pdfium`, not of the feature
//! (`docs/patterns/document-rendering.md` DR16).

use std::fmt;
use std::io;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::Path;
use std::sync::Arc;

use super::image_preview::PreparedImagePreview;
use super::ImagePreviewTarget;

/// A document larger than this is refused before parsing starts.
const DEFAULT_MAX_ENCODED_BYTES: u64 = 128 * 1024 * 1024;
/// Ceiling on one rasterised page, in pixels.
const DEFAULT_MAX_PAGE_PIXELS: u64 = 64 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PdfPreviewLimits {
    pub(crate) max_encoded_bytes: u64,
    pub(crate) max_page_pixels: u64,
}

impl Default for PdfPreviewLimits {
    fn default() -> Self {
        Self {
            max_encoded_bytes: DEFAULT_MAX_ENCODED_BYTES,
            max_page_pixels: DEFAULT_MAX_PAGE_PIXELS,
        }
    }
}

/// One rasterised page, plus how many the document has.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedPdfPage {
    /// Zero-based, as state everywhere else is. The UI adds one when it draws
    /// the indicator; keeping both conventions in one type is the documented
    /// way this goes wrong.
    pub page: usize,
    pub total_pages: usize,
    pub image: PreparedImagePreview,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PdfPreviewError {
    Io(io::ErrorKind),
    NotRegularFile,
    EmptyTarget,
    TooLarge {
        actual: u64,
        limit: u64,
    },
    /// Not a PDF this reader understands: damaged, or encrypted, which `hayro`
    /// states it does not support.
    Unreadable,
    NoPages,
    PageOutOfRange {
        page: usize,
        total: usize,
    },
    PageTooLarge {
        actual: u64,
        limit: u64,
    },
    RendererPanicked,
    ArithmeticOverflow,
}

impl fmt::Display for PdfPreviewError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(kind) => write!(formatter, "pdf preview I/O failed: {kind:?}"),
            Self::NotRegularFile => formatter.write_str("pdf preview source is not a regular file"),
            Self::EmptyTarget => formatter.write_str("pdf preview target is empty"),
            Self::TooLarge { actual, limit } => {
                write!(formatter, "pdf is too large ({actual} > {limit} bytes)")
            }
            Self::Unreadable => formatter.write_str("pdf could not be read (damaged or encrypted)"),
            Self::NoPages => formatter.write_str("pdf contains no pages"),
            Self::PageOutOfRange { page, total } => {
                write!(
                    formatter,
                    "pdf page {page} is outside a {total}-page document"
                )
            }
            Self::PageTooLarge { actual, limit } => {
                write!(
                    formatter,
                    "pdf page is too large ({actual} > {limit} pixels)"
                )
            }
            Self::RendererPanicked => formatter.write_str("pdf renderer panicked"),
            Self::ArithmeticOverflow => formatter.write_str("pdf page size arithmetic overflowed"),
        }
    }
}

impl std::error::Error for PdfPreviewError {}

pub(crate) fn is_pdf_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("pdf"))
}

/// Rasterise one page, sized to `target`.
///
/// Only the requested page is rendered, so a four-hundred-page document costs
/// what a one-page document costs. Page navigation is therefore a new request
/// rather than a seek through work already done (DR17).
pub(crate) fn read_pdf_page_preview(
    path: &Path,
    page: usize,
    target: ImagePreviewTarget,
    limits: PdfPreviewLimits,
) -> Result<PreparedPdfPage, PdfPreviewError> {
    if target.width_px == 0 || target.height_px == 0 {
        return Err(PdfPreviewError::EmptyTarget);
    }

    let metadata = std::fs::metadata(path).map_err(|error| PdfPreviewError::Io(error.kind()))?;
    if !metadata.is_file() {
        return Err(PdfPreviewError::NotRegularFile);
    }
    if metadata.len() > limits.max_encoded_bytes {
        return Err(PdfPreviewError::TooLarge {
            actual: metadata.len(),
            limit: limits.max_encoded_bytes,
        });
    }

    let bytes = std::fs::read(path).map_err(|error| PdfPreviewError::Io(error.kind()))?;
    render_with_panic_boundary(|| render_page(bytes, page, target, limits))
}

fn render_page(
    bytes: Vec<u8>,
    page_index: usize,
    target: ImagePreviewTarget,
    limits: PdfPreviewLimits,
) -> Result<PreparedPdfPage, PdfPreviewError> {
    let pdf =
        hayro::hayro_syntax::Pdf::new(Arc::new(bytes)).map_err(|_| PdfPreviewError::Unreadable)?;
    let pages = pdf.pages();
    let total_pages = pages.len();
    if total_pages == 0 {
        return Err(PdfPreviewError::NoPages);
    }
    let page = pages
        .get(page_index)
        .ok_or(PdfPreviewError::PageOutOfRange {
            page: page_index,
            total: total_pages,
        })?;

    let (page_width, page_height) = page.render_dimensions();
    if !(page_width.is_finite() && page_height.is_finite())
        || page_width <= 0.0
        || page_height <= 0.0
    {
        return Err(PdfPreviewError::Unreadable);
    }

    // Fit inside the target box. The panel's exact content rect is what the
    // image path already decodes to, so a page and a picture selected in the
    // same column arrive at the same size and respond to a resize identically.
    let scale = (target.width_px as f32 / page_width)
        .min(target.height_px as f32 / page_height)
        .max(f32::MIN_POSITIVE);

    let projected =
        (page_width * scale).ceil().max(1.0) as u64 * (page_height * scale).ceil().max(1.0) as u64;
    if projected > limits.max_page_pixels {
        return Err(PdfPreviewError::PageTooLarge {
            actual: projected,
            limit: limits.max_page_pixels,
        });
    }

    let cache = hayro::RenderCache::new();
    let interpret = hayro::hayro_interpret::InterpreterSettings::default();
    let settings = hayro::RenderSettings {
        x_scale: scale,
        y_scale: scale,
        ..Default::default()
    };
    let pixmap = hayro::render(page, &cache, &interpret, &settings);
    let width = u32::from(pixmap.width());
    let height = u32::from(pixmap.height());
    let pixels = u64::from(width)
        .checked_mul(u64::from(height))
        .ok_or(PdfPreviewError::ArithmeticOverflow)?;
    if pixels > limits.max_page_pixels {
        return Err(PdfPreviewError::PageTooLarge {
            actual: pixels,
            limit: limits.max_page_pixels,
        });
    }

    let rgba = composite_onto_white(pixmap);
    // Hashed exactly as the image path hashes its own output. The encoder uses
    // this to decide whether the picture on screen is still the right one, so
    // deriving it from anything but the pixels — page number and size, say —
    // would let two different pages of equal size be treated as identical and
    // the second one would never be drawn.
    let data_fingerprint = {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        rgba.hash(&mut hasher);
        hasher.finish()
    };

    Ok(PreparedPdfPage {
        page: page_index,
        total_pages,
        image: PreparedImagePreview {
            width,
            height,
            data_fingerprint,
            rgba,
        },
    })
}

/// Flatten a rendered page onto white paper.
///
/// A PDF page has no background of its own — the white is something viewers
/// add. Transmitted as rendered, the terminal's own background shows through
/// and dark body text becomes unreadable, while opaque callout boxes stay
/// legible; that mixed result is what makes the bug misleading rather than
/// obvious. Found by building and running a prototype, not by reading a spec.
fn composite_onto_white(pixmap: hayro::vello_cpu::Pixmap) -> Vec<u8> {
    let premultiplied = pixmap.take();
    let mut rgba = Vec::with_capacity(premultiplied.len().saturating_mul(4));
    for pixel in premultiplied.iter() {
        let alpha = u32::from(pixel.a);
        // Source-over against opaque white, with a premultiplied source.
        let over =
            |channel: u8| -> u8 { (u32::from(channel) + 255 * (255 - alpha) / 255).min(255) as u8 };
        rgba.extend_from_slice(&[over(pixel.r), over(pixel.g), over(pixel.b), 255]);
    }
    rgba
}

fn render_with_panic_boundary<F>(render: F) -> Result<PreparedPdfPage, PdfPreviewError>
where
    F: FnOnce() -> Result<PreparedPdfPage, PdfPreviewError>,
{
    catch_unwind(AssertUnwindSafe(render)).map_err(|_| PdfPreviewError::RendererPanicked)?
}

/// A real, minimal PDF with one filled rectangle per page.
///
/// Generated rather than committed, for the same reason the workbook fixture
/// is: page count and per-page colour become parameters, so a test can ask for
/// a three-page document whose pages are visibly distinguishable and then
/// assert which one came back.
///
/// `pages` gives each page an RGB fill in the 0.0..=1.0 range PDF uses. An
/// empty slice yields one page with no content at all, which is what proves
/// the white compositing step: a blank page renders fully transparent.
#[cfg(test)]
pub(crate) fn pdf_fixture(pages: &[[f32; 3]], width: u32, height: u32) -> Vec<u8> {
    let page_count = pages.len().max(1);
    let mut objects: Vec<String> = Vec::new();

    let kids: Vec<String> = (0..page_count)
        .map(|index| format!("{} 0 R", 3 + index * 2))
        .collect();
    objects.push("<< /Type /Catalog /Pages 2 0 R >>".to_owned());
    objects.push(format!(
        "<< /Type /Pages /Kids [{}] /Count {page_count} >>",
        kids.join(" ")
    ));

    for index in 0..page_count {
        let contents_id = 4 + index * 2;
        objects.push(format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {width} {height}] /Contents {contents_id} 0 R /Resources << >> >>"
        ));
        let stream = match pages.get(index) {
            Some([red, green, blue]) => {
                format!("{red} {green} {blue} rg\n0 0 {width} {height} re\nf\n")
            }
            None => String::new(),
        };
        objects.push(format!(
            "<< /Length {} >>\nstream\n{stream}endstream",
            stream.len()
        ));
    }

    let mut body = String::from("%PDF-1.7\n");
    let mut offsets = Vec::with_capacity(objects.len());
    for (index, object) in objects.iter().enumerate() {
        offsets.push(body.len());
        body.push_str(&format!("{} 0 obj\n{object}\nendobj\n", index + 1));
    }

    let startxref = body.len();
    body.push_str(&format!("xref\n0 {}\n", objects.len() + 1));
    body.push_str("0000000000 65535 f \n");
    for offset in &offsets {
        body.push_str(&format!("{offset:010} 00000 n \n"));
    }
    body.push_str(&format!(
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{startxref}\n%%EOF\n",
        objects.len() + 1
    ));
    body.into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
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
                "herdr-pdf-{label}-{}-{}",
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

    fn target(width_px: u32, height_px: u32) -> ImagePreviewTarget {
        ImagePreviewTarget {
            width_px,
            height_px,
        }
    }

    fn pixel(prepared: &PreparedPdfPage, x: u32, y: u32) -> [u8; 4] {
        let index = ((y * prepared.image.width + x) * 4) as usize;
        prepared.image.rgba[index..index + 4]
            .try_into()
            .expect("four channels")
    }

    /// TP-FPDF-01 / TP-FPDF-08: a document reports its real page count, only
    /// the page asked for is rasterised, and two pages never share a
    /// fingerprint.
    ///
    /// Rendering the whole document up front would make opening a long PDF cost
    /// proportional to its length, for a panel that shows one page (DR17).
    #[test]
    fn renders_the_requested_page_and_reports_the_document_length() {
        let td = TempDir::new("pages");
        let path = td.write(
            "three.pdf",
            &pdf_fixture(&[[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]], 40, 20),
        );

        let first = read_pdf_page_preview(&path, 0, target(40, 20), PdfPreviewLimits::default())
            .expect("first page renders");
        assert_eq!(first.total_pages, 3);
        assert_eq!(first.page, 0);

        let third = read_pdf_page_preview(&path, 2, target(40, 20), PdfPreviewLimits::default())
            .expect("third page renders");
        assert_eq!(third.page, 2);

        let [red_r, _, red_b, _] = pixel(&first, 20, 10);
        let [blue_r, _, blue_b, _] = pixel(&third, 20, 10);
        assert!(
            red_r > 200 && red_b < 60,
            "page 1 is the red one, got {:?}",
            pixel(&first, 20, 10)
        );
        assert!(
            blue_b > 200 && blue_r < 60,
            "page 3 is the blue one, got {:?}",
            pixel(&third, 20, 10)
        );
        assert_ne!(
            first.image.data_fingerprint, third.image.data_fingerprint,
            "two different pages must not share a fingerprint, or the encoder \
             treats the second as already drawn and never sends it"
        );
    }

    /// TP-FPDF-02: a page is composited onto white before it leaves this
    /// module.
    ///
    /// A PDF page carries no background of its own. Sent as rendered, the
    /// terminal's dark background shows through and dark body text becomes
    /// unreadable — while opaque boxes stay legible, which is what makes this
    /// bug misleading rather than obvious. Found by running a prototype.
    #[test]
    fn a_blank_page_is_composited_onto_white_not_left_transparent() {
        let td = TempDir::new("white");
        let path = td.write("blank.pdf", &pdf_fixture(&[], 20, 10));

        let prepared = read_pdf_page_preview(&path, 0, target(20, 10), PdfPreviewLimits::default())
            .expect("a blank page still renders");

        assert!(
            prepared
                .image
                .rgba
                .chunks_exact(4)
                .all(|px| px == [255, 255, 255, 255]),
            "every pixel of an empty page must be opaque white"
        );
    }

    /// TP-FPDF-03: page indices outside the document are a typed refusal.
    #[test]
    fn a_page_past_the_end_is_refused_with_both_numbers() {
        let td = TempDir::new("range");
        let path = td.write("two.pdf", &pdf_fixture(&[[0.0; 3], [0.0; 3]], 20, 10));

        assert_eq!(
            read_pdf_page_preview(&path, 2, target(20, 10), PdfPreviewLimits::default()),
            Err(PdfPreviewError::PageOutOfRange { page: 2, total: 2 })
        );
    }

    /// TP-FPDF-04: damaged and non-PDF input fails as a typed error, never a
    /// panic. `hayro` also refuses encrypted documents, which lands here.
    #[test]
    fn damaged_and_non_pdf_input_fails_without_panicking() {
        let td = TempDir::new("damaged");

        let truncated = {
            let mut bytes = pdf_fixture(&[[0.0; 3]], 20, 10);
            bytes.truncate(bytes.len() / 3);
            td.write("truncated.pdf", &bytes)
        };
        assert!(
            read_pdf_page_preview(&truncated, 0, target(20, 10), PdfPreviewLimits::default())
                .is_err(),
            "a truncated document must not render"
        );

        let lying = td.write("notreally.pdf", b"plain text pretending to be a pdf\n");
        assert_eq!(
            read_pdf_page_preview(&lying, 0, target(20, 10), PdfPreviewLimits::default()),
            Err(PdfPreviewError::Unreadable)
        );
    }

    /// TP-FPDF-05: the bounded gates refuse work before it is done.
    #[test]
    fn size_and_target_gates_refuse_before_rendering() {
        let td = TempDir::new("limits");
        let path = td.write("one.pdf", &pdf_fixture(&[[0.0; 3]], 20, 10));
        let actual = std::fs::metadata(&path).expect("metadata").len();

        assert_eq!(
            read_pdf_page_preview(
                &path,
                0,
                target(20, 10),
                PdfPreviewLimits {
                    max_encoded_bytes: 8,
                    ..PdfPreviewLimits::default()
                }
            ),
            Err(PdfPreviewError::TooLarge { actual, limit: 8 })
        );

        assert_eq!(
            read_pdf_page_preview(&path, 0, target(0, 10), PdfPreviewLimits::default()),
            Err(PdfPreviewError::EmptyTarget)
        );

        assert!(matches!(
            read_pdf_page_preview(
                &path,
                0,
                target(4000, 4000),
                PdfPreviewLimits {
                    max_page_pixels: 16,
                    ..PdfPreviewLimits::default()
                }
            ),
            Err(PdfPreviewError::PageTooLarge { .. })
        ));
    }

    /// TP-FPDF-06: the rendered page is sized to the panel it was asked for,
    /// so resizing the preview column re-renders rather than rescaling a stale
    /// bitmap — the same contract the image path already keeps.
    #[test]
    fn the_page_is_rendered_to_fit_the_requested_target() {
        let td = TempDir::new("fit");
        // A portrait page: twice as tall as it is wide.
        let path = td.write("portrait.pdf", &pdf_fixture(&[[0.0; 3]], 100, 200));

        let prepared =
            read_pdf_page_preview(&path, 0, target(300, 300), PdfPreviewLimits::default())
                .expect("renders");

        assert!(
            prepared.image.width <= 300 && prepared.image.height <= 300,
            "the page must fit inside the target box, got {}x{}",
            prepared.image.width,
            prepared.image.height
        );
        // Aspect preserved: the height constraint binds, so the page is 300
        // tall and about 150 wide rather than stretched to fill the square.
        assert!(
            prepared.image.height > prepared.image.width,
            "a portrait page must stay portrait, got {}x{}",
            prepared.image.width,
            prepared.image.height
        );
        let ratio = prepared.image.height as f32 / prepared.image.width as f32;
        assert!(
            (ratio - 2.0).abs() < 0.1,
            "aspect ratio must survive the fit, got {ratio}"
        );
    }

    #[test]
    fn missing_and_non_file_paths_are_typed_failures() {
        let td = TempDir::new("missing");
        assert_eq!(
            read_pdf_page_preview(
                &td.root.join("nope.pdf"),
                0,
                target(20, 10),
                PdfPreviewLimits::default()
            ),
            Err(PdfPreviewError::Io(io::ErrorKind::NotFound))
        );
        assert_eq!(
            read_pdf_page_preview(&td.root, 0, target(20, 10), PdfPreviewLimits::default()),
            Err(PdfPreviewError::NotRegularFile)
        );
    }

    #[test]
    fn pdf_paths_are_recognised_case_insensitively() {
        assert!(is_pdf_path(Path::new("manual.pdf")));
        assert!(is_pdf_path(Path::new("MANUAL.PDF")));
        assert!(!is_pdf_path(Path::new("manual.pdfx")));
        assert!(!is_pdf_path(Path::new("manual.txt")));
    }
}
