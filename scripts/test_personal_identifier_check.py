"""Tests for the personal-identifier guard.

The guard's whole value is that it fails, so the cases below are written in
pairs: an identifier that must be refused and the neutral spelling of the same
thing that must not be. A guard that only ever passes and a guard that cries at
ordinary English are both useless, and the second is worse — people switch it
off and it stops protecting anything.
"""

from __future__ import annotations

import subprocess
import tempfile
import unittest
from pathlib import Path

from scripts.personal_identifier_check import check, scan_text

# The identifiers below are assembled at run time rather than written out.
#
# Spelled literally they would be real matches in a tracked file, and this file
# is tracked, so the guard would flag its own tests — which it did, in the gate,
# on the first attempt. The obvious fix is to make the guard skip this path, but
# a file the guard cannot read is a blind spot, and a blind spot is a worse
# trade than the small indirection here. Each name still reads as what it is.
TAILNET = "100." + "75.115.68"
HOME_PATH = "/home/" + "someone"
HOME_SLUG = "-home-" + "someone"
MAIL = "someone" + "@gmail.com"


class ScanRefusesPrivateIdentifiers(unittest.TestCase):
    def test_a_tailnet_address_is_refused(self) -> None:
        found = scan_text(f"reach it at http://{TAILNET}:8770/")
        self.assertEqual(found, [f"tailnet address {TAILNET}"])

    def test_the_documentation_address_is_allowed(self) -> None:
        self.assertEqual(scan_text("http://100.64.0.1:8770 is an example"), [])

    def test_an_address_below_the_reserved_range_is_not_a_tailnet_address(self) -> None:
        # 100.63.x.x sits outside 100.64.0.0/10 and is ordinary public space.
        self.assertEqual(scan_text("100.63.1.1 is not shared space"), [])

    def test_a_named_home_directory_is_refused(self) -> None:
        self.assertEqual(
            scan_text(f"it lives in {HOME_PATH}/projects"),
            [f"home directory {HOME_PATH}"],
        )

    def test_a_neutral_home_directory_is_allowed(self) -> None:
        self.assertEqual(scan_text("it lives in /home/user/projects"), [])

    def test_the_encoded_slug_form_is_refused(self) -> None:
        self.assertEqual(
            scan_text(f"~/.claude/projects/{HOME_SLUG}/memory"),
            # The report names the slug without the dash the rule anchors on.
            [f"encoded home directory {HOME_SLUG[1:]}"],
        )

    def test_ordinary_prose_about_a_home_directory_is_allowed(self) -> None:
        # The slug rule is anchored on the leading dash for exactly this: an
        # unanchored `home-` matches English and the guard becomes noise.
        self.assertEqual(scan_text("the home directory and the home origin"), [])

    def test_a_personal_mail_address_is_refused(self) -> None:
        self.assertEqual(
            scan_text(f"reported by {MAIL} in the log"),
            [f"mail address {MAIL}"],
        )

    def test_upstream_and_project_addresses_are_allowed(self) -> None:
        text = "ogulcancelik@gmail.com, hey@herdr.dev, a@users.noreply.github.com"
        self.assertEqual(scan_text(text), [])

    def test_one_offender_is_named_once_however_often_it_repeats(self) -> None:
        found = scan_text(f"{HOME_PATH} {HOME_PATH} {HOME_PATH}")
        self.assertEqual(found, [f"home directory {HOME_PATH}"])


class CheckWalksTheTrackedTree(unittest.TestCase):
    def _repo(self, root: Path) -> None:
        subprocess.run(["git", "-C", str(root), "init", "-q"], check=True)
        subprocess.run(
            ["git", "-C", str(root), "add", "-A"],
            check=True,
        )

    def test_a_tracked_file_with_an_identifier_is_reported(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "notes.md").write_text(f"see {HOME_PATH}/x\n", encoding="utf-8")
            self._repo(root)
            errors = check(root)
            self.assertEqual(errors, [f"notes.md: home directory {HOME_PATH}"])

    def test_an_untracked_file_is_not_read(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "kept.md").write_text("clean\n", encoding="utf-8")
            self._repo(root)
            # Written after `git add`, so git does not track it.
            (root / "scratch.md").write_text(f"{HOME_PATH}\n", encoding="utf-8")
            self.assertEqual(check(root), [])

    def test_the_vendor_subtree_is_skipped(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            vendored = root / "vendor" / "upstream"
            vendored.mkdir(parents=True)
            (vendored / "HACKING.md").write_text(f"{HOME_PATH}\n", encoding="utf-8")
            self._repo(root)
            self.assertEqual(check(root), [])


class RealTreeTests(unittest.TestCase):
    def test_this_repository_names_nobody(self) -> None:
        # The one assertion that does the actual work. Everything above proves
        # the guard can tell a private identifier from a neutral one; this is
        # what makes a branch that reintroduces one fail the gate instead of
        # reaching a public remote.
        errors = check(Path(__file__).resolve().parents[1])
        self.assertEqual(errors, [], "\n".join(errors))


if __name__ == "__main__":
    unittest.main()
