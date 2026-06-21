use anyhow::Result;
use futures::StreamExt;
use indicatif::ProgressBar;
use reqwest::Client;
use std::{path::PathBuf, sync::Arc};
use tokio::{fs::File, io::AsyncWriteExt, sync::Semaphore};

use crate::{
    api::JioSaavnClient,
    config::Config,
    converter::{convert_to_mp3, is_ffmpeg_available},
    metadata::{
        generate_album_folder, generate_filename, tag_m4a, tag_mp3,
    },
    models::{Album, DownloadResult, DownloadStatus, Playlist, Track},
    ui::DownloadUI,
};

pub struct DownloadManager {
    pub client: Arc<JioSaavnClient>,
    pub config: Config,
    semaphore: Arc<Semaphore>,
    http: Client,
}

impl DownloadManager {
    pub fn new(client: Arc<JioSaavnClient>, config: Config) -> Result<Self> {
        let http = Client::builder()
            .use_rustls_tls()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .default_headers({
                let mut h = reqwest::header::HeaderMap::new();
                h.insert("Referer", "https://www.jiosaavn.com/".parse().unwrap());
                h.insert("Origin", "https://www.jiosaavn.com".parse().unwrap());
                h
            })
            .timeout(std::time::Duration::from_secs(config.timeout_secs + 90))
            .build()?;

        let max = config.max_concurrent_downloads;
        Ok(Self {
            client,
            config,
            semaphore: Arc::new(Semaphore::new(max)),
            http,
        })
    }

    // ─── Public entry points ─────────────────────────────────────────────────

    pub async fn download_song(&self, song_id: &str, ui: &DownloadUI) -> Result<DownloadResult> {
        ui.info("Fetching song info…");
        let track = self.client.get_song(song_id).await?;
        ui.display_track_info(&track);

        let folder = format!("{} - {} [{}]", track.artists, track.album, track.year);
        let output_dir = self.config.download_path().join(sanitize_filename::sanitize(&folder));
        tokio::fs::create_dir_all(&output_dir).await?;

        let overall = ui.add_overall_bar(1);
        let pb = ui.add_track_bar(&track, 0);
        let result = self
            .download_one(track, &output_dir, 1, 1, None, None, &pb)
            .await;
        pb.finish_and_clear();
        overall.inc(1);
        overall.finish_and_clear();
        Ok(result)
    }

    pub async fn download_album(&self, album: Album, ui: &DownloadUI) -> Vec<DownloadResult> {
        ui.display_album_info(&album);

        let folder = generate_album_folder(&album.artists, &album.title, &album.year);
        let output_dir = self.config.download_path().join(folder);
        let _ = tokio::fs::create_dir_all(&output_dir).await;

        // Pre-fetch album art once for all tracks
        let album_art: Option<Vec<u8>> = self
            .client
            .download_bytes(&album.high_res_image_url())
            .await
            .ok();

        // Save standalone cover.jpg
        if let Some(art) = &album_art {
            let _ = tokio::fs::write(output_dir.join("cover.jpg"), art).await;
        }

        let total = album.track_count();
        let overall = ui.add_overall_bar(total as u64);
        let artists = album.artists.clone();

        let tasks: Vec<_> = album
            .tracks
            .into_iter()
            .enumerate()
            .map(|(i, track)| {
                let client = Arc::clone(&self.client);
                let config = self.config.clone();
                let http = self.http.clone();
                let sem = Arc::clone(&self.semaphore);
                let out = output_dir.clone();
                let art = album_art.clone();
                let album_artist = artists.clone();
                let pb = ui.add_track_bar(&track, 0);
                let overall = overall.clone();

                tokio::spawn(async move {
                    let mgr = DownloadManager {
                        client,
                        config,
                        semaphore: sem,
                        http,
                    };
                    let _permit = mgr.semaphore.acquire().await.unwrap();
                    let result = mgr
                        .download_one(track, &out, i + 1, total, Some(&album_artist), art.as_deref(), &pb)
                        .await;
                    pb.finish_and_clear();
                    overall.inc(1);
                    result
                })
            })
            .collect();

        let results: Vec<DownloadResult> = futures::future::join_all(tasks)
            .await
            .into_iter()
            .filter_map(|r| r.ok())
            .collect();

        overall.finish_and_clear();
        results
    }

    pub async fn download_playlist(&self, playlist: Playlist, ui: &DownloadUI) -> Vec<DownloadResult> {
        ui.display_playlist_info(&playlist);

        let folder = format!("Playlist - {}", sanitize_filename::sanitize(&playlist.name));
        let output_dir = self.config.download_path().join(folder);
        let _ = tokio::fs::create_dir_all(&output_dir).await;

        // Save playlist cover (not passed to individual tracks — each fetches its own album art)
        if !playlist.image_url.is_empty() {
            let cover_url = playlist
                .image_url
                .replace("150x150", "500x500")
                .replace("50x50", "500x500");
            if let Ok(art) = self.client.download_bytes(&cover_url).await {
                let _ = tokio::fs::write(output_dir.join("cover.jpg"), art).await;
            }
        }

        let total = playlist.track_count();
        let overall = ui.add_overall_bar(total as u64);

        let tasks: Vec<_> = playlist
            .tracks
            .into_iter()
            .enumerate()
            .map(|(i, track)| {
                let client = Arc::clone(&self.client);
                let config = self.config.clone();
                let http = self.http.clone();
                let sem = Arc::clone(&self.semaphore);
                let out = output_dir.clone();
                let pb = ui.add_track_bar(&track, 0);
                let overall = overall.clone();

                tokio::spawn(async move {
                    let mgr = DownloadManager {
                        client,
                        config,
                        semaphore: sem,
                        http,
                    };
                    let _permit = mgr.semaphore.acquire().await.unwrap();
                    // album_art = None → each track fetches its own cover
                    let result = mgr
                        .download_one(track, &out, i + 1, total, None, None, &pb)
                        .await;
                    pb.finish_and_clear();
                    overall.inc(1);
                    result
                })
            })
            .collect();

        let results: Vec<DownloadResult> = futures::future::join_all(tasks)
            .await
            .into_iter()
            .filter_map(|r| r.ok())
            .collect();

        overall.finish_and_clear();
        results
    }

