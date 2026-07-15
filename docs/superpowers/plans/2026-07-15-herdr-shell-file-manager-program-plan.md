# Herdr Shell Foundation and File Manager Program Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> `superpowers:executing-plans` to implement this program task-by-task. Steps
> use checkbox (`- [ ]`) syntax for tracking. Do not dispatch subagents unless
> the user explicitly authorizes delegation.

**Goal:** Deliver a bounded client-owned Shell Foundation, migrate Files into
the Workspace Stage as its first native app surface, and then finish the five
approved Miller-column UX increments without expanding into the later
Apps/Desktop program.

**Architecture:** Preserve the existing server-owned runtime, PTY, workspace,
tab, and pane authority. Add a deterministic TUI-side shell projection with
named outer regions, bounded split tracks, transactional interaction, typed
built-in surfaces, fail-closed input routing, and versioned presentation
persistence. Move the existing `FmState` and its workers behind a
`NativeFiles` stage surface before extending Miller navigation.

**Tech Stack:** Rust 2024, Ratatui, Crossterm, Serde/JSON, existing Herdr
render-prof instrumentation, Cargo Nextest, Python and Bun maintenance tests,
Codebase Memory MCP, Git. No new dependency or wire protocol is planned.

## Global Constraints

- The approved scope is exactly seven Foundation phases, SF0-SF6, followed by
  five FM-next phases. Apps/Desktop expansion is a later independent program.
- SF0 is documentation and baseline freeze. Product mutation begins only at
  SF1 after I0-I5 are evidenced.
- Existing runtime/session facts remain server-owned. Shell geometry, focus,
  mouse capture, active native surface, viewport, and collapse state remain
  TUI/client presentation state.
- `render()` remains pure. Filesystem reads, watcher refresh, image decode,
  text preparation, persistence writes, and PTY resize never occur from
  render.
- No production code is written without a compile-valid behavior-specific
  failing test. Compile/setup/filter/fixture failures are not RED evidence.
- No production `unwrap()`. No new dependency without a separately evidenced
  need and user-approved scope.
- Stable Herdr processes and inherited sockets are never addressed. Manual
  testing uses `.local/ISOLATED-DEV-TEST.md` with throwaway XDG roots and all
  Herdr identity/socket variables cleared.
- Existing Files state, watcher, text/image preview workers, operations,
  selection authority, context menus, agent handoff, and failure paths are
  preserved during SF6; they are not reimplemented.
- Existing N2.1 path-stable leave behavior remains authoritative and is
  extended, not replaced, in FM-next 4.
- The shell tree remains finite: nesting depth at most 4, at most 8 children
  per split, at most 64 visible leaves, at most 128 serialized nodes, and at
  most 32 stack children.
- Shell layout compute remains O(node_count). No component owns an independent
  render loop and no layout graph is transmitted over the wire.
- Targeted staging only; never `git add -A`. Each behavior slice has atomic
  RED/GREEN/refactor commits. Only the CyPack fork may be pushed, only by
  fast-forward, never `upstream`, never force.
- The user-owned untracked `.superpowers/` directory is outside all file and
  commit boundaries.

---

## Approved Twelve-Phase Program

| Order | Phase | Deliverable | Product code? | Exit evidence |
|---:|---|---|---|---|
| 1 | SF0 | Approved design, baseline, bounds, file ownership, plans | No | A0-A7 ready; I0-I5 plan artifacts reviewed |
| 2 | SF1 | Characterization of legacy shell, Files swap, persistence, render hot paths | Tests only | Focused characterizations and full baseline green |
| 3 | SF2 | Named region model, bounded track solver, responsive shell view | Yes | Geometry, bounds, degradation, cache, and stale-hit tests green |
| 4 | SF3 | Resize, collapse, scroll, and layout persistence transactions | Yes | Preview/commit/cancel and migration/failure tests green |
| 5 | SF4 | SurfaceHost, focus scopes, capture, overlay-first input router | Yes | Routing matrix and background-blocking tests green |
| 6 | SF5 | Icon-only AppDock with typed built-in app definitions | Yes | Dock render/input/singleton/degradation tests green |
| 7 | SF6 | Files migrated from terminal curtain to `NativeFiles` Stage surface | Yes | Lifecycle, worker, overlay, hidden-terminal, and restore tests green |
| 8 | FM1 | Horizontally scrollable bounded Miller viewport | Yes | Depth, cache, viewport, shrink, close/reopen tests green |
| 9 | FM2 | Transactional Miller column-divider resize | Yes | Preview/commit/cancel/stale/resize/image-target tests green |
| 10 | FM3 | Stable mouse ownership in every rendered Miller column | Yes | Identity, selection, context, scroll, stale-generation tests green |
| 11 | FM4 | Finder-like path-stable growing and branching navigation | Yes | Restore, branch truncation, watcher deletion, bound tests green |
| 12 | FM5 | Evidence-backed preview/inspector placement decision | Decision first | Measured GO/NO-GO record; code only in a separately approved slice |

