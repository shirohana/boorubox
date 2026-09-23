//! Rust mirror of `packages/shared/src/index.ts`: the localhost transport
//! contract, the site-adapter record and the IPC contract with the webview.
//!
//! That file is this file's sibling. Change one, change the other — nothing
//! checks them against each other.
//!
//! FIXME: hand-mirroring two type sets is a shortcut. The right shape is one
//! generated from the other (`ts-rs` or `specta` emitting the TypeScript from
//! these structs), so a renamed field cannot compile on one side and silently
//! break the other. Not built yet because the shared package is also consumed
//! by the extension, which never sees Rust.

use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::error::AppError;

/// Default localhost port for the capture listener (docs/requirements.md §5).
pub const DEFAULT_PORT: u16 = 47201;

/// Only requests from an extension origin may post captures (§5).
pub const EXTENSION_ORIGIN_PREFIX: &str = "chrome-extension://";

/// The grid tile's edge in pixels: what the size slider offers and what
/// `set_grid_tile_size` clamps to (design D11). The webview has the same three
/// numbers for the slider's own bounds; these are the ones that decide what
/// reaches the settings file.
pub const GRID_TILE_MIN: u32 = 120;
pub const GRID_TILE_MAX: u32 = 360;
pub const GRID_TILE_DEFAULT: u32 = 180;

/// The click zoom's ceiling, as a percent of the fit: what the settings
/// slider offers and what `set_click_zoom_ceiling_percent` clamps to
/// (`click-zoom-ceiling` design D1). The webview has the same three numbers
/// for the slider's own bounds; these are the ones that decide what reaches
/// the settings file.
pub const CLICK_ZOOM_CEILING_MIN: u32 = 125;
pub const CLICK_ZOOM_CEILING_MAX: u32 = 350;
pub const CLICK_ZOOM_CEILING_DEFAULT: u32 = 150;

/// How an image entered the library. The string form is what the `images.source`
/// column holds and what crosses IPC, so it is defined once here: `as_str` and
/// `FromStr` back the serde, `ToSql` and `FromSql` impls below.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageSource {
    Extension,
    Local,
    LegacyBundle,
}

impl ImageSource {
    pub fn as_str(self) -> &'static str {
        match self {
            ImageSource::Extension => "extension",
            ImageSource::Local => "local",
            ImageSource::LegacyBundle => "legacy-bundle",
        }
    }
}

impl FromStr for ImageSource {
    type Err = AppError;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        match raw {
            "extension" => Ok(ImageSource::Extension),
            "local" => Ok(ImageSource::Local),
            "legacy-bundle" => Ok(ImageSource::LegacyBundle),
            other => Err(AppError::BadRequest(format!(
                "unknown image source {other:?}"
            ))),
        }
    }
}

impl Serialize for ImageSource {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for ImageSource {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        raw.parse().map_err(serde::de::Error::custom)
    }
}

impl rusqlite::ToSql for ImageSource {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        Ok(rusqlite::types::ToSqlOutput::from(self.as_str()))
    }
}

impl rusqlite::types::FromSql for ImageSource {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        value
            .as_str()?
            .parse()
            .map_err(|e| rusqlite::types::FromSqlError::Other(Box::new(e)))
    }
}

/// The five kinds of tag `tag-vocabulary` design D1 fixes. A tag belongs to
/// exactly one, general unless given another; the five names here are both
/// the storage form (`tags.category`) and the wire form, the same shape
/// `ImageSource` already uses for a column with the same job: `as_str` and
/// `FromStr` back the serde, `ToSql` and `FromSql` impls below.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum TagCategory {
    Artist,
    Copyright,
    Character,
    Meta,
    #[default]
    General,
}

impl TagCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            TagCategory::Artist => "artist",
            TagCategory::Copyright => "copyright",
            TagCategory::Character => "character",
            TagCategory::Meta => "meta",
            TagCategory::General => "general",
        }
    }
}

impl FromStr for TagCategory {
    type Err = AppError;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        match raw {
            "artist" => Ok(TagCategory::Artist),
            "copyright" => Ok(TagCategory::Copyright),
            "character" => Ok(TagCategory::Character),
            "meta" => Ok(TagCategory::Meta),
            "general" => Ok(TagCategory::General),
            other => Err(AppError::BadRequest(format!(
                "unknown tag category {other:?}"
            ))),
        }
    }
}

impl Serialize for TagCategory {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for TagCategory {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        raw.parse().map_err(serde::de::Error::custom)
    }
}

impl rusqlite::ToSql for TagCategory {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        Ok(rusqlite::types::ToSqlOutput::from(self.as_str()))
    }
}

impl rusqlite::types::FromSql for TagCategory {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        value
            .as_str()?
            .parse()
            .map_err(|e| rusqlite::types::FromSqlError::Other(Box::new(e)))
    }
}

/// One tag outside the `(general, unpinned)` default: its name, its category
/// and whether it is pinned — the vocabulary's own row (`tag-vocabulary`
/// design D2). What `tag_vocabulary` answers with, what `library.json`'s
/// `tags` key lists, and what a rebuild restores onto the row verbatim.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagEntry {
    pub name: String,
    pub category: TagCategory,
    pub pinned: bool,
}

/// One edit's every part (`stamps` design D1, D2): parsed from a stamp's text
/// in the webview, or built directly by the bulk tag dialog and the pinned
/// chip filling only the parts they mean. `tags::apply_edit` is the one door
/// every caller of this shape writes through, so a stamp, the dialog and the
/// chip cannot disagree about what one transaction contains. `add_collections`
/// and `remove_collections` are slugs (`collections::slug`), resolved to ids
/// before anything is written; `rating` sets and never clears — clearing has
/// its own control (`set_rating(None)`), which no metatag spells.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagEditSpec {
    pub add: Vec<String>,
    pub remove: Vec<String>,
    pub add_collections: Vec<String>,
    pub remove_collections: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rating: Option<String>,
}

/// What a site adapter extracted, verbatim. `fields` stays untyped JSON: the
/// extension owns extraction, and rejecting a capture over an adapter field
/// shape the app does not read yet would lose the image.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SiteAdapterRecord {
    pub site: String,
    pub fields: serde_json::Value,
}

/// The JSON part of a `POST /captures` multipart body, and the whole body of
/// `POST /captures/pending`, where the extension announces the capture it is
/// about to fetch bytes for (design D2). One shape for both: an announcement
/// that could not be sent as the later `meta` would be a second type to mirror
/// and a placeholder that names a different page than the image it becomes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureMeta {
    /// Caller-generated UUID. Delivery is idempotent on it.
    pub id: String,
    pub image_url: String,
    pub page_url: String,
    pub page_title: String,
    /// Epoch milliseconds.
    pub captured_at: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adapter: Option<SiteAdapterRecord>,
}

/// Body of `DELETE /captures/pending/{id}` and payload of `capture:withdrawn`:
/// the announced capture will not arrive, so its placeholder goes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureWithdrawn {
    pub id: String,
    /// Why, when the app or the extension knows; absent when nothing named one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Body of `GET /status` when a library is open.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusResponse {
    pub version: String,
    pub library_path: String,
    pub image_count: i64,
}

/// One recorded post (`booru-upload` design D2, D5): `site` holds
/// `BooruSite.id`, a stable slug rather than a foreign key, since the record
/// must outlive the site being removed (spec `posted-label`, "A record
/// outlives the site configuration"). The post's address is computed —
/// `<base_url>/posts/<remote_id>` — never stored, so there is one source of
/// truth for where a post lives.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostRef {
    pub site: String,
    pub remote_id: String,
    /// Epoch milliseconds.
    pub posted_at: i64,
}

