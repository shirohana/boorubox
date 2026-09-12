//! The single door every image enters the library by (design D4): decode, write
//! `inbox/<id>.part`, fsync, rename into `images/`, then insert the rows in one
//! transaction. Extension captures, local import and legacy-bundle import all
//! call `store_image`; none of them writes to `images/` itself.

use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use rusqlite::{Connection, Row, params, params_from_iter};

use crate::db;
use crate::error::{AppError, Result};
use crate::library::Library;
use crate::model::{ImageRecord, ImageSource, PostRef, SiteAdapterRecord};
use crate::rules;
use crate::tags;

/// Everything a caller knows about an image before it has a row. `id` is the
/// caller's UUID, and delivery is idempotent on it.
pub struct IngestInput<'a> {
    pub id: &'a str,
    pub bytes: &'a [u8],
    pub source: ImageSource,
    pub source_ref: Option<&'a str>,
    pub image_url: Option<&'a str>,
    pub page_url: Option<&'a str>,
    pub page_title: Option<&'a str>,
    /// What the caller's site adapter extracted. Stored as it arrived
    /// (design D11); also matched against the enabled auto-tag rules
    /// (`auto-tag-rules` design D5, D7), except for `source = LegacyBundle`.
    pub adapter: Option<&'a SiteAdapterRecord>,
    pub rating: Option<&'a str>,
    pub tags: &'a [String],
    pub captured_at: i64,
    /// The file's own modification time; `None` for anything that did not come
    /// from a file (design D11, `browse-polish`). The local import path is the
    /// only caller that passes `Some`.
    pub file_modified_at: Option<i64>,
    /// When the image was already deleted at the moment it arrived here
    /// (`legacy-bundle-import` design D3): a trashed bundle row goes through
    /// this same door rather than a second insert. Every caller but the
    /// bundle importer passes `None`.
    pub deleted_at: Option<i64>,
}

/// Whether the call stored the image or found it already there. The HTTP layer
/// turns these into 201 and 200 respectively.
#[derive(Debug)]
pub enum Ingested {
    Created(ImageRecord),
    Existing(ImageRecord),
}

impl Ingested {
    pub fn record(&self) -> &ImageRecord {
        match self {
            Ingested::Created(record) | Ingested::Existing(record) => record,
        }
    }
}

/// The columns `row_to_record` reads, in the order it reads them. Any SELECT
/// feeding that function must use this list.
pub const IMAGE_COLUMNS: &str = "id, ext, mime, size, width, height, source, source_ref, \
     image_url, page_url, page_title, adapter_json, rating, captured_at, created_at, updated_at, \
     deleted_at, missing, file_modified_at";

struct Decoded {
    width: u32,
    height: u32,
    ext: &'static str,
    mime: &'static str,
}

/// Store `input` and return as soon as its row exists.
///
/// The thumbnail is deliberately not generated here: this whole function runs
/// with the library mutex held, and the encode is the largest piece of that
/// hold (design D13). Every caller calls `thumbs::warm_thumbnail` on the record
/// once it has let the library go — a new caller that forgets leaves images
/// whose thumbnail is only built the first time the grid asks for one.
///
/// The sidecar is written after the row's own transaction commits, never
/// inside it (`library-sidecars` design D4): a failed commit must never leave
/// a sidecar for a row that does not exist, and a bulk edit holding the
/// library for thousands of file writes is the window this change must not
/// widen. The write happens on both outcomes, `Created` and `Existing` alike
/// — a redelivered capture that stored its row but never reached its sidecar
/// (this same call, failing on the line below, on a prior delivery) writes the
/// missing file on retry rather than reporting success with the hole still
/// open. A sidecar write failure fails this call even though the row is
/// already committed, which is what makes that retry the way back.
pub fn store_image(library: &Library, input: IngestInput) -> Result<Ingested> {
    if let Some(existing) = load_record(&library.conn, input.id)? {
        write_sidecar(library, &existing)?;
        return Ok(Ingested::Existing(existing));
    }

    let decoded = decode(input.bytes)?;
    let path = library.paths.image_path(input.id, decoded.ext);
    write_through_inbox(library, input.id, input.bytes, &path)?;

    let ingested = match insert_rows(library, &input, &decoded) {
        Ok(ingested) => ingested,
        Err(error) => {
            let _ = std::fs::remove_file(&path);
            return Err(error);
        }
    };
    write_sidecar(library, ingested.record())?;
    Ok(ingested)
}

