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
