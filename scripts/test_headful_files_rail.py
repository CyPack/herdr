"""Product proof for the Files locations rail.

Unit tests prove the projection; this proves the screen. It builds a synthetic
host — an isolated home carrying a localized user-directory list, a desktop
bookmark file, and real directories on disk — drives a real herdr against it,
opens the file manager, and reads the rows the rail actually paints.

Nothing here touches the live server, the user's config, or the user's home:
both the home and the config root are throwaway directories.
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
sidebar_width = 34
"""

#: `prefix+f` toggles the native file manager; prefix defaults to ctrl+b.
PREFIX = b"\x02"
TOGGLE_FILE_MANAGER = b"f"


def _make_host(root: Path, home: Path) -> None:
    """A desktop that keeps its directories in Turkish and curates bookmarks."""
    for child in ("İndirilenler", "Belgeler", "arsiv-deposu"):
        (home / child).mkdir(parents=True, exist_ok=True)

    config = root / "config"
    (config / "gtk-3.0").mkdir(parents=True, exist_ok=True)
    (config / "gtk-3.0" / "bookmarks").write_text(
        f"file://{home}/arsiv-deposu\n"
        f"file://{home}/Belgeler saha-belgeleri\n",
        encoding="utf-8",
    )
    (config / "user-dirs.dirs").write_text(
        '# written by a localized desktop\n'
        'XDG_DOWNLOAD_DIR="$HOME/İndirilenler"\n'
        'XDG_DOCUMENTS_DIR="$HOME/Belgeler"\n',
        encoding="utf-8",
    )


class FilesRailProductProof(unittest.TestCase):
    """What the rail paints, read off a real screen."""

    def setUp(self) -> None:
        if not (DEBUG_BINARY.exists() and os.access(DEBUG_BINARY, os.X_OK)):
            self.skipTest(f"no debug build at {DEBUG_BINARY}")
        self.root = Path(tempfile.mkdtemp(prefix="herdr-rail-root-"))
        self.home = Path(tempfile.mkdtemp(prefix="herdr-rail-home-"))
        _make_host(self.root, self.home)
        self._real_home = os.environ["HOME"]
        os.environ["HOME"] = str(self.home)

    def tearDown(self) -> None:
        os.environ["HOME"] = self._real_home

    def _rail_text(self) -> str:
        with HeadfulSession(CONFIG, binary=DEBUG_BINARY, root=self.root) as session:
            session.settle(12.0)
            session.send(PREFIX)
            session.send(TOGGLE_FILE_MANAGER)
            session.settle(8.0)
            return session.text()

    def test_the_rail_shows_the_places_this_host_actually_keeps(self) -> None:
        screen = self._rail_text()

        # The rail is on screen at all: without this a blank screen would pass
        # every "must not appear" assertion below.
        for heading in ("FAVORITES", "BOOKMARKS", "LOCATIONS"):
            self.assertIn(heading, screen, f"the rail must paint its {heading} section")

        # Positive: the host's own names, not the English ones it does not use.
        self.assertIn(
            "İndirilenler",
            screen,
            "the localized download directory must reach the screen",
        )
        self.assertIn(
            "arsiv-deposu",
            screen,
            "a curated bookmark must reach the screen",
        )
        # Negative: this host has no `Downloads` directory, so no row may claim one.
        self.assertNotIn(
            "Downloads",
            screen,
            "an English name this host does not use must not be painted",
        )
        # The bookmark labelled `saha-belgeleri` points at the same directory the
        # built-in block already claims, so path authority keeps it out of the
        # curated section rather than drawing that directory twice.
        self.assertNotIn(
            "saha-belgeleri",
            screen,
            "a bookmark whose target the built-in block already claims is not drawn twice",
        )


if __name__ == "__main__":
    unittest.main()
