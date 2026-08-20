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
import os
import subprocess
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


class CommentLineTests(unittest.TestCase):
    """What `git commit -v` puts under the message."""

    def test_a_commented_diff_mentioning_a_trailer_does_not_refuse_the_commit(
        self,
    ) -> None:
        """Found by mutation, and not hypothetical.

        With `commit.verbose` on, git writes the whole diff into
        `COMMIT_EDITMSG` as comment lines. A commit that edits *this file* then
        carries this file's own patterns under the `#`, and a gate that read
        them would refuse every change to itself — the one commit it must not
        block.

        Two independent things prevent it, and each is sufficient on its own:
        the comment lines are dropped before anything is matched, and the
        patterns are anchored at the start of a line so a `# +` prefix could
        not match anyway. Mutation shows this directly — removing either one
        alone changes nothing here, and removing both together fails this
        test. So this pins the *property*, not either mechanism, and a
        surviving single mutant on either is masking rather than a gap.
        """
        message = (
            "chore(commits): tune the gate\n"
            "\n"
            "# Please enter the commit message for your changes.\n"
            "# On branch master\n"
            "# ------------------------ >8 ------------------------\n"
            "# diff --git a/scripts/conventional_commits.py\n"
            "# +    re.compile(r\"^co-authored-by:.*\\bclaude\\b\", re.I),\n"
            "# +Co-Authored-By: Claude <noreply@anthropic.com>\n"
        )
        self.assertEqual(
            check_message(message),
            0,
            "a trailer quoted inside a commented diff is not a trailer",
        )


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


    def test_range_mode_reads_the_body_from_git(self) -> None:
        """The end of the path, against a real repository.

        Found by mutation: changing the pretty format from `%B` to `%s` — so
        the gate reads subjects and never sees a trailer — changed no test,
        because the range test above handed pre-built pairs to the helper and
        never went near git. A test that stops at the helper proves the helper.
        """
        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory)
            run = lambda *args: subprocess.run(
                ["git", "-C", str(repo), *args],
                check=True,
                capture_output=True,
                text=True,
            )
            run("init", "-q", "-b", "main")
            run("config", "user.name", "Test")
            run("config", "user.email", "test@example.com")
            (repo / "a.txt").write_text("one\n", encoding="utf-8")
            run("add", "a.txt")
            run(
                "commit",
                "-q",
                "-m",
                "feat(x): a thing\n\nCo-Authored-By: Claude <noreply@anthropic.com>",
            )

            cwd = os.getcwd()
            os.chdir(repo)
            try:
                messages = conventional_commits.git_messages("HEAD")
                problems = conventional_commits.attribution_problems_in_range(messages)
            finally:
                os.chdir(cwd)

        self.assertEqual(len(messages), 1, messages)
        self.assertIn(
            "Co-Authored-By",
            messages[0][1],
            "range mode must read the body, not just the subject",
        )
        self.assertEqual(len(problems), 1, problems)


if __name__ == "__main__":
    unittest.main()
