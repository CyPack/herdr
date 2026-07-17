# FM5 Native Files Preview Placement Decision

Date: 2026-07-17

Branch: `feat/native-fm`

Measured repository head: `059612b4`

Unchanged product-code head: `d7997d0d`

Decision: **NO-GO for Shell `RightPanel` and adaptive hybrid product work.
Keep the existing inline final Miller preview column.**

## Decision Boundary

FM5 is evidence-only. No Rust production file, protocol, persistence schema,
runtime authority, or preview worker behavior changed. The local measurement
helper was added only long enough to execute the pure shell/Miller projection,
then removed byte-for-byte before this record was written.

The decision compares:

- A — current inline final Miller preview;
- B — a local pure-geometry prototype adding a 32-cell `RightPanel` to the
  current `LeftPanel | WorkspaceStage` split and removing inline preview from
  the Miller chain;
- C — a breakpoint-based switch between A and B.

The B/C numbers are geometry prototypes, not claims that unimplemented
RightPanel render, focus, persistence, or transport behavior is production
measured.

## Architecture Facts

Fresh graph/source inspection proves:

- production uses the compatibility `ShellLayout::default()` with
  `LeftPanel | WorkspaceStage`;
- `DesktopWorkspace` defines a `RightPanel`, but it is explicitly
  `Collapsed { restore: 32 }`;
- `InspectorWorkspace` exists only as a closed template; no Native Files
  preview consumer, focus owner, hit projection, or persistence authority is
  attached to its RightPanel;
- current image/text placement comes from
  `MillerViewSnapshot::preview_content_rect()`, which requires exact preview
  generation and selected-path equality;
- render, hit-test, and image geometry therefore share one live snapshot
  authority in A.

AppDock expanded/collapsed is not a live production placement dimension under
the current compatibility shell. Treating it as measured product geometry
would be false. Activating a dock-bearing template remains separate product
scope; the FM5 decision does not smuggle it in.

## Raw Geometry Measurements

The exact temporary measurement passed 1/1 and wrote
`/tmp/herdr-fm5-measure.38uEtH.log` (113 lines, SHA-256
`07d79ea4a337b90d4113451f891c24e7f0ec71c814f5522961b773f603b80f20`).
Inputs:

- terminal sizes: 80x24, 100x30, 120x40, 160x50, 240x80;
- path depths: 1, 3, 5, 12, 32;
- LeftPanel: expanded 26 cells and collapsed 0 cells;
- Miller min/preferred/max: 16/28/64; divider: 1;
- B prototype RightPanel: 32 cells;
- A Files body removes header/status; preview content removes its title row.

### Expanded LeftPanel

`columns` includes A's inline preview but only B's directory columns.
Depth groups with identical results are compressed without dropping a case.

| Terminal | Depth | A Stage | A columns | A preview | B Stage | B nav columns | B preview |
|---|---:|---:|---:|---:|---:|---:|---:|
| 80x24 | 1/3/5/12/32 | 54 | 2 | 25x21 | 22 | 1 | 32x23 |
| 100x30 | 1/3/5/12/32 | 74 | 2 | 28x27 | 42 | 1 | 32x29 |
| 120x40 | 1 | 94 | 2 | 28x37 | 62 | 1 | 32x39 |
| 120x40 | 3/5/12/32 | 94 | 3 | 28x37 | 62 | 2 | 32x39 |
| 160x50 | 1 | 134 | 2 | 28x47 | 102 | 1 | 32x49 |
| 160x50 | 3/5/12/32 | 134 | 4 | 28x47 | 102 | 3 | 32x49 |
| 240x80 | 1 | 214 | 2 | 28x77 | 182 | 1 | 32x79 |
| 240x80 | 3 | 214 | 4 | 28x77 | 182 | 3 | 32x79 |
| 240x80 | 5/12/32 | 214 | 5 | 28x77 | 182 | 5 | 32x79 |

### Collapsed LeftPanel

| Terminal | Depth | A Stage | A columns | A preview | B Stage | B nav columns | B preview |
|---|---:|---:|---:|---:|---:|---:|---:|
| 80x24 | 1/3/5/12/32 | 80 | 2 | 28x21 | 48 | 1 | 32x23 |
| 100x30 | 1 | 100 | 2 | 28x27 | 68 | 1 | 32x29 |
| 100x30 | 3/5/12/32 | 100 | 3 | 28x27 | 68 | 2 | 32x29 |
| 120x40 | 1 | 120 | 2 | 28x37 | 88 | 1 | 32x39 |
| 120x40 | 3/5/12/32 | 120 | 4 | 28x37 | 88 | 3 | 32x39 |
| 160x50 | 1 | 160 | 2 | 28x47 | 128 | 1 | 32x49 |
| 160x50 | 3 | 160 | 4 | 28x47 | 128 | 3 | 32x49 |
| 160x50 | 5/12/32 | 160 | 5 | 28x47 | 128 | 4 | 32x49 |
| 240x80 | 1 | 240 | 2 | 28x77 | 208 | 1 | 32x79 |
| 240x80 | 3 | 240 | 4 | 28x77 | 208 | 3 | 32x79 |
| 240x80 | 5/12/32 | 240 | 5 | 28x77 | 208 | 5 | 32x79 |

### Measurement Interpretation

- At the approved minimum 80x24 with the normal expanded panel, A already
  exposes a useful 25x21 preview beside the focused current column.
- B gains only 7 columns and 2 rows of preview at 80x24, while shrinking Stage
  width from 54 to 22 and reducing visible navigation from two columns to one.
