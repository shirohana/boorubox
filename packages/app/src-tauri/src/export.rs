//! Writing the selected originals to a zip file (design D11).
//!
//! The library mutex is held only long enough to read each id's extension,
//! capture time and tags; copying the bytes and building the archive run
//! with it released, so an export cannot hold up a capture or a search for
//! as long as it takes to write. Entries are stored, not deflated (design
//! D11): the archive holds JPEG, PNG and WebP, and deflating bytes that are
//! already compressed costs CPU for no measurable size gain.
//!
//! An entry is what a file manager shows of the export: [`entry_name`] is
//! `yande.re`-style (id first so two entries can never collide, then the
//! image's tags, so a listing reads as something) and [`entry_mtime`] is
//! `captured_at`, so a folder of exports sorts along a time axis. The zip
//! timestamp is a wall-clock time with no zone, which every file manager
//! reads as local; the caller therefore passes its UTC offset and the entry
//! is written in that zone (design D11).

use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use zip::ZipWriter;
use zip::write::SimpleFileOptions;

use crate::error::Result;
use crate::library::{Library, LibraryPaths, SharedLibrary, with_library};
use crate::model::{ExportProgress, ExportReport};
use crate::query::{ID_CHUNK, placeholders};

/// The longest a zip entry name may be, including its extension — the
/// common filesystem/zip limit `entry_name` caps to by dropping whole tags.
const MAX_ENTRY_NAME_BYTES: usize = 255;

/// Write every id in `ids` to a zip at `path`, calling `on_progress` once per
/// id — including the ones left out (design D13) — so the last tick's `done`
/// always equals `total`.
///
/// An id with no row, or whose file is gone from `images/`, is left out of the
/// archive and named in `ExportReport.missing`; every other id is still
/// written (design `export-selected`, "A missing file is reported, not
/// fatal").
pub fn export_zip(
    library: &SharedLibrary,
    ids: &[String],
    path: &Path,
    utc_offset_minutes: i32,
    on_progress: &mut dyn FnMut(ExportProgress),
) -> Result<ExportReport> {
    let (paths, meta) = with_library(library, |library| {
        Ok((library.paths.clone(), image_meta_by_id(library, ids)?))
    })?;

    let total = ids.len() as i64;
    let mut zip = ZipWriter::new(File::create(path)?);

    let mut written = 0i64;
    let mut missing = Vec::new();
    on_progress(ExportProgress { done: 0, total });
    for (index, id) in ids.iter().enumerate() {
        match read_image(&meta, &paths, id)? {
            Some((info, bytes)) => {
                let options = SimpleFileOptions::default()
                    .compression_method(zip::CompressionMethod::Stored)
                    .last_modified_time(entry_mtime(info.captured_at, utc_offset_minutes));
                zip.start_file(entry_name(id, &info.tags, &info.ext), options)
                    .map_err(std::io::Error::from)?;
                zip.write_all(&bytes)?;
                written += 1;
            }
            None => missing.push(id.clone()),
        }
        on_progress(ExportProgress {
            done: index as i64 + 1,
            total,
        });
    }
    zip.finish().map_err(std::io::Error::from)?;

    Ok(ExportReport {
        path: path.display().to_string(),
        written,
        missing,
    })
}

/// What one archive entry needs beyond the bytes themselves: the extension
/// for the file path and entry suffix, `captured_at` for [`entry_mtime`],
/// and the tags for [`entry_name`], in the image's stored order (the same
/// `ORDER BY tags.name` `ingest::load_records` fills `ImageRecord.tags`
/// with).
struct ImageMeta {
    ext: String,
    captured_at: i64,
    tags: Vec<String>,
}

