//! The library's list of configured booru sites (`booru-sites` design D1,
//! D2): CRUD over the `booru_sites` table, the host-slug id a site keeps for
//! life, and the credential write/removal that rides along with saving and
//! deleting a site.

use rusqlite::{Connection, Row, params};

use crate::booru::credentials::Credentials;
use crate::db;
use crate::error::{AppError, Result};
use crate::library::Library;
use crate::model::BooruSite;

const SITE_COLUMNS: &str = "id, name, base_url, username, created_at, updated_at";

fn row_to_site(row: &Row) -> rusqlite::Result<BooruSite> {
    Ok(BooruSite {
        id: row.get(0)?,
        name: row.get(1)?,
        base_url: row.get(2)?,
        username: row.get(3)?,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
    })
}

/// Every configured site, ordered by name case-insensitively — there is no
/// other order for a human to scan by, the same reasoning `rules::list` uses.
pub fn list(conn: &Connection) -> Result<Vec<BooruSite>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {SITE_COLUMNS} FROM booru_sites ORDER BY name COLLATE NOCASE"
    ))?;
    let rows = stmt.query_map([], row_to_site)?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

pub fn require(conn: &Connection, id: &str) -> Result<BooruSite> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {SITE_COLUMNS} FROM booru_sites WHERE id = ?1"
    ))?;
    match stmt.query_row([id], row_to_site) {
        Ok(site) => Ok(site),
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            Err(AppError::NotFound(format!("booru site {id}")))
        }
        Err(error) => Err(error.into()),
    }
}

/// The host `Credentials` keys an account on (design D7): parsed out of
/// `base_url` on every call rather than stored beside it, so the address is
/// the one source of truth for what a site's account is.
pub fn host_of(base_url: &str) -> Result<String> {
    let url = reqwest::Url::parse(base_url).map_err(|error| {
        AppError::BadRequest(format!("{base_url:?} is not a valid address: {error}"))
    })?;
    url.host_str()
        .map(str::to_string)
        .ok_or_else(|| AppError::BadRequest(format!("{base_url:?} has no host")))
}

/// `base_url`'s host, lowercased with non-alphanumerics collapsed to `-` and
/// suffixed `-2`, `-3`, … on collision within this library (design D2). The
/// port never appears in it — `Url::host_str` already excludes it, and two
/// instances on the same host at different ports would otherwise get the same
/// slug on the first save and the suffix on the second, which is exactly what
/// the collision rule is for.
fn derive_id(conn: &Connection, base_url: &str) -> Result<String> {
    let slug = slugify(&host_of(base_url)?);
    let mut candidate = slug.clone();
    let mut suffix = 2;
    while site_exists(conn, &candidate)? {
        candidate = format!("{slug}-{suffix}");
        suffix += 1;
    }
    Ok(candidate)
}

/// The id a host with nothing alphanumeric in it gets. `[::]` — an IPv6
/// literal, brackets and all, as `Url::host_str` reports it — collapses to
/// the empty string, and an empty id would give the first such site the id
/// `""` and the next one `-2`.
const HOSTLESS_SLUG: &str = "booru";

fn slugify(host: &str) -> String {
    let mut out = String::new();
    let mut last_was_dash = false;
    for ch in host.to_lowercase().chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            last_was_dash = false;
        } else if !last_was_dash {
            out.push('-');
            last_was_dash = true;
        }
    }
    let slug = out.trim_matches('-');
    if slug.is_empty() {
        return HOSTLESS_SLUG.to_string();
    }
    slug.to_string()
}

fn site_exists(conn: &Connection, id: &str) -> Result<bool> {
    let exists: i64 = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM booru_sites WHERE id = ?1)",
        [id],
        |row| row.get(0),
    )?;
    Ok(exists != 0)
}

/// The site already using `base_url`, other than `excluding_id` — what "two
/// sites SHALL NOT share a base address" (`booru-sites`) is checked against,
/// so the refusal can name it. `excluding_id` is `None` on create, where
/// nothing is excluded.
fn conflicting_site(
    conn: &Connection,
    base_url: &str,
    excluding_id: Option<&str>,
) -> Result<Option<BooruSite>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {SITE_COLUMNS} FROM booru_sites WHERE base_url = ?1 AND id IS NOT ?2"
    ))?;
    match stmt.query_row(params![base_url, excluding_id], row_to_site) {
        Ok(site) => Ok(Some(site)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(error) => Err(error.into()),
    }
}

