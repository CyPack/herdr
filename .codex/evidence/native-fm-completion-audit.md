# Native File Manager Completion Audit — 2026-07-15

## Decision

The native-FM delivery tree is complete for the approved scope: all thirteen
core A1–C6 modules, integrated N1/N3/N4, narrow N2.1, M1, and M2 are published.
M3 is complete as an evidence-backed implementation NO-GO. No speculative
production lane is active.

This audit found continuity drift, not missing product code: the ignored local
PRD tree still described B0, the A3 remainder, and B1–C6 as pending even though
tracked tasks, commits, gates, and graph evidence proved them closed. The local
tree and module checklists were synchronized on 2026-07-15. Tracked
`.codex/TASKS.md` remains the single task authority.

## Test Points

| Test point | What is tested | Expected result | Reason |
|---|---|---|---|
| TP-AUDIT-SCOPE | Root PRD tree and every A1–C6 module checklist against tracked tasks | Every approved core module has one closure record; templates and future gated work are not misclassified as active | A stale local plan can cause a later agent to reimplement already published work |
| TP-AUDIT-EVIDENCE | Module closure commits, later full gates, real-host/real-filesystem evidence, and graph freshness | Every module maps to an auditable commit/evidence chain and is protected by a later complete gate | A checked box without executable or published evidence is not completion |
| TP-AUDIT-OPEN | S5, S6, S7, and N2.2 activation criteria | Exactly those four items remain unchecked and none is active without its recorded independent trigger | Future architecture must emerge from measured pressure, not an empty-looking queue |
| TP-AUDIT-GIT | Local feature, CyPack feature, CyPack master, remotes, staged paths, and ancestry | Publication is fast-forward, exact-SHA, targeted-stage, fork-only; `upstream` is unchanged | Completion evidence is unreliable if it is mixed, unpushed, or published to the wrong repository |
| TP-AUDIT-REGRESSION | Fresh M3 characterization plus the latest complete product-tree gate | 16/16 exact-head characterization remains green; the unchanged product tree retains 3202/3202 plus only the named B0 host-probe skip and all platform/maintenance gates | Documentation-only closure must not pretend to be a new full product run, but it must name the latest valid product evidence |

## Core Scope Matrix

| Scope | Closure reference | Protecting evidence |
|---|---|---|
| A1 filesystem model/natural sort | `c6e64a3` | Covered by the later A2 full 2886/2886 gate and subsequent full suites |
| A2 responsive Miller render/N1 | `6c7c58f` | FM 43/43; full 2886/2886; Linux/Windows clippy; Bun 17/17; Python 64/64 |
| A3 navigation/viewport/mouse scope | `d713b71`…`9d69c82` | targeted 164/164; scope 4/4; full 2966/2966; isolated SGR-mouse PTY |
| A4 watcher/reconciliation | `01ba91d` | FM 69/69; full 2912/2912; real-fs create/rename/delete/burst; platform and maintenance gates |
| B0 Path Beta feasibility | `bcba84d` | generated/corrupt decode, encoder lifecycle, isolated real Kitty Path Beta and Path Alpha baseline |
| B1 bounded text preview | product head `2b2dcd3`; closure `a0f82a3` | targeted 64/64; full 2948/2948; Linux/Windows and maintenance gates |
| B2 native image preview | `de1eff5`…`2989434` | B2/FM/Kitty 96/96; full 2983/2983; 0/271425 host pixel difference; cleanup proof |
| C1 header actions/N3 authority | through `267ad91` | final full 2996/2996 plus focused geometry/dispatch/authority gates |
| C2 row actions/N4 bulk authority | through `cb5a43e` | final full 3020/3020 plus bounded multi-selection/atomicity gates |
| C3 context/plugin actions | `d56e3db`…`3c11369` | C3.3 8/8; plugin/context 35/35; FM/watcher/menu 112/112; full 3041/3041 |
| C4 filesystem operations | through `c674296` | recovery 46/46; C4 core 67/67; broad 218/218; full 3131/3131; artifact and real-fs checks |
| C5 agent handoff | through `f744e4d` | C5.4 4/4; related 17/17; full 3143/3143; exact rollback/failure coverage |
| C6 Finder-fidelity closure | through `f52cb85` | full 3171/3171; isolated composition; platform/maintenance/unwrap/artifact checks |

All full nextest runs above had only the explicitly named ignored
`path_beta_real_host_probe` where stated; the probe was independently executed
in isolated throwaway-host work and is not an unexplained skip.

## Post-Core Roadmap Matrix

| Scope | Result | Evidence |
|---|---|---|
| N2.0/N2.1 | unbounded history NO-GO; narrow path-stable parent return complete | RED `e433a2f`; GREEN `c530836`; exact 6/6; FM 65/65; full 3177/3177 |
| M1 | focused-agent attachment picker complete | `948ccf8`…`7d3144e`; exact 20/20; full 3197/3197 |
| M2 | focused-agent worktree launcher complete | RED `dab1e20`; GREEN `0ae6175`; exact 5/5; combined 131/131; full 3202/3202 |
| M3 | general interface implementation NO-GO | `.codex/evidence/m3-general-ui-interface.md`; fresh 16/16 characterization; no product diff |

## Only Remaining Future Triggers

1. S5: a second independently owned page/component repeats render, hit
   geometry, lifecycle, focus/close ownership, and event routing.
2. S6: a real additional resizable region needs persisted identity, migration,
   restore, and adversarial-width behavior.
3. S7: a real nested popup must retain and restore parent ownership.
4. N2.2: independent retained-history demand supplies finite eviction and
   restore semantics.

Until one condition is proved, these are deferred decision points rather than
active implementation tasks. A third frame action may justify a private pure
draw/geometry helper, but does not by itself authorize a registry.

## Audit Baseline

- Baseline HEAD before this documentation audit: `5b13aec`.
- Local `feat/native-fm`, `origin/feat/native-fm`, and `origin/master` were
  exactly `5b13aec2ba68bbd42ab658913bf9cf7f90e93357`.
- `origin` is `CyPack/herdr`; `upstream` is `ogulcancelik/herdr` and is outside
  write scope.
- Graph status was `ready` at 19,534 nodes / 91,017 edges and freshness was
  separately proved with current `miller_layout` and M1/M2 symbols.
- The only visible untracked path was the user-owned
  `.codex/skills/ratatui-design-intelligence/`; it was not staged or modified.
- Fresh completion-audit characterization reran the exact 16 protected M3
  tests: 16/16 passed, 3,187 skipped, zero retry; nextest run
  `c3e40137-6400-4547-9eb8-729f29fd6583`.

## Closure Rule

The audit is publishable only after local PRD consistency checks, tracked-task
open-item checks, `git diff --check`, no-product-diff checks, targeted staging,
commit inspection, fetch/fast-forward ancestry verification, and exact remote
SHA verification for CyPack feature then CyPack master. Never push upstream or
force.
