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
| TP-FIP-FORMAT-05 | An image with no decoder settles as a stated limit from its extension alone, and the worker leaves it there: the file is never opened, and the state never decays into a decode failure. | Selecting a large SVG reads it from disk to rediscover what its name already said, and a boundary is presented to the user as an error. Before this, such a file fell through to the text reader and produced an image icon above "text preview source is binary". | `an_undecodable_image_format_never_reaches_the_decoder` |
| TP-FIP-IMAGE-QUIET-01 | A decode in flight draws no label; only resting states — no decoder, a failure, no kitty graphics — speak. | A "loading" label flickers once per row as the cursor moves through a directory. Optimised, a decode is about 16 ms, so the label never informs anyone and only draws attention to itself. | `a_decode_in_flight_draws_no_label_but_resting_states_still_speak` |

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

## Workbook preview — read natively, not deferred to a provider

`xlsx` and its relatives used to sit in the same extension list as `docx`, and
that list resolves through an *optional plugin provider*. No provider is ever
supplied — `preview_capability` is called from exactly one place, with a
hardcoded `PreviewProviderSet::default()` — so the branch that would use one is
unreachable. Every workbook therefore rendered as `(metadata only)`, always,
with no way to configure otherwise. These rows keep the native reader wired in.

The reader deliberately does **not** evaluate formulas: it shows the value the
writing application cached, which is what the user saw in Excel. An evaluator is
a parser, an AST, a dependency graph, cycle detection and a function library —
a separate product (`docs/patterns/document-rendering.md` DR12/DA9).

| ID | Behavior | Breaks if lost | Verified by |
|---|---|---|---|
| TP-FSH-01 | Workbook extensions resolve to the native reader; document formats with no reader stay honest metadata. | The reported defect returns: every spreadsheet shows "(metadata only)" and no configuration can change it, because the provider branch is unreachable. | `workbooks_route_to_the_native_sheet_reader` |
| TP-FSH-02 | The classifier routes exactly the extension set the reader accepts. | The two lists drift and a workbook is either routed to a reader that refuses it, or refused by a router the reader would have handled — the same defect FAZ A fixed for images. | `every_readable_workbook_extension_is_classified_as_a_sheet` |
| TP-FSH-03 | Cell values, sheet names and numeric classification survive the read. | The preview shows a grid that does not match the file, which is worse than showing nothing. | `reads_cell_values_and_sheet_names` |
| TP-FSH-04 | Every sheet name is read even though only the first is materialised. | Sheet switching has no input to work from, and the preview silently implies a multi-sheet workbook has one sheet. | `reads_every_sheet_name_while_materialising_only_the_first` |
| TP-FSH-05 | A sheet larger than the window is truncated to it and still reports its real size. | `calamine` materialises a range densely, so a large sheet becomes an out-of-memory kill triggered by moving the cursor onto a file (DA8). | `large_sheet_is_windowed_and_reports_its_real_size` |
| TP-FSH-06 | Damaged workbooks and files whose extension lies fail as typed errors, never panics. | A malformed download takes the process down from a directory listing — about as untrusted as input gets. | `damaged_and_misnamed_workbooks_fail_without_panicking` |
| TP-FSH-07 | An empty sheet is a valid preview, not a failure. | A workbook that simply has nothing in it reads as a bug in herdr. | `empty_sheet_is_a_valid_preview` |
| TP-FSH-08 | A formula cell shows the cached value, not the expression. | The panel shows `=B1*A1` where Excel showed `84`, which reads as a broken preview rather than a deliberate limit. | `formula_cells_show_the_cached_value_not_the_expression` |
| TP-FSH-09 | Newlines and tabs inside a cell collapse to spaces. | One cell breaks its row into pieces and every column after it loses alignment. | `control_characters_in_a_cell_collapse_to_one_line` |
| TP-FSH-10 | Columns are padded by display width, so alignment holds across double-width glyphs. | Columns line up for ASCII data and silently shear for everyone else — the group least likely to appear in a casual manual check. | `sheet_rows_align_columns_by_display_width`, `column_width_uses_display_width_and_is_capped` |
| TP-FSH-11 | Selecting a workbook resolves end to end: pending sheet state, bounded worker, applied result, trail detail. | The layers each stay correct while the panel still shows the wrong thing — which is exactly how the original defect hid from unit tests. | `selecting_a_workbook_resolves_to_a_sheet_preview_end_to_end` |
| TP-FSH-12 | A workbook result whose generation no longer matches is dropped, leaving pending state intact. | A slow read lands after the user moved on and overwrites the preview of a different file (DR7). | `a_stale_workbook_result_cannot_replace_the_current_preview` |
| TP-FSH-13 | An oversized workbook is refused before the parser is handed anything. | The cheap outer gate disappears and the inner row/column ceilings become the only defence against a file that expands enormously. | `oversized_workbook_is_refused_before_parsing` |
| TP-FSH-14 | The header names the active sheet and says how many others exist. | A multi-sheet workbook silently presents its first sheet as the whole file. | `sheet_header_names_the_active_sheet_and_counts_the_rest` |
| TP-FSH-15 | Opening the file manager onto a workbook classifies it as one. | The immediate path fell through to the text reader, so a workbook selected at open time was read as text until the cursor moved away and back. | `opening_onto_a_document_prepares_it_as_that_document` |

