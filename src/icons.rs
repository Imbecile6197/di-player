//! Bundled DI Player application icon helpers.
//!
//! Icons are embedded in the binary and always written out at runtime so a
//! broken/empty install under ~/.local cannot poison the tray or window icon.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

pub const APP_ICON_NAME: &str = "com.alchymista.DiPlayer";
/// Monochrome symbolic icon used in the system tray.
pub const TRAY_ICON_NAME: &str = "com.alchymista.DiPlayer-symbolic";

const ICON_SVG: &[u8] =
    include_bytes!("../resources/icons/hicolor/scalable/apps/com.alchymista.DiPlayer.svg");
const ICON_SYMBOLIC_SVG: &[u8] =
    include_bytes!("../resources/icons/hicolor/scalable/apps/com.alchymista.DiPlayer-symbolic.svg");

const ICON_PNG_16: &[u8] =
    include_bytes!("../resources/icons/hicolor/16x16/apps/com.alchymista.DiPlayer.png");
const ICON_PNG_22: &[u8] =
    include_bytes!("../resources/icons/hicolor/22x22/apps/com.alchymista.DiPlayer.png");
const ICON_PNG_24: &[u8] =
    include_bytes!("../resources/icons/hicolor/24x24/apps/com.alchymista.DiPlayer.png");
const ICON_PNG_32: &[u8] =
    include_bytes!("../resources/icons/hicolor/32x32/apps/com.alchymista.DiPlayer.png");
const ICON_PNG_48: &[u8] =
    include_bytes!("../resources/icons/hicolor/48x48/apps/com.alchymista.DiPlayer.png");
const ICON_PNG_64: &[u8] =
    include_bytes!("../resources/icons/hicolor/64x64/apps/com.alchymista.DiPlayer.png");
const ICON_PNG_96: &[u8] =
    include_bytes!("../resources/icons/hicolor/96x96/apps/com.alchymista.DiPlayer.png");
const ICON_PNG_128: &[u8] =
    include_bytes!("../resources/icons/hicolor/128x128/apps/com.alchymista.DiPlayer.png");
const ICON_PNG_256: &[u8] =
    include_bytes!("../resources/icons/hicolor/256x256/apps/com.alchymista.DiPlayer.png");
const ICON_PNG_512: &[u8] =
    include_bytes!("../resources/icons/hicolor/512x512/apps/com.alchymista.DiPlayer.png");
const ICON_SYMBOLIC_PNG_64: &[u8] =
    include_bytes!("../resources/icons/hicolor/64x64/apps/com.alchymista.DiPlayer-symbolic.png");

const APP_PNG_SIZES: &[(u16, &[u8])] = &[
    (16, ICON_PNG_16),
    (22, ICON_PNG_22),
    (24, ICON_PNG_24),
    (32, ICON_PNG_32),
    (48, ICON_PNG_48),
    (64, ICON_PNG_64),
    (96, ICON_PNG_96),
    (128, ICON_PNG_128),
    (256, ICON_PNG_256),
    (512, ICON_PNG_512),
];

/// Directory that StatusNotifier hosts should add to the icon theme search path.
pub fn tray_icon_theme_path() -> PathBuf {
    data_icons_root().join("tray")
}

fn data_icons_root() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("di-player/icons")
}

fn hicolor_root() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("icons/hicolor")
}

fn write_file(path: &Path, bytes: &[u8]) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let tmp = path.with_extension("tmp");
    if let Ok(mut f) = fs::File::create(&tmp) {
        if f.write_all(bytes).is_ok() && f.flush().is_ok() {
            let _ = fs::rename(&tmp, path);
            return;
        }
    }
    let _ = fs::remove_file(&tmp);
}

fn png_name(dir: &Path) -> PathBuf {
    dir.join(format!("{APP_ICON_NAME}.png"))
}

