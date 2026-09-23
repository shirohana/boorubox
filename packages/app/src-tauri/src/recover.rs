//! Moving a damaged database aside and rebuilding one from the sidecars alone
//! (design D10–D12). Detection (`PRAGMA quick_check`) lives in `db.rs`; this
//! module owns what happens once damage — or a deliberate manual rebuild — is
//! met.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, params};

use crate::collections;
use crate::db;
use crate::error::{AppError, Result};
use crate::library::LibraryPaths;
use crate::model::{
    BooruSite, Collection, Note, RebuildFailure, RebuildReport, Rule, Stamp, TagEntry,
};
use crate::sidecar;
use crate::tags;

/// Rows are inserted in chunks of this size, each its own transaction, so a
/// rebuild's uncommitted work stays bounded rather than holding one rollback
/// journal open for the whole walk (design D11).
const CHUNK: usize = 500;

/// Move `library.sqlite` and, if there is one, its `-journal`, aside under a
/// shared `.corrupt-<now_ms>` suffix (design D10) — renamed, never deleted, in
/// the same folder so the evidence stays where the user was told to look.
/// Returns the database's new path, what `RebuildReport.kept_as` names, or
/// `None` when there was no database to keep.
///
/// A folder whose `library.sqlite` is already gone is the case §7's reversal
/// opens with — "every image file was intact on disk; every tag the user had
/// typed was in the one file that was gone" — and it is the goal this whole
/// change states: a folder the database can be *deleted* from and rebuilt.
/// There is nothing to keep aside then, which is not a failure; refusing here
/// would throw away a rebuild that had already finished its work. The journal
/// goes aside either way, database or not: a stale hot journal left beside the
/// database about to be renamed into place is exactly what SQLite would try to
/// roll back into it, which is how this library died in the first place.
pub fn move_aside(paths: &LibraryPaths) -> Result<Option<PathBuf>> {
    let now = db::now_ms();
    let db_path = paths.db_path();
    let kept = db_path.is_file().then(|| corrupt_name(&db_path, now));
    if let Some(kept) = &kept {
        fs::rename(&db_path, kept)?;
    }

    let journal = journal_path(&db_path);
    if journal.is_file() {
        fs::rename(&journal, corrupt_name(&journal, now))?;
    }
    Ok(kept)
}

fn journal_path(db_path: &Path) -> PathBuf {
    let mut name = db_path.as_os_str().to_os_string();
    name.push("-journal");
    PathBuf::from(name)
}

fn corrupt_name(path: &Path, now: i64) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(format!(".corrupt-{now}"));
    PathBuf::from(name)
}

/// Where a rebuild in progress is built (design D11): a plain sibling of
/// `library.sqlite`, through the ordinary `db::open` so the fresh file gets
/// every migration and every trigger from the one definition, never a second
/// schema hand-copied here.
fn building_path(paths: &LibraryPaths) -> PathBuf {
    let mut name = paths.db_path().into_os_string();
    name.push(".rebuilding");
    PathBuf::from(name)
}

/// Build a working `library.sqlite` from the sidecars alone (design D11):
/// walk `images/**/*.json`, insert each row with its stored facts and
/// `missing` computed from whether the image file is still beside it, link
/// its tags and posts, then restore `library.json`'s rules, sites, note,
/// collections and tag vocabulary verbatim, and every membership the sidecars
/// name. Never opens, decodes or rewrites an image or a thumbnail — an
/// image's dimensions come from its sidecar — and never removes a file under
/// the folder.
///
/// A stale `library.sqlite.rebuilding` left by an interrupted attempt is
/// overwritten, not adopted: an in-progress rebuild that never reached the
/// final rename is not a fresh database anyone should trust half-built.
/// Building into a temp file and renaming it in only at the very end is what
/// makes an interruption here leave the original library exactly as it was.
pub fn rebuild(
    paths: &LibraryPaths,
    on_progress: &mut dyn FnMut(i64, i64),
) -> Result<RebuildReport> {
    let building = building_path(paths);
    let _ = fs::remove_file(&building);
    let conn = db::open(&building)?;

    let sidecar_paths = walk_sidecars(&paths.images_dir())?;
    let (images, mut failures, parsed) = insert_sidecars(&conn, &sidecar_paths, on_progress)?;

    let library_file = sidecar::library_path(paths);
    let (rules, sites) = match sidecar::read_library(&library_file) {
        Ok(file) => {
            let tx = conn.unchecked_transaction()?;
            let rules = insert_rules(&tx, &file.rules)?;
            let sites = insert_sites(&tx, &file.booru_sites)?;
            insert_note(&tx, &file.note)?;
            // The file wins (design D5): the migration's own seed is one row
            // this build put there before a single sidecar was read, and a
            // library-level file that came back from disk is the truer answer
            // to "what collections exist" than a seed guessed before it was
            // read. A file with no `collections` key at all is not that
            // answer — it was written before this build existed — so the seed
            // stands there, exactly as it does when the file is missing.
            if let Some(from_file) = &file.collections {
                tx.execute("DELETE FROM collections", [])?;
                insert_collections(&tx, from_file)?;
            }
            // After the sidecar pass, an upsert rather than an insert
            // (`tag-vocabulary` design D2): a tag the sidecars already linked
            // already has a `(general, unpinned)` row that this only touches
            // up; a tag no sidecar names — the carrier-less artist the spec
            // asks a rebuild to bring back — is created fresh here. A file
            // with no `tags` key at all (`file.tags: None`) says nothing
            // about the vocabulary, so every tag from the sidecars stands
            // general and unpinned, exactly as `db.rs`'s migration defaults
            // them.
            if let Some(vocabulary) = &file.tags {
                insert_vocabulary(&tx, vocabulary)?;
            }
            // The stamps, restored the same way and for the same reason
            // (`stamps` design D3): a stamp names no tag or image row, so it
            // has no dependency on the sidecar pass or on the vocabulary
            // above it — restored here only because this is where every
            // other library-level exceptions list already is.
            if let Some(stamps) = &file.stamps {
                insert_stamps(&tx, stamps)?;
            }
            tx.commit()?;
            (rules, sites)
        }
        // No `library.json` at all — a library never written by a build that
        // has this change — leaves nothing library-level to restore, and that
        // is not a failure. One that is *there* and will not parse is: the
        // rules, the sites and the note are gone with it, and a report showing
        // a silent zero would let the user find that out the next time a rule
        // does not fire instead of now.
        //
        // Either way the migration's own seed (`db.rs` `SCHEMA_V6`) is left
        // standing rather than deleted: only a file that lists collections
        // replaces it (above), so a library rebuilt with no readable
        // library-level file gets exactly the one collection a freshly opened
        // library would.
        Err(AppError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => (0, 0),
        Err(error) => {
            failures.push(RebuildFailure {
                file: library_file.display().to_string(),
                reason: error.to_string(),
            });
            (0, 0)
        }
    };

    // Placeholders, then memberships (design D5): a membership row references
    // a collection id, and `image_collections.collection_id` is a foreign key
    // — every id a sidecar names has to exist in `collections` before
    // `insert_memberships` can link to it.
    let tx = conn.unchecked_transaction()?;
    insert_placeholder_collections(&tx, &parsed)?;
    insert_memberships(&tx, &parsed)?;
    tx.commit()?;

    // Computed, never carried through the match arms above (CLAUDE.md: single
    // source of truth) — the seed, the file's own rows and every placeholder
    // all land in one table, and this is the one place that has to agree with
    // what a query against the rebuilt library would find.
    let collections: i64 =
        conn.query_row("SELECT COUNT(*) FROM collections", [], |row| row.get(0))?;

    drop(conn);
    let kept = move_aside(paths)?;
    fs::rename(&building, paths.db_path())?;

    Ok(RebuildReport {
        images,
        failed: failures.len() as i64,
        failures,
        kept_as: kept
            .map(|path| path.display().to_string())
            .unwrap_or_default(),
        rules,
        sites,
        collections,
    })
}

/// The file's own collections, restored verbatim (design D5) — id and times as
/// `library.json` names them, the same rule [`insert_rules`] and
/// [`insert_sites`] already follow for their own ids. `slug` is recomputed
/// through [`collections::slug`] rather than trusted from the file: it is
/// derived from `name`, and design D2 keeps that derivation in the one place
/// that owns it, not duplicated into every writer that ever produces a
/// `Collection`.
fn insert_collections(conn: &Connection, collections: &[Collection]) -> Result<()> {
    for collection in collections {
        conn.execute(
            "INSERT INTO collections (id, name, slug, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                collection.id,
                collection.name,
                collections::slug(&collection.name),
                collection.created_at,
                collection.updated_at,
            ],
        )?;
    }
    Ok(())
}

