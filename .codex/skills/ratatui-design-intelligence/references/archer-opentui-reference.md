# R073 Archer: OpenTUI Application Intelligence

## Classification

- Source: https://github.com/Inakitajes/archer
- Commit: `fb97e50e1314d928adf64319b7ac098c1f2f6d67`
- Graph project: `home-ayaz-.cartography-refpool-archer`
- Form: complete terminal orchestration application
- Stack: Bun, TypeScript, OpenTUI and OpenCode SDK
- Direct Ratatui compatibility: none
- License: not detected; behavior-only reimplementation

Archer belongs in the Ratatui design-intelligence pool because it is a strong
full-application and scenario reference. It does not belong in the direct
component dependency bucket.

The complete verified system, component and data-flow maps are:

- `/home/ayaz/.cartography/pools/ratatui/discoveries/archer/ARCHER-SYSTEM-MAP.md`
- `/home/ayaz/.cartography/pools/ratatui/discoveries/archer/component-catalog.json`
- `/home/ayaz/.cartography/pools/ratatui/discoveries/archer/data-flow.json`
- `/home/ayaz/.cartography/pools/ratatui/classifications/R073.json`

## Why Herdr should mine it

- IDE-like border-only shell rather than painted dashboard cards;
- rounded Unicode panels with terminal-background-derived border elevation;
- pipeline navigator plus detail, todos and tabbed session/reports/logs;
- same dashboard works for live, attached and historical runs;
- semantic runtime-to-view `ProgressUI` boundary;
- bounded and coalesced agent activity/transcript storage;
- queued permission and review overlays;
- launcher, history browser and configuration-editor page templates;
- keyboard/mouse parity and render-derived hit regions;
- responsive information-priority degradation.

## Screen templates

| Template | Structure | Herdr candidate |
|---|---|---|
| Launcher | pipeline list + preview/prompt/options + footer + validation modal | workspace/task/agent launch flow |
| Live dashboard | pipeline + detail/todos + session/reports/logs + status footer | agent execution workspace |
| Run browser | history list + metadata/phase detail + summary modal | session history and handoff browser |
| Config editor | global/project tabs + editable tree + contextual detail + modal family | settings/keybind/component registry pages |

## Data boundary to preserve

```text
runtime protocol -> semantic client event -> bounded page state
                 -> pure view projection -> Ratatui drawing
```

Do not move Archer's mutable OpenTUI renderables into Herdr. Convert its
behavior to Herdr's existing state/update/`compute_view()`/pure-render model.
Runtime facts remain server/API facts; focus, selection, tabs, scroll, overlays
and hit regions remain TUI presentation state.

## Visual tokens to extract

- canvas background: transparent;
- modal overlay: exact terminal background;
- border levels: dim and normal derived from terminal background;
- accent border: current focus owner;
- success/error/warning colors: status semantics;
- dim/faint: secondary metadata and inactive rails;
- selected chip: colored background with readable foreground;
- rounded border glyph profile plus an explicit Herdr ASCII fallback.

## Motion contract

Archer's only meaningful animation is a braille spinner selected from wall time.
The dashboard also repaints periodically for elapsed time. Progress bars and the
live cursor are static. Herdr should use demand-driven idle ticks and retain a
reduced-motion/no-animation profile.

## Do not attribute to Archer

- no general tiling, docking or split resizing;
- no Rust/Ratatui component pack;
- no source-copy permission;
- no explicit ASCII, color-depth or reduced-motion fallback;
- no exhaustive full-screen snapshot coverage.

Use R033 `ratatui-hypertile` for actual split-tree tiling. Use R023 and the
visual-language guide for Herdr theme tokens. Archer supplies the page shell,
workflow surfaces and semantic data projection that sit inside those systems.

## Verification

- Codebase Memory full index: 1,339 nodes / 4,325 edges;
- `bun test`: 269 passed / 0 failed / 873 assertions;
- `bun run typecheck`: passed;
- compiled Bun binary: passed.
