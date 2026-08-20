"""Product proof for the bar's style presets and grouped islands.

The request this answers was visual — "cpu and mem in one frame, the clock in
another, switchable" — so the answer is read off a screen: one style line
turns a framed strip into floating islands, and three grouped sections paint
as ONE box, not three. Counted by corner glyphs on the bar's own rows, because
a chrome that resolves the slots and a geometry that carries the rects are
both present in a build that paints the wrong number of frames.

Driven against this tree's debug binary (the landing-gate pattern), so the
gate measures the code being landed and needs no delivered-artifact probe.
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

ISLANDS = """onboarding = false

[shell.bars.top]
enabled = true
size = 3
style = "islands"

[[shell.bars.top.sections]]
kind = "content"
min = 8
max = 10
widget = { kind = "label", text = "IDENT" }

[[shell.bars.top.sections]]
kind = "fill"

[[shell.bars.top.sections]]
kind = "content"
min = 8
max = 9
group = "sys"
widget = { kind = "label", text = "CPUPART" }

[[shell.bars.top.sections]]
kind = "content"
min = 8
max = 9
group = "sys"
widget = { kind = "label", text = "MEMPART" }

[[shell.bars.top.sections]]
kind = "content"
min = 8
max = 9
group = "sys"
widget = { kind = "label", text = "SWPPART" }
"""

#: The framed look: the style line gone — and the group lines with it,
#: because `group` is an explicit key and explicit keys outrank the style;
#: a framed bar of three rows leaves one across, which a group's frame is
#: (correctly) refused on. The control is the *old* bar, not a broken one.
FRAMED = ISLANDS.replace('style = "islands"\n', "").replace('group = "sys"\n', "")


@unittest.skipUnless(
    DEBUG_BINARY.exists() and os.access(DEBUG_BINARY, os.X_OK),
    "no debug build to drive",
)
class BarStyleProductProof(unittest.TestCase):
    """The look, as the person meets it."""

    def _corners_on_bar(self, session: HeadfulSession, rows: int) -> int:
        return sum(session.row(y).count("╭") for y in range(rows))

    def test_islands_style_paints_islands_and_one_frame_per_group(self) -> None:
        """H14 — one style line: two islands, one of them holding three parts."""
        with HeadfulSession(ISLANDS, binary=DEBUG_BINARY) as session:
            session.settle(18)
            for label in ("IDENT", "CPUPART", "MEMPART", "SWPPART"):
                self.assertIsNotNone(
                    session.find(label, within_rows=3),
                    f"{label} must be painted on the bar:\n{session.text()[:600]}",
                )
            self.assertEqual(
                self._corners_on_bar(session, 3),
                2,
                "the ident island and ONE shared sys island — two frames, "
                f"not four and not none:\n{session.text()[:600]}",
            )

    def test_removing_the_style_line_is_the_framed_look_again(self) -> None:
        """H15 — the negative control: same bar, one line less, one outer frame."""
        with HeadfulSession(FRAMED, binary=DEBUG_BINARY) as session:
            session.settle(18)
            self.assertIsNotNone(
                session.find("IDENT", within_rows=3),
                "the bar itself must still draw",
            )
            self.assertEqual(
                self._corners_on_bar(session, 3),
                1,
                "without the style line the bar wears its own single panel: "
                f"one corner, not two:\n{session.text()[:600]}",
            )


if __name__ == "__main__":
    unittest.main()
