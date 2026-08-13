---
doc: herdr-pattern-catalog
domain: tui-composition
created: 2026-07-13
status: canonical — her pattern kaynak-repo:dosya + confidence taşır
agentic_triggers:
  - "composable shell · named regions · swappable component slot · addable page · popup stack"
  - "compositor · component trait · dockable panel · plugin ui composition · routines page"
  - "herdr ui mimarisi · herdr popup · herdr panel sistemi"
related:
  - docs/references/tui-composition.md          # kaynak registry (bu kataloğun kanıt tablosu)
  - .cartography/tui-composition-SYSTEM-MAP.json # evidence graph
example_pool: local refpool (yazi-src, superfile-src) + live source reads (zellij/helix/lazygit/gitui/k9s/neovim/tui-realm/cursive/ratatui-templates)
---

# Pattern Kataloğu — tui-composition

> "Advanced terminal apps" composition-mimarisi kıyaslaması (2026-07-13). Amaç: herdr'ı named-region +
> swappable-component-slot + addable-page + popup + dockable-panel destekleyen bir "desktop-app shell
> in a terminal" hâline getirmek için kanıtlanmış mimari desenleri damıtmak.

## Karşılaştırma tablosu

| Uygulama | Layout kompozisyonu | Resize modeli | Sayfa/görünüm modeli | Popup/overlay katmanı | Pluggable/swappable component |
|---|---|---|---|---|---|
| **zellij** | Per-tab recursive split TREE (`TiledPaneLayout`, KDL nested) | Layout-time ratio (`Percent`/`Fixed`) + runtime keybind-driven directional/auto-grow | Her tab = kendi ağacı; `Screen` tab koleksiyonunu yönetir | Ayrı `Vec<FloatingPaneLayout>` katmanı, absolute x/y/w/h, `z_indices` stack + `pinned` always-on-top | Plugin = `PluginPane`, `TerminalPane` ile AYNI `Pane` trait'i; ANSI-emit → per-client VTE Grid |
| **helix** | Compositor: FLAT `Vec<Box<dyn Component>>` stack (ayrıca ayrı bir n-ary split `Tree` editor içinde) | Split-tree: iteratif top-down eşit-bölme; Compositor katmanları resize'a duyarlı değil (her zaman full Rect) | Yok (tek editor "sayfası"); picker/prompt katman olarak push edilir | `Compositor.layers` — Vec-pozisyonu = z-order, `push`/`pop`/`replace_or_push`, event `Consumed` ile top-down kesiliyor | `Component` trait (render zorunlu, diğerleri default) + generic `Overlay<T>`/`Popup<T>` decorator'lar |
| **lazygit** | Declarative `boxlayout.Box` tree, HER FRAME state'ten yeniden inşa edilir (responsive/conditional) | Weight/Size alanları + frame-başı yeniden-hesaplama (statik resize-mode yok, layout zaten dinamik) | `Context` (window'dan ayrık, `ContextKind` enum) + `ContextStack` push/pop/replace | Ayrı content-sized/centered popup positioning geçişi, aynı context-stack focus/z-order'ını paylaşır | `types.Context` arayüzü — her context aynı sözleşmeyi uygular |
| **gitui** | Flat struct: 5 tab (match) + named panel field'ları (tree DEĞİL) | Manuel ratatui `Layout::split` per-draw | Tab index (usize) + match — sayfa kavramı yok | `draw_popups!` — tab çizildikten SONRA ~30 named popup field, unconditional draw (no-op when hidden); ayrı küçük `PopupStack` (sadece ESC-nav sırası) | `DrawableComponent`+`Component` trait (draw/event/focus/visible) — "composition by CODE not by DATA" (açık kaynak-yorumu) |
| **k9s** | Persistent `tview.Flex` chrome (header/status→Content→crumbs→flash), TEK swappable body | Chrome sabit; Content = tview.Pages (kendi iç boyutlandırması) | Generic `model.Stack`→`ui.Pages`→`view.PageStack`, `:command`→alias→GVR→REGISTRY(`MetaViewers`)→construct→push | Aynı push/pop stack popup'lar için de kullanılır (help ekranı bile aynı stack'e push/pop edilir) | `model.Component` evrensel arayüz + registry (`map[GVR]MetaViewer{viewerFn,enterFn}`) — yeni sayfa = 1 registry satırı |
| **neovim** | Window split-tree (recursive bisection) + AYRI floating-window katmanı | `:split`/`:vsplit` + `'equalalways'`; float boyutu `nvim_open_win` config'te sabit | Yok (buffer/window modeli, "sayfa" kavramı plugin'lere bırakılmış) | `nvim_open_win(relative,row,col,anchor,zindex,style=minimal)` — numeric zindex (default 50, builtin 100/200/250) | Plugin deseni: scratch buffer (`nofile`) + float VEYA normal-window'a swap (alpha-nvim) — placement içerikten ayrık |
| **yazi** | Lua `ui.Layout` (ratatui Layout wrapper) + Lua duck-type component ağacı (Root/Tab/Header/Status) | Lua tarafında `:split()` ile hesaplanır, Rust host'a opak | Yok (tek dosya-yöneticisi "sayfası") | Tier 2: Rust'ta HARDCODED fixed-order ~9-popup if-chain (anti-pattern) + istisna: `mgr::Modal`'ın Lua'ya bağlanan dinamik `children_add(component,order)`/`children_remove(id)` registry'si | Lua tarafı: closed-set Rust `Renderable` primitive'lerini (Line/Text/List/Bar/...) Lua kompoze eder, `TypeId` match ile render edilir |
| **superfile** | Flat Elm-model (bubbletea) — struct field per panel/modal | Bubbletea `WindowSizeMsg` → her modelin kendi `Update` | Yok | Modal field'ları struct'ta, elle `View()`'da kompoze | Yok (registry yok, hand-wired) |
| **tui-realm** (crate) | Manuel ratatui `Layout::split`, `Application` sadece render dispatcher | Manuel (uygulayıcı hesaplar) | Yok (component-mount registry var ama "sayfa" kavramı yok) | Manuel: ayrı `ComponentId` mount + koşullu `view()` çağrısı, built-in stack YOK | `Application<ComponentId,Msg,UserEvent>`: ID-keyed `mount`/`umount`/`remount`/`view`/`query`/`attr`/`active`/`blur`/`tick` |
| **cursive** (crate) | `StackView{layers:Vec<Child>}` — genuine dynamic layer stack | Her layer HER FRAME layout edilir (arka plan popup altında bile canlı kalır) | Yok (genel-amaçlı popup/dialog stack) | `modal:bool` + `placement(Fullscreen\|Floating)`; front-to-back dispatch, modal event'i ignore etse bile propagasyonu DURDURUR | `View` trait (draw zorunlu, layout/on_event/required_size/take_focus default) |
| **ratatui/templates** | Manuel `Layout::split`, flat `Vec<Box<dyn Component>>` + tokio mpsc `Action` bus | Manuel | Yok | Yok (built-in yok) | `Component` trait (handle_key_event/handle_mouse_event/update/draw) |
| **herdr (mevcut)** | BSP tree (`src/layout.rs`, binary, f32 ratio) — leaf = sadece terminal pane | Mouse-drag + ratio | Yok | Hand-written `render_X()` fonksiyonları `src/ui.rs`'ten çağrılıyor, ortak trait/registry yok | Yok — yeni panel/dialog eklemek = yeni fonksiyon + `ui.rs`'e wire etmek |