The exact Foundation implementation tasks are in
`docs/superpowers/plans/2026-07-15-herdr-shell-foundation-v0-implementation.md`.
The exact FM tasks are in
`docs/superpowers/plans/2026-07-15-herdr-file-manager-post-shell-implementation.md`.

## Frozen Phase Dependencies

```text
SF0
  -> SF1 characterization
  -> SF2 geometry
  -> SF3 interaction and persistence
  -> SF4 surface and input ownership
  -> SF5 AppDock consumer
  -> SF6 Files Stage migration
  -> FM1 horizontal viewport
  -> FM2 Miller divider resize
  -> FM3 all-column mouse ownership
  -> FM4 path-stable growing navigation
  -> FM5 preview/inspector decision
```

No phase may be skipped because later state and input contracts depend on the
previous phase. A phase may be reverted independently because its file and
commit boundary is frozen in the child plans.

## Current Codebase Evidence Boundary

The planning checkpoint is `aca2243` on `feat/native-fm`. Codebase Memory was
freshly queried at 19,534 nodes and 91,017 edges and proved current through
`home-ayaz-projects-herdr.src.ui.file_manager.miller_layout` at
`src/ui/file_manager.rs:122`.

| Current surface | Evidence | Program consequence |
|---|---|---|
| Shell model | `src/ui/shell.rs` has `TopBar`, `LeftPanel`, `CenterContent`, `RightPanel`, `BottomBar`; default is dynamic sidebar plus fill center | SF2 adds `AppDock` and `WorkspaceStage` while retaining a compatibility projection during migration |
| View projection | `src/ui.rs::compute_view_internal` computes shell, tab, FM, pane, split, and hit geometry in one function | SF2 extracts shell projection; SF4 separates active-surface projection and routing |
| Render | `BaseLayer::render` branches on `app.file_manager.is_some()` over `terminal_area` | SF6 replaces the curtain branch with a typed stage surface dispatch |
| Input | `handle_mouse_without_agent_frame_action` is overlay -> FM -> sidebar divider -> remaining routes | SF4 turns this precedence into explicit semantic routing and capture |
| FM mouse | `handle_file_manager_mouse` accepts only current `terminal_area` row snapshots | FM3 introduces column-scoped stable target identities |
| Persistence | `SNAPSHOT_VERSION` is 3 at HEAD and tag `v0.7.3`; only sidebar width/split shell facts exist | SF3 adds validated shell presentation snapshot and bumps to 4; wire protocol stays unchanged |
| Frame diff | `ClientRenderState::prepare_frame` returns `None` for an identical semantic or ANSI frame | SF1 characterizes and SF2-SF6 preserve zero outgoing identical frames |
| Retained PTY | `render_retained_pty_update_and_stream` patches dirty pane rows from the last frame | Static shell must not force full redraw for a terminal dirty row |
| Render queue | `ClientWriterQueue::try_send_render` has one optional render slot | Pending render count remains at most one |
| Existing N2.1 | `FmState::leave` calls `reload_focusing_path` and has reorder/delete/root/viewport tests | FM4 extends the chain without redoing the published behavior |

## Mandatory Test-Point Catalog

