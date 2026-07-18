# Herdr Miller Trail Plain-Wheel Scroll Forward Fix

## Problem

The isolated debug server receives the user's live Miller gesture as
`ScrollUp`/`ScrollDown` with no modifier. The current horizontal adapter only
accepts native `ScrollLeft`/`ScrollRight` or Shift+wheel, so a wheel event over
the empty body of a Trail column is consumed without changing either the
horizontal viewport or a row selection.

Fresh evidence from the test-owned server log:

- 318 parsed vertical wheel events;
- zero native horizontal wheel events;
- zero Shift-modified vertical wheel events;
- the fractional reducer/render families remain green (6/6).

The bug is therefore the live input mapping, not fractional geometry.

## Dependency Chain

```text
terminal SGR ScrollUp/ScrollDown
  -> raw input parser
  -> App::handle_file_manager_mouse
  -> immutable Trail row/column hit geometry
  -> App::handle_miller_horizontal_scroll
  -> TrailViewSnapshot::horizontal_scroll_target
  -> MillerHorizontalViewport::offset_cells
  -> next pure Trail projection/render
```

## Behavior Contract

1. Plain wheel over a live visible Trail row keeps the existing vertical
   one-row selection behavior.
2. Plain wheel over the empty body of a live Trail column falls back to
   horizontal movement when horizontal overflow exists:
   `ScrollUp -> left`, `ScrollDown -> right`.
3. The fallback uses the existing direction-aware
   `ceil(reference_column_width / 3)` step from `TrailViewSnapshot`.
4. Native horizontal wheel and Shift+wheel remain supported everywhere in the
   Files Stage.
5. Detail panel, Files header, foreign/outside surface, stale generation or
   revision, degenerate geometry, and one-column/no-overflow cases remain
   inert or retain their existing owner.
6. Render and filesystem behavior do not change; no visual baseline update is
   expected.

## Test Points

| ID | What is tested | Expected result | Why |
|---|---|---|---|
| TP-TRAIL-WHEEL-FALLBACK-01 | Deep mixed-width Trail; modifierless wheel over empty live column body | One event moves by the snapshot's one-third step and survives the next frame | Reproduces the exact live terminal event that currently no-ops |
| TP-TRAIL-WHEEL-FALLBACK-02 | Modifierless wheel over a visible live row | Only vertical selection changes; horizontal offset does not | Preserves mouse-first row navigation |
| TP-TRAIL-WHEEL-FALLBACK-03 | Detail/header/outside/stale/one-column inputs | No unauthorized horizontal mutation | Prevents the fallback from broadening input authority |
| TP-TRAIL-WHEEL-FALLBACK-04 | Native horizontal and Shift+wheel regression families | Existing direction, clamp, fractional step, and auto-follow behavior stays green | Protects the already published T7.7/T7.8 contract |
| TP-TRAIL-WHEEL-FALLBACK-05 | Full Rust, Linux/Windows Clippy, Chromium visual suite, maintenance gates | All applicable gates pass with no baseline mutation | Input-only fixes must not regress rendering or platform builds |

## Atomic Delivery

1. `test: reproduce miller plain-wheel scroll regression`
2. `fix: add miller plain-wheel scroll fallback`
3. `docs: record miller plain-wheel scroll closure`

Every production edit follows a compile-valid failing test. Only targeted files
are staged; the user-owned `.superpowers/` directory is never touched.
