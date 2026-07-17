# Codex CLI Bootstrap — Herdr Native FM

Follow this sequence before changing code.

1. Read `/home/ayaz/projects/herdr/AGENTS.md` and `CLAUDE.md` completely. The active account is `CyPack`, so external-contributor guardrails apply.
2. Read `.codex/CURRENT.md`, `.codex/TASKS.md`,
   `.codex/CHANGE-PIPELINE-TASKS.md`, and `.codex/HANDOFF.md` in that order.
   Recount every unchecked item and create the in-session task list without
   summarizing tasks away. `.codex/NEXT-SESSION-PROMPT.md` is the canonical
   copy/paste trigger.
3. Load the project skill `$herdr-native-fm`. Load `rust-dev` plus its lessons before Rust edits.
4. Call codebase-memory `index_status(project="home-ayaz-projects-herdr")`. Test freshness with `miller_layout` or another recent symbol. Use graph discovery before grep for code.
5. Run:

   ```bash
   git status --short --branch
   git log --oneline -8
   git remote -v
   ```

6. Treat every completed checkpoint recorded in `.codex/CURRENT.md` and
   `.codex/TASKS.md`, including C6.4, as published product history. Do not
   discard, reset, or reimplement that chain when starting the next module.
7. State test points before implementation. Use TDD, pure render, no production `unwrap()`, and evidence before completion claims.
8. Never touch the installed stable Herdr or inherited stable socket. Manual tests use `.local/ISOLATED-DEV-TEST.md` exactly.
9. Before a commit, review the targeted diff, rerun proportional gates, and use
   a lowercase conventional commit message. This project has standing user
   authorization for autonomous targeted commits and CyPack fork-only
   fast-forward pushes; do not repeatedly ask for commit alignment.

## First Action in a Fresh CLI Session

Verify Git and graph freshness against `.codex/CURRENT.md`, then resume the
first priority-eligible unchecked microtask in `.codex/TASKS.md`. At the
2026-07-17 checkpoint that is **FIP-G.1**: load
`superpowers:writing-plans` and turn the approved Files Interaction Polish
design into a code-level TDD implementation plan. Do not edit Rust before
FIP-G.2 reconciles every `TP-FIP-*` point with fresh graph and gate evidence.
The non-product change-pipeline queue and trigger-gated S5/S7 remain paused.
Preserve atomic commit concerns and push only the CyPack fork after fresh
evidence.
