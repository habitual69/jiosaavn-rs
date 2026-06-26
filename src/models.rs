use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Track {
    pub id: String,
    pub title: String,
    pub artists: String,
    pub album: String,
    pub year: String,
    pub duration: u64,
    pub encrypted_media_url: String,
    pub image_url: String,
    pub has_lyrics: bool,
    pub music: String,
    pub copyright_text: String,
    pub language: String,
    pub explicit: bool,
    pub release_date: String,
    pub perma_url: String,
    pub media_preview_url: String,
}

impl Track {
    pub fn is_available(&self) -> bool {
        !self.media_preview_url.is_empty() || !self.encrypted_media_url.is_empty()
    }

    pub fn high_res_image_url(&self) -> String {
        self.image_url
            .replace("150x150", "500x500")
            .replace("50x50", "500x500")
    }
}

#[derive(Debug, Clone)]
pub struct Album {
    pub id: String,
    pub title: String,
    pub artists: String,
    pub year: String,
    pub tracks: Vec<Track>,
    pub image_url: String,
}

impl Album {
    pub fn track_count(&self) -> usize {
        self.tracks.len()
    }

    pub fn high_res_image_url(&self) -> String {
        self.image_url
            .replace("150x150", "500x500")
            .replace("50x50", "500x500")
    }
}

#[derive(Debug, Clone)]
pub struct Playlist {
    pub name: String,
    pub tracks: Vec<Track>,
    pub image_url: String,
}

impl Playlist {
    pub fn track_count(&self) -> usize {
        self.tracks.len()
    }
}

#[derive(Debug)]
pub enum DownloadStatus {
    Completed,
    Failed(String),
}

#[derive(Debug)]
pub struct DownloadResult {
    pub track: Track,
    pub status: DownloadStatus,
    pub file_path: Option<PathBuf>,
}

impl DownloadResult {
    pub fn success(&self) -> bool {
        matches!(self.status, DownloadStatus::Completed)
    }
}
