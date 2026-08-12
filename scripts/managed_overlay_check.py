"""Check that every collection the managed spaces overlay can carry is merged.

`spaces.managed.toml` is written by `herdr space promote`, `herdr space move
--new-group` and the sidebar's two-click module road. It is parsed into a
`SpacesConfig` and then merged, field by field, into the live config by
`merge_managed_spaces_str`.

The merge is written by hand, so it can fall behind the model. That is not a
hypothetical: `[[spaces.node]]` was added to `SpacesConfig` for the N-level
tree and the merge was never extended, so every module a user created was
parsed correctly, held in a local variable, and dropped when the function
returned. No diagnostic fired, because nothing was invalid — the value was
simply never read. The module existed on disk and nowhere else.

A validator cannot catch that class: the file is valid and `herdr config
check` is right to say so. What catches it is a structural gate — this one.
Every `Vec` field of `SpacesConfig` must be extended by the merge, or be
listed in EXEMPT_FIELDS with the reason it is not a collection to append.

Exit code 0 when the merge covers the model, 1 when it has fallen behind.
"""

from __future__ import annotations

import argparse
import re
import sys
from dataclasses import dataclass
from pathlib import Path


DEFAULT_MODEL = Path("src/config/model.rs")
DEFAULT_IO = Path("src/config/io.rs")

MODEL_STRUCT = "SpacesConfig"
MERGE_FN = "merge_managed_spaces_str"

# Fields that are deliberately not appended, with the reason. A field belongs
# here only when appending it would be wrong, never when appending it was
# merely forgotten.
EXEMPT_FIELDS = {
    # `[spaces.icons]` is one table of defaults, not a list of entries. There
    # is no meaning to "append another icons table"; letting the overlay
    # override it would be a policy decision about who owns the row glyphs,
    # and nobody has made that decision. Until someone does, the overlay does
    # not carry icons.
    "icons": "a single settings table, not a collection to append",
}


@dataclass(frozen=True)
class Field:
    """One field of the model struct, as written in the source."""

    name: str
    type_name: str

    @property
    def is_collection(self) -> bool:
        return self.type_name.startswith("Vec<")


class CheckError(Exception):
    """The source could not be read the way this check expects."""


def _struct_body(source: str, struct_name: str) -> str:
    """The brace-matched body of `pub struct <struct_name> { ... }`."""

    match = re.search(rf"\bstruct\s+{re.escape(struct_name)}\s*\{{", source)
    if match is None:
        raise CheckError(f"struct {struct_name} not found")
    start = match.end()
    depth = 1
    for index in range(start, len(source)):
        char = source[index]
        if char == "{":
            depth += 1
        elif char == "}":
            depth -= 1
            if depth == 0:
                return source[start:index]
    raise CheckError(f"struct {struct_name} is not brace-balanced")


def _fn_body(source: str, fn_name: str) -> str:
    """The brace-matched body of `fn <fn_name>(...) ... { ... }`."""

    match = re.search(rf"\bfn\s+{re.escape(fn_name)}\b", source)
    if match is None:
        raise CheckError(f"fn {fn_name} not found")
    brace = source.find("{", match.end())
    if brace == -1:
        raise CheckError(f"fn {fn_name} has no body")
    depth = 1
    for index in range(brace + 1, len(source)):
        char = source[index]
        if char == "{":
            depth += 1
        elif char == "}":
            depth -= 1
            if depth == 0:
                return source[brace + 1 : index]
    raise CheckError(f"fn {fn_name} is not brace-balanced")


def model_fields(source: str, struct_name: str = MODEL_STRUCT) -> list[Field]:
    """Every `pub <name>: <type>,` field of the struct, in source order.

    Doc comments and attributes are skipped rather than parsed: the check
    only needs the name and enough of the type to tell a list from a table.
    """

    body = _struct_body(source, struct_name)
    fields: list[Field] = []
    for line in body.splitlines():
        stripped = line.strip()
        if not stripped or stripped.startswith(("///", "//", "#[")):
            continue
        match = re.match(r"pub\s+(\w+)\s*:\s*(.+?),\s*$", stripped)
        if match is None:
            continue
        fields.append(Field(name=match.group(1), type_name=match.group(2).strip()))
    if not fields:
        raise CheckError(f"struct {struct_name} parsed to zero fields")
    return fields


def merged_fields(source: str, fn_name: str = MERGE_FN) -> set[str]:
    """Field names the merge appends from the overlay into the live config.

    Both halves are required: extending `config.spaces.x` from
    `managed.spaces.y` would type-check for same-typed collections and
    silently merge the wrong list, so the check reads the pair.
    """

    body = _fn_body(source, fn_name)
    pattern = re.compile(
        r"config\.spaces\.(\w+)\s*\.\s*extend\(\s*managed\.spaces\.(\w+)\s*\)"
    )
    names: set[str] = set()
    for target, origin in pattern.findall(body):
        if target != origin:
            raise CheckError(
                f"{fn_name} extends config.spaces.{target} from "
                f"managed.spaces.{origin}: the overlay would merge into the "
                f"wrong collection"
            )
        names.add(target)
    return names


def check(model_path: Path, io_path: Path) -> list[str]:
    """Problems found, empty when the merge covers the model."""

    fields = model_fields(model_path.read_text(encoding="utf-8"))
    merged = merged_fields(io_path.read_text(encoding="utf-8"))
    problems: list[str] = []

    for field in fields:
        if field.name in merged:
            if field.name in EXEMPT_FIELDS:
                problems.append(
                    f"{field.name} is merged but also listed as exempt: "
                    f"remove it from EXEMPT_FIELDS or stop merging it"
                )
            continue
        if field.name in EXEMPT_FIELDS:
            continue
        if field.is_collection:
            problems.append(
                f"{MODEL_STRUCT}.{field.name}: {field.type_name} is never "
                f"merged from the managed overlay. Add "
                f"`config.spaces.{field.name}.extend(managed.spaces."
                f"{field.name});` to {MERGE_FN}, or list it in EXEMPT_FIELDS "
                f"with the reason it must not be appended."
            )
        else:
            problems.append(
                f"{MODEL_STRUCT}.{field.name}: {field.type_name} is neither "
                f"merged nor exempt. Decide which, and say why in "
                f"EXEMPT_FIELDS if it is exempt."
            )

    known = {field.name for field in fields}
    for name in sorted(merged - known):
        problems.append(
            f"{MERGE_FN} merges spaces.{name}, which {MODEL_STRUCT} no longer "
            f"has: the merge is ahead of the model"
        )
    for name in sorted(set(EXEMPT_FIELDS) - known):
        problems.append(
            f"EXEMPT_FIELDS names spaces.{name}, which {MODEL_STRUCT} no "
            f"longer has: drop the stale exemption"
        )

    return problems


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--model", type=Path, default=DEFAULT_MODEL)
    parser.add_argument("--io", type=Path, default=DEFAULT_IO)
    args = parser.parse_args(argv)

    try:
        problems = check(args.model, args.io)
    except CheckError as err:
        print(f"managed overlay check could not run: {err}", file=sys.stderr)
        return 1

    if problems:
        print(
            "the managed spaces overlay has fallen behind the config model:",
            file=sys.stderr,
        )
        for problem in problems:
            print(f"  - {problem}", file=sys.stderr)
        return 1

    print("managed overlay check: every spaces collection is merged")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
