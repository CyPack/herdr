# Tiling and Layout Guide

## Capability boundary

Herdr already has a responsive Miller file-manager layout with one, two or
three columns. That is a content-specific responsive layout. A general tiling
workspace additionally needs a persistent split tree, focus geometry, resize
operations, pane hit-testing, swap/dock semantics and serialization decisions.

## Primary extraction: R033 ratatui-hypertile

Use the indexed project
`home-ayaz-.cartography-refpool-ratatui-hypertile`.

| Need | Evidence | Adaptation |
|---|---|---|
| Split tree | `src/core/state/mutation.rs`, `HypertileState.split_with_ratio` | Model leaf/split nodes independently from pane runtime. |
| Cached geometry | `src/core/state/mod.rs`, `HypertileState.compute_layout` | Cache only derived layout; invalidate on tree/ratio/area changes. |
| Resize | `src/core/state/mutation.rs`, `HypertileState.resize_focused` | Normalize ratios and no-op on unchanged values. |
| Directional focus | `src/core/state/focus.rs`, `HypertileState.focus_direction` | Choose targets from computed rectangles, not tree order. |
| Mouse resize/swap | `extras/src/runtime/mouse.rs` tests | Separate hit-test, drag state, preview, commit and cancel. |
| Visual focus | `extras/src/runtime/types.rs`, `BorderConfig` | Theme normal and focused border set/style separately. |
| Palette overlay | `extras/src/runtime/render.rs`, `render_palette` | Reuse centered bounded popup semantics, not its hard-coded colors. |

Hypertile's high-value invariant is `Node::Split { direction, ratio, first,
second }` plus a leaf pane. The mutation replaces a focused leaf with a split,
rebuilds the index, moves focus to the new pane and invalidates layout cache.

## Herdr target model

Keep these concepts separate:

```text
PaneTree          persistent presentation structure
PaneId            stable client identity
ComputedPaneRect  derived geometry for current terminal area
FocusPath         selected leaf and directional navigation origin
ResizeGesture     transient mouse/keyboard interaction state
PaneContent       existing Herdr client surface descriptor
PaneRuntime       runtime/session facts owned outside layout presentation
```

Do not embed PTYs, terminal parsers or server sessions in the split tree. A leaf
references a surface identity; it does not own the runtime.

## Responsive strategy

Use ordered layout modes instead of a single nest of percentage constraints:

| Mode | Example trigger | Behavior |
|---|---|---|
| compact | width below feature minimum | One primary surface; secondary surfaces become tabs/popups. |
| standard | enough for two useful panes | Primary + contextual side surface. |
| wide | enough for three useful panes | Persistent explorer/editor/inspector or Miller columns. |
| workspace | explicit user split tree | Honor tree while enforcing every leaf minimum. |

Triggers must be based on content minimums, not device names. Test exact
boundary widths, one cell below, and one cell above. Use saturating geometry.

Ratatui core R014 supplies `Constraint::{Length,Min,Max,Percentage,Ratio,Fill}`,
`Flex`, `Layout::spacing`, `Rect::centered`, and nested horizontal/vertical
areas. It supplies allocation primitives, not docking state.

TUI Studio R061 is useful for authoring concepts: its layout engine supports
responsive root fill, absolute, grid and flexbox calculation. Its Ratatui
exporter currently reduces non-absolute container layout to horizontal or
vertical `Layout` constraints, so it is a generator reference rather than a
complete runtime layout solution.

## Advanced research backlog

- `ratatui-interact`: resizable `SplitPane` with mouse/focus support.
- `rat-widget`: split, tabbed and multi-page structural widgets.
- `FrankenTUI`: reference-only pane tree, drag/dock preview, atomic operations,
  interaction timeline and undo/redo. Do not assume Ratatui API compatibility.

## Required tests

- split/close/swap preserve unique leaf identities;
- ratio normalization and minimum sizes never create invalid rectangles;
- resize at terminal edges and zero/tiny areas is safe, including a tiny terminal;
- directional focus has deterministic ties and safe no-target behavior;
- drag cancel restores the pre-gesture tree;
- computed geometry changes with area while persistent tree stays unchanged;
- render receives precomputed rectangles and performs no mutation.
