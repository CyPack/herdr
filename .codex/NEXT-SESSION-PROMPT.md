# NEXT SESSION TRIGGER — Herdr Shell Foundation / Native FM

Updated: 2026-07-16 CEST

Continue `/home/ayaz/projects/herdr` on branch `feat/native-fm`. This is a
`mid_flight_adoption` continuation. Do not restart the project, discard valid
work, reimplement completed modules, or infer state from chat memory.

## Mandatory Start Order — No Skips

1. Read `/home/ayaz/projects/herdr/AGENTS.md` completely.
2. Read `/home/ayaz/projects/herdr/CLAUDE.md` completely.
3. Use project skill `$herdr-native-fm`.
4. Before executing that skill, read all of its lessons:
   - `.codex/skills/herdr-native-fm/lessons/errors.md`
   - `.codex/skills/herdr-native-fm/lessons/golden-paths.md`
   - `.codex/skills/herdr-native-fm/lessons/edge-cases.md`
   - `/home/ayaz/.codex/skills/_shared/common-errors.md`
5. Load `rust-dev` and its lessons before Rust work. If its symlink still
   resolves to a missing target, report that honestly; do not claim it loaded
   and do not mutate global tooling without separate authorization.
6. Read, in exact order:
   - `.codex/BOOTSTRAP.md`
   - `.codex/CURRENT.md`
   - `.codex/TASKS.md`
   - `.codex/CHANGE-PIPELINE-TASKS.md`
   - `.codex/HANDOFF.md`
   - `.codex/MEMORY.md`
   - `.planning/STATE.md`
   - `.codex/evidence/shell-foundation-sf4-stage-progress.md`
7. Read the approved design/implementation sources relevant to SF4:
   - `docs/superpowers/specs/2026-07-15-herdr-shell-foundation-v0-design.md`
   - `docs/superpowers/plans/2026-07-15-herdr-shell-file-manager-program-plan.md`
   - `docs/superpowers/plans/2026-07-15-herdr-shell-foundation-v0-implementation.md`
   - `docs/superpowers/plans/2026-07-15-herdr-file-manager-post-shell-implementation.md`
8. Before any manual runtime test, read `.local/ISOLATED-DEV-TEST.md` and use
   its throwaway-XDG, cleared-socket contract exactly.

## Mandatory Task-List Trigger

Immediately create/update the in-session task list from every unchecked item
in both canonical registries. Do not summarize away, merge, silently drop, or
renumber tasks.

At this handoff there are:

- 36 unchecked product/Shell/FM/deferred items in `.codex/TASKS.md`.
- 89 unchecked non-product pipeline items in
  `.codex/CHANGE-PIPELINE-TASKS.md`.
- 125 unchecked items total, copied exactly into `.codex/HANDOFF.md` section 8.

Recount them. If counts differ, stop before code and reconcile the registry,
handoff, and current state. Mark only one product microtask `in_progress`:
the first SF6.1 RED. Keep every later product task pending and the
non-product pipeline paused.

Priority is mandatory:

1. P0 ACTIVE: SF6 Files-to-Stage migration. SF4 and SF5 are FULLY
   CLOSED (SF5.1 `cb0c77fd` 7/7; SF5.2 `d031ef26` 7/7 with the SF5
   closure gate incl. Bun/Python). Next: SF6.1 move the Files render
   projection out of the terminal curtain onto the Workspace Stage, then
   SF6.2 lifecycle/input migration, SF6.3 perf/failure/isolated closure,
   then FM1 horizontal Miller viewport and FM2 column drag-resize (the
   custom-layout target). Evidence:
   `.codex/evidence/shell-foundation-sf5-app-dock-progress.md`. The
   custom-layout target guide (scrollable Miller area + edge drag-resize
   columns + SSH performance architecture) lives locally at
   `docs/superpowers/specs/2026-07-17-herdr-custom-layout-architecture-guide.md`.
2. P0 NEXT: SF4.3 -> SF4.4 -> SF5 -> SF6 -> FM1 -> FM2 -> FM3 -> FM4
   -> FM5.
3. P1 PAUSED: `herdr-change-pipeline` T3.1-T10.9, until the current sequential
   product phase closes.
4. P2 LATER, separately authorized: Apps/Desktop, real TopBar/BottomBar/
   RightPanel consumers, btop/Music/terminal app definitions and launcher.
5. P3 trigger-gated: arbitrary S5 ComponentRegistry and S7 popup stack.

Do not jump to a lower priority because it is easier. Do not mix product,
continuity, and pipeline files in one commit.

## Current Verified Truth

- Branch: `feat/native-fm`.
- Verified product head: `d031ef26`
  (`feat: activate dock apps with bounded name popover`, SF5.2 CLOSED —
  SF5 phase closed).
- Matching RED: `406db487`
  (`test: define app dock interaction and popover`).
