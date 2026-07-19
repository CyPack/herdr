# Herdr Native Files Layout V1 — Locked

## Status

- Version: `Files Layout V1`
- Decision: approved and locked by the user on 2026-07-19
- Freeze checkpoint: `d98c31c70946e496cb6536f02fc96e45974df2de`
- Branch family: `feat/native-fm`
- Visual oracle: Playwright Chromium
- Source design:
  `docs/superpowers/specs/2026-07-19-herdr-files-content-locations-rail-design.md`
- Source implementation plan:
  `docs/superpowers/plans/2026-07-19-herdr-files-content-locations-rail-implementation.md`

This document freezes the currently approved Files composition as a durable
product version. It does not replace the detailed FCL design. It names the
resulting composition, records the invariants that must survive later work,
and defines when a future change requires `Layout V2`.

## V1 Composition

```text
global agent/workspace panel | Files locations rail | Miller Trail | detail
```

The global left panel remains Herdr's agent/workspace runtime tracker while
Native Files is active. Files-local Favorites and Locations occupy the left
edge of the Files content area. The scrollable Miller Trail grows to the right
of that rail. The detail/preview region remains at the Trail's active end.

The locations rail is not restored to the global shell sidebar. Files
activation remains a singleton stage action and does not replace the current
Spaces/Projects body with file shortcuts.

## Locked Product Laws

### V1-L1 — Global runtime visibility

Opening Files preserves the global agent/workspace tracker. Files navigation
never consumes that global panel's full height.

### V1-L2 — Files-local location ownership

Favorites and Locations render and hit-test only inside the Native Files
content area. Wide and standard modes use a persistent rail. Compact mode uses
one bounded `Locations` drawer and gives the closed drawer's width back to the
Trail.

### V1-L3 — Explicit origin

Location highlight authority is exact `Location(path)` or `Direct(path)`
identity. It is not inferred from cwd equality, path ancestry, or longest
prefix. Descending beneath Home keeps Home highlighted until another exact
location is explicitly activated.

### V1-L4 — Scroll ownership

The locations rail owns vertical location scrolling. The Trail owns
horizontal navigation only inside its projected rectangle. One horizontal
gesture advances by the existing fractional one-third column-width contract.
Partially visible columns remain rendered and hit-testable from the same
current-frame projection.

### V1-L5 — Column and detail behavior

Every loaded path segment may own one Miller column. Directory activation
branches or extends the Trail; file activation updates the detail state
without inventing another directory column. Active-end follow, manual
horizontal offset, per-path column widths, grouped mtime rows, row actions,
and exact path revalidation remain part of V1.

### V1-L6 — Non-blocking filesystem preparation

Root switching, Miller navigation, and watcher/current refresh directory reads
use the bounded one-executing/one-latest-pending file-manager I/O lane.
Directory enumeration and per-entry metadata reads do not run in render,
mouse input, or scheduled result-apply paths.

### V1-L7 — One geometry authority

Rail, separator, Trail, detail, drawer, rows, section headers, timestamps, and
actions derive from the current immutable frame projection. Rectangles are
bounded and disjoint. Stale generation, stale model revision, clipped targets,
gaps, separators, and inaccessible rows are inert.

### V1-L8 — Responsive modes

- Wide: persistent locations rail targeting 24 cells, clamped to 18–28.
- Standard: persistent compact rail in the 16–20 cell range while preserving
  at least one useful Miller column.
- Compact: no persistent rail; one complete `Locations` action opens the
  bounded drawer.

Mode boundaries derive from content minimums, not device names.

## Visual Baseline Lock

The following committed Playwright Chromium PNGs are the V1 layout oracle:

- `VIS-18` wide composition;
- `VIS-19` Home origin below a deeper cwd;
- `VIS-20` exact nested-origin transfer;
- `VIS-21` standard composition;
- `VIS-22` compact, drawer closed;
- `VIS-23` compact, drawer open;
- `VIS-24` pending location with resident Trail preserved;
- `VIS-25` bounded failure with resident Trail preserved.

The supporting Trail oracles remain part of the contract:

- `VIS-07..10` depth, rebranch, detail, and deep-link behavior;
- `VIS-11` horizontal viewport;
- `VIS-12` fractional one-third scrolling;
- `VIS-13` bounded directory omissions;
- `VIS-14` metadata preview;
- `VIS-15..17` mixed mtime groups and reorder selection.

Existing baselines must not be globally rewritten. A deliberate visual change
requires a spec-scoped mutation proof, a documented semantic diff, and the
versioning decision below.

## Versioning Rule

The following are permitted as `V1.x` work when all V1 visual and semantic
oracles remain unchanged:

- latency, allocation, queueing, and render optimizations;
- correctness fixes that preserve the locked geometry and ownership;
- additional observability that is disabled by default;
- accessibility or failure-state hardening that does not change the visible
  composition.

The following require an explicit `Files Layout V2` decision:

- moving file locations back into the global runtime panel;
- changing the four-surface ownership or order;
- replacing the persistent rail/drawer responsive model;
- changing the fractional scroll contract;
- changing the fundamental column/detail hierarchy;
- accepting intentional changes to the V1 composition baselines.

No agent may silently reinterpret a V2-class change as a V1 bug fix.

## Performance Amendment Boundary

The reported rapid-click latency is not part of the approved visual design.
It is a V1 performance defect investigation. Its work may optimize dispatch,
state projection, bounded worker scheduling, render coalescing, or frame
streaming only if the V1 laws and Chromium baselines remain unchanged.

The active investigation contract is:
`docs/superpowers/specs/2026-07-19-herdr-files-rapid-navigation-latency-prd.md`.

## Freeze Evidence

At the freeze checkpoint:

- FCL-0 through FCL-7 and 25/25 `TP-FCL-*` points are closed;
- full Rust passed 3,577/3,577 with 3 declared skips;
- Playwright Chromium passed 33/33;
- Linux and Windows Clippy passed with warnings denied;
- focused FCL passed 29/29;
- Python maintenance passed 68/68;
- Bun suites passed 5/5 and 12/12;
- Codebase Memory recorded 23,854 nodes / 124,093 edges;
- CyPack `feat/native-fm` and `master` both equal the freeze checkpoint.

These values are freeze evidence, not a claim that later commits inherit the
same results without fresh verification.

## Safety

Layout V1 grants no authority to touch the installed stable Herdr process,
stable socket, existing terminal/browser/editor sessions, upstream,
release-channel files, or user-owned `.superpowers/`. Runtime measurements use
only the isolated, test-owned environment defined by
`.local/ISOLATED-DEV-TEST.md`.
