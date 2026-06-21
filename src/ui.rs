use console::style;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::sync::Arc;

use crate::models::{Album, DownloadResult, DownloadStatus, Playlist, Track};

const LOGO: &str = r#"
     ██╗██╗ ██████╗ ███████╗ █████╗  █████╗ ██╗   ██╗███╗   ██╗
     ██║██║██╔═══██╗██╔════╝██╔══██╗██╔══██╗██║   ██║████╗  ██║
     ██║██║██║   ██║███████╗███████║███████║██║   ██║██╔██╗ ██║
██   ██║██║██║   ██║╚════██║██╔══██║██╔══██║╚██╗ ██╔╝██║╚██╗██║
╚█████╔╝██║╚██████╔╝███████║██║  ██║██║  ██║ ╚████╔╝ ██║ ╚████║
 ╚════╝ ╚═╝ ╚═════╝ ╚══════╝╚═╝  ╚═╝╚═╝  ╚═╝  ╚═══╝  ╚═╝  ╚═══╝
                   High-Performance Music Downloader v1.0 (Rust)
"#;

pub struct DownloadUI {
    pub multi: Arc<MultiProgress>,
}

impl DownloadUI {
    pub fn new() -> Self {
        Self {
            multi: Arc::new(MultiProgress::new()),
        }
    }

    pub fn display_logo(&self) {
        println!("{}", style(LOGO).cyan().bold());
    }

    pub fn info(&self, msg: &str) {
        eprintln!("{} {}", style("ℹ").blue().bold(), msg);
    }

    pub fn success(&self, msg: &str) {
        eprintln!("{} {}", style("✓").green().bold(), msg);
    }

    pub fn error(&self, msg: &str) {
        eprintln!("{} {}", style("✗").red().bold(), msg);
    }

    pub fn warn(&self, msg: &str) {
        eprintln!("{} {}", style("⚠").yellow().bold(), msg);
    }

    pub fn display_album_info(&self, album: &Album) {
        println!();
        println!("{}", style("─── Album ─────────────────────────────").dim());
        println!("  {}  {}", style("Title:").cyan().bold(),  album.title);
        println!("  {}  {}", style("Artist:").cyan().bold(), album.artists);
        println!("  {}   {}", style("Year:").cyan().bold(),  album.year);
        println!("  {} {}", style("Tracks:").cyan().bold(),  album.track_count());
        println!("{}", style("────────────────────────────────────────").dim());
        println!();
    }

    pub fn display_playlist_info(&self, playlist: &Playlist) {
        println!();
        println!("{}", style("─── Playlist ───────────────────────────").dim());
        println!("  {}  {}", style("Name:").cyan().bold(),   playlist.name);
        println!("  {} {}", style("Tracks:").cyan().bold(),  playlist.track_count());
        println!("{}", style("────────────────────────────────────────").dim());
        println!();
    }

    pub fn display_track_info(&self, track: &Track) {
        println!();
        println!("{}", style("─── Track ──────────────────────────────").dim());
        println!("  {}  {}", style("Title:").cyan().bold(),  track.title);
        println!("  {} {}", style("Artist:").cyan().bold(),  track.artists);
        println!("  {}  {}", style("Album:").cyan().bold(),  track.album);
        println!("{}", style("────────────────────────────────────────").dim());
        println!();
    }

    /// Create a per-track progress bar attached to the shared MultiProgress.
    pub fn add_track_bar(&self, track: &Track, total_bytes: u64) -> ProgressBar {
        let pb = self.multi.add(ProgressBar::new(total_bytes));
        let style = ProgressStyle::with_template(
            "  {spinner:.cyan} {msg:<40} [{bar:30.green/dim}] {bytes:>8}/{total_bytes:<8} {binary_bytes_per_sec}",
        )
        .unwrap()
        .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]);
        pb.set_style(style);
        let title = if track.title.len() > 38 {
            format!("{}…", &track.title[..37])
        } else {
            track.title.clone()
        };
        pb.set_message(title);
        pb.enable_steady_tick(std::time::Duration::from_millis(80));
        pb
    }

    /// Create an overall progress bar.
    pub fn add_overall_bar(&self, total: u64) -> ProgressBar {
        let pb = self.multi.add(ProgressBar::new(total));
        let style = ProgressStyle::with_template(
            "{spinner:.blue} Overall  [{bar:40.cyan/dim}] {pos}/{len} tracks",
        )
        .unwrap()
        .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]);
        pb.set_style(style);
        pb.enable_steady_tick(std::time::Duration::from_millis(80));
        pb
    }

    pub fn show_summary(&self, results: &[DownloadResult]) {
        println!();
        println!("{}", style("─── Download Summary ───────────────────").cyan());
        println!(
            "  {:<4} {:<42} {:<26} {}",
            style("#").bold(),
            style("Track").bold(),
            style("Artist").bold(),
            style("Status").bold(),
        );
        println!("{}", style("────────────────────────────────────────────────────────────────────────────").dim());

        for (i, r) in results.iter().enumerate() {
            let status = match &r.status {
                DownloadStatus::Completed => style("✓ Complete").green().to_string(),
                DownloadStatus::Failed(e) => style(format!("✗ {}", &e[..e.len().min(20)])).red().to_string(),
                DownloadStatus::Skipped => style("⊘ Skipped").yellow().to_string(),
            };
            println!(
                "  {:<4} {:<42} {:<26} {}",
                i + 1,
                truncate(&r.track.title, 40),
                truncate(&r.track.artists, 24),
                status,
            );
        }

        println!("{}", style("────────────────────────────────────────────────────────────────────────────").dim());

        let ok = results.iter().filter(|r| r.success()).count();
        let fail = results.iter().filter(|r| matches!(r.status, DownloadStatus::Failed(_))).count();
        let skip = results.iter().filter(|r| matches!(r.status, DownloadStatus::Skipped)).count();
        println!(
            "\n  {} completed   {} failed   {} skipped\n",
            style(ok).green().bold(),
            style(fail).red().bold(),
            style(skip).yellow().bold(),
        );
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() > max {
        format!("{}…", &s[..max - 1])
    } else {
        s.to_string()
    }
}
