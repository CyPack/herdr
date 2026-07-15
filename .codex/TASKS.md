# Durable Tasks — Herdr Native FM

## P0 — Completion Audit (2026-07-15)

- [x] Reconcile the ignored local `00-MODULE-TREE.md` and all A1–C6 module
  checklists against tracked commits and gate evidence.
- [x] Prove all thirteen core modules plus N1/N3/N4, N2.1, M1, and M2 are
  closed; prove M3 is an evidence-backed implementation NO-GO.
- [x] Preserve exactly four evidence-gated future items: S5, S6, S7, and N2.2.
- [x] Record module-by-module commit, regression, graph, and Git evidence in
  `.codex/evidence/native-fm-completion-audit.md`.
- [x] Keep ignored local continuity repair, tracked documentation, and product
  code as separate concerns; no product code changed in this audit.

At the completion-audit checkpoint the product queue intentionally contained
only four trigger-gated future items; that absence was a verified architecture
decision, not missing decomposition. Later explicit user demand now activates
the bounded Shell/Files/FM program below. The historical P4 decisions remain
evidence of what was rejected, while the new active task list records the
specific trigger, limits, and implementation boundaries. The separate
non-product tooling lane does not grant product authority.

## Active Product Program — Shell Foundation -> Files -> FM-next

The user has now supplied independent concrete product demand that was absent
at the P4.0 checkpoint: Files must become a real app surface instead of a
terminal curtain; AppDock/WorkspaceStage must exist; shell regions must be
bounded, resizable, collapsible, scroll-aware, and overlay-safe; Miller must
support horizontal traversal, column resize, and all-column mouse ownership.
This activates a new bounded program without reviving the rejected arbitrary
component registry, unbounded history, visual editor, or Apps/Desktop scope.

Authoritative plans:

- `docs/superpowers/plans/2026-07-15-herdr-shell-file-manager-program-plan.md`
- `docs/superpowers/plans/2026-07-15-herdr-shell-foundation-v0-implementation.md`
- `docs/superpowers/plans/2026-07-15-herdr-file-manager-post-shell-implementation.md`

### SF0 — Design and Baseline Freeze

- [x] Preserve the current branch, published FM work, tests, graph, and
  user-owned `.superpowers/` state under `mid_flight_adoption`.
- [x] Complete A0-A7 across product, layout, input, runtime/PTY, persistence,
  failure, platform, and performance dimensions.
- [x] Freeze bounds, degradation, typed surface ownership, non-goals, migration,
  rollback, and performance budgets in the approved design.
- [x] Obtain explicit user approval for 7 Foundation + 5 FM phases.
- [x] Discover exact current symbols/call paths through Codebase Memory and
  prove graph freshness with current `miller_layout` rather than `ready`.
- [x] Write the program, Foundation, and FM-next code-level TDD plans with
  exact files, interfaces, test names, expected results/reasons, commands,
  commits, full gates, and publication boundaries.
- [x] Self-review phase count, A0-A7/I0-I14 coverage, primitive/interaction/
  lifecycle/persistence/FM requirements, RED validity, placeholders, links,
  whitespace, and current graph-symbol assumptions; record evidence in
  `.codex/evidence/shell-foundation-plan-review.md`.
- [x] Self-review the complete plan set, update continuity, targeted-stage only
  documentation/continuity, commit and CyPack-only FF publish, verify remote
  SHA, and close SF0 evidence. Artifact commit `32856f7` is published to both
  CyPack refs with exact SHA equality; post-publication graph freshness is
  19,808 nodes / 91,543 edges with current `miller_layout` proof.

### SF1 — Characterization Tests (I6 closed)

- [x] Re-query every named baseline symbol and detect drift from the plans.
- [x] Run the exact legacy desktop/mobile shell, Files composition, v3
  persistence, identical-frame, retained dirty-row, and bounded-render-queue
  characterization inventory.
- [x] Add test-only
  `files_curtain_currently_replaces_terminal_surface`; prove current Files
  replacement behavior and unchanged terminal runtime registry.
- [x] Run the focused set plus fresh full nextest/direct maintenance baseline.
- [x] Commit tests only as `test: characterize shell foundation baseline`.
  Commit `7b9b626d` is fast-forward published to both CyPack refs; focused
  characterization is 11/11, full nextest is 3203/3203 plus the named B0 host
  probe skip, Linux/Windows Clippy is clean, Bun is 17/17, Python is 64/64,
  and the refreshed graph is 19,809 nodes / 91,610 edges.

### SF2 — Shell Geometry Foundation (I7, SF2.4 active)

- [x] RED `shell_layout_places_dock_sidebar_stage_without_overlap` with a
  compile-valid behavior assertion.
- [x] Add bounded named-region model and typed templates.
- [x] Add deterministic fixed/content-bounded/resizable/fill/collapsed solver
  and frozen tiny-terminal degradation. The complete SF2.1-SF2.3 chain through
  `f272a881` is published to both CyPack refs; focused shell is 81/81, frozen
  SF1 is 11/11, full Nextest is 3232/3232 plus the named B0 skip, and the
  single-worker graph is fresh at 19,966 nodes / 92,183 edges.
- [ ] Project cached `ShellView` with generation-safe flattened semantic hits.
- [ ] Close bounds, O(node_count), legacy-equivalence, Linux/Windows/full gates,
  atomic commits, publication, and graph refresh.

Progress evidence:
`.codex/evidence/shell-foundation-sf2-geometry-progress.md`.

### SF3 — Resize / Collapse / Scroll / Persistence

- [ ] Add transactional divider preview/commit/cancel with zero preview disk/
  PTY churn and at most one commit resize.
- [ ] Add bounded collapse/restore and owning horizontal/vertical viewports.
- [ ] Add snapshot v4 shell presentation state; migrate v3 sidebar width,
  preserve sidebar-section ownership, contain invalid shell data, reject future
  versions.
- [ ] Close failure, migration, performance, full-gate, Git, and graph evidence.

### SF4 — SurfaceHost and Input Router

- [ ] Add typed client-local Terminal/Files Stage surface state without new
  runtime/protocol identity.
- [ ] Add focus scope, active capture, topmost hit, page/global shortcut, and
  fail-closed precedence.
- [ ] Prove overlays block every background action and hidden terminal input is
  inert while Files owns Stage.
- [ ] Split shell projection from active-surface projection and preserve pure
  deterministic render/retained PTY behavior.

### SF5 — AppDock

- [ ] Render icon-only Terminal/Files dock at preferred 5, min 3, max 9 cells.
- [ ] Add stable active/running/disabled targets, singleton activation, bounded
  right-click name popover, overlay blocking, resize/collapse, and tiny-terminal
  behavior.
- [ ] Close UI/input/failure/performance/full-gate/Git/graph evidence.

### SF6 — Files as Native Workspace Stage

- [ ] Replace the terminal curtain branch with typed `NativeFiles` Stage
  projection/render while preserving AppDock/LeftPanel independence.
- [ ] Preserve `FmState`, watcher, text/image workers, operations, selection,
  context menus, agent handoff, and all failure/recovery semantics.
- [ ] Prove singleton open/reactivate/close/failure restores previous Stage and
  focus; terminal process stays alive but hidden input/hits/cursor are absent.
- [ ] Close snapshot, render queue, retained PTY, isolated runtime, performance,
  full-gate, Git, remote-SHA, and graph evidence.

### FM1 — Horizontal Miller Viewport

- [ ] Add logical history <=32, resident directory projections <=5, and at
  most five visible complete columns.
- [ ] Add native horizontal wheel, Shift+wheel, and bounded header navigation;
  clamp after path/cache/terminal shrink and clear stale hits.
- [ ] Prove close/reopen reset, inaccessible ancestors, render purity, resource
  bounds, full gates, publication, and graph freshness.

### FM2 — Miller Column Resize

- [ ] Reuse the Shell resize transaction for min 16/preferred 28/max 64 column
  widths.
- [ ] Prove preview causes zero persistence/PTY/filesystem/image-target churn;
  commit updates one revision and at most one final image target.
- [ ] Close stale divider, terminal resize, cancel, 1,000-move bound,
  cross-layer/full/performance/Git/graph gates.

### FM3 — All-Column Mouse Ownership

- [ ] Generate stable column/directory/entry/generation row targets for every
  rendered directory column.
- [ ] Route plain/right/double/wheel gestures in parent/current/preview/ancestor
  columns; keep Ctrl/Shift operation authority current-directory-only.
- [ ] Revalidate non-current paths before mutation; consume stale/reordered/
  deleted/evicted targets without replay or side effect.
- [ ] Close overlay/background-blocking, context/operation/selection, isolated
  SGR mouse, full-gate, Git, and graph evidence.

### FM4 — Finder-Like Path-Stable Growing Navigation

- [ ] Append one child segment on directory selection, truncate descendants on
  ancestor branch change, and replace deeper chain with file preview.
- [ ] Restore exact child focus/cursor/viewport; handle missing/hidden/reordered/
  deleted/root/inaccessible paths deterministically.
- [ ] Preserve all N2.1 tests, chain <=32, resident <=5, watcher generations,
  close/reopen reset, adversarial 10,000-action invariants, and performance.
- [ ] Close full gates, isolated deep-navigation proof, publication, and graph.

### FM5 — Preview / Inspector Placement

- [ ] Measure inline final column, Shell RightPanel, and adaptive hybrid across
  terminal/path/Unicode/preview/failure/focus/performance fixtures.
- [ ] Record raw evidence and explicit GO/NO-GO. A NO-GO keeps inline preview;
  a GO requiring product code must receive a separate approved micro plan.
- [ ] Commit the evidence/decision independently. Do not expand into
  Apps/Desktop or speculative RightPanel consumers.

## Active Non-Product Tooling Lane — Change Pipeline

- [x] Define Ratatui/reference intelligence v2.1 (`86a25e8`).
- [x] Define and generalize Herdr change intelligence plus delivery
  (`0ea0f77`, `600c0d6`).
- [x] Create the durable macro/micro task registry and mid-flight adoption
  contract.
- [x] Review and approve the written specs plus registry.
- [x] Produce and self-review the exact code-level TDD implementation plan.
- [x] Approve the detailed TDD implementation plan and open T2 execution.
- [x] Implement and verify Ratatui Design Intelligence v2.1 with atomic
  baseline/RED/GREEN/governance commits and fresh module gates.
- [ ] Implement and verify `herdr-change-pipeline`, adapters, pilots, Git
  publication, and graph refresh; paused at T3.1 while the sequential active
  product lane closes its current phase.

Full non-product macro/micro registry:
`.codex/CHANGE-PIPELINE-TASKS.md`.

This lane does not authorize Rust product changes and does not activate S5,
S6, S7, or N2.2. A parallel feature/bugfix session may use the registry's
mid-flight adoption contract only after it inventories and preserves the live
work state.

## P0 — Close the Current Increment

- [x] Recover and audit Claude session `f53c720f-f795-4778-970b-d227714ffb1a`.
- [x] Implement A2.2 parent/current/preview Miller columns.
- [x] Prove narrow-width, root, file-placeholder, directory-preview, hidden-cwd, and closed-FM cases.
- [x] Pass the complete `just check` equivalent.
- [x] Align on A2.2 product commit message.
- [x] Commit A2.2 with targeted staging (`6c7c58f`).
- [x] Push `feat/native-fm` and fast-forward fork `master` on the CyPack fork only.
- [x] Reindex codebase-memory after the commit and prove freshness with `miller_layout`.

## P0 — Version the Codex CLI Setup Separately

- [x] Add repo-local bootstrap, current state, task list, memory contract, handoff, evidence, launcher, and project skill.
- [x] Add scoped global Codex hook/pointer and memory update note.
- [x] Record standing authorization for autonomous atomic commits and CyPack
  fork-only fast-forward pushes; do not repeatedly ask for alignment.
- [x] Stage only `.codex/` and `.planning/STATE.md`, commit as
  `docs: add Codex continuity for native file manager`, reindex, and publish.

## P1 — A4 Watcher (Published)

Test points must be written and made RED before production code.

### A4 Test-Point Contract

| Test point | What is tested | Expected result | Why it is required |
|------------|----------------|-----------------|--------------------|
| TP-A4.1-NORMALIZE | Create, modify, remove, rename, duplicate-burst, and irrelevant-path raw events through a pure normalization seam | Relevant events become deterministic refresh intents; duplicates coalesce; unrelated paths are ignored; no filesystem or render dependency | Native backends emit noisy and platform-shaped events, so runtime behavior must not depend directly on backend quirks |
| TP-A4.2-LIFECYCLE | Open FM, change directory, close FM, watcher startup failure, channel closure, and stale-event generation | Exactly one active watcher belongs to the current FM directory; rebinding retires the prior watcher; close leaves no watcher work; failures do not panic; stale events cannot mutate current state | Watcher ownership and teardown races are the highest leak/stale-update risk |
| TP-A4.3-SELECTION | Refresh after sibling create/modify, selected-path delete, selected-path rename, empty-directory transition, and hidden-entry filtering | Preserve selection by exact path when it still exists; otherwise select the nearest valid row and clamp to zero for empty state; preview/parent caches match the resulting selection | Refreshing only the entry vector can silently move the cursor to the wrong file or leave preview context stale |
| TP-A4.4-REAL-FS | Create, delete, and rename files in a temporary directory while the watcher is active | `FmState` converges to disk state within a bounded deadline without fixed timing assumptions or render-time I/O | Pure unit tests cannot prove that the selected backend delivers usable real filesystem events |
| TP-A4.5-FALLBACK | Forced watcher initialization/runtime failure and a backend classified as unreliable or unsupported | The system enters an explicit, testable fallback/degraded state; polling behavior is bounded if selected; silent permanent staleness is forbidden | FUSE, NFS, exFAT, permission, and resource-limit failures invalidate a happy-path-only native watcher |
| TP-A4.6-GATES | Linux full suite, Windows-target clippy, formatting, maintenance tests, dependency advisories, and diff cleanliness | Every applicable gate passes with fresh evidence; zero-test filters and retry-only greens are reported rather than hidden | A cross-platform filesystem feature is not complete when only the local unit path passes |

