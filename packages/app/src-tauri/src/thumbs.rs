//! Thumbnails under `<library>/.thumbs/<a1>/<id>.jpg` (`one-level-buckets`
//! design D1) — a derived cache that is safe to delete and regenerated on
//! demand (design D7).

use std::fs::{self, File};
use std::path::{Path, PathBuf};

use image::{DynamicImage, ImageEncoder, codecs::jpeg::JpegEncoder};

use crate::error::{AppError, Result};
use crate::ingest;
use crate::library::{LibraryPaths, SharedLibrary, with_library, with_library_if_open};
use crate::model::{ImageRecord, ThumbsReport};

/// Longest edge of a generated thumbnail (`one-level-buckets` design D3):
/// 768, sized for the 640 px tile cap (`sidebar-inspector-polish`) at DPR 2
/// (1.67×). `downscale` never upscales, so a smaller source keeps its own
/// size. Existing thumbnails stay at their old size until `regenerate_all`
/// re-renders them (design D4).
pub const THUMB_EDGE: u32 = 768;

/// JPEG quality, unchanged by the edge going up (design D3): high enough that
/// the recompression is invisible at `THUMB_EDGE`, low enough that a library
/// of thousands stays a cache and not a second copy.
const THUMB_QUALITY: u8 = 82;

/// Path of `record`'s thumbnail, generating it first if it is not there.
///
/// Takes the paths rather than the `Library` because every caller runs this
/// with the library mutex released: it decodes, downscales and re-encodes the
/// full image, the largest single piece of work on the capture path, and
/// holding the one connection through it stops the window (design D13).
///
/// Callers on the capture path swallow the error rather than fail the ingest,
/// so this must never panic: a corrupt or missing source file is an `Err`.
pub fn ensure_thumbnail(paths: &LibraryPaths, record: &ImageRecord) -> Result<PathBuf> {
    let thumb = thumbnail_path(paths, &record.id);
    if thumb.is_file() {
        return Ok(thumb);
    }
    // FIXME: the source is read and decoded a second time here — `ingest` had
    // the decoded image in hand a moment earlier. The right shape is a variant
    // taking the already-decoded `DynamicImage`, with this path kept for the
    // on-demand regeneration `thumbnail_path` needs. Not built yet: the store
    // now hands the bytes back before the thumbnail, so the double read costs
    // the caller's thread and not the library lock.
    write_through_part(
        &paths.image_path(&record.id, &record.ext),
        &part_path(paths, &record.id),
        &thumb,
    )?;
    Ok(thumb)
}

/// Generate the thumbnail a freshly stored image will be shown by, and let a
/// failure pass.
///
/// The thumbnail is a derived cache `.thumbs/` may lose at any time (design
/// D7) and `ensure_thumbnail` regenerates it on demand, so failing here would
/// only throw away an image the caller already has and cannot fetch again — the
/// capture is gone from the page by then. Every path that stores an image calls
/// this, and calls it after releasing the library.
pub fn warm_thumbnail(paths: &LibraryPaths, record: &ImageRecord) {
    let _ = ensure_thumbnail(paths, record);
}

/// Re-render every image's thumbnail at the current `THUMB_EDGE`
/// (`one-level-buckets` design D4): a user-started background pass, the same
/// shape `library::backfill_sidecars` runs the sidecar repair in — the id
/// list and the records are read once under the lock, trashed rows included
/// (a trashed image keeps its thumbnail until it is deleted forever). Before
/// each image, the lock is taken again only to check the open library's root
/// still matches `root`; the decode, downscale and encode never run under it
/// — they run against `paths`, cloned once above the loop, with the library
/// free for a search or a capture, `ensure_thumbnail`'s own contract for
/// every caller. Each render goes through `write_through_part` directly
/// rather than `ensure_thumbnail`, whose already-there check would skip
/// every image this pass exists to redo: the rename `write_through_part`
/// ends in replaces `<id>.jpg` atomically, so a render that fails (a source
/// gone missing) leaves the old thumbnail in place rather than losing it to
/// an earlier delete.
///
/// `root` is the path this pass was started for: every image checks the open
/// library's root still matches it before touching a file, and the pass
/// returns short — without writing that image or any after it — the moment it
/// does not, the same check that stops `backfill_sidecars` on a library
/// switch (`pending-work` spec's "Switching libraries mid-pass").
pub fn regenerate_all(
    library: &SharedLibrary,
    root: &Path,
    on_progress: &mut dyn FnMut(i64, i64),
) -> Result<ThumbsReport> {
    let (paths, ids) = with_library(library, |open| {
        Ok((
            open.paths.clone(),
            crate::library::all_image_ids(&open.conn)?,
        ))
    })?;
    if paths.root != root {
        return Ok(ThumbsReport::default());
    }
    let records = with_library(library, |open| ingest::load_records(&open.conn, &ids))?;

    let total = records.len() as i64;
    let mut report = ThumbsReport::default();
    on_progress(0, total);
    for (index, record) in records.iter().enumerate() {
        let still_open = with_library_if_open(library, |open| {
            Ok(matches!(open, Some(open) if open.paths.root == root))
        })?;
        if !still_open {
            // The library closed, or a switch moved it onto another root:
            // this pass's write for this image, and every one after it,
            // belongs to a folder that is no longer open.
            return Ok(report);
        }
        match write_through_part(
            &paths.image_path(&record.id, &record.ext),
            &part_path(&paths, &record.id),
            &thumbnail_path(&paths, &record.id),
        ) {
            Ok(_) => report.regenerated += 1,
            Err(_) => report.failed += 1,
        }
        on_progress(index as i64 + 1, total);
    }

    Ok(report)
}

