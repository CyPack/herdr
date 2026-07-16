# Shell Foundation SF2 Geometry Progress Evidence

Date: 2026-07-16

## Decision

Result: **PASS for SF2.1-SF2.4. SF2 is closed; SF3.1 is next.**

The bounded named-region model, typed templates, validation, track policies,
deterministic solver, responsive degradation, cached `ShellView` projection,
and generation-safe flattened semantic hits are published. The compatibility
projection preserves the existing sidebar/center output, and the next product
slice is SF3.1 transactional resize.

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
- `2a440478` — `test: define shell view projection contracts`
- `07133b8b` — `refactor: project shell geometry through bounded view`

The SF2.4 GREEN commit contains only `src/ui/shell.rs`, the new
`src/ui/shell/view.rs`, `src/ui.rs`, `src/app/state.rs`, and `src/app/mod.rs`.
The user-owned `.superpowers/` tree was not staged.

Before SF2.4 publication, both CyPack refs resolved to
`c7307469c17354d60721c84f09f7acba7c08b036` and were proven ancestors of the
local RED/GREEN chain. Sequential fast-forward pushes published both commits.
CyPack `feat/native-fm` and fork `master` both resolve to exact SHA
`07133b8b9e9cf10b9b3dea0febe22a8389457164`. `upstream` was not pushed and no
force operation occurred.

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

SF2.4 RED commit `2a440478` is compile-valid behavior evidence:

- `unchanged_geometry_key_reuses_shell_generation` observed generation 0
  instead of 7; run `21b60a22-fa23-44d9-83e5-2618f69ee830`.
- `area_or_constraint_change_advances_shell_generation_once` observed
  generations `[0, 0]` instead of `[8, 8]`; run
  `1ee8f7e0-787d-4e19-90df-c58715b3eb61`.
- `flattened_hits_are_complete_disjoint_and_in_bounds` observed zero hits
  instead of two; run `681bf711-d097-4bd0-aed4-71ff4dee65ab`.
- `stale_shell_hit_generation_is_rejected` returned the current AppDock hit
  for a stale generation instead of `None`; run
  `2d72d7fd-e55d-440a-91dd-953106a40ab1`.
- `legacy_sidebar_and_center_rects_match_compatibility_projection` passed as
  an explicit compatibility characterization; run
  `57772f1f-ffef-46d7-a7fc-ad8391d2c479`.

The mobile-empty and generation-exhaustion tests were added after GREEN as
failure-path characterizations. They are not represented as missing-behavior
RED evidence.

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

## Cached View and Hit Contract

- `ViewState` owns one client-local aggregate `ShellView`; no region-by-region
  fields, runtime facts, socket messages, filesystem handles, or PTY authority
  were added.
- The complete cache key carries terminal area plus layout, constraint, and
  collapse revisions. A same-key projection returns the owned prior view
  without solver invocation or HashMap/Vec clone.
- Every authoritative key change advances generation exactly once and
  flattens the finite named regions in a deterministic order. Absent and
  zero-area regions produce no hit target, and the `CenterContent`
  compatibility alias never creates duplicate input authority.
- `hit_at` rejects a generation mismatch before coordinate lookup. Generation
  exhaustion keeps the new geometry visible but clears every hit so `u64`
  wrap cannot alias an old token.
- Desktop maps `WorkspaceStage` back through the current center/terminal flow.
  Mobile owns a distinct empty-projection revision, clears desktop hits once,
  and reuses the empty view on identical frames.
- The production compute path moves the cached view with `mem::take`; render
  remains pure, and the existing retained PTY/render queue behavior is
  unchanged.

## Fresh Verification

SF2.4 exact-head closure:

- Exact cached-view/failure selection: 7/7 pass; run
  `97100ca9-f187-44b5-a8cf-eb6966564ba9`.
- Broad shell family: 88/88 pass; run
  `8c0602de-6fbf-4a8e-9683-556d940afea2`.
- Every `src/ui.rs` test: 41/41 pass; run
  `f56f4f7b-49a2-445c-9e9d-7efe145da2ae`.
- Frozen SF1 baseline: 11/11 pass; run
  `3e18ca5d-ac40-4e8b-a2df-576bc22aee5b`.
- Full repository Nextest: 3239/3239 pass plus only the named B0 real-host
  probe skip; no retry; run `a66a2977-7356-4bab-8fa8-4d224cdf958e`.
- Ordinary Cargo proves `path_beta_real_host_probe` as 1 ignored and 0 failed
  without executing it.
- Linux all-target and canonical Windows MSVC binary Clippy pass with
  `-D warnings`; Bun integration assets 5/5, plugin marketplace 12/12, and
  Python maintenance 64/64 pass; fmt and diff checks are clean.
- Deterministic performance evidence: a cache hit invokes the dynamic resolver
  zero times; a miss retains the existing at-most-two-visits-per-node solver;
  flattened outer hits are capped by the six finite region identities. No
  unsupported wall-clock latency claim is made.

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

Before SF2.4 refresh, the built-in MCP channel reported `ready` at the SF2.3
19,966 nodes / 92,183 edges and could not find `compute_shell_view`, proving
that `ready` alone was stale. No proxy or process was restarted. The documented
single-worker command ran:

```text
CBM_WORKERS=1 codebase-memory-mcp cli index_repository '{"repo_path":"/home/ayaz/projects/herdr","mode":"fast","persistence":true}'
```

It completed with zero extraction errors at 20,017 nodes / 91,917 edges. CLI
and built-in MCP status/search/snippet calls now return the by-value
`compute_shell_view`, both new failure-path characterizations, and existing
`miller_layout`. The graph reports `compute_shell_view` with no loop,
allocation-in-loop, recursion, or linear scan, and the earlier bounded solver
visit proof remains green.

## Exact Next Gate

SF2 completed delivery gates I7-I14. SF3.1 now begins its fresh drift and
characterization pass before the first new RED. The next behavior-specific RED
commit is `test: define transactional shell resize`, starting with
`divider_down_captures_original_constraints` and
`drag_preview_clamps_without_dirty_or_pty_resize`. Production code must not be
written until the compile-valid assertions fail for missing resize-transaction
behavior rather than setup or compilation.