/// One row of the library, as the webview sees it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageRecord {
    pub id: String,
    pub ext: String,
    /// Path to the file, relative to the library root, `/`-separated on every
    /// platform (design D1, D2): `images/<a1>/<b2>/<id>.<ext>`. The one field
    /// that names where the file is — the webview builds its URL from this
    /// rather than composing the layout itself.
    pub file: String,
    pub mime: String,
    pub size: i64,
    pub width: u32,
    pub height: u32,
    pub source: ImageSource,
    /// Adapter site for `extension`, bundle id for `legacy-bundle`, else null.
    pub source_ref: Option<String>,
    pub image_url: Option<String>,
    pub page_url: Option<String>,
    pub page_title: Option<String>,
    /// The X account `page_url` names, by the same rule the `account:` search
    /// filter uses (`query::x_account`, `inspector-polish` design D4).
    /// Derived at load time, never a column and never part of the sidecar —
    /// a second source for this would let the filter and the display
    /// disagree about what an image's account is.
    pub account: Option<String>,
    /// What the capturing client's site adapter extracted, as received
    /// (design D11). Storage only in this schema: nothing derives tags, a
    /// rating or an artist from it yet.
    pub adapter: Option<SiteAdapterRecord>,
    /// `g` | `s` | `q` | `e` in the TypeScript mirror. Kept a string here
    /// because legacy-bundle import (D9) carries whatever the old library
    /// stored, and ingest must not drop an image over an unknown rating.
    pub rating: Option<String>,
    pub tags: Vec<String>,
    pub captured_at: i64,
    /// The file's own modification time, epoch milliseconds; `None` for an
    /// image that never came from a file (design D11).
    pub file_modified_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
    pub deleted_at: Option<i64>,
    /// The file under `images/` was not there at the last check.
    pub missing: bool,
    /// Where this image has been posted, one entry per site (`booru-upload`
    /// design D5). Empty for an image never posted anywhere.
    pub posts: Vec<PostRef>,
    /// The collections this image is in, by id, sorted (`collections` design
    /// D4). Never a tag: not sent to a booru, not matched by a tag term.
    pub collections: Vec<String>,
}

/// A named, unordered set of images the user keeps for themselves
/// (`collections` design D1–D3): favourites, a project, a batch to upload.
/// Referenced everywhere else by `id`, which never changes; `name` is the only
/// field a rename touches.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Collection {
    pub id: String,
    pub name: String,
    /// `name`, lower-cased with runs of whitespace as `_` (design D2),
    /// computed once in Rust so the webview never recomputes it and the query
    /// language and the sidebar always agree on what a collection is called.
    pub slug: String,
    /// Epoch milliseconds.
    pub created_at: i64,
    pub updated_at: i64,
}

/// One collection's row in the sidebar and the inspector's menus: the
/// collection plus how many of the matched result are in it (design D7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionCount {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub count: i64,
}

/// What `update_facts` takes (`editable-info` design D1): `None` and an empty
/// string (after trimming) both mean "clear this field" — the webview's Save
/// button has no way to tell the two apart once a user has emptied a field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FactsEdit {
    pub page_title: Option<String>,
    pub page_url: Option<String>,
    pub image_url: Option<String>,
}

/// The `tagcount:` comparison, as `parseTagSearch` emits it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TagCountOperator {
    #[serde(rename = "=")]
    Eq,
    #[serde(rename = ">")]
    Gt,
    #[serde(rename = "<")]
    Lt,
    #[serde(rename = ">=")]
    Gte,
    #[serde(rename = "<=")]
    Lte,
    #[serde(rename = "range")]
    Range,
    #[serde(rename = "list")]
    List,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagCountFilter {
    pub operator: TagCountOperator,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub values: Option<Vec<i64>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<i64>,
}

/// One `tagcount:`-syntax term over one tag category, or every tag when
/// `category` is `None` (`tagcount:` itself). `category-count-search` design
/// D1: the TypeScript mirror is `TagCountTerm` in `packages/shared/src/index.ts`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagCountTerm {
    pub category: Option<TagCategory>,
    pub filter: TagCountFilter,
}

/// The parsed query the webview sends to Rust, which compiles it to SQL. The
/// parser (`$lib/domain/tag-utils`) is the only definition of the query
/// language; Rust never sees the query string (design D3).
///
/// Arrays, not sets: this crosses the IPC boundary as JSON (design D13).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedTagSearch {
    pub include_tags: Vec<String>,
    pub exclude_tags: Vec<String>,
    pub or_groups: Vec<Vec<String>>,
    /// `g` | `s` | `q` | `e`.
    pub ratings: Vec<String>,
    /// MIME types from `is:png` and friends.
    pub file_types: Vec<String>,
    /// `tagcount:` and the five category count metatags (`category-count-search`
    /// design D1): one entry per count metatag matched, `tagcount:`'s own entry
    /// carrying `category: None`. Empty means no count filter.
    pub tag_count_terms: Vec<TagCountTerm>,
    pub include_unrated: bool,
    pub accounts: Vec<String>,
    pub exclude_accounts: Vec<String>,
    /// Slugs (design D6): `collection:my_favorites` compiles against
    /// `collections.slug`, never the id — the webview never sees one.
    pub collections: Vec<String>,
    pub exclude_collections: Vec<String>,
    /// `collection:any` / `-collection:none` (`category-count-search` design
    /// D4): the image is in at least one collection. Independent of
    /// `no_collection` — both set compiles to a clause that matches nothing,
    /// which is what the query says.
    pub any_collection: bool,
    /// `collection:none` / `-collection:any`: the image is in no collection.
    pub no_collection: bool,
}

/// What the four sorts compare. An enum rather than the legacy's `field-direction`
/// string: it arrives from IPC and names a column expression, and this file's
/// rule is that nothing but a placeholder is ever formatted into SQL. The enum
/// cannot spell a column that does not exist (design D6).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortField {
    #[default]
    Captured,
    Updated,
    Size,
    Dimensions,
    /// `deleted_at`; meaningful in the trash view only (`trash` design D16).
    Trashed,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortDirection {
    Asc,
    #[default]
    Desc,
}

/// Newest capture first is the default the grid opens on (spec `sort-and-group`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sort {
    pub field: SortField,
    pub direction: SortDirection,
}

/// How the result set is divided. A grouping also restricts it: `XAccount`
/// admits only pages naming an account, `Duplicates` only images sharing their
/// dimensions and byte size with another (design D7).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GroupBy {
    #[default]
    None,
    XAccount,
    Duplicates,
}

/// One group of the whole result set, in the order the result is returned in.
/// `key` is raw — the account handle, or `WIDTHxHEIGHT-SIZE` — because a display
/// label is UI and §6 keeps UI out of the storage layer (design D7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupSlice {
    pub key: String,
    pub count: i64,
}

