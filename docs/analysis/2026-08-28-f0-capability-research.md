# F0 — Capability Negotiation: Araştırma ve Mimari Karar

> **Faz:** F0 (S6 capability + S7 öncelik) — remote-media tasarımının ilk fazı
> **Kaynak tasarım:** `docs/references/remote-media-transport.md` §L1, §7.9.6
> **Pattern:** `docs/patterns/remote-media-transport.md` RM8, RM11, RM13 · anti-pattern RA5, RA7
> **Dal:** `feat/media-capability` · taban `master@097f77b3`
>
> Bu belge **ölçülmüş mevcut durumu** ve F0'ın tek kritik mimari kararını kaydeder.
> Her iddia dosya:satır kanıtına bağlıdır.

---

## 1. Mevcut durum — ölçülmüş envanter

### 1.1 Protokol sürümü ve el sıkışma

| Olgu | Kanıt |
|---|---|
| `PROTOCOL_VERSION = 21` | `src/protocol/wire.rs:25` |
| Sürüm kontrolü **tam eşleşme**; eski DE yeni DE reddedilir | `wire.rs:1048-1067` `check_client_version` |
| `Hello` 9 alan taşır (`version`, `cols`, `rows`, `cell_width_px`, `cell_height_px`, `requested_encoding`, `keybindings`, `launch_mode`, `pixel_mouse`) | `wire.rs:354-378` |
| `Welcome` 3 alan taşır (`version`, `encoding`, `error`) | `wire.rs:693-701` |
| El sıkışma tek yerde uygulanıyor | `src/server/client_transport.rs:570-633` |
| Serileştirme `serde` + bincode (uzunluk-önekli çerçeve) | `wire.rs:9`, `wire.rs:40` `LENGTH_PREFIX_BYTES` |

### 1.2 Belge ile kaynak arasındaki sürüklenme (düzeltildi)

Kanonik tasarım belgesi `PROTOCOL_VERSION = 20` diyor; kaynak **21**. Sürüm 21'in
kendi yorumu, F0'ın var olma sebebini birinci ağızdan anlatıyor:

> *"21, not 20: protocol 20 is already published in the preview channel, and `Hello`
> now carries the client's pixel-mouse capability. The check below is exact-match,
> so reusing 20 would let a released client speak a different dialect under the
> same number."* — `wire.rs:15-19`

Yani **tek bir boolean yetenek** (`pixel_mouse`) eklemek için tüm istemci filosu
kırıldı. Bu, `RA7`'nin ("`PROTOCOL_VERSION`'ı medya için bump etmek") tarif ettiği
tuzağın **zaten yaşanmış** hâlidir. Medya 4-5 yetenek daha getirecek; aynı yolla
her biri bir bump demek olurdu.

### 1.3 Öncelik katmanı — beklenenden ileri

`src/server/client_transport.rs:176-300` üç şeritli bir yazar kuyruğu uyguluyor:

| Şerit | Alan | Semantik |
|---|---|---|
| control | `VecDeque<Vec<u8>>` | sınırsız, FIFO |
| ordered | `VecDeque<Vec<u8>>` | tek-slot geri-basınç (`send_ordered` doluysa `Full` döner) |
| render | `Option<Vec<u8>>` | tek slot, en-yeni-kazanır |

`recv()` (satır 265-283) **kesin öncelik** uyguluyor: önce `control`, sonra `ordered`,
sonra `render`. Yani §L5'in 1. kuralı ("kontrol çerçeveleri medyanın önüne geçer,
her zaman") **zaten canlıda**.

Test durumu: `client_writer_prioritizes_control_and_reports_render_drain`
(`client_transport.rs:1120`) bu davranışı pinliyor. **Ancak bu test hiçbir davranış
kimliğine (TP-*) bağlı değil** — `behaviors/` altında `client_transport.rs` geçmiyor.
Fork disiplinine göre (`behaviors/README.md`): *"A fork behavior that no test names is
a behavior the next merge can delete without telling anyone."* Öncelik davranışı
upstream'in de sahip olduğu bir dosyada yaşıyor → **shared surface**, kayıt gerekli.

---

## 2. F0'ın tek kritik mimari kararı

### 2.1 Problem ifadesi

RM8 der ki: *"Yetenek merdiveni protokole aittir, plugin'e değil."* §L1 der ki:
*"Yetenek listesi boş gelen eski bir istemci, bugünkü davranışı bit-birebir aynı
şekilde görür. Bu, medyayı **additive** yapar."*

**Additive** iddiası şu somut testi geçmek zorundadır:

> Sunucuya *yeni bir medya yeteneği* eklendiğinde, o yeteneği bilmeyen istemciler
> **kırılmadan** ve **sürüm bump'ı olmadan** çalışmaya devam etmelidir.

### 2.2 Neden naif enum bu testi GEÇEMEZ

Tasarım belgesindeki taslak:

```rust
pub enum Capability {
    MediaStreams,
    AudioSink { codecs: &'static [MediaCodec] },
    ...
}
```

İki ayrı kusuru var:

**(a) `&'static [T]` deserialize edilemez.** `Deserialize` sahiplik ister → `Vec<T>`.
Bu mekanik bir düzeltme.

**(b) Asıl kusur: bincode kendi-tanımlayıcı (self-describing) DEĞİLDİR.**
Enum varyantı bir tamsayı ayrımcıyla (discriminant) kodlanır. Karşı taraf bilmediği
bir ayrımcı görürse **tüm mesajın çözümü hata verir** — o alanı atlayamaz, çünkü
ne uzunluğunu ne şeklini bilir. Bu depoda tam bu tuzağa daha önce düşülmüş:

> *"bincode enum'una varyant 'sona ekledim' sanısı — Notify 5→6 kaydı, indeksle
> okuyan test 9.4 sn timeout … aynı-build roundtrip kaymayı GÖREMEZ (iki taraf
> birlikte kayar)"* — `~/.claude/skills/rust-dev/lessons/errors.md`, 2026-08-11

Sonuç: `Capability` bir enum olursa, **yeni varyant = yeni wire = yeni sürüm bump**.
Tam olarak kaçınmak istediğimiz döngü. Enum, mekanizmayı kurar ama vaadini tutmaz.

### 2.3 Karar: ada dayalı, bilinmeyeni-yok-sayan kayıtlar

```rust
/// Tek bir yetenek ilanı: ad + isteğe bağlı parametre değerleri.
pub struct CapabilityEntry {
    pub name: String,
    pub values: Vec<String>,
}
```

`Hello.capabilities: Vec<CapabilityEntry>` · `Welcome.accepted: Vec<CapabilityEntry>`.

**Kural:** tanınmayan `name` **sessizce yok sayılır**; tanınan ad tipli bir yardımcıyla
okunur. Wire şekli (ad listesi) yeni yetenek eklendiğinde **değişmez** → sürüm bump
gerekmez → additive vaadi gerçekten tutulur.

### 2.4 Bu karar icat değil — endüstri normu

