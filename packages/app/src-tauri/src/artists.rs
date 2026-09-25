//! Artist entries (`artist-entries`): a tag that owns a list of profile URLs,
//! matched against a capture's record to correct a handle or display name
//! that is not the artist's tag (design D1, D3, D4), and the rename action
//! that records a correction from an image carrying the wrong name (design
//! D5). `ingest::resolve_tag_text` is the one caller that derives an artist
//! tag at capture time; this module only matches, stores and rewrites.

use rusqlite::{Connection, params};

use crate::error::{AppError, Result};
use crate::ingest;
use crate::library::Library;
use crate::model::{
    ArtistEntry, RenameArtistInput, RenameArtistPreview, RenameArtistPreviewInput,
    RenameArtistReport, SiteAdapterRecord, TagCategory,
};
use crate::tags;

// ---------------------------------------------------------------------------
// URL normalisation and prefix matching (design D3)
// ---------------------------------------------------------------------------

/// The two pieces of `new URL(...)` that `x_account` and `normalized` each
/// read: `hostname` (no userinfo, no port) and `pathname`. `None` wherever the
/// JS constructor would throw — chiefly a string with no scheme.
///
/// This is a splitter, not a URL parser: it does not resolve `..` segments or
/// re-encode anything, so a path the JS constructor would normalise is left as
/// written. X status URLs and artist profile URLs have neither.
pub(crate) fn host_and_path(url: &str) -> Option<(&str, &str)> {
    let after_scheme = url.split_once("://")?.1;
    let authority_end = after_scheme
        .find(['/', '?', '#'])
        .unwrap_or(after_scheme.len());
    let (authority, rest) = after_scheme.split_at(authority_end);
    let host = authority
        .rsplit_once('@')
        .map_or(authority, |(_userinfo, host)| host);
    let host = host.split_once(':').map_or(host, |(name, _port)| name);
    let path = rest.split(['?', '#']).next().unwrap_or("");
    Some((host, path))
}

/// Normalise `url` for ownership comparison (design D3): the text is trimmed
/// first, so a value pasted with surrounding whitespace still matches; a
/// missing scheme is then read as `https://`; a host is required, with a dot
/// in it and no whitespace — a bare name like `metaljelly` is an artist's
/// name, not a URL, and a bare `x.com` would own every X capture — the host
/// is lower-cased and a leading `www.`, `mobile.` or `m.` is dropped;
/// `twitter.com` reads as `x.com`; the query and fragment are already gone by
/// the time [`host_and_path`] hands back a path; every trailing `/` is
/// dropped, and an empty path left after that is refused the same way an
/// empty host is (`x.com/` and `x.com` both name no one); the path is
/// lower-cased too — right for the sites this reads today (X handles are
/// case-insensitive, Pixiv ids are digits) and merely harmless for a
/// case-sensitive path on a site the app does not read yet. The result is
/// `host` with `path` appended and no scheme, so `http` and `https` compare
/// equal — this is the string `artist_urls.url` stores.
pub fn normalized(url: &str) -> Option<String> {
    let trimmed = url.trim();
    let with_scheme = if trimmed.contains("://") {
        trimmed.to_string()
    } else {
        format!("https://{trimmed}")
    };
    let (host, path) = host_and_path(&with_scheme)?;
    if host.is_empty() || !host.contains('.') || host.chars().any(char::is_whitespace) {
        return None;
    }
    let mut host = host.to_lowercase();
    for prefix in ["www.", "mobile.", "m."] {
        if let Some(rest) = host.strip_prefix(prefix) {
            host = rest.to_string();
            break;
        }
    }
    if host == "twitter.com" {
        host = "x.com".to_string();
    }
    let path = path.trim_end_matches('/').to_lowercase();
    if path.is_empty() {
        return None;
    }
    Some(format!("{host}{path}"))
}

/// Whether `entry_url` (already normalised) owns `candidate` (already
/// normalised): equal, or continuing it past a `/` (design D3). The boundary
/// is what keeps `x.com/metaljelly0811` from owning `x.com/metaljelly08110`.
pub fn owns(entry_url: &str, candidate: &str) -> bool {
    candidate == entry_url || candidate.starts_with(&format!("{entry_url}/"))
}

// ---------------------------------------------------------------------------
// Derivation (design D4)
// ---------------------------------------------------------------------------

/// The adapter field an artist tag falls back to, by site (`auto-artist-tag`
/// design D1): the next adapter with an author field the app reads is one
/// more row here.
const ARTIST_FIELDS: [(&str, &str); 2] = [("x", "handle"), ("pixiv", "artist")];

/// The adapter field a record's own profile URL is built from, by site
/// (design D4): the next adapter the app reads a stable profile id from is
/// one more row here, beside its URL template.
const PROFILE_URLS: [(&str, &str, &str); 2] = [
    ("x", "handle", "https://x.com/{}"),
    ("pixiv", "userId", "https://www.pixiv.net/users/{}"),
];

