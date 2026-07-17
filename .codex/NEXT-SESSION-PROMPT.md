# NEXT SESSION TRIGGER — Herdr Files Interaction Polish

Updated: 2026-07-17 CEST

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
6. Read the approved design completely:
   - `docs/superpowers/specs/2026-07-17-herdr-files-interaction-polish-design.md`
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

Expected canonical inventory (after the 2026-07-18 planning-gate closure):

- `.codex/TASKS.md`: 52 unchecked product/deferred tasks;
- `.codex/CHANGE-PIPELINE-TASKS.md`: 89 unchecked paused tooling tasks;
- total: 141;
- `.codex/HANDOFF.md` section 8: exact 141-block copy.

Recount all three and compare exact task blocks. If count or text differs,
stop before code and reconcile CURRENT/TASKS/HANDOFF.

Status assignment:

- set only **FIP-0.1** to `in_progress`;
- keep FIP-0.2 through FIP-6 pending;
- keep S5/S7 trigger-gated;
- keep change-pipeline T3.1-T10.9 paused;
- never choose an easier lower-priority task.

## Exact First Work

FIP-G.1/FIP-G.2 are CLOSED. The approved code-level TDD plan is
`docs/superpowers/plans/2026-07-18-herdr-files-interaction-polish-implementation.md`
(commit `dd81ef59`; 29 bite-sized tasks; all 57 unique `TP-FIP-*` IDs mapped —
the earlier "55" figure excluded the two E2E IDs, nothing was dropped).

1. Read that plan completely; it is the execution contract.
2. Start at plan Task 1 (FIP-0.1 baseline freeze) and execute tasks in order
   with separate RED/GREEN/refactor commits.
3. Before any Rust production edit, restore the broken global `rust-dev`
   skill (`~/.codex/skills/rust-dev` is a symlink to the missing
   `~/.claude/skills/rust-dev`) or explicitly record that only the
   herdr-local HP1-HP10 catalog is available.
4. Follow the plan's verified-owner map; re-verify any symbol the plan marks
   as graph-verified if the graph has been refreshed since `dd81ef59`.

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

Final handoff graph evidence:

- 21,064 nodes / 98,009 edges;
- major packages: app 2,615; ui 968; pane 715; fm 457; server 407;
- single-worker CLI and built-in MCP agree;
- FIP-G.1 is found in six canonical continuity modules;
- `focused_child` and current handoff send/fail-closed seams are present;
- freshness was verified from current symbols, not `ready` alone.

Root-cause hypotheses already supported by graph/source evidence and requiring
fresh-plan confirmation:

- default ShellLayout has no AppDock; sidebar tab route changes visual tab but
  does not invoke existing Files Stage activation;
- `MillerPathSegment.focused_child` is not populated; resident projection falls
  back to a cursor initialized at zero;
- directory snapshot capability reduction discards symlink identity and row
  render has no semantic icon classification;
- current handoff delivery appends carriage return and can prepare an implicit
  Claude split for a non-agent target;
- existing `agent_panel_entries`, target identity lookup, and bounded terminal
  input seam can be reused instead of adding runtime ownership.

After committed changes, refresh the graph with the safe single-worker route if
the long-lived channel is stale:

```bash
CBM_WORKERS=1 codebase-memory-mcp cli index_repository \
  '{"repo_path":"/home/ayaz/projects/herdr","mode":"fast","persistence":false}'
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