/// Which set of images a `SearchRequest` draws from (`trash` design D2): the
/// library, with deleted images excluded, or the trash, with only deleted
/// images and never an undeleted one. No third `all` variant — nothing in the
/// app shows both at once, and a variant with no caller is a dead control this
/// type has no reason to carry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SearchView {
    Library,
    Trash,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchRequest {
    pub query: ParsedTagSearch,
    /// Free text matched against page title and URLs through FTS5 (design D14).
    pub text: String,
    pub view: SearchView,
    pub sort: Sort,
    pub group: GroupBy,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub images: Vec<ImageRecord>,
    /// Matches before `limit`/`offset`.
    pub total: i64,
    /// Every group of the result; empty when ungrouped.
    pub groups: Vec<GroupSlice>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagCount {
    pub name: String,
    pub count: i64,
}

/// What `export_zip` answers with (`selection-and-bulk` design D11): how many
/// of the selection made it into the archive, and which ids did not because
/// their file was gone from `images/` by the time it was copied.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportReport {
    pub path: String,
    pub written: i64,
    pub missing: Vec<String>,
}

/// Payload of the `export:progress` event, emitted once per id including the
/// ones left out — so the last tick's `done` always equals `total`, mirroring
/// `ImportProgress` (design D13).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportProgress {
    pub done: i64,
    pub total: i64,
}

/// One sidecar `recover::rebuild` could not read (`library-sidecars` design
/// D11): the file's own path, and why, so the report can name it rather than
/// silently drop the image.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RebuildFailure {
    pub file: String,
    pub reason: String,
}

/// What `rebuild_library` answers with (design D11, D13), the contract pinned
/// in the change's `tasks.md` header: how many images came back, how many
/// sidecars could not be read and which, the rules and sites restored from
/// `library.json`, and the name the previous database was kept under (design
/// D10) — never a path the user has to go looking for.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RebuildReport {
    pub images: i64,
    pub failed: i64,
    pub failures: Vec<RebuildFailure>,
    pub kept_as: String,
    pub rules: i64,
    pub sites: i64,
    pub collections: i64,
}

/// Payload of the `library:rebuild` event, shown where no library is open
/// (design D13): its own event and type, never `import:progress` — that
/// carries fields this pass has no answer for and would put a phantom import
/// tile on a screen that has no import running.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RebuildProgress {
    pub done: i64,
    pub total: i64,
}

/// Payload of the `library:sidecars` event, shown as one tile in the pending-
/// work band while a library is open (design D13, spec `pending-work`) — the
/// backfill's own progress, distinct from [`RebuildProgress`] even though the
/// shape is the same, because the two are seen in different places and mirror
/// separately into `packages/shared`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SidecarsProgress {
    pub done: i64,
    pub total: i64,
}

/// What `delete_forever` and `empty_trash` answer with (`trash` design D4,
/// D7): how many images were permanently removed, and the full path of every
/// file that could not be unlinked — named on screen rather than swept, since
/// the record is gone either way and there is no unlink left to retry.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteReport {
    pub deleted: i64,
    pub files_left: Vec<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RatingCounts {
    pub g: i64,
    pub s: i64,
    pub q: i64,
    pub e: i64,
    pub unrated: i64,
}

/// What the sidebar draws. The two halves answer different questions and are
/// counted differently on purpose (design D8): `tags` over the fully filtered
/// result, `ratings` with the rating clause dropped, so a pill says how many the
/// search would return if that rating were asked for instead.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagCounts {
    /// Every tag carried by the matched set, plus — at zero, after the
    /// carried rows — every name of the request's own query
    /// (`include_tags`, `exclude_tags`, an `or_groups` member) that the
    /// matched set does not carry but that a real `tags` row exists for
    /// (design D8): a name no tag has is left out, since no menu could act
    /// on it.
    pub tags: Vec<TagCount>,
    pub ratings: RatingCounts,
    /// Every collection in the library, counted over the matched set with the
    /// rating clause included (design D7) — the sidebar's Collections section
    /// and the inspector's badges read this rather than keeping a count of
    /// their own.
    pub collections: Vec<CollectionCount>,
    /// Every account of the request's view — including one with no matches —
    /// when `group` is `x-account`; empty otherwise, since the rail this
    /// backs is shown only then and the pass over every page address is not
    /// free enough to run when it is not shown.
    ///
    /// Each count drops the query's own `account:`/`-account:` clause and
    /// honours everything else — the rating pills' rule (design D8 of
    /// `tags-and-ratings`, "how many if I switched"), applied to accounts:
    /// while `account:alice` narrows the search, `bob`'s count still answers
    /// what he would give if the search switched to him. Ordered by count
    /// descending then handle ascending, the order `GroupSlice`s for an
    /// account grouping already use.
    pub accounts: Vec<GroupSlice>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageCounts {
    pub total: i64,
    pub extension: i64,
    pub local: i64,
    pub legacy_bundle: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListenerStatus {
    pub running: bool,
    pub port: u16,
    /// Why the listener is not running, for settings to show (§5).
    pub error: Option<String>,
}

impl Default for ListenerStatus {
    fn default() -> Self {
        ListenerStatus {
            running: false,
            port: DEFAULT_PORT,
            error: None,
        }
    }
}

/// Everything the UI needs to decide between `/start` and the library.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryStatus {
    pub opened: bool,
    /// The path a launch-time open is still running against (`launch-screen`
    /// design D1) — set while `setup`'s blocking open of the remembered
    /// library runs, `None` once it settles either way. Lets the webview
    /// name the folder on the opening screen before `opened` can be true.
    pub opening: Option<String>,
    pub library_path: Option<String>,
    /// A remembered path that could not be opened; kept until another is picked.
    pub missing_path: Option<String>,
    /// A remembered path that could not be opened *because it is damaged*
    /// (`library-sidecars` design D9) — distinct from `missing_path`, which
    /// covers every other reason, so the start screen never has to guess
    /// which wording fits.
    pub damaged_path: Option<String>,
    /// A remembered path that could not be opened *because a newer build
    /// wrote it* (`library-recovery`'s "A library from a newer build"). Its
    /// own slot rather than a silence: a status naming none of the three
    /// reads on the start screen as "choose a library folder", which loses
    /// the folder the user already has and says nothing about why it will
    /// not open. Never offered a rebuild — rebuilding it would downgrade it.
    pub newer_path: Option<String>,
    pub image_count: i64,
    pub listener: ListenerStatus,
    pub version: String,
}

/// Which palette the app paints. `System` follows the OS (design D12).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

/// The preferences the webview reads and writes one field at a time (design D5).
/// The listener port is not here: it is already on `LibraryStatus.listener`, and
/// a second copy would be a second thing to keep in step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub theme: Theme,
    pub grid_tile_size: u32,
    /// Whether the grid's tag footer shows outside edit mode, where the mode
    /// forces it on regardless (`tile-tags-in-edit-mode` design D3). Kept
    /// beside `grid_tile_size` rather than with the mode itself: it is a
    /// display preference of this machine, like the tile size, not a fact
    /// about the library.
    pub show_tile_tags: bool,
    /// How far a click in the viewer may zoom, as a percent of the fit
    /// (`click-zoom-ceiling` design D1): the click's target is the cover or
    /// this ceiling times the fit, whichever is smaller.
    pub click_zoom_ceiling_percent: u32,
    /// Whether the sidebar's notes panel is folded away (`notes` design D13):
    /// a preference held for months, not the session state app-shell keeps out
    /// of `settings.json` — D13 records why that reading was right until a
    /// panel big enough to write in sat in the sidebar.
    pub notes_collapsed: bool,
    /// Whether the sidebar's collections section is folded away
    /// (`browse-feedback` design D4), copied from `notes_collapsed`'s line.
    pub collections_collapsed: bool,
    /// Whether the app reopens `LibraryStatus.libraryPath`'s remembered
    /// folder automatically at launch, or waits on the start screen for the
    /// user to pick one (`launch-screen` design D3). Takes effect at the
    /// next launch, not immediately.
    pub open_last_on_launch: bool,
}

