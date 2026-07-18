# Files Visibility, Preview, Plugin and Mouse Research

Date: 2026-07-18
Branch/HEAD at research start: `feat/native-fm` /
`6a972703113b473babd26a6ab18d14d1c937ac46`

## Closure update

FMR-1 is now closed. The research-start observations about `flatten()` and
silent non-UTF-8/hidden omission describe the pre-fix source. Current closure
evidence, TDD commits, VIS-13, and fresh gates are recorded in
`.codex/evidence/files-visibility-runtime-matrix.md`. FMR-2 sidebar mouse is
the active dependency.

## Verified repository and runtime state

- Git HEAD and CyPack `feat/native-fm`/`master` are exactly `6a972703`.
- The tracked worktree was clean; user-owned `.superpowers/` was the only
  untracked path and was not touched.
- Codebase Memory was fresh at 21,304 nodes / 98,123 edges and returned
  `plain_wheel_over_empty_trail_body_uses_fractional_horizontal_fallback`.
- The installed binary `/home/ayaz/.local/bin/herdr` is dated 2026-07-12 and
  currently has live client/server processes.
- The repository debug binary `target/debug/herdr` is dated 2026-07-18 and
  also has live client/server processes.
- Therefore the post-reboot symptom is not lost commits or `/tmp` source.
  Running plain `herdr` selects the older installed binary; the new scroll and
  sidebar behavior exists only in the current debug build until a separately
  authorized install/release workflow updates the installed binary.
- No process was stopped, restarted, signaled, or connected to during this
  audit.

## Directory visibility data path

Current authority chain:

```text
mouse/keyboard TrailRowView exact path
  -> FmState::activate_trail_entry
  -> TrailSnapshots::activate_entry
  -> TrailSnapshots::select_dir
  -> read_directory_snapshot
  -> TrailColSnapshot
  -> project_trail_view_inner
  -> render_trail_view
```

The current source proves that a directory may appear empty or fail to append
for distinct reasons:

1. `show_hidden == false` removes every name beginning with `.`. A directory
   containing only dotfiles becomes an `Available` empty column.
2. `read_dir(...).flatten()` silently removes individual iterator errors.
   A partial read can be rendered as a shorter or empty successful snapshot.
3. `entry.file_name().to_str()?` silently removes non-UTF-8 entries.
4. A directory-level `read_dir` failure becomes `Missing`,
   `PermissionDenied`, `ReadOnly`, or `Unavailable`; `select_dir` refuses to
   append the column when the status is not `Available`.
5. `project_trail_view_inner` returns a completely empty default view when
   Trail columns and snapshot columns are temporarily misaligned.
6. Existing tests prove ordinary deep directory activation and sidebar
   navigation separately, but do not yet provide one table-driven
   classification across hidden-only, non-UTF-8, partial iterator error,
   permission, symlink-directory, stale alignment, and fifth-to-sixth-column
   activation.

The existing fifth-column model behavior is not a depth cap: Trail supports a
bounded chain and `every_visible_column_is_loaded`/deep-link families prove
new loaded columns can be appended. The reported live case must be classified
by exact path kind/status and binary identity before changing trail depth.

## Files sidebar mouse gap

Current authority chain:

```text
compute_file_manager_sidebar_row_areas
  -> ViewState.file_manager_sidebar_row_areas
  -> file_manager_sidebar_path_at (geometry + live model revalidation)
  -> request_file_manager_sidebar_navigation
  -> App::handle_scheduled_tasks
  -> sync_file_manager_sidebar_navigation
  -> FmState::open_trail_to
```

There are two adjacent tests:

- `clicking_file_sidebar_item_prepares_exact_typed_navigation_request`
  proves the mouse hit produces a request but intentionally does not consume
  it.
- `sidebar_navigation_opens_exact_directory_and_rejects_stale_targets`
  injects the request directly and calls the consumer manually.

No single test currently drives a real primary click through scheduled-task
consumption and then asserts the live Files generation, exact cwd, loaded
trail root, and visible rows. This seam gap can admit a runtime regression
while both adjacent tests remain green. The older installed binary is also a
confirmed confounder and must be eliminated from the reproduction.

## Current preview/render capability

Native Trail detail currently has three prepared variants:

