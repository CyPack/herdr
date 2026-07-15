# Component Architecture and Lifecycle

## Three different “component” meanings

Do not collapse these categories:

1. Ratatui widget: renders into a `Rect`/`Buffer`, optionally with external
   state through `StatefulWidget`.
2. Herdr UI component: owns presentation state, computes geometry/hit regions,
   handles actions and delegates pure drawing.
3. Authoring/registry tool: catalogs components, props, examples and exports,
   but may not run inside Herdr.

R014 is category 1. Herdr needs category 2. R061 TUI Studio and `tui-pantry`
primarily inspire category 3.

## Herdr component contract

Prefer a small explicit contract over a framework-wide rewrite:

```text
ComponentState      pure client presentation data
ComponentAction     semantic input/update request
ComponentView       derived rectangles, visible rows, hit regions, styles
update(action)      mutates state outside render
compute_view(area)  pure geometry and projection
render(view, state) pure buffer drawing
```

Add a registry only when multiple pages need runtime selection or extension.
Registry metadata should include stable ID, supported actions, minimum geometry,
state schema/version, capability requirements and render entry point.

## Source roles

### R061 TUI Studio

Useful concepts:

- component definition/catalog and drag/drop authoring;
- tree, toolbar, canvas, property/style/layout editors;
- absolute/grid/flex layout calculations;
- responsive canvas mode and resize handles;
- exporter boundary, including Ratatui `Block`, `BorderType`, `Layout`, buttons,
  lists, tabs, tables and progress bars.

Limitations:

- React/DOM runtime is not Herdr's component runtime;
- CSS radius becomes `BorderType::Rounded` in export;
- exporter reduces complex layout semantics and needs generated-code review;
- use as authoring and inventory reference, not copy-in architecture.

### R022 tui-skeleton

This is a component pack, not an application skeleton. It exposes ten loading
widgets: block, table, list, text, streaming text, vertical/horizontal bar
charts, braille bar, key-value table and line chart. All derive animation from
external elapsed time and share builder-style base/highlight/mode/block props.

Herdr use cases: plugin discovery, remote data, indexing, file preview and agent
history loading. Loading placeholder shape should match the final component
geometry to avoid layout jumps.

### R059 animate

Provides tween/spring state, `once/cycle/alternate`, easing and Ratatui type
interpolators. The global `tick(delta)` advances atomic frame time. For Herdr,
wrap animation clocks in application update state rather than calling generated
mutable animation methods from render.

### Newly discovered frameworks

`ratatui-kit` offers component identity, reconciliation, hooks, routing, input
layers, themes and async state. It is a strong architecture comparison, but
adopting it could conflict with Herdr's existing architecture and should be
treated as a focused pattern mine unless a separate migration ADR is approved.

### R073 Archer

Archer is category 3 at the source-code level and a strong category-2 behavior
reference. Its `ProgressUI` interface separates semantic orchestration events
from the OpenTUI surface. Its four application screens share panel, focus,
overlay, navigation and theme patterns. Preserve that semantic boundary, but
replace mutable OpenTUI renderables with Herdr's pure view projection and render
contract. Archer has no detected license, so use behavioral reimplementation
only.

`ratatui-interact` is a promising direct component source for focus/mouse-aware
buttons, inputs, context menus, dialogs, toasts, tabs and split panes. Index and
review exact code and license before dependency or adaptation decisions.

## Component quality checklist

- stable semantic action API;
- state testable without PTY or terminal;
- pure geometry with zero/tiny-area behavior;
- keyboard and mouse parity;
- focus, hover, pressed, disabled, loading, empty and error states;
- theme and terminal capability fallback;
- bounded lists/scroll and Unicode-width correctness;
- no render-time I/O, sleeps or state mutation;
- snapshot/buffer tests plus state-transition tests;
- documented reuse mode and source/license provenance.
