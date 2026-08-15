---
doc: herdr-analysis
domain: document-rendering
subject: Önizleme yığınının katman-katman anatomisi + yazi/ratatui-image/yazi-driver kıyası + PNG·JPEG·XLSX·PDF planı
created: 2026-07-25
method: doğrudan kaynak okuma (grep + sed, satır-numaralı). Cargo ÇALIŞTIRILMADI.
status: canonical — L0..L5 katman modeli bu dosyada tanımlanır; sonraki analizler buna atıf yapar
git_note: /docs/* gitignored → lokal. Makine kopyası ~/.cartography/herdr-analysis/
agentic_triggers:
  - "png önizleme · jpeg · görsel önizleme · image preview · kitty graphics"
  - "xlsx · spreadsheet · tablo önizleme · calamine"
  - "pdf önizleme · pdf render"
  - "server modunda görsel çıkmıyor · headless preview · scheduler drift"
  - "yazi karşılaştırma · driver · sixel · halfblock fallback"
related:
  - docs/analysis/2026-07-25-preview-performance-and-signals.md
  - docs/analysis/2026-07-25-preview-provider-source.md
  - docs/analysis/2026-07-24-document-render-internal-state.md
  - docs/analysis/2026-07-24-document-render-ecosystem.md
  - docs/patterns/document-rendering.md
---

# Önizleme Yığını — Katman Anatomisi ve Uygulama Planı (2026-07-25)

> **Bu dosya bir önceki turun en önemli iddiasını DÜZELTİR.**
> `2026-07-25-preview-performance-and-signals.md` "PNG önizlemesi server modunda **mimari olarak**
> erişilemez" dedi. Bu **yanlış tanı**. Doğrusu aşağıda: taşıma katmanı ZATEN kurulu; eksik olan
> **zamanlayıcı (scheduler) bağlaması** — ölçülen fark **iki çağrı + bir görünürlük niteleyici**.

---

## 0. Katman modeli (L0–L5) — bundan sonraki tüm tartışmanın ortak dili

Önizleme tek bir "özellik" değil, **altı bağımsız katman**. Her dosya tipi bu katmanların
**farklı bir alt kümesini** kullanır. Bugüne kadarki kafa karışıklığının kaynağı budur:
XLSX'i "görsel önizleme" sanmak, onu gereksiz yere L0–L3'e sokar.

| # | Katman | Sorumluluk | herdr'daki yeri |
|---|---|---|---|
| **L0** | **Terminal görsel protokolü** | Piksel'i terminale nasıl söyleriz | `src/kitty_graphics.rs` — **yalnız Kitty** |
| **L1** | **Piksel üretimi (decode+resize)** | Dosya baytı → RGBA tampon | `src/app/image_preview_worker.rs` (`image` crate, sınırlı worker) |
| **L2** | **Yerleşim/geometri** | Hangi hücreye, kaç px, hangi kırpma | `kitty_graphics.rs:108` `file_manager_image_target`, `HostPlacement` |
| **L3** | **Taşıma (server → client)** | Baytlar istemci terminaline nasıl varır | `FrameData.graphics` + `render_stream.rs` splice |
| **L4** | **Zamanlama (scheduler)** | İş ne zaman tetiklenir, sonuç ne zaman toplanır | `app/runtime.rs:198` **VS** `server/headless.rs:3611` |
| **L5** | **İçerik dönüşümü** | Dosya → *anlamlı ara temsil* (metin/ızgara/piksel) | Bugün yalnız metin (`file_preview_worker.rs`) ve görsel (L1) |

### Dosya tipi → katman haritası (KRİTİK)

| Dosya | L0 | L1 | L2 | L3 | L4 | L5 |
|---|---|---|---|---|---|---|
| **PNG / JPEG** | ✅ gerekir | ✅ | ✅ | ✅ | ✅ | — |
| **XLSX** | ❌ **gerekmez** | ❌ | ❌ | ❌ | ✅ | ✅ *(sheet→ızgara)* |
| **PDF (metin modu)** | ❌ | ❌ | ❌ | ❌ | ✅ | ✅ *(sayfa→metin)* |
| **PDF (görsel modu)** | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ *(sayfa→raster)* |

⇒ **XLSX önizlemesi grafik yığınına hiç dokunmaz.** Metin önizleme yolunun kardeşidir.
Bu, XLSX'i PNG'den **bağımsız ve daha ucuz** bir iş yapar. İkisini aynı kovaya koymak hatadır.

---

## 1. 🔴 Gerçek kısıt: L4 scheduler ikizlenmesi (mimari değil, **drift**)

herdr'ın iki çalışma modu var ve **iki ayrı zamanlayıcı** yazılmış:

```rust
// src/app/runtime.rs:198  — MONOLITHIC mod (herdr'ı doğrudan çalıştırınca)
pub(crate) fn handle_scheduled_tasks(&mut self, now: Instant, geometry_dirty: bool) -> bool {
    changed |= self.sync_file_operation_worker();
    changed |= self.sync_file_manager_agent_handoff();
    changed |= self.sync_file_manager_agent_handoff_send();
    changed |= self.sync_agent_attachment_delivery();
    changed |= self.sync_agent_reference_picker();
    changed |= self.sync_file_manager_plugin_action();
    changed |= self.sync_file_manager_io_results();
    changed |= self.sync_file_manager_location_request();
    changed |= self.sync_file_manager_watcher_at(now);
    changed |= self.sync_file_preview_worker();
    changed |= self.sync_image_preview_worker();      // ← 11 çağrı
    ...
}

// src/server/headless.rs:3611  — SERVER modu (normal kullanım!)
fn handle_scheduled_tasks_headless(&mut self, now: Instant, geometry_dirty: bool) -> bool {
    changed |= self.app.sync_file_manager_io_results();
    changed |= self.app.sync_file_manager_location_request();
    changed |= self.app.sync_file_preview_worker();
    // ← yalnız 3 çağrı. sync_image_preview_worker DAHİL 8 tanesi YOK.
    ...
}
```

**Server modunda eksik olan 8 çağrı:**
`sync_file_operation_worker` · `sync_file_manager_agent_handoff` · `..._send` ·
`sync_agent_attachment_delivery` · `sync_agent_reference_picker` · `sync_file_manager_plugin_action` ·
`sync_file_manager_watcher_at` · **`sync_image_preview_worker`**

Bu bir **tasarım kararı değil, kopyalanmış-ve-geride-kalmış zamanlayıcı**. Yorum satırı bunu
itiraf ediyor (`headless.rs:3607`): *"Similar to `App::handle_scheduled_tasks` but without resize
polling"* — yani niyet **eşdeğerlik**, gerçekleşen **ıraksama**. conf **0.97** (iki fonksiyon
yan yana okundu).

### İkinci eksik: hücre boyutu App'e hiç yazılmıyor

```rust
// src/app/mod.rs:1203 — YALNIZ monolithic render döngüsünde
self.image_preview_cell_size = cell_size;
```
`grep -rn "image_preview_cell_size" src` → üretim kodunda **tek atama** bu. Server'da yok.
Worker hedefi bundan türetiyor (`image_preview_worker.rs:280`), dolayısıyla server'da
`HostCellSize::default()` (= bilinmeyen) kalır → `file_manager_image_target` `None` döner →
iş hiç kuyruğa girmez. conf **0.97**.

### Üçüncü (küçük) eksik: görünürlük

`pub(super) fn sync_image_preview_worker` (`image_preview_worker.rs:268`) — `super` = `crate::app`.
`crate::server` göremez. Kardeşi `sync_file_preview_worker` zaten `pub(crate)`
(`file_preview_worker.rs:382`) ve server onu çağırabiliyor (`headless.rs:3616`). **Asimetri
kasıtlı değil**; biri ihtiyaç doğunca yükseltilmiş, diğeri unutulmuş. conf 0.95.

---

## 2. ✅ Zaten çalışan: L0–L3 tam kurulu (yaygın yanlış inanışın aksine)

Bir önceki tur "server modunda görsel imkânsız" derken **taşımanın kurulu olduğunu görmemişti.**
Kanıt zinciri:

| Katman | Server tarafı kanıtı |
|---|---|
| **L3 kodlama** | `headless.rs:3449` → `frame.graphics.extend(kitty_graphics::encode_local_pane_graphics(...))` |
| **L3 önbellek** | `client.graphics_cache` + `graphics_surface_reset_pending` (`headless.rs:3440-3457`) — istemci başına diff'li |
| **L3 sınır** | `MAX_GRAPHICS_FRAME_SIZE` taşma koruması (`headless.rs:3465`) |
| **L2 geometri** | `compute_view_with_cell_size(..., client.cell_size)` (`headless.rs:814`, `:3366`) |
| **L0/L2 girdi** | istemci `cell_width_px`/`cell_height_px`'i **tel üzerinden bildiriyor** (`client_transport.rs:289,338,461,710`) → `client.cell_size` (`clients.rs:34`) |
| **L1 çekirdek** | `encode_local_pane_graphics` FM önizlemesini de kodluyor: `kitty_graphics.rs:237` `FmImagePreviewState::Ready { target, prepared }` okuma noktası |

⇒ **Boru hattının %90'ı server modunda döşenmiş.** Eksik olan, boruya suyu veren musluk (L4).

---

## 3. Kıyas — herdr · yazi · ratatui-image (katman katman)

| Katman | **yazi** | **ratatui-image** | **herdr (bugün)** | herdr (hedef) |
|---|---|---|---|---|
| **L0 protokol** | **7 sürücü**: `kgp`, `kgp_old`, `iip`, `sixel`, `chafa`, `ueberzug`, halfblock | kitty · iTerm2 · sixel · halfblock | **1**: yalnız kitty | +halfblock fallback (sıfır bağımlılık) |
| **L1 decode** | `image` + `fast_image_resize` | `image` | `image`, **sınırlı worker + panic bariyeri** | değişmez |
| **L2 geometri** | driver içi | widget içi | **ayrı saf modül** (`file_manager_image_target`) — en temizi | değişmez |
| **L3 taşıma** | **yok** (tek process, `w.write_all` → stdout, `drivers/kgp.rs:328`) | **yok** (aynı process) | **`FrameData.graphics` + splice** — üçünde tek olan | değişmez |
| **L4 zamanlama** | senkron/blocking preview task | çizim anında | **iki zamanlayıcı, biri geride** 🔴 | tek kaynak |
| **L5 dönüşüm** | **plugin (Lua) + harici komut** (`xlsx`→`pandoc`/`xlsx2csv` vb.) | yok | metin + görsel | +sheet, +pdf |

### Bundan çıkan üç ders

1. **herdr L3'te yazi'den ÜSTÜN.** yazi tek-process olduğu için uzaktan/attach session'da
   görsel gösteremez; herdr'ın protokol yan-kanalı bunu yapabilir. Bu **fork'un ayırt edici
   değeri** — atılacak değil, tamamlanacak bir yatırım.
2. **herdr L0'da geride.** Tek sürücü = Kitty dışı terminalde (tmux, Konsole, xterm) **sessiz
   hiçlik**. yazi'nin halfblock/chafa fallback'i her yerde bir şey gösterir. Bu bizim en büyük
   gerçek boşluğumuz — ama **L4'ten sonra** gelir.
3. **yazi L5'i dışarı atmış** (plugin + harici binary). Bu bize XLSX/PDF için **iki seçenek**
   sunuyor: dışarı at (plugin) veya içeri al (saf-Rust crate). Bkz. §5.

---

## 4. FAZ 1 — PNG/JPEG'i server modunda çalıştır (en yüksek getiri/maliyet)

### Değişiklik yüzeyi (üç nokta)

| # | Dosya | Değişiklik |
|---|---|---|
| 1 | `src/app/image_preview_worker.rs:268` | `pub(super)` → `pub(crate)` (kardeşiyle simetri) |
| 2 | `src/server/headless.rs` render yolu | ön-plan istemcinin `cell_size`'ını `self.app.image_preview_cell_size`'a yaz |
| 3 | `src/server/headless.rs:3611` | `changed \|= self.app.sync_image_preview_worker();` |

### ⚠️ Tek gerçek tasarım sorusu: **çok istemcili hücre boyutu**

`App` **tek** `image_preview_cell_size` tutar; server'da **N istemci, N farklı `cell_size`**
olabilir (`clients.rs:34`). Kimin boyutu kazanır?

| Seçenek | Not |
|---|---|
| **(a) Ön-plan istemcisi** ✅ önerilen | `foreground_client_id` zaten var (`headless.rs:3453`). Diğer istemciler o kareyi alır — pane grafiklerinde **bugün de böyle**. Yeni asimetri getirmez. |
| (b) İstemci başına worker | N× bellek/CPU, sınırlı-worker desenini kırar. Talep gelmeden yapılmaz. |
| (c) En küçük ortak boyut | Ön-planda bulanık görüntü. Kötü. |

Karar: **(a)** — mevcut `encode_local_pane_graphics` davranışıyla tutarlı.

### Test noktaları — **koddan ÖNCE yazılacak** (beklenen sonuç + sebep)

| ID | Test | Beklenen | Neden bu test |
|---|---|---|---|
| **F1-R1** | `headless_scheduler_syncs_image_preview_worker` | RED: server zamanlayıcısı çağırmıyor | Kök nedeni kilitler; regresyonda ilk düşen bu olur |
| **F1-R2** | `headless_render_publishes_foreground_cell_size_to_app` | RED: `image_preview_cell_size` `default()` kalıyor | İkinci sessiz engel; F1-R1 tek başına yeşil olup **hâlâ görüntü çıkmayabilir** |
| **F1-R3** | `server_frame_carries_fm_image_graphics_when_ready` | `frame.graphics` boş değil + `\x1b_G` içeriyor | Uçtan uca kanıt — L1→L3 zincirinin tamamı |
| **F1-R4** | `unknown_cell_size_client_gets_no_graphics_and_no_panic` | boş `graphics`, panik yok | Fail-safe: `is_known()` false yolu |
| **F1-R5** | `scheduler_parity_headless_vs_monolithic` | İki zamanlayıcının çağrı kümesi **kasıtlı fark listesi** dışında eşit | 🎯 **Asıl kalıcı koruma** — drift'in TEKRARINI engeller |
| **F1-R6** | insan doğrulaması (izole dev test) | Gerçek terminalde PNG görünüyor | Otomatik test terminal göstermez |

**F1-R5 en değerli test.** Diğerleri bu hatayı düzeltir; F1-R5 **bu hata sınıfını** düzeltir.
Diğer 7 eksik çağrı (dosya işlemleri, watcher, plugin action, agent handoff) da aynı drift'in
kurbanı — F1-R5 hepsini görünür kılar.

⚠️ **F1-R5 muhtemelen daha fazla kırık ortaya çıkarır.** Bu iyi haber, kapsam patlaması değil:
bulguları listele, PNG dışındakileri ayrı iş kalemi yap.

### Sıralama kısıtı

`server/headless.rs` **upstream çakışma listesinde** (19 dosyadan biri, ayrıca upstream orada
1137 satır değiştirmiş). `kitty_graphics.rs` de öyle (upstream 462 satır + `36de78dd` "preserve
kitty graphics during host repaints"). ⇒ **Önce merge, sonra bu faz.** Aksi hâlde aynı çakışmayı
iki kez çözeriz. (`upstream-merge-recon.md` ile aynı sonuç, bağımsız gerekçeyle.)

---

## 5. FAZ 2 — XLSX önizlemesi (grafik yığınına **hiç girmez**)

XLSX bir görsel değil, **ızgara**. Doğru hedef: `TextPreview`'in kardeşi bir `SheetPreview`.

### Boru hattı

```
dosya → [sınırlı worker] calamine::open_workbook_auto → Range<Data>
      → sınırlı ızgara (ilk N satır × M sütun, hücre başına maks uzunluk)
      → FmFilePreview::Sheet(SheetPreview)  ← YENİ varyant
      → TrailDetailPreview::Sheet           ← YENİ varyant
      → ratatui Table/özel ızgara çizimi     ← saf render
```

### Neden `calamine`

| Ölçüt | Değer |
|---|---|
| Lisans | MIT — AGPL fork'la uyumlu |
| Saf Rust | ✅ harici binary/sistem kütüphanesi **yok** (`pdfium`in aksine) |
| Kapsam | xls · xlsx · xlsb · ods |
| Model | `Range<Data>` — satır/sütun aralığı, tam da ızgara önizlemesinin istediği |
| API şekli | Senkron+bloklayıcı → **sınırlı worker deseni zaten var**, birebir uyar |

### Mevcut desenlerle uyum (yeni mimari YOK)

`file_preview_worker.rs` şablonu **doğrudan** kopyalanabilir: tek yuva · en-son-kazanır ·
`generation` + `accepts()` ile bayat sonuç reddi · `catch_unwind` panik bariyeri · `PendingText`
ile aynı "beklemede" durumu. `FmFilePreview::PendingText`'in ikizi `PendingSheet`.

### Sınırlar (fail-safe — büyük dosya savunması)

| Sınır | Öneri | Sebep |
|---|---|---|
| Dosya boyutu | üst sınır, aşan → `MetadataOnly` | 200 MB xlsx bellek patlatır |
| Satır × sütun | ör. 500 × 64 | Önizleme paneli zaten daha azını gösterir |
| Hücre metni | ör. 256 karakter | Tek hücre paneli boğmasın |
| Sayfa | ilk sayfa (sonra sekme) | Kapsamı küçük tut |

### Test noktaları

| ID | Beklenen |
|---|---|
| F2-R1 | `.xlsx` seçimi `PendingSheet` kurar (bugün `MetadataOnly`) — RED |
| F2-R2 | Sınırlı ızgara: 10.000 satırlık dosya sınırda kesilir, **kesildiği görünür** (sessiz kırpma yok) |
| F2-R3 | Bozuk/şifreli xlsx → `Unavailable(sebep)`, panik yok |
| F2-R4 | Formül hücresi: **önbellek değeri** gösterilir; yoksa açık işaret |
| F2-R5 | Birleşik hücre / boş satır ızgarayı kaydırmaz |
| F2-R6 | Bayat sonuç reddi (`generation`) — hızlı gezinmede yanlış dosya içeriği ekrana **düşmez** |

**F2-R6 sessiz-hata avcısı:** en tehlikeli bug "A.xlsx seçiliyken B.xlsx içeriği görünmesi"dir.

---

## 6. FAZ 3 — PDF (iki ayrı yol, ayrı ayrı kararlaştırılır)

| | **3a — metin modu** | **3b — görsel modu** |
|---|---|---|
| Çıktı | Sayfa metni | Sayfa rasteri |
| Katman | L5 + L4 | L0–L5 tamamı |
| Bağımlılık | saf Rust (`pdf-extract` vb.) | `pdfium-render` (**harici binary**) veya `mupdf` (**AGPL** — fork'la uyumlu, upstream'e değil) |
| Faz 1'e bağımlı | ❌ | ✅ **kesinlikle** |
| Risk | Düşük | Dağıtım/lisans riski |
| Tarama-edilmiş PDF | ❌ boş çıkar | ✅ tek işe yarayan yol |

**Öneri:** 3a'yı Faz 2 ile birlikte yap (aynı metin boru hattı, bedava sayılır). 3b'yi
Faz 1 kanıtlandıktan **sonra** ayrı karar olarak aç. `pdfium-render` daha önce
**harici binary gerekçesiyle reddedildi** (`document-render-ecosystem.md`) — yeniden açılma
koşulu: L0 fallback'i de çözülmüş ve dağıtım stratejisi netleşmiş olması.

---

## 7. FAZ 4 — L0 fallback (Kitty olmayan terminaller)

Bugün Kitty dışı terminalde görsel önizleme **sessizce hiçbir şey göstermiyor** —
`kitty_graphics::is_enabled()` false ise dal hiç çalışmıyor.

| Seçenek | Bağımlılık | Kalite | Not |
|---|---|---|---|
| **halfblock** (`▀` + fg/bg renk) | **sıfır** | Düşük ama **her yerde** | Zaten elimizdeki RGBA'dan üretilir; L1 değişmez |
| sixel | orta | Orta-yüksek | Konsole/foot/xterm |
| chafa / ueberzug | **harici binary** | — | yazi böyle yapıyor; bizim dağıtım modelimize uymaz |

**Öneri: yalnız halfblock.** Sıfır bağımlılık, saf fonksiyon (RGBA → hücre ızgarası), test edilebilir,
`FrameData` dışında hiçbir şeye dokunmaz (normal hücre içeriği olarak gider — **L3'e bile ihtiyaç
duymaz**). "Hiçbir şey" yerine "kaba ama tanınır" göstermek doğru fail-safe.

---

## 8. Öncelik sırası (gerekçeli)

| Sıra | İş | Neden burada |
|---|---|---|
| **0** | Upstream merge | `headless.rs` + `kitty_graphics.rs` çakışma listesinde; sonraya bırakmak işi iki kez yaptırır |
| **1** | **Faz 1 (PNG server)** | En yüksek getiri/maliyet oranı; zaten yazılmış %90'ı canlıya alır |
| **1.5** | **F1-R5 parite testi** | Drift sınıfını kapatır; kalan 7 eksik çağrıyı görünür yapar |
| **2** | **Faz 2 (XLSX)** | Bağımsız (grafik yığınına dokunmaz) → Faz 1 ile **paralel** gidebilir |
| **2.5** | Faz 3a (PDF metin) | Faz 2'nin boru hattını yeniden kullanır |
| **3** | Faz 4 (halfblock) | Kitty dışı kullanıcılara ilk kez bir şey gösterir |
| **4** | Faz 3b (PDF görsel) | Harici bağımlılık kararı; en son |

**Paralellik notu:** Faz 1 (L0–L4) ile Faz 2 (L4–L5) yalnız L4'te kesişir. Farklı dosyalarda
çalışırlar → aynı anda ilerletilebilir.

---

## 9. Bu turda İNCELENMEYEN

- Kalan **7 eksik scheduler çağrısının** kullanıcıya yansıyan etkisi (dosya işlemleri, watcher,
  plugin action server modunda da mı kırık?) → F1-R5 bunu ölçecek
- `resize_preview_active` erken-çıkışının (`image_preview_worker.rs:270`) server modunda anlamı
- Grafik önbelleğinin (`graphics_cache`) istemci kopukluğunda temizlenme davranışı
- Animasyonlu GIF/WebP — `image` crate destekler, önizleme politikası tanımsız
- `umya-spreadsheet` (XLSX **yazma**) — bu tur yalnız okuma kapsamda
- Upstream 125 commit'in bu katmanlara etkisi (özellikle `36de78dd` kitty graphics repaint)

## 10. Reddedilen / ertelenen + yeniden-açılma koşulları

| Karar | Gerekçe | Yeniden açılır eğer |
|---|---|---|
| İstemci başına görsel worker | N× kaynak; sınırlı-worker desenini kırar | Farklı `cell_size`'lı çok-istemci kullanımı gerçek şikâyet olursa |
| chafa/ueberzug sürücüsü | Harici binary — dağıtım modelimize aykırı | Tek dosya dağıtımından vazgeçilirse |
| `pdfium-render` | Harici binary + dağıtım riski | Faz 4 biter **ve** dağıtım stratejisi netleşirse |
| sixel sürücüsü | halfblock daha ucuz ve daha yaygın | halfblock kalitesi somut şikâyet alırsa |
| XLSX'i grafik olarak render | Yanlış katman; ızgara metindir | — (mimari olarak yanlış, açılmaz) |

## 11. Yeniden kullanılabilir reçete (gelecek turda kopyala)

> **"İki mod, iki zamanlayıcı" drift avı:** Bir özellik bir modda çalışıp diğerinde çalışmıyorsa,
> önce **iki modun zamanlayıcılarını yan yana koy** (`grep -n "changed |=" <her iki dosya>`).
> Protokol/taşıma katmanını suçlamadan önce bunu yap — taşıma genellikle ortaktır ve çalışır.
> Sonra kalıcı korumayı **parite testi** olarak yaz; tek bir çağrıyı eklemek semptomu,
> parite testi hastalığı iyileştirir.

## 11.5 ✅ Faz 0 (upstream senkronu) TAMAMLANDI — Faz 1 dayanakları yeniden doğrulandı

**2026-07-25:** 130 commit senkronlandı (`b48bd903` → `362d6f14`), fork AGPL'de kaldı,
`just check` yeşil (**4022/4022**, 8 ardışık temiz tur). Detay: `.local/prd/2026-07-25-FAZ0-upstream-sync-PRD.md`.

Bu dosyadaki §1–§4 iddiaları merge SONRASI tek tek yeniden ölçüldü — **hepsi hâlâ geçerli**,
yalnız satır numaraları kaydı:

| İddia | Merge öncesi | Merge sonrası | Durum |
|---|---|---|---|
| `sync_image_preview_worker` = `pub(super)` | `image_preview_worker.rs:268` | aynı | ✅ |
| Headless zamanlayıcı onu çağırmıyor | yok | **hâlâ yok** | ✅ Faz 1 geçerli |
| `image_preview_cell_size` tek üretim ataması (monolithic) | `app/mod.rs:1203` | `app/mod.rs:1262` | ✅ |
| Headless `encode_local_pane_graphics` çağırıyor (L3 kurulu) | `headless.rs:3449` | `headless.rs:3744` | ✅ |
| İstemci `cell_width_px` teli üzerinden bildiriyor | `client_transport.rs` | `protocol/wire.rs:328,360` | ✅ |
| `OptionalPlugin` `action_id`'yi atıyor | `trail_snapshots.rs:709` | `:710` | ✅ |

### Faz 0'ın Faz 1'e getirdikleri

- `PROTOCOL_VERSION` **16 → 18** — ⚠️ yeni build, eski herdr sunucusuyla konuşamaz; izole test zorunlu
- Upstream `36de78dd` "preserve kitty graphics during host repaints" **artık ağaçta**
- Upstream `88370e15` pane-graphics streaming API (~3.700 satır) **artık ağaçta** — L3 altyapısı zenginleşti
- `HostSourceKey` soyutlaması geldi: FM önizlemesi `Terminal { pane_id, image_id }`'ye eşlendi
- `compute_tab_surface` / `TabSurfaceView` — Faz 1'in dokunacağı geometri artık ayrı modülde

### Faz 0'ın ortaya çıkardığı ve düzeltilen 4 gizli kırılganlık

Hepsi **aynı sınıf**: satırlar `modified` DESC + natsort ile sıralanıyor; fixture'lar art arda
yazıldığında mtime'lar aynı tick'e düşerse isme göre, düşmezse yaratılma sırasına göre diziliyor.
Upstream'in +339 testi koşuyu yavaşlatınca gizli bağımlılık açığa çıktı.

| Test | Belirti |
|---|---|
| `fm::branch_change_retires_descendant_focus_and_rebinds_ancestor` | satır indeksi ters döndü |
| `file_operation_worker::app_copy_action_prepares_exact_selection…` | seçim sırası ters döndü |
| `ui::compute_view_refreshes_and_clears_file_manager_action_bar_content` | satır 0 etiketi değişti |
| `file_preview_worker::stale_worker_completion_after_scroll_is_rejected` | ilk istek yanlış dosyaya gitti → 2 sn timeout |

Çözüm tek merkezde: `crate::fm::pin_equal_fixture_mtimes` — `sort_entries`'in yanında yaşıyor.
**Kural:** FM satır sırasına, indeksine veya seçim sırasına dayanan her test fixture mtime'larını
sabitlemek ZORUNDA.

## 12. Doğrulanamayanlar (dürüstlük kaydı)

- `cargo`/`just check` **çalıştırılmadı** — tüm iddialar kaynak okumasından, test sonucundan değil
- codebase-memory-mcp grafiği bu turda kullanılmadı (grep + sed ile doğrudan okuma)
- Faz 1'in gerçekten görüntü ürettiği **canlı terminalde doğrulanmadı** (F1-R6 insan testi)
- `calamine` API'si crates.io'dan **teyit edilmedi**; şekil önceki tur ekosistem taramasından

---
*v1.0.0 — 2026-07-25 · L0–L5 katman modeli burada tanımlandı. Önceki turun "mimari olarak*
*erişilemez" tanısını düzeltir: kısıt L4 scheduler drift'idir, L3 taşıma zaten kuruludur.*
