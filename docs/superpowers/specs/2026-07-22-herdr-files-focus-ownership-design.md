# Herdr Files Focus Ownership and Active Cursor Design

**Date:** 2026-07-22
**Program:** FFO — Files Focus Ownership
**Status:** Approved on 2026-07-22; code-level implementation plan pending review
**Scope:** Native Files client-side focus, input authority, action authority, and visual projection
**Selected approach:** One Files region focus owner, with a separate active Miller column inside the Trail

## 1. Decision

Native Files will expose one truthful focus owner across its two navigation
regions:

1. **Locations Rail** — the fixed Home, Desktop, Downloads, and configured
   location list.
2. **Miller Trail** — the dynamic root, ancestor, current, child, and detail
   area to the right.

The most recent accepted, meaningful keyboard or mouse interaction selects
exactly one of those owners. Directional keyboard input, active-row styling,
and the File Action Bar derive from that same owner.

The existing FileManagerLocationsFocus enum remains the top-level Files focus
authority. TrailState::active_col remains the subordinate authority that
selects one Miller column only while the Miller Trail owns top-level focus.
Resident preview depth, accepted location origin, multi-selection, hover, and
painted styles are not focus authority.

When the Locations Rail owns focus, the File Action Bar actions Copy, Paste,
New Folder, and Delete are all disabled. They become eligible again only after
the Miller Trail becomes the current focus owner, and dispatch must revalidate
that owner from current AppState before performing any action.

Exactly one valid active cursor row may use the strong filled-blue focus style.
The inactive accepted-location origin remains visible as subdued context but
must not use a filled row or underline.

## 2. Terminology

| Product term | Meaning | Current implementation owner |
|---|---|---|
| Locations Rail | Fixed Files-local location list | FileManagerLocationsState plus FileManagerLocationsView |
| Miller Trail | Dynamic Miller columns and detail body | FmState, TrailState, TrailSnapshots, TrailViewSnapshot |
| Active Miller Column | The Miller column that receives vertical navigation | TrailState::active_col |
| Focus Owner | The one Files region receiving directional input | FileManagerLocationsFocus |
| Focus Cursor | The valid row painted with the strong filled-blue style | Rail cursor or selected entry in Active Miller Column |
| Accepted Origin | The location represented by the last accepted Trail root | FileManagerLocationOrigin |
| Origin Marker | Subdued Rail indication of Accepted Origin | FileManagerLocationsState::highlighted_path |
| File Action Bar | Header Copy, Paste, New Folder, Delete controls | FileManagerActionBarModel |
| Detail/Preview Pane | Prepared metadata or content associated with Trail state | TrailSnapshots detail projection |

These names are the preferred vocabulary for bug reports, tests, PRDs, code
comments, and future Files interaction work.

## 3. Problem Statement

### 3.1 Mouse-to-keyboard ownership defect

The current wide-layout sequence is:

~~~text
click Locations Rail row
  -> FileManagerLocationsFocus = Rail
  -> Rail cursor/follow request changes

click live Miller Trail row
  -> FmState cursor/selection changes
  -> FileManagerLocationsFocus remains Rail

press Up or Down
  -> handle_locations_rail_key wins the owner-first branch
  -> Rail cursor moves instead of the visible Miller selection
~~~

The row click is exact-path and visually changes Trail state, but it does not
transfer top-level focus. The model therefore contains two conflicting
signals: the Trail looks recently targeted while the Rail still owns input.

The same split exists for accepted vertical wheel movement over a live Trail
row or section header. The reducer can change TrailState::active_col and the
Trail cursor without changing FileManagerLocationsFocus.

### 3.2 Visual ownership defect

The existing renderer deliberately uses:

- Rail focused cursor: accent plus bold and reversed;
- inactive accepted origin: accent plus bold and underlined;
- Trail cursor: a different surface-color cursor style, only when Trail owns
  focus.

The underline is therefore not a terminal corruption. It is an old
accepted-origin design that now conflicts with the approved single-owner UX.
It resembles a broken or collapsed highlight and does not transfer the strong
focus signal to the newly active Miller row.

### 3.3 Header action authority defect

FileManagerActionBarModel is currently computed from FmState selection and
operation state without considering FileManagerLocationsFocus. A Rail-focused
screen can therefore continue to display or dispatch actions derived from an
older Trail selection.

