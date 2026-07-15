#!/usr/bin/env python3
"""Validate the canonical reference-project intelligence pipeline manifest."""

from __future__ import annotations

import json
from pathlib import Path
import re
import sys
from typing import Any


EXPECTED_PHASE_IDS = [f"P{index}" for index in range(15)]
EXPECTED_PIPELINE_ID = "reference-project-intelligence-v2"
EXPECTED_PIPELINE_VERSION = "2.1.0"
EXPECTED_SCOPE = "research-corpus-and-integration-intelligence-only"
REQUIRED_STATUSES = {
    "queued",
    "running",
    "passed",
    "partial",
    "blocked",
    "failed",
    "skipped",
}
REQUIRED_ARTIFACTS = {
    "run.json",
    "identity.json",
    "index.json",
    "evidence.jsonl",
    "verification.json",
    "ANALYSIS.md",
    "component-catalog.json",
    "data-flow.json",
    ".cartography/SYSTEM-MAP.json",
    "classification.json",
    "herdr-system-map.json",
    "behavior-gap-matrix.json",
    "data-authority-map.json",
    "layout-fidelity-spec.json",
    "component-integration-map.json",
    "implementation-plan.json",
    "integration-verification.json",
    "stack-adaptation-map.json",
}
INTEGRATION_ARTIFACTS = {
    "herdr-system-map.json",
    "behavior-gap-matrix.json",
    "data-authority-map.json",
    "layout-fidelity-spec.json",
    "component-integration-map.json",
    "implementation-plan.json",
    "integration-verification.json",
    "stack-adaptation-map.json",
}
STACK_ARTIFACT = "stack-adaptation-map.json"
STACK_PHASE_BINDINGS = {
    "P2": "create_stack_adaptation_source_semantics",
    "P5": "classify_stack_translation_mode_and_license_boundary",
    "P9": "finalize_stack_target_binding_and_semantic_diff",
    "P14": "audit_stack_adaptation_traceability_and_isolation",
}
REQUIRED_INTEGRATION_ROLES = {
    "herdr_cartographer",
    "behavior_gap_analyst",
    "data_authority_analyst",
    "layout_fidelity_analyst",
    "integration_architect",
    "integration_test_designer",
}
REQUIRED_AUTHORITY_VALUES = {
    "server",
    "api_transport",
    "client_presentation",
    "absent",
    "out_of_scope",
}
REQUIRED_TEST_LAYERS = {
    "state_and_reducer",
    "geometry_and_view",
    "buffer_and_golden",
    "input_and_mouse",
    "focus_and_overlay",
    "adapter_and_contract",
    "api_and_event_cross",
    "multi_client_and_reconnect",
    "capability_and_platform",
    "benchmark_and_soak",
}


def _dependency_cycle(graph: dict[str, list[str]]) -> list[str] | None:
    visiting: set[str] = set()
    visited: set[str] = set()
    path: list[str] = []

    def visit(node: str) -> list[str] | None:
        if node in visiting:
            start = path.index(node)
            return path[start:] + [node]
        if node in visited:
            return None

        visiting.add(node)
        path.append(node)
        for dependency in graph.get(node, []):
            cycle = visit(dependency)
            if cycle is not None:
                return cycle
        path.pop()
        visiting.remove(node)
        visited.add(node)
        return None

    for node in graph:
        cycle = visit(node)
        if cycle is not None:
            return cycle
    return None


