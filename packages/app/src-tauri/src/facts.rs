//! The one write for an image's editable facts — title, page address, image
//! address (`editable-info` design D1). Everything else about an image (tags,
//! rating) is `tags.rs`'s; this module only ever touches these three columns.

use rusqlite::params;

use crate::error::{AppError, Result};
use crate::ingest;
use crate::library::Library;
use crate::model::{FactsEdit, ImageRecord};
use crate::tags;

/// Validate `edit`, write the three facts, mark the image as changed, rewrite
/// its sidecar, and answer with the row as it now stands.
///
/// Addresses are validated before the transaction opens, so a bad one never
/// touches the database. Inside it, `tags::mark_updated` runs first — the
/// same shape `tags::update_tags` uses — so an id with no row refuses the
/// whole edit before the three-column update runs; the transaction rolls back
/// on the way out either way.
pub fn update(library: &Library, id: &str, edit: &FactsEdit) -> Result<ImageRecord> {
    let page_title = clean(edit.page_title.as_deref());
    let page_url = validated_address(edit.page_url.as_deref())?;
    let image_url = validated_address(edit.image_url.as_deref())?;

    let tx = library.conn.unchecked_transaction()?;
    tags::mark_updated(&tx, id, None)?;
    tx.execute(
        "UPDATE images SET page_title = ?1, page_url = ?2, image_url = ?3 WHERE id = ?4",
        params![page_title, page_url, image_url, id],
    )?;
    tx.commit()?;

    // After the commit, never inside it (`library-sidecars` design D4).
    crate::sidecar::write_one(&library.paths, &library.conn, id)?;
    ingest::require_record(&library.conn, id)
}

