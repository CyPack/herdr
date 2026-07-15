# M2.0 Evidence — Worktree Management Actions

Date: 2026-07-15

Decision: **narrow GO** for one non-destructive focused-agent worktree launcher
that enters the existing `OpenExistingWorktree` flow. Four new independent
List/Create/Remove/Switch implementations are **NO-GO**.

## Requested Product Delta

`.local/prd/native-fm/99-MISSION-VISION.md` asks for create/open/remove/switch
worktree buttons or a panel reachable from the FM/agent flow. The durable value
is visible access without leaving the focused agent, not a second worktree
backend or a second set of dialogs.

## Existing Surface Inventory

| Requested action | Existing authority and surfaces | M2.0 decision |
|---|---|---|
| List | Neutral `worktree.list` API and `herdr worktree list`; Spaces/mobile already group open worktree workspaces; `OpenExistingWorktree` already renders a bounded searchable list with paths and status | New list backend/panel NO-GO |
| Open | Projects/Spaces Git-workspace context menu, configurable `open_worktree` binding, existing open dialog, neutral API/CLI | Reimplementation NO-GO; visible focused-agent launcher GO |
| Switch | `worktree.open` canonicalizes the checkout and focuses the existing workspace when already open; Spaces/mobile already switch directly | Separate switch operation/button NO-GO |
| Create | Existing New Worktree dialog, Git-workspace context menu, default `prefix+shift+g`, neutral deferred API/CLI | New create button/authority NO-GO |
| Remove | Existing linked-worktree-only context item, explicit confirmation/force stage, optional binding, neutral deferred API/CLI | Agent/FM-frame destructive shortcut NO-GO |

The selected delta is one complete `[w]` action on the focused agent's bottom
frame. It requests the existing open-worktree dialog for the active source
workspace. The dialog remains the list/search/open/switch owner. M2 adds no new
worktree state, filesystem/Git mutation, server command, panel, or protocol.

## Ownership and Identity

### Client-local

- pure `[w]` geometry and semantic rendering;
- one derived hit target containing the source workspace ID plus exact focused
  `PaneId`/`TerminalId` snapshot;
- input routing that emits only the existing
  `request_open_existing_worktree = Some(workspace_idx)` intent.

### Existing server/runtime owners

- `worktree.list` resolves a workspace ID or cwd to a canonical repo source and
  returns current checkout/open-workspace facts;
- `worktree.open` accepts exactly one path or branch, rejects bare/prunable
  entries, canonicalizes checkout paths, focuses an already-open workspace, or
  creates exactly one workspace and emits existing events;
- deferred create/remove operations own canonical checkout-key exclusion,
  filesystem/Git execution, completion, and recovery;
- linked-worktree membership and public workspace IDs remain the only removal
  authority. A frame coordinate or path label never authorizes deletion.

Herdr has no persistent tab ID. The launcher snapshot therefore uses workspace
ID, `PaneId`, and `TerminalId`; current tab position is only a live projection
through the stable pane.

## Existing Failure Coverage Reused

- list reports open workspace IDs and preserves prunable entries;
- open rejects invalid source/path combinations, bare/prunable worktrees, and
  reuses already-open checkouts found through canonical subpaths;
- create rejects relative paths, path collisions, duplicate in-flight work,
  and create/remove concurrency on the same canonical checkout key;
- remove rejects non-managed/non-linked workspaces, requires explicit force for
  dirty Windows checkouts, rejects duplicate/concurrent work, preserves event
  context, and recovers leftover checkout state;
- UI tests prove Create/Open/Remove submit through the same API paths and that
  linked children cannot become create/open sources.

## M2.1 Frozen RED/GREEN Test Points

Production code is forbidden until each matching missing behavior is observed
RED.

1. `focused_agent_worktree_action_is_capability_gated_and_disjoint`
   - RED: no `[w]` action/hit area exists.
   - GREEN: only a focused agent in an eligible non-linked Git workspace gets
     one complete action; non-agent, linked child, missing Git source,
     borderless, narrow, mobile, and modal states expose none; `[w]` and M1
     `[+]` never overlap or cover terminal inner cells.
2. `focused_agent_worktree_action_render_is_ascii_and_no_color_safe`
   - RED: no rendered launcher exists.
   - GREEN: the semantic action stays inside its exact area, preserves corners,
     scrollbar/split borders, adjacent action text, and no-color readability.
3. `worktree_action_click_revalidates_exact_workspace_pane_and_terminal`
   - RED: no typed frame dispatch exists.
   - GREEN: unmodified left click on the current snapshot emits one existing
     open-worktree request; workspace reorder, focus/terminal replacement,
     linked-child transition, stale geometry, modifier, and outside clicks emit
     nothing and never reach the terminal.
4. `worktree_action_routes_to_existing_open_dialog_without_new_authority`
   - RED: no route connects the frame action to the current dialog.
   - GREEN: scheduled handling opens the existing `OpenExistingWorktree` state,
     entries and search behavior; no new worker, watcher, Git command, create,
     remove, force, pane, or protocol request is introduced.
5. `worktree_action_close_error_and_empty_list_preserve_agent_resources`
   - RED: no lifecycle exists.
   - GREEN: cancel, list error, empty list, target exit, and close/reopen preserve
     every existing pane/terminal/process and clear only derived launcher state.
6. `worktree_action_complete_gate`
   - Exact M2.1 and existing worktree UI/API/CLI tests, all attachment/pane/input
     regressions, full nextest, Linux and Windows clippy, Bun/Python maintenance,
     fmt/diff/unwrap/debug-marker checks, graph freshness, and fork-only FF
     publication all pass with only the named B0 host probe skipped.

## Explicit NO-GO Boundaries

- no new worktree list or management panel;
- no Create/Remove/Switch backend or duplicate dialog;
- no removal from agent/FM coordinates and no implicit force;
- no branch deletion;
- no path-string identity or private TUI socket authority;
- no new default keybinding until a separate conflict/accessibility decision;
- no generic frame-action registry or popup stack; M3 remains evidence-gated.
