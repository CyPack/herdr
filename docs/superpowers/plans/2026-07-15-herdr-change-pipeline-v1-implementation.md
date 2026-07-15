# Herdr Change Pipeline v1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> `superpowers:executing-plans` to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking. Do not dispatch subagents unless the
> user explicitly authorizes delegation.

**Goal:** Build the repo-local `herdr-change-pipeline` v1 skill that validates
native/reference/composite A0-A7 analysis and governs explicitly authorized
I0-I14 delivery with complete traceability, TDD, failure, performance,
publication, and mid-flight adoption contracts.

**Architecture:** Use two immutable phase manifests and catalog-driven artifact
validation. Keep pure shared graph/schema helpers in `contracts.py`, analysis
rules in `validate_analysis.py`, delivery rules in `validate_delivery.py`, and
P14 normalization in `reference_adapter.py`; a small CLI composes them without
importing or mutating Herdr product code.

**Tech Stack:** Python 3 standard library, `unittest`, JSON, JSON Schema Draft
2020-12, Markdown, Codebase Memory MCP, and Git. No new dependencies.

## Global Constraints

- Module version is `1.0.0`; pipeline IDs are
  `herdr-change-intelligence-v1` and `herdr-change-delivery-v1`.
- Analysis phases are exactly A0-A7; delivery phases are exactly I0-I14.
- Product authorization defaults false; only I0 can accept explicit true
  authorization after validating A7 readiness and the frozen worktree scope.
- Native modes never require or fabricate reference source/license artifacts.
- Reference mode requires P14, stack adaptation, source hashes, license/reuse,
  integration V=0, and product-isolation evidence.
- Every fractal node, dimension, decision, requirement, test, slice, commit,
  and verification link uses stable unique IDs and acyclic traceability.
- The nine architecture-constitution rules, eight risk domains, 15 checklist
  questions, five test layers, and nine cross-test families are canonical.
- Valid RED is compile-valid and fails only at the intended behavior assertion.
- Existing Herdr/product files, stable processes, inherited sockets, unrelated
  changes, upstream refs, and force pushes remain out of scope.

---

## Frozen Canonical Catalogs

The manifests, templates, validators, docs, and evals use these exact IDs and
labels. A shorter alias or reordered catalog is a contract failure.

| Analysis ID | Name |
|---|---|
| A0 | Intake, Mode and Evidence Boundary |
| A1 | Goals, Actors, Scenarios and Success |
| A2 | Fractal Decomposition |
| A3 | Parallel Dimensional Investigation |
| A4 | Concept, Pattern and Option Brainstorm |
| A5 | Fresh Herdr Cartography and Fit |
| A6 | Cross-Dimension Synthesis and Target Contract |
| A7 | Normalized Handoff and Readiness Audit |

| Delivery ID | Name |
|---|---|
| I0 | Authorization and Immutable Handoff |
| I1 | PRD, Scenarios, Authority and Non-Goals |
| I2 | Fresh Target Cartography and Drift |
| I3 | Current, Expected and Semantic Diff |
| I4 | Test Architecture Before Production Code |
| I5 | Dependency-Ordered Task and Slice Graph |
| I6 | Characterization and Baseline |
| I7 | Observed RED Per Slice |
| I8 | Minimal Production-Grade GREEN |
| I9 | Refactor and Local Invariant Gates |
| I10 | Cross-Layer Behavioral Tests |
| I11 | Failure, Recovery, Security and Capability |
| I12 | Performance Budgets and Optimization |
| I13 | Full Repository, Migration and Cadence Gates |
| I14 | Evidence Audit, Git Publication and Graph Refresh |

The twelve dimension IDs are:

1. `product_value` — goal, actor, scenario, and user value;
2. `behavior_lifecycle` — behavior, state transitions, interaction, lifecycle;
3. `page_input_ownership` — hierarchy, navigation, overlays, focus, input;
4. `layout_capability` — responsive geometry, terminal cells, fallback;
5. `component_tokens` — contracts, variants, tokens, design consistency;
6. `data_authority` — provenance, transforms, caching, authority;
7. `runtime_protocol_pty` — runtime/client, API, events, PTY, concurrency;
8. `failure_security_resources` — failure, recovery, trust, resource bounds;
9. `persistence_compatibility` — storage, migration, rollback, compatibility;
10. `platform_accessibility` — OS, accessibility, Unicode, color, degradation;
11. `performance_soak` — allocation, latency, throughput, soak risk;
12. `integration_license_cost` — tier, dependency, license, reuse, maintenance.

Architecture-constitution IDs and risk-domain IDs are also exact:

| ID | Architecture constitution rule |
|---|---|
| AC1 | Runtime and Client Boundary |
| AC2 | Small Core and Change Placement |
| AC3 | Single PTY and Terminal Authority |
| AC4 | Process-Isolated Plugins |
| AC5 | Ordered and Bounded Events |
| AC6 | Pure Projection and Render |
| AC7 | Separated Persistence Domains |
| AC8 | Isolated Platform Behavior |
| AC9 | Evidence Cannot Be Compressed Away |

| ID | Risk domain |
|---|---|
| R1 | runtime/client authority |
| R2 | PTY/terminal hot path |
| R3 | plugin host and declarative extension surface |
| R4 | event/protocol/subscriptions |
| R5 | persistence/storage/migration |
| R6 | page/projection/render/input/focus |
| R7 | platform capabilities |
| R8 | integration/core dependency boundary |