/// The vocabulary's exceptions, restored onto the rows the sidecar pass
/// already created (`tag-vocabulary` design D2): `ON CONFLICT` upserts a
/// carrying tag's category and pin onto its existing row, and creates a
/// carrier-less one fresh, under its own category with no image tagging it —
/// the spec's "The vocabulary comes back" scenario.
fn insert_vocabulary(conn: &Connection, entries: &[TagEntry]) -> Result<()> {
    for entry in entries {
        conn.execute(
            "INSERT INTO tags (name, category, pinned) VALUES (?1, ?2, ?3)
             ON CONFLICT (name) DO UPDATE SET category = excluded.category,
                                               pinned = excluded.pinned",
            params![entry.name, entry.category, entry.pinned],
        )?;
    }
    Ok(())
}

/// Stamps restored with their own id and timestamps verbatim (design D3), the
/// same rule [`insert_rules`] and [`insert_sites`] already follow for their
/// own ids — never through `stamps::upsert`, which mints a fresh id on every
/// create.
fn insert_stamps(conn: &Connection, stamps: &[Stamp]) -> Result<()> {
    for stamp in stamps {
        conn.execute(
            "INSERT INTO stamps (id, name, text, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                stamp.id,
                stamp.name,
                stamp.text,
                stamp.created_at,
                stamp.updated_at
            ],
        )?;
    }
    Ok(())
}

/// A placeholder, named by its own id, for every membership id the sidecars
/// name that is not already a collection (design D5) — a membership whose
/// collection the library-level file does not list comes back rather than
/// being lost, under a name the user can rename away. `INSERT OR IGNORE`: an
/// id already present (the seed, or one of the file's own rows) is left
/// exactly as it stands, never overwritten by a placeholder guess.
fn insert_placeholder_collections(conn: &Connection, parsed: &[sidecar::Sidecar]) -> Result<()> {
    let now = db::now_ms();
    let mut seen = HashSet::new();
    for sidecar in parsed {
        for id in &sidecar.collections {
            if !seen.insert(id.as_str()) {
                continue;
            }
            conn.execute(
                "INSERT OR IGNORE INTO collections (id, name, slug, created_at, updated_at)
                 VALUES (?1, ?1, ?2, ?3, ?3)",
                params![id, collections::slug(id), now],
            )?;
        }
    }
    Ok(())
}

/// One `image_collections` row per id a sidecar names (design D5), from the
/// `Sidecar`s [`insert_sidecars`] already parsed rather than a second read of
/// every file. `added_at` has no sidecar field of its own to restore — a
/// membership carries only the id — so this uses the sidecar's own
/// `updated_at` as the closest fact on hand; nothing in the app reads the
/// column back today (design D3's `add`/`remove` write it, nothing queries
/// it), so a rebuild is free to approximate it.
///
/// `ON CONFLICT DO NOTHING`, the rule [`tags::link_tag`] already follows for a
/// repeated tag: this pass runs outside the per-sidecar savepoint, so an id
/// listed twice in one hand-edited `collections` array would otherwise end the
/// whole rebuild — and end it again on every retry.
fn insert_memberships(conn: &Connection, parsed: &[sidecar::Sidecar]) -> Result<()> {
    for sidecar in parsed {
        for collection_id in &sidecar.collections {
            conn.execute(
                "INSERT INTO image_collections (image_id, collection_id, added_at)
                 VALUES (?1, ?2, ?3)
                 ON CONFLICT DO NOTHING",
                params![sidecar.id, collection_id, sidecar.updated_at],
            )?;
        }
    }
    Ok(())
}

