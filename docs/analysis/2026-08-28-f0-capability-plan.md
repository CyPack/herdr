# F0 — PRD, Bağımlılık Zinciri, Test Planı, Görevler

> **Araştırma tabanı:** `2026-08-28-f0-capability-research.md` (aynı dizin)
> **Dal:** `feat/media-capability` · taban `master@097f77b3` · worktree `~/projects/herdr-worktrees/media-capability`
> Sıra: PRD → bölümleme → bağımlılık zinciri → **test noktaları** → görevler → icra.

---

## 1. PRD

### 1.1 Amaç

herdr'ın istemci el sıkışmasına, **sürüm bump'ı gerektirmeden genişletilebilen** bir
yetenek anlaşması eklemek; ve hâlihazırda doğru olan kontrol-önceliği davranışını bir
sonraki upstream merge'inin sessizce silemeyeceği biçimde kaydetmek.

F0 medya **taşımaz**. F0'ın ürünü, F1-F3'ün üzerine kırılmadan inebileceği zemindir.

### 1.2 Kapsam

**İÇİNDE**
1. `CapabilityEntry` wire tipi (ad + değerler), bilinmeyeni-yok-sayan okuma sözleşmesi.
2. `Hello.capabilities` (istemci ilanı) ve `Welcome.accepted` (sunucu onayı) alanları.
3. Kesişim mantığı: sunucu **yalnız** hem kendi desteklediği hem istemcinin ilan ettiği
   yetenekleri `accepted`'a koyar; sunucu **yalnız** `accepted` içindekileri kullanır.
4. Tek seferlik `PROTOCOL_VERSION` 21 → 22 ve "son medya bump'ı" gerekçe yorumu.
5. Sunucu tarafında yetenek durumunun bağlantı ömrü boyunca taşınması
   (`ServerEvent::ClientConnected` üzerinden).
6. İstemci tarafında kendi yeteneklerini ilan etmesi ve sunucunun kabulünü saklaması.
7. Kesin-öncelik davranışının davranış kaydına (`behaviors/shared-surfaces.md`) TP ile
   bağlanması.

**DIŞINDA** (gerekçeleriyle: research §3)
`MediaOpen/Chunk/Close` · codec bağımlılığı · kredi/akış kontrolü · medya şeridi ·
son-kullanma-tarihli düşürme · QUIC yan kanal · ses/video ingest.

### 1.3 Kabul kriterleri (ölçülebilir)

| # | Kriter | Nasıl ölçülür |
|---|---|---|
| A1 | Boş yetenek ilan eden istemci **bugünküyle bit-birebir aynı** davranışı görür | TP-1.1: boş ilanla üretilen `Welcome` baytları, alan-öncesi davranışın beklenen çıktısıyla aynı; `accepted` boş |
| A2 | Bilinmeyen ad **sessizce yok sayılır**, bağlantı düşmez | TP-1.2 |
| A3 | Yeni yetenek adı eklemek wire şeklini **değiştirmez** | TP-1.3 (aynı bayt uzunluğu ve çözülebilirlik) |
| A4 | Sunucu `accepted` dışındaki hiçbir yeteneği kullanmaz | TP-2.3 (kesişim testi) |
| A5 | Kontrol çerçeveleri medyadan önce gider (mevcut davranış korunur) | TP-3.1 + TP-3.2 |
| A6 | Kesin-öncelik davranışı bir TP kimliğiyle kayıtlı | `just check` C1 kapısı |
| A7 | Sürüm 22, tam-eşleşme semantiği **değişmedi** | TP-4.1 |
| A8 | Geride kırmızı test yok; lint temiz | `cargo nextest run` + `clippy -D warnings` (HP kutusunda) |

---

## 2. Bölümleme — katmanlar ve alt katmanlar

