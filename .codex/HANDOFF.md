# SESSION HANDOFF — Herdr Files Interaction Polish

Updated: 2026-07-18 CEST

## 0. SONRAKI ADIM — TEK AKTİF İŞ

FIP-G.1 ve FIP-G.2 KAPANDI (2026-07-18). Onaylı code-level TDD planı:
`docs/superpowers/plans/2026-07-18-herdr-files-interaction-polish-implementation.md`
(commit `dd81ef59`; 29 görev; 57 benzersiz `TP-FIP-*` ID eşlendi — eski "55"
sayımı iki E2E ID'sini dışlıyordu, hiçbir ID atlanmadı).

FIP-0 fazı (Task 1-5) 2026-07-18 oturumunda KAPANDI: baseline 3,443/3,443 (run df00a924), exporter, Playwright 1.54.1+Chromium 139 harness, 7/7 görsel self-test, mutation kanıtı. FIP-1 Rust+görsel tarafı KAPANDI (2026-07-18): NAV-01..04/08 RED/GREEN
zinciri (`3680c09b`→`17487a7b`→`584ef59e`→`7e736472`→`0fc16895`→modifier-gate
fix→VIS-01 `2dcfffa4`); ctrl-click activation sızıntısı bulunup kapatıldı; full
suite 3,454/3,454 + 1 skip; görsel suite 9/9. `TP-FIP-E2E-01` FIP-6.3 final
koşumuna açıkça devredildi. FIP-2 çekirdeği KAPANDI (2026-07-18): FOCUS-01 bind (`7b435ae2`/`7cd51a39`),
FOCUS-03/04/10 re-resolution (`e74c8954`/`f75d1e48`), VIS-02 görsel kanıt
(resident kolonda nonzero `beta` highlight; görsel suite 10/10); full suite
3,456/3,456 + 2 skip (write_visual_fixtures ignored dahil). FIP-2.5 de KAPANDI: FOCUS-02/05/06 characterization (`2df663c2`), FOCUS-07
viewport clamp RED/GREEN (`2df663c2`/`9233048a`); FOCUS-08 unbound→None resolver
testiyle, FOCUS-09 mevcut FM3 stale-generation aileleriyle korunuyor. FIP-2
TAMAMEN KAPALI. FIP-3.1/3.2/3.3 KAPANDI (`4663d42d` characterization, `9c9b804a` canonical
FileEntryKind: snapshot symlink kimliğini koruyarak 6 kind hazırlıyor; bridge
alanları kind türevleriyle tutarlılığı test ediliyor; full 3,463/3,463 + iki
Clippy hedefi temiz). FIP-3.5/3.6 de KAPANDI (`fa2bc768` RED / `34d73460` GREEN): pure visual_class
(17 sınıf, exact-name > lowercase-ext, kind her zaman kazanır) + Nerd/ASCII
tek-hücre glyph tabloları (ASCII benzersizliği testli) + `render_entry_row`
artık her satıra semantik ikon basıyor; full 3,469/3,469, iki Clippy hedefi
temiz, görsel suite ikonlu baseline'larla 10/10. NOT: Nerd PUA glyph'leri
browser fontunda GÖRÜNMEZ (deterministik boş hücre) — VIS-03/04 ASCII-profil
fixture'ları için render profil seçimi (AppState client-local alanı) FIP-3.7/
3.8'de eklenecek; Rust exact-cell testleri semantik otorite. Kalan FIP-3:
3.4 consumer migration, 3.7 edge ailesi (ICON-08/09/11/13 + control escaping),
3.8 VIS-03/04 (ASCII profil). FIP-4.1/4.2 de KAPANDI (`98d2df6b` RED / no-submit GREEN): FM agent handoff
payload artık TAM path bytes — `\r` kaldırıldı, eski path+Enter kontratını
pinleyen test onaylı REF-05/07 kontratına yükseltildi; full 3,469/3,469.
NOT: M1 attachment picker'ın kendi path+CR delivery'si (satır ~18) FIP kapsamı
DIŞI ve bilinçli korunuyor; Claude-split gönderimi (satır ~341) FIP-4.5/4.6'da
makinesiyle birlikte kalkacak. FIP-4 TAMAMEN KAPANDI (2026-07-18):
4.3/4.4 (`ec9bed86` RED / `8371b5d3` GREEN — reference_path_is_deliverable
validator'ı prepare + delivery seam'lerinde: silinen path, FIFO'ya dönüşen
kind ve control-char path sıfır byte + tek failure; directory referansı ve
noktalama byte-for-byte characterize edildi); 4.5/4.6 (`b172bd74` RED /
`05866164` GREEN — non-agent focus artık HİÇBİR şey hazırlamıyor, implicit
Claude split makinesi TAMAMEN silindi: −594 satır, FileManagerClaudeSplitRequest
tipi + state alanı + launch/complete/rollback/sync + eski-davranış testleri;
M1 attachment picker ve normal pane split'leri korunuyor); 4.7 (`94a4cd96` —
REF-08/09/10 vanished-workspace/changed-terminal/exactly-once ailesi, ürün
değişikliği gerekmedi). FIP-5.1/5.2 de KAPANDI (`b515e293` RED / `2a3a7946`
GREEN): AgentReferencePicker modeli + blocking Mode::AgentReferencePicker +
AgentReferenceRequest (path+files_generation+workspace+pane+terminal) tipiyle
genişletilen delivery slotu; reference action artık canlı agents
projeksiyonundan picker açıyor (focused agent ilk + preselected), explicit
aktivasyon tam kimlik snapshot'lıyor, send seam FOCUSED değil SEÇİLEN pane
binding'ini doğruluyor; eski focus-türevi testler yeni kontrata yeniden
yazıldı. FIP-5 TAMAMEN KAPANDI (2026-07-18): 5.3/5.4 (`a9a356fe` RED /
`507a588d` GREEN — panel-shell reuse render, Up/Down/j/k + Enter + row-click
tek seçim otoritesi, outside-click sıfır byte kapanış, arka plan fail-closed;
saf AppState geometri helper'ları render+hit-test'i tek kaynakta tutuyor);
5.5/5.6 (`c89140bb` RED / `d42334e1` GREEN — frame başına liveness recompute,
vanished-target aktivasyonu görünür failure + sıfır byte); 5.7 (`233ec07d` —
"Send to Agent" → "Add Reference to Agent..." tüm literaller, grep 0);
5.8 (`51d045c9` — VIS-05/06 baseline'ları: picker 120x40 + disabled-row tiny
60x20, görsel suite 14/14). Full 3,494/3,494 + 2 skip; iki Clippy temiz.
FIP-6 BÜYÜK ORANDA KAPANDI (2026-07-18): 6.1 focused aileler
(11+9+17+35+20, hepsi non-zero), 6.2 taze görsel 14/14, 6.4 tüm gate'ler
(full 3,494 + fmt + iki Clippy + python 64 + bun 5/5+12/12 + diff-check +
unwrap-scan temiz), 6.5 purity (windowed byte-identical + no-fs-io), 6.6
reindex + snippet doğrulama, 6.7/6.8 continuity + FF yayın. TEK AÇIK: 6.3
E2E-01/02 — izole tmux harness'ta SGR mouse SERVER PARSER'A ULAŞIYOR
(herdr-server.log DEBUG Mouse events kanıtlı) ama handle_mouse dispatch
sidebar-tab/new-tab/new-workspace için NO-OP; klavye (C-b prefix hint)
çalışıyor. Olası server-modu mouse regresyonu VEYA harness ortam farkı —
taze session'da adanmış investigation gerekli (.codex/evidence/
fip-progress.md son bölüm). zsh tuzağı: tmux send-keys -H'e ${=seq}.
Stable socket inode/mode/mtime birebir korundu. SONRAKİ BÜYÜK PROGRAM
AKTİF: MILLER TRAIL (2026-07-18) — kullanıcı kanonik referansı circet-miller
verdi; kontrat: docs/superpowers/specs/2026-07-18-herdr-miller-trail-ux-
contract.md (5 yasa), plan: docs/superpowers/plans/2026-07-18-herdr-miller-
trail-program.md (T1-T7). T1 KAPANDI (`3b0c2ed0`/`7d5edecb`). T2 KAPANDI
(`12a53be4` RED / `59cdb470` GREEN — fm::trail_snapshots::TrailSnapshots:
index+path hizalı sync, fail-closed select_dir, refresh_col path-bazlı
seçim koruması, sliding-window hizalama; aile 5/5, full 3,505/3,505 + 2
skip, iki clippy temiz). T3 KAPANDI (`0c6b0d87` RED / `1982e20e` GREEN —
ui::file_manager::trail_view: project_trail_view + render_trail_view,
deepest auto-scroll, per-index genişlik, ata-kolon seçim vurgusu, stale
hizasızlık inert; aile 7/7, VIS-07/08 baseline, görsel suite 16/16, full
3,512/3,512 + 2 skip). T4 KAPANDI (`42c95ee8` RED / `81ad452a` GREEN —
trail_row_at hit otoritesi, activate_entry generation-safe branch/mark,
move_selection klavyesi, TrailState.active_col LAW-2 odak; full
3,521/3,521 + 2 skip). T5 KAPANDI (`33c12968` RED / `55e12fea` GREEN —
TrailDetail activate-anı hazırlık, Text/Image/Unpreviewable açık durumlar,
klasör aktivasyonu paneli kapatır, sağ klamplı side-panel + bordered
render; VIS-09 baseline, görsel 17/17, full 3,528/3,528 + 2 skip). Saha
kusurları: FIP-D2 kapalı (agentless toast), FIP-D3 trail kontratıyla
süperseed, FIP-D4 (kitty pixel delivery) AÇIK iz. T6 KAPANDI (`0a9189fc`
RED / `c8e5dd4d` GREEN — open_trail_to deep-link kurucusu: fail-closed
kök, ancestor zinciri, kök-dışı fallback, dürüst kısmi iniş; VIS-10
baseline, görsel 18/18, full 3,534/3,534 + 2 skip). FIP-D1'in ÜRÜN-düzeyi
kapanışı T7 entegrasyonunda (canlı sidebar tıkı bu seam'e bağlanınca).
TRAIL-T7.1 KAPANDI (`7d75f0e4`): 5 satırlık characterization test-noktası
tablosu; adversarial AppState altında Files aç/kapa/yeniden-aç Stage
generation lifecycle testi; agent-reference, rename/delete/multi-select ve
watcher exact-path otoriteleri kanonik ID'lerle pinlendi; legacy
parent/current/preview/resident testleri `TRAIL-T7.6 teardown` olarak
işaretlendi ama silinmedi. Taze gate: full 3,535/3,535 + 2 skip, Chromium
18/18, Linux+Windows clippy, Python 64/64, Bun 5/5 + 12/12, fmt temiz.
TRAIL-T7.2 KAPANDI (`62696987` RED / `19efb656` GREEN): FmState
TrailState+TrailSnapshots sahibi; `selected()`/operations/agent handoff
exact-path otoritesi aligned trail snapshot'tan türetilir; enter/leave root'u
koruyarak kolon biriktirir; prepared refresh/navigation apply disk-I/O'suz;
hidden policy future branch'lere taşınır. Taze gate: full 3,541/3,541 + 2
skip, Chromium 18/18, Linux+Windows clippy, Python 64/64, Bun 5/5 + 12/12,
fmt temiz. TRAIL-T7.3 KAPANDI (`e63482f2` RED / `4d95ae72` GREEN):
`ViewState::file_manager_trail` canlı frame'in exact geometri otoritesi;
üretim `render_file_manager` orta paneli root-to-active Trail + detail paneli
çizer, legacy component karakterizasyonu T7.6'ya kadar izole kalır.
Dar-detail `clamp(min>max)` panic'i yeni regression testiyle kapandı; image
Pending/Loading/Unavailable durumları Trail detail panelinde görünür kalır.
VIS-01..06 kasıtlı Trail baseline değişimleri ham mutation fail'i + gözle
inceleme + spec-scoped update ile onaylandı. Taze gate: full 3,545/3,545 + 2
skip, Chromium 18/18, Linux+Windows clippy, Python 64/64, Bun 5/5 + 12/12,
fmt/diff/unwrap taraması temiz. TRAIL-T7.4 + FIP-D1 KAPANDI (`4cf63908` RED /
`0f775b83` GREEN): generation-bound Trail frame mouse/klavye/sidebar,
row-action ve right-click otoritesi; ancestor rebranch, exact path/index,
stale-frame ve atomic bulk tavanı; Native Files legacy navigation,
double-click ve non-current scroll seam'leri kaldırıldı. Taze gate: full
3,552/3,552 + 2 skip, Chromium 18/18, Linux+Windows clippy `-D warnings`,
Python 64/64, Bun 5/5 + 12/12, fmt/diff/unwrap temiz. TRAIL-T7.5 +
FIP-D4 ürün kodu KAPANDI (`8a3a944b` RED / `95f6e541` GREEN): watcher exact
aktif Trail kolonuna bağlandı ve yalnız aynı snapshot'ı path-bazlı yeniler;
decode worker ile Kitty placement aynı generation-bound Trail detail
`content_rect` otoritesini tüketir; stale path/geometri ve legacy PREVIEW
fail-closed kalır; cache/resize/error aileleri korundu. Taze gate: focused
3/3, watcher/image/Kitty 57/57, full 3,555/3,555 + 2 skip, Chromium 18/18,
Linux+Windows clippy `-D warnings`, Python 64/64, Bun 5/5 + 12/12,
fmt/diff/unwrap temiz. Ghostty headful canlı foto kabul kanıtı izole reçetede
kullanıcıyla ayrıca bekliyor. TRAIL-T7.6 + FIP-D3 KAPANDI (`e8abc7b0` RED /
`3c36f104` GREEN): parent/current/resident projection, legacy watcher/image
seam'leri ve geçiş test mezarlığı kaldırıldı; Trail topolojisi path-kimli
layout preferences ve typed resize otoritesinin tek kaynağı; detail sınırı
mouse/keyboard/commit/render boyunca 20–64; source audit 4/4, exact
`"(unavailable)"` ve teardown marker grep=0. Final gate: full 3,507/3,507 +
2 skip, Chromium 18/18, Linux+Windows clippy `-D warnings`, maintenance
68/68, Bun 5/5 + 12/12, fmt/diff temiz. Miller Trail T1-T7 PROGRAMI
TAMAMEN KAPALI. KULLANICI SAHA DÜZELTMESİ TRAIL-T7.7 KAPANDI (`06d24f3e`
RED / `35c1393c` GREEN; baseline uzlaştırması `2d2c231f`): canlı Trail yatay
origin'i artık render tarafından tüketiliyor; Shift+wheel/native yatay wheel
ile sola açılan ata kolonları sonraki frame'de geri sıçramıyor. Yeni
branch/klasör ve responsive resize `follow_active` ile aktif uca otomatik
takibi yeniden kuruyor; render tek başına manuel moda geçmiyor. VIS-11
60x20 dar viewport baseline'ı eklendi. Taze gate: focused 1/1, Trail 76/76,
full 3,508/3,508 + 2 skip, Chromium 19/19, Linux+Windows clippy
`-D warnings`, maintenance 68/68, Bun 5/5 + 12/12, fmt/diff/production
unwrap taraması temiz. `.local/herdr-trail-test.sh` manuel akışı yalnız
test-sahipli server/root için başlangıç ve kapanış semantik temizliği yapar;
stable Herdr/socket ve kullanıcı süreçlerine dokunmaz. TRAIL-T7.8 KAPANDI
(`4e6e922b` RED / `febe65ef` GREEN / `97d5fe01` VIS-12 / `26da2437`
fixture-alignment ve yayın): mutable `first_visible` otoritesi mutlak
`offset_cells` ile değişti; wheel yön-duyarlı kolon genişliğinin üçte biri
kadar ilerler; clipped kolon/row/action geometri, Unicode display-cell dilimi
ve generation+revision-bound Trail snapshot render/input'ın tek kaynağıdır.
Taze gate: full Rust 3,512/3,512 + 2 skip, Chromium 20/20 ve tek-hücre
mutation 15-pixel kırmızı, Linux+Windows clippy, maintenance 68/68, Bun
5/5 + 12/12, fmt/diff/unwrap/source audit temiz. Tek-worker graph
21,296/98,085 ready; CyPack iki ref `26da2437` ile birebir. TRAIL-T7.9
modifier'sız wheel canlı-terminal fallback'i KAPANDI (`a63e39e7`
plan / `1ca992c6` RED / `051f2829` GREEN): izole debug logunda 318
modifier'sız `ScrollUp/ScrollDown`, sıfır native yatay/Shift olayı görüldü.
Görünür satır düz wheel dikey seçim otoritesini korur; boş canlı Trail kolon
gövdesindeki aynı olay mevcut 1/3-kolon yatay reducer'ına düşer.
Detail/header/outside/stale fail-closed kalır. Focused aileler
1/1+4/4+3/3+2/2+1/1, full Nextest exit 0 ve 3,513 test envanteri, Chromium
20/20, iki Clippy, Python 68/68, Bun 5/5+12/12, fmt/diff temiz; graph
21,304/98,123 ready. Sıradaki bağımsız işler: FIP-6.3 E2E harness investigation
ve kullanıcı önceliğine göre custom-layout B-zinciri. FIP-3 TAMAMEN KAPANDI (2026-07-18): 3.4 characterized migration
(`bcecfdc8` — FileEntry alanları kind-türevi metodlara döndü, çifte symlink
stat kalktı, 3-kategori grep 0 kalıntı); 3.7 icon edge ailesi (`91e33f6f` RED
/ `706130cf` GREEN — display-cell truncation, disjoint action rect'leri,
cursor>class hiyerarşisi, render saflığı, control-char `display_name()`
escape'i C0→Control Picture); 3.8 ASCII görsel kanıt (`bcea05f6` RED /
`d4b8514a` GREEN — client-local `file_icon_profile` AppState alanı; `c10cfffe`
VIS-03/04 mixed-kind 120x40 + tiny 60x16 baseline'ları, 10 kind ASCII glyph
browser'da görünür doğrulandı). Full 3,476/3,476 + 2 skip; görsel suite 12/12;
iki Clippy hedefi temiz.
KULLANICI DİREKTİFİ (2026-07-18): FIP-2 kapandıktan sonra, Excalidraw
mockup'ındaki zengin layout'u kolay/hızlı/production-grade kurmayı sağlayan
CUSTOM LAYOUT ALTYAPISI programı tasarlanacak (kendi brainstorm→design→plan
kapısıyla). 1. öncelik: yazi/superfile'ı aşan file manager. Mockup dökümü:
`.local/prd/custom-layout-target-mockup.md`; temel:
`docs/superpowers/specs/2026-07-17-herdr-custom-layout-architecture-guide.md`.
FIP-3..6 kuyruğu silinmedi; sıralama kararı bu direktifle güncellendi. Rust implementasyonu başlamadan önce kırık global `rust-dev`
skill symlink'i (`~/.codex/skills/rust-dev` → eksik `~/.claude/skills/rust-dev`)
onarılmalı; herdr-lokal HP1-HP10 kataloğu mevcut ve önceliklidir.

Bu sıra tartışmaya açık bir öneri değil, kanonik görev önceliğidir.
Change-pipeline T3.1-T10.9 paused; S5/S7 trigger-gated; Apps/Desktop ve
drag-and-drop kapsam dışı.

## 1. DURUM ÖZETİ

- Repo: `/home/ayaz/projects/herdr`
- Branch: `feat/native-fm`
- Yerel başlangıç HEAD’i: `f097f6c7`
- Yerel tasarım zinciri:
  - `fc76f648 docs: design files interaction polish`
  - `f097f6c7 docs: record canonical graph project ids`
- Bu handoff turu başında CyPack `feat/native-fm` ve `master`:
  `b7d4217c441c0cf842e5775ff2556d641c5a7940`
- User-owned, untracked alan: `.superpowers/`
- `.superpowers/` hiçbir koşulda stage/edit/delete edilmez.
- Eski SF0-SF6 + FM1-FM5 programı tamam ve yayınlıdır; yeniden uygulanmaz.
- Yeni aktif program Files Interaction Polish’tir.
- Açık görev envanteri (FIP-4 kapanışı sonrası, 2026-07-18):
  - `.codex/TASKS.md`: 16
  - `.codex/CHANGE-PIPELINE-TASKS.md`: 89
  - toplam: 105
- Sadece FIP-6.3 (E2E investigation) veya FIP-1.6 kapanışı in-progress yapılabilir; diğer görevler pending/paused kalır.
- Fresh continuity gates (2026-07-18 planning-gate closure):
  - exact task copy 96/96 (FIP-6 kapanış koşusu sonrası);
  - 57 unique `TP-FIP-*` (fresh deterministic count; the earlier "55" excluded
    the two E2E IDs — all 57 are mapped in the implementation plan);
  - Nextest run `4da2ee18-b784-4c38-aaab-98a2e8787511`,
    3,443/3,443 passed + 1 named skip;
  - Linux all-target and Windows MSVC Clippy clean;
  - Bun 5/5 + 12/12;
  - Python maintenance 64/64;
  - fmt, Markdown references and `git diff --check` clean.

## 2. ÜRÜN VİZYONU VE SINIRLAR

Bu program yeni bir genel UI framework’ü kurmaz. Mevcut typed Stage, bounded
Miller, generation-safe hit, terminal identity ve popup seams üzerine dört
gözle görülür kusuru production-grade biçimde düzeltir:

1. Default sidebar’daki Files primary click’i Native Files Stage’i açar;
   Spaces/Projects Terminal Stage’e client-local ve runtime kimliğini bozmadan
   döner.
2. Bir klasöre girildiğinde resident/önceki kolon row zero’ı değil, girilen
   exact child path’i highlight eder.
3. Directory, regular file, symlink-directory, symlink-file, broken symlink ve
   unsupported special entry semantik olarak ayrılır; ortak dosya sınıfları
   için deterministic icon classification ve Nerd/ASCII tek-cell profilleri
   vardır.
4. `Send to Agent` yerine `Add Reference to Agent...` görünür; mevcut Agents
   projection’ından explicit target seçilir ve file/directory path’i chat
   terminaline yalnızca metin olarak eklenir.

Mutlak no-submit sözleşmesi:

- payload tam olarak güvenli UTF-8 `path.as_bytes()` olur;
- CR, LF, Enter, prefix, suffix, shell quoting veya implicit whitespace yoktur;
- otomatik submit yoktur;
- implicit Claude split/chat oluşturulmaz;
- delivery yalnız hâlâ aynı workspace/pane/terminal/agent kimliğine yapılır;
- source path son seam’de yeniden doğrulanır;
- control/non-UTF-8, broken/special, stale identity, vanished path ve
  backpressure sıfır byte ile fail closed olur;
- retry queue/hot retry yoktur.

Drag-and-drop, speculative ComponentRegistry, yeni popup framework, server
protocol genişletme, persistence migration, Apps/Desktop ve unrelated shell
redesign açıkça kapsam dışıdır.

## 3. ZORUNLU BAŞLATMA SIRASI

Fresh agent aşağıdaki sırayı atlamadan uygular:

1. `/home/ayaz/projects/herdr/AGENTS.md` dosyasını tamamen oku.
2. `/home/ayaz/projects/herdr/CLAUDE.md` dosyasını tamamen oku.
3. `$herdr-native-fm` skill’ini kullan; skill çalıştırmadan önce:
   - `.codex/skills/herdr-native-fm/lessons/errors.md`
   - `.codex/skills/herdr-native-fm/lessons/golden-paths.md`
   - `.codex/skills/herdr-native-fm/lessons/edge-cases.md`
   - `/home/ayaz/.codex/skills/_shared/common-errors.md`
4. Şu dosyaları sırayla ve tamamen oku:
   - `.codex/BOOTSTRAP.md`
   - `.codex/CURRENT.md`
   - `.codex/TASKS.md`
   - `.codex/CHANGE-PIPELINE-TASKS.md`
   - `.codex/HANDOFF.md`
   - `.codex/MEMORY.md`
   - `.planning/STATE.md`
   - `.codex/NEXT-SESSION-PROMPT.md`
5. Tasarım kaynağını tamamen oku:
   - `docs/superpowers/specs/2026-07-17-herdr-files-interaction-polish-design.md`
6. Git/remote gerçeğini doğrula:
   - `git status --short --branch`
   - `git log --oneline --decorate -12`
   - `git remote -v`
   - `git ls-remote origin refs/heads/feat/native-fm refs/heads/master`
7. Codebase Memory’yi graph-first kullan:
   - canonical project: `home-ayaz-projects-herdr`
   - `get_architecture`
   - `search_graph`
   - `trace_path`
   - `get_code_snippet`
   - gerekirse `query_graph`
   - code discovery için grep ancak graph yetersizse
8. `.codex/TASKS.md` ve `.codex/CHANGE-PIPELINE-TASKS.md` içindeki her
   unchecked maddeyi continuation satırlarıyla yeniden say ve session task
   listesine eksiksiz aktar. Beklenen 54 + 89 = 143’tür. Sayı farklıysa kod
   yazmadan CURRENT/TASKS/HANDOFF drift’ini uzlaştır.
9. FIP-G.1 için `superpowers:writing-plans` ve lessons dosyalarını yükle.
10. Rust işi başlayacağı zaman ayrıca `rust-dev` ve lessons dosyalarını yükle;
    skill erişilemiyorsa erişilmiş gibi davranma.
11. Her bug/feature için production kodundan önce behavior-specific failing
    test; RED çıktısını oku; ayrı commit. Sonra minimum GREEN; ayrı commit.
12. Runtime/Chromium/PTY işi başlamadan
    `.local/ISOLATED-DEV-TEST.md` dosyasını tamamen oku ve throwaway
    XDG/socket kontratını uygula.

## 4. CODEBASE MCP — KANONİK HARİTA VE KÖK NEDEN

Kanonik Codebase Memory project ID:
`home-ayaz-projects-herdr`.

Final handoff refresh; single-worker CLI ve built-in MCP aynı store’u doğrular:

- nodes: 21,064
- edges: 98,009
- ana paketler: `app` 2,615; `ui` 968; `pane` 715; `fm` 457;
  `server` 407; `cli` 406; `terminal` 298; `workspace` 236.

Graph-first denetimle kanıtlanan mevcut kusur zincirleri:

- Default `ShellLayout::default` yalnız LeftPanel + WorkspaceStage üretir;
  default görünür shell’de AppDock yoktur. Sidebar tab click route yalnız
  `sidebar_tab` değiştirir; Stage activation yapmaz. Buna karşılık mevcut
  `activate_dock_app(Files)` seam’i bounded Files Stage’i açar. FIP-1 bu
  davranışları tek authority’de yakınsar.
- `MillerPathSegment.focused_child` tanımlanır fakat üretim akışında
  doldurulmaz. Resident projection `segment.cursor` kullanır; yeni segment
  cursor default 0 olduğu için nonzero child’dan sonra yanlış ilk satır
  highlight edilir. FIP-2 path identity’yi transfer öncesi bağlar.
- `read_directory_snapshot` / entry capability hazırlığı symlink kimliğini
  `(is_dir, operation_supported)` ikilisine indirger. `render_entry_row`
  semantik icon üretmez. FIP-3 tek canonical `FileEntryKind` ve derived
  capability modeline geçer.
- Mevcut agent handoff final payload’a `b'\r'` ekler; non-agent hedef hazırlığı
  implicit Claude split oluşturabilir. Existing `agent_panel_entries`,
  terminal identity lookup ve bounded `try_send_terminal_input` target
  projection/delivery için yeniden kullanılabilir. FIP-4/FIP-5 submit ve
  implicit split davranışını bu action’dan çıkarır.

Bu maddeler chat özeti değil, tasarım dosyasındaki symbol/call-path evidence’ın
özetidir. Planlama sırasında exact qualified names ve snippets tekrar okunur;
stale `ready` çıktısı freshness kanıtı sayılmaz.

Post-refresh MCP proof ayrıca FIP-G.1’i `.codex/BOOTSTRAP.md`,
`.codex/CURRENT.md`, `.codex/HANDOFF.md`, `.codex/MEMORY.md`,
`.codex/NEXT-SESSION-PROMPT.md` ve `.codex/TASKS.md` içinde buldu;
`focused_child` yalnız field/constructor sonuçlarında, mevcut
`sync_file_manager_agent_handoff_send` ise çağıranı ve fail-closed test
ailesiyle birlikte güncel graph’ta bulundu.

## 5. KATMANLAR VE OTORİTE SINIRLARI

Layer 0 — Semantic identities:

- exact path identity;
- Miller focused-child identity;
- `FileEntryKind`;
- workspace/pane/terminal/agent target snapshot.

Layer 1 — Filesystem preparation:

- metadata/kind classification transition/reload aşamasında;
- render sırasında filesystem I/O yok;
- symlink ve special entry truthfulness;
- control-character display escaping.

Layer 2 — Pure visual classification:

- exact-name override;
- case-insensitive extension class;
- deterministic generic fallback;
- Nerd ve ASCII tek display-cell token;
- ürün binary’sine yeni runtime dependency yok.

Layer 3 — Projection/render:

- pure `compute_view`/`render` contract;
- exact path selection/highlight;
- generation-scoped typed hits;
- tiny/narrow geometry;
- row action ile icon/name overlap yok.

Layer 4 — Input/controller:

- overlay/capture > typed hit > background precedence;
- primary click semantics;
- stale hit inert;
- picker keyboard/mouse/outside/Escape ownership;
- multi-selection reference action disabled.

Layer 5 — Runtime adapter:

- one bounded terminal-input attempt;
- last-seam target and path validation;
- exact bytes, no submit;
- zero implicit resource creation;
- zero hot retry.

Layer 6 — Verification/observability:

- Rust semantic tests;
- deterministic Ratatui TestBackend cell fixtures;
- Playwright Chromium screenshots;
- isolated real mouse;
- PTY exact-byte capture;
- performance/resource/invariant/full-gate evidence.

## 6. TEST-FIRST KAPISI

Her implementasyon dilimi başlamadan önce test planı şu alanları açıkça
belirtir:

- test ID ve exact test name;
- bug/feature’ın mevcut davranışı;
- beklenen RED failure ve neden gerçek davranış failure’ı olduğu;
- minimum GREEN authority seam’i;
- non-happy-path matrisi;
- regression family;
- focused command;
- broad command;
- platform/visual/runtime command;
- expected pass/fail/skip count;
- artifact ve cleanup sonucu;
- RED/GREEN/refactor commit subject;
- rollback sınırı.

Oracle önceliği:

1. pure Rust state/model/input tests;
2. Ratatui TestBackend exact-cell tests;
3. Playwright Chromium deterministic cell-grid screenshots;
4. isolated terminal mouse smoke;
5. PTY byte capture;
6. full repository/platform gates.

Playwright gerçek TUI semantiğini icat etmez. TestBackend’in exact cell buffer’ı
browser grid fixture’ına aktarılır. ASCII profile canonical cross-machine
baseline’dir; Nerd mapping Rust cell testleriyle ayrıca doğrulanır. Browser
yoksa gate açıkça fail olur; skip-success yoktur. Tek cell mutation snapshot’ı
fail ettirmelidir.

Kritik edge families:

- modified/middle/release-only/outside mouse;
- overlay/capture background blocking;
- collapsed/tiny sidebar;
- stale generation/hit;
- exact child at nonzero index;
- four-plus-level chain;
- reorder/delete/hide/branch-retirement;
- duplicate/malformed path;
- empty/root/permission/unavailable directory;
- symlink-file/symlink-dir/broken link;
- FIFO/socket/device/metadata failure;
- mixed-case extensions, dotfiles, exact well-known names;
- long ASCII/Unicode/control-containing filenames;
- narrow display-cell truncation;
- target disappears or terminal identity changes;
- no-longer-agent/runtime unavailable;
- path deleted or kind changes;
- non-UTF-8/control path;
- channel busy/full;
- cancel/outside/Escape;
- exact-once and zero retry;
- no CR/LF/Enter/submit;
- zero stable socket/process/temp residue.

## 7. GIT VE YAYIN DİSİPLİNİ

- Acting account `CyPack`; external-contributor/fork guardrail geçerli.
- Upstream’a push, issue veya PR açma yok.
- Normal implementation RED ve GREEN ayrı atomic commit.
- Refactor yalnız GREEN arkasında ve ayrı concern olarak yapılır.
- Commit subject lowercase conventional; emoji/AI co-author yok.
- Bulk stage yok; yalnız owned files `git add -- <exact paths>`.
- `.superpowers/`, `.local/`, unrelated dirty files stage edilmez.
- Commit öncesi staged diff ve staged file list okunur.
- Push öncesi `git fetch origin`; local HEAD’in remote ref’lerin
  descendant’ı olduğu `merge-base --is-ancestor` ile doğrulanır.
- Yalnız CyPack `feat/native-fm` ve fork `master` fast-forward edilir.
- Push sonrası iki remote SHA local HEAD ile birebir doğrulanır.
- Codebase Memory commit sonrası yenilenir; recent changed symbols/snippets
  tekrar sorgulanır.
- Completion claim ancak fresh command output ve exact count ile yapılır.

Standing user authorization targeted continuity/product commits ve CyPack-only
FF push’ları kapsar. Ancak bu yetki scope genişletmez, upstream’a izin vermez,
stable runtime’a dokunma izni vermez ve test kapılarını kaldırmaz.

## 8. AÇIK GÖREV ENVANTERİ — MACHINE-EXACT COPY

Bu bölüm iki canonical registry’den mechanically copied unchecked task
bloklarını continuation satırlarıyla içerir. Beklenen kaynak sayıları 7 ve
89, toplam 96 olmalıdır. Fresh agent bu kopyaya kör güvenmez; kaynaklardan yeniden
sayar ve exact diff yapar.

<!-- OPEN_TASKS_START -->

### Source: `.codex/TASKS.md` — 7 unchecked

- [ ] **FIP-1.6** Add Playwright `TP-FIP-VIS-01` plus isolated real-mouse
  `TP-FIP-E2E-01` evidence without touching the stable Herdr socket.
  PARTIAL 2026-07-18: `TP-FIP-VIS-01` is GREEN (deterministic exported
  fixtures, both stage snapshots approved, visual suite 9/9). The isolated
  real-mouse `TP-FIP-E2E-01` smoke is explicitly deferred to the FIP-6.3
  closure run on the final build; do not claim it before that run.

- [ ] **FIP-6.3** Run isolated terminal mouse and PTY-byte smokes using
  `.local/ISOLATED-DEV-TEST.md`; prove exact path bytes and zero CR/LF.

- [ ] **FIP-6.7** Update `.codex` current/tasks/evidence, planning state,
  lessons, and next-session handoff with exact fresh evidence.

- [ ] **FIP-6.8** Verify clean tracked worktree, atomic RED/GREEN history,
  fast-forward ancestry, exact remote SHA equality, and CyPack-only push.

- [ ] Implement and verify `herdr-change-pipeline`, adapters, pilots, Git
  publication, and graph refresh; paused at T3.1 while the sequential active
  product lane closes its current phase.

- [ ] S5 ComponentRegistry only when a second real component/page proves the
  abstraction; do not build a speculative registry.

- [ ] S7 popup stack with ownership, focus, close ordering, and nested popup
  tests.

### Source: `.codex/CHANGE-PIPELINE-TASKS.md` — 89 unchecked

- [ ] **T3.1** Write RED `TP-CHG-MODULE` tests for module identity, version,
  directories, required documents, and default authorization=false.

- [ ] **T3.2** Create `.codex/skills/herdr-change-pipeline/` with `SKILL.md`,
  `README.md`, `AGENTS.md`, `module.json`, `assets/`, `references/`, `scripts/`,
  `tests/`, `evals/`, `lessons/`, and `cartography/`.

- [ ] **T3.3** Implement minimal manifest/schema validation and deterministic
  diagnostics until scaffold tests pass.

- [ ] **T3.4** Document skill routing, output ownership, resume behavior,
  source-of-truth order, and the separation from Ratatui reference research.

- [ ] **T3.5** Add errors, golden paths, edge cases, and shared-error routing.

- [ ] **T3.6** Verify the scaffold independently of Herdr product compilation.

- [ ] **T4.A0.1** RED-test every intake mode and reject unknown/ambiguous modes.

- [ ] **T4.A0.2** Model goals, inputs, evidence sources, current-work state,
  product authorization=false, and mode-conditional artifacts.

- [ ] **T4.A0.3** Implement `mid_flight_adoption` metadata: existing branch,
  commits, diffs, tests, known debt, current failures, and preserved evidence.

- [ ] **T4.A0.4** Block rather than fabricate when mandatory MCP/source evidence
  is unavailable.

- [ ] **T4.A1.1** RED-test missing actors, scenarios, success criteria, and
  explicit non-goals.

- [ ] **T4.A1.2** Emit measurable target behavior and acceptance boundaries for
  features, bugs, pages, layouts, runtime work, and composite requests.

- [ ] **T4.A2.1** RED-test orphan nodes, illegal level jumps, missing ownership,
  and missing failure/recovery leaves.

- [ ] **T4.A2.2** Implement the canonical chain: initiative -> experience/
  workflow -> page -> region/layout -> component -> interaction/behavior ->
  state transition -> failure/recovery.

- [ ] **T4.A2.3** Preserve parent/child traceability and stable identifiers.

- [ ] **T4.A3.1** RED-test omitted required dimensions, duplicate ownership,
  unresolved contradictions, and unjustified conditional omissions.

- [ ] **T4.A3.2** Cover product; behavior; page/input; layout/capability;
  component/tokens; data authority; runtime/API/event/PTY; failure/security/
  resources; persistence/migration; platform/accessibility; performance; and
  integration/license dimensions.

- [ ] **T4.A3.3** Record evidence, confidence, conflicts, and dependency edges.

- [ ] **T4.A4.1** RED-test single-option conclusions without explicit
  justification and visual-only pattern matching.

- [ ] **T4.A4.2** Produce alternative concepts, reusable patterns, rejected
  options, tradeoffs, capability fallbacks, and reversibility notes.

- [ ] **T4.A5.1** RED-test stale/absent graph evidence and `ready`-only
  freshness claims.

- [ ] **T4.A5.2** Map current owners, call/data paths, protocol/persistence
  boundaries, existing tests, and reuse candidates.

- [ ] **T4.A5.3** Emit current-versus-target architectural and functional fit.

- [ ] **T4.A6.1** RED-test unresolved conflicts and unsupported go decisions.

- [ ] **T4.A6.2** Select target architecture, behavior, data flow, fallbacks,
  budgets, and `go`, `conditional_go`, `no_go`, or `blocked` status.

- [ ] **T4.A7.1** RED-test incomplete traceability, missing decision evidence,
  conditional gaps, and mutable handoff fields.

- [ ] **T4.A7.2** Emit and validate immutable `change-intent-package.json`.

- [ ] **T4.A7.3** Prove native, reference-adapted, composite, no-go, blocked,
  and mid-flight packages through fixtures/evals.

- [ ] **T4.A7.4** Verify that A7 readiness still grants no product mutation.

- [ ] **T5.I0** Reject absent/invalid handoff, unapproved target, stale current
  state, or missing product authorization; accept mid-flight evidence only
  after provenance and current-phase classification.

- [ ] **T5.I1** Generate PRD, authority checklist, risk register, non-goals,
  rollback, compatibility, and migration obligations.

- [ ] **T5.I2** Refresh graph/repository evidence and detect drift between A7
  handoff and the live target.

- [ ] **T5.I3** Freeze current behavior, target behavior, semantic diff,
  retained behavior, change strategy, and ownership impact.

- [ ] **T5.I4** Build the test-point catalog with `what`, `current`, `expected`,
  `diff`, `result`, and `reason` for every applicable obligation.

- [ ] **T5.I5** Produce dependency-ordered implementation slices, test slices,
  commit boundaries, rollback points, and owned file sets.

- [ ] **T5.I6** Capture characterization evidence before moving behavior or
  architecture.

- [ ] **T5.I7** Require an observed behavior-specific RED; reject compile,
  environment, setup, flaky, or already-green false REDs.

- [ ] **T5.I8** Implement the minimum GREEN change and preserve exact command
  output as evidence.

- [ ] **T5.I9** Refactor only behind green tests; enforce local ownership and
  invariants.

- [ ] **T5.I10** Run cross-layer and cross-feature tests across all applicable
  families.

- [ ] **T5.I11** Verify failure, recovery, security, resources, capability
  negotiation, and degraded behavior.

- [ ] **T5.I12** Verify declared latency, allocation, throughput, memory, queue,
  and terminal-render budgets with calibrated fixtures.

- [ ] **T5.I13** Run complete repository, platform, protocol, migration,
  dependency, docs, and release-cadence gates applicable to the change.

- [ ] **T5.I14** Audit evidence, targeted staging, atomic commits, allowed
  publication, remote SHA, graph reindex, and current-symbol freshness.

- [ ] **T6.1** Server/runtime truth versus TUI/client projection.

- [ ] **T6.2** Snapshot/event ordering, revision, replay, duplicate, gap, and
  slow-subscriber behavior.

- [ ] **T6.3** PTY/terminal chunk boundaries, UTF-8/ANSI splits, queue pressure,
  resize, EOF, detach/reattach, and multi-pane throughput.

- [ ] **T6.4** Plugin host timeouts, crashes, output bounds, process cleanup,
  malformed data, version compatibility, and path confinement.

- [ ] **T6.5** Page/layout/component keyboard, mouse, focus, modal, resize,
  Unicode, narrow viewport, empty/loading/error, and terminal fallback states.

- [ ] **T6.6** Persistence interruption, corruption, migration, disk-full,
  concurrent owner, quota, and large-scrollback behavior.

- [ ] **T6.7** Platform isolation and Linux/macOS/Windows policy differences.

- [ ] **T6.8** Performance regression, slow-client isolation, soak, task leak,
  zombie process, and chaos behavior.

- [ ] **T6.9** Backward/forward protocol, old/new client, old/new plugin, and
  old persisted-state compatibility.

- [ ] **T7.1** P14 Ratatui/reference-project adapter.

- [ ] **T7.2** Native feature fixture.

- [ ] **T7.3** Mid-flight file-manager feature plus bugfix fixture.

- [ ] **T7.4** Page and interaction-flow fixture.

- [ ] **T7.5** Responsive layout and tiling fixture.

- [ ] **T7.6** Design-system/component/token fixture.

- [ ] **T7.7** Runtime capability and protocol fixture.

- [ ] **T7.8** Composite multi-dimension conflict fixture.

- [ ] **T7.9** Explicit no-go and blocked-MCP fixtures.

- [ ] **T7.10** Unauthorized delivery fixture proving I0 rejection.

- [ ] **T7.11** Verify that native mode invents no repository/source/license and
  reference mode omits no source/provenance/license obligations.

- [ ] **T8.1** Focused schema/validator unit tests.

- [ ] **T8.2** Complete tests for both skills and all negative fixtures.

- [ ] **T8.3** JSON parse, schema, stable-ID, version, and deterministic-output
  checks.

- [ ] **T8.4** Skill validation, README/AGENTS/SKILL consistency, and lesson
  format checks.

- [ ] **T8.5** Eval coverage for A0-A7, I0-I14, adapters, mid-flight adoption,
  blocked, no-go, and unauthorized paths.

- [ ] **T8.6** Legacy P0-P14 backward-compatibility verification.

- [ ] **T8.7** Product isolation and exact diff-boundary verification.

- [ ] **T8.8** Placeholder, whitespace, broken-link, and untracked-artifact
  scans.

- [ ] **T8.9** Proportional `just check` or documented exact equivalent.

- [ ] **T9.1** Preserve each baseline, RED, GREEN, refactor, governance, fixture,
  and evidence concern in reviewable atomic commits.

- [ ] **T9.2** Target-stage only the declared owned files and verify the staged
  name list before every commit.

- [ ] **T9.3** Fetch and prove fast-forward ancestry before any authorized push.

- [ ] **T9.4** Push only the permitted CyPack feature branch/ref; never
  `upstream`, never force.

- [ ] **T9.5** Verify exact local/remote SHA equality after publication.

- [ ] **T9.6** Reindex Codebase Memory after committed implementation changes.

- [ ] **T9.7** Record node/edge counts and query current pipeline/module symbols;
  never infer freshness from `ready` alone.

- [ ] **T10.1** Run one native page/feature request through A0-A7 without
  product mutation.

- [ ] **T10.2** Run one reference project through P0-P14 -> adapter -> A7.

- [ ] **T10.3** Run one mid-flight file-manager feature/bugfix adoption and
  prove existing evidence preservation plus remaining-gate enforcement.

- [ ] **T10.4** Run one composite conflict to a justified conditional-go/no-go.

- [ ] **T10.5** Prove unauthorized I0 rejection and blocked-MCP truthfulness.

- [ ] **T10.6** If separately authorized, run one non-product fixture through
  I14 before using the pipeline on Herdr product code.

- [ ] **T10.7** Record new errors, golden paths, edge cases, and any cross-skill
  lessons in the required tables.

- [ ] **T10.8** Update this registry, `.codex/CURRENT.md`, `.codex/TASKS.md`, and
  handoff state with exact final evidence and next action.

- [ ] **T10.9** Perform final self-review: requirements, tests, failure paths,
  Git state, publication state, graph freshness, and remaining blockers.

<!-- OPEN_TASKS_END -->

## 9. REFERANSLAR

Canonical:

- `AGENTS.md`
- `CLAUDE.md`
- `.codex/BOOTSTRAP.md`
- `.codex/CURRENT.md`
- `.codex/TASKS.md`
- `.codex/CHANGE-PIPELINE-TASKS.md`
- `.codex/HANDOFF.md`
- `.codex/MEMORY.md`
- `.planning/STATE.md`
- `.codex/NEXT-SESSION-PROMPT.md`

FIP:

- `docs/superpowers/specs/2026-07-17-herdr-files-interaction-polish-design.md`
- 55 unique `TP-FIP-*` IDs in that design

Safety and prior closure:

- `.local/ISOLATED-DEV-TEST.md`
- `.codex/evidence/miller-production-progress.md`
- `.codex/evidence/fm5-preview-placement-decision.md`
- `.codex/evidence/native-fm-completion-audit.md`
- `.codex/evidence/fm1-miller-viewport-progress.md`

Skills:

- `.codex/skills/herdr-native-fm/SKILL.md`
- `.codex/skills/herdr-native-fm/lessons/`
- `/home/ayaz/.codex/skills/session-handoff/SKILL.md`
- `/home/ayaz/.codex/skills/superpowers/writing-plans/SKILL.md` or the
  currently installed canonical equivalent exposed in the skill catalog
- `rust-dev` plus lessons before Rust work

MCP:

- Codebase Memory project `home-ayaz-projects-herdr`
- graph tools: `get_architecture`, `search_graph`, `trace_path`,
  `get_code_snippet`, `query_graph`, `index_repository`

## 10. FAIL-CLOSED DURDURMA KOŞULLARI

Agent şu koşullarda uygulama yazmayı durdurur ve gerçeği raporlar:

- task counts 54/89/143 ile uyuşmuyor;
- HEAD/remote ancestry beklenmedik;
- tracked unrelated dirty product diff var;
- `.superpowers/` ownership sınırı ihlal riski var;
- Codebase Memory recent symbol döndürmüyor;
- required skill/lesson okunamıyor;
- behavior-specific RED beklenen nedenle fail etmiyor;
- üç ardışık fix denemesi aynı blocker’da sonuç vermiyor;
- Playwright/Chromium bulunmuyor ve visual gate çalışmıyor;
- stable socket/process isolation kanıtlanamıyor;
- full gate veya platform gate fail;
- push target upstream veya non-FF görünüyor.

Bir gate fail olursa fail’i saklama, bypass etme veya “muhtemelen flake” deme.
Kök nedeni ayır, gerekiyorsa değişikliği geri al, yeni test ekle ve fresh
evidence üret.

## 11. TAMAMLANMA TANIMI

FIP ancak şu koşulların tamamı kanıtlandığında kapanır:

- 55 `TP-FIP-*` noktası traceable ve green;
- default visible Files click gerçek Native Files Stage’i açar;
- exact resident child highlight her chain/failure durumunda doğru;
- entry kinds/icons semantic, deterministic, pure ve font-fallback-safe;
- explicit agent picker current/other live agents için doğru;
- file/directory path exact bytes only, no CR/LF/Enter/submit;
- tüm stale/identity/path/control/backpressure vakaları zero-byte fail closed;
- Playwright Chromium, isolated mouse ve PTY byte tests green;
- full Rust/Linux/Windows/Bun/Python/maintenance gates green;
- performance/resource/invariant budgets green;
- stable runtime/socket untouched ve residue zero;
- graph fresh ve changed call/data-flow seams tekrar okunmuş;
- atomic commit chain ve clean tracked tree doğrulanmış;
- CyPack iki ref exact local SHA ile eşit;
- canonical continuity/evidence/lessons güncel.

## 12. BAŞLATMA TETİKLEYİCİSİ

Fresh session’da:

```text
Herdr Files Interaction Polish programına kanonik handoff’tan devam et.
AGENTS.md, CLAUDE.md ve .codex/NEXT-SESSION-PROMPT.md içindeki mandatory start
order’ı atlamadan uygula. Codebase Memory graph-first çalış; 54 ürün + 89
paused pipeline = 143 açık görevi exact continuation satırlarıyla yeniden say
ve session task listesine aktar. Yalnız FIP-G.1’i in_progress yap. Onaylı
tasarımı superpowers:writing-plans ile code-level RED/GREEN plana dönüştür,
FIP-G.2 graph/test/gate reconciliation bitmeden Rust yazma. Stable Herdr/socket
ve user-owned .superpowers/ alanına dokunma. Upstream’a push etme.
```
