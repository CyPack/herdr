"""Headful tests: the bar, read off the screen and pressed with a real click.

These are the checks a unit test cannot make. Everything below drives the
installed binary and asserts on painted cells, because three defects in August
2026 were green in the Rust suite and wrong on screen: a section that parsed and
drew nothing, a popup sized so the tool inside it refused to start, and a reload
that answered "applied" while changing nothing.

Skipped, loudly, when there is no installed binary — a machine that cannot run
the product cannot answer these questions, and pretending otherwise would make
the suite report a pass it never measured.
"""


from __future__ import annotations

import subprocess
import unittest
from pathlib import Path

from .headful_harness import HeadfulSession, binary_available

BAR_WITHOUT_ACTION = """onboarding = false

[shell.bars.top]
enabled = true
size = 3
border = true

[[shell.bars.top.sections]]
kind = "content"
min = 6
max = 16
widget = { kind = "label", text = "HEADFUL" }

[[shell.bars.top.sections]]
kind = "fill"
weight = 1

[[shell.bars.top.sections]]
kind = "content"
min = 9
max = 10
widget = { kind = "resource", metric = "cpu" }
"""

#: `sleep` rather than `true`: a popup running a command that exits immediately
#: opens and closes faster than the screen is read, so the assertion measures an
#: empty frame and calls the click broken. Measured 2026-08-18 — the first draft
#: of this fixture used `true` and failed for exactly that reason, which is also
#: what a person sees when they wire a bar section to a command that returns at
#: once: a flash, and nothing to look at.
POPUP_ACTION = (
    'action = { kind = "popup", argv = ["sleep", "30"], width = "60%", height = "60%" }\n'
)
BAR_WITH_ACTION = BAR_WITHOUT_ACTION + POPUP_ACTION


@unittest.skipUnless(binary_available(), "no installed herdr to drive")
class HeadfulBarTests(unittest.TestCase):
    """The bar as the person meets it."""

    def test_a_configured_bar_reaches_the_screen(self) -> None:
        """H1 — a bar written in config is painted, with its section content.

        The label is asserted rather than the frame: a bordered bar draws its
        border out of the same config, so a frame around an empty strip would
        pass a check that only looked for the box. That empty-but-framed shape
        is exactly what an unbounded `content` section produced before
        TP-CHROME-124.
        """
        with HeadfulSession(BAR_WITHOUT_ACTION) as session:
            session.settle(18)
            self.assertIsNotNone(
                session.find("HEADFUL", within_rows=5),
                f"the configured bar never painted its label:\n{session.text()[:600]}",
            )
            self.assertIsNotNone(
                session.find_regex(r"CPU\s+\d", within_rows=5),
                "the resource section painted no reading",
            )

    def test_a_bar_section_action_opens_a_popup(self) -> None:
        """H2 — pressing a section with an action opens a popup.

        `config check` accepting an action says the table is well formed; it says
        nothing about whether a press reaches it. The two questions were confused
        for three rounds in August 2026, so this asserts the second one.
        """
        with HeadfulSession(BAR_WITH_ACTION) as session:
            session.settle(18)
            found = session.find_regex(r"CPU", within_rows=5)
            self.assertIsNotNone(found, "no section to press")
            row, col = found
            session.click(row, col + 2)
            self.assertIn(
                "popup",
                session.text().lower(),
                f"pressing a section with an action drew no popup:\n{session.text()[:900]}",
            )

    def test_a_reload_carries_a_new_action_to_a_running_client(self) -> None:
        """TP-CHROME-127 at the surface the person actually touches.

        The Rust test proves the chrome learns the action. This proves the press
        lands, which is the claim that was wrong: the client kept answering to
        the table it started with while the reload reported success.
        """
        with HeadfulSession(BAR_WITHOUT_ACTION) as session:
            session.settle(18)
            found = session.find_regex(r"CPU", within_rows=5)
            self.assertIsNotNone(found, "no section to press")
            row, col = found

            session.click(row, col + 2)
            self.assertNotIn(
                "popup",
                session.text().lower(),
                "a section with no action must not open anything",
            )

            session.write_config(BAR_WITH_ACTION)
            reloaded = session.reload_config_via_cli()
            self.assertEqual(reloaded.returncode, 0, reloaded.stderr)
            session.settle(5)

            session.click(row, col + 2)
            self.assertIn(
                "popup",
                session.text().lower(),
                "an action added by reload never reached the press path:\n"
                + session.text()[:900],
            )

    def test_the_harness_leaves_the_live_environment_alone(self) -> None:
        """H3 — the person's own config is untouched, byte for byte.

        Asserted rather than assumed: every earlier one-off script claimed
        isolation in a comment, and a comment cannot fail.
        """
        live = Path.home() / ".config" / "herdr" / "config.toml"
        if not live.exists():
            self.skipTest("no live config on this machine")
        before = live.read_bytes()
        with HeadfulSession(BAR_WITH_ACTION) as session:
            session.settle(12)
        self.assertEqual(
            before, live.read_bytes(), "a headful test wrote into the live config"
        )

    def test_the_harness_stops_the_server_it_started(self) -> None:
        """H4 — no server outlives the session.

        A leaked server keeps its socket and answers the next test, which then
        measures a herdr nobody configured.
        """
        session = HeadfulSession(BAR_WITHOUT_ACTION)
        session.start()
        session.settle(12)
        socket_root = session._root / "state"  # noqa: SLF001 - the harness under test
        session.stop()
        listeners = subprocess.run(
            ["ss", "-xl"], capture_output=True, text=True, check=False
        ).stdout
        self.assertNotIn(
            str(socket_root),
            listeners,
            "a socket from a stopped headful session is still listening",
        )


if __name__ == "__main__":
    unittest.main()