This is not only visual inconsistency. Copy or Delete must never gain authority
from a hidden or inactive owner. Header paint state and dispatch-time
authorization must both reject actions while the Rail owns focus.

## 4. Evidence and Root-Cause Decision

### 4.1 Fresh graph evidence

Codebase Memory was refreshed against local HEAD b4ac62a0 and reports 24,217
nodes and 128,975 edges. Freshness was proven by resolving the
flf_scale_locations_follow_navigation symbol introduced at that HEAD. Ready
status alone was not used as freshness evidence.

The current dependency chain resolves these production symbols:

- HeadlessServer::handle_client_input_events
- App::route_client_events
- App::handle_mouse_without_agent_frame_action
- App::handle_file_manager_mouse_at
- App::handle_file_manager_row_mouse
- handle_file_manager_key
- handle_locations_rail_key
- FileManagerLocationsState::focus_trail
- render_file_manager_locations
- render_trail_view
- compute_file_manager_action_bar_model
- dispatch_file_manager_header_action
- current_action_paths

### 4.2 Baseline test evidence

The current four-test baseline passed 4/4 under Nextest, run
35c726f8-d299-4997-b100-7ecdb7beac06:

- flf_mouse_location_click_synchronizes_cursor_and_typed_intent
- single_click_selects_current_row_and_refreshes_preview
- flf_render_rail_focus_suppresses_trail_cursor_style
- flf_render_rail_cursor_wins_and_origin_remains_subdued

These tests confirm the current contract but do not assert a Rail-to-Trail
mouse focus transfer. Two visual tests explicitly accept the now-rejected
underline and region-specific cursor styles.

### 4.3 Hypothesis results

| Hypothesis | Result | Evidence | Confidence |
|---|---|---|---|
| H1: Trail mouse and wheel paths mutate subordinate Trail state without transferring the top-level owner | Confirmed | handle_file_manager_row_mouse and handle_file_manager_vertical_wheel omit FileManagerLocationsState::focus_trail | High |
| H2: stale or incorrect hit geometry routes the click to the wrong region | Rejected for the reported reproduction | current-frame generation checks, typed TrailRowView identity, exact trail index, entry index, and path validation all precede activation | High |
| H3: terminal rendering corrupts a full highlight into a line | Rejected | render_file_manager_locations intentionally applies the UNDERLINED modifier to inactive accepted origin | Certain |
| H4: Header Action Bar can retain inactive Trail authority while Rail owns focus | Confirmed | model construction and current_action_paths do not check FileManagerLocationsFocus | High |

## 5. Regional Architecture

### 5.1 Server and transport boundary

HeadlessServer::handle_client_input_events owns client event intake, foreground
client promotion, redraw scheduling, and forwarding into App. It must remain
unaware of Rail, Trail, cursor styling, or File Action Bar focus.

No server state, socket message, protocol version, shared runtime field, or
platform API changes are authorized by FFO.

### 5.2 App event routing

App::route_client_events owns ordered key/mouse delivery and render-needed
aggregation. App::handle_mouse_without_agent_frame_action owns overlay-first
mouse priority and routes Native Files events before background panes.

These layers may observe whether an accepted Files event changed visible state,
but they must not derive a focus owner from coordinates independently. Focus
transfer belongs to the typed Files input reducer after current-frame
validation.

### 5.3 Files focus authority

FileManagerLocationsState owns the top-level FileManagerLocationsFocus value.
The state should expose named transitions rather than scattered direct field
writes:

- focus_rail: makes the Rail the owner and preserves its cursor/origin laws;
- focus_trail: retires incompatible pending Rail intent and makes Trail the
  owner;
- owner predicates used by input, render, and action preparation.

The implementation plan may refine names, but it must not introduce a second
top-level focus enum or infer ownership from painted cells.

### 5.4 Rail navigation authority

The Rail owns:

- exact accessible location cursor;
- accepted location origin;
- pending FollowPreview or EnterTrail intent;
- Rail scroll;
- compact drawer lifecycle.

Rail preview completion may update the accepted Trail while retaining Rail
focus. Only an accepted EnterTrail transition may transfer focus from Rail to
Trail asynchronously. A stale, failed, replaced, closed, or model-mismatched
completion cannot change focus.

