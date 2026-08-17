---
doc: herdr-pattern-catalog
domain: native-file-manager
created: 2026-07-13
status: canonical — her pattern kaynak-repo:dosya + confidence taşır
agentic_triggers:
  - "miller column · çok-kolon · parent current preview · file list layout"
  - "image preview · kitty placement · unicode placeholder · thumbnail · gfx cache"
  - "file manager pattern · nasıl yapılmış · hangi teknik"
related:
  - docs/references/native-file-manager.md         # kaynak registry + örnek havuz
  - .local/prd/native-file-manager-DECISION.md      # karar + plan (§2 image Path β)
  - .local/prd/native-fm-BACKBONE-ARCHITECTURE.md   # katman haritası
example_pool: ~/.cartography/refpool/               # kod okuma (codebase-mcp indexli)
---

# Pattern Kataloğu — native-file-manager

> Örnek-proje havuzundan (`~/.cartography/refpool/`) damıtılmış, kaynak-koddan doğrulanmış teknikler.
> Her pattern: NE · NE ZAMAN KULLAN · NE ZAMAN KULLANMA · kaynak · confidence. Uygulamadan ÖNCE ilgili
> patterni oku + kaynağı codebase-mcp ile aç.

## [P1] Miller çok-kolon layout (parent/current/preview) · conf 0.95
- **NE:** dizin gezginini yatay N-kolona böl (sol=parent, orta=current, sağ=preview). Ratio-tabanlı esnek.
- **KAYNAK:** `joshuto/src/ui/views/tui_folder_view.rs` — `constraints: &[Constraint; 3]` + `Layout::default().direction(Direction::Horizontal)`; **`Constraint::Ratio(0, _)` = o kolonu GİZLE** (dinamik aç/kapa). codebase-mcp: `home-user-.cartography-refpool-joshuto`.
- **NE ZAMAN:** klasik ranger/miller deneyimi (bir üst + mevcut + önizleme). herdr'da L1 TileLayout/ratatui Layout ile birebir ifade edilir.
- **NE ZAMAN KULLANMA:** tek-panel dar sidebar yeterliyse (Constraint[1]). Aşırı-dar terminalde parent'ı Ratio(0) ile gizle.

## [P2] Image'ı preview-kolonunun İÇİNE gömme · conf 0.9
- **NE:** görsel önizlemeyi ayrı overlay değil, preview-kolonu Rect'inin içine yerleştir (offset ile).
- **KAYNAK:** joshuto preview kolonu — image `image_offset`/preview-rect ile kolonun içine çizilir (kitty/sixel).
- **NE ZAMAN:** miller 3-kolonda sağ kolon hem metin hem görsel preview gösterecekse.
- **herdr eşlemesi:** preview-rect'i hesapla → L2 `KittyImagePlacement.render.{grid_cols,grid_rows,viewport_col,viewport_row}` bu rect'e map et (DECISION §2 Path β).

## [P3] ⭐ Unicode-placeholder virtual placement (MULTIPLEXER-SAFE image) · conf 0.95
- **NE:** ham Kitty APC escape yerine `a=T,U=1,f=32,t=d` ile "virtual placement" oluştur + hücrelere `U+10EEEE` unicode-placeholder karakterleri (renk-kodlu image-id) yaz. Karakterler NORMAL METİN → multiplexer anlamadan taşır → görsel doğru yerde kalır.
- **KAYNAK:** `ratatui-image/src/protocol/kitty.rs:157-271` (kaynak-koddan doğrulandı). codebase-mcp: `home-user-.cartography-refpool-ratatui-image`.
- **NE ZAMAN:** herdr bir MULTIPLEXER olduğu için image'ın pane/kompozisyon içinde stabil kalması gerektiğinde. Zellij bunu çözememiş → herdr'ın kitty_graphics'i zaten pin/virtual destekliyor (herdr-existing-fm map).
- **NE ZAMAN KULLANMA:** herdr host'a re-emit ederken absolute placement de yeterli (kitty_graphics.rs:581-631 zaten normalize ediyor) — virtual placeholder ÇOCUK→herdr yönünde kritik, herdr→host yönünde değil.
- **ANTI-PATTERN:** viuer/viu "dump" (ham escape bas) → multiplexer'da kaybolur. Bunu KULLANMA.

