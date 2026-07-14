---
name: herdr-native-fm
description: Resume and execute Herdr native file-manager and UI-composition work from the verified Codex handoff. Use for herdr native-FM, Miller columns, watcher, previews, file operations, UI-ARCH continuation, or when the user says to continue the recovered Claude session in /home/ayaz/projects/herdr.
---

# Herdr Native FM Continuation

Use the repository continuity package; do not reconstruct state from chat memory.

## Start

1. Read `/home/ayaz/projects/herdr/AGENTS.md` and `CLAUDE.md` completely.
2. Read `.codex/BOOTSTRAP.md`, `.codex/CURRENT.md`, and `.codex/TASKS.md` in that order.
3. Read `.codex/HANDOFF.md` when resuming across sessions or changing phases.
4. Read only the relevant sections linked from `.codex/MEMORY.md`; use the evidence file when exact historical proof is needed.
5. Run codebase-memory `index_status` before code discovery. Verify freshness with a recently added symbol; reindex when stale.
6. Run `git status --short --branch`, inspect the current diff, and preserve unrelated/user changes.

## Safety

- Never kill or restart a live Herdr, terminal, browser, editor, or user-session process.
- Never run an inherited Herdr socket against a development binary.
- For manual runtime checks, use the throwaway-XDG recipe in `.local/ISOLATED-DEV-TEST.md`.
- Treat `origin` (`CyPack/herdr`) as the fork and `upstream` (`ogulcancelik/herdr`) as read-only.
- Do not open an upstream issue or PR for the user.

## Development Discipline

- Load `rust-dev` and its lessons before Rust work.
- Define test points with expected result and reason before editing.
- Add a failing test first for new behavior, then implement the minimum change.
- Keep render pure; cache filesystem-derived FM context in state, never read the filesystem during render.
- Use targeted staging and lowercase conventional commits. The current project
  continuity grants standing user authorization for autonomous commits and
  CyPack fork-only fast-forward pushes; do not repeatedly ask for alignment.
- Prefer `just check`; if `just` is unavailable, run every command from the `check` recipe and state that it was an equivalent gate.

## Current Boundary

Do not reimplement A2.2 or A4. They are published checkpoints. The
authoritative next module and commit split are in `.codex/CURRENT.md` and
`.codex/TASKS.md`.
