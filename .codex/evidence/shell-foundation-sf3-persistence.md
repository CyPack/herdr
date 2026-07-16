# Shell Foundation SF3.3 Persistence Evidence

Date: 2026-07-16

## Decision

Result: **PASS; SF3 is closed. SF4.1 typed Stage surface state is next.**

SF3.3 raises the session snapshot schema from v3 to v4 and makes the bounded
shell presentation model authoritative for the left-panel width and collapsed
state. Existing v3 snapshots migrate from the legacy sidebar facts, invalid v4
shell preferences are contained without losing the session/workspace payload,
and future snapshot versions remain rejected. The runtime/client wire protocol
remains version 16.

## Test Points and Expected Results

| Test point | Expected result | Reason |
|---|---|---|
| Legacy v3 migration | Sidebar width becomes the bounded LeftPanel preference while sidebar-section and unrelated session state remain intact | Existing users must not lose state during the schema transition |
| v4 idempotent round trip | Decode/encode/decode produces the same typed shell value | A persisted layout must not drift on repeated saves |
| Invalid shell containment | Malformed, over-limit, duplicate, unknown-component, or unknown-template shell data falls back safely while workspaces survive | Optional presentation corruption must not destroy runtime/session authority |
| Future version rejection | A snapshot newer than v4 is still rejected | Forward-incompatible state must fail explicitly |
| v4 authority | Valid v4 shell width wins over conflicting legacy audit fields | Two persistence owners would make restore nondeterministic |
| Preview versus commit capture | Resize preview is absent from capture; one committed transition is captured once | Mouse motion must not create disk churn or transient persisted geometry |
| Collapse round trip | Collapsed visibility and its bounded restore width survive save/restore | Expand must recover the last committed user preference |
| Startup and handoff restore | Scalar compatibility state and aggregate `ShellPresentationState` initialize consistently | Normal startup and handoff must not diverge |

Compile/setup/filter failures were not counted as RED. Every recorded RED
compiled and failed at its intended behavior assertion.

## Atomic TDD Chain

1. Legacy migration contracts:
   - RED commit `da41127f`; 0/2 at the intended assertions, run
     `570315bf-30a0-41c3-8d85-7bbd887f1787`.
   - GREEN commit `be917131`; exact 2/2, run
     `bf29c6b0-8f0e-4181-b8e0-88bde6ca7875`; persistence 59/59, run
     `8b2b069b-d4e9-46c0-a06a-3238a51c20be`.
2. Typed snapshot v4 authority:
   - RED commit `352e394d`; valid future-version failure, run
     `6e598859-f8bb-4aae-864e-7116fd2c38bb`.
   - GREEN commit `385a0bcc`; exact 1/1, run
     `d33c4881-5c39-4883-aa31-6de1a832e72d`; persistence 60/60, run
     `89e28e92-bd49-4e0d-a735-0c58bde7e320`.
3. Corruption containment:
   - RED commits `1b06456e` and `e12e78cf`; 0/4 complete failure matrix, run
     `cfcaf936-e787-4957-a1a4-866c57db10ff`.
   - GREEN commit `d22d0d15`; exact containment 5/5, run
     `3fcb1a41-e447-487e-9572-1fbd27b419bc`; persistence 64/64, run
     `1c274e3b-9660-40fb-ad17-7288ddf1ba3d`.
4. Capture and restore authority:
   - RED commit `6fb8f803`; preview/commit/future contracts passed and the
     conflicting legacy-width assertion failed as intended, run
     `61d63d69-d76a-4626-aac6-719bdf8efec7`.
   - GREEN commit `ef9d7f2b`; exact 4/4, run
     `cc598196-555d-47b6-b81e-7dd2854d59f9`; broad persistence/shell input
     75/75, run `6b1ba9fe-1365-4502-aa63-aa1a3f8ce082`.
5. Collapse preference round trip:
   - RED commit `4dd62047`; missing collapsed restore failed as intended, run
     `75525323-5a55-47be-8e09-de0fcc45fd54`.
   - GREEN commit `90be6893`; exact 1/1, run
     `f82f720c-3578-48bf-bf89-c2e86fc2857d`; complete new snapshot matrix
     12/12, run `4c5094ae-d71c-4496-91d1-2b9e95a701d8`; broad persistence,
     shell, and sidebar input 137/137, run
     `2b56e0bc-e23b-435e-a13c-9eeca5ebca92`.

The product chain is eleven targeted-stage commits and ends at exact SHA
`90be689359988424b2a7c6206ff45a3207422196`.