/// From the record this call already holds, not a re-read of the row it just
/// wrote: `sidecar::write_for`'s three statements per image are pure cost on
/// the one door 25,000 bundle-imported images come through, and a sidecar
/// built from the record the caller is handed back cannot disagree with it.
fn write_sidecar(library: &Library, record: &ImageRecord) -> Result<()> {
    crate::sidecar::write(&library.paths, &crate::sidecar::Sidecar::from(record))
}

/// Undecodable bytes are an error before anything is written, so a rejected
/// capture leaves no file, no part file and no row.
fn decode(bytes: &[u8]) -> Result<Decoded> {
    let format = image::guess_format(bytes)?;
    let image = image::load_from_memory_with_format(bytes, format)?;
    Ok(Decoded {
        width: image.width(),
        height: image.height(),
        ext: format.extensions_str().first().copied().unwrap_or("bin"),
        mime: format.to_mime_type(),
    })
}

/// Write, fsync, rename. A crash before the rename leaves at most a stray
/// `.part`, which `Library::open_or_create` sweeps; a crash after it leaves a
/// file with no row, which the missing/orphan pass can see.
fn write_through_inbox(library: &Library, id: &str, bytes: &[u8], dest: &Path) -> Result<()> {
    let part = library.paths.part_path(id);
    let write = || -> Result<()> {
        let mut file = File::create(&part)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        Ok(())
    };
    if let Err(error) = write() {
        let _ = std::fs::remove_file(&part);
        return Err(error);
    }
    // The destination's bucket (design D1, D3): created on demand rather than
    // up front, so an empty library never pays for 65,536 directories it may
    // never fill.
    if let Some(bucket) = dest.parent()
        && let Err(error) = std::fs::create_dir_all(bucket)
    {
        let _ = std::fs::remove_file(&part);
        return Err(error.into());
    }
    std::fs::rename(&part, dest).map_err(|error| {
        let _ = std::fs::remove_file(&part);
        AppError::Io(error)
    })
}

fn insert_rows(library: &Library, input: &IngestInput, decoded: &Decoded) -> Result<Ingested> {
    // `unchecked_transaction` because `store_image` takes `&Library`: the
    // exclusive access rusqlite normally wants is provided one level up, by the
    // mutex around the single connection (design D1).
    let tx = library.conn.unchecked_transaction()?;
    let now = db::now_ms();
    let adapter_json = input
        .adapter
        .map(serde_json::to_string)
        .transpose()
        .map_err(|error| {
            AppError::BadRequest(format!("adapter record cannot be stored: {error}"))
        })?;
    let (final_tags, final_rating) = resolve_tags_and_rating(&tx, input)?;
    let inserted = tx.execute(
        "INSERT INTO images (id, ext, mime, size, width, height, source, source_ref, image_url,
                             page_url, page_title, adapter_json, rating, captured_at, created_at,
                             updated_at, file_modified_at, deleted_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)
         ON CONFLICT (id) DO NOTHING",
        params![
            input.id,
            decoded.ext,
            decoded.mime,
            input.bytes.len() as i64,
            decoded.width,
            decoded.height,
            input.source,
            input.source_ref,
            input.image_url,
            input.page_url,
            input.page_title,
            adapter_json,
            final_rating,
            input.captured_at,
            now,
            now,
            input.file_modified_at,
            input.deleted_at,
        ],
    )?;

    if inserted == 0 {
        // Unreachable while D1 holds: the caller above already found no row, and
        // one mutex-guarded connection means nobody inserted in between. Give a
        // second writer here and it also orphans the file just renamed in.
        drop(tx);
        return Ok(Ingested::Existing(require_record(&library.conn, input.id)?));
    }

    for tag in &final_tags {
        tags::link_tag(&tx, input.id, tag)?;
    }
    tx.commit()?;

    Ok(Ingested::Created(require_record(&library.conn, input.id)?))
}

