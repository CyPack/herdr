# Pattern Kataloğu — Remote Media Transport (`RM*` / anti-pattern `RA*`)

> Registry: `docs/references/remote-media-transport.md` (kanıt, ölçüm, kaynak)
> Bu dosya o registry'nin **buyurgan** hâli: ne yapılır, ne yapılmaz, hangi ölçekte.
> Her pattern bir kanıta bağlıdır — çıplak tavsiye yok.
>
> ⚠ `docs/*` herdr'da gitignored. Makine kopyası: `~/.cartography/herdr-remote-media-patterns.md`

---

## Ölçek matrisi — hangi pattern hangi durumda

| Durum | Önce sor | Zorunlu | Önerilen | Gereksiz |
|---|---|---|---|---|
| **HER durum** | **RM11** (iş yer değiştirebilir mi?) | RM13 (kapsama ile karar ver) | — | — |
| Yerel istemci (aynı makine) | — | — | RM1 | RM3, RM5, RM6, RM7 |
| **P1** uzak ses/müzik | RM11 → veri taşınabiliyorsa DUR | RM3, RM4, RM8, RM12 | RM2 | RM5, RM6, RM7, RM10 |
| **P2** uzak film | RM11 | RM2, RM3, RM4, RM8, RM12 | RM9 | RM5, RM6 (buffer emer) |
| Uzak TUI/metin | — | RM1, RM2 | RM8 | RM5, RM6, RM7 |
| **P3** interaktif görsel (browser pane) | **RM11 → C3/C4 ise DUR, client'a taşı** | RM2, RM5, RM6, RM7, RM8, RM9, RM12 | RM4, RM10 | — |
| Tarayıcı/PWA istemci | — | RM3, RM8 | RM9 | RM2 (WebSocket kendi çerçeveler) |

---

## RM1 — Sıkıştırmayı içerik sınıfına göre uygula, tek tip uygulama

**Ne:** Metin/hücre yolunu `zstd-3` ile sıkıştır. Medya yolunu **sıkıştırma**.

**Neden (ölçüm, §7.5):** terminal metin çerçevesi gzip ile **39×**, zstd-19 ile **52×** küçülüyor.
PNG ise gzip ile **1.01×** — sıfır kazanç. base64'lü PNG'deki 1.33× yalnızca base64'ün 4/3
şişmesini geri almak.

**Nasıl:** sıkıştırma kararı `MediaKind`'a bağlanır, global bir switch olmaz.

**Anti-pattern → [RA1]**

---

## RM2 — Ham kare gönderme; codec katmanı zorunludur

**Ne:** Sürekli görsel içerik **asla** kare-başına PNG/JPEG olarak gönderilmez.

**Neden (ölçüm):** 1080p @24fps kitty passthrough **146 Mbps**, aynı içerik H.264 **5.06 Mbps**
(hareketli) / **0.08 Mbps** (ekran içeriği). Fark 29×–1800×.
Üretimdeki kanıt: `herdr-browser` kare-başına JPEG kullanıyor ve README'si uzaktan
"pratik değil" diyor — ödediği bedel **260×** (§7.7.2).

**Nasıl:** L3'e temporal codec koy. Kaynağı değiştirmen gerekmez; araya codec koyman gerekir.

**Anti-pattern → [RA2]**

---

## RM3 — Ses için Opus, tek codec, iki istemci

**Ne:** Ses her zaman Opus. Rust istemcide `audiopus`, tarayıcıda WebCodecs `AudioDecoder`.

**Neden:** ölçüldü — Opus 64k = **0.34 MB/dk**, ham PCM = **10.99 MB/dk** (**1/32**).
Algoritmik gecikme 5–26.5 ms, gerçek-zamanlı için tasarlanmış. Tek codec seçilmezse sunucuda
ikinci kodlama yolu doğar ve iki istemci ayrışır.

**Ölçek:** 64 kbps = 5.7 KB/s → DERP relay üzerinde bile ihmal edilebilir. Bu yüzden
**ses fazı (F1) taşıma işinden bağımsız ilerleyebilir.**

---

## RM4 — Zaman damgası olmadan medya gönderme

**Ne:** Her medya parçası sunucu saatinde `pts_us` taşır; istemci `pts + target_latency` anında
oynatır, **geldiği anda değil**.

