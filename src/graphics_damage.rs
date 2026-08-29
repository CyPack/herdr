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
    let frame_len = (width as usize) * (height as usize) * 4;
    if previous.len() != frame_len || next.len() != frame_len || frame_len == 0 {
        // A resize (or a caller bug) is never patchable.
        return DamageOutcome::FullFrame;
    }
    if previous == next {
        return DamageOutcome::Unchanged;
    }

    let tiles_x = width.div_ceil(DAMAGE_TILE_PX) as usize;
    let tiles_y = height.div_ceil(DAMAGE_TILE_PX) as usize;
    let mut dirty = vec![false; tiles_x * tiles_y];
    let mut dirty_count = 0usize;
    let row_bytes = (width as usize) * 4;

    // Row-run comparison: within one pixel row a tile is DAMAGE_TILE_PX * 4
    // contiguous bytes, so each tile-row comparison is one slice equality —
    // memcmp, not a per-pixel loop — on a path that runs per streamed frame.
    for (tile_y, dirty_row) in dirty.chunks_mut(tiles_x).enumerate() {
        let y_start = tile_y * DAMAGE_TILE_PX as usize;
        let y_end = (y_start + DAMAGE_TILE_PX as usize).min(height as usize);
        for y in y_start..y_end {
            let row_start = y * row_bytes;
            let prev_row = &previous[row_start..row_start + row_bytes];
            let next_row = &next[row_start..row_start + row_bytes];
            if prev_row == next_row {
                continue;
            }
            for (tile_x, tile_dirty) in dirty_row.iter_mut().enumerate() {
                if *tile_dirty {
                    continue;
                }
                let x_start = tile_x * DAMAGE_TILE_PX as usize * 4;
                let x_end = (x_start + DAMAGE_TILE_PX as usize * 4).min(row_bytes);
                if prev_row[x_start..x_end] != next_row[x_start..x_end] {
                    *tile_dirty = true;
                    dirty_count += 1;
                }
            }
        }
    }

    if dirty_count == 0 {
        // Byte-identical rows can still differ overall only if some row
        // differed, so this is unreachable in practice — but a differ that
        // could answer "changed" with zero rects would strand the caller.
        return DamageOutcome::Unchanged;
    }
    let share = dirty_count as f32 / (tiles_x * tiles_y) as f32;
    if share > MAX_DAMAGE_SHARE {
        return DamageOutcome::FullFrame;
    }

    // Merge horizontal runs of dirty tiles per tile-row (xpra merge_rects
    // pattern, one axis: enough to keep a scroll from becoming a storm).
    let mut rects = Vec::new();
    for tile_y in 0..tiles_y {
        let mut tile_x = 0;
        while tile_x < tiles_x {
            if !dirty[tile_y * tiles_x + tile_x] {
                tile_x += 1;
                continue;
            }
            let run_start = tile_x;
            while tile_x < tiles_x && dirty[tile_y * tiles_x + tile_x] {
                tile_x += 1;
            }
            if rects.len() == MAX_DAMAGE_RECTS {
                // Past the cap the escape overhead beats the savings.
                return DamageOutcome::FullFrame;
            }
            let x = (run_start as u32) * DAMAGE_TILE_PX;
            let y = (tile_y as u32) * DAMAGE_TILE_PX;
            rects.push(DamageRect {
                x,
                y,
                width: (((tile_x - run_start) as u32) * DAMAGE_TILE_PX).min(width - x),
                height: DAMAGE_TILE_PX.min(height - y),
            });
        }
    }
    DamageOutcome::Patches(rects)
}

/// Raw RGBA bytes one `a=f` patch escape may carry.
///
/// tuios measured (kitty 0.48.2) that `a=f` does not combine with `m=`
/// continuation chunks: a patch must fit ONE escape. 3072 raw bytes matches
/// the proven `KITTY_CHUNK_BYTES` and encodes to a 4096-byte base64 payload.
/// 3072 / 4 = 768 pixels per escape, so a taller rect is emitted as
/// row bands. No `o=z` here: at three kilobytes the deflate header and the
/// per-frame CPU buy nothing measurable.
pub(crate) const PATCH_MAX_RAW_BYTES: usize = 3072;

