---
doc: herdr-references-registry
domain: document-rendering
created: 2026-07-24
status: canonical — çıplak iddia yok; her giriş tier + confidence taşır (evidence-propagation uyumlu)
git_note: >
  /docs/* herdr .gitignore'da IGNORED (yalnız /docs/next/ istisna) → bu dizin LOKAL yaşar,
  upstream'e/PR'a SIZMAZ (external-contributor guardrail'e bilinçli uyum). Kayıp riskine karşı
  makine-kopyası: ~/.cartography/herdr-document-rendering-SYSTEM-MAP.json
agentic_triggers:
  - "png · jpeg · görsel önizleme · image preview · kitty graphics · sixel · protokol"
  - "xlsx · excel · spreadsheet · tablo · csv · calamine · hücre · formül"
  - "pdf · belge · document viewer · pdftoppm · pdfium · lopdf"
  - "terminal excel · terminal tablo · sanal kaydırma · dondurulmuş kolon"
  - "dosya render · dosya edit · preview provider · plugin previewer · fallback"
related:
  - docs/patterns/document-rendering.md              # pattern kataloğu (bu registry'nin damıtılmış hâli)
  - docs/analysis/2026-07-24-document-render-ecosystem.md  # tam analiz (bu registry'nin kaynağı)
  - docs/references/README.md                        # rust-engineering registry (kardeş domain)
  - .cartography/document-rendering-SYSTEM-MAP.json  # evidence graph
---

# herdr Referans Registry — DOMAIN: document-rendering

> "Hangi iddia hangi kaynağa dayanıyor" tablosu. Belge/görsel render konusunda derin araştırma yapan
> HER agent bulduğu kaynağı BURAYA ekler ([[reference-registry]] 5-adım).
>
> **Tier sözlüğü:**
> `official` (protokol/dil resmî spesifikasyonu) · `official-registry` (crates.io v1 API — sürüm/bakım
> gerçeği) · `official-docs` (docs.rs API yüzeyi) · `official-api` (GitHub Search API) ·
> `source-code` (canlı kod/config — **en güçlü kanıt**) · `source-repo` (proje README) ·
> `source-docs` (bu reponun kendi docs'u) · `authoritative` (tanınmış ikincil kaynak).
>
> **Doğrulama tarihi:** 2026-07-24. Sürüm/bakım değerleri zamanla bayatlar — yeniden doğrulama
> reçetesi: `docs/analysis/2026-07-24-document-render-ecosystem.md` §K.

## Birincil yerel kaynaklar (bu repo — en güçlü kanıt)

| Etiket | Kaynak/URL | Tier | Conf | Desteklediği pattern | Konu |
|---|---|---|---|---|---|
| `[herdr-preview-capability]` | `src/fm/preview_capability.rs` | source-code | 1.0 | DR8, DR14 | **En kritik dosya.** `PreviewCapability` enum (NativeText/NativeImage/MetadataOnly/OptionalPlugin/Unsupported); satır 126-140: xlsx/xls/ods/pdf/doc/docx/ppt/pptx → `OptionalPlugin`; `plugin_or_fallback` (`:181`) zarif düşüş |
| `[herdr-kitty-graphics]` | `src/kitty_graphics.rs` (2075 satır) | source-code | 1.0 | DR2, DR4, DR5 | `KITTY_CHUNK_BYTES=3072`, `HOST_IMAGE_ID_BASE=10_000`, `FILE_MANAGER_PREVIEW_IMAGE_ID=1`, `HostCellSize::from_terminal` (`:34`), `HostViewKey` (`:69`) |
| `[herdr-image-limits]` | `src/fm/image_preview.rs:11-15` (734 satır toplam) | source-code | 1.0 | DR6 | 5 katmanlı kaynak sınırı: encoded 64 MiB · dimension 32_768 · pixels 64 Mpx · decoded 256 MiB · output 64 MiB + tipli hatalar |
| `[herdr-preview-worker]` | `src/app/file_preview_worker.rs:75-88` (1508 satır) | source-code | 1.0 | DR7 | `generation.wrapping_add(1).max(1)` + `accepts()`; `FilePreviewKey{files_generation:u32, preview_generation:u64}` çift-generation stale-reject |
| `[herdr-image-worker]` | `src/app/image_preview_worker.rs` (999 satır) | source-code | 0.95 | DR6, DR7 | Görsel önizleme worker'ı (bounded) |
| `[herdr-ghostty-bindings]` | `src/ghostty/mod.rs:185,208,227` | source-code | 1.0 | DR3 | `KittyImageFormat`, `KittyImageDescriptor`, `KittyPlacementRenderInfo` — mutlak yerleştirme muhasebesi |
| `[herdr-vendor-kitty]` | `vendor/libghostty-vt/src/terminal/kitty/graphics.zig`, `graphics_command.zig`, `include/ghostty/vt/kitty_graphics.h` | source-code | 0.95 | DR1, DR13 | VT katmanı **yalnızca Kitty** destekliyor; `find vendor -ipath "*sixel*"` → **boş** (sixel YOK) |
| `[herdr-config-experimental]` | `src/config/model.rs:1018,1879-1902` | source-code | 1.0 | DR1 | `experimental.kitty_graphics: bool`; test `kitty_graphics_default_off_and_parse` → **varsayılan false** |
| `[herdr-fm-layout]` | `src/ui/file_manager.rs:129,141,156,172,218` | source-code | 1.0 | DR9, DR11 | Miller yerleşimi `[parent|div|current|div|preview]`; `file_manager_preview_content_area()` |
| `[herdr-fm-miller]` | `src/app/file_manager_miller.rs:220,254,321` | source-code | 0.95 | DR11 | `MillerResizeColumnId::Preview` — preview kolonu yeniden boyutlandırılabilir |
| `[herdr-cargo]` | `Cargo.toml:47-48` | source-code | 1.0 | DR6 | `image 0.25.10` (png/jpeg/gif/webp) + `png 0.17` + `syntect 5.3.0` + `unicode-width 0.2` + `base64 0.22.1` **zaten bağımlılık** |
| `[herdr-windows-beta]` | `docs/next/website/src/content/docs/windows-beta.mdx:50,52,57,59` | source-docs | 1.0 | DR1, DR13 | *"Kitty graphics rendering \| unverified"*; *"Leave `experimental.kitty_graphics = false` unless you are specifically testing"*; clipboard-image Windows'ta bağlı değil |
| `[herdr-plugin-contract]` | `docs/next/website/src/content/docs/plugins.mdx:55-75,225-246` | source-docs | 1.0 | DR8, DR14 | argv komut sözleşmesi; env: `HERDR_PLUGIN_ACTION_ID`, `HERDR_PLUGIN_CONTEXT_JSON`, `HERDR_PLUGIN_ROOT/CONFIG_DIR/STATE_DIR`; manifest `platforms = ["linux","macos","windows"]` |
| `[herdr-plugin-runtime]` | `src/app/api/plugins/runtime.rs:19,55-56` + `src/app/api/plugins/mod.rs:185,521-557` | source-code | 0.95 | DR8 | Plugin action çözümleme + env enjeksiyonu |
| `[herdr-text-preview]` | `src/fm/text_preview.rs` (307 satır) | source-code | 0.9 | DR10 | Mevcut metin önizleme yolu (CSV bugün buraya düşüyor) |

## Terminal grafik protokolü (resmî spesifikasyon)

| Etiket | Kaynak/URL | Tier | Conf | Desteklediği pattern | Konu |
|---|---|---|---|---|---|
| `[kitty-graphics-spec]` | https://raw.githubusercontent.com/kovidgoyal/kitty/master/docs/graphics-protocol.rst | official | 0.95 | DR2, DR3, DR4, DR5 | `a=d` silme + `d=i/I/a/A/c/C/p/P/z/Z` (küçük harf=yalnız yerleşim, BÜYÜK=veriyi de serbest bırakır) · `U=1` sanal yerleşim + **`U+10EEEE`** placeholder · `t=d/f/t/s` iletim · chunk ≤**4096 B**, son hariç **4'ün katı**, `m=1/m=0` · `z=` 32-bit z-index (negatif metnin altı) · *"images must be scrolled along with text"* |
| `[kitty-graphics-canonical]` | https://sw.kovidgoyal.net/kitty/graphics-protocol/ | official | — | — | ⚠️ **doğrulanamadı** (DNS timeout 2026-07-24) — içerik `[kitty-graphics-spec]` üzerinden alındı |

## Rust crate'leri — görsel (crates.io v1 API, 2026-07-24)

| Etiket | Kaynak/URL | Tier | Conf | Desteklediği pattern | Konu |
|---|---|---|---|---|---|
| `[ratatui-image-crate]` | https://crates.io/api/v1/crates/ratatui-image | official-registry | 0.95 | DR1 | v**11.0.6** · 2026-06-25 · MIT · 593.358 indirme (242.832 son dönem) · kitty+sixel+iterm2+halfblock |
| `[ratatui-image-src]` | `/home/ayaz/.cartography/refpool/ratatui-image/` (yerel indeks + disk) | source-code | 1.0 | DR1, DR7 | `Picker.from_query_stdio` (`src/picker.rs:94`), `from_query_stdio_with_options` (`:106-165`), `query_stdio_capabilities` (`:441-482`), `query_with_timeout` (`:560-598`), `cap_parser.Parser.push` (`cap_parser.rs:140-261`), `detect_tmux_and_outer_protocol_from_env` (`:296-325`), `interpret_parser_responses` (`:484-553`), `test_from_query_stdio_no_hang` (`:622`), `SlicedImage.slice_rows` (`sliced.rs`), `ResizeEncodeRender.needs_resize` |
| `[ratatui-image-cargo]` | `refpool/ratatui-image/Cargo.toml` | source-code | 1.0 | DR1, DR13 | Bağımlılıklar: `image`, `icy_sixel`, `base64-simd`, `rand`, `ratatui ^0.30.1`, `self_cell`, `thiserror`; unix `rustix ^0.38`, **windows `windows 0.58`** (herdr `windows-sys 0.61.2` ile ÇAKIŞIR); rust-version 1.86, edition 2024; feature `chafa-dyn`/`chafa-static` (LGPL C kütüphanesi) |
| `[icy-sixel-crate]` | https://crates.io/api/v1/crates/icy_sixel | official-registry | 0.95 | DR13 | v0.5.0 · 2025-12-27 · MIT/Apache · 690.369 indirme · saf Rust SIXEL |
| `[viuer-crate]` | https://crates.io/api/v1/crates/viuer | official-registry | 0.95 | — | v0.11.0 · **2025-12-09** (durgun) · MIT · 1.073.734 indirme |

## Rust crate'leri — elektronik tablo

| Etiket | Kaynak/URL | Tier | Conf | Desteklediği pattern | Konu |
|---|---|---|---|---|---|
| `[calamine-crate]` | https://crates.io/api/v1/crates/calamine | official-registry | 0.95 | DR9, DR10 | v**0.36.0** · 2026-07-06 · MIT · **10.190.660** indirme (2.997.944 son dönem) · saf Rust · Rust 1.88, edition 2021, ~12.309 satır/17 dosya |
| `[calamine-api]` | https://docs.rs/calamine/latest/calamine/ | official-docs | 0.9 | DR9, DR10, DR12 | `open_workbook()`, `open_workbook_auto()`, `Xlsx`/`Ods`/`Xls`/`Xlsb`; `Reader::worksheet_range()` → `Range`; **`worksheet_formula()`** → `XlsxCellFormula` (önbellek değeri + formül metni); `Data`/`DataRef` enum. **YAZMA YOK — salt-okuma** |
| `[umya-crate]` | https://crates.io/api/v1/crates/umya-spreadsheet | official-registry | 0.95 | DR15 | v**3.0.1** · 2026-07-13 · MIT · 870.928 indirme · saf Rust |
| `[umya-api]` | https://docs.rs/umya-spreadsheet/latest/umya_spreadsheet/ | official-docs | **0.75** | DR15 | `reader::xlsx::read(path)` → `sheet_by_name_mut()` → `cell_mut("A1").set_value()/set_value_number()/set_value_bool()` → `writer::xlsx::write(&book, path)`. ⚠️ **formül/grafik/pivot korunumu BELGELENMEMİŞ** — POC şartı |
| `[rust-xlsxwriter-crate]` | https://crates.io/api/v1/crates/rust_xlsxwriter | official-registry | 0.95 | — | v**0.96.0** · 2026-07-01 · MIT/Apache · 3.087.885 indirme · **yalnızca sıfırdan yazma** |

## Rust crate'leri — PDF ve düzenleme

| Etiket | Kaynak/URL | Tier | Conf | Desteklediği pattern | Konu |
|---|---|---|---|---|---|
| `[pdfium-render-crate]` | https://crates.io/api/v1/crates/pdfium-render | official-registry | 0.95 | DR16 | v**0.9.3** · 2026-07-14 · MIT/Apache · 1.716.750 indirme · Chromium PDFium sarmalayıcı — **harici binary/DLL gerektirir** |
| `[lopdf-crate]` | https://crates.io/api/v1/crates/lopdf | official-registry | 0.95 | DR16 | v**0.44.0** · 2026-07-10 · MIT · 13.068.721 indirme · saf Rust PDF nesne modeli — **rasterleştirmez** |
| `[edtui-crate]` | https://crates.io/api/v1/crates/edtui | official-registry | 0.95 | DR15 | v**0.11.6** · **2026-07-18** (çok aktif) · MIT · 220.591 indirme · vim-esinli ratatui editör |
| `[tui-textarea-crate]` | https://crates.io/api/v1/crates/tui-textarea | official-registry | 0.95 | DR15 | v0.7.0 · **2024-10-22** (🔴 21 ay durgun) · MIT · 2.156.644 indirme · ratatui 0.30 uyumu şüpheli |

## Dosya yöneticisi önizleme mimarileri (yerel refpool — transfer edilebilir desenler)

| Etiket | Kaynak/URL | Tier | Conf | Desteklediği pattern | Konu |
|---|---|---|---|---|---|
| `[yazi-pdf-plugin]` | `refpool/yazi-src/yazi-plugin/preset/plugins/pdf.lua` (55 satır, tamamı okundu) | source-code | 1.0 | DR16, DR17, DR18 | `pdftoppm -f N -l N -singlefile -jpeg -jpegopt quality=..` **tek sayfa raster**; `M:peek`/`M:seek`/`M:preload` sözleşmesi; `job.skip`=sayfa; stderr'den `"the last page %((%d+)%)"` ile sayfa sınırı → `upper_bound=true`; `ya.file_cache(job)` + `fs.cha(cache)` cache atlama; `ya.sleep(image_delay/1000 …)` debounce |
| `[yazi-preview-config]` | `refpool/yazi-src/yazi-config/preset/yazi-default.toml` | source-code | 1.0 | DR5, DR6 | `image_delay=30` · `image_quality=75` · `image_filter="triangle"` · `image_alloc=536870912` (512MB) · `image_bound=[10000,10000]`; previewer eşlemeleri (`application/pdf → pdf`, `image/* → image`, `image/svg+xml → svg`, `image/{avif,hei?,jxl} → magick`, `application/{json,ndjson} → json`) |
| `[yazi-preview-cfg-rs]` | `refpool/yazi-src/yazi-config/src/preview/preview.rs:73,85` | source-code | 1.0 | DR5, DR6 | `deserialize_image_delay`, `deserialize_image_quality` |
| `[yazi-scheduler]` | `refpool/yazi-src/yazi-scheduler/src/scheduler.rs:53` + `ongoing.rs:28` + `yazi-actor/src/tasks/cancel.rs:15` | source-code | 1.0 | DR7 | `Scheduler.cancel`, `Ongoing.cancel`, `Cancel.act` — merkezi iptal mimarisi |
| `[yazi-plugin-utils]` | `refpool/yazi-src/yazi-plugin/src/utils/image.rs:10,20,30` + `preview.rs:15,36` + `tasks.rs:8` | source-code | 0.95 | DR6, DR17 | `image_info`, `image_show`, `image_precache`, `preview_code`, `preview_widget`, `task` |
| `[yazi-previewer-cfg]` | `refpool/yazi-src/yazi-config/src/plugin/previewer.rs:24-36,51` | source-code | 0.95 | DR8 | `Previewer{url_pat, mime_pat, any_file, any_dir}` + `PreviewerMatcher` — eşleme sözleşmesi |
| `[joshuto-preview-script]` | `refpool/joshuto/src/config/preview/preview_option_raw.rs:19` | source-code | 1.0 | DR8 | `preview_script: Option<String>` — harici script sözleşmesi (en basit uzantı modeli) |
| `[superfile-src]` | `refpool/superfile-src/src/internal/` (model.go, model_render.go, common/) | source-code | 0.8 | — | Go/Bubble Tea dosya yöneticisi — karşılaştırma referansı |

## Terminal spreadsheet projeleri (GitHub)

| Etiket | Kaynak/URL | Tier | Conf | Desteklediği pattern | Konu |
|---|---|---|---|---|---|
| `[gh-terminal-spreadsheets]` | https://api.github.com/search/repositories?q=terminal+spreadsheet+in:name,description&sort=stars | official-api | 0.95 | DR9–DR12 | 20 repo taraması: yıldız/dil/lisans/`archived`/`pushed_at` — ekosistem haritası |
| `[csvlens-repo]` | https://github.com/YS-L/csvlens | source-repo | 0.9 | **DR9, DR10, DR11, DR12** | 🎯 **UX şablonu.** v0.12.0, MIT, Rust 1.88+. Regex arama+vurgulama · satır/kolon regex filtre · kolon genişliği + **soldan kolon dondurma** · **TAB ile satır/kolon/hücre seçim modu** · doğal sıralama · hücre kopyalama · line wrap · **hücre editi YOK** |
| `[cell-repo]` | https://github.com/garritfra/cell | source-repo | 0.9 | **DR9, DR15** | 🎯 **Mimari şablon.** Rust+ratatui, MIT, 312★. `cell-sheet-core/` (*"Data model, formula engine, file I/O (no TUI dependency)"*) + `cell-sheet-tui/` (*"Ratatui rendering, Vim modes, event loop"*). Formüller SUM/AVERAGE/COUNT/MIN/MAX/IF; CSV/TSV/`.cell`; vim modları; headless `--read/--write/--eval` |
| `[cell-core-crate]` | https://crates.io/api/v1/crates/cell-sheet-core | official-registry | 0.95 | DR9 | v0.5.1 · 2026-06-30 · MIT · 465 indirme · 11 sürüm (Nisan 2026'dan beri) · release-plz |
| `[tshts-repo]` | https://github.com/SamuelSchlesinger/tshts | source-repo | 0.9 | **DR12** (maliyet kanıtı) | Rust+ratatui+crossterm, MIT, 43★. **60+ formül fonksiyonu** (sayısal/koşullu/lookup/string/tarih/web/mantık/info) + AST tabanlı döngü tespiti + bağımlılık yeniden inşası + mutlak referans (`$A$1`). Native JSON `.tshts`. → **Formül motorunun ayrı bir ürün olduğunun kanıtı** |
| `[sc-im-repo]` | https://github.com/andmarti1424/sc-im | source-repo | 0.9 | DR12, DR15 | C/ncurses, 5655★, `NOASSERTION` lisans, push 2026-07-20. Vim hareketleri · UNDO/REDO · **65.536 satır × 702 kolon** · CSV/TAB/**XLSX import+export**, ODS import, Markdown export · LUA scripting. Opsiyonel bağımlılık: `libxml-2.0`+`libzip` (xlsx/ods). **Windows dokümante değil** |
| `[visidata-repo]` | https://github.com/saulpw/visidata | source-repo | 0.9 | DR10 | Python, 9199★, **GPL-3.0**, push 2026-07-15. *"terminal interface for exploring and arranging tabular data"*; tsv/csv/sqlite/json/**xlsx**/hdf5. **Platform: *"Linux, OS/X, or Windows (with WSL)"* → Windows native YOK** |
| `[tv-repo]` | https://github.com/alexhallam/tv | source-repo | 0.9 | **DR11** | Rust, MIT/Unlicense, v1.6.1. Tek-atış yazıcı (TUI değil). csv/parquet/feather/ipc · *"Automatic large file streaming (>5MB)"* · **significant digit printing** · NA renklendirme · **column overflow logic** · Unicode truncation · dotfile config |
| `[sheets-go-repo]` | https://github.com/maaslalani/sheets | official-api | 0.9 | — | Go, 2287★, MIT, push 2026-07-18 — Bubble Tea tabanlı |