The five test-layer IDs are `pure_unit`, `deterministic_component`,
`real_integration`, `performance_regression`, and `soak_chaos`. The nine
cross-test IDs are:

| ID | Cross-test family |
|---|---|
| X1 | server state -> snapshot/API -> TUI projection |
| X2 | command/input -> runtime mutation -> event -> client convergence |
| X3 | snapshot/reconnect -> ordering/gap -> stale/duplicate handling |
| X4 | multi-client -> detach/output/reattach -> common source of truth |
| X5 | PTY output -> parser -> detection/remote/TUI with slow consumer |
| X6 | plugin process -> versioned data/event -> declarative rendering |
| X7 | event hook -> recursion/storm/bounded queue -> recovery |
| X8 | persistence save/migration/restart -> restored projections |
| X9 | platform capability -> supported/degraded/unavailable behavior |

---

## Frozen File Map

**Create under `.codex/skills/herdr-change-pipeline/`:**

```text
README.md
AGENTS.md
SKILL.md
module.json
assets/analysis-pipeline-v1.json
assets/delivery-pipeline-v1.json
assets/analysis-run-template.json
assets/delivery-run-template.json
assets/artifact-catalog.json
assets/schemas/module.schema.json
assets/schemas/analysis-pipeline.schema.json
assets/schemas/analysis-artifacts.schema.json
assets/schemas/change-intent-package.schema.json
assets/schemas/delivery-pipeline.schema.json
assets/schemas/delivery-artifacts.schema.json
assets/schemas/evidence-record.schema.json
assets/templates/fractal-analysis-tree-template.json
assets/templates/analysis-dimension-matrix-template.json
assets/templates/option-set-template.json
assets/templates/change-intent-package-template.json
assets/templates/authorization-template.json
assets/templates/feature-architecture-checklist-template.json
assets/templates/architecture-risk-register-template.json
assets/templates/requirements-traceability-template.json
assets/templates/test-point-catalog-template.json
assets/templates/cross-test-matrix-template.json
assets/templates/performance-budget-template.json
references/analysis-guide.md
references/delivery-guide.md
references/mid-flight-adoption.md
references/module-governance.md
scripts/contracts.py
scripts/validate_analysis.py
scripts/validate_delivery.py
scripts/reference_adapter.py
scripts/validate_change_pipeline.py
tests/test_module_and_phase_contract.py
tests/test_analysis_modes_and_fractal.py
tests/test_dimensions_options_and_synthesis.py
tests/test_change_intent_and_reference_adapter.py
tests/test_delivery_authorization_and_trace.py
tests/test_delivery_tdd_and_cross_tests.py
tests/test_delivery_failure_performance_publication.py
tests/test_midflight_and_negative_scenarios.py
tests/test_docs_evals_and_cartography.py
evals/evals.json
fixtures/native-page-ready/*.json
fixtures/reference-ready/*.json
fixtures/mid-flight-feature-bugfix/*.json
fixtures/composite-conflict/*.json
fixtures/no-go/*.json
fixtures/blocked-mcp/*.json
fixtures/unauthorized-delivery/*.json
lessons/errors.md
lessons/golden-paths.md
lessons/edge-cases.md
.cartography/SYSTEM-MAP.json
```

The artifact catalog is the source of truth for every required run filename,
owning phase, schema family, required fields, mode condition, and hash policy.
This avoids dozens of drifting one-off validators while retaining exact
per-artifact enforcement.

## Public Python Interfaces

| Function | Parameters | Return |
|---|---|---|
| `validate_module` | `module: object`, `root: Path` | `list[str]` |
| `validate_phase_graph` | `manifest: object`, `expected_ids: list[str]` | `list[str]` |
| `validate_artifact_catalog` | `catalog: object` | `list[str]` |
| `validate_analysis_run` | `run_dir: Path` | `list[str]` |
| `validate_fractal_tree` | `document: object` | `list[str]` |
| `validate_dimension_matrix` | `document: object`, `modes: set[str]` | `list[str]` |
| `validate_option_set` | `document: object` | `list[str]` |
| `validate_change_intent` | `document: object` | `list[str]` |
| `adapt_reference_p14` | `p14_dir: Path`, `analysis_inputs: dict[str, object]` | `tuple[dict[str, object] | None, list[str]]` |
| `build_reference_change_intent` | `loaded: dict[str, object]`, `analysis_inputs: dict[str, object]` | `dict[str, object]` |
| `validate_delivery_run` | `run_dir: Path` | `list[str]` |
| `authorize_delivery` | `change_intent: dict[str, object]`, `authorization: dict[str, object]` | `list[str]` |
| `validate_traceability` | `document: object` | `list[str]` |
| `validate_red_evidence` | `record: object` | `list[str]` |
| `validate_test_strategy` | `document: object` | `list[str]` |
| `validate_performance_budget` | `document: object` | `list[str]` |
| `validate_publication_record` | `document: object` | `list[str]` |

All validators return deterministic error strings sorted by artifact and field.
CLI exit codes: `0` pass, `1` validation/load failure, `2` usage error.

## Test-Point Contract

