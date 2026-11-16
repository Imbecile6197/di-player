#!/usr/bin/env bash
# Najde, proč DI Player (Flatpak) není vidět v přehledu aplikací, a opraví, co jde.
#   curl -fsSL https://raw.githubusercontent.com/Imbecile6197/di-player/main/scripts/fix-launcher.sh | bash
set -u
ID=com.alchymista.DiPlayer
DESKTOP=$ID.desktop

echo "== Flatpak"
flatpak info "$ID" 2>/dev/null | grep -E 'Verze|Version|Instalace|Installation|Původ|Origin' \
    || echo "DI Player není nainstalovaný jako Flatpak."

echo "== Spouštěče $DESKTOP"
fixed=0
for d in "$HOME/.local/share/applications" /usr/local/share/applications /usr/share/applications \
         "$HOME/.local/share/flatpak/exports/share/applications" /var/lib/flatpak/exports/share/applications; do
    f="$d/$DESKTOP"
    [ -e "$f" ] || continue
    echo "-- $f"
    grep -E '^(Exec|Icon|NoDisplay|Hidden|TryExec)=' "$f" | sed 's/^/   /'
    # Starý spouštěč (Exec=di_player) zakrývá Flatpak. Uživatelský smažeme rovnou.
    if grep -qE '^Exec=di_player' "$f"; then
        if [ -w "$f" ] && [ "$d" = "$HOME/.local/share/applications" ]; then
            rm -f "$f" && echo "   -> starý spouštěč smazán" && fixed=1
        else
            echo "   -> starý systémový spouštěč, smažte ho: sudo rm '$f'"
        fi
    fi
done
[ "$fixed" = 1 ] && update-desktop-database "$HOME/.local/share/applications" 2>/dev/null

echo "== Složky aplikací v sezení (XDG_DATA_DIRS)"
case ":${XDG_DATA_DIRS:-}:" in
    *flatpak/exports/share*) echo "Flatpak složky jsou v sezení vidět." ;;
    *) echo "Flatpak složky v sezení chybí -> odhlaste se a znovu přihlaste." ;;
esac

if command -v rpm >/dev/null && rpm -q di-player >/dev/null 2>&1; then
    echo "== Nainstalovaný je i starý balíček RPM: sudo dnf remove di-player"
fi
if command -v dpkg >/dev/null && dpkg -s di-player >/dev/null 2>&1; then
    echo "== Nainstalovaný je i starý balíček DEB: sudo apt remove di-player"
fi
echo "Hotovo."