| Protokol | Aynı problemi nasıl çözmüş | Kanıt |
|---|---|---|
| **SSH** (RFC 4253 §7.1) | Algoritma **ad listesi** (virgülle ayrık); iki taraf kesişimden seçer, bilinmeyen ad yok sayılır | official spec |
| **TLS** (RFC 8446 §4.2) | Uzantılar tipli; *"clients MUST ignore unrecognized extensions"* | official spec |
| **HTTP/2** (RFC 9113 §6.5.2) | *"An endpoint that receives a SETTINGS frame with any unknown or unsupported identifier MUST ignore that setting."* | official spec |
| **Jellyfin device profile** | İstemci ne çözebildiğini **ilan eder**, sunucu Direct Play / Remux / Transcode merdiveninden seçer | `docs/references/remote-media-transport.md` §7.6.2 (T2) |
| **`StructuPath/herdr-browser`** | kitty → chafa → düz metin merdivenini **kendi başına** yeniden icat etti | aynı belge §7.7.4 (RM8'in doğuş kanıtı) |

Üç bağımsız IETF protokolü ve iki bağımsız uygulama aynı şekle yakınsamış. Bu, RM13'ün
("kapsama ile karar ver") gerektirdiği kanıt: **ada dayalı liste, enum'un çözdüğü her
şeyi çözer ve enum'un çözemediği ileri-uyumluluğu da çözer** → enum tartışmasız düşer.

### 2.5 Bedeli, dürüstçe

| Bedel | Ölçü | Neden kabul edilebilir |
|---|---|---|
| El sıkışmada string taşınır | Tipik 3-5 giriş × ~20 bayt ≈ **100 bayt**, bağlantı başına **bir kez** | Opus'un 5,7 KB/s'i yanında ölçülemez (§7.5) |
| Tip güvenliği kayboluyor | — | Kaybolmuyor: adlar `pub const` sabitler, okuma tipli yardımcılarla (`fn has(&self, name) -> bool`, `fn values_of(&self, name)`) yapılır; sözleşme test edilir |
| Yazım hatası derleme zamanında yakalanmaz | — | Sabit + `#[deny(clippy::...)]` yerine **sabitlere zorunlu erişim**: literal string kullanan çağrı yeri kalmayacak, kayıt testi bunu pinler |

### 2.6 Tek seferlik sürüm bump'ı: 21 → 22

Alanın **kendisini** eklemek wire'ı değiştirir; bu kaçınılmaz ve **son** medya
bump'ıdır. Yorumda bunu açıkça yazacağız ki gelecekteki bir oturum "medya için bump"
refleksine dönmesin (RA7).

---

## 3. F0 kapsam sınırı — ne YAPILMAYACAK (dürüstlük bölümü)

| Yapılmayacak | Neden | Nereye ait |
|---|---|---|
| `MediaOpen`/`MediaChunk`/`MediaClose` mesajları | Üreticisi yok; ölü kod + clippy `dead_code` | F1 |
| Opus/codec bağımlılığı | F0 medya taşımıyor | F1 |
| Kredi tabanlı akış kontrolü (`MediaCredit`) | Akış yokken kredi anlamsız | F2 |
| Medya şeridi + son-kullanma-tarihli düşürme | Aynı sebep; şerit boş kalırsa ölü kod | F1 |
| QUIC / yan kanal | RM12 + K3: yalnız P3 için gerekli, en düşük öncelik | F3 (ertelendi) |

**F0'ın S7 payı**, bu yüzden yeni kod değil **koruma**dır: hâlihazırda doğru olan
kesin-öncelik davranışını adlandırılmış bir davranış kimliğine bağlamak, ki bir
sonraki upstream merge'i sessizce silemesin (`behaviors/README.md` tek kuralı).

---

## 4. Ölçülen risk noktaları

| # | Risk | Kanıt / gerekçe | Azaltma |
|---|---|---|---|
| R1 | `Hello`'ya alan eklemek başka literal sitelerini kırar (E0063) | rust-dev `lessons/errors.md`: *"Yeni AppState alanı → E0063 beklenmedik İKİNCİ literal'de"* | `grep -rn "pixel_mouse" src/` ile TÜM `Hello` literal sitelerini önce say, hepsini aynı dalgada güncelle |
| R2 | Sürüm sabitini bump etmek sürüme bağlı testleri kırar | `autodetect.rs:559` `PROTOCOL_VERSION + 1` kullanıyor | Bump sonrası `cargo check --all-targets` ile tam patlama yarıçapını al |
| R3 | Yeni `pub` fonksiyon bin crate'te `dead_code` | rust-dev `lessons/errors.md` (2026-07-12) | Üreticiyi (handshake) aynı dalgada bağla — F0'da zaten bağlanıyor |
| R4 | Wire enum/struct sırası kayması | `lessons/errors.md` (2026-08-11): aynı-build roundtrip kaymayı göremez | Ayrımcıyı **sabit kodlayan** test yaz (mevcut `tag()` testleri deseni, `wire.rs:1141`) |
| R5 | `cargo test` paralel flaky | `lessons/errors.md` E1 | Yalnız `cargo nextest run` |
| R6 | Laptopta derleme yasak | `~/.claude/rules/remote-build-offload.md` (kullanıcı talimatı) | `herdr-hp-check` ile HP kutusunda koş |

---

## 5. Kaynak izi

| İddia | Kaynak | Tip | Güven |
|---|---|---|---|
| Sürüm 21, tam-eşleşme, bump gerekçesi | `src/protocol/wire.rs:15-25,1048-1067` | source_code | 0.95 |
| Öncelik zaten uygulanmış | `src/server/client_transport.rs:265-283` | source_code | 0.95 |
| Öncelik testi var ama TP kaydı yok | `client_transport.rs:1120` + `grep behaviors/` boş | source_code | 0.9 |
| bincode kendi-tanımlayıcı değil, varyant ekleme kırar | `lessons/errors.md` 2026-08-11 (ölçülmüş olay) | project-memory | 0.9 |
| Ad-listesi deseni endüstri normu | RFC 4253 §7.1 · RFC 8446 §4.2 · RFC 9113 §6.5.2 | spec | 0.95 |
| Jellyfin device profile / plugin merdiveni | `docs/references/remote-media-transport.md` §7.6.2, §7.7.4 | registry (ölçülmüş) | 0.9 |
| Opus 64k = 5,7 KB/s (string bedelinin ihmal edilebilirliği) | aynı belge §7.5 (ffmpeg ölçümü) | measurement | 0.95 |

*v1.0.0 — 2026-08-28*
