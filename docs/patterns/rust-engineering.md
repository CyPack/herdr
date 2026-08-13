---
doc: rust-engineering
scope: >
  herdr'ın PROJE-SPESİFİK Rust mühendislik pattern kataloğu (HP1–HP10). Global katman
  (~/.claude/skills/rust-dev/references/architecture-patterns.md, P1–P25) her Rust projesinde
  geçerli; BU dosya herdr'a özgü somutlamaları ve herdr'a özel pattern'leri ekler ve herdr
  işlerinde global katmanı ÖNCELER.
created: 2026-07-12
status: canonical (lokal — /docs/* gitignored, upstream'e sızmaz)
agentic_triggers:
  - "herdr feature · herdr bug fix · herdr refactor · sidebar · pane · workspace · tab"
  - "detection manifest · agent detection · libghostty · vendored patch"
  - "protocol version · wire.rs · integration version · characterization test"
  - "nextest · just check · flaky test · cargo test fail"
related:
  - docs/references/README.md                     # etiket sözlüğü + tier/confidence tabloları
  - ~/.claude/skills/rust-dev/                    # global Rust katmanı
  - AGENTS.md                                     # birincil kaynak (UPSTREAM DOSYASI — DÜZENLEME)
  - .local/CURRENT-HANDOFF.md                     # aktif iş durumu
---

# herdr Rust Engineering Patterns (HP1–HP10)

> Her pattern: **ne / ne zaman KULLAN / ne zaman KULLANMA / kaynak-etiket · confidence**.
> Etiketler `docs/references/README.md`'de çözülür. AGENTS.md satır numaraları 2026-07-12
> snapshot'ına göredir (upstream değişirse yeniden doğrula).

## HP1 — AppState saflığı: state ≠ runtime
- **Ne:** `AppState`/`Workspace` saf veri; `PaneState` ≠ `PaneRuntime`. Domain logic PTY'siz,
  async'siz test edilir (`AppState::test_new()`, `Workspace::test_new()`).
- **KULLAN:** her yeni state alanı/davranışı — önce "PTY'siz test edilebilir mi?" sorusu.
- **KULLANMA:** runtime-yüzeyli glue kodunu zorla saflaştırmaya çalışma; sınırı P5/HP2 ile çiz.
- Kaynak: `[herdr-agents]` AGENTS.md:28,94 · conf 0.9

## HP2 — Runtime/client sınır sınıflandırması (her yeni alan/event için ZORUNLU kapı)
- **Ne:** eklemeden önce sınıflandır: shared-runtime-fact → server state + JSON API/event yolu;
  TUI-presentation → sadece client. Private TUI socket'ine YENİ paylaşılan davranış ekleme.
  API adlandırması nötr (sidebar/row/card/widget YASAK).
- **Örnek ayrım:** pane/agent metadata, process/terminal state, event'ler = server;
  sidebar layout, seçim, renk, modal, mouse/viewport = client.
- Kaynak: `[herdr-agents]` AGENTS.md:36-51 · conf 0.9
- **Somut vaka:** Projects sidebar işi TUI-presentation olarak client'ta tutuldu (Task#1-13,
  `[herdr-handoff]`) — doğru sınıflandırma örneği.

## HP3 — Test döngüsü: nextest zorunlu, `just` öncelikli
- **Ne:** `just test` / `just check` (= fmt-check + nextest + maintenance-script testleri).
  Doğrudan runner gerekiyorsa `cargo nextest run --bin herdr`.
- **NEDEN nextest:** `cargo test` paralel-thread + global `CONFIG_PATH_ENV_VAR` paylaşımı →
  ~14 sahte FAIL (kanıt: seri 2531/2531 vs nextest 2533/2533 — `[herdr-discipline-memory]` · conf 0.85).
- **Commit kapısı:** `just check` geçmeden commit YOK; başarısız check bypass edilmez, ya düzelt
  ya neden dar kapsamın yeterli olduğunu açıkça yaz (`[herdr-agents]` AGENTS.md:92 · conf 0.9).
- **Build ön-koşulu:** `export PATH="$HOME/.local/bin:$PATH"` (zig 0.15.2 — libghostty-vt).

## HP4 — Refactor-risk protokolü (characterization + adversarial invariants)
- **Ne:** 2+ core surface / persisted state / protokol-ID / identity / restore-handoff / detection
  authority / state-projection dokunan iş = refactor-risk. Önce korunan davranışı adlandır +
  characterization test; identity/state işlerinde `AppState::assert_invariants_for_test()` /
  `Workspace::assert_invariants_for_test()` + `test_with_adversarial_identity_state()` fixture'ları.
  Geniş refactor'da roundtable.
- **KULLANMA:** rutin lokal fix'te tören olarak (AGENTS.md açıkça "not for routine local fixes").
- Kaynak: `[herdr-agents]` AGENTS.md:96 · conf 0.9

## HP5 — Lint zinciri: fmt-check → clippy -D warnings → (pre-commit hook)
- **Ne:** `just lint` = `cargo fmt --check` + `cargo clippy --all-targets --locked -- -D warnings`;
  `.githooks/pre-commit` fmt-check koşar (`just install-hooks`). `clippy.toml`:
  `too-many-arguments-threshold = 11` (bilinçli proje eşiği).
- Kaynak: `[herdr-justfile]` · conf 0.9 · `[herdr-clippy-cfg]` · conf 0.95

## HP6 — Cross-compile lint: Windows hatalarını Unix'te yakala
- **Ne:** `just windows-lint` → `x86_64-pc-windows-msvc` hedefine clippy. `#[cfg(windows)]`
  dallarındaki tip/import hataları CI beklemeden lokal yakalanır.
- **KULLAN:** platform-gated kod dokunan HER değişiklikte (P4 ile birlikte).
- Kaynak: `[herdr-justfile]` windows-lint recipe · conf 0.9

## HP7 — Commit/release disiplini
- **Ne:** lowercase conventional commit, emoji YOK, AI co-author YOK; issue bağlantısı `refs #N`
  (fixes/closes YASAK — master unreleased taşır, release CI kapatır). Commit ÖNCESİ mesaj öner +
  hizalan. CI `conventional-commits` job'u subject/PR-title'ı doğrular.
- Kaynak: `[herdr-agents]` AGENTS.md:163-177 · conf 0.9 · `[herdr-ci]` · conf 0.9
- **Fork notu (bu makine):** external-contributor guardrail aktif (hesap ≠ ogulcancelik) —
  upstream'e push/issue/PR YASAK; upstream katkı = kullanıcı elle Discussion (HP10).

## HP8 — Vendored-patch izlenebilirliği (libghostty-vt)
- **Ne:** vendored kaynak üstü her patch: `vendor/libghostty-vt.patches.md` index'inde +
  `vendor/patches/libghostty-vt/` altında dosya olarak; neden/issue/upstream-PR/base-commit/
  verification/kaldırma-koşulu yazılı. `just check` index-listeli VE reverse-apply-temiz doğrular.
  Upstream güncellemesinde her aktif patch tek tek kontrol edilir.
- Kaynak: `[herdr-agents]` AGENTS.md:141-149 · conf 0.9 · `[herdr-vendor-index]` · conf 0.9

## HP9 — Protokol/entegrasyon versiyon kuralı (release-göreli bump)
- **Ne:** `src/protocol/wire.rs::PROTOCOL_VERSION` SON RELEASE tag'ine göre karşılaştırılıp
  bump edilir (source zaten ileriyse tekrar bump YOK); `*_INTEGRATION_VERSION` sabitleri
  per-commit sayaç DEĞİL, release-göreli migration versiyonu. Test fixture'larındaki hardcoded
  protokol beklentileri birlikte güncellenir.
- Kaynak: `[herdr-agents]` AGENTS.md:184-185 · conf 0.9

## HP10 — Fork/contribution guardrail + docs yerleşimi
- **Ne:** (1) `website/src/content/docs/` = released docs, unreleased davranış YAZILMAZ;
  unreleased → `docs/next/`. (2) Root README/CHANGELOG/website-latest.json normal işte
  DÜZENLENMEZ. (3) Lokal PRD/plan/keşif → `.local/` (gitignored, locally controlled).
  (4) /docs/* gitignored → BU katalog + references lokal yaşar, PR'a giremez (bilinçli).
- Kaynak: `[herdr-agents]` AGENTS.md:151-161,231-238 · conf 0.9 · `[herdr-contributing]` · conf 0.9

## HP11 — Cursor, activation, preview ve render authority'sini ayır
- **Ne:** Dikey gezinme yalnız exact owner içindeki cursor identity'yi değiştirir;
  entry türüne bakıp implicit activation yapmaz. Directory activation explicit
  Right/`l`/Enter command'ıdır. Primary click exact row focus/select intent'idir;
  directory için child focus vermeden bounded preview hazırlayabilir. Cursor-follow preview fallible I/O ise bounded
  latest worker'a gider ve generation/source/owner/index/path/current-cursor
  eşleşmeden apply olmaz. Clamped/stale/coalesced input render yetkisi vermez.
- **NEDEN:** Cursor reducer'ın `activate_entry` çağırması directory landing'de
  active column transferi üretir; aynı burst'ün kalan input'u child üzerinde
  çalışır. Async sonucu yalnız filesystem generation ile doğrulamak da kullanıcı
  yatay focus değiştirdikten sonra stale preview'nin authority çalmasına izin
  verir.
- **OWNER LAW:** `deepest()` resident/prepared data extent'idir, focus değildir.
  İlk preview entegrasyonu parent owner'ı geri yükler; Trail render, auto-follow,
  hit-test, resize compatibility projection ve watcher binding aynı
  `active_col()` authority'sini kullanır.
- **ÖLÇÜM:** Terminal event multiplicity ile reducer multiplicity ayrı ölçülür.
  Herdr Ghostty kanıtında 333 vertical packet'in 226 aynı-yön delta'sı `<2 ms`
  aynı-coordinate triplet/sextuplet idi; normalization yalnız bu exact kimlikte
  uygulanır. Reversal/owner/coordinate/`>=2 ms` korunur.
- Kaynak: `[herdr-fmn-2026-07-21]` source+RED/GREEN+isolated trace · conf 0.95

## HP12 — Directional navigation ile generic activation'ı eşitleme
- **Ne:** Miller Left/Right command'larını entry activation shortcut'ı olarak
  modelleme. Left varsa tam bir resident parent edge geçer; root'ta inerttir.
  Right/`l` yalnız exact cursor entry directory ise tam bir child edge geçer
  veya bounded activation intent üretir. File/non-entry/stale/boundary üzerinde
  model, worker, focus ve render açısından inerttir. Enter explicit activation;
  primary click ise file/directory için exact cursor-focus surface'idir.
- **NEDEN:** Directory dispatch başarısız olunca generic `activate_entry`
  fallback'ine düşmek, file üzerinde Right basışını `SelectedFile` mutation'a
  dönüştürür; resident child'ı truncate eder ve görünür yatay hareket olmadığı
  halde render ister. Mapping'in var olması interaction contract'ın doğru
  olduğu anlamına gelmez.
- **TEST LAW:** Reducer dispatch + App render authority birlikte test edilir.
  Üç veya daha derin resident chain'de her event en fazla bir kolon ilerler;
  root/deepest/file/stale sınırları inerttir; nonresident directory yalnız
  exact identity'li bounded completion ile focus alır. Enter ve primary-click
  regression matrix'leri ayrı korunur.
- Kaynak: `[herdr-fmh-2026-07-22]` behavioral RED + 3/3 + 10/10 + 190/190 · conf 0.98

## HP13 — Nested authority katmanlarını birbirinden türetme
- **Ne:** Prepared data extent, selected identity, active Miller column,
  top-level Files region focus, visual projection ve destructive action
  authority ayrı state eksenleridir. Bir eksenin dolu veya görünür olması
  diğerine yetki vermez. Son kabul edilmiş canlı intent top-level owner'ı
  değiştirebilir; stale frame, resident preview, coalesced input veya eski
  enabled geometry değiştiremez.
- **INPUT LAW:** Typed current-frame validation önce gelir. Kabul edilmiş Trail
  click/wheel/body intent'i Rail → Trail transferi yapar; clamped cursor hareketi
  bile yalnız owner değiştiyse render ister. Rejected/coalesced/stale intent hem
  model hem authority bakımından inerttir.
- **ACTION LAW:** Paint-time action model yalnız bir capability snapshot'ıdır.
  Copy/Paste/New Folder/Delete, context/plugin/rename ve worker admission
  sınırlarının tamamı current `AppState` içindeki Trail owner'ı tekrar doğrular.
  Rail owner altında resident Trail selection metadata korunabilir ama sıfır
  operation authority taşır.
- **VISUAL LAW:** Güçlü filled cursor yalnız current top-level owner'ın geçerli
  row'unda ve Trail için yalnız `active_col()` içinde görünür. Accepted origin,
  resident ancestor, multi-selection ve hover ayrı, daha zayıf stillerdir.
- Kaynak: `[herdr-ffo-2026-07-22]` accepted/stale input RED/GREEN + destructive
  boundary tests + deterministic VIS-26/VIS-27 · conf 0.99

## HP14 — Primary pointer focus ile hierarchy activation'ı ayır
- **Ne:** Bir Miller row'a unmodified primary click, entry file veya directory
  olsa da exact `(column,index,path)` cursor'ını ve top-level Trail owner'ını
  aynı transaction'da o satıra taşır. Directory click `TrailActivate` göndermez;
  hazır owner projection'ını disk-free kurar ve gerekirse yalnız `TrailPreview`
  gönderir. Child focus yalnız Right/`l`/Enter ile geçer.
- **ASYNC LAW:** Preview completion resident child'ı ekleyebilir/değiştirebilir,
  fakat current generation/source/owner/index/path/cursor eşleşmeden apply olamaz
  ve `active_col()` değerini owner'dan child'a taşıyamaz. Backpressure/failure
  clicked highlight'ı geri alamaz.
- **VISUAL LAW:** Accepted click'in hemen ardından ve preview tamamlandıktan
  sonra aynı exact owner row güçlü filled cursor'dır. Sonraki Up/Down aynı
  kolonda bir satır ilerler; Right aynı dispatch'te child ilk actionable row'u
  güçlü cursor yapar.
- **TEST LAW:** RED'i hem preview öncesi hem sonrası active-column üzerinde kur;
  exact stale identity, no-filesystem-read, rapid latest-preview, hidden-child
  viewport ve Right-first-child failure path'lerini birlikte koru.
- Kaynak: `[herdr-dclick-2026-07-23]` graph root-cause + reducer/App RED/GREEN +
  145/145 input/invariant gate · conf 0.99

## HP15 — Refresh, navigasyon DEĞİLDİR: cursor authority'sini yeniden projeksiyonda koru
- **Ne:** Bir kolonu diskten tazeleyen yol (watcher/reconcile) projeksiyonu
  **cursor authority**'sinden kurar: `TrailState::cursor_path_in_col(col)` =
  dikey override kazanır, aktive edilmiş `selected` yalnız YEDEKTİR. Detay
  paneli de odaklı satıra hazırlanır. `select_file` → `mark_selection`
  **cursor override'ı siler**; refresh bunu çağırıyorsa override önce saklanır,
  satır hâlâ diskteyse geri konur.
- **NEDEN:** Tık `cursor`'ı taşır ama `selected`'ı taşımaz (HP14 gereği).
  Refresh `selected`'ı okursa 2 saniyelik reconcile her tıktan sonra odağı
  aktive edilmiş satıra — yeni açılmış kolonda ilk satıra — geri çeker.
  Kullanıcıya görünen: "nereye tıklarsam tıklayayım 2-3 saniye sonra odak en
  üste gidiyor". Kod yıllardır böyleydi; kusuru **görünür kılan**, server
  modunda hiç çalışmayan reconcile'ın headless zamanlayıcıya bağlanmasıydı.
- **LATENT-DEFECT LAW:** Uykuda bir yolu canlandıran her düzeltme (scheduler
  parity, feature flag, yeni surface) o yolun kusurlarını *yeni* regresyon gibi
  gösterir. Suçu son commit'te aramadan önce "bu kod yolu daha önce hiç
  çalıştı mı?" diye sor.
- **TEST LAW:** RED'i gerçek zaman çizgisiyle kur — önce tık, sonra reconcile
  deadline'ı (`WATCH_RECONCILE_INTERVAL`), sonra imleç+detay assert'i. Uçuştaki
  klon senaryosunu düzeltmeye çalışma: apply girişindeki canlı-source guard'ı
  onu zaten reddediyor (ölçüldü, ilk hipotez yanlıştı).
- Kaynak: `[herdr-fmrefresh-2026-07-26]` RED(cursor 0 vs 2)/GREEN + 4190/4190 ·
  `TP-FMW-REFRESH-05` · conf 0.97

## HP16 — Popup pane tab surface'in ÜYESİ DEĞİLDİR
- **Ne:** `app.popup_pane` ayrı bir eksendir; `surface.pane_infos` içinde
  görünmez ve `PaneInfo`'su yoktur. Pane başına dönen HER süpürme (kitty
  grafik yerleştirme, görünürlük kontrolü, benzeri) popup'ı sessizce atlar.
  Popup açıkken **resim katmanının sahibi popup'tır** — girdi (tuş önceliği) ve
  render (`render_popup_pane` koşulsuz çizilir) sahipliğiyle aynı kural.
- **NEDEN:** Popup'ta çalışan `herdr view` kitty dizilerini kendi PTY'sine
  yazar; toplanmazsa metni görünür, resmi görünmez. Altındaki yüzeyin resmi de
  toplanmaya devam ederse popup'ın ÜSTÜNE boyanır — yani "yanlış resim" ile
  "hiç resim yok" aynı kusurun iki yüzüdür.
- **KAPSAM LAW:** Yeni bir per-pane geçişi yazarken üç sahipliği birlikte
  sor: pane_infos üyeleri · file-manager surface · popup. Biri eksikse
  davranış yüzeye göre sessizce değişir.
- Kaynak: `[herdr-popupgfx-2026-07-26]` kaynak-okuma + `TP-FPOPUP-01` +
  4190/4190 · conf 0.93

## HP17 — Eklenti kaydı manifest'in SNAPSHOT'ıdır
- **Ne:** `plugins.json` kurulum anındaki eylem listesini saklar; çalışma
  zamanı `manifest_path`'i yeniden OKUMAZ. Manifest'i düzenlemek yetmez —
  `herdr plugin link <yol>` ile yeniden bağlanmalıdır.
- **NEDEN:** Manifest'te doğru görünen bir eylem canlıda yokmuş gibi davranır
  ve hata kodda aranır. Teşhis tek satır:
  `python3 -c "import json;[print(p['plugin_id'],[a['id'] for a in p.get('actions',[])]) for p in json.load(open('<config>/plugins.json'))]"`
- **ORTAM LAW:** Kayıt profil-kapsamlıdır (`herdr` vs `herdr-dev`) ve
  `XDG_CONFIG_HOME`'a bağlıdır. Bir profilde bağlanan eklenti diğerinde yoktur;
  `~/Downloads/herdr-test/baslat.sh` bu yüzden her açılışta yeniden bağlar.
- Kaynak: `[herdr-pluginreg-2026-07-26]` iki config'in kayıt farkı ölçüldü ·
  conf 0.9

## HP18 — Ekran-yerel her yüzey SINIFLANDIRILIR (yeni `AppState` alanı için ZORUNLU kapı)

> **ANAYASA (kullanıcı iron kısıtı, 2026-08-09):** *"Bir display'de yaptığım şey diğer
> display'deki client'e ASLA müdahale etmemeli."* Her yeni alanda, koda dokunmadan ÖNCE
> sorulacak soru: **"Bu alan bir EKRANIN mı, yoksa OTURUMUN mu gerçeği?"** Cevap
> tasarım aşamasında verilmezse iki kez ödenir (G2 reveal'ı iki kez elden geçti).

- **Ne:** `AppState`'e alan eklemek her zaman bir sınıflandırma kararıdır.
  Alan ya oturum gerçeğidir, ya config'tir, ya da ekran-yerel sunumdur —
  üçüncüsüyse `client_surfaces!` makrosunun **dört grubundan birine** girer:

  | Grup | Oturum kendi başına değiştirdiğinde kim görür | Örnek |
  |---|---|---|
  | `inherited` | Yalnız default (sonra bağlanan devralır) | `active`, workspace seçimi |
  | `broadcast` | **Her ekran** | `mode`, menü, diyalog, prompt, rail, kaydırma |
  | `owned` | Kimse — ama tek ekranlıyken oturumla aynı slot | `stage`, `file_manager` |
  | `ephemeral` | Kimse, hiçbir koşulda | drag, press, selection, bloke edici picker |

- **NEDEN (dört ayrı hata sınıfı, dördü de yaşandı):**
  1. **Sınıflandırmama** → alan sessizce paylaşılır; bir ekranda menü açmak
     hepsinde açar. Semptom render hatası gibi görünür, oysa eksik olan takas.
  2. **Her şeyi `inherited` yapmak** → API bir pane'e odaklanıp oturumu terminal
     moduna alır, ekran navigate'te kalır ve **kullanıcının yazdığını yutar**.
     Hiçbir birim test yakalamaz; iki-client testi yakalar (TP-SUR-BROADCAST-01).
  3. **`broadcast`'i her şeye uygulamak** → workspace seçimi de yayılır ve
     ekran-başına odak tamamen çöker (TP-SUR-BROADCAST-02).
  4. **Karşılaştırılamayanı terfi ettirmek** → dizin listesini her park'ta
     yürümek demektir: ekran × kare maliyeti. Karşılaştırılamayan `owned`'dır.
  5. **Kardeş setleri farklı evlerde bırakmak (G7 dersi, 2026-08-10)** → alanın
     kendisi doğru sınıflandırılmış olsa bile, AYNI kavramsal yüzeyin bir başka
     seti oturum evinde kalırsa bir ekranın aksiyonu diğerinin görünümünü iteler.
     Yaşandı: `collapsed_space_keys` per-display'di ama `expanded_chat_workspaces`
     oturum alanıydı — aktivasyon reveal'ı oraya yazınca "çekmeceler kendi kendine
     açılıp kapanıyor" bildirimi geldi. Kural: bir ekran-görünümü ÜRETEN her
     türetim yalnız per-display setlerden okur ve yazan her aksiyon aynı eve
     yazar (`chat_drawer_collapsed` + TP-DRAWER-08 emsali).

- **İhlal SANMA (yaşayan-ekran nüansı):** MD sözü YAŞAYAN ekranlar arasındadır.
  İlk kez görülen bir ekran, oturumun sürüldüğü default'u BİLEREK devralır
  (TP-SUR-DEFAULT-01) — "yeni taktığım monitör son durumu gösterdi" bir sızıntı
  değil, tasarımdır. Kanıt deseni: her iki ekranı ÖNCE tanıt, SONRA birinde
  eylem yap (TP-DRAWER-08 fixture'ı).

- **DEĞİŞMEZ:** *Tek ekran varken oturum ile o ekran aynı şeydir ve tek slotu
  paylaşırlar.* Demet park/terfi kuralı, worker anahtarı ve benimseme kuralının
  üçü de buna dayanır. Delinirse her monolitik koşum ve her test bozulur.
  "Hangisi tek ekran" sorusu, haritaya **ekleme yapılmadan ÖNCE** sorulmalıdır;
  sonra sorulursa yalnız ekranın durumu bir daha bakılmayacak slota park edilir.

- **Makro sözleşmesi:** alan tek yerde bildirilir, kaydet/yükle çifti oradan
  üretilir → **yarım göç derlenmez**. Bu, düzen değil garantidir: 80+ alanda
  yarım eklenmiş bir alan, yukarıdaki 1. hatanın ta kendisidir.

- **Arka plan işi de sınıflandırılır:** her worker tek uçuşta-istek tutar ve
  yenisi eskisini iptal eder. Bu **tek istekçi** varsayımına dayanır; ekran
  başına tarayıcı varken varsayım düşer ve bound **açlığa** dönüşür — iki ekran
  her tik birbirini iptal eder, hiçbir sonuç gelmez. Worker da ekran başına
  anahtarlanır, iki ekranın altında oturuma (TP-SUR-FM-03).

- **PRD kapısı:** multi-display'e değebilecek her PRD, `.local/prd/TEMPLATE.md`
  şablonundaki "Multi-Display Etki Analizi" bölümünü (§4) doldurur (alan envanteri +
  sınıf kararı + iki-ekran test noktası). Bölümü boş bir PRD, tasarımı bitmemiş
  bir PRD'dir.

- Kaynak: `docs/patterns/multi-client-focus.md` · `behaviors/shared-surfaces.md`
  TP-SUR-* (24 satır) + `behaviors/chat-drawer-modes.md` TP-DRAWER-01..08 ·
  üç fazın + G7 çekmece göçünün uçtan uca ölçümü · conf 0.95 (executable)

## Anti-pattern'ler (herdr-spesifik)

| YAPMA | Doğru | Kaynak |
|---|---|---|
| Whole-pane text match ile agent-durum tespiti | Evidence-based detection: `herdr agent read <pane> --source detection` ile buffer yakala, invariant/alternatif kontrolleri AND/OR gate olarak kodla; user-viewport'u durum için KULLANMA (scroll'lanabilir) | `[herdr-agents]` AGENTS.md:33,137-139 · conf 0.9 |
| Yeni `AppState` alanını sınıflandırmadan eklemek | HP18 dört-grup kapısı; şüphede `owned` (en az zarar) | `[herdr-sur-2026-07-27]` üç faz · conf 0.95 |
| Tek slotlu worker'a ikinci istekçi eklemek | Ekran başına anahtarla; iki ekranın altında oturuma düş | `[herdr-sur-2026-07-27]` açlık ölçüldü · conf 0.95 |
| Asılan testi beklemek ("yavaş olabilir") | `--config profile.default.slow-timeout.terminate-after=3` ile ADINI al | 25 dk beklendi, sonra ölçüldü · conf 0.9 |
| Rutin manifest tuning'e dev full-screen fixture suite eklemek | Manifest hot-reload döngüsü + canlı pane read; Rust testleri parsing/rule-semantics/precedence/cache'e odaklı kalır | `[herdr-agents]` AGENTS.md:137-139 · conf 0.9 |
| `cargo test` çıktısına bakıp "flaky değil, benim kodum bozuk" panik | Önce nextest'le koş (E1) — izolasyon sorunu vs gerçek race ayrımı | `[herdr-discipline-memory]` · conf 0.85 |
| AGENTS.md/CLAUDE.md'yi lokal ihtiyaç için düzenlemek | UPSTREAM dosyası — lokal agentic içerik `.local/` + docs/patterns (ignored) katmanında | `[herdr-agents]` scope + .gitignore · conf 0.9 |
| Ana feature branch'ini yeni işle kirletmek | Per-feature AYRI branch (preview-sync → feat/preview-tab-sync deseni); merge sadece kullanıcı onayıyla | `[herdr-discipline-memory]` (2026-07-12 feedback) · conf 0.85 |
| grep ile sembol keşfi | codebase-memory-mcp `search_graph`/`trace_path` (önce `index_status` tazelik kontrolü) | `[codebase-mcp]` · conf 0.7 (tazelik open) |
| Up/Down/wheel reducer'ında landed directory'yi otomatik aktive etmek | Cursor-only state transition; explicit Right/`l`/Enter activation; bounded stale-safe preview | `[herdr-fmn-2026-07-21]` · conf 0.95 |
| Worker testinde result slot dolu diye son request tamamlandı varsaymak | Test wait seam'i exact `latest_generation` sonucunu bekler; eski result yeni request'in completion kanıtı değildir | `[herdr-fmn-2026-07-21]` · conf 0.95 |
| Resident child var diye `deepest()` değerini focus/geometry authority yapmak | Prepared extent'i state'te tut; focus, auto-follow, render, hit-test, resize ve watcher için `active_col()` kullan; explicit Right/Left ile owner değişimini test et | `[herdr-fmn-2026-07-22]` · conf 0.98 |
| Right directory değilken generic activation fallback'i çağırmak | Right/`l` için exact directory gate; file/non-entry/stale/boundary `Inert`; Enter activation'ını ve primary-click cursor focus'unu ayrı tut; reducer mutation ve render override'ı birlikte RED et | `[herdr-fmh-2026-07-22]` · conf 0.98 |
| Primary directory click'i `TrailActivate` olarak route edip child focus vermek | Exact cursor-focus + owner projection; bounded `TrailPreview`; child focus yalnız Right/`l`/Enter; preview öncesi/sonrası aynı strong row'u test et | `[herdr-dclick-2026-07-23]` · conf 0.99 |
| Resident selection veya painted highlight'tan top-level focus owner çıkarmak | `FileManagerLocationsFocus` tek region owner; typed accepted input named transition çağırır, render yalnız projection yapar | `[herdr-ffo-2026-07-22]` · conf 0.99 |
| Önceki frame'de enabled görünen file action'ı current state'i doğrulamadan dispatch etmek | Header, context, plugin, rename ve worker sınırlarında current Trail owner + selection authority'yi yeniden hesapla; Rail owner fail-closed | `[herdr-ffo-2026-07-22]` · conf 0.99 |
| Watcher/reconcile projeksiyonunu `selected`'dan kurmak | `cursor_path_in_col` authority'si (override kazanır, selected yedek); `mark_selection` override'ı sildiği için sakla-geri koy; detay da odaklı satıra hazırlanır | `[herdr-fmrefresh-2026-07-26]` · conf 0.97 |
| Uykudaki bir yolu canlandıran düzeltmeyi "regresyon kaynağı" sanmak | Önce "bu yol daha önce hiç çalıştı mı?" — latent kusur canlanma anında yeni görünür; suç son commit'te değil | `[herdr-fmrefresh-2026-07-26]` · conf 0.95 |
| Per-pane süpürmede yalnız `surface.pane_infos` üzerinde dönmek | Üç sahipliği birlikte sor: pane_infos · file-manager surface · popup. Popup açıkken resim katmanının sahibi popup'tır | `[herdr-popupgfx-2026-07-26]` · conf 0.93 |
| Manifest'i düzenleyip canlıda görünmesini beklemek | `plugins.json` snapshot'tır; `herdr plugin link` ile yeniden bağla, profil-kapsamlı olduğunu unutma | `[herdr-pluginreg-2026-07-26]` · conf 0.9 |

## Karar matrisi (herdr işi → zorunlu disiplin)

| İş tipi | Zorunlu adımlar |
|---|---|
| Yeni sidebar/TUI feature | HP2 sınıflandırma → HP1 saf-state test → TDD + `just check` → headful `cargo run` doğrulama |
| Server/API alanı ekleme | HP2 (shared-fact kanıtı) → HP9 protokol kontrolü → integration test |
| Detection/manifest işi | Anti-pattern-1 protokolü (canlı buffer kanıtı) → manifest hot-reload döngüsü |
| Identity/state refactor | HP4 tam protokol (characterization + adversarial + roundtable) |
| Cursor/odak/refresh dokunuşu | HP11+HP14 authority ayrımı → HP15 (refresh navigasyon değildir) → gerçek zaman çizgisiyle RED |
| Grafik/render per-pane geçişi | HP16 üç sahiplik kontrolü (pane_infos · FM surface · popup) |
| Eklenti manifest/eylem değişikliği | HP17 `plugin link` ile yeniden bağla + `plugins.json`'dan eylem id'lerini doğrula |
| libghostty-vt dokunuşu | HP8 patch-index + `just check` reverse-apply doğrulaması |
| Her commit | HP3 `just check` + HP7 mesaj hizalama + HP5 lint zinciri |

---
*v1.0.0 — 2026-07-12 · reference-registry Adım-3 artefaktı. Global katman: rust-dev skill P1–P25.
Evidence graph: .cartography/rust-engineering-SYSTEM-MAP.json (+ ~/.cartography kopyası).*