| ID | What/current behavior | Expected result | Reason |
|---|---|---|---|
| TP-CHG-MODULE | Sibling module absent | Wrong identity/version/pipeline/schema/auth/required file fails | Machine-enforced module boundary |
| TP-CHG-MODES | No typed intake validator | Eleven canonical modes pass; unknown and missing conditional artifacts fail | Native/reference share governance without false prerequisites |
| TP-CHG-FRACTAL | No fractal schema | Orphans, cycles, illegal level transitions, hidden children, missing recovery fail | Complete behavior traceability |
| TP-CHG-DIMENSIONS | No dimension matrix | Twelve statuses/owners/evidence complete; unsupported omission/NA fails | No silent risk compression |
| TP-CHG-PARALLEL | No ownership contract | Duplicate artifact owner, mutable input hash, unmerged conflict, no synthesis owner fails | Deterministic parallel analysis |
| TP-CHG-OPTIONS | No option contract | Non-trivial single option without necessity evidence fails | Honest brainstorming |
| TP-CHG-HANDOFF | No common handoff | Missing goal/diff/owner/risk/test/approval/readiness/hash fails | Equivalent delivery input |
| TP-CHG-REFERENCE-ADAPTER | No adapter | Valid P14 maps losslessly; native mode works without reference records | Reference is optional adapter |
| TP-CHG-NO-GO | Prose-only stop semantics | No-go/blocked/rejected cannot create delivery or authorization | Prevent speculative implementation |
| TP-CHG-DELIVERY-AUTH | No I0 validator | False/missing/stale/overbroad authorization fails | Analysis never grants mutation |
| TP-CHG-TRACE | No full delivery trace | Missing/duplicate/cyclic evidence-to-verification link fails | Cross-session provenance |
| TP-CHG-COMPAT | Ratatui v2.1 is separate | P0-P14 artifacts remain accepted and adapter requires v2.1 | Corpus stability |
| TP-CHG-RED | No executable evidence classifier | Compile/setup/flaky/already-green failure rejected; intended assertion accepted | True TDD evidence |
| TP-CHG-CROSS | No strategy validator | Five layers and nine applicable families/waivers required | Cross-layer safety |
| TP-CHG-PERFORMANCE | Budgets are hypotheses | Missing workload/env/baseline/target/samples/result fails | Reproducible performance |
| TP-CHG-MIDFLIGHT | Interim prose contract | Existing branch/diff/commits/tests preserved; current I-phase and gaps justified | Continue safely without historical invention |
| TP-CHG-PUBLICATION | No module publication record | Staging/ancestry/ref/SHA/reindex/current evidence required | Truthful closure |

### Task 1: RED Module, Pipeline, and Artifact-Catalog Identity

**Files:**

- Create: `tests/test_module_and_phase_contract.py`
- Later GREEN: module manifest, pipeline manifests, run templates, catalog,
  schemas, `scripts/contracts.py`, and CLI.

**Interfaces:** `validate_module`, `validate_phase_graph`,
`validate_artifact_catalog`.

- [ ] **Step 1: Write compile-valid RED tests**

Use test constants:

```python
ANALYSIS_IDS = [f"A{i}" for i in range(8)]
DELIVERY_IDS = [f"I{i}" for i in range(15)]
MODES = {
    "reference_project_adaptation", "native_feature", "page", "layout",
    "design_system", "component", "interaction_flow", "behavior_correction",
    "architecture_refactor", "runtime_capability", "composite",
}

def test_module_identity_and_default_authority(self):
    self.assertEqual(self.module["module_id"], "herdr-change-pipeline")
    self.assertEqual(self.module["module_version"], "1.0.0")
    self.assertEqual(self.module["analysis_pipeline_id"], "herdr-change-intelligence-v1")
    self.assertEqual(self.module["delivery_pipeline_id"], "herdr-change-delivery-v1")
    self.assertIs(self.module["product_code_changes_authorized_by_default"], False)

def test_phase_ids_are_exact(self):
    self.assertEqual([p["id"] for p in self.analysis["phases"]], ANALYSIS_IDS)
    self.assertEqual([p["id"] for p in self.delivery["phases"]], DELIVERY_IDS)
```

Add negative copies for duplicate IDs, forward/self/cyclic dependencies,
unknown owner/status, missing gate/output, duplicate artifact ownership, and
missing schema `$id`.

- [ ] **Step 2: Run RED**

```bash
python .codex/skills/herdr-change-pipeline/tests/test_module_and_phase_contract.py
```

Expected: ERROR because the module files are absent; the test file itself
imports and parses correctly.

- [ ] **Step 3: Commit RED locally**

```bash
git add -- .codex/skills/herdr-change-pipeline/tests/test_module_and_phase_contract.py
git commit -m "test(change-pipeline): define module and phase contracts"
```

### Task 2: GREEN Module Scaffold and Phase Graphs

**Files:** Create module identity, both manifests/templates, catalog, seven
schemas, `scripts/contracts.py`, `scripts/validate_change_pipeline.py`, lesson
table headers, and empty README/AGENTS/SKILL documents with truthful
`implementation_status: scaffold` wording.

**Interfaces:** shared loaders, cycle check, module/phase/catalog validators.

- [ ] **Step 1: Implement common deterministic helpers**

`scripts/contracts.py` begins with:

