---
doc: herdr-analysis
domain: document-rendering
subject: herdr iç durum — PNG/XLSX/PDF önizleme altyapısının kod gerçeği
created: 2026-07-24
method: kaynak okuma + codebase-memory-mcp + evidence/TASKS taraması; test SAYIMLARI ölçüldü, test SONUÇLARI çalıştırılmadı
status: canonical — her iddia (claim, evidence=dosya:satır, confidence)
git_note: >
  /docs/* herdr .gitignore'da IGNORED (yalnız /docs/next/ istisna) → lokal yaşar, upstream'e sızmaz.
  Makine kopyası: ~/.cartography/herdr-document-rendering-*
agentic_triggers:
  - "png önizleme · image preview · kitty graphics · experimental.kitty_graphics"
  - "xlsx · pdf · docx · belge önizleme · document viewer · MetadataOnly"
  - "preview_capability · PreviewProviderSet · OptionalPlugin · plugin previewer"
  - "bounded worker · generation · stale reject · image_preview_worker"
  - "dosya edit · içerik yazma · hücre editi"
related:
  - docs/analysis/2026-07-24-document-render-ecosystem.md
  - docs/analysis/2026-07-24-architecture-seams.md
  - docs/references/document-rendering.md
  - docs/patterns/document-rendering.md
  - .codex/evidence/b2-image-dependency.md
  - .codex/evidence/files-visibility-preview-plugin-research.md
  - .codex/evidence/files-preview-capability-test-points.md
---

# herdr — Dosya Render/Edit İç Durumu (BÖLÜM 4A)

**Kapsam:** `/home/user/projects/herdr`, branch `feat/native-fm`. İnceleme **salt-okuma** yapıldı: hiçbir kaynak dosya değiştirilmedi, `cargo build`/`cargo test` çalıştırılmadı, herdr server/socket'e dokunulmadı, `.superpowers/` açılmadı, git mutasyonu yapılmadı.

**Metot:** codebase-memory-mcp (`home-user-projects-herdr`, 24.357 node / 129.892 edge, `status: ready`) ile keşif → doğrudan kaynak okuma ile doğrulama → `.codex/evidence/`, `.codex/TASKS.md`, `docs/superpowers/`, `.local/prd/` taraması.

**Kanıt sözleşmesi:** Her iddia `(claim, evidence = dosya:satır veya birebir alıntı, confidence 0..1)` üçlüsüyle verilir. Çıplak iddia yoktur. Doğrulanamayan her şey **açıkça işaretlidir** (§I sonu).

**Bu dosyanın amacı:** Bu, tek seferlik bir rapor değil, **kalıcı referans havuzudur**. Gelecekteki her değerlendirme turu (belge sağlayıcısı bağlama, edit sürecine geçiş, plugin adaptörü) buradan başlayacak. Bu yüzden §K, §L ve §M bölümleri "o gün geldiğinde ne okunacak, ne karar verilecek" sorusuna önceden cevap verir.

---

## İÇİNDEKİLER

| Bölüm | Başlık |
|---|---|
| §0 | Üç temel soruya net cevap |
| §A | Yetenek matrisi (+ A.1 ölü dal anomalisi) |
| §B | Görsel render boru hattı (çağrı zinciri, protokol, kapılar, verdict) |
| §C | Mimari engeller (PNG / XLSX / PDF) |
| §D | Politika engelleri (tam alıntılarla) |
| §E | 13 yeniden kullanılabilir kalıp |
| §F | Editleme yüzeyi — bugünkü gerçek |
| §G | Planlanmış ama yapılmamış işler |
| §H | Boşluk grid'i (hedef ⟷ gerçek) |
| §I | Kanıt sözleşmesi özeti + doğrulanamayanlar |
| §J | Karar girdisi (üç cümle) |
| **§K** | **Bir belge sağlayıcısı bağlamak: adım adım kontrol listesi** |
| **§L** | **EDIT ihtiyacı doğduğunda okunacak dosyalar ve verilecek kararlar** |
| **§M** | **Bu turda İNCELENMEYEN iç yüzeyler** |

---

## ÖZET (üç cümle)

Herdr'da **PNG önizleme gerçek ve canlı bir üretim yolu** — decode (`image` crate) → bounded worker → Kitty graphics protokolü → host terminale doğrudan byte yazımı — ama **varsayılan olarak KAPALI** (`experimental.kitty_graphics = false`).

**XLSX ve PDF için render yok**: bu uzantılar bilinçli bir mimari kararla `MetadataOnly` sınıfına konmuş, ekranda yalnızca `"optional document viewer"` metni çıkıyor; ayrıştırıcı, decoder veya çizim yolu **hiç yazılmamış**.

**Hiçbir dosya türü için içerik editleme yok** — FM'in mutasyon yüzeyi tamamen dosya-sistemi seviyesinde (rename/copy/move/delete), bayt seviyesinde değil.

---

## §0. ÜÇ TEMEL SORUYA NET CEVAP

### Soru 1 — Bugün bir PNG dosyasına tıklandığında ekranda ne oluyor?

**İKİ farklı sonuç var, tek belirleyici: `experimental.kitty_graphics` config bayrağı (varsayılan `false`).**

#### Durum A — bayrak KAPALI (varsayılan, yani normal kullanıcı deneyimi)

Ekranda **gerçek görsel çizilmez**. Trail detay panelinde sarı renkli tek satır metin çıkar: `"(Kitty graphics req.)"`.

```rust
// src/ui/file_manager.rs:1000-1001
let label_and_style = if !app.kitty_graphics_enabled {
    Some(("(Kitty graphics req.)", styles.warning))
```

Bu davranış testle kilitli: `src/ui/file_manager.rs:1669-1697` → `image_preview_has_explicit_non_kitty_fallback_and_ready_content_is_clear`; test hem fallback metnini (`:1675` `assert!(fallback.contains("(Kitty graphics req.)"))`) hem de bayrak açıkken metnin kaybolduğunu (`:1695-1697`) doğruluyor. **conf 0.95**

#### Durum B — bayrak AÇIK

**Gerçek piksel çizilir.** Ancak yedi koşulun **hepsi** sağlanmalı:

| # | Koşul | Kanıt |
|---|---|---|
| 1 | `config.experimental.kitty_graphics = true` | `src/config/model.rs:1018` |
| 2 | `direct_attach_requested == false` (yani `herdr attach` ile pane'e doğrudan bağlanılmamış) | `src/client/mod.rs:1308-1309` |
| 3 | `app.mode == Mode::Terminal` (overlay/dialog açık değil) | `src/kitty_graphics.rs:353, 364-374` |
| 4 | `HostCellSize::is_known()` — hücre piksel boyutu biliniyor | `src/kitty_graphics.rs:51-53`; bilinmezse 8×16 fallback (`:55-61`) |
| 5 | Host terminal Kitty graphics protokolünü destekliyor (Kitty / Ghostty) | protokol byte'ları doğrudan host'a yazılıyor: `:326-345` |
| 6 | Uzantı desteklenen formatta: **yalnızca PNG / JPEG / GIF / WebP** | `src/fm/image_preview.rs:219-224` |
| 7 | Boyut limitleri aşılmamış | `src/fm/image_preview.rs:11-15` |

Koşullar sağlanmazsa ara durumlar **metin olarak** görünür:

| Durum | Ekran metni | Kanıt |
|---|---|---|
| Henüz geometri yayınlanmadı | `"(image preview pending)"` | `ui/file_manager.rs:1004-1006` |
| Decode sürüyor | `"(loading image...)"` | `:1007-1009` |
| Hazır | *(metin yok — piksel çizilir)* | `:1010` |
| İzin hatası | `"(permission denied)"` | `:1043` |
| Boyut limiti | `"(image too large)"` | `:1044-1048` |
| Format desteklenmiyor | `"(unsupported image)"` | `:1049` |
| Decode/panik hatası | `"(image decode failed)"` | `:1050-1052` |
| Diğer | `"(image preview unavailable)"` | `:1053-1056` |

### Soru 2 — `src/kitty_graphics.rs` üretimde canlı mı, yoksa yazılmış-bağlanmamış mı?

**CANLI — yazılmış, bağlanmış, üretim kodundan çağrılıyor, test edilmiş; ama feature-gated (varsayılan kapalı).** İskelet değil, ölü kod değil.

#### Onu çağıran üretim kodu (test dışı, 7 ayrı nokta)

| Çağıran | Satır | Ne yapıyor |
|---|---|---|
| `src/app/mod.rs` | `:409` | `kitty_graphics::set_enabled(config.experimental.kitty_graphics)` — global anahtarı kuruyor |
| `src/app/mod.rs` | `:134`, `:893` | `image_preview_cell_size: HostCellSize` — AppState'te hücre boyutu taşınıyor |
| `src/app/mod.rs` | `:1174` | `clear_all_host_graphics()?` — yaşam döngüsü temizliği |
| `src/app/image_preview_worker.rs` | `:12` | `use crate::kitty_graphics::file_manager_image_target` — worker decode hedefini buradan alıyor |
| `src/ui.rs` | `:165, 173, 193, 201, 217, 265, 520` | `HostCellSize` render hattı boyunca taşınıyor |
| `src/main.rs` | `:725-726`, `:791-792` | `is_enabled()` + `clear_all_host_graphics()?` — çıkışta host temizliği |
| `src/pane.rs` | `:1684`, `:1816` | `is_enabled()` — pane grafik yolu |
| `src/app/input/terminal.rs` | `:708` | `HostCellSize::default()` |

**conf 0.95** — çağrı noktaları grep ile sayıldı, hepsi `#[cfg(test)]` dışında.

#### Testleri

| Dosya | Test sayısı |
|---|---|
| `src/kitty_graphics.rs` | **21** |
| `src/fm/image_preview.rs` | **9** |
| `src/app/image_preview_worker.rs` | **8** |
| **Toplam bu hatta** | **38** |

*(Ölçüm yöntemi: `grep -c '#\[test\]'`. Bu bir **sayım**, bir **geçme raporu değil** — bkz. §I doğrulanamayanlar.)*

**En güçlü test:** `file_manager_ready_image_reuses_upload_cache_and_cleans_up_on_close` (`src/kitty_graphics.rs:1418-1543`). Gerçek `AppState` + `compute_view()` üzerinden çalışıyor ve şunları doğruluyor:

| Doğrulanan | Satır |
|---|---|
| Upload komutu üretiliyor: `a=t,t=d,f=32,s=80,v=64` | `:1490` |
| Display komutu + doğru imleç konumu | `:1492-1500` |
| İkinci frame'de **sıfır bayt** (dedup çalışıyor) | `:1509-1511` |
| İçerik değişince `a=d,d=I` + yeni `a=t` + `a=p` | `:1531-1535` |
| FM kapanınca `a=d,d=I` temizliği + cache boş | `:1538-1542` |

Ek olarak `file_manager_ready_image_placement_uses_trail_detail_content_rect` (`:1277-1344`) placement'ın Trail detay panelinin **tam content rect'ini** kullandığını, `file_manager_image_placement_is_centered_bounded_and_client_local` (`:1347-1415`) ortalama + sınır + bozuk RGBA reddini doğruluyor.

#### Bağımsız üçüncü kanıt: planlama → uygulama izi

`.local/prd/native-file-manager-DECISION.md:45` (2026-07-13 tarihli, kod okumasıyla yazılmış) "Path β" için şunu öngörmüş:

> *"**Pane-bağı SADECE 2 yerde:** (1) `collect_visible_placements`'ın `app.view.pane_infos` gezmesi (bypass — kendi placement'ımızı veririz), (2) dedup anahtarı `sources: HashMap<(PaneId,u32),u32>` + `host_image_id(pane_id,...)` (**sentetik/rezerve bir `PaneId` yeter**). → **Yeni kod ≈ (a) decode+resize, (b) `KittyImagePlacement`+`HostPlacement` inşa [sentetik pane_id], (c) `encode_local_pane_graphics`'in kardeşi. Encoder/dedup/tmux-frame HEPSİ HAZIR.**"*

Kodda birebir gerçekleşmiş:

```rust
// src/kitty_graphics.rs:23
const FILE_MANAGER_PREVIEW_PANE_RAW: u32 = u32::MAX;
// src/kitty_graphics.rs:190
pane_id: PaneId::from_raw(FILE_MANAGER_PREVIEW_PANE_RAW),
```

Ve `collect_visible_placements` bypass'ı da uygulanmış:

```rust
// src/kitty_graphics.rs:392-398
let placements = if app.file_manager.is_some() {
    collect_file_manager_image_placement(app, cell_size, &uploaded_images)
        .into_iter().collect()
} else {
    collect_visible_placements(app, terminal_runtimes, cell_size, &uploaded_images)
};
```

Yani planlanan spike **uygulanmış ve üretime girmiş**. **conf 0.95** (iki bağımsız kaynak: kod + planlama belgesi).

#### Tek "iskelet" kalıntı

`path_beta_real_host_probe` (`src/kitty_graphics.rs:1552-1583`) `#[ignore = "requires an explicit throwaway Kitty/Ghostty host and --no-capture"]` işaretli — gerçek host'ta göz doğrulaması **manuel** yapılıyor, CI'da çalışmıyor.

### Soru 3 — XLSX ve PDF için sıfır mı, kısmi mi?

**Render/parse açısından TAM SIFIR. Ama üç yerde "tanıma" var — bilinçli bir "yapmama kararı" olarak.**

| Katman | XLSX/PDF durumu | Kanıt | conf |
|---|---|---|---|
| Bağımlılık | **SIFIR** | `grep -rn "calamine\|pdfium\|mupdf\|spreadsheet" src/` → 0 sonuç; `Cargo.toml:21-48` tam bağımlılık listesi | 0.95 |
| Parser / decoder | **SIFIR** | hiç kod yok | 0.95 |
| Render yolu | **SIFIR** | çizim dalı yok | 0.95 |
| **Uzantı tanıma (VAR)** | `MetadataOnly{DocumentMetadata}` | `preview_capability.rs:126-136`: `"pdf","doc","docx","odt","rtf","xls","xlsx","ods","ppt","pptx","odp"` | 0.98 |
| **İkon/kind sınıflandırması (VAR, AYRI liste)** | ikon amaçlı | `src/fm/entry_kind.rs:168`: `"md"\|"mdx"\|"rst"\|"txt"\|"pdf"\|"doc"\|"docx"\|"odt"\|"xls"\|"xlsx"\|"ods"...` | 0.9 |
| **Ekran çıktısı (VAR)** | tek satır metin | `"optional document viewer"` — `preview_capability.rs:34` | 0.95 |
| **Test/fixture (VAR)** | 4 ayrı yerde | `preview_capability.rs:246-251` (`manual.pdf`), `ui/file_manager/trail_view.rs:1701-1705`, `ui/visual_fixture.rs:1250-1251`, Playwright baseline `tests/visual/fixtures/generated/vis-14-trail-metadata-preview.json` | 0.95 |

**Ayrıca "binary tespiti" ayrı bir yol:** `.bin/.exe/.dll/.so/.dylib/.class/.wasm/.o/.a/.pyc` → `MetadataOnly{BinaryMetadata}` → `"binary file"` (`preview_capability.rs:162-171`). Ve içerik bazlı NUL-bayt tespiti: `text_preview.rs:128` → `TextPreviewError::Binary` → `"(binary file)"` (`ui/file_manager.rs:1062`). XLSX bu yola **düşmez** çünkü uzantısı zaten daha önce yakalanıyor.

**Bu sıfır bilinçli:** `.codex/evidence/files-preview-capability-test-points.md:11` — *"PDF/office | metadata-only | optional plugin + metadata fallback | **no parser/process in native render**"*.

---

## §A. YETENEK MATRİSİ

Karar mantığının tek otoritesi: `preview_capability()` — `src/fm/preview_capability.rs:74-174`.

Bu fonksiyon **saf**. Dosya başlığındaki sözleşme (`:1-5`):

> *"Pure file-preview provider selection. Capability selection is client-local prepared state. It never reads the filesystem, checks `PATH`, loads configuration, spawns a process, or mutates file-manager navigation."*

**conf 0.98** (kod + dokümante edilmiş sözleşme).

| Dosya türü | Bugün ne oluyor | Kod yolu (dosya:satır) | Sınır / limit | Test | Durum |
|---|---|---|---|---|---|
| **Dizin / symlink-dizin** | Önizleme YOK — içerik Trail kolonlarında | `preview_capability.rs:79-86` → `Unsupported{DirectoryUsesTrail}` | — | ✅ `:214-218` | **Canlı** (tasarım gereği) |
| **UTF-8 metin / kaynak / config** | Bounded okuma + syntect renklendirme | `:173` → `read_text_preview` (`text_preview.rs:113-162`) → `highlight_text_preview` (`:189-227`) | **64 KiB** hard tavan (`text_preview.rs:11`), **128 satır** highlight (`:13`), NUL bayt → `Binary` (`:128`), UTF-8 sınır koruması (`:164-174`) | ✅ 17 test (`app/file_preview_worker.rs`) | **Canlı** |
| **Markdown** (`.md/.markdown/.mdown`) | Düz metin (plugin sağlayıcı yoksa) | `:123-125` → `plugin_or_fallback(..., NativeText)` | Metinle aynı | ✅ `:331-341` | **Canlı** (zengin render YOK) |
| **PNG / JPEG / GIF / WebP** | Gerçek piksel render (Kitty) — **bayrak açıksa** | `:109-111` → `NativeImage` → `image_preview_worker.rs:128` → `kitty_graphics.rs:226-257` | 64 MB encoded, 32.768 px kenar, 64 Mpx, 256 MB decoded, 64 MB RGBA çıktı (`image_preview.rs:11-15`) | ✅ 9+8+21 = 38 test | **Canlı ama feature-gated** |
| **PDF / DOC(X) / ODT / RTF / XLS(X) / ODS / PPT(X) / ODP** | Sadece metin: `"optional document viewer"` | `:126-136` → `MetadataOnly{DocumentMetadata}`, etiket `:34` | — (hiç okuma yok) | ✅ `:246-251` + VIS-14 Playwright baseline | **YOK** (bilinçli) |
| **Arşiv** (`zip/tar/gz/bz2/xz/7z/rar/zst` + `.tar.gz/.tar.bz2/.tar.xz`) | `"optional archive viewer"` | `:137-149` | — | ✅ `:260-265` | **YOK** (bilinçli) |
| **Ses/Video** (`mp3/flac/wav/ogg/m4a/aac/mp4/mkv/mov/avi/webm/mpeg/mpg`) | `"optional media viewer"` | `:150-161` | — | ✅ `:266-279` | **YOK** (bilinçli) |
| **Genel ikili** (`bin/exe/dll/so/dylib/class/wasm/o/a/pyc`) | `"binary file"` | `:162-171` | — | ✅ `:280-286` | **YOK** (bilinçli) |
| **Kırık symlink / özel dosya** | `"broken symlink"` / `"special file"` | `:87-96` | — | ✅ `:287-300` | **Canlı** (fail-closed) |
| **Non-UTF-8 / kontrol karakterli yol** | `"path cannot be previewed safely"` | `:98-107` | — | ✅ `:301-307`, `:377-394` (unix `\xff` testi) | **Canlı** (fail-closed) |

### A.1 KRİTİK ANOMALİ — `OptionalPlugin` dalı üretimde ÖLÜ

`preview_capability()`'nin plugin dalı **kodda mevcut**:

```rust
// src/fm/preview_capability.rs:180-196
fn plugin_or_fallback(
    provider: Option<&PreviewPluginProvider>,
    fallback: PreviewFallback,
) -> PreviewCapability {
    if let Some(provider) = provider
        .filter(|provider| provider.platform_supported && !provider.action_id.trim().is_empty())
    {
        return PreviewCapability::OptionalPlugin {
            action_id: provider.action_id.clone(),
            fallback,
        };
    }
    match fallback {
        PreviewFallback::NativeText => PreviewCapability::NativeText,
        PreviewFallback::MetadataOnly(reason) => PreviewCapability::MetadataOnly { reason },
    }
}
```

Ama **tek üretim çağrısı sağlayıcı setini her zaman boş veriyor**:

```rust
// src/fm/trail_snapshots.rs:704
let preview = match preview_capability(path, kind, &PreviewProviderSet::default()) {
```

`PreviewProviderSet::default()` tüm alanları `None` yapar (`preview_capability.rs:66-72`, `#[derive(Default)]`).

Repo genelinde bu tipin `default()` dışında inşa edildiği **hiçbir üretim yeri yok**. Grep sonucu (üretim satırları):

```
src/fm/trail_snapshots.rs:18-19   use ... PreviewProviderSet
src/fm/trail_snapshots.rs:704     preview_capability(path, kind, &PreviewProviderSet::default())
src/fm/mod.rs:23                  pub(crate) mod preview_capability;
```

Diğer tüm eşleşmeler `src/fm/preview_capability.rs` içindeki `#[cfg(test)]` bloğunda (`:202-207` `fn provider(...)`, `:321-329` test seti).

**Sonuç:** PDF / XLSX / arşiv / medya **her koşulda `MetadataOnly`'ye düşer**; plugin adaptörü kayıt yüzeyi henüz yazılmamış. `OptionalPlugin` varyantı üretimde **hiç üretilemez**.

**İki bağımsız kanıt:** (a) grep sonucu, (b) `.codex/TASKS.md:527-529` — FMR-5'te *"Execute dependency order P0 provenance → P1 visibility → P2 status → P3 sidebar mouse → P4 capability matrix → **P5 plugin adapter** → P6 gates → P7 ranking"* maddesi **açık** (`[ ]`). **conf 0.95**

### A.2 İkinci uyarı: uzantı listesi iki yerde yaşıyor

| Yer | Amaç | Satır |
|---|---|---|
| `src/fm/preview_capability.rs:126-136` | Önizleme yeteneği seçimi | `"pdf","doc","docx","odt","rtf","xls","xlsx","ods","ppt","pptx","odp"` |
| `src/fm/entry_kind.rs:168` | İkon / dosya-türü sınıflandırması | `"md"\|"mdx"\|"rst"\|"txt"\|"pdf"\|"doc"\|"docx"\|"odt"\|"xls"\|"xlsx"\|"ods"...` |

Yeni bir belge türü eklenirse **iki yerin de** güncellenmesi gerekir; sadece birini güncellemek ikon-önizleme tutarsızlığı üretir. **conf 0.85**

---

## §B. GÖRSEL RENDER BORU HATTI

### B.1 Tam çağrı zinciri (adım adım)

```
1. Kullanıcı Trail'de bir .png seçer
   → FmState::activate_trail_entry → TrailSnapshots (src/fm/trail_snapshots.rs:695-700)
   → prepare_trail_detail (:703-725) → TrailDetailPreview::Image

2. FM state'e Pending önizleme kurulur (SAF — disk okuması YOK)
   → src/fm/mod.rs:582-593
     FmPreview::File(FmFilePreview::Image(FmImagePreview{
         source_path, generation, state: FmImagePreviewState::Pending }))

3. compute_view() geometriyi hesaplar → Trail detail content_rect
   → src/kitty_graphics.rs:120-143  file_manager_trail_image_content_area()
        · TrailDetailPreview::Image değilse → None
        · preview.source_path != detail.path ise → None   (path otoritesi)
        · snapshot.detail_panel.content_rect döner
   → src/kitty_graphics.rs:87-106   image_geometry_for_content_area()
        hedef piksel kutusu = (rect.width × cell_width_px, rect.height × cell_height_px)

4. Hücre piksel boyutu HOST terminalden sorulur
   → src/kitty_graphics.rs:34-49  HostCellSize::from_terminal()
        crossterm::terminal::window_size()
        width_px  = size.width  / size.columns   (min 1)
        height_px = size.height / size.rows      (min 1)
        başarısızsa → fallback 8×16 px (:55-61)

5. Bounded worker decode eder (input/render thread'i DIŞINDA)
   → src/app/image_preview_worker.rs:126-130
   → src/fm/image_preview.rs:140-185  read_image_preview()
      • metadata.len() > 64 MB → EncodedTooLarge, dosya AÇILMADAN reddedilir (:154-160)
      • File::open + take(limit) + 1-bayt sentinel ile taşma tespiti (:162-182)
      • format whitelist: SADECE Png|Jpeg|Gif|WebP (:219-224)
      • image::Limits ile decoder sınırları (:226-249)
      • decoder.total_bytes() kontrolü (:235-241)
      • EXIF orientation + aspect-fit downscale, ASLA upscale (:319-376; test :626-639)
      • checked_rgba_bytes ile çıktı sınırı (:260, :272, :397-410)
      • catch_unwind panik bariyeri (:412-419) → DecoderPanicked
      • çıktı: PreparedImagePreview{width, height, rgba, data_fingerprint} (ham RGBA8)

6. Sonuç state'e yazılır → FmImagePreviewState::Ready{target, prepared}
   → STALE REDDİ (üç katmanlı):
      · worker slot:   src/app/image_preview_worker.rs:60-63  accepts(generation, key)
      · target eşitliği: src/kitty_graphics.rs:242-246
      · path eşitliği:   src/kitty_graphics.rs:135-137

7. Placement inşası (sentetik pane kimliği ile)
   → src/kitty_graphics.rs:226-257  collect_file_manager_image_placement()
   → src/kitty_graphics.rs:155-224  file_manager_image_placement_in_content_area()
      • pane_id = PaneId::from_raw(u32::MAX)   ← FILE_MANAGER_PREVIEW_PANE_RAW (:23)
      • image_id = 1, placement_id = 1          (:24-25)
      • RGBA uzunluğu w*h*4 değilse placement REDDEDİLİR (:170-174)
      • grid_cols/rows = ceil(px / cell_px) (:177-178)
      • alan taşarsa → None (:181-185)
      • içerik alanında ORTALAMA (:186-187)
      • data yalnızca cache'te YOKSA klonlanır (:205-209, :252-256) → gereksiz alloc yok

8. Kitty protokol baytlarına encode
   → src/kitty_graphics.rs:479-594  encode_graphics_update()
      • upload:  \x1b_Ga=t,t=d,f={fmt},s={W},v={H},i={id},q=2;<base64>\x1b\   (:807-823)
                 3072 baytlık chunk'lar, m=1 (devam) / m=0 (son)            (:1059-1073)
      • display: \x1b[{row};{col}H
                 \x1b_Ga=p,i={id},p={pid},c={cols},r={rows},z={z},C=1,q=2;\x1b\ (:825-857)
                 opsiyonel x/y/w/h (kaynak kırpma) ve X/Y (piksel offset)
      • delete:  a=d,d=I,i={id}   → görsel + tüm placement'ları  (:796-798)
                 a=d,d=i,i={id},p={pid} → yalnız placement       (:800-805)
      • format kodları: Rgb=24, Rgba=32, Png=100                 (:1051-1057)
      • clipping: negatif viewport → kaynak kırpma x/y/w/h       (:859-1001)

9. Host'a yazım — İKİ AYRI YOL
   (a) LOKAL istemci: doğrudan stdout
       → src/kitty_graphics.rs:326-345  paint_local_pane_graphics()
         çerçeveleme: \x1b7 (cursor kaydet) + bytes + \x1b8 (geri yükle)  (:318-324)
         test: path_beta_frames_graphics_without_cursor_drift (:1546-1550)
   (b) UZAK istemci: protokol üzerinden
       → src/server/headless.rs:1304   ServerMessage::Graphics{bytes}
       → src/client/mod.rs:1724-1731   → stdout.write_all(&bytes) + flush
```

### B.2 Protokol detayı (mimari açıdan kritik)

| Gerçek | Kanıt | conf |
|---|---|---|
| Grafik baytları wire protokolünde **birinci sınıf mesaj** | `src/protocol/wire.rs:617-621` — `ServerMessage::Graphics{ bytes: Vec<u8> }`, doc: *"Client-local Kitty graphics bytes to write directly to the host terminal."* | 0.98 |
| Metin frame'i de grafik taşıyabiliyor | `wire.rs:471` — `FrameData.graphics: Vec<u8>`, doc: *"Kitty graphics protocol bytes to apply after the text frame."* | 0.98 |
| Grafik için **ayrı, 16× büyük** frame tavanı var | `wire.rs:22-26` — `MAX_GRAPHICS_FRAME_SIZE = 32 * 1024 * 1024` vs normal `MAX_FRAME_SIZE = 2 * 1024 * 1024`; doc: *"this larger cap is only for explicit image payloads that are naturally much larger after base64 encoding"* | 0.98 |
| Hücre piksel boyutu el sıkışmada taşınıyor | `wire.rs:317-320` (`Hello`), `:349-352` (`Resize`) — doc: *"or 0 when client-side Kitty graphics are disabled"* | 0.95 |
| Client gelen image-id'leri kaydedip çıkışta temizliyor | `client/mod.rs:2177-2199` `record_received_kitty_graphics` / `clear_received_kitty_graphics`; `:2201-2246` id parse | 0.9 |
| Client max frame boyutunu bayrağa göre seçiyor | `client/mod.rs:1527` | 0.9 |
| Aynı görsel iki kez upload edilmiyor (dedup) | `kitty_graphics.rs:520-543` imza karşılaştırması; test `:1214-1226` *"unchanged local image is fully deduplicated"* | 0.95 |
| Görsel değişince eski host görseli siliniyor | `kitty_graphics.rs:598-624` `release_superseded_source_image` | 0.9 |
| Protokol versiyonu | `wire.rs:16` — `PROTOCOL_VERSION = 16` | 0.98 |
| Clipboard görsel taşıma da destekli (ayrı yol) | `wire.rs:28-29` `MAX_CLIPBOARD_IMAGE_PAYLOAD = 16 MB`; `wire.rs:335-341` `ClientMessage::ClipboardImage{extension, data}` | 0.9 |

**Mimari sonuç:** herdr'ın `render()` saflığı korunuyor — piksel ratatui buffer'ında değil, **ayrı bir yan kanaldan** host terminale gidiyor. Yani "grafik baytlarını protokolden geçirme" mimari kısıtı **zaten aşılmış durumda**; yeni bir belge/görsel kaynağı için transport icat etmeye gerek yok.

### B.3 Devre dışı kalma kapıları (5 adet)

| # | Kapı | Kanıt | Sonuç |
|---|---|---|---|
| 1 | **`experimental.kitty_graphics` varsayılan `false`** | `config/model.rs:1017-1018` (doc: *"Experimental local Kitty graphics rendering for attached clients. Default: false."*) + test `:1879-1881` `assert!(!config.experimental.kitty_graphics)` | Kullanıcı elle açmadıkça **piksel yok** |
| 2 | Global anahtar `App::new()`'da bir kez set ediliyor | `app/mod.rs:409` | Çalışma anında değişmez; config değişikliği restart ister |
| 3 | Client tarafı ayrı gate + direct-attach'ta kapalı | `client/mod.rs:1308-1309` — `loaded_config.config.experimental.kitty_graphics && !direct_attach_requested` | `herdr attach` ile pane'e doğrudan bağlanınca kapalı |
| 4 | Sadece `Mode::Terminal` | `kitty_graphics.rs:353` (`mode_ok`), `:364-374` (erken dönüş + `clear_bytes()`) | Overlay/dialog açıkken görseller temizleniyor |
| 5 | Hücre piksel boyutu bilinmiyorsa | `kitty_graphics.rs:51-53` `is_known()`, `:364` | Alan 0 ise iptal; `window_size()` başarısızsa 8×16 fallback |

### B.4 "Canlı mı, iskelet mi?" — verdict

**CANLI, ama opt-in.** Üç bağımsız kanıt:

1. **Kod:** decode → placement → encode → stdout zinciri kesintisiz; sentetik `PaneId` wiring'i tamam (`kitty_graphics.rs:23, 190, 392-412`).
2. **Test:** 38 test; en güçlüsü gerçek `AppState` + `compute_view()` üzerinden upload/dedup/replace/cleanup dörtlüsünü doğruluyor (`:1418-1543`).
3. **Planlama izi:** DECISION §2'de öngörülen tasarım kodda birebir gerçekleşmiş.

**Tek iskelet kalıntı:** `path_beta_real_host_probe` `#[ignore]` (`:1553`) — gerçek host'ta piksel doğrulaması manuel.

---

## §C. MİMARİ ENGELLER (PNG / XLSX / PDF)

### C.1 PNG — render VAR, kalan iş küçük

| Ne var | Ne eksik | Hangi katman değişmeli | Risk |
|---|---|---|---|
| Decode + 6 katmanlı limit + panik bariyeri | — | — | — |
| Kitty upload / display / delete / dedup | **Sixel / half-block / unicode-placeholder fallback YOK** | client + `kitty_graphics.rs` | Kitty desteklemeyen terminalde hiç görsel yok |
| Trail detail'de ortalanmış yerleşim | Zoom / pan / gerçek boyut yok | `compute_view` + input | Düşük |
| Bounded worker + üç katmanlı stale reddi | — | — | — |
| Uzak istemci transportu | — | — | — |
| — | **Varsayılan kapalı** | `config/model.rs:1018` | Kullanıcı özelliği bilmiyor |
| — | **Belgelenmemiş** | `website/src/content/docs/` (18 sayfa) + `docs/next/` içinde konuya dair sayfa bulunamadı | Keşfedilebilirlik sıfır |

**PNG editi:** hiçbir altyapı yok. `image` crate `default-features = false` ile **yalnızca decoder** feature'larıyla alınmış (`Cargo.toml:47`): `features = ["png", "jpeg", "gif", "webp"]`. Encoder yolu, dirty-buffer, undo yığını yok.

### C.2 XLSX — hiçbir şey yok (ama en ucuz eklenti)

| Katman | Bugün | Gereken |
|---|---|---|
| Bağımlılık | Yok | Yeni crate → **politika kapısı** (§D.1) |
| Sınıflandırma | `MetadataOnly` (`preview_capability.rs:129`) | Yeni `PreviewCapability` varyantı (örn. `NativeTable`) |
| State | `FmFilePreview` 4 varyant: `PendingText`/`Text`/`Image`/`Unavailable` (`fm/mod.rs:245-258`) | 5. varyant + bounded satır/sütun limitleri |
| Worker | Metin + görsel için iki ayrı bounded worker | 3. worker **veya** metin worker'ının genelleştirilmesi |
| Render | `render_file_preview` metin/görsel dallı (`ui/file_manager.rs:940-988`) | Tablo çizimi + kolon genişliği hesabı (`unicode-width` zaten var, `Cargo.toml:43`) |
| **Protokol** | — | **Değişiklik GEREKMEZ** (sonuç metin/hücre → mevcut frame yolu) |
| Edit | Yok | Hücre imleci + input state + yazma yolu + `.xlsx` encoder |

**conf 0.85** — XLSX render, PDF'e göre belirgin şekilde ucuz: yeni transport yok, native kütüphane yok, saf Rust seçenek var.

### C.3 PDF — en pahalı

| Katman | Bugün | Gereken |
|---|---|---|
| Bağımlılık | Yok | pdfium/mupdf → **native C + build script** → 4 hedef binary (`CLAUDE.md` Release Channels: `herdr-linux-x86_64`, `herdr-linux-aarch64`, `herdr-macos-x86_64`, `herdr-macos-aarch64`) + Windows beta |
| Render yolu | — | Sayfa → raster → **mevcut PNG boru hattına besleme** |
| Feature gate | — | Kitty flag'ine ek olarak PDF gate |
| Sayfalama | Yok | Sayfa navigasyonu = yeni input/state ekseni |
| Edit | Yok | Kapsam dışı sayılmalı |

**Mimari avantaj:** PDF sayfası raster'a çevrilirse **yeni transport gerekmez**.

`.local/prd/native-file-manager-DECISION.md:45` (kod okumasıyla yazılmış, 2026-07-13):

> *"`encode_graphics_update` (satır 267) **mekanik olarak kaynak-agnostik** — `&[HostPlacement]` alıp upload/display/delete + dedup üretir, nereden geldiğini umursamaz. `encode_upload_image` (satır 587-595) byte'ları **doğrudan `placement.placement.data`'dan** okur (ghostty deposuna çağrı YOK). Yani yerel-decode edilmiş bir görsel, kendi `.data`'sıyla TÜM pipeline'dan (encode/dedup/diff/delete/base64-chunk/tmux-frame) değişmeden akar."*

Bu iddiayı bu turda **kodda doğruladım**: `kitty_graphics.rs:479` imzası (`placements: &[HostPlacement]`) ve `:807-823` gövdesi (`placement.placement.data` doğrudan okuma) tam olarak böyle. Satır numaraları o günden bu yana kaymış (267→479, 587→807) ama davranış aynı. **conf 0.9**

---

## §D. POLİTİKA ENGELLERİ (tam alıntılarla)

### D.1 Bağımlılık politikası — en sert engel

> **"no new Rust dependency unless existing dependencies cannot satisfy a proven need and the plan explicitly justifies it."**
> — `.codex/NEXT-SESSION-PROMPT.md:373-374`

> **"Don't add dependencies without a reason. Check whether existing dependencies cover the need first."**
> — `CLAUDE.md`, Code Conventions bölümü

#### Emsal karar — `image` crate nasıl kabul edildi

Kaynak: `.codex/evidence/b2-image-dependency.md` (2026-07-14).

**Karar cümlesi (`:3-15`):**

> *"## Decision*
> *Use `image 0.25.10` with default features disabled and only the common preview formats enabled when TP-B2.1 first requires the production decoder:*
> ```toml
> image = { version = "0.25.10", default-features = false, features = ["png", "jpeg", "gif", "webp"] }
> ```
> *Keep the existing direct `png 0.17.16` dependency unchanged. It backs the established production `ghostty::decode_png_rgba` path. Consolidating that decoder onto `png 0.18` is a separate behavior-migration concern and is not required for B2."*

**Ölçülen alternatifler (`:17-24`) — birebir:**

| Candidate | Format scope | Exact additional lock packages | Existing package upgrades | Decision |
|-----------|--------------|--------------------------------|---------------------------|----------|
| Existing `png 0.17.16` only | PNG | 0 | 0 | *Rejected: no JPEG/GIF/WebP and no shared orientation/resize API* |
| `image 0.25.10`, PNG only | PNG | 5 | 0 | *Rejected: five packages still leave the user-facing format gap* |
| `image 0.25.10`, common formats | PNG/JPEG/GIF/WebP | **12** | 0 | ✅ *Selected: bounded common-format coverage without default-format bloat* |
| `image 0.25.10`, defaults | Broad defaults plus rayon/AVIF/EXR/TIFF and others | **78** | 0 | *Rejected: unnecessary compile, security, and platform surface* |

**Tam lock deltası da yazılmış (`:26-39`):** `byteorder-lite 0.1.0`, `color_quant 1.1.0`, `gif 0.14.2`, `image 0.25.10`, `image-webp 0.2.4`, `moxcms 0.8.1`, `png 0.18.1`, `pxfm 0.1.30`, `quick-error 2.0.1`, `weezl 0.1.12`, `zune-core 0.5.1`, `zune-jpeg 0.5.15`.

**Kabul için sunulan kanıt seti — yeni bir crate için de aynısı beklenecek:**

| Kriter | Birebir alıntı | Satır |
|---|---|---|
| Build script / proc macro yok | *"**No selected package contains a build script or proc macro.**"* | `:41` |
| Lisans uyumu | *"License metadata is MIT, Apache-2.0, BSD-3-Clause, Unlicense, or Zlib compatible combinations."* | `:41-42` |
| Advisory taraması | *"Package-registry advisory queries returned no advisory for 11 of the 12 selected packages. The two `image` advisories affect only `<0.23.12` and `>=0.10.2,<0.21.3`; neither affects `0.25.10`."* | `:46-48` |
| Windows cross-check | *"A clean `cargo +1.96.1 check --locked --target x86_64-pc-windows-msvc` for the selected feature set passed."* | `:49-50` |
| MSRV uyumu | *"`image 0.25.10` declares Rust `1.88.0`; Herdr pins Rust `1.96.1`."* | `:51` |
| Derleme maliyeti ölçümü | 3 örnekli medyan wall-time + max RSS + target bayt tablosu | `:56-61` |
| Dürüst yorum | *"Wall-time variance is not treated as a speed claim. The reliable cost delta is seven more lock packages and about 2.43 MB more clean check artifacts for common-format coverage; RSS was effectively unchanged."* | `:63-65` |

**xlsx/pdf için anlamı:**

- **`calamine` gibi saf-Rust bir XLSX crate'i bu barajı geçebilir** — build script yok, MIT lisanslı, saf Rust. Ama 12 paketlik `image` bile bu kadar ayrıntılı ölçüm gerektirdiğine göre, **eşdeğer bir evidence dosyası hazırlamak zorunlu**. **conf 0.85**
- **`pdfium` / `mupdf` build script + native C kütüphanesi taşır** → *"No selected package contains a build script"* kriterini **doğrudan ihlal eder**; ayrıca 4 platform binary'si ve Windows beta var. **Politika engeli yüksek.** **conf 0.9**

### D.2 Zorunlu decode limitleri (yeni parser'lar için de geçerli olacak)

> *"`image::Limits` makes width and height strict, but documents `max_alloc` as a best-effort limit that some decoders may ignore. TP-B2.1 therefore must enforce all of these independently before full decode or placement allocation:*
> *1. bounded encoded input bytes;*
> *2. strict decoder width and height;*
> *3. checked width × height pixel count;*
> *4. checked decoder `total_bytes()`;*
> *5. bounded RGBA output bytes;*
> *6. bounded aspect-fit target dimensions and placement bytes."*
> — `.codex/evidence/b2-image-dependency.md:69-78`

> *"Decoder work remains outside render. Unsupported, corrupt, truncated, oversized, zero-area, and arithmetic-overflow inputs must return explicit failures without panic."*
> — `b2-image-dependency.md:80-82`

**Bir XLSX/PDF parser'ı için bu maddelerin karşılığı:** bounded girdi baytı, satır/sütun/sayfa üst sınırı, hücre sayısı çarpım kontrolü, çıktı bellek sınırı, ve **panik yerine tipli hata**.

### D.3 Mimari sınır kararı — PDF/office açıkça PLUGIN tarafında

> *"Use a hybrid boundary:*
> *- Native Herdr core owns directory enumeration truth, exact path identity, Trail state, mouse geometry, status/error projection, lightweight bounded text, and current Kitty image placement.*
> *- A typed preview capability registry inside the client selects a native lightweight provider or an optional external/plugin action.*
> *- **Plugin panes own heavyweight expert workflows such as full file browsing, rendered Markdown, rich git diff, PDF/office tooling, or external commands.***
> *- Plugin failure, absence, timeout, malformed output, or unsupported platform must degrade to an explicit native fallback without changing selection, cwd, Trail, or terminal runtime identity.*
>
> *This is a research decision, not an implementation-complete claim."*
> — `.codex/evidence/files-visibility-preview-plugin-research.md:178-193`

Gerekçesi test-points tablosunda:

> *"| PDF/office | metadata-only | optional plugin + metadata fallback | **no parser/process in native render** |"*
> — `.codex/evidence/files-preview-capability-test-points.md:11`

Ve sınıflandırıcının sözleşmesi:

> *"The classifier consumes only prepared kind, exact path name/extension, and an injected provider set. It performs no filesystem/config/PATH lookup, process spawn, socket access, or navigation mutation."*
> — `files-preview-capability-test-points.md:20-22`

**Bu, XLSX/PDF'i native'e çekmenin önündeki en somut yazılı engeldir.** Aşmak için açık bir karar revizyonu gerekir — sessizce geçilemez. **conf 0.9**

### D.4 Diğer bağlayıcı kurallar

| Kural | Kaynak | Belge işine etkisi |
|---|---|---|
| Runtime/client sınırı | `CLAUDE.md`: *"Do not add new shared behavior that only works through the private TUI client socket."* + *"Use neutral server/API names, not UI-surface names like sidebar, row, card, or widget."* | Belge içeriği **client-local prepared state** kalmalı |
| Render saflığı | `CLAUDE.md`: *"`compute_view()` handles geometry and mutations. `render()` takes `&AppState` and only draws. Never mutate state during render."* | Parser render'da ÇALIŞAMAZ |
| State/runtime ayrımı | `CLAUDE.md`: *"`AppState` is pure data, testable without PTYs or async."* | Belge state'i PTY'siz test edilebilir olmalı |
| `unwrap()` yasağı | `CLAUDE.md`: *"Rust: no `unwrap()` in production code."* | Parser hatası tipli enum olmalı (`TextPreviewError` / `ImagePreviewError` deseni) |
| Bounded worker/queue | `.codex/NEXT-SESSION-PROMPT.md:370`: *"no unbounded history/cache/queue/worker"* | Yeni worker bounded + tek-slot olmalı |
| Kimlik otoritesi | `:371`: *"generation/path/terminal identity is authority, never coordinates alone"* | Stale reddi zorunlu |
| Fail-closed | `:372`: *"stale and ambiguous state consumes inert/fails closed"* | Belirsizlikte önizleme gösterme |
| Platform izolasyonu | `CLAUDE.md`: OS API'leri `src/platform/` altında, `#[cfg(windows)]`/`#[cfg(unix)]` zorunlu | Native kütüphane 4+ hedefte derlenmeli |
| Protokol versiyonu | `CLAUDE.md` + `src/protocol/wire.rs:16` (`PROTOCOL_VERSION = 16`) | Wire değişirse: *"compare `PROTOCOL_VERSION` against the latest released tag. Bump it only if the current source protocol is not already greater than the latest released protocol."* |
| Refactor-risk sınıflandırması | `CLAUDE.md`: *"Treat changes as refactor-risk when they touch two or more core surfaces, persisted state, protocol/API IDs, workspace/tab/pane identity, restore/handoff, agent detection authority, or UI/input state projection."* | Belge yüzeyi 2+ surface'e dokunursa **roundtable + karakterizasyon testi** gerekir |
| Fork disiplini | `.codex/NEXT-SESSION-PROMPT.md:378-380`: *"Acting account: CyPack, external contributor/fork. Never push upstream and never open upstream issue/PR."* | Sadece `origin HEAD:feat/native-fm` + `origin HEAD:master` |
| Staging disiplini | `:382`: *"Use exact-path staging only, never `git add -A`."* | — |
| Adopsiyon anı doğrulaması | `.codex/TASKS.md:521-522` (FMR-4): *"Re-verify exact versions/licenses/security boundaries immediately before adopting any code or runtime dependency."* | Karar eskiyse tekrar doğrula |
| Docs disiplini | `CLAUDE.md`: *"Stable public docs live in `website/src/content/docs/`... Do not document unreleased behavior there during normal feature or fix work."* + unreleased → `docs/next/` | Yeni özellik dokümanı `docs/next/` altına |

---

## §E. YENİDEN KULLANILABİLİR KALIPLAR (belge yüzeyi için hazır)

| # | Kalıp | Nerede | Belge yüzeyine nasıl uyar |
|---|---|---|---|
| **E1** | **Bounded worker + generation slot** | `app/file_preview_worker.rs` (17 test), `app/image_preview_worker.rs` (8 test): `ImagePreviewSlot::sync()` → `Started{generation}` / `Stopped` / `Unchanged` (`image_preview_worker.rs:44-58`), `accepts(gen, key)` stale reddi (`:60-63`) | XLSX/PDF parser'ı **birebir aynı iskeletle** yazılır |
| **E2** | **Closure enjeksiyonlu worker (test edilebilirlik)** | `image_preview_worker.rs:126-137`: `new()` → `with_processor(wake, closure)`; testler sahte processor veriyor | Parser'ı gerçek dosya olmadan test etmeyi sağlar |
| **E3** | **İçerik hash'li iş anahtarı** | `file_preview_worker.rs:38-50` `FilePreviewKey{files_generation, path, preview_generation, content_sha256, truncated}`; `:5` `use sha2::{Digest, Sha256}` | Aynı XLSX yeniden seçilince **yeniden parse edilmez** |
| **E4** | **Panik bariyeri (iki katmanlı)** | `image_preview.rs:412-419` `catch_unwind(AssertUnwindSafe)` → `DecoderPanicked`; ayrıca worker thread'inde `image_preview_worker.rs:149-152` | Üçüncü-taraf parser panik atarsa uygulama düşmez — belge parser'ları için **kritik** |
| **E5** | **Worker alive-guard (thread ölümü tespiti)** | `image_preview_worker.rs:105-116` `ImageWorkerAliveGuard` — `Drop`'ta `alive = false` + `notify_one()` | Parser thread'i çökerse UI kilitlenmez |
| **E6** | **Tipli hata → UI etiketi + stil eşlemesi** | `ui/file_manager.rs:1041-1058` (etiket) ve `:1024-1039` (stil); metin karşılığı `:1060-1075` | Yeni `DocumentPreviewError` için hazır şablon; `warning` vs `error` ayrımı da hazır |
| **E7** | **Kaynak-agnostik grafik encoder** | `kitty_graphics.rs:479-594` `encode_graphics_update(bytes, placements: &[HostPlacement], ...)`; sentetik `PaneId::from_raw(u32::MAX)` emsali (`:23, 190`) | PDF sayfa raster'ı **yeni transport olmadan** beslenir |
| **E8** | **Grafik dedup + temizlik yaşam döngüsü** | `kitty_graphics.rs:298-305` `HostGraphicsCache{images, placements, sources, view}`; `release_superseded_source_image` (`:598-624`); `clear_bytes` (`:659-669`); `update_view` (`:671-677`) | Sayfa değişiminde eski görseli silme mekaniği hazır |
| **E9** | **Detail content_rect otoritesi** | `kitty_graphics.rs:120-143` — placement ve decode hedefi **aynı** rect'i paylaşır; `preview.source_path != detail.path` ise `None` | Belge görünümü için geometri kaynağı hazır; path otoritesi bedava gelir |
| **E10** | **Overlay modu + isim girişi** | `app/file_rename.rs:134-140`: `state.name_input = name; state.name_input_replace_on_type = true; state.enter_overlay_mode(Mode::RenameFile)` | **Hücre editi için doğrudan şablon** |
| **E11** | **Mod-tabanlı tuş yönlendirme** | `app/input/mod.rs:102` `Mode::RenameFile => self.handle_rename_key_via_api(key_event)`; ayrıca mod grupları `:229`, `:284`, `:837` | Yeni edit modu bu 4 noktaya eklenir |
| **E12** | **Doğrulama + reddetme akışı** | `fm/rename.rs:43-122` `validate_rename_name_component` + `RenameNameIssue:30` + `RenameNamePlatform:14` (Windows/Unix farkı); `file_rename.rs` `reject_file_manager_rename(...)` | Hücre değeri doğrulaması aynı desende |
| **E13** | **Plan → preflight → execute → progress → sonuç** | `fm/operations.rs`: `FileOperationRequest:19` → `FileOperationPreflightError:27` → `FileOperationPlan:132` → `execute_file_operation_with_observer:587` / `_with_host:599` → `FileOperationProgressEvent:526` → `FileOperationItemResult:481` → `FileOperationExecutionResult:503`; iptal `FileOperationCancellation:419` | Belge **yazma** işlemi için tam iskelet: atomik, iptal edilebilir, ilerleme raporlu, per-item sonuçlu |
| **E14** | **Onay dialogu geometrisi** | `ui/dialogs.rs:748-757` `file_delete_confirmation_inner_rect`, `:759-780` `file_delete_choose_button_rects` (3 buton), `:782-790` `file_delete_permanent_button_rects` (2 buton); mod→başlık eşlemesi `:52` | "Kaydet / Farklı kaydet / İptal" için hazır |
| **E15** | **Ratatui hücre fixture → Playwright oracle** | `ui/visual_fixture.rs:27-45` `export_cell_fixture(name, buffer)` → `CellFixture{name, width, height, cells[]}`; `tests/visual/*.spec.ts` (9 spec + snapshot dizinleri) | Yeni belge yüzeyi için görsel test **aynı hatta** eklenir; PDF emsali zaten var (`vis-14-trail-metadata-preview.json`) |

*(Not: rapor 13 kalıpla özetlenmişti; kalıcı sürümde E2 ve E5 ayrıştırılarak 15'e çıkarıldı — hiçbir kalıp çıkarılmadı, iki tanesi görünür kılındı.)*

---

## §F. EDİTLEME YÜZEYİ — BUGÜNKÜ GERÇEK

### F.1 Var olan mutasyonlar (hepsi **dosya-sistemi seviyesi**)

| İşlem | Modül | Boyut | Public API | Not |
|---|---|---|---|---|
| Kopyala / Taşı | `src/fm/operations.rs` | 67 KB | `FileOperationKind:13`, `FileOperationRequest:19`, `execute_file_operation:579`, `execute_file_operation_with_observer:587`, `execute_file_operation_with_host:599`, `FileOperationProgressEvent:526`, `FileOperationPhase:519` | Preflight + plan + iptal (`FileOperationCancellation:419`) + per-item sonuç |
| Yeniden adlandır | `src/fm/rename.rs` | 63 KB | `execute_rename_operation:352`, `..._with_observer:364`, `..._with_host:381`, `validate_rename_name_component:43`, `RenameNamePlatform:14`, `RenameNameIssue:30` | Tekil + **toplu**: `BulkRenameOperationRequest:451`, `execute_bulk_rename_operation:695`, `..._with_host:724` |
| Sil | `src/fm/delete.rs` | 29 KB | — | Çöp kutusu (`trash` crate, `Cargo.toml:48`) veya kalıcı; onay dialogu `ui/dialogs.rs:748-790` |
| Oluştur | `src/app/creation.rs` | — | — | Yeni dosya/dizin |

### F.2 Var **olmayan**: içerik editleme

| Aranan | Sonuç | Kanıt |
|---|---|---|
| Metin editörü / dirty buffer | **Yok** | `FmFilePreview` yalnızca okuma varyantları (`fm/mod.rs:245-258`); `TextPreview.content: String` salt-okunur veri (`text_preview.rs:41-46`) |
| Undo/redo yığını | **Yok** | FM'de undo yapısı grep'te bulunamadı |
| Dosya içeriğine yazma | **Yok** | `operations.rs` / `rename.rs` / `delete.rs` API'leri yalnızca path-seviyesi |
| Hücre / tablo state'i | **Yok** | — |
| Kaydet / kaydetme onayı | **Yok** | Mevcut onay dialogları yalnız silme + worktree kapsamında |

**conf 0.9**

### F.3 Bir "hücre editi" için hazır iskele (somut yol haritası)

```
Mode::EditCell  ── eklenecek → src/app/state.rs:1465-1489 (Mode enum, 24 varyant)
  │
  ├─ 1. GİRİŞ    state.name_input = <mevcut hücre değeri>
  │              state.name_input_replace_on_type = true
  │              state.enter_overlay_mode(Mode::EditCell)
  │              emsal: src/app/file_rename.rs:134-140
  │
  ├─ 2. TUŞ      src/app/input/mod.rs:102   → dispatch satırı
  │              src/app/input/mod.rs:229   → metin-modu grubu (RenameWorkspace|Tab|Pane|File)
  │              src/app/input/mod.rs:284   → ikinci grup
  │              src/app/input/mod.rs:837   → üçüncü grup
  │
  ├─ 3. DOĞRULA  src/fm/rename.rs:43-122 deseni
  │              (tipli issue enum + platform farkındalığı + reddetme yolu)
  │
  ├─ 4. DIALOG   src/ui/dialogs.rs:52       → mod → başlık eşlemesi
  │              src/ui/dialogs.rs:748-790  → buton geometrisi (2 veya 3 buton)
  │
  ├─ 5. YAZMA    src/fm/operations.rs:579-599
  │              plan → preflight → execute → observer deseni
  │
  └─ 6. TEST     src/ui/visual_fixture.rs   → hücre fixture export
                 tests/visual/mutation.spec.ts → Playwright oracle
```

**⚠️ KRİTİK UYARI:** `Mode::EditCell` **`wants_ascii_input()` allowlist'ine EKLENMEMELİ** (`src/app/state.rs:1492-1520`). Kod yorumunda gerekçe yazılı:

> *"This is an explicit **allowlist** of the prefix command/navigation realm: any mode NOT listed defaults to leaving the user's IME alone (the safe default), so adding a new text-entry or overlay mode can never silently force ASCII."*

Yani serbest metin girişi modları listede **olmamalı**; listeye eklemek CJK/IME kullanıcılarını bozar.

---

## §G. PLANLANMIŞ AMA YAPILMAMIŞ İŞLER

| ID | Başlık | Nerede | Durum / neden durdu |
|---|---|---|---|
| **FMR-4** | Reference projects and plugin research | `.codex/TASKS.md:512-522` | 3 alt madde ✅ (`herdr-plugin-hunk`, `herdr-file-viewer`/`quicklook`/`reviewr`/`markdown-viewer`, Yazi/Superfile/Broot/Chafa/Circet referansları); son madde ❌: *"Re-verify exact versions/licenses/security boundaries immediately before adopting any code or runtime dependency"* — adopsiyon yapılmadığı için açık |
| **FMR-5** | Integration architecture and delivery | `TASKS.md:523-532` | Hybrid sınır ✅ seçilmiş (`:524-526`); **P5 plugin adapter** + P6 gates + P7 ranking ❌ (`:527-532`) → `OptionalPlugin` dalının ölü kalmasının **doğrudan nedeni** |
| **FMR-0** | Scroll version lab and ranking | `TASKS.md:441-451` | İlk 2 alt madde ✅ (dört checkpoint çıkarıldı, commit kimlikleri kaydedildi); matris koşumu (`:448-449`) + sıralama (`:450-451`) ❌ |
| **Path β spike** | Image preview fizibilite | `.local/prd/native-file-manager-DECISION.md:166-167` ("B0 SPIKE") | **Fiilen kapanmış** — kod uygulanmış (`kitty_graphics.rs:226-257`), ama plan belgesinde hâlâ "spike" olarak duruyor; `path_beta_real_host_probe` `#[ignore]` |
| **RightPanel / Inspector paneli** | Ayrı önizleme paneli | `.codex/evidence/fm5-preview-placement-decision.md:181-203` | **NO-GO**: *"**NO-GO for B and C. Keep A.** This is not a claim that a RightPanel can never be useful. It means the current evidence does not justify paying its architecture and interaction cost for a 4-cell normal-case preview gain when A is already useful at the approved minimum and preserves more navigation context."* Yeniden açmak için 7 önkoşul listelenmiş (`:192-200`) |
| **Sixel / non-Kitty fallback** | — | — | **Hiç planlanmamış**; `chafa` yalnız referans olarak anılıyor: *"multi-protocol terminal image fallback reference (Kitty/Sixel/symbol-based rendering)"* (`files-visibility-preview-plugin-research.md:173`) |
| **XLSX/PDF native render** | — | — | **Hiçbir task / plan / spec yok** — `docs/superpowers/specs/` (17 dosya) ve `docs/superpowers/plans/` (16 dosya) içinde belge-render konulu dosya bulunmuyor |

### G.1 Doküman durumu

`website/src/content/docs/` (stable, 18 sayfa: `agent-skill`, `agents`, `cli-reference`, `concepts`, `configuration`, `how-to-work`, `index`, `install`, `integrations`, `keyboard`, `marketplace`, `persistence-remote`, `plugins`, `quick-start`, `session-state`, `socket-api`, `windows-beta` + `ja`/`zh-cn` çevirileri) ve `docs/next/` içinde **görsel önizlemeyi anlatan kullanıcıya dönük sayfa bulunamadı**.

Yani PNG önizleme **belgelenmemiş**, keşfedilemez, deneysel bir özellik. **conf 0.8** (negatif grep — dosya adı ve içerik taraması; tam metin okuması yapılmadı).

---

## §H. BOŞLUK GRİDİ (kullanıcı hedefi ⟷ bugünkü iç gerçek)

```
  ── herdr dosya render/edit yeteneği · 2026-07-24 · branch feat/native-fm ──
     (🎯 Kullanıcı hedefi: 6 · 🔍 Bugünkü iç gerçek: 6 karşılık + 2 bonus)
┌────┬──────────────────────────┬─────┬──────────────────────────────────────────────────────────┐
│ #  │ 🎯 Kullanıcı hedefi       │ ⟷  │ 🔍 Herdr'ın bugünkü iç gerçeği (kanıtlı)                  │
├────┼──────────────────────────┼─────┼──────────────────────────────────────────────────────────┤
│ 1  │ PNG terminalde RENDER    │ ✅  │ VAR ve canlı: image crate decode → bounded worker →       │
│    │                          │     │ Kitty graphics → host stdout. PNG/JPEG/GIF/WebP.          │
│    │                          │     │ ⚠️ experimental.kitty_graphics VARSAYILAN KAPALI          │
│    │                          │     │ (config/model.rs:1018). Kapalıyken ekranda sadece         │
│    │                          │     │ "(Kitty graphics req.)" (ui/file_manager.rs:1000-1001).   │
│    │                          │     │ 38 test (9+8+21). Sixel/half-block fallback YOK.          │
├────┼──────────────────────────┼─────┼──────────────────────────────────────────────────────────┤
│ 2  │ PNG EDİT                 │ ❌  │ YOK. image crate default-features=false, SADECE decoder   │
│    │                          │     │ (Cargo.toml:47). Dirty-buffer / undo / encoder yok.       │
│    │                          │     │ FM mutasyonu path-seviyesi (rename/copy/move/delete).     │
├────┼──────────────────────────┼─────┼──────────────────────────────────────────────────────────┤
│ 3  │ XLSX tablo RENDER        │ ❌  │ YOK. preview_capability.rs:129 → MetadataOnly.            │
│    │                          │     │ Ekranda tek satır: "optional document viewer".            │
│    │                          │     │ Bağımlılık sıfır (calamine yok). Karar yazılı:            │
│    │                          │     │ "no parser/process in native render"                      │
│    │                          │     │ (files-preview-capability-test-points.md:11).             │
│    │                          │     │ ℹ️ İyi haber: PROTOKOL DEĞİŞİKLİĞİ GEREKMEZ (metin/hücre).│
├────┼──────────────────────────┼─────┼──────────────────────────────────────────────────────────┤
│ 4  │ XLSX hücre EDİT          │ ❌  │ YOK. Ama overlay+input+doğrulama+plan/execute iskeleti    │
│    │                          │     │ HAZIR: file_rename.rs:134-140 · input/mod.rs:102 ·        │
│    │                          │     │ rename.rs:43-122 · operations.rs:579-599 · dialogs.rs:748.│
│    │                          │     │ Eksik: Mode::EditCell + tablo state + xlsx yazıcı.        │
├────┼──────────────────────────┼─────┼──────────────────────────────────────────────────────────┤
│ 5  │ PDF sayfa RENDER         │ ❌  │ YOK. MetadataOnly (preview_capability.rs:129).            │
│    │                          │     │ ℹ️ Grafik boru hattı kaynak-agnostik: sayfa raster'ı      │
│    │                          │     │ HostPlacement olarak beslenebilir, YENİ TRANSPORT YOK     │
│    │                          │     │ (kitty_graphics.rs:479 + DECISION §2:45).                 │
│    │                          │     │ ⛔ Engel: pdfium/mupdf = build-script + native C →         │
│    │                          │     │ b2-image-dependency.md:41 kriterini İHLAL + 4 platform.    │
├────┼──────────────────────────┼─────┼──────────────────────────────────────────────────────────┤
│ 6  │ PDF içi EDİT             │ ❌  │ YOK ve hiçbir planda geçmiyor. En pahalı kalem;           │
│    │                          │     │ mevcut yazılı karar PDF'i açıkça plugin tarafına atıyor   │
│    │                          │     │ (files-visibility-preview-plugin-research.md:188).        │
├────┼──────────────────────────┼─────┼──────────────────────────────────────────────────────────┤
│ +  │ (bonus) Metin + syntax   │ ✅  │ VAR: 64 KiB bounded okuma + syntect (128 satır),          │
│    │  highlight render        │     │ SHA-256 önbellekli worker, 17 test. Tam canlı.            │
├────┼──────────────────────────┼─────┼──────────────────────────────────────────────────────────┤
│ +  │ (bonus) Plugin ile       │ ❓  │ TİP VAR, WIRING YOK: PreviewCapability::OptionalPlugin    │
│    │  harici önizleme         │     │ tanımlı ama tek üretim çağrısı                            │
│    │                          │     │ PreviewProviderSet::default() (trail_snapshots.rs:704)     │
│    │                          │     │ → dal ÜRETİMDE ÖLÜ. FMR-5 "P5 plugin adapter" açık.       │
└────┴──────────────────────────┴─────┴──────────────────────────────────────────────────────────┘
  Açıklama: ✅ hedef karşılanıyor  ❓ kısmi/iskelet var, bağlantı yok  ❌ yok (aksiyon gerekir)
            ⚠️ koşullu/gated  ⛔ politika veya platform engeli  ℹ️ lehte olan mimari gerçek
```

---

## §I. KANIT SÖZLEŞMESİ ÖZETİ

| # | İddia | Kanıt | conf |
|---|---|---|---|
| 1 | PNG önizleme uçtan uca canlı üretim yolu | `image_preview.rs:140-185` + `image_preview_worker.rs:126-175` + `kitty_graphics.rs:226-345` + 38 test | **0.95** |
| 2 | Varsayılan kapalı | `config/model.rs:1017-1018` + test `:1879-1881` + `app/mod.rs:409` + `client/mod.rs:1308-1309` (4 bağımsız nokta) | **0.98** |
| 3 | Kapalıyken metin uyarısı gösteriliyor | `ui/file_manager.rs:1000-1001` + test `:1669-1697` | **0.95** |
| 4 | XLSX/PDF için hiç parser/bağımlılık yok | grep `calamine\|pdfium\|mupdf\|spreadsheet` = 0 sonuç; `Cargo.toml:21-48` tam liste | **0.95** |
| 5 | XLSX/PDF `MetadataOnly`'ye sabitlenmiş | `preview_capability.rs:126-136` + `trail_snapshots.rs:703-725` + `files-preview-capability-test-points.md:11` | **0.95** |
| 6 | `OptionalPlugin` dalı üretimde ölü | `trail_snapshots.rs:704` (`::default()`) + repo genelinde başka inşa yok + `TASKS.md:527` P5 açık | **0.9** |
| 7 | Grafik transportu protokolde birinci sınıf | `wire.rs:22-26, 471, 617-621` + `headless.rs:1304` + `client/mod.rs:1724-1731` | **0.95** |
| 8 | Encoder kaynak-agnostik (PDF raster besleyebilir) | `kitty_graphics.rs:479-594` + `:190` sentetik PaneId + DECISION `:45` | **0.9** |
| 9 | İçerik editleme hiç yok | `fm/mod.rs:245-258` (salt-okuma varyantlar) + `operations.rs`/`rename.rs`/`delete.rs` API'leri path-seviyesi | **0.9** |
| 10 | Yeni bağımlılık politika kapısı sert | `NEXT-SESSION-PROMPT.md:373-374` + `CLAUDE.md` + `b2-image-dependency.md` tam ölçüm emsali | **0.95** |
| 11 | PDF/office yazılı kararla plugin tarafında | `files-visibility-preview-plugin-research.md:186-191` + `TASKS.md:524-526` | **0.9** |
| 12 | Worker sync çağrısı çok noktalı (sessiz hata riski) | `sync_image_preview_worker`: 6 üretim noktası; `sync_file_preview_worker`: 5 üretim noktası (§K.3) | **0.9** |
| 13 | `/docs/*` git tarafından ignore ediliyor | `.gitignore:10-12` — `/docs/*`, `!/docs/next/`, `!/docs/next/**` | **0.98** |

### ⚠️ DOĞRULANAMAYANLAR (dürüstçe işaretli)

| # | Ne doğrulanmadı | Neden | Ne yapılmalı |
|---|---|---|---|
| 1 | **Test SONUÇLARI** | `cargo test` çalıştırılmadı (görev kapsamı: salt-okuma). `.codex/evidence/files-preview-capability-test-points.md:29`'daki `Rust: 3,526/3,526 PASS, 2 skipped` rakamı **o günün kaydından alıntıdır**, benim ölçümüm değil. Benim ölçtüğüm şey test **sayıları** (`grep -c '#[test]'`). | Karar öncesi `just check` çalıştırılmalı |
| 2 | **Gerçek host'ta piksel** | `path_beta_real_host_probe` `#[ignore]` (`kitty_graphics.rs:1553`); gerçek Kitty/Ghostty host + `--no-capture` gerektiriyor | İzole dev test (`.local/ISOLATED-DEV-TEST.md`) ile manuel doğrulama |
| 3 | **Doküman içeriği tam taranmadı** | `website/src/content/docs/` dosya adları ve grep ile tarandı, 18 sayfanın tam metni okunmadı | Belgeleme kararı öncesi `configuration.mdx` tam okunmalı |
| 4 | **`kitty_graphics.rs` son 388 satırı** | Dosya 2.076 satır; 1-1688 okundu, kalan kısım test bloğunun devamı (test sayımı grep ile tam yapıldı) | Encoder değişikliği yapılacaksa tam okunmalı |

---

## §J. KARAR GİRDİSİ (üç net cümle)

1. **PNG zaten var — asıl iş onu "açmak" ve tamamlamak.** En düşük maliyetli kazanç: `experimental.kitty_graphics`'i varsayılan açmak (veya terminal yetenek tespitine bağlamak) + Kitty desteklemeyen terminaller için fallback + belgeleme. **Yeni bağımlılık gerekmez.**

2. **XLSX render, PDF'ten belirgin şekilde ucuz:** protokol değişikliği gerektirmez, saf Rust bir crate ile karşılanabilir, mevcut bounded-worker + metin-render + tipli-hata kalıplarına doğrudan oturur. Tek gerçek engel, `b2-image-dependency.md` formatında bir gerekçe belgesi hazırlamak.

3. **PDF ve her türlü EDİT, mevcut yazılı mimari kararla çelişiyor** (*"no parser/process in native render"*, *"plugin panes own ... PDF/office tooling"*). Bu yönde gidilecekse önce o karar açıkça revize edilmeli — sessizce aşmak repo disiplinine aykırı olur.

---

## §K. BİR BELGE SAĞLAYICISI BAĞLAMAK: ADIM ADIM KONTROL LİSTESİ

> **Bu bölüm ne zaman okunur:** `PreviewProviderSet.documents` (veya `markdown`/`archives`/`media`) doldurulup `OptionalPlugin` dalı canlandırılmak istendiğinde. Yani "PDF'e tıklayınca harici bir görüntüleyici açılsın" veya "XLSX için bir sağlayıcı tanımlayayım" denildiğinde.

### K.1 Neyi değiştiriyoruz — tek cümlelik özet

Bugün `preview_capability(path, kind, &PreviewProviderSet::default())` çağrısındaki **`::default()`** sabiti, gerçek bir sağlayıcı setiyle değiştirilecek. Tip sistemi ve fallback mantığı **zaten hazır**; eksik olan yalnızca "sağlayıcıları nereden okuyup nasıl taşıyacağız" zinciri.

### K.2 Dokunulacak dosyalar — sıra ile

| Sıra | Dosya:satır | Ne yapılacak | Neden bu sırada |
|---|---|---|---|
| **1** | `src/fm/preview_capability.rs:60-72` | `PreviewPluginProvider` / `PreviewProviderSet` tipleri **zaten var** — dokunma. Yalnızca yeni bir sağlayıcı kategorisi gerekiyorsa alan ekle | Tip önce netleşmeli |
| **2** | *(yeni)* Sağlayıcı kaynağı | Sağlayıcı seti nereden gelecek? **Üç seçenek — §K.4'te karar tablosu** | Kaynak belirlenmeden taşıma tasarlanamaz |
| **3** | `src/fm/mod.rs:616` (`FmState`) | Sağlayıcı setini `FmState`'e alan olarak ekle **veya** çağrı anında parametre olarak geçir | `trail_snapshots` buradan besleniyor |
| **4** | `src/fm/trail_snapshots.rs:703-725` (`prepare_trail_detail`) | İmzayı `providers: &PreviewProviderSet` alacak şekilde değiştir; `::default()` sabitini kaldır | **Asıl değişiklik burası** |
| **5** | `src/fm/trail_snapshots.rs:695-700` ve tüm `prepare_trail_detail` çağıranları | Yeni parametreyi ilet | Derleyici zorlayacak — sessiz hata riski düşük |
| **6** | `src/fm/trail_snapshots.rs:36-45` (`TrailDetailPreview`) | `OptionalPlugin` için yeni varyant gerekiyor mu? Bugün `MetadataOnly(String)`'e düşüyor — plugin aksiyonu **çalıştırılabilir** olacaksa `action_id`'yi taşıyan bir varyant lazım | Aksiyon tetiklenebilir olmalı |
| **7** | `src/ui/file_manager.rs` (detay render) | Yeni varyantın nasıl görüneceği: etiket + "Enter ile aç" ipucu | Render saf kalmalı |
| **8** | `src/app/input/file_manager.rs` | Aksiyonu tetikleyen tuş/tık yolu | Input katmanı |
| **9** | Plugin çalıştırma yolu | `cli/plugin.rs` action invoke (bkz. `.local/prd/native-file-manager-DECISION.md:139`: *"`herdr-plugin.toml` fixture: `[[actions]]`(id/title/**contexts**/command)... `cli/plugin.rs:377` action list/invoke"*) | Mevcut plugin altyapısı reuse |