## Pattern kataloğu (rank = herdr'a taşınabilirlik önceliği)

### [P1] ⭐ Compositor: `Vec<Box<dyn Component>>` layer stack + Component trait · conf 0.95
- **NE:** Tek bir düz `Vec<Box<dyn Component>>` yığını; her katman `render(full_area, ...)` alır ve arka-planı ezerek çizer (Vec sırası = z-order); event'ler tepeden (`iter_mut().rev()`) dağıtılır, `Consumed` dönünce durur, `Ignored` alttaki katmana düşer.
- **KAYNAK:** `[helix-compositor]` `helix-term/src/compositor.rs` (Cursive'den ilham alındığı kaynak yorumunda açıkça yazıyor) — bkz. `docs/references/tui-composition.md`. Cursive'in kendi `StackView`'i (`[cursive-stackview]`) aynı deseni bağımsızca doğruluyor (modal-farklılığıyla, P4'e bkz).
- **NE ZAMAN:** herdr'ın "popups" + "addable pages" ihtiyacı — mevcut BSP pane-tree render'ı DEĞİŞTİRİLMEDEN, üstüne eklenen non-invasive bir katman. Mevcut pane-tree = layer 0; yeni sayfa = layer-0'ı `replace_or_push` ile değiştiren alternatif bir Component; popup/dialog = üste push edilen ek katman.
- **NE ZAMAN KULLANMA:** Eğer herdr tüm popup'ları BSP ağacının İÇİNE (bir pane olarak) yerleştirmek isterse gerek yok — ama bu, zellij'in floating-panes'i ayrı tutma kararıyla (P2) ÇELİŞİR; ikisi birlikte kullanılmalı (BSP ağacı içeriği için P1 gerekmez, sadece üst-katman popup/sayfa için).
- **herdr eşlemesi:** `src/ui.rs`'teki mevcut `render_X()` fonksiyonlarının çoğu (dialogs, menus, onboarding, settings, keybind_help, navigator, release_notes) zaten kavramsal olarak birer "popup katmanı" — bunları generic `Popup<T>`/`Overlay<T>` (P3) decorator'larıyla sarmalayıp bir `Compositor` yığınına taşımak, hand-wired `render()` dispatch'ini ortadan kaldırır.

