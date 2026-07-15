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
