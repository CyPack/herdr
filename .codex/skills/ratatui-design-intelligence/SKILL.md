---
name: ratatui-design-intelligence
description: Use when designing, reviewing, or implementing Herdr TUI pages, responsive layouts, tiling or split panes, rounded borders and pill controls, themes, popups, modals, component registries, loading states, or animation; also use when a repository, demo, owner profile, article, or example application must be indexed, mapped, classified, extracted, and published into the Ratatui reference corpus. Routes work through the evidence-backed pool and preserves Herdr's pure-render and client/runtime boundaries.
---

# Ratatui Design Intelligence

Use this skill before changing Herdr TUI structure or visual behavior. It is a
router and adaptation guide, not permission to copy third-party code.

## Version 2.1.0 stack adaptation

For every cross-stack source, produce `stack-adaptation-map.json` and classify
each mapping as `direct_api`, `structural_adapter`,
`behavior_reimplementation`, or `reject`. Bind source semantics, data flow,
behavior trigger, target ownership, semantic/failure/performance/ownership
diffs, license boundary, acceptance tests, cross tests, and verification
evidence before P14 can pass.

P14 hands validated reference-adapter input to the sibling module
`herdr-change-pipeline`. This handoff does not grant product authority and does
not authorize product code changes or stable runtime operations.

## Required workflow

When the user supplies a new reference-project URL for corpus intake, read
`references/canonical-reference-pipeline.md`, instantiate
`assets/reference-project-run-template.json`, and execute the canonical phase
graph in `assets/reference-project-pipeline-v2.json`. P0–P7 publish source
intelligence; then read `references/herdr-integration-analysis-pipeline.md` and
execute P8–P14 for behavior/data/layout/component/test integration intelligence.
Do not substitute the
short design-query workflow below for a full corpus run.

1. Classify the request into one or more surfaces:
   - tiling, split, docking, resize, responsive layout;
   - rounded borders, buttons, theme tokens, terminal fallback;
   - popup, modal, drawer, toast, command palette, multi-page navigation;
   - component lifecycle, registry, authoring, loading state, animation.
2. Read the matching reference file below. Read `references/herdr-adaptation-guide.md`
   for every implementation task.
3. Query the code graph before filesystem search:
   - `search_graph` for candidate symbols;
   - `trace_path` for ownership and calls;
   - `get_code_snippet` for exact source;
   - fall back to repository files only when graph results are insufficient.
4. Return candidates in four separate buckets:
   - direct Ratatui component/API;
   - adaptable architecture or algorithm;
   - full application/scenario reference;
   - negative/caution reference.
5. Bind every recommendation to source ID, exact file/symbol, license/reuse mode,
   and Herdr adaptation cost. Never infer capability from a repository name.
6. Before production code, identify the protected behavior and write a failing
   test. Geometry belongs in `compute_view()` or another pure computation path;
   `render()` only draws from `&AppState`.

## Reference router

- Tiling, resize, docking, responsive pages:
  `references/tiling-and-layout.md`
- Rounded borders, pill controls, colors and fallbacks:
  `references/visual-language.md`
- Popups, modals, overlays, command palettes and pages:
  `references/popups-and-pages.md`
- Component APIs, registries, loading and animation:
  `references/component-architecture.md`
- Herdr ownership, purity, testing and rollout constraints:
  `references/herdr-adaptation-guide.md`
- Verified sources and research backlog:
  `references/source-catalog.md`
- Deterministic pilot query examples and no-match boundaries:
  `references/pilot-query-results.md`
- New repository/example intake, indexing, mapping, classification and corpus
  publication: `references/canonical-reference-pipeline.md`
- Herdr layer audit, behavior gap, datum provenance/authority, exact cell-level
  layout fidelity, component binding, implementation slicing and cross tests:
  `references/herdr-integration-analysis-pipeline.md`
- Refreshable current Herdr runtime/API/client/state/view/render/input/test
  baseline for P8: `references/herdr-layered-architecture-baseline.md`

## Hard constraints

- Rounded means terminal-cell illusion: Unicode corner glyphs, padding, color,
  focus and disabled states. Do not promise pixel radius.
- A responsive Miller layout is not a general tiling manager. Do not conflate
  Herdr's 1/2/3-column file manager with free split/dock/resize behavior.
- Keep shared runtime/session facts in server/API paths. Page, panel, modal,
  selection, focus, hover and theme state are TUI-client presentation state.
- Never mutate state, perform I/O, sleep, or advance animation inside `render()`.
- Overlays clear their target region before drawing; input ownership is a stack,
  so the top modal consumes relevant input before background surfaces.
- Truecolor is optional. Define ANSI-256, ANSI-16 and no-color fallbacks.
- Treat `tui-studio` as an authoring/export reference, not a Rust component
  runtime. Treat FrankenTUI as reference-only unless a separate compatibility
  review proves otherwise.
- Review licenses before copying code. Prefer reimplementation from behavior and
  tests when the source is reference-only or its license is incompatible.

## Output contract

For a reference-project corpus run, publish the artifacts and terminal status
required by `reference-project-pipeline-v2.json`. A source-only result closes at
P7 but is not implementation-ready. A master run is complete only after P14;
MCP absence blocks graph-dependent phases, unresolved claims make the run
partial, and `passed` requires source and integration `V=0`, zero undeclared
cell-level fidelity differences, complete traceability and product isolation.

For a design query, provide:

1. recommended source-backed pattern;
2. alternative and why it ranks lower;
3. Herdr state/geometry/input ownership;
4. width/height and terminal-capability failure behavior;
5. tests required before implementation;
6. exact evidence links or graph symbols.
