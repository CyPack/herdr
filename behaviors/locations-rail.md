# Locations Rail and Follow — registered behaviors

Fork feature. The Rail is the locations panel beside the Trail; Follow is the
asynchronous load that happens as the Rail cursor moves. Upstream has neither.

The family is dominated by one hard problem: **the Rail moves faster than the
filesystem answers.** Most entries below are the rules that keep a late, slow
or dead result from overwriting what the user is looking at now. When resolving
a merge conflict here, the question to ask is always *"can an older result win
this race?"* — if yes, the resolution is wrong.

Format and rules: [`README.md`](README.md).

## Entering a location

| ID | Behavior | Breaks if lost | Verified by |
|---|---|---|---|
| TP-FLF-EMPTY-01 | An empty readable destination is a successful exact root with no fabricated Trail cursor or synthetic row. | An empty directory grows a phantom row that can be selected and acted on. | `flf_empty_root_succeeds_without_synthetic_cursor` |
| TP-FLF-EMPTY-02 | Explicit entry into an empty readable root transfers Trail ownership but never invents a row-zero cursor. | The same phantom row appears on the explicit-entry path. | `flf_empty_entered_destination_keeps_none_cursor` |
| TP-FLF-ENTER-01 | Explicit entry is a typed deferred request: it cannot move Trail focus before the exact root has been accepted. | Focus jumps to a destination that then fails to load, stranding the user in a column that does not exist. | `flf_rail_right_and_enter_queue_enter_without_immediate_focus` |
| TP-FLF-ENTER-02 | An explicit Rail entry transfers focus only after the exact root is accepted, and that same transition highlights row zero. | Either focus moves too early, or the user lands in a loaded directory with nothing selected. | `flf_entered_root_highlights_first_actionable_entry` |
| TP-FLF-ENTER-03 | Crossing one prepared hierarchy edge with Right owns the child column and immediately highlights its first real row. | Right enters a column with no cursor, so the next keystroke has no anchor. | `flf_entered_child_highlights_first_actionable_entry` |
| TP-FLF-ENTER-04 | Because explicit entry owns row zero immediately, the next Down advances exactly once, to row one. | Down after entry skips a row or does nothing, because entry left the cursor unset. | `flf_next_down_after_entry_selects_second_entry` |

## Cursor, focus and paint