/// `None` and an empty string after trimming both mean "clear this field" —
/// the form has no way to tell the two apart once a user has emptied a field.
fn clean(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

/// [`clean`], then: a non-empty address must be `http://` or `https://` with
/// something after the scheme, or the save is refused before anything is
/// written. No `url` crate — this app has no other need of one, and the
/// scheme check is the whole rule the spec asks for.
fn validated_address(value: Option<&str>) -> Result<Option<String>> {
    let Some(address) = clean(value) else {
        return Ok(None);
    };
    let after_scheme = address
        .strip_prefix("https://")
        .or_else(|| address.strip_prefix("http://"));
    match after_scheme {
        Some(rest) if !rest.is_empty() => Ok(Some(address)),
        _ => Err(AppError::BadRequest(format!(
            "{address:?} is not a web address"
        ))),
    }
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

    fn library() -> (tempfile::TempDir, Library) {
        let dir = tempfile::tempdir().unwrap();
        let library = Library::open_or_create(dir.path()).unwrap();
        (dir, library)
    }

    /// Rows enter through `ingest::store_image`, the one write path (design
    /// D4), so a fixture cannot drift from what the app actually stores. A
    /// bare import: no page title, no page address, no image address — the
    /// local-import case the proposal names.
    fn store(library: &Library, id: &str) {
        store_image(
            library,
            IngestInput {
                id,
                bytes: &png_bytes(),
                source: ImageSource::Local,
                source_ref: None,
                image_url: None,
                page_url: None,
                page_title: None,
                adapter: None,
                rating: None,
                tags: &[],
                captured_at: 1_700_000_000_000,
                file_modified_at: None,
                deleted_at: None,
            },
        )
        .unwrap();
    }

    fn edit(
        page_title: Option<&str>,
        page_url: Option<&str>,
        image_url: Option<&str>,
    ) -> FactsEdit {
        FactsEdit {
            page_title: page_title.map(str::to_string),
            page_url: page_url.map(str::to_string),
            image_url: image_url.map(str::to_string),
        }
    }

    #[test]
    fn a_title_and_two_addresses_are_stored_and_answered_back_trimmed() {
        let (_dir, library) = library();
        store(&library, "a");

        let record = update(
            &library,
            "a",
            &edit(
                Some("  a title  "),
                Some("  https://x.com/alice/status/9  "),
                Some("  https://x.com/alice/image.png  "),
            ),
        )
        .unwrap();

        assert_eq!(record.page_title.as_deref(), Some("a title"));
        assert_eq!(
            record.page_url.as_deref(),
            Some("https://x.com/alice/status/9")
        );
        assert_eq!(
            record.image_url.as_deref(),
            Some("https://x.com/alice/image.png")
        );
        assert_eq!(
            record.account.as_deref(),
            Some("alice"),
            "account is derived from page_url at load, for free (inspector-polish)",
        );
    }

    #[test]
    fn empty_strings_clear_all_three_facts() {
        let (_dir, library) = library();
        store(&library, "a");
        update(
            &library,
            "a",
            &edit(
                Some("a title"),
                Some("https://x.com/alice/status/9"),
                Some("https://x.com/alice/image.png"),
            ),
        )
        .unwrap();

        let record = update(&library, "a", &edit(Some(""), Some("   "), Some(""))).unwrap();

        assert_eq!(record.page_title, None);
        assert_eq!(record.page_url, None);
        assert_eq!(record.image_url, None);
        assert_eq!(record.account, None, "no page address, no account entry");
    }

    #[test]
    fn a_value_that_is_not_a_url_is_refused_and_nothing_changes() {
        let (_dir, library) = library();
        store(&library, "a");
        let before = ingest::require_record(&library.conn, "a").unwrap();

        let error = update(&library, "a", &edit(None, Some("not a url"), None)).unwrap_err();

        assert!(matches!(error, AppError::BadRequest(_)), "got {error}");
        let after = ingest::require_record(&library.conn, "a").unwrap();
        assert_eq!(after.page_url, None);
        assert_eq!(
            after.updated_at, before.updated_at,
            "a refused edit must not mark the row"
        );
    }

    #[test]
    fn a_scheme_with_nothing_after_it_is_refused() {
        let (_dir, library) = library();
        store(&library, "a");

        let error = update(&library, "a", &edit(None, Some("https://"), None)).unwrap_err();

        assert!(matches!(error, AppError::BadRequest(_)), "got {error}");
    }

    #[test]
    fn an_edit_for_an_image_that_is_not_there_is_refused() {
        let (_dir, library) = library();

        let error = update(&library, "nobody", &edit(Some("title"), None, None)).unwrap_err();

        assert!(matches!(error, AppError::NotFound(_)), "got {error}");
    }

    #[test]
    fn an_edit_marks_the_image_as_changed() {
        let (_dir, library) = library();
        store(&library, "a");
        let before = ingest::require_record(&library.conn, "a").unwrap();

        let record = update(&library, "a", &edit(Some("a title"), None, None)).unwrap();

        assert!(
            record.updated_at > before.updated_at,
            "sort-by-last-change reads this column (design D9)",
        );
    }

    #[test]
    fn an_edit_rewrites_the_sidecar_with_the_new_facts() {
        let (_dir, library) = library();
        store(&library, "a");

        update(
            &library,
            "a",
            &edit(Some("a title"), Some("https://x.com/alice/status/9"), None),
        )
        .unwrap();

        let sidecar = crate::sidecar::read(&crate::sidecar::path(&library.paths, "a")).unwrap();
        assert_eq!(sidecar.page_title.as_deref(), Some("a title"));
        assert_eq!(
            sidecar.page_url.as_deref(),
            Some("https://x.com/alice/status/9")
        );
    }

    #[test]
    fn a_free_text_search_for_the_new_title_finds_the_image() {
        let (_dir, library) = library();
        store(&library, "a");
        store(&library, "b");

        update(
            &library,
            "a",
            &edit(Some("a very particular title"), None, None),
        )
        .unwrap();

        let found = query::search(
            &library.conn,
            &SearchRequest {
                query: ParsedTagSearch::default(),
                text: "particular".to_string(),
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
        .collect::<Vec<_>>();

        assert_eq!(found, vec!["a".to_string()]);
    }
}
