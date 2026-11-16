#!/usr/bin/env bash
# Vydání nové verze DI Playeru jedním příkazem:
#   ./packaging/publish.sh 0.3.0 "Co je nového…"
#
# 1. zkontroluje čistý strom a záznam o vydání v metainfo,
# 2. zvedne verzi v Cargo.toml a commitne ji,
# 3. sestaví podepsaný flatpak repozitář a balíček (packaging/flatpak/build.sh),
# 4. otaguje vX.Y.Z a odešle main i tag na GitHub,
# 5. zveřejní flatpak repozitář na GitHub Pages a počká, až je dostupný
#    (packaging/flatpak/pages.sh),
# 6. vytvoří vydání na GitHubu s .flatpak a kontrolními součty.
set -euo pipefail
cd "$(dirname "$0")/.."

REPO="Imbecile6197/di-player"
METAINFO="data/com.alchymista.DiPlayer.metainfo.xml"
VERSION="${1:-}"
NOTES="${2:-}"

if [[ ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "Použití: $0 <verze, např. 0.3.0> [\"poznámky k vydání\"]" >&2
    exit 1
fi
if [[ -n "$(git status --porcelain)" ]]; then
    echo "Pracovní strom není čistý – nejdřív commitněte změny." >&2
    exit 1
fi
if git rev-parse -q --verify "refs/tags/v$VERSION" >/dev/null; then
    echo "Verze v$VERSION už existuje." >&2
    exit 1
fi
if ! grep -q "<release version=\"$VERSION\"" "$METAINFO"; then
    echo "V $METAINFO chybí <release version=\"$VERSION\" …> – doplňte poznámky k vydání." >&2
    exit 1
fi

CURRENT="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
if [[ "$CURRENT" != "$VERSION" ]]; then
    sed -i "0,/^version = \"$CURRENT\"/s//version = \"$VERSION\"/" Cargo.toml
    cargo update -q --workspace
fi

./packaging/flatpak/build.sh

if [[ -n "$(git status --porcelain)" ]]; then
    git add Cargo.toml Cargo.lock
    git commit -q -m "Release $VERSION"
fi
git tag -a "v$VERSION" -m "DI Player $VERSION"
git push -q origin HEAD "v$VERSION"

./packaging/flatpak/pages.sh

(cd dist && sha256sum "di-player-$VERSION.flatpak" > SHA256SUMS)
[[ -n "$NOTES" ]] || NOTES="DI Player $VERSION"
gh release create "v$VERSION" "dist/di-player-$VERSION.flatpak" dist/SHA256SUMS \
    -R "$REPO" \
    --title "DI Player $VERSION" \
    --notes "$NOTES

**Install:** https://imbecile6197.github.io/di-player/ – or in a terminal \`flatpak install --user https://imbecile6197.github.io/di-player/di-player.flatpakref\`
**Updates** come through GNOME Software or \`flatpak update\`. The file di-player-$VERSION.flatpak can also be installed directly and then receives updates from the same repository."
echo "Vydáno: https://github.com/$REPO/releases/tag/v$VERSION"
