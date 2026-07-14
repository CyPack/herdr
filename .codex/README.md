# Herdr Codex CLI Continuity

This directory is the durable entrypoint for continuing Herdr work from Codex CLI. It imports the useful state from Claude session `f53c720f-f795-4778-970b-d227714ffb1a` without copying the 3.36 MB raw transcript into the repository.

## Start

From any terminal:

```bash
herdr-codex
```

Equivalent direct command:

```bash
codex -C /home/ayaz/projects/herdr 'Use $herdr-native-fm. Read .codex/BOOTSTRAP.md and continue from .codex/CURRENT.md.'
```

## Canonical Files

- `BOOTSTRAP.md`: mandatory load order and safety checks.
- `CURRENT.md`: current branch, worktree, verification, and exact next action.
- `TASKS.md`: durable ordered task list and blockers.
- `HANDOFF.md`: detailed cross-session handoff.
- `MEMORY.md`: stable project facts and retrieval pointers.
- `MEMORY-SYSTEM.md`: how Codex memory works for this project.
- `evidence/claude-session-f53c720f.md`: verified reconstruction of the source Claude chat.
- `skills/herdr-native-fm/SKILL.md`: project-local Codex routing skill.
- `bin/herdr-codex`: deterministic CLI launcher.

The raw Claude session remains at its original local path. `.local/prd/native-fm/` remains the detailed native-FM research and planning vault.