def validate_pipeline(manifest: Any) -> list[str]:
    errors: list[str] = []
    if not isinstance(manifest, dict):
        return ["pipeline manifest must be a JSON object"]

    if manifest.get("schema_version") != 2:
        errors.append("schema_version must equal 2")
    if manifest.get("pipeline_id") != EXPECTED_PIPELINE_ID:
        errors.append(f"pipeline_id must equal {EXPECTED_PIPELINE_ID}")
    if manifest.get("pipeline_version") != EXPECTED_PIPELINE_VERSION:
        errors.append(f"pipeline_version must equal {EXPECTED_PIPELINE_VERSION}")
    if manifest.get("scope") != EXPECTED_SCOPE:
        errors.append(f"scope must equal {EXPECTED_SCOPE}")

    statuses = manifest.get("statuses")
    if not isinstance(statuses, list) or set(statuses) != REQUIRED_STATUSES:
        errors.append("statuses must contain the canonical status set exactly")

    roles = manifest.get("roles")
    role_set = set(roles) if isinstance(roles, list) else set()
    if not roles or len(role_set) != len(roles):
        errors.append("roles must be a non-empty unique list")
    for role in sorted(REQUIRED_INTEGRATION_ROLES - role_set):
        errors.append(f"required integration role {role} is missing")

    phases = manifest.get("phases")
    if not isinstance(phases, list):
        return errors + ["phases must be an array"]

    phase_ids = [phase.get("id") for phase in phases if isinstance(phase, dict)]
    if phase_ids != EXPECTED_PHASE_IDS or len(set(phase_ids)) != len(phase_ids):
        errors.append("phase ids must be unique and ordered exactly P0 through P14")
    known_phase_ids = set(phase_ids)
    dependency_graph: dict[str, list[str]] = {}
    all_jobs: set[str] = set()

    for phase in phases:
        if not isinstance(phase, dict):
            errors.append("every phase must be a JSON object")
            continue
        phase_id = phase.get("id")
        if not isinstance(phase_id, str):
            errors.append("every phase needs a string id")
            continue

        owner = phase.get("owner")
        if owner not in role_set:
            errors.append(f"phase {phase_id} owner {owner!r} is not a declared role")

        dependencies = phase.get("depends_on")
        if not isinstance(dependencies, list) or not all(
            isinstance(dependency, str) for dependency in dependencies
        ):
            errors.append(f"phase {phase_id} depends_on must be a string array")
            dependencies = []
        dependency_graph[phase_id] = dependencies
        for dependency in dependencies:
            if dependency not in known_phase_ids:
                errors.append(f"phase {phase_id} depends on unknown phase {dependency}")
            if dependency == phase_id:
                errors.append(f"phase {phase_id} cannot depend on itself")
            elif dependency in known_phase_ids and int(dependency[1:]) >= int(phase_id[1:]):
                errors.append(f"phase {phase_id} cannot depend on later phase {dependency}")

        jobs = phase.get("jobs")
        if not isinstance(jobs, list) or not jobs or not all(
            isinstance(job, str) and job for job in jobs
        ):
            errors.append(f"phase {phase_id} jobs must be a non-empty string array")
        else:
            for job in jobs:
                if job in all_jobs:
                    errors.append(f"job id {job} is duplicated")
                all_jobs.add(job)

        outputs = phase.get("outputs")
        if not isinstance(outputs, list) or not outputs:
            errors.append(f"phase {phase_id} outputs must be a non-empty array")

        gate = phase.get("gate")
        if not isinstance(gate, str) or not gate.strip():
            errors.append(f"phase {phase_id} needs a non-empty gate")

        stack_job = STACK_PHASE_BINDINGS.get(phase_id)
        if stack_job is not None:
            if not isinstance(jobs, list) or stack_job not in jobs:
                errors.append(f"phase {phase_id} must bind stack job {stack_job}")
            if not isinstance(outputs, list) or STACK_ARTIFACT not in outputs:
                errors.append(
                    f"phase {phase_id} must output stack-adaptation-map.json"
                )
            if not isinstance(gate, str) or "stack" not in gate.lower():
                errors.append(f"phase {phase_id} gate must audit stack adaptation")

    cycle = _dependency_cycle(dependency_graph)
    if cycle is not None:
        errors.append(f"dependency cycle detected: {' -> '.join(cycle)}")

    parallel_groups = manifest.get("parallel_groups")
    if not isinstance(parallel_groups, list):
        errors.append("parallel_groups must be an array")
    else:
        for group in parallel_groups:
            if not isinstance(group, list) or len(group) < 2:
                errors.append("every parallel group must contain at least two phases")
                continue
            for phase_id in group:
                if phase_id not in known_phase_ids:
                    errors.append(f"parallel group references unknown phase {phase_id}")

    artifacts = manifest.get("required_artifacts")
    artifact_set = set(artifacts) if isinstance(artifacts, list) else set()
    for artifact in sorted(REQUIRED_ARTIFACTS - artifact_set):
        errors.append(f"required artifact {artifact} is missing")

    authority = manifest.get("authority_contract")
    if not isinstance(authority, dict):
        errors.append("authority_contract must be an object")
    elif set(authority.get("allowed_values", [])) != REQUIRED_AUTHORITY_VALUES:
        errors.append("authority_contract allowed_values must contain the canonical set")

    fidelity = manifest.get("fidelity_contract")
    if not isinstance(fidelity, dict):
        errors.append("fidelity_contract must be an object")
    else:
        if fidelity.get("unit") != "terminal_cell":
            errors.append("fidelity unit must be terminal_cell")
        if fidelity.get("undeclared_diff_budget") != 0:
            errors.append("fidelity undeclared_diff_budget must equal zero")
        if not fidelity.get("canonical_viewports"):
            errors.append("fidelity canonical_viewports must be non-empty")
        if not fidelity.get("fallback_profiles"):
            errors.append("fidelity fallback_profiles must be non-empty")

    test_contract = manifest.get("test_contract")
    if not isinstance(test_contract, dict):
        errors.append("test_contract must be an object")
    else:
        if set(test_contract.get("layers", [])) != REQUIRED_TEST_LAYERS:
            errors.append("test_contract layers must contain the canonical set")
        if test_contract.get("test_first") is not True:
            errors.append("test_contract test_first must be true")
        if test_contract.get("fresh_evidence_before_completion") is not True:
            errors.append("test_contract fresh_evidence_before_completion must be true")

    completion = manifest.get("completion_contract")
    if not isinstance(completion, dict):
        errors.append("completion_contract must be an object")
    else:
        if completion.get("mcp_unavailable") != "blocked":
            errors.append("MCP-unavailable behavior must be blocked")
        if completion.get("unresolved_claims") != "partial":
            errors.append("unresolved claims must produce partial status")
        if completion.get("product_code_changes") != "forbidden_without_separate_user_request":
            errors.append("product code changes must require a separate user request")

    return errors


