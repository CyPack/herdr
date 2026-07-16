# Shell Foundation SF4.1 Stage Progress Evidence

Updated: 2026-07-16 CEST (SF4.1 closed)

## Scope and Boundary

SF4.1 introduces only typed client-local Workspace Stage identity and lifecycle.
It does not migrate Files rendering, add AppDock, define the complete focus/input
router, change server or wire identity, start a process, access an inherited
Herdr socket, or mutate terminal runtime ownership.

The protected legacy baseline remains the SF1 characterization set, especially
`files_curtain_currently_replaces_terminal_surface`: the curtain is temporary,
but its proof that the terminal runtime survives Files visibility changes is a
frozen migration invariant.

## Test-Point Contract

| ID | Test | Expected result | Reason | Status |
|---|---|---|---|---|
| SF4.1-01 | `stage_starts_on_terminal_workspace` | Default typed Stage resolves to the existing terminal workspace | New client state must preserve the legacy default and create no runtime identity | GREEN |
| SF4.1-02 | `activating_files_records_previous_surface` | Files becomes active and Terminal becomes previous | Close/failure needs deterministic restoration authority | GREEN |
| SF4.1-03 | `reactivating_singleton_files_keeps_one_surface` | Repeated Files activation is an exact no-op | Singleton activation must not duplicate instances or rewrite history | GREEN |
| SF4.1-04 | `stage_rejects_more_than_sixteen_builtin_instances` | The seventeenth instance is rejected without mutation | Fixed capacity bounds memory and failure behavior | GREEN |
| SF4.1-05 | `instance_generation_exhaustion_fails_without_aliasing` | `u32::MAX` cannot wrap or reuse identity | Generation overflow must fail closed | GREEN |
| SF4.1-06 | `closing_files_restores_previous_terminal_surface` | Files is removed and Terminal becomes active | Close must not strand an invalid active ID | GREEN |
| SF4.1-07 | `failed_files_open_restores_previous_surface_and_focus` | Preparation failure restores exact Stage/focus and leaves FM closed | Partial activation must be transactional | GREEN |
| SF4.1-08 | `stage_surface_switch_does_not_destroy_terminal_runtime` | Switch/reactivate/close/failure preserve exact terminal runtime identity/count | Presentation state must not own or destroy runtime state | GREEN |

## Atomic TDD Evidence

1. Default Stage: RED `557bcc77`, GREEN `6a18f0c7`.
   - RED run `377dcfa6-1d92-4a30-94ff-09d642745071` failed on
     `LegacyCenterContent` versus `TerminalWorkspace`.
   - GREEN run `ee7d2f2d-dde6-49bd-b8ac-4c5df6373466` passed 1/1.
2. Files activation history: RED `f22bdac4`, GREEN `b9180de3`.
   - RED run `b9ba622a-088e-41a7-b33e-6b167369ac87` failed because Files did
     not become active with Terminal recorded as previous.
   - Stage run `c1519297-4dee-40cd-8f87-5a7967110fed` passed 2/2; toggle run
     `7e7ec3bb-1d23-4a9c-97c4-69bbdfb983de` passed 2/2; cwd run
     `5f22d4ab-204a-4545-bae6-7e9a14d817e9` passed 1/1.
3. Files singleton: RED `96e6cddb`, GREEN `d20403d0`.
   - RED run `207a6e58-f34d-4c2e-899e-8a7d26cdfb37` observed history mutation
     on the second activation.
   - GREEN run `9b033be3-c450-497c-83c7-d42aaa20b834` passed 3/3.
4. Fixed instance bound: RED `27ad2a79`, GREEN `e8ef80ac`.
   - RED run `1590d626-3703-4797-96f6-0fcd5273b05c` accepted the seventeenth
     instance.
   - GREEN run `62001bb3-e1c7-4b7d-a523-5323f528de51` passed 4/4; toggle
     `9cd8f010-ed12-4b99-bc0c-f1ec247e045a` passed 2/2; cwd
     `e0b5d88b-93e4-4008-8df9-5a68dfaf2b84` passed 1/1.
