//! Artist entries (`artist-entries`): a tag that owns a list of profile URLs,
//! matched against a capture's record to correct a handle or display name
//! that is not the artist's tag (design D1, D3, D4), and the rename action
//! that records a correction from an image carrying the wrong name (design
//! D5). `ingest::resolve_tag_text` is the one caller that derives an artist
//! tag at capture time; this module only matches, stores and rewrites. A bare
//! "design Dn" below is `artist-entries`'; the preview, match and apply
//! additions cite `artist-workflow` by name, since the two changes both
//! number their decisions from D1.

use rusqlite::{Connection, params};

use crate::error::{AppError, Result};
use crate::ingest;
use crate::library::Library;
use crate::model::{
    ArtistApplyPreview, ArtistApplyReport, ArtistEntry, ArtistMatch, ArtistPreview,
    ArtistPreviewInput, RenameArtistInput, RenameArtistReport, SiteAdapterRecord, TagCategory,
};
use crate::query::{ID_CHUNK, placeholders};
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

/// Why [`normalized`] refuses `url` (design D2, `artist-url-any-site`): two
/// shapes, kept apart so the caller can say why instead of showing one
/// generic "not a URL" for both.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UrlRefusal {
    /// No host, a host without a dot, or whitespace in the host — not a URL
    /// at all, never mind one this app reads.
    NotAUrl,
    /// A host alone, and the host is one whose account identity lives in the
    /// path (see [`path_identity_hosts`]): the bare host would own every
    /// account on that site, so the paste is refused naming it rather than
    /// stored as a host that owns everyone.
    BareProfileHost(String),
}

impl UrlRefusal {
    /// The message shown for the pasted `url` — `normalize_all`'s
    /// `BadRequest` text, the dialog's refusal line, verbatim.
    pub fn message(&self, url: &str) -> String {
        match self {
            UrlRefusal::NotAUrl => format!("{url:?} does not look like a URL"),
            UrlRefusal::BareProfileHost(host) => format!(
                "{url:?} names {host} alone, which would own every {host} account; add the account's path"
            ),
        }
    }
}

/// Canonicalise a host the same way [`normalized`] canonicalises a pasted
/// one: lower-cased, a leading `www.`, `mobile.` or `m.` dropped,
/// `twitter.com` read as `x.com`. [`path_identity_hosts`] runs each
/// `PROFILE_URLS` template's host through this same function, so the table
/// and a pasted host cannot disagree about which host owns a path identity.
pub(crate) fn canonical_host(host: &str) -> String {
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
    host
}

/// Normalise `url` for ownership comparison (design D3; host-only handling
/// amended by `artist-url-any-site` D1–D3): the text is trimmed first, so a
/// value pasted with surrounding whitespace still matches; a missing scheme
/// is then read as `https://`; a host is required, with a dot in it and no
/// whitespace — a bare name like `metaljelly` is an artist's name, not a
/// URL — the host is canonicalised ([`canonical_host`]); the query and
/// fragment are already gone by the time [`host_and_path`] hands back a
/// path; every trailing `/` is dropped; the path is lower-cased too — right
/// for the sites this reads today (X handles are case-insensitive, Pixiv ids
/// are digits) and merely harmless for a case-sensitive path on a site the
/// app does not read yet. An empty path left after that is refused, naming
/// the host, only when the host is one [`path_identity_hosts`] lists: on
/// those sites the app already knows an account lives at a path under the
/// host, so a bare host would own every account there — the same argument
/// as before, now scoped to exactly the hosts it is about. Every other
/// host-only URL — its account identity is the subdomain itself (Fanbox), or
/// the app does not read the site at all — is stored as the host alone and
/// matches nothing until an adapter for it produces a candidate, which
/// always carries a path. The result is `host` with `path` appended and no
/// scheme, so `http` and `https` compare equal — this is the string
/// `artist_urls.url` stores.
pub fn normalized(url: &str) -> std::result::Result<String, UrlRefusal> {
    let trimmed = url.trim();
    let with_scheme = if trimmed.contains("://") {
        trimmed.to_string()
    } else {
        format!("https://{trimmed}")
    };
    let (host, path) = host_and_path(&with_scheme).ok_or(UrlRefusal::NotAUrl)?;
    if host.is_empty() || !host.contains('.') || host.chars().any(char::is_whitespace) {
        return Err(UrlRefusal::NotAUrl);
    }
    let host = canonical_host(host);
    let path = path.trim_end_matches('/').to_lowercase();
    if path.is_empty() {
        return if path_identity_hosts().contains(&host) {
            Err(UrlRefusal::BareProfileHost(host))
        } else {
            Ok(host)
        };
    }
    Ok(format!("{host}{path}"))
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

/// Hosts whose account identity lives in the path, not the subdomain (design
/// D1, `artist-url-any-site`): every [`PROFILE_URLS`] template's host,
/// canonicalised ([`canonical_host`]) the same way a pasted host is —
/// `x.com`, `pixiv.net` today. [`normalized`] refuses a host-only URL only
/// for a host in this set, so the old rule's argument ("a bare `x.com` would
/// own every X capture") stays true for exactly the hosts it was about; the
/// next adapter with a path identity adds itself by adding its
/// `PROFILE_URLS` row, nothing here.
pub(crate) fn path_identity_hosts() -> std::collections::HashSet<String> {
    PROFILE_URLS
        .iter()
        .filter_map(|(_, _, template)| {
            host_and_path(template).map(|(host, _)| canonical_host(host))
        })
        .collect()
}

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
fn record_profile_url(adapter: &SiteAdapterRecord) -> Option<String> {
    let (_, field, template) = PROFILE_URLS
        .iter()
        .find(|(site, _, _)| *site == adapter.site)?;
    let value = adapter.fields.get(*field)?.as_str()?;
    Some(template.replace("{}", value))
}

/// The X account an image's page URL names, read by the rule the Account row
/// and the `account:` filter already trust (`query::x_account`) — `None` for
/// a page that is not an account's (X's own pages, any other site) or no
/// page at all. Consulted only for an image stored with **no** adapter record
/// (see [`profile_url`]).
fn page_handle(page_url: Option<&str>) -> Option<&str> {
    crate::query::x_account(page_url?)
}

/// The image's own profile URL: the record's ([`record_profile_url`]) when
/// the image has a record, else — a legacy-bundle import, a capture whose page
/// never answered the context ask — the X account its page URL names
/// ([`page_handle`]). A record's fields are the author's while the page is
/// the tab's address (`artist-entries` design D4), so the page is never read
/// beside a record; with no record it is the one fact left, and a clicked
/// image's address on X names its author — the addresses that do not (the
/// timeline, search) are the reserved segments `x_account` refuses (D4,
/// amended 2026-09-28).
fn profile_url(adapter: Option<&SiteAdapterRecord>, page_url: Option<&str>) -> Option<String> {
    match adapter {
        Some(adapter) => record_profile_url(adapter),
        None => page_handle(page_url).map(|handle| format!("https://x.com/{handle}")),
    }
}

/// The artist tag a capture with this profile derives, entries aside:
/// [`artist_tag`]'s reading of the record, or the page handle spelled as a
/// tag for an image with no record — the same choice [`profile_url`] makes.
pub fn derived_tag(adapter: Option<&SiteAdapterRecord>, page_url: Option<&str>) -> Option<String> {
    match adapter {
        Some(adapter) => artist_tag(adapter),
        None => page_handle(page_url)
            .map(tags::underscored)
            .filter(|spelled| !spelled.is_empty()),
    }
}

/// The one candidate an image offers for matching (design D4): its own
/// profile URL ([`profile_url`]), normalised — `None` for a record with no
/// profile field the app reads, a missing or non-text field, a URL that does
/// not normalise, or neither a record nor an account page (`preview`'s own
/// call over a tag with no entry, or a menu opened with no image in scope).
/// A record's `postUrl` is not read here: it names the same handle the
/// profile URL already does, and an entry made to own it would add nothing
/// a profile-URL match does not already give.
pub fn candidate(adapter: Option<&SiteAdapterRecord>, page_url: Option<&str>) -> Option<String> {
    normalized(&profile_url(adapter, page_url)?).ok()
}

/// The entry that owns `candidate` (already normalised) — among several
/// owning entries (nested prefixes), the one with the longest owning URL, the
/// most specific (`artist-workflow` design D3, extracted from [`derive`] so
/// [`preview`], [`artist_match`] and `derive` share the one answer to "who
/// owns this URL"). `None` when no entry owns it.
pub fn owning_entry<'a>(candidate: &str, entries: &'a [ArtistEntry]) -> Option<&'a ArtistEntry> {
    entries
        .iter()
        .flat_map(|entry| entry.urls.iter().map(move |url| (entry, url.as_str())))
        .filter(|(_, entry_url)| owns(entry_url, candidate))
        .max_by_key(|(_, entry_url)| entry_url.len())
        .map(|(entry, _)| entry)
}

