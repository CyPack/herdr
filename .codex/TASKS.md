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
- [ ] C4.4 bounded progress/cancel lifecycle and watcher reconciliation.
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
- These remain north-star items and must not preempt active C4–C6 work.

## Ordering Resolution

A4, B0, B1, the A3 remainder, B2, C1, N3, C2, N4.2, C3.1, C3.2, C3.3,
C4.1, C4.2, C4.3, C4.4.1 progress, C4.4.2 cancellation, C4.4.3
reconciliation, C4.4.4 recovery, C4.4.5 gates, and C5.1–C5.5 are complete
through product head `f744e4d`. The next execution order is C6.1 native
sectioned sidebar → C6.2 pill/current-location styling → C6.3 integrated
header/row/context actions → C6.4 theme/spacing/empty-error/Finder-parity review.
S5–S7 and N2 remain evidence-gated deferred architecture, while M1–M3 remain
inactive north-star work.