### [P2] Uniform leaf trait ("Run"-benzeri enum) + BSP ağacının DIŞINDA z-indexli floating katman · conf 0.9
- **NE:** BSP/split-ağacının her leaf'i bir `Run`-benzeri enum tutar (Terminal | Plugin | Page-slot | Static), böylece "swappable component slot" ağacın normal bir parçası olur. Popup/floating panel'ler ise ağacın TAMAMEN DIŞINDA, ayrı bir `Vec` + `z_indices: Vec<Id>` + `pinned` bayrağıyla tutulur.
- **KAYNAK:** `[zellij-layout]` (`Run` enum), `[zellij-floating]` (`z_indices`, `pinned`). herdr için EN yüksek doğrudan-benzerlik kaynağı çünkü zellij de bir multiplexer.
- **NE ZAMAN:** herdr'ın mevcut `src/layout.rs` BSP ağacını GENİŞLETİRKEN (leaf tipini terminal-only'den generic'e çıkarmak) + popup'ları ayrı tutmak isterken.
- **NE ZAMAN KULLANMA:** Basit, az sayıda sabit popup varsa (mevcut durumda olduğu gibi) bu kadar genel bir mimari fazla mühendislik olabilir — P1 (Compositor) tek başına yeterli olabilir.
- **ANTI-PATTERN'dan KAÇIN:** zellij'in "stacked panes" özelliği ayrı bir katman DEĞİL, tree-node üzerinde bir bool bayrağı (`children_are_stacked`) — swappable-slot/stack ihtiyacı için yeni bir veri tipi icat etmek yerine mevcut ağaç node'una bir mod bayrağı eklemek yeterli olabilir.

