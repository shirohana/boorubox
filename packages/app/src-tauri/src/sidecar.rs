//! Per-image and library-level sidecars (design D1–D7): a second,
//! human-readable copy of everything `library.sqlite` holds, written beside
//! the files it describes so the database can be rebuilt from the folder
//! alone. Nothing reads a sidecar back at runtime — every read goes through
//! the database — except the rebuild this change also adds (`recover.rs`).

use std::fs::{self, File};
use std::io::Write as _;
use std::path::{Path, PathBuf};

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::booru::sites;
use crate::collections;
use crate::error::{AppError, Result};
use crate::ingest;
use crate::library::LibraryPaths;
use crate::model::{
    BooruSite, Collection, ImageRecord, ImageSource, Note, PostRef, Rule, SiteAdapterRecord, Stamp,
    TagEntry,
};
use crate::notes;
use crate::rules;
use crate::stamps;
use crate::tags;

/// The sidecar format's own number (design D1): what a future reader has to
/// know to parse this file, bumped only when the shape changes in a way a v1
/// reader would get wrong. Independent of `PRAGMA user_version` and of
/// `MIGRATIONS.len()` in `db.rs` — this change claims no schema version
/// (design D7).
const SIDECAR_VERSION: u32 = 1;

/// `library.json`'s own format number — the same idea as [`SIDECAR_VERSION`],
/// for the one file per library rather than per image (design D3).
const LIBRARY_VERSION: u32 = 1;

const LIBRARY_FILE_NAME: &str = "library.json";
const SIDECAR_PART_SUFFIX: &str = "json.part";

/// One image's row, its tags and its posts, exactly as
/// `images/<a1>/<id>.json` holds them (design D1; `one-level-buckets` design
/// D1). Deliberately not `ImageRecord` (design D2): `file` is a function of
/// `id` and `ext` (`LibraryPaths::image_path`) and storing it would be a
/// second spelling of the layout, and `missing` is a cache of the last stat
/// that the rebuild recomputes for free — see `maintenance::refresh_missing_for`,
/// the one write to `images` that never touches a sidecar.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sidecar {
    pub version: u32,
    pub id: String,
    pub ext: String,
    pub mime: String,
    pub size: i64,
    pub width: u32,
    pub height: u32,
    pub source: ImageSource,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adapter: Option<SiteAdapterRecord>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rating: Option<String>,
    /// Sorted (design D1): `load_records`' own `ORDER BY tags.name` already
    /// gives this order, so nothing here sorts a second time.
    pub tags: Vec<String>,
    pub captured_at: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_modified_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<i64>,
    pub posts: Vec<PostRef>,
    /// The collections this image is in, by id (`collections` design D4). An
    /// old sidecar with no such key reads as "in none" — additive, so the
    /// sidecar format stays 1 — and an old reader ignores the field it does
    /// not know.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub collections: Vec<String>,
}

impl From<&ImageRecord> for Sidecar {
    fn from(record: &ImageRecord) -> Self {
        Sidecar {
            version: SIDECAR_VERSION,
            id: record.id.clone(),
            ext: record.ext.clone(),
            mime: record.mime.clone(),
            size: record.size,
            width: record.width,
            height: record.height,
            source: record.source,
            source_ref: record.source_ref.clone(),
            image_url: record.image_url.clone(),
            page_url: record.page_url.clone(),
            page_title: record.page_title.clone(),
            adapter: record.adapter.clone(),
            rating: record.rating.clone(),
            tags: record.tags.clone(),
            captured_at: record.captured_at,
            file_modified_at: record.file_modified_at,
            created_at: record.created_at,
            updated_at: record.updated_at,
            deleted_at: record.deleted_at,
            posts: record.posts.clone(),
            collections: record.collections.clone(),
        }
    }
}