```
F0
├── L1  WIRE TİPİ (src/protocol/wire.rs)
│   ├── L1.a  CapabilityEntry struct + ad sabitleri
│   ├── L1.b  CapabilitySet sarmalayıcı (okuma sözleşmesi: has/values_of/intersect)
│   ├── L1.c  Hello.capabilities alanı
│   ├── L1.d  Welcome.accepted alanı
│   └── L1.e  PROTOCOL_VERSION 21→22 + gerekçe yorumu
│
├── L2  SUNUCU EL SIKIŞMASI (src/server/client_transport.rs)
│   ├── L2.a  Hello desen-eşleşmesine capabilities'i al
│   ├── L2.b  Sunucu-desteklenenler ∩ istemci-ilan edilenler → accepted
│   ├── L2.c  accepted'ı Welcome'a koy
│   └── L2.d  accepted'ı ClientConnected olayıyla oturuma taşı
│
├── L3  İSTEMCİ TARAFI (src/client/mod.rs)
│   ├── L3.a  İstemci kendi yeteneklerini ilan eder (F0'da: boş/temel)
│   └── L3.b  Welcome.accepted'ı sakla (F1'in okuyacağı yer)
│
└── L4  KAYIT + KORUMA (behaviors/)
    ├── L4.a  TP-MEDIA-CAP-01  yetenek anlaşması
    ├── L4.b  TP-MEDIA-CAP-02  bilinmeyeni yok sayma
    └── L4.c  TP-CLIENT-WRITE-PRIO-01  kesin öncelik (MEVCUT davranışı kaydet)
```

---

## 3. Bağımlılık zinciri

```
L1.a ──► L1.b ──► L1.c ──► L2.a ──► L2.b ──► L2.c ──► L3.a ──► L4.a/b
              └──► L1.d ──┘                    └──► L2.d ──► L3.b
L1.e ──────────────────────► (tüm sürüm-duyarlı testler)
L4.c  (bağımsız — L1..L3'ten önce de sonra da yapılabilir)
```

**Sıra kuralı (ters sıra hata vermez, BOŞ SONUÇ verir):**

| Adım | Neden bu sırada | Ters sıranın sessiz başarısızlığı |
|---|---|---|
| L1.a önce L1.c | Alan tipi yoksa alan eklenemez | derleme hatası (gürültülü, güvenli) |
| L1.c/L1.d önce L2.a | Desen-eşleşmesi alanı olmayan struct'tan çıkaramaz | E0026 (gürültülü) |
| **L2.b önce L2.c** | Kesişim hesaplanmadan `accepted` yazılırsa istemcinin ilanı olduğu gibi geri döner → **sunucu desteklemediği yeteneği onaylamış olur, test yeşil kalır** | ⚠ SESSİZ — A4 kriteri bu yüzden ayrı test edilir |
| **L2.d önce L3.b** | Sunucu accepted'ı oturuma taşımazsa istemci onu saklar ama sunucu unutmuştur; F1'de "istemci ses istedi ama sunucu bilmiyor" | ⚠ SESSİZ — F1'de patlar, F0'da görünmez → TP-2.4 bunu şimdi pinler |
| L4.c herhangi bir zaman | Mevcut davranışı kaydeder, yeni kod eklemez | — |

**Kritik gözlem:** zincirdeki iki sessiz başarısızlık noktası (L2.b, L2.d) tam da
test edilmezse F1'e taşınacak olanlar. Test planı bu ikisine ayrı test noktası ayırıyor.

---

## 4. TEST PLANI — icradan ÖNCE (ne · beklenen · neden)

> Yöntem: her nokta önce **KIRMIZI** yazılır, sonra minimal kod yeşile çevirir (TDD).
> Runner: `cargo nextest run` (paralel `cargo test` kanıtlı flaky — rust-dev E1).
> Koşum yeri: **HP kutusu** (`herdr-hp-check`), laptopta derleme yasak.

### L1 — Wire tipi

