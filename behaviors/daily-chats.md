# Daily chats — the directory no checkout claims

A chat started outside every repository files itself under `$HOME`. Since
TP-WSID-01 made `effective_cwd` prefer the checkout over the birthplace, no
workspace holds that directory any more — so nothing on the sidebar's side
ever asks to read it. Measured 2026-08-12 on the live machine: **1266
transcripts** under `$HOME`, **0** workspaces holding it, and the chat ledger
carrying exactly one key (a checkout's). Those conversations were reachable
from nowhere.

This family owns the half of that fix whose loss would be silent: the read
itself. A merge that quietly re-tied the read to the workspace list would not
fail a test unless one names this — the drawer would simply go back to showing
nothing, and it would look like there had never been anything there.

| ID | Behavior | Breaks if lost | Verified by |
|---|---|---|---|
| TP-DAILY-01 | The daily directory (`AppState::daily_chat_cwd`, `$HOME` in production) is read on every chat-row merge **whether or not a workspace lives there**, under the same glance/open budget as any other drawer (12 rows closed, 60 opened all the way), unioned with the ledger one-row-per-chat, and a directory that is missing — or a client with no home at all — costs nothing | The read follows the workspace list again and the section is born dead: `$HOME` has no workspace and never will, so "read what the workspaces ask for" finds none of these chats. The failure is invisible — an empty section reads as "no chats yet", not as a bug | `daily_chats_are_read_even_when_no_workspace_lives_in_that_directory`, `a_daily_directory_that_is_not_there_costs_nothing`, `an_open_daily_drawer_reads_past_the_glance_limit`, `a_daily_chat_the_ledger_also_saw_stays_one_row` |

| TP-DAILY-02 | The section is emitted above the whole tree — header first, then its chats — and an empty section is not drawn at all | These chats belong to no checkout, so a row that is not first is a row that is nowhere; and a header promising content that is not there reads as a broken surface on every machine that never started a chat outside a checkout | `the_daily_section_is_emitted_above_the_tree`, `a_daily_section_with_no_chats_is_not_drawn_at_all`, `the_daily_section_draws_a_header_its_count_and_its_chats` |
| TP-DAILY-03 | The fold is per-display and takes every row below the header with it; the section's rows keep area vectors of their own, carrying no `ws_idx` | Folding on a laptop would close the monitor's section — the complaint per-display surfaces exist for; and a daily row folded into the workspace-indexed vectors makes every press resolve as some other checkout's chat | `folding_the_daily_section_takes_every_row_below_it`, `daily_rows_stay_out_of_the_workspace_indexed_vectors`, `the_daily_switches_toggle_both_ways_and_carry_the_read_budget` |
| TP-DAILY-04 | Five chats and a switch; opening the section lists them all and moves the read budget with it, and the switch is the way back | Without the bound a machine with a thousand home transcripts buries the tree under its own history; without the budget the switch promises older chats the parse never fetched; without the way back it is not a switch | `the_daily_section_lists_five_chats_and_offers_the_rest`, `the_daily_switches_toggle_both_ways_and_carry_the_read_budget` |
| TP-DAILY-05 | The moment a workspace claims the daily directory the section goes quiet | Those chats are that workspace's drawer then; drawn in both places the reader has no way to tell which of the two is live | `a_workspace_claiming_the_daily_directory_silences_the_section` |
| TP-DAILY-06 | Focus hides the section — unless one of its chats is running, which keeps it visible | Daily chats sit in no tree, so they go quiet with everything else the filter narrows away; but a filter may narrow what you see and never hide where you are | `focus_hides_the_daily_section_unless_one_of_its_chats_is_running` |
| TP-DAILY-07 | A daily row resumes in the daily directory, switches to the chat's live tab instead of resuming it twice, and answers a stale index with nothing | Substituting the active workspace's path is #46 with the roles reversed; a second tab on one conversation is #45 in another surface; and the list can refresh between the frame a person clicked and the click arriving | `a_daily_chat_resumes_in_the_daily_directory`, `a_daily_chat_already_running_is_switched_to_rather_than_resumed`, `a_click_on_a_daily_row_that_no_longer_exists_does_nothing` |

| TP-DAILY-08 | The phone drawer draws the same section in the same order, with a plain section title rather than a fold; the cursor stops on its chats and on nothing else there, and a tap resumes in the daily directory | Two surfaces walking one tree is the point — a phone-only omission means a chat reachable from one screen and not the other; and a cursor resting on a title that does nothing is a press that goes nowhere, the rule `SectionTitle` and `ChatNote` already keep | `the_phone_drawer_carries_the_daily_section_at_the_top`, `the_phone_cursor_stops_on_daily_chats_but_not_on_their_title`, `tapping_a_daily_chat_resumes_it_in_the_daily_directory` |
