# Files architecture decision — surface model, flexibility, and what DnD costs

> Status: measured evaluation, 2026-08-22 (S36). The user's question, verbatim
> intent: is the Files module's architecture *enough* — should it become a
> terminal app like `spf`/`yazi`, live inside a tab/pane, resize flexibly
> between panes, and carry drag-and-drop — and at what performance/traffic
> price? This document answers with measurements from this codebase and the
> resource doctrine (`resource-doctrine.md`, RD0-RD9).

## What Files is today (measured)

- **One resident, stage-level surface.** `StageSurfaceView::NativeFiles` takes
  the stage; `SideBySideRight::Files` (TP-SBS-FILES-01) projects the same
  resident instance into the right half beside the terminal. One `FmState` per
  app, backgrounded rather than destroyed; `resident_files_generation` guards
  stale projections.
- **Pure-projection render.** `compute_view` produces rects
  (locations/miller/trail/rows/header) into whatever viewport owns Files
  (`files_viewport` = right half when beside); `render_file_manager(app,
  frame, area)` is already **area-parametric** — it draws into any rect it is
  handed. This is the single most important measured fact: the renderer does
  not care where Files lives.
- **Typed input capture.** Mouse routing goes through one gate
  (`handle_file_manager_mouse_at`), one Miller-resize transaction, full-row
  hit rects (`TrailRowView.rect` covers the whole row; `name_rect` its left
  part). The SBS center-gate follows `right_surface.area` since `10d9743f`.

## The four options

### (a) Stage surface + beside mode — today
- **Pros:** zero duplication (one resident instance, one input authority, one
  projection seam); restore/session already handled; RD-clean (nothing runs
  while hidden — projections are per-frame, state is inert). The beside mode
  already gives "files next to my terminal".
- **Cons:** exactly one Files at a time; it is not a peer of panes (cannot be
  tiled arbitrarily, swapped into layouts, or opened twice); keyboard focus
  hand-off to the right half is still owed (L4).

### (b) Files as a pane surface (in-tab/pane instances)
- **Pros:** full layout flexibility — tile, resize with the existing pane
  divider mechanics, one per pane, participates in every future layout
  feature for free; answers "should it live inside a tab/pane" with the
  app's own composition model.
- **Cons (measured, not guessed):** `FmState` today is a singleton on
  `AppState` (`file_manager: Option<FmState>`) with singleton companions
  (locations model, clipboard, operation worker authority, preview workers).
  Making it per-pane multiplies every one of those seams and every
  generation-guard; the operation worker's "one operation in flight per
  session" contract needs a redesign. Input routing gains a per-pane Files
  gate. Estimated blast radius: state + actions + input + compute + restore —
  a refactor-risk class change (two-plus core surfaces, persisted state).
- **RD note:** cost while idle stays ~0 either way (state is data), but
  N resident previews (image/PDF workers) can each hold memory; per-pane
  instances need an explicit cap or lazy-drop.

### (c) Embed an external TUI FM (`spf`, `yazi`) in a pane
- **Pros:** zero herdr code for the FM itself; herdr already hosts any TUI in
  a pane today — this works *now* with `yazi` in a split; mature feature sets.
- **Cons:** loses every integration seam we built and the user asked for:
  Add-Reference-to-Agent, click-to-open unification, Taildrop header seat,
  agent-directory gestures, PWA projection (an external FM renders raw cells,
  not structured rows — the web client cannot reproject it), theme/config
  coherence. Traffic: raw cell diffs are fine locally but *denser* than
  structured row updates for remote clients. Verdict: fine as a *user choice*
  in a pane (already possible), wrong as *the* architecture.

### (d) Hybrid — keep the resident stage/beside surface, add pane-hosted
### *views* of the same resident state later
- One `FmState`, N viewports: the area-parametric renderer makes multiple
  simultaneous projections geometrically trivial; the real work is input
  focus (which viewport owns the keys) and per-viewport cursors — a smaller,
  staged step toward (b) without multiplying workers or state.

