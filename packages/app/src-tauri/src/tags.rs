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
use crate::model::{ImageRecord, TagCategory, TagCount, TagEntry};
use crate::query::{ID_CHUNK, placeholders};

/// Ensure the tag row exists — general, unless [`link_one_tag`] already gave
/// it a category before calling this — and link the image to it. The plain
/// half of tag creation: `link_one_tag` is where a category is ever named at
/// creation (`tag-vocabulary` design D4), and the rebuild's own sidecar pass
/// (`recover::insert_sidecar`) calls straight here, since a sidecar carries
/// tag names only and the vocabulary is restored onto these rows afterwards.
pub fn link_tag(conn: &Connection, image_id: &str, tag: &str) -> Result<()> {
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

/// The whole rating alphabet, in the order the controls show it. `images.rating`
/// is a nullable column, so "no rating" is the absence of one of these.
pub const RATINGS: [&str; 4] = ["g", "s", "q", "e"];

/// Replace the image's whole tag set (design D2) and answer with the row as it
/// now stands (design D10). A category prefix in `tags` creates a tag under
/// that category (`tag-vocabulary` design D4); a prefix naming a tag that
/// already exists under a different category refuses the whole edit
/// (`Conflict::Refuse`) — the text came from the user's own hands, and the
/// refusal is the answer they need to pick another name.
///
/// One transaction: a half-applied edit would leave the image carrying some of
/// the tags the user removed and some of the ones they added, with the sidebar
/// counting a set nobody asked for.
pub fn update_tags(library: &Library, id: &str, tags: &[String]) -> Result<ImageRecord> {
    let text = read_metatags(tags);

    let tx = library.conn.unchecked_transaction()?;
    // First, so that an image that is not there refuses the edit before any tag
    // row is written; the transaction rolls the rest back on the way out.
    stamp(&tx, id, text.rating.as_deref())?;
    let unlinked = unlink_tags_other_than(&tx, id, &text.tags)?;
    let categorised = link_tags(&tx, id, &text, Conflict::Refuse)?;
    collect_orphans(&tx, &unlinked)?;
    tx.commit()?;

    // After the commit, never inside it (`library-sidecars` design D4).
    crate::sidecar::write_one(&library.paths, &library.conn, id)?;
    // Only when a categorised row was born (design D2): a plain save touches
    // no row of `library.json`'s exceptions list, so it costs nothing more.
    if categorised {
        crate::sidecar::write_library(&library.paths, &library.conn)?;
    }
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

    crate::sidecar::write_one(&library.paths, &library.conn, id)?;
    ingest::require_record(&library.conn, id)
}

/// One tag edit across every id in `ids`, in a single transaction
/// (`selection-and-bulk` design D10): every id gets `add` linked and `remove`
/// unlinked, orphans are collected once at the end, and the whole thing commits
/// together or not at all — an id partway through a large selection failing
/// must not leave the ones before it edited and the ones after it untouched.
///
/// Reuses [`link_tags`], [`remove_tags`] and `collect_orphans` rather than
/// looping `update_tags`: that command replaces an image's *whole* tag set from
/// free text, which is not what a bulk add/remove means, and a loop of it would
/// also be one transaction per image (design D10's "not a loop over the
/// single-image command"). `add` is read once for the whole call (design D4):
/// a category prefix in it creates a tag the same way the editor's does, and a
/// conflicting one refuses the whole edit before any id is touched.
pub fn bulk_update_tags(
    library: &Library,
    ids: &[String],
    add: &[String],
    remove: &[String],
) -> Result<()> {
    let text = read_metatags(add);

    let tx = library.conn.unchecked_transaction()?;
    let mut unlinked = Vec::new();
    let mut categorised = false;
    for id in ids {
        // First, so an id with no row fails the whole call before any tag link
        // changes — the transaction rolls every earlier id back on the way out.
        stamp(&tx, id, None)?;
        unlinked.extend(remove_tags(&tx, id, remove)?);
        categorised |= link_tags(&tx, id, &text, Conflict::Refuse)?;
    }
    collect_orphans(&tx, &unlinked)?;
    tx.commit()?;

    crate::sidecar::write_for(&library.paths, &library.conn, ids)?;
    if categorised {
        crate::sidecar::write_library(&library.paths, &library.conn)?;
    }
    Ok(())
}

/// Link `id` to each of `tags`, creating a plain, general `tags` row for one
/// not seen before, through [`link_tag`]. Used where the tags at hand are
/// already resolved names rather than raw editor text — a test fixture, the
/// export module's own — never a caller reading a category prefix; that goes
/// through [`link_tags`] instead.
pub fn add_tags(conn: &Connection, id: &str, tags: &[String]) -> Result<()> {
    for tag in tags {
        link_tag(conn, id, tag)?;
    }
    Ok(())
}

/// Unlink `id` from each of `tags`, and answer with the tag ids that lost this
/// use — the only ones `collect_orphans` need look at. Unlinking a tag `id`
/// does not carry is a no-op, exactly as removing one from the single-image
/// editor is.
pub fn remove_tags(conn: &Connection, id: &str, tags: &[String]) -> Result<Vec<i64>> {
    if tags.is_empty() {
        return Ok(Vec::new());
    }
    let mut values: Vec<Value> = vec![Value::Text(id.to_string())];
    values.extend(tags.iter().cloned().map(Value::Text));
    let names = placeholders(tags.len());

    let mut stmt = conn.prepare(&format!(
        "SELECT image_tags.tag_id FROM image_tags
         JOIN tags ON tags.id = image_tags.tag_id
         WHERE image_tags.image_id = ?1 AND tags.name IN ({names})"
    ))?;
    let unlinked: Vec<i64> = stmt
        .query_map(params_from_iter(&values), |row| row.get(0))?
        .collect::<rusqlite::Result<_>>()?;

    conn.execute(
        &format!(
            "DELETE FROM image_tags
             WHERE image_id = ?1
               AND tag_id IN (SELECT id FROM tags WHERE name IN ({names}))"
        ),
        params_from_iter(&values),
    )?;
    Ok(unlinked)
}

/// One rating across every id in `ids` (`selection-and-bulk` design D10)
/// rather than `set_rating` looped: the whole selection changes together, so
/// there is no point at which it is carrying two ratings. `None` clears every
/// id to unrated, exactly as `set_rating(None)` does for one image.
///
/// Chunked to [`ID_CHUNK`] ids per `UPDATE … WHERE id IN (…)`, inside one
/// transaction: SQLite's bound-parameter limit makes one statement per id in
/// the selection a real failure mode once a selection spans a whole large
/// library ("too many SQL variables"), and the transaction is what keeps a
/// selection split across statements changing together or not at all.
pub fn bulk_set_rating(library: &Library, ids: &[String], rating: Option<&str>) -> Result<()> {
    if let Some(value) = rating
        && !RATINGS.contains(&value)
    {
        return Err(AppError::BadRequest(format!("{value:?} is not a rating")));
    }
    if ids.is_empty() {
        return Ok(());
    }

    let now = db::now_ms();
    let tx = library.conn.unchecked_transaction()?;
    for chunk in ids.chunks(ID_CHUNK) {
        let mut values: Vec<Value> = vec![
            rating.map_or(Value::Null, |value| Value::Text(value.to_string())),
            Value::Integer(now),
        ];
        values.extend(chunk.iter().cloned().map(Value::Text));
        tx.execute(
            &format!(
                "UPDATE images SET rating = ?1, updated_at = ?2 WHERE id IN ({})",
                placeholders(chunk.len())
            ),
            params_from_iter(&values),
        )?;
    }
    tx.commit()?;

    crate::sidecar::write_for(&library.paths, &library.conn, ids)?;
    Ok(())
}

/// The `limit` tags most common among `ids`, with how many of them carry each
/// — the bulk tag dialog's quick-remove pills (`selection-and-bulk` design D9)
/// and, with `names` given, the pinned chips' tri-state (`tag-vocabulary`
/// design D8): the counts of exactly the pinned tags over the described
/// selection, a filter rather than a second command so the two callers share
/// the one chunked query. `names` empty answers no counts at all — nothing was
/// asked for — rather than every tag in the selection. `limit` is ignored when
/// `names` is given: `names` already bounds the answer to exactly the tags
/// asked for, and truncating it besides could cut a pinned tag off a list the
/// caller asked for by name rather than by rank.
///
/// Chunked to [`ID_CHUNK`] ids per statement, for the same reason as
/// `bulk_set_rating`: one `IN (…)` over the whole selection is a real "too
/// many SQL variables" failure once a selection spans a whole large library.
/// Each chunk's counts are summed in memory rather than in one `GROUP BY`
/// over every id, which is what a single statement cannot do split across
/// several — the sort applies once, to the merged totals, so the answer is
/// the same whichever chunk a given id landed in.
pub fn selection_tag_counts(
    conn: &Connection,
    ids: &[String],
    limit: i64,
    names: Option<&[String]>,
) -> Result<Vec<TagCount>> {
    if ids.is_empty() || names.is_some_and(<[String]>::is_empty) {
        return Ok(Vec::new());
    }

    let mut totals: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    for chunk in ids.chunks(ID_CHUNK) {
        let mut sql = format!(
            "SELECT tags.name, COUNT(*) AS carriers
             FROM image_tags
             JOIN tags ON tags.id = image_tags.tag_id
             WHERE image_tags.image_id IN ({})",
            placeholders(chunk.len())
        );
        let mut values = text_values(chunk);
        if let Some(names) = names {
            sql.push_str(&format!(
                " AND tags.name IN ({})",
                placeholders(names.len())
            ));
            values.extend(text_values(names));
        }
        sql.push_str(" GROUP BY image_tags.tag_id");

        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params_from_iter(values), |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;
        for row in rows {
            let (name, count) = row?;
            *totals.entry(name).or_insert(0) += count;
        }
    }

    let mut counts: Vec<TagCount> = totals
        .into_iter()
        .map(|(name, count)| TagCount { name, count })
        .collect();
    counts.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.name.cmp(&b.name)));
    // A negative `limit` means "no limit", the same convention the
    // single-statement query relied on by passing `limit` straight through.
    // `names` given is its own bound (see the doc comment above), so `limit`
    // does not additionally truncate it.
    if names.is_none() && limit >= 0 {
        counts.truncate(limit as usize);
    }
    Ok(counts)
}

