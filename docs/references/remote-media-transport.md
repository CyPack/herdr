# Remote Media Transport — uzak istemciye ses ve görüntü

> **Soru:** herdr'a uzaktan bağlanmış bir istemciye ses ve görüntü *kusursuz* nasıl ulaşır?
> Hangi katmanlarda özelleştirme, protokolde hangi genişletmeler gerekir?
>
> Bu belge kanıt-temelli bir tasarım kararıdır, bir öneri listesi değil. Her iddia ya
> repodaki bir dosyaya ya da dış bir kaynağa bağlanır. Doğrulanmamış olanlar açıkça
> işaretlidir.

---

## 0. Yönetici özeti

herdr, bu problemin **zor kısmını zaten çözmüş** — ve bu, araştırmanın en önemli bulgusu.
`src/remote/unix.rs` bir *SSH stdio üzerinden ince istemci köprüsü* kurar: istemci senin
Mac'inde native koşar, herdr'ın kendi wire protokolü SSH'in stdio'sundan geçer. Yani
`ServerMessage::Graphics` (kitty baytları) **zaten Mac'e ulaşıyor** ve
`ServerMessage::Notify` **zaten Mac'in hoparlöründen ses çıkarıyor**.

Bu, ytsub/NoctaVox/concord gibi uygulamaların yapısal olarak yapamadığı şeydir: onlarda
istemci sunucuda koşar, aygıt sunucudadır. herdr'da istemci doğru uçtadır.

Eksik olan şey taşıma değil, **akış**. Mevcut medya ilkelleri *olay* şeklinde: tek atışlık
resim, ateşle-unut bildirim sesi. Sürekli, saatli, geri-basınçlı bir medya akışı kavramı yok.

Beş yapısal boşluk var ve hepsi kapatılabilir:

| # | Boşluk | Kanıt |
|---|---|---|
| 1 | SSH stdio tek sıralı bayt akışı → head-of-line blocking | `remote/unix.rs` köprüsü tek stdio kanalı |
| 2 | Zaman damgası yok → A/V senkron ve jitter buffer imkânsız | `wire.rs`'de hiçbir mesajda `pts`/`ts` alanı yok |
| 3 | Ses *akışı* tipi yok, yalnız bildirim sesi | `sound.rs`: gömülü mp3 + dış oynatıcı süreci |
| 4 | Akış başına geri-basınç/kredi yok | `client_transport.rs` writer loop tek kuyruk |
| 5 | Sürüm kontrolü tam-eşleşme → medya artımlı çıkamaz | `wire.rs:21` `PROTOCOL_VERSION` exact-match |

---

## 1. Bugünkü durum — kanıtlı envanter

### 1.1 Taşıma

| Katman | Durum | Kanıt |
|---|---|---|
| Yerel taşıma | Unix domain socket | `src/ipc.rs`, `interprocess::local_socket` |
| Çerçeveleme | u32-LE uzunluk öneki + serde | `src/protocol/wire.rs` |
| Sürüm | `PROTOCOL_VERSION = 20`, **tam eşleşme** | `wire.rs:21` |
| Uzak taşıma | **SSH command stdio köprüsü** | `src/remote/unix.rs:194` `run_remote_client_bridge()` |
| SSH çoğullama | `ControlMaster=auto` + `ControlPath` | `remote/unix.rs:645` |
| Ağ yığını | **Yok** — `tokio` yalnız `rt-multi-thread, macros, sync, time` | `Cargo.toml:39` (`net` özelliği yok) |
| TLS/QUIC/WebSocket | **Yok** (Rust tarafında) | `rustls`/`quinn`/`tungstenite` bağımlılığı yok |
| PWA taşıması | Bun `serve` → WebSocket ↔ API soketi (NDJSON) | `herdr-web/src/server.ts:284`, `framing.ts` |

**Sonuç:** iki istemci, iki farklı taşıma. TUI istemcisi ikili wire'ı SSH stdio'dan konuşur;
PWA satır-sınırlı JSON'u WebSocket'ten konuşur. Medya tasarımı ikisini de karşılamalı.

### 1.2 Mevcut medya ilkelleri

Bunlar sıfırdan yazılacak şeyler değil — **genişletilecek** şeyler:

| İlkel | Yön | Sınır | Dosya |
|---|---|---|---|
| `ServerMessage::Graphics { bytes }` | S→C | 32 MB (`MAX_GRAPHICS_FRAME_SIZE`) | `wire.rs` |
| `ServerMessage::Notify { kind, message, body }` | S→C | — istemci **yerel** çalar | `wire.rs` + `sound.rs` |
| `ServerMessage::ReloadSoundConfig` | S→C | — | `wire.rs` |
| `ClientMessage::ClipboardImage { extension, data }` | C→S | 16 MB | `wire.rs` |
| `pane_graphics_stream` (API) | dış üretici → pane | JSON başlık + 64 KB gövde parçaları | `api/server/pane_graphics_stream.rs` |
| Piksel geometri anlaşması | C→S | `cell_width_px`, `cell_height_px` | `Hello` / `Resize` |

