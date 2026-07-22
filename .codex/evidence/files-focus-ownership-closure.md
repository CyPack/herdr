# Files Focus Ownership Closure Evidence

**Program:** FFO — Files Focus Ownership
**Date:** 2026-07-22
**Branch:** `feat/native-fm`
**Pre-closure product head:** `d85d610e`
**Product contract:**
`docs/superpowers/specs/2026-07-22-herdr-files-focus-ownership-design.md`
**Executable plan:**
`docs/superpowers/plans/2026-07-22-herdr-files-focus-ownership-implementation.md`

## 1. Closure status

| Claim | Evidence | Confidence |
|---|---|---|
| Native Files now has one truthful top-level owner across the Locations Rail and Miller Trail | `FileManagerLocationsFocus::{Rail, Trail}` remains the only region owner; accepted typed Trail input calls `focus_file_manager_trail`; stale/coalesced/blocked input tests remain inert | High (0.99) |
| Up/Down after a Trail click stays in the clicked active Miller column instead of moving the Rail cursor | `ffo_trail_click_transfers_focus_and_next_down_stays_in_clicked_column` plus the owner-first key reducer | High (0.99) |
| Rail-owned file actions have no authority even when an old Trail selection remains resident | `InactiveFocusOwner` is prepared with highest precedence and current state is revalidated at header, context, plugin, rename, and worker boundaries | High (0.99) |
| Exactly one current owner row receives the filled active cursor style | Semantic Ratatui tests plus reviewed VIS-26 Rail-owner and VIS-27 Trail-owner snapshots | High (0.99) |
| FFO added no filesystem hot path, worker, cache, timer, server/protocol/platform field, dependency, or blocking I/O | Source/diff audit in section 8; new `read_dir` calls are test-only before/after nonmutation snapshots | High (0.98) |
| Automated implementation and visual gates are complete | Full Rust, Linux/Windows lint, maintenance, integration, deterministic exporter, and Playwright evidence below | High (0.99) |
| Physical Ghostty/user acceptance is complete | **Pending.** The cleanup-first isolated command and exact acceptance matrix are recorded in section 10; no physical result is claimed before the user runs it | Pending |

## 2. Vocabulary and ownership model

Use these names in future bug reports and plans:

- **Locations Rail:** fixed Home/Desktop/Downloads/configured-location region.
- **Miller Trail:** dynamic root/ancestor/current/child/detail region.
- **Active Miller Column:** `TrailState::active_col()`; subordinate to Trail
  top-level ownership.
- **Focus Owner:** `FileManagerLocationsFocus`; exactly one of Rail or Trail.
- **Focus Cursor:** the single valid row painted with the strong filled style.
- **Accepted Origin / Origin Marker:** last accepted Trail root and its weaker
  Rail context marker; not focus authority.
- **File Action Bar:** Copy/Paste/New Folder/Delete header capabilities.

Prepared resident depth, cursor identity, active column, top-level region
focus, painted style, multi-selection, and destructive action authority are
separate state axes. No axis is inferred from another.

## 3. Root cause and rejected hypotheses

| Hypothesis | Decision | Evidence | Confidence |
|---|---|---|---|
| H1: accepted Trail row/wheel input mutates subordinate Trail state without transferring top-level ownership | Confirmed | Before GREEN, `handle_file_manager_row_mouse`, `activate_trail_row`, and accepted vertical wheel paths did not call `focus_trail`; the next owner-first key route still selected Rail | High |
| H2: stale or incorrect hit geometry routes the reported click into the Rail | Rejected for the reproduction | current Files generation, typed Trail row, exact column/index/path, and current-frame checks accept the row before activation; stale-frame regression cannot transfer owner | High |
| H3: the terminal corrupts a full Rail highlight into a thin line | Rejected | the old renderer deliberately added `UNDERLINED` to the inactive accepted origin; removing that modifier removes the line deterministically | Certain |
| H4: File Action Bar preparation and dispatch ignore top-level ownership | Confirmed | pre-FFO model accepted only selection/clipboard/operation state and direct dispatch paths could act on resident Trail selection while Rail owned focus | High |
| H5: the fix requires a server/global focus manager | Rejected | the defect and every required consumer are client-local TUI presentation state; server/protocol/platform diffs are empty | High |

Rejected designs:

- infer focus from the last painted highlight;
- transfer focus on hover or mouse move;
- treat resident `deepest()` or Trail `active_col()` as a second top-level owner;
- clear resident Trail selection when Rail takes focus;
- trust paint-time enabled geometry at dispatch time;
- introduce a cache, debounce, new worker, channel, server field, or protocol
  message.

