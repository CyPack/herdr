# Miller Trail Programı — Katmanlar, Dependency Chain, PRD, Task Kırılımı

Tarih: 2026-07-18 · Girdi yasası: `docs/superpowers/specs/2026-07-18-herdr-miller-trail-ux-contract.md`
Referans implementasyon: circet-miller (`CircetMillerSection.tsx`, TrailCol/selectAt/auto-scroll desenleri).
Görsel oracle: Playwright Chromium (`tests/visual/`, mevcut 14/14 altyapı). Yayın: CyPack FF, RED/GREEN atomic.

## 1. KATMANLAR (dependency sırasıyla)

```
T1  Trail çekirdeği (saf veri)   — TrailState { cols: Vec<TrailCol> }, truncate+rebranch,
                                   dosya-tık-kolon-eklemez; PTY'siz test edilir (AppState saf-data ilkesi)
T2  Snapshot köprüsü             — her TrailCol ↔ read_directory_snapshot; watcher reconcile;
                                   bounded (≤32 derinlik); "(unavailable)" YOK — kolon = yüklü snapshot
T3  Render + geometri            — kolonlar soldan sağa, per-index genişlik, en-derin-kolon
                                   auto-scroll penceresi; hit-area'lar render'la tek kaynaktan
T4  Girdi                        — klasör-tık dallanma, ata-kardeş-tık kes+dallan, dosya-tık
                                   detay paneli, klavye eşdeğerleri; generation-safe hit'ler
T5  Detay/önizleme paneli        — resizable sağ panel; meta + mevcut text/image preview
                                   entegrasyonu; FIP-D4 (kitty graphics Ghostty) ayrı iz
T6  Sidebar → trail              — FAVORITES/LOCATIONS tıkı trail'i kökten kurar (FIP-D1'i
                                   kökten çözer); deep-link deseni
T7  Kapanış                      — VIS-07..10 baseline'ları, E2E, eski model kodunun
                                   characterized sökümü, gate'ler, yayın
```

Zincir: T1 → T2 → (T3 ∥ T4 birbirine kilitli) → T5 → T6 → T7. FIP-D4 T5'e paralel bağımsız iz.

## 2. TEST NOKTALARI (plan-anı; her katman kendi RED'inden önce tabloyu detaylandırır)

| Katman | Test | Beklenen | Neden |
|---|---|---|---|
| T1 | `folder_select_truncates_and_branches_trail` | trail[0..=i]+yeni kolon | Kontrat YASA-1 çekirdeği |
| T1 | `file_select_never_appends_a_column` | kolon sayısı sabit, selected işaretli | Dosya=panel, kolon değil |
| T1 | `ancestor_sibling_select_rebranches` | eski alt-dallar atılır | Kes+dallan; "geri" yok |
| T1 | `trail_depth_stays_bounded` | ≤32, en eski kök korunur | Bounded ilkesi |
| T2 | `every_visible_column_is_loaded` | hiçbir kolon içeriksiz değil | "(unavailable)" imkânsızlığı |
| T2 | `watcher_refresh_keeps_selection_by_path` | path ile yeniden çözüm | FIP-2'de kurulan exact-path ilkesi |
| T3 | `deepest_column_scrolls_into_view` | pencere en sağı gösterir | YASA-2 |
| T3/VIS | VIS-07 trail 4-derinlik, VIS-08 dallanma-sonrası | Chromium baseline | Görsel kanıt zorunlu |
| T4 | tık aileleri (yukarıdakilerin input-uçtan) | state geçişleri birebir | Girdi=tek otorite |
| T5 | `file_click_opens_resizable_detail_panel` | panel açık, kolonlar korunur | YASA-3 |
| T6 | `sidebar_favorite_builds_trail_from_root` | trail=ancestor zinciri | FIP-D1 kabul kriteri |

## 3. TASK KIRILIMI

- T1.1 RED `test: pin trail core transitions` → T1.2 GREEN `feat: add miller trail core`
- T2.1 RED snapshot köprüsü → T2.2 GREEN; T2.3 watcher characterization
- T3.1 geometri RED/GREEN + VIS-07/08 baseline commit
- T4.1 input RED/GREEN (mouse+klavye)
- T5.1 panel RED/GREEN + VIS-09; FIP-D4 izi ayrı task
- T6.1 sidebar RED/GREEN + VIS-10 (FIP-D1 kapanır)
- T7.1 eski parent/current/resident modelinin characterized sökümü (tek commit, dead-code'suz)
  + full gate'ler + continuity + FF yayın

## 4. KABUL

Kontrattaki 5 yasanın her biri en az bir Rust testi + bir Chromium baseline ile kanıtlı;
"(unavailable)" string'i src/'de grep=0; full suite + iki Clippy + görsel suite yeşil;
eski model kalıntısı yok; FIP-D1 kapalı.
