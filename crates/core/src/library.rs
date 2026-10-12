use crate::error::LibraryError;
use aether_common::Track;
use aether_metadata::{find_audio_files, read_track};
use rusqlite::{params, params_from_iter, Connection, Row};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    time::{Duration, UNIX_EPOCH},
};

/// One entry per schema version. Never edit an old entry: add a new one.
const MIGRATIONS: &[&str] = &[
    // version 1: the track library
    "CREATE TABLE tracks (
        id          INTEGER PRIMARY KEY,
        path        TEXT    NOT NULL UNIQUE,
        title       TEXT    NOT NULL,
        artist      TEXT    NOT NULL,
        album       TEXT    NOT NULL,
        duration_ms INTEGER NOT NULL,
        track_no    INTEGER,
        mtime       INTEGER NOT NULL
    );",
    // version 2: favorites
    "CREATE TABLE favorites (
        id          INTEGER PRIMARY KEY,
        track_id    INTEGER NOT NULL UNIQUE REFERENCES tracks(id) ON DELETE CASCADE
    );",
];

const TRACK_COLUMNS: &str = "tracks.id, tracks.path, tracks.title, tracks.artist, \
                             tracks.album, tracks.duration_ms, tracks.track_no";

const TRACK_ORDER: &str = "ORDER BY tracks.artist COLLATE NOCASE, tracks.album COLLATE NOCASE, \
                           COALESCE(tracks.track_no, 0), tracks.title COLLATE NOCASE";

/// A track as stored in the library. The id is stable across rescans.
#[derive(Debug, Clone, PartialEq)]
pub struct LibraryTrack {
    pub id: i64,
    pub track: Track,
}

/// What a scan did.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SyncReport {
    pub added: usize,
    pub updated: usize,
    pub unchanged: usize,
    pub removed: usize,
    pub failed: usize,
}

pub struct Library {
    conn: Connection,
}

