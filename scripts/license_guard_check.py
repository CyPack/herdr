"""Fail the build if this fork stops being AGPL-3.0-or-later.

Why this exists
---------------
Upstream relicensed to Apache-2.0 in `cd5ea1be`. The fork had never edited
`LICENSE`, `Cargo.toml`'s licence field, `nix/package.nix` or the README
licence section since the fork point, so in a three-way merge those files
matched the merge base on our side and upstream's version was taken silently:
no conflict, no warning, no failing test. The fork went from AGPL to Apache in
one `git merge` and only a manual read caught it.

Staying AGPL is deliberate. It keeps the fork's own work under copyleft, and
it costs nothing on the way in, because Apache-2.0 material may be
incorporated into an AGPL-licensed work. This guard makes that decision
survive every future sync.

`LICENSE-APACHE` and `NOTICE` are checked too: the upstream portions from
`cd5ea1be` onward carry Apache terms, and dropping their licence copy or the
attribution record would break what the fork owes them.

Run directly::

    python3 -m scripts.license_guard_check
"""

from __future__ import annotations

import sys
from pathlib import Path

AGPL_SPDX = "AGPL-3.0-or-later"
AGPL_MARKER = "GNU Affero General Public License"
APACHE_MARKER = "Apache License"
NIX_AGPL = "agpl3Plus"

REQUIRED_ATTRIBUTION = (
    ("LICENSE-APACHE", "upstream's Apache-2.0 text (Apache-2.0 section 4a)"),
    ("NOTICE", "the derivation and modification record (Apache-2.0 section 4b)"),
)


def _read(path: Path) -> str:
    return path.read_text(encoding="utf-8", errors="replace")


def check(root: Path) -> list[str]:
    """Return every licensing violation found under ``root`` (empty == healthy)."""
    errors: list[str] = []

    license_path = root / "LICENSE"
    if not license_path.is_file():
        errors.append("LICENSE is missing")
    else:
        body = _read(license_path)
        if AGPL_MARKER not in body:
            errors.append(
                "LICENSE no longer contains the AGPL text; an upstream sync may have "
                "replaced it with Apache-2.0"
            )
        elif body.lstrip().startswith(APACHE_MARKER):
            errors.append("LICENSE leads with the Apache text instead of the fork's AGPL grant")

    manifest = root / "Cargo.toml"
    if not manifest.is_file():
        errors.append("Cargo.toml is missing")
    else:
        declared = [
            line.split("=", 1)[1].strip().strip('"')
            for line in _read(manifest).splitlines()
            if line.strip().startswith("license")
            and "=" in line
            and not line.strip().startswith("license-file")
        ]
        if not declared:
            errors.append("Cargo.toml declares no license field")
        elif declared[0] != AGPL_SPDX:
            errors.append(
                f'Cargo.toml declares license = "{declared[0]}"; expected "{AGPL_SPDX}"'
            )

    nix_path = root / "nix" / "package.nix"
    if not nix_path.is_file():
        errors.append("nix/package.nix is missing")
    elif NIX_AGPL not in _read(nix_path):
        errors.append(
            f"nix/package.nix no longer declares {NIX_AGPL}; the Nix package would "
            "advertise the wrong license"
        )

    readme = root / "README.md"
    if not readme.is_file():
        errors.append("README.md is missing")
    else:
        body = _read(readme)
        if "dual-licensed" not in body and AGPL_SPDX not in body:
            errors.append(
                "README.md no longer states the fork's AGPL licensing; readers would be "
                "told the wrong terms"
            )

    for name, why in REQUIRED_ATTRIBUTION:
        if not (root / name).is_file():
            errors.append(f"{name} is missing; it carries {why}")

    return errors


def main(argv: list[str]) -> int:
    root = Path(argv[1]) if len(argv) > 1 else Path(__file__).resolve().parents[1]
    errors = check(root)
    if errors:
        print(f"license guard: {len(errors)} problem(s)")
        for error in errors:
            print(f"  - {error}")
        return 1
    print(f"license guard: OK — fork is {AGPL_SPDX} with upstream attribution intact")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
