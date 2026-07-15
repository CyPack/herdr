# Shell Foundation SF2 Geometry Progress Evidence

Date: 2026-07-15

## Decision

Result: **PASS for SF2.1-SF2.3. SF2 remains open; SF2.4 is next.**

The bounded named-region model, typed templates, validation, track policies,
deterministic solver, and responsive degradation are published. Cached
`ShellView` projection and flattened semantic hits are intentionally not part
of this checkpoint and remain the only unfinished SF2 microtask.

## Git and Publication Boundary

The published SF2 chain is:

- `c45deea7` — `test: define shell workspace geometry contract`
- `a518693c` — `feat: add shell workspace region identities`
- `b6bc2e80` — `test: define bounded shell tree contracts`
- `9e22a775` — `feat: add bounded shell tree validation`
- `8c10ca21` — `test: define bounded shell content contracts`
- `914e894b` — `test: define shell track and template contracts`
- `2868a681` — `feat: add bounded shell model and templates`
- `2abf2463` — `test: define shell allocation and degradation`
- `f272a881` — `feat: add deterministic shell layout solver`

Only `src/ui/shell.rs`, `src/ui/shell/model.rs`,
`src/ui/shell/template.rs`, and the new `src/ui/shell/layout.rs` entered the
SF2.3 GREEN commit. The user-owned `.superpowers/` tree was not staged.

Before publication, both CyPack refs resolved to
`2868a6812bf5c604657bf90985ddb106169a1d87` and were proven ancestors of the
local head. Sequential fast-forward pushes then published the complete
RED/GREEN pair. CyPack `feat/native-fm` and fork `master` both resolve to exact
SHA `f272a8811e70b054f5c67f23343d354ff43ecfae`. `upstream` was not pushed and
no force operation occurred.

## RED Validity

The SF2.3 track-policy and responsive tests compiled before failing at their
named behavior assertions. Exact RED evidence included:

- Fixed `ef289ffb-3728-4adb-9644-dddcd2386a4d`: legacy width 9, expected 5.
- ContentBounded `af6b860e-d5dc-44fa-80d5-099aaf34b2d5`: width 1, expected 3.
- Resizable `5af38115-61ea-4c9e-879c-0500680b1d92`: width 19, expected 5.
- Fill `fb30a948-7728-46b9-8eae-9954eaee795c`: width 40, expected 10.
- Collapsed `3624043c-4c5d-40bf-9cdc-783dc33308db`: non-zero, expected zero.
- Remainder `5fa19258-93c4-4935-9bfa-6060f66ab053`: width 3, expected 2.
- Priority degradation `ff672505-181c-41ee-ba45-44d2f520f9ce`: dock width 5,
  expected 3.
- Bounded visits `5ce871ea-4ff2-443d-8fdc-cc8a057e6cf9`: RED-only sentinel
  exceeded the two-visit contract.
- Explicit TooSmall `2ba9b43f-c2e1-41ee-ba45-44d2f520f9ce`: Workspace,
  expected TooSmall.
- LeftPanel bounds `c000d894-e128-479a-b12b-c14c34dacf77`: width 0,
  expected bounded width 2.
- Height degradation `df5f9408-709c-438d-b992-013a88b59e23`: Stage height 0,
  expected 1.
- Nested Stage `31538894-6c85-4faa-a072-b90603d04de2`: TopBar remained
  visible when the single row belonged to the nested Stage body.

`zero_area_never_underflows` and
`all_rects_are_inside_parent_without_overlap` passed immediately and are
truthfully classified as existing arithmetic characterizations, not invented
RED evidence. A later height-mode table initially failed because its test
helper supplied a different ContentBounded measurement than the rect
assertions; production code was not changed, the helper was bound to the same
measurement authority, and the corrected test passed.

## Solver and Failure Contract

- Validation rejects over-depth, over-count, duplicate, missing-stage,
  collapsed-stage, and invalid-track inputs before activation.
- The tracked solver uses bounded validation plus measure/allocate traversal;
  it has no per-cell loop and no sibling search inside a child loop.
