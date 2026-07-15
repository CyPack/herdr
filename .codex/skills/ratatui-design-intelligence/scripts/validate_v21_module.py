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
    for error in errors:
        print(f"FAIL: {error}")
    if errors:
        return 1

    print("PASS: ratatui-design-intelligence 2.1.0 module and stack contracts")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