```python
from __future__ import annotations

import json
from pathlib import Path
from typing import Any

STATUSES = {"queued", "running", "passed", "partial", "blocked", "failed", "skipped", "no_go", "rejected"}
MODES = {"reference_project_adaptation", "native_feature", "page", "layout", "design_system", "component", "interaction_flow", "behavior_correction", "architecture_refactor", "runtime_capability", "composite"}


def load_json(path: Path) -> tuple[Any | None, list[str]]:
    try:
        return json.loads(path.read_text(encoding="utf-8")), []
    except (OSError, json.JSONDecodeError) as error:
        return None, [f"{path}: cannot load JSON: {error}"]


def dependency_cycle(graph: dict[str, list[str]]) -> list[str] | None:
    active: list[str] = []
    done: set[str] = set()
    def visit(node: str) -> list[str] | None:
        if node in active:
            start = active.index(node)
            return active[start:] + [node]
        if node in done:
            return None
        active.append(node)
        for dependency in graph.get(node, []):
            found = visit(dependency)
            if found is not None:
                return found
        active.pop()
        done.add(node)
        return None
    for node in graph:
        found = visit(node)
        if found is not None:
            return found
    return None
```

- [ ] **Step 2: Create exact module identity**

`module.json` declares schema 1, module `1.0.0`, both pipeline IDs/version
`1.0.0`, all seven schema IDs, all required files, default authorization false,
`ratatui-design-intelligence >=2.1.0` as optional reference-adapter
compatibility, and no product/stable-runtime authority.

- [ ] **Step 3: Create exact A0-A7 and I0-I14 manifests**

Each phase has `id`, `name`, `owner`, `depends_on`, `jobs`, `outputs`, `gate`,
and `stop_statuses`. Use the IDs and names from Frozen Canonical Catalogs
verbatim. A3 declares immutable parallel groups and A6 as synthesis owner. I0
depends on validated A7 handoff plus authorization; later phases form the
approved order.

- [ ] **Step 4: Create artifact catalog and schema families**

Each catalog record has:

```json
{
  "artifact": "change-intent-package.json",
  "family": "analysis",
  "owner_phase": "A7",
  "required": true,
  "required_modes": ["all"],
  "schema_id": "https://herdr.dev/schemas/change-pipeline/1.0.0/change-intent-package.schema.json",
  "schema_version": 1,
  "required_fields": ["artifact_type", "schema_version", "package_id", "status", "analysis_modes", "input_hashes", "goals", "fractal_nodes", "dimension_summary", "current_herdr_evidence", "diffs", "authority_decisions", "selected_option", "rejected_options", "requirements", "initial_test_obligations", "risks", "approval_evidence", "delivery_readiness", "product_code_changes_authorized"]
}
```

Catalog all eight analysis and 20 delivery artifacts from the approved spec.
JSON schemas enforce a closed common envelope plus family-specific IDs;
catalog validation enforces exact per-artifact required fields.

- [ ] **Step 5: Run focused GREEN**

```bash
python .codex/skills/herdr-change-pipeline/tests/test_module_and_phase_contract.py
python .codex/skills/herdr-change-pipeline/scripts/validate_change_pipeline.py \
  module .codex/skills/herdr-change-pipeline
```

Expected: tests pass; CLI prints exact module/pipeline/artifact counts and exits
0; no Herdr product file is read or written.

- [ ] **Step 6: Commit scaffold GREEN**

```bash
git add -- .codex/skills/herdr-change-pipeline
git diff --cached --check
git commit -m "feat(change-pipeline): add versioned pipeline scaffold"
```

### Task 3: RED/GREEN A0-A3 Modes, Fractal Tree, and Dimensions

**Files:**

- Create: `tests/test_analysis_modes_and_fractal.py`
- Create: `tests/test_dimensions_options_and_synthesis.py`
- Create/modify: analysis templates, `scripts/validate_analysis.py`, fixtures.

**Interfaces:** `validate_analysis_run`, `validate_fractal_tree`,
`validate_dimension_matrix`.

- [ ] **Step 1: Write A0-A3 RED tests**

Cover all eleven modes, unknown mode, native-with-fabricated-source,
reference-without-source/license/P14, composite conflict set, parent/child
uniqueness, level adjacency, cycles, hidden descendants, required
failure/recovery leaves, all twelve dimensions, evidence-backed NA, exclusive
artifact owner, immutable input hashes, conflict owner, and synthesis owner.

Canonical fractal levels:

```python
LEVELS = ["initiative", "experience_workflow", "page", "region_layout", "component", "interaction_behavior", "state_transition", "failure_recovery"]
```

- [ ] **Step 2: Run RED and record exact missing-function failures**

```bash
python .codex/skills/herdr-change-pipeline/tests/test_analysis_modes_and_fractal.py
python .codex/skills/herdr-change-pipeline/tests/test_dimensions_options_and_synthesis.py
```

- [ ] **Step 3: Implement fractal validation**

```python
def validate_fractal_tree(document: object) -> list[str]:
    if not isinstance(document, dict) or not isinstance(document.get("nodes"), list):
        return ["fractal tree nodes must be an array"]
    errors: list[str] = []
    nodes = document["nodes"]
    by_id = {node.get("node_id"): node for node in nodes if isinstance(node, dict)}
    if len(by_id) != len(nodes):
        errors.append("fractal node ids must be unique non-empty strings")
    graph: dict[str, list[str]] = {}
    for node_id, node in by_id.items():
        parent = node.get("parent_id")
        children = node.get("children", [])
        graph[node_id] = children if isinstance(children, list) else []
        if parent is not None and parent not in by_id:
            errors.append(f"node {node_id} has unknown parent {parent}")
        if node.get("level") not in LEVELS:
            errors.append(f"node {node_id} has unknown level")
        for field in NODE_FIELDS:
            if field not in node:
                errors.append(f"node {node_id}.{field} is required")
    cycle = dependency_cycle(graph)
    if cycle:
        errors.append(f"fractal cycle detected: {' -> '.join(cycle)}")
    return sorted(errors)
```

