# Herdr Files Interaction Polish Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix four visible Files defects — sidebar Files click not opening Native Files Stage, resident Miller columns highlighting row zero instead of the entered child, missing semantic entry kinds/icons, and a submit-appending implicit-split agent handoff — as the approved `Add Reference to Agent...` no-submit program.

**Architecture:** Every correction converges on an existing owner instead of adding a framework: sidebar navigation routes into `StageState::activate_files`/`close_files` through `AppState::activate_dock_app`; Miller focus binds `MillerPathSegment.focused_child` path identity; entry semantics become one canonical `FileEntryKind` prepared in `read_directory_snapshot`; delivery reuses `try_send_terminal_input` with an exact-path payload and an explicit live-agent picker built from `agent_panel_entries`. Playwright Chromium renders exported Ratatui `TestBackend` cells as the visual oracle; Rust/PTY tests keep semantic authority.

**Tech Stack:** Rust (ratatui, tokio, cargo-nextest), test-only Node + Playwright Chromium under `tests/visual/`, existing Bun/Python maintenance gates.

**Product contract:** `docs/superpowers/specs/2026-07-17-herdr-files-interaction-polish-design.md` (approved, drag-and-drop excluded).

---

## Verified Current-Owner Map (graph-verified 2026-07-18, 21,064 nodes / 98,009 edges)

| Seam | Current owner (exact) | Target owner |
|---|---|---|
| Sidebar tab click | `AppState::handle_mouse`, `src/app/input/mouse.rs:553-564` — `self.sidebar_tab = tab;` only | same route, plus Files→`activate_dock_app(Files)`, Spaces/Projects→`StageState::close_files` |
| Files Stage activation | `AppState::activate_dock_app` (`src/app/actions.rs:556`), `StageState::activate_files` (`src/ui/surface_host.rs:117`), `StageState::close_files` (`:144`) | unchanged; gains the sidebar caller |
| Resident Miller focus | `MillerPathSegment` (`src/fm/miller.rs:35-41`) — `focused_child: Option<PathBuf>` declared, set to `None` in `new` (`:47`), never populated; `MillerState::visit` moves the departing projection without binding; resident render reads `column.cursor` (`src/ui/file_manager.rs:721,748`) | new `MillerState::bind_focused_child` + unique-path re-resolution in the resident projection consumer |
| Entry semantics | `entry_capabilities(&DirEntry) -> (bool, bool)` (`src/fm/mod.rs`), `FileEntry { name, path, is_dir, operation_supported }` (`src/fm/mod.rs:40-49`), `read_directory_snapshot` (`src/fm/mod.rs`), `render_entry_row` (`src/ui/file_manager.rs`) | `FileEntryKind` enum + derived capabilities + pure visual classifier `src/fm/entry_kind.rs`, icon render in `render_entry_row` |
| Agent handoff | `App::prepare_file_manager_agent_handoff` (`src/app/file_agent_handoff.rs:61-122`) — builds `FileManagerAgentHandoffRequest { path, terminal_id }` or implicit `FileManagerClaudeSplitRequest`; `App::sync_file_manager_agent_handoff_send` (`:158-189`) — `payload.push(b'\r')`; `App::try_send_terminal_input` (`:140-156`) — bounded one-shot; revalidation `file_manager_agent_handoff_is_current` (`:518-566`) | `AgentReferenceRequest` with workspace/pane/terminal identity, exact-path payload, no split branch, explicit picker |
| Visible copy | `"Send to Agent"` at `src/app/state.rs:948` (`FileManagerContextMenuAction` label), test pins at `state.rs:3807,3950`, menu item at `src/ui/file_manager.rs:2574`; row tag `FileManagerRowAction::SendAgent` glyph `>` (`state.rs:640-650`) | `"Add Reference to Agent..."` everywhere |
| Live agents projection | `agent_panel_entries(app) -> Vec<AgentPanelEntry>` (`src/ui/sidebar.rs`) | unchanged; picker consumer |

## Test-Point Coverage Map (57 unique `TP-FIP-*` IDs)

Continuity records "55 unique"; a fresh deterministic count of the design finds 57 (NAV 8, FOCUS 10, ICON 13, REF 18, VIS 6, E2E 2 — the 55 figure excludes the two E2E IDs). This plan covers all 57; none is dropped.

| IDs | Task(s) |
|---|---|
| TP-FIP-NAV-01, NAV-02 | Task 6 |
| TP-FIP-NAV-03 | Task 7 |
| TP-FIP-NAV-04..NAV-08 | Task 8 |
| TP-FIP-FOCUS-01 | Task 10 |
| TP-FIP-FOCUS-03, FOCUS-04, FOCUS-10 | Task 11 |
| TP-FIP-FOCUS-02, FOCUS-05..FOCUS-09 | Task 12 |
| TP-FIP-ICON-01..ICON-05 (kind) | Task 15 |
| TP-FIP-ICON-12 (parity) | Task 16 |
| TP-FIP-ICON-02, ICON-06, ICON-07, ICON-10 (classes/glyphs) | Task 17 |
| TP-FIP-ICON-08, ICON-09, ICON-11, ICON-13 | Task 18 |
| TP-FIP-REF-05, REF-07 | Task 20 |
| TP-FIP-REF-06, REF-11, REF-12, REF-13, REF-18 | Task 21 |
| TP-FIP-REF-03 (no split) | Task 22 |
| TP-FIP-REF-08, REF-09, REF-10, REF-14, and exact-once/zero-retry | Task 23 |
| TP-FIP-REF-01, REF-02, REF-04 | Task 24 |
| TP-FIP-REF-15, REF-16, REF-17 | Task 25 |
| (picker disappearance) part of REF-08/09 UI side | Task 26 |
| TP-FIP-VIS-01 | Task 9 |
| TP-FIP-VIS-02 | Task 13 |
| TP-FIP-VIS-03, VIS-04 | Task 19 |
| TP-FIP-VIS-05, VIS-06 | Task 28 |
| TP-FIP-E2E-01 | Task 9 |
| TP-FIP-E2E-02 | Task 29 |

## Gate Commands (justfile-equivalent; `just` binary is absent on this machine — run recipes directly)

```bash
# focused (replace FILTER)
cargo nextest run --locked -E 'test(/FILTER/)' --status-level fail --final-status-level fail --failure-output final --success-output never
# full Rust
cargo nextest run --locked --status-level fail --final-status-level fail --failure-output final --success-output never
# lint
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
# canonical Windows lint
LIBGHOSTTY_VT_SIMD=false cargo clippy --bin herdr --locked --target x86_64-pc-windows-msvc -- -D warnings
# maintenance
python3 -m unittest scripts.test_agent_detection_manifest_check scripts.test_changelog scripts.test_docs_translation_parity scripts.test_preview scripts.test_vendor_libghostty_vt scripts.test_vendor_portable_pty
bun test src/integration/assets/herdr-agent-state.test.ts
cd workers/plugin-marketplace && bun test
# visual (after Task 3)
cd tests/visual && npx playwright test
```

Expected full-suite baseline at plan time: 3,443 passed + 1 named skip (`path_beta_real_host_probe`). Every new task adds to that count; a zero-test filter never counts as evidence.

## Git Discipline (every task)

- RED and GREEN are separate commits; refactor separate again; never push a RED head.
- Stage exact paths only (`git add -- <paths>`); never `git add -A`; `.superpowers/` and `.local/` never staged.
- Push only `origin HEAD:feat/native-fm` and `origin HEAD:master` (CyPack fork), fast-forward, after phase gates; verify remote SHAs equal local HEAD. Never push `upstream`.
- Rollback boundary: each task's commits revert independently; the icon migration (Tasks 15-16) reverts as one characterized unit.

---

### Task 1: FIP-0.1 — Freeze baseline characterization and graph evidence

**Files:**
- Create: `.codex/evidence/fip-baseline-freeze.md`

- [ ] **Step 1: Run the design's 10-test characterization set and the full suite**

```bash
cargo nextest run --locked -E 'test(/sidebar_tab|activate_dock|current_row_mouse|directory_chain|agent_handoff|attachment_target|symlink|special_entry|truncat|unicode_row_action/)' --status-level fail --final-status-level fail --failure-output final --success-output never
cargo nextest run --locked --status-level fail --final-status-level fail --failure-output final --success-output never
```

Expected: all selected pass; full suite 3,443 passed + 1 skip. Record the exact run IDs.

- [ ] **Step 2: Record graph freshness**

