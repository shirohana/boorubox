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

/// One row of the library, as the webview sees it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageRecord {
    pub id: String,
    pub ext: String,
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
    pub created_at: i64,
    pub updated_at: i64,
    pub deleted_at: Option<i64>,
    /// The file under `images/` was not there at the last check.
    pub missing: bool,
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
    pub tag_count: Option<TagCountFilter>,
    pub include_unrated: bool,
    pub accounts: Vec<String>,
    pub exclude_accounts: Vec<String>,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchRequest {
    pub query: ParsedTagSearch,
    /// Free text matched against page title and URLs through FTS5 (design D14).
    pub text: String,
    pub include_deleted: bool,
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
    pub tags: Vec<TagCount>,
    pub ratings: RatingCounts,
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
    pub library_path: Option<String>,
    /// A remembered path that could not be opened; kept until another is picked.
    pub missing_path: Option<String>,
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
    /// New image id when `imported`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Why, when `skipped` or `failed`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportReport {
    pub imported: u32,
    pub skipped: u32,
    pub failed: u32,
    pub items: Vec<ImportOutcome>,
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
        };

        assert_eq!(
            serde_json::to_value(settings).unwrap(),
            serde_json::json!({ "theme": "dark", "gridTileSize": 180 }),
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

    #[test]
    fn a_search_carries_its_sort_and_group_and_answers_with_slices() {
        let req = SearchRequest {
            query: ParsedTagSearch::default(),
            text: String::new(),
            include_deleted: false,
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
        assert_eq!(json["includeDeleted"], false);
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
        };

        assert_eq!(
            serde_json::to_value(counts).unwrap(),
            serde_json::json!({
                "tags": [{ "name": "cat", "count": 12 }],
                "ratings": { "g": 1, "s": 2, "q": 3, "e": 4, "unrated": 5 },
            }),
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
}