def validate_run_template(pipeline: Any, run_template: Any) -> list[str]:
    errors: list[str] = []
    if not isinstance(pipeline, dict) or not isinstance(run_template, dict):
        return ["pipeline and run template must both be JSON objects"]

    if run_template.get("schema_version") != pipeline.get("schema_version"):
        errors.append("run template schema_version must match the pipeline")
    if run_template.get("pipeline_id") != pipeline.get("pipeline_id"):
        errors.append("run template pipeline_id must match the pipeline")
    if run_template.get("module_version") != EXPECTED_PIPELINE_VERSION:
        errors.append("run template module_version must equal 2.1.0")
    if run_template.get("pipeline_version") != pipeline.get("pipeline_version"):
        errors.append("run template pipeline_version must match the pipeline")

    target = run_template.get("target")
    if not isinstance(target, dict) or target.get("project") != "herdr":
        errors.append("run template target project must be herdr")
    else:
        if target.get("product_code_changes_authorized") is not False:
            errors.append("run template must not authorize product code changes")
        if target.get("stable_runtime_operations_authorized") is not False:
            errors.append("run template must not authorize stable runtime operations")

    stack = run_template.get("stack_adaptation")
    if not isinstance(stack, dict):
        errors.append("run stack_adaptation must be an object")
    else:
        if stack.get("artifact") != STACK_ARTIFACT:
            errors.append("run stack_adaptation artifact must match the pipeline")
        if stack.get("schema_version") != 1:
            errors.append("run stack_adaptation schema_version must equal 1")
        if stack.get("status") not in pipeline.get("statuses", []):
            errors.append("run stack_adaptation status is unknown")
        sha256 = stack.get("sha256")
        if sha256 is not None and not (
            isinstance(sha256, str) and re.fullmatch(r"[0-9a-f]{64}", sha256)
        ):
            errors.append("run stack_adaptation sha256 must be null or lowercase hex")
        if not isinstance(stack.get("pending_fields"), list):
            errors.append("run stack_adaptation pending_fields must be an array")
        blocker = stack.get("blocker")
        if blocker is not None and not isinstance(blocker, str):
            errors.append("run stack_adaptation blocker must be null or a string")

    fidelity = run_template.get("fidelity")
    pipeline_fidelity = pipeline.get("fidelity_contract", {})
    if not isinstance(fidelity, dict):
        errors.append("run template fidelity must be an object")
    else:
        if fidelity.get("unit") != pipeline_fidelity.get("unit"):
            errors.append("run template fidelity unit must match the pipeline")
        if fidelity.get("undeclared_diff_budget") != pipeline_fidelity.get(
            "undeclared_diff_budget"
        ):
            errors.append("run template fidelity diff budget must match the pipeline")

    statuses = pipeline.get("statuses", [])
    if run_template.get("status") not in statuses:
        errors.append("run template status must be declared by the pipeline")

    phases = pipeline.get("phases", [])
    expected_phase_ids = [
        phase.get("id") for phase in phases if isinstance(phase, dict)
    ]
    run_phases = run_template.get("phases")
    if not isinstance(run_phases, dict):
        errors.append("run template phases must be an object")
        run_phases = {}
    if list(run_phases) != expected_phase_ids:
        errors.append("run template phase keys must match pipeline phase order")

    owner_by_phase = {
        phase.get("id"): phase.get("owner")
        for phase in phases
        if isinstance(phase, dict)
    }
    for phase_id, phase_state in run_phases.items():
        if not isinstance(phase_state, dict):
            errors.append(f"run template phase {phase_id} must be an object")
            continue
        if phase_state.get("status") not in statuses:
            errors.append(f"run template phase {phase_id} has an unknown status")
        if phase_state.get("owner") != owner_by_phase.get(phase_id):
            errors.append(f"run template phase {phase_id} owner does not match pipeline")
        if not isinstance(phase_state.get("attempts"), list):
            errors.append(f"run template phase {phase_id} attempts must be an array")
        if not isinstance(phase_state.get("artifacts"), list):
            errors.append(f"run template phase {phase_id} artifacts must be an array")

    verification = run_template.get("verification")
    for key in (
        "stack_schema",
        "stack_traceability",
        "stack_product_isolation",
    ):
        if not isinstance(verification, dict) or key not in verification:
            errors.append(f"run verification {key} must be present and null")
        elif verification.get(key) is not None:
            errors.append(f"run verification {key} must be present and null")

    return errors


