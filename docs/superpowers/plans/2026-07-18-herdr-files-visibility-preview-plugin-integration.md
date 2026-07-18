# Files Visibility, Preview and Plugin Integration Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> `superpowers:subagent-driven-development` (recommended) or
> `superpowers:executing-plans` to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make every directory activation truthful and observable, restore
end-to-end sidebar shortcut mouse navigation, and add a production-grade
preview capability architecture that can reuse optional Herdr plugins without
moving Files authority out of the native client.

**Architecture:** Preserve `FmState` + Trail as the only navigation and input
authority. First separate directory enumeration outcomes from genuinely empty
directories, then close the full sidebar click-to-loaded-trail test seam.
Introduce preview capabilities only after these correctness gates; lightweight
text/image stays native while heavyweight viewers are explicit plugin-pane
adapters with bounded failure fallback.

**Tech Stack:** Rust, Ratatui `TestBackend`, crossterm mouse events, existing
Herdr plugin manifest/CLI/socket surface, Playwright 1.54.1 Chromium, nextest.

## Global Constraints

- Stable Herdr processes and inherited stable sockets are never touched.
- Every isolated run starts and ends with semantic cleanup of only its
  test-owned server/root; no `kill`, `pkill`, or `killall`.
- Reproduction records the executable path and build identity before behavior.
- `compute_view()` owns geometry; `render()` remains pure and performs no
  filesystem, process, socket, clock, random, or config I/O.
- Exact path + current generation/revision is authority; labels, row indices,
  and coordinates alone never authorize navigation.
- No production Rust change before a behavior-specific compile-valid RED.
- Every Rust command exports `PATH="$HOME/.local/bin:$PATH"`.
- Every RED run uses nextest `--no-fail-fast`.
- Visual acceptance is Playwright Chromium with ASCII fixture glyphs.
- No new product dependency until the capability matrix proves existing code
  and optional commands cannot satisfy the requirement.
- No upstream push, issue, or PR; no `.superpowers/` mutation or staging.

---

## Dependency chain

```text
P0 executable provenance and deterministic fixtures
  -> P1 directory visibility classification
      -> P2 explicit directory status projection
  -> P3 sidebar mouse end-to-end authority
  -> P4 native preview capability matrix
      -> P5 optional plugin adapter boundary
  -> P6 Chromium + isolated runtime + full gates
  -> P7 ranking and integration decision
```

P1 and P3 may be investigated independently after P0, but product commits stay
sequential. P4 cannot begin until navigation truth is green; otherwise preview
work would mask an empty/stale column bug.

### Task 1: Freeze executable provenance and directory cases

**Files:**

- Create: `.codex/evidence/files-visibility-runtime-matrix.md`
- Create: `src/fm/directory_visibility_tests.rs`
- Modify: `src/fm/mod.rs`
- Test: `src/fm/directory_visibility_tests.rs`

**Interfaces:**

- Consumes: `read_directory_snapshot(&Path, bool) -> FmDirectorySnapshot`.
- Produces: a table-driven fixture family with exact expected status,
  visible-entry count, omitted-entry count, and omission reason.

- [ ] **Step 1: Add the compile-valid characterization matrix**

  Add fixtures for ordinary files, empty directory, hidden-only directory,
  visible+hidden mix, symlink-directory, missing directory, non-directory,
  permission denied on Unix, non-UTF-8 name on Unix, and a six-level chain.
  The fifth-to-sixth activation row must assert
  `TrailActivateOutcome::Branched` and six aligned loaded snapshots.

- [ ] **Step 2: Run the matrix before production changes**

  Run:

  ```bash
  export PATH="$HOME/.local/bin:$PATH"
  cargo nextest run --locked --no-fail-fast directory_visibility
  ```

  Expected: existing intentional cases pass; the new omission-accounting
  assertions fail because `FmDirectorySnapshot` does not expose omitted
  entries or their reasons.

