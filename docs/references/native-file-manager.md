---
doc: herdr-references-registry
domain: native-file-manager
created: 2026-07-13
status: canonical — çıplak iddia yok; her giriş tier + confidence taşır (evidence-propagation uyumlu)
git_note: >
  /docs/* herdr .gitignore'da IGNORED (yalnız /docs/next/ istisna) → LOKAL yaşar, upstream'e SIZMAZ.
  Makine-kopyası: ~/.cartography/native-fm-references-SYSTEM-MAP.json + refpool klonları.
agentic_triggers:
  - "native file manager · files tab · dizin gezgini · file explorer · file browser"
  - "image preview · dosya önizleme · thumbnail · kitty graphics · miller column"
  - "ratatui örnek · file manager örnek · nasıl yapılmış · referans repo"
related:
  - docs/patterns/native-file-manager.md          # pattern kataloğu (bu registry'nin damıtılmışı)
  - .local/prd/native-file-manager-DECISION.md     # karar + katman-katman plan
  - .local/prd/native-fm-BACKBONE-ARCHITECTURE.md  # omurga katman haritası
  - ~/.cartography/{yazi-tui-core,yazi-fs-model,yazi-preview,herdr-tui-arch,herdr-existing-fm,native-fm-references}-SYSTEM-MAP.json
example_pool_root: ~/.cartography/refpool/   # KALICI klonlar (kod okuma için) — /tmp DEĞİL
---

# herdr Referans Registry — DOMAIN: native-file-manager

> herdr-native dosya-yöneticisi + image-preview inşası için "hangi teknik hangi kaynakta kanıtlı" tablosu.
> Uygulamaya geçmeden ÖNCE bu havuzu oku (kullanıcı direktifi: örnek-proje havuzunu öncele).
> Tier: `official` · `source_code` (canlı kod, en güçlü) · `spec` · `executable` (gh api / komut çıktısı).

## 🗂️ ÖRNEK-PROJE HAVUZU (KALICI klon + codebase-mcp indexli — kod okuyarak öğren)

| repo | yol (`~/.cartography/refpool/`) | codebase-mcp projesi | commit | NE İÇİN (birebir kaynak) |
|---|---|---|---|---|
| **joshuto** ⭐ | `joshuto/` | `home-ayaz-.cartography-refpool-joshuto` | d2581fb | **Miller 3-kolon (parent/current/preview) + preview-kolonu-içi image** — `src/ui/views/tui_folder_view.rs` `Constraint[3]`+`Direction::Horizontal`+`Ratio(0)`=gizle |
| **ratatui-image** ⭐ | `ratatui-image/` | `home-ayaz-.cartography-refpool-ratatui-image` | 109e700 | **Kitty encoder + U+10EEEE unicode-placeholder (mux-safe)** — `src/protocol/kitty.rs:157-271`; ayrıca sixel/iterm2/halfblocks + Picker auto-detect |
| **yeet** | `yeet/` | `home-ayaz-.cartography-refpool-yeet` | dbe6b1c | ratatui+ratatui-image+chafa-fallback+tokio-async+Lua-config canlı glue; modal(vim) |
| **rat-commander** | `rat-commander/` | `home-ayaz-.cartography-refpool-rat-commander` | 99b9791 | **per-slot Gfx protocol cache** (her frame image-id yeniden göndermeme, perf) — `src/ui/graphics/mod.rs`; MC-tarzı 2-panel |
| yazi | `yazi/` | (ana index: `tmp-yazi-src` /tmp/yazi-src) | 4dab480 | async-io+preview altın-standardı; `yazi-adapter` kendi Driver/EMULATOR-tespit soyutlaması |
| tui-file-explorer | `tui-file-explorer/` | (indexlenmedi) | bb00b63 | küçük/temiz API-yüzeyi ref ⚠️ "miller" iddiası ABARTILI (kaynak=2-panel `dual_pane.rs`) |
| ratatui-explorer | `ratatui-explorer/` | (indexlenmedi) | c518bdf | minimal ratatui file-widget iskeleti (image yok) |

**Kullanım:** `mcp__codebase-memory-mcp__get_architecture(<proje>)` → `search_graph`/`get_code_snippet` ile ilişkisel keşif;
veya klonu doğrudan Read/Grep. Yeni örnek eklenirse: klonla → `~/.cartography/refpool/` → `index_repository` → buraya satır ekle.

## 🖼️ Kitty Graphics / image-in-terminal (protokol + crate) — tier/conf

| kaynak | url | emit/parse | mux-safe? | tier | conf |
|---|---|---|---|---|---|
| Kitty graphics spec | sw.kovidgoyal.net/kitty/graphics-protocol/ | spec | unicode-placeholder = EVET | spec | 0.95 |
| ratatui-image `kitty.rs` | github.com/ratatui/ratatui-image | EMIT (stateful) | ✅ U+10EEEE virtual placement (kaynak-doğrulandı) | source_code | 0.95 |
| yazi-adapter | github.com/sxyazi/yazi/tree/main/yazi-adapter | EMIT (Driver/Emulator) | ✅ Kgp virtual | source_code | 0.9 |
| viuer / viu | github.com/atanunq/viuer · /viu | EMIT "dump" | ❌ mux-UNSAFE (ham escape) | official | 0.9 |
| notcurses-rs | github.com/dankamongmen/notcurses-rs | EMIT (FFI) | kısmi | official | 0.85 |
| termwiz (wezterm) | github.com/wezterm/wezterm | EMIT (iterm2+sixel) | — (monorepo, orantısız) | official | 0.85 |

## 🦀 FM-omurga crate'leri (eklenecek)

`image` (decode/resize) · `ratatui-image` (opsiyonel — encoder herdr'da var) · `nucleo` (fuzzy, helix-editor, 0.90) ·
`notify`+`notify-debouncer-*` (0.85) · `ignore`(BurntSushi)/`walkdir` (0.85). **yazi-vendor:** `natsort`·`Image::downscale`·`Scrollable`·`Line::truncate`.

## 📁 FM mimari referansları (ranked — native-fm-references-SYSTEM-MAP tam liste)

joshuto (0.95, miller+image) · yazi (0.95, async+preview) · yeet (0.9) · rat-commander (0.9, gfx-cache) ·
ratatui-explorer (0.9, minimal) · xplr (0.9, durgun) · felix (0.85, chafa-harici) · broot (0.82, panel+Kitty) ·
television (0.86, tokio+ratatui+nucleo) · tere (0.92, minimal-nav).

## ⚠️ KANITLI BOŞLUK (= fırsatımız)
ratatui ekosisteminde **N≥3 cascading Finder-tarzı miller browser YOK** (repo-hunter awesome-ratatui + çoklu arama
taradı). joshuto 3-kolon yapıyor ama sabit; gerçek N-cascading yok. → herdr-native FM burada özgün değer katabilir.

---
*Kaynak: repo-hunter agent (20 repo, gh-api doğrulu) + native-fm-references cartography (2026-07-13). Kardeş: docs/patterns/native-file-manager.md.*
