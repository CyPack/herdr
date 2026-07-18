# TRAIL-T7 Cerrahi Planı — Eski Model Sökümü + Trail Entegrasyonu

Tarih: 2026-07-18 · Girdi: T1-T6 KAPALI (`7d5edecb`→`c8e5dd4d`), trail çekirdeği
`fm::trail` + `fm::trail_snapshots` + `ui::file_manager::trail_view` hazır ve 3,534/3,534 yeşil.
Risk sınıfı: **refactor-risk** (2+ core surface: FmState/miller, ui render, input, watcher,
persisted horizontal state). CLAUDE.md gereği: characterization ÖNCE, adversarial fixture'lar,
`AppState::assert_invariants_for_test()` kullanımı.

## Söküm/entegrasyon yüzey haritası (2026-07-18 ölçümü)

| Yüzey | Dosya | Satır | İş |
|---|---|---|---|
| Eski model çekirdeği | `src/fm/miller.rs` | 810 | chain/resident-cache → TrailState/TrailSnapshots ile değiştir |
| Eski projeksiyon | `src/ui/file_manager/miller.rs` | 1,258 | `miller_viewport_geometry` KALIR (trail_view kullanıyor); MillerViewSnapshot/Resident/Preview projeksiyon katmanı sökülür |
| App adapter | `src/app/file_manager_miller.rs` | 803 | prepare/apply navigation makinesi → trail activate seam'ine |
| Girdi | `src/app/input/file_manager.rs` | 6,671 | tık/klavye arm'ları → `trail_row_at` + `activate_entry` + `move_selection`; sidebar tıkı → `open_trail_to` (**FIP-D1 ürün kapanışı**) |
| Render | `src/ui/file_manager.rs` (~596-800) | — | render_file_manager gövdesi → `render_trail_view`; `"(unavailable)"` 4 kullanım SİLİNİR |
| Watcher | `src/app/file_manager_watcher.rs` | — | refresh → `refresh_col` + `sync`; sidebar navigation seam'i `open_trail_to`'ya |
| Preview worker | `src/app/image_preview_worker.rs` | — | kitty hedefi trail detail panel content_rect'ine (FIP-D4 iziyle birleşir) |

## T7.1 characterization test noktaları

| ID | Korunan davranış / neden | Beklenen sonuç | Test kanıtı | T7.6 kararı |
|---|---|---|---|---|
| TP-TRAIL-T7-CHAR-01 | Files aç/kapa instance generation ailesi; model generation'ları yeniden başlayabildiği için stale iş yalnız Stage kimliğiyle reddedilebilir | Açılışta canlı generation vardır; kapanışta Files/model/typed action otoritesi temizlenir; aynı cwd yeniden açıldığında generation değişir; adversarial workspace/pane kimlik invariants'ı her geçişte korunur | `files_lifecycle_advances_generation_under_adversarial_identity_state`; `prepared_refresh_cannot_apply_after_files_close_reopen`; `plain_selection_and_cursor_focus_follow_close_reopen_lifecycle` | KALIR; trail köprüsünün lifecycle sözleşmesi |
| TP-TRAIL-T7-CHAR-02 | Agent reference path kaynağı; T7.2 `FmState::selected()` sonucunu trail'in exact-path seçimine taşıyacak, terminale yazma hattı yeni bir path icat edemez | Canlı tek satırın exact path'i typed picker/request'e taşınır; bulk/in-flight/stale otorite fail-closed; gönderim yalnız aynı path'in UTF-8 byte'larıdır ve submit byte'ı yoktur | `row_send_agent_prepares_exact_path_and_focused_terminal_identity`; `send_agent_authority_fails_closed_without_current_single_path`; `existing_agent_receives_exact_path_bytes_with_no_submit` | KALIR; seçili-path üreticisi trail olur |
| TP-TRAIL-T7-CHAR-03 | Rename/delete/multi-select otorite ayrımı; cursor odaktır, explicit path set bulk işlemdir | Rename ve delete typed intent'leri exact anchored path taşır; bulk varken row rename fail-closed; normal cursor hareketi explicit set'i değiştirmez; stale row identity başka girdiyi seçemez | `row_rename_opens_exact_file_modal_without_filesystem_work`; `row_delete_converges_on_shared_typed_confirmation_authority`; `row_rename_rejects_bulk_selection_and_inflight_operation`; `keyboard_toggle_range_and_cursor_only_movement_share_selection_model`; `row_selection_snapshot_carries_stable_path_identity` | KALIR; row geometry/path çözümü trail'e uyarlanır |
| TP-TRAIL-T7-CHAR-04 | Watcher sonrası exact-path reconciliation; rename/delete selection'ı bayat bırakamaz | Rename edilen seçili path explicit set ve anchor'dan düşer; cursor güvenli canlı satıra clamp olur; stale pre-reopen completion yeni Files instance'ına uygulanmaz | `watcher_rename_prunes_selected_path_and_keeps_cursor_safe`; `prior_generation_completion_cannot_reload_reopened_same_cwd` | KALIR; refresh taşıyıcısı `TrailSnapshots::refresh_col` olur |
| TP-TRAIL-T7-CHAR-05 | Eski parent/current/preview ve resident projection render sözleşmesi yalnız kontrollü söküm sınırıdır | T7.3/T7.6'ya kadar testler yeşil kalır ve `TRAIL-T7.6 teardown` etiketiyle topluca bulunur; ara commit bunları sessizce değiştiremez | `windowed_render_uses_current_resident_parent_unavailable_and_preview_sources`; `windowed_render_rejects_stale_current_and_preview_generation`; `miller_columns_render_parent_current_and_directory_preview`; FM parent/preview/resident model testleri | SİL/DEĞİŞTİR; aynı commit'te trail eşdeğer kanıtı yeşil olmalı |