### 5.5 Trail navigation authority

FmState and TrailState own:

- resident Miller column chain;
- Active Miller Column;
- exact cursor override;
- activated branch selection;
- explicit multi-selection;
- prepared child/detail state.

TrailState::active_col is meaningful only inside the top-level Trail owner. A
resident or deeper prepared column never grants focus by itself.

### 5.6 Pure view and render projection

compute_view continues to prepare geometry and current-frame typed targets.
render_file_manager_locations and render_trail_view remain read-only
projections of AppState.

Both renderers consume the same semantic active-focus style. They do not
mutate focus, normalize cursors, start I/O, or repair stale state.

### 5.7 File Action Bar authority

File Action Bar preparation receives or reads the current top-level focus
owner:

- Rail owner: all four actions disabled with a typed inactive-owner reason;
- Trail owner: existing selection, clipboard, writability, operation, and
  supported-kind checks apply.

Dispatch revalidates the same rule against current AppState. A previously
painted enabled action cannot survive a focus change, stale frame, operation
start, selection change, close/reopen, or Files generation change.

## 6. Dependency and Event Chains

### 6.1 Mouse

~~~text
HeadlessServer::handle_client_input_events
  -> App::route_client_events
  -> App::handle_mouse_event_headless
  -> App::handle_mouse_without_agent_frame_action
  -> overlay and surface ownership gates
  -> App::handle_file_manager_mouse_at
  -> current Files/Locations/Trail generation validation
  -> typed region or row hit
  -> focus-owner transition
  -> exact Rail or Trail mutation
  -> visible-state render decision
~~~

### 6.2 Keyboard

~~~text
RawInputEvent::Key
  -> App::handle_key_headless
  -> focused Native Files route
  -> handle_file_manager_key
  -> top-level owner branch
       Rail  -> handle_locations_rail_key
       Trail -> Active Miller Column reducer
  -> optional bounded preview/activation request
  -> visible-state render decision
~~~

### 6.3 Rendering

~~~text
compute_view
  -> current Files/Locations/Trail typed geometry
  -> File Action Bar prepared authority

render_file_manager
  -> render_file_manager_locations
  -> render_trail_view
  -> render File Action Bar and status
~~~

### 6.4 Header actions

~~~text
current focus owner + Trail selection + clipboard + writable state + operation
  -> compute FileManagerActionBarModel
  -> render enabled/disabled controls
  -> header click
  -> dispatch-time recomputation
  -> exact typed operation intent or fail closed
~~~

## 7. Focus State Machine

### 7.1 Rail to Trail

A current, accepted interaction transfers Rail to Trail when any of these
occurs:

- primary click on a live Trail row;
- Ctrl-click or Shift-click on a live Trail row;
- right-click on a live Trail row before opening its context menu;
- click on a current row-local action target, aligned to that exact row;
- accepted vertical wheel input over a live Trail row or section header;
- primary click inside the live Trail body, including empty column or detail
  space, when no higher-priority resize/action target owns the event;
- accepted horizontal Trail scroll;
- keyboard Right or Enter after an accepted Rail EnterTrail transition.

The transfer itself is a visible state change even if the subordinate Trail
cursor is already at a boundary. An event rejected as stale, blocked,
coalesced, malformed, or outside the live Trail does not transfer focus.

### 7.2 Trail to Rail

Trail transfers to Rail through:

- Left from the root Miller column;
- opening the compact Locations drawer;
- a live accessible Rail row click;
- accepted Rail-local wheel or cursor interaction.

The exact Location versus Direct-origin cursor rules from FLF remain intact.

### 7.3 Events that do not change owner

- pointer movement without an accepted hover-sensitive control;
- divider resize begin/drag/end;
- stale rows or stale geometry;
- separator clicks;
- clicks outside Native Files;
- blocked background input under an overlay;
- clamped/coalesced duplicate input that causes no focus or model change;
- asynchronous preview completion without EnterTrail intent;
- filesystem watcher or preview refresh.

## 8. Visual Contract

### 8.1 Active focus cursor

Rail and Trail use one semantic active-focus cursor style based on the existing
Rail affordance:

- accent-backed filled row;
- readable foreground against accent;
- bold;
- reversed modifier retained as a no-color semantic fallback where applicable.

