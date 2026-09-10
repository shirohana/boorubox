//! The one durable write a successful upload produces (`booru-upload` design
//! D5): the `posts` row and the image's `updated_at` bump, together, taken
//! only after the last network call that can fail hard has already
//! succeeded. Nothing before this point touches the library; nothing here
//! ever runs across a network `.await`.

use rusqlite::params;

use crate::error::Result;
use crate::library::Library;
use crate::model::PostRef;

/// Record `post` against `image_id`. An upsert, not a bare insert: a second
/// successful upload to a site the image already has a record for (outside
/// this app, per `booru-upload` design D12 — re-upload from here is not
/// offered) still lands on one row per `(image_id, site)`, matching the
/// table's own primary key.
pub fn record(library: &Library, image_id: &str, post: &PostRef) -> Result<()> {
    let tx = library.conn.unchecked_transaction()?;
    tx.execute(
        "INSERT INTO posts (image_id, site, remote_id, posted_at) VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT (image_id, site) DO UPDATE SET remote_id = excluded.remote_id,
                                                     posted_at = excluded.posted_at",
        params![image_id, post.site, post.remote_id, post.posted_at],
    )?;
    tx.execute(
        "UPDATE images SET updated_at = ?1 WHERE id = ?2",
        params![post.posted_at, image_id],
    )?;
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingest::require_record;

    fn library() -> (tempfile::TempDir, Library) {
        let dir = tempfile::tempdir().unwrap();
        let library = Library::open_or_create(dir.path()).unwrap();
        (dir, library)
    }

    fn insert_bare_image(library: &Library, id: &str) {
        library
            .conn
            .execute(
                "INSERT INTO images (id, ext, mime, size, width, height, source, captured_at,
                                     created_at, updated_at)
                 VALUES (?1, 'png', 'image/png', 1, 1, 1, 'local', 0, 0, 0)",
                [id],
            )
            .unwrap();
    }

    #[test]
    fn recording_a_post_stamps_the_row_and_bumps_updated_at() {
        let (_dir, library) = library();
        insert_bare_image(&library, "img-1");

        record(
            &library,
            "img-1",
            &PostRef {
                site: "danbooru".to_string(),
                remote_id: "42".to_string(),
                posted_at: 1_700_000_000_000,
            },
        )
        .unwrap();

        let record = require_record(&library.conn, "img-1").unwrap();
        assert_eq!(
            record.posts,
            vec![PostRef {
                site: "danbooru".to_string(),
                remote_id: "42".to_string(),
                posted_at: 1_700_000_000_000,
            }]
        );
        assert_eq!(record.updated_at, 1_700_000_000_000);
    }

    #[test]
    fn recording_twice_for_the_same_site_replaces_the_row_rather_than_duplicating_it() {
        let (_dir, library) = library();
        insert_bare_image(&library, "img-1");
        record(
            &library,
            "img-1",
            &PostRef {
                site: "danbooru".to_string(),
                remote_id: "42".to_string(),
                posted_at: 1,
            },
        )
        .unwrap();

        record(
            &library,
            "img-1",
            &PostRef {
                site: "danbooru".to_string(),
                remote_id: "43".to_string(),
                posted_at: 2,
            },
        )
        .unwrap();

        let record = require_record(&library.conn, "img-1").unwrap();
        assert_eq!(record.posts.len(), 1);
        assert_eq!(record.posts[0].remote_id, "43");
    }

    #[test]
    fn two_sites_for_the_same_image_are_both_kept() {
        let (_dir, library) = library();
        insert_bare_image(&library, "img-1");
        record(
            &library,
            "img-1",
            &PostRef {
                site: "danbooru".to_string(),
                remote_id: "1".to_string(),
                posted_at: 1,
            },
        )
        .unwrap();

        record(
            &library,
            "img-1",
            &PostRef {
                site: "self-hosted".to_string(),
                remote_id: "2".to_string(),
                posted_at: 2,
            },
        )
        .unwrap();

        let record = require_record(&library.conn, "img-1").unwrap();
        assert_eq!(record.posts.len(), 2);
    }
}
