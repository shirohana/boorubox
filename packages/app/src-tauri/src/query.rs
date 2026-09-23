//! Compiles a `ParsedTagSearch` into SQL — AND tags, OR groups, exclusions,
//! rating, `is:`, `tagcount:`, `account:` and the FTS5 free-text match — and
//! owns the `x_account()` scalar those account filters need (design D3).
//!
//! It also compiles the sort and the grouping (design D6, D7), so nothing in
//! the webview decides membership of a result, its order or its groups.
//!
//! The rules were lifted from the legacy `filters.ts` and `grouping.ts`, whose
//! cases live in this file's tests now that those modules are gone. The comments
//! below name their functions where a clause reproduces a JavaScript answer
//! (`[].some()` is false, `!img.rating` is falsiness) rather than an obvious
//! SQL one.

use rusqlite::functions::FunctionFlags;
use rusqlite::types::Value;
use rusqlite::{Connection, OptionalExtension, params_from_iter};

use crate::error::Result;
use crate::ingest;
use crate::model::{
    CollectionCount, GroupBy, GroupSlice, ParsedTagSearch, RatingCounts, SearchRequest,
    SearchResult, SearchView, Sort, SortDirection, SortField, TagCount, TagCountOperator,
    TagCounts,
};
use crate::tags;

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

/// How many ids a single `IN (…)` statement should bind at once. SQLite's
/// bound-parameter limit is comfortably above this in the bundled build, but a
/// selection spanning a whole large library still becomes one placeholder per
/// id in one statement — a caller that builds an `IN` list over the whole
/// selection (`bulk_set_rating`, `selection_tag_counts`, `export::export_zip`,
/// `matching_ids`) chunks it to this size rather than trust the limit never to
/// be reached.
pub const ID_CHUNK: usize = 900;

pub fn search(conn: &Connection, req: &SearchRequest) -> Result<SearchResult> {
    let plan = Plan::for_request(req, RatingClause::Included);
    let ids = search_ids(conn, req)?;

    Ok(SearchResult {
        total: plan.total(conn)?,
        // `load_records` keeps the order it is given and fills the tags in one
        // extra statement, so the page costs three queries however large it is.
        images: ingest::load_records(conn, &ids)?,
        groups: plan.groups(conn)?,
    })
}

/// Just the ids of a request's page, in the same order `search` pages them,
/// with no records, no tags and no missing-file pass (`selection-and-bulk`
/// design D3). `search` calls this too, so the selection's row *n* and the
/// grid's row *n* are only ever read from the one `ORDER BY` this compiles.
pub fn search_ids(conn: &Connection, req: &SearchRequest) -> Result<Vec<String>> {
    Plan::for_request(req, RatingClause::Included).page(conn, req.limit, req.offset)
}

/// The zero-based row `id` occupies in `req`'s order — the same order
/// `search_ids` pages — or `None` when `id` is not in the matched set at all
/// (`inspector-polish` design D2). Built on the same `Plan` as `search_ids`,
/// with `ROW_NUMBER() OVER (<the plan's order>)` filtered to the one id
/// asked for: one query, no walk of the pages between here and there.
pub fn search_position(conn: &Connection, req: &SearchRequest, id: &str) -> Result<Option<i64>> {
    let plan = Plan::for_request(req, RatingClause::Included);
    let mut stmt = conn.prepare(&format!(
        "{} SELECT row_number FROM (
             SELECT id, ROW_NUMBER() OVER (ORDER BY {}) - 1 AS row_number FROM {}
         ) WHERE id = ?",
        plan.ctes,
        plan.order_by(),
        plan.rows
    ))?;
    let mut params = plan.params.clone();
    params.push(Value::Text(id.to_string()));
    Ok(stmt
        .query_row(params_from_iter(&params), |row| row.get(0))
        .optional()?)
}

/// Which of `ids` a `search` of `req` still matches, on the same `Plan` as
/// `search_ids` and `search_position` — what the selection prunes itself by
/// after a write re-reads the search (`browse-fixes` design D1), so the
/// count, the thumbnail strip and the next bulk action describe only images
/// the result still shows. `limit`/`offset` are ignored: the question is
/// membership in the whole result, not one page of it. Chunked to
/// [`ID_CHUNK`] ids per `IN (…)`, for the same reason `bulk_set_rating`
/// chunks — a selection can itself span a whole large library.
pub fn matching_ids(conn: &Connection, req: &SearchRequest, ids: &[String]) -> Result<Vec<String>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let plan = Plan::for_request(req, RatingClause::Included);
    let mut matched = Vec::new();
    for chunk in ids.chunks(ID_CHUNK) {
        let mut stmt = conn.prepare(&format!(
            "{} SELECT id FROM {} WHERE id IN ({})",
            plan.ctes,
            plan.rows,
            placeholders(chunk.len())
        ))?;
        let mut params = plan.params.clone();
        params.extend(text_values(chunk));
        let rows = stmt.query_map(params_from_iter(&params), |row| row.get(0))?;
        matched.extend(rows.collect::<rusqlite::Result<Vec<String>>>()?);
    }
    Ok(matched)
}

/// The sidebar's two halves for one request (design D8). Both ignore `limit`
/// and `offset` — the panel describes the whole result set, never the pages that
/// happen to be loaded — and both honour the group restriction.
///
/// One command rather than two: the halves are one panel refreshed as one unit,
/// and two commands would let the pills and the tag list describe different
/// queries while one was in flight.
pub fn tag_counts(conn: &Connection, req: &SearchRequest) -> Result<TagCounts> {
    Ok(TagCounts {
        // Counted over the result *as filtered*, rating clause included: a tag
        // count answers "how many of what I am looking at also carry this",
        // which is the narrowing move.
        tags: Plan::for_request(req, RatingClause::Included).tag_counts(conn)?,
        // Counted with the rating clause dropped: a pill answers "how many would
        // I get if I switched to this", which is a sideways move. Counting these
        // after the rating filter would show the selected rating's own count and
        // four zeros. Making the two halves uniform breaks one of the two
        // questions, whichever direction is chosen — the asymmetry is the point.
        ratings: Plan::for_request(req, RatingClause::Dropped).rating_counts(conn)?,
        // Rating clause included, like `tags` above (design D7): "how many of
        // what I am looking at is in this collection" is the same narrowing
        // question, not the pill's sideways one.
        collections: Plan::for_request(req, RatingClause::Included).collection_counts(conn)?,
        // The account rail asks the pills' question, not the tag list's: how
        // many would `bob` show if the search switched to him, `account:alice`
        // notwithstanding. `for_account_counts` also skips the whole pass when
        // the request is not grouped by account, so an ungrouped search never
        // pays for it.
        accounts: Plan::for_account_counts(req).account_counts(conn)?,
    })
}

