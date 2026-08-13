# Bar icons — putting a picture in a herdr bar

Audience: agents and people adding chrome to `[shell.bars]`. Companion to
`docs/patterns/custom-layout.md`; behaviours are registered in
`behaviors/surface-chrome.md` under `TP-ART-01..05`.

---

## BI1 · An icon is cells, and that is the load-bearing decision

herdr's server sends the client a **cell diff**: `BlitEncoder`
(`src/protocol/render_ansi.rs`) skips every cell that did not change. A bar is
drawn in every frame and its icon changes almost never, so an icon made of cells
costs a few hundred bytes once and **nothing** thereafter.

The image path is a different pipe and it is measurably more expensive:

| | cells | Kitty graphics |
|---|---|---|
| travels in | the cell diff | `FrameData.graphics`, *after* the text frame |
| unchanged frame costs | **0 bytes** | re-emitted / needs a retained-graphics ledger |
| frame ceiling | 2 MB | **32 MB** (`MAX_GRAPHICS_FRAME_SIZE`) |
| enabled by default | yes | no — `config.experimental.kitty_graphics` |
| terminal support asked | n/a | **not probed at all** in this build |

So: **draw bar chrome with cells.** The image path is deferred, not refused; it
needs a capability probe and region tracking first. Do not reach for it to put a
16×16 PNG in a status bar.

## BI2 · Two pixels per cell, upper is the foreground

`▀` (U+2580) paints the top half in the foreground and lets the background show
below, so one cell carries two vertically stacked pixels each with its own
24-bit colour. A cell whose **top** pixel is transparent uses `▄` instead, so
transparency never needs an invented background colour.

Consequences you must design around:

- A picture `W` pixels wide by `H` pixels tall occupies `W` cells by `⌈H/2⌉` rows.
- A bar with `size = 5, border = true` gives **3** content rows → up to 6 pixel rows.
- A bar with `size = 3, border = true` gives **1** row → 2 pixel rows. Plan the
  mark for the bar, not the other way round.

Rejected alternatives, and why: **braille** is 2×4 but single-colour, so a logo
loses its colours; **quadrants** are 2×2 with only two colours per cell, so
diagonals break up; **sextants** need a font this project cannot assume.

## BI3 · Writing one

```toml
[[shell.bars.top.sections]]
kind   = "fixed"
cells  = 10
widget = { kind = "icon", art = "herd" }                 # bundled
action = { kind = "popup", argv = ["btop"] }             # an icon is also a button

[[shell.bars.top.sections]]
kind   = "fixed"
cells  = 8
widget = { kind = "icon", palette = { r = "red", g = "green" }, pixels = [
  "..rrrr..",
  ".rrrrrr.",
  "..gggg..",
] }

[[shell.bars.top.sections]]
kind   = "fixed"
cells  = 3
widget = { kind = "icon", glyph = "*" }                  # the font already has it
```

Rules the checker enforces, each with its own message:

- **Exactly one** of `glyph`, `art`, `pixels`. Two would need a precedence rule
  the config file cannot show.
- Palette keys are **one character**; longer keys make a pixel row readable two
  ways.
- `.` and a space are **transparent**. Any other character must be in the palette
  — an unnamed key is refused rather than silently treated as transparent,
  because on screen the two are identical.
- Rows must all be the **same width**.
- A `fixed` section narrower than the picture is **refused**, naming both
  numbers. A clipped picture is the wrong picture.
- A `fill` section is **not** refused — its width is not known until the terminal
  has a size, and a config that loads on one screen and not another is worse than
  a mark clipped by a narrow window. It clips, exactly as a label does.

Colours use the same grammar as `shell.bars.<edge>.color`: a palette token
(`mauve`, `accent`, `teal`, …) or a literal `#rrggbb`. They resolve **at draw
time**, so switching theme recolours a picture without re-deriving geometry.

## BI4 · Verify it the way the tests do

```bash
# the product's own answer, before you look at a screen
herdr config check
```

A picture that reaches the buffer is not the same as one a person can see. This
fork shipped a bar label whose foreground was the colour of the surface under it:
every state test passed, the glyph dump passed, and the bar looked empty. When
checking chrome, read **colour**, not just symbols
(`behaviors/surface-chrome.md` TP-CHROME-55).

## BI5 · Anti-patterns

| Don't | Do |
|---|---|
| Reach for Kitty graphics for a status-bar icon | Cells; BI1 |
| Design the mark first, fit the bar after | Bar height decides pixel rows; BI2 |
| Assume an odd pixel-row count fills its lower half | It stays transparent, by design |
| Use a multi-character palette key | One character, or the row is ambiguous |
| Silently clip a picture that does not fit a declared width | Refuse where it is written |
| Bake `Color` at config time | Keep specs; resolve against the live palette |
| Prove an icon works with a symbol dump | Read the cell colours too |

## BI6 · Why this format is ready for third-party icons

The user's stated goal is an "app store"-like surface where TUI apps ship their
own marks. This format is deliberately shaped for that:

- **Text** — diffable, reviewable, signable in git.
- **Inert** — a palette and a grid; nothing executable. Accepting a third-party
  icon is not accepting code, which keeps it below the bar that
  `skill-mcp-vetting` sets for anything that runs.
- **Bounded** — the cell footprint is known before it is drawn, so a bar budget
  can be computed rather than discovered.

When an API endpoint arrives, the thing it transports is this document. The
renderer does not change.