`pane_graphics_stream` özellikle önemli: **sürekli kare besleme deseni zaten var**
(başlık + parçalı gövde + idle/total timeout'lar). Video akışı bu desenin
zamanlanmış ve geri-basınçlı hâlidir.

### 1.3 `sound.rs`'in bilinçli kararı — ve neden değişmesi gerekiyor

Dosyanın kendi başlığı: *"Uses afplay (macOS), Windows MediaPlayer, or decoder-capable Linux
audio players — **no Rust audio dependencies**."* Bu, ikili boyutu ve çapraz-derleme
karmaşıklığından kaçınan bilinçli bir karardır ve README'nin *"one rust binary, no electron"*
vaadiyle uyumludur.

Ses **akışı** bu kararı kısmen bozar: ses başına bir süreç doğurmak (15 sn timeout ile) sürekli
akış için kullanılamaz. Bu gerçek bir maliyettir ve §5.5'te bir feature flag ile sınırlandırılır —
sessizce geçilmez.

---

## 2. Neden "sadece SSH yeter" cevabı yanlış — üç tavan

### Tavan 1 — Head-of-line blocking

SSH stdio köprüsü **tek sıralı bir bayt akışıdır**. 32 MB'lık bir `Graphics` çerçevesi
kuyruğa girdiğinde, arkasındaki her tuş vuruşu ve her render çerçevesi onun bitmesini bekler.
Kontrol ve medya aynı boruyu paylaştığı sürece, medya arttıkça **arayüz yavaşlar**.

Bu, kullanıcının daha önce ölçtüğü lag probleminin akrabasıdır ve TCP seviyesinde de
tekrarlanır: tek TCP bağlantısında bir paket kaybı, arkasındaki tüm akışları bekletir.
QUIC'in var olma sebebi tam olarak budur.

### Tavan 2 — Sesin TTY'de temsili yok

Hiçbir escape dizisi ses taşımaz. Terminal grafik protokolleri (kitty, sixel, iTerm2) yalnız
piksel içindir. Dolayısıyla ses **ancak istemci tarafında bir sink varsa** çalar.

herdr bu testi geçiyor (istemci Mac'te native koşuyor), ama `Notify` yalnız *gömülü mp3'lerden
birini* çalabiliyor — sunucudan gelen keyfi bir PCM/Opus akışını değil.

### Tavan 3 — Saat yok

`wire.rs`'deki hiçbir mesajda zaman damgası yok. Çerçeveler "geldiğinde" çizilir. Bu, metin
için doğru karardır (en yeni durum tek doğrudur) ama medya için ölümcüldür:

- Jitter buffer kurulamaz → ağdaki her dalgalanma doğrudan duyulur.
- Ses ve görüntü hizalanamaz → dudak senkronu yok.
- Saat kayması (client kristali ≠ server kristali) telafi edilemez → dakikalar içinde sürüklenme.

**Üç tavan da aynı kök nedene inmez** — bu önemli. Tavan 1 taşıma katmanında, Tavan 2 istemci
sink katmanında, Tavan 3 protokol semantiğinde. Üçü ayrı ayrı çözülür ve ayrı ayrı test edilir.

---

## 3. "Kusursuz" ne demek — ölçülebilir hedefler

Tasarıma girmeden önce kabul kriteri. Ölçülemeyen hedef karşılanamaz.

| Boyut | Hedef | Neden bu sayı |
|---|---|---|
| Ses kesintisi | 60 dakikada 0 underrun | Tek bir kesinti bile duyulur; görüntüde bir kare düşmesi duyulmaz |
| Ses uçtan uca gecikme | interaktif < 150 ms · dinleme modu < 800 ms | Opus algoritmik gecikmesi 5–26.5 ms; kalanı jitter buffer + ağ ([Opus](https://en.wikipedia.org/wiki/Opus_(audio_format))) |
| A/V senkron | \|Δ\| < 40 ms | Yayın standartlarının toleransı (ITU-R BT.1359) bundan gevşektir; 40 ms güvenli tarafta kalır |
| Saat sürüklenmesi | < 1 ms/saat | snapcast üretimde < 0.2 ms sapma bildiriyor — ulaşılabilir bir çıta |
| Kontrol gecikmesi | medya akarken tuş yankısı **değişmemeli** | Regresyon kapısı: medya arayüzü yavaşlatmamalı |
| Bozulma davranışı | sıkışmada video düşer, **ses düşmez** | Ses kesintisi algısal olarak çok daha maliyetli |

Son iki satır tasarımın omurgasıdır: **medya kontrolü asla bloklamaz, ses videoyu asla
beklemez.**

---

## 4. Katman katman tasarım

Yedi katman. Her biri için: bugün ne var, ne eklenir, hangi prior-art'a dayanır.

```
L6  Kaynak / ingest      pane süreci → API (opt-in), plugin sistem sesini besler
L5  Kontrol & geri-basınç kredi tabanlı akış kontrolü, öncelik, düşürme politikası
L4  Saat & senkron        server zaman damgası + offset tahmini + resample düzeltmesi
L3  Codec                 Opus (ses) · kitty/PNG passthrough & H.264 (video)
L2  Akış çoğullama        stream_id, bağımsız kanallar, kontrolden ayrı
L1  Çerçeveleme & yetenek capability negotiation (exact-match sürümün yanına)
L0  Taşıma                SSH stdio (kontrol) + QUIC-over-Tailscale (medya)
```

---

### L0 — Taşıma: kontrolü ve medyayı ayır

**Bugün:** tek SSH stdio kanalı, `ControlMaster` ile çoğullanmış.

**Karar:** SSH köprüsü **kontrol yolu olarak kalır** — kanıtlanmış, sıfır-konfigürasyon,
kimlik doğrulaması çözülmüş. Medya için **ikinci ve bağımsız bir yol** açılır.

İki aday değerlendirildi:

| Aday | Artı | Eksi | Karar |
|---|---|---|---|
| İkinci SSH kanalı (aynı ControlMaster) | Yeni kimlik doğrulama yok, hemen çalışır | Aynı TCP bağlantısı → TCP seviyesinde HOL sürüyor | **Fallback** |
| QUIC over Tailscale | Akış başına bağımsızlık, datagram desteği, kayıpta HOL yok | Yeni bağımlılık (`quinn`), yeni dinleyici | **Birincil** |

Tailscale'in burada özel bir avantajı var ve bu tesadüf değil — kullanıcının kurulumu zaten
Tailscale: **kimlik, şifreleme ve NAT geçişi çözülmüş durumda.** QUIC'i tailnet üzerine
koymak, normalde QUIC'i üretime almanın en pahalı kısımlarını (sertifika, kimlik, erişim
kontrolü) ortadan kaldırır. Dinleyici yalnız `tailscale0` arayüzüne bağlanır; tailnet ACL'i
erişimi daraltır.

Ses için ayrıca **QUIC datagram** kullanılır (güvenilir akış değil): geç gelen bir ses paketi
zaten işe yaramaz, yeniden iletimi beklemek kesintiyi *uzatır*. Bu, gerçek zamanlı ses
taşımasının temel kuralıdır ve MoQ'nun da yaptığı ayrımdır
([MoQ transport](https://www.wowza.com/blog/what-is-media-over-quic-moq-and-why-are-people-talking-about-it)).

**Fallback zinciri:** QUIC → ikinci SSH kanalı → mevcut tek kanal (yalnız ses, düşük bitrate).
Hiçbir ortamda özellik tamamen kaybolmaz.

---

### L1 — Çerçeveleme & yetenek anlaşması

**Bugün:** `PROTOCOL_VERSION = 20`, **tam eşleşme**. Kaynak kodun kendi yorumu bu kararın
nedenini açıklıyor (yayınlanmış bir istemcinin farklı lehçeyi aynı numarayla konuşması).

**Problem:** tam eşleşme, medyanın artımlı çıkmasını imkânsız kılar. Medya yeteneği eklenen
her sürüm, medya istemeyen tüm istemcileri de kırar.

**Ekleme:** sürüm kontrolünü *bozmadan* yanına yetenek listesi:

```rust
// wire.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Capability {
    /// İstemci medya akışı alabilir ve saat senkronuna katılabilir.
    MediaStreams,
    /// İstemcide çalışan bir ses sink'i var.
    AudioSink { codecs: &'static [MediaCodec] },
    /// İstemci video çözüp terminale kendisi çizebilir.
    VideoDecode { codecs: &'static [MediaCodec] },
    /// İstemci ikinci bir medya yolu açabilir.
    SideChannel { kinds: &'static [SideChannelKind] },
}

// ClientMessage::Hello'ya eklenir:
    capabilities: Vec<Capability>,

// ServerMessage::Welcome'a eklenir:
    accepted: Vec<Capability>,
```

Kural: **sunucu yalnız `accepted` içindeki yetenekleri kullanır.** Yetenek listesi boş gelen
eski bir istemci, bugünkü davranışı bit-birebir aynı şekilde görür. Bu, medyayı *additive*
yapar ve `PROTOCOL_VERSION`'ın sertliğini korur.

---

### L2 — Akış çoğullama

**Bugün:** `Graphics` tek atışlık, kimliksiz. Hangi pane'e ait olduğu bağlamdan çıkar; iki
eşzamanlı görsel kaynak ayırt edilemez.

**Ekleme:** kimlikli, açılıp kapanan akışlar.

```rust
pub enum ServerMessage {
    // ... mevcut varyantlar korunur ...

    /// Yeni bir medya akışı açıldı.
    MediaOpen {
        stream_id: u32,
        pane_id: String,
        kind: MediaKind,          // Audio | Video
        codec: MediaCodec,
        /// Ses: örnekleme hızı + kanal. Video: piksel boyutu + kare hızı.
        params: MediaParams,
        /// Sunucunun bu akış için önerdiği oynatma gecikmesi (µs).
        target_latency_us: u32,
    },

    /// Akıştan bir parça.
    MediaChunk {
        stream_id: u32,
        /// Monoton dizi numarası — boşluk = kayıp, istemci gizleme uygular.
        seq: u64,
        /// Sunucu saatinde sunum zamanı (µs). A/V senkronun tek dayanağı.
        pts_us: u64,
        data: Vec<u8>,
    },

    /// Akış kapandı.
    MediaClose { stream_id: u32, reason: MediaCloseReason },
}
```

`stream_id` sayesinde ses ve video **bağımsız** akar: video sıkışsa bile ses kendi
zamanlamasında devam eder (§3'ün son satırı bu alanla mümkün olur).

`Graphics` **kaldırılmaz.** Tek atışlık görsel (bir TUI'nin çizdiği kapak resmi) için doğru
ilkel odur ve maliyeti sıfırdır. `MediaChunk` yalnız *sürekli* kaynaklar içindir.

---

### L3 — Codec

**Ses: Opus.** Tek aday, ve gerekçesi güçlü:

- Algoritmik gecikme varsayılan 26.5 ms, 5 ms'ye kadar indirilebilir — gerçek zamanlı için
  tasarlanmış tek yaygın codec.
- Rust tarafında `audiopus`/`opus` crate'leri ile, tarayıcı tarafında **WebCodecs
  `AudioDecoder` ile native** çözülür. **Tek codec, iki istemci.** Bu, PWA ve TUI
  istemcilerinin ayrışmasını engeller.
- snapcast'in de düşük gecikme seçeneği Opus'tur.

48 kHz stereo, 20 ms çerçeve, 64–128 kbps. Bu bitrate SSH tek-kanal fallback'inde bile rahat
sığar — bu yüzden **F1 fazı yalnız sesle başlar** (§6).

**Video: iki ayrı problem, iki ayrı cevap.**

| Kaynak | Doğru davranış | Gerekçe |
|---|---|---|
| Pane'deki TUI'nin kitty/sixel çıktısı | Passthrough, ama **bayat kare düşürme** ile | Piksel-kesin olmalı; yeniden kodlama metni bulanıklaştırır |
| Gerçek video (medya oynatıcı pane'i) | Sunucuda **H.264/VP9**, istemcide çöz → istemcide kitty'ye çiz | Bant genişliği kazancı burada; waypipe aynı ayrımı yapıyor |

İkinci satır tasarımın en büyük kazancı. Kullanıcının daha önce hesapladığı tablo:
24 fps kitty passthrough ≈ 60–120 Mbps (*hesap, ölçüm değil*). Aynı içerik H.264'te
2–8 Mbps'dir. **İstemci tarafında yeniden terminale çizmek**, herdr'ın "gerçek terminal
görünümleri, yorumlanmış değil" ilkesini bozmadan bu kazancı verir.

waypipe'ın bilinen tuzağı da not: her tampon için ayrı video akışı tutmak, yüzeyler tamponlar
arasında döndükçe **titremeye** yol açıyor. Ders: akış **yüzeye** bağlanır, tampona değil —
`stream_id` pane'e bağlıdır, kareye değil.

```rust
pub enum MediaCodec {
    OpusV1,      // ses
    RawPcmS16,   // ses, yerel/hata ayıklama
    KittyRaw,    // video, passthrough
    Png,         // video, düşük kare hızlı
    H264,        // video, yüksek kare hızlı
    Vp9,
}
```

---

### L4 — Saat ve senkron

Bu katman **doğrudan snapcast'ten alınır**, çünkü problem birebir aynı ve snapcast'in
ölçülmüş sonucu var (sapma tipik olarak < 0.2 ms).

Mekanizma üç parça:

**1. Sunucu zaman damgası.** Her `MediaChunk` sunucu-yerel saatte `pts_us` taşır.

**2. Sürekli saat offset tahmini.** İstemci düzenli olarak ping atar; iki mesaj eklenir:

```rust
ClientMessage::TimeSync   { client_send_us: u64 }
ServerMessage::TimeSyncReply { client_send_us: u64, server_recv_us: u64, server_send_us: u64 }
```

İstemci bunlardan offset ve RTT çıkarır, medyan filtreyle gürültüyü atar. (NTP'nin dört-zaman
damgası deseni; burada yeniden icat edilmez.)

**3. Sürüklenme düzeltmesi.** İstemci kristali sunucununkiyle asla tam eşleşmez. snapcast'in
çözümü doğrudan uygulanabilir: **tek örnek ekleyip çıkararak** hızlı/yavaş çalmak
(48 kHz'de bir örnek ≈ 0.02 ms — duyulmaz).

**Oynatma kuralı:** parça `pts_us + target_latency_us` anında çalınır, *geldiği anda* değil.
Erken gelen bekler, geç gelen atılır. Jitter buffer budur ve **A/V senkron bu kuraldan bedava
çıkar**: ses ve video aynı `pts` uzayını paylaştığı için ayrıca hizalanmaları gerekmez.

---

### L5 — Kontrol, öncelik ve geri-basınç

**Bugün:** `client_transport.rs` tek writer kuyruğu. Öncelik kavramı yok.

**Üç kural:**

**1. Kesin öncelik.** Kontrol çerçeveleri (`Input`, `Resize`, `Frame`, `Terminal`) medya
çerçevelerinin **önüne geçer**, her zaman. §3'ün regresyon kapısı budur.

**2. Son kullanma tarihi — kuyrukta değil, kaynakta düşür.** Bir medya parçasının `pts`'i
geçmişte kaldıysa gönderilmez. Yavaş bir bağlantıda kuyruk büyümez, **içerik seyrelir.**
Bu, ağ sıkıştığında gecikmenin sonsuz büyümesini (bufferbloat) yapısal olarak imkânsız kılar.

**3. Kredi tabanlı akış kontrolü.** İstemci tampon derinliğini ilan eder, sunucu aşmaz:

```rust
ClientMessage::MediaCredit { stream_id: u32, chunks: u16 }
```

**Bozulma sırası** (sıkışma arttıkça, bu sırayla):
video bitrate düşür → video kare hızı düşür → videoyu tamamen düşür → sesin bitrate'ini düşür
→ (asla) sesi düşür. Ses en son gider, çünkü algısal maliyeti en yüksek olan odur.

---

### L6 — Kaynak: ses nereden geliyor?

Kolayca gözden kaçan ama tasarımı belirleyen soru. herdr, pane'de koşan keyfi bir programın
(NoctaVox, mpv, sparkplayer) sesini **sihirle yakalayamaz**.

İki yol var ve seçim protokolün temizliğini belirler:

| Yol | Artı | Eksi |
|---|---|---|
| **(a) Pane süreci API ile opt-in eder** | Temiz, platform-bağımsız, `pane_graphics_stream` deseninin aynısı | Uygulamanın iş birliği gerekir |
| **(b) Sunucuda sistem sesi yakalama** (PipeWire monitor) | Her uygulamayla çalışır | Linux'a özel, her şeyi yakalar, protokole platform sızdırır |

**Karar: (a) protokol katmanının cevabıdır, (b) bir plugin'dir.**

Yani `pane_audio_stream` API endpoint'i eklenir (mevcut `pane_graphics_stream`'in tam
kardeşi: JSON başlık + parçalı gövde). PipeWire yakalama ise herdr'ın **plugin sistemine**
yazılır ve topladığı sesi bu endpoint'e besler.

Bu ayrım kritik: **protokol PipeWire'ı bilmemeli.** Bilirse macOS ve Windows'ta anlamsız bir
mesaj tipi taşır ve her yeni ses altyapısı protokol değişikliği gerektirir.

---

## 5. İstemci tarafı — iki istemci, iki gerçeklik

### 5.1 TUI istemcisi (Rust, Mac'te native)

Ses sink'i gerekiyor. `sound.rs`'in "Rust ses bağımlılığı yok" kararı burada bilinçli olarak
ve **sınırlı biçimde** geri alınır:

```toml
[features]
media-sink = ["dep:cpal", "dep:audiopus"]
```

- `media-sink` **varsayılan değildir.** Dağıtılan ikili değişmez; isteyen derler ya da
  medya-etkin ikiliyi indirir.
- Flag kapalıyken fallback: parçalar bir dış sürece (`mpv --no-video -`) stdin'den beslenir.
  Gecikme kontrolü zayıftır ama çalışır ve hiçbir bağımlılık eklemez.
- Linux'ta `media-sink` ALSA geliştirme başlıkları ister — sparkplayer'ın README'sinde
  görüldüğü gibi bu gerçek bir derleme yüküdür. Feature flag tam da bunu izole etmek için.

Video: `image` crate zaten var (PNG/JPEG/WebP). H.264 çözme yeni bir bağımlılıktır ve
**F3'e ertelenir** — F1/F2 onsuz tam değer üretir.

### 5.2 PWA istemcisi (tarayıcı)

Tarayıcı burada **daha kolay istemci**, ve bu şaşırtıcı ama doğru:

- Ses: `AudioDecoder` (WebCodecs, Opus native) → `AudioWorklet` içinde halka tamponu.
  Araştırma bunun doğru yol olduğunu doğruluyor: çözülmüş `AudioData` hoparlöre ancak
  AudioWorklet üzerinden, örnek-hassas bir halka tamponuyla ulaşır.
- Video: `VideoDecoder` → canvas. Terminale çizmeye gerek yok; PWA zaten web'in dilinde
  çiziyor (*"Chrome uyarlanır, ajan uyarlanmaz"* ilkesi bunu zaten söylüyor).
- Taşıma: mevcut Bun WebSocket köprüsü ikili çerçeve taşıyabilir; QUIC gerekmez.
  WebTransport ileride değerlendirilebilir ama **F4 için gerekli değildir**.

Kritik tasarım kazancı: **`pts_us` + Opus seçimi sayesinde iki istemci aynı akışı tüketir.**
Sunucu tarafında ikinci bir kodlama yolu yoktur.

---

## 6. Faz planı — her faz tek başına değer üretir

| Faz | İçerik | Tek başına ne kazandırır | Doğrulama (V) |
|---|---|---|---|
| **F0** | `Capability` anlaşması (L1) | Medya yok; ama sonraki her şeyi kırılmadan çıkarılabilir yapar | Eski istemci ↔ yeni sunucu: davranış bit-birebir aynı |
| **F1** | Opus ses akışı + saat senkronu (L2/L3/L4), **mevcut SSH kanalı** | Uzak pane'in sesi Mac'ten çıkar. Asıl özellik bu | 60 dk kesintisiz çalma, 0 underrun; sürüklenme < 1 ms/saat |
| **F2** | QUIC-over-Tailscale yan kanal (L0) + kredi/öncelik (L5) | Medya akarken arayüz yavaşlamıyor | Medya akarken tuş yankısı gecikmesi F1-öncesi taban ile aynı (±5 ms) |
| **F3** | Video akışı + istemci-tarafı transcode-to-kitty (L3) | Uzaktan gerçek video | 24 fps @ < 8 Mbps; A/V \|Δ\| < 40 ms |
| **F4** | PWA pariteti (WebCodecs) | Telefondan ses/video | Aynı akış, iki istemci, ikinci kodlama yolu yok |

**F1 tek başına, kullanıcının sorduğu problemin çoğunu çözer** ve QUIC gerektirmez —
Opus'un 64–128 kbps'i mevcut SSH kanalına rahat sığar. Bu, faz sırasının en önemli özelliği:
en büyük değer, en küçük mimari riskle önde geliyor.

---

## 7. Bu tasarımın *yapmadığı* şeyler

Kapsam dürüstlüğü:

- **Sistem sesini otomatik yakalamaz.** Pane süreci opt-in eder ya da bir plugin besler (§L6).
  "herdr'ı aç, NoctaVox'un sesi Mac'ten gelsin" F1'de *kendiliğinden* olmaz — plugin gerekir.
- **X11/Wayland penceresi taşımaz.** mpv'nin GUI penceresi hâlâ sunucuda kalır. herdr terminal
  yüzeyidir; pencere taşıma waypipe'ın işidir ve ayrı bir problemdir.
- **Genel amaçlı bir medya sunucusu değildir.** Hedef, pane'e bağlı akışlardır.
- **"Kusursuz" mutlak değil, §3'teki ölçülebilir eşiklerdir.** Kayıplı bir ağda mutlak
  kusursuzluk fiziksel olarak mümkün değildir; tasarım bozulmayı *yönetir*, yok saymaz.

---

## 7.5 ÖLÇÜMLER (2026-08-12, bu makinede)

Aşağıdakiler tahmin değil, ölçümdür. Yöntem: gerçek ekran görüntüsü (`Screenshot-30.png`,
5760×2160) ölçeklenip kodlandı; ffmpeg 8.1.2, libx264/libopus.

### Sıkıştırma katmanı — nerede işe yarar, nerede yaramaz

| Yük | Ham | gzip-6 | zstd-3 | zstd-19 |
|---|---|---|---|---|
| Terminal metin çerçevesi (200×50, kod) | 11 041 B | 282 B (**39×**) | 272 B (41×) | 214 B (52×) |
| En kötü durum renkli hücreler (200×50 RGB) | 384 619 B | 84 109 B (**4.6×**) | 92 530 B (4.2×) | 68 481 B (5.6×) |
| PNG 720p | 323 169 B | 318 644 B (**1.01×**) | 323 191 B (1.00×) | — |
| base64(PNG 720p) | 430 892 B | 324 239 B (**1.33×**) | 322 544 B (1.34×) | — |

**Sonuç (RM1):** gzip/zstd **metin için hayat kurtarır (39–52×), medya için hiçbir şey
yapmaz (1.01×)**. base64'lü görüntüde görülen 1.33× tam olarak base64'ün 4/3 şişmesini geri
almaktır — bir bayt bile fazlası değil. **Medya yolunda sıkıştırma katmanı çözüm değildir;
codec katmanı çözümdür.**

### Kare başına maliyet (gerçek ekran görüntüsü)

| Çözünürlük | Ham RGB | PNG | PNG+b64 | JPEG-80 | JPEG-80+b64 |
|---|---|---|---|---|---|
| 360p | 675 KB | 104.0 KB | 138.7 KB | 27.8 KB | 37.1 KB |
| 720p | 2 700 KB | 315.6 KB | 420.8 KB | 86.8 KB | 115.8 KB |
| 1080p | 6 075 KB | 572.0 KB | 762.7 KB | 175.5 KB | 234.0 KB |

24 fps kitty passthrough karşılığı: 360p **26.6 Mbps** · 720p **80.8 Mbps** · 1080p **146 Mbps**.
Bu, §L3'teki "60–120 Mbps" tahminini doğrular ve **ölçülmüş hâle getirir** (açık madde 2 kapandı).

### H.264 gerçek kodlama (24 fps, 10 sn, libx264 veryfast)

| İçerik | Çözünürlük | Bitrate | MB/dakika |
|---|---|---|---|
| TUI / kayan terminal (`-tune stillimage`, crf 26) | 720p | **0.04 Mbps** | 0.3 |
| TUI / kayan terminal | 1080p | **0.08 Mbps** | 0.5 |
| Tam hareketli video (crf 23) | 720p | **2.50 Mbps** | 17.9 |
| Tam hareketli video | 1080p | **5.06 Mbps** | 36.2 |

**Sonuç (RM2):** 1080p'de kitty passthrough (146 Mbps) ile H.264 (5.06 Mbps) arasında
**29× fark** var. Ekran içeriğinde fark **1800×**'e çıkıyor. §L3'ün istemci-tarafı
transcode kararının tüm gerekçesi bu tablodur.

### Opus gerçek kodlama (48 kHz stereo, 60 sn)

| Bitrate | MB/dakika | 100 MB kaç dakika ses |
|---|---|---|
| 32 kbps | 0.19 | **540 dk** |
| 64 kbps | 0.34 | **294 dk** |
| 96 kbps | 0.47 | 213 dk |
| 128 kbps | 0.64 | **157 dk** |
| Ham PCM s16 | 10.99 | 9 dk |

**Sonuç (RM3):** Opus 64k, ham PCM'in **1/32'si**. F1'in mevcut SSH kanalına sığacağı
iddiası doğrulandı: 64 kbps = **5.7 KB/s**, herhangi bir bağlantıda ihmal edilebilir.

### 100 MB neye yeter — tek tabloda

| Yük | 100 MB ile süre |
|---|---|
| Kitty PNG passthrough 1080p @24fps | **5.5 saniye** |
| H.264 1080p tam hareket | 2.8 dakika |
| H.264 720p tam hareket | 5.6 dakika |
| H.264 1080p ekran içeriği | 200 dakika |
| Opus 128k ses | 157 dakika |
| Opus 64k ses | 294 dakika |

### Taşıma tavanı — ölçülemeyen kısım

MacBook ölçüm anında çevrimdışıydı (`tailscale status`: son görülme 14 sa önce) ve **son
bağlantısı `relay "fra"` üzerindendi — doğrudan değil.** Bu tasarımın en büyük tek değişkeni:
DERP relay'leri paylaşımlı, QoS sınırlı ve doğrudan bağlantıdan belirgin biçimde yavaştır
([Tailscale docs](https://tailscale.com/docs/reference/connection-types)).

**Açık madde:** gerçek Mac↔Linux throughput ölçülmedi. Ölçüm komutu:
`tailscale ping macbooks-macbook-pro` (direct mi relay mi) + `tailscale status` satırındaki
`direct`/`relay` etiketi. Relay çıkarsa **protokol tasarımından bağımsız olarak** medya
bütçesi çöker — önce bu düzeltilir.

---

## 7.6 Video endüstrisinden transfer — Jellyfin/YouTube/Netflix analizi

Soru: "az trafik + küçük paketle yüksek kalite kesintisiz akış" nasıl yapılıyor, hangisi bize uyar?

### 7.6.1 Önce sınıflandırma — kopyalanacak sistemi doğru seç

En sık yapılan hata, gecikme sınıfı farklı bir sistemden trick kopyalamaktır. Üç ayrı sınıf var
ve **trick'ler birbirinin yerine geçmez**:

| Sınıf | Uçtan uca gecikme | İstemci buffer | Örnek | Baskın strateji |
|---|---|---|---|---|
| **A. VOD / yayın** | 6–30 sn | 10–30 sn | YouTube, Netflix, standart HLS | Önceden kodlanmış ABR merdiveni + CDN + **büyük buffer** |
| **B. Düşük gecikmeli canlı** | 1–3 sn | 1–3 sn | LL-HLS, CMAF, MoQ | Kısmi segment (200–400 ms chunk) |
| **C. İnteraktif** | < 100 ms | 1–3 kare | **Moonlight/Sunshine, RDP, WebRTC** | Gerçek-zamanlı encoder rate control + FEC + damage tracking |

**herdr C sınıfındadır.** Pane'e tuş basıp yankısını beklersin; 10 saniyelik buffer kullanılamaz.

Bunun doğrudan sonucu: **Jellyfin ve YouTube'un en meşhur trick'lerinin çoğu bize UYMAZ.**
Kopyalanacak asıl prior-art oyun akışı (Sunshine/Moonlight) ve uzak masaüstüdür (RDP/RemoteFX) —
çünkü onlar da ekran içeriğini, interaktif olarak, tek izleyiciye taşır. Bu, herdr'ın problemiyle
birebir aynı şekildir.

### 7.6.2 Transfer EDEN trick'ler

**T1 — Damage tracking / kirli dikdörtgen (RDP).** Sadece *değişeni* gönder. Video codec'lerindeki
P-frame'in ekran karşılığı.
→ **herdr'da ZATEN VAR:** `RenderEncoding::TerminalAnsi` = *"already-diffed terminal ANSI byte
streams"*. Bu bir P-frame'dir, adı öyle konmamış sadece. En büyük kazanç zaten cepte.

**T2 — Direct Play (Jellyfin).** İstemci orijinali çözebiliyorsa **hiç dokunma**. Karar,
istemcinin ilan ettiği *device profile* ile medyanın özelliklerinin eşleştirilmesinden çıkar —
tahminle değil. Üç kademe: Direct Play (dokunma) → Remux (yalnız kapsayıcı değiştir) →
Transcode (yeniden kodla, en pahalısı).
→ **herdr karşılığı:** Ghostty kitty graphics çözebiliyorsa passthrough (Direct Play);
çözemiyorsa sunucuda halfblocks'a indir (Transcode). §L1'de önerdiğim `Capability` anlaşması
tam olarak Jellyfin'in device profile'ıdır. **Kritik ders: karar veri-güdümlü olmalı, sezgisel değil.**

**T3 — İçerik sınıfı farkındalığı (HEVC-SCC / AV1 palette + intra block copy).** Ekran içeriği
doğal videodan yapısal olarak farklıdır: sınırlı renk paleti, birebir tekrar eden bloklar, keskin
kenarlar. Modern codec'lerin hepsinde (HEVC-SCC, VVC, AV1) buna özel araçlar var.
→ **Ölçümüm bunu doğruluyor: 0.08 Mbps (TUI) ↔ 5.06 Mbps (hareket) = 60×.** herdr pane'i
sınıflandırmalı (metin / görsel / video) ve codec parametresini ona göre seçmeli. Sabit
parametre, iki durumdan birinde mutlaka yanlıştır.

**T4 — İlerlemeli iyileştirme (RemoteFX Progressive).** Önce düşük çözünürlüklü kareyi gönder,
sonraki karelerde keskinleştir. Dar bantta masaüstü *hızlı* görünür.
→ herdr'da yeni odaklanan pane 20 ms'de bulanık görünür, sonra netleşir. Algısal olarak
"boş bekleme"den kat kat iyidir.

**T5 — Değişken kare hızı (Sunshine).** Sabit fps yerine **içeriğin değişme hızına** uy.
→ Terminal içeriği patlamalı değişir: durgun bir pane **0 bps** etmeli. Sabit 24 fps göndermek
herdr için saf israftır. Bu, T1 ile birleşince en büyük ikinci kazanç.

**T6 — Buffer tabanlı adaptasyon (BOLA, dash.js varsayılanı).** Bant genişliğini **tahmin etme** —
buffer doluluğuna bak. Lyapunov optimizasyonu ile kalite/donma dengesini kurar ve bant tahmini
gerektirmemesi asıl üstünlüğüdür.
→ herdr'ın jitter buffer'ı (§L4) zaten bu sinyali üretiyor. Buffer boşalıyorsa kaliteyi düşür.
Fark: bizim buffer saniyeler değil **milisaniyeler** — algoritma aynı, sabitler farklı.

**T7 — FEC (Sunshine/Moonlight).** Kayıpta yeniden iletim isteme, **fazladan parite gönder**.
→ §L0'da ses için QUIC *datagram* seçtim (yeniden iletim yok). Datagram + FEC birlikte gider;
tek başına datagram kayıpta çıtırtı demektir.

**T8 — Sunucu, istemcinin tüketim hızına göre kısılır (RDP).** Sunucu ekran güncelleme üretimini
istemcinin tüketebildiği hıza *throttle* eder.
→ §L5'teki kredi tabanlı akış kontrolünün ta kendisi. Bağımsız bir kaynaktan doğrulanmış oldu.

### 7.6.3 Transfer ETMEYEN trick'ler — ve neden

Bunları kopyalamak zaman kaybı olur; listelenmesi listelenmemesi kadar değerli:

| Trick | Neden uymaz |
|---|---|
| **CDN / edge cache** | herdr noktadan noktaya, **tek izleyici**. Fan-out problemi yok |
| **Önceden kodlanmış ABR merdiveni** | Canlı akışta önceden kodlanacak içerik yok. Yerine **gerçek-zamanlı encoder rate control** (WebRTC modeli) |
| **Netflix Dynamic Optimizer (per-shot convex hull)** | Tüm başlığı önceden analiz eden **çevrimdışı** yöntem. VMAF ile %27–37 kazandırıyor ama canlıda uygulanamaz. *İlkesi* (içerik-uyarlamalı kodlama) T3 olarak alınır |
| **Büyük buffer (10–30 sn)** | A sınıfının çözümü. herdr'da **zararlı** — interaktifliği öldürür |
| **Manifest / playlist modeli** | Canlı terminalde seek yok, segment listesi anlamsız |

### 7.6.4 herdr katmanlarına eşleme

| herdr katmanı | Uygulanacak trick | Durum |
|---|---|---|
| L0 Taşıma | T7 (FEC), BBR (QUIC ile gelir) | Yeni |
| L1 Yetenek | **T2 (Direct Play / device profile)** | Zaten planlandı — şimdi gerekçesi güçlendi |
| L2 Akış | T5 (değişken kare hızı) | Yeni |
| L3 Codec | **T3 (içerik sınıfı)**, T4 (ilerlemeli) | Kısmen planlandı |
| L4 Saat | T6 (buffer tabanlı adaptasyon) | Jitter buffer'dan bedava |
| L5 Kontrol | **T8 (tüketim hızına kısma)** | Zaten planlandı — doğrulandı |
| L6 Kaynak | **T1 (damage tracking)** | ✅ **ZATEN VAR** (`TerminalAnsi`) |

**Sonuç (RM4):** Yedi katmandan üçünde plan zaten doğruydu ve bağımsız kaynakla doğrulandı;
üçüne yeni trick eklendi (T5, T4, T7); birinde (L6/T1) **herdr endüstriyi zaten uyguluyor.**

**Sonuç (RM5) — en yüksek kaldıraçlı iki iş:** T5 (değişken kare hızı — durgun pane 0 bps) ve
T3 (içerik sınıfı — 60× fark). İkisi de codec/akış katmanında, ikisi de QUIC gerektirmiyor,
ikisi de F1–F2 içinde yapılabilir.

---

## 7.7 YÜZEY ÇALIŞMASI — herdr-browser plugin'i (canlı varoluş kanıtı)

Bu bölüm teorik değil: ekosistemde **bu tasarımın müşterisi zaten var ve tam öngörülen yerde
duvara toslamış.**

Atlas taraması (`atlas search --eco herdr-plugins`, 591 kayıt, kaynak 2026-08-03) aynı adlı
üç repo buldu:

| Repo | Skor | Yıldız | Not |
|---|---|---|---|
| `ogulcancelik/herdr-browser` | 0.636 | **★246** [active] | **Kanonik** — herdr'ın kendi yazarı |
| `StructuPath/herdr-browser` | 0.639 | ★— [active] | Bağımsız proje (fork değil), Vercel agent-browser tabanlı |
| `Epsomsaltskerosinelamp950/herdr-browser` | 0.636 | ★0 | ⚠ oto-üretilmiş görünen hesap adı — **slop şüphesi, vet edilmeden dokunma** |

> ⚠ Araç notu: `atlas why herdr-browser` ad çakışmasında yanlış repoyu (★0 olanı) getirdi.
> Aynı adlı çoklu kayıtta `why` ile repo seçimi güvenilir değil, tam URL ile doğrula.

### 7.7.1 Kanonik plugin'in mimarisi

*"Frames come from CDP screencast and reach the terminal through Herdr's **pane graphics
stream**."* — yani §1.2'de envanterlediğim ingest yolu **üretimde kullanılıyor**.

| Mekanizma | Ne yapıyor | Hangi trick |
|---|---|---|
| `Page.screencastFrameAck`'i **geciktirerek** geri-basınç | Kare kodlandıktan sonra atmak yerine **hiç ürettirmiyor** | **T8**, üstelik benim tasarımımdan daha iyi yerde |
| İki kademeli tempo: durgun sayfa **15 fps**, etkileşimden sonra **750 ms boyunca 30 fps** | Kare hızını olaya bağlıyor | **T5, kısmi** (etkileşim-güdümlü, içerik-güdümlü değil) |
| `captureScale` (0.1–1.0), `screencastEveryNthFrame`, `screencastPollMs` | Kalite merdiveni | **Manuel** — uyarlanabilir değil (T6 yok) |
| HiDPI'de her piksel iki kez ödeniyor | Hem Chromium encoder hem terminal decode | Maliyet notu |

### 7.7.2 Kritik bulgu — kendi README'si duvarı belgeliyor

> Uzaktan SSH kullanımı pratik değil, çünkü **kare başına bant genişliği çok yüksek**.

Bu, bu belgenin §2 Tavan 1'inin **bağımsız, üretim-içi doğrulamasıdır**. Teorik iddia değil;
herdr'ın yazarı kendi plugin'inde bu sınırı yaşamış ve yazmış.

**Sayıyla (§7.5 ölçümlerimle birleştirince):**

| Durum | Hesap | Sonuç |
|---|---|---|
| 720p JPEG-80 kare | 86.8 KB (ölçüldü) | — |
| Durgun sayfa @15 fps | 86.8 KB × 15 | **10.4 Mbps** |
| Etkileşim @30 fps | 86.8 KB × 30 | **20.8 Mbps** |
| Aynı içerik H.264 ekran-içeriği | ölçüldü | **0.04 Mbps** |
| **Fark** | | **≈260×** |

**Sonuç (RM6):** Durgun bir sayfada plugin saniyede **15 adet birbirinin aynı kareyi**
gönderiyor. Temporal öngörü (T1) veya içerik-sınıfı codec'i (T3) bunu neredeyse sıfıra
indirirdi. Ödenen bedel 260× ve tamamı L3 (codec) katmanının yokluğundan.

### 7.7.3 Kaynak katmanının sert sınırı

CDP `Page.startScreencast` doğrulandı: parametreler `format` (jpeg|png), `quality`,
`maxWidth/maxHeight`, `everyNthFrame`; teslimat **tam kare**, `screencastFrameAck` ile onay.
**Kirli dikdörtgen / delta kare desteği YOK.**

**Sonuç (RM7):** T1 (damage tracking) bu kaynakta **uygulanamaz** — CDP delta vermiyor.
Bu yüzden herdr-browser için tek gerçek kurtarıcı L3'tür: gelen tam kareleri **sunucuda
temporal codec'e sokmak**. Kaynağı değiştirmek gerekmiyor, araya codec koymak gerekiyor.
Bu, T1'in her yerde mümkün olmadığını, T3'ün ise mümkün olduğunu gösteren somut vaka.

### 7.7.4 StructuPath varyantı — T2'yi bağımsız olarak yeniden keşfetmiş

Ayrı bir proje (fork değil) ve şu üç kademeli render yolunu kuruyor:

```
kitty graphics (varsa)  →  chafa ile ANSI sembol  →  yalnız-metin son çare
```

Terminal yeteneğini **başlangıçta yokluyor** ve en güçlü modu otomatik seçiyor.

**Sonuç (RM8):** Bu, Jellyfin'in **Direct Play → Remux → Transcode** merdiveninin birebir
karşılığıdır ve ekosistemde **kendiliğinden ortaya çıkmıştır**. §L1'de önerdiğim `Capability`
anlaşmasının doğru soyutlama olduğunun en güçlü kanıtı: iki bağımsız geliştirici aynı
merdiveni ayrı ayrı icat etmişse, o merdiven protokole ait demektir — her plugin'in yeniden
yazması gereken bir şeye değil.

Ayrıca: WebSocket ile kare teslimi ve **yalnız sayfa içeriği değiştiğinde gönderme** —
kanonik plugin'in etkileşim-güdümlü temposundan daha doğru bir T5 uygulaması.

### 7.7.5 Katman eşlemesi — iki plugin arasında ne var, ne yok

| Katman | Kanonik | StructuPath | Protokole ait olan |
|---|---|---|---|
| L6 Kaynak | CDP tam kare | CDP + değişim-güdümlü | — (T1 imkânsız, RM7) |
| L5 Kontrol | ✅ T8 (ack geciktirme) | polling fallback | T8'i **protokole** taşı |
| L4 Saat | ❌ pts yok, jitter yok | ❌ | **T6 — protokol** |
| L3 Codec | ❌ kare başına JPEG | ❌ JPEG + chafa | **T3/T4 — protokol** |
| L2 Akış | pane_graphics_stream | WebSocket | akış kimliği |
| L1 Yetenek | ❌ manuel config | ✅ 3 kademeli probe | **T2 — protokol** |
| L0 Taşıma | ❌ **SSH'te kırık** | ❌ aynı | **ana blocker** |

**Sonuç (RM9) — stratejik:** herdr-browser, medya taşıma işinin **ilk ve en değerli
müşterisidir**. F1–F3 indiğinde, README'sinin bugün "pratik değil" dediği şey çalışır hâle
gelir. Dahası bu vaka, hangi işin plugin'e hangisinin protokole ait olduğunu ayırıyor:
**iki plugin de L1, L3, L4'ü ayrı ayrı çözmeye çalışıp yarım bırakmış** — çünkü bunlar
plugin katmanının işi değil.

---

## 7.8 PROFİL TAKSONOMİSİ — §7.6'nın düzeltmesi

**§7.6.1'de yapılan hata:** "herdr C sınıfıdır (interaktif, <100 ms)" denip **tüm medya yolu**
o sınıfa göre tasarlandı.

**Doğrusu: gecikme sınıfı herdr'ın değil, AKIŞIN özelliğidir.** Film izleyen bir pane interaktif
değildir; 5 saniye buffer'lanabilir ve buffer'landığı anda A sınıfının bütün kolaylıkları geri
gelir. Film ile ajan-izleyen-browser'ı aynı boruya koymak, ikisini birden en zor duruma göre
tasarlamak demekti — gereksiz pahalı.

| Profil | Örnek | Sınıf | Buffer | Codec | A/V senkron | Ayrı taşıma gerekli mi |
|---|---|---|---|---|---|---|
| **P1** Ses dinleme | müzik | B | 2–10 sn | Opus 64–128k | — | ❌ |
| **P2** Film | video+ses | **A** | 2–10 sn | H.264 + Opus | ✅ zorunlu | ❌ |
| **P3** İnteraktif görsel | browser sürükleme, ajan izleme | **C** | 1–3 kare | ekran-içeriği codec | — | ✅ |
| **P4** Anlık görsel | kapak resmi, tek screenshot | — | yok | tek atış PNG | — | ❌ (bugün çalışıyor) |
| **P5** Olay sesi | bildirim | — | yok | gömülü dosya | — | ❌ (bugün çalışıyor) |

**Sonuç (RM12):** Ayrı taşıma (QUIC) **yalnız P3'ün** ihtiyacıdır. P1 ve P2 mevcut SSH
kanalında, büyük buffer'la çalışır — ölçümler destekliyor: Opus 64k = 5.7 KB/s,
H.264 1080p film = 5.06 Mbps.

**Ama öncelik (L5) her profil için gerekli.** Film 5 sn buffer'lı olsa bile medya kontrol
akışını paylaşırsa tuş yankısı gecikir. Ayrı *taşıma* P3'ün derdi; ayrı *öncelik* herkesin.

---

## 7.9 ÇÖZÜM DEĞERLENDİRME — kapsama (subsumption) analizi

### 7.9.1 Değerlendirme kuralı

Kullanıcı tarafından formüle edilen ve buradan itibaren kanonik olan karar kuralı:

```
1. Bir çözüm birden fazla case'i çözüyorsa → değerlidir.
2. Yeni bir çözüm, öncekilerin çözümünü BOZMUYOR ve üstelik GEREKSİZ kılıyorsa
   → tek ve yegâne çözüm odur, diğerleri tartışmadan düşer.
3. Kapsama yoksa → case/state bazlı pros-cons tablosu ZORUNLU; "en iyi" tekil cevap yoktur.
```

Bu, mimari kararı zevk meselesi olmaktan çıkarır: kapsama ilişkisi **kanıtlanır ya da
kanıtlanamaz**.

### 7.9.2 Aday çözümler

| # | Çözüm | Ne yapar |
|---|---|---|
| **S1** | **Veri yer değiştirme** | MPD httpd, dosya paylaşımı (SMB), port tüneli — medya değil *kaynak* paylaşılır |
| **S2** | **İşi client'a taşıma** | Uygulama Mac'te koşar, sunucu yalnız veriyi servis eder |
| **S3** | Opus ses akışı, mevcut SSH kanalı | F1 |
| **S4** | Büyük buffer + H.264, mevcut SSH kanalı | F2' (P2 için) |
| **S5** | QUIC yan kanal + tam medya yığını | F2–F3 |
| **S6** | Capability anlaşması | F0 |
| **S7** | Öncelik + geri-basınç | L5 |

### 7.9.3 Kapsama matrisi

`✅` çözer · `➖` uygulanamaz · `❌` çözmez

| | C1 müzik | C2 film | C3 browser-ajan | C4 browser-preview | C5 browser-interaktif | C6 bildirim | C7 anlık görsel |
|---|---|---|---|---|---|---|---|
| **S1** veri taşı | ✅ | ✅ | ➖ | ✅ | ➖ | ➖ | ➖ |
| **S2** client'a taşı | ✅ | ✅ | ✅ | ✅ | ✅ | ➖ | ➖ |
| **S3** Opus/SSH | ✅ | ❌ | ➖ | ➖ | ❌ | ✅ | ➖ |
| **S4** buffer+H.264/SSH | ✅ | ✅ | ❌ | ➖ | ❌ | ✅ | ✅ |
| **S5** QUIC tam yığın | ✅ | ✅ | ✅ | ➖ | ✅ | ✅ | ✅ |
| **S6** capability | ➖ | ➖ | ➖ | ➖ | ➖ | ➖ | ✅ |
| **S7** öncelik | ➖ | ➖ | ➖ | ➖ | ➖ | ➖ | ➖ |

### 7.9.4 Kapsama sonuçları — kanıtlanan ve kanıtlanamayan

**K1 — S5, S3 ve S4'ü KAPSAR.** S5 kurulduğunda S3/S4 ayrıca yaşamaz.
⚠ Ama bu onları *rakip* yapmaz: **S3 → S4 → S5, S5'in inşa AŞAMALARIDIR.** Codec, saat senkronu,
jitter buffer, akış mesajları aynen taşınır; yalnız *taşıma bağlaması* değişir.
**S3'te yazılan kodun %100'ü S5 altında geçerli kalır** → artımlı yol güvenli, çöp iş yok.

**K2 — S2, uygulanabildiği her yerde HERKESİ kapsar ve S5 bunu ASLA yenemez.**
NoctaVox Mac'te koşarsa ses bit-birebir, gecikme sıfır, codec kaybı yok. Hiçbir taşıma çözümü
"hiç taşımamak"tan iyi olamaz. → **S5, S2'yi gereksiz KILMAZ.**

**K3 — Dolayısıyla tek ve yegâne çözüm YOKTUR; iki kademeli bir hakimiyet vardır:**

```
İş yer değiştirebiliyor mu?
├─ EVET → S1/S2 (veri veya işi taşı)   ← bu dalda TEK ve YEGÂNE, tartışma kapanır
└─ HAYIR → S5 (QUIC yığını)            ← bu dalda TEK ve YEGÂNE, S3/S4 onun aşamaları
```

**K4 — S6 ve S7 rakip değil, ORTOGONAL ZORUNLULUK.** Hiçbir case'i tek başına çözmezler ama
S3/S4/S5'in hepsi onlara muhtaçtır. Matriste `➖` dolu olmaları "değersiz" demek değil,
"karşılaştırma ekseninde değil" demektir. → **Önce onlar yapılır.**

### 7.9.5 QUIC nedir — ve tüm sorunları çözer mi

**Ne olduğu:** TCP'nin yerine geçen bir taşıma protokolü. UDP üzerine kurulu, HTTP/3'ün altında
çalışan, IETF standardı. Dört özelliği bizi ilgilendiriyor:

| Özellik | Bize ne kazandırır |
|---|---|
| **Bağımsız akışlar** | Tek bağlantıda çok kanal; birinde kayıp/tıkanma diğerini BEKLETMEZ → Tavan 1 (head-of-line) yapısal olarak kalkar |
| **Datagram** | Güvenilmez mesaj gönderebilme → geç kalan ses paketi zaten işe yaramaz, yeniden iletimi beklemek kesintiyi UZATIR |
| **Gömülü şifreleme** (TLS 1.3) | Ayrı güvenlik katmanı gerekmez |
| **Bağlantı göçü** | IP değişse bile oturum yaşar → Mac wifi'dan hücresele geçtiğinde kopmaz |

**Tüm sorunları çözer mi? HAYIR — ve bu dürüst cevap.** QUIC yalnız **L0 (taşıma)** katmanının
sorunlarını çözer. Şunları çözmez:

- Codec seçimi (L3) — ham kare göndermeye devam edersen QUIC seni kurtarmaz, sadece 146 Mbps'i
  daha düzgün taşır
- Saat/senkron (L4) — `pts` yoksa QUIC'in bundan haberi olmaz
- Yetenek anlaşması (L1)
- **"Bu iş neden orada koşuyor" sorusu** — K2

**Maliyeti:** `quinn` bağımlılığı + herdr'ın `tokio`'su şu an `net` özelliği olmadan kullanılıyor
(`Cargo.toml:39`) → bloklamalı thread modeliyle gerçek mimari sürtünme. Açık madde 4.

**Karar:** QUIC, kendi katmanında tek ve yegâne doğru cevaptır. **Ama o katman yalnız P3 için
gereklidir** (RM12) ve P3, C4/C3'ün S2 ile çözülebildiği görüldükten sonra **en düşük öncelikli
case'tir.**

### 7.9.6 Revize faz planı

| Faz | Kapsam | Gerekçe |
|---|---|---|
| **F0** ✅ | S6 capability + S7 öncelik | Ortogonal zorunluluk (K4) — her şeyin önkoşulu · **TAMAMLANDI 2026-08-28** (`feat/media-capability`): yetenekler AD tabanlı, PROTOCOL_VERSION 21→22, TP-MEDIA-CAP-01/02/03 + TP-CLIENT-WRITE-PRIO-01 kayıtlı, `just check` 5998/5998. S7'nin kesin-öncelik yarısı ZATEN uygulanmıştı (`client_transport.rs` recv) — F0 onu adlandırıp merge'e karşı kilitledi; kredi/son-kullanma F1-F2'de. |
| **F1** | S3: P1 müzik (Opus, mevcut kanal) | En büyük değer, en küçük risk; S5'in 1. aşaması (K1) |
| **F2** | S4: P2 film (buffer + H.264 + A/V senkron) | Aynı kanal, QUIC yok; S5'in 2. aşaması |
| **F3** | *(ERTELENDİ)* S5 QUIC + P3 | Yalnız C5 için gerekli; C3/C4 S2 ile çözülü (K2) |
| **—** | S1/S2 yönlendirme rehberi | Kod değil **doküman** işi: hangi case hangi dala gider |

**Sonuç (RM13):** Önceki plana göre kapsam **ciddi biçimde küçüldü.** QUIC, video transcode ve
istemci-tarafı H.264 çözme F3'e — yani belirsiz geleceğe — ertelendi. F0–F2 tek başına
kullanıcının saydığı ilk iki case'i tam kapatıyor.

---

## 8. Referanslar

| Kaynak | Ne için kullanıldı | Tip | Güven |
|---|---|---|---|
| `src/protocol/wire.rs`, `src/remote/unix.rs`, `src/sound.rs`, `src/api/server/pane_graphics_stream.rs`, `Cargo.toml` | Mevcut durum envanteri (§1) | source_code | 0.95 |
| `herdr-web/src/server.ts`, `framing.ts` | PWA taşıma katmanı | source_code | 0.9 |
| [snapcast](https://github.com/badaix/snapcast) | Saat senkronu, sürüklenme düzeltmesi, codec seti (§L4) | official_docs | 0.9 |
| [Opus](https://en.wikipedia.org/wiki/Opus_(audio_format)) | Algoritmik gecikme 5–26.5 ms (§L3) | reference | 0.85 |
| [waypipe](https://gitlab.freedesktop.org/mstoeckl/waypipe) | Video kodlama kararı + tampon-titremesi tuzağı (§L3) | official_docs | 0.85 |
| [MoQ / Media over QUIC](https://www.wowza.com/blog/what-is-media-over-quic-moq-and-why-are-people-talking-about-it) | Güvenilir akış ↔ datagram ayrımı (§L0) | reference | 0.8 |
| [WezTerm multiplexing](https://wezterm.org/multiplexing.html) | unix/ssh/tls domain deseni; öngörülü yankı (§L0) | official_docs | 0.85 |
| [WebCodecs + AudioWorklet](https://developer.mozilla.org/en-US/docs/Web/API/AudioWorklet) | Tarayıcı ses yolu (§5.2) | official_docs | 0.85 |
| [sshx](https://github.com/ekzhang/sshx) | Öngörülü yankı, uçtan uca şifreleme deseni | source_code | 0.7 |
| [Jellyfin transcoding](https://jellyfin.org/docs/general/post-install/transcoding/) + [DLNA/stream selection](https://deepwiki.com/jellyfin/jellyfin/3.3-dlna-and-stream-selection) | T2 Direct Play / device profile (§7.6.2) | official_docs | 0.85 |
| [MS-RDPEGFX RemoteFX Progressive](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-rdpegfx/1dcd953d-672b-457e-9dec-a6fe639bae8f) | T4 ilerlemeli iyileştirme, T8 throttle, damage tracking | official_docs | 0.9 |
| [Moonlight/Sunshine](https://github.com/moonlight-stream/moonlight-docs/wiki/Frequently-Asked-Questions) | T5 değişken kare hızı, T7 FEC, C-sınıfı gecikme hedefi | official_docs | 0.8 |
| [BOLA (UMass)](https://people.cs.umass.edu/~ramesh/Site/HOME_files/Bola.pdf) | T6 buffer tabanlı adaptasyon, bant tahmini gerektirmez | spec | 0.9 |
| [HEVC SCC extension](https://www.merl.com/publications/docs/TR2015-126.pdf) | T3 ekran içeriği kodlama (palette, intra block copy) | spec | 0.9 |
| [Netflix Dynamic Optimizer](https://netflixtechblog.com/dynamic-optimizer-a-perceptual-video-encoding-optimization-framework-e19f1e3a277f) | Transfer ETMEYEN listesi gerekçesi (§7.6.3) | official_docs | 0.85 |
| [LL-HLS / CMAF chunked](https://www.mux.com/articles/low-latency-live-streaming-developers-guide-ll-hls-webrtc-cmaf) | B-sınıfı sınıflandırma, kısmi segment deseni | reference | 0.8 |

**Doğrulanmamış / açık kalanlar (V):**

1. `MAX_GRAPHICS_FRAME_SIZE = 32 MB` çerçevesinin SSH köprüsünde ölçülen kuyruk etkisi —
   Tavan 1 mantık olarak kesin, **sayısal olarak ölçülmedi**.
2. 24 fps kitty passthrough için 60–120 Mbps tahmini bir **hesaptır, ölçüm değildir**.
3. WezTerm mux protokolünün görsel ilettiği doğrulanamadı (dokümantasyon sessiz).
4. `quinn`'in herdr'ın mevcut bloklamalı (blocking) iş parçacığı modeline entegrasyon maliyeti
   incelenmedi — `tokio` şu an `net` özelliği olmadan kullanılıyor, bu gerçek bir mimari sürtünme.
5. macOS'ta `cpal` + herdr'ın mevcut sinyal/terminal yönetiminin etkileşimi test edilmedi.

---

*v1.0.0 — 2026-08-12 — Kaynak: "uzaktan bağlanmış cliente ses ve görüntü gider mi" araştırması.
Bu araştırma daha önce iki kez sıfırdan yapılıp kaydedilmemişti; bu belge o kaybı kapatır.*
