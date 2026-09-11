pub mod normalize {
    use sha2::{Digest, Sha256};
    pub const AUDIO: &[&str] = &[
        "mp3", "flac", "m4a", "aac", "ogg", "opus", "wav", "ape", "wv",
    ];
    pub fn is_audio(name: &str) -> bool {
        name.rsplit('.')
            .next()
            .map(|x| AUDIO.contains(&x.to_ascii_lowercase().as_str()))
            .unwrap_or(false)
    }
    pub fn clean(name: &str) -> String {
        let stem = name
            .rsplit(['\\', '/'])
            .next()
            .unwrap_or(name)
            .rsplit_once('.')
            .map(|x| x.0)
            .unwrap_or(name);
        let mut s = stem.to_lowercase();
        for mark in [
            "[flac]",
            "[320k]",
            "[320kbps]",
            "cd1",
            "cd2",
            "disc 1",
            "disc 2",
        ] {
            s = s.replace(mark, "")
        }
        s = s
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || matches!(c, '一'..='龥') {
                    c
                } else {
                    ' '
                }
            })
            .collect();
        while s.contains("  ") {
            s = s.replace("  ", " ")
        }
        s.trim()
            .trim_start_matches(|c: char| c.is_ascii_digit())
            .trim_matches(|c: char| c == ' ' || c == '-')
            .to_string()
    }
    pub fn id(parts: &str) -> String {
        let mut h = Sha256::new();
        h.update(parts.as_bytes());
        format!("track_{}", &hex::encode(h.finalize())[..16])
    }
    pub fn parse(name: &str) -> (String, String) {
        let filename = name.rsplit(['\\', '/']).next().unwrap_or(name);
        let stem = filename.rsplit_once('.').map(|x| x.0).unwrap_or(filename);
        if let Some((artist, title)) = stem.split_once(" - ") {
            (clean(title), clean(artist))
        } else {
            (clean(stem), "未知艺术家".into())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::parse;

        #[test]
        fn parses_artist_title_separator_before_cleaning() {
            assert_eq!(
                parse(r"Music\Chris Medina - What Are Words. flac"),
                ("what are words".into(), "chris medina".into())
            );
        }
    }
}
pub mod score {
    use crate::models::{Settings, Source};
    pub fn source(s: &Source, cfg: &Settings) -> f64 {
        let quality = match s.format.to_ascii_lowercase().as_str() {
            "flac" | "alac" => 100.0,
            "ape" | "wav" | "wv" => 92.0,
            "mp3" if s.bitrate.unwrap_or(0) >= 320 => 82.0,
            "aac" | "m4a" => 75.0,
            "mp3" => 60.0,
            _ => 45.0,
        };
        let mb = s.upload_speed.unwrap_or(0) as f64 / 1_048_576.0;
        let speed = if mb > 10.0 {
            100.0
        } else if mb > 5.0 {
            90.0
        } else if mb > 2.0 {
            80.0
        } else if mb > 1.0 {
            70.0
        } else if mb > 0.5 {
            55.0
        } else if mb > 0.1 {
            30.0
        } else {
            10.0
        };
        let q = match s.queue_length.unwrap_or(99) {
            0 => 100.0,
            1..=2 => 90.0,
            3..=5 => 75.0,
            6..=10 => 50.0,
            11..=30 => 25.0,
            _ => 5.0,
        };
        match cfg.selector_mode.as_str() {
            "speed" => speed * 0.5 + q * 0.25 + quality * 0.15 + 10.0,
            "quality" => quality * 0.5 + speed * 0.2 + q * 0.15 + 15.0,
            _ => speed * 0.3 + quality * 0.25 + q * 0.2 + 25.0,
        }
    }
}
pub mod library {
    use crate::{db::Database, models::Track, services::normalize};
    use lofty::{
        file::{AudioFile, TaggedFileExt},
        probe::Probe,
        tag::Accessor,
    };
    use std::path::Path;
    use walkdir::WalkDir;
    pub async fn scan(root: &Path, db: Database) -> Result<Vec<Track>, String> {
        let root = root.to_path_buf();
        tokio::task::spawn_blocking(move || {
            let mut out = vec![];
            for e in WalkDir::new(root)
                .follow_links(false)
                .into_iter()
                .filter_map(Result::ok)
            {
                let p = e.path();
                if !p.is_file() || !normalize::is_audio(&p.to_string_lossy()) {
                    continue;
                }
                let name = p.file_name().unwrap_or_default().to_string_lossy();
                let (title0, artist0) = normalize::parse(&name);
                let tagged = Probe::open(p).ok().and_then(|x| x.read().ok());
                let tag = tagged
                    .as_ref()
                    .and_then(|x| x.primary_tag().or_else(|| x.first_tag()));
                let title = tag
                    .and_then(|x| x.title())
                    .map(|x| x.to_string())
                    .unwrap_or(title0);
                let artist = tag
                    .and_then(|x| x.artist())
                    .map(|x| x.to_string())
                    .unwrap_or(artist0);
                let album = tag
                    .and_then(|x| x.album())
                    .map(|x| x.to_string())
                    .unwrap_or_default();
                let duration = tagged
                    .as_ref()
                    .map(|x| x.properties().duration().as_secs_f64())
                    .unwrap_or(0.);
                let format = p
                    .extension()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_uppercase();
                let t = Track {
                    id: normalize::id(&format!("{}{}{}", title, artist, duration.round())),
                    title,
                    artist,
                    album,
                    duration,
                    cover: None,
                    local_path: Some(p.to_string_lossy().into()),
                    formats: vec![format],
                    source_count: 1,
                    favorite: false,
                    sources: vec![],
                };
                let _ = db.save_track(&t);
                out.push(t)
            }
            Ok(out)
        })
        .await
        .map_err(|e| e.to_string())?
    }
}
