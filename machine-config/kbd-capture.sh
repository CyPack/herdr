#!/usr/bin/env bash
# Klavye teşhis kaydedici (Claude Code oturumu için, 2026-07-12).
# Kernel input katmanından (libinput) 30 saniye boyunca tuş event'lerini yakalar.
# Firmware'in GERÇEKTE hangi keycode'u gönderdiğini gösterir (xkb/GNOME yorumundan önce).
# Kullanım: sudo bash /tmp/kbd-capture.sh
set -u
LOG=/tmp/kbd-listen.log
DUR=30

echo "=== Cihaz haritası (hangi event hangi klavye) ===" | tee "$LOG"
awk '/^N: Name=/{name=$0} /^H: Handlers=.*kbd/{print "  " name " -> " $0}' /proc/bus/input/devices | tee -a "$LOG"
echo | tee -a "$LOG"
echo ">>> KAYIT BAŞLADI — ${DUR} saniyen var. ŞİMDİ sırayla bas:"
echo ">>>   Alt+T   Alt+W   Alt+1   Alt+2   Alt+3   ve   Win+T"
echo ">>> (Alt'ı basılı tutup harfe/rakama bas; her kombinasyon arasında 1 sn bekle)"
echo

timeout "$DUR" stdbuf -oL libinput debug-events --show-keycodes 2>/dev/null \
  | grep --line-buffered KEYBOARD_KEY \
  | tee -a "$LOG"

chmod 644 "$LOG" 2>/dev/null
echo
echo ">>> BİTTİ. Kayıt: $LOG ($(grep -c KEYBOARD_KEY "$LOG" 2>/dev/null || echo 0) tuş event'i)."
echo ">>> Şimdi Claude'a 'tamam, oku' yaz."