/// Each requested id's extension, capture time and tags, for the ids that
/// still have a row. An id the library no longer knows is left out of the
/// map, which `export_zip` reads exactly as it reads a row whose file is
/// gone: left out and named missing.
///
/// Two chunked queries, not a join per id: the image row and the tag rows
/// are fetched separately, [`ID_CHUNK`] ids per statement — one `IN (…)`
/// built over a whole large selection binds one parameter per id in one
/// statement, which SQLite refuses past its variable limit ("too many SQL
/// variables") once a selection spans a whole large library.
fn image_meta_by_id(library: &Library, ids: &[String]) -> Result<HashMap<String, ImageMeta>> {
    let mut meta = HashMap::with_capacity(ids.len());
    for chunk in ids.chunks(ID_CHUNK) {
        let mut stmt = library.conn.prepare(&format!(
            "SELECT id, ext, captured_at FROM images WHERE id IN ({})",
            placeholders(chunk.len())
        ))?;
        let rows = stmt.query_map(rusqlite::params_from_iter(chunk), |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })?;
        for row in rows {
            let (id, ext, captured_at) = row?;
            meta.insert(
                id,
                ImageMeta {
                    ext,
                    captured_at,
                    tags: Vec::new(),
                },
            );
        }
    }
    for chunk in ids.chunks(ID_CHUNK) {
        let mut stmt = library.conn.prepare(&format!(
            "SELECT image_tags.image_id, tags.name FROM image_tags
             JOIN tags ON tags.id = image_tags.tag_id
             WHERE image_tags.image_id IN ({})
             ORDER BY tags.name",
            placeholders(chunk.len())
        ))?;
        let rows = stmt.query_map(rusqlite::params_from_iter(chunk), |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        for row in rows {
            let (id, tag) = row?;
            if let Some(info) = meta.get_mut(&id) {
                info.tags.push(tag);
            }
        }
    }
    Ok(meta)
}

