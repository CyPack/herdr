# Ratatui Design Intelligence v2.1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> `superpowers:executing-plans` to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking. Do not dispatch subagents unless the
> user explicitly authorizes delegation.

**Goal:** Upgrade the canonical repo-local Ratatui reference intelligence
module to version `2.1.0` with a machine-enforced cross-stack adaptation map
while preserving the stable P0-P14 pipeline and product-isolation contract.

**Architecture:** Keep `reference-project-intelligence-v2` and its existing
validator intact as the compatibility owner. Add an independent module
manifest and a focused v2.1 validator for module/stack/governance contracts;
make only additive changes to P2/P5/P9/P14 and the run template.

**Tech Stack:** Python 3 standard library, `unittest`, JSON, JSON Schema Draft
2020-12, Markdown, Codebase Memory MCP, and Git. No new dependencies.

## Global Constraints

- Module version is exactly `2.1.0`; stable pipeline ID remains
  `reference-project-intelligence-v2`; pipeline schema remains `2`.
- P0-P14 IDs/order, canonical statuses, existing artifacts, authority values,
  fidelity contract, and prior integration gates remain valid.
- `stack-adaptation-map.json` is additive, required, phase-owned, run-tracked,
  validator-enforced, documented, eval-covered, and product-authority neutral.
- Translation modes are exactly `direct_api`, `structural_adapter`,
  `behavior_reimplementation`, and `reject`.
- Graph-dependent claims block when MCP evidence is unavailable.
- Product-code changes, stable runtime operations, inherited sockets, upstream
  writes, force pushes, and unrelated user files are forbidden.

---

## Frozen File Map

**Create:**

- `.codex/skills/ratatui-design-intelligence/README.md` — human entry point.
- `.codex/skills/ratatui-design-intelligence/AGENTS.md` — subtree agent rules.
- `.codex/skills/ratatui-design-intelligence/module.json` — identity/version
  source of truth.
- `.codex/skills/ratatui-design-intelligence/references/module-governance.md`
  — version, Git, authority, stop, and handoff rules.
- `.codex/skills/ratatui-design-intelligence/assets/schemas/stack-adaptation-map.schema.json`
  — Draft 2020-12 artifact schema.
- `.codex/skills/ratatui-design-intelligence/assets/templates/stack-adaptation-map-template.json`
  — empty valid run artifact.
- `.codex/skills/ratatui-design-intelligence/scripts/validate_v21_module.py`
  — module, schema, template, stack record, docs, and isolation validator.
- `.codex/skills/ratatui-design-intelligence/tests/test_v21_module_contract.py`
- `.codex/skills/ratatui-design-intelligence/tests/test_v21_stack_contract.py`
- `.codex/skills/ratatui-design-intelligence/tests/test_v21_phase_and_run_contract.py`
- `.codex/skills/ratatui-design-intelligence/tests/test_v21_governance_and_compatibility.py`

**Modify:**

- `assets/reference-project-pipeline-v2.json` — version, artifact, template,
  P2/P5/P9/P14 jobs/outputs/gates.
- `assets/reference-project-run-template.json` — module/pipeline versions,
  stack artifact state, verification fields.
- `scripts/validate_reference_pipeline.py` — additive pipeline/run binding
  checks only.
- `tests/test_validate_reference_pipeline.py` — preserve legacy tests; add the
  exact baseline count assertion only after inventory.
- `SKILL.md`, `evals/evals.json`, lessons, and `.cartography/SYSTEM-MAP.json` —
  routing and evidence consistency.

## Test Points

