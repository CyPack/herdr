# Feature Change Map

> Bir feature eklemeden, çıkarmadan veya değiştirmeden önce okunacak tek dosya.
> Amaç: "buraya dokunursam ne kırılır" sorusunun tahminle değil, **haritayla**
> cevaplanması.
>
> Kardeş dosyalar: `rust-engineering.md` (nasıl çalışılır — HP1–HP18) ·
> `behaviors/README.md` (davranış nasıl korunur) · `docs/references/README.md`
> (bir kararın kaynağı nedir).
>
> Ölçümler 2026-07-27, `master` @ `5e175550`. Rakamlar bayatlarsa aşağıdaki
> komutlarla tazelenir — **tahminle güncelleme**.

---

## 0. Otuz saniyelik kullanım

```
1. Feature hangi KATMANA düşüyor?          → §1 tablosu
2. O katmana dokunmak neyi tetikler?       → §2 bağımlılık zinciri
3. Yeni STATE ekliyor muyum?               → §3 dört soru (atlanırsa çoklu-ekran bozulur)
4. Hangi DİKİŞ'ten geçiyor?                → §4 (üretimde 5 yer, hepsi bu kadar)
5. Neyi ölçmeliyim?                        → §5 metrikler + ölçme komutu
6. Nasıl kapatılır?                        → §6 kapılar
```

---

## 1. Katman haritası

Ölçüm: `find src -name "*.rs" | wc -l` → **280 dosya / 282.714 satır**.

| Katman | Yer | Boyut | Ne yapar | Ne YAPMAZ |
|---|---|---:|---|---|
| **State** | `app/state.rs` | 5.780 | Saf veri. `AppState` = **176 alan** | Kanal yok, async yok, I/O yok (HP1) |
| **Actions** | `app/actions.rs` | 6.815 | State üzerinde geçişler | Render etmez, girdi okumaz |
| **Input** | `app/input/` (12) | 34.840 | Tuş/fare → action | State'i doğrudan çizmez |
| **Render** | `ui/` (34) + `ui.rs` | 32.524 | `&AppState` → çizim | **Render sırasında mutasyon YASAK** (HP1) |
| **Layout** | `layout.rs` | 1.029 | Pane ağacı, odak | Terminal bilmez |
| **Workspace** | `workspace/` (9) | 5.303 | Workspace/tab/pane organizasyonu | PTY bilmez |
| **FM** | `fm/` (16) | 16.934 | Dosya tarayıcısı modeli | Ekran/görüntü bilmez |
| **Server** | `server/headless.rs` (3) | 12.192 | Protokol, client, frame yayını | UI mantığı barındırmaz |

**Geometri hesabı `compute_view()`'da, çizim `render()`'da.** Bu ayrım HP1'in
kalbi; bozulursa render sırası davranışı belirlemeye başlar ve test edilemez hale
gelir.

---

## 2. Bağımlılık zinciri — "X'e dokunduysan Y'ye bak"