Verify `index_status(project="home-ayaz-projects-herdr")` still reports 21,064/98,009 (or refresh with `CBM_WORKERS=1 codebase-memory-mcp cli index_repository '{"repo_path":"/home/ayaz/projects/herdr","mode":"fast","persistence":false}'`) and that `focused_child`, `sync_file_manager_agent_handoff_send`, and `activate_files` resolve from current source.

- [ ] **Step 3: Write `.codex/evidence/fip-baseline-freeze.md`** with run IDs, counts, and graph numbers; commit:

```bash
git add -- .codex/evidence/fip-baseline-freeze.md
git commit -m "docs: freeze files interaction polish baseline"
```

### Task 2: FIP-0.3 — Test-only Ratatui cell-fixture exporter

**Files:**
- Create: `src/ui/visual_fixture.rs` (module gated `#[cfg(test)]` via `src/ui/mod.rs`)
- Modify: `src/ui/mod.rs` (add `#[cfg(test)] pub(crate) mod visual_fixture;`)

- [ ] **Step 1: Write the failing test** in `src/ui/visual_fixture.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exported_fixture_serializes_every_cell_with_style() {
        let backend = ratatui::backend::TestBackend::new(4, 2);
        let mut terminal = ratatui::Terminal::new(backend).expect("test terminal");
        terminal
            .draw(|f| {
                let p = ratatui::widgets::Paragraph::new("ab")
                    .style(ratatui::style::Style::default().fg(ratatui::style::Color::Rgb(1, 2, 3)));
                f.render_widget(p, ratatui::layout::Rect::new(0, 0, 2, 1));
            })
            .expect("draw");
        let fixture = export_cell_fixture("nav-01", terminal.backend().buffer());
        assert_eq!(fixture.width, 4);
        assert_eq!(fixture.height, 2);
        assert_eq!(fixture.name, "nav-01");
        assert_eq!(fixture.cells.len(), 8);
        assert_eq!(fixture.cells[0].symbol, "a");
        assert_eq!(fixture.cells[0].fg, "rgb(1,2,3)");
    }
}
```

- [ ] **Step 2: Run to verify it fails**

```bash
cargo nextest run --locked -E 'test(/exported_fixture_serializes_every_cell_with_style/)' --status-level fail --final-status-level fail --failure-output final --success-output never
```

Expected: compile error is NOT valid RED here — this is a new test-only module, so first make it compile with `export_cell_fixture` returning an empty `CellFixture`, then observe the assertion failure `cells.len() == 0` vs 8. That assertion failure is the RED evidence.

- [ ] **Step 3: Implement the exporter** (same file):

```rust
#[derive(serde::Serialize)]
pub(crate) struct FixtureCell {
    pub symbol: String,
    pub fg: String,
    pub bg: String,
    pub modifiers: Vec<String>,
    pub x: u16,
    pub y: u16,
}

#[derive(serde::Serialize)]
pub(crate) struct CellFixture {
    pub name: String,
    pub width: u16,
    pub height: u16,
    pub cells: Vec<FixtureCell>,
}

pub(crate) fn export_cell_fixture(name: &str, buffer: &ratatui::buffer::Buffer) -> CellFixture {
    let area = buffer.area();
    let mut cells = Vec::with_capacity(usize::from(area.width) * usize::from(area.height));
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            let cell = &buffer[(x, y)];
            cells.push(FixtureCell {
                symbol: cell.symbol().to_string(),
                fg: format_color(cell.fg),
                bg: format_color(cell.bg),
                modifiers: format_modifiers(cell.modifier),
                x,
                y,
            });
        }
    }
    CellFixture { name: name.to_string(), width: area.width, height: area.height, cells }
}
```

`format_color` maps `Color::Rgb(r,g,b)` to `rgb(r,g,b)` and named palette colors to their names; `format_modifiers` lists set `Modifier` flags as lowercase strings. Write both as small exhaustive-match helpers in the same file (compile-checked; no `unwrap()`).

An `#[ignore]`d test `write_visual_fixtures` renders the real Files UI states needed by Tasks 9/13/19/28 through the existing runtime-render fixture pattern (`#[tokio::test]`, see `TerminalRuntime::test_with_screen_bytes` precedent in lessons) and serializes JSON only into a caller-provided directory from env `HERDR_VISUAL_FIXTURE_DIR`; missing env → explicit panic message, never a default path.

- [ ] **Step 4: Run tests** — same filter; expected PASS. Then full FM/ui family regression:

```bash
cargo nextest run --locked -E 'test(/visual_fixture/)' --status-level fail --final-status-level fail --failure-output final --success-output never
```

- [ ] **Step 5: Commit**

```bash
git add -- src/ui/visual_fixture.rs src/ui/mod.rs
git commit -m "test: add ratatui cell fixture exporter"
```

### Task 3: FIP-0.2 — Isolated Playwright Chromium package

**Files:**
- Create: `tests/visual/package.json`, `tests/visual/package-lock.json`, `tests/visual/playwright.config.ts`, `tests/visual/.gitignore`

- [ ] **Step 1: Create the package** (exact contents):

`tests/visual/package.json`:

```json
{
  "name": "herdr-visual-tests",
  "private": true,
  "devDependencies": {
    "@playwright/test": "1.54.1"
  },
  "scripts": {
    "test": "playwright test"
  }
}
```

`tests/visual/playwright.config.ts`:

```ts
import { defineConfig, devices } from "@playwright/test";

export default defineConfig({
  testDir: ".",
  workers: 1,
  retries: process.env.CI ? 1 : 0,
  forbidOnly: true,
  updateSnapshots: "none",
  use: {
    ...devices["Desktop Chrome"],
    viewport: { width: 1280, height: 800 },
    deviceScaleFactor: 1,
    locale: "en-US",
    timezoneId: "UTC",
    colorScheme: "dark",
    contrast: "no-preference",
    reducedMotion: "reduce",
    screenshot: "only-on-failure",
    trace: "on-first-retry",
  },
  expect: { toHaveScreenshot: { maxDiffPixels: 0 } },
  outputDir: "./test-results",
});
```

`tests/visual/.gitignore`:

```text
node_modules/
test-results/
playwright-report/
fixtures/generated/
```

- [ ] **Step 2: Install with lockfile and browser** (test-only; never touches the Rust product):

```bash
cd tests/visual && npm install && npx playwright install chromium
```

Expected: `package-lock.json` created; Chromium downloads. If the machine blocks installs, STOP and report (supply-chain rule: lockfile committed, exact-pinned version).

- [ ] **Step 3: Commit**

```bash
git add -- tests/visual/package.json tests/visual/package-lock.json tests/visual/playwright.config.ts tests/visual/.gitignore
git commit -m "test: add isolated playwright chromium harness"
```

### Task 4: FIP-0.4 — Deterministic browser cell-grid renderer + self-tests

**Files:**
- Create: `tests/visual/harness/grid.html`, `tests/visual/harness/grid.js`, `tests/visual/harness.spec.ts`
- Create: `tests/visual/fixtures/self-test.json` (hand-written, small)

- [ ] **Step 1: Write the renderer.** `grid.html` loads `grid.js`; `grid.js` exposes `window.renderFixture(fixture)` that builds a CSS grid of `span` cells (one per fixture cell, `grid-column: x+1; grid-row: y+1`, monospace `"DejaVu Sans Mono"`, fixed `16px/1` metrics, `white-space: pre`), sets `color`/`background` from `fg`/`bg`, and maps modifiers `bold`→`font-weight:700`, `italic`, `underlined`→`text-decoration:underline`, `dim`→`opacity:.6`, `reversed`→swap fg/bg. Malformed input (missing `width`/`height`/`cells`, cell count ≠ width×height) throws with a visible `#error` element. No product logic is copied.

- [ ] **Step 2: Write self-tests** `harness.spec.ts`:

```ts
import { test, expect } from "@playwright/test";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const harness = fileURLToPath(new URL("./harness/grid.html", import.meta.url));
const load = (name: string) =>
  JSON.parse(readFileSync(new URL(`./fixtures/${name}.json`, import.meta.url), "utf8"));

test("every cell maps to exactly one grid position", async ({ page }) => {
  await page.goto(`file://${harness}`);
  const fixture = load("self-test");
  await page.evaluate((f) => (window as any).renderFixture(f), fixture);
  await expect(page.locator(".cell")).toHaveCount(fixture.cells.length);
});

test("wide symbol cannot shift following cells", async ({ page }) => {
  await page.goto(`file://${harness}`);
  const fixture = load("self-test"); // contains one 全 wide glyph followed by "x"
  await page.evaluate((f) => (window as any).renderFixture(f), fixture);
  const boxes = await page.locator(".cell").evaluateAll((els) =>
    els.map((e) => (e as HTMLElement).getBoundingClientRect().left));
  expect(boxes[2] - boxes[1]).toBeCloseTo(boxes[1] - boxes[0], 1);
});

