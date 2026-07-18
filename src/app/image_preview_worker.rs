use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::thread::JoinHandle;

use tokio::sync::Notify;

use crate::fm::image_preview::{read_image_preview, ImagePreviewLimits};
use crate::fm::{
    FmFilePreview, FmImagePreviewState, FmPreview, ImagePreviewError, ImagePreviewTarget,
    PreparedImagePreview,
};
use crate::kitty_graphics::file_manager_image_target;

#[derive(Debug, Clone, PartialEq, Eq)]
struct ImagePreviewKey {
    path: PathBuf,
    model_generation: u64,
    target: ImagePreviewTarget,
}

impl ImagePreviewKey {
    fn new(path: &Path, model_generation: u64, target: ImagePreviewTarget) -> Self {
        Self {
            path: path.to_path_buf(),
            model_generation,
            target,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ImagePreviewSync {
    Unchanged,
    Started { generation: u64 },
    Stopped,
}

#[derive(Debug, Default)]
struct ImagePreviewSlot {
    generation: u64,
    active: Option<ImagePreviewKey>,
}

impl ImagePreviewSlot {
    fn sync(&mut self, target: Option<ImagePreviewKey>) -> ImagePreviewSync {
        if self.active == target {
            return ImagePreviewSync::Unchanged;
        }
        self.generation = self.generation.wrapping_add(1).max(1);
        self.active = target;
        if self.active.is_some() {
            ImagePreviewSync::Started {
                generation: self.generation,
            }
        } else {
            ImagePreviewSync::Stopped
        }
    }

    fn accepts(&self, generation: u64, key: &ImagePreviewKey) -> bool {
        self.generation == generation && self.active.as_ref() == Some(key)
    }
}

#[derive(Debug)]
struct ImagePreviewRequest {
    generation: u64,
    key: ImagePreviewKey,
}

#[derive(Debug)]
struct ImagePreviewResult {
    generation: u64,
    key: ImagePreviewKey,
    output: Result<PreparedImagePreview, ImagePreviewError>,
}

#[derive(Debug, Default)]
struct ImagePreviewDrain {
    current: Option<ImagePreviewResult>,
    disconnected: bool,
}

#[derive(Debug)]
struct ImagePreviewWorkerState {
    pending: Option<ImagePreviewRequest>,
    result: Option<ImagePreviewResult>,
    alive: bool,
    closed: bool,
}

impl Default for ImagePreviewWorkerState {
    fn default() -> Self {
        Self {
            pending: None,
            result: None,
            alive: true,
            closed: false,
        }
    }
}

type SharedImageWorkerState = Arc<(Mutex<ImagePreviewWorkerState>, Condvar)>;

struct ImageWorkerAliveGuard {
    shared: SharedImageWorkerState,
    wake: Arc<Notify>,
}

impl Drop for ImageWorkerAliveGuard {
    fn drop(&mut self) {
        let (state, _) = &*self.shared;
        lock_image_state(state).alive = false;
        self.wake.notify_one();
    }
}

pub(super) struct ImagePreviewWorker {
    slot: ImagePreviewSlot,
    shared: SharedImageWorkerState,
    handle: Option<JoinHandle<()>>,
    disconnect_reported: bool,
}

impl ImagePreviewWorker {
    pub(super) fn new(wake: Arc<Notify>) -> Self {
        Self::with_processor(wake, |path, target| {
            read_image_preview(path, target, ImagePreviewLimits::default())
        })
    }

    fn with_processor<F>(wake: Arc<Notify>, processor: F) -> Self
    where
        F: Fn(&Path, ImagePreviewTarget) -> Result<PreparedImagePreview, ImagePreviewError>
            + Send
            + 'static,
    {
        let shared = Arc::new((
            Mutex::new(ImagePreviewWorkerState::default()),
            Condvar::new(),
        ));
        let worker_shared = shared.clone();
        let handle = std::thread::spawn(move || {
            let _alive_guard = ImageWorkerAliveGuard {
                shared: worker_shared.clone(),
                wake: wake.clone(),
            };
            while let Some(request) = take_next_image_request(&worker_shared) {
                let output = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    processor(&request.key.path, request.key.target)
                }))
                .unwrap_or(Err(ImagePreviewError::DecoderPanicked));
                let result = ImagePreviewResult {
                    generation: request.generation,
                    key: request.key,
                    output,
                };
                let (state, _) = &*worker_shared;
                let mut state = lock_image_state(state);
                if state.closed {
                    break;
                }
                state.result = Some(result);
                drop(state);
                wake.notify_one();
            }
        });

        Self {
            slot: ImagePreviewSlot::default(),
            shared,
            handle: Some(handle),
            disconnect_reported: false,
        }
    }

