---
doc: resource-doctrine
scope: >
  herdr'ın KAYNAK ve TRAFİK ANAYASASI — server-side + client-side mimari prensipleri,
  neden bu tercihler yapıldı (pros/cons + prior-art + yaşanmış olaylar), error-handling
  doktrini, ölçülmüş sayılar ve her yeni feature'ın geçmek zorunda olduğu kontrol listesi.
  docs/patterns/rust-engineering.md kod-mühendisliği katmanıysa, BU dosya çalışma-zamanı
  ekonomisi katmanıdır. Her yeni bileşen/feature bu doktrine karşı tasarlanır.
created: 2026-08-17
status: canonical — versiyonlu (docs/patterns gitignore istisnası); her iddia ölçüm- veya
  kaynak-etiketli, çıplak iddia yok
agentic_triggers:
  - "herdr feature · herdr yeni bileşen · pane · workspace · render · scroll · devir · update"
  - "kaynak tüketimi · CPU · bellek · RAM · fd · trafik · bant genişliği · verimlilik"
  - "dormancy · uyku · idle · retired · freight · handoff · lifecycle · yaşam döngüsü"
  - "performans · optimizasyon · profil · ölçüm · perf"
related:
  - docs/patterns/rust-engineering.md      # HP1-HP18 kod-mühendisliği kataloğu (kardeş katman)
  - behaviors/shared-surfaces.md           # TP-* davranış kayıtları (bu doktrinin bekçi testleri)
  - tests/perf_curve.rs                    # maliyet eğrisi harness'ı (#[ignore], elle koşulur)
  - vendor/libghostty-vt.patches.md        # vendored patch disiplini (0002 bu doktrinin ürünü)
  - CLAUDE.md                              # UPSTREAM dosyası — DÜZENLENMEZ; bu dosya onu TAMAMLAR
---

# herdr Kaynak & Trafik Doktrini (RD0–RD9)

> **Bu dosyanın varlık sebebi (kullanıcı anayasası, verbatim):**
> *"bu bir multiplexer, amaç aktif agentların ve processlerin arkaplanda 7-24 çalışabiliyor
> olması"* ve *"herdr gelistirme surecinde desktop app'ler gibi kaynak tuketimi olmasini
> kesinlikle istemiyoruz; web servisleri gibi yuksek ve gereksiz trafikler olmasini da
> istemiyoruz."*
>
> İki cümle birlikte tasarımın tamamını verir:
> ```
> CANLILIK korunur — agent client olmadan da yaşar; bu ürünün varlık sebebi.
> MALİYET  kalkar  — yaşayan şey, kimse bakmıyorken bedel ödemez.
> ```
> Bugünkü ölçüm bu ikisinin ayrılabilir olduğunu kanıtladı: 61 canlı PTY'nin toplam bedeli
> %0,6 core iken pahalı olan "bakılmıyorken bakılıyormuş gibi çizmek"ti.

Her prensip şu şablonu taşır: **KURAL → NEDEN (ölçüm/olay) → PROS/CONS → PRIOR-ART → BEKÇİ**.
Bekçi = `behaviors/shared-surfaces.md`'de adıyla yaşayan test; bekçisiz prensip bir sonraki
merge'in sessizce silebileceği bir dilek olarak kalır.

---

## RD0 · Ölç, tasarla, tekrar ölç — sıra budur, tersi değil

**KURAL:** Hiçbir kaynak/trafik kararı ölçümsüz alınmaz; hiçbir "düzeldi" iddiası ürün
katmanından taze kanıtsız yapılmaz. Ölçüm koşulları sonucun parçasıdır: kirli koşum kirli
cevap üretir.

**NEDEN (yaşanmış):** Aynı soruya ("guard testi kırdı mı?") dört kez cevap verildi, ilk üçü
yanlıştı — belirleyici olan akıl yürütme değil ölçüm KOŞULUYDU (tek örnek → kirli tam koşum →
temiz tam koşum). Kanonik koşum reçetesi bundan doğdu: *temiz ağaç → prebuild → setsid koşum →
bitene kadar sıfır derleme*. İkinci vaka: koordinatör hattının "ilk etki 24 saat sonra"
çıkarımı, adaylık koşulu ölçülünce çürüdü (46/46 "boş" pane'de canlı shell vardı — bayrağı
açmak ≠ etki üretmek).

