from __future__ import annotations

import copy
import importlib.util
import json
from pathlib import Path
import unittest


SKILL_ROOT = Path(__file__).resolve().parents[1]
PIPELINE_PATH = SKILL_ROOT / "assets" / "reference-project-pipeline-v2.json"
RUN_PATH = SKILL_ROOT / "assets" / "reference-project-run-template.json"
VALIDATOR_PATH = SKILL_ROOT / "scripts" / "validate_reference_pipeline.py"
STACK_ARTIFACT = "stack-adaptation-map.json"
STACK_TEMPLATE = "assets/templates/stack-adaptation-map-template.json"
EXPECTED_JOBS = {
    "P2": "create_stack_adaptation_source_semantics",
    "P5": "classify_stack_translation_mode_and_license_boundary",
    "P9": "finalize_stack_target_binding_and_semantic_diff",
    "P14": "audit_stack_adaptation_traceability_and_isolation",
}


def load_validator_module():
    spec = importlib.util.spec_from_file_location(
        "validate_reference_pipeline", VALIDATOR_PATH
    )
    if spec is None or spec.loader is None:
        raise RuntimeError("could not load reference pipeline validator")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


VALIDATOR = load_validator_module()
PIPELINE = json.loads(PIPELINE_PATH.read_text(encoding="utf-8"))
RUN_TEMPLATE = json.loads(RUN_PATH.read_text(encoding="utf-8"))


def phase(pipeline: dict[str, object], phase_id: str) -> dict[str, object]:
    return next(item for item in pipeline["phases"] if item["id"] == phase_id)


def bindings_exist() -> bool:
    if PIPELINE.get("pipeline_version") != "2.1.0":
        return False
    if STACK_ARTIFACT not in PIPELINE.get("required_artifacts", []):
        return False
    if PIPELINE.get("artifact_templates", {}).get(STACK_ARTIFACT) != STACK_TEMPLATE:
        return False
    for phase_id, job_id in EXPECTED_JOBS.items():
        current = phase(PIPELINE, phase_id)
        if job_id not in current.get("jobs", []):
            return False
        if STACK_ARTIFACT not in current.get("outputs", []):
            return False
    return (
        RUN_TEMPLATE.get("module_version") == "2.1.0"
        and RUN_TEMPLATE.get("pipeline_version") == "2.1.0"
        and isinstance(RUN_TEMPLATE.get("stack_adaptation"), dict)
    )


BINDINGS_EXIST = bindings_exist()


class V21PhaseBindingPresenceTests(unittest.TestCase):
    def test_stack_phase_and_run_bindings_exist(self):
        self.assertTrue(
            BINDINGS_EXIST,
            "P2/P5/P9/P14 and run state must bind stack adaptation",
        )


@unittest.skipUnless(
    BINDINGS_EXIST,
    "phase/run bindings are the missing RED behavior",
)
class V21PhaseAndRunContractTests(unittest.TestCase):
    def test_pipeline_versions_and_stack_artifact_are_declared(self):
        self.assertEqual(PIPELINE["pipeline_version"], "2.1.0")
        self.assertIn(STACK_ARTIFACT, PIPELINE["required_artifacts"])
        self.assertEqual(
            PIPELINE["artifact_templates"][STACK_ARTIFACT], STACK_TEMPLATE
        )

    def test_stack_artifact_is_phase_owned(self):
        for phase_id, job_id in EXPECTED_JOBS.items():
            with self.subTest(phase=phase_id):
                current = phase(PIPELINE, phase_id)
                self.assertIn(job_id, current["jobs"])
                self.assertIn(STACK_ARTIFACT, current["outputs"])
                self.assertIn("stack", current["gate"].lower())

    def test_run_tracks_stack_artifact_without_authority(self):
        self.assertEqual(RUN_TEMPLATE["module_version"], "2.1.0")
        self.assertEqual(RUN_TEMPLATE["pipeline_version"], "2.1.0")
        self.assertEqual(
            RUN_TEMPLATE["stack_adaptation"],
            {
                "artifact": STACK_ARTIFACT,
                "schema_version": 1,
                "status": "queued",
                "sha256": None,
                "pending_fields": [],
                "blocker": None,
            },
        )
        self.assertIs(
            RUN_TEMPLATE["target"]["product_code_changes_authorized"], False
        )

    def test_run_verification_tracks_stack_gates(self):
        verification = RUN_TEMPLATE["verification"]
        self.assertIsNone(verification["stack_schema"])
        self.assertIsNone(verification["stack_traceability"])
        self.assertIsNone(verification["stack_product_isolation"])

    def test_canonical_pipeline_and_run_pass_validator(self):
        self.assertEqual(VALIDATOR.validate_pipeline(PIPELINE), [])
        self.assertEqual(
            VALIDATOR.validate_artifact_templates(PIPELINE, SKILL_ROOT), []
        )
        self.assertEqual(
            VALIDATOR.validate_run_template(PIPELINE, RUN_TEMPLATE), []
        )

    def test_missing_phase_job_fails(self):
        candidate = copy.deepcopy(PIPELINE)
        phase(candidate, "P5")["jobs"].remove(EXPECTED_JOBS["P5"])
        errors = VALIDATOR.validate_pipeline(candidate)
        self.assertTrue(
            any("phase P5 must bind stack job" in error for error in errors)
        )

    def test_missing_stack_output_fails(self):
        candidate = copy.deepcopy(PIPELINE)
        phase(candidate, "P14")["outputs"].remove(STACK_ARTIFACT)
        errors = VALIDATOR.validate_pipeline(candidate)
        self.assertTrue(
            any("phase P14 must output stack-adaptation-map.json" in error for error in errors)
        )

    def test_invalid_run_stack_status_fails(self):
        candidate = copy.deepcopy(RUN_TEMPLATE)
        candidate["stack_adaptation"]["status"] = "complete"
        errors = VALIDATOR.validate_run_template(PIPELINE, candidate)
        self.assertIn("run stack_adaptation status is unknown", errors)

    def test_run_cannot_omit_stack_verification(self):
        candidate = copy.deepcopy(RUN_TEMPLATE)
        del candidate["verification"]["stack_traceability"]
        errors = VALIDATOR.validate_run_template(PIPELINE, candidate)
        self.assertIn(
            "run verification stack_traceability must be present and null",
            errors,
        )


if __name__ == "__main__":
    unittest.main()