Execution rule: introduce the smallest test seam needed for each point, run it
RED for the intended missing behavior, then implement only enough production
code to make it GREEN. Complete one test point before beginning the next.

- [x] A4.0: select stable `notify-debouncer-full 0.7.0` (transitive
  `notify 8.2.0`) after local dependency, exact-version, feature, and OSV
  checks; reject upstream release candidates and defer the manifest change
  until the first RED test requires the backend.
- [x] A4.1: define a pure watcher-event normalization seam and test create, modify, delete, rename, duplicate burst, and irrelevant-path events.
- [x] A4.2: connect watcher lifecycle outside render; render remains pure and filesystem-free.
- [x] A4.3: refresh `FmState` after a debounced event while preserving selection by path when possible and clamping safely when not.
- [x] A4.4: prove real-filesystem create/delete/rename behavior in temporary directories.
- [x] A4.5: use native watcher first, explicit polling fallback on init/runtime
  failure, and bounded reconciliation for silent FUSE/NFS/exFAT-class
  backends; unchanged polls do not dirty render.
- [x] A4.6: run Linux, Windows-target, maintenance, and full nextest gates.

### Close A4 Without Mixing Concerns

- [x] Align on product commit: `feat: add live filesystem watching to native file manager`.
- [x] Targeted-stage only `Cargo.toml`, `Cargo.lock`,
  `src/app/file_manager_watcher.rs`, `src/app/mod.rs`, `src/app/runtime.rs`,
  `src/fm/watcher.rs`, and `src/fm/mod.rs`; commit the A4 feature as
  `01ba91d`.
- [x] Align on separate test commit:
  `test: make timing-sensitive lifecycle tests deterministic`.
- [x] Targeted-stage only `src/server/headless.rs` and
  `src/terminal/state.rs`; commit the deterministic fixture fixes as
  `8cd4e89`.
- [x] Restore codebase-memory MCP, run a
  full reindex, and prove `miller_layout`, `NativeFileManagerWatcher`, and
  `normalize_watch_events` are present. Never claim freshness from `ready`
  alone.
- [x] Fetch and verify fast-forward ancestry, then push only the CyPack feature
  branch and fork master. Never push `upstream`; never force.

## P1 — B0 Image Path Beta Spike (Published — GO)

- [x] B0.1 decode a generated test PNG to RGBA and record dependency/cost.
- [x] B0.2 construct a synthetic `KittyImagePlacement`/PaneId without touching
  the stable Herdr session.
- [x] B0.3 prove `encode_graphics_update` framing and lifecycle cleanup.
- [x] B0.4 render Path Beta in a throwaway Kitty host and capture a
  Path Alpha Yazi-in-pane baseline.
- [x] Record wiring size, failure modes, visual-capture evidence, and B2
  go/no-go. Decision: conditional GO; B2 must reuse the existing
  `kitty_graphics` encoder/cache and add bounded decode, generation safety,
  cleanup, and real-host closure tests.
- [x] Commit the isolated product/test concern as `bcba84d`
  (`test: prove native image path beta feasibility`), full-reindex, and
  fast-forward publish to CyPack feature/master only.

## P2 — B1 Text Preview (Verified and Published)

Production code begins only after the matching test point is RED.

### B1 Test-Point Contract

| Test point | What is tested | Expected result | Why it is required |
|------------|----------------|-----------------|--------------------|
| TP-B1.1-BOUNDED-READ | Small regular UTF-8, exact byte boundary, over-limit input, CRLF, multi-byte boundary, newline-free input, and one very long line | Exact in-limit content is preserved; over-limit input produces explicit truncation metadata without splitting UTF-8; allocation/read work is bounded before I/O | Large or adversarial files must not freeze state refresh or cause uncontrolled allocation |
| TP-B1.2-FAILURES | Missing/read-race, permission denied, directory/non-regular, binary NUL, and invalid UTF-8 | No panic or silent lossy success; each case maps to a stable explicit preview status/fallback | Selection and disk state can change between metadata and read, so a happy-path loader is unsafe |
| TP-B1.3-CLASSIFY | Known extension, shebang evidence, unknown extension, unsupported syntax, and highlighter failure | Deterministic syntax choice or plain-text fallback; content remains visible; styles stay bounded | Highlighting must not become a new availability authority for preview content |
| TP-B1.4-LIFECYCLE | Cursor movement, A4 watcher reload, selected-file delete/replace, hidden toggle, and close/reopen | Preview always matches the current selection/generation; stale content is never rendered; closing clears prepared preview state | Navigation and filesystem refresh can otherwise display content from the wrong file |
| TP-B1.5-RENDER | Normal, narrow, and zero preview rectangles plus empty/error/truncated/long styled models | Buffer output has the expected content/status/truncation marker; zero-area is panic-free; render performs no filesystem I/O | Pure render and responsive Miller layout are project invariants |
| TP-B1.6-GATES | Targeted/full nextest, doctest applicability, Linux/Windows canonical clippy, Bun/Python maintenance, render cross-check, and diff cleanliness | Zero fail/retry; skipped or N/A gates are named; a zero-test filter cannot count as green | Narrow unit success cannot establish repo-level production readiness |

- [x] B1.0 select minimal pure-Rust `syntect 5.3.0` for B1.2 after measuring
  compile/runtime/binary/license/OSV/Windows cost. B1.1 adds no dependency;
  B1.2 must use a generation-safe bounded worker, not synchronous input/render
  highlighting. Re-run exact dependency and OSV deltas before manifest change.
- [x] B1.1 add a bounded text-read model in the state refresh path; render
  performs no I/O.
- [x] B1.2 add deterministic syntax classification/highlighting with explicit
  unsupported, binary, invalid-encoding, and highlighter-failure paths.
- [x] B1.3 enforce byte, line, and rendered-column truncation/lazy limits.
- [x] B1.4 prove navigation/watcher lifecycle freshness and responsive render.
- [x] Cross-check render/truncation behavior and pass the full gate: targeted
  64/64, full nextest 2948/2948 with one named B0 host-probe skip, Linux and
  canonical Windows clippy clean, Bun 17/17, Python 64/64, fmt/diff clean,
  doctest N/A for the binary-only crate, and exact five-package OSV delta with
  no security-severity advisory.

## P2 — A3 Navigation and Selection Remainder

Production code begins only after the matching test point is RED. Keep layout
geometry pure and shared by render/hit-testing; do not infer rows from painted
buffer content.

### A3 Remainder Test-Point Contract

| Test point | What is tested | Expected result | Why it is required |
|------------|----------------|-----------------|--------------------|
| TP-A3.2-VIEWPORT | Long current list; repeated up/down; top/bottom; resize taller/narrower/zero-height; reload that removes rows; enter/leave | Cursor is always in range, selected row is visible whenever a row can be drawn, viewport start clamps to the last valid window, and empty/zero-height states remain zero and panic-free | A cursor without explicit viewport invariants disappears or underflows after navigation, resize, and watcher refresh |
| TP-A3.3-HIT-GEOMETRY | Current-row rectangles in one/two/three-column layouts; header/title/divider/parent/preview/empty space; scrolled row offsets; zero-width/height | Only a visible current-row rectangle resolves to its exact entry index; all non-row and degenerate points return no action; render and input consume the same computed geometry | Independent mouse arithmetic drifts from responsive Miller layout and can activate the wrong file |
| TP-A3.3-DISPATCH | Single click on file/dir, double click on directory/file, wheel up/down at bounds, selection followed by keyboard enter | Single click selects exactly that row; directory double-click follows the same enter path; file double-click remains selected until an opener action is explicitly designed; wheel/navigation preserve clamp and refresh preview generation | Hit-testing alone does not prove input routing, action semantics, or stale-preview safety |
| TP-A3.4-SCOPE | Cursor highlight versus multi-selection state across keyboard/mouse navigation and close/reopen | v1 has one cursor-owned visual selection only; no speculative multi-select collection is added. N4/C2 owns later multi-select semantics and must start with its own RED tests | Mixing cursor focus and future bulk selection now would create ambiguous destructive-operation authority |
| TP-A3.5-GATES | Targeted state/geometry/input/render tests, full nextest, Linux/Windows clippy, Bun/Python maintenance, isolated manual mouse cross-check, and diff cleanliness | Every applicable gate passes without retry-only green; manual testing uses throwaway XDG and cleared Herdr socket variables | Mouse geometry is terminal-sensitive and cannot be closed by a narrow unit test alone |

- [x] A3.2 add explicit cursor-follow viewport/scroll state and clamp
  invariants, beginning with TP-A3.2-VIEWPORT RED.
- [x] A3.3 compute named current-row hit rectangles from the responsive Miller
  layout, then wire click/double-click/wheel dispatch test-first.
- [x] A3.4 record v1 single visual-selection scope in code/tests; defer actual
  multi-select state and bulk semantics to N4/C2.
- [x] Run the complete A3.5 gate and isolated manual mouse cross-check before
  publishing the increment.

## P2 — B2 Image Preview (B0 GO; Ordered After B1/A3)

Production code begins only after the matching test point is RED. B0 proves
Path Beta feasibility, not unbounded image decoding or lifecycle safety.

### B2 Test-Point Contract

| Test point | What is tested | Expected result | Why it is required |
|------------|----------------|-----------------|--------------------|
| TP-B2.0-DEPENDENCY | Existing PNG path versus minimal decode/downscale options; exact lock delta, features, license, OSV, compile cost, and Windows support | Select the smallest supportable pure-Rust path or document why the existing dependency is sufficient before changing the manifest | Image crates can add large transitive/security/platform cost; dependency choice must be evidence-driven |
| TP-B2.1-DECODE | Supported image, alpha, exact byte/pixel boundaries, corrupt/truncated input, absurd dimensions, allocation overflow, and decode failure | Decode/downscale work is hard-bounded before allocation; valid pixels are deterministic; every failure is explicit and panic-free | Untrusted or huge images can exhaust memory or stall the UI even when render itself is pure |
| TP-B2.2-PLACEMENT | Prepared image state to synthetic PaneId/local preview slot across one/two/three-column and zero/narrow geometry | Placement stays client-local, uses current FM preview geometry, and emits no server/private-TUI protocol coupling | B0's synthetic placement must become a real FM seam without making presentation state runtime authority |
| TP-B2.3-LIFECYCLE | Cursor movement, watcher reload, replace/delete, enter/leave, close/reopen, resize, stale generation, and worker failure | Only the current selected path/generation can publish pixels; every transition removes superseded placements/cache state; failure degrades explicitly | Async decode and filesystem refresh can otherwise paint the wrong file or leak graphics resources |
| TP-B2.4-PAINT | Existing `kitty_graphics` encoder/cache upload, display, dedup, redisplay, replacement, and removal from the FM preview slot | Reuse Path Beta framing/cache; unchanged frames do not re-upload; render performs no filesystem/decode work | A second graphics pipeline would duplicate lifecycle bugs and violate the established pure-render boundary |
| TP-B2.5-HOST-GATES | Deterministic image comparison, isolated Kitty real-host capture, non-Kitty fallback, full nextest, Linux/Windows clippy, Bun/Python maintenance, and diff cleanliness | Pixels/placement/fallback match expected evidence; all applicable gates pass with no retry-only green; throwaway XDG leaves no process/temp artifact | Unit framing cannot prove terminal-host rendering, cleanup, or graceful unsupported-host behavior |

- [x] B2.0 select `image 0.25.10` with default features disabled and only
  `png/jpeg/gif/webp` after exact lock, license, advisory, compile-cost, and
  Windows checks. Keep direct `png 0.17.16` unchanged; full evidence is in
  `.codex/evidence/b2-image-dependency.md`.
- [x] B2.1 bounded decode/downscale path with corrupt/huge image failures.
- [x] B2.2 construct preview placement with synthetic PaneId and no server/TUI
  protocol coupling.
- [x] B2.3 add a generation-safe image worker and local preview painting beside
  existing pane graphics encoding.
- [x] B2.4 per-slot cache/dedup, cleanup, resize, navigation, and stale-image
  generation tests.
- [x] B2.5 require image-compare plus real throwaway host evidence before
  closure. Final evidence: B2/FM/Kitty 96/96; full nextest 2983/2983 with the
  named B0 interactive probe skipped; Linux/Windows clippy clean; Bun 17/17;
  Python 64/64; source-to-host comparison 0/271425 pixels different; FM close
  returned the captured preview area to one background color; semantic exit
  left no test process, socket, or throwaway XDG root.

## P3 — C1 Header Actions + N3 Action Bar

Production code begins only after the matching test point is RED. Header
geometry is client-local presentation/input state; it must not enter the
server protocol, and render must remain pure.

