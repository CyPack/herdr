# Files surface, icons and agent reference — registered behaviors

Fork feature (`FIP` = Files Interaction Polish). Covers how the Files Stage is
entered from the sidebar, how entry rows are drawn, and how a path is handed to
a running agent.

The agent-reference contract is the sharpest part of this family and deserves
stating once, plainly:

> **Exactly the UTF-8 path bytes cross the boundary, once, with no submit byte,
> to the pane identity the user chose.** Everything in the `REF` section is a
> way that promise could be broken — a vanished pane, a retired identity, a
> multi-selection, a filename with spaces, a repeated failure.

Format and rules: [`README.md`](README.md).

## Entering and leaving the Files Stage

| ID | Behavior | Breaks if lost | Verified by |
|---|---|---|---|
| TP-FIP-NAV-01 | A primary click on the visible Files tab opens the Native Files Stage, not merely the visual tab. | The tab highlights but the surface never appears, so Files looks broken from the sidebar. | `files_tab_primary_click_opens_native_files_stage` |
| TP-FIP-NAV-02 | Reactivating Files from the visible tab keeps the open singleton surface without resetting file-manager state. | Clicking the tab while Files is open throws away the loaded branch and cursor. | `files_tab_click_reuses_open_singleton_files_stage` |
| TP-FIP-NAV-03 | Switching to Spaces or Projects while Files is open restores the terminal Stage client-locally, with identical terminal identities and no runtime mutation. | Leaving Files disturbs running panes or reattaches terminals to different identities. | `projects_tab_click_restores_terminal_stage_and_preserves_identity`, `spaces_tab_click_restores_terminal_stage_and_preserves_identity` |
| TP-FIP-NAV-04 | Modified, middle, release-only and outside clicks do not transition the Stage. | Stray gestures swap the whole surface out from under the user. | `modified_left_click_on_files_tab_does_not_activate_stage` |
| TP-FIP-NAV-08 | A collapsed sidebar exposes no Files tab target. | A hidden tab keeps a live hit target, so clicks in empty chrome switch surfaces. | `collapsed_sidebar_files_tab_is_inert` |

## Ancestor focus binding

| ID | Behavior | Breaks if lost | Verified by |
|---|---|---|---|
| TP-FIP-FOCUS-01 | Entering a child at a nonzero index binds that exact child path to the departing segment, never leaving it unset for a row-zero fallback. | Ancestor columns highlight the first row instead of the path actually taken. | `entering_nonzero_child_binds_exact_focused_child_in_departing_segment` |
| TP-FIP-FOCUS-02 | Descending four levels binds every resident ancestor to its exact next path segment, not only the immediate parent. | Deep paths show correct highlighting one level up and wrong highlighting above that. | `four_level_descent_binds_every_resident_ancestor_focus` |
| TP-FIP-FOCUS-05 | Changing branch through an ancestor retires the descendant focus together with its segments. | Stale descendant highlights survive a branch change and describe a path that is no longer open. | `branch_change_retires_descendant_focus_and_rebinds_ancestor` |
| TP-FIP-FOCUS-06 | The re-entered ancestor binds the NEW child after a branch change. | The ancestor keeps pointing at the previous branch. | `branch_change_retires_descendant_focus_and_rebinds_ancestor` |

## Entry row rendering and icons

| ID | Behavior | Breaks if lost | Verified by |
|---|---|---|---|
| TP-FIP-ICON-01 | Every entry row renders its prepared semantic icon in one leading display cell before the name. | Rows lose their kind at a glance, and column alignment drifts. | `classify_covers_all_six_entry_kinds`, `entry_row_renders_semantic_icon_before_name`, `snapshot_prepares_canonical_entry_kinds`, `supports_agent_reference`, `visual_class_kind_wins_over_extension` |
| TP-FIP-ICON-02 | The icon occupies exactly one display cell so names start at a fixed column. | Names no longer align, which makes scanning a directory slower than plain text. | `entry_row_renders_semantic_icon_before_name`, `visual_class_no_extension_maps_to_generic` |
| TP-FIP-ICON-06 | Extension matching is case-insensitive. | `README.MD` and `readme.md` get different icons. | `visual_class_extension_match_is_case_insensitive` |
| TP-FIP-ICON-07 | Exact well-known names win before extension matching. | A file named for its role gets a generic extension icon instead of its specific one. | `visual_class_uses_exact_name_override_before_extension` |
| TP-FIP-ICON-08 | A narrow column keeps the complete icon glyph and truncates the name by display cells, never by bytes; wide CJK cells are measured at glyph-start positions only. | Truncation splits a multi-byte character or a wide glyph, corrupting the row. | `icon_never_overlaps_row_action_cells`, `narrow_column_keeps_complete_icon_and_truncates_name_by_display_cells` |
| TP-FIP-ICON-09 | The cursor style owns the whole row including the icon cell, and multi-selection stays visually distinct from the cursor. | The icon cell escapes the cursor highlight, or selected rows become indistinguishable from the focused one. | `cursor_style_wins_over_icon_class_and_multi_select_stays_distinct` |
| TP-FIP-ICON-10 | Entry rows honour the client-local icon profile, so a deterministic ASCII fallback drives cross-machine visual fixtures; Nerd Font private-use glyphs render empty in a browser font. | Visual snapshots become machine-dependent and stop being comparable. | `entry_row_honors_ascii_icon_profile`, `every_visual_class_has_one_cell_glyph_in_both_profiles` |
| TP-FIP-ICON-11 | Render consumes prepared entry data only: a path that vanished after the snapshot renders byte-identically to a live one. | Rendering touches the filesystem, so a deleted file makes the frame differ or stall. | `render_entry_row_performs_no_filesystem_io`, `visual_class` |
| TP-FIP-ICON-13 | A hostile file name containing control characters renders as printable escapes and never shifts or clips row content. | A crafted filename rewrites the terminal around it — the classic escape-injection hole. | `control_characters_in_name_render_escaped_and_do_not_shift_rows`, `display_name`, `escape_control_chars`, `escape_control_chars_maps_every_control_to_printable` |