fn text_values(items: &[String]) -> Vec<Value> {
    items.iter().cloned().map(Value::Text).collect()
}

/// Tags beginning with `prefix`, most used first and then by name, at most
/// `limit` of them (design D12).
///
/// Ordering by usage rather than alphabetically is what makes a short list
/// useful: in an eight-row popover, alphabetical order buries the tag used daily
/// behind five used once. Ties break by name so the list is deterministic.
///
/// A `LEFT JOIN` (`tag-vocabulary` design D3): a categorised or pinned tag can
/// outlive every image that carried it (`collect_orphans` spares it), so an
/// inner join would drop such a tag from suggestions entirely; the left join
/// offers it at count 0 instead. `COUNT(image_tags.tag_id)`, not `COUNT(*)`:
/// the left join still produces one row for a carrier-less tag, with every
/// `image_tags` column `NULL`, and `COUNT(*)` would count that row as one use
/// rather than zero.
pub fn suggestions(conn: &Connection, prefix: &str, limit: i64) -> Result<Vec<TagCount>> {
    // LIKE is ASCII case-insensitive in SQLite, which is the case-insensitive
    // match the editor wants; with an ESCAPE clause and BINARY collation the
    // UNIQUE index on `tags.name` cannot serve it, so this is a scan of
    // `tags`, bounded by the vocabulary size rather than sped up by an index.
    let mut stmt = conn.prepare(
        r"SELECT tags.name, COUNT(image_tags.tag_id) AS uses
          FROM tags
          LEFT JOIN image_tags ON image_tags.tag_id = tags.id
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

/// Delete each of `tag_ids` that no `image_tags` row references any more and
/// that carries no vocabulary the user built (design D5, narrowed by
/// `tag-vocabulary` design D3): a categorised or pinned tag survives having no
/// carrier, since a tag someone filed as an artist or pinned is kept
/// deliberately, not as a by-product of one image — losing it with the
/// image's last trash-and-delete would make the artist's next capture a
/// general tag again.
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
               AND category = 'general' AND pinned = 0
               AND NOT EXISTS (SELECT 1 FROM image_tags WHERE image_tags.tag_id = tags.id)",
            placeholders(tag_ids.len())
        ),
        params_from_iter(tag_ids.iter().copied().map(Value::Integer)),
    )?;
    Ok(())
}

/// What a save's raw tag text means, one reading for the editor, the bulk add
/// list and a rule's tags alike (`tag-vocabulary` design D4): the plain tags
/// to store, the rating a `rating:g|s|q|e` among them named, and the category
/// a prefix named for whichever of `tags` it introduced.
pub struct TagText {
    pub tags: Vec<String>,
    pub rating: Option<String>,
    pub categories: Vec<(String, TagCategory)>,
}

