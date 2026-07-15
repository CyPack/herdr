# Herdr Native-FM Planning State

- Updated: 2026-07-15
- Branch: `feat/native-fm`
- Published CyPack M3 evidence checkpoint: `e9f2fe0`.
- Verified M2.1 chain: RED `dab1e20`; GREEN product head `0ae6175`.
- M3.0 evidence is published with exact SHA equality to CyPack
  `feat/native-fm` and fork `master`. No product publication is pending.
- Completion audit: all approved native-FM core/post-core scope is reconciled
  in `.codex/evidence/native-fm-completion-audit.md`; ignored local PRD drift
  was repaired without product changes. Only S5/S6/S7/N2.2 remain trigger-gated.

## Completed

- A2.2 responsive Miller columns.
- A3 viewport, stable row geometry, keyboard/mouse navigation.
- A4 native watcher plus bounded reconciliation fallback.
- B0 image Path Beta, B1 bounded text preview, B2 bounded native image preview.
- C1/N3 header actions, prepared selection authority, and clipboard model.
- C2/N4 stable row actions, cursor-independent bounded multi-selection, and
  bulk authority.
- C3.1 deterministic file context-menu model.
- C3.2 global-popup reuse, exact path-stable right-click selection policy,
  bounded placement, keyboard/mouse lifecycle, disabled styling, activation-
  time authority revalidation, and typed client-local intent only.
- C3.3 enabled/available/host-supported plugin file actions, deterministic
  ordering, exact public path context, activation-time disable-race checks,
  non-UTF-8 fail-closed behavior, and display-width-aware dynamic labels.
- C4.1 immutable exact-path preflight, staged no-replace COPY, atomic/EXDEV-safe
  MOVE, bounded single-lane worker, pure generation/terminal state, header and
  context Copy authority, Paste dispatch, and matching-cwd reconciliation.
- C4.2 exact-path Trash/Permanent confirmation, immutable symlink-safe delete
  preflight, immediate identity revalidation, restricted platform-trash
  backend, permanent file/directory deletion, shared bounded worker lane,
  ordered per-item recovery evidence, and matching-cwd reconciliation.
- C4.3 exact single-target Rename intent, shared platform-aware component
  validation, immutable identity revalidation, no-replace file/directory/
  symlink commit, bounded cycle-safe bulk staging, explicit recovery paths,
  shared worker lane, and matching-generation App reconciliation.
- C4.4.1 one bounded latest-value worker progress slot, monotonic started-item
  projection, same-generation App consumption, and production progress
  adapters shared by transfer, delete, single rename, and bulk rename.
- C4.4.2 explicit reversible/irreversible cancellation boundaries, typed Esc
  intent, matching-generation App/worker routing, repeated-cancel idempotence,
  revalidation-race precedence, and buffered-completion authority.
- C4.4.3 deterministic worker/watcher reconciliation for queued,
  watcher-first, delayed, polling-fallback, selection-pruning, cwd/rebind, and
  same-cwd close/reopen races.
- C4.4.4 terminal recovery for disconnect-after-progress, caught panic,
  cancel-to-next-generation, uncertain private bulk-recovery paths, generation
  floor preservation, lane reuse, baseline cleanup, and no-hot-retry sync.
- C4.4.5 complete closure gate across focused recovery, broad C4/FM, full Rust,
  Linux/Windows lint, Bun/Python maintenance, ignored inventory, graph
  freshness, and artifact/diff cleanliness.
- C5 exact existing-agent literal path handoff and non-agent Claude split with
  exact new-resource rollback.
- C6 Finder-fidelity sidebar, integrated action authority, visual polish,
  explicit directory/preview/operation failure truth, and isolated runtime
  composition proof.
- P4.0 architecture evidence gate: S5 component registry, S6 persisted shell,
  and S7 popup stack remain implementation NO-GO without concrete consumers.
- N2.0 rejected an arbitrary/unbounded Miller history state machine; N2.1
  preserves the exact departed child on parent navigation through a bounded
  path-focus refresh seam.
- M1.0 proved a genuine product delta and selected the narrow existing-agent,
  single-file attachment overlay described in
  `.codex/evidence/m1-agent-attachment-picker.md`.
- M1.1–M1.4 add the pure focused-agent `[+]` action, `prefix+a`, Clear-first
  private picker, exact workspace/`PaneId`/`TerminalId` authority, path-stable
  mouse/keyboard input, and one-shot literal path-plus-CR delivery.
- M2.0–M2.4 are closed: `[w]` uses pure cached capability and exact identity
  to enter the existing open dialog; duplicate Create/Remove/List/Switch
  implementations remain NO-GO.
- M3.0–M3.3 are closed implementation NO-GO: M1/M2 repeat only small pure
  geometry/render mechanics, not lifecycle, authority, focus, event, or
  cleanup ownership. No trait, registry, migration, or Rust product diff was
  created. Evidence: `.codex/evidence/m3-general-ui-interface.md`.

## Active Next Increment

No speculative production increment is active. Activate future work only when
one recorded independent trigger is satisfied by concrete product demand.

## Ordered Roadmap

1. S5 only after a second independently owned page/component repeats render,
   hit geometry, lifecycle, focus/close ownership, and event routing.
2. S6 only after a real additional resizable region requires persisted identity,
   migration, restore, and adversarial-width behavior.
3. S7 only after a real nested popup must retain and restore parent ownership.
4. N2.2 only after independent retained-history demand plus finite eviction and
   restore semantics.
5. A third frame action may first justify a private pure draw/geometry helper;
   it does not automatically authorize a registry.

## Fresh M2 Closure Evidence

- Exact M2.1 5/5; worktree/attachment regression 131/131.
- Full nextest 3202/3202; only `path_beta_real_host_probe` ignored.
- Linux all-target and canonical Windows MSVC clippy clean with `-D warnings`.
- Bun 17/17; Python maintenance 64/64; fmt, diff, production-unwrap,
  debug-marker, and ignored-inventory checks clean.
- Graph refresh: 19,534 nodes / 91,017 edges. Current symbols for
  `miller_layout`, `compute_agent_worktree_action_area`, and cached capability
  proved freshness beyond `ready`.
- No dependency, protocol, persisted runtime state, Git/filesystem mutation,
  process, socket, watcher, worker, pane, or stable Herdr resource changed.

## Fresh M3 Evidence

- Graph-first inventory at 19,534 nodes / 91,017 edges returned current
  `miller_layout`, M1/M2 compute/render areas, and input ownership; `ready`
  alone was not accepted.
- Characterization set: 16/16, zero retry, run
  `32ca7f37-b65c-45ef-9dbf-548e8263d383`.
- Protected Base terminal/FM swap, desktop/mobile shell regions, responsive and
  disjoint M1/M2 geometry/render, exact stale-identity dispatch, picker/dialog
  cleanup, modal/context focus/close, and old snapshot compatibility.
- Documentation/continuity only: no Rust, dependency, protocol, persistence,
  worker, watcher, filesystem/Git operation, process, pane, or socket change.

## Non-Negotiable Boundaries

- Pure render; filesystem preparation and execution stay outside render.
- No production `unwrap()` and no hidden authority from labels/coordinates.
- C3 emits intent only; C4/C5 own all side effects and execution-time checks.
- Never touch stable Herdr/socket or kill/restart user processes.
- Targeted staging only; never `git add -A`; never push `upstream` or force.

Canonical sources: `.codex/CURRENT.md`, `.codex/TASKS.md`, and
`.codex/HANDOFF.md`.
