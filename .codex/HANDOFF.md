# SESSION HANDOFF — Herdr Shell Foundation / Native FM — 2026-07-16

> AUTHORITATIVE CURRENT CHECKPOINT. This section supersedes every lower
> historical "next action" statement. The prior handoff is preserved after the
> current checkpoint as an evidence appendix; do not resume from its stale
> SF3/SF4-start pointer.

## 0. EXECUTIVE STATE

- Repository: `/home/ayaz/projects/herdr`
- Branch: `feat/native-fm`
- Acting identity: CyPack external contributor; `origin` is the writable
  `CyPack/herdr` fork and `upstream` is read-only.
- Current verified product head: `d1002cac` (`feat: commit miller
  column widths through clamped seam`, FM2.1 model core CLOSED). Prior:
  FM1.3 geometry core `3e8a50d0`, FM1.2 `97710337`, FM1.1 `5e8616e0`. Shell
  Foundation fully closed in scoped form at `887471c2` (SF6 evidence
  records the deferred OPEN items).
  SF4 fully closed at `f973740e` (4.1 8/8, 4.2 8/8, 4.3 6/6); the SF4
  registry entries in `.codex/TASKS.md` were reconciled at `5349bb85`,
  and the custom-layout architecture guide lives locally at
  `docs/superpowers/specs/2026-07-17-herdr-custom-layout-architecture-guide.md`
  (gitignored program artifact, like the plans).
- SF5.1: `AppDockModel`/`app_dock_entry_areas`/`render_app_dock` in
  `src/ui/app_dock.rs` — icon-only Terminal+Files from the typed stage
  authority, bounded 3..=9 template track, pure render, BaseLayer wiring
  that stays a no-op until a dock-bearing template is live. Dock table
  7/7; full suite 3,322/3,322.
- SF4.2 closed 8/8 at `20f659c1`; SF4.3 is now the ACTIVE microphase with
  slice 01 GREEN (surface-exclusive hit geometry: pane/split projection
  and `rt.resize` side effects gated behind
  `stage.surface_view() == TerminalWorkspace` on desktop AND mobile;
  `surface_view()`'s dead-code allow removed with its named consumption
  condition satisfied). Evidence:
  `.codex/evidence/shell-foundation-sf4-surface-projection-progress.md`.
- Prior closed heads this session: SF4.2-05 scoped core `8b1882eb`/`5eb63763`,
  SF4.2-04 characterization `119e4a2d`, SF4.2-03 `bb6f8970`/`efe6446b`,
  SF4.2-02 `41362e89`/`017ba97f`, SF4.2-01 `92777e23`/`f4f5e3cb`, SF4.1-08
  `784fdc2e`/`944a9d4c`, and test-stability `3c853a70`.
- Program: 7 Shell Foundation phases SF0-SF6 plus 5 FM phases FM1-FM5.
- Closed phases: SF0, SF1, SF2, SF3, and microphase SF4.1 (8/8 slices GREEN).
- Active phase: SF4 SurfaceHost and input router.
- Microphase SF4.2 is CLOSED with 8/8 slices GREEN; the closure gate ran
  the full direct `just check` equivalent (Rust 3,309/3,309 + B0 skip, Bun
  5/5 + 12/12, Python 64/64, both Clippy targets, fmt/diff/unwrap).
  Historical slice detail: slices 01 (frozen seven-tier router), 02 (overlay
  mouse blocking), 03 (overlay keyboard ownership over captures, shared
  exhaustive `blocking_overlay_active()` classifier), 04 (capture-ownership
  characterization), and 05 CLOSED (focus restore: `overlay_return_mode` +
  `enter_overlay_mode` + validity-filtered `leave_modal`; SF4.2-05b sweep
  wired EVERY production overlay entry — 24 call sites — with a structural
  zero-remaining-direct-assignment proof and a ContextMenu-from-Copy row
  keeping a live copy session through an overlay episode) are GREEN. Slice
  06 (inert regions) is GREEN as a characterization (`3580ff19`): valid RED
  was refuted with source evidence (`flatten_region_hits` empty-rect filter,
  `hit_at` generation+containment, `on_sidebar_divider` collapse guard,
  degenerate toggle rects), so the test freezes Hidden/zero-area inertness,
  compact-rail interactivity, and the previously unpinned adversarial
  collapsed-divider guard. Slice 07 (`bb3ac54d`/`c6b024ce`) wires the
  topmost-hit tier: `shell_mouse_input_owner(position)` resolves
  `ShellView::region_hit_at` against the EXACT current generation, so old
  coordinates re-resolve to their current owner and never grant vanished
  authority; the caller still consumes only the overlay comparison, so
  dispatch stays bit-identical until SF4.3/SF6 consume semantic targets.
- Slice 08 (`20f659c1`, characterization): the hidden-terminal seal was
  proven already closed by recon plus an 8-kind event matrix through the
  full production `App::handle_mouse` with a control phase proving the
  same press reaches the live terminal once Files closes.
- SF5.2: shared `activate_dock_app` authority (Files singleton open /
  Terminal stage restore), fail-closed `handle_app_dock_mouse` over live
  dock terrain, `ContextMenuKind::AppDock` popover through
  `enter_overlay_mode` (SF4.2 blocking/restore free), SF3 region-generic
  `ResizeTransaction` pinned for dock 3..=9 bounds.
- SF6.1: Files now owns the COMPLETE WorkspaceStage while active — no
  tab-bar carve-out (terminal-app chrome), `terminal_area == stage`, the
  SF1 curtain characterization was replaced by
  `files_renders_as_native_workspace_stage_surface`, and exactly two
  old-arithmetic fixtures were migrated to the frozen stage geometry.
- SF6.2: Files keyboard/mouse now route from the TYPED
  `StageSurfaceView::NativeFiles` authority; 37 direct test fixtures
  migrated onto the open transaction; the plan's composite regression
  command ran 214/214.
- Immediate next microtask: FM1.3 — horizontal viewport geometry REDs
  (Stage widths 0/15/16/31/32/56/84/140/400, <=5 complete disjoint
  columns + dividers, focused visible) + scroll/render catalog, then FM2
  drag-resize (the custom-layout target). See
  `.codex/evidence/fm1-miller-viewport-progress.md`.
- Product tree: clean at `d1002cac`; only the user-owned untracked
  `.superpowers/` tree exists and must remain untouched/unstaged.
- Full exact-head gate: 3,344/3,344 Rust tests (`--no-fail-fast`), one
  named B0 real-host probe skipped; Linux all-target and Windows MSVC bin
  Clippy; fmt/diff/added-production-unwrap checks passed (Bun/Python last
  green at the SF6 gate `887471c2`).
- Both CyPack refs (`feat/native-fm`, fork `master`) equal exact SHA
  `d1002cacb8f2b6eb730d0a9ab6217cff9ac7f6a9` at this checkpoint;
  `upstream` untouched.
- Fresh sequential Codebase Memory store refreshed post-publication with
  current `blocking_overlay_active`, `shell_mouse_input_owner`,
  `route_shell_input`, and `miller_layout` source.
- The built-in MCP channel now serves the fresh store; this was verified with
  exact symbol and snippet, never `ready` alone. A fresh session must repeat
  that proof before trusting the transport.
- Stable Herdr process/socket, installed binary, user terminal/editor/browser,
  and every user process were untouched.

## 1. NORTH-STAR MISSION

Herdr is not merely a terminal multiplexer. The product mission is to provide
a desktop/workspace-class UX from a terminal while retaining server/runtime
truth, SSH efficiency, deterministic Ratatui rendering, and native or
terminal-hosted app surfaces.

The approved architecture direction is:

```text
Thin bounded Shell Foundation
  -> typed Workspace Stage and fail-closed input ownership
  -> icon-only AppDock
  -> Files as the first native Stage surface
  -> Finder-like bounded Miller UX continuation
  -> later, separately authorized Apps/Desktop expansion
```

The outer shell has named semantic regions. Their contents may use bounded
split containers, fixed/resizable/fill tracks, collapse/expand, scroll
viewports, focus scopes, overlay ownership, and reusable page templates.
Foundation v0 is deliberately not a free-form window manager, visual editor,
arbitrary plugin layout DSL, floating-window system, or per-component render
loop.

Files is the first real native app consumer. It must cease being a curtain in
front of live terminals and become the Stage owner while terminal runtimes stay
alive, hidden input stays inert, Agent Sidebar remains independent, and the
Stage can restore the previous surface on close or failure.

Performance mission: layout computation remains bounded and deterministic;
render remains pure; shell state stays client-side; identical frames remain
skippable; retained PTY dirty-row patching and bounded render queues remain
intact; preview drag produces no disk/PTY/filesystem churn; no new network
layout protocol is introduced without separate evidence and authorization.

## 2. AUTHORITATIVE SOURCE ORDER

The next agent must treat this order as mandatory, not advisory:

1. `AGENTS.md` completely.
2. `CLAUDE.md` completely; currently byte-equivalent project rules, but still
   read it independently.
3. Project skill `$herdr-native-fm` and every mandatory lesson:
   `lessons/errors.md`, `lessons/golden-paths.md`, `lessons/edge-cases.md`, and
   `/home/ayaz/.codex/skills/_shared/common-errors.md`.
4. `.codex/BOOTSTRAP.md`.
5. `.codex/CURRENT.md`.
6. `.codex/TASKS.md`.
7. `.codex/CHANGE-PIPELINE-TASKS.md`.
8. This `.codex/HANDOFF.md` current checkpoint and relevant historical
   evidence sections.
9. `.codex/MEMORY.md` for stable decisions and operational lessons.
10. `.planning/STATE.md`.
11. Approved design and execution plans:
    - `docs/superpowers/specs/2026-07-15-herdr-shell-foundation-v0-design.md`
    - `docs/superpowers/plans/2026-07-15-herdr-shell-file-manager-program-plan.md`
    - `docs/superpowers/plans/2026-07-15-herdr-shell-foundation-v0-implementation.md`
    - `docs/superpowers/plans/2026-07-15-herdr-file-manager-post-shell-implementation.md`
12. Current SF4 evidence:
    `.codex/evidence/shell-foundation-sf4-stage-progress.md`.
13. `.local/ISOLATED-DEV-TEST.md` before any manual runtime test.

`rust-dev` is mandated by the project skill, but
`/home/ayaz/.codex/skills/rust-dev` currently resolves to a missing canonical
Claude target. Do not claim it loaded. Report the tooling gap and follow the
tracked Rust rules, Herdr lessons, TDD protocol, and existing code patterns.
Do not repair global skill links unless the user separately authorizes that
out-of-repository tooling change.

Use `$ratatui-design-intelligence` only when the task genuinely requires new
reference/design comparison. Its output is evidence, not product-code
authority. The non-product `herdr-change-pipeline` skill is not implemented
yet; T3.1 is its next RED task when that lane is unpaused.

## 3. REQUIRED SESSION BOOTSTRAP TRIGGERS

Immediately after reading the sources, the next agent must create/update its
in-session task list. It must ingest every unchecked task from both canonical
registries; it may group them for display but may not drop, silently merge, or
renumber them. At most one product microtask is in progress. The first active
item is SF4.1-08; every later task stays pending.

Required read-only start commands:

```bash
git status --short --branch
git log --oneline -12
git remote -v
git rev-parse HEAD
git ls-remote origin refs/heads/feat/native-fm refs/heads/master
```

Never print credential-bearing MCP config or broad process arguments. Never
use `git add -A`, force push, push `upstream`, open an upstream issue/PR, reset
or discard user changes, or touch `.superpowers/`.

## 4. CODEBASE MEMORY / ARCHITECTURE-FIRST PROTOCOL

Graph discovery precedes source grep for code symbols and relationships:

1. Call `index_status(project="home-ayaz-projects-herdr")`.
2. Search current `AppState.try_open_file_manager_with` and `miller_layout`.
3. Read the exact new transaction source with `get_code_snippet`.
4. Locate the frozen terminal-runtime preservation fixture and trace its
   runtime calls before editing.
5. Use `trace_path` for callers/callees/data flow and `get_architecture` for
   ownership boundaries; use grep only after graph discovery or for literal,
   config, and documentation searches.
6. Reject `ready` alone. If the built-in channel still returns 20,291 / 94,542
   or cannot find the transaction, label it stale.
7. Do not restart/kill the proxy or user processes. The proven safe refresh is:

```bash
CBM_WORKERS=1 codebase-memory-mcp cli index_repository \
  '{"repo_path":"/home/ayaz/projects/herdr","mode":"fast","persistence":false}'
```

8. Prove refresh with CLI/built-in status, current symbol search, and exact
   snippet. Record node/edge counts and which transport supplied the evidence.