## PDF preview — rasterised natively, one page at a time

`pdf` sat in the same unreachable optional-plugin branch workbooks did, so it
showed "(metadata only)" for every document. It is now rendered in-process with
`hayro`: pure Rust, no external binary to locate and no library to ship beside
the executable, which is what made the project's earlier rejection of PDF
preview obsolete — that rejection was about shipping `pdfium`, not about the
feature.

A page resolves to the same `PreparedImagePreview` an image does, so the entire
Kitty delivery path applies unchanged rather than growing a parallel one.

| ID | Behavior | Breaks if lost | Verified by |
|---|---|---|---|
| TP-FPDF-01 | Only the requested page is rasterised, and the document's real length is reported with it. | Opening a long PDF costs time proportional to its length for a panel that shows one page; and without the count there is nothing to drive page navigation. | `renders_the_requested_page_and_reports_the_document_length` |
| TP-FPDF-02 | A page is composited onto white before it leaves the reader. | A PDF page carries no background of its own, so the terminal's dark background shows through and dark body text becomes unreadable — while opaque boxes stay legible, which is what makes the bug misleading rather than obvious. | `a_blank_page_is_composited_onto_white_not_left_transparent` |
| TP-FPDF-03 | A page index past the end is refused with both the index and the total. | Page navigation walks off the end into a panic or a blank panel instead of stopping. | `a_page_past_the_end_is_refused_with_both_numbers` |
| TP-FPDF-04 | Damaged, non-PDF and encrypted input fails as a typed error, never a panic. | `hayro` is experimental and explicitly does not support encrypted documents; a downloaded PDF would take the process down from a directory listing. | `damaged_and_non_pdf_input_fails_without_panicking` |
| TP-FPDF-05 | Encoded size, empty target and projected pixel count are all refused before rendering. | A single page can expand to an arbitrary raster, and the cost is paid before anything notices. | `size_and_target_gates_refuse_before_rendering` |
| TP-FPDF-06 | The page is rendered to fit the requested target, preserving aspect ratio. | Kitty scales an image to exactly fill the cell box it is given, so a portrait page stretches across a wide pane unless the fit is computed here. | `the_page_is_rendered_to_fit_the_requested_target` |
| TP-FPDF-07 | PDFs route to the native rasteriser; document formats with no reader stay metadata. | The unreachable-provider defect returns for PDFs specifically. | `pdfs_route_to_the_native_rasteriser` |
| TP-FPDF-08 | Two different pages of the same size do not share a data fingerprint. | The encoder decides from the fingerprint whether the picture on screen is still current; deriving it from anything but the pixels means the second page is treated as already drawn and never sent. | `renders_the_requested_page_and_reports_the_document_length` |

## PDF page navigation — bounded, never wrapping

`PageDown`/`PageUp` and the two arrows in the `< N / M >` indicator move through
the document. The arrows are ASCII because the file manager's existing
single-cell click targets are (`FileManagerRowAction::label`), and because
`◀`/`▶` are East Asian Ambiguous width: hosts disagree on whether they occupy
one cell or two, which would move the glyph out from under its own hit target.

