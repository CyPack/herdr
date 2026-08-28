# F1 — Opus ses akışı: araştırma ve envanter

**Tarih:** 2026-08-28 · **Dal:** `feat/media-capability` (F0 master'a indi) · **Faz:** F1
**Kanonik tasarım:** `docs/references/remote-media-transport.md` §4 (L0-L6), §5.1, §6, §7.9.6
**Pattern kataloğu:** `docs/patterns/remote-media-transport.md` (RM1-RM13 / RA1-RA10)

> ⛔ Bu alan **üç kez** araştırıldı, üçüncüsünde kalıcılaştırıldı. Bu belge sıfırdan araştırma
> DEĞİL; kanonik tasarımın F1'e düşen kısmını **ölçülmüş kod gerçekliğiyle** yüzleştirir ve
> yalnız orada açık kalan kararları kapatır.

---

## 1. Mevcut durum — ölçülmüş envanter (tahmin değil)

### 1.1 Yazıcı kuyruğu: üç şerit, kesin öncelik

`src/server/client_transport.rs` — `ClientWriterQueueState`:

| Alan | Tip | Anlamı |
|---|---|---|
| `control` | `VecDeque<Vec<u8>>` | Sınırsız kuyruk; **her zaman önce** boşalır |
| `ordered` | `VecDeque<Vec<u8>>` | Sıralı render; `send_ordered` doluysa `Full` döner (geri-basınç) |
| `render` | `Option<Vec<u8>>` | **Tek slot** — en yeni render eskisini ezer (coalescing) |

`recv()` sırası: `control` → `ordered` → `render`. Bu davranış F0'da
`TP-CLIENT-WRITE-PRIO-01` olarak davranış kaydına bağlandı (`behaviors/shared-surfaces.md`).

**F1 için sonuç:** medya dördüncü şerittir ve **EN SONA** gelir
(`control → ordered → render → media`). Gerekçe: kabul kriteri B4 (kontrol gecikmesi
değişmeyecek) ancak medya en düşük öncelikte olursa yapısal olarak garanti edilir. Medyanın
render'ın önüne geçmesi, terminal çiziminin medya yüzünden gecikmesi demektir — bu, kullanıcının
"arayüz yavaşlamasın" kısıtının doğrudan ihlalidir.

### 1.2 ⚠ KRİTİK BULGU — yazma çağrısı bloklayıcı ve atomik

```rust
// src/server/client_transport.rs:791
fn write_framed_bytes(stream: &mut LocalStream, data: &[u8]) -> bool {
    if let Err(err) = stream.write_all(data) { ... }   // ← TEK ÇAĞRI, TÜM MESAJ
    if let Err(err) = stream.flush() { ... }
```

Sabitler (`src/protocol/wire.rs`):

| Sabit | Değer |
|---|---|
| `MAX_FRAME_SIZE` | 2 MB |
| `MAX_GRAPHICS_FRAME_SIZE` | **32 MB** |
| `MAX_CLIPBOARD_IMAGE_PAYLOAD` | 16 MB |

**Bu, kanonik belgenin §8 açık maddesi 1'in cevabıdır** ("32 MB çerçevenin SSH köprüsündeki
kuyruk etkisi — mantık kesin, sayı yok"). Sayı hâlâ ölçülmedi ama **mekanizma kod düzeyinde
kesinleşti:**

> Bir `Graphics` çerçevesi yazılırken yazıcı iş parçacığı `write_all` içinde **bloke** olur.
> O sırada `control` şeridinde bekleyen bir mesaj da, medya şeridinde bekleyen bir ses parçası
> da yazılamaz. **Şerit önceliği, süren bir yazmayı kesemez.**

Yani ses gecikmesinin üst sınırı şerit sırası tarafından değil, **en büyük tek mesajın yazma
süresi** tarafından belirlenir:

```
en_kötü_ses_gecikmesi ≳ (en büyük çerçeve baytı) / (bağlantı yazma hızı)
```

Kaba büyüklük (ölçülecek — TP-MEDIA-HOL-01): 32 MB / 10 MB·s⁻¹ ≈ **3,2 s**.
Bu, B2 (< 800 ms) ve B1 (0 underrun) kriterlerini tek başına düşürebilir.

**F1'in buna cevabı üç katmanlı:**
1. Medya şeridi en sona → kontrol/render **hiç** kötüleşmez (B4 yapısal garanti).
2. Jitter buffer **uyarlanır** (sabit değil): gözlenen jitter × 3, 10-500 ms arasına kırpılır
   (videocall-rs deseni, §3.1). Kısa stall'ları yutar.
3. **Kaynakta son-kullanma-tarihli düşürme** (RM6/⛔RA4): stall bitince kuyrukta biriken eski
   parçalar gönderilmez, seyrelir. Stall sonrası "hızlandırılmış patlama" olmaz.
4. Gerçek çerçeve boyutları **ölçülür**; 32 MB teorik tavandır, pratik dağılım değil.
   Ölçüm F1'in kabul kapısının parçasıdır, varsayım değil.

⛔ Bu bulgu **çerçeve parçalama (chunking)** çözümünü davet ediyor — ama o, kontrol yolunun
yazma semantiğini değiştirir ve F1'in kapsamı dışıdır (kanonik §7.9.6: QUIC/yan kanal F3'e
ertelendi). F1 ölçer ve belgeler; çözmeye kalkmaz.

