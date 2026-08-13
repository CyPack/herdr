---
doc: herdr-pattern-catalog
domain: custom-layout
created: 2026-07-24
status: canonical — her pattern dosya:satır + test/commit + confidence taşır
snapshot: feat/native-fm @ b48bd903
scope_note: >
  Bu katalog herdr'ın KENDİ layout altyapısından damıtılmış İÇ desenleri tutar (bölge/track/generation/
  transaction/persist). Dış TUI ekosistemi desenleri (Compositor, PageStack, floating z-index, Lua ağacı)
  KARDEŞ katalogda: docs/patterns/tui-composition.md — ÇAKIŞMA YOK, tamamlayıcıdır.
  tui-composition = "başkaları nasıl yapmış"; bu dosya = "herdr'da ne kanıtlandı".
agentic_triggers:
  - "custom layout · shell template · region · track · bölge ekle"
  - "yeni bölge · yeni yüzey · yeni template · layout DSL · kullanıcı layout'u"
  - "drag resize · divider · collapse · responsive degradation · geometry cache"
  - "stale hit · generation · fail-closed · resize transaction · persist round-trip"
related:
  - docs/references/custom-layout.md                # kaynak registry (bu kataloğun kanıt tablosu)
  - docs/analysis/2026-07-24-custom-layout-state.md # durum analizi (tasarım↔kod grid'i)
  - docs/patterns/tui-composition.md                # kardeş katalog (dış ekosistem desenleri)
  - docs/patterns/rust-engineering.md               # kardeş katalog (genel Rust disiplini)
---

# Pattern Kataloğu — custom-layout

> herdr'ın shell/layout alt sisteminde **sahada kanıtlanmış** desenler (SF0→SF6 + FM1→FM5, 2026-07).
> Her desen bu repodan çıkarıldı, dışarıdan devşirilmedi — kaynak dosya:satır + test adı + commit taşır.
>
> **Bu kataloğun ayırt edici değeri:** desenlerin çoğu ÜRETİMDE ÇALIŞIYOR ama bir kısmı ÜRETİME
> BAĞLI DEĞİL (kod var, şalter kapalı). Her pattern'de **durum** alanı bunu açıkça söyler —
> "kanıtlanmış" ile "bağlanmış" karıştırılmasın.

---

## ID uzayı

| Önek | Anlam | Aralık |
|---|---|---|
| `CL1..CL12` | Pattern (bu katalog) | 12 desen |
| `CLA1..CLA12` | Anti-pattern (bu katalog) | 12 madde |
| `CL-KM` | Karar matrisi (bu kataloğun sonu) | 1 tablo |

Kardeş domain'ler: `HP*` (rust-engineering) · `P*` (tui-composition) · `DR*`/`DA*` (document-rendering)
· `FM*` (native-file-manager). **Çakışma yok** — `CL*` bu domain'e ayrılmıştır.
`docs/references/custom-layout.md` "Desteklediği pattern" kolonu bu ID'lerle **birebir eşleşir**.

---

## Durum sözlüğü

| İşaret | Anlam |
|---|---|
| 🟢 **ÜRETİMDE** | Canlı render/input yolunda çalışıyor, testli |
| 🟡 **KANITLANMIŞ, BAĞLANMAMIŞ** | Kod + test tam, ama üretim yolu bu dalı çağırmıyor |
| 🔴 **KISMİ** | Bir yönü çalışıyor, karşı yönü eksik |

---

## Pattern kataloğu

### [CL1] Bounded named-region ağacı + fail-closed validation · conf 0.95 · 🟢 ÜRETİMDE (legacy yolla)

- **NE:** Layout, isimli bölgelerden (`RegionId`) oluşan serileştirilebilir bir `Split|Slot` ağacıdır.
  Ağaç, projeksiyona ulaşmadan ÖNCE tek bir iteratif geçişle doğrulanır; doğrulama başarılıysa
  `ValidatedShellLayout` tipi üretilir (**geçerlilik tip-seviyesinde taşınır**, bool ile değil).
  Her sınır sabit ve sayılabilir: `MAX_NESTED_SPLIT_DEPTH=4`, `MAX_SPLIT_CHILDREN=8`,
  `MAX_VISIBLE_LEAVES=64`, `MAX_SERIALIZED_NODES=128`, `MAX_STACK_CHILDREN=32`,
  `MAX_COMPONENT_PLACEMENTS=64`. 13 tipli hata (`ShellValidationError`), hiçbiri string değil.
