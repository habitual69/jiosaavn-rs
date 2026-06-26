mod api;
mod config;
mod converter;
mod downloader;
mod metadata;
mod models;
mod ui;

use anyhow::Result;
use clap::{Parser, ValueEnum};
use console::style;
use std::io::{BufRead, Write};
use std::sync::Arc;

use api::JioSaavnClient;
use config::Config;
use downloader::DownloadManager;
use models::DownloadResult;
use ui::DownloadUI;

// ─── CLI definition ───────────────────────────────────────────────────────────

#[derive(Clone, Debug, ValueEnum)]
enum OutputFormat {
    Mp3,
    M4a,
}

#[derive(Clone, Debug, ValueEnum)]
enum Quality {
    #[value(name = "320k")] Q320k,
    #[value(name = "256k")] Q256k,
    #[value(name = "192k")] Q192k,
    #[value(name = "128k")] Q128k,
}

impl Quality {
    fn as_str(&self) -> &'static str {
        match self {
            Quality::Q320k => "320k",
            Quality::Q256k => "256k",
            Quality::Q192k => "192k",
            Quality::Q128k => "128k",
        }
    }
}

#[derive(Parser, Debug)]
#[command(
    name        = "jiosaavn",
    version     = "1.0.0",
    author      = "habitual69",
    about       = "High-performance JioSaavn music downloader\nSupports songs, albums, playlists, and featured collections",
    long_about  = None,
)]
struct Cli {
    /// JioSaavn URL — song, album, playlist, featured, or artist
    #[arg(value_name = "URL")]
    url: Option<String>,

    /// Text file with one JioSaavn URL per line (bulk download)
    #[arg(short = 'F', long, value_name = "FILE")]
    file: Option<String>,

    /// Maximum concurrent downloads
    #[arg(short = 'c', long, default_value_t = 5, value_name = "N")]
    concurrent: usize,

    /// Output directory
    #[arg(short = 'o', long, default_value = "Downloads", value_name = "DIR")]
    output: String,

    /// Output format
    #[arg(short = 'f', long, value_enum, value_name = "FORMAT")]
    format: Option<OutputFormat>,

    /// MP3 quality (only used when format is mp3)
    #[arg(short = 'q', long, value_enum, value_name = "QUALITY")]
    quality: Option<Quality>,

    /// Save current settings to config file for future runs
    #[arg(long)]
    save_config: bool,

    /// Print current configuration and exit
    #[arg(long)]
    show_config: bool,

    /// Search JioSaavn interactively — shows paginated table, enter # to download
    #[arg(long, value_name = "QUERY")]
    search: Option<String>,

    /// Download by JioSaavn token ID (e.g. BAAJWBFRBk) — use with --type
    #[arg(long, value_name = "TOKEN")]
    id: Option<String>,

    /// Content type for --id (song, album, playlist, featured, artist) [default: song]
    #[arg(long, value_name = "TYPE", default_value = "song")]
    r#type: String,

    /// Show what would be downloaded without actually downloading
    #[arg(long)]
    dry_run: bool,

    /// Re-download even if file already exists
    #[arg(long)]
    no_skip: bool,

    /// Apply a quality/concurrency preset (hifi, mobile, podcast)
    #[arg(long, value_name = "PROFILE")]
    profile: Option<String>,

    /// Disable M3U playlist generation for albums and playlists
    #[arg(long)]
    no_m3u: bool,

    /// Save lyrics as .lrc sidecar files
    #[arg(long)]
    save_lyrics: bool,

    /// Number of URLs to process in parallel during bulk download (default: 1)
    #[arg(long, default_value_t = 1, value_name = "N")]
    parallel_urls: usize,

    /// Append download history to <download_dir>/history.jsonl
    #[arg(long)]
    log: bool,

    /// Check for a newer release on GitHub and exit
    #[arg(long)]
    check_update: bool,

    /// Skip the automatic update check at startup
    #[arg(long)]
    no_update_check: bool,

    /// HTTP/HTTPS proxy URL (e.g. http://user:pass@host:port)
    #[arg(long, value_name = "URL")]
    proxy: Option<String>,