`PageDown`/`PageUp` rather than the arrow keys: `Left`/`Right` already move
between Trail columns, so taking them would mean selecting a PDF stops the file
manager from navigating directories.

Turning a page is only a change of `page`. `sync_image_preview_worker` rebuilds
its key from it every turn, so the request follows without a second authority
deciding when to re-render.

| ID | Behavior | Breaks if lost | Verified by |
|---|---|---|---|
| TP-FPDF-09 | Turning back from the first page is refused, never wrapped. | Holding the arrow down silently jumps to the far end of the document with no event to tell the reader it happened. | `turning_back_from_the_first_page_is_refused` |
| TP-FPDF-10 | Turning forward from the last page is refused, never wrapped. | Same, in the other direction; and stepping past the end resolves to `PageOutOfRange`, turning navigation into an error message. | `turning_forward_from_the_last_page_is_refused` |
| TP-FPDF-11 | Each step moves exactly one page. | The reader loses their place in a document they are reading sequentially. | `turning_pages_moves_one_page_at_a_time` |
| TP-FPDF-12 | With no page rendered yet, forward is refused rather than guessed; backward stays available. | The upper bound is unknown before a render, so guessing it lands on `PageOutOfRange`. The lower bound is known without reading anything. | `turning_forward_without_a_known_total_is_refused` |
| TP-FPDF-13 | The document length survives a page turn. | Held inside the ready state it was lost the moment a turn put the preview back into rasterising, so the forward bound vanished and the reader could only move one page per completed render. | `turning_pages_does_not_forget_how_long_the_document_is` |
| TP-FPDF-14 | A preview that is not a PDF has no page to turn. | Page navigation leaks into text, workbook and image previews. | `turning_a_page_without_a_pdf_preview_is_inert` |
| TP-FPDF-15 | The indicator numbers pages from one while the state stays zero-based. | The two conventions meet in more than one place and every page is shown off by one. | `pdf_page_indicator_numbers_pages_from_one` |
| TP-FPDF-16 | An arrow with nowhere to go is neither drawn nor clickable. | A dim-but-clickable arrow reads as a frozen application the moment someone clicks it and nothing happens. | `pdf_page_indicator_omits_the_arrow_it_cannot_follow` |
| TP-FPDF-17 | Each hit target covers exactly the cell its arrow is drawn in. | A wider zone turns the empty half of the status line into a hidden button; an offset one clicks the wrong thing. | `pdf_page_indicator_zones_sit_on_their_own_glyph` |
| TP-FPDF-18 | A hit target never lands outside the row it was measured against. | A rect that overhangs the row places a click target on top of a neighbouring widget. | `pdf_page_indicator_zones_stay_inside_a_narrow_row` |
| TP-FPDF-19 | A document with no pages produces no indicator. | An empty document draws arrows that point at nothing. | `pdf_page_indicator_needs_at_least_one_page` |
| TP-FPDF-20 | The arrows reach the screen in the cells their hit targets claim. | Render and input agree on a value that does not match what was drawn, and every click lands one cell off. | `rendered_page_indicator_arrows_land_on_their_hit_targets` |
| TP-FPDF-21 | `PageDown`/`PageUp` turn the previewed PDF's pages. | The only keyboard route through a document disappears. | `page_keys_turn_the_previewed_pdf` |
| TP-FPDF-22 | The same keys over any other preview change nothing, including the Trail cursor. | The new binding leaks into unrelated previews. | `page_keys_are_inert_without_a_pdf_preview` |
| TP-FPDF-23 | Clicking an arrow turns the page it points at. | The indicator is a PDF preview's only mouse affordance; without it a mouse-first file manager has no page navigation. | `clicking_the_page_indicator_arrows_turns_the_pdf` |
| TP-FPDF-24 | The rest of the indicator row is not a button. | The empty half of the status line silently turns pages. | `clicking_beside_the_page_indicator_leaves_the_page_alone` |
| TP-FPDF-25 | Turning a page submits a render for that page, and the length learned earlier outlives it. | Either the new page never renders, or the forward bound is forgotten and a second turn is refused while the first is still rasterising. | `turning_a_page_rasterises_it_without_forgetting_the_document_length` |
| TP-FPDF-26 | A render that lands after a turn is rejected. | The wrong page is installed with no further event to correct it. | `a_page_render_that_lands_after_a_turn_is_rejected` |
| TP-FPDF-27 | Opening the file manager onto a PDF or a workbook classifies it the same way moving the cursor onto one does. | The immediate path fell through to the text reader, so a PDF selected at open time was shown as its own raw bytes until the cursor moved away and back. | `opening_onto_a_document_prepares_it_as_that_document` |

