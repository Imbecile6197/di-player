#!/usr/bin/env bash
# Zveřejní flatpak repozitář DI Playeru dist/flatpak/repo na GitHub Pages veřejného
# repozitáře (větev gh-pages):
#   https://imbecile6197.github.io/di-player/                 – stránka s instalací
#   https://imbecile6197.github.io/di-player/di-player.flatpakref
#   https://imbecile6197.github.io/di-player/di-player.flatpakrepo
#   https://imbecile6197.github.io/di-player/repo/            – OSTree repozitář
# Větev gh-pages má vždy jediný commit (force push), aby git historie nerostla
# o staré verze. Skript čeká, dokud Pages nezačnou servírovat nový commit.
set -euo pipefail
cd "$(dirname "$0")/../.."

APP_ID="com.alchymista.DiPlayer"
BRANCH="master"
PUBLIC_REPO="Imbecile6197/di-player"
BASE_URL="https://imbecile6197.github.io/di-player"
REPO="$PWD/dist/flatpak/repo"
SITE="$PWD/dist/pages"
REF_PATH="refs/heads/app/$APP_ID/x86_64/$BRANCH"
GPG_B64="$(base64 -w0 packaging/flatpak/di-player-repo.gpg)"

[[ -f "$REPO/$REF_PATH" ]] || { echo "Chybí $REPO – nejdřív packaging/flatpak/build.sh" >&2; exit 1; }

rm -rf "$SITE"
mkdir -p "$SITE"
rsync -a "$REPO/" "$SITE/repo/"
touch "$SITE/.nojekyll"

# Starý RPM/DEB repozitář zůstává, dokud z něj uživatelé nepřejdou na flatpak –
# jinak by jim dnf/apt při aktualizaci hlásily chybu nedostupného repozitáře.
git fetch -q "https://github.com/$PUBLIC_REPO.git" gh-pages
git archive FETCH_HEAD rpm deb di-player.repo add-repo-fedora.sh add-repo-ubuntu.sh 2>/dev/null \
    | tar -x -C "$SITE" || echo "Starý RPM/DEB repozitář na gh-pages nenalezen – pokračuji bez něj."

cat > "$SITE/di-player.flatpakrepo" <<EOF
[Flatpak Repo]
Title=DI Player
Url=$BASE_URL/repo/
Homepage=https://github.com/$PUBLIC_REPO
Comment=Native GNOME player for DI.fm premium radio
Icon=$BASE_URL/$APP_ID.svg
DefaultBranch=$BRANCH
GPGKey=$GPG_B64
EOF

cat > "$SITE/di-player.flatpakref" <<EOF
[Flatpak Ref]
Name=$APP_ID
Branch=$BRANCH
Title=DI Player
Url=$BASE_URL/repo/
SuggestRemoteName=di-player
IsRuntime=false
Homepage=https://github.com/$PUBLIC_REPO
Comment=Native GNOME player for DI.fm premium radio
Icon=$BASE_URL/$APP_ID.svg
RuntimeRepo=https://dl.flathub.org/repo/flathub.flatpakrepo
GPGKey=$GPG_B64
EOF

install -m644 "resources/icons/hicolor/scalable/apps/$APP_ID.svg" "$SITE/$APP_ID.svg"
cat > "$SITE/index.html" <<EOF
<!doctype html>
<html lang="cs">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>DI Player</title>
<style>
  :root { color-scheme: light dark; --accent: #2f8fd8; }
  body { font: 16px/1.5 system-ui, sans-serif; max-width: 42rem; margin: 3rem auto; padding: 0 1rem; }
  header { display: flex; align-items: center; gap: 1rem; }
  header img { width: 64px; height: 64px; }
  h1 { margin: 0; }
  a.button { display: inline-block; background: var(--accent); color: #fff; padding: .6rem 1.2rem;
             border-radius: 999px; text-decoration: none; font-weight: 600; }
  pre { background: color-mix(in srgb, currentColor 8%, transparent); padding: .8rem; border-radius: 8px;
        overflow-x: auto; }
  .muted { opacity: .7; font-size: .9rem; }
</style>
</head>
<body>
<header>
  <img src="$APP_ID.svg" alt="">
  <div><h1>DI Player</h1><div>Nativní GNOME přehrávač prémiového rádia DI.fm</div></div>
</header>
<h2>Instalace</h2>
<p><a class="button" href="di-player.flatpakref">Nainstalovat DI Player</a></p>
<p>Otevře se v aplikaci Software. Nebo v terminálu:</p>
<pre>flatpak install --user $BASE_URL/di-player.flatpakref</pre>
<p>Aktualizace pak instaluje aplikace Software nebo příkaz <code>flatpak update</code>.</p>
<p class="muted">Je-li to vaše první aplikace z Flatpaku a DI Player se v přehledu aplikací neobjeví, odhlaste se a znovu přihlaste.</p>
<h2 lang="en">Installation</h2>
<p lang="en">Click the button above to install with Software, or run the command in a terminal.
Updates are installed by Software or by <code>flatpak update</code>.
If it is your first Flatpak app and DI Player does not appear in the app overview, log out and back in once.</p>
<p class="muted">Starší balíčky RPM/DEB (repozitáře <code>rpm/</code> a <code>deb/</code>) se už neaktualizují – přejděte prosím na flatpak.</p>
<p class="muted">Vydání a licence: <a href="https://github.com/$PUBLIC_REPO">github.com/$PUBLIC_REPO</a></p>
</body>
</html>
EOF

cd "$SITE"
git init -q -b gh-pages
git add -A
git -c user.name="$(git -C .. config user.name)" -c user.email="$(git -C .. config user.email)" \
    commit -q -m "Flatpak repozitář"
git push -q -f "https://github.com/$PUBLIC_REPO.git" gh-pages:gh-pages

# Zapnout Pages (jen poprvé).
if ! gh api "repos/$PUBLIC_REPO/pages" >/dev/null 2>&1; then
    gh api -X POST "repos/$PUBLIC_REPO/pages" -f "source[branch]=gh-pages" -f "source[path]=/" >/dev/null
fi

expected="$(cat "$REPO/$REF_PATH")"
echo "Čekám, až GitHub Pages zveřejní commit ${expected:0:12}…"
for _ in $(seq 1 90); do
    served="$(curl -fsS "$BASE_URL/repo/$REF_PATH?t=$(date +%s%N)" 2>/dev/null || true)"
    if [[ "$served" == "$expected" ]]; then
        echo "Repozitář zveřejněn: $BASE_URL/"
        exit 0
    fi
    sleep 10
done
echo "GitHub Pages nový commit do 15 minut nezveřejnily." >&2
exit 1