- Separate test-stability commit `3c853a70` closed the parallel-load
  process-exit suppression flake class in `src/terminal/state.rs`.
- SF0-SF5 are ALL closed (SF4.1 8/8, SF4.2 8/8 at `20f659c1`, SF4.3 6/6
  at `f973740e`, SF5.1 `cb0c77fd`, SF5.2 `d031ef26`). The active phase is
  SF6 (Files-to-Stage migration).
- Closed SF4.1 pairs:
  - `557bcc77` / `6a18f0c7`: default Terminal Stage.
  - `f22bdac4` / `b9180de3`: Files activation history.
  - `96e6cddb` / `d20403d0`: singleton reactivation.
  - `27ad2a79` / `e8ef80ac`: fixed 16-instance bound.
  - `207c9da3` / `f31ab28a`: checked generation exhaustion.
  - `a5e5bace` / `e1c82036`: close restores Terminal.
  - `056f0879` / `f0f32075`: failed open restores exact Stage/focus.
  - `784fdc2e` / `944a9d4c`: stage switches preserve terminal runtime
    (`AppDefinition`/`LaunchPolicy` + pure `StageState::surface_view()`).
- Next tests are not yet written: the SF6.1 catalog from the plan's
  "Task SF6.1" (Files render projection out of the terminal curtain).
- Legacy `AppState.file_manager: Option<FmState>` curtain still renders. Do
  not remove it until SF6.
- `previous_pane_focus` is existing pane history, not the new SF4.2 focus
  router; `overlay_return_mode` is the client-local overlay focus-restore
  seam, never persisted.
- Protocol remains 16. SF4.1 and SF4.2 stayed client-local presentation
  state.
- Full current SF5 CLOSURE gate: Nextest 3,329/3,329 passed plus one
  named B0 skip (`--no-fail-fast`), Linux all-target Clippy, Windows MSVC
  bin Clippy, Bun 5/5 + 12/12, Python 64/64, fmt, diff and
  added-production-`unwrap()` clean.
- Both CyPack refs equal exact SHA
  `d031ef26d65b26967ac758a28da9dc478d996ae0`.
- User-owned `.superpowers/` is untracked and must never be staged or edited.

## Mandatory Git and Remote Audit

Run before edits:

```bash
git status --short --branch
git log --oneline -12
git remote -v
git rev-parse HEAD
git ls-remote origin refs/heads/feat/native-fm refs/heads/master
```

Expected policy:

- `origin` = writable CyPack fork.
- `upstream` = read-only `ogulcancelik/herdr`; never push it.
- Never `git add -A`, force, reset/discard user changes, or publish a RED-only
  remote tip.
- Standing authorization exists for targeted atomic commits and CyPack-only
  fast-forward pushes after fresh verification.

If local/remote SHA differs from `.codex/CURRENT.md`, analyze the drift before
editing. Never overwrite remote work.

## Mandatory Codebase Memory / Architecture Audit

Use Codebase Memory MCP before grep for code discovery:

1. `index_status(project="home-ayaz-projects-herdr")`.
2. Search `AppState.try_open_file_manager_with`.
3. Search `miller_layout`.
4. Read the exact transaction source via `get_code_snippet`.
5. Locate and trace the frozen SF1 terminal-runtime preservation test/fixture.
6. Use `trace_path` for runtime ownership and current callers.
7. Use `get_architecture` when ownership is unclear.

Do not trust `ready` alone. The fresh sequential store at handoff is 20,396
nodes / 93,372 edges and contains `StageState.surface_view`,
launch-policy-consulting `activate_files`, and `miller_layout`. A new session
must prove current symbols and snippet content before accepting freshness.

If refresh is required, do not restart/kill the proxy or user processes. Use:

```bash
CBM_WORKERS=1 codebase-memory-mcp cli index_repository \
  '{"repo_path":"/home/ayaz/projects/herdr","mode":"fast","persistence":false}'
```

Then verify status, new symbol, old `miller_layout`, and exact source snippet.
Label CLI evidence honestly if the built-in transport remains stale.

## Adoption Checkpoint — Show Before Code

Report all of the following in commentary before editing:

1. Preserved current commits/diffs/user files.
2. A0-A7 status and any gaps for this microtask.
3. Current I0-I14 delivery phase.
4. Current architecture owners: Stage presentation versus terminal runtime.
5. The exact next microtask and why it is the smallest dependency-safe slice.
6. RED test name, what it tests, expected result, and reason.
7. Failure/setup conditions that do not count as RED.
8. Owned files and forbidden files.
9. RED/GREEN commit boundaries.
10. Verification and CyPack-only publication boundary.

## Exact Next TDD Slice

Write the compile-valid table-driven behavior RED:

```text
shell_input_router_follows_frozen_precedence
```

Test points to announce first:

