# Multi-monitor herdr: why two clients mirror each other, and what per-client focus would take

> Status: architecture research, verified against the code on 2026-07-12 (fork `CyPack/herdr`,
> master `933e4b8`). No implementation decision has been made. This note exists so anyone who
> wants to work on multi-monitor support starts with the evidence instead of re-deriving it.

## The observable problem

Open two herdr clients on two monitors, attach both to the same server, and click a tab in
either one: **both screens switch**. You cannot look at two different tabs of the same herdr
session at the same time. Users coming from tmux will recognize this instantly — it is exactly
the behavior of attaching one tmux session from two terminals.

Two things that are *not* broken, and matter for any design work:

- **Background tabs already run asynchronously.** Pane processes and agent runtimes live
  server-side in `TerminalRuntimeRegistry` (`src/terminal/runtime_registry.rs`); an agent in an
  unfocused tab keeps working, keeps producing output, and keeps reporting state to the agent
  panel. The limitation is purely about *viewing*, not about *execution*.
- **Multiple simultaneous clients are supported and stable** — they just share one view
  (`tests/multi_client.rs::multi_client_allows_multiple_simultaneous_connections`).

## Why it happens: the view is server-owned and singular

Evidence chain, all verifiable in the tree at `933e4b8`:

1. **One `AppState` for the whole server.** The focused workspace and tab are single fields of
   the shared state — `AppState.active` (`src/app/state.rs`) plus `Workspace.active_tab`
   (`src/workspace.rs`). There is no per-client copy of either. Every input path that switches
   tabs funnels into `AppState::switch_workspace_tab` (`src/app/actions.rs`), which mutates
   that single state.

2. **Frames are broadcast to every client.** The headless server renders one frame from that
   one state and sends it to all attached clients —
   `tests/multi_client.rs::multi_client_broadcasts_frame_updates_to_all_clients` pins this
   behavior. A tab switch performed by client A is therefore repainted on client B by design.

3. **The shared frame is sized to the smallest client.** When a smaller client joins, the
   effective terminal size shrinks for everyone
   (`tests/multi_client.rs::multi_client_effective_size_shrinks_when_smaller_client_joins`,
   `...smallest_leaving_resizes_up_for_remaining_clients`) — the same "smallest attached
   client wins" rule tmux applies to a shared session.

4. **"Foreground client" is an input/ownership concept, not a view concept.** The server
   tracks which full-app client acted most recently
   (`src/server/clients.rs::latest_app_client`, keyed by `last_activity`) and pins pane
   runtime sizes plus client-local notifications to it
   (`src/server/headless.rs::client_local_notifications_target_foreground_client_only`).
   Non-foreground clients still see the same workspace/tab — they are followers of the same
   view, at their own frame size.

5. **Half of a per-client render path already exists.**
   `src/ui.rs::compute_view_without_resizing_panes` computes view geometry for a
   *non-foreground client's own frame size* without touching the shared pane runtimes. So the
   codebase can already lay out the same state differently per client — what it cannot do yet
   is lay out *different state* (a different focused tab) per client.

## Working with it today (no code changes)

Run **one server instance per monitor**. The server/client socket is path-addressed, so a
second, fully independent session is one environment variable away:

```
env HERDR_SOCKET_PATH=$HOME/.config/herdr/monitor2.sock herdr
```

The integration tests use exactly this mechanism to spawn isolated servers
(`tests/server_headless.rs::spawn_server`, `tests/detach_reattach.rs::spawn_server`), so it is
a supported seam, not a trick.

Properties of this setup:

- Each instance owns its own workspaces/tabs/panes; input, focus, and size are fully
  independent per monitor. Split your projects across instances and both screens work in
  parallel, with all agents running concurrently.
- Limitation: a pane/chat lives in exactly one instance. There is no cross-instance tab
  move/handoff, and the Projects sidebar of instance A knows nothing about chats opened in
  instance B's instance-local tabs (the underlying session files are shared on disk, so
  resumable chats appear in both Projects lists — but the *live* tab exists in only one).

## The real feature: per-client focus (design sketch for whoever picks this up)

Goal: N clients attached to one server, each with its own focused workspace/tab, sharing the
same panes/agents/runtimes underneath.

What would have to change, roughly in dependency order:

1. **Split "session organization" from "who is looking at what".** Today `AppState.active`,
   `Workspace.active_tab`, and the whole `ViewState` projection double as both. Per-client
   focus needs a `client_id → (active workspace, active tab, view/scroll state)` map on the
   server, with the shared `AppState` keeping only session structure. The repo's own
   runtime/client boundary guardrail (AGENTS.md, "Runtime/client boundary guardrail") already
   points this direction: view state is client business, session structure is shared.
2. **Route input per client.** Key/mouse handling currently mutates the single focus
   (`src/app/input/*` → `switch_workspace_tab`). Input events arrive tagged with a client
   connection (`src/server/client_transport.rs`); the handlers would need to resolve *that
   client's* focus map entry instead of the global one.
3. **Render per client.** The broadcast in the headless loop becomes a per-client render:
   compute view for each client's (focus, frame size) pair — `compute_view_without_resizing_panes`
   is the existing precedent — and send each client its own frame. Cost scales with attached
   clients; dirty-tracking per client is the interesting part.
4. **Decide pane-runtime sizing.** Today runtimes resize to the foreground client
   (`resize_background_tab_panes_*` in `src/ui.rs`). With two clients viewing two different
   tabs, both viewed tabs want "foreground" sizing; the smallest-client rule would need to be
   scoped per viewed tab instead of per server.
5. **Protocol impact.** New client→server messages (or fields) for per-client focus and
   per-client frames touch `src/protocol/wire.rs`; `PROTOCOL_VERSION` rules in AGENTS.md apply.
   Persistence should stay untouched if focus maps are ephemeral (a reconnecting client can
   fall back to the shared/default focus).

Open questions someone should answer before writing code:

- Does the *server* still need a canonical "active tab" (e.g. for API consumers,
  `herdr agent read`, notifications), and if so, which client's focus feeds it?
- What do client-local vs. shared notifications mean when two clients watch different tabs?
- How does this interact with the mobile client and `terminal_observe` attach-less viewing
  (`src/server/headless.rs::terminal_observe_allows_multiple_clients_without_attach_ownership`),
  which is already a second "viewer" concept?

## Provenance

Findings were produced with a code-knowledge-graph pass over this repository plus targeted
reading of the files and tests referenced above; every claim links to a symbol or test you can
run (`cargo nextest run --bin herdr -E 'test(multi_client)'`). Written on the CyPack fork;
upstream may evolve — re-verify the anchors before building on this.