## [P4] Per-slot Gfx protocol cache (image'ı her frame yeniden gönderme) · conf 0.9
- **NE:** her preview-slot için yüklenmiş image-id'yi cache'le; içerik değişmedikçe upload'ı TEKRARLAMA (sadece display).
- **KAYNAK:** `rat-commander/src/ui/graphics/mod.rs` (Gfx katmanı, per-slot cache). codebase-mcp: `home-user-.cartography-refpool-rat-commander`.
- **NE ZAMAN:** preview sık redraw olduğunda (performans). **herdr'da ZATEN VAR:** kitty_graphics `sources: HashMap<(PaneId,u32),u32>` + image/placement signature dedup (kitty_graphics.rs:267-330) → aynı desen. Yerel-preview placement'ı bu cache'e sentetik-key ile eklemek yeter.

## [P5] StatefulProtocol / Picker auto-detect · conf 0.85
- **NE:** terminal protokolünü otomatik tespit edip (kitty/sixel/iterm2/halfblocks) uygun encoder seç; `StatefulProtocol` resize'da yeniden-encode eder.
- **KAYNAK:** `ratatui-image` Picker/StatefulProtocol API.
- **NE ZAMAN:** standalone ratatui app. **herdr için:** protokol-tespit katmanını KULLANMA (herdr'ın kendi encoder'ı + host'u sabit) — sadece `image` decode/resize + rect→grid geometri eşlemesini ödünç al (DECISION §3).

## [P6] İki-aşamalı preview (preload + peek + cache) · conf 0.9
- **NE:** (a) PRELOAD: liste gezinirken arka planda decode→resize→disk-cache; (b) PEEK: seçimde cache'ten oku, 30ms debounce.
- **KAYNAK:** yazi (`yazi-preview` cartography map): `Image::precache` + `ya.file_cache`, twox128 key.
- **NE ZAMAN:** büyük dizinlerde akıcı preview. **herdr:** L3 tokio task'ı olarak; cache key = path-hash.

## [P7] ratatui + ratatui-image + chafa-fallback glue · conf 0.9
- **NE:** ratatui app'e image widget'ı bağlama + terminal desteklemezse chafa(ASCII)-fallback.
- **KAYNAK:** `yeet` (ratatui+ratatui-image+chafa+tokio+Lua). codebase-mcp: `home-user-.cartography-refpool-yeet`.
- **NE ZAMAN:** canlı entegrasyon deseni referansı (widget lifecycle + fallback). herdr'da fallback = halfblocks/ASCII.

## ⚠️ KANITLI BOŞLUK = FIRSAT
ratatui'de **N≥3 cascading Finder-tarzı miller browser YOK** (repo-hunter taradı). joshuto 3-kolon (sabit).
→ herdr-native FM gerçek esnek-kolon + agent-entegrasyonu ile özgün değer katar.

## Anti-pattern'ler (YAPMA)
| Anti-pattern | Doğru |
|---|---|
| viuer/viu "dump" ham-escape image | P3 unicode-placeholder (mux-safe) |
| README "miller" iddiasına güven (tui-file-explorer) | KAYNAK KODU oku — repo-hunter doc-vs-kod çelişkisi buldu |
| ratatui-image protokol-tespit katmanını herdr'a taşı | Sadece decode/geometri ödünç al (encoder herdr'da) |
| Image'ı her frame yeniden upload | P4 per-slot cache (herdr dedup zaten var) |

## Ölçek / karar matrisi
| Durum | Pattern |
|---|---|
| Dar sidebar (tek kolon) | Constraint[1], preview yok |
| Klasik FM (list+preview) | P1 (2-3 kolon) + P2 (image-in-column) |
| Zengin spf-deneyimi | P1 (3-kolon) + P2 + P3 (image) + P6 (preload) |
| Image herdr-native | DECISION §2 Path β + P3 + P4 (herdr encoder reuse) |
| Image yazi-in-pane (fallback) | Path α (TERM-probe riski — spike ölç) |

---
*Kaynak: örnek havuz (~/.cartography/refpool, codebase-mcp indexli) + repo-hunter + 6 cartography map. 2026-07-13.*
