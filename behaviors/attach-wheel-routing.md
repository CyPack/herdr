# Attach wheel routing — the mode line

A wheel over an attached terminal has always been routed by the server
(`wheel_routing()`): an application that asked for the mouse gets real wheel
events, an alternate screen with alternate-scroll gets arrow keys, everything
else scrolls the host's scrollback. What no client could do was KNOW which of
those it was talking to. The web client's reading mode needs exactly that word
to take an upward gesture locally without stealing vim's wheel or feeding
claude's history-jog — the two live complaints of 2026-08-10 (herdr-web
`.local/PRD-F27-F28-SCROLL-LOCALITY.md` §13).

So attach and observe connections now receive `terminal.routing` — once on
attach, once per change, never per frame.

| ID | Behavior | What breaks if it is lost | Verified by |
|---|---|---|---|
| TP-WHEELMODE-01 | `ServerMessage::TerminalRouting { routing }` survives the wire for all three routing states, appended after every older variant so bincode discriminants stay stable | The sidecar's gesture gate reads garbage or the message never decodes; the web reading mode falls back to its button forever and nobody sees why | `terminal_routing_roundtrip` |
| TP-WHEELMODE-02 | The json-lines translator spells routing as `host_scroll` / `alternate_scroll` / `mouse_report` under `"type": "terminal.routing"` | The browser gates its reading gesture on exactly these words; a serde rename would silently disable gesture entry on every pane | `terminal_routing_line_spells_the_routing_in_snake_case` |

Source marker: `src/server/headless.rs` (full-render attach arm, `attach_wheel_routing`).
