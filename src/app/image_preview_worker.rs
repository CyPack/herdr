#[cfg(test)]
mod tests {
    use super::*;
    use crate::fm::image_preview::{ImagePreviewError, ImagePreviewTarget, PreparedImagePreview};
    use crate::fm::{FmFilePreview, FmImagePreviewState, FmPreview, FmState};
    use crate::kitty_graphics::HostCellSize;
    use image::{DynamicImage, ImageFormat, Rgba, RgbaImage};
    use ratatui::layout::Rect;
    use std::io::Cursor;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;
    use std::time::{Duration, Instant};
    use tokio::sync::Notify;

    fn target(width_px: u32, height_px: u32) -> ImagePreviewTarget {
        ImagePreviewTarget {
            width_px,
            height_px,
        }
    }

    fn key(path: &str, model_generation: u64, target: ImagePreviewTarget) -> ImagePreviewKey {
        ImagePreviewKey::new(Path::new(path), model_generation, target)
    }

    #[test]
    fn image_slot_rejects_stale_path_generation_and_resize_results() {
        let first = key("alpha.png", 1, target(80, 40));
        let same = first.clone();
        let resized = key("alpha.png", 1, target(40, 20));
        let reloaded = key("alpha.png", 2, target(40, 20));
        let navigated = key("beta.png", 3, target(40, 20));
        let mut slot = ImagePreviewSlot::default();

        let ImagePreviewSync::Started {
            generation: first_worker_generation,
        } = slot.sync(Some(first.clone()))
        else {
            panic!("first target must start")
        };
        assert_eq!(slot.sync(Some(same)), ImagePreviewSync::Unchanged);

        let ImagePreviewSync::Started {
            generation: resize_worker_generation,
        } = slot.sync(Some(resized.clone()))
        else {
            panic!("resize target must rebind")
        };
        assert!(!slot.accepts(first_worker_generation, &first));
        assert!(slot.accepts(resize_worker_generation, &resized));

        let ImagePreviewSync::Started {
            generation: reload_worker_generation,
        } = slot.sync(Some(reloaded.clone()))
        else {
            panic!("model reload must rebind")
        };
        assert!(!slot.accepts(resize_worker_generation, &resized));
        assert!(slot.accepts(reload_worker_generation, &reloaded));

        let ImagePreviewSync::Started {
            generation: navigation_worker_generation,
        } = slot.sync(Some(navigated.clone()))
        else {
            panic!("navigation must rebind")
        };
        assert!(!slot.accepts(reload_worker_generation, &reloaded));
        assert!(slot.accepts(navigation_worker_generation, &navigated));
        assert_eq!(slot.sync(None), ImagePreviewSync::Stopped);
        assert!(!slot.accepts(navigation_worker_generation, &navigated));
    }

    #[test]
    fn image_worker_converts_processor_panic_to_typed_failure_and_stays_alive() {
        let mut worker = ImagePreviewWorker::with_processor(
            Arc::new(Notify::new()),
            |_path, _target| -> Result<PreparedImagePreview, ImagePreviewError> {
                panic!("simulated image processor panic")
            },
        );
        let current_key = key("panic.png", 1, target(8, 8));
        assert!(matches!(
            worker.sync_target(Some(current_key.clone())),
            ImagePreviewSync::Started { .. }
        ));

        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            let drained = worker.drain();
            assert!(
                !drained.disconnected,
                "panic boundary must keep worker alive"
            );
            if let Some(result) = drained.current {
                assert_eq!(result.key, current_key);
                assert_eq!(result.output, Err(ImagePreviewError::DecoderPanicked));
                break;
            }
            assert!(
                Instant::now() < deadline,
                "timed out waiting for panic result"
            );
            std::thread::yield_now();
        }
    }

    struct TempDir {
        root: PathBuf,
    }

    impl TempDir {
        fn new(tag: &str) -> Self {
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            let root = std::env::temp_dir().join(format!(
                "herdr-image-worker-{}-{tag}-{}",
                std::process::id(),
                COUNTER.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir_all(&root).expect("create image worker temp directory");
            Self { root }
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    fn encoded_png(width: u32, height: u32) -> Vec<u8> {
        let rgba = RgbaImage::from_fn(width, height, |x, y| {
            Rgba([
                u8::try_from(x % 256).expect("x channel"),
                u8::try_from(y % 256).expect("y channel"),
                0x7f,
                0xff,
            ])
        });
        let mut output = Cursor::new(Vec::new());
        DynamicImage::ImageRgba8(rgba)
            .write_to(&mut output, ImageFormat::Png)
            .expect("encode PNG fixture");
        output.into_inner()
    }

    fn test_app() -> crate::app::App {
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        crate::app::App::new(
            &crate::config::Config::default(),
            true,
            None,
            api_rx,
            crate::api::EventHub::default(),
        )
    }

    fn current_image_state(app: &crate::app::App) -> &FmImagePreviewState {
        match &app
            .state
            .file_manager
            .as_ref()
            .expect("open file manager")
            .preview
        {
            FmPreview::File(FmFilePreview::Image(preview)) => &preview.state,
            other => panic!("expected image preview, got {other:?}"),
        }
    }

    fn wait_for_ready(app: &mut crate::app::App) -> (ImagePreviewTarget, u32, u32) {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let _ = app.sync_image_preview_worker();
            if let FmImagePreviewState::Ready { target, prepared } = current_image_state(app) {
                return (*target, prepared.width, prepared.height);
            }
            assert!(
                Instant::now() < deadline,
                "timed out waiting for image preview"
            );
            std::thread::yield_now();
        }
    }

    #[test]
    fn app_clears_ready_pixels_on_resize_and_applies_only_the_new_target() {
        let temp = TempDir::new("resize");
        std::fs::write(temp.root.join("sample.png"), encoded_png(160, 80))
            .expect("write PNG fixture");
        let mut app = test_app();
        app.state.view.terminal_area = Rect::new(10, 5, 38, 10);
        app.image_preview_cell_size = HostCellSize {
            width_px: 8,
            height_px: 16,
        };
        app.state.file_manager = Some(FmState::new(&temp.root));

        assert!(
            app.sync_image_preview_worker(),
            "Pending -> Loading dirties frame"
        );
        assert_eq!(
            current_image_state(&app),
            &FmImagePreviewState::Loading {
                target: target(96, 128),
            }
        );
        assert_eq!(wait_for_ready(&mut app), (target(96, 128), 96, 48));

        app.image_preview_cell_size = HostCellSize {
            width_px: 4,
            height_px: 8,
        };
        assert!(
            app.sync_image_preview_worker(),
            "resize must clear old pixels immediately"
        );
        assert_eq!(
            current_image_state(&app),
            &FmImagePreviewState::Loading {
                target: target(48, 64),
            }
        );
        assert_eq!(wait_for_ready(&mut app), (target(48, 64), 48, 24));

        app.state.file_manager = None;
        assert!(!app.sync_image_preview_worker());
        assert!(app.state.file_manager.is_none());
    }
}