**Neden:** zaman damgası yoksa jitter buffer kurulamaz, A/V hizalanamaz, saat kayması telafi
edilemez. A/V senkron ayrıca uğraşılacak bir iş değildir — ses ve video aynı `pts` uzayını
paylaşırsa **bedava** gelir.

**Nasıl:** snapcast modeli — sunucu damgalar, istemci offset tahmin eder, sürüklenmeyi tek örnek
ekleyip çıkararak düzeltir (48 kHz'de bir örnek ≈ 0.02 ms, duyulmaz).

**Anti-pattern → [RA3]**

---

## RM5 — Kare hızını içeriğin değişme hızına bağla

**Ne:** Sabit fps yok. Durgun içerik **0 bps** etmeli.

**Neden:** terminal ve web içeriği patlamalı değişir. `herdr-browser` durgun sayfada saniyede
**15 adet birbirinin aynı** kareyi gönderiyor (§7.7.2) — saf israf.

**Nasıl (iyiden kötüye):**
1. **Değişim-güdümlü** — yalnız içerik değiştiğinde gönder (StructuPath varyantı böyle yapıyor)
2. **Etkileşim-güdümlü** — durgun 15 fps, etkileşimden sonra 750 ms 30 fps (kanonik plugin)
3. ❌ Sabit fps

**Ölçek:** en yüksek kaldıraçlı iki işten biri (diğeri RM7). QUIC gerektirmez.

---

## RM6 — Geri-basıncı tüketimde değil **üretimde** uygula

**Ne:** Yavaş istemcide kareyi kodlayıp sonra atma — **hiç ürettirme**.

**Neden:** kodlama maliyeti ödendikten sonra atmak, CPU'yu boşa yakar ve kuyruğu şişirir.
Kanonik `herdr-browser` bunu doğru yapıyor: `Page.screencastFrameAck`'i **geciktirerek**
Chromium'a bir sonraki kareyi ürettirmiyor. RDP de aynı ilkeyi kullanıyor (sunucu, ekran
güncelleme üretimini istemcinin tüketim hızına kısar).

**Nasıl:** kredi tabanlı akış kontrolü (`MediaCredit`) + üreticiye geri yayılan sinyal.
Son kullanma tarihi geçmiş parça **kuyruğa değil çöpe** gider → bufferbloat yapısal olarak imkânsız.

**Anti-pattern → [RA4]**

---

## RM7 — İçerik sınıfını tespit et, codec parametresini ona göre seç

**Ne:** Pane'i sınıflandır (metin / durağan görsel / hareketli video) ve codec'i buna göre ayarla.

**Neden (ölçüm):** aynı codec, aynı çözünürlük, 24 fps — ekran içeriği **0.08 Mbps**,
hareketli video **5.06 Mbps**. **60×** fark. Sabit parametre iki durumdan birinde mutlaka yanlış.
Endüstri bunu standarda yazmış: HEVC-SCC / VVC / AV1'de palette mode + intra block copy.

**Ölçek:** RM5 ile birlikte en yüksek kaldıraç. İkisi çarpımsal.

---

## RM8 — Yetenek merdiveni protokole aittir, plugin'e değil

**Ne:** İstemci ne çözebildiğini **ilan eder**; sunucu en güçlü ortak modu seçer.
Merdiven: **dokunma → yeniden paketle → yeniden kodla** (Jellyfin: Direct Play → Remux → Transcode).

**Neden:** iki bağımsız geliştirici aynı merdiveni ayrı ayrı icat etti — `StructuPath/herdr-browser`
(kitty → chafa ANSI → yalnız-metin, başlangıçta yoklayarak) ve Jellyfin (device profile).
**Bir soyutlama iki kez bağımsız icat edildiyse protokole aittir.**

**Nasıl:** `Hello.capabilities` / `Welcome.accepted`. Sunucu yalnız `accepted` içindekini kullanır.
Boş liste gönderen eski istemci bugünkü davranışı bit-birebir görür.

**Anti-pattern → [RA5]**

---

## RM9 — Bozulma sırasını önceden yaz

**Ne:** Sıkışma arttıkça şu sırayla feda et:
`video bitrate ↓ → video fps ↓ → video tamamen düş → ses bitrate ↓ → (asla) ses düşme`

**Neden:** ses kesintisi algısal olarak video kaybından kat kat pahalıdır. Bu sıra tasarım
anında yazılmazsa, sıkışma anında rastgele olan feda edilir.

