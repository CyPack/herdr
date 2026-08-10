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
