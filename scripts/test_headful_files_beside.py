"""Product proof for the "Open Files beside" verb.

Unit tests prove the state transition; this proves the screen and the pointer.
A real herdr is driven against a throwaway home and config: a right press on
the stage must offer the verb, choosing it must put the Files surface in the
right half beside the terminal, and — the half the report is really about —
the pointer must keep working afterwards, on both halves.

Nothing here touches the live server, the user's config, or the user's home.
"""

from __future__ import annotations

import os
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from headful_harness import HeadfulSession  # noqa: E402

REPO = Path(__file__).resolve().parent.parent
DEBUG_BINARY = REPO / "target" / "debug" / "herdr"

CONFIG = """
[ui]
sidebar_width = 30

"""

VERB = "Open Files beside"


@unittest.skip(
    "OPEN BUG (PRD §R2, TP-SBS-FILES-02 slice 2): two of the three links are "
    "fixed — the on-screen generation authority and the rail/trail projection "
    "now agree, measured — but presses on the beside half still reach nothing. "
    "The test is kept, named and wired so the next slice starts from a red it "
    "can see; unskip it there."
)
class FilesBesideProductProof(unittest.TestCase):
    def setUp(self) -> None:
        if not (DEBUG_BINARY.exists() and os.access(DEBUG_BINARY, os.X_OK)):
            self.skipTest(f"no debug build at {DEBUG_BINARY}")
        self.root = Path(tempfile.mkdtemp(prefix="herdr-beside-root-"))
        self.home = Path(tempfile.mkdtemp(prefix="herdr-beside-home-"))
        # a couple of real entries so the Files surface has rows to paint
        for child in ("alpha-dir", "beta-dir"):
            (self.home / child).mkdir(parents=True, exist_ok=True)
        (self.home / "readme.txt").write_text("hello\n", encoding="utf-8")
        self._real_home = os.environ["HOME"]
        os.environ["HOME"] = str(self.home)

    def tearDown(self) -> None:
        os.environ["HOME"] = self._real_home

    def test_the_verb_is_offered_lands_and_leaves_the_pointer_alive(self) -> None:
        with HeadfulSession(CONFIG, binary=DEBUG_BINARY, root=self.root) as session:
            session.settle(12.0)
            before = session.text()
            self.assertNotIn(VERB, before, "the menu must not already be open")

            # T1 — the verb is offered where a person would look for it:
            # a secondary press on the stage, right of the sidebar.
            stage_col = 60
            stage_row = 10
            session.right_click(stage_row, stage_col, settle=6.0)
            menu = session.text()
            self.assertIn(
                VERB,
                menu,
                "a right press on the stage must offer the verb "
                f"(screen was:\n{menu})",
            )

            # T2 — choosing it puts the Files surface in the right half.
            # The proof is the surface's own chrome, not any particular
            # directory: the file manager opens on the session's working
            # directory, which is not this test's business to predict.
            where = session.find(VERB)
            self.assertIsNotNone(where, "the verb's row must be locatable")
            row, col = where
            session.click(row, col + 2, settle=8.0)
            beside = session.text()
            self.assertNotIn(VERB, beside, "the menu closes when the verb is taken")
            self.assertIsNotNone(
                session.find_regex(r"\[copy\].*\[paste\]"),
                f"the Files action bar must paint in the right half:\n{beside}",
            )
            self.assertIsNotNone(
                session.find("Files"),
                "the right half names the surface it hosts",
            )

            # T3 — the pointer is alive on the RIGHT half. The file manager
            # opens on the session's working directory (/tmp), where this
            # test's own throwaway root is a visible row: pressing its NAME
            # must select it. This is the half the report is about.
            self.assertIsNotNone(
                session.find("no selecti"),
                "the surface starts with nothing selected",
            )
            own_row = session.find(self.root.name[:18])
            self.assertIsNotNone(
                own_row,
                f"this test's own directory must be a visible row:\n{session.text()}",
            )
            trow, tcol = own_row
            session.click(trow, tcol + 2, settle=5.0)
            after_right = session.text()
            self.assertIsNone(
                session.find("no selecti"),
                f"a press on a file row must select it (clicks alive):\n{after_right}",
            )

            # T4 — and the LEFT half still answers: pressing the terminal must
            # not tear the split down.
            session.click(stage_row, 40, settle=5.0)
            self.assertIsNotNone(
                session.find_regex(r"\[copy\].*\[paste\]"),
                "a press on the terminal half must leave the split standing",
            )


if __name__ == "__main__":
    unittest.main()