/// One request compiled: the CTEs that name the matched set and its groups, what
/// the row queries read from, and the order they read it in.
///
/// The filter text — and so its bound values — appears exactly once, inside the
/// `matched` CTE, however many times a grouped query reads the set back. Two
/// copies of it would be two positional parameter lists to keep in step, which
/// is the hazard `Filter` exists to remove.
struct Plan {
    ctes: String,
    /// `FROM` for the row queries: the matched set, or its grouped join.
    rows: &'static str,
    /// Group key and size ordering, ahead of the sort (design D7). `None` when
    /// nothing is grouped, which is also what makes `groups` empty.
    group_order: Option<&'static str>,
    /// The view's own predicate (`view_predicate`), kept alongside `ctes`
    /// rather than re-derived: `account_counts` needs it outside `matched`, to
    /// restrict the every-account scan without re-running the rest of the
    /// filter.
    view: &'static str,
    sort: String,
    params: Vec<Value>,
}

/// The columns the sort, the group key, the rating counts and the page load
/// read out of the matched set.
const MATCHED_COLUMNS: &str =
    "id, page_url, rating, width, height, size, captured_at, updated_at, deleted_at";

/// The account whose page the image came from. A page naming none is not in the
/// group at all, so grouping by account also restricts the result (design D7).
const GROUPED_BY_ACCOUNT: &str = "grouped AS (
       SELECT matched.*, x_account(page_url) AS group_key
       FROM matched
       WHERE x_account(page_url) IS NOT NULL
     )";

/// "Duplicates" keeps the legacy meaning — same pixel dimensions, same byte size
/// — because a content hash is a new column, a migration and a backfill over
/// every image, which is its own change with its own argument.
const GROUPED_BY_DUPLICATE_KEY: &str = "grouped AS (
       SELECT matched.*, width || 'x' || height || '-' || size AS group_key
       FROM matched
     )";

const EVERY_SLICE: &str = "slices AS (
       SELECT group_key, COUNT(*) AS group_size FROM grouped GROUP BY group_key
     )";

/// A duplicate needs a twin: a triple only one image carries is no group, and
/// that image is not in the result either.
const SLICES_OF_TWO_OR_MORE: &str = "slices AS (
       SELECT group_key, COUNT(*) AS group_size
       FROM grouped GROUP BY group_key HAVING COUNT(*) > 1
     )";

/// The join is the group restriction: a row whose key made no slice is gone.
const GROUPED_ROWS: &str = "grouped JOIN slices USING (group_key)";

/// The legacy viewer's group order: accounts by size descending then name,
/// duplicate keys by key.
const ACCOUNTS_LARGEST_FIRST: &str = "group_size DESC, group_key";
const DUPLICATE_KEYS_BY_KEY: &str = "group_key";

impl Plan {
    fn for_request(req: &SearchRequest, rating: RatingClause) -> Plan {
        Plan::compiled(req, compile(req, rating, AccountClause::Included))
    }

    /// A plan for the account rail (design accounts-in-tag-counts): the account
    /// clause dropped so `bob` still shows a count while `account:alice` is in
    /// effect, everything else — rating included — honoured, same as
    /// `for_request`'s default.
    fn for_account_counts(req: &SearchRequest) -> Plan {
        Plan::compiled(
            req,
            compile(req, RatingClause::Included, AccountClause::Dropped),
        )
    }

    /// The CTEs, row source and group order a compiled `filter` produces —
    /// shared by every `Plan` constructor so the grouping match arms exist
    /// exactly once, whatever the filter was compiled with.
    fn compiled(req: &SearchRequest, filter: Filter) -> Plan {
        let matched = format!(
            "WITH matched AS (SELECT {MATCHED_COLUMNS} FROM images WHERE {})",
            filter.sql()
        );
        let (ctes, rows, group_order) = match req.group {
            GroupBy::None => (matched, "matched", None),
            GroupBy::XAccount => (
                format!("{matched},\n     {GROUPED_BY_ACCOUNT},\n     {EVERY_SLICE}"),
                GROUPED_ROWS,
                Some(ACCOUNTS_LARGEST_FIRST),
            ),
            GroupBy::Duplicates => (
                format!(
                    "{matched},\n     {GROUPED_BY_DUPLICATE_KEY},\n     {SLICES_OF_TWO_OR_MORE}"
                ),
                GROUPED_ROWS,
                Some(DUPLICATE_KEYS_BY_KEY),
            ),
        };
        Plan {
            ctes,
            rows,
            group_order,
            view: view_predicate(req.view),
            sort: sort_sql(req.sort),
            params: filter.params,
        }
    }

    /// Matches before `limit`/`offset`, the group restriction included — what is
    /// counted is what is shown (spec `library-browse`).
    fn total(&self, conn: &Connection) -> Result<i64> {
        let total = conn.query_row(
            &format!("{} SELECT COUNT(*) FROM {}", self.ctes, self.rows),
            params_from_iter(&self.params),
            |row| row.get(0),
        )?;
        Ok(total)
    }

    fn page(&self, conn: &Connection, limit: i64, offset: i64) -> Result<Vec<String>> {
        let mut stmt = conn.prepare(&format!(
            "{} SELECT id FROM {} ORDER BY {} LIMIT ? OFFSET ?",
            self.ctes,
            self.rows,
            self.order_by()
        ))?;
        let mut params = self.params.clone();
        params.push(Value::Integer(limit));
        params.push(Value::Integer(offset));
        let ids = stmt.query_map(params_from_iter(&params), |row| row.get(0))?;
        Ok(ids.collect::<rusqlite::Result<_>>()?)
    }

    fn order_by(&self) -> String {
        match self.group_order {
            Some(group) => format!("{group}, {}", self.sort),
            None => self.sort.clone(),
        }
    }

    /// Every group of the whole result, which is what makes the grid's layout
    /// computable before a page is loaded: headings, counts and the row a group
    /// starts on are arithmetic over these (design D7).
    fn groups(&self, conn: &Connection) -> Result<Vec<GroupSlice>> {
        let Some(order) = self.group_order else {
            return Ok(Vec::new());
        };
        let mut stmt = conn.prepare(&format!(
            "{} SELECT group_key, group_size FROM slices ORDER BY {order}",
            self.ctes
        ))?;
        let slices = stmt.query_map(params_from_iter(&self.params), |row| {
            Ok(GroupSlice {
                key: row.get(0)?,
                count: row.get(1)?,
            })
        })?;
        Ok(slices.collect::<rusqlite::Result<_>>()?)
    }

    /// Every account of the request's view, zero-matches included, counted
    /// with the account clause dropped (design accounts-in-tag-counts) —
    /// `for_account_counts` builds `self` for exactly this. Empty whenever the
    /// request is not grouped by account: `slices` is only ever a group of
    /// accounts when `group_order` is `ACCOUNTS_LARGEST_FIRST`, and this is
    /// also the rail's own visibility rule, so nothing is skipped that the
    /// rail would have shown.
    fn account_counts(&self, conn: &Connection) -> Result<Vec<GroupSlice>> {
        if self.group_order != Some(ACCOUNTS_LARGEST_FIRST) {
            return Ok(Vec::new());
        }
        let mut stmt = conn.prepare(&format!(
            "{} SELECT a.handle, COALESCE(s.group_size, 0)
             FROM (SELECT DISTINCT x_account(page_url) AS handle FROM images
                   WHERE {} AND x_account(page_url) IS NOT NULL) AS a
             LEFT JOIN slices AS s ON s.group_key = a.handle
             ORDER BY 2 DESC, 1",
            self.ctes, self.view
        ))?;
        let slices = stmt.query_map(params_from_iter(&self.params), |row| {
            Ok(GroupSlice {
                key: row.get(0)?,
                count: row.get(1)?,
            })
        })?;
        Ok(slices.collect::<rusqlite::Result<_>>()?)
    }

