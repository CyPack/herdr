# Shared surfaces — registered behaviors

Fork behaviors whose code lives in files **upstream also owns**. That is the whole
reason this file is separate: a three-way merge can only take upstream's version of a
region silently when upstream has a version of that file. Fork-only files cannot be
reverted without a conflict; these can.

**During a sync, read this file first.** When a conflict lands in `src/app/state.rs`,
`src/ui.rs`, `src/app/input/sidebar.rs`, `src/client/mod.rs` or `src/ui/panes.rs`, the
"Breaks if lost" column is what decides whether a resolution is acceptable.

Format and rules: [`README.md`](README.md).

## Activation and keybinding

How the fork's surfaces are reached at all. Upstream owns the keybind registry, the navigate-mode dispatcher and the help screen, so these are the first rows a sync can quietly unbind.

| ID | Behavior | Breaks if lost | Verified by |
|---|---|---|---|
| TP-ACT-1 | `toggle_file_manager` opens the file manager and closes it again, symmetrically. | The toggle becomes one-way: the surface opens but the same action no longer dismisses it. | `toggle_file_manager_opens_and_closes` |
| TP-ACT-2 | Opening the file manager starts from the active workspace's directory, not a process-global cwd. | The file manager opens somewhere the user was not working, in the wrong workspace. | `open_file_manager_uses_active_workspace_cwd` |
| TP-ACT-3 | The bound key (default prefix+f) drives the toggle end-to-end through the navigate-key handler: enum, mapping and dispatch wiring together. | The action exists but no keystroke reaches it, because one link in the enum/mapping/dispatch chain was dropped. | `prefix_f_key_toggles_file_manager` |
| TP-ACT-4 | Dispatching the `ToggleFileManager` action toggles the surface *and* leaves navigate mode. | The user lands in the file manager with navigate mode still armed, so the next keystroke is interpreted by the wrong owner. | `toggle_file_manager_action_opens_and_leaves_navigate_mode` |
| TP-ACT-5 | The file manager action is discoverable in the keybind help. | The feature becomes undiscoverable: it works but nothing tells the user it exists. | `help_lists_the_file_manager_action` |
| TP-ACT-DEFAULT | The shipped default keybinds bind prefix+f to the file manager. | A fresh install has no way to open the fork's main surface without hand-editing config. | `default_binds_prefix_f_to_file_manager` |
| TP-M1.1-KEYBIND | The agent-attachment picker owns exactly one configurable prefix action and registers it through the existing user-first conflict registry rather than hard-coding input. | A user's own binding is silently overridden, or the fork's binding collides with an upstream one and one of them stops working. | `default_binds_prefix_a_to_agent_attachment_picker_without_conflict` |

## Repaint gate

The router decides which input events are worth a redraw. This lives in upstream's mouse routing, and the failure mode is not a wrong pixel but a terminal that repaints on every mouse move.

| ID | Behavior | Breaks if lost | Verified by |
|---|---|---|---|
| TP-REPAINT-2B | A hover move with no blocking overlay (plain terminal or the native file-manager surface) changes nothing herdr draws, so the router must not request a render for it. | Every pointer movement across the window forces a full repaint, wasting CPU and flickering on slow links. | `inert_mouse_move_declines_render` |
| TP-REPAINT-2C | Generic press, release and drag input always requests a repaint. | A click produces no visible response until some unrelated event happens to redraw. | `non_move_mouse_events_always_request_render` |
| TP-REPAINT-2D | While a hover-sensitive overlay owns the pointer, a move can change its highlight, so the router keeps requesting a render. | Menu and popup highlights stop following the mouse, because the move gate suppressed the repaint they needed. | `mouse_move_over_blocking_overlay_requests_render` |
| TP-REPAINT-2E | Non-mouse interaction (keys, paste) is low-frequency and always repaints, unchanged by the move gate. | A repaint optimisation aimed at mouse traffic accidentally swallows keyboard feedback. | `keyboard_and_paste_always_request_render` |
| TP-REPAINT-2F | Wheel input repaints on the same generic path as press/release/drag; the native-FM vertical wheel has its own exact typed override. | Scrolling produces no visible movement until another event triggers a draw. | `non_move_mouse_events_always_request_render` |

## compute_view composition

`compute_view` is upstream's, and the fork hangs a great deal off it. Every row here is a rule about geometry published for one frame and retired with it.

