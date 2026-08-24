"""Chat context engine (W-CHAT FAZ A) — seat Claude Code chats under herdr modules.

herdr's own transcript reader (`src/claude_sessions.rs`) deliberately never
parses `cwd`/`gitBranch` out of a transcript: a chat belongs to the directory
its file's slug encodes.  That is right for the drawer, and it is exactly why
1300+ chats born under $HOME are invisible to the module tree — the work they
did inside project checkouts is written INSIDE the transcript, not in its path.

This module reads what herdr does not:

* per-session facts — the sequence of working directories the chat visited
  (`cwd_runs`), its title, and the last SUBSTANTIVE message timestamp (the
  same content sieve contract as TP-DRAW-16: tool noise and injected blocks
  must not advance the clock, or this plan and the drawer's age column would
  disagree about the same chat);
* the work log — `git commit` / `git push` tool calls with their results, so
  a chat's commits can be shown grouped by feature/fix instead of drowning in
  the transcript;
* a seat plan — which ledger key (`module:<key>` or a checkout directory)
  each chat should be moved under, with confidence and basis, capped per
  target below the ledger's MAX_CHATS_PER_WORKSPACE.

Nothing here writes herdr's ledger: the plan is applied through the server
(`herdr chat seat`, FAZ B) so the file keeps its single atomic owner
(TP-WSCHAT-09).

PRD: .local/prd/chat-context-wave.md
"""

from __future__ import annotations

import argparse
import fnmatch
import json
import os
import re
import sqlite3
import sys
import time
import tomllib
from collections import defaultdict

DEFAULT_PROJECTS_DIR = os.path.expanduser("~/.claude/projects")
DEFAULT_CACHE_DB = os.path.expanduser("~/.cache/herdr-chat-context/state.db")
DEFAULT_CONFIG = os.path.expanduser("~/.config/herdr/config.toml")
DEFAULT_MANAGED = os.path.expanduser("~/.config/herdr/spaces.managed.toml")

MODULE_KEY_PREFIX = "module:"  # mirrors src/persist/workspace_chats.rs::MODULE_KEY_PREFIX

# Conventional commit shape — scripts/conventional_commits.py is the gate's
# authority for NEW commits; this is the reader for HISTORICAL ones.
_CONVENTIONAL_RE = re.compile(
    r"^(?P<type>feat|fix|docs|refactor|test|chore|perf|build|ci|style|revert)"
    r"(?:\((?P<scope>[^)]*)\))?!?:\s*(?P<subject>.+)$"
)
_COMMIT_RESULT_RE = re.compile(r"\[([^\s\]]+)[ \t]+([0-9a-f]{7,40})\]")
_PUSH_REFSPEC_RE = re.compile(r"\S+\s+->\s+(\S+)")
_TS_FUTURE_SLACK_S = 300  # mirrors FUTURE_TIMESTAMP_SLACK in claude_sessions.rs

# ---------------------------------------------------------------------------
# Substantive-content sieve (TP-DRAW-16 parity)
# ---------------------------------------------------------------------------


def _visible_text(content) -> str:
    """Human-visible text of a message content (str or block list)."""
    if isinstance(content, str):
        return content
    if isinstance(content, list):
        parts = []
        for block in content:
            if isinstance(block, dict) and block.get("type") == "text":
                parts.append(block.get("text") or "")
        return "\n".join(parts)
    return ""


def _user_text_is_substantive(text: str) -> bool:
    t = text.strip()
    if not t:
        return False
    # Injected machinery arrives wrapped in angle brackets (<system-reminder>,
    # <command-name>, <local-command-stdout>...) — never typed by a person.
    if t.startswith("<"):
        return False
    if t.startswith("[Request interrupted") or t.startswith("[Request cancelled"):
        return False
    if t.startswith("Caveat:"):
        return False
    return True


def _entry_advances_clock(entry: dict) -> bool:
    typ = entry.get("type")
    msg = entry.get("message") or {}
    content = msg.get("content")
    if typ == "user":
        if entry.get("toolUseResult") is not None:
            return False
        if isinstance(content, list) and any(
            isinstance(b, dict) and b.get("type") == "tool_result" for b in content
        ):
            return False
        return _user_text_is_substantive(_visible_text(content))
    if typ == "assistant":
        return bool(_visible_text(content).strip())
    return False


