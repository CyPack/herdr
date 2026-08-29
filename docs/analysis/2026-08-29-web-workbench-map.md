# WB-0 — WEB-WORKBENCH SİSTEM HARİTASI (2026-08-29 S52 · oturum 3251125c "HERDR WEB BROWSER")

> Kaynak: codebase-mcp (33081 düğüm, fast reindex 19:4x) + `grep` satır doğrulaması + tb vetli klonu + click-bridge repo + plugin-atlas (591 kayıt) + refpool (2990 db). Her satır `dosya:satır` kanıtlı; "muhtemelen" yok. Kanonik graf: `.cartography/web-workbench-SYSTEM-MAP.json`. Devir: `~/.claude/handoffs/herdr/Web-workbench-57-handoff.md` §B.

## §0 · Ontoloji (Katman 1)

| | |
|---|---|
| **Hedef (tek cümle)** | Tab-strip'e bir **browser butonu** koy; tıklayınca sağda **agent'a bağlı** bir browser pane (terminal-browser) doğsun; sayfadaki **element/bölge/not/ağ olayları** etiketli bir sözleşmeyle **o agent'a** ulaşsın — herdr ağırlaşmadan. |
| **Sistem türü** | TUI multiplexer (Rust, ratatui) + harici Electron tarayıcı (tb) + agent PTY'leri; JSON socket API. |
| **Aktörler** | kullanıcı (yerel/Mac client) · herdr server (render+API) · tb daemon (Electron, `--preload` izole-dünya JS, `--main-script` node) · agent süreci (Claude Code vb., PTY) · agent tarafı araçlar (CLI `herdr`, `events.subscribe`). |
| **IN** | buton+ikon+klavye · pane doğumu (split-right) · pane↔agent bağı (kalıcı) · olay sözleşmesi + rapor API + event kind · agent'a teslim (stage/submit) · preload `wb.js` (Alt+Click element, bölge seç, not) · CDP SS · network/endpoint yakalama · görsel doğrulama (kalıntı 0, sağlam siteler). |
| **OUT** | video/ses optimizasyonu (arşiv) · tb'nin kendi motoru (vendored, fork YOK) · click-bridge server'ına bağımlılık (payload sözleşmesi PORT edilir, süreç değil) · upstream'e katkı (yasak). |

## §1 · Bileşenler + kod çapaları (Katman 2) — 11 katman

