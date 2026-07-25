# Upstream sync protocol

How to pull `ogulcancelik/herdr` into this fork without losing fork behavior or
fork licensing. Written from the 2026-07-25 sync, which moved 130 commits
(`b48bd903` → `362d6f14`) and is the worked example throughout.

> Scope note: this fork never pushes to upstream and never opens issues or PRs
> there. Upstream is a read-only source.

---

## The iron law

> **A sync is not finished when it compiles and the suite is green. It is
> finished when every registered behavior and the licence are still provably
> ours.**

Green means nothing on its own here: the licence flipped to Apache-2.0 in a
fully green tree.

---

## Phase 0 — Green baseline (gate; do not skip)

```bash
just check                                    # must exit 0
python3 -m scripts.behavior_registry_check     # must be OK
git status --porcelain                         # clean but for known untracked
```

Record the test count. Every later number is compared against it.

**Why it is a gate, not a formality:** without a green baseline, every failure
after the merge is ambiguous — merge damage or pre-existing breakage? That
ambiguity costs more than the baseline run.

## Phase 1 — Measure before merging

Conflicts are cheaper in small steps, but only measurement says how small.
`git merge-tree` answers this without touching a ref or a file:

```bash
git fetch upstream --tags
git merge-tree --write-tree HEAD upstream/master | grep -c CONFLICT
git merge-tree --write-tree HEAD <intermediate-tag> | grep -c CONFLICT
```

2026-07-25 measurement: 19 conflicting files in one jump, versus 9 → 11 → 6
across `v0.7.4` → `v0.7.5` → `master`. Staged won, and each stage stayed small
enough to reason about.

## Phase 2 — Isolated worktree

```bash
git worktree add -b sync/upstream-<n> ../herdr-worktrees/upstream-sync <branch>
```

The main worktree and any parallel session's branch stay untouched, and a bad
sync is discarded with `git worktree remove`.

⚠️ **Do not set `CARGO_TARGET_DIR` to another worktree's target.** The live
integration tests locate the binary with
`path.starts_with(env!("CARGO_MANIFEST_DIR"))`, so a shared target directory
makes every `live_*` test fail for a reason that has nothing to do with the
merge. Sharing is safe for `cargo check` only.

## Phase 3 — Resolve, one stage at a time

Merge one step, resolve, then run the full gate before starting the next.

**Resolution rules.** Blanket `-X ours` / `-X theirs` is banned; it destroys
data in the first file where both sides were right.

| Situation | Rule |
|---|---|
| Upstream fix, does not touch fork behavior | take theirs |
| Fork feature, upstream never touched the line | keep ours |
| Both changed different things in one hunk | **merge both intents by hand** |
| Upstream removed an API the fork calls | find the replacement; if the fork still needs the old shape, re-add it deliberately and say so in the commit |
| Upstream renamed or resignatured | update every call site — grep call sites, type positions and tests separately |
| Upstream reworked its own tests | take theirs; they encode upstream's semantics |
| Upstream test asserts upstream UI | re-baseline it for the fork **and write down why** in the test |

Per stage:

```bash
grep -rn "^<<<<<<< \|^>>>>>>> " src/ tests/   # must be empty
cargo check --all-targets --locked
cargo fmt
just check
```

## Phase 4 — Licence gate

The sync will take upstream's licence files without conflicting, because the
fork has not edited them since the fork point.

```bash
python3 -m scripts.license_guard_check
```

If it fails, restore all four together — `LICENSE`, `Cargo.toml`,
`nix/package.nix`, the README licence section — and keep `LICENSE-APACHE` and
`NOTICE` in place for upstream's Apache-licensed portions.

## Phase 5 — Behavior gate

```bash
python3 -m scripts.behavior_registry_check
```

A failure here reads directly as *"the merge removed a test that pinned one of
our behaviors."* Restore the behavior, not the registry row.

Then the fork's own surfaces:

```bash
cargo nextest run --locked --no-fail-fast -E 'test(fm::) or test(input::file_manager) or test(ui::file_manager)'
cargo nextest run --locked --no-fail-fast -E 'test(trail) or test(file_operation_worker) or test(preview)'
cargo nextest run --locked --no-fail-fast -E 'test(project) or test(file_agent_handoff)'
```

## Phase 6 — Flake hunt

Merges do not only break behavior; they change timing, and timing exposes
fragility that was always there.

```bash
for i in $(seq 1 8); do cargo nextest run --locked --no-fail-fast --status-level fail; done
```

Eight clean consecutive runs before integrating. The 2026-07-25 sync surfaced
four order-dependent tests this way; three of them looked exactly like merge
regressions and were not.

**When one fails, prove the cause before fixing it.** Reproduce deterministically
first — for the mtime class, inject a `sleep` between fixture writes and watch
the test fail every time; then pin the fixture and confirm the same injected
scenario passes. A fix without a reproduction is a guess.

## Phase 7 — Integrate

```bash
git merge-base --is-ancestor <main-branch> sync/upstream-<n>   # fast-forward only
git -C <main-worktree> merge --ff-only sync/upstream-<n>
just check                                                     # in the main worktree
```

Push is a separate, explicitly approved step. Never push to `upstream`.

## Phase 8 — Record

Update the behavior registry if the merge changed a contract, note protocol
version changes, and write down anything the next sync should expect.

---

## Traps this protocol exists to catch

| Trap | Signature | Phase |
|---|---|---|
| Silent relicence | No conflict; `LICENSE` shrinks from 671 to 201 lines | 4 |
| Behavior deleted with its test | Suite green, feature quietly gone | 5 |
| Shared `CARGO_TARGET_DIR` | Every `live_*` test fails; nothing to do with the merge | 2 |
| Order-dependent fixtures | Passes alone, fails in the full run, or the reverse | 6 |
| Upstream removed an API we call | Compile error far from the conflict | 3 |
| Upstream characterization digest | Structural asserts pass, only the hash differs | 3 |
| Double handling after a refactor | One root cause, several unrelated-looking failures | 3, 5 |
| Protocol version bump | New build cannot talk to an older running server | 8 |

## Worked example — 2026-07-25

| | |
|---|---|
| Range | 130 commits, `b48bd903` → `362d6f14` |
| Conflicts | 9 + 11 + 6 files across three stages (19 in one jump) |
| Tests | 3683 → 4022 |
| Result | `just check` exit 0, eight clean consecutive runs |
| Caught | Silent relicence · triple failure from one double-handled key path · four order-dependent fixtures |
| Left open | Push (needs approval) · visual check (needs a human at a terminal) |

Full record: `.local/prd/2026-07-25-FAZ0-upstream-sync-PRD.md`.