## Opening a file: which action is offered

A plugin action may name the extensions it handles. Naming none means every
file, because every manifest written before the field existed omits it — and
reading empty as "nothing" would silently disable every installed plugin.

An action that does not handle the selection is **absent** from the menu, not
disabled. A greyed-out entry says "not right now"; this one does not apply to
these files at all. Offering it anyway is what launched the spreadsheet editor
on a PDF and left an empty tab with nothing to explain it.

Built-in `Open` only descends into directories, so a picture needs its own
entry: `Enlarge` opens the full-frame viewer. The menu decides from the file
name, which is all a pure projection has; the execution path checks the live
preview again, so a name that promises a picture herdr cannot decode is refused
there rather than opening the viewer onto an empty frame.

| ID | Behavior | Breaks if lost | Verified by |
|---|---|---|---|
| TP-FOPEN-01 | An action naming extensions matches only those files. | The reported defect: a spreadsheet plugin offered on a PDF, launching an editor that cannot read it. | `an_action_that_names_extensions_matches_only_those_files` |
| TP-FOPEN-02 | An action naming no extensions matches every file. | Every installed plugin goes silently offline. | `an_action_without_extensions_matches_every_file` |
| TP-FOPEN-03 | Extension comparison ignores case. | `.XLSX` downloads make the plugin look like it randomly fails to appear. | `extension_matching_ignores_case` |
| TP-FOPEN-04 | Every path in the selection must match. | The wrong program runs on the files that did not match. | `a_partly_matching_selection_does_not_match` |
| TP-FOPEN-05 | A file with no extension matches only an unrestricted action. | Either a panic on the way to the answer, or an action offered where it cannot apply. | `a_file_without_an_extension_matches_only_an_unrestricted_action` |
| TP-FOPEN-06 | The last dotted segment is the extension. | `archive.tar.gz` classification drifts between callers. | `a_multi_dotted_name_matches_its_last_segment` |
| TP-FOPEN-07 | An empty selection matches nothing. | "All paths match" is vacuously true for an empty list, so every action would be offered with no file to run on. | `an_empty_selection_matches_nothing` |
| TP-FOPEN-08 | A spreadsheet action is absent from a PDF's menu and present on a workbook's. | The defect returns, or the fix over-corrects and hides the action where it belongs. | `a_plugin_action_is_offered_only_for_the_extensions_it_handles` |
| TP-FOPEN-09 | An unrestricted action stays in every file's menu. | Backward compatibility for every manifest on disk today. | `a_plugin_action_without_extensions_is_offered_on_every_file` |
| TP-FOPEN-10 | A partly-matching selection is not offered the action. | The defect, one selection wider. | `a_plugin_action_is_withheld_from_a_partly_matching_selection` |
| TP-FOPEN-11 | `.XLSX`, `xlsx` and ` csv ` all reduce to one stored form; empties are dropped. | Matching needs three rules instead of one, and a stray empty string becomes an extension no file has. | `plugin_manifest_normalizes_action_file_extensions` |
| TP-FOPEN-12 | A manifest without the field still parses. | Every installed plugin fails to load. | `plugin_manifest_without_file_extensions_still_parses` |
| TP-FOPEN-13 | A file with a picture offers `Enlarge`. | A PDF or image has no working entry in the menu at all. | `a_picture_offers_enlarge` |
| TP-FOPEN-14 | A file drawn from cells disables `Enlarge` with a reason. | An entry that looks available and does nothing reads as a frozen application. | `a_file_without_a_picture_disables_enlarge_with_a_reason` |
| TP-FOPEN-15 | A multiple selection disables `Enlarge`. | The viewer shows one file; which one is left ambiguous. | `a_multiple_selection_disables_enlarge` |
| TP-FOPEN-16 | Choosing `Enlarge` opens the viewer on that file. | The entry exists and does nothing, which is worse than not offering it. | `context_enlarge_opens_the_viewer_on_the_named_file` |
| TP-FOPEN-17 | The execution path re-checks the live preview. | A name that promises a picture herdr cannot show opens the viewer onto an empty frame. | `context_enlarge_refuses_a_file_with_no_picture` |
| TP-FOPEN-18 | An intent naming a file that is no longer selected opens nothing. | A menu opened before the cursor moved enlarges whatever happens to be selected now. | `context_enlarge_ignores_an_intent_for_a_different_file` |

