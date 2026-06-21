use anyhow::{anyhow, Result};
use once_cell::sync::Lazy;
use regex::Regex;
use reqwest::Client;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Semaphore;

use crate::models::{Album, Playlist, Track};

static ALBUM_SONG_RX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"https://www\.jiosaavn\.com/(album|song)/.+?/([^/?#]+)").unwrap()
});
static PLAYLIST_RX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"https://www\.jiosaavn\.com/s/playlist/.+/([^/?#]+)").unwrap()
});
static FEATURED_RX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"https://www\.jiosaavn\.com/featured/.+?/([^/?#]+)").unwrap()
});
static JSON_RX: Lazy<Regex> = Lazy::new(|| Regex::new(r"(\{.+\})").unwrap());

const BASE_URL: &str = "https://www.jiosaavn.com/api.php";
const BITRATE: &str = "320";

pub struct JioSaavnClient {
    client: Client,
    semaphore: Arc<Semaphore>,
}

impl JioSaavnClient {
    pub fn new() -> Result<Self> {
        let client = Client::builder()
            .use_rustls_tls()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .build()?;

        Ok(Self {
            client,
            semaphore: Arc::new(Semaphore::new(10)),
        })
    }

    async fn request(&self, url: &str, params: &[(&str, &str)]) -> Result<Value> {
        let _permit = self.semaphore.acquire().await?;
        let mut delay = std::time::Duration::from_secs(1);

        for attempt in 0..3u32 {
            let resp = self.client.get(url).query(params).send().await;

            match resp {
                Ok(r) => {
                    let text = r.text().await?;

                    if let Ok(v) = serde_json::from_str::<Value>(&text) {
                        return Ok(v);
                    }
                    // JioSaavn sometimes wraps JSON in extra text
                    if let Some(m) = JSON_RX.find(&text) {
                        if let Ok(v) = serde_json::from_str::<Value>(m.as_str()) {
                            return Ok(v);
                        }
                    }
                    return Err(anyhow!("Could not parse API response"));
                }
                Err(e) if attempt < 2 && (e.is_timeout() || e.is_connect()) => {
                    tokio::time::sleep(delay).await;
                    delay *= 2;
                }
                Err(e) => return Err(e.into()),
            }
        }

        Err(anyhow!("All retry attempts failed"))
    }

    pub async fn get_song(&self, song_id: &str) -> Result<Track> {
        let url = format!(
            "{}?__call=webapi.get&token={}&type=song",
            BASE_URL, song_id
        );
        let data = self.request(&url, &[]).await?;

        // API returns { song_id: { song_data } } — grab the first value
        if let Value::Object(map) = &data {
            if let Some(inner) = map.values().next() {
                if inner.is_object() {
                    return Ok(self.parse_track(inner));
                }
            }
        }
        Ok(self.parse_track(&data))
    }

