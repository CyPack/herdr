from __future__ import annotations

import copy
import importlib.util
import json
from pathlib import Path
import unittest


SKILL_ROOT = Path(__file__).resolve().parents[1]
VALIDATOR_PATH = SKILL_ROOT / "scripts" / "validate_reference_pipeline.py"
PIPELINE_PATH = SKILL_ROOT / "assets" / "reference-project-pipeline-v2.json"
RUN_TEMPLATE_PATH = SKILL_ROOT / "assets" / "reference-project-run-template.json"
TEMPLATE_ROOT = SKILL_ROOT / "assets" / "templates"
SKILL_PATH = SKILL_ROOT / "SKILL.md"
MASTER_GUIDE_PATH = SKILL_ROOT / "references" / "herdr-integration-analysis-pipeline.md"
BASELINE_PATH = SKILL_ROOT / "references" / "herdr-layered-architecture-baseline.md"


def load_validator_module():
    spec = importlib.util.spec_from_file_location("validate_reference_pipeline", VALIDATOR_PATH)
    if spec is None or spec.loader is None:
        raise RuntimeError("could not load validator module")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class ReferencePipelineValidatorTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.validator = load_validator_module()
        cls.pipeline = json.loads(PIPELINE_PATH.read_text(encoding="utf-8"))
        cls.run_template = json.loads(RUN_TEMPLATE_PATH.read_text(encoding="utf-8"))

    def test_canonical_pipeline_passes(self):
        self.assertEqual(self.validator.validate_pipeline(self.pipeline), [])

    def test_master_pipeline_has_source_and_herdr_integration_phases(self):
        self.assertEqual(
            [phase["id"] for phase in self.pipeline["phases"]],
            [f"P{index}" for index in range(15)],
        )
        self.assertEqual(
            [phase["name"] for phase in self.pipeline["phases"][8:]],
            [
                "herdr_current_state_cartography",
                "behavior_gap_and_coverage_analysis",
                "data_authority_contract_and_transport_map",
                "cell_level_layout_fidelity_and_responsive_optimization",
                "component_input_focus_and_overlay_integration_blueprint",
                "implementation_slicing_tdd_and_cross_test_plan",
                "integration_readiness_audit",
            ],
        )

    def test_master_pipeline_requires_integration_intelligence_artifacts(self):
        required = set(self.pipeline["required_artifacts"])
        self.assertTrue(
            {
                "herdr-system-map.json",
                "behavior-gap-matrix.json",
                "data-authority-map.json",
                "layout-fidelity-spec.json",
                "component-integration-map.json",
                "implementation-plan.json",
                "integration-verification.json",
            }.issubset(required)
        )

    def test_integration_artifact_templates_are_declared_and_valid(self):
        errors = self.validator.validate_artifact_templates(self.pipeline, SKILL_ROOT)
        self.assertEqual(errors, [])

    def test_behavior_gap_template_has_complete_behavior_contract(self):
        template = json.loads(
            (TEMPLATE_ROOT / "behavior-gap-matrix-template.json").read_text(
                encoding="utf-8"
            )
        )
        fields = set(template["record_contract"])
        self.assertTrue(
            {
                "behavior_id",
                "trigger",
                "preconditions",
                "reference_transition",
                "visible_result",
                "failure_result",
                "current_herdr_coverage",
                "target_action",
                "state_owner",
                "acceptance_tests",
            }.issubset(fields)
        )

    def test_data_authority_template_has_provenance_and_failure_contract(self):
        template = json.loads(
            (TEMPLATE_ROOT / "data-authority-map-template.json").read_text(
                encoding="utf-8"
            )
        )
        fields = set(template["record_contract"])
        self.assertTrue(
            {
                "reference_source",
                "herdr_authority",
                "transport",
                "normalization",
                "refresh_trigger",
                "stale_policy",
                "failure_state",
                "verification",
            }.issubset(fields)
        )

    def test_fidelity_phase_has_strict_cell_level_gate(self):
        fidelity = self.pipeline["phases"][11]
        self.assertIn("capture_deterministic_reference_viewports", fidelity["jobs"])
        self.assertIn("define_glyph_and_color_fallback_profiles", fidelity["jobs"])
        self.assertIn("zero undeclared", fidelity["gate"])

    def test_data_authority_phase_requires_absent_as_explicit_state(self):
        data_phase = self.pipeline["phases"][10]
        self.assertIn("classify_each_datum_server_api_client_or_absent", data_phase["jobs"])
        self.assertIn("absent", data_phase["gate"])

    def test_duplicate_phase_id_fails(self):
        candidate = copy.deepcopy(self.pipeline)
        candidate["phases"][1]["id"] = "P0"
        errors = self.validator.validate_pipeline(candidate)
        self.assertTrue(any("phase ids" in error for error in errors))

    def test_unknown_dependency_fails(self):
        candidate = copy.deepcopy(self.pipeline)
        candidate["phases"][2]["depends_on"].append("P99")
        errors = self.validator.validate_pipeline(candidate)
        self.assertTrue(any("unknown phase P99" in error for error in errors))

    def test_empty_gate_fails(self):
        candidate = copy.deepcopy(self.pipeline)
        candidate["phases"][3]["gate"] = ""
        errors = self.validator.validate_pipeline(candidate)
        self.assertTrue(any("non-empty gate" in error for error in errors))

    def test_missing_required_artifact_fails(self):
        candidate = copy.deepcopy(self.pipeline)
        candidate["required_artifacts"].remove("evidence.jsonl")
        errors = self.validator.validate_pipeline(candidate)
        self.assertTrue(any("required artifact evidence.jsonl" in error for error in errors))

    def test_dependency_cycle_fails(self):
        candidate = copy.deepcopy(self.pipeline)
        candidate["phases"][0]["depends_on"] = ["P14"]
        errors = self.validator.validate_pipeline(candidate)
        self.assertTrue(any("dependency cycle" in error for error in errors))

    def test_canonical_run_template_matches_pipeline(self):
        self.assertEqual(
            self.validator.validate_run_template(self.pipeline, self.run_template), []
        )

    def test_run_template_phase_drift_fails(self):
        candidate = copy.deepcopy(self.run_template)
        candidate["phases"].pop("P13")
        errors = self.validator.validate_run_template(self.pipeline, candidate)
        self.assertTrue(any("phase keys" in error for error in errors))

    def test_run_template_has_target_and_fidelity_contracts(self):
        self.assertEqual(self.run_template["target"]["project"], "herdr")
        self.assertEqual(self.run_template["fidelity"]["unit"], "terminal_cell")
        self.assertEqual(
            self.run_template["fidelity"]["undeclared_diff_budget"], 0
        )

    def test_skill_routes_master_runs_to_v2_and_p14(self):
        skill = SKILL_PATH.read_text(encoding="utf-8")
        self.assertIn("reference-project-pipeline-v2.json", skill)
        self.assertIn("herdr-integration-analysis-pipeline.md", skill)
        self.assertIn("herdr-layered-architecture-baseline.md", skill)
        self.assertIn("complete only after P14", skill)
        self.assertNotIn("reference-project-pipeline-v1.json", skill)

    def test_master_guides_have_complete_phase_and_baseline_sections(self):
        guide = MASTER_GUIDE_PATH.read_text(encoding="utf-8")
        baseline = BASELINE_PATH.read_text(encoding="utf-8")
        for phase_id in range(8, 15):
            self.assertIn(f"## P{phase_id}", guide)
        for layer_id in range(11):
            self.assertIn(f"### L{layer_id}", baseline)


if __name__ == "__main__":
    unittest.main()
