mod api;
mod config;
mod converter;
mod downloader;
mod metadata;
mod models;
mod ui;

use anyhow::Result;
use clap::{Parser, ValueEnum};
use std::sync::Arc;

use api::JioSaavnClient;
use config::Config;
use downloader::DownloadManager;
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
    about       = "High-performance JioSaavn music downloader\nSupports songs, albums, and playlists",
    long_about  = None,
)]
struct Cli {
    /// JioSaavn URL — song, album, or playlist
    #[arg(value_name = "URL")]
    url: Option<String>,

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

    if cli.save_config {
        config.save();
    }

    // Ensure download directory exists
    tokio::fs::create_dir_all(config.download_path()).await?;

    let url = match &cli.url {
        Some(u) => u.clone(),
        None => {
            eprintln!("Error: a JioSaavn URL is required.\nRun `jiosaavn --help` for usage.");
            std::process::exit(1);
        }
    };

    let ui = DownloadUI::new();
    ui.display_logo();

    if cli.save_config {
        ui.info(&format!(
            "Settings saved — format={}, quality={}, concurrent={}",
            config.output_format, config.mp3_quality, config.max_concurrent_downloads
        ));
    }

    let (url_type, url_id) = match JioSaavnClient::parse_url(&url) {
        Ok(pair) => pair,
        Err(e) => {
            ui.error(&e.to_string());
            ui.info("Supported URL forms: /song/…  /album/…  /s/playlist/…  /featured/…");
            std::process::exit(1);
        }
    };

    let client = Arc::new(JioSaavnClient::new()?);
    let manager = DownloadManager::new(Arc::clone(&client), config)?;

    match url_type.as_str() {
        "song" => {
            let result = manager.download_song(&url_id, &ui).await?;
            ui.show_summary(std::slice::from_ref(&result));
        }
        "album" => {
            ui.info("Fetching album…");
            let album = client.get_album(&url_id).await?;
            let results = manager.download_album(album, &ui).await;
            ui.show_summary(&results);
        }
        "playlist" => {
            ui.info("Fetching playlist…");
            let playlist = client.get_playlist(&url_id).await?;
            let results = manager.download_playlist(playlist, &ui).await;
            ui.show_summary(&results);
        }
        other => {
            ui.error(&format!("Unknown URL type '{other}'"));
            std::process::exit(1);
        }
    }

    ui.success("Done!");
    Ok(())
}