/// One entry of the start screen's recent list. `name` and `available` are
/// derived from the path when the list is asked for, never stored (design D4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentLibrary {
    pub path: String,
    /// The folder's basename.
    pub name: String,
    /// `library.sqlite` is there and readable, checked at call time.
    pub available: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImportStatus {
    Imported,
    Skipped,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportOutcome {
    pub path: String,
    pub status: ImportStatus,
    /// New image id when `imported`. A bundle row (`legacy-bundle-import`
    /// design D6) also carries its id when `skipped` — the bundle keeps the
    /// row's own id rather than minting one, so the id is already known
    /// whether or not the row turned out to be new.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Why, when `skipped` or `failed`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl ImportOutcome {
    pub fn imported(path: impl Into<String>, id: String) -> Self {
        ImportOutcome {
            path: path.into(),
            status: ImportStatus::Imported,
            id: Some(id),
            reason: None,
        }
    }

    pub fn skipped(path: impl Into<String>, id: Option<String>, reason: impl Into<String>) -> Self {
        ImportOutcome {
            path: path.into(),
            status: ImportStatus::Skipped,
            id,
            reason: Some(reason.into()),
        }
    }

    pub fn failed(path: impl Into<String>, id: Option<String>, reason: impl Into<String>) -> Self {
        ImportOutcome {
            path: path.into(),
            status: ImportStatus::Failed,
            id,
            reason: Some(reason.into()),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportReport {
    pub imported: u32,
    pub skipped: u32,
    pub failed: u32,
    pub items: Vec<ImportOutcome>,
    /// Set when the run stopped on a Cancel rather than running to the end
    /// (`import-pause-cancel` design D3). The counts above are honest either
    /// way: everything counted here is really in the library.
    #[serde(default)]
    pub cancelled: bool,
}

/// One bundle part as `bundle_plan` (`import-confirm` design D2) found it,
/// before any row is read: either its row count, or why it would not open.
/// Exactly one of `rows` and `error` is `Some`. Both keys always serialize —
/// as `null` on the side that is `None` — because the shared contract
/// (`packages/shared/src/index.ts`) pins them as `number | null` and
/// `string | null`, not optional: a webview reading a missing key and a
/// webview reading `null` are the same case, but only one matches the type
/// the two runtimes agreed on.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BundlePartPlan {
    pub path: String,
    pub rows: Option<i64>,
    pub error: Option<String>,
}

/// What `bundle_plan` answers with: every part the confirm screen shows, in
/// the order `import_bundle` will read them, and `total` — the same number
/// the run's first `import:progress` carries (design D2), computed once here
/// so the webview never has to know an unopenable part is worth one item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BundlePlan {
    pub parts: Vec<BundlePartPlan>,
    pub total: u32,
}

/// Payload of the `import:progress` event emitted while an import runs.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportProgress {
    pub done: u32,
    pub total: u32,
    pub imported: u32,
    pub skipped: u32,
    pub failed: u32,
}

/// A named pattern and the tags it adds (`auto-tag-rules` design D1, D2).
/// Rules live in the library, not `settings.json`: library policy travels
/// with the folder. `tags` is a rule's payload, not tag rows — it is never
/// searched, counted or joined, only written later (design D2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rule {
    pub id: String,
    pub name: String,
    pub pattern: String,
    pub is_regex: bool,
    pub tags: Vec<String>,
    pub enabled: bool,
    /// Epoch milliseconds.
    pub created_at: i64,
    pub updated_at: i64,
}

/// What `rules_upsert` takes: `id` absent creates, present edits (design D6).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub name: String,
    pub pattern: String,
    pub is_regex: bool,
    pub tags: Vec<String>,
    pub enabled: bool,
}

/// One row of the rules list: the rule plus its pattern's validity, compiled
/// at read time and never stored (design D6) — a regular expression the
/// engine cannot use is reported here rather than dropped.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleListEntry {
    pub rule: Rule,
    pub pattern_error: Option<String>,
}

/// What `rules_import` answers with (design D10).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RulesImportReport {
    pub imported: u32,
    pub skipped: u32,
}

/// One rule's outcome in a `rules_run`: how many images it matched, in the
/// `rules` half of [`RulesRunReport`], or its `pattern_error` in the
/// `invalid` half — the same shape serves both (design D9).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleRunCount {
    pub id: String,
    pub name: String,
    pub matched: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pattern_error: Option<String>,
}

/// What `rules_run` answers with: how many images were examined and changed,
/// and each rule's own count, invalid ones named separately with their reason
/// rather than dropped (design D9).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RulesRunReport {
    pub examined: i64,
    pub changed: i64,
    pub rules: Vec<RuleRunCount>,
    pub invalid: Vec<RuleRunCount>,
}

/// A saved edit, written once in the tag language and applied by a click
/// (`stamps` design D1, D3): the text is stored exactly as typed, never the
/// parsed [`TagEditSpec`] — the grammar belongs to the webview, and storing
/// the parsed lists would freeze a stamp the user meant to keep editing.
/// Shaped like [`Rule`]: library-level, listed by creation order rather than
/// by name, since there is no reordering (`stamps` design D3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Stamp {
    pub id: String,
    pub name: String,
    pub text: String,
    /// Epoch milliseconds.
    pub created_at: i64,
    pub updated_at: i64,
}

/// What `stamps_upsert` takes: `id` absent creates, present edits (design D3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StampInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub name: String,
    pub text: String,
}

/// The library's one free-text scratchpad (`notes` design D3). A library that
/// has never been written to reads as an empty note, never an error.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    pub content: String,
    pub updated_at: i64,
}

/// A booru this library posts to (`booru-upload` design D1, D2; `booru-sites`).
/// The API key is never a field of this type — it lives in the OS credential
/// store, keyed on `<host>/<username>` (design D7), never in `library.sqlite`.
///
/// `id` is a slug derived once from `base_url`'s host at creation (design D2)
/// and never changes afterwards, even across an edit to `base_url`: it is what
/// `PostRef.site` holds, and it must outlive the row it came from.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BooruSite {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub username: String,
    pub created_at: i64,
    pub updated_at: i64,
}

/// What `booru_site_test` answers with (design D8): judged by status code
/// alone, never by the response body — a self-hosted fork's own profile shape
/// must never fail a test a real Danbooru would pass.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum BooruConnectionTest {
    Connected,
    CredentialRejected,
    /// The transport's own reason, or the status the site answered with.
    Unreachable {
        reason: String,
    },
}

/// The upload form's values, prefilled per `booru-upload` design D10 and
/// editable before sending; edits never reach the image's own row. `rating`
/// is never empty here — the webview form refuses to send until one is
/// chosen (spec `booru-upload`, "An upload will not be sent without tags and
/// a rating"); [`crate::commands::booru_upload`] checks it again regardless,
/// the same defence-in-depth `rules::upsert` applies to its own inputs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BooruUploadForm {
    /// Space-joined into `tag_string` at the request builder, not here —
    /// `booru::client::BooruClient::create_post`.
    pub tags: Vec<String>,
    /// `g` | `s` | `q` | `e`, checked against [`crate::tags::RATINGS`].
    pub rating: String,
    /// The page the image was captured from, or the image's own address.
    pub source: String,
    /// Empty when no rule matched the page address (design D11); folded into
    /// `tag_string` alongside `tags` when present, since Danbooru has no
    /// separate artist field on `POST /posts.json` (design D3, D10).
    pub artist: String,
    pub commentary_title: String,
    pub commentary_body: String,
}

