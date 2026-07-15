# M1 Evidence — Focused-Agent Attachment Picker

Date: 2026-07-15

Decision: **GO**, but only for the narrow existing-agent, single-file picker
defined below. M1 does not attach a terminal client, create an agent, create a
pane, upload file bytes, or add a general overlay/component registry.

## Product Delta

The durable product vision in `.local/prd/native-fm/99-MISSION-VISION.md` is
specific: a `+` action on the agent frame must let the user choose a photo or
document for the currently focused agent without replacing the terminal with
the full native-FM page.

That behavior does not exist today:

| Existing surface | Current authority and effect | Why it does not satisfy M1 |
|---|---|---|
| Native FM `SendAgent` / C5 | While the full FM owns `CenterContent`, bind one current UTF-8 path to the focused agent terminal and send the literal path plus CR after scheduled revalidation | Requires leaving the visible agent flow and opening the FM page first |
| CLI `agent attach` | Attach the human terminal client to an agent runtime through `run_terminal_attach` | “Attach” means client/session attachment, not a file reference |
| CLI/API `agent send` | Resolve a named agent and send arbitrary text through the neutral agent API | Has no file picker, selected-path authority, or frame action |
| Plugin file-context action | Invoke an installed, enabled plugin action with current file-context parameters | Requires the full FM context and delegates product meaning to a plugin |
| Remote terminal image-drop bridge | Recognize a local absolute image path only for a remote client, bounded-read the image, and emit `ClientMessage::ClipboardImage` | Transfers image bytes for remote clipboard/image paste; it rejects documents and is not file-reference delivery |

M1 therefore adds one missing interaction, not a second implementation of
`agent attach` or `agent send`:

> From a focused existing agent terminal, activate the frame `+` action or its
> configurable keyboard equivalent, choose exactly one current regular file in
> a blocking client-local picker, and deliver that exact path to the same agent
> through the existing C5 terminal-input authority.

## Scope and Non-Goals

### M1 v1 includes

- an agent-only `+` frame affordance and default `prefix+a` keyboard binding;
- a bounded attachment picker that keeps the terminal pane as the base layer;
- navigation from the active workspace `identity_cwd`, matching native-FM open;
- exactly one regular file per invocation;
- exact UTF-8 path delivery followed by one CR, matching C5;
- current file and target-agent revalidation at the scheduled effect boundary;
- explicit success, cancel, busy, unavailable, stale-target, stale-file, and
  unsupported-path outcomes.

### M1 v1 excludes

- directories and multi-selection;
- file-byte upload, MIME inference, image transcoding, remote clipboard image
  transport, drag-and-drop, shell quoting, or lossy path conversion;
- selecting a different agent, creating a new agent, creating a pane, or
  rolling back any pre-existing resource;
- watcher ownership for the short-lived picker; navigation refreshes state and
  confirmation revalidates the selected file outside render/input;
- new protocol fields, persisted session fields, dependencies, generic page or
  component registries, and a generic overlay stack.

Single-file v1 is intentional. C5 already has an auditable one-path contract.
Multiple paths would require a new agent-visible message format or repeated CRs
that could submit multiple prompts. That is a separate product decision, not a
safe extension of M1.

## Ownership and Reuse

### Client-local presentation state

`AgentAttachmentPickerState` owns:

- one private `FmState` used only by the picker;
- one immutable target snapshot containing workspace ID, `PaneId`, and
  `TerminalId`; Herdr has no persistent tab ID, so the current tab position is
  a live projection located through the stable pane identity;
- one lifecycle state: browsing, pending, or recoverable error;
- no runtime handle and no filesystem watcher.

`ViewState` owns only derived rectangles:

- an optional exact `AgentAttachmentActionArea` for the focused agent frame;
- picker row/action rectangles while the overlay is open.

`compute_view()` derives all geometry. `render()` remains `&AppState`-only and
performs no filesystem, runtime, focus, or hit-area mutation.

### Existing runtime/session authority

Workspace/tab/pane/terminal organization and agent-terminal state remain in
their existing owners. M1 adds no TUI-private socket fact. At confirmation the
scheduled App effect must prove that workspace ID, `PaneId`, and `TerminalId`
still form the active, focused chain through the current tab projection and
that the terminal still satisfies
`TerminalState::is_agent_terminal()`.

