# Flatpak repozitář DI Player

DI Player se vydává jako flatpak z vlastního podepsaného repozitáře na GitHub
Pages repozitáře `Imbecile6197/di-player` (větev `gh-pages`):

- stránka s instalací: https://imbecile6197.github.io/di-player/
- instalace: `flatpak install --user https://imbecile6197.github.io/di-player/di-player.flatpakref`

Aktualizace instaluje aplikace Software nebo `flatpak update`.

## Vydání

    ./packaging/publish.sh X.Y.Z "poznámky"

Nejdřív doplňte `<release version="X.Y.Z">` do `data/com.alchymista.DiPlayer.metainfo.xml`.
Skript zvedne verzi, sestaví a podepíše repozitář, otaguje a odešle `main`,
zveřejní repozitář na Pages (čeká na nasazení) a vytvoří vydání na GitHubu
s `.flatpak` a `SHA256SUMS`.

## Sestavení a zveřejnění ručně

    flatpak install --user flathub org.flatpak.Builder org.gnome.Sdk//51 \
        org.freedesktop.Sdk.Extension.rust-stable//26.08
    ./packaging/flatpak/build.sh   # dist/flatpak/repo + dist/di-player-<verze>.flatpak
    ./packaging/flatpak/pages.sh   # force-push gh-pages, čeká na nasazení Pages

Manifest `com.alchymista.DiPlayer.yml` přibaluje mpv (s libass a libplacebo),
které DI Player spouští jako samostatný proces. ffmpeg je v runtime GNOME,
kodeky AAC doplní rozšíření `org.freedesktop.Platform.codecs-extra`.

Repozitář se udržuje v `dist/flatpak/repo` (uchovávají se 3 poslední verze).
Kdyby se ztratil, stačí ho sestavit znovu – klienti stáhnou nový commit.

`pages.sh` na Pages ponechává i starý RPM/DEB repozitář (`rpm/`, `deb/`),
aby uživatelům dnf/apt nehlásily chybu nedostupného repozitáře.

## Převzetí nastavení

Ve flatpaku je konfigurace v `~/.var/app/com.alchymista.DiPlayer/config/di-player`.
Při prvním spuštění se do ní zkopírují `settings.json` a `credentials.json`
z dřívější instalace (`~/.config/di-player`, sandbox ho vidí jen pro čtení).

## Podpisový klíč

Repozitář je podepsaný klíčem GPG `E72317594A080699E3C2805B3A7D83C99CC8C04E`
(„DI Player Flatpak Repository“). Veřejná část je v `di-player-repo.gpg` a ve
`.flatpakref`/`.flatpakrepo`, soukromá jen v `~/.gnupg` autora.

**Soukromý klíč zálohujte** – bez něj nejde podepsat další verzi a uživatelé by
museli repozitář přidat znovu s novým klíčem:

    gpg --export-secret-keys --armor E72317594A080699E3C2805B3A7D83C99CC8C04E > di-player-repo-private.asc

(soubor uložte mimo počítač, např. do správce hesel; do gitu nepatří).