/// The artist name `adapter` names, spelled as [`tags::underscored`] spells
/// every tag (`auto-artist-tag` design D1) — `None` for a site with no author
/// field the app reads (see [`ARTIST_FIELDS`]), a missing or non-text field,
/// or a field that spells to nothing. Pure and total: every odd shape is
/// `None` rather than an error, so nothing a capture could carry makes the
/// caller's answer anything but what it would be without this. [`derive`]'s
/// fallback once no entry owns any candidate URL.
pub fn artist_tag(adapter: &SiteAdapterRecord) -> Option<String> {
    let field = ARTIST_FIELDS
        .iter()
        .find_map(|(site, field)| (*site == adapter.site).then_some(*field))?;
    let text = adapter.fields.get(field)?.as_str()?;
    let spelled = tags::underscored(text);
    (!spelled.is_empty()).then_some(spelled)
}

/// The record's own profile URL (design D4): `x` from `handle`, `pixiv` from
/// `userId` (see [`PROFILE_URLS`]) — `None` for a site with no profile field
/// the app reads, a missing field, or one that is not text.
fn profile_url(adapter: &SiteAdapterRecord) -> Option<String> {
    let (_, field, template) = PROFILE_URLS
        .iter()
        .find(|(site, _, _)| *site == adapter.site)?;
    let value = adapter.fields.get(*field)?.as_str()?;
    Some(template.replace("{}", value))
}

/// The one candidate a capture's record offers for matching (design D4): its
/// own profile URL ([`profile_url`]), normalised — `None` for a site with no
/// profile field the app reads, a missing or non-text field, a URL that does
/// not normalise, or no record at all (`rename_preview`'s own call, over an
/// image with no adapter record). The page URL and the record's `postUrl`
/// are not read here: the page URL is the tab's address, which on X is often
/// the timeline or another user's profile when a repost is captured, and the
/// post URL names the same handle the profile URL already does — an entry
/// made to own either would tag every later capture from that page, or add
/// nothing a profile-URL match does not already give.
pub fn candidate(adapter: Option<&SiteAdapterRecord>) -> Option<String> {
    normalized(&profile_url(adapter?)?)
}

/// The artist tag a capture with `adapter` should carry (design D4): the tag
/// of the entry that owns [`candidate`]'s answer — among several owning
/// entries (nested prefixes), the one with the longest owning URL, the most
/// specific. That entry is skipped in favour of the fallback when its tag
/// exists under a category other than artist (`upsert` and `rename` refuse
/// creating one that way, but `set_category` can change a tag afterwards) —
/// a capture never loses its artist tag to a stale entry. With no owning
/// entry, or no candidate at all, the answer is [`artist_tag`]'s guess from
/// the record alone.
pub fn derive(
    conn: &Connection,
    adapter: &SiteAdapterRecord,
    entries: &[ArtistEntry],
) -> Result<Option<String>> {
    let Some(candidate) = candidate(Some(adapter)) else {
        return Ok(artist_tag(adapter));
    };
    let owning = entries
        .iter()
        .flat_map(|entry| entry.urls.iter().map(move |url| (entry, url.as_str())))
        .filter(|(_, entry_url)| owns(entry_url, &candidate))
        .max_by_key(|(_, entry_url)| entry_url.len());
    let owned_by_an_artist_tag = match owning {
        Some((entry, _)) => match tags::existing_tag_category(conn, &entry.tag)? {
            Some(category) if category != TagCategory::Artist => None,
            _ => Some(entry.tag.clone()),
        },
        None => None,
    };
    Ok(owned_by_an_artist_tag.or_else(|| artist_tag(adapter)))
}

// ---------------------------------------------------------------------------
// The store (design D1, D2)
// ---------------------------------------------------------------------------

/// Every artist entry, grouped by tag in order (design D1): rows are read
/// ordered by tag then URL, so a run of one tag's rows is already contiguous.
pub fn list(conn: &Connection) -> Result<Vec<ArtistEntry>> {
    let mut stmt = conn.prepare("SELECT tag, url FROM artist_urls ORDER BY tag, url")?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    let mut entries: Vec<ArtistEntry> = Vec::new();
    for row in rows {
        let (tag, url) = row?;
        match entries.last_mut() {
            Some(entry) if entry.tag == tag => entry.urls.push(url),
            _ => entries.push(ArtistEntry {
                tag,
                urls: vec![url],
            }),
        }
    }
    Ok(entries)
}

fn existing_owner(conn: &Connection, url: &str) -> Result<Option<String>> {
    match conn.query_row("SELECT tag FROM artist_urls WHERE url = ?1", [url], |row| {
        row.get(0)
    }) {
        Ok(tag) => Ok(Some(tag)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(error) => Err(error.into()),
    }
}

/// Normalises every URL in `urls`, refusing (naming it) the first that does
/// not, then drops any later one that normalises to a URL already seen —
/// design D1's "two lines of one list that normalise to the same URL are one
/// URL" — keeping the first occurrence's position. Shared by `upsert` and
/// `rename`, the two writers that take a URL list from a caller rather than
/// reading one back from the store.
fn normalize_all(urls: &[String]) -> Result<Vec<String>> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::with_capacity(urls.len());
    for url in urls {
        let one = normalized(url)
            .ok_or_else(|| AppError::BadRequest(format!("{url:?} does not look like a URL")))?;
        if seen.insert(one.clone()) {
            out.push(one);
        }
    }
    Ok(out)
}

