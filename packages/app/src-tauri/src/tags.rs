//! The one tag write path, the rating write, and the autocomplete vocabulary.
//!
//! Every rule about what a tag edit *means* to the library lives here rather
//! than in the webview (§6, design D3): `bulk_update_tags` and the auto-tag
//! rules will write tags without ever passing through the editor, and each
//! would otherwise need its own copy of the rating rule and the orphan sweep.

use rusqlite::types::Value;
use rusqlite::{Connection, params, params_from_iter};

use crate::db;
use crate::error::{AppError, Result};
use crate::ingest;
use crate::library::Library;
use crate::model::{ImageRecord, TagCount};
use crate::query::placeholders;

/// The whole rating alphabet, in the order the controls show it. `images.rating`
/// is a nullable column, so "no rating" is the absence of one of these.
pub const RATINGS: [&str; 4] = ["g", "s", "q", "e"];

/// Replace the image's whole tag set (design D2) and answer with the row as it
/// now stands (design D10).
///
/// One transaction: a half-applied edit would leave the image carrying some of
/// the tags the user removed and some of the ones they added, with the sidebar
/// counting a set nobody asked for.
pub fn update_tags(library: &Library, id: &str, tags: &[String]) -> Result<ImageRecord> {
    let edit = TagEdit::read(tags);

    let tx = library.conn.unchecked_transaction()?;
    // First, so that an image that is not there refuses the edit before any tag
    // row is written; the transaction rolls the rest back on the way out.
    stamp(&tx, id, edit.rating.as_deref())?;
    let unlinked = unlink_tags_other_than(&tx, id, &edit.tags)?;
    for tag in &edit.tags {
        ingest::link_tag(&tx, id, tag)?;
    }
    collect_orphans(&tx, &unlinked)?;
    tx.commit()?;

    ingest::require_record(&library.conn, id)
}

/// Write `g`, `s`, `q`, `e` or nothing at all, and answer with the changed row.
///
/// The value originates in this app's own UI, so anything else is a bug here
/// rather than a foreign library's rating: storing it would put a value in the
/// library that no filter can select. `ImageRecord.rating` stays an
/// `Option<String>` for the other direction — ingest must not drop an image over
/// a rating it does not know (design D11).
pub fn set_rating(library: &Library, id: &str, rating: Option<&str>) -> Result<ImageRecord> {
    if let Some(value) = rating
        && !RATINGS.contains(&value)
    {
        return Err(AppError::BadRequest(format!("{value:?} is not a rating")));
    }

    let changed = library.conn.execute(
        "UPDATE images SET rating = ?1, updated_at = ?2 WHERE id = ?3",
        params![rating, db::now_ms(), id],
    )?;
    if changed == 0 {
        return Err(AppError::NotFound(format!("image {id}")));
    }

    ingest::require_record(&library.conn, id)
}