/// `library.json`: the rules, the booru sites, the note, the collections, the
/// tag vocabulary's exceptions and the stamps — everything in the library
/// that is not per image (design D3, `stamps` design D3). Rule and site ids
/// are the database's own, written and restored verbatim: `posts.site` holds
/// a site id and rule ids travel in the export format, so regenerating either
/// on rebuild would break a reference. No API key, ever: `BooruSite` has no
/// field for one (`booru-sites` design D7), so that is a property of the type
/// serialized here, not a rule this module has to remember.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryFile {
    pub version: u32,
    pub rules: Vec<Rule>,
    pub booru_sites: Vec<BooruSite>,
    pub note: Note,
    /// The collections, by id and name (`collections` design D4): a
    /// membership is written into each image's sidecar, but the collection
    /// itself — the only place its name lives — is library-level, the same
    /// as a rule or a site. Always written by this build, even empty (an old
    /// reader ignores a key it does not know, so the format stays 1), and
    /// `Option` rather than a defaulted `Vec` because a rebuild has to tell
    /// two files apart: `"collections": []` is "the user has none, the seed
    /// was deleted", while no key at all is a file written before this build
    /// existed and says nothing about collections — there the migration's own
    /// seed is the better answer than an emptied table.
    #[serde(default)]
    pub collections: Option<Vec<Collection>>,
    /// The tag vocabulary's exceptions (`tag-vocabulary` design D2): every tag
    /// that is not `(general, unpinned)`, with its category and its pin,
    /// sorted by name. The same `Option` reasoning as `collections` above:
    /// `None` is a file written before this change, and every tag comes back
    /// general and unpinned on a rebuild; `Some` — empty included — is
    /// restored onto the rows verbatim.
    #[serde(default)]
    pub tags: Option<Vec<TagEntry>>,
    /// The stamps, by creation order (`stamps` design D3). The same `Option`
    /// reasoning as `collections` and `tags` above: `None` is a file written
    /// before this change and says nothing about stamps, while `Some` — empty
    /// included — is restored onto the table verbatim; this build always
    /// writes `Some`.
    #[serde(default)]
    pub stamps: Option<Vec<Stamp>>,
}

/// Where `id`'s sidecar lives: the same bucket as its image, through
/// `LibraryPaths::image_path` (design D1) rather than a second `shard_dirs`
/// call — `image_path`'s own extension parameter is exactly this filename.
pub fn path(paths: &LibraryPaths, id: &str) -> PathBuf {
    paths.image_path(id, "json")
}

/// Where the library-level file lives: the library root, beside
/// `library.sqlite` (design D3).
pub fn library_path(paths: &LibraryPaths) -> PathBuf {
    paths.root.join(LIBRARY_FILE_NAME)
}

fn part_path(paths: &LibraryPaths, id: &str) -> PathBuf {
    paths
        .inbox_dir()
        .join(format!("{id}.{SIDECAR_PART_SUFFIX}"))
}

fn library_part_path(paths: &LibraryPaths) -> PathBuf {
    paths.inbox_dir().join("library.json.part")
}

/// Read and parse the sidecar at `path`. A caller decides what a malformed one
/// means — the rebuild counts and names it as a failure (design D11) rather
/// than letting one bad document fail the whole read.
///
/// A `version` that is not [`SIDECAR_VERSION`] is one of those failures: the
/// number exists so that a reader never has to guess which shape it is holding
/// (design D1), and parsing a shape this build does not know would adopt
/// whichever fields happen to line up and silently drop the rest. Counted and
/// named instead, so a rebuild reports the file rather than quietly rewriting
/// a newer one as v1.
pub fn read(path: &Path) -> Result<Sidecar> {
    let bytes = fs::read(path)?;
    let sidecar: Sidecar = serde_json::from_slice(&bytes).map_err(|error| {
        AppError::BadRequest(format!(
            "sidecar at {} cannot be read: {error}",
            path.display()
        ))
    })?;
    check_version("sidecar", path, sidecar.version, SIDECAR_VERSION)?;
    Ok(sidecar)
}

/// The shared refusal behind [`read`] and [`read_library`]: a file whose
/// format number is not the one this build writes is a read failure whose
/// reason names both numbers, so the rebuild's report says why the file was
/// skipped rather than just that it was.
fn check_version(kind: &str, path: &Path, found: u32, expected: u32) -> Result<()> {
    if found == expected {
        return Ok(());
    }
    Err(AppError::BadRequest(format!(
        "{kind} at {} is format version {found}, and this BooruBox reads version {expected}",
        path.display()
    )))
}