impl Library {
    /// Open (or create) a library database file.
    pub fn open(path: &Path) -> Result<Self, LibraryError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| LibraryError::Io(format!("{}: {e}", parent.display())))?;
        }
        Self::from_connection(Connection::open(path)?)
    }

    pub fn open_in_memory() -> Result<Self, LibraryError> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    fn from_connection(mut conn: Connection) -> Result<Self, LibraryError> {
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        migrate(&mut conn)?;
        Ok(Self { conn })
    }

    pub fn schema_version(&self) -> Result<i64, LibraryError> {
        Ok(self
            .conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))?)
    }

    pub fn track_count(&self) -> Result<usize, LibraryError> {
        let n: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM tracks", [], |r| r.get(0))?;
        Ok(n as usize)
    }

    pub fn tracks(&self) -> Result<Vec<LibraryTrack>, LibraryError> {
        self.query_tracks(TRACK_ORDER, [])
    }

    /// Case-insensitive search across title, artist and album. Every word
    /// of the query must match at least one of the three fields.
    pub fn search(&self, query: &str) -> Result<Vec<LibraryTrack>, LibraryError> {
        let patterns: Vec<String> = query
            .split_whitespace()
            .map(|word| format!("%{}%", escape_like(word)))
            .collect();
        if patterns.is_empty() {
            return self.tracks();
        }

        let conditions: Vec<String> = (1..=patterns.len())
            .map(|n| {
                format!(
                    "(title LIKE ?{n} ESCAPE '\\' OR artist LIKE ?{n} ESCAPE '\\' \
                     OR album LIKE ?{n} ESCAPE '\\')"
                )
            })
            .collect();

        self.query_tracks(
            &format!("WHERE {} {TRACK_ORDER}", conditions.join(" AND ")),
            params_from_iter(patterns),
        )
    }

    pub fn is_favorite(&self, track_id: i64) -> Result<bool, LibraryError> {
        Ok(self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM favorites WHERE track_id = ?1)",
            [track_id],
            |r| r.get(0),
        )?)
    }

    pub fn set_favorite(&self, track_id: i64, favorite: bool) -> Result<(), LibraryError> {
        self.require_track(track_id)?;
        if favorite {
            self.conn.execute(
                "INSERT OR IGNORE INTO favorites (track_id) VALUES (?1)",
                [track_id],
            )?;
        } else {
            self.conn
                .execute("DELETE FROM favorites WHERE track_id = ?1", [track_id])?;
        }
        Ok(())
    }

    /// Flip the favorite flag and return the new state.
    pub fn toggle_favorite(&self, track_id: i64) -> Result<bool, LibraryError> {
        let now_favorite = !self.is_favorite(track_id)?;
        self.set_favorite(track_id, now_favorite)?;
        Ok(now_favorite)
    }

    /// Favorites, most recently added first.
    pub fn favorites(&self) -> Result<Vec<LibraryTrack>, LibraryError> {
        self.query_tracks(
            "JOIN favorites ON favorites.track_id = tracks.id ORDER BY favorites.id DESC",
            [],
        )
    }

    fn require_track(&self, track_id: i64) -> Result<(), LibraryError> {
        let exists: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM tracks WHERE id = ?1)",
            [track_id],
            |r| r.get(0),
        )?;
        if exists {
            Ok(())
        } else {
            Err(LibraryError::TrackNotFound(track_id))
        }
    }

    /// Run `SELECT <track columns> FROM tracks <tail>` and map the rows.
    fn query_tracks(
        &self,
        tail: &str,
        params: impl rusqlite::Params,
    ) -> Result<Vec<LibraryTrack>, LibraryError> {
        let sql = format!("SELECT {TRACK_COLUMNS} FROM tracks {tail}");
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params, row_to_track)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Bring the library in line with a folder on disk. Files whose
    /// modification time is unchanged are skipped without reading their tags.
    pub fn sync_folder(&mut self, root: &Path) -> Result<SyncReport, LibraryError> {
        // One canonical form of the path, so scanning "~/Music" and
        // "/home/me/Music/." can't create duplicate rows.
        let root = fs::canonicalize(root)
            .map_err(|e| LibraryError::Io(format!("{}: {e}", root.display())))?;
        let files = find_audio_files(&root);
        let known = self.known_files()?;
        let mut report = SyncReport::default();
        let mut seen: HashSet<String> = HashSet::new();

        let tx = self.conn.transaction()?;

        for path in &files {
            let key = path.to_string_lossy().into_owned();
            seen.insert(key.clone());

            let mtime = match file_mtime_ms(path) {
                Ok(m) => m,
                Err(e) => {
                    tracing::warn!("cannot stat {}: {e}", path.display());
                    report.failed += 1;
                    continue;
                }
            };

            let is_known = known.contains_key(&key);
            if known.get(&key) == Some(&mtime) {
                report.unchanged += 1;
                continue;
            }

            match read_track(path) {
                Ok(track) => {
                    upsert_track(&tx, &track, mtime)?;
                    if is_known {
                        report.updated += 1;
                    } else {
                        report.added += 1;
                    }
                }
                Err(e) => {
                    tracing::warn!("skipping {}: {e}", path.display());
                    report.failed += 1;
                }
            }
        }

        // Files that used to be under this folder but are gone now.
        for key in known.keys() {
            if Path::new(key).starts_with(&root) && !seen.contains(key) {
                tx.execute("DELETE FROM tracks WHERE path = ?1", [key])?;
                report.removed += 1;
            }
        }

        tx.commit()?;
        Ok(report)
    }

    fn known_files(&self) -> Result<HashMap<String, i64>, LibraryError> {
        let mut stmt = self.conn.prepare("SELECT path, mtime FROM tracks")?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?;
        Ok(rows.collect::<Result<HashMap<_, _>, _>>()?)
    }
}

/// Apply every migration newer than the database's current version,
/// each in its own transaction.
fn migrate(conn: &mut Connection) -> Result<(), LibraryError> {
    let current: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    let supported = MIGRATIONS.len() as i64;

    if current > supported {
        return Err(LibraryError::SchemaTooNew {
            found: current,
            supported,
        });
    }

    for (i, sql) in MIGRATIONS.iter().enumerate().skip(current as usize) {
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.execute_batch(&format!("PRAGMA user_version = {};", i + 1))?;
        tx.commit()?;
    }
    Ok(())
}