def _ts_acceptable(ts: str, now_epoch: float) -> bool:
    """Reject future-dated lines beyond a small skew allowance."""
    try:
        # ISO-8601 Zulu; lexicographic order is chronological, but the future
        # check needs an epoch.
        epoch = time.mktime(time.strptime(ts[:19], "%Y-%m-%dT%H:%M:%S"))
    except (ValueError, TypeError):
        return False
    # transcripts are UTC; allow generous slack for timezone (worst case +14h)
    return epoch <= now_epoch + 14 * 3600 + _TS_FUTURE_SLACK_S


# ---------------------------------------------------------------------------
# Commit / push extraction
# ---------------------------------------------------------------------------


def _tool_use_blocks(entry: dict):
    msg = entry.get("message") or {}
    content = msg.get("content")
    if not isinstance(content, list):
        return
    for block in content:
        if isinstance(block, dict) and block.get("type") == "tool_use":
            yield block


def _tool_result_text(entry: dict) -> tuple[str | None, str]:
    """(tool_use_id, combined result text) for a tool-result user line."""
    text_parts = []
    tid = None
    result = entry.get("toolUseResult")
    if isinstance(result, dict):
        text_parts.append(str(result.get("stdout") or ""))
        text_parts.append(str(result.get("stderr") or ""))
    elif isinstance(result, str):
        text_parts.append(result)
    msg = entry.get("message") or {}
    content = msg.get("content")
    if isinstance(content, list):
        for block in content:
            if isinstance(block, dict) and block.get("type") == "tool_result":
                tid = block.get("tool_use_id") or tid
                inner = block.get("content")
                if isinstance(inner, str):
                    text_parts.append(inner)
                elif isinstance(inner, list):
                    for b in inner:
                        if isinstance(b, dict) and b.get("type") == "text":
                            text_parts.append(b.get("text") or "")
    return tid, "\n".join(p for p in text_parts if p)


def _commit_repo(command: str, cwd: str) -> str:
    m = re.search(r"git\s+-C\s+(?:\"([^\"]+)\"|'([^']+)'|(\S+))", command)
    if m:
        path = m.group(1) or m.group(2) or m.group(3)
        return os.path.expanduser(path)
    m = re.match(r"\s*cd\s+(?:\"([^\"]+)\"|'([^']+)'|(\S+))\s*(?:&&|;)", command)
    if m:
        path = m.group(1) or m.group(2) or m.group(3)
        return os.path.expanduser(path)
    return cwd or ""


def _commit_message(command: str) -> str:
    m = re.search(r"<<\s*'?EOF'?\n(.+?)\n", command, re.DOTALL)
    if m and "-m" in command:
        return m.group(1).strip()
    m = re.search(r"-m\s+\"((?:[^\"\\]|\\.)*)\"", command, re.DOTALL)
    if not m:
        m = re.search(r"-m\s+'([^']*)'", command, re.DOTALL)
    if not m:
        return ""
    return m.group(1).split("\n", 1)[0].strip()


def _classify_subject(subject: str) -> tuple[str, str | None, str]:
    m = _CONVENTIONAL_RE.match(subject)
    if not m:
        return "other", None, subject
    return m.group("type"), m.group("scope"), m.group("subject")


def _push_branches(command: str, result_text: str) -> set[str]:
    branches = set(_PUSH_REFSPEC_RE.findall(result_text))
    tokens = command.split()
    if "push" in tokens:
        after = tokens[tokens.index("push") + 1 :]
        positional = [t for t in after if not t.startswith("-")]
        # first positional is the remote; the rest are refspecs/branches
        for tok in positional[1:]:
            branches.add(tok.split(":")[-1])
    branches.discard("HEAD")
    return branches


# ---------------------------------------------------------------------------
# Per-session fact extraction
# ---------------------------------------------------------------------------


