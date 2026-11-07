# DI Player

Native GNOME (GTK4 + libadwaita) internet radio player for [DI.fm](https://www.di.fm) premium accounts.

## Install from packages (recommended)

Download the latest release assets from  
**https://github.com/Imbecile6197/di-player/releases**

### Fedora

```bash
sudo dnf install ./di-player-0.1.0-1.fc45.x86_64.rpm
# runtime deps pulled automatically: gtk4, libadwaita, mpv
di_player
```

### Ubuntu (25.04+)

```bash
sudo apt update
sudo apt install ./di-player_0.1.0_amd64.deb
# or: sudo apt install ./di-player_0.1.0_amd64.deb mpv
di_player
```

> The `.deb` is built on Ubuntu 25.04 (libadwaita ≥ 1.6, glibc 2.39).  
> For Ubuntu 24.04 or older, use the source install below.

### Build from source (Fedora / Ubuntu)

```bash
git clone https://github.com/Imbecile6197/di-player.git
cd di-player
./scripts/install.sh --deps
```

`--deps` installs build packages with `sudo`, builds a release binary into
`~/.local/bin/di_player`, and registers the desktop entry + icons.

## Features

- Premium login (email/password) or paste a listen key
- Channel browser with genre filters, search, and cover art
- Now-playing metadata
- mpv-backed streaming (premium 320 kbps MP3)
- StatusNotifierItem system tray (show/hide, play/pause, stop, quit)
- Close-to-tray; credentials stored in `~/.config/di-player/`

## System tray (GNOME)

Enable the **AppIndicator and KStatusNotifierItem** extension  
(Fedora: `sudo dnf install gnome-shell-extension-appindicator`).

## Login

1. Sign in with your DI.fm premium email and password, **or**
2. Paste the listen key from DI.fm → Member settings → Hardware player.

## Packaging (maintainers)

```bash
# Fedora .rpm + Ubuntu .deb (needs podman)
./scripts/build-packages.sh
# artifacts land in dist/
```

## Development

```bash
cargo run
cargo test
make release
make uninstall   # remove ~/.local install
```

## License

GPL-3.0-or-later
