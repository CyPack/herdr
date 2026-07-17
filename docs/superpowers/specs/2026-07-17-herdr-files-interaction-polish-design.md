# Herdr Files Interaction Polish Design

## Status

- Design date: 2026-07-17
- Product area: native Files Workspace / Miller file manager
- Change class: behavior correction, interaction, visual semantics, safe agent input
- User decision: approved
- Implementation status: not started by this design
- Drag-and-drop status: explicitly removed from the MVP and from this program
- Required visual verifier: Playwright Chromium
- Source branch at design time: `feat/native-fm`
- Source checkpoint at design time:
  `b7d4217c441c0cf842e5775ff2556d641c5a7940`
- Canonical code graph at design time: 21,078 nodes / 98,023 edges
- Stable runtime safety: this design authorizes no interaction with the installed
  Herdr process or inherited stable socket

## Executive Decision

This program is a focused production correction on top of the completed native
Files Workspace. It does not reopen the shell-foundation, Miller viewport, or
drag-resize architecture.

The program delivers four user-visible outcomes:

1. The visible `Files` navigation target opens or reuses Native Files Stage on
   the first primary click. Switching to `Spaces` or `Projects` leaves Files
   Stage and restores the terminal Stage without destroying terminal runtime
   state.
2. After entering a child directory, the exact child remains highlighted in
   its previous Miller column. The highlight is bound to stable path identity,
   not to a stale row index or the default index `0`.
3. Every visible file row carries a prepared semantic icon for directories,
   symlinks, common file classes, and unsupported/broken targets. Rendering
   performs no filesystem metadata lookup.
4. The existing `Send to Agent` surface becomes the truthful
   `Add Reference to Agent...` action. It inserts one exact, validated file or
   directory path into an explicitly selected live agent terminal and sends no
   Enter, carriage return, line feed, submit command, or implicit chat creation.

The action reuses Herdr's existing Files row-action/context-menu pipeline,
Agents projection, typed terminal identity, bounded terminal input adapter, and
popup interaction language. It does not introduce mouse drag-and-drop.

## Why This Scope Is the Correct Level

The earlier shell and Miller work solved different load-bearing problems:
surface ownership, hidden-terminal isolation, generation-safe hits, bounded
column residency, horizontal viewport behavior, and divider capture. Those
layers are not being rebuilt here.

The current defects sit at four precise seams:

- visible navigation intent does not call the already implemented Files Stage
  activation;
- resident Miller selection state has a declared stable child field that is
  never populated;
- filesystem entry metadata is reduced before visual semantics are prepared;
- agent handoff appends `\r` and implicitly creates a Claude split when the
  focused terminal is not already an agent.

The correction therefore stays within existing architecture and removes
accidental behavior instead of adding a parallel framework.

## Scope

### In Scope

- one-click sidebar `Files` activation of Native Files Stage;
- deterministic transition from Files to terminal Stage through `Spaces` and
  `Projects`;
- exact-path resident-column focus persistence and re-resolution;
- file, directory, symlink, broken-link, special-target, and common file-class
  visual semantics;
- a Nerd Font primary icon profile and a deterministic one-cell ASCII fallback;
- row-action and right-click copy changed from `Send to Agent` to
  `Add Reference to Agent...`;
- explicit live-agent target picker built from the current Agents projection;
- exact file and directory path insertion with zero submit bytes;
- preparation-time and send-time path/target revalidation;
- deterministic Ratatui buffer fixtures rendered and screenshot-tested in
  Playwright Chromium;
- isolated real-terminal mouse and PTY-input smoke verification;
- graph refresh, full repository gates, atomic commits, and CyPack-only push.

### Explicit Non-Goals

- drag-and-drop from Files to chats, Spaces, Projects, or Agents;
- drag payloads, hover drop zones, ghost rows, cross-column drag capture, or
  filesystem move/copy via this interaction;
- automatically pressing Enter or submitting a chat;
- automatically creating a new Claude split, chat, pane, tab, or workspace;
- parsing or reading file contents for the reference action;
- MIME sniffing or filesystem reads during render;
- persisted icon preferences or a new font-management subsystem;
- a general component registry, popup-stack rewrite, or new server protocol;
- multiple-path reference insertion in this MVP;
- non-UTF-8 or control-character path encoding;
- changes to stable public docs before release review;
- touching upstream, opening a PR, or performing a release.

## Test-First Contract

No production-code slice starts until its named RED test fails for the intended
behavioral reason. A compilation error, missing fixture, unavailable browser,
or environment failure is not valid RED evidence.

Every RED is followed immediately by its matching GREEN before the next RED is
introduced. Failing RED commits may exist locally for audit history but are
never pushed as a remote branch head. No implementation phase is complete
until its focused tests and all previously closed tests are green.

### Test Oracle Hierarchy

| Oracle | Proves | Does not claim |
|---|---|---|
| Pure/model unit tests | path identity, kind classification, state transitions, fail-closed rules | real terminal mouse reporting |
| Ratatui `TestBackend` cell tests | exact cells, styles, truncation, hit geometry, render purity | browser rendering or PTY delivery |
| Playwright Chromium snapshots | deterministic human-visible projection of the exact prepared Ratatui cell buffer | native TUI input semantics by itself |
| Isolated real-terminal smoke | actual mouse route and exact PTY bytes in a throwaway Herdr environment | broad model exhaustiveness |
| Full repository gates | absence of known cross-surface regression | untested product behavior |