### 1.3 Protokol yüzeyi (F0 sonrası)

- `PROTOCOL_VERSION = 22`, **tam eşleşme** korunuyor.
- `Hello.capabilities: Vec<CapabilityEntry>` · `Welcome.accepted: Vec<CapabilityEntry>`.
- `CapabilitySet`: `from_entries` (ilk kazanır) · `has` (bilinmeyen = false, **hata değil**) ·
  `values_of` · `intersect` · `entries` · `into_entries`.
- Ad sabitleri: `media.streams` · `media.audio.sink` · `media.video.decode` · `media.side_channel`.
- `server_capabilities()` ve `client_capabilities()` şu an **boş** döner (F0 kasıtlı kararı:
  mekanizma var, ilan yok).

**F1 için sonuç:** yeni bir medya yeteneği `PROTOCOL_VERSION` bump'ı **gerektirmez** (RA7).
F0'ın tüm amacı buydu. Bump refleksine kapılmadan önce sorulacak soru: *"yetenek adıyla
çözülür mü?"* — F1'de cevap **evet**.

⚠ Ama `ServerMessage`/`ClientMessage` enum'una **yeni varyant eklemek** ayrı bir konudur:
bincode self-describing değildir, varyant **sırası** wire tag'idir. Yeni varyantlar
**enum'un SONUNA** eklenir ve tag sırası testle çivilenir (2026-08-11 ölçülmüş olayı).

### 1.4 Mevcut enum sırası (yeni varyantların ekleneceği yer)

`ServerMessage` son varyantlar: `... GraphicsFile{..}, GraphicsTransmissionRetired{..}` ← **buradan sonrası F1**
`ClientMessage` son varyantlar: `... InputPixels{..}, GraphicsTransmissionStarted{..}` ← **buradan sonrası F1**

### 1.5 `sound.rs`'in kararı ve sınırı

```
//! Embeds mp3 files in the binary and plays them via system audio tools.
//! Uses afplay (macOS), Windows MediaPlayer, or decoder-capable Linux audio
//! players — no Rust audio dependencies.
```