### K.3 ⚠️ SESSİZ HATA RİSKLERİ (en kritik bölüm)

Bu bölüm, "derledi ama çalışmıyor" sınıfı hataları önlemek için.

#### Risk 1 — `sync_*_worker()` çağrısı ÇOK NOKTALI

Worker'ları uyandıran fonksiyonlar tek bir yerden değil, **birden fazla üretim noktasından** çağrılıyor. Yeni bir worker eklenirse **hepsine** eklenmeli; birini atlamak "bazı yollardan girince önizleme gelmiyor" şeklinde **sessiz** bir hata üretir.

**`sync_image_preview_worker` — üretim çağrı noktaları (6):**

| Dosya:satır | Bağlam |
|---|---|
| `src/app/runtime.rs:212` | `changed \|= self.sync_image_preview_worker();` — ana runtime tick |
| `src/app/mod.rs:1204` | `if self.sync_image_preview_worker() { ... }` |
| `src/app/input/file_manager.rs:1418` | `let _ = app.sync_image_preview_worker();` |
| `src/app/input/file_manager.rs:3099` | `let _ = app.sync_image_preview_worker();` |
| `src/app/input/file_manager.rs:3121` | `let _ = app.sync_image_preview_worker();` |
| `src/app/input/file_manager.rs:3126` | `let _ = app.sync_image_preview_worker();` |