- [ ] **Step 3: Record runtime identity without contacting a server**

  Record `readlink /proc/<pid>/exe`, binary mtime, `herdr --version`,
  `target/debug/herdr --version`, Git SHA, and isolated socket/root paths.
  Expected: the test window resolves to the debug executable built from the
  recorded SHA.

- [ ] **Step 4: Commit RED**

  Commit subject:

  ```text
  test: classify invisible file manager entries
  ```

### Task 2: Make directory enumeration outcomes truthful

**Files:**

- Modify: `src/fm/mod.rs`
- Modify: `src/fm/trail_snapshots.rs`
- Modify: `src/ui/file_manager/trail_view.rs`
- Test: `src/fm/directory_visibility_tests.rs`
- Test: `src/ui/file_manager/trail_view.rs`

**Interfaces:**

- Consumes: Task 1 fixture matrix.
- Produces: `FmDirectorySnapshot` with an explicit bounded issue summary;
  Trail columns distinguish empty, filtered-only, partially unreadable, and
  directory-level failure without exposing path-labelled telemetry.

- [ ] **Step 1: Replace silent per-entry flattening**

  Iterate `read_dir` results explicitly. Count entry-read failures and
  non-UTF-8 omissions in bounded numeric fields; never log arbitrary filenames
  or create an unbounded error list.

- [ ] **Step 2: Preserve hidden policy without calling it empty**

  Count hidden omissions separately. A hidden-only directory remains
  navigable and `Available`, but its column exposes a prepared “hidden items
  omitted; press `.` to show” state instead of looking indistinguishable from
  a genuinely empty directory.

- [ ] **Step 3: Project explicit column status**

  Extend the pure Trail view snapshot with prepared non-actionable status rows.
  Status rows carry no entry path, row action, or navigation authority.

- [ ] **Step 4: Verify RED becomes GREEN**

  Run:

  ```bash
  export PATH="$HOME/.local/bin:$PATH"
  cargo nextest run --locked --no-fail-fast directory_visibility trail_snapshots trail_view
  ```

  Expected: all new classification/status cases and existing loaded-column,
  rebranch, hidden-toggle, and stale-alignment families pass.

- [ ] **Step 5: Commit GREEN**

  Commit subject:

  ```text
  fix: expose file manager directory visibility states
  ```

### Task 3: Close sidebar shortcut mouse end to end

**Files:**

- Modify: `src/app/input/sidebar.rs`
- Modify: `src/app/file_manager_watcher.rs`
- Modify: `src/app/runtime.rs`
- Test: `src/app/input/sidebar.rs`

**Interfaces:**

- Consumes: `file_manager_sidebar_path_at`, one-shot
  `request_file_manager_sidebar_navigation`, and
  `sync_file_manager_sidebar_navigation`.
- Produces: one test helper that drives primary mouse down, scheduled task
  consumption, recompute, and final Trail projection.

- [ ] **Step 1: Write the missing end-to-end RED**

  Build a real accessible temp target with one visible file, compute the
  sidebar row, send unmodified left-down, call the production scheduled-task
  seam, recompute, and assert:

  - request consumed exactly once;
  - Files generation unchanged;
  - cwd equals exact target;
  - Trail root and snapshot directory equal target;
  - visible Trail row contains the target file.

  Repeat for Home, Downloads/pin, symlink-directory, stale model replacement,
  collapsed sidebar, overlay ownership, modified click, and inaccessible
  target.

- [ ] **Step 2: Run and observe the behavioral RED**

  Run:

  ```bash
  export PATH="$HOME/.local/bin:$PATH"
  cargo nextest run --locked --no-fail-fast sidebar_shortcut_mouse
  ```

  Expected: fail at the first actual production seam that does not preserve
  request/hit/navigation authority. A pass means the current source is correct
  and the live report is classified as executable/runtime drift; do not invent
  a product fix in that case.

- [ ] **Step 3: Apply only the confirmed source fix**

  If RED identifies a code defect, change only that authority seam. If the
  test is green and the executable audit identifies an old binary, add a
  visible build-identity diagnostic to the test/run helper rather than
  changing mouse semantics.