/// The step an upload sequence failed at (design D6). `Authenticate` is the
/// booru itself rejecting the credential — a 401/403 on any of the four
/// calls — never a local credential-store failure: that fails the command
/// outright as an [`AppError::Credential`] before any request is sent
/// (`booru-sites` design D7), so it never reaches this type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum UploadStep {
    Authenticate,
    CreateUpload,
    AwaitProcessing,
    CreatePost,
    Commentary,
}

/// Which step failed, the booru's own message where it gave one — never a
/// paraphrase (design D6) — and the remote reference a failure can point at:
/// the upload id when processing times out, the post id when only the
/// commentary fails.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadStepError {
    pub step: UploadStep,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote_ref: Option<String>,
}

/// Whether the optional artist commentary call ran, and how it went. Its own
/// failure is not the upload's failure (design D6): the post already exists,
/// so [`BooruUploadOutcome::Posted`] still returns with this as a warning
/// rather than turning the whole upload into a [`BooruUploadOutcome::Failed`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum CommentaryOutcome {
    /// No commentary title or body was given.
    Skipped,
    Applied,
    Failed {
        message: String,
    },
}

/// What `booru_upload` answers with. Only [`BooruUploadOutcome::Posted`] ever
/// carries a [`PostRef`] — nothing is recorded against the image for a
/// [`BooruUploadOutcome::Failed`], however far the sequence got before it
/// failed (design D5).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "camelCase")]
pub enum BooruUploadOutcome {
    Posted {
        post: PostRef,
        commentary: CommentaryOutcome,
    },
    Failed {
        error: UploadStepError,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Nothing checks this file against `packages/shared/src/index.ts`, so the
    /// wire form of the types added for the app shell is pinned here: a key or a
    /// spelling that drifts from the mirror is a webview reading `undefined`.
    #[test]
    fn app_settings_crosses_the_wire_in_camel_case() {
        let settings = AppSettings {
            theme: Theme::Dark,
            grid_tile_size: GRID_TILE_DEFAULT,
            show_tile_tags: true,
            click_zoom_ceiling_percent: CLICK_ZOOM_CEILING_DEFAULT,
            notes_collapsed: true,
            collections_collapsed: true,
            open_last_on_launch: false,
        };

        assert_eq!(
            serde_json::to_value(settings).unwrap(),
            serde_json::json!({
                "theme": "dark",
                "gridTileSize": 180,
                "showTileTags": true,
                "clickZoomCeilingPercent": 150,
                "notesCollapsed": true,
                "collectionsCollapsed": true,
                "openLastOnLaunch": false,
            }),
        );
    }

    /// `tag-vocabulary` task 1.2: the wire spelling a hand-mirrored rename
    /// would silently break — camelCase keys, and every category its own
    /// lower-case name in both directions.
    #[test]
    fn a_tag_entry_crosses_the_wire_in_camel_case_with_lowercase_categories() {
        let entry = TagEntry {
            name: "kantoku".to_string(),
            category: TagCategory::Artist,
            pinned: true,
        };
        assert_eq!(
            serde_json::to_value(&entry).unwrap(),
            serde_json::json!({ "name": "kantoku", "category": "artist", "pinned": true }),
        );

        for (category, name) in [
            (TagCategory::Artist, "artist"),
            (TagCategory::Copyright, "copyright"),
            (TagCategory::Character, "character"),
            (TagCategory::Meta, "meta"),
            (TagCategory::General, "general"),
        ] {
            assert_eq!(serde_json::to_value(category).unwrap(), name);
            assert_eq!(
                serde_json::from_value::<TagCategory>(serde_json::json!(name)).unwrap(),
                category,
            );
        }
    }

    /// `category-count-search` task 1.1: a term's wire shape — camelCase
    /// keys, and `category` serialised as `null` for `tagcount:` itself
    /// rather than an absent key (design D1).
    #[test]
    fn a_tag_count_term_crosses_the_wire_in_camel_case() {
        let every = TagCountTerm {
            category: None,
            filter: TagCountFilter {
                operator: TagCountOperator::Eq,
                value: Some(5),
                values: None,
                min: None,
                max: None,
            },
        };
        assert_eq!(
            serde_json::to_value(&every).unwrap(),
            serde_json::json!({
                "category": null,
                "filter": { "operator": "=", "value": 5 },
            }),
        );

        let artist = TagCountTerm {
            category: Some(TagCategory::Artist),
            filter: TagCountFilter {
                operator: TagCountOperator::Gt,
                value: Some(0),
                values: None,
                min: None,
                max: None,
            },
        };
        assert_eq!(
            serde_json::to_value(&artist).unwrap(),
            serde_json::json!({
                "category": "artist",
                "filter": { "operator": ">", "value": 0 },
            }),
        );
    }

    /// `stamps` task 1.1: `TagEditSpec`'s wire shape — camelCase keys, and
    /// `rating` absent from the JSON entirely when a stamp does not set one,
    /// the same convention `RuleInput.id` already follows for an absent field.
    #[test]
    fn a_tag_edit_spec_crosses_the_wire_in_camel_case() {
        let edit = TagEditSpec {
            add: vec!["cat".to_string(), "animal".to_string()],
            remove: vec!["dog".to_string()],
            add_collections: vec!["cute".to_string()],
            remove_collections: vec!["uncategorized".to_string()],
            rating: Some("g".to_string()),
        };

        assert_eq!(
            serde_json::to_value(&edit).unwrap(),
            serde_json::json!({
                "add": ["cat", "animal"],
                "remove": ["dog"],
                "addCollections": ["cute"],
                "removeCollections": ["uncategorized"],
                "rating": "g",
            }),
        );

        let tag_only = TagEditSpec {
            add: vec!["cat".to_string()],
            ..Default::default()
        };
        let json = serde_json::to_value(&tag_only).unwrap();
        assert!(json.get("rating").is_none(), "{json}");
        assert_eq!(
            serde_json::from_value::<TagEditSpec>(json).unwrap(),
            tag_only
        );
    }

    fn bare_image(id: &str) -> ImageRecord {
        ImageRecord {
            id: id.to_string(),
            ext: "png".to_string(),
            file: crate::library::LibraryPaths::relative_image_path(id, "png"),
            mime: "image/png".to_string(),
            size: 1,
            width: 1,
            height: 1,
            source: ImageSource::Local,
            source_ref: None,
            image_url: None,
            page_url: None,
            page_title: None,
            account: None,
            adapter: None,
            rating: None,
            tags: Vec::new(),
            captured_at: 0,
            file_modified_at: None,
            created_at: 0,
            updated_at: 0,
            deleted_at: None,
            missing: false,
            posts: Vec::new(),
            collections: Vec::new(),
        }
    }

    /// Design D11: `fileModifiedAt` crosses the wire in camelCase and `null`
    /// round-trips, so an image with no file behind it (every extension
    /// capture) reads back as absent rather than 0 or an error.
    #[test]
    fn file_modified_at_is_camel_case_and_null_round_trips() {
        let without_file = bare_image("a");
        let json = serde_json::to_value(&without_file).unwrap();
        assert_eq!(json["fileModifiedAt"], serde_json::Value::Null);
        assert_eq!(
            serde_json::from_value::<ImageRecord>(json).unwrap(),
            without_file
        );

        let with_file = ImageRecord {
            file_modified_at: Some(1_600_000_000_000),
            ..bare_image("b")
        };
        let json = serde_json::to_value(&with_file).unwrap();
        assert_eq!(json["fileModifiedAt"], 1_600_000_000_000_i64);
        assert_eq!(
            serde_json::from_value::<ImageRecord>(json).unwrap(),
            with_file
        );
    }

    #[test]
    fn every_theme_is_its_lowercase_name_in_both_directions() {
        for (theme, name) in [
            (Theme::System, "system"),
            (Theme::Light, "light"),
            (Theme::Dark, "dark"),
        ] {
            assert_eq!(serde_json::to_value(theme).unwrap(), name);
            assert_eq!(
                serde_json::from_value::<Theme>(serde_json::json!(name)).unwrap(),
                theme,
            );
        }
    }

    #[test]
    fn a_withdrawal_omits_the_reason_key_when_there_is_none() {
        let with_reason = CaptureWithdrawn {
            id: "c-1".to_string(),
            reason: Some("image could not be decoded".to_string()),
        };
        let without = CaptureWithdrawn {
            id: "c-1".to_string(),
            reason: None,
        };

        assert_eq!(
            serde_json::to_value(with_reason).unwrap(),
            serde_json::json!({ "id": "c-1", "reason": "image could not be decoded" }),
        );
        // Not `"reason": null`: the webview tells "no reason given" from a
        // reason by the key being absent, and a null would read as one.
        assert_eq!(
            serde_json::to_value(without).unwrap(),
            serde_json::json!({ "id": "c-1" }),
        );
    }

    /// `x-account` is the one spelling in this file that a `rename_all` could
    /// silently get wrong (`xAccount`, `x_account`), and the webview matches on
    /// the string. Pinned in both directions with the rest of the view types.
    #[test]
    fn the_view_types_cross_the_wire_in_the_spellings_the_webview_matches_on() {
        for (group, name) in [
            (GroupBy::None, "none"),
            (GroupBy::XAccount, "x-account"),
            (GroupBy::Duplicates, "duplicates"),
        ] {
            assert_eq!(serde_json::to_value(group).unwrap(), name);
            assert_eq!(
                serde_json::from_value::<GroupBy>(serde_json::json!(name)).unwrap(),
                group,
            );
        }

        for (field, name) in [
            (SortField::Captured, "captured"),
            (SortField::Updated, "updated"),
            (SortField::Size, "size"),
            (SortField::Dimensions, "dimensions"),
        ] {
            assert_eq!(serde_json::to_value(field).unwrap(), name);
            assert_eq!(
                serde_json::from_value::<SortField>(serde_json::json!(name)).unwrap(),
                field,
            );
        }

        assert_eq!(
            serde_json::to_value(Sort::default()).unwrap(),
            serde_json::json!({ "field": "captured", "direction": "desc" }),
        );
    }

    /// `trash` design D2: the webview matches on these two literal strings, so
    /// a `rename_all` that quietly changed one would leave `search` filtering
    /// on nothing the request meant.
    #[test]
    fn a_search_view_crosses_the_wire_as_library_or_trash() {
        for (view, name) in [
            (SearchView::Library, "library"),
            (SearchView::Trash, "trash"),
        ] {
            assert_eq!(serde_json::to_value(view).unwrap(), name);
            assert_eq!(
                serde_json::from_value::<SearchView>(serde_json::json!(name)).unwrap(),
                view,
            );
        }
    }

    /// `library-sidecars` task 2.6: the pinned contract's exact field names,
    /// since this is the hand-mirrored half of `packages/shared` a rename here
    /// would silently break — `keptAs` above all, the one name a `rename_all`
    /// could not derive from `kept_as` by accident-proofing alone.
    #[test]
    fn a_rebuild_report_and_its_progress_cross_the_wire_in_camel_case() {
        let report = RebuildReport {
            images: 24_998,
            failed: 2,
            failures: vec![RebuildFailure {
                file: "images/ab/c/abc.json".to_string(),
                reason: "sidecar at images/ab/c/abc.json cannot be read: EOF".to_string(),
            }],
            kept_as: "library.sqlite.corrupt-1757000000000".to_string(),
            rules: 3,
            sites: 1,
            collections: 4,
        };

        assert_eq!(
            serde_json::to_value(&report).unwrap(),
            serde_json::json!({
                "images": 24_998,
                "failed": 2,
                "failures": [{
                    "file": "images/ab/c/abc.json",
                    "reason": "sidecar at images/ab/c/abc.json cannot be read: EOF",
                }],
                "keptAs": "library.sqlite.corrupt-1757000000000",
                "rules": 3,
                "sites": 1,
                "collections": 4,
            }),
        );
        assert_eq!(
            serde_json::from_value::<RebuildReport>(serde_json::to_value(&report).unwrap())
                .unwrap(),
            report
        );

        let rebuild_progress = RebuildProgress { done: 3, total: 20 };
        assert_eq!(
            serde_json::to_value(rebuild_progress).unwrap(),
            serde_json::json!({ "done": 3, "total": 20 }),
        );
        let sidecars_progress = SidecarsProgress { done: 3, total: 20 };
        assert_eq!(
            serde_json::to_value(sidecars_progress).unwrap(),
            serde_json::json!({ "done": 3, "total": 20 }),
        );
    }

    /// `library-sidecars` design D9: `damagedPath` and `newerPath` are two
    /// further independent slots beside `missingPath` — a status naming none
    /// of them, or exactly one, must be representable, since the start screen
    /// keys its wording on which one is set.
    #[test]
    fn a_library_status_carries_damaged_and_newer_paths_alongside_missing_path() {
        let none_of_them = LibraryStatus::default();
        assert_eq!(none_of_them.missing_path, None);
        assert_eq!(none_of_them.damaged_path, None);
        assert_eq!(none_of_them.newer_path, None);

        let damaged = LibraryStatus {
            damaged_path: Some("/libraries/art".to_string()),
            ..LibraryStatus::default()
        };
        let json = serde_json::to_value(&damaged).unwrap();
        assert_eq!(json["damagedPath"], "/libraries/art");
        assert_eq!(json["missingPath"], serde_json::Value::Null);
        assert_eq!(json["newerPath"], serde_json::Value::Null);

        let newer = LibraryStatus {
            newer_path: Some("/libraries/art".to_string()),
            ..LibraryStatus::default()
        };
        let json = serde_json::to_value(&newer).unwrap();
        assert_eq!(json["newerPath"], "/libraries/art");
        assert_eq!(json["missingPath"], serde_json::Value::Null);
        assert_eq!(json["damagedPath"], serde_json::Value::Null);
    }

    #[test]
    fn a_delete_report_crosses_the_wire_in_camel_case() {
        let report = DeleteReport {
            deleted: 3,
            files_left: vec!["images/a.png".to_string()],
        };

        assert_eq!(
            serde_json::to_value(report).unwrap(),
            serde_json::json!({ "deleted": 3, "filesLeft": ["images/a.png"] }),
        );
    }

    #[test]
    fn a_search_carries_its_sort_and_group_and_answers_with_slices() {
        let req = SearchRequest {
            // `any_collection` set and `no_collection` left default: the
            // round-trip below is what pins `category-count-search` design
            // D4's wire keys, camelCase and present even when false.
            query: ParsedTagSearch {
                any_collection: true,
                ..Default::default()
            },
            text: String::new(),
            view: SearchView::Library,
            sort: Sort {
                field: SortField::Size,
                direction: SortDirection::Asc,
            },
            group: GroupBy::XAccount,
            limit: 200,
            offset: 0,
        };

        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(
            json["sort"],
            serde_json::json!({ "field": "size", "direction": "asc" })
        );
        assert_eq!(json["group"], "x-account");
        assert_eq!(json["view"], "library");
        assert_eq!(json["query"]["anyCollection"], true);
        assert_eq!(json["query"]["noCollection"], false);
        assert_eq!(
            serde_json::from_value::<SearchRequest>(json).unwrap(),
            req,
            "the request must round-trip through the wire form the webview sends",
        );

        let result = SearchResult {
            images: Vec::new(),
            total: 3,
            groups: vec![GroupSlice {
                key: "alice".to_string(),
                count: 3,
            }],
        };
        assert_eq!(
            serde_json::to_value(result).unwrap(),
            serde_json::json!({
                "images": [],
                "total": 3,
                "groups": [{ "key": "alice", "count": 3 }],
            }),
        );
    }

    #[test]
    fn the_sidebar_counts_cross_the_wire_in_camel_case() {
        let counts = TagCounts {
            tags: vec![TagCount {
                name: "cat".to_string(),
                count: 12,
            }],
            ratings: RatingCounts {
                g: 1,
                s: 2,
                q: 3,
                e: 4,
                unrated: 5,
            },
            collections: vec![CollectionCount {
                id: "favorites".to_string(),
                name: "Favorites".to_string(),
                slug: "favorites".to_string(),
                count: 2,
            }],
            accounts: vec![GroupSlice {
                key: "alice".to_string(),
                count: 3,
            }],
        };

        assert_eq!(
            serde_json::to_value(counts).unwrap(),
            serde_json::json!({
                "tags": [{ "name": "cat", "count": 12 }],
                "ratings": { "g": 1, "s": 2, "q": 3, "e": 4, "unrated": 5 },
                "collections": [{
                    "id": "favorites",
                    "name": "Favorites",
                    "slug": "favorites",
                    "count": 2,
                }],
                "accounts": [{ "key": "alice", "count": 3 }],
            }),
        );
    }

    #[test]
    fn the_export_report_and_progress_cross_the_wire_in_camel_case() {
        let report = ExportReport {
            path: "/libraries/art/export.zip".to_string(),
            written: 19,
            missing: vec!["gone-1".to_string()],
        };
        assert_eq!(
            serde_json::to_value(report).unwrap(),
            serde_json::json!({
                "path": "/libraries/art/export.zip",
                "written": 19,
                "missing": ["gone-1"],
            }),
        );

        let progress = ExportProgress { done: 3, total: 20 };
        assert_eq!(
            serde_json::to_value(progress).unwrap(),
            serde_json::json!({ "done": 3, "total": 20 }),
        );
    }

    #[test]
    fn a_recent_library_carries_its_path_name_and_availability() {
        let entry = RecentLibrary {
            path: "/libraries/art".to_string(),
            name: "art".to_string(),
            available: false,
        };

        assert_eq!(
            serde_json::to_value(entry).unwrap(),
            serde_json::json!({ "path": "/libraries/art", "name": "art", "available": false }),
        );
    }

    /// `auto-tag-rules` task 1.1: nothing checks this file against
    /// `packages/shared/src/index.ts`, so the wire spellings a hand-mirrored
    /// rename would silently break are pinned here — `isRegex`,
    /// `patternError` and `createdAt` above all.
    #[test]
    fn a_rule_and_its_list_entry_cross_the_wire_in_camel_case() {
        let rule = Rule {
            id: "r-1".to_string(),
            name: "pixiv".to_string(),
            pattern: "pixiv".to_string(),
            is_regex: false,
            tags: vec!["pixiv".to_string()],
            enabled: true,
            created_at: 1_700_000_000_000,
            updated_at: 1_700_000_000_001,
        };

        assert_eq!(
            serde_json::to_value(&rule).unwrap(),
            serde_json::json!({
                "id": "r-1",
                "name": "pixiv",
                "pattern": "pixiv",
                "isRegex": false,
                "tags": ["pixiv"],
                "enabled": true,
                "createdAt": 1_700_000_000_000i64,
                "updatedAt": 1_700_000_000_001i64,
            }),
        );

        let entry = RuleListEntry {
            rule: rule.clone(),
            pattern_error: Some("unclosed group".to_string()),
        };
        let json = serde_json::to_value(&entry).unwrap();
        assert_eq!(json["patternError"], "unclosed group");
        assert_eq!(
            serde_json::from_value::<RuleListEntry>(json).unwrap(),
            entry
        );
    }

    #[test]
    fn a_rule_input_omits_the_id_when_creating() {
        let creating = RuleInput {
            id: None,
            name: "pixiv".to_string(),
            pattern: "pixiv".to_string(),
            is_regex: false,
            tags: vec!["pixiv".to_string()],
            enabled: true,
        };

        assert_eq!(
            serde_json::to_value(&creating).unwrap(),
            serde_json::json!({
                "name": "pixiv",
                "pattern": "pixiv",
                "isRegex": false,
                "tags": ["pixiv"],
                "enabled": true,
            }),
        );

        let editing = RuleInput {
            id: Some("r-1".to_string()),
            ..creating
        };
        assert_eq!(
            serde_json::to_value(&editing).unwrap()["id"],
            "r-1".to_string()
        );
    }

    /// `stamps` task 1.2: the same drift risk `a_rule_and_its_list_entry_
    /// cross_the_wire_in_camel_case` pins for `Rule` — camelCase keys, and
    /// `StampInput.id` absent from the JSON entirely when creating.
    #[test]
    fn a_stamp_and_its_input_cross_the_wire_in_camel_case() {
        let stamp = Stamp {
            id: "s-1".to_string(),
            name: "Cat".to_string(),
            text: "cat animal".to_string(),
            created_at: 1_700_000_000_000,
            updated_at: 1_700_000_000_001,
        };

        assert_eq!(
            serde_json::to_value(&stamp).unwrap(),
            serde_json::json!({
                "id": "s-1",
                "name": "Cat",
                "text": "cat animal",
                "createdAt": 1_700_000_000_000i64,
                "updatedAt": 1_700_000_000_001i64,
            }),
        );

        let creating = StampInput {
            id: None,
            name: "Cat".to_string(),
            text: "cat animal".to_string(),
        };
        assert_eq!(
            serde_json::to_value(&creating).unwrap(),
            serde_json::json!({ "name": "Cat", "text": "cat animal" }),
        );

        let editing = StampInput {
            id: Some("s-1".to_string()),
            ..creating
        };
        assert_eq!(
            serde_json::to_value(&editing).unwrap()["id"],
            "s-1".to_string()
        );
    }

    #[test]
    fn a_rules_run_report_names_its_invalid_rules_separately() {
        let report = RulesRunReport {
            examined: 400,
            changed: 12,
            rules: vec![RuleRunCount {
                id: "r-1".to_string(),
                name: "pixiv".to_string(),
                matched: 12,
                pattern_error: None,
            }],
            invalid: vec![RuleRunCount {
                id: "r-2".to_string(),
                name: "broken".to_string(),
                matched: 0,
                pattern_error: Some("unclosed group".to_string()),
            }],
        };

        let json = serde_json::to_value(&report).unwrap();
        assert_eq!(json["rules"][0]["matched"], 12);
        assert!(json["rules"][0].get("patternError").is_none());
        assert_eq!(json["invalid"][0]["patternError"], "unclosed group");
        assert_eq!(
            serde_json::from_value::<RulesRunReport>(json).unwrap(),
            report
        );
    }

    #[test]
    fn a_note_crosses_the_wire_in_camel_case() {
        let note = Note {
            content: "remember to tag these".to_string(),
            updated_at: 1_700_000_000_000,
        };

        assert_eq!(
            serde_json::to_value(&note).unwrap(),
            serde_json::json!({
                "content": "remember to tag these",
                "updatedAt": 1_700_000_000_000i64,
            }),
        );
    }

    /// `booru-upload` task 1.1: a `PostRef` crosses the wire in camel case, and
    /// an `ImageRecord` with no posts carries an empty array rather than
    /// omitting the key — the webview's `posted-label` reads `image.posts`
    /// unconditionally.
    #[test]
    fn a_post_ref_crosses_the_wire_in_camel_case_and_an_image_defaults_to_no_posts() {
        let post = PostRef {
            site: "danbooru".to_string(),
            remote_id: "12345".to_string(),
            posted_at: 1_700_000_000_000,
        };
        assert_eq!(
            serde_json::to_value(&post).unwrap(),
            serde_json::json!({
                "site": "danbooru",
                "remoteId": "12345",
                "postedAt": 1_700_000_000_000i64,
            }),
        );

        let image = bare_image("a");
        assert_eq!(
            serde_json::to_value(&image).unwrap()["posts"],
            serde_json::json!([])
        );

        let posted = ImageRecord {
            posts: vec![post.clone()],
            ..image
        };
        assert_eq!(
            serde_json::to_value(&posted).unwrap()["posts"],
            serde_json::json!([{ "site": "danbooru", "remoteId": "12345", "postedAt": 1_700_000_000_000i64 }]),
        );
    }

    #[test]
    fn a_booru_site_crosses_the_wire_in_camel_case_with_no_credential_field() {
        let site = BooruSite {
            id: "danbooru-donmai-us".to_string(),
            name: "Danbooru".to_string(),
            base_url: "https://danbooru.donmai.us".to_string(),
            username: "alice".to_string(),
            created_at: 1_700_000_000_000,
            updated_at: 1_700_000_000_001,
        };

        let json = serde_json::to_value(&site).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "id": "danbooru-donmai-us",
                "name": "Danbooru",
                "baseUrl": "https://danbooru.donmai.us",
                "username": "alice",
                "createdAt": 1_700_000_000_000i64,
                "updatedAt": 1_700_000_000_001i64,
            }),
        );
        // The whole point of design D7: nothing here ever names the API key.
        assert!(json.get("apiKey").is_none());
        assert!(json.get("api_key").is_none());
    }

    /// `booru-sites` design D8: the webview matches on these three tags.
    #[test]
    fn a_connection_test_crosses_the_wire_as_a_tagged_status() {
        assert_eq!(
            serde_json::to_value(BooruConnectionTest::Connected).unwrap(),
            serde_json::json!({ "status": "connected" }),
        );
        assert_eq!(
            serde_json::to_value(BooruConnectionTest::CredentialRejected).unwrap(),
            serde_json::json!({ "status": "credentialRejected" }),
        );
        assert_eq!(
            serde_json::to_value(BooruConnectionTest::Unreachable {
                reason: "connection refused".to_string()
            })
            .unwrap(),
            serde_json::json!({ "status": "unreachable", "reason": "connection refused" }),
        );
    }

    #[test]
    fn an_upload_form_crosses_the_wire_in_camel_case() {
        let form = BooruUploadForm {
            tags: vec!["1girl".to_string(), "blue_sky".to_string()],
            rating: "s".to_string(),
            source: "https://example.test/p".to_string(),
            artist: "pixiv_user_1".to_string(),
            commentary_title: "a title".to_string(),
            commentary_body: String::new(),
        };

        assert_eq!(
            serde_json::to_value(&form).unwrap(),
            serde_json::json!({
                "tags": ["1girl", "blue_sky"],
                "rating": "s",
                "source": "https://example.test/p",
                "artist": "pixiv_user_1",
                "commentaryTitle": "a title",
                "commentaryBody": "",
            }),
        );
    }

    /// `booru-upload` design D6: the webview matches on these five step names,
    /// and `remoteRef` is absent — not `null` — when there is none, the same
    /// rule `CaptureWithdrawn.reason` follows.
    #[test]
    fn an_upload_step_error_names_its_step_and_omits_a_missing_remote_ref() {
        for (step, name) in [
            (UploadStep::Authenticate, "authenticate"),
            (UploadStep::CreateUpload, "createUpload"),
            (UploadStep::AwaitProcessing, "awaitProcessing"),
            (UploadStep::CreatePost, "createPost"),
            (UploadStep::Commentary, "commentary"),
        ] {
            assert_eq!(serde_json::to_value(step).unwrap(), name);
        }

        let without_ref = UploadStepError {
            step: UploadStep::CreateUpload,
            message: "duplicate of post #123".to_string(),
            remote_ref: None,
        };
        assert_eq!(
            serde_json::to_value(&without_ref).unwrap(),
            serde_json::json!({ "step": "createUpload", "message": "duplicate of post #123" }),
        );

        let with_ref = UploadStepError {
            remote_ref: Some("upload-9".to_string()),
            ..without_ref
        };
        assert_eq!(
            serde_json::to_value(&with_ref).unwrap()["remoteRef"],
            "upload-9",
        );
    }

    #[test]
    fn a_commentary_outcome_crosses_the_wire_as_a_tagged_status() {
        assert_eq!(
            serde_json::to_value(CommentaryOutcome::Skipped).unwrap(),
            serde_json::json!({ "status": "skipped" }),
        );
        assert_eq!(
            serde_json::to_value(CommentaryOutcome::Applied).unwrap(),
            serde_json::json!({ "status": "applied" }),
        );
        assert_eq!(
            serde_json::to_value(CommentaryOutcome::Failed {
                message: "title too long".to_string()
            })
            .unwrap(),
            serde_json::json!({ "status": "failed", "message": "title too long" }),
        );
    }

    /// `booru-upload` design D5, D6: only `Posted` ever carries a `PostRef`,
    /// and the webview tells the two outcomes apart by `outcome`.
    #[test]
    fn an_upload_outcome_crosses_the_wire_as_a_tagged_union() {
        let posted = BooruUploadOutcome::Posted {
            post: PostRef {
                site: "danbooru".to_string(),
                remote_id: "9".to_string(),
                posted_at: 1_700_000_000_000,
            },
            commentary: CommentaryOutcome::Applied,
        };
        assert_eq!(
            serde_json::to_value(&posted).unwrap(),
            serde_json::json!({
                "outcome": "posted",
                "post": { "site": "danbooru", "remoteId": "9", "postedAt": 1_700_000_000_000i64 },
                "commentary": { "status": "applied" },
            }),
        );

        let failed = BooruUploadOutcome::Failed {
            error: UploadStepError {
                step: UploadStep::AwaitProcessing,
                message: "timed out".to_string(),
                remote_ref: Some("upload-9".to_string()),
            },
        };
        let json = serde_json::to_value(&failed).unwrap();
        assert_eq!(json["outcome"], "failed");
        assert_eq!(json["error"]["step"], "awaitProcessing");
        assert_eq!(json["error"]["remoteRef"], "upload-9");
    }
}
