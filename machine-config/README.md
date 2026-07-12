# machine-config — Ayaz workstation kurulum kaydı (LOCAL branch)

> **Bu branch (`local/machine-config`) master'a ASLA merge edilmez, PR açılmaz.**
> Amaç: makine yeniden kurulumunda herdr çalışma düzenini dakikalar içinde geri getirmek.
> Master ilerledikçe bu branch master üstünde rebase/merge ile taşınır.
> Durum: **2026-07-12 CANLI DOĞRULANDI** — super+t/w/1..9 herdr'da çalışıyor (kullanıcı onayı).

## ⌨️ Fiziksel tuş gerçeği (M96BT2 klavye — HER kısayol kararının temeli)

Klavye **Mac layout modunda** (FN+A) — firmware Alt↔Super TAKASLI (libinput kernel kaydıyla kanıtlı):

| Fiziksel tuş | Firmware'in gönderdiği | Yani |
|---|---|---|
| **Alt** yazan tuş | `KEY_LEFTMETA` | **SUPER** |
| **Win/logo** tuşu | `KEY_LEFTALT` | **ALT** |

Kullanıcı "Alt+T" basıyorum derken sisteme **super+t** gider. Bu takas xkb/hwdb/remapper
taramalarında GÖRÜNMEZ — kanıt aracı: `sudo bash machine-config/kbd-capture.sh`
(30 sn libinput kaydı → /tmp/kbd-listen.log). Modlar: FN+A=Mac, FN+S=Windows, FN+WIN=Win-kilidi.

## Super-devri düzeni (2026-07-12)

| Kombinasyon (mantıksal) | Fiziksel basış | Sahibi | Ne yapar |
|---|---|---|---|
| `super+t` | Alt+T | herdr | yeni home-chat tab (`new_chat_tab`, master 9ccc83a) |
| `super+w` | Alt+W | herdr | aktif tab'ı kapat |
| `super+1..9` | Alt+1..9 | herdr | herdr tab geçişi |
| `super+tab` | Alt+Tab (fiziksel) | kitty | kitty tab döngüsü |
| `alt+1..4` | Win+1..4 | GNOME | workspace geçişi (dokunulmadı) |

## 🔴 İKİ KÖK NEDEN (debug serüveninin dersi — 2026-07-12)

İlk kurulumda super+t/w/1..9 çalışmadı. Katman-zinciri analiziyle İKİ bağımsız kök neden bulundu:

1. **Klavye firmware Mac-modu** (yukarıda) — kullanıcının bastığı tuş beklenenden farklı keysym üretir.
2. **kitty.conf'ta `kitty_mod super`** (satır ~251): kitty'nin ~40 BUILT-IN default kısayolu
   (`kitty_mod+t`=new_tab, `kitty_mod+w`=close_window, …) super'e bağlanır ve super'i YUTAR.
   Bunlar conf dosyasında YAZMAZ (gömülü default) — `grep map` ile bulunamaz!

**Çözüm:** kitty'de `map <combo> no_op` = "kitty yakalamaz, tuşu içindeki programa geçirir"
(man kitty.conf). kitty.conf sonundaki no_op bloğu super+t/w/1..9'u herdr'a geçirir.
Kitty'de SON tanım kazanır → orijinal map'ler yorumlanmadan override edildi.

## Kurulum (yeni makinede)

1. **herdr config:** `cp machine-config/herdr-config.toml ~/.config/herdr/config.toml`
   — doğrula: `herdr config check` → `config: ok`.
2. **kitty config (TAM):** `cp machine-config/kitty.conf ~/.config/kitty/kitty.conf`
   (içinde: kitty_mod=super + no_op bloğu + startup_session herdr + yorumlanmış super+N goto_tab'lar)
3. **kitty session:** `mkdir -p ~/.config/kitty/sessions && cp machine-config/kitty-herdr.session ~/.config/kitty/sessions/herdr.session`
4. **GNOME:** Super+5..9'u boşalt:
   ```bash
   for i in 5 6 7 8 9; do gsettings set org.gnome.shell.keybindings switch-to-application-$i "[]"; done
   ```
5. **Klavye:** M96BT2 Mac modunda olmalı (FN+A) — değilse fiziksel tuş haritası kayar.
6. herdr binary: CyPack/herdr master'dan `cargo build --release` (PATH'te zig 0.15.2).
7. **Doğrulama:** herdr içinde fiziksel Alt+T → home'da claude tab'ı açılmalı.
   Sorunda: `sudo bash machine-config/kbd-capture.sh` ile katman-1'den başla.

## Geri alma

- **herdr keys:** `~/.config/herdr/config.toml` `[keys]` bloğunu sil (veya `herdr config reset-keys`).
- **kitty:** no_op bloğunu sil + `#map super+N goto_tab N` yorumlarını aç + `startup_session none`.
- **GNOME:** eski değerler `machine-config/gnome-old-values.txt` → `gsettings set` ile geri.
- **Tam yedek:** `~/.config/keybind-backups-2026-07-12/pre-super-devri/` (dokunulmamış halleriyle).

## Teşhis metodolojisi (kısayol "çalışmıyor" → katman zinciri)

```
1 klavye-firmware (kbd-capture.sh — yazılımdan görünmez!)
2 OS remap (xkb-options · udev hwdb · input-remapper/keyd)
3 compositor (gsettings wm/shell + dconf media-keys custom + extension şemaları tek tek)
4 terminal emülatör (kitty_mod tanımı! + built-in default'lar + ESKİ süreçlerin RAM config'i)
5 multiplexer (herdr: config check + server binary/config tazeliği /proc/PID/exe)
6 uygulama
```
Çift-taraflı canlı test: kullanıcı basar + `herdr tab list` diff. Detay: memory
`keyboard-shortcut-layer-debugging.md`. NOT: herdr tmux KULLANMAZ (kendi Rust runtime'ı) —
tmux-dönemi kısayol notları (antiX repo) bu katmana uygulanmaz.

## Notlar

- herdr client kitty keyboard protocol push eder (DISAMBIGUATE) + `super` parse eder —
  super'in herdr'a ulaşması kitty'nin tuşu GEÇİRMESİNE bağlıdır (no_op bloğu).
- kitty yeni sekme gerekirse: no_op super+t'yi aldı; `kitten @ launch --type=tab` kullan
  ya da ayrı map ekle. Çıplak shell'li pencere: `kitty --session none`.
- Eski kitty süreçleri config değişikliğini GÖRMEZ — pencereleri kapat/yeniden aç
  (reload kısayolu kitty_mod+F5 = fiziksel Alt+F5).
