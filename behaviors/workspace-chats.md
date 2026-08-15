# Workspace chats — which agent chat ran in which workspace

The sidebar answers "which chat did I work with on this branch". That question
cannot be answered from the agent's own on-disk store, which is keyed by the
directory the agent was launched in: a branch checkout very often is not that
directory. **Measured 2026-07-30 against a live session:** 9 of 14 workspaces
resolved to no chats that way, and 4 of the 9 sessions herdr was actively wired
to lived under a different directory than their workspace.

The live wiring (`agent_session` on a pane) knows the truth, but only while the
pane exists — closing a tab erases it, and the question is asked in the past
tense. So the association is recorded as it happens, in an append-only ledger at
`~/.config/herdr/workspace-chats.json`.

Deliberately **outside** the session snapshot: that snapshot describes the LIVE
layout and the restore contract depends on its shape, while this ledger is
history that outlives the workspaces it describes.

Design and the measurements behind it:
`.local/prd/2026-07-30-spaces-branch-chat-drawer-PRD.md`.

## Ledger core (`src/persist/workspace_chats.rs`)

| ID | Behavior | What breaks if it is lost | Verified by |
|---|---|---|---|
| TP-WSCHAT-01 | An observation becomes a record keyed by workspace | The drawer stays permanently empty — the premise of the feature | `a_first_sighting_becomes_a_record` |
| TP-WSCHAT-02 | A repeat sighting upserts: no duplicate row, `last_seen` advances, `first_seen` is never rewritten; an identical repeat reports no change | Hook reports repeat constantly — without upsert the ledger grows one row per report and "when did I start here" is overwritten; without the no-change signal every report schedules a disk write | `a_repeat_sighting_updates_the_time_without_duplicating_or_rewriting_the_first` · `an_identical_repeat_reports_no_change` |
| TP-WSCHAT-03 | The most recent sighting leads the list | The drawer would have to sort at render time — state work in the render path | `the_most_recent_sighting_leads_the_list` |
| TP-WSCHAT-04 | Each workspace keeps its own chats | Per-branch attribution collapses into one shared list, answering the wrong question | `each_workspace_keeps_its_own_chats` |
| TP-WSCHAT-05 | History is capped per workspace, dropping the oldest | An unbounded ledger turns a convenience into a disk-space bug | `the_ledger_caps_a_workspaces_history_and_drops_the_oldest` |
| TP-WSCHAT-06 | An empty key or session id is refused | A chat gets attributed to nowhere and shows under the wrong workspace | `an_unresolvable_observation_is_refused` |
| TP-WSCHAT-07 | The ledger round-trips through disk | It stops outliving the live wiring — the one thing it exists for | `the_ledger_round_trips_through_disk` |
| TP-WSCHAT-08 | A missing, corrupt or unknown-version file degrades to empty without panicking | A hand-edited or truncated file stops the server from starting; a guessed-at future schema silently misreads history | `a_corrupt_or_missing_ledger_degrades_to_empty_without_panicking` |
| TP-WSCHAT-09 | Saving is atomic and leaves no temp file | A crash mid-write truncates the file, and the next start reads it as corrupt and drops the whole history | `saving_leaves_no_temp_file_and_replaces_the_previous_ledger` |
| TP-WSCHAT-10 | The key is canonical, falling back to the raw path when the directory is gone | Two spellings of one directory split a branch's history; a removed directory would silently start over | `the_ledger_key_is_canonical_and_falls_back_for_missing_paths` |

## Observation (snapshot → associations)

| ID | Behavior | What breaks if it is lost | Verified by |
|---|---|---|---|
| TP-WSCHAT-11 | Observations are derived from the session SNAPSHOT (which already resolved hook-authority-over-persisted precedence), one association per workspace+session, ignoring panes with no session | Re-deriving the precedence here lets the ledger and the session file disagree about the same pane; a split showing one agent twice would list the chat twice | `the_observer_collects_one_association_per_workspace_and_session` · `the_observer_deduplicates_the_same_chat_across_panes` · `the_observer_ignores_panes_without_a_session` |
| TP-WSCHAT-12 | A still-fresh sighting is not re-recorded; a stale one is, without moving `first_seen` | The observer runs on every debounced save, so a live chat would advance `last_seen` on every pass and rewrite the ledger continuously — a permanent write loop | `a_fresh_sighting_is_not_re_recorded_but_a_stale_one_is` |

## Wiring (`src/app/session.rs`)

| ID | Behavior | What breaks if it is lost | Verified by |
|---|---|---|---|
| TP-WSCHAT-13 | The association is recorded through the real session-save funnel; a workspace with no agent records nothing | A ledger that only works in a unit test leaves the drawer empty in production — the exact failure this feature exists to prevent | `a_session_save_folds_the_live_wiring_into_the_ledger` · `a_session_save_without_any_agent_records_nothing` |
| TP-WSCHAT-14 | A `--no-session` run tracks chats in memory but writes nothing | Every unit test that captures a save writes into the real config directory (observed during development: a test run created `~/.config/herdr-dev/workspace-chats.json`), and `--no-session` stops meaning "leaves nothing on disk" | `a_no_session_run_tracks_chats_in_memory_but_writes_nothing` |

