"""Tests for scripts/chat_context.py — the chat context engine (W-CHAT FAZ A).

Every fixture is synthetic and lives in a temp directory: these tests must
never read the live ~/.claude tree (the TP-DRAW-13 lesson — a fixture that
reads the machine's history moves whenever the agent running it writes to
its own transcript).

Test points come from .local/prd/chat-context-wave.md §6 (T-A1..T-A10).
"""

import json
import os
import sqlite3
import tempfile
import unittest
from pathlib import Path

from scripts import chat_context as cc


def _entry(
    ts="2026-08-20T10:00:00.000Z",
    cwd="/home/user",
    typ="user",
    text="hello",
    git_branch="main",
    content=None,
):
    """One transcript line in the shape Claude Code writes."""
    msg_content = content if content is not None else text
    return {
        "type": typ,
        "timestamp": ts,
        "cwd": cwd,
        "gitBranch": git_branch,
        "sessionId": "s-fixture",
        "uuid": "u",
        "message": {"role": typ, "content": msg_content},
    }


def _tool_use(command, tool_id="tu-1"):
    return [
        {"type": "text", "text": "running"},
        {"type": "tool_use", "id": tool_id, "name": "Bash", "input": {"command": command}},
    ]


def _tool_result(tool_id, stdout):
    return {
        "type": "user",
        "timestamp": "2026-08-20T10:00:05.000Z",
        "cwd": "/home/user",
        "sessionId": "s-fixture",
        "toolUseResult": {"stdout": stdout, "stderr": "", "interrupted": False},
        "message": {
            "role": "user",
            "content": [{"type": "tool_result", "tool_use_id": tool_id, "content": stdout}],
        },
    }


def _write_jsonl(path, entries):
    path.parent.mkdir(parents=True, exist_ok=True)
    with open(path, "w") as f:
        for e in entries:
            f.write(json.dumps(e) + "\n")


class ExtractFactsTest(unittest.TestCase):
    """T-A1 / T-A3 — scanner facts: cwd runs, title, substantive clock."""

    def _facts(self, entries, name="abc.jsonl"):
        with tempfile.TemporaryDirectory() as td:
            p = Path(td) / name
            _write_jsonl(p, entries)
            return cc.extract_session_facts(str(p))

    def test_cwd_runs_are_extracted_in_order(self):
        # T-A1: three cwd runs, correct order — the foundation of segmentation.
        entries = (
            [_entry(cwd="/home/u", text=f"a{i}") for i in range(3)]
            + [_entry(cwd="/home/u/projects/alpha", text=f"b{i}") for i in range(4)]
            + [_entry(cwd="/home/u", text=f"c{i}") for i in range(2)]
        )
        facts = self._facts(entries)
        runs = [(r["dir"], r["weight"]) for r in facts["cwd_runs"]]
        self.assertEqual(
            runs,
            [("/home/u", 3), ("/home/u/projects/alpha", 4), ("/home/u", 2)],
        )

    def test_title_prefers_custom_then_ai_then_first_user(self):
        entries = [
            {"type": "ai-title", "aiTitle": "AI TITLE", "sessionId": "s"},
            _entry(text="first user words"),
        ]
        self.assertEqual(self._facts(entries)["title"], "AI TITLE")
        entries.insert(0, {"type": "custom-title", "customTitle": "CUSTOM", "sessionId": "s"})
        self.assertEqual(self._facts(entries)["title"], "CUSTOM")
        self.assertEqual(
            self._facts([_entry(text="first user words")])["title"], "first user words"
        )

    def test_tool_noise_does_not_advance_the_clock(self):
        # T-A3: tool-result user lines and injected <...> blocks must not move
        # the last-substantive-message clock — parity with TP-DRAW-16, or the
        # age surface and our plan disagree about the same chat.
        entries = [
            _entry(ts="2026-08-20T10:00:00.000Z", text="real question"),
            _tool_result("tu-9", "big output"),
            _entry(
                ts="2026-08-20T11:00:00.000Z",
                text="<system-reminder>noise</system-reminder>",
            ),
            _entry(
                ts="2026-08-20T12:00:00.000Z",
                typ="assistant",
                content=[{"type": "tool_use", "id": "x", "name": "Bash", "input": {}}],
            ),
        ]
        facts = self._facts(entries)
        self.assertEqual(facts["last_substantive_ts"], "2026-08-20T10:00:00.000Z")

    def test_assistant_visible_text_advances_the_clock(self):
        entries = [
            _entry(ts="2026-08-20T10:00:00.000Z", text="q"),
            _entry(
                ts="2026-08-20T10:30:00.000Z",
                typ="assistant",
                content=[{"type": "text", "text": "an answer"}],
            ),
        ]
        facts = self._facts(entries)
        self.assertEqual(facts["last_substantive_ts"], "2026-08-20T10:30:00.000Z")

    def test_session_id_comes_from_filename(self):
        facts = self._facts([_entry()], name="0f9a3b2c-1111-2222-3333-444455556666.jsonl")
        self.assertEqual(facts["session_id"], "0f9a3b2c-1111-2222-3333-444455556666")