def extract_session_facts(path: str) -> dict:
    """Read one transcript and distil the facts the planner needs.

    Single pass, line by line; malformed lines are skipped (the file is a
    live append log and its tail can be mid-write).
    """
    session_id = os.path.splitext(os.path.basename(path))[0]
    now_epoch = time.time()

    custom_title = ""
    ai_title = ""
    first_user = ""
    last_prompt = ""
    last_substantive_ts = ""
    first_ts = ""
    cwd_runs: list[dict] = []
    commits: list[dict] = []
    pending: dict[str, dict] = {}  # tool_use_id -> commit awaiting its result
    pending_push: dict[str, dict] = {}  # tool_use_id -> {repo, command}

    def _note_cwd(d: str, ts: str):
        if not d:
            return
        if cwd_runs and cwd_runs[-1]["dir"] == d:
            cwd_runs[-1]["weight"] += 1
            cwd_runs[-1]["last_ts"] = ts or cwd_runs[-1]["last_ts"]
        else:
            cwd_runs.append({"dir": d, "weight": 1, "first_ts": ts, "last_ts": ts})

    try:
        fh = open(path, "r", errors="replace")
    except OSError:
        return {
            "session_id": session_id,
            "title": "",
            "last_prompt": "",
            "cwd_runs": [],
            "commits": [],
            "last_substantive_ts": "",
            "first_ts": "",
        }
    with fh:
        for line in fh:
            if len(line) > 4_000_000:
                continue
            try:
                entry = json.loads(line)
            except (json.JSONDecodeError, ValueError):
                continue
            if not isinstance(entry, dict):
                continue
            typ = entry.get("type")

            if typ == "custom-title":
                custom_title = str(entry.get("customTitle") or "") or custom_title
                continue
            if typ == "ai-title":
                ai_title = str(entry.get("aiTitle") or "") or ai_title
                continue

            ts = str(entry.get("timestamp") or "")
            cwd = entry.get("cwd")
            if typ in ("user", "assistant") and isinstance(cwd, str):
                _note_cwd(cwd, ts)
                if ts and not first_ts:
                    first_ts = ts

            if typ == "user":
                text = _visible_text((entry.get("message") or {}).get("content"))
                if (
                    entry.get("toolUseResult") is None
                    and _user_text_is_substantive(text)
                ):
                    first_user = first_user or text.split("\n", 1)[0]
                    last_prompt = text[:200]
                # commit/push results ride user lines
                tid, result_text = _tool_result_text(entry)
                if tid and tid in pending and result_text:
                    m = _COMMIT_RESULT_RE.search(result_text)
                    if m:
                        pending[tid]["branch"] = m.group(1)
                        pending[tid]["sha"] = m.group(2)
                        pending[tid]["status"] = "committed"
                    pending.pop(tid, None)
                if tid and tid in pending_push and result_text:
                    push = pending_push.pop(tid)
                    branches = _push_branches(push["command"], result_text)
                    if "To " in result_text or "Everything up-to-date" in result_text:
                        for c in commits:
                            if c["repo"] == push["repo"] and (
                                not branches or c.get("branch") in branches
                            ):
                                c["pushed"] = True

            if typ == "assistant":
                for block in _tool_use_blocks(entry):
                    if block.get("name") != "Bash":
                        continue
                    command = str((block.get("input") or {}).get("command") or "")
                    if "git" not in command:
                        continue
                    if re.search(r"\bgit\b[^|;&]*\bcommit\b", command) and "-m" in command:
                        subject_raw = _commit_message(command)
                        ctype, scope, subject = _classify_subject(subject_raw)
                        commit = {
                            "repo": _commit_repo(command, cwd or ""),
                            "branch": None,
                            "type": ctype,
                            "scope": scope,
                            "subject": subject,
                            "sha": None,
                            "ts": ts,
                            "pushed": False,
                            "status": "attempted",
                        }
                        commits.append(commit)
                        tid = block.get("id")
                        if tid:
                            pending[tid] = commit
                    elif re.search(r"\bgit\b[^|;&]*\bpush\b", command):
                        tid = block.get("id")
                        if tid:
                            pending_push[tid] = {
                                "repo": _commit_repo(command, cwd or ""),
                                "command": command,
                            }

            if _entry_advances_clock(entry) and ts and _ts_acceptable(ts, now_epoch):
                if ts > last_substantive_ts:
                    last_substantive_ts = ts

    title = custom_title or ai_title or first_user
    return {
        "session_id": session_id,
        "title": title.strip(),
        "last_prompt": last_prompt.strip(),
        "cwd_runs": cwd_runs,
        "commits": commits,
        "last_substantive_ts": last_substantive_ts,
        "first_ts": first_ts,
    }


