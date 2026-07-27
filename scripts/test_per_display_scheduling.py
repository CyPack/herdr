"""Scheduled work that reads a display's state must run in that display's view.

Per-display surfaces are registers: outside a display's window they hold the
session's own view, which once a second display attaches is nobody's browser.
A scheduled task that reads one from outside the loop therefore takes a
request, matches nothing, and drops the action without a word — on every
display, not just the extra ones.

That failure is invisible in the suite because each half is individually
correct: the field is per-display, the consumer compiles, every unit test
passes. Only the pairing is wrong. This test pins the pairing.
"""

import re
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SRC = ROOT / "src"

# Consumers that are deliberately outside the loop, with the reason. Adding a
# name here is a claim that the function reads no per-display surface.
OUTSIDE_LOOP_ALLOWED = {
    # One channel, one operation in flight: draining per display would let the
    # first display served eat progress meant for the session.
    "sync_file_operation_worker",
}


def rust_sources():
    return {p: p.read_text(encoding="utf-8") for p in SRC.rglob("*.rs")}


def bundled_fields(state_src):
    """Field names declared inside client_surfaces! — the per-display registers."""
    start = state_src.index("client_surfaces! {")
    end = state_src.index("\n}\n", state_src.index("    ephemeral {", start))
    return set(re.findall(r"^\s{4}([a-z_][a-z0-9_]*)\s*:", state_src[start:end], re.M))


def function_body(sources, name):
    """Brace-matched body of the first `fn <name>` found."""
    for text in sources.values():
        match = re.search(r"fn " + re.escape(name) + r"\s*\([^)]*\)[^{]*\{", text)
        if not match:
            continue
        depth = 0
        start = match.end() - 1
        for index in range(start, len(text)):
            if text[index] == "{":
                depth += 1
            elif text[index] == "}":
                depth -= 1
                if depth == 0:
                    return text[start : index + 1]
        return text[start:]
    return None


def scheduled_calls(runtime_src):
    """(outside, inside) scheduled task names around the per-display loop."""
    body = runtime_src.split("fn handle_scheduled_tasks", 1)[1]
    before, rest = body.split("for_each_display(|app|", 1)
    inside_block = rest.split("});", 1)[0]
    outside = re.findall(r"self\.(sync_\w+)\(", before)
    inside = re.findall(r"app\.(sync_\w+)\(", inside_block)
    return outside, inside


def reads_per_display_state(sources, name, fields):
    """Fields the function reads, following one level of its own helpers."""
    body = function_body(sources, name)
    if body is None:
        return set()
    helpers = set(re.findall(r"self\.(consume_\w+|start_\w+|open_\w+|apply_\w+)\(", body))
    text = body + "".join(function_body(sources, h) or "" for h in helpers)
    return {f for f in re.findall(r"state\.([a-z_][a-z0-9_]*)", text) if f in fields}


class PerDisplaySchedulingTest(unittest.TestCase):
    def test_outside_the_loop_reads_no_per_display_state(self):
        sources = rust_sources()
        fields = bundled_fields(sources[SRC / "app" / "state.rs"])
        self.assertTrue(fields, "no per-display fields found; the parser is stale")

        outside, _ = scheduled_calls(sources[SRC / "app" / "runtime.rs"])
        offenders = {}
        for name in dict.fromkeys(outside):
            if name in OUTSIDE_LOOP_ALLOWED:
                continue
            hits = reads_per_display_state(sources, name, fields)
            if hits:
                offenders[name] = sorted(hits)

        self.assertEqual(
            offenders,
            {},
            "these scheduled tasks read a display's own state but run outside "
            "every display's view, so once a second display attaches they act "
            "on nobody's state and silently do nothing: "
            + "; ".join(f"{k} reads {', '.join(v)}" for k, v in offenders.items())
            + ". Move the call inside for_each_display, or split the part that "
            "reads per-display state out of it.",
        )

    def test_both_schedulers_drive_the_same_per_display_work(self):
        """The monolithic loop and the headless server must not drift apart."""
        sources = rust_sources()
        _, runtime_inside = scheduled_calls(sources[SRC / "app" / "runtime.rs"])
        headless = sources[SRC / "server" / "headless.rs"]
        block = headless.split("for_each_display(|app|", 1)[1].split("});", 1)[0]
        headless_inside = re.findall(r"app\.(sync_\w+)\(", block)
        self.assertEqual(
            sorted(set(runtime_inside)),
            sorted(set(headless_inside)),
            "the two schedulers drive different per-display work; a fix applied "
            "to one path silently does not apply to the other",
        )


if __name__ == "__main__":
    unittest.main()