/// One ready-to-write kitty escape patching a region of image `image_id`.
///
/// Emitted against the STABLE streaming image identity (the trunk chain
/// that keeps one host image per streaming source) — `a=f,...,X=1` replaces
/// the pixels of frame 1 in place, which is exactly xpra's patch shape and
/// the protocol's own SSH-efficiency mechanism.
pub(crate) fn emit_patch_escapes(
    frame: &[u8],
    frame_width: u32,
    image_id: u32,
    rects: &[DamageRect],
) -> Vec<Vec<u8>> {
    use base64::Engine as _;
    use std::io::Write as _;

    /// Widest column band one escape can carry as a single row.
    const MAX_BAND_PX: u32 = (PATCH_MAX_RAW_BYTES / 4) as u32;

    let mut escapes = Vec::new();
    for rect in rects {
        if rect.width == 0 || rect.height == 0 {
            continue;
        }
        // A merged run can be wider than one escape's row (a 1920 px scroll
        // band is 7680 raw bytes): split columns first, then rows, so every
        // emitted band obeys the single-escape limit in both axes.
        let mut band_x = 0u32;
        while band_x < rect.width {
            let band_w = MAX_BAND_PX.min(rect.width - band_x);
            let row_bytes = (band_w * 4) as usize;
            let rows_per_escape = ((PATCH_MAX_RAW_BYTES / row_bytes).max(1)) as u32;
            let mut band_y = 0u32;
            while band_y < rect.height {
                let band_h = rows_per_escape.min(rect.height - band_y);
                let mut raw = Vec::with_capacity(row_bytes * band_h as usize);
                for row in 0..band_h {
                    let y = rect.y + band_y + row;
                    let start = ((y * frame_width + rect.x + band_x) * 4) as usize;
                    raw.extend_from_slice(&frame[start..start + row_bytes]);
                }
                let payload = base64::engine::general_purpose::STANDARD.encode(&raw);
                let mut escape = Vec::with_capacity(payload.len() + 64);
                // Writing to a Vec cannot fail; the let binding keeps the
                // no-unwrap rule without hiding a real error path.
                let _ = write!(
                    escape,
                    "\x1b_Ga=f,i={image_id},x={x},y={y},s={s},v={v},X=1,f=32;",
                    x = rect.x + band_x,
                    y = rect.y + band_y,
                    s = band_w,
                    v = band_h,
                );
                escape.extend_from_slice(payload.as_bytes());
                escape.extend_from_slice(b"\x1b\\");
                escapes.push(escape);
                band_y += band_h;
            }
            band_x += band_w;
        }
    }
    escapes
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

    fn parse_escape(esc: &[u8]) -> (std::collections::HashMap<String, String>, Vec<u8>) {
        let text = std::str::from_utf8(esc).expect("ascii escape");
        let body = text
            .strip_prefix("\x1b_G")
            .and_then(|t| t.strip_suffix("\x1b\\"))
            .expect("kitty escape frame");
        let (control, payload) = body.split_once(';').expect("control;payload");
        let keys = control
            .split(',')
            .map(|kv| {
                let (k, v) = kv.split_once('=').expect("k=v");
                (k.to_owned(), v.to_owned())
            })
            .collect();
        let mut decoded = Vec::new();
        let mut buffer = 0u32;
        let mut bits = 0u32;
        for byte in payload.bytes() {
            let value = match byte {
                b'A'..=b'Z' => byte - b'A',
                b'a'..=b'z' => byte - b'a' + 26,
                b'0'..=b'9' => byte - b'0' + 52,
                b'+' => 62,
                b'/' => 63,
                b'=' => continue,
                other => panic!("base64 dışı bayt {other}"),
            } as u32;
            buffer = (buffer << 6) | value;
            bits += 6;
            if bits >= 8 {
                bits -= 8;
                decoded.push((buffer >> bits) as u8);
            }
        }
        (keys, decoded)
    }

    /// One small rect becomes exactly one escape whose keys and payload
    /// reproduce the sub-image — the delta's reason to exist.
    #[test]
    fn a_small_rect_becomes_one_exact_patch_escape() {
        let width = 64u32;
        let mut frame = frame(width, 16, 0);
        // 8x2 rect at (4,3) with recognisable bytes.
        for y in 3..5u32 {
            for x in 4..12u32 {
                let i = ((y * width + x) * 4) as usize;
                frame[i..i + 4].copy_from_slice(&[x as u8, y as u8, 0xAB, 0xFF]);
            }
        }
        let rect = DamageRect {
            x: 4,
            y: 3,
            width: 8,
            height: 2,
        };
        let escapes = emit_patch_escapes(&frame, width, 42, &[rect]);
        assert_eq!(escapes.len(), 1);
        let (keys, payload) = parse_escape(&escapes[0]);
        assert_eq!(keys["a"], "f");
        assert_eq!(keys["i"], "42");
        assert_eq!(keys["x"], "4");
        assert_eq!(keys["y"], "3");
        assert_eq!(keys["s"], "8");
        assert_eq!(keys["v"], "2");
        assert_eq!(keys["X"], "1");
        assert_eq!(keys["f"], "32");
        assert_eq!(payload.len(), 8 * 2 * 4);
        assert_eq!(&payload[..4], &[4, 3, 0xAB, 0xFF]);
    }

    /// The tuios-measured constraint: `a=f` cannot use `m=` continuation,
    /// so no escape may carry more than PATCH_MAX_RAW_BYTES — a tall rect
    /// splits into row bands that reassemble exactly.
    #[test]
    fn a_patch_never_spans_the_escape_limit() {
        let width = 64u32;
        let mut frame = frame(width, 64, 0);
        for (index, byte) in frame.iter_mut().enumerate() {
            *byte = (index % 251) as u8;
        }
        let rect = DamageRect {
            x: 0,
            y: 0,
            width: 32,
            height: 32,
        };
        let escapes = emit_patch_escapes(&frame, width, 7, &[rect]);
        assert!(escapes.len() > 1, "4096 ham bayt tek escape'e sığamaz");
        let mut reassembled = vec![0u8; (32 * 32 * 4) as usize];
        let mut covered_rows = 0u32;
        for esc in &escapes {
            let (keys, payload) = parse_escape(esc);
            assert!(
                payload.len() <= PATCH_MAX_RAW_BYTES,
                "escape başına ham sınır"
            );
            assert!(keys.get("m").is_none(), "a=f ile m= birlikte YASAK");
            let (bx, by) = (
                keys["x"].parse::<u32>().unwrap(),
                keys["y"].parse::<u32>().unwrap(),
            );
            let (bw, bh) = (
                keys["s"].parse::<u32>().unwrap(),
                keys["v"].parse::<u32>().unwrap(),
            );
            assert_eq!(bx, 0);
            assert_eq!(bw, 32);
            for row in 0..bh {
                let src = (row * bw * 4) as usize;
                let dst = (((by + row) * 32) * 4) as usize;
                reassembled[dst..dst + (bw * 4) as usize]
                    .copy_from_slice(&payload[src..src + (bw * 4) as usize]);
            }
            covered_rows += bh;
        }
        assert_eq!(covered_rows, 32, "bantlar tüm satırları örter");
        for y in 0..32u32 {
            for x in 0..32u32 {
                let src = ((y * width + x) * 4) as usize;
                let dst = ((y * 32 + x) * 4) as usize;
                assert_eq!(
                    &reassembled[dst..dst + 4],
                    &frame[src..src + 4],
                    "({x},{y})"
                );
            }
        }
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