## Opening a file in its own tab (`herdr view`)

`herdr view <PATH> [--page N]` shows one file in the pane it runs in, so a plugin
action can open a picture in a tab the way the spreadsheet editor opens a workbook.
No external tool and no new dependency: the readers, the cell-size probe and the
Kitty emitter were already here.

Two rules shape the whole layer:

- **What to draw is pure.** `compute_frame` takes a path, a page and a cell grid
  and returns a `ViewerFrame`. No terminal, no clock, no escape sequence — so the
  interesting cases are unit tests rather than manual checks.
- **A file herdr cannot read is a frame, not an error.** Exiting on a bad file
  closes the tab the instant it opened, which reads as a crash. The reason goes in
  the status line and the tab stays up.

| ID | Behavior | Breaks if lost | Verified by |
|---|---|---|---|
| TP-FVIEW-TAB-01 | An image resolves to a frame with pixels and a status line naming it. | The feature does not exist. | `an_image_resolves_to_a_drawable_frame` |
| TP-FVIEW-TAB-02 | An undecodable file still produces a frame carrying the reason. | The tab closes instantly and reads as a crash. | `an_undecodable_file_still_produces_a_frame_with_the_reason` |
| TP-FVIEW-TAB-03 | A missing file is a frame, not an error. | Same, by a different route. | `a_missing_file_produces_a_frame_rather_than_an_error` |
| TP-FVIEW-TAB-04 | A grid with no room for picture and status yields no frame. | A host image lands over the line meant to label it, and cells drawn under a Kitty placement do not erase it. | `a_grid_too_small_to_hold_the_status_line_yields_no_frame` |
| TP-FVIEW-TAB-05 | An unknown cell size yields no frame. | A guessed grid places the picture in the wrong cells with nothing on screen to explain it. | `an_unknown_cell_size_yields_no_frame` |
| TP-FVIEW-TAB-06 | The picture is centred and never overhangs its grid. | Kitty scales to exactly the cell box it is handed, so an overhanging box stretches the picture. | `the_picture_is_centred_and_stays_inside_the_grid` |
| TP-FVIEW-TAB-07 | The same inputs give the same frame. | The layer's testability rests on this. | `computing_the_same_frame_twice_gives_the_same_answer` |
| TP-FVIEW-TAB-08 | Page turning clamps at both ends and never wraps. | The two surfaces show one document; disagreeing makes a page turn mean different things depending on where the file was opened. | `turning_pages_clamps_at_both_ends_and_never_wraps` |
| TP-FVIEW-TAB-09 | Without a known page count, forward is refused rather than guessed. | A guess past the end resolves to `PageOutOfRange`, turning navigation into an error message. | `turning_forward_without_a_page_count_is_refused` |
| TP-FVIEW-TAB-10 | Leaving deletes every picture the process placed. | Kitty images outlive the alternate screen, so the reader gets their shell back with a picture hanging over it. | `leaving_deletes_the_pictures` |
| TP-FVIEW-TAB-11 | A frame with no room still writes something. | A blank screen is indistinguishable from a hang. | `a_missing_frame_still_says_something` |
| TP-FVIEW-TAB-12 | The status line is truncated to the terminal width. | A longer line wraps, scrolling the picture out of the box it was placed in. | `the_status_line_is_truncated_to_the_width` |
| TP-FVIEW-TAB-13 | A bare path opens the first page. | The common invocation stops working. | `a_bare_path_opens_the_first_page` |
| TP-FVIEW-TAB-14 | `--page` is one-based in and zero-based out. | The viewer prints "page 3"; asking the reader to type 2 for it is a trap. | `the_page_option_is_one_based_for_the_reader` |
| TP-FVIEW-TAB-15 | Malformed arguments produce a message, never a panic. | A panic inside a pane prints a backtrace nobody asked for and the tab closes on it. | `malformed_arguments_are_refused_with_a_message` |

