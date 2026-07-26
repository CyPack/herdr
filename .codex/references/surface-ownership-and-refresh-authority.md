# Surface Ownership and Refresh Authority

## Record

| Field | Value |
|---|---|
| Verified | 2026-07-26 |
| Scope | Fork-local Files/Miller surface, popup panes, plugin action registry |
| Evidence tier | `source_code` + executable (RED/GREEN in-repo tests) |
| Gates at time of writing | full suite green, formatting clean, maintenance suite green |
| Overall confidence | high (`0.95`) for the three patterns; each is anchored by a registered behavior |

This record answers one question: when several layers can each claim the same
row, the same pixel, or the same capability, which one is authoritative? Three
defects in a single session all reduced to a layer reading the wrong authority,
and all three were invisible until an unrelated change made the losing path
execute.

Paths, plugin ids, project ids and file names below are placeholders. Substitute
the values from the checkout being worked on.

---

## P1 — A refresh is not a navigation

**Rule.** A path that re-reads a directory from disk rebuilds its projection
from the *cursor* authority, not from the activated selection.

The trail carries two distinct notions of "where the user is":

- a vertical cursor override, moved by a click or an arrow key
- an activated selection, moved only by explicit activation

The cursor accessor is defined as "the override wins; the activated selection is
its fallback". Any consumer that reads the activated selection directly has
silently opted out of that rule.

**Failure mode.** Clicking a row moves the cursor but deliberately does not move
the activated selection. A periodic reconcile that re-projects from the
selection therefore drags the highlight — and the detail panel — back to the
activated row a second or two after every click. On a freshly opened column that
row is the first one, so the user sees the focus "jump to the top" at a fixed
interval, with no input of their own.

**Second half of the same defect.** Re-marking a selection clears the cursor
override as a side effect. A refresh that re-marks the same path it already had
therefore destroys the override even when nothing about the selection changed.
Save the override before the re-mark and restore it while the row it names still
exists on disk; a row that disappeared should honestly lose focus.

**Detail follows focus.** The detail/preview panel belongs to the focused row,
which is what a plain cursor move already does. Preparing it from the activated
selection makes the panel describe a different file than the highlight.

**Test shape.** Build the real timeline: focus the row a click would focus,
advance the scheduler to the refresh deadline constant instead of sleeping, then
assert the projection cursor, the trail cursor authority and the detail panel
all still name that row. A test that only asserts "the row still exists" passes
while the defect is present.

---

## P2 — A floating pane is not a member of the surface it floats over

**Rule.** The popup pane is its own axis. It has no entry in the tab surface's
pane list, so every per-pane sweep — placing pictures, measuring visibility,
anything written as "for each pane in the surface" — skips it unless it is added
explicitly.

**Failure mode.** A viewer running inside a popup writes its pictures to its own
terminal. Nothing collects them, so the popup shows its text and status line and
never its image: the one thing the user opened it for is the one thing missing.
The mirror image is just as bad — if the surface underneath keeps placing its
own picture, that image paints across the popup, because a placed image is not
clipped by the text drawn on top of it later.

**Ownership.** While a popup is up it owns the picture layer. This is the same
ownership the input path already gives it (keys reach the popup first) and the
render path already gives it (the popup is drawn last, unconditionally). Making
the graphics path agree removes the z-order argument entirely, rather than
trying to win it.

**Checklist for any new per-pane pass.** Ask about three ownerships before
writing the loop: the tab-surface pane members, the file-manager surface, and
the popup. If one is unanswered, the behavior changes silently depending on
which surface the user happens to be on.

---

## P3 — An installed plugin registry is a snapshot, not a view

**Rule.** The registry file written at link time carries a copy of the
manifest's actions. The runtime reads the registry, never the manifest path.
Editing a manifest changes nothing until the plugin is linked again.

**Failure mode.** An action that is plainly present in the manifest is never
offered, and the search for the defect starts in routing or dispatch code — the
one place it cannot be.

**Diagnosis first, always.** Read the registry for the config profile under test
and print each plugin id with its action ids, then compare against the manifest
on disk. One command settles it before any source is read.

**Profile scoping.** The registry lives beside the config, so it is scoped per
profile (a debug profile and a release profile have separate registries) and per
config root. A plugin linked under one profile is invisible to the other. A test
launcher that re-links on every start is a cheap way to make this class of
confusion impossible.

---

## P0 — The law that ties the three together

**Activating a dormant path exposes its defects; it does not create them.**

All three failures above had existed for as long as the code did. They became
visible only when an unrelated change made the losing path execute: a periodic
reconcile that had never run under one of the two runtime modes was connected to
that mode's scheduler, and every defect on that path arrived at once, looking
exactly like a regression introduced by the change that connected it.

Two practical consequences:

1. Before blaming the most recent commit, ask whether the failing path had ever
   executed before it. The answer reframes the search from "what did I break?"
   to "what was never true here?"
2. Before fixing a suspected missing guard, probe the gate that would have
   caught it. In this session the first hypothesis — a stale worker snapshot
   overwriting live state — was disproved by instrumenting the apply path, which
   already revalidated live identity and correctly rejected the result. The real
   defect was one layer further in, and the "fix" would have been decoration.

---

## Anchors

Each pattern is held by a registered behavior in `behaviors/`, so a future merge
cannot remove it silently:

| Pattern | Registered behavior |
|---|---|
| P1 refresh authority | refresh re-projects from the cursor authority and keeps the focused row |
| P2 popup ownership | a popup pane's pictures are placed, and it owns the picture layer while up |
| P3 registry snapshot | not a runtime behavior; carried by the lessons files in this skill |