| Test point | Expected result | Reason |
|---|---|---|
| Precedence table rows (overlay, active capture, overlapping topmost hit, focused component, page shortcut, global shortcut, no target) | Exactly one owner per event following overlay -> capture -> active Stage surface -> shell/page -> global; the no-target row is inert | Input authority must be explicit and total before SF4.3 blocking and SF6 migration rely on it |
| Stale hit generation | Consumed without action | Old coordinates must never become authority |
| Collapsed/inert region focus | Cannot receive focus | Hidden geometry must not own input |
| Hidden background target | No fall-through to hidden terminal input | Fixes the reported curtain/input leak class |
| Recovery (terminal resize, surface close/failure, focus target disappearance, capture cancellation) | One valid owner restored without replay, duplicate action, or stuck capture | The router must fail closed under lifecycle churn |

Follow `docs/superpowers/plans/2026-07-15-herdr-shell-foundation-v0-implementation.md`
Task SF4.2 for the complete RED list. Compile failure, reactor panic,
environment/setup failure, rejected/zero-test filter, flaky timing, or an
already-green characterization is not a valid RED.

Planned RED commit:

```text
test: define shell focus and input ownership
```

After observing the correct assertion failure, implement only the minimum
GREEN: one bounded focus/capture router shared by mouse and keyboard, routing
through the frozen `ShellView` hit list. It must not add:

- AppDock rendering;
- Files Stage rendering migration;
- protocol/server/pane/tab/workspace/terminal identities;
- watcher, preview, operation, process or filesystem behavior;
- dependency or snapshot change;
- change-pipeline tooling;
- unrelated refactor.

Planned GREEN commit:

```text
feat: route shell input through semantic ownership
```

Do not push the RED alone. Close the pair locally, then run proportional
overlay/FM-input/sidebar/terminal-input/router tests. Before SF4.2 closure run
broad regressions and the full direct `just check` equivalent.

## Verification Contract

`just` is absent. The applicable direct `check` recipe is:

```bash
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo nextest run --locked --status-level fail --final-status-level fail \
  --failure-output final --success-output never
bun test src/integration/assets/herdr-agent-state.test.ts
(cd workers/plugin-marketplace && bun test)
LIBGHOSTTY_VT_SIMD=false cargo clippy --bin herdr --locked \
  --target x86_64-pc-windows-msvc -- -D warnings
python3 -m unittest \
  scripts.test_agent_detection_manifest_check \
  scripts.test_changelog \
  scripts.test_docs_translation_parity \
  scripts.test_preview \
  scripts.test_vendor_libghostty_vt \
  scripts.test_vendor_portable_pty
```

Also run staged/product diff checks, added production `unwrap()` audit,
ignored-test inventory, private operation/staging residue scan when applicable,
and exact test count. Poll every live command session until explicit exit code.

## Git Publication Contract

1. Target-stage only owned files.
2. Inspect staged names/stat and `git diff --cached --check`.
3. Use lowercase conventional commit, no emoji, no AI co-author.
4. Run fresh proportional gates; full gates before phase publication.
5. Fetch origin feature/master and prove fast-forward ancestry.
6. Push sequentially only to CyPack:

```bash
git push origin HEAD:feat/native-fm
git push origin HEAD:master
```

7. Verify exact remote SHA equality.
8. Reindex graph after committed product changes and prove current symbols/
   snippets, not `ready` alone.
9. Update `.codex/CURRENT.md`, `.codex/TASKS.md`, `.codex/HANDOFF.md`,
   `.planning/STATE.md`, and relevant evidence as a separate continuity commit.

## Non-Negotiable Safety

- Never kill/restart user processes, terminals, browsers, editors, Herdr, or
  MCP proxy processes.
- Never touch installed stable Herdr or inherited stable socket.
- Runtime testing uses cleared Herdr socket/session variables and throwaway
  XDG roots from `.local/ISOLATED-DEV-TEST.md`.
- Never stage/edit `.superpowers/`.
- Never push `upstream`, force, open upstream issues/PRs, or bypass the external
  contributor guardrail.
- No production `unwrap()`.
- Render stays pure; filesystem/runtime work remains in refresh/App paths.
- Topmost overlays consume input; hidden background terminal input is inert.
- Test failure paths, stale identities, capacity/generation exhaustion,
  close/reopen, cancellation, platform cfg, tiny geometry, and resource bounds;
  do not close on happy-path-only evidence.

## Required Handoff Maintenance

Before ending the next session:

1. Leave no RED-only HEAD, failed test, warning, temp artifact, or uncertain
   process/socket state.
2. Update all canonical state/task/evidence files with exact commits, run IDs,
   counts, graph source/count, remote SHAs, and next microtask.
3. Re-extract every unchecked item from both registries into the handoff and
   prove exact diff/count equality; do not summarize away tasks.
4. Keep completed, active, pending, paused, trigger-gated, and later-authorized
   lanes explicitly separate.
5. Produce/update this trigger prompt and open it in a text editor if the user
   requests a new handoff.

Start command:

```bash
herdr-codex
```
