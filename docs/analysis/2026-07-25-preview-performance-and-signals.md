---
doc: herdr-analysis
domain: preview-performance
subject: önizleme debounce · uzak kalite · grafik kare aşımı sinyali (Katman C+D)
created: 2026-07-25
method: salt-okuma kaynak analizi; benchmark ÇALIŞTIRILMADI — nicel ifadeler ÖLÇÜLMEDİ işaretli
status: canonical — kritik bulgu 4 bağımsız kanıtla doğrulandı
git_note: /docs/* gitignored → lokal. Makine kopyası ~/.cartography/herdr-analysis/
agentic_triggers:
  - "debounce · image preview worker · slot · generation · uzak kalite · ssh bant genişliği"
  - "MAX_GRAPHICS_FRAME_SIZE · sessiz düşürme · monolithic mode · headless önizleme"
related:
  - docs/analysis/2026-07-24-architecture-seams.md
  - docs/analysis/2026-07-24-document-render-internal-state.md
---

# Önizleme Performansı ve Sinyaller (Katman C+D) — 2026-07-25

> Koordinatör tarafından agent raporundan yazıldı (agent kesildi). Kritik bulgu koordinatörce
> bağımsız doğrulandı + 4. kanıt eklendi.

---

## 🔴 EN KRİTİK BULGU — FM görsel önizlemesi server/client modunda ERİŞİLEMEZ

**Bu, "PNG render hazır, sadece `experimental` bayrağını aç" varsayımını ÇÜRÜTÜR.**

Dört bağımsız kanıt (hepsi kaynaktan, `grep`/`sed` ile doğrulandı):

```
1. src/app/image_preview_worker.rs:268
     pub(super) fn sync_image_preview_worker(&mut self) -> bool
     ^^^^^^^^^^ = pub(in crate::app)  →  crate::server ERİŞEMEZ (derleme engeli)
   krş. file_preview_worker.rs:382 → pub(crate)  (metin worker'ı headless'tan çağrılabiliyor)

2. grep -rn "sync_image_preview_worker" src/server/   →  0 İSABET

3. image_preview_cell_size üretimde YALNIZ src/app/mod.rs:1203'te yazılır
     headless'ta hiç yazılmaz → HostCellSize::default() = (0,0)
     → is_known()==false (kitty_graphics.rs:51-53)
     → image_geometry_for_content_area:91-93 → None → hedef yok

4. App::run TEK üretim çağrısı: src/main.rs:782, bağlamı:
     app::App::new(config, true /* no_session — monolithic mode
                                  never saves/restores sessions */, …)
```

**Zincir sonucu (headless/server modunda):**
```
sync_image_preview_worker ÇAĞRILMAZ
  → ImagePreviewSlot.active daima None → FmImagePreviewState daima Pending
  → kitty_graphics.rs:237 `let ...Ready{..} = &preview.state else { return None }`
  → collect_file_manager_image_placement → None → sıfır grafik baytı
  → UI: ui/file_manager.rs:1002 "(image preview pending)"  ← KALICI
