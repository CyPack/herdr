# machine-config — Ayaz workstation kurulum kaydı (LOCAL branch)

> **Bu branch (`local/machine-config`) master'a ASLA merge edilmez, PR açılmaz.**
> Amaç: makine yeniden kurulumunda herdr çalışma düzenini dakikalar içinde geri getirmek.
> Master ilerledikçe bu branch master üstünde rebase/merge ile taşınır.

## Super-devri düzeni (2026-07-12)

Tab yönetimi tek modifier'da: **Super** (Windows tuşu). OS seviyesinde Alt↔Super
takası YOKTUR (xkb-options boş) — roller şöyle dağıtılmıştır:

| Kombinasyon | Sahibi | Ne yapar |
|---|---|---|
| `super+t` | herdr | yeni home-chat tab (default agent, cwd=$HOME) — `new_chat_tab` action (9ccc83a) |
| `super+w` | herdr | aktif tab'ı kapat (`close_tab`, prefix+shift+x da çalışır) |
| `super+1..9` | herdr | herdr tab geçişi (`switch_tab`, prefix+1..9 da çalışır) |
| `super+tab` / `super+shift+tab` | kitty | kitty tab döngüsü (dokunulmadı) |
| `kitty_mod+alt+1..9` | kitty | kitty tab geçişi (yedek yol) |
| `alt+1..4` | GNOME | workspace geçişi (dokunulmadı — kullanıcı alışkanlığı) |

## Kurulum (yeni makinede)

1. **herdr config:** `cp machine-config/herdr-config.toml ~/.config/herdr/config.toml`
   — doğrula: `herdr config check` → `config: ok` beklenir.
2. **kitty session:** `mkdir -p ~/.config/kitty/sessions && cp machine-config/kitty-herdr.session ~/.config/kitty/sessions/herdr.session`
3. **kitty.conf** içinde:
   - `startup_session ~/.config/kitty/sessions/herdr.session` (dosyadaki SON startup_session satırı olmalı — kitty'de son değer kazanır)
   - `map super+1..9 goto_tab N` satırları KALDIRILMIŞ/yorumlanmış olmalı (yoksa kitty yutar, herdr göremez)
4. **GNOME:** Super+5..9'u boşalt (Super+num'u GNOME yutmasın):
   ```bash
   for i in 5 6 7 8 9; do gsettings set org.gnome.shell.keybindings switch-to-application-$i "[]"; done
   ```
   (1..4 bu makinede zaten boştu.)
5. herdr binary'si CyPack/herdr master'dan build edilir: `cargo build --release`
   (PATH'te zig 0.15.2 gerekli — vendored libghostty-vt build-dep'i).

## Geri alma

- **herdr keys:** `~/.config/herdr/config.toml` içindeki `[keys]` bloğunu sil (veya `herdr config reset-keys`).
- **kitty:** `startup_session` satırını `startup_session none` yap; `#map super+N goto_tab N` satırlarının `#`'larını kaldır.
- **GNOME:** eski değerler `machine-config/gnome-old-values.txt` içinde; her satırı `gsettings set org.gnome.shell.keybindings <anahtar> "<değer>"` ile geri yükle.
- **Tam yedek:** `~/.config/keybind-backups-2026-07-12/pre-super-devri/` (kitty.conf + herdr config + gnome değerleri, dokunulmamış halleriyle).

## Notlar

- herdr client kitty keyboard protocol'ü push eder (client/mod.rs) ve `super`
  modifier'ını parse eder (raw_input.rs) — super kombinasyonlarının herdr'a
  ulaşması bu ikisine dayanır. Sorun görülürse önce `herdr config check`, sonra
  kitty'de `kitty +kitten show_key -m kitty` ile super raporlanıyor mu bak.
- Çıplak shell'li kitty penceresi: `kitty --session none`.