The graph is an architecture map, not authority to mutate unrelated code.
Classify every new fact as shared runtime/server truth or client/TUI
presentation state before adding it.

## 5. CURRENT ARCHITECTURE AND OWNERSHIP

- `AppState` remains pure client/application data; terminal runtime ownership
  is separate.
- `src/ui/surface_host.rs` owns bounded typed Stage presentation identity:
  `BuiltInAppId`, `AppInstanceId`, `AppInstance`, `AppSurfaceRef`, and
  `StageState`.
- Stage storage is a fixed array capped at 16 built-in instances. Generation is
  checked `u32`; exhaustion and capacity overflow fail without mutation.
- Terminal is the default singleton instance. Files activation records the
  previous surface. Files reactivation is idempotent. Close removes Files and
  restores Terminal.
- `AppState::try_open_file_manager_with` snapshots Stage and existing pane-
  focus history, activates Files, and commits prepared `FmState` only on
  success. Preparation failure restores exact Stage/focus and does not clear a
  pending sidebar navigation request. Success clears that stale request once.
- Existing `previous_pane_focus` is pane-navigation history, not a completed
  SF4 focus-scope model. SF4.2 owns new focus routing; do not overload this
  field speculatively.
- `AppState.file_manager: Option<FmState>` and the legacy Files curtain still
  render today. SF6, not SF4.1, removes that curtain and migrates Files to the
  Stage surface.
- `TerminalRuntimeRegistry` and server-owned terminal facts must remain alive
  and independent of Stage presentation. The pending eighth SF4.1 test freezes
  this bridge.
- Render is pure. Filesystem reads, watcher work, preview decoding/highlighting,
  file operations, and runtime mutation stay in state refresh/App worker paths.
- Protocol remains version 16. SF4.1 adds no protocol, server, pane, tab,
  workspace, terminal, process, watcher, dependency, or persistence identity.

## 6. SF4.1 ATOMIC HISTORY AND CURRENT TEST EVIDENCE

| Slice | RED | GREEN | State |
|---|---|---|---|
| default terminal Stage | `557bcc77` | `6a18f0c7` | closed |
| Files activation history | `f22bdac4` | `b9180de3` | closed |
| singleton reactivation | `96e6cddb` | `d20403d0` | closed |
| 16-instance bound | `27ad2a79` | `e8ef80ac` | closed |
| generation exhaustion | `207c9da3` | `f31ab28a` | closed |
| close restoration | `a5e5bace` | `e1c82036` | closed |
| failed-open rollback | `056f0879` | `f0f32075` | closed |
| terminal runtime preservation | not written | not written | NEXT |

The seventh RED run
`7df14514-5602-42d2-962e-fd5803c038b4` compiled, ran 1 test, and failed only
on the missing Stage restoration assertion. The final Stage run
`6fed21bd-e1aa-46f7-90a4-b617e0b7b0a6` passed 7/7. Open/cwd/render passed 3/3
in `b8487190-8d24-4ceb-b2c5-e7c0556e70ef`; close authority passed 1/1 in
`10da9a43-b851-489a-b907-b2526439c696`; toggle passed 2/2 in
`6d747dc5-b2e2-4069-bfd6-5f875c477849`.

Full repository verification at `f0f32075`:

- Nextest `a9d9e8b1-7a9f-403f-9d34-499d0f13a612`: 3,299/3,299 passed,
  one named B0 host probe skipped, zero retry.
- Linux all-target Clippy with `-D warnings`: PASS.
- Canonical Windows MSVC bin Clippy with `LIBGHOSTTY_VT_SIMD=false`: PASS.
- Bun integration 5/5 and marketplace 12/12: PASS.
- Python maintenance 64/64: PASS.
- Fmt, diff, added-production-`unwrap()`, and residue boundaries: PASS.

One Nextest expression selected zero tests (`85af8790-...`) and was explicitly
rejected as evidence. Compile/setup/filter/zero-test failures are never RED.

## 7. PRIORITY AND DEPENDENCY MODEL

- **P0 ACTIVE:** SF4.1-08 terminal runtime preservation, then SF4.1 closure.
- **P0 NEXT:** SF4.2 focus/input precedence -> SF4.3 overlay/background
  blocking -> SF4.4 pure surface projection and SF4 closure.
- **P0 AFTER SF4:** SF5 AppDock -> SF6 Files native Stage migration -> FM1 ->
  FM2 -> FM3 -> FM4 -> FM5.
- **P1 PAUSED:** non-product change-pipeline T3.1-T10.9. Unpause only after the
  current sequential product phase closes; never mix files/commits.
- **P2 LATER / NEW AUTHORIZATION:** Apps/Desktop expansion, real TopBar/
  BottomBar/RightPanel consumers, btop/Music/terminal app definitions, app
  catalog/launcher and broader layout customization.
- **P3 TRIGGER-GATED NO-GO:** arbitrary S5 ComponentRegistry and S7 popup
  ownership stack. Implement only after their concrete independent triggers.
- **Explicitly out of scope:** visual layout editor, arbitrary user JSON/TOML
  layout DSL, free redocking, floating windows, unbounded Miller history,
  component-owned render loops, and a new network/layout protocol.

No lower-priority task may preempt P0 merely because it is easier. A blocker
does not authorize jumping lanes; record it and request user direction if safe
in-scope alternatives are exhausted.

## 8. COMPLETE UNCHECKED TASK INVENTORY — 125 ITEMS

The two blocks below are generated from the canonical registries and are part
of this handoff. Count after SF4.1 closure: 36 product-program items plus 89
change-pipeline items. The next agent must recount after reading; any mismatch
means registry drift and must be reconciled before code.

### 8.1 Product / Shell / FM / Deferred Registry — exact unchecked items

<!-- PRODUCT_OPEN_TASKS_START -->
- [ ] RED-test focus scope entry/restore, active capture, topmost semantic hit,
  page/global shortcut precedence, stale generation rejection, and no-owner
  fallback before adding router production state.
- [ ] Add one bounded focus/capture router shared by mouse and keyboard. It
  must route overlay -> capture -> active Stage surface -> shell/page -> global
  and never infer authority from paint output or stale coordinates.
- [ ] Prove terminal resize, surface close/failure, focus target disappearance,
  hidden/zero regions, and capture cancellation restore one valid owner without
  replay, duplicate action, or stuck capture.
- [ ] RED-test every active overlay/context/modal path against Files, shell,
  dock placeholder geometry, and terminal input. Topmost owner must consume the
  event even when disabled or stale; no event may fall through to a hidden
  terminal or background surface.
- [ ] Make background hit areas, cursor ownership, scroll ownership, and raw
  terminal input inert whenever a topmost overlay or another Stage surface owns
  the interaction.
- [ ] Cover right/middle/modified mouse, double-click timing, wheel, drag,
  keyboard prefix/global keys, stale frame generations, close/reopen, and tiny
  terminal failure paths.
- [ ] Split cached shell geometry projection from typed active-surface
  projection without moving filesystem, worker, PTY, or runtime work into
  render.
- [ ] Prove identical active state skips network frame production; terminal
  dirty-row retained patching and bounded render queue behavior remain frozen;
  switching Stage visibility does not destroy or resize hidden terminal runtime
  on every input/render event.
- [ ] Close SF4 UI/input/failure/performance/platform/full-gate/Git/graph
  evidence before starting SF5. Product and change-pipeline commits remain
  separate.
- [ ] Render icon-only Terminal/Files dock at preferred 5, min 3, max 9 cells.
- [ ] Add stable active/running/disabled targets, singleton activation, bounded
  right-click name popover, overlay blocking, resize/collapse, and tiny-terminal
  behavior.
- [ ] Close UI/input/failure/performance/full-gate/Git/graph evidence.
- [ ] Replace the terminal curtain branch with typed `NativeFiles` Stage
  projection/render while preserving AppDock/LeftPanel independence.
- [ ] Preserve `FmState`, watcher, text/image workers, operations, selection,
  context menus, agent handoff, and all failure/recovery semantics.
- [ ] Prove singleton open/reactivate/close/failure restores previous Stage and
  focus; terminal process stays alive but hidden input/hits/cursor are absent.
- [ ] Close snapshot, render queue, retained PTY, isolated runtime, performance,
  full-gate, Git, remote-SHA, and graph evidence.
- [ ] Add logical history <=32, resident directory projections <=5, and at
  most five visible complete columns.
- [ ] Add native horizontal wheel, Shift+wheel, and bounded header navigation;
  clamp after path/cache/terminal shrink and clear stale hits.
- [ ] Prove close/reopen reset, inaccessible ancestors, render purity, resource
  bounds, full gates, publication, and graph freshness.
- [ ] Reuse the Shell resize transaction for min 16/preferred 28/max 64 column
  widths.
- [ ] Prove preview causes zero persistence/PTY/filesystem/image-target churn;
  commit updates one revision and at most one final image target.
- [ ] Close stale divider, terminal resize, cancel, 1,000-move bound,
  cross-layer/full/performance/Git/graph gates.
- [ ] Generate stable column/directory/entry/generation row targets for every
  rendered directory column.
- [ ] Route plain/right/double/wheel gestures in parent/current/preview/ancestor
  columns; keep Ctrl/Shift operation authority current-directory-only.
- [ ] Revalidate non-current paths before mutation; consume stale/reordered/
  deleted/evicted targets without replay or side effect.
- [ ] Close overlay/background-blocking, context/operation/selection, isolated
  SGR mouse, full-gate, Git, and graph evidence.
- [ ] Append one child segment on directory selection, truncate descendants on
  ancestor branch change, and replace deeper chain with file preview.
- [ ] Restore exact child focus/cursor/viewport; handle missing/hidden/reordered/
  deleted/root/inaccessible paths deterministically.
- [ ] Preserve all N2.1 tests, chain <=32, resident <=5, watcher generations,
  close/reopen reset, adversarial 10,000-action invariants, and performance.
- [ ] Close full gates, isolated deep-navigation proof, publication, and graph.
- [ ] Measure inline final column, Shell RightPanel, and adaptive hybrid across
  terminal/path/Unicode/preview/failure/focus/performance fixtures.
- [ ] Record raw evidence and explicit GO/NO-GO. A NO-GO keeps inline preview;
  a GO requiring product code must receive a separate approved micro plan.
- [ ] Commit the evidence/decision independently. Do not expand into
  Apps/Desktop or speculative RightPanel consumers.
- [ ] Implement and verify `herdr-change-pipeline`, adapters, pilots, Git
  publication, and graph refresh; paused at T3.1 while the sequential active
  product lane closes its current phase.
- [ ] S5 ComponentRegistry only when a second real component/page proves the
  abstraction; do not build a speculative registry.
- [ ] S7 popup stack with ownership, focus, close ordering, and nested popup
  tests.
<!-- PRODUCT_OPEN_TASKS_END -->

### 8.2 Non-Product Change Pipeline — exact unchecked items

<!-- PIPELINE_OPEN_TASKS_START -->
- [ ] **T3.1** Write RED `TP-CHG-MODULE` tests for module identity, version,
  directories, required documents, and default authorization=false.
- [ ] **T3.2** Create `.codex/skills/herdr-change-pipeline/` with `SKILL.md`,
  `README.md`, `AGENTS.md`, `module.json`, `assets/`, `references/`, `scripts/`,
  `tests/`, `evals/`, `lessons/`, and `cartography/`.
- [ ] **T3.3** Implement minimal manifest/schema validation and deterministic
  diagnostics until scaffold tests pass.
- [ ] **T3.4** Document skill routing, output ownership, resume behavior,
  source-of-truth order, and the separation from Ratatui reference research.
- [ ] **T3.5** Add errors, golden paths, edge cases, and shared-error routing.
- [ ] **T3.6** Verify the scaffold independently of Herdr product compilation.
- [ ] **T4.A0.1** RED-test every intake mode and reject unknown/ambiguous modes.
- [ ] **T4.A0.2** Model goals, inputs, evidence sources, current-work state,
  product authorization=false, and mode-conditional artifacts.
- [ ] **T4.A0.3** Implement `mid_flight_adoption` metadata: existing branch,
  commits, diffs, tests, known debt, current failures, and preserved evidence.
- [ ] **T4.A0.4** Block rather than fabricate when mandatory MCP/source evidence
  is unavailable.
- [ ] **T4.A1.1** RED-test missing actors, scenarios, success criteria, and
  explicit non-goals.
- [ ] **T4.A1.2** Emit measurable target behavior and acceptance boundaries for
  features, bugs, pages, layouts, runtime work, and composite requests.
- [ ] **T4.A2.1** RED-test orphan nodes, illegal level jumps, missing ownership,
  and missing failure/recovery leaves.