test("malformed fixture fails loudly", async ({ page }) => {
  await page.goto(`file://${harness}`);
  await expect(
    page.evaluate(() => (window as any).renderFixture({ width: 2 }))
  ).rejects.toThrow();
});

test("styles round-trip", async ({ page }) => {
  await page.goto(`file://${harness}`);
  await page.evaluate((f) => (window as any).renderFixture(f), load("self-test"));
  await expect(page.locator(".cell").first()).toHaveCSS("color", "rgb(1, 2, 3)");
});
```

`fixtures/self-test.json` is a hand-written 3×1 fixture: cells `[{symbol:"a",fg:"rgb(1,2,3)",bg:"rgb(0,0,0)",modifiers:[],x:0,y:0},{symbol:"全",…,x:1,y:0},{symbol:"x",…,x:2,y:0}]`.

- [ ] **Step 3: Run and verify**

```bash
cd tests/visual && npx playwright test harness.spec.ts
```

Expected: 4 passed. Missing browser must fail explicitly (verify by reading output, not assuming).

- [ ] **Step 4: Commit**

```bash
git add -- tests/visual/harness tests/visual/harness.spec.ts tests/visual/fixtures/self-test.json
git commit -m "test: add deterministic browser cell grid renderer"
```

### Task 5: FIP-0.5 + FIP-0.6 — Mutation-fails proof and artifact containment

**Files:**
- Create: `tests/visual/mutation.spec.ts`

- [ ] **Step 1: Write the mutation self-test**

```ts
import { test, expect } from "@playwright/test";
// renders self-test fixture, screenshots it as baseline name "mutation-base",
// then mutates cells[0].symbol to "Z" and asserts toHaveScreenshot REJECTS.
test("a one-cell mutation fails the snapshot", async ({ page }) => {
  await page.goto(`file://${harness}`);
  const fixture = load("self-test");
  await page.evaluate((f) => (window as any).renderFixture(f), fixture);
  await expect(page.locator("#grid")).toHaveScreenshot("mutation-base.png");
  fixture.cells[0].symbol = "Z";
  await page.evaluate((f) => (window as any).renderFixture(f), fixture);
  await expect(
    expect(page.locator("#grid")).toHaveScreenshot("mutation-base.png")
  ).rejects.toThrow();
});
```

(`harness`/`load` helpers identical to Task 4 — repeat them in this file.)

- [ ] **Step 2: Generate the baseline once explicitly** (`npx playwright test mutation.spec.ts --update-snapshots`), then run without update and read PASS. Confirm `updateSnapshots: "none"` in config means ordinary runs never rewrite baselines.

- [ ] **Step 3: Verify artifact containment** — `git status --short` shows nothing outside `tests/visual/` (ignored dirs cover `test-results/`, `fixtures/generated/`).

- [ ] **Step 4: Commit**

```bash
git add -- tests/visual/mutation.spec.ts tests/visual/mutation.spec.ts-snapshots
git commit -m "test: prove one cell mutation fails visual snapshot"
```

### Task 6: FIP-1.1 RED + FIP-1.2 GREEN — Sidebar Files click opens Native Files Stage (TP-FIP-NAV-01, NAV-02)

**Files:**
- Modify: `src/app/input/mouse.rs` (sidebar tab route, ~553-564)
- Test: same file `#[cfg(test)]` tests (follow the existing sidebar-tab test family)

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn files_tab_primary_click_opens_native_files_stage() {
    let mut state = AppState::test_new();
    // arrange a desktop view with a visible sidebar and known tab rects,
    // following the existing sidebar_tab_at test fixture in this file.
    let files_rect = state.sidebar_files_tab_rect_for_test();
    let mut runtimes = TerminalRuntimeRegistry::test_new();
    state.handle_mouse(&mut runtimes, left_down_at(files_rect.x, files_rect.y));
    assert_eq!(state.sidebar_tab, SidebarTab::Files);
    assert_eq!(
        state.stage.surface_view(),
        crate::ui::surface_host::StageSurfaceView::NativeFiles
    );
    assert!(state.file_manager.is_some());
}

#[test]
fn files_tab_click_reuses_open_singleton_files_stage() {
    let mut state = AppState::test_new();
    state.activate_dock_app(crate::ui::surface_host::BuiltInAppId::Files);
    let generation_before = state.stage.active_files_generation_for_test();
    let files_rect = state.sidebar_files_tab_rect_for_test();
    let mut runtimes = TerminalRuntimeRegistry::test_new();
    state.handle_mouse(&mut runtimes, left_down_at(files_rect.x, files_rect.y));
    assert_eq!(state.stage.active_files_generation_for_test(), generation_before);
    assert!(state.file_manager.is_some());
}
```

Fixture helpers (`left_down_at`, tab-rect helper) must reuse the existing sidebar mouse test helpers in `src/app/input/mouse.rs`; if a tab-rect helper does not exist, derive the coordinate the same way the existing `sidebar_tab_at` tests do. If `active_files_generation_for_test` does not exist, assert singleton reuse through the SF4.1 seam used by `reactivating_singleton_files_keeps_one_surface` (`src/ui/surface_host.rs` tests) instead — do not invent a new production accessor for the test.

- [ ] **Step 2: Run RED**

```bash
cargo nextest run --locked -E 'test(/files_tab_primary_click_opens_native_files_stage|files_tab_click_reuses_open_singleton_files_stage/)' --status-level fail --final-status-level fail --failure-output final --success-output never
```

Expected RED: first test fails on `surface_view() == NativeFiles` (actual: `TerminalWorkspace`) because the route only assigns `sidebar_tab`. That is behavior failure, not compile failure. The singleton test may already pass via `activate_dock_app` — if it passes before the change, keep it as characterization and say so in the commit body.

- [ ] **Step 3: Commit RED**

```bash
git add -- src/app/input/mouse.rs
git commit -m "test: pin default files navigation"
```

- [ ] **Step 4: Implement minimum GREEN** — in the sidebar-tab branch of `handle_mouse`:

```rust
if let Some(tab) = self.sidebar_tab_at(mouse.column, mouse.row) {
    self.sidebar_tab = tab;
    if tab == crate::app::state::SidebarTab::Files {
        self.activate_dock_app(crate::ui::surface_host::BuiltInAppId::Files);
    } else {
        self.request_file_manager_sidebar_navigation = None;
    }
    return None;
}
```

`activate_dock_app` already owns open-or-reuse (`StageState::activate_files` launch policy) and fail-closed preparation; no second opener is added. Preserve the existing I/O-free comment block.

- [ ] **Step 5: Run GREEN + regression**

```bash
cargo nextest run --locked -E 'test(/files_tab|sidebar_tab|activate_dock/)' --status-level fail --final-status-level fail --failure-output final --success-output never
```

Expected: all pass, including the pre-existing sidebar tab tests.

- [ ] **Step 6: Commit GREEN**

```bash
git add -- src/app/input/mouse.rs
git commit -m "fix: open files from visible navigation"
```

### Task 7: FIP-1.3 RED + FIP-1.4 GREEN — Spaces/Projects restore Terminal Stage (TP-FIP-NAV-03)

**Files:**
- Modify: `src/app/input/mouse.rs`
- Test: same file

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn spaces_tab_click_restores_terminal_stage_and_preserves_runtime_identity() {
    let mut state = AppState::test_new();
    state.ensure_test_terminals();
    let terminal_ids_before: Vec<_> = state.terminals.keys().cloned().collect();
    state.activate_dock_app(crate::ui::surface_host::BuiltInAppId::Files);
    assert_eq!(state.stage.surface_view(), StageSurfaceView::NativeFiles);
    let spaces_rect = /* Spaces tab coordinate, same fixture helper as Task 6 */;
    let mut runtimes = TerminalRuntimeRegistry::test_new();
    state.handle_mouse(&mut runtimes, left_down_at(spaces_rect.x, spaces_rect.y));
    assert_eq!(state.sidebar_tab, SidebarTab::Spaces);
    assert_eq!(state.stage.surface_view(), StageSurfaceView::TerminalWorkspace);
    let terminal_ids_after: Vec<_> = state.terminals.keys().cloned().collect();
    assert_eq!(terminal_ids_before, terminal_ids_after);
}
```

Add the symmetric `projects_tab_click_restores_terminal_stage` variant. Runtime-identity preservation across stage switches already has the SF4.1-08 precedent (`stage_surface_switch_does_not_destroy_terminal_runtime`) — extend with the sidebar route, do not duplicate the runtime fixture.

