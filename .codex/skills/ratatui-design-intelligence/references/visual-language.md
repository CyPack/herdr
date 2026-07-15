# Visual Language: Rounded, Colorful, State-Aware

## What “rounded” means in a TUI

Ratatui R014 proves `BorderType::Rounded` renders `╭─╮`, `│ │`, `╰─╯`.
This is a cell glyph set, not pixel geometry. Strong rounded/pill illusion comes
from four ingredients:

1. rounded border symbols;
2. horizontal padding around a short label;
3. foreground/background contrast;
4. state-specific border, fill and text styles.

For buttons, reserve a minimum height of three rows when a full border is
required. A one-row compact button should use bracket/pill glyph conventions
and cannot display four rounded corners honestly.

## Semantic token baseline

R023 `ThemePalette` provides a useful minimum vocabulary:

```text
accent, secondary, bg, fg, muted, selection,
error, warning, success, info
```

Herdr should extend that into component-state tokens rather than scattering raw
RGB values:

```text
surface.base / surface.raised / surface.overlay
border.normal / border.focused / border.disabled / border.danger
text.primary / text.muted / text.disabled / text.inverse
action.normal / action.hover / action.focused / action.pressed / action.disabled
status.info / status.success / status.warning / status.error
pane.active / pane.inactive / pane.drop_target / pane.resize_handle
```

R023 includes 15 palettes and an `is_light()` brightness heuristic. It does not
provide border-symbol policy, focus/disabled variants, persistence or terminal
capability fallback, so it is a token seed rather than the complete system.

R033 separates normal and focused border sets/styles. Combine that pattern with
semantic tokens so pane focus does not depend on ad-hoc RGB literals.

## Capability ladder

Design every token for these profiles:

| Profile | Strategy |
|---|---|
| truecolor | RGB theme values and subtle gradients where readable. |
| ANSI-256 | map each semantic token to a stable indexed color. |
| ANSI-16 | use high-contrast named colors plus bold/reverse. |
| no-color | preserve meaning through glyph, label, border weight and modifier. |
| ASCII-safe | replace rounded/dashed/special glyph sets with `+`, `-`, `|`. |

`termprofile` is the strongest newly discovered candidate for detecting
truecolor, ANSI-256, ANSI-16, no-color and no-TTY and converting Ratatui styles.
Do not add it as a dependency before checking whether Herdr already has enough
terminal capability data.

## Component recipes

### Rounded pane

- normal: rounded border + muted border token;
- focused: same geometry, accent border + bold title;
- disabled: ASCII/plain fallback allowed, muted text, no hover response;
- resize hover: render a one-cell highlighted boundary without changing tree;
- drop target: temporary raised surface + accent/dashed border.

### Pill button

- label is padded symmetrically;
- default/focused/pressed/disabled styles are explicit;
- focused state must remain visible without color;
- hit region includes the visible padded surface;
- label truncates safely and never indexes past a narrow area.

### Colorful tiling

Use color for ownership and state, not decoration alone. Keep content
backgrounds restrained; reserve saturated tokens for active borders, titles,
drop targets, errors and selection. Test adjacent pane contrast in all profiles.

## Animation cautions

R059 `animate` provides tween/spring interpolation for Ratatui types, but its
README suggests mutating animation at the top of a widget draw method. That
conflicts with Herdr's pure-render contract. Advance time and animation state in
the event/update path; render only the resulting snapshot.

R022 `tui-skeleton` is safer for loading placeholders because widgets are
stateless and derive appearance from an externally supplied `elapsed_ms`.
Visible loading surfaces may request a faster tick; idle surfaces must return to
a lower rate.

`tachyonfx` is a high-value future effects source, but it transforms the frame
buffer after widgets render. Any adoption needs a narrow adapter and explicit
purity/performance characterization.
# R073 Archer terminal-native elevation

Archer verifies a useful alternative to painted dashboard cards: leave normal
panel backgrounds transparent, derive two border elevations from the terminal's
reported background, and use the exact terminal background only to mask the
screen beneath a blocking overlay. Focus changes the border token rather than
painting another rectangle. Port this as a semantic-token strategy, then add
Herdr's required truecolor/256/16/no-color and ASCII profiles; Archer does not
provide those fallbacks.