/// Every `*.json` under `dir`, depth-first and in a stable order so two
/// rebuilds of the same folder produce the same report. A flat sidecar from a
/// pre-bucket library (`sharded-image-dirs`) is found the same way as a
/// bucketed one: this walks whatever tree is there rather than assuming two
/// levels.
///
/// Shorter filenames first within a folder, which decides the one case where
/// the order is load-bearing: two files carrying one id, the "conflicted copy"
/// a sync client leaves (design D15). A copy's name can only ever be *longer*
/// than the `<id>.json` it was made from, so the file the app itself writes —
/// and will go on rewriting at every edit — is the one whose row lands, and
/// the copy is what gets counted and named. Plain alphabetical order gives the
/// copy the row instead, and a database that disagrees with the file the app
/// maintains would stay that way through every later rebuild.
fn walk_sidecars(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    collect_json_files(dir, &mut out)?;
    out.sort_by(|left, right| walk_order(left).cmp(&walk_order(right)));
    Ok(out)
}

fn walk_order(path: &Path) -> (Option<&Path>, usize, &std::ffi::OsStr) {
    let name = path.file_name().unwrap_or(path.as_os_str());
    (path.parent(), name.len(), name)
}

fn collect_json_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_json_files(&path, out)?;
        } else if path.extension().is_some_and(|ext| ext == "json") {
            out.push(path);
        }
    }
    Ok(())
}

/// Insert every sidecar's row, chunked into transactions of [`CHUNK`]
/// (design D11's "commit in chunks so memory stays flat"). A sidecar that will
/// not parse is counted and named rather than stopping the walk (design D11,
/// the same rule `row_to_record` already applies to a malformed
/// `adapter_json` — one bad document must not take the rebuild down with it).
fn insert_sidecars(
    conn: &Connection,
    sidecar_paths: &[PathBuf],
    on_progress: &mut dyn FnMut(i64, i64),
) -> Result<(i64, Vec<RebuildFailure>, Vec<sidecar::Sidecar>)> {
    let total = sidecar_paths.len() as i64;
    let mut images = 0i64;
    let mut failures = Vec::new();
    // Kept for the membership pass (design D5) rather than re-read from disk
    // — but only the sidecars that name a collection, which is the whole set
    // `insert_memberships` and `insert_placeholder_collections` read: a
    // library that uses no collections holds nothing here, and one that does
    // never holds an adapter record it has already inserted for the sake of a
    // list of ids.
    let mut parsed = Vec::new();
    on_progress(0, total);

    for chunk in sidecar_paths.chunks(CHUNK) {
        let tx = conn.unchecked_transaction()?;
        for sidecar_path in chunk {
            match sidecar::read(sidecar_path) {
                Ok(sidecar) => match insert_within_savepoint(&tx, sidecar_path, &sidecar) {
                    Ok(()) => {
                        images += 1;
                        if !sidecar.collections.is_empty() {
                            parsed.push(sidecar);
                        }
                    }
                    Err(error) => failures.push(RebuildFailure {
                        file: sidecar_path.display().to_string(),
                        reason: error.to_string(),
                    }),
                },
                Err(error) => failures.push(RebuildFailure {
                    file: sidecar_path.display().to_string(),
                    reason: error.to_string(),
                }),
            }
            on_progress(images + failures.len() as i64, total);
        }
        tx.commit()?;
    }
    Ok((images, failures, parsed))
}

/// One sidecar's insert, inside a savepoint, so a row the database will not
/// take is counted and named exactly like a file that would not parse rather
/// than rolling back the five hundred rows sharing its chunk.
///
/// The case this is really for is two files carrying one id — what a sync
/// client's "conflicted copy" of a sidecar leaves in the bucket, which design
/// D15 says outright this change will produce and never merge. Without the
/// savepoint the second one's `UNIQUE` failure ends the whole rebuild, and
/// ends it again on every retry: the one library that most needs rebuilding
/// would be the one that cannot be.
fn insert_within_savepoint(
    conn: &Connection,
    sidecar_path: &Path,
    sidecar: &sidecar::Sidecar,
) -> Result<()> {
    conn.execute_batch("SAVEPOINT image")?;
    match insert_sidecar(conn, sidecar_path, sidecar) {
        Ok(()) => {
            conn.execute_batch("RELEASE image")?;
            Ok(())
        }
        Err(error) => {
            conn.execute_batch("ROLLBACK TO image; RELEASE image")?;
            Err(error)
        }
    }
}

/// One sidecar's row, its tags and its posts (design D11). `missing` is
/// computed here, for free, since the walk is already looking at the folder —
/// the one column the sidecar deliberately does not carry (`sidecar` design
/// D2). Design D11 spells the check as "is `<id>.<ext>` beside this sidecar",
/// and it is asked of the sidecar's own directory rather than of
/// `LibraryPaths::image_path`: a flat library from before `sharded-image-dirs`
/// keeps its sidecar beside its image too, and asking for the bucketed path
/// would call every one of its images missing until something stat'd them
/// again.
fn insert_sidecar(
    conn: &Connection,
    sidecar_path: &Path,
    sidecar: &sidecar::Sidecar,
) -> Result<()> {
    let adapter_json = sidecar
        .adapter
        .as_ref()
        .map(serde_json::to_string)
        .transpose()
        .map_err(|error| {
            AppError::BadRequest(format!("adapter record cannot be stored: {error}"))
        })?;
    let image = sidecar_path.with_file_name(format!("{}.{}", sidecar.id, sidecar.ext));
    let missing = !image.is_file();

    conn.execute(
        "INSERT INTO images (id, ext, mime, size, width, height, source, source_ref, image_url,
                             page_url, page_title, adapter_json, rating, captured_at, created_at,
                             updated_at, file_modified_at, deleted_at, missing)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)",
        params![
            sidecar.id,
            sidecar.ext,
            sidecar.mime,
            sidecar.size,
            sidecar.width,
            sidecar.height,
            sidecar.source,
            sidecar.source_ref,
            sidecar.image_url,
            sidecar.page_url,
            sidecar.page_title,
            adapter_json,
            sidecar.rating,
            sidecar.captured_at,
            sidecar.created_at,
            sidecar.updated_at,
            sidecar.file_modified_at,
            sidecar.deleted_at,
            missing,
        ],
    )?;

    // The one place a `tags` row is created (`tags::link_tag`'s own doc
    // comment) — the rebuild recreates tag ids fresh rather than restoring
    // them, exactly as design D3 says to for this internal id.
    for tag in &sidecar.tags {
        tags::link_tag(conn, &sidecar.id, tag)?;
    }
    for post in &sidecar.posts {
        conn.execute(
            "INSERT INTO posts (image_id, site, remote_id, posted_at) VALUES (?1, ?2, ?3, ?4)",
            params![sidecar.id, post.site, post.remote_id, post.posted_at],
        )?;
    }
    Ok(())
}