**`sync_file_preview_worker` — üretim çağrı noktaları (5):**

| Dosya:satır | Bağlam |
|---|---|
| `src/app/runtime.rs:211` | `changed \|= self.sync_file_preview_worker();` |
| `src/app/input/file_manager.rs:3098` | `let _ = app.sync_file_preview_worker();` |
| `src/app/input/file_manager.rs:3120` | `let _ = app.sync_file_preview_worker();` |
| `src/app/input/file_manager.rs:3125` | `let _ = app.sync_file_preview_worker();` |
| **`src/server/headless.rs:3616`** | `changed \|= self.app.sync_file_preview_worker();` ← **headless/uzak yol AYRI** |

Tanımlar: `src/app/image_preview_worker.rs:268`, `src/app/file_preview_worker.rs:382`.

**conf 0.9** (grep ile sayıldı, test satırları elendi).

**Kritik gözlem:** `sync_file_preview_worker` **`server/headless.rs:3616`'da da** çağrılıyor ama `sync_image_preview_worker` **çağrılmıyor**. Yani metin önizleme headless/uzak yolda senkronlanıyor, görsel önizleme senkronlanmıyor. Yeni bir belge worker'ı eklenirse **hangi yolda çalışması gerektiğine bilinçli karar verilmeli** — aksi halde "lokalde çalışıyor, uzakta çalışmıyor" (veya tersi) sessiz hatası doğar.

