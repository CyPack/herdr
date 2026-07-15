# N2.0 Path-Stable Miller Navigation Decision

Date: 2026-07-15
Scope: specification/reference evidence only; no production code changed
Decision: narrow implementation GO for path-stable parent return; original
unbounded/dynamic Miller-column state machine NO-GO

## Question

Herdr already renders a responsive one/two/three-column Miller projection and
prepares parent/current/preview data outside render. N2.0 therefore asks which
observable navigation behavior is still missing, rather than whether Herdr can
adopt another file manager's internal architecture.

## Pinned Reference Evidence

| Reference | Pinned evidence | Observed contract |
|---|---|---|
| Ranger | [official README](https://github.com/ranger/ranger), three-column overview and `h/j/k/l` navigation | The visible model is a fixed parent/current/preview arrangement. It does not establish an arbitrarily growing visible column chain. |
| Yazi | `sxyazi/yazi` at `4dab4803479f5e1343c41daebd7e237b90e3d009`: `yazi-core/src/tab/tab.rs`, `yazi-actor/src/mgr/{enter,leave,cd,hover}.rs`, `yazi-core/src/tab/folder.rs`; [official navigation docs](https://yazi-rs.github.io/docs/quick-start/) | `Tab` owns one current folder, one optional parent, separate preview, and bounded active transitions. On `leave`, the former parent folder becomes current and `hover` repositions the new parent on the current path. Folder/path history may restore older cursor state, but it is not a growing visible column chain. |
| Joshuto | `kamiyaa/joshuto` at `d2581fb06581d12c0274af26f828b5f3f2a6c726`: `src/history.rs`, `src/tab/mod.rs`, `src/commands/{change_directory,parent_cursor_move}.rs`, `src/types/state/file_manager_state.rs` | Rendering and watching consume parent/current/child lists. `generate_entries_to_root` positions every ancestor list on its descendant, so returning to a parent focuses the directory just exited. Joshuto additionally supports parent-column sibling navigation and retained per-directory history; those are independent features, not required Miller projection behavior. |

The two independent implementation references agree on the relevant product
behavior: leaving a directory should focus that departed child in the new
current directory. They do not justify an unbounded visible column chain.

## Current Herdr Delta

Herdr's `FmState` already supplies:

- cached parent/current/preview state with filesystem work outside render;
- responsive one/two/three-column projection through `miller_layout`;
- path-stable `reload()` when the selected entry still exists;
- generation invalidation for preview work;
- deterministic cursor clamping after reorder/delete;
- selection clearing across directory boundaries and a clean close/reopen.

The missing behavior is isolated in `FmState::leave()`: it changes `cwd` to the
parent, forces `cursor = 0`, and reloads. If the departed directory is not the
first naturally sorted sibling, focus jumps to an unrelated entry. Parent
context already proves the correct path identity, but current-list focus drops
it.

## Accepted N2.1 Contract

`FmState::leave()` captures the departed `cwd` path before moving to its
parent. After the existing parent reload, it focuses the exact departed path if
that path remains a visible current-list entry. This is a path-identity rule,
never an index rule.

| Transition | Required result |
|---|---|
| Leave from a visible child directory | New `cwd` is the parent; current cursor selects the exact departed child; explicit multi-selection is cleared; preview describes that child. |
| Leave at filesystem root | Complete FM state is unchanged; no refresh/generation churn and no panic. |
| Departed path disappeared before the parent read | Navigation still reaches the readable parent; cursor uses the existing deterministic top/clamp fallback; no stale or synthetic entry is retained. |
| Departed path is hidden while `show_hidden == false` | Do not reveal unrelated or synthetic hidden entries in the current list; use the same fallback. |
| Parent contents reorder after leave | Existing `reload()` preserves the selected child by exact path. |
| Focused child is deleted after leave | Existing `reload()` retains/clamps the previous row, refreshes preview, and contains no stale selection identity. |
| Viewport is narrower than the focused row position | `compute_view`/`sync_viewport` scrolls the existing viewport to contain the cursor; render remains read-only. |
| Enter selected directory | Preserve current v1 behavior: enter only a directory, clear explicit selection, open it at cursor/viewport zero, and refresh context. |
| Cursor on a file or empty directory | Enter remains a no-op. |
| Hidden toggle | Existing path-preserving reload remains authoritative when the path is visible and deterministic fallback applies otherwise. |
| Close/reopen | No navigation history is persisted; a fresh `FmState` starts from its existing constructor contract. |

## Explicit Non-Goals

- no arbitrary visible column chain, auto-split tree, or auto-prune state
  machine;
- no retained back/forward or per-directory cursor history;
- no Joshuto-style parent-column sibling keybindings;
- no new input mapping, render geometry, dependency, worker, timer, watcher,
  process, server state, socket message, protocol field, or persisted snapshot;
- no filesystem access during render.

Per-directory cursor history may be reconsidered only as a separately named
feature with a finite eviction budget and independent user demand. Parent-column
sibling navigation requires its own interaction design because Yazi/Ranger's
default navigation does not establish it as a universal contract.

## Resource and Failure Budgets

| Budget | Bound |
|---|---|
| Additional retained navigation state | Zero fields and zero history entries. The departed `PathBuf` is local to one synchronous `leave()` call. |
| Additional filesystem work | No additional directory traversal or read beyond the existing `reload()`/context refresh path. |
| Focus lookup | One linear exact-path search over the already bounded current snapshot: `O(entries.len())`; no retry loop. |
| Async/runtime ownership | None. No task, thread, timer, channel, watcher, or process is created. |
| Cleanup | Local path drops at return; existing preview generation invalidation and worker lifecycle remain authoritative. |
| Cross-platform behavior | `PathBuf` equality against entries returned by the same parent read; no Unix-only identity or canonicalization assumption. |
| Failure recovery | Missing/hidden/raced path falls back deterministically and never blocks reaching the parent. Read failures retain existing typed directory status behavior. |

## Exact RED-Capable Test Plan

The RED commit precedes production code and targets `src/fm/mod.rs` only.

| Test | Setup and action | Expected result | Why current code is RED / protected risk |
|---|---|---|---|
| `leave_focuses_departed_child_by_path` | Create naturally ordered siblings where the opened child is not index zero, construct state inside it, then `leave()` | `cwd` is parent and `selected().path` equals the departed child | Current `leave()` forces cursor zero and selects the wrong sibling. Protects path versus index identity. |
| `leave_focuses_child_preview_and_clears_selection` | Give the departed child a nested entry; create explicit selection inside it; record preview generation; then leave | Selection set/anchor are empty, selected path is departed child, preview is that directory, generation advances | Current focus points elsewhere. Protects coupled selection/preview authority without render I/O. |
| `leave_focus_survives_parent_reorder_and_deletion` | Leave onto a nonzero child, insert a sorting sibling and reload, then delete the focused child and reload | Reorder preserves exact path; deletion removes stale identity and uses bounded row/clamp fallback | First focus assertion is RED today; the rest characterizes existing watcher/reload safety. |
| `leave_focus_scrolls_into_bounded_viewport` | Create enough earlier siblings to place departed child below a two-row viewport; leave then `sync_viewport(2)` | Cursor selects child and viewport contains it within legal bounds | Current cursor zero masks the required scroll. Protects responsive projection without changing layout. |
| `leave_missing_or_hidden_child_uses_top_fallback` | Construct from a disappeared child and separately from a hidden child with hidden entries disabled; leave | Parent opens, no synthetic child appears, cursor is deterministic and valid | Expected to pass as characterization; protects race and hidden-policy failure paths. |
| `leave_at_root_preserves_complete_state` | Snapshot root cwd, entries, cursor, viewport, selection, parent, preview, and generation; leave | Every field remains identical | Extends the existing cwd-only assertion so a no-op cannot trigger hidden lifecycle work. |

Required implementation sequence:

1. commit the failing/characterization tests;
2. run the exact tests and record the expected path-focus failures;
3. implement the smallest local path-focus change;
4. rerun exact tests, all FM tests, then the complete direct `just check`
   equivalent and isolated runtime checks only if the risk surface requires
   them;
5. refresh the code graph and verify `FmState::leave` source freshness before
   publication.

## Final Decision

- Original N2 dynamic/unbounded Miller state machine: **implementation NO-GO**.
  Current Herdr and all inspected references use a bounded visible projection;
  replacing the green model would add architecture without user value.
- N2.1 path-stable parent return: **implementation GO**. It is an observable,
  independently corroborated navigation defect with a small reversible change,
  exact failing tests, zero new retained state, and no server/render expansion.