/// Rules restored with their own id and timestamps verbatim (design D3): rule
/// ids travel in the export format (`auto-tag-rules` design D10), so
/// regenerating one on rebuild would break a reference to it. Never through
/// `rules::upsert`, which mints a fresh id on every create.
fn insert_rules(conn: &Connection, rules: &[Rule]) -> Result<i64> {
    for rule in rules {
        let tags_json = serde_json::to_string(&rule.tags)
            .map_err(|error| AppError::BadRequest(format!("tags cannot be stored: {error}")))?;
        conn.execute(
            "INSERT INTO rules (id, name, pattern, is_regex, tags_json, enabled, created_at,
                                updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                rule.id,
                rule.name,
                rule.pattern,
                rule.is_regex,
                tags_json,
                rule.enabled,
                rule.created_at,
                rule.updated_at
            ],
        )?;
    }
    Ok(rules.len() as i64)
}

/// Sites restored with their own id verbatim (design D3): `posts.site` holds
/// this id, so regenerating it on rebuild would orphan every post recorded
/// against it. Never through `booru::sites::save`, which derives a fresh id
/// from the address on every create.
fn insert_sites(conn: &Connection, sites: &[BooruSite]) -> Result<i64> {
    for site in sites {
        conn.execute(
            "INSERT INTO booru_sites (id, name, base_url, username, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                site.id,
                site.name,
                site.base_url,
                site.username,
                site.created_at,
                site.updated_at
            ],
        )?;
    }
    Ok(sites.len() as i64)
}

