---
doc: document-rendering
domain: document-rendering
scope: >
  herdr'ın terminalde BELGE/GÖRSEL render+edit pattern kataloğu (DR1–DR18, DA1–DA12).
  Kardeş katmanlar: docs/patterns/rust-engineering.md (HP1–HP10, Rust mühendisliği) ve
  docs/patterns/tui-composition.md (TUI kompozisyonu). Bu dosya belge/görsel önizleme
  işlerinde onları TAMAMLAR, çakışmada mimari kural (AGENTS.md) kazanır.
created: 2026-07-24
status: canonical (lokal — /docs/* gitignored, upstream'e sızmaz)
agentic_triggers:
  - "görsel önizleme · image preview · png · jpeg · kitty graphics · sixel · protokol"
  - "xlsx · excel · spreadsheet · tablo · csv · hücre · formül · calamine"
  - "pdf · belge önizleme · document viewer · pdftoppm · raster"
  - "preview worker · stale · generation · debounce · cache · iptal"
  - "ssh · uzak · bant genişliği · downscale · fallback"
related:
  - docs/references/document-rendering.md                  # etiket sözlüğü + tier/confidence
  - docs/analysis/2026-07-24-document-render-ecosystem.md   # tam gerekçe + karşılaştırma
  - docs/patterns/rust-engineering.md                       # HP1–HP10 (state saflığı, test kapısı)
  - docs/patterns/native-file-manager.md                    # FM davranış pattern'leri
---

# herdr Belge/Görsel Render Pattern Kataloğu (DR1–DR18)

> Her pattern: **ne / ne zaman KULLAN / ne zaman KULLANMA / kaynak-etiket · confidence**.
> Etiketler `docs/references/document-rendering.md`'de çözülür.
> Satır numaraları 2026-07-24 snapshot'ına göredir (kod değişirse yeniden doğrula).

---

## BÖLÜM 1 — TERMİNAL GRAFİK PROTOKOLÜ (DR1–DR7)

### DR1 — Yetenek sondajı: sabit varsayma, TIMEOUT'lu sor
- **Ne:** Dış host terminalin grafik yeteneğini çalışma anında sorgula; yanıt gelmezse/çöp gelirse
  zarifçe düş. Sondaj **asla asılmamalı** (test ile garanti altına al).
- **Referans zincir:** `Picker.from_query_stdio` → `query_stdio_capabilities` → `query_with_timeout`
  → `cap_parser.Parser.push` (kısmi/çöp toleransı) → `interpret_parser_responses`; tmux ayrı eksen
  (`detect_tmux_and_outer_protocol_from_env`). Asılmama testi: `test_from_query_stdio_no_hang`.
- **KULLAN:** Terminal kimliği bilinmiyorsa (SSH, tmux, bilinmeyen emülatör); ve
  `experimental.kitty_graphics` bayrağını kaldırmadan ÖNCE **ön koşul olarak**.
- **KULLANMA:** herdr'ın kendi VT'sini taşıdığı **iç** pane için — orada yetenek zaten bilinir.
  Sondaj yalnızca dış host terminale karşı anlamlıdır.
- Kaynak: `[ratatui-image-src]` · conf 0.95 · `[herdr-config-experimental]` · conf 1.0
- **herdr durumu:** `HostCellSize::from_terminal` fallback'i doğru (8×16 px), ama **protokol sondajı YOK**.

### DR2 — Yerleştirme/temizleme: küçük harf ≠ bellek temizliği
- **Ne:** Kitty'de görsel bir kez iletilir (`i=<id>`), ayrı yerleştirilir (`p=<placement_id>`).
  Silme `a=d` + `d=` anahtarı. **Küçük harf** (`d=i`) yalnızca yerleşimi siler, veri terminalde kalır;
  **BÜYÜK harf** (`d=I`) veriyi de serbest bırakır.
- **KULLAN:** `d=i` — aynı görseli yakında tekrar göstereceksen (kaydırma, sekme dönüşü):
  yeniden iletim maliyeti sıfırlanır.
- **KULLANMA:** `d=i`'yi kalıcı temizlik sanma → terminal tarafında sızıntı. Dosya değiştiyse veya
  bellek baskısı varsa `d=I`.
- Kaynak: `[kitty-graphics-spec]` · conf 0.95
- **herdr'ın doğru yaptığı:** `HOST_IMAGE_ID_BASE = 10_000` ile id uzayı ayrımı +
  `FILE_MANAGER_PREVIEW_IMAGE_ID = 1` → agent pane'lerinin ürettiği görsellerle çakışma imkânsız
  (`[herdr-kitty-graphics]` · conf 1.0).

### DR3 — Yerleştirme modeli seçimi: mutlak vs sanal (Unicode placeholder)
- **Ne:** İki yol var. (a) **Mutlak yerleştirme** — uygulama konumu hesaplar, her frame yönetir.
  (b) **Sanal yerleştirme** — `a=p,U=1,i=…,c=…,r=…` ile sanal yerleşim kurulur, ekrana `U+10EEEE`
  placeholder karakteri diakritiklerle basılır (satır/kolon diakritikte, id ön-plan renginde);
  görsel metinle birlikte kayar.
- **KULLAN (sanal):** Görsel metin akışıyla kaymalıysa; multiplexer/pane sistemlerinde en sağlam.
- **KULLAN (mutlak):** Sabit bir panel/kolon içinde piksel-hassas yerleşim gerekiyorsa —
  **herdr'ın Miller preview kolonu tam bu durum**.
- **KULLANMA (sanal):** Terminal desteği daha dar; basit tek-panel senaryoda gereksiz karmaşa.
- Kaynak: `[kitty-graphics-spec]` · conf 0.95 · `[herdr-ghostty-bindings]` `KittyPlacementRenderInfo` · conf 1.0

### DR4 — Chunking + iletim ortamı: uzak çalışıyorsan `t=d`
- **Ne:** `t=d` (doğrudan base64) · `t=f` (dosya) · `t=t` (geçici dosya) · `t=s` (paylaşımlı bellek).
  Doğrudan iletimde chunk ≤ **4096 bayt**, son hariç hepsi **4'ün katı**, `m=1` devam / `m=0` bitiş.
- **KULLAN:** `t=d` — herdr uzaktan (SSH) kullanılabildiği için tek güvenli varsayılan.
- **KULLANMA:** `t=f`/`t=s` — terminal ile uygulama farklı makinedeyse **çalışmaz**.
- Kaynak: `[kitty-graphics-spec]` · conf 0.95
- **herdr doğrulaması:** `KITTY_CHUNK_BYTES = 3072` → 4096 altında **ve** 4'ün katı (4×768). ✅

### DR5 — Debounce: hızlı gezinmede iletimi iptal et
- **Ne:** Seçim değişiminden sonra kısa bir gecikme (~30 ms) bekle; bu sürede seçim yine değiştiyse
  önizlemeyi hiç üretme/iletme.
- **KULLAN:** Her önizleme türünde (görsel, PDF, tablo) — ok tuşu basılı tutulduğunda kuyruk oluşmasın.
- **KULLANMA:** Tek-atış açık komutlarda (kullanıcı Enter'a bastı) — orada gecikme sadece yavaşlık.
- Kaynak: `[yazi-preview-config]` `image_delay=30` · `[yazi-preview-cfg-rs]` `deserialize_image_delay` · conf 1.0
- **herdr durumu:** ❌ **EKSİK** — eklenmeli (Aşama 0.2).

### DR6 — Çok katmanlı kaynak sınırı (decompression bomb savunması)
- **Ne:** Görsel önizlemede risk dosya boyutu değil, **decode edilmiş piksel sayısıdır**. Tek kapı
  yetmez; encoded bayt / boyut / piksel sayısı / decoded bayt / çıktı bayt ayrı ayrı sınırlanır ve
  her biri **tipli hata** döndürür.
- **KULLAN:** Kullanıcı-kontrollü her dosya decode'unda (görsel, ileride PDF raster çıktısı, SVG).
- **KULLANMA:** Kendi ürettiğin, boyutu bilinen ara ürünlerde tekrar kapı koyma — gereksiz.
- Kaynak: `[herdr-image-limits]` · conf 1.0 (referans: `[yazi-preview-config]` `image_alloc`/`image_bound`)
- **herdr'ın üstünlüğü:** 5 katmanlı sınır, yazi'nin 2 katmanlı modelinden titiz. **Yeni belge
  türleri aynı sınır felsefesini miras almalı.**

### DR7 — Bounded worker + generation + stale-reject (mimarinin kalbi)
- **Ne:** Önizleme üretimi UI iş parçacığında **asla** yapılmaz. Her istek bir generation taşır;
  dönen sonuç `accepts(generation, key)` kapısından geçmezse **sessizce atılır** (ekrana yazılmaz,
  cache'e girmez).
- **KULLAN:** Her yeni önizleme türü **bu mevcut worker'a takılır**. XLSX/PDF için paralel yol açma.
- **KULLANMA:** Generation'ı yalnızca iptal için kullanıp sonucu yine de cache'e yazma → cache zehirlenir.
- Kaynak: `[herdr-preview-worker]` `generation.wrapping_add(1).max(1)` + `accepts()` · conf 1.0 ·
  referans `[yazi-scheduler]` (`Scheduler.cancel`) + `[yazi-pdf-plugin]` (`only_if = job.file.url`) · conf 1.0
- **Not:** herdr `FilePreviewKey`'de **çift generation** taşıyor (`files_generation:u32` +
  `preview_generation:u64`) → dizin listesi yenilendiğinde de bayat sonuç reddediliyor. Bu, yazi'nin
  `only_if` sözleşmesinden **tip düzeyinde daha güçlü**.

---

## BÖLÜM 2 — UZANTI VE ZARİF DÜŞÜŞ (DR8)

### DR8 — Opsiyonel sağlayıcı + zarif düşüş (herdr'ın hazır uzantı noktası)
- **Ne:** Bir dosya türü için harici/opsiyonel bir sağlayıcı tanımlanır; sağlayıcı yoksa veya
  platform desteklemiyorsa **tanımlı bir fallback**'e düşülür (metin veya metadata), asla sessiz hata olmaz.
- **herdr sözleşmesi:** `PreviewCapability::OptionalPlugin { action_id, fallback }` +
  `plugin_or_fallback()` → sağlayıcı `platform_supported` ve `action_id` boş değilse plugin,
  değilse `PreviewFallback::NativeText` veya `MetadataOnly(reason)`.
- **KULLAN:** Harici araç gerektiren her tür (PDF→`pdftotext`/`pdftoppm`, XLSX→`xlsx2csv`,
  arşiv→`bsdtar`, medya→`ffmpeg`). **Aşama 0'ın tamamı bu pattern üstünde yükselir.**
- **KULLANMA:** Saf Rust ile ucuza yapılabilen bir şeyi plugin'e itme (görsel decode zaten native).
- Kaynak: `[herdr-preview-capability]` `preview_capability.rs:126-140,181` · conf 1.0 ·
  `[herdr-plugin-contract]` · conf 1.0 · referans `[joshuto-preview-script]`, `[yazi-previewer-cfg]` · conf 1.0
- **Kritik bulgu:** `xlsx/xls/ods/pdf/doc/docx/ppt/pptx` **zaten** bu yola bağlı; sağlayıcı yazılmamış
  olduğu için bugün `MetadataOnly(DocumentMetadata)` gösteriliyor.

---

## BÖLÜM 3 — TABLO/SPREADSHEET (DR9–DR12, DR15)

### DR9 — Seyrek hücre modeli + saf snapshot
- **Ne:** Tablo state'i **saf veri** olarak modellenir: seyrek `BTreeMap<(row,col), CellValue>` +
  ayrı `formulas` haritası + viewport + kolon genişlikleri. PTY/async içermez, testi PTY gerektirmez.
- **KULLAN:** Her tablo türü (XLSX, CSV, TSV) aynı snapshot'a indirgenmelidir — render tek yol.
- **KULLANMA:** Yoğun `Vec<Vec<String>>` matris — boş hücreli gerçek tablolarda bellek israfı;
  ayrıca 1M satırda OOM.
- Kaynak: `[cell-repo]` (*"Data model, formula engine, file I/O (no TUI dependency)"*) · conf 0.9 ·
  `[calamine-api]` `Data` enum + seyrek `Range` · conf 0.9
- **Uyum:** herdr `AGENTS.md` "State is separated from runtime" + "Render is pure" ile **birebir**
  (`docs/patterns/rust-engineering.md` HP1).

### DR10 — Sanal kaydırma (viewport materyalizasyonu)
- **Ne:** Yalnızca ekranda görünen satır/kolon penceresi materyalize edilir; kaynak seyrek yapıdan
  dilimlenir. Toplam boyut ne olursa olsun bellek ve render maliyeti **sabit**.
- **KULLAN:** Satır sayısı bilinmeyen/1000+ olabilecek her tabloda (yani hepsinde).
- **KULLANMA:** Onlarca satırlık sabit küçük tablolarda erken optimizasyon olarak (yine de zararsız).
- Kaynak: `[csvlens-repo]` · conf 0.9 · `[tv-repo]` (*"Automatic large file streaming (>5MB)"*) · conf 0.9 ·
  `[calamine-api]` `Range.start()/end()` · conf 0.9

### DR11 — Kolon genişliği, taşma ve anlamlı basamak
- **Ne:** Genişlik = `min(maks_içerik_genişliği, tavan)`; uzun metin kısaltılır, sayılar anlamlı
  basamağa yuvarlanır ("decimal dust" temizliği); terminal darsa kolon taşma mantığı devreye girer;
  soldan **kolon dondurma** (satır/başlık kolonları sabit kalır).
- **KULLAN:** Her tablo görünümünde; dondurmayı özellikle dar Miller preview kolonunda.
- **KULLANMA:** Sabit genişlikli kolonlar — Unicode/CJK genişliğinde hizalama bozulur.
- Kaynak: `[tv-repo]` (significant digits, column overflow, Unicode truncation) · conf 0.9 ·
  `[csvlens-repo]` (*"freezing columns from the left"*) · conf 0.9
- **herdr avantajı:** `unicode-width 0.2` **zaten bağımlılık** → doğru genişlik hesabı bedava
  (`[herdr-cargo]` · conf 1.0).

### DR12 — Formül: METNİ göster, motoru YAZMA
- **Ne:** Formül çubuğunda `worksheet_formula()` ile **formül metni**, grid hücresinde
  `worksheet_range()` ile **uygulamanın önbelleğe aldığı değer** gösterilir. Yeniden hesaplama yapılmaz.
- **KULLAN:** XLSX/ODS görüntülemenin tamamında. Kullanıcının %95 ihtiyacı "bu hücrede ne yazıyor +
  formülü neydi" sorusudur.
- **KULLANMA:** Kendi formül motorunu yazma — ayrıştırıcı + AST + bağımlılık grafiği + topolojik
  yeniden hesap + döngü tespiti + 60 fonksiyonluk stdlib gerekir; bu **ayrı bir üründür**.
- Kaynak: `[tshts-repo]` (60+ fonksiyon + AST döngü tespiti = maliyet kanıtı) · conf 0.9 ·
  `[calamine-api]` `worksheet_formula` · conf 0.9
- **Yeniden değerlendirme:** Kullanıcı **canlı yeniden hesaplama** isterse ve edit zaten çalışıyorsa;
  o zaman hazır `cell-sheet-core` (MIT, TUI-bağımsız) incelenmeli — sıfırdan yazma yine son çare.

### DR15 — Edit: önce kayıpsız formatta, sonra (belki) zengin formatta
- **Ne:** Düzenleme yeteneği kademeli açılır: (1) kayıpsız/basit format (CSV/TSV) editlenebilir,
  (2) zengin format (XLSX) **salt-okunur** kalır ve bunu UI'da açıkça söyler (`[salt-okunur]` rozeti),
  (3) zengin format yazma ancak veri korunumu **ölçüldükten** sonra açılır.
- **KULLAN:** Her yeni "düzenlenebilir" belge türünde bu üç adım sırayla.
- **KULLANMA:** Zengin formatı doğrudan yazmaya açma — kütüphanenin modellemediği her şey (pivot,
  grafik, koşullu biçimlendirme, makro, veri doğrulama) **sessizce kaybolur**. Kullanıcının Excel
  dosyasını bozmak, "önizleme yok"tan çok daha kötüdür.
- **Widget seçimi:** `edtui` (aktif, 2026-07-18, ratatui-native) — `tui-textarea` **KULLANMA**
  (2024-10-22'den beri durgun, ratatui 0.30 uyumu şüpheli).
- Kaynak: `[umya-api]` (yazma yolu, ⚠️ korunum belgelenmemiş) · conf 0.75 · `[edtui-crate]` ·
  conf 0.95 · `[tui-textarea-crate]` · conf 0.95 · `[cell-repo]` (formüller yalnızca native formatta korunur) · conf 0.9

---

## BÖLÜM 4 — PDF (DR16–DR18)

### DR16 — Metin katmanı önce, raster sonra
- **Ne:** PDF için iki ayrı yol vardır: metin çıkarma (ucuz, aranabilir, saf Rust mümkün) ve
  rasterleştirme (pahalı, harici araç/binary, taranmış PDF için tek çare). **Metin yolu önce gelir.**
- **KULLAN (metin):** Varsayılan; senaryoların ~%80'i. `pdftotext` (harici) veya `lopdf` (metadata).
- **KULLAN (raster):** Düzenin kendisi önemliyse veya metin katmanı yoksa — harici `pdftoppm` ile.
- **KULLANMA:** Native Rust rasterleştirici (`pdfium-render`) — harici pdfium binary/DLL'in 4 platforma
  dağıtımını gerektirir, herdr'ın tek-binary modelini bozar.
- Kaynak: `[yazi-pdf-plugin]` (10k★ FM'in seçimi: harici `pdftoppm`) · conf 1.0 ·
  `[pdfium-render-crate]`, `[lopdf-crate]` · conf 0.95

### DR17 — Tek-sayfa raster: maliyeti sayfa sayısından bağımsız kıl
- **Ne:** Sayfa aralığını daralt (`-f N -l N -singlefile`) → 400 sayfalık PDF'te de maliyet tek sayfalık.
  Sayfa gezinme ayrı bir mekanizma değil, mevcut **kaydırma/skip** kavramının yeniden kullanımıdır.
- **KULLAN:** Her çok-sayfalı belge türünde (PDF, ileride PPTX).
- **KULLANMA:** Belgenin tamamını önden rasterleştirme — kullanıcı ilk sayfayı görmeden bekler.
- Kaynak: `[yazi-pdf-plugin]` (`job.skip` = sayfa, `M:seek` + `ya.clamp(-1, job.units, 1)`) · conf 1.0

### DR18 — Sınırı hatadan öğren + cache ile sıfır tekrar
- **Ne:** Sayfa sayısını öğrenmek için belgeyi önden ayrıştırma; aracın **hata çıktısından** üst sınırı
  yakala (`stderr:match("the last page %((%d+)%)")`) ve üst sınır olarak yayınla. Üretilen çıktı
  içerik-anahtarlı cache'e yazılır; cache varsa iş **hiç başlatılmaz**.
- **KULLAN:** Harici araç kullanan her önizleme yolunda.
- **KULLANMA:** Cache anahtarına generation koymayı unutma — aksi hâlde bayat sonuç cache'e sızar (DR7).
- Kaynak: `[yazi-pdf-plugin]` (`upper_bound=true`, `ya.file_cache(job)` + `fs.cha(cache)`) · conf 1.0

---

## BÖLÜM 5 — UZAK/SSH (DR13–DR14)

### DR13 — Yalnızca taşıyabildiğin protokolü destekle
- **Ne:** Uygulamanın VT katmanı hangi grafik protokolünü **parse edebiliyorsa** yalnızca onu üret.
  Desteklenmeyen protokol için encoder eklemek ölü koddur.
- **KULLAN:** Protokol kararında önce "bizim VT'miz bunu okuyabiliyor mu?" sorusu.
- **KULLANMA:** "İleride lazım olur" diye sixel/iTerm2 encoder ekleme — `vendor/libghostty-vt`'de
  sixel **yok** (`find vendor -ipath "*sixel*"` → boş).
- Kaynak: `[herdr-vendor-kitty]` · conf 0.95 · `[herdr-windows-beta]` (*"Kitty graphics rendering |
  unverified"* — Windows) · conf 1.0
- **Windows notu:** herdr'ın kendi dokümanı Windows'ta Kitty grafiği **doğrulanmamış** sayıyor ve
  `experimental.kitty_graphics = false` bırakılmasını söylüyor → Windows'ta **fallback yolu birinci
  sınıf olmalı**, grafik değil.

### DR14 — Uzak oturumda kademeli düşüş (bant genişliği bütçesi)
- **Ne:** Kitty `t=d` base64 → **%33 şişme**. 1920×1080 RGBA ≈ 8.3 MB → base64 ≈ 11 MB → 10 Mbit/s
  SSH'ta ~9 sn, üstelik pane çıktısıyla aynı kanalı paylaşır. Beş kademeli azaltma uygulanır:
  1. **Hedef alana downscale — iletmeden ÖNCE** (10–100×)
  2. **Debounce** (DR5) — gezinmede %90+ iptal
  3. **Kayıplı format** (JPEG, kalite ~75) uzakta (3–10×)
  4. **İçerik-hash cache + `d=i`** (DR2) — tekrar ziyarette %100
  5. **Uzakta grafiği tamamen kapat → metin/metadata fallback** (DR8)
- **KULLAN:** Uzak oturum tespit edildiğinde otomatik; kullanıcı ayarıyla geçersiz kılınabilir.
- **KULLANMA:** Lokal oturumda kalite düşürme — gereksiz görsel kayıp.
- Kaynak: `[kitty-graphics-spec]` (base64 iletim) · conf 0.95 · `[yazi-preview-config]`
  (`image_quality`, `image_delay`) · conf 1.0 · `[herdr-preview-capability]` (`PreviewFallback` hazır) · conf 1.0
- **herdr durumu:** Mekanizma (`PreviewFallback`) **hazır**; eksik olan **uzak oturum tespiti +
  politika**. Yeni bağımlılık gerektirmez.

---

## ANTİ-PATTERN'LER (DA1–DA12)

> **herdr kolonu:** ✅ = bu tuzağa karşı korunma **kodda doğrulandı** · ❌ = korunma **yok** (açık risk) ·
> ➖ = ilgili katman henüz yok (tablo/edit gelmeden anlamsız) · ❓ = kodda doğrulanmadı.

| # | Anti-pattern | Neden felaket | Doğrusu | herdr | Kaynak · conf |
|---|---|---|---|---|---|
| **DA1** | Görseli tam çözünürlükte iletmek | SSH'ta 11 MB base64 ≈ 9 sn donma | Hedef `Rect`×`HostCellSize` downscale, **sonra** ilet (DR14) | ❓ `HostCellSize` hesabı var; downscale-önce-ilet akışı doğrulanmadı | `[kitty-graphics-spec]` · 0.95 |
| **DA2** | Senkron decode/harici süreçle UI'yı bloklamak | Bozuk/dev PDF = donmuş TUI | Bounded worker + generation (DR7) | ✅ **çözülmüş** — `file_preview_worker` + `image_preview_worker` | `[herdr-preview-worker]` · 1.0 |
| **DA3** | Debounce'suz önizleme | Ok tuşu basılı = onlarca iletim kuyruğu | `image_delay` ~30 ms (DR5) | ❌ **EKSİK** — Aşama 0.2 | `[yazi-preview-config]` · 1.0 |
| **DA4** | Küçük harf `d=i`'yi bellek temizliği sanmak | Terminal tarafında görsel verisi birikir | Kalıcı temizlikte `d=I` (DR2) | ❓ hangi varyantın kullanıldığı doğrulanmadı | `[kitty-graphics-spec]` · 0.95 |
| **DA5** | Mod/görünüm geçişinde görseli silmemek | Hayalet görseller metnin üstünde kalır | View-key değişiminde açık `a=d` | ✅ **çözülmüş** — `HostViewKey{workspace_index, tab_index, file_manager_open}` | `[herdr-kitty-graphics]` · 1.0 |
| **DA6** | Görselin metinle birlikte kayacağını varsaymak/varsaymamak | Görsel yerinde kalır, metin akar | Spec: *"images must be scrolled along with text"* → sanal yerleştirme veya her frame yeniden yerleştir (DR3) | ❓ Miller kolonu kaymadığı için bugün tetiklenmiyor | `[kitty-graphics-spec]` · 0.95 |
| **DA7** | Decode limiti koymamak | 20000×20000 PNG ≈ 1.6 GB → OOM | Çok katmanlı sınır (DR6) | ✅ **çözülmüş — ekosistemin üstünde** (5 kapı) | `[herdr-image-limits]` · 1.0 |
| **DA8** | Tüm tablo satırlarını belleğe açmak | 1M satır = OOM | Viewport sanal kaydırma (DR10) | ➖ tablo katmanı yok (Aşama 1) | `[tv-repo]`, `[csvlens-repo]` · 0.9 |
| **DA9** | Formül motoru yazmaya girişmek | Ayrıştırıcı+AST+bağımlılık+döngü+60 fn = ayrı ürün | Formül metni + önbellek değeri (DR12) | ➖ tablo katmanı yok — **karar peşinen verildi: yazılmayacak** | `[tshts-repo]` · 0.9 |
| **DA10** | Zengin formatı (XLSX) doğrudan yazmaya açmak | Pivot/grafik/makro/koşullu biçim sessizce kaybolur | Kademeli edit: CSV önce, XLSX ölçülene kadar salt-okunur (DR15) | ➖ edit katmanı yok (Aşama 2) | `[umya-api]` · 0.75 |
| **DA11** | VT'nin parse edemediği protokolü üretmek | Ölü kod + yanlış beklenti | Yalnızca taşınabilir protokol (DR13) | ✅ **uyuyor** — yalnız Kitty üretiliyor, sixel encoder yok | `[herdr-vendor-kitty]` · 0.95 |
| **DA12** | Harici aracın varlığını varsaymak | `pdftoppm` yoksa Windows'ta sessiz hata | `platform_supported` + `PreviewFallback` (DR8) | ✅ **çözülmüş** — `plugin_or_fallback()` | `[herdr-preview-capability]` · 1.0 |

**Karne:** 5 ✅ çözülmüş · 1 ❌ açık risk (DA3 debounce) · 3 ➖ katman yok · 3 ❓ doğrulanmadı.
❓'ler bir sonraki turun **doğrulama borcu**dur (`docs/references/document-rendering.md` §Doğrulama borcu).

---

## ÖLÇEK / KARAR MATRİSİ

> "Hangi durumda hangi pattern?" — **beş eksende** karar: dosya boyutu · lokal↔uzak · terminal
> yeteneği · edit ihtiyacı · **platform**. Eksenler bağımsız değil; çakışmada **en kısıtlayıcı olan
> kazanır** (ör. Windows + uzak + grafik-doğrulanmamış → fallback, tartışma yok).

### Eksen 1 — Dosya boyutu / karmaşıklık

| Durum | Seçilecek pattern | Neden |
|---|---|---|
| Küçük görsel (< birkaç MB, ekran boyutunda) | DR2 + DR4 doğrudan iletim | Downscale sonrası maliyet zaten düşük |
| Büyük/dev görsel (yüksek çözünürlük) | **DR6 sınır kapıları** + DR14/1 downscale | Decode bombası riski birincil |
| Küçük tablo (< ~1000 satır) | DR9 seyrek model | Sanal kaydırma yine de zararsız |
| Büyük tablo (1000+ / bilinmeyen) | **DR10 sanal kaydırma** + DR11 | Materyalizasyon = OOM riski |
| Çok sayfalı belge (PDF) | **DR17 tek-sayfa** + DR18 cache | Sayfa sayısından bağımsız maliyet |
| Taranmış PDF (metin katmanı yok) | DR16 raster yolu (harici) | Metin çıkarma sonuç vermez |

### Eksen 2 — Uzak mı, lokal mi

| Durum | Seçilecek pattern | Neden |
|---|---|---|
| Lokal oturum, grafik destekli | DR2/DR3/DR4 tam görsel | Bant genişliği sorun değil |
| Uzak (SSH), grafik destekli | **DR14 kademe 1-4** (downscale + debounce + JPEG + cache) | %33 base64 şişmesi + paylaşılan kanal |
| Uzak, düşük bant genişliği | **DR14 kademe 5** → DR8 metin/metadata fallback | Görsel maliyeti kullanıcı deneyimini bozar |
| tmux/multiplexer altında | DR1 (tmux ekseni ayrı sondalanır) | Sarmalama escape'leri değiştirir |

### Eksen 3 — Terminal yeteneği

| Durum | Seçilecek pattern | Neden |
|---|---|---|
| Kitty grafik destekli, doğrulanmış | DR2/DR3 native görsel | herdr'ın taşıyabildiği tek protokol (DR13) |
| Yetenek bilinmiyor | **DR1 timeout'lu sondaj** → sonuca göre | Sessiz bozuk çıktıdan kaçın |
| Grafik desteksiz / Windows (unverified) | **DR8 fallback** (metin/metadata) birinci sınıf | herdr dokümanı Windows'ta grafiği doğrulanmamış sayıyor |
| Sixel-only terminal | ⚠️ Bugün desteklenmiyor (DR13) | VT sixel parse etmiyor; fallback'e düş |

### Eksen 4 — Edit ihtiyacı

| Durum | Seçilecek pattern | Neden |
|---|---|---|
| Sadece görüntüleme | DR9–DR12 (+ `[salt-okunur]` rozeti) | csvlens kanıtı: editsiz de çok kullanışlı |
| Basit/kayıpsız format editi (CSV/TSV) | **DR15 adım 1** + `edtui` | Veri kaybı riski yok |
| Zengin format editi (XLSX) | **DR15 adım 3** — POC'suz AÇMA | Pivot/grafik/makro kaybı riski (DA10) |
| Canlı formül yeniden hesaplama | ❌ **DR12 KULLANMA** — ayrı ürün | 60+ fn + AST + bağımlılık grafiği maliyeti |

### Eksen 5 — Platform (Linux / macOS / Windows)

> herdr üç platform destekliyor; Windows **beta** ve grafik yolu kendi dokümanında *"unverified"*.
> Bu eksen diğer dördünü **geçersiz kılabilir** — platform desteklemiyorsa diğer optimizasyonlar konusuz.

| Durum | Seçilecek pattern / teknoloji | Neden |
|---|---|---|
| **Linux / macOS**, Kitty destekli host | DR2/DR3/DR4 native görsel + DR6 sınırlar | Tam yol açık; grafik birinci sınıf |
| **Windows** (beta) | **DR8 fallback birinci sınıf** + DR13 · grafiği varsayma | `[herdr-windows-beta]`: *"Kitty graphics rendering \| unverified"*, `experimental.kitty_graphics=false` bırakılması öneriliyor |
| Herhangi bir platform, **saf Rust crate** (`calamine`, `lopdf`) | ✅ Tercih et | Tek-binary dağıtım korunur, üç platformda aynı davranış |
| Herhangi bir platform, **harici araç** (`pdftoppm`, `xlsx2csv`) | DR8 + manifest `platforms = [...]` filtresi · DA12 | Araç Windows'ta yoksa **sessiz hata yerine** tanımlı fallback |
| Herhangi bir platform, **harici binary/DLL** (`pdfium`) | ❌ **Kaçın** | 4 platforma binary shipping; tek-binary modelini bozar (§L reddedilenler) |
| Platform-özel bağımlılık ailesi çakışması | ❌ **Kaçın** | Örn. `ratatui-image` → `windows 0.58` vs herdr `windows-sys 0.61.2` = iki binding ailesi |
| Windows'ta edit yolu | Aşama 2'de **en son** | Beta platformda veri-kaybı riskli özellik açma (DA10) |

**Platform karar kuralı:** Bir özellik üç platformda **aynı** davranamıyorsa, ya (a) saf-Rust
alternatifi seçilir, ya (b) `platform_supported` ile açıkça sınırlanır + fallback tanımlanır.
"Linux'ta çalışıyor, Windows'a sonra bakarız" **DA12'nin ta kendisidir**.

### Karar refleksi (yeni belge türü eklerken — 6 soru)
```
1. VT'miz bu çıktıyı taşıyabiliyor mu?          → hayır: DR13, fallback tasarla
2. Native mi, harici araç mı?                    → harici: DR8 OptionalPlugin + platform filtresi
3. Kaynak sınırları ne?                          → DR6 kapıları (decode/piksel/çıktı)
4. Mevcut worker'a takılıyor mu?                 → DR7 generation + stale-reject ZORUNLU
5. Uzakta ne olacak?                             → DR14 kademeli düşüş politikası
6. Üç platformda da aynı mı davranıyor?          → hayır: saf-Rust alternatif VEYA
                                                    platform_supported + tanımlı fallback (Eksen 5)
```

---

## herdr'ın MEVCUT DURUMU — pattern uyum karnesi

| Pattern | herdr durumu | Kanıt |
|---|---|---|
| DR1 yetenek sondajı | ❌ **YOK** (fallback var, sondaj yok) | `[herdr-kitty-graphics]` `HostCellSize::from_terminal` |
| DR2 yerleştirme/temizleme | ✅ **VAR** (id-uzayı izolasyonlu) | `HOST_IMAGE_ID_BASE`, `HostViewKey` |
| DR3 yerleştirme modeli | ✅ **VAR** (mutlak — Miller için doğru) | `KittyPlacementRenderInfo` |
| DR4 chunking | ✅ **VAR ve protokole uygun** | `KITTY_CHUNK_BYTES = 3072` |
| DR5 debounce | ❌ **EKSİK** | — (Aşama 0.2) |
| DR6 kaynak sınırı | ✅ **VAR — ekosistemin üstünde** (5 katman) | `image_preview.rs:11-15` |
| DR7 worker/stale-reject | ✅ **VAR — ekosistemle eşdeğer+** (çift generation) | `file_preview_worker.rs:75-88` |
| DR8 opsiyonel sağlayıcı | ✅ **MEKANİZMA VAR, SAĞLAYICI YOK** | `preview_capability.rs:126-140` |
| DR9–DR12 tablo | ❌ **YOK** (CSV metin olarak düşüyor) | `text_preview.rs` |
| DR13 protokol kısıtı | ✅ **UYUYOR** (yalnız Kitty) | `vendor/libghostty-vt` |
| DR14 uzak düşüş | 🟡 **MEKANİZMA VAR, POLİTİKA YOK** | `PreviewFallback` |
| DR15 edit | ❌ **YOK** | — (Aşama 2) |
| DR16–DR18 PDF | ❌ **YOK** | `PreviewReason::DocumentMetadata` |

**Özet:** Grafik altyapısı ve worker disiplini ekosistem seviyesinde veya üstünde. Eksik olan
**içerik sağlayıcılar** (DR8'in doldurulması), **debounce** (DR5), **uzak politika** (DR14) ve
**tablo katmanı** (DR9–DR12).

---
*v1.0.0 — 2026-07-24 · reference-registry 5-adım pipeline Adım-3 artefaktı.*
*Tam gerekçe: `docs/analysis/2026-07-24-document-render-ecosystem.md` ·*
*Kaynak sözlüğü: `docs/references/document-rendering.md`*
