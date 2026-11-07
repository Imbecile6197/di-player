# DI Player

Native GNOME (GTK4 + libadwaita) internet radio player for [DI.fm](https://www.di.fm) premium accounts.

![License](https://img.shields.io/badge/license-GPL--3.0-blue)

## Features

- Premium login (email/password) or paste a listen key
- Channel browser with genre filters, search, and cover art
- Now-playing metadata
- mpv-backed streaming (premium 320 kbps MP3)
- StatusNotifierItem system tray (show/hide, play/pause, stop, quit)
- Close-to-tray; credentials stored in `~/.config/di-player/`

## Install (Fedora / Ubuntu)

### One-liner from this repository

```bash
git clone https://github.com/Imbecile6197/di-player.git
cd di-player
./scripts/install.sh --deps
```


`--deps` installs build packages with `sudo` (Fedora `dnf` or Ubuntu `apt`), then builds a release binary into `~/.local/bin/di_player` and registers the desktop entry + icons.

Afterwards run:

```bash
di_player
```

Or find **DI Player** in the app menu.

### Manual dependencies

**Fedora**

```bash
sudo dnf install gcc pkgconf-pkg-config openssl-devel \
  gtk4-devel libadwaita-devel mpv cargo rust
```

**Ubuntu / Debian**

```bash
sudo apt update
sudo apt install build-essential pkg-config libssl-dev \
  libgtk-4-dev libadwaita-1-dev mpv cargo rustc
```

Then:

```bash
cargo build --release
./scripts/install.sh
# or: make install
```

### Uninstall

```bash
make uninstall
```

## System tray (GNOME)

Enable the **AppIndicator and KStatusNotifierItem** extension (Fedora package `gnome-shell-extension-appindicator`).

## Login

1. Sign in with your DI.fm premium email and password, **or**
2. Paste the listen key from DI.fm → Member settings → Hardware player.

## Development

```bash
cargo run
cargo test
make release
```

## License

GPL-3.0-or-later
