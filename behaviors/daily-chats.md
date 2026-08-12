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

## Not yet landed

TP-DAILY-02..08 (the section's own rows, its fold, the older-row switch, the
double-count guard, the focus exception, the press target, and the phone
drawer's parity) are specified in `.local/prd/daily-chats-section.md` §6–§7 and
land with the emission and render layers. They are listed there rather than
here on purpose: a registry row whose test does not exist yet is a gate that
reports coverage it does not have.
