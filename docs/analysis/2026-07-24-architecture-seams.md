---
doc: herdr-analysis
domain: architecture-seams
subject: katmanlar arası dikiş — state→compute_view→render→protocol→client; belge yüzeyi + custom layout genişleme maliyeti
created: 2026-07-24
method: codebase-memory-mcp (24.357 node/129.892 edge) + kaynak çapraz-doğrulama; grafik navigasyon, KAYNAK otorite
status: canonical — her iddia (claim, evidence=qualified_name/dosya:satır, confidence)
git_note: >
  /docs/* herdr .gitignore'da IGNORED (yalnız /docs/next/ istisna) → lokal yaşar, upstream'e sızmaz.
  DOĞRULANDI 2026-07-24: `.gitignore:10 /docs/*` · `:11 !/docs/next/` · `:12 !/docs/next/**`;
  `git check-ignore -v docs/analysis/2026-07-24-architecture-seams.md` → `.gitignore:10:/docs/*` (ignored).
  Makine kopyası: ~/.cartography/herdr-architecture-seams-*
agentic_triggers:
  - "mimari · katman · seam · dikiş · veri akışı · render pipeline"
  - "compute_view · render purity · Compositor · BaseLayer · RenderCtx"
  - "protocol · wire · FrameData · graphics · PROTOCOL_VERSION · render_ansi"
  - "server client boundary · runtime/client guardrail · client-local state"
  - "yeni yüzey ekleme · genişleme maliyeti · StageSurfaceView · BuiltInAppId"
  - "codebase mcp tazelik · grafik güvenilirliği · trace_path eksik kenar"
related:
  - docs/analysis/2026-07-24-document-render-internal-state.md
  - docs/analysis/2026-07-24-custom-layout-state.md
  - docs/patterns/custom-layout.md
  - docs/patterns/document-rendering.md
---

# herdr — Katmanlar Arası Mimari Dikiş Analizi

**Tarih:** 2026-07-24 · **HEAD:** `b48bd903` ("docs: pin directory click publication tip") · **Branch:** `feat/native-fm`

**Kapsam:** katmanlar arası dikiş (state → compute_view → render → protocol → client); "yeni belge yüzeyi (PNG/PDF/XLSX render + edit)" ve "custom layout template" özelliklerinin bu dikişte nereye çarptığı.

**Metot:** codebase-memory-mcp grafiği (24.357 node / 129.892 edge, `ready`) ile navigasyon + **kaynak dosyalardan çapraz-doğrulama**. Analiz salt-okuma yapıldı: hiçbir kaynak dosya değiştirilmedi, git mutasyonu yapılmadı, `index_repository` çalıştırılmadı, herdr server/socket'e dokunulmadı.

**Kanıt sözleşmesi:** her iddia `(claim, evidence, confidence)` üçlüsüyle yaşar. `evidence` = `qualified_name` veya `dosya:satır`. `confidence` ≥ 0.9 = kaynaktan okundu; 0.7–0.85 = yalnız grafik metriği; çıplak iddia yok.

**Kapsam notu:** `src/fm/*` iç detayları ve `kitty_graphics` yerleşim matematiği kardeş analizlerin alanıdır (bkz. `related`). Bu belgede yalnızca **dikişe değen** yüzeyleri kanıtlanmıştır.

---

## İÇİNDEKİLER

- [§0 · Grafik tazeliği ve kalibrasyon uyarıları](#0--grafik-tazeliği-ve-kalibrasyon-uyarıları)
- [§Ⓐ · Beş kritik soruya doğrudan cevap](#ⓐ--beş-kritik-soruya-doğrudan-cevap)
- [§A · Katman ve veri akışı haritası](#a--katman-ve-veri-akışı-haritası)
- [§B · Alt sistem kartları (B1–B6)](#b--alt-sistem-kartları)
- [§C · Önizleme boru hattı — tam çağrı zinciri](#c--önizleme-boru-hattı--tam-çağrı-zinciri)
- [§D · Grafik/görsel yetenek raporu](#d--grafikgörsel-yetenek-raporu)
- [§E · Genişleme maliyeti tabloları](#e--genişleme-maliyeti-tabloları)
- [§F · Mimari borç ve kırılganlık noktaları (13 madde)](#f--mimari-borç-ve-kırılganlık-noktaları)
- [§G · codebase-memory-mcp kullanım reçetesi ve BİLİNEN SINIRLARI](#g--codebase-memory-mcp-kullanım-reçetesi-ve-bilinen-sinirlari)
- [§H · Yeni yüzey/bölge eklerken kopyalanacak invariant deseni](#h--yeni-yüzeybölge-eklerken-kopyalanacak-invariant-deseni)
- [§I · Bu turda İNCELENMEYEN dikişler](#i--bu-turda-i̇ncelenmeyen-dikişler)
- [§J · Kapanış ve dosya yolu envanteri](#j--kapanış-ve-dosya-yolu-envanteri)

---

## §0 · Grafik tazeliği ve kalibrasyon uyarıları

> **ÖNCE BUNU OKU.** Bu bölümü atlayan bir agent, grafiğin eksik/kirli çıktısına güvenip yanlış sonuca varır.

| # | Bulgu | Kanıt | Güven |
|---|---|---|---|
| 0.1 | Grafik güncel, durum `ready`, 24.357 node / 129.892 edge; HEAD `b48bd903` ile uyumlu | `mcp__codebase-memory-mcp__index_status` + `git log --oneline -3` çapraz doğrulama | 0.95 |
| 0.2 | **Grafiğin Rust CALLS kenarları EKSİK.** `trace_path("preview_capability", inbound, depth=4)` **yalnızca 3 test** çağıranı döndürdü; gerçek üretim çağrısı `src/fm/trail_snapshots.rs:704`'te var ve grafikte **YOK**. Aynı şekilde `trace_path("read_image_preview", inbound)` gerçek çağıran yerine yalnız `__file__` node'unu döndürdü (gerçek çağrı: `src/app/image_preview_worker.rs:128`) | MCP `trace_path` çıktısı ⟷ `grep -rn "preview_capability(" src/ --include="*.rs"` | 0.95 |
| 0.3 | **Grafik kirli:** `.codex/evidence/miller-scroll-version-lab/v{0,1,2,3}-*/src/ui.rs` altında `src/ui.rs`'in **4 tam kopyası** indekslenmiş → `compute_view` (in_degree 81/82/84/85 + gerçek 113) ve `BaseLayer` semboller **5 kez** görünüyor | `search_graph(name_pattern="^(AppState\|Workspace\|compute_view\|BaseLayer\|…)$")` → 22 sonuç, 8'i lab kopyası | 0.95 |
| 0.4 | `search_graph(file_pattern="src/ui/shell")` **token limitini aştı** (131.211 karakter, tek satır) → shell alt sistemi **tamamen kaynaktan** okundu | MCP hata çıktısı: "exceeds maximum allowed tokens" | 0.95 |
| 0.5 | Çalışma ağacı `HEAD` ile temiz (session başında yalnız `?? .superpowers/` untracked); `src/fm/*` ve `src/app/*` dosya mtime'ları 18–23 Temmuz arası → indeks bu durumu kapsıyor | `git status` (session snapshot) + `ls -la src/fm/ src/app/` | 0.9 |

### Grafiğin göstermediği, kaynaktan bulunan somut semboller

| Sembol | Grafiğin dediği | Gerçek (kaynak) |
|---|---|---|
| `preview_capability` | yalnız 3 test çağıranı | **`src/fm/trail_snapshots.rs:704`** üretim çağrısı |
| `read_image_preview` | yalnız `__file__` node'u | **`src/app/image_preview_worker.rs:128`** |
| `highlight_text_preview` | — | `src/app/file_preview_worker.rs:365, 1212, 1313, 1397` + `src/fm/mod.rs:4086, 4121, 4148, 4186, 4224, 4249` |
| `read_text_preview` | — | `src/fm/mod.rs:562, 600` |
| Shell alt sisteminin **tüm** public tipleri | token aşımı → hiç dönmedi | `src/ui/shell/{model,layout,template,view,interaction}.rs` |

### Yöntem sözleşmesi (bundan sonraki her tur için)

```
GRAFİK = NAVİGASYON.   KAYNAK = OTORİTE.
Grafik "yok" diyorsa → grep ile DOĞRULA, "yok" sonucunu kabul ETME.
Grafik "var" diyorsa → dosya:satır ile teyit et, in_degree'yi tek başına delil sayma.
```

---

## §Ⓐ · Beş kritik soruya doğrudan cevap

### Ⓐ1 · Render gerçekten server-side mi? Client'a ne gidiyor? Kitty baytları bu boru hattından geçebiliyor mu?

**EVET, render tamamen server-side.** `src/server/render_stream.rs:286` `render_virtual_with_runtime_registry` içinde `crate::ui::compute_view_with_cell_size` (`:296`) veya `compute_view_without_resizing_panes` (`:298`) ve ardından `crate::ui::render_with_runtime_registry` (`:308`) çağrılır; hedef `CursorTrackingBackend` (ratatui `TestBackend` sarmalayıcısı, `:182-209`). **İstemci çizmez.**

Client'a giden **iki kodlama** var — `ClientRenderState` (`render_stream.rs:13-18`):

| Kodlama | Mesaj | Davranış |
|---|---|---|
| `RenderEncoding::SemanticFrame` | `ServerMessage::Frame(FrameData)` | tam hücre dizisi; önceki kareyle aynıysa atlanır (`:46-55`) |
| `RenderEncoding::TerminalAnsi` | `ServerMessage::Terminal(TerminalFrame{seq,width,height,full,bytes})` | `BlitEncoder` ile artımlı ANSI diff (`:56-85`) |

**Kitty ikili baytları BU BORU HATTINDAN GEÇİYOR. Protokol genişletmesi GEREKMİYOR.**

```
wire.rs:460-472   pub struct FrameData { cells, width, height, cursor, hyperlinks,
                      /// Kitty graphics protocol bytes to apply after the text frame.
                      pub graphics: Vec<u8> }          ← OPAK YAN-KANAL, ZATEN VAR
wire.rs:16        pub const PROTOCOL_VERSION: u32 = 16;
wire.rs:20/25     MAX_FRAME_SIZE = 2 MiB · MAX_GRAPHICS_FRAME_SIZE = 32 MiB
wire.rs:318,350   ClientMessage{…, cell_width_px: u32, cell_height_px: u32}
                      ↑ istemci KENDİ hücre piksel geometrisini sunucuya bildirir
```

#### `render_ansi.rs` KANITI — ANSI kodlayıcı **grafik-agnostiktir**

- `src/protocol/render_ansi.rs:433` → `let _ = writer.write_all(b"\x1b[?2026h");` (senkron çıkış BAŞLANGICI)
- `src/protocol/render_ansi.rs:468` → `let _ = writer.write_all(b"\x1b[?2026l");` (senkron çıkış SONU)
- `render_ansi.rs`'te `graphics` kelimesinin geçtiği **TEK** yer `:802` ve o bir test fixture'ıdır: `graphics: Vec::new()`
- Fonksiyon envanteri — `blit_frame_to` `:385` · `blit_frame_to_with_cursor_memory` `:398` · `blit_frame_to_with_cursor_memory_and_policy` `:417` · `write_all_cells` `:587` · `write_changed_cells` `:724` · `write_cell` `:683` · `color_to_sgr_fg` `:226` · `color_to_sgr_bg` `:261` · `modifier_to_sgr_parts` `:301` · `build_sgr` `:352` · `cells_equal` `:370` · `cells_visually_equal` `:709` · `write_hyperlink_if_changed` `:658` · `sanitized_hyperlink_uri` `:634` · `resolve_host_cursor_state` `:504` · `write_host_cursor_state` `:564` · `write_ime_anchor_cursor_state` `:578` — **tamamı hücre / SGR / kursör / hyperlink**. Grafik yok.

**Grafik enjeksiyonu kodlayıcının DIŞINDA, kodlanmış bayt akışına post-processing splice olarak yapılır:**

```rust
// src/server/render_stream.rs:61-73
let mut encoded = blit_encoder.encode(&frame, false);
crate::render_prof::event("prepare_frame.ansi.changed");
crate::render_prof::counter("prepare_frame.ansi.bytes", encoded.bytes.len() as u64);
if encoded.full { crate::render_prof::event("prepare_frame.ansi.full"); }
else            { crate::render_prof::event("prepare_frame.ansi.partial"); }
insert_graphics_before_sync_end(&mut encoded.bytes, &frame.graphics);          // :69
crate::render_prof::counter("prepare_frame.graphics.bytes", frame.graphics.len() as u64);

// src/server/render_stream.rs:128-140
const SYNC_OUTPUT_END: &[u8] = b"\x1b[?2026l";

fn insert_graphics_before_sync_end(encoded: &mut Vec<u8>, graphics: &[u8]) {
    if graphics.is_empty() {
        return;
    }
    if let Some(sync_end) = rfind_subslice(encoded, SYNC_OUTPUT_END) {
        encoded.splice(sync_end..sync_end, graphics.iter().copied());          // :136
    } else {
        encoded.extend_from_slice(graphics);                                    // :138
    }
}

// src/server/render_stream.rs:142-150
fn rfind_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || needle.len() > haystack.len() { return None; }
    haystack.windows(needle.len()).rposition(|window| window == needle)
}
```

→ Grafik, senkron-çıkış sonlandırıcısından **hemen önce** araya sokulur ⇒ metin + görsel **tek atomik kare** olarak uygulanır (yırtılma yok).

#### Sunucu tarafında `frame.graphics` doldurma

```rust
// src/server/headless.rs:3440-3458
let mut next_graphics_cache = client.graphics_cache.clone();      // ÖNBELLEK CLIENT-BAŞINA
let graphics_surface_reset_pending = client.graphics_surface_reset_pending;
if is_app_client && self.app.state.kitty_graphics_enabled && cell_size.is_known() {
    if graphics_surface_reset_pending {
        frame.graphics = next_graphics_cache.clear_bytes();                       // :3443-3445
    }
    let graphics_started = crate::render_prof::timer();
    frame.graphics.extend(crate::kitty_graphics::encode_local_pane_graphics(      // :3449-3454
        &self.app.state, &self.app.terminal_runtimes, cell_size, &mut next_graphics_cache));
    crate::render_prof::duration_since("full_render.graphics_encode", graphics_started);
} else {
    frame.graphics = next_graphics_cache.clear_bytes();                           // :3456-3458
}

// src/server/headless.rs:3465-3481
let mut commit_graphics_cache = true;
if frame.graphics.len() > MAX_GRAPHICS_FRAME_SIZE {
    warn!(client_id, graphics_bytes = frame.graphics.len(), max = MAX_GRAPHICS_FRAME_SIZE,
          "dropping oversized graphics payload for client frame");
    frame.graphics.clear();
    commit_graphics_cache = false;                                                // :3474
}
let max_frame_size = if frame.graphics.is_empty() { MAX_FRAME_SIZE }
                     else { MAX_GRAPHICS_FRAME_SIZE };                            // :3477-3481

// src/server/headless.rs:3567-3568
if commit_graphics_cache { client.graphics_cache = next_graphics_cache; }
```

#### İstemci tarafı tüketim

```rust
// src/client/mod.rs:1308-1309  — KAPI
let kitty_graphics_enabled =
    loaded_config.config.experimental.kitty_graphics && !direct_attach_requested;

// src/client/mod.rs:1527  — kitty açıkken kare sınırı yükselir
let max_frame_size = if kitty_graphics_enabled { /* 32 MiB */ } else { /* 2 MiB */ };

