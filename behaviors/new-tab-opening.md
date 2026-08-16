# Opening a tab — the bookkeeping every path shares

Three places in this fork create a tab and put a running command in it: a right
press on a bar section (`app/tabs.rs`), a project chat (`app/projects.rs`), and
a plugin pane (`app/api/plugins/panes.rs`). They differ in where the working
directory comes from, what environment they add, how they report failure, and
what extra bookkeeping they do — but underneath they perform the same sequence,
and it is the sequence, not the differences, that is easy to lose.

Register the runtime. Clear the alias the new root pane shadows. Register the
terminal. Take focus. Persist the session. Announce the tab, then its pane.

Every step is invisible when it stops happening. An unregistered pane is one no
surface can draw; an unannounced tab is one every API subscriber is missing; a
surviving alias goes on resolving to a pane that is no longer what it names.
None of them changes anything on screen at the moment they break.

`app/tabs.rs` has carried TP-CHROME-61 for the right-press path since it was
written. The project-chat path carried nothing: `git grep "TP-"
src/app/projects.rs` found one match, and it was a comment about a mobile
drawer. Its registration was covered incidentally by T5a-7, which reaches for
the terminal and would fail if it were missing — but its announcement and its
alias handling were owned by no test at all. The rows below are that gap closed,
and they were written **before** the shared helper that moves those lines, so
that "it already worked" and "the refactor did not break it" stay
distinguishable.

Design rationale: `.local/prd/f71-new-tab-callers.md`.

| id | behavior | why it matters | tests |
| --- | --- | --- | --- |
| TP-TAB-NEW-01 | Opening a project chat announces both the tab and its root pane | An unannounced tab leaves every API subscriber with a tab list that is missing a tab, and nothing on screen says so. TP-CHROME-61 holds this for the right-press path; this is the same contract on the path that had none, and these two emissions are exactly the lines a shared helper moves | `open_chat_tab_announces_the_tab_and_its_pane` |
| TP-TAB-NEW-02 | A new root pane clears the alias it shadows, and clears only that one | The quietest step there is: if it stopped, nothing on screen would change and an old alias would go on resolving to a pane that is no longer what it names. The second half matters as much as the first — a helper that cleared the whole table would satisfy a one-sided check while losing every other alias | `open_chat_tab_clears_only_the_alias_its_new_pane_shadows` |