/// Whether any site other than `excluding_id` still posts as
/// `(host, username)` — the guard on removing that account's stored key
/// (design D7). A keychain entry is keyed on the account, and one account can
/// serve several sites: the same host on two ports is two rows and one booru
/// account, so removing or re-pointing one of them must not take the other's
/// key with it. The host is parsed per row because it lives in `base_url`,
/// not in a column of its own (`host_of`); a row whose address will not parse
/// cannot be claiming any account, so it is skipped.
fn account_is_still_in_use(
    conn: &Connection,
    host: &str,
    username: &str,
    excluding_id: &str,
) -> Result<bool> {
    let mut stmt =
        conn.prepare("SELECT base_url FROM booru_sites WHERE username = ?1 AND id <> ?2")?;
    let rows = stmt.query_map(params![username, excluding_id], |row| {
        row.get::<_, String>(0)
    })?;
    for base_url in rows {
        if host_of(&base_url?).is_ok_and(|other| other == host) {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Remove the key for `(host, username)` unless another site still posts as
/// that account (design D7).
fn forget_account(
    library: &Library,
    credentials: &dyn Credentials,
    host: &str,
    username: &str,
    excluding_id: &str,
) -> Result<()> {
    if account_is_still_in_use(&library.conn, host, username, excluding_id)? {
        return Ok(());
    }
    credentials.delete(host, username)
}

/// Create a site when `id` is absent, or edit the one it names — `id` itself
/// never changes on an edit, even one that changes `base_url`'s host (design
/// D2). `api_key` is `None` to leave whatever credential is already stored at
/// the (possibly new) `(host, username)` account untouched — the write-only
/// form field this backs starts blank on every edit (`booru-sites` design,
/// task 5.2), and re-saving a display name must not demand the key again. An
/// edit that moves the site to another account drops the key stored for the
/// one it left, behind `forget_account`'s guard.
///
/// The database write and the credential write are deliberately two separate
/// outcomes: `booru-sites` design D7 requires "the site's other settings
/// saved" even when the credential store refuses, so the row is committed
/// before `credentials.set` is ever called, and its error — if any — is the
/// only thing this call fails with.
pub fn save(
    library: &Library,
    credentials: &dyn Credentials,
    id: Option<&str>,
    name: &str,
    base_url: &str,
    username: &str,
    api_key: Option<&str>,
) -> Result<BooruSite> {
    let name = name.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest("a site needs a name".to_string()));
    }
    let username = username.trim();
    if username.is_empty() {
        return Err(AppError::BadRequest("a site needs a username".to_string()));
    }
    // Validates the address as a side effect: an invalid one is refused here,
    // before anything is written.
    let host = host_of(base_url)?;

    if let Some(existing) = conflicting_site(&library.conn, base_url, id)? {
        return Err(AppError::BadRequest(format!(
            "{base_url:?} is already configured as {:?}",
            existing.name
        )));
    }

    let now = db::now_ms();
    // Read before the write: an edit that changes the host or the username
    // changes which booru account the site posts as, and the key stored for
    // the old one is then nobody's unless another site still uses it (design
    // D7).
    let previous_account = match id {
        Some(id) => {
            let previous = require(&library.conn, id)?;
            Some((host_of(&previous.base_url)?, previous.username))
        }
        None => None,
    };

    let site = match id {
        Some(id) => {
            let changed = library.conn.execute(
                "UPDATE booru_sites SET name = ?1, base_url = ?2, username = ?3, updated_at = ?4
                 WHERE id = ?5",
                params![name, base_url, username, now, id],
            )?;
            if changed == 0 {
                return Err(AppError::NotFound(format!("booru site {id}")));
            }
            require(&library.conn, id)?
        }
        None => {
            let new_id = derive_id(&library.conn, base_url)?;
            library.conn.execute(
                "INSERT INTO booru_sites (id, name, base_url, username, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
                params![new_id, name, base_url, username, now],
            )?;
            require(&library.conn, &new_id)?
        }
    };

    // Library-level, after the commit (`library-sidecars` design D3, D5): a
    // single `execute` outside a transaction is already its own commit.
    crate::sidecar::write_library(&library.paths, &library.conn)?;

    if let Some(api_key) = api_key {
        credentials.set(&host, username, api_key)?;
    }
    // After the new key, not before: a credential store that refuses must
    // cost the stale entry, never the key the user just entered.
    if let Some((previous_host, previous_username)) = previous_account
        && (previous_host.as_str(), previous_username.as_str()) != (host.as_str(), username)
    {
        forget_account(
            library,
            credentials,
            &previous_host,
            &previous_username,
            &site.id,
        )?;
    }

    Ok(site)
}

/// Remove a site and its stored credential (`booru-sites`: "Removing a site
/// SHALL remove its stored key") — unless another site still posts as the
/// same account, which is `forget_account`'s guard. Posts already recorded
/// against it are kept
/// — `posts.site` carries no foreign key to this table (design D2) — so
/// deleting the row here touches nothing under `posts`.
pub fn delete(library: &Library, credentials: &dyn Credentials, id: &str) -> Result<()> {
    let site = require(&library.conn, id)?;
    library
        .conn
        .execute("DELETE FROM booru_sites WHERE id = ?1", [id])?;
    crate::sidecar::write_library(&library.paths, &library.conn)?;
    let host = host_of(&site.base_url)?;
    forget_account(library, credentials, &host, &site.username, id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::booru::credentials::InMemoryCredentials;

    fn library() -> (tempfile::TempDir, Library) {
        let dir = tempfile::tempdir().unwrap();
        let library = Library::open_or_create(dir.path()).unwrap();
        (dir, library)
    }

    // ---- slug derivation (task 1.3) ----

    #[test]
    fn the_slug_is_the_host_without_its_port() {
        assert_eq!(
            slugify(&host_of("http://localhost:8080").unwrap()),
            "localhost"
        );
    }

    #[test]
    fn the_slug_is_lowercase_even_when_the_address_is_not() {
        assert_eq!(
            slugify(&host_of("https://Danbooru.Donmai.Us").unwrap()),
            "danbooru-donmai-us"
        );
    }

    #[test]
    fn punctuation_in_the_host_collapses_to_single_dashes() {
        assert_eq!(slugify("my---booru..local"), "my-booru-local");
        assert_eq!(slugify("-leading-and-trailing-"), "leading-and-trailing");
    }

    #[test]
    fn a_host_with_nothing_alphanumeric_in_it_still_gets_an_id() {
        assert_eq!(slugify(&host_of("http://[::]:3000").unwrap()), "booru");
        // A literal with digits in it keeps them; only an all-punctuation
        // host needs the fallback.
        assert_eq!(slugify(&host_of("http://[::1]:3000").unwrap()), "1");
    }

    #[test]
    fn a_second_site_on_such_a_host_is_suffixed_rather_than_sharing_an_empty_id() {
        let (_dir, library) = library();
        let credentials = InMemoryCredentials::default();

        let first = save(
            &library,
            &credentials,
            None,
            "First",
            "http://[::]:3000",
            "alice",
            None,
        )
        .unwrap();
        let second = save(
            &library,
            &credentials,
            None,
            "Second",
            "http://[::]:3001",
            "alice",
            None,
        )
        .unwrap();

        assert_eq!(first.id, "booru");
        assert_eq!(second.id, "booru-2");
    }

    #[test]
    fn a_second_site_on_the_same_host_gets_a_suffixed_id() {
        let (_dir, library) = library();
        let credentials = InMemoryCredentials::default();

        let first = save(
            &library,
            &credentials,
            None,
            "Primary",
            "http://localhost:3000",
            "alice",
            None,
        )
        .unwrap();
        let second = save(
            &library,
            &credentials,
            None,
            "Secondary",
            "http://localhost:3001",
            "alice",
            None,
        )
        .unwrap();

        assert_eq!(first.id, "localhost");
        assert_eq!(second.id, "localhost-2");
    }

    // ---- save / list / delete (task 1.3, 1.5) ----

    #[test]
    fn save_then_list_round_trips_a_site() {
        let (_dir, library) = library();
        let credentials = InMemoryCredentials::default();

        let created = save(
            &library,
            &credentials,
            None,
            "Danbooru",
            "https://danbooru.donmai.us",
            "alice",
            Some("secret-key"),
        )
        .unwrap();

        assert_eq!(created.id, "danbooru-donmai-us");
        let listed = list(&library.conn).unwrap();
        assert_eq!(listed, vec![created]);
        assert_eq!(
            credentials.get("danbooru.donmai.us", "alice").unwrap(),
            Some("secret-key".to_string())
        );
    }

    /// `library-sidecars` task 1.10: the site list is in `library.json` after
    /// a save, and removing a site removes it from the file.
    #[test]
    fn saving_and_deleting_a_site_are_each_visible_in_library_json() {
        let (_dir, library) = library();
        let credentials = InMemoryCredentials::default();
        let library_json = crate::sidecar::library_path(&library.paths);

        let site = save(
            &library,
            &credentials,
            None,
            "Danbooru",
            "https://danbooru.donmai.us",
            "alice",
            Some("secret-key"),
        )
        .unwrap();
        let file = crate::sidecar::read_library(&library_json).unwrap();
        assert_eq!(file.booru_sites.len(), 1);
        assert_eq!(file.booru_sites[0].name, "Danbooru");

        delete(&library, &credentials, &site.id).unwrap();
        let file = crate::sidecar::read_library(&library_json).unwrap();
        assert!(file.booru_sites.is_empty());
    }

    #[test]
    fn editing_keeps_the_id_even_when_the_host_changes() {
        let (_dir, library) = library();
        let credentials = InMemoryCredentials::default();
        let created = save(
            &library,
            &credentials,
            None,
            "Local",
            "http://localhost:3000",
            "alice",
            None,
        )
        .unwrap();

        let edited = save(
            &library,
            &credentials,
            Some(&created.id),
            "Local",
            "http://booru.example.lan",
            "alice",
            None,
        )
        .unwrap();

        assert_eq!(edited.id, created.id);
        assert_eq!(edited.base_url, "http://booru.example.lan");
        assert_eq!(list(&library.conn).unwrap().len(), 1);
    }

    #[test]
    fn a_duplicate_base_address_is_refused_and_names_the_existing_site() {
        let (_dir, library) = library();
        let credentials = InMemoryCredentials::default();
        save(
            &library,
            &credentials,
            None,
            "Danbooru",
            "https://danbooru.donmai.us",
            "alice",
            None,
        )
        .unwrap();

        let error = save(
            &library,
            &credentials,
            None,
            "Danbooru (again)",
            "https://danbooru.donmai.us",
            "bob",
            None,
        )
        .unwrap_err();

        assert!(matches!(&error, AppError::BadRequest(message) if message.contains("Danbooru")));
        assert_eq!(list(&library.conn).unwrap().len(), 1);
    }

    #[test]
    fn saving_a_site_with_no_credential_yet_leaves_it_unset() {
        let (_dir, library) = library();
        let credentials = InMemoryCredentials::default();

        save(
            &library,
            &credentials,
            None,
            "Danbooru",
            "https://danbooru.donmai.us",
            "alice",
            None,
        )
        .unwrap();

        assert_eq!(
            credentials.get("danbooru.donmai.us", "alice").unwrap(),
            None
        );
    }

    /// `booru-sites` design D7: the database write is unconditional, so a
    /// refusing credential store still leaves the site's other settings saved.
    #[test]
    fn a_refusing_credential_store_still_saves_the_site() {
        let (_dir, library) = library();
        let credentials = InMemoryCredentials::refusing("keychain is locked");

        let error = save(
            &library,
            &credentials,
            None,
            "Danbooru",
            "https://danbooru.donmai.us",
            "alice",
            Some("secret-key"),
        )
        .unwrap_err();

        assert!(matches!(error, AppError::Credential { .. }));
        let listed = list(&library.conn).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].name, "Danbooru");
    }

    #[test]
    fn delete_removes_the_site_and_its_credential_but_leaves_posts_intact() {
        let (_dir, library) = library();
        let credentials = InMemoryCredentials::default();
        let site = save(
            &library,
            &credentials,
            None,
            "Danbooru",
            "https://danbooru.donmai.us",
            "alice",
            Some("secret-key"),
        )
        .unwrap();
        library
            .conn
            .execute(
                "INSERT INTO images (id, ext, mime, size, width, height, source, captured_at,
                                     created_at, updated_at)
                 VALUES ('img-1', 'png', 'image/png', 1, 1, 1, 'local', 0, 0, 0)",
                [],
            )
            .unwrap();
        library
            .conn
            .execute(
                "INSERT INTO posts (image_id, site, remote_id, posted_at)
                 VALUES ('img-1', ?1, '42', 0)",
                [&site.id],
            )
            .unwrap();

        delete(&library, &credentials, &site.id).unwrap();

        assert!(list(&library.conn).unwrap().is_empty());
        assert_eq!(
            credentials.get("danbooru.donmai.us", "alice").unwrap(),
            None
        );
        let remaining: i64 = library
            .conn
            .query_row(
                "SELECT COUNT(*) FROM posts WHERE image_id = 'img-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(remaining, 1, "the post record must outlive the site");
    }

    // ---- one keychain entry, several sites (design D7) ----

    /// Two rows, one booru account: the same host on two ports. Removing one
    /// must leave the other able to post.
    fn two_sites_on_one_account(library: &Library, credentials: &dyn Credentials) -> BooruSite {
        save(
            library,
            credentials,
            None,
            "Primary",
            "http://booru.local:3000",
            "alice",
            Some("secret-key"),
        )
        .unwrap();
        save(
            library,
            credentials,
            None,
            "Secondary",
            "http://booru.local:3001",
            "alice",
            None,
        )
        .unwrap()
    }

    #[test]
    fn delete_keeps_the_key_while_another_site_still_posts_as_that_account() {
        let (_dir, library) = library();
        let credentials = InMemoryCredentials::default();
        let second = two_sites_on_one_account(&library, &credentials);

        delete(&library, &credentials, &second.id).unwrap();

        assert_eq!(
            credentials.get("booru.local", "alice").unwrap(),
            Some("secret-key".to_string()),
            "the site that is left still posts as this account"
        );
    }

    #[test]
    fn deleting_the_last_site_on_an_account_removes_its_key() {
        let (_dir, library) = library();
        let credentials = InMemoryCredentials::default();
        let second = two_sites_on_one_account(&library, &credentials);
        let first = list(&library.conn)
            .unwrap()
            .into_iter()
            .find(|site| site.id != second.id)
            .unwrap();

        delete(&library, &credentials, &second.id).unwrap();
        delete(&library, &credentials, &first.id).unwrap();

        assert_eq!(credentials.get("booru.local", "alice").unwrap(), None);
    }

    #[test]
    fn editing_the_username_removes_the_key_of_the_account_left_behind() {
        let (_dir, library) = library();
        let credentials = InMemoryCredentials::default();
        let site = save(
            &library,
            &credentials,
            None,
            "Danbooru",
            "https://danbooru.donmai.us",
            "alice",
            Some("alice-key"),
        )
        .unwrap();

        save(
            &library,
            &credentials,
            Some(&site.id),
            "Danbooru",
            "https://danbooru.donmai.us",
            "bob",
            Some("bob-key"),
        )
        .unwrap();

        assert_eq!(
            credentials.get("danbooru.donmai.us", "alice").unwrap(),
            None
        );
        assert_eq!(
            credentials.get("danbooru.donmai.us", "bob").unwrap(),
            Some("bob-key".to_string())
        );
    }

    #[test]
    fn editing_the_host_removes_the_key_of_the_account_left_behind() {
        let (_dir, library) = library();
        let credentials = InMemoryCredentials::default();
        let site = save(
            &library,
            &credentials,
            None,
            "Local",
            "http://booru.local:3000",
            "alice",
            Some("secret-key"),
        )
        .unwrap();

        save(
            &library,
            &credentials,
            Some(&site.id),
            "Local",
            "http://booru.example.lan",
            "alice",
            Some("secret-key"),
        )
        .unwrap();

        assert_eq!(credentials.get("booru.local", "alice").unwrap(), None);
        assert_eq!(
            credentials.get("booru.example.lan", "alice").unwrap(),
            Some("secret-key".to_string())
        );
    }

    #[test]
    fn editing_a_site_off_a_shared_account_keeps_the_key_the_other_site_uses() {
        let (_dir, library) = library();
        let credentials = InMemoryCredentials::default();
        let second = two_sites_on_one_account(&library, &credentials);

        save(
            &library,
            &credentials,
            Some(&second.id),
            "Secondary",
            "http://booru.local:3001",
            "bob",
            None,
        )
        .unwrap();

        assert_eq!(
            credentials.get("booru.local", "alice").unwrap(),
            Some("secret-key".to_string())
        );
    }

    #[test]
    fn re_saving_a_site_unchanged_keeps_its_key() {
        let (_dir, library) = library();
        let credentials = InMemoryCredentials::default();
        let site = save(
            &library,
            &credentials,
            None,
            "Danbooru",
            "https://danbooru.donmai.us",
            "alice",
            Some("secret-key"),
        )
        .unwrap();

        save(
            &library,
            &credentials,
            Some(&site.id),
            "Danbooru (renamed)",
            "https://danbooru.donmai.us",
            "alice",
            None,
        )
        .unwrap();

        assert_eq!(
            credentials.get("danbooru.donmai.us", "alice").unwrap(),
            Some("secret-key".to_string())
        );
    }

    #[test]
    fn delete_is_refused_for_a_site_that_does_not_exist() {
        let (_dir, library) = library();
        let credentials = InMemoryCredentials::default();

        let error = delete(&library, &credentials, "no-such-site").unwrap_err();

        assert!(matches!(error, AppError::NotFound(_)));
    }
}