/// Read `tags` into [`TagText`]: the category prefix reads at the same pass
/// over the same tokens as the rating metatag, not a second walk over the
/// list. Called by the tag editor's write path (`update_tags`), by
/// `bulk_update_tags`'s add list, and by ingest's and the auto-tag rules' own
/// application of a rule's tags, so a rule's `artist:cat` or `rating:e` and
/// one typed into the editor mean the same thing.
///
/// Blanks and repeats go: whatever built `tags` is free text or the union of
/// an image's own tags with a rule's, and the set this names is what was
/// meant however many times a tag occurs in it. A token with an empty name
/// after its category prefix is dropped the same way (design D4).
pub fn read_metatags(tags: &[String]) -> TagText {
    let mut kept: Vec<String> = Vec::new();
    let mut rating = None;
    let mut categories: Vec<(String, TagCategory)> = Vec::new();
    for raw in tags {
        let tag = raw.trim();
        if tag.is_empty() {
            continue;
        }
        if let Some(value) = rating_metatag(tag) {
            rating = Some(value);
            continue;
        }
        let (name, category) = match category_metatag(tag) {
            Some((category, name)) if !name.is_empty() => (name, Some(category)),
            Some(_) => continue,
            None => (tag, None),
        };
        if !kept.iter().any(|seen| seen == name) {
            kept.push(name.to_string());
        }
        // First-wins, independent of the `kept` dedup above: a plain `cat`
        // ahead of a rule's `artist:cat` must still name the category, or a
        // capture whose source tags already carry a name a rule also tags
        // under a prefix would silently keep that name general forever.
        if let Some(category) = category
            && !categories.iter().any(|(seen, _)| seen == name)
        {
            categories.push((name.to_string(), category));
        }
    }
    TagText {
        tags: kept,
        rating,
        categories,
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

/// The category a token's prefix names and the name after it, or `None` for a
/// token with none (`tag-vocabulary` design D4). Long form first, Danbooru's
/// own short forms beside it — `art:`, `copy:`, `char:`, `gen:` — since the
/// owner types Danbooru; `meta:` has no short form of its own, the same
/// alphabet Danbooru uses. Every prefix ends in `:`, so none is a prefix of
/// another and checking them in any order finds the same, and only, match.
const CATEGORY_PREFIXES: [(&str, TagCategory); 9] = [
    ("artist:", TagCategory::Artist),
    ("art:", TagCategory::Artist),
    ("copyright:", TagCategory::Copyright),
    ("copy:", TagCategory::Copyright),
    ("character:", TagCategory::Character),
    ("char:", TagCategory::Character),
    ("meta:", TagCategory::Meta),
    ("general:", TagCategory::General),
    ("gen:", TagCategory::General),
];

fn category_metatag(tag: &str) -> Option<(TagCategory, &str)> {
    CATEGORY_PREFIXES.iter().find_map(|(prefix, category)| {
        tag.get(..prefix.len())
            .filter(|candidate| candidate.eq_ignore_ascii_case(prefix))
            .map(|_| (*category, tag[prefix.len()..].trim()))
    })
}

/// What [`link_tags`] does when a token's category disagrees with the tag's
/// stored one (`tag-vocabulary` design D4): `Refuse` for the editor and the
/// bulk add list — text that came from the user's own hands, where the
/// refusal is the answer they need to pick another name — `Keep` for a rule's
/// tags, at capture time and on a run over stored images alike, so a
/// capture's success or a run's progress never depends on a rule's spelling
/// matching what the tag already is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Conflict {
    Refuse,
    Keep,
}

/// Link `image_id` to every tag in `text.tags`, creating a row under its named
/// category for one not seen before, and settling every category `text`
/// named against whatever category the tag already has (`tag-vocabulary`
/// design D4). `update_tags`, `bulk_update_tags` and the rule application
/// (`ingest::insert_rows`, `rules::apply_rules_to_image`) all go through here,
/// so a categorised tag is created identically everywhere it can be created.
///
/// `text.categories` reaching here with the same name twice cannot happen
/// through [`read_metatags`], which already keeps only the first pair per
/// name; the lookup below stays first-wins on its own rather than trusting
/// that, since a `TagText` can also be built by hand (`rules.rs`'s own
/// `apply_rules_to_image`). The first prefix for a name wins and a second one
/// is ignored, not refused, because the spec is silent on the case and
/// refusing would block a save over a typo.
///
/// Answers whether a categorised row was born — `true` only when at least one
/// of `text.categories`' tags did not already exist — which is what tells a
/// caller whether `library.json` needs rewriting (design D2): a plain save
/// changes no row of its exceptions list, so it costs nothing more than it
/// already did.
pub fn link_tags(
    conn: &Connection,
    image_id: &str,
    text: &TagText,
    policy: Conflict,
) -> Result<bool> {
    let mut named: std::collections::HashMap<&str, TagCategory> = std::collections::HashMap::new();
    for (name, category) in &text.categories {
        named.entry(name.as_str()).or_insert(*category);
    }
    let mut categorised = false;
    for tag in &text.tags {
        categorised |= link_one_tag(
            conn,
            image_id,
            tag,
            named.get(tag.as_str()).copied(),
            policy,
        )?;
    }
    Ok(categorised)
}

/// One tag of [`link_tags`]: create the row under `category` when this token
/// named one and no row exists yet, settle a category disagreement by
/// `policy` when one does, then link the image through [`link_tag`] either
/// way. Answers whether the row was created under a *non-general* category
/// here — the fact [`link_tags`] sums into its own answer.
fn link_one_tag(
    conn: &Connection,
    image_id: &str,
    tag: &str,
    category: Option<TagCategory>,
    policy: Conflict,
) -> Result<bool> {
    let mut created_categorised = false;
    if let Some(category) = category {
        match existing_tag_category(conn, tag)? {
            None => {
                conn.execute(
                    "INSERT INTO tags (name, category) VALUES (?1, ?2)",
                    params![tag, category],
                )?;
                // `general` is the default every plain-created row already
                // gets, so a `general:x` token is not an exception
                // `library.json` has to carry (design D2) — only a row born
                // under another category needs the file rewritten.
                created_categorised = category != TagCategory::General;
            }
            Some(existing) if existing != category => match policy {
                Conflict::Refuse => return Err(category_conflict(tag, existing, category)),
                Conflict::Keep => {}
            },
            Some(_) => {}
        }
    }
    link_tag(conn, image_id, tag)?;
    Ok(created_categorised)
}

fn existing_tag_category(conn: &Connection, tag: &str) -> Result<Option<TagCategory>> {
    match conn.query_row("SELECT category FROM tags WHERE name = ?1", [tag], |row| {
        row.get(0)
    }) {
        Ok(category) => Ok(Some(category)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(error) => Err(error.into()),
    }
}

/// The one message `Conflict::Refuse` answers with (`tag-vocabulary` design
/// D4): names the tag and both categories, so the editor, the bulk dialog and
/// a later stamp apply all read the same reason for the same conflict.
fn category_conflict(tag: &str, existing: TagCategory, wanted: TagCategory) -> AppError {
    AppError::BadRequest(format!(
        "{tag:?} is {} and cannot become {}; use another name, e.g. {tag}_({})",
        described(existing),
        described(wanted),
        wanted.as_str(),
    ))
}

fn described(category: TagCategory) -> String {
    let article = if category == TagCategory::Artist {
        "an"
    } else {
        "a"
    };
    format!("{article} {} tag", category.as_str())
}

/// Every tag outside the `(general, unpinned)` default, by name — the
/// vocabulary's own exceptions list (`tag-vocabulary` design D2): what
/// `library.json`'s `tags` key holds, what the `tag_vocabulary` command
/// answers with, and what [`set_category`] and [`set_pinned`] answer with
/// after their own write, so the caller redraws from one list rather than
/// trusting its own edit landed.
pub fn vocabulary(conn: &Connection) -> Result<Vec<TagEntry>> {
    let mut stmt = conn.prepare(
        "SELECT name, category, pinned FROM tags
         WHERE category != 'general' OR pinned = 1
         ORDER BY name",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(TagEntry {
            name: row.get(0)?,
            category: row.get(1)?,
            pinned: row.get(2)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// Change `name`'s category everywhere it is shown, without touching any
/// image's tag set (spec `tag-vocabulary`, "A category is changed where the
/// tag is shown"), and answer with the vocabulary as it now stands. Refused
/// when `name` is not a tag at all: there is no row to recategorise, and a
/// menu naming no image has no tag set to have created one from.
pub fn set_category(library: &Library, name: &str, category: TagCategory) -> Result<Vec<TagEntry>> {
    let changed = library.conn.execute(
        "UPDATE tags SET category = ?1 WHERE name = ?2",
        params![category, name],
    )?;
    if changed == 0 {
        return Err(AppError::NotFound(format!("tag {name}")));
    }
    crate::sidecar::write_library(&library.paths, &library.conn)?;
    vocabulary(&library.conn)
}

/// Pin or unpin `name`, and answer with the vocabulary as it now stands. Same
/// refusal as [`set_category`] for a name that is not a tag.
pub fn set_pinned(library: &Library, name: &str, pinned: bool) -> Result<Vec<TagEntry>> {
    let changed = library.conn.execute(
        "UPDATE tags SET pinned = ?1 WHERE name = ?2",
        params![pinned, name],
    )?;
    if changed == 0 {
        return Err(AppError::NotFound(format!("tag {name}")));
    }
    crate::sidecar::write_library(&library.paths, &library.conn)?;
    vocabulary(&library.conn)
}

/// Record the edit against the image, and refuse one that names no image.
/// Called by the tag editor's writers and by `rules::apply_rules_to_image`, so
/// a run stamps a row exactly as an edit does.
///
/// A missing rating token leaves the rating alone rather than clearing it: the
/// tag box has no way to spell "take the rating away", which is what
/// `set_rating(None)` is for.
///
/// `updated_at` moves on every edit, or "sort by last change" — one of the four
/// sorts — would be a lie (design D9).
pub(crate) fn stamp(conn: &Connection, id: &str, rating: Option<&str>) -> Result<()> {
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
    use crate::model::{ImageSource, ParsedTagSearch, SearchRequest, SearchView};
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
                file_modified_at: None,
                deleted_at: None,
            },
        )
        .unwrap();
    }

    fn library() -> (tempfile::TempDir, Library) {
        let dir = tempfile::tempdir().unwrap();
        let library = Library::open_or_create(dir.path()).unwrap();
        (dir, library)
    }

    /// A row with no file and no thumbnail — cheap to insert by the thousand,
    /// for the tests that only care about ids crossing the SQLite variable
    /// chunk boundary rather than about a row's other columns.
    fn bare_image(library: &Library, id: &str) {
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

    fn category_of(library: &Library, name: &str) -> TagCategory {
        library
            .conn
            .query_row("SELECT category FROM tags WHERE name = ?1", [name], |row| {
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

    /// `auto-tag-rules` task 2.1: `read_metatags` is the function ingest calls
    /// on a rule's tags, so its three rating cases are pinned directly rather
    /// than only through the editor's write path above.
    #[test]
    fn read_metatags_takes_the_rating_metatag_out_and_leaves_the_rest() {
        let text = read_metatags(&strs(&["cat", "rating:s"]));
        assert_eq!(text.tags, vec!["cat".to_string()]);
        assert_eq!(text.rating.as_deref(), Some("s"));
    }

    #[test]
    fn read_metatags_keeps_a_rating_the_alphabet_does_not_know_as_a_tag() {
        let text = read_metatags(&strs(&["rating:unknown"]));
        assert_eq!(text.tags, vec!["rating:unknown".to_string()]);
        assert_eq!(text.rating, None);
    }

    #[test]
    fn read_metatags_with_no_rating_token_answers_no_rating() {
        let text = read_metatags(&strs(&["cat", "cute"]));
        assert_eq!(text.tags, vec!["cat".to_string(), "cute".to_string()]);
        assert_eq!(text.rating, None);
    }

    /// `tag-vocabulary` task 1.3: the design's own worked example — a rating
    /// token, a long-form and a short-form category prefix, an empty-name
    /// prefix dropped like a blank, and a plain tag, all in one pass.
    #[test]
    fn read_metatags_reads_categories_from_their_prefixes_alongside_the_rating() {
        let text = read_metatags(&strs(&[
            "Artist:Cat",
            "rating:s",
            "art:foo",
            "copyright:",
            "bar",
        ]));

        assert_eq!(
            text.tags,
            vec!["Cat".to_string(), "foo".to_string(), "bar".to_string()]
        );
        assert_eq!(text.rating.as_deref(), Some("s"));
        assert_eq!(
            text.categories,
            vec![
                ("Cat".to_string(), TagCategory::Artist),
                ("foo".to_string(), TagCategory::Artist),
            ]
        );
    }

    /// Review finding 1: a source tag's plain name ahead of a rule's prefixed
    /// one for the same name must still name the category — an ingest that
    /// appends a rule's tags after the source tags would otherwise keep the
    /// name general forever, since a prefix never re-categorises (design D4).
    #[test]
    fn a_category_pair_is_kept_whichever_order_the_plain_and_prefixed_forms_arrive_in() {
        let forward = read_metatags(&strs(&["cat", "artist:cat"]));
        let backward = read_metatags(&strs(&["artist:cat", "cat"]));

        assert_eq!(
            forward.categories,
            vec![("cat".to_string(), TagCategory::Artist)]
        );
        assert_eq!(
            backward.categories,
            vec![("cat".to_string(), TagCategory::Artist)]
        );
        assert_eq!(forward.tags, vec!["cat".to_string()]);
        assert_eq!(backward.tags, vec!["cat".to_string()]);
    }

    // -- `tag-vocabulary` task 1.3: "A prefix creates a tag under a category" --

    #[test]
    fn a_prefix_creates_a_tag_under_a_category() {
        let (_dir, library) = library();
        store(&library, "a", None, &[]);

        let record = edit(&library, "a", &["artist:kantoku", "1girl"]);

        assert_eq!(
            record.tags,
            vec!["1girl".to_string(), "kantoku".to_string()]
        );
        assert_eq!(category_of(&library, "kantoku"), TagCategory::Artist);
        assert_eq!(category_of(&library, "1girl"), TagCategory::General);
    }

    #[test]
    fn an_existing_artist_saved_plainly_is_still_an_artist() {
        let (_dir, library) = library();
        store(&library, "a", None, &[]);
        edit(&library, "a", &["artist:kantoku"]);
        store(&library, "b", None, &[]);

        let record = edit(&library, "b", &["kantoku"]);

        assert_eq!(record.tags, vec!["kantoku".to_string()]);
        assert_eq!(category_of(&library, "kantoku"), TagCategory::Artist);
    }

    #[test]
    fn an_existing_artist_saved_with_the_prefix_is_accepted_exactly_as_the_plain_tag() {
        let (_dir, library) = library();
        store(&library, "a", None, &[]);
        edit(&library, "a", &["artist:kantoku"]);
        store(&library, "b", None, &[]);

        let record = edit(&library, "b", &["artist:kantoku"]);

        assert_eq!(record.tags, vec!["kantoku".to_string()]);
        assert_eq!(category_of(&library, "kantoku"), TagCategory::Artist);
    }

    #[test]
    fn a_conflicting_prefix_refuses_the_save_naming_the_tag_and_both_categories() {
        let (_dir, library) = library();
        store(&library, "a", None, &["cat"]);

        let error = update_tags(&library, "a", &strs(&["artist:cat", "1girl"])).unwrap_err();

        let AppError::BadRequest(reason) = error else {
            panic!("got {error}")
        };
        assert!(reason.contains("cat"), "{reason}");
        assert!(reason.contains("general"), "{reason}");
        assert!(reason.contains("artist"), "{reason}");
        assert_eq!(
            ingest::require_record(&library.conn, "a").unwrap().tags,
            vec!["cat".to_string()],
            "the image's tags are unchanged"
        );
        assert_eq!(category_of(&library, "cat"), TagCategory::General);
    }

    /// Review finding 1: the same conflict with the plain tag typed *ahead*
    /// of the prefix, the order a capture's own source tags plus a rule's
    /// `artist:cat` would arrive in — the pairing must still reach
    /// `link_tags` for the save to be refused, not silently keep `cat`
    /// general.
    #[test]
    fn a_conflicting_prefix_still_refuses_the_save_with_the_plain_form_typed_first() {
        let (_dir, library) = library();
        store(&library, "a", None, &["cat"]);

        let error = update_tags(&library, "a", &strs(&["cat", "artist:cat"])).unwrap_err();

        assert!(matches!(error, AppError::BadRequest(_)), "got {error}");
        assert_eq!(
            ingest::require_record(&library.conn, "a").unwrap().tags,
            vec!["cat".to_string()],
            "the image's tags are unchanged"
        );
        assert_eq!(category_of(&library, "cat"), TagCategory::General);
    }

    #[test]
    fn a_conflict_in_a_bulk_edit_refuses_the_whole_edit_and_changes_no_image() {
        let (_dir, library) = library();
        store(&library, "a", None, &["cat"]);
        let ids: Vec<String> = (0..50).map(|index| format!("img-{index}")).collect();
        for id in &ids {
            store(&library, id, None, &[]);
        }

        let error = bulk_update_tags(&library, &ids, &strs(&["artist:cat"]), &[]).unwrap_err();

        assert!(matches!(error, AppError::BadRequest(_)), "got {error}");
        for id in &ids {
            assert!(
                ingest::require_record(&library.conn, id)
                    .unwrap()
                    .tags
                    .is_empty(),
                "nothing of the fifty is changed"
            );
        }
        assert_eq!(category_of(&library, "cat"), TagCategory::General);
    }

    /// Spec `tag-vocabulary`, "A conflict in a rule at capture time": a rule
    /// naming `artist:cat` matches a capture, `cat` already general — the
    /// capture still succeeds, carrying the plain tag, and `cat` stays
    /// general (design D4's `Conflict::Keep`).
    #[test]
    fn a_conflicting_rule_at_capture_time_still_succeeds_and_keeps_the_existing_category() {
        let (_dir, library) = library();
        store(&library, "a", None, &["cat"]);
        crate::rules::upsert(
            &library,
            &crate::model::RuleInput {
                id: None,
                name: "pixiv".to_string(),
                pattern: "pixiv".to_string(),
                is_regex: false,
                tags: vec!["artist:cat".to_string()],
                enabled: true,
            },
        )
        .unwrap();

        let ingested = store_image(
            &library,
            IngestInput {
                id: "b",
                bytes: &png_bytes(),
                source: ImageSource::Extension,
                source_ref: Some("x"),
                image_url: None,
                page_url: Some("https://x.com/alice/status/1"),
                page_title: Some("a pixiv piece"),
                adapter: None,
                rating: None,
                tags: &[],
                captured_at: 1_700_000_000_000,
                file_modified_at: None,
                deleted_at: None,
            },
        )
        .unwrap();

        assert_eq!(ingested.record().tags, vec!["cat".to_string()]);
        assert_eq!(category_of(&library, "cat"), TagCategory::General);
    }

    /// Review finding 5: `read_metatags` already keeps only the first pair
    /// per name, so this shape cannot arrive through it — but `link_tags`
    /// takes a `TagText` built by hand too (`rules::apply_rules_to_image`),
    /// and its own dedup must independently agree: the first prefix for a
    /// name wins, a second one is ignored rather than refused.
    #[test]
    fn link_tags_keeps_the_first_category_named_for_a_tag_reached_by_two_prefixes() {
        let (_dir, library) = library();
        store(&library, "a", None, &[]);
        let text = TagText {
            tags: vec!["cat".to_string()],
            rating: None,
            categories: vec![
                ("cat".to_string(), TagCategory::Artist),
                ("cat".to_string(), TagCategory::Character),
            ],
        };

        let tx = library.conn.unchecked_transaction().unwrap();
        link_tags(&tx, "a", &text, Conflict::Refuse).unwrap();
        tx.commit().unwrap();

        assert_eq!(category_of(&library, "cat"), TagCategory::Artist);
    }

    // -- `tag-vocabulary` task 1.3: `vocabulary` / `set_category` / `set_pinned` --

    #[test]
    fn vocabulary_lists_every_categorised_or_pinned_tag_sorted_by_name() {
        let (_dir, library) = library();
        store(&library, "a", None, &["cat", "dog"]);
        edit(&library, "a", &["artist:zebra", "cat", "dog"]);
        set_pinned(&library, "dog", true).unwrap();

        let entries = vocabulary(&library.conn).unwrap();

        assert_eq!(
            entries,
            vec![
                TagEntry {
                    name: "dog".to_string(),
                    category: TagCategory::General,
                    pinned: true,
                },
                TagEntry {
                    name: "zebra".to_string(),
                    category: TagCategory::Artist,
                    pinned: false,
                },
            ]
        );
    }

    #[test]
    fn set_category_changes_it_everywhere_without_touching_any_images_tags() {
        let (_dir, library) = library();
        store(&library, "a", None, &["azur_lane"]);

        let entries = set_category(&library, "azur_lane", TagCategory::Copyright).unwrap();

        assert_eq!(category_of(&library, "azur_lane"), TagCategory::Copyright);
        assert_eq!(
            ingest::require_record(&library.conn, "a").unwrap().tags,
            vec!["azur_lane".to_string()]
        );
        assert_eq!(
            entries,
            vec![TagEntry {
                name: "azur_lane".to_string(),
                category: TagCategory::Copyright,
                pinned: false,
            }]
        );
    }

    #[test]
    fn set_category_for_an_unknown_tag_is_refused() {
        let (_dir, library) = library();

        let error = set_category(&library, "nobody", TagCategory::Artist).unwrap_err();

        assert!(matches!(error, AppError::NotFound(_)), "got {error}");
    }

    #[test]
    fn set_category_rewrites_library_json() {
        let (_dir, library) = library();
        store(&library, "a", None, &["azur_lane"]);

        set_category(&library, "azur_lane", TagCategory::Copyright).unwrap();

        let file =
            crate::sidecar::read_library(&crate::sidecar::library_path(&library.paths)).unwrap();
        let entries = file.tags.expect("the key is always written");
        assert!(
            entries
                .iter()
                .any(|entry| entry.name == "azur_lane" && entry.category == TagCategory::Copyright)
        );
    }

    /// Review finding 3: `general:x` creates a row that is already the
    /// default — not an exception `library.json` has to carry (design D2) —
    /// so it must not rewrite the file; `artist:x` does, since it creates one.
    #[test]
    fn a_general_prefixed_save_does_not_rewrite_library_json_but_a_categorised_one_does() {
        let (_dir, library) = library();
        store(&library, "a", None, &[]);
        // `artist:seed` first, only to make `library.json` exist at all
        // (`write_library` runs on the first categorised save, not on
        // `open_or_create`) — the baseline this test measures against.
        edit(&library, "a", &["artist:seed"]);
        let file = crate::sidecar::library_path(&library.paths);
        let baseline = std::fs::metadata(&file).unwrap().modified().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));

        edit(&library, "a", &["artist:seed", "general:newtag"]);
        assert_eq!(
            std::fs::metadata(&file).unwrap().modified().unwrap(),
            baseline,
            "a general:x save must not rewrite library.json"
        );

        std::thread::sleep(std::time::Duration::from_millis(10));
        edit(
            &library,
            "a",
            &["artist:seed", "general:newtag", "artist:newartist"],
        );
        assert!(
            std::fs::metadata(&file).unwrap().modified().unwrap() > baseline,
            "a save that creates a categorised row must rewrite library.json"
        );
    }

    #[test]
    fn set_pinned_toggles_and_answers_the_vocabulary() {
        let (_dir, library) = library();
        store(&library, "a", None, &["tagme"]);

        let entries = set_pinned(&library, "tagme", true).unwrap();
        assert!(
            entries
                .iter()
                .any(|entry| entry.name == "tagme" && entry.pinned)
        );

        let entries = set_pinned(&library, "tagme", false).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn set_pinned_for_an_unknown_tag_is_refused() {
        let (_dir, library) = library();

        let error = set_pinned(&library, "nobody", true).unwrap_err();

        assert!(matches!(error, AppError::NotFound(_)), "got {error}");
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

    /// `library-sidecars` task 1.6: a tag edit and a rating change (through
    /// the `rating:` metatag) are both visible in the sidecar afterwards, and
    /// the metatag never lands there as a literal tag.
    #[test]
    fn an_edit_rewrites_the_sidecar_with_the_new_tags_and_rating() {
        let (_dir, library) = library();
        store(&library, "a", None, &["cat"]);

        edit(&library, "a", &["cat", "rating:s"]);

        let sidecar = crate::sidecar::read(&crate::sidecar::path(&library.paths, "a")).unwrap();
        assert_eq!(sidecar.tags, vec!["cat".to_string()]);
        assert_eq!(sidecar.rating.as_deref(), Some("s"));
    }

    #[test]
    fn an_edit_for_an_image_that_is_not_there_changes_nothing() {
        let (_dir, library) = library();
        store(&library, "a", None, &["cat"]);

        let error = update_tags(&library, "nobody", &["dog".to_string()]).unwrap_err();

        assert!(matches!(error, AppError::NotFound(_)), "got {error}");
        assert_eq!(tag_names(&library), vec!["cat".to_string()]);
    }

    fn strs(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    }

    #[test]
    fn bulk_add_reaches_every_selected_image() {
        let (_dir, library) = library();
        let ids: Vec<String> = (0..50).map(|index| format!("img-{index}")).collect();
        for id in &ids {
            store(&library, id, None, &[]);
        }

        bulk_update_tags(&library, &ids, &strs(&["cat", "cute"]), &[]).unwrap();

        for id in &ids {
            assert_eq!(
                ingest::require_record(&library.conn, id).unwrap().tags,
                vec!["cat".to_string(), "cute".to_string()],
            );
        }
    }

    #[test]
    fn bulk_adding_a_tag_already_carried_and_removing_one_not_carried_change_nothing() {
        let (_dir, library) = library();
        store(&library, "a", None, &["cat"]);

        bulk_update_tags(&library, &strs(&["a"]), &strs(&["cat"]), &strs(&["dog"])).unwrap();

        assert_eq!(
            ingest::require_record(&library.conn, "a").unwrap().tags,
            vec!["cat".to_string()]
        );
    }

    #[test]
    fn a_bulk_edit_adds_and_removes_in_one_pass() {
        let (_dir, library) = library();
        store(&library, "a", None, &["cat", "old"]);

        bulk_update_tags(&library, &strs(&["a"]), &strs(&["new"]), &strs(&["old"])).unwrap();

        assert_eq!(
            ingest::require_record(&library.conn, "a").unwrap().tags,
            vec!["cat".to_string(), "new".to_string()],
        );
    }

    #[test]
    fn an_unknown_id_fails_the_whole_bulk_edit_and_edits_no_image() {
        let (_dir, library) = library();
        store(&library, "a", None, &["cat"]);
        store(&library, "b", None, &["cat"]);

        let error = bulk_update_tags(
            &library,
            &strs(&["a", "no-such-id", "b"]),
            &strs(&["new"]),
            &[],
        )
        .unwrap_err();

        assert!(matches!(error, AppError::NotFound(_)), "got {error}");
        assert_eq!(
            ingest::require_record(&library.conn, "a").unwrap().tags,
            vec!["cat".to_string()],
            "processed before the unknown id, but the transaction must roll it back too",
        );
        assert_eq!(
            ingest::require_record(&library.conn, "b").unwrap().tags,
            vec!["cat".to_string()],
        );
    }

    #[test]
    fn a_bulk_add_rewrites_all_three_sidecars() {
        let (_dir, library) = library();
        let ids = strs(&["a", "b", "c"]);
        for id in &ids {
            store(&library, id, None, &[]);
        }

        bulk_update_tags(&library, &ids, &strs(&["cat", "cute"]), &[]).unwrap();

        for id in &ids {
            let sidecar = crate::sidecar::read(&crate::sidecar::path(&library.paths, id)).unwrap();
            assert_eq!(sidecar.tags, vec!["cat".to_string(), "cute".to_string()]);
        }
    }

    #[test]
    fn a_bulk_removal_taking_the_last_use_of_a_tag_collects_it() {
        let (_dir, library) = library();
        store(&library, "a", None, &["cat", "solo"]);
        store(&library, "b", None, &["cat"]);

        bulk_update_tags(&library, &strs(&["a", "b"]), &[], &strs(&["solo"])).unwrap();

        assert_eq!(tag_names(&library), vec!["cat".to_string()]);
    }

    #[test]
    fn bulk_rating_reaches_every_selected_image() {
        let (_dir, library) = library();
        let ids: Vec<String> = (0..12).map(|index| format!("img-{index}")).collect();
        for id in &ids {
            store(&library, id, None, &[]);
        }

        bulk_set_rating(&library, &ids, Some("e")).unwrap();

        for id in &ids {
            assert_eq!(rating_of(&library, id), Some("e".to_string()));
        }
    }

    /// A selection this large used to build one `UPDATE … WHERE id IN (…)`
    /// with one bound parameter per id, which SQLite refuses past its
    /// variable limit ("too many SQL variables"); this pins that the chunked
    /// statement still reaches every id in one call.
    #[test]
    fn bulk_rating_reaches_every_id_past_the_sqlite_variable_chunk_size() {
        let (_dir, library) = library();
        let ids: Vec<String> = (0..2500).map(|index| format!("img-{index}")).collect();
        for id in &ids {
            bare_image(&library, id);
        }

        bulk_set_rating(&library, &ids, Some("e")).unwrap();

        for id in &ids {
            assert_eq!(rating_of(&library, id), Some("e".to_string()));
        }
    }

    #[test]
    fn bulk_rating_of_none_clears_every_selected_image() {
        let (_dir, library) = library();
        store(&library, "a", Some("s"), &[]);
        store(&library, "b", Some("q"), &[]);

        bulk_set_rating(&library, &strs(&["a", "b"]), None).unwrap();

        assert_eq!(rating_of(&library, "a"), None);
        assert_eq!(rating_of(&library, "b"), None);
    }

    #[test]
    fn a_bulk_rating_rewrites_every_touched_sidecar() {
        let (_dir, library) = library();
        let ids = strs(&["a", "b"]);
        for id in &ids {
            store(&library, id, None, &[]);
        }

        bulk_set_rating(&library, &ids, Some("q")).unwrap();

        for id in &ids {
            let sidecar = crate::sidecar::read(&crate::sidecar::path(&library.paths, id)).unwrap();
            assert_eq!(sidecar.rating.as_deref(), Some("q"));
        }
    }

    #[test]
    fn bulk_rating_moves_updated_at_on_every_row() {
        let (_dir, library) = library();
        store(&library, "a", None, &[]);
        store(&library, "b", None, &[]);
        let before_a = ingest::require_record(&library.conn, "a")
            .unwrap()
            .updated_at;
        let before_b = ingest::require_record(&library.conn, "b")
            .unwrap()
            .updated_at;

        bulk_set_rating(&library, &strs(&["a", "b"]), Some("g")).unwrap();

        assert!(
            ingest::require_record(&library.conn, "a")
                .unwrap()
                .updated_at
                > before_a
        );
        assert!(
            ingest::require_record(&library.conn, "b")
                .unwrap()
                .updated_at
                > before_b
        );
    }

    #[test]
    fn a_bulk_rating_outside_the_alphabet_is_refused() {
        let (_dir, library) = library();
        store(&library, "a", None, &[]);

        let error = bulk_set_rating(&library, &strs(&["a"]), Some("safe")).unwrap_err();

        assert!(matches!(error, AppError::BadRequest(_)), "got {error}");
        assert_eq!(rating_of(&library, "a"), None);
    }

    #[test]
    fn selection_tag_counts_orders_by_frequency_then_name() {
        let (_dir, library) = library();
        store(&library, "a", None, &["cat", "cute"]);
        store(&library, "b", None, &["cat"]);
        store(&library, "c", None, &["cat", "dog"]);
        store(&library, "d", None, &["dog"]);

        let counts =
            selection_tag_counts(&library.conn, &strs(&["a", "b", "c", "d"]), 10, None).unwrap();

        assert_eq!(
            counts,
            vec![
                TagCount {
                    name: "cat".to_string(),
                    count: 3
                },
                TagCount {
                    name: "dog".to_string(),
                    count: 2
                },
                TagCount {
                    name: "cute".to_string(),
                    count: 1
                },
            ],
        );
    }

    #[test]
    fn selection_tag_counts_ignores_images_outside_the_selection() {
        let (_dir, library) = library();
        store(&library, "in", None, &["cat"]);
        store(&library, "out", None, &["cat"]);

        let counts = selection_tag_counts(&library.conn, &strs(&["in"]), 10, None).unwrap();

        assert_eq!(
            counts,
            vec![TagCount {
                name: "cat".to_string(),
                count: 1
            }],
            "the image left out of the selection must not inflate its count",
        );
    }

    /// A selection this large crosses more than one chunk of the `IN (…)`
    /// query, so the totals have to be summed across chunks to be right —
    /// `cat` on every image and `dog` on half of them is wrong if either
    /// chunk's count is dropped instead of merged into the running total.
    #[test]
    fn selection_tag_counts_sums_across_chunks_past_the_sqlite_variable_chunk_size() {
        let (_dir, library) = library();
        let ids: Vec<String> = (0..2500).map(|index| format!("img-{index}")).collect();
        for (index, id) in ids.iter().enumerate() {
            bare_image(&library, id);
            let mut tags = vec!["cat".to_string()];
            if index % 2 == 0 {
                tags.push("dog".to_string());
            }
            add_tags(&library.conn, id, &tags).unwrap();
        }

        let counts = selection_tag_counts(&library.conn, &ids, 10, None).unwrap();

        assert_eq!(
            counts,
            vec![
                TagCount {
                    name: "cat".to_string(),
                    count: 2500
                },
                TagCount {
                    name: "dog".to_string(),
                    count: 1250
                },
            ],
        );
    }

    #[test]
    fn a_limit_truncates_the_offered_tags() {
        let (_dir, library) = library();
        store(&library, "a", None, &["cat", "cute", "dog"]);

        let counts = selection_tag_counts(&library.conn, &strs(&["a"]), 2, None).unwrap();

        assert_eq!(counts.len(), 2);
    }

    #[test]
    fn a_selection_with_no_tags_offers_none() {
        let (_dir, library) = library();
        store(&library, "a", None, &[]);

        let counts = selection_tag_counts(&library.conn, &strs(&["a"]), 10, None).unwrap();

        assert!(counts.is_empty());
    }

    /// `tag-vocabulary` design D8: the pinned chips' own filter — counts of
    /// exactly the named tags, ignoring any other tag the selection carries.
    #[test]
    fn a_names_filter_restricts_counts_to_the_named_tags() {
        let (_dir, library) = library();
        store(&library, "a", None, &["cat", "dog", "bird"]);
        store(&library, "b", None, &["cat"]);
        let names = strs(&["cat", "bird"]);

        let counts =
            selection_tag_counts(&library.conn, &strs(&["a", "b"]), 10, Some(&names)).unwrap();

        assert_eq!(
            counts,
            vec![
                TagCount {
                    name: "cat".to_string(),
                    count: 2
                },
                TagCount {
                    name: "bird".to_string(),
                    count: 1
                },
            ]
        );
    }

    #[test]
    fn a_names_filter_of_nothing_offers_no_counts() {
        let (_dir, library) = library();
        store(&library, "a", None, &["cat"]);

        let counts = selection_tag_counts(&library.conn, &strs(&["a"]), 10, Some(&[])).unwrap();

        assert!(counts.is_empty());
    }

    /// Review finding 4: `limit` must not cut a pinned list down — `names`
    /// is already the bound the pinned chips (design D8) asked for.
    #[test]
    fn a_names_filter_ignores_the_limit_so_a_pinned_list_is_never_cut() {
        let (_dir, library) = library();
        store(&library, "a", None, &["cat", "dog", "bird"]);
        let names = strs(&["cat", "dog", "bird"]);

        let counts = selection_tag_counts(&library.conn, &strs(&["a"]), 1, Some(&names)).unwrap();

        assert_eq!(
            counts.len(),
            3,
            "the limit must not truncate a names filter"
        );
    }

    fn found(library: &Library, query: ParsedTagSearch) -> Vec<String> {
        query::search(
            &library.conn,
            &SearchRequest {
                query,
                text: String::new(),
                view: SearchView::Library,
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
    fn setting_a_rating_rewrites_the_sidecar() {
        let (_dir, library) = library();
        store(&library, "a", None, &[]);

        set_rating(&library, "a", Some("e")).unwrap();

        let sidecar = crate::sidecar::read(&crate::sidecar::path(&library.paths, "a")).unwrap();
        assert_eq!(sidecar.rating.as_deref(), Some("e"));
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
    fn tag_usage_fixture() -> (tempfile::TempDir, Library) {
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
        let (_dir, library) = tag_usage_fixture();

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
        let (_dir, library) = tag_usage_fixture();

        assert!(suggested(&library, "zebra", 8).is_empty());
    }

    #[test]
    fn an_empty_prefix_lists_the_most_used_tags() {
        let (_dir, library) = tag_usage_fixture();

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

    /// Spec `tag-vocabulary`, "An artist's last image is trashed and
    /// deleted": the vocabulary outlives its carriers for a categorised tag,
    /// unlike the plain, unpinned one `a_tag_no_image_carries_any_more_is_
    /// never_suggested` pins above — and `suggestions`' `LEFT JOIN` (design
    /// D3) offers it at count 0.
    #[test]
    fn an_artist_survives_its_last_image_being_deleted_forever_and_is_suggested_at_zero() {
        let (_dir, library) = library();
        store(&library, "a", None, &[]);
        edit(&library, "a", &["artist:kantoku"]);

        crate::trash::trash_images(&library, &strs(&["a"])).unwrap();
        crate::trash::delete_forever(&library, &strs(&["a"])).unwrap();

        assert_eq!(category_of(&library, "kantoku"), TagCategory::Artist);
        assert_eq!(
            suggested(&library, "kan", 8),
            vec![("kantoku".to_string(), 0)]
        );
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