/// What actually gets stored: `input.tags` unioned with the enabled rules'
/// matches against the title and the adapter record (`auto-tag-rules` design
/// D5, D7), except for `LegacyBundle` — `legacy-bundle-import` records
/// "whatever the bundle carries is what is stored", and re-deriving tags for
/// images that already carry the old library's would fight that and
/// double-apply a rule that has since changed. The combined list then goes
/// through `tags::split_rating` unconditionally, the same rule the tag editor
/// applies (design D8), so a `rating:e` a rule names — or one a caller simply
/// hands in through `input.tags` — never becomes a literal tag either way.
/// `input.rating`, what the source itself supplied, wins over anything a rule
/// extracted (design D8's "a rule SHALL NOT overwrite a rating the source
/// supplied").
fn resolve_tags_and_rating(
    conn: &Connection,
    input: &IngestInput,
) -> Result<(Vec<String>, Option<String>)> {
    let mut candidate: Vec<String> = input.tags.to_vec();
    if input.source != ImageSource::LegacyBundle {
        let enabled = rules::enabled_rules(conn)?;
        let haystacks = rules::haystacks(input.page_title, input.adapter);
        // Appended, not unioned: `split_rating` below is the one deduper of
        // this list, and it drops repeats keeping the first occurrence — so a
        // tag the source supplied and a rule also names is stored once, in the
        // source's position, without a second dedupe spelling it here.
        candidate.extend(rules::auto_tags(&enabled, &haystacks));
    }
    let (final_tags, extracted_rating) = tags::split_rating(&candidate);
    let final_rating = input.rating.map(str::to_string).or(extracted_rating);
    Ok((final_tags, final_rating))
}

/// Map a row selected with [`IMAGE_COLUMNS`]. `tags` comes back empty: tags are
/// a second query, so that a hundred records cost two statements, not a hundred.
pub fn row_to_record(row: &Row) -> rusqlite::Result<ImageRecord> {
    let id: String = row.get(0)?;
    let ext: String = row.get(1)?;
    let file = crate::library::LibraryPaths::relative_image_path(&id, &ext);
    Ok(ImageRecord {
        id,
        ext,
        file,
        mime: row.get(2)?,
        size: row.get(3)?,
        width: row.get(4)?,
        height: row.get(5)?,
        source: row.get(6)?,
        source_ref: row.get(7)?,
        image_url: row.get(8)?,
        page_url: row.get(9)?,
        page_title: row.get(10)?,
        // A record that will not parse reads as none rather than failing the
        // row: the column is a document written by a client, and one bad
        // document must not take a whole page of the grid down with it.
        adapter: row
            .get::<_, Option<String>>(11)?
            .and_then(|json| serde_json::from_str(&json).ok()),
        rating: row.get(12)?,
        tags: Vec::new(),
        captured_at: row.get(13)?,
        created_at: row.get(14)?,
        updated_at: row.get(15)?,
        deleted_at: row.get(16)?,
        missing: row.get(17)?,
        file_modified_at: row.get(18)?,
        posts: Vec::new(),
    })
}

pub fn load_record(conn: &Connection, id: &str) -> Result<Option<ImageRecord>> {
    Ok(load_records(conn, std::slice::from_ref(&id.to_string()))?.pop())
}

/// Load records for `ids`, in the order given; ids with no row are dropped.
///
/// Three statements however many ids are asked for, never one per image: this
/// is also what `booru-upload` design D5 relies on for `ImageRecord.posts` —
/// the `posts` table is joined in here alongside the existing tags fill, so
/// every caller of `load_record`/`load_records` (a search page and a single-
/// image read alike) gets it for free.
pub fn load_records(conn: &Connection, ids: &[String]) -> Result<Vec<ImageRecord>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders = vec!["?"; ids.len()].join(", ");

    let mut stmt = conn.prepare(&format!(
        "SELECT {IMAGE_COLUMNS} FROM images WHERE id IN ({placeholders})"
    ))?;
    let mut by_id: HashMap<String, ImageRecord> = stmt
        .query_map(params_from_iter(ids), row_to_record)?
        .map(|record| record.map(|record| (record.id.clone(), record)))
        .collect::<rusqlite::Result<_>>()?;

    let mut stmt = conn.prepare(&format!(
        "SELECT image_tags.image_id, tags.name FROM image_tags
         JOIN tags ON tags.id = image_tags.tag_id
         WHERE image_tags.image_id IN ({placeholders})
         ORDER BY tags.name"
    ))?;
    let mut rows = stmt.query(params_from_iter(ids))?;
    while let Some(row) = rows.next()? {
        let image_id: String = row.get(0)?;
        if let Some(record) = by_id.get_mut(&image_id) {
            record.tags.push(row.get(1)?);
        }
    }

    // `remote_id`/`posted_at IS NOT NULL`: both columns are nullable from
    // Phase 1's empty table (design D1) and `booru::posts::record` — the only
    // writer — always sets both, so a row missing either came from outside
    // this app. Reading one into `PostRef`'s non-optional fields would fail
    // the whole query, so every page of the grid holding such a row would
    // fail to load; dropping the row costs one label.
    let mut stmt = conn.prepare(&format!(
        "SELECT image_id, site, remote_id, posted_at FROM posts
         WHERE image_id IN ({placeholders})
           AND remote_id IS NOT NULL AND posted_at IS NOT NULL
         ORDER BY site"
    ))?;
    let mut rows = stmt.query(params_from_iter(ids))?;
    while let Some(row) = rows.next()? {
        let image_id: String = row.get(0)?;
        if let Some(record) = by_id.get_mut(&image_id) {
            record.posts.push(PostRef {
                site: row.get(1)?,
                remote_id: row.get(2)?,
                posted_at: row.get(3)?,
            });
        }
    }

    Ok(ids.iter().filter_map(|id| by_id.remove(id)).collect())
}

