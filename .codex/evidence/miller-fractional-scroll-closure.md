# Miller Fractional Scroll Closure — 2026-07-18

## Outcome

TRAIL-T7.8 is closed. Miller Trail horizontal scrolling is deterministic,
cell-based, and width-aware: one wheel event advances by
`ceil(reference_column_width / 3)`. The viewport can expose clipped leading
and trailing columns without mutating Trail selection or filesystem state.

## Atomic chain

- Design: `2e8f2409` (`docs: design fractional miller trail scrolling`)
- Plan: `4d725565` (`docs: plan fractional miller trail scrolling`)
- RED: `4e6e922b` (`test: require fractional miller trail scrolling`)
- GREEN: `febe65ef` (`fix: add fractional miller trail scrolling`)
- Visual: `97d5fe01` (`test: approve fractional miller scroll visual`)
- Full-suite fixture alignment: `26da2437`
  (`test: align row action fixture with clipped trail width`)

The code/test chain was fast-forward published to CyPack only. Local HEAD,
`origin/feat/native-fm`, and `origin/master` were all verified at
`26da243712f5f79c3eda49cd07a88892f775246d`. No upstream operation occurred.

## Protected behavior

- `MillerHorizontalViewport` owns only absolute `offset_cells` plus
  `follow_active`; mutable `first_visible` is no longer state authority.
- Pure geometry clamps the requested offset, intersects logical
  column/divider intervals with the viewport, and emits only nonempty rects.
- Mixed widths 18/30/48 produce 6/10/16-cell directional steps.
- Render and input consume the same generation/revision-bound
  `TrailViewSnapshot`.
- Partially clipped actions are omitted instead of becoming smaller phantom
  targets.
- Entry labels are sliced by display cells; wide glyph continuations are
  never split into invalid output.
- Manual resize clamps the offset without rearming follow; Trail navigation
  rearms active-column auto-follow.
- The 10,000-action deterministic invariant suite remains green.

## Visual evidence

- VIS-12 exports a real 60x20 Ratatui buffer with a 20-cell leading suffix and
  the beginning of a clipped 48-cell trailing column.
- A temporary single-cell `r` to `Z` mutation made the scoped Chromium test
  fail by exactly 15 pixels.
- Regeneration restored the ignored fixture byte-for-byte at SHA-256
  `d047d5c64e5a53548f00b8d1c3548867d4edba6c2db113f06f7201963ebde604`.
- VIS-10 and VIS-11 intentional auto-follow/clipping changes were inspected
  and updated only through `trail.spec.ts`; no global snapshot update ran.

## Fresh gates

- `cargo fmt --check`: pass
- `cargo nextest run --locked --no-fail-fast`: 3,512/3,512 pass, 2 skip
- Linux Clippy `--all-targets --locked -- -D warnings`: pass
- Windows MSVC Clippy with `LIBGHOSTTY_VT_SIMD=false`: pass
- Python maintenance: 68/68 pass
- Agent-state Bun: 5/5 pass
- Plugin marketplace Bun: 12/12 pass
- Playwright Chromium: 20/20 pass
- `git diff --check`: pass
- Added production `unwrap()`: zero
- Mutable `horizontal.first_visible` production reads: zero
- Isolated test root after cleanup: absent

The single command `.local/herdr-trail-test.sh verify` starts with semantic
cleanup, runs the full gate sequence, and finishes with semantic cleanup. It
only owns `/tmp/herdr-trail-manual-test`; stable Herdr, its socket, and user
processes remain outside its authority.

## Codebase graph

The required single-worker CLI refresh completed without restarting any proxy:
21,296 nodes / 98,085 edges. MCP status is `ready` and fresh snippets resolve:

- `src/fm/miller.rs::MillerHorizontalViewport` with `offset_cells`
- `src/ui/file_manager/miller.rs::miller_viewport_geometry_at_offset`
- `src/app/file_manager_miller.rs::handle_miller_horizontal_scroll`, consuming
  `file_manager_trail` and writing the bounded offset
