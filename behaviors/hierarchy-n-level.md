# N-level node tree — registered behaviors

The generalisation of the four-level tree: `[[spaces.node]]` containers hang
under each other by `parent`, split rules hang their buckets under nodes, and
yesterday's `[[spaces.project]]` is the depth-one case — a parentless node.
Assignment stays config-authored and render-time resolved; nothing about the
tree shape is ever persisted except the per-display folds.

Format and rules: [`README.md`](README.md).

## Model

| ID | Behavior | Breaks if lost | Verified by |
|---|---|---|---|
| TP-NODE-01 | An absent `[[spaces.node]]` table produces no nodes: the feature is opt-in and an untouched config renders exactly as before. | Every existing sidebar changes shape on update — the additive contract of the whole spaces family breaks. | `an_absent_node_table_produces_no_nodes` |
| TP-NODE-02 | A `[[spaces.project]]` doubles as a parentless node — and may carry a `parent` itself, so the sugar is complete. | Projects and nodes become two vocabularies; the tree walker forks and the two halves drift. | `a_project_entry_may_carry_a_parent_too` |
| TP-NODE-03 | Tree-shape problems (cycles, unknown parents, depth past eight) degrade to a flatter tree plus a diagnostic — never a failed load, never a hung walker. | One typo in a parent chain takes the whole sidebar down, or worse, spins the emitter forever. | `a_cycle_is_reported_and_its_nodes_go_top_level` |

## Emission

| ID | Behavior | Breaks if lost | Verified by |
|---|---|---|---|
| TP-NODE-04 | The chain reads top-down and order stays order-by-first-member, recursively: every ancestor header appears before its children, where the subtree's first member sits in workspace order. | Headers appear after or away from their content and the drawer's spatial memory — built on workspace order — dies. | `nested_nodes_emit_parent_before_child_before_the_bucket` |
| TP-NODE-05 | A parented bucket with one member elides its header: the checkout hangs straight under the node, indented. | "Move this branch under X" draws a one-member bucket ceremony instead of the branch simply sitting under X. | `a_single_member_parented_bucket_hangs_its_member_under_the_node` |

## Folds

| ID | Behavior | Breaks if lost | Verified by |
|---|---|---|---|
| TP-NODE-06 | Folding a node is a statement about one screen: another living display's tree does not move. | The session-wide project-fold complaint returns, one level up — the exact HP18 class-5 failure this migration closes. | `one_displays_node_fold_never_moves_anothers_tree` |
| TP-NODE-07 | A fold recorded by the retired session-wide project set still reads as folded; unfolding withdraws the legacy record and every new fold lives per-display. | Old sessions lose their folds on update, or worse, a legacy record re-folds every screen forever. | `a_folded_legacy_project_key_still_reads_folded_and_unfolds_forward` |

## Moves (the management surface over the tree)

| ID | Behavior | Breaks if lost | Verified by |
|---|---|---|---|
| TP-RANK-08 | A move writes `parent` onto the managed rule and re-hangs in place: same key, new parent, one rule; a plan without a parent (and a move back to top level) keeps the field out of the file entirely. | Moving stacks duplicate rules or strands stale parents — the managed overlay stops being a faithful mirror of the last decision, and plain promotes stop being byte-compatible. | `upsert_writes_the_parent_the_plan_carries`, `moving_again_rehangs_the_same_rule` |
| TP-RANK-09 | The three move operations resolve against the validated forest — under is the target, beside is the target's parent, above is the grandparent; the top of the tree answers top level, and an unknown target is refused, never silently flattened. | A typo'd CLI key quietly drops a branch to top level, or "beside"/"above" land on the wrong generation — the user's mental model of the three verbs breaks. | `move_ops_resolve_under_beside_and_above`, `move_ops_at_the_top_answer_top_level`, `an_unknown_move_target_is_refused` |
| TP-RANK-10 | "Move under a new group" writes the `[[spaces.node]]` entry and the membership that points at it in one document update, and re-writing the same group upserts by key. | A crash between two writes leaves a parent naming a group that does not exist — a ghost-parent diagnostic on every load for a group the user asked for. | `a_new_group_lands_with_its_member_in_one_write` |
| TP-RANK-11 | `herdr space move <branch>` takes exactly one destination — `--under`/`--beside`/`--above <node-key>`, `--top`, or `--new-group <name>` — refuses ambiguous or incomplete invocations, and re-hangs without re-styling: the label and icon a promote once wrote survive the move. | The CLI contract agents script against (G6 guide) drifts, or every move silently strips the label and icon the user chose at promote time. | `move_args_parse_the_five_destinations`, `move_args_refuse_ambiguous_or_incomplete_invocations`, `a_move_keeps_the_promoted_label_and_icon` |
| TP-RANK-12 | The menu's move plan is the promote plan carrying a parent (and possibly the group it creates) — same key, same branch, never a project rank; a row without a branch or membership moves nothing. | The mouse road and the CLI road write different rules for the same gesture, and the managed overlay forks into two dialects. | `move_plan_carries_the_parent_and_the_new_group` |
| TP-RANK-13 | A linked branch row offers "Move...": a verb submenu (verbs hidden when no node exists to point at), then a target picker showing display names and resolving by index; "Under a new group..." collects a name through the rename input, and an escaped input disarms the pending move so a plain rename can never create a group. | The tree can only be re-shaped by hand-editing config — the mouse-first management surface (G5's whole point) is gone, or worse, a stale pending move turns a rename into a surprise group. | `move_walks_the_submenu_then_the_picker`, `the_move_menu_hides_the_verbs_without_targets`, `the_target_picker_lists_the_forest_by_name`, `a_new_group_pick_opens_the_name_input`, `an_escaped_group_name_never_leaks_into_the_next_rename` |