### C1/N3 Test-Point Contract

| Test point | What is tested | Expected result | Why it is required |
|------------|----------------|-----------------|--------------------|
| TP-C1.1-GEOMETRY | Copy, paste, new-folder, and delete buttons at normal, narrow, zero-height, and degenerate coordinates | Named rectangles are ordered, disjoint, right-aligned, and complete; narrow layouts retain only whole higher-priority buttons; degenerate layouts expose no action | Render and future input must share one fail-closed geometry seam so clipped labels never leave phantom click targets |
| TP-C1.1-VIEW | Open/closed FM in desktop/mobile `compute_view`, plus component render with and without a preceding full-frame compute | `ViewState` snapshots current header rectangles only while FM is open; render consumes the same geometry and clears stale areas on close | Independent render/input arithmetic would drift after responsive layout changes |
| TP-C1.2-DISPATCH | Left click inside every action rectangle, gaps, cwd identity, outside header, narrow hidden actions, zero area, stale frame, and non-left mouse buttons | Only a current visible rectangle resolves to its exact action tag; every gap/stale/hidden/degenerate/non-left event is consumed or ignored according to an explicit contract without triggering a file operation | Geometry alone does not prove safe routing, and destructive tags must never be inferred from coordinates |
| TP-N3.1-CONTENT | Directory/file/empty selection, writable/read-only/error state, clipboard empty/populated, watcher refresh, navigation, and close/reopen | Persistent action content reflects the current selection and prepared state without filesystem I/O during render; stale selection state is cleared | An action bar that lags selection can advertise operations for the wrong path |
| TP-N3.2-AUTHORITY | Enabled and disabled copy/paste/new-folder/delete states, including missing path, unsupported target, read-only destination, and in-flight operation | Disabled actions are visibly distinct and dispatch no side effect; enablement comes from explicit state, never label presence or paint output | Hidden or implicit authority is unsafe for destructive and filesystem-mutating actions |
| TP-C1-GATES | Targeted geometry/input/render tests, full nextest, Linux/Windows clippy, Bun/Python maintenance, isolated mouse cross-check when dispatch lands, graph freshness, and diff cleanliness | All applicable gates pass without retry-only green; the one intentional B0 host probe remains named; no stable Herdr/socket or temp artifact is touched | Header actions cross rendering, input, and future filesystem authority, so narrow unit success is insufficient |

- [x] C1.1 named header-button rectangles and action tag enum. RED commit
  `0ed5e51`; GREEN commit `c9bfbf9`. Geometry/render/ViewState targeted 4/4;
  full nextest 2986/2986 with one named B0 host probe skipped; Linux/Windows
  clippy, Bun 17/17, Python 64/64, fmt, and diff-check clean.
- [x] C1.2 hit-test dispatch with disjoint geometry and narrow/zero-area cases.
  RED commit `dbc6798`; GREEN commit `7fd01de`. Exact tags 2/2, full FM
  input 13/13, full nextest 2988/2988 with one named B0 host probe skipped;
  Linux/Windows clippy, Bun 17/17, Python 64/64, fmt, and diff-check clean.
  Dispatch is deliberately side-effect free until N3 defines authority.
- [x] N3.1 selection-sensitive persistent action-bar content. RED commit
  `b5cc95c`; GREEN commit `510eebc`. Targeted 3/3, FM 135/135, full nextest
  2991/2991 with one named B0 host probe skipped; Linux/Windows clippy, Bun
  17/17, Python 64/64, fmt, and diff-check clean. Selection/clipboard content
  is client-local and render remains filesystem-free.
- [x] N3.2 explicit enabled/disabled states with no hidden side effects. RED
  commit `446613a`; GREEN commit `267ad91`. Exact authority/preparation/render/
  dispatch 7/7, FM/input/render/Kitty regression 165/165, full nextest
  2996/2996 with one named B0 host probe skipped; Linux/Windows clippy, Bun
  17/17, Python 64/64, fmt, and diff-check clean. Missing cwd, read-only cwd,
  unsupported Unix special targets, empty clipboard, absent selection, and
  in-flight operation all fail closed. Disabled clicks are consumed with no
  state or filesystem mutation.

## P3 — C2 Row Actions + N4 Multi-Select

Production code begins only after the matching test point is RED. Row action
geometry is a client-local ViewState projection. It must share the existing
responsive Miller layout and must never infer authority from rendered text.
N4 selection state remains distinct from the cursor so destructive bulk
authority has one explicit source of truth.

### C2/N4 Test-Point Contract

| Test point | What is tested | Expected result | Why it is required |
|------------|----------------|-----------------|--------------------|
| TP-C2.1-ROW-GEOMETRY | Current-row name and action rectangles in one/two/three-column layouts, first/middle/last visible row, scrolled viewport, narrow/zero dimensions, long Unicode names, and divider/header/empty regions | Every visible current row has one bounded name rectangle plus zero or more complete disjoint action rectangles; clipped actions disappear as whole targets; rectangles never cross the current Miller column or resolve outside the visible viewport | Row-local controls that use independent arithmetic can overlap names, dividers, or adjacent rows and dispatch an action for the wrong path |
| TP-C2.2-ROW-DISPATCH | Unmodified left click on each visible row action, row name, gaps, hidden/clipped actions, stale row index/path after watcher reload, non-left and modified clicks | Only a current visible rectangle whose snapshotted row identity still matches returns its exact row-action tag; name clicks preserve selection behavior; gaps, stale identities, hidden actions, and unsupported buttons fail closed without filesystem mutation | Coordinates alone are insufficient authority when watcher refresh can reorder or delete entries between compute and input |
| TP-N4.1-SELECTION-STATE | Ctrl-toggle, Shift-range anchor, plain click/cursor movement, keyboard equivalents, hidden toggle, reload reorder/delete, directory enter/leave, and close/reopen | Multi-selection is an explicit deduplicated path/identity set separate from cursor focus; range order follows the current visible list; missing entries are pruned deterministically; navigation and lifecycle rules are explicit and panic-free | Conflating cursor focus with bulk selection can silently expand a destructive operation to unintended files |
| TP-N4.2-BULK-AUTHORITY | Zero/one/many selections, mixed supported/unsupported entries, read-only target, clipboard state, selection clear, select-all/range limits, and operation-in-flight state | Bulk toolbar labels/counts and enabled/disabled reasons derive only from prepared selection authority; one unsupported/stale member disables or excludes according to an explicit tested policy; clear/select-all are bounded and deterministic | Bulk operations need auditable all-target authority and cannot inherit single-row assumptions |
| TP-C2-N4-GATES | Focused geometry/state/input/render tests, watcher reorder/delete regression, full nextest, Linux/Windows clippy, Bun/Python maintenance, isolated mouse cross-check if runtime dispatch lands, graph freshness, and diff cleanliness | Every applicable gate passes with the B0 host probe as the only named skip; no stable Herdr/socket, user process, or residual temp state is touched | Responsive row actions plus multi-selection cross rendering, input, watcher reconciliation, and future destructive-operation authority |

- [x] C2.1 split each row into disjoint name/action rectangles. RED `d4d404e`,
  GREEN `9a15328`; focused/readability 8/8, FM impact 71/71, full nextest
  2998/2998 with one named B0 host probe skipped, Linux/Windows clippy, Bun
  17/17, Python 64/64, fmt, diff-check, and graph freshness clean. The rejected
  3-cell prototype proved ordinary-name truncation; complete one-cell `>rx`
  targets retain all three tags without crossing the Miller column.
- [x] C2.2a make exact unmodified-left row-action classification RED for each
  visible tag; row-name, non-left, modified, outside, and hidden targets remain
  non-actions.
- [x] C2.2b bind every snapshotted action target to stable path identity and
  prove watcher reorder/delete and stale-index cases fail closed.
- [x] C2.2c route exact row-action tags before terminal input while preserving
  current row-name selection behavior and performing no filesystem mutation.
- C2.2 RED `94e4a02`, GREEN `9ef90c6`; exact 3/3, all FM input 17/17, FM
  impact 74/74, full nextest 3001/3001 with one named B0 probe skipped,
  Linux/Windows clippy, Bun 17/17, Python 64/64, fmt, diff-check, and graph
  freshness clean.
- [x] N4.1a cursor-independent path-identity state. RED `e876223`, GREEN
  `590e376`; plain replace, Ctrl toggle, current-order Shift range, stable
  anchor, deduplication, stale-target rejection, and cursor independence.
- [x] N4.1b watcher/reload, hidden, navigation, empty, and close/reopen
  lifecycle. RED `1789bbd`, GREEN `5c14439`; live paths survive reorder while
  missing/hidden identities and anchors prune deterministically.
- [x] N4.1c exact mouse/keyboard gestures, stable row identity, and pure visual
  projection. RED `699a6a6` + `fc19237`, GREEN `86b618a`; broad 137/137 and
  full 3015/3015 plus one named B0 skip, Linux/Windows clippy, Bun 17/17,
  Python 64/64, fmt/diff, and graph freshness clean.
- [x] N4.2a derive zero/one/many toolbar identity and Copy/Delete authority
  only from explicit selection paths; preserve current visible order and fail
  closed for stale, ambiguous, mixed unsupported, read-only, clipboard-empty,
  and operation-in-flight states. RED `d5e027f` + `0c76017`, GREEN `0302b10`.
- [x] N4.2b make Ctrl+A select-all and Ctrl+Shift+A clear exact and bounded.
  Complete unique sets up to 4,096 paths succeed; overflow, duplicate, stale
  anchor, ambiguous selected identity, and oversized range reject atomically.
  RED `36c815f`, GREEN `57e2a44`.
- [x] N4.2c prove rejected keyboard Shift range preserves cursor, paths, and
  anchor; RED `50619ff`, GREEN `cb5a43e`. Persistent toolbar render covers no
  selection, one name, `N selected`, clipboard count, and distinct disabled
  styling without render-time I/O.
- N4.2 gates: focused staged runs 6/6 + 4/4 + 2/2, broad FM/input/render
  112/112, full nextest 3020/3020 plus one named B0 probe skip, Linux/Windows
  clippy, Bun 17/17, Python 64/64, fmt/diff, and graph freshness clean.

## P3 — C3 Context Menu

Production code begins only after the matching test point is RED. Reuse the
existing `ContextMenuKind`/`ContextMenuState` popup lifecycle; do not create a
parallel FM-only modal stack. C3 models and dispatches action intent only. C4
owns filesystem mutation and C5 owns agent delivery.

### C3 Test-Point Contract

| Test point | What is tested | Expected result | Why it is required |
|------------|----------------|-----------------|--------------------|
| TP-C3.1-CONTEXT-MODEL | Cursor-only/zero, one file, one directory, multiple, stale/ambiguous, unsupported, read-only, and operation-in-flight prepared selection | No explicit selection produces no file menu; otherwise Open/Copy/Rename/Delete/Compress/Send-to-Agent remain in deterministic order with exact enabled/disabled reasons; multiple selection permits only bulk-capable actions; in-flight overrides every item | A context menu cannot invent authority from focus or hide one unsafe member inside an apparently valid bulk action |
| TP-C3.2-POPUP-GEOMETRY | Right-click first/middle/last visible rows at all Miller breakpoints, screen edges, narrow/zero areas, stale row identity, and background/divider/header/preview regions | Only an exact live current-row identity opens the existing context-menu state; popup clamps inside the screen, keeps complete rows, and never crosses into a hidden terminal target | Watcher reorder plus responsive geometry can otherwise open a menu for the wrong path or place unreachable items off-screen |
| TP-C3.2-POPUP-LIFECYCLE | Right-click selection policy, hover, Up/Down, Enter, Esc, outside click, FM close, reload delete/reorder, and disabled item activation | Focus/highlight remains bounded; close paths clear menu state; stale/disabled activation is consumed without action or filesystem mutation; enabled items emit only exact intent tags | Existing pane/workspace modal behavior must remain intact while FM-specific state fails closed across watcher and input races |
| TP-C3.3-PLUGIN-SURFACE | Manifest `contexts=["file"]`, wrong/unknown contexts, disabled plugin, one/many paths, ordering, duplicate action IDs, and invocation context serialization | Valid enabled file actions append deterministically with exact path context; invalid/disabled/duplicate declarations fail closed; shared plugin/runtime facts use neutral API names rather than TUI-only socket fields | Plugin extension is part of the C3 promise and must not deepen the private TUI client boundary or fabricate unsafe filesystem authority |
| TP-C3-GATES | Focused model/geometry/input/render/plugin tests, existing context-menu regressions, FM/watcher regressions, full nextest, Linux/Windows clippy, Bun/Python maintenance, graph freshness, and diff cleanliness | Every applicable gate passes; the named B0 host probe is the only skip; no stable Herdr/socket or user process is touched | C3 crosses a mature global modal path, so isolated happy-path tests cannot establish production safety |

- [x] C3.1a add `ContextMenuKind::File` and a deterministic six-item model from
  prepared N4.2 selection authority; add no popup opening or real action.
- [x] C3.1b preserve all existing workspace/tab/pane/project menu item and
  invariant behavior while adding exact disabled reasons for file items.
- C3.1 plan `d56e3db`, model RED/GREEN `5d6fc1d`/`02c60e7`, precedence
  RED/GREEN `d9f28b5`/`0832ccc`; focused 5/5, menu-model 7/7, full nextest
  3025/3025 plus one named B0 skip, Linux/Windows clippy, Bun 17/17, Python
  64/64, fmt/diff, and graph freshness clean.