/// The record, or `NotFound`. What every command answering with one row uses.
pub fn require_record(conn: &Connection, id: &str) -> Result<ImageRecord> {
    load_record(conn, id)?.ok_or_else(|| AppError::NotFound(format!("image {id}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png_bytes(width: u32, height: u32) -> Vec<u8> {
        let image = image::DynamicImage::ImageRgba8(image::RgbaImage::new(width, height));
        let mut out = std::io::Cursor::new(Vec::new());
        image.write_to(&mut out, image::ImageFormat::Png).unwrap();
        out.into_inner()
    }

    fn input<'a>(id: &'a str, bytes: &'a [u8], tags: &'a [String]) -> IngestInput<'a> {
        IngestInput {
            id,
            bytes,
            source: ImageSource::Extension,
            source_ref: Some("danbooru"),
            image_url: Some("https://example.test/i.png"),
            page_url: Some("https://example.test/p"),
            page_title: Some("a page"),
            adapter: None,
            rating: Some("s"),
            tags,
            captured_at: 1_700_000_000_000,
            file_modified_at: None,
            deleted_at: None,
        }
    }

    fn entry_count(dir: &Path) -> usize {
        std::fs::read_dir(dir).unwrap().count()
    }

    /// Files anywhere under `dir`, buckets descended into — `entry_count`
    /// above only sees the top level, which a sharded id's bucket directory
    /// satisfies at "one entry" whether it holds one file or several.
    fn recursive_file_count(dir: &Path) -> usize {
        std::fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .map(|path| {
                if path.is_dir() {
                    recursive_file_count(&path)
                } else {
                    1
                }
            })
            .sum()
    }

    fn library() -> (tempfile::TempDir, Library) {
        let dir = tempfile::tempdir().unwrap();
        let library = Library::open_or_create(dir.path()).unwrap();
        (dir, library)
    }

    /// `library-sidecars` task 1.5: a stored image has its sidecar beside its
    /// file with the row's tags and rating.
    #[test]
    fn storing_an_image_writes_its_sidecar_with_the_right_tags_and_rating() {
        let (_dir, library) = library();
        let bytes = png_bytes(4, 7);
        let tags = vec!["blue_sky".to_string(), "1girl".to_string()];

        store_image(&library, input("id-1", &bytes, &tags)).unwrap();

        let sidecar = crate::sidecar::read(&crate::sidecar::path(&library.paths, "id-1")).unwrap();
        assert_eq!(
            sidecar.tags,
            vec!["1girl".to_string(), "blue_sky".to_string()]
        );
        assert_eq!(sidecar.rating.as_deref(), Some("s"));
    }

    /// `library-sidecars` task 1.5: deleting the sidecar and redelivering the
    /// same id writes it again and still reports `Existing`, with no second
    /// image file created.
    #[test]
    fn redelivering_a_stored_id_after_its_sidecar_was_deleted_writes_it_again() {
        let (_dir, library) = library();
        let bytes = png_bytes(4, 7);
        let tags: Vec<String> = Vec::new();
        store_image(&library, input("id-1", &bytes, &tags)).unwrap();
        let sidecar_path = crate::sidecar::path(&library.paths, "id-1");
        std::fs::remove_file(&sidecar_path).unwrap();

        let ingested = store_image(&library, input("id-1", &bytes, &tags)).unwrap();

        assert!(matches!(ingested, Ingested::Existing(_)));
        assert!(sidecar_path.is_file());
        assert_eq!(
            recursive_file_count(&library.paths.images_dir()),
            2,
            "the one image file and its one rewritten sidecar, no second of either"
        );
    }

    /// `library-sidecars` design D4: the row is already committed by the time
    /// the sidecar is written, so a sidecar write failure still fails the
    /// call — the caller's non-2xx is what makes a retry (and the repair
    /// above) reachable at all.
    #[test]
    fn a_sidecar_write_failure_fails_the_call_even_though_the_row_is_committed() {
        let (_dir, library) = library();
        let bytes = png_bytes(4, 7);
        let tags: Vec<String> = Vec::new();
        let sidecar_path = crate::sidecar::path(&library.paths, "id-1");
        // Occupy the sidecar's own destination with a directory, so the
        // rename `sidecar::write` ends with cannot land.
        std::fs::create_dir_all(&sidecar_path).unwrap();

        let error = store_image(&library, input("id-1", &bytes, &tags)).unwrap_err();

        assert!(
            matches!(error, AppError::Io(_)),
            "unexpected error: {error}"
        );
        assert!(
            load_record(&library.conn, "id-1").unwrap().is_some(),
            "the row still landed"
        );
    }

    #[test]
    fn stores_the_file_and_the_row() {
        let (_dir, library) = library();
        let bytes = png_bytes(4, 7);
        let tags = vec!["blue_sky".to_string(), "1girl".to_string()];

        let ingested = store_image(&library, input("id-1", &bytes, &tags)).unwrap();

        let record = match ingested {
            Ingested::Created(record) => record,
            Ingested::Existing(_) => panic!("first ingest reported Existing"),
        };
        assert_eq!(record.ext, "png");
        assert_eq!(record.mime, "image/png");
        assert_eq!(record.width, 4);
        assert_eq!(record.height, 7);
        assert_eq!(record.size, bytes.len() as i64);
        assert_eq!(record.source, ImageSource::Extension);
        assert_eq!(
            record.tags,
            vec!["1girl".to_string(), "blue_sky".to_string()]
        );
        assert!(library.paths.image_path("id-1", "png").is_file());
        assert_eq!(entry_count(&library.paths.inbox_dir()), 0);
        assert_eq!(library.image_count().unwrap(), 1);
    }

    #[test]
    fn the_same_id_twice_stores_one_file_and_one_row() {
        let (_dir, library) = library();
        let first = png_bytes(4, 7);
        let second = png_bytes(9, 9);
        let tags = vec!["blue_sky".to_string()];

        store_image(&library, input("id-1", &first, &tags)).unwrap();
        let again = store_image(&library, input("id-1", &second, &tags)).unwrap();

        assert!(matches!(again, Ingested::Existing(_)));
        assert_eq!(
            again.record().width,
            4,
            "the retry must not replace the stored image"
        );
        assert!(library.paths.image_path("id-1", "png").is_file());
        assert_eq!(
            recursive_file_count(&library.paths.images_dir()),
            2,
            "no second image or sidecar file anywhere under images/, not just at the top \
             level — one image file and its one sidecar (`library-sidecars` design D1)"
        );
        assert_eq!(library.image_count().unwrap(), 1);
    }

    #[test]
    fn undecodable_bytes_leave_nothing_behind() {
        let (_dir, library) = library();
        let tags: Vec<String> = Vec::new();

        let error =
            store_image(&library, input("id-1", b"not an image at all", &tags)).unwrap_err();

        assert!(
            matches!(error, AppError::Decode(_)),
            "unexpected error: {error}"
        );
        assert_eq!(entry_count(&library.paths.images_dir()), 0);
        assert_eq!(entry_count(&library.paths.inbox_dir()), 0);
        assert_eq!(library.image_count().unwrap(), 0);
    }

    #[test]
    fn the_adapter_record_is_stored_and_read_back_unchanged() {
        let (_dir, library) = library();
        let bytes = png_bytes(2, 2);
        let tags: Vec<String> = Vec::new();
        let adapter = SiteAdapterRecord {
            site: "x".to_string(),
            fields: serde_json::json!({
                "handle": "alice",
                "postUrl": "https://x.com/alice/status/1",
                "aliases": ["a", "b"],
            }),
        };
        let mut input = input("id-1", &bytes, &tags);
        input.adapter = Some(&adapter);

        store_image(&library, input).unwrap();
        let record = load_record(&library.conn, "id-1").unwrap().unwrap();

        assert_eq!(record.adapter, Some(adapter));
    }

    #[test]
    fn an_image_stored_without_an_adapter_record_reads_back_with_none() {
        let (_dir, library) = library();
        let bytes = png_bytes(2, 2);
        let tags: Vec<String> = Vec::new();

        store_image(&library, input("id-1", &bytes, &tags)).unwrap();
        let record = load_record(&library.conn, "id-1").unwrap().unwrap();

        assert_eq!(record.adapter, None);
    }

    #[test]
    fn a_row_whose_adapter_record_will_not_parse_still_loads() {
        let (_dir, library) = library();
        let bytes = png_bytes(2, 2);
        let tags: Vec<String> = Vec::new();
        store_image(&library, input("id-1", &bytes, &tags)).unwrap();
        library
            .conn
            .execute(
                "UPDATE images SET adapter_json = 'not json' WHERE id = 'id-1'",
                [],
            )
            .unwrap();

        let record = load_record(&library.conn, "id-1").unwrap().unwrap();

        assert_eq!(record.adapter, None);
        assert_eq!(record.id, "id-1");
    }

    /// Design D11: the column is appended to `IMAGE_COLUMNS`, not slotted in
    /// the middle, so this is the guarantee that a record round-trips whether
    /// or not it carries a file's modification time.
    #[test]
    fn file_modified_at_round_trips_present_and_absent() {
        let (_dir, library) = library();
        let bytes = png_bytes(2, 2);
        let tags: Vec<String> = Vec::new();

        let mut with_file = input("id-1", &bytes, &tags);
        with_file.file_modified_at = Some(1_600_000_000_000);
        store_image(&library, with_file).unwrap();
        let record = load_record(&library.conn, "id-1").unwrap().unwrap();
        assert_eq!(record.file_modified_at, Some(1_600_000_000_000));

        store_image(&library, input("id-2", &bytes, &tags)).unwrap();
        let record = load_record(&library.conn, "id-2").unwrap().unwrap();
        assert_eq!(record.file_modified_at, None);
    }

    #[test]
    fn load_records_keeps_the_order_it_was_asked_for() {
        let (_dir, library) = library();
        let bytes = png_bytes(2, 2);
        let tags: Vec<String> = Vec::new();
        for id in ["a", "b", "c"] {
            store_image(&library, input(id, &bytes, &tags)).unwrap();
        }

        let ids = vec!["c".to_string(), "a".to_string(), "missing".to_string()];
        let records = load_records(&library.conn, &ids).unwrap();

        let got: Vec<&str> = records.iter().map(|record| record.id.as_str()).collect();
        assert_eq!(got, vec!["c", "a"]);
    }

    /// `booru-upload` task 1.6: `load_records` — the one function a search
    /// page and a single-image read both go through — fills `posts` the same
    /// way it already fills `tags`, in one extra statement regardless of how
    /// many images are asked for.
    #[test]
    fn load_records_fills_posts_alongside_tags() {
        let (_dir, library) = library();
        let bytes = png_bytes(2, 2);
        let tags: Vec<String> = Vec::new();
        store_image(&library, input("posted", &bytes, &tags)).unwrap();
        store_image(&library, input("unposted", &bytes, &tags)).unwrap();
        crate::booru::posts::record(
            &library,
            "posted",
            &crate::model::PostRef {
                site: "danbooru".to_string(),
                remote_id: "42".to_string(),
                posted_at: 1_700_000_000_000,
            },
        )
        .unwrap();

        let ids = vec!["posted".to_string(), "unposted".to_string()];
        let records = load_records(&library.conn, &ids).unwrap();

        assert_eq!(
            records[0].posts,
            vec![crate::model::PostRef {
                site: "danbooru".to_string(),
                remote_id: "42".to_string(),
                posted_at: 1_700_000_000_000,
            }]
        );
        assert!(records[1].posts.is_empty());
    }

    /// A `posts` row with a null column is one this app never wrote (only
    /// `booru::posts::record` writes here, and it sets both). It must cost
    /// its own label and nothing else: reading a null into `PostRef` would
    /// fail the query, and with it every page of the grid the row appears on.
    #[test]
    fn a_posts_row_with_a_null_column_is_skipped_rather_than_failing_the_read() {
        let (_dir, library) = library();
        let bytes = png_bytes(2, 2);
        let tags: Vec<String> = Vec::new();
        store_image(&library, input("posted", &bytes, &tags)).unwrap();
        library
            .conn
            .execute(
                "INSERT INTO posts (image_id, site, remote_id, posted_at)
                 VALUES ('posted', 'danbooru', '42', NULL)",
                [],
            )
            .unwrap();

        let record = require_record(&library.conn, "posted").unwrap();

        assert!(record.posts.is_empty());
    }

    /// `auto-tag-rules` task 2.2: `insert_rows` splits a `rating:` token out of
    /// `input.tags` unconditionally, the same rule the tag editor applies —
    /// not only when a rule adds one.
    #[test]
    fn a_fresh_image_whose_input_tags_include_a_rating_token_is_stored_rated_with_no_such_tag() {
        let (_dir, library) = library();
        let bytes = png_bytes(2, 2);
        let tags = vec!["cat".to_string(), "rating:s".to_string()];

        let ingested = store_image(
            &library,
            IngestInput {
                rating: None,
                tags: &tags,
                ..input("id-1", &bytes, &[])
            },
        )
        .unwrap();

        let record = ingested.record();
        assert_eq!(record.tags, vec!["cat".to_string()]);
        assert_eq!(record.rating.as_deref(), Some("s"));
    }

    /// `auto-tag-rules` task 2.4: an enabled rule adds its tags to a capture
    /// whose title or adapter fields it matches (spec `auto-tag-rules`, "A
    /// capture arrives tagged").
    #[test]
    fn a_capture_matching_a_rule_is_stored_with_its_tags() {
        let (_dir, library) = library();
        crate::rules::upsert(
            &library,
            &crate::model::RuleInput {
                id: None,
                name: "pixiv".to_string(),
                pattern: "pixiv".to_string(),
                is_regex: false,
                tags: vec!["pixiv".to_string()],
                enabled: true,
            },
        )
        .unwrap();
        let bytes = png_bytes(2, 2);

        let ingested = store_image(
            &library,
            IngestInput {
                page_title: Some("a pixiv piece"),
                rating: None,
                tags: &[],
                ..input("id-1", &bytes, &[])
            },
        )
        .unwrap();

        assert_eq!(ingested.record().tags, vec!["pixiv".to_string()]);
    }

    /// A local import matches rules against its filename, which `import.rs`
    /// already sets as the title (design D5).
    #[test]
    fn a_local_import_matching_on_its_filename_is_stored_with_the_rules_tags() {
        let (_dir, library) = library();
        crate::rules::upsert(
            &library,
            &crate::model::RuleInput {
                id: None,
                name: "scans".to_string(),
                pattern: "scan".to_string(),
                is_regex: false,
                tags: vec!["scan".to_string()],
                enabled: true,
            },
        )
        .unwrap();
        let bytes = png_bytes(2, 2);

        let ingested = store_image(
            &library,
            IngestInput {
                source: ImageSource::Local,
                page_title: Some("a-scan.png"),
                rating: None,
                tags: &[],
                ..input("id-1", &bytes, &[])
            },
        )
        .unwrap();

        assert_eq!(ingested.record().tags, vec!["scan".to_string()]);
    }

    /// `legacy-bundle-import` records "whatever the bundle carries is what is
    /// stored" (design D7): a bundle-sourced ingest gains nothing from a rule
    /// that would otherwise match its title.
    #[test]
    fn a_bundle_sourced_ingest_gains_no_rule_tags() {
        let (_dir, library) = library();
        crate::rules::upsert(
            &library,
            &crate::model::RuleInput {
                id: None,
                name: "pixiv".to_string(),
                pattern: "pixiv".to_string(),
                is_regex: false,
                tags: vec!["pixiv".to_string()],
                enabled: true,
            },
        )
        .unwrap();
        let bytes = png_bytes(2, 2);

        let ingested = store_image(
            &library,
            IngestInput {
                source: ImageSource::LegacyBundle,
                page_title: Some("a pixiv piece"),
                rating: None,
                tags: &[],
                ..input("id-1", &bytes, &[])
            },
        )
        .unwrap();

        assert!(ingested.record().tags.is_empty());
    }

    /// Design D8: the source's own rating wins over a rule's guess.
    #[test]
    fn a_rules_rating_tag_does_not_overwrite_a_supplied_rating() {
        let (_dir, library) = library();
        crate::rules::upsert(
            &library,
            &crate::model::RuleInput {
                id: None,
                name: "explicit".to_string(),
                pattern: "pixiv".to_string(),
                is_regex: false,
                tags: vec!["rating:e".to_string()],
                enabled: true,
            },
        )
        .unwrap();
        let bytes = png_bytes(2, 2);

        let ingested = store_image(
            &library,
            IngestInput {
                page_title: Some("a pixiv piece"),
                rating: Some("s"),
                tags: &[],
                ..input("id-1", &bytes, &[])
            },
        )
        .unwrap();

        assert_eq!(ingested.record().rating.as_deref(), Some("s"));
    }

    /// Spec `auto-tag-rules`, "Retrying a delivery": re-delivering a stored id
    /// adds no tags even after a matching rule was added.
    #[test]
    fn re_delivering_a_stored_id_adds_no_tags_from_a_rule_added_since() {
        let (_dir, library) = library();
        let bytes = png_bytes(2, 2);
        store_image(
            &library,
            IngestInput {
                page_title: Some("a pixiv piece"),
                rating: None,
                tags: &[],
                ..input("id-1", &bytes, &[])
            },
        )
        .unwrap();
        crate::rules::upsert(
            &library,
            &crate::model::RuleInput {
                id: None,
                name: "pixiv".to_string(),
                pattern: "pixiv".to_string(),
                is_regex: false,
                tags: vec!["pixiv".to_string()],
                enabled: true,
            },
        )
        .unwrap();

        let ingested = store_image(
            &library,
            IngestInput {
                page_title: Some("a pixiv piece"),
                rating: None,
                tags: &[],
                ..input("id-1", &bytes, &[])
            },
        )
        .unwrap();

        assert!(matches!(ingested, Ingested::Existing(_)));
        assert!(ingested.record().tags.is_empty());
    }

    /// Design D6: an unusable regex never stops a capture — it is skipped, and
    /// the delivery still succeeds with whatever the other rules matched.
    #[test]
    fn a_rule_with_an_unusable_regex_leaves_the_capture_stored_and_successful() {
        let (_dir, library) = library();
        library
            .conn
            .execute(
                "INSERT INTO rules (id, name, pattern, is_regex, tags_json, enabled, created_at,
                                    updated_at)
                 VALUES ('r-1', 'broken', '(unterminated', 1, '[\"x\"]', 1, 0, 0)",
                [],
            )
            .unwrap();
        let bytes = png_bytes(2, 2);

        let ingested = store_image(
            &library,
            IngestInput {
                page_title: Some("anything"),
                rating: None,
                tags: &[],
                ..input("id-1", &bytes, &[])
            },
        )
        .unwrap();

        assert!(matches!(ingested, Ingested::Created(_)));
        assert!(ingested.record().tags.is_empty());
    }

    fn search_view(library: &Library, view: crate::model::SearchView) -> Vec<String> {
        crate::query::search(
            &library.conn,
            &crate::model::SearchRequest {
                query: crate::model::ParsedTagSearch::default(),
                text: String::new(),
                view,
                sort: Default::default(),
                group: Default::default(),
                limit: 100,
                offset: 0,
            },
        )
        .unwrap()
        .images
        .into_iter()
        .map(|record| record.id)
        .collect()
    }

    /// `legacy-bundle-import` task 1.2: a `Some` `deleted_at` goes into the row
    /// the one insert writes, so a trashed bundle row is absent from the
    /// library the moment it arrives rather than needing a second write to
    /// trash it.
    #[test]
    fn a_deleted_at_input_stores_a_row_the_library_search_skips_and_the_trash_search_finds() {
        let (_dir, library) = library();
        let bytes = png_bytes(2, 2);
        let tags: Vec<String> = Vec::new();

        store_image(
            &library,
            IngestInput {
                deleted_at: Some(1_700_000_000_000),
                ..input("id-1", &bytes, &tags)
            },
        )
        .unwrap();

        assert!(search_view(&library, crate::model::SearchView::Library).is_empty());
        assert_eq!(
            search_view(&library, crate::model::SearchView::Trash),
            vec!["id-1".to_string()]
        );
        let record = load_record(&library.conn, "id-1").unwrap().unwrap();
        assert_eq!(record.deleted_at, Some(1_700_000_000_000));
    }
}
