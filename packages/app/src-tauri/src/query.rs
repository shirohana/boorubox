//! Compiles a `ParsedTagSearch` into SQL — AND tags, OR groups, exclusions,
//! rating, `is:`, `tagcount:`, `account:` and the FTS5 free-text match — and
//! owns the `x_account()` scalar those account filters need (design D3).
//!
//! `src/lib/domain/filters.ts` is the tested statement of what each clause
//! means; where it and this file disagree, it is right and this file is the
//! bug. Read `filterByTagSearch` before changing anything here.

use rusqlite::functions::FunctionFlags;
use rusqlite::types::Value;
use rusqlite::{Connection, params_from_iter};

use crate::error::Result;
use crate::ingest;
use crate::model::{ParsedTagSearch, SearchRequest, SearchResult, TagCountOperator};

/// Register the SQL functions the compiled queries call. `db::open` calls this
/// for every connection, so a query may assume they are there.
pub fn register_functions(conn: &Connection) -> Result<()> {
    conn.create_scalar_function(
        "x_account",
        1,
        FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC,
        |ctx| {
            let url: Option<String> = ctx.get(0)?;
            Ok(url.and_then(|url| x_account(&url).map(str::to_string)))
        },
    )?;
    Ok(())
}

/// `?, ?, ?` for an `IN` list of `count` bound values. Every user value reaches
/// SQLite as a parameter; nothing but placeholders is ever formatted into SQL.
pub fn placeholders(count: usize) -> String {
    vec!["?"; count].join(", ")
}

pub fn search(conn: &Connection, req: &SearchRequest) -> Result<SearchResult> {
    let filter = compile(req);
    let where_sql = filter.sql();

    let total: i64 = conn.query_row(
        &format!("SELECT COUNT(*) FROM images WHERE {where_sql}"),
        params_from_iter(&filter.params),
        |row| row.get(0),
    )?;

    // `id` breaks the tie on equal `captured_at`. Without it SQLite may order
    // two same-instant rows differently between the page-1 and page-2 queries,
    // and a row then repeats on one page and is skipped on the other.
    let mut stmt = conn.prepare(&format!(
        "SELECT id FROM images
         WHERE {where_sql}
         ORDER BY images.captured_at DESC, images.id DESC
         LIMIT ? OFFSET ?"
    ))?;
    let mut page_params = filter.params.clone();
    page_params.push(Value::Integer(req.limit));
    page_params.push(Value::Integer(req.offset));
    let ids: Vec<String> = stmt
        .query_map(params_from_iter(&page_params), |row| row.get(0))?
        .collect::<rusqlite::Result<_>>()?;

    // `load_records` keeps the order it is given and fills the tags in one
    // extra statement, so the page costs three queries however large it is.
    Ok(SearchResult {
        images: ingest::load_records(conn, &ids)?,
        total,
    })
}

/// A `WHERE` fragment and the values its placeholders consume, kept together so
/// a clause cannot be added without its parameters — the two lists are read
/// positionally, and a clause pushed without its values silently shifts every
/// later binding.
#[derive(Default)]
struct Filter {
    clauses: Vec<String>,
    params: Vec<Value>,
}

impl Filter {
    fn add(&mut self, clause: impl Into<String>) {
        self.clauses.push(clause.into());
    }

    fn add_bound(&mut self, clause: impl Into<String>, values: impl IntoIterator<Item = Value>) {
        self.clauses.push(clause.into());
        self.params.extend(values);
    }

    fn sql(&self) -> String {
        if self.clauses.is_empty() {
            "1 = 1".to_string()
        } else {
            self.clauses.join("\n  AND ")
        }
    }
}

/// Stands in for a clause that can never be true. `IN ()` is not valid SQLite,
/// and filters.ts reaches the same answer for the cases that produce it: an
/// empty OR group (`[].some()` is false) and a `tagcount:` whose operand is
/// missing (every comparison against `undefined` is false).
const MATCHES_NOTHING: &str = "1 = 0";

/// The image's tag count, as a correlated subquery. filters.ts compares
/// `img.tags.length`; `image_tags` is that list.
const TAG_COUNT: &str = "(SELECT COUNT(*) FROM image_tags WHERE image_tags.image_id = images.id)";

