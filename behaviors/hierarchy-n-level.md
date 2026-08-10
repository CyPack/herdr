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

## Chat re-homes (a chat belongs to the drawer the user says it does)

| ID | Behavior | Breaks if lost | Verified by |
|---|---|---|---|
| TP-CHAT-MOVE-01 | A re-home wins over every source — the ledger projection AND the agent's own cwd-keyed store merge — applied as the last step of the row sync: the chat appears in exactly one drawer, in recency order, never duplicated, and a move to an unobserved drawer still lands. | The agent-store merge leaks a moved chat back into its source drawer on the next refresh, and the user's decision silently un-does itself. | `a_move_relocates_the_chat_out_of_every_source_drawer`, `a_move_to_an_unobserved_drawer_creates_it`, `a_move_never_duplicates_and_tolerates_the_unknown` |
| TP-CHAT-MOVE-02 | The decision survives a restart, and yesterday's ledger file (no `moves` field) still loads — additive on schema version 1, so an older binary keeps reading the history. | A restart forgets every re-home, or an update strands users on an unreadable ledger. | `moves_round_trip_and_an_old_file_loads_without_them` |
| TP-CHAT-MOVE-03 | `set_move` and `clear_move` answer honestly: an identical decision, an unknown withdrawal, or empty identity never schedules a disk write. | Every menu click rewrites the ledger file, or a no-op decision looks like a change. | `set_and_clear_move_report_change_honestly` |
| TP-CHAT-MOVE-04 | A chat row's right-click owns its own menu (checked before the workspace cards): "Move to branch..." opens a picker of the other open drawers by ledger key, "Move back" shows only while a re-home is in force, and the selection parks a request the App loop writes — the session id is resolved at menu-open so a list refresh can never re-target the move. | Right-clicking a chat falls through to the branch menu, moves are impossible from the surface, or an index shift under an open menu moves the wrong chat. | `the_chat_menu_offers_move_and_conditionally_back`, `chat_move_walks_the_picker_and_requests_the_move`, `move_back_requests_a_clear` |

## Workspace identity (the row's meaning IS the data's key)

| ID | Behavior | Breaks if lost | Verified by |
|---|---|---|---|
| TP-WSID-01 | `Workspace::effective_cwd` answers with the checkout when one is carried and the birthplace otherwise — the single directory every data operation reads. | Visual identity and data identity split again: rows say "branch X" while spawns and attribution use the birthplace. | `effective_cwd_prefers_the_checkout_over_the_birthplace` |
| TP-WSID-02 | A "+" chat request carries the checkout, never the directory the workspace was born in. | Agents spawn in $HOME from a branch row — the feature's whole promise inverted. | `a_new_chat_request_carries_the_checkout_not_the_birthplace` |
| TP-WSID-03 | The drawer reads AND keys its openness by the effective directory, so two rows sharing a birthplace never share a chat list or an open state. | One chat list appears under several branch rows (the MFA complaint), and opening one drawer opens a stranger's. | `the_drawer_reads_by_the_checkout_not_the_birthplace` |
| TP-WSID-04 | The ledger observer attributes sightings to the checkout when the workspace carries one. | New history keeps landing under shared birthplaces and the bleed regrows from fresh data. | `the_observer_attributes_by_the_checkout_when_one_is_carried` |
| TP-WSID-05 | A workspace without a checkout keeps the birthplace as its effective directory: every pre-existing drawer, spawn and attribution behavior is unchanged. | The identity fix silently re-keys plain workspaces and their history vanishes on update. | `clicking_a_drawer_row_asks_for_that_chat_and_plus_starts_a_new_one` |
