#!/usr/bin/env bash
# Build a static dnf+apt repository into ./site for GitHub Pages.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DIST="${DIST:-$ROOT/dist}"
SITE="${SITE:-$ROOT/site}"
VERSION="${VERSION:-0.2.0}"

RPM="$(ls "$DIST"/di-player-*.rpm 2>/dev/null | head -1 || true)"
DEB="$(ls "$DIST"/di-player_*.deb 2>/dev/null | head -1 || true)"
[[ -n "$RPM" && -f "$RPM" ]] || { echo "missing RPM in $DIST"; exit 1; }
[[ -n "$DEB" && -f "$DEB" ]] || { echo "missing DEB in $DIST"; exit 1; }

rm -rf "$SITE"
mkdir -p "$SITE/rpm" "$SITE/deb/dists/stable/main/binary-amd64" "$SITE/deb/pool/main"

cp "$RPM" "$SITE/rpm/"
cp "$DEB" "$SITE/deb/pool/main/"
cp "$ROOT/packaging/di-player.repo" "$SITE/di-player.repo"
cp "$ROOT/scripts/add-repo-fedora.sh" "$SITE/add-repo-fedora.sh"
cp "$ROOT/scripts/add-repo-ubuntu.sh" "$SITE/add-repo-ubuntu.sh"
chmod 755 "$SITE"/add-repo-*.sh

# index page with copy-paste commands
cat > "$SITE/index.html" <<'EOF'
<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8"/>
  <title>DI Player packages</title>
  <style>
    body{font-family:system-ui,sans-serif;max-width:48rem;margin:2rem auto;padding:0 1rem;line-height:1.45}
    code,pre{background:#111;color:#d7ffef;padding:.15rem .35rem;border-radius:4px}
    pre{padding:1rem;overflow:auto}
    h1{margin-bottom:.2rem}
  </style>
</head>
<body>
  <h1>DI Player package repository</h1>
  <p>Native GNOME player for DI.fm — Fedora (dnf) and Ubuntu (apt) packages.</p>

  <h2>Fedora — add source</h2>
  <pre>curl -fsSL https://imbecile6197.github.io/di-player/add-repo-fedora.sh | sudo bash
sudo dnf install di-player</pre>

  <h2>Ubuntu — add source</h2>
  <pre>curl -fsSL https://imbecile6197.github.io/di-player/add-repo-ubuntu.sh | sudo bash
sudo apt install di-player</pre>

  <p>Source: <a href="https://github.com/Imbecile6197/di-player">github.com/Imbecile6197/di-player</a></p>
</body>
</html>
EOF

echo "==> Generating RPM repodata…"
podman run --rm -v "$SITE/rpm:/repo:Z" docker.io/library/fedora:41 \
  bash -lc 'dnf install -y createrepo_c >/dev/null && createrepo_c /repo'

echo "==> Generating APT metadata…"
podman run --rm -v "$SITE/deb:/repo:Z" docker.io/library/debian:trixie-slim \
  bash -lc '
    set -euo pipefail
    apt-get update -qq
    apt-get install -y -qq dpkg-dev >/dev/null
    cd /repo
    dpkg-scanpackages pool /dev/null > dists/stable/main/binary-amd64/Packages
    gzip -9c dists/stable/main/binary-amd64/Packages > dists/stable/main/binary-amd64/Packages.gz
    cat > dists/stable/Release <<EOF
Origin: DI Player
Label: DI Player
Suite: stable
Codename: stable
Architectures: amd64
Components: main
Description: DI Player apt repository
EOF
    # Append checksums for apt
    {
      echo "Date: $(date -Ru)"
      echo "MD5Sum:"
      for f in dists/stable/main/binary-amd64/Packages dists/stable/main/binary-amd64/Packages.gz; do
        size=$(wc -c < "$f")
        md5=$(md5sum "$f" | awk "{print \$1}")
        printf " %s %8d %s\n" "$md5" "$size" "${f#dists/stable/}"
      done
      echo "SHA256:"
      for f in dists/stable/main/binary-amd64/Packages dists/stable/main/binary-amd64/Packages.gz; do
        size=$(wc -c < "$f")
        sha=$(sha256sum "$f" | awk "{print \$1}")
        printf " %s %8d %s\n" "$sha" "$size" "${f#dists/stable/}"
      done
    } >> dists/stable/Release
  '

# nojekyll so GitHub Pages serves Packages/repodata as static files
touch "$SITE/.nojekyll"

echo "Site ready at $SITE"
find "$SITE" -type f | sort | head -50