| # | katman | çapa (dosya:satır) | ölçülen |
|---|---|---|---|
| L1 | **Tab-strip butonları** | `src/ui/tabs.rs:16-18` `SPLIT_BUTTON_WIDTH=3`; `TabBarView.split_right_hit_area/split_down_hit_area` :37-41; `pinned_split_button_hit_areas(seats_width, area_right, y)` :245-264 — kademeli koltuk: ≥6 hücre → iki buton, ≥3 → yalnız right, yoksa hiç; `compute_tab_bar_view` :276-431 (SPLIT_BUTTON :338, :383); testler :1251-1359 (`the_split_buttons_stand_pinned_at_the_strip_far_right`, `…six_spare_cells…`, `…three_spare_cells…`, `…too_narrow…`) | `AppState` view alanları `src/app/state.rs:2050-2052`; **boyama karakterizasyon digest'i** `src/ui/tab_surface.rs:374-381` (her yeni glyph = re-baseline; 2026-08-20/21 emsali) |
| L2 | **Tıklama → aksiyon → icra** | `src/app/input/mouse.rs:754-766` (TP-TAB-SPLIT-02 dispatch) → `MouseAction::SplitFocusedPane{direction}` :54 → `on_split_right_button` :2378 (`rect_contains`) → `src/app/input/mod.rs:1044` → `split_focused_pane_via_api` `src/app/input/navigate.rs:632-649` → `runtime_pane_split("tui.pane.split", PaneSplitParams{…})` → `handle_pane_split` `src/app/api/panes.rs:32-131` (PaneCreated + layout.updated emit; `PaneInfo` döner) | test emsali `a_click_on_a_split_button_asks_for_that_direction` mouse.rs:8821-8871 (`compute_view` → hit-area → `handle_mouse` → `MouseAction`) · klavye: `super+d new_split:right`, `super+s new_split:down` (keybind-map) |
| L3 | **Komutla split (pane'i program doğurur)** | `Workspace::split_pane_argv_command` `src/workspace.rs:1027-1056`, `_with_ratio` :1059-1089; `Tab::split_focused_command` `src/workspace/tab.rs:316-346` (`SplitCommand::Shell`) | çağıranlar: plugin split pane `src/app/api/plugins/panes.rs:119` · bar-section `src/app/tabs.rs:191` · overlay `src/app/input/navigate.rs:1159` · layouts `src/app/api/layouts.rs:421`. ⚠ `pane.split` API'si **komut almaz** (`PaneSplitParams` `src/api/schema/panes.rs:51-67`: direction/ratio/cwd/focus/env) |
| L4 | **Plugin pane yolu (emsal)** | `PluginPaneOpenParams` `src/api/schema/plugins.rs:563-580` (placement Overlay/Split/Zoomed…); `open_plugin_split_pane` `src/app/api/plugins/panes.rs:81-172`; `plugin_pane_launch_env` :242-275 → env: `HERDR_SOCKET_PATH`, `HERDR_ENV=1`, `HERDR_PLUGIN_ID`, `HERDR_PLUGIN_ENTRYPOINT_ID`, `HERDR_PLUGIN_CONTEXT_JSON`, `HERDR_BIN_PATH` | tb'nin kendi herdr plugin'i: `zenbu-labs.terminal-browser` `open-split` → `terminal-browser open --split right` (klon `herdr-plugin/open-split.sh`) — agent bağı YOK, sadece pane |
| L5 | **Pane↔agent modeli + kalıcılık** | `PaneInfo` `src/api/schema/panes.rs:516-560` (`agent_session: Option<AgentSessionInfo>`, `tokens`, `state_labels`, `dormant`, `revision`); `pane_info` `src/app/creation.rs:548-602`; `terminal_agent_session_info` :654-677; restore `src/persist/restore.rs:871-890`; capture-contract testleri `src/persist/snapshot.rs:2029-2102`; durable yazım `src/persist/durable.rs` (TP-PERSIST-01..04) | `pane.report_metadata` token'ları **TTL'li** (`expire_metadata_tokens` `src/app/actions.rs:1523-1563`) → kalıcı bağ için UYGUN DEĞİL (K2) |
| L6 | **Olay sistemi** | `EventKind` `src/api/schema/events.rs:199-260` (27 tür); **`pane.pointer` = TP-INP-MOUSE-01** out-of-band köprü: `EventData::PanePointer{pane_id,kind,button,column,row,x_px,y_px}` :433-446; doc `docs/next/website/src/content/docs/socket-api.mdx:836-858` ("a CDP driver for an embedded browser, a click-to-context tool can deliver it") | `events.subscribe`/`events.wait` API (`api_method_name` `src/api/server.rs:401-508`); emit `emit_event(EventEnvelope)` panes.rs:121-125. API JSON socket = `wire.rs` DEĞİL → PROTOCOL_VERSION bump gerekmez (claim cl9) |
| L7 | **Agent'a teslim** | `agent.prompt` `src/app/api/agents.rs:62-131` (`AgentPromptParams{target,text,wait}` `src/api/schema/agents.rs:176-181`; `Blocked` → `agent_blocked`; metin + gecikmeli Enter); `agent.send_keys` :245-288; `pane.send_text` panes.rs:1501-1520 (Enter'sız = **stage**); `resolve_agent_target` `src/app/terminal_targets.rs:75-105` (pane id VEYA agent adı) | herdr-annotations plugin'inin "stage, submit etme" deseniyle örtüşür (K4) |
| L8 | **terminal-browser (tb)** | kurulu v0.6.0 `~/.local/share/terminal-browser/app/{cli/dist,electron,browser,agent-browser}`; `open --help` :19-27 → `--preload=<path>` (izole dünya; `globalThis.terminalBrowser{theme,onTheme,quit}`; `--terminal-browser-session=<key>` argv) + `--main-script=<path>` (Electron main'de node); `ls --json` (cdp port + pane id); `action -- <agent-browser cmd>` | klon: `browser/src/page/controller.ts:124-129` (`additionalArguments`), herdr transport `engine/crates/pixel-core/src/herdr.rs:22-23` (`HERDR_PANE_ID`+`HERDR_SOCKET_PATH` → `pane.graphics.info`) · ⚠ klon eski (preload plumbing yok) — `--preload` **lab'da doğrulanacak** (TN-WB-5) |
| L9 | **click-bridge (prior-art, port edilecek)** | payload `snippet/click-bridge.js:137-153` `{component, component_chain, source{file,line}, selector, text, box, console_errors[≤10], failed_requests[≤10], viewport}`; server `server.py` 127.0.0.1:7823 `POST /click` → `~/.click-bridge/last.json` + `history.jsonl`; hook `hooks/claude-code-inject.sh` (session-wired token, exactly-once, 300 s tazelik, priority protocol: source → console → network → box) | tools: `dev-browser.sh` (CDP :9222), `portal-screenshot.py`, `pair-url.sh`, `self-heal.sh` |
| L10 | **Plugin ekosistemi** (atlas, 591) | sorgu "open a web browser preview pane split beside the agent" → 0.590 `herdr-oh-my-agent`/`hunk-autodiff`/`agents-picker` (alakasız); "annotate screenshot element and send to the agent" → 0.542 `agent-office`… (alakasız) ; "capture network requests devtools api endpoints" → 0.496 **`howaboua.annotations`** (herdr-annotations: seçim→not→popup pane→**agent'a stage**; `[[actions]] contexts=["pane","selection"]`, `[[panes]] placement="overlay"`), 0.494 `dotfiles.github-link-preview` (`[[link_handlers]]` → yan pane önizleme) | **KARAR: PRIOR-ART YOK → kendimiz yazarız**; `howaboua.annotations` = stage-deseni referansı (kod alınmaz, lisans bakılmadı) |
| L11 | **refpool** (2990 db) | "annotation overlay" → protobuf/vmlinux gürültüsü; "devtools protocol screenshot" → `protocol.rs` isim eşleşmeleri | prior-art YOK (sorgu+çıktı kanıtı) |

## §2 · İlişkisel ağ (Katman 3) — hedef akış

```
[tab strip]  … ⋯ + │⇥│⇩│🌐 ←TP-TAB-BROWSER-01 (L1: 3. koltuk, kademeli: 9/6/3 hücre)
        │ tık (L2: MouseAction::OpenWebPane → open_web_pane_via_api)  ·  klavye super+? (herdr-keybindings)
        ▼
  pane.web.open {target_pane_id?, direction:right, ratio:0.45, url?}      ← YENİ API (L3 yolu: split_pane_argv_command)
        │  argv = terminal-browser open <url> --preload <wb.js> --main-script <wb-main.js>
        │  env  = HERDR_SOCKET_PATH · HERDR_PANE_ID · HERDR_WEB_LINKED_AGENT=<pane_id> · HERDR_WEB_SESSION=<uuid>
        ▼
  TerminalState.web_link = {agent_pane_id, agent_session(kind,value), url}   ← L5 kalıcı (snapshot capture/restore)
  PaneInfo.web = Some(WebPaneInfo{...})                                       ← `herdr pane list` JSON
        │
  tb (L8) ── preload wb.js: Alt+Click / bölge / not  ──►  main-script: CDP screenshot + Network.*  ──►
        ▼
  pane.web.report_event {pane_id, event_kind: click|select|annotate|screenshot|network|console,
                         url, selector, component, source{file,line}, rect, screenshot_ref, note,
                         console_errors[], failed_requests[], request{method,url,status}}   ← YENİ API (L6)
        │  emit EventKind::PaneWebEvent ("pane.web.event") → events.subscribe aboneleri (agent araçları, plugin'ler)
        ▼
  teslim (L7): deliver:"stage" → pane.send_text (Enter YOK)  |  deliver:"submit" → agent.prompt (Enter)
               hedef = web_link.agent_pane_id (yoksa hata `web_pane_unlinked`, sessiz düşüş YOK)
```

## §3 · Karar noktaları (K1-K6) — öneri + gerekçe + emsal

| K | karar | öneri | gerekçe / emsal |
|---|---|---|---|
| **K1** buton yolu | `pane.split` (komutsuz) DEĞİL; yeni **`pane.web.open`** API + `MouseAction::OpenWebPane` + klavye + CLI aynı yolu sürer | TP-TAB-SPLIT-02 ilkesi ("ikinci yol = semantik sürüklenmesi"); `pane.split` komut almıyor (L3); plugin split pane deseni (`split_pane_argv_command` + launch env) birebir emsal (L4). Alternatif (tb plugin'i `plugin.pane.open`) reddedildi: plugin kurulumu + agent bağı yok |
| **K2** pane↔agent bağı | `TerminalState.web_link: Option<WebLink>` + snapshot capture/restore + `PaneInfo.web` | metadata token'ları TTL'li (L5) → kalıcı bağ için uygun değil; kalıcılık TP-PERSIST deseni (fsync'li durable save) — 08-29 modül-kaybı dersi |
| **K3** olay taşıyıcısı | herdr-native: **`pane.web.report_event`** (rapor API) + **`EventKind::PaneWebEvent`** — click-bridge server'a bağımlılık YOK, payload sözleşmesi port | `pane.pointer` TP-INP-MOUSE-01 zaten "dış köprü teslim eder" der (L6); tek kaynak, herdr kapalıyken hiçbir süreç yok (RD §6-2) |
| **K4** agent teslimi | iki mod: `stage` (varsayılan; `pane.send_text`, Enter yok — kullanıcı okuyup gönderir) · `submit` (`agent.prompt`) | herdr-annotations "stage without submitting" deseni; `agent.prompt` Blocked'da hata verir → stage her durumda çalışır; sessiz düşüş yok (RD6) |
| **K5** SS | tb `--main-script` node: CDP `Page.captureScreenshot{clip:rect}` → `$XDG_STATE_HOME/herdr/web/<pane>/<ts>.png` → `screenshot_ref` | tb `ls --json` cdp port veriyor (L8); alternatif `terminal-browser action -- screenshot` (agent-browser). **Lab'da doğrulanmadan kesinleşmez** (TN-WB-5) |
| **K6** endpoint yakalama | main-script CDP `Network.requestWillBeSent/responseReceived` → toplu `event_kind:network` (dedup endpoint listesi) · OpenAPI çıkarımı sonraki faz (mitmproxy2swagger deseni) | click-bridge `failed_requests` zaten son 10'u taşıyor (L9); tam liste CDP'den |

## §4 · RD §6 8-soru ön cevapları (PRD'de kesinleşir)

1. **Eksen:** Etkinlik (olay tetikli) — Dikkat/Canlılık eksenine maliyet yok. 2. **İzleyicisiz maliyet:** browser pane kapalıyken 0 kod koşar (buton salt hit-test; olay yolu yalnız rapor geldiğinde). 3. **Trafik:** yalnız olayda; `PaneInfo.web` `skip_serializing_if = None` → mevcut JSON değişmez. 4. **Animasyon:** yok (statik glyph). 5. **Canlılık:** pane defteri (`terminals` map) — süreç ağacı değil. 6. **Veri:** olaylar atılmaz; teslim edilemeyen olay `web_pane_unlinked`/`agent_blocked` ile **reddedilir** (refusal-over-loss), history dosyası opsiyonel. 7. **Sürüm:** yeni EventKind yalnız JSON API'de → eski `herdr` CLI abone değilse görmez; `PaneInfo.web` additive (`serde default`) → eski snapshot okunur. 8. **Kayıt:** TP-TAB-BROWSER-01/02 · TP-WEB-LINK-01 · TP-WEB-EVENT-01/02 · TP-WEB-DELIVER-01 (+ behaviors/tab-strip.md, behaviors/web-workbench.md YENİ).

## §5 · Test noktaları (Katman 4 — ne / beklenen / NEDEN)

| TN | ne | beklenen | NEDEN |
|---|---|---|---|
| TN-WB-1 | strip 3. koltuk hit-test (≥9/6/3 hücre kademeleri) | `web_hit_area` doğru; split rect'leri byte-identical (mevcut 7 test yeşil) | TP-TAB-SPLIT-01 geometrisi kırılmamalı; kademeli koltuk kuralı |
| TN-WB-1b | `tab_surface` karakterizasyon digest'i | yeni glyph → digest re-baseline (yorum satırı + tarih) | digest tam bu değişikliği yüzeye çıkarmak için var |
| TN-WB-2 | buton/klavye/CLI → `pane.web.open` → pane doğumu | `PaneCreated` + `PaneInfo.web.linked_agent_pane_id` = odaklı agent pane'i; PTY `winsize xpix>0` | tek yol (TP-TAB-SPLIT-02); FIX-1 dev-blok dersi |
| TN-WB-3 | bağ kalıcılığı | snapshot capture→restore sonrası `web` alanı aynı; eski snapshot (alan yok) yüklenir | TP-PERSIST + additive şema |
| TN-WB-4 | `pane.web.report_event` **gerçek dispatch** (`LoopEvent::Api` üzerinden) | event aboneye ulaşır + teslim modu çalışır; bağsız pane → `web_pane_unlinked` | [TZK-45] app-handler'ı doğrudan çağıran test gerçek yolu sürmez |
| TN-WB-5 | tb `--preload wb.js` lab (izole XDG) | `globalThis.terminalBrowser` var; Alt+Click JSON'u `pane.web.report_event`'e ulaşır; SS dosyası var | tb klonu eski; kurulu 0.6.0 help flag'i gösteriyor ama kanıt yok |
| TN-WB-6 | browser pane kapalı | tb süreci yok, `loop.tick` normal, `pane list` JSON değişmedi | RD §6-2/3 |
| TN-WB-7 | görsel: kalıntı 0 + web app (mnmveldops :3001, cc-dashboard) | kullanıcı gözü + iki client (V4.TN-5 deseni) | kabul ürün katmanından |
| TN-WB-8 | odaklı pane agent değilse buton | en yakın agent pane'e bağla YA DA bağsız aç + uyarı (PRD kararı) | sessiz yanlış bağ = en kötü hata |

## §6 · Görev kırılımı (WB-1 PRD için taslak)

WB-2 (buton) → 2a hit-area+koltuk · 2b glyph+digest · 2c `MouseAction::OpenWebPane` · 2d keybinding · 2e CLI `herdr pane web open` · WB-3 (bağ) → 3a `WebLink` + `PaneInfo.web` · 3b snapshot · 3c `pane.web.open` handler · WB-4 (olay) → 4a şema + `EventKind::PaneWebEvent` · 4b `pane.web.report_event` handler + gerçek-dispatch testi · 4c teslim (stage/submit) · WB-5 (preload) → 5a `wb.js` (click-bridge port) · 5b `wb-main.js` (CDP SS) · 5c lab · WB-6 (network) · WB-7 (görsel).

## §7 · V (açık düğümler) — harita sonlanma ölçüsü

`.cartography/web-workbench-SYSTEM-MAP.json` `variant.V`: açık claim'ler = (cl8) tb `--preload` kurulu sürümde çalışır mı [lab], (cl10) CDP screenshot main-script'ten erişilebilir mi [lab]. Bunlar WB-5 lab'ında kapanır; kod tasarımını bloklamaz (K5 alternatifi var).

---
## EK-A · Kullanıcı eki (19:5x) — commit zinciri · ikon altyapısı · header-bar alanı · 5×5 paralel

### A.1 Split-buton commit zinciri (git log `src/ui/tabs.rs`, katman katman)
| SHA | tarih | ne | katman |
|---|---|---|---|
| `2a1a8d64` | 06-30 | hide single-tab tab row | strip görünürlüğü |
| `299b5d35`/`c3d3ee06`/`71fed7b4` | 07-25 | Files peer entry · switch Files↔terminal · **stage tabs pinned LEFT** (TP-FTAB-ENTRY-05 — koltuk oyma emsali) | strip yerleşimi |
| `eb490c0c` | 07-26 | her display kendi aktif tab'ı | çok-ekran |
| `e48d8306` | 08-10 | configurable tab bar status (sağ segmentler: datetime/command; `src/app/tab_bar_status.rs`, `tab_bar_right_separator`) | strip sağ alanı |
| `d4244bad` | 08-20 | tab adı 20 hücre + **split butonları `+` yanında** | TP-TAB-NAME-01 / SPLIT-01 |
| `6123ee4d` | 08-20 | full-app frame digest re-baseline | karakterizasyon |
| `865ecc1d` | 08-21 | **split butonları strip'in en sağına PİNLENDİ** (kademeli koltuk) | TP-TAB-SPLIT-01 revised |
| `93c93075`/`8074ed8b`/`0013e97a` | 08-24 | upstream v0.8.2 senkronu + test dünyası onarımı | merge dikişi |

**Boyama (painter) çapası:** `src/ui/tabs.rs:642-660` — `+` = `" + "`, split-right = `" \u{2590} "` (▐), split-down = `" \u{2584} "` (▄), renk `p.overlay1`; scroll `" < "`/`" > "` :521-537; status segmentleri :696-714. Hit-area → view kopyası `src/ui.rs:784-785`. **Glyph literal'leri inline** — merkezi registry YOK (WB-8'in nedeni).

### A.2 L12 · İkon altyapısı (ölçüldü)
| bulgu | çapa |
|---|---|
| FM için `IconProfile { Nerd, Ascii }` + `VisualClass::glyph(profile)` (17 sınıf; Nerd = PUA `\u{f07b}` vb., Ascii = deterministik fallback, visual-fixture kanonik profili) | `src/fm/entry_kind.rs:55-110` |
| Varsayılan `file_icon_profile = Nerd`; Ascii yalnız fixture'larda | `src/app/mod.rs:984`, `src/ui/visual_fixture.rs:259` |
| Sidebar satır ikonları config'ten: `[spaces.icons] project/branch/chat/daily` (emoji default, her satır kendi `icon`ını ezebilir) | `src/config/model.rs:525-565, 579-615` |
| Bar widget'ları `glyph/art/pixels/palette` alanları taşıyor (mini-art altyapısı zaten var) | `src/config/model.rs:1232-1245` |
| Status göstergesi stili `dots|symbols` | `src/config/model.rs:166-180` |
| Claude aktivite glyph seti | `src/terminal/title.rs:1` |
| **Kullanıcının terminali:** kitty `font_family JetBrainsMono Nerd Font Mono` (`~/.config/kitty/kitty.conf:28`); fontTools cmap: **codicon PUA (U+EA60–U+EBEB) 388 glyph**, `nf-cod-browser` U+EB77 ✓, `nf-fa-globe` U+F0AC ✓ | executable kanıt (fontTools) |

**K7 — ikon stratejisi (öneri):** yeni `src/ui/glyphs.rs` (strip/bars chrome için tek registry): `ChromeGlyph { NewTab, SplitRight, SplitDown, Browser, … }` × `IconProfile` → `&'static str`; Nerd profili **codicon** (VS Code ikon dili: `nf-cod-browser`, `nf-cod-split-horizontal` U+EB56, `nf-cod-split-vertical` U+EB57, `nf-cod-add`) · Ascii profili mevcut (`+`, `▐`, `▄`, browser için `@`/`w`) · config `[strip.icons] browser = "…"` override (SpaceIconsConfig deseni) · **kullanıcının göndereceği ikonlar** = config satırı (kod değişmez). Glyph seçimi profile göre (fixture'larda Ascii → digest deterministik). Kayıt: TP-ICON-01 (profil), TP-ICON-02 (override), behaviors/surface-chrome.md.
⚠ Codicon'lar 1 hücre genişliğinde PUA — Nerd Font Mono varyantında güvenli; "Nerd Font" (Mono olmayan) çift genişlik verebilir → `display_width_u16` ile ölç (mevcut yardımcı `src/ui/text.rs`).

### A.3 L13 · Header/shell bar alanı (ölçüldü)
| bulgu | çapa |
|---|---|
| 4 kenar bar (`BarEdge::{Top,Bottom,Left,Right}`), stiller `framed/islands/plain/pills`, bar config paneli sağ-tık ile (TP-CHROME-150..152) | `src/app/bar_config_panel.rs:1-40` |
| Section modeli: `id/kind/cells/weight/min/max/border/group` + `widget{kind,text,metric,display,glyph,art,pixels,palette,format}` + `action{kind: popup, argv, command}` | `src/config/model.rs:1013-1300` |
| Davranış kaydı: surface-chrome.md **193** TP satırı (son: TP-CHROME-166), shell-spec.md 16, tab-strip.md 3 | behaviors/ |
**Sonuç:** strip chrome (tab-strip.md) ile bar chrome (surface-chrome.md) ayrı yüzeyler; browser butonu **strip**'te (TP-TAB-BROWSER-*), ileride bar'a bir `action{kind:"web", url}` eklemek mümkün (WB sonrası; cl12 open). Bar section'ları `glyph/art/pixels` ile mini-ikon/pixel-art taşıyabiliyor → kullanıcının "ascii mini ikon" vizyonu bar tarafında zaten altyapılı; strip tarafı WB-8 ile aynı registry'yi kullanır.

### A.4 K8 — 5 agent × 5 pane paralel (birinci sınıf gereksinim)
- Bağ modeli **N:N**: her web pane tam bir `agent_pane_id` taşır (odaklı agent pane'inden alınır; WB-10). `PaneInfo.web.linked_agent_pane_id` + `agent_session{kind,value}` kopyası (agent respawn'da kimlik değişirse bağ `stale` işaretlenir, sessiz yanlış hedef YOK).
- Olay yönlendirme: `pane.web.report_event` → yalnız bağlı agent'a teslim; `events.subscribe` abonesi `pane_id` filtresiyle kendi pane'ini dinler (mevcut `Subscription` param deseni, events.rs:18-90).
- Agent kapanınca (`note_agent_closed` `src/app/actions.rs:4965`) bağ → `orphan`; buton tekrar bağlar (re-link verb). TN-WB-9: 5 pane/5 agent fixture — çapraz teslim 0.
- Kaynak: browser pane başına bir tb daemon DEĞİL — tb tek daemon, pane başına pencere (`terminal-browser ls --json` browserKey/tabId) → 5 pane = 1 Electron süreci (RD §6-2).

### A.5 Ek TN
| TN | ne | beklenen | NEDEN |
|---|---|---|---|
| TN-WB-9 | 5 agent × 5 web pane; her pane'den olay | her olay yalnız kendi agent pane'ine; `pane list` 5 farklı `linked_agent_pane_id` | çapraz teslim = sessiz hata |
| TN-WB-10 | glyph registry: Nerd vs Ascii vs config override; `display_width_u16 == 1` | fixture digest'i Ascii'de değişmez; Nerd'de codicon 1 hücre | çift-genişlik glyph koltuğu taşırır |
| TN-WB-11 | agent kapanınca bağ | `web.link_state = orphan`; olay → `web_pane_unlinked` hatası | refusal-over-loss (RD6) |

### A.6 Harness notu — "ekranın altında task'ları görmüyorum"
Ölçüm: `ToolSearch select:TaskCreate,TaskList,TaskUpdate,TaskGet` → **eşleşme yok** (bu oturumun araç setinde Task araçları yok; 55 §12'de de aynı ölçüm). CC'nin alt task şeridi bu araçlarla dolar → bu oturumda **boş kalması beklenen davranış**. Kanonik defter `.local/TASKS-30.md` (her görev kanıt-kriterli). `herdr-todos-windows` plugin'i de aynı araçların aynasıdır → o da boş kalır. Araç seti oturum başlangıcında belirlenir; yeni oturumda görünürse T-1 bloğu oraya da aktarılır.