The exact palette composition is centralized so the two renderers cannot
drift. Entry kind coloring, multi-selection, accepted origin, pending state,
and failure state never override the active focus cursor.

### 8.2 Accepted origin

When the accepted origin differs from the active Rail cursor or the Trail owns
focus, the Rail may retain subdued context:

- accent foreground;
- bold;
- no reversed fill;
- no underline;
- no second active-looking row.

If the Rail cursor and accepted origin are the same, the active focus cursor
wins. Direct(path) continues to invent no Rail marker.

### 8.3 Trail and multi-selection

Only the selected row in Active Miller Column receives the active focus cursor
while Trail owns focus. Resident ancestor selections may remain structurally
selected for branch context but do not receive the filled focus style.

Multi-selection remains a separate collection and style. When the focused row
is also multi-selected, active focus styling wins while the collection
authority remains unchanged.

### 8.4 Empty destinations

An empty or failed active destination has no fabricated row and therefore no
filled cursor. Region focus remains Trail state, but the renderer must not
paint a fake selection merely to satisfy a visual count.

## 9. Header Action Contract

| Focus owner | Copy | Paste | New Folder | Delete |
|---|---|---|---|---|
| Locations Rail | Disabled | Disabled | Disabled | Disabled |
| Miller Trail | Existing exact selection rules | Existing clipboard/cwd rules | Existing unsupported state until separately implemented | Existing exact selection and writable rules |

Required safeguards:

1. Model preparation exposes a typed inactive-focus reason.
2. Header paint uses that prepared reason.
3. Mouse dispatch checks the prepared current-frame action state.
4. Operation dispatch independently recomputes current focus and action
   authority.
5. Focus change after compute_view but before click fails closed.
6. No Rail path is silently reinterpreted as a deletable/copyable Trail
   selection.
7. Clicking a disabled header control is consumed without focus, selection,
   clipboard, worker, filesystem, or render-authority mutation.

## 10. Performance and Resource Contract

FFO is a client presentation/input correction:

- zero filesystem enumeration on focus transfer;
- zero new worker, channel, timer, cache, dependency, or protocol field;
- O(1) top-level owner transition;
- existing exact row lookup and bounded Trail/Rail projection remain;
- one accepted focus change may request one semantic render;
- inert, stale, blocked, and coalesced events do not create a render storm;
- no pointer-hover focus model is introduced.

The existing Yazi transfer law remains: cursor movement, speculative preview,
and explicit activation have separate authority. FFO adds the missing region
owner transition; it does not change the bounded I/O or cache architecture.

## 11. Failure, Race, and Lifecycle Contract

- A stale Trail row cannot transfer focus.
- A stale Rail row cannot transfer focus.
- An old root or child preview completion cannot steal focus.
- A worker failure, panic, or disconnect cannot change focus.
- Close/reopen resets to the established initial Trail owner and rejects old
  generations.
- Compact drawer close restores its captured prior focus according to the
  existing lifecycle.
- Resize may change visible geometry but cannot invent ownership.
- Overlay entry blocks all background focus transfer.
- A File Action Bar dispatch revalidates focus after every relevant lifecycle
  transition.
- Failure paths preserve last accepted Trail/origin and never authorize
  filesystem operations from inactive visual state.

## 12. Test-First Verification Matrix

No production Rust change may precede an observed behavior-specific RED.