# ---------------------------------------------------------------------------
# Incremental cache — the claude-sessions meta.db pattern, facts flavoured
# ---------------------------------------------------------------------------


class FactsCache:
    """SQLite cache keyed by (path, mtime_ns, size): unchanged files skip parse."""

    def __init__(self, db_path: str = DEFAULT_CACHE_DB):
        os.makedirs(os.path.dirname(db_path), exist_ok=True)
        self._con = sqlite3.connect(db_path)
        self._con.execute(
            "CREATE TABLE IF NOT EXISTS facts("
            " path TEXT PRIMARY KEY, mtime_ns INTEGER, size_b INTEGER,"
            " session_id TEXT, slug TEXT, facts TEXT)"
        )

    def close(self):
        self._con.close()

    def scan_dir(self, project_dir: str) -> dict:
        """Scan one slug directory; parse only new/changed files."""
        slug = os.path.basename(project_dir.rstrip("/"))
        parsed = cached = 0
        seen = set()
        try:
            names = os.listdir(project_dir)
        except OSError:
            names = []
        rows = {
            r[0]: (r[1], r[2])
            for r in self._con.execute(
                "SELECT path, mtime_ns, size_b FROM facts WHERE slug=?", (slug,)
            )
        }
        for name in names:
            if not name.endswith(".jsonl"):
                continue
            path = os.path.join(project_dir, name)
            try:
                st = os.stat(path)
            except OSError:
                continue
            seen.add(path)
            key = (st.st_mtime_ns, st.st_size)
            if rows.get(path) == key:
                cached += 1
                continue
            facts = extract_session_facts(path)
            self._con.execute(
                "INSERT OR REPLACE INTO facts(path, mtime_ns, size_b, session_id, slug, facts)"
                " VALUES(?,?,?,?,?,?)",
                (path, st.st_mtime_ns, st.st_size, facts["session_id"], slug, json.dumps(facts)),
            )
            parsed += 1
        removed = 0
        for path in set(rows) - seen:
            self._con.execute("DELETE FROM facts WHERE path=?", (path,))
            removed += 1
        self._con.commit()
        return {"parsed": parsed, "cached": cached, "removed": removed}

    def all_facts(self, slug: str | None = None) -> list[dict]:
        q = "SELECT facts FROM facts"
        args: tuple = ()
        if slug:
            q += " WHERE slug=?"
            args = (slug,)
        return [json.loads(r[0]) for r in self._con.execute(q, args)]


# ---------------------------------------------------------------------------
# Space rules — the same first-match world the sidebar resolves
# ---------------------------------------------------------------------------

_WORD_RE = re.compile(r"[a-z0-9]{3,}")

# Tokens too generic to seat a chat on their own: they appear in almost every
# prompt in this corpus and would turn the lexicon layer into a lottery.
# "agent"/"claude"/"codex" earned their place by measurement: the first live
# dry-run seated 150 chats under the agents bucket on those words alone.
_STOPWORDS = {
    "the", "and", "for", "with", "this", "that", "les", "der", "die",
    "dosya", "dosyalari", "files", "file", "yap", "yeni", "new", "main",
    "bir", "icin", "gibi", "olarak", "sonra", "once", "module", "modul",
    "agent", "agents", "claude", "codex", "test", "code", "dallar",
    "dallari", "genel", "veri", "data", "has",
    # measured on the live corpus, second calibration pass: "task" seated
    # methodology chats under Task İzolasyonu, "herdr" seated every herdr
    # conversation under the web module.
    "task", "tasks", "taak", "sistem", "system", "continue", "herdr",
    "proje", "projesi", "web",
}

