# NEXT SESSION TRIGGER — Herdr Files Interaction Polish

Updated: 2026-07-22 CEST

## Current Override — FMH Fully Gated Locally; Isolated E2E Next

Continue `feat/native-fm` from the exact published
`origin/feat/native-fm` tip `787bb96b`. Verify local HEAD and the remote ref
before work. Checkpoint
`refs/checkpoints/herdr-fm-horizontal-pre-fix-20260722` preserves that
pre-FMH head. The user has physically accepted both the main stutter fix and
the FMN vertical/wheel build as working perfectly. Do not restart FMP/FMN or
reimplement their published commits.

The current uncommitted FMH source/docs lane has one production semantic
change: Right/`l` over a non-directory exact cursor returns `Inert` instead of
falling through to file activation. The required interaction law is:

1. Left moves exactly one resident column left whenever one exists; root is
   inert.
2. Right/`l` moves/activates exactly one child only over a directory.
3. Right/`l` over a file/non-entry/stale identity is model-, worker-, focus-,
   and render-inert.
4. Enter/click retain explicit file/directory activation.

TDD evidence: behavioral RED `0ddfe67c-72fc-4f0f-baa5-715f83a1f1c6`, FMH
3/3, cross-layer 10/10, broad FM 190/190, full 3,622/3,622 + 4 skip, both
Clippy targets, Python 68/68, Bun 5/5 + 12/12, exporter 1/1, Chromium 33/33,
zero generated JSON/PNG delta, and clean source/dependency/vendor/diff audits.
Graph CLI store is current at 24,078 / 129,027 and resolves every FMH symbol;
the built-in long-lived channel is stale at 24,072 / 129,520 and `ready` must
not be mistaken for freshness. Continue with the cleanup-first isolated E2E.
Do not commit until the message is proposed and aligned; exact-path staging
only.

FMN-1 through FMN-5 are closed and published:

1. raw Ghostty trace: 333 vertical packets, 226 same-direction deltas below
   2 ms in identical-coordinate triplets/occasional sextuplets;
2. one-to-one routing rejected duplicate Herdr dispatch; host micro-burst and
   old automatic branch amplification were confirmed;
3. Up/Down/`j/k`/Shift/wheel move one exact owner-column cursor row;
   Right/`l` owns directory traversal while Enter/click remain explicit
   activation commands;
4. directory cursor preview uses the bounded latest worker and rejects stale
   generation/source/owner/index/path/current-cursor results;
5. wheel normalization coalesces only identical owner/direction/coordinates
   strictly below 2 ms and preserves reversal, changes, 2 ms, and 5 ms input;
6. initial preview restores the parent owner; every render/hit/resize/watcher
   projection follows `active_col()`, while `deepest()` is resident extent;
7. focused 302/302, active-owner trio 3/3, full 3,619/3,619 + 4 skip, fmt,
   both Clippy targets, Python 68/68, Bun 5/5 + 12/12, exporter 1/1, and full
   Chromium 33/33 are green. Six reviewed VIS-01..06 PNGs changed; generated
   JSON and VIS-07..25 stayed clean.

The exact next task is **FMH-4**: hand the user one cleanup-first isolated
command, collect Left/Right physical acceptance, then align/commit/push only
to CyPack. Leave
Home/Desktop/Downloads pre-warm for FMN-6 measurement-first work; no
general/unbounded LRU.

Read these current authorities before code:

- `.codex/evidence/files-performance-fix-closure-and-navigation-followups.md`;
- `.codex/references/yazi-file-manager-performance-transfer.md`;
- `.codex/TASKS.md` FMN section;
- `.local/ISOLATED-DEV-TEST.md`.

The pre-edit graph resolved the current horizontal reducer and key route;
FMH-3 post-edit freshness is closed in the current CLI store at 24,078 nodes /
129,027 edges with all new tests and the production `return Inert`. After commit,
recheck SHA-bound freshness with `handle_file_manager_key`,
`move_active_left`, `move_active_right`, and the new FMH regression symbols.
Lower FIP task counts and
“exact first work” paragraphs in this historical file are superseded by this
override; recount the canonical task files rather than using old totals.
Never touch stable Herdr/socket or `.superpowers/`; never regenerate PNGs
blindly; use exact-path staging and align commit messages before commit.

## Historical Override — Miller Trail Closed