## T7.2 FmState köprüsü test noktaları

| ID | Korunan davranış / neden | Beklenen sonuç | RED kanıtı |
|---|---|---|---|
| TP-TRAIL-T7-BRIDGE-01 | Tek exact-path seçim otoritesi; agent handoff ve operasyon çağıranları row index'ten path üretmemeli | `TrailState::selected_path()` en derin işaretli kolonu döndürür; directory seçiminde son boş child kolonuna rağmen seçilen klasör kaybolmaz | `selected_path_uses_deepest_marked_column` |
| TP-TRAIL-T7-BRIDGE-02 | Trail path'ini canlı `FileEntry`'ye bağlayan tek snapshot seam'i | `TrailSnapshots::selected_entry()` trail'in exact path'ini index-aligned snapshot'lardan bulur; deep-link dosyasında aynı path/kind döner | `selected_entry_resolves_deepest_exact_path` |
| TP-TRAIL-T7-BRIDGE-03 | `FmState` trail ve snapshot'ların lifecycle sahibidir; canlı açılışta legacy cursor ile trail ayrışamaz | `FmState::new` sonrası seçili row, trail selected path ve snapshot entry aynıdır; her trail kolonu aynı index'teki snapshot directory ile hizalıdır | `fmstate_owns_aligned_trail_bridge_on_open` |
| TP-TRAIL-T7-BRIDGE-04 | Cursor hareketi agent/operation seçimini trail exact-path otoritesine taşır; explicit bulk set bağımsız kalır | `move_down` sonrası `selected()` yeni trail-selected entry'dir, trail/snapshot hizası korunur ve multi-selection değişmez | `cursor_move_rebuilds_trail_selection_without_bulk_authority` |
| TP-TRAIL-T7-BRIDGE-05 | Empty/missing durum fail-closed kalır; test modeli disk/PTY gerektirmez | `test_empty` tek kök trail taşır, snapshot/selection boş kalır; geçersiz cursor bir path icat etmez | Mevcut `test_empty` aileleri + full suite; GREEN sırasında regression taraması |
| TP-TRAIL-T7-BRIDGE-06 | Watcher/navigation apply yeni modelde atomik yakınsar | Prepared refresh/navigation kabul edildiğinde cwd, cursor, trail selected ve snapshots aynı commit noktasında hizalanır; reddedilen payload hiçbir bridge alanını değiştirmez | Mevcut prepared refresh/navigation characterization aileleri + GREEN sonrası focused expression |