## Decision

**Stay on (a), evolve toward (d).** The renderer is already viewport-neutral;
the singleton state is the flexibility bottleneck, and (d) relaxes it without
the (b) blast radius. Concrete next steps, in order:
1. Keyboard focus hand-off to the beside surface (L4 — already queued).
2. Draggable SBS divider (`ratio_percent` exists 20-80; reuse the Miller
   divider's typed capture — one transaction owner, no new pattern).
3. Only then evaluate per-pane *views* (d) with a measured cap on preview
   workers.

`spf`/`yazi` remain available today as ordinary pane programs for anyone who
prefers them; no architecture change needed for that.

## Drag and drop (the user's priority)

Terminal DnD is **in-app only** (no OS drag into/out of a TTY), and the
mechanics already exist: `MouseEventKind::Drag` drives the Miller and SBS
dividers through a typed capture with one owner. A row drag is the same
pattern with a different payload.

- **Sidebar agent reordering** (the concrete ask: "move an agent up from the
  bottom-left"): Down on a row arms a *candidate* drag; crossing a row
  boundary converts it to a drag transaction (so plain clicks stay clicks);
  render shows an insertion seam (a `─` line) rather than a floating ghost;
  Up commits the reorder. RD cost: zero while idle, redraws only on
  drag-motion events — no animation, no timer. Persisted order needs a home
  (session state or config overlay) — that is the one real design decision.
- **Files row drag (move/copy into a directory):** same capture, drop target
  = directory row under the pointer; commits through the existing operation
  worker (`Move`/`Copy`), so progress/cancel/undo semantics come free.
- **Cons / risks:** drag-vs-click disambiguation (threshold: one row of
  vertical travel); scroll-during-drag (reuse hover-column wheel);
  multi-client (a drag is client-local input state — fine, it lives in the
  TUI layer per the runtime/client guardrail).
- **Verdict: feasible, cheap at idle, pattern-consistent.** Sequence it
  after L4 focus work; sidebar reorder first (smaller, the user's named
  pain), Files DnD second.

## Web/port references (measured 2026-08-22)

- **ratcn** (kristoferlund/ratcn): shadcn-style copy-paste components for
  ratatui; the site itself runs the *same component code* in the browser via
  WASM. Preview-release, API explicitly unstable. Value to us: interaction
  patterns (tooltips!) to port into our own widgets — a tooltip in ratatui is
  a deferred overlay draw on hover cell-hit, which fits our compute/render
  split; **check the repo license before copying code**, and prefer
  design-reference over dependency (API instability + our fork discipline).
  License verified 2026-08-22: **MIT** — copying patterns into this AGPL fork
  is clean. Components today: Button, List, Select, Tabs, Dialog, Toaster,
  BarChart, **Tooltip**, ScrollArea; text input/area still missing.
- **gronke/ui-components** (MIT): one Rust component definition renders as a
  Lit web component *and* a ratatui widget (Taffy: CSS flexbox → cells).
  Early (unpublished to npm), but the architecture is the interesting part
  for the PWA: **component-level dual-target** vs our current model
  (server-rendered cells, bytes ∝ changed cells). For the remote/Mac traffic
  goal, our server-render already wins on wire bytes; a WASM client-render
  (ratzilla road) moves CPU to the client and drops server traffic to
  state-sync only — worth a measured comparison when the browser-plugin wave
  (PRD §O) starts, not before.

## Test points for whatever lands first (rule: named before code)

| point | expected | why |
|---|---|---|
| SBS divider drag | Down on divider arms capture, Drag moves `ratio_percent` within 20-80, Up commits; click elsewhere untouched | one-owner capture is the pattern's contract |
| agent-row drag reorder | Down+cross-row = drag, insertion seam drawn, Up persists order, plain click still selects | drag must not eat clicks |
| reorder persistence | order survives restart via its chosen home | an order that resets teaches the feature is fake |
| beside keyboard focus (L4) | a chosen gesture moves key focus to Files-beside and back | the known limit this doc inherits |