- Fixed, ContentBounded, Resizable, Fill, and Collapsed policies use saturating
  terminal-cell arithmetic and deterministic weighted remainder ordering.
- Width degradation is RightPanel, compact LeftPanel, AppDock, then explicit
  TooSmall while preserving one Stage cell whenever feasible.
- Height degradation removes BottomBar, then TopBar, and preserves one Stage
  row whenever feasible, including when Stage is nested inside the desktop
  body.
- The compact LeftPanel width clamps to its declared min/max rather than
  overriding template constraints.
- The real `DesktopWorkspace` template is characterized at normal, compact,
  and TooSmall sizes. Invalid internal tracked layouts produce no partial
  region projection.
- Legacy layouts with no track map continue through the original Ratatui
  projection unchanged.
- Production additions contain no `unwrap()` or `expect()`.

## Fresh Verification

- Core solver selection: 17/17 pass, including all 15 planned solver tests;
  run `6c4ca824-3574-417a-bef1-fc6a2b05f3ed`.
- Real-template and invalid-tree characterizations: 2/2 pass; run
  `fabfd4fc-88ce-4c9c-be53-9c74f0005d26`.
- Corrected width/height threshold tables: 2/2 pass; run
  `bd9baf2d-5369-443d-aa1a-0b52bb4d73b8`.
- Broad shell family: 81/81 pass; run
  `fdfeab8c-cda8-4e5b-8499-f123da60dfb7`.
- Frozen SF1 baseline: 11/11 pass; run
  `86077e5c-70f1-4ca8-80f6-54e491b2ce34`.
- Full repository Nextest: 3232/3232 pass plus only the named B0 real-host
  probe skip; run `0dbe4603-d62a-4ca6-b1e1-7ffed936d6a0`.
- Ordinary Cargo test proves `path_beta_real_host_probe` as 1 ignored and 0
  failed without executing it.
- `cargo fmt --check`: PASS.
- Linux all-target Clippy with `-D warnings`: PASS.
- Canonical Windows MSVC binary Clippy with `LIBGHOSTTY_VT_SIMD=false` and
  `-D warnings`: PASS.
- Bun integration assets 5/5 and plugin marketplace 12/12: PASS.
- Python maintenance modules 64/64: PASS.
- `git diff --check`: PASS.

`just` is not installed, so every applicable lowercase `justfile` `check`
command was executed directly. No stable Herdr process, inherited socket,
installed binary, terminal, browser, editor, or other user process was
contacted, restarted, or terminated.

## Graph Evidence

The built-in MCP channel reported `ready` at the old 19,890 nodes / 91,721
edges and could not find the new solver, proving that `ready` alone was stale.
No proxy or process was restarted. The documented single-worker command ran:

```text
CBM_WORKERS=1 codebase-memory-mcp cli index_repository '{"repo_path":"/home/ayaz/projects/herdr","mode":"fast","persistence":true}'
```

It completed with zero extraction errors at 19,966 nodes / 92,183 edges. CLI
status, search, and exact snippet calls return current `allocate_lengths`, the
real Desktop template test, the invalid-tree failure test, and the existing
`miller_layout`. The graph reports `allocate_lengths` with loop depth 1, no
linear scan in a loop, no allocation in a loop, and no recursion.

## Exact Next Gate

SF2.4 starts with test-only contracts:

1. `unchanged_geometry_key_reuses_shell_generation`;
2. `area_or_constraint_change_advances_shell_generation_once`;
3. `flattened_hits_are_complete_disjoint_and_in_bounds`;
4. `stale_shell_hit_generation_is_rejected`;
5. `legacy_sidebar_and_center_rects_match_compatibility_projection`.

The tests must be compile-valid and fail only for missing cached `ShellView`
generation/hit behavior. Production then adds `src/ui/shell/view.rs`, one
aggregate `ShellView` in `ViewState`, and compatibility projection without
changing visible output. SF2 closes only after exact tests, all `src/ui.rs`
tests, the SF1 baseline, Linux/Windows/full gates, CyPack-only publication, and
fresh graph evidence.
