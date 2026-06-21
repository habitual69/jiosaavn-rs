# Changelog

All notable changes to jiosaavn-rs are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [1.0.0] — 2026-06-22

### Added

#### Core downloader
- Stream M4A (AAC) audio directly from JioSaavn CDN at 320 kbps
- Optional FFmpeg-based conversion to MP3 at selectable quality (`320k`, `256k`, `192k`, `128k`)
- Configurable concurrency with `tokio` semaphore (default 5 parallel downloads)
- Automatic retry with exponential back-off (3 attempts, 1 s → 2 s → 4 s) on timeout/connect errors
- Skip logic — already-downloaded files (`.m4a` or `.mp3`) are never re-fetched
- Region-unavailable tracks reported as failed without crashing the batch

#### API client (`api.rs`)
- JioSaavn API support: songs, albums, playlists (paginated, 100 tracks/page), featured playlists
- CDN auth-token resolution via `song.generateAuthToken`
- Lyrics fetching via `lyrics.getLyrics` with HTML `<br>` → newline normalisation
- URL parser accepting all four JioSaavn URL patterns
- HTML entity unescaping for titles and album names (`&amp;`, `&quot;`, `&#039;`, etc.)

#### Metadata tagging
- **M4A** — full `mp4ameta` tag set: title, artist, album artist, album, year/release date, track number, total tracks, genre (language), copyright, lyrics, 500×500 JPEG artwork, Apple parental advisory (explicit / clean)
- **MP3** — ID3v2.4 tags via `id3`: title, artist, album artist, album, year, TRCK (`pos/total`), TCOM (composer), TCOP (copyright), USLT (lyrics), APIC (front cover art)
- Album art fetched at 500×500 resolution; albums pre-fetch once and share across all tracks
- `cover.jpg` saved in every album/playlist folder

#### CLI
- `clap`-based argument parser with `--help` and `--version`
- Flags: `-f/--format`, `-q/--quality`, `-o/--output`, `-c/--concurrent`, `--save-config`, `--show-config`
- Config persisted as `jiosaavn_config.json` beside the binary; created with defaults on first run

#### Terminal UI
- ASCII banner rendered in cyan
- Per-track progress bars with spinner, byte counter, and transfer rate
- Overall progress bar tracking completed tracks
- Colour-coded summary table (✓ green / ✗ red / ⊘ yellow) at the end of every run

#### Packaging
- `build.rs` embeds `jsd.ico` into the Windows PE header via `winres`
- `scripts/install_linux.sh` — installs binary, PNG icon, and `.desktop` entry (user or system-wide)
- `scripts/package_macos.sh` — creates a signed `.app` bundle; supports `universal` flag for fat binary (arm64 + x86_64)
- `scripts/package_windows.sh` — cross-compiles `.exe` from Linux via `mingw-w64`
- GitHub Actions release workflow — builds and publishes native binaries for Linux, macOS (arm64 + x86_64), and Windows on every `v*` tag push

#### Release assets (v1.0.0)
| Asset | Platform |
|-------|----------|
| `jiosaavn-linux-x86_64.tar.gz` | Linux x86-64 |
| `jiosaavn-macos-aarch64.tar.gz` | macOS Apple Silicon |
| `jiosaavn-macos-x86_64.tar.gz` | macOS Intel |
| `jiosaavn-windows-x86_64.zip` | Windows x86-64 |

[1.0.0]: https://github.com/habitual69/jiosaavn-rs/releases/tag/v1.0.0