// src/client/mod.rs:1706-1712  — SEMANTIC yol
let graphics = if state.kitty_graphics_enabled { frame_data.graphics.as_slice() } else { &[] };
write_encoded_frame_with_graphics(&mut stdout, &encoded.bytes, graphics);

// src/client/mod.rs:1717-1718  — ANSI yol (baytlar zaten grafiği İÇERİYOR)
if state.kitty_graphics_enabled && contains_kitty_graphics_bytes(&frame.bytes) {
    record_received_kitty_graphics(&frame.bytes);
}

// src/client/mod.rs:2157-2171
fn write_encoded_frame_with_graphics(mut writer: impl io::Write,
                                     encoded: &[u8], graphics: &[u8]) -> io::Result<()> {
    writer.write_all(encoded)?;
    if graphics.is_empty() { return Ok(()); }
    record_received_kitty_graphics(graphics);
    writer.write_all(b"\x1b7")?;      // kursör kaydet
    writer.write_all(graphics)?;
    writer.write_all(b"\x1b8")        // kursör geri yükle
}

// src/client/mod.rs:2173-2175
fn contains_kitty_graphics_bytes(bytes: &[u8]) -> bool {
    bytes.windows(3).any(|window| window == b"\x1b_G")
}
```

#### Dört kesin sonuç

1. **`FrameData.graphics: Vec<u8>` OPAKtır.** Protokol içeriği hakkında hiçbir varsayım yapmaz → yeni piksel-tabanlı belge yüzeyi (PDF sayfası, XLSX grafiği) aynı alandan geçer, **`PROTOCOL_VERSION` bump'ı GEREKMEZ** *(confidence 0.95)*.
2. **Render saflığı korunuyor çünkü grafikler render'ın ÇIKTISI değil, KARDEŞİ.** `render()` yalnız hücre yazar; `encode_local_pane_graphics` `&AppState`'i **ayrı** okur ve bayt üretir. `Component::render(&self, frame: &mut Frame, area: Rect, ctx: &RenderCtx)` sözleşmesi (`src/ui/compose.rs:33-36`; `RenderCtx.app: &'a AppState` `:22-25`) hiç ihlal edilmez — mutasyon **tip düzeyinde imkânsız**.
3. **Lokal ve uzak yol AYNI fonksiyonu paylaşır:** `src/app/mod.rs:1209 paint_local_pane_graphics` → stdout **ve** `src/server/headless.rs:3449` → `frame.graphics`; ikisi de `encode_local_pane_graphics` çağırır. Fark yalnız **hedef** (stdout vs wire) ve **önbellek sahipliği** (global `LOCAL_HOST_GRAPHICS: OnceLock<Mutex<HostGraphicsCache>>` `src/kitty_graphics.rs:308` vs per-client `client.graphics_cache` `src/server/clients.rs:50`).
4. **CLAUDE.md guardrail'i bu tasarımı DOĞRULUYOR ve SINIRLIYOR:**
   - *Doğruluyor:* `graphics` nötr isimli, sunum-agnostik, sunucu-sahipli bir runtime gerçeği; hiçbir UI-yüzey ismi (sidebar/row/card/widget) protokole sızmamış.
   - *Sınırlıyor:* shell/layout tiplerinin **hiçbiri** protokole giremez — `src/ui/shell.rs:12-13` bunu açıkça beyan eder: *"Pure TUI presentation (AGENTS.md runtime/client boundary): none of these types are shared runtime facts, and none appear in `protocol`/`api::schema`."* Yani custom layout **client-local** kalmak ZORUNDA; `ShellSnapshotV1` de bu yüzden `protocol`'de değil `persist`'te durur.
   - *Kritik ayrım:* belge **DÜZENLEME** farklı bir sınıftır. İki istemci aynı belgeyi düzenlerse tampon **paylaşılan runtime gerçeğidir** → server state + `src/api/` şeması + **`PROTOCOL_VERSION` 16→17**. Render için gerekmeyen protokol değişikliği **edit için gerekir** *(confidence 0.85 — guardrail metni ima ediyor, henüz kod yok)*.

### Ⓐ2 · Yeni belge yüzeyi için TAM dosya/tip listesi → [§E-1](#e-1--senaryo-a-yeni-belge-yüzeyi-png--pdf--xlsx)

### Ⓐ3 · Custom layout için TAM dosya/tip listesi → [§E-2](#e-2--senaryo-b-custom-layout-template)

### Ⓐ4 · Mimari kırılganlıklar → [§F](#f--mimari-borç-ve-kırılganlık-noktaları) (13 madde, somut sembollerle)

### Ⓐ5 · Grafik tazeliği → [§0](#0--grafik-tazeliği-ve-kalibrasyon-uyarıları)

> **Özet:** grafik `ready` ve HEAD `b48bd903` ile uyumlu, **AMA** Rust CALLS kenarları eksik (`preview_capability`, `read_image_preview` üretim çağrıları grafikte yok) ve `.codex/evidence/miller-scroll-version-lab/` altındaki 4 `src/ui.rs` kopyası sembolleri 5'e katlıyor.

---

## §A · Katman ve veri akışı haritası

```
╔═══════════════════════ SAF VERİ (PTY/tokio gerektirmez) ═══════════════════════╗
║  AppState  src/app/state.rs (4.118 sat)            Workspace  src/workspace.rs ║
║   qn: home-user-projects-herdr.src.app.state.AppState  (in_degree 89)          ║
║   doc: "Testable without PTYs or a tokio runtime."                             ║
║   ├─ workspaces / active / selected / mode                                     ║
║   ├─ stage: StageState               (ui/surface_host.rs)                      ║
║   ├─ file_manager: Option<FmState>   (fm/mod.rs:616)                           ║
║   ├─ shell_presentation: ShellPresentationState  (shell/interaction.rs:262)    ║
║   ├─ shell_interaction:  ShellInteractionState   (shell/interaction.rs:562)    ║
║   └─ view: ViewState        ◄── compute_view'ın TEK yazdığı yer                ║
╚════════════════════════════════════╤═══════════════════════════════════════════╝
                                     │
        ┌────────────────────────────▼─────────────────────────────┐
        │ compute_view*  src/ui.rs:150/155/169/183 → _internal:260 │  MUTASYON+GEOMETRİ
        │  ① ShellLayout::default()          ui.rs:303  ⚠ SABİT    │
        │  ② ShellGeometryKey{area,layout_rev,constraints,collapse} │
        │  ③ shell::compute_shell_view → ShellView(gen, regions,   │
        │       hits, degradation)         shell/view.rs:108       │
        │  ④ region → sidebar_area / main_area   ui.rs:322-323     │
        │  ⑤ stage.surface_view() ile yüzey seçimi   ui.rs:328-337 │
        │  ⑥ FM alt-projeksiyonları (locations/miller/trail/rows)  │
        │       ui.rs:338-359, sync_* fn'leri :245,640,669,716,761 │
        │  ⑦ pane_infos + split_borders (+ PTY resize yan etkisi)  │
        │  ⑧ app.view = ViewState{ ~30 alan }     ui.rs:478-507    │
        └────────────────────────────┬─────────────────────────────┘
                                     │  app.view.*  (salt-okunur artefakt)
        ┌────────────────────────────▼─────────────────────────────┐
        │ render(&AppState, &mut Frame)   src/ui.rs:779/784        │  SAF ÇİZİM
        │   Compositor[ BaseLayer , OverlayLayer ]  compose.rs:42  │
        │    L0 BaseLayer  ui.rs:807  sidebar│tab_bar│AppDock│     │
        │        └ match stage.surface_view() {NativeFiles|Terminal│
        │                                       Workspace}  :844   │
        │    L1 OverlayLayer ui.rs:861  match app.mode { 22 kol }  │
        │   z-order = boyama sırası (ratatui'de z-index YOK) :39-41│
        └───────────┬────────────────────────────────┬─────────────┘
                    │ LOKAL                          │ SUNUCU (headless)
   ┌────────────────▼────────────────┐   ┌───────────▼──────────────────────────┐
   │ app/mod.rs:1169-1215            │   │ render_stream::render_virtual*:271   │
   │ SyncOutputGuard::begin()        │   │  → CursorTrackingBackend(TestBackend)│
   │ [clear_all_host_graphics? :1174]│   │  → FrameData::from_ratatui_buffer_   │
   │ terminal.draw(compute+render)   │   │      with_hyperlinks   wire.rs:492   │
   │ image_preview_cell_size=cell    │   │  → frame.graphics = encode_local_    │
   │ sync_image_preview_worker():1204│   │      pane_graphics()  headless:3449  │
   │ paint_local_pane_graphics():1209│   │                                       │
   │   └─ stdout'a \x1b7…\x1b8 :338  │   │                                       │
   └─────────────────────────────────┘   └───────────┬──────────────────────────┘
                                                     │ ServerMessage
                                     ┌───────────────▼───────────────────────────┐
                                     │ PROTOKOL  wire.rs  PROTOCOL_VERSION = 16  │
                                     │  FrameData{cells,w,h,cursor,hyperlinks,   │
                                     │            graphics: Vec<u8>}   :460-472  │
                                     │  TerminalFrame{seq,w,h,full,bytes} :573   │
                                     │  MAX_FRAME_SIZE 2MiB / GRAPHICS 32MiB     │
                                     │  ⚠ render_ansi.rs graphics'e DOKUNMAZ     │
                                     │     splice render_stream.rs:69,130-140    │
                                     └───────────────┬───────────────────────────┘
                                     ┌───────────────▼───────────────────────────┐
                                     │ CLIENT  src/client/mod.rs                 │
                                     │  Semantic → write_encoded_frame_with_     │
                                     │             graphics():2157 → \x1b7…\x1b8 │
                                     │  ANSI     → bytes zaten graphics içeriyor │
                                     │  kapı: experimental.kitty_graphics :1308  │
                                     └───────────────────────────────────────────┘
```

---

## §B · Alt sistem kartları

### B1 · Shell (dış kabuk / named-region layout)

