#!/usr/bin/env python3
from __future__ import annotations

import json
from pathlib import Path
import sys
from typing import Any


EXPECTED = {
    "schema_version": 1,
    "module_id": "ratatui-design-intelligence",
    "module_version": "2.1.0",
    "pipeline_id": "reference-project-intelligence-v2",
    "pipeline_version": "2.1.0",
    "pipeline_schema_version": 2,
    "compatibility_family": "reference-project-intelligence-v2",
    "scope": "research-corpus-and-integration-intelligence-only",
}
EXPECTED_SCHEMA_ID = (
    "https://herdr.dev/schemas/ratatui-design-intelligence/2.1.0/"
    "stack-adaptation-map.schema.json"
)
TRANSLATION_MODES = {
    "direct_api",
    "structural_adapter",
    "behavior_reimplementation",
    "reject",
}
REQUIRED_DIFFS = {"semantic", "failure", "performance", "ownership"}
REQUIRED_RECORD_FIELDS = {
    "mapping_id",
    "source",
    "source_data_flow",
    "source_behavior_trigger",
    "normalized_behavior",
    "semantics",
    "target",
    "translation_mode",
    "functional_match",
    "architectural_fit",
    "diffs",
    "capability_fallbacks",
    "license_reuse",
    "acceptance_tests",
    "cross_tests",
    "confidence",
    "verification_evidence",
}
STACK_STATUSES = {
    "queued",
    "running",
    "partial",
    "blocked",
    "passed",
    "failed",
    "rejected",
}
EVIDENCE_FIELDS = {"kind", "location", "claim", "confidence"}
README_SECTIONS = [
    "Purpose",
    "Inputs",
    "Versions",
    "P0-P14",
    "Outputs",
    "Commands",
    "Statuses",
    "MCP Blocking",
    "Product Isolation",
    "Handoff",
]
GOVERNANCE_FILES = {
    "README.md",
    "AGENTS.md",
    "references/module-governance.md",
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
    if not isinstance(files, list) or not all(
        isinstance(relative, str) and relative for relative in files
    ):
        errors.append("required_files must be a non-empty string array")
        files = []
    elif len(files) != len(set(files)):
        errors.append("required_files must be a unique array")
        files = []

    root = skill_root.resolve()
    for relative in files:
        path = (root / relative).resolve()
        if root not in path.parents:
            errors.append(f"required file escapes module: {relative}")
        elif not path.is_file():
            errors.append(f"required file is missing: {relative}")

    outputs = module.get("required_output_artifacts")
    if outputs != ["stack-adaptation-map.json"]:
        errors.append(
            "required_output_artifacts must equal stack-adaptation-map.json"
        )

    handoff = module.get("handoff")
    if not isinstance(handoff, dict):
        errors.append("handoff must be a JSON object")
    else:
        if handoff.get("type") != "p14_reference_adapter_input":
            errors.append("handoff type must equal p14_reference_adapter_input")
        if handoff.get("target_module") != "herdr-change-pipeline":
            errors.append("handoff target must equal herdr-change-pipeline")
        if handoff.get("grants_product_authority") is not False:
            errors.append("handoff must not grant product authority")

    return errors


def validate_stack_schema(schema: Any) -> list[str]:
    if not isinstance(schema, dict):
        return ["stack adaptation schema must be a JSON object"]

    errors: list[str] = []
    if schema.get("$schema") != "https://json-schema.org/draft/2020-12/schema":
        errors.append("stack schema must use JSON Schema Draft 2020-12")
    if schema.get("$id") != EXPECTED_SCHEMA_ID:
        errors.append("stack schema $id does not match module version 2.1.0")
    if schema.get("additionalProperties") is not False:
        errors.append("stack schema root must reject additional properties")

    definitions = schema.get("$defs")
    record = definitions.get("record") if isinstance(definitions, dict) else None
    if not isinstance(record, dict):
        return errors + ["stack schema must define $defs.record"]
    if record.get("additionalProperties") is not False:
        errors.append("stack schema record must reject additional properties")
    if set(record.get("required", [])) != REQUIRED_RECORD_FIELDS:
        errors.append("stack schema record required fields are incomplete")
    properties = record.get("properties")
    modes = (
        properties.get("translation_mode", {}).get("enum")
        if isinstance(properties, dict)
        else None
    )
    if set(modes or []) != TRANSLATION_MODES:
        errors.append("stack schema translation modes are incomplete")
    return errors


def _validate_evidence(prefix: str, evidence: Any) -> list[str]:
    if not isinstance(evidence, list) or not evidence:
        return [f"{prefix} must be a non-empty array"]

    errors: list[str] = []
    for index, item in enumerate(evidence):
        item_prefix = f"{prefix}[{index}]"
        if not isinstance(item, dict):
            errors.append(f"{item_prefix} must be an object")
            continue
        errors.extend(
            f"{item_prefix}.{field} is required"
            for field in sorted(EVIDENCE_FIELDS - set(item))
        )
        confidence = item.get("confidence")
        if not isinstance(confidence, (int, float)) or isinstance(confidence, bool):
            errors.append(f"{item_prefix}.confidence must be between 0 and 1")
        elif not 0 <= confidence <= 1:
            errors.append(f"{item_prefix}.confidence must be between 0 and 1")
    return errors


def validate_stack_document(document: Any) -> list[str]:
    if not isinstance(document, dict):
        return ["stack adaptation document must be a JSON object"]

    errors: list[str] = []
    expected_root = {
        "artifact_type": "stack-adaptation-map",
        "schema_version": 1,
        "pipeline_id": "reference-project-intelligence-v2",
        "pipeline_version": "2.1.0",
    }
    errors.extend(
        f"{key} must equal {value}"
        for key, value in expected_root.items()
        if document.get(key) != value
    )
    if document.get("status") not in STACK_STATUSES:
        errors.append(f"unknown stack status {document.get('status')}")
    contract = document.get("record_contract")
    if not isinstance(contract, list) or set(contract) != REQUIRED_RECORD_FIELDS:
        errors.append("record_contract must contain every required record field")
    elif len(contract) != len(set(contract)):
        errors.append("record_contract fields must be unique")

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
        errors.extend(
            f"{prefix}.{field} is required" for field in sorted(missing)
        )
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

        confidence = record.get("confidence")
        if not isinstance(confidence, (int, float)) or isinstance(confidence, bool):
            errors.append(f"{prefix}.confidence must be between 0 and 1")
        elif not 0 <= confidence <= 1:
            errors.append(f"{prefix}.confidence must be between 0 and 1")
        errors.extend(
            _validate_evidence(
                f"{prefix}.verification_evidence",
                record.get("verification_evidence"),
            )
        )
    return errors


def load_json(path: Path) -> tuple[Any | None, list[str]]:
    try:
        return json.loads(path.read_text(encoding="utf-8")), []
    except (OSError, json.JSONDecodeError) as error:
        return None, [f"{path}: cannot load JSON: {error}"]


def _load_text(path: Path) -> tuple[str | None, list[str]]:
    try:
        return path.read_text(encoding="utf-8"), []
    except OSError as error:
        return None, [f"{path}: cannot load text: {error}"]


def validate_governance_documents(skill_root: Path) -> list[str]:
    paths = {
        "readme": skill_root / "README.md",
        "agents": skill_root / "AGENTS.md",
        "governance": skill_root / "references" / "module-governance.md",
        "skill": skill_root / "SKILL.md",
    }
    documents: dict[str, str] = {}
    errors: list[str] = []
    for name, path in paths.items():
        content, load_errors = _load_text(path)
        errors.extend(load_errors)
        if content is not None:
            documents[name] = content

    module, module_errors = load_json(skill_root / "module.json")
    evals, eval_errors = load_json(skill_root / "evals" / "evals.json")
    system_map, map_errors = load_json(
        skill_root / ".cartography" / "SYSTEM-MAP.json"
    )
    errors.extend(module_errors + eval_errors + map_errors)
    if errors:
        return errors

    readme = documents["readme"]
    headings = [
        line.removeprefix("## ")
        for line in readme.splitlines()
        if line.startswith("## ")
    ]
    if headings != README_SECTIONS:
        errors.append("README level-two sections are not canonical or ordered")

    for name, content in documents.items():
        if "2.1.0" not in content:
            errors.append(f"{name} must declare module version 2.1.0")
    for command in (
        "python -m unittest discover",
        "validate_reference_pipeline.py",
        "validate_v21_module.py",
    ):
        if command not in readme:
            errors.append(f"README is missing validation command {command}")

    combined = "\n".join(
        documents[name] for name in ("readme", "agents", "governance")
    )
    for term in (
        "Codebase Memory",
        "blocked",
        "partial",
        "product code",
        "stable runtime",
        "targeted staging",
        "CyPack",
        "fast-forward",
        "TDD",
        "herdr-change-pipeline",
    ):
        if term not in combined:
            errors.append(f"governance documents are missing term {term}")

    if not isinstance(module, dict):
        errors.append("module governance requires an object manifest")
    elif not GOVERNANCE_FILES.issubset(set(module.get("required_files", []))):
        errors.append("module manifest does not require governance files")

    skill = documents["skill"]
    for term in (
        "stack-adaptation-map.json",
        "behavior_reimplementation",
        "herdr-change-pipeline",
        "does not grant product authority",
    ):
        if term not in skill:
            errors.append(f"SKILL routing is missing term {term}")

    eval_items = evals.get("evals") if isinstance(evals, dict) else None
    if not isinstance(eval_items, list):
        errors.append("evals must contain an evals array")
    else:
        eval_ids = {
            item.get("id") for item in eval_items if isinstance(item, dict)
        }
        for eval_id in (
            "stack-adaptation-v21",
            "blocked-mcp-product-isolation",
        ):
            if eval_id not in eval_ids:
                errors.append(f"eval coverage is missing {eval_id}")

    if not isinstance(system_map, dict):
        errors.append("cartography must be a JSON object")
    else:
        nodes = system_map.get("nodes", {})
        components = {
            item.get("id"): item
            for item in nodes.get("components", [])
            if isinstance(item, dict)
        }
        claims = {
            item.get("id"): item
            for item in nodes.get("claims", [])
            if isinstance(item, dict)
        }
        verifications = {
            item.get("id"): item
            for item in nodes.get("verifications", [])
            if isinstance(item, dict)
        }
        if system_map.get("module_version") != "2.1.0":
            errors.append("cartography module_version must equal 2.1.0")
        if "C9" not in components:
            errors.append("cartography is missing stack component C9")
        if claims.get("CL9", {}).get("status") != "verified":
            errors.append("cartography claim CL9 must be verified")
        if verifications.get("V9", {}).get("component_ref") != "C9":
            errors.append("cartography verification V9 must bind C9")
        if system_map.get("variant", {}).get("V") != 0:
            errors.append("cartography variant must close at V=0")

    return errors


def main(argv: list[str]) -> int:
    if len(argv) != 2:
        print("usage: validate_v21_module.py <module-root>")
        return 2

    root = Path(argv[1])
    manifest, errors = load_json(root / "module.json")
    schema, schema_load_errors = load_json(
        root / "assets" / "schemas" / "stack-adaptation-map.schema.json"
    )
    template, template_load_errors = load_json(
        root / "assets" / "templates" / "stack-adaptation-map-template.json"
    )
    errors.extend(schema_load_errors)
    errors.extend(template_load_errors)
    if manifest is not None:
        errors.extend(validate_module(manifest, root))
    if schema is not None:
        errors.extend(validate_stack_schema(schema))
    if template is not None:
        errors.extend(validate_stack_document(template))
    errors.extend(validate_governance_documents(root))
    for error in errors:
        print(f"FAIL: {error}")
    if errors:
        return 1

    print("PASS: ratatui-design-intelligence 2.1.0 module and stack contracts")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