/// Write `sidecar` through `inbox/<id>.json.part` and a rename (design D6): no
/// reader ever sees a half-written file, and no fsync — the failure this
/// answers is another process rewriting `library.sqlite`, not a power cut, and
/// a sidecar lost to one is rewritten by the next open's backfill.
pub fn write(paths: &LibraryPaths, sidecar: &Sidecar) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(sidecar)
        .map_err(|error| AppError::BadRequest(format!("sidecar cannot be encoded: {error}")))?;
    write_through_part(
        &part_path(paths, &sidecar.id),
        &path(paths, &sidecar.id),
        &bytes,
    )
}

/// Write every id in `ids`' sidecar from the row it has right now: one
/// `load_records` call for the whole slice (design D4), then [`write_for_records`].
/// An id with no row is silently skipped — `load_records` already drops it —
/// rather than failing the whole call over a caller's stale id.
pub fn write_for(paths: &LibraryPaths, conn: &Connection, ids: &[String]) -> Result<()> {
    write_for_records(paths, &ingest::load_records(conn, ids)?)
}

/// [`write_for`]'s own body, split out for a caller that already holds the
/// records (review finding 3, `stamps`): `tags::apply_edit` reads `ids` once,
/// for its own answer, and writes the sidecars from that same read rather
/// than through `write_for`'s own second `load_records` call over the same
/// ids.
pub fn write_for_records(paths: &LibraryPaths, records: &[ImageRecord]) -> Result<()> {
    for record in records {
        write(paths, &Sidecar::from(record))?;
    }
    Ok(())
}

/// One id's sidecar, for a caller holding an id rather than a slice: the same
/// `write_for` over a slice of one, spelled once here instead of at each of
/// the single-image write paths (design D5).
pub fn write_one(paths: &LibraryPaths, conn: &Connection, id: &str) -> Result<()> {
    write_for(paths, conn, std::slice::from_ref(&id.to_string()))
}

/// Remove `id`'s sidecar, treating "already gone" as success — the same
/// convention `trash::remove_if_present` uses for an image file. Called
/// before the image file itself on a permanent delete (design D5): a sidecar
/// that survives is the one thing that can resurrect a deleted image on a
/// later rebuild.
pub fn remove_for(paths: &LibraryPaths, id: &str) -> Result<()> {
    match fs::remove_file(path(paths, id)) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(AppError::Io(error)),
    }
}

/// Read `library.json` at `path`. A `version` this build does not write is a
/// read failure naming the version, for the same reason [`read`] refuses one.
pub fn read_library(path: &Path) -> Result<LibraryFile> {
    let bytes = fs::read(path)?;
    let file: LibraryFile = serde_json::from_slice(&bytes).map_err(|error| {
        AppError::BadRequest(format!(
            "library file at {} cannot be read: {error}",
            path.display()
        ))
    })?;
    check_version("library file", path, file.version, LIBRARY_VERSION)?;
    Ok(file)
}

/// Write `library.json` from the rules, the booru sites, the note, the
/// collections, the tag vocabulary and the stamps as they stand right now
/// (design D3, `tag-vocabulary` design D2, `stamps` design D3).
pub fn write_library(paths: &LibraryPaths, conn: &Connection) -> Result<()> {
    let file = LibraryFile {
        version: LIBRARY_VERSION,
        rules: rules::list(conn)?
            .into_iter()
            .map(|entry| entry.rule)
            .collect(),
        booru_sites: sites::list(conn)?,
        note: notes::get(conn)?,
        collections: Some(collections::list(conn)?),
        tags: Some(tags::vocabulary(conn)?),
        stamps: Some(stamps::list(conn)?),
    };
    let bytes = serde_json::to_vec_pretty(&file).map_err(|error| {
        AppError::BadRequest(format!("library file cannot be encoded: {error}"))
    })?;
    write_through_part(&library_part_path(paths), &library_path(paths), &bytes)
}