- [ ] **T4.A2.2** Implement the canonical chain: initiative -> experience/
  workflow -> page -> region/layout -> component -> interaction/behavior ->
  state transition -> failure/recovery.
- [ ] **T4.A2.3** Preserve parent/child traceability and stable identifiers.
- [ ] **T4.A3.1** RED-test omitted required dimensions, duplicate ownership,
  unresolved contradictions, and unjustified conditional omissions.
- [ ] **T4.A3.2** Cover product; behavior; page/input; layout/capability;
  component/tokens; data authority; runtime/API/event/PTY; failure/security/
  resources; persistence/migration; platform/accessibility; performance; and
  integration/license dimensions.
- [ ] **T4.A3.3** Record evidence, confidence, conflicts, and dependency edges.
- [ ] **T4.A4.1** RED-test single-option conclusions without explicit
  justification and visual-only pattern matching.
- [ ] **T4.A4.2** Produce alternative concepts, reusable patterns, rejected
  options, tradeoffs, capability fallbacks, and reversibility notes.
- [ ] **T4.A5.1** RED-test stale/absent graph evidence and `ready`-only
  freshness claims.
- [ ] **T4.A5.2** Map current owners, call/data paths, protocol/persistence
  boundaries, existing tests, and reuse candidates.
- [ ] **T4.A5.3** Emit current-versus-target architectural and functional fit.
- [ ] **T4.A6.1** RED-test unresolved conflicts and unsupported go decisions.
- [ ] **T4.A6.2** Select target architecture, behavior, data flow, fallbacks,
  budgets, and `go`, `conditional_go`, `no_go`, or `blocked` status.
- [ ] **T4.A7.1** RED-test incomplete traceability, missing decision evidence,
  conditional gaps, and mutable handoff fields.
- [ ] **T4.A7.2** Emit and validate immutable `change-intent-package.json`.
- [ ] **T4.A7.3** Prove native, reference-adapted, composite, no-go, blocked,
  and mid-flight packages through fixtures/evals.
- [ ] **T4.A7.4** Verify that A7 readiness still grants no product mutation.
- [ ] **T5.I0** Reject absent/invalid handoff, unapproved target, stale current
  state, or missing product authorization; accept mid-flight evidence only
  after provenance and current-phase classification.
- [ ] **T5.I1** Generate PRD, authority checklist, risk register, non-goals,
  rollback, compatibility, and migration obligations.
- [ ] **T5.I2** Refresh graph/repository evidence and detect drift between A7
  handoff and the live target.
- [ ] **T5.I3** Freeze current behavior, target behavior, semantic diff,
  retained behavior, change strategy, and ownership impact.
- [ ] **T5.I4** Build the test-point catalog with `what`, `current`, `expected`,
  `diff`, `result`, and `reason` for every applicable obligation.
- [ ] **T5.I5** Produce dependency-ordered implementation slices, test slices,
  commit boundaries, rollback points, and owned file sets.
- [ ] **T5.I6** Capture characterization evidence before moving behavior or
  architecture.
- [ ] **T5.I7** Require an observed behavior-specific RED; reject compile,
  environment, setup, flaky, or already-green false REDs.
- [ ] **T5.I8** Implement the minimum GREEN change and preserve exact command
  output as evidence.
- [ ] **T5.I9** Refactor only behind green tests; enforce local ownership and
  invariants.
- [ ] **T5.I10** Run cross-layer and cross-feature tests across all applicable
  families.
- [ ] **T5.I11** Verify failure, recovery, security, resources, capability
  negotiation, and degraded behavior.
- [ ] **T5.I12** Verify declared latency, allocation, throughput, memory, queue,
  and terminal-render budgets with calibrated fixtures.
- [ ] **T5.I13** Run complete repository, platform, protocol, migration,
  dependency, docs, and release-cadence gates applicable to the change.
- [ ] **T5.I14** Audit evidence, targeted staging, atomic commits, allowed
  publication, remote SHA, graph reindex, and current-symbol freshness.
- [ ] **T6.1** Server/runtime truth versus TUI/client projection.
- [ ] **T6.2** Snapshot/event ordering, revision, replay, duplicate, gap, and
  slow-subscriber behavior.
- [ ] **T6.3** PTY/terminal chunk boundaries, UTF-8/ANSI splits, queue pressure,
  resize, EOF, detach/reattach, and multi-pane throughput.
- [ ] **T6.4** Plugin host timeouts, crashes, output bounds, process cleanup,
  malformed data, version compatibility, and path confinement.
- [ ] **T6.5** Page/layout/component keyboard, mouse, focus, modal, resize,
  Unicode, narrow viewport, empty/loading/error, and terminal fallback states.
- [ ] **T6.6** Persistence interruption, corruption, migration, disk-full,
  concurrent owner, quota, and large-scrollback behavior.
- [ ] **T6.7** Platform isolation and Linux/macOS/Windows policy differences.
- [ ] **T6.8** Performance regression, slow-client isolation, soak, task leak,
  zombie process, and chaos behavior.
- [ ] **T6.9** Backward/forward protocol, old/new client, old/new plugin, and
  old persisted-state compatibility.
- [ ] **T7.1** P14 Ratatui/reference-project adapter.
- [ ] **T7.2** Native feature fixture.
- [ ] **T7.3** Mid-flight file-manager feature plus bugfix fixture.
- [ ] **T7.4** Page and interaction-flow fixture.
- [ ] **T7.5** Responsive layout and tiling fixture.
- [ ] **T7.6** Design-system/component/token fixture.
- [ ] **T7.7** Runtime capability and protocol fixture.
- [ ] **T7.8** Composite multi-dimension conflict fixture.
- [ ] **T7.9** Explicit no-go and blocked-MCP fixtures.
- [ ] **T7.10** Unauthorized delivery fixture proving I0 rejection.
- [ ] **T7.11** Verify that native mode invents no repository/source/license and
  reference mode omits no source/provenance/license obligations.
- [ ] **T8.1** Focused schema/validator unit tests.
- [ ] **T8.2** Complete tests for both skills and all negative fixtures.
- [ ] **T8.3** JSON parse, schema, stable-ID, version, and deterministic-output
  checks.
- [ ] **T8.4** Skill validation, README/AGENTS/SKILL consistency, and lesson
  format checks.
- [ ] **T8.5** Eval coverage for A0-A7, I0-I14, adapters, mid-flight adoption,
  blocked, no-go, and unauthorized paths.
- [ ] **T8.6** Legacy P0-P14 backward-compatibility verification.
- [ ] **T8.7** Product isolation and exact diff-boundary verification.
- [ ] **T8.8** Placeholder, whitespace, broken-link, and untracked-artifact
  scans.
- [ ] **T8.9** Proportional `just check` or documented exact equivalent.
- [ ] **T9.1** Preserve each baseline, RED, GREEN, refactor, governance, fixture,
  and evidence concern in reviewable atomic commits.
- [ ] **T9.2** Target-stage only the declared owned files and verify the staged
  name list before every commit.
- [ ] **T9.3** Fetch and prove fast-forward ancestry before any authorized push.
- [ ] **T9.4** Push only the permitted CyPack feature branch/ref; never
  `upstream`, never force.
- [ ] **T9.5** Verify exact local/remote SHA equality after publication.
- [ ] **T9.6** Reindex Codebase Memory after committed implementation changes.
- [ ] **T9.7** Record node/edge counts and query current pipeline/module symbols;
  never infer freshness from `ready` alone.
- [ ] **T10.1** Run one native page/feature request through A0-A7 without
  product mutation.
- [ ] **T10.2** Run one reference project through P0-P14 -> adapter -> A7.
- [ ] **T10.3** Run one mid-flight file-manager feature/bugfix adoption and
  prove existing evidence preservation plus remaining-gate enforcement.
- [ ] **T10.4** Run one composite conflict to a justified conditional-go/no-go.
- [ ] **T10.5** Prove unauthorized I0 rejection and blocked-MCP truthfulness.
- [ ] **T10.6** If separately authorized, run one non-product fixture through
  I14 before using the pipeline on Herdr product code.
- [ ] **T10.7** Record new errors, golden paths, edge cases, and any cross-skill
  lessons in the required tables.
- [ ] **T10.8** Update this registry, `.codex/CURRENT.md`, `.codex/TASKS.md`, and
  handoff state with exact final evidence and next action.
- [ ] **T10.9** Perform final self-review: requirements, tests, failure paths,
  Git state, publication state, graph freshness, and remaining blockers.
<!-- PIPELINE_OPEN_TASKS_END -->

## 9. EXACT NEXT MICROTASK CONTRACT

SF4.1-08 is closed (RED `784fdc2e`, GREEN `944a9d4c`; evidence in
`.codex/evidence/shell-foundation-sf4-stage-progress.md`). The next microtask
is the first SF4.2 RED. Before production code, announce these test points
with expected result and reason:

1. `shell_input_router_follows_frozen_precedence` (table-driven)
   - Test: overlay, active capture, overlapping topmost hit, focused
     component, page shortcut, global shortcut, and no-target rows resolve
     through one precedence table.
   - Expected: exactly one owner per event following
     overlay -> capture -> active Stage surface -> shell/page -> global; the
     no-target row is inert.
   - Reason: input authority must be explicit and total before SF4.3 blocking
     and SF6 migration can rely on it.
2. Stale/inert rejection companions within the same contract.
   - Test: stale hit generation, collapsed/inert region focus, and hidden
     background targets.
   - Expected: consumed without action; no fall-through to hidden terminal
     input.
   - Reason: old coordinates and paint output must never become authority.
3. Recovery.
   - Test: terminal resize, surface close/failure, focus target
     disappearance, and capture cancellation.
   - Expected: one valid owner is restored without replay, duplicate action,
     or stuck capture.
   - Reason: the router must fail closed under lifecycle churn.

Follow `docs/superpowers/plans/2026-07-15-herdr-shell-foundation-v0-implementation.md`
Task SF4.2 for the complete RED list and commit messages
(`test: define shell focus and input ownership`;
`feat: route shell input through semantic ownership`).

## 10. FILE OWNERSHIP AND COMMIT BOUNDARY

Likely SF4.2 owned files, subject to graph/source confirmation:

- create/modify `src/app/input/shell.rs`; modify `src/app/input/mod.rs`,
  `src/app/input/overlays.rs`, `src/app/state.rs`.
- `src/ui/surface_host.rs` only if the router consumes the typed surface view.

Forbidden in this microtask: AppDock render, Files render migration,
watcher/preview/operation changes, protocol/persistence, dependencies,
docs/website, change-pipeline module, `.superpowers/`, stable runtime tooling,
and unrelated refactors.

Every concern has separate targeted staging. RED and GREEN are separate atomic
commits but are never pushed as a RED-only remote tip. Continuity/evidence is a
separate docs commit after fresh product verification.

## 11. GIT PUBLICATION PROTOCOL

1. Inspect status/diff; preserve user changes.
2. Stage only named owned files with `git add -- <paths>`.
3. Run `git diff --cached --check` and inspect staged names/stat.
4. Use lowercase conventional commits, no emoji, no AI co-author.
5. Run proportional gates after GREEN and complete direct `just check` before
   phase closure/publication. `just` is currently absent; execute lowercase
   `justfile` commands directly.
6. `git fetch origin feat/native-fm master`.
7. Prove both remote refs are ancestors of local HEAD. Never rewrite history.
8. Push sequentially only to CyPack:

```bash
git push origin HEAD:feat/native-fm
git push origin HEAD:master
```

9. Verify both exact remote SHAs equal local HEAD.
10. Never push `upstream`, force, or publish a RED-only tip.

Standing user authorization covers these targeted atomic commits and CyPack-
only fast-forward pushes; do not repeatedly ask for commit-message alignment.

## 12. SAFETY / FAILURE / NON-HAPPY-PATH RULES

- Never kill, restart, or signal user processes.
- Never touch installed stable Herdr or its stable/inherited socket.
- Manual runtime tests use `.local/ISOLATED-DEV-TEST.md`, cleared Herdr socket
  and identity variables, unique throwaway XDG roots, semantic exit, exact
  test-owned PID/resource cleanup, and residue proof.
- Do not run a debug binary against inherited `HERDR_SOCKET_PATH` or
  `HERDR_CLIENT_SOCKET_PATH`.
- No production `unwrap()`; log with `tracing`; platform behavior remains in
  `src/platform/` or compile-gated according to project rules.
- Render never reads the filesystem or mutates App/runtime state.
- Input never derives destructive authority from labels, glyphs, painted
  buffers, or stale coordinates.
- Overlay/topmost ownership consumes input; disabled/stale targets do not fall
  through to background terminal actions.
- Runtime/session facts stay server-owned; Stage/focus/hits/layout remain TUI
  presentation state unless evidence proves otherwise.
