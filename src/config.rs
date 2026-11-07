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
    pub quality: String,
    pub close_to_tray: bool,
    #[serde(default = "default_window_width")]
    pub window_width: i32,
    #[serde(default = "default_window_height")]
    pub window_height: i32,
}

fn default_window_width() -> i32 {
    1100
}

fn default_window_height() -> i32 {
    740
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            last_channel_key: None,
            volume: 0.85,
            quality: "premium_high".into(),
            close_to_tray: true,
            window_width: default_window_width(),
            window_height: default_window_height(),
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
