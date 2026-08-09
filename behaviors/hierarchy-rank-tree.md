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

## Grouping and folding

| ID | Behavior | Breaks if lost | Verified by |
|---|---|---|---|
| TP-PROJ-GROUP-01 | The project takes a top-level row of its own, and everything it gathers — module headers, checkouts, chat drawers — steps in one level under it. | The four-level tree collapses back to flat peers: the umbrella stops reading as an umbrella. | `a_project_gathers_its_spaces_under_one_top_level_header`, `workspace_rows_shift_one_step_under_a_project` |
| TP-PROJ-GROUP-02 | Folding a project — by click or key — hides everything but the checkout the user is standing in, and the header click does nothing else. | Folding loses the user's place, or a header click switches workspaces as a side effect. | `a_collapsed_project_keeps_the_active_checkout_visible`, `clicking_project_header_toggles_project_only` |
| TP-PROJ-GROUP-03 | Folded, the project header answers for everything it hides with one aggregate state dot. | A folded project goes silent: a blocked agent inside it becomes invisible. | `collapsed_project_header_carries_an_aggregate_state_dot` |
| TP-PROJ-GROUP-04 | Spaces and workspaces no project claims render exactly as before, in workspace order, after the project blocks they interleave with. | Adding one project re-groups checkouts it never mentioned. | `spaces_outside_a_project_render_after_it_unchanged`, `entries_without_projects_have_no_project_header` |

## Persistence

| ID | Behavior | Breaks if lost | Verified by |
|---|---|---|---|
| TP-PROJ-PERS-01 | Project folds ride the session file like space folds; a session written before projects existed still loads. | Every restart forgets the folds, or an old session file stops loading at all. | `capture_contract_tracks_project_folds` |

## Mobile drawer

| ID | Behavior | Breaks if lost | Verified by |
|---|---|---|---|
| TP-MOB-98 | The phone drawer carries the project level: the umbrella row tops its spaces, every level under it steps in one, and tapping it folds the project through the row producer — position, not key. | The fourth level exists on the desktop only, and a phone reader sees the flat pre-project drawer again. | `the_drawer_tops_a_project_and_steps_its_levels_in`, `toggling_the_project_row_folds_the_project` |

## Rank promotion (`herdr space`)

| ID | Behavior | Breaks if lost | Verified by |
|---|---|---|---|
| TP-RANK-01 | `herdr space promote <branch>` writes a managed rule — repo, exact branch match, key, label, icon — under the do-not-hand-edit header. | The one-command promotion an agent relies on stops producing a rule the loader can pick up. | `upsert_writes_a_managed_rule_with_the_header` |
| TP-RANK-02 | `--as project` writes the rule and its umbrella together, the umbrella claiming exactly the promoted space. | Promotion past module rank lands the space with no project to sit under, or the umbrella grabs spaces it was never asked to. | `upsert_as_project_writes_the_umbrella_too` |
| TP-RANK-03 | Promoting the same target twice updates the managed entry in place. | Every re-promotion stacks a duplicate rule and the first stale one keeps winning first-match. | `upsert_is_idempotent_per_key` |
| TP-RANK-04 | Demote removes managed entries only — by key, branch, or slug — and a project that loses every member leaves with them; user config is never touched. | Demote either strands empty umbrellas in the sidebar or starts editing the user's own config.toml. | `remove_takes_the_rule_and_its_orphaned_project`, `remove_without_a_match_changes_nothing` |
| TP-RANK-05 | The managed overlay loads after the user's own config, so hand-written rules win first-match; a broken overlay is reported and skipped, never fatal. | Promotion output silently outranks hand-written rules, or one bad managed file takes the whole config down. | `managed_spaces_overlay_merges_after_user_rules`, `managed_spaces_overlay_tolerates_a_broken_file` |
| TP-RANK-06 | A branch row's context menu offers "Promote to module/project", and "Demote from module" only when a rule already claims the checkout. | The mouse road to promotion disappears, or every row dangles a demote that can do nothing. | `linked_worktree_menu_offers_promotion_and_conditional_demote`, `linked_worktree_context_menu_keeps_safe_close_and_explicit_remove` |
| TP-RANK-07 | The menu writes exactly the plan the CLI would write — same key, branch match, and project shape — then re-reads the rules so the sidebar regroups in place. | Menu promotion and CLI promotion drift apart, and one of them stops round-tripping with demote. | `promote_plan_from_a_workspace_row_matches_the_cli_shape` |

## Row icons

| ID | Behavior | Breaks if lost | Verified by |
|---|---|---|---|
| TP-ICON-01 | Every row kind wears its glyph: project headers their icon, checkout rows the branch glyph, chat rows the chat glyph — widths measured, never assumed. | Row kinds stop being tellable apart at a glance, which is the reason the icons exist. | `workspace_and_chat_rows_carry_their_kind_icons`, `project_header_row_draws_chevron_icon_and_name` |
| TP-ICON-02 | A rule's or project's own `icon` overrides the row-kind default; blank means "use the default", not "draw an empty glyph". | Per-module icons stop being configurable, or a blank icon renders as a hole in every row. | `split_rules_carry_optional_icons`, `spaces_project_name_falls_back_to_key`, `group_header_under_a_project_is_indented_and_shows_its_rule_icon` |
| TP-ICON-03 | A chat row carries the chat glyph and never a state dot, extending TP-WSCHAT-20's rule. | Chat rows start impersonating workspaces, and the attention column lies. | `workspace_and_chat_rows_carry_their_kind_icons` |
| TP-ICON-04 | Row-kind icon defaults live in `[spaces.icons]` and work without any config; a partial table keeps the unmentioned defaults. | Icons become all-or-nothing: setting one glyph silently blanks the other two. | `space_icons_defaults_and_partial_override` |
