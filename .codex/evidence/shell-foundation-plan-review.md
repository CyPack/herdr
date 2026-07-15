# Shell Foundation and FM-next Plan Review Evidence

Date: 2026-07-15

## Decision

Result: **PASS; SF0 closed and current delivery phase I6/SF1**.

The user approved the bounded direction:

1. SF0-SF6 Shell Foundation and Files Stage migration;
2. FM1-FM5 horizontal/resize/mouse/path-stable/preview decision;
3. Apps/Desktop deferred to a later independent program.

No Rust product behavior was changed by this planning checkpoint. The complete
artifact was committed as `32856f7`, fast-forward published to both CyPack
refs, and verified by exact remote SHA equality.

## Authoritative Inputs Reviewed

- `AGENTS.md`
- `CLAUDE.md`
- `.codex/BOOTSTRAP.md`
- `.codex/CURRENT.md`
- `.codex/TASKS.md`
- `.codex/HANDOFF.md`
- `.codex/CHANGE-PIPELINE-TASKS.md`
- `docs/superpowers/specs/2026-07-15-ratatui-reference-intelligence-v2-1-design.md`
- `docs/superpowers/specs/2026-07-15-herdr-integration-delivery-pipeline-design.md`
- `docs/superpowers/specs/2026-07-15-herdr-shell-foundation-v0-design.md`

Skills and lessons applied:

- `herdr-native-fm`
- `ratatui-design-intelligence`
- `rust-dev`
- `superpowers:writing-plans`

## Fresh Codebase Memory Evidence

- Project: `home-ayaz-projects-herdr`
- Status: 19,534 nodes / 91,017 edges
- Freshness symbol:
  `home-ayaz-projects-herdr.src.ui.file_manager.miller_layout`
- Freshness source: `src/ui/file_manager.rs:122-159`
- Signature: `(area: Rect) -> MillerLayout`
- `ready` was not accepted without the current symbol and source location.

Exact graph snippets/call paths reviewed:

| Symbol | Current evidence | Planning consequence |
|---|---|---|
| `src.ui.shell.ShellLayout` | One recursive root; default is `LeftPanel` plus `CenterContent` | SF2 evolves the existing seam instead of introducing a second outer layout owner |
| `src.ui.shell.layout_node` | Recursive allocation, one child vector allocation, no validation bounds | SF2 validates before compute and freezes O(node_count) limits |
| `src.ui.compute_view_internal` | 78 outgoing graph edges; calls shell, mobile, sidebar, Files, tabs, panes, split, and hit projection | SF2/SF4 split shell and active-surface projection without moving render-time work into state mutation |
| `src.ui.compose.Component` / `Compositor` | Pure render and two fixed layers | Preserve pure fixed composition; do not build an arbitrary registry |
| `src.app.state.AppState` | Large pure-data state with Files/workers/view/sidebar facts | Add aggregate Shell/Stage state, not more unrelated loose fields |
| `src.app.state.ViewState` | Current region map plus current-column FM hits and terminal/pane geometry | Replace with aggregate ShellView/Files view generations and clear hidden-surface hits |
| `src.app.input.mod.App.handle_mouse_without_agent_frame_action` | Overlay, then FM, then sidebar and remaining routes | SF4 makes the existing precedence explicit and fail-closed |
| `src.app.input.file_manager.App.handle_file_manager_mouse` | Uses `terminal_area` and current-column stable row path snapshots only | SF6 moves ownership to NativeFiles Stage; FM3 adds column identity |
| `src.app.actions.AppState.open_file_manager` | Creates `FmState` from active workspace cwd | SF4/SF6 wrap existing state in a typed local app lifecycle |
| `src.app.actions.AppState.close_file_manager` | Clears Files request/state and file context mode | SF6 preserves cleanup while restoring previous Stage/focus |
| `src.persist.snapshot.SessionSnapshot` | Snapshot 3 stores sidebar width and section split, no shell tree | SF3 uses snapshot 4 with v3 migration and corruption containment |
| `src.persist.snapshot.migrate_snapshot` | Called from `parse_snapshot`; forwards v3 fields | Add shell raw-value validation/fallback without losing unrelated state |
| `src.server.render_stream.ClientRenderState.prepare_frame` | Identical semantic/ANSI frame returns `None` | Characterize zero outgoing unchanged frames |
| `src.server.headless.HeadlessServer.render_retained_pty_update_and_stream` | Applies dirty pane rows to prior frame and refuses unsafe states | Static shell must keep the retained dirty-row path |
| `src.server.client_transport.ClientWriterQueue.try_send_render` | One optional render slot; second pending payload returns Full | Preserve at-most-one render backpressure |
| `src.fm.mod.FmState.leave` | Leaves to parent and delegates exact departed-path focus | FM4 must keep every published N2.1 path-stable test green |
| `src.fm.mod.FmState.reload_focusing_path` | One directory snapshot, exact-path search, clamp, context refresh | Extend the same authority rather than reimplementing leave |