- Every filesystem operation retains execution-time TOCTOU revalidation,
  bounded worker ownership, explicit partial failure, and recovery evidence.
- Compile/setup/environment/filter/flaky failures are not RED. A passing test
  written before production may be characterization but not missing-behavior
  RED.
- No completion claim without fresh command output and explicit exit status.

## 13. ENVIRONMENT AND TOOLING FACTS

- `just`: absent; use the exact direct recipe.
- Rust tests: Cargo Nextest; total at this checkpoint is 3,300 inventory items
  with 3,299 run and one intentionally skipped B0 real-host probe.
- Windows canonical lint:
  `LIBGHOSTTY_VT_SIMD=false cargo clippy --bin herdr --locked --target x86_64-pc-windows-msvc -- -D warnings`.
- `RIPGREP_CONFIG_PATH` may point to a missing file; use
  `env -u RIPGREP_CONFIG_PATH rg ...`.
- Codebase Memory native parallel extraction has a known
  `munmap_chunk(): invalid pointer` failure. Use `CBM_WORKERS=1` sequential CLI
  refresh; do not restart the MCP proxy.
- Long `exec_command` jobs may return a live `session_id`; poll until explicit
  `exit_code`. Never infer completion from an orchestration wrapper ending.
- The current Codex built-in graph channel may remain stale until a fresh
  session; use truthful CLI-labelled evidence when necessary.

## 14. COMPLETE PROGRAM OUTCOME ORDER

1. Close SF4.1 runtime-preserving typed Stage.
2. Close SF4.2 focus and capture ownership.
3. Close SF4.3 overlay/background blocking.
4. Close SF4.4 pure surface projection and retained-render performance.
5. Build SF5 icon-only bounded AppDock.
6. Migrate Files in SF6 without losing any existing FM capability/failure
   semantics.
7. Add FM1 bounded horizontal history/viewport.
8. Add FM2 transactional column resize.
9. Add FM3 all-column stable mouse ownership.
10. Add FM4 Finder-like bounded path-stable navigation.
11. Make FM5 evidence-based preview/Inspector placement GO/NO-GO.
12. Only then request/confirm Apps/Desktop expansion scope.

## 15. NEXT-SESSION COPY/PASTE TRIGGER

The canonical copy/paste prompt is `.codex/NEXT-SESSION-PROMPT.md`. Start with:

```bash
herdr-codex
```

Then paste the complete prompt. It orders skill/lesson loading, graph-first
architecture reconstruction, exact 128-task ingestion, Git state verification,
test-point declaration, TDD, safe publication, and handoff maintenance.

## 16. HANDOFF ACCEPTANCE CHECKLIST

The next agent has not completed onboarding until it can state all of these
without guessing:

- current local and remote SHAs;
- why SF4.1 is 7/8 rather than complete;
- the next exact RED and why compile/setup failure is invalid;
- Stage versus terminal-runtime ownership;
- product, pipeline, deferred, and later-program priorities;
- exact unchecked task count after recount;
- current graph count and a current-symbol snippet;
- why the built-in MCP channel may be stale;
- direct `just check` equivalent;
- stable runtime and `.superpowers/` safety boundaries;
- allowed commit/push refs and forbidden upstream actions.

If any answer is missing, stop before editing and finish the relevant read/
graph/Git inspection. Do not ask the user to reconstruct information already
present in these canonical files.

## 17. HISTORICAL HANDOFF APPENDIX

Everything below preserves prior-session evidence and commit history. Its old
"next action" and environment counts are historical unless repeated in the
authoritative checkpoint above.

# HISTORICAL SESSION HANDOFF — Herdr Native FM — 2026-07-16

## 1. SONRAKI ADIM

The user explicitly approved the bounded twelve-phase product program: SF0-SF6
then FM1-FM5; Apps/Desktop remains later. SF0-SF3 are closed. SF3 now includes
transactional divider resize, committed bounded collapse/restore, fail-closed
two-axis scroll ownership, and typed snapshot-v4 persistence with v3 migration
and invalid-shell containment.

The published SF3.3 chain is migration RED/GREEN `da41127f` / `be917131`,
typed-v4 RED/GREEN `352e394d` / `385a0bcc`, corruption REDs `1b06456e` and
`e12e78cf` with GREEN `d22d0d15`, restore-authority RED/GREEN `6fb8f803` /
`ef9d7f2b`, and collapse round-trip RED/GREEN `4dd62047` / `90be6893`.
At the product checkpoint, CyPack `feat/native-fm` and fork `master` both
resolved to exact product SHA
`90be689359988424b2a7c6206ff45a3207422196`.

Fresh SF3.3 closure is snapshot matrix 12/12, broad persistence/shell/sidebar
input 137/137, frozen SF1 11/11, full Nextest 3292/3292 plus only the named B0
real-host skip, Linux/Windows Clippy, Bun 17/17, Python 64/64, and clean
fmt/diff/production-unwrap/residue boundaries. The fresh sequential graph is
20,291 nodes / 94,542 edges; exact source reads prove `miller_layout`,
`ShellSnapshotV1`, `SessionSnapshot.restored_left_panel_preference`, and
`ShellPresentationState.from_restored_left_panel`. Full evidence is
`.codex/evidence/shell-foundation-sf3-persistence.md`.

The immediate action is SF4.1: graph-first trace the existing terminal/Files
swap, tab/workspace identity, `FmState`, input precedence, and runtime ownership.
Then write the smallest compile-valid behavior RED
`stage_starts_on_terminal_workspace`. Do not add Stage production state before
that assertion RED. Keep AppDock rendering, Files Stage migration, and the
broader SF4 router out of this first state slice.

The previous S6/dynamic-N2 NO-GO was valid when no real consumer/demand
existed. New explicit AppDock/WorkspaceStage/Files, resize/collapse/overlay,
horizontal Miller, column resize, and all-column mouse demand supplies the
missing bounded trigger. It does not authorize S5 arbitrary registry, S7 popup
stack, unbounded history, visual editor, or Apps/Desktop. The non-product
change-pipeline lane remains paused at T3.1 and must not be mixed with product
commits. Never touch stable Herdr/socket/processes or stage `.superpowers/`.

## 2. AKTİF PROJE

- Project: `/home/ayaz/projects/herdr`
- Branch: `feat/native-fm`
- Goal: native, Lua-free file manager on Herdr's composable TUI foundation.
- Acting GitHub identity: `CyPack` external contributor; fork-only writes.

## 3. KAYNAK OTURUM

- Claude resume ID: `f53c720f-f795-4778-970b-d227714ffb1a`
- Raw JSONL: `/home/ayaz/.claude/projects/-home-ayaz-projects-herdr/f53c720f-f795-4778-970b-d227714ffb1a.jsonl`
- SHA-256: `368fb0a5045d1435c64679c8d0dea2a4283d58891231c91bb6e30350b69c2d30`
- Span: `2026-07-14T00:19:57Z`–`02:41:40Z`
- Reconstruction: `.codex/evidence/claude-session-f53c720f.md`

## 4. CLAUDE OTURUMUNDA TAMAMLANANLAR

1. Onboarding, rust-dev lessons, codebase graph freshness audit, task restoration.
2. S2 named region extraction, commit `c043c1e`.
3. Isolated-development test documentation and Claude semantic hook.
4. S3 re-scope and A2.1 center-region directory render, commit `d026e94`.
5. `prefix+f` activation across the two-layer keybinding system, commit `74d3cc9`.
6. Keyboard navigation/input interception, commit `d2b27e6`.
7. Detailed native-FM next-session prompt generated.

## 5. BU CODEX OTURUMUNDA TAMAMLANANLAR

- Closed SF3.3 through eleven atomic product commits ending at `90be6893`:
  snapshot-v4 typed shell state, v3 compatibility migration, corruption
  containment, v4 restore authority, and committed collapse/restore capture.
- Ran fresh SF3 closure: snapshot matrix 12/12, broad 137/137, SF1 11/11, full
  3292/3292 plus only B0, Linux/Windows Clippy, Bun 17/17, Python 64/64, and
  graph 20,291 / 94,542. Snapshot version is 4, wire protocol remains 16, and
  stable runtime/socket/processes were untouched.
- Closed SF3.2 through ten atomic product commits ending at `45a2e87e`:
  bounded collapse/restore, shared input adapter, monotonic cache revision, and
  fail-closed two-axis topmost scroll ownership.
- Ran fresh phase closure: scroll 6/6, broad 202/202, SF1 11/11, full
  3281/3281 plus only B0, Linux/Windows Clippy, Bun 17/17, Python 64/64, and
  graph 20,236 / 94,402. Stable runtime/socket/processes were untouched.
- User approved the detailed T1 plans and opened T2 execution.
- Preserved the 28-file Ratatui intelligence baseline, then implemented v2.1
  through separate identity, cross-stack, phase/run, and governance RED/GREEN
  commits. Added fail-closed MCP/product isolation, eval, cartography, lessons,
  and human/agent governance without touching Rust or stable runtime state.
- Recovered and verified the Claude transcript directly from local storage.
- Implemented A2.2 responsive Miller columns with cached parent/preview state.
- Added RED tests first, then achieved full GREEN verification.
- Built this Codex CLI continuity, memory, skill, hook, launcher, and handoff package.
- Committed A2.2 as `6c7c58f`, reindexed it, and fast-forward pushed only the
  CyPack feature branch and fork master.
- Implemented A4 native file watching test-point-first: pure normalization,
  generation/lifecycle safety, bounded channel/coalescing, path-preserving
  refresh, real-filesystem convergence, explicit polling fallback, and a
  2-second reconciliation safety-net.
- Made two pre-existing wall-clock-sensitive tests deterministic after full
  nextest exposed them under parallel load.
- Committed A4 separately as `01ba91d` and the deterministic test-only fixes
  separately as `8cd4e89`, using targeted staging only.
- Completed B0 Image Path Beta test-point-first with generated exact RGBA,
  malformed decode, synthetic local placement, upload/display/dedup/view/
  replacement/removal lifecycle, cursor framing, and isolated real-host tests.
- Captured a visible local Path Beta pattern in throwaway Kitty and an
  independent Path Alpha Yazi preview baseline; closed only the test windows
  with targeted semantic input.
- Recorded a conditional GO for B2: reuse existing `kitty_graphics`, bound all
  decode/allocation work, keep I/O outside render, reject stale generations,
  and prove cleanup plus real-host output.
- Committed B0 separately as `bcba84d`, full-reindexed it, and fast-forward
  published only to CyPack feature/master.
- Closed B1.0 dependency research: minimal pure-Rust `syntect 5.3.0` is the
  B1.2 choice, but measured latency requires generation-safe bounded background
  preparation. B1.1 begins without adding it.
- Completed B1 text preview through strict RED/GREEN commits: 64 KiB bounded
  UTF-8 reader, explicit failures, state-refresh ownership, pure deterministic
  syntax preparation, one-active/one-pending generation-safe worker, reload
  identity preservation, lifecycle rejection, Ratatui style mapping, explicit
  error/truncation states, and bounded responsive render.
- Proved cursor navigation, watcher reload, selected-file replace/delete,
  hidden toggle, close/reopen, worker panic/disconnect, narrow/zero geometry,
  long lines, and stale-result rejection without touching stable Herdr.
- Re-ran the actual five-package dependency/OSV delta and the entire direct
  `just check` equivalent. B1 product/test head is `2b2dcd3`; continuity and
  graph/publication follow separately.
- Completed A3 remainder as seven atomic RED/GREEN/scope commits from
  `d713b71` through `9d69c82`: persistent cursor viewport, shared responsive
  CURRENT-row geometry, single/double-click and bounded wheel runtime routing,
  stale-target rejection, preview refresh, and explicit cursor-only v1 scope.
- Ran the full A3 gate and isolated real PTY SGR-mouse cross-check without
  stable Herdr/socket access; exited semantically and removed all throwaway
  state. Full-reindexed and published `9d69c82` to CyPack feature/master only.
- Completed B2 as an auditable range from dependency decision `de1eff5`
  through fallback fix `2989434`: bounded common-format decode/downscale,
  responsive client-local placement, generation-safe worker lifecycle,
  cached Kitty paint/dedup/transition cleanup, and explicit non-Kitty fallback.
- Ran B2/FM/Kitty 96/96, full nextest 2983/2983 plus one named B0 host-probe
  skip, Linux/Windows clippy, Bun 17/17, Python 64/64, fmt, and diff-check.
- Proved the production FM path in isolated Kitty: `assets/logo.png` rendered
  in PREVIEW with 0/271425 image-compare pixel differences, FM close cleared
  the same region to one background color, semantic exit returned code 0, and
  no test process/socket/temp root remained. Stable Herdr was untouched.