| ID | Behavior | Breaks if lost | Verified by |
|---|---|---|---|
| TP-A3.2-VIEWPORT | `compute_view` owns viewport normalization for both responsive layouts: shrinking or expanding the available height keeps the cursor visible and clamps stale offsets to the new maximum. | After a resize the cursor is scrolled off-screen, or a stale offset points past the end of the list. | `compute_view_normalizes_file_manager_viewport_after_resize`, `current_row_actions_follow_miller_geometry_at_all_breakpoints`, `enter_and_leave_normalize_viewport_for_new_directory`, `viewport_follows_cursor_and_clamps_at_both_edges`, `viewport_handles_zero_rows_reload_shrink_and_empty_list` |
| TP-C2.1-VIEWSTATE | Desktop `compute_view` snapshots the current name and action rects from one geometry source, then clears both when the file manager closes. | Stale terminal coordinates stay clickable after the surface is gone, so a click hits an invisible target. | `compute_view_snapshots_and_clears_file_manager_row_areas` |
| TP-N3.1-LIFECYCLE | `compute_view` rebuilds persistent action-bar content after navigation or reload, clears it on close, and restores the current empty selection plus the client-local clipboard summary on reopen. | The action bar shows the previous directory's state, or the clipboard summary vanishes on reopen. | `compute_view_refreshes_and_clears_file_manager_action_bar_content` |
| TP-C6.4-VISUAL | Expanded, collapsed desktop and responsive mobile layouts compose the same exact state without stale sidebar or row authority, and overlays paint above the composed surface without changing its prepared operation state. | A layout breakpoint shows a different truth than the state holds, or opening a menu mutates the pending operation. | `native_fm_composes_sidebar_breakpoints_and_status_across_full_frames`, `native_fm_context_and_delete_modal_compose_above_status_surface` |
| TP-FMN-NAV-08 | The legacy Miller projection runs first during `compute_view`, but its viewport may not drag an inactive preview child into focus before the Trail projection is published. | Focus lands on a preview column the user never activated, so the next keystroke acts on the wrong directory. | `compute_view_auto_follow_tracks_active_trail_owner` |

## Context menu, plugins and destructive confirmation

These converge on upstream's popup, menu-render and dialog owners. The recurring rule is that authority is typed and prepared in advance — never reconstructed from what the popup happens to be showing.

| ID | Behavior | Breaks if lost | Verified by |
|---|---|---|---|
| TP-C3.1-CONTEXT-MODEL | Cursor focus or an empty prepared selection does not invent a menu; the six core actions keep deterministic order; read-only state disables only cwd-writing actions; multiple selection permits only bulk-capable actions while preserving prepared path order; and unsupported, stale or in-flight authority disables every item, with in-flight taking priority. | A menu appears for nothing, actions reorder under the user's hand, or an action runs against a selection that is stale or already being operated on. | `file_context_kind_exposes_deterministic_labels`, `file_context_menu_requires_explicit_prepared_selection`, `invalid_or_in_flight_file_context_menu_fails_closed`, `multiple_file_context_menu_disables_single_target_actions`, `single_file_context_menu_has_stable_order_and_read_only_authority` |
| TP-C3.2-POPUP-LIFECYCLE | Disabled file actions stay visibly dim even when highlighted, enabled rows keep normal and selected contrast, and the model is read-only during rendering. | A disabled action looks enabled once highlighted, inviting a click that silently does nothing — or rendering mutates the menu it is drawing. | `disabled_and_stale_file_context_actions_fail_closed`, `disabled_file_context_items_have_distinct_highlight_safe_style`, `file_context_menu_keyboard_owns_focus_and_emits_exact_intent`, `file_context_menu_mouse_hover_click_outside_and_close_lifecycle` |
| TP-C3.3-PLUGIN-SURFACE | Plugin actions append after built-ins in stable qualified-id order, carry the exact prepared paths through the neutral public plugin API and back across JSON, size the shared popup by display cells rather than byte length, and fail closed on unknown action contexts. Lossy path conversion is forbidden: a Unix path JSON cannot represent exactly keeps built-ins but exposes no plugin action. | A plugin receives the wrong file, a non-UTF-8 path silently becomes a different path, unicode titles corrupt the popup frame, or link order changes the menu. | `file_context_menu_appends_plugins_and_serializes_exact_path_intent`, `file_context_menu_hides_plugins_for_non_utf8_paths`, `file_context_menu_plugin_action_is_typed_and_disable_race_fails_closed`, `file_manifest_actions_are_enabled_filtered_and_deterministic`, `manifest_rejects_unknown_file_action_context`, `plugin_context_merge_preserves_exact_file_paths`, `plugin_file_action_context_round_trips_exact_paths`, `plugin_file_context_menu_uses_display_width_for_unicode_title` |
| TP-C4.2-CONFIRM | The first modal makes reversible and irreversible choices explicit and reports the exact bounded selection size; irreversible authority has a visibly separate second gate that cannot be mistaken for the default trash action. | A permanent delete is one keystroke away from a trash delete, or the user cannot tell how many files they are about to destroy. | `context_delete_opens_the_same_confirmation_model`, `delete_confirmation_ignores_modified_destructive_shortcuts`, `delete_confirmation_mouse_buttons_are_bounded_and_fail_closed`, `delete_confirmation_rejects_empty_and_inflight_authority`, `file_delete_choose_action_renders_distinct_safe_choices`, `file_delete_permanent_stage_renders_irreversible_warning`, `header_delete_opens_exact_confirmation_without_mutation`, `permanent_delete_requires_second_confirmation`, `stale_or_reopened_confirmation_cannot_emit_delete_request`, `trash_confirmation_emits_request_while_cancel_is_side_effect_free` |
| TP-C6.3-AUTHORITY | A revalidated plugin intent reaches the existing App-owned command runtime exactly once; the scheduler consumes the typed intent and never infers an action from the popup title. | An action runs twice, or a renamed menu label changes which command executes. | `context_open_converges_on_existing_navigation_authority_once`, `file_manager_plugin_intent_uses_existing_command_runtime_once`, `row_delete_converges_on_shared_typed_confirmation_authority`, `unsupported_context_action_is_consumed_without_side_effects` |
| TP-C6.3-LIFECYCLE | Closing the surface invalidates every pending action authority immediately, before a same-cwd reopen could make stale paths look current again at the scheduled boundary. | An action prepared before close executes after reopen against paths that only look the same. | `close_file_manager_clears_pending_action_authority`, `unsupported_context_action_is_consumed_without_side_effects` |