- [x] C3.2a route exact right-click current-row identity into the existing
  popup lifecycle with bounded placement and selection policy. RED `69864d6`,
  GREEN `ad5f8a5`; popup focused 4/4 and broad FM/global-menu 48/48.
- [x] C3.2b render enabled/disabled file items and prove keyboard/mouse close,
  highlight, stale-target, and no-side-effect dispatch semantics. Lifecycle
  RED/GREEN `73df647`/`45c151f`; render RED/GREEN `1078215`/`0915964`.
  Typed intent is revalidated against current path order and authority; no
  filesystem or agent side effect runs. Full gate: 3033/3033 plus one named
  B0 skip, Linux/Windows clippy, Bun 17/17, Python 64/64, fmt/diff clean.
- [x] C3.3 define and verify the plugin file-action surface without deepening
  private TUI socket coupling. RED `0e06181`, GREEN `3c11369`. Focused 8/8,
  plugin/context 35/35, FM/watcher/global-menu 112/112, full nextest 3041/3041
  plus only the named B0 host probe skip, Linux/Windows clippy, Bun 17/17,
  Python 64/64, schema/fmt/diff clean. Graph 18,246 / 85,535 is fresh.

## P3 — C4 Safe File Operations

### C4 Test-Point Contract

| Test point | What is tested | Expected result | Why it is required |
|------------|----------------|-----------------|--------------------|
| TP-C4.1-PREFLIGHT | Zero/one/many exact prepared sources; missing, replaced, unsupported, and non-UTF-8 targets; destination absent/file/directory/read-only; same path, ancestor/descendant, symlink, collision, and operation-in-flight state | Build one immutable bounded operation plan from current authority or fail before the first write; default collision policy never overwrites; symlinks are never followed implicitly; render performs no filesystem work | C3 intent is only a snapshot, so every real operation must defeat TOCTOU and path-identity ambiguity before mutation |
| TP-C4.1-COPY | File/directory/multi-source copy, staged destination, existing target, permission/disk/write failure injection, cancellation at each phase, metadata policy, symlink no-follow behavior, and partial cleanup | Success publishes complete destinations in deterministic order; failure/cancel removes staging data and reports every committed/uncommitted item explicitly; no silent partial success or implicit overwrite | Recursive copy and multi-source operations otherwise leave plausible-looking but incomplete data |
| TP-C4.1-MOVE | Same-filesystem rename, cross-filesystem fallback, collision, source/destination replacement, partial copy, cancellation, and source-removal failure | Same-filesystem move uses atomic rename where supported; fallback commits verified copy before source removal; source is never deleted after failed/incomplete copy; any partial terminal state is explicit and recoverable | Cross-device moves turn one apparent action into copy plus destructive delete and require a stronger commit boundary |
| TP-C4.1-LIFECYCLE | Bounded worker/queue, one in-flight operation, progress monotonicity, cancel idempotence, FM close/reopen, stale completion generation, and panic/error conversion | Filesystem work stays outside render; memory/work concurrency is bounded; stale callbacks cannot mutate current state; every operation reaches one explicit terminal state | A responsive TUI must not trade UI liveness for unsafe background mutation or accept late results into new state |
| TP-C4.1-WATCHER | Own-operation watcher bursts, rename/create/delete reorder, reconciliation deadline, selection pruning, and polling fallback | Watcher and explicit completion converge to one current listing without duplicate entries, stale selection, hot retry, or lost terminal result | Native operations and watcher refresh race by design and must have a deterministic reconciliation owner |
| TP-C4.2-CONFIRM | Header/context destructive intents, current path/order authority, explicit trash versus permanent-delete choice, stale dialog, FM close/reopen, and operation-in-flight state | No destructive worker plan exists before a current explicit confirmation; stale, unsupported, missing, reordered, or in-flight authority fails closed | A click snapshot must never become delayed destructive authority |
| TP-C4.2-TRASH | File/directory/multi-source trash, symlink-as-link behavior, missing/replaced paths, backend unavailable/permission failures, cancellation boundaries, and platform result mapping | Default destructive action moves exact entries to platform trash without following symlinks; every item has an explicit terminal result and failures preserve remaining sources | Trash is the recoverable default but platform backends can partially fail and must not be reported as all-or-nothing success |
| TP-C4.2-DELETE | Separately gated permanent delete for file, empty/non-empty directory, symlink, read-only/permission failure, replacement race, cancellation, and partial progress | Permanent deletion is never implicit, requires stronger confirmation, revalidates identity immediately before mutation, never follows symlinks, and reports irreversible partial completion exactly | Permanent delete has no rollback boundary and therefore needs stronger authority and failure accounting than trash |
| TP-C4.2-RECOVERY | Worker panic/disconnect, partial multi-item terminal state, watcher event reorder/burst, selection pruning, retry, and temp-artifact scan | UI remains responsive; completed items, retained sources, and failed items stay distinguishable; current listing converges once without hot retry or leaked staging data | Destructive partial failure must remain understandable and recoverable instead of looking like a clean success |
| TP-C4.3-INTENT | Context-menu and row Rename routes; exact current single-target identity; stale/reordered/multi-selection/closed-FM/in-flight authority | Every route converges on one typed current target; stale or ambiguous authority is consumed without opening a dialog, scheduling work, or mutating disk | Rename must not turn an old coordinate or display label into filesystem authority; the header has no Rename control |
| TP-C4.3-NAME | Empty, unchanged, `.`, `..`, separator-bearing, absolute, NUL/non-UTF-8, platform-reserved, and over-limit component names | Invalid names fail before worker scheduling; unchanged input is an explicit no-op; the accepted name is one bounded filesystem component | User text is untrusted path input and must never escape the current parent or create platform-specific undefined behavior |
| TP-C4.3-COLLISION | Existing exact target, case-fold collision where applicable, file/directory mismatch, duplicate bulk outputs, and target replacement before commit | Default policy never overwrites; all collisions fail closed with exact source/target evidence before mutation or at final revalidation | Rename has no safe implicit overwrite policy, especially across case-insensitive filesystems |
| TP-C4.3-ATOMIC | Same-directory no-replace rename, immediate source identity revalidation, symlink-as-link behavior, source replacement, and destination creation races | A single rename uses the strongest platform no-replace primitive available; symlinks are renamed rather than followed; a race cannot replace an unrelated entry | Validation alone cannot close TOCTOU; the commit primitive must preserve both source and destination identity |
| TP-C4.3-BULK | Deterministic old-to-new mapping, bounded input count, duplicate outputs, chains, swaps/cycles, temporary staging, injected failure, and recovery | The complete mapping validates first; cycles use private collision-safe staging; terminal state distinguishes committed, restored, retained, and uncertain items without silent partial success | Bulk rename is a transaction-like graph operation and naive sequential renames corrupt chains or swaps |
| TP-C4.3-LIFECYCLE | One bounded worker lane, cancellation boundaries, panic/disconnect, close/reopen generation, watcher reorder/burst, selection pruning, and temp-artifact scan | Render stays pure; stale completion cannot mutate a new FM generation; every item terminalizes; current listing converges once; private staging is removed or explicitly reported | Rename must compose with the existing C4 worker and A4 watcher without introducing a second race-prone lifecycle |
| TP-C4.4-PROGRESS | Transfer, delete, single-rename, and bulk-rename progress for zero/small/bounded-many inputs; repeated/coalesced worker updates; terminal completion | Aggregate and per-item progress are monotonic, bounded, and never exceed known work; terminal state is exact; progress transport cannot create an unbounded queue or render-time mutation | Long operations need truthful responsiveness without turning progress reporting into a memory or event-storm failure mode |
| TP-C4.4-CANCEL | Cancel before execution, during reversible staging/copy, at irreversible publish/delete boundaries, repeated cancel, and cancel/completion races | Cancellation is idempotent; reversible work is restored or removed; already committed work is reported as committed; every remaining item terminalizes without claiming an impossible rollback | Cancellation cannot be modeled as a generic success/failure bit once an operation crosses irreversible boundaries |
| TP-C4.4-RECONCILE | Own-operation watcher bursts before/during/after completion, polling fallback, selected-path removal/rename, cwd change, close/reopen, and stale generations | One current generation owns reconciliation; matching cwd converges once and preserves/prunes selection by exact path; stale callbacks cannot reload or project into a reopened FM | Worker completion and filesystem watchers race by design, so refresh ownership must be deterministic |
| TP-C4.4-RECOVERY | Worker panic/disconnect after progress, cancellation followed by a new operation, uncertain rename recovery evidence, and lane reuse | The existing single worker lane remains reusable; no operation stays in-flight forever; uncertain paths remain visible; no second scheduler, hot retry, or private artifact survives | A terminal UI must recover from worker failure without losing evidence or requiring process restart |
| TP-C4.4-GATES | Focused progress/cancel/reconcile/failure tests, all C4 operation regressions, full nextest, Linux/Windows clippy, Bun/Python maintenance, graph freshness, and artifact/diff cleanliness | Every applicable gate passes with only the named B0 host probe skipped; no stable Herdr/socket or user process is touched | C4 cannot close until all operation kinds share one verified lifecycle rather than separate happy paths |
| TP-C4-GATES | Focused preflight/copy/move/failure/cancel/watcher tests, isolated real-filesystem cross-check, existing FM/context/plugin regressions, full nextest, Linux/Windows clippy, Bun/Python maintenance, graph freshness, temp-artifact and diff cleanliness | All applicable gates pass; only the named B0 host probe is skipped; no stable Herdr/socket or user process is touched; no staging/temp artifact remains | Destructive-capable filesystem work cannot be closed by happy-path unit tests alone |

- [x] C4.1 copy/move outside render, with collision, permission, partial-write,
  cancellation, and cross-filesystem tests.
- C4.1 RED/GREEN chain: preflight `386ddce`/`a9f022b`, staged COPY
  `47c753e`/`2848d97`, safe MOVE `e422d03`/`606d7ea`, bounded worker
  `f1590be`/`88cda7f`, and App lifecycle `626b7c3`/`98c51e4`.
- C4.1 exact-path preflight is immutable and revalidated; COPY stages then
  no-replace publishes, MOVE prefers same-filesystem rename and uses
  copy-before-delete on EXDEV. The App owns one persistent worker lane and a
  pure generation/status projection; completion reloads only the matching cwd.
- C4.1 gates: operation core 15/15, App/worker 8/8, broad FM/watcher/preview
  147/147, full nextest 3064/3064 plus one named B0 skip, Linux/Windows clippy,
  Bun 17/17, Python 64/64, fmt/diff/temp clean. Fresh graph: 18,453 / 86,399.
- [x] C4.2 trash/delete with confirmation, symlink, missing-path, and rollback
  policy; destructive permanent delete is never implicit.
- C4.2 RED/GREEN chain: confirmation authority `733d423`/`12730a6`, modal
  render `5e1f50d`/`95b2a01`, delete core `9c1316b`/`8c558da`, worker lifecycle
  `877519b`/`73b4b39`, per-item recovery `31dacd4`/`d64b9be`, isolated real
  trash test `d316e79`, worker terminalization `193b166`/`61150b3`, modifier
  hardening `c08315b`/`92f453f`, root-path rejection `5c143b2`/`917cd57`.
- C4.2 exact-path confirmation defaults to Trash and requires a separate
  Permanent Delete stage. Immutable symlink-safe preflight and immediate
  revalidation run outside render; trash and permanent delete share the one
  bounded operation lane and preserve ordered per-item terminal evidence.
- C4.2 gates: focused 29/29, broad FM/watcher/preview/context/plugin 321/321,
  full nextest 3086/3086 plus one named B0 skip, Linux/Windows clippy, Bun
  17/17, Python 64/64, fmt/diff/temp clean. Exact OSV query for `trash 5.2.6`
  returned no vulnerability record. Fresh graph: 18,576 / 86,769.
- [x] C4.3 rename and bulk-rename validation, conflicts, and atomicity limits.
- C4.3 atomic chain: intent `2028bce`/`09ad9cd`, name behavior
  `4162b2b`/`59e7d97`/`6e92672`, atomic execution `902c480`/`8ec583b`,
  cycle-safe bulk recovery `01aec01`/`3396df3`, recovery-path evidence
  `73a547a`/`308fb5d`, worker lifecycle `c023c37`/`4cffcb7`, App bulk
  lifecycle `770366c`/`36cb8a6`, canonical injected I/O error `03ac819`, and
  shared operation-name validation `91d3a41`/`c7043e2`.
- C4.3 context-menu and row intent require one exact current selection; the
  single-name modal deliberately rejects multi-selection. Typed bounded bulk
  mappings enter at the App boundary, validate completely, stage cycles and
  swaps privately, and preserve exact retained/restored/uncertain recovery
  paths. Single and bulk rename share the existing operation lane and the
  common platform-aware name authority.
- C4.3 gates: focused rename/bulk/lifecycle 163/163, full nextest 3109/3109
  plus only the named B0 host probe skipped, Linux/Windows clippy, Bun 17/17,
  Python 64/64, fmt/diff/temp clean. Fresh graph: 18,722 / 88,526 with
  `miller_layout` and current single/bulk/name/App symbols.
- [x] C4.4 bounded progress/cancel lifecycle and watcher reconciliation.
- [x] C4.4.1 make TP-C4.4-PROGRESS RED and add one bounded/coalesced progress
  projection shared by transfer, delete, single rename, and bulk rename.
