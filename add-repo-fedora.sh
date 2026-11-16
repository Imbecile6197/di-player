#!/usr/bin/env bash
# Add the DI Player dnf repository (Fedora).
set -euo pipefail

REPO_URL="${REPO_URL:-https://imbecile6197.github.io/di-player/di-player.repo}"
DEST="/etc/yum.repos.d/di-player.repo"

if [[ "$(id -u)" -ne 0 ]]; then
  exec sudo bash "$0" "$@"
fi

curl -fsSL "$REPO_URL" -o "$DEST"
chmod 644 "$DEST"
dnf clean expire-cache >/dev/null 2>&1 || true

echo "Added $DEST"
echo "Install with:  sudo dnf install di-player"
