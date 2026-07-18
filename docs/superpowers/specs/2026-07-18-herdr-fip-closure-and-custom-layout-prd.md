# PRD — FIP Kapanışı (E2E Investigation) + Custom Layout Altyapı Programı

Tarih: 2026-07-18 · Durum: ONAY BEKLEMEZ (kullanıcı direktifiyle otonom) · Sahip: herdr FIP programı
Kaynak direktif: kullanıcı `/goal` (katmanlara ayır → araştırma/analiz → dependency chain → PRD →
task/sub-task → çalış; görsel testler MUTLAKA Playwright Chromium; git disiplini; codebase graph güncel).

Bu PRD, kapanan FIP-1..5 + FIP-6(7/8 gate) programının ÜZERİNE kalan iki iş kümesini formalize eder:
(A) FIP-6.3 E2E investigation + kapanış, (B) kullanıcı direktifi Custom Layout altyapı programı
("yazi/superfile'ı aşan file manager" birincil hedefine hizmet eden layout temeli).

---

## 1. KATMANLI DEKOMPOZİSYON (sureci featurelari katmanlara ayir)

```
L0  Kanıt/Transport katmanı     — tmux/PTY harness, SGR injection, throwaway-XDG izolasyon
L1  Client input katmanı        — src/client/input.rs framer → ClientMessage::Input
L2  Server dispatch katmanı     — server/headless.rs handle_client_input_events →
                                  App::route_client_events → handle_mouse_event_headless
L3  AppState hit/route katmanı  — input/mouse.rs arms, sidebar_tab_at, view.*_hit_areas,
                                  shell.region_hit_at (S2 region sistemi), generation gating
L4  Ürün davranışı              — Files stage activation, picker, reference delivery (KAPALI ✅)
────────────────────────────────────────────────────────────────────────────
LX  Custom Layout altyapısı     — ShellLayout/AppDock/Stage seam'leri üzerine mockup bölgeleri
                                  (TopBar/LeftPanel×2/CenterStage-tabs/RightRail/RightPanel/BottomBar)
```

## 2. DEPENDENCY CHAIN ANALİZİ

```
A-zinciri (FIP kapanışı):
  A1 canlı-tık doğrulaması (insan, 30sn)  ──┬─> A2a harness düzeltme (tık ÇALIŞIYORSA)
                                            └─> A2b P0 regresyon kök-neden (tık ÇALIŞMIYORSA)
  A2a|A2b ──> A3 E2E-01 mouse smoke GREEN ──> A4 E2E-02 PTY byte smoke ──> A5 FIP-6.3/1.6 [x]
  A5 ──> A6 FIP FINAL kapanış kaydı (registry + HANDOFF + yayın)

B-zinciri (Custom Layout — A'dan BAĞIMSIZ başlar, A5'ten önce BİTEMEZ):
  B1 brainstorm/keşif (mockup ↔ mevcut seam eşleme doğrulaması)
  B1 ──> B2 design spec (superpowers:brainstorming → writing-plans kapısı)
  B2 ──> B3 implementation plan (RED/GREEN task'lar + TP-ID kapsama)
  B3 ──> B4 katman-katman yürütme (her katman: test noktaları → RED → GREEN → görsel → yayın)
Kesişim: B4'ün görsel oracle'ı A ile aynı Playwright altyapısını kullanır (hazır, 14/14).
```

Kritik yol: **A1 insan-doğrulaması** tek dış bağımlılık (blocking=true); B1-B3 ona bağımlı değil.

## 3. ARAŞTIRMA/İNCELEME BULGULARI (bu oturumda toplanan kanıt)

