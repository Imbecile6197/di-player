mod ui;

use di_player::icons::{self, APP_ICON_NAME};
use gtk4::prelude::*;
use libadwaita as adw;

const APP_ID: &str = "com.alchymista.DiPlayer";

fn main() {
    // Write real icon files before GTK/tray start (fixes empty 0-byte installs).
    icons::install_user_icons();

    let app = adw::Application::builder().application_id(APP_ID).build();

    app.connect_startup(|_| {
        adw::StyleManager::default().set_color_scheme(adw::ColorScheme::PreferDark);

        if let Some(display) = gdk4::Display::default() {
            let theme = gtk4::IconTheme::for_display(&display);
            theme.add_search_path(icons::bundled_hicolor_dir());
            theme.add_search_path(icons::tray_icon_theme_path());
            if let Some(data) = dirs::data_dir() {
                theme.add_search_path(data.join("icons"));
            }
        }

        let provider = gtk4::CssProvider::new();
        provider.load_from_string(include_str!("style.css"));
        if let Some(display) = gdk4::Display::default() {
            gtk4::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }
    });

    app.connect_activate(|app| {
        if let Some(win) = app.active_window() {
            win.present();
            return;
        }
        let ui = ui::DiPlayerWindow::new(app);
        ui.window.set_icon_name(Some(APP_ICON_NAME));
        ui.window.present();
    });

    let code = app.run();
    std::process::exit(code.into());
}
