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
use crate::layout::PaneId;
use crate::terminal::TerminalRuntimeRegistry;

const KITTY_CHUNK_BYTES: usize = 3072;
const HOST_IMAGE_ID_BASE: u32 = 10_000;
const FILE_MANAGER_PREVIEW_PANE_RAW: u32 = u32::MAX;
const FILE_MANAGER_PREVIEW_IMAGE_ID: u32 = 1;
const FILE_MANAGER_PREVIEW_PLACEMENT_ID: u32 = 1;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct HostCellSize {
    pub width_px: u32,
    pub height_px: u32,
}

impl HostCellSize {
    pub(crate) fn from_terminal(area: Rect) -> Self {
        let Ok(size) = crossterm::terminal::window_size() else {
            return Self::fallback_for_area(area);
        };
        if size.columns == 0 || size.rows == 0 {
            return Self::fallback_for_area(area);
        }
        if size.width == 0 || size.height == 0 {
            return Self::fallback_for_area(area);
        }
        Self {
            width_px: (size.width as u32 / size.columns as u32).max(1),
            height_px: (size.height as u32 / size.rows as u32).max(1),
        }
        .for_area(area)
    }

    pub(crate) fn is_known(self) -> bool {
        self.width_px > 0 && self.height_px > 0
    }

