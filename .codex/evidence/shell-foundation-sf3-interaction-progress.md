# Shell Foundation SF3 Interaction Progress

Date: 2026-07-16

## Decision

Result: **PASS; SF3.1 and its I2-I14 delivery gates are closed. SF3.2
collapse/restore and scroll ownership is next.**

SF3.1 adds one client-local, transient resize transaction shared by mouse and
keyboard input. Preview changes only projected shell geometry. It does not
write persistence or resize a PTY. A changed commit marks persistence dirty
once and requests at most one high-level PTY resize; cancel, stale capture,
same-size commit, terminal resize, and no-capture paths are inert or
effect-free by contract.

## Git and Publication Boundary

- Pure reducer RED/GREEN: `368c4d3a` / `d89a7f94`.
- Lifecycle RED/GREEN: `b6570ee4` / `807cb76c`.
- Sidebar adapter REDs: `96a1660e`, `09944834`; GREEN: `61b915a9`.
- Keyboard/input-ownership REDs: `4888c3f8`, `4026c28b`, `960b6d5f`;
  GREEN: `336fa3de`.
- Product checkpoint exact SHA:
  `336fa3ded217c49ceaed1d8876127a843562e152`.
- CyPack `feat/native-fm` and fork `master` both resolved to that exact product
  SHA after fast-forward ancestor checks. `upstream` was not pushed and no
  force push occurred.
- The user-owned untracked `.superpowers/` tree was not staged.

## Behavior and Ownership Evidence

- `ResizeTransaction` owns stable divider identity, view generation, pointer
  origin, original tracks, and transient preview tracks.
- Pointer preview is original-coordinate-relative; repeated keyboard steps are
  current-preview-relative but reuse the same bounded normalization function.
- Mouse Down captures, Drag previews, and Mouse Up commits. A click without
  movement clears capture without dirtying or requesting a runtime resize.
- During preview, `compute_view` projects the transient width and suppresses
  pane/runtime resize. Generation rebasing keeps the capture valid across its
  own geometry preview frames.
- Terminal area drift cancels the preview from original constraints without
  persistence or PTY effects.
- Active keyboard capture consumes axis arrows and `h`/`l`, commits on Enter,
  cancels on Escape, and consumes unsupported keys inertly.
- Input order is topmost modal/overlay, active shell capture, native surface,
  then terminal. The context-menu Escape test proves the overlay remains the
  owner while the terminal-dispatch test proves resize keys do not leak to a
  PTY surface.
- The existing sidebar-section divider remains independently owned; SF3.1
  changes only the outer sidebar/WorkspaceStage divider.

## RED and GREEN Evidence

- Initial reducer RED: 2/2 behavior assertions failed, run
  `b2f282ec-7905-4550-a520-27835819a929`.
- Lifecycle RED: 7/7 behavior assertions failed, run
  `9450ba24-2e05-420e-a187-36200cce3d9b`.
- Sidebar adapter RED: 5/5 behavior assertions failed, run
  `55fe6aa7-7015-4601-9300-bb82f8a7b933`.
- No-op commit RED: 1/1 behavior assertion failed, run
  `96edd1bd-3aaf-4858-be17-f08903d21c5c`.
- Keyboard state RED: 5/5 behavior assertions failed, run
  `ce5b6dae-e8b9-475a-bde8-86f21d2bd524`.
- App dispatch RED: 1/1 behavior assertion failed, run
  `b819e0f3-4488-45d5-9f36-3c4e359ef773`.
- Direction/overlay RED: 2/2 behavior assertions failed, run
  `43cd9351-d417-4b52-8dc2-6cfe7a5e804a`.
- Final keyboard/ownership selection: 8/8 pass, run
  `d4f8951f-5b6b-4337-80d8-6c00849ee1e0`.
- Broad shell/sidebar/input family: 119/119 pass, run
  `7e67c017-c4e8-4120-95a4-7363696ea52f`.

Compile failures, zero-test filters, and fixture/setup errors were not counted
as RED. The one zero-test exact-name probe was corrected to a bounded regex and
is not closure evidence.

## Full Closure Gates

- Full repository Nextest: 3264/3264 pass, one intentional skip, no retry;
  run `a49f6991-2fad-4787-87ed-c32662e7a00e`.
- Ordinary Cargo ignored inventory contains exactly
  `kitty_graphics::tests::path_beta_real_host_probe`; it was listed without
  executing the real-host probe.
- Frozen SF1 characterization: 11/11 pass, run
  `4f549c24-91d6-4457-ad59-a6b9b77b801e`.
- Linux all-target/all-feature Clippy and canonical Windows MSVC binary Clippy
  pass with `-D warnings`.
- Bun integration assets 5/5 and plugin marketplace 12/12 pass (17/17 total).
- Python maintenance modules pass 64/64.
- `cargo fmt --check`, `git diff --check`, and added-production-`unwrap()`
  inspection pass.
- No stable Herdr process, installed binary, inherited socket, user process,
  real PTY, or persistence file was contacted, restarted, or terminated.

## Graph Evidence

The pre-refresh built-in graph reported `ready` at 20,118 nodes / 93,603 edges
but could not find the new key-route or keyboard-step symbols, so it was
correctly treated as stale. A process-safe single-worker CLI incremental
refresh completed with zero extraction errors at 20,132 nodes / 93,587 edges.
Fresh CLI searches return:

- `AppState.handle_shell_resize_key`, connected to `src/app/input/mod.rs`;
- `ShellInteractionState.preview_keyboard_resize_step`;
- `ResizeTransaction.preview_keyboard_step`; and
- the existing `miller_layout` anchor.

The already-running built-in MCP transport continues to serve its prior
snapshot. It was not restarted because user-process/session safety takes
precedence; current freshness is proven from the refreshed CLI graph content,
not from the stale transport's `ready` flag.

## Next Test Point

SF3.2 begins with a fresh drift/ownership pass, then the smallest compile-valid
behavior RED is `collapse_remembers_last_committed_width`:

- expected: collapse stores the last committed, bounded width and produces one
  revision/dirty transition;
- repeated collapse expected: inert, with no second revision or dirty mark;
- reason: restore cannot be correct or persistence-safe unless collapse owns a
  stable committed value rather than transient preview geometry.

No scroll reducer or snapshot v4 production code is authorized before its own
matching behavior RED.
