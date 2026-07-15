# Herdr Change Pipeline Program Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> `superpowers:executing-plans` to implement this program plan task-by-task.
> Steps use checkbox (`- [ ]`) syntax for tracking. Do not dispatch subagents
> unless the user explicitly authorizes delegation.

**Goal:** Deliver two independently versioned, machine-validated repo-local
skills that turn reference or native change requests into an immutable Herdr
change-intent package and then govern production delivery through explicit
authorization, TDD, failure, performance, Git, and graph gates.

**Architecture:** Upgrade `ratatui-design-intelligence` to v2.1 without
changing its stable P0-P14 identity, then build the sibling
`herdr-change-pipeline` v1 module with separate A0-A7 analysis and I0-I14
delivery graphs. The sole cross-module boundary is a validated P14 reference
adapter that emits the same immutable `change-intent-package.json` used by
native modes; it never grants product authority.

**Tech Stack:** Markdown, JSON, JSON Schema Draft 2020-12, Python 3 standard
library validators and `unittest`, Codebase Memory MCP, Git, and the existing
repo-local Codex skill/lesson conventions. No new Python or Rust dependency.

## Global Constraints

- Module versions are `ratatui-design-intelligence` `2.1.0` and
  `herdr-change-pipeline` `1.0.0`.
- Stable pipeline IDs remain `reference-project-intelligence-v2`,
  `herdr-change-intelligence-v1`, and `herdr-change-delivery-v1`.
- Artifact `schema_version`, module version, and pipeline version remain
  independent fields.
- Product-code authorization defaults to `false`; A7/P14 readiness never
  implies mutation authority.
- P0-P14 IDs, order, statuses, prior required artifacts, and gates remain
  backward compatible.
- Codebase Memory graph tools precede code file search; missing mandatory MCP
  evidence produces `blocked`, never fabricated data.
- Existing user-owned `.superpowers/` and untracked Ratatui skill work are
  preserved. Stable Herdr processes and inherited sockets are never touched.
- No production behavior without a compile-valid, behavior-specific observed
  RED. Setup, compile, fixture, timing, or unrelated failures are not RED.
- Every queue, log, history, artifact list, and evidence stream has an explicit
  bound or an append-only durable-file contract.
- No unexplained bug, failing test, retry-only green, stale graph, missing
  artifact, unauthorized diff, force push, or upstream write may remain at
  closure.

---

## Approved Plan Set and Execution Order

1. This program plan freezes shared interfaces, global test points, Git
   boundaries, and execution order.
2. Execute
   `docs/superpowers/plans/2026-07-15-ratatui-design-intelligence-v2-1-implementation.md`.
3. Require the complete v2.1 compatibility gate before starting the reference
   adapter in the sibling module.
4. Execute
   `docs/superpowers/plans/2026-07-15-herdr-change-pipeline-v1-implementation.md`.
5. Run the combined compatibility, isolation, pilot, Git, and graph closure
   gate.

The two implementation plans are separate because either module can be
reviewed, rejected, reverted, and verified independently. They share no Python
imports or mutable runtime state.

## Frozen File Ownership

### Ratatui v2.1 plan owns

- Existing subtree only:
  `.codex/skills/ratatui-design-intelligence/`.
- New identity/governance files: `README.md`, `AGENTS.md`, `module.json`,
  `references/module-governance.md`.
- New stack contract files:
  `assets/schemas/stack-adaptation-map.schema.json` and
  `assets/templates/stack-adaptation-map-template.json`.
- Existing pipeline, run template, validator, tests, evals, skill router,
  lessons, and `.cartography/SYSTEM-MAP.json` only where v2.1 requires them.
- No file under `src/`, `tests/`, `website/`, `docs/next/`, `vendor/`, or
  `.codex/skills/herdr-change-pipeline/`.

### Change-pipeline v1 plan owns

- New subtree only: `.codex/skills/herdr-change-pipeline/`.
- Its README, AGENTS, SKILL, module manifest, two pipeline manifests, two run
  templates, schemas, artifact catalog/templates, validator, tests, evals,
  references, lessons, fixtures, and cartography.
