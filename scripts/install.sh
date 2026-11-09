#!/usr/bin/env bash
# Install DI Player to ~/.local for the current user (Fedora + Ubuntu/Debian).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PREFIX="${PREFIX:-$HOME/.local}"
BIN_DIR="$PREFIX/bin"
APP_DIR="$PREFIX/share/applications"
ICON_BASE="$PREFIX/share/icons/hicolor"

die() { echo "error: $*" >&2; exit 1; }
need() { command -v "$1" >/dev/null 2>&1 || die "missing dependency: $1"; }

detect_os() {
  if [[ -f /etc/os-release ]]; then
    # shellcheck disable=SC1091
    . /etc/os-release
    echo "${ID:-unknown}"
  else
    echo unknown
  fi
}

install_build_deps() {
  local os
  os="$(detect_os)"
  case "$os" in
    fedora|rhel|centos|nobara|ultramarine)
      echo "==> Installing Fedora build dependencies (sudo)…"
      sudo dnf install -y \
        gcc pkgconf-pkg-config openssl-devel \
        gtk4-devel libadwaita-devel \
        mpv cargo rust
      ;;
    ubuntu|debian|linuxmint|pop|elementary|zorin)
      echo "==> Installing Ubuntu/Debian build dependencies (sudo)…"
      sudo apt-get update
      sudo apt-get install -y \
        build-essential pkg-config libssl-dev \
        libgtk-4-dev libadwaita-1-dev \
        mpv cargo rustc
      ;;
    arch|manjaro|endeavouros)
      echo "==> Installing Arch build dependencies (sudo)…"
      sudo pacman -Syu --needed --noconfirm \
        base-devel openssl gtk4 libadwaita mpv rust
      ;;
    *)
      echo "Unrecognized distro ($os). Skipping package install."
      echo "Need: rustc/cargo, gtk4, libadwaita, openssl, mpv, pkg-config"
      ;;
  esac
}

build() {
  need cargo
  need pkg-config
  echo "==> Building release binary…"
  (cd "$ROOT" && cargo build --release)
}

install_files() {
  local bin="$ROOT/target/release/di_player"
  [[ -x "$bin" ]] || die "release binary missing at $bin"

  echo "==> Installing to $PREFIX…"
  install -Dm755 "$bin" "$BIN_DIR/di_player"

  # Icons
  local sizes=(16 22 24 32 48 64 96 128 256 512)
  local s
  for s in "${sizes[@]}"; do
    local src="$ROOT/resources/icons/hicolor/${s}x${s}/apps/com.alchymista.DiPlayer.png"
    if [[ -f "$src" ]]; then
      install -Dm644 "$src" \
        "$ICON_BASE/${s}x${s}/apps/com.alchymista.DiPlayer.png"
    fi
  done
  install -Dm644 \
    "$ROOT/resources/icons/hicolor/scalable/apps/com.alchymista.DiPlayer.svg" \
    "$ICON_BASE/scalable/apps/com.alchymista.DiPlayer.svg"

  # Do not write index.theme here — a partial user hicolor index shadows the
  # system theme and breaks other GTK apps' bundled icons.

  local icon_path="$ICON_BASE/512x512/apps/com.alchymista.DiPlayer.png"
  cat > "$APP_DIR/com.alchymista.DiPlayer.desktop" <<EOF
[Desktop Entry]
Name=DI Player
GenericName=Internet Radio
Comment=Native GNOME player for DI.fm premium radio
Exec=$BIN_DIR/di_player
Icon=$icon_path
Terminal=false
Type=Application
Categories=AudioVideo;Audio;Player;Tuner;
Keywords=radio;di.fm;digitally;imported;electronic;
StartupNotify=true
StartupWMClass=com.alchymista.DiPlayer
EOF

  if command -v gtk4-update-icon-cache >/dev/null 2>&1; then
    gtk4-update-icon-cache -f "$ICON_BASE" >/dev/null 2>&1 || true
  elif command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -f "$ICON_BASE" >/dev/null 2>&1 || true
  fi
  if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$APP_DIR" >/dev/null 2>&1 || true
  fi

  echo
  echo "Installed DI Player."
  echo "  binary : $BIN_DIR/di_player"
  echo "  desktop: $APP_DIR/com.alchymista.DiPlayer.desktop"
  echo
  echo "Ensure $BIN_DIR is on your PATH, then run: di_player"
  echo "GNOME tray: enable the AppIndicator extension."
}

main() {
  local with_deps=0
  for arg in "$@"; do
    case "$arg" in
      --deps) with_deps=1 ;;
      -h|--help)
        cat <<EOF
Usage: scripts/install.sh [--deps]

  --deps   Install distro build packages with sudo (Fedora/Ubuntu/Arch)
EOF
        exit 0
        ;;
    esac
  done

  if [[ "$with_deps" -eq 1 ]]; then
    install_build_deps
  fi
  build
  install_files
}

main "$@"
