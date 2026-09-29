//! Playback samples under `<library>/.samples/<a1>/<id>.mp4` (`hevc-samples` design D1): an
//! H.264 copy of a video the webview's engine cannot decode, made on request by the bundled
//! sidecar with the OS encoder. A derived cache, safe to delete, like `.thumbs/`.

use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::error::{AppError, Result};
use crate::ffmpeg;
use crate::library::LibraryPaths;
use crate::model::{ImageRecord, SamplesReport};

#[cfg(windows)]
const ENCODER: &str = "h264_mf";
#[cfg(not(windows))]
const ENCODER: &str = "h264_videotoolbox";

const MIN_BITRATE: i64 = 2_000_000;
const MAX_BITRATE: i64 = 12_000_000;
/// For a video whose container states no length, so no bitrate can be derived from its size.
const UNKNOWN_DURATION_BITRATE: i64 = 6_000_000;

/// One encode at a time in the process: two clips opened in a row must not run two encoders,
/// and a request for a clip already encoding waits here, then finds the file.
static ENCODING: Mutex<()> = Mutex::new(());

/// Where `id`'s sample lives, whether or not it has been made: `.samples/<a1>/<id>.mp4`,
/// bucketed by `library::shard_dirs` like the thumbnail. Dropping a record deletes this file.
pub fn sample_path(paths: &LibraryPaths, id: &str) -> PathBuf {
    bucketed_samples_path(paths, id, &format!("{id}.mp4"))
}

fn part_path(paths: &LibraryPaths, id: &str) -> PathBuf {
    bucketed_samples_path(paths, id, &format!("{id}.mp4.part"))
}

fn bucketed_samples_path(paths: &LibraryPaths, id: &str, file_name: &str) -> PathBuf {
    let mut path = paths.samples_dir();
    path.extend(crate::library::shard_dirs(id));
    path.push(file_name);
    path
}

/// Path of `record`'s playback sample, encoding it first if it is not there. Takes the paths,
/// not the `Library`: the encode runs as long as the clip and must not hold the library lock.
/// `on_progress` gets the fraction done, 0 to 1, while the encoder runs.
///
/// The encode goes to a `.mp4.part` that is renamed into place once it is on disk, so a crash
/// never leaves `<id>.mp4` holding half a video that every later call would accept.
pub fn ensure_sample(
    paths: &LibraryPaths,
    record: &ImageRecord,
    on_progress: &mut dyn FnMut(f64),
) -> Result<PathBuf> {
    let sample = sample_path(paths, &record.id);
    if sample.is_file() {
        return Ok(sample);
    }
    let _encoding = ENCODING
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if sample.is_file() {
        return Ok(sample);
    }
    let part = part_path(paths, &record.id);
    let source = paths.image_path(&record.id, &record.ext);
    let mut write = || -> Result<()> {
        if let Some(dir) = sample.parent() {
            fs::create_dir_all(dir)?;
        }
        ffmpeg::encode_sample(
            &source,
            &part,
            ENCODER,
            sample_bitrate(record.size, record.duration_ms),
            record.duration_ms.unwrap_or(0),
            on_progress,
        )?;
        // Opened for write: Windows refuses to flush a read-only handle.
        fs::OpenOptions::new().write(true).open(&part)?.sync_all()?;
        Ok(())
    };
    if let Err(error) = write() {
        let _ = fs::remove_file(&part);
        return Err(error);
    }
    fs::rename(&part, &sample).map_err(|error| {
        let _ = fs::remove_file(&part);
        AppError::Io(error)
    })?;
    // The encode holds no library lock, so the record can be deleted forever meanwhile;
    // its sample would then be renamed into place after `delete_forever` looked for it.
    if !source.is_file() {
        let _ = fs::remove_file(&sample);
        return Err(AppError::NotFound(format!("image {}", record.id)));
    }
    Ok(sample)
}

/// The source's own bitrate (`size × 8 / duration`) held between 2 and 12 Mbps, so a sample is
/// about as big as its source without a wild guess turning short clips into starved ones.
fn sample_bitrate(size: i64, duration_ms: Option<i64>) -> i64 {
    match duration_ms {
        Some(ms) if ms > 0 => (size * 8 * 1000 / ms).clamp(MIN_BITRATE, MAX_BITRATE),
        _ => UNKNOWN_DURATION_BITRATE,
    }
}

