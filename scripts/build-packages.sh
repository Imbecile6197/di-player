#!/usr/bin/env bash
# Build Fedora .rpm locally and Ubuntu .deb inside podman (ubuntu:24.04).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="${VERSION:-0.1.0}"
OUT_DIR="$ROOT/dist"
export VERSION OUT_DIR

mkdir -p "$OUT_DIR"
rm -f "$OUT_DIR"/di-player*.rpm "$OUT_DIR"/di-player*.deb "$OUT_DIR"/*.tar.gz

echo "==> Building release binary on host (Fedora RPM)…"
(cd "$ROOT" && CARGO_TARGET_DIR="$ROOT/target" cargo build --release --locked)
"$ROOT/scripts/package-rpm.sh"

if ! command -v podman >/dev/null; then
  echo "podman not found; skipping Ubuntu .deb"
  ls -lh "$OUT_DIR"
  exit 0
fi

echo "==> Building Ubuntu 25.04 binary + .deb in podman…"
# Separate target dir so the Fedora binary is not overwritten.
# Ubuntu 24.04 ships libadwaita 1.5; we need >= 1.6 (25.04+).
podman run --rm \
  -v "$ROOT:/src:Z" \
  -w /src \
  -e VERSION="$VERSION" \
  docker.io/library/ubuntu:25.04 \
  bash -lc '
    set -euo pipefail
    export DEBIAN_FRONTEND=noninteractive
    apt-get update
    apt-get install -y \
      build-essential pkg-config curl ca-certificates \
      libssl-dev libgtk-4-dev libadwaita-1-dev \
      dpkg-dev
    curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
    # shellcheck disable=SC1091
    source "$HOME/.cargo/env"
    export CARGO_TARGET_DIR=/src/target-ubuntu
    cargo build --release --locked
    BIN=/src/target-ubuntu/release/di_player OUT_DIR=/src/dist ./scripts/package-deb.sh
  '

echo
echo "Artifacts in $OUT_DIR:"
ls -lh "$OUT_DIR"
(
  cd "$OUT_DIR"
  tar -czf "di-player-${VERSION}-linux-packages.tar.gz" di-player-*.rpm di-player_*.deb
  ls -lh "di-player-${VERSION}-linux-packages.tar.gz"
)
