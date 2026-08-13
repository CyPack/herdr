---
doc: herdr-references-registry
domain: custom-layout
created: 2026-07-24
status: canonical — çıplak iddia yok; her giriş tier + confidence taşır (evidence-propagation uyumlu)
snapshot: feat/native-fm @ b48bd903
git_note: >
  /docs/* herdr .gitignore'da IGNORED (yalnız /docs/next/ istisna) → bu dizin LOKAL yaşar,
  upstream'e/PR'a SIZMAZ (external-contributor guardrail'e bilinçli uyum).
  Doğrulandı: `git check-ignore -v docs/patterns/tui-composition.md` → `.gitignore:10:/docs/*`.
  Makine-kopyası: ~/.cartography/herdr-custom-layout-*
agentic_triggers:
  - "custom layout · shell template · region · track · bölge geometrisi"
  - "AppDock · TopBar · RightPanel · BottomBar · dock track"
  - "drag resize · divider · ResizeTransaction · kolon genişliği · collapse"
  - "B-chain · layout DSL · kullanıcı tanımlı layout · layout kalıcılığı"
  - "ShellLayout · RegionRects · ShellView · ShellGeometryKey · StageSurfaceView"
related:
  - docs/patterns/custom-layout.md                 # pattern kataloğu (bu registry'nin damıtılmış hâli)
  - docs/analysis/2026-07-24-custom-layout-state.md # durum analizi (bu registry'nin tüketicisi)
  - docs/references/tui-composition.md             # kardeş registry (dış TUI ekosistemi kaynakları)
  - docs/references/native-file-manager.md         # kardeş registry (FM refpool klonları)
---

# herdr Referans Registry — DOMAIN: custom-layout

> Shell/layout alt sisteminin "hangi iddia hangi kaynağa dayanıyor" tablosu. Bu domain'de derin
> araştırma yapan HER agent bulduğu kaynağı BURAYA ekler ([[reference-registry]] 5-adım).
>
> **Kardeş registry ayrımı:** `docs/references/tui-composition.md` DIŞ ekosistem kaynaklarını
> (zellij/helix/k9s/cursive…) tutar. **Bu dosya herdr'ın KENDİ layout altyapısının iç kanıtını**
> tutar — kaynak modüller, test adları, commit SHA'ları, spec/lock dosyaları. Çakışma yok; dış
> referans gerektiğinde tui-composition registry'sine köprü kurulur.

## Tier sözlüğü

`source_code` (bu reponun canlı kodu — en güçlü yerel kanıt) · `spec` (dondurulmuş tasarım/plan
dokümanı) · `product-lock` (kilitlenmiş ürün sözleşmesi, ihlali V2 kararı gerektirir) ·
`continuity` (`.codex` kayıt/handoff beyanı — kendi beyanı, taze kanıt değil) ·
`evidence-file` (`.codex/evidence/*` faz kapanış kaydı) · `executable` (bu turda çalıştırılmış
komut çıktısı — grep/ls/git) · `commit` (git nesnesi) · `research-note` (analiz notu, uygulama kararı yok).

---

## 1. Kaynak modüller — DIŞ KABUK (shell)

| Etiket | Kaynak | Tier | Conf | Desteklediği pattern | Konu |
|---|---|---|---|---|---|
| `[shell-model]` | `src/ui/shell/model.rs` (351 satır) | source_code | 0.98 | CL1, CL2, CL10 | `RegionId`(7), `TrackPolicy`(5), `ShellNode{Slot,Split}`, `ShellLayout::validate`, `ValidatedShellLayout`, `ShellValidationError`(13), `RegionRects`, bounded sabitler (depth≤4, children≤8, leaves≤64, nodes≤128, stacks≤32, placements≤64) |
| `[shell-model-deser]` | `src/ui/shell/model.rs:269-314` | source_code | 0.95 | CL10 | `SerializedShellLayout` untagged: `Tree{root,tracks,stacks,component_placements}` VEYA `Template{template}`; `deny_unknown_fields`; deserialize→validate zorunlu |
| `[shell-model-legacy]` | `src/ui/shell/model.rs:146-153` | source_code | 0.95 | CL2 | `from_legacy_root` → `tracks: BTreeMap::new()` (BOŞ) — üretimdeki TrackPolicy ölü-yolunun kökü |
| `[shell-template]` | `src/ui/shell/template.rs` (155) | source_code | 0.95 | CL2 | 5 template: `StageOnly, DockStage, DockSidebarStage, DesktopWorkspace, InspectorWorkspace`; `dock_track()` 3/5/9, `sidebar_track()` 4/26/40 |
| `[shell-template-nodsl]` | `src/ui/shell/template.rs:10` | source_code | 0.98 | CL-KM (karar matrisi) | v0 kararı birebir: *"Closed built-in page templates; Foundation v0 exposes no arbitrary layout DSL."* |
| `[shell-solver]` | `src/ui/shell/layout.rs` (593) | source_code | 0.95 | CL2, CL6 | `solve`, `measure_node`/`allocate_node`, `TrackRequest`, `allocate_lengths`, `distribute_fill` (largest-remainder), `ResponsiveDegradation`(5) |
| `[shell-solver-degrade]` | `src/ui/shell/layout.rs:443-503` | source_code | 0.95 | CL6 | `degrade_workspace_requests` sırası: RightPanel collapse → LeftPanel compact(4) → AppDock collapse → TooSmall; dikey: BottomBar → TopBar |
| `[shell-solver-fallback]` | `src/ui/shell/layout.rs:349-376` | source_code | 0.95 | CL2 | `request_for_child` → track yoksa `request_from_legacy_size`; üretimde HEP bu dal çalışıyor |
| `[shell-view]` | `src/ui/shell/view.rs` (206) | source_code | 0.98 | CL3, CL5 | `ShellGeometryKey{area, layout_revision, constraints_revision, collapse_revision}`, `ShellView{generation, regions, hits, degradation, geometry_key}`, `compute_shell_view`, `region_hit_at`, `flatten_region_hits` |
| `[shell-view-exhaust]` | `src/ui/shell/view.rs:139-156` | source_code | 0.95 | CL5 | Generation exhaustion → `hits: Vec::new()` + generation ilerlemez ⇒ eski hit'e ALIAS yok (fail-closed) |
| `[shell-interaction]` | `src/ui/shell/interaction.rs` (48 KB, 45 test fn) | source_code | 0.95 | CL4, CL5, CL6 | `DividerId`, `MillerResizeColumnId`, `MillerDividerId`, `ResizeTargetId`, `ResizeBounds`, `ResizeTransaction`, `ResizeDecision`, `ResizeUpdate`, `RegionCollapseState`, `ScrollViewportState`, `ShellPresentationState` |
| `[shell-resize-target]` | `src/ui/shell/interaction.rs:101-113` | source_code | 0.98 | **CL4** | `ResizeTargetId::{Shell(DividerId), Miller(MillerDividerId)}` — TEK transaction ailesinin tip-seviyesi kanıtı |
| `[shell-divider-id]` | `src/ui/shell/interaction.rs:53-97` | source_code | 0.95 | CL5 | `MillerDividerId{files_generation:u32, model_revision:u64, leading/trailing: MillerResizeColumnId, axis}` — çift-generation + tam yol kimliği |
| `[shell-resize-update]` | `src/ui/shell/interaction.rs:157-164` | source_code | 0.95 | CL4, CL9 | `ResizeUpdate{decision, mark_persistence_dirty, request_pty_resize}` — preview asla effect üretmez |
| `[shell-mod]` | `src/ui/shell.rs` (1554) | source_code | 0.95 | CL1, CL10 | `ShellLayout::default()` (legacy 2 slot, `:70-93`), `template_persistence_parts` (`:43`), `validate_persisted_shell_parts` (`:55`), 63 test fn |
| `[shell-clientlocal]` | `src/ui/shell.rs:12-16` | source_code | 0.95 | CL11 | Modül doc birebir: *"Pure TUI presentation… none of these types are shared runtime facts, and none appear in `protocol`/`api::schema`"* — server/client sınırının yazılı beyanı |
| `[shell-deadcode]` | `src/ui/shell.rs:23-24` | source_code | 0.9 | — | `#[allow(dead_code)] mod interaction` — reducer'ların adapter'lardan önce landing ettiğinin işareti (R9) |

## 2. Kaynak modüller — ÜRETİM RENDER YOLU

| Etiket | Kaynak | Tier | Conf | Desteklediği pattern | Konu |
|---|---|---|---|---|---|
| `[ui-compute-view]` | `src/ui.rs:270-340` | source_code | 0.98 | CL3 | Üretim geometri: `ShellLayout::default()` (`:303`), `ShellGeometryKey::new(area, LEGACY_DESKTOP_SHELL_LAYOUT_REVISION, sidebar_w, collapse_rev)` (`:305-310`), `resolve_dynamic` sadece LeftPanel (`:313-318`) |
| `[ui-legacy-rev]` | `src/ui.rs:135-136` | source_code | 0.95 | CL3 | `LEGACY_DESKTOP_SHELL_LAYOUT_REVISION=1`, `MOBILE_EMPTY_SHELL_LAYOUT_REVISION=2` — cache anahtarının template-körlüğünün kökü (R2) |
| `[ui-dock-dark]` | `src/ui.rs:829-840` | source_code | 0.98 | — | **Dark-AppDock'un birebir beyanı:** *"The AppDock renders only when the current shell projects it a non-empty region (the legacy default template projects none, so this stays a no-op until a dock-bearing template is live)"* |
| `[ui-dock-hits-empty]` | `src/ui.rs:458-463` | source_code | 0.95 | — | *"the legacy default template projects no dock region, so this stays empty until one is live"* — hit-area tarafındaki ikinci bağımsız beyan |
| `[ui-stage-guard]` | `src/ui.rs:838-851` | source_code | 0.95 | CL8 | `terminal_surface_active` gate + `match app.stage.surface_view()` render seçimi (tipli otorite) |
| `[ui-compositor]` | `src/ui.rs:790-800` | source_code | 0.9 | CL9 | `compose::Compositor::new(vec![...])` — base chrome + tek aktif overlay |
| `[ui-preview-gate]` | `src/ui.rs:511` | source_code | 0.9 | CL4 | `resize_panes_during_shell_preview` — preview sırasında pane resize bastırma |
| `[ui-mobile-empty]` | `src/ui.rs:598-610` | source_code | 0.95 | — | `compute_empty_shell_view` + yorum: *"named shell regions are a desktop concept for now"* (R6) |
| `[ui-render-stream]` | `src/server/render_stream.rs:298` → `compute_view_without_resizing_panes` (`src/ui.rs:183`) | source_code | 0.9 | CL11 | Sunucu render akışının geometri girişi |
| `[ui-compose]` | `src/ui/compose.rs` (133) | source_code | 0.9 | CL9 | `Compositor`, `RenderCtx`, `dyn Component` |

## 3. Kaynak modüller — STAGE / SURFACE

| Etiket | Kaynak | Tier | Conf | Desteklediği pattern | Konu |
|---|---|---|---|---|---|
| `[stage-host]` | `src/ui/surface_host.rs` (649) | source_code | 0.98 | CL8 | `BuiltInAppId{Terminal,Files}`, `AppSurfaceRef`, `StageSurfaceView`, `LaunchPolicy{Singleton}`, `AppDefinition`, `AppInstanceId{app,generation:u32}`, `StageState`, `StageStateError`(2) |
| `[stage-bounds]` | `src/ui/surface_host.rs:1, :77-83` | source_code | 0.98 | CL8 | `MAX_BUILT_IN_INSTANCES=16`; `instances: [Option<AppInstance>; 16]`; **`last_generations: [Option<u32>; 2]`** ⚠️ yeni app eklerken büyütülmeli (R5) |
| `[stage-index]` | `src/ui/surface_host.rs:10-15` | source_code | 0.95 | CL8 | `BuiltInAppId::index()` sabit `0/1` — kapalı-enum genişleme maliyetinin ikinci noktası |
| `[stage-surface-view]` | `src/ui/surface_host.rs:106-115` | source_code | 0.95 | CL8 | `surface_view()` doc: *"pure read of typed Stage state: it can never own, create, resize, or destroy terminal runtime state"* |
| `[stage-activate]` | `src/ui/surface_host.rs:117-142` | source_code | 0.9 | CL8 | `activate_files` — Files'a özel; genelleştirme (`activate(app)`) yeni yüzey için gerekli |
| `[stage-tests]` | `src/ui/surface_host.rs:253-647` | source_code | 0.95 | CL8 | 8 test: `stage_starts_on_terminal_workspace`, `activating_files_records_previous_surface`, `reactivating_singleton_files_keeps_one_surface`, `stage_rejects_more_than_sixteen_builtin_instances`, `instance_generation_exhaustion_fails_without_aliasing`, `closing_files_restores_previous_terminal_surface`, `failed_files_open_restores_previous_surface_and_focus`, `active_surface_alone_populates_stage_hits`, `hidden_surface_has_no_stale_hits_or_cursor`, `stage_surface_switch_does_not_destroy_terminal_runtime` |
| `[app-dock]` | `src/ui/app_dock.rs` (384) | source_code | 0.95 | CL2 | `AppDockEntry{app,active,running,enabled}`, `AppDockModel::for_state`, `app_dock_entry_areas`, `render_app_dock`; ikon çiftleri (Nerd `❯`/`▤`, ASCII `>`/…) |
| `[app-dock-track-doc]` | `src/ui/app_dock.rs:1-8` | source_code | 0.9 | CL2 | Modül doc: boyut politikası `template::dock_track()` tarafından donduruluyor (preferred 5, min 3, max 9) |

## 4. Kaynak modüller — MILLER (kolon zinciri)

| Etiket | Kaynak | Tier | Conf | Desteklediği pattern | Konu |
|---|---|---|---|---|---|
| `[miller-model]` | `src/fm/miller.rs` | source_code | 0.98 | CL7 | `MillerState{chain: VecDeque<MillerPathSegment>, horizontal, focused_directory, preview_preferred_width, revision}`, `MillerPathSegment{directory, preferred_width}`, `MillerHorizontalViewport{offset_cells:u32, follow_active:bool}`, `MillerAdjacentWidthTarget` |
| `[miller-bounds]` | `src/fm/miller.rs:10-14` | source_code | 0.98 | CL7 | `MAX_MILLER_HISTORY_DEPTH=32`, `MILLER_COLUMN_MIN_WIDTH=16`, `MILLER_COLUMN_PREFERRED_WIDTH=28`, `MILLER_COLUMN_MAX_WIDTH=64`, `MILLER_DETAIL_MIN_WIDTH=20` — **rehber §1.3 ile birebir** |
| `[miller-commit]` | `src/fm/miller.rs:143-227` | source_code | 0.9 | CL4 | `preferred_widths_for`, `commit_column_width`, `commit_adjacent_column_widths` |
| `[miller-invariants]` | `src/fm/miller.rs:228` | source_code | 0.9 | CL7 | `assert_miller_invariants_for_test` |
| `[miller-adapter]` | `src/app/file_manager_miller.rs` | source_code | 0.95 | CL4 | `begin_miller_resize_capture` (`:102`), `commit_miller_resize` (`:164`), `handle_miller_resize_key` (`:236`), `handle_miller_horizontal_scroll` (`:290`), `miller_resize_column_id` (`:321`) |
| `[miller-mouse]` | `src/app/input/file_manager.rs:655` | source_code | 0.9 | CL4 | `handle_active_miller_resize_mouse` |
| `[miller-view]` | `src/ui/file_manager/miller.rs` | source_code | 0.9 | CL3 | `project_miller_view_with_resize_preview` (`:128`), `miller_resize_column_is_live` (`:348`) |
| `[trail-view]` | `src/ui/file_manager/trail_view.rs` | source_code | 0.9 | CL7 | `trail_column_width` (`:121`), `fractional_scroll_step` (`:559`), `horizontal_scroll_target` (`:597`) |
| `[miller-cancel]` | `src/app/input/shell.rs:291` | source_code | 0.9 | CL4 | `cancel_miller_resize_for_terminal_area` — terminal resize transaction'ı iptal eder |

## 5. Kaynak modüller — ÖNİZLEME / YÜZEY TÜRÜ (belge/tablo genişlemesi için)

| Etiket | Kaynak | Tier | Conf | Desteklediği pattern | Konu |
|---|---|---|---|---|---|
| `[preview-capability]` | `src/fm/preview_capability.rs` | source_code | 0.95 | CL8 | `PreviewCapability{NativeText, NativeImage, MetadataOnly, OptionalPlugin{action_id,fallback}, Unsupported}`, `PreviewFallback`, `PreviewReason`(8), `PreviewPluginProvider`, `PreviewProviderSet{markdown,documents,archives,media}` |
| `[preview-docs-ext]` | `src/fm/preview_capability.rs:126-136` | source_code | 0.95 | CL8 | **PDF/XLSX/DOCX ZATEN sınıflandırılmış:** `["pdf","doc","docx","odt","rtf","xls","xlsx","ods","ppt","pptx","odp"]` → `documents` sağlayıcısı veya `MetadataOnly(DocumentMetadata)` |
| `[preview-image]` | `src/fm/preview_capability.rs:109-111` | source_code | 0.95 | CL8 | `is_image_preview_path` → `PreviewCapability::NativeImage` |
| `[preview-purity]` | `src/fm/preview_capability.rs:1-5` | source_code | 0.95 | CL9 | Modül doc: *"never reads the filesystem, checks PATH, loads configuration, spawns a process, or mutates file-manager navigation"* |
| `[trail-detail]` | `src/fm/trail_snapshots.rs:36-45` | source_code | 0.9 | CL8 | `TrailDetailPreview{PendingText, Text(TextPreview), Image, MetadataOnly(String), Unpreviewable(String)}` |
| `[trail-detail-image-todo]` | `src/fm/trail_snapshots.rs:40-42` | source_code | 0.9 | — | Kod yorumu birebir: *"A recognized image; pixel delivery is the Kitty-graphics track (FIP-D4) and completes at integration"* ⇒ PNG piksel render TAMAMLANMADI |
| `[trail-detail-map]` | `src/fm/trail_snapshots.rs:705` | source_code | 0.9 | CL8 | `PreviewCapability::NativeImage => TrailDetailPreview::Image` eşlemesi |
| `[preview-worker]` | `src/app/file_preview_worker.rs:57, :92, :382` | source_code | 0.85 | CL9 | `FilePreviewSync`, `FilePreviewSource`, `sync_file_preview_worker` — bounded I/O şeridi |
| `[plugin-registry]` | `src/persist/plugin_registry.rs` | source_code | 0.9 | CL11 | `plugins.json` (`session::data_dir()`), atomik yazım (tmp+rename), `InstalledPluginInfo` (`api::schema`) — **SUNUCU tarafı** |

## 6. Kaynak modüller — KALICILIK

| Etiket | Kaynak | Tier | Conf | Desteklediği pattern | Konu |
|---|---|---|---|---|---|
| `[persist-shell-v1]` | `src/persist/snapshot.rs:28-42` | source_code | 0.98 | CL10 | `ShellSnapshotV1{schema_version, template, root, region_constraints, component_placements, collapse_restore_widths, pinned_dock_order}` + `deny_unknown_fields` |
| `[persist-versions]` | `src/persist/snapshot.rs:15-18` | source_code | 0.95 | CL10 | `SNAPSHOT_VERSION=4`, `SHELL_SNAPSHOT_VERSION=1` |
| `[persist-migration]` | `src/persist/snapshot.rs:57, :118-127, :635-650` | source_code | 0.9 | CL10 | v3→v4 sidebar migration; şema-versiyon reddi; gelecek-versiyon reddi |
| `[persist-hardcoded-template]` | `src/persist/snapshot.rs:75` | source_code | 0.95 | — | `let template = ShellTemplateId::DockSidebarStage;` — SABİT KODLU; runtime'la çelişir (R3) |
| `[persist-pinned]` | `src/persist/snapshot.rs:44-48` | source_code | 0.9 | CL10 | `PinnedBuiltinAppV1{Terminal, Files}` — yeni yüzey eklenirse şema uyumluluğu gerekir |
| `[persist-restore-partial]` | `src/app/mod.rs:438, :954` | source_code | 0.93 | CL10 | Restore SADECE `restored_left_panel_preference()` okuyor ⇒ `root`/`tracks`/`placements`/`pinned_dock_order` geri UYGULANMIYOR |
| `[persist-validate]` | `src/ui/shell.rs:55-61` | source_code | 0.9 | CL10 | `validate_persisted_shell_parts` — diskten gelen ağaç aynı bounded validate'ten geçiyor |
| `[persist-no-miller-width]` | `src/persist/` grep `column_width|preferred_width` = 0 | executable | 0.9 | — | Miller kolon genişlikleri persist EDİLMİYOR |

## 7. Diğer layout otoriteleri (çakışma haritası)

| Etiket | Kaynak | Tier | Conf | Desteklediği pattern | Konu |
|---|---|---|---|---|---|
| `[tile-layout]` | `src/layout.rs` (957) | source_code | 0.9 | CL-KM | `PaneId`, `PaneInfo`, `SplitBorder`, `NavDirection`, `Node`, `TileLayout`, `find_in_direction` — WorkspaceStage İÇİNDEKİ BSP pane ağacı; shell'den BAĞIMSIZ |
| `[mobile-ui]` | `src/ui/mobile.rs` (47 KB) | source_code | 0.85 | — | Kendi header/terminal split'i; shell bölgeleri kullanılmıyor |
| `[config-model]` | `src/config/model.rs:924, :1046` | source_code | 0.9 | CL-KM | Layout alanı YOK; sadece `mobile_width_threshold` (64) ve klavye düzeni notu |

## 8. Spec / plan / lock dokümanları

| Etiket | Kaynak | Tier | Conf | Desteklediği pattern | Konu |
|---|---|---|---|---|---|
| `[guide-custom-layout]` | `docs/superpowers/specs/2026-07-17-herdr-custom-layout-architecture-guide.md` (149 satır) | spec | 0.8 ⚠️ | tüm CL* | Hedef deneyim §1, faz zinciri §2, bounded model §3, SSH perf §4, input §5, anti-pattern §6, durum §7. **BAYAT:** §7 tablosu 6 faz geride; §3 resident-bound emekli. Kendi beyanı: *"DERIVED, not invented"* |
| `[prd-fip-custom-layout]` | `docs/superpowers/specs/2026-07-18-herdr-fip-closure-and-custom-layout-prd.md` | spec | 0.9 | CL-KM | **B-zincirinin tek tanımı:** §2 A/B ayrımı, §4-B B1..B4 enumerasyonu, §6 kabul kriterleri |
| `[lock-files-layout-v1]` | `docs/superpowers/specs/2026-07-19-herdr-files-layout-v1-lock.md` | **product-lock** | 0.95 | CL-KM | Kompozisyon `global panel \| locations rail \| Miller Trail \| detail`; V1-L1..L8; VIS-07..25; **Versioning Rule** (dört-yüzey değişimi = V2); freeze `d98c31c70946e496cb6536f02fc96e45974df2de` |
| `[spec-shell-foundation]` | `docs/superpowers/specs/2026-07-15-herdr-shell-foundation-v0-design.md` (47 KB) | spec | 0.9 | CL1..P6 | SF programının kanonik tasarımı (bounds, degradation, typed surface, migration, rollback, perf bütçeleri) |
| `[plan-shell-foundation]` | `docs/superpowers/plans/2026-07-15-herdr-shell-foundation-v0-implementation.md` (34 KB) | spec | 0.9 | CL1..P6 | SF0-SF6 kod-seviyesi TDD planı |
| `[plan-fm-post-shell]` | `docs/superpowers/plans/2026-07-15-herdr-file-manager-post-shell-implementation.md` (27 KB) | spec | 0.9 | CL4, CL7 | FM1-FM5; "Frozen Interfaces and Bounds"; FM2.1 "do not add dock-specific drag state" precedent'i |
| `[plan-shell-fm-program]` | `docs/superpowers/plans/2026-07-15-herdr-shell-file-manager-program-plan.md` (17 KB) | spec | 0.85 | — | Program-üstü plan |
| `[mockup-custom-layout]` | `.local/prd/custom-layout-target-mockup.md` | spec (lokal) | 0.85 | CL-KM | Excalidraw mockup bölge dökümü + mevcut-seam eşleme tablosu (TopBar/LeftPanel×2/CenterStage/RightRail/RightPanel/BottomBar) |

## 9. Evidence dosyaları (faz kapanış kayıtları)

| Etiket | Kaynak | Tier | Conf | Desteklediği pattern | Konu |
|---|---|---|---|---|---|
| `[ev-sf1]` | `.codex/evidence/shell-foundation-sf1-characterization.md` | evidence-file | 0.85 | — | Baseline karakterizasyon (11/11) |
| `[ev-sf2]` | `.codex/evidence/shell-foundation-sf2-geometry-progress.md` (10 KB) | evidence-file | 0.85 | CL1, CL2, CL3 | Bölge modeli + solver + cached view kapanışı |
| `[ev-sf3-interaction]` | `.codex/evidence/shell-foundation-sf3-interaction-progress.md` | evidence-file | 0.85 | CL4 | Divider transaction + sidebar adaptasyonu |
| `[ev-sf3-collapse]` | `.codex/evidence/shell-foundation-sf3-collapse-scroll-progress.md` | evidence-file | 0.85 | CL6 | Collapse/scroll reducer |
| `[ev-sf3-persist]` | `.codex/evidence/shell-foundation-sf3-persistence.md` (8.5 KB) | evidence-file | 0.85 | CL10 | Snapshot v4 shell state |
| `[ev-sf4-stage]` | `.codex/evidence/shell-foundation-sf4-stage-progress.md` (11 KB) | evidence-file | 0.85 | CL8 | Tipli Stage + lifecycle |
| `[ev-sf4-router]` | `.codex/evidence/shell-foundation-sf4-input-router-progress.md` (26 KB) | evidence-file | 0.85 | **CL12**, CL5 | Input precedence 8 dilim (overlay→capture→topmost→focus→page→global→fail-closed) |
| `[ev-sf4-projection]` | `.codex/evidence/shell-foundation-sf4-surface-projection-progress.md` (10 KB) | evidence-file | 0.85 | CL9 | Saf render + retained path |
| `[ev-sf5-dock]` | `.codex/evidence/shell-foundation-sf5-app-dock-progress.md` | evidence-file | 0.85 | CL2 | AppDock |
| `[ev-sf6-files]` | `.codex/evidence/shell-foundation-sf6-files-stage-progress.md` | evidence-file | 0.85 | CL8 | Files→Stage migration |
| `[ev-plan-review]` | `.codex/evidence/shell-foundation-plan-review.md` | evidence-file | 0.8 | — | Plan öz-denetimi |
| `[ev-fm5-preview]` | `.codex/evidence/fm5-preview-placement-decision.md` (10.5 KB) | evidence-file | 0.85 | CL-KM | **RightPanel/Inspector NO-GO kararı** — inline preview korundu |
| `[ev-layout-v1-audit]` | `.codex/evidence/files-layout-v1-symlink-audit.md` | evidence-file | 0.8 | — | V1 freeze denetimi |

## 10. Continuity kayıtları (B-chain durumu)

| Etiket | Kaynak | Tier | Conf | Desteklediği pattern | Konu |
|---|---|---|---|---|---|
| `[cont-next-session]` | `.codex/NEXT-SESSION-PROMPT.md:128` | continuity | 0.9 | CL-KM | *"Custom-layout B-chain is separate and starts only from its own approved design/plan."* |
| `[cont-current]` | `.codex/CURRENT.md:412` | continuity | 0.9 | CL-KM | *"then custom-layout B-chain only under its own plan."* |
| `[cont-handoff]` | `.codex/HANDOFF.md:601-604` | continuity | 0.9 | CL-KM | Kullanıcı direktifi: custom layout programı **kendi brainstorm→design→plan kapısıyla**; mockup + rehber pointer'ları |
| `[cont-miller-retired]` | `.codex/NEXT-SESSION-PROMPT.md:118-121` | continuity | 0.85 | — | *"Do not restart T7 or restore the retired parent/current/resident projection"* ⇒ rehber §3 resident-bound EMEKLİ |
| `[tasks-sf-fm]` | `.codex/TASKS.md:888-1200` | continuity | 0.9 | — | SF0-SF6 + FM1-FM5 blokları, TAMAMI `[x]`; commit SHA'ları ve gate sayıları |
| `[tasks-s5]` | `.codex/TASKS.md:1954` | continuity | 0.9 | CL-KM | S5 ComponentRegistry — *"only when a second real component/page proves the abstraction"* (AÇIK) |
| `[tasks-s7]` | `.codex/TASKS.md:1959` | continuity | 0.9 | CL-KM | S7 popup stack (AÇIK, trigger-gated) |
| `[tasks-open-14]` | `.codex/TASKS.md` grep `^- \[ \]` = 14 | executable | 0.9 | — | Açık madde envanteri |

## 11. Commit çapaları (faz kapanışları)

| Etiket | Commit | Tier | Conf | Konu |
|---|---|---|---|---|
| `[c-sf0]` | `32856f7` | commit | 0.85 | SF0 artefakt yayını |
| `[c-sf1]` | `7b9b626d` | commit | 0.85 | `test: characterize shell foundation baseline` |
| `[c-sf2]` | `07133b8b9e9cf10b9b3dea0febe22a8389457164` | commit | 0.9 | SF2 kapanış (bölge+solver+cached view) |
| `[c-sf2-partial]` | `f272a881`, `2a440478`/`07133b8b` | commit | 0.85 | SF2.1-2.3 zinciri, SF2.4 RED/GREEN |
| `[c-sf3-divider]` | `368c4d3a`/`d89a7f94`, `b6570ee4`/`807cb76c` | commit | 0.85 | Divider reducer RED/GREEN |
| `[c-sf3-sidebar]` | `96a1660e`, `09944834` → `61b915a9` | commit | 0.85 | Sidebar divider adaptasyonu |
| `[c-sf3-keys]` | `4888c3f8`, `4026c28b`, `960b6d5f` → `336fa3de` | commit | 0.85 | Klavye resize yolu |
| `[c-sf3-collapse]` | `45a2e87e` | commit | 0.85 | SF3.2 collapse/scroll kapanışı |
| `[c-sf3-persist]` | `90be689359988424b2a7c6206ff45a3207422196` | commit | 0.9 | SF3.3 snapshot v4 |
| `[c-sf4-1]` | `557bcc77`/`6a18f0c7` … `784fdc2e`/`944a9d4c` (8 çift) | commit | 0.85 | SF4.1 Stage lifecycle dilimleri |
| `[c-sf4-2]` | `20f659c1` (head); `92777e23`/`f4f5e3cb`, `41362e89`/`017ba97f`, `bb6f8970`/`efe6446b`, `119e4a2d`, `8b1882eb`/`5eb63763`, `27f8699f`/`3880c66b`, `3580ff19`, `bb3ac54d`/`c6b024ce` | commit | 0.85 | SF4.2 input router 8 dilim |
| `[c-sf4-4]` | `7796d855`/`acc82ffd`, `bb5a6899`/`1bc69cf5`, `a9b67112`/`f973740e`, `08d73676`, `1f57ccbb` | commit | 0.85 | SF4.4 projeksiyon + saflık + retained path |
| `[c-sf4-close]` | `f973740e` | commit | 0.9 | SF4 kapanış başı |
| `[c-sf5]` | `64d5dd5e`/`cb0c77fd` (5.1), `406db487`/`d031ef26` (5.2) | commit | 0.85 | AppDock |
| `[c-fm1]` | `35cfbc00` | commit | 0.85 | Miller viewport projeksiyonu |
| `[c-layout-v1-freeze]` | `d98c31c70946e496cb6536f02fc96e45974df2de` | commit | 0.9 | Files Layout V1 freeze checkpoint |
| `[c-snapshot]` | `b48bd903` | commit | 0.95 | Bu analizin çekildiği HEAD |

## 12. Araştırma notları (uygulama kararı YOK)

| Etiket | Kaynak | Tier | Conf | Konu |
|---|---|---|---|---|
| `[research-multimonitor]` | `research/multi-monitor-shared-view.md` (7.6 KB, `933e4b8`'e karşı doğrulanmış) | research-note | 0.85 | Tek `AppState`; `switch_workspace_tab` tek mutasyon noktası; frame broadcast; en-küçük-client boyutu; "foreground client" input kavramı — **implementation decision YOK** |
| `[research-yazi-coalesce]` | `.local/prd/2026-07-20-yazi-render-coalescing-study.md` | research-note | 0.8 | Render coalescing çalışması |

## 13. Dış ekosistem köprüsü

> Dış TUI layout kaynakları **kardeş registry'de** tutulur; burada tekrar EDİLMEZ, köprü verilir.

| Konu | Kardeş registry girdisi | Bu domain'e katkısı |
|---|---|---|
| Deklaratif layout dosyası (KDL) | `[zellij-layout]`, `[zellij-kdl-docs]` — `docs/references/tui-composition.md:37-38` | Senaryo (a) kullanıcı-tanımlı template şeması |
| Persistent chrome + swappable body + registry | `[k9s-app-layout]`, `[k9s-registrar]`, `[k9s-command]` | S5 kararı; CL2 genişlemesi |
| Compositor / overlay katmanı | `[helix-compositor]`, `[helix-overlay]`, `[helix-popup]`, `[cursive-stackview]` | S7 popup stack; CL9 |
| Floating pane z-index | `[zellij-floating]`, `[nvim-api-doc]` | Gelecek: bölge-dışı floating panel |
| Ratatui Component trait | `[ratatui-templates-component]` | CL9 Compositor sözleşmesi |

## 14. ⚠️ İNCELENMEYEN adaylar (confidence ataması YOK)

> Bu turda **araştırılmadı**. Kayıt amacı: gelecekte sıfırdan başlanmasın. İncelendiklerinde
> yukarıdaki tablolara tier+conf ile TAŞINACAKLAR. Detaylı gerekçe:
> `docs/analysis/2026-07-24-custom-layout-state.md` Bölüm I.

| Aday | Nereden başlanmalı | Not |
|---|---|---|
| tmux `select-layout` preset'leri / wezterm Lua mux | tmux(1) man; wezterm `wezterm.mux` docs | Preset vs serbest-ağaç ikilisi |
| i3 / sway tiling config | i3 user guide "Configuring i3"; sway-config(5) | Kullanıcı DSL ifade gücü |
| Cassowary constraint solver | Badros & Borning makalesi; `cassowary-rs` | ⚠️ ratatui 0.26'da Layout motorundan ÇIKARILDI — tarihsel bağlam kritik |
| CSS Grid track sizing | CSS Grid L1 §7; Flexbox §9 | `TrackPolicy` ≈ `minmax()`/`fr` semantiği |
| Zed dock/panel sistemi | `zed-industries/zed` → `crates/workspace/src/dock.rs`, `pane_group.rs` | Dock kalıcılığı (R3/S6) |
| VSCode workbench layout | `src/vs/workbench/browser/layout.ts` | Layout şema versiyonlama |
| `ratatui-hypertile` | crates.io varlık doğrulaması ÖNCE | ⚠️ refpool'da **YOK** (doğrulandı: `~/.cartography/refpool/` = yazi-src, superfile-src, joshuto, ratatui-image, yeet, rat-commander) |
| `tui-studio` | Varlık/kimlik doğrulaması ÖNCE | ⚠️ refpool'da **YOK** |

---

## Registry bakım kuralı

Yeni kaynak eklerken: **etiket + kaynak + tier + conf + desteklediği pattern + konu** — altı kolon da
zorunlu. Çıplak kaynak (tier/conf'suz) girdi KABUL EDİLMEZ. Fetch edilemeyen/doğrulanamayan kaynak
`⚠️ doğrulanamadı` diye işaretlenir, uydurulmaz. Bölüm 14'ten bir aday incelenirse ilgili tabloya
TAŞINIR (kopyalanmaz).

---

*Kaynak: tek-agent derin analiz turu, 2026-07-24 · salt-okuma · `feat/native-fm` @ `b48bd903`.*
*Damıtılmış hâli: `docs/patterns/custom-layout.md` · Tüketici analiz: `docs/analysis/2026-07-24-custom-layout-state.md`.*
