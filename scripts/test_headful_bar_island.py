"""Headful test: a bar section wearing its own frame, read off the screen.

The question a person asked was visual — "put cpu/mem in one frame and the
clock in another, like the islands in OpenBar" — so the answer has to be read
off a screen. The Rust suite proves the rectangles, the tones and the widget
placement; none of that distinguishes a build that computes an island from one
that computes it and never paints it, and this fork has shipped exactly that
shape before.

Skipped, loudly, when there is no installed binary: a machine that cannot run
the product cannot answer this, and pretending otherwise would report a pass
nothing measured.
"""

from __future__ import annotations

import json
import subprocess
import unittest

from .headful_harness import INSTALLED_BINARY, HeadfulSession, binary_available


def installed_build_publishes_section_keys() -> bool:
    """Whether the installed binary knows a section can carry its own keys.

    A product proof measures a delivered artefact, and an artefact older than
    the feature is not a failing feature — it is an absent artefact, exactly
    like a missing binary. Without this the gate would go red between a landing
    and its delivery, on every machine, for a reason that has nothing to do
    with the code.

    Asked of `shell spec` rather than guessed from a version string: the spec
    is this build's own statement of what it accepts, which is precisely the
    question here, and a version string goes stale while a grammar cannot.
    """
    if not binary_available():
        return False
    try:
        printed = subprocess.run(
            [str(INSTALLED_BINARY), "shell", "spec", "--json"],
            capture_output=True,
            text=True,
            timeout=30,
            check=False,
        )
    except (OSError, subprocess.SubprocessError):
        return False
    if printed.returncode != 0:
        return False
    try:
        return "section_keys" in json.loads(printed.stdout)
    except json.JSONDecodeError:
        return False


#: Loud on purpose. A skip that reads like "nothing to do here" is how a proof
#: stops being a gate; this one names the remedy, so a permanently skipped run
#: reads as an undelivered landing rather than as a pass.
NOT_DELIVERED = (
    "the installed herdr predates section frames (its `shell spec` publishes no "
    "`section_keys`); land and deliver, then this proof measures again"
)

#: `size = 5` because both frames are drawn: the bar spends two rows on its own
#: border and an island needs the three that leaves. A bordered bar of three —
#: the documented minimum for a bar — leaves one row and is refused, which is
#: the arithmetic TP-CHROME-135 exists to say out loud.
BAR_TEMPLATE = """onboarding = false

[shell.bars.top]
enabled = true
size = 5
border = true
color = "mauve"

[[shell.bars.top.sections]]
kind = "content"
min = 8
max = 12
border = {island}
color = "teal"
widget = {{ kind = "label", text = "ISLANDCPU" }}

[[shell.bars.top.sections]]
kind = "fill"
weight = 1

[[shell.bars.top.sections]]
kind = "content"
min = 9
max = 12
border = {island}
widget = {{ kind = "label", text = "ISLANDCLK" }}
"""

WITH_ISLANDS = BAR_TEMPLATE.format(island="true")
WITHOUT_ISLANDS = BAR_TEMPLATE.format(island="false")

#: The bar occupies the first five rows; row 0 is its own top border. An
#: island's top-left corner therefore lands on row 1, which is the row this
#: test counts. Counting the whole screen would fold in the sidebar's and the
#: tab strip's own corners and pass on a bar with no island in it at all.
ISLAND_CORNER_ROW = 1
CORNER = "╭"


@unittest.skipUnless(binary_available(), "no installed herdr to drive")
@unittest.skipUnless(installed_build_publishes_section_keys(), NOT_DELIVERED)
class HeadfulBarIslandTests(unittest.TestCase):
    """Islands as the person meets them."""

    def _corner_count(self, session: HeadfulSession) -> int:
        return session.row(ISLAND_CORNER_ROW).count(CORNER)

    def test_a_section_that_asks_for_a_frame_is_drawn_inside_one(self) -> None:
        """H9 — two islands are painted inside the bar, and hold their labels.

        Both halves are asserted because either alone is satisfiable by a
        broken build: corners with no labels is a pair of empty boxes, and
        labels with no corners is the feature not existing.
        """
        with HeadfulSession(WITH_ISLANDS) as session:
            session.settle(18)
            screen = session.text()[:900]

            self.assertIsNotNone(
                session.find("ISLANDCPU", within_rows=5),
                f"the first island never painted its label:\n{screen}",
            )
            self.assertIsNotNone(
                session.find("ISLANDCLK", within_rows=5),
                f"the second island never painted its label:\n{screen}",
            )
            self.assertEqual(
                self._corner_count(session),
                2,
                "two sections asked for frames, so the bar's first inner row "
                f"carries two rounded corners:\n{screen}",
            )

            # The label belongs inside the frame, not on the row the frame is
            # drawn on. A build that painted the box over its own content would
            # satisfy every count above.
            corner_row = session.row(ISLAND_CORNER_ROW)
            self.assertNotIn(
                "ISLANDCPU",
                corner_row,
                f"the frame row is the frame's, not the label's: {corner_row!r}",
            )
            label = session.find("ISLANDCPU", within_rows=5)
            assert label is not None
            self.assertEqual(
                label[0],
                ISLAND_CORNER_ROW + 1,
                "an island's label sits one row below its frame, "
                f"not on row {label[0]}",
            )

    def test_the_same_bar_without_frames_draws_none(self) -> None:
        """H10 — the negative control.

        Without it, H9 proves nothing: a bar that happened to draw rounded
        corners for some other reason would satisfy it, and so would a build
        that ignores `border` while the bar's own frame supplies the glyphs.
        The labels are asserted here too, so a failure to draw the bar at all
        cannot masquerade as a correct absence of islands.
        """
        with HeadfulSession(WITHOUT_ISLANDS) as session:
            session.settle(18)
            screen = session.text()[:900]

            self.assertIsNotNone(
                session.find("ISLANDCPU", within_rows=5),
                f"the bar itself never drew, so its lack of islands proves "
                f"nothing:\n{screen}",
            )
            self.assertEqual(
                self._corner_count(session),
                0,
                "no section asked for a frame, so the bar's inner row carries "
                f"no rounded corners:\n{screen}",
            )


if __name__ == "__main__":
    unittest.main()