    fn sync_target(&mut self, target: Option<ImagePreviewKey>) -> ImagePreviewSync {
        let sync = self.slot.sync(target.clone());
        let (state, pending) = &*self.shared;
        let mut state = lock_image_state(state);
        match sync {
            ImagePreviewSync::Started { generation } => {
                if let Some(key) = target {
                    state.pending = Some(ImagePreviewRequest { generation, key });
                    crate::render_prof::event("fm.image_worker.submitted");
                    pending.notify_one();
                }
            }
            ImagePreviewSync::Stopped => {
                state.pending = None;
                state.result = None;
            }
            ImagePreviewSync::Unchanged => {}
        }
        sync
    }

    fn drain(&mut self) -> ImagePreviewDrain {
        let (state, _) = &*self.shared;
        let mut state = lock_image_state(state);
        let result = state.result.take();
        let disconnected = !state.alive && !state.closed && !self.disconnect_reported;
        self.disconnect_reported |= disconnected;
        drop(state);

        let current = result.and_then(|result| {
            if self.slot.accepts(result.generation, &result.key) {
                crate::render_prof::event("fm.image_worker.completed");
                Some(result)
            } else {
                crate::render_prof::event("fm.image_worker.rejected");
                None
            }
        });
        ImagePreviewDrain {
            current,
            disconnected,
        }
    }
}

impl Drop for ImagePreviewWorker {
    fn drop(&mut self) {
        let (state, pending) = &*self.shared;
        let mut state = lock_image_state(state);
        state.closed = true;
        state.pending = None;
        state.result = None;
        drop(state);
        pending.notify_one();

        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn take_next_image_request(shared: &SharedImageWorkerState) -> Option<ImagePreviewRequest> {
    let (state, pending) = &**shared;
    let mut state = lock_image_state(state);
    while state.pending.is_none() && !state.closed {
        state = match pending.wait(state) {
            Ok(state) => state,
            Err(poisoned) => poisoned.into_inner(),
        };
    }
    if state.closed {
        return None;
    }
    state.pending.take()
}

fn lock_image_state(
    state: &Mutex<ImagePreviewWorkerState>,
) -> MutexGuard<'_, ImagePreviewWorkerState> {
    match state.lock() {
        Ok(state) => state,
        Err(poisoned) => poisoned.into_inner(),
    }
}

impl super::App {
    #[cfg(test)]
    pub(in crate::app) fn image_preview_worker_generation_for_test(&self) -> u64 {
        self.image_preview_worker.slot.generation
    }