- From 100x30 through 160x50, B gains only four preview columns and two rows
  while consistently removing one navigation column.
- At 240x80 and depth >=5 both options hit the same five-column bound; B still
  gains only four preview columns.
- A has no RightPanel collapse transition. B would keep one additional panel
  visible in the measured prototype; C would add a placement transition at
  every breakpoint crossing. Because no breakpoint makes A unusable, C's
  transition frequency has cost but no measured activation benefit.
- Path depth does not reduce A's focused/preview availability because the
  bounded viewport clamps around focus. It only changes how many ancestor
  columns can coexist.
- ASCII, CJK, emoji, and long grapheme names do not change geometry; they use
  the tested display-width clipping path inside the same rect.
- `none`, directory, text, image, pending, error, loading, truncated, and
  non-Kitty states share the same typed preview rect. They alter content and
  semantic status, not placement geometry.

## Behavior, Failure, Accessibility, and Transport

Fresh exact decision gate
`915bce56-1990-43b1-87a8-7078ddde6016` passed 14/14:

- Unicode/emoji identity survives;
- Unicode row names cannot overwrite action cells;
- long text clips to preview width;
- pending/warning/error states have explicit semantic text/style roles;
- non-Kitty image fallback is explicit;
- stale path/generation/resize image results are rejected;
- preview failures render without retry loops;
- stale text-worker completion after scroll is rejected;
- identical Miller frame sends zero payload;
- Miller render queue remains single-slot;
- overlay blocks every typed Miller row gesture;
- failed Files open restores prior surface/focus;
- close/reopen resets ephemeral Miller state;
- one-column degradation preserves current and two-column geometry restores
  inline preview.

Accessibility/interaction review:

| Dimension | A — current | B — RightPanel | C — hybrid |
|---|---|---|---|
| Keyboard owner | Existing Files/Miller owner; preview is display state | Needs a new RightPanel focus/scroll owner and order | Needs both owners plus deterministic transfer |
| Mouse travel | Preview is adjacent to current across one divider | Preview is separated from current by the remaining Stage | Changes with breakpoint |
| Overlay blocking | Production-tested | No production hit/focus consumer exists | Both paths and transition need tests |
| Close/reopen | Generation and ephemeral state tested | New panel target retirement required | Target retirement plus relocation required |
| Stage switch | One snapshot retired with Files | Separate outer-region consumer must retire | Two consumers and breakpoint transition |
| Status semantics | Explicit text plus style; not color-only | Would need the same renderer/labels | Must preserve both |
| Screen reader | Ratatui cell UI has no OS-level screen-reader bridge | Moving cells does not create one | Moving cells does not create one |
| Tiny/mobile | Existing current-first fail-closed degradation | A 32-cell rail competes directly with minimum Stage | Breakpoint adds another transition edge |

Performance/transport:

- A has real release evidence: compute p95 10us/14us and full-frame p95
  1,153us/4,115us at 120x40/240x80, below 500us and 8ms/16ms budgets.
- A identical-frame outgoing payload is 0 and pending render payload is
  bounded to one.
- B/C have only pure-geometry measurements. Their unimplemented render,
  target relocation, focus, collapse, and outgoing-byte behavior cannot be
  called within budget. This alone prevents a production GO.

## Option Gate

| GO requirement | A | B | C |
|---|---|---|---|
| Focused path readable at minimum | PASS | Marginal: Stage falls to 22 | Inherits B at any narrow breakpoint |
| Useful preview at activation | PASS: 25x21 at 80x24 | PASS: 32x23 | No breakpoint needed to make A useful |
| No render filesystem read | PASS | Unimplemented | Unimplemented transition |
| No new wire/runtime authority | PASS | Could remain client-local, but new presentation authority required | Two presentation authorities |
| Measured frame/outgoing budgets | PASS | FAIL: not implemented/measured | FAIL: not implemented/measured |
| Unambiguous focus/scroll/overlay/lifecycle | PASS | FAIL: owners absent | FAIL: transfer absent |
| One bounded revertible slice | Already delivered | Requires render, projection, input, persistence/collapse, worker target, tests | Larger than B; two placements and transitions |

## Final Decision and Rollback

**NO-GO for B and C. Keep A.**

This is not a claim that a RightPanel can never be useful. It means the current
evidence does not justify paying its architecture and interaction cost for a
4-cell normal-case preview gain when A is already useful at the approved
minimum and preserves more navigation context.

No rollback is required because FM5 changed no product code. If future
independent demand supplies a real Inspector/metadata consumer, it must start a
new approved micro-plan with:

- explicit RightPanel component/focus/hit ownership;
- snapshot and persistence/migration boundary;
- responsive breakpoint oracle;
- stale worker/target relocation tests;
- overlay, mobile, close/reopen, and Stage-switch tests;
- release-profile frame/outgoing measurements;
- one separately revertible implementation chain.

Apps/Desktop, speculative RightPanel consumers, and a generic component
registry remain out of scope.

## Final Graph and Repository Boundary

After the decision commit, the safe single-worker incremental refresh reparsed
11 changed files with zero extraction errors; the final three-file continuity
refresh also completed with zero errors and produced 21,078 nodes / 98,023
edges. Both the CLI and long-lived MCP channel now return the same
fresh store and find current `App::handle_key_headless` and
`miller_viewport_geometry` symbols. Freshness is established by those symbols,
not by `ready` alone.

The worktree has zero product-code diff. The only untracked path is the
user-owned `.superpowers/`, which was never staged or edited.