/// Insert a track, or update it in place if the path already exists.
/// This keeps the row's id, which playlists will point at. (`INSERT OR REPLACE`
/// would delete and re-create the row with a new id.)
fn upsert_track(conn: &Connection, track: &Track, mtime: i64) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO tracks (path, title, artist, album, duration_ms, track_no, mtime)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(path) DO UPDATE SET
             title       = excluded.title,
             artist      = excluded.artist,
             album       = excluded.album,
             duration_ms = excluded.duration_ms,
             track_no    = excluded.track_no,
             mtime       = excluded.mtime",
        params![
            track.path.to_string_lossy(),
            track.title,
            track.artist,
            track.album,
            track.duration.as_millis() as i64,
            track.track_no,
            mtime,
        ],
    )?;
    Ok(())
}

/// Make a user's text safe to use inside a LIKE pattern, so typing `%` or `_`
/// searches for those characters instead of acting as wildcards.
fn escape_like(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

fn row_to_track(row: &Row<'_>) -> rusqlite::Result<LibraryTrack> {
    Ok(LibraryTrack {
        id: row.get(0)?,
        track: Track {
            path: PathBuf::from(row.get::<_, String>(1)?),
            title: row.get(2)?,
            artist: row.get(3)?,
            album: row.get(4)?,
            duration: Duration::from_millis(row.get::<_, i64>(5)?.max(0) as u64),
            track_no: row.get(6)?,
        },
    })
}

fn file_mtime_ms(path: &Path) -> std::io::Result<i64> {
    let modified = fs::metadata(path)?.modified()?;
    Ok(modified
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::LibraryError;
    use aether_common::testing::write_silent_wav;
    use std::time::SystemTime;

    /// A music folder with a.wav and sub/b.wav.
    fn music_dir() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("sub")).unwrap();
        write_silent_wav(&dir.path().join("a.wav"), 1);
        write_silent_wav(&dir.path().join("sub/b.wav"), 1);
        dir
    }

    fn find(lib: &Library, title: &str) -> LibraryTrack {
        lib.tracks()
            .unwrap()
            .into_iter()
            .find(|t| t.track.title == title)
            .unwrap()
    }

    #[test]
    fn first_scan_adds_every_track() {
        let dir = music_dir();
        let mut lib = Library::open_in_memory().unwrap();

        let report = lib.sync_folder(dir.path()).unwrap();
        assert_eq!(
            report,
            SyncReport {
                added: 2,
                ..Default::default()
            }
        );

        let titles: Vec<_> = lib
            .tracks()
            .unwrap()
            .into_iter()
            .map(|t| t.track.title)
            .collect();
        assert_eq!(titles, vec!["a", "b"]);
    }

    #[test]
    fn rescanning_does_not_duplicate_or_reread() {
        let dir = music_dir();
        let mut lib = Library::open_in_memory().unwrap();
        lib.sync_folder(dir.path()).unwrap();

        let report = lib.sync_folder(dir.path()).unwrap();
        assert_eq!(
            report,
            SyncReport {
                unchanged: 2,
                ..Default::default()
            }
        );
        assert_eq!(lib.track_count().unwrap(), 2);
    }

    #[test]
    fn a_different_spelling_of_the_same_path_does_not_duplicate() {
        let dir = music_dir();
        let mut lib = Library::open_in_memory().unwrap();
        lib.sync_folder(dir.path()).unwrap();

        lib.sync_folder(&dir.path().join("sub/..")).unwrap();
        assert_eq!(lib.track_count().unwrap(), 2);
    }

    #[test]
    fn a_changed_file_is_updated_and_keeps_its_id() {
        let dir = music_dir();
        let mut lib = Library::open_in_memory().unwrap();
        lib.sync_folder(dir.path()).unwrap();
        let before = find(&lib, "a");
        assert_eq!(before.track.duration.as_secs(), 1);

        // Rewrite a.wav longer, and push its modified time forward so the
        // test doesn't depend on the clock's precision.
        let a = dir.path().join("a.wav");
        write_silent_wav(&a, 2);
        fs::File::options()
            .write(true)
            .open(&a)
            .unwrap()
            .set_modified(SystemTime::now() + Duration::from_secs(10))
            .unwrap();

        let report = lib.sync_folder(dir.path()).unwrap();
        assert_eq!(report.updated, 1);
        assert_eq!(report.unchanged, 1);

        let after = find(&lib, "a");
        assert_eq!(after.id, before.id);
        assert_eq!(after.track.duration.as_secs(), 2);
    }

    #[test]
    fn deleted_files_are_removed() {
        let dir = music_dir();
        let mut lib = Library::open_in_memory().unwrap();
        lib.sync_folder(dir.path()).unwrap();

        fs::remove_file(dir.path().join("sub/b.wav")).unwrap();
        let report = lib.sync_folder(dir.path()).unwrap();

        assert_eq!(report.removed, 1);
        assert_eq!(report.unchanged, 1);
        assert_eq!(lib.track_count().unwrap(), 1);
    }

    #[test]
    fn unreadable_files_are_counted_not_fatal() {
        let dir = tempfile::tempdir().unwrap();
        write_silent_wav(&dir.path().join("good.wav"), 1);
        fs::write(dir.path().join("bad.flac"), "not audio").unwrap();

        let mut lib = Library::open_in_memory().unwrap();
        let report = lib.sync_folder(dir.path()).unwrap();
        assert_eq!(report.added, 1);
        assert_eq!(report.failed, 1);
    }

    #[test]
    fn the_library_survives_closing_and_reopening() {
        let music = music_dir();
        let db_dir = tempfile::tempdir().unwrap();
        let db_path = db_dir.path().join("nested/library.db");

        {
            let mut lib = Library::open(&db_path).unwrap();
            assert_eq!(lib.schema_version().unwrap(), MIGRATIONS.len() as i64);
            lib.sync_folder(music.path()).unwrap();
        }

        let lib = Library::open(&db_path).unwrap();
        assert_eq!(lib.schema_version().unwrap(), MIGRATIONS.len() as i64);
        assert_eq!(lib.track_count().unwrap(), 2);
    }

    #[test]
    fn a_database_from_a_newer_version_is_rejected() {
        let db_dir = tempfile::tempdir().unwrap();
        let db_path = db_dir.path().join("library.db");
        {
            let conn = Connection::open(&db_path).unwrap();
            conn.execute_batch("PRAGMA user_version = 99;").unwrap();
        }

        let err = Library::open(&db_path).err().unwrap();
        assert!(matches!(err, LibraryError::SchemaTooNew { found: 99, .. }));
    }

    /// A library with known tags, inserted directly (no audio files needed).
    fn library_with_tags() -> Library {
        let lib = Library::open_in_memory().unwrap();
        let rows = [
            ("One More Time", "Daft Punk", "Discovery"),
            ("Harder Better Faster Stronger", "Daft Punk", "Discovery"),
            ("Blue Monday", "New Order", "Power, Corruption & Lies"),
            ("100% Pure", "Some Band", "Greatest Hits"),
            ("Track", "Other Band", "Under_Score"),
        ];
        for (i, (title, artist, album)) in rows.iter().enumerate() {
            let track = Track {
                path: PathBuf::from(format!("/music/{i}.mp3")),
                title: title.to_string(),
                artist: artist.to_string(),
                album: album.to_string(),
                duration: Duration::from_secs(180),
                track_no: Some(i as u32 + 1),
            };
            upsert_track(&lib.conn, &track, 0).unwrap();
        }
        lib
    }

    fn titles(tracks: Vec<LibraryTrack>) -> Vec<String> {
        tracks.into_iter().map(|t| t.track.title).collect()
    }

    #[test]
    fn search_is_case_insensitive_across_title_artist_and_album() {
        let lib = library_with_tags();
        assert_eq!(titles(lib.search("monday").unwrap()), vec!["Blue Monday"]);
        assert_eq!(
            titles(lib.search("DAFT").unwrap()),
            vec!["One More Time", "Harder Better Faster Stronger"]
        );
        assert_eq!(lib.search("discovery").unwrap().len(), 2);
        assert_eq!(titles(lib.search("lies").unwrap()), vec!["Blue Monday"]);
    }

    #[test]
    fn every_word_must_match_something() {
        let lib = library_with_tags();
        assert_eq!(
            titles(lib.search("daft more").unwrap()),
            vec!["One More Time"]
        );
        assert!(lib.search("daft new").unwrap().is_empty());
    }

    #[test]
    fn an_empty_search_returns_everything() {
        let lib = library_with_tags();
        assert_eq!(lib.search("   ").unwrap().len(), 5);
    }

    #[test]
    fn wildcard_characters_are_searched_literally() {
        let lib = library_with_tags();
        assert_eq!(titles(lib.search("%").unwrap()), vec!["100% Pure"]);
        assert_eq!(titles(lib.search("_").unwrap()), vec!["Track"]);
    }

    #[test]
    fn toggling_a_favorite_flips_it() {
        let lib = library_with_tags();
        let id = find(&lib, "Blue Monday").id;

        assert!(!lib.is_favorite(id).unwrap());
        assert!(lib.toggle_favorite(id).unwrap());
        assert!(lib.is_favorite(id).unwrap());
        assert!(!lib.toggle_favorite(id).unwrap());
        assert!(!lib.is_favorite(id).unwrap());
    }

    #[test]
    fn favorites_are_listed_newest_first() {
        let lib = library_with_tags();
        lib.set_favorite(find(&lib, "One More Time").id, true)
            .unwrap();
        lib.set_favorite(find(&lib, "Blue Monday").id, true)
            .unwrap();

        assert_eq!(
            titles(lib.favorites().unwrap()),
            vec!["Blue Monday", "One More Time"]
        );
    }

    #[test]
    fn favoriting_an_unknown_track_is_an_error() {
        let lib = Library::open_in_memory().unwrap();
        let err = lib.set_favorite(12345, true).unwrap_err();
        assert!(matches!(err, LibraryError::TrackNotFound(12345)));
    }

    #[test]
    fn favorites_survive_a_rescan_of_a_changed_file() {
        let dir = music_dir();
        let mut lib = Library::open_in_memory().unwrap();
        lib.sync_folder(dir.path()).unwrap();
        let id = find(&lib, "a").id;
        lib.set_favorite(id, true).unwrap();

        let a = dir.path().join("a.wav");
        write_silent_wav(&a, 2);
        fs::File::options()
            .write(true)
            .open(&a)
            .unwrap()
            .set_modified(SystemTime::now() + Duration::from_secs(10))
            .unwrap();
        lib.sync_folder(dir.path()).unwrap();

        assert!(lib.is_favorite(id).unwrap());
    }

    #[test]
    fn deleting_a_file_removes_its_favorite() {
        let dir = music_dir();
        let mut lib = Library::open_in_memory().unwrap();
        lib.sync_folder(dir.path()).unwrap();
        lib.set_favorite(find(&lib, "b").id, true).unwrap();

        fs::remove_file(dir.path().join("sub/b.wav")).unwrap();
        lib.sync_folder(dir.path()).unwrap();

        assert!(lib.favorites().unwrap().is_empty());
    }

    #[test]
    fn an_older_database_is_upgraded_without_losing_data() {
        let db_dir = tempfile::tempdir().unwrap();
        let db_path = db_dir.path().join("library.db");
        {
            // Build a database the way version 1 of the app would have.
            let conn = Connection::open(&db_path).unwrap();
            conn.execute_batch(MIGRATIONS[0]).unwrap();
            conn.execute_batch(
                "INSERT INTO tracks (path, title, artist, album, duration_ms, track_no, mtime)
                 VALUES ('/old/song.mp3', 'Old Song', 'Old Artist', 'Old Album', 1000, 1, 5);
                 PRAGMA user_version = 1;",
            )
            .unwrap();
        }

        let lib = Library::open(&db_path).unwrap();
        assert_eq!(lib.schema_version().unwrap(), MIGRATIONS.len() as i64);
        assert_eq!(lib.track_count().unwrap(), 1);

        // The new favorites table works on the upgraded database.
        let id = lib.tracks().unwrap()[0].id;
        lib.set_favorite(id, true).unwrap();
        assert!(lib.is_favorite(id).unwrap());
    }
}
