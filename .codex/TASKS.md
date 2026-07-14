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
- These remain north-star items and must not preempt the active B1/A3/B2 path.

## Ordering Resolution

A4, B0, B1, the A3 remainder, B2, C1, and N3 are complete through N3.2
product head `267ad91`. The next execution order is C2 → N4 → C3 → C4 → C5
→ C6. S5–S7 and N2 remain evidence-gated deferred architecture, while M1–M3
remain inactive north-star work.
