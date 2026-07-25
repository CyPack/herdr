# Behavior registry

This fork carries features upstream does not have. This directory is where
those behaviors are written down and mechanically guarded.

## The problem it solves

An upstream sync is a three-way merge. When a region matches the merge base on
our side and upstream changed it, upstream's version is taken **silently** — no
conflict, no warning. That is the exact shape of a lost fork behavior: the
merge compiles, the suite is green, and the test that used to pin the behavior
is simply gone.

This is not hypothetical. During the 2026-07-25 sync:

- `LICENSE`, `Cargo.toml`, `nix/package.nix` and the README licence section
  were replaced with Apache-2.0 without a single conflict. Nothing in the suite
  noticed; a human read caught it.
- A key-handling regression that broke release forwarding was caught only
  because *upstream* happened to have a test for it. Had it been one of ours,
  it would have shipped.

## The one rule

> **A fork behavior that no test names is a behavior the next merge can delete
> without telling anyone.**

So: every behavior gets an id, and every id names the tests that pin it.
`just check` fails if a named test stops existing.

## Layout

| File | What it holds |
|---|---|
| `shared-surfaces.md` | Behaviors living in files upstream also owns — **read this one first during a sync** |
| `<feature>.md` | Documented behaviors: id, what it does, what breaks if lost, which tests pin it |
| `UNDOCUMENTED.md` | Generated ledger — real behaviors with real test bindings, carrying no prose |
| `UPSTREAM-SYNC-PROTOCOL.md` | How to pull upstream without losing any of it |

## What the ledger is, and is not

The ledger is **not** a backlog to burn down. Every row in it is already guarded by
C1: if its test disappears in a sync, the build fails. What a row lacks is prose,
and prose is only read in one situation — a human deciding whether a merge
conflict resolution is acceptable.

That situation requires upstream to have a version of the file. It cannot arise in
a file upstream has never seen, so the split is deliberate:

| Where the behavior lives | Documented? | Why |
|---|---|---|
| A file upstream also owns | Yes — `shared-surfaces.md` | Silent three-way take is possible here |
| A fork-only file | No — ledger is enough | Upstream has no version to take; the risk is a loud compile error, not a silent revert |

Measured on 2026-07-25: 48 of 241 behaviors sat in upstream-owned files. Those are
documented. Documenting the other 193 would add prose no conflict will ever consult.
If that ratio shifts — because a fork file gets upstreamed, or fork code moves into a
shared file — promote the affected rows out of the ledger then.

## Registry format

A registry is a markdown table. The checker reads the first and fourth columns;
the middle two are for humans.

```markdown
| ID | Behavior | Breaks if lost | Verified by |
|---|---|---|---|
| TP-MTIME-02 | The ordering rule is symmetric... | Kind-based grouping returns... | `newer_directory_sorts_before_older_file` |
```

- **ID** — reuses the `TP-*` markers already spread through the tree. The same
  id must appear in a source comment next to the test, which is how a reader
  gets from code to contract and back.
- **Breaks if lost** — write the user-visible consequence, not the mechanism.
  This column is what a future reader uses to judge whether a merge conflict
  resolution is acceptable.
- **Verified by** — backticked test names. Rust `fn` names and Playwright
  `test("...")` names both count.

## What the checker enforces

| # | Check | Why it exists |
|---|---|---|
| C1 | Every named test still exists in the tree | The gate this whole system is for: a merge that deletes a test now fails the build |
| C2 | Every registered id still has a source marker | The behavior may have been removed along with its code |
| C3 | Every source marker is registered or ledgered | A new behavior cannot arrive undocumented |
| C4 | The ledger stays truthful and shrinks | Documented ids must leave it; ids no source uses must leave it too |
| C5 | No id is defined twice | One id, one meaning |
| C6 | Every marker follows `TP-<FAMILY>-<NN>` | A marker the pattern only partly matches is silently renamed into a shorter id, merging unrelated behaviors and breaking the path from a row back to the code |

C1 applies to ledger rows as well, so every behavior is guarded from day one;
only the prose is owed.

C6 was added on 2026-07-25 after it turned out the checker had been truncating
`TP-Act.1`…`TP-Act.5` into a single invented `TP-A`, dropping `TP-2B`…`TP-2F`
entirely, and inventing ids such as `TP-FLF-BOUNDED-TP` that then reached a
registry. Silent truncation is worse than an unknown id, because it looks like
coverage.

Three run forms are legal and expand to several ids: `TP-DCLICK-01/02/04`
(sibling numbers), `TP-FIP-ICON-01..05` (a numbered range), and
`TP-FLF-STEP-01/RENDER-01` (a sibling subfamily — a tail ending in a number
aligns with the end of the base, so this names `TP-FLF-RENDER-01`).

## Working with it

```bash
# Verify (also runs inside `just check`)
python3 -m scripts.behavior_registry_check

# Regenerate the ledger after adding or moving markers
python3 -m scripts.behavior_registry_check --write-ledger
```

**Adding a behavior:** put a `TP-<FAMILY>-<NN>` comment above the test that
pins it, add a row to the right `<feature>.md`, and run the checker. If you are
not ready to write the description, regenerate the ledger instead — but a
marker with no test at all fails the build, which is the point.

**Documenting a ledger row:** move it into a `<feature>.md` with a real
description, then regenerate the ledger. The check fails while an id sits in
both places, so the debt cannot quietly stay.

**Deleting a behavior:** remove the marker, the tests, and the registry row
together, and say why in the commit. The checker will not let you remove only
some of them.

## Related guards

`scripts/license_guard_check.py` covers the one thing no behavior test could:
it fails the build if the fork stops being AGPL-3.0-or-later, or if the Apache
attribution for upstream's post-`cd5ea1be` code goes missing.