Playwright is mandatory but is not allowed to replace Rust or PTY tests. A web
browser cannot directly own a native terminal's crossterm mouse events, so
claiming otherwise would be false assurance.

### Test Point Catalog

#### Navigation and Mouse Ownership

| ID | Test | Expected result | Why it exists |
|---|---|---|---|
| `TP-FIP-NAV-01` | primary click on visible sidebar `Files` from terminal Stage | sidebar becomes Files and exactly one Native Files Stage opens | covers the real default layout, where AppDock is absent |
| `TP-FIP-NAV-02` | click `Files` when Native Files is already open | same singleton remains active; no reset, duplicate, or terminal mutation | reactivation must be idempotent |
| `TP-FIP-NAV-03` | click `Spaces` or `Projects` while Files is open | Files closes client-locally and terminal Stage resumes with identical pane/runtime identity | supplies a visible, symmetric exit path |
| `TP-FIP-NAV-04` | modified, middle, release-only, or outside click | consumed or ignored according to existing router; no Stage transition | prevents coordinate inference and accidental activation |
| `TP-FIP-NAV-05` | Files click while top overlay/capture owns input | overlay/capture wins; no background activation | preserves SF4 input precedence |
| `TP-FIP-NAV-06` | Files sidebar path click after Stage activation | exact prepared path navigates once | proves sidebar body remains usable |
| `TP-FIP-NAV-07` | stale sidebar path or generation | inert and fail closed; current cwd and Stage remain unchanged | prevents stale geometry from becoming authority |
| `TP-FIP-NAV-08` | tiny/collapsed sidebar variants | only visible typed target acts; hidden target is inert | accessibility and responsive safety |

#### Miller Stable Focus

| ID | Test | Expected result | Why it exists |
|---|---|---|---|
| `TP-FIP-FOCUS-01` | enter a child at nonzero index | previous column highlights that exact child path, never row zero | direct regression for the reported defect |
| `TP-FIP-FOCUS-02` | descend through at least four levels | every resident ancestor highlights its exact next path segment | proves the whole chain, not one parent/current special case |
| `TP-FIP-FOCUS-03` | directory entries reorder between snapshots | highlight re-resolves by path and moves to the new index | index is not identity |
| `TP-FIP-FOCUS-04` | focused child is deleted or hidden | no unrelated row becomes highlighted; focus becomes absent or deterministic current fallback | avoids a false top-row story |
| `TP-FIP-FOCUS-05` | branch changes to an ancestor sibling | retired descendant focus/projections are removed atomically | prevents old branch identity leakage |
| `TP-FIP-FOCUS-06` | leave and revisit a resident column | exact path focus and viewport remain bounded and consistent | validates resident cache behavior |
| `TP-FIP-FOCUS-07` | focused row lies outside current viewport | viewport clamps so the exact row is visible without overflow | selection and visibility must agree |
| `TP-FIP-FOCUS-08` | empty, root, unavailable, permission-denied parent | cursor is absent and no fabricated row is highlighted | explicit non-happy-path behavior |
| `TP-FIP-FOCUS-09` | stale generation row click | consumed inert; focus path and cwd unchanged | preserves FM3 generation authority |
| `TP-FIP-FOCUS-10` | duplicate/malformed path fixture | unique-path resolver returns no authority | malformed state must fail closed |

#### Entry Kind and Icons

| ID | Test | Expected result | Why it exists |
|---|---|---|---|
| `TP-FIP-ICON-01` | regular directory | directory kind, directory icon, existing `/` affordance retained if width permits | directory remains visually and textually distinct |
| `TP-FIP-ICON-02` | regular file with no extension | generic-file icon | deterministic fallback |
| `TP-FIP-ICON-03` | symlink to file and symlink to directory | distinct link-file/link-directory kinds and icons | link identity must not be discarded |
| `TP-FIP-ICON-04` | broken symlink | broken/unsupported icon; operations and reference action disabled | fail-closed visual truth |
| `TP-FIP-ICON-05` | FIFO/socket/device or metadata failure | unsupported/special icon and no destructive/reference authority | prevents regular-file impersonation |
| `TP-FIP-ICON-06` | mixed-case common extensions | case-insensitive deterministic class | real repositories contain mixed case |
| `TP-FIP-ICON-07` | dotfiles and exact well-known names | stable override before extension matching | `.gitignore`, `Dockerfile`, and `Makefile` have no useful suffix |
| `TP-FIP-ICON-08` | long ASCII and Unicode names in narrow columns | prefix icon remains complete; name truncates by display cells; row actions do not overlap | avoids broken glyphs and hit drift |
| `TP-FIP-ICON-09` | cursor and multi-selection styles | semantic icon remains visible; cursor style wins, multi-select remains distinct | visual state hierarchy |
| `TP-FIP-ICON-10` | Nerd and ASCII profiles | each category maps to one display-cell token; fallback contains no PUA glyph | font-capability resilience |
| `TP-FIP-ICON-11` | render profiling | zero filesystem/config/process/socket reads from icon render | preserves pure render and SSH latency |
| `TP-FIP-ICON-12` | sort and file operations after kind migration | directory-first order and supported operations stay byte-for-byte equivalent | protects model-refactor behavior |
| `TP-FIP-ICON-13` | UTF-8 filename containing newline/tab/escape/control | visible label escapes the control into printable text; following rows and hit cells do not move | a legal filename must not become terminal layout/input control |

