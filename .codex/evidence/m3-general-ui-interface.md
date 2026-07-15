# M3.0 Evidence — General Panel/Page/Button Interface

Date: 2026-07-15

Decision: **implementation NO-GO**. M1 `[+]` and M2 `[w]` are two real frame-
action consumers, but they do not share an independently owned lifecycle or
event contract. The repeated code is limited to small pure geometry/render
mechanics. A trait, component registry, action registry, or general panel/page
interface would parameterize unlike authority instead of removing duplicated
ownership.

## Trigger Result

The P4/S5 trigger requires a second independently owned component/page that
duplicates render, hit-area, lifecycle, and event routing. M1 and M2 satisfy
only the first two mechanical parts:

| Dimension | M1 `[+]` | M2 `[w]` | Shared contract result |
|---|---|---|---|
| Product owner | Single-file attachment picker | Existing open-worktree dialog launcher | Different |
| Derived identity | `PaneId` + `TerminalId` | Workspace ID + `PaneId` + `TerminalId` | Related but not interchangeable |
| Capability | Focused agent, border, width | Focused agent, border, wider frame, cached non-linked Git root | M2 has independent capability authority |
| Lifecycle | Opens and closes private `Mode::AttachFile` picker state | Emits one existing `request_open_existing_worktree` intent consumed by the established dialog | Different |
| Input effect | Opens picker after exact target revalidation | Revalidates workspace, focused pane, terminal, agent status, and cached Git capability | Different |
| Focus/close | Blocking overlay owns input; close restores terminal/navigate mode | Existing dialog owns search, focus, close, empty/error behavior | Different owners |
| Cleanup | Clears picker and pending delivery state | `Option::take()` clears only the queued dialog request | Different |
| Persistence/protocol | Derived `ViewState` only | Derived `ViewState` plus transient App request only | Neither crosses persistence or wire boundaries |

Result: two buttons exist, but there are not two instances of one lifecycle.
TP-M3-TRIGGER therefore fails and production abstraction remains forbidden.

## Quantified Duplication

- `compute_agent_attachment_action_area` is 42 lines; the worktree counterpart
  is 48 lines. They share terminal mode, focused-pane, bottom-border, minimum-
  height, exact terminal lookup, agent-status, and bottom-row placement checks.
  M2 additionally owns workspace lookup, cached Git/worktree capability,
  workspace identity, a wider joint breakpoint, and a left-shifted rectangle.
- The two render functions are 21 lines each and have the same bounded
  `set_stringn` template. Their only semantic inputs are the typed area, width,
  and ASCII token. This is real mechanical repetition, but extracting a helper
  would not remove lifecycle, identity, capability, or dispatch ownership.
- `App::handle_mouse` has two intentionally separate branches. M2 must consume
  hits before M1 because `[w]` is adjacent to `[+]`, then revalidate current
  workspace/Git authority and emit the existing dialog intent. M1 revalidates
  the exact pane/terminal target and opens a private picker. A generic callback
  or enum router would centralize unlike effects without reducing their checks.
- `compute_view_internal` derives both optional hit areas every desktop frame;
  mobile explicitly writes both as `None`. No retained registry or cleanup
  owner is missing.

The only plausible extraction is a private pure draw helper. With two three-
cell actions it would replace two small explicit functions with a parameterized
function plus two typed call sites, while leaving all meaningful duplication
unchanged. It is not an M3 interface trigger. Re-evaluate a local helper only
if a third action repeats the exact bounded render contract.

## Current Architecture and Boundaries

- `Compositor` still has two fixed layers. `BaseLayer` performs one explicit
  terminal/native-FM `CenterContent` swap. M1 and M2 render inside terminal
  pane borders and do not create a second page/component owner.
- `OverlayLayer` still selects exactly one overlay by `Mode`. M1 reuses it as
  `AttachFile`; M2 reuses the existing `OpenExistingWorktree` mode. There is no
  nested popup ownership demand.
- `ShellLayout::default()` still owns only `LeftPanel` and `CenterContent`.
  M1/M2 add no right/bottom region, resize persistence, or restore migration.
- The action-area types occur only in App/UI code. No matching symbol exists in
  `src/persist` or `src/protocol`; no dependency, server state, socket message,
  worker, watcher, filesystem operation, process, pane, or Git command is
  introduced by M3.0.

TP-M3-BOUNDARY passes because the current concrete design stays client-local.
Introducing a general TUI-named runtime/API abstraction would weaken that
boundary and is explicitly rejected.

## Protected Characterization Set

The following existing tests are the RED-capable invariants for any future
candidate. They were run together at exact head `a61cfb6`:

| Protected behavior | Tests |
|---|---|
| Base terminal/FM content swap | `open_file_manager_renders_directory_list_in_center` |
| Desktop/mobile named shell projection | `desktop_shell_regions_match_computed_geometry`, `mobile_view_leaves_shell_regions_empty`, `nested_tree_lays_out_all_regions_and_survives_degenerate_area` |
| M1/M2 capability, responsive geometry, disjoint placement, bounded ASCII/no-color render | four focused pane-action geometry/render tests in `src/ui/panes.rs` |
| Exact stale-identity input authority | attachment and worktree frame-click revalidation tests |
| Existing M2 lifecycle and cleanup | existing-dialog routing and list-error resource-preservation tests |
| Overlay/background and context ownership | attachment-picker blocking plus context-menu keyboard/mouse lifecycle tests |
| Snapshot compatibility | `old_snapshot_defaults_sidebar_fields` |

Fresh result: **16/16 passed**, 3,187 skipped, zero retry; nextest run
`32ca7f37-b65c-45ef-9dbf-548e8263d383`.

The full exact-head M2 closure remains 3,202/3,202 with only the named B0 real-
host probe skipped, Linux/Windows clippy clean, Bun 17/17, Python 64/64, and
fmt/diff clean. M3.0 changes documentation/continuity only, so rerunning the
entire build matrix would add no product-code evidence.

## Graph Evidence

The graph was checked before source inspection. Status was 19,534 nodes and
91,017 edges, and freshness was independently proven by current results for
`miller_layout`, `compute_agent_attachment_action_area`,
`compute_agent_worktree_action_area`, both typed action areas, both render
functions, and `App::handle_mouse`; `ready` alone was not accepted.

## Final Decision and Future Triggers

- M3.1: no trait/interface is defined.
- M3.2: no consumer is migrated and there is no mixed ownership state.
- M3.3: keep/revert resolves to **keep the current concrete typed seams**; no
  production diff exists to revert.
- S5 remains independently deferred until a second page/component duplicates
  render, hit geometry, lifecycle, focus/close ownership, and event routing.
- S6 remains deferred until a real additional resizable region requires
  persisted identity and migration.
- S7 remains deferred until a real nested popup must retain parent ownership.
- N2.2 remains deferred until independent cursor-history demand and finite
  eviction/restore semantics exist.
- A third frame action may justify only a private pure geometry/render helper;
  it does not automatically justify a registry or general panel/page contract.

This closes M3 without speculative production work and preserves an explicit,
testable activation path for future product demand.