- [ ] **Step 2: Run RED** — filter `test(/spaces_tab_click_restores|projects_tab_click_restores/)`. Expected failure: `surface_view()` stays `NativeFiles` because the route never calls `close_files`.

- [ ] **Step 3: Commit RED** — `git commit -m "test: pin terminal stage restore from sidebar"` (stage `src/app/input/mouse.rs`).

- [ ] **Step 4: GREEN** — extend the Task 6 branch:

```rust
if tab == crate::app::state::SidebarTab::Files {
    self.activate_dock_app(crate::ui::surface_host::BuiltInAppId::Files);
} else {
    self.request_file_manager_sidebar_navigation = None;
    if self.stage.surface_view() == crate::ui::surface_host::StageSurfaceView::NativeFiles {
        self.close_files_stage_client_locally();
    }
}
```

**Verified owner (FIP-G.2):** the shared authority is `activate_dock_app(crate::ui::surface_host::BuiltInAppId::Terminal)` (`src/app/actions.rs:556` — doc comment: "Terminal restores the terminal stage. Shared by the dock left-click and the popover activation so both paths stay identical"). Its close seam is `AppState::close_file_manager` (`src/app/actions.rs:522`), which already performs `stage.close_files()`, FM teardown, and projected-geometry retirement in one transaction. Route the sidebar Spaces/Projects branch through `activate_dock_app(Terminal)` so all three activation surfaces (dock click, popover, sidebar tab) stay on one authority:

```rust
} else {
    self.request_file_manager_sidebar_navigation = None;
    if self.stage.surface_view() == crate::ui::surface_host::StageSurfaceView::NativeFiles {
        self.activate_dock_app(crate::ui::surface_host::BuiltInAppId::Terminal);
    }
}
```

- [ ] **Step 5: Run GREEN + regression** — filter `test(/spaces_tab|projects_tab|closing_files|stage_surface_switch/)`; expected all pass.

- [ ] **Step 6: Commit** — `git commit -m "fix: restore terminal stage from spaces and projects"`.

### Task 8: FIP-1.5 — Navigation edge family (TP-FIP-NAV-04..NAV-08)

**Files:**
- Test: `src/app/input/mouse.rs`

- [ ] **Step 1: Write the family tests** (names exact):

```text
modified_or_middle_click_on_files_tab_does_not_activate_stage
release_only_event_on_files_tab_does_not_activate_stage
outside_click_near_files_tab_does_not_activate_stage
overlay_owner_blocks_files_tab_activation            // open Mode::ContextMenu first; SF4.2-02 precedent
capture_owner_blocks_files_tab_activation            // begin a divider ResizeTransaction capture first
files_sidebar_path_click_navigates_once_after_activation   // NAV-06: request_file_manager_sidebar_navigation consumed once
stale_sidebar_navigation_generation_fails_closed     // NAV-07: reuse C6.1 stale-model fixture pattern
collapsed_sidebar_hides_files_tab_target             // NAV-08: sidebar_collapsed = true → sidebar_tab_at returns None
tiny_terminal_files_tab_is_inert_when_hidden         // NAV-08: degenerate geometry
```

Each asserts `surface_view()` unchanged and `sidebar_tab` unchanged (except NAV-06 which asserts exactly one navigation). Use the existing overlay fixtures from SF4.2 slices 02/03 and the C6.1 scheduled-navigation fixtures — the assertions are new (Stage must not transition), the arrangement is existing.

- [ ] **Step 2: Run** — filter `test(/files_tab_activation|files_tab_target|files_sidebar_path_click|stale_sidebar_navigation/)`. Some of these may pass immediately (the router already blocks overlays per SF4.2) — passing-before-change tests are recorded as characterization in the commit message; any FAILING one exposes a real leak and gets its own minimal GREEN in this task following the Task 6 route (fail-closed consumption, no new authority).

- [ ] **Step 3: Commit** — `git commit -m "test: characterize files navigation edge ownership"` (plus a separate `fix:` commit only if a leak was found).

### Task 9: FIP-1.6 — VIS-01 snapshot + E2E-01 isolated mouse smoke

**Files:**
- Create: `tests/visual/navigation.spec.ts`; fixture export added to the Task 2 `write_visual_fixtures` ignored test
- Evidence: `.codex/evidence/fip-progress.md` (append)

- [ ] **Step 1: Export fixtures** — run the ignored exporter for two states: default terminal Stage, and post-`activate_dock_app(Files)` Stage (ASCII icon profile, 120×40):

```bash
HERDR_VISUAL_FIXTURE_DIR=tests/visual/fixtures/generated cargo test write_visual_fixtures -- --ignored --nocapture
```

- [ ] **Step 2: Write `navigation.spec.ts`** rendering both fixtures and asserting `toHaveScreenshot("vis-01-terminal.png")` / `"vis-01-files.png"`; generate baselines explicitly once; commit specs + baselines (`git commit -m "test: add files chromium visual oracle"` — first use of the visual oracle).

- [ ] **Step 3: E2E-01 isolated mouse smoke.** Read `.local/ISOLATED-DEV-TEST.md` completely first. Use the golden-path recipe (unique throwaway XDG config/state/runtime roots + socket, test-owned foreground `herdr server`, test-owned tmux with ALL Herdr identity/socket vars explicitly unset, debug client via `env -u HERDR_SOCKET_PATH -u HERDR_CLIENT_SOCKET_PATH cargo run`). Send one SGR mouse click at the visible Files tab coordinate, capture the pane, verify Files Stage is visible; close semantically (`prefix+q`), assert zero residue and stable socket inode/mode/mtime unchanged. Record evidence in `.codex/evidence/fip-progress.md`.

### Task 10: FIP-2.1 RED + FIP-2.2 GREEN — Bind exact child focus (TP-FIP-FOCUS-01)

**Files:**
- Modify: `src/fm/miller.rs` (new `bind_focused_child`), `src/fm/mod.rs` (`FmState::enter` call site)
- Test: `src/fm/miller.rs`, `src/fm/mod.rs`

- [ ] **Step 1: Write the failing test** in `src/fm/mod.rs` tests (real-tree fixture pattern of `navigation_visits_keep_bounded_state_and_cache_departed_projection`):

```rust
#[test]
fn entering_nonzero_child_binds_exact_focused_child_in_departing_segment() {
    let root = temp_test_root();                       // existing helper pattern
    for name in ["alpha", "beta", "gamma"] {
        std::fs::create_dir_all(root.join(name)).expect("fixture dir");
    }
    let mut state = FmState::new(root.clone());
    let beta_index = state.entries.iter().position(|e| e.path == root.join("beta")).expect("beta row");
    state.cursor = beta_index;                          // nonzero child
    state.enter();
    assert_eq!(state.cwd, root.join("beta"));
    let segment = state
        .miller
        .segment_for_directory_for_test(&root)
        .expect("departing segment");
    assert_eq!(segment.focused_child.as_deref(), Some(root.join("beta").as_path()));
}
```

Add `segment_for_directory_for_test` as a `#[cfg(test)]` accessor over the existing `chain` (test-only; no production accessor).

- [ ] **Step 2: Run RED** — filter `test(/entering_nonzero_child_binds_exact_focused_child/)`. Expected failure: `focused_child` is `None` (set only in `MillerPathSegment::new`, graph-proven no other writer). Behavior failure, not compile failure.

- [ ] **Step 3: Commit RED** — `git commit -m "test: pin resident miller child focus"` (stage `src/fm/mod.rs src/fm/miller.rs`).

- [ ] **Step 4: GREEN** — add to `MillerState`:

```rust
pub(crate) fn bind_focused_child(&mut self, directory: &Path, child: &Path) {
    if let Some(segment) = self
        .chain
        .iter_mut()
        .find(|segment| segment.directory == directory)
    {
        segment.focused_child = Some(child.to_path_buf());
    }
}
```

Call it from `FmState::enter` immediately before the existing `miller.visit(...)`/reload flow, with the pre-transition `cwd` and the entered child path. Zero new state, no extra directory read.

- [ ] **Step 5: Run GREEN + regression** — filter `test(/miller|focused_child/)`; then broad FM family `test(/fm::/)` equivalent used by prior phases. Expected all pass; existing `assert_miller_invariants_for_test` suites stay green.

- [ ] **Step 6: Commit** — `git commit -m "fix: preserve miller child focus by path"`.

### Task 11: FIP-2.3 RED + FIP-2.4 GREEN — Unique-path re-resolution (TP-FIP-FOCUS-03, 04, 10)