*Not: Bu gözlem bir hata iddiası değil — görsel önizlemenin headless'ta senkronlanmaması, Kitty grafiklerinin ayrı `Graphics` mesajıyla gitmesi nedeniyle **kasıtlı** olabilir. Doğrulanmadı; belge worker'ı eklenirken açıkça sorgulanmalı.*

#### Risk 2 — Uzantı listesi iki yerde

`preview_capability.rs:126-136` ve `entry_kind.rs:168` bağımsız listeler (§A.2). Yalnız birini güncellemek: ikon "belge" gösterirken önizleme "metin" davranır (veya tersi). **Derleyici yakalamaz.**

#### Risk 3 — `PreviewProviderSet` filtreleme kuralı

`plugin_or_fallback` sağlayıcıyı **iki koşulla** kabul ediyor (`preview_capability.rs:184-186`):
```rust
.filter(|provider| provider.platform_supported && !provider.action_id.trim().is_empty())
```
Yani `platform_supported = false` veya boş/whitespace `action_id` **sessizce fallback'e düşer** — hata mesajı yok. Sağlayıcı doldurulup da görünmüyorsa **ilk bakılacak yer burasıdır**. Test emsali: `preview_capability.rs:342-352` (*"unsupported-platform providers must fall back"*).

#### Risk 4 — `trail_snapshots` fail-closed davranışı