class WorklogTest(unittest.TestCase):
    """T-A4 / T-A5 / T-A6 — commit and push extraction."""

    def _facts(self, entries):
        with tempfile.TemporaryDirectory() as td:
            p = Path(td) / "w.jsonl"
            _write_jsonl(p, entries)
            return cc.extract_session_facts(str(p))

    def test_commit_with_dash_c_attributes_to_that_repo(self):
        # T-A4: `git -C ~/repo commit` belongs to ~/repo, not to the cwd —
        # lose this and a commit lands under the wrong project.
        entries = [
            _entry(
                typ="assistant",
                cwd="/home/u",
                content=_tool_use(
                    'git -C /home/u/projects/alpha commit -m "feat(scan): add scanner"'
                ),
            ),
            _tool_result("tu-1", "[feat/scanner 1a2b3c4] feat(scan): add scanner"),
        ]
        commits = self._facts(entries)["commits"]
        self.assertEqual(len(commits), 1)
        c = commits[0]
        self.assertEqual(c["repo"], "/home/u/projects/alpha")
        self.assertEqual(c["type"], "feat")
        self.assertEqual(c["scope"], "scan")
        self.assertEqual(c["subject"], "add scanner")
        self.assertEqual(c["sha"], "1a2b3c4")
        self.assertEqual(c["branch"], "feat/scanner")

    def test_commit_without_result_sha_is_attempted(self):
        # T-A5: a failed commit must not pollute the chronology as done work.
        entries = [
            _entry(typ="assistant", content=_tool_use('git commit -m "fix(x): broken"')),
        ]
        commits = self._facts(entries)["commits"]
        self.assertEqual(len(commits), 1)
        self.assertIsNone(commits[0]["sha"])
        self.assertEqual(commits[0]["status"], "attempted")

    def test_push_marks_the_branch_pushed(self):
        # T-A6: pushed=true only for the pushed branch.
        entries = [
            _entry(
                typ="assistant",
                cwd="/home/u/projects/alpha",
                content=_tool_use('git commit -m "feat(a): one"', "tu-1"),
            ),
            _tool_result("tu-1", "[feat/a 1111111] feat(a): one"),
            _entry(
                typ="assistant",
                cwd="/home/u/projects/alpha",
                content=_tool_use('git commit -m "fix(b): two"', "tu-2"),
            ),
            _tool_result("tu-2", "[fix/b 2222222] fix(b): two"),
            _entry(
                typ="assistant",
                cwd="/home/u/projects/alpha",
                content=_tool_use("git push origin feat/a", "tu-3"),
            ),
            _tool_result("tu-3", "To github.com:me/alpha.git\n   aaa..bbb  feat/a -> feat/a"),
        ]
        commits = self._facts(entries)["commits"]
        by_branch = {c["branch"]: c for c in commits}
        self.assertTrue(by_branch["feat/a"]["pushed"])
        self.assertFalse(by_branch["fix/b"]["pushed"])

    def test_non_conventional_subject_keeps_raw_and_type_other(self):
        entries = [
            _entry(typ="assistant", content=_tool_use('git commit -m "wip stuff"')),
        ]
        c = self._facts(entries)["commits"][0]
        self.assertEqual(c["type"], "other")
        self.assertEqual(c["subject"], "wip stuff")