5. Generation exhaustion: RED `207c9da3`, GREEN `f31ab28a`.
   - RED run `ad50211f-d444-41a0-b92d-32ad2fd04fd1` wrapped `u32::MAX` to 0.
   - GREEN run `70f09452-a146-4c44-8efb-50daea5428e2` passed 5/5.
6. Close restoration: RED `a5e5bace`, GREEN `e1c82036`.
   - RED run `d6249882-e3c3-4dc8-8d49-fff1e3370253` left Files active.
   - GREEN Stage run `b5121cf8-6e53-4172-80dd-49a1c647becb` passed 6/6;
     pending-action run `744f8985-a097-437f-9f0b-f3fa6030b8b9` passed 1/1;
     Esc/q run `80524d63-53a2-441a-88e5-961b47396812` passed 1/1; toggle run
     `d2916b8d-cd59-468f-8269-ab3b7432904c` passed 2/2.
7. Failed-open rollback: RED `056f0879`, GREEN `f0f32075`.
   - RED run `7df14514-5602-42d2-962e-fd5803c038b4` compiled and ran one
     test, then failed only because Files Stage/focus was not restored.
   - The first exact GREEN run
     `a6cf8c27-7338-4d94-910b-1e43df596964` passed 1/1 and exposed a
     dead-code warning. The seam was narrowed so production constructs the
     typed preparation error instead of suppressing the warning.
   - Final Stage run `6fed21bd-e1aa-46f7-90a4-b617e0b7b0a6` passed 7/7.
     Open/cwd/render run `b8487190-8d24-4ceb-b2c5-e7c0556e70ef` passed 3/3;
     close-authority run `10da9a43-b851-489a-b907-b2526439c696` passed 1/1;
     toggle run `6d747dc5-b2e2-4069-bfd6-5f875c477849` passed 2/2.

8. Runtime preservation: RED `784fdc2e`, GREEN `944a9d4c`.
   - RED run `ffb0f3b0-b98e-49f7-8c1d-834455b4b22d` compiled, ran exactly one
     test, and failed only on the missing typed bridge:
     `LegacyFileManagerCurtain` versus `TerminalWorkspace` at
     `src/ui/surface_host.rs:369`. The test extends the frozen SF1 fixture
     (`Workspace::test_new` + `TerminalRuntimeRegistry` +
     `TerminalRuntime::test_with_screen_bytes`, `#[tokio::test]`) and drives
     switch, singleton reactivation, close (including
     `test_process_pty_bytes` usability proof), failed open, and the
     zero-new-identity resource boundary.
   - GREEN adds `AppDefinition`/`LaunchPolicy::Singleton` consulted by
     `activate_files` and the pure `StageState::surface_view()` typed
     projection (`StageSurfaceView`); no AppDock render, Files migration,
     focus router, protocol/server identity, dependency, watcher, process, or
     filesystem behavior was added.
   - GREEN exact run `4c57dd90-1fa1-4904-ac26-f9ab5959db21` passed 1/1; Stage
     exact run `d2e2eeda-c36d-465b-a07b-15bb1d795ff5` passed 8/8.

Compile failures, setup failures, rejected Nextest filters, and zero-test runs
are not counted as RED or GREEN evidence. In particular, run
`85af8790-8739-4f0b-a9b9-0c55426bc53e` selected zero tests and was rejected;
the exact inventory was queried before the successful named runs above.

## Separate Test-Stability Work (parallel-load flake class closed)

The fresh full gate at `b33ee430` failed once on
`terminal::state::tests::missing_session_after_process_exit_waits_for_fresh_process_evidence`
under 452-test parallel load. Root cause: the suppression record is stamped
with real `Instant::now()`
(`suppress_full_lifecycle_hook_report_with_session_ref`), while four tests
captured their synthetic baseline before that stamp with millisecond offsets;
load-induced descheduling inverted the release comparison in
`detected_state_observed_before_release_suppression`. The established
`Instant::now() + Duration::from_secs(1)` idiom (already used by
`fresh_detected_process_keeps_old_session_suppressed_after_process_exit`) was
applied to the four vulnerable tests in test-only commit `3c853a70`
(`test: make process-exit suppression tests deterministic under load`).
Isolated repro passed, focused `terminal::state` was 94/94, and the clean full
baseline was 3,299/3,299 (run `d9690d94-f9c2-48ad-b52e-236135164537`).