#### Add Reference to Agent

| ID | Test | Expected result | Why it exists |
|---|---|---|---|
| `TP-FIP-REF-01` | open row action or right-click action | one `Add Reference to Agent...` intent with the exact current path | both surfaces converge on one authority |
| `TP-FIP-REF-02` | picker opens with focused live agent | current chat is first and preselected; every live agent appears once | current and explicit targets are both supported |
| `TP-FIP-REF-03` | no focused agent | picker lists live Agents without creating a split/chat | removes implicit runtime mutation |
| `TP-FIP-REF-04` | select live target | request snapshots workspace, pane, terminal, and exact path identity | stable target contract |
| `TP-FIP-REF-05` | regular file delivery | PTY receives exactly `path.as_bytes()` once | the reference is inserted |
| `TP-FIP-REF-06` | directory delivery | PTY receives exactly the directory path once | directories are first-class |
| `TP-FIP-REF-07` | successful delivery payload audit | payload contains no `\r`, `\n`, Enter sequence, submit command, prefix, suffix, or implicit whitespace | absolute no-submit guarantee |
| `TP-FIP-REF-08` | vanished pane or workspace | zero bytes sent; visible bounded failure feedback | stale target fails closed |
| `TP-FIP-REF-09` | pane now maps to another terminal ID | zero bytes sent | changed terminal identity cannot inherit authority |
| `TP-FIP-REF-10` | terminal is no longer an agent or runtime unavailable | zero bytes sent; one visible failure; no retry loop | prevents wrong-terminal injection |
| `TP-FIP-REF-11` | selected path deleted before send | zero bytes sent | path authority is revalidated at the last seam |
| `TP-FIP-REF-12` | selected target changes from file/dir to special/broken | zero bytes sent | supported-kind contract is revalidated |
| `TP-FIP-REF-13` | non-UTF-8 or control-character path | action disabled or delivery rejected with zero bytes | prevents terminal control injection and lossy identity |
| `TP-FIP-REF-14` | terminal input channel is busy/full | one bounded attempt, zero hot retry, visible failure | backpressure stays bounded |
| `TP-FIP-REF-15` | picker canceled/outside-click/Escape | no request and zero bytes | explicit user intent is required |
| `TP-FIP-REF-16` | stale row or context selection | picker does not open | source path identity is not inferred from coordinates |
| `TP-FIP-REF-17` | multiple selection | action remains disabled in this MVP | prevents ambiguous insertion order |
| `TP-FIP-REF-18` | special characters and spaces without controls | exact UTF-8 bytes preserved, still no submit | paths are chat text, not shell commands |

#### Visual and End-to-End

| ID | Test | Expected result | Why it exists |
|---|---|---|---|
| `TP-FIP-VIS-01` | default terminal Stage then Files activation fixture | Files identity, Miller columns, and active sidebar state match approved snapshot | first-click visibility |
| `TP-FIP-VIS-02` | nonzero child selection in resident column | screenshot shows highlight on exact child, not first row | human-visible regression proof |
| `TP-FIP-VIS-03` | mixed icon fixture | directory, link, source, config, document, image, archive, and unsupported rows are distinguishable | validates useful icon vocabulary |
| `TP-FIP-VIS-04` | narrow and tiny viewports | no half glyph, overlap, panic, or off-screen popup | responsive production behavior |
| `TP-FIP-VIS-05` | target picker with current and other agents | exact labels, focus, disabled states, and bounded geometry | chooser usability |
| `TP-FIP-VIS-06` | target disappears while picker is open | stale row becomes disabled or activation closes with visible failure | failure is visible, not silent |
| `TP-FIP-E2E-01` | isolated terminal mouse click on visible Files | real app opens Native Files Stage; stable server/socket untouched | validates host mouse routing |
| `TP-FIP-E2E-02` | isolated PTY captures add-reference delivery | captured bytes equal exact path and contain no CR/LF | final no-submit proof |

## Current-State Evidence

### Default Files Navigation

`ShellLayout::default` contains only `LeftPanel` and `WorkspaceStage`. The
implemented AppDock is not present in the default shell tree.

`AppState::handle_mouse` handles a sidebar tab click by assigning
`self.sidebar_tab = tab` and returning. It does not call
`activate_dock_app(Files)` or `open_file_manager`.

`activate_dock_app(Files)` already opens the file manager if absent, and the
AppDock fixture test verifies that isolated route. The test does not exercise
the actual default sidebar path. The production correction is therefore route
convergence, not a second file-manager opener.

### Miller Highlight

`MillerPathSegment` declares:

- `directory`;
- `focused_child`;
- `cursor`;
- `viewport_start`;
- `preferred_width`.

`MillerPathSegment::new` sets `focused_child = None` and `cursor = 0`.
Graph-augmented source search finds no other use of `focused_child`.

The Miller projection renders:

- current column selection from `FmState.cursor`;
- prepared immediate parent selection from `FmParent.cursor`;
- resident non-current selection from `MillerPathSegment.cursor`.

`MillerState::visit` appends/truncates segments and moves a departing projection
into the resident cache, but it never binds the chosen child path or updates
the departing segment's cursor. A new resident segment therefore exposes the
default cursor `0`, exactly matching the reported previous-column highlight.

### Entry Semantics

