# Files as a workspace tab — registered behaviors

Fork feature. Files opens in the same content area terminals and agents open
in, as a peer entry in the workspace tab strip.

This family exists because it **reverses a written contract**. Until 2026-07-25
`src/ui.rs` stated that the tab strip was terminal-app chrome and that the Files
surface owned the *complete* stage, reclaiming the strip's row. Reversing that
deliberately is why every rule below is written down rather than left implicit.

Two ideas run through the whole family:

- **A tab is not a mode.** Leaving Files backgrounds it; it keeps its state and
  stays in the strip to come back to. Only an explicit dismissal closes it.
- **An open Files tab is not necessarily the active surface.** Every rule that
  used to read `file_manager.is_some()` as "Files owns the stage" is now wrong,
  and each place that got it wrong is pinned below.

Format and rules: [`README.md`](README.md).

## Shell chrome

| ID | Behavior | Breaks if lost | Verified by |
|---|---|---|---|
| TP-FTAB-CHROME-01 | The tab strip is shell chrome, not terminal-app chrome: it stays present and byte-identical while the Files surface owns the stage. | Files hides the strip, so it reads as leaving the workspace rather than switching to another tab, and the Files tab becomes unreachable by mouse. | `tab_strip_stays_present_while_the_files_surface_owns_the_stage`, `compute_view_normalizes_file_manager_viewport_after_resize`, `compute_view_snapshots_and_clears_file_manager_row_areas` |
| TP-FTAB-CHROME-02 | Both stage surfaces receive exactly the same content rect below the strip. | An off-by-one hides the last Files row — invisible in a screenshot, and reported only as "the bottom entry is unreachable". | `both_surfaces_receive_the_same_content_area` |

## Strip entries

| ID | Behavior | Breaks if lost | Verified by |
|---|---|---|---|
| TP-FTAB-ENTRY-01 | A stage app owns its own entry, disjoint from every terminal tab rect, held in its own vector so `tab_hit_areas` stays index-aligned with `ws.tabs`. | A click on the Files entry resolves as a terminal tab index, switching to whichever tab happens to share that position. | `files_appears_as_a_peer_entry_disjoint_from_terminal_tabs` |
| TP-FTAB-ENTRY-02 | The entry carries the instance's `AppInstanceId`, not a position, and a reopened instance gets a new generation. | Geometry retained across a close and reopen authorizes the new instance, so a stale click acts on a surface the user never pointed at. | `reopened_files_entry_carries_a_new_instance_generation` |
| TP-FTAB-ENTRY-03 | The strip paints exactly one active entry: no terminal tab is active while a stage app owns the content. | Two entries look active at once, so the user cannot tell where a keystroke will land. | `only_the_owning_surface_paints_an_active_strip_entry` |
| TP-FTAB-ENTRY-05 | Stage entries are pinned to the strip's leading edge, ahead of every terminal tab, and stay there while the terminal tabs overflow and scroll. | The Files tab drifts with the terminal tabs and scrolls out of reach, so a pinned entry is not pinned. | `files_entry_is_pinned_left_of_every_terminal_tab` |
| TP-FTAB-ENTRY-04 | `hide_tab_bar_when_single_tab` counts stage entries, so an open Files tab brings the strip back. | With the option on and one terminal tab, the Files tab exists but has no visible entry to click. | `single_terminal_tab_plus_files_keeps_the_strip_visible` |

## Switching

| ID | Behavior | Breaks if lost | Verified by |
|---|---|---|---|
| TP-FTAB-INPUT-01 | Focusing a terminal tab returns the stage to the terminal workspace and **backgrounds** Files; every resident instance stays in the strip. | Clicking a terminal tab destroys the Files tab and its directory, which is not tab behavior at all. | `clicking_a_terminal_tab_leaves_files_open_as_an_inactive_entry` |
| TP-FTAB-INPUT-02 | View projections read the file manager only while it owns the stage, through `staged_file_manager`, never the field. | A backgrounded Files tab keeps its rows and header actions clickable underneath the terminal surface. | `inactive_files_tab_projects_no_stage_geometry` |
| TP-FTAB-INPUT-03 | Clicking a strip entry activates the exact instance its geometry names, and the switch itself retires the terminal projection. | Stale pane hit rectangles act in the window between the switch and the next compute. | `clicking_the_files_entry_activates_it_and_retires_terminal_geometry` |
| TP-FTAB-INPUT-04 | An identity that is no longer resident is inert: retired strip geometry cannot bring its surface back. | A click on cells a closed tab used to occupy resurrects or misdirects a surface. | `stage_entry_geometry_is_inert_after_its_instance_closes` |
| TP-FTAB-INPUT-05 | Repeated switching between surfaces destroys no terminal runtime and rebinds no pane/terminal identity. | A terminal dies behind a tab the user expects to return to — silently, with no error anywhere. | `tab_switching_between_surfaces_preserves_every_terminal_runtime` |

## Launcher and toggle

| ID | Behavior | Breaks if lost | Verified by |
|---|---|---|---|
| TP-FTAB-DOCK-01 | The Files launcher raises the resident instance when one exists and opens a new one only when none is. | Clicking Files in the sidebar silently does nothing whenever a backgrounded Files tab already exists. | `dock_files_activation_returns_to_a_backgrounded_files_tab` |
| TP-FTAB-DOCK-02 | Leaving Files through the shell (dock Terminal, Spaces/Projects click) backgrounds the tab instead of closing it. | An ordinary sidebar click discards the Files tab's state. | `dock_terminal_activation_backgrounds_files_instead_of_closing_it`, `spaces_tab_click_restores_terminal_stage_and_preserves_identity`, `projects_tab_click_restores_terminal_stage_and_preserves_identity` |
| TP-FTAB-DOCK-03 | The toggle has three states: dismiss the active Files surface, raise a backgrounded one, open when none exists. | The keybinding closes a backgrounded Files tab instead of raising it, throwing away the directory left open there. | `toggle_raises_a_backgrounded_files_tab_and_still_dismisses_an_active_one` |

## Notes for the next sync

- `TP-FTAB-INPUT-05` is the one to defend hardest. Every other failure here is
  visible on screen; this one is silent and destroys a running process.
- `TP-FTAB-INPUT-02` is the rule most likely to be undone by accident: reading
  `app.file_manager` directly is the obvious thing to write and is now wrong.
  If a projection needs the file manager, it goes through `staged_file_manager`.
- Surface exclusivity (`SF4.3-01`, `SF4.3-02` in `src/ui/surface_host.rs`) is
  **unchanged** by this family. Only the strip is shared; the content below it
  still has exactly one owner per frame.
- Phase 1 is client-local by design: `StageState` holds the Files tab, so
  nothing here touches `workspace::Tab`, `TabSnapshot`, `TabInfo` or
  `PROTOCOL_VERSION`. A Files tab therefore does not survive a restart. Making
  it survive is a separate, deliberate decision — see
  `.local/prd/2026-07-25-files-as-tab-PRD.md` §3.
