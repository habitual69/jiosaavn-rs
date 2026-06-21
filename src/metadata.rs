use anyhow::Result;
use id3::{
    frame::{Lyrics, Picture, PictureType},
    Tag, TagLike, Version,
};
use mp4ameta::{Img, ImgFmt, Tag as Mp4Tag};
use sanitize_filename::sanitize;
use std::path::Path;

use crate::models::Track;

// ─── M4A / MP4 tagging ───────────────────────────────────────────────────────

pub fn tag_m4a(
    path: &Path,
    track: &Track,
    album_art: Option<&[u8]>,
    lyrics: Option<&str>,
    position: usize,
    total: usize,
    album_artist: Option<&str>,
) -> Result<()> {
    let mut tag = Mp4Tag::read_from_path(path)?;

    tag.set_title(&track.title);
    tag.set_artist(&track.artists);
    tag.set_album_artist(album_artist.unwrap_or(&track.artists));
    tag.set_album(&track.album);

    let year = if !track.release_date.is_empty() {
        &track.release_date
    } else {
        &track.year
    };
    if !year.is_empty() {
        tag.set_year(year);
    }

    tag.set_track_number(position as u16);
    tag.set_total_tracks(total as u16);

    if !track.language.is_empty() {
        let genre = capitalise(&track.language);
        tag.set_genre(&genre);
    }

    if !track.copyright_text.is_empty() {
        tag.set_copyright(&track.copyright_text);
    }

    if let Some(l) = lyrics.filter(|s| !s.is_empty()) {
        tag.set_lyrics(l);
    }

    if let Some(art) = album_art {
        tag.add_artwork(Img::new(ImgFmt::Jpeg, art.to_vec()));
    }

    // Parental advisory: 4 = explicit, 2 = clean (Apple convention)
    tag.set_advisory_rating(if track.explicit {
        mp4ameta::AdvisoryRating::Explicit
    } else {
        mp4ameta::AdvisoryRating::Clean
    });

    tag.write_to_path(path)?;
    Ok(())
}

// ─── MP3 / ID3 tagging ───────────────────────────────────────────────────────

pub fn tag_mp3(
    path: &Path,
    track: &Track,
    album_art: Option<&[u8]>,
    lyrics: Option<&str>,
    position: usize,
    total: usize,
) -> Result<()> {
    let mut tag = Tag::new();

    tag.set_title(&track.title);
    tag.set_artist(&track.artists);
    tag.set_album_artist(&track.artists);
    tag.set_album(&track.album);

    if let Ok(y) = track.year.parse::<i32>() {
        tag.set_year(y);
    }

    // TRCK frame encodes "pos/total"
    tag.set_text("TRCK", format!("{}/{}", position, total));

    if !track.language.is_empty() {
        tag.set_genre(capitalise(&track.language));
    }
    if !track.music.is_empty() {
        tag.set_text("TCOM", &track.music);
    }
    if !track.copyright_text.is_empty() {
        tag.set_text("TCOP", &track.copyright_text);
    }

    if let Some(l) = lyrics.filter(|s| !s.is_empty()) {
        tag.add_frame(Lyrics {
            lang: "eng".to_string(),
            description: String::new(),
            text: l.to_string(),
        });
    }

    if let Some(art) = album_art {
        tag.add_frame(Picture {
            mime_type: "image/jpeg".to_string(),
            picture_type: PictureType::CoverFront,
            description: "Cover".to_string(),
            data: art.to_vec(),
        });
    }

    tag.write_to_path(path, Version::Id3v24)?;
    Ok(())
}

// ─── Filename / folder helpers ────────────────────────────────────────────────

pub fn generate_filename(track: &Track, position: usize, ext: &str) -> String {
    format!("{:02}. {}{}", position, sanitize(&track.title), ext)
}

pub fn generate_album_folder(artists: &str, album: &str, year: &str) -> String {
    let artist_display = if artists.chars().filter(|&c| c == ',').count() >= 2 {
        "Various Artists".to_string()
    } else {
        sanitize(artists)
    };
    format!("{} - {} [{}]", artist_display, sanitize(album), year)
}

fn capitalise(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}