Add reciprocal parent/child, adjacent-level, descendant status, terminal state,
authority, acceptance, and recovery-leaf checks.

- [ ] **Step 4: Implement dimension/parallel validation**

Require the exact twelve dimension IDs in Frozen Canonical Catalogs,
applicability/status/evidence/owner,
immutable input SHA-256, one output owner, explicit conflicts, and A6 synthesis
resolution. `not_applicable` needs non-empty reason plus evidence; `blocked`
needs missing input/decision.

- [ ] **Step 5: Add native/reference/composite fixtures and run GREEN**

Native fixture contains no source/license fields. Reference fixture contains
P14/stack/license hashes. Composite fixture preserves an unresolved conflict
and remains blocked until synthesis resolves it. Run both test files; expect
zero failures.

- [ ] **Step 6: Commit A0-A3**

```bash
git add -- .codex/skills/herdr-change-pipeline
git commit -m "feat(change-pipeline): validate typed fractal analysis"
```

### Task 4: RED/GREEN A4-A7 Options, Synthesis, Handoff, and Reference Adapter

**Files:**

- Tests: `test_dimensions_options_and_synthesis.py`,
  `test_change_intent_and_reference_adapter.py`
- Modify: `scripts/validate_analysis.py`
- Create: `scripts/reference_adapter.py`, option/handoff templates and fixtures.

**Interfaces:** `validate_option_set`, `validate_change_intent`,
`adapt_reference_p14`, and
`build_reference_change_intent(loaded: dict[str, object], analysis_inputs:
dict[str, object]) -> dict[str, object]`.

- [ ] **Step 1: Write RED tests**

Cover two/three options for non-trivial decisions, explicit necessity evidence
for one option, tradeoffs/authority/data/failure/performance/fallback/cost/
reversibility/tests, `go|conditional_go|no_go|blocked`, fresh A5 graph evidence
not `ready` alone, complete A7 hashes/approval/readiness, false product auth,
lossless P14 mapping, invalid V, missing stack map, incompatible license, and
native mode without P14.

- [ ] **Step 2: Implement option and handoff validators**

`validate_change_intent` must reject non-ready terminal decisions, missing
trace roots, missing diff families, missing approval, mutable/unhashed inputs,
and `product_code_changes_authorized != false`.

- [ ] **Step 3: Implement reference adapter fail-closed**

```python
def adapt_reference_p14(
    p14_dir: Path,
    analysis_inputs: dict[str, object],
) -> tuple[dict[str, object] | None, list[str]]:
    required = {
        "integration-verification.json",
        "stack-adaptation-map.json",
        "behavior-gap-matrix.json",
        "data-authority-map.json",
        "layout-fidelity-spec.json",
        "component-integration-map.json",
        "implementation-plan.json",
    }
    loaded: dict[str, object] = {}
    errors: list[str] = []
    for name in sorted(required):
        value, load_errors = load_json(p14_dir / name)
        errors.extend(load_errors)
        if value is not None:
            loaded[name] = value
    verification = loaded.get("integration-verification.json")
    if not isinstance(verification, dict) or verification.get("integration_v") != 0:
        errors.append("reference adapter requires integration_v equal zero")
    if not isinstance(verification, dict) or verification.get("product_isolation") != "passed":
        errors.append("reference adapter requires passed product isolation")
    if errors:
        return None, sorted(errors)
    return build_reference_change_intent(loaded, analysis_inputs), []
```

Build the normalized result from explicit analysis inputs and immutable P14
evidence. The helper retains artifact hashes and source IDs; it never copies
executable source or sets product authorization true:

```python
def build_reference_change_intent(
    loaded: dict[str, object],
    analysis_inputs: dict[str, object],
) -> dict[str, object]:
    artifact_hashes = analysis_inputs.get("artifact_hashes")
    if not isinstance(artifact_hashes, dict):
        raise ValueError("analysis_inputs.artifact_hashes must be an object")
    missing_hashes = set(loaded) - set(artifact_hashes)
    if missing_hashes:
        raise ValueError(
            "missing immutable artifact hashes: " + ", ".join(sorted(missing_hashes))
        )
    return {
        "artifact_type": "change-intent-package",
        "schema_version": 1,
        "package_id": analysis_inputs["package_id"],
        "status": "ready",
        "analysis_modes": analysis_inputs["analysis_modes"],
        "goals": analysis_inputs["goals"],
        "fractal_nodes": analysis_inputs["fractal_nodes"],
        "dimension_summary": analysis_inputs["dimension_summary"],
        "current_herdr_evidence": analysis_inputs["current_herdr_evidence"],
        "diffs": analysis_inputs["diffs"],
        "authority_decisions": analysis_inputs["authority_decisions"],
        "selected_option": analysis_inputs["selected_option"],
        "rejected_options": analysis_inputs["rejected_options"],
        "requirements": analysis_inputs["requirements"],
        "initial_test_obligations": analysis_inputs["initial_test_obligations"],
        "risks": analysis_inputs["risks"],
        "approval_evidence": analysis_inputs["approval_evidence"],
        "delivery_readiness": "ready",
        "product_code_changes_authorized": False,
        "reference_trace": {
            name: {
                "sha256": artifact_hashes[name],
                "source_ids": analysis_inputs["source_ids"].get(name, []),
            }
            for name in sorted(loaded)
        },
    }
```