- C4.4.1 atomic chain: worker/App `aa9c894`/`da46bfb`, transfer
  `84db86a`/`2141593`, delete `edc1588`/`d0a0c8a`, single rename
  `3469883`/`94465e2`, and bulk rename `f5ea272`/`cd4368a`. The worker owns one
  latest-value same-generation progress slot; repeated updates coalesce,
  started-item count is monotonic/bounded, App projects only Pending to
  Running, and completion/stale generations clear or reject progress.
- C4.4.1 gates: focused C4 operation regression 57/57, full nextest 3115/3115
  plus only the named B0 host probe skipped, Linux/Windows clippy, Bun 17/17,
  Python 64/64, fmt/diff/temp clean. Separate test-only `30d99bd` made the OMP
  lifecycle fixture use one explicit clock after parallel load exposed mixed
  real/synthetic `Instant` ordering. Fresh graph: 18,745 / 87,178 with the
  progress type, shared worker seam, four observer adapters, and
  `miller_layout`.
- [x] C4.4.2 make TP-C4.4-CANCEL RED and define exact reversible versus
  irreversible cancellation boundaries with idempotent terminalization.
- C4.4.2 atomic chain: single rename `29572ab`/`d246f09`, delete
  `43d573b`/`1cf0ca4`, typed Esc intent `eef9a9b`/`a0d91ec`, end-to-end
  generation-safe App cancellation `699f21c`/`9eb7f4b`, single rename race
  precedence `f0a8280`/`26484ed`, buffered completion race
  `a66bef7`/`dfe21e6`, and bulk rename race precedence
  `15c7a27`/`d77858a`.
- C4.4.2 preserves existing transfer staging/commit rollback, checks delete
  before irreversible mutation, gives observed cancel precedence over single/
  bulk revalidation races, routes repeated Esc only to the matching active
  generation, and rejects cancellation after completion is buffered.
- C4.4.2 gates: broad C4/input 98/98, full nextest 3122/3122 plus only the named
  B0 skip, Linux/Windows clippy, Bun 17/17, Python 64/64, fmt/diff/temp clean.
  Fresh graph: 18,756 / 87,282 with the typed key intent, App/worker cancel
  seams, cancellation tests, and `miller_layout` after stale `ready` proof.
- [x] C4.4.3 make TP-C4.4-RECONCILE RED and prove one matching-generation
  refresh across watcher bursts, polling fallback, selection pruning, and
  close/reopen races.
- C4.4.3 atomic chain: queued ownership `0b04e73`/`9a22d1e`, watcher-first
  ownership `411de3d`/`6bdb97c`, delayed burst ownership
  `e9361ab`/`38280fb`, same-cwd reopen generation `1d7350a`/`779d771`, and
  fallback/selection cross-check `d1a2d2e`. Runtime-only watcher generation,
  revision, and exact path ownership coalesce before/during/after completion;
  external paths reload immediately and stale reopened generations fail closed.
- C4.4.3 gates: broad C4/FM 126/126, full nextest 3128/3128 plus only the named
  B0 skip, Linux/canonical Windows clippy, Bun 17/17, Python 64/64, fmt/diff and
  operation/staging artifact checks clean. Fresh graph: 18,786 / 87,697 with
  `own_operation_reconcile`, exact lifecycle tests, and `miller_layout` after
  stale `ready` proof.
- [x] C4.4.4 make TP-C4.4-RECOVERY RED and prove lane reuse after cancel,
  panic, disconnect, and uncertain bulk recovery without orphan state.
- C4.4.4 atomic chain: disconnected lane RED/GREEN `0881976`/`7847a6c`,
  progress-panic coverage `8974f4c`, cancellation-to-next-generation coverage
  `bcc9ef5`, exact private bulk-recovery evidence `7e2af79`, disconnect cleanup
  idempotence `03b9395`, and test-fixture lint closure `c674296`. A dead worker
  terminalizes every remaining item, clears reconciliation ownership, and is
  replaced at the prior generation floor; caught panic and cancellation reuse
  the existing lane; uncertain staging paths remain exact App evidence; a
  second sync is a no-op rather than a hot retry.
- [x] C4.4.5 run TP-C4.4-GATES and the complete C4 gate before publication.
- C4.4.5 gates: focused recovery 46/46, C4 core 67/67, broad C4/FM 218/218,
  final full nextest 3131/3131 plus only the named B0 host probe skipped,
  Linux/canonical Windows clippy, Bun 17/17, Python 64/64, fmt/diff and
  operation/staging artifact checks clean. The stale graph was disproved and
  refreshed to 18,793 nodes / 87,788 edges with the production recovery seam,
  exact recovery tests, and `miller_layout`.
- [x] C4.3 real temporary-filesystem tests cover file, directory, symlink,
  collision, replacement race, cycles, swaps, injected rollback failure, and
  exact recovery paths; no `.herdr-rename-stage-*` artifact remains.

## P3 — C5 Agent Handoff

- [x] C5.1 graph-first verification and runtime/client classification of the
  existing pane/agent send, split, start, identity, and cleanup surfaces.
- [x] C5.2 define one typed current-authority handoff intent carrying exact
  path identity and intended terminal/agent identity; stale selection, closed
  FM, unsupported/non-UTF-8 path, reordered rows, and missing target fail
  closed before side effects.
- [x] C5.3 send the selected literal path to an existing intended agent through
  the neutral server/runtime API with quoting, whitespace, metacharacter,
  Unicode, duplicate-name, stale-terminal, and send-failure coverage.
- [x] C5.4 split one terminal and launch Claude through the existing pane
  lifecycle; split failure, spawn failure, early exit, cancellation, and stale
  completion clean up only the newly created resources and never touch the
  stable Herdr/socket or an existing pane.
- [x] C5.5 run focused handoff/failure tests, pane/agent/API regressions, full
  nextest, Linux/Windows clippy, Bun/Python maintenance, isolated runtime proof
  where required, graph freshness, and artifact/diff cleanliness.

- C5.1 verified the existing neutral runtime seams before implementation:
  `App::try_send_terminal_input`, direct-argv `spawn_agent_split`, Workspace/
  Tab spawn rollback, exact pane/terminal identity, `PaneDied`, and detached
  runtime shutdown. Shared runtime facts remain on existing terminal/pane
  state; FM request/selection/presentation remains client-local.
- C5.2 RED/GREEN is `65c3928`/`ec7539d`. One exact current path is bound to the
  focused agent terminal without input-time side effects; bulk, busy, stale,
  missing, non-UTF-8, or lost-agent authority fails closed.
- C5.3 RED/GREEN is `00664c7`/`66b00d7`. The existing intended agent receives
  one atomic UTF-8 path plus one terminal Enter through the shared terminal
  input seam. Shell syntax is never constructed; missing runtime and
  backpressure are visible one-shot failures with no hot retry.
- C5.4 RED/GREEN is `6c6a409`/`f744e4d`. A non-agent source prepares exact FM
  cwd plus workspace/pane/terminal identities, then the scheduled App boundary
  revalidates and creates one `Down` split with direct argv `["claude"]` and
  empty extra env. Focus/FM transition occurs only after the first literal path
  send succeeds. Spawn, stale/cancel, and first-send failures remove only the
  exact newly owned pane/terminal/runtime; retry owns one new pane. Existing
  `PaneDied` cleanup handles early exit without touching the source pane.
- C5.5 evidence: exact C5.4 4/4, related handoff/agent-start/pane-exit 17/17,
  full nextest 3143/3143 plus only the named B0 probe skipped (run
  `418dc969-0218-42f7-8ef3-26ed6c12ec3b`), Linux all-target and canonical
  Windows MSVC clippy, Bun 17/17, Python 64/64, fmt/diff/production-unwrap
  checks clean. The only real test process was test-owned `/bin/cat` (or the
  compile-gated Windows equivalent), shut down by its fixture; stable Herdr and
  socket state were untouched. Fresh graph: 18,854 nodes / 88,064 edges with
  `miller_layout` plus all new split/ownership/rollback symbols.

| Test point | What is tested | Expected result | Reason |
|---|---|---|---|
| TP-C5-AUTHORITY | Selection, target identity, row reorder, FM close/reopen, unsupported path, operation in flight | Only current exact path and uniquely resolved current terminal/agent produce a typed intent; stale or ambiguous authority is consumed without side effect | Display labels and old coordinates must never become agent-input authority |
| TP-C5-SEND | Literal file/directory paths with spaces, quotes, shell metacharacters, Unicode, duplicate agent names, stale terminal, backpressure/send failure | The intended terminal receives one exact literal handoff payload; no shell interpolation, wrong-pane delivery, duplicate send, or silent success | Paths are untrusted text and agent identity can change concurrently |
| TP-C5-SPLIT | Split placement, Claude argv/env, spawn failure, early exit, cancellation, partial setup, retry | Success owns one new pane/process; every failure removes only newly created state and leaves the original layout/session usable | Split-and-launch crosses layout, PTY, process, and agent identity boundaries |
| TP-C5-ISOLATION | Existing stable Herdr/socket, inherited socket variables, throwaway XDG runtime, stale callbacks | Tests use only isolated runtime state; stable processes and sockets are untouched; stale completion cannot attach to a new pane generation | Manual/runtime verification must not corrupt the user's active Herdr session |
| TP-C5-GATES | Focused failure families, API/pane regressions, full platform and maintenance gates, graph/artifact checks | Every applicable gate passes with only the named B0 probe skipped and no leaked pane/process/temp artifact | Handoff is not complete until failure cleanup and cross-platform behavior share fresh evidence |

## P3 — C6 Finder-Fidelity Polish

- [x] C6.1 replace the existing `Files` sidebar placeholder with one native,
  bounded, sectioned FM navigation model. Prepare FAVORITES, optional PINNED,
  and LOCATIONS outside render; derive exact item hit areas in `compute_view`;
  route clicks as typed path requests consumed by an App-owned refresh boundary.
- [x] C6.2 pill highlight and current-location marker.
- [x] C6.2a derive one exact current-location visual authority from the open
  `FmState.cwd` and the prepared accessible sidebar item; add no cached
  highlight state and no render-time filesystem work.
- [x] C6.2b render a complete responsive pill plus right-aligned warning/eject
  marker with display-width truncation and explicit narrow/zero-area behavior.
- [x] C6.2c prove navigation/watcher cwd changes, close/reopen, tab switching,
  stale model paths, and the complete C6.2 gate before publication.
- [x] C6.3 integrate header/row/context actions consistently.
- [x] C6.3a inventory every existing header, row-local, context, and plugin
  action kind, label/icon, enabled reason, selection cardinality, and typed
  dispatch seam; define one explicit cross-surface integration matrix.
- [x] C6.3b render and route each surface from current prepared authority while
  preserving its responsive geometry; converge equivalent actions on the
  existing C3/C4 intent path without a second filesystem owner.
- [x] C6.3c prove stale/reordered selection, operation-in-flight, unsupported,
  narrow/mobile, popup close, FM close/reopen, and complete publication gates.
- [x] C6.4 theme, spacing, empty/error states, and visual Finder-parity review.
- [x] C6.4a define palette-role, spacing, separator, focus, disabled, warning,
  and current-location visual tokens across all native FM surfaces.
- [x] C6.4b make empty directory, unavailable path, permission/read-only,
  preview failure, operation failure/recovery, and no-selection states explicit
  without render-time I/O or layout drift.
- [x] C6.4c run buffer-level breakpoint/theme/accessibility assertions plus an
  isolated real-host Finder-parity review; record accepted differences and close
  the v1 A–C visual gate before any deferred architecture work.