| TP | Ne test edilir | Beklenen sonuç | NEDEN bu test |
|---|---|---|---|
| **TP-1.1** | Boş `capabilities` ile `Hello` roundtrip + boş `accepted` ile `Welcome` roundtrip | Tam eşitlik; `accepted.is_empty()` | A1'in wire yarısı: "boş ilan = bugünkü davranış" iddiası kanıtsız kalmasın |
| **TP-1.2** | `CapabilitySet` içinde tanınmayan ad (`"future.thing"`) + tanınan ad | Tanınan bulunur, tanınmayan `has()` false döner, **panik/hata yok** | A2. Bu, tasarımın tüm additive vaadinin taşıyıcısı — kırılırsa F1+ her yeteneği bump ister |
| **TP-1.3** | Aynı `Hello`'yu 1 ve 3 yetenekle serileştir | İkisi de çözülebilir; **şekil aynı**, yalnız uzunluk artar (ayrımcı kayması YOK) | A3 + risk R4: bincode varyant-kayması tuzağı (2026-08-11 olayı) bir daha yaşanmasın |
| **TP-1.4** | `values_of("media.audio")` çok değerli girdide | Değerler sırayla döner | Codec listesi F1'de bu yoldan okunacak; boş dönerse F1 sessizce codec'siz kalır |
| **TP-1.5** | Aynı ad iki kez ilan edilirse | İlk kayıt kazanır, panik yok | Kötü niyetli/bozuk istemci girdisi — el sıkışma çökmemeli |
| **TP-1.6** | `intersect()` — istemci {A,B,C} ∩ sunucu {B,C,D} | `{B,C}`; **sunucuya özgü D DIŞARIDA** | A4'ün saf mantık yarısı; L2.b'nin sessiz başarısızlığının birim testi |

### L2 — Sunucu el sıkışması

| TP | Ne test edilir | Beklenen sonuç | NEDEN bu test |
|---|---|---|---|
| **TP-2.1** | Yetenek ilan eden istemci el sıkışması | `Welcome.error == None`, `accepted` = kesişim | Mutlu yol; el sıkışmanın uçtan uca çalıştığı kanıtı |
| **TP-2.2** | Yalnız bilinmeyen yetenek ilan eden istemci | Bağlantı **kabul edilir**, `accepted` boş | A2'nin uçtan uca hâli — birim testi geçip entegrasyonda düşme ihtimalini kapatır |
| **TP-2.3** | İstemci sunucunun desteklemediği bir ad ilan eder | O ad `accepted`'ta **YOK** | ⚠ L2.b sessiz başarısızlık noktası (bkz. §3) — "olduğu gibi geri yansıtma" hatası |
| **TP-2.4** | El sıkışma sonrası `ClientConnected` olayı | Olay `accepted` yeteneklerini taşır | ⚠ L2.d sessiz başarısızlık noktası — F1'in okuyacağı tek yer burası |
| **TP-2.5** | Yanlış sürümlü istemci (21 veya 23) | Reddedilir, `Welcome.error` dolu; **yetenek işlenmez** | A7: bump'ın sürüm semantiğini değiştirmediği; yetenek anlaşması sürüm kapısını **atlamamalı** |
| **TP-2.6** | İlk mesaj `Hello` değil | Bugünkü ret davranışı **değişmemiş** | Regresyon: yeni alan hata yolunu bozmasın |

### L3 — İstemci

