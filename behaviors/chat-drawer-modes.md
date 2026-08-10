# Chat drawer modes — registered behaviors

Fork feature, the revision of TP-FOCUS-03's reveal. One `[ui] chat_drawer_mode`
value decides how workspace chat drawers open and close: `all-active` (the
default) derives an open drawer for every workspace with a live agent,
`focused` follows the active workspace around, and `manual` only ever obeys
the disclosure clicks.

All of it stands on the per-display ground laid by the L1 migration: the
expanded and suppressed sets are broadcast surfaces, so what one display
opens, quiets, or derives never moves a drawer on another display.

Format and rules: [`README.md`](README.md).

## Mode config

| ID | Behavior | Breaks if lost | Verified by |
|---|---|---|---|
| TP-DRAWER-01 | `[ui] chat_drawer_mode` parses `all-active`, `focused`, and `manual`, and defaults to all-active. | The mode surface disappears: configs stop selecting drawer behaviour and every install falls back to whatever the code happens to do. | `chat_drawer_mode_config_parses_and_defaults` |
| TP-DRAWER-02 | A mode change applies on config reload, without a restart. | Trying a mode means restarting the server, which defeats a live-tunable UI preference and costs the session handoff dance. | `reload_applies_chat_drawer_mode_without_restart` |

## Mode derivation

| ID | Behavior | Breaks if lost | Verified by |
|---|---|---|---|
| TP-DRAWER-03 | In all-active, a workspace whose panes would put a row in the agents panel derives an open chat drawer — the panel's own criterion, so the two surfaces never disagree about "active agent". | The default mode opens nothing: the drawers the user asked to see stay shut, and the feature quietly becomes manual. | `an_agent_workspace_derives_an_open_drawer_in_all_active` |
| TP-DRAWER-04 | Quieting a drawer beats the derivation: a suppressed workspace stays shut with a live agent inside. | A drawer with a live agent cannot be closed at all — the derivation reopens it on the next frame and the disclosure click turns into a lie. | `a_quieted_drawer_stays_shut_despite_a_live_agent` |
| TP-DRAWER-05 | Focused and manual never derive an open drawer; only the expanded set speaks there. | The modes stop meaning anything: drawers open by themselves in manual, and focused loses its one-drawer promise. | `focused_and_manual_never_derive_an_open_drawer` |

## Mode movement

| ID | Behavior | Breaks if lost | Verified by |
|---|---|---|---|
| TP-DRAWER-06 | Focused keeps a one-drawer promise: arriving at a workspace opens its drawer AND closes the one the focus left, on this display. | Drawers accumulate as the focus travels and focused decays into a slow all-active nobody chose. | `arriving_in_focused_opens_the_new_and_closes_the_old` |
| TP-DRAWER-08 | The whole pipeline is per-display: one display quieting a derived drawer changes its own verdict and leaves every other living display's untouched (a display seen for the first time adopts the driven default on purpose — TP-SUR-DEFAULT-01). | The user's iron constraint — "what I do on one display must never interfere with the client on another" — silently breaks at the exact surface this feature was rebuilt to protect. | `one_displays_quieting_never_moves_anothers_derived_drawer` |
| TP-DRAWER-07 | The disclosure click inverts what the person sees: closing a derived-open drawer quiets the derivation for this display (the expanded set is not faked), and reopening withdraws the quieting first. | Closing a derived drawer either does nothing or forges a hand-open that outlives its agent; either way the click stops meaning "toggle what I see". | `toggling_a_derived_drawer_quiets_it_instead_of_faking_a_hand_open` |

