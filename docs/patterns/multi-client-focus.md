# Multi-client focus — one session, several displays

## The scope, in the user's words

> One herdr session, N displays, a herdr client attached to that session on each
> display. Think of five tabs open in the tile-layout content area: I want to
> watch all five in parallel, one per monitor, at the same time. Today whichever
> tab I clicked last is the tab — and the agent — every display shows. That
> throws away the whole point of having several monitors.

Read that literally, because two nearby things are **not** in scope:

| In scope | Not in scope |
|---|---|
| herdr's own tabs — `Workspace.tabs`, each a `Tab` in `src/workspace/tab.rs` with its own pane tree, layout, name and resumed agent session | The terminal emulator's tabs (Ghostty, kitty) |
| The tile-layout content area those tabs render into | tmux windows |
| Which display is looking at which tab | Which tabs exist, their names, their pane trees — those stay shared session state |

The unit of parallelism is a **tab**, and the thing that must stop being global
is **who is looking at which one**.

## Why it is three couplings, not one

A single "active tab" field is the visible symptom. Underneath there are three
independent globals, and fixing only the middle one leaves the feature broken in
ways that are harder to see than the original bug.

| # | Coupling | Where | Consequence if left global |
|---|---|---|---|
| B1 | Active workspace | `AppState.active` | Tabs in different workspaces still drag each other |
| B2 | Active tab | `Workspace.active_tab` | **The reported symptom** |
| B3 | Focused pane | `TileLayout::focus_pane` | Two displays on one tab steal each other's pane focus, and typing lands in the wrong pane |

And two mechanism couplings ride along:

| # | Coupling | Where | Consequence |
|---|---|---|---|
| M1 | Pane size follows the foreground client | `render_and_stream` | Fixing B2 alone makes this **worse**: two displays on different tabs each resize the other's panes |
| M2 | One foreground client, "latest active" | `foreground_client_id` | Touching a display promotes it and pulls state with it |

**B2 and M1 must land together.** Separating focus without separating size ships
a regression: panes jump to the dimensions of whichever display was touched
last, which is worse than one shared tab.

## The model

```
AppState
  viewer: Option<ClientId>          ← whose view is being resolved right now
  ↓ mirrored by AppState::set_viewer
Workspace
  active_tab_by_client: HashMap<ClientId, usize>   ← storage
  default_tab: usize                                ← what a fresh client adopts
  viewer: Option<ClientId>
```

`ClientId` shares the input-source id space, so the id a client's input already
carries is the same id its view resolves through. `0` is the local/monolithic
path.

The viewer window opens at exactly two seams, and closes on a single exit path
so an early return cannot leave another client's view installed:

| Seam | Function | Why |
|---|---|---|
| Input | `App::route_client_events_from` | Write authority — who is switching |
| Render | `HeadlessServer::render_and_stream`, App arm | Read authority — who we are drawing for |

### One authority

`Workspace::active_tab_index()` is the only way to ask which tab is active. The
storage carries no accessor of its own beyond `default_tab()`, whose name says
out loud that it resolves for nobody. This is deliberate: this fork has been
bitten three times by a consumer reading a raw field and silently opting out of
an authority rule (`.codex/references/surface-ownership-and-refresh-authority.md`).

`Deref for Workspace` goes through `active_tab()`, so every `ws.layout`,
`ws.panes`, `ws.zoomed` resolves for the current viewer without a single call
site changing.

### Adoption — the rule that is easy to miss

**A client adopts `default_tab` the first time it is seen, and is independent
from then on.**

Without adoption, a display that has not switched tabs yet owns no tab and keeps
resolving through the default — and the default follows whichever display
switched last. So a display the user never touched is *still* dragged onto the
tab another display just opened. That is the original bug wearing a different
hat, and only a two-client end-to-end test catches it; per-client unit tests all
pass while it is present.

Consequence worth knowing: if a client has not rendered yet when another client
switches, the first one adopts the *new* tab. In production a client renders as
soon as it attaches, so the window is one frame wide. In tests it is wide enough
to write a fixture that proves the wrong thing.