**Files:**
- Modify: `src/fm/miller.rs` (resolver), resident projection consumer — **verified (FIP-G.2):** resident columns render through `render_snapshot_panel(app, frame, column, &title, &resident.entries, column.cursor, ...)` in the `MillerDirectorySource::Resident(id)` arm of `src/ui/file_manager.rs` (~:706-726), where `resident = fm.miller.resident_projection(id)`. `column.cursor` comes from the prepared Miller viewport snapshot; the GREEN change makes the snapshot preparation derive that cursor from `resolve_resident_selection(segment, &resident.entries)` for resident columns (`None` → no selected row) instead of the raw cached segment cursor
- Test: `src/fm/miller.rs`, `src/ui/file_manager.rs`

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn resident_highlight_reresolves_focused_child_after_reorder() {
    // build a MillerDirectoryProjection whose entries move "beta" from index 1 to 2,
    // segment.focused_child = Some(beta), segment.cursor = 1 (stale);
    // assert resolved selection index == 2 (path wins over cached index).
}

#[test]
fn deleted_focused_child_yields_no_resident_selection() {
    // focused_child = Some(beta) but beta absent from entries;
    // assert resolved selection is None — NOT Some(0).
}

#[test]
fn duplicate_path_fixture_yields_no_selection_authority() {
    // two entries with identical path (malformed snapshot);
    // assert resolver returns None.
}
```

Write these as real code against the new pure resolver signature below (the test file defines the fixtures with `MillerPathSegment { directory, focused_child, cursor, viewport_start, preferred_width }` literals — all fields public).

- [ ] **Step 2: Run RED** — the resolver does not exist yet, so first add the minimal compiling stub returning `segment.cursor` position semantics (current behavior: `Some(cursor.min(len-1))`), then observe the reorder test failing on `Some(1) != Some(2)` and the deleted test failing on `Some(0) != None`. Those behavior failures are the RED evidence.

- [ ] **Step 3: Commit RED** — `git commit -m "test: pin resident focus re-resolution"`.

- [ ] **Step 4: GREEN** — pure resolver in `src/fm/miller.rs`:

```rust
pub(crate) fn resolve_resident_selection(
    segment: &MillerPathSegment,
    entries: &[FileEntry],
) -> Option<usize> {
    let focused = segment.focused_child.as_deref()?;
    let mut matches = entries.iter().enumerate().filter(|(_, e)| e.path == focused);
    let (index, _) = matches.next()?;
    if matches.next().is_some() {
        return None; // duplicate identity: no authority
    }
    Some(index)
}
```

Wire the resident projection consumer (the seam feeding `column.cursor` for non-current columns) to use `resolve_resident_selection` output instead of the raw cached `cursor`; `None` renders no selected row. Update derived `cursor` and clamp `viewport_start` at resolution time so the focused row is visible (FOCUS-07 groundwork).

- [ ] **Step 5: Run GREEN + regression** — filter `test(/resident|miller/)` plus the Miller production family (`test(/miller_viewport|miller_layout/)`); expected all pass with the FM1-FM4 suites unchanged.

- [ ] **Step 6: Commit** — `git commit -m "fix: resolve resident focus by unique path"`.

### Task 12: FIP-2.5 — Focus family (TP-FIP-FOCUS-02, 05, 06, 07, 08, 09)

**Files:**
- Test: `src/fm/mod.rs`, `src/fm/miller.rs`, `src/app/input/file_manager.rs`

- [ ] **Step 1: Write the family** (real-tree fixtures; names exact):

```text
four_level_descent_binds_every_resident_ancestor_focus        // FOCUS-02: a/b/c/d chain, assert each segment.focused_child
branch_change_retires_descendant_focus_atomically             // FOCUS-05: extend revisiting_ancestor_truncates_descendants fixture with focused_child assertions
leave_and_revisit_keeps_bounded_exact_focus                   // FOCUS-06: enter/leave/enter, focus + viewport bounded (LRU eviction respected)
resident_viewport_clamps_focused_row_visible                  // FOCUS-07: focused index beyond viewport_start+height → clamped
empty_root_unavailable_and_denied_parents_have_no_fabricated_focus  // FOCUS-08: reuse FmDirectoryStatus fixtures; resolver None
stale_generation_resident_row_click_is_inert                  // FOCUS-09: FM3 generation fixture; focus path and cwd unchanged
```

- [ ] **Step 2: Run, classify** — expected: FOCUS-02/05/06/07 fail RED against gaps (bind/clamp not yet covering that path) or pass as characterization; FOCUS-08/09 likely pass via existing fail-closed seams. Read each result; fix only observed failures with minimal deltas inside the Task 10/11 seams.

- [ ] **Step 3: Commits** — `test:` commit for the family; separate `fix:` commit(s) per observed failure. Run the broad FM regression + full suite before closing FIP-2.

### Task 13: FIP-2.6 — VIS-02 snapshot

**Files:**
- Modify: Task 2 exporter states; Create: `tests/visual/focus.spec.ts`

- [ ] **Step 1: Export** a three-column fixture where `beta` (index 1) was entered — resident column must show highlight on `beta`, not row 0. **Step 2:** spec + explicit baseline + run PASS. **Step 3:** commit `test: add resident focus visual proof`.

### Task 14: FIP-3.1 — Characterize current entry behavior

**Files:**
- Test: `src/fm/mod.rs`

- [ ] **Step 1: Write characterizations** (pass-before-change, explicitly labeled):

```text
sort_orders_directories_first_then_name          // pins sort_entries current order
symlink_to_directory_currently_lists_as_directory
broken_symlink_currently_lists_as_unsupported_file
operation_supported_gates_row_and_context_actions
watcher_reload_preserves_selected_path            // existing A4 behavior, re-pinned at entry level
```

- [ ] **Step 2: Run (all must PASS)** — these freeze behavior the Task 15/16 migration must preserve byte-for-byte (ICON-12). Commit `test: characterize file entry semantics`.

### Task 15: FIP-3.2 RED + FIP-3.3 GREEN — `FileEntryKind` (TP-FIP-ICON-01..05)

**Files:**
- Create: `src/fm/entry_kind.rs`; Modify: `src/fm/mod.rs` (`FileEntry`, `read_directory_snapshot`, `entry_capabilities`)

- [ ] **Step 1: Write the failing classification tests** (real filesystem, Unix-gated where symlinks are used):

```rust
#[test]
fn snapshot_classifies_all_six_entry_kinds() {
    let root = temp_test_root();
    std::fs::create_dir(root.join("dir")).expect("dir");
    std::fs::write(root.join("file.txt"), b"x").expect("file");
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(root.join("dir"), root.join("link-dir")).expect("link-dir");
        std::os::unix::fs::symlink(root.join("file.txt"), root.join("link-file")).expect("link-file");
        std::os::unix::fs::symlink(root.join("missing"), root.join("broken")).expect("broken");
        nix_mkfifo_or_skip(root.join("fifo"));   // use libc mkfifo via existing test util or std process `mkfifo`
    }
    let snapshot = read_directory_snapshot(&root, false);
    let kind_of = |name: &str| snapshot.entries.iter().find(|e| e.name == name).map(|e| e.kind);
    assert_eq!(kind_of("dir"), Some(FileEntryKind::Directory));
    assert_eq!(kind_of("file.txt"), Some(FileEntryKind::RegularFile));
    #[cfg(unix)]
    {
        assert_eq!(kind_of("link-dir"), Some(FileEntryKind::SymlinkDirectory));
        assert_eq!(kind_of("link-file"), Some(FileEntryKind::SymlinkFile));
        assert_eq!(kind_of("broken"), Some(FileEntryKind::BrokenSymlink));
        assert_eq!(kind_of("fifo"), Some(FileEntryKind::UnsupportedSpecial));
    }
}
```

Plus capability-derivation tests: `broken_symlink_and_special_disable_operations_and_reference`, `symlink_kinds_keep_native_operations_enabled`.

- [ ] **Step 2: Run RED** — add the enum + `kind` field as a compiling stub (`kind: FileEntryKind::RegularFile` default) so the failure is behavioral: `Directory != RegularFile`. Commit RED `test: pin file entry semantic kinds`.

- [ ] **Step 3: GREEN** — `src/fm/entry_kind.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileEntryKind {
    Directory,
    RegularFile,
    SymlinkDirectory,
    SymlinkFile,
    BrokenSymlink,
    UnsupportedSpecial,
}

impl FileEntryKind {
    pub fn is_directory_target(self) -> bool {
        matches!(self, Self::Directory | Self::SymlinkDirectory)
    }
    pub fn supports_native_operation(self) -> bool {
        !matches!(self, Self::BrokenSymlink | Self::UnsupportedSpecial)
    }
    pub fn supports_agent_reference(self) -> bool {
        self.supports_native_operation()
    }
}