fn compile(req: &SearchRequest) -> Filter {
    let query = &req.query;
    let mut filter = Filter::default();

    if !req.include_deleted {
        filter.add("images.deleted_at IS NULL");
    }
    push_rating(&mut filter, query);
    push_file_types(&mut filter, query);
    push_tag_count(&mut filter, query);
    push_accounts(&mut filter, query);
    push_tags(&mut filter, query);
    push_text(&mut filter, &req.text);
    filter
}

/// filters.ts keeps "unrated" and the rating list in ONE predicate: an image
/// passes if `includeUnrated` and it has no rating, OR its rating is listed.
/// Two AND-ed clauses would make `rating:s` plus unrated match nothing.
/// `!img.rating` is JS falsiness, so an empty rating string is unrated too.
fn push_rating(filter: &mut Filter, query: &ParsedTagSearch) {
    if query.ratings.is_empty() && !query.include_unrated {
        return;
    }
    let mut alternatives: Vec<String> = Vec::new();
    if query.include_unrated {
        alternatives.push("images.rating IS NULL OR images.rating = ''".to_string());
    }
    if !query.ratings.is_empty() {
        alternatives.push(format!(
            "images.rating IN ({})",
            placeholders(query.ratings.len())
        ));
    }
    filter.add_bound(
        format!("({})", alternatives.join(" OR ")),
        text_values(&query.ratings),
    );
}

fn push_file_types(filter: &mut Filter, query: &ParsedTagSearch) {
    if query.file_types.is_empty() {
        return;
    }
    filter.add_bound(
        format!("images.mime IN ({})", placeholders(query.file_types.len())),
        text_values(&query.file_types),
    );
}

fn push_tag_count(filter: &mut Filter, query: &ParsedTagSearch) {
    let Some(tag_count) = &query.tag_count else {
        return;
    };
    match tag_count.operator {
        TagCountOperator::Eq => push_tag_count_comparison(filter, "=", tag_count.value),
        TagCountOperator::Gt => push_tag_count_comparison(filter, ">", tag_count.value),
        TagCountOperator::Lt => push_tag_count_comparison(filter, "<", tag_count.value),
        TagCountOperator::Gte => push_tag_count_comparison(filter, ">=", tag_count.value),
        TagCountOperator::Lte => push_tag_count_comparison(filter, "<=", tag_count.value),
        TagCountOperator::Range => match (tag_count.min, tag_count.max) {
            // `BETWEEN` is inclusive on both ends, like `>= min && <= max`.
            (Some(min), Some(max)) => filter.add_bound(
                format!("{TAG_COUNT} BETWEEN ? AND ?"),
                [Value::Integer(min), Value::Integer(max)],
            ),
            _ => filter.add(MATCHES_NOTHING),
        },
        TagCountOperator::List => match tag_count.values.as_deref() {
            Some(values) if !values.is_empty() => filter.add_bound(
                format!("{TAG_COUNT} IN ({})", placeholders(values.len())),
                values
                    .iter()
                    .copied()
                    .map(Value::Integer)
                    .collect::<Vec<_>>(),
            ),
            _ => filter.add(MATCHES_NOTHING),
        },
    }
}

fn push_tag_count_comparison(filter: &mut Filter, operator: &str, value: Option<i64>) {
    match value {
        Some(value) => {
            filter.add_bound(format!("{TAG_COUNT} {operator} ?"), [Value::Integer(value)])
        }
        None => filter.add(MATCHES_NOTHING),
    }
}

/// `x_account()` is NULL for a page that is not an X account page, and
/// `NULL IN (…)` is NULL — never true — so the include side needs no null test.
/// The exclude side does: `NULL NOT IN (…)` is NULL as well, which would drop
/// every non-X image from `-account:alice` instead of keeping it.
fn push_accounts(filter: &mut Filter, query: &ParsedTagSearch) {
    if !query.accounts.is_empty() {
        filter.add_bound(
            format!(
                "x_account(images.page_url) IN ({})",
                placeholders(query.accounts.len())
            ),
            text_values(&query.accounts),
        );
    }
    if !query.exclude_accounts.is_empty() {
        filter.add_bound(
            format!(
                "(x_account(images.page_url) IS NULL
                   OR x_account(images.page_url) NOT IN ({}))",
                placeholders(query.exclude_accounts.len())
            ),
            text_values(&query.exclude_accounts),
        );
    }
}

