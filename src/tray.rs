use crate::i18n;
use crate::icons::{self, TRAY_ICON_NAME};
use ksni::blocking::{Handle, TrayMethods};
use ksni::menu::StandardItem;
use ksni::{Icon, MenuItem};
use std::sync::mpsc::Sender;

#[derive(Debug, Clone)]
pub enum TrayCommand {
    ToggleWindow,
    PlayPause,
    Stop,
    Quit,
}

pub struct DiTray {
    pub playing: bool,
    pub channel: String,
    pub track: String,
    tx: Sender<TrayCommand>,
    icon_pixmap: Vec<Icon>,
}

impl DiTray {
    pub fn new(tx: Sender<TrayCommand>) -> Self {
        Self {
            playing: false,
            channel: String::new(),
            track: String::new(),
            tx,
            icon_pixmap: icons::tray_icon_pixmap(),
        }
    }
}

impl ksni::Tray for DiTray {
    fn id(&self) -> String {
        "com.alchymista.DiPlayer".into()
    }

    fn title(&self) -> String {
        i18n::t("app-title")
    }

    fn icon_name(&self) -> String {
        TRAY_ICON_NAME.into()
    }

    fn icon_theme_path(&self) -> String {
        icons::tray_icon_theme_path().to_string_lossy().into_owned()
    }

    fn icon_pixmap(&self) -> Vec<Icon> {
        self.icon_pixmap.clone()
    }

    fn tool_tip(&self) -> ksni::ToolTip {
        let description = if !self.track.is_empty() && !self.channel.is_empty() {
            format!("{} — {}", self.channel, self.track)
        } else if !self.channel.is_empty() {
            self.channel.clone()
        } else if !self.track.is_empty() {
            self.track.clone()
        } else {
            i18n::t("tray-tooltip")
        };

        ksni::ToolTip {
            title: i18n::t("app-title"),
            description,
            icon_name: TRAY_ICON_NAME.into(),
            icon_pixmap: self.icon_pixmap.clone(),
        }
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        let _ = self.tx.send(TrayCommand::ToggleWindow);
    }

    fn secondary_activate(&mut self, _x: i32, _y: i32) {
        let _ = self.tx.send(TrayCommand::PlayPause);
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let play_label = if self.playing {
            i18n::t("tray-pause")
        } else {
            i18n::t("tray-play")
        };
        let play_icon = if self.playing {
            "media-playback-pause-symbolic"
        } else {
            "media-playback-start-symbolic"
        };

        vec![
            StandardItem {
                label: i18n::t("tray-show"),
                icon_name: "view-reveal-symbolic".into(),
                activate: Box::new(|this: &mut Self| {
                    let _ = this.tx.send(TrayCommand::ToggleWindow);
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: play_label,
                icon_name: play_icon.into(),
                activate: Box::new(|this: &mut Self| {
                    let _ = this.tx.send(TrayCommand::PlayPause);
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: i18n::t("tray-stop"),
                icon_name: "media-playback-stop-symbolic".into(),
                activate: Box::new(|this: &mut Self| {
                    let _ = this.tx.send(TrayCommand::Stop);
                }),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: i18n::t("tray-quit"),
                icon_name: "application-exit-symbolic".into(),
                activate: Box::new(|this: &mut Self| {
                    let _ = this.tx.send(TrayCommand::Quit);
                }),
                ..Default::default()
            }
            .into(),
        ]
    }
}

pub fn spawn_service(tx: Sender<TrayCommand>) -> Result<Handle<DiTray>, ksni::Error> {
    icons::install_user_icons();
    DiTray::new(tx)
        .assume_sni_available(true)
        .spawn()
}

pub fn update_now_playing(
    handle: &Handle<DiTray>,
    playing: bool,
    channel: &str,
    track: &str,
) {
    handle.update(|tray| {
        tray.playing = playing;
        tray.channel = channel.to_string();
        tray.track = track.to_string();
    });
}

pub fn refresh_menu(handle: &Handle<DiTray>) {
    handle.update(|_| {});
}