## Locations in the shell sidebar

The Locations rail is fork work living inside upstream's sidebar. Ownership is the theme: Files is a center-stage launcher and must not take over global shell surfaces.

| ID | Behavior | Breaks if lost | Verified by |
|---|---|---|---|
| TP-C6.1-MODEL | Filesystem discovery happens before render: existing well-known directories keep Finder order, missing favorites are omitted, configured pins stay visible but marked inaccessible, duplicate path authority stays with the first section, optional PINNED disappears when empty, and adversarial configuration cannot create an unbounded model. | Render blocks on the filesystem, a hostile config grows the sidebar without limit, or a duplicated path makes a later row steal authority from an earlier one. | `file_locations_model_is_bounded_across_all_sections`, `file_locations_model_orders_sections_and_deduplicates_path_authority`, `file_locations_preparation_uses_live_home_and_pin_state` |
| TP-C6.1-LIFECYCLE | Location preparation reads live home and pin state at preparation time rather than trusting a cached snapshot. | The rail shows the pins or home directory from an earlier moment. | `file_locations_preparation_uses_live_home_and_pin_state`, `location_navigation_opens_exact_directory_and_rejects_stale_targets` |
| TP-C6.1-NAV | A content rail row carries exact path identity; mouse input prepares one typed request and performs no directory read itself. | Clicking a location reads the filesystem on the input path, or navigates by row index instead of by path. | `clicking_file_locations_rail_item_prepares_exact_typed_navigation_request`, `location_navigation_opens_exact_directory_and_rejects_stale_targets`, `stale_file_locations_rail_hit_area_is_inert_after_model_refresh` |
| TP-C6.1-GEOMETRY | Cached hit geometry cannot authorize a path after the prepared model changes underneath it. | A click resolves against a rail that has already been rebuilt, opening whatever now sits at those coordinates. | `stale_file_locations_rail_hit_area_is_inert_after_model_refresh` |
| TP-FCL-INPUT-01 | A fresh row click and vertical rail scroll are content-owned, and raw host bytes at the remote-client boundary prepare the exact typed request. | Rail interaction is claimed by the global sidebar, or a remote client's clicks do not reach the same request path a local one does. | `clicking_file_locations_rail_item_prepares_exact_typed_navigation_request`, `fcl_input_fresh_row_click_and_vertical_rail_scroll_are_content_owned`, `headless_raw_mouse_locations_navigation_loads_exact_trail` |
| TP-FCL-SHELL-01 | Files is a center-stage launcher: the global Spaces projection, including its workspace/agent tracking body and its wheel, stays owned by Spaces after activation — even when a legacy Files tab value is present. | Opening Files hijacks the global workspace tracker, so the user loses the sidebar view they were using. | `fcl_shell_files_activation_preserves_spaces_sidebar_projection`, `legacy_files_tab_value_keeps_visible_spaces_wheel_interaction`, `legacy_files_tab_value_renders_spaces_tracker_not_locations` |
| TP-FCL-SHELL-02 | Projects and Files are independent presentation owners; opening Files cannot silently switch the global body away from a user-selected Projects view. | Opening a file browser silently changes an unrelated part of the shell. | `fcl_shell_files_activation_preserves_projects_sidebar_owner` |
| TP-FMR-SIDEBAR-HL-01 | Raw host mouse bytes arriving at the headless server prepare the exact typed locations request. The source pins -01, -02 and -03 together with one live headless test, because the App-level test cannot exercise the real remote boundary. | Remote clients get different navigation behavior from local ones, and no App-level test can see it. | `headless_raw_mouse_locations_navigation_loads_exact_trail` |
| TP-FMR-SIDEBAR-HL-02 | The headless scheduled loop consumes the prepared request into the existing Files generation rather than creating a second authority. Pinned together with -01 and -03. | A remote-driven navigation lands in a generation nothing else is reading. | `headless_raw_mouse_locations_navigation_loads_exact_trail` |
| TP-FMR-SIDEBAR-HL-03 | The resulting Trail is the exact requested one, verified across the real client boundary rather than in-process. Pinned together with -01 and -02. | The remote path resolves to a near-miss directory and only a live client would reveal it. | `headless_raw_mouse_locations_navigation_loads_exact_trail` |