- `Image`: extension-classified image path; decode/Kitty placement is handled
  by the image worker against the generation-bound detail content rect.
- `Text`: bounded text read and highlighting outside render.
- `Unpreviewable(String)`: explicit read/format failure.

The classifier is binary: recognized image extension versus “try as text”.
It does not yet expose a capability matrix for PDF, office documents, archive
listing, audio/video metadata, Markdown rendering, syntax delegation, git
diff, binary hex/metadata, or external preview providers.

Native core should retain directory identity, selection, security, bounded
read limits, generation cancellation, and pure render. Optional heavyweight
renderers may be adapters, never filesystem or input authorities.

## Reference and plugin findings

### `edmundmiller/herdr-plugin-hunk`

Primary source: <https://github.com/edmundmiller/herdr-plugin-hunk>

- One commit (`11ba5dcca4358203ca68f160becf6870cf016c18`), version `0.1.0`.
- Six manifest actions open worktree/staged/branch diffs in split or tab.
- Uses `HERDR_PLUGIN_CONTEXT_JSON`, `HERDR_BIN_PATH`, exact argv construction,
  `pane split`/`tab create`, `pane rename`, and `pane run`.
- Requires Herdr 0.7.0+, Python, and `hunk` or `bunx hunkdiff`.
- No GitHub release and no detected license file.
- Fit: useful reference for context validation and pane orchestration.
- Non-fit: it does not extend native Files rows, directory loading, or the
  Trail detail renderer.

### `smarzban/herdr-file-viewer`

Primary source: <https://github.com/smarzban/herdr-file-viewer>

- MIT, Rust/Ratatui, version `1.13.0`, 158 stars at research time.
- One process owns a tree and content pane.
- `.gitignore`-aware tree; hidden/changed filters; worktree switching.
- Delegates Markdown, diff, and syntax rendering to optional `glow`, `delta`,
  and `bat`, with plain-text fallback.
- Rendering runs off the input thread; monotonic jobs discard stale results;
  renderer panic is contained.
- Treats file content and renderer output as untrusted and neutralizes escape
  sequences.
- Best reference for preview capability boundaries, stale-result rejection,
  optional renderer fallback, and security tests.
- It should remain a separately launched expert viewer, not replace Herdr's
  native Files navigation authority.

### Other relevant Herdr plugins

- <https://github.com/dwarvesf/herdr-quicklook>: MIT, overlay quick preview;
  uses `bat`/`less`, optional `fzf`, and can escalate to file-viewer.
- <https://github.com/persiyanov/herdr-reviewr>: MIT, diff/file sidebar and
  explicit split-pane lifecycle.
- <https://github.com/arvindparmar-me/herdr-markdown-viewer>: narrow
  Markdown-in-split reference; no detected license.
- Official examples:
  <https://github.com/ogulcancelik/herdr-plugin-examples> demonstrate action,
  event, link-handler, pane, and install-build contracts.
- Marketplace discovery is GitHub topic-based (`herdr-plugin`); repository
  presence is not a quality or security endorsement.

### Broader file/preview references

- <https://github.com/sxyazi/yazi>: async cross-platform file manager; useful
  for bounded asynchronous preview/cancellation patterns.
- <https://github.com/yorukot/superfile>: multi-panel terminal file-manager
  UX reference.
- <https://github.com/Canop/broot>: tree navigation and filtering reference.
- <https://github.com/hpjansson/chafa>: multi-protocol terminal image
  fallback reference (Kitty/Sixel/symbol-based rendering).
- Canonical Miller UX remains the user-provided
  `CircetMillerSection.tsx`; external projects cannot override its Trail laws.

## Architecture decision

Use a hybrid boundary:

- Native Herdr core owns directory enumeration truth, exact path identity,
  Trail state, mouse geometry, status/error projection, lightweight bounded
  text, and current Kitty image placement.
- A typed preview capability registry inside the client selects a native
  lightweight provider or an optional external/plugin action.
- Plugin panes own heavyweight expert workflows such as full file browsing,
  rendered Markdown, rich git diff, PDF/office tooling, or external commands.
- Plugin failure, absence, timeout, malformed output, or unsupported platform
  must degrade to an explicit native fallback without changing selection,
  cwd, Trail, or terminal runtime identity.

This is a research decision, not an implementation-complete claim.
