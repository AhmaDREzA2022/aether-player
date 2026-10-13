use aether_common::{PlayerError, Track};
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::tag::Accessor;
use std::path::{Path, PathBuf};
use walkdir::{DirEntry, WalkDir};

pub const SUPPORTED_EXTENSIONS: &[&str] = &["mp3", "flac", "ogg", "wav", "m4a"];

/// Read one audio file into a `Track`.
/// Missing tags fall back to the file name / "Unknown ..." instead of failing.
pub fn read_track(path: &Path) -> Result<Track, PlayerError> {
    if !path.exists() {
        return Err(PlayerError::NotFound(path.display().to_string()));
    }
    let tagged = lofty::read_from_path(path)
        .map_err(|e| PlayerError::UnsupportedFormat(format!("{}: {e}", path.display())))?;

    let duration = tagged.properties().duration();
    let tag = tagged.primary_tag().or_else(|| tagged.first_tag());

    let text = |value: Option<std::borrow::Cow<'_, str>>| {
        value
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    };

    let title = tag
        .and_then(|t| text(t.title()))
        .unwrap_or_else(|| file_stem(path));
    let artist = tag
        .and_then(|t| text(t.artist()))
        .unwrap_or_else(|| "Unknown artist".to_string());
    let album = tag
        .and_then(|t| text(t.album()))
        .unwrap_or_else(|| "Unknown album".to_string());
    let track_no = tag.and_then(|t| t.track());

    Ok(Track {
        path: path.to_path_buf(),
        title,
        artist,
        album,
        duration,
        track_no,
    })
}

/// Find supported audio files under `root` without reading their tags.
/// Hidden files and folders are skipped. The result is sorted by path.
pub fn find_audio_files(root: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| !is_hidden(e))
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| has_supported_extensions(e.path()))
        .map(|e| e.into_path())
        .collect();
    files.sort();
    files
}

/// Recursively scan a folder and read every supported file.
/// Files that fail to read are logged and skipped.
pub fn scan_folder(root: &Path) -> Vec<Track> {
    find_audio_files(root)
        .iter()
        .filter_map(|path| match read_track(path) {
            Ok(track) => Some(track),
            Err(err) => {
                tracing::warn!("skipping {}: {err}", path.display());
                None
            }
        })
        .collect()
}

fn is_hidden(entry: &DirEntry) -> bool {
    entry.depth() > 0 && entry.file_name().to_string_lossy().starts_with('.')
}

fn has_supported_extensions(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| SUPPORTED_EXTENSIONS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

fn file_stem(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Unknown title")
        .to_string()
}

#[cfg(test)]
mod tests {

    use super::*;
    use std::path::Path;
    use std::{fs, vec};

    /// Writes a valid wav file for silence so tests need no commited audio.
    fn write_silent_wav(path: &Path, seconds: u32) {
        let sample_rate: u32 = 8000;
        let data_len = sample_rate * 2 * seconds; // 16-bit mono
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&(36 + data_len).to_le_bytes());
        bytes.extend_from_slice(b"WAVEfmt ");
        bytes.extend_from_slice(&16u32.to_le_bytes()); // fmt chunk size
        bytes.extend_from_slice(&1u16.to_le_bytes()); // PCM
        bytes.extend_from_slice(&1u16.to_le_bytes()); // mono
        bytes.extend_from_slice(&sample_rate.to_le_bytes());
        bytes.extend_from_slice(&(sample_rate * 2).to_le_bytes()); // byte rate
        bytes.extend_from_slice(&2u16.to_le_bytes()); // block align
        bytes.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&data_len.to_le_bytes());
        bytes.extend(std::iter::repeat(0u8).take(data_len as usize));
        fs::write(path, bytes).unwrap();
    }

    #[test]
    fn scans_nested_folders_and_ignores_the_rest() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("sub")).unwrap();
        fs::create_dir_all(root.join(".hidden")).unwrap();

        write_silent_wav(&root.join("a.wav"), 1);
        write_silent_wav(&root.join("sub/b.wav"), 1);
        write_silent_wav(&root.join(".hidden/c.wav"), 1);
        fs::write(root.join("notes.txt"), "not music").unwrap();

        let tracks = scan_folder(root);
        let names: Vec<_> = tracks.iter().map(|t| t.title.as_str()).collect();
        assert_eq!(names, vec!["a", "b"]);
    }

    #[test]
    fn missing_tags_fail_back_to_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("my song.wav");
        write_silent_wav(&path, 1);

        let track = read_track(&path).unwrap();
        assert_eq!(track.title, "my song");
        assert_eq!(track.artist, "Unknown artist");
        assert_eq!(track.album, "Unknown album");
        assert_eq!(track.track_no, None);
        assert_eq!(track.duration.as_secs(), 1);
    }

    #[test]
    fn corrupt_files_are_skipped_not_fatal() {
        let dir = tempfile::tempdir().unwrap();
        write_silent_wav(&dir.path().join("good.wav"), 1);
        fs::write(dir.path().join("bad.flac"), "this is not audio").unwrap();

        let tracks = scan_folder(dir.path());
        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0].title, "good");
    }

    #[test]
    fn reading_a_missing_returns_not_found() {
        let err = read_track(Path::new("/definitely/not/here.mp3")).unwrap_err();
        assert!(matches!(err, PlayerError::NotFound(_)));
    }

    #[test]
    fn tags_are_read_into_the_right_fields() {
        use lofty::config::WriteOptions;
        use lofty::tag::{Tag, TagExt, TagType};

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("song.wav");
        write_silent_wav(&path, 1);

        let mut tag = Tag::new(TagType::Id3v2);
        tag.set_title("The Title".to_string());
        tag.set_artist("The Artist".to_string());
        tag.set_album("The Album".to_string());
        tag.set_track(7);
        tag.save_to_path(&path, WriteOptions::default()).unwrap();

        let track = read_track(&path).unwrap();
        assert_eq!(track.title, "The Title");
        assert_eq!(track.artist, "The Artist");
        assert_eq!(track.album, "The Album");
        assert_eq!(track.track_no, Some(7));
    }
}