- [ ] **Step 4: Verify focused and regression families**

  Run:

  ```bash
  export PATH="$HOME/.local/bin:$PATH"
  cargo nextest run --locked --no-fail-fast sidebar_shortcut_mouse clicking_file_sidebar_item sidebar_navigation
  ```

  Expected: click, scheduled consumption, stale fail-closed, singleton
  generation, and prior Files-tab activation families pass.

- [ ] **Step 5: Commit only if production changed**

  RED subject:

  ```text
  test: reproduce files shortcut mouse regression
  ```

  GREEN subject:

  ```text
  fix: restore files shortcut mouse navigation
  ```

### Task 4: Define the native preview capability matrix

**Files:**

- Create: `src/fm/preview_capability.rs`
- Modify: `src/fm/mod.rs`
- Modify: `src/fm/trail_snapshots.rs`
- Modify: `src/ui/file_manager/trail_view.rs`
- Test: `src/fm/preview_capability.rs`

**Interfaces:**

- Consumes: exact `FileEntryKind`, path name/extension, existing bounded text
  preview, image preview worker, and `TrailDetailPreview`.
- Produces: pure `PreviewCapability` and prepared provider choice; no command
  execution from render.

- [ ] **Step 1: Write the capability RED table**

  Cover directory, UTF-8 text, source/config, Markdown, recognized image,
  PDF, office document, archive, audio, video, generic binary, broken symlink,
  special file, non-UTF-8 path, control path, oversized text, missing optional
  provider, and unsupported platform.

- [ ] **Step 2: Run the table**

  Run:

  ```bash
  export PATH="$HOME/.local/bin:$PATH"
  cargo nextest run --locked --no-fail-fast preview_capability
  ```

  Expected: fail because current `prepare_trail_detail` only distinguishes
  recognized image extension from “try bounded text”.

- [ ] **Step 3: Add a pure provider decision**

  Model at least:

  - `NativeText`;
  - `NativeImage`;
  - `MetadataOnly`;
  - `OptionalPlugin { action_id }`;
  - `Unsupported { reason }`.

  Classification uses prepared kind/name and an injected capability set. It
  does not call `which`, read config, spawn a process, or inspect filesystem
  metadata.

- [ ] **Step 4: Preserve native fallback**

  Missing plugin/provider maps to bounded native text or metadata-only status.
  No provider may change cwd, Trail selection, generation, or terminal state.

- [ ] **Step 5: Verify and commit**

  Run:

  ```bash
  export PATH="$HOME/.local/bin:$PATH"
  cargo nextest run --locked --no-fail-fast preview_capability text_preview image_preview trail_detail
  ```

  Expected: capability table and existing preview families pass.

  Commit subjects:

  ```text
  test: define files preview capabilities
  feat: add bounded files preview capability selection
  ```

### Task 5: Add an optional plugin preview adapter

**Files:**

- Create: `src/app/file_preview_plugin.rs`
- Modify: `src/app/file_manager_plugin.rs`
- Modify: `src/app/state.rs`
- Modify: `src/app/runtime.rs`
- Test: `src/app/file_preview_plugin.rs`

**Interfaces:**

- Consumes: Task 4 `OptionalPlugin { action_id }`, current exact selected path,
  Files generation, plugin action inventory, and existing plugin invocation
  worker.
- Produces: a bounded explicit “Open in…” intent; heavyweight preview opens in
  a plugin pane and never injects renderer output into native Ratatui cells.

- [ ] **Step 1: Write fail-closed REDs**

  Cover missing/disabled plugin, stale selection, changed generation,
  unsupported platform, malformed action metadata, worker busy, launch
  failure, completion after Files close/reopen, and duplicate activation.

- [ ] **Step 2: Implement one-shot exact-context invocation**

  Snapshot exact path, Files generation, plugin/action identity, workspace,
  pane, and terminal identity where available. Revalidate before dispatch.
  Use the existing bounded plugin worker; no retry queue.

