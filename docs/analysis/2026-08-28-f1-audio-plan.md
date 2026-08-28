# F1 PRD + Plan — Opus ses akışı (mevcut SSH kanalı)

**Tarih:** 2026-08-28 · **Faz:** F1 · **Önkoşul:** F0 ✅ (master'da)
**Araştırma:** `2026-08-28-f1-audio-research.md` (aynı dizin) — **önce o okunur**
**Kanonik:** `docs/references/remote-media-transport.md` §4/§5.1/§6/§7.9.6 ·
`docs/patterns/remote-media-transport.md` RM3·RM4·RM6·RM9·RM12 · ⛔RA3·RA4·RA7·RA8

---

## 1. Amaç

Uzak bir herdr istemcisinde koşan pane'in **sesi, kullanıcının kendi makinesinden çıksın.**
Bugün imkânsız: `ServerMessage::Notify` yalnız gömülü bir mp3 çalabiliyor; sunucudan gelen
keyfi bir ses akışını taşıyacak ilkel yok.

**Neden F1 önce (kanonik §7.9.6):** *"F1 tek başına, kullanıcının sorduğu problemin çoğunu
çözer ve QUIC gerektirmez."* Ölçülmüş bütçe: Opus 64 kbps = **5,7 KB/s** = 0,34 MB/dk =
ham PCM'in **1/32**'si. K1 kapsama kanıtı: S3→S4→S5 rakip değil **aşama** — F1'de yazılan
kodun %100'ü sonraki fazlarda geçerli kalır.

## 2. Kapsam

### İÇİNDE
1. `media.streams` + `media.audio.sink` yeteneklerinin **gerçekten** ilan edilmesi (F0 mekanizması).
2. `MediaOpen` / `MediaChunk{stream_id,seq,pts_us,data}` / `MediaClose` + `MediaCredit` wire mesajları.
3. Opus encode (sunucu) / decode (istemci): 48 kHz stereo, 20 ms çerçeve, 64-128 kbps.
4. Saat: `TimeSync`/`TimeSyncReply` + medyan offset + **uyarlanır** jitter buffer + sürüklenme düzeltmesi.
5. Medya şeridi (4.) + **kaynakta son-kullanma-tarihli düşürme**.
6. `pane_audio_stream` API endpoint'i (`pane_graphics_stream` kardeşi).
7. `media-sink` feature flag'i — **varsayılan DEĞİL**.

### DIŞINDA (gerekçeli)
| Dışarıda | Neden | Nereye |
|---|---|---|
| Video / H.264 | Ayrı profil (P2), ayrı gecikme sınıfı | F2 |
| QUIC yan kanal | Yalnız C5 için gerekli (K2); C3/C4 S2 ile çözülü | F3 (ERTELENDİ) |
| **Büyük çerçevelerin parçalanması** | Kontrol yolunun yazma semantiğini değiştirir; F1 **ölçer, çözmez** (araştırma §1.2) | F2/F3 |
| PipeWire / sistem sesi yakalama | **Protokol platformu bilmemeli** (kanonik §L6) | plugin katmanı |
| PWA istemcisi | Aynı akış, ikinci istemci (`pts_us`+Opus sayesinde ikinci kodlama yolu yok) | F4 |

## 3. Kabul kriterleri — ölçülebilir

| # | Kriter | Hedef | Neden bu sayı | Nasıl ölçülür |
|---|---|---|---|---|
| **B1** | Ses kesintisi | 60 dk'da **0 underrun** | Tek kesinti duyulur; bir video karesi düşmesi duyulmaz (RM9) | uzun koşum + underrun sayacı |
| **B2** | Uçtan uca gecikme | **< 800 ms** | Opus algoritmik 5-26,5 ms; kalan jitter buffer (≤500 ms) + ağ | pts→çalma damgası farkı |
| **B3** | Saat sürüklenmesi | **< 1 ms/saat** | snapcast üretimde <0,2 ms — ulaşılabilir çıta | uzun koşumda offset trendi |
| **B4** | **Kontrol gecikmesi değişmez** | tuş yankısı taban ±0 | Regresyon kapısı; medya şeridi **en sonda** olduğu için yapısal | şerit sırası testi + akarken/akmazken ölçüm |
| **B5** | Bozulma davranışı | sıkışmada **ses düşmez** | Ses algısal olarak en pahalısı (RM9) | yapay daraltma |
| **B6** | Varsayılan ikili değişmez | `media-sink` kapalıyken derleme yüzeyi aynı | `sound.rs`'in "no Rust audio deps" kararı korunur | flag kapalı `just check` |
| **B7** | Kırmızı test yok | `just check` tam yeşil (**flag açık VE kapalı**) | Bayrak altındaki kod test edilmezse yoktur | HP kutusu, iki koşum |
| **B8** | Her davranış TP kayıtlı | `behavior_registry_check.py` OK | Merge sessizce silemesin | kapı |
| **B9** | **Codec interop** | Bizim akışımız **bağımsız libopus** ile çözülür | Kendi decoder'ımızla roundtrip bozuk-ama-tutarlı codec'i de geçer | `ffmpeg -c:a libopus` çapraz doğrulama |

## 4. Katman bölümlemesi

```
F1
├── L1 YETENEK      media.streams + media.audio.sink GERÇEKTEN ilan edilir
│   ├── L1.a sunucu ilanı (media.streams, media.audio.sink=[opus])
│   ├── L1.b istemci ilanı — YALNIZ sink varsa (media-sink açık + aygıt açıldı)
│   └── L1.c ⚠ codec kesişimi + BOŞ-DEĞER tuzağı (§5 S2)
├── L2 AKIŞ         MediaOpen / MediaChunk / MediaClose / MediaCredit
│   ├── L2.a mesaj tipleri — enum SONUNA, tag-sıra testi
│   ├── L2.b stream_id yaşam döngüsü (aç→parça→kapat; çift kapatma panik değil)
│   └── L2.c seq (boşluk=kayıp→gizleme) + pts_us
├── L3 CODEC        Opus (opus-rs), dikişin arkasında
│   ├── L3.a media-sink feature flag ([features] bölümü SIFIRDAN açılır)
│   ├── L3.b MediaEncoder/MediaDecoder dikişi + opus-rs uygulaması
│   ├── L3.c bitrate politikası (varsayılan 64k)
│   └── L3.d ⚠ interop doğrulaması (B9)
├── L4 SAAT         ⭐ en kritik katman
│   ├── L4.a TimeSync/TimeSyncReply + (c2s−s2c)/2 + MEDYAN + 60 sn bayatlık temizliği
│   ├── L4.b uyarlanır jitter buffer (RFC 3550 J; ×3; 10..500 ms; smoothing 0,99)
│   ├── L4.c sürüklenme düzeltmesi (tek örnek ekle/çıkar ≈ 0,021 ms)
│   └── L4.d oynatma kuralı: pts_us + target_latency anında çal
├── L5 ÖNCELİK      4. şerit + kaynakta düşürme
│   ├── L5.a medya şeridi — control → ordered → render → MEDIA (en son)
│   ├── L5.b kaynakta son-kullanma-tarihli düşürme (⛔RA4 bufferbloat)
│   ├── L5.c MediaCredit ile istemci tampon derinliği
│   └── L5.d bozulma sırası (RM9)
└── L6 INGEST       pane_audio_stream API (⛔ platform sızıntısı yok)
```

## 5. Bağımlılık zinciri + **sessiz başarısızlık avı**

```
L3.a ──────────────────────────────► (tüm L3/L4 kodu bayrak altında)
L1.a ─► L1.c ─► L2.a ─► L2.c ─► L3.b ─► L4.a ─► L4.b ─► L4.d ─► L5.a ─► L6
L1.b ─┘                  └─► L4.c        └─► L5.b ─► L5.c
```

| # | Adım | Ters/eksik yapılırsa | Sınıf |
|---|---|---|---|
| **S1** | L1.b önce L1.a | İstemci sink'i yokken ilan eder → sunucu **gönderir, kimse çalmaz** | ⚠ **SESSİZ** |
| **S2** | L1.c'de **boş-değer** kontrolü atlanır | `intersect` **adı korur, değerleri boşaltır**. Ortak codec yokken `has(AUDIO_SINK)==true` → sunucu akış açar, istemci **hiçbir şey çözemez** | ⚠ **SESSİZ** — kodda ölçüldü (`wire.rs` `intersect`) |
| **S3** | L2.c önce L4.b | `pts_us` taşınmadan buffer yazılır → buffer **hep boş / hep dolu** | ⚠ **SESSİZ** |
| **S4** | L4.a önce L4.c | Offset yokken sürüklenme düzeltilir → **rastgele** hız oynaması | ⚠ **SESSİZ** |
| **S5** | L3.a önce L3.b | Bayrak yokken codec kodu → **varsayılan ikili şişer** (B6 ihlali) — **derlenir!** | ⚠ **SESSİZ** |
| **S6** | Yeni varyant enum **ortasına** düşer | bincode tag kayması → eski istemci mesajı yanlış çözer | ⚠ **SESSİZ** (2026-08-11 olayı: 9,4 sn timeout) |
| **S7** | L3.d atlanır (yalnız kendi roundtrip'i) | Bozuk ama **kendi içinde tutarlı** codec testi geçer; F4'te WebCodecs çözemez | ⚠ **SESSİZ** |
| **S8** | L5.b önce L5.a | Şerit yokken düşürme → kontrol çerçevesi de düşebilir | 🔊 gürültülü |
| **S9** | `media-sink` açıkken `just check` koşulmaz | Bayrak altındaki kodun **hiçbir testi yoktur** | ⚠ **SESSİZ** (B7 bunun kapısı) |

**Kural:** ⚠ işaretli **her** adım için ayrı test noktası. (F0'da bu av iki gerçek kusur yakaladı;
F1'de daha en baştan **yedi** tane çıkardı.)

---

## 6. TEST NOKTALARI — ne · beklenen · **NEDEN**

> Her nokta önce **KIRMIZI** yazılır, sonra minimal kod **YEŞİL** yapar.
> Koşum: `cargo nextest run` (paralel `cargo test` kanıtlı flaky — rust-dev E1).

### L1 — yetenek

| TP | Ne test edilir | Beklenen | **NEDEN** |
|---|---|---|---|
| **TP-MEDIA-CAP-04** | Sunucu ilanı `media.streams` + `media.audio.sink=["opus"]` içerir | `Welcome.accepted` boş değil | F0'ın "boş ilan" assert'i **kasıtlı** değişiyor; bu değişimin bilinçli olduğu testle kayda geçer |
| **TP-MEDIA-CAP-05** | `media-sink` **kapalı** derlemede istemci `media.audio.sink` ilan **etmez** | ilan yok | **S1**: sink'i olmayan istemcinin ilanı, sunucuyu kimsenin çalmayacağı bayt üretmeye ikna eder — hiçbir test bunu kırmızıya çevirmez |
| **TP-MEDIA-CAP-06** ⚠ | İstemci `["opus"]`, sunucu `["pcm"]` ilan eder | `has(AUDIO_SINK)`==**true** AMA `values_of(...)`==**boş** → ses **açılmaz** | **S2**: `intersect` adı koruyup değerleri boşaltıyor (ölçüldü). "İsim var" ile "ortak codec var" **aynı şey değil**; kod bunları karıştırırsa akış açılır ve sessizce çözülemez |
| **TP-MEDIA-CAP-07** | İstemci `["opus","pcm"]` ∩ sunucu `["pcm","opus"]` | sonuç `["opus","pcm"]` — **istemcinin** sırası | Sıra = tercih. `intersect` `self`'in sırasını koruyor; `self` istemci tarafı. Sıra tersine dönerse sessizce **yanlış codec** seçilir |

### L2 — akış mesajları

| TP | Ne | Beklenen | **NEDEN** |
|---|---|---|---|
| **TP-MEDIA-WIRE-01** ⚠ | `ServerMessage`/`ClientMessage` varyant **sırası** sabit; yeni varyantlar **sonda** | Sıra listesi testte **çivili**; ortaya ekleme testi kırar | **S6**: bincode self-describing değil, sıra = wire tag. 2026-08-11'de tam bu kayma 9,4 sn timeout üretti ve **hiçbir derleme hatası vermedi** |
| **TP-MEDIA-WIRE-02** | `MediaOpen/MediaChunk/MediaClose/MediaCredit` roundtrip | encode→decode alan-alan eşit | Temel serileştirme sözleşmesi |
| **TP-MEDIA-WIRE-03** | `MediaChunk` boyut tavanı | `MAX_FRAME_SIZE`(2 MB) aşan parça **reddedilir**, panik yok | 20 ms Opus ≈ 160 bayt; MB'lık bir "ses parçası" bozuk/hostil girdidir |
| **TP-MEDIA-STREAM-01** | `stream_id` yaşam döngüsü: aç→parça→kapat | Kapalı `stream_id`'ye gelen parça **sessizce atılır** | Kapanış ile son parça yarışır; panik = uzak istemcinin sunucuyu düşürmesi |
| **TP-MEDIA-STREAM-02** | Çift `MediaClose` | İkincisi no-op | Aynı yarış; idempotan kapanış |
| **TP-MEDIA-SEQ-01** | `seq` boşluğu (5,6,**8**) | Kayıp **raporlanır**, akış devam eder | Kayıp normaldir; akışı kesmek kesintiyi kayıptan pahalı yapar |

### L3 — codec

| TP | Ne | Beklenen | **NEDEN** |
|---|---|---|---|
| **TP-MEDIA-OPUS-01** | encode→decode roundtrip, 48 kHz stereo 20 ms | Örnek **sayısı** korunur (960/kanal) | Çerçeve sınırı kayarsa ses hızlanır/yavaşlar; sayı ilk ve en ucuz kapı |
| **TP-MEDIA-OPUS-INTEROP** ⚠ | Bizim encoder → **`ffmpeg -c:a libopus`** ile çöz | Bağımsız decoder aynı süreyi/örnek sayısını üretir | **S7**: kendi decoder'ımız kendi bug'ımızı doğrular. İki **bağımsız** uygulama = çapraz doğrulama. Aynı test F4'ün (WebCodecs native Opus) ön koşulunu **şimdi** kanıtlar |
| **TP-MEDIA-FLAG-01** ⚠ | `media-sink` **kapalı**: derleme + testler | Yeşil; ses sembolleri **yok** | **S5/B6**: bayraklı kod bayrak dışına sızarsa derlenir ve sessizce ikiliyi şişirir |
| **TP-MEDIA-FLAG-02** ⚠ | `media-sink` **açık**: `just check` | Yeşil | **S9**: bayrak altındaki kod CI'da derlenmezse test edilmemiştir |
| **TP-MEDIA-BITRATE-01** | 64 kbps'de 1 sn ses | ≈ 5,7 KB ±%15 | Kanonik §7.5 ölçümüyle **aynı** sayı; sapma yanlış yapılandırma demektir |

### L4 — saat ve jitter (⭐)

| TP | Ne | Beklenen | **NEDEN** |
|---|---|---|---|
| **TP-MEDIA-CLOCK-01** | Bilinen 4 damgadan offset | `(c2s−s2c)/2` | Formül tek satır ama yanlış işaret **sessizce** sabit gecikme ekler |
| **TP-MEDIA-CLOCK-02** | 9 örnek, biri **10× aykırı** | Medyan aykırıyı yutar | Ortalama tek gecikmeli ping'le bozulur; snapcast'in medyan tercihi bu yüzden |
| **TP-MEDIA-CLOCK-03** ⚠ | Son senkrondan **>60 sn** geçmiş | Tampon **temizlenir**, yeni ölçüm taban olur | Laptop uyanınca saat sıçrar; bayat medyan **saatlerce** yanlış kalır. herdr'ın **dormancy** özelliği bunu garanti eder |
| **TP-MEDIA-JITTER-01** | Düzenli varış | Jitter tahmini ≈ 0 | RFC 3550 J'nin taban davranışı |
| **TP-MEDIA-JITTER-02** | Geç varış | J artar, hedef gecikme **yumuşayarak** yükselir (0,99) | Ani hedef değişimi duyulur (hız sıçraması); yumuşatma bunu engeller |
| **TP-MEDIA-JITTER-03** ⚠ | Erken gelen parça | **BEKLER** (`pts+target` anına kadar) | ⛔**RA3** "geldiği anda çal" ölümcül: jitter doğrudan sese geçer |
| **TP-MEDIA-JITTER-04** ⚠ | `pts+target`'ı geçmiş parça | **ATILIR**, çalınmaz | Geç sesi çalmak kaymayı kalıcılaştırır; atmak tek örneklik kayıp verir |
| **TP-MEDIA-JITTER-05** | Hedef gecikme kırpması | 10 ms ≤ hedef ≤ 500 ms | B2 (<800 ms) ancak tavan varsa garanti |
| **TP-MEDIA-DRIFT-01** | +%0,01 sürüklenme | Düzeltme **tek örnek** ekler/çıkarır | 48 kHz'de 1 örnek ≈ 0,021 ms = duyulmaz; blok atlamak duyulur |

### L5 — öncelik ve geri-basınç

| TP | Ne | Beklenen | **NEDEN** |
|---|---|---|---|
| **TP-MEDIA-PRIO-01** ⚠ | Kuyrukta control+render+media varken `recv()` sırası | `control` → `ordered` → `render` → **`media`** | **B4**: medya render'ın önüne geçerse terminal çizimi medya yüzünden gecikir. `TP-CLIENT-WRITE-PRIO-01`'in **regresyon** kapısı |
| **TP-MEDIA-PRIO-02** | Medya şeridi dolu, control gelir | Control **önce** çıkar | Aynı garantinin dolu-kuyruk hâli |
| **TP-MEDIA-DEADLINE-01** ⚠ | `pts`'i geçmiş parça gönderim sırası | **Kaynakta düşer**, kuyruğa girmez | ⛔**RA4**: kuyruğa yığmak bufferbloat üretir; gecikme sınırsız büyür ve **asla** toparlamaz |
| **TP-MEDIA-CREDIT-01** | İstemci kredi=2 ilan eder | Sunucu 2'den fazla bekleyen parça tutmaz | Geri-basınç olmadan yavaş istemci sunucu belleğini şişirir |
| **TP-MEDIA-DEGRADE-01** | Sıkışma artar | Sıra: video-bitrate → video-fps → video-drop → ses-bitrate → **(asla) ses** | RM9. F1'de video yok → **ses hiç düşmez**; testi şimdi yazmak F2'nin sırayı bozmasını engeller |

### L6 — ingest

| TP | Ne | Beklenen | **NEDEN** |
|---|---|---|---|
| **TP-MEDIA-API-01** | `pane_audio_stream` JSON başlık + parçalı gövde | `pane_graphics_stream` ile **aynı** sözleşme | İki kardeş endpoint ayrışırsa her istemci ikisini ayrı öğrenir |
| **TP-MEDIA-API-02** ⚠ | Protokol yüzeyinde PipeWire/ALSA/CoreAudio adı **geçmez** | grep temiz | Kanonik §L6: protokol platformu bilirse macOS/Windows'ta anlamsız mesaj taşır ve **her yeni ses altyapısı protokol değişikliği** ister |

### Ölçüm noktaları (test değil, **sayı üretir**)

| TP | Ne ölçülür | Neden |
|---|---|---|
| **TP-MEDIA-HOL-01** ⚠ | Tek `write_all` süresi: 1 MB / 8 MB / 32 MB yerel + SSH | Araştırma §1.2: bloklayıcı yazma ses gecikmesinin **gerçek** üst sınırı. Kanonik §8 açık madde 1'i kapatır |
| **TP-MEDIA-FRAMESIZE-01** | Pratikte gözlenen `Graphics` çerçeve boyutu dağılımı | 32 MB **tavan**tır, dağılım değil. `target_latency` varsayılanı buradan gelir |
| **TP-MEDIA-SINK-MACOS** ⚠ | `cpal` + herdr sinyal/terminal yönetimi (macOS) | Kanonik §8 açık madde 5: **test edilmedi**. L3'ten önce izole prob |

---

## 7. Görev tablosu

| # | Görev | Bağımlı | TP |
|---|---|---|---|
| F1.1 | PRD + test noktaları (**bu belge**) | — | — |
| F1.2 | L3.a `[features]` + `media-sink` (BOŞ, kod yok) | — | FLAG-01 |
| F1.3 | L2.a wire mesajları + tag-sıra testi | F1.2 | WIRE-01/02/03 |
| F1.4 | L1.a/b/c yetenek ilanı + boş-değer tuzağı | F1.3 | CAP-04..07 |
| F1.5 | L2.b/c stream yaşam döngüsü + seq/pts | F1.3 | STREAM-01/02, SEQ-01 |
| F1.6 | L3.b/c codec dikişi + opus-rs | F1.2 | OPUS-01, BITRATE-01 |
| F1.7 | L3.d interop (ffmpeg çapraz) | F1.6 | OPUS-INTEROP |
| F1.8 | L4.a saat offset + medyan + 60 sn | F1.5 | CLOCK-01/02/03 |
| F1.9 | L4.b/c/d jitter + drift + oynatma | F1.8 | JITTER-01..05, DRIFT-01 |
| F1.10 | L5.a medya şeridi | F1.3 | PRIO-01/02 |
| F1.11 | L5.b/c/d düşürme + kredi + bozulma | F1.10 | DEADLINE-01, CREDIT-01, DEGRADE-01 |
| F1.12 | L6 `pane_audio_stream` | F1.5 | API-01/02 |
| F1.13 | Ölçümler | F1.10 | HOL-01, FRAMESIZE-01, SINK-MACOS |
| F1.14 | Davranış kaydı + `just check` (iki bayrak) | hepsi | B7/B8 |
| F1.15 | Belge güncelleme + makine kopyaları + devir NN=50 | F1.14 | — |

## 8. Riskler

| # | Risk | Kanıt | Azaltma |
|---|---|---|---|
| S1 | `opus-rs` 0.1.x, 97k indirme, "production-ready" **iddiası** | crates.io | Dikiş + **INTEROP** testi; düşerse `opus`+`opusic-sys`'e dön |
| S2 | `cpal` × herdr sinyal/terminal (macOS) | kanonik §8 md.5 **test edilmedi** | TP-MEDIA-SINK-MACOS, L3'ten önce |
| S3 | 32 MB çerçeve **bloklayıcı** yazma | `write_framed_bytes` **ölçüldü** | Şerit en sonda + uyarlanır buffer + kaynakta düşürme; TP-MEDIA-HOL-01 sayıyı verir |
| S4 | Bayraklı kod `clippy dead_code` üretir | F0'da **7 kez** | Üreticiyi aynı dalgada bağla, yoksa **gerekçeli** allow |
| S5 | Yeni varyant enum ortasına düşer | 2026-08-11, 9,4 sn timeout | TP-MEDIA-WIRE-01 |
| S6 | `PROTOCOL_VERSION` bump refleksi | ⛔RA7, fork bunu yaşadı | F0'ın tüm amacı buydu; **bump YOK** |
| S7 | Laptopta derleme | yük ort. 61,66 ölçüldü | `herdr-hp-check` |
| S8 | HP'de üretilen dosya geri gelmez | F0'da **2 kez** | fmt yerelde; artefakt `scp` |
| S9 | `behaviors/shared-surfaces.md` paralel ajanla çakışır | `wt.sh claims` | Anlaşma: dosya **SONUNA**, `TP-MEDIA-*` vs `TP-GFX-*` |

## 9. V (sonlanma ölçüsü)

```
V = (yazılmamış test noktası) + (kırmızı test) + (kapsanmamış kabul kriteri B1-B9)
  + (kayıtsız shared-surface davranışı) + (düşen kapı)

Başlangıç: V(F1) = 33 test noktası + 9 kriter = ölçülüyor
DUR: V=0 · V iki tur sabit · eskalasyon kapısı (§D)
```
