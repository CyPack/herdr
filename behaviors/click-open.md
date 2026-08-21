# Pane · Click-to-open (fork behaviors)

Ctrl+click on terminal output learns a second token type: filesystem paths.
CC's `src/ui.rs:42`-style output opens on the right without leaving the
keyboard's world. The URL pass keeps owning URLs; a plain click keeps going
to the pane.

| id | behavior | why it matters | tests |
|---|---|---|---|
| TP-CLICKOPEN-01 | The modified press resolves a path token under the cell through the SAME soft-wrap line walk the URL resolver uses (one extraction seam, `visible_line_at_pane_cell`); the scanner captures a quoted span (the one way a space survives), strips a trailing `:line`/`:line:col` and sentence punctuation in punctuation→suffix→punctuation order, and refuses URLs, bare words and all-digit "extensions". Resolution: `~` through the worktree expander, relative against the pane's foreground cwd. A directory opens Files there; a file travels the ONE plugin-intent seam every open-click travels (`queue_file_open_intent` — the FM preview click and the context menu share it), image/PDF falls back to the preview viewer, and a missing path answers with a toast, never silence — with the press consumed, never leaked to the pane | The user's ask: click what CC prints, see it open beside — for any TUI's output, not one agent's. Two extraction walks would drift on exactly the wrapped lines that need them; a second open-file resolver would drift from what the menu shows; and a swallowed miss teaches that clicking does nothing | `path_tokens_are_found_stripped_and_refused_by_class`, `ctrl_click_on_a_missing_path_answers_with_a_toast`, `ctrl_click_on_an_existing_image_path_opens_the_viewer`, `ctrl_click_on_a_directory_opens_files_there`, `ctrl_click_url_invokes_plugin_link_handler_but_super_click_does_not` |
