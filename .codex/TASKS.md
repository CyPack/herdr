# Durable Tasks — Herdr Native FM

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

## P1 — B0 Image Path Beta Spike (Independent Risk Track)

- [ ] B0.1 decode a generated test PNG to RGBA and record dependency/cost.
- [ ] B0.2 construct a synthetic `KittyImagePlacement`/PaneId without touching
  the stable Herdr session.
- [ ] B0.3 prove `encode_graphics_update` framing and lifecycle cleanup.
- [ ] B0.4 render Path Beta in a throwaway Kitty/Ghostty host and capture a
  Path Alpha Yazi-in-pane baseline.
- [ ] Record wiring size, failure modes, image-compare evidence, and B2
  go/no-go. B0 must pass before B2; it does not block B1/A3.

## P2 — B1 Text Preview

- [ ] B1.1 bounded text read in state refresh path; render performs no I/O.
- [ ] B1.2 syntax highlighting with explicit unsupported/binary/encoding paths.
- [ ] B1.3 large-file truncation/lazy policy with byte/line limits and tests.
- [ ] Cross-check render/truncation behavior and pass the full gate.

## P2 — A3 Navigation and Selection Remainder

- [ ] A2.4/A3.2 cursor-follow viewport and scroll state with clamp invariants.
- [ ] A3.3 mouse row hit areas, click dispatch, double-click/enter behavior,
  and zero-width/narrow-layout tests.
- [ ] A3.4 make the visual-selection versus multi-selection scope explicit;
  define state only after test points and C2/N4 dependency review.

## P2 — B2 Image Preview (Blocked by B0)

- [ ] B2.1 bounded decode/downscale path with corrupt/huge image failures.
- [ ] B2.2 construct preview placement with synthetic PaneId and no server/TUI
  protocol coupling.
- [ ] B2.3 add local preview painting beside existing pane graphics encoding.
- [ ] B2.4 per-slot cache/dedup, cleanup, resize, navigation, and stale-image
  generation tests.
- [ ] Require image-compare plus real throwaway host evidence before closure.

## P3 — C1 Header Actions + N3 Action Bar

- [ ] C1.1 named header-button rectangles and action tag enum.
- [ ] C1.2 hit-test dispatch with disjoint geometry and narrow/zero-area cases.
- [ ] N3.1 selection-sensitive persistent action-bar content.
- [ ] N3.2 explicit enabled/disabled states with no hidden side effects.

## P3 — C2 Row Actions + N4 Multi-Select

- [ ] C2.1 split each row into disjoint name/action rectangles.
- [ ] C2.2 map row-button tags to actions without ambiguous hit targets.
- [ ] N4.1 multi-select state and keyboard/mouse semantics.
- [ ] N4.2 bulk toolbar and selection-clear/range invariants.

## P3 — C3 Context Menu

- [ ] C3.1 add file context-menu kind and deterministic item model.
- [ ] C3.2 right-click popup placement/render/close/focus tests.
- [ ] C3.3 define the plugin file-action surface without deepening private
  TUI socket coupling.

## P3 — C4 Safe File Operations

- [ ] C4.1 copy/move outside render, with collision, permission, partial-write,
  cancellation, and cross-filesystem tests.
- [ ] C4.2 trash/delete with confirmation, symlink, missing-path, and rollback
  policy; destructive permanent delete is never implicit.
- [ ] C4.3 rename and bulk-rename validation, conflicts, and atomicity limits.
- [ ] C4.4 bounded progress/cancel lifecycle and watcher reconciliation.
- [ ] Require isolated real-filesystem cross-check and leave no temp artifacts.

## P3 — C5 Agent Handoff

- [ ] C5.1 graph-first verification of the pane/agent API surface.
- [ ] C5.2 send the selected path to the intended agent pane with identity and
  quoting tests.
- [ ] C5.3 terminal split then Claude launch, with failure cleanup and no stable
  session/socket interference.

## P3 — C6 Finder-Fidelity Polish

- [ ] C6.1 native sectioned sidebar.
- [ ] C6.2 pill highlight and current-location marker.
- [ ] C6.3 integrate header/row/context actions consistently.
- [ ] C6.4 theme, spacing, empty/error states, and visual Finder-parity review.

## P4 — Deferred UI Architecture

- [ ] S5 ComponentRegistry only when a second real component/page proves the
  abstraction; do not build a speculative registry.
- [ ] S6 resizable shell regions plus deferred `ShellLayout` persistence,
  restore/migration, and adversarial-width tests.
- [ ] S7 popup stack with ownership, focus, close ordering, and nested popup
  tests.
- [ ] N2 dynamic Miller auto-navigation is v2-only after v1 A–C completion.

## Future Mission — Recorded, Not Active

- [ ] M1 FM-interactive CLI attachment buttons.
- [ ] M2 git-worktree management buttons.
- [ ] M3 general panel/page/button super-interface evaluation.
- These remain north-star items and must not preempt the active B0/B1/A3 path.

## Ordering Resolution

A4 and the separate continuity concern are published. The next execution order
is: run B0 risk spike; implement B1 and A3 remainder; allow B2 only after B0
passes; then execute
C1 → C2 → C3 → C4 → C5 → C6. S5–S7 and N2 remain evidence-gated deferred
architecture, while M1–M3 remain inactive north-star work.
