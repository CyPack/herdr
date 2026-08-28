# Tab ↔ chat binding — a tab wears the name of the chat it hosts

A fork feature upstream does not have. A tab opened to resume a chat, or a tab
an agent's chat was detected running in, should read as that conversation and
follow its name — an external Claude `/rename` or Herdr's own rename verb — so
the tab strip answers "which chat is this" the way the sidebar drawer does.

The binding lives in `Tab.chat_title` (a DERIVED, never-persisted mirror of the
bound chat's title) and is read through the single `tab_display_name` seam every
surface already uses (tab bar, agent panel, window title, the daily-area
derivation). Precedence: an explicit rename (`custom_name`) wins, then the
hosted chat's title, then the tab number.

Format and rules: [`README.md`](README.md).

| ID | Behavior | Breaks if lost | Verified by |
|---|---|---|---|
| TP-TAB-CHAT-01 | A tab wears the title of the chat it hosts. `tab_display_name` resolves an explicit rename (`custom_name`) first, then `chat_title`, then the number; `is_auto_named` treats a tab carrying a chat title as named, not as a dimmed placeholder. `chat_title` is DERIVED and never persisted: `apply_project_chat_tab_name` seeds it with the project label for an unnamed tab, a resume overrides it immediately from the clicked row, and `sync_bound_tab_titles` — run last in `load_chat_history`, after the merge and every ledger overlay — mirrors each bound tab's current `row.title` onto it, so an external `/rename` and Herdr's rename verb both reach the tab through one path. A withdrawn (blank) name leaves the tab alone; an explicit rename still outranks the chat title. | The reported request: "chat session name ini degistirdigimde tab name i de degismeli" and "tab actigimda ... o tab i o chat session ile sync yap". Before this the resume path stamped the project DIRECTORY into `custom_name`, so a resumed tab read as its folder and could never follow the conversation, and a herdr rename froze `custom_name` so a later `/rename` could not move it — measured on the live server, all ten open tabs carried `custom_name: null` and `resumed_session_id: null`, wearing bare numbers. Persisting `chat_title` would let a restart show a stale name a rename had since changed; re-deriving it each sync keeps it honest. Writing the derived name into `custom_name` instead of `chat_title` would make an explicit tab rename indistinguishable from a follow-the-chat name, so the user's own rename could be silently overwritten by the next `/rename`. Seeding via the sync only (no direct write in `apply_chat_rename`) would miss a chat with no drawer row yet. | `resuming_a_chat_names_the_tab_after_the_conversation`, `a_bound_tab_wears_its_chats_title_and_follows_a_rename`, `a_rename_reaches_the_open_tab_wearing_that_conversation`, `project_chat_tab_name_applied_to_auto_named_tab`, `project_chat_tab_name_respects_existing_custom_name` |
