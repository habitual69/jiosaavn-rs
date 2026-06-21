use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};
use tokio::process::Command;

pub fn is_ffmpeg_available() -> bool {
    which_ffmpeg().is_some()
}

fn which_ffmpeg() -> Option<PathBuf> {
    // Check PATH
    let name = if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" };
    std::env::var_os("PATH")
        .and_then(|paths| {
            std::env::split_paths(&paths)
                .map(|dir| dir.join(name))
                .find(|p| p.is_file())
        })
}

pub async fn convert_to_mp3(input: &Path, quality: &str) -> Result<PathBuf> {
    let ffmpeg = which_ffmpeg()
        .ok_or_else(|| anyhow!(
            "FFmpeg not found in PATH. Install it or use -f m4a to keep the original format.\n\
             https://ffmpeg.org/download.html"
        ))?;

    let output = input.with_extension("mp3");

    let status = Command::new(ffmpeg)
        .args([
            "-y",
            "-i",
            input.to_str().unwrap(),
            "-codec:a",
            "libmp3lame",
            "-b:a",
            quality,
            "-q:a",
            "0",
            "-map_metadata",
            "0",
            "-id3v2_version",
            "3",
            output.to_str().unwrap(),
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await?;

    if !status.success() {
        return Err(anyhow!("FFmpeg exited with code {:?}", status.code()));
    }

    if !output.exists() {
        return Err(anyhow!("FFmpeg finished but output file is missing"));
    }

    // Remove the source .m4a
    let _ = tokio::fs::remove_file(input).await;

    Ok(output)
}