def validate_artifact_templates(pipeline: Any, skill_root: Path) -> list[str]:
    errors: list[str] = []
    if not isinstance(pipeline, dict):
        return ["pipeline must be a JSON object"]

    mappings = pipeline.get("artifact_templates")
    if not isinstance(mappings, dict):
        return ["artifact_templates must be an object"]

    missing = INTEGRATION_ARTIFACTS - set(mappings)
    extra = set(mappings) - INTEGRATION_ARTIFACTS
    for artifact in sorted(missing):
        errors.append(f"artifact template mapping for {artifact} is missing")
    for artifact in sorted(extra):
        errors.append(f"unknown artifact template mapping {artifact}")

    root = skill_root.resolve()
    for artifact, relative_path in mappings.items():
        if not isinstance(relative_path, str) or not relative_path:
            errors.append(f"artifact template path for {artifact} must be a string")
            continue
        path = (skill_root / relative_path).resolve()
        if root not in path.parents:
            errors.append(f"artifact template path for {artifact} escapes skill root")
            continue
        try:
            template = json.loads(path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as error:
            errors.append(f"cannot load artifact template for {artifact}: {error}")
            continue
        if not isinstance(template, dict):
            errors.append(f"artifact template for {artifact} must be an object")
            continue
        expected_type = artifact.removesuffix(".json")
        if template.get("artifact_type") != expected_type:
            errors.append(
                f"artifact template for {artifact} must declare type {expected_type}"
            )
        fields = template.get("record_contract")
        if (
            not isinstance(fields, list)
            or not fields
            or not all(isinstance(field, str) and field for field in fields)
            or len(fields) != len(set(fields))
        ):
            errors.append(
                f"artifact template for {artifact} needs a unique non-empty record_contract"
            )

    return errors


def main(argv: list[str]) -> int:
    if len(argv) not in {2, 3}:
        print("usage: validate_reference_pipeline.py <pipeline.json> [run-template.json]")
        return 2

    path = Path(argv[1])
    try:
        manifest = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        print(f"FAIL: cannot load pipeline: {error}")
        return 1

    errors = validate_pipeline(manifest)
    errors.extend(validate_artifact_templates(manifest, path.resolve().parents[1]))
    if len(argv) == 3:
        run_template_path = Path(argv[2])
        try:
            run_template = json.loads(run_template_path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as error:
            print(f"FAIL: cannot load run template: {error}")
            return 1
        errors.extend(validate_run_template(manifest, run_template))
    if errors:
        for error in errors:
            print(f"FAIL: {error}")
        return 1

    phases = manifest["phases"]
    job_count = sum(len(phase["jobs"]) for phase in phases)
    print(f"PASS: {len(phases)} phases, {job_count} jobs")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