| Dokunduğun şey | Zorunlu kontrol | Neden (gerçek olay) |
|---|---|---|
| **`AppState`'e yeni alan** | §3 dört soru + `client_surfaces!` grubu | Sınıflandırılmayan alan sessizce paylaşılır → menü tüm ekranlarda açılır |
| **`Mode` varyantı** | `broadcast` grubu + overlay render kolu | API modu ayarlar, ekran navigate'te kalır → **yazdığı yutulur** (TP-SUR-BROADCAST-01) |
| **Yeni worker / arka plan iş** | Ekran başına anahtar (`worker_key`) | Tek slot + iki istekçi = **açlık**: hiçbir sonuç gelmez (TP-SUR-FM-03) |
| **`stage` / `file_manager`** | İkisi TEK yüzey, ayrı gruba konmaz | Ayrılırsa "içeriği olmayan sahne" durumu temsil edilebilir olur |
| **Pane odağı / tab değişimi** | `stage`'i de taşı | Odak doğru gider, ekran Files'ta kalır → hiçbir şey olmamış görünür (TP-SUR-STAGE-06) |
| **Persist edilen alan** | `WorkspaceSnapshot` alan adı = **disk formatı** | Toplu rename snapshot'ı bozar; bu bir kez yaşandı |
| **Protokol mesajı / API alanı** | HP2 sınıflandırma kapısı + `PROTOCOL_VERSION` (HP9) | Client/server sürüm uyuşmazlığı = kullanıcıya "protocol 16 vs 18" |
| **`Tab`'a yeni alan** | **5 literal site**: `tab.rs` ×2 (`new_with_runtime`, `from_existing_pane`) · `workspace.rs` ×2 (test kurucuları) · `persist/restore.rs` ×1 | Kurucuların hepsi aynı default'u istemez: restore/taşınan-pane muaf olmazsa **restart 27 tab'ı birden yanar** (unseen/flaş, 2026-07-29) |
| **Tab yaşam-döngüsü bayrağı** (unseen, flash) | Temizleme hunisini TEK noktaya bağla: `Workspace::switch_tab` | Odak hunilerinin hepsi (tab-bar tıklaması, API focus, sidebar chat, spam-guard) oradan geçer; başka yere koyarsan bir yol atlanır ve bayrak asılı kalır |
| **Zamana bağlı görsel efekt** (flaş, blink, spinner) | `sync_animation_timer_with_interval` koşuluna ekle + fazı **elapsed**-bazlı hesapla | Headless renderer YALNIZCA tick'te çizer: timer armlanmazsa faz doğru hesaplanır ve **hiç çizilmez** (hiçbir state testi yakalamaz). Tick-bazlı faz 16 ms monolitik / 128 ms headless'ta farklı hızda akar |
| **Sidebar / agent panel satır stili** | `render_agent_detail` stil bloğu + **stili pinleyen mevcut testler** (`grep -n "palette\.\(text\|surface_dim\|overlay0\)" src/ui/sidebar.rs`) | Stil sözlüğünü değiştirmek renk-assert'i olan testleri kırar; bunlar ID'siz olabilir (davranış kaydı yok) → kırıldığında "regresyon mu, güncelleme mi" ayrımını SEN yapmalısın (2026-07-29: 2 test güncellendi, asıl konuları korundu) |
| **Sidebar satır listesi** (`WorkspaceListEntry`) | Varyant eklemek 6+ iterasyon sitesini zorlar (sidebar ×3, mobile ×3, actions ×1) — derleyici hepsini bulur. Ama **hit-test vektörünü kirletme**: `WorkspaceCardArea` workspace-indeksli, oraya başka tür satır koymak tıklamayı yanlış hedefe götürür → AYRI vektör (`workspace_chat_row_areas`) | TP-FTAB-ENTRY-05 aynı tuzağı tab strip'te belgeliyor: *"appending here would make a stage click resolve as a terminal tab index"* |
| **Sidebar'da yeni satır TÜRÜ** (workspace olmayan: grup başlığı vb.) | **KENDİ area vektörünü** al (`workspace_group_header_areas`). `ws_idx` taşımayan bir satırı workspace-indeksli vektöre koyma; hit-test'te de kendi bloğunda eşleştir | Başlık workspace sanılırsa tıklaması "pozisyonu paylaşan" workspace'e çözülür. Aynı tuzağın üçüncü tekrarı (TP-FTAB-ENTRY-05 → TP-WSCHAT-17 → TP-TREE-05) |
| **İki bağımsız açılır-kapanır (disclosure) aynı satıra düşüyorsa** | DUR. Aynı gutter'da iki ok = iki farklı kontrol, ayırt edilemez. Çözüm stil değil **yapı**: birine kendi satırını ver | 2026-07-30: `▾▾` yan yana çizildi, `▸` üç ayrı anlam kazandı, kullanıcı "kesinlikle tatmin etmedi" dedi. Renk/ton denemeleri bunu çözmez (TP-TREE-01/08) |
| **Satır prefix'i / girinti değiştirme** | Bütçeyi ÖLÇ: `card.rect.width` sidebar genişliğinden 1 KÜÇÜK. Prefix +1 ve trailing rezerv +1 = isimden 2 sütun gider; 21 karakterlik workspace adı kesilir ve `cross_area` entegrasyon testi düşer | Trailing chrome'u **rezerve et**, üstüne çizme: eski `+` ismin son harfini sessizce yiyordu |
| **Sidebar varsayılan genişliği** (`config/model.rs`) | Bu bir ÜRÜN varsayılanı; onu türeten ~13 test var (bölücü koordinatları, `terminal_area` rect'i, pane sarması). Testleri ÜRÜNE bağlama — fixture'da (`app_for_mouse_test`) genişliği **sabitle** | 26→30 değişiminde 13 test düştü; hepsi +4 kaymaydı. Fixture pinlemek 8'ini tek satırda çözdü ve testleri konularına geri döndürdü |
| **Mobil yüzey** (`ui/mobile.rs`) | Geometrisi **workspace başına tam 2 satır** (`pos * 2`). Sidebar satır listesine yeni tür eklersen mobil onları FİLTRELEMELİ, yoksa her pozisyon kayar → switcher yanlış workspace'i seçer | Tek yardımcıya indir (`mobile_space_entries`), üç çağrı sitesinin ayrı ayrı hatırlamasına bırakma |
| **Sidebar'a yeni buton/glyph** | `ui/tab_surface.rs` **frame digest** karakterizasyonu kırılır — bu BEKLENEN. Yapısal assert'ler (rect'ler, pane sayısı, cursor) geçmeye devam ediyorsa yeniden temellendir, gerekçesini yorumla yaz | Digest pikselleri izler; bir glyph eklemek tam da onun yüzeye çıkarması gereken şeydir |
| **Yeni kalıcı dosya** (`~/.config/herdr/*.json`) | `session.json`'a EKLEME — restore sözleşmesi onun şekline bağlı. Ayrı dosya + atomik yazım (`persist/plugin_registry.rs` emsali) + bozuk dosyada panik YOK + `--no-session`'da yazma YOK | `--no-session` kapısı atlanırsa her unit test gerçek config dizinine yazar (2026-07-30'da yaşandı) |
| **Tab strip etiketi** (`tab_chrome_label`) | Genişlik + hit-area otomatik uyumlu — glyph'i etiketten geçir, ayrı sütun açma | `tab_width` etiketten türer; yan kanal açarsan hit-test kayar. Zoomed `" Z"` eki bu yolun kanıtlı emsali |
| **Rename / sembol taşıma** | En az 3 grep kategorisi (çağrı · tip · string · barrel · test · config) | grep AST değildir; kaçan referans runtime crash |
| **`ui.rs` arka plan süpürmesi** | `tab_is_watched` benzeri koruma | Her ekran diğerinin sekmesini yeniden boyutlandırır |
| **`vendor/libghostty-vt`** | `vendor/libghostty-vt.patches.md` her aktif patch | Patch sessizce düşer (HP8) |

**Sembol izini elle sürme.** Kod grafiği indeksli
(`home-ayaz-projects-herdr`, 31.617 düğüm / 168.657 kenar):

```
search_graph(name_pattern=...)        # sembolü bul
trace_path(function_name=..., mode=calls)   # çağrı zinciri
get_code_snippet(qualified_name=...)  # tam kaynak
```

---

## 3. Yeni state eklerken — dört soru

`AppState`'e alan eklemek **her zaman** bir sınıflandırma kararıdır. Makro
(`client_surfaces!`) yarım göçü derleme hatasına çevirir, ama **hangi gruba**
koyacağını sana söylemez. Sıra önemli:

```
1. Bu oturum GERÇEĞİ mi?  (terminaller, workspace'ler, plugin pane'leri)
   → EVET: demete GİRMEZ. Bitti.

2. Config mi?  (keybind, palette, tema)
   → EVET: demete GİRMEZ. Bitti.

3. Oturum bunu KENDİ BAŞINA değiştirdiğinde (hiçbir ekran hizmet edilmezken)
   kim görmeli?
   → Sadece default    → inherited   (active, stage'in üstündeki seçim)
   → Her ekran         → broadcast   (mode, menüler, diyaloglar, rail, kaydırma)
   → Hiç kimse         → owned/ephemeral

4. Karşılaştırılabilir mi? (ucuz PartialEq)
   → HAYIR → owned (tek ekranlıyken terfi eder) veya ephemeral (asla)
```

**Yakalanması en zor hata:** 3. adımı atlayıp her şeyi `inherited` yapmak.
O zaman API'nin ayarladığı mod bağlı ekranlara ulaşmaz ve kullanıcı yazdığını
yutan bir ekranla kalır — hiçbir birim test yakalamaz, iki-client testi yakalar.

**Değişmez kural:**

> Tek ekran varken oturum ile o ekran aynı şeydir ve tek slotu paylaşırlar.

Demet park/terfi, worker anahtarı ve benimseme kuralının üçü de buna dayanıyor.
Delinirse her monolitik koşum bozulur.

---

### 3b. Görsel dikkat özellikleri — beş katmanlı kontrol listesi

"Kullanıcı bir şeyi fark etsin" isteyen her özellik (unseen işareti, flaş,
aktif-satır vurgusu, bildirim noktası) **aynı beş katmandan** geçer. Biri
atlanırsa özellik "state doğru, ekranda yok" durumuna düşer — bu ailenin
karakteristik arızası.

| # | Katman | Soru | Atlanırsa |
|---|---|---|---|
| 1 | **Sınıf** | Bilgi kimin: oturumun mu, bir ekranın mı? (§3) | Yanlış sınıf → ya her ekranda sızar ya hiç görünmez |
| 2 | **İşaretleme (SET)** | Hangi yaratma/olay yolları işaretler, hangileri MUAF? | Hepsi işaretlenirse sinyal enflasyonu (restart tüm strip'i yakar); azı işaretlenirse asıl senaryo sessiz kalır |
| 3 | **Temizleme (CLEAR)** | Tek huni hangisi? Kalıcı mı, yoksa geri dönünce tekrar yanar mı? | Temizlenmeyen işaret bilgi taşımaz; huni çoklu olursa bir yol atlanır |
| 4 | **Çizim (RENDER)** | Hangi stil kanalı? Komşu durumlarla (aktif/DIM/drag) **ayırt edilebilir** mi? | DIM bir dalın altında kalırsa vurgu susturulur; aktif stille aynı kanal seçilirse iki durum karışır |
| 5 | **Sürüş (FRAMES)** | Efekt zamana bağlıysa frame'ler geliyor mu? | Timer armlanmaz → çizim kodu doğru, ekranda hiçbir şey yok |

**Çift kanal kuralı:** vurgu yalnız renkle taşınmaz. Şekil (glyph, çubuk) +
parlaklık/zemin birlikte kullanılır — tema değişse veya renk algısı farklı olsa
da sinyal ayakta kalır (`● ` + accent-bold, `TP-TAB-UNSEEN-04`).

**Kanıt kuralı:** bu ailede state testi YETMEZ. Vurgu bir **buffer** iddiasıdır:
`TestBackend` ile çiz, hücrenin `fg`/`bg`/`modifier`'ına bak. FM-önizleme dersi
(4265 test yeşil, ekranda hiçbir şey yok) tam bu yüzden ödendi.

---

## 4. Dikişler — çapraz kesen değişiklik nereye düşer

Üretimde bir client bağlamı açan **tam 5 yer** var. Yenisini eklemeden önce
mevcut beşinin yetmediğini kanıtla:

| # | Yer | Ne için |
|---|---|---|
| 1 | `app/mod.rs` `route_client_events_from` | Girdi yönlendirme |
| 2 | `server/headless.rs` `handle_client_input_events` | Protokol girdisi |
| 3 | `server/headless.rs` `render_and_stream` | Render geçişi |
| 4 | `server/headless.rs` `negotiated_tab_sizes` | Boyut pazarlığı |
| 5 | `app/runtime.rs` `for_each_display` | Zamanlanmış iş (worker'lar) |

Her biri **tek çıkış yolunda** kapanır — erken `return` başka bir ekranın
görünümünü kurulu bırakamaz. Bu yapısal, yorum değil.

---

## 5. Korunacak metrikler

| Metrik | Taban (2026-07-27) | Nasıl ölçülür | Neden |
|---|---|---|---|
| Test | **4231 geçer / 0 kalır** | `cargo nextest run --locked` | Kırmızı ile commit yok |
| Süre | **~46 sn** | aynı komut | 2× aşarsa kilitlenme ara (aşağı bak) |
| Kayıtlı davranış | **102** | `grep -c '^| TP-' behaviors/*.md` | Azalması = merge bir feature'ı yuttu |
| Clippy | **0 uyarı** | `cargo clippy --all-targets --locked -- -D warnings` | Uyarı = hata (HP5) |
| Windows lint | temiz | `just check` içinde | Unix'te yakala (HP6) |
| Pointer layout geçişi | ön plan **0**, diğer ekran **8 olayda 1** | `view_recomputes_for_input` sayacı | Ölçülmüş regresyon buradan geldi |
| Sıcak yolda derin klon | **0** (FmState taşınır, kopyalanmaz) | kod incelemesi + `take_owned` | Kare başına dizin listesi kopyalamak |
| `unwrap()` (prod) | **0** | `rg 'unwrap\(\)' src --glob '!*test*'` | HP-çıta |

**Test süresi 2× uzadıysa kilitlenme ara — bekleme değil:**

```bash
cargo nextest run --locked --no-fail-fast \
  --config 'profile.default.slow-timeout.period="25s"' \
  --config 'profile.default.slow-timeout.terminate-after=3'
```
Bu, asılan testin **adını** verir. Bir kez 25 dakika bekledikten sonra öğrenildi.

---

## 6. Kapılar

```bash
just check       # fmt + clippy(+Windows target) + nextest + bun + 128 python bakım testi
```

`just check` exit 0 olmadan commit yok. Bypass yok — ya düzelt ya da neden
daha dar bir doğrulamanın yettiğini **yazılı** açıkla (CLAUDE.md).

Davranış eklediysen: `behaviors/<feature>.md` satırı **zorunlu**. Adı bir testin
sahiplenmediği fork davranışı, bir sonraki merge'in sessizce silebileceği
davranıştır — merge yeşil biter, özellik gitmiştir.

---

## 7. Bu projede pahalıya mal olmuş anti-pattern'ler

| YAPMA | DOĞRU | Bedeli |
|---|---|---|
| Yeni alanı sınıflandırmadan eklemek | §3 dört soru | Menü tüm ekranlarda açılır |
| Tek slotlu worker'a ikinci istekçi | Ekran başına anahtar | Açlık: hiçbir sonuç gelmez |
| "Sahne" ile "içerik"i ayrı gruba koymak | Tek grup | İçeriksiz sahne temsil edilebilir olur |
| Toplu mekanik rename | Alıcı tipine göre + snapshot alanlarını koru | Disk formatı bozulur |
| Asılan testi beklemek | `--slow-timeout` ile adını al | 25 dakika |
| `git add -A` (kapsamsız) | Yola göre stage | Scratch dizin repoya girer |
| Kilitlenmeden önce "muhtemelen yavaş" demek | Ölç, sonra iddia et | Yanlış teşhis |
| Görsel efekti yalnız state testiyle doğrulamak | `TestBackend` + hücre `fg/bg/modifier` iddiası | "State doğru, ekran boş" — FM önizlemesi bu şekilde canlıda hiç gelmedi |
| Zamana bağlı efekti timer'a bağlamamak | `sync_animation_timer` koşuluna ekle | Headless tick gelmez → efekt hiç çizilmez, hiçbir test görmez |
| Vurguyu yalnız renkle taşımak | Çift kanal: şekil + renk/zemin | Tema/DIM dalı sinyali susturur |
| Yeni `Tab` alanını her kurucuda aynı default'la doldurmak | Kurucu başına anlamı sor (spawn ≠ restore ≠ taşıma) | Restart tüm strip'i yakar/strobe eder |
| Kırılan renk-assert'ini refleksle "güncellemek" | ID'sini ara (`grep behaviors/`); yoksa asıl konusunu koru, sadece stil tabanını taşı | Testin koruduğu gerçek davranış sessizce silinir |
| İki farklı kontrolü aynı gutter'a aynı glyph'le koymak | Birine kendi satırını ver (yapısal ayrım) | Kullanıcı hangi okun ne yaptığını bilemez; stil denemesi çözmez |
| Bir glyph'e üçüncü anlam yüklemek (`▸` = grup + çekmece + odaklı) | Anlam başına ayrı kanal: ok=disclosure, zemin=odak, `●`=başka yerde açık | Üç anlamlı glyph hiçbir anlam taşımaz |
| Görsel durumu **hover**'a bağlamak | Seçim/aktiflik gibi girdiyle değişen duruma bağla | TP-REPAINT-2B'yi kırar: fare hareketi başına tam render (uzakta doğrudan gecikme) |
| Trailing buton çizerken metin bütçesinden düşmemek | Rezerve et, üstüne çizme | Uzun ad son harfini sessizce kaybeder |
| Kaynak/model tercihini kullanıcıya sormadan seçmek | Görsel model kullanıcı-sahiplidir → `AskUserQuestion` + preview | Yanlış model = beğenilmeyen UI, iki kez iş |

---

*v1.1.0 — 2026-07-29 · MCF → SUR → FM üç fazı + tab-attention/agent-panel
fazından damıtıldı. Her satır ya ölçüm ya da gerçekten yaşanmış bir olaydır;
hiçbiri tahmin değildir.*
