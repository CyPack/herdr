# Spaces as a three-level tree — registered behaviors

Fork feature. The Spaces tab shows a repository, the checkouts under it, and the
remembered chats under each checkout — three levels, one meaning per control.

It exists because of a specific, measured failure. The Spaces tab had grown two
independent disclosures: "show this repository's other checkouts" and "show this
checkout's remembered chats" (`workspace-chats.md`). Both were drawn as the same
arrow, in the same leading gutter, one column apart, on the same row:

```
▾▾· herdr               +      <- two arrows, two different controls
   master
      spaces cekmece tas…      <- chats, floating, unattached
▾  · fix-clipboard      +      <- a child's arrow in the parent's column
```

Three things were wrong and none of them were fixable by styling:

- **One row, two disclosures.** No reader can tell which arrow folds the
  repository and which opens the chats when they are adjacent and identical.
- **`▸` meant three things** — group folded, drawer folded, and *this chat is
  the focused tab*. A glyph with three meanings carries none.
- **A grouped child drew its arrow in column 0**, the repository's column, so a
  checkout claimed to be a group.

The fix is structural rather than cosmetic: give the repository a row of its
own. Once the two disclosures live on different rows, they cannot be confused,
and the third meaning of `▸` can be handed back to the accent, which is how
everything else in this sidebar says "this is the one you are on".

Two ideas run through the family:

- **One gutter, one meaning.** A leading arrow means "open what is under me" and
  nothing else, at every depth. Depth is spent on indentation; state is spent on
  the dot; actions are spent on the trailing edge. No column does two jobs.
- **A header is not a workspace.** `GroupHeader` carries no `ws_idx` and is laid
  out into its own area vector. Nothing that resolves a workspace can reach it,
  which is what keeps a header press from folding, selecting, or switching to
  whichever workspace happens to share its row.

## Shape of the list

| ID | Behavior | Breaks if lost | Verified by |
|---|---|---|---|
| TP-TREE-01 | A repository with two or more checkouts gets a header row of its own, and that row owns the group's arrow. | Both disclosures return to one row one column apart, and no reader can tell which arrow does what. | `a_repository_with_two_checkouts_gets_a_header_row_of_its_own` |
| TP-TREE-02 | A repository with a single checkout gets no header. | Every workspace pays a header row, doubling the vertical cost of a dozen-plus list for a group that does not exist. | `a_lone_checkout_gets_no_header_row` |
| TP-TREE-03 | A folded group keeps its header and the checkout in use, and hides the rest. | Folding hides where you are standing, so the sidebar stops answering "which checkout am I in". | `collapsed_group_hides_inactive_children_but_keeps_active_visible` |
| TP-TREE-04 | Every checkout is a child of the header, the main one included. | The main checkout stays at the header's depth and its arrow lands back in the repository's column — the original collision. | `a_repository_with_two_checkouts_gets_a_header_row_of_its_own` · `parent_workspace_row_stays_clickable_when_grouped` |

## Layout

| ID | Behavior | Breaks if lost | Verified by |
|---|---|---|---|
| TP-TREE-05 | Header rows are laid out into their own vector, never into the workspace-indexed one. | A header press resolves as whichever workspace shares its position — the trap already paid for by the tab strip (TP-FTAB-ENTRY-05) and the chat drawer (TP-WSCHAT-17). | `parent_workspace_row_stays_clickable_when_grouped` |
| TP-TREE-06 | Every row kind is measured by one function, and the group gap falls only where a new top-level unit begins. | A drawn-but-uncounted row makes the list scroll past itself; a gap in the wrong place draws a separator inside a group instead of around it. | `the_scroll_metrics_count_the_header_row` · `space_row_gap_preserves_compact_worktree_children` |
| TP-TREE-07 | A header is laid out above the checkouts it introduces. | The header names a group that appears somewhere else in the list. | `parent_workspace_row_stays_clickable_when_grouped` |

## Drawing

