use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::fmt::Write as FmtWrite;
use std::hash::{Hash, Hasher};
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

use base64::Engine;
use ratatui::layout::Rect;

use crate::app::state::AppState;
use crate::app::Mode;
use crate::fm::image_preview::{ImagePreviewTarget, PreparedImagePreview};
use crate::ghostty::{
    KittyImageDescriptor, KittyImageFormat, KittyImagePlacement, KittyPlacementRenderInfo,
};
use crate::layout::{PaneId, PaneInfo};
use crate::terminal::TerminalRuntimeRegistry;

const KITTY_CHUNK_BYTES: usize = 3072;
/// Payloads at or above this size are worth deflating. A browser pane measured
/// on 2026-08-25 sent a remote client 25 MB for one click as raw RGBA; the same
/// pixels deflate 147x in 3 ms at level 1. Below this size the call costs more
/// than the bytes it saves, and the encoder runs once per frame per client.
const KITTY_COMPRESSION_MIN_BYTES: usize = 64 * 1024;
/// Level 1, not 6. Level 6 reaches 508x instead of 147x on the same frame but
/// spends seven times the CPU for it, on a path that runs per frame per client.
const KITTY_COMPRESSION_LEVEL: u32 = 1;
pub(crate) const HEADLESS_GRAPHICS_TRANSACTION_BUDGET: usize =
    crate::protocol::MAX_GRAPHICS_FRAME_SIZE - crate::protocol::MAX_FRAME_SIZE;
const HOST_IMAGE_ID_BASE: u32 = 10_000;
/// Host ids at or above this floor belong to the pane-layer stream road;
/// below it live the terminal-native pictures (a PTY drawing kitty itself).
/// The wire-level tests key on the same split.
pub(crate) const PANE_LAYER_HOST_ID_FLOOR: u32 = 0x8000_0000;
const FILE_MANAGER_PREVIEW_PANE_RAW: u32 = u32::MAX;
const FILE_MANAGER_PREVIEW_IMAGE_ID: u32 = 1;
const FILE_MANAGER_PREVIEW_PLACEMENT_ID: u32 = 1;
#[cfg(test)]
const PANE_GRAPHICS_IMAGE_ID_BIT: u32 = 1 << 31;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct HostCellSize {
    pub width_px: u32,
    pub height_px: u32,
}

impl HostCellSize {
    pub(crate) fn try_from_terminal(area: Rect) -> Option<Self> {
        let Ok(size) = crossterm::terminal::window_size() else {
            return None;
        };
        if size.columns == 0 || size.rows == 0 || size.width == 0 || size.height == 0 {
            return None;
        }
        Some(
            Self {
                width_px: (size.width as u32 / size.columns as u32).max(1),
                height_px: (size.height as u32 / size.rows as u32).max(1),
            }
            .for_area(area),
        )
    }

    pub(crate) fn is_known(self) -> bool {
        self.width_px > 0 && self.height_px > 0
    }

    pub(crate) fn fallback_for_area(area: Rect) -> Self {
        Self {
            width_px: 8,
            height_px: 16,
        }
        .for_area(area)
    }

