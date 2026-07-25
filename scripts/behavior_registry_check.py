"""Verify the fork's behavior registry against the source tree.

Why this exists
---------------
This fork carries features upstream does not have. An upstream sync is a
three-way merge, so when only upstream touched a region its version wins with
no conflict and no warning. That is how a fork behavior disappears quietly:
the merge compiles, the suite is green, and nobody notices the test that used
to pin the behavior is gone.

The registry closes that hole. Every fork behavior gets an id, a plain
description of what breaks if it is lost, and the names of the tests that pin
it. This checker then answers one question mechanically: is every registered
behavior still pinned by a test that actually exists?

Run directly for a summary::

    python3 -m scripts.behavior_registry_check
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

# Docs that live beside the registries but are not registries themselves.
RESERVED = {"README.md", "UPSTREAM-SYNC-PROTOCOL.md", "UNDOCUMENTED.md"}

SOURCE_DIRS = ("src", "tests")
# Behaviors are pinned by Rust tests and by Playwright visual specs; a scan
# limited to Rust would call every visually-pinned behavior undocumented.
SOURCE_SUFFIXES = {".rs", ".ts"}

# `TP-FLF-MOUSE-01`, `TP-C4.1`, and the `TP-DCLICK-01/02/04` run form all
# appear in the tree today; the pattern has to accept every one of them.
_MARKER = re.compile(
    r"\bTP-[A-Z][A-Z0-9]*(?:\.[0-9]+)?(?:-[A-Za-z0-9]+)*(?:/[A-Za-z0-9]+)*"
)
_TABLE_ROW = re.compile(r"^\|(?P<cells>.+)\|\s*$")
# Rust test names are identifiers; Playwright test names are sentences.
_BACKTICKED = re.compile(r"`([A-Za-z0-9_][A-Za-z0-9_ -]*)`")
_RUST_FN = re.compile(r"\bfn\s+([A-Za-z0-9_]+)")
_SPEC_TEST = re.compile(r"\btest\(\s*['\"]([A-Za-z0-9_ -]+)['\"]")


def expand_marker_ids(text: str) -> set[str]:
    """Return every behavior id a chunk of text refers to.

    A run such as ``TP-DCLICK-01/02/04`` names three sibling behaviors; each
    trailing element replaces the last segment of the base id.
    """
    ids: set[str] = set()
    for raw in _MARKER.findall(text):
        base, *rest = raw.split("/")
        ids.add(base)
        head = base.rsplit("-", 1)[0] if "-" in base else base
        for tail in rest:
            ids.add(f"{head}-{tail}")
    return ids


def _iter_source_files(root: Path):
    for directory in SOURCE_DIRS:
        base = root / directory
        if not base.is_dir():
            continue
        for path in sorted(base.rglob("*")):
            if path.is_file() and path.suffix in SOURCE_SUFFIXES:
                yield path


def _read(path: Path) -> str:
    return path.read_text(encoding="utf-8", errors="replace")


def _parse_registries(root: Path) -> tuple[dict[str, dict], list[str]]:
    """Parse ``behaviors/*.md`` into ``{id: entry}``, reporting duplicates."""
    entries: dict[str, dict] = {}
    errors: list[str] = []
    behaviors = root / "behaviors"
    if not behaviors.is_dir():
        return entries, errors

    for path in sorted(behaviors.glob("*.md")):
        if path.name in RESERVED:
            continue
        for line in _read(path).splitlines():
            match = _TABLE_ROW.match(line)
            if not match:
                continue
            cells = [cell.strip() for cell in match.group("cells").split("|")]
            if not cells:
                continue
            entry_id = cells[0].strip("`")
            if not entry_id.startswith("TP-"):
                continue
            if entry_id in entries:
                errors.append(
                    f"{entry_id}: duplicate registry entry in "
                    f"{entries[entry_id]['file']} and {path.name}"
                )
                continue
            tests = _BACKTICKED.findall(cells[3]) if len(cells) > 3 else []
            entries[entry_id] = {"file": path.name, "tests": tests}
    return entries, errors


def _parse_ledger(root: Path) -> dict[str, list[str]]:
    """Parse the debt ledger into ``{id: [test names]}``.

    Ledger entries carry their test bindings so an undocumented behavior is
    still guarded against losing its test; only the prose description is owed.
    """
    path = root / "behaviors" / "UNDOCUMENTED.md"
    if not path.is_file():
        return {}
    ledger: dict[str, list[str]] = {}
    for line in _read(path).splitlines():
        match = _TABLE_ROW.match(line)
        if not match:
            continue
        cells = [cell.strip() for cell in match.group("cells").split("|")]
        entry_id = cells[0].strip("`")
        if not entry_id.startswith("TP-"):
            continue
        ledger[entry_id] = _BACKTICKED.findall(cells[1]) if len(cells) > 1 else []
    return ledger


def check(root: Path) -> list[str]:
    """Return every registry violation found under ``root`` (empty == healthy)."""
    entries, errors = _parse_registries(root)
    ledger = _parse_ledger(root)

    source_markers: set[str] = set()
    defined_fns: set[str] = set()
    for path in _iter_source_files(root):
        text = _read(path)
        source_markers |= expand_marker_ids(text)
        defined_fns |= set(_RUST_FN.findall(text))
        defined_fns |= set(_SPEC_TEST.findall(text))

    for entry_id in sorted(entries):
        entry = entries[entry_id]
        # C1: a registered behavior whose test no longer exists is exactly the
        # silent loss this registry was built to catch.
        if not entry["tests"]:
            errors.append(f"{entry_id} ({entry['file']}): no test named in the registry row")
        for test in entry["tests"]:
            if test not in defined_fns:
                errors.append(
                    f"{entry_id} ({entry['file']}): test `{test}` no longer exists in the tree"
                )
        # C2: the marker is how a reader finds the behavior from the code side.
        if entry_id not in source_markers:
            errors.append(
                f"{entry_id} ({entry['file']}): no source marker left; "
                "the behavior may have been removed"
            )

    # C1 again, for the ledger: description debt must not become coverage debt.
    for entry_id in sorted(ledger):
        if entry_id in entries:
            continue
        tests = ledger[entry_id]
        if not tests:
            errors.append(f"{entry_id} (ledger): no test named for this behavior")
        for test in tests:
            if test not in defined_fns:
                errors.append(
                    f"{entry_id} (ledger): test `{test}` no longer exists in the tree"
                )

    # C3: new behaviors must be documented, not smuggled in. A bare family
    # name that only introduces its own numbered children is a pointer, not a
    # behavior, so it needs no test of its own.
    known = set(entries) | set(ledger)
    unknown = source_markers - known
    families = {m for m in unknown if any(k.startswith(f"{m}-") for k in known)}
    for marker in sorted(unknown - families):
        errors.append(
            f"{marker}: marked in the source but absent from behaviors/ and the ledger"
        )

    # C4: the ledger is a debt list, so it has to shrink and stay truthful.
    for marker in sorted(set(ledger) & set(entries)):
        errors.append(f"{marker}: now documented, so remove it from the UNDOCUMENTED ledger")
    for marker in sorted(set(ledger) - source_markers):
        errors.append(f"{marker}: listed in the UNDOCUMENTED ledger but no source marker uses it")

    return errors


def collect_source_bindings(root: Path) -> dict[str, set[str]]:
    """Map every source marker to the test functions that follow it.

    A marker sits in the comment directly above the test it pins, which is the
    convention already used throughout the tree.
    """
    bindings: dict[str, set[str]] = {}
    fn_start = re.compile(r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+([A-Za-z0-9_]+)")
    for path in _iter_source_files(root):
        lines = _read(path).splitlines()
        for index, line in enumerate(lines):
            ids = expand_marker_ids(line)
            if not ids:
                continue
            name = None
            # The convention puts the marker in the comment above its test.
            for candidate in lines[index : index + 12]:
                match = fn_start.match(candidate) or _SPEC_TEST.search(candidate)
                if match:
                    name = match.group(1)
                    break
            # A marker written inside a test body still belongs to that test.
            if name is None:
                for candidate in reversed(lines[max(0, index - 200) : index]):
                    match = fn_start.match(candidate) or _SPEC_TEST.search(candidate)
                    if match:
                        name = match.group(1)
                        break
            if name is None:
                continue
            for entry_id in ids:
                bindings.setdefault(entry_id, set()).add(name)
    return bindings


def write_ledger(root: Path) -> int:
    """Regenerate the debt ledger from the source, keeping documented ids out."""
    entries, _ = _parse_registries(root)
    bindings = collect_source_bindings(root)
    rows = [
        f"| {entry_id} | " + ", ".join(f"`{t}`" for t in sorted(tests)) + " |"
        for entry_id, tests in sorted(bindings.items())
        if entry_id not in entries
    ]
    header = (
        "# Undocumented behavior ledger\n"
        "\n"
        "> Generated by `python3 -m scripts.behavior_registry_check --write-ledger`.\n"
        "> Do not hand-edit rows; move an entry into a `behaviors/<feature>.md`\n"
        "> registry instead, which is how this list is meant to shrink.\n"
        "\n"
        "Every row is a real fork behavior whose test is already guarded: if the\n"
        "test disappears, `just check` fails. What each row still owes is the\n"
        "prose — what the behavior is and what breaks if it is lost.\n"
        "\n"
        "| ID | Verified by |\n"
        "|---|---|\n"
    )
    path = root / "behaviors" / "UNDOCUMENTED.md"
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(header + "\n".join(rows) + "\n", encoding="utf-8")
    return len(rows)


def main(argv: list[str]) -> int:
    args = [a for a in argv[1:] if not a.startswith("--")]
    root = Path(args[0]) if args else Path(__file__).resolve().parents[1]
    if "--write-ledger" in argv:
        count = write_ledger(root)
        print(f"behavior registry: ledger rewritten with {count} undocumented behavior(s)")
        return 0
    errors = check(root)
    entries, _ = _parse_registries(root)
    ledger = _parse_ledger(root)
    if errors:
        print(f"behavior registry: {len(errors)} problem(s)")
        for error in errors:
            print(f"  - {error}")
        return 1
    print(
        f"behavior registry: OK — {len(entries)} documented, "
        f"{len(ledger)} awaiting documentation"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
