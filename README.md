<div align="center">

```
     ██╗██╗ ██████╗ ███████╗ █████╗  █████╗ ██╗   ██╗███╗   ██╗
     ██║██║██╔═══██╗██╔════╝██╔══██╗██╔══██╗██║   ██║████╗  ██║
     ██║██║██║   ██║███████╗███████║███████║██║   ██║██╔██╗ ██║
██   ██║██║██║   ██║╚════██║██╔══██║██╔══██║╚██╗ ██╔╝██║╚██╗██║
╚█████╔╝██║╚██████╔╝███████║██║  ██║██║  ██║ ╚████╔╝ ██║ ╚████║
 ╚════╝ ╚═╝ ╚═════╝ ╚══════╝╚═╝  ╚═╝╚═╝  ╚═╝  ╚═══╝  ╚═╝  ╚═══╝
```

**High-Performance JioSaavn Music Downloader**

[![Release](https://img.shields.io/github/v/release/habitual69/jiosaavn-rs?style=flat-square&color=orange)](https://github.com/habitual69/jiosaavn-rs/releases/latest)
[![License](https://img.shields.io/github/license/habitual69/jiosaavn-rs?style=flat-square)](LICENSE)
[![Build](https://img.shields.io/github/actions/workflow/status/habitual69/jiosaavn-rs/release.yml?style=flat-square)](https://github.com/habitual69/jiosaavn-rs/actions)
[![Platform](https://img.shields.io/badge/platform-Linux%20%7C%20macOS%20%7C%20Windows-blue?style=flat-square)](#installation)

Download songs, albums, and playlists from JioSaavn with full metadata tagging, album art embedding, and optional MP3 conversion — all from your terminal.

</div>

---

## Features

- **Songs, Albums & Playlists** — paste any JioSaavn URL and it just works
- **Concurrent downloads** — configurable parallelism (default 5 tracks at once)
- **M4A or MP3 output** — keeps the native AAC stream or converts via FFmpeg
- **Full metadata tagging** — title, artist, album, year, track number, genre, composer, copyright, lyrics
- **Album art** — 500×500 JPEG embedded in every file; `cover.jpg` saved alongside albums
- **Synced lyrics** — fetched and embedded when available
- **Progress UI** — per-track spinners + byte-level progress + summary table
- **Persistent config** — settings saved to JSON; override any flag per run
- **Cross-platform** — native binaries for Linux, macOS (Apple Silicon & Intel), and Windows

---

## Installation

### Pre-built binaries (recommended)

Download the latest release for your platform from the [Releases](https://github.com/habitual69/jiosaavn-rs/releases/latest) page.

| Platform | File |
|----------|------|
| Linux (x86_64) | `jiosaavn-linux-x86_64.tar.gz` |
| macOS (Apple Silicon) | `jiosaavn-macos-aarch64.tar.gz` |
| macOS (Intel) | `jiosaavn-macos-x86_64.tar.gz` |
| Windows (x86_64) | `jiosaavn-windows-x86_64.zip` |

**Linux / macOS**
```sh
tar xzf jiosaavn-*.tar.gz
chmod +x jiosaavn
sudo mv jiosaavn /usr/local/bin/
```

**Windows** — extract the zip and add the folder to your `PATH`.

### Build from source

```sh
git clone https://github.com/habitual69/jiosaavn-rs
cd jiosaavn-rs
cargo build --release
# binary at target/release/jiosaavn
```

Requires Rust 1.75+ (`rustup update stable`).

---

## Prerequisites

- **FFmpeg** — required only when using `-f mp3` (MP3 output)
  - Linux: `sudo apt install ffmpeg` / `sudo pacman -S ffmpeg`
  - macOS: `brew install ffmpeg`
  - Windows: [ffmpeg.org/download.html](https://ffmpeg.org/download.html)

M4A output works without FFmpeg.

---

## Usage

```
jiosaavn [OPTIONS] <URL>
```

### Arguments

| Argument | Description |
|----------|-------------|
| `<URL>` | JioSaavn URL — song, album, or playlist |

### Options

| Flag | Default | Description |
|------|---------|-------------|
| `-f, --format <FORMAT>` | `mp3` | Output format: `mp3` or `m4a` |
| `-q, --quality <QUALITY>` | `320k` | MP3 bitrate: `320k`, `256k`, `192k`, `128k` |
| `-o, --output <DIR>` | `Downloads` | Output directory |
| `-c, --concurrent <N>` | `5` | Max concurrent downloads |
| `--save-config` | — | Save current flags as defaults |
| `--show-config` | — | Print current config and exit |
| `-h, --help` | — | Show help |
| `-V, --version` | — | Show version |

### Examples

```sh
# Download a single song as MP3 (320k)
jiosaavn https://www.jiosaavn.com/song/song-name/XXXXX

# Download a full album as M4A
jiosaavn -f m4a https://www.jiosaavn.com/album/album-name/XXXXX

# Download a playlist with 10 parallel downloads
jiosaavn -c 10 https://www.jiosaavn.com/s/playlist/user/playlist-name/XXXXX

# Save settings so every future run uses 256k MP3 in ~/Music
jiosaavn -f mp3 -q 256k -o ~/Music --save-config https://www.jiosaavn.com/...

# Check what settings are active
jiosaavn --show-config
```

---

## Output structure

```
Downloads/
├── Artist - Album [Year]/
│   ├── cover.jpg
│   ├── 01. Track Title.mp3
│   ├── 02. Track Title.mp3
│   └── ...
└── Playlist - Playlist Name/
    ├── cover.jpg
    ├── 01. Track Title.mp3
    └── ...
```

---

## Supported URL formats

| Type | Example |
|------|---------|
| Song | `https://www.jiosaavn.com/song/name/TOKEN` |
| Album | `https://www.jiosaavn.com/album/name/TOKEN` |
| Playlist | `https://www.jiosaavn.com/s/playlist/user/name/TOKEN` |
| Featured | `https://www.jiosaavn.com/featured/name/TOKEN` |

---

## Configuration

On first run a config file (`jiosaavn_config.json`) is created next to the binary with these defaults:

```json
{
  "download_dir": "Downloads",
  "max_concurrent_downloads": 5,
  "output_format": "mp3",
  "mp3_quality": "320k",
  "bitrate": "320",
  "timeout_secs": 30,
  "chunk_size": 8192
}
```

Use `--save-config` to persist any CLI override, or edit the file directly.

---

## Platform packaging

```sh
# Linux — install binary + icon + .desktop entry
./scripts/install_linux.sh          # user install
sudo ./scripts/install_linux.sh     # system-wide

# macOS — build .app bundle (add "universal" for fat binary)
./scripts/package_macos.sh
./scripts/package_macos.sh universal

# Windows — cross-compile .exe from Linux
./scripts/package_windows.sh
```

---

## License

MIT — see [LICENSE](LICENSE).

---

<div align="center">
Made with ♥ in Rust · <a href="https://github.com/habitual69/jiosaavn-rs/issues">Report a bug</a>
</div>