## Image formats — one table, four consumers

"Which image formats do we handle" used to be answered independently by the icon
classifier, the preview router, the decoder's own guard, and the `image` feature
list. Any two of them disagreeing produced a silent defect, and one did: a `.bmp`
carried an image icon and then failed as *binary text*, because the classifier
said image and the router did not. These rows exist so the four cannot drift
apart again.

| ID | Behavior | Breaks if lost | Verified by |
|---|---|---|---|
| TP-FIP-FORMAT-01 | Every format the table claims as decodable is decoded from a real sample at test time. | The `image` feature list is the one part that cannot be derived from the table, so a format claimed without its decoder ships: the router sends the file to the image path and the decoder rejects it, on a file we said we support. | `every_listed_format_actually_decodes` |
| TP-FIP-FORMAT-02 | Extension routing derives from the same table as decoder support, case-insensitively, and formats we deliberately do not decode (`svg`, `avif`) do not route there. | The routing list and the decoder list diverge again — the original defect — or an undecodable format reaches the decoder only to be refused. | `every_listed_extension_routes_to_the_image_preview` |
| TP-FIP-FORMAT-03 | A corrupt file of every listed format fails or produces bounded output, and never panics. | Each decoder added is new parsing surface reached straight from a directory listing, which is about as untrusted as input gets. | `corrupt_input_of_every_listed_format_fails_without_panicking` |
| TP-FIP-FORMAT-04 | Every decodable extension is classified as an image, so the icon and the preview agree. | A file previews as a picture while showing a generic icon: the same defect mirrored. The reverse is allowed — `.svg` looks like an image and is not decodable. | `every_decodable_image_extension_carries_the_image_icon` |

## Agent reference — the path-bytes contract

