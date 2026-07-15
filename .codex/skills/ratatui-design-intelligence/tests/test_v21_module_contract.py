from __future__ import annotations

import copy
import importlib.util
import json
from pathlib import Path
import unittest


SKILL_ROOT = Path(__file__).resolve().parents[1]
VALIDATOR_PATH = SKILL_ROOT / "scripts" / "validate_v21_module.py"
MODULE_PATH = SKILL_ROOT / "module.json"
IMPLEMENTATION_EXISTS = VALIDATOR_PATH.is_file() and MODULE_PATH.is_file()


def load_validator_module():
    spec = importlib.util.spec_from_file_location(
        "validate_v21_module", VALIDATOR_PATH
    )
    if spec is None or spec.loader is None:
        raise RuntimeError("could not load v2.1 validator module")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class V21ModulePresenceTests(unittest.TestCase):
    def test_v21_validator_and_manifest_exist(self):
        self.assertTrue(
            IMPLEMENTATION_EXISTS,
            "v2.1 validator and module manifest must exist",
        )


@unittest.skipUnless(
    IMPLEMENTATION_EXISTS,
    "v2.1 validator and module manifest are the missing RED behavior",
)
class V21ModuleContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.validator = load_validator_module()
        cls.manifest = json.loads(MODULE_PATH.read_text(encoding="utf-8"))

    def test_canonical_module_identity_passes(self):
        self.assertEqual(
            self.validator.validate_module(self.manifest, SKILL_ROOT), []
        )

    def test_module_pipeline_and_schema_versions_are_independent(self):
        self.assertEqual(self.manifest["module_version"], "2.1.0")
        self.assertEqual(self.manifest["pipeline_version"], "2.1.0")
        self.assertEqual(self.manifest["pipeline_schema_version"], 2)
        self.assertNotEqual(
            self.manifest["module_version"],
            str(self.manifest["pipeline_schema_version"]),
        )

    def test_product_and_stable_runtime_authority_are_false(self):
        self.assertIs(self.manifest["product_code_changes_authorized"], False)
        self.assertIs(
            self.manifest["stable_runtime_operations_authorized"], False
        )

    def test_wrong_module_version_fails(self):
        candidate = copy.deepcopy(self.manifest)
        candidate["module_version"] = "2.0.0"
        errors = self.validator.validate_module(candidate, SKILL_ROOT)
        self.assertIn("module_version must equal 2.1.0", errors)

    def test_duplicate_required_file_fails(self):
        candidate = copy.deepcopy(self.manifest)
        candidate["required_files"].append(candidate["required_files"][0])
        errors = self.validator.validate_module(candidate, SKILL_ROOT)
        self.assertIn("required_files must be a unique array", errors)

    def test_required_file_cannot_escape_module(self):
        candidate = copy.deepcopy(self.manifest)
        candidate["required_files"] = ["../../../../AGENTS.md"]
        errors = self.validator.validate_module(candidate, SKILL_ROOT)
        self.assertTrue(any("escapes module" in error for error in errors))

    def test_handoff_cannot_grant_product_authority(self):
        candidate = copy.deepcopy(self.manifest)
        candidate["handoff"]["grants_product_authority"] = True
        errors = self.validator.validate_module(candidate, SKILL_ROOT)
        self.assertIn("handoff must not grant product authority", errors)

    def test_cli_usage_error_is_two(self):
        self.assertEqual(self.validator.main(["validate_v21_module.py"]), 2)


if __name__ == "__main__":
    unittest.main()
