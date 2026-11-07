use super::models::{
    AuthResponse, Channel, ChannelFilter, CurrentlyPlaying, StreamChannel,
};
use crate::config::Credentials;
use reqwest::blocking::Client;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use thiserror::Error;

const API_BASE: &str = "https://api.audioaddict.com/v1/di";
const LISTEN_BASE: &str = "https://listen.di.fm";
/// Public AudioAddict mobile client credentials (ephemeron).
const BASIC_AUTH: &str = "Basic ZXBoZW1lcm9uOmRheWVpcGgwbmVAcHA=";
const APP_UA: &str = "DiPlayer/0.1 (GNOME; Rust)";

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("authentication failed: {0}")]
    Auth(String),
    #[error("API error ({0}): {1}")]
    Status(u16, String),
    #[error("invalid response: {0}")]
    Parse(String),
    #[error("{0}")]
    Message(String),
}

#[derive(Clone)]
pub struct DiClient {
    http: Client,
}

impl DiClient {
    pub fn new() -> Result<Self, ApiError> {
        let http = Client::builder()
            .user_agent(APP_UA)
            .timeout(std::time::Duration::from_secs(30))
            .build()?;
        Ok(Self { http })
    }

    pub fn authenticate(&self, username: &str, password: &str) -> Result<Credentials, ApiError> {
        let response = self
            .http
            .post(format!("{API_BASE}/members/authenticate"))
            .header(AUTHORIZATION, BASIC_AUTH)
            .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
            .form(&[("username", username), ("password", password)])
            .send()?;

        let status = response.status();
        let body = response.text().unwrap_or_default();

        if !status.is_success() {
            let msg = body.trim().to_string();
            return Err(ApiError::Auth(if msg.is_empty() {
                format!("HTTP {}", status.as_u16())
            } else {
                msg
            }));
        }

        let auth: AuthResponse = serde_json::from_str(&body)
            .map_err(|e| ApiError::Parse(format!("auth JSON: {e}")))?;

        let listen_key = auth
            .listen_key
            .filter(|k| !k.is_empty())
            .ok_or_else(|| ApiError::Auth("missing listen_key in response".into()))?;

        Ok(Credentials {
            email: auth.email.unwrap_or_else(|| username.to_string()),
            listen_key,
            api_key: auth.api_key.unwrap_or_default(),
            member_id: auth.id.unwrap_or(0),
            display_name: match (auth.first_name, auth.last_name) {
                (Some(f), Some(l)) => format!("{f} {l}").trim().to_string(),
                (Some(f), None) => f,
                _ => username.to_string(),
            },
        })
    }

    pub fn channels(&self) -> Result<Vec<Channel>, ApiError> {
        let response = self
            .http
            .get(format!("{API_BASE}/channels"))
            .header(AUTHORIZATION, BASIC_AUTH)
            .send()?;
        Self::json(response)
    }

    pub fn channel_filters(&self) -> Result<Vec<ChannelFilter>, ApiError> {
        let response = self
            .http
            .get(format!("{API_BASE}/channel_filters"))
            .header(AUTHORIZATION, BASIC_AUTH)
            .send()?;
        Self::json(response)
    }

    pub fn stream_channels(&self) -> Result<Vec<StreamChannel>, ApiError> {
        let response = self
            .http
            .get(format!("{LISTEN_BASE}/premium_high.json"))
            .send()?;
        Self::json(response)
    }

    pub fn currently_playing(&self) -> Result<Vec<CurrentlyPlaying>, ApiError> {
        let response = self
            .http
            .get(format!("{API_BASE}/currently_playing"))
            .header(AUTHORIZATION, BASIC_AUTH)
            .send()?;
        Self::json(response)
    }

    /// Fetch a PLS playlist and return the first stream URL with `listen_key` attached.
    pub fn resolve_stream_url(&self, playlist_url: &str, listen_key: &str) -> Result<String, ApiError> {
        let response = self.http.get(playlist_url).send()?;
        let status = response.status();
        let body = response.text().unwrap_or_default();
        if !status.is_success() {
            return Err(ApiError::Status(status.as_u16(), body));
        }

        let stream = parse_pls_file1(&body).ok_or_else(|| {
            ApiError::Message("playlist did not contain a stream URL".into())
        })?;
        Ok(attach_listen_key(&stream, listen_key))
    }

    pub fn stream_url_for_key(&self, channel_key: &str, listen_key: &str) -> Result<String, ApiError> {
        let playlist = format!("{LISTEN_BASE}/premium_high/{channel_key}.pls");
        self.resolve_stream_url(&playlist, listen_key)
    }

    pub fn fetch_bytes(&self, url: &str) -> Result<Vec<u8>, ApiError> {
        let response = self.http.get(url).send()?;
        let status = response.status();
        if !status.is_success() {
            return Err(ApiError::Status(
                status.as_u16(),
                format!("failed to download {url}"),
            ));
        }
        Ok(response.bytes()?.to_vec())
    }

    fn json<T: serde::de::DeserializeOwned>(response: reqwest::blocking::Response) -> Result<T, ApiError> {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        if !status.is_success() {
            return Err(ApiError::Status(status.as_u16(), body));
        }
        serde_json::from_str(&body).map_err(|e| ApiError::Parse(e.to_string()))
    }
}

fn parse_pls_file1(body: &str) -> Option<String> {
    for line in body.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("File1=") {
            let url = rest.trim();
            if !url.is_empty() {
                return Some(url.to_string());
            }
        }
    }
    None
}

pub fn attach_listen_key(stream_url: &str, listen_key: &str) -> String {
    if stream_url.contains(listen_key) {
        return stream_url.to_string();
    }
    if stream_url.contains('?') {
        format!("{stream_url}&{listen_key}")
    } else {
        format!("{stream_url}?{listen_key}")
    }
}
