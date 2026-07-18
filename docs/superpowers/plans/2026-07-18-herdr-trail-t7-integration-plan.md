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

## Kabul (plan §4 ile aynı)

Kontrat 5 yasa ≥1 Rust testi + ≥1 Chromium baseline ile kanıtlı (T1-T6'da sağlandı; T7 canlı
yüzeye taşındığını kanıtlar); `"(unavailable)"` src'de grep=0; eski model kalıntısı yok;
FIP-D1 canlı kapanış; tüm gate'ler yeşil; big-bang yok (alt-adım başına yayın).

## Devam notu

Bu plan taze session'da T7.1'den başlar. Trail seam API'leri değişmeden kalmalı; değişiklik
gerekirse önce bu plana işlenir.