## Sending a file over Tailscale

Right-click a file in the manager and pick **Send with Tailscale...**; a centred
picker lists the machines on the tailnet and `Enter`, or a click on a row, hands
the selection to Taildrop.

- **Built in rather than a plugin.** The destination has to be chosen, and a
  plugin action runs headless — it has nowhere to ask.
- **Identity is the DNS name, not the host name.** Host names repeat on a real
  tailnet; sending to a repeated one is ambiguous, and picking the wrong machine
  is not recoverable from herdr.
- **Offline machines are listed, marked, and sorted last.** Taildrop queues for a
  machine that is not up yet; hiding them answers "where is my laptop?" with
  silence.

| ID | Behavior | Breaks if lost | Verified by |
|---|---|---|---|
| TP-FSEND-TS-01 | The local machine is never a destination. | Taildrop refuses a send to itself, so the entry can only fail. | `the_local_machine_is_not_a_destination` |
| TP-FSEND-TS-02 | Reachable devices are listed before unreachable ones. | The two machines that can receive are buried under thirteen that cannot. | `reachable_devices_are_listed_first` |
| TP-FSEND-TS-03 | Machines sharing a host name fall back to their DNS label. | Two rows read the same and picking between them is guesswork. | `duplicate_host_names_are_disambiguated` |
| TP-FSEND-TS-04 | The send target is the unique DNS name, without its trailing dot. | An ambiguous target sends to whichever machine tailscale resolves first. | `the_send_target_is_the_unique_name` |
| TP-FSEND-TS-05 | A tailnet of one is an empty list, not a failure. | The reader goes looking for a fault that is not there. | `a_tailnet_of_one_is_empty_not_broken` |
| TP-FSEND-TS-06 | Unreadable status output is a message, never a panic. | A panic while opening a menu takes herdr down with it. | `unreadable_status_is_refused_with_a_message` |
| TP-FSEND-TS-07 | File names cannot be read as options. | A file named `-n` is an ordinary file; without `--` the send fails on something that was never a flag. | `file_names_cannot_be_read_as_options` |
| TP-FSEND-TS-08 | The outcome names the files and stays one line. | "Sent" alone leaves the reader checking whether what went is what they meant. | `the_outcome_names_what_was_sent` |
| TP-FSEND-TS-09 | A folder is refused in herdr's own words, before anything is spawned. | Taildrop handed a directory reports nothing useful and the send looks like it worked. | `folders_are_refused_with_a_reason` |
| TP-FSEND-TS-10 | An offline device is marked in text, not by colour alone. | A monochrome terminal shows no difference and the reader cannot tell the file will wait. | `an_offline_device_is_marked_in_text` |
| TP-FSEND-TS-11 | A frame with no room draws nothing rather than shrinking. | A clamped box overlaps its own border and lists devices nobody can act on. | `a_frame_with_no_room_draws_nothing` |
| TP-FSEND-TS-12 | A long tailnet stops growing the box. | A box taller than the screen cannot be closed by eye. | `a_long_device_list_is_bounded` |
| TP-FSEND-TS-13 | A click selects the row it landed on. | Recomputing geometry in the mouse path lands one row off and sends to the wrong machine. | `a_click_selects_the_row_it_landed_on` |
| TP-FSEND-TS-14 | The title names the file being sent. | A picker saying only "Send" leaves the reader guessing which file is going. | `the_title_names_what_is_being_sent` |
| TP-FSEND-TS-15 | The device names reach the frame buffer. | An overlay wired to a rect nothing is drawn into looks exactly like a broken feature. | `the_device_names_are_drawn_into_the_frame` |
| TP-FSEND-TS-16 | An empty tailnet says so on screen. | An empty box and a failed lookup are indistinguishable unless one is spelled out. | `an_empty_tailnet_says_so_on_screen` |
| TP-FSEND-TS-17 | Choosing the menu entry opens the picker on the named files. | The entry exists and appears to do nothing. | `context_send_tailscale_opens_the_picker_on_the_named_files` |
| TP-FSEND-TS-18 | The picker survives a full frame, not just its own draw call. | The stage underneath paints over it and the feature is invisible. | `the_picker_is_visible_in_a_full_frame` |
| TP-FSEND-TS-19 | A pinned device outranks an online one. | A pinned laptop that is asleep drops below strangers and the pin is worthless. | `a_pinned_device_outranks_an_online_one` |
| TP-FSEND-TS-20 | Pins keep the order they were added in. | Re-sorting them alphabetically defeats the point of choosing an order. | `pins_keep_the_order_they_were_added_in` |
| TP-FSEND-TS-21 | Pinning toggles, and a new pin lands last. | Pushing new pins to the front moves the reader's top slot every time. | `pinning_toggles_and_appends` |
| TP-FSEND-TS-22 | Pinned and online are two marks in two columns. | Folding them into one glyph makes the list lie about one of them. | `pinned_and_online_are_marked_independently` |
| TP-FSEND-TS-23 | The uncut journey: right-click, Enter on the entry, worker tick, picker open. | The layers pass in isolation while the user watches the menu close on nothing. | `menu_enter_on_send_with_tailscale_opens_the_picker` |
| TP-FSEND-TS-24 | The same journey by mouse click. | Keyboard and mouse take different routes to the same dispatch; only one was covered. | `menu_click_on_send_with_tailscale_opens_the_picker` |
| TP-FSEND-TS-25 | The headless scheduler consumes queued context-menu intents. | In server mode every context action queues an intent nothing reads: the menu closes and nothing happens — the reported bug. | `headless_scheduler_consumes_context_menu_intents`, `scheduler_parity_headless_vs_monolithic` |
| TP-FSEND-TS-26 | The hit test agrees with the rendered cells, not with its own layout. | Layout and render sharing a mistake passes a self-consistent test while every click selects the machine above the cursor. | `a_click_lands_on_the_device_that_is_drawn_there` |
| TP-FSEND-TS-27 | A successful send marks its row with ✓; a failed one does not. | The status line names only the last outcome, so an unsure reader presses again and the file goes out several times. | `a_successful_send_marks_the_device_row` |

