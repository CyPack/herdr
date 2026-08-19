"""Product proof for the live focus toggle.

`test_headful_bar_focus.py` proves that a quieted bar is absent from a session
that *started* in focus mode. That is the easier half, and it leaves a real gap
open: it never exercises the path where focus changes while a client is running.

A mutation proved the gap is real. Feeding the geometry key the unfiltered bars
leaves all 916 unit tests green — every projection is correct — while a running
client keeps serving the composition it already had. Nothing on screen moves.

So this drives one session and toggles focus inside it.
"""

from __future__ import annotations

import os
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from headful_harness import HeadfulSession  # noqa: E402

REPO = Path(__file__).resolve().parent.parent
DEBUG_BINARY = REPO / "target" / "debug" / "herdr"

#: A label nothing else on the screen produces.
PROBE = "TOGGLEPROBE"

CONFIG = f"""
[ui.sidebar.spaces]
focus_only = false

[shell.bars.top]
enabled = true
size = 1
border = false
hide_when_focused = true

[[shell.bars.top.sections]]
kind = "content"
min = 4
widget = {{ kind = "label", text = "{PROBE}" }}
"""


class LiveFocusToggleProductProof(unittest.TestCase):
    def setUp(self) -> None:
        if not (DEBUG_BINARY.exists() and os.access(DEBUG_BINARY, os.X_OK)):
            self.skipTest(f"no debug build at {DEBUG_BINARY}")

    def test_toggling_focus_moves_the_bar_off_and_back_on_screen(self) -> None:
        with HeadfulSession(CONFIG, binary=DEBUG_BINARY) as session:
            session.settle(12.0)

            # Positive control. Without it every assertion below could pass on a
            # build that never draws a bar at all.
            self.assertIn(
                PROBE,
                session.text(),
                "the bar has to be on screen before its absence can mean anything",
            )

            # The footer's focus switch, located before *each* click rather than
            # once. Quieting the bar gives its row back to everything below it,
            # so the switch itself moves up a line — measured: (20, 11) before,
            # (19, 11) after. A coordinate captured once would send the second
            # click into whatever moved into that cell, and the test would read
            # a working product as broken.
            def focus_switch_row() -> int:
                hit = session.find("focus")
                self.assertIsNotNone(
                    hit, f"the Spaces footer draws no focus switch:\n{session.text()}"
                )
                return hit[0]

            def click_the_focus_switch() -> None:
                hit = session.find("focus")
                self.assertIsNotNone(
                    hit, f"the Spaces footer draws no focus switch:\n{session.text()}"
                )
                session.click(*hit)
                session.settle(5.0)

            row_with_bar = focus_switch_row()
            click_the_focus_switch()
            self.assertNotIn(
                PROBE,
                session.text(),
                "turning focus on has to take the quieted bar off the screen of a RUNNING client — "
                "this is the half a passing unit suite cannot feel",
            )

            # The discriminating measurement, and the reason the label alone is
            # not enough: a quieted bar has to give its row back, so everything
            # below moves up. Painting nothing while still reserving the row
            # leaves a blank strip — invisible to a text search, obvious on
            # screen, and exactly what happens when the geometry key stops
            # seeing the filtered value and serves the composition it had.
            row_without_bar = focus_switch_row()
            self.assertLess(
                row_without_bar,
                row_with_bar,
                "a quieted bar must return its row to the layout, not merely stop painting: "
                f"the footer stayed on row {row_without_bar}",
            )

            click_the_focus_switch()
            self.assertIn(
                PROBE,
                session.text(),
                "turning focus back off has to bring it back; a one-way hide is a loss, not a filter",
            )
            self.assertEqual(
                focus_switch_row(),
                row_with_bar,
                "and the row it took back has to be given up again",
            )


if __name__ == "__main__":
    unittest.main()
