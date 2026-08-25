---
doc: herdr-references-remote-graphics-transport
domain: remote-graphics-transport
created: 2026-08-25
status: canonical — her satir kaynak + confidence tasir (evidence-propagation uyumlu)
pattern_id_space: RG1-RG9 (anti-pattern RGA1-RGA5)
related:
  - .local/research/kitty-libghostty-index.md        # kitty spec + libghostty + ghostling #13 + §8 o=z dogrulamasi
  - .local/research/terminal-graphics-landscape.md   # 8+ proje manzarasi (TUIOS, carbonyl, browsh, zellij...)
  - .local/research/browser-priorart-index.md        # terminal-browser / terminal-code feature indeksi
  - .local/prd/s32-wave.md                           # SR3.11-R3.14 olcumler + tasarim
  - .local/measure/                                  # olcum aletleri (kitty_meter.py + T/P kosumlari)
  - docs/references/remote-media-transport.md        # kardes domain: ses/goruntu AKISI (RM1-RM10)
---

# Uzak Istemciye Grafik Tasima — Referans Registry

Kardes domain ayrimi: `remote-media-transport` **akis** (ses/video, zaman damgasi, jitter) sorusunu
kapsar; bu dosya **kare/goruntu** tasimasini (kitty graphics protocol, sikistirma, yerlestirme).

## RG1 — Uzak istemcide yalniz `t=d` calisir (KAPALI)
kitty spec: uzak istemciler dosya/temp/shared-memory mediumlarini kullanamaz, pixel datayi
DOGRUDAN gondermek zorundadir. Kaynak: kitty graphics-protocol resmi dokumani (official, conf 0.95).
**herdr karsiligi (OLCULDU 2026-08-25):** `client/mod.rs:746-748` SSH_CONNECTION/SSH_TTY/TMUX/STY
gorunce AppDirectGraphics istemez → `direct-kitty` dosya-frame yolu kapanir → inline `t=d` yasar.
Kullanicinin CANLI kurulumunda dogrulandi (`pane.graphics.info` → transport alani YOK). Kapi
**spec-dogru**; kaldirilmasi degil, ucuzlatilmasi gerekir.

## RG2 — Ghostty `o=z` (zlib) destekler (KAPALI)
`graphics_command.zig`: `Compression = {none, zlib_deflate}`, `'z' => .zlib_deflate`;
`graphics_image.zig`: `decompressZlib()` + RFC1950 zlib. Chunk'li akista cozme `complete()` icinde,
TUM chunk'lar biriktikten SONRA. Decompress tavani 400 MB. (source_code, conf 0.95)
kitty ✅ · Ghostty ✅ · WezTerm/foot/iTerm2 ⚠️ dogrulanmadi (conf 0.45) → statik tablo yerine
`a=q` probe + sikistirmasiz fallback onerilir.

## RG3 — Sikistirma TUM yuke uygulanir, chunk basina DEGIL (KAPALI)
Dogru sira: `raw → zlib → base64 → 4KB chunk`. Kontrol anahtarlari yalniz ILK chunk'ta.
`S=` yalniz PNG+sikistirma birlesiminde zorunlu; `f=32+o=z` icin gerekmez. (official, conf 0.9)
**Bekci:** `compression_covers_the_whole_payload_before_chunking` (M2' mutanti bu testi oldurur).

## RG4 — Sikistirmasiz RGBA'nin gercek bedeli (OLCULDU)
Laboratuvar (uzak mod, kullanicinin topolojisi): terminal-browser pane'inde **tik basina 25.0 MB**;
kare `a=t t=d f=32 s=1411 v=1739 q=2 m=1`, `o=` YOK. Idle 0 B/s (RD doktrini calisiyor).
Alet: `.local/measure/kitty_meter.py` (executable, conf 0.95).

## RG5 — zlib kazanci: 147x (L1) / 508x (L6) / PNG 288x (OLCULDU)
Gercek yakalanan kare (9.4 MB piksel): L1 66.6 KB @ 3 ms · L6 19.3 KB @ 21 ms · PNG 34.1 KB.
**Secim L1**: kare-basina-istemci yolunda L6'nin ekstra 3.5x'i 7x CPU ister (RD doktrini).

## RG6 — Sikistirma encode'u YAVASLATMAZ, HIZLANDIRIR (OLCULDU — sezgiye aykiri)
herdr'in kendi profilcisi (`full_render.graphics_encode`, ikisi de RELEASE build):
**11.19 ms → 3.47 ms ortalama** (p95 33.55 → 16.78). Sebep: 9.4 MB base64 isi 117 KB'a duser.
⚠️ ILK OLCUM GECERSIZDI (debug vs release karsilastirmasi) — raporlanmadan yakalandi; optimizasyon
seviyesi karistirilmis bir CPU kiyasi sonucu 11x TERS gosterir.

## RG7 — Bir kez ilet, cok kez yerlestir (ACIK — sonraki is)
kitty `i=`/`I=` + placement `p=` ile ayni goruntu yeniden gonderilmez; pane tasima/resize/scroll
sifir yeni bayt uretmeli. zellij 0.45.0 (2026-08-20) bunu yapiyor; TUIOS "image ID reuse →
flicker-free video" diyor. (conf 0.85) **herdr'da olculmedi.**

## RG8 — Damage-rect / partial frame (ACIK — sonraki is)
kitty `a=f` + (x,y,s,v) + `c` compose ile yalniz degisen dikdortgen. herdr API'si zaten
`file_frame_damage: true` ilan ediyor ama inline yolda kullanilmiyor. Carbonyl damage-rect +
Chrome shared-mem bitmap ile ~49x kazaniyor (taraflı kaynak, conf 0.55).

## RG9 — Adaptive frame skipping (ACIK — model referansi)
Mosh SSP: byte-stream degil EKRAN STATE'i senkronlar, ara kareleri ATLAR ("does not need to send
every byte it receives"). kitty dokumani SSH'te client-driven animasyonun yetersiz oldugunu ACIKCA
soyler. Lisans: Mosh GPL-3 → yalniz tasarim referansi. (conf 0.85)

## Anti-pattern'ler
| ID | YAPMA | NEDEN |
|---|---|---|
| RGA1 | Chunk basina sikistirmak | Terminal `complete()`'te tek akis bekler → `DecompressionFailed` |
| RGA2 | `q=2` ile sikistirmayi bring-up'ta birlikte acmak | Cozme hatasi SESSIZCE yutulur → bos kare, hata yok (conf 0.8) |
| RGA3 | Kucuk yuku sikistirmak | Kare-basina CPU, kazancsiz (esik 64 KB) |
| RGA4 | Terminal destegini statik TERM tablosuyla varsaymak | Uzakta TERM yanlis tasinir; `a=q` probe + fallback |
| RGA5 | CPU kiyasini farkli optimizasyon seviyeleriyle yapmak | RG6'da 11x ters sonuc uretti |

## Lisans kapisi
kitty (GPL-3) · timg (GPL-2) · mosh (GPL-3) · chafa/browsh (LGPL) → **yalniz tasarim referansi**.
Kod alinabilir havuz: ratatui-image (MIT) · zellij (MIT) · TUIOS (MIT) · carbonyl (BSD-3) ·
terminal-browser / terminal-code (MIT).