- Continuity updates only after fresh verification:
  `.codex/CHANGE-PIPELINE-TASKS.md`, `.codex/CURRENT.md`, `.codex/TASKS.md`,
  and `.codex/HANDOFF.md`.
- No Herdr Rust, dependency, protocol, persisted-state, release-doc, stable
  runtime, or socket surface.

## Frozen Cross-Module Interface

`ratatui-design-intelligence` P14 produces its existing verified artifact set
plus `stack-adaptation-map.json`. The sibling module consumes these only
through the frozen public interface
`adapt_reference_p14(p14_dir: Path, analysis_inputs: dict[str, object]) ->
tuple[dict[str, object] | None, list[str]]`. The first tuple member is present
only when the error list is empty.

The adapter must verify P14 status, source/target hashes, integration V=0,
product isolation, license/reuse classifications, stack-map completeness, and
traceability. Native modes never call this function and never fabricate source
or license records.

The delivery boundary is the frozen public interface
`authorize_delivery(change_intent: dict[str, object], authorization:
dict[str, object]) -> list[str]`. It returns an empty list only for an A7-ready,
hash-matched, explicitly authorized scope.

No validator imports Herdr product code. No plan task changes Rust.

## Mandatory Pre-Code Test-Point Catalog

These points are defined before executable changes. Each implementation plan
maps them to exact tests and commands.

| Test point | What is tested | Current/initial result | Expected result | Why required |
|---|---|---|---|---|
| TP-PROG-BOUNDARY | Exact files changed by each module and continuity closure | Ratatui subtree is untracked; sibling subtree absent | Each test/commit touches only its frozen ownership set; unauthorized paths fail isolation | Parallel file-manager work must not be staged or overwritten |
| TP-PROG-VERSION | Module, pipeline, and artifact schema versions | No module manifest; no sibling module | All three version classes are explicit and cross-file consistent without conflation | Durable consumers need stable compatibility semantics |
| TP-PROG-AUTH | P14/A7 readiness versus product authorization | Current research run template is false; sibling absent | Ready analysis remains false; I0 rejects false/missing/overbroad authorization | Analysis must never silently mutate product code |
| TP-PROG-MCP | Mandatory graph claim without MCP evidence | Existing contract says blocked | Validators/evals produce `blocked`; no graph fields are invented | Anti-hallucination is an architecture invariant |
| TP-PROG-RED | RED evidence classification | Existing validator tests are green baseline only | Target behavior fails for the intended assertion; compile/setup/flaky failures are rejected | Passing tests do not prove test-first development |
| TP-PROG-TRACE | Evidence -> node/dimension -> decision -> requirement -> test -> slice -> commit -> verification | Cross-module chain absent | Link set is complete, acyclic, and hash-bound | Multi-session execution must remain auditable |
| TP-PROG-COMPAT | Existing P0-P14 order/status/artifact/gate behavior | 18/18 baseline and 15 phases/97 jobs were previously reported | Fresh legacy suite stays green after v2.1 and sibling module work | Additive governance must not break corpus intake |
| TP-PROG-NATIVE | Native change analysis without source repository/license | Sibling module absent | Native fixture reaches A7 without fabricated reference fields | Reference intake is optional, not privileged |
| TP-PROG-REFERENCE | P14 adapter into common handoff | No common adapter | Reference fixture maps losslessly and retains source/license/stack trace | Source evidence must survive normalization |
| TP-PROG-MIDFLIGHT | Existing feature/bugfix adoption | Prose-only interim contract | Existing commits/diffs/tests are preserved, gaps are explicit, current I-phase is justified | Active file-manager work must not restart or invent historical RED |
| TP-PROG-NOGO | `no_go`, `blocked`, and `rejected` analysis | Sibling module absent | No delivery task or authorization is created | The pipeline must be able to stop bad work |
| TP-PROG-CROSS | Five layers and nine cross-test families | Design-only | Applicability and evidence/waiver are complete; silent omission fails | Happy-path unit tests cannot close architecture changes |
| TP-PROG-PERF | Workload/environment/baseline/target/regression evidence | Design hypotheses only | Uncalibrated or missing budgets cannot pass I12 | Performance claims require reproducible context |
| TP-PROG-PUBLISH | Targeted staging, ancestry, remote SHA, graph freshness | No implementation publication | Allowed refs only, FF proof, SHA equality, fresh artifact/symbol evidence | Local green does not prove published/fresh state |