Bu karar **bilinçli**: dağıtılan ikili hiçbir ses kütüphanesine bağlanmıyor.
`Cargo.toml`'da **`[features]` bölümü hiç yok** (ölçüldü: `[package] [dependencies]
[patch.crates-io] [target.'cfg(windows)'.dependencies] [dev-dependencies]`).

**F1 için sonuç:** `[features]` bölümü **sıfırdan açılır**, `media-sink` **varsayılan değildir**.
Kabul kriteri B6 bunun kapısıdır: flag kapalıyken derleme yüzeyi değişmez.

### 1.6 L6 deseni hazır — `pane_graphics_stream`

`src/api/server/pane_graphics_stream.rs` (1241 satır) tam kardeş deseni sağlıyor:
JSON başlık satırı + parçalı gövde, idle/total timeout çifti, `stream_registry` ile
sahiplik, `dispatch_stream_open`/`dispatch_stream_frame`.

`pane_audio_stream` bu dosyanın yapısını izler — yeniden icat edilmez.

---

## 2. Codec kararı — kanıtla, tercihle değil

### 2.1 Adaylar ve ölçülen durum (crates.io, 2026-08-28)

| Crate | Sürüm | Son güncelleme | İndirme | Lisans | C bağımlılığı | Not |
|---|---|---|--:|---|---|---|
| `opus` (SpaceManiac) | 0.4.0 | 2026-08-23 | 1.615.507 | MIT/Apache-2.0 | **VAR** → `opusic-sys 0.7.3` | libopus'a güvenli bağlama; referans uygulama |
| **`opus-rs`** (restsend) | **0.1.32** | **2026-08-23** | 97.237 | BSD-3-Clause | **YOK** (yalnız opsiyonel `libm`) | Saf Rust, libopus 1.6'dan port, `#![no_std]` + alloc'suz |
| `ropus` (0x4D44) | 0.12.18 | 2026-05-11 | 3.143 | BSD-3-Clause | `cc` + `wide` | Saf Rust (fixed-point), bit-exact iddiası; **videocall-rs'in seçimi** |
| `audiopus` | 0.2.0 | **2021-04-22** | 1.439.219 | — | VAR | ⛔ **5 yıldır güncellenmedi**; 0.3.0-rc.0 rc'de kalmış |
| `magnum-opus` | 0.3.2 | 2020-06-06 | 14.266 | — | VAR | ⛔ terk edilmiş |

Advisory taraması (`pkg-registry`, RustSec/GHSA): `opus` **temiz**, `cpal` **temiz**.

⚠ Kanonik belge §L3 `audiopus`'u aday sayıyordu. **Ölçüm bunu çürüttü:** son yayın 2021.
Belge yazıldığında bu kontrol edilmemişti. Bu, belgeyi düzelten ilk F1 bulgusudur.

### 2.2 Prior-art: alandaki tek gerçek taşıyıcı ne yaptı

`videocall-rs` — 91 repoluk `remote-media` ekosisteminde **RM03 (jitter-buffer) taşıyan tek
repo** ve gerçek zamanlı Opus'u ağ üzerinden taşıyan referans uygulama. Kendi
`Cargo.toml`'larındaki yorum verbatim:

```toml
# bot/Cargo.toml:28
ropus = "0.12"  # Pure-Rust Opus encoding (replaces C libopus/audiopus-sys)

# neteq/Cargo.toml:52
# Pure-Rust Opus (xiph/opus port, bit-exact, BSD-3): the native Opus codec —
```

Yani alanın referans uygulaması **C libopus/audiopus-sys'ten saf Rust'a GEÇTİ.** Bu, bizim
bağımsız olarak yeniden keşfetmemiz gereken bir karar değil; **kanıt**.

### 2.3 Karar

**Birincil: `opus-rs` (saf Rust) · Yedek: `opus` (libopus bağlaması)**

| Boyut | Gerekçe |
|---|---|
| **Derleme yüzeyi** | `just check` üç platformu kapsıyor (Linux nextest + clippy + **windows-lint**). Saf Rust crate hiçbir sistem paketi istemez; C bağlaması Windows/macOS'ta ya sistem libopus ya vendored `cc` derlemesi ister. `media-sink` flag'i **CI'da açık koşacaksa** (B7: kırmızı test bırakma), bu yük her platforma yayılır. |
| **Lisans** | herdr **AGPL-3.0-or-later + ticari** çift lisanslı. BSD-3-Clause her iki kolda da güvenli. (⛔ snapcast **GPL-3.0**: yalnız **tasarım referansı**, kod kopyalanmaz — ticari kolu kırar.) |
| **Bakım** | 2026-08-23 (5 gün önce), aktif; `audiopus`'un tersine. |
| **Prior-art** | Alanın referans uygulaması saf-Rust'a geçti (§2.2). |
| **Geri alınabilirlik** | Codec bir **dikişin** arkasına konur (`MediaEncoder`/`MediaDecoder`). Crate değişimi tek dosyalık iştir. Bu, kararı ucuz ve tersinir yapar. |

