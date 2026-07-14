use image::imageops::FilterType;
use image::metadata::Orientation;
use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader, Limits};
use std::fmt;
use std::fs::{self, File};
use std::io::{Cursor, Read};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::Path;

const DEFAULT_MAX_ENCODED_BYTES: usize = 64 * 1024 * 1024;
const DEFAULT_MAX_DIMENSION: u32 = 32_768;
const DEFAULT_MAX_PIXELS: u64 = 64 * 1024 * 1024;
const DEFAULT_MAX_DECODED_BYTES: u64 = 256 * 1024 * 1024;
const DEFAULT_MAX_OUTPUT_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ImagePreviewTarget {
    pub width_px: u32,
    pub height_px: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ImagePreviewLimits {
    pub(crate) max_encoded_bytes: usize,
    pub(crate) max_width: u32,
    pub(crate) max_height: u32,
    pub(crate) max_pixels: u64,
    pub(crate) max_decoded_bytes: u64,
    pub(crate) max_output_bytes: u64,
}

impl Default for ImagePreviewLimits {
    fn default() -> Self {
        Self {
            max_encoded_bytes: DEFAULT_MAX_ENCODED_BYTES,
            max_width: DEFAULT_MAX_DIMENSION,
            max_height: DEFAULT_MAX_DIMENSION,
            max_pixels: DEFAULT_MAX_PIXELS,
            max_decoded_bytes: DEFAULT_MAX_DECODED_BYTES,
            max_output_bytes: DEFAULT_MAX_OUTPUT_BYTES,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreparedImagePreview {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImagePreviewError {
    Io(std::io::ErrorKind),
    NotRegularFile,
    EmptyTarget,
    EncodedTooLarge {
        actual: u64,
        limit: u64,
    },
    UnsupportedFormat,
    DecodeFailed,
    DecoderPanicked,
    DimensionsTooLarge {
        width: u32,
        height: u32,
        max_width: u32,
        max_height: u32,
    },
    PixelCountTooLarge {
        actual: u64,
        limit: u64,
    },
    DecodedBytesTooLarge {
        actual: u64,
        limit: u64,
    },
    OutputTooLarge {
        actual: u64,
        limit: u64,
    },
    ArithmeticOverflow,
}

impl fmt::Display for ImagePreviewError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(kind) => write!(formatter, "image preview I/O failed: {kind:?}"),
            Self::NotRegularFile => {
                formatter.write_str("image preview source is not a regular file")
            }
            Self::EmptyTarget => formatter.write_str("image preview target is empty"),
            Self::EncodedTooLarge { actual, limit } => {
                write!(
                    formatter,
                    "encoded image is too large ({actual} > {limit} bytes)"
                )
            }
            Self::UnsupportedFormat => formatter.write_str("image format is not supported"),
            Self::DecodeFailed => formatter.write_str("image decoding failed"),
            Self::DecoderPanicked => formatter.write_str("image decoder panicked"),
            Self::DimensionsTooLarge {
                width,
                height,
                max_width,
                max_height,
            } => write!(
                formatter,
                "image dimensions are too large ({width}x{height} > {max_width}x{max_height})"
            ),
            Self::PixelCountTooLarge { actual, limit } => {
                write!(
                    formatter,
                    "image pixel count is too large ({actual} > {limit})"
                )
            }
            Self::DecodedBytesTooLarge { actual, limit } => {
                write!(
                    formatter,
                    "decoded image is too large ({actual} > {limit} bytes)"
                )
            }
            Self::OutputTooLarge { actual, limit } => {
                write!(
                    formatter,
                    "image preview output is too large ({actual} > {limit} bytes)"
                )
            }
            Self::ArithmeticOverflow => {
                formatter.write_str("image preview size arithmetic overflowed")
            }
        }
    }
}

impl std::error::Error for ImagePreviewError {}

pub(crate) fn read_image_preview(
    path: &Path,
    target: ImagePreviewTarget,
    limits: ImagePreviewLimits,
) -> Result<PreparedImagePreview, ImagePreviewError> {
    if target.width_px == 0 || target.height_px == 0 {
        return Err(ImagePreviewError::EmptyTarget);
    }

    let metadata = fs::metadata(path).map_err(|error| ImagePreviewError::Io(error.kind()))?;
    if !metadata.is_file() {
        return Err(ImagePreviewError::NotRegularFile);
    }

    let encoded_limit = usize_to_u64(limits.max_encoded_bytes)?;
    if metadata.len() > encoded_limit {
        return Err(ImagePreviewError::EncodedTooLarge {
            actual: metadata.len(),
            limit: encoded_limit,
        });
    }

    let file = File::open(path).map_err(|error| ImagePreviewError::Io(error.kind()))?;
    let mut limited = file.take(encoded_limit);
    let initial_capacity = limits.max_encoded_bytes.min(64 * 1024);
    let mut encoded = Vec::with_capacity(initial_capacity);
    limited
        .read_to_end(&mut encoded)
        .map_err(|error| ImagePreviewError::Io(error.kind()))?;

    let mut sentinel = [0_u8; 1];
    let extra = limited
        .get_mut()
        .read(&mut sentinel)
        .map_err(|error| ImagePreviewError::Io(error.kind()))?;
    if extra != 0 {
        return Err(ImagePreviewError::EncodedTooLarge {
            actual: encoded_limit
                .checked_add(1)
                .ok_or(ImagePreviewError::ArithmeticOverflow)?,
            limit: encoded_limit,
        });
    }

    prepare_image_preview_bytes(&encoded, target, limits)
}

pub(crate) fn prepare_image_preview_bytes(
    encoded: &[u8],
    target: ImagePreviewTarget,
    limits: ImagePreviewLimits,
) -> Result<PreparedImagePreview, ImagePreviewError> {
    if target.width_px == 0 || target.height_px == 0 {
        return Err(ImagePreviewError::EmptyTarget);
    }

    let actual = usize_to_u64(encoded.len())?;
    let encoded_limit = usize_to_u64(limits.max_encoded_bytes)?;
    if actual > encoded_limit {
        return Err(ImagePreviewError::EncodedTooLarge {
            actual,
            limit: encoded_limit,
        });
    }

    decode_with_panic_boundary(|| decode_image(encoded, target, limits))
}

fn decode_image(
    encoded: &[u8],
    target: ImagePreviewTarget,
    limits: ImagePreviewLimits,
) -> Result<PreparedImagePreview, ImagePreviewError> {
    let mut reader = ImageReader::new(Cursor::new(encoded))
        .with_guessed_format()
        .map_err(|error| ImagePreviewError::Io(error.kind()))?;
    let format = reader
        .format()
        .ok_or(ImagePreviewError::UnsupportedFormat)?;
    if !matches!(
        format,
        ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::Gif | ImageFormat::WebP
    ) {
        return Err(ImagePreviewError::UnsupportedFormat);
    }

    let mut decoder_limits = Limits::no_limits();
    decoder_limits.max_alloc = Some(limits.max_decoded_bytes);
    reader.limits(decoder_limits);
    let mut decoder = reader
        .into_decoder()
        .map_err(|_| ImagePreviewError::DecodeFailed)?;
    let (width, height) = decoder.dimensions();
    validate_source_dimensions(width, height, &limits)?;

    let decoded_bytes = decoder.total_bytes();
    if decoded_bytes > limits.max_decoded_bytes {
        return Err(ImagePreviewError::DecodedBytesTooLarge {
            actual: decoded_bytes,
            limit: limits.max_decoded_bytes,
        });
    }

    let mut enforced_limits = Limits::no_limits();
    enforced_limits.max_image_width = Some(limits.max_width);
    enforced_limits.max_image_height = Some(limits.max_height);
    enforced_limits.max_alloc = Some(limits.max_decoded_bytes);
    decoder
        .set_limits(enforced_limits)
        .map_err(|_| ImagePreviewError::DecodeFailed)?;

    let orientation = decoder
        .orientation()
        .map_err(|_| ImagePreviewError::DecodeFailed)?;
    let (resize_width, resize_height) = resize_dimensions(width, height, target, orientation)?;
    let (output_width, output_height) = if orientation_swaps_axes(orientation) {
        (resize_height, resize_width)
    } else {
        (resize_width, resize_height)
    };
    checked_rgba_bytes(output_width, output_height, limits.max_output_bytes)?;

    let mut image =
        DynamicImage::from_decoder(decoder).map_err(|_| ImagePreviewError::DecodeFailed)?;
    if image.width() != resize_width || image.height() != resize_height {
        image = image.resize_exact(resize_width, resize_height, FilterType::Triangle);
    }
    image.apply_orientation(orientation);

    let rgba = image.into_rgba8();
    let actual_width = rgba.width();
    let actual_height = rgba.height();
    let expected_len = checked_rgba_bytes(actual_width, actual_height, limits.max_output_bytes)?;
    if usize_to_u64(rgba.as_raw().len())? != expected_len {
        return Err(ImagePreviewError::DecodeFailed);
    }

    Ok(PreparedImagePreview {
        width: actual_width,
        height: actual_height,
        rgba: rgba.into_raw(),
    })
}

fn validate_source_dimensions(
    width: u32,
    height: u32,
    limits: &ImagePreviewLimits,
) -> Result<(), ImagePreviewError> {
    if width == 0 || height == 0 {
        return Err(ImagePreviewError::DecodeFailed);
    }
    if width > limits.max_width || height > limits.max_height {
        return Err(ImagePreviewError::DimensionsTooLarge {
            width,
            height,
            max_width: limits.max_width,
            max_height: limits.max_height,
        });
    }

    let pixels = u64::from(width)
        .checked_mul(u64::from(height))
        .ok_or(ImagePreviewError::ArithmeticOverflow)?;
    if pixels > limits.max_pixels {
        return Err(ImagePreviewError::PixelCountTooLarge {
            actual: pixels,
            limit: limits.max_pixels,
        });
    }
    Ok(())
}

fn resize_dimensions(
    width: u32,
    height: u32,
    target: ImagePreviewTarget,
    orientation: Orientation,
) -> Result<(u32, u32), ImagePreviewError> {
    let (oriented_width, oriented_height) = if orientation_swaps_axes(orientation) {
        (height, width)
    } else {
        (width, height)
    };
    let (output_width, output_height) = aspect_fit(
        oriented_width,
        oriented_height,
        target.width_px,
        target.height_px,
    )?;
    if orientation_swaps_axes(orientation) {
        Ok((output_height, output_width))
    } else {
        Ok((output_width, output_height))
    }
}

fn aspect_fit(
    width: u32,
    height: u32,
    target_width: u32,
    target_height: u32,
) -> Result<(u32, u32), ImagePreviewError> {
    if width == 0 || height == 0 || target_width == 0 || target_height == 0 {
        return Err(ImagePreviewError::ArithmeticOverflow);
    }
    if width <= target_width && height <= target_height {
        return Ok((width, height));
    }

    let width_limited = u64::from(target_width)
        .checked_mul(u64::from(height))
        .ok_or(ImagePreviewError::ArithmeticOverflow)?
        <= u64::from(target_height)
            .checked_mul(u64::from(width))
            .ok_or(ImagePreviewError::ArithmeticOverflow)?;

    if width_limited {
        let scaled_height = u64::from(height)
            .checked_mul(u64::from(target_width))
            .ok_or(ImagePreviewError::ArithmeticOverflow)?
            / u64::from(width);
        Ok((target_width, u64_to_nonzero_u32(scaled_height)?))
    } else {
        let scaled_width = u64::from(width)
            .checked_mul(u64::from(target_height))
            .ok_or(ImagePreviewError::ArithmeticOverflow)?
            / u64::from(height);
        Ok((u64_to_nonzero_u32(scaled_width)?, target_height))
    }
}

fn orientation_swaps_axes(orientation: Orientation) -> bool {
    matches!(
        orientation,
        Orientation::Rotate90
            | Orientation::Rotate270
            | Orientation::Rotate90FlipH
            | Orientation::Rotate270FlipH
    )
}

fn u64_to_nonzero_u32(value: u64) -> Result<u32, ImagePreviewError> {
    let value = value.max(1);
    u32::try_from(value).map_err(|_| ImagePreviewError::ArithmeticOverflow)
}

fn usize_to_u64(value: usize) -> Result<u64, ImagePreviewError> {
    u64::try_from(value).map_err(|_| ImagePreviewError::ArithmeticOverflow)
}

pub(crate) fn checked_rgba_bytes(
    width: u32,
    height: u32,
    limit: u64,
) -> Result<u64, ImagePreviewError> {
    let actual = u64::from(width)
        .checked_mul(u64::from(height))
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or(ImagePreviewError::ArithmeticOverflow)?;
    if actual > limit {
        return Err(ImagePreviewError::OutputTooLarge { actual, limit });
    }
    Ok(actual)
}

pub(crate) fn decode_with_panic_boundary<F>(
    decode: F,
) -> Result<PreparedImagePreview, ImagePreviewError>
where
    F: FnOnce() -> Result<PreparedImagePreview, ImagePreviewError>,
{
    catch_unwind(AssertUnwindSafe(decode)).map_err(|_| ImagePreviewError::DecoderPanicked)?
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{DynamicImage, ImageFormat, Rgba, RgbaImage};
    use std::fs;
    use std::io::Cursor;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn target(width_px: u32, height_px: u32) -> ImagePreviewTarget {
        ImagePreviewTarget {
            width_px,
            height_px,
        }
    }

    fn fixture(width: u32, height: u32) -> RgbaImage {
        RgbaImage::from_fn(width, height, |x, y| {
            let red = u8::try_from(x.saturating_mul(47) % 256).expect("fixture red");
            let green = u8::try_from(y.saturating_mul(83) % 256).expect("fixture green");
            let blue = u8::try_from((x + y).saturating_mul(29) % 256).expect("fixture blue");
            let alpha = if (x + y) % 2 == 0 { 255 } else { 96 };
            Rgba([red, green, blue, alpha])
        })
    }

    fn encoded(format: ImageFormat, width: u32, height: u32) -> Vec<u8> {
        let image = DynamicImage::ImageRgba8(fixture(width, height));
        let mut output = Cursor::new(Vec::new());
        image
            .write_to(&mut output, format)
            .expect("encode deterministic image fixture");
        output.into_inner()
    }

    fn limits_for(encoded_len: usize) -> ImagePreviewLimits {
        ImagePreviewLimits {
            max_encoded_bytes: encoded_len,
            max_width: 32,
            max_height: 32,
            max_pixels: 32 * 32,
            max_decoded_bytes: 32 * 32 * 8,
            max_output_bytes: 32 * 32 * 4,
        }
    }

    #[test]
    fn common_formats_decode_and_png_alpha_is_exact() {
        let png = encoded(ImageFormat::Png, 2, 2);
        let expected = fixture(2, 2).into_raw();
        let decoded = prepare_image_preview_bytes(&png, target(8, 8), limits_for(png.len()))
            .expect("decode PNG");
        assert_eq!((decoded.width, decoded.height), (2, 2));
        assert_eq!(decoded.rgba, expected);

        for format in [ImageFormat::Jpeg, ImageFormat::Gif, ImageFormat::WebP] {
            let bytes = encoded(format, 4, 3);
            let decoded =
                prepare_image_preview_bytes(&bytes, target(4, 3), limits_for(bytes.len()))
                    .expect("decode enabled common format");
            assert_eq!((decoded.width, decoded.height), (4, 3));
            assert_eq!(decoded.rgba.len(), 4 * 3 * 4);
        }
    }

    #[test]
    fn encoded_byte_limit_is_exact_and_checked_before_format_detection() {
        let png = encoded(ImageFormat::Png, 2, 2);
        prepare_image_preview_bytes(&png, target(2, 2), limits_for(png.len()))
            .expect("exact encoded byte limit passes");

        let error = prepare_image_preview_bytes(
            &png,
            target(2, 2),
            ImagePreviewLimits {
                max_encoded_bytes: png.len() - 1,
                ..limits_for(png.len())
            },
        )
        .expect_err("over-limit input must fail before decode");
        assert_eq!(
            error,
            ImagePreviewError::EncodedTooLarge {
                actual: u64::try_from(png.len()).expect("fixture length fits u64"),
                limit: u64::try_from(png.len() - 1).expect("fixture length fits u64"),
            }
        );

        let arbitrary = vec![0_u8; 9];
        let error = prepare_image_preview_bytes(
            &arbitrary,
            target(1, 1),
            ImagePreviewLimits {
                max_encoded_bytes: 8,
                ..limits_for(8)
            },
        )
        .expect_err("size failure precedes unsupported-format detection");
        assert!(matches!(error, ImagePreviewError::EncodedTooLarge { .. }));
    }

    #[test]
    fn corrupt_truncated_and_unsupported_inputs_are_explicit_and_panic_free() {
        let png = encoded(ImageFormat::Png, 2, 2);
        let mut truncated = png.clone();
        truncated.truncate(truncated.len() / 2);
        let truncated_result = std::panic::catch_unwind(|| {
            prepare_image_preview_bytes(&truncated, target(2, 2), limits_for(truncated.len()))
        });
        assert!(
            truncated_result.is_ok(),
            "truncated input escaped panic boundary"
        );
        assert_eq!(
            truncated_result.expect("panic-free result"),
            Err(ImagePreviewError::DecodeFailed)
        );

        let error =
            prepare_image_preview_bytes(b"this is not an image", target(2, 2), limits_for(20))
                .expect_err("unsupported format must be explicit");
        assert_eq!(error, ImagePreviewError::UnsupportedFormat);

        let panic_result = decode_with_panic_boundary(|| -> Result<PreparedImagePreview, _> {
            panic!("simulated third-party decoder panic")
        });
        assert_eq!(panic_result, Err(ImagePreviewError::DecoderPanicked));
    }

    #[test]
    fn dimensions_pixels_and_decoder_bytes_are_bounded_before_full_decode() {
        let png = encoded(ImageFormat::Png, 4, 4);
        let base = limits_for(png.len());

        let dimensions = prepare_image_preview_bytes(
            &png,
            target(4, 4),
            ImagePreviewLimits {
                max_width: 3,
                ..base
            },
        )
        .expect_err("width limit");
        assert_eq!(
            dimensions,
            ImagePreviewError::DimensionsTooLarge {
                width: 4,
                height: 4,
                max_width: 3,
                max_height: 32,
            }
        );

        let pixels = prepare_image_preview_bytes(
            &png,
            target(4, 4),
            ImagePreviewLimits {
                max_pixels: 15,
                ..base
            },
        )
        .expect_err("pixel limit");
        assert_eq!(
            pixels,
            ImagePreviewError::PixelCountTooLarge {
                actual: 16,
                limit: 15,
            }
        );

        let decoded_bytes = prepare_image_preview_bytes(
            &png,
            target(4, 4),
            ImagePreviewLimits {
                max_decoded_bytes: 63,
                ..base
            },
        )
        .expect_err("decoded byte limit");
        assert_eq!(
            decoded_bytes,
            ImagePreviewError::DecodedBytesTooLarge {
                actual: 64,
                limit: 63,
            }
        );
    }

    #[test]
    fn rgba_length_checks_overflow_exact_boundary_and_output_limit() {
        assert_eq!(checked_rgba_bytes(1, 1, 4), Ok(4));
        assert_eq!(
            checked_rgba_bytes(1, 1, 3),
            Err(ImagePreviewError::OutputTooLarge {
                actual: 4,
                limit: 3,
            })
        );
        assert_eq!(
            checked_rgba_bytes(u32::MAX, u32::MAX, u64::MAX),
            Err(ImagePreviewError::ArithmeticOverflow)
        );
    }

    #[test]
    fn aspect_fit_downscales_without_upscaling() {
        let png = encoded(ImageFormat::Png, 4, 2);
        let limits = limits_for(png.len());

        let downscaled =
            prepare_image_preview_bytes(&png, target(2, 2), limits).expect("aspect-fit downscale");
        assert_eq!((downscaled.width, downscaled.height), (2, 1));
        assert_eq!(downscaled.rgba.len(), 2 * 4);

        let original = prepare_image_preview_bytes(&png, target(8, 8), limits)
            .expect("do not upscale small source");
        assert_eq!((original.width, original.height), (4, 2));
        assert_eq!(original.rgba, fixture(4, 2).into_raw());
    }

    #[test]
    fn orientation_aware_fit_swaps_axes_before_downscaling() {
        assert_eq!(
            resize_dimensions(4, 2, target(1, 2), Orientation::Rotate90),
            Ok((2, 1))
        );
        assert_eq!(
            resize_dimensions(4, 2, target(1, 2), Orientation::Rotate270FlipH),
            Ok((2, 1))
        );
        assert_eq!(
            resize_dimensions(4, 2, target(1, 2), Orientation::FlipHorizontal),
            Ok((1, 1))
        );
        assert_eq!(
            resize_dimensions(100, 1, target(1, 1), Orientation::NoTransforms),
            Ok((1, 1))
        );
    }

    #[test]
    fn zero_target_is_rejected_before_input_or_decoder_work() {
        let error = prepare_image_preview_bytes(
            b"not an image",
            target(0, 10),
            ImagePreviewLimits::default(),
        )
        .expect_err("zero target");
        assert_eq!(error, ImagePreviewError::EmptyTarget);
    }

    #[test]
    fn file_read_is_bounded_and_nonregular_or_missing_sources_are_explicit() {
        let temp = TempDir::new("bounded-read");
        let png = encoded(ImageFormat::Png, 2, 2);
        let path = temp.root.join("preview.png");
        fs::write(&path, &png).expect("write PNG fixture");

        let prepared = read_image_preview(&path, target(2, 2), limits_for(png.len()))
            .expect("exact-size file read");
        assert_eq!((prepared.width, prepared.height), (2, 2));

        let error = read_image_preview(
            &path,
            target(2, 2),
            ImagePreviewLimits {
                max_encoded_bytes: png.len() - 1,
                ..limits_for(png.len())
            },
        )
        .expect_err("metadata limit must reject before file allocation");
        assert!(matches!(error, ImagePreviewError::EncodedTooLarge { .. }));

        assert_eq!(
            read_image_preview(&temp.root, target(2, 2), limits_for(png.len())),
            Err(ImagePreviewError::NotRegularFile)
        );
        assert_eq!(
            read_image_preview(
                &temp.root.join("missing.png"),
                target(2, 2),
                limits_for(png.len()),
            ),
            Err(ImagePreviewError::Io(std::io::ErrorKind::NotFound))
        );
    }

    fn unique() -> u64 {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        COUNTER.fetch_add(1, Ordering::Relaxed)
    }

    struct TempDir {
        root: PathBuf,
    }

    impl TempDir {
        fn new(tag: &str) -> Self {
            let root = std::env::temp_dir().join(format!(
                "herdr-image-preview-test-{}-{tag}-{}",
                std::process::id(),
                unique()
            ));
            fs::create_dir_all(&root).expect("create temp image root");
            Self { root }
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}