/// The artist tag a capture with `adapter` should carry (design D4): the tag
/// of the entry [`owning_entry`] finds for [`candidate`]'s answer. That entry
/// is skipped in favour of the fallback when its tag exists under a category
/// other than artist (`upsert` and `rename` refuse creating one that way, but
/// `set_category` can change a tag afterwards) — a capture never loses its
/// artist tag to a stale entry. With no owning entry, or no candidate at all,
/// the answer is [`artist_tag`]'s guess from the record alone.
pub fn derive(
    conn: &Connection,
    adapter: &SiteAdapterRecord,
    entries: &[ArtistEntry],
) -> Result<Option<String>> {
    let Some(candidate) = candidate(Some(adapter), None) else {
        return Ok(artist_tag(adapter));
    };
    let owned_by_an_artist_tag = match owning_entry(&candidate, entries) {
        Some(entry) => match tags::existing_tag_category(conn, &entry.tag)? {
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
        let one = normalized(url).map_err(|refusal| AppError::BadRequest(refusal.message(url)))?;
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
// Preview and match (`artist-workflow` design D3, D5)
// ---------------------------------------------------------------------------

/// The entry's own URLs, the carrier count (trash included), and the image's
/// own profile URL as a further candidate line — replacing `rename_preview`
/// (`artist-workflow` design D3): `carriers` and `urls` name the tag alone,
/// so the dialog can open on a sidebar row or a pinned chip with no image in
/// scope; `candidate` is `None` for a tag with no adapter, no profile URL, or
/// a URL **any** entry already owns — this one (already in `urls`) or
/// another, where appending it could only end in the "already owned by"
/// refusal, and the Artist row (`artist-workflow` design D5) already says
/// which artist that is.
pub fn preview(conn: &Connection, input: &ArtistPreviewInput) -> Result<ArtistPreview> {
    let tag = tags::underscored(&input.tag);
    let carriers = carrier_count(conn, &tag)?;
    let entries = list(conn)?;
    let urls = entries
        .iter()
        .find(|entry| entry.tag == tag)
        .map(|entry| {
            entry
                .urls
                .iter()
                .map(|url| format!("https://{url}"))
                .collect()
        })
        .unwrap_or_default();
    let candidate = candidate(input.adapter.as_ref(), input.page_url.as_deref())
        .filter(|candidate| owning_entry(candidate, &entries).is_none());
    Ok(ArtistPreview {
        carriers,
        urls,
        candidate: candidate.map(|candidate| format!("https://{candidate}")),
    })
}

/// The image's own profile URL, the entry that owns it (if any), and the
/// artist tag a capture from this image derives regardless of any entry
/// (`artist-workflow` design D5) — `None` when the image yields no profile
/// URL at all ([`candidate`]: its record, or its page URL for an image stored
/// with none). Ownership does not consult the owning entry's category the way
/// [`derive`] does: the row this backs names who owns the URL as the
/// vocabulary has it today, and a stale category is fixed where it is listed
/// (Settings → Artists), not hidden from the chip.
pub fn artist_match(
    conn: &Connection,
    adapter: Option<&SiteAdapterRecord>,
    page_url: Option<&str>,
) -> Result<Option<ArtistMatch>> {
    let Some(candidate) = candidate(adapter, page_url) else {
        return Ok(None);
    };
    let entries = list(conn)?;
    let owner = owning_entry(&candidate, &entries).map(|entry| entry.tag.clone());
    Ok(Some(ArtistMatch {
        url: format!("https://{candidate}"),
        owner,
        derived: derived_tag(adapter, page_url),
    }))
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
/// its pinned group, its note and every `image_tags` link (`merged: false`).
/// With `to` already an artist tag, the carriers' links move onto it
/// (`INSERT OR IGNORE`, so an image already carrying both is left linked
/// once), a pinned group and a note on `from` each move over only if `to`
/// has none of its own (`tag-notes` design D5), and the now carrier-less
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
        // The note on the handle is what the merge is confirming
        // (`tag-notes` design D5), the same rule as the pinned group above:
        // carried over only when the target has none of its own.
        let from_note: Option<String> =
            tx.query_row("SELECT note FROM tags WHERE id = ?1", [from_id], |row| {
                row.get(0)
            })?;
        if from_note.is_some() {
            tx.execute(
                "UPDATE tags SET note = ?1 WHERE id = ?2 AND note IS NULL",
                params![from_note, to_id],
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

// ---------------------------------------------------------------------------
// Applying an entry to stored images (`artist-workflow` design D6)
// ---------------------------------------------------------------------------

/// Every non-deleted image whose own profile URL ([`candidate`]: its record,
/// or its page URL for an image stored with none) some URL in `owned_urls`
/// (already normalised) owns — the one scan [`apply_preview`] and [`apply`]
/// both read, so "images from these URLs" has one answer. A record that does
/// not parse as JSON is read as no record; an image yielding no candidate is
/// skipped rather than failing the scan.
fn from_profiles(conn: &Connection, owned_urls: &[String]) -> Result<Vec<String>> {
    let mut stmt =
        conn.prepare("SELECT id, adapter_json, page_url FROM images WHERE deleted_at IS NULL")?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, Option<String>>(1)?,
            row.get::<_, Option<String>>(2)?,
        ))
    })?;
    let mut ids = Vec::new();
    for row in rows {
        let (id, adapter_json, page_url) = row?;
        let adapter = adapter_json
            .as_deref()
            .and_then(|json| serde_json::from_str::<SiteAdapterRecord>(json).ok());
        let Some(candidate) = candidate(adapter.as_ref(), page_url.as_deref()) else {
            continue;
        };
        if owned_urls.iter().any(|url| owns(url, &candidate)) {
            ids.push(id);
        }
    }
    Ok(ids)
}

/// How many stored images `urls` would reach and how many of them carry no
/// artist tag at all (`artist-workflow` design D6): `urls` as the caller
/// gives them — typed into a form, mid-edit — so a line that does not
/// normalise is ignored rather than refusing the whole preview.
pub fn apply_preview(conn: &Connection, urls: &[String]) -> Result<ArtistApplyPreview> {
    let owned: Vec<String> = urls.iter().filter_map(|url| normalized(url).ok()).collect();
    let ids = from_profiles(conn, &owned)?;
    let untagged = count_carrying_no_artist_tag(conn, &ids)?;
    Ok(ArtistApplyPreview {
        images: ids.len() as i64,
        untagged,
    })
}

fn count_carrying_no_artist_tag(conn: &Connection, ids: &[String]) -> Result<i64> {
    let mut count = 0i64;
    for chunk in ids.chunks(ID_CHUNK) {
        let in_list = placeholders(chunk.len());
        let sql = format!(
            "SELECT COUNT(*) FROM images
             WHERE id IN ({in_list})
             AND id NOT IN (
                 SELECT image_tags.image_id FROM image_tags
                 JOIN tags ON tags.id = image_tags.tag_id
                 WHERE tags.category = 'artist'
             )"
        );
        count += conn.query_row(&sql, rusqlite::params_from_iter(chunk), |row| {
            row.get::<_, i64>(0)
        })?;
    }
    Ok(count)
}

/// Add `tag` — an existing artist entry's tag — as an artist tag to every
/// image [`from_profiles`] finds for that entry's URLs, unless it carries the
/// tag already (`artist-workflow` design D6): refused before any write when
/// `tag` has no entry (`AppError::NotFound`) or exists under a category other
/// than artist (`tags::category_conflict`, the same check [`derive`] makes,
/// so apply never links an image under a general or character tag). One
/// transaction — the owner's largest artist is hundreds of images, the scale
/// `rename` already writes in one. Trash is excluded, unlike `rename`: apply
/// only adds, so a trashed image loses nothing by being left out and restores
/// as it was.
pub fn apply(library: &Library, tag: &str) -> Result<ArtistApplyReport> {
    let tag = tags::underscored(tag);

    let tx = library.conn.unchecked_transaction()?;

    let entries = list(&tx)?;
    let urls = entries
        .iter()
        .find(|entry| entry.tag == tag)
        .map(|entry| entry.urls.clone())
        .ok_or_else(|| AppError::NotFound(format!("artist entry {tag}")))?;
    if let Some(category) = tags::existing_tag_category(&tx, &tag)?
        && category != TagCategory::Artist
    {
        return Err(tags::category_conflict(&tag, category, TagCategory::Artist));
    }

    let ids = from_profiles(&tx, &urls)?;
    let text = tags::read_metatags(&[format!("artist:{tag}")]);
    let mut tagged_ids = Vec::new();
    let mut tagged = 0i64;
    let mut skipped = 0i64;
    let mut categorised = false;
    for id in &ids {
        let already: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM image_tags
             JOIN tags ON tags.id = image_tags.tag_id
             WHERE image_tags.image_id = ?1 AND tags.name = ?2)",
            params![id, &tag],
            |row| row.get(0),
        )?;
        if already {
            skipped += 1;
            continue;
        }
        categorised |= tags::link_tags(&tx, id, &text, tags::Conflict::Keep)?;
        tags::mark_updated(&tx, id, None)?;
        tagged += 1;
        tagged_ids.push(id.clone());
    }
    tx.commit()?;

    // After the commit, never inside it (`library-sidecars` design D4, D5).
    if !tagged_ids.is_empty() {
        let records = ingest::load_records(&library.conn, &tagged_ids)?;
        crate::sidecar::write_for_records(&library.paths, &records)?;
    }
    if categorised {
        crate::sidecar::write_library(&library.paths, &library.conn)?;
    }

    Ok(ArtistApplyReport { tagged, skipped })
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
        assert_eq!(normalized("https:///no-host"), Err(UrlRefusal::NotAUrl));
    }

    #[test]
    fn a_bare_name_is_none() {
        assert_eq!(
            normalized("metaljelly"),
            Err(UrlRefusal::NotAUrl),
            "a name is not a URL"
        );
    }

    #[test]
    fn text_with_whitespace_and_no_scheme_is_none() {
        assert_eq!(normalized("hello world"), Err(UrlRefusal::NotAUrl));
    }

    #[test]
    fn a_bare_path_identity_host_is_refused_naming_it() {
        for (url, host) in [
            ("https://x.com/", "x.com"),
            ("x.com", "x.com"),
            ("https://www.pixiv.net", "pixiv.net"),
        ] {
            let refusal = normalized(url).unwrap_err();
            assert_eq!(
                refusal,
                UrlRefusal::BareProfileHost(host.to_string()),
                "{url}"
            );
            let message = refusal.message(url);
            assert!(message.contains(host), "{message}");
            assert!(message.contains("add the account's path"), "{message}");
        }
    }

    #[test]
    fn a_host_only_url_elsewhere_is_stored_as_its_host() {
        assert_eq!(
            normalized("https://kanibiimu.fanbox.cc/"),
            Ok("kanibiimu.fanbox.cc".to_string())
        );
        assert_eq!(
            normalized("KANIBIIMU.fanbox.cc"),
            Ok("kanibiimu.fanbox.cc".to_string())
        );
    }

    #[test]
    fn a_host_only_entry_owns_its_own_pages_and_not_a_longer_host() {
        assert!(owns("kanibiimu.fanbox.cc", "kanibiimu.fanbox.cc/posts/1"));
        assert!(!owns("kanibiimu.fanbox.cc", "kanibiimu.fanbox.cc.evil"));
    }

    #[test]
    fn path_identity_hosts_follow_the_profile_url_table() {
        let hosts: std::collections::HashSet<String> = ["x.com", "pixiv.net"]
            .into_iter()
            .map(str::to_string)
            .collect();

        assert_eq!(path_identity_hosts(), hosts);
    }

    #[test]
    fn surrounding_whitespace_is_trimmed_first() {
        assert_eq!(
            normalized(" https://x.com/a \r\n"),
            Ok("x.com/a".to_string())
        );
    }

    #[test]
    fn a_mobile_twitter_subdomain_and_a_query_string_both_fold_away() {
        assert_eq!(
            normalized("mobile.twitter.com/A?s=20"),
            Ok("x.com/a".to_string())
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
            candidate(Some(&record), None),
            Some("pixiv.net/users/3439325".to_string())
        );
    }

    #[test]
    fn a_record_without_a_user_id_yields_no_pixiv_profile_candidate() {
        let record = adapter("pixiv", serde_json::json!({ "artist": "someone" }));

        assert_eq!(candidate(Some(&record), None), None);
    }

    /// An image stored with no record — a legacy-bundle import, a capture
    /// whose page never answered — offers the X account its page URL names,
    /// the account the Account row already shows for it.
    #[test]
    fn an_image_without_a_record_offers_its_x_page_as_a_profile_candidate() {
        assert_eq!(
            candidate(
                None,
                Some("https://x.com/Alice_Art/status/1984922180078211565/photo/1")
            ),
            normalized("https://x.com/alice_art").ok(),
        );
        assert_eq!(
            candidate(None, Some("https://twitter.com/alice_art/status/1")),
            normalized("https://x.com/alice_art").ok(),
        );
        assert_eq!(
            derived_tag(None, Some("https://x.com/Alice_Art/status/1")),
            Some("alice_art".to_string()),
        );
    }

    #[test]
    fn an_image_without_a_record_off_an_account_page_offers_no_candidate() {
        assert_eq!(candidate(None, Some("https://x.com/home")), None);
        assert_eq!(candidate(None, Some("https://x.com/i/status/1")), None);
        assert_eq!(
            candidate(None, Some("https://www.pixiv.net/artworks/1")),
            None
        );
        assert_eq!(candidate(None, None), None);
        assert_eq!(derived_tag(None, Some("https://x.com/home")), None);
    }

    /// The page is the tab's address, not the author's (design D4): a record
    /// that yields no profile URL never falls back to it.
    #[test]
    fn a_record_never_defers_to_the_page_url() {
        let record = adapter("pixiv", serde_json::json!({ "artist": "someone" }));
        assert_eq!(
            candidate(Some(&record), Some("https://x.com/alice_art/status/1")),
            None
        );
        assert_eq!(
            derived_tag(Some(&record), Some("https://x.com/alice_art/status/1")),
            Some("someone".to_string())
        );
    }

    // ---- the store (task 1.4) ----

    fn library() -> (tempfile::TempDir, Library) {
        let dir = tempfile::tempdir().unwrap();
        let library = Library::open_or_create(dir.path()).unwrap();
        (dir, library)
    }

    // ---- owning_entry (task 1.1) ----

    #[test]
    fn owning_entry_picks_the_longest_of_two_nested_owners() {
        let entries = vec![entry("a", &["x.com/foo"]), entry("b", &["x.com/foo/bar"])];
        let candidate = normalized("https://x.com/foo/bar").unwrap();

        let owner = owning_entry(&candidate, &entries);

        assert_eq!(owner.map(|entry| entry.tag.as_str()), Some("b"));
    }

    #[test]
    fn owning_entry_answers_none_for_the_neighbour() {
        let entries = vec![entry("a", &["x.com/metaljelly0811"])];
        let candidate = normalized("https://x.com/metaljelly08110").unwrap();

        assert_eq!(owning_entry(&candidate, &entries), None);
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

    /// A host-only Fanbox profile and the `fanbox.cc/@name` path spelling
    /// both save on one entry (design D3, `artist-url-any-site`): Fanbox is
    /// not a `path_identity_hosts` host, so neither line is refused.
    #[test]
    fn upsert_accepts_a_fanbox_profile() {
        let (_dir, library) = library();

        let entries = upsert(
            &library,
            &entry(
                "kanibiimu",
                &[
                    "https://kanibiimu.fanbox.cc/",
                    "https://www.fanbox.cc/@kanibiimu",
                ],
            ),
        )
        .unwrap();

        let mut urls = entries[0].urls.clone();
        urls.sort();
        assert_eq!(
            urls,
            vec![
                "fanbox.cc/@kanibiimu".to_string(),
                "kanibiimu.fanbox.cc".to_string(),
            ]
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
        tags::set_note(&library, "metaljelly0811", Some("also posts as jelly")).unwrap();

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
        let note: Option<String> = library
            .conn
            .query_row(
                "SELECT note FROM tags WHERE name = 'metaljelly'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(note, Some("also posts as jelly".to_string()));
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
        crate::tags::groups_are_consistent(&library.conn);
    }

    /// `tag-notes` design D5, the pinned-group rule beside it: a merge
    /// carries `from`'s note onto `to` only when `to` has none of its own —
    /// the owner's note on the handle is what the merge is confirming, but a
    /// note `to` already carries is not silently overwritten.
    #[test]
    fn a_merge_carries_the_note_when_the_target_has_none_and_keeps_the_targets_own() {
        let (_dir, library) = library();
        store_captured(&library, "a", &["artist:kantoku_(pixiv)"]);
        store_captured(&library, "b", &["artist:kantoku"]);
        tags::set_note(&library, "kantoku_(pixiv)", Some("danbooru: kantoku")).unwrap();

        rename(
            &library,
            &RenameArtistInput {
                from: "kantoku_(pixiv)".to_string(),
                to: "kantoku".to_string(),
                urls: vec![],
            },
        )
        .unwrap();

        let note: Option<String> = library
            .conn
            .query_row("SELECT note FROM tags WHERE name = 'kantoku'", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(note, Some("danbooru: kantoku".to_string()));

        // The other order: `to` already has a note of its own, `from`'s is
        // never overwritten onto it.
        store_captured(&library, "c", &["artist:someone_(twitter)"]);
        store_captured(&library, "d", &["artist:someone"]);
        tags::set_note(&library, "someone_(twitter)", Some("stale, never applied")).unwrap();
        tags::set_note(&library, "someone", Some("the target's own note")).unwrap();

        rename(
            &library,
            &RenameArtistInput {
                from: "someone_(twitter)".to_string(),
                to: "someone".to_string(),
                urls: vec![],
            },
        )
        .unwrap();

        let note: Option<String> = library
            .conn
            .query_row("SELECT note FROM tags WHERE name = 'someone'", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(note, Some("the target's own note".to_string()));
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

    // ---- preview (task 1.2, replacing rename_preview) ----

    #[test]
    fn artist_preview_lists_the_entrys_urls_and_counts_trash() {
        let (_dir, library) = library();
        store_captured(&library, "a", &["artist:metaljelly"]);
        store_captured(&library, "b", &["artist:metaljelly"]);
        crate::trash::trash_images(&library, &["b".to_string()]).unwrap();
        upsert(
            &library,
            &entry("metaljelly", &["https://x.com/metaljelly0811"]),
        )
        .unwrap();

        let preview = preview(
            &library.conn,
            &ArtistPreviewInput {
                tag: "metaljelly".to_string(),
                adapter: None,
                page_url: None,
            },
        )
        .unwrap();

        assert_eq!(preview.carriers, 2, "trash included");
        assert_eq!(
            preview.urls,
            vec!["https://x.com/metaljelly0811".to_string()]
        );
        assert_eq!(preview.candidate, None, "no adapter, no candidate");
    }

    #[test]
    fn artist_preview_offers_the_page_url_of_an_image_without_a_record() {
        let (_dir, library) = library();
        let preview = preview(
            &library.conn,
            &ArtistPreviewInput {
                tag: "alice".to_string(),
                adapter: None,
                page_url: Some("https://x.com/Alice_Art/status/1/photo/1".to_string()),
            },
        )
        .unwrap();
        assert_eq!(
            preview.candidate,
            Some("https://x.com/alice_art".to_string())
        );
    }

    #[test]
    fn artist_preview_offers_an_unowned_candidate() {
        let (_dir, library) = library();
        let record = adapter("x", serde_json::json!({ "handle": "alice_art" }));

        let preview = preview(
            &library.conn,
            &ArtistPreviewInput {
                tag: "alice_art".to_string(),
                adapter: Some(record),
                page_url: None,
            },
        )
        .unwrap();

        assert_eq!(
            preview.candidate,
            Some("https://x.com/alice_art".to_string())
        );
    }

    #[test]
    fn artist_preview_withholds_a_candidate_this_entry_owns() {
        let (_dir, library) = library();
        upsert(
            &library,
            &entry("metaljelly", &["https://x.com/metaljelly0811"]),
        )
        .unwrap();
        let record = adapter("x", serde_json::json!({ "handle": "MetalJelly0811" }));

        let preview = preview(
            &library.conn,
            &ArtistPreviewInput {
                tag: "metaljelly".to_string(),
                adapter: Some(record),
                page_url: None,
            },
        )
        .unwrap();

        assert_eq!(preview.candidate, None, "already among this entry's urls");
    }

    #[test]
    fn artist_preview_withholds_a_candidate_another_entry_owns() {
        let (_dir, library) = library();
        upsert(&library, &entry("bob", &["https://x.com/bob_art"])).unwrap();
        let record = adapter("x", serde_json::json!({ "handle": "bob_art" }));

        let preview = preview(
            &library.conn,
            &ArtistPreviewInput {
                tag: "alice".to_string(),
                adapter: Some(record),
                page_url: None,
            },
        )
        .unwrap();

        assert_eq!(preview.candidate, None, "another artist owns it");
    }

    #[test]
    fn artist_preview_of_a_tag_without_entry_has_no_urls() {
        let (_dir, library) = library();
        store_captured(&library, "a", &["someone"]);

        let preview = preview(
            &library.conn,
            &ArtistPreviewInput {
                tag: "someone".to_string(),
                adapter: None,
                page_url: None,
            },
        )
        .unwrap();

        assert!(preview.urls.is_empty());
        assert_eq!(preview.carriers, 1);
    }

    #[test]
    fn a_pixiv_record_without_a_user_id_offers_no_candidate_in_the_preview() {
        let (_dir, library) = library();
        let record = adapter("pixiv", serde_json::json!({ "artist": "someone" }));

        let preview = preview(
            &library.conn,
            &ArtistPreviewInput {
                tag: "someone".to_string(),
                adapter: Some(record),
                page_url: None,
            },
        )
        .unwrap();

        assert_eq!(preview.candidate, None);
    }

    // ---- artist_match (task 1.3) ----

    #[test]
    fn artist_match_names_the_owner_and_the_derived_tag() {
        let (_dir, library) = library();
        upsert(
            &library,
            &entry("metaljelly", &["https://x.com/metaljelly0811"]),
        )
        .unwrap();
        let record = adapter("x", serde_json::json!({ "handle": "MetalJelly0811" }));

        let matched = artist_match(&library.conn, Some(&record), None)
            .unwrap()
            .unwrap();

        assert_eq!(matched.url, "https://x.com/metaljelly0811");
        assert_eq!(matched.owner, Some("metaljelly".to_string()));
        assert_eq!(matched.derived, Some("metaljelly0811".to_string()));
    }

    #[test]
    fn artist_match_without_owner_has_only_the_derived_tag() {
        let (_dir, library) = library();
        let record = adapter("x", serde_json::json!({ "handle": "alice_art" }));

        let matched = artist_match(&library.conn, Some(&record), None)
            .unwrap()
            .unwrap();

        assert_eq!(matched.owner, None);
        assert_eq!(matched.derived, Some("alice_art".to_string()));
    }

    #[test]
    fn artist_match_of_a_pixiv_user() {
        let (_dir, library) = library();
        let record = adapter(
            "pixiv",
            serde_json::json!({ "artist": "someone", "userId": "3439325" }),
        );

        let matched = artist_match(&library.conn, Some(&record), None)
            .unwrap()
            .unwrap();

        assert_eq!(matched.url, "https://pixiv.net/users/3439325");
        assert_eq!(matched.derived, Some("someone".to_string()));
    }

    #[test]
    fn artist_match_of_an_owned_pixiv_user() {
        let (_dir, library) = library();
        upsert(
            &library,
            &entry("kani_beam", &["https://www.pixiv.net/users/3439325"]),
        )
        .unwrap();
        let record = adapter(
            "pixiv",
            serde_json::json!({ "artist": "someone", "userId": "3439325" }),
        );

        let matched = artist_match(&library.conn, Some(&record), None)
            .unwrap()
            .unwrap();

        assert_eq!(matched.owner, Some("kani_beam".to_string()));
        assert_eq!(matched.derived, Some("someone".to_string()));
    }

    /// `artist_match`'s owner ignores a stale category (`artist-workflow`
    /// design D5's own risk note): the chip still names who owns the URL once
    /// `set_category` has moved the entry's tag out of `artist` — the
    /// Artists list is where a stale category gets fixed, not the chip.
    #[test]
    fn artist_match_names_the_owner_even_once_its_tag_left_the_artist_category() {
        let (_dir, library) = library();
        upsert(
            &library,
            &entry("kani_beam", &["https://www.pixiv.net/users/3439325"]),
        )
        .unwrap();
        store_captured(&library, "seed", &["kani_beam"]);
        tags::set_category(&library, "kani_beam", TagCategory::General).unwrap();
        let record = adapter(
            "pixiv",
            serde_json::json!({ "artist": "someone", "userId": "3439325" }),
        );

        let matched = artist_match(&library.conn, Some(&record), None)
            .unwrap()
            .unwrap();

        assert_eq!(matched.owner, Some("kani_beam".to_string()));
    }

    #[test]
    fn artist_match_is_none_without_a_profile_url() {
        let (_dir, library) = library();
        let pixiv = adapter("pixiv", serde_json::json!({ "artist": "someone" }));
        assert_eq!(
            artist_match(&library.conn, Some(&pixiv), None).unwrap(),
            None
        );

        let danbooru = adapter("danbooru", serde_json::json!({ "artist": "someone" }));
        assert_eq!(
            artist_match(&library.conn, Some(&danbooru), None).unwrap(),
            None
        );
    }

    #[test]
    fn artist_match_of_an_image_without_a_record_reads_its_page_url() {
        let (_dir, library) = library();
        upsert(&library, &entry("alice", &["https://x.com/alice_art"])).unwrap();

        let matched = artist_match(
            &library.conn,
            None,
            Some("https://x.com/Alice_Art/status/1984922180078211565/photo/1"),
        )
        .unwrap()
        .unwrap();

        assert_eq!(matched.url, "https://x.com/alice_art");
        assert_eq!(matched.owner, Some("alice".to_string()));
        assert_eq!(matched.derived, Some("alice_art".to_string()));
    }

    #[test]
    fn artist_match_is_none_for_an_image_without_a_record_off_an_account_page() {
        let (_dir, library) = library();
        assert_eq!(
            artist_match(&library.conn, None, Some("https://x.com/home")).unwrap(),
            None
        );
        assert_eq!(
            artist_match(
                &library.conn,
                None,
                Some("https://www.pixiv.net/artworks/1")
            )
            .unwrap(),
            None
        );
        assert_eq!(artist_match(&library.conn, None, None).unwrap(), None);
    }

    // ---- apply / apply_preview (task 1.4) ----

    /// Stores a plain, adapter-less image (no capture-time derivation runs),
    /// then attaches an adapter record directly to the row — an image
    /// carrying a profile URL but no artist tag of its own, the shape
    /// `apply`'s own scan (`from_profiles`) has to tell from one that does.
    fn store_with_adapter(
        library: &Library,
        id: &str,
        site: &str,
        fields: serde_json::Value,
        tags: &[&str],
    ) {
        store_captured(library, id, tags);
        let json = serde_json::to_string(&adapter(site, fields)).unwrap();
        library
            .conn
            .execute(
                "UPDATE images SET adapter_json = ?1 WHERE id = ?2",
                params![json, id],
            )
            .unwrap();
    }

    /// Stores an image with no adapter record at all and the page it came
    /// from — the shape of a legacy-bundle import.
    fn store_at_page(library: &Library, id: &str, page_url: &str, tags: &[&str]) {
        store_captured(library, id, tags);
        library
            .conn
            .execute(
                "UPDATE images SET page_url = ?1 WHERE id = ?2",
                params![page_url, id],
            )
            .unwrap();
    }

    #[test]
    fn apply_tags_an_image_without_a_record_by_its_page_url() {
        let (_dir, library) = library();
        store_at_page(
            &library,
            "a",
            "https://x.com/Alice_Art/status/1/photo/1",
            &[],
        );
        store_at_page(
            &library,
            "b",
            "https://x.com/someone_else/status/2/photo/1",
            &[],
        );
        store_at_page(&library, "c", "https://x.com/home", &[]);
        upsert(&library, &entry("alice", &["https://x.com/alice_art"])).unwrap();

        let preview =
            apply_preview(&library.conn, &["https://x.com/alice_art".to_string()]).unwrap();
        assert_eq!(preview.images, 1);

        let report = apply(&library, "alice").unwrap();
        assert_eq!(
            report,
            ArtistApplyReport {
                tagged: 1,
                skipped: 0
            }
        );
        let tags_of = |id: &str| ingest::require_record(&library.conn, id).unwrap().tags;
        assert_eq!(tags_of("a"), vec!["alice".to_string()]);
        assert!(tags_of("b").is_empty());
        assert!(tags_of("c").is_empty());
    }

    #[test]
    fn apply_tags_only_images_whose_profile_the_entry_owns() {
        let (_dir, library) = library();
        store_with_adapter(
            &library,
            "a",
            "x",
            serde_json::json!({ "handle": "alice_art" }),
            &[],
        );
        store_with_adapter(
            &library,
            "b",
            "x",
            serde_json::json!({ "handle": "alice_art2" }),
            &[],
        );
        store_with_adapter(
            &library,
            "c",
            "danbooru",
            serde_json::json!({ "artist": "alice_art" }),
            &[],
        );
        upsert(&library, &entry("alice", &["https://x.com/alice_art"])).unwrap();

        let report = apply(&library, "alice").unwrap();

        assert_eq!(
            report,
            ArtistApplyReport {
                tagged: 1,
                skipped: 0
            }
        );
        assert_eq!(
            ingest::require_record(&library.conn, "a").unwrap().tags,
            vec!["alice".to_string()]
        );
        assert!(
            ingest::require_record(&library.conn, "b")
                .unwrap()
                .tags
                .is_empty()
        );
        assert!(
            ingest::require_record(&library.conn, "c")
                .unwrap()
                .tags
                .is_empty()
        );
    }

    /// `apply` and `preview` look `tag` up the same way `upsert` and `rename`
    /// store one — `tags::underscored`, not `tags::canonical` — so a name
    /// with inner whitespace still reaches the entry `upsert` saved under it.
    #[test]
    fn apply_of_a_name_with_whitespace_reaches_the_underscored_entry() {
        let (_dir, library) = library();
        store_with_adapter(
            &library,
            "a",
            "x",
            serde_json::json!({ "handle": "alice_art" }),
            &[],
        );
        upsert(&library, &entry("alice_art", &["https://x.com/alice_art"])).unwrap();

        let report = apply(&library, "alice art").unwrap();

        assert_eq!(
            report,
            ArtistApplyReport {
                tagged: 1,
                skipped: 0
            }
        );
        assert_eq!(
            ingest::require_record(&library.conn, "a").unwrap().tags,
            vec!["alice_art".to_string()]
        );
    }

    #[test]
    fn apply_is_additive() {
        let (_dir, library) = library();
        store_with_adapter(
            &library,
            "a",
            "x",
            serde_json::json!({ "handle": "alice_art" }),
            &["cat", "artist:alice_art"],
        );
        upsert(&library, &entry("alice", &["https://x.com/alice_art"])).unwrap();

        apply(&library, "alice").unwrap();

        assert_eq!(
            ingest::require_record(&library.conn, "a").unwrap().tags,
            vec![
                "alice".to_string(),
                "alice_art".to_string(),
                "cat".to_string()
            ]
        );
    }

    #[test]
    fn apply_twice_tags_nothing_the_second_time() {
        let (_dir, library) = library();
        store_with_adapter(
            &library,
            "a",
            "x",
            serde_json::json!({ "handle": "alice_art" }),
            &[],
        );
        upsert(&library, &entry("alice", &["https://x.com/alice_art"])).unwrap();

        let first = apply(&library, "alice").unwrap();
        let second = apply(&library, "alice").unwrap();

        assert_eq!(
            first,
            ArtistApplyReport {
                tagged: 1,
                skipped: 0
            }
        );
        assert_eq!(
            second,
            ArtistApplyReport {
                tagged: 0,
                skipped: 1
            }
        );
    }

    #[test]
    fn apply_leaves_the_trash_alone() {
        let (_dir, library) = library();
        store_with_adapter(
            &library,
            "a",
            "x",
            serde_json::json!({ "handle": "alice_art" }),
            &[],
        );
        crate::trash::trash_images(&library, &["a".to_string()]).unwrap();
        upsert(&library, &entry("alice", &["https://x.com/alice_art"])).unwrap();

        let report = apply(&library, "alice").unwrap();

        assert_eq!(
            report,
            ArtistApplyReport {
                tagged: 0,
                skipped: 0
            }
        );
        crate::trash::restore_images(&library, &["a".to_string()]).unwrap();
        assert!(
            !ingest::require_record(&library.conn, "a")
                .unwrap()
                .tags
                .contains(&"alice".to_string())
        );
    }

    #[test]
    fn apply_rewrites_the_sidecars_of_the_images_it_tagged() {
        let (_dir, library) = library();
        store_with_adapter(
            &library,
            "a",
            "x",
            serde_json::json!({ "handle": "alice_art" }),
            &[],
        );
        store_with_adapter(
            &library,
            "b",
            "x",
            serde_json::json!({ "handle": "alice_art2" }),
            &[],
        );
        upsert(&library, &entry("alice", &["https://x.com/alice_art"])).unwrap();

        apply(&library, "alice").unwrap();

        let sidecar_a = crate::sidecar::read(&crate::sidecar::path(&library.paths, "a")).unwrap();
        assert!(sidecar_a.tags.contains(&"alice".to_string()));
        let sidecar_b = crate::sidecar::read(&crate::sidecar::path(&library.paths, "b")).unwrap();
        assert!(
            !sidecar_b.tags.contains(&"alice".to_string()),
            "an untouched image's sidecar is unchanged"
        );
    }

    #[test]
    fn apply_moves_updated_at_of_tagged_images_only() {
        let (_dir, library) = library();
        store_with_adapter(
            &library,
            "a",
            "x",
            serde_json::json!({ "handle": "alice_art" }),
            &[],
        );
        store_with_adapter(
            &library,
            "b",
            "x",
            serde_json::json!({ "handle": "alice_art2" }),
            &[],
        );
        let before = 1_600_000_000_000_i64;
        for id in ["a", "b"] {
            library
                .conn
                .execute(
                    "UPDATE images SET updated_at = ?1 WHERE id = ?2",
                    params![before, id],
                )
                .unwrap();
        }
        upsert(&library, &entry("alice", &["https://x.com/alice_art"])).unwrap();

        apply(&library, "alice").unwrap();

        let after_a = ingest::require_record(&library.conn, "a")
            .unwrap()
            .updated_at;
        let after_b = ingest::require_record(&library.conn, "b")
            .unwrap()
            .updated_at;
        assert!(after_a > before, "the tagged image's updated_at moves");
        assert_eq!(after_b, before, "the untouched image's updated_at does not");
    }

    #[test]
    fn apply_refuses_a_tag_without_entry() {
        let (_dir, library) = library();
        store_captured(&library, "a", &["cat"]);

        let error = apply(&library, "nobody").unwrap_err();

        assert!(matches!(error, AppError::NotFound(_)), "got {error}");
        assert_eq!(
            ingest::require_record(&library.conn, "a").unwrap().tags,
            vec!["cat".to_string()]
        );
    }

    #[test]
    fn apply_refuses_a_tag_of_another_category() {
        let (_dir, library) = library();
        upsert(
            &library,
            &entry("kani_beam", &["https://www.pixiv.net/users/3439325"]),
        )
        .unwrap();
        store_captured(&library, "seed", &["kani_beam"]);
        tags::set_category(&library, "kani_beam", TagCategory::General).unwrap();
        // Carries the entry's own URL, so a refusal that stopped happening
        // would actually link it — "a" left untagged is not, by itself,
        // evidence the refusal fired.
        store_with_adapter(
            &library,
            "a",
            "pixiv",
            serde_json::json!({ "userId": "3439325" }),
            &["cat"],
        );

        let error = apply(&library, "kani_beam").unwrap_err();

        assert!(matches!(error, AppError::BadRequest(_)), "got {error}");
        assert_eq!(
            ingest::require_record(&library.conn, "a").unwrap().tags,
            vec!["cat".to_string()]
        );
    }

    #[test]
    fn apply_preview_counts_images_and_untagged() {
        let (_dir, library) = library();
        store_with_adapter(
            &library,
            "a",
            "x",
            serde_json::json!({ "handle": "alice_art" }),
            &["artist:alice"],
        );
        store_with_adapter(
            &library,
            "b",
            "x",
            serde_json::json!({ "handle": "alice_art" }),
            &[],
        );
        store_with_adapter(
            &library,
            "c",
            "x",
            serde_json::json!({ "handle": "alice_art" }),
            &[],
        );
        crate::trash::trash_images(&library, &["c".to_string()]).unwrap();

        let preview = apply_preview(
            &library.conn,
            &[
                "https://x.com/alice_art".to_string(),
                "not a url".to_string(),
            ],
        )
        .unwrap();

        assert_eq!(preview.images, 2, "trash excluded, the bad URL ignored");
        assert_eq!(preview.untagged, 1);
    }
}