| ID | Behavior | Breaks if lost | Verified by |
|---|---|---|---|
| TP-FLF-COMPACT-01 | At narrow widths the drawer is the compact Rail owner and paints the same strong cursor semantics as the persistent wide Rail; root Left opens it, and Esc restores the Trail while retiring a deferred entry request that can no longer own focus. | The compact layout behaves like a different application, or an in-flight request lands after Esc. | `flf_compact_drawer_focus_matches_wide_rail`, `flf_compact_root_left_opens_drawer_and_escape_invalidates_entry`, `flf_mouse_location_click_synchronizes_cursor_and_typed_intent` |
| TP-FLF-FOCUS-01 | Exact location authority seeds the exact cursor, and a direct descendant never invents an ancestor match. Left is pure owner focus, crossing one resident edge per event; the first event past the root transfers to the Rail, where a further Left is render-neutral. | Focus lands on a guessed ancestor, or repeated Left walks past the root and repaints pointlessly. | `flf_cursor_normalizes_exact_location_without_inferred_direct_ancestor`, `flf_rail_owner_swallows_shift_ctrl_and_hidden_trail_actions`, `flf_root_left_focuses_rail_with_exact_or_direct_fallback`, `left_arrow_moves_one_column_per_event_and_transfers_to_rail_at_root` |
| TP-FLF-MOUSE-01 | Mouse directory selection keeps the exact parent row focused; its prepared child may stay resident, but unlike explicit keyboard entry it cannot synthesize child focus. | A single click behaves like an Enter, moving focus the user did not ask to move. | `flf_mouse_directory_click_preserves_parent_focus_contract` |
| TP-FLF-NO-HIGHLIGHT-01 | Trail identity stays resident while the Rail owns focus, but only the Rail cursor may carry the painted focus style. | Two cursors appear focused at once, so the user cannot tell which surface a keystroke will hit. | `flf_render_rail_focus_suppresses_trail_cursor_style` |
| TP-FLF-PREVIEW-01 | Rail vertical ownership advances one accessible row and cannot mutate the resident Miller Trail; mouse uses the same exact Rail cursor but carries surface-specific Follow rather than Enter intent. | Browsing the Rail rewrites the Trail behind it, losing the user's loaded branch. | `flf_mouse_location_click_synchronizes_cursor_and_typed_intent`, `flf_rail_up_down_move_one_and_never_mutate_trail` |
| TP-FLF-RENDER-01 | Focus changes alter paint only: never state, row hit targets, column projection, or repeated output bytes. Routine `compute_view` reconciliation likewise cannot snap a still-valid keyboard cursor back to the accepted origin. | Moving focus mutates hit geometry, so clicks resolve to different rows than the ones drawn, or the cursor silently jumps home on an unrelated repaint. | `flf_render_is_state_pure_and_geometry_identical`, `flf_reconcile_preserves_valid_cursor_distinct_from_origin` |
| TP-FLF-STEP-01 | Rail cursor motion advances exactly one accessible item per event and clamps at both boundaries without manufacturing a visible mutation. | Held keys skip items, or hitting the end repaints as if something changed. | `flf_clamped_and_deferred_keys_decline_immediate_render`, `flf_cursor_scroll_reveals_exact_model_line`, `flf_cursor_steps_accessible_items_one_at_a_time_and_clamps`, `flf_model_line_identity_matches_render_section_law`, `flf_rail_up_down_move_one_and_never_mutate_trail`, `flf_reconcile_preserves_valid_cursor_distinct_from_origin` |
| TP-FLF-VIS-01 | The keyboard target is the strongest Rail identity and the accepted origin stays visible without impersonating current focus; ANSI and no-color users can still tell cursor from origin through modifiers when every relevant colour agrees, and the compact drawer paints the same semantics as the wide Rail. | On restricted-colour terminals the cursor and the origin become indistinguishable, or cursor semantics change with terminal width. | `flf_render_no_color_distinguishes_cursor_from_origin_by_modifiers`, `flf_render_rail_cursor_wins_and_origin_remains_subdued`, `flf_compact_drawer_focus_matches_wide_rail` |

## Asynchronous Follow

| ID | Behavior | Breaks if lost | Verified by |
|---|---|---|---|
| TP-FLF-BOUND-01 | While one request executes, a hundred cursor targets collapse to the final pending target instead of growing a FIFO queue. | Fast cursor movement queues work that must all drain before the user sees the destination they actually stopped on. | `flf_blocked_hundred_move_burst_processes_first_and_final_only` |
| TP-FLF-BOUND-02 | Cursor, focus and the caller's scheduled progress stay available while the only filesystem processor is deterministically held. | A slow directory freezes the whole surface instead of just its pending result. | `flf_blocked_root_keeps_cursor_input_and_render_loop_responsive` |
| TP-FLF-BOUNDED-01 | Explicit host calibration for the bounded lane, ignored in routine suites because it creates 110k synthetic inodes and records timing observations rather than portable CI budgets. | Timing budgets get asserted in CI where they are not portable, producing noise instead of signal. | `flf_scale_locations_follow_navigation` |
| TP-FLF-BLOCKED-01 | The same calibration pairing for the blocked-lane observation. | As above. | `flf_scale_locations_follow_navigation` |
| TP-FLF-FAST-01 | Both surface intents use the resident snapshot fast path and perform zero worker processor calls. | Every Rail keystroke schedules filesystem work, so browsing turns into I/O. | `flf_resident_follow_and_enter_perform_zero_worker_reads` |
| TP-FLF-IO-01 | Follow is asynchronous, preserves the resident Trail while blocked, and keeps the Rail as focus owner after an exact success. | A slow filesystem blanks the Trail, or focus escapes the Rail mid-browse. | `flf_follow_request_keeps_rail_focus_until_exact_success` |
| TP-FLF-IO-02 | Right or Enter over the exact pending Follow upgrades that intent in place, synchronously, before a scheduled drain can observe the weaker intent; no second worker generation and no second request cross the App boundary. | Two requests race for one destination and the weaker Follow result wins over the explicit Enter. | `flf_enter_promotes_exact_pending_without_duplicate_submission`, `flf_rail_enter_promotes_pending_follow_before_scheduled_drain` |
| TP-FLF-PERF-01 | First-entry initialization is entirely snapshot-backed, and crossing a prepared child edge cannot enumerate the filesystem. | Opening or stepping into a prepared directory stalls on a fresh enumeration. | `flf_first_entry_initialization_performs_zero_filesystem_reads` |