Local release comparison proved both HEAD and latest local tag `v0.7.3` use
`SNAPSHOT_VERSION = 3`. The plan therefore assigns shell persistence to
snapshot 4 and explicitly keeps wire `PROTOCOL_VERSION` unchanged.

## Requirement Coverage Review

| Requirement | Plan evidence | Result |
|---|---|---|
| 7 Foundation + 5 FM phases | Program table has exactly 12 ordered rows | PASS |
| Named outer regions | TopBar/AppDock/LeftPanel/WorkspaceStage/RightPanel/BottomBar frozen | PASS |
| Bounded nested composition | Depth 4, split 8, visible 64, serialized 128, stack 32 | PASS |
| Track policies | Fixed, ContentBounded, Resizable, Fill, Collapsed | PASS |
| Primitive/pattern pool | Split, RegionSlot, SurfaceHost, Stack, Scroll, Divider, Collapse, Overlay | PASS |
| Resize transaction | Mouse and keyboard preview/commit/cancel/reset; zero preview churn | PASS |
| Collapse/restore | Stored committed width, clamp, idempotence, Stage rejection | PASS |
| Focus/mouse/overlay | Frozen precedence, capture cleanup, stale generation, background blocking | PASS |
| App lifecycle | Typed definition/instance/surface, singleton Terminal/Files, bounded instances | PASS |
| Files migration | Existing FmState/workers/actions preserved; terminal alive but inert | PASS |
| Persistence | v3 migration, v4 round trip, malformed/over-limit fallback, future reject | PASS |
| Render/SSH performance | Identical skip, retained row, bounded queue, frame/layout budgets | PASS |
| Horizontal Miller | History 32, current plus four cached projections, at most five visible | PASS |
| Column resize | Shared transaction, 16/28/64 bounds, image target one-shot | PASS |
| All-column mouse | Column/directory/entry/generation identity and live revalidation | PASS |
| Finder-like navigation | Append, branch truncation, exact focus restore, watcher pruning, close reset | PASS |
| Preview/inspector | Three measured options and explicit GO/NO-GO gate | PASS |
| Non-goals | No editor, redock, floating windows, arbitrary DSL/plugin tree, protocol, Apps/Desktop | PASS |
| Failure paths | Invalid persistence, stale hits, watcher/worker/I/O/queue/resize/overlay failures | PASS |
| Git discipline | Atomic RED/GREEN/refactor, targeted staging, CyPack-only FF, remote SHA | PASS |

## RED Validity Review

- The first new behavior test is exactly
  `shell_layout_places_dock_sidebar_stage_without_overlap`.
- It is planned as a compile-valid serialization/geometry assertion against
  the existing shell test seam. Missing new named-region behavior causes the
  assertion/deserialize expectation to fail; a compile/setup/filter failure is
  rejected.
- SF1 is characterization and remains GREEN because it records the current
  curtain behavior before replacement.
- Every later production task names its RED test, expected failing behavior,
  GREEN responsibility, failure cases, focused command, atomic commit, and
  broader gate.

## Structural Validation

The local structural audit returned:

- program phase rows: 12;
- Foundation implementation headings: 6 (SF1-SF6; SF0 is documentation);
- FM implementation headings: 5;
- executable plan checkboxes: 167 before final review additions;
- A0-A7 coverage: PASS;
- I0-I14 coverage: PASS;
- mandatory capability/constraint keyword matrix: PASS;
- unfinished-marker scan: clean;
- tracked and ignored-new-file whitespace checks: clean;
- referenced design/plan/continuity files: present and non-empty.

No Markdown linter is installed, so no unavailable lint command is presented as
evidence. No Rust/product/full-suite claim is made for this documentation-only
checkpoint; SF1 begins with a fresh focused and full characterization baseline.

## Current Gate and Next Action

- SF0: closed at artifact `32856f7` with targeted documentation/continuity
  staging, CyPack-only fast-forward publication, exact remote SHA equality,
  and a zero-error single-worker graph refresh.
- A0-A7: complete.
- I0-I5: complete through explicit approval, fresh cartography, semantic diff,
  test architecture, and dependency-ordered slice plans.
- I6/SF1: next.
- Post-publication graph: 19,808 nodes / 91,543 edges with current
  `miller_layout` exact-symbol proof beyond `ready`.
- Product Rust changes: none.
- Stable process/socket contact: none.
- Next test-only action: run the frozen SF1 inventory, then add
  `files_curtain_currently_replaces_terminal_surface` without altering product
  behavior.
