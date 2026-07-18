# Miller Plain-Wheel Scroll Forward-Fix Evidence

Date: 2026-07-18

## Root Cause

The test-owned debug server received the user's live Miller gesture, but the
event kind did not match the product's horizontal adapter:

- parsed `ScrollUp`/`ScrollDown` with `KeyModifiers(0x0)`: 318;
- parsed native `ScrollLeft`/`ScrollRight`: 0;
- parsed Shift-modified vertical wheel: 0.

The published horizontal path accepted only native horizontal wheel and
Shift+wheel. Plain vertical wheel over a visible row correctly owned vertical
selection, but the same event over empty Trail column body had no row owner and
was consumed without changing horizontal viewport state.

The existing fractional reducer/render path was healthy before the fix:

- `fractional_scroll`: 2/2;
- `horizontal_wheel`: 3/3;
- `shift_wheel_scrolls_deep_trail_left`: 1/1.

## TDD Chain

- Plan: `a63e39e7` — `docs: plan miller plain-wheel scroll fallback`
- RED: `1ca992c6` — `test: reproduce miller plain-wheel scroll regression`
  - expected offset: `173 -> 163`;
  - actual offset before fix: `173`;
  - Nextest: 0/1, exact assertion failure.
- GREEN: `051f2829` — `fix: add miller plain-wheel scroll fallback`
  - modifierless wheel over empty live Trail column body normalizes to the
    existing horizontal reducer;
  - visible row wheel keeps vertical selection ownership;
  - detail/header/outside/stale geometry does not receive fallback authority.

## Fresh Verification

- Exact GREEN: 1/1.
- Plain-wheel family: 4/4.
- Native horizontal-wheel family: 3/3.
- Fractional-scroll family: 2/2.
- Shift-wheel persistence: 1/1.
- Full Nextest: exit 0; current inventory 3,513 tests.
- Linux Clippy all-targets with `-D warnings`: clean.
- Windows MSVC Clippy with `LIBGHOSTTY_VT_SIMD=false` and `-D warnings`: clean.
- Python maintenance: 68/68.
- Integration assets Bun: 5/5.
- Plugin marketplace Bun: 12/12.
- Playwright Chromium: 20/20; no baseline mutation.
- `cargo fmt --check`: clean.
- `git diff --check`: clean.

The single-worker graph refresh completed without extraction errors:
21,304 nodes / 98,123 edges. The long-lived MCP channel returned the new
`plain_wheel_over_empty_trail_body_uses_fractional_horizontal_fallback` test
and the updated production `App::handle_file_manager_mouse` body.

## Runtime Safety

The stable Herdr processes/socket were not modified. The already-running
test-owned debug binary also remained untouched; the user must exit that test
window normally and rerun `.local/herdr-trail-test.sh run` to load the new
binary for headful acceptance.