## Losing the race — staleness and failure

| ID | Behavior | Breaks if lost | Verified by |
|---|---|---|---|
| TP-FLF-DISCONNECT-01 | A dead lifecycle discards even a ready result, replaces the lane once, never replays, and still permits a later explicit try. | A dead worker either replays stale results or permanently disables navigation. | `flf_worker_disconnect_reports_failure_restarts_once_and_next_request_succeeds` |
| TP-FLF-FAIL-01 | Stable failure classes preserve the last accepted Trail and never install a failed root as the accepted location; a marker requires current cursor, Files generation and model revision authority, because matching only a path is stale. | A failed destination becomes the accepted location, so the surface shows a directory that never loaded. | `flf_missing_changed_type_permission_preserve_last_accepted_trail`, `flf_render_pending_failure_apply_only_to_current_cursor` |
| TP-FLF-FAIL-02 | A root processor panic becomes a typed failure, and the same bounded lane accepts and completes the next explicit request. | One bad directory kills the lane and every later navigation silently does nothing. | `flf_root_panic_reports_failure_and_lane_remains_reusable` |
| TP-FLF-HISTORY-01 | Keyboard entry is a fresh focus transaction: a previously selected deeper destination cannot survive behind it. | An older deeper selection re-emerges after an unrelated entry. | `flf_keyboard_activation_discards_hidden_destination_history` |
| TP-FLF-LATEST-01 | Only the latest worker ticket may update the accepted origin, Trail root, cursor and focus projection. | An older, slower result overwrites the newer destination the user is already looking at. | `flf_latest_root_only_updates_cursor_origin_trail_and_focus` |
| TP-FLF-STALE-01 | Replacing the model retires request and error authority tied to an obsolete cursor before selecting the new first accessible row, and a failure marker carries cursor, generation and revision authority rather than path alone. | An error from the previous model is attributed to the new one, or a stale error is shown against the current destination. | `flf_cursor_reconcile_retires_obsolete_pending_and_failure`, `flf_render_pending_failure_apply_only_to_current_cursor` |
| TP-FLF-STALE-02 | If input moves the cursor before a scheduled request is consumed, an already-ready old result cannot win that scheduling race. | The destination flips back to the previous target purely on timing. | `flf_result_before_request_rejects_old_root_after_cursor_move` |
| TP-FLF-STALE-03 | Path equality is not generation equality: an A to B back to A sequence finishes with the newest Follow intent, never the original Enter. | Returning to a path resurrects a superseded intent, because only the path was compared. | `flf_same_path_a_b_a_cannot_revive_old_enter_intent` |
| TP-FLF-STALE-04 | Pending identity alone is insufficient: losing Rail focus retires a late completion exactly like a close, model or generation change. | A completion lands after the user left the Rail and yanks focus back. | `flf_close_model_focus_and_generation_invalidate_completion` |

---

## Notes for the next sync

- `TP-FLF-STALE-03` is the subtlest entry in the fork: comparing paths instead
  of generations looks correct and passes casual review. If a resolution ever
  reduces an identity check to a path comparison, it violates this contract.
- `TP-FLF-BOUNDED-01` and `TP-FLF-BLOCKED-01` are deliberately excluded from
  routine runs. Do not "fix" them into CI; they are host calibration.