/// Where `id`'s thumbnail lives, whether or not it has been generated:
/// `.thumbs/<a1>/<id>.jpg` (`one-level-buckets` design D1), sharing
/// `library::shard_dirs` with `LibraryPaths::relative_image_path` — one
/// definition of the bucket rule for both directories. Dropping a record
/// deletes this file (design D16), so the name is defined once here.
pub fn thumbnail_path(paths: &LibraryPaths, id: &str) -> PathBuf {
    bucketed_thumbs_path(paths, id, &format!("{id}.jpg"))
}

fn part_path(paths: &LibraryPaths, id: &str) -> PathBuf {
    bucketed_thumbs_path(paths, id, &format!("{id}.jpg.part"))
}

fn bucketed_thumbs_path(paths: &LibraryPaths, id: &str, file_name: &str) -> PathBuf {
    let mut path = paths.thumbs_dir();
    path.extend(crate::library::shard_dirs(id));
    path.push(file_name);
    path
}

/// Encode to a part file, fsync, rename — the same shape as [`crate::ingest`],
/// for the same reason: a crash mid-encode must not leave `<id>.jpg` holding
/// half a JPEG, which every later call would accept as a finished thumbnail.
fn write_through_part(source: &Path, part: &Path, dest: &Path) -> Result<()> {
    let encode = || -> Result<()> {
        let thumb = downscale(image::load_from_memory(&fs::read(source)?)?);
        // `.thumbs/` is the user's to delete at any moment, so it is recreated
        // here rather than assumed to survive from `Library::open_or_create`.
        if let Some(dir) = dest.parent() {
            fs::create_dir_all(dir)?;
        }
        let mut file = File::create(part)?;
        // JPEG carries no alpha channel: handing the encoder an RGBA buffer is
        // an error, not a transparent thumbnail.
        let rgb = thumb.to_rgb8();
        JpegEncoder::new_with_quality(&mut file, THUMB_QUALITY).write_image(
            rgb.as_raw(),
            rgb.width(),
            rgb.height(),
            image::ExtendedColorType::Rgb8,
        )?;
        file.sync_all()?;
        Ok(())
    };
    if let Err(error) = encode() {
        let _ = fs::remove_file(part);
        return Err(error);
    }
    fs::rename(part, dest).map_err(|error| {
        let _ = fs::remove_file(part);
        AppError::Io(error)
    })
}

