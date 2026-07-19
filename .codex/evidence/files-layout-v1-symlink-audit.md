# Files Layout V1 Symlink Audit

Date: 2026-07-19

## Scope

The word “symlink” refers to two independent systems in the current Herdr
continuity:

1. filesystem entries and configured locations consumed by Native Files;
2. machine-local Claude/Codex continuity and skill pointers.

They have different owners and failure modes. Neither may be used as evidence
for the other.

## Native Files Symlinks

Fresh Codebase Memory at 23,854 nodes / 124,093 edges returned the current
production test seams:

- `src/fm/mod.rs::symlink_to_directory_counts_as_directory`;
- `src/app/input/sidebar.rs::locations_rail_mouse_symlink_directory_loads_exact_trail`;
- `src/fm/mod.rs::symlink_uses_its_own_modification_time`;
- `src/fm/mod.rs::snapshot_symlink_classification_is_independent_of_mtime_sort`;
- `src/fm/rename.rs::rename_real_filesystem_preserves_file_directory_and_symlink_payloads`;
- `src/fm/delete.rs::real_trash_backend_isolated_child_preserves_symlink_target`.

Verified contract:

- a symlink whose target is a directory is actionable as a directory;
- a Files location row may point at that symlink and opens a Trail whose exact
  root identity remains the symlink path;
- the target directory's children are visible through the symlink;
- mtime sorting uses symlink-preserving metadata rather than silently adopting
  the target's timestamp;
- rename/delete families preserve or isolate symlink payload/target authority.

This coverage proves classification and exact-path behavior. It does not prove
that a cold, remote, removable, cyclic, dangling, or slow symlink target meets
the rapid-navigation latency budget.

## Claude/Codex Local Symlinks

Live read-only audit:

| Pointer | Target | Status |
|---|---|---|
| `~/.codex/claude-memory` | `~/.claude/projects/-home-ayaz/memory` | OK |
| `~/.codex/shared-state` | `~/.claude/projects/-home-ayaz/memory/cli-state` | OK |
| `~/.codex/handoffs/herdr/CURRENT.md` | `/home/ayaz/projects/herdr/.codex/CURRENT.md` | OK |
| `~/.codex/skills/rust-dev` | `~/.claude/skills/rust-dev` | BROKEN |
| `~/.codex/skills/mnm-laptop-mdate` | `~/.claude/skills/mnm-laptop-mdate` | BROKEN |
| `~/.codex/skills/openwa-scheduled-send` | `~/.claude/skills/openwa-scheduled-send` | BROKEN |

Fresh `agent-parity-status --strict` confirms:

- canonical memory, Codex memory link, Codex state link, and home state link
  are OK;
- Codex has three broken skill symlinks;
- the only Herdr-development-relevant broken link is `rust-dev`;
- the Herdr-local fallback remains `docs/patterns/rust-engineering.md`.

These pointers are CLI startup/continuity infrastructure. Native Files does
not traverse them while routing mouse input, mutating the Trail, scheduling
directory work, projecting the view, or streaming a frame. They are therefore
not a direct cause of the reported Files click latency.

No global symlink was created, repaired, removed, or retargeted by this audit.

## Performance Classification

The FMP reproduction matrix must compare:

1. resident real directory;
2. resident directory symlink to a local target;
3. non-resident real directory;
4. non-resident directory symlink to the same local target;
5. dangling/changed-type symlink failure;
6. an explicitly user-provided slow/remote symlink only if it can be tested
   inside the isolated, test-owned environment.

Expected classification:

- resident real and resident symlink clicks perform zero filesystem reads;
- equivalent local real/symlink targets do not create an unbounded request or
  frame queue;
- non-resident reads stay in the bounded one-executing/one-latest-pending FM
  worker;
- dangling/changed-type results fail closed and preserve the resident Trail;
- CLI continuity symlink health has no correlation with Files input/render
  profiler counters.

## Decision

Keep the two symlink systems separate:

- add Native Files symlink latency/failure cases to FMP;
- leave the machine-global broken skill links unchanged during the product
  investigation;
- if the user requests parity repair, handle it as a separate local-tooling
  task and validate with `agent-parity-status --strict`.
