---
doc: herdr-analysis
domain: custom-layout
subject: shell template/region/track altyapısının tasarım↔kod durum analizi
created: 2026-07-24
method: kaynak okuma (src/ui/shell/*, src/ui.rs, src/persist, src/fm, src/ui/surface_host.rs) + spec/evidence/continuity taraması + hedefli grep doğrulaması
status: canonical — her iddia (claim, evidence=dosya:satır/test adı/commit, confidence)
snapshot: feat/native-fm @ b48bd903 (docs: pin directory click publication tip)
git_note: >
  /docs/* herdr .gitignore'da IGNORED (yalnız /docs/next/ istisna) → lokal yaşar, upstream'e sızmaz.
  Doğrulandı: `git check-ignore -v docs/patterns/tui-composition.md` → `.gitignore:10:/docs/*`.
  Makine kopyası: ~/.cartography/herdr-custom-layout-*
agentic_triggers:
  - "custom layout · layout · shell template · region · track · bölge"
  - "AppDock · TopBar · RightPanel · BottomBar · sidebar geometrisi"
  - "drag resize · divider · kolon genişliği · collapse · responsive"
  - "B-chain · B1 · B2 · B3 · B4 · layout DSL · kullanıcı tanımlı layout"
  - "ShellLayout · RegionRects · ShellView · ResizeTransaction · StageSurfaceView"
related:
  - docs/references/custom-layout.md
  - docs/patterns/custom-layout.md
  - docs/superpowers/specs/2026-07-17-herdr-custom-layout-architecture-guide.md
  - docs/superpowers/specs/2026-07-18-herdr-fip-closure-and-custom-layout-prd.md
  - docs/superpowers/specs/2026-07-19-herdr-files-layout-v1-lock.md
  - docs/patterns/tui-composition.md
  - docs/analysis/2026-07-24-architecture-seams.md   # kardeş analiz (katmanlar arası dikiş haritası)
---

# herdr Custom Layout Altyapısı — Durum Analizi (2026-07-24)

> **Kapsam:** shell/layout alt sisteminin İÇ detayı — tip tip, fonksiyon fonksiyon.
> **Metot:** salt-okuma. Bu turda hiçbir `cargo`/`just`/test komutu ÇALIŞTIRILMADI; test/gate
> sayıları `.codex` continuity kayıtlarının geçmiş beyanıdır, taze kanıt değildir (bkz. Kanıt Sözleşmesi).
> **Kardeş doküman:** katmanlar arası genel dikiş haritası `docs/analysis/2026-07-24-architecture-seams.md`.

---

## ÖZET — Üç cümle

1. **Altyapı KODLANDI ve TEST EDİLDİ ama ÜRETİMDE BAĞLI DEĞİL.** Named-region shell, 5 tipli
   template, bounded track solver, generation-güvenli `ShellView`, paylaşılan `ResizeTransaction`
   ve tam AppDock mevcut; üretim render'ı hâlâ `ShellLayout::default()` (eski 2 bölgeli
   `LeftPanel | WorkspaceStage`) kullanıyor.
2. **Kullanıcının asıl istediği iki etkileşim — kenar drag-resize ve yatay kaydırılabilir Miller —
   TAM ÇALIŞIYOR.** Bunlar shell template'inden bağımsız, Files stage'i içinde canlı; 19 isimli
   test kanıtı var.
3. **B-zinciri (custom layout programı) HİÇ BAŞLAMADI: 0/4.** Onaylı tasarımı YOK — ne design
   spec'i ne SYSTEM-MAP'i yazıldı; kendi brainstorm→design→plan kapısında bekliyor.

---

## 0. YEDİ SORUYA NET CEVAPLAR

### S1 — Olgunluk tek cümleyle?

**KISMİ ÜRETİM — "yazıldı, test edildi, ama şalteri açılmadı".** Alt sistem ikiye ayrılıyor:
**Miller/Stage/input/render ekseni ÜRETİMDE ve tam çalışıyor**; **shell-template/dock/persist ekseni
İSKELET — kod var, üretim render'ına bağlı değil.**

**Kanıt:** üretim `compute_view` hâlâ `ShellLayout::default()` çağırıyor (`src/ui.rs:303`), bu da
`from_legacy_root` ile SADECE `LeftPanel + WorkspaceStage` slot'u üreten, `tracks` haritası BOŞ olan
eski 2 bölgeli ağaç (`src/ui/shell.rs:70-93`, `src/ui/shell/model.rs:146-153`). Buna karşılık kolon
drag-resize ve yatay Miller scroll canlı (S5).

### S2 — Mimari rehber var mı, ne vaat ediyor, kaçı karşılanmış?

**VAR:** `docs/superpowers/specs/2026-07-17-herdr-custom-layout-architecture-guide.md`
(8.736 byte, 149 satır). Kendi ifadesiyle *"This guide is DERIVED, not invented"* — yani **tasarım
değil, dondurulmuş planlardan türetilmiş kılavuz**. Otorite sırası: çakışmada planlar kazanır.

**Vaadi (§1):** yatay kaydırılabilir Miller + her görünür kolon kenarında drag-resize (16..64,
tercih 28) + SSH-dostu performans mimarisi + bounded çoklu-nesne yönetimi.

**Skor: 26 maddede 13 ✅ · 7 ❓ · 6 ❌** (tam grid Bölüm C).

**⚠️ Rehber BAYAT:** §7 durum tablosu "SF5.1 NEXT, FM1/FM2 pending" diyor — oysa `.codex/TASKS.md:1114-1200`'de
SF5→FM5'in TAMAMI `[x]`. Ayrıca §3'teki `MAX_RESIDENT_MILLER_COLUMNS = 5` sabiti Miller Trail T7 ile
emekliye ayrılmış. **Rehber tek gerçeklik kaynağı olarak KULLANILMAMALI.**

### S3 — "B-chain" nedir, onaylı tasarımı var mı, başladı mı?

**Tanım:** `docs/superpowers/specs/2026-07-18-herdr-fip-closure-and-custom-layout-prd.md` §2, işi iki
paralel zincire ayırıyor:
- **A-zinciri** = FIP kapanışı (E2E mouse harness investigation → A6 final).
- **B-zinciri** = **Custom Layout altyapı programı** — Excalidraw mockup'ındaki layout'u
  (TopBar + çift LeftPanel + tab'lı CenterStage + RightRail + RightPanel + BottomBar) kurulabilir kılmak.

**ONAYLI TASARIMI/PLANI: YOK. Başlamadı: 0/4.**

| Adım | İstenen artefakt | Gerçek |
|---|---|---|
| B1 Keşif | `.cartography/custom-layout-SYSTEM-MAP.json` | ❌ YOK (dizinde 7 harita var, bu değil) |
| B2 Design spec | `docs/superpowers/specs/…custom-layout-design.md` | ❌ YOK |
| B3 Impl. plan | RED adları + GREEN seam'leri + VIS-ID | ❌ YOK |
| B4 Yürütme | katman katman RED→GREEN→Playwright→FF | ❌ YOK |

`.codex/TASKS.md`'de B1-B4 için **checkbox maddesi bile açılmamış** — program görev kaydına girmemiş,
sadece PRD metninde duruyor.

**Neyi bekliyor:** teknik blocker değil, **yönetişim kapısı**. Üç bağımsız continuity kaydı aynı şeyi söylüyor:
- `.codex/NEXT-SESSION-PROMPT.md:128` — *"Custom-layout B-chain is separate and **starts only from its own approved design/plan**."*
- `.codex/CURRENT.md:412` — *"then custom-layout B-chain **only under its own plan**."*
- `.codex/HANDOFF.md:601-604` — *"CUSTOM LAYOUT ALTYAPISI programı tasarlanacak (**kendi brainstorm→design→plan kapısıyla**)."*

Önkoşulu (FIP-1/FIP-2 kapanışı) karşılandı. PRD §2'ye göre B, A'dan bağımsız BAŞLAR ama A5'ten önce
BİTEMEZ. **Bugün B1'i başlatmanın önünde hiçbir kod bağımlılığı yok.** B1 için hazır girdi listesi
Bölüm H'de.

### S4 — Kullanıcı bugün kendi layout'unu tanımlayabiliyor mu?

**HAYIR.** Üç ayrı engel:

1. `src/config/model.rs`'de layout alanı **YOK** (grep=0; sadece `mobile_width_threshold` (`:924`) ve
   klavye düzeni (`:1046`) geçiyor).
2. 5 template tanımlı ama üretimde seçilemiyor — `ShellTemplateId::` ÜRETİMDE tek kullanım
   `src/persist/snapshot.rs:75`'te **sabit kodlu** `DockSidebarStage`; render onu hiç okumaz. Diğer 2
   kullanım `src/ui/app_dock.rs:193,:350` = TEST.
3. v0 kararı açıkça reddediyor: *"Foundation v0 exposes no arbitrary layout DSL"*
   (`src/ui/shell/template.rs:10`).

**EN KISA YOL — parser yazmaya gerek YOK.** `ShellLayout` **zaten `Deserialize`** ve hem ham ağaç hem
template kabul ediyor:

```rust
// src/ui/shell/model.rs:287-292
#[derive(Deserialize)] #[serde(untagged)]
enum SerializedShellLayout {
    Tree(SerializedShellTree),          // { root, tracks, stacks, component_placements }
    Template(SerializedShellTemplate),  // { template: "DesktopWorkspace" }
}
```

`deny_unknown_fields` + `validate()` ile fail-closed (13 tipli hata, `model.rs:120-135`). Yani
`~/.config/herdr/layout.json` okumak için **3 adım** yeter — detay Bölüm F-a.

### S5 — Drag-resize ve scrollable Miller: kodda var mı, testi var mı?

**İKİSİ DE VAR VE ÜRETİMDE ÇALIŞIYOR. Bu, rehberin asıl hedef etkileşimi ve TAM TESLİM EDİLMİŞ.**

**Drag-resize (kenar sürükleme):**
- Sabitler rehberle **birebir**: `MILLER_COLUMN_MIN_WIDTH=16`, `MILLER_COLUMN_PREFERRED_WIDTH=28`,
  `MILLER_COLUMN_MAX_WIDTH=64` (`src/fm/miller.rs:11-13`), `MILLER_DETAIL_MIN_WIDTH=20` (`:14`).
- **Tek otorite ilkesi tutuldu:** `ResizeTargetId::{Shell(DividerId), Miller(MillerDividerId)}`
  (`src/ui/shell/interaction.rs:101-104`) — ayrı `MillerTrioDrag` **silindi**, ikinci drag state yok.
- Kimlik + staleness: `MillerDividerId{files_generation:u32, model_revision:u64, leading/trailing:
  MillerResizeColumnId{directory: PathBuf, generation}, axis}` (`interaction.rs:53-59`).
- Adapter: `begin_miller_resize_capture` / `commit_miller_resize` / `handle_miller_resize_key`
  (`src/app/file_manager_miller.rs:102, :164, :236`), `handle_active_miller_resize_mouse`
  (`src/app/input/file_manager.rs:655`).
- Terminal-resize iptali: `cancel_miller_resize_for_terminal_area` (`src/app/input/shell.rs:291`),
  `src/ui.rs:271`'den çağrılıyor.

**Test kanıtı — 13 isimli test:**

| Test | Konum |
|---|---|
| `miller_divider_down_starts_typed_capture` | `src/app/input/file_manager.rs:2061` |
| `miller_resize_projection_tracks_active_owner_after_commit` | `:2110` |
| `miller_resize_profile_counts_transaction_changes_and_commit` | `:2190` |
| `miller_resize_profile_covers_keyboard_preview_and_commit` | `:2278` |
| `miller_resize_1000_moves_has_bounded_side_effects` | `:2586` |
| `miller_resize_escape_cancels_preview_without_closing_files` | `:3648` (async) |
| `miller_resize_keyboard_preview_and_enter_commit_once` | `:3725` (async) |
| `route_client_input_files_escape_cancels_miller_resize_without_pty_leak` | `src/app/mod.rs:4655` |
| `divider_down_captures_original_constraints` | `src/ui/shell/interaction.rs:830` |
| `divider_double_click_resets_to_preferred_once` | `:978` |
| `stale_divider_generation_is_consumed_inert` | `:1049` |
| `sidebar_divider_drag_is_preview_only_until_mouse_up` | `src/app/input/sidebar.rs:1785` |
| `sidebar_divider_mouse_up_is_the_commit_boundary` | `:1805` |

**Scrollable Miller (yatay viewport):**
- Model: `MillerState.chain: VecDeque<MillerPathSegment>` + `MillerHorizontalViewport{offset_cells:u32,
  follow_active:bool}` (`src/fm/miller.rs:32-45, :53-59`).
- Reducer: `handle_miller_horizontal_scroll` (`src/app/file_manager_miller.rs:290`),
  `fractional_scroll_step` / `horizontal_scroll_target` (`src/ui/file_manager/trail_view.rs:559, :597`).

**Test kanıtı — 6 isimli test:**

| Test | Konum |
|---|---|
| `fractional_scroll_uses_each_leading_columns_own_width` | `src/app/input/file_manager.rs:7346` |
| `plain_wheel_over_empty_trail_body_uses_fractional_horizontal_fallback` | `:7427` |
| `fractional_scroll_resize_clamps_and_navigation_rearms_auto_follow` | `:7658` |
| `shift_wheel_scrolls_deep_trail_left_and_persists_render_origin` | `:7739` |
| `grouped_miller_header_wheel_moves_owning_column_not_horizontal_offset` | `:7037` |
| `fcl_input_trail_horizontal_scroll_never_moves_the_locations_rail` | `:10021` |

### S6 — Layout kalıcılığı: yazılıyor mu, nerede, migration var mı?

**YAZILIYOR ama GERİ YÜKLENMİYOR — yarım round-trip.**

- **Nerede:** `ShellSnapshotV1` (`src/persist/snapshot.rs:28-42`), session snapshot'ının içinde.
  `SNAPSHOT_VERSION = 4` (`:16`), `SHELL_SNAPSHOT_VERSION = 1` (`:18`).
- **Şema:** `{schema_version, template: ShellTemplateId, root: ShellNode, region_constraints:
  BTreeMap<RegionId,TrackPolicy>, component_placements: Vec<ComponentPlacement>,
  collapse_restore_widths: BTreeMap<RegionId,u16>, pinned_dock_order: Vec<PinnedBuiltinAppV1>}` —
  yani **tam bir özel ağaç taşıyabiliyor.**
- **Migration VAR:** v3 sidebar genişliği → v4'e taşınıyor (`from_legacy_sidebar_width`, `:57`);
  geçersiz shell verisi kapsanıyor; gelecek versiyon reddediliyor (`:118-127` + `:635-650`);
  `validate_persisted_shell_parts` ile fail-closed (`src/ui/shell.rs:55-61`).
- **⚠️ EKSİK:** restore SADECE `restored_left_panel_preference()` okuyor (`src/app/mod.rs:438`, `:954`)
  — yani **width + collapsed**. `root`, `region_constraints`, `component_placements`,
  `pinned_dock_order` **yazılıyor, doğrulanıyor, ama asla geri uygulanmıyor.**
- **⚠️ TUTARSIZLIK:** `from_left_panel_preference` template'i **sabit** `DockSidebarStage` (dock'lu)
  yazıyor (`:75`), runtime ise `ShellLayout::default()` (dock'suz) çalıştırıyor → **dosya çalışan
  sistemi temsil etmiyor.**
- **⚠️ Miller kolon genişlikleri HİÇ persist edilmiyor:** `MillerPathSegment.preferred_width`
  (`src/fm/miller.rs:18`) sadece RAM'de; `src/persist/` içinde `column_width`/`preferred_width`
  grep = **0 sonuç** → oturum arası kayboluyor.

### S7 — Yeni yüzey türü (belge/tablo görüntüleyici) için minimum değişiklik kümesi?

**KRİTİK KEŞİF: sıfırdan başlamıyorsun — PDF/XLSX/DOCX ZATEN sınıflandırılmış.**

```rust
// src/fm/preview_capability.rs:126-136 — MEVCUT KOD
if matches_extension(extension.as_deref(), &[
    "pdf","doc","docx","odt","rtf","xls","xlsx","ods","ppt","pptx","odp",
]) {
    return plugin_or_fallback(providers.documents.as_ref(),
        PreviewFallback::MetadataOnly(PreviewReason::DocumentMetadata));
}
```

| Format | Bugünkü sınıflandırma | Kanıt | Eksik |
|---|---|---|---|
| PNG/JPG… | `PreviewCapability::NativeImage` → `TrailDetailPreview::Image` | `preview_capability.rs:109-111`; `trail_snapshots.rs:705` | **Piksel teslimatı YOK** — *"pixel delivery is the Kitty-graphics track (FIP-D4) and completes at integration"* (`trail_snapshots.rs:40-42`) |
| PDF/XLSX/DOCX | `OptionalPlugin{action_id, fallback}` veya `MetadataOnly(DocumentMetadata)` | `preview_capability.rs:126-136` | `documents` sağlayıcısı bağlı değil; render yok |
| Arşiv / medya | `archives` / `media` yuvası | `:137-161` | Aynı |

**İki yol var — tam liste Bölüm F-b'de.**
- **Yol B-1 (ÖNERİLEN, ucuz, V1.x):** detay yüzeyi olarak Files içinde. Layout V1-L5 *"file activation
  updates the detail state"* zaten öngörüyor → **V2 kararı GEREKMEZ.**
- **Yol B-2 (pahalı, tam Stage app'i):** `BuiltInAppId` kapalı enum'unu genişletme. **En kritik tuzak:**
  `StageState.last_generations: [Option<u32>; 2]` — **dizi boyutu 2** (`src/ui/surface_host.rs:82`);
  üçüncü app eklerken 3'e çıkarılmazsa `BuiltInAppId::index()` (`:10-15`) ile index panic.

---

## A. Kavram Haritası

herdr'da "layout" tek otorite değil — **üç bağımsız, iç içe geçmiş sistem**:

```
┌─ SUNUCU (paylaşılan runtime — protocol/api::schema) ────────────────────────┐
│  Workspace ─> Tab ─> Pane  (kimlik + PTY)   TerminalRuntimeRegistry         │
│  plugins.json / InstalledPluginInfo  (api::schema — SUNUCU tarafı)          │
└───────────────────────────────┬─────────────────────────────────────────────┘
                                │ TEK AppState · frame TÜM client'lara broadcast
┌───────────────────────────────▼─ İSTEMCİ (TUI presentation, client-local) ──┐
│                                                                             │
│ ① DIŞ KABUK — src/ui/shell/                                                 │
│   ShellLayout{ root: ShellNode(Split|Slot), tracks: BTreeMap<RegionId,       │
│                TrackPolicy>, stacks, component_placements }                 │
│   RegionId ∈ {TopBar, AppDock, LeftPanel, WorkspaceStage, RightPanel,        │
│               BottomBar, CenterContent(legacy)}                             │
│         │ validate()  → ValidatedShellLayout   (13 fail-closed hata)        │
│         │ solve(area) → RegionRects + ResponsiveDegradation                 │
│         │ compute_shell_view(ShellGeometryKey) → ShellView{generation,       │
│         │        regions, hits, geometry_key}  → region_hit_at(gen,pos)     │
│   ⚠️ ÜRETİM: ShellLayout::default() — SADECE 2 slot, tracks BOŞ             │
│                                                                             │
│ ┌──────────────┬──────────────────────────────────┬──────────────────────┐  │
│ │ LeftPanel    │       WorkspaceStage             │ AppDock · TopBar ·   │  │
│ │ (sidebar)    │  ┌── StageState.surface_view() ──┐│ RightPanel · Bottom │  │
│ │              │  │  16 instance · gen u32        ││ ⇒ rect BOŞ ⇒ ÇİZİLMEZ│ │
│ │              │  ▼                            ▼  ││                      │ │
│ │              │ TerminalWorkspace       NativeFiles│                     │ │
│ └──────────────┴───────┬──────────────────────┬───┴──────────────────────┘  │
│                        │                      │                             │
│ ② PANE AĞACI           │      ③ MILLER TRAIL  │                             │
│   src/layout.rs        │        src/fm/miller.rs                            │
│   TileLayout{Node}     │        MillerState{chain: VecDeque<MillerPath-      │
│   BSP · PaneId         │          Segment{directory, preferred_width}>,      │
│   SplitBorder          │          horizontal: MillerHorizontalViewport}     │
│                        │        chain≤32 · 16/28/64 · detail≥20             │
│                        │                │                                   │
│                        │                └─> TrailDetailPreview              │
│                        │                    {PendingText, Text, Image,      │
│                        │                     MetadataOnly, Unpreviewable}   │
│                        │                    ◄── PreviewCapability           │
│                        │                        (PDF/XLSX ⇒ OptionalPlugin) │
│                                                                             │
│ ④ TEK RESIZE OTORİTESİ — ResizeTargetId::Shell(DividerId)                   │
│                                       | ::Miller(MillerDividerId)           │
│ ⑤ RENDER — Compositor(Vec<Box<dyn Component>>) → saf çizim                  │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Kritik ayrım:** ① dış kabuğu böler (sidebar/dock/panel), ② `WorkspaceStage` içindeki terminal
pane'lerini böler, ③ Files stage'i içindeki dosya kolonlarını böler. **Klasik tiling pane-split ile
shell template'i AYNI ŞEY DEĞİL** ve birbirinin bölgesini üretemez.

---

## B. Bileşen Envanteri

| Bileşen | Dosya | Anahtar tipler | Durum | Test kanıtı | Kısıt |
|---|---|---|---|---|---|
| Bölge modeli + doğrulama | `src/ui/shell/model.rs` | `RegionId`, `TrackPolicy`, `ShellNode`, `ShellLayout::validate`, `ValidatedShellLayout`, `ShellValidationError`(13) | **Üretimde (legacy yolla)** | `typed_templates_validate_without_runtime_registry` (`shell.rs:506`) | depth≤4, children≤8, leaves≤64, nodes≤128, placements≤64, stacks≤32 |
| Tipli template'ler | `src/ui/shell/template.rs` | `ShellTemplateId{StageOnly, DockStage, DockSidebarStage, DesktopWorkspace, InspectorWorkspace}` | **İSKELET — üretimde seçilemez** | `desktop_workspace_template_solves_normal_compact_and_too_small` (`shell.rs:996`) | `template.rs:10`: "no arbitrary layout DSL" |
| Track solver | `src/ui/shell/layout.rs` | `solve`, `TrackRequest`, `allocate_lengths`, `distribute_fill`, `ResponsiveDegradation` | **Üretimde (kısmi)** | `shell.rs` 63 test fn, `tracked_horizontal_layout` matrisi | legacy'de `tracks` boş → `request_from_legacy_size` |
| Geometri projeksiyonu | `src/ui/shell/view.rs` | `ShellGeometryKey`, `ShellView`, `compute_shell_view`, `region_hit_at`, `flatten_region_hits` | **Üretimde** | `geometry_cache_profile_counts_desktop_and_empty_hits_and_misses` (`view.rs:188`) | gen exhaustion → `hits: Vec::new()` fail-closed |
| Resize/collapse/scroll reducer | `src/ui/shell/interaction.rs` | `ResizeTargetId`, `ResizeTransaction`, `ResizeBounds`, `MillerDividerId`, `RegionCollapseState`, `ScrollViewportState`, `ShellPresentationState` | **Üretimde** | 45 test fn (S5'te 13'ü listelendi) | preview asla persist/PTY yazmaz |
| Stage / surface host | `src/ui/surface_host.rs` | `BuiltInAppId`, `AppSurfaceRef`, `StageSurfaceView`, `LaunchPolicy`, `AppDefinition`, `StageState` | **Üretimde** | 8 test (`:253-647`) | **16 instance**, `last_generations: [_; 2]` |
| Miller kolon modeli | `src/fm/miller.rs` | `MillerState`, `MillerPathSegment`, `MillerHorizontalViewport`, `MillerAdjacentWidthTarget`, `commit_adjacent_column_widths` | **Üretimde** | `assert_miller_invariants_for_test` (`:228`) | chain≤32, 16/28/64 |
| Miller resize adapter | `src/app/file_manager_miller.rs`, `src/app/input/file_manager.rs` | `begin_miller_resize_capture`, `commit_miller_resize`, `handle_miller_resize_key`, `handle_miller_horizontal_scroll` | **Üretimde** | 19 isimli test (S5) | — |
| Önizleme yeteneği | `src/fm/preview_capability.rs` | `PreviewCapability{NativeText, NativeImage, MetadataOnly, OptionalPlugin, Unsupported}`, `PreviewProviderSet{markdown, documents, archives, media}`, `PreviewReason`(8) | **Üretimde** | `preview_capability.rs:198+` | saf — FS/PATH/process okumaz (`:3-5`) |
| Detay yüzeyi | `src/fm/trail_snapshots.rs` | `TrailDetailPreview{PendingText, Text, Image, MetadataOnly, Unpreviewable}` | **Kısmi** | — | `Image` = piksel teslimatı **FIP-D4, tamamlanmadı** (`:40-42`) |
| AppDock | `src/ui/app_dock.rs` | `AppDockModel`, `AppDockEntry`, `app_dock_entry_areas`, `render_app_dock` | **KOD TAM — ÜRETİMDE KARANLIK** | `app_dock.rs:193, :350` (test `DockStage`'i elle kurar) | rect boş → `ui.rs:833` guard false |
| Kalıcılık | `src/persist/snapshot.rs` | `ShellSnapshotV1`, `PinnedBuiltinAppV1`, `RestoredLeftPanelPreference` | **KISMİ — yazılıyor, geri uygulanmıyor** | `valid_v4_shell_json` (`:727`), `v4_session_with_shell_json` (`:772`) | snapshot v4 / shell schema v1 |
| Eklenti kaydı | `src/persist/plugin_registry.rs` | `InstalledPluginInfo` (api::schema), `plugins.json` | **Üretimde** | — | **SUNUCU tarafı** |
| Compositor | `src/ui/compose.rs` | `Compositor`, `RenderCtx`, `dyn Component` | **Üretimde** (`ui.rs:797`) | `ui.rs:1745` saflık testi | base chrome + tek overlay |
| Pane tiling | `src/layout.rs` | `TileLayout`, `Node`, `PaneId`, `SplitBorder`, `NavDirection`, `find_in_direction` | **Üretimde** | — | Shell'den bağımsız |
| Mobile | `src/ui/mobile.rs` (47 KB) | kendi header/terminal split'i | **Üretimde, shell'den AYRI** | — | `compute_empty_shell_view` → bölge yok |

---

## C. TASARIM ↔ KOD BOŞLUK GRİD'İ ⭐

Sol sütun = mimari rehberin **satır satır vaadi**. Sağ = kaynaktan doğrulanmış kod gerçeği.

```
  ── Custom Layout: mimari rehber vaadi ⟷ kod gerçeği ──  herdr @ b48bd903 · 2026-07-24
     🗂️ SOL  = docs/superpowers/specs/2026-07-17-herdr-custom-layout-architecture-guide.md
     🦀 SAĞ  = src/ (her satır kaynaktan okundu, grep+read çapraz doğrulama)
┌────┬─────────────────────────────────────────────┬────┬──────────────────────────────────────────────────────┐
│ #  │ 🗂️ REHBERİN VAAT ETTİĞİ YETENEK (§ referans) │ ⟷ │ 🦀 KODDA BULUNAN GERÇEK + KANIT                       │
├────┼─────────────────────────────────────────────┼────┼──────────────────────────────────────────────────────┤
│ 1  │ §1.1 Files, Stage'i TİPLİ surface olarak    │ ✅ │ StageSurfaceView::NativeFiles; render seçimi TİPLİ    │
│    │ sahiplenir — terminal curtain YOK           │    │ otoriteden: ui.rs:843-851 match stage.surface_view()  │
├────┼─────────────────────────────────────────────┼────┼──────────────────────────────────────────────────────┤
│ 2  │ §1.2 Miller = yatay viewport; çocuğa girince│ ✅ │ MillerState.chain: VecDeque<MillerPathSegment>        │
│    │ kolon EKLENİR (Finder gibi), viewport kayar │    │ miller.rs:53-59; visit() :80; MillerHorizontalViewport│
│    │ (wheel, Shift+wheel, header okları)         │    │ {offset_cells,follow_active} :32 — 6 isimli test (S5) │
├────┼─────────────────────────────────────────────┼────┼──────────────────────────────────────────────────────┤
│ 3  │ §1.3 HER görünür kolon kenarı drag divider; │ ✅ │ MILLER_COLUMN_MIN=16 / PREFERRED=28 / MAX=64          │
│    │ 16..64 arası (tercih 28); SF3 resize        │    │ miller.rs:11-13 — REHBERLE BİREBİR.                   │
│    │ transaction'ını YENİDEN KULLANIR — YENİ     │    │ ResizeTargetId::{Shell|Miller} interaction.rs:101-104 │
│    │ drag state YOK                              │    │ ⇒ TEK transaction ailesi. Ayrı MillerTrioDrag SİLİNDİ │
├────┼─────────────────────────────────────────────┼────┼──────────────────────────────────────────────────────┤
│ 4  │ §1.4 / §3 Mouse HER görünür kolonda;        │ ✅ │ MillerDividerId{files_generation:u32, model_revision: │
│    │ generation-denetimli; stale = ConsumedStale │    │ u64, leading/trailing: MillerResizeColumnId{directory │
│    │ SIFIR mutasyonla                            │    │ :PathBuf, generation}} interaction.rs:53-59.          │
│    │                                             │    │ stale_divider_generation_is_consumed_inert :1049      │
├────┼─────────────────────────────────────────────┼────┼──────────────────────────────────────────────────────┤
│ 5  │ §3 MAX_MILLER_HISTORY_DEPTH = 32 segment    │ ✅ │ miller.rs:10 = 32; seed() truncate(32) :78            │
├────┼─────────────────────────────────────────────┼────┼──────────────────────────────────────────────────────┤
│ 6  │ §3 MAX_RESIDENT_MILLER_COLUMNS = 5 resident │ ❌ │ Sabit KODDA YOK (grep=0). Miller Trail T7 ile         │
│    │ dizin projeksiyonu; aktif kolon tahliye     │    │ "retired parent/current/resident projection"          │
│    │ edilmez                                     │    │ NEXT-SESSION-PROMPT.md:118-121 ⇒ REHBER BAYAT         │
├────┼─────────────────────────────────────────────┼────┼──────────────────────────────────────────────────────┤
│ 7  │ §3 MAX_MULTI_SELECTION_PATHS = 4096 tavan   │ ✅ │ Seçim tavanı korunuyor (FM3 kapanış kaydı)            │
├────┼─────────────────────────────────────────────┼────┼──────────────────────────────────────────────────────┤
│ 8  │ §3 Stage 16-instance sınırı                 │ ✅ │ MAX_BUILT_IN_INSTANCES=16 surface_host.rs:1;          │
│    │                                             │    │ stage_rejects_more_than_sixteen_builtin_instances :292│
├────┼─────────────────────────────────────────────┼────┼──────────────────────────────────────────────────────┤
│ 9  │ §4.1 Retained shell path: değişmeyen        │ ✅ │ compute_shell_view: geometry_key eşitse previous      │
│    │ geometri anahtarı AYNI generation'la döner  │    │ AYNEN döner view.rs:114-117 + cache profil testi :188 │
├────┼─────────────────────────────────────────────┼────┼──────────────────────────────────────────────────────┤
│ 10 │ §4.2 TAM OLARAK BİR surface hesaplar;       │ ✅ │ pane geometrisi + rt.resize sadece TerminalWorkspace  │
│    │ gizli surface sıfır maliyet, resize yan     │    │ aktifken: ui.rs:840. active_surface_alone_populates_  │
│    │ etkisi almaz                                │    │ stage_hits (surface_host.rs:381) + hidden_surface_    │
│    │                                             │    │ has_no_stale_hits_or_cursor :466                      │
├────┼─────────────────────────────────────────────┼────┼──────────────────────────────────────────────────────┤
│ 11 │ §4.3 Saf render: aynı state = byte-eşit     │ ✅ │ Compositor ui.rs:797; çift-çizim byte-eşitlik testi   │
│    │ buffer, sıfır mutasyon; render'da clock/    │    │ ui.rs:1745 "BOTH stage surfaces". preview_capability  │
│    │ rastgelelik/FS okuması YOK                  │    │ .rs:3-5 saflık sözleşmesi açıkça yazılı               │
├────┼─────────────────────────────────────────────┼────┼──────────────────────────────────────────────────────┤
│ 12 │ §4.6 Drag preview LOKAL-TRANSAKSİYONEL:     │ ✅ │ ResizeUpdate{mark_persistence_dirty, request_pty_     │
│    │ preview'da pane resize YOK, commit bir kez  │    │ resize} interaction.rs:160-164; resize_panes_during_  │
│    │                                             │    │ shell_preview ui.rs:511; miller_resize_1000_moves_    │
│    │                                             │    │ has_bounded_side_effects input/file_manager.rs:2586   │
├────┼─────────────────────────────────────────────┼────┼──────────────────────────────────────────────────────┤
│ 13 │ §5 Input önceliği: overlay → capture →      │ ✅ │ route_shell_input + blocking_overlay_active() +       │
│    │ topmost hit → focus → page → global →       │    │ shell_mouse_input_owner(pos); SF4.2 8/8 slice GREEN   │
│    │ fail-closed. Drag = ACTIVE CAPTURE          │    │ (TASKS.md:1043-1070), closure head 20f659c1           │
├────┼─────────────────────────────────────────────┼────┼──────────────────────────────────────────────────────┤
│ 14 │ §2 Adlandırılmış bölgeler CANLI: TopBar,    │ ❓ │ RegionId'de 7 varyant TANIMLI (model.rs:18-28) AMA    │
│    │ AppDock, RightPanel, BottomBar              │    │ ÜRETİM ShellLayout::default() SADECE LeftPanel +      │
│    │                                             │    │ WorkspaceStage slot'u üretir — shell.rs:70-93         │
│    │                                             │    │ ⇒ diğer 4 bölge rect=Rect::default() (boş)            │
├────┼─────────────────────────────────────────────┼────┼──────────────────────────────────────────────────────┤
│ 15 │ §2 Tipli template'ler seçilebilir           │ ❓ │ 5 template TANIMLI. ShellTemplateId:: ÜRETİMDE tek    │
│    │                                             │    │ kullanım: snapshot.rs:75 SABİT KODLU DockSidebarStage.│
│    │                                             │    │ Render onu HİÇ OKUMAZ. Diğer 2 hit = app_dock.rs      │
│    │                                             │    │ :193,:350 (TEST). ⇒ 4 template üretimde ERİŞİLEMEZ    │
├────┼─────────────────────────────────────────────┼────┼──────────────────────────────────────────────────────┤
│ 16 │ §2 Bounded track solver (Fixed/Content-     │ ❓ │ Solver TAM: request_from_policy layout.rs:378-421.    │
│    │ Bounded/Resizable/Fill/Collapsed)           │    │ AMA legacy default `tracks` BOŞ (from_legacy_root     │
│    │                                             │    │ model.rs:146-153) ⇒ request_from_legacy_size          │
│    │                                             │    │ fallback layout.rs:355-360. TrackPolicy yolu ÜRETİMDE │
│    │                                             │    │ ÖLÜ — sadece testlerde çalışıyor                      │
├────┼─────────────────────────────────────────────┼────┼──────────────────────────────────────────────────────┤
│ 17 │ §2 SF5 AppDock: görünür app switcher,       │ ❓ │ Model+geometri+render+popover+resize TAM (384 satır). │
│    │ Files'a sidebar'sız erişim; dock resize/    │    │ AMA rect boş ⇒ HİÇ ÇİZİLMEZ. Kaynak yorumu bunu       │
│    │ collapse SF3 reducer'ını kullanır           │    │ AÇIKÇA beyan ediyor: "the legacy default template     │
│    │                                             │    │ projects none, so this stays a no-op until a dock-    │
│    │                                             │    │ bearing template is live" ui.rs:829-833 + :459-463    │
├────┼─────────────────────────────────────────────┼────┼──────────────────────────────────────────────────────┤
│ 18 │ Özelleştirilmiş ağaç DİSKE yazılır ve GERİ  │ ❓ │ ShellSnapshotV1 root+region_constraints+component_    │
│    │ YÜKLENİR (S6 vaadi)                         │    │ placements+pinned_dock_order YAZIYOR + validate       │
│    │                                             │    │ EDİYOR (snapshot.rs:28-42, :118-127).                 │
│    │                                             │    │ AMA restore SADECE restored_left_panel_preference()   │
│    │                                             │    │ okuyor: app/mod.rs:438, :954. AĞAÇ GERİ DÖNMÜYOR      │
├────┼─────────────────────────────────────────────┼────┼──────────────────────────────────────────────────────┤
│ 19 │ Kolon genişlikleri kullanıcı düzeni olarak  │ ❌ │ MillerPathSegment.preferred_width (miller.rs:18)      │
│    │ korunur                                     │    │ sadece RAM'de. src/persist/ içinde column_width /     │
│    │                                             │    │ preferred_width grep = 0 SONUÇ ⇒ oturum arası kaybolur│
├────┼─────────────────────────────────────────────┼────┼──────────────────────────────────────────────────────┤
│ 20 │ Kullanıcı tanımlı layout / layout DSL       │ ❌ │ AÇIKÇA REDDEDİLMİŞ v0 kararı: "Foundation v0 exposes  │
│    │                                             │    │ no arbitrary layout DSL" template.rs:10.              │
│    │                                             │    │ (ama Deserialize altyapısı HAZIR — bkz F-a)           │
├────┼─────────────────────────────────────────────┼────┼──────────────────────────────────────────────────────┤
│ 21 │ §6 ComponentRegistry = ANTI-PATTERN         │ ❌ │ S5 hâlâ AÇIK + NO-GO: "only when a second real        │
│    │ ("ikinci tüketici yoksa over-engineering")  │    │ component/page proves the abstraction" TASKS.md:1954  │
│    │                                             │    │ ⇒ mockup artık 3 tüketici sağlıyor: TETİKLENDİ ama    │
│    │                                             │    │ karar VERİLMEDİ                                       │
├────┼─────────────────────────────────────────────┼────┼──────────────────────────────────────────────────────┤
│ 22 │ Responsive degradation merdiveni            │ ❓ │ Kod TAM: Wide/Standard/Compact/TooSmall layout.rs:    │
│    │ (Wide→Standard→Compact→TooSmall)            │    │ 443-503. AMA legacy layout'ta RightPanel & AppDock    │
│    │                                             │    │ YOK ⇒ collapse_region() false döner ⇒ sadece          │
│    │                                             │    │ LeftPanel-compact basamağı erişilebilir              │
├────┼─────────────────────────────────────────────┼────┼──────────────────────────────────────────────────────┤
│ 23 │ §4 SSH bütçeleri test edilir ("hoped değil")│ ❓ │ render_prof sayaçları var (shell.geometry_cache.hit/  │
│    │                                             │    │ miss view.rs:115,118; miller_resize_profile_* testler)│
│    │                                             │    │ AMA p95/outgoing-byte kapanışı "P7 isolated runtime"a │
│    │                                             │    │ devredilmiş — bu oturumda TAZE ÖLÇÜM YOK              │
├────┼─────────────────────────────────────────────┼────┼──────────────────────────────────────────────────────┤
│ 24 │ Çoklu client/monitor'da ayrışabilen layout  │ ❌ │ TEK AppState; frame TÜM client'lara broadcast; en     │
│    │ (rehberde örtük "thin clients")             │    │ küçük client boyutu kazanır                           │
│    │                                             │    │ research/multi-monitor-shared-view.md §1-3            │
├────┼─────────────────────────────────────────────┼────┼──────────────────────────────────────────────────────┤
│ 25 │ Mobile/dar ekranda shell bölgeleri          │ ❌ │ compute_empty_shell_view ⇒ bölge haritası KASITEN boş │
│    │                                             │    │ ui.rs:600-608 "named shell regions are a desktop      │
│    │                                             │    │ concept for now". mobile.rs kendi split'i (47 KB)     │
├────┼─────────────────────────────────────────────┼────┼──────────────────────────────────────────────────────┤
│ 26 │ §7 Durum tablosu: "SF5.1 NEXT, FM1/FM2      │ ❌ │ TASKS.md'de SF5→FM5'in TAMAMI [x] (satır 1114-1200).  │
│    │ pending"                                    │    │ ⇒ REHBER 6 FAZ GERİDE, güncellenmemiş                 │
└────┴─────────────────────────────────────────────┴────┴──────────────────────────────────────────────────────┘
  Açıklama: ✅ vaat karşılandı (kaynaktan doğrulandı)   ❓ kısmi — kod VAR, üretime BAĞLI DEĞİL
            ❌ yok / açıkça reddedilmiş / rehber bayat
  SKOR: 13 ✅  ·  7 ❓  ·  6 ❌
  DESEN: Miller/Stage/input/render ekseni TAM YEŞİL. Shell-template/dock/persist ekseni TAMAMEN ❓.
         ⇒ "Yazıldı, test edildi, ama şalteri açılmadı."
```

---

## D. SF Faz Zinciri — Ne Teslim Etti, Ne Kilitledi

| Faz | Teslim | Kapanış SHA | Kilitlediği şey |
|---|---|---|---|
| SF0 | Tasarım dondu, 7+5 faz onaylandı | `32856f7` | Kapsam sınırı (registry/DSL yok) |
| SF1 | Karakterizasyon — mevcut curtain test'e çivilendi | `7b9b626d` | Regresyon zemini (11/11) |
| **SF2** | **Bölge modeli + tipli template + solver + cache'li `ShellView`** | `07133b8b` | **Custom layout'un ALFABESİ**: bölge/track/generation grameri |
| **SF3** | **Paylaşılan `ResizeTransaction` + collapse + scroll + snapshot v4** | `90be6893` | **TEK RESIZE OTORİTESİ** — FM2 bu yüzden ikinci drag state yaratmadı |
| **SF4** | Tipli Stage + input router + saf render + surface projeksiyon | `f973740e` | **INPUT ÖNCELİK SIRASI** — yeni bölge eklemek bu zinciri değiştirmez |
| SF5 | AppDock model/geometri/render + popover; dock 3..=9 pinlendi | `64d5dd5e`/`cb0c77fd` (5.1), `406db487`/`d031ef26` (5.2) | Dock'un `ResizeTransaction`'a bağlanması |
| SF6 | Files curtain → tipli `NativeFiles` Stage | — | Stage'in gerçek tüketicisi |
| FM1 | Yatay Miller viewport + bounded projeksiyon | `35cfbc00` | Kaydırılabilir alan |
| FM2 | Kolon resize; `MillerTrioDrag` **KALDIRILDI** | — | Tek otorite ilkesinin sahada kanıtı |
| FM3 | Tüm kolonlarda mouse + stale revalidation | — | Generation sözleşmesi |
| FM4 | Path-stable büyüyen navigasyon | — | Finder davranışı |
| FM5 | Preview/Inspector yerleşimi ölçüldü → **inline korundu (NO-GO)** | — | RightPanel tüketicisi ERTELENDİ |

**Ara SF ayrıntıları (kanıt zinciri):**
- SF3.1 divider reducer RED/GREEN çiftleri: `368c4d3a`/`d89a7f94`, `b6570ee4`/`807cb76c`;
  sidebar adaptasyonu REDs `96a1660e`, `09944834` → GREEN `61b915a9`; klavye REDs `4888c3f8`,
  `4026c28b`, `960b6d5f` → GREEN `336fa3de`.
- SF3.2 collapse/scroll ürün zinciri `45a2e87e`'de bitiyor.
- SF3.3 persistence 11-commit RED/GREEN zinciri `90be6893`'te kapanıyor.
- SF4.1 8 davranış dilimi: `557bcc77`/`6a18f0c7`, `f22bdac4`/`b9180de3`, `96e6cddb`/`d20403d0`,
  `27ad2a79`/`e8ef80ac`, `207c9da3`/`f31ab28a`, `a5e5bace`/`e1c82036`, `056f0879`/`f0f32075`,
  `784fdc2e`/`944a9d4c`.
- SF4.2 8 dilim closure head `20f659c1`; SF4.4 `7796d855`/`acc82ffd`, `bb5a6899`/`1bc69cf5`,
  `a9b67112`/`f973740e`, saflık `08d73676`, retained-path `1f57ccbb`.

**Sonraki kilit — Files Layout V1 (2026-07-19):**
`docs/superpowers/specs/2026-07-19-herdr-files-layout-v1-lock.md`, freeze checkpoint
`d98c31c70946e496cb6536f02fc96e45974df2de`. Kompozisyon
`global agent/workspace panel | Files locations rail | Miller Trail | detail`; 8 ürün yasası
(V1-L1..L8); VIS-07..25 Playwright baseline'ları. **Versiyonlama kuralı:** dört-yüzey
sahipliğini/sırasını değiştirmek `Files Layout V2` kararı gerektirir —
*"No agent may silently reinterpret a V2-class change as a V1 bug fix."*

---

## E. Açık İş Envanteri

`.codex/TASKS.md`: **14 açık madde.** Layout ile ilgili olanlar:

| ID | Başlık | Kaynak | Önkoşul / Blocker |
|---|---|---|---|
| **S5** | ComponentRegistry — "ancak ikinci gerçek component/page kanıtlarsa" | `TASKS.md:1954` | **B-zincirinin asıl kapısı.** Mockup'ın TopBar/RightPanel/BottomBar'ı = 3 tüketici ⇒ tetikleyici karşılandı, karar verilmedi |
| **S7** | Popup stack (ownership, focus, close ordering, nested) | `TASKS.md:1959` | Trigger-gated, bağımsız |
| **FIP-6.3** | İzole terminal mouse + PTY-byte smoke | `TASKS.md:855` | A-zinciri; **B bundan önce BİTEMEZ** (PRD §2) |
| **FIP-1.6** | Playwright `TP-FIP-VIS-01` + izole gerçek-mouse | `TASKS.md:772` | A-zinciri |
| FIP-6.7 / 6.8 | Continuity + temiz worktree kapanışı | `:863`, `:865` | A-zinciri kuyruğu |

Kalan 8 madde (DCLICK-6 `:35`, FFO-8/9 `:92`,`:100`, FMN-6 `:187`, FMR-0/4/5 `:441`,`:512`,`:523`,
change-pipeline `:1216`) file-manager/pipeline şeridi — layout altyapısı değil.

---

## F. Genişleme Reçetesi — İki Senaryo

### Ortak Adım 0 — ŞALTER (her iki senaryo da buna bağlı)

| Ne | Nerede | Nasıl |
|---|---|---|
| `ShellLayout::default()` → aktif template | `src/ui.rs:303` | `ShellPresentationState`'e `active_template: ShellTemplateId` ekle → `template.build()` |
| `LEGACY_DESKTOP_SHELL_LAYOUT_REVISION` sabit `1` | `src/ui.rs:135`, `:306` | **ZORUNLU:** template değişince revision artmalı; yoksa `ShellGeometryKey` eşit kalır → cache YANLIŞ hit verir (R2) |
| `resolve_dynamic` sadece `LeftPanel` biliyor | `src/ui.rs:313-318` | AppDock/RightPanel/TopBar/BottomBar genişlik çözümü |

**Test kancası:** `desktop_workspace_template_solves_normal_compact_and_too_small` (`shell.rs:996`)
zaten var, sadece üretimden çağrılmıyor. RED: `production_render_projects_dock_region_for_dock_bearing_template`.
**Risk:** AppDock anında görünür olur → VIS-18..25 baseline'ları kırılır → **Files Layout V1'e göre
V2-sınıfı karar gerekir.**

### Senaryo (a) — Kullanıcı kendi layout template'ini tanımlasın

| Sıra | Dosya | Tip / fonksiyon | İş |
|---|---|---|---|
| a1 | `src/ui/shell/template.rs:10` | `ShellTemplateId` | v0 "no DSL" kararını kaldır; `Custom(ValidatedShellLayout)` varyantı veya ayrı `LayoutSource` enum'u |
| a2 | `src/config/model.rs` | yeni `layout: Option<ShellLayout>` | **Bugün config'de layout alanı YOK** (grep=0). `src/config/io.rs` ile yükle |
| a3 | `src/ui.rs:303` | Adım 0 | config → `active_template` |
| a4 | `src/persist/snapshot.rs:75` | `from_left_panel_preference` | Sabit `DockSidebarStage` yerine aktif template'i yaz (R3'ü kapatır) |
| a5 | `src/app/mod.rs:438, :954` | restore | `root` + `region_constraints` + `component_placements`'ı GERİ UYGULA |
| a6 | `src/fm/miller.rs:18` | `MillerPathSegment.preferred_width` | Bounded map olarak snapshot'a ekle (≤32 giriş) |

**Test kancaları:** `valid_v4_shell_json` (`snapshot.rs:727`) genişletilir. REDs:
`invalid_user_layout_falls_back_to_builtin_template`, `restored_shell_tree_reproduces_persisted_regions`,
`user_layout_without_workspace_stage_is_rejected` (`MissingWorkspaceStage` zaten var, `model.rs:261`).
**Risk:** DÜŞÜK-ORTA. Şema v4/v1 yerinde, migration gerekmez. Asıl risk kullanıcı layout'unun
`TooSmall`'a düşmesi — degradation ladder zaten fail-safe.

### Senaryo (b) — Yeni yüzey türü (belge/tablo görüntüleyici: PNG/XLSX/PDF)

#### Yol B-1 — DETAY YÜZEYİ olarak (ucuz, Files içinde kalır) ✅ ÖNERİLEN İLK ADIM

| Sıra | Dosya | Tip | İş |
|---|---|---|---|
| b1.1 | `src/fm/trail_snapshots.rs:36-45` | `TrailDetailPreview` | Yeni varyant: `Document(DocumentPreview)` / `Table(TablePreview)` |
| b1.2 | `src/fm/preview_capability.rs:44-58` | `PreviewCapability` | `NativeDocument` / `NativeTable` varyantı (bugün `OptionalPlugin`'e düşüyor) |
| b1.3 | `src/fm/preview_capability.rs:66-72` | `PreviewProviderSet` | `documents` yuvası zaten var — doldur |
| b1.4 | `src/app/file_preview_worker.rs:57,:92` | `FilePreviewSync`, `FilePreviewSource` | Bounded worker'a belge kaynağı ekle (**FS okuması render'da DEĞİL** — V1-L6) |
| b1.5 | `src/ui/file_manager/trail_view.rs` | detay render | Tablo için kolon/satır çizimi |
| b1.6 | `src/persist/plugin_registry.rs` + `api::schema::InstalledPluginInfo` | — | Harici viewer ise **SUNUCU tarafı** kayıt |

**Test kancaları:** `preview_capability.rs:198+` matrisi (RED:
`xlsx_selects_native_table_capability_when_provider_present`);
`.codex/evidence/files-preview-capability-test-points.md` mevcut.
**Risk:** DÜŞÜK. Layout V1-L5 *"file activation updates the detail state without inventing another
directory column"* zaten öngörüyor → **V1.x işi, V2 değil.**

#### Yol B-2 — TAM STAGE UYGULAMASI olarak (Terminal/Files gibi üçüncü app)

| Sıra | Dosya:satır | Tip | Dikkat |
|---|---|---|---|
| b2.1 | `surface_host.rs:4-7` | `BuiltInAppId{Terminal, Files}` | Yeni varyant: `Documents` |
| b2.2 | `surface_host.rs:10-15` | `BuiltInAppId::index()` | `0/1` sabit kodlu → `2` ekle |
| b2.3 | `surface_host.rs:82` | **`last_generations: [Option<u32>; 2]`** | ⚠️ **DİZİ BOYUTU 2** — `3`'e çıkarmak ZORUNLU, yoksa index panic |
| b2.4 | `surface_host.rs:231` | `Default` impl | `[Some(0), None]` → `[Some(0), None, None]` |
| b2.5 | `surface_host.rs:37-46` | `AppSurfaceRef`, `StageSurfaceView` | İkişer varyant ekle |
| b2.6 | `surface_host.rs:61-67` | `AppInstance::built_in` | match kolu |
| b2.7 | `surface_host.rs:117` | `activate_files` | Genelleştir → `activate(app: BuiltInAppId)` (bugün Files'a özel) |
| b2.8 | `src/ui.rs:843-851` | render match | Üçüncü kol + renderer |
| b2.9 | `src/ui/app_dock.rs:36-45` | `AppDockEntry::icon` | Nerd + ASCII ikon çifti |
| b2.10 | `src/persist/snapshot.rs:44-48` | `PinnedBuiltinAppV1{Terminal, Files}` | Yeni varyant + **şema uyumluluğu** (eski snapshot'lar bilmez) |
| b2.11 | `src/ui/shell/model.rs:42-49` | `ShellComponentId` | Bölgeye yerleştirilecekse |

**`LaunchPolicy` notu:** bugün tek varyant `Singleton` (`surface_host.rs:26-28`). Aynı anda iki belge
açılacaksa `Multi{max}` gerekir — `StageState` zaten 16 instance taşıyor: **kapasite var, politika yok.**
**Test kancaları:** `surface_host.rs:253-647`'deki 8 testin her biri üçüncü app için yansıtılmalı;
özellikle `stage_rejects_more_than_sixteen_builtin_instances` (`:292`) ve
`instance_generation_exhaustion_fails_without_aliasing` (`:316`).
**Risk:** ORTA-YÜKSEK — Adım 0 olmadan dock ikonu görünmez; dört-yüzey sahipliği değişirse V2 kararı gerekir.

**Tavsiye:** Önce **B-1** (V1.x, düşük risk, PDF/XLSX zaten sınıflandırılmış). B-2'yi ancak kullanıcı
"belgeyi tam ekran, sekmeli, Files'tan bağımsız istiyorum" derse aç.

---

## G. Riskler ve Kırılganlıklar

| # | Risk | Kanıt | Şiddet |
|---|---|---|---|
| **R1** | **Layout V1 kilidi B-zincirini V2 sınıfına sokuyor.** AppDock'u görünür yapmak / bölge eklemek "four-surface ownership veya order" değişikliğidir → açık V2 kararı + VIS-07..25 baseline yenilemesi | `files-layout-v1-lock.md` "Versioning Rule" | **YÜKSEK** |
| **R2** | **Geometry cache stale-hit.** `ShellGeometryKey` = (area, layout_rev, constraints_rev, collapse_rev) — **template kimliği İÇERMİYOR**. Template değişip revision sabit kalırsa cache eski geometriyi döndürür, hit-test yanlış bölgeye gider | `view.rs:16-21` ⟷ `ui.rs:306` sabit `1` | **YÜKSEK** |
| **R3** | **Kalıcılık yalanı.** Snapshot `template: DockSidebarStage` (dock'lu) yazıyor, runtime `ShellLayout::default()` (dock'suz) çalıştırıyor | `snapshot.rs:75` ⟷ `ui.rs:303` | ORTA |
| **R4** | **Rehber bayat, tek gerçeklik kaynağı değil.** §7 tablosu 6 faz geride; §3 resident-column bound emekli | rehber `:136-144` ⟷ `TASKS.md:1114-1200`; `NEXT-SESSION-PROMPT.md:118` | ORTA |
| **R5** | **Kapalı enum genişleme maliyeti.** `last_generations: [Option<u32>; 2]` sabit boyutlu — yeni app eklerken büyütülmezse index panic; `BuiltInAppId::index()` sabit kodlu | `surface_host.rs:82`, `:10-15` | ORTA (b2 için) |
| **R6** | **Mobile ayrı dünya.** Shell bölgeleri kasten boş; custom layout mobile'da HİÇ çalışmaz — iki ayrı kompozisyon otoritesi bakım borcu | `ui.rs:600-608`, `mobile.rs` 47 KB | ORTA |
| **R7** | **Çoklu client layout ayrışamaz.** Tek `AppState` + broadcast + en küçük client kazanır. Per-client layout sunucuya taşınırsa CLAUDE.md guardrail'i ("sidebar/row/card = TUI presentation") ihlal olur | `research/multi-monitor-shared-view.md` §1-3 | ORTA |
| **R8** | **Server/client sınırı cazibesi.** Shell tipleri şu an açıkça client-local; belge yüzeyi eklerken plugin kaydı SUNUCU tarafta (`api::schema`) — sınırı karıştırmak kolay | `src/ui/shell.rs:12-16` ⟷ `persist/plugin_registry.rs:5` | ORTA |
| **R9** | **Ölü kod çürümesi.** `interaction` modülü `#[allow(dead_code)]`; 4 template + TrackPolicy yolu + AppDock render üretimde erişilmiyor | `shell.rs:23-24` | DÜŞÜK-ORTA |
| **R10** | Miller kolon genişlikleri oturum arası kayboluyor | `persist/` grep = 0 | DÜŞÜK |

---

## H. B1 İÇİN HAZIR GİRDİ (B-chain başlatıldığında kullanılacak)

> Bu bölüm **B1'in GİRDİSİDİR, çıktısı değil.** `.cartography/custom-layout-SYSTEM-MAP.json`
> BİLİNÇLİ OLARAK üretilmemiştir — o artefakt B-chain'in kendi onaylı tasarım kapısına aittir
> (`NEXT-SESSION-PROMPT.md:128`). Aşağıdaki liste, o kapı açıldığında cartographer'ın sıfırdan
> başlamamasını sağlar.

### H.1 — Cartography'nin cevaplaması gereken sorular

| # | Soru | Neden kritik | Nerede bakılır |
|---|---|---|---|
| Q1 | Mockup'taki her bölge hangi mevcut seam'e bağlanır, hangisi no-goal? | PRD kabul kriteri: "her bölge ya çalışan seam'e bağlı ya açıkça no-goal" | `.local/prd/custom-layout-target-mockup.md` eşleme tablosu ⟷ `RegionId` (model.rs:18-28) |
| Q2 | `ShellGeometryKey` template kimliğini nasıl taşıyacak? | R2 (cache stale-hit) — B4'te patlar | `view.rs:16-21`, `ui.rs:306` |
| Q3 | TopBar/BottomBar `TrackPolicy::Fixed{cells:0}` olarak duruyor — gerçek yükseklik nereden gelir? | `desktop_workspace()` bunları 0 hücre veriyor (template.rs:91,96) → bölge var ama görünmez | `src/ui/shell/template.rs:77-99` |
| Q4 | RightRail (mockup) = AppDock'un sağa yansıtılmışı mı, yeni bölge mi? | `RegionId`'de RightRail YOK; AppDock tek dock track'i | mockup satırı "AppDock pattern (SF5) rotated to right edge" |
| Q5 | CenterStage tab-strip'i stage-local chrome mu, shell bölgesi mi? | Bugün tab bar `terminal_surface_active` iken carve ediliyor (ui.rs:329-338) — Files'ta yok | `ui.rs:326-340` |
| Q6 | LeftPanel dikey bölünme (mockup'ta 2 panel) solver'da destekleniyor mu? | `MAX_NESTED_SPLIT_DEPTH=4` var, ama LeftPanel bugün tek slot | `model.rs:9`, `layout.rs:164-209` |
| Q7 | S5 ComponentRegistry bu program için ZORUNLU mu, yoksa placement→renderer match yeter mi? | Rehber §6 registry'yi anti-pattern sayıyor; 3 tüketici tetikleyici olabilir | `TASKS.md:1954`, `docs/patterns/tui-composition.md` P3 |
| Q8 | V1 kilidi hangi bölge eklemelerini V2'ye zorlar, hangileri V1.x kalır? | R1 — yanlış sınıflama tüm baseline'ları gereksiz kırar | `files-layout-v1-lock.md` "Versioning Rule" |
| Q9 | Mobile bu programın kapsamında mı (no-goal mı)? | R6 — kapsam dışıysa açıkça yazılmalı | `ui.rs:600-608` |
| Q10 | Persist round-trip'in kapatılması B-chain'in parçası mı, ayrı iş mi? | S6 — bugün yarım; custom layout onsuz kalıcı olmaz | `snapshot.rs:28-42` ⟷ `app/mod.rs:438` |

### H.2 — Haritalanacak seam'ler (node adayları)

```
GEOMETRİ ZİNCİRİ    ShellLayout → validate → solve → RegionRects → compute_shell_view → ShellView
                    → region_hit_at → (input router)
RENDER ZİNCİRİ      compute_view (ui.rs:303) → shell_view.regions.get(R) → render_X(area)
                    → Compositor (ui.rs:797)
ETKİLEŞİM ZİNCİRİ   mouse/key → route_shell_input → blocking_overlay_active → capture
                    → topmost hit → ResizeTransaction{Shell|Miller} → ResizeUpdate
KALICILIK ZİNCİRİ   ShellPresentationState → ShellSnapshotV1 → session.json
                    → restore → restored_left_panel_preference  ⚠️ KIRIK HALKA
STAGE ZİNCİRİ       StageState → surface_view() → render match → BuiltInAppId → AppDock entry
```

### H.3 — B1'de okunacak kaynak dosyalar (öncelik sırası)

1. `src/ui/shell/model.rs` (351 satır) — bölge/track/validate grameri
2. `src/ui/shell/template.rs` (155) — 5 template + track sabitleri
3. `src/ui/shell/layout.rs` (593) — solver + degradation
4. `src/ui/shell/view.rs` (206) — geometri cache + hit
5. `src/ui/shell/interaction.rs` (48 KB) — resize/collapse/scroll reducer
6. `src/ui.rs:270-470` + `:780-860` — üretim compute + render
7. `src/persist/snapshot.rs:1-160` + `:440-460` — ShellSnapshotV1
8. `src/ui/app_dock.rs` (384) — dark bileşen
9. `src/ui/surface_host.rs` (649) — Stage kapalı enum'ları
10. `src/ui/mobile.rs` — ayrı dünya (kapsam kararı için)

### H.4 — B1 çıktısının uyması gereken sözleşme

- Şema: `.cartography/*-SYSTEM-MAP.json` kardeşleriyle aynı (örnek:
  `.cartography/tui-composition-SYSTEM-MAP.json`, `files-content-locations-rail-SYSTEM-MAP.json`).
- Ad: `custom-layout-SYSTEM-MAP.json` (generic `SYSTEM-MAP.json` ÇAKIŞMA — kullanma).
- Her node: `claim + evidence(dosya:satır/test/commit) + confidence`; θ-kuralı
  (1 official/executable ≥0.9 VEYA 2 bağımsız ≥0.7).
- Variant **V** = (verification kriterisiz bileşen) + (conf<θ iddia) + (çözülmemiş bağımlılık);
  V azalmıyorsa DUR.
- Descent adayları: bu analizde ❓ işaretli 7 madde + R2 (cache anahtarı) — kök neden alt sistemde.

---

## I. BU TURDA İNCELENMEYEN LAYOUT ALTERNATİFLERİ (gelecek araştırma havuzu)

> ⚠️ **Dürüstlük notu:** Aşağıdakiler bu turda **İNCELENMEDİ**. Hiçbiri hakkında doğrulanmış iddia
> yoktur — sadece "neden ilginç / hangi soruyu cevaplar / nereden başlanmalı" kaydıdır. Confidence
> ataması YAPILMAMIŞTIR; araştırıldıklarında `docs/references/custom-layout.md`'ye tier+conf ile eklenecektir.

| Aday | Neden ilginç | Hangi soruyu cevaplayabilir | Nereden başlanmalı | Durum |
|---|---|---|---|---|
| **zellij KDL layout sistemi** | Aynı kategori (multiplexer); deklaratif dosyadan layout ağacı kurma — herdr'ın S4/F-a hedefinin birebir muadili | Kullanıcı-tanımlı layout dosyası nasıl şemalanır? `pane_template`/`tab_template` kompozisyonu nasıl bounded tutulur? | ZATEN INDEXED: `[zellij-layout]`, `[zellij-kdl-docs]` — `docs/references/tui-composition.md:37-38` (conf 0.95/0.9). Derinleştirilecek: KDL→ağaç validation, hata mesajları | **kısmen indexed, derinleştirilmedi** |
| **tmux / wezterm pane tanımı** | tmux `select-layout` preset'leri + wezterm Lua config — "preset template" vs "serbest ağaç" ikilisinin olgun örnekleri | Preset sayısı ne kadar tutulmalı? Kullanıcı serbest ağaç mı ister, isimli preset mi? | tmux man `select-layout` (even-horizontal/main-vertical/tiled); wezterm `wezterm.mux` docs | **incelenmedi** |
| **i3 / sway tiling config** | Olgun kullanıcı-tanımlı tiling; `for_window` kuralları + workspace layout | Bölge/pencere eşleştirme kuralları deklaratif nasıl yazılır? Kullanıcı DSL'i ne kadar ifade gücü ister? | i3 user guide "Configuring i3" + sway-config(5) | **incelenmedi** |
| **Cassowary constraint solver (GNOME/Cocoa AutoLayout)** | herdr'ın `TrackPolicy` solver'ı elle yazılmış greedy bir çözücü; Cassowary genel-amaçlı lineer kısıt çözücüsüdür | Greedy `allocate_lengths` yerine kısıt-tabanlı çözüm gerekir mi? Ne zaman aşırı mühendislik olur? | Badros & Borning "Cassowary" makalesi; Rust `cassowary-rs` crate (ratatui'nin ESKİ Layout motoru — ratatui 0.26'da kaldırıldı, bu tarihsel bağlam ÖNEMLİ) | **incelenmedi** |
| **CSS Grid / Flexbox track modeli** | herdr'ın `TrackPolicy{Fixed, ContentBounded, Resizable, Fill{weight}, Collapsed}` neredeyse birebir CSS Grid `minmax()`/`fr` semantiği | Track terminolojisi ve kenar-durumları (taşma, min-content, fr dağıtımı) olgun bir spesifikasyondan devşirilebilir mi? | CSS Grid Level 1 spec §7 (track sizing algorithm); Flexbox §9 (resolving flexible lengths) | **incelenmedi** |
| **Zed panel sistemi** | Modern Rust GUI; dock/panel/sol-sağ-alt yerleşimi + kalıcılık — herdr'ın AppDock+RightPanel hedefiyle örtüşür | Dock'lar nasıl kalıcılaştırılır? Panel state workspace başına mı global mi? | `zed-industries/zed` → `crates/workspace/src/dock.rs`, `pane_group.rs` | **incelenmedi** |
| **VSCode workbench layout** | Endüstri standardı: activity bar + side bar + panel + editor grid + kalıcı layout state | Kullanıcı layout'u sürükle-bırak ile değiştirdiğinde şema nasıl versiyonlanır (bizim R3/S6 sorunumuz)? | VSCode `src/vs/workbench/browser/layout.ts`; "Custom Layout" docs | **incelenmedi** |
| **ratatui-hypertile** | Layout/tiling yardımcı crate'i olarak adı geçiyor | Ratatui ekosisteminde bounded tiling için hazır bir çözüm var mı? | ⚠️ **DÜZELTME:** refpool'da **YOK** (`~/.cartography/refpool/` içeriği: yazi-src, superfile-src, joshuto, ratatui-image, yeet, rat-commander). Önce varlığı crates.io'da doğrulanmalı | **indexed DEĞİL — önce varlık doğrulaması gerek** |
| **tui-studio** | TUI layout tasarım aracı olarak adı geçiyor | Görsel layout editörü herdr için anlamlı mı (yoksa P7-benzeri gelecek faz mı)? | ⚠️ **DÜZELTME:** refpool'da **YOK**. Önce varlık/kimlik doğrulaması gerek | **indexed DEĞİL — önce varlık doğrulaması gerek** |
| **helix Compositor / cursive StackView** | Overlay/katman yönetimi — S7 popup stack kararı için | Popup stack ayrı bir katman mı yoksa shell bölgesi mi? | ZATEN INDEXED: `[helix-compositor]` conf 0.95, `[cursive-stackview]` — `docs/references/tui-composition.md:50`, P1/P4 | **indexed, custom-layout açısından değerlendirilmedi** |
| **k9s PageStack + registry** | Persistent chrome + tek swappable body + registry — F-a/S5 için birincil desen | Yeni bölge/sayfa eklemek 1 registry satırına inebilir mi? | ZATEN INDEXED: `[k9s-app-layout]`, `[k9s-registrar]` — P3 conf 0.95 | **indexed, custom-layout açısından değerlendirilmedi** |

**Araştırma sırası önerisi (maliyet/fayda):**
1. zellij KDL derinleştirme (aynı kategori, zaten indexed) → S4 doğrudan cevap
2. CSS Grid track sizing (spec okuması, klon gerektirmez) → `TrackPolicy` kenar-durumları
3. Zed dock persistence → R3/S6 (persist round-trip) çözümü
4. VSCode workbench → layout şema versiyonlama
5. i3/sway + tmux/wezterm → kullanıcı DSL ifade gücü kalibrasyonu
6. Cassowary → SADECE greedy solver yetersiz kalırsa (bugün yetiyor — over-engineering riski)
7. hypertile/tui-studio → önce varlık doğrulaması

---

## J. REDDEDİLEN / ERTELENEN KARARLAR ve YENİDEN-AÇILMA KOŞULLARI

| Karar | Durum | Gerekçe | Kaynak | **Hangi kanıt gelirse yeniden değerlendirilir** |
|---|---|---|---|---|
| **S5 ComponentRegistry** | NO-GO (açık madde) | "Speculative registry — ikinci gerçek tüketici yok" | `TASKS.md:1954`; rehber §6 anti-pattern tablosu | **İkinci VE üçüncü gerçek component/page üretime bağlandığında.** Mockup'ın TopBar+RightPanel+BottomBar'ı bu tetikleyiciyi NOMİNAL olarak karşılıyor; ama "gerçek tüketici" = ÜRETİMDE ÇİZİLEN bölge demek — bugün hiçbiri çizilmiyor (Adım 0 yapılmadan tetikleyici SAYILMAZ) |
| **S7 Popup stack** | Trigger-gated (açık) | Nested popup / close-ordering baskısı henüz kanıtlanmadı | `TASKS.md:1959` | İç içe 2+ popup'ın close-ordering hatası ÜRETİMDE gözlenirse, veya bir bölgeye anchored non-modal popup gerekirse (`docs/patterns/tui-composition.md` P4/P5) |
| **RightPanel / Inspector yerleşimi** | **NO-GO (FM5 kapandı)** | Ölçüm sonucu inline preview korundu | `TASKS.md` FM5 bloğu (`:1195+`) | Belge/tablo yüzeyi (F-b) inline detay kolonuna SIĞMAZSA — örn. XLSX tablo genişliği `MILLER_DETAIL_MIN_WIDTH=20`'yi anlamlı biçimde aşarsa. Bu, ölçülebilir bir eşiktir |
| **Arbitrary layout DSL** | Reddedildi (v0 kararı) | "Foundation v0 exposes no arbitrary layout DSL" | `src/ui/shell/template.rs:10` | Kullanıcı 5 built-in template ile karşılanamayan somut bir düzen talep ettiğinde. **Not:** `Deserialize` altyapısı zaten hazır (`model.rs:287-292`) — karar teknik değil ürün kararıdır |
| **N2 unbounded Miller state machine** | NO-GO | "Bounded projection zaten referansları karşılıyor; arbitrary chain kanıtlanmış değer katmıyor" | `TASKS.md` N2 bloğu | Kullanıcı 32'den derin zincir veya 5'ten fazla eşzamanlı görünür kolon talep ederse |
| **Multi-client ayrık layout** | Kapsamda değil | Tek `AppState` + broadcast mimarisi; per-client view sunucu değişikliği gerektirir | `research/multi-monitor-shared-view.md` | Kullanıcı iki monitörde FARKLI sekme/layout görmek isterse. **Dikkat:** bu, CLAUDE.md runtime/client guardrail'ini yeniden yorumlamayı gerektirir (layout "shared runtime fact" olur mu?) |
| **Mobile shell bölgeleri** | Kasten dışarıda | "named shell regions are a desktop concept for now" | `ui.rs:606-608` | Dar-ekran kullanıcısı dock/panel talep ederse, VEYA iki ayrı kompozisyon otoritesinin bakım maliyeti ölçülebilir hâle gelirse (R6) |
| **P7 Lua/script-taşınan layout ağacı** | ŞİMDİ DEĞİL | "Rust-tanımlı sayfalar yeterli; VM-embedding büyük yatırım" | `docs/patterns/tui-composition.md` P7 | Gerçek kullanıcı-plugin ekosistemi hedefi ürün kararı hâline gelirse |
| **Files Layout V2** | Açılmadı | V1 kilitli ve donduruldu | `files-layout-v1-lock.md` | Adım 0 (şalter) yapılmak istendiğinde ZORUNLU olarak açılır — AppDock'un görünür olması dört-yüzey kompozisyonunu değiştirir |

---

## Kanıt Sözleşmesi

| İddia | Kanıt tipi ve sayısı | Confidence |
|---|---|---|
| Üretim `ShellLayout::default()` kullanıyor | source ×2 bağımsız (`ui.rs:303` + `shell.rs:70-93` Default impl) | **0.98** |
| AppDock üretimde çizilmiyor | source ×3 (`ui.rs:832-833` guard + `:459-463` + kaynak yorumunun açık beyanı) | **0.98** |
| Drag-resize + yatay scroll ÇALIŞIYOR | source + **19 isimli test** (13 drag + 6 scroll) | **0.96** |
| Template'ler üretimde seçilemez | executable grep (`ShellTemplateId::` → 3 hit: 1 persist sabiti + 2 test) | **0.95** |
| Tek resize otoritesi (Shell+Miller) | source (`interaction.rs:101-113`) + adapter (`file_manager_miller.rs`) | **0.95** |
| PDF/XLSX zaten sınıflandırılmış | source (`preview_capability.rs:126-136` tam liste okundu) | **0.95** |
| **B-zinciri 0/4, onaylı tasarımı yok** | artefakt yokluğu ×2 (`ls` specs/ + `.cartography/`) + continuity beyanı ×3 | **0.94** |
| Persist yazıyor ama geri uygulamıyor | source ×2 (`snapshot.rs:28-42` ⟷ `app/mod.rs:438,954` grep) | 0.93 |
| `/docs/*` gitignored | executable (`git check-ignore -v` → `.gitignore:10`) | **0.99** |
| PNG piksel teslimatı tamamlanmamış | source, kod yorumu açık: "completes at integration" (`trail_snapshots.rs:40-42`) | 0.90 |
| Rehber §3 resident-bound bayat | grep=0 + `NEXT-SESSION-PROMPT.md:118-121` "retired" | 0.85 |
| hypertile/tui-studio refpool'da YOK | executable (`ls ~/.cartography/refpool/` + grep) | 0.90 |
| Test/gate sayıları (3.513 vb.) | ⚠️ continuity dosyalarının **kendi beyanı** | 0.60 — **taze değil** |

**Doğrulanamayanlar (dürüst kayıt):**
- Bu turda hiçbir `cargo`/`just`/test komutu **çalıştırılmadı** (salt-okuma scope'u). Yeşil-test ve
  p95 iddiaları `.codex` kayıtlarının geçmiş beyanıdır.
- `docs/patterns/tui-composition.md` P1-P7 kataloğu başlık düzeyinde tarandı, içerik derinliğinde
  yeniden okunmadı (mevcut conf değerleri o dokümandan devralındı).
- Bölüm I'deki dış referansların HİÇBİRİ bu turda incelenmedi — orada confidence ataması yoktur.
- `docs/analysis/2026-07-24-architecture-seams.md` bu yazım anında henüz mevcut değildi (kardeş
  agent üretiyor) — `related:` ileri referanstır.

---

## Karar Girdisi — Üç Cümle

- **Ne var:** SF2/SF3/SF4'ün ürettiği gramer (bölge + track + generation + tek transaction)
  production-grade ve test edilmiş. Kullanıcının asıl istediği iki etkileşim — kolon kenar
  drag-resize (16..64) ve yatay kaydırılabilir Miller — **CANLI ve 19 isimli testle kanıtlı**.
  Mimari kendini sahada bir kez kanıtladı.
- **Ne yok:** Üretim render'ı eski 2 bölgeli default'ta duruyor; AppDock, TopBar, RightPanel,
  BottomBar, 4 template ve TrackPolicy yolu **karanlıkta bekliyor**. Kullanıcı config'den layout
  tanımlayamıyor. B-zincirinin onaylı tasarımı hiç yazılmadı.
- **Sıradaki adım:** `src/ui.rs:303` bu tesisatın vanası. Açmadan önce **iki kapı** var — (1)
  `Files Layout V1` kilidi bunu V2-sınıfı ilan ediyor, (2) B1→B2→B3 kendi design gate'inden geçmeli.
  **Teknik olarak küçük, yönetişim olarak büyük bir adım.** Belge/tablo yüzeyi ise bu vanadan
  **bağımsız** ilerleyebilir: PDF/XLSX zaten sınıflandırılmış, detay-yüzeyi yolu (B-1) V1.x
  kapsamında ve düşük riskli — kullanıcının PNG/XLSX/PDF odağı için **hemen başlanabilir tek şerit budur**.

---

*Kaynak: tek-agent derin analiz turu, 2026-07-24 · salt-okuma · `feat/native-fm` @ `b48bd903`.*
*Damıtılmış desenler: `docs/patterns/custom-layout.md` · Kanıt tablosu: `docs/references/custom-layout.md`.*