fn push_tags(filter: &mut Filter, query: &ParsedTagSearch) {
    // AND: one `EXISTS` per tag, so a repeated tag costs a repeated test rather
    // than throwing the count of distinct names off.
    for tag in &query.include_tags {
        filter.add_bound(has_any_tag(1), [Value::Text(tag.clone())]);
    }
    for group in &query.or_groups {
        if group.is_empty() {
            filter.add(MATCHES_NOTHING);
            continue;
        }
        filter.add_bound(has_any_tag(group.len()), text_values(group));
    }
    if !query.exclude_tags.is_empty() {
        filter.add_bound(
            format!("NOT {}", has_any_tag(query.exclude_tags.len())),
            text_values(&query.exclude_tags),
        );
    }
}

/// "this image carries at least one of these tag names" — the shape every tag
/// clause is built from. Tag names compare with SQLite's BINARY collation,
/// which is the case-sensitive `Array.includes` filters.ts uses.
fn has_any_tag(count: usize) -> String {
    format!(
        "EXISTS (SELECT 1 FROM image_tags
                 JOIN tags ON tags.id = image_tags.tag_id
                 WHERE image_tags.image_id = images.id
                   AND tags.name IN ({}))",
        placeholders(count)
    )
}

fn push_text(filter: &mut Filter, text: &str) {
    let text = text.trim();
    if text.is_empty() {
        return;
    }
    filter.add_bound(
        "images.rowid IN (SELECT rowid FROM images_fts WHERE images_fts MATCH ?)",
        [Value::Text(fts_string(text))],
    );
}

/// Wrap what the user typed as one FTS5 string literal. Unquoted, `-`, `:`,
/// `*`, `^`, `(` and a lone `"` are FTS5 operators or a syntax error, and a
/// search box is not a query language — `text` is a second input beside the tag
/// syntax (design D14), so every character in it is data. Inside a
/// double-quoted string FTS5 reads none of them as syntax; doubling `"` is its
/// own escape for a quote within that string.
fn fts_string(text: &str) -> String {
    format!("\"{}\"", text.replace('"', "\"\""))
}

fn text_values(items: &[String]) -> Vec<Value> {
    items.iter().cloned().map(Value::Text).collect()
}

/// Hosts whose first path segment names an account.
const X_HOSTS: [&str; 4] = ["x.com", "twitter.com", "www.x.com", "www.twitter.com"];

/// First path segments that are X's own pages, not accounts.
const X_RESERVED: [&str; 6] = [
    "i",
    "home",
    "explore",
    "notifications",
    "messages",
    "search",
];

/// The Rust half of a two-language pair: `getXAccountFromUrl` in
/// `src/lib/domain/grouping.ts` is the same rule, and it is what builds the
/// X-account groups in the webview while this one backs `account:` in SQL.
/// Change one alone and `account:alice` and the alice group show different
/// images, with nothing failing.
///
/// FIXME: one rule, two implementations, no shared fixture. The right shape is
/// the webview asking Rust for its groups so this is the only copy; short of
/// that, one table of URLs both sides are tested against.
fn x_account(url: &str) -> Option<&str> {
    let (host, path) = host_and_path(url)?;
    if !X_HOSTS.contains(&host.to_lowercase().as_str()) {
        return None;
    }
    let segment = path.strip_prefix('/')?.split('/').next()?;
    if segment.is_empty() || X_RESERVED.contains(&segment.to_lowercase().as_str()) {
        return None;
    }
    Some(segment)
}

