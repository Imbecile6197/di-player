# DI Player

Native GNOME (GTK4 + libadwaita) internet radio player for [DI.fm](https://www.di.fm) premium accounts.

## Install (Flatpak)

```bash
flatpak install --user https://imbecile6197.github.io/di-player/di-player.flatpakref
```

Or open https://imbecile6197.github.io/di-player/ and click **Install** (opens GNOME Software).
Updates come through Software or `flatpak update`. Works on any distribution with Flatpak
and Flathub (the GNOME runtime is pulled from Flathub).

### Moving from the RPM/DEB packages

The old dnf/apt repository is no longer updated. Install the Flatpak, then remove the package
and its source:

```bash
sudo dnf remove di-player && sudo rm /etc/yum.repos.d/di-player.repo          # Fedora
sudo apt remove di-player && sudo rm /etc/apt/sources.list.d/di-player.list   # Ubuntu
```

Your login and settings from `~/.config/di-player/` are copied into the Flatpak on first start.

## Build from source

```bash
git clone https://github.com/Imbecile6197/di-player.git
cd di-player
./scripts/install.sh --deps
```

## Features

- Premium login (email/password) or paste a listen key
- Channel browser with genre filters, search, favorites, list/grid views, and cover art
- Decorative visualizer (bars, wave, sine, circular, particles, mountain, oscilloscope, dots)
- Themes: Neon Club, Soft Night, Monochrome, Minimal
- Cover-driven accent colors, blurred backdrop, rotating cover while playing
- Fullscreen Now Playing (F11) and keyboard shortcuts
- UI languages: English, Čeština, Deutsch (or system default)
- Now-playing metadata
- mpv-backed streaming with selectable premium bitrate (320 / 128 / 64 kbps)
- StatusNotifierItem system tray (show/hide, play/pause, stop, quit)
- Close-to-tray; credentials in `~/.config/di-player/` (Flatpak: `~/.var/app/com.alchymista.DiPlayer/config/di-player/`)

## System tray (GNOME)

Enable **AppIndicator and KStatusNotifierItem**  
(`sudo dnf install gnome-shell-extension-appindicator` on Fedora).

## Login

1. DI.fm premium email/password, **or**
2. Listen key from DI.fm → Member settings → Hardware player.

## Maintainers

```bash
./packaging/flatpak/build.sh   # signed Flatpak repo + dist/di-player-<version>.flatpak
./packaging/flatpak/pages.sh   # publish to GitHub Pages (gh-pages)
```

## License

GPL-3.0-or-later