- Completed C1.1 test-first: RED contract `0ed5e51`, GREEN product `c9bfbf9`.
  Added client-local Copy/Paste/NewFolder/Delete tags, one responsive pure
  geometry seam, desktop/mobile `ViewState` snapshots, complete-button hiding,
  stale-area clearing, and render consumption without filesystem work.
- Full nextest exposed a pre-existing lifecycle fixture clock race. Proved the
  mixed real/synthetic timing boundary, fixed only the test base clock, and
  committed it separately as `9aa1e59`; no C1 product code was mixed into the
  stability commit.
- Ran the complete direct `just check` equivalent at the C1.1 tip: targeted
  4/4, lifecycle family 27/27, full nextest 2986/2986 plus one named B0 probe
  skip, Linux/Windows clippy, Bun 17/17, Python 64/64, fmt, and diff-check.
- Completed C1.2 test-first: RED `dbc6798`, GREEN `7fd01de`. Added a private
  `NotHandled`/`Consumed`/`HeaderAction(tag)` dispatch seam, exact unmodified
  left-click tag mapping, and fail-closed identity/gap/outside/hidden/zero/
  stale/non-left behavior without AppState/protocol/filesystem mutation.
- Ran exact dispatch 2/2, all FM input 13/13, full nextest 2988/2988 plus one
  named B0 probe skip, Linux/Windows clippy, Bun 17/17, Python 64/64, fmt, and
  diff-check. Cwd, cursor, and fixture disk entries remain unchanged by header
  tag dispatch.
- Completed N3.1 test-first: RED `b5cc95c`, GREEN `510eebc`. Added a pure
  selection/clipboard action-bar ViewState model, client-local clipboard paths,
  selected file/directory identity, explicit empty content, desktop/mobile
  refresh, and render fallback without filesystem or protocol coupling.
- Proved navigation, reload-selected-delete, empty transition, close/reopen,
  selected-name rendering, and clipboard-summary persistence. Gates: 3/3,
  FM 135/135, full 2991/2991 plus one named B0 skip, Linux/Windows clippy,
  Bun 17/17, Python 64/64, fmt/diff clean.
- Completed N3.2 test-first: RED `446613a`, GREEN `267ad91`. Added explicit
  per-action enabled/disabled authority and reasons, prepared cwd writability
  and regular-target support, distinct disabled rendering, and fail-closed
  input dispatch with no disabled-click state/filesystem mutation.
- Proved missing cwd, read-only reload, Unix special target, empty clipboard,
  absent selection, unsupported target, and in-flight precedence. Gates:
  exact authority/preparation/render/dispatch 7/7, broad FM/input/render/Kitty
  165/165, full 2996/2996 plus one named B0 skip, Linux/Windows clippy, Bun
  17/17, Python 64/64, fmt/diff clean.
- Completed C2.1 test-first: RED `d4d404e`, GREEN `9a15328`. Added client-local
  SendAgent/Rename/Delete row tags, one pure responsive name/action geometry
  snapshot, desktop/mobile ViewState lifecycle, and pure render consumption.
- The first 3-cell-per-action prototype passed focused tests but failed two
  broad readability characterizations by truncating ordinary names. Reworked
  complete buttons to one-cell `>`, `r`, and `x` targets; the focused plus
  readability set passed 8/8 and the FM impact set passed 71/71.
- Ran the complete direct `just check` equivalent at the C2.1 tip: full
  nextest 2998/2998 plus one named B0 probe skip, Linux all-target and canonical
  Windows MSVC bin clippy, Bun 17/17, Python 64/64, fmt, and diff-check clean.
  Fast graph reindex is fresh at 18,042 nodes / 84,123 edges and returns the
  new geometry/action symbols; freshness was not inferred from `ready` alone.
- Completed C2.2 test-first: RED `94e4a02`, GREEN `9ef90c6`. Row action
  snapshots now carry stable path identity, and exact unmodified-left dispatch
  requires the live index to match that path and remain operation-supported.
- Proved exact SendAgent/Rename/Delete tags, unchanged name selection,
  non-left/modifier/outside/hidden/closed fail-closed behavior, watcher-style
  reorder rejection, unsupported-target rejection, and zero cwd/cursor/
  clipboard/filesystem side effects. The outer router consumes tags before
  hidden terminal input but deliberately executes no real operation.
- C2.2 gates: exact 3/3, all FM input 17/17, FM impact 74/74, full nextest
  3001/3001 plus one named B0 probe skip, Linux/Windows clippy, Bun 17/17,
  Python 64/64, fmt/diff clean. Fast graph reindex is fresh at 18,049 nodes /
  83,839 edges and returned the current path-bearing area plus validation
  handler snippets; freshness was not inferred from `ready` alone.
- Completed N4.1 test-first as seven atomic commits: state `e876223`/`590e376`,
  lifecycle `1789bbd`/`5c14439`, gesture/render RED `699a6a6`, stable row
  identity RED `fc19237`, and integrated GREEN `86b618a`.
- Added a cursor-independent deduplicated path set and stable anchor, current-
  order inclusive range selection, reload/hidden pruning, enter/leave clearing,
  close/reopen reset, exact plain/Ctrl/Shift mouse gestures, Space and
  Shift+Up/Down keyboard equivalents, and distinct pure multi-row rendering.
- `FileManagerRowArea` now carries stable path identity; same-index watcher
  reorder is consumed without selecting the wrong live entry. Combined or
  unknown modifiers and stale targets fail closed. N4.1 performs no filesystem
  operation and adds no server or wire-protocol state.
- N4.1 gates: focused 7/7, broad FM/watcher/input/render/Kitty 137/137, full
  nextest 3015/3015 plus one named B0 probe skip, Linux/Windows clippy, Bun
  17/17, Python 64/64, fmt/diff clean. Fast graph reindex is fresh at 18,078
  nodes / 83,865 edges and returned live model/input/render connections.
- Completed N4.2 test-first as seven atomic commits: bulk authority RED
  `d5e027f`, ViewState lifecycle RED `0c76017`, bulk authority GREEN `0302b10`,
  bounded selection RED/GREEN `36c815f`/`57e2a44`, and keyboard overflow
  atomicity RED/GREEN `50619ff`/`cb5a43e`.
- Cursor focus no longer grants Copy/Delete authority. Prepared bulk paths
  retain current visible order; zero/one/many labels and file/directory/
  multiple/unavailable kinds are explicit. Stale/ambiguous or any unsupported
  member disables the complete selection; operation-in-flight has precedence.
- Added atomic 4,096-path select-all/range ceilings, Ctrl+A select-all,
  Ctrl+Shift+A clear, stale/duplicate rejection, and keyboard overflow cursor
  preservation. No partial set is silently selected and render remains pure.
- N4.2 gates: focused staged runs 6/6 + 4/4 + 2/2, broad FM/input/render
  112/112, full nextest 3020/3020 plus one named B0 probe skip, Linux/Windows
  clippy, Bun 17/17, Python 64/64, fmt/diff clean. Fast graph reindex is fresh
  at 18,091 nodes / 84,102 edges and returned current selection, builder, and
  keyboard source rather than relying on `ready`.
- Defined the complete C3 test-point contract in `d56e3db`, then completed
  C3.1 model RED/GREEN `5d6fc1d`/`02c60e7` and adversarial in-flight
  precedence RED/GREEN `d9f28b5`/`0832ccc`.
- Added the existing-stack `ContextMenuKind::File` model with deterministic
  Open/Copy/Rename/Delete/Compress/Send-to-Agent order, exact prepared paths,
  file/directory/multiple/unavailable kinds, and explicit item authority.
  No explicit selection produces no menu; mixed invalid selection fails closed.
- C3.1 gates: focused 5/5, combined menu models 7/7, full nextest 3025/3025
  plus one named B0 skip, Linux/Windows clippy, Bun 17/17, Python 64/64,
  fmt/diff clean. Fast graph reindex is fresh at 18,115 nodes / 84,003 edges
  and returned current constructor/variant source rather than relying on ready.
- Completed C3.2 as six atomic RED/GREEN commits: popup geometry
  `69864d6`/`ad5f8a5`, lifecycle/typed intent `73df647`/`45c151f`, and
  disabled render `1078215`/`0915964`.
- Exact current-row and row-action-cell right click now uses stable path
  identity, preserves selected bulk members, replaces unselected targets, and
  opens the existing globally bounded popup at every Miller breakpoint.
- Context-menu keyboard/mouse focus precedes the visible FM. Disabled rows
  remain dim and inert; enabled rows emit only typed client-local intent after
  current path/order/action authority revalidation. Reorder, delete,
  operation-in-flight, outside click, Esc, and FM close are fail-closed. C3.2
  executes no filesystem or agent operation.
- C3.2 gates: popup 4/4, lifecycle 3/3, render 1/1, broad FM/global-menu
  51/51, menu/render 26/26, full nextest 3033/3033 plus one named B0 skip,
  Linux/Windows clippy, Bun 17/17, Python 64/64, fmt/diff clean.
- Fast parallel graph refresh crashed codebase-memory 0.8.1 in native
  Tree-sitter cleanup. No service/process was restarted or killed. A supported
  one-shot `CBM_WORKERS=1` CLI refresh completed with zero extraction errors;
  the current graph is 18,139 nodes / 86,595 edges with fresh C3.2 symbols and
  source snippet evidence.
- Completed C3.3 as RED `0e06181` and GREEN `3c11369`. Enabled, available,
  host-supported `contexts=["file"]` actions append after built-ins in stable
  qualified-ID order; duplicate identities, unknown/wrong contexts, disabled
  plugins, and non-UTF-8 path conversion fail closed.
- Added neutral `PluginActionContext::File` and optional exact
  `PluginInvocationContext.file_paths`; generated next API schema is current.
  Right-click/activation revalidate registry plus path authority, disable races
  emit no intent, and CJK titles use display-cell geometry. No plugin command,
  filesystem operation, agent action, stable socket, or user process was
  touched.
- C3.3 gates: focused 8/8, plugin/context 35/35, FM/watcher/global-menu
  112/112, full 3041/3041 plus only `path_beta_real_host_probe` skipped,
  Linux/Windows clippy, Bun 17/17, Python 64/64, schema/fmt/diff clean. Graph
  is fresh at 18,246 nodes / 85,535 edges with current snippets.
- Completed C4.1 as five RED/GREEN pairs: preflight `386ddce`/`a9f022b`,
  staged COPY `47c753e`/`2848d97`, safe MOVE `e422d03`/`606d7ea`, bounded
  worker `f1590be`/`88cda7f`, and App lifecycle `626b7c3`/`98c51e4`.
  Preflight revalidates exact identities before writes; COPY staged-publishes
  without replacement; MOVE is atomic on one filesystem and copy-before-delete
  on EXDEV. Panic/cancel/partial results are explicit and render stays pure.
- Header/context Copy share exact clipboard authority. Paste owns one App
  worker lane and matching-cwd reconciliation; close/reopen cannot project old
  entries. C4.1 gates: core 15/15, App/worker 8/8, broad 147/147, full
  3064/3064 plus one named B0 skip, Linux/Windows clippy, Bun 17/17, Python
  64/64, fmt/diff/temp clean. Fresh graph: 18,453 / 86,399.
- Completed C4.2 as seventeen atomic test/product commits from `733d423`
  through `917cd57`. Header/context Delete converge on typed exact-path
  confirmation; Trash is default and Permanent requires a separate stage.
  Modified keys, stale/reordered/no-selection/closed/in-flight authority fail
  closed before a worker plan exists.
- Immutable delete preflight snapshots symlink metadata and file identity,
  rejects roots/unnamed paths, and revalidates before every mutation. Trash
  and permanent delete share the C4 worker lane and preserve ordered per-item
  completed/retained/failed evidence across partial errors, cancellation,
  panic, and disconnect. Matching-cwd completion owns reconciliation.
- C4.2 gates: focused 29/29, broad 321/321, full nextest 3086/3086 plus one
  named B0 skip, Linux/Windows clippy, Bun 17/17, Python 64/64, fmt/diff/temp
  clean. Isolated real Trash used throwaway HOME/XDG. Fresh graph: 18,576 /
  86,769 with both `miller_layout` and current delete symbols.
- Completed C4.3 as eighteen atomic test/product commits from `2028bce`
  through `c7043e2`. Context-menu and row Rename require one exact current
  target; stale/reordered/multi-selected/unsupported/closed/in-flight intent
  fails closed. The header has no Rename control and the single-name modal
  deliberately remains single-target.
- Common platform-aware component validation and immutable identity snapshots
  protect both single and typed bounded bulk plans. Single rename uses
  immediate revalidation plus platform no-replace; bulk chains/swaps/cycles
  use private collision-safe staging and deterministic publish. Failure and
  rollback preserve renamed/unchanged/restored/retained/uncertain per-item
  evidence, including exact recovery paths.