## 4. Commit and exact file-set ledger

| Commit | Concern | Exact paths |
|---|---|---|
| `bf9fcf46` | `docs: define file manager focus ownership` | `docs/superpowers/specs/2026-07-22-herdr-files-focus-ownership-design.md` |
| `0e415d81` | `docs: plan file manager focus ownership` | `docs/superpowers/plans/2026-07-22-herdr-files-focus-ownership-implementation.md` |
| `0549c8aa` | `test: align file manager focus characterizations` | `src/app/input/file_manager.rs`, `src/ui.rs` |
| `3c5f94e4` | `test: specify file manager focus ownership` | `src/app/input/file_manager.rs` |
| `6b18529a` | `fix: transfer file manager focus on trail input` | `src/app/file_manager_locations.rs`, `src/app/input/file_manager.rs` |
| `83fb77ec` | `test: require trail-owned file actions` | `src/app/file_operation_worker.rs`, `src/app/input/file_manager.rs`, `src/ui/file_manager.rs` |
| `de6656e5` | `fix: bind file actions to trail focus` | `src/app/api/plugins/mod.rs`, `src/app/file_operation_worker.rs`, `src/app/file_rename.rs`, `src/app/input/file_manager.rs`, `src/app/input/modal.rs`, `src/app/state.rs`, `src/ui.rs`, `src/ui/file_manager.rs` |
| `680eb194` | `test: specify unified file manager focus styling` | `src/ui/file_manager.rs` |
| `4422f8ae` | `fix: unify file manager focus cursor styling` | `src/ui/file_manager.rs`, `src/ui/file_manager/locations.rs`, `src/ui/file_manager/trail_view.rs` |
| `d85d610e` | `test: cover file manager focus ownership visuals` | `src/ui/visual_fixture.rs`, `tests/visual/files-locations.spec.ts`, reviewed VIS-26 PNG, reviewed VIS-27 PNG |

The closure documentation commit is intentionally not self-referential by SHA;
its conventional subject is `docs: record file manager focus ownership` and
Git is the authoritative identity after publication.

## 5. TDD evidence

### 5.1 Protected baseline

The four pre-change characterizations passed 4/4 in run
`35c726f8-d299-4997-b100-7ecdb7beac06`:

- `flf_mouse_location_click_synchronizes_cursor_and_typed_intent`
- `single_click_selects_current_row_and_refreshes_preview`
- `flf_render_rail_focus_suppresses_trail_cursor_style`
- `flf_render_rail_cursor_wins_and_origin_remains_subdued`

### 5.2 Input ownership RED → GREEN

Commit `3c5f94e4` introduced the behavior-first REDs before production code.
The durable terminal transcript did not retain a separate run UUID after
session compaction, so no UUID is invented. The assertion-level mismatches
were:

- `ffo_trail_click_transfers_focus_and_next_down_stays_in_clicked_column`:
  expected `Trail`, observed `Rail` after a valid exact-row click;
- `ffo_live_empty_trail_click_transfers_owner_without_mutating_trail`:
  expected `Trail`, observed `Rail` while `FmState` itself remained unchanged;
- `ffo_clamped_trail_wheel_transfers_owner_and_requests_one_render`:
  a live accepted clamped wheel remained Rail-owned instead of producing the
  one ownership repaint.

The stale-frame negative remained fail-closed. Commit `6b18529a` added the
single named transition and restored the focused matrix to green without a
filesystem request.

### 5.3 Action authority RED → GREEN

Commit `83fb77ec` made ownership explicit at the action-model call sites and
added direct copy/paste/delete plus stale-header nonmutation tests. The RED was
compilation-level: the old three-argument model had no focus-owner parameter
and `FileManagerActionDisabledReason` had no `InactiveFocusOwner` variant.
That is a valid interface-contract RED; it was not counted as passing behavior.

Commit `de6656e5` added the typed reason, owner precedence, and current-state
revalidation across header/context/plugin/rename/worker boundaries. The new
tests prove no clipboard mutation, confirmation, worker admission, plugin or
rename request, or filesystem mutation under Rail ownership.

### 5.4 Visual RED → GREEN