**⚠ Kalan risk ve onu kapatan ölçüm:** `opus-rs` 0.1.x, 97k indirme, kendi README'sinde
"Production-ready" **iddia ediyor** — iddia kanıt değildir. Kapanış:

> **TP-MEDIA-OPUS-INTEROP** — bizim encoder'ımızın ürettiği akış, **bağımsız bir libopus
> uygulamasıyla** (`ffmpeg -c:a libopus` / `opusdec`) çözülebilmelidir. Yalnız kendi
> decoder'ımızla roundtrip **yeterli değildir**: bozuk ama kendi içinde tutarlı bir codec
> bu testi geçer. İki bağımsız uygulama = [[core-principles]] §2 çapraz doğrulaması.
> Bu test aynı zamanda F4'ün (PWA · WebCodecs native Opus) ön koşulunu **şimdi** kanıtlar.

Interop testi düşerse karar **`opus` + `opusic-sys`'e döner** ve derleme yükü kabul edilir —
dikiş bunu tek dosyada mümkün kılar.

### 2.4 İstemci ses sink'i: `cpal`

`cpal 0.18.2` — 2026-08-16, 18.912.777 indirme, RustAudio (Rust ses ekosisteminin kanonik
organizasyonu), Apache-2.0, **advisory temiz**.
Ekosistem kanıtı: RM01 (device-selection) **40 repo** taşıyor; `spotify-player`, `ncspot`
cpal kullanıyor.

⚠ Platform ağacı geniş (Linux `alsa`, macOS `objc2-*`/`coreaudio-rs`, Windows `windows 0.62`).
Bu **tam olarak** `media-sink` flag'inin izole etmek için var olduğu yüktür.
⚠ Kanonik belge §8 açık madde 5: *"macOS'ta cpal + herdr'ın sinyal/terminal yönetiminin
etkileşimi test edilmedi."* → F1'de **erken izole prob** (TP-MEDIA-SINK-MACOS), kod
yazmadan önce değil ama L3'ten önce.

---

## 3. Saat ve jitter — iki referanstan damıtılan mekanizma

### 3.1 Jitter tahmini: RFC 3550 (videocall-rs üzerinden doğrulandı)

`~/.cartography/refpool/videocall-rs/videocall-codecs/src/jitter_estimator.rs`:

```
D(i,j) = (R_j − R_i) − (S_j − S_i)          // varış farkı − gönderim farkı
J(i)   = J(i−1) + (|D(i−1,i)| − J(i−1))/16   // RFC 3550 üstel yumuşatma
```
Yalnız **sıralı** paketlerde güncellenir (RFC 3550 kuralı).

`jitter_buffer.rs` — uyarlanır oynatma gecikmesi:

| Sabit | Değer | herdr karşılığı |
|---|---|---|
| `MIN_PLAYOUT_DELAY_MS` | 10 | ses için de 10 ms taban |
| `MAX_PLAYOUT_DELAY_MS` | 500 | B2 (<800 ms) ile uyumlu tavan |
| `JITTER_MULTIPLIER` | 3,0 | güvenlik payı |
| `DELAY_SMOOTHING_FACTOR` | 0,99 | hedef gecikme **ani** değişmez |
| `MAX_BUFFER_SIZE` | 200 | üst sınır |

⚠ **Uyarlama:** videocall-rs `seq × FRAME_PERIOD_MS`'i gönderim zamanı **vekili** olarak
kullanıyor (sabit 30 fps varsayımı). herdr'da `pts_us` **gerçek** gönderim zamanıdır →
vekile gerek yok, `D` doğrudan `(arrival − pts)` farklarından çıkar. Bu, kaynağın
kare hızı değiştiğinde bozulmayan **daha doğru** bir tahmin verir.

⚠ Lisans: videocall-rs — kopyalamadan önce lisans kapısı işletilecek; algoritma RFC 3550'dir
(spec, telif değil), **kod kopyalanmaz**, desen alınır.