    pub(super) fn sync_image_preview_worker(&mut self) -> bool {
        if self.state.view.file_manager_miller.resize_preview_active {
            return false;
        }

        let target = self.state.file_manager.as_ref().and_then(|file_manager| {
            let FmPreview::File(FmFilePreview::Image(preview)) = &file_manager.preview else {
                return None;
            };
            let target = file_manager_image_target(
                &self.state.view.file_manager_miller,
                file_manager,
                self.image_preview_cell_size,
            )?;
            Some(ImagePreviewKey::new(
                &preview.source_path,
                preview.generation,
                target,
            ))
        });

        match self.image_preview_worker.sync_target(target.clone()) {
            ImagePreviewSync::Started { .. } => {
                return target.is_some_and(|key| {
                    crate::render_prof::event("fm.image_target.refresh");
                    set_image_state(
                        &mut self.state,
                        &key,
                        FmImagePreviewState::Loading { target: key.target },
                    )
                });
            }
            ImagePreviewSync::Stopped => {
                return set_pending_image_state(&mut self.state);
            }
            ImagePreviewSync::Unchanged => {}
        }

        let drained = self.image_preview_worker.drain();
        let mut changed = false;
        if drained.disconnected {
            tracing::warn!("fm: image preview worker stopped; using explicit failure fallback");
            if let Some(key) = target.as_ref() {
                changed |= set_image_state(
                    &mut self.state,
                    key,
                    FmImagePreviewState::Unavailable {
                        target: key.target,
                        error: ImagePreviewError::DecodeFailed,
                    },
                );
            }
        }
        if let Some(result) = drained.current {
            let state = match result.output {
                Ok(prepared) => FmImagePreviewState::Ready {
                    target: result.key.target,
                    prepared,
                },
                Err(error) => FmImagePreviewState::Unavailable {
                    target: result.key.target,
                    error,
                },
            };
            changed |= set_image_state(&mut self.state, &result.key, state);
        }
        changed
    }
}

fn set_pending_image_state(state: &mut crate::app::state::AppState) -> bool {
    let Some(file_manager) = state.file_manager.as_mut() else {
        return false;
    };
    let FmPreview::File(FmFilePreview::Image(preview)) = &mut file_manager.preview else {
        return false;
    };
    if preview.state == FmImagePreviewState::Pending {
        return false;
    }
    preview.state = FmImagePreviewState::Pending;
    true
}

fn set_image_state(
    state: &mut crate::app::state::AppState,
    key: &ImagePreviewKey,
    next: FmImagePreviewState,
) -> bool {
    let Some(file_manager) = state.file_manager.as_mut() else {
        return false;
    };
    let FmPreview::File(FmFilePreview::Image(preview)) = &mut file_manager.preview else {
        return false;
    };
    if preview.source_path != key.path || preview.generation != key.model_generation {
        return false;
    }
    if preview.state == next {
        return false;
    }
    preview.state = next;
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fm::image_preview::{ImagePreviewError, ImagePreviewTarget, PreparedImagePreview};
    use crate::fm::{FmFilePreview, FmImagePreviewState, FmPreview, FmState};
    use crate::kitty_graphics::HostCellSize;
    use image::{DynamicImage, ImageFormat, Rgba, RgbaImage};
    use ratatui::layout::{Position, Rect};
    use std::io::Cursor;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{mpsc, Arc};
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

    #[test]
    fn image_worker_profile_counts_submitted_completed_and_rejected() {
        let (started_tx, started_rx) = mpsc::channel::<ImagePreviewTarget>();
        let (release_tx, release_rx) = mpsc::channel::<()>();
        let mut worker =
            ImagePreviewWorker::with_processor(Arc::new(Notify::new()), move |_path, target| {
                started_tx
                    .send(target)
                    .map_err(|_| ImagePreviewError::DecodeFailed)?;
                release_rx
                    .recv_timeout(Duration::from_secs(2))
                    .map_err(|_| ImagePreviewError::DecodeFailed)?;
                Ok(PreparedImagePreview {
                    width: 1,
                    height: 1,
                    data_fingerprint: u64::from(target.width_px),
                    rgba: vec![0, 0, 0, 0xff],
                })
            });
        let first_target = target(80, 40);
        let second_target = target(40, 20);

        let (_, profile) = crate::render_prof::observe_for_test(|| {
            assert!(matches!(
                worker.sync_target(Some(key("sample.png", 1, first_target))),
                ImagePreviewSync::Started { .. }
            ));
            assert_eq!(
                started_rx
                    .recv_timeout(Duration::from_secs(2))
                    .expect("first image request starts"),
                first_target
            );
            let second_key = key("sample.png", 1, second_target);
            assert!(matches!(
                worker.sync_target(Some(second_key.clone())),
                ImagePreviewSync::Started { .. }
            ));

            release_tx.send(()).expect("release stale image request");
            assert_eq!(
                started_rx
                    .recv_timeout(Duration::from_secs(2))
                    .expect("second image request starts"),
                second_target
            );
            let stale = worker.drain();
            assert!(
                stale.current.is_none(),
                "the first completion is rejected after the target changes"
            );
            assert!(!stale.disconnected);

            release_tx.send(()).expect("release current image request");
            let deadline = Instant::now() + Duration::from_secs(2);
            loop {
                let drained = worker.drain();
                assert!(!drained.disconnected);
                if let Some(current) = drained.current {
                    assert_eq!(current.key, second_key);
                    assert!(current.output.is_ok());
                    break;
                }
                assert!(
                    Instant::now() < deadline,
                    "timed out waiting for current image result"
                );
                std::thread::yield_now();
            }
        });

        assert_eq!(profile.counter("fm.image_worker.submitted"), 2);
        assert_eq!(profile.counter("fm.image_worker.rejected"), 1);
        assert_eq!(profile.counter("fm.image_worker.completed"), 1);
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

    fn wait_for_worker_result_generation(app: &crate::app::App, generation: u64) {
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            let has_result = {
                let (state, _) = &*app.image_preview_worker.shared;
                lock_image_state(state)
                    .result
                    .as_ref()
                    .is_some_and(|result| result.generation == generation)
            };
            if has_result {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "timed out waiting for image worker generation {generation}"
            );
            std::thread::yield_now();
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
        let frame = Rect::new(10, 5, 38, 10);
        app.image_preview_cell_size = HostCellSize {
            width_px: 8,
            height_px: 16,
        };
        app.state
            .try_open_file_manager_with(|_| Some(FmState::new(&temp.root)))
            .expect("Files activation");
        crate::ui::compute_view(&mut app.state, frame);

        assert!(
            app.sync_image_preview_worker(),
            "Pending -> Loading dirties frame"
        );
        assert_eq!(
            current_image_state(&app),
            &FmImagePreviewState::Loading {
                target: target(128, 80),
            }
        );
        assert_eq!(wait_for_ready(&mut app), (target(128, 80), 128, 64));

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
                target: target(64, 40),
            }
        );
        assert_eq!(wait_for_ready(&mut app), (target(64, 40), 64, 32));

        app.state.file_manager = None;
        assert!(!app.sync_image_preview_worker());
        assert!(app.state.file_manager.is_none());
    }