/// Tags beginning with `prefix`, most used first and then by name, at most
/// `limit` of them (design D12).
///
/// Ordering by usage rather than alphabetically is what makes a short list
/// useful: in an eight-row popover, alphabetical order buries the tag used daily
/// behind five used once. Ties break by name so the list is deterministic.
///
/// An inner join, so a tag no image carries cannot be offered — `collect_orphans`
/// already makes that unreachable, and a suggestion matching nothing is exactly
/// what design D5 keeps out of the table.
pub fn suggestions(conn: &Connection, prefix: &str, limit: i64) -> Result<Vec<TagCount>> {
    // LIKE is ASCII case-insensitive in SQLite, which is the case-insensitive
    // match the editor wants; the UNIQUE index on `tags.name` serves the prefix.
    let mut stmt = conn.prepare(
        r"SELECT tags.name, COUNT(*) AS uses
          FROM tags
          JOIN image_tags ON image_tags.tag_id = tags.id
          WHERE tags.name LIKE ?1 ESCAPE '\'
          GROUP BY tags.id
          ORDER BY uses DESC, tags.name
          LIMIT ?2",
    )?;
    let rows = stmt.query_map(params![like_prefix(prefix), limit], |row| {
        Ok(TagCount {
            name: row.get(0)?,
            count: row.get(1)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// The tags linked to `id`, for a caller about to take those links away.
pub fn tag_ids_of(conn: &Connection, image_id: &str) -> Result<Vec<i64>> {
    let mut stmt = conn.prepare("SELECT tag_id FROM image_tags WHERE image_id = ?1")?;
    let ids = stmt.query_map([image_id], |row| row.get(0))?;
    Ok(ids.collect::<rusqlite::Result<_>>()?)
}

/// Delete each of `tag_ids` that no `image_tags` row references any more, so a
/// row in `tags` always has at least one use (design D5).
///
/// Scoped to the ids a statement just unlinked and run inside that statement's
/// transaction, rather than as a sweep over the whole table: `tags` is the
/// autocomplete vocabulary and the sidebar's list, so an orphan is visible to
/// the user as a tag that suggests itself and matches nothing, and a sweep would
/// let that live until someone ran it.
///
/// The two later changes that inherit this must each decide: `trash`'s soft
/// delete SHALL NOT call it — a restored image keeps its tags, and its
/// `image_tags` rows survive the soft delete — while its `delete_forever` SHALL,
/// because the cascade takes the links without touching `tags`.
pub fn collect_orphans(conn: &Connection, tag_ids: &[i64]) -> Result<()> {
    if tag_ids.is_empty() {
        return Ok(());
    }
    conn.execute(
        &format!(
            "DELETE FROM tags
             WHERE id IN ({})
               AND NOT EXISTS (SELECT 1 FROM image_tags WHERE image_tags.tag_id = tags.id)",
            placeholders(tag_ids.len())
        ),
        params_from_iter(tag_ids.iter().copied().map(Value::Integer)),
    )?;
    Ok(())
}

/// What the editor's text means: the tags to store, and the rating a
/// `rating:g|s|q|e` among them named.
struct TagEdit {
    tags: Vec<String>,
    rating: Option<String>,
}

impl TagEdit {
    /// Blanks and repeats go: the editor is free text, and the set it names is
    /// what the user meant however many times they spelled a tag.
    fn read(tags: &[String]) -> TagEdit {
        let mut edit = TagEdit {
            tags: Vec::new(),
            rating: None,
        };
        for raw in tags {
            let tag = raw.trim();
            if tag.is_empty() {
                continue;
            }
            match rating_metatag(tag) {
                Some(rating) => edit.rating = Some(rating),
                None if edit.tags.iter().any(|kept| kept == tag) => {}
                None => edit.tags.push(tag.to_string()),
            }
        }
        edit
    }
}

/// The rating a `rating:g|s|q|e` names, which is set instead of being stored as
/// a tag (design D3). Anything else after `rating:` is an ordinary tag: the
/// metatag alphabet is `g|s|q|e`, and silently dropping `rating:unknown` would
/// lose a tag the user typed.
///
/// Case-insensitive because `parseTagSearch` reads the same metatag with an `i`
/// flag: a `Rating:S` stored as a tag would be a tag no search for it can find.
fn rating_metatag(tag: &str) -> Option<String> {
    let value = tag
        .get(..RATING_PREFIX.len())
        .filter(|prefix| prefix.eq_ignore_ascii_case(RATING_PREFIX))
        .map(|prefix| tag[prefix.len()..].to_ascii_lowercase())?;
    RATINGS.contains(&value.as_str()).then_some(value)
}

const RATING_PREFIX: &str = "rating:";

/// Record the edit against the image, and refuse one that names no image.
///
/// A missing rating token leaves the rating alone rather than clearing it: the
/// tag box has no way to spell "take the rating away", which is what
/// `set_rating(None)` is for.
///
/// `updated_at` moves on every edit, or "sort by last change" — one of the four
/// sorts — would be a lie (design D9).
fn stamp(conn: &Connection, id: &str, rating: Option<&str>) -> Result<()> {
    let now = db::now_ms();
    let changed = match rating {
        Some(rating) => conn.execute(
            "UPDATE images SET rating = ?1, updated_at = ?2 WHERE id = ?3",
            params![rating, now, id],
        )?,
        None => conn.execute(
            "UPDATE images SET updated_at = ?1 WHERE id = ?2",
            params![now, id],
        )?,
    };
    if changed == 0 {
        return Err(AppError::NotFound(format!("image {id}")));
    }
    Ok(())
}

/// Drop this image's links to every tag not in `keep`, and answer with the tag
/// ids that lost one — the only ids `collect_orphans` has to look at.
///
/// The select and the delete share one clause and one parameter list: two
/// spellings of "the links being taken away" could disagree, and the difference
/// would be a tag row nothing ever collects.
fn unlink_tags_other_than(conn: &Connection, id: &str, keep: &[String]) -> Result<Vec<i64>> {
    let kept = if keep.is_empty() {
        String::new()
    } else {
        format!(
            " AND tag_id NOT IN (SELECT id FROM tags WHERE name IN ({}))",
            placeholders(keep.len())
        )
    };
    let mut values: Vec<Value> = vec![Value::Text(id.to_string())];
    values.extend(keep.iter().cloned().map(Value::Text));

    let mut stmt = conn.prepare(&format!(
        "SELECT tag_id FROM image_tags WHERE image_id = ?1{kept}"
    ))?;
    let unlinked: Vec<i64> = stmt
        .query_map(params_from_iter(&values), |row| row.get(0))?
        .collect::<rusqlite::Result<_>>()?;

    conn.execute(
        &format!("DELETE FROM image_tags WHERE image_id = ?1{kept}"),
        params_from_iter(&values),
    )?;
    Ok(unlinked)
}

/// `%`, `_` and the escape character are LIKE syntax; a typed prefix is data.
fn like_prefix(prefix: &str) -> String {
    let escaped = prefix
        .replace('\\', r"\\")
        .replace('%', r"\%")
        .replace('_', r"\_");
    format!("{escaped}%")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingest::{IngestInput, store_image};
    use crate::model::{ImageSource, ParsedTagSearch, SearchRequest};
    use crate::query;

    fn png_bytes() -> Vec<u8> {
        let image = image::DynamicImage::ImageRgb8(image::RgbImage::new(4, 4));
        let mut out = std::io::Cursor::new(Vec::new());
        image.write_to(&mut out, image::ImageFormat::Png).unwrap();
        out.into_inner()
    }

    /// Rows enter through `ingest::store_image`, the one write path (design D4),
    /// so a fixture cannot drift from what the app actually stores.
    fn store(library: &Library, id: &str, rating: Option<&str>, tags: &[&str]) {
        let tags: Vec<String> = tags.iter().map(|tag| (*tag).to_string()).collect();
        store_image(
            library,
            IngestInput {
                id,
                bytes: &png_bytes(),
                source: ImageSource::Extension,
                source_ref: Some("x"),
                image_url: None,
                page_url: Some("https://x.com/alice/status/1"),
                page_title: Some("a page"),
                adapter: None,
                rating,
                tags: &tags,
                captured_at: 1_700_000_000_000,
            },
        )
        .unwrap();
    }

    fn library() -> (tempfile::TempDir, Library) {
        let dir = tempfile::tempdir().unwrap();
        let library = Library::open_or_create(dir.path()).unwrap();
        (dir, library)
    }

    fn edit(library: &Library, id: &str, tags: &[&str]) -> ImageRecord {
        let tags: Vec<String> = tags.iter().map(|tag| (*tag).to_string()).collect();
        update_tags(library, id, &tags).unwrap()
    }

    fn tag_names(library: &Library) -> Vec<String> {
        let mut stmt = library
            .conn
            .prepare("SELECT name FROM tags ORDER BY name")
            .unwrap();
        let names = stmt.query_map([], |row| row.get::<_, String>(0)).unwrap();
        names.map(std::result::Result::unwrap).collect()
    }

    fn rating_of(library: &Library, id: &str) -> Option<String> {
        library
            .conn
            .query_row("SELECT rating FROM images WHERE id = ?1", [id], |row| {
                row.get(0)
            })
            .unwrap()
    }

    #[test]
    fn tags_are_added_to_an_image_that_had_none() {
        let (_dir, library) = library();
        store(&library, "a", None, &[]);

        let record = edit(&library, "a", &["cat", "cute"]);

        assert_eq!(record.tags, vec!["cat".to_string(), "cute".to_string()]);
    }

    #[test]
    fn an_edit_replaces_the_whole_set_rather_than_adding_to_it() {
        let (_dir, library) = library();
        store(&library, "a", None, &["cat", "dog"]);

        let record = edit(&library, "a", &["cat", "bird"]);

        assert_eq!(record.tags, vec!["bird".to_string(), "cat".to_string()]);
    }

    #[test]
    fn an_empty_editor_leaves_the_image_with_no_tags_and_in_the_library() {
        let (_dir, library) = library();
        store(&library, "a", None, &["cat"]);

        let record = edit(&library, "a", &[]);

        assert!(record.tags.is_empty());
        assert_eq!(library.image_count().unwrap(), 1);
    }

    #[test]
    fn a_tag_named_twice_is_stored_once_and_blanks_are_dropped() {
        let (_dir, library) = library();
        store(&library, "a", None, &[]);

        let record = edit(&library, "a", &["cat", "cat", "", "  ", " dog "]);

        assert_eq!(record.tags, vec!["cat".to_string(), "dog".to_string()]);
    }

    #[test]
    fn a_rating_metatag_sets_the_rating_and_is_not_stored_as_a_tag() {
        let (_dir, library) = library();
        store(&library, "a", None, &[]);

        let record = edit(&library, "a", &["cat", "rating:s"]);

        assert_eq!(record.tags, vec!["cat".to_string()]);
        assert_eq!(record.rating.as_deref(), Some("s"));
        assert!(!tag_names(&library).contains(&"rating:s".to_string()));
    }

    #[test]
    fn a_rating_the_alphabet_does_not_know_stays_an_ordinary_tag() {
        let (_dir, library) = library();
        store(&library, "a", Some("q"), &[]);

        let record = edit(&library, "a", &["rating:unknown"]);

        assert_eq!(record.tags, vec!["rating:unknown".to_string()]);
        assert_eq!(
            record.rating.as_deref(),
            Some("q"),
            "the rating is untouched"
        );
    }

    #[test]
    fn a_tag_another_image_still_carries_survives_the_unlink() {
        let (_dir, library) = library();
        store(&library, "a", None, &["cat", "cute"]);
        store(&library, "b", None, &["cat"]);

        edit(&library, "a", &["cute"]);

        assert_eq!(
            tag_names(&library),
            vec!["cat".to_string(), "cute".to_string()],
            "`cat` lost one use, not its last one",
        );
        assert_eq!(
            ingest::require_record(&library.conn, "b").unwrap().tags,
            vec!["cat".to_string()],
            "the other image keeps the tag it was never asked about",
        );
    }

    #[test]
    fn a_tag_whose_last_use_was_removed_leaves_the_table() {
        let (_dir, library) = library();
        store(&library, "a", None, &["cat", "solo"]);

        edit(&library, "a", &["cat"]);

        assert_eq!(tag_names(&library), vec!["cat".to_string()]);
    }

    #[test]
    fn an_edit_stamps_the_image_as_changed() {
        let (_dir, library) = library();
        store(&library, "a", None, &[]);
        let before = ingest::require_record(&library.conn, "a").unwrap();

        let record = edit(&library, "a", &["cat"]);

        assert!(
            record.updated_at > before.updated_at,
            "sort-by-last-change reads this column (design D9)",
        );
    }

    #[test]
    fn an_edit_for_an_image_that_is_not_there_changes_nothing() {
        let (_dir, library) = library();
        store(&library, "a", None, &["cat"]);

        let error = update_tags(&library, "nobody", &["dog".to_string()]).unwrap_err();

        assert!(matches!(error, AppError::NotFound(_)), "got {error}");
        assert_eq!(tag_names(&library), vec!["cat".to_string()]);
    }

    fn found(library: &Library, query: ParsedTagSearch) -> Vec<String> {
        query::search(
            &library.conn,
            &SearchRequest {
                query,
                text: String::new(),
                include_deleted: false,
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

    #[test]
    fn every_rating_and_the_absence_of_one_round_trips_through_a_search() {
        let (_dir, library) = library();
        store(&library, "a", None, &[]);

        for rating in RATINGS {
            let record = set_rating(&library, "a", Some(rating)).unwrap();

            assert_eq!(record.rating.as_deref(), Some(rating));
            assert_eq!(
                found(
                    &library,
                    ParsedTagSearch {
                        ratings: vec![rating.to_string()],
                        ..Default::default()
                    }
                ),
                vec!["a".to_string()],
            );
        }

        let cleared = set_rating(&library, "a", None).unwrap();

        assert_eq!(cleared.rating, None);
        assert_eq!(
            found(
                &library,
                ParsedTagSearch {
                    include_unrated: true,
                    ..Default::default()
                }
            ),
            vec!["a".to_string()],
        );
    }

    #[test]
    fn a_value_that_is_not_a_rating_is_refused_and_the_row_stands() {
        let (_dir, library) = library();
        store(&library, "a", Some("s"), &[]);
        let before = ingest::require_record(&library.conn, "a").unwrap();

        for value in ["safe", "S", "", "general"] {
            let error = set_rating(&library, "a", Some(value)).unwrap_err();
            assert!(matches!(error, AppError::BadRequest(_)), "got {error}");
        }

        assert_eq!(rating_of(&library, "a").as_deref(), Some("s"));
        assert_eq!(
            ingest::require_record(&library.conn, "a")
                .unwrap()
                .updated_at,
            before.updated_at,
            "a refused rating must not stamp the row either",
        );
    }

    #[test]
    fn a_rating_for_an_image_that_is_not_there_says_so() {
        let (_dir, library) = library();

        let error = set_rating(&library, "nobody", Some("s")).unwrap_err();

        assert!(matches!(error, AppError::NotFound(_)), "got {error}");
    }

    #[test]
    fn setting_a_rating_stamps_the_image_as_changed() {
        let (_dir, library) = library();
        store(&library, "a", None, &[]);
        let before = ingest::require_record(&library.conn, "a").unwrap();

        let record = set_rating(&library, "a", Some("e")).unwrap();

        assert!(record.updated_at > before.updated_at);
    }

    /// `cat` on three images, `cathedral` on two, `catalogue` on one, `dog` on
    /// one — a usage order that is not the alphabetical one, so the two orders
    /// cannot be confused.
    fn vocabulary() -> (tempfile::TempDir, Library) {
        let (dir, library) = library();
        store(
            &library,
            "a",
            None,
            &["cat", "cathedral", "catalogue", "dog"],
        );
        store(&library, "b", None, &["cat", "cathedral"]);
        store(&library, "c", None, &["cat"]);
        (dir, library)
    }

    fn suggested(library: &Library, prefix: &str, limit: i64) -> Vec<(String, i64)> {
        suggestions(&library.conn, prefix, limit)
            .unwrap()
            .into_iter()
            .map(|tag| (tag.name, tag.count))
            .collect()
    }

    #[test]
    fn a_prefix_lists_the_tags_that_start_with_it_most_used_first() {
        let (_dir, library) = vocabulary();

        assert_eq!(
            suggested(&library, "cat", 8),
            vec![
                ("cat".to_string(), 3),
                ("cathedral".to_string(), 2),
                ("catalogue".to_string(), 1),
            ],
        );
    }

    #[test]
    fn a_prefix_nothing_starts_with_lists_nothing() {
        let (_dir, library) = vocabulary();

        assert!(suggested(&library, "zebra", 8).is_empty());
    }

    #[test]
    fn an_empty_prefix_lists_the_most_used_tags() {
        let (_dir, library) = vocabulary();

        let names: Vec<String> = suggested(&library, "", 2)
            .into_iter()
            .map(|(name, _)| name)
            .collect();

        assert_eq!(names, vec!["cat".to_string(), "cathedral".to_string()]);
    }

    #[test]
    fn a_prefix_matches_whatever_case_the_tag_was_stored_in() {
        let (_dir, library) = library();
        store(&library, "a", None, &["Cathedral"]);

        assert_eq!(
            suggested(&library, "cat", 8),
            vec![("Cathedral".to_string(), 1)]
        );
    }

    #[test]
    fn a_tag_no_image_carries_any_more_is_never_suggested() {
        let (_dir, library) = library();
        store(&library, "a", None, &["cathedral"]);

        edit(&library, "a", &[]);

        assert!(suggested(&library, "cat", 8).is_empty());
    }

    #[test]
    fn a_prefix_that_looks_like_like_syntax_is_only_ever_a_value() {
        let (_dir, library) = library();
        store(&library, "a", None, &["cat", "100%_wool"]);

        assert!(
            suggested(&library, "%", 8).is_empty(),
            "a bare wildcard must match nothing, not everything",
        );
        assert_eq!(
            suggested(&library, "100%_", 8),
            vec![("100%_wool".to_string(), 1)]
        );
    }
}