### 3.2 Saat offset'i: snapcast (⛔ GPL-3.0 — yalnız tasarım referansı)

`client/time_provider.cpp`:
```
diff_ms = (c2s − s2c) / 2          // NTP dört-damga deseninin kısaltması
diffToServer_ = medyan(diffBuffer)  // gürültü medyan filtreyle atılır
// son senkron > 60 sn ise tampon TEMİZLENİR (uyku/suspend sonrası bayat offset)
```

Üç ders, üçü de F1'e giriyor:
1. Offset **medyan**la filtrelenir (ortalama değil — tek bir gecikmeli ping ortalamayı bozar).
2. **Bayat tampon temizlenir**: 60 sn'den eski senkron varsa geçmiş atılır. Laptop uyuduğunda
   saat sıçrar; bu kural onu yakalar. (herdr'ın **dormancy** özelliği bunu doğrudan ilgilendirir.)
3. Sürüklenme düzeltmesi **tek örnek** ekleyip çıkararak yapılır (48 kHz'de 1 örnek ≈ 0,021 ms).

---

## 4. Kanonik belgeye yapılan düzeltmeler (bu araştırmanın ürünü)

| # | Belgede yazan | Ölçülen | Aksiyon |
|---|---|---|---|
| D1 | `audiopus` aday codec (§L3) | Son yayın **2021-04-22**, rc'de donmuş | `opus-rs` birincil, `opus` yedek; belge güncellenecek |
| D2 | §8 açık madde 1: 32 MB kuyruk etkisi "ölçülmedi" | **Mekanizma bulundu**: `write_all` atomik+bloklayıcı → şerit önceliği süren yazmayı kesemez | Sayı TP-MEDIA-HOL-01 ile ölçülecek; tasarım 3 katmanlı savunmayla yanıtlıyor (§1.2) |
| D3 | §L1 `Capability` **enum** olarak taslaklanmıştı | F0 **ad tabanlı** uyguladı (bincode self-describing değil) | Belge F0'da düzeltildi ✅ |
| D4 | §L4 "NTP dört-damga" | snapcast pratikte `(c2s−s2c)/2` + **medyan** + **60 sn bayatlık temizliği** | 60 sn kuralı tasarıma eklendi (dormancy ile kesişiyor) |

---

## 5. Kaynak kaydı (kanıt sözleşmesi)

| Kaynak | Ne için | Tip | Güven |
|---|---|---|---|
| `src/server/client_transport.rs:250-300,791-801` | Şerit yapısı + bloklayıcı yazma | source_code | 0,95 |
| `src/protocol/wire.rs:38-46` | Çerçeve tavanları | source_code | 0,95 |
| `src/sound.rs:1-6` | "no Rust audio dependencies" kararı | source_code | 0,95 |
| `Cargo.toml` | `[features]` yokluğu | source_code | 0,95 |
| crates.io API (2026-08-28) | 6 crate'in sürüm/tarih/indirme/lisans verisi | official_registry | 0,9 |
| `pkg-registry` MCP (RustSec/GHSA) | `opus`, `cpal` advisory taraması → temiz | official_db | 0,9 |
| `videocall-rs` `*/Cargo.toml` + `videocall-codecs/src/jitter_*.rs` | Saf-Rust Opus geçişi + RFC 3550 jitter + uyarlanır gecikme | source_code | 0,9 |
| `snapcast` `client/time_provider.cpp`, `common/message/time.hpp` | Offset formülü + medyan + 60 sn bayatlık | source_code (⛔ GPL, tasarım-ref) | 0,9 |
| RFC 3550 (RTP) | Interarrival jitter tanımı | spec | 0,95 |
| `docs/references/remote-media-transport.md` | Kanonik tasarım | project_doc | 0,95 |

**Lisans kapısı işletildi:** snapcast GPL-3.0 → kod kopyalanmaz (herdr'ın ticari kolunu kırar),
yalnız formül/desen. videocall-rs → kopyalamadan önce lisans doğrulanacak; şu an yalnız desen.
`opus-rs`/`ropus` BSD-3-Clause, `cpal` Apache-2.0, `opus` MIT/Apache → çift lisansla uyumlu.
