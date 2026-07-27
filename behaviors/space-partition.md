# Config-driven space partitioning — registered behaviors

Fork feature. `[[spaces.split]]` rules let one repository present as several
sidebar spaces, so a repository holding independent product modules groups by
module instead of by repository.

Upstream keys a space by the repository's git directory and derives the group
header from the repository's main checkout. That is the right default and is
kept: an empty rule list reproduces upstream grouping byte for byte. What the
rules add is a second, config-owned answer to "which space is this checkout in".

Two ideas run through the family:

- **Resolution is presentation, not session truth.** The membership persisted in
  `session.json` stays the repository's, because that is what restore validates
  against (`restored_worktree_space_membership` re-derives the key from git and
  drops anything that disagrees). Rules are applied at render time instead, so a
  config reload re-groups every open workspace and no rule can corrupt a session
  file or survive as a stale key after the config changes.
- **A config space has no main checkout.** Upstream's group needs a non-linked
  member to render as the header row. A module space has no such checkout — all
  of its members are linked worktrees — so the header falls to the first member
  in workspace order and is titled after the rule, not after that member.

Format and rules: [`README.md`](README.md).

## Rule matching

| ID | Behavior | Breaks if lost | Verified by |
|---|---|---|---|
| TP-SPLIT-MATCH-01 | An empty rule list claims nothing; grouping is upstream's. | The feature stops being additive: every user without `[[spaces.split]]` gets whatever the resolver's fallback happens to do. | `empty_rule_list_never_claims_anything`, `unclaimed_checkout_keeps_the_repo_space` |
| TP-SPLIT-MATCH-02 | A rule only claims checkouts of its own `repo`. | One repository's rules silently re-group another's worktrees. | `rule_ignores_other_repositories` |
| TP-SPLIT-MATCH-03 | Patterns match the branch first, then the checkout directory name — so a detached checkout, which has no branch, is still claimable. | Detached worktrees fall out of their module and pile up in the repo space. | `rule_claims_by_branch`, `rule_falls_back_to_directory_name_for_detached_checkouts` |
| TP-SPLIT-MATCH-04 | First matching rule wins, in config order — not most-specific-wins. | Which module a checkout lands in stops being readable from the config file and starts depending on a specificity heuristic. | `first_matching_rule_wins` |
| TP-SPLIT-MATCH-05 | `*` is the only wildcard and matches any run including empty; a pattern without `*` must equal the whole value. | `main` starts matching `main-2`, so a narrow rule quietly swallows neighbouring branches. | `glob_without_wildcard_requires_exact_value`, `glob_matches_prefix_suffix_and_interior_wildcards`, `glob_star_matches_everything_including_empty` |

## Grouping

| ID | Behavior | Breaks if lost | Verified by |
|---|---|---|---|
| TP-SPLIT-GROUP-01 | A config space groups even though every member is a linked worktree. | The feature does nothing visible: upstream's "needs a non-linked member" filter rejects every module space, and the modules render as a flat list. | `config_space_groups_worktrees_that_have_no_main_checkout` |
| TP-SPLIT-GROUP-02 | Several rules over one repository produce several sibling groups. | The whole point — one repo, many spaces — collapses back to one space. | `config_space_splits_one_repository_into_several_groups` |
| TP-SPLIT-GROUP-03 | A config space with one member renders as a plain row, matching the repo space's own threshold. | A single-worktree module renders a header over nothing, so the sidebar grows a row that carries no information. | `config_space_with_a_single_member_stays_flat` |
| TP-SPLIT-GROUP-04 | Checkouts no rule claims keep the repository group, header included. | Adding one rule re-groups checkouts it never mentioned. | `unclaimed_worktrees_keep_the_repository_group` |

## Header and collapse

| ID | Behavior | Breaks if lost | Verified by |
|---|---|---|---|
| TP-SPLIT-HEAD-01 | The header row of a config space is titled after the rule's `label`; every other member keeps its own name. | Collapsed, the group reads as whichever checkout happens to lead it, so the sidebar no longer names the module. | `config_space_header_row_shows_the_rule_label` |
| TP-SPLIT-HEAD-02 | Collapse state is keyed by the rule key, and only the header row reports a group. | Collapsing a module toggles the repository's group, or every member claims to be a header and the collapse chevron appears on all of them. | `config_space_collapse_state_is_keyed_by_the_rule_key` |

## Config

| ID | Behavior | Breaks if lost | Verified by |
|---|---|---|---|
| TP-SPLIT-CONF-01 | An unusable rule (no repo, relative repo, no key, no pattern) is dropped and reported, never fatal. | One typo in `[[spaces.split]]` takes down the whole config load. | `space_split_entry_problem_rejects_unusable_entries`, `spaces_rules_drop_unusable_entries` |
| TP-SPLIT-CONF-02 | A duplicate key is reported, because two rules sharing a key silently merge two modules into one space. | The merge reads as a grouping bug and is debugged in the sidebar instead of in the config. | `spaces_diagnostics_report_duplicate_keys` |
| TP-SPLIT-CONF-03 | `[spaces]` is a known section: valid ones apply, an invalid one is isolated without disturbing other sections. | The section warns as unknown, or a malformed table takes the rest of the config with it. | `load_live_config_recognizes_spaces_section`, `load_live_config_isolates_invalid_spaces_section` |