# Curated domain vocabulary per live module key — data, not code: the label
# alone cannot say that "naspan" is OTDR work or "goconnectit" is Euronet.
# Keys that do not exist in the loaded rules are silently unused.
DEFAULT_EXTRA_LEXICON: dict[str, list[str]] = {
    "ccd:t4f": ["t4f", "storing", "productie", "opmerking", "weekly", "taak", "monteur"],
    "ccd:bamcheck": ["bamcheck"],
    "ccd:whatsapp": ["whatsapp", "openwa", "evolution", "chatwoot"],
    "ccd:circet": ["circet", "kast", "lade", "vezel", "miller", "kanban"],
    "ccd:has": ["hasrapport", "hasfill", "workbench"],
    "ccd:otdr": ["otdr", "yokogawa", "aq7280", "naspan", "voorspan", "midspan", "sor", "lasse", "marker"],
    "ccd:co-euronet": [
        "voorinfra", "euronet", "scu", "goconnectit", "planbord", "planboard",
        "yukle", "upload", "huisnummer", "postcode", "sor",
    ],
    "ccd:co-odc": ["odc"],
    "ccd:platform": ["ingest", "kraal", "mega", "sse", "multiprofile"],
    "mnm:sealed": ["sealed", "0023"],
    "mnm:infra-db": ["supabase", "migration", "veldops", "mnmveldops"],
    "mnm:mobil": ["pwa", "serwist", "offline"],
    "mnm:admin": ["admin"],
    "herdr:termius": ["termius", "iphone", "mobil"],
}

# Known non-repo working directories that ARE module work — deterministic
# evidence, measured on the corpus (the voorinfra-api task dir alone carried
# 19 of the last 120 home chats).
DEFAULT_DIR_HINTS: dict[str, str] = {
    "~/projects/scrapling-workspace/tasks/voorinfra-api": "ccd:co-euronet",
    "~/scripts/voorinfra-drive-sync": "ccd:co-euronet",
    "~/projects/sor-dosyalari-cok-hassas-lasse-lar": "ccd:otdr",
    "~/projects/sor-dosyasi-duzeltme": "ccd:otdr",
}


class SpaceRules:
    def __init__(
        self,
        splits: list[dict],
        projects: list[dict],
        dir_hints: dict[str, str] | None = None,
        extra_lexicon: dict[str, list[str]] | None = None,
    ):
        self.splits = splits
        self.projects = projects
        self.by_key = {r["key"]: r for r in splits}
        self.extra_lexicon = extra_lexicon or {}
        # only hints whose target rule actually exists can seat a chat
        self.dir_hints = {
            d: k for d, k in (dir_hints or {}).items() if k in self.by_key
        }
        # repo dir -> rules in declaration order (first match wins)
        self.by_repo: dict[str, list[dict]] = defaultdict(list)
        for rule in splits:
            self.by_repo[rule["repo"]].append(rule)
        self.repo_dirs = sorted(
            {r["repo"] for r in splits}
            | {d for p in projects for d in p.get("repos", [])},
            key=len,
            reverse=True,
        )

    def owning_repo(self, directory: str) -> str | None:
        """Which known repo does this cwd belong to (checkout, subdir, or
        worktree-style sibling like <repo>-<branch>)?"""
        d = directory.rstrip("/")
        for repo in self.repo_dirs:
            if d == repo or d.startswith(repo + os.sep):
                return repo
            base = os.path.basename(repo)
            parent = os.path.dirname(repo)
            if os.path.dirname(d) == parent and os.path.basename(d).startswith(base + "-"):
                return repo
        return None

    def match_module(self, repo: str, candidates: list[str]) -> dict | None:
        """First-match a branch-like name against the repo's rules — the same
        order semantics the sidebar uses."""
        for rule in self.by_repo.get(repo, []):
            for name in candidates:
                for pat in rule["patterns"]:
                    if fnmatch.fnmatch(name, pat):
                        return rule
        return None


def _rule_tokens(rule: dict, extra_lexicon: dict[str, list[str]]) -> set[str]:
    """Human names only — label, key and the curated dictionary. Never glob
    patterns: measured live, "*sor-inventory*" leaking the token "sor" made
    one module swallow every chat naming a shared domain word."""
    tokens: set[str] = set()
    for source in [rule.get("label") or "", rule.get("key") or ""]:
        tokens |= set(_WORD_RE.findall(source.lower()))
    tokens |= {t.lower() for t in extra_lexicon.get(rule.get("key") or "", [])}
    return tokens - _STOPWORDS