Miller Trail T1-T7 and FIP-D1/D3/D4 product code are closed through
`e8abc7b0` RED / `3c36f104` GREEN. Do not restart T7 or restore the retired
parent/current/resident projection. Fresh closure gates are Rust 3,507/3,507
+ 2 skip, Chromium 18/18, both Clippy targets, maintenance 68/68, Bun 5/5 +
12/12, and source audit 4/4 with exact `"(unavailable)"` grep=0.

The default next independent lane is FIP-6.3 E2E mouse-harness investigation.
It must begin from `.codex/HANDOFF.md` and `.codex/TASKS.md`, preserve the
stable socket, and follow the isolated runtime recipe. Custom-layout B-chain
is separate and starts only from its own approved design/plan. Recount the
7 + 89 unchecked registry tasks; do not infer priority from older text below.

Continue `/home/ayaz/projects/herdr` on branch `feat/native-fm` from the
canonical Files Interaction Polish handoff. This is `mid_flight_adoption`.
Do not restart the project, reimplement completed SF/FM work, infer state from
chat memory, or begin with a lower-priority task.

## Exact Objective

Deliver the user-approved Files Interaction Polish program:

1. visible default-sidebar Files primary click opens Native Files Stage;
2. Spaces/Projects restore Terminal Stage client-locally without changing
   pane/terminal/runtime identity;
3. Miller resident columns highlight the exact child path entered, never a
   fabricated row-zero fallback;
4. directory/file/symlink/broken/special kinds receive deterministic semantic
   icons with Nerd and ASCII one-cell profiles;
5. `Add Reference to Agent...` targets an explicit live agent and inserts an
   exact safe UTF-8 file or directory path only;
6. no CR, LF, Enter, submit, implicit whitespace, implicit Claude split/chat,
   retry queue, or hot retry;
7. Rust semantic tests, deterministic Ratatui cells, Playwright Chromium,
   isolated mouse, PTY byte capture, full platform gates, performance and
   cleanup evidence close the program.

Drag-and-drop, Apps/Desktop, server protocol expansion, speculative
ComponentRegistry, popup framework, and unrelated change-pipeline work are
out of scope.

## Mandatory Start Order — No Skips

1. Read `/home/ayaz/projects/herdr/AGENTS.md` completely.
2. Read `/home/ayaz/projects/herdr/CLAUDE.md` completely.
3. Use `$herdr-native-fm`.
4. Before executing that skill, completely read:
   - `.codex/skills/herdr-native-fm/lessons/errors.md`
   - `.codex/skills/herdr-native-fm/lessons/golden-paths.md`
   - `.codex/skills/herdr-native-fm/lessons/edge-cases.md`
   - `/home/ayaz/.codex/skills/_shared/common-errors.md`
5. Read these canonical files completely and in order:
   - `.codex/BOOTSTRAP.md`
   - `.codex/CURRENT.md`
   - `.codex/TASKS.md`
   - `.codex/CHANGE-PIPELINE-TASKS.md`
   - `.codex/HANDOFF.md`
   - `.codex/MEMORY.md`
   - `.planning/STATE.md`
   - `.codex/NEXT-SESSION-PROMPT.md`
6. Read the current FMN authorities completely:
   - `.codex/evidence/files-performance-fix-closure-and-navigation-followups.md`
   - `.codex/references/yazi-file-manager-performance-transfer.md`
   - `docs/superpowers/specs/2026-07-19-herdr-files-rapid-navigation-latency-prd.md`
7. Audit Git and remotes before edits:

   ```bash
   git status --short --branch
   git log --oneline --decorate -12
   git remote -v
   git rev-parse HEAD
   git ls-remote origin refs/heads/feat/native-fm refs/heads/master
   ```

8. Preserve every unrelated/user-owned change. `.superpowers/` is untracked,
   user-owned, and must never be edited, staged, deleted, or cleaned.
9. Use Codebase Memory before grep/glob for every code-discovery question.
10. Recount and copy every unchecked canonical task into the in-session task
    list with continuation lines intact.

## Mandatory Task-List Trigger

Expected canonical inventory after FMH-3 closure:

- `.codex/TASKS.md`: 12 unchecked product/deferred tasks;
- `.codex/CHANGE-PIPELINE-TASKS.md`: 89 unchecked paused tooling tasks;
- total: 101;
- `.codex/HANDOFF.md` section 8: exact 101-block copy.

