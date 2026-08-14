---
doc: herdr-analysis
domain: decision-matrix
subject: altı incelemenin sentezi · öncelik tablosu · dört şeridin ayrı yol haritaları · edit alternatifleri
created: 2026-07-24
method: altı paralel derin incelemenin (vizyon-misyon, chat forensics, mimari seam, belge render iç/dış, custom layout) çapraz-doğrulanmış sentezi
status: canonical — kararlar kullanıcı onaylı; kanıtlar kaynak analizlerde
decision_state: >
  KULLANICI ONAYI 2026-07-24 — Şerit: (1) Sıfır-bağımlılık hızlı kazançlar.
  Edit hedefi: şimdilik SADECE görüntüleme; edit ihtiyacı doğduğunda yeni analiz turu açılacak.
git_note: >
  /docs/* herdr .gitignore'da IGNORED (yalnız /docs/next/ istisna) → lokal yaşar, upstream'e sızmaz.
  Makine kopyası: ~/.cartography/herdr-analysis/
agentic_triggers:
  - "ne yapalım · öncelik · sıradaki adım · karar · roadmap · yol haritası"
  - "alternatif · seçenek · pros cons · maliyet · risk"
  - "belge render kararı · xlsx kararı · pdf kararı · edit kararı"
  - "custom layout kararı · B-chain kararı · layout vanası"
related:
  - docs/analysis/README.md
  - docs/analysis/2026-07-24-vision-mission-state.md
  - docs/analysis/2026-07-24-chat-forensics-codex-cursor-handover.md
  - docs/analysis/2026-07-24-architecture-seams.md
  - docs/analysis/2026-07-24-document-render-internal-state.md
  - docs/analysis/2026-07-24-document-render-ecosystem.md
  - docs/analysis/2026-07-24-custom-layout-state.md
---

# Karar Matrisi ve Yol Haritaları — 2026-07-24

> Bu dosya altı incelemenin **sentezi**dir. Ham kanıtlar kaynak analizlerdedir; burada yalnızca
> kesişimler, karar noktaları ve **her seçeneğin ayrı yol haritası** vardır.
>
> **Neden dört şeridin de yol haritası yazıldı:** Kullanıcı direktifi (birebir) —
> *"tum bu secenekleri de onlarin olasi sonraki adimlar icin referanslari neler onlari da kaydet
> cunku daha sonra farkli amaclar icin benzer bir degerlendirme sureclerine giricez"*.
> Yani seçilmeyen şeritler **iptal değil, ertelenmiş** — girişleri hazır bekliyor.

---

## 0. ONAYLANAN KARAR (2026-07-24)

| Konu | Karar | Gerekçe |
|---|---|---|
| **Aktif şerit** | **Şerit 1 — Sıfır-bağımlılık hızlı kazançlar** | Yeni crate yok, mimari karar yok, mevcut sözleşmeye oturuyor; en yüksek değer/maliyet oranı |
| **Belge edit** | **Şimdilik yalnızca GÖRÜNTÜLEME** | `csvlens` kanıtı: düzenleme olmadan da yüksek değer. Mevcut mimari karar (*"no parser/process in native render"*) korunur |
| **Edit'in geleceği** | **İptal DEĞİL — ertelendi.** İhtiyaç doğduğunda §5'teki alternatif analiz turu başlatılacak | Kullanıcı: *"tam editleme ihtiyaclari durumunda hedef analizleri kaynak referans projeleri patternleri konseptleri tekrar analiz etme surecine gecicez"* |
| **Kalıcılaştırma** | Tüm analizler/kaynaklar/pattern'ler `docs/analysis|references|patterns` + makine kopyası | Kullanıcı: *"hicbir analiz referans projeler kaynaklar falan kessinlikle bosa gidemez silinemez!"* |

---

## 1. SENTEZ — Altı incelemenin kesişimi

### 1.1 Üç bağımsız agent'ın aynı satıra işaret etmesi

```rust
// src/fm/preview_capability.rs:126-136 — YOL AÇIK
if matches_extension(extension.as_deref(), &[
    "pdf","doc","docx","odt","rtf","xls","xlsx","ods","ppt","pptx","odp",
]) {
    return plugin_or_fallback(providers.documents.as_ref(), ...);
}

// src/fm/trail_snapshots.rs:704 — TEK ÜRETİM ÇAĞRISI, SAĞLAYICI BOŞ
preview_capability(path, kind, &PreviewProviderSet::default())
//                                                  ^^^^^^^^^ tüm alanlar None
```

**XLSX/PDF için mimari yol tasarlanmış, kapı takılmış, arkası boş.** Kayıtlı sebep: `FMR-5 → P5 plugin
adapter` görevi hâlâ `[ ]` (`.codex/TASKS.md:527`).

### 1.1.b ⚠️ KRİTİK DÜZELTME (2026-07-24 Katman A araştırması) — sağlayıcı seti TEK BAŞINA ETKİSİZ

İlk sentezde *"documents sağlayıcısını doldur → PDF/XLSX hemen içerik gösterir"* denmişti. **Bu eksikti.**
Zincirde **ikinci bir sessiz kısa devre** var:

```rust
// src/fm/trail_snapshots.rs:709-714 — action_id BURADA ATILIYOR
PreviewCapability::OptionalPlugin { fallback, .. } => match fallback {
//                                            ^^ action_id düşürülüyor
    PreviewFallback::NativeText => TrailDetailPreview::PendingText,
    PreviewFallback::MetadataOnly(reason) =>
        TrailDetailPreview::MetadataOnly(reason.label().to_owned()),
};
```
Ve `TrailDetailPreview` (`trail_snapshots.rs:36-45`) varyantları: `PendingText | Text | Image |
MetadataOnly(String) | Unpreviewable(String)` — **plugin varyantı YOK**.

**Sonuç:** `PreviewProviderSet` doldurulsa bile `OptionalPlugin`, davranışsal olarak fallback'iyle
**birebir aynıdır**. Kod yeşil derlenir, testler geçer, **ekranda hiçbir şey değişmez**.
conf **0.97** (iki bağımsız nokta: `..` pattern'i + enum tanımı, kaynaktan doğrulandı).

→ İş **üç parçalıdır**: ① sağlayıcı kaynağı ② `TrailDetailPreview` plugin varyantı ③ render dalı.
→ Zorunlu koruma: **A2-R1 RED testi** (`optional_plugin_capability_survives_into_trail_detail`) — bu
   test olmadan A2 "yeşil ama etkisiz" biter.

### 1.1.c 🎁 Karşı-bulgu: adaptörün yarısı ZATEN CANLI

Dosya-bağlamlı plugin zinciri üretimde çalışıyor — ama **bağlam menüsünde**, önizlemede değil:
```
InstalledPluginRegistry (state.rs:2385)
  → file_manifest_actions()                    api/plugins/mod.rs:729   [enabled+manifest+platform+dedup]
  → FileManagerContextMenuModel::from_action_bar_with_plugins()  state.rs:1002
  → sağ-tık menüsü  input/file_manager.rs:757  → revalidation  input/modal.rs:702
  → plugin_invocation_params()  state.rs:1117  → sync_file_manager_plugin_action()  mod.rs:210-264
  → start_plugin_command()  runtime.rs:16
```
Hazır olanlar: `PluginActionContext::File` (`api/schema/plugins.rs:347`) · `PluginInvocationContext.file_paths`
(`:382-386`) · saf platform çözümü (`manifest.rs:483-511`, I/O yok) · deterministik sıra + qualified-id
dedup · scheduled-boundary revalidation · **3 test** (`mod.rs:2686`, `:1910`, `:2808`). conf **0.95**.

→ Sağlayıcı setini registry'den türetmek **yeni mimari icat etmek değil, test edilmiş bir deseni
  ikizlemek**tir.

Çapraz doğrulama (üç bağımsız inceleme, farklı yöntemlerle aynı sonuç):

| Bulgu | belge-iç | mimari-seam | ekosistem | custom-layout |
|---|:---:|:---:|:---:|:---:|
| `OptionalPlugin` üretimde ölü | ✔ | ✔ (F1) | ✔ | — |
| Layout persist yazılıyor, okunmuyor | — | ✔ (F2) | — | ✔ (S6) |
| `last_generations: [Option<u32>; 2]` genişleme tuzağı | — | ✔ (F5) | — | ✔ (b2.3) |
| Grafik transportu protokol değişikliği gerektirmiyor | ✔ (B.2) | ✔ (§1) | — | — |

### 1.2 Durum özeti — iki odak alanı

```
  ── herdr · odak alanları · 2026-07-24 · HEAD b48bd903 (feat/native-fm) ──
┌──────────────────────┬────────────────────────────────┬──────────────────────────────────┐
│ Alan                 │ 🟢 HAZIR                        │ 🔴 EKSİK                          │
├──────────────────────┼────────────────────────────────┼──────────────────────────────────┤
│ PNG render           │ Tam boru hattı + 38 test        │ Varsayılan KAPALI (experimental) │
│                      │ Kanıt: 0/271425 piksel farkı    │ non-Kitty fallback yok           │
│                      │ 5 katmanlı decode limiti        │ belgelenmemiş · debounce yok     │
├──────────────────────┼────────────────────────────────┼──────────────────────────────────┤
│ Grafik transportu    │ FrameData.graphics OPAK         │ 32 MiB aşımı SESSİZ düşürüyor    │
│                      │ 32 MiB tavan · SSH çalışıyor    │ (yalnız warn!, kullanıcıya sinyal│
│                      │ splice: atomik tek kare         │  yok → sebepsiz boş kutu)        │
├──────────────────────┼────────────────────────────────┼──────────────────────────────────┤
│ XLSX / PDF           │ Uzantılar sınıflandırılmış      │ documents sağlayıcısı BOŞ        │
│                      │ Plugin argv sözleşmesi hazır    │ parser sıfır · FMR-5 P5 açık     │
├──────────────────────┼────────────────────────────────┼──────────────────────────────────┤
│ Belge EDİT           │ overlay+input+doğrulama+plan/   │ İçerik yazma yolu SIFIR          │
│                      │ execute iskeleti hazır (13 kalıp)│ hiçbir kayıtta geçmiyor         │
├──────────────────────┼────────────────────────────────┼──────────────────────────────────┤
│ Custom layout        │ Bölge modeli + solver + resize +│ ui.rs:303 ShellLayout::default() │
│                      │ persist + input router: TESTLİ  │ → 4 bölge karanlık               │
│                      │ Drag-resize + yatay scroll CANLI│ B1-B4 hiç başlamadı · config yok │
└──────────────────────┴────────────────────────────────┴──────────────────────────────────┘
```

### 1.3 Bağımsızlık bulgusu (kritik)

**Belge şeridi ile custom layout şeridi paralel ilerleyebilir.** Belge yüzeyi için önerilen yol
(**B-1: Miller detay kolonunda görüntüleyici**) `ui.rs:303`'teki layout vanasını açmayı
GEREKTİRMEZ ve `Files Layout V1` kilidini ihlal etmez (V1-L5 zaten *"file activation updates the
detail state"* diyor → **V1.x işi, V2 kararı gerekmez**).

---

## 2. ÖNCELİK TABLOSU (maliyet/değer sıralı)

| # | İş | Bağımlılık | Büyüklük | Risk | Şerit | Değer |
|---|---|---|---|---|---|---|
| 1 | `documents` sağlayıcısı + **`TrailDetailPreview` plugin varyantı** + render dalı (⚠️ üçü birlikte — §1.1.b) | **SIFIR** | S→**M** | Düşük | **1** | PDF+XLSX görünür içerik. ⚠️ Yalnız sağlayıcı = **etkisiz** |
| 2 | Kitty debounce (yazi: 30 ms) + uzak-oturum kalite politikası | **SIFIR** | S–M | Düşük | **1** | SSH'ta donma biter |
| 3 | Task envanteri mutabakatı + `FFO-8` kutusu düzeltmesi | — | XS | Yok | **1** | Sonraki agent yanlış iş yapmaz |
| 4 | DCLICK-6 + FFO-9 fiziksel E2E (**kullanıcı-sahipli**) | — | XS | Yok | **4** | İki program kapanır |
| 5 | Capability sondajı → `experimental` bayrağını kaldır | SIFIR | M | Orta | **1** | PNG herkese açılır |
| 6 | Native XLSX görüntüleme (Miller preview + tam ekran grid) | `calamine` | M→L | Orta | **2** | Asıl "terminal excel" |
| 7 | Custom Layout B1→B2 (cartography + design spec) | — | M | Orta | **3** | 4 karanlık bölge açılır |
| 8 | Layout'u canlı yap (`ui.rs:303`) | — | L | **Yüksek** | **3** | V2 kararı + VIS baseline yenilemesi |
| 9 | CSV hücre editi (`edtui`) | `edtui` | L | Yüksek | **edit** | Edit'in güvenli girişi |
| 10 | XLSX yazma | `umya-spreadsheet` | XL | **Çok yüksek** | **edit** | ⚠️ POC'suz yapılmamalı |
| ❌ | Formül motoru · native PDF raster (`pdfium`) | — | XXL | — | — | **Tavsiye edilmiyor** |

---

## 3. DÖRT ŞERİDİN AYRI YOL HARİTALARI

> Her şerit bağımsız girişe sahiptir. Seçilmeyenler **ertelenmiş**, silinmemiştir.

### ŞERİT 1 — Sıfır-bağımlılık hızlı kazançlar ✅ **AKTİF**

**Hedef:** Yeni crate eklemeden, mimari karar açmadan, mevcut sözleşmeleri kullanarak PDF/XLSX'i
görünür kılmak + SSH deneyimini düzeltmek + kayıt hijyenini sağlamak.

| Adım | İş | Dokunulacak | Test kancası | Referans |
|---|---|---|---|---|
| 1.1 | `PreviewProviderSet` üretim inşası (bugün `default()`) | `src/fm/trail_snapshots.rs:704` · sağlayıcı kaynağı (config veya plugin registry) | `preview_capability.rs:198+` matrisi · yeni RED: `xlsx_selects_plugin_capability_when_provider_present` | belge-iç §A.1 · mimari-seam §E-1 A-1 madde 2 |
| 1.2 | Örnek plugin: `pdftotext` / `xlsx2csv` argv sözleşmesi | plugin manifest + `HERDR_PLUGIN_ACTION_ID`, `HERDR_PLUGIN_CONTEXT_JSON` | plugin smoke fixture (`tests/fixtures/plugin-smoke/`) | ekosistem §E Aşama 0.1 · `docs/next/website/src/content/docs/plugins.mdx:225-246` |
| 1.3 | `platform_supported` + fallback (araç yoksa sessiz hata olmasın) | `preview_capability.rs:181` `plugin_or_fallback` — **zaten doğru desende** | mevcut fallback testleri | ekosistem §G G12 |
| 1.4 | Kitty debounce (yazi `image_delay=30ms` emsali) | `src/app/image_preview_worker.rs` slot yolu | `miller_resize_1000_moves_has_bounded_side_effects` deseni | ekosistem §P05/P06 · `yazi-default.toml:26` |
| 1.5 | Uzak-oturum kalite/kapalı politikası | `PreviewFallback` enum'u **zaten mevcut** — eksik olan uzak-oturum tespiti | yeni RED: uzak oturumda grafik kapalı | ekosistem §P06 tablo (5 mitigasyon) |
| 1.6 | 32 MiB sessiz düşürmeye kullanıcı sinyali | `src/server/headless.rs:3466-3475` | headless frame testleri | mimari-seam §F7 |
| 1.7 | Task envanteri mutabakatı: `FFO-8` kutusu, FMH-4 hayaleti, HANDOFF §8 tazeliği | `.codex/TASKS.md` · `.codex/HANDOFF.md` §8 | — (kayıt işi) | chat-forensics §C.2 grid · §G uyarı 1-2 |

**Çıkış kriteri:** PDF/XLSX seçildiğinde `"optional document viewer"` yerine **gerçek içerik**;
SSH'ta hızlı gezinmede donma yok; task envanteri üç dosyada tutarlı.

**Sonraki doğal adım:** 1.5 tamamlanınca **Adım 5** (capability sondajı → `experimental` bayrağını
kaldır) — ekosistem §P01, `ratatui-image`'ın `query_with_timeout` + `cap_parser` deseni referans.

---

### ŞERİT 2 — Native XLSX görüntüleyici (`calamine`) ⏸️ **ERTELENDİ**

**Hedef:** Plugin'e bağlı olmadan, herdr içinde native tablo görüntüleme (csvlens sınıfı deneyim).

**Giriş kapısı:** `calamine` crate kararı — `b2-image-dependency.md` formatında gerekçe belgesi.

| Adım | İş | Not |
|---|---|---|
| 2.1 | **Bağımlılık gerekçe belgesi** (`.codex/evidence/` altına, `b2-image-dependency.md` şablonuyla) | Build-script taraması · lisans matrisi · advisory sorgusu · Windows cross-check · derleme maliyeti (3 örnekli medyan) · lock paket sayımı |
| 2.2 | `PreviewCapability::NativeTable` varyantı | `preview_capability.rs:45` — saf fonksiyon, 3 kapsamlı test var |
| 2.3 | `TrailDetailPreview::Table` + `FmFilePreview::Table` | `trail_snapshots.rs:36` · `fm/mod.rs:245` — exhaustive match'ler derleyici kalkanı |
| 2.4 | `document_preview_worker.rs` (image worker şablonuyla) | ~1000 satır; `Key{path,generation,target}` + `Slot` + `accepts()` + `catch_unwind` + `AliveGuard` + `Drop` |
| 2.5 | ⚠️ `sync_*_worker()` çağrısını TÜM senkron noktalarına ekle | **Ölçüldü (2026-07-24):** `sync_image_preview_worker` **6**, `sync_file_preview_worker` **5** üretim çağrı noktası. `app/runtime.rs:211` · `app/mod.rs:1204` · `app/input/file_manager.rs:3098,3120,3125`. ⚠️ **Asimetri:** `server/headless.rs:3616` YALNIZ file-preview worker'ı senkronluyor, image worker'ı senkronlamıyor → belge worker'ı eklenirken **bilinçli karar** gerektirir (kasıtlı mı, eksik mi — doğrulanmadı). Detay: belge-iç §K.3 |
| 2.6 | Miller preview'da tablo özeti (dar) | ekosistem §C.3 Katman 1 mockup'ı |
| 2.7 | Tam ekran sayfa modu (arama/filtre/dondurulmuş kolon/fx çubuğu) | ekosistem §C.3 Katman 2 mockup'ı |
| 2.8 | CSV/TSV aynı grid'e | Ek maliyet ~S |

**Kilit teknik kararlar (araştırma sonucu, gerekçeli):**

| Karar | Seçim | Gerekçe |
|---|---|---|
| XLSX okuma | **`calamine` 0.36.0** (MIT, saf Rust, 10,19M indirme, 2026-07-06) | Windows sorunsuz (tek-binary korunur), salt-okuma = düşük risk yüzeyi |
| Formül | **Motor YAZMA.** `worksheet_formula()` metni + `worksheet_range()` önbellek değeri yan yana | `tshts`'in 60+ fonksiyonu formül motorunun ayrı ürün olduğunun kanıtı |
| Bellek | Seyrek `BTreeMap<(row,col), CellValue>` + viewport sanal kaydırma | 1M satır `Vec`'e açmak = OOM (`tv` deseni) |
| Mimari | Core (veri+I/O, TUI'siz) ↔ TUI (render) ayrımı | `garritfra/cell` yapısı herdr'ın *"State is separated from runtime"* ilkesiyle kelime kelime aynı |
| Kolon genişliği | `min(maks_içerik, tavan)` + anlamlı basamak | `tv` deseni; `unicode-width` **zaten bağımlılık** |

**Referans projeler:** `csvlens` (UX şablonu — salt görüntüleyici ama çok yetenekli) · `garritfra/cell`
(Rust+ratatui mimari şablonu) · `tshts` (formül maliyeti kanıtı) · `sc-im` (702 kolon = kapsam
daraltmanın meşruluğu) · `VisiData` (kolon işlemleri fikri). Tam tablo: `docs/references/document-rendering.md`.

---

### ŞERİT 3 — Custom Layout B1→B2 ⏸️ **ERTELENDİ**

**Hedef:** 4 karanlık bölgeyi (TopBar/AppDock/RightPanel/BottomBar) açmanın tasarım kapısını geçmek.

**Blocker teknik değil, yönetişimsel.** Üç bağımsız kayıt aynı şeyi söylüyor:
*"Custom-layout B-chain is separate and starts only from its own approved design/plan."*
Önkoşulu (SF0-SF6 + FM1-FM5) **%100 kapalı**; kod bağımlılığı yok.

| Adım | Artefakt | Durum | İçerik |
|---|---|---|---|
| B1 | `.cartography/custom-layout-SYSTEM-MAP.json` | ❌ yok | 7 mockup bölgesi ↔ `ShellLayout`/`AppDock`/`Stage` seam eşlemesi |
| B2 | `docs/superpowers/specs/…custom-layout-design.md` | ❌ yok | Bölge sözleşmeleri · runtime/client sınıflandırması · no-goal'lar |
| B3 | Implementation plan | ❌ yok | RED adları + beklenen fail'ler + GREEN seam'leri + VIS-ID'ler |
| B4 | Katman katman yürütme | ❌ yok | İlk dilim: file-manager'ı zenginleştiren bölgeler |

**B1'e girmeden bilinmesi gerekenler (custom-layout analizi §"B1 için hazır girdi"):**
- Vana: `src/ui.rs:303` `ShellLayout::default()` + `:306` sabit revision + `:312-318` `resolve_dynamic`
- **R1 riski:** AppDock'u görünür yapmak `Files Layout V1` kilidine göre **V2-sınıfı karar** → VIS-07..25 baseline yenilemesi
- **R2 riski:** `ShellGeometryKey` template kimliği içermiyor → revision artırılmazsa **stale-hit**
- **Anti-pattern uyarısı (kayıtlı):** *"Arbitrary component registry → over-engineering → P4.0 S5 NO-GO"*. Somut bölge tüketicisi eklenmeli, genel registry değil.
- İyi haber: `ShellLayout` **zaten `Deserialize`** (`untagged {Tree, Template}`, `deny_unknown_fields` + `validate()`) → kullanıcı-tanımlı layout için **parser yazmaya gerek yok**

**Bu turda incelenmeyen dış referanslar (ileride bakılacak):** zellij KDL layout, tmux/wezterm pane
tanımları, i3/sway tiling config, Cassowary constraint solver, `ratatui-hypertile` + `tui-studio`
(refpool'da indexed, derinlemesine bakılmadı), CSS Grid track modeli, Zed/VSCode panel sistemi.

---

### ŞERİT 4 — Fiziksel E2E + gate doğrulaması ⏸️ **KISMEN AKTİF** (kullanıcı-sahipli)

**Hedef:** İki açık programı kapatmak + tüm test iddialarını taze kanıta bağlamak.

| Adım | İş | Sahip |
|---|---|---|
| 4.1 | **DCLICK-6** fiziksel E2E — root/ancestor/current/rightmost kolonlarda dosya+klasör tıkla; anında dolgun focus, aynı kolonda ↑/↓, Right→ilk child, akıcı hızlı tıklama, sıfır residue | **Kullanıcı** (agent TUI açamaz) |
| 4.2 | **FFO-9** `TP-FFO-E2E-01` — mouse→key ownership, tek adım wheel, Right/Left, Rail-disabled action'lar, tek dolgun cursor, yoğun input, sıfır residue | **Kullanıcı** |
| 4.3 | `just check` bir kez gerçekten koşsun | Agent veya kullanıcı |

**Komut (izole, canlıya sıfır dokunma):**
```bash
cd /home/ayaz/projects/herdr && HERDR_RENDER_PROF=1 ./.local/herdr-trail-test.sh run
```

**Neden önemli:** Tüm "3.683/3.683 geçti" tipi iddiaların kaynağı aynı ajan zincirinin yazdığı
**korelasyonlu belgeler** (vizyon-misyon §R8, confidence **0,6**). Tek bir gerçek koşum bunu
executable kanıta çevirir.

---

## 4. BELGE EDİT — ALTERNATİF HEDEFLER, PROS/CONS, ADIMLAR

> **Durum: ERTELENDİ (iptal değil).** Kullanıcı: *"tam editleme ihtiyaclari durumunda hedef
> analizleri kaynak referans projeleri patternleri konseptleri tekrar analiz etme surecine gecicez"*.
> Bu bölüm **o turun giriş kapısıdır**.

### 4.1 Neden şimdi değil — üç bağımsız gerekçe

| # | Gerekçe | Kaynak |
|---|---|---|
| 1 | **Kayıtsız hedef.** "Edit" kelimesi ne Cursor/Codex chat'lerinde, ne PRD'lerde, ne `.codex/TASKS.md`'de geçiyor. Tüm kayıtlarda kelime *"preview/render"* | chat-forensics §E.3 |
| 2 | **Mevcut mimari karara aykırı.** *"no parser/process in native render"* + *"plugin panes own heavyweight expert workflows such as ... PDF/office tooling"* | belge-iç §D.3 |
| 3 | **Görüntüleme tek başına yüksek değerli.** `csvlens` kanıtı: regex arama+filtre, dondurulmuş kolon, satır/kolon/hücre seçimi, kopyalama — hepsi düzenleme olmadan | ekosistem §C.1.3 |

### 4.2 Alternatif edit hedefleri — pros/cons

#### Alternatif A — **CSV hücre editi** (en güvenli giriş)

| Pros | Cons |
|---|---|
| Format basit — kayıp riski yok (pivot/grafik/makro yok) | Yine de yeni alt sistem (dirty buffer, undo, kaydetme) |
| `edtui` 0.11.6 (MIT, **2026-07-18** — çok aktif) hazır widget | Watcher/generation çakışması: kendi yazımın önizlemeyi geçersizleştirmesi |
| Mevcut overlay+input+doğrulama+plan/execute iskeleti doğrudan kullanılabilir (13 kalıp) | `Mode::EditCell` eklenmesi 4 router noktasına dokunur |
| Client-local kalabilir → protokol değişmez (tek-istemci varsayımıyla) | ⚠️ `tui-textarea` KULLANMA — 21 aydır durgun (0.7.0, 2024-10-22) |

**Adımlar:** `Mode::EditCell` (`app/state.rs:1465`) → giriş (`file_rename.rs:134-140` emsali) →
tuş (`app/input/mod.rs:102`) → doğrulama (`fm/rename.rs:43-122` deseni) → dialog
(`ui/dialogs.rs:748-790`) → yazma (`fm/operations.rs:579-599` plan/preflight/execute) → test
(`visual_fixture` + `tests/visual/mutation.spec.ts`).
⚠️ `Mode::EditCell` **`wants_ascii_input()` allowlist'ine EKLENMEMELİ** (`state.rs:1492-1520`).

#### Alternatif B — **XLSX yazma** (`umya-spreadsheet`)

| Pros | Cons |
|---|---|
| Tek gerçek oku→değiştir→yaz seçeneği (3.0.1, MIT, saf Rust, 2026-07-13) | ⚠️ **Formül/grafik/pivot/makro korunumu docs'ta BELGELENMEMİŞ** (confidence 0,75) |
| Windows sorunsuz | Kullanıcının Excel dosyasını bozmak, "önizleme yok"tan **çok daha kötü** |
| API basit: `reader::xlsx::read` → `sheet_by_name_mut` → `cell_mut().set_value` → `writer::xlsx::write` | POC şart: yaz-oku-karşılaştır ile kayıp ölçümü |

**Zorunlu ön koşullar:** (a) POC ile kayıp ölçümü, (b) yazmadan önce yedek, (c) desteklenmeyen
özellik tespitinde açık uyarı, (d) `rust_xlsxwriter` **değil** (sıfırdan üretim, mevcut dosyayı düzenleyemez).

#### Alternatif C — **Harici editöre devret** (herdr'ın kendi deseni)

| Pros | Cons |
|---|---|
| **Vizyonla en uyumlu**: *"herdr editörü barındırır, editör olmaz"* | Terminal-içi editör deneyimi yok |
| Mevcut `prefix+e` scrollback editör pane'i deseni (`$VISUAL`/`$EDITOR`) hazır | Belge formatları için terminal editörü sınırlı |
| Sıfır yeni alt sistem, sıfır veri kaybı riski | Kullanıcı beklentisini karşılamayabilir |

#### Alternatif D — **Plugin pane'de edit**

| Pros | Cons |
|---|---|
| Mevcut mimari karara **birebir uyar** (FMR-5 hybrid sınırı) | Plugin ekosistemi henüz yok (P5 adapter açık) |
| herdr çekirdeği hiç değişmez | Plugin'in kendi edit UX'i herdr'dan bağımsız olur |

### 4.3 Edit turu açılırsa — kod öncesi cevaplanacak SORU

> **Düzenleme tamponu paylaşılan runtime gerçeği mi?**
> — CLAUDE.md runtime/client boundary guardrail

| Cevap | Sonuç |
|---|---|
| **Evet** (iki istemci aynı belgeyi açabilir) | Server state + `src/api/` şeması + **`PROTOCOL_VERSION` 16→17** kaçınılmaz |
| **Hayır** (tek-istemci, client-local) | TUI'de kalabilir, protokol değişmez |

Bu karar verilmeden edit'e başlamak = sonradan protokol refactor'u.

### 4.4 Edit turu başlatılırsa okunacaklar (giriş kapısı)

```
1. docs/analysis/2026-07-24-document-render-internal-state.md
      §E (13 yeniden kullanılabilir kalıp) · §F (editleme yüzeyi) · §"EDIT ihtiyacı doğduğunda..."
2. docs/analysis/2026-07-24-architecture-seams.md  §E-1 A-3 (edit maliyet tablosu, 6 madde)
3. docs/analysis/2026-07-24-document-render-ecosystem.md  §C.4 (⚠️ veri kaybı) · §E Aşama 2
4. docs/patterns/document-rendering.md  (anti-pattern G10: sessiz veri kaybı)
5. .codex/evidence/files-visibility-preview-plugin-research.md  (hybrid sınır kararı — revize edilecek mi?)
6. CLAUDE.md  runtime/client boundary guardrail
```

---

## 5. GELECEK DEĞERLENDİRME TURLARI İÇİN — AÇIK BAŞLIKLAR

Kullanıcı: *"yeni ihtiyaclarimiz oldugunda daha fazlasini istedigimizde alternatif kaynaklari
cozumleri referanslari projeleri frameworkleri falan hepsini didik didik arastirip incelicez"*.
Bu turda **kapsam dışı bırakılanların** kaydı:

| Başlık | Neden bu turda değil | Nereden başlanmalı |
|---|---|---|
| DOCX/ODT/PPTX görüntüleme | Odak XLSX/PDF/PNG'ydi | ekosistem §"Bu turda araştırılmayan alternatifler" |
| Markdown zengin render | Bugün düz metne düşüyor (bilinçli) | `termimad`, `mdcat`; FMR-3 matrisi `markdown` yuvası hazır |
| Kod diff görüntüleyici | Ayrı domain | `delta`, `difftastic`, `herdr-plugin-hunk` (FMR-4'te incelendi) |
| Veri formatları (parquet/arrow/jsonl) | Talep yok | `calamine` kararından sonra doğal genişleme |
| SVG / AVIF / HEIC | `image` crate feature seti dışında | `b2-image-dependency.md` format kararı yeniden açılır |
| OCR (taranmış PDF) | Çok ileri | raster yolu açılırsa gündeme gelir |
| Notebook (.ipynb) | Talep yok | — |
| Layout: zellij KDL, i3/sway, Cassowary, hypertile | B1 kapsamı | custom-layout §"İncelenmeyen layout alternatifleri" |
| Multi-client / multi-monitor layout ayrışması | Ayrı mimari tur | `research/multi-monitor-shared-view.md` |
| Plugin marketplace ekonomisi, sponsorluk modeli | Vizyon ekseni | vizyon-misyon §"İncelenmeyen vizyon eksenleri" |
| Codex rollout 59 MB + 66 MB oturumları tam parse | Zaman | chat-forensics §"İncelenmeyen kaynaklar" + reçete |

---

## 6. METODOLOJİ NOTU — Bu tur nasıl yürütüldü

Gelecekte aynı ölçekte bir değerlendirme gerektiğinde kopyalanabilir şablon:

```
1. Durum haritası (tek agent, 5-10 dk)
   pwd · git status/log/branch · codebase-mcp index_status · continuity dosyaları · dizin envanteri
2. Bölümleme (kullanıcı sorusunu bağlama göre katmanlara ayır)
   → bu turda 6: vizyon-misyon · chat forensics · mimari seam · belge-iç · belge-dış · layout
3. Paralel fan-out (her bölüme 1 agent)
   Her prompt'a ZORUNLU: makro scope (İÇİNDE/DIŞINDA) · kanıt sözleşmesi (claim+evidence+confidence)
   · codebase-mcp protokolü · salt-okuma sınırı · Türkçe rapor · "token cimriliği yok"
4. Çapraz doğrulama
   Aynı bulguyu iki bağımsız agent buldu mu? → confidence yükselt. Çelişki var mı? → kaynağa in.
5. Sentez + karar matrisi (koordinatör)
6. KALICILAŞTIRMA (bu tur) — analysis/references/patterns + makine kopyası
```

**Bu turda öğrenilen tuzaklar:**
- `git branch -vv`'deki "behind 742" **lokal checkout** farkıdır — fork↔upstream ölçümü `rev-list --count` ile yapılır (fork 819 commit **önde**)
- codebase-mcp grafiği **navigasyon**, kaynak **otorite** — Rust CALLS kenarları eksik olabilir
- Süreklilik belgeleri **korelasyonlu kaynaktır** — dördü aynı şeyi söylemesi bağımsız doğrulama değildir
- `.codex/evidence/*-version-lab/` gibi kod kopyaları grafiği kirletir (aynı sembol 5 kez)

---

*v1.0.0 — 2026-07-24 · Altı paralel incelemenin sentezi. Kararlar kullanıcı onaylı;*
*seçilmeyen şeritler ertelenmiş, girişleri hazır.*
