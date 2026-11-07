#!/usr/bin/env bash
# Build an Ubuntu/Debian .deb. Prefer an Ubuntu rootfs/container binary.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="${VERSION:-0.1.0}"
ARCH="${ARCH:-amd64}"
OUT_DIR="${OUT_DIR:-$ROOT/dist}"
BIN="${BIN:-$ROOT/target/release/di_player}"

[[ -x "$BIN" ]] || { echo "Binary not found: $BIN"; exit 1; }
command -v dpkg-deb >/dev/null || { echo "dpkg-deb required"; exit 1; }

STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT
PKG="$STAGE/di-player_${VERSION}_${ARCH}"

mkdir -p "$PKG/DEBIAN" \
  "$PKG/usr/bin" \
  "$PKG/usr/share/applications" \
  "$PKG/usr/share/icons/hicolor/scalable/apps" \
  "$PKG/usr/share/doc/di-player"

install -m 755 "$BIN" "$PKG/usr/bin/di_player"

sed 's|^Icon=.*|Icon=com.alchymista.DiPlayer|; s|^Exec=.*|Exec=di_player|' \
  "$ROOT/data/com.alchymista.DiPlayer.desktop" \
  > "$PKG/usr/share/applications/com.alchymista.DiPlayer.desktop"

install -m 644 \
  "$ROOT/resources/icons/hicolor/scalable/apps/com.alchymista.DiPlayer.svg" \
  "$PKG/usr/share/icons/hicolor/scalable/apps/com.alchymista.DiPlayer.svg"

for s in 16 32 48 64 128 256; do
  mkdir -p "$PKG/usr/share/icons/hicolor/${s}x${s}/apps"
  install -m 644 \
    "$ROOT/resources/icons/hicolor/${s}x${s}/apps/com.alchymista.DiPlayer.png" \
    "$PKG/usr/share/icons/hicolor/${s}x${s}/apps/com.alchymista.DiPlayer.png"
done

install -m 644 "$ROOT/LICENSE" "$PKG/usr/share/doc/di-player/copyright"
install -m 644 "$ROOT/README.md" "$PKG/usr/share/doc/di-player/README.md"

cat > "$PKG/DEBIAN/control" <<EOF
Package: di-player
Version: ${VERSION}
Section: sound
Priority: optional
Architecture: ${ARCH}
Depends: libgtk-4-1, libadwaita-1-0, mpv
Maintainer: Alchymista <alchymista@users.noreply.github.com>
Description: Native GNOME player for DI.fm premium radio
 DI Player is a GTK4/libadwaita client for Digitally Imported (DI.fm)
 premium internet radio, with channel browser, login and system tray.
Homepage: https://github.com/Imbecile6197/di-player
EOF

cat > "$PKG/DEBIAN/postinst" <<'EOF'
#!/bin/sh
set -e
if command -v gtk-update-icon-cache >/dev/null; then
  gtk-update-icon-cache -f /usr/share/icons/hicolor >/dev/null 2>&1 || true
fi
if command -v gtk4-update-icon-cache >/dev/null; then
  gtk4-update-icon-cache -f /usr/share/icons/hicolor >/dev/null 2>&1 || true
fi
if command -v update-desktop-database >/dev/null; then
  update-desktop-database /usr/share/applications >/dev/null 2>&1 || true
fi
EOF
chmod 755 "$PKG/DEBIAN/postinst"

mkdir -p "$OUT_DIR"
dpkg-deb --root-owner-group --build "$PKG" "$OUT_DIR/di-player_${VERSION}_${ARCH}.deb"
echo "DEB package:"
ls -lh "$OUT_DIR/di-player_${VERSION}_${ARCH}.deb"
