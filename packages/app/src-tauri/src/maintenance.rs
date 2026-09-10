//! Per-source counts and the missing-file pass that marks and clears
//! `images.missing`.

use rusqlite::{params, params_from_iter};

use crate::error::Result;
use crate::library::Library;
use crate::model::{ImageCounts, ImageSource};
use crate::query::placeholders;

/// Total plus one count per source, from one `GROUP BY source` (design D2).
///
/// `total` is the sum of the groups, which makes it the non-deleted count
/// `Library::image_count` reports. Counting it separately would let the two
/// numbers disagree on the same screen.
pub fn image_counts(library: &Library) -> Result<ImageCounts> {
    let mut stmt = library
        .conn
        .prepare("SELECT source, COUNT(*) FROM images WHERE deleted_at IS NULL GROUP BY source")?;
    let mut rows = stmt.query([])?;

    let mut counts = ImageCounts::default();
    while let Some(row) = rows.next()? {
        let source: ImageSource = row.get(0)?;
        let count: i64 = row.get(1)?;
        match source {
            ImageSource::Extension => counts.extension = count,
            ImageSource::Local => counts.local = count,
            ImageSource::LegacyBundle => counts.legacy_bundle = count,
        }
        counts.total += count;
    }
    Ok(counts)
}

/// Re-check the files behind `ids` and set or clear `missing` accordingly, so a
/// file that reappears stops showing as missing (spec `library-folder`).
///
/// `ids` is the page of results about to be shown, never the whole library.
/// This stats one file per id, and a ten-thousand-image grid would otherwise
/// pay ten thousand stat calls on every keystroke in the search box. Widen what
/// the caller passes and that cost comes straight back.
pub fn refresh_missing_for(library: &Library, ids: &[String]) -> Result<()> {
    let changed = images_whose_file_state_changed(library, ids)?;
    if changed.is_empty() {
        return Ok(());
    }

    // One transaction: a half-applied pass would leave the grid disagreeing
    // with the disk about some rows and not others.
    let tx = library.conn.unchecked_transaction()?;
    {
        let mut stmt = tx.prepare("UPDATE images SET missing = ?1 WHERE id = ?2")?;
        for (id, missing) in &changed {
            stmt.execute(params![missing, id])?;
        }
    }
    tx.commit()?;
    Ok(())
}

/// The ids whose file presence no longer matches their stored `missing` flag,
/// paired with the new value. Rows that already agree are left out so the pass
/// writes nothing when nothing moved on disk.
fn images_whose_file_state_changed(
    library: &Library,
    ids: &[String],
) -> Result<Vec<(String, bool)>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let mut stmt = library.conn.prepare(&format!(
        "SELECT id, ext, missing FROM images WHERE id IN ({})",
        placeholders(ids.len())
    ))?;
    let rows = stmt.query_map(params_from_iter(ids), |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, bool>(2)?,
        ))
    })?;

    let mut changed = Vec::new();
    for row in rows {
        let (id, ext, was_missing) = row?;
        let is_missing = !library.paths.image_path(&id, &ext).exists();
        if is_missing != was_missing {
            changed.push((id, is_missing));
        }
    }
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingest::{IngestInput, store_image};
    use crate::query;

    fn png_bytes() -> Vec<u8> {
        let image = image::DynamicImage::ImageRgb8(image::RgbImage::new(4, 4));
        let mut out = std::io::Cursor::new(Vec::new());
        image.write_to(&mut out, image::ImageFormat::Png).unwrap();
        out.into_inner()
    }

    /// Rows enter through `ingest::store_image`, the one write path (design
    /// D4), so the fixture cannot drift from what the app actually stores.
    fn store(library: &Library, id: &str, source: ImageSource, tags: &[&str]) {
        let tags: Vec<String> = tags.iter().map(|tag| (*tag).to_string()).collect();
        store_image(
            library,
            IngestInput {
                id,
                bytes: &png_bytes(),
                source,
                source_ref: None,
                image_url: None,
                page_url: Some("https://x.com/alice/status/1"),
                page_title: Some("sunset over kyoto"),
                adapter: None,
                rating: Some("s"),
                tags: &tags,
                captured_at: 1_700_000_000_000,
                file_modified_at: None,
            },
        )
        .unwrap();
    }

    fn library() -> (tempfile::TempDir, Library) {
        let dir = tempfile::tempdir().unwrap();
        let library = Library::open_or_create(dir.path()).unwrap();
        (dir, library)
    }

    fn missing_flag(library: &Library, id: &str) -> bool {
        library
            .conn
            .query_row("SELECT missing FROM images WHERE id = ?1", [id], |row| {
                row.get(0)
            })
            .unwrap()
    }

    #[test]
    fn counts_every_source_and_leaves_deleted_rows_out() {
        let (_dir, library) = library();
        for id in ["e1", "e2", "e3"] {
            store(&library, id, ImageSource::Extension, &[]);
        }
        store(&library, "l1", ImageSource::Local, &[]);
        store(&library, "l2", ImageSource::Local, &[]);
        store(&library, "gone", ImageSource::Extension, &[]);
        query::mark_deleted(&library.conn, "gone", 1);

        let counts = image_counts(&library).unwrap();

        assert_eq!(counts.total, 5);
        assert_eq!(counts.extension, 3);
        assert_eq!(counts.local, 2);
        assert_eq!(counts.legacy_bundle, 0);
        assert_eq!(counts.total, library.image_count().unwrap());
    }

    #[test]
    fn counts_an_empty_library_as_zero() {
        let (_dir, library) = library();

        assert_eq!(image_counts(&library).unwrap(), ImageCounts::default());
    }

    #[test]
    fn a_file_deleted_outside_the_app_is_marked_missing_and_restoring_it_clears_the_mark() {
        let (_dir, library) = library();
        store(&library, "a", ImageSource::Extension, &[]);
        let ids = vec!["a".to_string()];
        let path = library.paths.image_path("a", "png");
        let bytes = std::fs::read(&path).unwrap();

        std::fs::remove_file(&path).unwrap();
        refresh_missing_for(&library, &ids).unwrap();
        assert!(missing_flag(&library, "a"));

        std::fs::write(&path, &bytes).unwrap();
        refresh_missing_for(&library, &ids).unwrap();
        assert!(!missing_flag(&library, "a"));
    }

    #[test]
    fn only_the_ids_it_was_given_are_re_checked() {
        let (_dir, library) = library();
        store(&library, "checked", ImageSource::Extension, &[]);
        store(&library, "ignored", ImageSource::Extension, &[]);
        std::fs::remove_file(library.paths.image_path("checked", "png")).unwrap();
        std::fs::remove_file(library.paths.image_path("ignored", "png")).unwrap();

        refresh_missing_for(&library, &["checked".to_string()]).unwrap();

        assert!(missing_flag(&library, "checked"));
        assert!(!missing_flag(&library, "ignored"));
    }

    #[test]
    fn an_empty_id_list_and_an_unchanged_library_are_both_no_ops() {
        let (_dir, library) = library();
        store(&library, "a", ImageSource::Extension, &[]);

        refresh_missing_for(&library, &[]).unwrap();
        refresh_missing_for(&library, &["a".to_string(), "not-a-row".to_string()]).unwrap();

        assert!(!missing_flag(&library, "a"));
    }
}