## Image preview in server mode

The file manager's image preview is driven by a bounded worker that only advances
when something drives it, and it decodes against a cell size only the client knows.
Both halves live in the server loop, and both were missing — so a preview that
worked perfectly in the local TUI showed nothing at all over a socket. These rows
exist because that failure was completely silent: no error, no warning, just a
panel that stayed empty.

| ID | Behavior | Breaks if lost | Verified by |
|---|---|---|---|
| TP-FMR-IMAGE-HL-01 | The headless scheduled loop drives the image preview worker, exactly as the monolithic loop does. | Server-mode image previews never leave `Pending`: the decode is never started, so the panel stays empty forever with nothing reported. | `headless_scheduler_syncs_the_image_preview_worker` |
| TP-FMR-IMAGE-HL-02 | The foreground client's resolved cell size is published to the image preview from the same decision that resolves host graphics, not from a second policy. | The preview decodes against a cell size of zero, derives no target and renders nothing — a second silent blocker that survives even after the scheduler call is restored. | `headless_publishes_foreground_cell_size_to_the_image_preview` |
| TP-FMR-IMAGE-HL-03 | A ready file-manager image reaches the client as kitty graphics inside the streamed frame. | The decode succeeds but the pixels never cross the socket, so every earlier link can be green while the user still sees nothing. | `server_frame_carries_fm_image_graphics_when_ready` |
| TP-FMR-IMAGE-HL-04 | A client whose cell size is unknown receives no graphics and no panic; the size is never guessed. | A guessed cell size is worse than none: kitty scales to exactly fill the given cell box, so the image is silently stretched instead of simply absent. | `unknown_cell_size_client_gets_no_graphics_and_no_panic` |
| TP-SRV-SCHED-PARITY-01 | The headless and monolithic schedulers make the same set of `sync_*`/`refresh_*` calls, apart from an explicit in-source difference list; closing a gap forces that list to be updated. | The two loops drift apart again and a feature works locally but not over a socket, with no test failing until a human notices. | `scheduler_parity_headless_vs_monolithic` |

## Pane bottom-border actions and the attachment picker

Fork affordances drawn into upstream's pane chrome. Geometry that upstream reflows is the risk; every row insists the action stays bounded, exact and no-color safe.

