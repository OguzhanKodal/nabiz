#!/usr/bin/env bash
# Nabız'ı Arch / CachyOS üzerinde kaynaktan derleyip kurar.
# Kullanım: proje klasöründe `bash packaging/install.sh`
set -euo pipefail
cd "$(dirname "$0")/.."

echo "==> Gerekli paketler"
sudo pacman -S --needed --noconfirm base-devel rust nodejs npm webkit2gtk-4.1 polkit

echo "==> Arayüz bağımlılıkları"
npm install

echo "==> Derleme (birkaç dakika sürebilir)"
npm run tauri build -- --no-bundle

echo "==> Kurulum"
sudo bash packaging/install-files.sh

echo "==> Bitti. Uygulamalar menüsünden 'Nabız'ı aç ya da terminalde: nabiz"