**Ek:** kontrol çerçeveleri (`Input`, `Resize`, `Frame`) **her zaman** medyanın önünde.
Regresyon kapısı: medya akarken tuş yankısı gecikmesi değişmemeli.

---

## RM10 — İlerlemeli iyileştirme: önce bulanık, sonra net

**Ne:** Yeni odaklanan/açılan pane'e önce düşük çözünürlüklü kare gönder, sonra keskinleştir.

**Neden:** RemoteFX Progressive'in ilkesi — dar bantta masaüstü *hızlı* görünür.
Algısal olarak boş bekletmekten kat kat iyi.

**Ölçek:** yalnız sürekli görsel yüzeylerde anlamlı; metin pane'inde gereksiz.

---

## RM11 — Taşımayı optimize etmeden önce sor: iş yer değiştirebilir mi?

**Ne:** Medya taşıma inşa etmeden önce, işin **başka yerde koşup koşamayacağını** sor.
Tercih sırası:

```
1. VERİ taşı     (MPD httpd, SMB paylaşımı, port tüneli)   ← en ucuz, kayıpsız
2. İŞİ taşı      (uygulama client'ta koşsun)               ← kayıpsız, sıfır transport
3. PİKSEL taşı   (codec + akış)                            ← pahalı
4. AYGIT taşı    (ses/pencere forward)                     ← en kırılgan
```

**Neden:** hiçbir taşıma çözümü **"hiç taşımamak"tan** iyi olamaz. NoctaVox Mac'te koşarsa ses
bit-birebir, gecikme sıfır, codec kaybı yok — QUIC bunu asla yenemez, sadece yaklaşabilir.

**Kanıt:** `herdr-browser`'ın uzaktan çalışmaması, kullanıcının gerçek kullanımını (kendi
dev-server'ına bakmak) **hiç engellemiyor** — o case client-side browser + port tüneliyle
zaten çözülü. 260× bedel ölçüldü ama asıl soru "bu iş neden orada koşuyor" idi.

**Anti-pattern → [RA9]**

---

## RM12 — Gecikme sınıfı sistemin değil, AKIŞIN özelliğidir

**Ne:** Her akış bir **profil** ilan eder; katmanlar profile göre orkestre edilir.

| Profil | Sınıf | Buffer | Ayrı taşıma |
|---|---|---|---|
| P1 ses dinleme | B | 2–10 sn | ❌ |
| P2 film | **A** | 2–10 sn | ❌ |
| P3 interaktif görsel | **C** | 1–3 kare | ✅ |
| P4 anlık görsel | — | yok | ❌ |
| P5 olay sesi | — | yok | ❌ |

**Neden:** film izleyen pane interaktif değildir — 5 sn buffer'lanır ve A sınıfının bütün
kolaylıkları geri gelir. Film ile ajan-izleyen-browser'ı aynı boruya koymak, ikisini de en zor
duruma göre tasarlamaktır.

**Ayrım:** ayrı **taşıma** yalnız P3'ün derdi; ayrı **öncelik** her profilin.

**Anti-pattern → [RA8]**

---

## RM13 — Çözümleri kapsama (subsumption) ile değerlendir, zevkle değil

**Ne:** Aday çözümleri case'lere karşı matrise koy ve kapsama ilişkisini **kanıtla**:

```
1. Bir çözüm birden fazla case'i çözüyorsa → değerlidir.
2. Yeni çözüm öncekileri BOZMUYOR ve GEREKSİZ kılıyorsa → tek ve yegâne odur; diğerleri düşer.
3. Kapsama yoksa → case/state bazlı pros-cons tablosu ZORUNLU; tekil "en iyi" yoktur.
```

**Uygulandığında ne çıktı (§7.9):**
- QUIC yığını, ara çözümleri **kapsıyor** → onlar rakip değil, **inşa aşamaları** (yazılan kod
  %100 geçerli kalıyor, çöp iş yok)
- Ama **işi taşımayı (RM11) kapsamıyor** → tek çözüm yok, **iki kademeli hakimiyet** var:
  yer değiştirebiliyorsa RM11 tek ve yegâne; değiştiremiyorsa QUIC yığını tek ve yegâne
- Capability ve öncelik hiçbir case'i tek başına çözmüyor ama hepsinin önkoşulu →
  **ortogonal zorunluluk**, karşılaştırma ekseninde değil, **önce yapılır**