**PROS:** yanlış optimizasyona hiç girilmedi; her inen değişikliğin etkisi sayıyla biliniyor.
**CONS:** ölçüm disiplini zaman alır; izole tezgâh kurmak (throwaway XDG, kısa soket yolu)
ekstra iştir. Kabul edilen bedel.

**PRIOR-ART:** klasik "measure, don't guess" (Brendan Gregg / systems performance geleneği);
bu repoda somutlaması `tests/perf_curve.rs` + `eu-stack`/`/proc` örnekleme reçeteleri.

**BEKÇİ:** eğri harness'ı `render_cost_curve` (elle koşulur); süreç kuralları
`.local/prd/pane-runtime-lifecycle.md` §7e'de.

---

## RD1 · Üç-eksen modeli — tek "durum" ekseni yalan söyler

**KURAL:** Bir pane üç bağımsız soruyla tarif edilir ve cevapları üç ayrı otoriteden gelir:

| eksen | soru | otorite | değerler |
|---|---|---|---|
| **Dikkat** | Bakan var mı? | client'lar (`tab_effectively_watched` — TEK tanım) | Watched · Unwatched |
| **Canlılık** | PTY'de çocuk var mı? | çocuk süreç + herdr defteri | Live · Exited |
| **Etkinlik** | Son N sürede çıktı var mı? | PTY | Active · Quiet |

Maliyet sözleşmesi bu eksenlerin çarpımıdır: *Watched+Live+Active* tam bedel öder;
*Unwatched+Live+Quiet* **sıfır tick, sıfır geometri, sıfır algılama** öder;
*Unwatched+Exited* **dormant** olur (scrollback diske, runtime bırakılır).

**NEDEN:** Ölçülen üç ayrı kusurun (arka plan süpürmesi, odak-geçiş süpürmesi, izleyicisiz
render) ortak kökü bu eksenlerin karıştırılmasıydı — model uydurma değil, ölçümden türedi.

