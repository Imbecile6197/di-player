//! Clean-up after the pre-Flatpak releases.
//!
//! Versions up to 0.2.0 wrote their own launcher to
//! `~/.local/share/applications/com.alchymista.DiPlayer.desktop` on every start
//! (`Exec=di_player`). That user-level file has priority over the entry the
//! Flatpak exports, so after switching to the Flatpak GNOME either hid the app
//! (the binary no longer exists) or kept starting the old one, with a generic
//! icon. The Flatpak gets write access to exactly that file and rewrites it to
//! launch the Flatpak.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

pub const APP_ID: &str = "com.alchymista.DiPlayer";

/// Launcher equivalent to the one the Flatpak exports.
pub fn flatpak_desktop_entry() -> String {
    format!(
        "[Desktop Entry]
Name=DI Player
GenericName=Internet Radio
Comment=Native GNOME player for DI.fm premium radio
Exec=flatpak run {APP_ID}
Icon={APP_ID}
Terminal=false
Type=Application
Categories=AudioVideo;Audio;Player;Tuner;
Keywords=radio;di.fm;digitally;imported;electronic;
StartupNotify=true
StartupWMClass={APP_ID}
X-Flatpak={APP_ID}
"
    )
}

/// True for a launcher written by an old non-Flatpak DI Player.
pub fn is_stale_entry(contents: &str) -> bool {
    contents.lines().any(|l| l.trim() == "Exec=di_player")
}

/// Rewrite a stale launcher at `path` in place. The file is bind-mounted into
/// the sandbox, so it is truncated and rewritten rather than replaced.
pub fn repair_desktop_entry(path: &Path) -> bool {
    let Ok(contents) = std::fs::read_to_string(path) else {
        return false;
    };
    if !is_stale_entry(&contents) {
        return false;
    }
    let Ok(mut file) = OpenOptions::new().write(true).truncate(true).open(path) else {
        return false;
    };
    file.write_all(flatpak_desktop_entry().as_bytes()).is_ok()
}

/// Run at start-up inside the Flatpak only.
pub fn repair_host_launcher() {
    if std::env::var_os("FLATPAK_ID").is_none() {
        return;
    }
    if let Some(home) = dirs::home_dir() {
        let path = home
            .join(".local/share/applications")
            .join(format!("{APP_ID}.desktop"));
        if repair_desktop_entry(&path) {
            eprintln!("DI Player: replaced the old launcher {}", path.display());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewrites_only_stale_launchers() {
        let dir = std::env::temp_dir().join(format!("di-player-legacy-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let stale = dir.join("stale.desktop");
        std::fs::write(&stale, "[Desktop Entry]\nName=DI Player\nExec=di_player\nIcon=/x.png\n").unwrap();
        assert!(repair_desktop_entry(&stale));
        let fixed = std::fs::read_to_string(&stale).unwrap();
        assert!(fixed.contains("Exec=flatpak run com.alchymista.DiPlayer"));
        assert!(!is_stale_entry(&fixed));
        assert!(!repair_desktop_entry(&stale), "second run must be a no-op");
        assert!(!repair_desktop_entry(&dir.join("missing.desktop")));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
