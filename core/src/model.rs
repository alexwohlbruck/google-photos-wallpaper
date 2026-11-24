//! Plain data types shared with every frontend over FFI.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Record)]
pub struct Photo {
    /// Stable id: Google media item id, or a hash of the local file path.
    pub id: String,
    pub source_id: String,
    /// Absolute path to the image on disk.
    pub path: String,
    pub filename: String,
    pub width: u32,
    pub height: u32,
    /// RFC 3339 timestamp when known (Google items), otherwise None.
    pub created: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Enum)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SourceKind {
    /// A folder on disk, scanned for images.
    LocalFolder { path: String, recursive: bool },
    /// Photos chosen through the Google Photos Picker and cached locally.
    GooglePhotos,
    /// A Google Photos album shared by link. Synced periodically (unofficial).
    SharedAlbum { url: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Record)]
pub struct Source {
    pub id: String,
    pub name: String,
    pub kind: SourceKind,
    pub enabled: bool,
    pub photo_count: u32,
    /// Shared albums: when the last successful sync finished (unix seconds).
    #[serde(default)]
    pub last_synced_unix: Option<i64>,
    /// Shared albums: why the last sync failed, if it did.
    #[serde(default)]
    pub sync_error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, uniffi::Enum)]
#[serde(rename_all = "snake_case")]
pub enum Order {
    Random,
    Sequential,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Record)]
#[serde(default)]
pub struct Filters {
    /// Skip photos narrower than this (0 = no limit).
    pub min_width: u32,
    /// Skip photos shorter than this (0 = no limit).
    pub min_height: u32,
    /// Skip photos whose width/height ratio is below this (0 = no limit).
    /// e.g. 1.0 keeps only landscape and square photos.
    pub min_aspect: f64,
    /// Skip photos whose width/height ratio is above this (0 = no limit).
    pub max_aspect: f64,
    /// Skip rated photos scoring below this, 0...1 (0 = no limit). Unrated photos pass.
    pub min_score: f64,
    /// If not empty, only use photos with at least one of these tags.
    pub only_tags: Vec<String>,
    /// Never use photos with any of these tags.
    pub never_tags: Vec<String>,
}

impl Default for Filters {
    fn default() -> Self {
        Self { min_width: 1280, min_height: 720, min_aspect: 1.0, max_aspect: 0.0, min_score: 0.0, only_tags: vec![], never_tags: vec![] }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Record)]
#[serde(default)]
pub struct Settings {
    /// Minutes between automatic changes. 0 disables the timer.
    pub interval_minutes: u32,
    pub paused: bool,
    pub change_on_launch: bool,
    pub order: Order,
    pub filters: Filters,
    /// Hours between shared album syncs.
    pub sync_hours: u32,
    /// Let the frontend rate photos for wallpaper suitability (on-device models).
    pub auto_rate: bool,
    /// In random order, show higher-rated photos more often.
    pub prefer_high_scores: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            interval_minutes: 60,
            paused: false,
            change_on_launch: false,
            order: Order::Random,
            filters: Filters::default(),
            sync_hours: 6,
            auto_rate: true,
            prefer_high_scores: true,
        }
    }
}

/// A wallpaper-suitability rating produced by a frontend's on-device models.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Record)]
pub struct PhotoScore {
    pub photo_id: String,
    /// 0 (unusable) ... 1 (excellent).
    pub value: f64,
    /// Short human-readable description, e.g. "landscape · Mountain lake at sunset".
    pub label: String,
    /// Rating algorithm version; bump to re-rate everything.
    pub version: u32,
    /// What's in the photo: "people", "landscape", "flowers", "animals"…
    #[serde(default)]
    pub tags: Vec<String>,
    /// A short name for the photo as a wallpaper, e.g. "Bridge at Golden Hour".
    #[serde(default)]
    pub title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct TagCount {
    pub tag: String,
    pub count: u32,
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct Status {
    pub current: Option<Photo>,
    /// Unix seconds of the next scheduled change, if the timer is running.
    pub next_change_unix: Option<i64>,
    /// Photos that pass the filters across enabled sources.
    pub eligible_count: u32,
    pub google_signed_in: bool,
    pub google_configured: bool,
    /// Non-None while a long task (scan, Google import) is running.
    pub activity: Option<String>,
    /// Photos in enabled sources that haven't been rated yet.
    pub unrated_count: u32,
}
