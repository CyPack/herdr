#!/usr/bin/env python3
from __future__ import annotations

import argparse
import re
import subprocess
from pathlib import Path

ALLOWED_TYPES = {
    "feat",
    "fix",
    "perf",
    "docs",
    "ci",
    "test",
    "refactor",
    "chore",
    "release",
}
SUBJECT_RE = re.compile(r"^(?P<kind>[a-z]+)(?:\([^)]+\))?!?:\s+\S")

# CLAUDE.md, "Commit Style": lowercase conventional commits, no emojis, and no
# AI co-author lines. The first two were mechanical from the day they were
# written; the third was a sentence in a document, and on 2026-08-20 two
# commits reached master carrying `Co-Authored-By: Claude …` because an agent's
# own default outranked a rule nothing could enforce. A rule a tool cannot
# check is a rule that holds until somebody is busy.
#
# Matched on the assistant, not on the trailer. Refusing every
# `Co-Authored-By:` would be a different rule from the one the document states
# and would break the ordinary practice of crediting a person who worked on the
# change — a gate that cannot tell those apart is worse than the sentence it
# replaced, because it is wrong in a direction nobody expects.
AI_ATTRIBUTION_PATTERNS = (
    re.compile(r"^\s*co-authored-by:.*noreply@anthropic\.com", re.I | re.M),
    re.compile(r"^\s*co-authored-by:.*\bclaude\b", re.I | re.M),
    re.compile(r"^\s*co-authored-by:.*copilot@github\.com", re.I | re.M),
    re.compile(
        r"^\s*co-authored-by:.*\b(chatgpt|openai|codex|gemini|cursor|devin|aider)\b",
        re.I | re.M,
    ),
    re.compile(r"^.*generated with \[?claude code", re.I | re.M),
    re.compile(r"^\s*(?:\U0001F916\s*)?generated with \[?[a-z ]*\b(ai|copilot)\b", re.I | re.M),
)


def ai_attribution_problems(message: str) -> list[str]:
    """The lines in one commit message that credit an assistant as an author.

    Returns the offending lines rather than a bare verdict: a refusal nobody
    can act on sends somebody to guess which of twenty lines is the problem.
    """
    offending: list[str] = []
    for line in message.splitlines():
        # The whole line, not the matched span. Two patterns can hit the same
        # trailer and each reports only as far as it looked, so quoting the
        # match hands somebody half a line to go and find — and twice.
        if any(pattern.search(line) for pattern in AI_ATTRIBUTION_PATTERNS):
            stripped = line.strip()
            if stripped not in offending:
                offending.append(stripped)
    return offending


def commit_message_body(path: Path) -> str:
    """One commit message with its comment lines dropped, as git will store it."""
    lines = [
        line
        for line in path.read_text(encoding="utf-8").splitlines()
        if not line.startswith("#")
    ]
    return "\n".join(lines)


def git_messages(rev_range: str) -> list[tuple[str, str]]:
    """Whole messages with their commits — the trailers live in the body.

    Paired with the abbreviated sha because one refusal can cover many commits,
    and two commits carrying the same trailer print the same line twice: the
    person would read one sentence repeated and still have to go and find which
    of the two it belongs to.
    """
    # `%x00` is expanded by git into its own output. Putting the byte in the
    # argument instead makes execve refuse the call outright, which is what the
    # first draft of this did.
    output = subprocess.check_output(
        ["git", "log", "--pretty=format:%h %B%x00", rev_range], text=True
    )
    pairs: list[tuple[str, str]] = []
    for record in output.split("\x00"):
        record = record.strip("\n")
        if not record.strip():
            continue
        commit, _, message = record.partition(" ")
        pairs.append((commit, message))
    return pairs


def attribution_problems_in_range(
    messages: list[tuple[str, str]],
) -> list[str]:
    """Every assistant-credited line in a range, each named by its commit."""
    return [
        f"{commit}  {problem}"
        for commit, message in messages
        for problem in ai_attribution_problems(message)
    ]


def git_subjects(rev_range: str) -> list[str]:
    output = subprocess.check_output(
        ["git", "log", "--pretty=format:%s", rev_range], text=True
    ).strip()
    return [line.strip() for line in output.splitlines() if line.strip()]


def valid_subject(subject: str) -> bool:
    match = SUBJECT_RE.match(subject)
    return bool(match and match.group("kind") in ALLOWED_TYPES)


def commit_message_subject(path: Path) -> str | None:
    for line in path.read_text(encoding="utf-8").splitlines():
        subject = line.strip()
        if subject and not subject.startswith("#"):
            return subject
    return None


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Validate conventional commit subjects")
    parser.add_argument("subjects", nargs="*")
    parser.add_argument("--range", dest="rev_range")
    parser.add_argument("--message-file")
    args = parser.parse_args(argv)

    subjects = list(args.subjects)
    attributed: list[str] = []
    if args.rev_range:
        subjects.extend(git_subjects(args.rev_range))
        attributed.extend(
            attribution_problems_in_range(git_messages(args.rev_range))
        )
    if args.message_file:
        path = Path(args.message_file)
        subject = commit_message_subject(path)
        if subject:
            subjects.append(subject)
        # No commit to name here: the hook runs before there is one, and the
        # message being refused is the one on screen.
        attributed.extend(ai_attribution_problems(commit_message_body(path)))

    if attributed:
        print("commit message credits an assistant as an author:")
        for line in attributed:
            print(f"  {line}")
        print(
            "CLAUDE.md, Commit Style: lowercase conventional commits, no emojis, "
            "and no AI co-author lines."
        )
        print(
            "remove the trailer. a human Co-Authored-By line for somebody who "
            "worked on the change is still fine."
        )
        return 1

    invalid = [subject for subject in subjects if not valid_subject(subject)]
    if invalid:
        print("invalid commit subject(s):")
        for subject in invalid:
            print(f"  {subject}")
        print(
            "commit subjects must use conventional commits because preview notes are generated from them."
        )
        print("example: fix(update): install selected channel")
        print("expected: type(optional-scope): subject")
        print("allowed types: " + ", ".join(sorted(ALLOWED_TYPES)))
        return 1
    return 0


def main_for_test(argv: list[str]) -> int:
    """`main` with an explicit argument list, so a test need not touch `sys.argv`."""
    return main(argv)


if __name__ == "__main__":
    raise SystemExit(main())
