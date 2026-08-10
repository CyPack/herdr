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