## Terminal emülatörü yetenekleri

| Etiket | Kaynak/URL | Tier | Conf | Desteklediği pattern | Konu |
|---|---|---|---|---|---|
| `[wezterm-imgcat]` | https://wezterm.org/imgcat.html | official-docs | 0.85 | DR1 | iTerm2 inline images ✅ + `doNotMoveCursor=1` uzantısı. ⚠️ *"The image protocol isn't fully handled by multiplexer sessions at this time"* — multiplexer uyarısı herdr için doğrudan ilgili. Sixel/Kitty desteği **bu sayfada belirtilmemiş** |
| `[wt-release-1.22]` | https://github.com/microsoft/terminal/releases/tag/v1.22.10731.0 | source-repo | 0.6 | DR13 | Bu yama sürümünde sixel/kitty **bahsi yok** (deadlock fix, ConPTY stabilite, JP çeviri). ⚠️ Yama notu — özellik sorusuna kesin cevap DEĞİL |
| `[wt-release-notes-1.22]` | https://learn.microsoft.com/en-us/windows/terminal/release-notes/1.22 | official-docs | — | DR13 | ⚠️ **doğrulanamadı** (DNS timeout 2026-07-24) — **Windows Terminal Sixel desteği AÇIK SORU** |
| `[wt-sixel-doc]` | https://github.com/microsoft/terminal/blob/main/doc/reference/Sixel.md | — | — | DR13 | ⚠️ **doğrulanamadı** (HTTP 404 — dosya yok) |

