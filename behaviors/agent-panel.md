# Agent panel — active-row highlight

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

Source marker: `src/ui/sidebar.rs::render_agent_detail` (style block).