/// Replace `entry.tag`'s whole URL set (design D1, D5): refused, before any
/// write, for an empty tag, an empty URL list (naming the tag — the webview's
/// "Add artist" with a blank textarea must not silently save an entry that
/// owns nothing, and emptying an existing entry's URLs this way must not
/// delete it out from under the delete confirmation), a URL that does not
/// normalise (naming it), a URL another artist already owns (naming that
/// artist — a URL this same tag already owns is not a conflict, so
/// re-saving its own set always succeeds), or a tag that already exists
/// under a category other than artist (`tags::category_conflict`'s wording —
/// `set_category` is the only other way a tag gets a category, and an entry
/// must not claim one it does not own). Duplicate URLs are deduped by
/// [`normalize_all`]. Answers with the vocabulary of entries as it now
/// stands, the same convention `tags::set_category` follows.
pub fn upsert(library: &Library, entry: &ArtistEntry) -> Result<Vec<ArtistEntry>> {
    let tag = tags::underscored(&entry.tag);
    if tag.is_empty() {
        return Err(AppError::BadRequest("an artist needs a tag".to_string()));
    }
    let urls = normalize_all(&entry.urls)?;
    if urls.is_empty() {
        return Err(AppError::BadRequest(format!(
            "{tag:?} needs at least one URL"
        )));
    }

    let tx = library.conn.unchecked_transaction()?;
    if let Some(existing) = tags::existing_tag_category(&tx, &tag)?
        && existing != TagCategory::Artist
    {
        return Err(tags::category_conflict(&tag, existing, TagCategory::Artist));
    }
    for url in &urls {
        if let Some(owner) = existing_owner(&tx, url)?
            && owner != tag
        {
            return Err(AppError::BadRequest(format!(
                "{url:?} is already owned by {owner:?}"
            )));
        }
    }

    tx.execute("DELETE FROM artist_urls WHERE tag = ?1", params![tag])?;
    for url in &urls {
        tx.execute(
            "INSERT INTO artist_urls (url, tag) VALUES (?1, ?2)",
            params![url, tag],
        )?;
    }
    tx.commit()?;

    crate::sidecar::write_library(&library.paths, &library.conn)?;
    list(&library.conn)
}

/// Delete `tag`'s entry — idempotent, a tag with no entry is the same outcome
/// as one deleted now. No image changes: entries are vocabulary, matched only
/// at capture time (design D7).
pub fn delete(library: &Library, tag: &str) -> Result<Vec<ArtistEntry>> {
    let tag = tags::underscored(tag);
    library
        .conn
        .execute("DELETE FROM artist_urls WHERE tag = ?1", params![tag])?;
    crate::sidecar::write_library(&library.paths, &library.conn)?;
    list(&library.conn)
}

// ---------------------------------------------------------------------------
// Rename (design D5)
// ---------------------------------------------------------------------------

/// The carrier count (trash included) and the one profile URL [`derive`]
/// would read from this image, if any (design D4, D5): the webview never
/// builds a profile URL itself, so what the dialog prefills is exactly what
/// the match will read.
pub fn rename_preview(
    conn: &Connection,
    input: &RenameArtistPreviewInput,
) -> Result<RenameArtistPreview> {
    let from = tags::canonical(&input.from);
    let carriers = carrier_count(conn, &from)?;
    let urls = candidate(input.adapter.as_ref())
        .into_iter()
        .map(|url| format!("https://{url}"))
        .collect();
    Ok(RenameArtistPreview { carriers, urls })
}

fn carrier_count(conn: &Connection, tag_name: &str) -> Result<i64> {
    Ok(conn.query_row(
        "SELECT COUNT(*) FROM image_tags
         JOIN tags ON tags.id = image_tags.tag_id
         WHERE tags.name = ?1",
        [tag_name],
        |row| row.get(0),
    )?)
}

fn carrier_ids(conn: &Connection, tag_name: &str) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT image_tags.image_id FROM image_tags
         JOIN tags ON tags.id = image_tags.tag_id
         WHERE tags.name = ?1",
    )?;
    let ids = stmt.query_map([tag_name], |row| row.get::<_, String>(0))?;
    Ok(ids.collect::<rusqlite::Result<_>>()?)
}