**PROS:** her yeni feature "hangi eksende maliyet ekliyorum?" sorusuna indirgenebilir.
**CONS:** eksen otoritelerinin tek-tanımlılığı disiplin ister (izlenme tanımı iki yerde
farklılaşırsa uyut-uyandır flapping'i doğar — bu yüzden tanım tek fonksiyonda).

**PRIOR-ART:** Chrome tab discarding / Android LMK "foreground ≠ alive ≠ active" ayrımının
terminal karşılığı.

**BEKÇİ:** TP-DORMANT-01..12 ailesi + TP-MCF-SIZE-03..06.

---

## RD2 · Canlılık ucuz, çizim pahalı — render yalnız olaya ve izleyiciye

**KURAL:** İzleyicisi olmayan hiçbir şey ÇİZİLMEZ. Render tick'e değil olaya bağlıdır;
geometri izleyicisiz durumda bir kez hesaplanır (API/ilk attach için) ama kare üretilmez.
Kalıcı animasyon, nabız/pulse/blink ve hover'a bağlı görsel durum YASAKTIR; göstergeler
statiktir (dormant işareti: tek dim satır).

**NEDEN (ölçülen):** 0-client server saniyede 62 çöp kare çiziyordu; 45 artık server toplam
**30,6 saat** CPU'yu kimsenin görmediği kareye yaktı. 2-client bağlıyken her render 30
workspace × tüm tab'ları kendi alanına sarıyordu → cols salınımı → tam scrollback reflow
(stack örneklerinin %75'i `resizeCols`). Fix'ler sonrası: ana thread %36→%20, salınım %75→%0,
izleyicisiz kare 0.

**PROS:** idle maliyeti pane sayısından bağımsız ≈0'a iner (ölçüm: 20 idle pane = %0,72 core).
**CONS:** "canlı hissiyat" için animasyon kullanılamaz — UI dili statik ayrımlara (glyph +
girinti + renk) mahkûmdur. Bilinçli takas; uzak/Tailscale doktrini de aynı şeyi emreder.

**PRIOR-ART:** macOS App Nap (occlusion-based throttling), damage-tracking gelenekli
compositor'lar. Kod içi itiraf da kayıtlı: eski süpürme kendi yorumunda kusurunu yazıyordu.

**BEKÇİ:** `frames_with_no_attached_client_are_computed_once_not_per_tick` (TP-MCF-SIZE-06),
TP-REPAINT-2B (inert hover render isteyemez), SIZE-03/04/05 testleri.

---

## RD3 · Bayt ∝ değişen hücre — trafik diff'tir, kare değil

**KURAL:** Wire'a giden şey ekranda DEĞİŞEN şeydir. Client'a `TerminalAnsi` diff'i gider;
tam kare (`SemanticFrame`) yalnız yerel/handshake yoludur. TCP dinleyici yoktur — taşıma
unix soket, uzak erişim SSH'ın işi. İçerik değişmiyorsa trafik sıfırdır ve bu test edilebilir:
aynı state iki kez çizilir, tamponlar `assert_eq!` ile karşılaştırılır — eşitse hücre-diff'i
tanım gereği hiçbir şey yollamaz.

**NEDEN:** Kullanıcı doktrini (verbatim): *"uzaktan bağlandığımızda Tailscale ile asla kasma
donma hissetmemeliyiz; güzel gözüksün diye veri trafiğini yanlış mimariyle animasyonlarla
arttırmak çok büyük aptallık olur."* Ölçülen kural: **bayt ∝ değişen HÜCRE · CPU ∝ RENDER
sayısı** — maliyet sürekliliktedir; kullanıcı eyleminin tetiklediği tek seferlik değişim
(520 hücre bile) pratikte bedavadır.

**PROS:** worst-case sürekli üretimde bile toplam wire ~2,75 MB/s'de doyuyor (ölçüldü);
tipik patlamalı kullanımda çok altı. **CONS:** diff-tabanlı hat, saat okuyan/sayaç artıran
bir çizeri AFFETMEZ — öz-değişen içerik her karede tüm bölgeyi yollar; bu yüzden "değişmediyse
trafik yok" testi her yeni çizer için zorunludur.

**PRIOR-ART:** mosh'un State Synchronization Protocol'ü (ekran DURUMUNU senkronlar, byte
akışını değil) hedeflediğimiz sınıfın kanonik örneğidir — havuzda henüz indeksli değil
(bkz. §7 boşluk listesi). tmux'un çizgisi (her şey her zaman canlı) ise bilinçli KARŞI-örnek.

**BEKÇİ:** golden-path "değişmediyse trafik yok" test deseni; eğri bayt sütunu
(`render_cost_curve` — client-tarafı sayaçlardan, wchar'dan değil).

---

## RD4 · Süreç ağacı sahiplik DEĞİLDİR — canlılığı defter söyler

**KURAL:** "Bu server'ın canlı işi var mı?" sorusunun cevabı işletim sisteminin süreç
ağacından ÇIKARILMAZ. Otorite herdr'ın kendi pane defteridir (`child_pid` +
`child_wait_completed`) ve her kritik canlılık kararı İKİ kaynakla doğrulanır: defter kaydı
VE `kill(pid, 0)`. Kayıt yoksa güvenli taraf "canlı" saymaktır.

**NEDEN (ölçülen — bu doktrinin en pahalı dersi):** `--handoff-import` ebeveynliği koparır:
devralan server PTY fd'lerini alır ama çocuklar eski server'ın çocuğudur ve o çıkınca
`systemd --user`'a reparent olur. 47 server'lık ölçümde kural "server yaşar ⟺ doğrudan çocuğu
var" olsaydı **kullanıcının canlı oturumu ölür (0 doğrudan çocuk, 60 PTY thread), ~20 çöp
demo-server yaşardı** (birinde 57 canlı fish). Taslak bu ölçümle çürütüldü ve kayda geçti.

**PROS:** çok kez güncellenmiş uzun ömürlü oturum — en çok korunması gereken şekil — güvende.
**CONS:** defter bayatlayabilir; bu yüzden ikinci kaynak (OS) zorunlu ve tek bayrağa güvenmek
yasak (tek bayrak bir kez gerçek regresyon üretti, iki-kaynak kuralıyla kapandı).

**PRIOR-ART:** kendi ölçümümüz birincil kaynak; genel ders "PID/ağaç tabanlı sahiplik
daemon'larda kırılgandır" (reparenting, double-fork gelenekleri).

**BEKÇİ:** `a_server_with_a_live_child_never_retires` (TP-SRV-RETIRE-01 — çürütülen taslağın
adıyla bekçisi), `a_handoff_pane_whose_process_is_gone_counts_as_retired` (TP-PANE-RETIRED-02).

---

## RD5 · Durum-sınıflı yaşam döngüsü — kendi kendine koşan sistem

**KURAL:** Pane sınıfları ve geçişleri kullanıcı denetimine muhtaç olmadan, politikayla
yürür: *retired (çocuğu ölmüş) + izlenmeyen + [24 saat sessiz VEYA bellek baskısı]* →
**dormant** (scrollback atomik diske, PTY actor + reader + terminal core bırakılır, pane
kimliği/etiketi/agent metadata'sı kalır) → dokunuşta uyanır (agent'lıysa `--resume` planına,
değilse geçmişi replay'li taze shell'e). Clientless + childless server 30 dk sonra kendini
emekli eder (önce dormant-all + final save) — bayrak: `idle_server_exit`, bilinçli kademeli
açılış.

İki kural BÜKÜLMEZ ve ürün sözüdür, optimizasyon parametresi değil:
1. **Canlı çocuğu olan pane'e dormancy ASLA dokunmaz** — API'yle de, sessizlikle de, bellek
   baskısıyla da. (7/24 agent = ürünün varlık sebebi.)
2. **İzlenen pane'e ASLA** — bakılan ekranı boşaltmak render hatasından ayırt edilemez.

**NEDEN + PRIOR-ART (karşılaştırmalı, PRD §7c'de tam):**

| sistem | politikası | aldığımız ders |
|---|---|---|
| tmux · zellij · wezterm | dormancy YOK; pane ölene kadar tam bedel | Gelenekte hazır cevap yok → analoji terminal dışından alındı |
| Chrome tab discarding | BASKI tetikler; sekme kalır, tıklayınca geri gelir; aktif asla atılmaz | Kullanıcı tarifinin birebir karşılığı → sessiz uyandırma + statik gösterge |
| Android LMK / App Nap | baskı-temelli; ön plan hedef değil | "ön plan" = süreç canlılığı (görünürlük değil) |
| systemd IdleAction | saf zaman | tek başına kör → baskı birincil, zaman ikincil |

**PROS:** günlerce açık laptop + unutulmuş onlarca pane senaryosunda kaynak tabanı sabit
kalır; kullanıcı rutin temizlik yapmak zorunda değildir (açık gereksinimdi). **CONS:** uyanma
tek kareden uzun sürerse hissedilir (statik "uyanıyor" işareti sözleşmesi); devirde dormancy
durumu sıfırlanır ve politika yeniden uyutur (kabul edilmiş basitlik).

**BEKÇİ:** TP-DORMANT-01..12 (canlı-çocuk baskı altında bile testli), TP-SRV-RETIRE-01..03.

---

## RD6 · Veri sessizce atılmaz — capture-first, refusal-over-loss

**KURAL:** Bir runtime bırakılmadan önce taşıdığı geçmiş YAKALANIR ve atomik yazılır
(tmp + fsync + rename); yazılamıyorsa uyutma REDDEDİLİR (`HistoryWriteFailed`) — bellekteki
tek kopyayı riske atmaktansa runtime tutulur. Yakalanabilir geçmiş varken "kullanıcı sıfırdan
da olur demişti" diye atılmaz: **izin cümlesi gereklilik değildir**; izni gereklilik gibi
okumak, kullanıcının vermediği bir kaybı ona mal etmektir. Bilinçli tek istisna: resume'la
uyanan agent pane'i transkripti kendisi yeniden çizer — replay çift gösterirdi; orada dosya
replay'siz silinir ve bu ayrım bir testin ADIYLA kayıtlıdır.

**NEDEN (yaşanmış):** `fs::write` truncate ile başlar — çökme anı yarım dosya bırakır ve uyanış
onu çöp olarak replay ederdi (GAP-B); alt-screen'de yakalama boş dönerdi ve pane veri kaybına
uyurdu (GAP-C) — önce refusal'la, sonra vendored patch 0002 ile (primary doğrudan okunarak)
kökten kapandı. Devir tarafında aynı ilkenin üç kaybı ölçülüp kapatıldı: agent-pane bastırması
(%100 kayıp), 8 KiB inline tavanı, alt-screen (%100). Dört ardışık canlı devirde toplam
scrollback 15.720→16.111→17.051→17.162 — kayıpsız.

**PROS:** "geçmişim gitti" sınıfı şikâyet yapısal olarak kapandı. **CONS:** refusal yolları
durum uzayını büyütür (her refusal bir enum + test); atomik yazım çıplak write'tan pahalıdır.
İkisi de veri kaybından ucuz.

**BEKÇİ:** TP-DORMANT-03/09/10, TP-HANDOFF-HIST-01/02/03.

---

## RD7 · Degrade-gracefully — iyileştirme bloker değildir, sürüm iki yönlü uyumludur

**KURAL:** Bir taşıma/iyileştirme katmanı çökerse davranış bir önceki bilinen-iyi hâle düşer
ve bunu SÖYLER (warn), sessizce başarısız olmaz. Freight dosyası yazılamazsa devir yine olur
(inline replay = eski davranış, bayt-aynı). Sürüm matrisi iki yönlü tasarlanır: yeni exporter
→ eski importer (dosyayı bilmez, inline'ı kullanır) ve eski exporter → yeni importer (dosya
yok, fallback) İKİSİ de tam çalışır — protokol/şema alanı değişmeden (`HANDOFF_VERSION`
sabit kaldı; taşıyıcı, soket yolundan türetilen yan-dosyadır).

**NEDEN:** Manifest borusu byte-başına okunur (fd geçişi paylaşımlı stream'i tamponlatmaz;
ölçüm: **0,77 MiB/s**, tamponlu muadili ~1450 MiB/s) → inline bütçe BÜYÜTÜLEMEZ; 15 MiB
manifest 20 sn okuma = READY_TIMEOUT'a komşu. Çözüm bu kısıtın etrafından tasarlandı: büyük
veri diskte, boru yalnız metadata.

**PROS:** kısmi güncellenmiş filoda (eski+yeni karışık) hiçbir kombinasyon kırılmaz; iyileştirme
katmanı korkusuzca evrilir. **CONS:** fallback yolu ölü koda dönüşebilir — bu yüzden inline
bütçenin kırpımı ayrı testle pinli.

**PRIOR-ART:** protokol evriminde "ignore-unknown + sidecar taşıma" deseni; guard'larda
fail-open tercihi (yanlışlıkla kapanan kapı, kaçan tek vakadan pahalıdır — wt guard aynı ilke).

**BEKÇİ:** `a_missing_or_corrupt_freight_file_degrades_to_inline_replay`,
`handoff_history_ansi_full_keeps_what_the_inline_budget_drops`, sürüm-gate freight testleri.

---

## RD8 · Sözleşme > gözlem — ve davranış = kayıt

**KURAL:** Bir guard'ın ölçütü niyetinin proxy'siyse, proxy'yi geçerli kılan varsayım kayda
yazılır ve varsayımın bir SÖZLEŞMEYE mi (upstream API dokümanı, ölçülü sabit) yoksa bir
GÖZLEME mi dayandığı ayrılır. Sözleşmeye dayanıyorsa savunma katmanı EKLENMEZ (ölü koşul +
yanlış gerekçeli kayıt üretir); sözleşme değişirse vendored-patch kapıları haber verir.
Ve her fork davranışı bir TP satırı + adlı testle kayıtlıdır — kayıtsız davranış, bir sonraki
merge'in sessizce silebileceği bir şeydir (yaşandı: sessiz relicense vakası).

**NEDEN (taze örnek):** "alt-screen'de `max_offset_from_bottom` hep 0" bir gözlem sanılıyordu;
upstream'in kendi API doc'u bunun yazılı sözleşme olduğunu söylüyor (*"If the terminal has no
scrollback (e.g. the alternate screen is active), the viewport always remains on the active
area"* — bindings.rs). Bu ayrım, scroll-guard'ına gereksiz `!alternate_screen` koşulu
eklemekten kurtardı.

**BEKÇİ:** `behaviors/` registry + `behavior_registry_check` (testi olmayan marker build'i
düşürür); vendored patch reverse-apply kapısı.

---

## RD9 · Error-handling doktrini (özet)

- **`unwrap()`/`expect()` üretim kodunda yasak** — `Result` + `?`; hatalar `tracing` ile.
- Hatalar üç sınıfa ayrılır ve sınıfına göre davranır:
  1. **Bloker** (devir el sıkışması, protokol): açık hata, rollback yolu (`rollback_handoff_
     before_commit` TÜM hata yollarının geçtiği tek nokta — ölçülerek doğrulandı).
  2. **İyileştirme** (freight, gösterge): warn + bilinen-iyi davranışa düş (RD7).
  3. **Veri-riski** (dormancy yazımı, capture): REFUSAL — işlemi yapma, kaynağı koru (RD6).
- **Atomik dosya deseni** her kalıcı yazımda: tmp + `sync_all` + rename (repo'nun kendi
  emsalleri: update.rs, logging.rs, product_announcements.rs — desen icat edilmedi, uyuldu).
- **Tek-okuyucu / consume semantiği**: taşıma dosyaları (freight, dormant history) okunduğu
  yerde silinir; parse başarısız olsa bile silinir — çöp birikmez, çift-replay olmaz. Çift
  çökme artıkları pid-damgalı ada dayanan süpürmeyle temizlenir.
- **Sessiz kısıt yok**: kapsam daraltıldıysa/metrik alınamadıysa raporda söylenir (eğri
  raporundaki "wchar 0 okudu" itirafı bu sınıfın örneğiydi; sonra kökten düzeltildi).

---

## §4 · Ölçülmüş sayılar (referans tablosu — tarihli, bayatlayabilir)

| ölçüm | değer | tarih |
|---|---|---|
| İdle pane CPU | 0,036 %/pane (20 pane = %0,72) | 2026-08-16 |
| 20 üretken pane (400 satır/s) tabanı | %13,2 | 2026-08-16 |
| Client eğrisi kırılması | N≈2'de ~1 çekirdek (%97,6); plato %104-106 | 2026-08-16 |
| Frame paylaşımı | 24,4 → 6,4 fps/client (N=1→10); toplam ~64 f/s sabit | 2026-08-16 |
| Wire baytı (worst-case) | 0,67 → 1,35 → ~2,75 MB/s plato (N=1→2→4+) | 2026-08-17 |
| Manifest borusu | 0,77 MiB/s (byte-başına read; tamponlanamaz) | 2026-08-16 |
| SCM_RIGHTS çekirdek tavanı | 253 fd/mesaj (254=EINVAL); herdr tavanı 128 | 2026-08-16 |
| 46 idle shell (v2 hedefi) | 141 MiB RSS + 46 PTY fd; CPU ≈ 0 | 2026-08-16 |
| Devir zinciri koruması | 4 ardışık canlı devirde scroll 15.720→17.162, kayıp 0 | 2026-08-17 |
| Due-geçmiş deadline spin'i (canlı, 27 pane + 15 agent) | 167.000 boş tur/s × ~5 µs = ana thread %98 → clamp sonrası 119 tur/s, %3,5 | 2026-08-25 |
| Housekeeping pass kapısı (10 ms) | sel altında 179.721 tick/s'te pass ≤99/s; idle'da ek uyanış 0 (tick 4-6/s değişmedi) | 2026-08-25 |
| Housekeeping tur maliyeti | kapı öncesi ~300 µs/tur (display surface-swap dahil) → kapı sonrası tur 5 µs | 2026-08-25 |

Reçeteler: kanonik tam-koşum PRD §7e · izole idle taban PRD §16 (soket yolu ≤108 bayt!) ·
eğri: `cargo nextest run -E 'test(render_cost_curve)' --run-ignored ignored-only --no-capture`.

## §5 · Süreç arşivi — bu mimari hangi hatalardan öğrendi (özet; detay PRD §13 + devirler)

| olay | ders → doktrine girdiği yer |
|---|---|
| Aynı soruya 4 cevap, 3'ü yanlış | ölçüm koşulu sonucun parçası → RD0 |
| "server yaşar ⟺ doğrudan çocuk" taslağı canlı oturumu öldürecekti | defter + çift kaynak → RD4 |
| Bayrak açıldı, aday 0 çıktı (46/46 canlı shell) | bayrak ≠ etki; adaylık koşulunu ölç → RD0/RD5 |
| Seed-önce-replay-sonra: taşınan geçmiş alternate ekrana akacaktı (entegrasyon testi İLK koşumda yakaladı) | uykudaki yolu canlandıran değişiklik latent kusuru yeni gösterir; test noktaları koddan önce → RD0/RD6 |
| `fs::write` yarım dosya; alt-screen boş yakalama | atomik yazım + refusal-over-loss → RD6 |
| 8 KiB inline tavanı büyütülemedi (0,77 MiB/s) | kısıtın etrafından tasarım: sidecar freight → RD7 |
| Alt-screen "max=0" gözlem sanıldı, sözleşme çıktı | sözleşme > gözlem → RD8 |
| Sessiz relicense (üç-yollu merge uyarısız aldı) | davranış = kayıt (TP + registry) → RD8 |
| Karşılayıcısı olmayan due-geçmiş randevu döngüyü 167k tur/s boş uyandırdı; ptrace örneklem profili iki kez yanlış suçlu gösterdi | uyanış, işleyicinin kadansına clamp'lenir + suçlu atfı in-process sayaçla (prof marker) → RD2 + `headless-loop-cadence.md` |

## §6 · YENİ FEATURE KONTROL LİSTESİ — tasarım aşamasında cevaplanır, PR'da değil

Her yeni bileşen/feature için sekiz soru; "bilmiyorum" = önce ölç:

1. **Eksen:** Bu iş Dikkat/Canlılık/Etkinlik eksenlerinin hangisinde maliyet ekliyor? (RD1)
2. **İzleyicisiz maliyet:** Kimse bakmıyorken bu kod KAÇ KEZ koşar? Cevap "tick başına" ise
   tasarım reddedilir; olaya bağlanır. (RD2)
3. **Trafik:** Wire'a ne zaman bayt yazar? İçerik değişmeden yazıyorsa (saat, sayaç,
   allocation-sıralı map) reddedilir. "Değişmediyse trafik yok" testi yazılır. (RD3)
4. **Animasyon:** Kalıcı animasyon/pulse/hover-görseli var mı? Varsa statik alternatife
   çevrilir. (RD2)
5. **Canlılık kararı:** Süreç ağacına mı defterlere mi dayanıyor? Tek kaynak mı? (RD4)
6. **Veri:** Bu iş herhangi bir durumda kullanıcı verisini (scrollback, oturum, metadata)
   atıyor/kırpıyor mu? Atıyorsa: yakalanabilir miydi? Refusal mı degrade mi? (RD6/RD7)
7. **Sürüm:** Eski sürümle her iki yönde karşılaşınca ne olur? (RD7)
8. **Kayıt:** Hangi TP satırı + adlı test bu davranışı sahiplenecek? Sayı probe mu tavan mı,
   satırda ayrılmış mı? (RD8)

Bir feature bu listeyi geçemiyorsa PRD'ye "bilinçli istisna + gerekçe + geri-alma koşulu"
yazılmadan ilerlemez.

## §7 · Prior-art indeks durumu ve BOŞLUKLAR (geliştirilecek altyapı için)

**İndeksli (refpool, FTS ~0,6 sn):** zellij, wezterm(-ssh), mprocs, sshx, tenex, orbt, oly,
kode-bridge, shell-compose, gwm-cli, ratatui ekosistemi — PTY yaşam döngüsü / IPC taşıma /
protokol el sıkışması / stale-socket konseptleri kapsanıyor (18 konsept).

**⚠ İNDEKSLENMESİ GEREKEN BOŞLUK — trafik-verimlilik sınıfı (bu doktrinin RD3 alanı):**

| aday | neden kritik |
|---|---|
| **mosh** | State Synchronization Protocol: ekran DURUMUNU senkronlar; diff-wire hedefimizin kanonik referansı |
| **eternal-terminal** | TCP üstü kalıcı oturum + yeniden bağlanma ekonomisi |
| **shpool** | Rust, "tmux'tan hafif" oturum kalıcılığı — minimal-kaynak kıyas tabanı |
| **tmux (çekirdek)** | karşı-örnek olarak: her-şey-canlı modelinin maliyet yapısı |
| **dtach / abduco** | sıfıra yakın taban maliyetli detach — alt sınır kıyası |

Bu beşi `prior-art-pool` akışıyla indekslenip sınıflandırılmalı (lisans notuyla: yalnız
tasarım referansı; kod kopyalama lisans kapısından geçer).

## §8 · Açık gap'ler — doktrinin kendisinin bildiği eksikler

| gap | durum |
|---|---|
| v2 idle-shell (141 MiB + 46 fd) | tasarım hazır (`.local/prd/idle-shell-dormancy-v2.md`), ⛔ kullanıcı onayı bekliyor (SIGHUP geri alınamaz) |
| `idle_server_exit` bayrağı | kod canlıda, kapalı; dormancy gözlemi sonrası kullanıcı açar |
| 10-client worst-case akıcılığı (6,4 fps/client) | ayrı eksen: render hattında retained/damage payını büyütmek; dormancy bunu ÇÖZMEZ (ölçüldü) |
| Perf regresyon bütçesi | eğri elle koşuluyor (#[ignore]); "idle server > X% ise kırmızı" sınıfı otomatik bütçe testi YOK |
| Canlı trafik gözlemlenebilirliği | per-client bayt/frame sayaçları API'de yok (T-D ailesi: `dormant` + `alternate_screen` alanlarıyla birlikte) |
| Dormant dosya bütçesi | wake/close/restore tüketiyor; çok uzun ömürlü oturumda toplam-boyut tavanı/yaş GC'si tanımsız (ölçülmedi) |
| 10-client × housekeeping kombinasyonu | pass başına client-sayısı kadar display defter-değişimi; 10 client'ta ~1000 swap/s TEORİK — canlıda ölçülmedi. Görev: HP'de 10-sahte-client + çıktı seli stres lab'ı → sayılar §4'e (`headless-loop-cadence.md`) |
| Windows | devir/freight/dormancy-capture unix-only (kayıtlı sınır; refusal non-unix'te yaşıyor) |

## §9 · Agent onboarding (harness köprüsü)

`CLAUDE.md`/`AGENTS.md` **upstream dosyalarıdır — düzenlenmez**; bu dosya onları tamamlar.
Bir agent bu repoda kaynak/trafik alanına dokunacaksa sırası:

1. **codebase-mcp İLK** (`index_status` tazelik → `get_architecture` → ilgili sembollere
   `search_graph`/`trace_path`) — proje haritası/ilişkisel ağ metin aramasından önce gelir.
2. Bu dosya (RD0-RD9 + §6 kontrol listesi) + `docs/patterns/rust-engineering.md` (HP1-HP18).
3. `behaviors/shared-surfaces.md` ilgili TP ailesi — davranış eklenecekse TP satırı + test
   AYNI commit'te; sayılar SABİTTEN okunur (kayıttaki sayı testin probe'u olabilir).
4. Oturum durumu: `.local/CURRENT-HANDOFF.md` (kanonik pointer, gitignored) → hattın devri.
5. Görevler: hattın TASKS dosyası (`.local/TASKS-*.md`) — TaskCreate aracı varsa oraya AYNEN
   aktarılır; yoksa dosya kanonik listedir ve her durum değişiminde güncellenir.
6. Git disiplini: task başına dal → `wt.sh claim` → RED-önce → kapı (`just check` +
   registry) → iniş yalnız setsid'li `wt.sh auto` → kanıt yalnız `merge-base --is-ancestor`.
   ⭐ Bu makinede iniş = canlıya çıkış (auto-deliver) — bilerek indirilir, teslim doğrulanır.

---
*v1.0.0 — 2026-08-17 · Kaynaklar: pane-runtime-lifecycle PRD §1-17 (ölçümler executable,
conf 0.95) · idle-shell-dormancy-v2 PRD · behaviors/shared-surfaces.md TP kayıtları ·
Chrome tab discarding / App Nap / LMK / systemd IdleAction (official docs, conf 0.9) ·
ghostty upstream API sözleşmesi (source, conf 0.95) · mosh SSP (paper, conf 0.9 — havuza
indekslenecek). Kardeş: rust-engineering.md (kod katmanı) · bu dosya (çalışma-zamanı ekonomisi).*