/// The id's metadata and bytes, if the row and its file are both there.
/// `Ok(None)` is "missing" (spec `export-selected`'s "A missing file is
/// reported, not fatal"): a row the library does not have, or a file gone
/// from `images/`. Any other read failure — a permission error, a device
/// error — is a real error and stops the export rather than being folded
/// into the same "missing" the user sees for an ordinary deleted file.
fn read_image<'a>(
    meta: &'a HashMap<String, ImageMeta>,
    paths: &LibraryPaths,
    id: &str,
) -> Result<Option<(&'a ImageMeta, Vec<u8>)>> {
    let Some(info) = meta.get(id) else {
        return Ok(None);
    };
    match std::fs::read(paths.image_path(id, &info.ext)) {
        Ok(bytes) => Ok(Some((info, bytes))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

/// The zip entry name for one image (owner's Human update on task 6.1): `<id>
/// <tag1> <tag2> ….<ext>`, `yande.re`-style, so the id keeps entries
/// collision-free (two selected images never share one) while the tags make
/// a Finder/Explorer listing of the archive readable without opening the
/// app. `tags` is read in the image's stored order and joined with single
/// spaces; within a tag, `/`, `\` and any control character (including NUL)
/// are replaced with `_` so a tag can never introduce a path separator or a
/// literal control byte into the name.
///
/// The full name is capped at [`MAX_ENTRY_NAME_BYTES`] bytes, including the
/// extension — the common filesystem/zip limit — by dropping whole tags off
/// the end until it fits. A tag is never cut mid-character: every candidate
/// name is built from whole sanitized tags, so a multibyte tag near the cap
/// is either kept whole or dropped whole, never split. No tags (or the `id`
/// and extension alone already at the cap): `<id>.<ext>`, as before.
fn entry_name(id: &str, tags: &[String], ext: &str) -> String {
    let suffix = format!(".{ext}");
    let sanitized: Vec<String> = tags.iter().map(|tag| sanitize_tag(tag)).collect();
    for take in (0..=sanitized.len()).rev() {
        let mut name = String::from(id);
        for tag in &sanitized[..take] {
            name.push(' ');
            name.push_str(tag);
        }
        name.push_str(&suffix);
        if take == 0 || name.len() <= MAX_ENTRY_NAME_BYTES {
            return name;
        }
    }
    unreachable!("the `take == 0` iteration always returns")
}

/// One tag, made safe to sit inside a file name: `/` and `\` would turn it
/// into a directory component, and a control character (NUL included)
/// confuses filesystems and zip tools that assume a printable name.
/// Everything else — including non-ASCII — is kept exactly as stored.
fn sanitize_tag(tag: &str) -> String {
    tag.chars()
        .map(|ch| match ch {
            '/' | '\\' => '_',
            ch if ch.is_control() => '_',
            ch => ch,
        })
        .collect()
}

/// The zip entry's last-modified time for `captured_at` (epoch
/// milliseconds), as wall-clock fields in the zone `utc_offset_minutes`
/// east of UTC. A zip timestamp carries no zone and is shown as local time,
/// so writing UTC fields would show every file off by the offset; the
/// webview passes its own offset, which is the zone the app already renders
/// capture times in. A `captured_at` the DOS timestamp cannot represent —
/// before 1980, after 2107, an invalid instant or offset — falls back to
/// [`zip::DateTime::default()`] (`1980-01-01`) rather than failing the
/// export over one image's date, as D13 keeps a dropped progress tick from
/// failing it.
fn entry_mtime(captured_at: i64, utc_offset_minutes: i32) -> zip::DateTime {
    let seconds = captured_at.div_euclid(1000);
    let zone = time::UtcOffset::from_whole_seconds(utc_offset_minutes * 60);
    time::OffsetDateTime::from_unix_timestamp(seconds)
        .ok()
        .zip(zone.ok())
        .and_then(|(instant, zone)| {
            let local = instant.to_offset(zone);
            zip::DateTime::try_from(time::PrimitiveDateTime::new(local.date(), local.time())).ok()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingest::{IngestInput, store_image};
    use crate::model::ImageSource;
    use crate::tags;

    fn png_bytes(seed: u8) -> Vec<u8> {
        let mut image = image::RgbImage::new(2, 2);
        image.put_pixel(0, 0, image::Rgb([seed, seed, seed]));
        let mut out = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(image)
            .write_to(&mut out, image::ImageFormat::Png)
            .unwrap();
        out.into_inner()
    }

    fn library_with(ids: &[&str]) -> (tempfile::TempDir, Library) {
        let dir = tempfile::tempdir().unwrap();
        let library = Library::open_or_create(dir.path()).unwrap();
        for (index, id) in ids.iter().enumerate() {
            store_image(
                &library,
                IngestInput {
                    id,
                    bytes: &png_bytes(index as u8 + 1),
                    source: ImageSource::Local,
                    source_ref: None,
                    image_url: None,
                    page_url: None,
                    page_title: None,
                    adapter: None,
                    rating: None,
                    tags: &[],
                    captured_at: 0,
                    file_modified_at: None,
                    deleted_at: None,
                },
            )
            .unwrap();
        }
        (dir, library)
    }

    fn shared(library: Library) -> SharedLibrary {
        std::sync::Arc::new(std::sync::Mutex::new(Some(library)))
    }

    fn read_entry(archive: &mut zip::ZipArchive<File>, name: &str) -> Vec<u8> {
        use std::io::Read;
        let mut file = archive.by_name(name).unwrap();
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).unwrap();
        bytes
    }

    #[test]
    fn the_archive_holds_one_stored_entry_per_id_identical_to_the_file() {
        let (dir, library) = library_with(&["a", "b"]);
        let originals: HashMap<&str, Vec<u8>> = ["a", "b"]
            .iter()
            .map(|id| {
                (
                    *id,
                    std::fs::read(library.paths.image_path(id, "png")).unwrap(),
                )
            })
            .collect();
        let shared = shared(library);
        let out = dir.path().join("out.zip");

        let report = export_zip(
            &shared,
            &["a".to_string(), "b".to_string()],
            &out,
            0,
            &mut |_| {},
        )
        .unwrap();

        assert_eq!(report.written, 2);
        assert!(report.missing.is_empty());

        let mut archive = zip::ZipArchive::new(File::open(&out).unwrap()).unwrap();
        assert_eq!(archive.len(), 2);
        for id in ["a", "b"] {
            let name = format!("{id}.png");
            assert_eq!(
                read_entry(&mut archive, &name),
                originals[id],
                "entry for {id} must match the file under images/"
            );
            assert_eq!(
                archive.by_name(&name).unwrap().compression(),
                zip::CompressionMethod::Stored,
                "entries must be stored, not deflated (design D11)"
            );
        }
    }

    #[test]
    fn a_file_missing_since_the_query_is_left_out_and_named_while_the_rest_are_written() {
        let (dir, library) = library_with(&["a", "gone"]);
        std::fs::remove_file(library.paths.image_path("gone", "png")).unwrap();
        let shared = shared(library);
        let out = dir.path().join("out.zip");

        let report = export_zip(
            &shared,
            &["a".to_string(), "gone".to_string()],
            &out,
            0,
            &mut |_| {},
        )
        .unwrap();

        assert_eq!(report.written, 1);
        assert_eq!(report.missing, vec!["gone".to_string()]);

        let mut archive = zip::ZipArchive::new(File::open(&out).unwrap()).unwrap();
        assert_eq!(archive.len(), 1);
        assert!(archive.by_name("a.png").is_ok());
    }

    #[test]
    fn the_last_progress_tick_reports_the_run_finished() {
        let (dir, library) = library_with(&["a", "b", "c"]);
        let shared = shared(library);
        let out = dir.path().join("out.zip");
        let ticks = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let seen = ticks.clone();

        export_zip(
            &shared,
            &["a".to_string(), "b".to_string(), "c".to_string()],
            &out,
            0,
            &mut |progress| seen.lock().unwrap().push(progress),
        )
        .unwrap();

        let ticks = ticks.lock().unwrap().clone();
        assert_eq!(ticks.first(), Some(&ExportProgress { done: 0, total: 3 }));
        assert_eq!(ticks.last(), Some(&ExportProgress { done: 3, total: 3 }));
    }

    #[test]
    fn an_id_the_library_never_had_is_reported_missing_too() {
        let (dir, library) = library_with(&["a"]);
        let shared = shared(library);
        let out = dir.path().join("out.zip");

        let report = export_zip(
            &shared,
            &["a".to_string(), "no-such-id".to_string()],
            &out,
            0,
            &mut |_| {},
        )
        .unwrap();

        assert_eq!(report.written, 1);
        assert_eq!(report.missing, vec!["no-such-id".to_string()]);
    }

    /// A selection this large used to build one `SELECT … WHERE id IN (…)`
    /// with one bound parameter per id, which SQLite refuses past its
    /// variable limit ("too many SQL variables"); this pins that both
    /// chunked queries still read every id's row and tags.
    #[test]
    fn image_meta_is_read_for_every_id_past_the_sqlite_variable_chunk_size() {
        let dir = tempfile::tempdir().unwrap();
        let library = Library::open_or_create(dir.path()).unwrap();
        let ids: Vec<String> = (0..2500).map(|index| format!("img-{index}")).collect();
        for id in &ids {
            library
                .conn
                .execute(
                    "INSERT INTO images (id, ext, mime, size, width, height, source,
                                         captured_at, created_at, updated_at)
                     VALUES (?1, 'png', 'image/png', 1, 1, 1, 'local', 0, 0, 0)",
                    [id],
                )
                .unwrap();
        }
        tags::add_tags(
            &library.conn,
            &ids[2000],
            &["straddles_the_chunk".to_string()],
        )
        .unwrap();

        let meta = image_meta_by_id(&library, &ids).unwrap();

        assert_eq!(meta.len(), 2500);
        assert!(ids.iter().all(|id| meta.get(id).unwrap().ext == "png"));
        assert_eq!(
            meta[&ids[2000]].tags,
            vec!["straddles_the_chunk".to_string()],
        );
    }

    /// `read_image` must not fold a real read error into "missing": only a
    /// file genuinely gone (`NotFound`) is reported that way, per the
    /// `export-selected` spec's "A missing file is reported, not fatal" —
    /// which is a claim about a deleted file, not about a folder standing
    /// where a file belongs.
    #[test]
    fn a_read_failure_that_is_not_a_missing_file_stops_the_export_as_an_error() {
        let (dir, library) = library_with(&["a"]);
        let path = library.paths.image_path("a", "png");
        std::fs::remove_file(&path).unwrap();
        std::fs::create_dir(&path).unwrap();
        let shared = shared(library);
        let out = dir.path().join("out.zip");

        let error = export_zip(&shared, &["a".to_string()], &out, 0, &mut |_| {}).unwrap_err();

        assert!(
            matches!(error, crate::error::AppError::Io(_)),
            "a directory where a file belongs must surface as an error, not a missing file: {error:?}"
        );
    }

    #[test]
    fn an_untagged_image_keeps_the_id_only_name() {
        assert_eq!(entry_name("abc123", &[], "png"), "abc123.png");
    }

    #[test]
    fn a_few_tags_join_the_id_with_single_spaces() {
        let tags = vec!["animal_ears".to_string(), "kitsune".to_string()];
        assert_eq!(
            entry_name("abc123", &tags, "jpg"),
            "abc123 animal_ears kitsune.jpg",
        );
    }

    #[test]
    fn a_tag_with_a_slash_or_backslash_cannot_open_a_directory() {
        let tags = vec!["a/b".to_string(), "c\\d".to_string()];
        assert_eq!(entry_name("id", &tags, "png"), "id a_b c_d.png");
    }

    #[test]
    fn a_control_character_including_nul_is_replaced() {
        let tags = vec!["odd\u{0}tag\nname".to_string()];
        assert_eq!(entry_name("id", &tags, "png"), "id odd_tag_name.png");
    }

    /// Two tags that together push the name past the 255-byte cap: the whole
    /// second tag is dropped, the first stays intact, and the id always
    /// leads (so two selected images can never collide, cap or no cap).
    #[test]
    fn tags_are_dropped_whole_from_the_end_past_the_255_byte_cap() {
        let tag1 = "x".repeat(240);
        let tag2 = "y".repeat(240);
        let name = entry_name("m", &[tag1.clone(), tag2], "png");

        assert_eq!(name, format!("m {tag1}.png"));
        assert!(name.len() <= MAX_ENTRY_NAME_BYTES);
    }

    /// The dropped tag is a multibyte one sized so that including it would
    /// cross the byte cap mid-run; dropping the whole tag rather than
    /// truncating it means the result is never cut inside a character.
    #[test]
    fn a_multibyte_tag_near_the_cap_is_kept_whole_or_dropped_whole() {
        let ascii_tag = "x".repeat(240); // leaves ~9 bytes of budget
        let multibyte_tag = "あ".repeat(4); // 12 bytes, three-byte chars
        let name = entry_name("m", &[ascii_tag.clone(), multibyte_tag.clone()], "png");

        assert_eq!(
            name,
            format!("m {ascii_tag}.png"),
            "the multibyte tag must be dropped whole, not truncated mid-character",
        );
        assert!(name.len() <= MAX_ENTRY_NAME_BYTES);
        assert!(!name.contains(&multibyte_tag));
    }

    #[test]
    fn entry_mtime_carries_captured_at_into_the_zip_date_time() {
        // 2024-03-02 08:15:30 UTC, epoch milliseconds.
        let captured_at = 1_709_367_330_000i64;

        let mtime = entry_mtime(captured_at, 0);

        assert_eq!(
            (
                mtime.year(),
                mtime.month(),
                mtime.day(),
                mtime.hour(),
                mtime.minute(),
            ),
            (2024, 3, 2, 8, 15),
        );
    }

    /// The same instant written for a caller eight hours east of UTC: the
    /// fields are that zone's wall clock, which is what a file manager shows.
    #[test]
    fn entry_mtime_is_written_in_the_callers_zone() {
        let mtime = entry_mtime(1_709_367_330_000, 8 * 60);

        assert_eq!((mtime.day(), mtime.hour(), mtime.minute()), (2, 16, 15));
    }

    /// An offset no zone has (past ±24h) falls back like a bad instant does.
    #[test]
    fn an_impossible_offset_falls_back_to_the_crate_default() {
        assert_eq!(
            entry_mtime(1_709_367_330_000, 30 * 60),
            zip::DateTime::default()
        );
    }

    /// Before the zip format's 1980 floor, the timestamp cannot be
    /// represented; the entry falls back to the crate's default rather than
    /// failing the export over one image's date.
    #[test]
    fn a_captured_at_before_1980_falls_back_to_the_crate_default() {
        assert_eq!(entry_mtime(0, 0), zip::DateTime::default());
    }
}
