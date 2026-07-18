# Files Visibility Runtime Matrix

Date: 2026-07-18

## Outcome

FMR-1 is closed. The reported “some folders look empty” behavior was not a
Trail depth cap and was not caused by commits living under `/tmp`.

Three directory-entry classes were silently omitted from otherwise successful
snapshots:

1. dot-prefixed entries while hidden files are disabled;
2. names that are not valid UTF-8;
3. individual `ReadDir` iterator failures.

The native Files model now carries bounded omission counts without retaining
or logging entry names. Trail projection reserves a non-actionable status row
when any omission exists. Actionable rows keep their exact path/index hit
authority and remain visible beside the status.

## Executable provenance

| Surface | Evidence | Classification |
|---|---|---|
| Git commits | source and commit chain persisted after reboot | not `/tmp` loss |
| Plain `herdr` | `/home/ayaz/.local/bin/herdr`, dated 2026-07-12 | older installed build |
| Repository debug | `target/debug/herdr`, dated 2026-07-18 at investigation start | current source build |
| Live processes | installed and debug client/server processes both existed | no process was touched |

The reboot symptom was therefore partly executable drift: plain `herdr`
resolved to the older installed binary. This task did not install, stop,
restart, signal, or connect to either runtime.

## Classified directory matrix

| Case | Expected model/render | Evidence |
|---|---|---|
| ordinary visible entries | `Available`; sorted actionable rows | `dirs_first_then_natural_order` |
| genuinely empty | `Available`; no invented row | existing empty/status families |
| hidden-only | `Available`; no actionable hit; `hidden items omitted` | `directory_visibility_hidden_only_column_explains_omitted_items` |
| visible + hidden | visible exact-path row plus separate omission status | `directory_visibility_mixed_column_keeps_rows_and_explains_omissions` |
| non-UTF-8-only, Unix | no lossy path row; `unreadable names omitted` | `directory_visibility_non_utf8_only_column_explains_omitted_names` |
| iterator entry failure | bounded failure count, no `flatten()` loss | `directory_visibility_counts_iterator_entry_errors` |
| missing directory | `Missing`; empty entries; fail closed | `current_directory_status_distinguishes_available_missing_and_unavailable` |
| non-directory path | `Unavailable`; empty entries | same status family |
| permission error | deterministic `PermissionDenied` classification | `directory_error_kind_classification_is_platform_independent` |
| symlink directory | directory-target semantics preserved | `snapshot_sort_and_symlink_classification_baseline` |
| stale Trail/snapshot alignment | projects no geometry | `misaligned_snapshots_project_nothing` |
| fifth-to-sixth descent | deepest loaded column remains visible; no depth-5 cap | `deepest_column_scrolls_into_view`, `every_visible_column_is_loaded` |

Directory-level failures remain fail-closed and are not converted into
omission statuses. Status rows carry no entry path, entry index, row action,
or hit rectangle.

## TDD and visual chain

| Layer | RED | GREEN / closure |
|---|---|---|
| hidden-only status | `b385ca3a` | `8618451a` |
| non-UTF-8 omission | `0a341440` | `a59783b7` |
| iterator seam/refactor | — | `8e5b7f4d` |
| iterator failure count | `d2f082db` | `3b87c317` |
| mixed rows + status geometry | `5a952f11` | `948177ad` |
| Chromium oracle | `90fc7ff4` | `de136da5` |

VIS-13 was exported from the real Ratatui `TestBackend` buffer with the ASCII
icon profile. Baseline creation was scoped to `trail.spec.ts`; the update-less
trail + mutation run passed 8/8 and the complete Chromium suite passed 21/21.
The approved PNG visibly contains `visible.txt` and a separate
`hidden items omitted` row.

## Fresh gates

- Rust: 3,517/3,517 PASS, 2 skipped.
- Focused Trail/directory family: 43/43 PASS.
- Linux clippy: clean with `-D warnings`.
- Windows clippy: clean for `x86_64-pc-windows-msvc`.
- Playwright Chromium: 21/21 PASS.
- Formatting and diff checks: clean.

## Next dependency

FMR-2 is next: close the full sidebar primary-click to scheduled-consumer to
loaded-Trail test seam. The already confirmed old installed binary remains a
runtime confounder; if current source passes the end-to-end test, do not invent
new mouse semantics.