## Spaces row model (`src/ui/sidebar.rs`)

| ID | Behavior | What breaks if it is lost | Verified by |
|---|---|---|---|
| TP-WSCHAT-15 | A workspace's chat drawer is closed until it is opened — the state records which drawers are OPEN, the inverse of the Projects tab | Spaces routinely holds a dozen-plus workspaces; opening every drawer at once buries the workspace list the tab exists for | `a_workspaces_chat_drawer_is_closed_until_it_is_opened` |
| TP-WSCHAT-16 | An open drawer with no chats shows a placeholder; a busy one lists a capped number and folds the rest into an inert "older" row | An empty gap reads as a broken drawer, and an uncapped drawer pushes every other workspace off the screen | `an_open_drawer_with_no_chats_shows_a_placeholder` · `a_busy_drawer_lists_a_capped_number_of_chats_and_an_older_row` |
| TP-WSCHAT-17 | Scroll metrics and layout derive from the same row list, and chat rows are laid out in their own vector rather than the workspace-indexed card vector | Counting rows one way and drawing them another scrolls past rows that were never drawn; folding chat rows into the card vector makes a chat click resolve as a workspace switch (the trap the tab strip documents as TP-FTAB-ENTRY-05) | `the_scroll_metrics_and_the_layout_agree_on_the_drawer_rows` · `chat_rows_stay_out_of_the_workspace_indexed_card_vector` |
| TP-WSCHAT-19 | The drawer toggle LEADS the workspace row (immediately after the worktree-group chevron when that row has one), offered only where there is history; it opens AND closes, and the rest of the row still selects the workspace | A disclosure arrow only reads as "this row opens" when it precedes the name — at the far edge it reads as an unrelated control; the two chevrons must stay adjacent on the left rather than sharing a cell, which would make one unreachable; an arrow on every row that only ever reveals "(no chats)" is noise; a toggle that swallows the row makes the workspace unclickable | `only_a_workspace_with_history_offers_a_drawer_toggle` · `the_drawer_toggle_opens_and_closes_without_stealing_the_workspace_click` |
| TP-WSCHAT-20 | An open drawer draws its chats under the workspace, indented past the branch children, and a chat row never takes the accent background | State that is right but draws nothing is this family's known failure (the file-manager previews); the accent background marks the active workspace and the active agent card, so a chat wearing it would read as one of those | `an_open_drawer_draws_its_chats_below_the_workspace` |
| TP-WSCHAT-21 | Drawer rows take their title and age from the agent's own store. A chat filed under a different directory is looked for in the other open workspaces' directories before the row falls back to a short id. | A session id is not an answer to "which chat did I work with". The fallback still exists — a chat filed somewhere herdr has no workspace for cannot be titled — but it is now the last resort rather than the first answer for every chat that moved. Superseded the original "stays untitled by design" on 2026-07-30, when the measurement showed one worktree with 1 chat in its own directory and 4 filed elsewhere (TP-DRAW-06). | `drawer_rows_take_their_title_from_the_agents_own_store` |
| TP-WSCHAT-22 | The drawer uses the Projects tab's wired-state vocabulary: `▸` this chat is the focused tab, `●` open in another tab, blank not open — and the disclosure arrow LEADS the row | Two surfaces inventing different alphabets for one fact makes the sidebar unreadable; a trailing arrow reads as an unrelated control at the far edge | `an_open_drawer_draws_its_chats_below_the_workspace` · `only_a_workspace_with_history_offers_a_drawer_toggle` |
| TP-WSCHAT-23 | Every workspace row carries a trailing "+" that starts a chat rooted at that workspace, mirroring the Projects tab's per-project button | Without it the drawer can only ever show history: there is no way to begin the first chat on a branch from the surface that lists them | `clicking_a_drawer_row_asks_for_that_chat_and_plus_starts_a_new_one` |
| TP-WSCHAT-24 | Clicking a drawer row asks for that chat; a chat already wired to a live tab is focused instead of resumed, and the trailing "+" is not swallowed by the row it sits on | A row that cannot be clicked is decoration; resuming a live chat twice spawns a second process against one transcript — the spam-click guard the Projects tab already learned | `clicking_a_drawer_row_asks_for_that_chat_and_plus_starts_a_new_one` |
| TP-WSCHAT-25 | The "+" on a repository root opens a choice — chat agents plus worktree actions — while a linked worktree starts a chat directly; picking the worktree entry reaches the worktree request and is never persisted as a chat agent | "Start something new here" genuinely means two things at a repo root and one inside a worktree, so asking there and not here is the difference between a useful question and a click nobody needed; and the worktree rows must be matched before the agent catch-all, or choosing one silently sets "New worktree" as the default agent | `clicking_a_drawer_row_asks_for_that_chat_and_plus_starts_a_new_one` |
| TP-WSCHAT-18 | The mobile drawer *does* see chat rows. **Reversed 2026-08-01.** | The old rule protected the flat switcher and its two-rows-per-workspace arithmetic. The drawer derives positions from a single row list instead, so the hazard is gone — and the exclusion was hiding every remembered chat from anyone working on a phone (TP-MOB-60) | `the_mobile_drawer_sees_chat_rows` |