- **KAYNAK:** `[shell-model]` `src/ui/shell/model.rs:9-14` (sabitler), `:170-266` (`validate`),
  `:120-135` (hata enum'u), `:105-117` (`ValidatedShellLayout`).
- **NE ZAMAN KULLAN:** Dışarıdan (config/disk/kullanıcı) gelen HER layout ağacında. Doğrulama
  iteratiftir (stack tabanlı), özyineleme yok → derin/kötü-niyetli ağaç stack overflow yapamaz.
  Zorunlu invariant'ları hataya çevir: `MissingWorkspaceStage`, `CollapsedWorkspaceStage`,
  `DuplicateRegion` — yani "stage her zaman var ve asla tamamen kapalı değil" garantisi.
- **NE ZAMAN KULLANMA:** Ağaç tamamen derleme-zamanı sabitse (tek bir hardcoded split) bu makine
  fazladır — ama herdr'da template + persist + (gelecekte) config üç ayrı giriş noktası var,
  o yüzden gerekli.
- **DİKKAT:** `from_legacy_root` (`:146-153`) `tracks`'i BOŞ bırakır. Doğrulama geçer ama solver
  legacy dala düşer (CL2). "Valid" ≠ "track politikası uygulanıyor".

### [CL2] TrackPolicy: bölge başına bounded boyut sözleşmesi + tipli template · conf 0.95 · 🟡 KANITLANMIŞ, BAĞLANMAMIŞ

- **NE:** Her bölge bir `TrackPolicy` taşır: `Fixed{cells}` · `ContentBounded{min,max}` ·
  `Resizable{min,preferred,max}` · `Fill{weight}` · `Collapsed{restore}`. Solver bunları
  `TrackRequest`'e çevirip üç aşamada dağıtır: (1) min'leri karşıla, (2) `desired`'a kadar büyüt
  (stage için 1 hücre rezerve ederek), (3) kalanı `Fill` ağırlıklarına largest-remainder ile paylaştır.
  Template'ler bu politikaların isimli, kapalı kombinasyonlarıdır (5 adet).
- **KAYNAK:** `[shell-solver]` `src/ui/shell/layout.rs:279-347` (`allocate_lengths`), `:378-421`
  (`request_from_policy`), `:537-592` (`distribute_fill`); `[shell-template]` `src/ui/shell/template.rs`
  (`dock_track()` 3/5/9, `sidebar_track()` 4/26/40).
  Test: `desktop_workspace_template_solves_normal_compact_and_too_small` (`src/ui/shell.rs:996`),
  `typed_templates_validate_without_runtime_registry` (`:506`).
- **NE ZAMAN KULLAN:** Bir bölgenin boyutu "sabit / içerikten / kullanıcıdan / artan alandan"
  hangisiyle geliyorsa onu **veri olarak** ifade et; if-zinciriyle değil. Yeni bölge eklemek =
  bir `TrackPolicy` satırı, solver'a dokunmadan.
- **NE ZAMAN KULLANMA:** Tek bir dinamik + tek bir fill bölge varsa (bugünkü legacy durum) politika
  makinesi fazladır — nitekim üretim `request_from_legacy_size` (`:365-376`) fallback'ini kullanıyor.
- **⚠️ DURUM UYARISI:** Bu desen **üretimde ÇALIŞMIYOR**. `ShellLayout::default()` `tracks`'i boş
  bırakıyor (`[shell-model-legacy]`) → `request_for_child` (`:349-363`) hep legacy dala düşüyor.
  5 template'ten 4'ü üretimden erişilemez; `ShellTemplateId::` ÜRETİMDE tek kullanım
  `src/persist/snapshot.rs:75`'te sabit kodlu. **Şalter:** `src/ui.rs:303`.

### [CL3] Generation-anahtarlı geometri cache (retained path) · conf 0.95 · 🟢 ÜRETİMDE

- **NE:** Geometri projeksiyonu **tam bir anahtarla** cache'lenir: `ShellGeometryKey{area,
  layout_revision, constraints_revision, collapse_revision}`. Anahtar eşitse önceki `ShellView`
  **aynı generation ile** aynen döner (solver hiç çağrılmaz, koleksiyon klonlanmaz); anahtar
  değişirse generation tam bir kez ilerler. Her `ShellView` kendi anahtarını taşır — cache dışarıda
  bir map değil, değerin kendisindedir.
- **KAYNAK:** `[shell-view]` `src/ui/shell/view.rs:14-37` (anahtar), `:108-122` (`compute_shell_view`),
  `:139-167` (`project_changed_geometry`). Test:
  `geometry_cache_profile_counts_desktop_and_empty_hits_and_misses` (`:188`) — hit/miss sayaçlarını
  `render_prof` üzerinden ölçüyor.
- **NE ZAMAN KULLAN:** Uzak bağlantıda (SSH) her frame yeniden çözülmesi pahalı olan HER geometri.
  Kural: "kirli bir PTY satırı shell'i yeniden çözdürmemeli" — bu, retained-path testinin ta kendisi.
- **NE ZAMAN KULLANMA:** Geometri zaten O(1) ve girdi her frame değişiyorsa cache sadece dallanma ekler.
- **⚠️ KRİTİK TUZAK:** **Anahtar, geometriyi belirleyen HER girdiyi içermek ZORUNDADIR.** Bugünkü
  anahtar **template kimliğini İÇERMİYOR**; `LEGACY_DESKTOP_SHELL_LAYOUT_REVISION` sabit `1`
  (`src/ui.rs:135, :306`). Template seçilebilir hâle gelirse ve revision artmazsa cache **eski
  geometriyi doğru sanarak** döndürür → hit-test yanlış bölgeye gider. Yeni bir girdi eklerken
  anahtara da ekle, yoksa bu desen sessizce zehirlenir.

### [CL4] TEK resize transaction otoritesi (çok hedefli, tek yaşam döngüsü) · conf 0.98 · 🟢 ÜRETİMDE

- **NE:** Sürükle-boyutlandırma için sistemde **bir tane** transaction tipi vardır; hedefi tiplidir:
  `ResizeTargetId::{Shell(DividerId), Miller(MillerDividerId)}`. İkinci bir "drag state" YARATILMAZ.
  Yaşam döngüsü: `begin` → `preview*` (saf, effect üretmez) → `commit`/`cancel`/`terminal_resize`.
  Sınır olayları tek bir `ResizeUpdate{decision, mark_persistence_dirty, request_pty_resize}`
  döndürür — **preview asla bu isteği üretmez**.
- **KAYNAK:** `[shell-resize-target]` `src/ui/shell/interaction.rs:99-113`; `[shell-resize-update]`
  `:157-164`; `begin`/`begin_miller` `:662-690`; `commit`/`cancel`/`terminal_resize` `:752-790`.
  Adapter: `src/app/file_manager_miller.rs:102,:164,:236`.
  **Tarihsel kanıt:** FM2 planı ayrı `MillerTrioDrag`'ı KALDIRDI ve bu transaction'a taşıdı
  (`.codex/TASKS.md` FM2 bloğu: *"Do not retain two resize authorities."*).
- **TESTLER (13):** `divider_down_captures_original_constraints` (`interaction.rs:830`),
  `divider_double_click_resets_to_preferred_once` (`:978`),
  `miller_divider_down_starts_typed_capture` (`src/app/input/file_manager.rs:2061`),
  `miller_resize_projection_tracks_active_owner_after_commit` (`:2110`),
  `miller_resize_profile_counts_transaction_changes_and_commit` (`:2190`),
  `miller_resize_profile_covers_keyboard_preview_and_commit` (`:2278`),
  `miller_resize_1000_moves_has_bounded_side_effects` (`:2586`),
  `miller_resize_escape_cancels_preview_without_closing_files` (`:3648`),
  `miller_resize_keyboard_preview_and_enter_commit_once` (`:3725`),
  `route_client_input_files_escape_cancels_miller_resize_without_pty_leak` (`src/app/mod.rs:4655`),
  `sidebar_divider_drag_is_preview_only_until_mouse_up` (`src/app/input/sidebar.rs:1785`),
  `sidebar_divider_mouse_up_is_the_commit_boundary` (`:1805`),
  `stale_divider_generation_is_consumed_inert` (`interaction.rs:1049`).
- **NE ZAMAN KULLAN:** İkinci bir sürüklenebilir kenar eklerken. Yeni hedef = `ResizeTargetId`'ye
  bir varyant; reducer'a DOKUNMA. Bu, "yeni bölge eklemek ucuz" iddiasının somut dayanağıdır.
- **NE ZAMAN KULLANMA:** Sürükleme yoksa (sadece klavyeyle boyut) transaction fazla olabilir — ama
  herdr'da klavye yolu da AYNI transaction'ı kullanıyor (`preview_keyboard_resize_step`, `:636`),
  bu da iki kod yolunu tek semantiğe indirdiği için tercih edildi.
- **ANTI-PATTERN'dan KAÇIN:** Bileşene özel drag state (`XDrag`, `YDrag`) — iki otorite kaçınılmaz
  olarak birbirinden ayrışır (FM2'nin düzelttiği tam da buydu).

### [CL5] Generation-kapılı hit + stale fail-closed (ASLA alias) · conf 0.95 · 🟢 ÜRETİMDE

- **NE:** Konumsal otorite SADECE mevcut generation'a karşı çözülür: `region_hit_at(generation, pos)`
  generation uyuşmazsa `None` döner. Hit alanları düzleştirilmiş, sıralı ve sınırlı bir listede
  tutulur (`Vec<ShellHitArea>`, `.rev()` ile topmost). Kimlikler **tam** taşınır: `MillerDividerId`
  hem `files_generation:u32` hem `model_revision:u64` hem de tam `PathBuf` içerir — koordinat değil,
  KİMLİK karşılaştırılır.
- **KAYNAK:** `[shell-view]` `src/ui/shell/view.rs:86-105`; `[shell-divider-id]`
  `src/ui/shell/interaction.rs:51-97`; `[shell-view-exhaust]` `:139-156`.
- **EXHAUSTION KURALI (kritik incelik):** generation `checked_add` ile artar; taşarsa **generation
  ilerlemez, `hits` BOŞALTILIR** ve yeni geometri yine de görünür kalır. Yani tükenme durumunda
  sistem "eski bir generation'a geri sarmaz" (alias yok) — sadece etkileşimsiz kalır.
  Test: `instance_generation_exhaustion_fails_without_aliasing` (`src/ui/surface_host.rs:316`),
  `stale_divider_generation_is_consumed_inert` (`interaction.rs:1049`).
- **NE ZAMAN KULLAN:** Geometrinin frame'ler arası değişebildiği HER hit-test. Relayout sonrası
  aynı piksel yeni geometriye karşı yeniden çözülmeli; eski hedef sessizce tüketilmeli
  (`ConsumedStale`), asla "en yakın" hedefe düşürülmemeli.
- **NE ZAMAN KULLANMA:** Geometri kesinlikle sabitse (statik tam ekran) generation gereksiz yüktür.
- **ANTI-PATTERN'dan KAÇIN:** Sadece `Rect`'e dayalı hit otoritesi. Relayout'tan sonra eski koordinat
  yeni bir bölgeye denk gelir ve **yanlış hedefe mutasyon** uygular — rehber §6 bunu açıkça
  reddedilmiş sayıyor.

### [CL6] Collapse + kademeli responsive degradation merdiveni · conf 0.9 · 🔴 KISMİ

- **NE:** Alan yetmediğinde layout çökmez; **isimli ve sıralı** basamaklardan iner:
  `Workspace → Wide → Standard → Compact → TooSmall`. Yatay merdiven: RightPanel collapse →
  LeftPanel compact (4 hücre) → AppDock collapse → TooSmall. Dikey: BottomBar → TopBar → TooSmall.
  Her basamaktan sonra "yetti mi?" yeniden ölçülür. Collapse **committed** bir durumdur ve
  `restore` genişliğini hatırlar (`TrackPolicy::Collapsed{restore}`); geçici preview asla bu
  durumu yazmaz.
- **KAYNAK:** `[shell-solver-degrade]` `src/ui/shell/layout.rs:443-503`, `minimum_required` `:525-535`;
  `RegionCollapseState` + `collapse`/`expand` `src/ui/shell/interaction.rs:169-193, :486-540`.
  Test: `collapse_remembers_last_committed_width` (SF3.2 RED, `.codex/TASKS.md` SF3 bloğu),
  `collapsed_sidebar_exposes_no_divider_capture` (`src/app/input/shell.rs:837`).
- **NE ZAMAN KULLAN:** Terminal genişliği kullanıcı kontrolünde olan HER çok-bölgeli layout.
  Basamakları **isimlendir** — "responsive" bir boolean değil, sıralı ve test edilebilir bir enum olsun.
- **NE ZAMAN KULLANMA:** Tek bölge varsa; ya da mobil gibi tamamen ayrı bir kompozisyon devreye
  giriyorsa (herdr mobile'da tam olarak bu yapılıyor: bölgeler kasten boş).
- **⚠️ DURUM UYARISI:** Merdivenin kodu tam ama **legacy layout'ta yalnız LeftPanel-compact basamağı
  erişilebilir** — RightPanel ve AppDock bölge olarak yok, `collapse_region` false döner.
  Şalter (CL2) açılmadan degradation'ın çoğu ölçülemez.

### [CL7] Bounded zincir + kayan viewport (sonsuz büyümeyi sayılabilir kılma) · conf 0.95 · 🟢 ÜRETİMDE

- **NE:** Mantıksal olarak sınırsız büyüyebilen bir yapı (dizin zinciri) iki katmana ayrılır:
  (a) **ucuz mantıksal zincir** — `VecDeque<MillerPathSegment{directory, preferred_width}>`,
  sert tavan `MAX_MILLER_HISTORY_DEPTH=32`; (b) **kayan görsel pencere** —
  `MillerHorizontalViewport{offset_cells:u32, follow_active:bool}`. Render maliyeti GÖRÜNÜR kolon
  sayısına bağlıdır, zincir derinliğine değil. Genişlikler bounded: 16 / 28 / 64, detay ≥ 20.
- **KAYNAK:** `[miller-bounds]` `src/fm/miller.rs:10-14`; `[miller-model]` `:15-59`; `seed()`
  `truncate(32)` `:62-79`. Test: `assert_miller_invariants_for_test` (`:228`).
- **KAYDIRMA TESTLERİ (6):** `fractional_scroll_uses_each_leading_columns_own_width`
  (`src/app/input/file_manager.rs:7346`),
  `plain_wheel_over_empty_trail_body_uses_fractional_horizontal_fallback` (`:7427`),
  `fractional_scroll_resize_clamps_and_navigation_rearms_auto_follow` (`:7658`),
  `shift_wheel_scrolls_deep_trail_left_and_persists_render_origin` (`:7739`),
  `grouped_miller_header_wheel_moves_owning_column_not_horizontal_offset` (`:7037`),
  `fcl_input_trail_horizontal_scroll_never_moves_the_locations_rail` (`:10021`).
- **NE ZAMAN KULLAN:** "Kullanıcı istediği kadar derine inebilsin" gereksinimi + sabit bellek/latency
  bütçesi çakıştığında. Büyüme **sayıları** değiştirir, **şekli** değiştirmez.
- **NE ZAMAN KULLANMA:** Eleman sayısı doğal olarak küçük ve sabitse.
- **TARİHSEL NOT:** Bu desenin erken hâlinde ayrıca `MAX_RESIDENT_MILLER_COLUMNS=5` bir resident
  projeksiyon cache'i vardı; Miller Trail T7 ile **emekliye ayrıldı**
  (`.codex/NEXT-SESSION-PROMPT.md:118-121`). Mimari rehber §3 hâlâ eski sayıyı yazıyor — **rehber
  bu noktada bayat.** Ders: bounded desen korunur, SAYILAR revize edilebilir; dokümantasyon
  sayılara değil şekle bağlanmalı.

### [CL8] Tipli Stage surface: tek sahip, tipli otorite, sınırlı örnek · conf 0.95 · 🟢 ÜRETİMDE

- **NE:** Ekranın merkezi alanını **tam olarak bir** tipli yüzey sahiplenir. Render seçimi bir
  boolean'dan değil, tipli projeksiyondan gelir: `match stage.surface_view() { NativeFiles => …,
  TerminalWorkspace => … }`. Gizli yüzey **hiçbir geometri projekte etmez** ve resize yan etkisi
  ALMAZ. Örnekler sabit kapasiteli (`[Option<AppInstance>; 16]`), kimlik `{app, generation:u32}`.
  Yüzey değişimi, gizlenen yüzeyin geometrisini **aynı mutasyonda** emekliye ayırır.
- **KAYNAK:** `[stage-host]` `src/ui/surface_host.rs:36-83`; `surface_view()` `:106-115`;
  `[ui-stage-guard]` `src/ui.rs:838-851`.
  Test: `active_surface_alone_populates_stage_hits` (`surface_host.rs:381`),
  `hidden_surface_has_no_stale_hits_or_cursor` (`:466`),
  `stage_surface_switch_does_not_destroy_terminal_runtime` (`:535`).
- **YÜZEY TÜRÜ SEÇİMİ (uzantı noktası):** İçerik-türü kararı ayrı ve **saf** bir katmandadır:
  `PreviewCapability{NativeText, NativeImage, MetadataOnly, OptionalPlugin{action_id,fallback},
  Unsupported}` (`src/fm/preview_capability.rs:44-58`). Bu fonksiyon dosya sistemine, `PATH`'e,
  config'e DOKUNMAZ (`:1-5`). PDF/XLSX/DOCX **zaten sınıflandırılmış** (`:126-136`).
- **NE ZAMAN KULLAN:** Aynı alanı paylaşan iki+ farklı uygulama/yüzey olduğunda. Tipli otorite,
  "iki yüzey aynı anda çizim yapıyor" hata sınıfını **derleme zamanında** imkânsız kılar.
- **NE ZAMAN KULLANMA:** Tek bir yüzey varsa.
- **⚠️ GENİŞLEME MALİYETİ (kapalı enum bedeli):** Üçüncü bir yüzey eklemek şu noktalara dokunmayı
  gerektirir: `BuiltInAppId` (`:4-7`), `index()` (`:10-15`, `0/1` sabit),
  **`last_generations: [Option<u32>; 2]` (`:82`) — dizi boyutu 2, büyütülmezse index panic**,
  `Default` (`:231`), `AppSurfaceRef`+`StageSurfaceView` (`:37-46`), `AppInstance::built_in` (`:61-67`),
  `activate_files`→genel `activate` (`:117`), render match (`src/ui.rs:843`), dock ikonu
  (`src/ui/app_dock.rs:36-45`), `PinnedBuiltinAppV1` (`src/persist/snapshot.rs:44-48`).
  `LaunchPolicy` bugün tek varyant `Singleton` (`:26-28`) — eşzamanlı çoklu örnek isteniyorsa
  `Multi{max}` gerekir (kapasite zaten 16, eksik olan politika).

### [CL9] Saf render + no-op çizim yolu · conf 0.95 · 🟢 ÜRETİMDE

- **NE:** Render **hiçbir durum mutasyonu yapmaz**; aynı state iki kez çizilince byte-eşit buffer üretir.
  Geometri `compute_view`'da hesaplanır, `render` sadece çizer. Çizilmeyecek bir bölge için render
  **no-op**'tur: rect boşsa çizim atlanır — koşul render'ın kendisindedir, çağıranda değil.
  Render'da saat, rastgelelik veya dosya sistemi okuması YOKTUR.
- **KAYNAK:** `[ui-compositor]` `src/ui.rs:790-800`; `[ui-dock-dark]` `:829-840` (boş-rect guard'ın
  canlı örneği); `[preview-purity]` `src/fm/preview_capability.rs:1-5`.
  Test: çift-çizim byte-eşitlik testi `src/ui.rs:1745` ("BOTH stage surfaces"),
  `terminal_dirty_row_keeps_retained_path_with_static_shell` (SF4.3-05).
- **NE ZAMAN KULLAN:** Sunucu-taraflı render + ince istemci mimarisinde ZORUNLU — diff katmanı
  yalnızca gerçekten değişen hücreleri gönderebilsin diye. Ayrıca deterministik görsel testin ön koşulu.
- **NE ZAMAN KULLANMA:** Hiç. (Bilinen tek istisna kaynakta bir temizlik adayı olarak kayıtlı:
  `render_projects_list` içindeki `SystemTime::now()` — rehber §4.3 bunu "precedent değil" diye işaretliyor.)
- **YAN FAYDA:** Boş-rect no-op sayesinde bir bileşen (AppDock) **kod tam olduğu hâlde** sıfır
  görsel etkiyle repoda durabilir. Bu, "önce bileşen, sonra yerleşim" sırasıyla çalışmayı mümkün kılar
  — ama **ölü kod riski** de yaratır (bkz. anti-pattern tablosu).

### [CL10] Versiyonlu + doğrulanmış persist round-trip · conf 0.9 · 🔴 KISMİ (yazıyor, geri uygulamıyor)

- **NE:** Sunum tercihleri, runtime'dan ayrı, **versiyonlu ve bounded** bir DTO'da saklanır:
  `ShellSnapshotV1{schema_version, template, root, region_constraints, component_placements,
  collapse_restore_widths, pinned_dock_order}`. Diskten okunan ağaç, **canlı ağaçla aynı**
  `validate` kapısından geçer (`validate_persisted_shell_parts`). İki bağımsız versiyon vardır:
  session (`SNAPSHOT_VERSION=4`) ve shell şeması (`SHELL_SNAPSHOT_VERSION=1`); gelecek versiyon reddedilir,
  eski versiyon migrate edilir (v3 sidebar genişliği → v4). DTO runtime/focus/hover/capture/geometri
  İÇERMEZ — sadece kullanıcı tercihleri.
- **KAYNAK:** `[persist-shell-v1]` `src/persist/snapshot.rs:28-42`; `[persist-versions]` `:15-18`;
  `[persist-migration]` `:57, :118-127, :635-650`; `[persist-validate]` `src/ui/shell.rs:55-61`.
  Test: `valid_v4_shell_json` (`:727`), `v4_session_with_shell_json` (`:772`).
- **NE ZAMAN KULLAN:** Kullanıcının elle ayarladığı HER geometri tercihi. Kural: **yazdığın şemayı
  okurken de aynı doğrulamadan geçir** — disk güvenilmez bir girdidir.
- **NE ZAMAN KULLANMA:** Tercih oturumluksa (geçici preview) — nitekim preview asla persist'e yazmaz (CL4).
- **⚠️ DURUM UYARISI — YARIM ROUND-TRIP:** Restore SADECE `restored_left_panel_preference()` okuyor
  (`src/app/mod.rs:438, :954`). `root`, `region_constraints`, `component_placements`,
  `pinned_dock_order` **yazılıyor, doğrulanıyor, ama geri uygulanmıyor.** Dahası
  `from_left_panel_preference` template'i **sabit** `DockSidebarStage` yazıyor (`snapshot.rs:75`)
  → **disk, çalışan sistemi temsil etmiyor.** Miller kolon genişlikleri hiç persist edilmiyor
  (`src/persist/` grep = 0). **Ders:** round-trip'in yazma yarısı tek başına değersizdir; okuma
  yarısı olmadan sadece yanıltıcı bir dosya üretir.

### [CL11] Client-local sunum sınırı (runtime/protocol'e sızmama) · conf 0.9 · 🟢 ÜRETİMDE

- **NE:** Layout/geometri/collapse/scroll durumu **istemci-yereldir**; `protocol` veya `api::schema`
  içinde görünmez. Paylaşılan runtime gerçekleri (workspace/tab/pane kimliği, PTY runtime, plugin
  kaydı) sunucudadır. Sınır yazılı olarak modül doc'unda beyan edilir.
- **KAYNAK:** `[shell-clientlocal]` `src/ui/shell.rs:12-16` birebir: *"Pure TUI presentation…
  none of these types are shared runtime facts, and none appear in `protocol`/`api::schema`."*
  Karşı taraf: `[plugin-registry]` `src/persist/plugin_registry.rs` → `plugins.json` +
  `api::schema::InstalledPluginInfo` (SUNUCU).
- **NE ZAMAN KULLAN:** Yeni bir alan eklerken önce sınıflandır: "paylaşılan runtime gerçeği mi,
  istemci sunumu mu?" Bölge genişliği/collapse = sunum. Pane/agent/process durumu = runtime.
  İsimlendirme nötr olmalı (server tarafında "sidebar/row/card" gibi UI adları KULLANMA).
- **NE ZAMAN KULLANMA:** Tek-süreç, tek-istemci uygulamada bu ayrım yüktür — ama herdr sunucu-render
  + çoklu istemci olduğu için zorunludur.
- **⚠️ GERİLİM NOKTASI:** Çoklu monitör/istemci senaryosunda "her istemci kendi layout'unu görsün"
  istenirse, layout **paylaşılan runtime gerçeğine** dönüşmek zorunda kalır — bu, sınırın yeniden
  yorumlanması demektir, sessizce yapılmamalı (`research/multi-monitor-shared-view.md`).

### [CL12] Fail-closed input sahiplik zinciri · conf 0.9 · 🟢 ÜRETİMDE

- **NE:** Girdi sahipliği **sabit ve tükenmeli** bir sırayla çözülür:
  `topmost overlay → active capture → z-sıralı topmost hit → focused component → page → global →
  fail-closed`. "Fail-closed" = hiçbir sahip bulunamazsa olay **tüketilir ve hiçbir şey yapılmaz**;
  arka plana sızmaz. Aktif bir capture (sürükleme) bırakılana kadar move/up olaylarını her yerde
  sahiplenir. Engelleyici overlay varsa alttaki her şey (hit, scroll, ham terminal girdisi) inert olur.
  Gizli yüzey HİÇBİR girdi almaz.
- **KAYNAK:** `route_shell_input` + `shell_key_input_owner()` + `shell_mouse_input_owner(position)` +
  `blocking_overlay_active()` (SF4.2, closure head `20f659c1`);
  `.codex/evidence/shell-foundation-sf4-input-router-progress.md` (26 KB, 8 dilim).
  Test: 8 slice GREEN (`.codex/TASKS.md:1043-1070`); gizli-terminal mührü 8-tip olay matrisiyle.
- **NE ZAMAN KULLAN:** Çakışan girdi tüketicileri (overlay + sürükleme + odak + kısayol) olan HER TUI.
  Sıra **veri olarak** yazılsın; her tüketicide ad-hoc "ben mi almalıyım?" kontrolü yapılmasın.
- **NE ZAMAN KULLANMA:** Tek girdi tüketicisi varsa.
- **NEDEN "fail-closed":** Sahipsiz olayı en alta düşürmek, kullanıcının görmediği bir bileşene
  yanlışlıkla komut göndermek demektir. Sessiz yutmak, yanlış hedefe uygulamaktan iyidir.
- **GENİŞLEME NOTU:** Yeni bölge eklemek bu zinciri **DEĞİŞTİRMEZ** — bölge sadece hit listesine
  girer (CL5). Miller, shell'in transaction'ını hiç dallanma eklemeden kullandı; bu, zincirin
  genişlemeye hazır olduğunun sahadaki kanıtıdır.

---

## Anti-pattern'ler (YAPMA)

| ID | Anti-pattern (YAPMA) | Doğru | Kanıt |
|---|---|---|---|
| **CLA1** | Bileşene özel ikinci drag state (`MillerTrioDrag`) | CL4 — tek transaction, tipli hedef ekle | FM2 planı bunu KALDIRDI: *"Do not retain two resize authorities"* |
| **CLA2** | Sadece `Rect`'e dayalı hit otoritesi (generation yok) | CL5 — generation + tam kimlik; stale = `ConsumedStale` | rehber §6 reddedilmiş listesi; `stale_divider_generation_is_consumed_inert` |
| **CLA3** | Geometri anahtarına yeni girdiyi eklemeyi unutmak | CL3 — anahtar HER belirleyici girdiyi içermeli | R2: `ShellGeometryKey` template kimliği içermiyor, `LEGACY_…REVISION` sabit `1` |
| **CLA4** | Render/input içinde dosya sistemi veya metadata okuması | CL9 — bounded worker şeridine taşı | rehber §6; V1-L6; `preview_capability.rs:1-5` |
| **CLA5** | Persist'in sadece yazma yarısını yapmak | CL10 — okuma yarısı olmadan dosya yanıltıcıdır | `snapshot.rs:28-42` yazıyor ⟷ `app/mod.rs:438` sadece width okuyor |
| **CLA6** | Disk şemasına runtime'da kullanılmayan bir "doğru" değer yazmak | CL10 — yazdığın değer çalışan sistemi temsil etsin | `snapshot.rs:75` sabit `DockSidebarStage` ⟷ üretim `ShellLayout::default()` |
| **CLA7** | Sınırsız görünür kolon/panel zinciri | CL7 — mantıksal zincir bounded, görsel pencere kayan | rehber §6; `MAX_MILLER_HISTORY_DEPTH=32` |
| **CLA8** | İkinci tüketici yokken component registry inşa etmek | Bekle — S5 tetikleyicisi "ÜRETİMDE ÇİZİLEN ikinci bileşen" | `TASKS.md:1954`; rehber §6 "P4.0 S5 NO-GO" |
| **CLA9** | Bileşeni yazıp yerleşime bağlamadan "bitti" demek | CL9 no-op çizim bunu MÜMKÜN kılar ama ölü kod üretir — durum alanını açıkça işaretle | AppDock: kod tam, `ui.rs:833` guard yüzünden hiç çizilmiyor |
| **CLA10** | Layout'u sunucu durumuna taşımak (çoklu istemci için) | CL11 — önce sınıflandır; sınır değişecekse açıkça karar ver | `src/ui/shell.rs:12-16` vs `research/multi-monitor-shared-view.md` |
| **CLA11** | Kilitli bir ürün kompozisyonunu "bug fix" diye değiştirmek | Layout V2 kararı aç | `files-layout-v1-lock.md`: *"No agent may silently reinterpret a V2-class change as a V1 bug fix"* |
| **CLA12** | Mimari rehberi tek gerçeklik kaynağı saymak | Kaynağı oku; rehber bayatlar | Rehber §7 tablosu 6 faz geride; §3 resident-bound emekli |

---

## KARAR MATRİSİ — "Ne eklemek istiyorum?"

| İstenen | Yol | Dokunulacak yer | Maliyet | Karar kapısı |
|---|---|---|---|---|
| **Mevcut bir bölgenin boyut kuralını değiştir** (ör. sidebar min/max) | CL2 | `template.rs` ilgili `*_track()` | **Çok düşük** — 1 satır | Yok (V1 görsel baseline'ı etkilerse VIS kontrolü) |
| **Yeni bir sürüklenebilir kenar** ekle | CL4 | `ResizeTargetId`'ye varyant + adapter | **Düşük** — reducer'a dokunma yok | Yok |
| **Var olan bir bölgeyi GÖRÜNÜR yap** (AppDock/TopBar/RightPanel/BottomBar) | CL2 + CL3 | `src/ui.rs:303` şalter + `:306` revision + `:313` resolve_dynamic | **Orta (teknik) / YÜKSEK (yönetişim)** | ⛔ **Files Layout V2 kararı** (dört-yüzey kompozisyonu değişir) + VIS-07..25 yenileme |
| **Yeni bir shell bölgesi** ekle (ör. RightRail) | CL1 + CL2 + CL5 | `RegionId` varyantı + `FLATTENED_REGION_ORDER` (`view.rs:5-12`) + template + track | **Orta** | ⛔ V2 kararı; ayrıca `RegionRects` compatibility eşlemesi (`model.rs:337-350`) gözden geçirilmeli |
| **Yeni içerik türü** (PDF/XLSX tablo, belge) — Files içinde detay | CL8 | `TrailDetailPreview` + `PreviewCapability` varyantı + `PreviewProviderSet.documents` + bounded worker + detay render | **Düşük-Orta** | ✅ **V1.x — V2 GEREKMEZ** (V1-L5 "file activation updates the detail state" öngörüyor) |
| **Yeni tam-ekran yüzey** (üçüncü Stage app'i) | CL8 | 11 nokta (bkz. CL8 genişleme maliyeti); **`last_generations: [_; 2]` → `[_; 3]` ZORUNLU** | **Yüksek** | ⛔ V2 + şalter (dock ikonu görünmeden anlamsız) + `PinnedBuiltinAppV1` şema uyumu |
| **Aynı anda birden çok belge/örnek** aç | CL8 | `LaunchPolicy::Multi{max}` varyantı | **Orta** | Kapasite zaten 16; sadece politika + UI (sekme?) kararı |
| **Yeni built-in template** ekle | CL2 | `ShellTemplateId` varyantı + `build()` kolu | **Düşük** — ama şalter olmadan erişilemez | Şaltere bağımlı |
| **Kullanıcı kendi layout'unu tanımlasın** (config/DSL) | CL1 + CL2 + CL10 | `config/model.rs` layout alanı + `template.rs:10` v0 kararını kaldır + şalter + restore'u tamamla | **Orta** — parser YAZILMAYACAK (`Deserialize` hazır, `model.rs:287-292`) | ⛔ Ürün kararı (v0 "no DSL" reddi) + şalter + V2 |
| **Kolon genişliklerini kalıcı kıl** | CL10 | `ShellSnapshotV1`'e bounded map (≤32) + restore | **Düşük** | Yok |
| **Persist round-trip'i tamamla** (ağaç geri yüklensin) | CL10 | `app/mod.rs:438,:954` + `snapshot.rs:75` | **Düşük-Orta** | Yok — ama şalter olmadan görünür etkisi yok |
| **Popup/overlay yığını** (iç içe, close-ordering) | CL12 + kardeş katalog | `docs/patterns/tui-composition.md` P1/P4/P5 | — | ⛔ S7 trigger-gated (`TASKS.md:1959`) |
| **Bölgeye takılabilir component registry** | — | — | **Yüksek** | ⛔ **S5 NO-GO** (`TASKS.md:1954`) — tetikleyici: ÜRETİMDE ÇİZİLEN ikinci bileşen. Bugün hiçbiri çizilmiyor → tetikleyici SAYILMAZ |
| **Mobile'da bölgeler** | — | `mobile.rs` + `ui.rs:600-608` | **Yüksek** | Kapsam kararı gerekli (bugün kasten dışarıda) |
| **Her istemci farklı layout görsün** | CL11 ihlali | Sunucu `AppState` ayrıştırması | **Çok yüksek** | ⛔ runtime/client guardrail'inin yeniden yorumlanması |

### Matrisin özeti — üç kova

1. **Serbest (kapı yok):** track sabiti değiştirme · yeni resize hedefi · kolon genişliği persist ·
   round-trip tamamlama · **belge/tablo detay yüzeyi (CL8, V1.x)**.
2. **Şaltere bağımlı (`src/ui.rs:303`) + V2 kararı:** bölge görünür kılma · yeni bölge · yeni template ·
   kullanıcı DSL'i · üçüncü Stage app'i.
3. **Kapalı (açık NO-GO / trigger-gated):** S5 component registry · S7 popup stack ·
   mobile bölgeleri · per-client layout.

> **Pratik sonuç:** Kullanıcının PNG/XLSX/PDF odağı **1. kovada** — hiçbir kapı beklemeden
> başlanabilir. Custom layout'un görünür kısmı **2. kovada** — teknik olarak küçük (bir şalter),
> yönetişim olarak büyük (V2 + B-chain design gate).

---

*Kaynak: herdr SF0→SF6 + FM1→FM5 programının kod+test+commit kanıtından damıtıldı, 2026-07-24.*
*Kanıt tablosu: `docs/references/custom-layout.md` · Durum analizi: `docs/analysis/2026-07-24-custom-layout-state.md`.*
*Dış ekosistem desenleri (Compositor/PageStack/floating/Lua): `docs/patterns/tui-composition.md`.*