## Verification Cadence

### Every RED/GREEN slice

Run the exact focused test command written in the active child-plan step, then
run `git diff --check` and `git diff --name-only`. Expected: the RED command
fails only at the named assertion; the GREEN command passes with zero failure;
diff paths equal the slice ownership list.

### Every module commit

```bash
python -m unittest discover -s .codex/skills/ratatui-design-intelligence/tests -p 'test_*.py' -v
python .codex/skills/ratatui-design-intelligence/scripts/validate_reference_pipeline.py .codex/skills/ratatui-design-intelligence/assets/reference-project-pipeline-v2.json .codex/skills/ratatui-design-intelligence/assets/reference-project-run-template.json
python .codex/skills/ratatui-design-intelligence/scripts/validate_v21_module.py
python -m unittest discover -s .codex/skills/herdr-change-pipeline/tests -p 'test_*.py' -v
python .codex/skills/herdr-change-pipeline/scripts/validate_module.py
git diff --cached --check
git diff --cached --name-only
```

Expected: zero failed/error tests, validator exit 0 with deterministic `PASS`,
all JSON changed by the active child task parses through its exact
`python -m json.tool FILE >/dev/null` command, no whitespace errors, and only
declared files are staged.

### Program closure

```bash
python -m unittest discover -s .codex/skills/ratatui-design-intelligence/tests -p 'test_*.py' -v
python -m unittest discover -s .codex/skills/herdr-change-pipeline/tests -p 'test_*.py' -v
git diff --check
git status --short --branch
```

Then run both canonical validators, eval coverage, cartography validation,
legacy compatibility, negative fixtures, isolation audit, and Codebase Memory
refresh/freshness queries. Product `just check` is required only if an audited
diff unexpectedly touches a product/build surface; such a diff is otherwise a
program failure and must be removed.

## Atomic Git Sequence

1. `docs(change-pipeline): approve implementation scope and plans`
2. `chore(ratatui-intel): preserve canonical module baseline`
3. `test(ratatui-intel): define v2.1 module contracts`
4. `feat(ratatui-intel): add cross-stack adaptation contract`
5. `docs(ratatui-intel): add module governance`
6. `test(change-pipeline): define module and analysis contracts`
7. `feat(change-pipeline): add change intelligence pipeline`
8. `test(change-pipeline): define delivery contracts`
9. `feat(change-pipeline): add authorized delivery governance`
10. `test(change-pipeline): add adapters and negative pilots`
11. `docs(change-pipeline): close verification and continuity`

RED and GREEN may remain separate local commits only when the branch tip is
not published failing. Targeted staging precedes every commit. A failed slice
is reverted only within its owned files; unrelated/user work is never reset.

## Program Stop Rules

- Stop T2 if the untracked Ratatui baseline changes after its freeze hash.
- Stop the reference adapter if v2.1 compatibility or P14 validation is not
  green.
- Stop any MCP-dependent phase on unavailable or stale required evidence.
- Stop a slice after three failed fixes and record exact hypotheses/results.
- Stop publication on divergence; never force.
- Stop closure on any failed, flaky, retry-only, zero-test, unexplained,
  unbounded, unauthorized, or untraceable result.

## Completion

This program closes only when both child plans are fully complete, all global
test points pass with fresh evidence, the two module versions and three
pipeline identities are queryable, legacy P0-P14 remains green, native/
reference/mid-flight/no-go/blocked/unauthorized pilots are proven, all intended
commits are published only to authorized CyPack refs with exact SHA equality,
and Codebase Memory returns current module artifacts rather than a `ready` flag
alone.