## Araç katmanı (bu araştırmada kanıtlı)

| Etiket | Kaynak | Tier | Conf | Konu |
|---|---|---|---|---|
| `[webfetch-crates-io]` | `WebFetch` → `https://crates.io/api/v1/crates/<ad>` | executable | 1.0 | 🟢 12/12 başarılı. Sürüm/bakım/indirme/lisans için **kanonik yol** |
| `[webfetch-gh-api]` | `WebFetch` → `https://api.github.com/search/repositories?q=…` | executable | 1.0 | 🟢 Ekosistem taraması; `pushed_at`+`archived`+`license.spdx_id` üçlüsü zorunlu |
| `[codebase-mcp-refpool]` | codebase-memory-mcp, indeksler: `home-ayaz-.cartography-refpool-{ratatui-image,yazi-src,joshuto,superfile-src,…}` | executable | 0.9 | 🟢 `get_architecture` + `search_graph` ile sembol bul → **sonra diskten tam kod oku** (en verimli kombinasyon) |
| `[pkg-registry-mcp-fail]` | `mcp__pkg-registry__get-cargo-package-details` | executable | 1.0 | 🔴 **4/4 başarısız** ("Error fetching package details") 2026-07-24. Alternatif: crates.io v1 API doğrudan |
| `[websearch-fail]` | `WebSearch` | executable | 1.0 | 🔴 **Oturum boyunca bozuk** (`output_config.effort 'xhigh' is not supported`). Alternatif: WebFetch + birincil API'ler |

