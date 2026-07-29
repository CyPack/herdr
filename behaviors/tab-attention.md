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

## Before touching this family: the five layers

Every "make the user notice something" feature crosses the same five layers.
Skip one and the feature lands in the characteristic failure of this family:
**state correct, screen empty.** (Recorded here rather than only in
`docs/patterns/feature-change-map.md` §3b because `docs/` is git-ignored — a
fresh clone would lose it.)

| # | Layer | Question | If skipped |
|---|---|---|---|
| 1 | Class | Does this belong to the session or to one display? | Wrong class either leaks to every screen or shows on none |
| 2 | Mark (SET) | Which creation paths mark, which are exempt? | Marking all of them inflates the signal — a restart lights the whole strip; marking too few leaves the reported scenario silent |
| 3 | Clear | One funnel, and is it permanent? | An unclearable mark carries no information; several funnels mean one path is missed |
| 4 | Draw | Which style channel, and is it distinguishable from the neighbouring states? | A `DIM` branch can mute the very thing meant to be noticed; sharing the active tab's channel merges two states |
| 5 | Frames | If the effect is time-based, do frames keep arriving? | The headless renderer only draws on animation ticks — no tick, no flash, and no state test sees it |

Two rules that cost real bugs to learn:

- **Two channels, not one.** Shape (glyph, bar) *plus* colour/background, so the
  signal survives a theme change or a `DIM` branch.
- **Buffer-level proof.** A state assertion is not evidence here. Draw through
  `TestBackend` and assert on the cell's `fg`/`bg`/`modifier`. The file-manager
  previews were invisible in production while 4265 tests were green.

Adding a field to `Tab` touches **five literal sites** (`workspace/tab.rs` ×2,
`workspace.rs` ×2, `persist/restore.rs` ×1) and they do not all want the same
default: spawn ≠ restore ≠ a pane the person moved themselves.

| ID | Behavior | What breaks if it is lost | Verified by |
|---|---|---|---|
| TP-TAB-UNSEEN-01 | A plugin tab opened through the API with `focus:false` is marked unseen; the focused variant comes out already seen | The sheets/edit "Open in New Tab" action goes back to being a silent no-op — the user never learns the tab exists | `a_plugin_tab_opened_in_the_background_is_marked_unseen` |
| TP-TAB-UNSEEN-02 | `tab create focus:false` through the API marks the tab; `focus:true` passes through the switch funnel and is born seen | Background API creates become invisible; or focused creates stay lit forever, making the signal meaningless | `a_background_tab_create_is_unseen_and_a_focused_one_is_not` |
| TP-TAB-UNSEEN-03 | The first visit clears the flag permanently — leaving and returning does not relight it | A highlight that never goes out (or relights) stops carrying information; the strip becomes noise | `visiting_a_tab_clears_its_unseen_flag_for_good` |
| TP-TAB-UNSEEN-04 | An unseen inactive tab is drawn with the `●` glyph + accent foreground + bold, while keeping the inactive background; the active tab keeps its accent background, so the two states are distinguishable side by side; visiting drops both channels | The "strong highlight" the feature exists for disappears from the frame even though state is correct (the FM-preview lesson: suite green, frame empty) | `an_unseen_background_tab_is_highlighted_until_visited` |
| TP-TAB-UNSEEN-05 | Constructors default `unseen` to `false`: restored tabs and tabs born from a moved pane never light up | A restart lights up the entire strip (every restored tab reads as "new"), drowning the real signal | `a_tab_born_from_a_moved_pane_is_not_unseen` |

## Spawn flash

The complement to the unseen mark, chosen by the user on 2026-07-29: a tab
spawned through the constructors (super+t chat tab, UI `+`, API create, plugin
tab) blinks for two seconds so its arrival is visible even when it opens
focused — the unseen mark can never show there because focusing clears it
instantly. Elapsed-based, not tick-based, so the blink looks the same under the
16 ms monolithic and 128 ms headless animation intervals. Design:
`.local/prd/2026-07-29-flash-and-agent-panel-PRD.md`.

| ID | Behavior | What breaks if it is lost | Verified by |
|---|---|---|---|
| TP-TAB-FLASH-01 | Spawned tabs carry a flash window; restored and moved-pane tabs carry none | Either a new tab arrives silently again (the super+t report), or a restart strobes all 27 restored tabs at once | `a_tab_born_from_a_moved_pane_does_not_flash` · `a_background_tab_create_is_unseen_and_a_focused_one_is_not` |
| TP-TAB-FLASH-02 | Inside the window the tab's earned style is REVERSED on the bright phase; outside it the strip carries no trace | The flash never renders, or a residue outlives the window and becomes a permanent glitch | `a_freshly_spawned_tab_flashes_and_the_flash_leaves_no_trace` · `the_flash_phase_blinks_through_its_window_and_then_goes_dark` |
| TP-TAB-FLASH-03 | An open flash window keeps the animation timer armed even with no working pane | The headless renderer only draws on ticks: with no tick the flash is computed correctly and never drawn — invisible to every state-level test | `a_fresh_tabs_flash_window_keeps_the_animation_timer_alive` |

Source markers: `src/workspace/tab.rs` (field, TP-TAB-UNSEEN-05) ·
`src/workspace.rs::switch_tab` (clear, TP-TAB-UNSEEN-03) ·
`src/app/api/plugins/panes.rs::open_plugin_tab` (mark, TP-TAB-UNSEEN-01) ·
`src/app/api/tabs.rs::handle_tab_create` (mark, TP-TAB-UNSEEN-02) ·
`src/ui/tabs.rs::tab_chrome_label` + `render_tab_bar` (draw, TP-TAB-UNSEEN-04).