`read_directory_snapshot` obtains symlink-aware filesystem information through
`entry_capabilities`, but reduces it to:

- `is_dir`;
- `operation_supported`.

`FileEntry` carries no symlink or visual-kind identity. `render_entry_row`
renders two spaces, the name, and an optional `/`. It has no icon classifier.

The correct insertion point is directory snapshot preparation. Render must
consume a pure semantic enum and never call `metadata`, `file_type`, `is_file`,
or `is_dir`.

### Agent Handoff

The existing row and context actions already converge on a typed
`FileManagerContextActionIntent::SendAgent`. Preparation requires exactly one
current supported path.

For an agent terminal, the current sender builds `path bytes + b'\r'`. For a
non-agent terminal, preparation creates a `FileManagerClaudeSplitRequest`.
Both are contrary to the approved reference-only behavior.

Herdr already has reusable pieces:

- exact Files row/context path validation;
- `agent_panel_entries` with stable workspace/tab/pane identities;
- terminal lookup by pane;
- agent-terminal classification;
- a bounded `try_send_terminal_input` seam;
- popup keyboard/mouse ownership and stale-selection patterns.

The new design adapts these owners rather than creating a parallel delivery
stack.

### Characterization Gate

At the design checkpoint, the following focused existing tests passed 10/10:

- sidebar tab switching;
- AppDock Files activation;
- all-column current-row mouse focus;
- directory-chain append;
- current `path + Enter` handoff behavior;
- vanished attachment target/path rejection;
- symlink-to-directory classification;
- unsupported special entry;
- long-name truncation;
- Unicode row-action isolation.

This proves the baseline is green. It also proves why the reported bugs can
coexist with green tests: the assertions encode adjacent behavior, not the
missing contracts listed in this design.

## Dimensional Analysis

| Dimension | Current reality | Target invariant | Primary proof |
|---|---|---|---|
| Product/UX | Files is implemented but its visible default tab does not open the Stage | one primary click opens/reuses Files; Spaces/Projects provide a coherent exit | `NAV-01..03`, `VIS-01`, isolated mouse |
| State ownership | Files and Miller state are client-local; terminals remain runtime-owned | no new shared/server fact; no terminal lifecycle mutation | state unit tests and runtime identity characterization |
| Identity | current rows have exact typed paths; resident highlight falls back to index `0` | path is authority for source row, focused child, and agent target | `FOCUS-01..10`, `REF-04/08/09/16` |
| Filesystem data | symlink observation is discarded after capability reduction | one canonical prepared entry kind drives capabilities and visuals | `ICON-01..07/12`, `REF-06/11/12` |
| Geometry | typed Miller row geometry exists; target-picker geometry does not | compute owns complete visible hit rows; hidden/clipped rows expose none | `NAV-04..08`, `REF-02/15`, `VIS-04..06` |
| Input routing | AppDock and sidebar navigation do not converge; handoff can mutate runtime | one navigation seam and one reference intent; overlays/capture remain higher priority | `NAV-01..07`, `REF-01..04/15` |
| Render | Files render is pure but lacks prepared icon semantics | icon and picker rendering consume prepared state only | `ICON-08..11`, Ratatui buffer tests |
| Terminal adapter | current handoff appends `\r` and may create a Claude split | one exact path-byte send, no submit, no new chat, no retry | `REF-03/05..14/18`, isolated PTY capture |
| Failure/security | several stale source checks exist; control paths and chosen-agent races are unspecified | every stale/invalid source or target sends zero bytes and produces bounded feedback | fail-closed matrix and adversarial tests |
| Performance | Miller is bounded and render is visible-state proportional | no metadata read in render; transition-only path resolution; bounded picker and send | profiling tests and budget table |
| Accessibility | directory `/` exists; icon/font fallback does not | text suffix plus one-cell ASCII profile; color is never sole semantic cue | `ICON-01/08..10`, `VIS-03/04` |
| Responsive behavior | Miller breakpoints exist; new picker/icons are unproven at tiny widths | no partial glyph, overlap, off-screen popup, or inferred hidden action | `NAV-08`, `ICON-08`, `VIS-04..06` |
| Platform | current filesystem capability code is cross-platform but link behavior varies by OS | pure classification is platform-neutral; filesystem integration is explicitly OS-gated and never silently skipped | platform matrix below and canonical Clippy gates |
| Persistence/migration | entry visuals, focus, picker, and Files open state are ephemeral | no snapshot/schema/protocol migration | snapshot compatibility and protocol-version unchanged checks |
| Dependencies/build | Rust product has no need for a new dependency; repo has no Playwright package | Playwright remains test-only with its own pinned lockfile | dependency diff audit and browser self-tests |
| Operations/rollout | stable Herdr is currently running outside this task boundary | only isolated debug runtime is used; remote sees only GREEN heads | isolated-runtime recipe, Git ancestry/SHA checks |

## Cross-Platform Contract