| ID | Test point | Expected result | Reason |
|---|---|---|---|
| TP-FFO-CHAR-01 | Current Rail click followed by Trail click | Existing baseline proves Trail cursor changes while top-level owner incorrectly remains Rail | Locks the actual defect |
| TP-FFO-MOUSE-01 | Primary live Trail row click from Rail | Trail becomes owner, exact row remains selected, next Up/Down moves only that Miller column | Core reported behavior |
| TP-FFO-MOUSE-02 | Live empty Trail/detail body click from Rail | Trail becomes owner without changing cursor, selection, preview, or I/O | Region focus must not depend on hitting text |
| TP-FFO-MOUSE-03 | Ctrl, Shift, and right click on a live Trail row | Trail owns focus and existing exact selection/context semantics remain | Mouse parity without selection regression |
| TP-FFO-MOUSE-04 | Current row-action click | Trail owns focus and cursor aligns to the exact action row before typed dispatch | Last meaningful object remains truthful |
| TP-FFO-WHEEL-01 | First accepted Trail row/header wheel event from Rail | Trail owns focus and moves at most one row; clamped movement still preserves the accepted owner transfer | Wheel and keyboard ownership parity |
| TP-FFO-WHEEL-02 | Coalesced duplicate, stale row, outside, separator, or overlay | No owner, cursor, worker, or render mutation | Fail-closed and render-storm protection |
| TP-FFO-KEY-01 | Left/Right across Rail and resident Trail columns | Exactly one owner/column edge per event; no implicit activation on file | Preserves FMH/FLF laws |
| TP-FFO-ACTION-01 | Rail-focused prepared File Action Bar | All four actions disabled with typed inactive-owner reason | Prevents stale hidden authority |
| TP-FFO-ACTION-02 | Trail focus restored | Eligible actions recompute from the exact current Trail selection | Restores intended behavior |
| TP-FFO-ACTION-03 | Focus changes after header geometry/model preparation | Mouse and direct dispatch both fail closed with zero operation side effects | Protects current-state authority |
| TP-FFO-VIS-01 | Rail focused with distinct accepted origin | Exactly one filled-blue Rail cursor; origin is accent/bold with no reverse or underline | Removes inconsistent underline |
| TP-FFO-VIS-02 | Trail focused | Exactly one filled-blue row in Active Miller Column; Rail has no active-looking row | Truthful single owner |
| TP-FFO-VIS-03 | Focused row is multi-selected | Focus style wins while explicit multi-selection membership remains | Keeps cursor and bulk authority separate |
| TP-FFO-VIS-04 | Color and no-color cell semantics | Focus, origin, inactive row, and multi-selection remain distinguishable by modifiers as well as color | Accessibility |
| TP-FFO-LIFE-01 | Compact drawer, resize, close/reopen | Prior owner restoration and generation rejection remain deterministic | Responsive lifecycle parity |
| TP-FFO-ASYNC-01 | Blocked/stale/failing root and child preview | No completion steals the current owner; last accepted state remains | Race safety |
| TP-FFO-RENDER-01 | Accepted versus inert focus events | Visible focus transition renders; stale, duplicate, and already-owned no-op events decline extra render | Performance neutrality |
| TP-FFO-IO-01 | Focus transfer under instrumented reader | Zero filesystem reads and zero new worker submissions | Input-loop safety |
| TP-FFO-E2E-01 | Isolated live mixed mouse/keyboard sequence | Last accepted Rail or Trail target owns arrows, active blue cursor, and Header Action Bar; zero residue | Full runtime acceptance |
| TP-FFO-GATE-01 | Complete repository and platform gates | Focused/broad/full Nextest, fmt, Linux/Windows Clippy, maintenance, visual checks, graph freshness, and diff hygiene pass | Production closure |

## 13. Delivery Slices

The code-level implementation plan will preserve these atomic boundaries:

1. **FFO-0 — specification and characterization:** written design, dependency
   map, existing green baseline, and behavior-specific RED tests.
2. **FFO-1 — focus authority helpers:** centralize top-level owner transitions
   without changing input behavior beyond the RED target.
3. **FFO-2 — mouse and wheel transfer:** wire current live Trail interactions
   to the single owner transition and preserve exact/stale semantics.
4. **FFO-3 — Header Action Bar fail-closed authority:** prepare and dispatch
   from the current owner.
5. **FFO-4 — unified focus visual language:** central style, remove origin
   underline, preserve multi-selection and no-color distinctions.
6. **FFO-5 — deterministic visual and cross-layer verification:** add only
   spec-scoped fixtures/snapshots; never regenerate unrelated PNGs.
7. **FFO-6 — isolated acceptance and closure:** full gates, live test, docs,
   lessons, graph/ADR refresh, exact-path commits, and CyPack-only publication.

The implementation plan must name exact files, tests, expected RED reasons,
minimum GREEN changes, rollback points, and commit boundaries before Rust
editing begins.

## 14. Architectural Rules for Future Files Features

1. **One top-level owner:** Rail and Trail cannot both own directional input.
2. **Nested authority is explicit:** Trail active column is subordinate to
   Trail region ownership.
3. **Last accepted intent wins:** rejected/stale input never changes focus.
4. **Paint is not authority:** colors, underlines, resident columns, and
   previews cannot grant focus.