### Size negotiation

A tab is sized to the **smallest display watching it**, component by component
— the model zellij uses (`zellij-server/src/screen.rs:2408-2425`, MIT).

The payoff is the common case: a tab watched by exactly one display keeps that
display's full size and compromises nothing. Only a genuinely shared tab shrinks.
Exactly one client owns a tab's resize, chosen by lowest id, so the outcome does
not depend on client iteration order.

herdr derives layout and pane size from the same area, so the owner runs a
resize pass at the negotiated size and then draws at its own size without
resizing again.

The background-tab sweep had the same problem in reverse — it resized every tab
except the viewer's own — so it skips any tab another display is watching
(`Workspace::tab_is_watched`).

## Doctrine

This is not a departure from `CLAUDE.md`'s runtime/client boundary; it is an
application of it.

> Shared runtime/session fact: belongs in server state.
> **TUI presentation state: belongs only in the TUI/client layer.**

Which tabs exist is a session fact and stays shared. Which one a display is
looking at is presentation state. The fork already recorded the same conclusion
for the Files surface: *"top-level focus remains client-local presentation
state."*

## Registered behaviors

Every rule above is pinned by a `TP-MCF-*` row in
[`behaviors/shared-surfaces.md`](../../behaviors/shared-surfaces.md). They all
live in files upstream also owns, which is exactly the shape a three-way merge
can revert silently: the merge compiles, the suite is green, and multi-display
use is quietly back to one tab for everyone.

## Related

`.local/prd/2026-07-26-multi-client-tab-focus-PRD.md` — dependency chain, test
points, acceptance criteria.

## Surfaces above the workspace

Splitting the workspace, the tab and the pane fixed three levels and left
everything above and beside them shared. The stage — which app surface is on
screen — sat one level higher, so opening Files still sent every display to
Files. The menus, dialogs, prompts, scrolls and gestures sat beside all four
and belonged to nobody in particular.

A surface is declared once, in `client_surfaces!`, and both halves of the swap
are generated from that declaration. That is not tidiness: with dozens of
fields, a surface added to the save side but not the load side keeps living in
one place, every display keeps resolving the same value, and the symptom — a
menu opening on all of them at once — reads as a rendering bug rather than a
missing swap. Declaring it once makes half-migration fail to compile.

### Three groups, and the question that sorts them

> When the session changes this with no display behind it, who should see it?

| Group | Answer | Examples |
|---|---|---|
| `inherited` | Only the default. A display attaching later adopts it; displays already attached keep their own. | `active`, `stage` |
| `broadcast` | Every display. | `mode`, menus, dialogs, prompts, rail, scrolls, sidebar geometry |
| `private` | Nobody, and a new display starts without it. | drag, press, selection, blocking pickers |

`broadcast` exists because a change with no display behind it is sometimes an
instruction rather than a preference. Focusing a pane through the API puts the
session in terminal mode; a display still parked in navigate mode swallows
everything its user types. `inherited` is the deliberate opposite: choosing a
workspace for one display is the entire point of keeping them apart.

`private` is forced rather than chosen. A surface that cannot be compared
cannot be told apart from a display merely being looked at, so promoting it is
never safe. That constraint turns out to be the behaviour you want anyway: a
blocking picker holding a live filesystem view should not be inherited, because
a display that has just attached has not opened one.

### Two rules that are load-bearing and look incidental

**A parked bundle stays in the map while its display is being served.** It is
what the next park compares against to decide whether the display actually
moved. Take it out and every park looks like a change, so the default — the
value the API and the notification path resolve through — is overwritten on
every frame by displays that did nothing. A pane in a background workspace then
reads as foreground and its agent finishes in silence. This was found by one
integration test, not by any unit test.

**A display adopts the default the first time it is seen.** Without it, an
untouched display keeps resolving the default, and the default follows whoever
switched last — the shared-focus complaint wearing a different hat.

### The seam rule — where a surface gets its owner

