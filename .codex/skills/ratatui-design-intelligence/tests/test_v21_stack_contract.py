from __future__ import annotations

import copy
import importlib.util
import json
from pathlib import Path
import unittest


SKILL_ROOT = Path(__file__).resolve().parents[1]
VALIDATOR_PATH = SKILL_ROOT / "scripts" / "validate_v21_module.py"
SCHEMA_PATH = (
    SKILL_ROOT / "assets" / "schemas" / "stack-adaptation-map.schema.json"
)
TEMPLATE_PATH = (
    SKILL_ROOT / "assets" / "templates" / "stack-adaptation-map-template.json"
)
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


def load_validator_module():
    spec = importlib.util.spec_from_file_location(
        "validate_v21_module", VALIDATOR_PATH
    )
    if spec is None or spec.loader is None:
        raise RuntimeError("could not load v2.1 validator module")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


VALIDATOR = load_validator_module()
STACK_CONTRACT_EXISTS = (
    SCHEMA_PATH.is_file()
    and TEMPLATE_PATH.is_file()
    and hasattr(VALIDATOR, "validate_stack_schema")
    and hasattr(VALIDATOR, "validate_stack_document")
)


def complete_record() -> dict[str, object]:
    return {
        "mapping_id": "MAP-001",
        "source": {
            "source_id": "R033",
            "revision": "0123456789abcdef",
            "file": "src/layout.rs",
            "symbol": "TileTree",
            "language": "rust",
            "ui_stack": "ratatui",
        },
        "source_data_flow": ["event", "state", "view_model", "frame"],
        "source_behavior_trigger": "resize divider drag committed",
        "normalized_behavior": "resize adjacent terminal-cell regions",
        "semantics": ["bounded", "reversible", "client_owned"],
        "target": {
            "project": "herdr",
            "revision": "fedcba9876543210",
            "file": "src/ui/layout.rs",
            "symbol": "ShellLayout",
            "authority": "tui_client",
        },
        "translation_mode": "behavior_reimplementation",
        "functional_match": "partial",
        "architectural_fit": "conditional",
        "diffs": {
            "semantic": ["Herdr requires responsive minimum widths"],
            "failure": ["tiny terminal must degrade without invalid geometry"],
            "performance": ["layout remains linear in visible leaves"],
            "ownership": ["no server runtime state is introduced"],
        },
        "capability_fallbacks": ["keyboard resize", "fixed responsive layout"],
        "license_reuse": {
            "classification": "behavior_only",
            "license": "MIT",
            "copy_source": False,
        },
        "acceptance_tests": ["TP-LAYOUT-RESIZE"],
        "cross_tests": ["X9-platform-capability"],
        "confidence": 0.94,
        "verification_evidence": [
            {
                "kind": "code_graph_symbol",
                "location": "R033:src/layout.rs::TileTree",
                "claim": "source owns a bounded split tree",
                "confidence": 0.98,
            }
        ],
    }


def complete_document() -> dict[str, object]:
    return {
        "artifact_type": "stack-adaptation-map",
        "schema_version": 1,
        "pipeline_id": "reference-project-intelligence-v2",
        "pipeline_version": "2.1.0",
        "status": "partial",
        "source_revision": "0123456789abcdef",
        "target_revision": "fedcba9876543210",
        "record_contract": sorted(REQUIRED_RECORD_FIELDS),
        "records": [complete_record()],
    }


class V21StackPresenceTests(unittest.TestCase):
    def test_stack_schema_template_and_validator_exist(self):
        self.assertTrue(
            STACK_CONTRACT_EXISTS,
            "stack schema, template, and validator functions must exist",
        )


@unittest.skipUnless(
    STACK_CONTRACT_EXISTS,
    "stack schema/template/validator are the missing RED behavior",
)
class V21StackContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
        cls.template = json.loads(TEMPLATE_PATH.read_text(encoding="utf-8"))

    def test_canonical_schema_and_template_pass(self):
        self.assertEqual(VALIDATOR.validate_stack_schema(self.schema), [])
        self.assertEqual(VALIDATOR.validate_stack_document(self.template), [])

    def test_template_record_contract_is_complete(self):
        self.assertEqual(set(self.template["record_contract"]), REQUIRED_RECORD_FIELDS)

    def test_complete_record_passes(self):
        self.assertEqual(VALIDATOR.validate_stack_document(complete_document()), [])

    def test_unknown_translation_mode_fails(self):
        candidate = complete_document()
        candidate["records"][0]["translation_mode"] = "visual_match"
        self.assertIn(
            "unknown translation mode visual_match",
            VALIDATOR.validate_stack_document(candidate),
        )

    def test_missing_failure_diff_fails(self):
        candidate = complete_document()
        del candidate["records"][0]["diffs"]["failure"]
        self.assertTrue(
            any(
                "records[0].diffs.failure" in error
                for error in VALIDATOR.validate_stack_document(candidate)
            )
        )

    def test_missing_record_field_fails(self):
        candidate = complete_document()
        del candidate["records"][0]["source_data_flow"]
        self.assertIn(
            "records[0].source_data_flow is required",
            VALIDATOR.validate_stack_document(candidate),
        )

    def test_duplicate_mapping_id_fails(self):
        candidate = complete_document()
        candidate["records"].append(copy.deepcopy(candidate["records"][0]))
        self.assertIn(
            "duplicate mapping_id MAP-001",
            VALIDATOR.validate_stack_document(candidate),
        )

    def test_confidence_out_of_range_fails(self):
        candidate = complete_document()
        candidate["records"][0]["confidence"] = 1.1
        self.assertIn(
            "records[0].confidence must be between 0 and 1",
            VALIDATOR.validate_stack_document(candidate),
        )

    def test_empty_verification_evidence_fails(self):
        candidate = complete_document()
        candidate["records"][0]["verification_evidence"] = []
        self.assertIn(
            "records[0].verification_evidence must be a non-empty array",
            VALIDATOR.validate_stack_document(candidate),
        )

    def test_incomplete_evidence_record_fails(self):
        candidate = complete_document()
        del candidate["records"][0]["verification_evidence"][0]["claim"]
        self.assertIn(
            "records[0].verification_evidence[0].claim is required",
            VALIDATOR.validate_stack_document(candidate),
        )

    def test_schema_is_closed_and_has_exact_modes(self):
        record = self.schema["$defs"]["record"]
        self.assertIs(self.schema["additionalProperties"], False)
        self.assertIs(record["additionalProperties"], False)
        self.assertEqual(set(record["required"]), REQUIRED_RECORD_FIELDS)
        self.assertEqual(
            set(record["properties"]["translation_mode"]["enum"]),
            {
                "direct_api",
                "structural_adapter",
                "behavior_reimplementation",
                "reject",
            },
        )


if __name__ == "__main__":
    unittest.main()