## Completeness — where the drawer's rows come from

The ledger alone was not enough, and the measurement said so plainly. On
2026-07-30 the fourteen open workspaces held **1510** transcripts between them
in the agent's own store; the ledger knew **14**, because it only began
recording the day it was written. A ledger-only drawer showed almost nothing —
one workspace with 1225 transcripts showed two rows.

The reverse is also true and is why the ledger cannot simply be replaced: a
chat that started in one directory and moved is filed under the directory it
started in. One worktree held 1 transcript in its own directory and 4 in the
ledger. Neither source answers the question alone, so the drawer is their union.

| ID | Behavior | Breaks if lost | Verified by |
|---|---|---|---|
| TP-DRAW-01 | The drawer lists the chats the agent's own store holds for a workspace's directory, whether or not the ledger ever witnessed them. | The drawer shows only what herdr happened to be running for, which was 14 chats out of 1510. | `the_drawer_lists_chats_the_ledger_never_saw` |
| TP-DRAW-02 | The store and the ledger are unioned, not substituted. | A chat that started elsewhere and moved in disappears — and that association is the part nothing else records. | `the_drawer_unions_the_store_and_the_ledger_without_duplicating` |
| TP-DRAW-03 | One row per chat, with the store authoritative for title and mtime. | Every chat both sources know about is listed twice, and the drawer's cap spends its rows on duplicates. | `the_drawer_unions_the_store_and_the_ledger_without_duplicating` |
| TP-DRAW-04 | Rows are ordered by last activity, newest first. | The drawer cannot answer "which chat was I just in", which is the question it exists for. | `drawer_rows_are_ordered_newest_first` |
| TP-DRAW-05 | Every row can be dated: the transcript's mtime when known, the ledger's last sighting otherwise. | Only some rows carry an age, which reads as broken rather than partial. | `a_row_without_a_located_transcript_still_reports_its_last_activity` |
| TP-DRAW-06 | A chat filed under another open workspace's directory is looked for there before the row falls back to a bare id. | Chats that moved show as `9433af4a · claude` forever, even though the title is on disk one directory away. | `a_chat_filed_under_another_workspace_is_still_titled` |
| TP-DRAW-07 | The cross-directory search runs only for rows that are still untitled. | A directory holding 1225 transcripts is re-read on every refresh to answer a question already answered. | `a_chat_filed_under_another_workspace_is_still_titled` |
| TP-DRAW-08 | A known conventional-commit prefix is dropped from a branch row's label. | Every row spends five columns saying `feat/` and the part that tells the branches apart is what gets truncated. | `a_known_branch_prefix_is_dropped_and_a_chosen_namespace_is_kept` |
| TP-DRAW-09 | Only a closed set of prefixes is dropped, and only when something is left. | A namespace the person chose (`codex/…`) is deleted as if it were noise, or a branch called exactly `feat/` renders as a blank row. | `a_known_branch_prefix_is_dropped_and_a_chosen_namespace_is_kept` |
| TP-DRAW-10 | The drawer keeps five rows at a glance; a display that asks sees every chat it holds, and the deeper read is fetched with a higher cap (60) so "older" is not a promise the parse cannot keep | The row offers chats the fetch never loaded, or the sidebar turns into an archive nobody asked for | `a_deep_drawer_shows_five_until_it_is_opened_all_the_way` |
| TP-DRAW-11 | The "older chats" row is drawn (it was laid out but never painted — the desktop drawer ended in a blank line), says which way it goes ("… N older" / "… fewer"), owns a rect in its own vector, and both opens and folds the drawer | A reader clicks an invisible line, or the row opens a drawer with no way back, or the press resolves as the chat drawn above it and resumes a session nobody asked for | `the_older_chats_row_is_painted_and_says_which_way_it_goes` · `a_press_on_the_older_chats_row_opens_the_drawer_and_folds_it_back` |
| TP-DRAW-12 | How deep a drawer is opened is per display, keyed through the same ledger key its openness uses | One screen digging through old chats stretches the drawer on another — the multi-display rule this fork is built on | `opening_a_drawer_all_the_way_stays_on_this_display` |
| TP-DRAW-13 | A `--no-session` start reads neither store: the ledger *and* the transcript directory are both skipped | The clean-start promise is kept by half. In the product, `herdr --no-session` promises a clean start and loads the machine's history anyway. In the tests it is worse: every fixture built on `App::new(.., true, ..)` — `test_app` and `test_headless_server`, several hundred tests — reads the live `~/.claude/projects`, and since a chat row orders by transcript mtime, the measurement moves whenever the agent running it writes to its own transcript. A headless render test failed exactly that way under load and refused a landing | `a_sessionless_start_reads_neither_the_ledger_nor_the_transcript_store` |

### Cost

Reading the store costs one `read_dir` plus metadata per workspace. Transcripts
are ranked by mtime from that metadata alone and only the newest few are opened,
through a parse cache keyed by (mtime, size) — so a refresh re-reads a file only
when it actually changed. The merge rides the debounced session save, never a
frame.