5. **Input changes state; render projects it:** render remains pure.
6. **Actions follow current owner:** visible enabled state and dispatch-time
   permission use the same state law.
7. **No hidden destructive authority:** inactive-region selections cannot
   authorize Copy/Delete/Paste.
8. **No hover focus:** pointer movement alone does not move keyboard ownership.
9. **No I/O for focus:** focus transfer is constant-time client state.
10. **Current-frame geometry only:** stale rows, actions, and layouts fail
    closed.
11. **Preview never steals focus:** only explicit accepted activation crosses
    hierarchy or region boundaries.
12. **One strong cursor language:** all Files regions reuse one semantic active
    focus style.
13. **Accessibility is semantic:** color is reinforced with stable modifiers;
    geometry and hit targets do not depend on style.
14. **Render for visible change:** an input event is not automatically a render
    reason.
15. **No global focus god object:** extend the bounded Files owner seam rather
    than coupling unrelated app surfaces.

## 15. Alternatives

### 15.1 Rejected: patch only plain Trail row clicks

Adding focus_trail to one click branch would leave wheel, empty Trail body,
modified click, right click, row actions, and Header Action Bar authority
inconsistent. It fixes the reported happy path but not the ownership model.

### 15.2 Selected: central Files owner with bounded interaction adapters

Reuse FileManagerLocationsFocus for Rail versus Trail and TrailState::active_col
for the nested Miller owner. Each accepted typed interaction performs one
explicit transition. Render and Header Action Bar consume the same authority.

This is the smallest complete architecture because it reuses current state,
geometry, worker, and render seams without introducing a framework or protocol
change.

### 15.3 Rejected: global application focus manager

A new generic focus tree would cross Files, shell, overlays, panes, settings,
and server/client boundaries. The current defect does not justify that blast
radius and the result would risk a new god object.

### 15.4 Rejected: Header Action Bar targets Rail paths

Locations are navigation roots, not implicit filesystem-operation selections.
Mapping Delete or Paste to Home/Desktop/Downloads would be surprising and
dangerous. Rail focus therefore disables the complete Trail-scoped action bar.

### 15.5 Rejected: header click silently focuses Trail before dispatch

Implicit focus transfer would make a control appear authorized by the Rail
while operating on a hidden Trail selection. The user must first make Trail
the visible owner.

## 16. Non-Goals

FFO does not add or change:

- global app focus architecture;
- server/runtime/protocol/socket fields;
- layout widths, breakpoints, or Miller geometry;
- file sorting, mtime groups, preview formats, watchers, or root I/O;
- cache, pinned-location pre-warm, LRU, dependency, timer, or scheduler;
- hover-follow focus;
- multi-selection membership rules;
- File Action Bar feature set;
- stable Herdr installation, process, socket, or configuration;
- unrelated Playwright PNG baselines;
- upstream GitHub issue, PR, or publication.

## 17. Safety, Git, and Completion Gates

Implementation is complete only when:

1. the written design and code-level plan are reviewed;
2. every applicable TP-FFO test point has fresh evidence;
3. the behavior-specific RED is observed before production code;
4. focused and broad owner/input/render/action tests pass;
5. full Nextest, formatting, Linux Clippy, Windows Clippy, maintenance, and
   deterministic visual gates pass;
6. the isolated throwaway-XDG live test passes and leaves zero test residue;
7. stable Herdr, inherited stable sockets, and user sessions were never
   targeted;
8. .superpowers remains untouched and unstaged;
9. exact-path staging and aligned lowercase conventional commit messages are
   used;
10. publication targets only origin/feat/native-fm on the CyPack fork;
11. Codebase Memory is refreshed and current FFO symbols are queried;
12. the approved focus decision is stored in the Codebase Memory ADR after the
    implementation architecture is final;
13. local and remote SHAs are explicitly verified after authorized
    publication.

## 18. Approval Record

The user approved the complete proposed focus plan and specifically approved
the remaining product decision on 2026-07-22:

> While the Locations Rail owns focus, Copy, Paste, New Folder, and Delete are
> all disabled. After the Miller Trail becomes focused, they operate only on
> the current Trail authority.

The written design is approved. Rust implementation remains blocked until the
subsequent code-level TDD plan is written and approved.