    // ─── Core single-track download ───────────────────────────────────────────

    async fn download_one(
        &self,
        mut track: Track,
        output_dir: &PathBuf,
        position: usize,
        total: usize,
        album_artist: Option<&str>,
        album_art: Option<&[u8]>,
        pb: &ProgressBar,
    ) -> DownloadResult {
        match self
            .try_download(&mut track, output_dir, position, total, album_artist, album_art, pb)
            .await
        {
            Ok((file_path, bytes)) => DownloadResult {
                track,
                status: DownloadStatus::Completed,
                file_path: Some(file_path),
                bytes_downloaded: bytes,
            },
            Err(e) => {
                pb.set_message(format!("✗ {}", track.title));
                DownloadResult {
                    track,
                    status: DownloadStatus::Failed(e.to_string()),
                    file_path: None,
                    bytes_downloaded: 0,
                }
            }
        }
    }

    async fn try_download(
        &self,
        track: &mut Track,
        output_dir: &PathBuf,
        position: usize,
        total: usize,
        album_artist: Option<&str>,
        mut album_art: Option<&[u8]>,
        pb: &ProgressBar,
    ) -> Result<(PathBuf, u64)> {
        // Playlist tracks often lack encrypted_media_url — fetch full track data
        if track.encrypted_media_url.is_empty() && !track.id.is_empty() {
            if let Ok(full) = self.client.get_song(&track.id).await {
                *track = full;
            }
        }

        let m4a_path = output_dir.join(generate_filename(track, position, ".m4a"));

        // Skip if already downloaded (check both m4a and mp3)
        let mp3_path = m4a_path.with_extension("mp3");
        if m4a_path.exists() || mp3_path.exists() {
            return Ok((if mp3_path.exists() { mp3_path } else { m4a_path }, 0));
        }

        if !track.is_available() {
            return Err(anyhow::anyhow!("Track unavailable in your region"));
        }

        let cdn_url = self.client.get_cdn_url(&track.encrypted_media_url).await?;
        if cdn_url.is_empty() {
            return Err(anyhow::anyhow!("Empty CDN URL"));
        }

        let bytes_downloaded = self.stream_to_file(&cdn_url, &m4a_path, pb).await?;

        // Fetch per-track album art if not provided (playlist downloads)
        let owned_art: Option<Vec<u8>>;
        if album_art.is_none() {
            owned_art = self.client.download_bytes(&track.high_res_image_url()).await.ok();
            album_art = owned_art.as_deref();
        }

        // Lyrics
        let lyrics_owned: Option<String>;
        if track.has_lyrics {
            lyrics_owned = self.client.get_lyrics(&track.id).await;
        } else {
            lyrics_owned = None;
        }
        let lyrics = lyrics_owned.as_deref();

        // Tag the M4A file
        tag_m4a(&m4a_path, track, album_art, lyrics, position, total, album_artist)?;

        // Convert to MP3 if configured
        let final_path = if self.config.output_format.to_lowercase() == "mp3" {
            if is_ffmpeg_available() {
                let mp3 = convert_to_mp3(&m4a_path, &self.config.mp3_quality).await?;
                tag_mp3(&mp3, track, album_art, lyrics, position, total)?;
                mp3
            } else {
                eprintln!(
                    "\nWarning: FFmpeg not found — saving as M4A. \
                     Install FFmpeg or pass -f m4a to suppress this warning."
                );
                m4a_path
            }
        } else {
            m4a_path
        };

        Ok((final_path, bytes_downloaded))
    }

    // ─── Streaming file download ──────────────────────────────────────────────

    async fn stream_to_file(&self, url: &str, path: &PathBuf, pb: &ProgressBar) -> Result<u64> {
        let mut delay = std::time::Duration::from_secs(1);

        for attempt in 0..3u32 {
            match self.do_stream(url, path, pb).await {
                Ok(n) => return Ok(n),
                Err(e) if attempt < 2 => {
                    // Remove partial file before retry
                    let _ = tokio::fs::remove_file(path).await;
                    tokio::time::sleep(delay).await;
                    delay *= 2;
                    pb.set_message(format!("Retry {attempt}…"));
                    let _ = e;
                }
                Err(e) => return Err(e),
            }
        }
        Err(anyhow::anyhow!("Download failed after 3 attempts"))
    }

    async fn do_stream(&self, url: &str, path: &PathBuf, pb: &ProgressBar) -> Result<u64> {
        let resp = self.http.get(url).send().await?;
        resp.error_for_status_ref()?;

        let total = resp.content_length().unwrap_or(0);
        if total > 0 {
            pb.set_length(total);
        }

        let mut file = File::create(path).await?;
        let mut stream = resp.bytes_stream();
        let mut downloaded: u64 = 0;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            file.write_all(&chunk).await?;
            downloaded += chunk.len() as u64;
            pb.set_position(downloaded);
        }

        file.flush().await?;
        Ok(downloaded)
    }
}