Recount all three and compare exact task blocks. If count or text differs,
stop before code and reconcile CURRENT/TASKS/HANDOFF.

Status assignment:

- keep **FMH-4** `in_progress` until physical E2E and publication close;
- keep FMN-6 pending (FMN-0 through FMN-5 are closed);
- keep S5/S7 trigger-gated;
- keep change-pipeline T3.1-T10.9 paused;
- never choose an easier lower-priority task.

## Exact First Work

1. Revalidate the uncommitted FMH diff, checkpoint, stable-runtime exclusion,
   completed automated gates, and exact task-copy parity.
2. Run the cleanup-first isolated helper and collect exact one-edge Left,
   directory-only Right, inert file-Right, vertical/wheel regression, semantic
   exit, and zero-residue acceptance without touching stable Herdr.
3. After physical acceptance, align the proposed product and documentation
   commit messages before any staging; use exact paths only and never stage
   `.superpowers/`.
4. Run final delta gates, exact-path stage, commit, refresh the graph, push only
   `origin/feat/native-fm`, and verify exact local/remote SHA equality.
5. Keep FMN-6 cache/pre-warm work separate and measurement-first.

## Codebase Memory Protocol

Canonical project:

```text
home-ayaz-projects-herdr
```

Required discovery order:

1. `get_architecture`
2. `search_graph`
3. `trace_path` with `calls` or `data_flow`
4. `get_code_snippet` using an exact qualified name returned by graph search
5. `query_graph` for complex ownership/edge questions
6. graph-augmented `search_code`
7. grep only for strings/config/non-code or when graph evidence is insufficient

Current FMH graph evidence takes precedence over the historical bullets below:

- single-worker CLI store: 24,078 nodes / 129,027 edges;
- 12 changed files / 0 extraction errors;
- all three FMH tests and the live Right branch's non-directory `return Inert`
  resolve from current source;
- the long-lived built-in channel is stale at 24,072 / 129,520 and `ready`
  alone is not freshness evidence.

Historical FMN handoff graph evidence:

- 23,925 nodes / 124,127 edges;
- built-in MCP status `ready`, cross-checked with current final-fix and FMN
  symbols rather than accepted alone;
- `route_client_events` and `handle_client_input_events` expose the inert-move
  render decision chain;
- `install_trail_operation_projection` exposes resident file projection;
- `queue_file_manager_trail_directory_activation` exposes the bounded explicit
  click path;
- `move_trail_cursor_in_column` exposes the cursor-only reducer;
- `FileManagerVerticalWheelBurstGate` exposes the measured host-packet gate;
- `queue_file_manager_trail_directory_preview_identity` exposes bounded
  cursor-follow directory preview.

FMN-1 root-cause verdict:

- H1 confirmed: one physical high-resolution/momentum gesture can decode into
  identical-coordinate triplets/sextuplets below 2 ms;
- H2 rejected: the Herdr route is one-to-one and counters show no duplicate
  dispatcher application;
- H3 confirmed and closed: the old activation-coupled reducer branched on a
  directory; the cursor-only reducer retains owner-column authority.

After committed changes, refresh the graph with the safe single-worker route if
the long-lived channel is stale:

```bash
CBM_WORKERS=1 codebase-memory-mcp cli index_repository \
  '{"repo_path":"/home/ayaz/projects/herdr","project":"home-ayaz-projects-herdr","incremental":true}'
```

Do not restart/kill the MCP proxy or any user process. Re-query changed recent
symbols after indexing.

## TDD and Test-Point Protocol

Before each production edit:

1. identify exact test point(s);
2. state current behavior;
3. state expected behavior;
4. state expected RED failure and why;
5. add a compile-valid behavior test;
6. run it and read the actual failure;
7. commit RED separately;
8. implement minimum GREEN;
9. run focused + regression tests;
10. commit GREEN separately;
11. refactor only behind green tests;
12. run proportionate and final gates.

Never accept:

- a test that passes before the product change;
- compile failure as behavioral RED;
- snapshot-only semantic authority;
- manual “looks right” without automated evidence;
- skipped Playwright as success;
- “should work” completion language;
- hidden failing tests or unverified platform claims.

Required non-happy-path families include:

- mouse modifiers/buttons/releases/outside coordinates;
- overlay/capture precedence and hidden background;
- collapsed/tiny/resized layouts;
- stale generations and stale typed hits;
- deep Miller chain, reorder/delete/hide/branch retirement;
- duplicate/malformed/missing exact path;
- empty/root/permission/unavailable directories;
- symlink target types, broken symlink and special entries;
- mixed-case extensions, dotfiles, Unicode, long and control names;
- exact display-cell truncation and no icon/action overlap;
- target disappearance, terminal identity replacement and non-agent state;
- path deletion/kind change between picker and delivery;
- control/non-UTF-8 rejection;
- busy/full channel, exact-once, cancellation and zero retry;
- payload audit proving zero CR/LF/Enter/submit;
- zero process/socket/temp residue.

## Playwright Chromium Visual Contract

Playwright is mandatory for visual acceptance but is not the semantic source of
truth.

- Export exact Ratatui TestBackend cells through test-only code.
- Browser renders a deterministic monospace cell grid.
- Canonical cross-machine snapshots use ASCII icon profile.
- Nerd profile mappings are tested in Rust exact-cell tests.
- Pin viewport, DPR, font, locale, timezone, color scheme and motion.
- Use one CI worker.
- Ordinary CI cannot update snapshots.
- Missing/malformed fixture and missing browser fail explicitly.
- A controlled one-cell mutation must fail its snapshot.
- Capture screenshot on failure and trace on first retry.

Before any manual or physical test, read `.local/ISOLATED-DEV-TEST.md`.
Clear inherited Herdr socket variables and use unique throwaway XDG/socket
state. Never touch installed stable Herdr or its socket.

## Architecture Guardrails

- `AppState` stays pure data; runtime stays separate.
- `compute_view()` owns geometry/mutation; `render()` is pure.
- no filesystem/config/process/socket reads in icon render;
- shared runtime facts remain server/API; FIP presentation state remains TUI;
- no private TUI socket protocol expansion;
- no production `unwrap()`;
- no unbounded history/cache/queue/worker;
- one bounded terminal send attempt;
- generation/path/terminal identity is authority, never coordinates alone;
- stale and ambiguous state consumes inert/fails closed;
- no new Rust dependency unless existing dependencies cannot satisfy a proven
  need and the plan explicitly justifies it.

## Git and Publication Protocol

- Acting account: CyPack, external contributor/fork.
- Never push upstream and never open upstream issue/PR.
- Preserve existing valid history; no reset/checkout destruction.
- Use exact-path staging only, never `git add -A`.
- Review staged file list and staged diff before every commit.
- Commit style: lowercase conventional, no emoji, no AI co-author.
- One concern per commit; RED/GREEN/refactor separate.
- Product, continuity and non-product pipeline concerns do not mix.
- Fetch before publication.
- Prove fast-forward ancestry for both CyPack refs.
- Push only:
  - `origin HEAD:feat/native-fm`
  - `origin HEAD:master`
- Verify both remote SHAs equal local HEAD.
- Verify tracked worktree clean; `.superpowers/` may remain untracked and
  untouched.

## Stop Conditions

Stop before mutation and report exact evidence if:

- task inventory does not reconcile;
- Git ancestry or remote ownership differs;
- unrelated tracked product changes exist;
- graph is stale and cannot be safely refreshed;
- a required skill/lesson is unavailable;
- RED does not fail for the expected behavior;
- three fixes hit the same blocker without new evidence;
- Chromium/Playwright visual gate cannot run;
- stable socket isolation cannot be proven;
- any full/platform gate remains failing;
- requested publication is non-fast-forward or targets upstream.

## Completion Definition

Do not claim FIP complete until every condition in `.codex/HANDOFF.md`
section 11 is freshly proven and recorded. In particular: all 55 test points,
exact resident focus, semantic icons, explicit live-agent picker, exact
path-only no-submit payload, all fail-closed cases, Playwright/PTY/runtime/full
gates, budgets, zero residue, fresh graph, atomic history, continuity updates,
and exact CyPack remote SHA equality.

## Start Now

Read the mandatory sources, reconstruct the exact 143-item task list, verify
Git and graph truth, set only FIP-G.1 in progress, and produce the detailed
code-level TDD plan. Do not ask which task to do and do not start Rust before
FIP-G.2 closes.
