# Tab attention — unseen background tabs

A tab opened in the background (plugin "Open in New Tab", API `tab create
focus:false`) is the only evidence that the action worked. Without a mark the
strip shows nothing and the action reads as a silent no-op — the reported
scenario behind this family. The model is tmux's window activity flag: the mark
is persistent, belongs to the session (not to a display), and the first visit
clears it for good.

The flag lives on `Tab` (shared session organization), NOT in the
`client_surfaces!` registers — so the park/promote/broadcast/adopt machinery
never touches it and the popup-leak family of bugs cannot reach it. Design
rationale and the class/seam analysis: `.local/prd/2026-07-29-unseen-tab-highlight-PRD.md`.

| ID | Behavior | What breaks if it is lost | Verified by |
|---|---|---|---|
| TP-TAB-UNSEEN-01 | A plugin tab opened through the API with `focus:false` is marked unseen; the focused variant comes out already seen | The sheets/edit "Open in New Tab" action goes back to being a silent no-op — the user never learns the tab exists | `a_plugin_tab_opened_in_the_background_is_marked_unseen` |
| TP-TAB-UNSEEN-02 | `tab create focus:false` through the API marks the tab; `focus:true` passes through the switch funnel and is born seen | Background API creates become invisible; or focused creates stay lit forever, making the signal meaningless | `a_background_tab_create_is_unseen_and_a_focused_one_is_not` |
| TP-TAB-UNSEEN-03 | The first visit clears the flag permanently — leaving and returning does not relight it | A highlight that never goes out (or relights) stops carrying information; the strip becomes noise | `visiting_a_tab_clears_its_unseen_flag_for_good` |
| TP-TAB-UNSEEN-04 | An unseen inactive tab is drawn with the `●` glyph + accent foreground + bold, while keeping the inactive background; the active tab keeps its accent background, so the two states are distinguishable side by side; visiting drops both channels | The "strong highlight" the feature exists for disappears from the frame even though state is correct (the FM-preview lesson: suite green, frame empty) | `an_unseen_background_tab_is_highlighted_until_visited` |
| TP-TAB-UNSEEN-05 | Constructors default `unseen` to `false`: restored tabs and tabs born from a moved pane never light up | A restart lights up the entire strip (every restored tab reads as "new"), drowning the real signal | `a_tab_born_from_a_moved_pane_is_not_unseen` |

Source markers: `src/workspace/tab.rs` (field, TP-TAB-UNSEEN-05) ·
`src/workspace.rs::switch_tab` (clear, TP-TAB-UNSEEN-03) ·
`src/app/api/plugins/panes.rs::open_plugin_tab` (mark, TP-TAB-UNSEEN-01) ·
`src/app/api/tabs.rs::handle_tab_create` (mark, TP-TAB-UNSEEN-02) ·
`src/ui/tabs.rs::tab_chrome_label` + `render_tab_bar` (draw, TP-TAB-UNSEEN-04).
