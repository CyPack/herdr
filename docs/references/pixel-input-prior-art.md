# Referans — terminal içinde piksel-hassas girdi (PIX-1 araştırması)

**2026-08-27 · kaynak: `~/.cartography/refpool` (2799 indeksli repo) + vetli tb klonu**

| etiket | kaynak | tier | conf | bulgu |
|---|---|---|---|---|
| `[tb-engine]` | `~/.tmp-research/terminal-browser/engine/crates/pixel-core` | source_code (vetli klon) | 0.95 | tb, `terminal.rs:361`'de `?1003h ?1006h ?1016h` açar; `mouse_pixels = !wrapper.relayed() && probe_mouse_pixels()` (DECRQM `?1016$p`). herdr modunda `herdr.rs:102 mouse_position_px` — `pixel_mouse` false ise gelen sayıyı hücre sayıp kendi merkezine çevirir. Yani **iki taraf da 1016'ya bakar**; herdr hücre gönderirse çözünürlük kalıcı olarak hücreye kilitlenir. |
| `[tb-herdr-transport]` | aynı, `herdr.rs:57` | source_code | 0.95 | tb, `pane.graphics.info` yanıtında `file_frame_transport == "direct-kitty"` ARAR; yoksa hızlı dosya-frame yolundan vazgeçip PTY escape'e düşer. herdr bu alanı yalnız `direct_graphics_available` iken doldurur → SSH'ta kapalı (C2). |
| `[waytermirror]` | refpool `waytermirror` (MIT) | source_code | 0.85 | Wayland ekranını terminale aynalar + **çift yönlü girdi**. Terminal fare raporunu HİÇ kullanmaz: istemcide `libinput_event_pointer_get_dx/dy` ham delta, sunucuda `uinput` `EV_ABS`/`ABS_X/ABS_Y` mutlak konum. Piksel hassasiyeti tam, bedeli uinput izni + terminal-dışı bağımlılık. **Kontrast dersi:** terminal içinde kalan bir üründe DECSET 1016 tek yoldur. |
| `[vt-player]` | refpool `vt` | source_code | 0.8 | Protokol seçim zinciri kitty → sixel → halfblock → ascii. herdr'ın grafik düşüş zinciriyle aynı sıra. |
| `[kgp-pool]` | refpool konsept `kitty-graphics` | index | 0.9 | 149 repo / 1027 isabet; en yoğun uygulamalar `broot` (`src/kitty/image_renderer.rs`), `rio` (`kitty_graphics_protocol.rs`, `kitty_virtual.rs`), `f4` (Go). |

## Karar kaydı
DECSET 1016 yolunu terk edip libinput/uinput'a geçmek **reddedildi**: herdr terminal içinde yaşayan
bir üründür, uinput ayrıcalığı ve terminal-dışı bağımlılık ürünün varlık sebebine aykırı.
Doğru çözüm 1016 yolunu **her taşımada** (SSH dahil) açık tutmaktır → PIX-1 F1/F2.


---

## Ek — kullanıcının 7 kaynağı (2026-08-27, WebFetch + havuz klonu; tarih/kitty metadata ZORUNLU alan)