    fn for_area(self, area: Rect) -> Self {
        if area.width == 0 || area.height == 0 {
            return Self::default();
        }
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HostViewKey {
    workspace_index: usize,
    tab_index: usize,
    file_manager_open: bool,
}

#[derive(Debug)]
struct HostPlacement {
    pane_id: PaneId,
    host_image_id: Option<u32>,
    area: Rect,
    cell_size: HostCellSize,
    source_key: HostSourceKey,
    placement: KittyImagePlacement,
    scrollback_offset: u32,
}

fn image_geometry_for_content_area(
    area: Rect,
    cell_size: HostCellSize,
) -> Option<(Rect, ImagePreviewTarget)> {
    if !cell_size.is_known() {
        return None;
    }
    let width_px = u32::from(area.width).checked_mul(cell_size.width_px)?;
    let height_px = u32::from(area.height).checked_mul(cell_size.height_px)?;
    if width_px == 0 || height_px == 0 {
        return None;
    }
    Some((
        area,
        ImagePreviewTarget {
            width_px,
            height_px,
        },
    ))
}

pub(crate) fn file_manager_image_target(
    app: &crate::app::state::AppState,
    cell_size: HostCellSize,
) -> Option<ImagePreviewTarget> {
    image_geometry_for_content_area(file_manager_raster_content_area(app)?, cell_size)
        .map(|(_, target)| target)
}

/// The rect a raster preview may draw into: the enlarged viewer's when one is
/// open, the Trail's detail panel otherwise.
///
/// One authority for both. Enlarging is not a second picture but the same one
/// decoded and placed into a different rect, so the decode target, the Kitty
/// placement and the indicator hit test all move together — and the worker's
/// key changes with the rect, which is what makes the viewer re-decode at full
/// size instead of stretching the panel-sized pixels.
pub(crate) fn file_manager_raster_content_area(app: &crate::app::state::AppState) -> Option<Rect> {
    let file_manager = app.file_manager.as_ref()?;
    if let Some(viewer) = app.preview_viewer.as_ref() {
        let content = app.view.preview_viewer_content_area?;
        // The viewer names the file it was opened on. A selection that has
        // since moved on belongs to the panel behind it, not to this rect.
        return file_manager_trail_image_content_area(&app.view.file_manager_trail, file_manager)
            .is_some()
            .then_some(())
            .and(
                (file_manager.trail_snapshots.detail()?.path == viewer.source_path)
                    .then_some(content),
            );
    }
    file_manager_trail_image_content_area(&app.view.file_manager_trail, file_manager)
}

/// The panel rect a raster preview may draw into, if one is selected.
///
/// A PDF page and an image are the same thing to everything downstream: both
/// resolve to RGBA sized to this exact rect, so both answer here rather than
/// growing a parallel geometry path with its own rounding.
pub(crate) fn file_manager_trail_image_content_area(
    snapshot: &crate::ui::TrailViewSnapshot,
    file_manager: &crate::fm::FmState,
) -> Option<Rect> {
    let detail = file_manager.trail_snapshots.detail()?;
    let source_path = match (&detail.preview, &file_manager.preview) {
        (
            crate::fm::trail_snapshots::TrailDetailPreview::Image,
            crate::fm::FmPreview::File(crate::fm::FmFilePreview::Image(preview)),
        ) => &preview.source_path,
        (
            crate::fm::trail_snapshots::TrailDetailPreview::Pdf,
            crate::fm::FmPreview::File(crate::fm::FmFilePreview::Pdf(preview)),
        ) => &preview.source_path,
        _ => return None,
    };
    if source_path != &detail.path {
        return None;
    }
    snapshot
        .detail_panel
        .as_ref()
        .map(|panel| panel.content_rect)
}

#[cfg(test)]
fn file_manager_image_placement(
    file_manager_area: Rect,
    cell_size: HostCellSize,
    prepared: &PreparedImagePreview,
) -> Option<HostPlacement> {
    let content_area = crate::ui::file_manager_preview_content_area(file_manager_area)?;
    file_manager_image_placement_in_content_area(content_area, cell_size, prepared, true)
}

fn file_manager_image_placement_in_content_area(
    content_area: Rect,
    cell_size: HostCellSize,
    prepared: &PreparedImagePreview,
    include_data: bool,
) -> Option<HostPlacement> {
    let (area, target) = image_geometry_for_content_area(content_area, cell_size)?;
    if prepared.width == 0
        || prepared.height == 0
        || prepared.width > target.width_px
        || prepared.height > target.height_px
    {
        return None;
    }

    let expected_len = u64::from(prepared.width)
        .checked_mul(u64::from(prepared.height))?
        .checked_mul(4)?;
    if u64::try_from(prepared.rgba.len()).ok()? != expected_len {
        return None;
    }

    let grid_cols = prepared.width.div_ceil(cell_size.width_px);
    let grid_rows = prepared.height.div_ceil(cell_size.height_px);
    if grid_cols == 0
        || grid_rows == 0
        || grid_cols > u32::from(area.width)
        || grid_rows > u32::from(area.height)
    {
        return None;
    }
    let viewport_col = i32::try_from((u32::from(area.width) - grid_cols) / 2).ok()?;
    let viewport_row = i32::try_from((u32::from(area.height) - grid_rows) / 2).ok()?;

    Some(HostPlacement {
        host_image_id: None,
        pane_id: PaneId::from_raw(FILE_MANAGER_PREVIEW_PANE_RAW),
        area,
        cell_size,
        source_key: HostSourceKey::Terminal {
            pane_id: PaneId::from_raw(FILE_MANAGER_PREVIEW_PANE_RAW),
            image_id: FILE_MANAGER_PREVIEW_IMAGE_ID,
        },
        scrollback_offset: 0,
        placement: KittyImagePlacement {
            image_id: FILE_MANAGER_PREVIEW_IMAGE_ID,
            placement_id: FILE_MANAGER_PREVIEW_PLACEMENT_ID,
            z: 0,
            x_offset: 0,
            y_offset: 0,
            image_width: prepared.width,
            image_height: prepared.height,
            format: KittyImageFormat::Rgba,
            data_len: prepared.rgba.len(),
            data_fingerprint: prepared.data_fingerprint,
            data: if include_data {
                prepared.rgba.clone()
            } else {
                Vec::new()
            },
            render: KittyPlacementRenderInfo {
                pixel_width: prepared.width,
                pixel_height: prepared.height,
                grid_cols,
                grid_rows,
                viewport_col,
                viewport_row,
                source_x: 0,
                source_y: 0,
                source_width: 0,
                source_height: 0,
            },
        },
    })
}

fn collect_file_manager_image_placement(
    app: &AppState,
    cell_size: HostCellSize,
    uploaded_images: &HashMap<u32, ImageSignature>,
) -> Option<HostPlacement> {
    let file_manager = app.file_manager.as_ref()?;
    // Whichever track produced them, ready pixels are ready pixels.
    let (target, prepared) = match &file_manager.preview {
        crate::fm::FmPreview::File(crate::fm::FmFilePreview::Image(preview)) => {
            match &preview.state {
                crate::fm::FmImagePreviewState::Ready { target, prepared } => (target, prepared),
                _ => return None,
            }
        }
        crate::fm::FmPreview::File(crate::fm::FmFilePreview::Pdf(preview)) => {
            match &preview.state {
                crate::fm::FmPdfPreviewState::Ready {
                    target, prepared, ..
                } => (target, prepared),
                _ => return None,
            }
        }
        _ => return None,
    };
    let content_area = file_manager_raster_content_area(app)?;
    if image_geometry_for_content_area(content_area, cell_size).map(|(_, target)| target)?
        != *target
    {
        return None;
    }

    let mut placement =
        file_manager_image_placement_in_content_area(content_area, cell_size, prepared, false)?;
    let format_code = kitty_format_code(placement.placement.format);
    let signature = image_signature(&placement, format_code);
    let host_id = host_image_id(placement.pane_id, &placement.placement);
    if uploaded_images.get(&host_id).copied() != Some(signature) {
        placement.placement.data = prepared.rgba.clone();
    }
    Some(placement)
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
enum HostSourceKey {
    Terminal { pane_id: PaneId, image_id: u32 },
    PaneLayer { pane_id: PaneId, layer_id: String },
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
struct ImageSignature {
    image_width: u32,
    image_height: u32,
    format_code: u32,
    data_len: usize,
    data_fingerprint: u64,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
struct PlacementSignature {
    x: u16,
    y: u16,
    cols: u32,
    rows: u32,
    source_x: u32,
    source_y: u32,
    source_width: u32,
    source_height: u32,
    x_offset: u32,
    y_offset: u32,
    z: i32,
    scrollback_offset: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ClippedPlacement {
    x: u16,
    y: u16,
    cols: u32,
    rows: u32,
    source_x: u32,
    source_y: u32,
    source_width: u32,
    source_height: u32,
    x_offset: u32,
    y_offset: u32,
}

#[derive(Debug, Default, Clone)]
pub(crate) struct HostGraphicsCache {
    images: HashMap<u32, ImageSignature>,
    placements: HashMap<(u32, u32), PlacementSignature>,
    /// Host image currently backing each (pane, source image id) pair.
    sources: HashMap<HostSourceKey, u32>,
    oversized: HashMap<HostSourceKey, ImageSignature>,
    continuation: Option<(HostSourceKey, u32, usize)>,
    view: Option<HostViewKey>,
    replay_placements: bool,
    replayed_placements: HashSet<(u32, u32)>,
}

static KITTY_GRAPHICS_ENABLED: AtomicBool = AtomicBool::new(false);
/// Whether payloads travel deflated (`o=z`). On by default: the terminals that
/// speak the graphics protocol at all inflate it, and the bytes it saves are
/// the difference between a usable and an unusable remote session.
static KITTY_PAYLOAD_COMPRESSION: AtomicBool = AtomicBool::new(true);
static LOCAL_HOST_GRAPHICS: OnceLock<Mutex<HostGraphicsCache>> = OnceLock::new();

pub(crate) fn set_enabled(enabled: bool) {
    KITTY_GRAPHICS_ENABLED.store(enabled, Ordering::Release);
}

/// TP-GFX-ZLIB-01 kill switch, for a terminal that cannot inflate `o=z`.
/// One transmission that removes every image the outer terminal knows,
/// whether or not this server was the one that put it there. `clear_bytes`
/// can only delete ids this cache remembers; after a lost delete frame, a
/// server restart, or another program's leftovers, the outer terminal holds
/// ids nobody remembers -- this is the only delete that reaches them.
pub(crate) const KITTY_DELETE_ALL: &[u8] = b"\x1b_Ga=d,d=A,q=2\x1b\\";

/// Resets the cache bookkeeping and returns the delete-all barrier the next
/// frame must open with. TP-GFX-RESET-01 wiring.
pub(crate) fn reset_barrier_bytes(cache: &mut HostGraphicsCache) -> Vec<u8> {
    let _ = cache.clear_bytes();
    KITTY_DELETE_ALL.to_vec()
}

pub(crate) fn set_kitty_payload_compression(enabled: bool) {
    KITTY_PAYLOAD_COMPRESSION.store(enabled, Ordering::Release);
}

pub(crate) fn is_enabled() -> bool {
    KITTY_GRAPHICS_ENABLED.load(Ordering::Acquire)
}

fn frame_graphics_bytes(bytes: &[u8]) -> Vec<u8> {
    let mut framed = Vec::with_capacity(bytes.len() + 4);
    framed.extend_from_slice(b"\x1b7");
    framed.extend_from_slice(bytes);
    framed.extend_from_slice(b"\x1b8");
    framed
}

pub(crate) fn paint_local_pane_graphics(
    app: &AppState,
    graphics: &crate::app::pane_graphics::Runtime,
    terminal_runtimes: &TerminalRuntimeRegistry,
    cell_size: HostCellSize,
) -> io::Result<()> {
    let cache = LOCAL_HOST_GRAPHICS.get_or_init(|| Mutex::new(HostGraphicsCache::default()));
    let Ok(mut cache) = cache.lock() else {
        return Ok(());
    };
    if graphics.slots.is_empty() && !cache.has_pane_sources() {
        let encoded = encode_local_pane_graphics(
            app,
            graphics,
            terminal_runtimes,
            app.view.tab_surface(),
            cell_size,
            None,
            // The monolithic paint path always re-blits over the pictures.
            true,
            &mut cache,
        );
        drop(cache);
        if encoded.bytes.is_empty() {
            return Ok(());
        }
        let mut stdout = io::stdout().lock();
        stdout.write_all(&frame_graphics_bytes(&encoded.bytes))?;
        return stdout.flush();
    }

    let mut stdout = io::stdout().lock();
    loop {
        let encoded = encode_local_pane_graphics(
            app,
            graphics,
            terminal_runtimes,
            app.view.tab_surface(),
            cell_size,
            None,
            // The monolithic paint path always re-blits over the pictures.
            true,
            &mut cache,
        );
        if !encoded.bytes.is_empty() {
            stdout.write_all(&frame_graphics_bytes(&encoded.bytes))?;
        }
        if !encoded.incomplete {
            break;
        }
    }
    stdout.flush()
}

pub(crate) struct EncodedGraphics {
    pub(crate) bytes: Vec<u8>,
    pub(crate) incomplete: bool,
}

/// `reseat_placements`: whether this frame's text blit overdraws the
/// placements. A full re-blit (resize, divider release, a repaint) must
/// re-seat cached pictures; a frame whose only change is status text keeps
/// them seated — the client blits only the changed cells, no picture is
/// overdrawn, and a re-seat per tick is graphics bytes on the wire every
/// second, reading as the picture blinking. TP-GFX-REPLAY-01
pub(crate) fn encode_local_pane_graphics(
    app: &AppState,
    graphics: &crate::app::pane_graphics::Runtime,
    terminal_runtimes: &TerminalRuntimeRegistry,
    surface: crate::ui::TabSurfaceView<'_>,
    cell_size: HostCellSize,
    transaction_budget: Option<usize>,
    reseat_placements: bool,
    cache: &mut HostGraphicsCache,
) -> EncodedGraphics {
    // Two surfaces, two rules (fork). Pane graphics belong to a terminal
    // application and must not paint over herdr's chrome — terminal mode. The
    // file manager preview is herdr's OWN drawing: it stays visible under a
    // floating menu and is only hidden when an overlay covers the whole frame.
    let files_on_stage = app.staged_file_manager().is_some();
    let mode_ok = if files_on_stage {
        !app.overlay_hides_stage_surface()
    } else {
        app.mode == Mode::Terminal
    };
    let visible = mode_ok && cell_size.is_known();
    if reseat_placements {
        // The caller's text blit overdraws the placements: file the re-seat
        // request up front so the incremental and the legacy road read the
        // same pending-replay bookkeeping — exactly what the unconditional
        // in-path request used to do, now asked for per cause instead of per
        // frame. TP-GFX-REPLAY-01
        cache.request_placement_replay();
    }
    if graphics.slots.is_empty() {
        // TP-GFX-RESIZE-01 reseat half: a caller that just re-blitted every
        // text cell (a full frame) wiped the pictures off the screen with the
        // text. Its re-seat request — filed above per cause, or already
        // pending in the incremental bookkeeping that clear_pane_sources
        // resets — is read first and later forces the legacy path to
        // re-display unchanged placements. A text-refresh-only frame files
        // no request. TP-GFX-REPLAY-01
        let replay_placements = cache.replay_placements;
        let mut bytes = cache.clear_pane_sources();
        if !visible {
            bytes.extend(cache.clear_bytes());
            return EncodedGraphics {
                bytes,
                incomplete: false,
            };
        }
        // Fork placement priority: a floating popup owns the picture layer;
        // otherwise the file manager's own preview owns the stage; only then
        // do terminal pane placements paint (TP fork surface rules).
        let placements = if let Some(popup) = collect_popup_pane_placements(
            app,
            graphics,
            terminal_runtimes,
            cell_size,
            &cache.images,
            true,
        ) {
            popup
        } else if files_on_stage {
            collect_file_manager_image_placement(app, cell_size, &cache.images)
                .into_iter()
                .collect()
        } else {
            collect_visible_placements(
                app,
                graphics,
                terminal_runtimes,
                surface,
                cell_size,
                &cache.images,
            )
        };
        let view_changed = cache.update_view(active_view_key(app));
        cache.reset_incremental_state();
        encode_terminal_graphics_update_legacy(
            &mut bytes,
            &placements,
            view_changed,
            replay_placements,
            cache,
        );
        return EncodedGraphics {
            bytes,
            incomplete: false,
        };
    }

    let live_pane_sources = graphics
        .slots
        .iter()
        .filter(|(_, slot)| slot.layer.is_some())
        .map(|((pane_id, layer_id), _)| HostSourceKey::PaneLayer {
            pane_id: *pane_id,
            layer_id: layer_id.clone(),
        })
        .collect::<HashSet<_>>();
    let placements = if visible {
        if let Some(popup) = collect_popup_pane_placements(
            app,
            graphics,
            terminal_runtimes,
            cell_size,
            &cache.images,
            true,
        ) {
            popup
        } else if files_on_stage {
            collect_file_manager_image_placement(app, cell_size, &cache.images)
                .into_iter()
                .collect()
        } else {
            collect_visible_placements(
                app,
                graphics,
                terminal_runtimes,
                surface,
                cell_size,
                &cache.images,
            )
        }
    } else {
        Vec::new()
    };
    cache.update_view(visible.then(|| active_view_key(app)).flatten());
    // A full re-blit overwrites Kitty placements with text, so that frame
    // must display cached images again even when nothing about them changed.
    // The re-seat request was filed at the top of this function, per cause:
    // a text-refresh-only frame blits just the changed status cells, leaves
    // every picture seated, and files none. TP-GFX-REPLAY-01
    encode_graphics_update_incremental(cache, &placements, &live_pane_sources, transaction_budget)
}

fn encode_terminal_graphics_update_legacy(
    bytes: &mut Vec<u8>,
    placements: &[HostPlacement],
    view_changed: bool,
    replay_placements: bool,
    cache: &mut HostGraphicsCache,
) {
    let current_sources = placements
        .iter()
        .filter(|placement| matches!(placement.source_key, HostSourceKey::Terminal { .. }))
        .map(|placement| placement.source_key.clone())
        .collect::<HashSet<_>>();
    cache
        .sources
        .retain(|source, _| current_sources.contains(source));

    let mut current_placements = HashSet::new();
    for placement in placements {
        let Some((clipped, format_code)) = clipped_placement(placement) else {
            continue;
        };
        let host_id = placement
            .host_image_id
            .unwrap_or_else(|| host_image_id(placement.pane_id, &placement.placement));
        let placement_id = host_placement_id(&placement.source_key, &placement.placement);
        let image_signature = image_signature(placement, format_code);
        let placement_signature =
            placement_signature(clipped, placement.placement.z, placement.scrollback_offset);
        let placement_key = (host_id, placement_id);
        current_placements.insert(placement_key);

        match cache.images.get(&host_id).copied() {
            Some(existing) if existing == image_signature => {}
            Some(_) => {
                // An assigned identity is refreshed in place: kitty replaces
                // the image when the same id is retransmitted, and a delete
                // here blanks the live picture between frames.
                if placement.host_image_id.is_none() {
                    encode_delete_image(bytes, host_id);
                    cache.placements.retain(|(image_id, id), _| {
                        if *image_id == host_id {
                            current_placements.remove(&(*image_id, *id));
                            false
                        } else {
                            true
                        }
                    });
                }
                if !encode_upload_image(bytes, placement, format_code, host_id) {
                    continue;
                }
                cache.images.insert(host_id, image_signature);
            }
            None => {
                if !encode_upload_image(bytes, placement, format_code, host_id) {
                    continue;
                }
                cache.images.insert(host_id, image_signature);
            }
        }

        release_superseded_terminal_image_legacy(
            bytes,
            cache,
            &mut current_placements,
            placement.source_key.clone(),
            host_id,
        );

        match cache.placements.get_mut(&placement_key) {
            Some(existing)
                if !view_changed && !replay_placements && *existing == placement_signature => {}
            Some(existing) => {
                encode_display_placement(
                    bytes,
                    clipped,
                    host_id,
                    placement_id,
                    placement.placement.z,
                );
                *existing = placement_signature;
            }
            None => {
                encode_display_placement(
                    bytes,
                    clipped,
                    host_id,
                    placement_id,
                    placement.placement.z,
                );
                cache.placements.insert(placement_key, placement_signature);
            }
        }
    }

    let stale = cache
        .placements
        .keys()
        .filter(|key| !current_placements.contains(key))
        .copied()
        .collect::<Vec<_>>();
    for (host_id, placement_id) in stale {
        encode_delete_placement(bytes, host_id, placement_id);
        cache.placements.remove(&(host_id, placement_id));
    }
    // Fork close-out: a single stale placement keeps its image cached (a
    // scrolled-away picture returns cheaply — the stale-placement proof pins
    // that), but when the surface has NOTHING left on screen the uploads go
    // with it, so closing Files leaves the host terminal and this cache
    // holding nothing (the FM upload-reuse proof pins this side).
    if view_changed && current_placements.is_empty() && cache.placements.is_empty() {
        let stale_images = cache.images.keys().copied().collect::<Vec<_>>();
        for host_id in stale_images {
            encode_delete_image(bytes, host_id);
            cache.images.remove(&host_id);
        }
    }
}

fn release_superseded_terminal_image_legacy(
    bytes: &mut Vec<u8>,
    cache: &mut HostGraphicsCache,
    current_placements: &mut HashSet<(u32, u32)>,
    source: HostSourceKey,
    host_id: u32,
) {
    let Some(previous) = cache.sources.insert(source, host_id) else {
        return;
    };
    if previous == host_id || cache.sources.values().any(|id| *id == previous) {
        return;
    }
    encode_delete_image(bytes, previous);
    cache.images.remove(&previous);
    cache.placements.retain(|(image_id, placement_id), _| {
        if *image_id == previous {
            current_placements.remove(&(*image_id, *placement_id));
            false
        } else {
            true
        }
    });
}

fn encode_graphics_update_incremental(
    cache: &mut HostGraphicsCache,
    placements: &[HostPlacement],
    live_pane_sources: &HashSet<HostSourceKey>,
    transaction_budget: Option<usize>,
) -> EncodedGraphics {
    let desired_sources = placements
        .iter()
        .map(|placement| placement.source_key.clone())
        .collect::<HashSet<_>>();
    let desired_placements = placements
        .iter()
        .filter_map(|placement| {
            clipped_placement(placement).map(|_| {
                let host_id = placement
                    .host_image_id
                    .unwrap_or_else(|| host_image_id(placement.pane_id, &placement.placement));
                (
                    host_id,
                    host_placement_id(&placement.source_key, &placement.placement),
                )
            })
        })
        .collect::<HashSet<_>>();
    let start = cache
        .continuation
        .as_ref()
        .and_then(|(source, id, _)| {
            placements
                .iter()
                .position(|placement| placement_identity(placement) == (source.clone(), *id))
        })
        .map(|index| index + 1)
        .or_else(|| cache.continuation.as_ref().map(|cursor| cursor.2))
        .map_or(0, |index| index % placements.len().max(1));
    let mut bytes = Vec::new();
    let mut emitted = false;

    let mut dead_sources = cache
        .sources
        .keys()
        .filter(|source| {
            matches!(source, HostSourceKey::PaneLayer { .. })
                && !live_pane_sources.contains(*source)
        })
        .cloned()
        .collect::<Vec<_>>();
    dead_sources.sort_by_key(source_order);
    for source in dead_sources {
        let host_id = cache.sources[&source];
        let last_reference = !cache
            .sources
            .iter()
            .any(|(other, id)| *other != source && *id == host_id);
        if emitted && last_reference {
            return EncodedGraphics {
                bytes,
                incomplete: true,
            };
        }
        cache.sources.remove(&source);
        if last_reference {
            encode_delete_image(&mut bytes, host_id);
            cache.images.remove(&host_id);
            cache.placements.retain(|(id, _), _| *id != host_id);
            cache.replayed_placements.retain(|(id, _)| *id != host_id);
            emitted = true;
        }
    }
    cache.sources.retain(|source, _| {
        matches!(source, HostSourceKey::PaneLayer { .. }) || desired_sources.contains(source)
    });
    cache
        .oversized
        .retain(|source, _| live_pane_sources.contains(source) || desired_sources.contains(source));

    let mut stale = cache
        .placements
        .keys()
        .filter(|key| !desired_placements.contains(key))
        .copied()
        .collect::<Vec<_>>();
    stale.sort_unstable();
    for key @ (host_id, placement_id) in stale {
        if emitted {
            return EncodedGraphics {
                bytes,
                incomplete: true,
            };
        }
        encode_delete_placement(&mut bytes, host_id, placement_id);
        cache.placements.remove(&key);
        cache.replayed_placements.remove(&key);
        emitted = true;
    }

    for offset in 0..placements.len() {
        let index = (start + offset) % placements.len();
        let placement = &placements[index];
        let signature = image_signature(placement, kitty_format_code(placement.placement.format));
        if transaction_budget.is_some()
            && cache.oversized.get(&placement.source_key) == Some(&signature)
        {
            continue;
        }
        cache.oversized.remove(&placement.source_key);
        let host_id = placement
            .host_image_id
            .unwrap_or_else(|| host_image_id(placement.pane_id, &placement.placement));
        if cache.images.get(&host_id) != Some(&signature)
            && !image_transaction_fits(placement, transaction_budget)
        {
            cache
                .oversized
                .insert(placement.source_key.clone(), signature);
            continue;
        }
        let mut candidate = cache.clone();
        let Some(transaction) = encode_placement_update(&mut candidate, placement) else {
            continue;
        };
        if transaction.is_empty() {
            *cache = candidate;
            continue;
        }
        if emitted {
            return EncodedGraphics {
                bytes,
                incomplete: true,
            };
        }
        *cache = candidate;
        let (source, id) = placement_identity(placement);
        cache.continuation = Some((source, id, (index + 1) % placements.len()));
        bytes = transaction;
        emitted = true;
    }

    cache.replay_placements = false;
    cache.replayed_placements.clear();
    EncodedGraphics {
        bytes,
        incomplete: false,
    }
}

fn image_transaction_fits(placement: &HostPlacement, budget: Option<usize>) -> bool {
    let Some(budget) = budget else {
        return true;
    };
    let data = placement.placement.data_len;
    let encoded = data.div_ceil(3).saturating_mul(4);
    let command_overhead = data.div_ceil(KITTY_CHUNK_BYTES).saturating_mul(16) + 1024;
    encoded.saturating_add(command_overhead) <= budget
}

fn placement_identity(placement: &HostPlacement) -> (HostSourceKey, u32) {
    (
        placement.source_key.clone(),
        host_placement_id(&placement.source_key, &placement.placement),
    )
}

fn source_order(source: &HostSourceKey) -> (u32, String) {
    match source {
        HostSourceKey::Terminal { pane_id, .. } => (pane_id.raw(), String::new()),
        HostSourceKey::PaneLayer { pane_id, layer_id } => (pane_id.raw(), layer_id.clone()),
    }
}

fn encode_placement_update(
    cache: &mut HostGraphicsCache,
    placement: &HostPlacement,
) -> Option<Vec<u8>> {
    let (clipped, format_code) = clipped_placement(placement)?;
    let host_id = placement
        .host_image_id
        .unwrap_or_else(|| host_image_id(placement.pane_id, &placement.placement));
    let placement_id = host_placement_id(&placement.source_key, &placement.placement);
    let key = (host_id, placement_id);
    let image_signature = image_signature(placement, format_code);
    let placement_signature =
        placement_signature(clipped, placement.placement.z, placement.scrollback_offset);
    let image_current = cache.images.get(&host_id) == Some(&image_signature);
    let placement_current = cache.placements.get(&key) == Some(&placement_signature)
        && (!cache.replay_placements || cache.replayed_placements.contains(&key));
    if image_current
        && placement_current
        && cache.sources.get(&placement.source_key) == Some(&host_id)
    {
        return None;
    }

    let mut bytes = Vec::new();
    let mut displayed = false;
    if !image_current {
        if cache.images.contains_key(&host_id)
            && (placement.host_image_id.is_some()
                || matches!(placement.source_key, HostSourceKey::PaneLayer { .. }))
        {
            if !encode_transmit_and_display(
                &mut bytes,
                placement,
                clipped,
                format_code,
                host_id,
                placement_id,
            ) {
                return None;
            }
            displayed = true;
        } else {
            if cache.images.contains_key(&host_id) {
                encode_delete_image(&mut bytes, host_id);
                cache.placements.retain(|(id, _), _| *id != host_id);
                cache.replayed_placements.retain(|(id, _)| *id != host_id);
            }
            if !encode_upload_image(&mut bytes, placement, format_code, host_id) {
                return None;
            }
        }
        cache.images.insert(host_id, image_signature);
    }

    release_superseded_source_image(&mut bytes, cache, placement.source_key.clone(), host_id);
    if !displayed && !placement_current {
        encode_display_placement(
            &mut bytes,
            clipped,
            host_id,
            placement_id,
            placement.placement.z,
        );
    }
    cache.placements.insert(key, placement_signature);
    if cache.replay_placements {
        cache.replayed_placements.insert(key);
    }
    Some(bytes)
}

fn release_superseded_source_image(
    bytes: &mut Vec<u8>,
    cache: &mut HostGraphicsCache,
    source: HostSourceKey,
    host_id: u32,
) {
    let Some(previous) = cache.sources.insert(source, host_id) else {
        return;
    };
    if previous == host_id || cache.sources.values().any(|id| *id == previous) {
        return;
    }
    encode_delete_image(bytes, previous);
    cache.images.remove(&previous);
    cache.placements.retain(|(id, _), _| *id != previous);
    cache.replayed_placements.retain(|(id, _)| *id != previous);
}

#[cfg(test)]
fn drain_graphics_updates(
    cache: &mut HostGraphicsCache,
    placements: &[HostPlacement],
    live: &HashSet<HostSourceKey>,
) -> Vec<u8> {
    let mut bytes = Vec::new();
    loop {
        let encoded = encode_graphics_update_incremental(cache, placements, live, None);
        bytes.extend(encoded.bytes);
        if !encoded.incomplete {
            return bytes;
        }
    }
}

#[cfg(test)]
fn encode_graphics_update(
    bytes: &mut Vec<u8>,
    placements: &[HostPlacement],
    replay: bool,
    images: &mut HashMap<u32, ImageSignature>,
    host_placements: &mut HashMap<(u32, u32), PlacementSignature>,
    sources: &mut HashMap<HostSourceKey, u32>,
) {
    let mut cache = HostGraphicsCache {
        images: std::mem::take(images),
        placements: std::mem::take(host_placements),
        sources: std::mem::take(sources),
        ..HostGraphicsCache::default()
    };
    let mut live = cache
        .sources
        .keys()
        .filter(|source| matches!(source, HostSourceKey::PaneLayer { .. }))
        .cloned()
        .collect::<HashSet<_>>();
    live.extend(
        placements
            .iter()
            .filter(|placement| matches!(placement.source_key, HostSourceKey::PaneLayer { .. }))
            .map(|placement| placement.source_key.clone()),
    );
    if live.is_empty() {
        encode_terminal_graphics_update_legacy(bytes, placements, replay, replay, &mut cache);
    } else {
        if replay {
            cache.request_placement_replay();
        }
        bytes.extend(drain_graphics_updates(&mut cache, placements, &live));
    }
    *images = cache.images;
    *host_placements = cache.placements;
    *sources = cache.sources;
}

pub(crate) fn clear_all_host_graphics() -> io::Result<()> {
    let cache = LOCAL_HOST_GRAPHICS.get_or_init(|| Mutex::new(HostGraphicsCache::default()));
    let mut bytes = Vec::new();
    if let Ok(mut cache) = cache.lock() {
        bytes = cache.clear_bytes();
    }
    if bytes.is_empty() {
        return Ok(());
    }
    let mut stdout = io::stdout().lock();
    stdout.write_all(&bytes)?;
    stdout.flush()
}

impl HostGraphicsCache {
    /// Test-only: the caches an encode leaves behind, all empty at once.
    #[cfg(test)]
    pub(crate) fn is_empty(&self) -> bool {
        self.images.is_empty()
            && self.placements.is_empty()
            && self.sources.is_empty()
            && self.replayed_placements.is_empty()
    }

    /// Test-only ledger view: which (image, placement) pairs the server still
    /// believes exist on the outer terminal. Lets integration tests compare a
    /// simulated kitty against this accounting to catch stranded placements.
    #[cfg(test)]
    pub(crate) fn test_placement_keys(&self) -> Vec<(u32, u32)> {
        self.placements.keys().copied().collect()
    }

    fn clear_pane_sources(&mut self) -> Vec<u8> {
        let pane_sources = self
            .sources
            .keys()
            .filter(|source| matches!(source, HostSourceKey::PaneLayer { .. }))
            .cloned()
            .collect::<Vec<_>>();
        let mut removed_images = HashSet::new();
        for source in pane_sources {
            if let Some(image_id) = self.sources.remove(&source) {
                removed_images.insert(image_id);
            }
            self.oversized.remove(&source);
        }

        let mut bytes = Vec::new();
        for image_id in removed_images {
            if self.sources.values().any(|id| *id == image_id) {
                continue;
            }
            encode_delete_image(&mut bytes, image_id);
            self.images.remove(&image_id);
            self.placements.retain(|(id, _), _| *id != image_id);
            self.replayed_placements.retain(|(id, _)| *id != image_id);
        }
        self.reset_incremental_state();
        bytes
    }

    fn has_pane_sources(&self) -> bool {
        self.sources
            .keys()
            .any(|source| matches!(source, HostSourceKey::PaneLayer { .. }))
    }

    fn reset_incremental_state(&mut self) {
        self.oversized.clear();
        self.continuation = None;
        self.replay_placements = false;
        self.replayed_placements.clear();
    }

    pub(crate) fn trust_pane_layer(
        &mut self,
        key: &crate::app::pane_graphics::Key,
        host_id: u32,
        layer: &crate::app::pane_graphics::Layer,
    ) {
        let source = HostSourceKey::PaneLayer {
            pane_id: key.0,
            layer_id: key.1.clone(),
        };
        self.oversized.remove(&source);
        self.sources.insert(source, host_id);
        self.images
            .insert(host_id, pane_layer_image_signature(layer));
    }

    pub(crate) fn forget_pane_layer(&mut self, key: &crate::app::pane_graphics::Key, host_id: u32) {
        let source = HostSourceKey::PaneLayer {
            pane_id: key.0,
            layer_id: key.1.clone(),
        };
        self.sources.remove(&source);
        self.oversized.remove(&source);
        self.images.remove(&host_id);
        self.placements.retain(|(id, _), _| *id != host_id);
        self.replayed_placements.retain(|(id, _)| *id != host_id);
    }

    pub(crate) fn request_placement_replay(&mut self) {
        if !self.replay_placements {
            self.replay_placements = true;
            self.replayed_placements.clear();
        }
    }

    #[cfg(test)]
    fn hide_except_live_pane_layers(&mut self, live: &HashSet<HostSourceKey>) -> Vec<u8> {
        drain_graphics_updates(self, &[], live)
    }

    #[cfg(test)]
    pub(crate) fn test_image_count(&self) -> usize {
        self.images.len()
    }

    #[cfg(test)]
    pub(crate) fn test_mark_pane_layer_entry(&mut self) {
        self.images.insert(
            PANE_LAYER_HOST_ID_FLOOR + 1,
            ImageSignature {
                image_width: 1,
                image_height: 1,
                format_code: 32,
                data_len: 4,
                data_fingerprint: 2,
            },
        );
    }

    #[cfg(test)]
    pub(crate) fn test_mark_non_empty(&mut self) {
        self.images.insert(
            HOST_IMAGE_ID_BASE,
            ImageSignature {
                image_width: 1,
                image_height: 1,
                format_code: 32,
                data_len: 4,
                data_fingerprint: 1,
            },
        );
        self.placements.insert(
            (HOST_IMAGE_ID_BASE, 1),
            PlacementSignature {
                x: 0,
                y: 0,
                cols: 1,
                rows: 1,
                source_x: 0,
                source_y: 0,
                source_width: 1,
                source_height: 1,
                x_offset: 0,
                y_offset: 0,
                z: 0,
                scrollback_offset: 0,
            },
        );
    }

    /// TP-GFX-RESIZE-01: a geometry change sweeps only the pictures a
    /// repaint cannot rebuild — the terminal-native ones, below the
    /// pane-layer floor. Stream entries stay: their road replays placements
    /// without retransmitting, and deleting them here would force the very
    /// retransmit that road exists to avoid.
    pub(crate) fn clear_terminal_native_bytes(&mut self) -> Vec<u8> {
        let mut bytes = Vec::new();
        let native: Vec<u32> = self
            .images
            .keys()
            .copied()
            .filter(|id| *id < PANE_LAYER_HOST_ID_FLOOR)
            .collect();
        for id in native {
            encode_delete_image(&mut bytes, id);
            self.images.remove(&id);
            self.placements.retain(|(image, _), _| *image != id);
            self.sources.retain(|_, host| *host != id);
        }
        if self
            .continuation
            .as_ref()
            .is_some_and(|(_, id, _)| *id < PANE_LAYER_HOST_ID_FLOOR)
        {
            self.continuation = None;
        }
        bytes
    }

    pub(crate) fn clear_bytes(&mut self) -> Vec<u8> {
        let mut bytes = Vec::new();
        for id in self.images.keys().copied().collect::<Vec<_>>() {
            encode_delete_image(&mut bytes, id);
        }
        self.images.clear();
        self.placements.clear();
        self.sources.clear();
        self.reset_incremental_state();
        self.view = None;
        bytes
    }

    fn update_view(&mut self, view_key: Option<HostViewKey>) -> bool {
        if self.view == view_key {
            return false;
        }
        self.view = view_key;
        self.continuation = None;
        true
    }
}

fn active_view_key(app: &AppState) -> Option<HostViewKey> {
    // Keyed on stage ownership so the cache invalidates when the surface the
    // images belong to changes, rather than when a background tab happens to
    // exist.
    if app.staged_file_manager().is_some() {
        let workspace_index = app.active.unwrap_or(usize::MAX);
        let tab_index = app
            .active
            .and_then(|index| app.workspaces.get(index))
            .map(crate::workspace::Workspace::active_tab_index)
            .unwrap_or(usize::MAX);
        return Some(HostViewKey {
            workspace_index,
            tab_index,
            file_manager_open: true,
        });
    }
    let ws_idx = app.active?;
    let ws = app.workspaces.get(ws_idx)?;
    Some(HostViewKey {
        workspace_index: ws_idx,
        tab_index: ws.active_tab_index(),
        file_manager_open: false,
    })
}

/// Pictures drawn by the popup pane, when one is up.
///
/// The popup floats above everything and is not a member of the tab surface,
/// so the pane loop below never sees it. Without this a `herdr view` — or any
/// program drawing with the kitty protocol — inside a popup showed its text
/// and nothing else: the file it was opened on was the one thing missing.
///
/// `None` means no popup, which is different from a popup that happens to be
/// drawing nothing: while one is up it OWNS the picture layer, and the
/// surface underneath must place nothing, or that image would paint straight
/// across the popup it is supposed to be behind.
fn collect_popup_pane_placements(
    app: &AppState,
    graphics: &crate::app::pane_graphics::Runtime,
    terminal_runtimes: &TerminalRuntimeRegistry,
    cell_size: HostCellSize,
    uploaded_images: &HashMap<u32, ImageSignature>,
    include_data: bool,
) -> Option<Vec<HostPlacement>> {
    let popup = app.popup_pane.as_ref()?;
    // The same geometry the popup is drawn with, from the same helper, so the
    // picture cannot land anywhere other than inside its frame.
    let (_outer, inner) = crate::ui::popup_pane_rects(app, app.view.terminal_area)?;
    let mut placements = Vec::new();
    let mut popup_layers = graphics
        .slots
        .iter()
        .filter_map(|((pane_id, layer_id), slot)| {
            (*pane_id == popup.pane_id)
                .then(|| {
                    slot.layer
                        .as_ref()
                        .map(|layer| (layer_id, slot.host_image_id, layer))
                })
                .flatten()
        })
        .collect::<Vec<_>>();
    popup_layers.sort_by_key(|(layer_id, _, layer)| (layer.z_index, layer_id.as_str()));
    for (layer_id, host_id, layer) in popup_layers {
        placements.push(pane_graphics_host_placement_at(
            popup.pane_id,
            inner,
            layer_id,
            host_id,
            cell_size,
            layer,
            uploaded_images,
            include_data,
        ));
    }
    let Some(runtime) = terminal_runtimes.get(&popup.terminal_id) else {
        return Some(placements);
    };
    let scrollback_offset = runtime
        .scroll_metrics()
        .map(|m| m.offset_from_bottom as u32)
        .unwrap_or(0);
    for placement in runtime.kitty_image_placements_with_data_filter(|descriptor| {
        if !include_data {
            return false;
        }
        let format_code = kitty_format_code(descriptor.format);
        let signature = image_signature_from_descriptor(descriptor, format_code);
        let host_id = stream_host_image_id(popup.pane_id, descriptor.image_id);
        uploaded_images.get(&host_id).copied() != Some(signature)
    }) {
        placements.push(HostPlacement {
            host_image_id: Some(stream_host_image_id(popup.pane_id, placement.image_id)),
            pane_id: popup.pane_id,
            area: inner,
            cell_size,
            source_key: HostSourceKey::Terminal {
                pane_id: popup.pane_id,
                image_id: placement.image_id,
            },
            placement,
            scrollback_offset,
        });
    }
    Some(placements)
}

fn collect_visible_placements(
    app: &AppState,
    graphics: &crate::app::pane_graphics::Runtime,
    terminal_runtimes: &TerminalRuntimeRegistry,
    surface: crate::ui::TabSurfaceView<'_>,
    cell_size: HostCellSize,
    uploaded_images: &HashMap<u32, ImageSignature>,
) -> Vec<HostPlacement> {
    let ws_idx = match app.active {
        Some(idx) => idx,
        None => {
            tracing::debug!("collect_visible_placements: no active workspace");
            return Vec::new();
        }
    };
    if app
        .workspaces
        .get(ws_idx)
        .and_then(crate::workspace::Workspace::active_tab)
        .is_none()
    {
        tracing::debug!(ws_idx, "collect_visible_placements: no active tab");
        return Vec::new();
    }

    tracing::debug!(
        ws_idx,
        terminal_runtimes_len = terminal_runtimes.len(),
        pane_infos_len = surface.pane_infos.len(),
        "collect_visible_placements: starting iteration"
    );
    let mut placements = Vec::new();
    for info in surface.pane_infos {
        let mut pane_layers = graphics
            .slots
            .iter()
            .filter_map(|((pane_id, layer_id), slot)| {
                (*pane_id == info.id)
                    .then(|| {
                        slot.layer
                            .as_ref()
                            .map(|layer| (layer_id, slot.host_image_id, layer))
                    })
                    .flatten()
            })
            .collect::<Vec<_>>();
        pane_layers.sort_by_key(|(layer_id, _, layer)| (layer.z_index, layer_id.as_str()));
        for (layer_id, host_image_id, layer) in pane_layers {
            placements.push(pane_graphics_host_placement(
                info,
                layer_id,
                host_image_id,
                cell_size,
                layer,
                uploaded_images,
                true,
            ));
        }

        let runtime = match app.runtime_for_pane_in_workspace(terminal_runtimes, ws_idx, info.id) {
            Some(rt) => rt,
            None => {
                tracing::debug!(pane_id = ?info.id, "collect_visible_placements: runtime not found");
                continue;
            }
        };
        for placement in runtime.kitty_image_placements_with_data_filter(|descriptor| {
            let format_code = kitty_format_code(descriptor.format);
            let signature = image_signature_from_descriptor(descriptor, format_code);
            let host_id = stream_host_image_id(info.id, descriptor.image_id);
            uploaded_images.get(&host_id).copied() != Some(signature)
        }) {
            let scrollback_offset = runtime
                .scroll_metrics()
                .map(|m| m.offset_from_bottom as u32)
                .unwrap_or(0);
            placements.push(HostPlacement {
                pane_id: info.id,
                host_image_id: Some(stream_host_image_id(info.id, placement.image_id)),
                area: info.inner_rect,
                cell_size,
                source_key: HostSourceKey::Terminal {
                    pane_id: info.id,
                    image_id: placement.image_id,
                },
                placement,
                scrollback_offset,
            });
        }
    }
    tracing::debug!(
        placements_len = placements.len(),
        "collect_visible_placements: done"
    );
    placements
}

fn pane_graphics_host_placement(
    info: &PaneInfo,
    layer_id: &str,
    host_id: u32,
    cell_size: HostCellSize,
    layer: &crate::app::pane_graphics::Layer,
    uploaded_images: &HashMap<u32, ImageSignature>,
    include_data: bool,
) -> HostPlacement {
    pane_graphics_host_placement_at(
        info.id,
        info.inner_rect,
        layer_id,
        host_id,
        cell_size,
        layer,
        uploaded_images,
        include_data,
    )
}

/// The pane-layer placement for one pane identified by id and rect.
///
/// Split out from [`pane_graphics_host_placement`] because the popup pane is
/// not a member of the tab surface — it has no `PaneInfo` — yet it draws into
/// a rect and owns pictures exactly like any other pane.
fn pane_graphics_host_placement_at(
    pane_id: PaneId,
    inner_rect: Rect,
    layer_id: &str,
    host_id: u32,
    cell_size: HostCellSize,
    layer: &crate::app::pane_graphics::Layer,
    uploaded_images: &HashMap<u32, ImageSignature>,
    include_data: bool,
) -> HostPlacement {
    let format = pane_graphics_kitty_format(layer.format);
    let signature = pane_layer_image_signature(layer);
    let data = if !include_data || uploaded_images.get(&host_id).copied() == Some(signature) {
        Vec::new()
    } else {
        layer.inline_data().map(<[u8]>::to_vec).unwrap_or_default()
    };
    let render = layer.render;
    let grid_cols = if render.grid_cols == 0 {
        u32::from(inner_rect.width)
    } else {
        render.grid_cols
    };
    let grid_rows = if render.grid_rows == 0 {
        u32::from(inner_rect.height)
    } else {
        render.grid_rows
    };

    HostPlacement {
        pane_id,
        host_image_id: Some(host_id),
        area: inner_rect,
        cell_size,
        source_key: HostSourceKey::PaneLayer {
            pane_id,
            layer_id: layer_id.to_owned(),
        },
        scrollback_offset: 0,
        placement: KittyImagePlacement {
            image_id: 1,
            placement_id: 1,
            z: layer.z_index,
            x_offset: 0,
            y_offset: 0,
            image_width: layer.image_width,
            image_height: layer.image_height,
            format,
            data_len: layer.data_len(),
            data_fingerprint: layer.data_fingerprint,
            data,
            render: KittyPlacementRenderInfo {
                pixel_width: layer.image_width,
                pixel_height: layer.image_height,
                grid_cols,
                grid_rows,
                viewport_col: render.viewport_col,
                viewport_row: render.viewport_row,
                source_x: 0,
                source_y: 0,
                source_width: 0,
                source_height: 0,
            },
        },
    }
}

fn pane_graphics_kitty_format(format: crate::api::schema::PaneGraphicsFormat) -> KittyImageFormat {
    match format {
        crate::api::schema::PaneGraphicsFormat::Png => KittyImageFormat::Png,
        crate::api::schema::PaneGraphicsFormat::Rgb => KittyImageFormat::Rgb,
        crate::api::schema::PaneGraphicsFormat::Rgba
        | crate::api::schema::PaneGraphicsFormat::Bgra => KittyImageFormat::Rgba,
    }
}

fn host_image_id(pane_id: PaneId, placement: &KittyImagePlacement) -> u32 {
    let format_code = kitty_format_code(placement.format);
    host_image_id_for_signature(
        pane_id,
        ImageSignature {
            image_width: placement.image_width,
            image_height: placement.image_height,
            format_code,
            data_len: placement.data_len,
            data_fingerprint: placement.data_fingerprint,
        },
    )
}

fn host_image_id_for_signature(pane_id: PaneId, signature: ImageSignature) -> u32 {
    let mut hasher = DefaultHasher::new();
    pane_id.raw().hash(&mut hasher);
    signature.hash(&mut hasher);
    HOST_IMAGE_ID_BASE + ((hasher.finish() as u32) % 900_000)
}

fn stream_host_image_id(pane_id: PaneId, guest_image_id: u32) -> u32 {
    // TP-GFX-STABLE-01: identity follows the SOURCE, not the content. A
    // streaming pane repaints one guest image id with new pixels every frame;
    // content-hashing that into a fresh host id per frame made every lost
    // delete a permanently stranded placement. One (pane, guest image) pair
    // keeps one host image id for its whole life, and a content change is a
    // retransmit of that same id.
    let mut hasher = DefaultHasher::new();
    "stream.stable".hash(&mut hasher);
    pane_id.raw().hash(&mut hasher);
    guest_image_id.hash(&mut hasher);
    HOST_IMAGE_ID_BASE + ((hasher.finish() as u32) % 900_000)
}

fn host_placement_id(source_key: &HostSourceKey, placement: &KittyImagePlacement) -> u32 {
    let mut hasher = DefaultHasher::new();
    match source_key {
        HostSourceKey::Terminal { pane_id, .. } => pane_id.raw().hash(&mut hasher),
        HostSourceKey::PaneLayer { pane_id, layer_id } => {
            "pane.graphics".hash(&mut hasher);
            pane_id.raw().hash(&mut hasher);
            layer_id.hash(&mut hasher);
        }
    }
    placement.image_id.hash(&mut hasher);
    placement.placement_id.hash(&mut hasher);
    1 + ((hasher.finish() as u32) % 900_000)
}

pub(crate) struct DirectFileCommand {
    pub(crate) leading: Vec<u8>,
    pub(crate) control: String,
}

pub(crate) fn prepare_direct_file(
    app: &AppState,
    graphics: &crate::app::pane_graphics::Runtime,
    surface: crate::ui::TabSurfaceView<'_>,
    cell_size: HostCellSize,
    allow_placement: bool,
    cache: &HostGraphicsCache,
    key: &crate::app::pane_graphics::Key,
) -> Option<DirectFileCommand> {
    let slot = graphics.slots.get(key)?;
    let layer = slot.layer.as_ref()?;
    layer.direct_lease()?;

    let info = allow_placement
        .then(|| surface.pane_infos.iter().find(|info| info.id == key.0))
        .flatten()
        .filter(|_| app.mode == Mode::Terminal && cell_size.is_known() && app.active.is_some());
    if let Some(command) = info
        .map(|info| {
            pane_graphics_host_placement(
                info,
                &key.1,
                slot.host_image_id,
                cell_size,
                layer,
                &cache.images,
                false,
            )
        })
        .and_then(|placement| direct_file_command(&placement, slot.host_image_id))
        .map(|(command, _, _, _)| command)
    {
        return Some(command);
    }

    let inline_fallback_available = layer.data_len()
        <= crate::api::schema::PANE_GRAPHICS_STREAM_MAX_BYTES
        && graphics.can_store_inline(key, layer.data_len());
    (!inline_fallback_available).then(|| direct_file_upload_command(layer, slot.host_image_id))
}

fn direct_file_upload_command(
    layer: &crate::app::pane_graphics::Layer,
    host_image_id: u32,
) -> DirectFileCommand {
    DirectFileCommand {
        leading: Vec::new(),
        control: format!(
            "a=t,f=32,s={},v={},i={host_image_id},q=0",
            layer.image_width, layer.image_height
        ),
    }
}

fn direct_file_command(
    placement: &HostPlacement,
    host_image_id: u32,
) -> Option<(DirectFileCommand, ClippedPlacement, u32, u32)> {
    let (clipped, format_code) = clipped_placement(placement)?;
    let placement_id = host_placement_id(&placement.source_key, &placement.placement);
    let mut control = format!(
        "a=T,f={format_code},s={},v={},i={host_image_id},p={placement_id},c={},r={},z={},C=1,q=0",
        placement.placement.image_width,
        placement.placement.image_height,
        clipped.cols,
        clipped.rows,
        placement.placement.z,
    );
    append_placement_controls(&mut control, clipped);
    Some((
        DirectFileCommand {
            leading: format!("\x1b[{};{}H", clipped.y + 1, clipped.x + 1).into_bytes(),
            control,
        },
        clipped,
        format_code,
        placement_id,
    ))
}

#[cfg(unix)]
pub(crate) fn encode_kitty_regular_file(
    out: &mut Vec<u8>,
    leading: &[u8],
    control: &str,
    path: &str,
) {
    let payload = base64::engine::general_purpose::STANDARD.encode(path.as_bytes());
    out.extend_from_slice(b"\x1b7");
    out.extend_from_slice(leading);
    let _ = write!(out, "\x1b_G{control},t=f;{payload}\x1b\\");
    out.extend_from_slice(b"\x1b8");
}

fn encode_delete_image(out: &mut Vec<u8>, id: u32) {
    let _ = write!(out, "\x1b_Ga=d,d=I,i={id},q=2;\x1b\\");
}

fn encode_delete_placement(out: &mut Vec<u8>, host_id: u32, host_placement_id: u32) {
    let _ = write!(
        out,
        "\x1b_Ga=d,d=i,i={host_id},p={host_placement_id},q=2;\x1b\\"
    );
}

fn encode_upload_image(
    out: &mut Vec<u8>,
    placement: &HostPlacement,
    format_code: u32,
    host_id: u32,
) -> bool {
    if placement.placement.data.is_empty() {
        return false;
    }

    let control = format!(
        "a=t,t=d,f={format_code},s={},v={},i={host_id},q=2",
        placement.placement.image_width, placement.placement.image_height,
    );
    encode_kitty_data(out, &control, &placement.placement.data);
    true
}

fn encode_transmit_and_display(
    out: &mut Vec<u8>,
    placement: &HostPlacement,
    clipped: ClippedPlacement,
    format_code: u32,
    host_id: u32,
    host_placement_id: u32,
) -> bool {
    if placement.placement.data.is_empty() {
        return false;
    }
    let _ = write!(out, "\x1b[{};{}H", clipped.y + 1, clipped.x + 1);
    let mut control = format!(
        "a=T,t=d,f={format_code},s={},v={},i={host_id},p={host_placement_id},c={},r={},z={},C=1,q=2",
        placement.placement.image_width,
        placement.placement.image_height,
        clipped.cols,
        clipped.rows,
        placement.placement.z,
    );
    append_placement_controls(&mut control, clipped);
    encode_kitty_data(out, &control, &placement.placement.data);
    true
}

fn encode_display_placement(
    out: &mut Vec<u8>,
    clipped: ClippedPlacement,
    host_id: u32,
    host_placement_id: u32,
    z: i32,
) {
    let _ = write!(out, "\x1b[{};{}H", clipped.y + 1, clipped.x + 1);
    let mut control = format!(
        "a=p,i={host_id},p={host_placement_id},c={},r={},z={z},C=1,q=2",
        clipped.cols, clipped.rows,
    );
    append_placement_controls(&mut control, clipped);
    let _ = write!(out, "\x1b_G{control};\x1b\\");
}

fn append_placement_controls(control: &mut String, clipped: ClippedPlacement) {
    if clipped.source_x > 0 {
        let _ = write!(control, ",x={}", clipped.source_x);
    }
    if clipped.source_y > 0 {
        let _ = write!(control, ",y={}", clipped.source_y);
    }
    if clipped.source_width > 0 {
        let _ = write!(control, ",w={}", clipped.source_width);
    }
    if clipped.source_height > 0 {
        let _ = write!(control, ",h={}", clipped.source_height);
    }
    if clipped.x_offset > 0 {
        let _ = write!(control, ",X={}", clipped.x_offset);
    }
    if clipped.y_offset > 0 {
        let _ = write!(control, ",Y={}", clipped.y_offset);
    }
}

fn clipped_placement(placement: &HostPlacement) -> Option<(ClippedPlacement, u32)> {
    if placement.area.width == 0 || placement.area.height == 0 {
        tracing::debug!(
            area_w = placement.area.width,
            area_h = placement.area.height,
            "clipped_placement: area zero"
        );
        return None;
    }
    let render = placement.placement.render;
    if render.grid_cols == 0 || render.grid_rows == 0 {
        tracing::debug!(
            grid_cols = render.grid_cols,
            grid_rows = render.grid_rows,
            "clipped_placement: grid zero"
        );
        return None;
    }
    let format_code = kitty_format_code(placement.placement.format);

    let left_clip_cells = if render.viewport_col < 0 {
        render.viewport_col.saturating_neg() as u32
    } else {
        0
    };
    let top_clip_cells = if render.viewport_row < 0 {
        render.viewport_row.saturating_neg() as u32
    } else {
        0
    };
    let viewport_col = render.viewport_col.max(0) as u32;
    let viewport_row = render.viewport_row.max(0) as u32;
    tracing::debug!(
        viewport_col = viewport_col,
        viewport_row = viewport_row,
        area_w = placement.area.width,
        area_h = placement.area.height,
        scrollback_offset = placement.scrollback_offset,
        raw_viewport_row = render.viewport_row,
        cond1 = viewport_col >= placement.area.width as u32,
        cond2 = viewport_row >= placement.area.height as u32,
        "clipped_placement: viewport check"
    );
    if viewport_col >= placement.area.width as u32 || viewport_row >= placement.area.height as u32 {
        return None;
    }

    let visible_cols = render
        .grid_cols
        .saturating_sub(left_clip_cells)
        .min(placement.area.width as u32 - viewport_col);
    let visible_rows = render
        .grid_rows
        .saturating_sub(top_clip_cells)
        .min(placement.area.height as u32 - viewport_row);
    tracing::debug!(
        visible_cols = visible_cols,
        visible_rows = visible_rows,
        left_clip_cells = left_clip_cells,
        top_clip_cells = top_clip_cells,
        "clipped_placement: visible dims check"
    );
    if visible_cols == 0 || visible_rows == 0 {
        return None;
    }

    let source_width = if render.source_width == 0 {
        placement.placement.image_width
    } else {
        render.source_width
    };
    let source_height = if render.source_height == 0 {
        placement.placement.image_height
    } else {
        render.source_height
    };
    let pixel_width = render
        .pixel_width
        .max(
            render
                .grid_cols
                .saturating_mul(placement.cell_size.width_px),
        )
        .max(1);
    let pixel_height = render
        .pixel_height
        .max(
            render
                .grid_rows
                .saturating_mul(placement.cell_size.height_px),
        )
        .max(1);

    let crop_left_px = left_clip_cells.saturating_mul(placement.cell_size.width_px);
    let crop_top_px = top_clip_cells.saturating_mul(placement.cell_size.height_px);
    let visible_width_px = visible_cols.saturating_mul(placement.cell_size.width_px);
    let visible_height_px = visible_rows.saturating_mul(placement.cell_size.height_px);

    let source_x = render.source_x + scale_pixels(crop_left_px, source_width, pixel_width);
    let source_y = render.source_y + scale_pixels(crop_top_px, source_height, pixel_height);
    let source_width = scale_pixels(visible_width_px, source_width, pixel_width)
        .max(1)
        .min(placement.placement.image_width.saturating_sub(source_x));
    let source_height = scale_pixels(visible_height_px, source_height, pixel_height)
        .max(1)
        .min(placement.placement.image_height.saturating_sub(source_y));

    if source_width == 0 || source_height == 0 {
        tracing::debug!(
            source_width = source_width,
            source_height = source_height,
            image_width = placement.placement.image_width,
            image_height = placement.placement.image_height,
            "clipped_placement: source dims zero"
        );
        return None;
    }

    tracing::debug!("clipped_placement: success");
    Some((
        ClippedPlacement {
            x: placement.area.x + viewport_col as u16,
            y: placement.area.y + viewport_row as u16,
            cols: visible_cols,
            rows: visible_rows,
            source_x,
            source_y,
            source_width,
            source_height,
            x_offset: if left_clip_cells == 0 {
                placement.placement.x_offset
            } else {
                0
            },
            y_offset: if top_clip_cells == 0 {
                placement.placement.y_offset
            } else {
                0
            },
        },
        format_code,
    ))
}

fn scale_pixels(value: u32, source: u32, dest: u32) -> u32 {
    ((value as u64).saturating_mul(source as u64) / dest.max(1) as u64).min(u32::MAX as u64) as u32
}

fn pane_layer_image_signature(layer: &crate::app::pane_graphics::Layer) -> ImageSignature {
    ImageSignature {
        image_width: layer.image_width,
        image_height: layer.image_height,
        format_code: kitty_format_code(pane_graphics_kitty_format(layer.format)),
        data_len: layer.data_len(),
        data_fingerprint: layer.data_fingerprint,
    }
}

fn image_signature(placement: &HostPlacement, format_code: u32) -> ImageSignature {
    ImageSignature {
        image_width: placement.placement.image_width,
        image_height: placement.placement.image_height,
        format_code,
        data_len: placement.placement.data_len,
        data_fingerprint: placement.placement.data_fingerprint,
    }
}

fn image_signature_from_descriptor(
    descriptor: KittyImageDescriptor,
    format_code: u32,
) -> ImageSignature {
    ImageSignature {
        image_width: descriptor.image_width,
        image_height: descriptor.image_height,
        format_code,
        data_len: descriptor.data_len,
        data_fingerprint: descriptor.data_fingerprint,
    }
}

fn placement_signature(
    clipped: ClippedPlacement,
    z: i32,
    scrollback_offset: u32,
) -> PlacementSignature {
    PlacementSignature {
        x: clipped.x,
        y: clipped.y,
        cols: clipped.cols,
        rows: clipped.rows,
        source_x: clipped.source_x,
        source_y: clipped.source_y,
        source_width: clipped.source_width,
        source_height: clipped.source_height,
        x_offset: clipped.x_offset,
        y_offset: clipped.y_offset,
        z,
        scrollback_offset,
    }
}

fn kitty_format_code(format: KittyImageFormat) -> u32 {
    match format {
        KittyImageFormat::Rgb => 24,
        KittyImageFormat::Rgba => 32,
        KittyImageFormat::Png => 100,
    }
}

/// Write a Kitty command whose payload is chunked base64, to any sink.
///
/// Shared with the standalone viewer: the chunking rule is the protocol's, not
/// the file manager's, and writing it twice would mean two places to get the
/// `m=` continuation flag wrong.
pub(crate) fn encode_kitty_data_to(
    out: &mut impl std::io::Write,
    control: &str,
    data: &[u8],
) -> std::io::Result<()> {
    let mut buffer = Vec::new();
    encode_kitty_data(&mut buffer, control, data);
    out.write_all(&buffer)
}

/// Deflate a payload worth deflating, or leave it alone (TP-GFX-ZLIB-01).
///
/// The whole payload is one zlib stream, produced before any chunking: a
/// terminal inflates once, after the last chunk arrives, so compressing chunk
/// by chunk would hand it a stream that decodes as garbage.
fn compress_kitty_payload(data: &[u8]) -> Option<Vec<u8>> {
    if data.len() < KITTY_COMPRESSION_MIN_BYTES
        || !KITTY_PAYLOAD_COMPRESSION.load(Ordering::Acquire)
    {
        return None;
    }
    let mut encoder = flate2::write::ZlibEncoder::new(
        Vec::with_capacity(data.len() / 8),
        flate2::Compression::new(KITTY_COMPRESSION_LEVEL),
    );
    if encoder.write_all(data).is_err() {
        return None;
    }
    // A payload that grew is a payload not worth announcing as compressed.
    encoder
        .finish()
        .ok()
        .filter(|deflated| deflated.len() < data.len())
}

fn encode_kitty_data(out: &mut Vec<u8>, control: &str, data: &[u8]) {
    let deflated = compress_kitty_payload(data);
    let payload = deflated.as_deref().unwrap_or(data);
    let control = if deflated.is_some() {
        std::borrow::Cow::Owned(format!("{control},o=z"))
    } else {
        std::borrow::Cow::Borrowed(control)
    };

    let mut chunks = payload.chunks(KITTY_CHUNK_BYTES).peekable();
    let Some(first) = chunks.next() else {
        return;
    };
    let more = if chunks.peek().is_some() { 1 } else { 0 };
    let encoded = base64::engine::general_purpose::STANDARD.encode(first);
    let _ = write!(out, "\x1b_G{control},m={more};{encoded}\x1b\\");

    while let Some(chunk) = chunks.next() {
        let more = if chunks.peek().is_some() { 1 } else { 0 };
        let encoded = base64::engine::general_purpose::STANDARD.encode(chunk);
        let _ = write!(out, "\x1b_Gm={more};{encoded}\x1b\\");
    }
}

#[cfg(test)]
fn inflate_for_test(bytes: &[u8]) -> Vec<u8> {
    use std::io::Read as _;
    let mut out = Vec::new();
    flate2::read::ZlibDecoder::new(bytes)
        .read_to_end(&mut out)
        .expect("the wire carries one inflatable zlib stream");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fm::image_preview::{ImagePreviewTarget, PreparedImagePreview};

    // ---- TP-GFX-ZLIB-01: remote-friendly payloads -----------------------
    //
    // A browser pane measured on 2026-08-25 cost a remote client 25 MB for one
    // click: full RGBA frames, base64, no compression. The pixels compress
    // 147x at zlib level 1 in 3 ms, so the bytes were not the price of the
    // picture — they were the price of not compressing it.

    /// Every payload the encoder emitted, reassembled: (control of the first
    /// chunk, the base64-decoded bytes of all of them).
    fn decode_kitty_stream(bytes: &[u8]) -> (String, Vec<u8>) {
        let text = String::from_utf8_lossy(bytes).into_owned();
        let mut control = String::new();
        let mut payload = Vec::new();
        for (index, raw) in text.split("\u{1b}_G").skip(1).enumerate() {
            let body = raw.strip_suffix("\u{1b}\\").unwrap_or(raw);
            let (head, data) = body.split_once(';').expect("kitty command has a payload");
            if index == 0 {
                control = head.to_owned();
            } else {
                assert!(
                    head.starts_with("m="),
                    "continuation chunks carry only the continuation flag: {head}"
                );
            }
            payload.extend(
                base64::engine::general_purpose::STANDARD
                    .decode(data)
                    .expect("chunk is valid base64"),
            );
        }
        (control, payload)
    }

    fn compressible_pixels(bytes: usize) -> Vec<u8> {
        // A screenshot is mostly flat colour, which is why it compresses; a
        // random buffer would measure the opposite of the real workload.
        (0..bytes).map(|i| ((i / 4096) % 251) as u8).collect()
    }

    /// U1: the wire carries `o=z` and the terminal can get the pixels back.
    /// Smaller output alone proves nothing — the only contract that matters is
    /// that inflating what we sent returns exactly what we were given.
    // TP-GFX-RESET-01
    #[test]
    fn reset_barrier_wipes_unknown_images_too() {
        let mut cache = HostGraphicsCache::default();
        cache.test_mark_non_empty();
        let bytes = reset_barrier_bytes(&mut cache);
        assert_eq!(bytes, KITTY_DELETE_ALL.to_vec());
        assert!(cache.is_empty());
    }

    #[test]
    fn large_payload_is_compressed_and_inflates_to_the_original() {
        let pixels = compressible_pixels(512 * 1024);
        let mut out = Vec::new();
        encode_kitty_data(&mut out, "a=t,t=d,f=32,s=100,v=100,i=7,q=2", &pixels);

        let (control, wire) = decode_kitty_stream(&out);
        assert!(
            control.contains("o=z"),
            "control announces compression: {control}"
        );
        let inflated = inflate_for_test(&wire);
        assert_eq!(
            inflated, pixels,
            "the terminal gets the original pixels back"
        );
        assert!(
            wire.len() * 4 < pixels.len(),
            "compressed {} bytes is not smaller than {} raw",
            wire.len(),
            pixels.len()
        );
    }

    /// U2: compression covers the whole payload before it is split. Ghostty
    /// inflates once, after the last chunk arrives; compressing per chunk
    /// produces a stream that decodes as garbage.
    #[test]
    fn compression_covers_the_whole_payload_before_chunking() {
        let pixels = compressible_pixels(512 * 1024);
        let mut out = Vec::new();
        encode_kitty_data(&mut out, "a=t,t=d,f=32,s=100,v=100,i=7,q=2", &pixels);

        let text = String::from_utf8_lossy(&out);
        let chunks = text.matches("\u{1b}_G").count();
        assert!(chunks > 1, "a payload this size is chunked: {chunks}");
        let (_, wire) = decode_kitty_stream(&out);
        // One zlib stream, not one per chunk: inflating the concatenation
        // succeeds and consumes everything.
        assert_eq!(inflate_for_test(&wire), pixels);
        assert_eq!(
            text.matches("o=z").count(),
            1,
            "only the first chunk carries the control keys"
        );
    }

    /// U3: a small payload is left alone. Paying zlib for a few kilobytes
    /// spends CPU on every frame to save bytes nobody notices.
    #[test]
    fn small_payload_is_left_uncompressed() {
        let pixels = compressible_pixels(4 * 1024);
        let mut out = Vec::new();
        encode_kitty_data(&mut out, "a=t,t=d,f=32,s=8,v=8,i=7,q=2", &pixels);

        let (control, wire) = decode_kitty_stream(&out);
        assert!(
            !control.contains("o=z"),
            "no compression announced: {control}"
        );
        assert_eq!(wire, pixels, "the payload travels as-is");
    }

    /// U4: the switch is exact. A kill switch that changes the output in any
    /// other way is not a kill switch.
    #[test]
    fn disabled_compression_is_byte_for_byte_the_old_output() {
        let pixels = compressible_pixels(512 * 1024);
        let control = "a=t,t=d,f=32,s=100,v=100,i=7,q=2";

        set_kitty_payload_compression(false);
        let mut plain = Vec::new();
        encode_kitty_data(&mut plain, control, &pixels);
        set_kitty_payload_compression(true);
        let mut compressed = Vec::new();
        encode_kitty_data(&mut compressed, control, &pixels);
        set_kitty_payload_compression(true);

        let (plain_control, plain_wire) = decode_kitty_stream(&plain);
        assert!(!plain_control.contains("o=z"));
        assert_eq!(plain_wire, pixels);
        assert!(
            plain.len() > compressed.len(),
            "disabling costs the old bytes"
        );
    }

    /// U5: compression changes the transport, never the picture. `s=`/`v=`
    /// describe the pixels, not the bytes, and Ghostty sizes the image from
    /// them.
    #[test]
    fn geometry_keys_survive_compression() {
        let pixels = compressible_pixels(512 * 1024);
        let mut out = Vec::new();
        encode_kitty_data(&mut out, "a=t,t=d,f=32,s=1411,v=1739,i=151712,q=2", &pixels);

        let (control, _) = decode_kitty_stream(&out);
        for key in ["a=t", "t=d", "f=32", "s=1411", "v=1739", "i=151712", "q=2"] {
            assert!(control.contains(key), "{key} survives: {control}");
        }
    }

    use crate::fm::{FmFilePreview, FmImagePreview, FmImagePreviewState, FmPreview, FmState};
    use crate::ghostty::KittyPlacementRenderInfo;

    const PATH_BETA_RGBA: [u8; 16] = [
        255, 0, 0, 255, // red
        0, 255, 0, 192, // translucent green
        0, 0, 255, 128, // translucent blue
        255, 255, 255, 0, // transparent white
    ];

    fn generated_path_beta_png() -> Vec<u8> {
        let mut png_bytes = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut png_bytes, 2, 2);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().expect("valid PNG header");
            writer
                .write_image_data(&PATH_BETA_RGBA)
                .expect("valid RGBA payload");
        }
        png_bytes
    }

    fn decode_path_beta_png(bytes: &[u8]) -> Option<(u32, u32, Vec<u8>)> {
        let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
        decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
        let mut reader = decoder.read_info().ok()?;
        let mut output = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut output).ok()?;
        if info.color_type != png::ColorType::Rgba || info.bit_depth != png::BitDepth::Eight {
            return None;
        }
        output.truncate(info.buffer_size());
        Some((info.width, info.height, output))
    }

    fn path_beta_placement(rgba: Vec<u8>) -> HostPlacement {
        let mut hasher = DefaultHasher::new();
        rgba.hash(&mut hasher);
        let data_fingerprint = hasher.finish();
        let data_len = rgba.len();

        HostPlacement {
            pane_id: PaneId::from_raw(0xB0),
            host_image_id: None,
            area: Rect::new(2, 2, 8, 4),
            cell_size: HostCellSize {
                width_px: 1,
                height_px: 1,
            },
            source_key: HostSourceKey::Terminal {
                pane_id: PaneId::from_raw(0xB0),
                image_id: 1,
            },
            scrollback_offset: 0,
            placement: KittyImagePlacement {
                image_id: 1,
                placement_id: 1,
                z: 0,
                x_offset: 0,
                y_offset: 0,
                image_width: 2,
                image_height: 2,
                format: KittyImageFormat::Rgba,
                data_len,
                data_fingerprint,
                data: rgba,
                render: KittyPlacementRenderInfo {
                    pixel_width: 2,
                    pixel_height: 2,
                    grid_cols: 8,
                    grid_rows: 4,
                    viewport_col: 0,
                    viewport_row: 0,
                    source_x: 0,
                    source_y: 0,
                    source_width: 0,
                    source_height: 0,
                },
            },
        }
    }

    #[test]
    fn path_beta_generated_png_roundtrips_exact_rgba_and_rejects_truncation() {
        let png = generated_path_beta_png();
        let (width, height, rgba) = decode_path_beta_png(&png).expect("generated PNG decodes");

        assert_eq!((width, height), (2, 2));
        assert_eq!(rgba, PATH_BETA_RGBA);
        assert!(decode_path_beta_png(&png[..png.len() / 2]).is_none());
    }

    #[test]
    fn path_beta_generated_rgba_constructs_stable_local_placement() {
        let (_, _, rgba) =
            decode_path_beta_png(&generated_path_beta_png()).expect("generated PNG decodes");
        let first = path_beta_placement(rgba.clone());
        let same = path_beta_placement(rgba);

        assert_eq!(first.pane_id, PaneId::from_raw(0xB0));
        assert_eq!(first.placement.format, KittyImageFormat::Rgba);
        assert_eq!(first.placement.data_len, PATH_BETA_RGBA.len());
        assert_eq!(first.placement.data, PATH_BETA_RGBA);
        assert_eq!(first.placement.render.grid_cols, 8);
        assert_eq!(first.placement.render.grid_rows, 4);
        assert_eq!(
            host_image_id(first.pane_id, &first.placement),
            host_image_id(same.pane_id, &same.placement)
        );
        assert_eq!(
            host_placement_id(&first.source_key, &first.placement),
            host_placement_id(&same.source_key, &same.placement)
        );
    }

    #[test]
    fn path_beta_generated_png_uses_existing_graphics_lifecycle() {
        let (_, _, rgba) =
            decode_path_beta_png(&generated_path_beta_png()).expect("generated PNG decodes");
        let mut images = HashMap::new();
        let mut placements = HashMap::new();
        let mut sources = HashMap::new();
        let mut bytes = Vec::new();

        encode_graphics_update(
            &mut bytes,
            &[path_beta_placement(rgba.clone())],
            false,
            &mut images,
            &mut placements,
            &mut sources,
        );
        let first = String::from_utf8_lossy(&bytes);
        assert!(first.contains("a=t,t=d,f=32,s=2,v=2"));
        assert!(first.contains("a=p"));
        assert!(first.contains("c=8,r=4"));
        assert!(first.contains("\x1b[3;3H"));

        bytes.clear();
        encode_graphics_update(
            &mut bytes,
            &[path_beta_placement(rgba.clone())],
            false,
            &mut images,
            &mut placements,
            &mut sources,
        );
        assert!(
            bytes.is_empty(),
            "unchanged local image is fully deduplicated"
        );

        encode_graphics_update(
            &mut bytes,
            &[path_beta_placement(rgba.clone())],
            true,
            &mut images,
            &mut placements,
            &mut sources,
        );
        let redisplay = String::from_utf8_lossy(&bytes);
        assert!(!redisplay.contains("a=t"));
        assert!(redisplay.contains("a=p"));

        bytes.clear();
        let mut changed_rgba = rgba;
        changed_rgba[0] = 254;
        encode_graphics_update(
            &mut bytes,
            &[path_beta_placement(changed_rgba)],
            false,
            &mut images,
            &mut placements,
            &mut sources,
        );
        let replacement = String::from_utf8_lossy(&bytes);
        assert!(replacement.contains("a=d,d=I"));
        assert!(replacement.contains("a=t"));
        assert!(replacement.contains("a=p"));

        bytes.clear();
        encode_graphics_update(
            &mut bytes,
            &[],
            false,
            &mut images,
            &mut placements,
            &mut sources,
        );
        let removal = String::from_utf8_lossy(&bytes);
        assert!(removal.contains("a=d,d=i"));
        assert!(!removal.contains("d=I"));
        assert_eq!(images.len(), 1);
        assert!(placements.is_empty());
        assert!(sources.is_empty());
    }

    /// An app whose file manager has an image selected, projected at `frame`.
    fn app_with_selected_image(frame: Rect) -> crate::app::state::AppState {
        let image_path = std::path::PathBuf::from("/virtual/preview.png");
        let mut file_manager = crate::fm::FmState::test_empty("/virtual");
        file_manager.entries = vec![crate::fm::FileEntry {
            name: "preview.png".into(),
            path: image_path.clone(),
            kind: crate::fm::entry_kind::FileEntryKind::RegularFile,
            modified: None,
        }];
        file_manager.preview_generation = 1;
        file_manager.preview = crate::fm::FmPreview::File(crate::fm::FmFilePreview::Image(
            crate::fm::FmImagePreview {
                source_path: image_path,
                generation: 1,
                state: crate::fm::FmImagePreviewState::Pending,
            },
        ));
        file_manager.sync_trail_bridge_for_test();

        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![crate::workspace::Workspace::test_new("one")];
        app.active = Some(0);
        app.selected = 0;
        app.mode = crate::app::state::Mode::Terminal;
        app.mobile_width_threshold = 0;
        app.sidebar_collapsed = true;
        app.sidebar_collapsed_mode = crate::config::SidebarCollapsedModeConfig::Hidden;
        app.try_open_file_manager_with(|_| Some(file_manager))
            .expect("Files activation");
        crate::ui::compute_view(&mut app, frame);
        app
    }

    fn open_viewer_on_selection(app: &mut crate::app::state::AppState, frame: Rect) {
        let source_path = app
            .file_manager
            .as_ref()
            .and_then(|file_manager| file_manager.trail_snapshots.detail())
            .expect("selected detail")
            .path
            .clone();
        app.preview_viewer = Some(crate::app::state::PreviewViewerState { source_path });
        app.mode = crate::app::state::Mode::PreviewViewer;
        crate::ui::compute_view(app, frame);
    }

    // TP-FVIEW-08: enlarging asks for more pixels. If the viewer reused the
    // panel's decode target it would stretch panel-sized pixels across the
    // frame, which is a blurrier picture rather than a bigger one — and the
    // whole feature would be indistinguishable from doing nothing.
    #[test]
    fn opening_the_viewer_asks_for_more_pixels_than_the_panel() {
        let cells = HostCellSize {
            width_px: 8,
            height_px: 16,
        };
        let frame = Rect::new(0, 0, 120, 30);
        let mut app = app_with_selected_image(frame);
        let panel = file_manager_image_target(&app, cells).expect("panel decode target");

        open_viewer_on_selection(&mut app, frame);
        let viewer = file_manager_image_target(&app, cells).expect("viewer decode target");

        assert!(
            viewer.width_px > panel.width_px && viewer.height_px > panel.height_px,
            "viewer target {viewer:?} must exceed panel target {panel:?}"
        );
        let content = app
            .view
            .preview_viewer_content_area
            .expect("viewer content area");
        assert_eq!(viewer.width_px, u32::from(content.width) * cells.width_px);
        assert_eq!(
            viewer.height_px,
            u32::from(content.height) * cells.height_px
        );
    }

    // TP-FVIEW-09: resizing the terminal while the viewer is open produces a
    // new decode target, so the picture is re-fitted instead of left placed
    // against geometry that no longer exists.
    #[test]
    fn resizing_while_the_viewer_is_open_retargets_the_picture() {
        let cells = HostCellSize {
            width_px: 8,
            height_px: 16,
        };
        let frame = Rect::new(0, 0, 120, 30);
        let mut app = app_with_selected_image(frame);
        open_viewer_on_selection(&mut app, frame);
        let before = file_manager_image_target(&app, cells).expect("first viewer target");

        let wider = Rect::new(0, 0, 160, 40);
        crate::ui::compute_view(&mut app, wider);
        let after = file_manager_image_target(&app, cells).expect("resized viewer target");

        assert!(
            after.width_px > before.width_px && after.height_px > before.height_px,
            "a larger frame must produce a larger target: {before:?} -> {after:?}"
        );
    }

    // TP-FVIEW-10: with the viewer closed the panel is the target again, so
    // closing does not leave the file manager decoding at full-frame size.
    #[test]
    fn closing_the_viewer_returns_the_target_to_the_panel() {
        let cells = HostCellSize {
            width_px: 8,
            height_px: 16,
        };
        let frame = Rect::new(0, 0, 120, 30);
        let mut app = app_with_selected_image(frame);
        let panel = file_manager_image_target(&app, cells).expect("panel decode target");

        open_viewer_on_selection(&mut app, frame);
        app.preview_viewer = None;
        app.mode = crate::app::state::Mode::Terminal;
        crate::ui::compute_view(&mut app, frame);

        assert_eq!(
            file_manager_image_target(&app, cells),
            Some(panel),
            "closing restores the panel's decode target exactly"
        );
    }

    // TP-TRAIL-T7-IMAGE-02: host placement and decode target share the live
    // Trail detail content rect. A valid legacy PREVIEW rect cannot authorize
    // pixels outside this exact panel.
    #[test]
    fn file_manager_ready_image_placement_uses_trail_detail_content_rect() {
        let cells = HostCellSize {
            width_px: 8,
            height_px: 16,
        };
        let image_path = std::path::PathBuf::from("/virtual/preview.png");
        let mut file_manager = crate::fm::FmState::test_empty("/virtual");
        file_manager.entries = vec![crate::fm::FileEntry {
            name: "preview.png".into(),
            path: image_path.clone(),
            kind: crate::fm::entry_kind::FileEntryKind::RegularFile,
            modified: None,
        }];
        file_manager.preview_generation = 1;
        file_manager.preview = crate::fm::FmPreview::File(crate::fm::FmFilePreview::Image(
            crate::fm::FmImagePreview {
                source_path: image_path,
                generation: 1,
                state: crate::fm::FmImagePreviewState::Pending,
            },
        ));
        file_manager.sync_trail_bridge_for_test();

        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![crate::workspace::Workspace::test_new("one")];
        app.active = Some(0);
        app.selected = 0;
        app.mode = crate::app::state::Mode::Terminal;
        app.mobile_width_threshold = 0;
        app.sidebar_collapsed = true;
        app.sidebar_collapsed_mode = crate::config::SidebarCollapsedModeConfig::Hidden;
        app.try_open_file_manager_with(|_| Some(file_manager))
            .expect("Files activation");
        crate::ui::compute_view(&mut app, Rect::new(0, 0, 120, 30));
        let content = app
            .view
            .file_manager_trail
            .detail_panel
            .as_ref()
            .expect("Trail detail panel")
            .content_rect;
        let expected_target = ImagePreviewTarget {
            width_px: u32::from(content.width) * cells.width_px,
            height_px: u32::from(content.height) * cells.height_px,
        };
        let prepared = PreparedImagePreview {
            width: 8,
            height: 8,
            data_fingerprint: 0x88,
            rgba: vec![0x88; 8 * 8 * 4],
        };
        let preview = app
            .file_manager
            .as_mut()
            .and_then(|fm| match &mut fm.preview {
                FmPreview::File(FmFilePreview::Image(preview)) => Some(preview),
                _ => None,
            })
            .expect("mutable image preview");
        preview.state = FmImagePreviewState::Ready {
            target: expected_target,
            prepared,
        };

        let placement = collect_file_manager_image_placement(&app, cells, &HashMap::new())
            .expect("Trail detail placement");
        assert_eq!(placement.area, content);
    }

    #[test]
    fn file_manager_image_placement_is_centered_bounded_and_client_local() {
        let cells = HostCellSize {
            width_px: 8,
            height_px: 16,
        };
        let prepared = PreparedImagePreview {
            width: 80,
            height: 64,
            data_fingerprint: 0x8064,
            rgba: vec![0x7f; 80 * 64 * 4],
        };
        let placement = file_manager_image_placement(Rect::new(10, 5, 38, 10), cells, &prepared)
            .expect("valid three-column local placement");

        assert_eq!(placement.pane_id, PaneId::from_raw(u32::MAX));
        assert_eq!(placement.area, Rect::new(36, 7, 12, 7));
        assert_eq!(placement.scrollback_offset, 0);
        assert_eq!(placement.placement.format, KittyImageFormat::Rgba);
        assert_eq!(
            (
                placement.placement.image_width,
                placement.placement.image_height
            ),
            (80, 64)
        );
        assert_eq!(placement.placement.data_len, prepared.rgba.len());
        assert_eq!(placement.placement.data, prepared.rgba);
        assert_eq!(
            (
                placement.placement.render.grid_cols,
                placement.placement.render.grid_rows
            ),
            (10, 4)
        );
        assert_eq!(
            (
                placement.placement.render.viewport_col,
                placement.placement.render.viewport_row
            ),
            (1, 1)
        );

        let (clipped, format) = clipped_placement(&placement).expect("placement remains visible");
        assert_eq!(format, 32);
        assert_eq!(
            (clipped.x, clipped.y, clipped.cols, clipped.rows),
            (37, 8, 10, 4)
        );

        let malformed = PreparedImagePreview {
            width: 80,
            height: 64,
            data_fingerprint: 3,
            rgba: vec![0; 3],
        };
        assert!(
            file_manager_image_placement(Rect::new(10, 5, 38, 10), cells, &malformed,).is_none()
        );

        let oversized = PreparedImagePreview {
            width: 97,
            height: 64,
            data_fingerprint: 0x9764,
            rgba: vec![0; 97 * 64 * 4],
        };
        assert!(
            file_manager_image_placement(Rect::new(10, 5, 38, 10), cells, &oversized,).is_none()
        );
    }

    #[test]
    fn file_manager_ready_image_reuses_upload_cache_and_cleans_up_on_close() {
        let graphics = crate::app::pane_graphics::Runtime::default();
        let cells = HostCellSize {
            width_px: 8,
            height_px: 16,
        };
        let first = PreparedImagePreview {
            width: 80,
            height: 64,
            data_fingerprint: 0x1111,
            rgba: vec![0x11; 80 * 64 * 4],
        };
        let mut app = AppState::test_new();
        app.workspaces = vec![crate::workspace::Workspace::test_new("one")];
        app.active = Some(0);
        app.selected = 0;
        app.mode = Mode::Terminal;
        app.mobile_width_threshold = 0;
        app.sidebar_collapsed = true;
        app.sidebar_collapsed_mode = crate::config::SidebarCollapsedModeConfig::Hidden;
        let frame = Rect::new(0, 0, 120, 30);
        let image_path = std::path::PathBuf::from("/tmp/preview.png");
        let mut file_manager = FmState::test_empty("/tmp");
        file_manager.cwd_writable = true;
        file_manager.entries = vec![crate::fm::FileEntry {
            name: "preview.png".into(),
            path: image_path.clone(),
            kind: crate::fm::entry_kind::FileEntryKind::RegularFile,
            modified: None,
        }];
        file_manager.preview = FmPreview::File(FmFilePreview::Image(FmImagePreview {
            source_path: image_path,
            generation: 1,
            state: FmImagePreviewState::Pending,
        }));
        file_manager.preview_generation = 1;
        file_manager.sync_trail_bridge_for_test();
        app.try_open_file_manager_with(|_| Some(file_manager))
            .expect("Files activation");
        crate::ui::compute_view(&mut app, frame);
        let content = app
            .view
            .file_manager_trail
            .detail_panel
            .as_ref()
            .expect("Trail image detail")
            .content_rect;
        let expected_target = ImagePreviewTarget {
            width_px: u32::from(content.width) * cells.width_px,
            height_px: u32::from(content.height) * cells.height_px,
        };
        let preview = app
            .file_manager
            .as_mut()
            .and_then(|fm| match &mut fm.preview {
                FmPreview::File(FmFilePreview::Image(preview)) => Some(preview),
                _ => None,
            })
            .expect("mutable image preview");
        preview.state = FmImagePreviewState::Ready {
            target: expected_target,
            prepared: first,
        };
        let runtimes = TerminalRuntimeRegistry::new();
        let mut cache = HostGraphicsCache::default();

        let uncached = collect_file_manager_image_placement(&app, cells, &cache.images)
            .expect("ready image placement");
        assert_eq!(uncached.area, content);
        assert!(!uncached.placement.data.is_empty());

        let first_bytes = encode_local_pane_graphics(
            &app,
            &graphics,
            &runtimes,
            app.view.tab_surface(),
            cells,
            None,
            false,
            &mut cache,
        );
        let first_text = String::from_utf8_lossy(&first_bytes.bytes);
        assert!(first_text.contains("a=t,t=d,f=32,s=80,v=64"));
        assert!(first_text.contains("a=p"));
        assert!(first_text.contains(&format!(
            "c={},r={}",
            uncached.placement.render.grid_cols, uncached.placement.render.grid_rows
        )));
        assert!(first_text.contains(&format!(
            "\x1b[{};{}H",
            i32::from(content.y) + uncached.placement.render.viewport_row + 1,
            i32::from(content.x) + uncached.placement.render.viewport_col + 1
        )));

        let cached = collect_file_manager_image_placement(&app, cells, &cache.images)
            .expect("cached image placement metadata");
        assert!(
            cached.placement.data.is_empty(),
            "cached frame must not clone the prepared RGBA allocation"
        );
        assert!(
            encode_local_pane_graphics(
                &app,
                &graphics,
                &runtimes,
                app.view.tab_surface(),
                cells,
                None,
                false,
                &mut cache,
            )
            .bytes
            .is_empty(),
            "unchanged FM frame is fully deduplicated"
        );

        let preview = app
            .file_manager
            .as_mut()
            .and_then(|fm| match &mut fm.preview {
                FmPreview::File(FmFilePreview::Image(preview)) => Some(preview),
                _ => None,
            })
            .expect("mutable image preview");
        preview.generation = 2;
        preview.state = FmImagePreviewState::Ready {
            target: expected_target,
            prepared: PreparedImagePreview {
                width: 80,
                height: 64,
                data_fingerprint: 0x2222,
                rgba: vec![0x22; 80 * 64 * 4],
            },
        };
        let replacement = encode_local_pane_graphics(
            &app,
            &graphics,
            &runtimes,
            app.view.tab_surface(),
            cells,
            None,
            false,
            &mut cache,
        );
        let replacement = String::from_utf8_lossy(&replacement.bytes);
        assert!(replacement.contains("a=d,d=I"));
        assert!(replacement.contains("a=t"));
        assert!(replacement.contains("a=p"));

        app.file_manager = None;
        let cleanup = encode_local_pane_graphics(
            &app,
            &graphics,
            &runtimes,
            app.view.tab_surface(),
            cells,
            None,
            false,
            &mut cache,
        );
        let cleanup = String::from_utf8_lossy(&cleanup.bytes);
        assert!(cleanup.contains("a=d,d=I"));
        assert!(cache.is_empty());
        assert!(cache.sources.is_empty());
    }

    #[test]
    fn path_beta_frames_graphics_without_cursor_drift() {
        let framed = frame_graphics_bytes(b"graphics");

        assert_eq!(framed, b"\x1b7graphics\x1b8");
    }

    #[test]
    #[ignore = "requires an explicit throwaway Kitty/Ghostty host and --no-capture"]
    fn path_beta_real_host_probe() {
        let (_, _, rgba) =
            decode_path_beta_png(&generated_path_beta_png()).expect("generated PNG decodes");
        let mut cache = HostGraphicsCache::default();
        let mut encoded = Vec::new();
        encode_graphics_update(
            &mut encoded,
            &[path_beta_placement(rgba)],
            false,
            &mut cache.images,
            &mut cache.placements,
            &mut cache.sources,
        );

        let mut stdout = std::io::stdout().lock();
        stdout
            .write_all(b"\x1b[2J\x1b[HPath Beta probe: 2x2 RGBA pattern\n")
            .expect("write probe heading");
        stdout
            .write_all(&frame_graphics_bytes(&encoded))
            .expect("write graphics probe");
        stdout.flush().expect("flush graphics probe");
        std::thread::sleep(std::time::Duration::from_secs(12));

        let cleanup = cache.clear_bytes();
        stdout
            .write_all(&frame_graphics_bytes(&cleanup))
            .expect("remove graphics resources");
        stdout.flush().expect("flush graphics cleanup");
    }

    #[test]
    fn fallback_cell_size_is_usable_only_for_nonempty_areas() {
        assert_eq!(
            HostCellSize::fallback_for_area(Rect::new(0, 0, 80, 24)),
            HostCellSize {
                width_px: 8,
                height_px: 16,
            }
        );
        assert!(!HostCellSize::fallback_for_area(Rect::default()).is_known());
    }

    fn scan_browser_frame(tick: u32, area_w: u16) -> HostPlacement {
        let mut placement = test_placement(0, 0);
        placement.area = Rect::new(0, 0, area_w, 10);
        placement.placement.data_fingerprint = 1000 + u64::from(tick);
        placement.placement.render.grid_cols = u32::from(area_w).saturating_sub(2);
        placement.placement.render.grid_rows = 8;
        placement
    }

    fn scan_neighbour_frame(tick: u32) -> HostPlacement {
        let mut placement = test_placement(0, 0);
        placement.pane_id = PaneId::from_raw(2);
        placement.host_image_id = Some(0x8000_0001);
        placement.source_key = HostSourceKey::PaneLayer {
            pane_id: PaneId::from_raw(2),
            layer_id: "primary".into(),
        };
        placement.placement.image_id = 900;
        placement.placement.placement_id = 900;
        placement.placement.data_fingerprint = 5 + u64::from(tick / 3);
        placement
    }

    fn scan_apply_to_kitty(
        feed: &mut std::collections::HashMap<(String, String), u32>,
        bytes: &[u8],
    ) {
        let text = String::from_utf8_lossy(bytes);
        let mut rest = text.as_ref();
        while let Some(start) = rest.find("\u{1b}_G") {
            let tail = &rest[start + 3..];
            let end = tail.find('\u{1b}').unwrap_or(tail.len());
            let head = tail[..end].split(';').next().unwrap_or("");
            let mut a = "";
            let mut i = "";
            let mut pl = "";
            let mut d = "";
            let mut c = 0u32;
            for part in head.split(',') {
                let Some((k, v)) = part.split_once('=') else {
                    continue;
                };
                match k {
                    "a" => a = v,
                    "i" => i = v,
                    "p" => pl = v,
                    "d" => d = v,
                    "c" => c = v.parse().unwrap_or(0),
                    _ => {}
                }
            }
            match a {
                "T" | "p" => {
                    feed.insert((i.to_owned(), pl.to_owned()), c);
                }
                "d" => match d {
                    "A" | "a" => feed.clear(),
                    "I" => feed.retain(|(image, _), _| image != i),
                    _ => {
                        feed.remove(&(i.to_owned(), pl.to_owned()));
                    }
                },
                _ => {}
            }
            rest = &rest[start + 3..];
        }
    }

    // TP-GFX-LEDGER-01 scan harness: replay the live wire's shape — a static
    // neighbour layer plus a terminal-source video pane whose content hash
    // changes every frame — through the incremental encoder, dropping some
    // encoded turns the way a full channel drops them (clone discarded,
    // bytes never delivered), across resize gestures. The terminal must
    // never be left showing more than one browser placement.
    #[test]
    fn a_dropped_turn_during_a_resize_strands_no_terminal_placement() {
        let live_sources: HashSet<HostSourceKey> = [HostSourceKey::PaneLayer {
            pane_id: PaneId::from_raw(2),
            layer_id: "primary".into(),
        }]
        .into_iter()
        .collect();
        let widths: [u16; 24] = [
            20, 20, 20, 17, 17, 20, 20, 16, 16, 16, 20, 20, 14, 14, 20, 20, 12, 12, 20, 20, 10, 10,
            10, 10,
        ];
        for drop_mask in 0u32..256 {
            let mut cache = HostGraphicsCache::default();
            let mut kitty = std::collections::HashMap::new();
            for (turn, width) in widths.iter().enumerate() {
                let placements = vec![
                    scan_neighbour_frame(turn as u32),
                    scan_browser_frame(turn as u32, *width),
                ];
                {
                    let mut next = cache.clone();
                    next.request_placement_replay();
                    let encoded = encode_graphics_update_incremental(
                        &mut next,
                        &placements,
                        &live_sources,
                        Some(HEADLESS_GRAPHICS_TRANSACTION_BUDGET),
                    );
                    let dropped = (drop_mask >> (turn % 8)) & 1 == 1;
                    if !dropped {
                        cache = next;
                        scan_apply_to_kitty(&mut kitty, &encoded.bytes);
                    }
                }
                let browser_alive: Vec<_> = kitty
                    .iter()
                    .filter(|((image, _), cols)| {
                        **cols > 0 && image.parse::<u64>().is_ok_and(|id| id < 0x8000_0000)
                    })
                    .map(|((image, placement), cols)| format!("i={image} p={placement} c={cols}"))
                    .collect();
                assert!(
                    browser_alive.len() <= 1,
                    "drop_mask={drop_mask:#010b} turn={turn} width={width}: the terminal \
                     still shows {} browser placements: {browser_alive:?}",
                    browser_alive.len()
                );
            }
        }
    }

    fn test_placement(viewport_col: i32, viewport_row: i32) -> HostPlacement {
        HostPlacement {
            pane_id: PaneId::from_raw(1),
            host_image_id: None,
            area: Rect::new(0, 0, 20, 10),
            cell_size: HostCellSize {
                width_px: 10,
                height_px: 10,
            },
            source_key: HostSourceKey::Terminal {
                pane_id: PaneId::from_raw(1),
                image_id: 7,
            },
            scrollback_offset: 0,
            placement: KittyImagePlacement {
                image_id: 7,
                placement_id: 3,
                z: 0,
                x_offset: 0,
                y_offset: 0,
                image_width: 30,
                image_height: 30,
                format: KittyImageFormat::Rgba,
                data_len: 30 * 30 * 4,
                data_fingerprint: 42,
                data: vec![255; 30 * 30 * 4],
                render: KittyPlacementRenderInfo {
                    pixel_width: 0,
                    pixel_height: 0,
                    grid_cols: 3,
                    grid_rows: 3,
                    viewport_col,
                    viewport_row,
                    source_x: 0,
                    source_y: 0,
                    source_width: 0,
                    source_height: 0,
                },
            },
        }
    }

    fn pane_layer_placement(viewport_col: i32, viewport_row: i32) -> HostPlacement {
        let mut placement = test_placement(viewport_col, viewport_row);
        placement.source_key = HostSourceKey::PaneLayer {
            pane_id: placement.pane_id,
            layer_id: "primary".into(),
        };
        placement
    }

    fn update(
        cache: &mut HostGraphicsCache,
        placements: &[HostPlacement],
        replay: bool,
    ) -> Vec<u8> {
        let mut bytes = Vec::new();
        encode_graphics_update(
            &mut bytes,
            placements,
            replay,
            &mut cache.images,
            &mut cache.placements,
            &mut cache.sources,
        );
        bytes
    }

    #[test]
    fn terminal_graphics_without_pane_layers_preserves_legacy_transcript() {
        fn record(transcript: &mut Vec<u8>, bytes: &[u8]) {
            transcript.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
            transcript.extend_from_slice(bytes);
        }

        fn fnv1a(bytes: &[u8]) -> u64 {
            bytes.iter().fold(0xcbf29ce484222325, |hash, byte| {
                (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
            })
        }

        let mut cache = HostGraphicsCache::default();
        let mut transcript = Vec::new();

        record(
            &mut transcript,
            &update(&mut cache, &[test_placement(0, 0)], false),
        );
        record(
            &mut transcript,
            &update(&mut cache, &[test_placement(0, 0)], false),
        );
        record(
            &mut transcript,
            &update(&mut cache, &[test_placement(-1, 2)], false),
        );
        record(
            &mut transcript,
            &update(&mut cache, &[test_placement(-1, 2)], true),
        );

        let mut changed = test_placement(-1, 2);
        changed.placement.data_fingerprint = 43;
        record(&mut transcript, &update(&mut cache, &[changed], false));
        record(&mut transcript, &update(&mut cache, &[], false));

        assert_eq!(transcript.len(), 10_084);
        assert_eq!(fnv1a(&transcript), 0xc5bd_83e4_b039_870e);
    }

    #[test]
    fn terminal_placement_id_preserves_legacy_identity() {
        let placement = test_placement(0, 0);
        let mut legacy = DefaultHasher::new();
        placement.pane_id.raw().hash(&mut legacy);
        placement.placement.image_id.hash(&mut legacy);
        placement.placement.placement_id.hash(&mut legacy);
        let expected = 1 + ((legacy.finish() as u32) % 900_000);

        assert_eq!(
            host_placement_id(&placement.source_key, &placement.placement),
            expected
        );
        assert_ne!(
            host_placement_id(
                &HostSourceKey::PaneLayer {
                    pane_id: placement.pane_id,
                    layer_id: "primary".into(),
                },
                &placement.placement,
            ),
            expected
        );
    }

    #[cfg(unix)]
    #[test]
    fn regular_file_command_is_rgba_quiet_zero_and_path_encoded() {
        let mut bytes = Vec::new();
        encode_kitty_regular_file(
            &mut bytes,
            b"\x1b[2;3H",
            "a=T,f=32,s=3,v=2,i=42,p=7,c=3,r=2,z=0,C=1,q=0",
            "/private/frame",
        );
        let text = String::from_utf8(bytes).unwrap();
        assert!(text.starts_with("\x1b7\x1b[2;3H\x1b_Ga=T,f=32"));
        assert!(text.contains(",C=1,q=0,t=f;L3ByaXZhdGUvZnJhbWU="));
        assert!(text.ends_with("\x1b\\\x1b8"));
    }

    #[test]
    fn direct_file_uses_one_clipped_transmit_and_display_at_the_final_position() {
        let mut placement = pane_layer_placement(-1, 2);
        placement.area = Rect::new(10, 4, 8, 6);
        let (command, _, _, _) = direct_file_command(&placement, (1 << 31) | 9).unwrap();
        let control = command.control;

        assert_eq!(command.leading, b"\x1b[7;11H");
        assert!(control.starts_with("a=T,f=32,s=30,v=30,i=2147483657,p="));
        assert!(control.contains(",c=2,r=3,z=0,C=1,q=0,x=10,w=20,h=30"));
        assert!(direct_file_command(&pane_layer_placement(30, 0), 9).is_none());
    }

    #[test]
    fn pane_graphics_image_ids_are_disjoint_from_terminal_image_ids() {
        let placement = test_placement(0, 0);
        let signature = image_signature(&placement, kitty_format_code(placement.placement.format));
        let terminal_id = host_image_id_for_signature(placement.pane_id, signature);
        let mut graphics = crate::app::pane_graphics::Runtime::default();
        let primary = (placement.pane_id, "primary".into());
        let pane_graphics_id = graphics.reserve_image_id(&primary).unwrap();
        graphics.slots.insert(
            primary.clone(),
            crate::app::pane_graphics::Slot::test(pane_graphics_id, None),
        );

        assert_eq!(terminal_id & PANE_GRAPHICS_IMAGE_ID_BIT, 0);
        assert_ne!(pane_graphics_id & PANE_GRAPHICS_IMAGE_ID_BIT, 0);
        assert_eq!(
            pane_graphics_id,
            graphics.reserve_image_id(&primary).unwrap()
        );
        assert_ne!(
            pane_graphics_id,
            graphics
                .reserve_image_id(&(placement.pane_id, "toolbar".into()))
                .unwrap()
        );
    }

    #[test]
    fn clipped_placement_handles_positive_viewport_without_wrapping() {
        let placement = test_placement(2, 2);
        let (clipped, _) = clipped_placement(&placement).expect("visible placement");

        assert_eq!(clipped.x, 2);
        assert_eq!(clipped.y, 2);
        assert_eq!(clipped.cols, 3);
        assert_eq!(clipped.rows, 3);
        assert_eq!(clipped.source_x, 0);
        assert_eq!(clipped.source_y, 0);
    }

    #[test]
    fn clipped_placement_crops_negative_viewport_offsets() {
        let placement = test_placement(-1, -1);
        let (clipped, _) = clipped_placement(&placement).expect("partially visible placement");

        assert_eq!(clipped.x, 0);
        assert_eq!(clipped.y, 0);
        assert_eq!(clipped.cols, 2);
        assert_eq!(clipped.rows, 2);
        assert_eq!(clipped.source_x, 10);
        assert_eq!(clipped.source_y, 10);
    }

    #[test]
    fn pane_graphics_layer_defaults_to_full_pane_grid() {
        let info = PaneInfo {
            id: PaneId::from_raw(9),
            rect: Rect::new(0, 0, 12, 5),
            inner_rect: Rect::new(2, 1, 8, 3),
            scrollbar_rect: None,
            borders: ratatui::widgets::Borders::NONE,
            is_focused: true,
        };
        let layer = crate::app::pane_graphics::Layer::inline(
            crate::api::schema::PaneGraphicsFormat::Rgba,
            80,
            30,
            vec![255; 80 * 30 * 4],
            crate::api::schema::PaneGraphicsPlacementParams::default(),
            0,
        );

        let placement = pane_graphics_host_placement(
            &info,
            "primary",
            PANE_GRAPHICS_IMAGE_ID_BIT | 1,
            HostCellSize {
                width_px: 10,
                height_px: 10,
            },
            &layer,
            &HashMap::new(),
            true,
        );
        let (clipped, format_code) = clipped_placement(&placement).expect("visible layer");

        assert_eq!(format_code, 32);
        assert_eq!(clipped.x, 2);
        assert_eq!(clipped.y, 1);
        assert_eq!(clipped.cols, 8);
        assert_eq!(clipped.rows, 3);
        assert_eq!(placement.placement.data.len(), 80 * 30 * 4);
    }

    // TP-FPOPUP-01: a popup pane draws pictures. It is not a member of the tab
    // surface, so the placement pass used to skip it entirely — a `herdr view`
    // opened over the file manager showed its page counter and no page, which
    // is the one thing the reader clicked for. While the popup is up it also
    // OWNS the layer: the file manager's own preview sits underneath it, and
    // placing that too would paint the old picture across the popup.
    #[test]
    fn a_popup_pane_owns_the_picture_layer_over_the_file_manager() {
        let cells = HostCellSize {
            width_px: 8,
            height_px: 16,
        };
        let frame = Rect::new(0, 0, 120, 30);
        let mut app = app_with_selected_image(frame);
        app.kitty_graphics_enabled = true;

        // The file manager's own preview is placed while nothing floats above.
        let preview_path = app
            .file_manager
            .as_ref()
            .and_then(|file_manager| file_manager.trail_snapshots.detail())
            .expect("selected detail")
            .path
            .clone();
        let content = app
            .view
            .file_manager_trail
            .detail_panel
            .as_ref()
            .expect("Trail detail panel")
            .content_rect;
        let preview = app
            .file_manager
            .as_mut()
            .and_then(|fm| match &mut fm.preview {
                FmPreview::File(FmFilePreview::Image(preview)) => Some(preview),
                _ => None,
            })
            .expect("mutable image preview");
        preview.state = FmImagePreviewState::Ready {
            target: ImagePreviewTarget {
                width_px: u32::from(content.width) * cells.width_px,
                height_px: u32::from(content.height) * cells.height_px,
            },
            prepared: PreparedImagePreview {
                width: 80,
                height: 64,
                data_fingerprint: 0x2222,
                rgba: vec![0x22; 80 * 64 * 4],
            },
        };
        assert!(
            collect_file_manager_image_placement(&app, cells, &HashMap::new()).is_some(),
            "the file manager preview is placed while nothing floats over it"
        );
        assert_eq!(
            preview_path.file_name().and_then(|name| name.to_str()),
            Some("preview.png")
        );

        // A popup opens over it, carrying a picture of its own.
        let popup_pane = PaneId::from_raw(4242);
        app.popup_pane = Some(crate::app::state::PopupPaneState {
            pane_id: popup_pane,
            terminal_id: crate::terminal::TerminalId::alloc(),
            width: None,
            height: None,
        });
        let mut graphics = crate::app::pane_graphics::Runtime::default();
        graphics.slots.insert(
            (popup_pane, "popup-test".to_string()),
            crate::app::pane_graphics::Slot::test(
                (1 << 31) | 7,
                Some(crate::app::pane_graphics::Layer::inline(
                    crate::api::schema::PaneGraphicsFormat::Rgba,
                    40,
                    20,
                    vec![0x33; 40 * 20 * 4],
                    crate::api::schema::PaneGraphicsPlacementParams::default(),
                    0,
                )),
            ),
        );
        crate::ui::compute_view(&mut app, frame);

        let runtimes = TerminalRuntimeRegistry::new();
        let placements =
            collect_popup_pane_placements(&app, &graphics, &runtimes, cells, &HashMap::new(), true)
                .expect("a popup owns the placement pass");
        let placement = placements
            .iter()
            .find(|placement| placement.pane_id == popup_pane)
            .expect("the popup's own picture is placed");
        let (_inner_outer, inner) =
            crate::ui::popup_pane_rects(&app, app.view.terminal_area).expect("popup geometry");
        assert_eq!(
            placement.area, inner,
            "the picture lands inside the popup's frame"
        );
        assert!(
            clipped_placement(placement).is_some(),
            "the popup's picture is on screen"
        );

        let mut cache = HostGraphicsCache::default();
        let bytes = encode_local_pane_graphics(
            &app,
            &graphics,
            &runtimes,
            app.view.tab_surface(),
            cells,
            None,
            false,
            &mut cache,
        );
        assert!(
            !bytes.bytes.is_empty(),
            "the popup's picture reaches the host terminal"
        );
        assert!(
            cache.sources.keys().all(|source| !matches!(
                source,
                HostSourceKey::Terminal { pane_id, .. }
                    if *pane_id == PaneId::from_raw(FILE_MANAGER_PREVIEW_PANE_RAW)
            )),
            "the file manager preview underneath is not painted across the popup"
        );
    }

    #[test]
    fn graphics_update_uploads_once_then_repositions_only() {
        let mut cache = HostGraphicsCache::default();
        let first = update(&mut cache, &[test_placement(0, 0)], false);
        assert!(String::from_utf8_lossy(&first).contains("a=t"));
        assert!(String::from_utf8_lossy(&first).contains("a=p"));
        assert!(update(&mut cache, &[test_placement(0, 0)], false).is_empty());

        let mut changed = test_placement(0, 0);
        changed.placement.z = 1;
        for placement in [changed, test_placement(0, 1)] {
            let bytes = update(&mut cache, &[placement], false);
            assert!(!String::from_utf8_lossy(&bytes).contains("a=t"));
            assert!(String::from_utf8_lossy(&bytes).contains("a=p"));
        }
    }

    #[test]
    fn view_change_redisplays_unchanged_visible_placement() {
        let mut cache = HostGraphicsCache::default();
        update(&mut cache, &[test_placement(0, 0)], false);
        assert_eq!(cache.placements.len(), 1);
        let bytes = update(&mut cache, &[test_placement(0, 0)], true);
        assert!(!String::from_utf8_lossy(&bytes).contains("a=t"));
        assert!(String::from_utf8_lossy(&bytes).contains("a=p"));
        assert_eq!(cache.placements.len(), 1);
    }

    #[test]
    fn surface_reset_deletes_then_reuploads_and_redisplays_placement() {
        let mut cache = HostGraphicsCache::default();
        update(&mut cache, &[test_placement(0, 0)], false);
        assert_eq!((cache.images.len(), cache.placements.len()), (1, 1));
        let mut bytes = cache.clear_bytes();
        bytes.extend(update(&mut cache, &[test_placement(0, 0)], false));
        let redisplay = String::from_utf8_lossy(&bytes);
        assert!(redisplay.contains("a=d,d=I"));
        assert!(redisplay.contains("a=t"));
        assert!(redisplay.contains("a=p"));
        assert_eq!((cache.images.len(), cache.placements.len()), (1, 1));
    }

    #[test]
    fn scrollback_offset_change_redisplays_placement() {
        let mut cache = HostGraphicsCache::default();
        update(&mut cache, &[test_placement(0, 0)], false);
        let mut scrolled = test_placement(0, 0);
        scrolled.scrollback_offset = 3;
        let bytes = update(&mut cache, &[scrolled], false);
        assert!(!String::from_utf8_lossy(&bytes).contains("a=t"));
        assert!(String::from_utf8_lossy(&bytes).contains("a=p"));
    }

    #[test]
    fn empty_image_data_does_not_mark_image_uploaded() {
        let mut images = HashMap::new();
        let mut placements = HashMap::new();
        let mut sources = HashMap::new();
        let mut bytes = Vec::new();
        let mut placement = test_placement(0, 0);
        placement.placement.data.clear();

        encode_graphics_update(
            &mut bytes,
            &[placement],
            false,
            &mut images,
            &mut placements,
            &mut sources,
        );

        assert!(bytes.is_empty());
        assert!(images.is_empty());
        assert!(placements.is_empty());
    }

    #[test]
    fn same_image_signature_reuses_host_upload_across_source_image_ids() {
        let mut images = HashMap::new();
        let mut placements = HashMap::new();
        let mut sources = HashMap::new();
        let mut bytes = Vec::new();
        let first = test_placement(0, 0);

        encode_graphics_update(
            &mut bytes,
            &[first],
            false,
            &mut images,
            &mut placements,
            &mut sources,
        );
        assert_eq!(images.len(), 1);
        assert_eq!(placements.len(), 1);

        bytes.clear();
        let mut same_image_new_source_id = test_placement(0, 0);
        same_image_new_source_id.placement.image_id = 8;
        same_image_new_source_id.placement.placement_id = 4;
        same_image_new_source_id.placement.data.clear();
        encode_graphics_update(
            &mut bytes,
            &[same_image_new_source_id],
            false,
            &mut images,
            &mut placements,
            &mut sources,
        );

        let reused = String::from_utf8_lossy(&bytes);
        assert!(!reused.contains("a=t"));
        assert!(reused.contains("a=p"));
        assert_eq!(images.len(), 1);
        assert_eq!(placements.len(), 1);
    }

    #[test]
    fn pane_layer_replacement_is_atomic_without_delete_to_blank() {
        let mut images = HashMap::new();
        let mut placements = HashMap::new();
        let mut sources = HashMap::new();
        let mut bytes = Vec::new();
        let mut first = pane_layer_placement(0, 0);
        first.host_image_id = Some((1 << 31) | 7);
        encode_graphics_update(
            &mut bytes,
            &[first],
            false,
            &mut images,
            &mut placements,
            &mut sources,
        );

        bytes.clear();
        let mut changed = pane_layer_placement(0, 0);
        changed.host_image_id = Some((1 << 31) | 7);
        changed.placement.data_fingerprint += 1;
        encode_graphics_update(
            &mut bytes,
            &[changed],
            false,
            &mut images,
            &mut placements,
            &mut sources,
        );

        let update = String::from_utf8_lossy(&bytes);
        assert!(update.contains("a=T,t=d"));
        assert!(update.contains(",p=") && update.contains(",C=1,q=2"));
        assert!(!update.contains("a=d"));
    }

    #[test]
    fn replaced_image_content_deletes_superseded_host_image() {
        let mut images = HashMap::new();
        let mut placements = HashMap::new();
        let mut sources = HashMap::new();
        let mut bytes = Vec::new();
        let first = test_placement(0, 0);

        encode_graphics_update(
            &mut bytes,
            &[first],
            false,
            &mut images,
            &mut placements,
            &mut sources,
        );
        assert_eq!(images.len(), 1);
        let superseded_host_id = *images.keys().next().expect("uploaded host image");

        // Same source image id, new pixel content: the fresh content maps to
        // a fresh host image id, so the replaced one must be deleted.
        bytes.clear();
        let mut changed = test_placement(0, 0);
        changed.placement.data_fingerprint = 43;
        encode_graphics_update(
            &mut bytes,
            &[changed],
            false,
            &mut images,
            &mut placements,
            &mut sources,
        );

        let update = String::from_utf8_lossy(&bytes);
        assert!(update.contains("a=t"), "changed content re-uploads");
        assert!(
            update.contains(&format!("a=d,d=I,i={superseded_host_id}")),
            "superseded host image is deleted"
        );
        assert_eq!(images.len(), 1);
        assert_eq!(placements.len(), 1);
    }

    #[test]
    fn shared_host_image_survives_while_another_source_references_it() {
        fn twin_placement() -> HostPlacement {
            let mut twin = test_placement(5, 5);
            twin.placement.image_id = 8;
            twin.placement.placement_id = 4;
            twin.source_key = HostSourceKey::Terminal {
                pane_id: twin.pane_id,
                image_id: twin.placement.image_id,
            };
            twin
        }

        let mut images = HashMap::new();
        let mut placements = HashMap::new();
        let mut sources = HashMap::new();
        let mut bytes = Vec::new();

        encode_graphics_update(
            &mut bytes,
            &[test_placement(0, 0), twin_placement()],
            false,
            &mut images,
            &mut placements,
            &mut sources,
        );
        assert_eq!(images.len(), 1, "same content dedups to one host image");

        // One source moves to new content while the other still shows the
        // old image: the shared host image must survive.
        bytes.clear();
        let mut changed = test_placement(0, 0);
        changed.placement.data_fingerprint = 43;
        encode_graphics_update(
            &mut bytes,
            &[changed, twin_placement()],
            false,
            &mut images,
            &mut placements,
            &mut sources,
        );

        let update = String::from_utf8_lossy(&bytes);
        assert!(!update.contains("a=d,d=I"), "shared host image survives");
        assert_eq!(images.len(), 2);
    }

    #[test]
    fn stale_source_entry_does_not_block_superseded_image_delete() {
        fn twin_placement() -> HostPlacement {
            let mut twin = test_placement(5, 5);
            twin.placement.image_id = 8;
            twin.placement.placement_id = 4;
            twin.source_key = HostSourceKey::Terminal {
                pane_id: twin.pane_id,
                image_id: twin.placement.image_id,
            };
            twin
        }

        let mut images = HashMap::new();
        let mut placements = HashMap::new();
        let mut sources = HashMap::new();
        let mut bytes = Vec::new();

        encode_graphics_update(
            &mut bytes,
            &[test_placement(0, 0), twin_placement()],
            false,
            &mut images,
            &mut placements,
            &mut sources,
        );
        assert_eq!(images.len(), 1);
        assert_eq!(sources.len(), 2);
        let shared_host_id = *images.keys().next().expect("uploaded host image");

        // The twin source is gone and the survivor changed content: the
        // vanished source's stale entry must not keep the old host image
        // alive.
        bytes.clear();
        let mut changed = test_placement(0, 0);
        changed.placement.data_fingerprint = 43;
        encode_graphics_update(
            &mut bytes,
            &[changed],
            false,
            &mut images,
            &mut placements,
            &mut sources,
        );

        let update = String::from_utf8_lossy(&bytes);
        assert!(
            update.contains(&format!("a=d,d=I,i={shared_host_id}")),
            "old host image is deleted once its last live source moves on"
        );
        assert_eq!(images.len(), 1);
        assert_eq!(sources.len(), 1);
    }

    // H49-4 (V4.TN-3): a source that leaves a shared image and comes back.
    // TP-GFX-STABLE-01
    #[test]
    fn a_streaming_retransmit_never_deletes_the_image_on_screen() {
        // The stable-identity fix made every new frame a retransmit of the
        // SAME host image id — but the encoder's refresh path deleted the
        // image before uploading the replacement, so the live picture blinked
        // off for the gap between delete and display on every single frame
        // (the 2026-08-29 16:2x live report: the pane "opens and closes"
        // continuously). Kitty replaces an image in place when the same id is
        // retransmitted; a delete must never travel between frames of a live
        // streaming source.
        fn frame(fingerprint: u64) -> HostPlacement {
            let mut frame = test_placement(3, 3);
            frame.placement.image_id = 9;
            frame.placement.placement_id = 2;
            frame.placement.data_fingerprint = fingerprint;
            frame.host_image_id = Some(510_000);
            frame.source_key = HostSourceKey::Terminal {
                pane_id: frame.pane_id,
                image_id: frame.placement.image_id,
            };
            frame
        }

        let mut cache = HostGraphicsCache::default();
        let live = HashSet::new();

        let bytes = drain_graphics_updates(&mut cache, &[frame(1)], &live);
        let first = String::from_utf8_lossy(&bytes);
        assert!(first.contains("i=510000"), "first frame uploads: {first}");

        let bytes = drain_graphics_updates(&mut cache, &[frame(2)], &live);
        let update = String::from_utf8_lossy(&bytes);
        assert!(
            !update.contains("a=d,d=I,i=510000"),
            "a retransmit must replace the image in place, never delete it \
             from the screen first: {update}"
        );
        assert!(
            update.contains("i=510000"),
            "the new content still travels: {update}"
        );
        assert_eq!(cache.images.len(), 1, "one image, refreshed in place");
        assert_eq!(cache.placements.len(), 1, "the placement survives");
    }

    #[test]
    fn a_source_returning_to_a_shared_image_displays_its_placement_again() {
        // Two sources share one content-hashed image. One moves to new
        // content and then returns to the old. Driven through the
        // INCREMENTAL encoder (the live path) — the test-shim helper routes
        // terminal-only frames to the legacy encoder, which the wire proved
        // dead (d=i count zero), so the cache is built by hand here.
        fn twin(fingerprint: u64) -> HostPlacement {
            let mut twin = test_placement(5, 5);
            twin.placement.image_id = 8;
            twin.placement.placement_id = 4;
            twin.placement.data_fingerprint = fingerprint;
            twin.source_key = HostSourceKey::Terminal {
                pane_id: twin.pane_id,
                image_id: twin.placement.image_id,
            };
            twin
        }

        let mut cache = HostGraphicsCache::default();
        let live = HashSet::new();

        // Turn 1: both sources on the shared image.
        let bytes = drain_graphics_updates(&mut cache, &[test_placement(0, 0), twin(42)], &live);
        let update = String::from_utf8_lossy(&bytes);
        assert_eq!(cache.images.len(), 1, "one shared host image: {update}");
        let shared_host_id = *cache.images.keys().next().expect("uploaded host image");
        assert_eq!(cache.placements.len(), 2, "both placements live: {update}");

        // Turn 2: the twin moves to new content — its old placement on the
        // shared image must be deleted (d=i), the shared image must survive.
        let bytes = drain_graphics_updates(&mut cache, &[test_placement(0, 0), twin(43)], &live);
        let update = String::from_utf8_lossy(&bytes);
        let twin_pid = host_placement_id(&twin(42).source_key, &twin(42).placement);
        assert!(
            update.contains(&format!("a=d,d=i,i={shared_host_id},p={twin_pid}")),
            "the twin's old placement on the shared image is deleted: {update}"
        );
        assert!(
            !update.contains(&format!("a=d,d=I,i={shared_host_id}")),
            "the shared image itself survives, the survivor still uses it: {update}"
        );
        assert_eq!(
            cache.images.len(),
            2,
            "shared image plus the twin's new one"
        );

        // Turn 3: the twin returns to the original content. Turn 2 removed
        // (shared_id, 4) from the cache, so the display must be emitted
        // again; a skipped display here is exactly a stranded blank/stale
        // cell on the terminal.
        let bytes = drain_graphics_updates(&mut cache, &[test_placement(0, 0), twin(42)], &live);
        let update = String::from_utf8_lossy(&bytes);
        assert!(
            update.contains(&format!("i={shared_host_id},p={twin_pid}")) && update.contains("a=p"),
            "the returning source's placement is displayed again: {update}"
        );
        assert_eq!(
            cache.placements.len(),
            2,
            "both placements live again: {update}"
        );
        assert_eq!(
            cache.images.len(),
            1,
            "the twin's abandoned image is released: {update}"
        );
    }

    #[test]
    fn stale_placement_deletes_placement_not_image_immediately() {
        let mut images = HashMap::new();
        let mut placements = HashMap::new();
        let mut sources = HashMap::new();
        let mut bytes = Vec::new();
        let placement = test_placement(0, 0);

        encode_graphics_update(
            &mut bytes,
            &[placement],
            false,
            &mut images,
            &mut placements,
            &mut sources,
        );
        assert_eq!(placements.len(), 1);

        bytes.clear();
        encode_graphics_update(
            &mut bytes,
            &[],
            false,
            &mut images,
            &mut placements,
            &mut sources,
        );
        let delete = String::from_utf8_lossy(&bytes);
        assert!(delete.contains("a=d,d=i"));
        assert!(!delete.contains("d=I"));
        assert!(placements.is_empty());
        assert_eq!(images.len(), 1);
    }

    #[test]
    fn trusted_direct_image_uses_reserved_id_for_placement_without_upload() {
        let key = (PaneId::from_raw(1), "primary".to_owned());
        let layer = crate::app::pane_graphics::Layer::inline(
            crate::api::schema::PaneGraphicsFormat::Rgba,
            30,
            30,
            vec![255; 30 * 30 * 4],
            Default::default(),
            0,
        );
        let reserved_id = (1 << 31) | 77;
        let mut cache = HostGraphicsCache::default();
        cache.trust_pane_layer(&key, reserved_id, &layer);
        let mut placement = pane_layer_placement(0, 0);
        placement.host_image_id = Some(reserved_id);
        placement.placement.data.clear();
        placement.placement.data_len = layer.data_len();
        placement.placement.data_fingerprint = layer.data_fingerprint;
        let mut bytes = Vec::new();

        encode_graphics_update(
            &mut bytes,
            &[placement],
            false,
            &mut cache.images,
            &mut cache.placements,
            &mut cache.sources,
        );

        let update = String::from_utf8_lossy(&bytes);
        assert!(update.contains(&format!("a=p,i={reserved_id}")));
        assert!(!update.contains("a=t"));

        let live = HashSet::from([HostSourceKey::PaneLayer {
            pane_id: key.0,
            layer_id: key.1.clone(),
        }]);
        let hidden = String::from_utf8(cache.hide_except_live_pane_layers(&live)).unwrap();
        assert!(hidden.contains("a=d,d=i"));
        assert!(!hidden.contains("a=d,d=I"));
        assert!(cache.images.contains_key(&reserved_id));
        assert!(cache.placements.is_empty());

        let mut returning = pane_layer_placement(0, 0);
        returning.host_image_id = Some(reserved_id);
        returning.placement.data.clear();
        returning.placement.data_len = layer.data_len();
        returning.placement.data_fingerprint = layer.data_fingerprint;
        let mut replay = Vec::new();
        encode_graphics_update(
            &mut replay,
            &[returning],
            false,
            &mut cache.images,
            &mut cache.placements,
            &mut cache.sources,
        );
        let replay = String::from_utf8(replay).unwrap();
        assert!(replay.contains(&format!("a=p,i={reserved_id}")));
        assert!(!replay.contains("a=t"));

        cache.forget_pane_layer(&key, reserved_id);
        let mut fallback = pane_layer_placement(0, 0);
        fallback.host_image_id = Some(reserved_id);
        let mut retransmit = Vec::new();
        encode_graphics_update(
            &mut retransmit,
            &[fallback],
            false,
            &mut cache.images,
            &mut cache.placements,
            &mut cache.sources,
        );
        assert!(String::from_utf8_lossy(&retransmit).contains("a=t"));
    }

    #[test]
    fn hidden_layer_and_full_redraw_replay_placement_without_pixels() {
        let mut images = HashMap::new();
        let mut placements = HashMap::new();
        let mut sources = HashMap::new();
        let mut bytes = Vec::new();
        let placement = pane_layer_placement(0, 0);
        encode_graphics_update(
            &mut bytes,
            &[placement],
            false,
            &mut images,
            &mut placements,
            &mut sources,
        );

        for (visible, replay) in [(false, false), (true, false), (true, true)] {
            bytes.clear();
            let current = visible.then(|| pane_layer_placement(0, 0));
            encode_graphics_update(
                &mut bytes,
                current.as_slice(),
                replay,
                &mut images,
                &mut placements,
                &mut sources,
            );
            let update = String::from_utf8_lossy(&bytes);
            assert!(!update.contains("a=t"));
            assert!(!update.contains("a=d,d=I"));
            assert_eq!(update.contains("a=p"), visible);
        }
        assert_eq!(images.len(), 1);
        assert_eq!(sources.len(), 1);
    }

    #[test]
    fn removed_pane_layer_deletes_unreferenced_host_image() {
        let mut cache = HostGraphicsCache::default();
        let mut bytes = Vec::new();
        encode_graphics_update(
            &mut bytes,
            &[pane_layer_placement(0, 0)],
            false,
            &mut cache.images,
            &mut cache.placements,
            &mut cache.sources,
        );
        let host_id = *cache.images.keys().next().expect("uploaded pane layer");

        bytes = drain_graphics_updates(&mut cache, &[], &HashSet::new());

        let delete = String::from_utf8_lossy(&bytes);
        assert!(delete.contains(&format!("a=d,d=I,i={host_id}")));
        assert!(cache.images.is_empty());
        assert!(cache.placements.is_empty());
        assert!(cache.sources.is_empty());
    }

    #[test]
    fn hidden_pane_layer_retains_image_and_removes_only_placement() {
        let mut images = HashMap::new();
        let mut placements = HashMap::new();
        let mut sources = HashMap::new();
        let mut bytes = Vec::new();
        encode_graphics_update(
            &mut bytes,
            &[pane_layer_placement(0, 0)],
            false,
            &mut images,
            &mut placements,
            &mut sources,
        );
        let host_id = *images.keys().next().expect("uploaded pane layer");

        bytes.clear();
        encode_graphics_update(
            &mut bytes,
            &[pane_layer_placement(100, 100)],
            false,
            &mut images,
            &mut placements,
            &mut sources,
        );

        let update = String::from_utf8_lossy(&bytes);
        assert!(update.contains("a=d,d=i"));
        assert!(!update.contains(&format!("a=d,d=I,i={host_id}")));
        assert_eq!(images.len(), 1);
        assert!(placements.is_empty());
        assert_eq!(sources.len(), 1);
    }

    #[test]
    fn clipped_terminal_source_retains_identity_for_later_content_replacement() {
        let mut images = HashMap::new();
        let mut placements = HashMap::new();
        let mut sources = HashMap::new();
        let mut bytes = Vec::new();
        encode_graphics_update(
            &mut bytes,
            &[test_placement(0, 0)],
            false,
            &mut images,
            &mut placements,
            &mut sources,
        );
        let original_host_id = *images.keys().next().expect("uploaded terminal image");

        bytes.clear();
        encode_graphics_update(
            &mut bytes,
            &[test_placement(100, 100)],
            false,
            &mut images,
            &mut placements,
            &mut sources,
        );
        assert_eq!(images.len(), 1);
        assert_eq!(sources.len(), 1);

        bytes.clear();
        let mut changed = test_placement(0, 0);
        changed.placement.data_fingerprint = 43;
        encode_graphics_update(
            &mut bytes,
            &[changed],
            false,
            &mut images,
            &mut placements,
            &mut sources,
        );

        let update = String::from_utf8_lossy(&bytes);
        assert!(update.contains(&format!("a=d,d=I,i={original_host_id}")));
        assert_eq!(images.len(), 1);
        assert_eq!(sources.len(), 1);
    }

    #[test]
    fn removed_pane_layer_preserves_image_shared_with_terminal_source() {
        let mut cache = HostGraphicsCache::default();
        let mut bytes = Vec::new();
        encode_graphics_update(
            &mut bytes,
            &[pane_layer_placement(0, 0), test_placement(4, 0)],
            false,
            &mut cache.images,
            &mut cache.placements,
            &mut cache.sources,
        );
        assert_eq!(cache.images.len(), 1);

        bytes = drain_graphics_updates(&mut cache, &[], &HashSet::new());
        encode_graphics_update(
            &mut bytes,
            &[test_placement(4, 0)],
            false,
            &mut cache.images,
            &mut cache.placements,
            &mut cache.sources,
        );

        let update = String::from_utf8_lossy(&bytes);
        assert!(!update.contains("a=d,d=I"));
        assert_eq!(cache.images.len(), 1);
        assert_eq!(cache.placements.len(), 1);
        assert_eq!(cache.sources.len(), 1);
    }

    #[test]
    fn changing_first_source_does_not_starve_second_source() {
        let layers = |first| {
            [(1, "a", first), (2, "b", 80)].map(|(id, name, fingerprint)| {
                let mut placement = pane_layer_placement(0, 0);
                placement.host_image_id = Some(PANE_GRAPHICS_IMAGE_ID_BIT | id);
                placement.source_key = HostSourceKey::PaneLayer {
                    pane_id: placement.pane_id,
                    layer_id: name.into(),
                };
                placement.placement.data_fingerprint = fingerprint;
                placement
            })
        };
        let initial = layers(42);
        let live = initial.iter().map(|p| p.source_key.clone()).collect();
        let mut cache = HostGraphicsCache::default();
        assert!(encode_graphics_update_incremental(&mut cache, &initial, &live, None).incomplete);
        assert!(
            encode_graphics_update_incremental(&mut cache, &layers(43), &live, None).incomplete
        );
        assert_eq!(cache.images.len(), 2, "second source uploaded next");

        let terminal = |id| {
            let mut placement = test_placement(0, 0);
            placement.placement.image_id = id;
            placement.placement.data_fingerprint = u64::from(id);
            placement.source_key = HostSourceKey::Terminal {
                pane_id: placement.pane_id,
                image_id: id,
            };
            placement
        };
        let second = terminal(99).source_key;
        let mut cache = HostGraphicsCache::default();
        for id in 1..=3 {
            assert!(
                encode_graphics_update_incremental(
                    &mut cache,
                    &[terminal(id), terminal(99)],
                    &HashSet::new(),
                    None,
                )
                .incomplete
            );
        }
        assert!(cache.sources.contains_key(&second));
    }

    #[test]
    fn large_terminal_image_is_local_but_quarantined_headless() {
        let placements = || {
            let mut large = test_placement(0, 0);
            large.placement.data_len = 24 * 1024 * 1024;
            let mut later = test_placement(4, 0);
            later.placement.image_id = 8;
            later.source_key = HostSourceKey::Terminal {
                pane_id: later.pane_id,
                image_id: 8,
            };
            [large, later]
        };
        for (budget, expected) in [
            (None, (true, 1, 0)),
            (Some(HEADLESS_GRAPHICS_TRANSACTION_BUDGET), (false, 1, 1)),
        ] {
            let mut cache = HostGraphicsCache::default();
            let encoded = encode_graphics_update_incremental(
                &mut cache,
                &placements(),
                &HashSet::new(),
                budget,
            );
            assert!(String::from_utf8_lossy(&encoded.bytes).contains("a=t"));
            assert_eq!(
                (
                    encoded.incomplete,
                    cache.images.len(),
                    cache.oversized.len()
                ),
                expected
            );
        }
    }

    #[test]
    fn maximum_pane_graphics_stream_payload_fits_client_graphics_frame() {
        let mut placement = pane_layer_placement(0, 0);
        placement.placement.format = KittyImageFormat::Png;
        placement.placement.image_width = 1;
        placement.placement.image_height = 1;
        placement.placement.data = vec![1_u8; crate::api::schema::PANE_GRAPHICS_STREAM_MAX_BYTES];
        placement.placement.data_len = placement.placement.data.len();
        let (clipped, format_code) = clipped_placement(&placement).expect("visible placement");
        let host_id = host_image_id(placement.pane_id, &placement.placement);
        let mut encoded = Vec::new();

        assert!(encode_upload_image(
            &mut encoded,
            &placement,
            format_code,
            host_id,
        ));
        encode_display_placement(&mut encoded, clipped, host_id, 1, 0);

        let mut framed = Vec::new();
        crate::protocol::write_message(
            &mut framed,
            &crate::protocol::ServerMessage::Graphics { bytes: encoded },
        )
        .unwrap();
        assert!(framed.len() <= crate::protocol::MAX_GRAPHICS_FRAME_SIZE + 4);
    }
}