`prepare_trail_detail` yalnızca `select_file` başarılıysa çağrılıyor (`trail_snapshots.rs:695-701`). Stale index / uyumsuz kolon durumunda hiç çağrılmaz. Yani "sağlayıcı doğru ama detay hiç hazırlanmıyor" durumu bu üst katmandan gelebilir.

#### Risk 5 — Render saflığı ihlali

Sağlayıcı seti **config'ten okunuyorsa**, o okuma **render veya `preview_capability` içinde OLAMAZ** (`preview_capability.rs:3-5` sözleşmesi: *"never reads the filesystem, checks `PATH`, loads configuration"*). Config → `FmState`/`AppState` yükleme anında taşınmalı.

#### Risk 6 — Platform tespiti

`platform_supported` alanını kim dolduruyor? Eğer "komut PATH'te var mı" kontrolüyse, bu da **saf katmanda yapılamaz** — yükleme anında yapılıp boolean olarak taşınmalı.

### K.4 Sağlayıcı kaynağı — üç seçenek ve karar kriteri

| Seçenek | Nasıl | Artı | Eksi | Politika uyumu |
|---|---|---|---|---|
| **A. Config dosyası** | `~/.config/herdr/config.toml` içine `[preview.providers]` bölümü | Kullanıcı kontrolü, kod değişikliği az | Yeni config şeması → `schemars` şema güncellemesi (`Cargo.toml:44`) | ✅ client-local kalır |
| **B. Plugin manifest'i** | `herdr-plugin.toml` `[[actions]]` `contexts=["file"]` üzerinden keşif | Mevcut plugin altyapısı reuse; `cli/plugin.rs:377` hazır | Plugin keşfi I/O gerektirir → yükleme anında yapılmalı | ✅ FMR-5 P5'in hedefi bu |
| **C. Sabit gömülü liste** | Kodda hardcode (`bat`, `glow`, `libreoffice --convert-to`…) | En hızlı prototip | Kullanıcı özelleştiremez; PATH kontrolü gerekir | ⚠️ "no process spawn in native" sınırına dikkat |