| TP | Ne test edilir | Beklenen sonuç | NEDEN bu test |
|---|---|---|---|
| **TP-3.1** | İstemcinin ürettiği `Hello` | Sürüm 22 + ilan alanı mevcut (F0'da boş liste meşru) | İstemci-sunucu simetrisi; tek taraflı uygulama F1'de patlar |
| **TP-3.2** | İstemci `Welcome.accepted`'ı saklar | Saklanan değer sunucununkiyle aynı | F1 istemci tarafı buradan okuyacak |

### L4 — Öncelik koruması (mevcut davranış)

| TP | Ne test edilir | Beklenen sonuç | NEDEN bu test |
|---|---|---|---|
| **TP-4.1** | Kontrol + render aynı anda kuyrukta | Kontrol **önce** çıkar | A5. Mevcut test bunu zaten yapıyor → TP kimliğiyle **adlandırılır** ki merge silemesin |
| **TP-4.2** | Kuyruk kapandıktan sonra gönderim | Hata döner, panik yok | Regresyon koruması; yeni alanlar bu yolu etkilememeli |
| **TP-4.3** | `behaviors/` kaydı ↔ test adı bağı | `just check` C1 kapısı yeşil | A6: kaydın kendisinin mekanik olarak doğrulanması |

### Regresyon kapısı (her adımda)

| Kontrol | Komut | Beklenen |
|---|---|---|
| Derleme + tam patlama yarıçapı | `cargo check --all-targets` | 0 hata |
| Biçim | `cargo fmt --check` | temiz |
| Lint | `cargo clippy --all-targets --locked -- -D warnings` | 0 uyarı |
| Test | `cargo nextest run` | **eskisi kadar veya daha fazla** yeşil, 0 kırmızı |
| Fork kapısı | `just check` | C1 davranış-kaydı kapısı dahil yeşil |

---

## 5. Görevler ve alt görevler

| # | Görev | Bağımlı | Test noktası | Statü |
|---|---|---|---|---|
| **T1** | `CapabilityEntry` + ad sabitleri (L1.a) | — | TP-1.1 | TODO |
| T1.1 | RED: TP-1.1 roundtrip testi | — | | |
| T1.2 | GREEN: struct + `Serialize/Deserialize` | T1.1 | | |
| **T2** | `CapabilitySet` okuma sözleşmesi (L1.b) | T1 | TP-1.2, 1.4, 1.5, 1.6 | TODO |
| T2.1 | RED: bilinmeyen-ad, çok-değer, tekrar-ad, kesişim testleri | T1 | | |
| T2.2 | GREEN: `has` / `values_of` / `intersect` | T2.1 | | |
| **T3** | `Hello.capabilities` + `Welcome.accepted` (L1.c/d) | T2 | TP-1.3 | TODO |
| T3.1 | RED: şekil-kararlılığı testi (1 vs 3 giriş) | T2 | | |
| T3.2 | GREEN: alanları ekle; **tüm literal sitelerini** güncelle (risk R1) | T3.1 | | |
| **T4** | `PROTOCOL_VERSION` 21→22 + gerekçe (L1.e) | T3 | TP-2.5 | TODO |
| T4.1 | Bump + "son medya bump'ı" yorumu (RA7 karşı-notu) | T3 | | |
| T4.2 | `cargo check --all-targets` ile patlama yarıçapını al ve kapat (risk R2) | T4.1 | | |
| **T5** | Sunucu el sıkışması (L2.a-c) | T4 | TP-2.1, 2.2, 2.3, 2.6 | TODO |
| T5.1 | RED: kesişim + bilinmeyen-ad + regresyon testleri | T4 | | |
| T5.2 | GREEN: desen-eşleşmesi, `SERVER_CAPABILITIES`, kesişim, `Welcome` | T5.1 | | |
| **T6** | `accepted`'ı oturuma taşı (L2.d) | T5 | TP-2.4 | TODO |
| T6.1 | RED: `ClientConnected` yetenek taşıma testi | T5 | | |
| T6.2 | GREEN: `ServerEvent` alanı + geçirme | T6.1 | | |
| **T7** | İstemci tarafı (L3) | T6 | TP-3.1, 3.2 | TODO |
| T7.1 | RED: istemci Hello ilanı + accepted saklama | T6 | | |
| T7.2 | GREEN: istemci uygulaması | T7.1 | | |
| **T8** | Davranış kaydı (L4) | T7 | TP-4.1, 4.2, 4.3 | TODO |
| T8.1 | TP-MEDIA-CAP-01/02 satırları + test adları | T7 | | |
| T8.2 | TP-CLIENT-WRITE-PRIO-01 (mevcut davranışı kaydet) | — | | |
| **T9** | Kapı + belge kapanışı | T8 | tüm regresyon | TODO |
| T9.1 | HP kutusunda `just check` | T8 | | |
| T9.2 | `docs/patterns/remote-media-transport.md` F0 durumunu işaretle | T9.1 | | |
| T9.3 | Makine kopyalarını senkronla (`~/.cartography/`) | T9.2 | | |

**Commit disiplini:** her T*, kendi commit'i (ne + NEDEN + ölçüm). RED ve GREEN ayrı
commit'ler (TDD izi git geçmişinde görünür kalır).

---

## 6. V (sonlanma ölçüsü)

```
V = (yazılmamış test noktası) + (kırmızı test) + (kapsanmamış kabul kriteri)
  + (kayıtsız shared-surface davranışı) + (düşen kapı)

Başlangıç: V = 20 TP + 8 kriter + 3 TP-kaydı = 31
DUR: V = 0  ·  V iki tur sabit (doygunluk)  ·  eskalasyon kapısı (§D)
```

*v1.0.0 — 2026-08-28*
