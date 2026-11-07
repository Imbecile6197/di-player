#!/usr/bin/env bash
# Build a Fedora/RHEL .rpm from the current release binary.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="${VERSION:-0.1.0}"
OUT_DIR="${OUT_DIR:-$ROOT/dist}"
BIN="$ROOT/target/release/di_player"

[[ -x "$BIN" ]] || { echo "Build release first: cargo build --release"; exit 1; }

WORKDIR="$(mktemp -d)"
trap 'rm -rf "$WORKDIR"' EXIT

TOP="$WORKDIR/rpmbuild"
mkdir -p "$TOP"/{BUILD,BUILDROOT,RPMS,SOURCES,SPECS,SRPMS}

cp "$BIN" "$TOP/SOURCES/di_player"
cp "$ROOT/data/com.alchymista.DiPlayer.desktop" "$TOP/SOURCES/"
# Force themed icon name inside the package (system hicolor install).
sed 's|^Icon=.*|Icon=com.alchymista.DiPlayer|; s|^Exec=.*|Exec=di_player|' \
  "$ROOT/data/com.alchymista.DiPlayer.desktop" > "$TOP/SOURCES/com.alchymista.DiPlayer.desktop"
cp "$ROOT/resources/icons/hicolor/scalable/apps/com.alchymista.DiPlayer.svg" \
  "$TOP/SOURCES/com.alchymista.DiPlayer.svg"

for s in 16 32 48 64 128 256; do
  src="$ROOT/resources/icons/hicolor/${s}x${s}/apps/com.alchymista.DiPlayer.png"
  [[ -f "$src" ]] || { echo "missing icon size $s"; exit 1; }
  cp "$src" "$TOP/SOURCES/com.alchymista.DiPlayer-${s}.png"
done

cp "$ROOT/packaging/di-player.spec" "$TOP/SPECS/di-player.spec"
sed -i "s/^Version:.*/Version:        ${VERSION}/" "$TOP/SPECS/di-player.spec"

rpmbuild \
  --define "_topdir $TOP" \
  --define "_sourcedir $TOP/SOURCES" \
  -bb "$TOP/SPECS/di-player.spec"

mkdir -p "$OUT_DIR"
find "$TOP/RPMS" -name '*.rpm' -exec cp {} "$OUT_DIR/" \;
echo "RPM packages:"
ls -lh "$OUT_DIR"/di-player*.rpm
