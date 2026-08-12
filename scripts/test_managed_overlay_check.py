"""Tests for the managed spaces overlay coverage gate."""

from __future__ import annotations

import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

from scripts.managed_overlay_check import (
    CheckError,
    check,
    merged_fields,
    model_fields,
)


MODEL_TEMPLATE = """
/// Spaces configuration.
#[derive(Debug, Default, Clone, Deserialize)]
#[serde(default)]
pub struct SpacesConfig {{
    pub split: Vec<SpaceSplitEntry>,
    /// `[[spaces.project]]` umbrellas.
    pub project: Vec<SpaceProjectEntry>,
{extra}
    /// `[spaces.icons]` row-kind icon defaults.
    pub icons: SpaceIconsConfig,
}}

pub struct Unrelated {{
    pub ignored: Vec<u8>,
}}
"""

IO_TEMPLATE = """
pub(crate) fn merge_managed_spaces_str(config: &mut Config, content: &str) -> Vec<String> {{
    match toml::from_str::<ManagedSpacesFile>(content) {{
        Ok(managed) => {{
{extends}
            Vec::new()
        }}
        Err(err) => vec![format!("parse error: {{err}}")],
    }}
}}
"""


def _extend(name: str) -> str:
    return f"            config.spaces.{name}.extend(managed.spaces.{name});"


class ManagedOverlayCheckTest(unittest.TestCase):
    def _write(self, model_extra: str, extends: list[str]) -> tuple[Path, Path]:
        directory = Path(self.tmp.name)
        model = directory / "model.rs"
        io = directory / "io.rs"
        model.write_text(MODEL_TEMPLATE.format(extra=model_extra), encoding="utf-8")
        io.write_text(IO_TEMPLATE.format(extends="\n".join(extends)), encoding="utf-8")
        return model, io

    def setUp(self) -> None:
        self.tmp = TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)

    def test_model_fields_reads_names_and_types(self) -> None:
        model, _ = self._write("    pub node: Vec<SpaceNodeEntry>,", [])
        fields = model_fields(model.read_text(encoding="utf-8"))
        self.assertEqual(
            [(field.name, field.is_collection) for field in fields],
            [("split", True), ("project", True), ("node", True), ("icons", False)],
            "doc comments and attributes are skipped, order is source order",
        )

    def test_a_covered_merge_has_no_problems(self) -> None:
        model, io = self._write(
            "    pub node: Vec<SpaceNodeEntry>,",
            [_extend("split"), _extend("project"), _extend("node")],
        )
        self.assertEqual(check(model, io), [])

    def test_a_collection_the_merge_forgot_is_named(self) -> None:
        """The historical bug: `node` was added to the model, never merged.

        This is the mutation that matters. If the gate cannot see this shape
        it cannot have prevented the defect it exists for.
        """

        model, io = self._write(
            "    pub node: Vec<SpaceNodeEntry>,",
            [_extend("split"), _extend("project")],
        )
        problems = check(model, io)
        self.assertEqual(len(problems), 1, problems)
        self.assertIn("SpacesConfig.node", problems[0])
        self.assertIn("config.spaces.node.extend(managed.spaces.node)", problems[0])

    def test_a_non_collection_field_may_be_exempt(self) -> None:
        """`icons` is a settings table, not a list; it is exempt by name."""

        model, io = self._write("", [_extend("split"), _extend("project")])
        self.assertEqual(check(model, io), [])

    def test_a_field_that_is_neither_merged_nor_exempt_is_reported(self) -> None:
        model, io = self._write(
            "    pub layout: SpaceLayoutConfig,",
            [_extend("split"), _extend("project")],
        )
        problems = check(model, io)
        self.assertEqual(len(problems), 1, problems)
        self.assertIn("neither merged nor exempt", problems[0])

    def test_a_merge_ahead_of_the_model_is_reported(self) -> None:
        model, io = self._write(
            "", [_extend("split"), _extend("project"), _extend("ghost")]
        )
        problems = check(model, io)
        self.assertEqual(len(problems), 1, problems)
        self.assertIn("no longer has", problems[0])

    def test_a_crossed_extend_is_refused(self) -> None:
        """Two same-typed collections would type-check while merging wrongly."""

        model, io = self._write(
            "    pub node: Vec<SpaceNodeEntry>,",
            [
                _extend("split"),
                _extend("project"),
                "            config.spaces.node.extend(managed.spaces.project);",
            ],
        )
        with self.assertRaises(CheckError) as raised:
            check(model, io)
        self.assertIn("wrong collection", str(raised.exception))

    def test_a_missing_struct_is_an_error_not_a_pass(self) -> None:
        """A parse that finds nothing must fail loudly, never silently pass."""

        directory = Path(self.tmp.name)
        model = directory / "empty.rs"
        model.write_text("// no structs here\n", encoding="utf-8")
        with self.assertRaises(CheckError):
            model_fields(model.read_text(encoding="utf-8"))

    def test_a_missing_function_is_an_error(self) -> None:
        with self.assertRaises(CheckError):
            merged_fields("fn something_else() {}")

    def test_the_real_sources_are_covered(self) -> None:
        """The gate runs against the tree it ships in, not only fixtures."""

        root = Path(__file__).resolve().parents[1]
        problems = check(root / "src/config/model.rs", root / "src/config/io.rs")
        self.assertEqual(problems, [], "\n".join(problems))


if __name__ == "__main__":
    unittest.main()
