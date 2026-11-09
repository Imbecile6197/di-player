use serde::{Deserialize, Serialize};

/// Premium DI.fm stream qualities (AudioAddict stream_set keys).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum StreamQuality {
    /// AAC ~64 kbps — lowest bandwidth.
    PremiumMedium,
    /// AAC ~128 kbps.
    Premium,
    /// MP3 320 kbps — default premium quality.
    #[default]
    PremiumHigh,
}

impl StreamQuality {
    pub const ALL: [StreamQuality; 3] = [
        StreamQuality::PremiumHigh,
        StreamQuality::Premium,
        StreamQuality::PremiumMedium,
    ];

    pub fn api_key(self) -> &'static str {
        match self {
            Self::PremiumMedium => "premium_medium",
            Self::Premium => "premium",
            Self::PremiumHigh => "premium_high",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::PremiumHigh => "320 kbps MP3",
            Self::Premium => "128 kbps AAC",
            Self::PremiumMedium => "64 kbps AAC",
        }
    }

    pub fn short_label(self) -> &'static str {
        match self {
            Self::PremiumHigh => "320k",
            Self::Premium => "128k",
            Self::PremiumMedium => "64k",
        }
    }

    pub fn from_api_key(key: &str) -> Self {
        match key {
            "premium_medium" => Self::PremiumMedium,
            "premium" => Self::Premium,
            _ => Self::PremiumHigh,
        }
    }

    pub fn index(self) -> u32 {
        match self {
            Self::PremiumHigh => 0,
            Self::Premium => 1,
            Self::PremiumMedium => 2,
        }
    }

    pub fn from_index(index: u32) -> Self {
        match index {
            1 => Self::Premium,
            2 => Self::PremiumMedium,
            _ => Self::PremiumHigh,
        }
    }
}