**Neden bu kural:** mimari kararı zevk meselesi olmaktan çıkarır. Kapsama ilişkisi kanıtlanır
ya da kanıtlanamaz; "bence bu daha iyi" cümlesi tabloya giremez.

---

# Anti-pattern'ler

## RA1 — Her şeyi tek tip sıkıştırmak
Medya yolunda gzip/zstd çalıştırmak **CPU yakar, bayt kazandırmaz** (1.01×). Sıkıştırmayı
"her yere aç" diye global açan konfigürasyon yanlıştır.

## RA2 — Kare-başına görüntü göndermeyi "yeterince iyi" saymak
Yerelde çalışır, uzakta çöker — ve **çöktüğü an tasarım kararını geri almak pahalıdır**.
`herdr-browser`'ın README'si bu tuzağın belgelenmiş hâlidir.

## RA3 — "Geldiği anda çiz" mantığı
Metin için doğru (en yeni durum tek doğrudur), **medya için ölümcül**. İki yüzeyin
zamanlama semantiği aynı değildir; aynı kod yolundan geçirilemez.

## RA4 — Yavaş istemcide kuyruğa yığmak
Kuyruk büyüdükçe gecikme sonsuz büyür (bufferbloat). Medya kuyruğu **derinlikle değil
son kullanma tarihiyle** yönetilir.

## RA5 — Yetenek tespitini her plugin'in kendi başına yapması
İki plugin de yarım çözdü. Sonuç: tutarsız davranış, tekrar eden kod, protokolün
bilmediği durum. Yeteneği **sunucu bilmeli**.

## RA6 — Gecikme sınıfı farklı bir sistemden trick kopyalamak
YouTube/Netflix **A sınıfı** (6–30 sn, büyük buffer, CDN, önceden kodlanmış merdiven).
herdr **C sınıfı** (interaktif, <100 ms). A sınıfının çözümleri burada **zararlıdır**:
büyük buffer interaktifliği öldürür, CDN'in fan-out'u tek izleyicide anlamsız, per-shot
çevrimdışı optimizasyon canlıda uygulanamaz. Doğru prior-art: **Moonlight/Sunshine, RDP, WebRTC.**

## RA8 — Tüm medyayı tek boruya koymak
Müzik, film ve interaktif görsel aynı gecikme sınıfında değildir. Tek boru, hepsini **en zor
olanın** kısıtlarına mahkûm eder: filme QUIC gerektirir, müziğe jitter mikro-yönetimi dayatır.
Profil ilanı (RM12) olmadan medya yolu yazılmaz.

## RA9 — "Nasıl taşırım" sorusuna, "neden burada koşuyor" sorulmadan cevap aramak
Taşıma optimizasyonu, yanlış yerleşimi asla telafi edemez. 260× bedel ölçüp mimariyi
düzeltmeye kalkmak, işi client'a taşımanın sıfır maliyetli olduğu bir dünyada kayıptır.
Önce RM11 karar ağacı, sonra katman tasarımı.

## RA10 — Ara çözümü "rakip" sanıp atlamak
Kapsama analizi yapılmazsa, F1'in F2 tarafından kapsandığı görülür ve "o zaman direkt F2
yapalım" denir. Yanlış: kapsanan çözüm **aşamadır**, kodu %100 geçerli kalır ve riski
küçültür. Atlanan şey iş değil, **güvenli artımlı yoldur**.

## RA7 — `PROTOCOL_VERSION`'ı medya için bump etmek
Sürüm tam-eşleşmedir; bump, medya istemeyen **tüm** istemcileri kırar. Medya **additive**
olmalı (RM8).

---

## Kaynak → pattern izi

| Kaynak | Doğurduğu pattern |
|---|---|
| Ölçüm §7.5 (bu makine, 2026-08-12) | RM1, RM2, RM3, RM7 |
| snapcast | RM4 |
| Jellyfin device profile | RM8 |
| RDP / RemoteFX | RM6, RM10 |
| Moonlight/Sunshine | RM5, RM9 |
| BOLA / dash.js | RM6 (buffer sinyali) |
| HEVC-SCC / AV1 | RM7 |
| `ogulcancelik/herdr-browser` | RM2 (bedel), RM6 (doğru uygulama), RA2 |
| `StructuPath/herdr-browser` | RM5, RM8 |
| CDP `Page.startScreencast` | RM2 (T1 imkânsız → codec şart) |

*v1.0.0 — 2026-08-12*