| etiket | kaynak | son güncelleme | kitty ilişkisi | bulgu/desen |
|---|---|---|---|---|
| `[terminal-code]` | github.com/zenbu-labs/terminal-code (klonlandı) | **2026-08-24** (yaş 2g) | KGP şart; Windows'ta yok → WSL | VS Code'u terminale koyar = tb'nin kardeşi; code-server + terminal-browser bileşimi. `--ssh` ile uzak backend — herdr'ın "uzak agent + yerel görüntü" senaryosuyla aynı ihtiyaç |
| `[tuios]` | opensourceprojects.dev/post/tuios + havuz klonu | **2026-08-27** (yaş 0g! aktif) v0.7.0 | KGP tam (shm+base64, `mpv --vo=kitty` çalışır) · CSI-u push/pop/query · OSC 66 deneysel | Go + Bubble Tea v2; BSP tiling; **olay-odaklı render (düşük CPU)** = herdr resource-doctrine ile aynı ilke; JSON kontrol protokolü = herdr API'sinin muadili. Protokol ipuçları (meta tarama): unicode-placeholder, kgp-animation, kgp-file, kgp-shm, kgp-zlib, decset-1016 — **havuzdaki en zengin KGP uygulaması** |
| `[tuitter]` | github.com/bddicken/tuitter (klonlandı) | **2026-04-05** (yaş 143g) | kitty görüntü modu opsiyonel (`X_IMAGE_MODE: auto/kitty/off`) | OpenTUI (TS/Bun) X istemcisi; görüntü modunu **kullanıcı bayrağıyla** düşürme deseni |
| `[kgp-spec]` | sw.kovidgoyal.net/kitty/graphics-protocol | canlı spec | — | **SSH/uzak için resmî öneri: `t=d` (direct, 4096B chunk)** — dosya/shm uzak istemciye kapalı; bu, herdr'ın SSH'ta direct-graphics'i kapatmasının spec-uyumlu olduğunu doğruluyor (C2 analizimizle örtüşür). Z-index negatif = metin altı; `o=z` = RFC1950 zlib; unicode placeholder U+10EEEE tmux/vim içinden geçer; animasyon `a=f` |
| `[remux]` | github.com/h3nock/remux (klonlandı) | **2026-08-24** (yaş 3g) | Ghostty çekirdeği (iOS) | iOS'tan uzak tmux; SwiftUI + SSH Citadel; **uzun-basış/sürükleme ile imleç** = mobil fare eşleme deseni (herdr-web mobil için ilgili) |
| `[kitty-0.48]` | changelog (0.46.2→0.48.2 farkı) | 0.48.2 güncel | — | 0.48: KGP **transient usage hint** (kısa ömürlü görüntü ipucu — tb/herdr frame'leri için doğal aday!); odaksız pencereye ilk tık artık **hem odaklar hem iletir** (bizim ilk-tık-odak sorunumuzun terminal-katmanı paraleli); dikey sekmeler; shader duraklatma. 0.47: drag-and-drop kitten (SSH üzerinden bile), OSC 9;4 progress, config auto-reload |
| `[tb-how]` | terminal-browser README "how does it work" | **2026-08-24** (yaş 2g, 171 commit) | KGP şart (Ghostty/kitty/cmux/VSCode) | Electron **offscreen rendering** → GPU'dan piksel; UI+içerik tek canvas (Rust motor + React); terminalin veremediği olaylar için **arka planda Swift yardımcı uygulaması** (macOS) — yani tb bile salt-terminal girdisine güvenmiyor; `--ssh` yalnız AĞ trafiğini uzağa taşır, site YERELDE çalışır (herdr modelinin tersi!) |

### Kalıcılaştırılan altyapı (kullanıcı talimatı: "tarih + kitty sürümü MUTLAKA")
`~/.cartography/refpool-meta.py` → `refpool-meta.json`: her repo için `last_commit`, `age_days`,
`remote`, `kitty_versions_mentioned`, `protocol_hints` (unicode-placeholder/kgp-shm/kgp-file/
kgp-zlib/kgp-animation/decset-1016/csi-u), `scanned_at`. Tam havuz taraması arka planda; yeni repo
klonlanınca `refpool-meta.py <repo>` çağrılır.

### herdr için çıkarımlar
1. **tuios** KGP uygulama derinliğinde referans (shm+base64+animation+placeholder) — grafik
   regresyonlarında karşılaştırma hedefi.
2. **kitty 0.48 "transient" ipucu**: herdr'ın client'a bastığı kısa ömürlü frame'ler için
   değerlendirilmeli (host kitty ≥0.48'te bellek/temizlik kazancı) — backlog adayı.
3. **tb'nin Swift yardımcısı**: salt-terminal girdi kanalının sınırı bilinçli bir tasarım sorunu;
   herdr'ın PIX-1'i (1016 uçtan uca) bu sınırı terminal İÇİNDE kalarak iten doğru hamle.
4. **`--ssh` modeli farkı**: tb sayfayı YERELDE çalıştırır (yalnız ağ uzak); herdr sayfayı UZAKTA
   çalıştırıp görüntüyü taşır. İki modelin maliyet profili farklı — değerlendirme havuzuna eklenecek
   karşılaştırma ekseni.