/// Record `to` as the owner of `input.urls`, move any URLs `from` owned along
/// with it, and retag every carrier of `from` — trash included — to `to`, one
/// transaction, refusing before any write (design D5): `from` must name an
/// existing artist tag, refused naming it and its category otherwise (the
/// proposal's non-goal — only an artist renames); `to` canonicalised and
/// refused if empty; `to` already a tag under a category other than artist
/// refused with the reason; a URL that does not normalise refused naming it;
/// a URL a third artist owns refused naming that artist. Duplicate URLs in
/// `input.urls` are deduped by [`normalize_all`].
///
/// With no `to` row yet, the `from` row is renamed in place — keeping its id,
/// its pinned group and every `image_tags` link (`merged: false`). With `to`
/// already an artist tag, the carriers' links move onto it (`INSERT OR
/// IGNORE`, so an image already carrying both is left linked once), a pinned
/// group on `from` moves over only if `to` has none, and the now carrier-less
/// `from` row is deleted (`merged: true`) — the point being that the old name
/// never survives the merge as a row nothing carries.
///
/// `to` canonicalising to the same name as `from` is not a rename — the tag
/// does not change, so there is nothing to retag — and answers
/// `{ retagged: 0, merged: false }` without opening a transaction or touching
/// a carrier's `updated_at`; a URL correction with the name left alone goes
/// through `upsert`, the settings section's own command, not this one.
pub fn rename(library: &Library, input: &RenameArtistInput) -> Result<RenameArtistReport> {
    let from = tags::canonical(&input.from);
    let to = tags::underscored(&input.to);
    if to.is_empty() {
        return Err(AppError::BadRequest("an artist needs a name".to_string()));
    }
    if to == from {
        return Ok(RenameArtistReport {
            retagged: 0,
            merged: false,
        });
    }
    let urls = normalize_all(&input.urls)?;

    let tx = library.conn.unchecked_transaction()?;

    let from_id = tags::existing_tag_id(&tx, &from)?
        .ok_or_else(|| AppError::NotFound(format!("artist tag {from}")))?;
    if let Some(category) = tags::existing_tag_category(&tx, &from)?
        && category != TagCategory::Artist
    {
        return Err(AppError::BadRequest(format!(
            "{from:?} is a {} tag, not an artist tag",
            category.as_str()
        )));
    }
    if let Some(existing) = tags::existing_tag_category(&tx, &to)?
        && existing != TagCategory::Artist
    {
        return Err(tags::category_conflict(&to, existing, TagCategory::Artist));
    }
    for url in &urls {
        if let Some(owner) = existing_owner(&tx, url)?
            && owner != from
            && owner != to
        {
            return Err(AppError::BadRequest(format!(
                "{url:?} is already owned by {owner:?}"
            )));
        }
    }

    // Read before the tag rows change below, so the id list names exactly the
    // images that carried `from` at the moment of the rename.
    let carriers = carrier_ids(&tx, &from)?;

    tx.execute(
        "UPDATE artist_urls SET tag = ?1 WHERE tag = ?2",
        params![to, from],
    )?;
    for url in &urls {
        tx.execute(
            "INSERT INTO artist_urls (url, tag) VALUES (?1, ?2)
             ON CONFLICT (url) DO UPDATE SET tag = excluded.tag",
            params![url, to],
        )?;
    }

    let to_id = tags::existing_tag_id(&tx, &to)?;
    let merged = to_id.is_some_and(|id| id != from_id);
    if merged {
        let to_id = to_id.expect("merged implies to_id is Some");
        tx.execute(
            "INSERT OR IGNORE INTO image_tags (image_id, tag_id)
             SELECT image_id, ?1 FROM image_tags WHERE tag_id = ?2",
            params![to_id, from_id],
        )?;
        tx.execute("DELETE FROM image_tags WHERE tag_id = ?1", params![from_id])?;
        let from_pinned_group: u32 = tx.query_row(
            "SELECT pinned_group FROM tags WHERE id = ?1",
            [from_id],
            |row| row.get(0),
        )?;
        if from_pinned_group != 0 {
            tx.execute(
                "UPDATE tags SET pinned_group = ?1 WHERE id = ?2 AND pinned_group = 0",
                params![from_pinned_group, to_id],
            )?;
        }
        tx.execute("DELETE FROM tags WHERE id = ?1", params![from_id])?;
        tags::compact_groups(&tx)?;
    } else {
        tx.execute(
            "UPDATE tags SET name = ?1, category = 'artist' WHERE id = ?2",
            params![to, from_id],
        )?;
    }

    for id in &carriers {
        tags::mark_updated(&tx, id, None)?;
    }
    tx.commit()?;

    // After the commit, never inside it (`library-sidecars` design D4, D5).
    let records = ingest::load_records(&library.conn, &carriers)?;
    crate::sidecar::write_for_records(&library.paths, &records)?;
    crate::sidecar::write_library(&library.paths, &library.conn)?;

    Ok(RenameArtistReport {
        retagged: carriers.len() as i64,
        merged,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingest::{self, IngestInput, store_image};
    use crate::model::{ImageSource, PinTarget};

    fn adapter(site: &str, fields: serde_json::Value) -> SiteAdapterRecord {
        SiteAdapterRecord {
            site: site.to_string(),
            fields,
        }
    }

    fn entry(tag: &str, urls: &[&str]) -> ArtistEntry {
        ArtistEntry {
            tag: tag.to_string(),
            urls: urls.iter().map(|url| (*url).to_string()).collect(),
        }
    }

    // ---- normalized / owns (task 1.1) ----

    #[test]
    fn a_prefix_owns_the_post() {
        let entry_url = normalized("https://x.com/metaljelly0811").unwrap();

        assert!(owns(
            &entry_url,
            &normalized("https://twitter.com/MetalJelly0811/status/123?s=20").unwrap()
        ));
        assert!(owns(
            &entry_url,
            &normalized("http://www.x.com/metaljelly0811/").unwrap()
        ));
        assert!(!owns(
            &entry_url,
            &normalized("https://x.com/metaljelly08110").unwrap()
        ));
    }

    #[test]
    fn a_schemeless_url_normalises_as_https() {
        assert_eq!(normalized("x.com/alice"), normalized("https://x.com/alice"),);
    }

    #[test]
    fn a_url_with_no_host_is_none() {
        assert_eq!(normalized("https:///no-host"), None);
    }

    #[test]
    fn a_bare_name_is_none() {
        assert_eq!(normalized("metaljelly"), None, "a name is not a URL");
    }

    #[test]
    fn text_with_whitespace_and_no_scheme_is_none() {
        assert_eq!(normalized("hello world"), None);
    }

    #[test]
    fn a_host_with_no_path_is_none() {
        assert_eq!(normalized("https://x.com/"), None, "trailing slash only");
        assert_eq!(normalized("https://x.com"), None, "no path at all");
    }

    #[test]
    fn surrounding_whitespace_is_trimmed_first() {
        assert_eq!(
            normalized(" https://x.com/a \r\n"),
            Some("x.com/a".to_string())
        );
    }

    #[test]
    fn a_mobile_twitter_subdomain_and_a_query_string_both_fold_away() {
        assert_eq!(
            normalized("mobile.twitter.com/A?s=20"),
            Some("x.com/a".to_string())
        );
    }

    // ---- artist_tag (`auto-artist-tag` task 1.3) ----

    #[test]
    fn an_x_record_derives_its_handle_as_the_artist_name() {
        let record = adapter(
            "x",
            serde_json::json!({ "handle": "Alice_Art", "displayName": "Alice ✿" }),
        );

        assert_eq!(artist_tag(&record), Some("alice_art".to_string()));
    }

    #[test]
    fn a_pixiv_record_derives_its_display_name_spelled_as_a_tag() {
        let record = adapter("pixiv", serde_json::json!({ "artist": "Some  Artist" }));

        assert_eq!(artist_tag(&record), Some("some_artist".to_string()));
    }

    #[test]
    fn a_record_from_another_site_derives_no_artist() {
        let record = adapter("danbooru", serde_json::json!({ "artist": "someone" }));

        assert_eq!(artist_tag(&record), None);
    }

    #[test]
    fn a_record_missing_the_field_derives_no_artist() {
        let record = adapter("pixiv", serde_json::json!({ "workId": "123" }));

        assert_eq!(artist_tag(&record), None);
    }

    #[test]
    fn a_non_text_or_blank_field_derives_no_artist() {
        assert_eq!(
            artist_tag(&adapter(
                "pixiv",
                serde_json::json!({ "artist": ["a", "b"] })
            )),
            None
        );
        assert_eq!(
            artist_tag(&adapter("pixiv", serde_json::json!({ "artist": 123 }))),
            None
        );
        assert_eq!(
            artist_tag(&adapter("pixiv", serde_json::json!({ "artist": "   " }))),
            None
        );
    }

    // ---- candidate (task 1.1) ----

    #[test]
    fn a_pixiv_record_offers_its_user_id_as_a_profile_candidate() {
        let record = adapter(
            "pixiv",
            serde_json::json!({ "artist": "someone", "userId": "3439325" }),
        );

        assert_eq!(
            candidate(Some(&record)),
            Some("pixiv.net/users/3439325".to_string())
        );
    }

    #[test]
    fn a_record_without_a_user_id_yields_no_pixiv_profile_candidate() {
        let record = adapter("pixiv", serde_json::json!({ "artist": "someone" }));

        assert_eq!(candidate(Some(&record)), None);
    }

    // ---- the store (task 1.4) ----

    fn library() -> (tempfile::TempDir, Library) {
        let dir = tempfile::tempdir().unwrap();
        let library = Library::open_or_create(dir.path()).unwrap();
        (dir, library)
    }

    // ---- derive (task 1.1; finding 2: category refusals) ----

    #[test]
    fn the_longest_owning_entry_wins() {
        let (_dir, library) = library();
        let entries = vec![entry("a", &["x.com/foo"]), entry("b", &["x.com/foo/bar"])];
        let record = adapter("x", serde_json::json!({ "handle": "foo/bar" }));

        let tag = derive(&library.conn, &record, &entries).unwrap();

        assert_eq!(tag, Some("b".to_string()));
    }

    #[test]
    fn fallback_to_the_field_when_nothing_owns() {
        let (_dir, library) = library();
        let entries = vec![entry("someone_else", &["x.com/someone_else"])];
        let record = adapter("x", serde_json::json!({ "handle": "alice" }));

        assert_eq!(
            derive(&library.conn, &record, &entries).unwrap(),
            Some("alice".to_string())
        );
    }

    #[test]
    fn derive_skips_an_owning_entry_whose_tag_is_not_an_artist_category() {
        let (_dir, library) = library();
        upsert(
            &library,
            &entry("kani_beam", &["https://www.pixiv.net/users/3439325"]),
        )
        .unwrap();
        // `upsert` refuses creating an entry over a non-artist tag, but
        // `set_category` can change one afterwards (design D4) — the scenario
        // this guards.
        store_captured(&library, "seed", &["kani_beam"]);
        tags::set_category(&library, "kani_beam", TagCategory::General).unwrap();
        let entries = list(&library.conn).unwrap();
        let record = adapter(
            "pixiv",
            serde_json::json!({ "artist": "かにビーム", "userId": "3439325" }),
        );

        let tag = derive(&library.conn, &record, &entries).unwrap();

        assert_eq!(
            tag,
            Some("かにビーム".to_string()),
            "the entry is skipped, so the field decides"
        );
    }

    #[test]
    fn list_groups_rows_by_tag_in_order() {
        let (_dir, library) = library();
        upsert(
            &library,
            &entry("metaljelly", &["https://x.com/metaljelly0811"]),
        )
        .unwrap();
        upsert(
            &library,
            &entry("alice", &["https://www.pixiv.net/users/1"]),
        )
        .unwrap();

        let entries = list(&library.conn).unwrap();

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].tag, "alice");
        assert_eq!(entries[0].urls, vec!["pixiv.net/users/1".to_string()]);
        assert_eq!(entries[1].tag, "metaljelly");
    }

    #[test]
    fn upsert_replaces_the_tags_whole_url_set() {
        let (_dir, library) = library();
        upsert(
            &library,
            &entry(
                "metaljelly",
                &["https://x.com/metaljelly0811", "https://x.com/old"],
            ),
        )
        .unwrap();

        let entries = upsert(
            &library,
            &entry("metaljelly", &["https://www.pixiv.net/users/99"]),
        )
        .unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].urls, vec!["pixiv.net/users/99".to_string()]);
    }

    #[test]
    fn the_one_owner_refusal_names_the_owner() {
        let (_dir, library) = library();
        upsert(&library, &entry("alice", &["https://x.com/alice_art"])).unwrap();

        let error = upsert(&library, &entry("bob", &["https://x.com/alice_art"])).unwrap_err();

        let AppError::BadRequest(reason) = error else {
            panic!("got {error}")
        };
        assert!(reason.contains("alice"), "{reason}");
        assert_eq!(list(&library.conn).unwrap().len(), 1, "bob got no entry");
    }

    #[test]
    fn upsert_refuses_an_empty_tag_and_a_url_that_does_not_normalise() {
        let (_dir, library) = library();

        assert!(matches!(
            upsert(&library, &entry("  ", &["https://x.com/a"])).unwrap_err(),
            AppError::BadRequest(_)
        ));
        assert!(matches!(
            upsert(&library, &entry("alice", &["https:///no-host"])).unwrap_err(),
            AppError::BadRequest(_)
        ));
        assert!(list(&library.conn).unwrap().is_empty());
    }

    #[test]
    fn upsert_refuses_a_tag_that_exists_under_another_category() {
        let (_dir, library) = library();
        store_captured(&library, "seed", &["char:miku"]);

        let error = upsert(&library, &entry("miku", &["https://x.com/miku_draws"])).unwrap_err();

        let AppError::BadRequest(reason) = error else {
            panic!("got {error}")
        };
        assert!(reason.contains("miku"), "{reason}");
        assert!(list(&library.conn).unwrap().is_empty());
    }

    #[test]
    fn upsert_refuses_an_empty_url_list() {
        let (_dir, library) = library();

        let error = upsert(&library, &entry("metaljelly", &[])).unwrap_err();

        let AppError::BadRequest(reason) = error else {
            panic!("got {error}")
        };
        assert!(reason.contains("metaljelly"), "{reason}");
        assert!(list(&library.conn).unwrap().is_empty());
    }

    #[test]
    fn upsert_refuses_emptying_an_existing_entrys_urls() {
        let (_dir, library) = library();
        upsert(
            &library,
            &entry("metaljelly", &["https://x.com/metaljelly0811"]),
        )
        .unwrap();

        let error = upsert(&library, &entry("metaljelly", &[])).unwrap_err();

        assert!(matches!(error, AppError::BadRequest(_)), "got {error}");
        assert_eq!(
            list(&library.conn).unwrap(),
            vec![entry("metaljelly", &["x.com/metaljelly0811"])],
            "an empty save must not silently delete the entry"
        );
    }

    #[test]
    fn upsert_dedupes_a_url_repeated_after_normalising() {
        let (_dir, library) = library();

        let entries = upsert(
            &library,
            &entry(
                "metaljelly",
                &[
                    "https://x.com/metaljelly0811",
                    "https://twitter.com/MetalJelly0811/",
                ],
            ),
        )
        .unwrap();

        assert_eq!(
            entries[0].urls,
            vec!["x.com/metaljelly0811".to_string()],
            "the same URL under two spellings is stored once"
        );
    }

    fn png_bytes() -> Vec<u8> {
        let image = image::DynamicImage::ImageRgb8(image::RgbImage::new(2, 2));
        let mut out = std::io::Cursor::new(Vec::new());
        image.write_to(&mut out, image::ImageFormat::Png).unwrap();
        out.into_inner()
    }

    fn store_captured(library: &Library, id: &str, tags: &[&str]) -> ingest::Ingested {
        let tags: Vec<String> = tags.iter().map(|tag| (*tag).to_string()).collect();
        store_image(
            library,
            IngestInput {
                id,
                bytes: &png_bytes(),
                source: ImageSource::Extension,
                source_ref: None,
                image_url: None,
                page_url: None,
                page_title: None,
                adapter: None,
                rating: None,
                tags: &tags,
                captured_at: 1_700_000_000_000,
                file_modified_at: None,
                deleted_at: None,
            },
        )
        .unwrap()
    }

    // ---- rename (task 1.6) ----

    #[test]
    fn a_fresh_rename_keeps_the_pinned_group_and_every_link_and_reports_merged_false() {
        let (_dir, library) = library();
        store_captured(&library, "a", &["artist:metaljelly0811"]);
        store_captured(&library, "b", &["artist:metaljelly0811"]);
        tags::place_pinned(&library, "metaljelly0811", PinTarget::Group(1)).unwrap();

        let report = rename(
            &library,
            &RenameArtistInput {
                from: "metaljelly0811".to_string(),
                to: "metaljelly".to_string(),
                urls: vec!["https://x.com/metaljelly0811".to_string()],
            },
        )
        .unwrap();

        assert_eq!(
            report,
            RenameArtistReport {
                retagged: 2,
                merged: false
            }
        );
        for id in ["a", "b"] {
            assert_eq!(
                ingest::require_record(&library.conn, id).unwrap().tags,
                vec!["metaljelly".to_string()]
            );
        }
        let group: u32 = library
            .conn
            .query_row(
                "SELECT pinned_group FROM tags WHERE name = 'metaljelly'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(group, 1);
        assert_eq!(
            list(&library.conn).unwrap()[0].urls,
            vec!["x.com/metaljelly0811".to_string()]
        );
    }

    #[test]
    fn merging_into_an_existing_artist_moves_links_and_the_pin_and_removes_the_old_row() {
        let (_dir, library) = library();
        for id in 0..10 {
            store_captured(&library, &format!("k{id}"), &["artist:kantoku"]);
        }
        for id in 0..3 {
            store_captured(&library, &format!("p{id}"), &["artist:kantoku_(pixiv)"]);
        }
        // A first, unrelated group so the merge has to compact around it.
        store_captured(&library, "seed", &["artist:someone_else"]);
        tags::place_pinned(&library, "someone_else", PinTarget::Group(1)).unwrap();
        // `from` (`kantoku_(pixiv)`) holds the pin, not `to` (`kantoku`): only
        // this way does the merge exercise "a pinned group on `from` moves
        // over only if `to` has none" — pinning `to` instead, as an earlier
        // version of this test did, left that branch untested since `to`'s
        // own pin would just sit there regardless of what `from` had.
        tags::place_pinned(&library, "kantoku_(pixiv)", PinTarget::Group(2)).unwrap();

        let report = rename(
            &library,
            &RenameArtistInput {
                from: "kantoku_(pixiv)".to_string(),
                to: "kantoku".to_string(),
                urls: vec![],
            },
        )
        .unwrap();

        assert_eq!(
            report,
            RenameArtistReport {
                retagged: 3,
                merged: true
            }
        );
        let carriers: i64 = library
            .conn
            .query_row(
                "SELECT COUNT(*) FROM image_tags JOIN tags ON tags.id = image_tags.tag_id
                 WHERE tags.name = 'kantoku'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(carriers, 13);
        let group: u32 = library
            .conn
            .query_row(
                "SELECT pinned_group FROM tags WHERE name = 'kantoku'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(group, 2, "kantoku inherits the pin kantoku_(pixiv) had");
        assert_eq!(
            tags::existing_tag_id(&library.conn, "kantoku_(pixiv)").unwrap(),
            None,
            "the old row is gone"
        );
        let groups: Vec<u32> = library
            .conn
            .prepare(
                "SELECT DISTINCT pinned_group FROM tags
                 WHERE pinned_group > 0 ORDER BY pinned_group",
            )
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        assert_eq!(
            groups,
            vec![1, 2],
            "compact_groups leaves no gap where kantoku_(pixiv)'s row was"
        );
    }

    #[test]
    fn a_name_taken_by_a_character_is_refused_and_nothing_changes() {
        let (_dir, library) = library();
        store_captured(&library, "a", &["artist:miku_draws"]);
        store_captured(&library, "seed", &[]);
        tags::update_tags(&library, "seed", &["char:miku".to_string()]).unwrap();

        let error = rename(
            &library,
            &RenameArtistInput {
                from: "miku_draws".to_string(),
                to: "miku".to_string(),
                urls: vec![],
            },
        )
        .unwrap_err();

        assert!(matches!(error, AppError::BadRequest(_)), "got {error}");
        assert_eq!(
            ingest::require_record(&library.conn, "a").unwrap().tags,
            vec!["miku_draws".to_string()],
        );
    }

    #[test]
    fn a_trashed_carrier_is_retagged() {
        let (_dir, library) = library();
        store_captured(&library, "a", &["artist:alice_art"]);
        crate::trash::trash_images(&library, &["a".to_string()]).unwrap();

        rename(
            &library,
            &RenameArtistInput {
                from: "alice_art".to_string(),
                to: "alice".to_string(),
                urls: vec![],
            },
        )
        .unwrap();

        crate::trash::restore_images(&library, &["a".to_string()]).unwrap();
        assert_eq!(
            ingest::require_record(&library.conn, "a").unwrap().tags,
            vec!["alice".to_string()],
        );
    }

    #[test]
    fn each_carriers_updated_at_moves_and_the_urls_are_owned_by_to_afterwards() {
        let (_dir, library) = library();
        store_captured(&library, "a", &["artist:metaljelly0811"]);
        let before = 1_600_000_000_000_i64;
        library
            .conn
            .execute(
                "UPDATE images SET updated_at = ?1 WHERE id = 'a'",
                params![before],
            )
            .unwrap();

        rename(
            &library,
            &RenameArtistInput {
                from: "metaljelly0811".to_string(),
                to: "metaljelly".to_string(),
                urls: vec!["https://x.com/metaljelly0811".to_string()],
            },
        )
        .unwrap();

        let after = ingest::require_record(&library.conn, "a")
            .unwrap()
            .updated_at;
        assert!(after > before, "rename bumps updated_at");
        assert_eq!(
            list(&library.conn).unwrap(),
            vec![entry("metaljelly", &["x.com/metaljelly0811"])]
        );
    }

    #[test]
    fn rename_to_the_same_name_is_a_no_op_and_touches_no_carrier() {
        let (_dir, library) = library();
        store_captured(&library, "a", &["artist:metaljelly"]);
        let before = 1_600_000_000_000_i64;
        library
            .conn
            .execute(
                "UPDATE images SET updated_at = ?1 WHERE id = 'a'",
                params![before],
            )
            .unwrap();

        let report = rename(
            &library,
            &RenameArtistInput {
                from: "metaljelly".to_string(),
                to: "metaljelly".to_string(),
                urls: vec!["https://x.com/metaljelly0811".to_string()],
            },
        )
        .unwrap();

        assert_eq!(
            report,
            RenameArtistReport {
                retagged: 0,
                merged: false
            }
        );
        let after = ingest::require_record(&library.conn, "a")
            .unwrap()
            .updated_at;
        assert_eq!(after, before, "updated_at does not move");
        assert!(
            list(&library.conn).unwrap().is_empty(),
            "the given URL is not recorded either — that goes through upsert"
        );
    }

    #[test]
    fn rename_refuses_a_from_that_is_not_an_artist_tag() {
        let (_dir, library) = library();
        store_captured(&library, "a", &["metaljelly0811"]);

        let error = rename(
            &library,
            &RenameArtistInput {
                from: "metaljelly0811".to_string(),
                to: "metaljelly".to_string(),
                urls: vec![],
            },
        )
        .unwrap_err();

        let AppError::BadRequest(reason) = error else {
            panic!("got {error}")
        };
        assert!(
            reason.contains("metaljelly0811") && reason.contains("general"),
            "{reason}"
        );
        assert_eq!(
            ingest::require_record(&library.conn, "a").unwrap().tags,
            vec!["metaljelly0811".to_string()],
            "nothing changes"
        );
    }

    #[test]
    fn rename_moves_urls_from_already_owned_even_when_none_are_given() {
        let (_dir, library) = library();
        store_captured(&library, "a", &["artist:metaljelly0811"]);
        upsert(
            &library,
            &entry("metaljelly0811", &["https://x.com/metaljelly0811"]),
        )
        .unwrap();

        rename(
            &library,
            &RenameArtistInput {
                from: "metaljelly0811".to_string(),
                to: "metaljelly".to_string(),
                urls: vec![],
            },
        )
        .unwrap();

        assert_eq!(
            list(&library.conn).unwrap(),
            vec![entry("metaljelly", &["x.com/metaljelly0811"])],
            "the URL from already owned moves to to even though input.urls is empty"
        );
    }

    #[test]
    fn a_rename_rewrites_library_json_and_the_carriers_sidecar() {
        let (_dir, library) = library();
        store_captured(&library, "a", &["artist:metaljelly0811"]);

        rename(
            &library,
            &RenameArtistInput {
                from: "metaljelly0811".to_string(),
                to: "metaljelly".to_string(),
                urls: vec!["https://x.com/metaljelly0811".to_string()],
            },
        )
        .unwrap();

        let file =
            crate::sidecar::read_library(&crate::sidecar::library_path(&library.paths)).unwrap();
        assert_eq!(
            file.artists,
            Some(vec![entry("metaljelly", &["x.com/metaljelly0811"])]),
            "library.json lists the entry under its new name"
        );

        let sidecar = crate::sidecar::read(&crate::sidecar::path(&library.paths, "a")).unwrap();
        assert_eq!(
            sidecar.tags,
            vec!["metaljelly".to_string()],
            "the carrier's sidecar carries the new tag"
        );
    }

    // ---- rename_preview (task 1.6) ----

    #[test]
    fn preview_counts_trash_and_lists_the_profile_url_with_a_scheme() {
        let (_dir, library) = library();
        store_captured(&library, "a", &["metaljelly0811"]);
        store_captured(&library, "b", &["metaljelly0811"]);
        crate::trash::trash_images(&library, &["b".to_string()]).unwrap();

        let preview = rename_preview(
            &library.conn,
            &RenameArtistPreviewInput {
                from: "metaljelly0811".to_string(),
                adapter: Some(adapter(
                    "x",
                    serde_json::json!({ "handle": "metaljelly0811" }),
                )),
            },
        )
        .unwrap();

        assert_eq!(preview.carriers, 2, "trash included");
        assert_eq!(
            preview.urls,
            vec!["https://x.com/metaljelly0811".to_string()]
        );
    }

    #[test]
    fn preview_has_no_url_for_a_record_with_no_profile_url() {
        let (_dir, library) = library();
        store_captured(&library, "a", &["someone"]);

        let preview = rename_preview(
            &library.conn,
            &RenameArtistPreviewInput {
                from: "someone".to_string(),
                adapter: Some(adapter(
                    "danbooru",
                    serde_json::json!({ "artist": "someone" }),
                )),
            },
        )
        .unwrap();

        assert!(preview.urls.is_empty());
    }
}
