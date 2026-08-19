"""Product proof for `hide_when_focused`.

The unit tests prove the projection: a track that opted in returns NONE while
focus is on. They cannot prove the screen, and on this surface the screen is
the whole point — a bar that is filtered in the model but still painted is
exactly the class of defect this repository keeps finding.

Two runs, one difference: the focus switch in the config.
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

#: A label nothing else on the screen can produce.
PROBE = "FOCUSPROBE"


def _config(*, focus_only: bool) -> str:
    return f"""
[ui.sidebar.spaces]
focus_only = {str(focus_only).lower()}

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


class QuietedBarProductProof(unittest.TestCase):
    def setUp(self) -> None:
        if not (DEBUG_BINARY.exists() and os.access(DEBUG_BINARY, os.X_OK)):
            self.skipTest(f"no debug build at {DEBUG_BINARY}")

    def _screen(self, *, focus_only: bool) -> str:
        with HeadfulSession(_config(focus_only=focus_only), binary=DEBUG_BINARY) as session:
            session.settle(12.0)
            return session.text()

    def test_the_bar_is_painted_outside_focus_and_absent_inside_it(self) -> None:
        # Positive control: the bar reaches the screen at all. Without this the
        # negative below would pass on a build that never draws bars.
        outside = self._screen(focus_only=False)
        self.assertIn(
            PROBE,
            outside,
            "the bar has to be on screen before its absence can mean anything",
        )

        # The measurement: the same config, focus on.
        inside = self._screen(focus_only=True)
        self.assertNotIn(
            PROBE,
            inside,
            "an edge that opted into hide_when_focused must not be painted in focus mode",
        )


if __name__ == "__main__":
    unittest.main()