Before calling this helper, `adapt_reference_p14` validates that every required
`analysis_inputs` key exists, `source_ids` is an object, and every SHA-256 is a
64-character lowercase hexadecimal string. Invalid input returns deterministic
adapter errors rather than allowing `KeyError` or `ValueError` to escape.

- [ ] **Step 4: Prove terminal analysis outcomes**

No-go, blocked, and rejected fixtures validate as terminal analysis results but
must not create `delivery-run.json` or `authorization.json`. Ready native and
reference fixtures produce the same handoff schema.

- [ ] **Step 5: Run GREEN and commit**

```bash
python .codex/skills/herdr-change-pipeline/tests/test_dimensions_options_and_synthesis.py
python .codex/skills/herdr-change-pipeline/tests/test_change_intent_and_reference_adapter.py
git add -- .codex/skills/herdr-change-pipeline
git commit -m "feat(change-pipeline): normalize analysis handoffs"
```

### Task 5: RED/GREEN I0-I5 Authorization, PRD, Drift, Diff, Tests, and Slices

**Files:**

- Create: `tests/test_delivery_authorization_and_trace.py`
- Create: `scripts/validate_delivery.py`
- Modify: delivery catalog/templates/fixtures.

**Interfaces:** `authorize_delivery`, `validate_traceability`,
`validate_delivery_run`.

- [ ] **Step 1: Write I0-I5 RED tests**

Reject missing/invalid/non-ready handoff, false authorization, unapproved
target, stale hash, undeclared dirty path, wrong branch/remote, stable socket
permission, missing 15 checklist entries, missing eight risk domains, stale A5
binding without I2 drift, missing current/expected/diff, missing test-point
fields, hidden slice dependency, conflicting path ownership, and missing
rollback/commit boundary.

- [ ] **Step 2: Implement authorization**

```python
def authorize_delivery(change_intent: dict[str, object], authorization: dict[str, object]) -> list[str]:
    errors = validate_change_intent(change_intent)
    if change_intent.get("delivery_readiness") != "ready":
        errors.append("change intent is not delivery-ready")
    if change_intent.get("product_code_changes_authorized") is not False:
        errors.append("analysis handoff must keep product authorization false")
    if authorization.get("product_code_changes_authorized") is not True:
        errors.append("I0 requires explicit product authorization true")
    for field in ("approved_target_id", "target_branch", "target_revision", "allowed_paths", "dirty_boundary", "remote", "approval_evidence"):
        if not authorization.get(field):
            errors.append(f"authorization.{field} is required")
    if authorization.get("stable_runtime_operations_authorized") is not False:
        errors.append("stable runtime operations must remain false")
    return sorted(set(errors))
```

- [ ] **Step 3: Implement checklist/risk/diff/test/slice validation**

Require question IDs `Q1`-`Q15`, all eight risk domain IDs, test fields
`what/current/expected/diff/expected_result/failure_signature/reason/fixture/
command/evidence_location/slice_id`, and task graph cycle/owner/rollback/commit
checks.

- [ ] **Step 4: Implement acyclic traceability**

Every requirement links backward to analysis evidence/node/dimension/decision
and forward to tests/slices/commits/verifications. Reject unknown endpoints,
duplicate link IDs, missing required link kinds, and cycles.

- [ ] **Step 5: Run GREEN and commit**

```bash
python .codex/skills/herdr-change-pipeline/tests/test_delivery_authorization_and_trace.py
git add -- .codex/skills/herdr-change-pipeline
git commit -m "feat(change-pipeline): enforce authorized delivery planning"
```

### Task 6: RED/GREEN I6-I10 Characterization, RED/GREEN Evidence, and Cross Tests

**Files:**

- Create: `tests/test_delivery_tdd_and_cross_tests.py`
- Modify: `scripts/validate_delivery.py`, evidence/cross-test templates.

**Interfaces:** `validate_red_evidence`, `validate_test_strategy`.

- [ ] **Step 1: Write RED evidence tests**

Fixtures cover genuine assertion RED, compilation failure, environment/setup
failure, unrelated test failure, flaky retry, already-green characterization,
minimal GREEN, refactor with semantic diff, zero-test selection, and missing
command output/exit code/timestamp/hash.

- [ ] **Step 2: Implement evidence classification**

```python
def validate_red_evidence(record: object) -> list[str]:
    if not isinstance(record, dict):
        return ["RED evidence must be an object"]
    errors: list[str] = []
    required = {"slice_id", "test_point_id", "command", "exit_code", "failure_class", "failure_signature", "observed_output_sha256", "timestamp", "target_behavior_missing"}
    errors.extend(f"RED.{field} is required" for field in sorted(required - set(record)))
    if record.get("exit_code") in (None, 0):
        errors.append("RED command must exit nonzero")
    if record.get("failure_class") != "behavior_assertion":
        errors.append("RED failure_class must equal behavior_assertion")
    if record.get("target_behavior_missing") is not True:
        errors.append("RED must prove target behavior missing")
    if record.get("retry_count", 0) != 0:
        errors.append("retry-only RED evidence is invalid")
    return sorted(errors)
```

