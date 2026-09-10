use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    pub id: String,
    pub username: String,
    pub remote_path: String,
    pub filename: String,
    pub size: u64,
    pub format: String,
    pub bitrate: Option<u32>,
    pub sample_rate: Option<u32>,
    pub bit_depth: Option<u16>,
    pub upload_speed: Option<u64>,
    pub queue_length: Option<u32>,
    pub score: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration: f64,
    pub cover: Option<String>,
    #[serde(default)]
    pub local_path: Option<String>,
    pub formats: Vec<String>,
    pub source_count: usize,
    pub favorite: bool,
    pub sources: Vec<Source>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadItem {
    pub id: String,
    pub track: Track,
    pub status: String,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub speed: u64,
    pub local_path: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub username: String,
    pub password: String,
    pub auto_account: bool,
    pub selector_mode: String,
    pub prefer_lossless: bool,
    pub prefer_flac: bool,
    pub minimum_bitrate: u32,
    pub buffer_seconds: u32,
    pub prefetch_count: u8,
    pub cache_limit_gb: u64,
    pub download_directory: String,
    pub organize_downloads: bool,
    pub bandwidth_limit: u32,
    pub slskd_url: String,
    pub api_key: String,
    pub close_to_tray: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            username: String::new(),
            password: String::new(),
            auto_account: true,
            selector_mode: "balanced".into(),
            prefer_lossless: true,
            prefer_flac: true,
            minimum_bitrate: 192,
            buffer_seconds: 0,
            prefetch_count: 2,
            cache_limit_gb: 10,
            download_directory: String::new(),
            organize_downloads: true,
            bandwidth_limit: 0,
            slskd_url: "http://127.0.0.1:5030".into(),
            api_key: String::new(),
            close_to_tray: true,
        }
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Bootstrap {
    pub onboarded: bool,
    pub tracks: Vec<Track>,
    pub favorites: Vec<Track>,
    pub history: Vec<Track>,
    pub downloads: Vec<DownloadItem>,
    pub queue: Vec<Track>,
    pub settings: Settings,
    pub connection: String,
    pub cache_used: u64,
}
#[derive(Serialize)]
pub struct CommandStatus {
    pub status: String,
}
#[derive(Clone)]
pub struct Paths {
    pub root: std::path::PathBuf,
    pub cache_audio: std::path::PathBuf,
    pub covers: std::path::PathBuf,
    pub downloads: std::path::PathBuf,
    pub runtime: std::path::PathBuf,
    pub logs: std::path::PathBuf,
}