    fn tag_counts(&self, conn: &Connection) -> Result<Vec<TagCount>> {
        let mut stmt = conn.prepare(&format!(
            "{} SELECT tags.name, COUNT(*) AS carriers
             FROM image_tags
             JOIN tags ON tags.id = image_tags.tag_id
             WHERE image_tags.image_id IN (SELECT id FROM {})
             GROUP BY image_tags.tag_id
             ORDER BY carriers DESC, tags.name",
            self.ctes, self.rows
        ))?;
        let counts = stmt.query_map(params_from_iter(&self.params), |row| {
            Ok(TagCount {
                name: row.get(0)?,
                count: row.get(1)?,
            })
        })?;
        Ok(counts.collect::<rusqlite::Result<_>>()?)
    }

    /// Every collection in the library, `LEFT JOIN`ed against the matched set
    /// so an empty collection is listed at zero rather than dropped (design
    /// D7) — the sidebar's own rule for a tag with no matches applies here
    /// too. Ordered by name, the same order [`collections::list`] uses for
    /// the menus, so the sidebar and a create/rename dialog never disagree
    /// about what order "by name" means.
    ///
    /// [`collections::list`]: crate::collections::list
    fn collection_counts(&self, conn: &Connection) -> Result<Vec<CollectionCount>> {
        let mut stmt = conn.prepare(&format!(
            "{} SELECT collections.id, collections.name, collections.slug,
                    COUNT(matched_members.image_id) AS count
             FROM collections
             LEFT JOIN (
                 SELECT image_collections.collection_id, image_collections.image_id
                 FROM image_collections
                 WHERE image_collections.image_id IN (SELECT id FROM {})
             ) AS matched_members ON matched_members.collection_id = collections.id
             GROUP BY collections.id
             ORDER BY collections.name COLLATE NOCASE",
            self.ctes, self.rows
        ))?;
        let counts = stmt.query_map(params_from_iter(&self.params), |row| {
            Ok(CollectionCount {
                id: row.get(0)?,
                name: row.get(1)?,
                slug: row.get(2)?,
                count: row.get(3)?,
            })
        })?;
        Ok(counts.collect::<rusqlite::Result<_>>()?)
    }

    fn rating_counts(&self, conn: &Connection) -> Result<RatingCounts> {
        let mut stmt = conn.prepare(&format!(
            "{} SELECT rating, COUNT(*) FROM {} GROUP BY rating",
            self.ctes, self.rows
        ))?;
        let mut rows = stmt.query(params_from_iter(&self.params))?;

        let mut counts = RatingCounts::default();
        while let Some(row) = rows.next()? {
            let rating: Option<String> = row.get(0)?;
            let count: i64 = row.get(1)?;
            match rating.as_deref() {
                // `!img.rating` is JS falsiness, so an empty rating string is
                // unrated here too, exactly as the filter reads it.
                None | Some("") => counts.unrated += count,
                Some("g") => counts.g += count,
                Some("s") => counts.s += count,
                Some("q") => counts.q += count,
                Some("e") => counts.e += count,
                // A rating a legacy-bundle import carried and no pill offers
                // (design D11). It belongs to no control, so it is counted
                // nowhere — which is what the lifted `computeRatingCounts` did
                // with the same value.
                Some(_) => {}
            }
        }
        Ok(counts)
    }
}

/// The sort as an `ORDER BY` tail. The field is an enum rather than a string
/// split at a hyphen: it arrives from IPC, and nothing but a placeholder is ever
/// formatted into SQL here — an enum cannot spell a column that does not exist
/// (design D6).
///
/// `id` breaks every tie, and that is not decoration: without it SQLite may
/// order two rows with equal sort values differently between the page-1 and
/// page-2 queries, and a row then repeats on one page and is skipped on the
/// other. Every column here has ties by nature, `size` and `dimensions` most.
fn sort_sql(sort: Sort) -> String {
    let column = match sort.field {
        SortField::Captured => "captured_at",
        SortField::Updated => "updated_at",
        SortField::Size => "size",
        // The area, which is what the lifted `sortImages` compared.
        SortField::Dimensions => "width * height",
        SortField::Trashed => "deleted_at",
    };
    let direction = match sort.direction {
        SortDirection::Asc => "ASC",
        SortDirection::Desc => "DESC",
    };
    format!("{column} {direction}, id DESC")
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
/// and the lifted `filterByTagSearch` reached the same answer for the two cases
/// that produce it: an empty OR group (`[].some()` is false) and a `tagcount:`
/// whose operand is missing (every comparison against `undefined` is false).
const MATCHES_NOTHING: &str = "1 = 0";

/// The image's tag count, as a correlated subquery — `image_tags` is the list
/// the lifted `filterByTagSearch` took `img.tags.length` of.
const TAG_COUNT: &str = "(SELECT COUNT(*) FROM image_tags WHERE image_tags.image_id = images.id)";

/// Whether the rating clause is part of the compiled filter. The rating half of
/// `tag_counts` is the one caller that drops it (design D8).
#[derive(Clone, Copy, PartialEq, Eq)]
enum RatingClause {
    Included,
    Dropped,
}

/// Whether the `account:`/`-account:` clause is part of the compiled filter.
/// The account rail (`account-rail-counts` design D1) is the one caller that
/// drops it: `bob`'s count in the rail has to answer "how many would I get if
/// I switched to him", the same sideways question `RatingClause::Dropped`
/// answers for a rating pill, so `account:alice` in the search cannot also
/// zero out every other account's row.
#[derive(Clone, Copy, PartialEq, Eq)]
enum AccountClause {
    Included,
    Dropped,
}

/// `images.deleted_at IS NULL` for the library, `IS NOT NULL` for the trash —
/// the one predicate `req.view` ever compiles to. Factored out so
/// `Plan::account_counts`'s every-account scan reads the same string
/// `compile` puts in `matched`, rather than a second copy that could drift
/// from it.
fn view_predicate(view: SearchView) -> &'static str {
    match view {
        SearchView::Library => "images.deleted_at IS NULL",
        SearchView::Trash => "images.deleted_at IS NOT NULL",
    }
}

fn compile(req: &SearchRequest, rating: RatingClause, accounts: AccountClause) -> Filter {
    let query = &req.query;
    let mut filter = Filter::default();

    filter.add(view_predicate(req.view));
    if rating == RatingClause::Included {
        push_rating(&mut filter, query);
    }
    push_file_types(&mut filter, query);
    push_tag_count(&mut filter, query);
    if accounts == AccountClause::Included {
        push_accounts(&mut filter, query);
    }
    push_collections(&mut filter, query);
    push_tags(&mut filter, query);
    push_text(&mut filter, &req.text);
    filter
}