/// The two pieces of `new URL(...)` that rule reads: `hostname` (no userinfo,
/// no port, lower-cased by the caller) and `pathname`. `None` wherever the JS
/// constructor would throw and `getXAccountFromUrl` returns null — chiefly a
/// string with no scheme.
///
/// This is a splitter, not a URL parser: it does not resolve `..` segments or
/// re-encode anything, so a path the JS constructor would normalise is left as
/// written. X status URLs have neither.
fn host_and_path(url: &str) -> Option<(&str, &str)> {
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

/// Mark `id` deleted the way a trash view eventually will. Phase 1 ships no
/// action that writes `deleted_at`, so the tests here and in `maintenance` set
/// it themselves rather than pretend an API exists.
#[cfg(test)]
pub fn mark_deleted(conn: &Connection, id: &str, at: i64) {
    let updated = conn
        .execute(
            "UPDATE images SET deleted_at = ?1 WHERE id = ?2",
            rusqlite::params![at, id],
        )
        .unwrap();
    assert_eq!(updated, 1, "no row to mark deleted: {id}");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingest::{IngestInput, store_image};
    use crate::library::Library;
    use crate::model::{ImageSource, TagCountFilter};

    fn png_bytes(width: u32, height: u32) -> Vec<u8> {
        encoded(width, height, image::ImageFormat::Png)
    }

    fn jpeg_bytes(width: u32, height: u32) -> Vec<u8> {
        encoded(width, height, image::ImageFormat::Jpeg)
    }

    fn encoded(width: u32, height: u32, format: image::ImageFormat) -> Vec<u8> {
        let image = image::DynamicImage::ImageRgb8(image::RgbImage::new(width, height));
        let mut out = std::io::Cursor::new(Vec::new());
        image.write_to(&mut out, format).unwrap();
        out.into_inner()
    }

    struct Fixture {
        _dir: tempfile::TempDir,
        library: Library,
    }

    /// Every row goes in through `ingest::store_image`, the one write path
    /// (design D4): a fixture built with hand-written INSERTs would pass while
    /// the real path stored something else.
    #[allow(clippy::too_many_arguments)]
    fn store(
        library: &Library,
        id: &str,
        bytes: &[u8],
        tags: &[&str],
        rating: Option<&str>,
        page_url: Option<&str>,
        page_title: &str,
        captured_at: i64,
    ) {
        let tags: Vec<String> = tags.iter().map(|tag| (*tag).to_string()).collect();
        let image_url = format!("https://pbs.test/media/{id}.png");
        store_image(
            library,
            IngestInput {
                id,
                bytes,
                source: ImageSource::Extension,
                source_ref: Some("x"),
                image_url: Some(&image_url),
                page_url,
                page_title: Some(page_title),
                rating,
                tags: &tags,
                captured_at,
            },
        )
        .unwrap();
    }

    /// `alice` has two cat images, `bob` one dog, `carol` one untagged and
    /// unrated, plus a non-X page and a deleted row. Captured-at is distinct
    /// everywhere so the expected order is unambiguous.
    fn fixture() -> Fixture {
        let dir = tempfile::tempdir().unwrap();
        let library = Library::open_or_create(dir.path()).unwrap();
        let png = png_bytes(4, 4);
        let jpeg = jpeg_bytes(4, 4);

        store(
            &library,
            "cat-s",
            &png,
            &["cat", "cute"],
            Some("s"),
            Some("https://x.com/alice/status/1"),
            "sunset over kyoto",
            600,
        );
        store(
            &library,
            "cat-dog-q",
            &png,
            &["cat", "dog", "cute"],
            Some("q"),
            Some("https://x.com/alice/status/2"),
            "a cat and a dog",
            500,
        );
        store(
            &library,
            "dog-e",
            &jpeg,
            &["dog"],
            Some("e"),
            Some("https://twitter.com/bob/status/3"),
            "dog park",
            400,
        );
        store(
            &library,
            "untagged",
            &png,
            &[],
            None,
            Some("https://x.com/i/status/4"),
            "promoted post",
            300,
        );
        store(
            &library,
            "no-account",
            &png,
            &["cat"],
            Some(""),
            Some("https://example.test/gallery/9"),
            "a gallery of cats",
            200,
        );
        store(
            &library,
            "trashed",
            &png,
            &["cat"],
            Some("s"),
            Some("https://x.com/alice/status/5"),
            "thrown away",
            100,
        );
        mark_deleted(&library.conn, "trashed", 1);

        Fixture { _dir: dir, library }
    }

    fn request(query: ParsedTagSearch) -> SearchRequest {
        SearchRequest {
            query,
            text: String::new(),
            include_deleted: false,
            limit: 100,
            offset: 0,
        }
    }

    fn text_request(text: &str) -> SearchRequest {
        SearchRequest {
            text: text.to_string(),
            ..request(ParsedTagSearch::default())
        }
    }

    fn found(fixture: &Fixture, req: &SearchRequest) -> Vec<String> {
        search(&fixture.library.conn, req)
            .unwrap()
            .images
            .into_iter()
            .map(|record| record.id)
            .collect()
    }

    fn tag_count_ids(fixture: &Fixture, tag_count: TagCountFilter) -> Vec<String> {
        found(
            fixture,
            &request(ParsedTagSearch {
                tag_count: Some(tag_count),
                ..Default::default()
            }),
        )
    }

    #[test]
    fn empty_query_lists_live_images_newest_capture_first() {
        let fixture = fixture();

        let result = search(&fixture.library.conn, &request(ParsedTagSearch::default())).unwrap();

        let ids: Vec<&str> = result.images.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(
            ids,
            vec!["cat-s", "cat-dog-q", "dog-e", "untagged", "no-account"]
        );
        assert_eq!(result.total, 5);
    }

    #[test]
    fn deleted_rows_are_excluded_until_asked_for() {
        let fixture = fixture();

        assert!(!found(&fixture, &request(ParsedTagSearch::default())).contains(&"trashed".into()));

        let with_deleted = SearchRequest {
            include_deleted: true,
            ..request(ParsedTagSearch::default())
        };
        let result = search(&fixture.library.conn, &with_deleted).unwrap();
        assert_eq!(result.total, 6);
        assert!(result.images.iter().any(|record| record.id == "trashed"));
    }

    #[test]
    fn search_returns_the_tags_of_each_row() {
        let fixture = fixture();

        let result = search(&fixture.library.conn, &request(ParsedTagSearch::default())).unwrap();

        let first = &result.images[0];
        assert_eq!(first.id, "cat-s");
        assert_eq!(first.tags, vec!["cat".to_string(), "cute".to_string()]);
    }

    #[test]
    fn and_with_an_exclusion() {
        let fixture = fixture();

        let ids = found(
            &fixture,
            &request(ParsedTagSearch {
                include_tags: vec!["cat".into()],
                exclude_tags: vec!["dog".into()],
                ..Default::default()
            }),
        );

        assert_eq!(ids, vec!["cat-s", "no-account"]);
    }

    #[test]
    fn two_include_tags_must_both_be_present() {
        let fixture = fixture();

        let ids = found(
            &fixture,
            &request(ParsedTagSearch {
                include_tags: vec!["cat".into(), "dog".into()],
                ..Default::default()
            }),
        );

        assert_eq!(ids, vec!["cat-dog-q"]);
    }

    #[test]
    fn an_or_group_matches_either_tag() {
        let fixture = fixture();

        let ids = found(
            &fixture,
            &request(ParsedTagSearch {
                or_groups: vec![vec!["cute".into(), "dog".into()]],
                ..Default::default()
            }),
        );

        assert_eq!(ids, vec!["cat-s", "cat-dog-q", "dog-e"]);
    }

    #[test]
    fn every_or_group_must_match() {
        let fixture = fixture();

        let ids = found(
            &fixture,
            &request(ParsedTagSearch {
                or_groups: vec![
                    vec!["cute".into(), "dog".into()],
                    vec!["dog".into(), "nothing".into()],
                ],
                ..Default::default()
            }),
        );

        assert_eq!(ids, vec!["cat-dog-q", "dog-e"]);
    }

    #[test]
    fn rating_lists_the_ratings_asked_for() {
        let fixture = fixture();

        let ids = found(
            &fixture,
            &request(ParsedTagSearch {
                ratings: vec!["s".into(), "q".into()],
                ..Default::default()
            }),
        );

        assert_eq!(ids, vec!["cat-s", "cat-dog-q"]);
    }

    #[test]
    fn unrated_is_ored_with_the_rating_list_not_anded() {
        let fixture = fixture();

        let ids = found(
            &fixture,
            &request(ParsedTagSearch {
                ratings: vec!["e".into()],
                include_unrated: true,
                ..Default::default()
            }),
        );

        // `no-account` carries `''`, which filters.ts reads as unrated.
        assert_eq!(ids, vec!["dog-e", "untagged", "no-account"]);
    }

    #[test]
    fn is_filters_by_mime() {
        let fixture = fixture();

        let ids = found(
            &fixture,
            &request(ParsedTagSearch {
                file_types: vec!["image/jpeg".into()],
                ..Default::default()
            }),
        );

        assert_eq!(ids, vec!["dog-e"]);
    }

    #[test]
    fn tag_count_equals() {
        let fixture = fixture();

        let ids = tag_count_ids(
            &fixture,
            TagCountFilter {
                operator: TagCountOperator::Eq,
                value: Some(1),
                values: None,
                min: None,
                max: None,
            },
        );

        assert_eq!(ids, vec!["dog-e", "no-account"]);
    }

    #[test]
    fn tag_count_greater_and_less_than() {
        let fixture = fixture();

        let greater = tag_count_ids(
            &fixture,
            TagCountFilter {
                operator: TagCountOperator::Gt,
                value: Some(2),
                values: None,
                min: None,
                max: None,
            },
        );
        assert_eq!(greater, vec!["cat-dog-q"]);

        let less = tag_count_ids(
            &fixture,
            TagCountFilter {
                operator: TagCountOperator::Lt,
                value: Some(1),
                values: None,
                min: None,
                max: None,
            },
        );
        assert_eq!(less, vec!["untagged"]);
    }

    #[test]
    fn tag_count_at_least_and_at_most() {
        let fixture = fixture();

        let at_least = tag_count_ids(
            &fixture,
            TagCountFilter {
                operator: TagCountOperator::Gte,
                value: Some(2),
                values: None,
                min: None,
                max: None,
            },
        );
        assert_eq!(at_least, vec!["cat-s", "cat-dog-q"]);

        let at_most = tag_count_ids(
            &fixture,
            TagCountFilter {
                operator: TagCountOperator::Lte,
                value: Some(1),
                values: None,
                min: None,
                max: None,
            },
        );
        assert_eq!(at_most, vec!["dog-e", "untagged", "no-account"]);
    }

    #[test]
    fn tag_count_range_includes_both_ends() {
        let fixture = fixture();

        let ids = tag_count_ids(
            &fixture,
            TagCountFilter {
                operator: TagCountOperator::Range,
                value: None,
                values: None,
                min: Some(1),
                max: Some(2),
            },
        );

        assert_eq!(ids, vec!["cat-s", "dog-e", "no-account"]);
    }

    #[test]
    fn tag_count_list_matches_any_listed_size() {
        let fixture = fixture();

        let ids = tag_count_ids(
            &fixture,
            TagCountFilter {
                operator: TagCountOperator::List,
                value: None,
                values: Some(vec![0, 3]),
                min: None,
                max: None,
            },
        );

        assert_eq!(ids, vec!["cat-dog-q", "untagged"]);
    }

    #[test]
    fn a_tag_count_without_its_operand_matches_nothing() {
        let fixture = fixture();

        let ids = tag_count_ids(
            &fixture,
            TagCountFilter {
                operator: TagCountOperator::Eq,
                value: None,
                values: None,
                min: None,
                max: None,
            },
        );

        assert!(ids.is_empty());
    }

    #[test]
    fn account_selects_one_x_account() {
        let fixture = fixture();

        let ids = found(
            &fixture,
            &request(ParsedTagSearch {
                accounts: vec!["alice".into()],
                ..Default::default()
            }),
        );

        assert_eq!(ids, vec!["cat-s", "cat-dog-q"]);
    }

    #[test]
    fn excluding_an_account_keeps_pages_that_have_none() {
        let fixture = fixture();

        let ids = found(
            &fixture,
            &request(ParsedTagSearch {
                exclude_accounts: vec!["alice".into()],
                ..Default::default()
            }),
        );

        assert_eq!(ids, vec!["dog-e", "untagged", "no-account"]);
    }

    #[test]
    fn free_text_matches_a_page_title() {
        let fixture = fixture();

        assert_eq!(found(&fixture, &text_request("kyoto")), vec!["cat-s"]);
    }

    #[test]
    fn free_text_matches_a_url() {
        let fixture = fixture();

        assert_eq!(
            found(&fixture, &text_request("example.test/gallery")),
            vec!["no-account"]
        );
    }

    #[test]
    fn free_text_is_data_not_fts_syntax() {
        let fixture = fixture();

        // Unquoted, each of these is an FTS5 operator or a syntax error.
        for text in ["-kyoto", "kyoto:", "kyo*", "\"kyoto", "NOT kyoto", "(kyoto"] {
            let result = search(&fixture.library.conn, &text_request(text));
            assert!(
                result.is_ok(),
                "{text:?} reached FTS5 as syntax: {result:?}"
            );
        }
        assert_eq!(found(&fixture, &text_request("\"kyoto\"")), vec!["cat-s"]);
    }

    #[test]
    fn text_with_no_searchable_word_is_not_an_error() {
        let fixture = fixture();

        let result = search(&fixture.library.conn, &text_request("--")).unwrap();

        assert!(result.images.is_empty());
        assert_eq!(result.total, 0);
    }

    #[test]
    fn blank_text_is_no_clause_at_all() {
        let fixture = fixture();

        assert_eq!(found(&fixture, &text_request("   ")).len(), 5);
    }

    #[test]
    fn text_and_tags_narrow_together() {
        let fixture = fixture();

        assert_eq!(
            found(&fixture, &text_request("dog")),
            vec!["cat-dog-q", "dog-e"]
        );

        let both = SearchRequest {
            text: "dog".to_string(),
            ..request(ParsedTagSearch {
                include_tags: vec!["cat".into()],
                ..Default::default()
            })
        };
        assert_eq!(found(&fixture, &both), vec!["cat-dog-q"]);
    }

    #[test]
    fn paging_counts_every_match_and_returns_one_page() {
        let fixture = fixture();

        let page = SearchRequest {
            limit: 2,
            offset: 2,
            ..request(ParsedTagSearch::default())
        };
        let result = search(&fixture.library.conn, &page).unwrap();

        assert_eq!(result.total, 5, "total is the match count before paging");
        let ids: Vec<&str> = result.images.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, vec!["dog-e", "untagged"]);
    }

    #[test]
    fn paging_walks_every_row_exactly_once() {
        let fixture = fixture();

        let mut seen: Vec<String> = Vec::new();
        for offset in (0..6).step_by(2) {
            let page = SearchRequest {
                limit: 2,
                offset,
                ..request(ParsedTagSearch::default())
            };
            seen.extend(found(&fixture, &page));
        }

        assert_eq!(
            seen,
            vec!["cat-s", "cat-dog-q", "dog-e", "untagged", "no-account"]
        );
    }

    #[test]
    fn a_tag_name_that_looks_like_sql_is_only_ever_a_value() {
        let fixture = fixture();

        let ids = found(
            &fixture,
            &request(ParsedTagSearch {
                include_tags: vec!["'); DROP TABLE images; --".into()],
                ..Default::default()
            }),
        );

        assert!(ids.is_empty());
        assert_eq!(fixture.library.image_count().unwrap(), 5);
    }

    #[test]
    fn x_account_reads_the_first_path_segment() {
        assert_eq!(x_account("https://x.com/alice/status/1"), Some("alice"));
        assert_eq!(x_account("https://twitter.com/Bob"), Some("Bob"));
        assert_eq!(x_account("https://www.x.com/carol/"), Some("carol"));
        assert_eq!(
            x_account("https://www.twitter.com/dave?src=x"),
            Some("dave")
        );
        assert_eq!(x_account("http://X.com:443/erin/status/2"), Some("erin"));
    }

    #[test]
    fn x_account_ignores_other_hosts_and_x_own_pages() {
        for url in [
            "https://example.test/alice",
            "https://notx.com/alice",
            "https://x.com",
            "https://x.com/",
            "https://x.com/?q=1",
            "https://x.com/i/status/1",
            "https://x.com/home",
            "https://x.com/EXPLORE",
            "https://x.com/notifications",
            "https://x.com/messages",
            "https://x.com/search?q=cat",
            "x.com/alice",
            "",
        ] {
            assert_eq!(x_account(url), None, "expected no account in {url:?}");
        }
    }
}