| Platform case | Required behavior and verification |
|---|---|
| Linux regular files/directories | real filesystem tests cover kind, sort, icon, reference, deleted-path race, FIFO/socket special cases |
| Linux symlinks | real file-link, directory-link, and broken-link tests; exact link path is inserted when its target remains supported |
| macOS | shared pure/model and compile behavior remain portable; Unix symlink semantics use the same gated tests when the macOS gate is available |
| Windows regular files/directories | pure/model tests plus canonical Windows Clippy; path separators and drive prefixes are preserved byte-for-byte as UTF-8 text |
| Windows file/directory symlinks | use platform-gated `symlink_file`/`symlink_dir` integration tests on the canonical VM; missing privilege is an explicit environment failure, not a green skip |
| Windows junction/reparse target not proven as a supported symlink | classify from the safe metadata result; if regular file/directory capability cannot be proved, show unsupported and disable reference |
| non-UTF-8 Unix path | visible behavior remains current v1 policy; reference action cannot create lossy text and is disabled/rejected |
| control-character path on every platform | visible row may exist where the platform permits it, but terminal-reference authority is denied |

## Target Architecture

### Layer 0 — Semantic Identities

#### Stable Miller Focus

`focused_child: Option<PathBuf>` becomes the canonical resident-column
selection identity. `cursor` remains a bounded derived/cache value used for
viewport math, never the authority.

One model seam performs:

```text
bind focused child path
  -> resolve unique path in exact directory snapshot
  -> update derived cursor if unique
  -> clamp viewport so resolved cursor is visible
  -> otherwise expose no resident selection
```

Before current directory entries become a departing resident projection, the
segment for that directory is bound to the child path that caused the
transition. Branch truncation retires descendant focus with the segment.

Projection re-resolves `focused_child` against the exact resident/prepared
entries for the current generation. It never displays row zero merely because
the cached index is absent or stale.

#### File Entry Kind

`FileEntry` gains one canonical semantic kind:

```rust
enum FileEntryKind {
    Directory,
    RegularFile,
    SymlinkDirectory,
    SymlinkFile,
    BrokenSymlink,
    UnsupportedSpecial,
}
```

Operational helpers derive from this enum:

- `is_directory_target()`;
- `supports_native_operation()`;
- `supports_agent_reference()`.

The existing duplicated `is_dir` and `operation_supported` fields are migrated
to derived methods in a characterized refactor. This prevents kind and
capability drift. This enum is client-local and is not persisted or added to
the server protocol.

#### Agent Reference Identity

The prepared request is explicit:

```rust
struct AgentReferenceRequest {
    path: PathBuf,
    source_files_generation: u32,
    workspace_id: WorkspaceId,
    pane_id: PaneId,
    terminal_id: TerminalId,
}
```

The exact concrete ID types follow existing Herdr definitions. No UI row index
or coordinate is stored as authority.

### Layer 1 — Filesystem Preparation

Directory reading classifies the entry from `DirEntry::file_type` and resolved
target metadata:

| Filesystem observation | Kind | Native/reference capability |
|---|---|---|
| regular directory | `Directory` | enabled |
| regular file | `RegularFile` | enabled |
| symlink resolving to directory | `SymlinkDirectory` | enabled |
| symlink resolving to regular file | `SymlinkFile` | enabled |
| broken symlink | `BrokenSymlink` | disabled |
| FIFO, socket, device, other special | `UnsupportedSpecial` | disabled |
| metadata failure that cannot prove file/dir | `UnsupportedSpecial` | disabled |

The action sends only a reference string; it does not open or read content.
Nevertheless, the same classification is repeated at the delivery seam to
close the watcher/TOCTOU window as far as the path API allows.

### Layer 2 — Pure Visual Classification

The visual classifier is a pure function of prepared kind and file name. It
uses exact-name overrides first, then a lowercase final extension.

#### Required Common Classes

| Class | Exact names/extensions |
|---|---|
| version control | `.gitignore`, `.gitattributes`, `.gitmodules` |
| build/package | `Cargo.toml`, `Cargo.lock`, `package.json`, lock files, `Dockerfile`, `Makefile`, `Justfile` |
| source code | `rs`, `c`, `h`, `cpp`, `hpp`, `go`, `py`, `rb`, `java`, `kt`, `swift`, `lua` |
| web code | `js`, `jsx`, `ts`, `tsx`, `html`, `htm`, `css`, `scss`, `sass`, `vue`, `svelte` |
| scripts/executable text | `sh`, `bash`, `zsh`, `fish`, `ps1`, `bat`, `cmd` |
| config/data | `toml`, `yaml`, `yml`, `json`, `jsonc`, `xml`, `csv`, `env`, `ini`, `conf`, `properties` |
| documents/text | `md`, `mdx`, `rst`, `txt`, `pdf`, `doc`, `docx`, `odt`, `xls`, `xlsx`, `ods`, `ppt`, `pptx` |
| images | `png`, `jpg`, `jpeg`, `gif`, `webp`, `svg`, `ico`, `bmp`, `tif`, `tiff`, `avif` |
| audio | `mp3`, `wav`, `flac`, `m4a`, `aac`, `ogg`, `opus` |
| video | `mp4`, `mkv`, `mov`, `webm`, `avi`, `m4v` |
| archives | `zip`, `tar`, `gz`, `bz2`, `xz`, `zst`, `7z`, `rar`, `tgz` |
| generic | every unmatched regular file |

Directory and symlink kind always wins over name/extension. Broken/special kind
always wins over every apparent extension.

#### Glyph Profiles

- `Nerd`: uses the same private-use icon language already present in Herdr's
  sidebar and AppDock.
- `Ascii`: uses deterministic one-cell category tokens. It is the no-font
  fallback and the canonical cross-machine Playwright fixture profile.