| ID | What/current behavior | Expected result | Reason |
|---|---|---|---|
| TP-V21-MODULE-IDENTITY | `module.json` is absent | Wrong/missing ID, `2.1.0`, pipeline ID, schema separation, or required file fails | Module compatibility must be machine-readable |
| TP-V21-STACK-ARTIFACT | Pipeline lacks stack artifact/template | Missing artifact, mapping, schema, or run state fails | Cross-stack translation cannot remain implicit prose |
| TP-V21-STACK-SCHEMA | No stack record validator exists | Every source/neutral/target/diff/license/test/evidence field validates; omission fails | Records must be implementation-auditable |
| TP-V21-TRANSLATION-MODE | No mode enum exists | Only four canonical modes pass | Classification must be deterministic |
| TP-V21-PHASE-BINDING | P2/P5/P9/P14 do not own stack work | Exact create/classify/finalize/audit jobs and outputs exist | Required artifacts need phase ownership |
| TP-V21-RUN-TEMPLATE | Run does not track stack state | Versions, artifact path/status/hash, phase evidence, and false product auth validate | Runs must resume and fail closed |
| TP-V21-DOC-CONSISTENCY | No README/AGENTS/module governance | All version/scope/commands/stop rules agree | Human and agent contracts must not drift |
| TP-V21-LEGACY-COMPAT | Existing P0-P14 baseline is green | IDs/order/statuses/old artifacts/gates remain green | v2.1 is additive |
| TP-V21-NEGATIVE-MCP | Prose says blocked | Executable/eval contract rejects graph completion without evidence | No fake graph data |
| TP-V21-ISOLATION | Run defaults false | Module, pipeline, run, docs, and validator all forbid product mutation | Research cannot acquire product authority |

### Task 1: Freeze and Commit the Existing Baseline

**Files:**

- Inspect/stage: `.codex/skills/ratatui-design-intelligence/**`
- Exclude: `.codex/skills/ratatui-design-intelligence/scripts/__pycache__/`
- Exclude: `.superpowers/**`, `src/**`, and every unrelated path.

**Interfaces:**

- Consumes: current untracked module tree and existing canonical validator.
- Produces: one reviewable baseline commit on which every v2.1 RED is based.

- [ ] **Step 1: Freeze the baseline inventory and hashes**

Run:

```bash
find .codex/skills/ratatui-design-intelligence -type f \
  ! -path '*/__pycache__/*' -print0 | sort -z | xargs -0 sha256sum \
  > /tmp/ratatui-intel-v2-baseline.sha256
git status --short --branch
```

Expected: only the Ratatui subtree plus separately owned `.superpowers/` are
untracked; the hash file is outside the repository.

- [ ] **Step 2: Run the exact baseline gates**

Run:

```bash
python -m unittest discover \
  -s .codex/skills/ratatui-design-intelligence/tests \
  -p 'test_*.py' -v
python .codex/skills/ratatui-design-intelligence/scripts/validate_reference_pipeline.py \
  .codex/skills/ratatui-design-intelligence/assets/reference-project-pipeline-v2.json \
  .codex/skills/ratatui-design-intelligence/assets/reference-project-run-template.json
```

Expected: all existing tests pass; validator prints `PASS: 15 phases, 97 jobs`.
If the observed counts differ, record the fresh counts and stop before changing
the contract.

- [ ] **Step 3: Remove generated Python cache only**

Run:

```bash
rm -rf .codex/skills/ratatui-design-intelligence/scripts/__pycache__ \
       .codex/skills/ratatui-design-intelligence/tests/__pycache__
git status --short
```

Expected: no `__pycache__` or `.pyc` remains; no source file is removed.

- [ ] **Step 4: Target-stage and commit the baseline**

Run:

```bash
git add -- .codex/skills/ratatui-design-intelligence
git diff --cached --name-only
git diff --cached --check
git commit -m "chore(ratatui-intel): preserve canonical module baseline"
```

Expected: staged paths are exclusively under the Ratatui skill root; commit
succeeds; `.superpowers/` remains untouched.

### Task 2: Add RED Module Identity Contracts

**Files:**

- Create: `tests/test_v21_module_contract.py`
- Later GREEN creates: `module.json`, `scripts/validate_v21_module.py`

**Interfaces:**

- Produces:
  `validate_module(module: object, skill_root: Path) -> list[str]` and CLI exit
  `0` pass, `1` contract failure, `2` usage error.

- [ ] **Step 1: Write compile-valid failing identity tests**

Create `tests/test_v21_module_contract.py` with:

```python
from __future__ import annotations

import importlib.util
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
VALIDATOR = ROOT / "scripts" / "validate_v21_module.py"
MODULE = ROOT / "module.json"


def load_validator():
    spec = importlib.util.spec_from_file_location("validate_v21_module", VALIDATOR)
    if spec is None or spec.loader is None:
        raise RuntimeError("could not load v2.1 validator")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class V21ModuleContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.validator = load_validator()
        cls.manifest = json.loads(MODULE.read_text(encoding="utf-8"))

def test_canonical_module_identity_passes(self):
    self.assertEqual(self.validator.validate_module(self.manifest, ROOT), [])

    def test_module_and_schema_versions_are_independent(self):
        self.assertEqual(self.manifest["module_version"], "2.1.0")
        self.assertEqual(self.manifest["pipeline_schema_version"], 2)
        self.assertNotEqual(self.manifest["module_version"], str(self.manifest["pipeline_schema_version"]))

    def test_product_authority_is_false(self):
        self.assertIs(self.manifest["product_code_changes_authorized"], False)


if __name__ == "__main__":
    unittest.main()
```

