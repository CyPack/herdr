"""Tests for the fork's behavior-registry check.

The registry exists so an upstream sync cannot silently drop a fork behavior.
These tests pin the five failure classes the checker must catch, because a
checker that reports success on a broken registry is worse than no checker.
"""

import tempfile
import unittest
from pathlib import Path

from scripts.behavior_registry_check import check, expand_marker_ids


def _write(root: Path, rel: str, text: str) -> None:
    path = root / rel
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


REGISTRY_HEADER = "| ID | Behavior | Breaks if lost | Verified by |\n|---|---|---|---|\n"


def _registry(*rows: str) -> str:
    return "# Feature\n\n" + REGISTRY_HEADER + "".join(rows)


def _row(entry_id: str, tests: str) -> str:
    return f"| {entry_id} | does a thing | the thing stops | {tests} |\n"


class ExpandMarkerIdsTests(unittest.TestCase):
    def test_plain_marker_yields_one_id(self) -> None:
        self.assertEqual(expand_marker_ids("// TP-MTIME-01: ordering"), {"TP-MTIME-01"})

    def test_slash_run_expands_to_sibling_ids(self) -> None:
        # `TP-DCLICK-01/02/04` is the convention already used in the tree for
        # one test that pins several numbered behaviors at once.
        self.assertEqual(
            expand_marker_ids("// TP-DCLICK-01/02/04 RED: click intent"),
            {"TP-DCLICK-01", "TP-DCLICK-02", "TP-DCLICK-04"},
        )

    def test_dotted_family_is_kept_whole(self) -> None:
        self.assertEqual(expand_marker_ids("// TP-C4.1: seam"), {"TP-C4.1"})

    def test_prose_without_markers_yields_nothing(self) -> None:
        self.assertEqual(expand_marker_ids("// nothing to see"), set())


