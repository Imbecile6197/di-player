# DI Player

Native GNOME (GTK4 + libadwaita) internet radio player for [DI.fm](https://www.di.fm) premium accounts.

## Add package source & install

### Fedora

```bash
curl -fsSL https://imbecile6197.github.io/di-player/add-repo-fedora.sh | sudo bash
sudo dnf install di-player
```

Or manually:

```bash
sudo curl -fsSL https://imbecile6197.github.io/di-player/di-player.repo \
  -o /etc/yum.repos.d/di-player.repo
sudo dnf install di-player
```

### Ubuntu

```bash
curl -fsSL https://imbecile6197.github.io/di-player/add-repo-ubuntu.sh | sudo bash
sudo apt install di-player
```

Or manually:

```bash
echo 'deb [trusted=yes arch=amd64] https://imbecile6197.github.io/di-player/deb stable main' \
  | sudo tee /etc/apt/sources.list.d/di-player.list
sudo apt update
sudo apt install di-player
```

> Ubuntu packages need **25.04+** (libadwaita ≥ 1.6). On older Ubuntu use source install below.

Then run `di_player` or open **DI Player** from the app menu.

Package repo / releases:  
https://imbecile6197.github.io/di-player/ · https://github.com/Imbecile6197/di-player/releases

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
- Close-to-tray; credentials in `~/.config/di-player/`

## System tray (GNOME)

Enable **AppIndicator and KStatusNotifierItem**  
(`sudo dnf install gnome-shell-extension-appindicator` on Fedora).

## Login

1. DI.fm premium email/password, **or**
2. Listen key from DI.fm → Member settings → Hardware player.

## Maintainers

```bash
./scripts/build-packages.sh      # dist/*.rpm + dist/*.deb
./scripts/publish-pages-repo.sh  # site/ for GitHub Pages
```

## License

GPL-3.0-or-later