```

conf **0.93** (4 halka statik kanıt; end-to-end runtime doğrulaması YAPILMADI).

### ⭐ YAZI KARŞILAŞTIRMASI — neden yazi'de çalışıyor (kullanıcı sorusu, 2026-07-25)

Kaynak: `~/.cartography/refpool/yazi-src/yazi-adapter/src/` (kaynaktan okundu).

| Boyut | **yazi** | **herdr** |
|---|---|---|
| Mimari | **Tek process** — TUI = uygulama | **Server/client** — server render, client frame alır |
| Görsel çıkışı | `w.write_all(&b1)` → **doğrudan stdout** (`drivers/kgp.rs:328-329`) | `FrameData.graphics` → protokol → client stdout |
| Sürücü sayısı | **7** — `kgp` (kitty), `kgp_old`, `iip` (iTerm2), `sixel`, `chafa`, `ueberzug` (X11 overlay) + `driver.rs` dispatcher; hepsi aynı imza `image_show(path, max: Rect) -> Result<Rect>` | **1** — yalnız kitty |
| Önizleme worker'ı | Aynı process → katman sorunu **yok** | ⚠️ `pub(super)` → server katmanı **çağıramıyor** |
| Fallback | Emülatör tespiti → uygun sürücü | **Yok** — Kitty desteklemeyen terminalde hiçbir şey |

**İki çıkarım:**
1. herdr'ın sorunu **protokol değil**. Kitty implementasyonu sağlam (462+739 satır, dedup, chunking,
   5 katmanlı limit). Sorun **worker'ın yanlış process katmanında olması**. yazi'de bu bölünme yok.
2. yazi'nin çok-sürücülü adapter deseni, herdr'ın "Kitty yoksa hiçbir şey" davranışının çözümü —
   ama Ghostty VT sixel ayrıştırmıyor (kanıt: `find vendor -ipath "*sixel*"` → boş), yani
   herdr'a doğrudan taşınamaz; ancak **halfblock/unicode fallback** sürücüsü taşınabilir.

### Sonuçları
- PNG önizlemesi yalnız **monolithic mod**da (`herdr` tek-process, session'sız) canlı.
- Normal server/client kullanımında (detach/reattach/SSH) **hiç çalışmıyor**.
- "SSH'ta 11 MB base64 donması" senaryosu **FM önizlemesi için bugün gerçekleşmiyor**; uzakta
  gerçekleşen tek büyük grafik yükü **terminal pane grafikleridir** (`collect_visible_placements`).
- ⚠️ **Karar gerektirir:** (i) ayrı bug olarak izle · (ii) düzelt (görünürlük + cell_size yolu) ·
  (iii) kabul et, C2'yi pane grafiklerine yönlendir.

---

## Düzeltilen iki önerme (önceki brifing yanlıştı)

| Önerme | Gerçek | Kanıt |
|---|---|---|
| "`app/input/file_manager.rs:3098,3120,3125` üretim çağrı noktası" | **Hepsi TEST** — dosya `:1329`'dan sonra tamamen `#[cfg(test)]` | `:1329` `#[cfg(test)]`, `:1330` `mod tests` |
| "`headless.rs:3616` asimetrisi kasıtlı mı?" | **KASITLI + tip düzeyinde zorunlu** — `pub(super)` vs `pub(crate)` | yukarıda kanıt 1 |

### Üretim çağrı noktalarının TAM listesi
| Fonksiyon | Yer |
|---|---|
| `sync_image_preview_worker` | `app/runtime.rs:212` · `app/mod.rs:1204` |
| `sync_file_preview_worker` | `app/runtime.rs:211` · `server/headless.rs:3616` |

Başka üretim çağrısı **yok**.

---

## C1 — Debounce: brifingdeki gerekçeyle **GEREKSİZ**

Mevcut slot zaten koalesans yapıyor:
```
ImagePreviewSlot: pending = tek Option (yeni istek eskisini ÜZERİNE YAZAR)  :86, :184
                  accepts(gen,key) → bayat sonuç REDDİ                       :60-62
MIN_RENDER_INTERVAL = 16 ms kare kapısı                                      mod.rs:49
```
⇒ N adımlık burst'te **≤2 decode** çalışır (uçuştaki + son hedef). "Her satırda decode" **yanlış**.

**Debounce'un gerçekten ekleyeceği:** ① burst'ün ilk decode'unu da elemek ② uzak yolda ara
iletimleri kesinlikle elemek ③ yavaş mount'ta I/O'yu hiç tetiklememek.
⇒ **C1'i tek başına perf işi olarak konumlandırma; C2'nin ilk kademesi yap.**

### Zaman nereden gelir (render saflığı bozulmadan)
Emsal **hazır**: `FileManagerVerticalWheelBurstGate` (`input/file_manager.rs:17,34-72,949-956`) —
saf çekirdek `accept_at(..., at: Instant)` + ince sarmalayıcı `Instant::now()`.
Ve `now` **her iki üretim çağrı noktasında zaten elde**: `runtime.rs:198` parametre, `mod.rs:1166` yerel.

⚠️ **İki fazlı zorunlu tasarım** (naif debounce sessiz hata üretir):
```
FAZ 1 (ANINDA): hedef değişti → generation bump + active=yeni  ⇒ eski iş derhal geçersiz, UI=Pending
FAZ 2 (pencere sonrası): pending=Some(Request)                  ⇒ UI=Loading
```
⚠️ **Zorunlu entegrasyon:** `runtime.rs:555-590` deadline dizisine ekle — yoksa kullanıcı durduğunda
ertelenen iş **hiç tetiklenmez** (önizleme asla gelmez).

Öneri: **40 ms**, `image_preview_debounce_ms`, `0`=kapalı. ⚠️ ÖLÇÜLMEDİ.

---

## C2 — Uzak-oturum tespiti **BUGÜN İMKÂNSIZ** (çift kanıt)