class CacheTest(unittest.TestCase):
    """T-A2 — incremental (mtime_ns, size) cache."""

    def test_unchanged_file_is_not_reparsed(self):
        with tempfile.TemporaryDirectory() as td:
            proj = Path(td) / "projects" / "-home-u"
            f = proj / "s1.jsonl"
            _write_jsonl(f, [_entry(text="hello world")])
            db = Path(td) / "state.db"
            cache = cc.FactsCache(str(db))
            r1 = cache.scan_dir(str(proj))
            r2 = cache.scan_dir(str(proj))
            self.assertEqual(r1["parsed"], 1)
            self.assertEqual(r2["parsed"], 0)
            self.assertEqual(r2["cached"], 1)
            # a change re-parses
            _write_jsonl(f, [_entry(text="hello world"), _entry(text="more")])
            r3 = cache.scan_dir(str(proj))
            self.assertEqual(r3["parsed"], 1)
            cache.close()

    def test_deleted_file_leaves_no_stale_row(self):
        with tempfile.TemporaryDirectory() as td:
            proj = Path(td) / "projects" / "-home-u"
            f = proj / "s1.jsonl"
            _write_jsonl(f, [_entry()])
            db = Path(td) / "state.db"
            cache = cc.FactsCache(str(db))
            cache.scan_dir(str(proj))
            os.unlink(f)
            cache.scan_dir(str(proj))
            con = sqlite3.connect(str(db))
            n = con.execute("SELECT COUNT(*) FROM facts").fetchone()[0]
            con.close()
            cache.close()
            self.assertEqual(n, 0)


def _rules_toml(td):
    """A miniature spaces config mirroring the live shape."""
    cfg = Path(td) / "config.toml"
    cfg.write_text(
        """
[[spaces.node]]
key = "co-alpha"
name = "Alpha Co"
parent = "project:demo"

[[spaces.split]]
repo = "~/projects/alpha"
match = ["*scanner*", "*scan-*"]
key = "alpha:scan"
label = "Scanner"
parent = "co-alpha"

[[spaces.split]]
repo = "~/projects/alpha"
match = ["*upload*"]
key = "alpha:upload"
label = "Upload"
parent = "co-alpha"

[[spaces.project]]
name = "demo"
key = "project:demo"
repos = ["~/projects/alpha"]
"""
    )
    return str(cfg)