- [ ] **Step 2: Run RED and verify the intended failure**

Run:

```bash
python .codex/skills/ratatui-design-intelligence/tests/test_v21_module_contract.py
```

Expected: ERROR because `validate_v21_module.py` is absent. A syntax/import
failure in the test itself is invalid RED and must be corrected.

- [ ] **Step 3: Commit the RED contract locally**

```bash
git add -- .codex/skills/ratatui-design-intelligence/tests/test_v21_module_contract.py
git diff --cached --check
git commit -m "test(ratatui-intel): define v2.1 module identity"
```

Expected: one test file committed; branch tip is not pushed.

### Task 3: Implement Module Identity GREEN

**Files:**

- Create: `module.json`
- Create: `scripts/validate_v21_module.py`
- Test: `tests/test_v21_module_contract.py`

**Interfaces:**

- `validate_module(module: object, skill_root: Path) -> list[str]`
- `main(argv: list[str]) -> int`

- [ ] **Step 1: Create the exact module manifest**

Create `module.json` with these canonical values and complete required-file
inventory:

```json
{
  "schema_version": 1,
  "module_id": "ratatui-design-intelligence",
  "module_version": "2.1.0",
  "pipeline_id": "reference-project-intelligence-v2",
  "pipeline_version": "2.1.0",
  "pipeline_schema_version": 2,
  "compatibility_family": "reference-project-intelligence-v2",
  "scope": "research-corpus-and-integration-intelligence-only",
  "product_code_changes_authorized": false,
  "stable_runtime_operations_authorized": false,
  "required_files": [
    "SKILL.md",
    "assets/reference-project-pipeline-v2.json",
    "assets/reference-project-run-template.json",
    "scripts/validate_reference_pipeline.py",
    "scripts/validate_v21_module.py"
  ],
  "required_output_artifacts": ["stack-adaptation-map.json"],
  "handoff": {
    "type": "p14_reference_adapter_input",
    "target_module": "herdr-change-pipeline",
    "grants_product_authority": false
  }
}
```

- [ ] **Step 2: Implement the minimal identity validator**

Create `scripts/validate_v21_module.py` with constants and behavior:

```python
#!/usr/bin/env python3
from __future__ import annotations

import json
from pathlib import Path
import sys
from typing import Any

EXPECTED = {
    "module_id": "ratatui-design-intelligence",
    "module_version": "2.1.0",
    "pipeline_id": "reference-project-intelligence-v2",
    "pipeline_version": "2.1.0",
    "pipeline_schema_version": 2,
    "scope": "research-corpus-and-integration-intelligence-only",
}


def validate_module(module: Any, skill_root: Path) -> list[str]:
    if not isinstance(module, dict):
        return ["module manifest must be a JSON object"]
    errors = [
        f"{key} must equal {value}"
        for key, value in EXPECTED.items()
        if module.get(key) != value
    ]
    if module.get("product_code_changes_authorized") is not False:
        errors.append("product code authorization must be false")
    if module.get("stable_runtime_operations_authorized") is not False:
        errors.append("stable runtime authorization must be false")
    files = module.get("required_files")
    if not isinstance(files, list) or len(files) != len(set(files)):
        errors.append("required_files must be a unique array")
        files = []
    root = skill_root.resolve()
    for relative in files:
        path = (root / relative).resolve()
        if root not in path.parents or not path.is_file():
            errors.append(f"required file is missing or escapes module: {relative}")
    handoff = module.get("handoff")
    if not isinstance(handoff, dict) or handoff.get("grants_product_authority") is not False:
        errors.append("handoff must not grant product authority")
    return errors


def main(argv: list[str]) -> int:
    if len(argv) != 2:
        print("usage: validate_v21_module.py <module-root>")
        return 2
    root = Path(argv[1])
    try:
        manifest = json.loads((root / "module.json").read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        print(f"FAIL: cannot load module manifest: {error}")
        return 1
    errors = validate_module(manifest, root)
    for error in errors:
        print(f"FAIL: {error}")
    if errors:
        return 1
    print("PASS: ratatui-design-intelligence 2.1.0 module identity")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
```