/// Write `bytes` to `part`, then rename onto `dest` (design D6): the same
/// temp-then-rename shape `ingest::write_through_inbox` uses for image bytes,
/// minus the fsync — a sidecar lost to a crash is rewritten by the next
/// open's backfill, so there is nothing here worth flushing a disk for.
fn write_through_part(part: &Path, dest: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(dir) = dest.parent() {
        fs::create_dir_all(dir)?;
    }
    let attempt = || -> Result<()> {
        let mut file = File::create(part)?;
        file.write_all(bytes)?;
        Ok(())
    };
    if let Err(error) = attempt() {
        let _ = fs::remove_file(part);
        return Err(error);
    }
    fs::rename(part, dest).map_err(|error| {
        let _ = fs::remove_file(part);
        AppError::Io(error)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::booru::credentials::InMemoryCredentials;
    use crate::ingest::{IngestInput, store_image};
    use crate::library::Library;
    use crate::model::{RuleInput, TagCategory};

    fn library() -> (tempfile::TempDir, Library) {
        let dir = tempfile::tempdir().unwrap();
        let library = Library::open_or_create(dir.path()).unwrap();
        (dir, library)
    }

    fn png_bytes() -> Vec<u8> {
        let image = image::DynamicImage::ImageRgb8(image::RgbImage::new(2, 2));
        let mut out = std::io::Cursor::new(Vec::new());
        image.write_to(&mut out, image::ImageFormat::Png).unwrap();
        out.into_inner()
    }

    fn store(library: &Library, id: &str, tags: &[&str]) {
        let tags: Vec<String> = tags.iter().map(|tag| (*tag).to_string()).collect();
        store_image(
            library,
            IngestInput {
                id,
                bytes: &png_bytes(),
                source: ImageSource::Extension,
                source_ref: Some("x"),
                image_url: Some("https://example.test/i.png"),
                page_url: Some("https://example.test/p"),
                page_title: Some("a page"),
                adapter: None,
                rating: Some("s"),
                tags: &tags,
                captured_at: 1_700_000_000_000,
                file_modified_at: None,
                deleted_at: None,
            },
        )
        .unwrap();
    }

    fn full_sidecar(id: &str) -> Sidecar {
        Sidecar {
            version: SIDECAR_VERSION,
            id: id.to_string(),
            ext: "png".to_string(),
            mime: "image/png".to_string(),
            size: 148_213,
            width: 1200,
            height: 800,
            source: ImageSource::Extension,
            source_ref: Some("x".to_string()),
            image_url: Some("https://example.test/i.png".to_string()),
            page_url: Some("https://example.test/p".to_string()),
            page_title: Some("a page".to_string()),
            adapter: Some(SiteAdapterRecord {
                site: "x".to_string(),
                fields: serde_json::json!({ "handle": "alice" }),
            }),
            rating: Some("s".to_string()),
            tags: vec!["artist:foo".to_string(), "landscape".to_string()],
            captured_at: 1_757_000_000_000,
            file_modified_at: None,
            created_at: 1_757_000_000_000,
            updated_at: 1_757_000_000_123,
            deleted_at: None,
            posts: vec![PostRef {
                site: "danbooru".to_string(),
                remote_id: "7412".to_string(),
                posted_at: 1_757_000_000_456,
            }],
            collections: vec!["favorites".to_string()],
        }
    }

    #[test]
    fn a_sidecar_round_trips_through_write_and_read_with_every_field_intact() {
        let (_dir, library) = library();
        let sidecar = full_sidecar("a1b2c3d4");

        write(&library.paths, &sidecar).unwrap();
        let read_back = read(&path(&library.paths, &sidecar.id)).unwrap();

        assert_eq!(
            read_back, sidecar,
            "including the None fields, file_modified_at and deleted_at"
        );
    }

    /// `collections` design D4: additive, so a sidecar written before this
    /// change — no `collections` key at all — reads as "in none" rather than
    /// failing to parse.
    #[test]
    fn a_sidecar_from_before_collections_with_no_such_key_reads_as_in_none() {
        let (_dir, library) = library();
        let sidecar = full_sidecar("a");
        let mut json = serde_json::to_value(&sidecar).unwrap();
        json.as_object_mut().unwrap().remove("collections");
        let file = path(&library.paths, "a");
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(&file, serde_json::to_vec_pretty(&json).unwrap()).unwrap();

        let read_back = read(&file).unwrap();

        assert!(read_back.collections.is_empty());
    }

    /// Design D1: `version` exists so a reader never guesses which shape it
    /// is holding. A file this build does not write is refused, with the
    /// version in the reason — so a rebuild counts and names it rather than
    /// adopting whichever fields happen to line up.
    #[test]
    fn a_sidecar_from_another_format_version_is_refused_by_its_version() {
        let (_dir, library) = library();
        let sidecar = Sidecar {
            version: 2,
            ..full_sidecar("a")
        };
        let file = path(&library.paths, "a");
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(&file, serde_json::to_vec_pretty(&sidecar).unwrap()).unwrap();

        let reason = read(&file).unwrap_err().to_string();

        assert!(reason.contains('2'), "{reason}");
    }

    #[test]
    fn a_library_file_from_another_format_version_is_refused_by_its_version() {
        let (_dir, library) = library();
        let file = library_path(&library.paths);
        fs::write(
            &file,
            serde_json::json!({
                "version": 2,
                "rules": [],
                "booruSites": [],
                "note": { "content": "", "updatedAt": 0 },
            })
            .to_string(),
        )
        .unwrap();

        let reason = read_library(&file).unwrap_err().to_string();

        assert!(reason.contains('2'), "{reason}");
    }

    #[test]
    fn writing_the_same_sidecar_twice_produces_identical_bytes() {
        let (_dir, library) = library();
        let sidecar = full_sidecar("a");

        write(&library.paths, &sidecar).unwrap();
        let first = fs::read(path(&library.paths, "a")).unwrap();
        write(&library.paths, &sidecar).unwrap();
        let second = fs::read(path(&library.paths, "a")).unwrap();

        assert_eq!(
            first, second,
            "rewriting an image whose facts did not change must cost a sync client nothing"
        );
    }

    #[test]
    fn a_part_file_left_by_a_killed_sidecar_write_is_swept_on_open() {
        let dir = tempfile::tempdir().unwrap();
        {
            let library = Library::open_or_create(dir.path()).unwrap();
            fs::write(part_path(&library.paths, "a"), b"half a sidecar").unwrap();
        }

        let library = Library::open_or_create(dir.path()).unwrap();

        assert!(!part_path(&library.paths, "a").exists());
    }

    #[test]
    fn write_for_writes_one_file_per_id_in_the_slice() {
        let (_dir, library) = library();
        for id in ["a", "b", "c"] {
            store(&library, id, &["cat"]);
        }

        write_for(
            &library.paths,
            &library.conn,
            &["a".to_string(), "b".to_string(), "c".to_string()],
        )
        .unwrap();

        for id in ["a", "b", "c"] {
            assert!(path(&library.paths, id).is_file());
        }
    }

    #[test]
    fn an_id_with_no_row_writes_nothing_rather_than_failing() {
        let (_dir, library) = library();

        write_for(&library.paths, &library.conn, &["nobody".to_string()]).unwrap();

        assert!(!path(&library.paths, "nobody").exists());
    }

    #[test]
    fn remove_for_deletes_the_sidecar_and_is_a_no_op_when_it_is_already_gone() {
        let (_dir, library) = library();
        store(&library, "a", &[]);
        assert!(path(&library.paths, "a").is_file());

        remove_for(&library.paths, "a").unwrap();
        assert!(!path(&library.paths, "a").exists());
        remove_for(&library.paths, "a").unwrap();
    }

    /// `snake_case` as the column names spell it, `camelCase` as the sidecar
    /// serialises it. Small enough to keep beside the one test that needs it,
    /// rather than pull a crate in for eighteen names.
    fn camel_case(name: &str) -> String {
        let mut out = String::with_capacity(name.len());
        let mut capitalise = false;
        for character in name.chars() {
            if character == '_' {
                capitalise = true;
            } else if capitalise {
                out.extend(character.to_uppercase());
                capitalise = false;
            } else {
                out.push(character);
            }
        }
        out
    }

    /// design D11's drift risk: the guard that fails this suite, not the next
    /// rebuild, the day a migration adds a column and nobody adds it here.
    ///
    /// The field names are read off `Sidecar`'s own serialisation rather than
    /// a list kept beside it: a list is a second spelling of the struct, and
    /// the cheapest way past a failure here would be to add the new column's
    /// name to it — which is exactly the drift this exists to catch. Every
    /// `Option` is filled in first, since a `None` one is omitted from the
    /// file and would read here as a field that is not there.
    #[test]
    fn every_images_column_but_missing_is_represented_in_the_sidecar() {
        let (_dir, library) = library();
        let mut stmt = library
            .conn
            .prepare("SELECT name FROM pragma_table_info('images')")
            .unwrap();
        let columns: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        assert!(!columns.is_empty(), "pragma_table_info named no column");

        let mut sidecar = full_sidecar("a");
        sidecar.file_modified_at = Some(1_757_000_000_001);
        sidecar.deleted_at = Some(1_757_000_000_002);
        let serialised = serde_json::to_value(&sidecar).unwrap();
        let fields: Vec<&String> = serialised.as_object().unwrap().keys().collect();

        for column in &columns {
            // The one named exception (design D2): a cache of the last stat,
            // which the rebuild recomputes from the folder for free.
            if column == "missing" {
                continue;
            }
            // The column holds the serialised form of `Sidecar.adapter`.
            let field = camel_case(if column == "adapter_json" {
                "adapter"
            } else {
                column
            });
            assert!(
                fields.contains(&&field),
                "images.{column} has no representation in Sidecar"
            );
        }
    }

    /// The same drift guard as `every_images_column_but_missing_is_
    /// represented_in_the_sidecar`, for the vocabulary's own row
    /// (`tag-vocabulary` design D2): the day a migration adds a column to
    /// `tags`, this fails the suite rather than letting the column go
    /// unmirrored in `TagEntry` — and so unrestored by a rebuild — with the
    /// rest of the suite green.
    #[test]
    fn every_tags_column_but_id_is_represented_on_tag_entry() {
        let (_dir, library) = library();
        let mut stmt = library
            .conn
            .prepare("SELECT name FROM pragma_table_info('tags')")
            .unwrap();
        let columns: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        assert!(!columns.is_empty(), "pragma_table_info named no column");

        let entry = TagEntry {
            name: "cat".to_string(),
            category: TagCategory::Artist,
            pinned_group: Some(1),
        };
        let serialised = serde_json::to_value(&entry).unwrap();
        let fields: Vec<&String> = serialised.as_object().unwrap().keys().collect();

        for column in &columns {
            // The internal row id: a tag is named by `name` everywhere
            // outside this table, and `TagEntry` never carries the database
            // id nothing else reads.
            if column == "id" {
                continue;
            }
            let field = camel_case(column);
            assert!(
                fields.contains(&&field),
                "tags.{column} has no representation in TagEntry"
            );
        }
    }

    #[test]
    fn a_library_file_round_trips_through_read_library() {
        let (_dir, library) = library();
        for (name, tag) in [("pixiv", "pixiv"), ("twitter", "twitter")] {
            rules::upsert(
                &library,
                &RuleInput {
                    id: None,
                    name: name.to_string(),
                    pattern: name.to_string(),
                    is_regex: false,
                    tags: vec![tag.to_string()],
                    enabled: true,
                },
            )
            .unwrap();
        }
        let credentials = InMemoryCredentials::default();
        sites::save(
            &library,
            &credentials,
            None,
            "Danbooru",
            "https://danbooru.donmai.us",
            "alice",
            Some("secret-key"),
        )
        .unwrap();
        notes::set(&library, "remember to tag these").unwrap();
        store(&library, "a", &[]);
        crate::tags::update_tags(&library, "a", &["artist:kantoku".to_string()]).unwrap();
        let stamp = crate::stamps::upsert(
            &library,
            &crate::model::StampInput {
                id: None,
                name: "Cat".to_string(),
                text: "cat animal".to_string(),
            },
        )
        .unwrap();

        write_library(&library.paths, &library.conn).unwrap();
        let file = read_library(&library_path(&library.paths)).unwrap();

        assert_eq!(file.rules.len(), 2);
        assert_eq!(file.booru_sites.len(), 1);
        assert_eq!(file.note.content, "remember to tag these");
        let collections = file.collections.expect("the key is always written");
        assert_eq!(
            collections.len(),
            1,
            "the seeded Favorites collection, by design D4"
        );
        assert_eq!(collections[0].name, "Favorites");
        let tags = file.tags.expect("the key is always written");
        assert_eq!(
            tags,
            vec![TagEntry {
                name: "kantoku".to_string(),
                category: TagCategory::Artist,
                pinned_group: None,
            }]
        );
        let stamps = file.stamps.expect("the key is always written");
        assert_eq!(stamps, vec![stamp]);
    }

    /// `tag-vocabulary` design D2: additive, so a `library.json` written
    /// before this change — no `tags` key at all — reads as `None` rather
    /// than failing to parse, the same rule `collections` already follows.
    #[test]
    fn a_library_file_from_before_the_vocabulary_with_no_tags_key_reads_as_none() {
        let (_dir, library) = library();

        write_library(&library.paths, &library.conn).unwrap();
        let file = library_path(&library.paths);
        let mut json: serde_json::Value =
            serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
        json.as_object_mut().unwrap().remove("tags");
        fs::write(&file, serde_json::to_vec_pretty(&json).unwrap()).unwrap();

        let read_back = read_library(&file).unwrap();

        assert_eq!(read_back.tags, None);
    }

    /// `pinned-collections` design D2: `Collection.pinned`'s `#[serde(default)]`
    /// means a `library.json` written before this change — whose collection
    /// entries carry no `pinned` key at all — still parses, every entry
    /// reading back unpinned, the same answer the migration's own default
    /// gives a database upgraded without ever having seen this key.
    #[test]
    fn a_library_file_without_collection_pins_reads_every_collection_unpinned() {
        let (_dir, library) = library();
        collections::create(&library, "Queue").unwrap();

        write_library(&library.paths, &library.conn).unwrap();
        let file = library_path(&library.paths);
        let mut json: serde_json::Value =
            serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
        for entry in json["collections"].as_array_mut().unwrap() {
            entry.as_object_mut().unwrap().remove("pinned");
        }
        fs::write(&file, serde_json::to_vec_pretty(&json).unwrap()).unwrap();

        let read_back = read_library(&file).unwrap();

        let collections = read_back.collections.expect("the key is always written");
        assert_eq!(collections.len(), 2, "Favorites plus the created Queue");
        assert!(
            collections.iter().all(|collection| !collection.pinned),
            "{collections:?}",
        );
    }

    /// `stamps` design D3: the same additive rule as `tags` and `collections`
    /// above — a `library.json` written before this change has no `stamps`
    /// key at all and reads as `None` rather than failing to parse.
    #[test]
    fn a_library_file_from_before_stamps_with_no_stamps_key_reads_as_none() {
        let (_dir, library) = library();

        write_library(&library.paths, &library.conn).unwrap();
        let file = library_path(&library.paths);
        let mut json: serde_json::Value =
            serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
        json.as_object_mut().unwrap().remove("stamps");
        fs::write(&file, serde_json::to_vec_pretty(&json).unwrap()).unwrap();

        let read_back = read_library(&file).unwrap();

        assert_eq!(read_back.stamps, None);
    }

    #[test]
    fn the_library_file_never_contains_the_api_key_even_when_one_is_stored() {
        let (_dir, library) = library();
        let credentials = InMemoryCredentials::default();
        sites::save(
            &library,
            &credentials,
            None,
            "Danbooru",
            "https://danbooru.donmai.us",
            "alice",
            Some("super-secret-key"),
        )
        .unwrap();

        write_library(&library.paths, &library.conn).unwrap();
        let text = fs::read_to_string(library_path(&library.paths)).unwrap();

        assert!(!text.contains("super-secret-key"));
    }
}
