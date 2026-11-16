#!/usr/bin/env bash
# Add the DI Player apt repository (Ubuntu/Debian).
set -euo pipefail

BASE="${BASE_URL:-https://imbecile6197.github.io/di-player}"
LIST="/etc/apt/sources.list.d/di-player.list"
KEYRING_DIR="/etc/apt/keyrings"

if [[ "$(id -u)" -ne 0 ]]; then
  exec sudo bash "$0" "$@"
fi

# Unsigned GitHub Pages repo — mark as trusted for this source only.
cat > "$LIST" <<EOF
deb [trusted=yes arch=amd64] ${BASE}/deb stable main
EOF
chmod 644 "$LIST"

apt-get update -o Dir::Etc::sourcelist="$LIST" -o Dir::Etc::sourceparts="-" -o APT::Get::List-Cleanup="0" >/dev/null

echo "Added $LIST"
echo "Install with:  sudo apt install di-player"