Task 5 appends the schema/template paths after their RED test exists. Task 7
appends README, AGENTS, and governance after their RED test exists. This keeps
each canonical manifest state truthful and GREEN without pre-creating empty
files or weakening the final required-file contract.

- [ ] **Step 3: Run focused GREEN for identity semantics**

Run the exact unit test method using `unittest` discovery after adjusting its
fixture as described:

```bash
python -m unittest discover \
  -s .codex/skills/ratatui-design-intelligence/tests \
  -p 'test_v21_module_contract.py' -v
```

Expected: identity tests and canonical CLI pass for the truthful Task 3 file
inventory. Do not call v2.1 complete because stack and governance contracts are
not yet present.

### Task 4: Add RED Stack Schema and Translation Contracts

**Files:**

- Create: `tests/test_v21_stack_contract.py`
- Later GREEN creates schema/template and extends validator.

**Interfaces:**

- `validate_stack_schema(schema: object) -> list[str]`
- `validate_stack_document(document: object) -> list[str]`

- [ ] **Step 1: Write failing stack tests**

The test must load the new schema/template and assert:

```python
REQUIRED_RECORD_FIELDS = {
    "mapping_id", "source", "source_data_flow", "source_behavior_trigger",
    "normalized_behavior", "semantics", "target", "translation_mode",
    "functional_match", "architectural_fit", "diffs", "capability_fallbacks",
    "license_reuse", "acceptance_tests", "cross_tests", "confidence",
    "verification_evidence"
}

def test_template_record_contract_is_complete(self):
    self.assertEqual(set(self.template["record_contract"]), REQUIRED_RECORD_FIELDS)

def test_unknown_translation_mode_fails(self):
    candidate = copy.deepcopy(self.example)
    candidate["records"][0]["translation_mode"] = "visual_match"
    self.assertIn(
        "unknown translation mode visual_match",
        self.validator.validate_stack_document(candidate),
    )

def test_missing_failure_diff_fails(self):
    candidate = copy.deepcopy(self.example)
    del candidate["records"][0]["diffs"]["failure"]
    self.assertTrue(any("diffs.failure" in error for error in self.validator.validate_stack_document(candidate)))
```

- [ ] **Step 2: Run RED**

```bash
python .codex/skills/ratatui-design-intelligence/tests/test_v21_stack_contract.py
```

Expected: ERROR because schema/template or validator functions are absent.

- [ ] **Step 3: Commit RED locally**

```bash
git add -- .codex/skills/ratatui-design-intelligence/tests/test_v21_stack_contract.py
git commit -m "test(ratatui-intel): define cross-stack adaptation contract"
```

### Task 5: Implement Stack Contract GREEN

**Files:**

- Create: `assets/schemas/stack-adaptation-map.schema.json`
- Create: `assets/templates/stack-adaptation-map-template.json`
- Modify: `scripts/validate_v21_module.py`
- Test: `tests/test_v21_stack_contract.py`

**Interfaces:**

- Schema `$id`:
  `https://herdr.dev/schemas/ratatui-design-intelligence/2.1.0/stack-adaptation-map.schema.json`
- Artifact identity: `artifact_type = "stack-adaptation-map"`,
  `schema_version = 1`, `pipeline_version = "2.1.0"`.

- [ ] **Step 1: Create the Draft 2020-12 schema**

The top-level schema must be closed with `additionalProperties: false`, require
`artifact_type`, `schema_version`, `pipeline_id`, `pipeline_version`, `status`,
`source_revision`, `target_revision`, and `records`, and use `$defs.record`.
`$defs.record.required` must equal `REQUIRED_RECORD_FIELDS`; translation mode
uses exactly:

```json
"enum": [
  "direct_api",
  "structural_adapter",
  "behavior_reimplementation",
  "reject"
]
```

`diffs` requires `semantic`, `failure`, `performance`, and `ownership` arrays.
`verification_evidence` requires at least one object with `kind`, `location`,
`claim`, and `confidence` between `0` and `1`.

- [ ] **Step 2: Create a valid empty template and one complete test example**

Template root:

```json
{
  "artifact_type": "stack-adaptation-map",
  "schema_version": 1,
  "pipeline_id": "reference-project-intelligence-v2",
  "pipeline_version": "2.1.0",
  "status": "queued",
  "source_revision": null,
  "target_revision": null,
  "record_contract": [
    "mapping_id", "source", "source_data_flow", "source_behavior_trigger",
    "normalized_behavior", "semantics", "target", "translation_mode",
    "functional_match", "architectural_fit", "diffs", "capability_fallbacks",
    "license_reuse", "acceptance_tests", "cross_tests", "confidence",
    "verification_evidence"
  ],
  "records": []
}
```

The test example must populate every field with deterministic fixture values;
do not weaken `minItems` merely to accept an incomplete record.

- [ ] **Step 3: Implement focused manual validation**

Extend `validate_v21_module.py` with the exact enum and recursive required-path
checks:

```python
TRANSLATION_MODES = {
    "direct_api", "structural_adapter", "behavior_reimplementation", "reject"
}
REQUIRED_DIFFS = {"semantic", "failure", "performance", "ownership"}
REQUIRED_RECORD_FIELDS = {
    "mapping_id", "source", "source_data_flow", "source_behavior_trigger",
    "normalized_behavior", "semantics", "target", "translation_mode",
    "functional_match", "architectural_fit", "diffs", "capability_fallbacks",
    "license_reuse", "acceptance_tests", "cross_tests", "confidence",
    "verification_evidence",
}
EXPECTED_SCHEMA_ID = (
    "https://herdr.dev/schemas/ratatui-design-intelligence/2.1.0/"
    "stack-adaptation-map.schema.json"
)


def validate_stack_schema(schema: Any) -> list[str]:
    if not isinstance(schema, dict):
        return ["stack adaptation schema must be a JSON object"]
    errors: list[str] = []
    if schema.get("$schema") != "https://json-schema.org/draft/2020-12/schema":
        errors.append("stack schema must use JSON Schema Draft 2020-12")
    if schema.get("$id") != EXPECTED_SCHEMA_ID:
        errors.append("stack schema $id does not match module version 2.1.0")
    record = schema.get("$defs", {}).get("record", {})
    if set(record.get("required", [])) != REQUIRED_RECORD_FIELDS:
        errors.append("stack schema record required fields are incomplete")
    modes = record.get("properties", {}).get("translation_mode", {}).get("enum")
    if set(modes or []) != TRANSLATION_MODES:
        errors.append("stack schema translation modes are incomplete")
    return errors


def validate_stack_document(document: Any) -> list[str]:
    if not isinstance(document, dict):
        return ["stack adaptation document must be a JSON object"]
    errors: list[str] = []
    if document.get("artifact_type") != "stack-adaptation-map":
        errors.append("artifact_type must equal stack-adaptation-map")
    if document.get("schema_version") != 1:
        errors.append("stack schema_version must equal 1")
    records = document.get("records")
    if not isinstance(records, list):
        return errors + ["records must be an array"]
    seen: set[str] = set()
    for index, record in enumerate(records):
        prefix = f"records[{index}]"
        if not isinstance(record, dict):
            errors.append(f"{prefix} must be an object")
            continue
        missing = REQUIRED_RECORD_FIELDS - set(record)
        errors.extend(f"{prefix}.{field} is required" for field in sorted(missing))
        mapping_id = record.get("mapping_id")
        if not isinstance(mapping_id, str) or not mapping_id:
            errors.append(f"{prefix}.mapping_id must be non-empty")
        elif mapping_id in seen:
            errors.append(f"duplicate mapping_id {mapping_id}")
        else:
            seen.add(mapping_id)
        mode = record.get("translation_mode")
        if mode not in TRANSLATION_MODES:
            errors.append(f"unknown translation mode {mode}")
        diffs = record.get("diffs")
        if not isinstance(diffs, dict):
            errors.append(f"{prefix}.diffs must be an object")
        else:
            errors.extend(
                f"{prefix}.diffs.{name} is required"
                for name in sorted(REQUIRED_DIFFS - set(diffs))
            )
    return errors
```

Append the two new schema/template paths to `module.json.required_files` in
this same GREEN slice so the canonical module validator proves they exist.
Extend `main()` in the same slice to load the canonical schema and template,
then concatenate errors in this deterministic order: `validate_module`,
`validate_stack_schema`, `validate_stack_document`. Missing or invalid JSON is
reported by path before semantic validation and exits non-zero.

