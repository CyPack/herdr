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

## Host desktop sources

The rail mirrors the sidebar the user already curates in their desktop file
manager. Design record and the reasoning behind the two omitted desktop rows:
[`docs/superpowers/specs/2026-07-31-herdr-files-host-bookmarks-and-virtual-locations-analysis.md`](../docs/superpowers/specs/2026-07-31-herdr-files-host-bookmarks-and-virtual-locations-analysis.md).

| ID | Behavior | Breaks if lost | Verified by |
|---|---|---|---|
| TP-FDB-PARSE-01 | The host bookmark list is read in file order, an explicit label outranks the directory name, URI escapes are decoded, and non-`file://` schemes are dropped rather than kept as unopenable rows. | The user's arrangement is re-sorted, a renamed entry loses its name, an escaped path resolves to the wrong directory, or a remote share becomes a row that can never open. | `desktop_bookmarks_preserve_order_labels_and_decoded_paths` |
| TP-FDB-PARSE-02 | The bookmark file is an external, hand-editable input and is capped before it reaches the model. | A runaway or hostile file turns startup model preparation into unbounded work. | `desktop_bookmarks_are_bounded_by_entry_ceiling` |
| TP-FDB-MODEL-01 | Bookmarks reach the rail as their own section in host order, and a bookmark whose target is gone stays visible and inaccessible instead of disappearing. | A broken bookmark vanishes silently, so the user never learns that a directory they rely on has moved. | `fdb_bookmarks_section_preserves_host_order_labels_and_broken_targets` |
| TP-FDB-MODEL-02 | The built-in block is fixed — Home, the XDG user directories, Network, Trash — and holds those directories whether or not the host also bookmarks them. Trash points at the directory the desktop file manager shows. | Bookmarking Downloads demotes it out of the built-in block and scatters it into the middle of the curated list, so the rail's stable top region stops being stable. | `fdb_well_known_directories_stay_in_the_built_in_block_when_also_bookmarked` |
| TP-FDB-XDG-01 | The well-known user directories are read from the list the host records (`user-dirs.dirs`), which is localized per path element, and an absolute override is honoured as written. | The built-in block is derived from English names the host may not use, so a Turkish, German or French desktop shows Home and Trash and nothing else — exactly the arrangement the user came here to see. | `user_directories_follow_the_localized_names_the_host_recorded` |
| TP-FDB-XDG-02 | That list is external, hand-editable input: comments, keys this surface does not publish and malformed lines are skipped without taking the readable entries with them, a partial list is completed from the unlocalized defaults, and a file past the read ceiling is refused before parsing. | One bad line costs the rail every directory around it, a host that records only some directories loses the rest, or a runaway file turns startup into unbounded work. | `user_directories_survive_comments_unknown_keys_and_malformed_lines`, `a_partial_localized_list_is_completed_rather_than_truncated`, `user_directories_fall_back_to_the_unlocalized_names`, `a_runaway_user_directory_list_is_refused_before_it_is_parsed` |
| TP-FDB-MODEL-03 | The built-in block labels each directory with the host's own name for it and carries identity on the kind rather than the label, so a directory the host points back at home draws no second Home row. | The rail claims names the host does not use, or renders Home twice when the desktop directory is turned off. | `fdb_built_in_block_follows_the_host_own_directory_names` |
| TP-FDB-VOL-01 | The rail offers the volumes the host has mounted, in mount-table order, restricted to local filesystems under the roots a desktop treats as places, with octal-escaped mount paths decoded and the table bounded. | Machinery (`/proc`, `tmpfs`), plumbing (`/boot`) and the already-present root turn the rail into an inventory — or worse, a network/FUSE mount slips in and a dead one hangs herdr at startup. | `mounted_volumes_offer_host_places_and_refuse_machinery`, `a_container_sized_mount_table_is_bounded` |
| TP-FDB-VOL-02 | A host with no desktop at all — no bookmark list, none of the well-known directories on disk — still reaches home, its mounted volumes and the filesystem root. | The rail is empty wherever there is no graphical file manager, so herdr stops carrying the experience it exists to carry on servers, containers and SSH sessions. | `fdb_a_host_without_a_desktop_still_gets_a_rail_worth_drawing` |
| TP-FDB-RENDER-01 | A painted rule divides sections, reusing the single content line the model already counts for a section gap. | Either the groups run together with no visible boundary, or drawing the boundary consumes an extra line and row hit targets stop matching the painted rows. | `fdb_section_boundary_paints_a_rule_without_shifting_row_identity` |

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
- `TP-FDB-MODEL-02` depends entirely on section dedup keeping path authority
  with the *first* section that claims it. Making the bookmark section win a
  duplicate — or emitting the built-in directories only when the host has no
  bookmark list — moves Downloads and Documents out of the fixed top block and
  into wherever the user happened to bookmark them. This was tried and rejected:
  the rail's top region has to stay in the same place between machines.
- Network discovery in `src/platform/linux.rs` deliberately does not `stat` the
  GVfs mount root. If a resolution ever reduces it to an `is_dir()` check, a
  dead `gvfsd-fuse` will hang herdr at startup.
- The built-in block's names come from `crate::platform::user_directories`, not
  from English constants. If a resolution ever reintroduces `home.join("Downloads")`
  it will keep passing on an English desktop and silently empty the block on
  every other one — which is how this gap survived unnoticed until it was measured.
- The volume allow list in `src/platform/linux.rs` is deliberately an allow
  list. Inverting it into a deny list of remote filesystems looks equivalent and
  is not: every filesystem type added upstream then arrives enabled, and the
  first one that is network-backed reintroduces the dead-mount startup hang that
  `network_mounts_root` refuses to `stat` for.
