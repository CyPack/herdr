//! Frame-to-frame damage detection for streaming pane graphics.
//!
//! A streaming pane (browser, video) re-transmits its whole RGBA frame today;
//! at the measured 26.9 Mbit/s uplink that demands 305–543 Mbit/s. Between
//! consecutive frames most pixels are identical, and the kitty protocol
//! itself carries partial-frame edits for exactly this reason — its own
//! docs name SSH as the motivation (`graphics-protocol.rst:875`, `a=f` with
//! `x,y,s,v`). This module is the pure half: given two frames, name the
//! rectangles that changed. Emission (escape building, the 3072-byte
//! single-escape limit, capability probing) lives with the encoder.
//!
//! The tiling-and-cap shape is pattern-transferred from tuios (MIT,
//! `internal/app/kitty_damage.go`) — the one implementation in the surveyed
//! 90-repo ecosystem that solved this same pty-saturation symptom — and
//! re-derived for RGBA pixel buffers with our own constants.
//! PRD: `.local/prd/graphics-damage-delta-prd.md` · TP-GFX-DELTA-01.

/// Tile edge in pixels. Small enough that a cursor blink dirties one tile,
/// large enough that the dirty-tile bitmap for a 4K frame stays a few KB.
pub(crate) const DAMAGE_TILE_PX: u32 = 32;

/// More distinct rectangles than this and the escape overhead outweighs the
/// savings: fall back to a full frame (tuios: maxDamageRects=256).
pub(crate) const MAX_DAMAGE_RECTS: usize = 256;

/// Past this fraction of dirty tiles a full frame is cheaper than patching
/// (tuios: maxDamageShare=0.9).
pub(crate) const MAX_DAMAGE_SHARE: f32 = 0.9;

/// One changed region, in pixels, within the frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DamageRect {
    pub(crate) x: u32,
    pub(crate) y: u32,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

/// What the differ decided about a frame pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DamageOutcome {
    /// Nothing changed: emit zero bytes (the still pane costs nothing).
    Unchanged,
    /// The changed regions, row-merged; strictly fewer pixels than the frame.
    Patches(Vec<DamageRect>),
    /// Patching would cost more than a full frame: send the frame.
    FullFrame,
}

/// Diffs two same-geometry RGBA frames into damage rectangles.
///
/// Frames of different sizes (a resize) are never patchable — the caller
/// must send a full frame and reset its previous-frame buffer.
pub(crate) fn diff_rgba_frames(
    previous: &[u8],
    next: &[u8],
    width: u32,
    height: u32,
) -> DamageOutcome {
    let _ = (previous, next, width, height);
    DamageOutcome::FullFrame
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(width: u32, height: u32, fill: u8) -> Vec<u8> {
        vec![fill; (width * height * 4) as usize]
    }

    fn dirty_pixel(data: &mut [u8], width: u32, x: u32, y: u32) {
        let index = ((y * width + x) * 4) as usize;
        data[index] = data[index].wrapping_add(1);
    }

    /// T5's essence: a still pane must cost zero bytes.
    #[test]
    fn an_unchanged_frame_emits_zero_bytes() {
        let previous = frame(256, 128, 7);
        let next = previous.clone();
        assert_eq!(
            diff_rgba_frames(&previous, &next, 256, 128),
            DamageOutcome::Unchanged
        );
    }

    /// The reason deltas exist: a lone change becomes one small rect, not a
    /// whole frame.
    #[test]
    fn a_single_dirty_pixel_yields_one_tile_sized_patch() {
        let previous = frame(256, 128, 7);
        let mut next = previous.clone();
        dirty_pixel(&mut next, 256, 40, 50);
        let DamageOutcome::Patches(rects) = diff_rgba_frames(&previous, &next, 256, 128) else {
            panic!("expected patches");
        };
        assert_eq!(rects.len(), 1);
        let rect = rects[0];
        assert!(rect.width <= DAMAGE_TILE_PX && rect.height <= DAMAGE_TILE_PX);
        // The dirty pixel lies inside the reported rect.
        assert!(rect.x <= 40 && 40 < rect.x + rect.width);
        assert!(rect.y <= 50 && 50 < rect.y + rect.height);
    }

    /// Adjacent dirty tiles on one row merge into a single rect — 256
    /// separate escapes for one horizontal scroll would drown the win in
    /// escape overhead (xpra's merge_rects lesson).
    #[test]
    fn adjacent_dirty_tiles_merge_into_one_rect() {
        let previous = frame(256, 128, 7);
        let mut next = previous.clone();
        for x in [10u32, 50, 90] {
            dirty_pixel(&mut next, 256, x, 10);
        }
        let DamageOutcome::Patches(rects) = diff_rgba_frames(&previous, &next, 256, 128) else {
            panic!("expected patches");
        };
        assert_eq!(rects.len(), 1, "one row of adjacent tiles must merge");
        assert!(rects[0].width >= 96);
    }

    /// tuios' maxDamageShare: past ~90% dirty a full frame is cheaper.
    #[test]
    fn a_mostly_changed_frame_falls_back_to_full() {
        let previous = frame(256, 128, 7);
        let next = frame(256, 128, 8);
        assert_eq!(
            diff_rgba_frames(&previous, &next, 256, 128),
            DamageOutcome::FullFrame
        );
    }

    /// tuios' maxDamageRects: scattered noise past the cap degrades to a
    /// full frame rather than an escape storm.
    #[test]
    fn scattered_noise_past_the_rect_cap_falls_back_to_full() {
        let width = 2048u32;
        let height = 1024u32;
        let previous = frame(width, height, 7);
        let mut next = previous.clone();
        // One dirty pixel per tile, on every other tile in both axes:
        // far more distinct, non-adjacent rects than MAX_DAMAGE_RECTS.
        let mut count = 0usize;
        let mut ty = 0;
        while ty * DAMAGE_TILE_PX < height {
            let mut tx = 0;
            while tx * DAMAGE_TILE_PX < width {
                if (tx + ty) % 2 == 0 {
                    dirty_pixel(&mut next, width, tx * DAMAGE_TILE_PX, ty * DAMAGE_TILE_PX);
                    count += 1;
                }
                tx += 2;
            }
            ty += 2;
        }
        assert!(count > MAX_DAMAGE_RECTS, "test setup must exceed the cap");
        assert_eq!(
            diff_rgba_frames(&previous, &next, width, height),
            DamageOutcome::FullFrame
        );
    }

    /// A geometry change can never patch: the caller sends a full frame.
    #[test]
    fn mismatched_geometry_is_a_full_frame() {
        let previous = frame(64, 64, 7);
        let next = frame(128, 64, 7);
        assert_eq!(
            diff_rgba_frames(&previous, &next, 128, 64),
            DamageOutcome::FullFrame
        );
    }
}