- [ ] **Step 4: Run focused GREEN and negative tests**

```bash
python .codex/skills/ratatui-design-intelligence/tests/test_v21_stack_contract.py
```

Expected: all canonical and negative stack tests pass.

- [ ] **Step 5: Commit GREEN**

```bash
git add -- \
  .codex/skills/ratatui-design-intelligence/assets/schemas/stack-adaptation-map.schema.json \
  .codex/skills/ratatui-design-intelligence/assets/templates/stack-adaptation-map-template.json \
  .codex/skills/ratatui-design-intelligence/scripts/validate_v21_module.py \
  .codex/skills/ratatui-design-intelligence/tests/test_v21_stack_contract.py
git commit -m "feat(ratatui-intel): add cross-stack adaptation contract"
```

### Task 6: Bind P2/P5/P9/P14 and Run State Test-First

**Files:**

- Create: `tests/test_v21_phase_and_run_contract.py`
- Modify: `assets/reference-project-pipeline-v2.json`
- Modify: `assets/reference-project-run-template.json`
- Modify: `scripts/validate_reference_pipeline.py`

**Interfaces:**

- P2 creates source/runtime semantics.
- P5 classifies initial translation mode and license boundary.
- P9 finalizes Herdr target symbols/match/diffs.
- P14 audits complete stack trace and product isolation.

- [ ] **Step 1: Write and run phase/run RED tests**

Tests must assert exact job IDs:

```python
EXPECTED = {
    "P2": "create_stack_adaptation_source_semantics",
    "P5": "classify_stack_translation_mode_and_license_boundary",
    "P9": "finalize_stack_target_binding_and_semantic_diff",
    "P14": "audit_stack_adaptation_traceability_and_isolation",
}

def test_stack_artifact_is_phase_owned(self):
    for phase_id, job in EXPECTED.items():
        phase = next(p for p in self.pipeline["phases"] if p["id"] == phase_id)
        self.assertIn(job, phase["jobs"])
        self.assertIn("stack-adaptation-map.json", phase["outputs"])

def test_run_tracks_stack_artifact_without_authority(self):
    self.assertEqual(self.run["stack_adaptation"]["artifact"], "stack-adaptation-map.json")
    self.assertEqual(self.run["stack_adaptation"]["status"], "queued")
    self.assertIs(self.run["target"]["product_code_changes_authorized"], False)
```

Run and expect assertion failures for missing keys/jobs, not JSON or import
errors.

- [ ] **Step 2: Make additive manifest changes**

Add `pipeline_version: "2.1.0"`; add the artifact to `required_artifacts` and
template mapping; append the exact jobs and output to P2/P5/P9/P14. Preserve
all prior values and order.

Add run fields:

```json
"module_version": "2.1.0",
"pipeline_version": "2.1.0",
"stack_adaptation": {
  "artifact": "stack-adaptation-map.json",
  "schema_version": 1,
  "status": "queued",
  "sha256": null,
  "pending_fields": [],
  "blocker": null
}
```

Add `stack_schema`, `stack_traceability`, and `stack_product_isolation` under
`verification`, all initially `null`.

- [ ] **Step 3: Extend existing validator without changing legacy semantics**

Add constants and checks for pipeline version, artifact, mapping, exact phase
bindings, run stack state, and false authorization. Do not refactor the existing
P0-P14 validator in this task.

- [ ] **Step 4: Run GREEN plus legacy suite**

```bash
python .codex/skills/ratatui-design-intelligence/tests/test_v21_phase_and_run_contract.py
python -m unittest discover \
  -s .codex/skills/ratatui-design-intelligence/tests -p 'test_*.py' -v
python .codex/skills/ratatui-design-intelligence/scripts/validate_reference_pipeline.py \
  .codex/skills/ratatui-design-intelligence/assets/reference-project-pipeline-v2.json \
  .codex/skills/ratatui-design-intelligence/assets/reference-project-run-template.json
```

Expected: all tests pass; validator still reports 15 phases with a new fresh
job count greater than 97; P0-P14 IDs/order are unchanged.

- [ ] **Step 5: Commit phase/run GREEN**

