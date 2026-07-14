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