    fn fallback_for_area(area: Rect) -> Self {
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
    area: Rect,
    cell_size: HostCellSize,
    placement: KittyImagePlacement,
    scrollback_offset: u32,
}

fn file_manager_image_geometry(
    file_manager_area: Rect,
    cell_size: HostCellSize,
) -> Option<(Rect, ImagePreviewTarget)> {
    file_manager_image_geometry_with(
        file_manager_area,
        cell_size,
        crate::fm::miller::MillerTrioOverrides::default(),
    )
}

fn file_manager_image_geometry_with(
    file_manager_area: Rect,
    cell_size: HostCellSize,
    overrides: crate::fm::miller::MillerTrioOverrides,
) -> Option<(Rect, ImagePreviewTarget)> {
    if !cell_size.is_known() {
        return None;
    }
    let area = crate::ui::file_manager_preview_content_area_with(file_manager_area, overrides)?;
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
    file_manager_area: Rect,
    cell_size: HostCellSize,
) -> Option<ImagePreviewTarget> {
    file_manager_image_geometry(file_manager_area, cell_size).map(|(_, target)| target)
}

#[cfg(test)]
fn file_manager_image_placement(
    file_manager_area: Rect,
    cell_size: HostCellSize,
    prepared: &PreparedImagePreview,
) -> Option<HostPlacement> {
    file_manager_image_placement_with_data(file_manager_area, cell_size, prepared, true)
}

#[allow(dead_code)] // Default-override seam kept for callers without overrides.
fn file_manager_image_placement_with_data(
    file_manager_area: Rect,
    cell_size: HostCellSize,
    prepared: &PreparedImagePreview,
    include_data: bool,
) -> Option<HostPlacement> {
    file_manager_image_placement_with_overrides(
        file_manager_area,
        cell_size,
        prepared,
        include_data,
        crate::fm::miller::MillerTrioOverrides::default(),
    )
}

fn file_manager_image_placement_with_overrides(
    file_manager_area: Rect,
    cell_size: HostCellSize,
    prepared: &PreparedImagePreview,
    include_data: bool,
    overrides: crate::fm::miller::MillerTrioOverrides,
) -> Option<HostPlacement> {
    let (area, target) = file_manager_image_geometry_with(file_manager_area, cell_size, overrides)?;
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
        pane_id: PaneId::from_raw(FILE_MANAGER_PREVIEW_PANE_RAW),
        area,
        cell_size,
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
    let crate::fm::FmPreview::File(crate::fm::FmFilePreview::Image(preview)) =
        &file_manager.preview
    else {
        return None;
    };
    let crate::fm::FmImagePreviewState::Ready { target, prepared } = &preview.state else {
        return None;
    };
    let overrides = app
        .file_manager
        .as_ref()
        .map(|file_manager| file_manager.trio_overrides)
        .unwrap_or_default();
    if file_manager_image_geometry_with(app.view.terminal_area, cell_size, overrides)
        .map(|(_, target)| target)?
        != *target
    {
        return None;
    }

    let mut placement = file_manager_image_placement_with_overrides(
        app.view.terminal_area,
        cell_size,
        prepared,
        false,
        overrides,
    )?;
    let format_code = kitty_format_code(placement.placement.format);
    let signature = image_signature(&placement, format_code);
    let host_id = host_image_id(placement.pane_id, &placement.placement);
    if uploaded_images.get(&host_id).copied() != Some(signature) {
        placement.placement.data = prepared.rgba.clone();
    }
    Some(placement)
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
    sources: HashMap<(PaneId, u32), u32>,
    view: Option<HostViewKey>,
}

static KITTY_GRAPHICS_ENABLED: AtomicBool = AtomicBool::new(false);
static LOCAL_HOST_GRAPHICS: OnceLock<Mutex<HostGraphicsCache>> = OnceLock::new();

pub(crate) fn set_enabled(enabled: bool) {
    KITTY_GRAPHICS_ENABLED.store(enabled, Ordering::Release);
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
    terminal_runtimes: &TerminalRuntimeRegistry,
    cell_size: HostCellSize,
) -> io::Result<()> {
    let cache = LOCAL_HOST_GRAPHICS.get_or_init(|| Mutex::new(HostGraphicsCache::default()));
    let mut bytes = Vec::new();
    if let Ok(mut cache) = cache.lock() {
        bytes = encode_local_pane_graphics(app, terminal_runtimes, cell_size, &mut cache);
    }
    if bytes.is_empty() {
        return Ok(());
    }

    let framed = frame_graphics_bytes(&bytes);

    let mut stdout = io::stdout().lock();
    stdout.write_all(&framed)?;
    stdout.flush()
}

pub(crate) fn encode_local_pane_graphics(
    app: &AppState,
    terminal_runtimes: &TerminalRuntimeRegistry,
    cell_size: HostCellSize,
    cache: &mut HostGraphicsCache,
) -> Vec<u8> {
    let mode_ok = app.mode == Mode::Terminal;
    let cell_ok = cell_size.is_known();
    tracing::debug!(
        mode_ok,
        cell_ok,
        cell_width_px = cell_size.width_px,
        cell_height_px = cell_size.height_px,
        active = ?app.active,
        pane_infos_len = app.view.pane_infos.len(),
        "paint_local_pane_graphics entry"
    );
    if !mode_ok || !cell_ok {
        tracing::debug!(
            reason = if !mode_ok {
                "not terminal mode"
            } else {
                "cell size unknown"
            },
            "paint_local_pane_graphics early return"
        );
        return cache.clear_bytes();
    }

    let view_key = active_view_key(app);
    let surface_changed = cache
        .view
        .as_ref()
        .zip(view_key.as_ref())
        .is_some_and(|(previous, next)| previous.file_manager_open != next.file_manager_open)
        || cache
            .view
            .as_ref()
            .is_some_and(|previous| previous.file_manager_open && view_key.is_none());
    let mut bytes = if surface_changed {
        cache.clear_bytes()
    } else {
        Vec::new()
    };
    let uploaded_images = cache.images.clone();
    let placements = if app.file_manager.is_some() {
        collect_file_manager_image_placement(app, cell_size, &uploaded_images)
            .into_iter()
            .collect()
    } else {
        collect_visible_placements(app, terminal_runtimes, cell_size, &uploaded_images)
    };
    tracing::debug!(
        placements_collected = placements.len(),
        "collect_visible_placements result"
    );

    let file_manager_source = (
        PaneId::from_raw(FILE_MANAGER_PREVIEW_PANE_RAW),
        FILE_MANAGER_PREVIEW_IMAGE_ID,
    );
    if app.file_manager.is_some()
        && placements.is_empty()
        && cache.sources.contains_key(&file_manager_source)
    {
        bytes.extend(cache.clear_bytes());
    }
    let view_changed = cache.update_view(view_key);
    encode_graphics_update(
        &mut bytes,
        &placements,
        view_changed,
        &mut cache.images,
        &mut cache.placements,
        &mut cache.sources,
    );
    tracing::debug!(
        placements = placements.len(),
        bytes = bytes.len(),
        cell_width_px = cell_size.width_px,
        cell_height_px = cell_size.height_px,
        "painting kitty graphics placements"
    );
    bytes
}

pub(crate) fn has_visible_pane_graphics(
    app: &AppState,
    terminal_runtimes: &TerminalRuntimeRegistry,
    cell_size: HostCellSize,
) -> bool {
    if app.mode != Mode::Terminal || !cell_size.is_known() {
        return false;
    }

    let Some(ws_idx) = app.active else {
        return false;
    };
    if app
        .workspaces
        .get(ws_idx)
        .and_then(crate::workspace::Workspace::active_tab)
        .is_none()
    {
        return false;
    }

    for info in &app.view.pane_infos {
        let Some(runtime) = app.runtime_for_pane_in_workspace(terminal_runtimes, ws_idx, info.id)
        else {
            continue;
        };
        let scrollback_offset = runtime
            .scroll_metrics()
            .map(|m| m.offset_from_bottom as u32)
            .unwrap_or(0);
        for placement in runtime.kitty_image_placements_with_data_filter(|_| false) {
            let host_placement = HostPlacement {
                pane_id: info.id,
                area: info.inner_rect,
                cell_size,
                placement,
                scrollback_offset,
            };
            if clipped_placement(&host_placement).is_some() {
                return true;
            }
        }
    }
    false
}

fn encode_graphics_update(
    bytes: &mut Vec<u8>,
    placements: &[HostPlacement],
    view_changed: bool,
    host_images: &mut HashMap<u32, ImageSignature>,
    host_placements: &mut HashMap<(u32, u32), PlacementSignature>,
    sources: &mut HashMap<(PaneId, u32), u32>,
) {
    // Prune sources that are no longer visible: a stale entry would keep its
    // old host image referenced and block the superseded-image delete.
    let current_sources: HashSet<(PaneId, u32)> = placements
        .iter()
        .map(|placement| (placement.pane_id, placement.placement.image_id))
        .collect();
    sources.retain(|source, _| current_sources.contains(source));

    let mut current_placements = HashSet::new();
    for placement in placements {
        let clipped = clipped_placement(placement);
        tracing::debug!(
            pane_id = ?placement.pane_id,
            has_clipped = clipped.is_some(),
            grid_cols = placement.placement.render.grid_cols,
            grid_rows = placement.placement.render.grid_rows,
            viewport_col = placement.placement.render.viewport_col,
            viewport_row = placement.placement.render.viewport_row,
            area_w = placement.area.width,
            area_h = placement.area.height,
            "clipped_placement result"
        );
        let Some((clipped, format_code)) = clipped else {
            continue;
        };
        let host_id = host_image_id(placement.pane_id, &placement.placement);
        let host_placement_id = host_placement_id(placement.pane_id, &placement.placement);
        let image_signature = image_signature(placement, format_code);
        let placement_signature =
            placement_signature(clipped, placement.placement.z, placement.scrollback_offset);
        let placement_key = (host_id, host_placement_id);
        current_placements.insert(placement_key);

        match host_images.get(&host_id).copied() {
            Some(existing) if existing == image_signature => {}
            Some(_) => {
                encode_delete_image(bytes, host_id);
                host_placements.retain(|(image_id, placement_id), _| {
                    if *image_id == host_id {
                        current_placements.remove(&(*image_id, *placement_id));
                        false
                    } else {
                        true
                    }
                });
                if !encode_upload_image(bytes, placement, format_code, host_id) {
                    continue;
                }
                host_images.insert(host_id, image_signature);
            }
            None => {
                if !encode_upload_image(bytes, placement, format_code, host_id) {
                    continue;
                }
                host_images.insert(host_id, image_signature);
            }
        }

        release_superseded_source_image(
            bytes,
            sources,
            host_images,
            host_placements,
            &mut current_placements,
            (placement.pane_id, placement.placement.image_id),
            host_id,
        );

        // A different view can repaint the same cells with text or overlays and
        // leave the host-side Kitty placement state out of sync with this cache.
        // Re-emit the placement even when its geometry signature is unchanged.
        match host_placements.get_mut(&placement_key) {
            Some(existing) if !view_changed && *existing == placement_signature => {}
            Some(existing) => {
                encode_display_placement(
                    bytes,
                    clipped,
                    host_id,
                    host_placement_id,
                    placement.placement.z,
                );
                *existing = placement_signature;
            }
            None => {
                encode_display_placement(
                    bytes,
                    clipped,
                    host_id,
                    host_placement_id,
                    placement.placement.z,
                );
                host_placements.insert(placement_key, placement_signature);
            }
        }
    }

    let mut stale_placements = Vec::new();
    for key in host_placements.keys() {
        if current_placements.contains(key) {
            continue;
        }
        stale_placements.push(*key);
    }
    for (host_id, host_placement_id) in stale_placements {
        encode_delete_placement(bytes, host_id, host_placement_id);
        host_placements.remove(&(host_id, host_placement_id));
    }
}

/// Records that `source` is now backed by `host_id` and deletes the host
/// image it previously pointed at once no other source references it.
fn release_superseded_source_image(
    bytes: &mut Vec<u8>,
    sources: &mut HashMap<(PaneId, u32), u32>,
    host_images: &mut HashMap<u32, ImageSignature>,
    host_placements: &mut HashMap<(u32, u32), PlacementSignature>,
    current_placements: &mut HashSet<(u32, u32)>,
    source: (PaneId, u32),
    host_id: u32,
) {
    let Some(previous) = sources.insert(source, host_id) else {
        return;
    };
    if previous == host_id || sources.values().any(|id| *id == previous) {
        return;
    }
    encode_delete_image(bytes, previous);
    host_images.remove(&previous);
    // The `d=I` delete also removes the image's placements host-side.
    host_placements.retain(|(image_id, placement_id), _| {
        if *image_id == previous {
            current_placements.remove(&(*image_id, *placement_id));
            false
        } else {
            true
        }
    });
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
    pub(crate) fn is_empty(&self) -> bool {
        self.images.is_empty() && self.placements.is_empty()
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
    }

    pub(crate) fn clear_bytes(&mut self) -> Vec<u8> {
        let mut bytes = Vec::new();
        for id in self.images.keys().copied().collect::<Vec<_>>() {
            encode_delete_image(&mut bytes, id);
        }
        self.images.clear();
        self.placements.clear();
        self.sources.clear();
        self.view = None;
        bytes
    }

    fn update_view(&mut self, view_key: Option<HostViewKey>) -> bool {
        if self.view == view_key {
            return false;
        }
        self.view = view_key;
        true
    }
}

fn active_view_key(app: &AppState) -> Option<HostViewKey> {
    if app.file_manager.is_some() {
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

fn collect_visible_placements(
    app: &AppState,
    terminal_runtimes: &TerminalRuntimeRegistry,
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
        pane_infos_len = app.view.pane_infos.len(),
        "collect_visible_placements: starting iteration"
    );
    let mut placements = Vec::new();
    for info in &app.view.pane_infos {
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
            let host_id = host_image_id_for_signature(info.id, signature);
            uploaded_images.get(&host_id).copied() != Some(signature)
        }) {
            let scrollback_offset = runtime
                .scroll_metrics()
                .map(|m| m.offset_from_bottom as u32)
                .unwrap_or(0);
            placements.push(HostPlacement {
                pane_id: info.id,
                area: info.inner_rect,
                cell_size,
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

fn host_placement_id(pane_id: PaneId, placement: &KittyImagePlacement) -> u32 {
    let mut hasher = DefaultHasher::new();
    pane_id.raw().hash(&mut hasher);
    placement.image_id.hash(&mut hasher);
    placement.placement_id.hash(&mut hasher);
    1 + ((hasher.finish() as u32) % 900_000)
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

    let _ = write!(out, "\x1b_G{control};\x1b\\");
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

fn encode_kitty_data(out: &mut Vec<u8>, control: &str, data: &[u8]) {
    let mut chunks = data.chunks(KITTY_CHUNK_BYTES).peekable();
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
mod tests {
    use super::*;
    use crate::fm::image_preview::{ImagePreviewTarget, PreparedImagePreview};
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
            area: Rect::new(2, 2, 8, 4),
            cell_size: HostCellSize {
                width_px: 1,
                height_px: 1,
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
            host_placement_id(first.pane_id, &first.placement),
            host_placement_id(same.pane_id, &same.placement)
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

    #[test]
    fn file_manager_image_target_uses_only_the_responsive_preview_content_area() {
        let cells = HostCellSize {
            width_px: 8,
            height_px: 16,
        };

        assert_eq!(
            file_manager_image_target(Rect::new(10, 5, 24, 10), cells),
            None,
            "one-column layout has no preview slot"
        );
        assert_eq!(
            file_manager_image_target(Rect::new(10, 5, 25, 10), cells),
            Some(ImagePreviewTarget {
                width_px: 96,
                height_px: 112,
            }),
            "two-column preview excludes the FM header, status, and PREVIEW title"
        );
        assert_eq!(
            file_manager_image_target(Rect::new(10, 5, 38, 10), cells),
            Some(ImagePreviewTarget {
                width_px: 96,
                height_px: 112,
            }),
            "three-column preview consumes the same named preview content seam"
        );
        assert_eq!(
            file_manager_image_target(Rect::new(10, 5, 38, 2), cells),
            None,
            "header-only preview column has no image content rows"
        );
        assert_eq!(
            file_manager_image_target(Rect::new(10, 5, 38, 10), HostCellSize::default()),
            None,
            "unknown host cell pixels cannot produce a decode target"
        );
        assert_eq!(
            file_manager_image_target(
                Rect::new(10, 5, 38, 10),
                HostCellSize {
                    width_px: u32::MAX,
                    height_px: u32::MAX,
                },
            ),
            None,
            "pixel multiplication overflow is rejected"
        );
    }

    // P3 RED: decode target and host placement must consume the same typed
    // preview content rect as text render. Width 57 is adversarial because the
    // legacy trio independently creates three narrow columns while the frozen
    // Miller snapshot owns current + preview at 28 cells each.
    #[test]
    fn file_manager_image_target_matches_windowed_snapshot_preview_rect() {
        let frame = Rect::new(0, 0, 57, 10);
        let cells = HostCellSize {
            width_px: 8,
            height_px: 16,
        };
        let current = std::path::PathBuf::from("/virtual/current");
        let mut file_manager = crate::fm::FmState::test_empty(current.clone());
        file_manager.miller.chain =
            std::iter::once(crate::fm::miller::MillerPathSegment::new(current.clone())).collect();
        file_manager.miller.focused_directory = current;
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
        let preview_content = app
            .view
            .file_manager_miller
            .columns
            .iter()
            .find(|column| column.kind.is_preview())
            .expect("typed preview column")
            .content_rect;
        let expected = ImagePreviewTarget {
            width_px: u32::from(preview_content.width) * cells.width_px,
            height_px: u32::from(preview_content.height) * cells.height_px,
        };

        assert_eq!(
            file_manager_image_target(frame, cells),
            Some(expected),
            "decode target must match the exact windowed preview content rect"
        );
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
        app.mode = Mode::Terminal;
        app.view.terminal_area = Rect::new(10, 5, 38, 10);
        let mut file_manager = FmState::test_empty("/tmp");
        file_manager.cwd_writable = true;
        file_manager.preview = FmPreview::File(FmFilePreview::Image(FmImagePreview {
            source_path: "/tmp/preview.png".into(),
            generation: 1,
            state: FmImagePreviewState::Ready {
                target: ImagePreviewTarget {
                    width_px: 96,
                    height_px: 112,
                },
                prepared: first,
            },
        }));
        file_manager.preview_generation = 1;
        app.file_manager = Some(file_manager);
        let runtimes = TerminalRuntimeRegistry::new();
        let mut cache = HostGraphicsCache::default();

        let uncached = collect_file_manager_image_placement(&app, cells, &cache.images)
            .expect("ready image placement");
        assert!(!uncached.placement.data.is_empty());

        let first_bytes = encode_local_pane_graphics(&app, &runtimes, cells, &mut cache);
        let first_text = String::from_utf8_lossy(&first_bytes);
        assert!(first_text.contains("a=t,t=d,f=32,s=80,v=64"));
        assert!(first_text.contains("a=p"));
        assert!(first_text.contains("c=10,r=4"));
        assert!(first_text.contains("\x1b[9;38H"));

        let cached = collect_file_manager_image_placement(&app, cells, &cache.images)
            .expect("cached image placement metadata");
        assert!(
            cached.placement.data.is_empty(),
            "cached frame must not clone the prepared RGBA allocation"
        );
        assert!(
            encode_local_pane_graphics(&app, &runtimes, cells, &mut cache).is_empty(),
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
            target: ImagePreviewTarget {
                width_px: 96,
                height_px: 112,
            },
            prepared: PreparedImagePreview {
                width: 80,
                height: 64,
                data_fingerprint: 0x2222,
                rgba: vec![0x22; 80 * 64 * 4],
            },
        };
        let replacement = encode_local_pane_graphics(&app, &runtimes, cells, &mut cache);
        let replacement = String::from_utf8_lossy(&replacement);
        assert!(replacement.contains("a=d,d=I"));
        assert!(replacement.contains("a=t"));
        assert!(replacement.contains("a=p"));

        app.file_manager = None;
        let cleanup = encode_local_pane_graphics(&app, &runtimes, cells, &mut cache);
        let cleanup = String::from_utf8_lossy(&cleanup);
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

    fn test_placement(viewport_col: i32, viewport_row: i32) -> HostPlacement {
        HostPlacement {
            pane_id: PaneId::from_raw(1),
            area: Rect::new(0, 0, 20, 10),
            cell_size: HostCellSize {
                width_px: 10,
                height_px: 10,
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
    fn graphics_update_uploads_once_then_repositions_only() {
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
        let first = String::from_utf8_lossy(&bytes);
        assert!(first.contains("a=t"));
        assert!(first.contains("a=p"));

        bytes.clear();
        let same = test_placement(0, 0);
        encode_graphics_update(
            &mut bytes,
            &[same],
            false,
            &mut images,
            &mut placements,
            &mut sources,
        );
        assert!(bytes.is_empty());

        let mut z_changed = test_placement(0, 0);
        z_changed.placement.z = 1;
        encode_graphics_update(
            &mut bytes,
            &[z_changed],
            false,
            &mut images,
            &mut placements,
            &mut sources,
        );
        let z_changed_bytes = String::from_utf8_lossy(&bytes);
        assert!(!z_changed_bytes.contains("a=t"));
        assert!(z_changed_bytes.contains("a=p"));

        bytes.clear();
        let moved = test_placement(0, 1);
        encode_graphics_update(
            &mut bytes,
            &[moved],
            false,
            &mut images,
            &mut placements,
            &mut sources,
        );
        let moved_bytes = String::from_utf8_lossy(&bytes);
        assert!(!moved_bytes.contains("a=t"));
        assert!(moved_bytes.contains("a=p"));
    }

    #[test]
    fn view_change_redisplays_unchanged_visible_placement() {
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
        let same = test_placement(0, 0);
        encode_graphics_update(
            &mut bytes,
            &[same],
            true,
            &mut images,
            &mut placements,
            &mut sources,
        );
        let redisplay = String::from_utf8_lossy(&bytes);
        assert!(!redisplay.contains("a=t"));
        assert!(redisplay.contains("a=p"));
        assert_eq!(placements.len(), 1);
    }

    #[test]
    fn surface_reset_deletes_then_reuploads_and_redisplays_placement() {
        let mut cache = HostGraphicsCache::default();
        let mut bytes = Vec::new();
        let placement = test_placement(0, 0);

        encode_graphics_update(
            &mut bytes,
            &[placement],
            false,
            &mut cache.images,
            &mut cache.placements,
            &mut cache.sources,
        );
        assert_eq!(cache.images.len(), 1);
        assert_eq!(cache.placements.len(), 1);

        bytes = cache.clear_bytes();
        let same = test_placement(0, 0);
        encode_graphics_update(
            &mut bytes,
            &[same],
            false,
            &mut cache.images,
            &mut cache.placements,
            &mut cache.sources,
        );

        let redisplay = String::from_utf8_lossy(&bytes);
        assert!(redisplay.contains("a=d,d=I"));
        assert!(redisplay.contains("a=t"));
        assert!(redisplay.contains("a=p"));
        assert_eq!(cache.images.len(), 1);
        assert_eq!(cache.placements.len(), 1);
    }

    #[test]
    fn scrollback_offset_change_redisplays_placement() {
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

        bytes.clear();
        let mut scrolled = test_placement(0, 0);
        scrolled.scrollback_offset = 3;
        encode_graphics_update(
            &mut bytes,
            &[scrolled],
            false,
            &mut images,
            &mut placements,
            &mut sources,
        );
        let redisplay = String::from_utf8_lossy(&bytes);
        assert!(!redisplay.contains("a=t"));
        assert!(redisplay.contains("a=p"));
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
    fn view_change_deletes_stale_placement_immediately() {
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
        bytes.clear();
        encode_graphics_update(
            &mut bytes,
            &[],
            true,
            &mut images,
            &mut placements,
            &mut sources,
        );

        let delete = String::from_utf8_lossy(&bytes);
        assert!(delete.contains("a=d,d=i"));
        assert!(placements.is_empty());
    }
}