```bash
git add -- \
  .codex/skills/ratatui-design-intelligence/assets/reference-project-pipeline-v2.json \
  .codex/skills/ratatui-design-intelligence/assets/reference-project-run-template.json \
  .codex/skills/ratatui-design-intelligence/scripts/validate_reference_pipeline.py \
  .codex/skills/ratatui-design-intelligence/tests/test_v21_phase_and_run_contract.py
git commit -m "feat(ratatui-intel): bind stack adaptation to canonical phases"
```

### Task 7: Add Governance, Evals, and Compatibility Closure

**Files:**

- Create: `README.md`, `AGENTS.md`, `references/module-governance.md`
- Create: `tests/test_v21_governance_and_compatibility.py`
- Modify: `SKILL.md`, `evals/evals.json`, lessons,
  `.cartography/SYSTEM-MAP.json`, `module.json`

**Interfaces:**

- Human command: both validators plus `unittest discover`.
- Agent router: v2.1 P0-P14 -> optional sibling reference adapter.
- Completion: MCP missing=`blocked`, unresolved=`partial`, product diff=`fail`.

- [ ] **Step 1: Write governance/compatibility RED tests**

Tests must read every version-bearing document, require `2.1.0`, require the
three canonical validator commands, require product/stable-runtime exclusions,
require the sibling handoff, verify legacy phase IDs/statuses/old artifact
subset, and require an eval containing `stack-adaptation-map.json`,
`behavior_reimplementation`, `blocked`, and `product isolation`.

- [ ] **Step 2: Run RED**

Expected: failures name absent README, AGENTS, governance, or missing v2.1
routing/eval content.

- [ ] **Step 3: Write human and agent contracts**

README sections must be exactly: Purpose, Inputs, Versions, P0-P14, Outputs,
Commands, Statuses, MCP Blocking, Product Isolation, Handoff. AGENTS must state
graph-first discovery, immutable evidence, phase gates, source/target
isolation, no fake MCP data, TDD, targeted staging, CyPack-only FF publication,
and all forbidden operations. Governance defines semver, schema versioning,
baseline freeze, stop rules, atomic commits, rollback, and removal conditions.
Append `README.md`, `AGENTS.md`, and `references/module-governance.md` to the
module manifest's required-file list in this same GREEN slice.

- [ ] **Step 4: Update router, evals, lessons, and cartography**

Add v2.1 routing to SKILL without making it generic Herdr delivery. Add one
stack-adaptation eval and one negative MCP/isolation eval. Update cartography
with a stack-adaptation component, claim, verification, version, and V=0 only
after all executable gates pass. Record only newly encountered lessons in the
required tables.

- [ ] **Step 5: Run complete module closure**

```bash
python -m unittest discover \
  -s .codex/skills/ratatui-design-intelligence/tests -p 'test_*.py' -v
python .codex/skills/ratatui-design-intelligence/scripts/validate_reference_pipeline.py \
  .codex/skills/ratatui-design-intelligence/assets/reference-project-pipeline-v2.json \
  .codex/skills/ratatui-design-intelligence/assets/reference-project-run-template.json
python .codex/skills/ratatui-design-intelligence/scripts/validate_v21_module.py \
  .codex/skills/ratatui-design-intelligence
find .codex/skills/ratatui-design-intelligence -name '*.json' -type f \
  -exec python -m json.tool {} /dev/null \;
git diff --check
```

Expected: zero failed/error tests, both validators exit 0, all JSON parses,
module manifest has no missing required file, legacy contracts stay green,
and diff is confined to the Ratatui subtree.

- [ ] **Step 6: Commit governance and closure**

```bash
git add -- .codex/skills/ratatui-design-intelligence
git diff --cached --name-only
git diff --cached --check
git commit -m "docs(ratatui-intel): add v2.1 module governance"
```

Expected: targeted subtree only; commit succeeds; no push yet if the sibling
plan is intended for the same publication unit.

## v2.1 Completion Gate

Before the sibling reference adapter can begin, fresh evidence must show:

- module ID/version/schema separation passes;
- stack artifact/schema/template/record negative tests pass;
- P2/P5/P9/P14 bindings and run tracking pass;
- all pre-v2.1 P0-P14 tests remain green;
- README/AGENTS/SKILL/module/pipeline/governance/evals/cartography agree;
- MCP absence is blocked and product authority remains false;
- no non-skill product path, stable runtime, inherited socket, upstream ref,
  or unrelated user file changed.