**Öneri (kanıta dayalı):** FMR-5'in yazılı planı **B** yönünde (*"P5 plugin adapter"*, `TASKS.md:527`) ve mimari karar da plugin tarafını işaret ediyor (§D.3). **B**, mevcut yazılı kararlarla çelişmeyen tek seçenek.

### K.5 Her adımın test kancası

| Adım | Test nerede yazılır | Emsal |
|---|---|---|
| Sağlayıcı seçimi mantığı | `src/fm/preview_capability.rs` `#[cfg(test)]` | `:319-375` `preview_capability_uses_only_explicit_supported_plugin_providers` — **zaten var, sadece genişletilir** |
| Detay hazırlama | `src/fm/trail_snapshots.rs` `#[cfg(test)]` (24 test) | mevcut aile |
| Render görünümü | `src/ui/file_manager.rs` `#[cfg(test)]` (47 test) | `:1669-1697` deseni |
| Görsel oracle | `tests/visual/trail.spec.ts` + fixture | `vis-14-trail-metadata-preview.json` **birebir emsal** |
| Input/aksiyon | `src/app/input/file_manager.rs` `#[cfg(test)]` | mevcut aile |

### K.6 Hangi hata nasıl görünür — teşhis tablosu

| Belirti | Muhtemel neden | Bakılacak yer |
|---|---|---|
| PDF'de hâlâ `"optional document viewer"` | `PreviewProviderSet` hâlâ `default()` | `trail_snapshots.rs:704` |
| Sağlayıcı tanımlı ama etkisiz | `platform_supported=false` veya boş `action_id` | `preview_capability.rs:184-186` |
| Bazı yollardan çalışıyor, bazılarından değil | `sync_*_worker()` çağrı noktalarından biri atlanmış | §K.3 Risk 1 tablosu |
| Uzakta çalışmıyor, lokalde çalışıyor | `server/headless.rs` senkron yolu eksik | `headless.rs:3616` çevresi |
| İkon "belge" ama önizleme "metin" | İki uzantı listesi ayrışmış | `entry_kind.rs:168` vs `preview_capability.rs:129` |
| Render'da panik / donma | Sağlayıcı çözümlemesi saf katmana sızmış | `preview_capability.rs:3-5` sözleşmesi |
| Detay hiç hazırlanmıyor | `select_file` fail-closed dönmüş | `trail_snapshots.rs:695-701` |

