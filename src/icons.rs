//! Bundled DI Player application icon helpers.
//!
//! Icons are embedded in the binary and always written out at runtime so a
//! broken/empty install under ~/.local cannot poison the tray or window icon.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

pub const APP_ICON_NAME: &str = "com.alchymista.DiPlayer";

const ICON_SVG: &[u8] =
    include_bytes!("../resources/icons/hicolor/scalable/apps/com.alchymista.DiPlayer.svg");
const ICON_PNG_16: &[u8] =
    include_bytes!("../resources/icons/hicolor/16x16/apps/com.alchymista.DiPlayer.png");
const ICON_PNG_32: &[u8] =
    include_bytes!("../resources/icons/hicolor/32x32/apps/com.alchymista.DiPlayer.png");
const ICON_PNG_48: &[u8] =
    include_bytes!("../resources/icons/hicolor/48x48/apps/com.alchymista.DiPlayer.png");
const ICON_PNG_64: &[u8] =
    include_bytes!("../resources/icons/hicolor/64x64/apps/com.alchymista.DiPlayer.png");
const ICON_PNG_128: &[u8] =
    include_bytes!("../resources/icons/hicolor/128x128/apps/com.alchymista.DiPlayer.png");
const ICON_PNG_256: &[u8] =
    include_bytes!("../resources/icons/hicolor/256x256/apps/com.alchymista.DiPlayer.png");

const HICOLOR_INDEX_THEME: &str = r#"[Icon Theme]
Name=Hicolor
Comment=Fallback icon theme
Hidden=true
Directories=16x16/apps,32x32/apps,48x48/apps,64x64/apps,128x128/apps,256x256/apps,scalable/apps

[16x16/apps]
Size=16
Context=Applications
Type=Fixed

[32x32/apps]
Size=32
Context=Applications
Type=Fixed

[48x48/apps]
Size=48
Context=Applications
Type=Fixed

[64x64/apps]
Size=64
Context=Applications
Type=Fixed

[128x128/apps]
Size=128
Context=Applications
Type=Fixed

[256x256/apps]
Size=256
Context=Applications
Type=Fixed

[scalable/apps]
Size=128
Context=Applications
Type=Scalable
MinSize=16
MaxSize=512
"#;

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
pub fn install_user_icons() {
    let hicolor = hicolor_root();
    write_file(hicolor.join("index.theme").as_path(), HICOLOR_INDEX_THEME.as_bytes());
    write_file(
        &hicolor
            .join("scalable/apps")
            .join(format!("{APP_ICON_NAME}.svg")),
        ICON_SVG,
    );
    write_file(&png_name(&hicolor.join("16x16/apps")), ICON_PNG_16);
    write_file(&png_name(&hicolor.join("32x32/apps")), ICON_PNG_32);
    write_file(&png_name(&hicolor.join("48x48/apps")), ICON_PNG_48);
    write_file(&png_name(&hicolor.join("64x64/apps")), ICON_PNG_64);
    write_file(&png_name(&hicolor.join("128x128/apps")), ICON_PNG_128);
    write_file(&png_name(&hicolor.join("256x256/apps")), ICON_PNG_256);

    // Flat tray directory — AppIndicator resolves IconName via IconThemePath.
    let tray = tray_icon_theme_path();
    write_file(&tray.join(format!("{APP_ICON_NAME}.png")), ICON_PNG_64);
    write_file(&tray.join(format!("{APP_ICON_NAME}.svg")), ICON_SVG);

    // Refresh icon cache so GTK/GNOME pick up the files immediately.
    refresh_icon_cache(&hicolor);

    // Keep desktop entry Icon= on an absolute PNG path (most reliable in GNOME).
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
    let icon = hicolor_root()
        .join("128x128/apps")
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

/// ARGB32 pixmap for StatusNotifierItem.
pub fn tray_icon_pixmap() -> Vec<ksni::Icon> {
    let Ok(img) = image::load_from_memory_with_format(ICON_PNG_64, image::ImageFormat::Png) else {
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