pub(crate) fn classify_dir_entry(entry: &std::fs::DirEntry) -> FileEntryKind {
    match entry.file_type() {
        Ok(ft) if ft.is_symlink() => match std::fs::metadata(entry.path()) {
            Ok(target) if target.is_dir() => FileEntryKind::SymlinkDirectory,
            Ok(target) if target.is_file() => FileEntryKind::SymlinkFile,
            Ok(_) => FileEntryKind::UnsupportedSpecial,
            Err(_) => FileEntryKind::BrokenSymlink,
        },
        Ok(ft) if ft.is_dir() => FileEntryKind::Directory,
        Ok(ft) if ft.is_file() => FileEntryKind::RegularFile,
        Ok(_) | Err(_) => FileEntryKind::UnsupportedSpecial,
    }
}
```

`read_directory_snapshot` stores `kind: classify_dir_entry(&entry)` on `FileEntry` while `is_dir`/`operation_supported` remain and are asserted equal to the derived values (`kind.is_directory_target()`, `kind.supports_native_operation()`) in a bridge test — the dual fields are removed in Task 16, not here (minimum GREEN).

- [ ] **Step 4: Run GREEN + Task 14 characterizations** (must stay green — classification must not change list membership or sort). **Step 5: Commit** `feat: classify file entry semantic kinds`.

### Task 16: FIP-3.4 — Migrate consumers off dual fields (TP-FIP-ICON-12)

**Files:**
- Modify: `src/fm/mod.rs` (`FileEntry` drops stored `is_dir`/`operation_supported` in favor of `kind` + derived methods `entry.is_dir()` / `entry.operation_supported()`), every consumer (`src/ui/file_manager.rs`, `src/app/input/file_manager.rs`, `src/app/file_agent_handoff.rs`, `src/fm/delete.rs`, `src/fm/rename.rs`, others found by compile)

- [ ] **Step 1:** This is a characterized refactor behind green tests (Task 14 + Task 15 + full suite). Convert the two fields to methods delegating to `kind`; fix every compile site mechanically; grep-verify no remaining field access with at least 3 categories (`\.is_dir\b`, `operation_supported`, struct literals `is_dir:` / `operation_supported:` in tests/fixtures).
- [ ] **Step 2:** Run Task 14 characterizations, the FM broad family, and the FULL suite — byte-for-byte behavioral parity required. **Step 3: Commit** `refactor: derive file capabilities from entry kind`.

### Task 17: FIP-3.5 RED + FIP-3.6 GREEN — Visual classifier and glyph profiles (TP-FIP-ICON-02, 06, 07, 10)

**Files:**
- Modify: `src/fm/entry_kind.rs` (pure visual classifier), `src/ui/file_manager.rs` (`render_entry_row` prefix)

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn visual_class_uses_exact_name_override_before_extension() {
    assert_eq!(visual_class(FileEntryKind::RegularFile, "Dockerfile"), VisualClass::BuildPackage);
    assert_eq!(visual_class(FileEntryKind::RegularFile, ".gitignore"), VisualClass::VersionControl);
    assert_eq!(visual_class(FileEntryKind::RegularFile, "Makefile"), VisualClass::BuildPackage);
}

#[test]
fn visual_class_extension_match_is_case_insensitive() {
    assert_eq!(visual_class(FileEntryKind::RegularFile, "photo.PNG"), VisualClass::Image);
    assert_eq!(visual_class(FileEntryKind::RegularFile, "main.RS"), VisualClass::SourceCode);
}

#[test]
fn kind_always_wins_over_extension() {
    assert_eq!(visual_class(FileEntryKind::Directory, "dir.png"), VisualClass::Directory);
    assert_eq!(visual_class(FileEntryKind::BrokenSymlink, "a.rs"), VisualClass::Broken);
    assert_eq!(visual_class(FileEntryKind::UnsupportedSpecial, "b.md"), VisualClass::Special);
}

#[test]
fn no_extension_regular_file_maps_to_generic() {
    assert_eq!(visual_class(FileEntryKind::RegularFile, "README2"), VisualClass::Generic);
}

#[test]
fn every_visual_class_has_one_cell_glyph_in_both_profiles() {
    for class in VisualClass::ALL {
        for profile in [IconProfile::Nerd, IconProfile::Ascii] {
            let glyph = class.glyph(profile);
            assert_eq!(unicode_width::UnicodeWidthStr::width(glyph), 1, "{class:?} {profile:?}");
            if profile == IconProfile::Ascii {
                assert!(glyph.is_ascii(), "{class:?} ascii profile must contain no PUA glyph");
            }
        }
    }
}
```

(**Verified (FIP-G.2):** `unicode-width = "0.2"` is already a DIRECT dependency at `Cargo.toml:42` (v0.2.2 in lock). No new dependency.)

- [ ] **Step 2: Run RED** (stub `visual_class` returning `Generic` → override/extension tests fail behaviorally). Commit `test: pin files icon classification`.

- [ ] **Step 3: GREEN** — implement `VisualClass` enum with the design's 13 classes (VersionControl, BuildPackage, SourceCode, WebCode, Script, ConfigData, Document, Image, Audio, Video, Archive, Generic, plus kind-driven Directory/SymlinkDir/SymlinkFile/Broken/Special), the exact-name table and lowercase-extension table exactly as the design's "Required Common Classes" section lists them, and `glyph(profile)` match tables (Nerd: existing sidebar/AppDock private-use icon language; Ascii: one-cell tokens like `/` dir, `@` symlink, `!` broken, `?` special, `#` source, `%` image, `~` config…, each unique per class). Wire `render_entry_row` to render `icon + ' '` before the name from the prepared entry (pure input; profile from existing settings-derived presentation state, default Nerd, ASCII when the existing no-nerd-font capability signal says so — reuse the same signal AppDock icons use).

- [ ] **Step 4: Run GREEN + FM render family.** **Step 5: Commit** `feat: render semantic files icons`.

### Task 18: FIP-3.7 — Icon edge family (TP-FIP-ICON-08, 09, 11, 13)

**Files:**
- Test: `src/ui/file_manager.rs`, `src/fm/entry_kind.rs`

- [ ] **Step 1: Write the family** (exact-cell `TestBackend` tests; remember lesson: assert glyph-start cells, skip wide-glyph continuation cells; convert byte offsets to char counts):

```text
narrow_column_keeps_complete_icon_and_truncates_name_by_display_cells   // ICON-08
icon_never_overlaps_row_action_cells                                    // ICON-08: disjoint rects
cursor_style_wins_over_icon_class_and_multi_select_stays_distinct       // ICON-09
control_characters_in_name_render_escaped_and_do_not_shift_rows        // ICON-13: name "a\nb" renders "a␊b" (or \n → printable escape), following row content unchanged
render_entry_row_performs_no_filesystem_io                              // ICON-11: render with a deleted-under-us path; identical buffer, no panic; plus a debug-assertion seam is NOT added — prove via prepared-state-only inputs (function signature takes no &Path filesystem access)
```

For ICON-13 add the display-label escaping in the prepared entry (`display_label: String` computed in `read_directory_snapshot` escaping every `char::is_control` into a printable form) — RED first on the layout-shift assertion.

- [ ] **Step 2: Run, fix observed failures minimally, commit** `test:`/`fix:` pairs (`fix: escape control characters in entry labels` expected as the one real product delta).

### Task 19: FIP-3.8 — VIS-03 / VIS-04 snapshots

**Files:**
- Modify: exporter states; Create: `tests/visual/icons.spec.ts`

- [ ] Export mixed-kind fixture (directory, link-dir, link-file, broken, source, config, document, image, archive, special — ASCII profile) at 120×40 and tiny 60×16; spec asserts both snapshots; explicit baseline; commit `test: add icon visual snapshots`.

### Task 20: FIP-4.1 RED + FIP-4.2 GREEN — Reference-only payload (TP-FIP-REF-05, REF-07)

**Files:**
- Modify: `src/app/file_agent_handoff.rs` (`sync_file_manager_agent_handoff_send`)
- Test: same file / `src/app/input/file_manager.rs` handoff family

- [ ] **Step 1: Write the failing test** (reuse the existing C5 send-test fixture that captures terminal input bytes):