## SF4.1 Closure Gate at `944a9d4c`

- Frozen SF1 characterization: 11/11, run
  `d1a7de31-2776-46e3-b66d-7059586675d9`.
- Broad Stage/open/close/toggle/curtain regressions: 15/15, run
  `3956a862-dec4-46dc-8560-921cb32dbf19`.
- Full Nextest: 3,300/3,300 passed, one named B0 real-host probe skipped,
  zero retry, run `5694bdd6-c22f-46ce-86b7-c496aea6e39c`.
- `cargo fmt --check`, Linux all-target Clippy, and canonical Windows MSVC bin
  Clippy with `LIBGHOSTTY_VT_SIMD=false`: PASS with `-D warnings`.
- Bun integration assets 5/5; plugin marketplace 12/12; Python maintenance
  64/64; `git diff --check` clean.
- Added-production-`unwrap()` audit: zero; fixture temp-root scan: zero
  residue; ignored inventory is exactly
  `herdr::bin/herdr kitty_graphics::tests::path_beta_real_host_probe`.
- Publication: sequential fast-forward pushes only to CyPack; both
  `feat/native-fm` and fork `master` equal exact SHA
  `944a9d4cf4ecb92f97e9be80b18060db6c5ffb4d`; `upstream` untouched.
- Post-publication sequential graph refresh: 20,396 nodes / 93,372 edges with
  current `StageState.surface_view`, `activate_files` consulting
  `BuiltInAppId::Files.definition().launch`, and `miller_layout`; freshness
  was not inferred from `ready` alone.

## Fresh Repository Verification at `f0f32075`

- `cargo fmt --check`: PASS.
- Linux `cargo clippy --all-targets --locked -- -D warnings`: PASS.
- Full Nextest run `a9d9e8b1-7a9f-403f-9d34-499d0f13a612`: 3,299/3,299
  passed, one named B0 real-host probe skipped, zero retry.
- Canonical Windows MSVC bin Clippy with `LIBGHOSTTY_VT_SIMD=false`: PASS.
- Bun integration assets: 5/5 PASS.
- Bun plugin marketplace: 12/12 PASS.
- Python maintenance: 64/64 PASS.
- Product diff check and added-production-`unwrap()` audit: PASS.
- Stable Herdr process/socket and every user process: untouched.
- User-owned untracked `.superpowers/`: untouched and unstaged.

`just` is not installed; these commands are the complete applicable lowercase
`justfile` `check` recipe executed directly.

## Codebase Memory Evidence

The pre-refresh built-in MCP snapshot reported `ready` at 20,291 nodes / 94,542
edges but did not contain `try_open_file_manager_with`; therefore `ready` was
rejected as freshness proof.

The supported sequential command completed with zero extraction errors and no
proxy/process restart:

```bash
CBM_WORKERS=1 codebase-memory-mcp cli index_repository \
  '{"repo_path":"/home/ayaz/projects/herdr","mode":"fast","persistence":false}'
```

The fresh graph store is 20,340 nodes / 93,429 edges. CLI `search_graph`
returns both current `AppState.try_open_file_manager_with` and
`miller_layout`; `get_code_snippet` returns the exact Stage/focus snapshot,
typed activation error, failure rollback, success-only stale-navigation clear,
and FM commit source.

The already-open Codex MCP transport still returns the old snapshot. It was
not restarted because session/process safety outranks convenience. A new
session must call built-in `index_status`, then require the new symbol and
exact snippet before trusting that transport.

## Exact Next Microtask

SF4.1 is CLOSED with 8/8 exact contracts, frozen SF1 11/11, broad
regressions, the complete direct `just check` equivalent, publication, and
fresh graph symbols above. The next microtask is the first SF4.2 RED:
table-driven `shell_input_router_follows_frozen_precedence` covering overlay,
active capture, overlapping topmost hit, focused component, page shortcut,
global shortcut, and no target (see
`docs/superpowers/plans/2026-07-15-herdr-shell-foundation-v0-implementation.md`
Task SF4.2). Do not begin SF5 AppDock, SF6 Files rendering migration, FM1
history, or change-pipeline T3.1 before SF4.2-SF4.4 close in order.
