# Herdr Adaptation Guide

## Ownership decision

Before any code, classify each fact:

| Fact | Owner |
|---|---|
| pane/agent process and terminal state | server/runtime/API |
| workspace/tab/pane organization currently shared | shared session model, avoid expanding identity coupling |
| page route, modal stack, focus, hover, selection | TUI client |
| split-tree geometry and resize gesture | TUI client unless a future protocol explicitly shares layouts |
| theme tokens and terminal capability profile | TUI client |
| loading/animation clock | TUI client update loop |

Never add a private TUI socket field merely because a page needs it.

## Pure render adaptation

Third-party references often mutate selection, hit regions or animation while
drawing. Translate them as follows:

```text
reference render mutation -> Herdr update or compute_view
reference runtime object   -> separate pure state and runtime handle
reference direct I/O       -> action/effect returned to owner loop
reference raw RGB          -> semantic theme token
reference pixel radius     -> terminal border/padding recipe
```

`compute_view()` may calculate rectangles and store the derived view according
to Herdr's existing contract. `render()` takes `&AppState` and only draws.

## Delivery sequence

1. Write a behavior note with exact source IDs/symbols and non-goals.
2. Choose the smallest Herdr surface and identify ownership.
3. Add characterization tests for existing behavior.
4. Write a failing test for the new state/geometry/input behavior.
5. Implement update/geometry before drawing polish.
6. Add theme and capability variants.
7. Add mouse hit regions from computed geometry.
8. Run narrow tests, then `just check` before commit.
9. Keep research/docs commits separate from product behavior commits.

## Suggested phased roadmap

### L1 — Design tokens and rounded primitives

- semantic palette plus truecolor/256/16/no-color mapping;
- normal/focused/disabled/danger border tokens;
- rounded block, compact/pill action, title and separator recipes;
- buffer tests for Unicode and ASCII fallback.

### L2 — Overlay stack and popup family

- stack ownership and focus restoration;
- `Clear`-first modal surface;
- confirm/input/help/command palette/context menu/toast variants;
- tiny-area, overflow and background-input tests.

### L3 — Page shell and responsive modes

- stable `PageId` and client route state;
- compact/standard/wide degradation contract per page;
- sidebar/tab/drawer conversion rules;
- boundary geometry tests.

### L4 — General pane workspace

- pure split tree and operations;
- cached computed geometry;
- directional focus, keyboard resize, mouse drag preview/commit;
- close/swap/zoom/preset operations and persistence decision.

### L5 — Motion and loading polish

- external animation clock and reduced-motion policy;
- stateless skeleton placeholders;
- narrowly scoped transitions/effects with CPU/tick budgets.

## Stop conditions

Stop and request an architectural decision when:

- a visual feature requires new shared runtime identity or protocol state;
- adopting a framework would replace Herdr's state/action/input architecture;
- a dependency adds overlapping event loops or terminal ownership;
- source license/reuse terms are unclear;
- the design has no safe tiny-terminal or no-color behavior.