Focused semantic RED run
`674d410a-31f4-470f-8963-64237d3a6cde` selected 22 tests: 18 passed and four
failed on the intended old contract. The failures covered the old surface
cursor tuple, unequal Rail/Trail focus tuples, and the deliberate origin
underline. During GREEN, run
`6d8d27bb-13f2-4def-9198-0d601656568b` exposed one additional real
composition defect: row-action cells still painted overlay/surface colors
inside an otherwise filled active row. The implementation extended the
semantic active-row style across icon/name/timestamp/action cells.

Focused semantic GREEN run
`ac2ba105-9567-4d54-a991-f85ffd95fce8` passed 25/25. The cursor wins over
entry-kind and multi-selection paint, while inactive selection, branch
context, and origin remain distinguishable.

## 6. Deterministic visual evidence

Two isolated exporter roots were produced by the ignored exact exporter test:

- run A `de4a690e-3d8f-4af5-b41b-b135d595af87`: 1/1;
- run B `442ffd36-fb97-4b2c-9601-d9cfd8b2cc88`: 1/1;
- recursive diff: empty.

Generated JSON hashes:

| Fixture | SHA-256 |
|---|---|
| VIS-26 Rail owner | `3796f223cc8fd64a7fe53243217fd11b526aec26c85cabd5ab63b5888540a12f` |
| VIS-27 Trail owner | `cd24fd27f4c3d81b636ddf1fcda147406cd9bbce43f8b2b7d424dc310fdfb77d` |

Update-disabled Playwright first proved exactly the expected delta:

- VIS-26: 217 pixels (1%) differed only where the old Home origin underline
  was removed;
- VIS-27: baseline absent because this is the new Trail-owner oracle.

The VIS-26 baseline/actual/diff and VIS-27 actual images were inspected
individually. No layout, icon, label, timestamp, order, clipping, or unrelated
palette shift was accepted. Each unique slug was listed first, updated alone,
and rerun. Focused `files-locations.spec.ts` finished 10/10; the full visual
suite later finished 35/35 with updates disabled.

## 7. Fresh automated gates at product head

`just` is not installed on this host. Every current `just check` child was run
directly; the absent wrapper itself is not reported as green.