def load_space_rules(
    config_path: str = DEFAULT_CONFIG,
    managed_path: str | None = None,
    home: str | None = None,
    dir_hints: dict[str, str] | None = None,
    extra_lexicon: dict[str, list[str]] | None = None,
) -> SpaceRules:
    home = home or os.path.expanduser("~")

    def _expand(p: str) -> str:
        if p.startswith("~"):
            return os.path.join(home, p[2:]) if len(p) > 2 else home
        return p

    def _load(path: str) -> dict:
        try:
            with open(path, "rb") as f:
                return tomllib.load(f)
        except (OSError, tomllib.TOMLDecodeError):
            return {}

    docs = [_load(config_path)]
    if managed_path:
        docs.append(_load(managed_path))

    splits: list[dict] = []
    projects: list[dict] = []
    for doc in docs:
        spaces = doc.get("spaces") or {}
        for raw in spaces.get("split") or []:
            if not raw.get("repo") or not raw.get("key"):
                continue
            splits.append(
                {
                    "key": raw["key"],
                    "label": raw.get("label") or raw["key"],
                    "repo": _expand(raw["repo"]),
                    "patterns": list(raw.get("match") or []),
                    "parent": raw.get("parent"),
                }
            )
        for raw in spaces.get("project") or []:
            projects.append(
                {
                    "key": raw.get("key") or "",
                    "name": raw.get("name") or "",
                    "repos": [_expand(r) for r in raw.get("repos") or []],
                }
            )
    hints = {
        _expand(d): k
        for d, k in (dir_hints if dir_hints is not None else DEFAULT_DIR_HINTS).items()
    }
    lexicon = extra_lexicon if extra_lexicon is not None else DEFAULT_EXTRA_LEXICON
    return SpaceRules(splits, projects, dir_hints=hints, extra_lexicon=lexicon)


# ---------------------------------------------------------------------------
# Classification (K3: deterministic first, lexicon second, open otherwise)
# ---------------------------------------------------------------------------

_REPO_DOMINANCE_MIN_WEIGHT = 5


def _branch_candidates(facts: dict, rules: SpaceRules, repo: str) -> list[str]:
    names: list[str] = []
    for c in facts.get("commits", []):
        if c.get("repo") == repo and c.get("branch"):
            names.append(c["branch"])
    base = os.path.basename(repo)
    parent = os.path.dirname(repo)
    for run in facts.get("cwd_runs", []):
        d = run["dir"].rstrip("/")
        if os.path.dirname(d) == parent and os.path.basename(d).startswith(base + "-"):
            names.append(os.path.basename(d)[len(base) + 1 :])
    return names


def _lexicon_hits(text: str, rule: dict, extra_lexicon: dict[str, list[str]]) -> int:
    words = set(_WORD_RE.findall(text.lower()))
    return len(words & _rule_tokens(rule, extra_lexicon))