## Editing text from the preview

Clicking a text preview opens the file in the editor tab, through the same
plugin intent the context menu's "Edit in New Tab" row emits — one dispatch
seam serves both paths. The editor itself is a plugin concern; the fresh config
shipped with the edit plugin turns on auto-save (2s), hot exit, and the menu
bar with clickable Undo/Redo.

| ID | Behavior | Breaks if lost | Verified by |
|---|---|---|---|
| TP-FEDIT-01 | Clicking a text preview queues the editor action for that exact file. | The panel the reader's eyes are on answers a click with nothing, while the picture panel next to it enlarges. | `clicking_a_text_preview_queues_the_editor_action` |
| TP-FEDIT-02 | A text preview with no matching editor swallows the click. | An intent nothing can run fails somewhere the reader cannot see. | `clicking_a_text_preview_without_an_editor_queues_nothing` |

## Watcher refreshes and the selection

The directory watcher fires for every change in the watched directory — most
often a SIBLING of the selection, because an auto-saving editor touches its
file every couple of seconds. A refresh must repaint what changed without
disturbing what did not.

| ID | Behavior | Breaks if lost | Verified by |
|---|---|---|---|
| TP-FMW-REFRESH-01 | A refresh caused by a sibling keeps the selection's preview, generation, and viewports; the selection follows its file through the mtime re-sort by path, never by row number. | The selected picture blinks to Pending on every auto-save next door, and the cursor visits whichever row number the file used to occupy. | `sibling_change_refresh_keeps_the_selected_preview_and_cursor` |
| TP-FMW-REFRESH-02 | A refresh in which the selected file itself changed does reset the preview. | The guard above fossilises a stale picture of a file that was rewritten on disk. | `selected_change_refresh_resets_the_preview` |
| TP-FMW-REFRESH-03 | A sibling refresh keeps the loaded trail DETAIL, not just the preview: a loaded text/workbook detail is re-applied after the refresh rebuilds it as pending. | Text and sheet panels render from the detail; preserving only the preview wedges them on "(loading preview...)" forever, because the worker sees a loaded preview and never re-submits. | `sibling_change_refresh_keeps_the_loaded_text_detail` |
| TP-FMW-REFRESH-04 | A watcher refresh keeps surviving rows in their previous relative order (with refreshed data); genuinely new paths append at the end. The full mtime re-sort happens only on user navigation. | Every auto-save leapfrogs the saved file to row 0, the rows reshuffle under the cursor every two seconds, and the focus highlight rides to the top of the column. | `watcher_refresh_keeps_the_row_order_stable`, `watcher_refresh_keeps_selection_by_path` |