```rust
#[test]
fn agent_reference_payload_is_exact_path_bytes_with_no_submit() {
    // arrange: existing C5.3 fixture — agent terminal, prepared handoff request for path "/tmp/refdir/a file.txt"
    // act: sync_file_manager_agent_handoff_send
    // assert on the captured bytes:
    let sent = captured_terminal_bytes();
    assert_eq!(sent, "/tmp/refdir/a file.txt".as_bytes());
    assert!(!sent.contains(&b'\r'));
    assert!(!sent.contains(&b'\n'));
}
```

- [ ] **Step 2: Run RED** — fails on trailing `\r` (current code: `payload.push(b'\r')` at `file_agent_handoff.rs:172`). Commit `test: pin reference-only agent payload`.

- [ ] **Step 3: GREEN** — delete the `+1` capacity and the `payload.push(b'\r')` line:

```rust
let payload = Bytes::copy_from_slice(path.as_bytes());
```

Update the existing C5.3 test that asserted `path + \r` (it pinned the OLD contract — rewrite its assertion to the new no-submit contract in the same commit, stating that in the body).

- [ ] **Step 4: Run GREEN + handoff family** (`test(/agent_handoff|file_agent/)`). **Step 5: Commit** `fix: insert agent references without submit`.

### Task 21: FIP-4.3 RED + FIP-4.4 GREEN — Directory acceptance + last-seam kind check (TP-FIP-REF-06, 11, 12, 13, 18)

**Files:**
- Modify: `src/app/file_agent_handoff.rs` (prepare + send seams)

- [ ] **Step 1: Failing tests**

```text
directory_reference_delivers_exact_directory_path            // REF-06: prepare with a directory entry; current prepare requires operation_supported only — verify whether directory passes today; if it already passes, this is characterization
deleted_path_before_send_sends_zero_bytes                    // REF-11: delete the file between prepare and sync; assert captured bytes empty + one failure message
path_kind_change_to_special_before_send_sends_zero_bytes     // REF-12: replace file with FIFO between prepare and sync (unix-gated)
control_character_path_disables_reference_action             // REF-13: entry with "\n" in name → supports_agent_reference false at prepare; and a direct send-seam rejection test
non_utf8_path_rejects_at_prepare                             // REF-13: existing to_str() guard — characterization
spaces_and_punctuation_paths_preserved_byte_for_byte         // REF-18
```

- [ ] **Step 2: Run RED** — REF-11/12 fail because `sync_file_manager_agent_handoff_send` revalidates identity (`file_manager_agent_handoff_is_current`) but not filesystem existence/kind. Commit `test: pin reference path revalidation`.

- [ ] **Step 3: GREEN** — shared validator in `src/app/file_agent_handoff.rs`:

```rust
fn reference_path_is_deliverable(path: &Path) -> bool {
    let Some(text) = path.to_str() else { return false };
    if text.chars().any(char::is_control) {
        return false;
    }
    match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => matches!(
            std::fs::metadata(path),
            Ok(target) if target.is_dir() || target.is_file()
        ),
        Ok(meta) => meta.is_dir() || meta.is_file(),
        Err(_) => false,
    }
}
```

Call it in `prepare_file_manager_agent_handoff` (alongside the existing entry check, which after Task 16 uses `kind.supports_agent_reference()`) and again in `sync_file_manager_agent_handoff_send` before building the payload; failure routes to the existing `show_file_manager_agent_handoff_failure` with zero bytes sent.

- [ ] **Step 4: Run GREEN + family. Step 5: Commit** `fix: revalidate reference paths at delivery`.

### Task 22: FIP-4.5 RED + FIP-4.6 GREEN — No implicit Claude split (TP-FIP-REF-03)

**Files:**
- Modify: `src/app/file_agent_handoff.rs` (`prepare_file_manager_agent_handoff` non-agent branch)

- [ ] **Step 1: Failing test**

```rust
#[test]
fn non_agent_focus_prepares_no_claude_split_for_reference_action() {
    // arrange: C5.4 fixture with a NON-agent focused terminal, one supported path
    // act: the reference intent (row/context SendAgent path)
    // assert:
    assert!(state.request_file_manager_claude_split.is_none());
    assert!(state.request_file_manager_agent_handoff.is_none());
    // and the picker-open request from Task 24 is what appears instead once FIP-5 lands;
    // at this task's point assert only: no split request, no send request, no new pane/terminal.
}
```

- [ ] **Step 2: Run RED** — fails because the non-agent branch sets `request_file_manager_claude_split` (`file_agent_handoff.rs:110-117`). Commit `test: pin no implicit claude split`.

- [ ] **Step 3: GREEN** — remove the `FileManagerClaudeSplitRequest` branch from THIS action's preparation (the reference action now always routes to the FIP-5 picker; until Task 24 lands, the non-agent branch simply returns `false`/no-op). **Verified (FIP-G.2):** `prepare_file_manager_agent_handoff` (`file_agent_handoff.rs:111`) is the SOLE production producer of `FileManagerClaudeSplitRequest`; the only other constructors are the test helper `prepare_claude_split` (`file_agent_handoff.rs:789`) and a test fixture (`src/app/input/file_manager.rs:5943`). Removing the branch therefore makes the whole split machinery (`launch/complete/rollback/sync_file_manager_claude_split`, the request type at `state.rs:759`, and their tests) dead code — Clippy `-D warnings` (dead_code) will reject it, so delete the machinery and its now-dead tests in the same characterized commit, keeping unrelated split workflows (M1 attachment picker, normal pane splits) untouched.

- [ ] **Step 4: Run GREEN + the C5 family** (some C5.4 tests pinned the OLD split behavior — rewrite them to the new contract, stating so in the commit body). **Step 5: Commit** `fix: remove implicit split from reference action`.

### Task 23: FIP-4.7 — Delivery fail-closed family (TP-FIP-REF-08, 09, 10, 14 + exact-once)

**Files:**
- Test: `src/app/file_agent_handoff.rs`

- [ ] **Step 1: Family tests** (existing C5 fail-closed fixtures cover several — extend, do not duplicate):

```text
vanished_workspace_or_pane_sends_zero_bytes                  // REF-08 (existing is_current covers; characterize + extend with workspace/pane fields from Task 24's AgentReferenceRequest)
changed_terminal_identity_sends_zero_bytes                   // REF-09
non_agent_or_unavailable_runtime_sends_zero_bytes_once       // REF-10: assert exactly one failure message, no retry
busy_channel_makes_one_bounded_attempt_and_no_hot_retry      // REF-14: TrySendError::Full fixture; assert single try_send, request consumed
delivery_is_exact_once_per_request                           // second sync call is a no-op (request taken)
```

- [ ] **Step 2: Run/classify/fix minimally; commit** `test: cover reference delivery failure family` (+ `fix:` only for observed leaks).

### Task 24: FIP-5.1 RED + FIP-5.2 GREEN — Picker model from live Agents (TP-FIP-REF-01, 02, 04)

**Files:**
- Create: `src/app/agent_reference_picker.rs` (client-local model + `AgentReferenceRequest`)
- Modify: `src/app/mod.rs` (module), `src/app/state.rs` (`Mode`/overlay entry via `enter_overlay_mode`), `src/app/input/file_manager.rs` (SendAgent intent routes to picker)

- [ ] **Step 1: Failing tests**

```rust
#[test]
fn reference_action_opens_picker_from_live_agents_projection() {
    // arrange: two workspaces with agent terminals via ensure_test_terminals + detected_agent (see
    // all_workspaces_agent_panel_entries_prefer_agent_names_for_agent_identity fixture, src/ui/sidebar.rs)
    // act: dispatch FileManagerContextMenuAction::SendAgent with one supported path
    // assert: picker overlay open (via enter_overlay_mode), rows == live agent entries, no bytes sent
}

#[test]
fn current_focused_agent_is_first_and_preselected() { /* focused agent row index 0, marked current, selected */ }

#[test]
fn picker_selection_snapshots_full_target_identity() {
    // activating a row prepares AgentReferenceRequest { path, source_files_generation, workspace_id, pane_id, terminal_id }
    // and sends nothing yet
}
```

- [ ] **Step 2: Run RED** (module stub compiles; behavior fails: no overlay opens). Commit `test: pin files agent target picker`.

- [ ] **Step 3: GREEN** — model:

```rust
pub(crate) struct AgentReferenceRequest {
    pub path: PathBuf,
    pub source_files_generation: u32,
    pub workspace_id: crate::workspace::WorkspaceId,
    pub pane_id: crate::pane::PaneId,
    pub terminal_id: crate::terminal::TerminalId,
}

pub(crate) struct AgentReferencePickerModel {
    pub source_path: PathBuf,
    pub source_files_generation: u32,
    pub rows: Vec<AgentReferencePickerRow>,
    pub selected: usize,
}

pub(crate) struct AgentReferencePickerRow {
    pub label: String,
    pub is_current: bool,
    pub workspace_id: crate::workspace::WorkspaceId,
    pub pane_id: crate::pane::PaneId,
    pub terminal_id: crate::terminal::TerminalId,
    pub live: bool,
}
```

