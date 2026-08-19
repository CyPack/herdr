"""Every python test module in `scripts/` is one `just check` actually runs.

`just check` names its python modules by hand, one after another on a single
line. A test file added without editing that line is a test nobody runs: it is
green because it never executed, and nothing anywhere says so. The failure is
silent in the direction that matters — a gate that stops guarding does not
report anything, it simply stops failing.

Measured 2026-08-18: fourteen modules were listed, `unittest discover` was not
used, and the repository had no check that the two agreed. This is that check,
and it is written to fail on the *addition* side because that is the direction
somebody takes when they are busy — writing the test is the work, remembering
the recipe is the afterthought.
"""


from __future__ import annotations

import re
import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
JUSTFILE = REPO_ROOT / "justfile"
SCRIPTS = REPO_ROOT / "scripts"


def _modules_named_by_just_check() -> set[str]:
    """The `scripts.test_*` modules the `check` recipe runs."""
    text = JUSTFILE.read_text()
    # The recipe body is indented under `check:`; take it up to the next
    # unindented line so a later recipe naming a module cannot count for this one.
    start = text.index("\ncheck:")
    rest = text[start + 1 :]
    end = len(rest)
    for match in re.finditer(r"\n(?=\S)", rest):
        if match.start() > 0:
            end = match.start()
            break
    body = rest[:end]
    return set(re.findall(r"scripts\.(test_[a-z0-9_]+)", body))


def _test_modules_on_disk() -> set[str]:
    return {path.stem for path in SCRIPTS.glob("test_*.py")}


class PythonSuiteCompletenessTests(unittest.TestCase):
    def test_every_test_module_on_disk_is_run_by_just_check(self) -> None:
        on_disk = _test_modules_on_disk()
        named = _modules_named_by_just_check()

        # Guard the harvest itself: a renamed recipe or a changed prefix would
        # leave both sides empty and this test green over nothing.
        self.assertGreaterEqual(
            len(named),
            10,
            "the check recipe named almost no python modules; this test is "
            "reading the wrong place in the justfile",
        )

        missing = sorted(on_disk - named)
        self.assertEqual(
            missing,
            [],
            "these test modules exist but `just check` never runs them, so they "
            f"are green without being executed: {missing}",
        )

    def test_just_check_names_no_module_that_is_missing_from_disk(self) -> None:
        """The other direction: a deleted or renamed module fails the recipe at
        run time with an import error, which is loud — but it fails *late*, after
        the whole Rust suite has run. Catching it here costs four minutes less."""
        named = _modules_named_by_just_check()
        on_disk = _test_modules_on_disk()
        phantom = sorted(named - on_disk)
        self.assertEqual(
            phantom,
            [],
            f"`just check` names modules that do not exist: {phantom}",
        )


if __name__ == "__main__":
    unittest.main()
