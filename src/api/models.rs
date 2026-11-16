use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct AuthResponse {
    pub id: Option<u64>,
    pub email: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub api_key: Option<String>,
    pub listen_key: Option<String>,
    pub activated: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Channel {
    pub id: u64,
    pub key: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub description_short: Option<String>,
    #[serde(default)]
    pub description_long: Option<String>,
    #[serde(default)]
    pub channel_director: Option<String>,
    #[serde(default)]
    pub asset_url: Option<String>,
    #[serde(default)]
    pub images: Option<ChannelImages>,
    #[serde(default)]
    pub channel_filter_ids: Vec<u64>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ChannelImages {
    #[serde(default)]
    pub default: Option<String>,
    #[serde(default)]
    pub square: Option<String>,
    #[serde(default)]
    pub compact: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StreamChannel {
    pub id: u64,
    pub key: String,
    pub name: String,
    pub playlist: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChannelFilter {
    pub id: u64,
    pub name: String,
    #[serde(default)]
    pub channels: Vec<FilterChannelRef>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FilterChannelRef {
    pub id: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CurrentlyPlaying {
    pub channel_id: u64,
    pub channel_key: String,
    pub track: TrackInfo,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TrackInfo {
    pub id: Option<u64>,
    pub display_artist: String,
    pub display_title: String,
    #[serde(default)]
    pub duration: Option<f64>,
}

impl Channel {
    pub fn short_description(&self) -> &str {
        self.description_short
            .as_deref()
            .or(self.description.as_deref())
            .unwrap_or("")
    }

    pub fn image_url(&self, size: u32) -> Option<String> {
        let template = self
            .images
            .as_ref()
            .and_then(|i| i.square.as_ref().or(i.default.as_ref()).or(i.compact.as_ref()))
            .or(self.asset_url.as_ref())?;

        Some(resolve_image_url(template, size))
    }
}

pub fn resolve_image_url(template: &str, size: u32) -> String {
    let url = if template.starts_with("//") {
        format!("https:{template}")
    } else if template.starts_with("http://") || template.starts_with("https://") {
        template.to_string()
    } else {
        format!("https://{template}")
    };

    // The AudioAddict CDN ignores `size=N` and serves the 1400×1400 original;
    // only `size=NxN` (or width+height) returns a thumbnail.
    let query = format!("?size={size}x{size}&quality=80");
    if let Some(start) = url.find("{?") {
        format!("{}{query}", &url[..start])
    } else if url.contains('?') {
        url
    } else {
        format!("{url}{query}")
    }
}

#[cfg(test)]
mod tests {
    use super::resolve_image_url;

    #[test]
    fn image_urls_request_square_thumbnails() {
        let t = "//cdn-images.audioaddict.com/4/0/e/x.png{?size,height,width,quality,pad}";
        assert_eq!(
            resolve_image_url(t, 192),
            "https://cdn-images.audioaddict.com/4/0/e/x.png?size=192x192&quality=80"
        );
        assert_eq!(
            resolve_image_url("//cdn-images.audioaddict.com/b/1/y.png", 96),
            "https://cdn-images.audioaddict.com/b/1/y.png?size=96x96&quality=80"
        );
    }
}