Builder filters `agent_panel_entries_from(app, runtimes)` to agent-classified live terminals, dedupes by pane, puts the focused agent first with `is_current`. The SendAgent intent (both row `>` and context menu) now calls the picker opener instead of `prepare_file_manager_agent_handoff`; the old prepare/sync pair becomes the picker-activation consumer: activation builds `AgentReferenceRequest` and stores it in the existing `request_file_manager_agent_handoff` slot widened to the new struct (rename field types accordingly; `file_manager_agent_handoff_is_current` gains workspace/pane assertions it currently derives implicitly). Concrete ID types follow the existing definitions used by `FileManagerClaudeSplitRequest` (`workspace_id`, `source_pane_id`, `source_terminal_id` fields at `file_agent_handoff.rs:110-117`).

- [ ] **Step 4: Run GREEN + handoff/picker family. Step 5: Commit** `feat: choose agent reference targets`.

### Task 25: FIP-5.3 RED + FIP-5.4 GREEN — Popup ownership (TP-FIP-REF-15, 16, 17)

**Files:**
- Modify: `src/app/agent_reference_picker.rs`, `src/ui/` picker render (new small fn in `src/ui/file_manager.rs` or sibling), input routes in `src/app/input/`

- [ ] **Step 1: Failing tests**

```text
picker_enters_overlay_mode_and_blocks_background_input      // uses enter_overlay_mode; SF4.2 blocking_overlay_active must classify it
escape_and_outside_click_close_picker_with_zero_bytes       // REF-15
keyboard_up_down_enter_and_mouse_click_share_selection      // one state authority
disabled_row_cannot_be_activated_by_keyboard_or_mouse
stale_source_row_or_context_does_not_open_picker            // REF-16: reuse C3.2 stale right-click fixture
multi_selection_disables_reference_action                   // REF-17: N4.2 action-state fixture — SendAgent disabled reason when >1 explicit selection
```

- [ ] **Step 2: Run RED; commit** `test: pin picker ownership and cancel paths`.
- [ ] **Step 3: GREEN** — register the picker as a blocking overlay through the existing `enter_overlay_mode`/`leave_modal`/`blocking_overlay_active()` seams (SF4.2 sweep proved every overlay entry goes through `enter_overlay_mode` — the picker must too), reuse `modal_stack_areas`/`render_modal_shell` popup geometry (P4.0 evidence: 8-10 existing callers), and the C3.2 keyboard/mouse/outside-click lifecycle language. No new popup framework.
- [ ] **Step 4: Run GREEN + overlay family (`test(/overlay|picker/)`). Step 5: Commit** `feat: own picker input through existing overlay seams`.

### Task 26: FIP-5.5 RED + FIP-5.6 GREEN — Target disappearance (picker side of REF-08/09)

**Files:**
- Modify: `src/app/agent_reference_picker.rs`

- [ ] **Step 1: Failing tests**

```text
target_pane_closed_while_picker_open_disables_row_on_recompute
activation_of_disappeared_target_fails_closed_with_visible_failure
terminal_identity_change_between_open_and_activation_sends_zero_bytes
```

- [ ] **Step 2: RED; commit** `test: pin picker target disappearance`. **Step 3: GREEN** — recompute live rows each frame from current `agent_panel_entries` (bounded `O(live agent panes)`), and activation re-runs full identity validation (the Task 21/23 last-seam validator) before storing the request. **Step 4: family + commit** `fix: fail closed on vanished picker targets`.

### Task 27: FIP-5.7 — Visible copy rename

**Files:**
- Modify: `src/app/state.rs:948` (label), `src/ui/file_manager.rs:2574` (menu item), test pins `src/app/state.rs:3807,3950` and any other `"Send to Agent"` literal (grep all: source, tests, row-action tooltip if any)

- [ ] **Step 1:** RED — update the two label-pinning tests to expect `"Add Reference to Agent..."`, run, read failure. **Step 2:** change every production literal; rerun; grep `-rn "Send to Agent" src/` must return zero. **Step 3: Commit** `fix: rename agent action to add reference` (test+prod in one commit is acceptable here as a pure-copy change pinned by existing tests; keep RED/GREEN separate if any behavior assertion changes).

### Task 28: FIP-5.8 — VIS-05 / VIS-06 snapshots

**Files:**
- Modify: exporter states; Create: `tests/visual/picker.spec.ts`

- [ ] Export picker fixtures: current+other live agents; disabled/disappearing row; tiny layout. Spec + explicit baselines + run PASS; commit `test: add picker visual snapshots`.

### Task 29: FIP-6 — Production closure (FIP-6.1..6.8)

**Files:**
- Modify: `.codex/CURRENT.md`, `.codex/TASKS.md`, `.codex/HANDOFF.md`, `.codex/NEXT-SESSION-PROMPT.md`, `.planning/STATE.md`, `.codex/evidence/fip-progress.md`, lessons files

- [ ] **Step 1 (6.1):** focused nextest over every `TP-FIP` Rust test family (one balanced regex per family; list before run; zero-test filter = invalid).
- [ ] **Step 2 (6.2):** `cd tests/visual && npx playwright test` from freshly exported fixtures — all specs; failure screenshots/traces on any failure; no skip-as-success.
- [ ] **Step 3 (6.3):** isolated PTY byte smoke (E2E-02): throwaway-XDG Herdr, deliver one reference to a test-owned agent-classified pane, capture PTY bytes, assert exact path bytes and zero `\r`/`\n`; plus the Task 9 mouse smoke re-run on the final build. Zero residue; stable socket untouched.
- [ ] **Step 4 (6.4):** full gates — fmt, full nextest, Linux Clippy all-targets, canonical Windows MSVC bin Clippy (`LIBGHOSTTY_VT_SIMD=false`), Bun 5/5 + 12/12, Python 64/64, `git diff --check`, added-production-`unwrap()` scan.
- [ ] **Step 5 (6.5):** render-purity double-draw byte-identical test for Files with icons; picker projection bound test; confirm zero new queue/cache/worker (`grep` new `VecDeque|HashMap` additions audit) and release p95 budgets unchanged via the existing `--release` perf gate.
- [ ] **Step 6 (6.6):** `CBM_WORKERS=1 codebase-memory-mcp cli index_repository '{"repo_path":"/home/ayaz/projects/herdr","mode":"fast","persistence":false}'`; re-query `FileEntryKind`, `bind_focused_child`, `resolve_resident_selection`, `AgentReferenceRequest`, picker symbols, and changed handoff seams by exact snippet.
- [ ] **Step 7 (6.7):** update all continuity files + lessons with exact fresh evidence.
- [ ] **Step 8 (6.8):** `git fetch origin`; `merge-base --is-ancestor` both refs; push `origin HEAD:feat/native-fm` and `origin HEAD:master`; `git ls-remote` equality with local HEAD; tracked tree clean; `.superpowers/` untouched.

---

## Self-Review Notes

- Every design requirement maps to a task (coverage table above; 57/57 IDs).
- Known deliberate deviations from mechanical TDD, each stated in its task: Task 8/12/14/23 contain characterization tests that may pass before change (recorded as such, per SF4.2-04/06 precedent); Task 20/22 rewrite old C5 tests that pinned the now-rejected contract (stated in commit bodies).
- Type consistency: `FileEntryKind` (Tasks 15-18, 21), `AgentReferenceRequest`/`AgentReferencePickerModel` (Tasks 24-26), `resolve_resident_selection` (Tasks 11-12) are each defined once and referenced by the defining task's exact signature.
- FIP-G.2 reconciliation CLOSED (2026-07-18): all four open verification points resolved against current source and the fresh 21,064/98,009 graph — Terminal restore authority `activate_dock_app(Terminal)`/`close_file_manager` (Task 7), resident `render_snapshot_panel(column.cursor)` seam (Task 11), direct `unicode-width 0.2` dependency (Task 17), sole-producer split machinery (Task 22). 57/57 `TP-FIP-*` IDs mapped; gate commands taken from the current `justfile`; no-drag/no-submit/client-local non-goals restated per task. Global `rust-dev` skill content is a broken symlink on this machine (`~/.codex/skills/rust-dev` → missing `~/.claude/skills/rust-dev`); the herdr-local catalog (HP1-HP10, FM/TUI patterns) is loaded and takes precedence, but the global layer must be restored before Rust implementation starts.