---

## §L. EDİT İHTİYACI DOĞDUĞUNDA OKUNACAK DOSYALAR VE VERİLECEK KARARLAR

> **Bu bölüm ne zaman okunur:** Kullanıcı şu an **yalnızca görüntüleme** istiyor, ancak ileride edit sürecine geçileceğini açıkça belirtti. Bu bölüm o günün **giriş kapısıdır** — sıfırdan keşif yapmaya gerek kalmasın diye.

### L.1 Önce okunacak dosyalar (sıra ile)

| Sıra | Dosya | Neden |
|---|---|---|
| 1 | **Bu doküman §E + §F** | Hazır kalıplar ve bugünkü mutasyon yüzeyi |
| 2 | `src/app/file_rename.rs` (tamamı) | Overlay + input + doğrulama + submit + reddetme zincirinin **canlı örneği** |
| 3 | `src/fm/rename.rs:1-450` | Doğrulama + preflight + plan + execute deseni |
| 4 | `src/fm/operations.rs:1-600` | Progress + iptal + per-item sonuç deseni |
| 5 | `src/app/state.rs:1465-1520` | `Mode` enum + `wants_ascii_input()` allowlist kuralı |
| 6 | `src/app/input/mod.rs:95-300` | Mod dispatch mimarisi |
| 7 | `src/ui/dialogs.rs:740-800` | Onay dialogu geometrisi |
| 8 | `CLAUDE.md` "Runtime/client boundary guardrail" | Edit state'i nereye ait? |
| 9 | `src/protocol/wire.rs:1-30, 300-360` | Protokol versiyonu + mesaj yapısı |
| 10 | `src/fm/watcher.rs` | Dosya değişince ne oluyor — edit ile çakışma riski |

### L.2 Kod yazmadan ÖNCE cevaplanması gereken sorular

#### Soru L.2.1 — Düzenleme tamponu paylaşılan runtime gerçeği mi, TUI sunum durumu mu?

Bu, `CLAUDE.md`'nin **açık sınıflandırma sorusu**:

> *"Before adding state, API fields, events, commands, or socket messages, classify the feature: Shared runtime/session fact: belongs in server state and should be exposed through the JSON API/event path when practical. TUI presentation state: belongs only in the TUI/client layer."*

