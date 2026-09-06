//! Thumbnails under `<library>/.thumbs/<id>.jpg` — a derived cache that is safe
//! to delete and regenerated on demand (design D7).

use std::fs::{self, File};
use std::path::{Path, PathBuf};

use image::{DynamicImage, ImageEncoder, codecs::jpeg::JpegEncoder};

use crate::error::{AppError, Result};
use crate::library::Library;
use crate::model::ImageRecord;

/// Longest edge of a generated thumbnail. 384 px until the grid exists to judge
/// it (design, Open Questions).
pub const THUMB_EDGE: u32 = 384;

/// JPEG quality. High enough that the recompression is invisible at 384 px,
/// low enough that a library of thousands stays a cache and not a second copy.
const THUMB_QUALITY: u8 = 82;

/// Path of `record`'s thumbnail, generating it first if it is not there.
///
/// Callers on the capture path swallow the error rather than fail the ingest,
/// so this must never panic: a corrupt or missing source file is an `Err`.
pub fn ensure_thumbnail(library: &Library, record: &ImageRecord) -> Result<PathBuf> {
    let thumb = thumbnail_path(library, &record.id);
    if thumb.is_file() {
        return Ok(thumb);
    }
    // FIXME: the source is read and decoded a second time here — `ingest` had
    // the decoded image in hand a moment earlier. The right shape is a variant
    // taking the already-decoded `DynamicImage`, with this path kept for the
    // on-demand regeneration `thumbnail_path` needs. Not built yet: on-ingest
    // thumbnailing is one image at a time behind the library mutex, so the
    // double read only shows up in a bulk import.
    write_through_part(
        &library.image_path(&record.id, &record.ext),
        &part_path(library, &record.id),
        &thumb,
    )?;
    Ok(thumb)
}

/// Where `id`'s thumbnail lives, whether or not it has been generated. Dropping
/// a record deletes this file (design D16), so the name is defined once here.
pub fn thumbnail_path(library: &Library, id: &str) -> PathBuf {
    library.thumbs_dir().join(format!("{id}.jpg"))
}

fn part_path(library: &Library, id: &str) -> PathBuf {
    library.thumbs_dir().join(format!("{id}.jpg.part"))
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
    use crate::model::ImageSource;

    fn png_bytes(width: u32, height: u32) -> Vec<u8> {
        let image = DynamicImage::ImageRgba8(image::RgbaImage::new(width, height));
        let mut out = std::io::Cursor::new(Vec::new());
        image.write_to(&mut out, image::ImageFormat::Png).unwrap();
        out.into_inner()
    }

    /// Store one image the way every caller does, so these tests see the record
    /// ingest produces — including the thumbnail it already asked for.
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
                rating: None,
                tags: &[],
                captured_at: 1_700_000_000_000,
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

        let thumb = ensure_thumbnail(&library, &record).unwrap();

        assert_eq!(dimensions(&thumb), (THUMB_EDGE, THUMB_EDGE * 600 / 800));
    }

    #[test]
    fn scales_a_tall_image_by_its_height() {
        let (_dir, library, record) = library_with_image(500, 1000);

        let thumb = ensure_thumbnail(&library, &record).unwrap();

        assert_eq!(dimensions(&thumb), (THUMB_EDGE / 2, THUMB_EDGE));
    }

    #[test]
    fn never_upscales_a_smaller_image() {
        let (_dir, library, record) = library_with_image(120, 90);

        let thumb = ensure_thumbnail(&library, &record).unwrap();

        assert_eq!(dimensions(&thumb), (120, 90));
    }

    #[test]
    fn ingest_generates_the_thumbnail_without_being_asked() {
        let (_dir, library, record) = library_with_image(800, 600);

        assert!(thumbnail_path(&library, &record.id).is_file());
    }

    #[test]
    fn regenerates_a_thumbnail_the_user_deleted() {
        let (_dir, library, record) = library_with_image(800, 600);
        fs::remove_dir_all(library.thumbs_dir()).unwrap();

        let thumb = ensure_thumbnail(&library, &record).unwrap();

        assert!(thumb.is_file());
        assert_eq!(dimensions(&thumb), (THUMB_EDGE, THUMB_EDGE * 600 / 800));
    }

    #[test]
    fn leaves_an_existing_thumbnail_alone() {
        let (_dir, library, record) = library_with_image(800, 600);
        let thumb = thumbnail_path(&library, &record.id);
        fs::write(&thumb, b"an older thumbnail").unwrap();

        assert_eq!(ensure_thumbnail(&library, &record).unwrap(), thumb);
        assert_eq!(fs::read(&thumb).unwrap(), b"an older thumbnail");
    }

    #[test]
    fn a_missing_source_file_is_an_error_not_a_panic() {
        let (_dir, library, record) = library_with_image(800, 600);
        fs::remove_file(library.image_path(&record.id, &record.ext)).unwrap();
        fs::remove_file(thumbnail_path(&library, &record.id)).unwrap();

        let error = ensure_thumbnail(&library, &record).unwrap_err();

        assert!(
            matches!(error, AppError::Io(_)),
            "unexpected error: {error}"
        );
        assert_eq!(fs::read_dir(library.thumbs_dir()).unwrap().count(), 0);
    }
}