/// "Unrated" and the rating list are ONE predicate: an image passes if
/// `includeUnrated` and it has no rating, OR its rating is listed. Two AND-ed
/// clauses would make `rating:s` plus unrated match nothing. `!img.rating` was
/// JS falsiness, so an empty rating string is unrated here too.
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

/// `collection:<slug>` / `-collection:<slug>` (design D6), compiled against
/// `collections.slug` — never the id, which the webview never sees. A slug
/// nothing has matches nothing by the `IN` alone, on both sides: no `NULL`
/// case to special-case the way `push_accounts` needs one for `x_account`,
/// since a membership join produces no row rather than a `NULL` one.
fn push_collections(filter: &mut Filter, query: &ParsedTagSearch) {
    if !query.collections.is_empty() {
        filter.add_bound(
            has_any_collection(query.collections.len()),
            text_values(&query.collections),
        );
    }
    if !query.exclude_collections.is_empty() {
        filter.add_bound(
            format!(
                "NOT {}",
                has_any_collection(query.exclude_collections.len())
            ),
            text_values(&query.exclude_collections),
        );
    }
}

/// "this image is in at least one collection whose slug is among these" — the
/// shape both halves of [`push_collections`] share, the same way
/// [`has_any_tag`] backs both the include and the exclude side of a tag
/// clause.
fn has_any_collection(count: usize) -> String {
    format!(
        "EXISTS (SELECT 1 FROM image_collections
                 JOIN collections ON collections.id = image_collections.collection_id
                 WHERE image_collections.image_id = images.id
                   AND collections.slug IN ({}))",
        placeholders(count)
    )
}

fn push_tags(filter: &mut Filter, query: &ParsedTagSearch) {
    // AND: one `EXISTS` per tag, so a repeated tag costs a repeated test rather
    // than throwing the count of distinct names off.
    for tag in &query.include_tags {
        filter.add_bound(has_any_tag(1), [Value::Text(tags::canonical(tag))]);
    }
    for group in &query.or_groups {
        if group.is_empty() {
            filter.add(MATCHES_NOTHING);
            continue;
        }
        filter.add_bound(has_any_tag(group.len()), canonical_tag_values(group));
    }
    if !query.exclude_tags.is_empty() {
        filter.add_bound(
            format!("NOT {}", has_any_tag(query.exclude_tags.len())),
            canonical_tag_values(&query.exclude_tags),
        );
    }
}

/// The search side of `tags::canonical` (`lowercase-tags` design D1): the
/// webview already lowercases its own copy of a parsed query (design D3), but
/// this canonicalises again rather than trust it, the same "two runtimes, one
/// rule each" split D1 states for the metatag alphabet.
fn canonical_tag_values(values: &[String]) -> Vec<Value> {
    values
        .iter()
        .map(|tag| Value::Text(tags::canonical(tag)))
        .collect()
}

/// "this image carries at least one of these tag names" — the shape every tag
/// clause is built from. Both sides are canonical (`tags::canonical`,
/// `lowercase-tags` design D1) by the time this runs, so BINARY collation's
/// exact match is the right one — no `COLLATE NOCASE`, and none is needed.
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