    /// Nest album downloads under Albums/<year>/ subfolders
    #[arg(long)]
    year_folders: bool,
}

// ─── Entry point ─────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut config = Config::load();

    if cli.show_config {
        config.print();
        return Ok(());
    }

    // Apply profile first (CLI overrides take precedence after)
    if let Some(ref profile) = cli.profile {
        config.apply_profile(profile);
    }

    // Apply CLI overrides
    if let Some(f) = &cli.format {
        config.output_format = match f {
            OutputFormat::Mp3 => "mp3",
            OutputFormat::M4a => "m4a",
        }
        .to_string();
    }
    if let Some(q) = &cli.quality {
        config.mp3_quality = q.as_str().to_string();
    }
    if cli.output != "Downloads" {
        config.download_dir = cli.output.clone();
    }
    if cli.concurrent != 5 {
        config.max_concurrent_downloads = cli.concurrent;
    }

    // Runtime-only flags
    config.dry_run = cli.dry_run;
    config.no_skip = cli.no_skip;
    config.generate_m3u = !cli.no_m3u;
    config.save_lyrics = cli.save_lyrics;
    config.log_downloads = cli.log;
    config.year_folders = cli.year_folders;

    if cli.save_config {
        config.save();
    }

    tokio::fs::create_dir_all(config.download_path()).await?;

    let ui = DownloadUI::new();
    ui.display_logo();

    // Update check (runs unless --no-update-check)
    if !cli.no_update_check {
        check_for_update(&ui).await;
    }

    if cli.save_config {
        ui.info(&format!(
            "Settings saved — format={}, quality={}, concurrent={}",
            config.output_format, config.mp3_quality, config.max_concurrent_downloads
        ));
    }

    // --check-update: already ran above; exit
    if cli.check_update {
        return Ok(());
    }

    let proxy_ref = cli.proxy.as_deref();
    let client = Arc::new(JioSaavnClient::new_with_proxy(proxy_ref)?);

    // ── Download by ID ─────────────────────────────────────────────────────
    if let Some(token) = &cli.id {
        let url = id_to_url(token, &cli.r#type);
        let results = match process_url(&url, &config, &client, &ui, proxy_ref).await {
            Ok(r) => r,
            Err(e) => { ui.error(&e.to_string()); std::process::exit(1); }
        };
        if config.log_downloads { log_history(&config, &url, &results).await; }
        ui.show_summary(&results);
        ui.success("Done!");
        return Ok(());
    }

    // ── Interactive search ─────────────────────────────────────────────────
    if let Some(query) = &cli.search {
        ui.info(&format!("Searching: {}", query));

        let (songs_res, albums_res) = tokio::join!(
            client.search_songs(query),
            client.search_albums(query)
        );

        // Build a flat entry list: songs first, then albums
        let mut entries: Vec<SearchEntry> = Vec::new();
        for s in songs_res.unwrap_or_default().into_iter().take(20) {
            let token = last_path_segment(&s.perma_url).unwrap_or(s.id.clone());
            entries.push(SearchEntry {
                kind:    "Song".into(),
                title:   s.title,
                artists: s.artists,
                year:    s.year,
                token:   token,
                url:     s.perma_url,
            });
        }
        for a in albums_res.unwrap_or_default().into_iter().take(10) {
            let url = format!("https://www.jiosaavn.com/album/x/{}", a.id);
            entries.push(SearchEntry {
                kind:    "Album".into(),
                title:   a.title,
                artists: a.artists,
                year:    a.year,
                token:   a.id.clone(),
                url,
            });
        }

        if entries.is_empty() {
            ui.error("No results found.");
            return Ok(());
        }

        // Restore terminal to normal (cooked) mode before interactive input.
        // MultiProgress leaves the terminal in raw mode; search needs line editing.
        ui.finalize();

        const PAGE: usize = 10;
        let total_pages = entries.len().div_ceil(PAGE);
        let mut page = 0usize;

        let selected_url = loop {
            let start = page * PAGE;
            let end   = (start + PAGE).min(entries.len());
            let slice = &entries[start..end];

            // ── table header ──────────────────────────────────────────────
            println!();
            println!("{}", style(format!(
                "─── Search Results ─ Page {}/{} ─ {} total ─────────────────────────────",
                page + 1, total_pages, entries.len()
            )).cyan().bold());
            println!(
                "  {:<4} {:<6} {:<36} {:<26} {:<6} {}",
                style("#").bold(),
                style("Type").bold(),
                style("Title").bold(),
                style("Artist").bold(),
                style("Year").bold(),
                style("Token / ID").bold(),
            );
            println!("{}", style("─".repeat(90)).dim());

            for (i, e) in slice.iter().enumerate() {
                println!(
                    "  {:<4} {:<6} {:<36} {:<26} {:<6} {}",
                    style(start + i + 1).cyan().bold(),
                    e.kind,
                    trunc(&e.title, 34),
                    trunc(&e.artists, 24),
                    e.year,
                    style(&e.token).dim(),
                );
            }
            println!("{}", style("─".repeat(90)).dim());

            // ── navigation prompt ─────────────────────────────────────────
            let mut hints = String::new();
            if total_pages > 1 {
                if page + 1 < total_pages { hints.push_str("  [N] Next page"); }
                if page > 0               { hints.push_str("  [P] Prev page"); }
            }
            hints.push_str("  [Q] Quit");
            println!("{}", style(&hints).dim());
            print!("  Enter # to download: ");
            std::io::stdout().flush().ok();

            let mut raw = String::new();
            std::io::stdin().lock().read_line(&mut raw).ok();
            let input = raw.trim().to_ascii_lowercase();

            match input.as_str() {
                "n" if page + 1 < total_pages => { page += 1; }
                "p" if page > 0               => { page -= 1; }
                "q"                           => { ui.info("Cancelled."); return Ok(()); }
                s => {
                    if let Ok(n) = s.parse::<usize>() {
                        if n >= 1 && n <= entries.len() {
                            break entries[n - 1].url.clone();
                        }
                    }
                    ui.warn(&format!("'{}' is not a valid choice.", s));
                }
            }
        };

        let results = match process_url(&selected_url, &config, &client, &ui, proxy_ref).await {
            Ok(r) => r,
            Err(e) => { ui.error(&e.to_string()); std::process::exit(1); }
        };
        if config.log_downloads { log_history(&config, &selected_url, &results).await; }
        ui.show_summary(&results);
        ui.success("Done!");
        return Ok(());
    }

    if let Some(file_path) = &cli.file {
        // ── Bulk download from text file ──────────────────────────────────────
        let raw = tokio::fs::read_to_string(file_path).await.map_err(|e| {
            anyhow::anyhow!("Cannot read file '{}': {}", file_path, e)
        })?;

        let urls: Vec<String> = raw
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .collect();

        if urls.is_empty() {
            ui.error("File contains no valid URLs.");
            std::process::exit(1);
        }

        ui.info(&format!("Bulk download: {} URL(s) from '{}'", urls.len(), file_path));
        let mut all_results: Vec<DownloadResult> = Vec::new();

        let parallel = cli.parallel_urls.max(1);
        if parallel > 1 {
            use futures::stream::{self, StreamExt};
            let config_arc = Arc::new(config.clone());
            let client_arc = Arc::clone(&client);
            let ui_arc = Arc::new(ui.clone());
            let proxy_owned = cli.proxy.clone();

            let results_nested: Vec<_> = stream::iter(urls.iter())
                .map(|url| {
                    let url = url.clone();
                    let cfg = (*config_arc).clone();
                    let cl = Arc::clone(&client_arc);
                    let u = Arc::clone(&ui_arc);
                    let px = proxy_owned.clone();
                    async move {
                        process_url(&url, &cfg, &cl, &u, px.as_deref()).await
                    }
                })
                .buffer_unordered(parallel)
                .collect()
                .await;

            for (url, res) in urls.iter().zip(results_nested.into_iter()) {
                match res {
                    Ok(mut r) => {
                        if config_arc.log_downloads {
                            log_history(&config_arc, url, &r).await;
                        }
                        all_results.append(&mut r);
                    }
                    Err(e) => ui_arc.error(&format!("Skipping '{}': {}", url, e)),
                }
            }
        } else {
            for url in &urls {
                match process_url(url, &config, &client, &ui, proxy_ref).await {
                    Ok(mut results) => {
                        if config.log_downloads {
                            log_history(&config, url, &results).await;
                        }
                        all_results.append(&mut results);
                    }
                    Err(e) => ui.error(&format!("Skipping '{}': {}", url, e)),
                }
            }
        }

        ui.show_summary(&all_results);
    } else {
        // ── Single URL ────────────────────────────────────────────────────────
        let url = match &cli.url {
            Some(u) => u.clone(),
            None => {
                eprintln!(
                    "Error: provide a JioSaavn URL or use -F <file> for bulk download.\n\
                     Run `jiosaavn --help` for usage."
                );
                std::process::exit(1);
            }
        };

        let results = match process_url(&url, &config, &client, &ui, proxy_ref).await {
            Ok(r) => r,
            Err(e) => {
                ui.error(&e.to_string());
                std::process::exit(1);
            }
        };
        if config.log_downloads {
            log_history(&config, &url, &results).await;
        }
        ui.show_summary(&results);
    }

    ui.success("Done!");
    Ok(())
}

