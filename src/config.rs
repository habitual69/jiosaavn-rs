use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const CONFIG_FILE: &str = "jiosaavn_config.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub download_dir: String,
    pub max_concurrent_downloads: usize,
    pub output_format: String,
    pub mp3_quality: String,
    pub bitrate: String,
    pub timeout_secs: u64,
    pub chunk_size: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            download_dir: "Downloads".to_string(),
            max_concurrent_downloads: 5,
            output_format: "mp3".to_string(),
            mp3_quality: "320k".to_string(),
            bitrate: "320".to_string(),
            timeout_secs: 30,
            chunk_size: 8192,
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(s) => match serde_json::from_str(&s) {
                    Ok(c) => return c,
                    Err(e) => eprintln!("Warning: could not parse config: {e}"),
                },
                Err(e) => eprintln!("Warning: could not read config: {e}"),
            }
        }
        let cfg = Self::default();
        cfg.save(); // create default config on first run
        cfg
    }

    pub fn save(&self) {
        let path = Self::config_path();
        let json = serde_json::to_string_pretty(self).unwrap_or_default();
        let _ = std::fs::write(&path, json);
    }

    pub fn print(&self) {
        println!("Current configuration:");
        println!("  Download directory  : {}", self.download_dir);
        println!("  Output format       : {}", self.output_format);
        println!("  MP3 quality         : {}", self.mp3_quality);
        println!("  Concurrent downloads: {}", self.max_concurrent_downloads);
        println!("  Config file         : {}", Self::config_path().display());
    }

    pub fn download_path(&self) -> PathBuf {
        PathBuf::from(&self.download_dir)
    }

    fn config_path() -> PathBuf {
        // Sit beside the executable when frozen, otherwise in CWD
        if let Ok(exe) = std::env::current_exe() {
            let candidate = exe.parent().unwrap_or(std::path::Path::new(".")).join(CONFIG_FILE);
            // Use CWD if exe dir looks like a temp dir (e.g. cargo run)
            if candidate.parent().map(|p| p.to_string_lossy().contains("target")).unwrap_or(false) {
                return PathBuf::from(CONFIG_FILE);
            }
            return candidate;
        }
        PathBuf::from(CONFIG_FILE)
    }
}