def classify(facts: dict, rules: SpaceRules) -> list[dict]:
    """Return seat candidates sorted by evidence weight; [] means stay open.

    Layer order (K3): deterministic first — the chat actually worked in a
    repo checkout (cwd/commit evidence) or in a known hinted directory —
    then the lexicon, then open. A wrong seat is worse than no seat.
    """
    home = os.path.expanduser("~")
    repo_weight: dict[str, int] = defaultdict(int)
    hint_weight: dict[str, int] = defaultdict(int)
    for run in facts.get("cwd_runs", []):
        d = run["dir"]
        if d.rstrip("/") == home:
            continue
        for hint_dir, key in rules.dir_hints.items():
            if d == hint_dir or d.startswith(hint_dir + os.sep):
                hint_weight[key] += run["weight"]
                break
        else:
            repo = rules.owning_repo(d)
            if repo:
                repo_weight[repo] += run["weight"]
    for c in facts.get("commits", []):
        repo = rules.owning_repo(c.get("repo") or "")
        if repo:
            repo_weight[repo] += 20

    text = " ".join([facts.get("title") or "", facts.get("last_prompt") or ""])
    weighted: list[tuple[int, dict]] = []

    for key, weight in hint_weight.items():
        if weight < _REPO_DOMINANCE_MIN_WEIGHT:
            continue
        rule = rules.by_key[key]
        weighted.append(
            (
                weight,
                {
                    "key": MODULE_KEY_PREFIX + key,
                    "dir": rule["repo"],
                    "conf": 0.85,
                    "basis": "dir-hint",
                    "label": rule["label"],
                },
            )
        )

    for repo, weight in repo_weight.items():
        if weight < _REPO_DOMINANCE_MIN_WEIGHT:
            continue
        candidates = _branch_candidates(facts, rules, repo)
        rule = rules.match_module(repo, candidates) if candidates else None
        if rule:
            conf = 0.9 if any(c.get("sha") for c in facts.get("commits", [])) else 0.85
            weighted.append(
                (
                    weight,
                    {
                        "key": MODULE_KEY_PREFIX + rule["key"],
                        "dir": rule["repo"],
                        "conf": conf,
                        "basis": "repo+branch",
                        "label": rule["label"],
                    },
                )
            )
            continue
        # lexicon restricted to this repo's own rules
        best, best_hits = None, 0
        for r in rules.by_repo.get(repo, []):
            hits = _lexicon_hits(text, r, rules.extra_lexicon)
            if hits > best_hits:
                best, best_hits = r, hits
        if best:
            weighted.append(
                (
                    weight,
                    {
                        "key": MODULE_KEY_PREFIX + best["key"],
                        "dir": best["repo"],
                        "conf": 0.7,
                        "basis": "repo+lexicon",
                        "label": best["label"],
                    },
                )
            )
        else:
            weighted.append(
                (
                    weight,
                    {
                        "key": repo,
                        "dir": repo,
                        "conf": 0.75,
                        "basis": "repo",
                        "label": os.path.basename(repo),
                    },
                )
            )

    if weighted:
        weighted.sort(key=lambda wv: -wv[0])
        seats, seen = [], set()
        for _, seat in weighted:
            if seat["key"] in seen:
                continue
            seen.add(seat["key"])
            seats.append(seat)
        return seats[:3]

    # Pure-home chat: lexicon across every rule.
    best, best_hits = None, 0
    for rule in rules.splits:
        hits = _lexicon_hits(text, rule, rules.extra_lexicon)
        if hits > best_hits:
            best, best_hits = rule, hits
    if best and best_hits >= 1:
        # One shared word is a hunch (0.60 — below the apply threshold);
        # two independent hits are a claim (0.68). The applied plan defaults
        # to min-conf 0.65, so single-word seats surface in reports but are
        # never written without an explicit lower threshold.
        conf = 0.6 if best_hits == 1 else min(0.63 + 0.05 * (best_hits - 1), 0.75)
        return [
            {
                "key": MODULE_KEY_PREFIX + best["key"],
                "dir": best["repo"],
                "conf": conf,
                "basis": "lexicon",
                "label": best["label"],
            }
        ]
    return []


# ---------------------------------------------------------------------------
# Plan
# ---------------------------------------------------------------------------


def build_plan(
    sessions: list[dict],
    min_conf: float = 0.6,
    per_target_cap: int = 150,
) -> dict:
    """Single-ownership plan: one (top) seat per session, capped per target
    keeping the newest — the ledger drops the OLDEST past its own cap, so we
    never feed it more than it can keep (K2c)."""
    candidates = []
    for s in sessions:
        seats = s.get("seats") or []
        if not seats:
            continue
        top = seats[0]
        if top["conf"] < min_conf:
            continue
        candidates.append(
            {
                "session_id": s["session_id"],
                "target_key": top["key"],
                "target_dir": top["dir"],
                "conf": top["conf"],
                "basis": top["basis"],
                "label": top.get("label") or "",
                "last_substantive_ts": s.get("last_substantive_ts") or "",
            }
        )
    by_target: dict[str, list[dict]] = defaultdict(list)
    for c in candidates:
        by_target[c["target_key"]].append(c)
    seats = []
    for target, rows in by_target.items():
        rows.sort(key=lambda r: r["last_substantive_ts"], reverse=True)
        seats.extend(rows[:per_target_cap])
    seats.sort(key=lambda r: (r["target_key"], r["last_substantive_ts"]), reverse=True)
    return {
        "version": 1,
        "generated_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "source": "seat-plan",
        "seats": seats,
    }


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------


def _cmd_scan(args) -> int:
    cache = FactsCache(args.cache_db)
    slugs = args.slugs or sorted(os.listdir(args.projects_dir))
    total = {"parsed": 0, "cached": 0, "removed": 0}
    t0 = time.time()
    for slug in slugs:
        d = os.path.join(args.projects_dir, slug)
        if not os.path.isdir(d):
            continue
        r = cache.scan_dir(d)
        for k in total:
            total[k] += r[k]
    cache.close()
    print(
        f"scan: {total['parsed']} parsed · {total['cached']} cached · "
        f"{total['removed']} removed · {time.time() - t0:.1f}s"
    )
    return 0