| Test point | What is tested | Expected result | Reason |
|---|---|---|---|
| TP-C6.1-MODEL | FAVORITES/PINNED/LOCATIONS ordering, empty optional section, duplicates, inaccessible paths, source cap | Deterministic bounded rows; invalid or repeated source data cannot create ambiguous path authority | Config and mount sources are live, fallible, and potentially unbounded |
| TP-C6.1-GEOMETRY | Item/header/blank rows, narrow and tiny heights, clipping, collapsed sidebar, non-Files tabs, stale prior frame | Only complete visible item rectangles are addressable; every hidden/inert/stale rectangle is cleared | Old coordinates and labels must never become navigation authority |
| TP-C6.1-RENDER | Three section headings, icon/label rows, placeholder removal, truncation, empty state | Files tab renders only prepared state and never reads filesystem/environment during render | Herdr render is pure and must remain deterministic |
| TP-C6.1-NAV | Exact item click, missing/file/unsupported/stale path, request replacement, scheduled consumption | Input prepares one typed exact path only; App refresh revalidates and opens that directory once, while every invalid case is a no-op | Filesystem work belongs in refresh paths, not mouse/render |
| TP-C6.1-LIFECYCLE | FM cwd changes, close/reopen, watcher reconciliation, tab switching | Sidebar current-location authority and Miller cwd cannot diverge; no stale request survives a lifecycle change | Two independently stale directory projections would misroute navigation |
| TP-C6.1-GATES | Focused model/geometry/render/navigation failures, sidebar/FM regressions, full platform and maintenance gates | All applicable checks pass with only the named B0 probe skipped and no stale hit area or filesystem artifact | Finder polish cannot regress core workspace/sidebar or FM safety |
| TP-C6.2-CURRENT | Exact accessible cwd, model-missing path, inaccessible item, closed FM, cwd change, and non-Files tab | Exactly one prepared accessible item whose exact path equals the open `FmState.cwd` receives current-location authority; every stale, missing, inaccessible, closed, or hidden case has no pill | A cached or label-derived highlight could advertise navigation authority for the wrong directory |
| TP-C6.2-PILL | Normal, Unicode, narrow, marker-reserved, and zero-width item rows | Both pill caps remain complete, label truncation uses display-cell width, trailing markers never overlap the pill, and insufficient space omits the whole pill instead of painting a clipped current signal | Private-use icons and responsive sidebar widths make byte-count or partial-decoration rendering unsafe |
| TP-C6.2-MARKER | Accessible, inaccessible, ejectable, and inaccessible-plus-ejectable rows at normal and narrow widths | Inaccessible rows show a right-aligned warning; accessible ejectable rows show eject; warning takes precedence when both flags are present; every marker stays inside the row | Access failure is stronger safety information than a removable-media affordance and must not be hidden by it |
| TP-C6.2-LIFECYCLE | Sidebar navigation, watcher-driven cwd transition, FM close/reopen, model refresh, and tab switch | The next frame derives styling from current FM/model state with no stale highlight cache; render performs no filesystem or runtime mutation | Cwd and sidebar projection change independently, so visual authority must converge without another lifecycle owner |
| TP-C6.2-GATES | Focused authority/layout/render/lifecycle failures, broad sidebar/FM regressions, full platform and maintenance gates, graph and artifact checks | Every applicable gate passes with only the named B0 probe skipped; no production `unwrap()`, temp residue, stable Herdr/socket access, or user-process change occurs | Finder styling crosses state projection, Unicode geometry, rendering, and existing click authority despite having no filesystem side effect |
| TP-C6.3-CATALOG | Header Copy/Paste/New Folder/Delete, row Send to Agent/Rename/Delete, built-in context actions, plugin file actions, zero/one/many selections, and enabled/disabled reasons | One durable matrix names which action appears on each surface, its prepared authority source, selection cardinality, and exact typed dispatch seam; no action is inferred from a label or icon | Existing safe actions were built in separate modules and visual integration must not create a second semantic catalog |
| TP-C6.3-AUTHORITY | Equivalent action from every supported surface; stale/reordered/missing/unsupported/read-only selection; operation in flight; disabled plugin | Equivalent actions converge on the existing C3/C4 request and scheduled revalidation path exactly once; every invalid case is consumed without filesystem work, duplicate dispatch, or popup/focus corruption | Surface consistency is unsafe if identical-looking controls have different authority or mutation owners |
| TP-C6.3-GEOMETRY | Normal/narrow/zero desktop and mobile layouts, hidden header/row controls, context popup at every edge, long Unicode labels, sidebar expanded/collapsed | Only complete visible controls receive hit geometry; labels/icons truncate by display cell; hidden/clipped/gap coordinates stay inert and no surface overlaps Miller dividers or terminal targets | Integrating four responsive surfaces can reintroduce independent arithmetic and phantom targets |
| TP-C6.3-LIFECYCLE | Watcher reorder/delete, selection change, popup open/close, operation start/finish/cancel, cwd transition, FM close/reopen, stale prior frame | The next compute/input boundary derives every surface from current prepared state; stale geometry and menu/action intent clear deterministically with no cached render authority | Each source state can change between paint and input, so integration requires one fail-closed convergence rule |
| TP-C6.3-GATES | Exact matrix/authority/geometry/lifecycle failures, broad C1–C5/FM/sidebar/context/plugin regressions, full Linux/Windows/Bun/Python gates, graph and artifact checks | All applicable checks pass with only the named B0 probe skipped; no production `unwrap()`, stable process/socket access, duplicate operation owner, or residue exists | C6.3 reconnects every prior action surface and therefore needs cross-module evidence rather than visual happy paths |
| TP-C6.4-THEME | Default and alternate palettes, focus/selection/current/disabled/warning/error/progress roles, narrow and Unicode rows | Every semantic state uses an existing palette role with readable contrast and deterministic precedence; no literal one-off color becomes hidden authority | Final polish must remain theme-safe and must not encode semantics only in ad hoc color choices |
| TP-C6.4-EMPTY-ERROR | Empty directory, no selection, unavailable/permission/read-only cwd, preview unavailable/truncated/error, operation failure/partial/uncertain recovery | Each state has explicit stable copy and bounded geometry, preserves actionable recovery evidence, and performs no filesystem/runtime mutation during render | Empty and failure states are the production path users see when mounts, permissions, previews, or operations fail |
| TP-C6.4-VISUAL | One/two/three-column breakpoints, expanded/collapsed Files sidebar, header/row/context/progress/modal composition, desktop/mobile, isolated real TUI | Buffer assertions and isolated screenshots show coherent Finder-inspired hierarchy, spacing, alignment, and focus; accepted platform/font differences are documented instead of silently ignored | Cell-level correctness can still compose into a confusing or inaccessible full screen |
| TP-C6.4-GATES | Theme/breakpoint/empty/error buffers, full FM/sidebar regressions, isolated runtime parity and cleanup, complete direct `just check`, graph/artifact/diff checks | V1 A–C closes only with all automated gates green, isolated runtime residue zero, and no stable Herdr/socket/user-process change | C6.4 is the final v1 quality gate and must establish both deterministic correctness and real-host usability |

### C6.3a Cross-Surface Action Matrix

This matrix is the durable semantic inventory for C6.3. Labels and glyphs are
presentation only; input dispatches the typed action enum and revalidates the
prepared authority at the existing App-owned boundary. `Unsupported` means the
control must render disabled and emit no intent because v1 has no matching
C3/C4/C5 owner; it must never look enabled while silently doing nothing.

| Surface | Typed action | Label / glyph | Cardinality | Prepared authority | Exact dispatch seam |
|---|---|---|---|---|---|
| Header | `Copy` | `[copy]` | one or many | action-bar exact selection paths | C3 intent → C4 scheduled clipboard controller |
| Header | `Paste` | `[paste]` | zero selection; non-empty clipboard | action-bar cwd writable + clipboard snapshot | existing C4 paste preflight → bounded worker |
| Header | `NewFolder` | `[new folder]` | zero selection | writable cwd only | `Unsupported` in v1; disabled, no request |
| Header | `Delete` | `[delete]` | one or many | action-bar exact selection paths | C3 intent → C4 delete confirmation → scheduled worker |
| Row | `SendAgent` | `>` | exactly one anchored current path | exact row hit path projected into a cloned current FM, validated, then applied as prepared selection | C3 intent → C5 scheduled handoff/split authority |
| Row | `Rename` | `r` | exactly one anchored current path | exact row hit path projected into a cloned current FM, validated, then applied as prepared selection | C3 intent → C4 rename modal → scheduled worker |
| Row | `Delete` | `x` | exactly one anchored current path | exact row hit path projected into a cloned current FM, validated, then applied as prepared selection | C3 intent → C4 delete confirmation → scheduled worker |
| Context | `Open` | `Open` | exactly one | revalidated context snapshot paths/kind | existing A3 enter/navigation state transition |
| Context | `Copy` | `Copy` | one or many | revalidated context snapshot paths/kind | C3 intent → C4 scheduled clipboard controller |
| Context | `Rename` | `Rename` | exactly one | revalidated context snapshot paths/kind | C3 intent → C4 rename modal → scheduled worker |
| Context | `Delete` | `Delete` | one or many | revalidated context snapshot paths/kind | C3 intent → C4 delete confirmation → scheduled worker |
| Context | `Compress` | `Compress` | one or many | revalidated context snapshot paths/kind | `Unsupported` in v1; disabled, no request |
| Context | `SendAgent` | `Send to Agent` | exactly one | revalidated context snapshot paths/kind | C3 intent → C5 scheduled handoff/split authority |
| Context plugin | manifest `Plugin` | manifest title | one or many UTF-8 paths | current enabled/platform-supported manifest + revalidated exact paths | existing plugin action lookup → App-owned plugin command runtime |

The required integration rule is one semantic action → one existing owner.
Header, row, and context variants of Copy, Rename, Delete, and Send to Agent
may differ in geometry, but must converge on the same typed request and current-
state validation. New Folder and Compress stay explicitly disabled until a
separate test-point-first operation owner is approved; C6.3 does not introduce
filesystem mutation inside render or input.

- C6.1 is complete as test-point plan `6464668`, RED contracts `4a65c15`,
  `4836b32`, and `1236f57`, then GREEN product `2bcdf14`. The prepared model
  is capped at 256 exact paths, preserves FAVORITES/optional PINNED/LOCATIONS
  order, deduplicates first authority, and keeps inaccessible pins visible but
  non-clickable. `compute_view` owns complete item rectangles; headers, gaps,
  tiny/collapsed/non-Files/stale geometry are inert. Mouse input replaces one
  typed exact-path intent without I/O; the scheduled App boundary consumes it
  once, revalidates current Files-tab/model/live-directory authority, opens
  `FmState`, and lets the existing watcher bind the new cwd. Tab changes and FM
  open/close clear stale intent. Focused C6.1 is 9/9; combined sidebar/FM
  nextest is 239/239 (run `d7202d9b-ffbc-409d-82f8-76ec191429d3`); full
  nextest is 3151/3151 plus only the named B0 probe skipped (run
  `c5232427-adc0-49b9-9898-daf479b623cd`). Linux/Windows clippy, Bun 17/17,
  Python 64/64, fmt/diff/production-unwrap/temp checks are clean. Fresh graph:
  18,899 nodes / 90,094 edges with `miller_layout` and all new sidebar symbols.

- C6.2 is complete as durable test-point plan `c3dfa6f`, RED contract
  `ac4eecb`, GREEN product `b88fc12`, and test-only lifecycle closure
  `a078d98`. Current-location styling is derived
  every frame from exact open `FmState.cwd` plus one prepared accessible item;
  no cached highlight authority or render-time I/O was added. Complete
  Powerline caps are emitted only when the row can fit the whole pill,
  display-cell truncation handles Unicode, and trailing warning/eject markers
  remain inside the final cell with warning precedence. Focused sidebar/FM
  groups are 11/11, 60/60, and 56/56; full nextest is 3154/3154 plus only the
  named B0 probe skipped (run `3ffc29fb-d053-4a6c-bbda-86b63745fc64`). Linux
  all-target and canonical Windows MSVC clippy, Bun 17/17, Python 64/64,
  fmt/diff/production-unwrap/artifact checks are clean. Fresh graph: 18,909
  nodes / 90,194 edges with `miller_layout` and all three new C6.2 helpers;
  the known MCP extraction crash recovered through the recorded single-worker
  CLI path with zero extraction errors and no process restart.

- C6.3 is complete as matrix commit `2648a08`, RED contracts `a12a870`,
  `9aad978`, `ab27caa`, and `0905e49`, GREEN/fix product commits `40c7ab9`,
  `dd00f25`, `e7614aa`, and `8b21442`, plus scheduled-delete test closure
  `2d974da`. New Folder and Compress are explicitly disabled because v1 has no
  mutation owner. Header and row actions converge on the existing typed
  context intent; Open revalidates then enters through `FmState`; plugin
  actions revalidate the current enabled manifest/action before invoking the
  existing App-owned command runtime once. Invalid row projection does not
  corrupt focus, and stale, unsupported, in-flight, popup-close, FM-close, and
  close/reopen authority is consumed fail-closed. Focused C6.3 nextest is
  118/118 (run `41e5dbf8-576c-4e6b-a7eb-eedd9897121b`); full nextest is
  3160/3160 plus only the named B0 probe skipped (run
  `ec91fccd-12fc-49b9-ae92-0d464de19552`). Linux all-target and canonical
  Windows MSVC clippy, Bun 17/17, Python 64/64, fmt/diff/production-unwrap/
  temp-artifact checks are clean. Fresh graph: 18,922 nodes / 89,277 edges;
  `ready` was cross-checked with current snippets for `miller_layout`, row
  dispatch, scheduled Open, and scheduled plugin execution.

### C6.4a Semantic Visual-Role Matrix

This matrix is the durable C6.4 palette and precedence contract. Every value is
an existing `Palette` role; C6.4 must not add literal RGB colors or infer state
from rendered glyphs. Background filling and geometry remain pure render work,
while filesystem-derived directory state is prepared only by `FmState`
construction/reload paths.