- [ ] **Step 3: Keep native state unchanged**

  Success may open/focus a plugin-owned pane. Failure produces one bounded
  status message. Neither result mutates Trail columns, cwd, selection, hidden
  policy, or terminal runtime identity.

- [ ] **Step 4: Verify Hunk is not selected as a file preview provider**

  Fixture manifests must classify:

  - `herdr-plugin-hunk`: diff workflow, not preview provider;
  - `herdr-file-viewer`: expert file-view action candidate;
  - `herdr-quicklook`: explicit quick-preview candidate;
  - missing license metadata: warning in research/selection evidence, never
    silent endorsement.

- [ ] **Step 5: Verify and commit**

  Run:

  ```bash
  export PATH="$HOME/.local/bin:$PATH"
  cargo nextest run --locked --no-fail-fast file_preview_plugin file_manager_plugin
  ```

  Expected: bounded dispatch and every stale/failure case pass.

  Commit subjects:

  ```text
  test: pin files preview plugin boundaries
  feat: open optional files previews through plugins
  ```

### Task 6: Visual, isolated runtime and closure gates

**Files:**

- Modify: `src/ui/visual_fixture.rs`
- Create: `tests/visual/files-visibility-preview.spec.ts`
- Create:
  `tests/visual/files-visibility-preview.spec.ts-snapshots/vis-files-visibility-preview-chromium-linux.png`
- Create: `.local/herdr-files-visibility-test.sh`
- Modify: `.codex/evidence/files-visibility-runtime-matrix.md`

**Interfaces:**

- Consumes: Tasks 1-5.
- Produces: deterministic Chromium fixtures and one cleanup-first,
  cleanup-last isolated diagnostic runner.

- [ ] **Step 1: Export exact Ratatui fixtures**

  Include genuine empty, hidden-only status, partial enumeration warning,
  permission/unavailable status, six-deep branch, loaded sidebar shortcut,
  text, image, metadata-only, optional-plugin, and provider-failure panels.

- [ ] **Step 2: Add Chromium snapshots and mutation proof**

  Run from `tests/visual`:

  ```bash
  npx playwright test files-visibility-preview.spec.ts --update-snapshots
  npx playwright test files-visibility-preview.spec.ts
  ```

  Expected: spec-scoped baseline creation then green. A controlled one-cell
  mutation must fail before restoration.

- [ ] **Step 3: Add the isolated runner**

  The script must:

  1. verify its owner marker;
  2. semantically stop only its own prior test server if present;
  3. remove only its own throwaway root;
  4. unset all six inherited `HERDR_*` identity/socket variables;
  5. record executable path, Git SHA, XDG root, socket, and log path;
  6. start the test-owned debug server;
  7. run the mouse/directory matrix;
  8. semantically stop that server;
  9. verify zero owned residue.

- [ ] **Step 4: Run full gates**

  Run format separately and verify exit code before any commit:

  ```bash
  export PATH="$HOME/.local/bin:$PATH"
  cargo fmt --check
  ```

  Then:

  ```bash
  export PATH="$HOME/.local/bin:$PATH"
  cargo nextest run --locked --no-fail-fast
  cargo clippy --all-targets --locked -- -D warnings
  LIBGHOSTTY_VT_SIMD=false cargo clippy --bin herdr --target x86_64-pc-windows-msvc --locked
  ```

  Run `npx playwright test` from `tests/visual` and the current Bun/Python
  maintenance families from `just check` or their exact recipe equivalent.

- [ ] **Step 5: Rank scroll versions only after common evidence**

  Score all four lab checkpoints using the matrix in
  `.codex/evidence/miller-scroll-version-lab/README.md`. Record raw test
  evidence, weighted score, rejected tradeoffs, and the selected production
  behavior. Do not choose by recency.

- [ ] **Step 6: Continuity, graph and publication**

  Update canonical task/hand-off state, refresh Codebase Memory with the
  single-worker CLI route, re-read changed symbols, target-stage only owned
  files, fetch/prove fast-forward ancestry, push only CyPack refs, and verify
  exact SHA equality.
