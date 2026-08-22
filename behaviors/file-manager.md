# File manager — registered behaviors

Fork feature. Upstream has no `src/fm`, so nothing upstream constrains these
behaviors and nothing upstream will preserve them: every entry here is ours to
keep alive across syncs.

Format and rules: [`README.md`](README.md).

---

## Row ordering

The file manager sorts rows by modification time, newest first, and falls back
to a natural name order only when the timestamps tie. That single decision is
the root of a whole class of test fragility, so it is documented before
anything else: four tests were silently order-dependent on it and only surfaced
during the 2026-07-25 upstream sync, when a larger suite slowed the run enough
to separate timestamps that used to land in the same filesystem tick.

| ID | Behavior | Breaks if lost | Verified by |
|---|---|---|---|
| TP-MTIME-01 | Freshness outranks entry kind. A newer file sorts above an older directory, a symlink is ordered by its own timestamp rather than its target's, and special filesystem entries take part in the same ordering instead of being hidden. | Rows regain a directory-first grouping, so the most recently touched work stops appearing at the top — the entire reason the list is time-ordered. | `newer_file_sorts_before_older_directory`, `symlink_uses_its_own_modification_time`, `special_entry_participates_in_mtime_ordering`, `snapshot_symlink_classification_is_independent_of_mtime_sort`, `deleted_dir_entry_stays_visible_and_sorts_as_unknown` |
| TP-MTIME-02 | The rule is symmetric: a newer directory sorts above an older file, with no directory-first exception in either direction. | A one-sided comparator reintroduces kind-based grouping through the back door and the ordering stops being explainable. | `newer_directory_sorts_before_older_file`, `newer_file_sorts_before_older_directory` |
| TP-MTIME-03 | Equal timestamps fall back to one deterministic natural-then-raw name order shared across kinds, and an entry whose metadata vanished between `read_dir` and preparation stays visible while sorting last as unknown. | Row order becomes unstable between identical runs, and a file deleted mid-listing disappears from the panel instead of being shown as unreadable. | `equal_mtimes_use_natural_then_raw_name_order`, `equal_mtime_entries_use_natural_order_across_kinds`, `fmstate_opens_with_cursor_at_top`, `deleted_dir_entry_stays_visible_and_sorts_as_unknown` |
| TP-MTIME-04 | Any test that asserts a row index, a row label, or a selection order pins its fixture timestamps first, through `pin_equal_fixture_mtimes`. | Tests pass on a fast machine and fail on a slow one. Four tests already did exactly this and were each misread as an upstream merge regression before the real cause was found. | `branch_change_retires_descendant_focus_and_rebinds_ancestor`, `app_copy_action_prepares_exact_selection_without_filesystem_work`, `compute_view_refreshes_and_clears_file_manager_action_bar_content`, `stale_worker_completion_after_scroll_is_rejected` |

---

## Notes for the next sync

- `sort_entries` in `src/fm/mod.rs` is the single ordering authority. If an
  upstream change ever introduces a second sort, these four entries are the
  contract it has to satisfy.
- TP-MTIME-04 is a testing rule rather than a product behavior, but it belongs
  here: it is the guard that keeps the other three honest.
| TP-FM-COPYPATH-01 | The native FM header offers `[copy path]`: it puts the open directory's absolute path on the clipboard through `request_clipboard_write` — the road every other copy in the app rides — needs no selection, raises no operation, and is always enabled while the Trail owns focus. | The header names the directory but offered no way to take that name with you — the reported gap ("dizinler yazıyor ama üst tarafta dizin kopyalama yok"). A second clipboard road would drift from OSC52/feedback semantics the existing road already carries. | `copy_path_puts_the_open_directory_on_the_clipboard_road`, `copy_path_with_no_file_manager_does_nothing` |
| TP-FM-DISMISS-01 | A click on an agent's row (expanded panel, collapsed rail, or phone drawer — every producer of the pane-focus click) dismisses the Files stage before focusing the pane. | Clicking the ALREADY-focused agent while Files covered the center was a silent no-op: focus did not change, no surface change fired, and the screen stayed on Files — the user's report verbatim. | `an_agent_row_click_dismisses_the_files_stage` |
| TP-FM-DIVIDER-01 | The FM column divider wears the palette's boundary tone (`overlay0`), never the surface tone. | `surface_dim` vanished against the panel background — the reported \"kolonlar arasi cizgi\" that made resizing feel blind. | `the_column_divider_reads_in_the_boundary_tone` |
| TP-FM-FILTER-01 | `/` filters the ACTIVE trail column as a projection: only matching names become rows (each still carrying its TRUE entry index), movement walks the matches and clamps inside them, an empty match set rejects moves, and after every keystroke the cursor is normalized onto a match — so operations always target an entry the person can see | A filter that re-indexed rows would hand copy/rename/delete a ghost target one off from the highlighted name; a cursor left standing on a filtered-out entry is the same ghost by another road | `a_filter_projects_only_matching_names_and_moves_within_them`, `a_filter_with_no_matches_rejects_movement`, `a_filtered_column_projects_matching_rows_with_true_indices`, `typing_normalizes_the_cursor_onto_a_match` |
| TP-FM-FILTER-02 | The filter keys to the DIRECTORY it was typed in, not a column index: other columns are untouched, entering a directory clears it, a column that leaves the screen drops it on sync, an empty pattern hides nothing, and matching folds case | Trail columns shift as branches open and close — an index-keyed filter would silently start narrowing somebody else's directory; a pattern that survived into the next directory is how "where did my files go" tickets are made | `the_filter_keys_to_its_directory_and_an_empty_pattern_is_everything`, `the_filter_matches_case_insensitively`, `a_filter_for_a_vanished_column_is_dropped_on_sync` |
| TP-FM-FILTER-03 | The editor's roads: `/` (and the header's `[search]` verb) opens it; typed characters narrow live, Backspace shrinks (and walks out from empty), Enter keeps the filter and hands the keys back, Esc drops it whole — and in normal keys Esc clears an accepted filter before it clears the selection. The live pattern rides the identity line with a caret while typing | A filter reachable only by keyboard is invisible to the mouse-first surface; an Esc that jumped straight to clearing the selection would strand the filter as the one thing Esc cannot reach; and a filter with no on-screen echo narrows the listing invisibly | `the_slash_road_types_a_filter_and_esc_walks_back_out`, `the_identity_line_carries_the_live_filter`, `file_manager_action_catalog_matches_supported_dispatch_seams`, `header_action_areas_progressively_hide_and_fail_closed` |
