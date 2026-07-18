from __future__ import annotations

import unittest
from pathlib import Path


PROJECT_ROOT = Path(__file__).resolve().parent.parent
SRC_ROOT = PROJECT_ROOT / "src"


def rust_sources() -> dict[Path, str]:
    return {
        path.relative_to(PROJECT_ROOT): path.read_text()
        for path in sorted(SRC_ROOT.rglob("*.rs"))
    }


class TrailT7TeardownTests(unittest.TestCase):
    def test_retired_miller_projection_symbols_are_absent(self) -> None:
        sources = rust_sources()
        retired = (
            "MillerDirectoryProjection",
            "MillerDirectorySource",
            "MillerRowColumnKind",
            "render_snapshot_panel",
            "resident_non_current",
        )

        hits = {
            symbol: [str(path) for path, text in sources.items() if symbol in text]
            for symbol in retired
        }
        self.assertEqual({symbol: paths for symbol, paths in hits.items() if paths}, {})

    def test_retired_watcher_and_image_seams_are_absent(self) -> None:
        sources = rust_sources()
        retired = ("request_current_refresh", "legacy_file_manager_image_target")

        hits = {
            symbol: [str(path) for path, text in sources.items() if symbol in text]
            for symbol in retired
        }
        self.assertEqual({symbol: paths for symbol, paths in hits.items() if paths}, {})

    def test_no_legacy_placeholder_or_teardown_marker_remains(self) -> None:
        sources = rust_sources()
        retired = ('"(unavailable)"', "TRAIL-T7.6 teardown")

        hits = {
            symbol: [str(path) for path, text in sources.items() if symbol in text]
            for symbol in retired
        }
        self.assertEqual({symbol: paths for symbol, paths in hits.items() if paths}, {})

    def test_retained_geometry_modules_need_no_dead_code_allowance(self) -> None:
        retained = (
            PROJECT_ROOT / "src" / "fm" / "miller.rs",
            PROJECT_ROOT / "src" / "ui" / "file_manager" / "miller.rs",
        )
        offenders = [
            str(path.relative_to(PROJECT_ROOT))
            for path in retained
            if "#![allow(dead_code)]" in path.read_text()
        ]
        self.assertEqual(offenders, [])


if __name__ == "__main__":
    unittest.main()