| Kaynak | Bulgu |
|---|---|
| `protocol/wire.rs:308-327` `ClientMessage::Hello` | alanlar: version, cols, rows, cell_width_px, cell_height_px, requested_encoding, keybindings, launch_mode → **uzaklık alanı YOK** |
| `server/clients.rs:24-61` `ClientConnection` | 18 alan → **uzaklık göstergesi YOK** |

Env tespiti de güvenilmez: uzak topolojide gerçek istemci **yerel makinede** çalışır
(`remote/unix.rs:194-218`), sunucuya **unix soketten** bağlanır → soket türünden çıkarım imkânsız.

| Seçenek | Protokol | Not |
|---|---|---|
| **A: config bayrağı** (`[remote] preview_quality`) | ❌ değişmez | ✅ Öneri. Çok-istemcili senaryoyu çözmez |
| B: istemci bildirir (`Hello` alanı) | ⛔ **16→17 bump** | Per-client çözer; CLAUDE.md refactor-risk sınıfı |
| C: ölçüm-tabanlı adaptif | — | ❌ `render_prof` üretimde **kapalı** (opt-in) → politika girdisi olamaz |

**`PreviewFallback` yeniden KULLANILAMAZ:** modül sözleşmesi (`preview_capability.rs:1-5`)
*"never ... loads configuration"* diyor; uzak politika config okur.
**Doğru katman:** `kitty_graphics.rs:87-118` hedef geometri (zaten downscale noktası) → **P2: /2 veya /3 çarpan**.

---

## D1 — Sessiz düşürme: 1 değil **3 yol**

```
headless.rs:3465-3475  ham grafik > 32 MiB    → warn! + graphics.clear()
headless.rs:3501-3543  serileşmiş kare aşımı  → warn! + graphics.clear()
headless.rs:3544-3552  grafik yok ama büyük   → warn! + continue  ← kare TAMAMEN atlanır
```
Üçü de yalnız `warn!`. `commit_graphics_cache=false` → **her karede yeniden dener** → gürültü riski.

**Önerilen kanal:** `app.config_diagnostic` (state `state.config_diagnostic`, render `ui.rs:909-916`,
deadline `runtime.rs:570`) — sunucu-sahipli, geçici, **zaten render ediliyor**, protokol değişikliği yok.
⚠️ **İdempotent set zorunlu** (aynı sebep → no-op), emsal `set_image_state:366-369`.
Metin: `graphics too large for this frame — image not shown (32 MiB limit)`.

---

## Bağımsızlık

```
D1  TAM BAĞIMSIZ (headless.rs + state.rs)        ⇒ paralel
C1  BAĞIMSIZ; C2-P1 ile image_preview_worker.rs'te çakışır
C2  C1'e mantıksal bağımlı; ana dosya kitty_graphics.rs
A   TAM BAĞIMSIZ (preview_capability.rs + trail_snapshots.rs) — C2 buraya DOKUNMAMALI
```

## Test noktaları (koddan ÖNCE)

Zaman-deterministik teknikler **kanıtlı**: ① zaman enjeksiyonu (`input/file_manager.rs:6058-6067`)
② çapa geri sarma (`app/preview.rs:971`) ③ `render_prof::observe_for_test` (`render_prof.rs:279`).
Hiçbir test uyumaz.

- **C1-T2** `..._invalidates_stale_generation_immediately_even_when_deferred` — **en kritik**; geçmezse yanlış görsel
- **C1-T4** `image_debounce_deadline_enters_loop_budget` — yoksa "önizleme hiç gelmez" hatası kaçar
- **C1-T5** burst → `fm.image_worker.submitted == 1`
- **C2-T1** downscale determinizmi · **C2-T5** kalite değişimi `ImageSignature`'ı değiştirir
- **D1-T2** ikinci yol (brifingde yoktu) · **D1-T3** tekrar eden düşüş kareyi kirletmez

## Upstream merge sonrası tazelenmesi gerekenler

`36de78dd` `render_stream.rs` + `headless/tests/pane_graphics.rs` değiştirmiş →
**D1'in tamamı** (3 düşürme yolu, `commit_graphics_cache`, splice noktası) merge sonrası yeniden
doğrulanmalı. C1/C2 tasarımı ve Soru 1-3 analizi **etkilenmez** (kaynak dosyaları değişen listede yok).
⇒ **D1'i merge'den SONRA, C1/A'yı önce/paralel ele almak daha güvenli.**

---
*v1.0.0 — 2026-07-25 · Hiçbir benchmark çalıştırılmadı.*
