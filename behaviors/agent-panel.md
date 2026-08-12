# Agent panel — active-row highlight and the close road

The lower sidebar panel answers "which agent am I on" at a glance. The active
row speaks the active tab's visual language — accent background with contrast
text — so the same question reads the same way on both surfaces, and the
passive rows give up their bold so the eye lands on the active one. Chosen by
the user on 2026-07-29 (accent-row model + passive dimming); design rationale:
`.local/prd/2026-07-29-flash-and-agent-panel-PRD.md`.

| ID | Behavior | What breaks if it is lost | Verified by |
|---|---|---|---|
| TP-AGPANEL-01 | The active agent's card (all of its rows, contiguous) wears the accent background with contrast text; no passive card does | The panel stops answering "which agent am I on" — the complaint this family exists for: active and passive rows differed only by a faint background | `the_active_agent_row_wears_the_accent_background_and_the_rest_stay_muted` |
| TP-AGPANEL-02 | A passive agent's name is drawn without bold | Every row shouts at the same volume, so the active card's emphasis carries no information | `the_active_agent_row_wears_the_accent_background_and_the_rest_stay_muted` |
| TP-AGPANEL-03 | A right-click on an agents-panel row opens that row's own menu carrying its (workspace, tab, pane); the panel's empty space opens nothing, and the row is matched before the per-tab roads because the panel is shared chrome on every sidebar tab | The press falls through to the workspace list's row resolver, which answers for a different section — the panel would manage a workspace the user never pointed at, or the Projects tab would swallow the press entirely | `right_click_on_an_agent_row_opens_its_menu_and_the_panel_gap_stays_inert` · `the_agent_row_menu_offers_exactly_the_close_verb` |
| TP-AGPANEL-04 | "Close agent" closes the row's OWN pane through the pane close road (graceful close with its confirmation gate intact, never a kill), not whichever pane is focused | Closing from the panel kills the wrong agent — the focused one — or bypasses the worktree-group confirmation the pane road owns | `closing_an_agent_row_closes_that_row_pane_not_the_focused_one` |
| TP-AGPANEL-05 | A chat row offers "Close agent" only while that chat still has a running tab behind it, and offers it last | A drawer full of finished transcripts grows buttons that cannot act, and an irreversible verb sits above reversible ones | `a_chat_row_offers_close_only_while_something_is_running` |
| TP-AGPANEL-06 | The chat road resolves the session's tab at pick time, not at menu-open time; a session that no longer runs closes nothing | A menu left open while the agent exits fires a close at a tab index something else has moved into — a bystander dies | `closing_a_chat_agent_targets_the_session_tab_and_a_stale_menu_is_inert` |

Source markers: `src/ui/sidebar.rs::render_agent_detail` (style block) ·
`src/app/input/mouse.rs` (right-click road) · `src/app/input/modal.rs` (both
dispatch roads) · `src/app/state.rs::ContextMenuKind::AgentEntry`.