| # | Bulgu | Kanıt | Güven |
|---|-------|-------|-------|
| R1 | SGR mouse byte'ları client framer'dan geçip SERVER parser'a ulaşıyor | herdr-server.log DEBUG `Mouse(MouseEvent{Down(Left),col=20,row=0})` | verified (executable) |
| R2 | Aynı oturumda klavye dispatch'i çalışıyor (C-b prefix hint overlay görünür) | tmux capture satır 21 | verified (executable) |
| R3 | handle_mouse dispatch'i sidebar-tab/new-tab/new-workspace tıklarında no-op | 3 hedefte capture değişimi yok | verified (executable) |
| R4 | Rota kodda mevcut: route_client_events → handle_mouse_event_headless → handle_mouse | app/mod.rs:1770-1776, 1912-1914 | verified (source) |
| R5 | zsh `$var` word-split ETMEZ → `tmux send-keys -H` tek arg tuzağı; `${=seq}` şart | cat -v deneyi + düzeltme sonrası R1 | verified (executable) |
| R6 | Stable socket izolasyonu korunuyor | inode/mode/mtime birebir (21223953 600 1783871657) | verified (executable) |
| H1 | Hipotez: view hit-area'ları headless'ta client-size'a göre taze değil VEYA S2 region/generation gate reddi | AÇIK — A2 investigation | open |
| H2 | Hipotez: harness ortam farkı (TERM=screen-256color, focus-event yokluğu) | AÇIK | open |

## 4. FEATURE BREAKDOWN + TASK/SUB-TASK ENUMERASYONU

### A — FIP-6.3 E2E Kapanışı
- **A1** [İNSAN, blocking] Canlı izole instance'ta tık doğrulaması (reçete: `.local/ISOLATED-DEV-TEST.md`).
- **A2** Investigation (taze session): L3'te iz sür — `sidebar_tab_at` girdi/çıktısını debug-log'la;
  `view.sidebar_tab_hit_areas` headless'ta dolu mu; `shell.region_hit_at` generation eşleşiyor mu.
  Test noktası: enjekte edilen tek Down(Left) için hit-area sorgusunun Some(Files) dönmesi —
  beklenen: Some; sebep: unit testlerle canlı arasındaki tek fark view tazeliği olabilir.
- **A3** E2E-01: Files-tab tıkı → capture'da `CURRENT` başlığı; beklenen: Stage swap; sebep: kusur #1'in gerçek-kullanıcı kanıtı.
- **A4** E2E-02: `herdr agent start e2e-cat -- cat` (launch_argv → agent-classified) → dosya satırı `>` →
  picker → Enter → `herdr agent read e2e-cat` çıktısında TAM path, `\r`/`\n` YOK; beklenen: birebir byte;
  sebep: no-submit kontratının PTY-uçtan-uca kanıtı.
- **A5** Registry kapanışı: FIP-6.3 + FIP-1.6 [x]; evidence append; continuity exact-sync.
- **A6** FF yayın + final graph reindex.

### B — Custom Layout Altyapı Programı
- **B1** Keşif: `.local/prd/custom-layout-target-mockup.md` bölge dökümü ↔ `ShellLayout/AppDock/Stage`
  seam'leri eşleme doğrulaması (cartographer: `custom-layout-SYSTEM-MAP.json` üret).
- **B2** Design spec (`docs/superpowers/specs/…custom-layout-design.md`): bölge sözleşmeleri,
  runtime/client boundary sınıflandırması (CLAUDE.md guardrail), no-goals.
- **B3** Implementation plan: RED-adları + beklenen fail'ler + GREEN seam'leri + görsel VIS-ID'leri.
- **B4** Yürütme: katman başına (test noktaları tablosu → RED commit → GREEN commit → Playwright
  baseline → gate'ler → continuity → FF push). İlk dilim: file-manager'ı zenginleştiren bölgeler
  (kullanıcı: "1. öncelik harika bir file manager").

## 5. TEST NOKTALARI POLİTİKASI (her katman için zorunlu şablon)

Her plan adımı KODDAN ÖNCE şu tabloyu üretir: | test | beklenen | neden |. Görsel doğrulama
MUTLAKA Playwright Chromium (`tests/visual/`, `--update-snapshots` yalnız YENİ baseline için,
mutation kanıtı ham buffer karşılaştırması). Rust exact-cell testleri semantik otorite.

## 6. KABUL KRİTERLERİ

- A: E2E-01/02 GREEN + sıfır residue + stable socket birebir + registry 0 açık FIP maddesi.
- B: mockup'taki her bölge ya çalışan seam'e bağlı ya da açıkça no-goal; her katman görsel baseline'lı;
  full suite + iki Clippy + görsel suite yeşil; big-bang yok (katman başına FF yayın).
- Genel: arkada fail test YOK (bugün: 3,494/3,494 + 14/14), continuity exact-sync, çıplak iddia YOK.