### Delivery seam

Refactor the narrow reusable part of C5 rather than call CLI parsing or add a
second sender. The shared scheduled helper accepts one typed request, validates
the live source file and exact target identity, constructs `path.as_bytes()`
plus one `b'\r'`, and calls the existing non-blocking
`try_send_terminal_input` exactly once.

The CLI `agent send` API is intentionally lower-ranked for this client-local
flow: it resolves a human-facing agent name again, while M1 already owns an
exact terminal identity. Re-resolving by name would weaken authority and add an
unnecessary server round trip.

## Recommended UI Pattern

1. Compute one complete three-cell `[+]` hit target on the focused agent
   pane's bottom border. It must not overlap a corner, scrollbar, split border,
   or `inner_rect`. Hide the mouse target when pane borders are disabled or the
   complete target does not fit; `prefix+a` remains the accessible route.
2. Activation binds the exact live target IDs before opening. A non-agent,
   missing runtime identity, active FM page, non-terminal mode, or insufficient
   screen returns a visible reason and creates no picker state.
3. Add one `Mode::AttachFile` to the existing single `OverlayLayer`. Paint the
   existing panes first, then a bounded popup using R014 `Clear`, Herdr's
   `render_panel_shell`, semantic palette roles, and responsive Miller content.
4. The overlay owns input first. Background terminal input, global shortcuts,
   pane focus, and the normal FM router do not fire. Escape cancels only this
   picker and returns to the exact still-valid target; target loss exits to the
   normal current focus without manufacturing identity.
5. The picker uses one-column Miller content below 25 cells, current+preview at
   25–37, and parent+current+preview from 38, reusing the existing 12-cell
   column and one-cell divider contract. It declines when the available screen
   is smaller than 18 columns by 10 rows: after the existing four-column/two-row
   popup margins and two-cell border, that is the smallest complete 12-column
   content surface with bounded header/list/footer. It reports
   `attachment picker needs more terminal space`.
6. Enter on a directory navigates. Enter on one supported regular file prepares
   one request. No checkbox/multi-select gestures are active in picker mode.
7. A successful send closes the overlay. Explicit cancel sends nothing. Busy
   consumes the one attempt, keeps the picker open, shows a recoverable error,
   and never hot-retries. Target loss/runtime loss/stale file/non-UTF-8 closes
   the invalid pending request and shows a visible failure.

### Source and reuse classification

- **R014 Ratatui** — direct API/reference: `Clear`, `Block`, `Paragraph`,
  `Rect`, `Constraint`, and centered bounded popup draw order. Low adaptation;
  these primitives and `Clear` are already dependencies/in use.
- **Existing Herdr symbols** — direct reuse: `OverlayLayer`, `Mode`,
  `centered_popup_rect`, `render_panel_shell`, `modal_stack_areas`,
  `miller_layout`, `PaneInfo::{rect,inner_rect,borders}`, C5's scheduled
  revalidation and `try_send_terminal_input`.
- **R033 hypertile** — architecture/algorithm reference only. Its bounded
  palette geometry supports the lifecycle choice, but no code or dependency is
  needed.
- **R061 TUI Studio** — authoring/reference only and rejected for runtime reuse;
  its DOM/CSS event model does not fit Herdr's Ratatui ownership.

### Lower-ranked alternative

Open the existing full native-FM `CenterContent` in an “attach mode” and reuse
all current input/render code unchanged. It is simpler internally, but ranks
lower because it hides the agent terminal and directly violates the stated
“without going to the FM page / without leaving the flow” product requirement.

Native OS file dialogs and drag-only delivery are also rejected for v1: they
are not uniformly available in terminal/remote sessions and would not provide
keyboard parity or deterministic cross-platform test authority.

## Delivery and Resource Budgets

| Resource | M1 v1 bound | Failure behavior |
|---|---:|---|
| Files per invocation | exactly 1 | zero, directory, or multi-selection cannot prepare a request |
| Encoded payload | at most 1 MiB including trailing CR, aligned with `MAX_INPUT_PAYLOAD` | larger path is rejected before runtime send; no truncation |
| Pending delivery | 1 | second confirm is consumed while pending |
| Runtime send attempts | 1 per explicit confirm | full/busy channel is visible and never automatically retried |
| Picker state | 1 | repeated open while active is idempotent/consumed |
| Watchers/workers/tasks | 0 new | no background lane or teardown race exists |
| Created panes/agents/processes | 0 | existing resources are never removed or modified beyond the explicit input send |