/// Delete every finished sample under `.samples/`, counting files and bytes. One that will not
/// unlink is left and not counted. A `.part` is never touched: an encode runs without the
/// library lock and owns its part, so unlinking it here would break the encode (ffmpeg keeps
/// writing to the unlinked inode on macOS; Windows refuses the unlink). Stray parts are the
/// sweep on open's to remove.
pub fn clear_all(paths: &LibraryPaths) -> Result<SamplesReport> {
    let mut report = SamplesReport::default();
    let Ok(buckets) = fs::read_dir(paths.samples_dir()) else {
        return Ok(report);
    };
    for bucket in buckets.flatten() {
        let Ok(files) = fs::read_dir(bucket.path()) else {
            continue;
        };
        for file in files.flatten() {
            let Ok(metadata) = file.metadata() else {
                continue;
            };
            let is_part = file.path().extension().is_some_and(|ext| ext == "part");
            if metadata.is_file() && !is_part && fs::remove_file(file.path()).is_ok() {
                report.removed += 1;
                report.bytes += metadata.len() as i64;
            }
        }
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingest::{self, IngestInput};
    use crate::library::Library;
    use crate::model::ImageSource;
    use std::path::Path;

    fn fixture(name: &str) -> Vec<u8> {
        fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("fixtures/video")
                .join(name),
        )
        .unwrap()
    }

    fn library_with_hvc1() -> (tempfile::TempDir, Library, ImageRecord) {
        let dir = tempfile::tempdir().unwrap();
        let library = Library::open_or_create(dir.path()).unwrap();
        ingest::store_image(
            &library,
            IngestInput {
                id: "abc",
                bytes: &fixture("hvc1.mp4"),
                source: ImageSource::Extension,
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
        let record = ingest::require_record(&library.conn, "abc").unwrap();
        (dir, library, record)
    }

    #[test]
    fn ensure_sample_of_the_hvc1_fixture_is_avc1_with_the_same_size_and_length() {
        let (_dir, library, record) = library_with_hvc1();

        let path = ensure_sample(&library.paths, &record, &mut |_| {}).unwrap();

        assert_eq!(path, sample_path(&library.paths, "abc"));
        let sample = crate::media::probe(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(sample.codec, Some("avc1"));
        assert_eq!((sample.width, sample.height), (16, 16));
        let length = sample.duration_ms.unwrap();
        assert!((length - 200).abs() <= 2, "{length}");
        assert!(!part_path(&library.paths, "abc").exists());
    }

    #[test]
    fn a_second_call_returns_the_cached_file() {
        let (_dir, library, record) = library_with_hvc1();
        let first = ensure_sample(&library.paths, &record, &mut |_| {}).unwrap();
        let modified = fs::metadata(&first).unwrap().modified().unwrap();
        let mut ticks = 0;

        let second = ensure_sample(&library.paths, &record, &mut |_| ticks += 1).unwrap();

        assert_eq!(second, first);
        assert_eq!(fs::metadata(&second).unwrap().modified().unwrap(), modified);
        assert_eq!(ticks, 0, "no encoder ran");
    }

    #[test]
    fn progress_is_reported_between_zero_and_one() {
        let (_dir, library, record) = library_with_hvc1();
        let mut ratios = Vec::new();

        ensure_sample(&library.paths, &record, &mut |ratio| ratios.push(ratio)).unwrap();

        assert!(!ratios.is_empty(), "the encoder printed no progress line");
        assert!(
            ratios.iter().all(|ratio| (0.0..=1.0).contains(ratio)),
            "{ratios:?}"
        );
    }

    #[test]
    fn clear_all_reports_count_and_bytes() {
        let (_dir, library, record) = library_with_hvc1();
        let sample = ensure_sample(&library.paths, &record, &mut |_| {}).unwrap();
        let other = sample_path(&library.paths, "zzz");
        fs::create_dir_all(other.parent().unwrap()).unwrap();
        fs::write(&other, b"12345").unwrap();
        let bytes = fs::metadata(&sample).unwrap().len() as i64 + 5;

        let report = clear_all(&library.paths).unwrap();

        assert_eq!(report, SamplesReport { removed: 2, bytes });
        assert!(!sample.exists() && !other.exists());
        assert_eq!(clear_all(&library.paths).unwrap(), SamplesReport::default());
    }

    #[test]
    fn clear_all_leaves_a_running_encodes_part_alone() {
        let (_dir, library, _record) = library_with_hvc1();
        let part = part_path(&library.paths, "zzz");
        fs::create_dir_all(part.parent().unwrap()).unwrap();
        fs::write(&part, b"12345").unwrap();

        let report = clear_all(&library.paths).unwrap();

        assert_eq!(report, SamplesReport::default());
        assert!(part.exists());
    }

    #[test]
    fn trashing_a_video_removes_its_sample() {
        let (_dir, library, record) = library_with_hvc1();
        let sample = ensure_sample(&library.paths, &record, &mut |_| {}).unwrap();
        crate::trash::trash_images(&library, &["abc".to_string()]).unwrap();
        assert!(sample.exists(), "trashing keeps it, like the thumbnail");

        crate::trash::delete_forever(&library, &["abc".to_string()]).unwrap();

        assert!(!sample.exists());
    }

    #[test]
    fn the_bitrate_rule_clamps() {
        assert_eq!(sample_bitrate(50_000_000, Some(75_000)), 5_333_333);
        assert_eq!(sample_bitrate(1_000, Some(200)), MIN_BITRATE);
        assert_eq!(sample_bitrate(500_000_000, Some(10_000)), MAX_BITRATE);
        assert_eq!(sample_bitrate(1_000, None), UNKNOWN_DURATION_BITRATE);
        assert_eq!(sample_bitrate(1_000, Some(0)), UNKNOWN_DURATION_BITRATE);
    }
}
