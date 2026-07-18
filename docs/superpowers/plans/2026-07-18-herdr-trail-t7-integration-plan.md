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