| ID | Behavior | Breaks if lost | Verified by |
|---|---|---|---|
| TP-TREE-08 | No row ever carries two disclosure arrows side by side, at any width. | The original complaint returns. This is the machine-checked form of it. | `no_row_ever_carries_two_disclosure_arrows_side_by_side` |
| TP-TREE-09 | A drawer's rows hang off their checkout's arrow column on a vertical rule. | Chats float under the list unattached, reading as siblings of the checkouts rather than as their contents. | `the_tree_puts_the_repository_the_checkouts_and_the_chats_on_their_own_depths` |
| TP-TREE-10 | The repository owns column 0; a checkout's arrow sits one indent step in; the disclosure column is reserved on every row whether or not that row has an arrow. | A checkout claims to be a repository, and sibling names step in and out as drawers appear and disappear. | `the_tree_puts_the_repository_the_checkouts_and_the_chats_on_their_own_depths` |
| TP-TREE-11 | The selection accent lives on exactly one focus carrier: the active workspace row wears it with contrast text, except while the drawer shows the chat the active tab resumes — the accent then belongs to that chat row and the card steps back to a quiet active tone. | "Which workspace am I in" has two possible answers, or none — and with the carrier rule lost, the card and the chat wear the accent at once and the answer doubles again. | `the_active_workspace_row_wears_the_accent_and_a_chat_row_never_does` · `default_space_workspace_style_tracks_active_state` |
| TP-FOCUS-01 | With the active tab's chat visible in the open drawer, the accent descends to that chat row (contrast text, bold) and the workspace card gives it up. | The highlight answers at the wrong depth: the branch lights up while the chat being worked in stays plain — the exact miss the user reported. | `the_accent_descends_to_the_visible_active_chat_row` |
| TP-FOCUS-02 | With the drawer shut, the workspace card keeps the accent even though its chat is the real focus object. | The accent follows an invisible row and the sidebar shows no selection at all. | `the_card_keeps_the_accent_while_its_chat_is_hidden` |
| TP-FOCUS-03 | (mode-aware since chat_drawer_mode) In focused mode, arriving at a workspace opens its chat drawer; a fold made while standing in it holds until the user leaves and returns, and an empty history is never revealed onto its placeholder. all-active derives instead of revealing and manual never moves a drawer — TP-DRAWER-05/06 carry those halves. | The chat the accent descends to hides behind a fold the reader has to know to open — the "hidden drawer" the user reported. | `activating_a_workspace_reveals_its_chat_drawer`, `a_fold_made_inside_the_workspace_holds_until_reactivation`, `an_empty_history_is_not_revealed_on_activation` |
| TP-TREE-12 | Every checkout offers a "+" that starts a chat there, and a checkout with history says how many chats it remembers. The "+" is never bound to hover. | Starting a chat on a branch — the point of the row — needs a detour, or a pointer move repaints the sidebar and breaks the loop-saturation guard (TP-REPAINT-2B). | `every_checkout_offers_a_plus_and_reports_how_many_chats_it_remembers` |
| TP-TREE-13 | The tree lays out without overflow down to the minimum configurable width. | Depth is charged in columns, so the narrowest sidebar is exactly where a prefix wider than the row wraps or panics. | `the_narrowest_sidebar_still_draws_the_tree` |

## Pressing

| ID | Behavior | Breaks if lost | Verified by |
|---|---|---|---|
| TP-TREE-14 | Pressing a repository header folds or unfolds its group and does nothing else — it never switches workspace. | A header behaves as a workspace, so folding a group moves you somewhere you did not ask to go. | `a_repository_press_folds_its_group_and_a_checkout_arrow_opens_its_drawer` · `clicking_worktree_parent_chevron_toggles_group_only` |
| TP-TREE-15 | Pressing a checkout's own arrow opens that checkout's drawer, and leaves the group alone. | The two disclosures act on each other, which is the collision in behavioural form. | `a_repository_press_folds_its_group_and_a_checkout_arrow_opens_its_drawer` |

## Mobile

| ID | Behavior | Breaks if lost | Verified by |
|---|---|---|---|
| TP-TREE-18 | The mobile drawer *does* see the header row. **Reversed 2026-08-01.** | The old rule protected the flat switcher, which derived a workspace position arithmetically, so a header shifted every position after it. The drawer that replaced it derives every position from one row list that render, hit-testing, height and the cursor all read, so an extra row kind cannot desynchronise them — and without the header a reader on a phone cannot tell which repository a worktree belongs to (TP-MOB-60). | `the_mobile_drawer_sees_the_group_header` |

## Cost

The sidebar renders server-side and reaches a remote session as the client's
ANSI diff, so wire cost tracks the number of changed cells and CPU cost tracks
the number of renders. Nothing here adds a periodic source of either: the tree
is static, differentiated by glyph, depth and colour, and every change to it is
caused by a keypress or a click. The one thing deliberately not done is binding
any visual state to hover, which would make a pointer move a repaint.

Sibling: `workspace-chats.md` (what the drawer remembers) ·
`space-partition.md` (which repository a checkout groups under) ·
`shared-surfaces.md` (per-display ownership).
