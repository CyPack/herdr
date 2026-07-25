"""Tests for the fork's license guard.

The 2026-07-25 upstream sync replaced LICENSE, the manifest, the Nix
derivation and the README licence section with Apache-2.0 without producing a
single conflict, because the fork had never edited those files since the fork
point. Nothing in the suite noticed. This guard is the missing test.
"""

import tempfile
import unittest
from pathlib import Path

from scripts.license_guard_check import check


AGPL_FIRST_LINES = (
    "Herdr is dual-licensed:\n"
    "\n"
    "1. Open source: GNU Affero General Public License v3.0 or later "
    "(AGPL-3.0-or-later).\n"
)


def _write(root: Path, rel: str, text: str) -> None:
    path = root / rel
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


class LicenseGuardTests(unittest.TestCase):
    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        self.root = Path(self._tmp.name)
        self.addCleanup(self._tmp.cleanup)
        self._healthy()

    def _healthy(self) -> None:
        _write(self.root, "LICENSE", AGPL_FIRST_LINES)
        _write(self.root, "Cargo.toml", '[package]\nname = "herdr"\nlicense = "AGPL-3.0-or-later"\n')
        _write(self.root, "nix/package.nix", "{\n  meta = {\n    license = lib.licenses.agpl3Plus;\n  };\n}\n")
        _write(self.root, "README.md", "## license\n\nHerdr is dual-licensed:\n")
        _write(self.root, "LICENSE-APACHE", "                                 Apache License\n")
        _write(self.root, "NOTICE", "Herdr (CyPack fork)\n")

    def test_healthy_tree_passes(self) -> None:
        self.assertEqual(check(self.root), [])

    def test_apache_license_body_is_reported(self) -> None:
        _write(self.root, "LICENSE", "                                 Apache License\n")
        errors = check(self.root)
        self.assertTrue(any("LICENSE" in e for e in errors), errors)

    def test_manifest_relicense_is_reported(self) -> None:
        _write(self.root, "Cargo.toml", '[package]\nname = "herdr"\nlicense = "Apache-2.0"\n')
        errors = check(self.root)
        self.assertTrue(any("Cargo.toml" in e for e in errors), errors)

    def test_nix_relicense_is_reported(self) -> None:
        _write(self.root, "nix/package.nix", "{\n  meta = {\n    license = lib.licenses.asl20;\n  };\n}\n")
        errors = check(self.root)
        self.assertTrue(any("package.nix" in e for e in errors), errors)

    def test_readme_relicense_is_reported(self) -> None:
        _write(self.root, "README.md", "## license\n\nHerdr is licensed under the Apache License 2.0.\n")
        errors = check(self.root)
        self.assertTrue(any("README" in e for e in errors), errors)

    def test_missing_apache_attribution_is_reported(self) -> None:
        # Upstream code from cd5ea1be onward is Apache-2.0; dropping its licence
        # copy or the NOTICE would break the attribution the fork owes it.
        (self.root / "LICENSE-APACHE").unlink()
        errors = check(self.root)
        self.assertTrue(any("LICENSE-APACHE" in e for e in errors), errors)

    def test_missing_notice_is_reported(self) -> None:
        (self.root / "NOTICE").unlink()
        errors = check(self.root)
        self.assertTrue(any("NOTICE" in e for e in errors), errors)

    def test_every_violation_is_reported_together(self) -> None:
        # A sync flips all four at once, so the report must not stop at the first.
        _write(self.root, "LICENSE", "                                 Apache License\n")
        _write(self.root, "Cargo.toml", 'license = "Apache-2.0"\n')
        _write(self.root, "nix/package.nix", "license = lib.licenses.asl20;\n")
        _write(self.root, "README.md", "Herdr is licensed under the Apache License 2.0.\n")
        self.assertGreaterEqual(len(check(self.root)), 4)


class RealTreeTests(unittest.TestCase):
    def test_this_repository_is_agpl(self) -> None:
        self.assertEqual(check(Path(__file__).resolve().parents[1]), [])


if __name__ == "__main__":
    unittest.main()