- [ ] **Step 3: Implement five-layer/nine-family strategy validation**

Require exact layers `pure_unit`, `deterministic_component`,
`real_integration`, `performance_regression`, `soak_chaos`. Require exact nine
cross-family IDs from I10. Each applicable entry needs scenario, fixture,
command, expected result, evidence, cadence, owner; a waiver needs approver,
reason, expiry, and residual risk.

- [ ] **Step 4: Run GREEN and commit**

```bash
python .codex/skills/herdr-change-pipeline/tests/test_delivery_tdd_and_cross_tests.py
git add -- .codex/skills/herdr-change-pipeline
git commit -m "feat(change-pipeline): enforce TDD and cross-test evidence"
```

### Task 7: RED/GREEN I11-I14 Failure, Performance, Full Gates, and Publication

**Files:**

- Create: `tests/test_delivery_failure_performance_publication.py`
- Modify: `scripts/validate_delivery.py`, performance/publication templates.

**Interfaces:** `validate_performance_budget`,
`validate_publication_record`.

- [ ] **Step 1: Write failure/risk RED tests**

For each applicable architecture risk domain, require malformed input,
timeout/cancel/crash/leak, bounded output/queue, path/secret/security, event
storm/order, ANSI/UTF-8/EOF/resize, persistence failure, migration/compatibility,
platform degradation, recovery, and resource evidence. A non-applicable domain
requires observed evidence and approver.

- [ ] **Step 2: Implement calibrated performance validation**

```python
def validate_performance_budget(document: object) -> list[str]:
    if not isinstance(document, dict) or not isinstance(document.get("budgets"), list):
        return ["performance budgets must be an array"]
    errors: list[str] = []
    required = {"metric", "workload", "environment", "baseline", "target", "maximum_regression", "sample_count", "method", "result", "status"}
    for index, budget in enumerate(document["budgets"]):
        if not isinstance(budget, dict):
            errors.append(f"budgets[{index}] must be an object")
            continue
        errors.extend(f"budgets[{index}].{field} is required" for field in sorted(required - set(budget)))
        if not isinstance(budget.get("sample_count"), int) or budget.get("sample_count", 0) < 1:
            errors.append(f"budgets[{index}].sample_count must be positive")
        if budget.get("status") == "passed" and budget.get("result") is None:
            errors.append(f"budgets[{index}] passed without result")
    return sorted(errors)
```

- [ ] **Step 3: Implement I13/I14 publication validation**

Require focused/full/schema/platform/migration/docs/cadence results, zero
unexplained failure, exact staged path list, commit list, fetch/ancestry proof,
allowed CyPack ref, no force/upstream, local/remote SHA equality, graph index
status plus current artifact/symbol query, and zero stable-runtime/socket
operations.

- [ ] **Step 4: Run GREEN and commit**

```bash
python .codex/skills/herdr-change-pipeline/tests/test_delivery_failure_performance_publication.py
git add -- .codex/skills/herdr-change-pipeline
git commit -m "feat(change-pipeline): enforce failure and publication gates"
```

### Task 8: RED/GREEN Mid-Flight Adoption and Negative Scenarios

**Files:**

- Create: `tests/test_midflight_and_negative_scenarios.py`
- Create: `references/mid-flight-adoption.md`
- Populate all scenario fixture directories.

**Interfaces:** mid-flight analysis input and current-I-phase classifier.

- [ ] **Step 1: Write mid-flight RED tests**

Require branch/worktree/HEAD, dirty paths with hashes, existing commit chain,
observed tests/evidence, known bugs/failures, approved target, scope boundary,
preserved slices, missing analysis gates, current I-phase with evidence, next
behavior-specific RED, rollback boundary, and no invented historical RED.

- [ ] **Step 2: Implement and document adoption validation**

Already valid evidence is hash-bound and retained; invalid/stale evidence is
marked `needs_refresh`; missing historical RED becomes an explicit gap and the
next unimplemented behavior starts a new RED. The classifier may advance only
through phases whose exit artifacts validate.

- [ ] **Step 3: Prove terminal negative fixtures**

`no-go` creates no delivery run. `blocked-mcp` contains no fabricated graph
node/count/symbol. `unauthorized-delivery` fails I0. `composite-conflict` stays
blocked until A6 records a decision. `native-page-ready` has no source/license
fiction. `reference-ready` retains P14 hashes.

- [ ] **Step 4: Run GREEN and commit**

```bash
python .codex/skills/herdr-change-pipeline/tests/test_midflight_and_negative_scenarios.py
git add -- .codex/skills/herdr-change-pipeline
git commit -m "test(change-pipeline): add adoption and negative pilots"
```

### Task 9: Governance, Evals, Cartography, and Complete Module Gate

**Files:**

- Create/finalize: README, AGENTS, SKILL, four references, evals, lessons,
  `.cartography/SYSTEM-MAP.json`, `tests/test_docs_evals_and_cartography.py`.

**Interfaces:** human start command, agent routing, validator CLI, v2.1 adapter
handoff, status/stop semantics, and continuity outputs.

- [ ] **Step 1: Write docs/eval/cartography RED tests**