/// Install icons for GTK, the desktop entry, and the system tray.
///
/// Writes only app icon files under `~/.local/share/icons/hicolor/.../apps/`.
/// Never writes `index.theme` — a partial user hicolor index shadows the system
/// one and breaks other apps' bundled action icons (e.g. PinkSky).
pub fn install_user_icons() {
    let hicolor = hicolor_root();
    write_file(
        &hicolor
            .join("scalable/apps")
            .join(format!("{APP_ICON_NAME}.svg")),
        ICON_SVG,
    );

    // Write every common size — GNOME Shell prefers large PNGs (256/512) when present.
    // Leaving stale oversized files behind keeps the old colourful mark in the app grid.
    for (size, bytes) in APP_PNG_SIZES {
        write_file(
            &png_name(&hicolor.join(format!("{size}x{size}/apps"))),
            bytes,
        );
    }

    // Symbolic (monochrome) icons for status area / AppIndicator.
    write_file(
        &hicolor
            .join("scalable/apps")
            .join(format!("{TRAY_ICON_NAME}.svg")),
        ICON_SYMBOLIC_SVG,
    );
    write_file(
        &hicolor
            .join("64x64/apps")
            .join(format!("{TRAY_ICON_NAME}.png")),
        ICON_SYMBOLIC_PNG_64,
    );

    // Flat tray directory — AppIndicator resolves IconName via IconThemePath.
    let tray = tray_icon_theme_path();
    write_file(&tray.join(format!("{TRAY_ICON_NAME}.png")), ICON_SYMBOLIC_PNG_64);
    write_file(&tray.join(format!("{TRAY_ICON_NAME}.svg")), ICON_SYMBOLIC_SVG);
    // Keep app name as fallback for hosts that ignore -symbolic.
    write_file(&tray.join(format!("{APP_ICON_NAME}.png")), ICON_PNG_64);

    refresh_icon_cache(&hicolor);
    ensure_desktop_entry();
}

fn refresh_icon_cache(hicolor: &Path) {
    let _ = std::process::Command::new("gtk4-update-icon-cache")
        .args(["-f", &hicolor.to_string_lossy()])
        .status();
    let _ = std::process::Command::new("gtk-update-icon-cache")
        .args(["-f", &hicolor.to_string_lossy()])
        .status();
}

fn ensure_desktop_entry() {
    let Some(apps) = dirs::data_dir().map(|d| d.join("applications")) else {
        return;
    };
    let _ = fs::create_dir_all(&apps);
    // Absolute 512px path so GNOME Shell picks the new mark even when its
    // themed-icon cache still holds an older size under the same name.
    let icon = hicolor_root()
        .join("512x512/apps")
        .join(format!("{APP_ICON_NAME}.png"));
    let icon = icon.display();
    let desktop = format!(
        "\
[Desktop Entry]
Name=DI Player
GenericName=Internet Radio
Comment=Native GNOME player for DI.fm premium radio
Exec=di_player
Icon={icon}
Terminal=false
Type=Application
Categories=AudioVideo;Audio;Player;Tuner;
Keywords=radio;di.fm;digitally;imported;electronic;
StartupNotify=true
StartupWMClass=com.alchymista.DiPlayer
"
    );
    write_file(
        &apps.join("com.alchymista.DiPlayer.desktop"),
        desktop.as_bytes(),
    );
    let _ = std::process::Command::new("update-desktop-database")
        .arg(&apps)
        .status();
}

/// Parent directory of hicolor — useful for GtkIconTheme::add_search_path.
pub fn bundled_hicolor_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("icons")
}

pub fn icon_png_128() -> &'static [u8] {
    ICON_PNG_128
}

/// Absolute path to the installed 128px icon (for debugging / desktop files).
pub fn installed_icon_path() -> PathBuf {
    hicolor_root()
        .join("128x128/apps")
        .join(format!("{APP_ICON_NAME}.png"))
}

/// ARGB32 pixmap for StatusNotifierItem (monochrome symbolic).
pub fn tray_icon_pixmap() -> Vec<ksni::Icon> {
    let Ok(img) =
        image::load_from_memory_with_format(ICON_SYMBOLIC_PNG_64, image::ImageFormat::Png)
    else {
        return Vec::new();
    };
    let (width, height) = image::GenericImageView::dimensions(&img);
    let mut data = img.into_rgba8().into_vec();
    for pixel in data.chunks_exact_mut(4) {
        pixel.rotate_right(1); // RGBA → ARGB
    }
    vec![ksni::Icon {
        width: width as i32,
        height: height as i32,
        data,
    }]
}