Both profiles must occupy exactly one Ratatui display cell. Icon plus separator
is budgeted before truncating the filename. The existing directory `/` suffix
is preserved when the remaining width permits, providing a non-icon semantic
cue.

UTF-8 entry names remain exact identity, but a prepared display label escapes
every control character into printable text before render. This prevents a
newline, tab, escape, or bidi/control code in a legal filename from changing
Ratatui layout or impersonating terminal input. The exact underlying path is
unchanged and the reference action remains disabled for control-character
paths.

### Layer 3 — Projection and Render

`compute_view` remains the only producer of actionable geometry.

- Sidebar navigation projects typed tab targets.
- Miller rows retain complete generation, column, directory, index, and exact
  path authority.
- The agent picker projects typed rows containing exact target identity.
- Hidden, clipped, stale, or zero-area items project no actionable hit.

Render remains `&AppState`-only and consumes:

- prepared Miller snapshot;
- prepared `FileEntryKind` and visual class;
- prepared target-picker model.

It performs no mutation and no filesystem, socket, terminal, process, clock,
random, or config I/O.

### Layer 4 — Input and Controller

#### Sidebar Navigation

The sidebar-tab mouse route emits a typed navigation intent instead of ending
after a visual tab assignment:

- `Files`: set Files sidebar, open/reuse Native Files Stage;
- `Spaces`: set Spaces sidebar, close Files Stage if active;
- `Projects`: set Projects sidebar, close Files Stage if active.

All three paths remain client presentation state. Closing Files does not close,
reset, respawn, refocus, or write to a terminal runtime.

#### Miller Focus

All row activation routes converge on the same stable focus transition. A
click cannot update only the current `FmState.cursor` while leaving its
departing segment unbound.

The controller resolves the typed row against current generation and exact
path, binds the source segment's focus, then executes navigation. A failure at
any step consumes the click without partial state.

#### Agent Target Picker

`Add Reference to Agent...` opens a modal picker using the existing popup
language:

- current focused live agent appears first and is marked `Current chat`;
- remaining live Agents follow in current `agent_panel_entries` order;
- the same pane is deduplicated;
- stale/unavailable targets are disabled or removed during recompute;
- keyboard, mouse hover, click, outside-click, and Escape follow existing modal
  ownership;
- background Files and terminal input is blocked while open.

Selection prepares an `AgentReferenceRequest`; it does not switch focus, open a
chat, or send bytes yet.

### Layer 5 — Runtime Adapter

The last seam revalidates all authority:

1. exact source path is still UTF-8;
2. path contains no Unicode/C0 control characters;
3. path still exists as a regular file/directory or supported symlink target;
4. workspace still exists with the same workspace ID;
5. pane still exists in that workspace;
6. pane still maps to the exact terminal ID;
7. terminal is still a live agent terminal;
8. terminal input adapter accepts one bounded send.

Payload contract:

```text
payload = exact UTF-8 path bytes
```

Forbidden payload additions:

- `\r`;
- `\n`;
- Enter key sequences;
- bracketed-paste control sequences;
- quotes or shell escaping;
- leading/trailing whitespace;
- automatic prompt text;
- retry duplicates.

The path is inserted at the agent application's current input cursor. Herdr
does not claim to understand or rewrite the agent application's editor state.
This is a chat-input reference action, not a shell command.

On failure, zero new request is queued. The adapter performs at most one send
attempt and displays one bounded failure message. There is no background retry
that could later target a different terminal.

### Layer 6 — Verification and Observability

Failures are user-visible but low-noise:

- stale source path;
- unavailable/changed target;
- unsupported path type;
- invalid path text;
- busy/unavailable terminal input.

Messages contain no file contents and no terminal buffer contents. Profiling
uses existing render/FM counters and does not label metrics with arbitrary
paths.

## Playwright Chromium Visual Oracle

### Why a Bridge Fixture Is Required

Herdr is a native Ratatui application. Playwright controls browser pages; it
does not emit native crossterm events into a terminal or inspect a Ratatui
buffer. A truthful browser visual test therefore needs a deterministic adapter.

### Selected Harness

A test-only Rust fixture exporter renders the real Files UI with Ratatui
`TestBackend` and serializes the resulting cells:

```text
width, height
cells[] = symbol + foreground + background + modifiers
semantic fixture name
```

The exporter is an ignored, explicit test and writes only to a caller-provided
temporary directory. Normal unit tests and product runtime never write visual
fixtures.

A small static browser harness under `tests/visual/` renders those cells into a
fixed CSS grid. It contains no copy of the product layout or icon classifier.
Playwright reads only the exported buffer, so visual drift must originate in
the real Ratatui projection.

### Determinism Contract

- Chromium-only Playwright project;
- Playwright version pinned in the visual-test lockfile;
- fixed Linux CI image, viewport, device scale, locale, color scheme, and
  reduced-motion setting;
- animations/transitions disabled;
- one worker in CI;
- ASCII icon profile for canonical cross-machine baselines;
- screenshot on failure and trace on first retry;
- explicit snapshot-update command; ordinary CI never updates snapshots;
- zero pixel tolerance by default; any later nonzero tolerance requires a
  documented rendering reason.

Nerd-profile mappings are asserted in Rust cell tests. This avoids a fake green
Chromium result caused by a missing private-use font rendering every icon as
the same tofu box.

### Visual Harness Self-Tests