## Architecture and Boundaries

- `ShellSnapshotV1` persists one schema-versioned, typed, bounded template
  tree with region constraints, component placements, collapse restore widths,
  and pinned built-in app order.
- The snapshot stores stable identities and committed preferences only. It does
  not persist derived `Rect`s, hit areas, geometry generations, mouse capture,
  drag preview, scroll extents, PTY/runtime handles, or filesystem facts.
- v4 shell data is authoritative. Legacy `sidebar_width` remains migration and
  audit input only; v3 uses the bounded `DockSidebarStage` projection.
- `ShellPresentationState` is the client-local owner. Startup and handoff apply
  the same restored width/collapse preference without introducing a new server
  fact or private TUI socket behavior.
- Invalid optional shell data logs a warning and falls back to compatibility
  preferences while preserving session/workspace state. Future top-level
  snapshot versions still return an error.
- `SNAPSHOT_VERSION` is 4. `PROTOCOL_VERSION` remains 16. No dependency,
  network/layout protocol, render mutation, filesystem operation, watcher,
  worker, or new runtime resource was added.
- Capture is proportional to the bounded shell tree and runs at existing save
  boundaries. Resize preview performs zero persistence write and zero PTY
  resize; commit remains the single dirty boundary.

## Fresh Verification

- `cargo fmt --check`: PASS.
- Full repository Nextest: 3292/3292 PASS, one intentional skip, run
  `807c0830-61d4-4058-9fed-55c20b6e184d`.
- Frozen SF1 characterization: 11/11 PASS, run
  `56071869-bc29-47be-964c-9112bc9cb552`.
- Correct ignored-only inventory lists exactly
  `herdr::bin/herdr kitty_graphics::tests::path_beta_real_host_probe`; it was
  listed and not executed.
- Linux all-target Clippy with `-D warnings`: PASS.
- Canonical Windows MSVC binary Clippy with `LIBGHOSTTY_VT_SIMD=false` and
  `-D warnings`: PASS.
- Bun integration assets: 5/5 PASS. Plugin marketplace: 12/12 PASS. Combined:
  17/17.
- Python maintenance modules: 64/64 PASS.
- `git diff --check`, eleven-commit scope, changed-file boundary, and targeted
  worktree inspection: PASS. Every newly added `unwrap()` is below the
  `#[cfg(test)]` module boundary; no production `unwrap()` was introduced.
- The tracked vendor `Cargo.toml.orig` predates this phase, the ignored local
  `.local` backup predates this range, and neither is phase residue. The only
  visible untracked tree is the user-owned `.superpowers/`, which was not
  touched or staged.

No stable Herdr process/socket, installed binary, user process, or real-host B0
probe was contacted, restarted, terminated, or executed.

## Git and Graph Evidence

Every product commit was targeted-stage only. CyPack `feat/native-fm` and fork
`master` both resolve to exact product SHA
`90be689359988424b2a7c6206ff45a3207422196`. `upstream` was not pushed; no
force push occurred.

The pre-refresh graph reported `ready` at 20,246 nodes / 94,411 edges and knew
`miller_layout`, but not `ShellSnapshotV1`; `ready` alone was therefore rejected
as freshness evidence. The supported sequential refresh completed with zero
extraction errors:

```text
CBM_WORKERS=1 codebase-memory-mcp cli index_repository '{"repo_path":"/home/ayaz/projects/herdr","mode":"fast","persistence":true}'
```

The persistent graph is now 20,291 nodes / 94,542 edges. CLI status, exact
search, and source-snippet reads prove current:

- `src.ui.file_manager.miller_layout`;
- `src.persist.snapshot.ShellSnapshotV1`;
- `src.persist.snapshot.SessionSnapshot.restored_left_panel_preference`;
- `src.ui.shell.interaction.ShellPresentationState.from_restored_left_panel`.

The already-running MCP reader still exposes its older snapshot; it was not
restarted or killed. Freshness is grounded in the newly written persistent
store and exact source snippets, not status text.

## Next Gate

SF4.1 begins with graph-first ownership/drift analysis around the existing
terminal/Files swap, tab/workspace identity, `FmState`, and input precedence.
Before production code, write the smallest compile-valid RED
`stage_starts_on_terminal_workspace`. Then proceed one behavior at a time
through typed client-local Stage activation, bounded singleton instances,
generation exhaustion, close/failure restoration, and preservation of the
terminal runtime. No AppDock render, Files Stage migration, or SF4 router work
may be mixed into that first state slice.