- Single and bulk rename reuse the existing operation worker and matching-cwd
  App reconciliation. The typed bulk App boundary is present for a future
  editor surface; it does not silently reinterpret multi-selection in the
  single modal. Render remains pure.
- C4.3 gates: focused/broad rename lifecycle 163/163, full nextest 3109/3109
  plus only `path_beta_real_host_probe` ignored, Linux/Windows clippy, Bun
  17/17, Python 64/64, fmt/diff/temp clean. Real temporary-filesystem tests
  cover file/directory/symlink, races, cycles, swaps, and rollback failure.
  Fresh graph: 18,722 / 88,526 with `miller_layout` and current single, bulk,
  shared-validator, and App-consumer symbols.
- Completed TP-C4.4-PROGRESS as ten atomic RED/GREEN commits from `aa9c894`
  through `cd4368a`: worker/App, transfer, delete, single rename, and bulk
  rename each have an observed RED then minimal GREEN. One latest-value
  same-generation worker slot coalesces updates; started count is monotonic and
  bounded; App projects Pending items to Running before exact completion.
- The first full suite exposed an unrelated OMP fixture mixing real and
  synthetic `Instant` values. Separate test-only `30d99bd` moved its complete
  lifecycle to one explicit monotonic clock; exact and 33-test family probes
  plus the second full suite passed.
- C4.4 progress gates: focused C4 operations 57/57, full nextest 3115/3115 plus
  only `path_beta_real_host_probe` ignored, Linux/Windows clippy, Bun 17/17,
  Python 64/64, fmt/diff/temp clean. Fresh graph: 18,745 / 87,178 with the
  progress type, common worker seam, four observer adapters, and
  `miller_layout` after the stale `ready` graph was disproven.
- Completed TP-C4.4-CANCEL as fourteen atomic RED/GREEN commits from `29572ab`
  through `d77858a`. Transfer rollback remains protected; delete checks before
  irreversible mutation; single/bulk rename prioritize already-observed cancel
  over later revalidation races. Repeated Esc routes only to the matching
  active generation and keeps FM open; buffered completion rejects late cancel.
- C4.4 cancellation gates: broad C4/input 98/98, full nextest 3122/3122 plus
  only `path_beta_real_host_probe` ignored, Linux/Windows clippy, Bun 17/17,
  Python 64/64, fmt/diff/temp clean. Fresh graph: 18,756 / 87,282 with typed
  input, App/worker cancel seams, exact tests, and `miller_layout` after stale
  `ready` was disproven.
- Completed TP-C4.4-RECONCILE as nine atomic test/product commits from
  `0b04e73` through `d1a2d2e`. Queued, watcher-first, and delayed own-operation
  events share one active watcher generation/revision/path owner; external
  paths remain immediately visible, polling fallback stays single-lane, and a
  stale completion cannot reload a same-cwd reopened FM.
- C4.4 reconciliation gates: broad C4/FM 126/126, full nextest 3128/3128 plus
  only `path_beta_real_host_probe` ignored, Linux/canonical Windows clippy, Bun
  17/17, Python 64/64, fmt/diff and operation/staging artifact checks clean.
  Fresh graph: 18,786 / 87,697 with the production ownership seam, delayed and
  same-cwd lifecycle tests, and `miller_layout` after stale `ready` proof.
- Completed TP-C4.4-RECOVERY as seven atomic commits from `0881976` through
  `c674296`. Disconnect-after-progress was observed RED before
  `new_after_generation` recovery: every remaining item terminalizes, runtime
  reconciliation ownership clears, the dead channel is replaced at its prior
  generation floor, and the next sync does not hot retry. Progress-then-panic,
  cancel-to-next-generation, stale cancel rejection, uncertain private staging
  evidence, and lane reuse are covered at App level without a second scheduler.
- C4.4 closure gates: focused recovery 46/46, C4 core 67/67, broad C4/FM
  218/218, final full nextest 3131/3131 plus only
  `path_beta_real_host_probe` skipped, Linux/canonical Windows clippy, Bun
  17/17, Python 64/64, fmt/diff and operation/staging artifact checks clean.
  Fresh graph: 18,793 / 87,788 with `new_after_generation`, exact recovery
  tests, and `miller_layout` after stale `ready` proof.
- Completed C5 existing-agent and non-agent handoff through product head
  `f744e4d`, then published the continuity checkpoint `f23dbc7` to both CyPack
  heads only. Existing agents receive one exact literal UTF-8 path plus Enter;
  non-agent sources use one direct-argv `claude` Down split with exact owned-
  resource rollback on every failure path.
- Completed C6.1 as durable plan `6464668`, RED contracts `4a65c15`, `4836b32`,
  `1236f57`, and GREEN `2bcdf14`. The new client-local module prepares a
  bounded 256-item FAVORITES/optional PINNED/LOCATIONS model outside render,
  deduplicates exact path authority, and keeps inaccessible pins visible but
  inert. `compute_view` owns item-only hit areas; input only replaces one typed
  path; the scheduled App boundary revalidates Files tab/model/live directory
  before opening exact `FmState` and rebinding the existing watcher. Tab change
  and FM open/close clear stale intent.
- C6.1 gates: exact 9/9; sidebar/FM 239/239 (run
  `d7202d9b-ffbc-409d-82f8-76ec191429d3`); full nextest 3151/3151 plus only
  `path_beta_real_host_probe` skipped (run
  `c5232427-adc0-49b9-9898-daf479b623cd`); Linux/Windows clippy, Bun 17/17,
  Python 64/64, fmt/diff/production-unwrap/temp clean. Fresh graph: 18,899 /
  90,094 with `miller_layout` and all new sidebar symbols.
- Completed C6.2 as durable plan `c3dfa6f`, RED contract `ac4eecb`, GREEN
  product `b88fc12`, and test-only lifecycle closure `a078d98`. Current
  authority is derived every frame from exact open
  `FmState.cwd` plus a prepared accessible row. The responsive accent pill is
  complete-or-omitted; Unicode truncation is display-cell safe; inaccessible
  warning overrides accessible eject, and every marker stays in the final row
  cell. Render remains pure and owns no filesystem/runtime mutation.
- C6.2 gates: focused sidebar/FM groups 11/11, 60/60, and 56/56; full nextest
  3154/3154 plus only `path_beta_real_host_probe` skipped (run
  `3ffc29fb-d053-4a6c-bbda-86b63745fc64`); Linux/Windows clippy, Bun 17/17,
  Python 64/64, fmt/diff/production-unwrap/artifact checks clean. Fresh graph:
  18,909 / 90,194 with `miller_layout` and the three C6.2 helper symbols. The
  known MCP extraction failure recovered via `CBM_WORKERS=1` CLI with zero
  extraction errors and no service/process restart.
- Completed C6.3 as matrix `2648a08`, RED contracts `a12a870`, `9aad978`,
  `ab27caa`, and `0905e49`, product commits `40c7ab9`, `dd00f25`, `e7614aa`,
  and `8b21442`, plus test closure `2d974da`. Unsupported New Folder/Compress
  are explicitly disabled; header, row, context, Open, and plugin paths
  converge on current typed and scheduled authority. Stale, reordered,
  unsupported, in-flight, popup-close, FM-close/reopen, and manifest-drift
  cases consume fail-closed without duplicate execution or focus corruption.
- C6.3 gates: focused 118/118 (run
  `41e5dbf8-576c-4e6b-a7eb-eedd9897121b`); full nextest 3160/3160 plus only
  `path_beta_real_host_probe` skipped (run
  `ec91fccd-12fc-49b9-ae92-0d464de19552`); Linux/Windows clippy, Bun 17/17,
  Python 64/64, fmt/diff/production-unwrap/temp clean. Fresh graph: 18,922 /
  89,277 with current `miller_layout`, row dispatcher, Open scheduler, and
  plugin scheduler snippets.

## 6. KOD DURUMU

Previously published product/test history through `bcba84d`:

- `c043c1e`: `src/ui/shell.rs`, `src/ui.rs`, `src/app/state.rs`, `src/app/mod.rs`.
- `d026e94`: `src/ui/file_manager.rs`, `src/ui.rs`, `src/app/state.rs`, `src/app/mod.rs`, `src/main.rs`.
- `74d3cc9`: `src/app/actions.rs`, `src/app/input/navigate.rs`, `src/config/keybinds.rs`, `src/config/model.rs`, `src/ui/keybind_help.rs`.
- `d2b27e6`: `src/app/input/file_manager.rs`, `src/app/input/mod.rs`, `src/fm/mod.rs`, `src/main.rs`.
- `6c7c58f`: `src/fm/mod.rs`, `src/ui/file_manager.rs`.
- `01ba91d`: `Cargo.toml`, `Cargo.lock`,
  `src/app/file_manager_watcher.rs`, `src/app/mod.rs`, `src/app/runtime.rs`,
  `src/fm/watcher.rs`, `src/fm/mod.rs`.
- `8cd4e89`: `src/server/headless.rs`, `src/terminal/state.rs`.
- `bcba84d`: `src/kitty_graphics.rs`.

B1 is an auditable 21-commit test/feature sequence from `439ff2c` through
`2b2dcd3`. Product paths are `Cargo.toml`, `Cargo.lock`,
`src/app/file_preview_worker.rs`, `src/app/mod.rs`, `src/app/runtime.rs`,
`src/fm/mod.rs`, `src/fm/text_preview.rs`, and `src/ui/file_manager.rs`.
Intermediate RED commits are intentional TDD checkpoints; the range tip is
fully green. Continuity/task commit `a0f82a3` and the complete B1 range were
fast-forward published to both CyPack `feat/native-fm` and fork `master`; exact
remote SHAs matched. Upstream was not pushed.

A3 is an auditable seven-commit sequence:

- `d713b71` / `027c364`: viewport RED/GREEN.
- `1fea0e7` / `6727342`: shared hit-geometry RED/GREEN.
- `3aa267b` / `33b585a`: runtime mouse dispatch RED/GREEN.
- `9d69c82`: cursor-only v1 selection-scope characterization.

All seven commits are published to both CyPack `feat/native-fm` and fork
`master`; upstream was not pushed.

B2 product/test history is an auditable sequence:

- `de1eff5`: dependency decision/evidence.
- `be200b9` / `e24cda8`: bounded decoder RED/GREEN.
- `054708f` / `983b6b3`: client-local placement RED/GREEN.
- `cf40a06` / `11f26eb`: generation-safe worker lifecycle RED/GREEN.
- `5c51f1a` / `c6b4762`: cached paint/cleanup RED/GREEN.
- `2989434`: width-safe fallback characterization and fix.

The range is fully green. The continuity/graph commits containing this handoff
are part of the publication unit; both CyPack heads are verified at that same
fast-forward branch tip. Upstream is not pushed.

C1.1 history is an auditable test/product pair plus an independent stability
fix:

- `0ed5e51`: compile-failing RED header geometry/ViewState contract.
- `9aa1e59`: deterministic process-generation suppression fixture.
- `c9bfbf9`: responsive header action geometry and pure render/ViewState seam.

The intermediate RED commit was never pushed alone. The publication unit adds
this continuity/graph commit and fast-forwards both CyPack heads only after all
fresh gates pass. Upstream is not pushed.

C1.2 is an auditable RED/GREEN pair:

- `dbc6798`: compile-failing exact-tag and fail-closed dispatch contract.
- `7fd01de`: private side-effect-free header action tag dispatch.

The intermediate RED commit was never pushed alone. This continuity/graph
commit completes the C1.2 publication unit.

N3.1 is an auditable RED/GREEN pair:

- `b5cc95c`: compile-failing selection/clipboard/lifecycle/render contract.
- `510eebc`: pure selection-sensitive persistent action-bar content.

The intermediate RED commit was never pushed alone. This continuity/graph
commit completes the N3.1 publication unit.

N3.2 is an auditable RED/GREEN pair:

- `446613a`: compile-failing explicit action-authority contract.
- `267ad91`: prepared fail-closed authority, disabled render, and input gate.

The intermediate RED commit was never pushed alone. This continuity/graph
commit completes the N3.2 publication unit.

C2.1 is an auditable RED/GREEN pair:

- `d4d404e`: compile-failing row action geometry/render/ViewState contract.
- `9a15328`: pure responsive row action geometry and ViewState/render seam.

The intermediate RED commit was never pushed alone. This continuity/graph
commit completes the C2.1 publication unit.

C2.2 is an auditable RED/GREEN pair:

- `94e4a02`: compile-failing stable-path row dispatch contract.
- `9ef90c6`: exact fail-closed path-and-tag dispatch before terminal input.

