"""Product proof for the "Open Files beside" verb.

Unit tests prove the state transition; this proves the screen and the pointer.
A real herdr is driven against a throwaway home and config: a right press on
the stage must offer the verb, choosing it must put the Files surface in the
right half beside the terminal, and — the half the report is really about —
the pointer must keep working afterwards, on rows of both kinds and on both
halves.

The file manager opens on the session's working directory, so the fixtures
that have to be VISIBLE are made there, under names unique to this run.

Nothing here touches the live server, the user's config, or the user's home.
"""

from __future__ import annotations

import os
import shutil
import sys
import tempfile
import time
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


class FilesBesideProductProof(unittest.TestCase):
    def setUp(self) -> None:
        if not (DEBUG_BINARY.exists() and os.access(DEBUG_BINARY, os.X_OK)):
            self.skipTest(f"no debug build at {DEBUG_BINARY}")
        self.root = Path(tempfile.mkdtemp(prefix="herdr-beside-root-"))
        self.home = Path(tempfile.mkdtemp(prefix="herdr-beside-home-"))
        # The file manager opens on the session's working directory, so the
        # fixtures that must be VISIBLE live there — in a directory of this
        # test's own, where nothing else can push them off the screen.
        # A SHORT path on purpose: the right half is narrow, and the header
        # spends its width on the directory before it can show a selection.
        self.stage = Path(tempfile.gettempdir()) / f"hb{os.getpid()}"
        self.stage.mkdir(parents=True, exist_ok=True)
        self.child_name = "kanit-cocuk"
        (self.stage / "bir-dizin" / self.child_name).mkdir(parents=True, exist_ok=True)
        (self.stage / "bir-dosya.txt").write_text("HELLOMARKER\n", encoding="utf-8")
        self._real_home = os.environ["HOME"]
        os.environ["HOME"] = str(self.home)

    def tearDown(self) -> None:
        os.environ["HOME"] = self._real_home
        shutil.rmtree(self.stage, ignore_errors=True)

    @staticmethod
    def _header(session: HeadfulSession) -> str:
        for line in session.text().splitlines():
            if "[copy]" in line:
                return line
        return ""

    def test_the_verb_is_offered_lands_and_leaves_the_pointer_alive(self) -> None:
        with HeadfulSession(
            CONFIG,
            binary=DEBUG_BINARY,
            root=self.root,
            cwd=self.stage,
            # A WIDE screen on purpose: the beside half is half a stage, and
            # the header has to fit a directory AND its selection before this
            # proof can read one. At 120 columns it elides both and the
            # assertion measures the width instead of the press.
            cols=200,
            rows=44,
        ) as session:
            session.settle(12.0)
            before = session.text()
            self.assertNotIn(VERB, before, "the menu must not already be open")

            # T1 — the verb is offered where a person would look for it:
            # a secondary press on the stage, right of the sidebar.
            session.right_click(10, 60, settle=6.0)
            menu = session.text()
            self.assertIn(
                VERB,
                menu,
                f"a right press on the stage must offer the verb (screen was:\n{menu})",
            )

            # T2 — choosing it puts the Files surface in the right half. The
            # proof is the surface's own chrome, not any particular directory.
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
                session.find("Files"), "the right half names the surface it hosts"
            )

            # T3 — a press on a FILE row selects it. The header carries the
            # selection, so it is the readable proof that the press landed.
            # T3 — a press on a FILE row selects it AND fills the detail
            # panel. The panel is the readable proof, and it proves two things
            # at once: the press landed, and the preview worker scheduled for
            # the beside half actually delivered (TP-SBS-FILES-03). The header
            # elides its text in a half this wide, so it cannot be the oracle.
            self.assertNotIn(
                "HELLOMARKER",
                session.text(),
                "nothing is previewed before anything is selected",
            )
            file_at = session.find("bir-dosya.txt")
            self.assertIsNotNone(
                file_at,
                f"the fixture file must be a visible row:\n{session.text()}",
            )
            session.click(file_at[0], file_at[1] + 2, settle=6.0)
            self.assertIn(
                "HELLOMARKER",
                session.text(),
                "a press on a file row must select it and preview its content:"
                f"\n{session.text()}",
            )

            # T5 — a press on a DIRECTORY row must open its child column. This
            # is the one proof that ASYNCHRONOUS work queued by the beside
            # half actually comes home: the listing is read on a worker, and a
            # result gate that asks who owns the STAGE throws it away
            # (TP-SBS-FILES-03).
            dir_at = session.find("bir-dizin")
            self.assertIsNotNone(
                dir_at,
                f"the fixture directory must be a visible row:\n{session.text()}",
            )
            session.click(dir_at[0], dir_at[1] + 2, settle=6.0)
            self.assertIn(
                self.child_name,
                session.text(),
                "pressing a directory must open its child column in the beside "
                f"half:\n{session.text()}",
            )

            # T4 — and the LEFT half still answers: pressing the terminal must
            # not tear the split down.
            session.click(10, 40, settle=5.0)
            self.assertIsNotNone(
                session.find_regex(r"\[copy\].*\[paste\]"),
                "a press on the terminal half must leave the split standing",
            )


    def test_depth_descent_stays_visible_and_the_wheel_pans_back(self) -> None:
        """TP-TRAIL-REVEAL-01 product proof — the user's live report, verbatim:
        "2. 3. dereceden klasor iclerine girmedeim ... millers columlarda
        gozukmuyorlar". Every press on a directory must produce a VISIBLE
        column, however deep the chain already is; and the horizontal wheel
        must pan back to the columns the reveal slid away."""
        depth_stage = Path(tempfile.gettempdir()) / f"hbd{os.getpid()}"
        shutil.rmtree(depth_stage, ignore_errors=True)
        deep = depth_stage / "kat-bir" / "kat-iki" / "kat-uc"
        deep.mkdir(parents=True)
        (deep / "derin-kanit.txt").write_text("DEEPMARKER\n", encoding="utf-8")
        (depth_stage / "kok-kanit.txt").write_text("ROOT\n", encoding="utf-8")
        try:
            with HeadfulSession(
                CONFIG,
                binary=DEBUG_BINARY,
                root=self.root,
                cwd=depth_stage,
                cols=200,
                rows=44,
            ) as session:
                session.settle(12.0)
                session.right_click(10, 60, settle=6.0)
                where = session.find(VERB)
                self.assertIsNotNone(where, "the verb's row must be locatable")
                session.click(where[0], where[1] + 2, settle=8.0)

                # Level 1 — the chain still fits, so this press worked before
                # the reveal too. It anchors the regression.
                at = session.find("kat-bir")
                self.assertIsNotNone(at, f"root listing must show kat-bir:\n{session.text()}")
                session.click(at[0], at[1] + 2, settle=6.0)
                self.assertIsNotNone(
                    session.find("kat-iki"),
                    f"pressing kat-bir must show its listing:\n{session.text()}",
                )

                # Level 2 — the chain has outgrown the beside half. Without the
                # reveal the freshly opened column right of the parent is
                # clipped off the viewport forever: the user's dead press.
                at = session.find("kat-iki")
                session.click(at[0], at[1] + 2, settle=6.0)
                self.assertIsNotNone(
                    session.find("kat-uc"),
                    "pressing a 2nd-level directory must keep its listing "
                    f"on screen (TP-TRAIL-REVEAL-01):\n{session.text()}",
                )

                # Level 3 — depth generalizes.
                at = session.find("kat-uc")
                session.click(at[0], at[1] + 2, settle=6.0)
                self.assertIsNotNone(
                    session.find("derin-kanit"),
                    f"pressing a 3rd-level directory must show its file:\n{session.text()}",
                )

                # The wheel road back: the reveal slid the root column away;
                # wheel-left over the trail must bring it back into view.
                anchor = session.find("derin-kanit") or session.find("kat-uc")
                self.assertIsNotNone(anchor, "an anchor row inside the trail")
                row, col = anchor
                for _ in range(24):
                    session.send(f"\x1b[<66;{col + 1};{row + 1}M".encode())
                    time.sleep(0.08)
                session.settle(3.0)
                self.assertIsNotNone(
                    session.find("kok-kanit"),
                    "wheel-left must pan the root column back into the "
                    f"viewport:\n{session.text()}",
                )
        finally:
            shutil.rmtree(depth_stage, ignore_errors=True)


if __name__ == "__main__":
    unittest.main()