fn insert_note(conn: &Connection, note: &Note) -> Result<()> {
    conn.execute(
        "INSERT INTO notes (id, content, updated_at) VALUES (1, ?1, ?2)",
        params![note.content, note.updated_at],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::booru::credentials::InMemoryCredentials;
    use crate::ingest::{IngestInput, store_image};
    use crate::library::Library;
    use crate::model::{
        ImageSource, ParsedTagSearch, PostRef, RuleInput, SearchRequest, SearchView,
    };
    use crate::query;

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

    fn store(library: &Library, id: &str, tags: &[&str], rating: Option<&str>) {
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
                page_title: Some("sunset over kyoto"),
                adapter: None,
                rating,
                tags: &tags,
                captured_at: 1_700_000_000_000,
                file_modified_at: None,
                deleted_at: None,
            },
        )
        .unwrap();
    }

    fn everything(view: SearchView) -> SearchRequest {
        SearchRequest {
            query: ParsedTagSearch::default(),
            text: String::new(),
            view,
            sort: Default::default(),
            group: Default::default(),
            limit: 100,
            offset: 0,
        }
    }

    fn tag_search(tag: &str) -> SearchRequest {
        SearchRequest {
            query: ParsedTagSearch {
                include_tags: vec![tag.to_string()],
                ..Default::default()
            },
            ..everything(SearchView::Library)
        }
    }

    fn text_search(text: &str) -> SearchRequest {
        SearchRequest {
            text: text.to_string(),
            ..everything(SearchView::Library)
        }
    }

    fn ids_of(conn: &Connection, req: &SearchRequest) -> Vec<String> {
        let mut ids: Vec<String> = query::search(conn, req)
            .unwrap()
            .images
            .into_iter()
            .map(|image| image.id)
            .collect();
        ids.sort();
        ids
    }

    /// Task 2.4: a full round trip. Everything the library held before the
    /// database is thrown away and rebuilt from the folder alone comes back —
    /// tags, rating, trash state, a recorded post, two rules, a site and the
    /// note — including a tag search and a full-text search, which only work
    /// if the FTS triggers fired on the rebuild's own inserts (design D11).
    #[test]
    fn rebuild_restores_everything_the_database_held() {
        let (dir, library) = library();
        store(&library, "a", &["1girl", "landscape"], Some("s"));
        store(&library, "b", &["cat"], None);
        store(&library, "trashed", &[], None);
        crate::trash::trash_images(&library, &["trashed".to_string()]).unwrap();
        crate::booru::posts::record(
            &library,
            "a",
            &PostRef {
                site: "danbooru".to_string(),
                remote_id: "42".to_string(),
                posted_at: 1_700_000_000_500,
            },
        )
        .unwrap();
        for (name, pattern) in [("pixiv", "pixiv"), ("twitter", "twitter")] {
            crate::rules::upsert(
                &library,
                &RuleInput {
                    id: None,
                    name: name.to_string(),
                    pattern: pattern.to_string(),
                    is_regex: false,
                    tags: vec![pattern.to_string()],
                    enabled: true,
                },
            )
            .unwrap();
        }
        let credentials = InMemoryCredentials::default();
        crate::booru::sites::save(
            &library,
            &credentials,
            None,
            "Danbooru",
            "https://danbooru.donmai.us",
            "alice",
            Some("secret-key"),
        )
        .unwrap();
        crate::notes::set(&library, "remember to tag these").unwrap();
        collections::add(&library, &["a".to_string()], "favorites").unwrap();
        let queue = collections::create(&library, "Queue").unwrap();
        collections::add(&library, &["b".to_string()], &queue.id).unwrap();

        let before_library = ids_of(&library.conn, &everything(SearchView::Library));
        let before_trash = ids_of(&library.conn, &everything(SearchView::Trash));
        let before_tag = ids_of(&library.conn, &tag_search("1girl"));
        let before_text = ids_of(&library.conn, &text_search("kyoto"));
        let before_record = crate::ingest::require_record(&library.conn, "a").unwrap();
        let before_rules = crate::rules::list(&library.conn).unwrap();
        let before_note = crate::notes::get(&library.conn).unwrap();
        let paths = library.paths.clone();
        drop(library);

        let report = rebuild(&paths, &mut |_, _| {}).unwrap();

        assert_eq!(report.images, 3);
        assert_eq!(report.failed, 0);
        assert!(report.failures.is_empty());
        assert_eq!(report.rules, 2);
        assert_eq!(report.sites, 1);
        assert_eq!(
            report.collections, 2,
            "Favorites and Queue, by id and name (`collections` design D5)"
        );
        assert!(
            Path::new(&report.kept_as).is_file(),
            "the old database is kept, named in the report"
        );

        let rebuilt = Library::open_existing(dir.path()).unwrap();
        assert_eq!(
            ids_of(&rebuilt.conn, &everything(SearchView::Library)),
            before_library
        );
        assert_eq!(
            ids_of(&rebuilt.conn, &everything(SearchView::Trash)),
            before_trash
        );
        assert_eq!(ids_of(&rebuilt.conn, &tag_search("1girl")), before_tag);
        assert_eq!(
            ids_of(&rebuilt.conn, &text_search("kyoto")),
            before_text,
            "the FTS triggers must fire on the rebuild's own inserts"
        );
        let after_record = crate::ingest::require_record(&rebuilt.conn, "a").unwrap();
        assert_eq!(after_record, before_record);
        let after_rules: Vec<_> = crate::rules::list(&rebuilt.conn)
            .unwrap()
            .into_iter()
            .map(|entry| entry.rule)
            .collect();
        let before_rules: Vec<_> = before_rules.into_iter().map(|entry| entry.rule).collect();
        assert_eq!(after_rules, before_rules, "rule ids must survive verbatim");
        assert_eq!(
            crate::booru::sites::list(&rebuilt.conn).unwrap()[0].id,
            "danbooru-donmai-us"
        );
        assert_eq!(crate::notes::get(&rebuilt.conn).unwrap(), before_note);
        assert_eq!(
            crate::ingest::require_record(&rebuilt.conn, "a")
                .unwrap()
                .collections,
            vec!["favorites".to_string()],
        );
        assert_eq!(
            crate::ingest::require_record(&rebuilt.conn, "b")
                .unwrap()
                .collections,
            vec![queue.id],
        );
        let names: Vec<String> = collections::list(&rebuilt.conn)
            .unwrap()
            .into_iter()
            .map(|collection| collection.name)
            .collect();
        assert_eq!(names, vec!["Favorites".to_string(), "Queue".to_string()]);
    }

    /// `tag-vocabulary` task 1.4, spec "The vocabulary comes back": an artist
    /// tag carried by one image, a pinned tag on the same image, and a
    /// copyright tag no image carries all come back after a rebuild — the
    /// vocabulary is restored onto the rows the sidecar pass already created,
    /// from `library.json`'s own exceptions list.
    #[test]
    fn rebuild_restores_the_vocabulary_including_a_tag_no_image_carries() {
        let (_dir, library) = library();
        store(&library, "a", &[], None);
        crate::tags::update_tags(&library, "a", &strs(&["artist:kantoku", "tagme"])).unwrap();
        crate::tags::set_pinned(&library, "tagme", true).unwrap();
        store(&library, "b", &[], None);
        crate::tags::update_tags(&library, "b", &strs(&["copyright:azur_lane"])).unwrap();
        crate::tags::update_tags(&library, "b", &[]).unwrap();
        let paths = library.paths.clone();
        drop(library);

        let report = rebuild(&paths, &mut |_, _| {}).unwrap();

        assert!(report.failures.is_empty(), "{:?}", report.failures);
        let rebuilt = Library::open_existing(paths.root.as_path()).unwrap();
        assert_eq!(
            tag_row(&rebuilt.conn, "kantoku"),
            ("artist".to_string(), false)
        );
        assert_eq!(
            tag_row(&rebuilt.conn, "tagme"),
            ("general".to_string(), true)
        );
        assert_eq!(
            tag_row(&rebuilt.conn, "azur_lane"),
            ("copyright".to_string(), false)
        );
        let suggested: Vec<String> = crate::tags::suggestions(&rebuilt.conn, "azur", 8)
            .unwrap()
            .into_iter()
            .map(|tag| tag.name)
            .collect();
        assert_eq!(suggested, vec!["azur_lane".to_string()]);
    }

    /// `stamps` task 1.2, spec "Stamps come back": two stamps survive a
    /// rebuild with their names and texts, in the order they were created —
    /// restored from `library.json` verbatim, the same rule the rules and the
    /// sites already follow for their own ids.
    #[test]
    fn rebuild_restores_the_stamps_in_creation_order() {
        let (_dir, library) = library();
        let cat = crate::stamps::upsert(
            &library,
            &crate::model::StampInput {
                id: None,
                name: "Cat".to_string(),
                text: "cat animal".to_string(),
            },
        )
        .unwrap();
        let reviewed = crate::stamps::upsert(
            &library,
            &crate::model::StampInput {
                id: None,
                name: "Reviewed".to_string(),
                text: "rating:g -tagme".to_string(),
            },
        )
        .unwrap();
        let paths = library.paths.clone();
        drop(library);

        let report = rebuild(&paths, &mut |_, _| {}).unwrap();

        assert!(report.failures.is_empty(), "{:?}", report.failures);
        let rebuilt = Library::open_existing(paths.root.as_path()).unwrap();
        assert_eq!(
            crate::stamps::list(&rebuilt.conn).unwrap(),
            vec![cat, reviewed],
        );
    }

    fn tag_row(conn: &Connection, name: &str) -> (String, bool) {
        conn.query_row(
            "SELECT category, pinned FROM tags WHERE name = ?1",
            [name],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap()
    }

    fn strs(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    }

    /// Task 2.5: a truncated sidecar is counted and named, is left on disk,
    /// and every other image still comes back.
    #[test]
    fn a_truncated_sidecar_is_counted_and_named_and_every_other_image_still_comes_back() {
        let (_dir, library) = library();
        store(&library, "a", &["cat"], None);
        store(&library, "broken", &[], None);
        let broken_path = sidecar::path(&library.paths, "broken");
        std::fs::write(&broken_path, b"{ not valid json").unwrap();
        let paths = library.paths.clone();
        drop(library);

        let report = rebuild(&paths, &mut |_, _| {}).unwrap();

        assert_eq!(report.images, 1);
        assert_eq!(report.failed, 1);
        assert_eq!(report.failures.len(), 1);
        assert!(report.failures[0].file.contains("broken.json"));
        assert!(
            broken_path.is_file(),
            "the unreadable sidecar is left on disk"
        );

        let rebuilt = Library::open_existing(paths.root.as_path()).unwrap();
        assert_eq!(
            ids_of(&rebuilt.conn, &everything(SearchView::Library)),
            vec!["a".to_string()]
        );
    }

    /// Two sidecars carrying one id — what a sync client's "conflicted copy"
    /// of a sidecar leaves in the bucket, which design D15 says this change
    /// will produce and never merge. The loser is counted and named like an
    /// unreadable file; it does not end the rebuild, and it does not end it
    /// again on every retry.
    #[test]
    fn a_second_sidecar_carrying_the_same_id_is_counted_and_named_not_fatal() {
        let (_dir, library) = library();
        store(&library, "a", &["cat"], None);
        store(&library, "b", &[], None);
        let original = sidecar::path(&library.paths, "a");
        // Synology Drive's own shape, and the one that sorts *before*
        // `a.json` alphabetically: the walk's shorter-name-first order is what
        // makes the file the app writes the one whose row lands.
        let conflicted = original.with_file_name("a (conflicted copy).json");
        std::fs::copy(&original, &conflicted).unwrap();
        let paths = library.paths.clone();
        drop(library);

        let report = rebuild(&paths, &mut |_, _| {}).unwrap();

        assert_eq!(report.images, 2, "both real images still come back");
        assert_eq!(report.failed, 1);
        assert!(
            report.failures[0].file.contains("conflicted copy"),
            "the duplicate must be named: {:?}",
            report.failures
        );
        let rebuilt = Library::open_existing(paths.root.as_path()).unwrap();
        assert_eq!(
            ids_of(&rebuilt.conn, &everything(SearchView::Library)),
            vec!["a".to_string(), "b".to_string()]
        );
        assert_eq!(
            crate::ingest::require_record(&rebuilt.conn, "a")
                .unwrap()
                .tags,
            vec!["cat".to_string()],
            "the row that did land keeps its own tags, not a half-rolled-back set"
        );
    }

    /// The goal this change states — a folder whose `library.sqlite` can be
    /// thrown away and rebuilt — with nothing to move aside. The rebuild
    /// finishes rather than failing after all its work, and says so by naming
    /// no kept file. The journal still goes aside: a stale one left beside the
    /// database being renamed into place is what SQLite would roll back into
    /// it (design D10).
    #[test]
    fn a_folder_whose_database_is_gone_rebuilds_and_keeps_nothing_aside() {
        let (_dir, library) = library();
        store(&library, "a", &["cat"], None);
        let paths = library.paths.clone();
        drop(library);
        std::fs::remove_file(paths.db_path()).unwrap();
        std::fs::write(journal_path(&paths.db_path()), b"a stale hot journal").unwrap();

        let report = rebuild(&paths, &mut |_, _| {}).unwrap();

        assert_eq!(report.images, 1);
        assert_eq!(report.kept_as, "", "there was no database to keep");
        assert_eq!(
            siblings_starting_with(&paths.root, "library.sqlite-journal.corrupt-").len(),
            1,
            "the stale journal must not be left beside the rebuilt database"
        );
        let rebuilt = Library::open_existing(paths.root.as_path()).unwrap();
        assert_eq!(
            ids_of(&rebuilt.conn, &everything(SearchView::Library)),
            vec!["a".to_string()]
        );
    }

    /// A `library.json` that is there and will not parse is a failure named in
    /// the report: the rules, the sites and the note went with it, and a
    /// silent `0, 0` would let the user find that out the next time a rule did
    /// not fire. An absent one stays silent — a library from before this
    /// change simply has none.
    #[test]
    fn an_unreadable_library_file_is_named_in_the_report() {
        let (_dir, library) = library();
        store(&library, "a", &[], None);
        crate::notes::set(&library, "remember to tag these").unwrap();
        let paths = library.paths.clone();
        std::fs::write(sidecar::library_path(&paths), b"{ not valid json").unwrap();
        drop(library);

        let report = rebuild(&paths, &mut |_, _| {}).unwrap();

        assert_eq!(report.images, 1, "every image still comes back");
        assert_eq!(report.rules, 0);
        assert_eq!(
            report.collections, 1,
            "the migration's own seed stands (design D5) when the file cannot be read"
        );
        assert_eq!(report.failed, 1);
        assert!(
            report.failures[0].file.ends_with("library.json"),
            "the library file must be named: {:?}",
            report.failures
        );
    }

    /// Task 1.4 / design D5: a membership naming a collection the
    /// library-level file does not list — because it is missing entirely —
    /// comes back under a placeholder named by that id, rather than the
    /// membership being lost.
    #[test]
    fn a_rebuild_with_no_library_json_gives_a_placeholder_named_by_the_id() {
        let (_dir, library) = library();
        store(&library, "a", &[], None);
        let queue = collections::create(&library, "Queue").unwrap();
        collections::add(&library, &["a".to_string()], &queue.id).unwrap();
        let paths = library.paths.clone();
        std::fs::remove_file(sidecar::library_path(&paths)).unwrap();
        drop(library);

        let report = rebuild(&paths, &mut |_, _| {}).unwrap();

        assert_eq!(
            report.collections, 2,
            "the seed, standing, plus the placeholder for the lost collection"
        );
        let rebuilt = Library::open_existing(paths.root.as_path()).unwrap();
        let placeholder = collections::list(&rebuilt.conn)
            .unwrap()
            .into_iter()
            .find(|collection| collection.id == queue.id)
            .expect("the membership's collection id comes back as a placeholder");
        assert_eq!(
            placeholder.name, queue.id,
            "named by its id, so it can be renamed rather than lost"
        );
        assert_eq!(
            crate::ingest::require_record(&rebuilt.conn, "a")
                .unwrap()
                .collections,
            vec![queue.id],
        );
    }

    /// Task 1.4 / design D5: the library-level file is the truer answer —
    /// renaming `Favorites` before a rebuild keeps that name; the migration's
    /// own seed yields to it rather than the rebuilt library showing both.
    #[test]
    fn a_rebuild_whose_file_renamed_favorites_keeps_the_rename() {
        let (_dir, library) = library();
        store(&library, "a", &[], None);
        collections::rename(&library, "favorites", "Starred").unwrap();
        let paths = library.paths.clone();
        drop(library);

        let report = rebuild(&paths, &mut |_, _| {}).unwrap();

        assert_eq!(report.collections, 1);
        let rebuilt = Library::open_existing(paths.root.as_path()).unwrap();
        let names: Vec<String> = collections::list(&rebuilt.conn)
            .unwrap()
            .into_iter()
            .map(|collection| collection.name)
            .collect();
        assert_eq!(
            names,
            vec!["Starred".to_string()],
            "the seed's own name must not reappear beside the rename"
        );
    }

    /// Design D5, the third case: a `library.json` written before this change
    /// has no `collections` key at all and says nothing about them, so the
    /// migration's own seed stands — only a file that *lists* collections
    /// (`"collections": []` included, the user having deleted every one) is
    /// the answer that replaces it. Without the distinction an upgraded
    /// library that nothing has rewritten the file for loses `Favorites` to
    /// its own rebuild.
    #[test]
    fn a_library_json_from_before_collections_leaves_the_seed_standing() {
        let (_dir, library) = library();
        store(&library, "a", &[], None);
        crate::notes::set(&library, "written by the older build").unwrap();
        let paths = library.paths.clone();
        let file = sidecar::library_path(&paths);
        let mut json: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
        json.as_object_mut().unwrap().remove("collections");
        std::fs::write(&file, serde_json::to_vec_pretty(&json).unwrap()).unwrap();
        drop(library);

        let report = rebuild(&paths, &mut |_, _| {}).unwrap();

        assert_eq!(report.collections, 1);
        assert!(report.failures.is_empty(), "{:?}", report.failures);
        let rebuilt = Library::open_existing(paths.root.as_path()).unwrap();
        let favorites = collections::list(&rebuilt.conn).unwrap();
        assert_eq!(favorites[0].id, "favorites");
        assert_eq!(favorites[0].name, "Favorites");
    }

    /// Spec `collections`, "Deleted stays deleted", through a rebuild: the
    /// file lists no collection because the user deleted every one, and an
    /// empty list is an answer — the seed the new database was born with
    /// yields to it.
    #[test]
    fn a_library_json_listing_no_collections_removes_the_seed() {
        let (_dir, library) = library();
        store(&library, "a", &[], None);
        collections::delete(&library, "favorites").unwrap();
        let paths = library.paths.clone();
        drop(library);

        let report = rebuild(&paths, &mut |_, _| {}).unwrap();

        assert_eq!(report.collections, 0);
        let rebuilt = Library::open_existing(paths.root.as_path()).unwrap();
        assert!(collections::list(&rebuilt.conn).unwrap().is_empty());
    }

    /// A hand-edited sidecar naming one collection twice must not end the
    /// rebuild: the membership pass runs outside the per-sidecar savepoint,
    /// so a `PRIMARY KEY` failure there would take every image with it.
    #[test]
    fn a_sidecar_naming_one_collection_twice_still_rebuilds() {
        let (_dir, library) = library();
        store(&library, "a", &[], None);
        collections::add(&library, &["a".to_string()], "favorites").unwrap();
        let paths = library.paths.clone();
        let file = sidecar::path(&paths, "a");
        let mut json: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
        json.as_object_mut().unwrap().insert(
            "collections".to_string(),
            serde_json::json!(["favorites", "favorites"]),
        );
        std::fs::write(&file, serde_json::to_vec_pretty(&json).unwrap()).unwrap();
        drop(library);

        let report = rebuild(&paths, &mut |_, _| {}).unwrap();

        assert_eq!(report.images, 1);
        assert!(report.failures.is_empty(), "{:?}", report.failures);
        let rebuilt = Library::open_existing(paths.root.as_path()).unwrap();
        assert_eq!(
            crate::ingest::require_record(&rebuilt.conn, "a")
                .unwrap()
                .collections,
            vec!["favorites".to_string()],
        );
    }

    /// A flat library from before `sharded-image-dirs`: the sidecar and its
    /// image sit side by side under `images/` rather than in a bucket, and
    /// design D11's check is "is `<id>.<ext>` beside this sidecar". Asking
    /// `image_path` instead would call every image in such a folder missing.
    #[test]
    fn a_flat_layout_image_beside_its_sidecar_does_not_come_back_missing() {
        let (_dir, library) = library();
        store(&library, "a", &[], None);
        let paths = library.paths.clone();
        let flat_sidecar = paths.images_dir().join("a.json");
        let flat_image = paths.images_dir().join("a.png");
        std::fs::rename(sidecar::path(&paths, "a"), &flat_sidecar).unwrap();
        std::fs::rename(paths.image_path("a", "png"), &flat_image).unwrap();
        drop(library);

        rebuild(&paths, &mut |_, _| {}).unwrap();

        let conn = db::open(&paths.db_path()).unwrap();
        let missing: bool = conn
            .query_row("SELECT missing FROM images WHERE id = 'a'", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert!(
            !missing,
            "the image is beside its sidecar, so the row must not come back missing"
        );
    }

    /// Task 2.5: an image whose sidecar is there but whose image file is gone
    /// comes back as a row shown missing.
    #[test]
    fn a_sidecar_whose_image_file_is_gone_comes_back_missing() {
        let (_dir, library) = library();
        store(&library, "a", &[], None);
        std::fs::remove_file(library.paths.image_path("a", "png")).unwrap();
        let paths = library.paths.clone();
        drop(library);

        rebuild(&paths, &mut |_, _| {}).unwrap();

        let rebuilt = Library::open_existing(paths.root.as_path()).unwrap();
        let record = crate::ingest::require_record(&rebuilt.conn, "a").unwrap();
        assert!(record.missing);
    }

    /// Task 2.5: an image file with no sidecar creates no row and is left
    /// exactly where it is.
    #[test]
    fn an_image_with_no_sidecar_creates_no_row_and_is_not_moved() {
        let (_dir, library) = library();
        store(&library, "a", &[], None);
        let sidecar_path = sidecar::path(&library.paths, "a");
        std::fs::remove_file(&sidecar_path).unwrap();
        let image_path = library.paths.image_path("a", "png");
        let paths = library.paths.clone();
        drop(library);

        let report = rebuild(&paths, &mut |_, _| {}).unwrap();

        assert_eq!(report.images, 0);
        assert_eq!(report.failed, 0);
        assert!(
            image_path.is_file(),
            "the orphaned image file is left alone"
        );
        let rebuilt = Library::open_existing(paths.root.as_path()).unwrap();
        assert!(ids_of(&rebuilt.conn, &everything(SearchView::Library)).is_empty());
    }

    /// Task 2.5: not one image byte, thumbnail byte or modification time
    /// changes across a rebuild — it only ever reads `*.json` files.
    #[test]
    fn no_image_or_thumbnail_byte_or_mtime_changes_across_a_rebuild() {
        let (dir, library) = library();
        store(&library, "a", &["cat"], None);
        crate::thumbs::warm_thumbnail(
            &library.paths,
            &crate::ingest::require_record(&library.conn, "a").unwrap(),
        );
        let paths = library.paths.clone();
        let before = snapshot(dir.path());
        drop(library);

        rebuild(&paths, &mut |_, _| {}).unwrap();

        let after = snapshot(dir.path());
        for (path, before_meta) in &before {
            if path.to_string_lossy().contains("library.sqlite") {
                continue;
            }
            let after_meta = after
                .get(path)
                .unwrap_or_else(|| panic!("{path:?} vanished"));
            assert_eq!(
                after_meta, before_meta,
                "{path:?} changed bytes or mtime across a rebuild"
            );
        }
    }

    /// Task 2.5: a stale `.rebuilding` left by an interrupted attempt is
    /// overwritten rather than adopted — proven by seeding it with a row that
    /// must not survive.
    #[test]
    fn a_stale_rebuilding_file_is_overwritten_not_adopted() {
        let (dir, library) = library();
        store(&library, "a", &[], None);
        let paths = library.paths.clone();
        drop(library);

        let stale = building_path(&paths);
        let stale_conn = db::open(&stale).unwrap();
        stale_conn
            .execute(
                "INSERT INTO images (id, ext, mime, size, width, height, source, captured_at,
                                     created_at, updated_at)
                 VALUES ('ghost', 'png', 'image/png', 1, 1, 1, 'local', 0, 0, 0)",
                [],
            )
            .unwrap();
        drop(stale_conn);

        rebuild(&paths, &mut |_, _| {}).unwrap();

        let rebuilt = Library::open_existing(dir.path()).unwrap();
        let ids = ids_of(&rebuilt.conn, &everything(SearchView::Library));
        assert_eq!(ids, vec!["a".to_string()], "the stale row must not survive");
    }

    /// Every entry directly under `dir` whose filename starts with `prefix` —
    /// used below to find a moved journal without hand-reconstructing its
    /// name, since `.corrupt-<now>` lands after `-journal`, not after it
    /// (`library.sqlite-journal.corrupt-<now>`, not the other order).
    fn siblings_starting_with(dir: &Path, prefix: &str) -> Vec<PathBuf> {
        fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with(prefix))
            })
            .collect()
    }

    /// Task 2.3: both files move, and are still there afterwards; a second
    /// call leaves the first pair untouched and makes a second pair.
    #[test]
    fn move_aside_moves_the_database_and_its_journal_and_never_collides() {
        let (_dir, library) = library();
        let paths = library.paths.clone();
        drop(library);
        let journal = journal_path(&paths.db_path());
        std::fs::write(&journal, b"a hot journal").unwrap();

        let first_kept = move_aside(&paths)
            .unwrap()
            .expect("a database was there to keep");

        assert!(first_kept.is_file(), "the database must move, not vanish");
        let first_kept_journals =
            siblings_starting_with(&paths.root, "library.sqlite-journal.corrupt-");
        assert_eq!(
            first_kept_journals.len(),
            1,
            "the journal must move alongside the database it belongs to"
        );
        assert!(!paths.db_path().is_file());
        assert!(!journal.is_file());

        // A fresh pair, as a second rebuild attempt would leave behind before
        // calling `move_aside` again.
        std::thread::sleep(std::time::Duration::from_millis(2));
        drop(Library::open_or_create(&paths.root).unwrap());
        std::fs::write(journal_path(&paths.db_path()), b"a second hot journal").unwrap();

        let second_kept = move_aside(&paths)
            .unwrap()
            .expect("a second database was there to keep");

        assert_ne!(first_kept, second_kept);
        assert!(
            first_kept.is_file(),
            "the first kept database must survive untouched"
        );
        assert!(
            first_kept_journals[0].is_file(),
            "the first kept journal must survive untouched"
        );
        assert!(second_kept.is_file());
        assert_eq!(
            siblings_starting_with(&paths.root, "library.sqlite-journal.corrupt-").len(),
            2,
            "the first kept journal stays and a second sits beside it"
        );
    }

    fn snapshot(
        root: &Path,
    ) -> std::collections::HashMap<PathBuf, (u64, Option<std::time::SystemTime>)> {
        fn walk(
            dir: &Path,
            root: &Path,
            out: &mut std::collections::HashMap<PathBuf, (u64, Option<std::time::SystemTime>)>,
        ) {
            for entry in fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    walk(&path, root, out);
                } else {
                    let meta = fs::metadata(&path).unwrap();
                    out.insert(
                        path.strip_prefix(root).unwrap().to_path_buf(),
                        (meta.len(), meta.modified().ok()),
                    );
                }
            }
        }
        let mut out = std::collections::HashMap::new();
        walk(root, root, &mut out);
        out
    }
}