| Surface / state | Foreground role | Background role | Modifier / marker | Precedence and bounded behavior |
|---|---|---|---|---|
| FM canvas | `text` | `panel_bg` | none | Fill only the current FM rect; zero area is inert and hidden terminal content cannot bleed through |
| Identity header | `subtext0` | `panel_bg` | bold | Exact cwd/selection identity truncates before complete action controls |
| Panel title | `overlay1` | `panel_bg` | bold | One complete row; title never consumes list geometry |
| Divider / separator | `surface_dim` | `panel_bg` | single cell line | Exists only at the active Miller breakpoint and never becomes hit authority |
| Cursor focus | `text` | `surface0` | bold only where already established | Cursor remains the sole focus signal; it does not grant bulk authority |
| Explicit multi-selection | `text` | `surface1` | none | Lower precedence than cursor focus; exact prepared paths only |
| Current sidebar location | `panel_contrast_fg` | `accent` | complete two-cap pill | Exact accessible cwd only; complete-or-omitted at narrow widths |
| Disabled action / inaccessible row | `overlay0` | inherited surface | dim for actions | Cannot dispatch; error/warning markers may still override its trailing cell |
| Read-only / warning | `yellow` | inherited surface | stable warning copy/marker | Stronger than ordinary focus/current decoration but weaker than error/recovery |
| Running operation | `yellow` | `surface0` | progress fraction + `Esc cancel` | Current bounded counts only; never obscures exact cwd identity on narrow layouts |
| Completed operation | `green` | `surface0` | terminal summary | Stable terminal evidence until replaced by a later operation |
| Cancelled operation | `peach` | `surface0` | terminal summary | Distinguishes committed work from untouched work |
| Partial / failed operation | `red` | `surface0` | failure counts + recovery hint | Highest semantic precedence; at least one exact recovery path remains inspectable when present |
| Empty / no selection | `overlay0` | `panel_bg` | explicit stable copy | Must differ from missing/unreadable and remain width/height bounded |
| Preview pending / unavailable / truncated | `overlay1`, warning/error as applicable | `panel_bg` | explicit stable copy | Ready pixels have no text underlay; every non-ready state remains distinguishable |

Deterministic semantic precedence is recovery/error > unavailable/warning/
read-only > running progress > cursor focus > explicit selection > ordinary
content. Geometry precedence remains modal/context popup > visible FM controls
> hidden terminal content; C6.4 changes no input authority.

### C6.4 Ordered Microtasks

1. C6.4a RED: characterize the current default/alternate-palette buffer and
   prove semantic token mappings, canvas fill, dividers, cursor, explicit
   selection, disabled actions, sidebar current/warning, and narrow/Unicode
   behavior.
2. C6.4a GREEN: introduce the smallest pure visual-token seam and route all
   native FM surfaces through the matrix without changing action authority.
3. C6.4b RED/GREEN: distinguish empty from missing/unreadable/read-only cwd in
   prepared FM state; render explicit preview and operation terminal/recovery
   summaries with bounded geometry and no render-time I/O.
4. C6.4c RED/GREEN: assert complete one/two/three-column, expanded/collapsed
   sidebar, context/modal/progress composition for desktop/mobile and alternate
   palettes; document accepted terminal/font differences.
5. C6.4 closure: run the isolated throwaway-XDG Finder-parity review, complete
   direct `just check` equivalent, canonical Windows lint, graph freshness,
   production-unwrap, diff, and artifact gates; publish only fast-forward to
   CyPack feature/master.

C6.4 is complete through plan `5b8f327`, semantic RED/GREEN
`2362751`/`3e73351`, directory-state RED/GREEN `4ed210e`/`37f760d`, status
RED/GREEN `04b8070`/`792c4d8`, preview RED/GREEN `3f9a0cd`/`101809c`, and
composition/test closure `03aeb6d` plus `f52cb85`. Final nextest is 3171/3171
with only the named B0 real-host probe skipped; Linux/Windows clippy, Bun
17/17, Python 64/64, fmt/diff/production-unwrap/artifact checks are clean.
The isolated headless API and 120x30 real PTY both used cleared Herdr identity
and socket variables plus throwaway XDG roots, exited semantically, and left
zero process/socket/temp residue. The PTY capture proved the complete sidebar,
header, PARENT/CURRENT/PREVIEW, selection copy, and row-action composition;
pixel/font differences remain host-owned, while exact colors and geometry are
covered by deterministic alternate-palette and breakpoint buffer tests.

## P4 — Deferred UI Architecture

- [x] P4.0 run the post-v1 architecture evidence gate before selecting any
  deferred implementation module.
- [x] P4.0a trace current component/page, `ShellLayout`, modal/context popup,
  and Miller navigation ownership; inventory concrete duplication, coupling,
  persistence, migration, focus, and nested-popup pressure.
- [x] P4.0b name characterization tests for every behavior a candidate refactor
  would protect, including adversarial identity/width/restore and popup close
  ordering; do not edit production code during the evidence pass.
- [x] P4.0c publish one explicit GO/NO-GO matrix and activate at most one of
  S5, S6, S7, or N2. A NO-GO leaves the candidate deferred with the missing
  trigger recorded instead of manufacturing abstraction work.
- [ ] S5 ComponentRegistry only when a second real component/page proves the
  abstraction; do not build a speculative registry.
- [x] S6 activation gate superseded by later explicit product demand and
  absorbed into active SF0-SF6. Implementation remains open in the active
  program above.
- [ ] S7 popup stack with ownership, focus, close ordering, and nested popup
  tests.
- [x] N2 dynamic/unbounded Miller state machine evaluated after v1 A–C;
  implementation NO-GO because the bounded current projection already matches
  the inspected references and an arbitrary visible chain adds no proven value.
- [x] N2.0 define an implementation-ready Miller navigation product contract
  before production code; this is the only active post-P4.0 discovery lane.
- [x] N2.0a compare at least two independent ranger/Joshuto/Yazi-class
  navigation references against Herdr's current cached parent/current/preview
  behavior; name the user-visible delta instead of copying an architecture.
- [x] N2.0b specify the bounded client-local transition model for enter, leave,
  cursor movement, watcher refresh, path disappearance, root, narrow width,
  selection, preview generation, and close/reopen; add no server/protocol state.
- [x] N2.0c write exact RED-capable characterization/test points, complexity and
  cleanup budgets, accepted static behavior, and a final implementation GO/NO-GO.
- [x] N2.1 implement only the proven path-stable parent-return delta from
  `.codex/evidence/n2-path-stable-miller-navigation.md`.
- [x] N2.1a RED: add exact departed-path focus, preview/selection, reorder/delete,
  viewport, missing/hidden, and root-no-op tests in `src/fm/mod.rs`; run and
  record the expected current cursor-zero failures before production edits.
- [x] N2.1b GREEN: make `FmState::leave()` focus the exact departed visible child
  after the existing reload, with deterministic top/clamp fallback and no new
  retained state, I/O pass, dependency, runtime owner, or render mutation.
- [x] N2.1c REFACTOR/GATES: run exact tests, all FM tests, full direct `just
  check` equivalent, production-unwrap/diff/artifact scans, then refresh the
  graph and publish only through the CyPack fork FF workflow.
- [x] N2.2 activation gate superseded by later explicit horizontal/Finder-like
  demand and absorbed into FM1-FM4 with finite 32-segment/5-resident bounds and
  a separate approved test-first plan. Implementation remains open above.

| Test point | What is tested | Expected result | Reason |
|---|---|---|---|
| TP-P4-EVIDENCE | Live graph ownership, duplicate geometry/input/render paths, persisted-state pressure, popup nesting, and v2 navigation demand | Every candidate has concrete source/test evidence and a named unmet or satisfied trigger; repository size or aesthetic preference alone cannot produce a GO | Deferred architecture must emerge from real pressure instead of replacing working concrete seams speculatively |
| TP-P4-CHARACTERIZE | Current layout identity, restore/migration, modal focus/close order, and Miller navigation invariants at normal/adversarial widths | The selected candidate has red-capable characterization points before refactor production code; unrelated candidates remain untouched | Broad UI refactors can preserve green unit tests while corrupting identity or lifecycle behavior |
| TP-P4-DECISION | S5/S6/S7/N2 benefit, blast radius, dependency order, reversibility, and complete gate cost | Exactly one candidate becomes active only when its trigger is proven; otherwise all remain deferred with a precise evidence gap | Sequential Git discipline requires one auditable architecture concern, not a mixed speculative rewrite |

### P4.0 Evidence and Decision Matrix

The rows below preserve the historical P4.0 decision snapshot. They do not
override the later explicitly approved bounded SF/FM program: only the former
S6 and N2.2 activation gates are superseded; S5 and S7 remain NO-GO.

| Candidate | Current evidence | Missing trigger / protected behavior | Decision |
|---|---|---|---|
| S5 ComponentRegistry | `Compositor` contains two fixed `Component` layers; `BaseLayer` performs one explicit terminal/FM content swap; no dynamic registration, per-component event ownership, or second page lifecycle exists | A second real independently owned component/page that duplicates render, hit-area, lifecycle, and event routing | Implementation NO-GO; keep the concrete content-swap pattern |
| S6 resizable persisted shell | `ShellLayout::default()` computes only LeftPanel/CenterContent; nested and serde fixtures exist; `SessionSnapshot` already persists the concrete sidebar width/split but no shell tree | A real RightPanel/BottomBar consumer or user-resizable region whose identity, migration, and restore cannot be represented by existing sidebar fields | Implementation NO-GO; preserve current snapshot compatibility |
| S7 popup ownership stack | One `Mode` selects one `OverlayLayer`; `render_modal_shell` has eight callers, `modal_stack_areas` ten, and context/modal transition tests already protect focus/close order | A real simultaneously nested popup that must retain parent ownership while a child opens and closes | Implementation NO-GO; reuse existing modal/context seams |
| N2 dynamic Miller | V1 A-C is closed; current `FmState::enter/leave/reload` already refresh cached parent/current/preview; `miller_layout` owns bounded 1/2/3-column projection; pinned Yazi/Joshuto evidence identifies only departed-child focus as missing | The original arbitrary visible chain has no trigger. Path-stable leave must protect exact identity plus missing/hidden/reorder/delete/root/viewport outcomes without new retained state | Dynamic/unbounded state machine NO-GO; narrow N2.1 path-stable parent return GO |

### N2.0 Test-Point Contract

| Test point | What is tested | Expected result | Reason |
|---|---|---|---|
| TP-N2-DELTA | Current enter/leave/reload behavior versus two independent dynamic-Miller references | The spec names only observable behavior absent from current Herdr; reference architecture and already-working parent/preview refresh are rejected as scope | N2 must add user value rather than replace a green state model with a fashionable one |
| TP-N2-TRANSITIONS | Root, file/directory cursor, enter/leave, watcher reorder/delete, hidden toggle, selection, preview generation, narrow columns, close/reopen | Every event has one bounded client-local transition, stale async work cannot retarget a new cwd, and no server/protocol/filesystem work enters render | Dynamic navigation crosses independently changing path, selection, viewport, and preview authority |
| TP-N2-BUDGET | Maximum retained path/column state, per-event work, refresh frequency, rollback/cleanup, cross-platform filesystem semantics | Explicit finite bounds and recovery outcomes exist before RED tests; no unbounded directory chain, hot retry, or hidden process/runtime owner is introduced | A new state machine is production-safe only when memory, latency, and failure behavior are designed first |
| TP-N2-GO | User-visible delta, implementation size, protected tests, reversibility, and complete gate cost | Exactly one implementation plan is approved or N2 returns to deferred with the missing evidence recorded | Discovery must terminate in a falsifiable decision rather than becoming permanent speculative design work |

### N2.0 Final Decision

- Evidence: `.codex/evidence/n2-path-stable-miller-navigation.md`, including
  pinned Yazi `4dab4803`, Joshuto `d2581fb0`, and Ranger/Yazi primary docs.
- Dynamic/unbounded visible Miller chain: implementation NO-GO. All inspected
  products use a bounded parent/current/preview projection and Herdr already
  provides responsive 1/2/3-column projection plus cached context refresh.
- Path-stable parent return: N2.1 implementation GO. Both source references
  focus the directory just exited when moving to its parent; Herdr currently
  loses that path identity by forcing cursor zero.
- Budget: zero new state fields/history, no extra directory read, one exact-path
  search over the existing snapshot, no protocol/server/render/worker change.
- Exact RED tests and failure-path expectations are frozen in the evidence file
  before any Rust edit.

### N2.1 Closure Evidence

- RED `e433a2f` produced the four expected departed-path failures and two
  passing failure-path characterizations, run
  `eeef105d-a35a-4e68-92f6-885a80c3cee1`. GREEN `c530836` routes ordinary
  reload and parent return through one exact-path-aware refresh seam.
- Exact N2.1 is 6/6 and all `fm::tests` are 65/65. Full nextest is 3177/3177
  with only `path_beta_real_host_probe` ignored, run
  `ac096bcc-80aa-45bb-9a78-954c73543cbe`.
- Linux all-target and canonical Windows MSVC clippy pass with `-D warnings`;
  Bun is 17/17; Python maintenance is 64/64; fmt, diff, added-production-
  unwrap/debug-marker, and ignored inventory checks are clean.
- Fresh graph is 18,997 nodes / 89,826 edges and returns current source for
  `FmState::reload_focusing_path` and `FmState::leave`; freshness is not inferred
  from `ready` alone.
- No stable process/socket, runtime protocol, persisted state, dependency,
  renderer, input mapping, worker, or release-doc surface changed. N2.2 remains
  a separately evidence-gated future feature.

## Future Mission Roadmap

The north-star queue is ordered M1 → M2 → M3. Only the named `.0` evidence
lane may activate first; every production module remains NO-GO until its
evidence contract proves non-duplicate user value and freezes RED test points.

### M1 — Focused-Agent Attachment Picker

- [x] M1.0 define the product delta before UI or runtime code. Compare native
  FM `SendAgent`/C5 handoff, CLI `agent_attach`, CLI `agent_send`, pane focus,
  plugin file-context actions, and remote image-drop transport. Decision:
  narrow GO for an existing-agent, single-file overlay picker; evidence is in
  `.codex/evidence/m1-agent-attachment-picker.md`.
- [x] M1.0a name the user story precisely: attach the TUI to an existing agent,
  deliver selected file identities to an existing agent, or create/focus a new
  agent. M1 means only the second case and does not create/focus an agent.
