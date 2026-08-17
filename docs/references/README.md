---
doc: herdr-references-registry
domain: _index + rust-engineering
created: 2026-07-12
updated: 2026-07-24  # domain index eklendi (document-rendering, custom-layout)
status: canonical — çıplak iddia yok; her giriş tier + confidence taşır (evidence-propagation uyumlu)
git_note: >
  /docs/* herdr .gitignore'da IGNORED (yalnız /docs/next/ istisna) → bu dizin LOKAL yaşar,
  upstream'e/PR'a SIZMAZ (external-contributor guardrail'e bilinçli uyum). Kayıp riskine karşı
  makine-kopyası: ~/.cartography/herdr-rust-engineering-SYSTEM-MAP.json
agentic_triggers:
  - "herdr geliştirme · herdr feature · herdr bug · herdr refactor · herdr test"
  - "rust pattern · rust prensip · rust mimari · production grade rust"
  - "hangi kaynak · referans · nereden bakayım · best practice rust"
related:
  - docs/patterns/rust-engineering.md            # pattern kataloğu (bu registry'nin damıtılmış hâli)
  - ~/.claude/skills/rust-dev/                   # GLOBAL Rust katmanı (skill + workflows + lessons)
  - .local/CURRENT-HANDOFF.md                    # aktif iş durumu (kanonik handoff pointer'ı)
  - .cartography/rust-engineering-SYSTEM-MAP.json # evidence graph
---

# herdr Referans Registry

## 📇 DOMAIN INDEX — hangi konu hangi dosyada

| Domain | Registry | Pattern kataloğu | Pattern ID uzayı | Analiz | Evidence graph |
|---|---|---|---|---|---|
| **rust-engineering** | ⬇️ bu dosya (aşağısı) | `docs/patterns/rust-engineering.md` | `HP1–HP14` + global `P1–P25` | — | `.cartography/rust-engineering-SYSTEM-MAP.json` |
| **tui-composition** | `docs/references/tui-composition.md` | `docs/patterns/tui-composition.md` | `TC*` | — | `.cartography/tui-composition-SYSTEM-MAP.json` |
| **native-file-manager** | `docs/references/native-file-manager.md` | `docs/patterns/native-file-manager.md` | `FM*` | — | `.cartography/herdr-fm-capability-SYSTEM-MAP.json` |
| **document-rendering** | `docs/references/document-rendering.md` | `docs/patterns/document-rendering.md` | **`DR1–DR18`** + anti-pattern **`DA1–DA12`** | `docs/analysis/2026-07-24-document-render-{internal-state,ecosystem}.md` | `.cartography/document-rendering-SYSTEM-MAP.json` |
| **custom-layout** | `docs/references/custom-layout.md` | `docs/patterns/custom-layout.md` | **`CL*`** | `docs/analysis/2026-07-24-custom-layout-state.md` | *(B1 artefaktı — henüz üretilmedi)* |
| **pty-ipc-runtime** | `docs/references/pty-ipc-runtime.md` | `docs/patterns/pty-ipc-runtime.md` | **`PI1–PI19`** + anti-pattern **`PA1–PA11`** | *(havuz: `~/.cartography/refpool/` 17 repo)* | `.cartography/pty-ipc-runtime-SYSTEM-MAP.json` |
| **remote-media-transport** | `docs/references/remote-media-transport.md` | `docs/patterns/remote-media-transport.md` | **`RM1–RM10`** + anti-pattern **`RA1–RA7`** | *(registry içinde §7.5 ölçümler · §7.6 endüstri · §7.7 herdr-browser yüzeyi)* | *(F0'da üretilecek)* |
| **architecture-seams** | *(analiz içinde)* | — | — | `docs/analysis/2026-07-24-architecture-seams.md` | — |
| **vision-mission** | *(analiz içinde)* | — | — | `docs/analysis/2026-07-24-vision-mission-state.md` | — |
| **session-continuity** | *(analiz içinde)* | — | — | `docs/analysis/2026-07-24-chat-forensics-codex-cursor-handover.md` | — |

> **Pattern ID çakışma kuralı:** Yeni domain açan agent, ID önekini bu tablodan seçer ve BURAYA yazar.
> Mevcut önekler: `HP`, `P`, `TC`, `FM`, `DR`, `DA`, `CL`, `PI`, `PA`, `RM`. Aynı öneki iki domain KULLANAMAZ.

> **Prior-art havuzu (`~/.cartography/refpool/`).** Dış referans repo'ları buraya **shallow
> klonlanır** ve `codebase-memory-mcp` ile indekslenir → registry satırları çıplak iddia değil,
> `search_graph(project="home-user-.cartography-refpool-<ad>")` ile canlı koda çözülür.
> ⚠️ Havuza herdr'dan **türemiş** bir proje eklenirse (`zynk` gibi) `derivative` işaretlenir ve
> θ-kuralında bağımsız ikinci kaynak olarak **sayılmaz** (sahte konsensüs).

**Analiz havuzu girişi:** `docs/analysis/README.md` — okuma sırası, kayıt kuralı, kalıcılık politikası.
**Karar sentezi:** `docs/analysis/2026-07-24-decision-matrix-and-roadmaps.md` — öncelik tablosu + dört şeridin yol haritaları + edit alternatifleri.

---

## DOMAIN: rust-engineering

> Bu repo için "hangi iddia hangi kaynağa dayanıyor" tablosu. Yeni derin araştırma yapan HER agent
> bulduğu kaynağı BURAYA (veya kendi domain dosyasına) ekler ([[reference-registry]] 5-adım). Tier sözlüğü:
> `official` (dil/araç resmî docs) · `official-tool` (aracın kendi sitesi) · `source_code`
> (bu reponun canlı kodu/config'i — en güçlü yerel kanıt) · `project-memory` (kanıtlı oturum dersi) ·
> `executable` (çalıştırılmış komut çıktısı).

## Birincil yerel kaynaklar (bu repo)

| Etiket | Kaynak | Tier | Conf | Desteklediği pattern | Konu |
|---|---|---|---|---|---|
| `[herdr-agents]` | `AGENTS.md` (CLAUDE.md → symlink; UPSTREAM DOSYASI — DÜZENLEME) | source_code | 0.9 | HP1–HP10 (patterns kataloğu) | Mimari prensipler, test/commit/konvansiyon kuralları |
| `[herdr-justfile]` | `justfile` (~15 recipe) | source_code | 0.9 | HP5 (lint kapısı), HP6 (windows-lint) | lint/ci/check/release-docs-check enforce zinciri |
| `[herdr-ci]` | `.github/workflows/ci.yml` | source_code | 0.9 | HP7 (commit gate) | conventional-commits job + 3-OS matrix + toolchain pin |
| `[herdr-toolchain]` | `rust-toolchain.toml` (1.96.1 + clippy + rustfmt) | source_code | 0.95 | HP5 | Reproducible toolchain pin |
| `[herdr-clippy-cfg]` | `clippy.toml` (`too-many-arguments-threshold = 11`) | source_code | 0.95 | HP5 | Proje-özel lint eşiği |
| `[herdr-vendor-index]` | `vendor/libghostty-vt.vendor.json` + `vendor/libghostty-vt.patches.md` | source_code | 0.9 | HP8 (vendored patch disiplini) | C-vendoring izlenebilirliği |
| `[herdr-tests]` | `tests/` (9 integration dosyası + support/mod.rs 747 satır + fixtures; nextest 2578 test) | source_code | 0.9 | HP3, HP4 | Test altyapısı olgunluğu |
| `[herdr-contributing]` | `CONTRIBUTING.md` | source_code | 0.9 | HP10 (fork guardrail) | External-contributor süreci (issue→approve→PR) |
| `[herdr-prd]` | `.local/prd/projects-files-sidebar-poc.md` | source_code (lokal) | 0.85 | — | Aktif feature'ın tek-gerçek-kaynağı |
| `[herdr-handoff]` | `.local/CURRENT-HANDOFF.md` → güncel session handoff | source_code (lokal) | 0.85 | — | Aktif iş durumu (drift-korumalı pointer) |
| `[herdr-fmn-2026-07-21]` | `.codex/evidence/files-performance-fix-closure-and-navigation-followups.md` + `.codex/references/yazi-file-manager-performance-transfer.md` | source_code + executable | 0.95 | HP11 | Cursor/activation ayrımı, bounded stale-safe preview, measured wheel normalization, human acceptance |
| `[herdr-fmh-2026-07-22]` | `src/app/input/file_manager.rs` FMH RED/GREEN + `.codex/evidence/files-performance-fix-closure-and-navigation-followups.md` | source_code + executable | 0.98 | HP12 | Left one-edge; Right directory-only; file/stale/boundary inert; render-neutral failure path |
| `[herdr-ffo-2026-07-22]` | `.codex/evidence/files-focus-ownership-closure.md` + FFO commits `3c5f94e4..d85d610e` | source_code + executable | 0.99 | HP13 | Rail/Trail owner transfer, current-state action authority, shared active cursor, deterministic VIS-26/VIS-27 |
| `[herdr-dclick-2026-07-23]` | `.codex/evidence/files-directory-click-focus-closure.md` + `TrailSnapshots::focus_entry` route | source_code + executable | 0.99 | HP11, HP12, HP14 | Primary directory click exact owner focus; bounded preview cannot steal child focus; Right-first-child |
| `[yazi-fm-transfer-2026-07-21]` | `.codex/references/yazi-file-manager-performance-transfer.md` | pinned source study + source_code | 0.95 | HP11, HP13 | Source-verified Yazi mechanisms and explicit Herdr transfer/rejection decisions; no inferred feature claims |

## Kanıtlı oturum dersleri (memory)

| Etiket | Kaynak | Tier | Conf | Desteklediği pattern | Konu |
|---|---|---|---|---|---|
| `[herdr-discipline-memory]` | `~/.claude/projects/-home-user/memory/feedback-herdr-fork-production-discipline.md` | project-memory | 0.85 | HP3 (nextest), çalışma disiplini | nextest v0.9.140 kurulumu; `cargo test` paralel flaky kök-neden KANITI (seri 2531/2531 vs nextest 2533/2533); per-feature branch kuralı; no happy-path/token-cimriliği-yok çalışma tarzı |

## Resmî dış kaynaklar (2026-07-12 curl canlılık-doğrulandı, HTTP 200)

| Etiket | Kaynak | Tier | Conf | Desteklediği pattern | Konu |
|---|---|---|---|---|---|
| `[rust-book]` | https://doc.rust-lang.org/book/ | official | 0.95 | P2, P10, P21, P23 (global katalog) | Ownership, error handling, concurrency |
| `[nomicon]` | https://doc.rust-lang.org/nomicon/ | official | 0.95 | P9, P19 | unsafe sözleşmeleri, Drop semantiği |
| `[api-guidelines]` | https://rust-lang.github.io/api-guidelines/ | official | 0.95 | P16, P18, P20 | Public API checklist |
| `[clippy-lints]` | https://rust-lang.github.io/rust-clippy/master/index.html | official | 0.95 | HP5, P12 | Lint kataloğu |
| `[nextest]` | https://nexte.st/ | official-tool | 0.9 | HP3, P15 | Process-izole test runner (doctest sınırı dahil) |
| `[miri]` | https://github.com/rust-lang/miri | official | 0.9 | P9 | UB detector (nightly) |
| `[cargo-book]` | https://doc.rust-lang.org/cargo/ | official | 0.95 | P24, P25 | Workspace/profil/feature |
| `[rust-patterns]` | https://rust-unofficial.github.io/patterns/ | authoritative | 0.85 | P16–P19 | Design pattern kataloğu |
| `[rustsec]` | https://rustsec.org/ | official-db | 0.9 | P6 | Advisory DB (pkg-registry MCP kaynağı) |
| `[edition-guide]` | https://doc.rust-lang.org/edition-guide/ | official | 0.95 | — | Edition migration |
| `[tokio-tutorial]` | https://tokio.rs/tokio/tutorial | official-tool | 0.9 | P22, P23 | Async sınır disiplini |
| `[std-docs]` | https://doc.rust-lang.org/std/ | official | 0.95 | — | Std API referansı |

## Araç katmanı (bu makinede kanıtlı)

| Etiket | Kaynak | Tier | Conf | Konu |
|---|---|---|---|---|
| `[sys-toolchain]` | rustc/cargo 1.96.1 + rust-analyzer + cargo-nextest v0.9.140 + cargo-miri + ast-grep (`~/.cargo/bin`) | executable | 1.0 | 2026-07-12 `rustc --version`/`rustup show`/`ls ~/.cargo/bin` çıktılarıyla doğrulandı |
| `[codebase-mcp]` | codebase-memory-mcp, index `home-user-projects-herdr` | executable | 0.9 | Sembol keşfi ZORUNLU yol; `detect_changes=0` tek başına tazelik değildir. 2026-07-22 doc-aware single-worker CLI store 24,327 node / 129,874 edge ile FFO sembollerini çözdü; long-lived built-in channel'ın daha eski sayımı ayrı ve stale olarak etiketlendi |
| `[pkg-registry-mcp]` | pkg-registry MCP cargo tool'ları | executable (bağlantı) | 0.9 | Crate meta + RustSec advisory sorgusu |

## Kayıt kuralı (yeni kaynak eklerken)

1. Etiket ver (`[kebab-case]`), tabloya satır ekle — tier + confidence ZORUNLU.
2. URL ise canlılık doğrula (`curl -o /dev/null -w '%{http_code}'`) — ölü/uydurma URL YASAK;
   fetch-edilemeyeni `⚠️ bulunamadı` işaretle.
3. Kaynak bir pattern'i destekliyorsa `docs/patterns/rust-engineering.md`'deki pattern ID'sini yaz.
4. Harita bağlantısı: `.cartography/rust-engineering-SYSTEM-MAP.json`'a claim/evidence olarak işle.

---
*v1.0.0 — 2026-07-12 · reference-registry 5-adım pipeline'ın Adım-1 artefaktı.*
