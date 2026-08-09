# Project umbrellas, rank promotion, row icons — registered behaviors

Fork feature, second story of the space-partition family. `[[spaces.split]]`
answers "which module is this checkout in"; `[[spaces.project]]` answers the
question above it — "which product do these modules and repositories belong
to" — and puts a fourth, top level over the existing three-level tree
(group header → workspace → chat drawer).

The carrying decisions are inherited from [space-partition.md](space-partition.md)
unchanged: resolution happens at render time and is never persisted, an empty
config reproduces today's sidebar byte for byte, and unusable config is dropped
loudly instead of failing the load.

Format and rules: [`README.md`](README.md).

## Project matching

| ID | Behavior | Breaks if lost | Verified by |
|---|---|---|---|
| TP-PROJ-MATCH-01 | An empty project list claims nothing; every space stays top-level and the sidebar renders as it always has. | The feature stops being additive: configs that never asked for projects get regrouped. | `empty_project_list_claims_nothing` |
| TP-PROJ-MATCH-02 | A project claims spaces two ways: by explicit space key, and by the repository root its members live in. | One of the two membership paths silently dies — promotion (space keys) stops landing branches under a project, or whole-repo grouping stops gathering a product's repositories. | `project_claims_by_space_key`, `project_claims_by_repo_root` |
| TP-PROJ-MATCH-03 | The first matching project wins, in config order — not most-specific-wins. | Which project a space lands in stops being readable from the config file and starts depending on a specificity heuristic. | `first_matching_project_wins` |

## Project config

| ID | Behavior | Breaks if lost | Verified by |
|---|---|---|---|
| TP-PROJ-CONF-01 | An unusable project entry (no key, relative repo, no usable member) is dropped and reported, never fatal. | One typo in `[[spaces.project]]` takes down the whole config load. | `space_project_entry_problem_rejects_unusable_entries`, `spaces_projects_drop_unusable_entries` |
| TP-PROJ-CONF-02 | A duplicate project key is reported, because two projects sharing a key share one collapse state. | The shared fold reads as a folding bug and is debugged in the sidebar instead of in the config. | `projects_diagnostics_report_duplicate_project_keys` |

## Row icons (config half)

The render half of these behaviors — where the glyphs are actually drawn —
lands with the tree render work and extends these rows with its test names.

| ID | Behavior | Breaks if lost | Verified by |
|---|---|---|---|
| TP-ICON-02 | A rule's or project's own `icon` overrides the row-kind default; blank means "use the default", not "draw an empty glyph". | Per-module icons stop being configurable, or a blank icon renders as a hole in every row. | `split_rules_carry_optional_icons`, `spaces_project_name_falls_back_to_key` |
| TP-ICON-04 | Row-kind icon defaults live in `[spaces.icons]` and work without any config; a partial table keeps the unmentioned defaults. | Icons become all-or-nothing: setting one glyph silently blanks the other two. | `space_icons_defaults_and_partial_override` |
