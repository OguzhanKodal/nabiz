#!/usr/bin/env bash
# Derlenmiş ikiliyi ve sistem dosyalarını kurar (root ile). install.sh ve guncelle-ekran.sh kullanır.
set -euo pipefail
cd "$(dirname "$0")/.."
ID=io.github.oguzhankodal.nabiz

install -Dm755 src-tauri/target/release/nabiz /usr/bin/nabiz
install -Dm644 packaging/$ID.policy /usr/share/polkit-1/actions/$ID.policy
install -Dm644 packaging/70-nabiz-keyboard.rules /usr/lib/udev/rules.d/70-nabiz-keyboard.rules
install -Dm644 packaging/$ID.desktop /usr/share/applications/$ID.desktop
rm -f /usr/share/applications/nabiz.desktop /usr/share/icons/hicolor/256x256/apps/nabiz.png

install -Dm644 packaging/icon.svg /usr/share/icons/hicolor/scalable/apps/$ID.svg
install -Dm644 src-tauri/icons/32x32.png /usr/share/icons/hicolor/32x32/apps/$ID.png
install -Dm644 src-tauri/icons/64x64.png /usr/share/icons/hicolor/64x64/apps/$ID.png
install -Dm644 src-tauri/icons/128x128.png /usr/share/icons/hicolor/128x128/apps/$ID.png
install -Dm644 src-tauri/icons/128x128@2x.png /usr/share/icons/hicolor/256x256/apps/$ID.png

gtk-update-icon-cache -q -t -f /usr/share/icons/hicolor 2>/dev/null || true
update-desktop-database -q /usr/share/applications 2>/dev/null || true
udevadm control --reload-rules && udevadm trigger --subsystem-match=hidraw