- every exported Ratatui cell maps to exactly one browser grid position;
- wide/combining symbols cannot shift following cells;
- foreground/background/modifiers round-trip;
- missing or malformed fixture fails the test, never renders an empty page;
- changing one known cell fails the matching screenshot.

## Performance and Resource Budgets

| Operation | Required bound |
|---|---|
| icon classification | `O(name length)` once during directory snapshot preparation |
| file-row render | `O(visible rows)` with no metadata lookup |
| stable focus resolution | `O(entries in one prepared/resident directory)` only on transition/reload, never per unchanged frame |
| Miller render | existing `O(visible columns × visible rows)` bound |
| sidebar activation | `O(1)` client-state transition plus existing FM open preparation |
| agent picker projection | `O(live agent panes)` with no terminal buffer read |
| target hit testing | bounded visible typed rows |
| path delivery | one validation pass and one bounded channel send |
| retry queue | zero |
| new unbounded cache | zero |

No new dependency is added to the Rust product. Playwright is isolated in the
test harness and does not ship in Herdr binaries.

## Failure, Security, and Edge-Case Policy

### Fail-Closed Matrix

| Condition | Behavior |
|---|---|
| stale Files/sidebar hit | consume inert |
| unavailable Files cwd | render existing unavailable status; do not fall through to terminal |
| stale Miller row | consume inert |
| focused child missing | no unrelated resident highlight |
| ambiguous duplicate entry path | no selection authority |
| broken/special entry | visible but operation/reference disabled |
| no live agent | picker shows a no-target state; no split/chat is created |
| target disappears | zero bytes and visible failure |
| terminal identity changes | zero bytes and visible failure |
| path disappears or changes kind | zero bytes and visible failure |
| control/non-UTF-8 path | action disabled/rejected |
| channel busy/unavailable | one failure, no retry |
| tiny terminal | bounded layout; action may be hidden, never inferred |
| Playwright browser unavailable | visual gate fails explicitly; no skipped-success claim |

### Terminal Injection Boundary

Unix filenames may legally contain newlines and other controls. Sending those
bytes to a terminal input channel could act as input rather than text.
Reference delivery therefore rejects any path string containing a Unicode
control character. This is intentionally stricter than filesystem visibility.

Spaces and ordinary punctuation are preserved byte-for-byte because this is a
chat reference, not a shell-escaped argument.

## Task and Subtask Tree

### FIP-0 — Baseline and Visual Harness

- `FIP-0.1`: freeze current characterization tests and graph evidence;
- `FIP-0.2`: add isolated Playwright package/config/lockfile;
- `FIP-0.3`: add test-only Ratatui cell fixture exporter;
- `FIP-0.4`: add browser cell-grid renderer and harness self-tests;
- `FIP-0.5`: prove a one-cell mutation fails a screenshot;
- `FIP-0.6`: keep all artifacts inside test/target paths.

Exit: harness self-tests green and no product behavior changed.

### FIP-1 — Visible Files Navigation

- `FIP-1.1 RED`: default sidebar Files click must open Native Files Stage;
- `FIP-1.2 GREEN`: converge sidebar route on existing activation seam;
- `FIP-1.3 RED`: Spaces/Projects must restore terminal Stage without runtime
  mutation;
- `FIP-1.4 GREEN`: implement symmetric client-only transition;
- `FIP-1.5`: overlay, modifier, collapsed, stale-path, and singleton edge tests;
- `FIP-1.6`: Playwright `VIS-01` snapshot and isolated mouse smoke.

Exit: one-click open/exit works through the actual default shell.

### FIP-2 — Miller Stable Child Focus

- `FIP-2.1 RED`: nonzero child must remain highlighted in departing column;
- `FIP-2.2 GREEN`: bind `focused_child` before resident transfer;
- `FIP-2.3 RED`: reordered/deleted child must re-resolve/fail closed;
- `FIP-2.4 GREEN`: unique-path resolver and absent-selection projection;
- `FIP-2.5`: deep chain, branch retirement, viewport, unavailable/root, stale
  generation, and malformed-state tests;
- `FIP-2.6`: Playwright `VIS-02` snapshot.

Exit: no resident column can fabricate a row-zero highlight.

### FIP-3 — Semantic Entry Kinds and Icons

- `FIP-3.1`: characterize current sorting, operations, symlink, special, watcher,
  preview, and action behavior;
- `FIP-3.2 RED`: require canonical kind classification for every filesystem
  observation;
- `FIP-3.3 GREEN`: introduce `FileEntryKind` and derived capability methods;
- `FIP-3.4`: migrate call sites without dual source-of-truth fields;
- `FIP-3.5 RED`: require exact-name/extension class and glyph mappings;
- `FIP-3.6 GREEN`: implement pure visual classifier and two glyph profiles;
- `FIP-3.7`: truncation, Unicode, control escaping, selection, render-purity,
  and no-I/O tests;
- `FIP-3.8`: Playwright `VIS-03` and `VIS-04` snapshots.

Exit: icons are meaningful, deterministic, font-safe, and operational behavior
is unchanged.

### FIP-4 — Reference-Only Delivery Core