| ID | Behavior | Breaks if lost | Verified by |
|---|---|---|---|
| TP-M1.1-GEOMETRY | Only the exact focused agent may own one complete bottom-border action, and capability failures must not leave stale geometry behind. | An unfocused pane offers an action, or a dead affordance stays clickable after the capability went away. | `focused_agent_attachment_action_is_exact_agent_only_and_responsive` |
| TP-M1.1-RENDER | Render consumes computed geometry only; the ASCII action preserves pane corners and terminal inner cells even with no-color tokens. | The pane border breaks, or the action is invisible on a no-color terminal. | `agent_attachment_action_render_is_bounded_ascii_and_no_color_safe` |
| TP-M2.1-GEOMETRY | One eligible focused agent gets a complete worktree launcher beside — never overlapping — the existing attachment action. | The two actions overwrite each other and one becomes unclickable. | `focused_agent_worktree_action_is_capability_gated_and_disjoint` |
| TP-M2.1-RENDER | The worktree launcher is a bounded ASCII token that stays distinct from `[+]` and preserves pane corners in no-color mode. | Two different actions look identical, so the user cannot tell which one they are about to trigger. | `focused_agent_worktree_action_render_is_ascii_and_no_color_safe` |
| TP-M2.1-ROUTE | The frame action emits only the existing open intent and enters the established searchable dialog without creating new authority. | A second, divergent worktree dialog path appears and drifts from the canonical one. | `focused_agent_worktree_action_routes_to_existing_open_dialog_without_new_authority` |
| TP-M2.1-FAILURE | A source that disappears after frame computation fails closed in the existing dialog owner and preserves every agent resource. | A vanished worktree source tears down agent resources that had nothing to do with it. | `worktree_action_list_error_preserves_agent_resources_and_clears_only_request` |
| TP-M1.2-OPEN | Picker state binds the exact stable identities and starts from the same workspace cwd authority as the file manager. | The picker browses a different directory than the file manager, against a target it identified loosely. | `opening_attachment_picker_binds_exact_target_and_workspace_cwd` |
| TP-M1.2-TINY | Incomplete modal geometry declines before allocating picker or file-manager state and returns one stable visible-reason classification. | A tiny terminal allocates state for a modal it cannot draw, then fails somewhere less recoverable. | `attachment_picker_tiny_area_declines_with_visible_reason` |
| TP-M1.2-UNAVAILABLE | Capability loss fails closed with a stable visible reason instead of silently consuming the configured action. | The keybinding appears dead with no explanation. | `attachment_picker_unavailable_target_is_visible_and_non_mutating` |
| TP-M1.2-CANCEL | Cancelling the overlay owns no runtime resource and restores terminal mode without preparing a delivery request. | Escape leaks a resource, leaves the terminal in raw mode, or still sends the attachment. | `attachment_picker_escape_restores_valid_focus_without_delivery` |
| TP-M1.2-AUTHORITY | Exactly one current regular UTF-8 file is exposed as attachment authority; directories are navigation targets, never attachment authority. | A directory is handed to an agent as if it were a file. | `attachment_picker_accepts_one_regular_file_and_disables_other_targets` |

## Client lane scheduling

The client's event lanes. Upstream owns the loop; the fork's rule is that a burst of presentation frames must never delay the user's own input.

| ID | Behavior | Breaks if lost | Verified by |
|---|---|---|---|
| TP-FMP-CLIENT-01 | An exact input event cannot sit behind a stale burst of complete semantic presentation frames. | Typing lags behind a redraw storm, so keystrokes land seconds after they were pressed. | `fmp_client_input_precedes_semantic_frame_backlog` |
| TP-FMP-CLIENT-02 | Complete semantic frames are replaceable snapshots: the client retains the newest one instead of replaying every stale intermediate frame. | The UI animates through a backlog of obsolete states before showing the current one. | `fmp_client_semantic_frame_burst_keeps_only_newest_snapshot` |
| TP-FMP-CLIENT-03 | Incremental terminal frames are not complete snapshots, so they remain lossless and ordered on the control lane. | Terminal output is coalesced like a snapshot and characters are lost. | `fmp_client_terminal_frames_remain_lossless_and_ordered` |
| TP-FMP-CLIENT-04 | Input-first does not mean control starvation: after one bounded input quantum, one ready ordered event must make progress. | Sustained input starves the control lane and the session stops responding to anything else. | `fmp_client_input_quantum_yields_to_ordered_control` |

## Notes for the next sync

- `TP-REPAINT-2B` is the only row here that asserts something *does not* happen. A
  resolution that makes every mouse move repaint will pass every other test in the
  suite.
- `TP-FMR-SIDEBAR-HL-01..03` are pinned by a single live headless test on purpose:
  the App-level equivalent cannot reach the real remote-client boundary.
- The `TP-ACT-*` and `TP-REPAINT-*` families were invisible to the registry until
  2026-07-25 — their markers did not follow `TP-<FAMILY>-<NN>` and the checker
  truncated them into `TP-A` and dropped `TP-2B..2F` entirely. Both families sit in
  upstream-owned files, which is exactly the bucket where invisibility costs most.
