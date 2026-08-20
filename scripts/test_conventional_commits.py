"""The commit-message gate, including the rule that only a document carried.

`CLAUDE.md` says commit subjects are lowercase conventional commits with no
emojis and **no AI co-author lines**. The first half was mechanical from the
day it was written; the second half was a sentence in a file, and on
2026-08-20 two commits landed on `master` carrying
`Co-Authored-By: Claude …` because an agent's own default outranked the
document nobody could enforce.

A rule a tool cannot check is a rule that holds until someone is busy. These
tests are the gate that makes the second half as mechanical as the first.
"""

from __future__ import annotations

import contextlib
import io
import tempfile
import unittest
from pathlib import Path

from . import conventional_commits


def check_message(text: str) -> int:
    """Run the gate the way the `commit-msg` hook runs it."""
    with tempfile.TemporaryDirectory() as directory:
        path = Path(directory) / "COMMIT_EDITMSG"
        path.write_text(text, encoding="utf-8")
        return conventional_commits.main_for_test(["--message-file", str(path)])


class ConventionalSubjectTests(unittest.TestCase):
    """The half that was already mechanical, pinned so it stays that way."""

    def test_a_conventional_subject_is_accepted(self) -> None:
        self.assertEqual(check_message("fix(update): install selected channel\n"), 0)

    def test_an_unconventional_subject_is_refused(self) -> None:
        self.assertEqual(check_message("Fixed the updater\n"), 1)


class AiCoAuthorTests(unittest.TestCase):
    """The half that was only a sentence."""

    def test_an_ai_co_author_trailer_is_refused(self) -> None:
        """The exact shape that reached master on 2026-08-20."""
        message = (
            "feat(shell): let one bar section wear its own frame\n"
            "\n"
            "Body text.\n"
            "\n"
            "Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>\n"
        )
        self.assertEqual(check_message(message), 1)

    def test_a_generated_with_line_is_refused(self) -> None:
        message = (
            "docs: write the guide\n"
            "\n"
            "Generated with [Claude Code](https://claude.com/claude-code)\n"
        )
        self.assertEqual(check_message(message), 1)

    def test_a_human_co_author_is_still_accepted(self) -> None:
        """The discriminating case.

        Refusing every `Co-Authored-By:` would be a different rule from the one
        `CLAUDE.md` states, and it would break the ordinary practice of
        crediting a person who worked on the change. A gate that cannot tell
        those apart is worse than the sentence it replaced, because it is
        wrong in a direction nobody expects.
        """
        message = (
            "fix(pane): keep the cursor where the person put it\n"
            "\n"
            "Co-Authored-By: Ada Lovelace <ada@example.com>\n"
        )
        self.assertEqual(check_message(message), 0)

    def test_the_refusal_says_which_line_and_which_rule(self) -> None:
        """A refusal nobody can act on sends someone to guess.

        Asserted on what the gate *prints*, not on what the helper returns.
        The helper's job is to name the offending lines; naming the rule is
        the refusal's job, and the person only ever sees the refusal.
        """
        message = (
            "feat(x): do a thing\n\nCo-Authored-By: Claude <noreply@anthropic.com>\n"
        )
        self.assertEqual(
            conventional_commits.ai_attribution_problems(message),
            ["Co-Authored-By: Claude <noreply@anthropic.com>"],
            "the helper names the line, so the refusal can quote it",
        )

        printed = io.StringIO()
        with contextlib.redirect_stdout(printed):
            self.assertEqual(check_message(message), 1)
        refusal = printed.getvalue()
        self.assertIn("Co-Authored-By", refusal)
        self.assertIn("CLAUDE.md", refusal)
        self.assertIn("still fine", refusal, "the remedy has to survive too")


class RangeModeTests(unittest.TestCase):
    """The path CI takes, where one refusal covers many commits."""

    def test_a_range_refusal_names_the_commit(self) -> None:
        """Two commits carrying the same trailer print the same line twice.

        Without the commit beside it, the person reads one sentence repeated
        and has to go and find which of the two it belongs to — a refusal that
        is technically complete and practically useless.
        """
        problems = conventional_commits.attribution_problems_in_range(
            [("abc1234", "feat(x): a\n\nCo-Authored-By: Claude <noreply@anthropic.com>\n"),
             ("def5678", "feat(y): b\n\nCo-Authored-By: Claude <noreply@anthropic.com>\n")]
        )
        self.assertEqual(len(problems), 2)
        self.assertTrue(problems[0].startswith("abc1234"), problems)
        self.assertTrue(problems[1].startswith("def5678"), problems)


if __name__ == "__main__":
    unittest.main()
