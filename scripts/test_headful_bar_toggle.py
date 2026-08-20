"""Product proof for the bar switch: the keybind and the hide button.

Unit tests prove the filter, the dispatchers and the click resolution; this
proves the screen. It drives a real herdr, reads the rows a bar occupies,
presses the key and the button, and watches the content underneath move up —
because the discriminating measurement is the *row shift*, not the label: a
bar that stopped painting but kept its rows would leave a blank strip that a
text search cannot tell from success (the lesson TP-CHROME-130's own product
proof was corrected by).

Driven against this tree's debug binary rather than the installed one, so the
gate measures the code being landed and needs no delivered-artifact probe.
Nothing here touches the live server, the user's config, or the user's home.
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

#: prefix defaults to ctrl+b; the switch is prefix+shift+b.
PREFIX = b"\x02"
TOGGLE_BARS = b"B"

CONFIG = """onboarding = false

[shell.bars.top]
enabled = true
size = 3
border = true

[[shell.bars.top.sections]]
kind = "content"
min = 12
max = 16
widget = { kind = "label", text = "SWITCHBAR" }

[[shell.bars.top.sections]]
kind = "fill"

[[shell.bars.top.sections]]
kind = "content"
min = 6
max = 8
border = false
widget = { kind = "label", text = "HIDEME" }
action = { kind = "hide" }
"""


@unittest.skipUnless(
    DEBUG_BINARY.exists() and os.access(DEBUG_BINARY, os.X_OK),
    "no debug build to drive",
)
class BarSwitchProductProof(unittest.TestCase):
    """The switch as the person meets it."""

    def test_the_key_takes_the_bar_off_the_screen_and_back(self) -> None:
        """H11 — press, gone, rows shift up; press again, back."""
        with HeadfulSession(CONFIG, binary=DEBUG_BINARY) as session:
            session.settle(18)
            label = session.find("SWITCHBAR", within_rows=4)
            self.assertIsNotNone(
                label,
                f"the bar never painted, so the switch has nothing to prove:"
                f"\n{session.text()[:600]}",
            )
            # The row the sidebar's first heading sits on, before the switch.
            anchor_before = session.find("Spaces")
            self.assertIsNotNone(anchor_before, "the sidebar anchors the shift")

            session.send(PREFIX)
            session.send(TOGGLE_BARS)
            session.settle(6)

            self.assertIsNone(
                session.find("SWITCHBAR", within_rows=6),
                f"the bar must leave the screen:\n{session.text()[:600]}",
            )
            anchor_after = session.find("Spaces")
            self.assertIsNotNone(anchor_after)
            self.assertLess(
                anchor_after[0],
                anchor_before[0],
                "the content underneath must move up — a blank strip where the "
                "bar was is a bar that stopped painting, not a bar that left",
            )

            session.send(PREFIX)
            session.send(TOGGLE_BARS)
            session.settle(6)

            self.assertIsNotNone(
                session.find("SWITCHBAR", within_rows=4),
                f"the second press must bring the bar back:\n{session.text()[:600]}",
            )
            restored = session.find("Spaces")
            self.assertEqual(
                restored[0],
                anchor_before[0],
                "and the content settles back exactly where it was",
            )

    def test_the_hide_button_takes_its_own_bar_off(self) -> None:
        """H12 — a click on the hide section quiets the bar it sits in.

        The button is re-located immediately before the click rather than
        trusted from an earlier read: quieting chrome moves everything under
        it, and a stale coordinate presses a cell that is no longer a button —
        the exact mistake this suite's focus-toggle proof shipped with once.
        """
        with HeadfulSession(CONFIG, binary=DEBUG_BINARY) as session:
            session.settle(18)
            button = session.find("HIDEME", within_rows=4)
            self.assertIsNotNone(
                button,
                f"the hide section never painted:\n{session.text()[:600]}",
            )

            row, col = button
            session.click(row, col + 1)
            session.settle(6)

            self.assertIsNone(
                session.find("SWITCHBAR", within_rows=6),
                f"a press on the hide section must switch its bar off:"
                f"\n{session.text()[:600]}",
            )

    def test_without_the_action_the_same_click_leaves_the_bar_standing(self) -> None:
        """H13 — the negative control.

        Without it H12 proves nothing: a bar that vanishes on *any* click —
        or a session that lost its bar for an unrelated reason — would pass.
        The same screen position, the same gesture, an action-free section:
        the bar must stay.
        """
        bare = CONFIG.replace('action = { kind = "hide" }\n', "")
        with HeadfulSession(bare, binary=DEBUG_BINARY) as session:
            session.settle(18)
            button = session.find("HIDEME", within_rows=4)
            self.assertIsNotNone(button)

            row, col = button
            session.click(row, col + 1)
            session.settle(6)

            self.assertIsNotNone(
                session.find("SWITCHBAR", within_rows=4),
                "a section with no action swallows the click and changes nothing",
            )


if __name__ == "__main__":
    unittest.main()
