#!/usr/bin/env bash
# Sestaví DI Player do podepsaného flatpak repozitáře dist/flatpak/repo (ten se
# zveřejňuje na GitHub Pages – packaging/flatpak/pages.sh) a k tomu jednosouborový
# balíček dist/di-player-<verze>.flatpak, který po instalaci odebírá aktualizace
# z téhož repozitáře.
#
# Potřebuje: flatpak install --user flathub \
#   org.flatpak.Builder org.gnome.Sdk//51 org.freedesktop.Sdk.Extension.rust-stable//26.08
# a soukromý klíč GPG repozitáře v ~/.gnupg (viz packaging/flatpak/README.md).
set -euo pipefail
cd "$(dirname "$0")/../.."

APP_ID="com.alchymista.DiPlayer"
BRANCH="master"
REPO_URL="${DIPLAYER_REPO_URL:-https://imbecile6197.github.io/di-player/repo/}"
GPG_KEY="E72317594A080699E3C2805B3A7D83C99CC8C04E"
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
# Pro zkoušky lze pracovní složku a adresu repozitáře přesměrovat.
WORK="${DIPLAYER_FLATPAK_WORK:-$PWD/dist/flatpak}"
BUNDLE="${DIPLAYER_FLATPAK_BUNDLE:-dist/di-player-$VERSION.flatpak}"
REPO="$WORK/repo"
mkdir -p "$WORK"

if ! gpg --list-secret-keys "$GPG_KEY" >/dev/null 2>&1; then
    echo "Chybí soukromý klíč repozitáře $GPG_KEY (viz packaging/flatpak/README.md)." >&2
    exit 1
fi

flatpak run --filesystem="$PWD" org.flatpak.Builder \
    --force-clean --user --disable-rofiles-fuse \
    --default-branch="$BRANCH" \
    --state-dir="$WORK/state" --repo="$REPO" \
    "$WORK/build" packaging/flatpak/$APP_ID.yml

# Podpis nových commitů (aplikace i ladicích symbolů) a souhrnu repozitáře.
flatpak build-sign --gpg-sign="$GPG_KEY" "$REPO" "$APP_ID" "$BRANCH"
if [[ -e "$REPO/refs/heads/runtime/$APP_ID.Debug/x86_64/$BRANCH" ]]; then
    flatpak build-sign --runtime --gpg-sign="$GPG_KEY" "$REPO" "$APP_ID.Debug" "$BRANCH"
fi
# flatpak-builder zapíše větve appstream bez podpisu a build-update-repo je při
# nezměněném obsahu znovu nevytvoří – klientům pak `flatpak update` hlásí
# „GPG verification enabled, but no signatures found“. Proto je smažeme, aby je
# build-update-repo níže vytvořil znovu a podepsal.
flatpak run --filesystem="$WORK" --command=ostree org.flatpak.Builder \
    --repo="$REPO" refs --delete appstream/x86_64 appstream2/x86_64 2>/dev/null || true
flatpak build-update-repo \
    --gpg-sign="$GPG_KEY" \
    --title="DI Player" \
    --default-branch="$BRANCH" \
    --generate-static-deltas \
    --prune --prune-depth=3 \
    "$REPO"

flatpak build-bundle \
    --repo-url="$REPO_URL" \
    --gpg-keys=packaging/flatpak/di-player-repo.gpg \
    --runtime-repo=https://dl.flathub.org/repo/flathub.flatpakrepo \
    "$REPO" "$BUNDLE" "$APP_ID" "$BRANCH"
ls -lh "$BUNDLE"