| Cevap | Sonuç |
|---|---|
| **TUI sunum durumu** (dosya yalnız bu istemcide düzenleniyor, kaydedilince diske yazılıyor) | Protokol değişikliği **gerekmez**; `PROTOCOL_VERSION` 16'da kalır |
| **Paylaşılan runtime gerçeği** (birden fazla istemci aynı tamponu görmeli, kaydedilmemiş değişiklik server'da yaşamalı) | Yeni wire mesajları gerekir → **`PROTOCOL_VERSION` 16 → 17** ve `CLAUDE.md`'deki bump kuralı işler: *"compare `src/protocol/wire.rs::PROTOCOL_VERSION` against the latest released tag. Bump it only if the current source protocol is not already greater than the latest released protocol. Update hardcoded protocol expectations and manual protocol fixtures in tests."* |

**Öneri:** Basit hücre editi için **TUI sunum durumu** yeterli ve çok daha ucuz. Çok-istemcili ortak düzenleme istenmiyorsa protokole dokunmaya gerek yok.

#### Soru L.2.2 — Hangi mimari kararlarla çelişiyoruz?

| Karar | Alıntı | Edit ile çelişkisi |
|---|---|---|
| *"no parser/process in native render"* | `files-preview-capability-test-points.md:11` | Edit, parse **ve** serialize gerektirir → çelişki doğrudan |
| *"Plugin panes own heavyweight expert workflows"* | `files-visibility-preview-plugin-research.md:188` | Belge editi tanımı gereği "heavyweight expert workflow" |
| *"`render()` ... only draws. Never mutate state during render."* | `CLAUDE.md` | Edit state'i `compute_view` veya action katmanında değişmeli, render'da değil |
| *"`AppState` is pure data, testable without PTYs or async."* | `CLAUDE.md` | Düzenleme tamponu saf veri olmalı; I/O worker'da |

**Bu çelişkiler kod yazmadan önce açıkça ele alınmalı** — ya karar revize edilir, ya edit plugin tarafına konur.

#### Soru L.2.3 — Refactor-risk sınıfında mıyız?

`CLAUDE.md`:

> *"Treat changes as refactor-risk when they touch two or more core surfaces, persisted state, protocol/API IDs, workspace/tab/pane identity, restore/handoff, agent detection authority, or UI/input state projection. Before moving code, identify the protected behavior and add or name characterization tests... Run a roundtable for broad refactors and release-risk regressions."*

Bir belge editi **UI/input state projection** + muhtemelen **persisted state**'e dokunur → **en az iki core surface** → **roundtable + karakterizasyon testi zorunlu**.

Kullanılacak test-only araçlar (`CLAUDE.md`'de adı geçiyor):
- `AppState::assert_invariants_for_test()`
- `Workspace::assert_invariants_for_test()`
- `AppState::test_with_adversarial_identity_state()`
- `Workspace::test_adversarial_identity_state()`

### L.3 Risk sınıfları

| Risk | Açıklama | Azaltma |
|---|---|---|
| **Watcher / generation çakışması** | `src/fm/watcher.rs` dosya değişimini izliyor. Kullanıcı düzenlerken dış bir değişiklik gelirse: önizleme generation'ı artar, tampon "stale" sayılıp **atılabilir** → veri kaybı | Düzenleme tamponu generation'dan **bağımsız** yaşamalı; dış değişiklikte kullanıcıya çakışma sorulmalı |
| **Veri kaybı — atomik yazma yok** | Doğrudan üzerine yazma, yazma sırasında çökmede dosyayı bozar | `operations.rs` deseni: geçici dosya + rename (plan/preflight/execute zaten bu mantıkta) |
| **Undo yokluğu** | Yanlış hücre değişikliği geri alınamaz | Bounded undo yığını — ama `NEXT-SESSION-PROMPT.md:370` *"no unbounded history"* → **sınırlı** derinlik |
| **Kaydedilmemiş değişiklikle kapanma** | FM kapanınca / workspace değişince tampon kaybolur | `Mode` geçişlerinde onay dialogu (`ConfirmClose` emsali, `dialogs.rs:910-918`) |
| **Format sadakati** | XLSX yazarken formül/stil/pivot kaybı | Yalnız değer editi + açık uyarı; veya salt-okunur kalma |
| **Eşzamanlı istemci** | İki istemci aynı dosyayı düzenlerse | L.2.1 kararına bağlı; TUI-local ise "son yazan kazanır" + uyarı |
| **Büyük dosya** | 64 KiB metin tavanı var; edit için tam dosya gerekir | Ayrı, daha büyük ama yine **bounded** edit limiti; aşınca salt-okunur |

### L.4 Hazır olanlar (§E'den edit'e doğrudan uyanlar)

| Kalıp | Edit'teki karşılığı |
|---|---|
| E10 (overlay + `name_input`) | Hücre/satır giriş kutusu |
| E11 (mod dispatch) | `Mode::EditCell` yönlendirmesi |
| E12 (doğrulama + reddetme) | Değer doğrulama |
| E13 (plan → execute → progress) | Kaydetme işlemi |
| E14 (onay dialogu) | "Kaydedilmemiş değişiklik var" |
| E1/E2/E4/E5 (bounded worker) | Arka planda serialize + yazma |
| E15 (visual fixture) | Edit ekranının görsel testi |

**Yani edit için gereken UI/işlem altyapısının büyük kısmı zaten mevcut.** Eksik olan: tampon state'i, undo, ve format-özel serializer.

---

## §M. BU TURDA İNCELENMEYEN İÇ YÜZEYLER

> Kapsam sınırı gereği bakılmayan, ama ileride ilgili olabilecek yüzeyler. Her biri için **neden atlandı** ve **hangi soru için bakılmalı** kayıtlı.

| # | Yüzey | Dosya | Neden kapsam dışıydı | İleride hangi soru için bakılmalı |
|---|---|---|---|---|
| 1 | **Clipboard görsel köprüsü** | `src/server/clipboard_image.rs` (147 satır) — `stage()`, `sanitize_extension()` (png/jpg/gif/webp/bmp), 24 saatlik staging TTL, `wire.rs:28-29` `MAX_CLIPBOARD_IMAGE_PAYLOAD = 16 MB` | Dosya **önizleme** hattı değil; uzak yapıştırma köprüsü | "Kullanıcı bir görseli panoya kopyalayıp FM'e yapıştırabilsin" veya "belge ekran görüntüsü alınsın" istenirse. Ayrıca `bmp` uzantısını kabul ediyor ama `image_preview.rs:219-224` BMP decode **etmiyor** — tutarsızlık olabilir, incelenmedi |
| 2 | **Ghostty passthrough grafikleri** | `src/ghostty/` + `vendor/libghostty-vt` | Pane **içindeki** çocuk uygulamanın (örn. pane'de çalışan `yazi`) ürettiği Kitty görselleri; FM önizleme yolundan bağımsız | "herdr içinde `yazi`/`chafa` çalıştırarak belge göstermek" (DECISION'daki **Path α**) değerlendirilirse. `.local/prd/native-file-manager-DECISION.md:39` risk kaydı: herdr child pane'e sabit `TERM=xterm-256color` veriyor → probe sorunu (conf 0.55, **doğrulanmamış**) |
| 3 | **Pane içi Kitty görselleri toplama** | `kitty_graphics.rs:703-765` `collect_visible_placements`, `:433-477` `has_visible_pane_graphics` | FM açıkken bu yol **bypass** ediliyor (`:392-398`); FM önizlemesini etkilemiyor | Belge görüntüleyici bir **pane** olarak çalıştırılırsa (plugin yolu) bu hat devreye girer |
| 4 | **`vendor/libghostty-vt` yama disiplini** | `vendor/libghostty-vt.patches.md`, `vendor/libghostty-vt.vendor.json` | Bu turda hiçbir vendor değişikliği gündemde değildi | Kitty protokol davranışı değiştirilecekse; `CLAUDE.md`: her yama gerekçe + upstream PR + kaldırma koşulu ile indekslenmeli, `just check` bunu doğruluyor |
| 5 | **Terminal tema / palet** | `src/app/theme_sync.rs`, `file_manager_visual_styles(&app.palette)` (`ui/file_manager.rs:999`) | Renk seçimi önizleme **yeteneğini** etkilemiyor | Tablo render'ı eklenirse (XLSX) kolon/başlık renkleri palete uymalı |
| 6 | **Mobil / dar ekran davranışı** | `src/ui/mobile.rs`, `app.mobile_width_threshold` | FM testlerinde `mobile_width_threshold = 0` ile devre dışı bırakılıyor (`kitty_graphics.rs:1306`, `:1435`) | Belge/tablo görünümü dar ekranda ne olacak? `fm5-preview-placement-decision.md:157` uyarısı: *"Tiny/mobile — Existing current-first fail-closed degradation"* |
| 7 | **Surface host / shell katmanı** | `src/ui/surface_host.rs`, `src/ui/shell.rs`, `src/ui/app_dock.rs` | Files yüzeyinin **konumlandırılması**; önizleme içeriğinden bağımsız | Belge görüntüleyici ayrı bir "stage"/surface olacaksa. `fm5-preview-placement-decision.md` RightPanel için NO-GO verdi — o karar burada yaşıyor |
| 8 | **Miller viewport / trail geometrisi** | `src/fm/miller.rs`, `src/fm/trail.rs`, `src/ui/file_manager/trail_view.rs` | Kolon genişliği/kaydırma; önizleme **içeriği** değil | Belge görünümü için daha geniş alan istenirse (kolon sayısı azaltma vs.) |
| 9 | **FM watcher** | `src/fm/watcher.rs` (17 KB), `notify-debouncer-full` (`Cargo.toml:45`) | Önizleme **tazeleme** yolu; bu tur statik yetenek envanteriydi | **Edit'e geçilirse ZORUNLU** (§L.3 birinci risk): dış değişiklik ile düzenleme tamponunun çakışması |
| 10 | **API / socket yüzeyi** | `src/app/api/`, `src/api.rs`, `docs/next/api/herdr-api.schema.json` | Önizleme API'den sürülmüyor | Belge önizleme durumunun dışarıdan sorgulanması istenirse; `CLAUDE.md` runtime/client sınırı burada belirleyici |
| 11 | **`kitty_graphics.rs` satır 1689-2076** | — | Dosya 2.076 satır; 1-1688 okundu, kalanı test bloğunun devamı. Test **sayımı** grep ile tam yapıldı (21) | Encoder/clipping mantığı değiştirilecekse tam okunmalı |
| 12 | **Windows'a özgü davranış** | `src/platform/windows.rs`, `Cargo.toml:53-69` `windows-sys` | Linux ortamında inceleme yapıldı | Belge parser'ı eklenirse Windows derlemesi ve davranışı ayrıca doğrulanmalı (`CLAUDE.md` Windows VM iş akışı) |

---

## EK — İLGİLİ MUTLAK YOLLAR

**Kaynak kod:**
`/home/user/projects/herdr/src/fm/preview_capability.rs` ·
`/home/user/projects/herdr/src/fm/image_preview.rs` ·
`/home/user/projects/herdr/src/fm/text_preview.rs` ·
`/home/user/projects/herdr/src/fm/mod.rs` ·
`/home/user/projects/herdr/src/fm/trail_snapshots.rs` ·
`/home/user/projects/herdr/src/fm/operations.rs` ·
`/home/user/projects/herdr/src/fm/rename.rs` ·
`/home/user/projects/herdr/src/fm/delete.rs` ·
`/home/user/projects/herdr/src/fm/entry_kind.rs` ·
`/home/user/projects/herdr/src/fm/watcher.rs` ·
`/home/user/projects/herdr/src/kitty_graphics.rs` ·
`/home/user/projects/herdr/src/app/image_preview_worker.rs` ·
`/home/user/projects/herdr/src/app/file_preview_worker.rs` ·
`/home/user/projects/herdr/src/app/file_rename.rs` ·
`/home/user/projects/herdr/src/app/runtime.rs` ·
`/home/user/projects/herdr/src/app/input/mod.rs` ·
`/home/user/projects/herdr/src/app/input/file_manager.rs` ·
`/home/user/projects/herdr/src/app/state.rs` ·
`/home/user/projects/herdr/src/app/mod.rs` ·
`/home/user/projects/herdr/src/ui/file_manager.rs` ·
`/home/user/projects/herdr/src/ui/dialogs.rs` ·
`/home/user/projects/herdr/src/ui/visual_fixture.rs` ·
`/home/user/projects/herdr/src/protocol/wire.rs` ·
`/home/user/projects/herdr/src/server/headless.rs` ·
`/home/user/projects/herdr/src/server/clipboard_image.rs` ·
`/home/user/projects/herdr/src/client/mod.rs` ·
`/home/user/projects/herdr/src/config/model.rs` ·
`/home/user/projects/herdr/src/ghostty/mod.rs` ·
`/home/user/projects/herdr/Cargo.toml`

**Karar ve kanıt belgeleri:**
`/home/user/projects/herdr/.codex/evidence/b2-image-dependency.md` ·
`/home/user/projects/herdr/.codex/evidence/files-visibility-preview-plugin-research.md` ·
`/home/user/projects/herdr/.codex/evidence/files-preview-capability-test-points.md` ·
`/home/user/projects/herdr/.codex/evidence/fm5-preview-placement-decision.md` ·
`/home/user/projects/herdr/.codex/NEXT-SESSION-PROMPT.md` ·
`/home/user/projects/herdr/.codex/TASKS.md` ·
`/home/user/projects/herdr/.local/prd/native-file-manager-DECISION.md` ·
`/home/user/projects/herdr/CLAUDE.md`

**Test yüzeyi:**
`/home/user/projects/herdr/tests/visual/` (9 spec + snapshot dizinleri) ·
`/home/user/projects/herdr/tests/visual/fixtures/generated/vis-14-trail-metadata-preview.json`

---

*Bu doküman `/docs/*` altında olduğu için git tarafından ignore edilir (`.gitignore:10-12`); lokal referans havuzunda yaşar, upstream'e sızmaz.*