The intermediate RED commit was never pushed alone. This continuity/graph
commit completes the C2.2 publication unit.

N4.1 is an auditable seven-commit sequence:

- `e876223` / `590e376`: selection state RED/GREEN.
- `1789bbd` / `5c14439`: lifecycle reconcile RED/GREEN.
- `699a6a6`: gesture and render RED.
- `fc19237`: stable row identity RED.
- `86b618a`: exact input, stable identity, and pure visual projection GREEN.

The compile-failing RED checkpoints were never pushed alone. This continuity/
graph commit completes the N4.1 publication unit.

C3.2 is an auditable six-commit sequence:

- `69864d6` / `ad5f8a5`: exact path-stable popup geometry RED/GREEN.
- `73df647` / `45c151f`: keyboard/mouse lifecycle and typed intent RED/GREEN.
- `1078215` / `0915964`: disabled/highlight-safe render RED/GREEN.

No RED checkpoint is published alone. This continuity/graph commit completes
the C4.1 publication unit before both CyPack heads are fast-forwarded.

C4.4 terminal recovery is an auditable seven-commit sequence:

- `0881976` / `7847a6c`: disconnected worker lane RED/GREEN.
- `8974f4c`: progress-then-panic terminalization and lane reuse coverage.
- `bcc9ef5`: cancellation-to-next-generation and stale-cancel coverage.
- `7e2af79`: uncertain private bulk-recovery path evidence and lane reuse.
- `03b9395`: real baseline cleanup plus no-hot-retry idempotence coverage.
- `c674296`: test-fixture Clippy closure after the warning-as-error gate found
  `field_reassign_with_default`.

The only production change is in `src/app/file_operation_worker.rs`: a dead
channel is replaced through the existing single worker constructor while
preserving the generation floor. No second scheduler, server field, protocol,
render mutation, dependency, or public docs surface was added. This continuity
commit closes the C4.4 publication unit before both CyPack heads are
fast-forwarded; no RED checkpoint is pushed alone.

C6.1 is an auditable plan/RED/GREEN sequence:

- `6464668`: durable sidebar test points and ordered C6.1–C6.4 decomposition.
- `4a65c15`: model/geometry/render/navigation RED contract.
- `4836b32`: live discovery and bounded-source RED contract.
- `1236f57`: stale-hit and hidden-Spaces input isolation RED contract.
- `2bcdf14`: sectioned model, pure geometry/render, typed input intent,
  lifecycle invalidation, scheduled revalidation, and watcher convergence.
- `c3dfa6f`: durable C6.2 current/pill/marker/lifecycle/gate test points.
- `ac4eecb`: C6.2 exact-authority, responsive Unicode pill, and marker RED
  contracts.
- `b88fc12`: pure current-location derivation plus bounded pill and
  warning/eject rendering.
- `a078d98`: explicit non-Files and close/reopen lifecycle coverage.

No RED checkpoint is published alone. Product/test paths are
`src/app/file_manager_sidebar.rs`, App state/runtime/actions/watcher/input,
desktop/mobile view computation, and pure sidebar render. This continuity/
graph commit closes C6.2 before both CyPack heads are fast-forwarded; upstream
is never pushed.

C6.4 is an auditable plan/RED/GREEN sequence:

- `5b8f327`: durable semantic roles, state precedence, and visual gate plan.
- `2362751` / `3e73351`: semantic palette and canvas RED/GREEN.
- `4ed210e` / `37f760d`: typed cwd availability RED/GREEN.
- `04b8070` / `792c4d8`: operation/recovery status RED/GREEN.
- `3f9a0cd` / `101809c`: preview warning/error RED/GREEN.
- `03aeb6d` / `f52cb85`: full-frame composition and image-target closure.

No RED checkpoint is published alone. Render remains pure; filesystem-derived
status is prepared only during FM refresh. This continuity/graph commit closes
C6.4 and the v1 A-C visual gate before both CyPack heads are fast-forwarded;
upstream is never pushed.

## 7. TEST KANITI

- B1/FM targeted: 64/64.
- Final full nextest: 2948/2948 passed, one explicit B0 interactive host probe
  skipped, no retry.
- Linux all-target and canonical Windows MSVC binary-target clippy passed with
  `-D warnings`.
- Bun 17/17; Python maintenance 64/64; fmt and diff-check clean.
- Doctests are N/A because Herdr has no library target.
- C6.1 focused model/geometry/render/input/navigation/lifecycle: 9/9.
- Broad sidebar/native-FM nextest: 239/239, run
  `d7202d9b-ffbc-409d-82f8-76ec191429d3`.
- Final C6.1 full nextest: 3151/3151, one named B0 real-host probe skipped,
  run `c5232427-adc0-49b9-9898-daf479b623cd`; no retry.
- C6.1 exact-head Linux all-target and canonical Windows MSVC bin clippy,
  Bun 17/17, Python 64/64, fmt/diff/production-unwrap/temp-artifact checks are
  clean. `just` remains absent; the complete recipe was executed directly.
- C6.2 focused sidebar/FM groups: 11/11, 60/60, and 56/56. Final full nextest:
  3154/3154 plus one named B0 real-host probe skipped, run
  `3ffc29fb-d053-4a6c-bbda-86b63745fc64`; no retry-only closure.
- C6.2 exact-head Linux all-target and canonical Windows MSVC bin clippy,
  Bun 17/17, Python 64/64, fmt/diff/production-unwrap/artifact checks are clean.
  `just` remains absent; the complete recipe was executed directly.
- C6.3 focused 118/118; full nextest 3160/3160 plus one named B0 probe skip;
  Linux/Windows clippy, Bun 17/17, Python 64/64, fmt/diff/production-unwrap/
  artifact checks clean.
- C6.4 focused semantic/directory/status/preview/composition sets passed;
  broad FM/Kitty and UI composition sets passed. Final full nextest 3171/3171
  plus only `path_beta_real_host_probe` skipped, run
  `339242c5-a4d2-4989-9583-8e904c6d7b1e`; no retry-only closure.
- C6.4 Linux all-target and canonical Windows MSVC bin clippy, Bun 17/17,
  Python 64/64, fmt/diff/production-unwrap/artifact checks are clean. Isolated
  headless API and 120x30 real PTY checks used throwaway XDG/socket state,
  exited semantically, and left zero process/socket/temp residue.
- Actual B1 lock delta is five packages with no existing-version upgrade.
  Exact OSV batch returned only severity-less `RUSTSEC-2025-0141` for
  unmaintained `bincode 1.3.3`; no security-severity advisory.
- A3 targeted broad regression: 164/164; scope: 4/4.
- Final A3 full nextest: 2966/2966, one named B0 host probe skipped, no retry.
- Linux/Windows clippy, Bun 17/17, Python 64/64, fmt/diff clean.
- Isolated `--no-session` PTY proved three columns, cursor click, directory
  double-click enter, long-list wheel down/up viewport clamp, semantic exit 0,
  and zero temp/process residue.
- B2/FM/Kitty targeted: 96/96; full nextest: 2983/2983 plus one named B0
  interactive probe skip; no retry.
- B2 Linux all-target and canonical Windows MSVC bin clippy passed with
  `-D warnings`; Bun 17/17; Python 64/64; fmt/diff clean.
- B2 isolated Kitty source-to-host comparison: 0/271425 pixel difference.
  Closing FM removed the image from the host, and semantic exit left zero
  test process/socket/temp residue.
- C1.1 geometry/render/ViewState targeted: 4/4; suppression/process-exit/stale
  lifecycle family: 27/27; final full nextest: 2986/2986 plus one named B0
  interactive probe skip, no retry-only closure.
- C1.1 Linux all-target and canonical Windows MSVC bin clippy passed with
  `-D warnings`; Bun 17/17; Python maintenance 64/64; fmt/diff clean.
- C1.2 exact dispatch: 2/2; full FM input: 13/13; final full nextest:
  2988/2988 plus one named B0 interactive probe skip, no retry-only closure.
- C1.2 Linux all-target and canonical Windows MSVC bin clippy passed with
  `-D warnings`; Bun 17/17; Python maintenance 64/64; fmt/diff clean.
- N3.1 targeted: 3/3; FM regression: 135/135; final full nextest: 2991/2991
  plus one named B0 interactive probe skip, no retry-only closure.
- N3.1 Linux all-target and canonical Windows MSVC bin clippy passed with
  `-D warnings`; Bun 17/17; Python maintenance 64/64; fmt/diff clean.
- N3.2 exact authority/preparation/render/dispatch: 7/7; broad FM/input/render/
  Kitty regression: 165/165; final full nextest: 2996/2996 plus one named B0
  interactive probe skip, no retry-only closure.
- N3.2 Linux all-target and canonical Windows MSVC bin clippy passed with
  `-D warnings`; Bun 17/17; Python maintenance 64/64; fmt/diff clean.
- C2.1 focused invariant/readability set: 8/8; FM impact: 71/71; final full
  nextest: 2998/2998 plus one named B0 interactive probe skip, no retry-only
  closure. Linux/Windows clippy, Bun 17/17, Python 64/64, fmt/diff clean.
- C2.2 exact dispatch/stale/no-side-effect: 3/3; all FM input: 17/17; FM impact:
  74/74; final full nextest: 3001/3001 plus one named B0 interactive probe
  skip, no retry-only closure. Linux/Windows clippy, Bun 17/17, Python 64/64,
  fmt/diff clean.
- N4.1 focused input/render: 7/7; broad FM/watcher/input/render/Kitty: 137/137;
  final full nextest 3015/3015 plus one named B0 interactive probe skip, no
  retry-only closure. Linux/Windows clippy, Bun 17/17, Python 64/64, fmt/diff
  clean.
- C3.2 focused popup 4/4, lifecycle 3/3, disabled render 1/1; broad
  FM/global-menu 51/51 and menu/render 26/26. Final full nextest 3033/3033
  plus one named B0 host-probe skip; Linux all-target and canonical Windows
  MSVC bin clippy, Bun 17/17, Python 64/64, fmt/diff clean.
- C3.3 focused 8/8; plugin/context 35/35; FM/watcher/global-menu 112/112;
  final full nextest 3041/3041 plus only the named B0 host probe skip. Linux
  all-target and canonical Windows MSVC bin clippy, Bun 17/17, Python 64/64,
  generated schema, fmt, and diff-check are clean.
- C4.1 operation core 15/15, App/worker 8/8, broad FM/watcher/preview 147/147;
  final full nextest 3064/3064 plus only the named B0 host probe skip. Linux
  all-target and canonical Windows MSVC bin clippy, Bun 17/17, Python 64/64,
  fmt/diff/temp checks, and graph freshness are clean.
- C4.2 focused delete 29/29 and broad FM/watcher/preview/context/plugin
  321/321; final full nextest 3086/3086 plus only the named B0 host probe skip.
  Linux/Windows clippy, Bun 17/17, Python 64/64, isolated real Trash,
  fmt/diff/temp checks, and graph freshness are clean.
- C4.3 focused/broad rename, bulk, worker, App, and watcher regression 163/163;
  final full nextest 3109/3109 plus only the named B0 host probe ignored.
  Linux/Windows clippy, Bun 17/17, Python 64/64, real temporary-filesystem
  rename/recovery coverage, fmt/diff/temp checks, and graph freshness are
  clean.
- C4.4 terminal recovery focused 46/46, C4 core 67/67, broad C4/FM 218/218;
  final full nextest 3131/3131 plus only `path_beta_real_host_probe` skipped.
  Linux all-target and canonical Windows MSVC bin clippy passed with
  `-D warnings`; Bun 17/17; Python 64/64; ignored-only inventory, fmt, diff,
  operation/staging artifact scan, and graph freshness are clean. The final
  graph is 18,793 nodes / 87,788 edges and returns the production recovery
  seam, exact failure tests, and `miller_layout`.

## 8. KRİTİK KARARLAR

- Pure render is non-negotiable.
- S3 registry deferred to S5; use concrete content swap until abstraction is earned.
- S2 persistence deferred to S6.
- A2.2 caches parent/preview in `FmState`.
- A4 uses stable `notify-debouncer-full 0.7.0` / `notify 8.2.0`.
- Native watching is primary; startup/runtime errors enter explicit polling
  fallback, and all active watchers reconcile every 2 seconds to cover silent
  FUSE/NFS/exFAT-class delivery failures.
- A4, B0, B1, A3, and B2 are implementation-complete, fully verified,
  graph-indexed, and published to the CyPack fork.
- C1, N3, and C2 are implementation-complete, fully verified, and
  graph-indexed.
  N3.2 supplies explicit selection/clipboard/target/in-flight authority and
  disabled-click no-side-effect behavior. C4 must still revalidate every real
  filesystem operation at execution time for TOCTOU and partial failure.