- [x] M1.0b trace server/client ownership, exact public IDs, path encoding,
  target availability, agent-state races, and current rollback behavior through
  `src/cli/agent.rs`, `src/app/file_agent_handoff.rs`, the neutral API, pane
  render/input, and `src/client/mod.rs`.
- [x] M1.0c define exactly one file, 1 MiB including CR, one pending request,
  one explicit send attempt, zero new workers/watchers/resources, literal UTF-8
  path behavior, close/reopen semantics, and exact RED-capable test names.
- [x] M1.1 RED/GREEN: add an agent-only pure `[+]` action model, responsive
  bottom-border geometry, no-color/ASCII render, and configurable `prefix+a`.
  Borderless/narrow/hidden/disabled actions expose no mouse target and render
  never covers `PaneInfo::inner_rect`.
- [x] M1.2 RED/GREEN: add one client-local `Mode::AttachFile` picker with a
  private `FmState` and exact workspace/`PaneId`/`TerminalId` target snapshot;
  tab position is a live projection, not a persistent identity. Use
  Clear-first bounded overlay rendering; route mouse/keyboard before background
  terminal/FM input; accept exactly one current regular UTF-8 file.
- [x] M1.3 RED/GREEN: prepare one typed request in input, then execute at the
  scheduled App boundary through a shared C5 literal terminal-send seam. Never
  shell-interpolate, upload bytes, create an agent/pane, or hot-retry.
- [x] M1.4 reconcile completion/cancel/error against current FM and agent
  identities, prove close/reopen, target-exit/replacement, stale-file, busy,
  oversized/non-UTF-8 and zero-resource-rollback cases, run full gates, refresh
  graph, then publish atomically.

| Test point | What is tested | Expected result | Reason |
|---|---|---|---|
| TP-M1-DELTA | C5 native file handoff versus CLI attach/send and plugin file actions | One user-visible action absent from all existing surfaces is named, or M1 terminates NO-GO | A second button for an existing behavior adds ambiguity rather than capability |
| TP-M1-IDENTITY | Stable selected paths, workspace ID, `PaneId`, `TerminalId`, live tab projection, reorder/delete, target exit/replacement | Activation uses exact live identities and stale snapshots emit no request | Files and agents can both change between render and click; Herdr has no persistent tab ID |
| TP-M1-AUTHORITY | Empty/multiple/unsupported selection, non-agent target, busy operation, disabled plugin, modified input | Every unavailable case has one reason and fails closed before side effects | UI enabled state is advisory unless execution revalidates authority |
| TP-M1-DELIVERY | Spaces, quotes, Unicode, non-UTF-8 paths, Windows separators, missing/directory target, exact size boundary | One regular UTF-8 path remains literal and gets one CR; unsupported paths fail visibly with no shell parsing, loss, truncation, upload, or partial delivery | File attachment is security-sensitive data transport |
| TP-M1-ROLLBACK | First-send failure, cancellation, target exit/replacement, close/reopen | Existing panes/agents/processes are never removed; M1 creates no runtime resource; pending client state and hit geometry clear exactly | Even a client-only multi-stage handoff must not retarget or damage unrelated work |
| TP-M1-BUDGET | One selected path, 1 MiB payload including CR, one pending request, one explicit attempt, zero new workers/watchers | Bounds are enforced before send; busy never hot-retries; stale requests are consumed once | Agent delivery can otherwise become an unbounded queue or memory surface |

### M1 Closure Evidence

- RED/GREEN chain: `948ccf8`, `88f6afa`, `10eb4a4`, `53038fd`, `cffc802`,
  `b6b4121`, `7d3144e`.
- Exact attachment family: 20/20. Full nextest: 3197/3197 with only the named
  B0 real-host probe skipped.
- Linux all-target and Windows MSVC bin clippy pass locked with `-D warnings`;
  Bun 17/17; Python maintenance 64/64; fmt and diff checks clean.
- Full graph refresh: 19,113 nodes / 91,118 edges. Current snippets for
  `miller_layout`, `sync_agent_attachment_delivery`, and
  `compute_agent_attachment_picker_row_areas` prove freshness beyond `ready`.
- M1 adds no dependency, protocol field, persisted runtime fact, pane, process,
  watcher, worker, byte upload, multi-file queue, or generic UI registry.

### M2 — Git Worktree Management Actions

- [x] M2.0 compare the requested FM buttons with existing TUI worktree dialogs,
  API `worktree list/open/create/remove`, CLI commands, and keybinds. Publish an
  action-by-action reuse matrix and final GO/NO-GO before adding FM controls.
- [x] M2.0a classify every fact: repository/worktree/operation state is server-
  owned; button geometry, focus, selection, and confirmation are client-only.
- [x] M2.0b freeze exact repo-root, checkout-path, branch, open-workspace,
  dirty-state, linked-worktree, concurrent-operation, and Windows path cases.
- [x] M2.1 RED/GREEN: add one pure focused-agent `[w]` launcher that routes to
  the existing `OpenExistingWorktree` list/search/open flow. Reuse current
  workspace IDs and canonical open behavior; add no panel, worker, Git command,
  filesystem write, create/remove shortcut, or generic action registry. Exact
  tests are frozen in `.codex/evidence/m2-worktree-management-actions.md`.
- [x] M2.2 implementation NO-GO at M2.0: Create already has existing API
  validation, deferred operation, dialog, context action, and keybind owners;
  M2.1 adds no duplicate mutation path.
- [x] M2.3 implementation NO-GO at M2.0: Remove remains linked-worktree-only
  behind the existing typed confirmation/force sequence; the agent frame gets
  no destructive shortcut and branch deletion remains out of scope.
- [x] M2.4 covered by existing server/API recovery owners plus the M2.1
  fail-closed client lifecycle tests; M2.1 creates no operation to reconcile.
  Complete Linux/Windows/API/runtime/graph/publication gates close M2.

| Test point | What is tested | Expected result | Reason |
|---|---|---|---|
| TP-M2-DELTA | Existing worktree dialogs/API/CLI versus proposed FM actions | Only missing lower-friction behavior survives; duplicate controls close NO-GO | The backend and much of the TUI workflow already exist |
| TP-M2-IDENTITY | Repo root, linked checkout, branch, canonical path, open workspace IDs, stale list row | Every action targets one current server-owned identity; stale/reordered UI emits nothing | Path strings alone are unsafe identity under concurrent Git changes |
| TP-M2-CREATE | Existing/new branch, path collision, invalid/relative path, linked source, concurrent remove, Git failure | Validation fails before mutation; success creates exactly one checkout and reports its identity | Create is a multi-resource operation with collision risk |
| TP-M2-REMOVE | Clean/dirty checkout, open workspace, force stage, missing/replaced path, unrelated directory, branch preservation | Default remove is conservative; force is explicit; unrelated/replacement paths and branches survive | Removal is destructive and TOCTOU-sensitive |
| TP-M2-RECOVERY | API disconnect, deferred worker panic/cancel, app close/reopen, server restart, partial Git artifacts | Final snapshot is truthful and retry/recovery never duplicates or deletes unrelated state | Client completion is not authority for server-owned worktree state |
| TP-M2-PLATFORM | Unix/Windows separators, drive roots, reserved components, symlinks, non-UTF-8 display limits | Shared policy is platform-neutral and OS behavior remains compile-gated/tested | Worktree paths are a known cross-platform boundary |

### M2 Closure Evidence

- Evidence/decision publication: `918f4fc`. RED `dab1e20`; GREEN `0ae6175`.
- `[w]` is pure client-local geometry/render state carrying exact workspace ID,
  `PaneId`, and `TerminalId`; activation revalidates current cached root Git/
  worktree capability and emits only the existing open-dialog intent.
- Exact M2.1 5/5; combined worktree/attachment regression 131/131; full
  nextest 3202/3202 with only `path_beta_real_host_probe` skipped.
- Linux all-target and Windows MSVC bin clippy pass with `-D warnings`; Bun
  17/17; Python maintenance 64/64; fmt and diff checks clean.
- Full graph refresh: 19,534 nodes / 91,017 edges. Current `miller_layout`,
  `compute_agent_worktree_action_area`, and cached capability symbols prove
  freshness beyond `ready`.
- No dependency, protocol, persisted state, worker, watcher, Git command,
  filesystem write, pane/process, create/remove/force path, or generic frame-
  action registry was added.

### M3 — General Panel/Page/Button Interface Evaluation

- [x] M3.0 rerun the P4 architecture evidence matrix after M1 `[+]` and M2
  `[w]` created two real frame-action consumers. The measured trigger failed,
  so M3 is closed implementation NO-GO without production refactoring.
- [x] M3.0a inventory duplicated lifecycle, render, input, focus, hit-geometry,
  persistence, protocol, and cleanup seams. Only small pure geometry/render
  mechanics repeat; lifecycle, authority, focus, and cleanup owners differ.
- [x] M3.0b name characterization tests for current BaseLayer terminal/FM swap,
  modal/context ownership, responsive shell regions, M1/M2 identity, and
  snapshot compatibility. The fresh combined set is 16/16.
- [x] M3.1 interface definition implementation NO-GO: two consumers do not
  share one independently owned lifecycle/event contract, so no trait,
  component registry, action registry, or generic panel/page API is defined.
- [x] M3.2 migration implementation NO-GO: no consumer is moved and no mixed
  ownership state is created. Existing typed seams remain independently
  reversible.
- [x] M3.3 final keep/revert decision: keep the concrete M1/M2 owners. The
  graph remains current at 19,534 nodes / 91,017 edges; targeted
  characterization is 16/16 and there is no product diff requiring full build
  gates or reversion.

| Test point | What is tested | Expected result | Reason |
|---|---|---|---|
| TP-M3-TRIGGER | Number of real consumers and duplicated ownership/lifecycle seams | At least two concrete consumers need the same contract; otherwise implementation NO-GO | Interfaces should be earned by repeated behavior |
| TP-M3-CHARACTERIZE | Base/content swap, event priority, hit areas, modal focus/close, widths, restore identity | Pre-refactor behavior is captured by RED-capable invariants before moving code | Broad UI refactors often preserve happy paths while breaking ownership |
| TP-M3-BOUNDARY | Server facts versus client presentation, platform gates, persistence and protocol | The interface contains only shared client behavior and requires no accidental wire/state migration | A UI abstraction must not deepen server/TUI coupling |
| TP-M3-MIGRATION | One-consumer-at-a-time conversion and rollback | Every atomic step stays green and independently reversible; mixed partial ownership is forbidden | Sequential Git discipline reduces refactor blast radius |
| TP-M3-DECISION | Complexity, duplication, coupling, binary/test cost, graph impact | Keep only measured improvement; otherwise revert and record the missing trigger | Passing tests alone do not prove an abstraction is beneficial |

### M3.0 Final Decision

- Evidence: `.codex/evidence/m3-general-ui-interface.md`.
- The two compute functions share focused-agent/border geometry, and the two
  21-line render functions share a bounded ASCII draw template. M2 separately
  owns workspace/Git capability and an existing-dialog intent; M1 owns a
  private picker overlay and one-shot delivery lifecycle.
- `BaseLayer` remains one terminal/FM content swap, `OverlayLayer` remains one
  `Mode`-selected owner, and `ShellLayout` gains no new persisted region.
- Action-area/request symbols do not cross into persistence or wire protocol.
  Mobile clears both derived targets every frame; desktop recomputes them.
- Fresh characterization: 16/16, nextest run
  `32ca7f37-b65c-45ef-9dbf-548e8263d383`. No retry and no production edit.
- Final decision: general interface/registry implementation NO-GO. A third
  exact frame action may earn a private pure draw helper, but a registry still
  requires duplicated lifecycle, focus/close ownership, and event routing.

### Future Ordering and Activation

1. M1.0–M1.4 are complete through the atomic RED/GREEN chain and full closure
   gates above. The exact single-file existing-agent scope remains frozen.
2. M2 is complete: duplicate management implementations remain NO-GO and the
   focused-agent launcher reuses the existing open dialog without a private
   TUI runtime or mutation path.
3. M3.0 is closed implementation NO-GO: M1 `[+]` and M2 `[w]` duplicate only
   small geometry/render mechanics, not lifecycle/ownership. S5–S7 remain
   NO-GO until their existing concrete triggers are independently met.
4. N2.2 remains separate from M1–M3 and inactive until cursor-history demand and
   finite eviction/restore semantics are proven.

## Ordering Resolution

A4, B0, B1, the A3 remainder, B2, C1, N3, C2, N4.2, C3.1, C3.2, C3.3,
C4.1, C4.2, C4.3, C4.4.1 progress, C4.4.2 cancellation, C4.4.3
reconciliation, C4.4.4 recovery, C4.4.5 gates, and C5.1–C5.5 are complete.
C6.1–C6.4, P4.0, N2.0, and N2.1 are complete through product commit `c530836`
plus the continuity commit containing this closure. S5–S7 and the original
dynamic/unbounded N2 state machine remain evidence-gated implementation NO-GO.
M1.0–M1.4 and M2.0–M2.4 are complete. M3.0–M3.3 are closed implementation
NO-GO by `.codex/evidence/m3-general-ui-interface.md`: `[+]` and `[w]` share
only small pure geometry/render mechanics, not lifecycle/ownership. N2.2 and
S5–S7 remain independently gated future work with explicit activation
criteria; no speculative production lane is active.
