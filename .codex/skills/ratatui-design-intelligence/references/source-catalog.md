# Source Catalog and Research Backlog

## Verified priority sources

| ID | Source | Role | Default reuse mode |
|---|---|---|---|
| R014 | https://github.com/ratatui/ratatui | Core layout, widget, border and popup primitives | direct API/reference |
| R022 | https://github.com/jharsono/tui-skeleton | Stateless animated loading component pack | dependency or adaptation after review |
| R023 | https://github.com/ricardodantas/ratatui-themes | Semantic palette seed and theme picker | token-pattern adaptation |
| R033 | https://github.com/nikolic-milos/ratatui-hypertile | Split tree, resize, focus, mouse and palette workspace | architecture/algorithm adaptation |
| R059 | https://github.com/vyfor/animate | Tween/spring interpolation and frame lifecycle | adaptation; keep mutation outside render |
| R061 | https://github.com/jalonsogo/tui-studio | Component authoring, responsive canvas and Ratatui export | authoring/reference only |
| R073 | https://github.com/Inakitajes/archer | OpenTUI IDE-like orchestration shell, data-flow and page templates | behavior/architecture adaptation only; no source copying |

Machine-readable claims live under
`/home/ayaz/.cartography/pools/ratatui/classifications/` and are bound to
immutable commit SHAs.

R073 is intentionally in this pool even though it uses OpenTUI rather than
Ratatui. It is a verified full-application/scenario source: launcher, live and
historical agent dashboard, run browser, config editor, modal family, semantic
runtime-to-view adapter and terminal-native rounded visual language. See
`archer-opentui-reference.md` for the extraction boundary.

## Newly discovered P0 candidates

These are researched but not yet cloned/indexed/classified, so do not present
README capabilities as code-verified Herdr facts.

| Candidate | Why it matters | Initial status |
|---|---|---|
| https://github.com/yexiyue/ratatui-kit | components, identity, hooks, routing, modal input layers, theme override, responsive tables | inspect/index next |
| https://github.com/Brainwires/ratatui-interact | buttons, forms, context menu, dialog, toast, tabs, resizable split pane, focus/mouse | inspect/index next |
| https://github.com/thscharler/rat-salsa | mature rat-widget/dialog/menu/popup/focus ecosystem | inspect/index next |
| https://github.com/ratatui/tachyonfx | composable cell-level effects, parallel/sequence transitions | inspect after animation clock design |
| https://github.com/aschey/termprofile | truecolor/256/16/no-color/no-TTY detection and Ratatui conversion | inspect before fallback implementation |
| https://github.com/ratatui/awesome-ratatui | maintained discovery catalog for frameworks/widgets/apps | reference collection |
| https://github.com/Dicklesworthstone/frankentui | advanced docking, drag, split, undo/replay and layout lab | reference-only; not assumed Ratatui compatible |
| https://github.com/joshka/tui-popup | centered, sized and draggable popup history | moved; inspect maintained `tui-widgets` destination |

Also prioritize already supplied pool sources `opaline`, `tui-tabs`,
`ratatui-elm`, `bevy_ratatui`, `schemaui`, `ratty`, `ATAC`, `rmpc` and complex
IDE/dashboard applications after the P0 component/framework pass.

## Official primary references

- https://ratatui.rs/concepts/layout/
- https://ratatui.rs/examples/layout/flex/
- https://ratatui.rs/recipes/render/overwrite-regions/
- https://docs.rs/ratatui/latest/ratatui/widgets/enum.BorderType.html
- https://docs.rs/ratatui/latest/ratatui/widgets/struct.Clear.html