---

## Kayıt kuralı (yeni kaynak eklerken)

1. Etiket ver (`[kebab-case]`), tabloya satır ekle — **tier + confidence ZORUNLU**.
2. URL ise canlılık doğrula; ölü/uydurma URL YASAK. Fetch edilemeyeni **`⚠️ doğrulanamadı` işaretle
   ve SATIRDA BIRAK** — silme. Bir sonraki tur oradan devam eder.
3. Kaynak bir pattern'i destekliyorsa `docs/patterns/document-rendering.md`'deki ID'yi (DR1–DR18,
   DA1–DA12) yaz.
4. Harita bağlantısı: `.cartography/document-rendering-SYSTEM-MAP.json`'a claim/evidence olarak işle.
5. Sürüm/bakım verisi **bayatlar** — yeniden doğrulama reçetesi:
   `docs/analysis/2026-07-24-document-render-ecosystem.md` §K.2.

## Doğrulama borcu (bir sonraki tur için açık kalemler)

| # | Ne doğrulanacak | Neden önemli | Nasıl |
|---|---|---|---|
| 1 | Windows Terminal Sixel desteği | herdr Windows grafik stratejisi | `learn.microsoft.com` erişimi VEYA `microsoft/terminal` release listesi taraması |
| 2 | `umya-spreadsheet` formül/grafik/pivot korunumu | Aşama 2.2 (XLSX yazma) veri kaybı riski | Kaynak kod incelemesi + gerçek dosyayla POC |
| 3 | WezTerm sixel/kitty desteği | Çoklu-terminal fallback stratejisi | WezTerm docs'un diğer sayfaları |
| 4 | `cell-sheet-core` API yüzeyi | Formül motoru gerekirse hazır çözüm olabilir | docs.rs + repo kaynak |
| 5 | §J'deki tüm aday projeler (termimad, delta, resvg, arrow…) | Gelecek domain turları | §K.2 reçetesi |

---
*v1.0.0 — 2026-07-24 · reference-registry 5-adım pipeline Adım-1 artefaktı.*
*Kaynak analiz: `docs/analysis/2026-07-24-document-render-ecosystem.md` ·*
*Damıtılmış katalog: `docs/patterns/document-rendering.md`*
