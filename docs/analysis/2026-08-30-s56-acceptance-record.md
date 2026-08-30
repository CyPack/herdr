# S56 ACCEPTANCE RECORD — FLICKER + AUDIO (2026-08-30, live @ fe6f42a7)

Two user-facing complaints opened this cycle; both closed the same day with
the user's own acceptance, on the live server, with every claim measured.

## 🌟 USER ACCEPTANCE (VERBATIM, TRANSLATED IN PLACE)

- **FLICKER — ACCEPTED:** "flicker olmuyor, agent geçişlerinde / resize'larda
  görüntü kalıntısı kesinlikle yok" (*no flicker; absolutely no residue on
  agent switches or resizes*) — local run, live trunk.
- **AUDIO — ACCEPTED:** "ses de sağlam şekilde geliyor, harika" (*audio comes
  through solid — great*) — after one browser-pane restart, video audible.

## ✅ WHAT LANDED (commit chain, in order)

| sha | change |
|---|---|
| 4345b144 | fix(gfx): re-seat placements per cause, not per frame (TP-GFX-REPLAY-01) |
| b0a279ab | fix(gfx): file the re-seat request once, at the encoder's door |
| 57bf70ec | feat(audio): recorder, watcher, constants derived from the protocol |
| 814080fe | feat(audio): start and stop a pane's capture from the server tick |
| fe6f42a7 | fix(pane): give every pane child the session runtime directory back |

## 🔍 THE THREE ROOT CAUSES FOUND AND CLOSED TODAY

1. **RE-SEAT PER FRAME.** Every full render asked the encoder to replay kitty
   placements, so a once-a-second legitimate status-bar change re-seated the
   browser picture on the wire each tick — rendered by the terminal as the
   pane visibly "refreshing". Now the re-seat is per cause: a text-refresh-only
   frame moves zero graphics bytes; resize/divider and every real full cause
   keep their re-seat.
2. **STALE ARTIFACT FETCH.** The delivery chain built on the fleet-chosen box
   but always fetched the binary from a fixed host, so a "delivered <sha>"
   label was written over a binary that did not contain that sha. Fetch now
   resolves the build node from the run ledger and refuses to deliver when it
   cannot — a stale fallback is a silent wrong delivery.
3. **STRIPPED ENVIRONMENT SILENCES AUDIO.** The server process carries no
   session variables through the delivery/handoff chain, so every pane child
   (the web pane's browser first) probed for the PipeWire socket, found
   nothing, and audio died silently while video kept flowing. Every pane child
   now gets the session runtime directory derived at spawn when absent —
   which also fixed months of silent `pactl` failures in agent shells.

## 📏 MEASUREMENTS (before → after)

- Scheduled-tasks full-render pulse: present in **2597/2604** profiled windows
  → **2 events in 186 s** (~99.9% down); the 99 bar ticks in that window all
  travelled the new text-refresh path with zero graphics bytes.
- PipeWire before the environment fix: the browser's AudioService spawned but
  never connected (no client, no stream); after fix + pane restart: stream
  born, user hears it.
- Every delivery since the fetch fix is sha-verified: installed hash equals
  the build box's hash equals the labelled trunk.

## 🗺️ OPEN AREAS (next development, recorded for versioning)

- **Browser resource use while playing video** (~63% CPU measured, expected
  Electron decode cost): sleep/throttle the browser while its pane is hidden
  (dormancy family), and review the adaptive-rendering prior art.
- **Retained-path replay economy** (delta-wiring on RESYNC_AFTER_PATCHES),
  unblocked by the flicker work.
- **Second-client residue** (separate, older item; terminal-app experiment
  pending on the user).