// ─── Search helpers ──────────────────────────────────────────────────────────

struct SearchEntry {
    kind:    String,
    title:   String,
    artists: String,
    year:    String,
    token:   String,
    url:     String,
}

/// Extract the last non-empty path segment from a URL (the JioSaavn token).
fn last_path_segment(url: &str) -> Option<String> {
    url.trim_end_matches('/')
        .rsplit('/')
        .next()
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

/// Build a downloadable URL from a bare token + content type.
fn id_to_url(token: &str, kind: &str) -> String {
    match kind {
        "album"    => format!("https://www.jiosaavn.com/album/x/{}", token),
        "playlist" => format!("https://www.jiosaavn.com/s/playlist/x/{}", token),
        "featured" => format!("https://www.jiosaavn.com/featured/x/{}", token),
        "artist"   => format!("https://www.jiosaavn.com/artist/x/{}", token),
        _          => format!("https://www.jiosaavn.com/song/x/{}", token),
    }
}

/// Truncate a string to `max` display characters, appending "…" if needed.
fn trunc(s: &str, max: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() > max {
        format!("{}…", chars[..max - 1].iter().collect::<String>())
    } else {
        s.to_string()
    }
}

// ─── Per-URL dispatcher ───────────────────────────────────────────────────────

async fn process_url(
    url: &str,
    base_config: &Config,
    client: &Arc<JioSaavnClient>,
    ui: &DownloadUI,
    proxy: Option<&str>,
) -> Result<Vec<DownloadResult>> {
    let (url_type, url_id) = match JioSaavnClient::parse_url(url) {
        Ok(pair) => pair,
        Err(e) => {
            return Err(anyhow::anyhow!(
                "{} — supported forms: /song/…  /album/…  /s/playlist/…  /featured/…  /artist/…",
                e
            ));
        }
    };

    let category_subdir = match url_type.as_str() {
        "song"     => "Singles",
        "album"    => "Albums",
        "playlist" => "Playlists",
        "featured" => "Featured",
        "artist"   => "Artists",
        _          => "Downloads",
    };

    let mut config = base_config.clone();
    config.download_dir = format!("{}/{}", base_config.download_dir, category_subdir);
    tokio::fs::create_dir_all(config.download_path()).await?;

    // Dry-run: fetch metadata and print, skip download
    if config.dry_run {
        match url_type.as_str() {
            "song" => {
                let track = client.get_song(&url_id).await?;
                ui.info(&format!("[DRY RUN] Song: {} — {}", track.title, track.artists));
            }
            "album" => {
                let album = client.get_album(&url_id).await?;
                ui.info(&format!(
                    "[DRY RUN] Album: {} — {} ({}) — {} tracks",
                    album.title, album.artists, album.year, album.track_count()
                ));
            }
            "playlist" | "featured" => {
                let playlist = client.get_playlist(&url_id).await?;
                ui.info(&format!(
                    "[DRY RUN] Playlist: {} — {} tracks",
                    playlist.name, playlist.track_count()
                ));
            }
            "artist" => {
                let (name, albums) = client.get_artist_albums(&url_id).await?;
                ui.info(&format!(
                    "[DRY RUN] Artist: {} — {} albums",
                    name, albums.len()
                ));
            }
            _ => {}
        }
        return Ok(vec![]);
    }

    let manager = DownloadManager::new_with_proxy(Arc::clone(client), config.clone(), proxy)?;

    let results = match url_type.as_str() {
        "song" => {
            let result = manager.download_song(&url_id, ui).await?;
            vec![result]
        }
        "album" => {
            ui.info("Fetching album…");
            let album = client.get_album(&url_id).await?;

            // Year-folder nesting: Albums/<year>/<album-folder>/
            let manager = if config.year_folders && !album.year.is_empty() {
                let year = album.year.clone();
                let mut yr_config = config.clone();
                yr_config.download_dir = format!("{}/{}", config.download_dir, year);
                tokio::fs::create_dir_all(yr_config.download_path()).await?;
                DownloadManager::new_with_proxy(Arc::clone(client), yr_config, proxy)?
            } else {
                manager
            };

            manager.download_album(album, ui).await
        }
        "playlist" => {
            ui.info("Fetching playlist…");
            let playlist = client.get_playlist(&url_id).await?;
            manager.download_playlist(playlist, ui).await
        }
        "featured" => {
            ui.info("Fetching featured collection…");
            let playlist = client.get_playlist(&url_id).await?;
            manager.download_playlist(playlist, ui).await
        }
        "artist" => {
            ui.info("Fetching artist discography…");
            let (artist_name, albums) = client.get_artist_albums(&url_id).await?;
            ui.info(&format!("Artist: {} — {} albums found", artist_name, albums.len()));
            manager.download_artist(&artist_name, albums, ui).await
        }
        other => {
            return Err(anyhow::anyhow!("Unknown URL type '{}'", other));
        }
    };

    Ok(results)
}

// ─── Update checker ───────────────────────────────────────────────────────────

async fn check_for_update(ui: &DownloadUI) {
    let current = env!("CARGO_PKG_VERSION");
    let client = match reqwest::Client::builder()
        .use_rustls_tls()
        .user_agent(format!("jiosaavn-rs/{}", current))
        .timeout(std::time::Duration::from_secs(5))
        .build()
    {
        Ok(c) => c,
        Err(_) => return,
    };

    let url = "https://api.github.com/repos/habitual69/jiosaavn-rs/releases/latest";
    let resp = match client.get(url).send().await {
        Ok(r) => r,
        Err(_) => return,
    };
    let json: serde_json::Value = match resp.json().await {
        Ok(v) => v,
        Err(_) => return,
    };
    if let Some(tag) = json["tag_name"].as_str() {
        let latest = tag.trim_start_matches('v');
        let current_clean = current.trim_start_matches('v');
        if latest != current_clean {
            ui.info(&format!(
                "Update available: v{} → {} (run: cargo install jiosaavn)",
                current_clean, latest
            ));
        }
    }
}

// ─── Download log ─────────────────────────────────────────────────────────────

async fn log_history(config: &Config, url: &str, results: &[DownloadResult]) {
    use tokio::io::AsyncWriteExt;

    let ok     = results.iter().filter(|r| r.success()).count();
    let failed = results.len() - ok;

    // Determine kind and title from results
    let kind = if results.len() == 1 { "song" } else if url.contains("/album/") { "album" } else { "playlist" };
    let title = results.first().map(|r| r.track.album.as_str()).unwrap_or("").to_string();

    let timestamp = chrono::Utc::now().to_rfc3339();
    let line = format!(
        "{{\"timestamp\":\"{}\",\"url\":\"{}\",\"kind\":\"{}\",\"title\":\"{}\",\"tracks_ok\":{},\"tracks_failed\":{}}}\n",
        timestamp, url, kind, title.replace('"', "\\\""), ok, failed
    );

    let log_path = config.download_path().join("history.jsonl");
    if let Ok(mut file) = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .await
    {
        let _ = file.write_all(line.as_bytes()).await;
    }
}