Paths with spaces, quotes, Unicode, and Windows separators are bytes of the
exact UTF-8 path, never shell text. Unix non-UTF-8 paths are visible but disabled
with an explicit reason. No lossy conversion, quoting, escaping, or partial
delivery is allowed.

## Exact RED Plan

Production Rust remains forbidden until the matching test is observed failing
for the intended missing behavior.

### M1.1 — Action and geometry

1. `focused_agent_attachment_action_is_exact_agent_only_and_responsive`
   - Expected RED: no action model/area exists.
   - GREEN: exactly one focused agent gets `[+]`; non-agent/unfocused/borderless/
     narrow/mobile-zero geometry has no stale hit target; `inner_rect` is never
     covered.
2. `default_binds_prefix_a_to_agent_attachment_picker_without_conflict`
   - Expected RED: the key action/config field does not exist.
   - GREEN: `prefix+a` resolves only to M1 and user conflict handling follows
     the existing binding registry.
3. `agent_attachment_action_render_is_bounded_ascii_and_no_color_safe`
   - Expected RED: no rendered `[+]` exists.
   - GREEN: focused/disabled semantic styles fit the exact area without
     overwriting corners, terminal cells, or adjacent border content.

### M1.2 — Picker ownership and input isolation

1. `opening_attachment_picker_binds_exact_target_and_workspace_cwd`
2. `attachment_picker_clear_overlay_is_responsive_and_blocks_background_input`
3. `attachment_picker_tiny_area_declines_with_visible_reason`
4. `attachment_picker_escape_restores_valid_focus_without_delivery`
5. `attachment_picker_accepts_one_regular_file_and_disables_other_targets`

Expected RED is the absence of picker state/mode/routes. GREEN must prove exact
target IDs, one private FM state, R014 Clear-first rendering, one/two/three
column degradation, mouse/keyboard parity, no background input, cancel with
zero request, and no directory/multi/non-UTF-8 authority.

### M1.3 — Scheduled literal delivery

1. `attachment_picker_enter_prepares_one_typed_request_without_delivery`
2. `attachment_delivery_sends_one_literal_path_and_closes_on_success`
3. `attachment_delivery_rejects_lost_agent_and_vanished_file`
4. `attachment_delivery_rejects_changed_focus_and_missing_runtime`
5. `attachment_delivery_backpressure_is_visible_without_hot_retry`
6. `attachment_payload_rejects_more_than_one_mib_including_enter`
7. `attachment_payload_rejects_non_utf8_path`

Expected RED is missing typed intent/shared C5 seam. GREEN must show no send in
input/render, one exact payload plus CR at the scheduled boundary, fail-closed
TOCTOU behavior, finite size, and zero duplicate sends.

### M1.4 — Lifecycle and complete gate

1. `attachment_picker_target_exit_or_replacement_cannot_retarget_delivery`
2. `attachment_picker_close_reopen_clears_pending_and_stale_geometry`
3. `attachment_picker_success_cancel_and_failure_preserve_existing_resources`
4. Exact M1 tests, all FM/input/render/agent tests, full nextest, Linux and
   Windows clippy, fmt, Bun, Python maintenance, diff/unwrap/debug-marker checks,
   ignored-test inventory, graph refresh and current-symbol freshness proof.

## Terminating Decision

M1 production is **complete** only for this exact existing-agent single-file
overlay flow. Its atomic TDD chain is `948ccf8` → `88f6afa` → `10eb4a4` →
`53038fd` → `cffc802` → `b6b4121` → `7d3144e`. Exact attachment tests are
20/20; full nextest is 3197/3197 with only the named B0 probe skipped; Linux
and Windows MSVC clippy, Bun 17/17, Python 64/64, fmt/diff checks, and graph
freshness at 19,113 nodes / 91,118 edges are clean.
Multi-file messages, new-agent creation, byte upload, drag/drop, watcher reuse,
generic component/page abstractions, and protocol changes remain independent
NO-GO items unless new evidence and test contracts activate them.
