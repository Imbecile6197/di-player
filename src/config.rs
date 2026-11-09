use crate::quality::StreamQuality;
use crate::theme::{AppLocale, ChannelViewMode, UiTheme};
use crate::visualizer::VizStyle;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Credentials {
    pub email: String,
    pub listen_key: String,
    pub api_key: String,
    pub member_id: u64,
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub last_channel_key: Option<String>,
    pub volume: f64,
    /// Legacy string form kept for older settings.json files.
    #[serde(default = "default_quality_key")]
    pub quality: String,
    pub close_to_tray: bool,
    #[serde(default = "default_window_width")]
    pub window_width: i32,
    #[serde(default = "default_window_height")]
    pub window_height: i32,
    #[serde(default = "default_true")]
    pub viz_enabled: bool,
    #[serde(default)]
    pub viz_style: String,
    #[serde(default)]
    pub theme: String,
    #[serde(default)]
    pub channel_view: String,
    #[serde(default)]
    pub locale: String,
    #[serde(default)]
    pub favorites: Vec<String>,
    #[serde(default)]
    pub muted: bool,
    #[serde(default)]
    pub volume_before_mute: f64,
}

fn default_quality_key() -> String {
    StreamQuality::PremiumHigh.api_key().into()
}

fn default_window_width() -> i32 {
    1100
}

fn default_window_height() -> i32 {
    740
}

fn default_true() -> bool {
    true
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            last_channel_key: None,
            volume: 0.85,
            quality: default_quality_key(),
            close_to_tray: true,
            window_width: default_window_width(),
            window_height: default_window_height(),
            viz_enabled: true,
            viz_style: VizStyle::Bars.id().into(),
            theme: UiTheme::Neon.id().into(),
            channel_view: ChannelViewMode::List.id().into(),
            locale: AppLocale::System.id().into(),
            favorites: Vec::new(),
            muted: false,
            volume_before_mute: 0.85,
        }
    }
}

impl AppSettings {
    pub fn stream_quality(&self) -> StreamQuality {
        StreamQuality::from_api_key(&self.quality)
    }

    pub fn set_stream_quality(&mut self, quality: StreamQuality) {
        self.quality = quality.api_key().into();
    }

    pub fn viz_style(&self) -> VizStyle {
        VizStyle::from_id(&self.viz_style)
    }

    pub fn set_viz_style(&mut self, style: VizStyle) {
        self.viz_style = style.id().into();
    }

    pub fn ui_theme(&self) -> UiTheme {
        UiTheme::from_id(&self.theme)
    }

    pub fn set_ui_theme(&mut self, theme: UiTheme) {
        self.theme = theme.id().into();
    }

    pub fn channel_view_mode(&self) -> ChannelViewMode {
        ChannelViewMode::from_id(&self.channel_view)
    }

    pub fn set_channel_view_mode(&mut self, mode: ChannelViewMode) {
        self.channel_view = mode.id().into();
    }

    pub fn app_locale(&self) -> AppLocale {
        AppLocale::from_id(&self.locale)
    }

    pub fn set_app_locale(&mut self, locale: AppLocale) {
        self.locale = locale.id().into();
    }

    pub fn is_favorite(&self, key: &str) -> bool {
        self.favorites.iter().any(|k| k == key)
    }

    pub fn toggle_favorite(&mut self, key: &str) -> bool {
        if let Some(pos) = self.favorites.iter().position(|k| k == key) {
            self.favorites.remove(pos);
            false
        } else {
            self.favorites.push(key.to_string());
            true
        }
    }
}

fn config_dir() -> Result<PathBuf> {
    let dir = dirs::config_dir()
        .context("no XDG config directory")?
        .join("di-player");
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

impl AppSettings {
    fn path() -> Result<PathBuf> {
        Ok(config_dir()?.join("settings.json"))
    }

    pub fn load() -> Self {
        let Ok(path) = Self::path() else {
            return Self::default();
        };
        fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::path()?;
        fs::write(path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }
}

fn credentials_path() -> Result<PathBuf> {
    Ok(config_dir()?.join("credentials.json"))
}

pub fn save_credentials(creds: &Credentials) -> Result<()> {
    let path = credentials_path()?;
    fs::write(&path, serde_json::to_string_pretty(creds)?)?;
    let mut perms = fs::metadata(&path)?.permissions();
    perms.set_mode(0o600);
    fs::set_permissions(&path, perms)?;
    Ok(())
}

pub fn load_credentials() -> Option<Credentials> {
    let path = credentials_path().ok()?;
    let data = fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}

pub fn clear_credentials() -> Result<()> {
    let path = credentials_path()?;
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

pub fn is_logged_in() -> bool {
    load_credentials()
        .map(|c| !c.listen_key.is_empty())
        .unwrap_or(false)
}