| Gate | Fresh result |
|---|---|
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --all-targets --locked -- -D warnings` | exit 0, 26.53 s |
| full `cargo nextest run --locked -E 'all()' --no-fail-fast ...` | run `947bae41-4901-4f38-ad9f-5f187dcc4399`, 3,680 passed, 6 skipped, 35.960 s |
| integration asset Bun test | 5 passed, 0 failed |
| plugin-marketplace Bun test | 12 passed, 0 failed |
| Python maintenance modules | 68 passed, `OK` |
| Windows target install | already up to date |
| `LIBGHOSTTY_VT_SIMD=false cargo clippy --bin herdr --locked --target x86_64-pc-windows-msvc -- -D warnings` | exit 0, 14.64 s |
| full Playwright Chromium | 35 passed, 18.5 s, zero diff |
| `cargo build --locked` | exit 0 |

A prior pre-commit full Nextest run
`27aca2e1-4847-4ec1-9826-d6c43eda4acf` also passed 3,680/3,680 with 6 skipped.

After every closure, lesson, continuity, and local reference edit was present,
the complete gate was rerun rather than assuming documentation was inert:

| Doc-aware gate | Fresh result |
|---|---|
| fmt | exit 0, 2.70 s |
| Linux all-target Clippy | exit 0, 0.43 s |
| full Nextest | run `195f02e5-dbc2-4853-a5e3-ea2e09624d5d`, 3,680 passed, 6 skipped, 35.260 s |
| integration Bun | 5 passed, 0 failed |
| plugin-marketplace Bun | 12 passed, 0 failed |
| Python maintenance | 68 passed, `OK` |
| Windows MSVC Clippy | exit 0, target already current |
| full Playwright Chromium | 35 passed, 18.6 s, one worker, updates disabled |

No retry-only green, unrun selected test, snapshot update, or ignored failure
is included in those totals. A final post-commit rerun is still required by the
publication protocol and must be reported from its actual output.

## 8. Architecture and performance neutrality

Command:

~~~bash
git diff b4ac62a0..HEAD -- src/server src/protocol src/platform Cargo.toml Cargo.lock
~~~

Result: empty. No server, wire protocol, platform, dependency, or lockfile
change belongs to FFO.

The added-source audit for `read_dir|metadata|spawn|channel|sleep|debounce|cache`
found only two `fs::read_dir` calls inside the stale-header test. They capture
the test directory before and after dispatch to prove filesystem nonmutation.
No production FFO diff adds enumeration, metadata I/O, spawn, channel, sleep,
debounce, cache, worker, or timer behavior.

The implementation reuses resident state and performs boolean owner
transitions. Accepted input can request a render only when cursor/owner/visible
state changes; stale or coalesced input remains inert. This is a structural
performance-neutrality claim, not a fabricated wall-clock benchmark.

## 9. Codebase Memory evidence

`manage_adr(get)` returned no prior ADR. `detect_changes` incorrectly returned
zero against the pre-FFO anchor, so it was rejected as sole freshness proof.
The supported single-worker CLI refresh completed with 40 changed files, 631
unchanged files, and zero definition/call/usage extraction errors.

Fresh CLI store before closure-doc indexing:

- 24,308 nodes;
- 129,842 edges;
- exact snippet:
  `home-ayaz-projects-herdr.src.app.input.file_manager.focus_file_manager_trail`;
- action snippet: `compute_file_manager_action_bar_model` with explicit
  `focus_owner` and `InactiveFocusOwner` precedence;
- trace: `focus_file_manager_trail` calls pending-request retirement and
  `FileManagerLocationsState::focus_trail`;
- package architecture still separates `app`, `ui`, `fm`, `server`, and
  `platform`.

After the closure and continuity records were present, a second supported
single-worker incremental refresh classified 8 changed, 664 unchanged, and 0
deleted files. Definition, call, usage, and semantic extraction each reported
zero errors. The final doc-aware CLI store is 24,327 nodes / 129,874 edges and
again resolves the exact focus snippet, action model, trace, and package
architecture.

The FFO ADR was written through this fresh CLI store and read back. It contains
the required PURPOSE, STACK, ARCHITECTURE, PATTERNS, TRADEOFFS, and PHILOSOPHY
sections, including rejected alternatives, action-boundary revalidation,
automated evidence, and explicit pending physical acceptance.

The long-lived built-in MCP channel remained on its older 24,217 / 128,975
snapshot and did not resolve the new exact symbol names. It is explicitly
stale; no proxy or user process was restarted.

## 10. Isolated physical acceptance — pending user run

The helper was re-read before recommendation. It uses only
`/tmp/herdr-trail-manual-test`, creates an ownership marker, clears all Herdr
socket/session identity variables, assigns separate throwaway XDG
config/state/runtime roots, semantically stops only its own dev server, waits
for its exact socket, installs EXIT cleanup, and removes only its marked root.
It never resolves `~/.config/herdr` or the stable socket.

One-command launch:

~~~bash
cd /home/ayaz/projects/herdr && HERDR_RENDER_PROF=1 ./.local/herdr-trail-test.sh run
~~~

Manual acceptance matrix:

1. Click Rail; Up/Down moves only the Rail cursor one row.
2. Click a Trail row; Up/Down moves only that active Miller column one row.
3. One slow wheel detent over Trail advances at most one row; a clamped detent
   may transfer focus but must not skip.
4. Right immediately focuses the first child row only over a directory; Left
   returns one resident parent edge, including the former fixed/root column.
5. Rail-owned Copy/Paste/New Folder/Delete look disabled and are inert.
6. Trail click restores only actions eligible for the exact Trail selection.
7. Exactly one filled active row exists; accepted Rail origin has no underline.
8. Dense click/wheel remains smooth with no inert render storm or synchronous
   directory-read burst in the isolated profiler log.
9. Normal exit removes the throwaway socket/root.

Until the user reports this matrix, `TP-FFO-E2E-01` remains pending. Automated
coverage must not be mislabeled as physical UX acceptance.

## 11. Explicit exclusions

- pinned Home/Desktop/Downloads pre-warm cache (separate measurement-first
  FMN-6 lane; no unbounded LRU);
- unrelated application-wide/global focus architecture;
- drag-and-drop;
- hover focus;
- protocol/runtime ownership expansion;
- a new worker, channel, debounce, timer, or filesystem read path;
- changes to already accepted vertical wheel, cursor-only movement, or
  directory-only Right/Left laws.

## 12. Safety and publication discipline

- Stable Herdr process/socket/config was never addressed.
- No terminal, browser, editor, or user-session process was signalled.
- `.superpowers/` remained user-owned, untracked, untouched, and unstaged.
- Every commit used exact-path staging and an approved lowercase conventional
  subject; `git add -A` and force push were never used.
- Publication target is only `origin HEAD:feat/native-fm` on the CyPack fork;
  no upstream push, issue, or PR is authorized.
