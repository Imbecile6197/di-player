use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum UiTheme {
    #[default]
    Neon,
    SoftNight,
    Mono,
    Minimal,
}

impl UiTheme {
    pub const ALL: [UiTheme; 4] = [
        UiTheme::Neon,
        UiTheme::SoftNight,
        UiTheme::Mono,
        UiTheme::Minimal,
    ];

    pub fn css_class(self) -> &'static str {
        match self {
            Self::Neon => "theme-neon",
            Self::SoftNight => "theme-soft-night",
            Self::Mono => "theme-mono",
            Self::Minimal => "theme-minimal",
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            Self::Neon => "neon",
            Self::SoftNight => "soft_night",
            Self::Mono => "mono",
            Self::Minimal => "minimal",
        }
    }

    pub fn from_id(id: &str) -> Self {
        match id {
            "soft_night" => Self::SoftNight,
            "mono" => Self::Mono,
            "minimal" => Self::Minimal,
            _ => Self::Neon,
        }
    }

    pub fn index(self) -> u32 {
        match self {
            Self::Neon => 0,
            Self::SoftNight => 1,
            Self::Mono => 2,
            Self::Minimal => 3,
        }
    }

    pub fn from_index(index: u32) -> Self {
        match index {
            1 => Self::SoftNight,
            2 => Self::Mono,
            3 => Self::Minimal,
            _ => Self::Neon,
        }
    }

    pub fn label_key(self) -> &'static str {
        match self {
            Self::Neon => "theme-neon",
            Self::SoftNight => "theme-soft-night",
            Self::Mono => "theme-mono",
            Self::Minimal => "theme-minimal",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ChannelViewMode {
    #[default]
    List,
    Grid,
}

impl ChannelViewMode {
    pub fn index(self) -> u32 {
        match self {
            Self::List => 0,
            Self::Grid => 1,
        }
    }

    pub fn from_index(index: u32) -> Self {
        if index == 1 {
            Self::Grid
        } else {
            Self::List
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            Self::List => "list",
            Self::Grid => "grid",
        }
    }

    pub fn from_id(id: &str) -> Self {
        if id == "grid" {
            Self::Grid
        } else {
            Self::List
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AppLocale {
    #[default]
    System,
    English,
    Czech,
    German,
}

impl AppLocale {
    pub const ALL: [AppLocale; 4] = [
        AppLocale::System,
        AppLocale::English,
        AppLocale::Czech,
        AppLocale::German,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::English => "en",
            Self::Czech => "cs",
            Self::German => "de",
        }
    }

    pub fn from_id(id: &str) -> Self {
        match id {
            "en" => Self::English,
            "cs" => Self::Czech,
            "de" => Self::German,
            _ => Self::System,
        }
    }

    pub fn index(self) -> u32 {
        match self {
            Self::System => 0,
            Self::English => 1,
            Self::Czech => 2,
            Self::German => 3,
        }
    }

    pub fn from_index(index: u32) -> Self {
        match index {
            1 => Self::English,
            2 => Self::Czech,
            3 => Self::German,
            _ => Self::System,
        }
    }

    pub fn label_key(self) -> &'static str {
        match self {
            Self::System => "locale-system",
            Self::English => "locale-en",
            Self::Czech => "locale-cs",
            Self::German => "locale-de",
        }
    }

    /// Resolved BCP-47 language tag used by Fluent.
    pub fn resolve(self) -> &'static str {
        match self {
            Self::English => "en",
            Self::Czech => "cs",
            Self::German => "de",
            Self::System => system_lang(),
        }
    }
}

fn system_lang() -> &'static str {
    let lang = std::env::var("LANG")
        .or_else(|_| std::env::var("LC_ALL"))
        .or_else(|_| std::env::var("LC_MESSAGES"))
        .unwrap_or_default()
        .to_lowercase();
    if lang.starts_with("cs") {
        "cs"
    } else if lang.starts_with("de") {
        "de"
    } else {
        "en"
    }
}