    pub async fn get_album(&self, album_id: &str) -> Result<Album> {
        let url = format!(
            "{}?__call=webapi.get&token={}&type=album",
            BASE_URL, album_id
        );
        let data = self.request(&url, &[]).await?;

        let tracks: Vec<Track> = data["songs"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .map(|s| self.parse_track(s))
            .collect();

        Ok(Album {
            id: album_id.to_string(),
            title: unescape_html(data["title"].as_str().unwrap_or("")),
            artists: data["primary_artists"].as_str().unwrap_or("").to_string(),
            year: value_to_string(&data["year"]),
            image_url: data["image"].as_str().unwrap_or("").to_string(),
            tracks,
        })
    }

    pub async fn get_playlist(&self, playlist_id: &str) -> Result<Playlist> {
        let mut all_tracks: Vec<Track> = Vec::new();
        let mut page = 1u32;
        let per_page = 100u32;
        let mut playlist_name = String::new();
        let mut playlist_image = String::new();
        let mut total_count = 0usize;

        loop {
            let url = format!(
                "{}?__call=webapi.get&token={}&type=playlist&_format=json&n={}&p={}",
                BASE_URL, playlist_id, per_page, page
            );
            let data = self.request(&url, &[]).await?;

            if page == 1 {
                playlist_name = data["listname"].as_str().unwrap_or("").to_string();
                playlist_image = data["image"].as_str().unwrap_or("").to_string();
                total_count = data["list_count"]
                    .as_str()
                    .and_then(|s| s.parse().ok())
                    .or_else(|| data["list_count"].as_u64().map(|n| n as usize))
                    .unwrap_or(0);
            }

            let songs = match data["songs"].as_array() {
                Some(s) if !s.is_empty() => s.clone(),
                _ => break,
            };

            let fetched = songs.len();
            all_tracks.extend(songs.iter().map(|s| self.parse_track(s)));

            if all_tracks.len() >= total_count || fetched < per_page as usize {
                break;
            }
            page += 1;
        }

        Ok(Playlist {
            id: playlist_id.to_string(),
            name: playlist_name,
            tracks: all_tracks,
            image_url: playlist_image,
        })
    }

    pub async fn get_cdn_url(&self, encrypted_url: &str) -> Result<String> {
        let params = [
            ("__call", "song.generateAuthToken"),
            ("url", encrypted_url),
            ("bitrate", BITRATE),
            ("api_version", "4"),
            ("_format", "json"),
            ("ctx", "web6dot0"),
            ("_marker", "0"),
        ];
        let data = self.request(BASE_URL, &params).await?;
        data["auth_url"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow!("No auth_url in CDN response"))
    }

    pub async fn get_lyrics(&self, song_id: &str) -> Option<String> {
        let url = format!(
            "{}?__call=lyrics.getLyrics&ctx=web6dot0&api_version=4&_format=json&_marker=0&lyrics_id={}",
            BASE_URL, song_id
        );
        let data = self.request(&url, &[]).await.ok()?;
        let text = data["lyrics"].as_str().filter(|s| !s.is_empty())?;
        Some(text.replace("<br>", "\n"))
    }

    pub async fn download_bytes(&self, url: &str) -> Result<Vec<u8>> {
        let _permit = self.semaphore.acquire().await?;
        let resp = self.client.get(url).send().await?;
        Ok(resp.bytes().await?.to_vec())
    }

    fn parse_track(&self, data: &Value) -> Track {
        let perma_url = data["perma_url"].as_str().unwrap_or("").to_string();

        // perma_url is the most reliable source for the token used by the song API
        let id = ALBUM_SONG_RX
            .captures(&perma_url)
            .map(|c| c[2].to_string())
            .unwrap_or_else(|| {
                data["id"]
                    .as_str()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| value_to_string(&data["id"]))
            });

        let title = unescape_html(
            data["song"]
                .as_str()
                .or_else(|| data["title"].as_str())
                .unwrap_or(""),
        );

        let has_lyrics = data["has_lyrics"].as_str().map(|s| s == "true").unwrap_or(false);

        let explicit = data["explicit_content"]
            .as_str()
            .map(|s| s != "0")
            .or_else(|| data["explicit_content"].as_u64().map(|n| n != 0))
            .unwrap_or(false);

        let duration = data["duration"]
            .as_str()
            .and_then(|s| s.parse().ok())
            .or_else(|| data["duration"].as_u64())
            .unwrap_or(0);

        Track {
            id,
            title,
            artists: data["primary_artists"].as_str().unwrap_or("").to_string(),
            album: unescape_html(data["album"].as_str().unwrap_or("")),
            year: value_to_string(&data["year"]),
            duration,
            encrypted_media_url: data["encrypted_media_url"]
                .as_str()
                .unwrap_or("")
                .to_string(),
            image_url: data["image"].as_str().unwrap_or("").to_string(),
            has_lyrics,
            music: data["music"].as_str().unwrap_or("").to_string(),
            label: data["label"].as_str().unwrap_or("").to_string(),
            copyright_text: data["copyright_text"].as_str().unwrap_or("").to_string(),
            language: data["language"].as_str().unwrap_or("").to_string(),
            explicit,
            featured_artists: data["featured_artists"].as_str().unwrap_or("").to_string(),
            singers: data["singers"].as_str().unwrap_or("").to_string(),
            starring: data["starring"].as_str().unwrap_or("").to_string(),
            release_date: data["release_date"].as_str().unwrap_or("").to_string(),
            perma_url,
            media_preview_url: data["media_preview_url"].as_str().unwrap_or("").to_string(),
        }
    }

    /// Parse a JioSaavn URL and return (url_type, token_id).
    pub fn parse_url(url: &str) -> Result<(String, String)> {
        if url.contains("/album/") || url.contains("/song/") {
            if let Some(c) = ALBUM_SONG_RX.captures(url) {
                return Ok((c[1].to_string(), c[2].to_string()));
            }
        } else if url.contains("/s/playlist/") || url.contains("/playlist/") {
            if let Some(c) = PLAYLIST_RX.captures(url) {
                return Ok(("playlist".to_string(), c[1].to_string()));
            }
        } else if url.contains("/featured/") {
            if let Some(c) = FEATURED_RX.captures(url) {
                return Ok(("playlist".to_string(), c[1].to_string()));
            }
        }
        Err(anyhow!("Invalid or unsupported JioSaavn URL: {}", url))
    }
}

// --- helpers ----------------------------------------------------------------

pub fn unescape_html(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#039;", "'")
        .replace("&apos;", "'")
        .replace("&#x27;", "'")
}

fn value_to_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        _ => String::new(),
    }
}