class ClassifyTest(unittest.TestCase):
    """T-A7 / T-A8 / T-A9 — classification layers and their order."""

    def setUp(self):
        self.td = tempfile.TemporaryDirectory()
        self.home = self.td.name
        os.makedirs(os.path.join(self.home, "projects", "alpha"), exist_ok=True)
        self.rules = cc.load_space_rules(_rules_toml(self.td.name), home=self.home)

    def tearDown(self):
        self.td.cleanup()

    def _base_facts(self, **over):
        facts = {
            "session_id": "sid-1",
            "title": "",
            "last_prompt": "",
            "cwd_runs": [{"dir": self.home, "weight": 5, "first_ts": "", "last_ts": ""}],
            "commits": [],
            "last_substantive_ts": "2026-08-20T10:00:00.000Z",
        }
        facts.update(over)
        return facts

    def test_dominant_repo_cwd_beats_lexicon(self):
        # T-A9: deterministic evidence (the chat actually worked in the repo)
        # wins over any keyword — K3 layer order.
        alpha = os.path.join(self.home, "projects", "alpha")
        facts = self._base_facts(
            title="upload things",  # lexicon would say alpha:upload
            cwd_runs=[
                {"dir": self.home, "weight": 2, "first_ts": "", "last_ts": ""},
                {"dir": alpha, "weight": 30, "first_ts": "", "last_ts": ""},
            ],
            commits=[
                {
                    "repo": alpha,
                    "branch": "feat/scan-fast",
                    "type": "feat",
                    "scope": "scan",
                    "subject": "s",
                    "sha": "abc1234",
                    "ts": "",
                    "pushed": False,
                    "status": "committed",
                }
            ],
        )
        seats = cc.classify(facts, self.rules)
        self.assertTrue(seats)
        top = seats[0]
        # branch feat/scan-fast matches *scan-* → module seat, module: key
        self.assertEqual(top["key"], "module:alpha:scan")
        self.assertEqual(top["dir"], alpha)
        self.assertGreaterEqual(top["conf"], 0.8)
        self.assertEqual(top["basis"], "repo+branch")

    def test_pure_home_chat_with_lexicon_hit_classifies(self):
        # T-A7: a $HOME-only chat with a module keyword in the title seats
        # under that module with lexicon confidence.
        facts = self._base_facts(title="upload the SOR files to the portal")
        seats = cc.classify(facts, self.rules)
        self.assertTrue(seats)
        self.assertEqual(seats[0]["key"], "module:alpha:upload")
        self.assertEqual(seats[0]["basis"], "lexicon")
        self.assertLess(seats[0]["conf"], 0.8)

    def test_no_signal_stays_open(self):
        # T-A8: a generic chat must NOT be seated — a wrong seat is worse
        # than no seat.
        facts = self._base_facts(title="what is the weather like")
        self.assertEqual(cc.classify(facts, self.rules), [])

    def test_repo_without_module_match_falls_to_repo_dir(self):
        alpha = os.path.join(self.home, "projects", "alpha")
        facts = self._base_facts(
            cwd_runs=[{"dir": alpha, "weight": 20, "first_ts": "", "last_ts": ""}],
        )
        seats = cc.classify(facts, self.rules)
        self.assertTrue(seats)
        self.assertEqual(seats[0]["key"], alpha)  # plain ledger dir key
        self.assertEqual(seats[0]["basis"], "repo")


class PlanTest(unittest.TestCase):
    """T-A10 + K2c — plan emission, dry-run, per-target cap."""

    def test_plan_caps_per_target_and_keeps_newest(self):
        rules = None  # plan works on pre-classified seats
        sessions = []
        for i in range(200):
            sessions.append(
                {
                    "session_id": f"sid-{i:03d}",
                    "seats": [{"key": "module:alpha:scan", "dir": "/r", "conf": 0.9, "basis": "repo"}],
                    "last_substantive_ts": f"2026-08-{(i % 28) + 1:02d}T10:00:00.000Z",
                }
            )
        plan = cc.build_plan(sessions, per_target_cap=150)
        seated = [s for s in plan["seats"] if s["target_key"] == "module:alpha:scan"]
        self.assertEqual(len(seated), 150)
        # newest kept: every dropped session is older-or-equal than every kept one
        kept = {s["session_id"] for s in seated}
        newest = sorted(sessions, key=lambda s: s["last_substantive_ts"], reverse=True)[:150]
        self.assertEqual(kept, {s["session_id"] for s in newest})

    def test_min_conf_filters_seats(self):
        sessions = [
            {
                "session_id": "low",
                "seats": [{"key": "k", "dir": "/r", "conf": 0.3, "basis": "lexicon"}],
                "last_substantive_ts": "2026-08-20T10:00:00.000Z",
            },
            {
                "session_id": "high",
                "seats": [{"key": "k", "dir": "/r", "conf": 0.9, "basis": "repo"}],
                "last_substantive_ts": "2026-08-20T10:00:00.000Z",
            },
        ]
        plan = cc.build_plan(sessions, min_conf=0.6)
        ids = {s["session_id"] for s in plan["seats"]}
        self.assertEqual(ids, {"high"})


if __name__ == "__main__":
    unittest.main()