/// Fit inside a `THUMB_EDGE` square, preserving the aspect ratio. An image
/// already smaller is copied at its own size: upscaling would cost bytes to
/// show the grid nothing it did not already have.
fn downscale(image: DynamicImage) -> DynamicImage {
    if image.width().max(image.height()) <= THUMB_EDGE {
        return image;
    }
    image.resize(
        THUMB_EDGE,
        THUMB_EDGE,
        image::imageops::FilterType::Lanczos3,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::ingest::{self, IngestInput};
    use crate::library::Library;
    use crate::model::ImageSource;

    fn png_bytes(width: u32, height: u32) -> Vec<u8> {
        let image = DynamicImage::ImageRgba8(image::RgbaImage::new(width, height));
        let mut out = std::io::Cursor::new(Vec::new());
        image.write_to(&mut out, image::ImageFormat::Png).unwrap();
        out.into_inner()
    }

    /// Store one image the way every caller does, so these tests see the record
    /// ingest produces. No thumbnail comes with it: the store returns before
    /// the encode, and each caller warms it with the library released (design
    /// D13).
    fn library_with_image(width: u32, height: u32) -> (tempfile::TempDir, Library, ImageRecord) {
        let dir = tempfile::tempdir().unwrap();
        let library = Library::open_or_create(dir.path()).unwrap();
        let bytes = png_bytes(width, height);
        let ingested = ingest::store_image(
            &library,
            IngestInput {
                id: "id-1",
                bytes: &bytes,
                source: ImageSource::Local,
                source_ref: None,
                image_url: None,
                page_url: None,
                page_title: Some("cat.png"),
                adapter: None,
                rating: None,
                tags: &[],
                captured_at: 1_700_000_000_000,
                file_modified_at: None,
                deleted_at: None,
            },
        )
        .unwrap();
        let record = ingested.record().clone();
        (dir, library, record)
    }

    fn dimensions(path: &Path) -> (u32, u32) {
        image::image_dimensions(path).unwrap()
    }

    #[test]
    fn scales_the_longest_edge_to_the_thumb_edge() {
        let (_dir, library, record) = library_with_image(800, 600);

        let thumb = ensure_thumbnail(&library.paths, &record).unwrap();

        assert_eq!(dimensions(&thumb), (THUMB_EDGE, THUMB_EDGE * 600 / 800));
    }

    #[test]
    fn scales_a_tall_image_by_its_height() {
        let (_dir, library, record) = library_with_image(500, 1000);

        let thumb = ensure_thumbnail(&library.paths, &record).unwrap();

        assert_eq!(dimensions(&thumb), (THUMB_EDGE / 2, THUMB_EDGE));
    }

    #[test]
    fn never_upscales_a_smaller_image() {
        let (_dir, library, record) = library_with_image(120, 90);

        let thumb = ensure_thumbnail(&library.paths, &record).unwrap();

        assert_eq!(dimensions(&thumb), (120, 90));
    }

    #[test]
    fn regenerates_a_thumbnail_the_user_deleted() {
        let (_dir, library, record) = library_with_image(800, 600);
        fs::remove_dir_all(library.paths.thumbs_dir()).unwrap();

        let thumb = ensure_thumbnail(&library.paths, &record).unwrap();

        assert!(thumb.is_file());
        assert_eq!(dimensions(&thumb), (THUMB_EDGE, THUMB_EDGE * 600 / 800));
    }

    #[test]
    fn leaves_an_existing_thumbnail_alone() {
        let (_dir, library, record) = library_with_image(800, 600);
        let thumb = thumbnail_path(&library.paths, &record.id);
        fs::create_dir_all(thumb.parent().unwrap()).unwrap();
        fs::write(&thumb, b"an older thumbnail").unwrap();

        assert_eq!(ensure_thumbnail(&library.paths, &record).unwrap(), thumb);
        assert_eq!(fs::read(&thumb).unwrap(), b"an older thumbnail");
    }

    #[test]
    fn ensure_thumbnail_creates_the_bucket_directory_before_writing() {
        let (_dir, library, record) = library_with_image(800, 600);
        let bucket = thumbnail_path(&library.paths, &record.id)
            .parent()
            .unwrap()
            .to_path_buf();
        assert!(
            !bucket.is_dir(),
            "the bucket must not exist before the thumbnail is generated"
        );

        ensure_thumbnail(&library.paths, &record).unwrap();

        assert!(bucket.is_dir());
    }

    #[test]
    fn a_missing_source_file_is_an_error_not_a_panic() {
        let (_dir, library, record) = library_with_image(800, 600);
        fs::remove_file(library.paths.image_path(&record.id, &record.ext)).unwrap();

        let error = ensure_thumbnail(&library.paths, &record).unwrap_err();

        assert!(
            matches!(error, AppError::Io(_)),
            "unexpected error: {error}"
        );
        assert!(!part_path(&library.paths, &record.id).exists());
    }

    /// `n` images, all the same 800x600 source, distinct ids — what the
    /// `regenerate_all` tests below need a library holding several of.
    fn library_with_images(n: u32) -> (tempfile::TempDir, Library, Vec<ImageRecord>) {
        let dir = tempfile::tempdir().unwrap();
        let library = Library::open_or_create(dir.path()).unwrap();
        let bytes = png_bytes(800, 600);
        let mut records = Vec::new();
        for index in 0..n {
            let id = format!("id-{index}");
            let ingested = ingest::store_image(
                &library,
                IngestInput {
                    id: &id,
                    bytes: &bytes,
                    source: ImageSource::Local,
                    source_ref: None,
                    image_url: None,
                    page_url: None,
                    page_title: None,
                    adapter: None,
                    rating: None,
                    tags: &[],
                    captured_at: 0,
                    file_modified_at: None,
                    deleted_at: None,
                },
            )
            .unwrap();
            records.push(ingested.record().clone());
        }
        (dir, library, records)
    }

    fn shared(library: Library) -> crate::library::SharedLibrary {
        std::sync::Arc::new(std::sync::Mutex::new(Some(library)))
    }

    /// A thumbnail-shaped JPEG at `width`x`height`, written straight to
    /// `path` rather than through `ensure_thumbnail` — what a thumbnail left
    /// over from the old `THUMB_EDGE` looks like on disk.
    fn write_jpeg(path: &Path, width: u32, height: u32) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let image = DynamicImage::ImageRgb8(image::RgbImage::new(width, height));
        let mut file = File::create(path).unwrap();
        let rgb = image.to_rgb8();
        JpegEncoder::new_with_quality(&mut file, THUMB_QUALITY)
            .write_image(
                rgb.as_raw(),
                rgb.width(),
                rgb.height(),
                image::ExtendedColorType::Rgb8,
            )
            .unwrap();
    }

    #[test]
    fn regenerate_all_replaces_a_small_thumbnail_with_one_at_the_current_edge() {
        let (_dir, library, record) = library_with_image(800, 600);
        let thumb = thumbnail_path(&library.paths, &record.id);
        write_jpeg(&thumb, 384, 288);
        let root = library.paths.root.clone();
        let shared = shared(library);

        regenerate_all(&shared, &root, &mut |_, _| {}).unwrap();

        assert_eq!(dimensions(&thumb), (THUMB_EDGE, THUMB_EDGE * 600 / 800));
    }

    #[test]
    fn regenerate_all_counts_an_unreadable_image_and_continues() {
        let (_dir, library, records) = library_with_images(3);
        let unreadable = &records[1];
        fs::remove_file(library.paths.image_path(&unreadable.id, &unreadable.ext)).unwrap();
        let root = library.paths.root.clone();
        let shared = shared(library);

        let report = regenerate_all(&shared, &root, &mut |_, _| {}).unwrap();

        assert_eq!(report.regenerated, 2);
        assert_eq!(report.failed, 1);
        with_library(&shared, |open| {
            for record in &records {
                if record.id != unreadable.id {
                    assert!(thumbnail_path(&open.paths, &record.id).is_file());
                }
            }
            Ok(())
        })
        .unwrap();
    }

    /// A render that fails must not have already destroyed the thumbnail it
    /// was about to replace: `write_through_part` renders to a `.part` and
    /// only then renames over `<id>.jpg`, so a source gone missing leaves the
    /// old file exactly where it was rather than removing it up front.
    #[test]
    fn a_missing_image_keeps_its_old_thumbnail_and_is_counted_failed() {
        let (_dir, library, records) = library_with_images(3);
        let missing = &records[1];
        let thumb = thumbnail_path(&library.paths, &missing.id);
        write_jpeg(&thumb, 384, 288);
        fs::remove_file(library.paths.image_path(&missing.id, &missing.ext)).unwrap();
        let root = library.paths.root.clone();
        let shared = shared(library);

        let report = regenerate_all(&shared, &root, &mut |_, _| {}).unwrap();

        assert_eq!(report.regenerated, 2);
        assert_eq!(report.failed, 1);
        assert_eq!(
            dimensions(&thumb),
            (384, 288),
            "a failed render must leave the old thumbnail in place"
        );
    }

    #[test]
    fn regenerate_all_reports_done_and_total() {
        let (_dir, library, records) = library_with_images(3);
        let root = library.paths.root.clone();
        let shared = shared(library);

        let mut ticks = Vec::new();
        let report =
            regenerate_all(&shared, &root, &mut |done, total| ticks.push((done, total))).unwrap();

        assert_eq!(ticks, vec![(0, 3), (1, 3), (2, 3), (3, 3)]);
        assert_eq!(report.regenerated, records.len() as i64);
        assert_eq!(report.failed, 0);
    }

    /// The `Mutex` this pass reads from is swapped mid-run, from inside the
    /// progress callback — the shape `library-sidecars` design D7's own
    /// switch-mid-pass test uses — and the pass must notice on its very next
    /// image rather than carry on writing into the library that is no longer
    /// open.
    #[test]
    fn regenerate_all_stops_when_the_library_is_switched() {
        let (_dir, library, records) = library_with_images(3);
        let root = library.paths.root.clone();
        let shared = shared(library);
        let switched = shared.clone();

        let mut ticks = Vec::new();
        let report = regenerate_all(&shared, &root, &mut |done, total| {
            ticks.push((done, total));
            if done == 1 {
                let other_dir = tempfile::tempdir().unwrap();
                let other = Library::open_or_create(other_dir.path()).unwrap();
                *switched.lock().unwrap() = Some(other);
            }
        })
        .unwrap();

        assert_eq!(ticks, vec![(0, 3), (1, 3)], "no tick after the switch");
        assert_eq!(
            report.regenerated + report.failed,
            1,
            "only the image processed before the switch counts"
        );
        assert!((records.len() as i64) > report.regenerated + report.failed);
    }
}