| ID | Behavior | Breaks if lost | Verified by |
|---|---|---|---|
| TP-FIP-REF-19 | A target pane closed while the picker is open renders as unavailable, and activating a target that disappeared after open fails closed with zero bytes and a visible failure. | A pane that closed mid-picker still receives bytes. | `activation_of_disappeared_target_fails_closed_with_visible_failure`, `fail_agent_reference_activation`, `target_pane_closed_while_picker_open_disables_row_on_recompute`, `terminal_identity_change_between_open_and_activation_sends_zero_bytes` |
| TP-FIP-REF-01 | Opening the agent reference picker performs no runtime work. | Merely opening a picker spawns processes or mutates panes. | `reference_action_opens_picker_from_live_agents_projection`, `sync_file_manager_agent_handoff` |
| TP-FIP-REF-02 | Selecting a row in the picker performs no runtime work; delivery alone crosses the App-owned send boundary. | Browsing the picker starts sending before the user confirms. | `current_focused_agent_is_first_and_preselected` |
| TP-FIP-REF-03 | A non-agent focused terminal does not trigger an implicit chat split for the reference action: no split request, no send request, no new pane or terminal. | Referencing a file in a plain shell silently launches an agent the user never asked for. | `non_agent_focus_prepares_no_claude_split_for_reference_action`, `send_agent_on_non_agent_terminal_prepares_no_authority` |
| TP-FIP-REF-04 | Delivery crosses the existing App-owned send boundary rather than a new path. | A second send path appears that bypasses the App's checks. | `file_manager_agent_handoff_is_current`, `keyboard_up_down_enter_and_mouse_click_share_selection`, `picker_selection_snapshots_full_target_identity`, `sync_file_manager_agent_handoff` |
| TP-FIP-REF-05 | The payload is exactly the UTF-8 path bytes and never a submit byte; the agent decides when to send. | The path is auto-submitted, executing whatever the agent had queued. | `existing_agent_receives_exact_path_bytes_with_no_submit`, `sync_file_manager_agent_handoff_send` |
| TP-FIP-REF-06 | A directory is a first-class reference target: its exact UTF-8 path bytes cross once, with no submit byte. | Directories cannot be referenced, or they are submitted automatically. | `directory_reference_delivers_exact_directory_path` |
| TP-FIP-REF-07 | The reference-only contract holds on every delivery path. | One path forgets the rule and submits. | `existing_agent_receives_exact_path_bytes_with_no_submit`, `sync_file_manager_agent_handoff_send` |
| TP-FIP-REF-08 | A workspace that vanished between prepare and send fails closed with zero bytes and one visible failure. | Bytes are written to whatever now occupies that slot. | `vanished_workspace_or_pane_sends_zero_bytes` |
| TP-FIP-REF-09 | The request is bound to the CHOSEN pane identity; when that pane's terminal no longer matches the snapshot, no bytes cross to the terminal that now lives there. | A path is typed into a different agent than the one the user picked. | `changed_terminal_identity_sends_zero_bytes` |
| TP-FIP-REF-10 | Exactly one failure is reported; later ticks stay silent. | A single failure repeats every tick and buries the surface in notices. | `vanished_workspace_or_pane_sends_zero_bytes` |
| TP-FIP-REF-11 | A missing or invalid target fails closed. | An invalid target is treated as valid and bytes cross anyway. | `deleted_path_before_send_sends_zero_bytes`, `reference_path_is_deliverable` |
| TP-FIP-REF-12 | A retired identity fails closed. | A stale identity still receives the payload. | `path_kind_change_to_special_before_send_sends_zero_bytes`, `reference_path_is_deliverable` |
| TP-FIP-REF-13 | A revalidation mismatch fails closed. | Revalidation is advisory rather than binding. | `control_character_path_disables_reference_action`, `non_utf8_path_rejects_at_prepare`, `reference_path_is_deliverable` |
| TP-FIP-REF-15 | The picker is a blocking overlay: background gestures are consumed fail-closed while it is open. | Clicks fall through to the surface underneath while a modal is up. | `escape_and_outside_click_close_picker_with_zero_bytes`, `handle_mouse`, `keyboard_up_down_enter_and_mouse_click_share_selection`, `picker_enters_overlay_mode_and_blocks_background_input` |
| TP-FIP-REF-16 | A non-live row cannot be activated by keyboard or mouse; the picker stays open with zero bytes prepared. | A dead target is selectable and produces a partial send. | `disabled_row_cannot_be_activated_by_keyboard_or_mouse`, `stale_source_row_or_context_does_not_open_picker` |
| TP-FIP-REF-17 | An explicit multi-selection disables the reference action: the intent carries multiple paths and opens nothing. | One path from a multi-selection is silently chosen and sent. | `multi_selection_disables_reference_action` |
| TP-FIP-REF-18 | Spaces and punctuation survive prepare, revalidation and delivery byte-for-byte; the validator never rejects them. | Ordinary filenames with spaces cannot be referenced. | `spaces_and_punctuation_paths_preserved_byte_for_byte` |

## Visual snapshots

| ID | Behavior | Breaks if lost | Verified by |
|---|---|---|---|
| TP-FIP-VIS-01 | The default terminal Stage and the activated Native Files Stage render from real exported Ratatui cells and match approved snapshots. | Layout regressions that compile and pass unit tests ship unnoticed. | `vis-01 terminal stage matches approved snapshot` |
| TP-FIP-VIS-02 | Descending through a nonzero child keeps both exact ancestor selections visible and never substitutes the first row. | The visual proof of correct ancestor highlighting disappears. | `vis-02 trail retains exact ancestor highlights`, `write_visual_fixtures` |
| TP-FIP-VIS-03 | A mixed-kind directory renders with the deterministic ASCII icon profile. | Icon snapshots depend on the fonts installed on the machine running them. | `vis-03 ascii icon classes match approved snapshot`, `write_visual_fixtures` |
| TP-FIP-VIS-04 | The same directory keeps base and inner isolation in its snapshot. | Unrelated chrome changes churn this fixture and it stops being a signal. | `vis-04 tiny screen icons match approved snapshot`, `write_visual_fixtures` |
| TP-FIP-VIS-05 | The blocking agent picker over the Files stage puts the current agent first and preselected. | The default target changes silently and users send to the wrong agent. | `vis-05 agent picker matches approved snapshot`, `write_visual_fixtures` |
| TP-FIP-VIS-06 | A disabled (vanished) second row is drawn as disabled on a tiny screen. | Unavailable targets look selectable at small sizes. | `vis-06 disabled row tiny picker matches approved snapshot`, `write_visual_fixtures` |

---

## Notes for the next sync

- `TP-FIP-ICON-13` is a security contract, not a cosmetic one: it is what stops
  a crafted filename from emitting escape sequences into the host terminal.
- `TP-FIP-REF-05` and `-07` forbid a submit byte. Any upstream change to how
  terminal input is encoded or flushed touches this promise.
- `TP-FIP-REF-19` was written as `TP-FIP-5.5` until 2026-07-25. The checker
  silently truncated it to `TP-FIP-5`, which is why the checker now rejects any
  marker that does not follow `TP-<FAMILY>-<NN>` outright.
