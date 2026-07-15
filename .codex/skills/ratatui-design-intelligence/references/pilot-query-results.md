# Pilot Query Results

These are deterministic rankings over the eight verified pilot sources. They
are not yet the production concept router.

## Query: desktop IDE shell

1. R033 — adaptable architecture: split tree, focus geometry, resize, mouse
   swap, plugin registry and command palette.
2. R061 — authoring/application reference: component tree, canvas, toolbars,
   property editors, palettes and modal family.
3. R014 — direct primitives: nested `Layout`, `Constraint`, `Flex`, `Block`,
   tabs, tables and scrollable widgets.

R033 is first because it supplies executable workspace behavior. R061 supplies
surface inventory but runs in React/DOM. R014 supplies primitives but no IDE
state model.

## Query: responsive tiling

1. R033 `HypertileState.split_with_ratio`, `compute_layout`,
   `resize_focused`, `focus_direction` — persistent tree and interaction.
2. R061 `LayoutEngine.calculateLayout` — explicit responsive root fill;
   `calculateGridLayout` and `calculateFlexboxLayout` — authoring algorithms.
3. R014 `Layout.flex`, `Constraint`, `Flex` example — direct responsive
   allocation primitives.

No verified pilot source proves a full free-docking or magnetic-snap manager.
FrankenTUI claims those capabilities but remains research-backlog/reference-only
until local code indexing and compatibility review.

## Query: rounded colorful popup

1. R014 `BorderType::Rounded` + `Clear` — direct canonical API.
2. R033 `BorderConfig` + `render_palette` — focused color state and bounded
   centered palette behavior.
3. R023 `ThemePalette` — semantic color seed.
4. R061 `ratatuiBorderType` — independent proof that CSS rounded preview
   becomes Unicode `BorderType::Rounded` in terminal export.

No verified pilot source provides a complete truecolor/256/16/no-color policy.
`termprofile` is the next candidate, not a verified answer yet.

## Query: animation without render-time mutation or I/O

1. R022 — direct match for stateless loading animation derived from external
   `elapsed_ms`; application owns tick cadence.
2. R059 — algorithm match for tween/spring interpolation, but lifecycle caution:
   move mutable animation updates out of draw/render for Herdr.

There is no verified pilot match for complex shader-like transitions that also
already satisfies Herdr's pure-render contract. `tachyonfx` requires a separate
buffer-adapter and purity/performance review.

## Query: component authoring registry

1. R061 — strongest authoring surface and exporter reference.
2. R033 — runtime plugin registry and per-pane plugin replacement pattern.
3. R022 — consistent builder contract across a concrete component pack.

None of these is a drop-in Herdr component runtime. A Herdr registry must retain
the existing state/action/input architecture and pure render boundary.