| | |
|---|---|
| **Amaç** | Pane ağacının DIŞINDAKİ isimli-bölge kompozisyon ağacı. `src/ui/shell.rs:12-13`: *"Pure TUI presentation (AGENTS.md runtime/client boundary): none of these types are shared runtime facts, and none appear in `protocol`/`api::schema`"* |
| **Anahtar tipler** | `RegionId` 7 varyant (`TopBar/AppDock/LeftPanel/CenterContent/WorkspaceStage/RightPanel/BottomBar`) `…src.ui.shell.model.RegionId` model.rs:18-28 · `TrackPolicy` 5 (`Fixed{cells}/ContentBounded{min,max}/Resizable{min,preferred,max}/Fill{weight}/Collapsed{restore}`) :32-38 · `ShellComponentId` 6 (`AppDock/AgentSidebar/WorkspaceStage/Inspector/TopBar/BottomBar`) :42-49 · `ComponentPlacement{component,region}` :52-55 · `StackContainer{children,selected}` :58-61 · `RegionSize{Dynamic,Fill}` :65-68 · `ShellNode{Slot{region},Split{direction,children}}` :72-80 · `ShellChild{size,node}` :84-87 · `ShellDirection{Horizontal,Vertical}` :91-94 · `ShellLayout{root,tracks,stacks,component_placements}` :98-103 · `ValidatedShellLayout` :107 · `ShellValidationError` 13 varyant :121-135 · `RegionRects{rects}` :319-321 (**`RegionRects::get` in_degree 374** — grafiğin 6. hotspot'u) · `ShellTemplateId` 5 template.rs:12-18 · `ResponsiveDegradation` 5 layout.rs:15-21 · `SolvedShellLayout` :23-28 · `TrackRequest` :229-238 · `MeasuredNode`/`MeasuredChild` :92-108 · `ShellGeometryKey{area,layout_revision,constraints_revision,collapse_revision}` view.rs:16-21 · `ShellView{generation,area,regions,hits,degradation,geometry_key}` :61-68 · `ShellHitArea{generation,target,rect}` :53-57 · `ShellHitTarget::Region(RegionId)` :47-49 |
| **Etkileşim tipleri** | `DividerId` interaction.rs:9 · `MillerResizeColumnId` :27 · `MillerDividerId` :53 · `ResizeTargetId` :101 · `ResizeBounds` :117 · `ResizeTransaction` :142 · `ResizeDecision` :151 · `ResizeUpdate` :160 · `RegionCollapseState` :169 · `CollapseDecision` :177 · `CollapseUpdate` :185 · `ScrollAxis` :194 · `ScrollViewportId` :205 · `ScrollOffset` :214 · `ScrollViewportMetrics` :224 · `ScrollViewportState` :236 · `ScrollOwner` :246 · `ScrollDecision` :254 · `ShellPresentationState` :262 · `ShellInteractionState` :562 |
| **Giriş noktaları** | `compute_shell_view(layout,key,previous,resolve_dynamic)` view.rs:108 · `compute_empty_shell_view(key,previous)` :124 (mobil) · `ShellView::hit_at(gen,pos)` :86 · `ShellView::region_hit_at(gen,pos)` :100 · `ShellLayout::validate()` model.rs:170 · `ShellLayout::compute_projection` shell.rs:108 · `ShellLayout::solve_tracked` :125 · `layout::solve` layout.rs:51 · `template_persistence_parts(template)` shell.rs:43 · `validate_persisted_shell_parts(root,constraints,placements)` shell.rs:54 · `ShellTemplateId::validated_layout()` template.rs:21 · `ShellTemplateId::build()` :25 |
| **Kısıtlar (fail-closed)** | `MAX_NESTED_SPLIT_DEPTH=4` · `MAX_SPLIT_CHILDREN=8` · `MAX_VISIBLE_LEAVES=64` · `MAX_SERIALIZED_NODES=128` · `MAX_STACK_CHILDREN=32` · `MAX_COMPONENT_PLACEMENTS=64` (model.rs:9-14). `WorkspaceStage` **zorunlu** ve **collapse edilemez** (`MissingWorkspaceStage` :132 / `CollapsedWorkspaceStage` :133). **Deserialize doğrulayıcıdır** — `impl<'de> Deserialize for ShellLayout` `#[serde(untagged)] {Tree, Template}` (:294-314) → geçersiz ağaç tipe **hiç dönüşemez** |
| **Invariantlar** | ① `hit_at` yalnız **tam eşleşen generation**'a cevap verir (view.rs:86-95) → bayat koordinat sessizce yanlış bölgeye gitmez. ② generation `checked_add` ile taşarsa geometri korunur ama **hit listesi boşaltılır** (:145-156) — aliasing yerine fail-closed. ③ Geometri cache'i `ShellGeometryKey` eşitliğine bağlı; hit/miss `render_prof::event("shell.geometry_cache.hit"/".miss")` ile sayılır (:115,118). ④ Solver her node'u **en fazla 2 kez** ziyaret eder (`measure_node` layout.rs:110 + `allocate_node` :164; test `shell_solver_visits_each_node_at_most_twice` shell.rs:1046, `visits <= 8`). ⑤ Degradasyon sırası **donmuş** — yatay: `RightPanel` collapse → `LeftPanel` compact (`LEFT_PANEL_COMPACT_WIDTH=4`) → `AppDock` collapse → `TooSmall` (`degrade_workspace_requests` layout.rs:443-480); dikey: `BottomBar` → `TopBar` → `TooSmall` (`degrade_height_requests` :482-503). ⑥ `STAGE_MIN_CELLS=1` (layout.rs:11) sahne asla 0'a düşmez — `minimum_required` :525-535 + `distribute_fill` stage-donor mantığı :579-591. ⑦ `RegionRects::get` **total**tir, panik atmaz; yok olan bölge `Rect::default()` döner (model.rs:324-330) |
| **Geriye-uyum köprüsü** | `CenterContent ↔ WorkspaceStage` çift yönlü eşleme: `canonical_region` model.rs:337-342, `compatibility_region` :344-350. `ShellLayout::default()` = `LeftPanel(Dynamic) \| WorkspaceStage(Fill)` (shell.rs:70-93) ve eşdeğerlik testi `default_matches_legacy_outer_split_exactly` (:210-248) eski `Layout::horizontal([Length(sidebar_w), Min(1)])` ile **byte-özdeşliği** donduruyor |
| **Genişleme noktaları** | `ShellTemplateId::build()` (yeni built-in template) · `TrackPolicy` yeni politika · `ShellComponentId` + `ComponentPlacement` (**şu an hiçbir renderer tüketmiyor** — bkz. §F3) |
| **Test kancaları** | `ShellLayout::compute_regions` (`#[cfg(test)]` shell.rs:100) · `solve_tracked_for_test` :135 · `SolvedShellLayout::{visit_count,degradation,regions}` layout.rs:36-48 · `compute_shell_view_for_test` shell.rs:1350 · `shell_hit_for_test` :1365 · `legacy_sidebar_resolver` :1372 · `degradation_for_test`/`degradation_for_test_with` :1438/:1442 · `compute_regions_with_visit_count` :1433 · `render_prof::observe_for_test` view.rs:193 · shell.rs'te 30+ karakterizasyon testi |

**Kayda değer shell testleri (regresyon kalkanı):**
`default_matches_legacy_outer_split_exactly` :210 · `absent_region_returns_empty_rect` :253 · `serde_round_trip_default_and_nested` :264 · `shell_layout_places_dock_sidebar_stage_without_overlap` :277 · `shell_rejects_depth_above_four` :336 · `shell_rejects_more_than_eight_split_children` :346 · `shell_rejects_more_than_sixty_four_visible_leaves` :355 · `shell_rejects_more_than_one_hundred_twenty_eight_serialized_nodes` :376 · `shell_rejects_duplicate_outer_region` :397 · `shell_rejects_collapsed_or_missing_stage` :408 · `shell_rejects_more_than_thirty_two_stack_children` :423 · `shell_rejects_more_than_sixty_four_component_placements` :437 · `shell_rejects_duplicate_component_placement` :455 · `shell_rejects_invalid_track_bounds` :470 · `shell_rejects_out_of_range_stack_selection` :492 · `typed_templates_validate_without_runtime_registry` :506 · `fixed_track_uses_exact_cells_or_available_space` :596 · `content_bounded_clamps_measurement` :618 · `resizable_track_clamps_preferred` :638 · `fill_weights_split_only_remaining_cells` :658 · `collapsed_track_is_zero_and_keeps_restore_width` :679 · `zero_area_never_underflows` :705 · `allocation_remainder_is_deterministic` :723 · `all_rects_are_inside_parent_without_overlap` :743 · `shell_degrades_in_frozen_priority_order` :782 · `shell_degradation_respects_left_panel_track_bounds` :869 · `shell_degrades_height_without_starving_stage` :899 · `nested_stage_drives_height_degradation` :964 · `desktop_workspace_template_solves_normal_compact_and_too_small` :996 · `invalid_tracked_layout_fails_closed_without_partial_regions` :1027 · `shell_solver_visits_each_node_at_most_twice` :1046 · `shell_reports_explicit_too_small_degradation` :1069 · `unchanged_geometry_key_reuses_shell_generation` :1098 · `area_or_constraint_change_advances_shell_generation_once` :1123 · `committed_collapse_revision_invalidates_shell_geometry_once` :1146 · `flattened_hits_are_complete_disjoint_and_in_bounds` :1179 · `collapsed_or_inert_region_cannot_receive_focus` :1211 · `stale_shell_hit_generation_is_rejected` :1269 · `legacy_sidebar_and_center_rects_match_compatibility_projection` :1289 · `mobile_empty_projection_clears_hits_once_and_reuses_generation` :1302 · `generation_exhaustion_keeps_geometry_but_clears_hit_authority` :1323 · `nested_tree_lays_out_all_regions_and_survives_degenerate_area` :1490

### B2 · Stage / Surface Host (`src/ui/surface_host.rs`)

| | |
|---|---|
| **Amaç** | `WorkspaceStage` bölgesini **tam olarak bir** tipli yüzeye sahiplendirir |
| **Anahtar tipler** | `StageState` · `StageSurfaceView{NativeFiles,TerminalWorkspace}` (**in_degree 38**) · `AppSurfaceRef` · `BuiltInAppId{Terminal,Files}` · `AppInstanceId{app:BuiltInAppId, generation:u32}` · `AppInstance{id,surface}` · `AppDefinition{id,launch}` · `LaunchPolicy` · `StageStateError` |
| **Alanlar** | `StageState.active: AppInstanceId` (**in_degree 317**) · `.previous: Option<AppInstanceId>` (in_degree 25) · `.instances` · `.instance_count: usize` · `.last_generations: [Option<u32>; 2]` |
| **Giriş** | `StageState::surface_view()` (**in_degree 47**) · `activate_files()` · `close_files()` · `active_instance_generation()` (**in_degree 62**) · `insert_instance(instance)` · `next_instance_id(app)` · `remove_instance_at(index)` · `instance(id)` · `instances()` · `active_surface()` · `previous_surface()` · `BuiltInAppId::index()` (**in_degree 118**) · `BuiltInAppId::definition()` · `AppInstance::built_in(id)` |
| **Kısıtlar** | `MAX_BUILT_IN_INSTANCES = 16` → aşımda `StageStateError::BuiltInInstanceCapacityReached`. `generation: u32` `checked_add` ile tükenirse `StageStateError::InstanceGenerationExhausted` (**aliasing YASAK**). `close_files` `previous`'a geri döner |
| **⚠ Genişleme tuzağı** | `StageState.last_generations: **[Option<u32>; 2]**` — `BuiltInAppId::index()` ile indekslenir. **Üçüncü built-in app eklemek bu dizi boyutunu değiştirmeyi ZORUNLU kılar** *(confidence 0.85 — grafik alan-tipi kanıtı; kaynak teyidi önerilir)* |
| **Test kancaları** | `stage_starts_on_terminal_workspace` · `reactivating_singleton_files_keeps_one_surface` · `closing_files_restores_previous_terminal_surface` · `activating_files_records_previous_surface` · `failed_files_open_restores_previous_surface_and_focus` · `stage_rejects_more_than_sixteen_builtin_instances` · `instance_generation_exhaustion_fails_without_aliasing` · `stage_surface_switch_does_not_destroy_terminal_runtime` · `hidden_surface_has_no_stale_hits_or_cursor` · `active_surface_alone_populates_stage_hits` |

### B3 · File Manager (`src/fm/*` + `src/ui/file_manager*` + `src/app/file_*`)

| | |
|---|---|
| **Model** | `FmState` fm/mod.rs:616-663 — `cwd`/`entries`/`cursor`/`viewport_start`/`show_hidden`/`cwd_writable`/`cwd_status`/`cwd_omissions`/**`directory_generation`**/`parent`/**`preview: FmPreview`**/`preview_viewport_start`/**`preview_generation`**/`multi_selection`/**`trail: TrailState`**/**`trail_snapshots: TrailSnapshots`**/**`miller: MillerState`** |
| **Önizleme tipleri** | `FmPreview{None, File(FmFilePreview), Directory(Vec<FileEntry>)}` :234-241 · `FmFilePreview{PendingText{source_path,generation}, Text(TextPreview), Image(FmImagePreview), Unavailable(TextPreviewError)}` :245-257 · `FmImagePreview{source_path,generation,state}` :261-265 · `FmImagePreviewState{Pending, Loading{target}, Ready{target,prepared}, Unavailable{target,error}}` :268-281 · `TrailDetailPreview{PendingText, Text(TextPreview), Image, MetadataOnly(String), Unpreviewable(String)}` trail_snapshots.rs:36-45 · `TrailColSnapshot` :49-53 |
| **Yetenek seçimi** | `preview_capability(path,kind,providers)` fm/preview_capability.rs:74 — **saf**; modül doc'u (:1-5): *"Capability selection is client-local prepared state. It never reads the filesystem, checks `PATH`, loads configuration, spawns a process, or mutates file-manager navigation."* Tipler: `PreviewCapability{NativeText, NativeImage, MetadataOnly{reason}, OptionalPlugin{action_id,fallback}, Unsupported{reason}}` :45-58 · `PreviewFallback{NativeText, MetadataOnly(PreviewReason)}` :12-15 · `PreviewReason` 8 varyant :18-27 · `PreviewPluginProvider{action_id,platform_supported}` :61-64 · `PreviewProviderSet{markdown,documents,archives,media}` :67-72 |
| **Piksel hattı** | fm/image_preview.rs — `read_image_preview(path,target,limits)` → `prepare_image_preview_bytes` → `decode_image` → `validate_source_dimensions` → `resize_dimensions`/`aspect_fit`/`orientation_swaps_axes` → `checked_rgba_bytes` → `decode_with_panic_boundary`; yardımcılar `u64_to_nonzero_u32`, `usize_to_u64`. Tipler: `ImagePreviewTarget{width_px,height_px}` :18 · `PreparedImagePreview{width,height,rgba}` :47 · `ImagePreviewLimits` |
| **Metin hattı** | fm/text_preview.rs — `read_text_preview(path,limits)` · `highlight_text_preview(path,preview)` (syntect) · `select_syntax` · `plain_text_preview` · `styled_line_spans` · `without_line_ending` · `crossing_scalar_is_valid` |
| **Dosya işlemleri** | `operations.rs` (67 KB, kopyala/taşı) · `rename.rs` (63 KB) · `delete.rs` (29 KB, trash) · `watcher.rs` (17 KB) · `entry_kind.rs` · `entry_time.rs` · `natsort.rs` · `miller.rs` · `trail.rs` · `trail_snapshots.rs` (57 KB) — **hepsi yol seviyesinde**. **İçerik yazan hiçbir üretim yolu yok** (`fs::write` yalnız test fixture'larında; `grep` doğrulandı) |
| **UI projeksiyonu** | `src/ui/file_manager.rs` (3.799 sat) + `file_manager/{locations.rs 20KB, miller.rs 22KB, trail_view.rs 69KB}` · re-export'lar ui.rs:36-51: `compute_file_manager_action_bar_model` · `file_manager_preview_content_area` :218 · `locations_drawer_content_area` · `FileManagerLocationsView` · `project_miller_view` · `project_trail_view` |
| **Girdi** | `src/app/input/file_manager.rs` — **427 KB, repodaki en büyük dosya**; `handle_file_manager_key` :310 → `FileManagerKeyDispatch{CancelOperation, Refresh, PreviewDirectory{trail_col,entry_index,expected_path}, ActivateDirectory{…}, …}` |
| **App-katmanı işçiler** | `file_preview_worker.rs` 1.508 sat · `image_preview_worker.rs` 999 sat · `file_operation_worker.rs` 3.722 sat · `file_manager_io_worker.rs` 2.804 sat · `file_manager_watcher.rs` 1.887 sat · `file_manager_locations.rs` 693 · `file_manager_locations_model.rs` 307 · `file_manager_miller.rs` 352 · `file_agent_handoff.rs` 1.040 · `file_delete_confirmation.rs` 467 · `file_rename.rs` 465 |
| **Test kancaları** | `tests/visual/*.spec.ts` (Playwright: `navigation`, `trail`, `mtime-groups`, `icons`, `picker`, `focus`, `fractional-scroll`, `mutation`, `files-locations`, `harness`) + `src/ui/visual_fixture.rs::export_cell_fixture(name,&Buffer) -> CellFixture` (:27) + `tests/visual/harness/grid.js` + `tests/visual/fixtures/{self-test.json, generated/}` |

### B4 · Compositor (`src/ui/compose.rs`, 133 sat)

```rust
// :22-25   Read-only context threaded to every component
pub(crate) struct RenderCtx<'a> {
    pub app: &'a AppState,
    pub terminals: &'a TerminalRuntimeRegistry,
}

// :33-36   One composable, pure-render layer (helix's `Component`)
pub(crate) trait Component {
    /// Paint into `area`. Pure: reads `ctx`, never mutates state.
    fn render(&self, frame: &mut Frame, area: Rect, ctx: &RenderCtx);
}

// :42-57   Back-to-front stack over immediate-mode rendering
pub(crate) struct Compositor { layers: Vec<Box<dyn Component>> }
impl Compositor {
    pub fn new(layers: Vec<Box<dyn Component>>) -> Self { Self { layers } }
    pub fn render(&self, frame: &mut Frame, area: Rect, ctx: &RenderCtx) {
        for layer in &self.layers { layer.render(frame, area, ctx); }
    }
}
```

- `Compositor::new` **in_degree 278** (grafiğin 8. hotspot'u)
- Doc `:39-41`: *"Z-order is strictly paint order — ratatui has no z-index, so a later `render` call overwrites earlier ones in overlapping cells."*
- **`&self` + `&AppState` ⇒ render sırasında mutasyon TİP DÜZEYİNDE imkânsız**
- Testler: `later_layer_paints_over_earlier` :87 (`"BBAA"` beklentisi) · `routed_render_is_deterministic` :116 (aynı state → aynı buffer)

### B5 · Server / Client sınırı

| | |
|---|---|
| **Sunucu render** | `render_virtual` render_stream.rs:271 · `render_virtual_with_runtime_registry` :286 · `render_terminal_virtual` :327 · `visible_hyperlinks` :360 · `focused_terminal_cursor` :382 · `focused_terminal_owns_host_cursor` :454 · `focused_terminal_suppresses_host_cursor` :482 · `CursorTrackingBackend` :182-269 |
| **İki kodlama** | `ClientRenderState::{Semantic{last_frame:Option<FrameData>}, TerminalAnsi{blit_encoder:BlitEncoder, seq:u64}}` :13-18; `new(render_encoding)` :21 · `reset_baseline` :31 · `reset_semantic_input_baseline` :38 · `prepare_frame(frame)` :44 · `last_frame()` :89 · `commit_sent_frame(prepared)` :96 · `terminal_seq()` :120 |
| **PreparedRender** | `PreparedRender::{Semantic{message}, TerminalAnsi{message,frame,encoded}}` :153-162; `message()` :165 · `into_frame()` :171 |
| **ANSI kodlayıcı** | `BlitEncoder` / `EncodedBlit` render_ansi.rs:39-56; `frame_with_drawn_cursor` :136 · `ProfBlitStats`/`compute_prof_blit_stats` :150/:156 · `HostCursorState` :497 · `repeat_ime_anchor_after_sync` :482/:487 (platform-gated) |
| **Client durum** | `src/client/mod.rs` — `kitty_graphics_enabled` :59, :76, :1308, :1476, :1515 · `write_encoded_frame_with_graphics` :2157 · `contains_kitty_graphics_bytes` :2173 · `record_received_kitty_graphics` :2177 · `clear_received_kitty_graphics` :2188 (+ çıkışta :567) · `resize_poll_loop(…, kitty_graphics_enabled, …)` :1517 · `current_terminal_geometry(kitty_graphics_enabled)` :1338 |
| **Sınır kuralı (CLAUDE.md)** | Yeni **paylaşılan runtime gerçeği** → server state + JSON API; yeni **sunum durumu** → yalnız TUI/client. Nötr isim zorunlu (sidebar/row/card/widget YASAK). Örnekler: pane/agent metadata, process state, terminal state, events → server; sidebar layout, token placement, colors, selection, modals, mouse/viewport state → TUI |
| **Test** | `tests/{client_mode,multi_client,server_headless,detach_reattach,live_handoff,cross_area,api_ping,auto_detect,cli_wrapper}.rs` + `tests/support/mod.rs` |

### B6 · Girdi yönlendirme (`src/app/input/`)

```rust
// src/app/input/shell.rs:10-25
/// The single owner the frozen shell input precedence resolves for one event.
///
/// Frozen order (design spec "Focus, Mouse, and Keyboard Routing"):
/// topmost blocking overlay -> active capture -> z-ordered topmost hit ->
/// focused component -> page/template shortcut -> global shortcuts ->
/// fail-closed consumption so hidden background surfaces never act.
pub(crate) enum ShellInputOwner {
    TopmostOverlay, ActiveCapture, TopmostHit(RegionId),
    FocusedComponent, PageShortcut, GlobalShortcut, FailClosed,
}

// :43-63   TOTAL BY CONSTRUCTION — her bağlam tam bir sahibe eşlenir
pub(crate) fn route_shell_input(context: ShellInputRouteContext) -> ShellInputOwner {
    if context.topmost_overlay { return ShellInputOwner::TopmostOverlay; }
    if context.active_capture  { return ShellInputOwner::ActiveCapture; }
    if let Some(target) = context.topmost_hit { return ShellInputOwner::TopmostHit(target); }
    if context.focused_component { return ShellInputOwner::FocusedComponent; }
    if context.page_shortcut     { return ShellInputOwner::PageShortcut; }
    if context.global_shortcut   { return ShellInputOwner::GlobalShortcut; }
    ShellInputOwner::FailClosed
}
```

- `ShellInputRouteContext{topmost_overlay, active_capture, topmost_hit, focused_component, page_shortcut, global_shortcut}` :31-38
- `AppState::shell_key_input_owner()` :72-84 — klavye pozisyonsuz ⇒ `topmost_hit: None`; `focused_component` = `stage.surface_view() == NativeFiles && file_manager.is_some()` (:73-75)
- `AppState::shell_mouse_input_owner(position)` :94-106 — pozisyonel hit **yalnız tam güncel generation**tan: `self.view.shell.region_hit_at(self.view.shell.generation, position)` (:98-101)
- `AppState::enter_overlay_mode(overlay)` :112-118 — `overlay_return_mode` hatırlama
- `AppState::blocking_overlay_active()` :123-146 — **exhaustive `match app.mode`**; doc: *"The match is exhaustive so a new mode must choose a side explicitly instead of silently leaking background input."*
- Dağıtım: `App::handle_key` mod.rs:77-129 · `handle_key_headless` :131-158 · `handle_active_capture_key` :160-167 (`debug_assert!(handled, "an active capture must consume every key")`) · `handle_focused_file_manager_key` :169-176 (`debug_assert!` FM yokluğunu yakalar) · `handle_mouse` :394 / mouse.rs:99
- Dosya boyutları: `file_manager.rs` **427 KB** · `mouse.rs` 170 KB · `navigate.rs` 119 KB · `sidebar.rs` 118 KB · `modal.rs` 82 KB · `copy_mode.rs` 74 KB · `terminal.rs` 54 KB · `mod.rs` 49 KB · `shell.rs` 39 KB · `settings.rs` 34 KB · `selection.rs` 13 KB · `overlays.rs` 35 KB

---

## §C · Önizleme boru hattı — tam çağrı zinciri

```
[1] SEÇİM
    TrailSnapshots::reconcile…                         src/fm/trail_snapshots.rs
      └─ trail.select_file(col_idx, &selected.path)                       :693
      └─ self.cols.truncate(col_idx + 1)                                  :694
      └─ self.detail = Some(prepare_trail_detail(&selected.path, selected.kind))  :695

[2] YETENEK  (saf; FS/PATH/config/process'e dokunmaz)
    prepare_trail_detail(path, kind)                                      :701
      └─ preview_capability(path, kind, &PreviewProviderSet::default())   :704
           ├─ Directory | SymlinkDirectory → Unsupported{DirectoryUsesTrail}  cap.rs:79-86
           ├─ BrokenSymlink               → Unsupported{BrokenSymlink}         :87-91
           ├─ UnsupportedSpecial          → Unsupported{SpecialFile}           :92-96
           ├─ path non-UTF8 ∨ kontrol karakteri → Unsupported{UnsafePath}      :98-107
           ├─ fm::is_image_preview_path(path)   → NativeImage                  :109-111
           ├─ md|markdown|mdown  → plugin_or_fallback(markdown , NativeText)   :123-125
           ├─ pdf|doc|docx|odt|rtf|xls|xlsx|ods|ppt|pptx|odp
           │                     → plugin_or_fallback(documents, MetadataOnly{DocumentMetadata}) :126-136
           ├─ zip|tar|gz|bz2|xz|7z|rar|zst|*.tar.{gz,bz2,xz}
           │                     → plugin_or_fallback(archives , MetadataOnly{ArchiveMetadata})  :137-149
           ├─ mp3|flac|wav|ogg|m4a|aac|mp4|mkv|mov|avi|webm|mpeg|mpg
           │                     → plugin_or_fallback(media    , MetadataOnly{MediaMetadata})    :150-161
           ├─ bin|exe|dll|so|dylib|class|wasm|o|a|pyc → MetadataOnly{BinaryMetadata}             :162-171
           └─ aksi                → NativeText                                                    :173
      └─ TrailDetail{path, kind, preview: TrailDetailPreview}              :719-723

    ⛔ ANOMALİ (descent adayı): çağrı `PreviewProviderSet::default()` — TÜM sağlayıcılar `None`
       ⇒ `plugin_or_fallback` (cap.rs:180-196) daima `fallback` koluna düşer
       ⇒ `PreviewCapability::OptionalPlugin` ÜRETİMDE ERİŞİLEMEZ.
       Kanıt: `grep -rn "PreviewProviderSet" src/` → 7 isabet; non-default kurulum
       YALNIZ `preview_capability.rs:321` (test). Sonuç: PDF/XLSX bugün DAİMA MetadataOnly.

[3a] METİN İŞÇİSİ                              src/app/file_preview_worker.rs (1.508 sat)
     App::sync_file_preview_worker() -> bool                              :382
       çağrı yerleri: app/runtime.rs:211 (headless döngü)
                      app/input/file_manager.rs:3098, 3120, 3125 (navigasyon)
       iş: fm::read_text_preview(path, TextPreviewLimits::default())      fm/mod.rs:562,600
           → preview.highlighted = Some(highlight_text_preview(path,&preview))
                                                    :365, 1212, 1313, 1397
       yardımcılar: take_next_request(shared) · lock_state(state) · process_preview(path,source)
       slot: en-son-kazanır; stale generation reddi
       (testler: highlight_slot_rebinds_and_rejects_stale_generation,
                 highlight_slot_close_rejects_prior_generation,
                 pending_slot_rejects_same_path_and_preview_generation_after_files_reopen,
                 highlight_worker_keeps_only_latest_pending_request,
                 pending_preview_worker_executes_first_and_latest_only,
                 text_worker_profile_counts_submitted_completed_and_rejected,
                 highlight_worker_reports_processor_disconnect_without_panic,
                 dropping_preview_worker_does_not_wait_for_blocked_processor,
                 branch_truncation_rejects_stale_preview_completion,
                 stale_worker_completion_after_scroll_is_rejected,
                 app_discards_inflight_highlight_after_file_manager_close,
                 app_reopen_highlights_only_the_new_file_manager_selection)

[3b] GÖRSEL İŞÇİSİ                             src/app/image_preview_worker.rs (999 sat)
     App::sync_image_preview_worker() -> bool                             :268
       ├─ KAPI: state.view.file_manager_miller.resize_preview_active → erken çık   :269-271
       ├─ hedef koşulu: FmPreview::File(FmFilePreview::Image(preview)) olmalı      :274
       ├─ hedef geometri:
       │    kitty_graphics::file_manager_image_target(
       │        &state.view.file_manager_trail, file_manager, self.image_preview_cell_size)  :277-281
       │      └─ file_manager_trail_image_content_area()          kitty_graphics.rs:120-142
       │           · trail_snapshots.detail()?.preview == TrailDetailPreview::Image
       │           · FmPreview::File(FmFilePreview::Image(p)) ∧ p.source_path == detail.path
       │           · snapshot.detail_panel.as_ref()?.content_rect
       │      └─ image_geometry_for_content_area(area, cell_size) → (Rect, ImagePreviewTarget)
       ├─ anahtar: ImagePreviewKey::new(&preview.source_path, preview.generation, target)   :282-286
       ├─ slot.sync(target):                                              :45-58, 177-196
       │    · self.active == target  → Unchanged
       │    · aksi → generation = wrapping_add(1).max(1); active = target
       │            → Started{generation} (pending = TEK istek, en-son-kazanır) | Stopped
       ├─ Started → set_image_state(Loading{target})  + render_prof "fm.image_target.refresh"  :290-299
       ├─ Stopped → set_pending_image_state(Pending)                       :300-302
       └─ Unchanged → drain():                                             :198-219
            · result.take() ; slot.accepts(result.generation, &result.key)?     :60-63, 207
                 EVET → render_prof "fm.image_worker.completed"
                 HAYIR→ render_prof "fm.image_worker.rejected"  ⇒ SONUÇ ATILIR
            · disconnected (worker öldü) → Unavailable{DecodeFailed} + tracing::warn  :308-320

     İşçi thread'i (std::thread::spawn + Condvar, tokio::sync::Notify ile uyandırma):  :143-167
       output = std::panic::catch_unwind(AssertUnwindSafe(|| processor(&key.path, key.target)))
                  .unwrap_or(Err(ImagePreviewError::DecoderPanicked));            :149-152
       ⇒ decoder paniği İŞÇİYİ ÖLDÜRMEZ, tipli hataya dönüşür
       Drop: state.closed = true; pending/result = None; notify; handle.join()    :222-236
       ImageWorkerAliveGuard: thread ölürse alive=false + wake                     :105-116
       take_next_image_request: Condvar bekleme + closed kontrolü                  :238-251
       lock_image_state: poisoned mutex'te into_inner() (panik YOK)                :253-260

[4] DURUM UYGULAMA (üçlü kilit)
     set_image_state(state, &key, next) -> bool                            :352-371
       ├─ state.file_manager yoksa                              → false
       ├─ preview FmFilePreview::Image değilse                  → false
       ├─ preview.source_path ≠ key.path ∨ preview.generation ≠ key.model_generation → false  :363
       ├─ preview.state == next (değişim yok)                   → false    :366  (gereksiz redraw yok)
       └─ preview.state = next                                 → true
     set_pending_image_state(state) -> bool                                :338-350

[5] ÇİZİM
     ui.rs BaseLayer → render_file_manager(app, frame, terminal_area)      ui.rs:846
       trail_view.rs:788   TrailDetailPreview::Image kolu
       file_manager.rs:1004-1011   durum → mesaj eşlemesi:
         Pending      → Some(mesaj)      :1004-1006
         Loading{..}  → Some(mesaj)      :1007-1009
         **Ready{..}  → None**           :1010     ⇒ hücreye HİÇBİR ŞEY yazılmaz; DELİK bırakılır
         Unavailable{error,..} → Some((mesaj, error))  :1011
     kitty_graphics piksel katmanı bu deliği doldurur (§D)

[6] LOKAL DÖNGÜ SIRASI (kritik — sıra bozulursa görsel bir kare geç gelir)
     app/mod.rs:1169-1215
       SyncOutputGuard::begin()                                            :1170
       [full_redraw ⇒ clear_all_host_graphics + terminal.clear]            :1172-1178
       terminal.draw(|frame| { cell_size = HostCellSize::from_terminal(area);
                               compute_view_with_cell_size(...);
                               render_with_runtime_registry(...); })       :1180-1202
       self.image_preview_cell_size = cell_size                            :1203
       if self.sync_image_preview_worker() { render_dirty = true; notify }  :1204-1207
       if kitty_graphics_enabled { paint_local_pane_graphics(...) }        :1208-1214
```

**Sınır özeti:** her iki işçi de **tek yuvalı, sınırlı, en-son-kazanır**; kuyruk yok, geri basınç yok. Kimlik üçlüsü (`path`, `model_generation`, `target`) her aşamada yeniden doğrulanır. `preview_generation` her bağlam yenilemesinde artar — `fm/mod.rs:648-650` doc: *"Monotonic client-local identity for preview work. Every context refresh invalidates in-flight image results even when the path is unchanged."*

---

## §D · Grafik/görsel yetenek raporu

| Soru | Cevap | Kanıt |
|---|---|---|
| Hangi protokol? | **Yalnızca Kitty graphics.** Sixel YOK, iTerm2 YOK | `src/kitty_graphics.rs` tek grafik modülü (2.075 sat); `use crate::ghostty::{KittyImageDescriptor, KittyImageFormat, KittyImagePlacement, KittyPlacementRenderInfo}` :14-17; kaçış dizisi `\x1b_G` client/mod.rs:2174. `grep -ri "sixel" src/` → sıfır isabet |
| Kapı | `config.experimental.kitty_graphics` — **deneysel, varsayılan kapalı** | `set_enabled(config.experimental.kitty_graphics)` app/mod.rs:409, 1587; client/mod.rs:1308-1309; `static KITTY_GRAPHICS_ENABLED: AtomicBool` kitty_graphics.rs:307, `set_enabled` :310, `is_enabled` :314 |
| İki içerik kaynağı | ① FM görsel önizlemesi ② **PTY passthrough** (pane içindeki uygulamanın ürettiği Kitty görselleri; vendored libghostty-vt ayrıştırır) | `collect_file_manager_image_placement` ⟷ `collect_visible_placements` kitty_graphics.rs:399-403; pane/terminal.rs:1106 `has_kitty_graphics_sequence`, :1746/:2005 `hide_kitty_placeholders` |
| Z-index | **YOK** — sıra = boyama sırası; grafikler metin frame'inden SONRA | compose.rs:39-41; `insert_graphics_before_sync_end` render_stream.rs:130 |
| FM yerleşimi | Tek-tonluk: `image_id=1`, `placement_id=1`, sanal pane `PaneId::from_raw(u32::MAX)`; içerik kutusunda **ortalanır** (`viewport_col/_row = (area - grid)/2`) | :23-25, :186-190 |
| Doğrulama kapıları | `prepared.width/height == 0` ∨ hedefi aşıyor → `None` (:160-165) · `rgba.len() != w*h*4` → `None` (:167-173) · `grid_cols/rows == 0` ∨ `> area` → `None` (:175-183) | :154-190 |
| Aktarım sınırı | `KITTY_CHUNK_BYTES = 3072` · `HOST_IMAGE_ID_BASE = 10_000` (pane görselleri için ayrılmış kimlik tabanı) | :21-22 |
| Hücre geometrisi | `HostCellSize::from_terminal(area)` → `crossterm::terminal::window_size()`; başarısız/sıfırsa **8×16 fallback** (`fallback_for_area`) | :34-60; `is_known()` :51 |
| Mod kapısı | `encode_local_pane_graphics` yalnız `app.mode == Mode::Terminal` **∧** `cell_size.is_known()`; aksi hâlde `cache.clear_bytes()` | :356-373 |
| Önbellek | `HostGraphicsCache{images: HashMap<u32,ImageSignature>, placements: HashMap<(u32,u32),PlacementSignature>, sources: HashMap<(PaneId,u32),u32>, view: Option<HostViewKey>}` :299-305; lokal global `LOCAL_HOST_GRAPHICS: OnceLock<Mutex<..>>` :308, uzak `client.graphics_cache` clients.rs:50 |
| Yüzey değişimi | `surface_changed` (file_manager_open geçişi) → `cache.clear_bytes()` :378-390; FM açık + placement boş + eski kaynak varsa temizle :403-411 | |
| Çerçeveleme | `frame_graphics_bytes(bytes)` → `\x1b7` + bytes + `\x1b8` (kursör kaydet/geri yükle) | :318-324; `paint_local_pane_graphics` :326-345 |
| Temizlik | `clear_all_host_graphics()` :626 — çıkışta main.rs:725-726, 791-792; tam-redraw'da app/mod.rs:1174; client tarafı `clear_received_kitty_graphics` :567, :2188 | |
| Retained-frame etkisi | `has_visible_pane_graphics(app, runtimes, cell_size)` :433 — headless.rs:3160'ta "retained PTY update" kararında kullanılır; `graphics_cache` doluysa retained yol reddedilir (:3152-3153) | testler: `retained_pty_update_allows_kitty_enabled_empty_graphics_cache` headless.rs:7945, `retained_pty_update_declines_when_graphics_cache_has_content` :7975 |
| Profilleme | `render_prof` sayaçları: `full_render.graphics_encode`, `prepare_frame.graphics.bytes`, `prepare_frame.ansi.{bytes,full,partial,changed,skip_current}`, `shell.geometry_cache.{hit,miss}`, `fm.image_worker.{submitted,completed,rejected}`, `fm.image_target.refresh`, `fm.filesystem.{read,read_success}`, `shell.compute_view` | ui.rs:267; view.rs:115,118; image_preview_worker.rs:185,208,211,292; render_stream.rs:48,51,58,62-72 |

---

## §E · Genişleme maliyeti tabloları

### E-1 · Senaryo A: Yeni belge yüzeyi (PNG / PDF / XLSX)

Üç ayrı alt-senaryo var; maliyetleri **çok** farklı.

#### A-0 · PNG ve tanınan raster formatlar — **ZATEN ÇALIŞIYOR**

`preview_capability → NativeImage → image_preview_worker → kitty_graphics`. Tek eksik: `experimental.kitty_graphics` varsayılan kapalı.

**Ek maliyet ≈ 0 · Risk: YOK.**

#### A-1 · PDF/XLSX → **hücre tabanlı** yüzey (metin/tablo; piksel değil) — SIRALI

| Sıra | Dosya:satır / tip | Değişiklik | Risk | Neden |
|---|---|---|---|---|
| 1 | `src/fm/preview_capability.rs:45` `PreviewCapability` | Yeni varyant (ör. `NativeDocument{kind}`) **veya** mevcut `OptionalPlugin`'i canlandır | **Düşük** | Saf fonksiyon; 3 kapsamlı test (`:209`, `:319`, `:378`). Uzantı listeleri `:126-161`'de hazır |
| 2 | `src/fm/trail_snapshots.rs:704` | `PreviewProviderSet::default()` → **gerçek** sağlayıcı seti | **Orta** | ⚠ **İLK BLOKER.** Bugün sabit `default()` ⇒ yeni kol sessizce ölü kalır. Sağlayıcı kaynağı (config/plugin registry) **mevcut değil**, sıfırdan kurulmalı |
| 3 | `src/fm/trail_snapshots.rs:36` `TrailDetailPreview` | + varyant | **Düşük** | `match`'ler exhaustive → derleyici tüketicileri gösterir ✅ |
| 4 | `src/fm/mod.rs:245` `FmFilePreview` | + varyant (`PendingDocument`/`Document`) | **Orta** | `FmState::current_refresh_request` (`:690-704`) gibi **elle yazılmış** exhaustive match'ler var |
| 5 | **YENİ** `src/app/document_preview_worker.rs` | `image_preview_worker.rs`'i şablon al: `Key{path,generation,target}` + `Slot{generation,active}` + `sync()/accepts()` + `catch_unwind` + `AliveGuard` + `Drop` | **Orta** | ~1.000 satır; kalıp kanıtlı ve 12 testle donmuş |
| 6 | `src/app/runtime.rs:211-212` · `src/app/mod.rs:1204` · `src/app/input/file_manager.rs:3098, 3120, 3125` (+ `:1418`) | `sync_document_preview_worker()` çağrısı ekle | **Orta** | ⚠ **≥5 farklı yer**; birini atlamak = "önizleme bazen gelmiyor" (sessiz hata) |
| 7 | `src/ui/file_manager/trail_view.rs:788` + `src/ui/file_manager.rs:1004-1011` | Yeni durum→çizim eşlemesi | **Düşük** | Saf render |
| 8 | `Cargo.toml` | PDF/XLSX ayrıştırıcı bağımlılığı | **Orta-Yüksek** | CLAUDE.md: *"Don't add dependencies without a reason."* + supply-chain: exact pin, lifecycle-script denetimi. PDF ayrıştırıcıları geniş saldırı yüzeyi |
| — | **PROTOKOL** | **DEĞİŞİKLİK YOK** | — | Hücreler `FrameData.cells`'ten gider; `PROTOCOL_VERSION = 16` sabit |
| — | **Test** | `AppState::test_new` · `preview_capability` birim testleri · `visual_fixture::export_cell_fixture` · `tests/visual/*.spec.ts` | — | Mevcut altyapı yeterli |

**Toplam risk: ORTA.** Kritik yol: madde 2 (sağlayıcı seti ölü) + madde 6 (çağrı yeri dağınıklığı).

#### A-2 · PDF/XLSX → **piksel** yüzey — A-1'in tümü **artı**

| Sıra | Dosya:satır | Değişiklik | Risk | Neden |
|---|---|---|---|---|
| 9 | `src/kitty_graphics.rs:23-25` | `FILE_MANAGER_PREVIEW_{PANE_RAW, IMAGE_ID, PLACEMENT_ID}` singleton'ları | **YÜKSEK** | **Tek** FM görseli varsayar (`image_id=1`, `placement_id=1`, `pane=u32::MAX`). İkinci eş-zamanlı piksel yüzeyi **kimlik alanı şeması** gerektirir (`HOST_IMAGE_ID_BASE=10_000` pane'ler için ayrılmış) |
| 10 | `src/kitty_graphics.rs:399-403` | `collect_file_manager_image_placement` yanına yeni toplayıcı | **Orta** | `encode_graphics_update` `images`/`placements`/`sources` üçlüsünü tutar; yeni kaynak üçlüye doğru kayıt yazmalı |
| 11 | `src/kitty_graphics.rs:356-373` | `mode_ok = app.mode == Mode::Terminal` kapısı | **Orta** | Yeni yüzey bir overlay modunda görünürse grafik **sessizce çizilmez** |
| 12 | `src/server/headless.rs:3466` | 32 MiB `MAX_GRAPHICS_FRAME_SIZE` | **Orta** | Yüksek çözünürlüklü PDF sayfası taşarsa kare **sessizce düşürülür** (yalnız `warn!`); kullanıcı boş kutu görür |
| 13 | `src/server/headless.rs:3152-3165` | `graphics_cache` retained-frame kararı + `has_visible_pane_graphics` | **Orta** | Yeni kaynağı bilmezse gereksiz tam-render veya eksik güncelleme |
| — | **PROTOKOL** | **YİNE DEĞİŞİKLİK YOK** | — | §Ⓐ1 kanıtı: `graphics: Vec<u8>` opak |

**Toplam risk: YÜKSEK — ama protokol yüzünden DEĞİL,** `kitty_graphics` kimlik/önbellek modelinin tek-görsel varsayımı yüzünden.

#### A-3 · Belge **EDIT** (düzenleme) — en pahalı, mimari karar gerektiren

| # | Alan | Durum | Risk |
|---|---|---|---|
| 14 | İçerik yazma yolu | **HİÇ YOK.** `fm/operations.rs`, `rename.rs`, `delete.rs` yalnız yol-seviyesi. Üretim kodunda `fs::write` **sıfır** (grep doğrulandı) | — |
| 15 | Düzenlenebilir tampon modeli | Yok. `TextPreview` **sınırlı ve salt-okunur** (`TextPreviewLimits`; bozuk UTF-8'de `Unavailable`) | **Yüksek** |
| 16 | Watcher etkileşimi | `src/app/file_manager_watcher.rs` (1.887 sat) + `directory_generation` — kendi yazımın watcher olayını tetikleyip önizlemeyi geçersizleştirmesi | **Yüksek** |
| 17 | Girdi yönlendirme | `shell_key_input_owner` `focused_component`'ı **`NativeFiles ∧ file_manager.is_some()`** olarak sabit kodluyor (input/shell.rs:73-75); `handle_key` `FocusedComponent` kolu **doğrudan** `handle_focused_file_manager_key`'e gidiyor (input/mod.rs:114-119; içinde `debug_assert!` FM yokluğunu yakalıyor :170-176) | **Yüksek** |
| 18 | **Sınır kararı (CLAUDE.md)** | Düzenleme tamponu **paylaşılan runtime gerçeği mi?** İki istemci aynı belgeyi açarsa **EVET** → server state + `src/api/` şeması + **`PROTOCOL_VERSION` 16→17** | **Yüksek** |
| 19 | `src/persist/snapshot.rs` | Kaydedilmemiş tampon oturum devrinde ne olacak? `SNAPSHOT_VERSION = 4` bump'ı | **Orta** |

**Toplam risk: ÇOK YÜKSEK.** Bu bir "özellik" değil, **yeni bir alt sistem**.

> **Öneri:** A-0/A-1'i önce teslim et; edit'i ayrı bir mimari tur olarak ele al ve **§18 kararını kod yazmadan ÖNCE ver.**

#### A-S · Yeni yüzeyi **Stage app'i** yapmak (Files gibi tam yüzey) — SIRALI

| Sıra | Dosya | Değişiklik | Risk |
|---|---|---|---|
| S1 | `src/ui/surface_host.rs` | `BuiltInAppId` +varyant · `StageSurfaceView` +varyant · `AppDefinition`/`LaunchPolicy` · **`last_generations: [Option<u32>; 2]` → `; 3`** | **YÜKSEK** — sessiz dizi-taşma; `BuiltInAppId::index()` in_degree **118** |
| S2 | `src/ui.rs:328-337, 424-437, 439-449, 561` | `terminal_surface_active` boolean'ı **üçlü** yüzey mantığına | **Orta** — 4 ayrı yerde tekrarlanmış |
| S3 | `src/ui.rs:844-851` | `BaseLayer` `match app.stage.surface_view()` | **Düşük** — exhaustive, derleyici zorlar ✅ |
| S4 | `src/app/input/shell.rs:72-84` | `focused_component` genelleştirme | **Yüksek** |
| S5 | `src/app/input/mod.rs:114-119, 143-145, 169-176` | `FocusedComponent` yönlendirmesi + `debug_assert` | **Yüksek** |
| S6 | `src/persist/snapshot.rs:44` | `PinnedBuiltinAppV1` +varyant; `SNAPSHOT_VERSION` 4→5 | **Orta** |
| S7 | `src/ui/app_dock.rs` | `AppDockModel::for_state` + `app_dock_entry_areas` | **Düşük** |

---

### E-2 · Senaryo B: Custom layout template

Burada **iki tamamen farklı** iş var. Ayırmak kritik.

#### B-1 · Yeni **built-in** template eklemek — **DÜŞÜK RİSK, ~1 saat**

| Dosya:satır | Değişiklik |
|---|---|
| `src/ui/shell/template.rs:12-18` | `ShellTemplateId` + varyant |
| `src/ui/shell/template.rs:25-74` | `build()` kolu (yardımcılar hazır: `shell_layout` :101, `slot` :108, `horizontal` :112, `vertical` :119, `dynamic_child` :126, `fill_child` :133, `dock_track` :140, `sidebar_track` :148) |
| `src/ui/shell.rs:506` | `typed_templates_validate_without_runtime_registry` beklenen bölge listesine ekle |

**Ama:** ⛔ **hiçbir görsel etkisi olmaz.** Neden → B-2.

#### B-2 · Layout'u **canlı** yapmak — **YÜKSEK RİSK; mimarinin gerçek boşluğu**

> **MERKEZÎ BULGU:** Shell veri modeli, doğrulaması, çözücüsü, kalıcılığı ve etkileşim reducer'ları **hazır ve testli**; ama **canlı geometri yolu bunların hiçbirini okumuyor.**

| # | Kanıt | Sonuç |
|---|---|---|
| B2.1 | `src/ui.rs:303` — `let shell_layout = ShellLayout::default();` | Layout **her karede sabit**. `default()` = `LeftPanel(Dynamic) \| WorkspaceStage(Fill)` (shell.rs:70-93). Template hiç kullanılmaz |
| B2.2 | `src/ui.rs:306` — `LEGACY_DESKTOP_SHELL_LAYOUT_REVISION` | `ShellGeometryKey.layout_revision` **sabit** ⇒ layout değişse cache miss oluşmaz ⇒ geometri güncellenmez |
| B2.3 | `src/ui.rs:312-318` — `resolve_dynamic` closure yalnız `RegionId::LeftPanel` için değer döndürür, diğer her bölge `0` | AppDock / RightPanel / TopBar / BottomBar `Dynamic` boyutla **daima 0 genişlik** |
| B2.4 | `src/persist/snapshot.rs:29-41` `ShellSnapshotV1{schema_version, template, root, region_constraints, component_placements, collapse_restore_widths, pinned_dock_order}` **kaydediliyor ve doğrulanıyor** (`validate_persisted_shell_parts` :124-128) | Tek geri-projeksiyon `restored_left_panel_preference()` :132-151 → sadece `(width, collapsed)`. Tüketiciler: `src/app/mod.rs:438, 954`. **`root`, `template`, `component_placements` okunuyor, doğrulanıyor, sonra ATILIYOR** |
| B2.5 | `ComponentPlacement` / `ShellComponentId` (6 varyant) / `StackContainer` | `grep`: yalnız `src/ui/shell/*` + `src/persist/snapshot.rs`. **Hiçbir renderer tüketmiyor.** `ShellComponentId → renderer` kayıt defteri (registry) **YOK** |
| B2.6 | `src/ui.rs:481, 496, 501` — `sidebar_rect`, `tab_bar_rect`, `terminal_area` ViewState'te **ayrı alanlar**; render bunları okur, `shell.regions`'ı değil | **Tek istisna:** AppDock — `app.view.shell.regions.get(RegionId::AppDock)` ui.rs:462 (geometri) + :832 (render). **Doğru desenin tek örneği** ✅ |
| B2.7 | `src/ui/shell/template.rs:10` doc: *"Closed built-in page templates; Foundation v0 exposes no arbitrary layout DSL."* | **Kasıtlı** kısıt |
| B2.8 | `src/ui/shell/model.rs:294-314` `Deserialize for ShellLayout` `#[serde(untagged)] {Tree, Template}` | Veri modeli **keyfi ağacı ZATEN kabul ediyor** (sınırlı doğrulamayla). Eksik olan **runtime kablolaması** |

**Canlı yapmak için dokunulacak TAM liste — SIRALI:**

| Sıra | Dosya:satır | Değişiklik | Risk | Neden |
|---|---|---|---|---|
| 1 | `src/app/state.rs` (`AppState`) | `shell_layout: ShellLayout` + `shell_layout_revision: u64` alanları | **Orta** | `AppState::test_new` + `assert_invariants_for_test` (:2871) + `test_with_adversarial_identity_state` (:2862) güncellenmeli |
| 2 | `src/ui.rs:303` | `ShellLayout::default()` → `app.shell_layout` | **YÜKSEK** | Tüm masaüstü geometrisinin kökü; regresyon riski maksimum. `default_matches_legacy_outer_split_exactly` (shell.rs:210) karakterizasyon testi kalkan ✅ |
| 3 | `src/ui.rs:306` | `LEGACY_DESKTOP_SHELL_LAYOUT_REVISION` → gerçek revizyon sayacı | **YÜKSEK** | Yanlışsa **bayat geometri** (yanlış cache hit) veya **her karede yeniden hesap** (perf). `render_prof "shell.geometry_cache.hit/miss"` ile ölç |
| 4 | `src/ui.rs:312-318` | `resolve_dynamic`'i **tüm** bölgeler için genelleştir | **Orta** | Bölge-başına ölçüm kaynağı gerekir (`TrackPolicy::ContentBounded` gerçek içerik ölçümü ister — layout.rs:385-394) |
| 5 | `src/ui.rs:322-323, 478-507` | `sidebar_rect`/`tab_bar_rect`/`terminal_area` → `shell.regions.get(...)` sorgusu | **YÜKSEK** | ViewState'in en çok okunan alanları; tüm render + mouse hit-test bunlara bağlı |
| 6 | **YENİ** component registry + `src/ui.rs:807-857` `BaseLayer` | `ShellComponentId → renderer` eşlemesi; `BaseLayer` bölge-bölge dolaşsın | **YÜKSEK** | `BaseLayer::render` şu an **sabit sırayla** çiziyor (sidebar → tab_bar → dock → stage → notifications) |
| 7 | `src/persist/snapshot.rs:132-151` | `restored_left_panel_preference` → **tam layout** restore | **Orta** | `SNAPSHOT_VERSION` 4→5; geri-uyumluluk (`from_legacy_sidebar_width` :56-61) korunmalı |
| 8 | `src/app/input/shell.rs:148+` | `begin_sidebar_resize` vb. sidebar-özel yardımcılar → genel `DividerId` | **Orta** | `DividerId::new(leading, trailing, axis)` (:16) **zaten genel** ✅; `ResizeTransaction`/`ResizeBounds`/`CollapseDecision`/`ResizeUpdate` hazır |
| 9 | `src/ui/shell/model.rs:9-14` | Sınırlar yeterli mi? (yaprak ≤64, derinlik ≤4, placement ≤64) | **Düşük** | Kullanıcı-tanımlı layout için makul; **değiştirme** |
| — | **PROTOKOL** | **DEĞİŞİKLİK YASAK** | — | `src/ui/shell.rs:12-13` — shell tipleri `protocol`/`api::schema`'ya giremez. Layout **client-local**. Guardrail kilitliyor ✅ |
| — | **Test kancaları** | `compute_regions` · `solve_tracked_for_test` · `visit_count` · `degradation_for_test` · `compute_shell_view_for_test` · `shell_hit_for_test` · `render_prof::observe_for_test` · `AppState::assert_invariants_for_test` · `Workspace::assert_invariants_for_test` · `tests/visual/*.spec.ts` | — | **Bu alan test altyapısı en zengin bölge** — refactor için ideal |

#### Özet karar tablosu

| İş | Risk | Neden |
|---|---|---|
| Built-in template ekle | 🟢 Düşük | Kapalı enum, testli, izole — **ama etkisiz** |
| Kalıcılığı genişlet (yeni alan) | 🟡 Orta | DTO + version bump; doğrulama hazır |
| Layout'u canlı yap | 🔴 Yüksek | compute_view + render + persist + input **dördü birden** |
| Component registry (bölge→renderer) | 🔴 Yüksek | Bugün **hiç yok**; `BaseLayer` sabit sıralı |
| Kullanıcı-tanımlı serbest ağaç (DSL) | 🟡 Orta *(canlı yapıldıktan SONRA)* | Veri modeli + doğrulayıcı **zaten hazır** (`untagged Tree`) |

---

## §F · Mimari borç ve kırılganlık noktaları

| # | Bulgu | Somut sembol + kanıt | Etki | Güven |
|---|---|---|---|---|
| **F1** | **Ölü kol: `PreviewCapability::OptionalPlugin`** | tek üretim çağrısı `preview_capability(path, kind, &PreviewProviderSet::default())` trail_snapshots.rs:704; `plugin_or_fallback` cap.rs:180-196 daima fallback'e düşer; non-default kurulum yalnız cap.rs:321 (test) | Eklenti tabanlı PDF/XLSX önizleme mimarisi **hiç bağlanmamış** — A-1'in ilk blokeri | 0.95 |
| **F2** | **Layout kalıcılığı yazılıyor, okunmuyor** | `ShellSnapshotV1{root, template, component_placements}` snapshot.rs:29-41 ⟷ `restored_left_panel_preference` :132-151 ⟷ `ShellLayout::default()` ui.rs:303 | Kullanıcı layout'u kaydedilir + doğrulanır, **restore edilmez** | 0.95 |
| **F3** | **`ComponentPlacement`/`StackContainer`/`ShellComponentId` tüketicisiz** | grep: yalnız `src/ui/shell/*` + `src/persist/snapshot.rs` | 6 bileşen kimliği tanımlı, hiçbiri bir renderer'a bağlı değil | 0.9 |
| **F4** | **Render purity ihlali (bilinen; kod tabanının kendi itirafı)** | `render_projects_list` içinde `let now = std::time::SystemTime::now();` **`src/ui/sidebar.rs:1279`** — fonksiyonun kendi doc'u (`:1273-1275`) *"Resolves every row's content from the `projects_sessions` cache; never mutates state or reads the disk (CLAUDE.md render purity)"* diyor. İtiraf: `src/ui.rs:1741-1743` *"(The sidebar Projects tab DOES read `SystemTime::now()` for relative timestamps; that sits outside the stage surface and is recorded in the SF4.3 evidence.)"*. Diğer aday: `src/ui/mobile.rs:1386` | Sidebar Projects sekmesi için **byte-özdeş determinizm YOK** → görsel fixture testi yapılamaz | 0.95 |
| **F5** | **`StageState.last_generations: [Option<u32>; 2]` sabit dizi** | `BuiltInAppId::index()` in_degree **118**; `insert_instance` / `next_instance_id` bu diziyle indeksliyor | Üçüncü built-in app **sessiz taşma** riski | 0.85 |
| **F6** | **kitty_graphics tek-görsel varsayımı** | `FILE_MANAGER_PREVIEW_IMAGE_ID = 1` · `FILE_MANAGER_PREVIEW_PLACEMENT_ID = 1` · `FILE_MANAGER_PREVIEW_PANE_RAW = u32::MAX` kitty_graphics.rs:23-25 | İkinci eş-zamanlı piksel yüzeyi kimlik şeması gerektirir | 0.95 |
| **F7** | **Grafik kare aşımı sessiz** | headless.rs:3466-3475 — 32 MiB üstü `warn!` + `frame.graphics.clear()` + `commit_graphics_cache = false`; kullanıcıya sinyal yok | Büyük PDF/görsel = **boş kutu**, sebepsiz | 0.95 |
| **F8** | **`resolve_dynamic` yalnız LeftPanel'i biliyor** | ui.rs:312-318 `if region == RegionId::LeftPanel { sidebar_w } else { 0 }` | Diğer 5 bölge `Dynamic` ile daima 0 → template'ler görünmez | 0.95 |
| **F9** | **`terminal_surface_active` boolean'ı 4 yerde tekrar** | ui.rs:328-329 (tab bar), :424 (split_borders), :439 (pane_infos), :561 (mobil) | Üçüncü yüzeyde kaçırma riski. Karşıt: `BaseLayer` `match` exhaustive ✅ (:844) | 0.9 |
| **F10** | **Dev dosyalar** | `src/app/input/file_manager.rs` **427 KB** · `src/app/mod.rs` 205 KB · `src/app/actions.rs` 205 KB · `src/fm/mod.rs` 195 KB · `src/app/input/mouse.rs` 170 KB · `src/app/state.rs` 154 KB · `src/app/file_operation_worker.rs` 151 KB · `src/app/worktrees.rs` 98 KB · `src/app/api.rs` 80 KB | edit-safety §3: 500+ LOC yapısal refactor öncesi ölü kod temizliği zorunlu; context bayatlama riski çok yüksek | 0.95 |
| **F11** | **Leiden cluster'ları klasör yapısıyla ayrışmıyor** | `get_architecture().clusters`: 8 büyük cluster'ın **hepsi `"src"` etiketli**, cohesion 0.57–0.72; buna karşılık cluster 28 `tests` 0.99, cluster 22 `scripts` 1.00 | De-facto modüller `src/` içinde **tek blob**; `AppState` üzerinden yüksek eşleşme. Runtime/client ayrıştırma hedefi henüz grafiğe yansımamış | 0.7 (saf grafik) |
| **F12** | **`.codex/evidence/miller-scroll-version-lab/` grafiği kirletiyor** | `src/ui.rs`'in 4 tam kopyası indekslenmiş; `compute_view`/`BaseLayer` 5'er kez | Her grafik sorgusu yanlış node döndürebilir. **Öneri: indekslemeden hariç tut** | 0.95 |
| **F13** | **POZİTİF — generation/stale-hit disiplini örnek düzeyde** | bkz. [§H](#h--yeni-yüzeybölge-eklerken-kopyalanacak-invariant-deseni) | Yeni yüzey eklerken **bu deseni kopyala, yeniden icat etme** | 0.95 |

---

## §G · codebase-memory-mcp kullanım reçetesi ve BİLİNEN SINIRLARI

> **Bu bölüm gelecekteki HER agent için kritiktir.** Grafiğe körü körüne güvenen bir agent bu projede yanlış sonuca varır — kanıtları aşağıda.

### G0 · Proje kimliği

```
project: "home-user-projects-herdr"
düğüm/kenar: 24.357 / 129.892   ·   durum: ready
diller: Rust 298 dosya · TOML 44 · Python 26 · TypeScript 20 · YAML 19 · Bash 13
```

### G1 · Doğru sorgu sırası (grep'ten ÖNCE)

```
1. index_status(project)                → taze mi, ready mi?  (HER ZAMAN İLK)
2. get_architecture(project)            → paketler, hotspot'lar, cluster'lar, file_tree
3. search_graph(query= | name_pattern= | semantic_query=)
                                        → sembol bul (qualified_name AL)
4. trace_path(function_name, direction) → çağrı zinciri  ⚠ RUST'TA GÜVENİLMEZ (G3)
5. get_code_snippet(qualified_name)     → TAM qualified_name ile kaynak oku
6. query_graph(cypher)                  → karmaşık sahiplik/toplu listeleme
7. search_code(pattern)                 → grafik-zenginleştirilmiş grep
   ── grep/Read: string/config/non-code İÇİN ve HER DOĞRULAMA İÇİN
```

### G2 · `src/` dışını ELEME (zorunlu)

Bu repoda `.codex/evidence/`, `website/`, `scripts/`, `tests/`, `docs/` de indekslidir. **Her `search_graph` çağrısında daralt:**

```jsonc
// İYİ — dosya yoluna göre daralt
{"project":"home-user-projects-herdr", "file_pattern":"src/ui/surface_host.rs"}

// İYİ — qualified_name desenine göre daralt
{"project":"home-user-projects-herdr", "qn_pattern":"^home-user-projects-herdr\\.src\\.ui\\..*"}

// KÖTÜ — lab kopyalarını da getirir
{"project":"home-user-projects-herdr", "name_pattern":"^compute_view$"}
```

Cypher ile eleme:
```cypher
MATCH (n:Function)
WHERE n.file_path STARTS WITH 'src/'
  AND NOT n.file_path CONTAINS '.codex'
  AND n.name = 'compute_view'
RETURN n.file_path, n.qualified_name, n.signature
```

### G3 · ⛔ SINIR 1 — Rust CALLS kenarları EKSİK (kanıtlı)

**Kanıt A:**
```
mcp__codebase-memory-mcp__trace_path{function_name:"preview_capability", direction:"inbound", depth:4}
→ {"callers":[
     {"name":"preview_capability_classifies_native_metadata_and_unsupported_cases", hop:1},
     {"name":"preview_capability_uses_only_explicit_supported_plugin_providers",     hop:1},
     {"name":"preview_capability_rejects_non_utf8_paths_without_lossy_classification",hop:1}]}
   ↑ ÜÇÜ DE TEST. Üretim çağrısı YOK.

grep -rn "preview_capability(" src/ --include="*.rs" | grep -v "^src/fm/preview_capability.rs"
→ src/fm/trail_snapshots.rs:704:    let preview = match preview_capability(path, kind, &PreviewProviderSet::default()) {
   ↑ GERÇEK ÜRETİM ÇAĞRISI — grafik bunu HİÇ göstermedi.
```

**Kanıt B:**
```
trace_path{function_name:"read_image_preview", direction:"inbound", depth:5}
→ callers: [ …test…, {"name":"src/app/image_preview_worker.rs", qualified_name:"…__file__"} ]
   ↑ Fonksiyon değil DOSYA node'u döndü. Gerçek: image_preview_worker.rs:128

**KURAL:** `trace_path` "çağıran yok" veya "yalnız test" derse → **ASLA ölü kod ilan etme.**
Önce `grep -rn "<fn_name>(" src/ --include="*.rs"` ile doğrula.
Ölü kod iddiası ancak GREP de boşsa yapılır.
```

### G4 · ⛔ SINIR 2 — Token limiti aşımı (kanıtlı)

```
search_graph{file_pattern:"src/ui/shell", limit:200}
→ Error: result (131.211 characters across 1 line) exceeds maximum allowed tokens.
```

**Ne yapılacak (sırayla dene):**
1. `limit`'i düşür (`limit: 25`) + `offset` ile sayfalayarak ilerle (`has_more` alanını izle)
2. `label` ile daralt: `{"label":"Class"}` veya `{"label":"Function"}`
3. `min_degree` ile gürültüyü kes: `{"min_degree": 2}`
4. `query_graph` (Cypher) ile **yalnız gereken kolonları** döndür:
   ```cypher
   MATCH (n) WHERE n.file_path STARTS WITH 'src/ui/shell'
     AND (n:Class OR n:Enum)
   RETURN n.file_path, n.name, n.qualified_name
   ```
5. **Hâlâ patlıyorsa → dosyayı doğrudan `Read` et.** Shell alt sistemi bu turda böyle çözüldü (`model.rs` 11 KB, `view.rs` 6 KB, `template.rs` 4,7 KB, `layout.rs` 17 KB — hepsi tek okumada rahat).

### G5 · ⛔ SINIR 3 — `.codex/evidence/miller-scroll-version-lab/` kirliliği (kanıtlı)

```
search_graph{name_pattern:"^(AppState|Workspace|compute_view|BaseLayer|TileLayout|Compositor|
                            RegionRects|ShellView|StageSurfaceView|ShellModel|ResizeTransaction)$"}
→ total: 22 sonuç. Bunların 8'i lab kopyası:

  home-user-projects-herdr..codex.evidence.miller-scroll-version-lab
      .v0-trail-baseline-3bd32bcf.src.ui.compute_view      (in_degree 81)
      .v1-horizontal-viewport-0f958efe.src.ui.compute_view (in_degree 82)
      .v2-fractional-one-third-84092e52.src.ui.compute_view(in_degree 84)
      .v3-plain-wheel-fallback-6a972703.src.ui.compute_view(in_degree 85)
  + aynı 4 sürüm için BaseLayer

  GERÇEK OLAN:
  home-user-projects-herdr.src.ui.compute_view             (in_degree 113)  ← src/ui.rs:150
```

**Ayırt etme kuralı:** gerçek sembolün `qualified_name`'i `home-user-projects-herdr.src.` ile başlar (nokta-nokta yok). Lab kopyaları `home-user-projects-herdr..codex.evidence.` ile başlar (**çift nokta** — `.codex` gizli dizinden geliyor).

**Kalıcı çözüm önerisi:** `.codex/evidence/miller-scroll-version-lab/` indekslemeden hariç tutulmalı (bu bir kod dizini değil, arşivlenmiş deney anlık görüntüleri).

### G6 · ⛔ SINIR 4 — Cypher takma adları (alias) güvenilmez

```cypher
-- İSTENEN: 5 kolon
MATCH (n) WHERE ... RETURN n.file_path AS file, labels(n)[0] AS kind, n.name AS name, ...
-- ALINAN: 2 kolon, çoğu satır tekrarlı
{"columns":["file","labels(n)"], "rows":[["src/app/preview.rs","[\"Class\"]"], ...], "total":20}
```

**Kural:** Cypher'da **`AS` takma adı kullanma**; doğrudan `RETURN n.file_path, n.name, n.qualified_name, n.signature` yaz. Bu formda 163 satırlık doğru sonuç alındı (önizleme fonksiyon envanteri).

### G7 · Çalışan sorgu örnekleri (bu turdan, kopyala-yapıştır)

```cypher
-- Bir dizindeki tüm üretim fonksiyonları + imzaları (ÇALIŞTI: 163 satır)
MATCH (n:Function) WHERE n.file_path CONTAINS 'preview'
RETURN n.file_path, n.name, n.qualified_name, n.signature
ORDER BY n.file_path, n.name
```

```cypher
-- Hot-path adayları (complexity özellikleri her Function/Method node'unda mevcut)
MATCH (f:Function)
WHERE f.transitive_loop_depth >= 3 OR f.linear_scan_in_loop >= 1
RETURN f.qualified_name, f.transitive_loop_depth, f.linear_scan_in_loop
ORDER BY f.transitive_loop_depth DESC
```

### G8 · Hotspot metriklerinin okunuşu (dikkat: test çağrıları dahil)

| Sembol | fan_in | Yorum |
|---|---:|---|
| `ClientControlWriter::clone` | 1275 | server transport — her client mesajında klonlanıyor |
| `TerminalRuntimeRegistry::iter` | 864 | pane runtime taraması sıcak yol |
| `RotatingFileGuard::write` | 593 | logging |
| **`Workspace::test_new`** | **462** | ⚠ **test altyapısı** — "en çok çağrılan" ≠ "en kritik üretim yolu" |
| `RawInputFramer::push` | 414 | girdi ayrıştırma |
| **`RegionRects::get`** | **374** | shell bölgeleri **zaten geniş tüketiliyor** → iyi bir dikiş |
| `TileLayout::new` | 289 | pane BSP |
| `Compositor::new` | 278 | render katman yığını |

**Kural:** `fan_in` **test çağrılarını da içerir**. Üretim kritikliği için `trace_path(include_tests=false)` ya da doğrudan grep ile teyit et.

### G9 · Altın kural

```
┌──────────────────────────────────────────────────────────────────────┐
│  GRAFİK = NAVİGASYON (nerede bakacağımı söyler)                      │
│  KAYNAK = OTORİTE     (ne olduğunu söyler)                           │
│                                                                      │
│  Grafik "YOK" diyorsa   → grep ile doğrula, kabul etme.              │
│  Grafik "VAR" diyorsa   → dosya:satır ile teyit et.                  │
│  Grafik patlarsa        → sayfalandır → Cypher → Read.               │
│  qualified_name'de `..` → LAB KOPYASI, gerçek değil.                 │
└──────────────────────────────────────────────────────────────────────┘
```

---

## §H · Yeni yüzey/bölge eklerken kopyalanacak invariant deseni

> §F13'ün genişletilmiş hâli. Bu 6 desen herdr'ın "sessiz yanlış davranış" sınıfını kapatan çekirdeğidir. **Yeni bir yüzey, bölge, işçi veya girdi tüketicisi eklerken bunları YENİDEN İCAT ETME — kopyala.**

### H1 · Generation-eşitliği ile pozisyonel otorite (bayat koordinat reddi)

**Kaynak:** `src/ui/shell/view.rs:86-105`

```rust
/// Resolve only a hit from this exact geometry generation. SF4 wires this
/// pure seam into the topmost input router.
pub(super) fn hit_at(&self, generation: u64, position: Position) -> Option<ShellHitTarget> {
    if generation != self.generation {
        return None;                                    // ← BAYAT KOORDİNAT ÖLÜR
    }
    self.hits
        .iter()
        .rev()                                          // ← z-order: son çizilen önce
        .find(|hit| hit.generation == generation && hit.rect.contains(position))
        .map(|hit| hit.target)
}

pub(crate) fn region_hit_at(&self, generation: u64, position: Position) -> Option<RegionId> {
    self.hit_at(generation, position).map(|target| match target {
        ShellHitTarget::Region(region) => region,       // ← exhaustive
    })
}
```

**Neden böyle:** Layout değiştikten sonra gelen bir mouse olayının koordinatı, artık BAŞKA bir bölgeye ait olabilir. Generation eşitliği olmadan tıklama sessizce yanlış hedefe gider.

**Kopyalarken dikkat:**
- Çağıran **daima güncel generation'ı** geçirmeli: `self.view.shell.region_hit_at(self.view.shell.generation, position)` (`src/app/input/shell.rs:98-101`)
- `.rev()` şart — hit listesi çizim sırasında (arkadan öne) doldurulur, en üstteki kazanmalı
- Test kalkanı: `stale_shell_hit_generation_is_rejected` (shell.rs:1269), `collapsed_or_inert_region_cannot_receive_focus` (:1211)

### H2 · Generation tükenmesinde ALIASING yerine FAIL-CLOSED

**Kaynak:** `src/ui/shell/view.rs:139-167`

```rust
fn project_changed_geometry(key, previous_generation, regions, degradation) -> ShellView {
    let Some(generation) = previous_generation.checked_add(1) else {
        // Exhaustion must never alias an older hit generation. Keep the new
        // geometry visible but fail closed with no interactive shell targets.
        return ShellView {
            generation: previous_generation,
            area: key.area,
            regions,
            hits: Vec::new(),          // ← ETKİLEŞİM ÖLÜR, GÖRÜNTÜ YAŞAR
            degradation,
            geometry_key: key,
        };
    };
    let hits = flatten_region_hits(&regions, generation);
    ShellView { generation, area: key.area, regions, hits, degradation, geometry_key: key }
}
```

**Aynı desen ikinci yerde:** `StageState::next_instance_id` — `generation.checked_add(1).ok_or(StageStateError::InstanceGenerationExhausted)` (`src/ui/surface_host.rs`).

**Neden böyle:** `u64` sarması teorik ama sarma olursa ESKİ bir generation ile YENİ geometri aynı numarayı taşır → bayat tıklama geçerli sayılır. Kabul edilemez. Çözüm: etkileşimi kapat, görüntüyü koru.

**Kopyalarken dikkat:** `wrapping_add` **kullanma** — `checked_add` + fail-closed. (Karşı örnek: `ImagePreviewSlot::sync` `wrapping_add(1).max(1)` kullanır — orada aliasing zararsızdır çünkü `accepts()` ayrıca **key eşitliği** de kontrol eder, bkz. H4.)

**Test kalkanı:** `generation_exhaustion_keeps_geometry_but_clears_hit_authority` (shell.rs:1323), `instance_generation_exhaustion_fails_without_aliasing` (surface_host.rs).

### H3 · Exhaustive `match` — derleyiciyi bekçi yap

**Üç örnek:**

```rust
// ① src/ui.rs:844-851 — stage yüzey seçimi
match app.stage.surface_view() {
    surface_host::StageSurfaceView::NativeFiles      => render_file_manager(app, frame, terminal_area),
    surface_host::StageSurfaceView::TerminalWorkspace=> render_panes(app, terminal_runtimes, frame, terminal_area),
}
// → Yeni varyant eklenince DERLEME HATASI. Renderer eklemeyi UNUTAMAZSIN.

// ② src/app/input/shell.rs:123-146 — overlay sahipliği
/// Every mode whose surface is a topmost blocking overlay for mouse and
/// keyboard routing. The match is exhaustive so a new mode must choose a
/// side explicitly instead of silently leaking background input.
pub(crate) fn blocking_overlay_active(&self) -> bool {
    match self.mode {
        Mode::Terminal | Mode::Prefix | Mode::Navigate | Mode::Copy | Mode::Resize => false,
        Mode::Onboarding | Mode::ReleaseNotes | … | Mode::AgentReferencePicker => true,
    }
}
// → `_ => false` YOK. Yeni mod eklemek TARAF SEÇMEYE ZORLAR.

// ③ src/ui/shell/view.rs:102-104 — hit hedefi
.map(|target| match target { ShellHitTarget::Region(region) => region })
```

**Kopyalarken dikkat:** `_ =>` joker kolu **kullanma**. Joker, "yeni varyant eklendi ama burası güncellenmedi" hatasını **derleme zamanından çalışma zamanına** kaydırır — herdr'ın kaçındığı tam da budur.

**⚠ Karşı-örnek (F9):** `terminal_surface_active` bir **boolean**'a indirgenmiş ve `src/ui.rs`'de 4 yerde tekrarlanıyor (`:328-329`, `:424`, `:439`, `:561`). Boolean, derleyici bekçiliğini kaybettirir. Üçüncü yüzey eklerken bu 4 nokta **elle** bulunmalı.

### H4 · Üçlü kimlik doğrulaması + tek-yuvalı işçi (en-son-kazanır)

**Kaynak:** `src/app/image_preview_worker.rs`

```rust
// :14-19  ANAHTAR = üçlü kimlik
struct ImagePreviewKey { path: PathBuf, model_generation: u64, target: ImagePreviewTarget }

// :44-63  YUVA — tek istek, en-son-kazanır + kabul kapısı
impl ImagePreviewSlot {
    fn sync(&mut self, target: Option<ImagePreviewKey>) -> ImagePreviewSync {
        if self.active == target { return ImagePreviewSync::Unchanged; }   // gereksiz iş yok
        self.generation = self.generation.wrapping_add(1).max(1);
        self.active = target;
        if self.active.is_some() { ImagePreviewSync::Started { generation: self.generation } }
        else                     { ImagePreviewSync::Stopped }
    }
    fn accepts(&self, generation: u64, key: &ImagePreviewKey) -> bool {
        self.generation == generation && self.active.as_ref() == Some(key)  // ← ÇİFT KONTROL
    }
}

// :352-371  DURUM UYGULAMA — üç ayrı red kapısı
fn set_image_state(state, key, next) -> bool {
    let Some(file_manager) = state.file_manager.as_mut() else { return false };        // ①
    let FmPreview::File(FmFilePreview::Image(preview)) = &mut file_manager.preview
        else { return false };                                                         // ②
    if preview.source_path != key.path || preview.generation != key.model_generation
        { return false; }                                                              // ③
    if preview.state == next { return false; }    // değişim yok → redraw yok
    preview.state = next;
    true
}
```

**Neden böyle:** Asenkron sonuç geri döndüğünde dünya değişmiş olabilir — kullanıcı başka dosya seçmiş, panel yeniden boyutlanmış, FM kapanmış olabilir. Tek bir kontrol yetmez; **her boyut ayrı doğrulanır.**

**Kopyalarken dikkat:**
- `generation` **sayaç değil kimlik**tir; `sync()` yalnız hedef DEĞİŞTİĞİNDE artırır
- `accepts()` hem generation hem key kontrol eder → `wrapping_add` güvenli olur
- Değişmeyen duruma yazma **`false` döndürmeli** (gereksiz redraw zinciri kırılır)
- Panik sınırı: `catch_unwind(AssertUnwindSafe(...)).unwrap_or(Err(DecoderPanicked))` (`:149-152`) — decoder paniği işçiyi öldürmez
- Zehirlenmiş mutex: `lock().unwrap_or_else(|p| p.into_inner())` (`:253-260`) — `unwrap()` yok
- Ölüm sinyali: `ImageWorkerAliveGuard` (`:105-116`) `Drop`'ta `alive=false` + `notify`
- Telemetri: `render_prof::event("fm.image_worker.{submitted,completed,rejected}")` — reddedilen sonuçlar **sayılır**, sessizce yutulmaz

### H5 · Total-by-construction girdi yönlendirici (fail-closed)

**Kaynak:** `src/app/input/shell.rs:43-63` (tam kod §B6'da)

**Neden böyle:** Girdi sahipliği belirsizse olay **iki yere birden** gidebilir (çift işlem) veya **hiçbir yere** gitmez (kayıp tuş). `route_shell_input` her bağlamı tam bir sahibe eşler; eşleşmeyen durum `FailClosed` — arka plandaki gizli yüzeye **sızmaz.**

**Kopyalarken dikkat:**
- Öncelik sırası **donmuş**: overlay → capture → pozisyonel hit → odaklı bileşen → sayfa kısayolu → global → fail-closed
- Bağlam kurucusu **saf** olmalı: `shell_key_input_owner`/`shell_mouse_input_owner` mutasyon yapmaz
- Yeni bir tier eklerken **hem enum hem router hem her iki dağıtıcı** (`handle_key` + `handle_key_headless`) güncellenmeli
- Erişilemez tier'lar `debug_assert!(false, ...)` ile işaretlenir (`input/mod.rs:121-126`, `:153-155`) — sessiz düşme yok
- Aktif capture **her tuşu tüketmeli**: `debug_assert!(handled, "an active capture must consume every key")` (`:166`)

### H6 · Doğrulanmış-tip (newtype proof) ile geçersiz durumu temsil edilemez kılma

**Kaynak:** `src/ui/shell/model.rs`

```rust
// :106-117  Proof that a shell tree satisfies the finite composition invariants.
pub(crate) struct ValidatedShellLayout(ShellLayout);

// :170  TEK giriş: doğrulamadan geçmeden ValidatedShellLayout ÜRETİLEMEZ
pub(crate) fn validate(self) -> Result<ValidatedShellLayout, ShellValidationError> { … }

// :294-314  Deserialize DOĞRULAYICIDIR — geçersiz JSON tipe hiç dönüşemez
impl<'de> Deserialize<'de> for ShellLayout {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> {
        let serialized = SerializedShellLayout::deserialize(deserializer)?;
        let validated = match serialized {
            SerializedShellLayout::Tree(tree)  => ShellLayout::from_parts(…).validate(),
            SerializedShellLayout::Template(t) => t.template.validated_layout(),
        };
        validated.map(ValidatedShellLayout::into_inner).map_err(de::Error::custom)
    }
}

// layout.rs:51-55  Çözücü YALNIZ doğrulanmış girdi kabul eder
pub(super) fn solve(layout: &ValidatedShellLayout, area: Rect, resolve_dynamic: …) -> SolvedShellLayout
```

**Neden böyle:** Doğrulama bir kez yapılır, sonuç **tip sisteminde taşınır**. Aşağı akıştaki hiçbir fonksiyonun "acaba doğrulanmış mıydı?" diye sormasına gerek kalmaz.

**Kopyalarken dikkat:**
- Doğrulama **tek geçişli ve sınırlı** olmalı (`MAX_SERIALIZED_NODES` ile bağlı iteratif traversal, `:171-216` — özyineleme yok, stack taşması yok)
- Kısmi başarı **yasak**: geçersiz layout `RegionRects::default()` + `TooSmall` döndürür, yarım geometri değil (`shell.rs:113-118`; test `invalid_tracked_layout_fails_closed_without_partial_regions` :1027)
- Getter'lar **total** olmalı: `RegionRects::get` panik atmaz, yok olan bölge `Rect::default()` (`model.rs:324-330`; test `absent_region_returns_empty_rect` :253)

### H7 · Kontrol listesi (yeni yüzey/bölge eklerken)

```
□ H1  Pozisyonel otorite generation-eşitliğine bağlı mı?
□ H2  Generation taşması checked_add + fail-closed mu? (wrapping ise key eşitliği VAR mı?)
□ H3  Yeni varyant exhaustive match'lere düşüyor mu? (`_ =>` joker YOK)
□ H3b Boolean'a indirgenmiş yüzey kontrolü ekliyor muyum? → ETME, enum kullan
□ H4  Asenkron sonuç üçlü kimlikle mi doğrulanıyor? (owner + path/id + generation)
□ H4b catch_unwind + poisoned-mutex into_inner + AliveGuard var mı?
□ H4c Değişmeyen duruma yazma `false` döndürüyor mu? (redraw zinciri kırılıyor mu?)
□ H4d render_prof event'leri (submitted/completed/rejected) eklendi mi?
□ H5  Girdi yönlendirici total mi? Erişilemez tier debug_assert ile işaretli mi?
□ H6  Doğrulama newtype ile taşınıyor mu? Kısmi başarı YASAK mı? Getter total mi?
□ —   Karakterizasyon testi ÖNCE yazıldı mı? (CLAUDE.md refactor-risk kuralı)
□ —   `just check` (fmt + nextest + maintenance) yeşil mi?
```

---

## §I · Bu turda İNCELENMEYEN dikişler

> Bu turun kapsamı "belge yüzeyi + custom layout" ekseniydi. Aşağıdakiler **bilinçli olarak** kapsam dışı bırakıldı. Her biri gelecek bir tur için hazır bir başlangıç noktasıdır.

| # | Dikiş | Neden kapsam dışıydı | Hangi soru için bakılmalı | Başlangıç noktaları |
|---|---|---|---|---|
| **I1** | **Multi-client / multi-monitor projeksiyon** | Tek-client render yolu belge yüzeyi için yeterliydi | "İki istemci farklı boyutta iken belge yüzeyi hangi geometriye göre render edilir? `compute_view_without_resizing_panes` neden var, hangi client foreground sayılır?" **Edit özelliği için ZORUNLU** (paylaşılan tampon = çoklu görünüm) | `research/multi-monitor-shared-view.md` · `src/ui.rs:183` `compute_view_without_resizing_panes` (doc: *"used by the headless server when a non-foreground client needs its own frame size while the shared pane runtimes stay pinned to the foreground client"*) · `src/server/clients.rs` · `tests/multi_client.rs` |
| **I2** | **`src/ui/mobile.rs` — ayrı kompozisyon otoritesi** | Masaüstü dikişi öncelikliydi; mobil `compute_empty_shell_view` ile shell'i **tamamen atlıyor** | "Mobil yolda shell/region modeli neden devre dışı? Yeni yüzey mobilde nasıl görünecek? `is_mobile_width` eşiği neye göre?" ⚠ **Ayrıca F4'ün ikinci render-purity adayı burada** (`mobile.rs:1386` `SystemTime::now()`) | `src/ui.rs:279-282` (`is_mobile_width` → `compute_mobile_view`) · `src/ui.rs:515-640` `compute_mobile_view` · `src/ui/shell/view.rs:124` `compute_empty_shell_view` · `src/ui/mobile.rs` · `ViewLayout::Mobile` kolları ui.rs:817, 826, 875 |
| **I3** | **Plugin sistemi ↔ server API sınırı** | F1'de "sağlayıcı kaynağı yok" tespit edildi ama **plugin altyapısının kendisi** incelenmedi | "`PreviewProviderSet` hangi kaynaktan doldurulmalı? Plugin `action_id` nasıl kaydediliyor, `platform_supported` kim belirliyor? Plugin bir **server** gerçeği mi client sunumu mu?" **A-1 madde 2'nin ön koşulu** | `src/plugin_command.rs` · `src/plugin_paths.rs` · `tests/fixtures/plugin-smoke/herdr-plugin.toml` · `workers/plugin-marketplace/` · `website/src/pages/plugins.astro` · `src/fm/preview_capability.rs:61-72` |
| **I4** | **Agent detection ↔ pane state dikişi** | Belge yüzeyiyle kesişmiyor | "Manifest hot-reload (`herdr server reload-agent-manifests`) hangi state'i geçersizleştiriyor? Detection **bottom-buffer** okuyor (viewport değil) — bu ayrım nerede zorlanıyor? Yeni bir yüzey pane'i gizlerse detection ne olur?" | `src/detect/{mod,manifest,manifest_update}.rs` + `manifests/*.toml` (19 ajan) · `scripts/agent_detection_manifest_check.py` · `website/agent-detection/` · CLAUDE.md "Agent Detection Updates" bölümü · `src/integration/` (14 ajan asset'i) |
| **I5** | **Persist migration zinciri** | `SNAPSHOT_VERSION=4` ve `SHELL_SNAPSHOT_VERSION=1` tespit edildi ama **migration mantığı** izlenmedi | "v1→v2→v3→v4 geçişleri nerede? `from_legacy_sidebar_width` gibi kaç geriye-uyum kolu var? Yeni alan eklerken hangi fixture'lar güncellenmeli?" **E-1/S6 ve E-2/madde 7 için ZORUNLU** | `src/persist/snapshot.rs:16` `SNAPSHOT_VERSION` · `:18` `SHELL_SNAPSHOT_VERSION` · `:347` `match raw.shell` · `:56-61` `from_legacy_sidebar_width` · `tests/fixtures/session/{current-herdr-session.json, current-herdr-dev-session.json, legacy-pre-tabs-v2.json}` |
| **I6** | **Handoff / detach-reattach yolu** | Render dikişine değmiyor gibi görünüyordu — **ama edit için kritik** | "Detach anında kaydedilmemiş belge tamponu ne olur? `handoff_runtime` hangi state'i taşıyor, hangisini bırakıyor? Live handoff sırasında `graphics_cache` ne oluyor?" | `src/handoff_runtime.rs` · `src/server/handoff.rs` · `src/server/terminal_attach.rs` · `tests/detach_reattach.rs` · `tests/live_handoff.rs` · `scripts/smoke_live_handoff_sessions.sh` · `src/server/clients.rs:126` `graphics_surface_reset_pending` |
| **I7** | **`src/api/` + `src/app/api.rs` JSON API yüzeyi** | Protokol (wire) tarafı incelendi, **JSON API şeması** incelenmedi | "Bir belge yüzeyi API'de görünmeli mi? `herdr agent read/explain` gibi CLI komutları yeni yüzeyi nasıl görür? `docs/next/api/herdr-api.schema.json` nasıl güncellenir?" **A-3/madde 18 kararının ikinci yarısı** | `src/api/schema/` · `src/app/api.rs` (2.046 sat) · `src/app/api_helpers.rs` · `docs/next/api/herdr-api.schema.json` · `tests/api_ping.rs` · `src/cli.rs` + `src/cli/` |
| **I8** | **Vendored libghostty-vt ↔ Kitty ayrıştırma derinliği** | `KittyImageDescriptor`/`KittyImagePlacement` tip sınırında durduk | "PTY passthrough görselleri hangi VT durumunda tutuluyor? `hide_kitty_placeholders` neyi gizliyor? Yeni bir piksel yüzeyi bu ayrıştırıcıyla çakışır mı?" | `src/ghostty/` · `vendor/libghostty-vt.vendor.json` · `vendor/libghostty-vt.patches.md` · `vendor/patches/libghostty-vt/` · `src/pane/terminal.rs:1106, 1746, 2005` |
| **I9** | **`render_prof` telemetri sistemi** | Sayaç isimleri toplandı, **toplama/raporlama mekanizması** izlenmedi | "Profil verisi nereye gidiyor, nasıl okunuyor? Yeni yüzey için hangi sayaçlar eklenmeli? `observe_for_test` dışında canlı gözlem var mı?" | `src/render_prof.rs` · `duration_guard` / `event` / `counter` / `timer` / `duration_since` / `observe_for_test` kullanımları |
| **I10** | **`src/app/actions.rs` (205 KB) eylem yüzeyi** | Girdi yönlendirme tier'ları incelendi; **eylem gövdeleri** incelenmedi | "Yeni bir yüzey hangi eylemleri kaydetmeli? Eylem ↔ keybinding ↔ menü üçlüsü nerede birleşiyor?" | `src/app/actions.rs` · `src/server/keybindings.rs` · `src/ui/{menus,keybind_help}.rs` · `src/app/input/modal.rs` |

**Öneri sırası (belge yüzeyi + custom layout hedefi için):**

```
1. I3  (plugin ↔ API)        → A-1'in blokerini açar
2. I5  (persist migration)   → hem A-S6 hem B-2/7 için ön koşul
3. I2  (mobile)              → yeni yüzeyin ikinci render otoritesi
4. I1  (multi-client)        → EDIT kararının (A-3/18) teknik temeli
5. I7  (JSON API)            → EDIT kararının ikinci yarısı
6. I6  (handoff)             → EDIT dayanıklılığı
```

---

## §J · Kapanış ve dosya yolu envanteri

### J1 · Tek cümlelik özet

**Piksel tarafı hazır, layout tarafı boş.** İkili görsel baytlar için sunucudan istemciye giden yol (`FrameData.graphics` → `insert_graphics_before_sync_end` → `\x1b7…\x1b8`) **opak, genel amaçlı ve protokol değişikliği gerektirmiyor**; `render_ansi.rs` grafiğe hiç dokunmaz, splice kodlayıcının **dışında** yapılır. Buna karşılık custom layout için veri modeli / doğrulayıcı / çözücü / kalıcılık / etkileşim reducer'ları **tamamı yazılmış ve testli, ama `src/ui.rs:303`'teki tek satırlık `ShellLayout::default()` yüzünden hiçbiri canlı yola bağlı değil**. Belge **düzenleme** ise her ikisinden de farklı bir sınıf: içerik yazma yolu sıfırdan kurulmalı ve *"tampon paylaşılan runtime gerçeği mi?"* sorusu CLAUDE.md guardrail'i uyarınca **kod yazılmadan önce** karara bağlanmalı — cevabı "evet" ise `PROTOCOL_VERSION` 16→17 kaçınılmaz.

### J2 · Dosya yolu envanteri (mutlak)

**Render / UI çekirdeği**
- `/home/user/projects/herdr/src/ui.rs` (3.324 sat) — `compute_view*`, `render*`, `BaseLayer`, `OverlayLayer`
- `/home/user/projects/herdr/src/ui/compose.rs` (133) — `Compositor`, `Component`, `RenderCtx`
- `/home/user/projects/herdr/src/ui/shell.rs` (1.554) — modül kökü, `ShellLayout::default`, `compute_projection`
- `/home/user/projects/herdr/src/ui/shell/model.rs` (351) — `RegionId`, `TrackPolicy`, `ShellLayout`, `ValidatedShellLayout`, `RegionRects`
- `/home/user/projects/herdr/src/ui/shell/layout.rs` (593) — solver, `ResponsiveDegradation`, degradasyon sırası
- `/home/user/projects/herdr/src/ui/shell/template.rs` (155) — `ShellTemplateId` (5 kapalı template)
- `/home/user/projects/herdr/src/ui/shell/view.rs` (205) — `ShellGeometryKey`, `ShellView`, `hit_at`, generation disiplini
- `/home/user/projects/herdr/src/ui/shell/interaction.rs` (1.516) — resize/collapse/scroll reducer'ları
- `/home/user/projects/herdr/src/ui/surface_host.rs` — `StageState`, `StageSurfaceView`, `BuiltInAppId`
- `/home/user/projects/herdr/src/ui/sidebar.rs` — ⚠ F4 render-purity ihlali `:1279`
- `/home/user/projects/herdr/src/ui/mobile.rs` — ⚠ ikinci purity adayı `:1386` (I2)
- `/home/user/projects/herdr/src/ui/app_dock.rs` — shell.regions tüketen tek doğru örnek
- `/home/user/projects/herdr/src/ui/visual_fixture.rs` (1.523) — `export_cell_fixture`
- `/home/user/projects/herdr/src/ui/file_manager.rs` (3.799)
- `/home/user/projects/herdr/src/ui/file_manager/{locations,miller,trail_view}.rs`

**Grafik**
- `/home/user/projects/herdr/src/kitty_graphics.rs` (2.075)
- `/home/user/projects/herdr/src/ghostty/` · `/home/user/projects/herdr/vendor/libghostty-vt.*`

**File Manager**
- `/home/user/projects/herdr/src/fm/{mod,preview_capability,image_preview,text_preview,trail_snapshots,trail,miller,operations,rename,delete,watcher,entry_kind,entry_time,natsort}.rs`

**App / state / işçiler**
- `/home/user/projects/herdr/src/app/state.rs` (4.118) · `mod.rs` (5.180) · `actions.rs` (5.775) · `runtime.rs` (1.087)
- `/home/user/projects/herdr/src/app/{image_preview_worker,file_preview_worker,file_operation_worker,file_manager_io_worker,file_manager_watcher}.rs`
- `/home/user/projects/herdr/src/app/input/{shell,mod,file_manager,mouse,modal,navigate,sidebar,terminal,copy_mode,overlays,selection,settings}.rs`

**Server / protokol / client**
- `/home/user/projects/herdr/src/server/{headless,render_stream,clients,handoff,terminal_attach,keybindings,socket_paths,clipboard_image}.rs`
- `/home/user/projects/herdr/src/protocol/{wire,render_ansi,mod}.rs`
- `/home/user/projects/herdr/src/client/mod.rs`
- `/home/user/projects/herdr/src/persist/snapshot.rs`
- `/home/user/projects/herdr/src/api/` · `/home/user/projects/herdr/src/app/api.rs`

**Test**
- `/home/user/projects/herdr/tests/{client_mode,multi_client,server_headless,detach_reattach,live_handoff,cross_area,api_ping,auto_detect,cli_wrapper}.rs`
- `/home/user/projects/herdr/tests/visual/` (Playwright: 10 spec + harness + fixtures)
- `/home/user/projects/herdr/tests/fixtures/session/` (3 snapshot fixture'ı)

### J3 · Analiz hijyeni beyanı

Bu analiz **salt-okuma** yapılmıştır. Yapılmayanlar: kaynak dosya değişikliği · git mutasyonu (commit/branch/push) · `index_repository` (grafik yeniden indeksleme) · herdr server veya socket'e erişim · `.superpowers/` açma · `.cartography/` altına yazma · `docs/references` veya `docs/patterns` altına yazma. **Tek yazma:** bu dosya (`docs/analysis/2026-07-24-architecture-seams.md`), ki `.gitignore:10 /docs/*` ile **ignored**tır (doğrulandı: `git check-ignore -v` → `.gitignore:10:/docs/*`) → lokal yaşar, upstream'e sızmaz.

---

*herdr mimari dikiş analizi · 2026-07-24 · HEAD `b48bd903` · branch `feat/native-fm`*