| ID | Test point | Expected result | Reason |
|---|---|---|---|
| TP-PROG-LEGACY | Existing desktop/sidebar/center and mobile geometry | Exact legacy rectangles and mobile behavior remain green through compatibility phases | Geometry refactor must not erase proven behavior |
| TP-PROG-GEOMETRY | Dock/sidebar/stage/right/top/bottom allocation | Every visible region is deterministic, non-negative, in bounds, and disjoint where required | This is the first new shell behavior |
| TP-PROG-EXHAUST | Terminal too small for preferred/min constraints | Frozen degradation order is applied and Stage is never collapsed | Tiny terminals are normal, not exceptional |
| TP-PROG-BOUNDS | Malicious/deep/wide/duplicate layout input | Validation rejects or falls back before recursion/allocation exceeds limits | Persisted state is untrusted input and complexity must stay finite |
| TP-PROG-DRAG | Divider preview, commit, cancel, terminal resize, stale hit | Preview writes no persistence and resizes no PTY; commit writes once and resizes at most once; cancel restores | Mouse motion must not create disk/network/PTY churn |
| TP-PROG-COLLAPSE | Collapse and restore after terminal/config changes | Prior valid width is restored through current min/max clamp; empty optional regions become zero | Predictable interaction and responsive safety |
| TP-PROG-SCROLL | Nested horizontal/vertical viewports | Only the topmost owning viewport changes and offsets clamp after content/area shrink | Prevent scroll leakage and stale offsets |
| TP-PROG-OVERLAY | Context menu/modal/popover over active Stage | Only the overlay receives actionable input; background surface remains unchanged | Fixes the reported curtain/input leak class |
| TP-PROG-SURFACE | Terminal <-> Files activation/close/failure | Exactly one active Stage surface; Files close/failure restores previous valid surface and focus | App lifecycle must be deterministic |
| TP-PROG-PTY | Resize preview and hidden terminal under Files | Zero preview PTY resize; commit at most one affected resize; hidden terminal process remains alive and inert | Preserve SSH latency and runtime authority |
| TP-PROG-RENDER | Identical frame, one dirty PTY row, queue full | No identical send; retained row remains partial; render queue remains bounded and retryable | Shell flexibility cannot regress transport performance |
| TP-PROG-PERSIST | v3 legacy, v4 valid, invalid v4, future version | v3 migrates, valid v4 restores, invalid shell falls back without losing unrelated session facts, future version rejects | Backward compatibility and corruption containment |
| TP-PROG-MILLER | Horizontal history, resize, all-column mouse, branching | All structures are bounded; stable path/generation is revalidated; stale targets are inert | Finder-like behavior must not weaken authority |
| TP-PROG-FAIL | I/O denial, watcher loss, worker panic/disconnect, invalid image/text | Existing explicit fallback and recovery behavior survives Stage/Miller changes | Happy-path-only migration is unacceptable |
| TP-PROG-PERF | Shell compute, full frame, outgoing bytes, resident Miller state | Budgets are measured in a named environment and regressions fail before publication | Performance claims require reproducible evidence |

## A0-A7 and I0-I14 Adoption

The approved design supplies A0-A7:

- A0: `mid_flight_adoption`; current branch, commits, tests, graph, and user
  changes are preserved.
- A1: Herdr desktop-like app workspace goal, user scenarios, and success
  criteria are frozen.
- A2: the twelve phases above are the approved fractal decomposition.
- A3: layout, input, lifecycle, persistence, runtime/PTY, failure, platform,
  and performance dimensions are represented in the design and test catalog.
- A4: bounded shell primitives, typed built-in surface host, and Files-first
  options were compared; arbitrary plugins/docking/editor were rejected.
- A5: fresh graph evidence maps every affected current symbol and hot path.
- A6: the target contract and non-goals are frozen in the design specification.
- A7: user approval makes the normalized handoff ready for the bounded scope.

Delivery gates apply per implementation phase:

| Gate | Required evidence before advancing |
|---|---|
| I0 | User-approved spec/plan, exact target phase, clean ownership boundary, fresh Git/graph status |
| I1 | Phase scenarios, authority, failure paths, non-goals, and rollback described in its task |
| I2 | Fresh graph query for symbols touched by that phase and drift comparison against this plan |
| I3 | Current/expected/semantic diff written before tests |
| I4 | Test points, layers, commands, expected failure, and reason written before code |
| I5 | Dependency-ordered RED/GREEN/refactor/commit slices frozen |
| I6 | Existing focused characterization plus full baseline green |
| I7 | Behavior-specific compile-valid RED observed and recorded |
| I8 | Minimal production-grade GREEN, including defined failures |
| I9 | Refactor only after GREEN; focused tests and invariants remain green |
| I10 | Cross-layer input -> state -> view -> render/runtime behavior green |
| I11 | Failure, recovery, stale identity, capability, platform, and resource bounds green |
| I12 | Performance budget measured and no unsupported optimization claim |
| I13 | Full direct `just check` equivalent and applicable migration/manual gates green |
| I14 | Evidence audit, targeted commits, CyPack-only FF publication, fresh graph symbol proof |

## Performance Budgets

Measurements use release or consistent debug mode on the same host, record
terminal size, sample count, warm-up, CPU, and build profile, and compare to a
recorded pre-change baseline.

| Metric | Budget |
|---|---:|
| Shell geometry compute p95 | <= 0.5 ms |
| 120x40 full frame p95 | <= 8 ms |
| 240x80 full frame p95 | <= 16 ms |
| Pending render payloads per client | <= 1 |
| Identical logical frame outgoing payloads | 0 |
| PTY resize calls during drag preview | 0 |
| PTY resize calls at one committed shell/Miller resize | <= 1 per affected active surface |
| Shell solver complexity | O(node_count), node_count <= 128 serialized / 64 visible |
| Miller logical history | <= 32 path segments |
| Resident Miller directory projections | <= 5 |

If a budget is missed, profile the measured path, keep correctness tests green,
and optimize only the proven bottleneck. Do not introduce component render
loops or a wire protocol as a speculative remedy.

## Per-Phase Git Discipline

Each phase follows this sequence:

1. Record fresh Git status, graph evidence, current behavior, expected
   behavior, test point, expected RED, and exact file ownership.
2. Add only the compile-valid behavior test; run it and require the intended
   assertion failure.
3. Targeted-stage the test files and use the exact `test:` commit message named
   by the active child-plan task.
4. Add the minimum complete product behavior, including the named failure
   branch; run the focused command and require GREEN.
5. Targeted-stage only declared product files and use the exact `feat:`,
   `fix:`, or `refactor:` commit message named by the active child-plan task.
6. If cleanup is needed, make a separate behavior-neutral refactor commit only
   after focused tests remain green.
7. Run phase cross-layer, failure, performance, Linux, Windows, maintenance,
   and full-suite gates as defined by the child plan.
8. Update continuity/evidence in a separate docs commit.
9. Fetch the CyPack fork, prove remote refs are ancestors of local commits,
   push the feature ref and permitted fork target only by fast-forward, verify
   remote SHA equality, then refresh Codebase Memory with the supported
   single-worker fallback if needed.

## Canonical Full Gate

`just` is currently absent on this machine, so execute the complete `check`
recipe directly and stop on the first failure:

```bash
set -euo pipefail
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo nextest run --locked -E 'all()' --status-level fail --final-status-level slow --failure-output final --success-output never
bun test src/integration/assets/herdr-agent-state.test.ts
(cd workers/plugin-marketplace && bun test)
rustup target add x86_64-pc-windows-msvc
LIBGHOSTTY_VT_SIMD=false cargo clippy --bin herdr --locked --target x86_64-pc-windows-msvc -- -D warnings
python3 -m unittest scripts.test_agent_detection_manifest_check scripts.test_changelog scripts.test_docs_translation_parity scripts.test_preview scripts.test_vendor_libghostty_vt scripts.test_vendor_portable_pty
git diff --check
```

Expected: every command exits 0; Nextest reports zero failed and zero retry-only
green; ignored/manual tests are named; Linux and canonical Windows Clippy have
zero warnings; Bun and Python suites have zero failures; the diff has no
whitespace error.

## Program Completion Contract

The program is complete only after all twelve phase entries are closed with
fresh evidence, no required test is missing, every resource bound is enforced,
the full gate is green, manual isolated Files behavior is demonstrated without
touching stable Herdr, commits are atomic and published only to the allowed
fork, remote SHAs match, and the refreshed graph contains the final shell and
Miller symbols. FM5 may legitimately close with a measured NO-GO; it may not
close with an unmeasured preference or silently expand into Apps/Desktop.