## Alt-adımlar (her biri kendi RED/GREEN + gate + FF yayını)

- **T7.1 Characterization**: korunacak davranışları pinle — Files aç/kapa generation,
  agent-reference akışı (path seçimi trail selected'dan), operations (rename/delete/multi-select)
  cursor otoritesi, `assert_invariants_for_test` adversarial aileleri. Eski-model-özel testler
  (parent/preview kolon, resident cache) SÖKÜM commit'inde birlikte silinecek şekilde İŞARETLE.
- **T7.2 FmState köprüsü**: FmState'e trail alanları (`trail: TrailState`,
  `trail_snapshots: TrailSnapshots` veya FmState'in yerine geçen sarmalayıcı); cursor/selected
  API'si trail selected'dan türetilir (agent handoff + operations kırılmasın).
- **T7.3 Render swap**: `render_file_manager` orta bölge → `project_trail_view`/`render_trail_view`;
  header/action-bar/status korunur; VIS-01-files/VIS-02 baseline'ları YENİ görünüme göre
  yeniden onaylanır (kasıtlı görsel değişim — mutation kanıtıyla).
- **T7.4 Girdi swap**: mouse/klavye arm'ları trail seam'lerine; sidebar FAVORITES/LOCATIONS →
  `open_trail_to` (FIP-D1 kapanış kanıtı: canlı tık + birim aile); çoklu-seçim/operations
  hit'leri trail satır rect'lerine.
- **T7.5 Watcher + preview**: refresh_col entegrasyonu; kitty image hedefi panel content_rect
  (FIP-D4 çözümü burada test edilir: Ghostty'de canlı foto kanıtı — izole dev reçetesi).
- **T7.6 Söküm + kapanış**: ölü eski-model kodu tek characterized commit'te silinir
  (`"(unavailable)"` grep=0 kabulü); full suite + iki clippy + python + bun + diff-check +
  unwrap-scan; görsel suite tüm baseline'lar; E2E izleri (FIP-6.3 investigation ayrı task);
  continuity + FF yayın; graf reindeks.

## T7.3 Render swap test noktaları

| ID | Korunan davranış / neden | Beklenen sonuç | RED kanıtı |
|---|---|---|---|
| TP-TRAIL-T7-RENDER-01 | `compute_view` render ve sonraki input katmanına tek Trail geometri snapshot'ı yayımlar; render kendi koordinatını yeniden hesaplamamalı | Native Files açıkken `ViewState::file_manager_trail` FmState trail/snapshot'larıyla index-aligned kolonları taşır; terminal yüzeyinde veya Files kapalıyken tamamen boştur | `compute_view_publishes_live_trail_snapshot_and_clears_it_outside_files` |
| TP-TRAIL-T7-RENDER-02 | Canlı orta panelin görsel otoritesi artık legacy parent/CURRENT/preview değildir | Üretim frame'i `file_manager_trail` satır/divider/detail rect'lerini çizer; `CURRENT`, `PREVIEW` ve `"(unavailable)"` üretmez | `production_render_consumes_exact_trail_snapshot_without_legacy_placeholders` |
| TP-TRAIL-T7-RENDER-03 | Header/action-bar/status T7.3 kapsamı dışında ve davranış olarak sabit kalmalıdır | Aynı cwd identity, named action labels ve prepared status satırı Trail gövdesinin üstünde/altında kalır | mevcut `header_actions_render_from_shared_responsive_geometry`, `action_bar_renders_selected_name_clipboard_count_and_empty_state`, status test aileleri + full suite |
| TP-TRAIL-T7-RENDER-04 | Render pure ve stale/misaligned Trail fail-closed kalır | Aynı state byte-identical buffer üretir; eksik/hizasız snapshot görünmez kolon veya placeholder icat etmez; state/snapshot değişmez | `production_trail_render_is_byte_identical_and_state_pure`; mevcut `misaligned_snapshots_project_nothing` |
| TP-TRAIL-T7-RENDER-05 | Kullanıcının onayladığı gerçek Ratatui hücreleri Chromium'da görünür yeni Trail düzenini kanıtlar | VIS-01 tek canlı kök kolonu; VIS-02 kökten `beta/deep` yoluna biriken kolonları ve exact ata vurgusunu gösterir; eski baseline mutasyonda fail olur, yalnız spec-scoped update ile yenilenir | `navigation.spec.ts` VIS-01 + `focus.spec.ts` VIS-02, ham buffer/snapshot mutation kanıtı |

## T7.4 Girdi swap test noktaları

| ID | Korunan davranış / neden | Beklenen sonuç | Test kanıtı |
|---|---|---|---|
| TP-TRAIL-T7-INPUT-01 | Render ve input aynı immutable Trail geometrisini tüketmelidir; aynı path ile kapanıp yeniden açılan Files instance eski frame'i canlandıramaz | `TrailViewSnapshot` aktif Files generation'ını taşır; generation uyuşmazsa mouse hit tüketilir ama trail, cwd, seçim ve operation intent değişmez | `stale_trail_frame_cannot_activate_reopened_files_instance` |
| TP-TRAIL-T7-INPUT-02 | Mouse row identity yalnız `(trail_index, entry_index, exact path)` üçlüsünden gelir; legacy parent/current/preview geometri otoritesi değildir | Tek tık dosyayı Trail detail seçimine taşır; klasör tek tıkta branch açar; ancestor tıkı eski alt dalı keser; index/path uyuşmazlığı fail-closed kalır | `mouse_activation_uses_exact_trail_row_and_rebranches_ancestor`; `stale_trail_row_path_is_inert` |
| TP-TRAIL-T7-INPUT-03 | Klavye ve mouse aynı `TrailState::active_col` otoritesini paylaşır (LAW 2) | Up/Down `TrailSnapshots::move_selection`; Left/Right active kolon odağı; Enter mevcut exact seçimi aynı activate seam'inde yeniden doğrular; hiçbir kol dışı cursor otoritesi trail'i ezmez | `keyboard_navigation_uses_active_trail_column` |
| TP-TRAIL-T7-INPUT-04 | Explicit bulk selection ve row operation hit'leri artık canlı Trail rect/path'lerinden türemelidir; stale legacy row alanları yetki veremez | Ctrl/Shift yalnız exact canlı Trail satırını seçer; rename/delete/send-agent Trail path'ini taşır; bulk/in-flight/read-only kontrolleri korunur; legacy row/action geometrisi değiştirilse dahi sonuç değişmez | `trail_row_hit_drives_bulk_and_operation_identity_without_legacy_geometry` |
| TP-TRAIL-T7-INPUT-05 | Sidebar Files satırı yalnız request hazırlar; App filesystem boundary'si FAVORITES/LOCATIONS hedefini fresh Trail olarak açar (LAW 5, FIP-D1) | Yetkili erişilebilir klasör için `open_trail_to(root=target,target=target)` sonucu tek kök kolon, boş seçim ve aynı Files generation içinde canlı FmState olur; inaccessible/missing/stale request mevcut trail'i atomik korur | `sidebar_navigation_opens_fresh_trail_root_and_preserves_generation`; mevcut sidebar stale/inaccessible aileleri |
| TP-TRAIL-T7-INPUT-06 | T7.6'ya kadar operations/watcher uyumluluk alanları Trail seçimiyle çelişemez | Kabul edilen Trail aktivasyonu exact sahip kolonun `cwd/entries/cursor` projeksiyonunu günceller; `selected()` Trail path'inden gelir; explicit selection yalnız live projection path'lerini içerir; `FmState` test invariants korunur | `trail_activation_reconciles_legacy_operation_projection`; characterization agent/rename/delete/multi-select aileleri |

### T7.4 kapanış kanıtı

`4cf63908` RED / `0f775b83` GREEN. Native Files mouse ve klavye girdisi,
sidebar deep-link, row actions, right-click ve bulk selection aynı
generation-bound `TrailViewSnapshot` otoritesine bağlandı. Eski Native
navigation dispatch'i, double-click state'i ve non-current Miller vertical
scroll seam'leri kaldırıldı; render/resize için hâlâ canlı Miller geometri
uyumluluğu T7.6'ya bırakıldı. Final state: Rust 3,552/3,552 + 2 skip,
Playwright Chromium 18/18, Linux ve Windows clippy `-D warnings`, Python 64/64,
Bun 5/5 + 12/12, fmt/diff/unwrap temiz. FIP-D1 ürün rotası fresh Trail açar ve
missing/inaccessible/stale hedefte atomik inert kalır.

## T7.5 Watcher + Kitty preview test noktaları

| ID | Korunan davranış / neden | Beklenen sonuç | RED kanıtı |
|---|---|---|---|
| TP-TRAIL-T7-WATCH-01 | Watcher hedefi transitional `cwd` değil, klavye/mouse ile paylaşılan aktif Trail kolonudur; boş child kolonu açıldığında ebeveyni izlemek canlı listeyi bayat bırakır | Directory branch sonrası watcher exact `trail.active_col()` dizinine bağlanır; bu dizindeki olay yalnız o snapshot'ı yeniler ve ancestor kolonları/selection'ları korur | `watcher_targets_and_refreshes_active_trail_column_without_collapsing_branch` |
| TP-TRAIL-T7-WATCH-02 | Yenileme row index değil exact path ile yakınsamaya devam etmelidir | Reorder/insert sonrası aktif kolondaki seçili path aynı kalır; silinen selection detail/descendant branch ve operation projection'dan atomik düşer; ikinci drainsiz tur generation churn üretmez | mevcut `current_watcher_refresh_reconciles_by_stable_path`, `watcher_rename_prunes_selected_path_and_keeps_cursor_safe` + GREEN focused ailesi |
| TP-TRAIL-T7-IMAGE-01 | Decode worker render edilmeyen legacy PREVIEW kolonundan boyut alamaz; canlı resim yüzeyi Trail detail panelidir | Image selection + committed `compute_view` sonrası ilk Loading target, `file_manager_trail.detail_panel.content_rect × HostCellSize` ile birebir aynıdır; panel yok/stale path/non-image durumunda iş başlamaz | `image_worker_targets_exact_trail_detail_panel_content_rect` |
| TP-TRAIL-T7-IMAGE-02 | Decode ve Kitty placement aynı typed geometriyi tüketmelidir; aksi halde hazır pixel yanlış hücrelere basılır | Ready image placement area exact Trail detail `content_rect`; target uyuşmazsa fail-closed; cache reuse/replacement/close cleanup sözleşmeleri korunur | `file_manager_ready_image_placement_uses_trail_detail_content_rect`; mevcut cache/cleanup aileleri |
| TP-TRAIL-T7-IMAGE-03 | FIP-D4 hata ve non-Kitty yollarında sessiz boşluk veya retry loop kabul etmez | Loading/Ready/typed failure aynı Trail panelinde deterministik render edilir; Kitty kapalıysa açık fallback kalır; stable failure yeni worker generation üretmez | mevcut `preview_error_states_render_without_retry_loop`, `image_preview_has_explicit_non_kitty_fallback_and_ready_content_is_clear` |
| TP-TRAIL-T7-IMAGE-04 | Görsel kabul gerçek Ratatui hücrelerinden Chromium ile kalmalıdır; Kitty byte delivery headful Ghostty kanıtına ek, onun yerine geçmez | Playwright Chromium tüm 18 baseline'ı korur; image detail panel hücre fixture'ı değişirse mutation kanıtı ve yalnız spec-scoped update; Ghostty izole reçetesiyle final canlı foto kanıtı | `cd tests/visual && npx playwright test`; T7.5 kapanışında headful kanıt durumu |

## Kabul (plan §4 ile aynı)

Kontrat 5 yasa ≥1 Rust testi + ≥1 Chromium baseline ile kanıtlı (T1-T6'da sağlandı; T7 canlı
yüzeye taşındığını kanıtlar); `"(unavailable)"` src'de grep=0; eski model kalıntısı yok;
FIP-D1 canlı kapanış; tüm gate'ler yeşil; big-bang yok (alt-adım başına yayın).

## Devam notu

Bu plan taze session'da T7.1'den başlar. Trail seam API'leri değişmeden kalmalı; değişiklik
gerekirse önce bu plana işlenir.