    // TP-TRAIL-T7-IMAGE-01: image decode size is owned by the committed Trail
    // detail panel, never by the retired Miller PREVIEW column.
    #[test]
    fn image_worker_targets_exact_trail_detail_panel_content_rect() {
        let temp = TempDir::new("trail-detail-target");
        std::fs::write(temp.root.join("sample.png"), encoded_png(160, 80))
            .expect("write PNG fixture");
        let mut app = test_app();
        app.image_preview_cell_size = HostCellSize {
            width_px: 8,
            height_px: 16,
        };
        app.state.mobile_width_threshold = 0;
        app.state.sidebar_collapsed = true;
        app.state
            .try_open_file_manager_with(|_| Some(FmState::new(&temp.root)))
            .expect("Files activation");
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 120, 30));
        let content = app
            .state
            .view
            .file_manager_trail
            .detail_panel
            .as_ref()
            .expect("image selection opens Trail detail")
            .content_rect;
        let expected = target(
            u32::from(content.width) * app.image_preview_cell_size.width_px,
            u32::from(content.height) * app.image_preview_cell_size.height_px,
        );

        assert!(
            app.sync_image_preview_worker(),
            "Pending -> Loading dirties the committed Trail frame"
        );
        assert_eq!(
            current_image_state(&app),
            &FmImagePreviewState::Loading { target: expected }
        );
    }

    #[test]
    fn preview_error_states_render_without_retry_loop() {
        let temp = TempDir::new("stable-preview-error");
        std::fs::write(temp.root.join("broken.png"), b"not an encoded image")
            .expect("write invalid image fixture");
        let mut app = test_app();
        app.state.kitty_graphics_enabled = true;
        app.image_preview_cell_size = HostCellSize {
            width_px: 8,
            height_px: 16,
        };
        app.state.mobile_width_threshold = 0;
        app.state.sidebar_collapsed = true;
        app.state
            .try_open_file_manager_with(|_| Some(FmState::new(&temp.root)))
            .expect("Files activation");
        let frame = Rect::new(0, 0, 120, 40);
        crate::ui::compute_view(&mut app.state, frame);

        assert!(
            app.sync_image_preview_worker(),
            "Pending must transition once to Loading"
        );
        let generation = app.image_preview_worker_generation_for_test();
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            let _ = app.sync_image_preview_worker();
            if matches!(
                current_image_state(&app),
                FmImagePreviewState::Unavailable {
                    error: ImagePreviewError::UnsupportedFormat | ImagePreviewError::DecodeFailed,
                    ..
                }
            ) {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "timed out waiting for stable image failure"
            );
            std::thread::yield_now();
        }

        for _ in 0..100 {
            assert!(
                !app.sync_image_preview_worker(),
                "a stable failure cannot dirty another frame or retry work"
            );
        }
        assert_eq!(
            app.image_preview_worker_generation_for_test(),
            generation,
            "a stable failure cannot allocate another worker generation"
        );

        let (first, _) = crate::server::render_stream::render_virtual(&mut app.state, frame, false);
        let (second, _) =
            crate::server::render_stream::render_virtual(&mut app.state, frame, false);
        assert_eq!(
            first, second,
            "stable failures must render deterministically"
        );
        let rendered = first
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(
            rendered.contains("(unsupported image)") || rendered.contains("(image decode failed)"),
            "the stable failure must render one explicit fallback: {rendered:?}"
        );
    }

    #[test]
    fn committed_resize_requests_at_most_one_image_target() {
        let temp = TempDir::new("committed-resize-target");
        std::fs::write(temp.root.join("sample.png"), encoded_png(160, 80))
            .expect("write PNG fixture");
        let mut app = test_app();
        app.image_preview_cell_size = HostCellSize {
            width_px: 8,
            height_px: 16,
        };
        app.state.mobile_width_threshold = 0;
        app.state.sidebar_collapsed = true;
        app.state
            .try_open_file_manager_with(|_| Some(FmState::new(&temp.root)))
            .expect("Files activation");
        let frame = Rect::new(0, 0, 90, 16);
        crate::ui::compute_view(&mut app.state, frame);

        assert!(
            app.sync_image_preview_worker(),
            "initial committed geometry starts one target"
        );
        let (original_target, _, _) = wait_for_ready(&mut app);
        let generation_before = app.image_preview_worker.slot.generation;

        let divider = app
            .state
            .view
            .file_manager_miller
            .dividers
            .get(1)
            .expect("three-column projection exposes the preview divider")
            .clone();
        assert!(
            app.begin_miller_resize_capture(divider.rect.x, divider.rect.y),
            "the typed divider starts capture"
        );
        let bounds = crate::ui::shell::ResizeBounds::new(
            crate::fm::miller::MILLER_COLUMN_MIN_WIDTH,
            crate::fm::miller::MILLER_COLUMN_MAX_WIDTH,
            crate::fm::miller::MILLER_COLUMN_MIN_WIDTH,
            crate::fm::miller::MILLER_COLUMN_MAX_WIDTH,
        );
        assert!(
            bounds.is_some_and(|bounds| app.state.shell_interaction.preview_resize(
                Position::new(divider.rect.x.saturating_add(4), u16::MAX),
                bounds,
            )),
            "the typed transaction previews the new pair"
        );
        crate::ui::compute_view(&mut app.state, frame);
        let (_, preview_profile) = crate::render_prof::observe_for_test(|| {
            for _ in 0..100 {
                assert!(
                    !app.sync_image_preview_worker(),
                    "preview sync cannot submit an image target"
                );
            }
        });
        assert_eq!(
            preview_profile.counter("fm.image_target.refresh"),
            0,
            "transient drag geometry cannot refresh the image target"
        );
        assert_eq!(
            app.image_preview_worker.slot.generation, generation_before,
            "preview produces zero worker generations"
        );

        assert!(
            app.commit_miller_resize(),
            "the valid transaction commits once"
        );
        assert!(
            !app.sync_image_preview_worker(),
            "release before committed compute still sees transient geometry"
        );
        assert_eq!(
            app.image_preview_worker.slot.generation, generation_before,
            "release alone cannot submit a target"
        );

        crate::ui::compute_view(&mut app.state, frame);
        let (started, committed_profile) =
            crate::render_prof::observe_for_test(|| app.sync_image_preview_worker());
        assert!(
            started,
            "the first committed frame starts one replacement target"
        );
        assert_eq!(
            committed_profile.counter("fm.image_target.refresh"),
            1,
            "the first committed frame refreshes the target exactly once"
        );
        assert_eq!(
            app.image_preview_worker.slot.generation,
            generation_before + 1,
            "commit requests at most one new target generation"
        );
        let (committed_target, _, _) = wait_for_ready(&mut app);
        assert_ne!(
            committed_target, original_target,
            "the exercised divider changes the target geometry"
        );
        assert_eq!(
            app.image_preview_worker.slot.generation,
            generation_before + 1,
            "completion cannot create another target generation"
        );
        assert!(
            !app.sync_image_preview_worker(),
            "the committed Ready target remains stable"
        );
    }

    #[test]
    fn stale_precommit_image_completion_is_rejected() {
        let temp = TempDir::new("stale-precommit");
        std::fs::write(temp.root.join("sample.png"), encoded_png(160, 80))
            .expect("write PNG fixture");
        let mut app = test_app();
        app.image_preview_cell_size = HostCellSize {
            width_px: 8,
            height_px: 16,
        };
        app.state.mobile_width_threshold = 0;
        app.state.sidebar_collapsed = true;
        app.state
            .try_open_file_manager_with(|_| Some(FmState::new(&temp.root)))
            .expect("Files activation");

        let (started_tx, started_rx) = mpsc::channel::<ImagePreviewTarget>();
        let (release_tx, release_rx) = mpsc::channel::<()>();
        app.image_preview_worker =
            ImagePreviewWorker::with_processor(app.render_notify.clone(), move |_path, target| {
                started_tx
                    .send(target)
                    .map_err(|_| ImagePreviewError::DecodeFailed)?;
                release_rx
                    .recv_timeout(Duration::from_secs(2))
                    .map_err(|_| ImagePreviewError::DecodeFailed)?;
                let marker = u8::try_from(target.width_px % 256).expect("target marker");
                Ok(PreparedImagePreview {
                    width: 1,
                    height: 1,
                    data_fingerprint: u64::from(target.width_px)
                        .wrapping_shl(32)
                        .wrapping_add(u64::from(target.height_px)),
                    rgba: vec![marker, marker, marker, 0xff],
                })
            });

        let frame = Rect::new(0, 0, 90, 16);
        crate::ui::compute_view(&mut app.state, frame);
        assert!(
            app.sync_image_preview_worker(),
            "the initial committed target starts generation one"
        );
        let original_target = started_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("first image job starts");

        let divider = app
            .state
            .view
            .file_manager_miller
            .dividers
            .get(1)
            .expect("three-column projection exposes the preview divider")
            .clone();
        assert!(
            app.begin_miller_resize_capture(divider.rect.x, divider.rect.y),
            "the typed divider starts capture"
        );
        let bounds = crate::ui::shell::ResizeBounds::new(
            crate::fm::miller::MILLER_COLUMN_MIN_WIDTH,
            crate::fm::miller::MILLER_COLUMN_MAX_WIDTH,
            crate::fm::miller::MILLER_COLUMN_MIN_WIDTH,
            crate::fm::miller::MILLER_COLUMN_MAX_WIDTH,
        );
        assert!(
            bounds.is_some_and(|bounds| app.state.shell_interaction.preview_resize(
                Position::new(divider.rect.x.saturating_add(4), u16::MAX),
                bounds,
            )),
            "the typed transaction accepts an out-of-area preview move"
        );
        crate::ui::compute_view(&mut app.state, frame);
        assert!(
            app.state.view.file_manager_miller.resize_preview_active,
            "precondition: transient geometry owns the projection"
        );

        release_tx.send(()).expect("finish the precommit job");
        wait_for_worker_result_generation(&app, 1);
        assert!(
            !app.sync_image_preview_worker(),
            "preview geometry cannot apply or replace the completed old job"
        );
        assert_eq!(
            current_image_state(&app),
            &FmImagePreviewState::Loading {
                target: original_target,
            },
            "the precommit completion remains unapplied"
        );

        assert!(
            app.commit_miller_resize(),
            "the valid typed transaction commits exactly once"
        );
        assert!(
            !app.sync_image_preview_worker(),
            "release alone cannot expose stale geometry before committed compute"
        );

        crate::ui::compute_view(&mut app.state, frame);
        assert!(
            !app.state.view.file_manager_miller.resize_preview_active,
            "the committed projection re-enables target synchronization"
        );
        assert!(
            app.sync_image_preview_worker(),
            "the committed target starts exactly one replacement generation"
        );
        let committed_target = started_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("committed image job starts");
        assert_ne!(
            committed_target, original_target,
            "the exercised divider must change the image target"
        );
        assert_eq!(
            app.image_preview_worker.slot.generation, 2,
            "one precommit and one committed target are the only generations"
        );
        assert!(
            !app.sync_image_preview_worker(),
            "draining the stale result cannot mutate the new Loading state"
        );
        assert_eq!(
            current_image_state(&app),
            &FmImagePreviewState::Loading {
                target: committed_target,
            },
            "generation one cannot overwrite generation two"
        );

        release_tx.send(()).expect("finish the committed job");
        wait_for_worker_result_generation(&app, 2);
        assert!(
            app.sync_image_preview_worker(),
            "only the committed generation may become Ready"
        );
        let FmImagePreviewState::Ready { target, prepared } = current_image_state(&app) else {
            panic!("committed image result must become Ready");
        };
        assert_eq!(*target, committed_target);
        assert_eq!(
            prepared.data_fingerprint,
            u64::from(committed_target.width_px)
                .wrapping_shl(32)
                .wrapping_add(u64::from(committed_target.height_px))
        );
        assert!(
            !app.sync_image_preview_worker(),
            "the committed Ready target remains stable"
        );
    }
}
