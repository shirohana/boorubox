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
use crate::model::{ImageRecord, ImageSource};

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
    pub rating: Option<&'a str>,
    pub tags: &'a [String],
    pub captured_at: i64,
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
     image_url, page_url, page_title, rating, captured_at, created_at, updated_at, deleted_at, \
     missing";

struct Decoded {
    width: u32,
    height: u32,
    ext: &'static str,
    mime: &'static str,
}

pub fn store_image(library: &Library, input: IngestInput) -> Result<Ingested> {
    if let Some(existing) = load_record(&library.conn, input.id)? {
        return Ok(Ingested::Existing(existing));
    }

    let decoded = decode(input.bytes)?;
    let path = library.image_path(input.id, decoded.ext);
    write_through_inbox(library, input.id, input.bytes, &path)?;

    let ingested = match insert_rows(library, &input, &decoded) {
        Ok(ingested) => ingested,
        Err(error) => {
            let _ = std::fs::remove_file(&path);
            return Err(error);
        }
    };

    if let Ingested::Created(record) = &ingested {
        // A thumbnail is a derived cache that `.thumbs/` can lose at any time
        // (design D7), so it is regenerated on demand. Failing the ingest here
        // would instead throw away an image the caller already handed us and
        // cannot re-send — the capture is gone from the page by then.
        let _ = crate::thumbs::ensure_thumbnail(library, record);
    }
    Ok(ingested)
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
    let part = library.part_path(id);
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
    let inserted = tx.execute(
        "INSERT INTO images (id, ext, mime, size, width, height, source, source_ref, image_url,
                             page_url, page_title, rating, captured_at, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
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
            input.rating,
            input.captured_at,
            now,
            now,
        ],
    )?;

    if inserted == 0 {
        // Unreachable while D1 holds: the caller above already found no row, and
        // one mutex-guarded connection means nobody inserted in between. Give a
        // second writer here and it also orphans the file just renamed in.
        drop(tx);
        return Ok(Ingested::Existing(require_record(&library.conn, input.id)?));
    }

    for tag in input.tags {
        link_tag(&tx, input.id, tag)?;
    }
    tx.commit()?;

    Ok(Ingested::Created(require_record(&library.conn, input.id)?))
}

fn link_tag(conn: &Connection, image_id: &str, tag: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO tags (name) VALUES (?1) ON CONFLICT (name) DO NOTHING",
        [tag],
    )?;
    conn.execute(
        "INSERT INTO image_tags (image_id, tag_id)
         SELECT ?1, id FROM tags WHERE name = ?2
         ON CONFLICT DO NOTHING",
        params![image_id, tag],
    )?;
    Ok(())
}

/// Map a row selected with [`IMAGE_COLUMNS`]. `tags` comes back empty: tags are
/// a second query, so that a hundred records cost two statements, not a hundred.
pub fn row_to_record(row: &Row) -> rusqlite::Result<ImageRecord> {
    Ok(ImageRecord {
        id: row.get(0)?,
        ext: row.get(1)?,
        mime: row.get(2)?,
        size: row.get(3)?,
        width: row.get(4)?,
        height: row.get(5)?,
        source: row.get(6)?,
        source_ref: row.get(7)?,
        image_url: row.get(8)?,
        page_url: row.get(9)?,
        page_title: row.get(10)?,
        rating: row.get(11)?,
        tags: Vec::new(),
        captured_at: row.get(12)?,
        created_at: row.get(13)?,
        updated_at: row.get(14)?,
        deleted_at: row.get(15)?,
        missing: row.get(16)?,
    })
}

pub fn load_record(conn: &Connection, id: &str) -> Result<Option<ImageRecord>> {
    Ok(load_records(conn, std::slice::from_ref(&id.to_string()))?.pop())
}

/// Load records for `ids`, in the order given; ids with no row are dropped.
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

    Ok(ids.iter().filter_map(|id| by_id.remove(id)).collect())
}

fn require_record(conn: &Connection, id: &str) -> Result<ImageRecord> {
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
            rating: Some("s"),
            tags,
            captured_at: 1_700_000_000_000,
        }
    }

    fn entry_count(dir: &Path) -> usize {
        std::fs::read_dir(dir).unwrap().count()
    }

    fn library() -> (tempfile::TempDir, Library) {
        let dir = tempfile::tempdir().unwrap();
        let library = Library::open_or_create(dir.path()).unwrap();
        (dir, library)
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
        assert!(library.image_path("id-1", "png").is_file());
        assert_eq!(entry_count(&library.inbox_dir()), 0);
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
        assert_eq!(entry_count(&library.images_dir()), 1);
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
        assert_eq!(entry_count(&library.images_dir()), 0);
        assert_eq!(entry_count(&library.inbox_dir()), 0);
        assert_eq!(library.image_count().unwrap(), 0);
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
}
