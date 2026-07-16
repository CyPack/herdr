# Shell Foundation SF4.2 Input Router Progress Evidence

Updated: 2026-07-16 CEST

## Scope and Boundary

SF4.2 introduces one frozen semantic input precedence shared by mouse and
keyboard. It does not render AppDock, migrate Files rendering, change
protocol/server identity, or touch watcher/preview/operation behavior. The
SF4.1 typed Stage closure (`944a9d4c`) and every earlier baseline stay frozen.

## Test-Point Contract (approved SF4.2 REDs)

| ID | Test | Expected result | Reason | Status |
|---|---|---|---|---|
| SF4.2-01 | `shell_input_router_follows_frozen_precedence` | Table-driven: overlay -> capture -> topmost hit -> focused component -> page -> global -> fail-closed, exactly one owner per row | Precedence must be a typed total authority, not implicit if-chain ordering | GREEN |
| SF4.2-02 | `overlay_blocks_every_background_mouse_action` | Every active overlay kind consumes background-targeted mouse events with zero background action | Topmost blocking overlays prevent every background route (design spec) | PENDING RED |
| SF4.2-03 | `overlay_blocks_background_keyboard_shortcut` | No global/page shortcut acts while a blocking overlay owns focus | Keyboard parity for the same rule | PENDING |
| SF4.2-04 | `capture_owns_move_and_up_outside_original_rect` | Active capture receives move/up even outside its origin rect | Only one owner captures a pointer gesture | PENDING |
| SF4.2-05 | `focus_restores_after_overlay_close` | Closing an overlay restores the previous valid focus owner or template fallback | Deterministic focus restoration | PENDING |
| SF4.2-06 | `collapsed_or_inert_region_cannot_receive_focus` | Collapsed/zero regions expose no focusable target | Hidden geometry must not own input | PENDING |
| SF4.2-07 | `stale_hit_generation_fails_closed` | A hit resolved against a non-current `ShellView` generation is consumed inert | Old coordinates must never become authority | PENDING |
| SF4.2-08 | `files_stage_blocks_hidden_terminal_input` | With Files active, no event reaches hidden terminal targets | Fixes the reported curtain/input leak class | PENDING |

## SF4.2-01 Atomic TDD Evidence

- RED `92777e23` (`test: define shell focus and input ownership`), run
  `67730dbd-a529-4cae-96eb-25a0056f7473`: compiled, ran exactly one test, and
  failed only on the first frozen-precedence row (`LegacyImplicitChain` versus
  `TopmostOverlay`). The table encodes all seven design-spec tiers, including
  the fail-closed no-owner row.
- GREEN `f4f5e3cb` (`feat: route shell input through semantic ownership`):
  - Pure `route_shell_input(ShellInputRouteContext) -> ShellInputOwner` in
    `src/app/input/shell.rs`; total by construction, fail-closed default.
  - `AppState::shell_key_input_owner()` projects current keyboard ownership:
    topmost overlay = `ContextMenu | ConfirmFileDelete | RenameFile` modes,
    active capture = `shell_resize_active()`, focused component = open FM or
    `AttachFile`, remaining input = global dispatch. Keyboard has no
    positional hit; v0 has no page/template shortcut owner yet.
  - `App::handle_key` now selects its tier through the router with behavior
    preserved exactly; the former mode-dispatch tail became
    `handle_global_key_dispatch`. Granted-but-unmappable tiers consume
    fail-closed with `debug_assert` documentation.
- GREEN exact run `f3dc189a-5f79-477e-8ed5-8c1efad91b3a`: 1/1.
- Broad input regressions (app::input, handle_key, context menu, shell
  resize, FM keys, attachment): 401/401, run
  `fb209eec-6050-48f2-bbbb-f9a3c9c914da`.
- Full Nextest: 3,301/3,301 passed, one named B0 real-host probe skipped,
  zero retry. `cargo fmt --check`, Linux all-target Clippy, canonical Windows
  MSVC bin Clippy: PASS with `-D warnings`. Bun 5/5 + 12/12; Python 64/64;
  diff and added-production-`unwrap()` checks clean.
- Publication: sequential fast-forward pushes only to CyPack; both refs equal
  exact SHA `f4f5e3cbb65e391b073c57ebe750d10dddb5d9b1`; `upstream` untouched.
- Post-publication sequential graph refresh: 20,410 nodes / 93,605 edges.

## SF4.2-02 Reconnaissance (recorded before its RED)

Current mouse ownership is distributed, not total:

- `App::handle_overlay_mouse` consumes only six overlay modes (`AttachFile`,
  `ConfirmFileDelete`, `ReleaseNotes`, `ProductAnnouncement`, `Navigator`,
  `KeybindHelp`) and returns `false` for every other mode.
- `ContextMenu`, `GlobalMenu`, `Settings`, `Rename*`, `ConfirmClose`,
  `OpenExistingWorktree` mouse ownership lives inside
  `AppState::handle_mouse` (`src/app/input/mouse.rs`) behind per-branch mode
  guards.
- Leak candidates observed in `handle_mouse_without_agent_frame_action`
  (`src/app/input/mod.rs`): the sidebar-divider double-click branch has no
  mode/overlay guard, so a double left-click on the divider can execute
  `reset_sidebar_resize_to_preferred()` while `Mode::ContextMenu` is open and
  the same event also reaches the menu's outside-close path; wheel events in
  `Mode::ContextMenu` fall through `AppState::handle_mouse` past the
  left-down-only menu branch toward background scroll handlers.
- The SF4.2-02 RED must prove the blocking rule end-to-end for every overlay
  kind against background targets (sidebar row, divider, pane area, FM row);
  full gesture-family coverage (right/middle/modified, drag, double-click
  timing) remains SF4.3 scope.

## Exact Next Microtask

Write the compile-valid behavior RED `overlay_blocks_every_background_mouse_action`
from the reconnaissance above. It must drive real background targets under
each active overlay kind and fail on an observed background action (for
example the unguarded divider double-click reset), never on setup. GREEN
routes `handle_mouse_without_agent_frame_action`'s overlay tier through the
frozen router before any background branch runs. Do not start SF4.3 gesture
coverage, SF5 AppDock, SF6 Files migration, or change-pipeline T3.1 before
SF4.2 closes.
