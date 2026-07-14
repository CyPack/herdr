# Codex Memory System for Herdr

Codex and Claude do not share conversation history automatically. Restarting either client does not import the other product's chats. Continuity is therefore file-based and source-attributed.

## Layers

1. Working memory: the active Codex conversation and current plan. It is fast but disposable.
2. Short-term project state: `.codex/CURRENT.md`, `.codex/TASKS.md`, and `.planning/STATE.md`.
3. Long-term project memory: `.codex/MEMORY.md`, architectural PRDs under `.local/prd/native-fm/`, and exact evidence under `.codex/evidence/`.
4. Code relationship memory: codebase-memory MCP project `home-ayaz-projects-herdr`. It is not a substitute for Git truth and must be freshness-tested.
5. Global Codex recall: `~/.codex/memories/`. This is managed by Codex; project updates are submitted as small notes under `extensions/ad_hoc/notes/`, not by rewriting generated memory indexes.
6. Cross-CLI owner routing: `agent-memory-route` maps Herdr cwd/intent to owner `herdr-native-fm`, skills `herdr-native-fm` and `rust-dev`, and the three canonical project state files.

## Retrieval Order

Load only what the task needs:

1. `CURRENT.md`
2. `TASKS.md`
3. relevant section of `MEMORY.md`
4. `HANDOFF.md` for cross-session reconstruction
5. evidence or raw source only when exact proof is necessary

For code, query the graph before reading broad file sets. For Git status, tests, ports, processes, remotes, and tool availability, verify live because those facts drift.

## Update Rules

- Update `CURRENT.md` after a verified state transition, commit, branch change, or new blocker.
- Update `TASKS.md` when work is discovered, completed, reordered, or blocked.
- Add stable decisions and reusable lessons to `MEMORY.md`; do not store transient command output there.
- Keep evidence immutable or append-only. Record source path, timestamp, SHA/hash, and what was inferred.
- Never store secrets, tokens, raw auth config, private message payloads, or entire chat transcripts in project memory.
- Mark unverified or drift-prone facts explicitly.
- Before handoff, synchronize `.planning/STATE.md`, `.codex/CURRENT.md`, and `.codex/HANDOFF.md`.

## Consolidation

When `MEMORY.md` becomes noisy, archive superseded details into evidence/timeline files and keep only current facts plus pointers. Never silently delete a decision without recording what superseded it.