- `FIP-4.1 RED`: current sender must produce exact path bytes with no CR/LF;
- `FIP-4.2 GREEN`: replace handoff payload with reference payload;
- `FIP-4.3 RED`: directories must be accepted; special/broken targets rejected;
- `FIP-4.4 GREEN`: shared source-path validation and last-seam metadata check;
- `FIP-4.5 RED`: non-agent focus must not create a Claude split;
- `FIP-4.6 GREEN`: remove implicit split behavior for this action;
- `FIP-4.7`: vanished path, changed terminal, lost agent, control path,
  backpressure, exact-once, and no-retry tests.

Exit: the core can only insert a safe exact path into a still-identical agent
terminal and can never submit it.

### FIP-5 — Explicit Agent Target Picker

- `FIP-5.1 RED`: action opens a picker based on live Agents projection;
- `FIP-5.2 GREEN`: prepare current/other agent rows with exact identities;
- `FIP-5.3 RED`: keyboard/mouse/modal ownership and cancel paths;
- `FIP-5.4 GREEN`: reuse existing popup geometry and focus language;
- `FIP-5.5 RED`: target disappearance/identity change between open and select;
- `FIP-5.6 GREEN`: recompute and last-seam fail-closed validation;
- `FIP-5.7`: rename visible copy to `Add Reference to Agent...`;
- `FIP-5.8`: Playwright `VIS-05` and `VIS-06` snapshots.

Exit: current chat and any selected live Agent are explicit, safe targets.

### FIP-6 — Production Closure

- `FIP-6.1`: focused nextest for every `TP-FIP-*` Rust test;
- `FIP-6.2`: Playwright Chromium suite with fresh fixtures;
- `FIP-6.3`: isolated terminal mouse and PTY byte smoke;
- `FIP-6.4`: format, full nextest, maintenance scripts, Linux Clippy, canonical
  Windows Clippy, Bun, and Python gates required by current Herdr continuity;
- `FIP-6.5`: render/performance and invariant checks;
- `FIP-6.6`: refresh codebase-memory graph and re-read changed symbols;
- `FIP-6.7`: update `.codex` current/tasks/evidence and planning state;
- `FIP-6.8`: verify clean tracked worktree, exact commit chain, and CyPack-only
  fast-forward push.

Exit: no failing test, stale graph, uncommitted tracked product change, or
unverified production claim remains.

## Atomic TDD and Git Delivery

Each behavior uses separate local RED and GREEN commits. Refactors follow only
after the matching GREEN and preserve the focused gate.

Planned commit families:

```text
test: pin default files navigation
fix: open files from visible navigation

test: pin resident miller child focus
fix: preserve miller child focus by path

test: pin file entry semantic kinds
refactor: derive file capabilities from entry kind

test: pin files icon classification
feat: render semantic files icons

test: pin reference-only agent payload
fix: insert agent references without submit

test: pin files agent target picker
feat: choose agent reference targets

test: add files chromium visual oracle
docs: close files interaction polish
```

Before each commit, the exact subject is announced and the staged diff is
audited. Commits use lowercase conventional style, no emoji, and no AI
co-author lines.

Push discipline:

1. never push a failing RED head;
2. run the phase-focused gate after GREEN;
3. fetch CyPack refs and verify fast-forward ancestry;
4. push only the authorized CyPack feature/master refs;
5. never push upstream;
6. after final push, verify remote SHAs equal local HEAD.

The user-owned untracked `.superpowers/` directory is excluded from every
stage, commit, and cleanup operation.

## Rollout and Rollback

These changes are client-local and require no persisted-state or protocol
migration.

Rollback is commit-granular:

- navigation route can revert independently;
- stable focus can revert independently without changing cache bounds;
- icon model migration reverts as one characterized unit;
- reference-only delivery and target picker are separate slices;
- Playwright harness is test-only.

No feature flag is required because each correction replaces demonstrably
incorrect or misleading behavior. If a phase fails its exit gate, it is not
pushed and the last remote GREEN remains deployable.

## Acceptance Criteria

The program is complete only when all statements below have fresh evidence:

- clicking the visible default `Files` target opens Native Files Stage;
- switching to Spaces/Projects restores terminal Stage without runtime loss;
- entering a non-first child highlights that exact child in the previous
  Miller column;
- reorder/delete/branch-change cannot highlight an unrelated row;
- file, directory, symlink, common file class, broken link, and special targets
  have truthful icons and fallback tokens;
- icon rendering performs no filesystem I/O;
- `Add Reference to Agent...` supports exactly one file or directory;
- current chat and another selected live Agent are supported;
- vanished pane, changed terminal identity, lost agent, deleted path, invalid
  path, unsupported target, and backpressure all send zero bytes;
- successful payload equals exact UTF-8 path bytes and contains no Enter,
  carriage return, line feed, or auto-submit behavior;
- no implicit split/chat is created;
- Playwright Chromium visual snapshots pass in the pinned environment;
- isolated native mouse and PTY byte smokes pass without touching stable Herdr;
- full repository gates pass;
- refreshed codebase graph resolves the new owners;
- tracked worktree is clean and authorized remote refs match the final GREEN
  commit.

## Design Self-Review Checklist

- one owner exists for each state transition;
- render remains pure;
- filesystem and runtime boundaries remain explicit;
- stable identities replace coordinate/index authority;
- every failure path is fail closed;
- no drag-and-drop remains in scope;
- no submit byte can be generated by the new action;
- Playwright's role is truthful and bounded;
- no server protocol or persisted-state expansion is hidden in the design;
- performance scales with visible/prepared state;
- every requirement maps to a named test point and task;
- no implementation completion is claimed by this document.