Every per-display field is read and written through one set of registers. Which
display those registers belong to is decided by whoever opened a viewer window
around the work. So the question that decides correctness is never "is this
field per-display?" — it is **"does the code that touches it run inside a viewer
window, and is it the right one?"**

There are four seams where work reaches those registers, and each one had to be
found the hard way:

| Seam | Where | Owner |
|---|---|---|
| Input routing | `App::route_client_events_from` | the client that sent the event |
| Scheduled work | `App::for_each_display` | each display in turn (TP-SUR-FM-02) |
| Render **and its graphics encode** | `HeadlessServer::render_and_stream` | the client being drawn (TP-MCF-CTX-03/06) |
| API requests that open a person's surface | `handle_api_request_…` | the focused display (TP-SUR-BROADCAST-05) |

Two of these were discovered as live bugs, and both were invisible to the unit
suite because the state logic was correct in isolation:

- **The encode ran one line too late.** The render arm restored the viewer
  before encoding graphics, so the encode read the session default — whose
  owned surfaces hold no file browser. Every file-manager preview stopped
  arriving the moment a second display attached, while one display kept working
  because a sole display shares the register slot with the session.
- **An API request has no display identity.** A file-manager plugin opens its
  viewer by calling back in (`herdr plugin pane open --placement popup`). With
  no viewer around it, the popup landed in the session's registers, and the
  broadcast rule then copied it onto every attached display. One preview click
  covered every screen.

The generalisable lesson: **a surface must be given its owner at the seam where
it is created.** Attribution cannot be recovered later — by the time the popup
exists in the registers, nothing in the state knows whose click produced it.

The corollary is why the API list is explicit rather than "scope every request
to the focused display": a *session instruction* — the API focusing a pane —
must stay session-wide, or a display parked in navigate mode swallows everything
its user types. The broadcast rule and the seam rule are two halves of one
question: does this change belong to a person, or to the session?

### The dependency chain, in the order a change flows

```
input / API / scheduler
        │
        ├── enter_viewer(display)      ← the seam; ownership decided HERE
        │
        ▼
   registers (this display's surfaces)
        │
        ├── work mutates them          ← popup opens, preview decodes, tab switches
        │
        ▼
   restore_viewer(previous)
        │
        ├── park       → surfaces_by_client[display]
        ├── promote    → default_surfaces  (only fields that actually changed)
        └── broadcast  → every parked bundle  (ONLY when no display was serving,
                                               and ONLY for session instructions)
        │
        ▼
   next display served: install_surfaces(parked)  ·  a first-time display: adopt(default) − person-opened
```

Read top to bottom, every live bug in this family is one of three things: the
wrong seam, a step running outside the window, or a field in the wrong class.

### The file browser

The stage and the contents behind it are opened in one transaction and can only
ever be as inheritable as each other, so they live in one group. Keeping them
together is what makes a stage pointing at contents that do not exist
unrepresentable rather than merely repaired.

Neither can be promoted by comparison: comparing a directory listing on every
park means walking a directory's worth of entries per display per frame. What
replaces comparison is the rule that with one display, that display *is* the
session — they share one slot rather than being kept in step, so every
monolithic run behaves as it always did, and the slots separate the moment a
second display attaches. Which display is the sole one has to be read before
anything is inserted, or a display attaching right now parks what the previous
sole display was holding into a slot it will never look in again.

For the same reason these surfaces are handed over rather than copied, in a map
of their own. The comparison the other bundle needs is why it is cloned on
entry; nothing compares these, so nothing needs a second copy.

The three file workers run once per display, inside that display's view. The
workers are unchanged and know nothing about displays: inside the window,
`state.file_manager` is that display's browser, exactly as during its render
and input. Two scheduling loops are the only places that know there is more
than one.

Each worker holds one bounded in-flight request, where a new one supersedes the
one before. That works because there is one requester; with a browser per
display the bound turns into starvation, so the workers are keyed by display
too — below two displays, by the session, for the same reason the surfaces are.