def _classified_sessions(args) -> list[dict]:
    cache = FactsCache(args.cache_db)
    rules = load_space_rules(args.config, args.managed)
    sessions = []
    for facts in cache.all_facts(args.slug):
        facts["seats"] = classify(facts, rules)
        sessions.append(facts)
    cache.close()
    return sessions


def _cmd_plan(args) -> int:
    sessions = _classified_sessions(args)
    plan = build_plan(sessions, min_conf=args.min_conf, per_target_cap=args.cap)
    seated = plan["seats"]
    open_n = len(sessions) - len({s["session_id"] for s in seated})
    by_target = defaultdict(int)
    for s in seated:
        by_target[s["target_key"]] += 1
    print(f"plan: {len(seated)} seated · {open_n} open")
    for target, n in sorted(by_target.items(), key=lambda kv: -kv[1]):
        print(f"  {n:5d}  {target}")
    if args.dry_run:
        print("(dry-run: hiçbir dosya yazılmadı)")
        return 0
    with open(args.out, "w") as f:
        json.dump(plan, f, indent=1)
    print(f"yazıldı: {args.out}")
    return 0


def _cmd_worklog(args) -> int:
    cache = FactsCache(args.cache_db)
    chats = {}
    for facts in cache.all_facts(args.slug):
        commits = [c for c in facts.get("commits", []) if c.get("sha") or args.include_attempted]
        if commits:
            chats[facts["session_id"]] = commits
    cache.close()
    out = {"version": 1, "generated_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()), "chats": chats}
    if args.dry_run:
        print(f"worklog: {len(chats)} chat, {sum(len(v) for v in chats.values())} commit (dry-run)")
        return 0
    with open(args.out, "w") as f:
        json.dump(out, f, indent=1)
    print(f"worklog: {len(chats)} chat → {args.out}")
    return 0


def _cmd_report(args) -> int:
    sessions = _classified_sessions(args)
    n_multi = sum(1 for s in sessions if len({r['dir'] for r in s.get('cwd_runs', [])}) > 1)
    n_commit = sum(1 for s in sessions if s.get("commits"))
    n_seated = sum(1 for s in sessions if s.get("seats"))
    print(
        f"{len(sessions)} oturum · çok-cwd {n_multi} · commit'li {n_commit} · "
        f"sınıflandırılabilir {n_seated} · open {len(sessions) - n_seated}"
    )
    return 0


def main(argv=None) -> int:
    p = argparse.ArgumentParser(prog="chat_context", description=__doc__)
    p.add_argument("--cache-db", default=DEFAULT_CACHE_DB)
    p.add_argument("--projects-dir", default=DEFAULT_PROJECTS_DIR)
    p.add_argument("--config", default=DEFAULT_CONFIG)
    p.add_argument("--managed", default=DEFAULT_MANAGED)
    p.add_argument("--slug", default=None, help="tek slug ile sınırla (plan/worklog/report)")
    sub = p.add_subparsers(dest="cmd", required=True)

    sp = sub.add_parser("scan", help="transcriptleri artımlı tara")
    sp.add_argument("--slugs", nargs="*", default=None)

    pp = sub.add_parser("plan", help="seat-plan üret")
    pp.add_argument("--min-conf", type=float, default=0.65)
    pp.add_argument("--cap", type=int, default=150)
    pp.add_argument("--out", default=os.path.expanduser("~/.cache/herdr-chat-context/seat-plan.json"))
    pp.add_argument("--dry-run", action="store_true")

    wp = sub.add_parser("worklog", help="chat-worklog üret")
    wp.add_argument("--out", default=os.path.expanduser("~/.config/herdr/chat-worklog.json"))
    wp.add_argument("--include-attempted", action="store_true")
    wp.add_argument("--dry-run", action="store_true")

    sub.add_parser("report", help="özet rapor")

    args = p.parse_args(argv)
    return {"scan": _cmd_scan, "plan": _cmd_plan, "worklog": _cmd_worklog, "report": _cmd_report}[
        args.cmd
    ](args)


if __name__ == "__main__":
    sys.exit(main())