/// The account an X page names, in its one implementation: this backs both the
/// `account:` filter and the X-account grouping, which are the same question
/// asked twice. The webview had a second copy of this rule (`getXAccountFromUrl`
/// in `grouping.ts`) for as long as it grouped its own results; its URL table is
/// the `x_account` tests below.
pub(crate) fn x_account(url: &str) -> Option<&str> {
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
                adapter: None,
                rating,
                tags: &tags,
                captured_at,
                file_modified_at: None,
                deleted_at: None,
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
            view: SearchView::Library,
            sort: Sort::default(),
            group: GroupBy::default(),
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

    /// `selection-and-bulk` design D3: the selection resolves a range through
    /// `search_ids`, and it only names the same images the grid pages if both
    /// go through the one `ORDER BY` — this pins that `search_ids` is not a
    /// second query that could drift from it.
    #[test]
    fn search_ids_names_the_same_page_search_would_load() {
        let fixture = fixture();
        let req = SearchRequest {
            limit: 2,
            offset: 1,
            ..request(ParsedTagSearch::default())
        };

        let paged = search(&fixture.library.conn, &req).unwrap();
        let ids = search_ids(&fixture.library.conn, &req).unwrap();

        assert_eq!(
            ids,
            paged
                .images
                .iter()
                .map(|record| record.id.clone())
                .collect::<Vec<_>>(),
        );
        assert_eq!(ids.len(), 2, "the limit must still apply");
    }

    /// `browse-fixes` design D1: what the selection prunes itself by after a
    /// write re-reads the search — the subset of a candidate set of ids that
    /// the request's `Plan` still matches.
    #[test]
    fn matching_ids_answers_the_subset_the_request_still_matches() {
        let fixture = fixture();
        let req = SearchRequest {
            limit: 1,
            offset: 1,
            ..request(ParsedTagSearch {
                include_tags: vec!["cat".into()],
                ..Default::default()
            })
        };
        let candidates = vec![
            "cat-s".to_string(),
            "cat-dog-q".to_string(),
            "dog-e".to_string(),
            "untagged".to_string(),
        ];

        let mut matched = matching_ids(&fixture.library.conn, &req, &candidates).unwrap();
        matched.sort();

        assert_eq!(matched, vec!["cat-dog-q".to_string(), "cat-s".to_string()]);
    }

    /// `inspector-polish` design D2: `search_position` answers the same row
    /// `search_ids` pages an id at, without walking pages to find it.
    #[test]
    fn search_position_matches_the_row_search_ids_returns_it_at() {
        let fixture = fixture();
        let req = request(ParsedTagSearch::default());

        let ids = search_ids(&fixture.library.conn, &req).unwrap();
        assert_eq!(
            ids[2], "dog-e",
            "the third-newest id, fixed by capture time"
        );

        let position = search_position(&fixture.library.conn, &req, "dog-e").unwrap();

        assert_eq!(position, Some(2));
        assert_eq!(
            ids.iter().position(|id| id == "dog-e"),
            Some(2),
            "search_position and search_ids must agree on the row"
        );
    }

    #[test]
    fn search_position_is_none_for_an_id_the_requests_tags_exclude() {
        let fixture = fixture();
        let req = request(ParsedTagSearch {
            include_tags: vec!["dog".into()],
            ..Default::default()
        });

        let position = search_position(&fixture.library.conn, &req, "cat-s").unwrap();

        assert_eq!(position, None, "cat-s carries no `dog` tag");
    }

    #[test]
    fn search_position_agrees_with_search_ids_when_a_group_is_set() {
        let fixture = fixture();
        let req = SearchRequest {
            group: GroupBy::XAccount,
            ..request(ParsedTagSearch::default())
        };

        let ids = search_ids(&fixture.library.conn, &req).unwrap();

        for (row, id) in ids.iter().enumerate() {
            let position = search_position(&fixture.library.conn, &req, id).unwrap();
            assert_eq!(position, Some(row as i64), "row for {id}");
        }
    }

    #[test]
    fn the_library_view_excludes_a_row_marked_deleted() {
        let fixture = fixture();

        assert!(!found(&fixture, &request(ParsedTagSearch::default())).contains(&"trashed".into()));
    }

    /// `trash` design D2: the trash view returns only marked rows, and never an
    /// undeleted one.
    #[test]
    fn the_trash_view_returns_only_the_marked_row() {
        let fixture = fixture();

        let trash = SearchRequest {
            view: SearchView::Trash,
            ..request(ParsedTagSearch::default())
        };
        let result = search(&fixture.library.conn, &trash).unwrap();

        assert_eq!(result.total, 1);
        assert_eq!(result.images[0].id, "trashed");
    }

    #[test]
    fn search_returns_the_tags_of_each_row() {
        let fixture = fixture();

        let result = search(&fixture.library.conn, &request(ParsedTagSearch::default())).unwrap();

        let first = &result.images[0];
        assert_eq!(first.id, "cat-s");
        assert_eq!(first.tags, vec!["cat".to_string(), "cute".to_string()]);
    }

    /// `lowercase-tags` design D1, spec `library-browse` "Case": a search
    /// term typed in capitals matches the canonical (lowercase) stored tag,
    /// whether it is an include, an exclude or inside an or-group.
    #[test]
    fn a_capitalised_search_term_matches_the_canonical_tag() {
        let fixture = fixture();

        assert_eq!(
            found(
                &fixture,
                &request(ParsedTagSearch {
                    include_tags: vec!["Cat".into()],
                    exclude_tags: vec!["Dog".into()],
                    ..Default::default()
                }),
            ),
            vec!["cat-s", "no-account"],
            "the same result as `cat -dog`",
        );
        assert_eq!(
            found(
                &fixture,
                &request(ParsedTagSearch {
                    or_groups: vec![vec!["Cute".into(), "Dog".into()]],
                    ..Default::default()
                }),
            ),
            vec!["cat-s", "cat-dog-q", "dog-e"],
            "the same result as `cute or dog`",
        );
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

        // `no-account` carries `''`, which the rule reads as unrated.
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
    fn collection_selects_its_members() {
        let fixture = fixture();
        crate::collections::add(
            &fixture.library,
            &["cat-s".to_string(), "dog-e".to_string()],
            "favorites",
        )
        .unwrap();

        let ids = found(
            &fixture,
            &request(ParsedTagSearch {
                collections: vec!["favorites".to_string()],
                ..Default::default()
            }),
        );

        assert_eq!(ids, vec!["cat-s", "dog-e"]);
    }

    #[test]
    fn excluding_a_collection_keeps_every_other_image() {
        let fixture = fixture();
        crate::collections::add(&fixture.library, &["cat-s".to_string()], "favorites").unwrap();

        let ids = found(
            &fixture,
            &request(ParsedTagSearch {
                exclude_collections: vec!["favorites".to_string()],
                ..Default::default()
            }),
        );

        assert_eq!(
            ids,
            vec!["cat-dog-q", "dog-e", "untagged", "no-account"],
            "the trashed row is out of the library view either way"
        );
    }

    #[test]
    fn an_unknown_collection_slug_matches_nothing() {
        let fixture = fixture();

        let ids = found(
            &fixture,
            &request(ParsedTagSearch {
                collections: vec!["no-such-slug".to_string()],
                ..Default::default()
            }),
        );

        assert!(ids.is_empty());
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

    /// Three images whose capture time, byte size and pixel area each put them in
    /// a different order, so a sort reading the wrong column cannot pass by
    /// coincidence. Sizes are what the encoder produces: a blank PNG is tiny
    /// however large it is, while a JPEG carries its tables.
    ///
    /// | id      | captured | size | area |
    /// | ------- | -------- | ---- | ---- |
    /// | `flat`  | 200      | 139  | 64   |
    /// | `later` | 300      | 217  | 1024 |
    /// | `heavy` | 100      | 634  | 256  |
    fn sortable() -> Fixture {
        let dir = tempfile::tempdir().unwrap();
        let library = Library::open_or_create(dir.path()).unwrap();
        let page = Some("https://x.com/alice/status/1");

        store(
            &library,
            "flat",
            &png_bytes(8, 8),
            &[],
            None,
            page,
            "flat",
            200,
        );
        store(
            &library,
            "later",
            &png_bytes(32, 32),
            &[],
            None,
            page,
            "later",
            300,
        );
        store(
            &library,
            "heavy",
            &jpeg_bytes(16, 16),
            &[],
            None,
            page,
            "heavy",
            100,
        );

        Fixture { _dir: dir, library }
    }

    fn sorted(fixture: &Fixture, field: SortField, direction: SortDirection) -> Vec<String> {
        found(
            fixture,
            &SearchRequest {
                sort: Sort { field, direction },
                ..request(ParsedTagSearch::default())
            },
        )
    }

    #[test]
    fn sorting_by_capture_time_runs_both_ways() {
        let fixture = sortable();

        assert_eq!(
            sorted(&fixture, SortField::Captured, SortDirection::Desc),
            vec!["later", "flat", "heavy"],
        );
        assert_eq!(
            sorted(&fixture, SortField::Captured, SortDirection::Asc),
            vec!["heavy", "flat", "later"],
        );
    }

    #[test]
    fn sorting_by_file_size_reads_the_bytes_not_the_pixels() {
        let fixture = sortable();

        assert_eq!(
            sorted(&fixture, SortField::Size, SortDirection::Asc),
            vec!["flat", "later", "heavy"],
        );
        assert_eq!(
            sorted(&fixture, SortField::Size, SortDirection::Desc),
            vec!["heavy", "later", "flat"],
        );
    }

    /// `trash` design D16: the trash's own order is the time of trashing, so
    /// what was thrown away last is first and a slip is one Restore away.
    #[test]
    fn the_trash_view_orders_by_trash_time() {
        let fixture = sortable();
        mark_deleted(&fixture.library.conn, "heavy", 10);
        mark_deleted(&fixture.library.conn, "later", 20);
        mark_deleted(&fixture.library.conn, "flat", 30);
        let trashed = |direction| {
            found(
                &fixture,
                &SearchRequest {
                    view: SearchView::Trash,
                    sort: Sort {
                        field: SortField::Trashed,
                        direction,
                    },
                    ..request(ParsedTagSearch::default())
                },
            )
        };

        assert_eq!(trashed(SortDirection::Desc), vec!["flat", "later", "heavy"]);
        assert_eq!(trashed(SortDirection::Asc), vec!["heavy", "later", "flat"]);
    }

    /// The area, as the lifted `sortImages` compared it — not either edge, and
    /// not the byte size, which orders these three differently.
    #[test]
    fn sorting_by_dimensions_compares_the_pixel_area() {
        let fixture = sortable();

        assert_eq!(
            sorted(&fixture, SortField::Dimensions, SortDirection::Asc),
            vec!["flat", "heavy", "later"],
        );
        assert_eq!(
            sorted(&fixture, SortField::Dimensions, SortDirection::Desc),
            vec!["later", "heavy", "flat"],
        );
    }

    /// The `sort-and-group` scenario "Sorting by last change": an old capture
    /// edited now comes first. The edit goes through the real write path, which
    /// is what marks `updated_at` (design D9).
    #[test]
    fn sorting_by_last_change_follows_an_edit_rather_than_the_capture() {
        let fixture = sortable();

        crate::tags::update_tags(&fixture.library, "heavy", &["cat".to_string()]).unwrap();

        assert_eq!(
            sorted(&fixture, SortField::Updated, SortDirection::Desc)[0],
            "heavy",
            "the oldest capture is the newest change",
        );
    }

    /// Every sort column has ties by nature, and `id DESC` is what keeps a page
    /// boundary from repeating one row and skipping another (design D6).
    #[test]
    fn paging_a_sort_whose_values_are_all_equal_shows_every_row_once() {
        let dir = tempfile::tempdir().unwrap();
        let library = Library::open_or_create(dir.path()).unwrap();
        let png = png_bytes(8, 8);
        for index in 0..6 {
            store(
                &library,
                &format!("same-{index}"),
                &png,
                &[],
                None,
                None,
                "identical",
                1_000,
            );
        }
        let fixture = Fixture { _dir: dir, library };

        let mut seen: Vec<String> = Vec::new();
        for offset in (0..6).step_by(2) {
            seen.extend(found(
                &fixture,
                &SearchRequest {
                    sort: Sort {
                        field: SortField::Size,
                        direction: SortDirection::Desc,
                    },
                    limit: 2,
                    offset,
                    ..request(ParsedTagSearch::default())
                },
            ));
        }

        let mut unique = seen.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), 6, "each row exactly once, got {seen:?}");
    }

    fn grouped(fixture: &Fixture, group: GroupBy) -> SearchResult {
        search(
            &fixture.library.conn,
            &SearchRequest {
                group,
                ..request(ParsedTagSearch::default())
            },
        )
        .unwrap()
    }

    fn slices(result: &SearchResult) -> Vec<(String, i64)> {
        result
            .groups
            .iter()
            .map(|slice| (slice.key.clone(), slice.count))
            .collect()
    }

    fn ids(result: &SearchResult) -> Vec<String> {
        result
            .images
            .iter()
            .map(|record| record.id.clone())
            .collect()
    }

    /// `alice` has three, `bob` and `carol` one each, and two pages name no
    /// account at all — one a foreign host, one of X's own pages.
    fn accounts() -> Fixture {
        let dir = tempfile::tempdir().unwrap();
        let library = Library::open_or_create(dir.path()).unwrap();
        let png = png_bytes(8, 8);
        let pages = [
            ("a1", "https://x.com/alice/status/1"),
            ("a2", "https://x.com/alice/status/2"),
            ("a3", "https://x.com/alice/status/3"),
            ("b1", "https://twitter.com/bob/status/1"),
            ("c1", "https://www.x.com/carol/status/1"),
            ("n1", "https://example.test/gallery/1"),
            ("n2", "https://x.com/i/status/1"),
        ];
        for (index, (id, page)) in pages.iter().enumerate() {
            store(
                &library,
                id,
                &png,
                &[],
                None,
                Some(page),
                "a page",
                1_000 - index as i64,
            );
        }

        Fixture { _dir: dir, library }
    }

    #[test]
    fn grouping_by_account_drops_pages_naming_none_and_puts_the_largest_group_first() {
        let fixture = accounts();

        let result = grouped(&fixture, GroupBy::XAccount);

        assert_eq!(
            slices(&result),
            vec![
                ("alice".to_string(), 3),
                ("bob".to_string(), 1),
                ("carol".to_string(), 1),
            ],
            "largest first, equal sizes by name",
        );
        assert_eq!(ids(&result), vec!["a1", "a2", "a3", "b1", "c1"]);
    }

    /// What is counted is what is shown (spec `library-browse`): the two images
    /// no account names are not in the result at all, so `total` cannot be the
    /// ungrouped count.
    #[test]
    fn a_grouping_that_admits_only_some_images_is_reflected_in_the_total() {
        let fixture = accounts();

        let ungrouped = grouped(&fixture, GroupBy::None);
        let result = grouped(&fixture, GroupBy::XAccount);

        assert_eq!(ungrouped.total, 7);
        assert!(ungrouped.groups.is_empty(), "nothing grouped, no slices");
        assert_eq!(result.total, 5);
        assert_eq!(
            result.total,
            result.groups.iter().map(|slice| slice.count).sum::<i64>(),
        );
    }

    /// A duplicate is the legacy pair of values: same pixel dimensions and same
    /// byte size. `twin-jpeg` shares its dimensions with the PNG pair and not
    /// its size, so dimensions alone would wrongly put it in their group.
    fn duplicates() -> Fixture {
        let dir = tempfile::tempdir().unwrap();
        let library = Library::open_or_create(dir.path()).unwrap();
        let small = png_bytes(8, 8);
        let large = png_bytes(32, 32);
        let rows: [(&str, &Vec<u8>); 6] = [
            ("small-1", &small),
            ("small-2", &small),
            ("large-1", &large),
            ("large-2", &large),
            ("lonely", &png_bytes(64, 64)),
            ("twin-jpeg", &jpeg_bytes(8, 8)),
        ];
        for (index, (id, bytes)) in rows.iter().enumerate() {
            store(
                &library,
                id,
                bytes,
                &[],
                None,
                None,
                "a page",
                1_000 - index as i64,
            );
        }

        Fixture { _dir: dir, library }
    }

    #[test]
    fn grouping_by_duplicates_keeps_only_triples_two_or_more_images_share() {
        let fixture = duplicates();

        let result = grouped(&fixture, GroupBy::Duplicates);

        // By key, as the lifted `getVisualOrder` ordered them — string order, so
        // `32x32-…` comes before `8x8-…`.
        assert_eq!(
            slices(&result),
            vec![("32x32-217".to_string(), 2), ("8x8-139".to_string(), 2)],
        );
        assert_eq!(
            ids(&result),
            vec!["large-1", "large-2", "small-1", "small-2"]
        );
        assert_eq!(result.total, 4, "the singletons are not in the result");
    }

    /// The membership question a duplicates grouping asks is about the *matched*
    /// set, not the library: an image whose twin the search filtered out has no
    /// twin left to be a duplicate of.
    #[test]
    fn a_duplicate_needs_its_twin_to_have_survived_the_filter() {
        let fixture = duplicates();

        let result = search(
            &fixture.library.conn,
            &SearchRequest {
                group: GroupBy::Duplicates,
                ..request(ParsedTagSearch {
                    exclude_tags: vec!["nothing".into()],
                    ..Default::default()
                })
            },
        )
        .unwrap();
        assert_eq!(result.total, 4, "a filter matching everything changes none");

        let text_narrowed = search(
            &fixture.library.conn,
            &SearchRequest {
                group: GroupBy::Duplicates,
                ..text_request("page")
            },
        )
        .unwrap();
        assert_eq!(text_narrowed.total, 4);

        crate::tags::update_tags(&fixture.library, "small-2", &["gone".to_string()]).unwrap();
        let one_twin_filtered_out = search(
            &fixture.library.conn,
            &SearchRequest {
                group: GroupBy::Duplicates,
                ..request(ParsedTagSearch {
                    exclude_tags: vec!["gone".into()],
                    ..Default::default()
                })
            },
        )
        .unwrap();

        assert_eq!(
            slices(&one_twin_filtered_out),
            vec![("32x32-217".to_string(), 2)],
            "`small-1` is alone in the matched set and so is no duplicate",
        );
    }

    #[test]
    fn a_group_orders_its_own_images_by_the_chosen_sort() {
        let fixture = accounts();

        let result = search(
            &fixture.library.conn,
            &SearchRequest {
                group: GroupBy::XAccount,
                sort: Sort {
                    field: SortField::Captured,
                    direction: SortDirection::Asc,
                },
                ..request(ParsedTagSearch::default())
            },
        )
        .unwrap();

        assert_eq!(
            ids(&result),
            vec!["a3", "a2", "a1", "b1", "c1"],
            "the group order is still largest-first; the sort orders within it",
        );
    }

    #[test]
    fn a_grouped_page_is_a_window_on_the_same_order() {
        let fixture = accounts();

        let result = search(
            &fixture.library.conn,
            &SearchRequest {
                group: GroupBy::XAccount,
                limit: 2,
                offset: 2,
                ..request(ParsedTagSearch::default())
            },
        )
        .unwrap();

        assert_eq!(ids(&result), vec!["a3", "b1"]);
        assert_eq!(
            result.groups.len(),
            3,
            "the slices describe the whole result, not the page",
        );
    }

    /// The `computeRatingCounts` block of the deleted `filters.test.ts`, on the
    /// same four images: `cat` on three of them with three different ratings,
    /// `dog` on the fourth.
    fn rated() -> Fixture {
        let dir = tempfile::tempdir().unwrap();
        let library = Library::open_or_create(dir.path()).unwrap();
        let png = png_bytes(8, 8);
        let rows = [
            ("a", vec!["cat"], Some("s")),
            ("b", vec!["cat"], Some("e")),
            ("c", vec!["cat"], None),
            ("d", vec!["dog"], Some("s")),
        ];
        for (index, (id, tags, rating)) in rows.iter().enumerate() {
            store(
                &library,
                id,
                &png,
                tags,
                *rating,
                None,
                "a page",
                1_000 - index as i64,
            );
        }

        Fixture { _dir: dir, library }
    }

    fn counts(fixture: &Fixture, req: &SearchRequest) -> TagCounts {
        tag_counts(&fixture.library.conn, req).unwrap()
    }

    fn tag_pairs(counts: &TagCounts) -> Vec<(String, i64)> {
        counts
            .tags
            .iter()
            .map(|tag| (tag.name.clone(), tag.count))
            .collect()
    }

    #[test]
    fn rating_counts_describe_the_filtered_set() {
        let fixture = rated();

        let counts = counts(&fixture, &request(ParsedTagSearch::default()));

        assert_eq!(
            counts.ratings,
            RatingCounts {
                g: 0,
                s: 2,
                q: 0,
                e: 1,
                unrated: 1,
            }
        );
    }

    /// Design D7: every collection is listed, zero included, and a matched
    /// image is counted with the rating clause in effect (`s` here narrows
    /// the matched set the same way it narrows the tag list beside it).
    #[test]
    fn collection_counts_list_every_collection_zero_included_over_the_matched_set() {
        let fixture = rated();
        let queue = crate::collections::create(&fixture.library, "Queue").unwrap();
        crate::collections::add(
            &fixture.library,
            &["a".to_string(), "b".to_string()],
            &queue.id,
        )
        .unwrap();

        let counts = counts(
            &fixture,
            &request(ParsedTagSearch {
                ratings: vec!["s".to_string()],
                ..Default::default()
            }),
        );

        let pairs: Vec<(String, i64)> = counts
            .collections
            .iter()
            .map(|collection| (collection.name.clone(), collection.count))
            .collect();
        assert_eq!(
            pairs,
            vec![("Favorites".to_string(), 0), ("Queue".to_string(), 1)],
            "`a` is rated `s` and in Queue; `b` is rated `e` and filtered out",
        );
    }

    /// Design D8's asymmetry, ported from `computeRatingCounts(…, includeRating:
    /// false)`: the pills answer "how many if I switch", so the search's own
    /// `rating:` is the one clause they ignore — while the tag list beside them
    /// counts over the result the rating filter produced.
    #[test]
    fn the_rating_half_ignores_the_searchs_own_rating_and_the_tag_half_does_not() {
        let fixture = rated();

        let counts = counts(
            &fixture,
            &request(ParsedTagSearch {
                include_tags: vec!["cat".into()],
                ratings: vec!["s".into()],
                ..Default::default()
            }),
        );

        assert_eq!(
            counts.ratings,
            RatingCounts {
                g: 0,
                s: 1,
                q: 0,
                e: 1,
                unrated: 1,
            },
            "every `cat`, whatever it is rated",
        );
        assert_eq!(
            tag_pairs(&counts),
            vec![("cat".to_string(), 1)],
            "only the one `cat` the rating filter left",
        );
    }

    #[test]
    fn unrated_counts_a_blank_rating_as_no_rating() {
        let fixture = rated();
        fixture
            .library
            .conn
            .execute("UPDATE images SET rating = '' WHERE id = 'd'", [])
            .unwrap();

        let counts = counts(&fixture, &request(ParsedTagSearch::default()));

        assert_eq!(counts.ratings.unrated, 2);
        assert_eq!(counts.ratings.s, 1);
    }

    #[test]
    fn counts_describe_the_whole_result_and_not_the_page_asked_for() {
        let fixture = rated();

        let whole = counts(&fixture, &request(ParsedTagSearch::default()));
        let one_row = counts(
            &fixture,
            &SearchRequest {
                limit: 1,
                offset: 0,
                ..request(ParsedTagSearch::default())
            },
        );

        assert_eq!(
            tag_pairs(&whole),
            vec![("cat".to_string(), 3), ("dog".to_string(), 1)],
            "most carried first, then by name",
        );
        assert_eq!(tag_pairs(&one_row), tag_pairs(&whole));
        assert_eq!(one_row.ratings, whole.ratings);
    }

    #[test]
    fn counts_honour_the_group_restriction() {
        let fixture = accounts();
        crate::tags::update_tags(&fixture.library, "n1", &["offsite".to_string()]).unwrap();
        crate::tags::update_tags(&fixture.library, "a1", &["onsite".to_string()]).unwrap();

        let ungrouped = counts(&fixture, &request(ParsedTagSearch::default()));
        let by_account = counts(
            &fixture,
            &SearchRequest {
                group: GroupBy::XAccount,
                ..request(ParsedTagSearch::default())
            },
        );

        assert_eq!(ungrouped.ratings.unrated, 7);
        assert_eq!(
            by_account.ratings.unrated, 5,
            "the two account-less pages go"
        );
        assert_eq!(
            tag_pairs(&by_account),
            vec![("onsite".to_string(), 1)],
            "`offsite` is on a page the grouping does not admit",
        );
    }

    fn account_pairs(counts: &TagCounts) -> Vec<(String, i64)> {
        counts
            .accounts
            .iter()
            .map(|slice| (slice.key.clone(), slice.count))
            .collect()
    }

    /// The account rail's own half of `tag_counts`: nothing to show when there
    /// is no rail, so an ungrouped request skips the whole pass over every
    /// page address.
    #[test]
    fn accounts_is_empty_when_the_request_is_not_grouped_by_account() {
        let fixture = accounts();

        let counts = counts(&fixture, &request(ParsedTagSearch::default()));

        assert!(counts.accounts.is_empty());
    }

    /// `alice` keeps two of her three when `cat` narrows the search and `bob`
    /// keeps his one, but `carol` — matching nothing — still gets a row at
    /// zero: the rail's whole point is to say what she would give if the
    /// search switched to her.
    #[test]
    fn grouped_by_account_a_tag_search_lists_every_account_zero_included() {
        let fixture = accounts();
        crate::tags::update_tags(&fixture.library, "a1", &["cat".to_string()]).unwrap();
        crate::tags::update_tags(&fixture.library, "a2", &["cat".to_string()]).unwrap();
        crate::tags::update_tags(&fixture.library, "b1", &["cat".to_string()]).unwrap();

        let counts = counts(
            &fixture,
            &SearchRequest {
                group: GroupBy::XAccount,
                ..request(ParsedTagSearch {
                    include_tags: vec!["cat".into()],
                    ..Default::default()
                })
            },
        );

        assert_eq!(
            account_pairs(&counts),
            vec![
                ("alice".to_string(), 2),
                ("bob".to_string(), 1),
                ("carol".to_string(), 0),
            ],
        );
    }

    /// Design D8's "how many if I switched", applied to accounts:
    /// `account:alice` narrows what `search` returns, but the rail still
    /// answers what every other account would give if the search switched to
    /// them.
    #[test]
    fn grouped_by_account_the_accounts_own_filter_is_dropped_for_the_rail() {
        let fixture = accounts();
        let req = SearchRequest {
            group: GroupBy::XAccount,
            ..request(ParsedTagSearch {
                accounts: vec!["alice".into()],
                ..Default::default()
            })
        };

        let counts = counts(&fixture, &req);
        let result = search(&fixture.library.conn, &req).unwrap();

        assert_eq!(
            account_pairs(&counts),
            vec![
                ("alice".to_string(), 3),
                ("bob".to_string(), 1),
                ("carol".to_string(), 1),
            ],
            "every account's own count, `account:alice` notwithstanding",
        );
        assert_eq!(
            result.total, 3,
            "the search itself still honours account:alice"
        );
    }

    /// The rail describes the view it was asked for: a trashed page's account
    /// shows up in the trash's rail and an untrashed one's does not, however
    /// many images that account has elsewhere.
    #[test]
    fn the_trash_views_rail_lists_only_accounts_of_trashed_images() {
        let fixture = accounts();
        mark_deleted(&fixture.library.conn, "a1", 1);
        mark_deleted(&fixture.library.conn, "a2", 2);
        mark_deleted(&fixture.library.conn, "b1", 3);

        let counts = counts(
            &fixture,
            &SearchRequest {
                view: SearchView::Trash,
                group: GroupBy::XAccount,
                ..request(ParsedTagSearch::default())
            },
        );

        assert_eq!(
            account_pairs(&counts),
            vec![("alice".to_string(), 2), ("bob".to_string(), 1)],
            "carol's only image was never trashed",
        );
    }

    /// The URL table of the deleted `getXAccountFromUrl`, which is what closes
    /// the two-implementations question: there is one rule now, and this is the
    /// fixture it answers to.
    #[test]
    fn x_account_reads_the_first_path_segment() {
        assert_eq!(x_account("https://x.com/alice/status/123"), Some("alice"));
        assert_eq!(x_account("https://twitter.com/bob/status/1"), Some("bob"));
        assert_eq!(x_account("https://www.x.com/carol"), Some("carol"));
        assert_eq!(
            x_account("https://www.twitter.com/dave/photo"),
            Some("dave")
        );
        assert_eq!(x_account("https://twitter.com/Bob"), Some("Bob"));
        assert_eq!(x_account("https://www.x.com/carol/"), Some("carol"));
        assert_eq!(
            x_account("https://www.twitter.com/dave?src=x"),
            Some("dave")
        );
        assert_eq!(x_account("http://X.com:443/erin/status/2"), Some("erin"));
    }

    #[test]
    fn x_account_ignores_x_own_pages_whatever_case_they_are_written_in() {
        for reserved in [
            "i",
            "home",
            "explore",
            "notifications",
            "messages",
            "search",
        ] {
            let url = format!("https://x.com/{reserved}/status/1");
            assert_eq!(x_account(&url), None, "{url:?} names no account");
            let shouted = format!("https://x.com/{}/status/1", reserved.to_uppercase());
            assert_eq!(x_account(&shouted), None, "{shouted:?} names no account");
        }
    }

    #[test]
    fn x_account_ignores_other_hosts_and_anything_that_is_not_a_url() {
        for url in [
            "https://example.com/alice",
            "https://example.test/alice",
            "https://notx.com/alice",
            "https://x.com",
            "https://x.com/",
            "https://x.com/?q=1",
            "not a url",
            "x.com/alice",
            "",
        ] {
            assert_eq!(x_account(url), None, "expected no account in {url:?}");
        }
    }

    /// The last row of that table: a record with no page URL. It reaches SQL as
    /// NULL rather than as a string, so the scalar function is what has to
    /// answer it — `x_account(NULL)` is NULL, and the image is in no group.
    #[test]
    fn an_image_with_no_page_url_is_in_no_account_group() {
        let dir = tempfile::tempdir().unwrap();
        let library = Library::open_or_create(dir.path()).unwrap();
        store(
            &library,
            "nowhere",
            &png_bytes(8, 8),
            &[],
            None,
            None,
            "a page",
            100,
        );
        store(
            &library,
            "somewhere",
            &png_bytes(8, 8),
            &[],
            None,
            Some("https://x.com/alice/status/1"),
            "a page",
            200,
        );
        let fixture = Fixture { _dir: dir, library };

        let result = grouped(&fixture, GroupBy::XAccount);

        assert_eq!(slices(&result), vec![("alice".to_string(), 1)]);
        assert_eq!(ids(&result), vec!["somewhere"]);
    }
}