Require both IDs/versions; A0-A7/I0-I14; 11 modes; nine constitution rules;
eight risks; 15 questions; five layers; nine cross families; MCP block; false
default auth; stable runtime/socket ban; mid-flight; reference adapter; native,
reference, layout, runtime, composite, no-go, blocked, and unauthorized evals;
cartography V=0 only when every component has a verification.

- [ ] **Step 2: Write human and agent guides**

README gives purpose, mode router, artifacts, commands, statuses, examples,
authorization, and handoff. AGENTS enforces graph-first, phase gates, immutable
hashes, exclusive artifact ownership, synthesis, TDD, failure recording,
isolation, targeted Git, and stop rules. SKILL remains a concise router.

- [ ] **Step 3: Complete evals and cartography**

Every eval declares required and forbidden concepts. System map declares
analysis, handoff, authorization, delivery, evidence, reference adapter,
mid-flight, validators, and closure components with claims, dependencies,
verifications, and `V: 0` only after fresh gates.

- [ ] **Step 4: Run the full module gate**

```bash
python -m unittest discover \
  -s .codex/skills/herdr-change-pipeline/tests -p 'test_*.py' -v
python .codex/skills/herdr-change-pipeline/scripts/validate_change_pipeline.py \
  module .codex/skills/herdr-change-pipeline
python .codex/skills/herdr-change-pipeline/scripts/validate_change_pipeline.py \
  fixtures .codex/skills/herdr-change-pipeline/fixtures
find .codex/skills/herdr-change-pipeline -name '*.json' -type f \
  -exec python -m json.tool {} /dev/null \;
git diff --check
```

Expected: zero failed/error tests; validators pass ready fixtures and reject
negative fixtures with their exact expected errors; all JSON parses; no
unauthorized path appears.

- [ ] **Step 5: Run combined Ratatui compatibility/isolation gate**

```bash
python -m unittest discover \
  -s .codex/skills/ratatui-design-intelligence/tests -p 'test_*.py' -v
python .codex/skills/ratatui-design-intelligence/scripts/validate_v21_module.py \
  .codex/skills/ratatui-design-intelligence
python -m unittest discover \
  -s .codex/skills/herdr-change-pipeline/tests -p 'test_*.py' -v
git diff --name-only
```

Expected: both suites green; diff contains only the two skill subtrees and
approved continuity/docs plan files; no product path.

- [ ] **Step 6: Commit governance**

```bash
git add -- .codex/skills/herdr-change-pipeline
git diff --cached --name-only
git diff --cached --check
git commit -m "docs(change-pipeline): add module governance and pilots"
```

### Task 10: Continuity, Publication, and Graph Closure

**Files:**

- Modify only after complete gates:
  `.codex/CHANGE-PIPELINE-TASKS.md`, `.codex/CURRENT.md`, `.codex/TASKS.md`,
  `.codex/HANDOFF.md`.

**Interfaces:** exact completion ledger and next product authorization state.

- [ ] **Step 1: Update task truth from fresh evidence**

Mark only actually completed T0-T10 items. Record test counts, validator
outputs, fixture outcomes, commits, publication state, graph counts, current
artifact queries, remaining blockers, and explicit product authorization
state. Do not copy planned counts as observed results.

- [ ] **Step 2: Run final no-placeholder/isolation/whitespace audit**

```bash
python - <<'PY'
from pathlib import Path
terms = ["T" + "BD", "T" + "ODO", "PLACE" + "HOLDER", "FIX" + "ME"]
roots = [Path(".codex/skills/herdr-change-pipeline"), Path(".codex/skills/ratatui-design-intelligence")]
hits = [(path, term) for root in roots for path in root.rglob("*") if path.is_file() for term in terms if term in path.read_text(encoding="utf-8", errors="ignore")]
if hits:
    raise SystemExit(hits)
print("placeholder audit: PASS")
PY
git diff --check
git status --short --branch
```

Expected: no unresolved marker in executable/governance sources, no whitespace
error, and only declared user-owned/unpublished paths remain.

- [ ] **Step 3: Commit continuity separately**

```bash
git add -- \
  .codex/CHANGE-PIPELINE-TASKS.md .codex/CURRENT.md \
  .codex/TASKS.md .codex/HANDOFF.md
git diff --cached --check
git commit -m "docs(change-pipeline): close verification and continuity"
```

- [ ] **Step 4: Publish only after ancestry proof**

```bash
git fetch origin feat/native-fm master
git merge-base --is-ancestor origin/feat/native-fm HEAD
git push origin HEAD:feat/native-fm
```

Fast-forward `origin/master` only when the separately verified standing CyPack
authorization and ancestry still allow it. Never push `upstream`; never force.
Verify remote SHA with `git ls-remote` before claiming publication.

- [ ] **Step 5: Reindex and prove freshness**

Run Codebase Memory indexing only after commits. Require zero extraction errors,
record fresh node/edge counts, then query current validator functions and both
module/pipeline artifact paths. `ready` without current results is insufficient.

## Completion Gate

The module is complete only when every task above has fresh evidence; all
A0-A7/I0-I14, schema, negative, mid-flight, adapter, compatibility, failure,
performance, docs, eval, and cartography tests pass; authorization false is
rejected at I0; no unexplained defect or retry-only result exists; Git
publication is fast-forward-only with exact SHA equality; and Codebase Memory
returns current module artifacts/functions.