- N4.2 is implementation-complete, fully verified, and graph-indexed. Its
  prepared path vector and action states are client-local presentation/input
  authority only; C3 may consume them for menu modeling, while C4 must still
  revalidate every target at execution time.
- C3.1 is implementation-complete, fully verified, and graph-indexed. It
  models context intent only and deliberately leaves popup routing/render/
  disabled dispatch to C3.2 and plugin extension to C3.3.
- C3.2 is implementation-complete, fully verified, and graph-indexed. The
  popup reuses the global modal stack, disabled actions are inert, and emitted
  typed intents are revalidated against current prepared path/order authority.
  C4/C5 remain the sole owners of real side effects.
- B1 uses minimal pure-Rust syntect outside input/render in a dedicated bounded
  worker. Plain prepared content remains availability authority; highlighting
  is optional enhancement and stale generations never mutate current state.
- B0's conditional B2 GO is satisfied: B2 retains bounded decode, generation,
  cleanup, and real-host evidence constraints.
- The user granted standing authorization for autonomous atomic commits and
  CyPack fork-only fast-forward pushes. Do not repeatedly ask for alignment;
  never relax targeted staging, verification, ancestry, or remote-SHA checks.
- C6.4 and the v1 A-C visual gate are implementation-complete, fully verified,
  and graph-indexed. P4.0 is read-only evidence gathering; it may activate at
  most one deferred architecture candidate and cannot itself justify product
  refactoring.
- At their historical checkpoint, P4.0 and N2.0 were complete and S5/S6/S7 plus
  the original dynamic/unbounded N2 state machine were implementation NO-GO.
  Pinned Yazi/Joshuto source proved one
  narrow missing behavior: leave should focus the exact child just exited.
  N2.1 receives implementation GO under zero-new-state/no-extra-read budgets;
  N2.2 retained history and parent-column sibling navigation remained deferred
  at that checkpoint; the later bounded SF/FM program activation is recorded
  at the top of this handoff.
- N2.1 is implementation-complete as RED `e433a2f` and GREEN `c530836`.
  Exact 6/6, FM 65/65, full nextest 3177/3177 plus one named ignored host probe,
  Linux/Windows clippy, Bun 17/17, Python 64/64, and fresh graph 18,997 / 89,826
  are clean. Ordinary reload behavior is preserved through the shared helper;
  leave holds only one local departed path and performs no additional read.
- M3.0–M3.3 are documentation-only implementation NO-GO. Graph-first evidence
  found two real action consumers but no shared page/component lifecycle:
  M1 owns `AttachFile` picker state and delivery cleanup; M2 owns cached Git/
  workspace authority and an existing-dialog request. Fresh characterization
  is 16/16, run `32ca7f37-b65c-45ef-9dbf-548e8263d383`; graph remains current
  at 19,534 / 91,017. Full evidence is
  `.codex/evidence/m3-general-ui-interface.md`.

## 9. GÜVENLİK

- Never kill user processes.
- Never touch `/home/ayaz/.local/bin/herdr` or the stable socket.
- Clear inherited socket variables and use throwaway XDG directories for runtime tests.
- Never stage ignored `.local` files into product commits.
- Never push `upstream`.
- C3.3 extends the neutral public JSON plugin context, not private TUI
  transport frames; `PROTOCOL_VERSION` remains 16 per `v0.7.3` comparison and
  repository precedent. C4 owns every filesystem side effect.

## 10. AÇIK GÖREVLER

See `.codex/TASKS.md` first for the active SF0-SF6 plus FM1-FM5 task list and
then the completed A3/B2/C1/N3/C2/N4 contracts and historical C3–C6, S5–S7,
N2, and M1–M3 roadmap. SF0-SF3 are closed through the published SF3.3 product
checkpoint `90be6893`; SF4.1 graph/drift analysis and the compile-valid
`stage_starts_on_terminal_workspace` RED are the next executable microtask.
A4, B0,
B1, A3, B2, C1, N3,
C2, N4, C3.1, C3.2, C3.3, C4.1, C4.2, C4.3, C4.4.1 PROGRESS, C4.4.2
CANCEL, C4.4.3 RECONCILE, C4.4.4 RECOVERY, C4.4.5 GATES, C5.1–C5.5, and
C6.1–C6.4 are closed. C5's atomic product chain ends at `f744e4d`: exact existing-agent send
and non-agent `Down` split → direct `claude` argv → first literal path send,
with exact new-resource rollback on every failure path. The complete gate is
3143/3143 Rust, Bun 17/17, Python 64/64, Linux/Windows clippy, and fresh graph
18,854 / 88,064. C6.1 then closes at product head `2bcdf14`, full Rust
3151/3151 plus one named skip, and fresh graph 18,899 / 90,094. C6.2 closes at product head
`b88fc12` plus test closure `a078d98`, full Rust 3154/3154 plus one named skip,
and fresh graph 18,909 / 90,194. C6.3 closes at product head `8b21442` plus
test closure `2d974da`, full Rust 3160/3160 plus one named skip, and fresh
graph 18,922 / 89,277. C6.4 closes at test/product head `f52cb85`, full Rust
3171/3171 plus one named skip, isolated API/PTY residue zero, and fresh graph
18,974 / 89,775. P4.0 closes as a documentation-only evidence matrix with
S5–S7 implementation NO-GO. N2.0 then rejects the original dynamic/unbounded
state machine and N2.1 closes as RED `e433a2f` → GREEN `c530836` with complete
gates. M1–M3 have durable macro/micro and failure-path test contracts.
M1.0–M1.4 are closed through `7d3144e`, exact attachment 20/20, full nextest
3197/3197 plus the named B0 skip, Linux/Windows clippy, Bun 17/17, Python
64/64, and fresh graph 19,113 / 91,118. M2 then closes as RED `dab1e20` →
GREEN `0ae6175`: exact M2.1 5/5, worktree/attachment 131/131, full 3202/3202
plus the named skip, Linux/Windows clippy, Bun 17/17, Python 64/64, and fresh
graph 19,534 / 91,017. Duplicate management implementations remain NO-GO.
M3.0–M3.3 close implementation NO-GO with a fresh 16/16 characterization set
and no product diff. At that checkpoint N2.2 and S5–S7 were independently
deferred with explicit activation criteria. The later approved product lane
activates only bounded S6/N2.2-derived work; S5 registry and S7 popup stack
remain deferred.

## 11. ORTAM

- `codex-cli 0.144.1` is installed.
- `just` is absent; direct recipe execution is required unless installed later.
- Full post-N3.1 graph reindex completed at 18,009 nodes / 83,964 edges.
  Freshness query returned current action-bar model/selection/kind types and
  `compute_file_manager_action_bar_model`; the builder is connected to
  desktop/mobile view computation, render fallback, and model tests. Lifecycle
  and render tests are present. Freshness was not inferred from `ready` alone.
- Full post-N3.2 graph reindex completed at 18,026 nodes / 84,120 edges.
  Freshness queries returned current `miller_layout`, action state, authority
  builder, entry-capability preparation, and mouse handler symbols. The pure
  builder is connected to desktop/mobile view computation, render fallback,
  and authority tests. Freshness was not inferred from `ready` alone.
- Fast post-C2.1 graph reindex completed at 18,042 nodes / 84,123 edges.
  Freshness queries returned `compute_file_manager_row_geometry`,
  `FileManagerRowAction`, and `FileManagerRowActionArea` from their current
  source files. Freshness was not inferred from `ready` alone.
- Fast post-C2.2 graph reindex completed at 18,049 nodes / 83,839 edges.
  Freshness queries and snippets returned the current path-bearing
  `FileManagerRowActionArea` and live path/support validation in
  `handle_file_manager_mouse`. Freshness was not inferred from `ready` alone.
- Fast post-N4.1 graph reindex completed at 18,078 nodes / 83,865 edges.
  Freshness queries returned `replace_selection`, `toggle_selection`,
  `extend_selection`, `reconcile_multi_selection`, stable row identity input,
  and the multi-selection render test with live connections. Freshness was not
  inferred from `ready` alone.
- Fast post-N4.2 graph reindex completed at 18,091 nodes / 84,102 edges.
  Freshness queries and source snippets returned current `select_all`,
  `MAX_MULTI_SELECTION_PATHS`, `compute_file_manager_action_bar_model`, and
  `handle_file_manager_key`, including the 4,096 ceiling and atomic Shift
  routing. Freshness was not inferred from `ready` alone.
- Fast post-C3.1 graph reindex completed at 18,115 nodes / 84,003 edges.
  Freshness queries and source snippets returned current
  `FileManagerContextMenuModel::from_action_bar` precedence/item mapping and
  `ContextMenuKind::File`. Freshness was not inferred from `ready` alone.
- Post-C3.2 graph refresh completed at 18,139 nodes / 86,595 edges through
  supported single-worker CLI fallback after the proxy-owned child crashed in
  parallel native extraction. CLI status, graph search, and source snippet
  returned `validated_file_context_action`, right-click selection tests, and
  disabled-render tests. No process/service was restarted or killed, and
  freshness was not inferred from `ready` alone.
- Post-C3.3 sequential graph refresh completed at 18,246 nodes / 85,535 edges
  with zero extraction errors. CLI status/search/snippet proof returned
  `file_manifest_actions`, `plugin_invocation_params`, Unicode popup geometry,
  and the end-to-end plugin disable-race test; `ready` alone was not accepted.
- Post-C4.3 fast refresh completed at 18,722 nodes / 88,526 edges. The stale
  pre-refresh graph said `ready` but returned none of the new rename symbols;
  after indexing, searches returned `miller_layout`, `RenameOperationPlan`,
  `BulkRenameOperationPlan`, `validate_rename_name_component`, and
  `consume_file_manager_bulk_rename_request` from current source.
- Post-C4.4.1 fast refresh completed at 18,745 nodes / 87,178 edges. The stale
  pre-refresh graph said `ready` but returned no `FileOperationWorkerProgress`;
  after indexing it returned the progress type,
  `execute_worker_task_with_progress`, all four operation observer adapters,
  and the prior `miller_layout` symbol.
- Post-C4.4.2 fast refresh completed at 18,756 nodes / 87,282 edges. The stale
  graph said `ready` but lacked `cancel_file_manager_operation`; after indexing
  it returned `FileManagerKeyDispatch`, the App cancel seam, generation-safe
  worker cancel, exact cancellation tests, and `miller_layout`.
- Post-C4.4.3 fast refresh completed at 18,786 nodes / 87,697 edges. The stale
  graph said `ready` but lacked `own_operation_reconcile`; after indexing it
  returned the exact ownership seam, delayed/same-cwd lifecycle tests, and
  `miller_layout`.
- Post-C4.4.4 fast refresh completed at 18,793 nodes / 87,788 edges. The stale
  graph said `ready` but returned only `miller_layout`, not the new recovery
  symbols; after indexing it returned `new_after_generation`, the disconnect,
  panic, and private-recovery tests, plus `miller_layout`.
- Post-C5.4 fast refresh completed at 18,854 nodes / 88,064 edges. Status was
  cross-checked with `miller_layout` and nine current split/ownership/rollback
  symbols including `sync_file_manager_claude_split`,
  `launch_file_manager_claude_split`, and
  `complete_file_manager_claude_split`; `ready` alone was not accepted.
- Post-C6.1 moderate refresh completed at 18,899 nodes / 90,094 edges. Status
  was cross-checked with `miller_layout`, `FileManagerSidebarModel`,
  `compute_file_manager_sidebar_row_areas`, `file_manager_sidebar_path_at`,
  and `sync_file_manager_sidebar_navigation`; `ready` alone was not accepted.
- Post-C6.4 moderate refresh completed at 18,974 nodes / 89,775 edges. Status
  was cross-checked with current `miller_layout`, `FmDirectoryStatus`,
  `file_manager_visual_styles`, and `file_manager_status_line` source snippets;
  `ready` alone was not accepted.
- `mcp-proxy.service` cold start measured 54 seconds for 26 servers. Readiness
  now has a 120-second internal and 150-second systemd budget; live proof was
  `expected=26 observed=26 critical_tools=14`.
- `~/.codex/skills/rust-dev` points to the canonical Claude `rust-dev` skill; parity reports no broken skill links.
- Global Codex hooks support SessionStart and UserPromptSubmit; Herdr context routing is scoped to this repo.

## 12. BAŞLATMA

Run:

```bash
herdr-codex
```

The new agent must read `AGENTS.md`, `CLAUDE.md`, `.codex/BOOTSTRAP.md`, `.codex/CURRENT.md`, and `.codex/TASKS.md`, then verify graph and Git state before acting.