class CheckTests(unittest.TestCase):
    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        self.root = Path(self._tmp.name)
        (self.root / "behaviors").mkdir()
        (self.root / "src").mkdir()
        self.addCleanup(self._tmp.cleanup)

    def _ledger(self, *entries: str) -> None:
        """Entries are ``"ID"`` or ``"ID=test_a,test_b"``.

        The ledger carries test bindings so an undocumented behavior still gets
        the "its test disappeared" gate; only the prose description is missing.
        """
        rows = ["| ID | Verified by |", "|---|---|"]
        for entry in entries:
            entry_id, _, tests = entry.partition("=")
            names = ", ".join(f"`{t}`" for t in tests.split(",") if t)
            rows.append(f"| {entry_id} | {names} |")
        _write(self.root, "behaviors/UNDOCUMENTED.md", "# Undocumented\n\n" + "\n".join(rows) + "\n")

    def test_registry_matching_source_and_tests_passes(self) -> None:
        _write(self.root, "behaviors/fm.md", _registry(_row("TP-FM-01", "`keeps_focus`")))
        _write(self.root, "src/fm.rs", "// TP-FM-01: keeps focus\nfn keeps_focus() {}\n")
        self._ledger()
        self.assertEqual(check(self.root), [])

    # C1 — the gate this whole system exists for.
    def test_missing_test_function_is_reported(self) -> None:
        _write(self.root, "behaviors/fm.md", _registry(_row("TP-FM-01", "`keeps_focus`")))
        _write(self.root, "src/fm.rs", "// TP-FM-01: keeps focus\n")
        self._ledger()
        errors = check(self.root)
        self.assertTrue(any("keeps_focus" in e and "TP-FM-01" in e for e in errors), errors)

    # C2 — behaviour deleted along with its code.
    def test_registry_id_without_source_marker_is_reported(self) -> None:
        _write(self.root, "behaviors/fm.md", _registry(_row("TP-FM-01", "`keeps_focus`")))
        _write(self.root, "src/fm.rs", "fn keeps_focus() {}\n")
        self._ledger()
        errors = check(self.root)
        self.assertTrue(any("TP-FM-01" in e and "marker" in e for e in errors), errors)

    # C3 — a new behaviour may not arrive undocumented.
    def test_unregistered_source_marker_is_reported(self) -> None:
        _write(self.root, "behaviors/fm.md", _registry(_row("TP-FM-01", "`keeps_focus`")))
        _write(
            self.root,
            "src/fm.rs",
            "// TP-FM-01: keeps focus\n// TP-FM-02: brand new\nfn keeps_focus() {}\n",
        )
        self._ledger()
        errors = check(self.root)
        self.assertTrue(any("TP-FM-02" in e for e in errors), errors)

    def test_ledger_entry_suppresses_unregistered_marker(self) -> None:
        _write(self.root, "behaviors/fm.md", _registry(_row("TP-FM-01", "`keeps_focus`")))
        _write(
            self.root,
            "src/fm.rs",
            "// TP-FM-01: keeps focus\n// TP-FM-02: known debt\n"
            "fn keeps_focus() {}\nfn holds_debt() {}\n",
        )
        self._ledger("TP-FM-02=holds_debt")
        self.assertEqual(check(self.root), [])

    # C1 extends to the ledger: an undocumented behavior is still a behavior,
    # and losing its test in a sync is the exact failure this system prevents.
    def test_ledger_entry_losing_its_test_is_reported(self) -> None:
        _write(self.root, "behaviors/fm.md", _registry(_row("TP-FM-01", "`keeps_focus`")))
        _write(
            self.root,
            "src/fm.rs",
            "// TP-FM-01: keeps focus\n// TP-FM-02: known debt\nfn keeps_focus() {}\n",
        )
        self._ledger("TP-FM-02=holds_debt")
        errors = check(self.root)
        self.assertTrue(any("holds_debt" in e and "TP-FM-02" in e for e in errors), errors)

    def test_ledger_entry_without_any_test_is_reported(self) -> None:
        _write(self.root, "behaviors/fm.md", _registry(_row("TP-FM-01", "`keeps_focus`")))
        _write(
            self.root,
            "src/fm.rs",
            "// TP-FM-01: keeps focus\n// TP-FM-02: known debt\nfn keeps_focus() {}\n",
        )
        self._ledger("TP-FM-02")
        errors = check(self.root)
        self.assertTrue(any("TP-FM-02" in e and "no test" in e.lower() for e in errors), errors)

    # C4 — the ledger must shrink, not rot.
    def test_documented_id_still_in_ledger_is_reported(self) -> None:
        _write(self.root, "behaviors/fm.md", _registry(_row("TP-FM-01", "`keeps_focus`")))
        _write(self.root, "src/fm.rs", "// TP-FM-01: keeps focus\nfn keeps_focus() {}\n")
        self._ledger("TP-FM-01=keeps_focus")
        errors = check(self.root)
        self.assertTrue(any("TP-FM-01" in e and "ledger" in e.lower() for e in errors), errors)

    def test_stale_ledger_entry_without_any_marker_is_reported(self) -> None:
        _write(self.root, "behaviors/fm.md", _registry(_row("TP-FM-01", "`keeps_focus`")))
        _write(self.root, "src/fm.rs", "// TP-FM-01: keeps focus\nfn keeps_focus() {}\n")
        self._ledger("TP-GONE-09=keeps_focus")
        errors = check(self.root)
        self.assertTrue(any("TP-GONE-09" in e for e in errors), errors)

    # C5 — one id, one meaning.
    def test_duplicate_id_across_registries_is_reported(self) -> None:
        _write(self.root, "behaviors/fm.md", _registry(_row("TP-FM-01", "`keeps_focus`")))
        _write(self.root, "behaviors/other.md", _registry(_row("TP-FM-01", "`keeps_focus`")))
        _write(self.root, "src/fm.rs", "// TP-FM-01: keeps focus\nfn keeps_focus() {}\n")
        self._ledger()
        errors = check(self.root)
        self.assertTrue(any("TP-FM-01" in e and "duplicate" in e.lower() for e in errors), errors)

    def test_registry_row_without_tests_is_reported(self) -> None:
        _write(self.root, "behaviors/fm.md", _registry(_row("TP-FM-01", "")))
        _write(self.root, "src/fm.rs", "// TP-FM-01: keeps focus\n")
        self._ledger()
        errors = check(self.root)
        self.assertTrue(any("TP-FM-01" in e and "no test" in e.lower() for e in errors), errors)

    def test_visual_spec_pins_count_as_coverage(self) -> None:
        # Some behaviors are pinned by Playwright specs, not Rust tests. A
        # checker blind to `.ts` would report them as undocumented forever.
        _write(self.root, "behaviors/fm.md", _registry(_row("TP-VIS-01", "`shows_focus_ring`")))
        _write(self.root, "src/fm.rs", "// TP-VIS-01: focus ring\n")
        (self.root / "tests").mkdir(exist_ok=True)
        _write(self.root, "tests/visual/focus.spec.ts", "// TP-VIS-01\ntest('shows_focus_ring', ...)\n")
        self._ledger()
        self.assertEqual(check(self.root), [])

    def test_spec_test_names_with_spaces_round_trip(self) -> None:
        # Playwright test names are sentences. If the reader is stricter than
        # the writer, a generated ledger row parses back as "no test named".
        _write(self.root, "behaviors/fm.md", _registry(_row("TP-VIS-01", "`vis-01 files stage`")))
        (self.root / "tests").mkdir(exist_ok=True)
        _write(
            self.root,
            "tests/visual/nav.spec.ts",
            "// TP-VIS-01: stage snapshot\ntest(\"vis-01 files stage\", async () => {});\n",
        )
        self._ledger()
        self.assertEqual(check(self.root), [])

    def test_family_reference_is_not_treated_as_a_behavior(self) -> None:
        # `TP-B1.2 defines the mapping` points at a family; the behaviors are
        # `TP-B1.2-FAILURES` and friends. Demanding a test for the family name
        # would report a violation that cannot be fixed.
        _write(
            self.root,
            "behaviors/fm.md",
            _registry(_row("TP-FM-01-FAILURES", "`keeps_focus`")),
        )
        _write(
            self.root,
            "src/fm.rs",
            "/// TP-FM-01 defines the complete mapping.\n"
            "// TP-FM-01-FAILURES: keeps focus\nfn keeps_focus() {}\n",
        )
        self._ledger()
        self.assertEqual(check(self.root), [])

    def test_reserved_docs_are_not_parsed_as_registries(self) -> None:
        # README/PROTOCOL/UNDOCUMENTED carry prose tables; parsing them as
        # registries would invent ids that no source marker can satisfy.
        _write(self.root, "behaviors/README.md", _registry(_row("TP-DOC-01", "`nope`")))
        _write(self.root, "behaviors/UPSTREAM-SYNC-PROTOCOL.md", _registry(_row("TP-DOC-02", "`nope`")))
        _write(self.root, "behaviors/fm.md", _registry(_row("TP-FM-01", "`keeps_focus`")))
        _write(self.root, "src/fm.rs", "// TP-FM-01: keeps focus\nfn keeps_focus() {}\n")
        self._ledger()
        self.assertEqual(check(self.root), [])


class RealTreeTests(unittest.TestCase):
    """The synthetic cases above prove the checker works; this one applies it.

    Without this, `just check` would verify the checker and never the tree it
    was written to protect.
    """

    def test_this_repository_registry_is_healthy(self) -> None:
        self.assertEqual(check(Path(__file__).resolve().parents[1]), [])


if __name__ == "__main__":
    unittest.main()
