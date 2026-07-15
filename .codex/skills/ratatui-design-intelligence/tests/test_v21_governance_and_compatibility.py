from __future__ import annotations

import importlib.util
import json
from pathlib import Path
import unittest


SKILL_ROOT = Path(__file__).resolve().parents[1]
README_PATH = SKILL_ROOT / "README.md"
AGENTS_PATH = SKILL_ROOT / "AGENTS.md"
GOVERNANCE_PATH = SKILL_ROOT / "references" / "module-governance.md"
SKILL_PATH = SKILL_ROOT / "SKILL.md"
MODULE_PATH = SKILL_ROOT / "module.json"
PIPELINE_PATH = SKILL_ROOT / "assets" / "reference-project-pipeline-v2.json"
EVALS_PATH = SKILL_ROOT / "evals" / "evals.json"
SYSTEM_MAP_PATH = SKILL_ROOT / ".cartography" / "SYSTEM-MAP.json"
VALIDATOR_PATH = SKILL_ROOT / "scripts" / "validate_v21_module.py"
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
LEGACY_STATUSES = {
    "queued",
    "running",
    "passed",
    "partial",
    "blocked",
    "failed",
    "skipped",
}
LEGACY_ARTIFACTS = {
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
GOVERNANCE_EXISTS = (
    README_PATH.is_file()
    and AGENTS_PATH.is_file()
    and GOVERNANCE_PATH.is_file()
    and hasattr(VALIDATOR, "validate_governance_documents")
)


class V21GovernancePresenceTests(unittest.TestCase):
    def test_governance_documents_and_validator_exist(self):
        self.assertTrue(
            GOVERNANCE_EXISTS,
            "README, AGENTS, module governance, and docs validator must exist",
        )


@unittest.skipUnless(
    GOVERNANCE_EXISTS,
    "governance documents and validator are the missing RED behavior",
)
class V21GovernanceAndCompatibilityTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.readme = README_PATH.read_text(encoding="utf-8")
        cls.agents = AGENTS_PATH.read_text(encoding="utf-8")
        cls.governance = GOVERNANCE_PATH.read_text(encoding="utf-8")
        cls.skill = SKILL_PATH.read_text(encoding="utf-8")
        cls.module = json.loads(MODULE_PATH.read_text(encoding="utf-8"))
        cls.pipeline = json.loads(PIPELINE_PATH.read_text(encoding="utf-8"))
        cls.evals = json.loads(EVALS_PATH.read_text(encoding="utf-8"))
        cls.system_map = json.loads(SYSTEM_MAP_PATH.read_text(encoding="utf-8"))

    def test_readme_has_exact_sections_in_order(self):
        headings = [
            line.removeprefix("## ")
            for line in self.readme.splitlines()
            if line.startswith("## ")
        ]
        self.assertEqual(headings, README_SECTIONS)

    def test_version_and_commands_are_consistent(self):
        for document in (self.readme, self.agents, self.governance, self.skill):
            with self.subTest(document=document[:30]):
                self.assertIn("2.1.0", document)
        for command in (
            "python -m unittest discover",
            "validate_reference_pipeline.py",
            "validate_v21_module.py",
        ):
            self.assertIn(command, self.readme)

    def test_human_and_agent_contracts_fail_closed(self):
        combined = "\n".join((self.readme, self.agents, self.governance))
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
            with self.subTest(term=term):
                self.assertIn(term, combined)

    def test_module_requires_governance_files(self):
        required = set(self.module["required_files"])
        self.assertTrue(
            {
                "README.md",
                "AGENTS.md",
                "references/module-governance.md",
            }.issubset(required)
        )

    def test_skill_routes_v21_stack_handoff_without_authority(self):
        for term in (
            "2.1.0",
            "stack-adaptation-map.json",
            "behavior_reimplementation",
            "herdr-change-pipeline",
            "does not grant product authority",
        ):
            with self.subTest(term=term):
                self.assertIn(term, self.skill)

    def test_evals_cover_stack_and_blocked_isolation(self):
        by_id = {item["id"]: item for item in self.evals["evals"]}
        self.assertIn("stack-adaptation-v21", by_id)
        self.assertIn("blocked-mcp-product-isolation", by_id)
        serialized = json.dumps(self.evals, sort_keys=True)
        for term in (
            "stack-adaptation-map.json",
            "behavior_reimplementation",
            "blocked",
            "product isolation",
        ):
            self.assertIn(term, serialized)

    def test_cartography_has_verified_v21_stack_component(self):
        self.assertEqual(self.system_map["module_version"], "2.1.0")
        components = {item["id"]: item for item in self.system_map["nodes"]["components"]}
        claims = {item["id"]: item for item in self.system_map["nodes"]["claims"]}
        verifications = {
            item["id"]: item for item in self.system_map["nodes"]["verifications"]
        }
        self.assertIn("C9", components)
        self.assertEqual(claims["CL9"]["status"], "verified")
        self.assertEqual(verifications["V9"]["component_ref"], "C9")
        self.assertEqual(self.system_map["variant"]["V"], 0)

    def test_legacy_pipeline_identity_remains_additive(self):
        self.assertEqual(
            [item["id"] for item in self.pipeline["phases"]],
            [f"P{index}" for index in range(15)],
        )
        self.assertEqual(set(self.pipeline["statuses"]), LEGACY_STATUSES)
        self.assertTrue(LEGACY_ARTIFACTS.issubset(self.pipeline["required_artifacts"]))

    def test_governance_validator_passes_canonical_documents(self):
        self.assertEqual(
            VALIDATOR.validate_governance_documents(SKILL_ROOT), []
        )


if __name__ == "__main__":
    unittest.main()
