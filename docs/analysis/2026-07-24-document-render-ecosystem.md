---
doc: herdr-analysis
domain: document-rendering
subject: dış ekosistem araştırması (terminal görsel/spreadsheet/PDF render + edit)
created: 2026-07-24
method: WebFetch birincil kaynaklar (crates.io v1 API, GitHub Search API, raw.githubusercontent) + codebase-memory-mcp refpool + herdr kaynak okuma
status: canonical — her iddia (claim, evidence, confidence); çıplak iddia yok
git_note: >
  /docs/* herdr .gitignore'da IGNORED (yalnız /docs/next/ istisna) → lokal yaşar, upstream'e sızmaz
  (external-contributor guardrail). Makine kopyası: ~/.cartography/herdr-document-rendering-*
agentic_triggers:
  - "png · jpeg · görsel önizleme · image preview · kitty graphics · sixel"
  - "xlsx · excel · spreadsheet · tablo · csv · calamine"
  - "pdf · belge · document viewer · pdftoppm · pdfium"
  - "terminal excel · terminal tablo · hücre editi · formül"
  - "dosya render · dosya edit · preview provider · plugin previewer"
related:
  - docs/references/document-rendering.md
  - docs/patterns/document-rendering.md
  - docs/analysis/2026-07-24-document-render-internal-state.md
  - docs/analysis/2026-07-24-decision-matrix-and-roadmaps.md
---

# Terminalde Belge Render/Edit — Dış Ekosistem Araştırması

> **Araştırma tarihi:** 2026-07-24 · **Kapsam:** salt OKUMA/ARAŞTIRMA (bu turda herdr koduna
> dokunulmadı, paket kurulmadı, git mutasyonu yapılmadı).
> **Kanıt sözleşmesi:** her iddia `(claim, evidence, confidence)`. Erişilemeyen kaynak
> `⚠️ doğrulanamadı` işaretlidir — içeriği UYDURULMAMIŞTIR.
> **Sürüm/tarih değerleri** 2026-07-24 itibarıyla crates.io v1 API'sinden alınmıştır; ileride
> yeniden doğrulanmalıdır (§K "Karşılaştırma metodolojisi" reçetesi ile).

---

## 0. YÖNETİCİ ÖZETİ — Araştırmanın zeminini değiştiren bulgu

Araştırmaya "herdr'a sıfırdan görsel render eklenecek" varsayımıyla başlandı. **Bu varsayım
yanlıştı.** herdr kaynak okuması şunu gösterdi:

| Bulgu | Kanıt | Confidence |
|---|---|---|
| herdr'da **çalışan Kitty grafik implementasyonu var** (2075 satır) | `src/kitty_graphics.rs` | 1.0 (kaynak kod) |
| `image` crate **zaten bağımlılık** (png/jpeg/gif/webp) + ayrıca `png 0.17` | `Cargo.toml:47-48` | 1.0 |
| **`xlsx`, `xls`, `ods`, `pdf`, `docx`, `pptx` ZATEN uzantı noktasına bağlı** — `PreviewCapability::OptionalPlugin` | `src/fm/preview_capability.rs:126-140` | 1.0 |
| Ama **hiç plugin yok** → bu dosyalar bugün `MetadataOnly(DocumentMetadata)` = "optional document viewer" yazısına düşüyor | `preview_capability.rs:36` + test `manual.pdf → MetadataOnly` | 1.0 |
| Kitty grafik **varsayılan KAPALI, deneysel** | `src/config/model.rs:1879` testi: `assert!(!config.experimental.kitty_graphics)` | 1.0 |
| Windows'ta Kitty grafik durumu herdr'ın **kendi dokümanında "unverified"** | `docs/next/website/src/content/docs/windows-beta.mdx:52` | 1.0 |
| Vendored libghostty-vt'de **sixel YOK**, sadece kitty | `find vendor -ipath "*sixel*"` → boş; `vendor/libghostty-vt/src/terminal/kitty/graphics.zig` var | 0.95 |

**Sonuç:** Doğru sorular şunlardır:
1. Zaten var olan `OptionalPlugin` yolunu **XLSX/PDF için nasıl doldururuz** (Rust bağımlılığı eklemeden),
2. Kitty grafiği **deneysel'den çıkarmak** için ekosistem ne öğretiyor,
3. **Edit** nerede başlar, maliyeti ne.

---

## A. ADAY TEKNOLOJİ TABLOSU

### A.1 Görsel render (Rust)

| Ad | Ne yapar | Lisans | Son sürüm + tarih | Ağırlık | Windows | herdr uyumu | Kaynak |
|---|---|---|---|---|---|---|---|
| **`image`** | Decode/encode/resize (PNG,JPEG,GIF,WEBP) | MIT/Apache | 0.25.10 (kullanımda) | **zaten var** | ✅ | **5/5** — mevcut | `Cargo.toml:47` |
| **`ratatui-image`** | Kitty+Sixel+iTerm2+halfblock widget, capability probe | MIT | **11.0.6** · 2026-06-25 | Orta: `icy_sixel`,`base64-simd`,`rand`,`self_cell`,`thiserror` + win: `windows 0.58` | ✅ (cfg'li) | **2/5** (bağımlılık) / **5/5** (referans) | crates.io + `refpool/ratatui-image/Cargo.toml` |
| **`icy_sixel`** | Saf Rust SIXEL encoder/decoder | MIT/Apache | 0.5.0 · 2025-12-27 | Hafif | ✅ | **1/5 bugün** — Ghostty VT sixel parse etmiyor | crates.io |
| **`viuer`** | Terminale görsel bas (kütüphane) | MIT | 0.11.0 · **2025-12-09** (7 ay durgun) | Orta | kısmi | **1/5** — kendi stdout'una yazar, herdr pane modeline ters | crates.io |
| **chafa (C, FFI)** | Unicode/ANSI'ye yüksek kaliteli düşürme | LGPL-3 | ratatui-image `chafa-dyn` feature | **Sistem C kütüphanesi** | zayıf | **1/5** — C bağımlılığı + LGPL | `refpool/ratatui-image/Cargo.toml:26` |

> ⚠️ `ratatui-image` Windows'ta `windows 0.58` kullanıyor; herdr `windows-sys 0.61.2` kullanıyor
> → eklenirse **iki ayrı Windows binding ailesi** taşınır.

### A.2 Elektronik tablo (XLSX) — Rust zinciri

| Ad | Ne yapar | Lisans | Son sürüm + tarih | İndirme | Windows | herdr uyumu | Kaynak |
|---|---|---|---|---|---|---|---|
| **`calamine`** | **OKU**: xlsx/xlsb/xls/ods, saf Rust, `worksheet_formula()` | MIT | **0.36.0** · 2026-07-06 | 10.19M | ✅ saf Rust | **5/5** | crates.io + docs.rs |
| **`rust_xlsxwriter`** | **YAZ**: sıfırdan xlsx üret (mevcut dosyayı düzenleyemez) | MIT/Apache | **0.96.0** · 2026-07-01 | 3.09M | ✅ | **2/5** | crates.io |
| **`umya-spreadsheet`** | **OKU→DEĞİŞTİR→YAZ** | MIT | **3.0.1** · 2026-07-13 | 870K | ✅ saf Rust | **4/5** ⚠️ formül korunumu belgelenmemiş | crates.io + docs.rs |
| `polars`/`arrow` | Kolonar analiz | MIT | — | — | ✅ | **0/5** — devasa, ihtiyaç görüntüleme | — |

### A.3 PDF — Rust zinciri

| Ad | Ne yapar | Lisans | Son sürüm | Windows | herdr uyumu |
|---|---|---|---|---|---|
| **`pdfium-render`** | Chromium PDFium sarmalayıcı — **gerçek raster** | MIT/Apache | **0.9.3** · 2026-07-14 | ✅ ama **harici pdfium binary/DLL** | **2/5** — 4 platform binary dağıtımı, tek-binary modelini bozar |
| **`lopdf`** | PDF nesne modeli (**rasterleştirmez**) | MIT | **0.44.0** · 2026-07-10 | ✅ saf Rust | **3/5** — metadata/sayfa sayısı OK |
| **`pdftoppm` (poppler, harici)** | PDF sayfa → JPEG/PNG | GPL (ayrı süreç) | sistem paketi | ⚠️ Windows'ta manuel | **5/5 (plugin olarak)** |

### A.4 Metin/hücre düzenleme widget'ları

| Ad | Lisans | Son sürüm | Bakım | herdr uyumu |
|---|---|---|---|---|
| **`edtui`** | MIT | **0.11.6** · **2026-07-18** | 🟢 çok aktif | **4/5** — vim-modlu, ratatui-native |
| `tui-textarea` | MIT | 0.7.0 · **2024-10-22** | 🔴 **21 aydır durgun**, ratatui 0.30 uyumu şüpheli | **1/5** |

### A.5 Terminal spreadsheet projeleri (GitHub Search API, confidence 0.95)

| Proje | Dil | ★ | Lisans | Son push | Edit? | Formül? | herdr'a ders |
|---|---|---|---|---|---|---|---|
| **saulpw/visidata** | Python | **9199** | GPL-3.0 | 2026-07-15 | ✅ | kısmi | UX referansı; **Windows sadece WSL** |
| **andmarti1424/sc-im** | C | **5655** | NOASSERTION | 2026-07-20 | ✅ | ✅ tam | xlsx I/O opsiyonel (libxml2+libzip); **Windows dokümante değil** |
| **maaslalani/sheets** | Go | **2287** | MIT | 2026-07-18 | ✅ | sınırlı | Bubble Tea |
| **garritfra/cell** | **Rust** | **312** | MIT | 2026-07-20 | ✅ | ✅ SUM/AVG/COUNT/MIN/MAX/IF | 🎯 **EN DEĞERLİ** — ratatui + core/TUI ayrımı |
| **SamuelSchlesinger/tshts** | **Rust** | 43 | MIT | 2026-05-28 | ✅ | ✅ 60+ fonksiyon | Formül motoru maliyetinin kanıtı |
| **YS-L/csvlens** | Rust | — | MIT | — | ❌ | ❌ | 🎯 Sanal kaydırma + dondurulmuş kolon UX |
| **alexhallam/tv** | Rust | — | MIT/Unlicense | — | ❌ | ❌ | Kolon hizalama/anlamlı basamak |
| freakout42/macrocalc | C | 32 | BSD-2 | 2026-04-08 | ✅ | ✅ | Lotus 1-2-3 uyumlu; niş |
| oaklandgit/vizigo | Go | 10 | MIT | 2026-05-02 | ✅ | ? | Küçük; izlenebilir |
| only-using-ai/rustxl | Rust | 7 | — | 2026-02-23 | ? | ? | Lisans YOK → kullanılamaz |
| SreeAditya-Dev/Cello-TUI | Rust | 4 | MIT | 2026-07-21 | ✅ | ✅ + AI | Çok yeni, olgunlaşmamış |
| xi/spreadsheet | Python | 6 | MIT | 2026-05-08 | ✅ | ? | Minimal referans |
| clintmoyer/sheets | C | 5 | GPL-3.0 | 2026-02-20 | ✅ | ? | Minimal vim-tuşlu |
| SheetJS/wk | TypeScript | 22 | Apache-2.0 | **2020-02-26** | ❌ | ❌ | 🔴 Terk edilmiş |

---

## B. TERMİNAL GÖRSEL RENDER — MİMARİ DESEN KARTLARI

> Damıtılmış hâli: `docs/patterns/document-rendering.md` (DR1–DR7).
> Kitty protokol detayları `raw.githubusercontent.com/kovidgoyal/kitty/master/docs/graphics-protocol.rst`
> üzerinden **birebir alıntıyla** doğrulandı (official, 0.95).

### P01 — Protokol seçimi ve yetenek sondajı
**Ne:** Terminalin desteklediği protokolü çalışma anında sor, sabit varsayma.
`ratatui-image` zinciri (gerçek kod kanıtı):
- `Picker.from_query_stdio` (`src/picker.rs:94`), `from_query_stdio_with_options` (`:106-165`)
- `query_stdio_capabilities` (`:441-482`) → `query_with_timeout` (`:560-598`)
- `cap_parser.Parser.push` (`cap_parser.rs:140-261`) — kısmi/çöp yanıt toleransı
- `detect_tmux_and_outer_protocol_from_env` (`:296-325`) — tmux ayrı eksen
- `iterm2_from_env` (`:327-345`), `enable_raw_mode` (`:370-410`), `font_size_fallback` (`:431-433`)
- `interpret_parser_responses` (`:484-553`), `Capability` enum (`:30-43`), `QueryResult` (`:555-559`)
- Test: `test_from_query_stdio_no_hang` (`:622`) — **sondaj asla asılmamalı**

**KULLAN:** Terminal kimliği bilinmiyorsa (SSH, tmux, bilinmeyen emülatör).
**KULLANMA:** herdr gibi kendi VT'sini taşıyan uygulamanın **iç** pane'i için. Sondaj sadece
**dış host terminale** karşı gereklidir.
**herdr'a ders:** `HostCellSize::from_terminal` (`kitty_graphics.rs:34`) `crossterm::terminal::window_size()`
kullanıp başarısızlıkta 8×16 px'e düşüyor — doğru desen; ama protokol sondajı YOK.
`experimental` bayrağını kaldırmadan önce kopyalanacak asıl parça budur.

### P02 — Yerleştirme ve temizleme (en sık hata)
Kitty'de görsel bir kez iletilir (`i=<id>`), ayrı yerleştirilir (`p=<placement_id>`). Silme `a=d`:

| Anahtar | Anlam |
|---|---|
| `d=i`/`d=I` | Belirtilen id'li görseller |
| `d=a`/`d=A` | Tüm görünür yerleşimler |
| `d=c`/`d=C` | İmleç konumunu kesen yerleşimler |
| `d=p`/`d=P` | `x`,`y` hücresindeki yerleşimler |
| `d=z`/`d=Z` | Belirtilen z-index'tekiler |

Resmî alıntı: *"The lowercase variant only deletes the images without necessarily freeing up the
stored image data... The uppercase variants will delete the image data as well."*

**KULLAN:** küçük harf (`d=i`) — aynı görseli tekrar göstereceksen (yeniden iletim maliyeti sıfır).
**KULLANMA:** küçük harfi bellek temizliği sanmak = terminal tarafında sızıntı.
**herdr:** `HOST_IMAGE_ID_BASE = 10_000` ile id uzayı ayrılmış, `FILE_MANAGER_PREVIEW_IMAGE_ID = 1`,
`FILE_MANAGER_PREVIEW_PLACEMENT_ID = 1`, `FILE_MANAGER_PREVIEW_PANE_RAW = u32::MAX` — agent
pane'lerinin görselleriyle çakışmayı önleyen doğru izolasyon.

### P03 — Unicode placeholder / sanal yerleştirme
`<ESC>_Ga=p,U=1,i=<image_id>,c=<columns>,r=<rows><ESC>\` ile sanal yerleşim; sonra ekrana
**`U+10EEEE`** placeholder karakteri diakritiklerle basılır — satır/kolon diakritiklerde, görsel id
ön-plan renginde taşınır.
**KULLAN:** görsel metin akışıyla kaymalıysa; multiplexer/pane sistemleri için en sağlam.
**KULLANMA:** piksel-hassas mutlak konum gerekiyorsa; terminal desteği daha dar.
**herdr:** `KittyPlacementRenderInfo` (`src/ghostty/mod.rs:227`) ile mutlak yerleştirme seçilmiş —
Miller kolonlarında doğru karar.

### P04 — Parçalama ve iletim ortamı
`t=d` (doğrudan base64), `t=f` (dosya), `t=t` (geçici dosya, oto-silinir), `t=s` (paylaşımlı bellek).
Doğrudan iletimde chunk ≤ **4096 bayt**, *"All chunks, except the last, must have a size that is a
multiple of 4"*, `m=1` devam / `m=0` bitiş.
**herdr durumu:** `KITTY_CHUNK_BYTES = 3072` (`kitty_graphics.rs:21`) → 4096 altında ve 4×768.
✅ **Protokole uygun, doğrulandı.**
**KULLANMA:** `t=f`/`t=s` SSH'ta çalışmaz (terminal ile uygulama farklı makinede). herdr uzaktan
kullanıldığı için `t=d` doğru varsayılan.

### P05 — Kaynak sınırlama (decompression bomb)
**herdr — örnek alınacak seviyede** (`src/fm/image_preview.rs:11-15`):
```
MAX_ENCODED_BYTES = 64 MiB · MAX_DIMENSION = 32_768 · MAX_PIXELS = 64 Mpx
MAX_DECODED_BYTES = 256 MiB · MAX_OUTPUT_BYTES = 64 MiB
```
Beş ayrı kapı + tipli hatalar (`EncodedTooLarge`, `DimensionsTooLarge`, `PixelCountTooLarge`,
`DecodedBytesTooLarge`, `OutputTooLarge`). yazi'nin tek-kapılı modelinden **daha titiz**.

yazi karşılaştırma (`yazi-config/preset/yazi-default.toml`):
`image_delay=30`, `image_quality=75`, `image_filter="triangle"`, `image_alloc=536870912` (512 MB),
`image_bound=[10000,10000]`.
**herdr'da EKSİK:** `image_delay` muadili **debounce** ve `image_quality` muadili
**kalite/bant genişliği ayarı**.

### P06 — SSH / uzak kullanımda bant genişliği (herdr için birinci sınıf sorun)
**Problem büyüklüğü (hesap):** Kitty `t=d` base64 → **%33 şişme**. 1920×1080 RGBA ham 8.3 MB →
base64 ≈ 11 MB. 10 Mbit/s SSH'ta **~9 sn**, üstelik pane çıktısı aynı kanalı paylaşır. Klasörde ok
tuşuyla gezinme her satırda bunu tekrarlar.

| # | Teknik | Kazanç | Maliyet | Kanıt |
|---|---|---|---|---|
| 1 | **İletmeden ÖNCE hedef alana downscale** | 10–100× | yok | herdr `HostCellSize` × preview `Rect` zaten hesaplı |
| 2 | **Debounce** (yazi 30 ms) | gezinmede %90+ iptal | 30 ms gecikme | `preview.rs:73` `deserialize_image_delay` |
| 3 | **PNG yerine JPEG** uzakta | 3–10× | kalite | yazi `image_quality=75`, `preview.rs:85` |
| 4 | **İçerik-hash cache + `d=i`** | tekrar ziyarette %100 | terminal belleği | Kitty P02; herdr'da `DefaultHasher` importu **zaten var** (`kitty_graphics.rs:1`) |
| 5 | **Uzakta grafiği kapat → metin/metadata** | %100 | görsel yok | `PreviewFallback` enum'u **zaten mevcut** |

**Öneri:** `PreviewFallback` bu iş için hazır; eksik olan tek şey **uzak oturum tespiti → otomatik
düşük kalite/kapalı politikası**. Yeni bağımlılık gerektirmez.

### P07 — Async, iptal ve bayat-sonuç reddi
| Sistem | Mekanizma | Kanıt |
|---|---|---|
| yazi | `Scheduler.cancel` (`yazi-scheduler/src/scheduler.rs:53`), `Ongoing.cancel` (`ongoing.rs:28`), `Cancel.act` (`yazi-actor/src/tasks/cancel.rs:15`) | refpool kod |
| yazi plugin | `peek`/`seek`/`preload` + `only_if = job.file.url` | `pdf.lua:9` |
| **herdr** | **generation + accepts()** | `file_preview_worker.rs:75-88` |

herdr gerçek kod:
```rust
self.generation = self.generation.wrapping_add(1).max(1);   // :75
fn accepts(&self, generation: u64, key: &FilePreviewKey) -> bool {
    self.generation == generation && self.active.as_ref() == Some(key)   // :86-88
}
```
`FilePreviewKey` hem `files_generation: u32` hem `preview_generation: u64` taşıyor → dizin
yenilendiğinde de bayat önizleme reddediliyor. yazi'nin `only_if` garantisiyle eşdeğer, üstelik
tip düzeyinde.

**En önemli mimari tavsiye:** Yeni belge türleri (XLSX/PDF) **bu mevcut worker'a takılmalı**;
paralel yol açılmamalı.

---

## C. TERMİNAL EXCEL / SPREADSHEET — DERİN BÖLÜM

### C.1 Mevcut projelerin analizi

#### C.1.1 sc-im (C, 5655★)
README birebir: *"Vim movements commands for editing cell content"*, *"UNDO / REDO"*,
*"65.536 rows and 702 columns supported"*, *"CSV / TAB delimited / XLSX file import and export.
ODS import. Markdown export"*, *"Cell shifting"*, *"Sort of rows"*, *"GNUPlot interaction"*,
*"Scripting support with LUA"*, *"Direct color support"*, *"Autobackup"*.
Bağımlılık: ncurses (wide), bison/yacc, gcc, make, pkg-config + **opsiyonel** `libxml-2.0`,
`libzip` (xlsx/ods), `lua`, tmux/xclip/pbpaste (pano), gnuplot.
Windows kurulumu **dokümante değil** (Linux/macOS).
→ **Ders:** 702 kolon = `ZZ` (26 + 26²). sc-im bile Excel'in 16384 kolonunu hedeflemiyor —
**kapsam daraltmak meşru**. XLSX I/O'yu opsiyonel tutmuş; herdr'ın `OptionalPlugin` felsefesiyle aynı.

#### C.1.2 VisiData (Python, 9199★)
*"a terminal interface for exploring and arranging tabular data"*; formatlar *"tsv, csv, sqlite,
json, xlsx (Excel), hdf5, and many other formats"*. Lisans GPLv3.
Platform: *"Linux, OS/X, or Windows (with WSL)"* → **Windows native YOK**. Python 3.8+.
→ **Ders:** En güçlü fikri hücre editi değil, **kolon işlemleri** (tipe çevir, frekans tablosu,
filtrele). Dosya yöneticisi önizlemesinde kullanıcının %90 ihtiyacı budur.

#### C.1.3 csvlens (Rust, MIT) — 🎯 herdr için en doğrudan UX şablonu
README özellikleri (birebir):
- Vim tuşları + ok tuşları ile gezinme, `Ctrl+f/b` veya PageUp/Down ile *"Scroll one window down/up"*
- **Regex ile arama + vurgulama**
- **Satır ve kolon filtreleme (regex)**
- **Kolon genişliği ayarı + soldan kolon dondurma**
- **Seçim modları: satır / kolon / hücre — `TAB` ile geçiş**
- Doğal sıralamayla satır sıralama
- **Hücre kopyalama (pano) + çıktı**
- Satır kaydırma (line wrap) seçenekleri, görsel satır işaretleme/temizleme
- **Hücre düzenleme: YOK — salt görüntüleyici.** Rust 1.88+, v0.12.0.

→ **Ders:** "Düzenleme olmadan da çok kullanışlı tablo görüntüleyici" tezinin canlı kanıtı.
Tüm bu liste **saf render + saf state** ile yapılabilir → herdr'ın "Render is pure" kuralına uyar.

#### C.1.4 garritfra/cell (Rust + ratatui, 312★) — 🎯 mimari şablon
README birebir:
```
cell-sheet-core/    # Data model, formula engine, file I/O (no TUI dependency)
cell-sheet-tui/     # Ratatui rendering, Vim modes, event loop
```
- Vim: `i` insert, `ESC` normal, `/` arama, sayaç önekli motion operatörleri
- Formüller: `SUM, AVERAGE, COUNT, MIN, MAX, IF`, Excel sözdizimi (`=A1+B1`, `=SUM(A1:A10)`)
- Undo/redo: `.` tekrar, `u` undo, `Ctrl-R` redo
- Görsel modlar: `v`, `V`, `Ctrl-V` (karakter/satır/blok)
- Mouse: opsiyonel (varsayılan kapalı) tıkla/sürükle/kaydır
- Headless CLI: `--read`, `--write`, `--eval`
- Formatlar: CSV, TSV, native `.cell` (**formüller yalnızca `.cell`'de korunur**)
- crates.io: `cell-sheet-core` **0.5.1**, 2026-06-30, MIT, 11 sürüm (Nisan 2026'dan beri), release-plz

→ **Ders:** *"Data model, formula engine, file I/O (**no TUI dependency**)"* ayrımı, herdr
CLAUDE.md'sindeki **"State is separated from runtime"** + **"Render is pure"** ile **kelime kelime
aynı felsefe**. herdr'da tablo modeli yazılacaksa şekli budur.

#### C.1.5 tshts (Rust + ratatui + crossterm, 43★) — formül motorunun gerçek maliyeti
Fonksiyonlar (README birebir):
- Sayısal: `SUM, AVERAGE, MIN, MAX, ABS, SQRT, ROUND, CEILING, FLOOR, INT, MOD, POWER, SIGN, LOG, LN, EXP, PI, RAND, RANDBETWEEN`
- Koşullu toplama: `SUMIF, COUNTIF, AVERAGEIF`
- Arama: `VLOOKUP, INDEX, MATCH`
- String: `CONCAT, LEN, UPPER, LOWER, PROPER, TRIM, LEFT, RIGHT, MID, FIND, SUBSTITUTE, REPLACE, REPT, EXACT, CLEAN, CHAR, CODE, TEXT, VALUE, NUMBERVALUE`
- Tarih: `TODAY, NOW, DATE, YEAR, MONTH, DAY`
- Web: `GET(url)` — *"non-blocking, cached for 5 min"*
- Mantık: `IF, AND, OR, NOT, TRUE, FALSE`
- Info: `ISBLANK, ISNUMBER, ISTEXT, TYPE, COUNT, COUNTA`

Ayrıca *"AST-based"* döngüsel referans tespiti, *"Automatic rebuilding of formula dependencies on
load"*, mutlak referanslar (`$A$1, $A1, A$1`). Hücre modeli: değer + formül (nullable).
Native format: insan-okunur JSON (`.tshts`); CSV import/export `Ctrl+L`/`Ctrl+E`.

→ **Ders:** Formül motoru = ayrıştırıcı + AST + bağımlılık grafiği + topolojik yeniden hesap +
döngü tespiti + 60 fonksiyonluk stdlib = **ayrı bir ürün**.
herdr için: **formül motoru YAZMA**; `calamine::worksheet_formula()` ile formül **metnini**,
`worksheet_range()` ile Excel'in **önbelleğe aldığı değeri** göster. Yan yana gösterilirse ihtiyacın
neredeyse tamamı karşılanır, maliyet sıfıra yakındır.

### C.2 UX mekaniği: sanal kaydırma, hücre modeli, dondurulmuş başlık

**Sanal kaydırma zorunlu:** XLSX 1M satır olabilir; hepsini `Vec<Row>`'a açmak = OOM. Doğru model:
viewport = (ilk_satır, ilk_kolon, yükseklik, genişlik), sadece görünen pencere materyalize edilir.
csvlens ve tv ikisi de böyle (tv: *"Automatic memory-efficient loading for large files (>5MB)"*).

**herdr avantajı:** `calamine::worksheet_range()` → `Range` **seyrek** yapı, `.start()`/`.end()`
sınırları verir → viewport dilimleme doğal, ekstra iş yok.

**Minimum hücre modeli:**
```rust
// Saf veri — PTY yok, async yok (CLAUDE.md: "AppState is pure data")
struct SheetSnapshot {
    sheet_names: Vec<String>,
    active_sheet: usize,
    dims: (u32, u32),                          // satır × kolon
    cells: BTreeMap<(u32,u32), CellValue>,     // seyrek
    formulas: BTreeMap<(u32,u32), String>,     // calamine::worksheet_formula()
    col_widths: Vec<u16>,                      // kullanıcı ayarı, csvlens deseni
    frozen_cols: u16,                          // soldan dondurma
}
enum CellValue { Empty, Text(String), Number(f64), Bool(bool), DateTime(String), Error(String) }
```
`calamine::Data` enum'u bununla neredeyse birebir eşleşir → dönüştürme ucuz.

**Kolon genişliği:** tv'nin çözümü — *"Significant digit printing: No more decimal dust taking
valuable terminal space"* + *"Long string/Unicode truncation"* + *"Column overflow logic"* +
*"NA comprehension & coloring"* + *"Dimensions printed first"*.
genişlik = `min(maks_içerik, tavan)`, sayılar anlamlı basamağa yuvarlanır.
herdr'ın `unicode-width` bağımlılığı **zaten var** → doğru genişlik hesabı bedava.

### C.3 herdr'ın Miller kolon mimarisine oturtma

Doğrulanmış yerleşim (`src/ui/file_manager.rs:141`):
`[ parent | divider | current | divider | preview ]` (3 kolon) veya `[ current | divider | preview ]`
(2 kolon, `:156`) veya preview yok (`:172`). `preview` bir `Rect`, `MillerResizeColumnId::Preview`
ile yeniden boyutlandırılabiliyor (`file_manager_miller.rs:220`), içerik alanı
`file_manager_preview_content_area()` ile alınıyor (`:218`).

**İki katmanlı tasarım öneriliyor.**

#### Katman 1 — Miller preview kolonunda tablo özeti (dar, salt-okuma)
```
┌─ ~/veri ──────────┬─ 2026-Q3 ───────────┬─ satis.xlsx ─────────────────────┐
│  arsiv/           │  ozet.md            │ 3 sayfa · 1.284 satır · 12 kolon │
│  raporlar/        │  satis.xlsx      ▸  │ ── Sheet1 (aktif) ──────────────  │
│  sablon/          │  tahmin.xlsx        │    A          B        C          │
│  notlar.txt       │  kaynak.csv         │ 1  Bölge      Ay       Tutar      │
│                   │                     │ 2  Kuzey      Oca      12.400     │
│                   │                     │ 3  Kuzey      Şub      13.150     │
│                   │                     │ 4  Güney      Oca       9.880     │
│                   │                     │ 5  Güney      Şub      11.020     │
│                   │                     │ 6  Doğu       Oca      15.300     │
│                   │                     │ …                                 │
│                   │                     │ [enter] tam ekran  [s] sayfa değiş│
└───────────────────┴─────────────────────┴──────────────────────────────────┘
```
Maliyet: `calamine` ile ilk N satır → mevcut `file_preview_worker` generation'ı içinde → saf
`render()`. **Yeni UI paradigması yok.**

#### Katman 2 — Tam ekran sayfa modu (dondurulmuş başlık + hücre seçimi)
```
┌─ satis.xlsx · Sheet1 · C4 ────────────────────────────────── 1.284×12 ─┐
│ fx │ =B4*1,21                                                          │
├────┼──────────────┬──────────┬────────────┬──────────┬─────────────────┤
│    │ A            │ B        │ C          │ D        │ E               │
│    │ Bölge        │ Tutar    │ KDV'li     │ Ay       │ Temsilci        │
├────┼──────────────┼──────────┼────────────┼──────────┼─────────────────┤
│  2 │ Kuzey        │   12.400 │     15.004 │ Oca      │ A. Yılmaz       │
│  3 │ Kuzey        │   13.150 │     15.912 │ Şub      │ A. Yılmaz       │
│  4 │ Güney        │    9.880 │  ▓11.955▓  │ Oca      │ B. Demir        │
│  5 │ Güney        │   11.020 │     13.334 │ Şub      │ B. Demir        │
│  6 │ Doğu         │   15.300 │     18.513 │ Oca      │ C. Kaya         │
├────┴──────────────┴──────────┴────────────┴──────────┴─────────────────┤
│ ↑↓←→ gez  TAB seçim(satır/kolon/hücre)  / ara  f filtre  y kopyala      │
│ z kolon dondur  s sayfa  q çık                    [salt-okunur]         │
└────────────────────────────────────────────────────────────────────────┘
   ▲ dondurulmuş: satır başlığı + başlık satırı        ▲ formül çubuğu
```

**Tasarım kararları ve gerekçeleri:**
- **Formül çubuğu (`fx`)** — `worksheet_formula()` metni + gridde `worksheet_range()` önbellek
  değeri. Formül motoru gerekmez (§C.1.5).
- **`TAB` ile seçim modu** — csvlens'ten doğrudan alınan, kanıtlanmış etkileşim.
- **`z` kolon dondurma** — csvlens *"freezing columns from the left"*.
- **`[salt-okunur]` rozeti** — dürüstlük; edit gelene kadar kullanıcı beklentisi doğru kalır.

### C.4 XLSX oku/yaz zinciri — KARAR

| Senaryo | Crate | Gerekçe |
|---|---|---|
| **Görüntüle** (Aşama 1) | **`calamine` 0.36.0** | Saf Rust, Windows sorunsuz, `worksheet_formula()` bonus, 10.2M indirme = olgun, salt-okuma = düşük risk yüzeyi |
| **Düzenle** (Aşama 2) | `umya-spreadsheet` 3.0.1 | Tek gerçek oku-değiştir-yaz seçeneği. ⚠️ **formül korunumu doğrulanmadı** — POC şart |
| **Sıfırdan üret** | `rust_xlsxwriter` 0.96.0 | herdr senaryosu değil |

⚠️ **Edit uyarısı:** `umya-spreadsheet` ile oku-yaz yapıldığında kütüphanenin modellemediği her şey
(pivot tablolar, grafikler, koşullu biçimlendirme, makrolar, veri doğrulama) **sessizce
kaybolabilir**. Dosya yöneticisinde kullanıcının Excel dosyasını bozmak, "önizleme yok"tan **çok daha
kötü** bir sonuçtur. Edit'e geçilirse: (a) yazmadan önce yedek, (b) desteklenmeyen özellik
tespitinde uyarı, (c) başta yalnızca `.csv` editlenebilir, XLSX salt-okunur — en güvenli başlangıç.

---

## D. PDF BÖLÜMÜ

### D.1 İki yol
| | **Metin katmanı** | **Raster** |
|---|---|---|
| Verir | Aranabilir/kopyalanabilir metin, hızlı | Sayfanın gerçek görünümü (düzen, şekil, tablo) |
| Rust | `lopdf` 0.44.0 (saf Rust) | `pdfium-render` 0.9.3 (**harici binary**) |
| Harici | `pdftotext` (poppler) | `pdftoppm` / `mutool` |
| Maliyet | Düşük | Yüksek (CPU + bant genişliği + dağıtım) |
| Windows | ✅ saf Rust | ⚠️ DLL dağıtımı |
| Taranmış PDF | ❌ metin yok | ✅ tek çözüm |

### D.2 yazi'nin kanıtlanmış deseni (`yazi-plugin/preset/plugins/pdf.lua`, 55 satırın tamamı okundu)
```lua
Command("pdftoppm")
  :arg({ "-f", job.skip + 1, "-l", job.skip + 1, "-singlefile",
         "-jpeg", "-jpegopt", "quality=" .. rt.preview.image_quality,
         tostring(job.file.path), tostring(cache) })
  :output()
```
sonra `ya.image_precache(Url(cache .. ".jpg"), cache)` → `ya.image_show(cache, job.area)`.

**4 ders:**
1. `-f N -l N -singlefile` → **yalnızca istenen tek sayfa** rasterleşir; 400 sayfalık PDF'te maliyet sabit.
2. `job.skip` = sayfa numarası → sayfa gezinme ayrı mekanizma değil, mevcut "kaydırma" kavramının
   yeniden kullanımı (`M:seek`, `pdf.lua:23-28`, `ya.clamp(-1, job.units, 1)`).
3. Sayfa sınırı **hatadan** öğreniliyor: `output.stderr:match("the last page %((%d+)%)")` → sonra
   `ya.emit("peek", { bound - 1, only_if = job.file.url, upper_bound = true })`. PDF'i önceden
   ayrıştırmadan sayfa sayısı. Zarif.
4. `ya.file_cache(job)` + `fs.cha(cache)` varsa iş atlanır → aynı sayfaya dönüşte sıfır maliyet.
   Ayrıca `ya.sleep(math.max(0, rt.preview.image_delay/1000 + start - os.clock()))` ile debounce.

### D.3 herdr için aşamalı PDF planı
| Aşama | Ne | Bağımlılık | Kazanç |
|---|---|---|---|
| **P0** | Plugin: `pdftotext -l 50` → metin | **Rust'ta sıfır** | Aranabilir metin, ~%80 kullanım |
| **P1** | Plugin: `pdftoppm` → PNG/JPEG → mevcut `NativeImage` yoluna besle | **Rust'ta sıfır** | Görsel sayfa + gezinme |
| **P2** | `lopdf` ile sayfa sayısı/metadata | saf Rust | Harici araçsız temel bilgi |
| **P3** | `pdfium-render` native raster | binary dağıtımı | ❌ **Tavsiye edilmez** |

**Net tavsiye:** PDF için **native Rust rasterleştirme yapmayın.** yazi 10k+ yıldızlı bir dosya
yöneticisi ve harici `pdftoppm` kullanıyor. herdr'ın tek-binary dağıtımı için PDFium DLL'ini 4
platforma taşımak, kazanılan şeye değmez.

---

## E. KADEMELİ YOL HARİTASI

### 🟢 Aşama 0 — Bugün mümkün, **sıfır yeni bağımlılık**
| # | İş | Ne değişir | Büyüklük | Risk | Kullanıcı kararı |
|---|---|---|---|---|---|
| 0.1 | `PreviewProviderSet.documents` için **örnek plugin** (`herdr-plugin-examples`): `xlsx2csv`/`pdftotext` çağıran argv | pdf/xlsx "optional document viewer" yazısı yerine **gerçek içerik** | **S** | Çok düşük | Yok — mevcut sözleşme |
| 0.2 | Kitty grafiği **debounce** (yazi `image_delay=30ms`) | Hızlı gezinmede iletim iptali | **S** | Düşük | Varsayılan gecikme değeri |
| 0.3 | Uzak oturumda **kalite/kapalı politikası** (mevcut `PreviewFallback`) | SSH'ta donma biter | **S–M** | Düşük | Uzak varsayılanı: kapalı mı düşük kalite mi |
| 0.4 | `experimental.kitty_graphics` olgunlaştırma: timeout'lu **capability sondajı** deseni | Bayrağın `false`'tan çıkması ön koşulu | **M** | Orta | Bayrak ne zaman kalkacak |

**Değeri:** "PNG/XLSX/PDF görünsün" hedefinin **büyük kısmı**, tek satır yeni bağımlılık olmadan
karşılanır — uzantı noktası zaten yerinde.

### 🟡 Aşama 1 — Küçük, gerekçeli bağımlılık
| # | İş | Crate | Büyüklük | Karar |
|---|---|---|---|---|
| 1.1 | **Native XLSX görüntüleme** (Miller preview, §C.3 K1) | **`calamine` 0.36.0** | **M** | 🔑 **`calamine` eklensin mi?** |
| 1.2 | **Tam ekran sayfa modu** (§C.3 K2) | yok (saf ratatui + mevcut `unicode-width`) | **L** | Kapsam: hangi csvlens özellikleri |
| 1.3 | CSV/TSV için aynı grid | yok | **S** | Yok |
| 1.4 | PDF sayfa sayısı/metadata native | `lopdf` 0.44.0 | **S** | Opsiyonel |

**1.1 gerekçesi:** Lehine — saf Rust (tek binary korunur), 10.2M indirme, aktif (2026-07-06), MIT,
salt-okuma = düşük risk yüzeyi. Aleyhine — herdr'ın "yeni bağımlılık ancak kanıtlı ihtiyaçla" kuralı.
**Kanıt:** kullanıcının açık talebi + `preview_capability.rs`'in xlsx'i zaten özel-durum listelemesi.

### 🔴 Aşama 2 — Ağır, dikkatli karar
| # | İş | Crate | Büyüklük | Uyarı |
|---|---|---|---|---|
| 2.1 | **Hücre düzenleme** (önce yalnızca CSV) | `edtui` 0.11.6 veya el yapımı | **L** | `tui-textarea` **KULLANMAYIN** — 21 ay durgun |
| 2.2 | **XLSX yazma** | `umya-spreadsheet` 3.0.1 | **XL** | ⚠️ Veri kaybı riski — POC'suz yapılmamalı |
| 2.3 | Formül motoru | — | **XXL** | ❌ **Yapmayın** |
| 2.4 | Native PDF raster | `pdfium-render` | **XL** | ❌ **Yapmayın** |

---

## F. KAYNAK REGISTRY'Sİ (özet)

> **Tam registry:** `docs/references/document-rendering.md` — 35+ kaynak, tier + confidence ile.
> Buradaki tablo yalnızca hızlı bakış içindir; kanonik olan referans dosyasıdır.

Kaynak sınıfları: `official` (protokol/dil resmî docs) · `official-registry` (crates.io v1 API) ·
`official-docs` (docs.rs) · `official-api` (GitHub Search API) · `source-code` (canlı kod — en güçlü
yerel kanıt) · `source-repo` (GitHub README) · `source-docs` (bu reponun docs'u).

---

## G. ANTİ-PATTERN'LER

> Damıtılmış hâli: `docs/patterns/document-rendering.md` §Anti-pattern (DA1–DA12).

| # | Anti-pattern | Neden felaket | Doğrusu | Kanıt |
|---|---|---|---|---|
| G1 | Görseli tam çözünürlükte iletmek | SSH'ta 11 MB base64 = 9 sn donma | Hedef `Rect`×`HostCellSize` downscale, **sonra** ilet | P06 |
| G2 | Senkron decode/harici süreçle UI blokla | Bozuk PDF = donmuş TUI | Bounded worker + generation + stale-reject | `file_preview_worker.rs:86`; yazi `Scheduler.cancel` |
| G3 | Debounce'suz önizleme | Ok tuşu basılı = 50 görsel kuyruğu | `image_delay` (yazi 30 ms) | `yazi-default.toml:26` |
| G4 | Küçük harf `d=i`'yi bellek temizliği sanmak | Terminalde görsel verisi birikir | Kalıcı temizlikte `d=I` | Kitty spec P02 |
| G5 | Alternate screen/mod geçişinde silmemek | Hayalet görseller metnin üstünde | Mod ve view değişiminde açık `a=d` | herdr `HostViewKey{workspace_index,tab_index,file_manager_open}` (`kitty_graphics.rs:69`) — **zaten çözmüş** |
| G6 | Kaydırmada görselin metinle kaymayacağını varsaymak | Görsel yerinde kalır, metin akar | Spec: *"images must be scrolled along with text"* → `U=1` veya her frame yeniden yerleştir | Kitty spec (5) |
| G7 | Decode limiti koymamak | 20000×20000 PNG ≈ 1.6 GB = OOM | Çok katmanlı sınır | herdr `image_preview.rs:11-15` — **örnek alınacak** |
| G8 | Tüm XLSX satırlarını belleğe açmak | 1M satır = OOM | Viewport sanal kaydırma; `calamine::Range` seyrek | tv: *"memory-efficient loading"* |
| G9 | Formül motoru yazmaya girişmek | Ayrıştırıcı+AST+bağımlılık+döngü+60 fn = ayrı ürün | Formül **metni** + önbellek **değeri** göster | tshts C.1.5 |
| G10 | XLSX oku-yazda sessiz veri kaybı | Pivot/grafik/makro kaybolur | Yedek + uyarı; veya sadece CSV editle | §C.4 |
| G11 | Kullanılmayan protokolü desteklemek | Ghostty VT sixel parse etmiyor → ölü kod | Kitty'ye odaklan; fallback metin/metadata | `find vendor -ipath "*sixel*"` → boş |
| G12 | Harici araç varlığını varsaymak | `pdftoppm` yoksa Windows'ta sessiz hata | `platform_supported` + `PreviewFallback` | `preview_capability.rs:181` `plugin_or_fallback` — **zaten doğru** |

---

## H. 6 SORUNUN NET CEVAPLARI

### H.1 Terminal Excel: en olgun projeler, hücre modeli, düzenleme UX'i, ratatui ile ne kadarı taklit edilebilir?
**En olgunlar:** VisiData (9199★, Python/GPLv3, Windows sadece WSL), sc-im (5655★, C/ncurses, Windows
dokümante değil), sheets (2287★, Go). **Rust+ratatui:** garritfra/cell (312★, MIT, aktif), tshts (43★, MIT).
**Hücre modeli:** Seyrek `BTreeMap<(row,col), CellValue>` + ayrı `formulas` haritası + viewport.
`calamine::Data` ile birebir eşleşir (§C.2).
**Düzenleme UX'i:** Modal (vim) — `i` insert, `ESC` normal, `u`/`Ctrl-R` undo/redo, `v/V/Ctrl-V`
görsel seçim, formül çubuğu, `/` arama. cell ve tshts ikisi de bu kalıbı kullanıyor.
**ratatui ile ne kadarı taklit edilebilir:** **Görüntüleme tarafının %100'ü** — csvlens'in tüm özellik
listesi (regex arama/filtre, dondurulmuş kolon, satır/kolon/hücre seçimi, kopyalama, kolon genişliği)
saf ratatui + saf state ile yapılabilir ve "Render is pure" kuralına uyar. **Düzenleme** de yapılabilir
(cell bunu ratatui ile yapmış) ama maliyeti bir mertebe büyük. **Formül motoru taklit EDİLMEMELİ.**
ASCII mockup'lar §C.3'te.

### H.2 Rust XLSX oku/yaz zinciri — somut karar
- **OKU: `calamine` 0.36.0** — MIT, 2026-07-06, 10.19M indirme, saf Rust (Windows sorunsuz), düşük
  ağırlık. **Formül desteği VAR ama okuma olarak**: `worksheet_formula()` formül metnini,
  `worksheet_range()` Excel'in önbellek değerini verir. Hesaplama motoru yok — ve gerekmiyor.
- **YAZ/DÜZENLE: `umya-spreadsheet` 3.0.1** — MIT, 2026-07-13, saf Rust. `reader::xlsx::read` →
  `sheet_by_name_mut` → `cell_mut("A1").set_value` → `writer::xlsx::write`.
  ⚠️ formül/grafik/pivot korunumu docs'ta belgelenmemiş — **POC şart**.
- **`rust_xlsxwriter` 0.96.0**: sadece sıfırdan üretim → herdr senaryosu değil.
- **KARAR:** Aşama 1'de yalnızca `calamine` (salt-okuma = düşük risk). Yazma kararını POC'a bağla;
  ilk edit **CSV ile** başlasın, XLSX salt-okunur kalsın.

### H.3 PDF: metin katmanı vs raster
- **Metin yolu:** `pdftotext` (harici, Rust'ta sıfır bağımlılık) veya `lopdf` 0.44.0 (saf Rust,
  metadata/sayfa sayısı; rasterleştirmez). Maliyet düşük, Windows ✅, SSH'ta bant genişliği ihmal
  edilebilir. Senaryoların ~%80'i.
- **Raster yolu:** `pdfium-render` 0.9.3 (harici pdfium binary/DLL — 4 platforma dağıtım, tek-binary
  modelini bozar) **veya** yazi'nin yolu: harici `pdftoppm` ile **tek sayfa** JPEG. SSH'ta bir sayfa
  JPEG ≈ 100–500 KB (kalite 75) → kabul edilebilir; PNG/tam çözünürlük olursa MB'lara çıkar.
- **KARAR:** Native Rust raster **yapma**. Sıra: P0 metin (plugin) → P1 `pdftoppm` plugin + mevcut
  `NativeImage` yolu → P2 `lopdf` metadata. P3 (`pdfium-render`) tavsiye edilmiyor.

### H.4 ratatui-image mimarisi ve herdr'ın kendi kitty_graphics.rs'i varken geçmek mantıklı mı?
**Mimari (gerçek kod kanıtı, `refpool/ratatui-image/`):**
- **`Picker`** (`src/picker.rs`): merkez. `from_query_stdio`/`from_query_stdio_with_options` ile
  terminale sorgu, `cap_parser` ile ayrıştırma; `font_size` (`:224`), `protocol_type`/
  `set_protocol_type` (`:214`,`:219`), `capabilities` (`:234`), `set_background_color` (`:229`),
  `halfblocks` (`:175`), `from_fontsize` (`:193`), `new_protocol` (`:256`), `new_protocol_raw`
  (`:241`), `new_resize_protocol` (`:277`). Sondaj timeout'lu, asılmama testi var.
- **Protocol backends** (`src/protocol/`): `kitty.rs`, `sixel.rs`, `iterm2.rs`, `halfblocks.rs`
  (+ `halfblocks/chafa.rs`, `halfblocks/primitive.rs`). `ProtocolType.next` (`:71`) döngüsel geçiş.
- **Resize protocol:** `StatefulImage.resize`, `ResizeEncodeRender.needs_resize` +
  `needs_resize_fit` / `needs_resize_crop`; alan değişince yeniden encode kararı.
- **`SlicedImage`** (`src/sliced.rs`): `slice_rows` ile bant bant render (kaydırma senaryosu).
- **Threading:** `src/thread.rs` + opsiyonel `tokio` feature (`sync`).
- **Bağımlılıklar:** `image`, `icy_sixel`, `base64-simd`, `rand`, `ratatui 0.30.1`, `self_cell`,
  `thiserror`; unix `rustix 0.38`, **windows `windows 0.58`**. Rust 1.86+, edition 2024.

**Geçmek mantıklı mı? — HAYIR, ama artıları var:**

| Artı (lehine) | Eksi (aleyhine) |
|---|---|
| 4 protokol birden (sixel/iTerm2/halfblock fallback) | herdr'ın Ghostty VT'si **sixel parse etmiyor** → 3'ü ölü kod |
| Olgun capability sondajı (timeout'lu, tmux farkında) | Bu parça **kopyalanabilir**; tüm crate gerekmez |
| Bakım devri (aktif: 2026-06-25) | herdr'ın 2075 satırı atılır; `HostViewKey`, id-uzayı izolasyonu, 5 katmanlı limit crate'te YOK |
| Widget API hazır | **Mimari çakışma:** ratatui-image client-side widget; herdr server-side render + client passthrough |
| — | `windows 0.58` vs herdr `windows-sys 0.61.2` → iki Windows binding ailesi |

**NET TAVSİYE:** Geçme — **öğren**. Alınacak tek somut parça: `Picker`'ın **timeout'lu capability
sondajı** deseni. Bu, `experimental.kitty_graphics = false` bayrağını kaldırmanın ön koşulu (Aşama 0.4).

### H.5 Aşama 0/1/2 — crate, iş, risk, karar
Tam tablo §E'de. Tek asıl kullanıcı kararı: **Aşama 1.1'de `calamine` eklenecek mi?**

### H.6 Erişilemeyen kaynaklar
§I'de.

---

## I. ⚠️ ERİŞİLEMEYEN KAYNAKLAR (dürüstlük kaydı — SİLİNMEZ)

| # | Kaynak | Sorun | Telafi | Etki |
|---|---|---|---|---|
| 1 | `sw.kovidgoyal.net/kitty/graphics-protocol/` | DNS timeout | ✅ Aynı içerik `raw.githubusercontent.com/kovidgoyal/kitty/master/docs/graphics-protocol.rst`'ten alındı | Yok — protokol iddiaları `official` tier |
| 2 | `learn.microsoft.com/.../terminal/release-notes/1.22` | DNS timeout | ❌ **Telafi EDİLEMEDİ** | **Windows Terminal Sixel desteği DOĞRULANAMADI.** Kontrol edilen tek kaynak (`v1.22.10731.0` GitHub release notu) sixel'den bahsetmiyor ama o bir yama sürümü. **Raporun sonucunu değiştirmiyor** (herdr sixel üretmiyor) |
| 3 | `github.com/microsoft/terminal/blob/main/doc/reference/Sixel.md` | HTTP 404 (dosya yok) | ❌ | Aynı boşluk |
| 4 | `umya-spreadsheet` formül/grafik/pivot korunumu | docs.rs'te belgelenmemiş; kaynak koda inilmedi | ❌ | **Aşama 2.2 için POC şartı** (conf 0.75) |
| 5 | `mcp__pkg-registry__*` cargo araçları | "Error fetching package details" — 4/4 başarısız | ✅ crates.io v1 API doğrudan kullanıldı | Yok — daha birincil kaynak |
| 6 | `WebSearch` aracı | Oturum boyunca API hatası (`output_config.effort 'xhigh' is not supported`) | ✅ Tüm dış kanıt WebFetch ile birincil kaynaklardan | Yok — arama özeti yerine birincil veri (daha güçlü) |
| 7 | `wezterm.org/imgcat.html` Sixel/Kitty desteği | Sayfa yalnızca iTerm2 protokolünü anlatıyor | kısmi | WezTerm'in sixel/kitty desteği bu turda doğrulanmadı |

---

## J. BU TURDA ARAŞTIRILMAYAN — GELECEKTE BAKILACAK ALTERNATİFLER

> **Amaç:** Bir sonraki değerlendirme turunda sıfırdan başlanmasın. Her satır: neden kapsam dışıydı,
> kim bakmalı, hangi sorguyla başlanmalı, bilinen aday.
> ⚠️ **Bu bölümdeki aday projeler bu turda DOĞRULANMADI** (sürüm/bakım/lisans kontrol edilmedi).
> Confidence ≈ 0.5 — "arama kuyruğu", kanıt değil. Kullanmadan önce §K reçetesiyle doğrulayın.

### J.1 Belge formatları
| Alan | Neden bu turda dışarıda | Başlangıç sorgusu | Bilinen aday (⚠️ doğrulanmadı) |
|---|---|---|---|
| **DOCX/ODT görüntüleme** | Kullanıcı önceliği png/xlsx/pdf idi; docx zaten `OptionalPlugin`'de listeli | `crates.io: docx-rs, docx-rust` · `github: terminal docx viewer` · harici: `pandoc`, `libreoffice --convert-to txt` | `docx-rs`; harici `pandoc` (en olası pratik yol) |
| **PPTX görüntüleme** | Aynı; slayt render = raster problemi (PDF'e indirgenebilir) | `libreoffice --convert-to pdf` → §D zinciri | Harici dönüştürme + mevcut PDF yolu |
| **Notebook (.ipynb)** | herdr `preview_capability.rs`'te hiç listeli değil (JSON olarak metin düşüyor) | `crates.io: nbformat` · `github: ipynb terminal viewer` | `jupyter nbconvert --to script` (harici) |

### J.2 Zengin metin
| Alan | Neden dışarıda | Başlangıç sorgusu | Bilinen aday (⚠️ doğrulanmadı) |
|---|---|---|---|
| **Markdown zengin render** | herdr'da markdown → `OptionalPlugin` fallback `NativeText`; `syntect` zaten var, acil değil | `crates.io: termimad, comrak, pulldown-cmark` | `termimad` (ratatui-uyumlu iddia), `pulldown-cmark` (parser), harici `mdcat`/`glow` |
| **Kod diff görüntüleme** | herdr bir dosya yöneticisi; diff ayrı özellik | `github: delta, difftastic` · `crates.io: similar` | `delta` (dandavison), `difftastic`, `similar` crate |
| **Syntax highlight iyileştirme** | `syntect 5.3.0` zaten bağımlılık, çalışıyor | `crates.io: tree-sitter-highlight` | `tree-sitter` (daha doğru ama ağır) |

### J.3 Veri formatları
| Alan | Neden dışarıda | Başlangıç sorgusu | Bilinen aday (⚠️ doğrulanmadı) |
|---|---|---|---|
| **Parquet / Arrow / Feather** | XLSX önceliği vardı; tv bunları destekliyor (referans mevcut) | `crates.io: arrow, parquet` · tv kaynak kodu | `arrow-rs`, `parquet` crate; ⚠️ ağırlık yüksek |
| **JSONL / NDJSON tablo görünümü** | JSON metin olarak zaten okunabiliyor | yazi `json` previewer (`yazi-default.toml:147`) | yazi'nin json plugin'i model olarak |
| **SQLite** | Kapsam dışıydı; VisiData destekliyor | `crates.io: rusqlite` | ⚠️ ağırlık + salt-okuma güvenliği sorusu |

### J.4 Görüntü formatları
| Alan | Neden dışarıda | Başlangıç sorgusu | Bilinen aday (⚠️ doğrulanmadı) |
|---|---|---|---|
| **SVG** | herdr `image` crate'i SVG desteklemiyor; yazi ayrı `svg` plugin'i kullanıyor (`yazi-default.toml:117`) | `crates.io: resvg, usvg` · yazi `svg.lua` | `resvg`/`usvg` (saf Rust); yazi svg plugin deseni |
| **AVIF / HEIC / JXL** | Aynı; yazi `magick` plugin'ine yönlendiriyor (`yazi-default.toml:116`) | yazi `magick.lua` · harici ImageMagick | Harici `magick` (yazi'nin seçimi) |
| **Video küçük resmi** | Medya `OptionalPlugin`'de listeli ama önceliksiz | yazi `video.lua` (`preset/plugins/video.lua`) | `ffmpeg`/`ffmpegthumbnailer` (harici) |
| **Font önizleme** | Niş | yazi `font.lua` (`preset/plugins/font.lua`) | yazi font plugin deseni |

### J.5 İleri seviye
| Alan | Neden dışarıda | Başlangıç sorgusu | Bilinen aday (⚠️ doğrulanmadı) |
|---|---|---|---|
| **OCR (taranmış PDF)** | Metin katmanı yoksa tek çare; çok ileri aşama | `harici: tesseract, ocrmypdf` | `tesseract` (harici, ağır) |
| **CAD / 3D (DWG, STL, STEP)** | herdr kullanıcı profilinde yok | `crates.io: stl_io` | ⚠️ raster gerektirir; muhtemelen hiç yapılmayacak |
| **Arşiv içi gezinme** | `OptionalPlugin.archives` listeli ama plugin yok | yazi `archive` previewer (`yazi-default.toml:159-161`) | `zip`/`tar` crate veya harici `7z`/`bsdtar` |

---

## K. KARŞILAŞTIRMA METODOLOJİSİ (yeniden-kullanılabilir reçete)

> **Amaç:** Gelecekte yeni bir domain için (örn. "DOCX görüntüleme", "diff viewer") aynı kalitede
> değerlendirme yapan agent bu reçeteyi kopyalayıp uygulasın. Bu turda bu yöntem uygulandı ve işe yaradı.

### K.1 Sıra (bu turda kanıtlanmış)
```
0. ZEMİN OKU  → hedef projede zaten ne VAR? (Cargo.toml + ilgili modül + config default + docs)
                ⚠️ EN KRİTİK ADIM. Bu turda "sıfırdan render" varsayımı burada çürüdü.
1. YEREL REFPOOL → codebase-memory-mcp: get_architecture + search_graph
                   → sonra refpool DİSKTEN doğrudan oku (daha hızlı, tam kod)
2. EKOSİSTEM TARA → GitHub Search API (yıldız sıralı) + crates.io v1 API
3. BİRİNCİL DOĞRULA → her aday için repo README + docs.rs API yüzeyi
4. TABLOLA        → lisans/bakım/ağırlık/Windows/uyum kolonlu matris
5. DESENLE        → mimari desen kartları (ne · KULLAN · KULLANMA · kaynak · conf)
6. KARARLA        → aşamalı yol haritası + reddedilenler + yeniden-değerlendirme koşulu
```

### K.2 Somut sorgu şablonları

**crates.io v1 API** (sürüm/bakım/indirme — en güvenilir):
```
https://crates.io/api/v1/crates/<CRATE_ADI>
→ İste: max_version, newest_version, updated_at, downloads, recent_downloads,
        repository, description, license, keywords, categories
```
> `updated_at` **bakım aktifliğinin tek nesnel ölçüsü**. Bu turda `tui-textarea`'nın 21 aydır
> durgun olduğu böyle yakalandı.

**GitHub Search API** (ekosistem taraması):
```
https://api.github.com/search/repositories?q=<TERİM>+in:name,description&sort=stars&order=desc&per_page=20
→ İste: full_name, description, language, stargazers_count, license.spdx_id, archived, pushed_at
```
> `pushed_at` + `archived` + `license.spdx_id` üçlüsü olmadan aday değerlendirilmez.
> Bu turda `SheetJS/wk`'nın 2020'de terk edildiği ve `rustxl`'in lisanssız olduğu böyle yakalandı.

**docs.rs** (API yüzeyi — "gerçekten ne yapabiliyor"):
```
https://docs.rs/<CRATE>/latest/<crate_snake>/
→ Sor: hangi struct açar, hangi metot okur/yazar, YAZMA destekliyor mu, hangi enum varyantları
```
> Bu turda `calamine`'in **salt-okuma** olduğu ve `umya`'nın yazabildiği böyle ayrıldı.

**codebase-memory-mcp refpool** (yerel indeksli referans projeler):
```
get_architecture(project="home-ayaz-.cartography-refpool-<AD>")
search_graph(project=..., query="<doğal dil>", limit=40)
→ sonra: /home/ayaz/.cartography/refpool/<AD>/ altından DOSYAYI DOĞRUDAN OKU
```
> MCP sembol/dosya yerini verir; **tam kod kanıtı için diskten okumak daha verimli**.
> Bu turda yazi `pdf.lua`'nın 55 satırının tamamı böyle okundu.

**Güvenlik/advisory:** `mcp__pkg-registry__get-package-advisories(ecosystem="rust", packageName=...)`
— ⚠️ bu turda pkg-registry MCP hata verdi; alternatif `https://rustsec.org/`.

### K.3 Aday tablosu şablonu (kolonlar ZORUNLU)
```
| Ad | Ne yapar | Lisans | Son sürüm + tarih | Ağırlık (bağımlılık) | Windows | <proje> uyumu (0-5) | Kaynak |
```
**Neden bu kolonlar:**
- **Lisans** → herdr AGPL-3.0-or-later; MIT/Apache sorunsuz, GPL ayrı süreç olmalı, **lisanssız = kullanılamaz**
- **Son sürüm + tarih** → bakım ölçüsü; 12+ ay durgunluk kırmızı bayrak
- **Ağırlık** → transitif bağımlılık + platform-özel binding çakışması (`windows` vs `windows-sys`!)
- **Windows** → herdr üç platform destekliyor; "saf Rust mu, harici binary mi" kritik ayrım
- **Uyum 0-5** → hedef projenin mimari kurallarına karşı puan, genel kalite değil

### K.4 Kanıt sözleşmesi (evidence-propagation)
- Her iddia `(claim, evidence, confidence)`. **Çıplak iddia YASAK.**
- `verified` = 1 official/executable kaynak ≥0.9 **VEYA** 2 BAĞIMSIZ kaynak ≥0.7
  (birbirini aktaran iki blog = tek kaynak, üçgenlemez).
- Erişilemeyen kaynak `⚠️ doğrulanamadı` **işaretlenir, silinmez** — bir sonraki tur oradan devam eder.
- Tier sözlüğü: `official` · `official-registry` · `official-docs` · `official-api` · `source-code`
  · `source-repo` · `source-docs` · `authoritative`.

### K.5 Bu turda işe yarayan/yaramayan araçlar (gelecek tur için not)
| Araç | Durum | Not |
|---|---|---|
| `WebFetch` (crates.io/GitHub API/raw.githubusercontent) | 🟢 mükemmel | Birincil kaynak; JSON API'ler sorunsuz |
| `codebase-memory-mcp` + diskten okuma | 🟢 mükemmel | Sembol bul → dosya oku kombinasyonu |
| `Bash` (grep/find/sed, hedef repo) | 🟢 mükemmel | Zemin okuması için vazgeçilmez |
| `WebSearch` | 🔴 bozuk | `effort 'xhigh' not supported` — oturum boyunca |
| `mcp__pkg-registry__*` cargo | 🔴 bozuk | "Error fetching package details" 4/4 |
| `learn.microsoft.com`, `sw.kovidgoyal.net` | 🔴 DNS timeout | GitHub raw alternatifi kullan |

---

## L. REDDEDİLEN ADAYLAR VE GEREKÇELERİ

> **Neden bu bölüm var:** Bir kararın *neden* verilmediği, verildiği kadar değerlidir. Koşullar
> değişirse karar **yeniden açılabilmelidir**. Her satırda "yeniden değerlendirme tetikleyicisi" var.

| Aday | Karar | Gerekçe | 🔄 Yeniden değerlendirme koşulu |
|---|---|---|---|
| **`ratatui-image`** | ❌ Bağımlılık olarak alma · ✅ **Desen olarak öğren** | herdr'ın 2075 satırlık kendi Kitty katmanı var; crate'in 4 protokolünden 3'ü Ghostty VT'de ölü; client-side widget modeli server-side render mimarisine ters; `windows 0.58` vs `windows-sys 0.61.2` ikinci binding ailesi | (a) herdr Ghostty VT'ye **sixel desteği** eklerse, (b) herdr kendi Kitty katmanının bakımını sürdüremez hâle gelirse, (c) crate `windows-sys`'e geçerse, (d) herdr client-side widget render'a dönerse |
| **`pdfium-render`** | ❌ Alma | Harici pdfium binary/DLL gerektirir → 4 platforma binary dağıtımı; herdr tek-binary dağıtıyor. Taranmış PDF dışında kazanç `pdftoppm` ile eşit | (a) Taranmış PDF birincil ihtiyaç olursa, (b) herdr zaten binary asset dağıtmaya başlarsa, (c) saf-Rust bir PDF rasterleştirici olgunlaşırsa |
| **`viuer`** | ❌ Alma | Kendi stdout'una yazar; herdr'ın pane/placement muhasebesine oturmaz. 2025-12-09'dan beri durgun | (a) herdr basit tek-görsel CLI alt-komutu isterse (`herdr imgcat` gibi), (b) crate ratatui-native API kazanırsa |
| **`tui-textarea`** | ❌ Alma | **2024-10-22'den beri sürüm yok (21 ay)**; ratatui 0.30 uyumu şüpheli | (a) Yeni sürüm çıkar + ratatui 0.30 uyumu doğrulanırsa. Aksi hâlde `edtui` tercih |
| **`icy_sixel`** | ❌ Alma | Ghostty VT sixel parse etmiyor → üretilen sixel herdr içinde görüntülenemez | (a) Ghostty VT'ye sixel eklenirse, (b) herdr sixel-destekli dış host terminaller için ayrı çıkış yolu açarsa |
| **chafa (FFI)** | ❌ Alma | Sistem C kütüphanesi + LGPL-3; herdr'ın tek-binary + saf-Rust eğilimine ters | (a) Grafik-desteksiz terminaller için yüksek kaliteli ASCII fallback birincil ihtiyaç olursa (o zaman bile saf-Rust halfblock önce denenmeli) |
| **`polars` / `arrow`** | ❌ Alma | Devasa ağırlık; ihtiyaç **görüntüleme**, analiz değil | (a) herdr veri analizi özelliği hedeflerse, (b) Parquet/Arrow birincil format olursa (§J.3) |
| **`rust_xlsxwriter`** | ❌ Alma (bu senaryoda) | Yalnızca sıfırdan üretim; mevcut dosyayı düzenleyemez | (a) herdr "rapor/dışa aktarım üret" özelliği eklerse (ör. dizin listesini xlsx'e aktar) |
| **Formül motoru (kendi yazımı)** | ❌ Yapma | Ayrıştırıcı + AST + bağımlılık grafiği + topolojik hesap + döngü tespiti + 60 fn stdlib = ayrı ürün (tshts kanıtı). `calamine` önbellek değeri + formül metni ihtiyacın ~%95'ini karşılar | (a) Kullanıcı **canlı yeniden hesaplama** talep ederse **ve** edit zaten çalışıyorsa, (b) hazır bir Rust formül-motoru crate'i olgunlaşırsa (o zaman `cell-sheet-core` incelenmeli — MIT, ayrılabilir tasarım) |
| **VisiData (harici süreç)** | ⚠️ Ertelendi | Python + GPLv3 + **Windows sadece WSL** → herdr'ın üç-platform sözü tutulamaz | (a) Yalnızca Linux/macOS için opsiyonel plugin olarak sunulursa (platform filtresi zaten var: `platform_supported`) |
| **sc-im (harici süreç)** | ⚠️ Ertelendi | Windows dokümante değil; lisans `NOASSERTION` (netleştirilmeli) | (a) Windows desteği doğrulanır **ve** lisans netleşirse — plugin olarak güçlü aday |
| **`only-using-ai/rustxl`** | ❌ Kullanma | **Lisans YOK** (`license_spdx_id: null`) → hukuken kullanılamaz | (a) Depoya açık bir lisans eklenirse |
| **`SheetJS/wk`** | ❌ Kullanma | Son push **2020-02-26** → terk edilmiş | (yeniden aktifleşmesi beklenmiyor) |

---

## M. SONUÇ

**Tek cümle:** herdr'ın belge render için ihtiyacı olan mimari **zaten mevcut** — `OptionalPlugin`
uzantı noktası, Kitty grafik katmanı, bounded worker + generation, çok katmanlı kaynak sınırları,
`PreviewFallback` zarif düşüşü. Eksik olan **içerik sağlayıcılar** ve **birkaç olgunlaştırma detayı**
(debounce, capability sondajı, uzak-oturum politikası).

**Önerilen sıra:**
1. **Aşama 0.1** — `documents` plugin örneği. Bugün, sıfır bağımlılık.
2. **Aşama 0.2–0.4** — debounce + uzak politika + capability sondajı (bayrak kaldırma ön koşulu).
3. **Aşama 1.1–1.2** — `calamine` + csvlens-tarzı grid. Tek yeni bağımlılık.
4. **Aşama 2** — Edit. Kararı POC'a bağla.

**Bu araştırmanın en önemli tek dosyası:** `src/fm/preview_capability.rs:126-140` — xlsx/pdf/docx
için ayrılmış yol **boş bekliyor**.

---
*v1.0.0 — 2026-07-24 · reference-registry 5-adım pipeline Adım-1/3 artefaktı.*
*Damıtılmış pattern kataloğu: `docs/patterns/document-rendering.md` · Kaynak registry:
`docs/references/document-rendering.md`.*
