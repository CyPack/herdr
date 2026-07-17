# SESSION HANDOFF — Herdr Files Interaction Polish

Updated: 2026-07-17 CEST

## 0. SONRAKI ADIM — TEK AKTİF İŞ

FIP-G.1 ve FIP-G.2 KAPANDI (2026-07-18). Onaylı code-level TDD planı:
`docs/superpowers/plans/2026-07-18-herdr-files-interaction-polish-implementation.md`
(commit `dd81ef59`; 29 görev; 57 benzersiz `TP-FIP-*` ID eşlendi — eski "55"
sayımı iki E2E ID'sini dışlıyordu, hiçbir ID atlanmadı).

İlk ve tek priority-eligible iş **FIP-0.1** (baseline freeze), ardından planın
Task 2-29 sırası. Rust implementasyonu başlamadan önce kırık global `rust-dev`
skill symlink'i (`~/.codex/skills/rust-dev` → eksik `~/.claude/skills/rust-dev`)
onarılmalı; herdr-lokal HP1-HP10 kataloğu mevcut ve önceliklidir.

Bu sıra tartışmaya açık bir öneri değil, kanonik görev önceliğidir.
Change-pipeline T3.1-T10.9 paused; S5/S7 trigger-gated; Apps/Desktop ve
drag-and-drop kapsam dışı.

## 1. DURUM ÖZETİ

- Repo: `/home/ayaz/projects/herdr`
- Branch: `feat/native-fm`
- Yerel başlangıç HEAD’i: `f097f6c7`
- Yerel tasarım zinciri:
  - `fc76f648 docs: design files interaction polish`
  - `f097f6c7 docs: record canonical graph project ids`
- Bu handoff turu başında CyPack `feat/native-fm` ve `master`:
  `b7d4217c441c0cf842e5775ff2556d641c5a7940`
- User-owned, untracked alan: `.superpowers/`
- `.superpowers/` hiçbir koşulda stage/edit/delete edilmez.
- Eski SF0-SF6 + FM1-FM5 programı tamam ve yayınlıdır; yeniden uygulanmaz.
- Yeni aktif program Files Interaction Polish’tir.
- Açık görev envanteri (FIP-G.1/G.2 kapanışı sonrası):
  - `.codex/TASKS.md`: 52
  - `.codex/CHANGE-PIPELINE-TASKS.md`: 89
  - toplam: 141
- Sadece FIP-0.1 in-progress yapılabilir; diğer 140 görev pending/paused kalır.
- Fresh continuity gates (2026-07-18 planning-gate closure):
  - exact task copy 141/141;
  - 57 unique `TP-FIP-*` (fresh deterministic count; the earlier "55" excluded
    the two E2E IDs — all 57 are mapped in the implementation plan);
  - Nextest run `4da2ee18-b784-4c38-aaab-98a2e8787511`,
    3,443/3,443 passed + 1 named skip;
  - Linux all-target and Windows MSVC Clippy clean;
  - Bun 5/5 + 12/12;
  - Python maintenance 64/64;
  - fmt, Markdown references and `git diff --check` clean.

## 2. ÜRÜN VİZYONU VE SINIRLAR

Bu program yeni bir genel UI framework’ü kurmaz. Mevcut typed Stage, bounded
Miller, generation-safe hit, terminal identity ve popup seams üzerine dört
gözle görülür kusuru production-grade biçimde düzeltir:

1. Default sidebar’daki Files primary click’i Native Files Stage’i açar;
   Spaces/Projects Terminal Stage’e client-local ve runtime kimliğini bozmadan
   döner.
2. Bir klasöre girildiğinde resident/önceki kolon row zero’ı değil, girilen
   exact child path’i highlight eder.
3. Directory, regular file, symlink-directory, symlink-file, broken symlink ve
   unsupported special entry semantik olarak ayrılır; ortak dosya sınıfları
   için deterministic icon classification ve Nerd/ASCII tek-cell profilleri
   vardır.
4. `Send to Agent` yerine `Add Reference to Agent...` görünür; mevcut Agents
   projection’ından explicit target seçilir ve file/directory path’i chat
   terminaline yalnızca metin olarak eklenir.

Mutlak no-submit sözleşmesi:

- payload tam olarak güvenli UTF-8 `path.as_bytes()` olur;
- CR, LF, Enter, prefix, suffix, shell quoting veya implicit whitespace yoktur;
- otomatik submit yoktur;
- implicit Claude split/chat oluşturulmaz;
- delivery yalnız hâlâ aynı workspace/pane/terminal/agent kimliğine yapılır;
- source path son seam’de yeniden doğrulanır;
- control/non-UTF-8, broken/special, stale identity, vanished path ve
  backpressure sıfır byte ile fail closed olur;
- retry queue/hot retry yoktur.

Drag-and-drop, speculative ComponentRegistry, yeni popup framework, server
protocol genişletme, persistence migration, Apps/Desktop ve unrelated shell
redesign açıkça kapsam dışıdır.

## 3. ZORUNLU BAŞLATMA SIRASI

Fresh agent aşağıdaki sırayı atlamadan uygular:

1. `/home/ayaz/projects/herdr/AGENTS.md` dosyasını tamamen oku.
2. `/home/ayaz/projects/herdr/CLAUDE.md` dosyasını tamamen oku.
3. `$herdr-native-fm` skill’ini kullan; skill çalıştırmadan önce:
   - `.codex/skills/herdr-native-fm/lessons/errors.md`
   - `.codex/skills/herdr-native-fm/lessons/golden-paths.md`
   - `.codex/skills/herdr-native-fm/lessons/edge-cases.md`
   - `/home/ayaz/.codex/skills/_shared/common-errors.md`
4. Şu dosyaları sırayla ve tamamen oku:
   - `.codex/BOOTSTRAP.md`
   - `.codex/CURRENT.md`
   - `.codex/TASKS.md`
   - `.codex/CHANGE-PIPELINE-TASKS.md`
   - `.codex/HANDOFF.md`
   - `.codex/MEMORY.md`
   - `.planning/STATE.md`
   - `.codex/NEXT-SESSION-PROMPT.md`
5. Tasarım kaynağını tamamen oku:
   - `docs/superpowers/specs/2026-07-17-herdr-files-interaction-polish-design.md`
6. Git/remote gerçeğini doğrula:
   - `git status --short --branch`
   - `git log --oneline --decorate -12`
   - `git remote -v`
   - `git ls-remote origin refs/heads/feat/native-fm refs/heads/master`
7. Codebase Memory’yi graph-first kullan:
   - canonical project: `home-ayaz-projects-herdr`
   - `get_architecture`
   - `search_graph`
   - `trace_path`
   - `get_code_snippet`
   - gerekirse `query_graph`
   - code discovery için grep ancak graph yetersizse
8. `.codex/TASKS.md` ve `.codex/CHANGE-PIPELINE-TASKS.md` içindeki her
   unchecked maddeyi continuation satırlarıyla yeniden say ve session task
   listesine eksiksiz aktar. Beklenen 54 + 89 = 143’tür. Sayı farklıysa kod
   yazmadan CURRENT/TASKS/HANDOFF drift’ini uzlaştır.
9. FIP-G.1 için `superpowers:writing-plans` ve lessons dosyalarını yükle.
10. Rust işi başlayacağı zaman ayrıca `rust-dev` ve lessons dosyalarını yükle;
    skill erişilemiyorsa erişilmiş gibi davranma.
11. Her bug/feature için production kodundan önce behavior-specific failing
    test; RED çıktısını oku; ayrı commit. Sonra minimum GREEN; ayrı commit.
12. Runtime/Chromium/PTY işi başlamadan
    `.local/ISOLATED-DEV-TEST.md` dosyasını tamamen oku ve throwaway
    XDG/socket kontratını uygula.

## 4. CODEBASE MCP — KANONİK HARİTA VE KÖK NEDEN

Kanonik Codebase Memory project ID:
`home-ayaz-projects-herdr`.

Final handoff refresh; single-worker CLI ve built-in MCP aynı store’u doğrular:

- nodes: 21,064
- edges: 98,009
- ana paketler: `app` 2,615; `ui` 968; `pane` 715; `fm` 457;
  `server` 407; `cli` 406; `terminal` 298; `workspace` 236.

Graph-first denetimle kanıtlanan mevcut kusur zincirleri:

- Default `ShellLayout::default` yalnız LeftPanel + WorkspaceStage üretir;
  default görünür shell’de AppDock yoktur. Sidebar tab click route yalnız
  `sidebar_tab` değiştirir; Stage activation yapmaz. Buna karşılık mevcut
  `activate_dock_app(Files)` seam’i bounded Files Stage’i açar. FIP-1 bu
  davranışları tek authority’de yakınsar.
- `MillerPathSegment.focused_child` tanımlanır fakat üretim akışında
  doldurulmaz. Resident projection `segment.cursor` kullanır; yeni segment
  cursor default 0 olduğu için nonzero child’dan sonra yanlış ilk satır
  highlight edilir. FIP-2 path identity’yi transfer öncesi bağlar.
- `read_directory_snapshot` / entry capability hazırlığı symlink kimliğini
  `(is_dir, operation_supported)` ikilisine indirger. `render_entry_row`
  semantik icon üretmez. FIP-3 tek canonical `FileEntryKind` ve derived
  capability modeline geçer.
- Mevcut agent handoff final payload’a `b'\r'` ekler; non-agent hedef hazırlığı
  implicit Claude split oluşturabilir. Existing `agent_panel_entries`,
  terminal identity lookup ve bounded `try_send_terminal_input` target
  projection/delivery için yeniden kullanılabilir. FIP-4/FIP-5 submit ve
  implicit split davranışını bu action’dan çıkarır.

Bu maddeler chat özeti değil, tasarım dosyasındaki symbol/call-path evidence’ın
özetidir. Planlama sırasında exact qualified names ve snippets tekrar okunur;
stale `ready` çıktısı freshness kanıtı sayılmaz.

Post-refresh MCP proof ayrıca FIP-G.1’i `.codex/BOOTSTRAP.md`,
`.codex/CURRENT.md`, `.codex/HANDOFF.md`, `.codex/MEMORY.md`,
`.codex/NEXT-SESSION-PROMPT.md` ve `.codex/TASKS.md` içinde buldu;
`focused_child` yalnız field/constructor sonuçlarında, mevcut
`sync_file_manager_agent_handoff_send` ise çağıranı ve fail-closed test
ailesiyle birlikte güncel graph’ta bulundu.

## 5. KATMANLAR VE OTORİTE SINIRLARI

Layer 0 — Semantic identities:

- exact path identity;
- Miller focused-child identity;
- `FileEntryKind`;
- workspace/pane/terminal/agent target snapshot.

Layer 1 — Filesystem preparation:

- metadata/kind classification transition/reload aşamasında;
- render sırasında filesystem I/O yok;
- symlink ve special entry truthfulness;
- control-character display escaping.

Layer 2 — Pure visual classification:

- exact-name override;
- case-insensitive extension class;
- deterministic generic fallback;
- Nerd ve ASCII tek display-cell token;
- ürün binary’sine yeni runtime dependency yok.

Layer 3 — Projection/render:

- pure `compute_view`/`render` contract;
- exact path selection/highlight;
- generation-scoped typed hits;
- tiny/narrow geometry;
- row action ile icon/name overlap yok.

Layer 4 — Input/controller:

- overlay/capture > typed hit > background precedence;
- primary click semantics;
- stale hit inert;
- picker keyboard/mouse/outside/Escape ownership;
- multi-selection reference action disabled.

Layer 5 — Runtime adapter:

- one bounded terminal-input attempt;
- last-seam target and path validation;
- exact bytes, no submit;
- zero implicit resource creation;
- zero hot retry.

Layer 6 — Verification/observability:

- Rust semantic tests;
- deterministic Ratatui TestBackend cell fixtures;
- Playwright Chromium screenshots;
- isolated real mouse;
- PTY exact-byte capture;
- performance/resource/invariant/full-gate evidence.

## 6. TEST-FIRST KAPISI

Her implementasyon dilimi başlamadan önce test planı şu alanları açıkça
belirtir:

- test ID ve exact test name;
- bug/feature’ın mevcut davranışı;
- beklenen RED failure ve neden gerçek davranış failure’ı olduğu;
- minimum GREEN authority seam’i;
- non-happy-path matrisi;
- regression family;
- focused command;
- broad command;
- platform/visual/runtime command;
- expected pass/fail/skip count;
- artifact ve cleanup sonucu;
- RED/GREEN/refactor commit subject;
- rollback sınırı.

Oracle önceliği:

1. pure Rust state/model/input tests;
2. Ratatui TestBackend exact-cell tests;
3. Playwright Chromium deterministic cell-grid screenshots;
4. isolated terminal mouse smoke;
5. PTY byte capture;
6. full repository/platform gates.

Playwright gerçek TUI semantiğini icat etmez. TestBackend’in exact cell buffer’ı
browser grid fixture’ına aktarılır. ASCII profile canonical cross-machine
baseline’dir; Nerd mapping Rust cell testleriyle ayrıca doğrulanır. Browser
yoksa gate açıkça fail olur; skip-success yoktur. Tek cell mutation snapshot’ı
fail ettirmelidir.

Kritik edge families:

- modified/middle/release-only/outside mouse;
- overlay/capture background blocking;
- collapsed/tiny sidebar;
- stale generation/hit;
- exact child at nonzero index;
- four-plus-level chain;
- reorder/delete/hide/branch-retirement;
- duplicate/malformed path;
- empty/root/permission/unavailable directory;
- symlink-file/symlink-dir/broken link;
- FIFO/socket/device/metadata failure;
- mixed-case extensions, dotfiles, exact well-known names;
- long ASCII/Unicode/control-containing filenames;
- narrow display-cell truncation;
- target disappears or terminal identity changes;
- no-longer-agent/runtime unavailable;
- path deleted or kind changes;
- non-UTF-8/control path;
- channel busy/full;
- cancel/outside/Escape;
- exact-once and zero retry;
- no CR/LF/Enter/submit;
- zero stable socket/process/temp residue.

## 7. GIT VE YAYIN DİSİPLİNİ

- Acting account `CyPack`; external-contributor/fork guardrail geçerli.
- Upstream’a push, issue veya PR açma yok.
- Normal implementation RED ve GREEN ayrı atomic commit.
- Refactor yalnız GREEN arkasında ve ayrı concern olarak yapılır.
- Commit subject lowercase conventional; emoji/AI co-author yok.
- Bulk stage yok; yalnız owned files `git add -- <exact paths>`.
- `.superpowers/`, `.local/`, unrelated dirty files stage edilmez.
- Commit öncesi staged diff ve staged file list okunur.
- Push öncesi `git fetch origin`; local HEAD’in remote ref’lerin
  descendant’ı olduğu `merge-base --is-ancestor` ile doğrulanır.
- Yalnız CyPack `feat/native-fm` ve fork `master` fast-forward edilir.
- Push sonrası iki remote SHA local HEAD ile birebir doğrulanır.
- Codebase Memory commit sonrası yenilenir; recent changed symbols/snippets
  tekrar sorgulanır.
- Completion claim ancak fresh command output ve exact count ile yapılır.

Standing user authorization targeted continuity/product commits ve CyPack-only
FF push’ları kapsar. Ancak bu yetki scope genişletmez, upstream’a izin vermez,
stable runtime’a dokunma izni vermez ve test kapılarını kaldırmaz.

## 8. AÇIK GÖREV ENVANTERİ — MACHINE-EXACT COPY

Bu bölüm iki canonical registry’den mechanically copied unchecked task
bloklarını continuation satırlarıyla içerir. Beklenen kaynak sayıları 52 ve
89, toplam 141’dir. Fresh agent bu kopyaya kör güvenmez; kaynaklardan yeniden
sayar ve exact diff yapar.

<!-- OPEN_TASKS_START -->

### Source: `.codex/TASKS.md` — 52 unchecked

- [ ] **FIP-0.1** Freeze current characterization tests and fresh graph
  evidence without changing product behavior.

- [ ] **FIP-0.2** Add an isolated Playwright Chromium package, config, and
  lockfile that never ships in the Herdr binary.

- [ ] **FIP-0.3** Add a test-only Ratatui cell-fixture exporter with exact
  character, foreground, background, modifier, and cell-position data.

- [ ] **FIP-0.4** Add the deterministic browser cell-grid renderer and harness
  self-tests for wide/combining glyphs, malformed fixtures, and exact cells.

- [ ] **FIP-0.5** Prove that a controlled one-cell mutation fails the matching
  screenshot and that ordinary CI never rewrites approved snapshots.

- [ ] **FIP-0.6** Keep screenshots, traces, browser state, and generated
  fixtures inside declared test/target artifact paths.

- [ ] **FIP-1.1 RED** Pin that a primary click on the visible default-sidebar
  Files tab opens the Native Files Stage.

- [ ] **FIP-1.2 GREEN** Converge the sidebar route on the existing bounded
  Files activation seam without duplicating Stage/runtime ownership.

- [ ] **FIP-1.3 RED** Pin that Spaces/Projects restore Terminal Stage while
  preserving exact pane, terminal, and runtime identity.

- [ ] **FIP-1.4 GREEN** Implement the symmetric client-local Stage transition.

- [ ] **FIP-1.5** Cover overlay/capture precedence, modified/middle/release-only
  clicks, collapsed/tiny sidebar, stale path/generation, and singleton reopen.

- [ ] **FIP-1.6** Add Playwright `TP-FIP-VIS-01` plus isolated real-mouse
  `TP-FIP-E2E-01` evidence without touching the stable Herdr socket.

- [ ] **FIP-2.1 RED** Pin that entering a nonzero child keeps that exact child
  highlighted in the departing/resident column rather than row zero.

- [ ] **FIP-2.2 GREEN** Bind exact `focused_child` path identity before
  resident-column ownership transfer.

- [ ] **FIP-2.3 RED** Pin reorder, delete, hide, duplicate, and malformed-path
  re-resolution/fail-closed behavior.

- [ ] **FIP-2.4 GREEN** Add one unique-path resolver and absent-selection
  projection; never substitute an unrelated first row.

- [ ] **FIP-2.5** Cover four-plus-level chains, branch retirement, leave/revisit,
  viewport clamp, empty/root/unavailable/permission states, and stale hits.

- [ ] **FIP-2.6** Add Playwright `TP-FIP-VIS-02` proof of exact-path highlight.

- [ ] **FIP-3.1** Characterize sorting, operations, symlink/special handling,
  watcher refresh, preview, row actions, Unicode, and narrow-column behavior.

- [ ] **FIP-3.2 RED** Require canonical filesystem classification for
  directory, regular file, symlink-directory, symlink-file, broken symlink,
  and unsupported special entries.

- [ ] **FIP-3.3 GREEN** Introduce `FileEntryKind` with derived capability
  methods while preserving current operational authority.

- [ ] **FIP-3.4** Migrate every consumer without retaining dual
  `is_dir`/`operation_supported` sources of truth.

- [ ] **FIP-3.5 RED** Pin exact-name, case-insensitive extension, semantic
  class, Nerd-glyph, and ASCII-fallback mappings.

- [ ] **FIP-3.6 GREEN** Implement a pure visual classifier and one-cell glyph
  profiles with no render-time filesystem/config/process work.

- [ ] **FIP-3.7** Cover truncation, display width, Unicode, control escaping,
  cursor/multi-select hierarchy, render purity, sorting, and operation parity.

- [ ] **FIP-3.8** Add Playwright `TP-FIP-VIS-03` and `TP-FIP-VIS-04`
  snapshots for mixed kinds and narrow/tiny layouts.

- [ ] **FIP-4.1 RED** Pin that the prepared payload is exactly the selected
  UTF-8 path bytes and contains no CR, LF, Enter, prefix, suffix, or whitespace.

- [ ] **FIP-4.2 GREEN** Replace the old send-and-submit payload with a
  reference-only payload.

- [ ] **FIP-4.3 RED** Pin directory acceptance and broken/special-path
  rejection.

- [ ] **FIP-4.4 GREEN** Share source-path validation and repeat the kind/path
  metadata check at the final delivery seam.

- [ ] **FIP-4.5 RED** Pin that a non-agent focus never creates a Claude
  split/chat for this action.

- [ ] **FIP-4.6 GREEN** Remove implicit split creation from the reference
  action while leaving unrelated split workflows unchanged.

- [ ] **FIP-4.7** Cover vanished path/workspace/pane, changed terminal,
  no-longer-agent runtime, control/non-UTF-8 path, backpressure, exact-once,
  cancellation, and zero-retry behavior.

- [ ] **FIP-5.1 RED** Pin that `Add Reference to Agent...` opens a picker from
  the existing live Agents projection.

- [ ] **FIP-5.2 GREEN** Project current chat first/preselected when live and
  every other live agent exactly once with stable identities.

- [ ] **FIP-5.3 RED** Pin keyboard, mouse, overlay ownership, outside-click,
  Escape, disabled row, and cancel paths.

- [ ] **FIP-5.4 GREEN** Reuse current popup geometry, focus, close, and
  responsive language instead of adding a popup framework.

- [ ] **FIP-5.5 RED** Pin target disappearance and workspace/pane/terminal
  identity change between picker open and activation.

- [ ] **FIP-5.6 GREEN** Recompute live rows and fail closed again at the final
  target-validation seam.

- [ ] **FIP-5.7** Rename all visible action copy to
  `Add Reference to Agent...`; support files and directories, never multi-select.

- [ ] **FIP-5.8** Add Playwright `TP-FIP-VIS-05` and `TP-FIP-VIS-06`
  snapshots for live, disabled, disappearing, and tiny-layout targets.

- [ ] **FIP-6.1** Run focused nextest for every Rust `TP-FIP-*` behavior.

- [ ] **FIP-6.2** Run the Playwright Chromium suite from fresh deterministic
  fixtures with failure screenshots/traces and no skipped-success claim.

- [ ] **FIP-6.3** Run isolated terminal mouse and PTY-byte smokes using
  `.local/ISOLATED-DEV-TEST.md`; prove exact path bytes and zero CR/LF.

- [ ] **FIP-6.4** Run format, full nextest, maintenance scripts, Linux Clippy,
  canonical Windows Clippy, Bun, and Python gates required by current Herdr.

- [ ] **FIP-6.5** Verify render purity, latency/resource budgets, state
  invariants, failure recovery, and zero new unbounded queue/cache/worker.

- [ ] **FIP-6.6** Refresh Codebase Memory and re-read every changed owner,
  caller, and data-flow seam rather than trusting `ready`.

- [ ] **FIP-6.7** Update `.codex` current/tasks/evidence, planning state,
  lessons, and next-session handoff with exact fresh evidence.

- [ ] **FIP-6.8** Verify clean tracked worktree, atomic RED/GREEN history,
  fast-forward ancestry, exact remote SHA equality, and CyPack-only push.

- [ ] Implement and verify `herdr-change-pipeline`, adapters, pilots, Git
  publication, and graph refresh; paused at T3.1 while the sequential active
  product lane closes its current phase.

Full non-product macro/micro registry:
`.codex/CHANGE-PIPELINE-TASKS.md`.

This lane does not authorize Rust product changes and does not activate S5,
S6, S7, or N2.2. A parallel feature/bugfix session may use the registry's
mid-flight adoption contract only after it inventories and preserves the live
work state.

- [ ] S5 ComponentRegistry only when a second real component/page proves the
  abstraction; do not build a speculative registry.

- [ ] S7 popup stack with ownership, focus, close ordering, and nested popup
  tests.

### Source: `.codex/CHANGE-PIPELINE-TASKS.md` — 89 unchecked

- [ ] **T3.1** Write RED `TP-CHG-MODULE` tests for module identity, version,
  directories, required documents, and default authorization=false.

- [ ] **T3.2** Create `.codex/skills/herdr-change-pipeline/` with `SKILL.md`,
  `README.md`, `AGENTS.md`, `module.json`, `assets/`, `references/`, `scripts/`,
  `tests/`, `evals/`, `lessons/`, and `cartography/`.

- [ ] **T3.3** Implement minimal manifest/schema validation and deterministic
  diagnostics until scaffold tests pass.

- [ ] **T3.4** Document skill routing, output ownership, resume behavior,
  source-of-truth order, and the separation from Ratatui reference research.

- [ ] **T3.5** Add errors, golden paths, edge cases, and shared-error routing.

- [ ] **T3.6** Verify the scaffold independently of Herdr product compilation.

- [ ] **T4.A0.1** RED-test every intake mode and reject unknown/ambiguous modes.

- [ ] **T4.A0.2** Model goals, inputs, evidence sources, current-work state,
  product authorization=false, and mode-conditional artifacts.

- [ ] **T4.A0.3** Implement `mid_flight_adoption` metadata: existing branch,
  commits, diffs, tests, known debt, current failures, and preserved evidence.

- [ ] **T4.A0.4** Block rather than fabricate when mandatory MCP/source evidence
  is unavailable.

- [ ] **T4.A1.1** RED-test missing actors, scenarios, success criteria, and
  explicit non-goals.

- [ ] **T4.A1.2** Emit measurable target behavior and acceptance boundaries for
  features, bugs, pages, layouts, runtime work, and composite requests.

- [ ] **T4.A2.1** RED-test orphan nodes, illegal level jumps, missing ownership,
  and missing failure/recovery leaves.

- [ ] **T4.A2.2** Implement the canonical chain: initiative -> experience/
  workflow -> page -> region/layout -> component -> interaction/behavior ->
  state transition -> failure/recovery.

- [ ] **T4.A2.3** Preserve parent/child traceability and stable identifiers.

- [ ] **T4.A3.1** RED-test omitted required dimensions, duplicate ownership,
  unresolved contradictions, and unjustified conditional omissions.

- [ ] **T4.A3.2** Cover product; behavior; page/input; layout/capability;
  component/tokens; data authority; runtime/API/event/PTY; failure/security/
  resources; persistence/migration; platform/accessibility; performance; and
  integration/license dimensions.

- [ ] **T4.A3.3** Record evidence, confidence, conflicts, and dependency edges.

- [ ] **T4.A4.1** RED-test single-option conclusions without explicit
  justification and visual-only pattern matching.

- [ ] **T4.A4.2** Produce alternative concepts, reusable patterns, rejected
  options, tradeoffs, capability fallbacks, and reversibility notes.

- [ ] **T4.A5.1** RED-test stale/absent graph evidence and `ready`-only
  freshness claims.

- [ ] **T4.A5.2** Map current owners, call/data paths, protocol/persistence
  boundaries, existing tests, and reuse candidates.

- [ ] **T4.A5.3** Emit current-versus-target architectural and functional fit.

- [ ] **T4.A6.1** RED-test unresolved conflicts and unsupported go decisions.

- [ ] **T4.A6.2** Select target architecture, behavior, data flow, fallbacks,
  budgets, and `go`, `conditional_go`, `no_go`, or `blocked` status.

- [ ] **T4.A7.1** RED-test incomplete traceability, missing decision evidence,
  conditional gaps, and mutable handoff fields.

- [ ] **T4.A7.2** Emit and validate immutable `change-intent-package.json`.

- [ ] **T4.A7.3** Prove native, reference-adapted, composite, no-go, blocked,
  and mid-flight packages through fixtures/evals.

- [ ] **T4.A7.4** Verify that A7 readiness still grants no product mutation.

- [ ] **T5.I0** Reject absent/invalid handoff, unapproved target, stale current
  state, or missing product authorization; accept mid-flight evidence only
  after provenance and current-phase classification.

- [ ] **T5.I1** Generate PRD, authority checklist, risk register, non-goals,
  rollback, compatibility, and migration obligations.

- [ ] **T5.I2** Refresh graph/repository evidence and detect drift between A7
  handoff and the live target.

- [ ] **T5.I3** Freeze current behavior, target behavior, semantic diff,
  retained behavior, change strategy, and ownership impact.

- [ ] **T5.I4** Build the test-point catalog with `what`, `current`, `expected`,
  `diff`, `result`, and `reason` for every applicable obligation.

- [ ] **T5.I5** Produce dependency-ordered implementation slices, test slices,
  commit boundaries, rollback points, and owned file sets.

- [ ] **T5.I6** Capture characterization evidence before moving behavior or
  architecture.

- [ ] **T5.I7** Require an observed behavior-specific RED; reject compile,
  environment, setup, flaky, or already-green false REDs.

- [ ] **T5.I8** Implement the minimum GREEN change and preserve exact command
  output as evidence.

- [ ] **T5.I9** Refactor only behind green tests; enforce local ownership and
  invariants.

- [ ] **T5.I10** Run cross-layer and cross-feature tests across all applicable
  families.

- [ ] **T5.I11** Verify failure, recovery, security, resources, capability
  negotiation, and degraded behavior.

- [ ] **T5.I12** Verify declared latency, allocation, throughput, memory, queue,
  and terminal-render budgets with calibrated fixtures.

- [ ] **T5.I13** Run complete repository, platform, protocol, migration,
  dependency, docs, and release-cadence gates applicable to the change.

- [ ] **T5.I14** Audit evidence, targeted staging, atomic commits, allowed
  publication, remote SHA, graph reindex, and current-symbol freshness.

- [ ] **T6.1** Server/runtime truth versus TUI/client projection.

- [ ] **T6.2** Snapshot/event ordering, revision, replay, duplicate, gap, and
  slow-subscriber behavior.

- [ ] **T6.3** PTY/terminal chunk boundaries, UTF-8/ANSI splits, queue pressure,
  resize, EOF, detach/reattach, and multi-pane throughput.

- [ ] **T6.4** Plugin host timeouts, crashes, output bounds, process cleanup,
  malformed data, version compatibility, and path confinement.

- [ ] **T6.5** Page/layout/component keyboard, mouse, focus, modal, resize,
  Unicode, narrow viewport, empty/loading/error, and terminal fallback states.

- [ ] **T6.6** Persistence interruption, corruption, migration, disk-full,
  concurrent owner, quota, and large-scrollback behavior.

- [ ] **T6.7** Platform isolation and Linux/macOS/Windows policy differences.

- [ ] **T6.8** Performance regression, slow-client isolation, soak, task leak,
  zombie process, and chaos behavior.

- [ ] **T6.9** Backward/forward protocol, old/new client, old/new plugin, and
  old persisted-state compatibility.

- [ ] **T7.1** P14 Ratatui/reference-project adapter.

- [ ] **T7.2** Native feature fixture.

- [ ] **T7.3** Mid-flight file-manager feature plus bugfix fixture.

- [ ] **T7.4** Page and interaction-flow fixture.

- [ ] **T7.5** Responsive layout and tiling fixture.

- [ ] **T7.6** Design-system/component/token fixture.

- [ ] **T7.7** Runtime capability and protocol fixture.

- [ ] **T7.8** Composite multi-dimension conflict fixture.

- [ ] **T7.9** Explicit no-go and blocked-MCP fixtures.

- [ ] **T7.10** Unauthorized delivery fixture proving I0 rejection.

- [ ] **T7.11** Verify that native mode invents no repository/source/license and
  reference mode omits no source/provenance/license obligations.

- [ ] **T8.1** Focused schema/validator unit tests.

- [ ] **T8.2** Complete tests for both skills and all negative fixtures.

- [ ] **T8.3** JSON parse, schema, stable-ID, version, and deterministic-output
  checks.

- [ ] **T8.4** Skill validation, README/AGENTS/SKILL consistency, and lesson
  format checks.

- [ ] **T8.5** Eval coverage for A0-A7, I0-I14, adapters, mid-flight adoption,
  blocked, no-go, and unauthorized paths.

- [ ] **T8.6** Legacy P0-P14 backward-compatibility verification.

- [ ] **T8.7** Product isolation and exact diff-boundary verification.

- [ ] **T8.8** Placeholder, whitespace, broken-link, and untracked-artifact
  scans.

- [ ] **T8.9** Proportional `just check` or documented exact equivalent.

- [ ] **T9.1** Preserve each baseline, RED, GREEN, refactor, governance, fixture,
  and evidence concern in reviewable atomic commits.

- [ ] **T9.2** Target-stage only the declared owned files and verify the staged
  name list before every commit.

- [ ] **T9.3** Fetch and prove fast-forward ancestry before any authorized push.

- [ ] **T9.4** Push only the permitted CyPack feature branch/ref; never
  `upstream`, never force.

- [ ] **T9.5** Verify exact local/remote SHA equality after publication.

- [ ] **T9.6** Reindex Codebase Memory after committed implementation changes.

- [ ] **T9.7** Record node/edge counts and query current pipeline/module symbols;
  never infer freshness from `ready` alone.

- [ ] **T10.1** Run one native page/feature request through A0-A7 without
  product mutation.

- [ ] **T10.2** Run one reference project through P0-P14 -> adapter -> A7.

- [ ] **T10.3** Run one mid-flight file-manager feature/bugfix adoption and
  prove existing evidence preservation plus remaining-gate enforcement.

- [ ] **T10.4** Run one composite conflict to a justified conditional-go/no-go.

- [ ] **T10.5** Prove unauthorized I0 rejection and blocked-MCP truthfulness.

- [ ] **T10.6** If separately authorized, run one non-product fixture through
  I14 before using the pipeline on Herdr product code.

- [ ] **T10.7** Record new errors, golden paths, edge cases, and any cross-skill
  lessons in the required tables.

- [ ] **T10.8** Update this registry, `.codex/CURRENT.md`, `.codex/TASKS.md`, and
  handoff state with exact final evidence and next action.

- [ ] **T10.9** Perform final self-review: requirements, tests, failure paths,
  Git state, publication state, graph freshness, and remaining blockers.

### End of Machine-Exact Task Copy

<!-- OPEN_TASKS_END -->

## 9. REFERANSLAR

Canonical:

- `AGENTS.md`
- `CLAUDE.md`
- `.codex/BOOTSTRAP.md`
- `.codex/CURRENT.md`
- `.codex/TASKS.md`
- `.codex/CHANGE-PIPELINE-TASKS.md`
- `.codex/HANDOFF.md`
- `.codex/MEMORY.md`
- `.planning/STATE.md`
- `.codex/NEXT-SESSION-PROMPT.md`

FIP:

- `docs/superpowers/specs/2026-07-17-herdr-files-interaction-polish-design.md`
- 55 unique `TP-FIP-*` IDs in that design

Safety and prior closure:

- `.local/ISOLATED-DEV-TEST.md`
- `.codex/evidence/miller-production-progress.md`
- `.codex/evidence/fm5-preview-placement-decision.md`
- `.codex/evidence/native-fm-completion-audit.md`
- `.codex/evidence/fm1-miller-viewport-progress.md`

Skills:

- `.codex/skills/herdr-native-fm/SKILL.md`
- `.codex/skills/herdr-native-fm/lessons/`
- `/home/ayaz/.codex/skills/session-handoff/SKILL.md`
- `/home/ayaz/.codex/skills/superpowers/writing-plans/SKILL.md` or the
  currently installed canonical equivalent exposed in the skill catalog
- `rust-dev` plus lessons before Rust work

MCP:

- Codebase Memory project `home-ayaz-projects-herdr`
- graph tools: `get_architecture`, `search_graph`, `trace_path`,
  `get_code_snippet`, `query_graph`, `index_repository`

## 10. FAIL-CLOSED DURDURMA KOŞULLARI

Agent şu koşullarda uygulama yazmayı durdurur ve gerçeği raporlar:

- task counts 54/89/143 ile uyuşmuyor;
- HEAD/remote ancestry beklenmedik;
- tracked unrelated dirty product diff var;
- `.superpowers/` ownership sınırı ihlal riski var;
- Codebase Memory recent symbol döndürmüyor;
- required skill/lesson okunamıyor;
- behavior-specific RED beklenen nedenle fail etmiyor;
- üç ardışık fix denemesi aynı blocker’da sonuç vermiyor;
- Playwright/Chromium bulunmuyor ve visual gate çalışmıyor;
- stable socket/process isolation kanıtlanamıyor;
- full gate veya platform gate fail;
- push target upstream veya non-FF görünüyor.

Bir gate fail olursa fail’i saklama, bypass etme veya “muhtemelen flake” deme.
Kök nedeni ayır, gerekiyorsa değişikliği geri al, yeni test ekle ve fresh
evidence üret.

## 11. TAMAMLANMA TANIMI

FIP ancak şu koşulların tamamı kanıtlandığında kapanır:

- 55 `TP-FIP-*` noktası traceable ve green;
- default visible Files click gerçek Native Files Stage’i açar;
- exact resident child highlight her chain/failure durumunda doğru;
- entry kinds/icons semantic, deterministic, pure ve font-fallback-safe;
- explicit agent picker current/other live agents için doğru;
- file/directory path exact bytes only, no CR/LF/Enter/submit;
- tüm stale/identity/path/control/backpressure vakaları zero-byte fail closed;
- Playwright Chromium, isolated mouse ve PTY byte tests green;
- full Rust/Linux/Windows/Bun/Python/maintenance gates green;
- performance/resource/invariant budgets green;
- stable runtime/socket untouched ve residue zero;
- graph fresh ve changed call/data-flow seams tekrar okunmuş;
- atomic commit chain ve clean tracked tree doğrulanmış;
- CyPack iki ref exact local SHA ile eşit;
- canonical continuity/evidence/lessons güncel.

## 12. BAŞLATMA TETİKLEYİCİSİ

Fresh session’da:

```text
Herdr Files Interaction Polish programına kanonik handoff’tan devam et.
AGENTS.md, CLAUDE.md ve .codex/NEXT-SESSION-PROMPT.md içindeki mandatory start
order’ı atlamadan uygula. Codebase Memory graph-first çalış; 54 ürün + 89
paused pipeline = 143 açık görevi exact continuation satırlarıyla yeniden say
ve session task listesine aktar. Yalnız FIP-G.1’i in_progress yap. Onaylı
tasarımı superpowers:writing-plans ile code-level RED/GREEN plana dönüştür,
FIP-G.2 graph/test/gate reconciliation bitmeden Rust yazma. Stable Herdr/socket
ve user-owned .superpowers/ alanına dokunma. Upstream’a push etme.
```
