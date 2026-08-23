# Sidebar · Projects tab (fork behaviors)

The Projects tab lists pinned project directories and their recent agent
chats, newest first. That list is rewritten by the session poll while the
laid-out row rects still describe the previous order — the exact window in
which a click used to land on whichever chat had *shifted into* the stale
index, resuming a conversation in the wrong project directory.

| id | behavior | why it matters | tests |
|---|---|---|---|
| TP-PROJTAB-01 | Projects-tab hit-testing is generation-guarded: `refresh_project_sessions_in` bumps `projects_sessions_generation` on every poll rewrite, `compute_view` snapshots that generation next to `project_row_areas`, and `project_row_kind_at` — the single seam every Projects-tab click and right-click resolves through — answers `None` whenever the two disagree. A stale click is a no-op; the next frame lays out fresh rects and the next click means what the user sees. The same stale-projection discipline `resident_files_generation` gives the Files surface | Without the guard a click resolved bare indices (`proj_idx`/`chat_idx`) against a list that had already shifted underneath the rects — measured red: clicking the drawn `sess-2` row resumed `sess-1`. That is "the chat opened in the wrong directory" (the #46 family), triggered by nothing more than an agent writing to its transcript between two frames | `a_stale_projects_click_is_inert_after_the_list_shifts`, `the_session_poll_bumps_the_projects_generation` |