### [P3] Persistent chrome + TEK swappable body + command/registry-resolved page-stack · conf 0.95
- **NE:** Dış çerçeve (header/toolbar/sidebar/statusbar) BİR KEZ inşa edilir ve asla yeniden inşa edilmez; içindeki TEK bir "content" bölgesi bir stack/registry ile değiştirilir. Yeni sayfa eklemek = `map[name]Constructor` registry'sine 1 satır eklemek; sayfa geçişi VE drill-down (örn. pane içi detay) AYNI push/pop mekanizmasını kullanır.
- **KAYNAK:** `[k9s-app-layout]` (`layout()`/`buildHeader()` — header/status→Content→crumbs→flash), `[k9s-registrar]` (`MetaViewers` registry), `[k9s-command]` (`:command`→alias→GVR→registry→construct→push).
- **NE ZAMAN:** herdr'ın açıkça istediği "routines/cron page" ihtiyacı İÇİN BİRİNCİL ÖNCELİKLİ desen — yeni bir `Routines` tipi + registry'ye 1 kayıt + `:routines`/keybind alias, ne compositor'a ne command-parser'a dokunmadan eklenir.
- **NE ZAMAN KULLANMA:** herdr'da zaten sabit-sayıda tab/workspace varsa (mevcut tab sistemi) ve "sayfa" kavramı sadece merkez içerik alanına mı yoksa TÜM pencereye mi uygulanacaksa netleştirilmeli — k9s'te "Content" TÜM gövdeyi kaplıyor (BSP pane-tree'nin üstünde bir üst-seviye kavram), herdr'da bu "yeni bir workspace/tab türü" mü yoksa "merkez bölgenin içeriği" mi olacağı bir tasarım kararı gerektirir.
- **herdr eşlemesi:** herdr'ın CLAUDE.md'sinde belirtilen "named regions (top toolbar, left/right panels, center content-with-tabs, bottom bar)" hedefiyle BİREBİR örtüşüyor — dış Flex-benzeri chrome + P1 Compositor'ın oturduğu TEK swappable merkez bölge.

### [P4] Modal-aware layered stack: her katman HER FRAME layout edilir, modal event-propagasyonunu durdurur · conf 0.95
- **NE:** `StackView`'daki her layer (üstteki popup dahil) her frame boyutlandırılır (arka plan popup altında bile "canlı" kalır); event dispatch üstten-alta gider ama `modal:bool` bayraklı bir katmana ulaşınca, katman event'i IGNORE etse BİLE event'in daha aşağı sızması DURDURULUR (arka planın yanlışlıkla tıklanmasını engeller).
- **KAYNAK:** `[cursive-stackview]` `cursive-core/src/views/stack_view.rs`. Helix'in kendi Compositor'ının (P1) esin kaynağı olarak kaynak-yorumunda anıldığı için P1 ile YÜKSEK YAKINLIK — ama helix'te "modal her zaman event yutar" garantisi YOK (sadece `Consumed`/`Ignored` var); cursive bu garantiyi ekliyor.
- **NE ZAMAN:** P1'i (Compositor) uygularken, "arka plandaki panel'e yanlışlıkla tıklama" riskini gidermek için `modal: bool` alanını P1'in katman tipine eklemek — bu, P1'in doğrudan bir GELİŞTİRMESİ, ayrı bir mimari değil.
- **NE ZAMAN KULLANMA:** herdr'ın TÜM popup'ları zaten modal davranıyorsa (mevcut `src/ui/dialogs.rs` muhtemelen öyle) bu ayrım gerekmeyebilir — ama non-modal bir "yardım ipucu" popup'ı (örn. which-key-benzeri) eklenirse gereklidir.

### [P5] Floating-window primitive: relative/anchor/zindex hesaplı Rect, ağaçtan bağımsız · conf 0.9
- **NE:** Bir popup'ın konumu `relative(editor|pane|cursor)` + `anchor` köşesi + `row`/`col` offset + `zindex` tamsayısıyla hesaplanır; bu Rect, BSP ağacındaki pane sınırlarına bağlı DEĞİLDİR (pane sınırlarının üzerinden geçebilir).
- **KAYNAK:** `[nvim-api-doc]` `nvim_open_win()`, `[nvim-zindex-issue]` (default 50, builtin 100/200/250).
- **NE ZAMAN:** P1/P2 ile birlikte, bir popup'ın "hangi pane'e/cursor'a göre" konumlanacağını hesaplayan yardımcı fonksiyon olarak — P1 SADECE katman yığınını yönetir, Rect HESAPLAMASINI yönetmez; bu boşluğu P5 dolduruyor.
- **NE ZAMAN KULLANMA:** Popup her zaman ekran-merkezli/sabit-boyutlu ise (mevcut herdr dialog'ları gibi) gerekmeyebilir — cursor/pane-göreli konumlandırma (örn. bir agent panelinin yanında bağlam-menüsü) gerektiğinde devreye girer.

### [P6] Basit ortak leaf trait: draw+event+focus (mimari değişikliği MİNİMİZE eden seçenek) · conf 0.9
- **NE:** `Component{draw(f,rect); event(ev)->EventState; focus(bool); focused()->bool; is_visible()->bool}` — sadece panel/popup'lar arasında TEK bir first-responder event-chain (`Vec<&mut dyn Component>`, ilk `Consumed` durur) sağlamak için.
- **KAYNAK:** `[gitui-component]` — aynı ratatui ekosisteminde, en düşük-risk/en az invasive seçenek.
- **NE ZAMAN:** herdr P1/P2/P3'ün TAMAMINI birden istemiyorsa, İLK ADIM olarak sadece bu trait'i tanıtıp mevcut `render_X()` fonksiyonlarını bu trait'e taşımak — sonrasında P1 Compositor'ı bu trait'in üzerine inşa etmek kolaylaşır (Component zaten var olur).
- **NE ZAMAN KULLANMA:** gitui'nin kendi kaynağının itiraf ettiği gibi bu yaklaşım "composition by code" kalır — registry/sayfa-sistemi (P3) gerektiğinde yetersiz, sadece bir ARA-ADIM olarak değerli.

### [P7] Lua/script-taşınan layout ağacı (novel, YÜKSEK maliyet) · conf 0.9
- **NE:** Bir scripting VM (Lua) `ui.Layout` (host'un layout tipini saran binding) ile bir component ağacı kompoze eder; host bu ağacı `TypeId`-eşlemeli KAPALI bir primitive kümesine (Line/List/Table/...) indirger ve render eder.
- **KAYNAK:** `[yazi-ui-layout]`, `[yazi-lua-components]`, `[yazi-renderable]`, `[yazi-renderer]`.
- **NE ZAMAN:** herdr gerçek KULLANICI-scriptlenebilir panel/sayfa düzeni istiyorsa (Rust-tanımlı sayfalar YETMEZSE) — büyük yatırım, VM-embedding gerektirir.
- **NE ZAMAN KULLANMA:** Şu an için herdr'ın hedefi Rust-tanımlı sayfalar/panel'ler (routines page vb.) — bu pattern ŞİMDİ GEREKMİYOR, gelecek bir "gerçek plugin ekosistemi" fazı için not edilmeli.

## Anti-pattern'ler (YAPMA)

| Anti-pattern | Doğru |
|---|---|
| Hardcoded fixed-order popup if-chain (yazi Tier-2, ~9 tip) — her yeni popup tipi if-chain'e elle eklenir | P1 (Compositor stack) veya P3 (registry) — yeni popup/sayfa = 1 push/1 registry-satırı, if-chain'e dokunma yok |
| Flat god-struct, her panel/modal için 1 field (superfile, gitui App) | P3 (registry) veya en azından P6 (ortak trait + `Vec<&mut dyn Component>`) — yeni panel eklemek N yere dokunmayı gerektirmemeli |
| Popup'ları BSP/split-ağacının İÇİNE zorlamak (bir "pane" gibi) | P2 — floating/popup katmanını ağacın TAMAMEN DIŞINDA, ayrı z-indexli bir yapıda tut (zellij deseni) |
| "Composition by code" (gitui'nin kendi itirafı) — her ebeveyn çocuklarını elle draw/event'e forward eder | P1/P3 registry+trait tabanlı, generic dispatch |
| Popup'ı arka-plan'ı yeniden-layout etmeden/hesaplamadan "üstüne bas" (resize sırasında arka plan bayatlar) | P4 — her katman her frame layout edilsin (cursive deseni), sadece render sırası z-order'a göre değişsin |
| Plugin/eklenti UI'sini host'un anlaması gereken yapısal bir ağaç olarak tasarlamak (yüksek bağlantı) | P2/zellij deseni — plugin sadece kendi rect'ine "render" eder (ANSI/grid çıktısı), host YAPIYI bilmek zorunda değildir |

## Ölçek / karar matrisi

| Durum | Pattern |
|---|---|
| Sadece birkaç sabit dialog/popup, minimum invaziflik | P6 (ortak Component trait) tek başına |
| Popup'lar + "routines" gibi addable sayfa gerekiyor | P1 (Compositor) + P3 (persistent chrome + registry) BİRLİKTE |
| BSP pane-tree'nin İÇİNDE de swappable slot gerekiyor (bir pane'i plugin/widget ile değiştirmek) | P2 (uniform Run-benzeri leaf enum) BSP ağacına eklenir |
| Popup'ların cursor/pane'e göre bağlamsal konumlanması gerekiyor (context-menu, hover-doc) | P1 + P5 (relative/anchor/zindex Rect hesaplama) |
| Arka planda canlı kalması gereken (resize'a duyarlı) modal popup'lar | P1 + P4 (her-frame-layout + modal-durdurma) |
| Gerçek kullanıcı-scriptlenebilir panel/plugin ekosistemi (uzun-vadeli, büyük yatırım) | P7 (Lua/script-taşınan ağaç) — ŞİMDİ değil, gelecek faz |

## Sonuç: herdr için önerilen sentez (üç mimari BİRLEŞTİRİLEREK)

1. **Dış çerçeve** (toolbar/sol-sağ panel/statusbar) = k9s'in persistent-chrome deseni (P3) — bir kez inşa edilir, sadece merkez "content" bölgesi değişir.
2. **Merkez bölge** = helix-tarzı bir `Compositor` (P1): layer 0 = herdr'ın MEVCUT BSP pane-tree render'ı (değiştirilmeden bir `Component`'e sarmalanır); yeni sayfalar (routines vb.) = layer-0'ı değiştiren alternatif `Component`'ler, k9s-tarzı bir registry'den (P3) `name → constructor` ile çözümlenir.
3. **BSP ağacının leaf'leri** = zellij-tarzı bir `Run`-benzeri enum'a genelleştirilir (P2) — böylece "swappable component slot" hem üst-seviye sayfa hem pane-içi slot için AYNI trait sözleşmesini paylaşır.
4. **Popup/overlay'ler** = P1'in üstüne push edilen ek katmanlar (helix `Popup<T>`/`Overlay<T>` generic decorator'ları), gerekirse P4 (modal-farkındalık) ve P5 (cursor/pane-göreli konumlandırma) ile güçlendirilir; BSP ağacının dışında kalırlar (zellij'in floating-panes kararına paralel).

---
*Kaynak: 6 paralel araştırma agent'ı (general-purpose), 2026-07-13 — zellij/helix/lazygit/gitui/k9s/neovim/emacs kaynak-kod+resmi-docs okuması + local refpool (yazi-src, superfile-src) + Rust ekosistem taraması (tui-realm, cursive, ratatui/templates). Detaylı kanıt tablosu: `docs/references/tui-composition.md`.*