## Enlarged preview viewer

`Enter`, or a click on the picture, opens the raster preview to fill the frame.
`Esc`/`q` close it and hand focus back to whoever had it.

The viewer holds no pixels of its own. `file_manager_raster_content_area` is the
single authority for "which rect does the raster preview live in", and the
viewer changes its answer — so the decode target, the Kitty placement and the
indicator hit test move together, and enlarging is a bigger decode rather than
an upscale of the panel-sized one.

It is deliberately **not** a surface-hiding overlay: the picture is a host image,
not cells in the frame buffer, so declaring it one would suppress the placement
pass and the viewer would open onto an empty frame.

| ID | Behavior | Breaks if lost | Verified by |
|---|---|---|---|
| TP-FVIEW-01 | The picture never overlaps the title and status rows. | A host image is not erased by the cells drawn under it, so an overlapping picture hides its own label permanently. | `viewer_content_leaves_room_for_its_title_and_status` |
| TP-FVIEW-02 | A frame too small for chrome and picture together yields no content rect. | Half a viewer places the image over the title it is supposed to be labelled by. | `a_frame_too_small_for_the_chrome_has_no_content_area` |
| TP-FVIEW-03 | `Enter` enlarges a picture; directories keep their meaning. | The one selection `Enter` had nothing to do with stays unusable, or directory navigation breaks. | `enter_opens_the_viewer_on_a_picture` |
| TP-FVIEW-04 | `q` and `Esc` close the viewer, not the application. | This is the forgotten tier — a suppressed global that must be gated on "no viewer open", or the first `q` inside the viewer quits herdr. | `q_and_esc_close_the_viewer_rather_than_the_app` |
| TP-FVIEW-05 | The viewer owns the keyboard: navigation keys do not move the selection behind it. | The file manager scrolls under a picture the user cannot see moving. | `the_viewer_owns_the_keyboard_while_it_is_open` |
| TP-FVIEW-06 | Page keys keep turning pages inside the viewer. | Enlarging a PDF gives a bigger first page and no way to read the rest. | `page_keys_still_turn_pages_inside_the_viewer` |
| TP-FVIEW-07 | Previews drawn from cells, and pictures with no decoder, refuse to open. | The viewer opens onto a full frame with nothing in it. | `the_viewer_refuses_previews_that_have_no_picture` |
| TP-FVIEW-08 | Opening the viewer asks for more pixels than the panel did. | Reusing the panel's decode target stretches panel-sized pixels across the frame: a blurrier picture rather than a bigger one, indistinguishable from doing nothing. | `opening_the_viewer_asks_for_more_pixels_than_the_panel` |
| TP-FVIEW-09 | Resizing while the viewer is open produces a new decode target. | The classic graphics bug: a picture left placed against geometry that no longer exists. | `resizing_while_the_viewer_is_open_retargets_the_picture` |
| TP-FVIEW-10 | Closing restores the panel's decode target exactly. | The file manager keeps decoding at full-frame size for a panel-sized hole. | `closing_the_viewer_returns_the_target_to_the_panel` |
| TP-FVIEW-11 | Clicking the picture enlarges it; the indicator's arrows still turn pages. | The arrows sit inside the same rect, so without the ordering they would open the viewer instead of turning the page. | `clicking_the_picture_opens_the_viewer_but_the_arrows_still_turn_pages` |

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
