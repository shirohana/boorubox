//! The bundled frame extractor (`video-files` design D2): a minimal ffmpeg shipped as a Tauri
//! sidecar, spawned once per poster. It decodes h264, hevc, vp8 and vp9 and writes PNG; the
//! caller downscales and encodes the thumbnail, so an image and a video share one path.

use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use image::DynamicImage;

use crate::error::{AppError, Result};

/// Where the sidecar is. Two explicit cases, no search list: beside the app executable in the
/// bundle and under `tauri dev`; under `cargo test` the test binary lives in `target/debug/deps`,
/// so the build script's copy under `binaries/` is used, named by the target triple.
fn locate() -> Result<PathBuf> {
    #[cfg(test)]
    {
        let name = concat!("ffmpeg-", env!("BOORUBOX_TARGET"));
        Ok(with_exe_suffix(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("binaries")
                .join(name),
        ))
    }
    #[cfg(not(test))]
    {
        let exe = std::env::current_exe()?;
        let dir = exe.parent().ok_or_else(|| {
            AppError::Io(std::io::Error::other("the app executable has no directory"))
        })?;
        Ok(with_exe_suffix(dir.join("ffmpeg")))
    }
}

fn with_exe_suffix(path: PathBuf) -> PathBuf {
    if cfg!(windows) {
        let mut name = path.into_os_string();
        name.push(".exe");
        PathBuf::from(name)
    } else {
        path
    }
}

/// The sidecar command every call starts from: no stdin, stderr captured for the failure line,
/// no console window on Windows.
fn extractor_command() -> Result<Command> {
    let mut command = Command::new(locate()?);
    command.stdin(Stdio::null()).stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW: no console flashes over the app.
        command.creation_flags(0x0800_0000);
    }
    Ok(command)
}

/// A non-zero exit is an `Io` error carrying the last non-empty line ffmpeg wrote to stderr.
fn check_status(output: &std::process::Output) -> Result<()> {
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let last_line = stderr
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("no output");
    Err(AppError::Io(std::io::Error::other(format!(
        "ffmpeg failed ({}): {last_line}",
        output.status
    ))))
}

/// Rewrite the mp4 at `source` into `dest` without re-encoding, its video sample entry tagged
/// `tag` (`hvc1` or `avc1`): parameter sets that sat in the stream move into the container
/// (`hevc-remux` design D1). File to file, because a moov-at-end mp4 cannot be read from a
/// pipe. A failed rewrite leaves no `dest`.
///
/// FIXME: this runs inside `ingest::store_image`, with the library lock held, and has no
/// timeout: a hung rewrite freezes every door of the library, not one thread. The right shape
/// is to rewrite before the lock is taken (the bytes are on disk in the inbox either way) or
/// to kill the child after N seconds. Not built: a stream copy of a 50 MB clip takes 0.15 s and
/// no input is known to hang it.
pub fn remux(source: &Path, dest: &Path, tag: &str) -> Result<()> {
    let output = extractor_command()?
        .args(["-nostdin", "-loglevel", "error", "-y", "-i"])
        .arg(source)
        .args([
            "-c",
            "copy",
            "-tag:v",
            tag,
            "-movflags",
            "+faststart",
            "-f",
            "mp4",
        ])
        .arg(dest)
        .spawn()?
        .wait_with_output()?;
    let checked = check_status(&output);
    if checked.is_err() {
        let _ = std::fs::remove_file(dest);
    }
    checked
}

/// Encode the video at `source` into an H.264 mp4 at `dest` with the OS encoder `encoder`, at
/// `bitrate` bits per second, audio to AAC (`hevc-samples` design D1). `on_progress` gets the
/// fraction done, 0 to 1, for every `out_time_us` line ffmpeg prints on stdout, measured
/// against `duration_ms`. A failed encode leaves no `dest`.
///
/// FIXME: no cancel and no timeout: moving on from the clip leaves the encode running to
/// completion (`hevc-samples` design D2). The right shape is a cancellation token checked
/// between progress lines that kills the child. Not built: a conversion is a one-off per clip
/// and its result is cached for the next open.
pub fn encode_sample(
    source: &Path,
    dest: &Path,
    encoder: &str,
    bitrate: i64,
    duration_ms: i64,
    on_progress: &mut dyn FnMut(f64),
) -> Result<()> {
    let mut child = extractor_command()?
        .args(["-nostdin", "-loglevel", "error", "-y", "-i"])
        .arg(source)
        .args(["-c:v", encoder, "-b:v"])
        .arg(bitrate.to_string())
        .args([
            "-pix_fmt",
            "yuv420p",
            "-tag:v",
            "avc1",
            "-c:a",
            "aac",
            "-b:a",
            "128k",
            "-movflags",
            "+faststart",
            "-progress",
            "pipe:1",
            "-nostats",
            "-f",
            "mp4",
        ])
        .arg(dest)
        .stdout(Stdio::piped())
        .spawn()?;
    // stderr is drained on its own thread: ffmpeg blocks on a full pipe, and stdout is read
    // to EOF below, so reading stderr only afterwards could deadlock.
    let mut stderr = child.stderr.take().map(|pipe| {
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            let _ = { pipe }.read_to_end(&mut bytes);
            bytes
        })
    });
    if let Some(stdout) = child.stdout.take() {
        for line in BufReader::new(stdout).lines().map_while(|line| line.ok()) {
            if let Some(ratio) = progress_ratio(&line, duration_ms) {
                on_progress(ratio);
            }
        }
    }
    let status = child.wait()?;
    let output = std::process::Output {
        status,
        stdout: Vec::new(),
        stderr: stderr
            .take()
            .and_then(|thread| thread.join().ok())
            .unwrap_or_default(),
    };
    let checked = check_status(&output);
    if checked.is_err() {
        let _ = std::fs::remove_file(dest);
    }
    checked
}

/// The fraction done that one line of ffmpeg's `-progress` output states, clamped to 0..=1:
/// `out_time_us=<microseconds>` against the clip's length. Every other key, `N/A` and a clip
/// of no known length give `None`.
fn progress_ratio(line: &str, duration_ms: i64) -> Option<f64> {
    let micros: f64 = line.strip_prefix("out_time_us=")?.trim().parse().ok()?;
    if duration_ms <= 0 {
        return None;
    }
    Some((micros / (duration_ms as f64 * 1000.0)).clamp(0.0, 1.0))
}

/// The first frame of the video at `path`, decoded from the PNG the extractor writes to stdout.
///
/// FIXME: no timeout. A hung extractor blocks the calling thread forever; posters are generated
/// with the library lock released, so here only that thread hangs (`remux` is the path that
/// holds the lock, see its own FIXME). The right
/// shape is to spawn, wait on a helper thread or poll `try_wait`, and kill the child after N
/// seconds. Not built: no input is known to hang it.
///
/// FIXME: every call spawns a process, through the unbounded `spawn_blocking` pool, and a
/// failed poster is spawned again on every request. A wiped `.thumbs/` under a grid of video
/// tiles starts dozens of ffmpeg processes at once. The right shape is a small semaphore around
/// the spawn (and a remembered failure). Not built: the image path has the same shape today at
/// far lower cost per call.
pub fn first_frame(path: &Path) -> Result<DynamicImage> {
    let mut command = extractor_command()?;
    command
        .args(["-nostdin", "-loglevel", "error", "-i"])
        .arg(path)
        .args(["-frames:v", "1", "-f", "image2pipe", "-vcodec", "png", "-"])
        .stdout(Stdio::piped());
    let output = command.spawn()?.wait_with_output()?;
    check_status(&output)?;
    Ok(image::load_from_memory(&output.stdout)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_lines_give_a_clamped_ratio() {
        assert_eq!(progress_ratio("out_time_us=500000", 1000), Some(0.5));
        assert_eq!(progress_ratio("out_time_us=9000000", 1000), Some(1.0));
        assert_eq!(progress_ratio("out_time_us=-1000", 1000), Some(0.0));
        assert_eq!(progress_ratio("out_time_us=N/A", 1000), None);
        assert_eq!(progress_ratio("out_time_ms=500000", 1000), None);
        assert_eq!(progress_ratio("out_time_us=500000", 0), None);
    }

    #[test]
    fn first_frame_of_the_h264_fixture_is_16_by_16() {
        let frame = first_frame(Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/fixtures/video/h264.mp4"
        )))
        .unwrap();

        assert_eq!((frame.width(), frame.height()), (16, 16));
    }

    #[test]
    fn first_frame_of_the_hevc_fixture_is_16_by_16() {
        let frame = first_frame(Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/fixtures/video/hvc1.mp4"
        )))
        .unwrap();

        assert_eq!((frame.width(), frame.height()), (16, 16));
    }

    #[test]
    fn remuxing_the_hev1_fixture_gives_an_hvc1_file_of_the_same_size_and_length() {
        let fixtures = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/video");
        let source = std::fs::read(format!("{fixtures}/hev1.mp4")).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("out.mp4");

        remux(Path::new(&format!("{fixtures}/hev1.mp4")), &dest, "hvc1").unwrap();

        let before = crate::media::probe(&source).unwrap();
        let after = crate::media::probe(&std::fs::read(&dest).unwrap()).unwrap();
        assert_eq!(before.codec, Some("hev1"));
        assert_eq!(after.codec, Some("hvc1"));
        assert_eq!((after.width, after.height), (before.width, before.height));
        assert_eq!(after.duration_ms, before.duration_ms);
    }

    #[test]
    fn a_file_ffmpeg_cannot_read_is_an_error_naming_its_last_line() {
        let error = first_frame(Path::new("/no/such/video.mp4")).unwrap_err();

        let message = error.to_string();
        assert!(message.contains("ffmpeg failed"), "{message}");
        assert!(
            message.ends_with("Error opening input files: No such file or directory"),
            "{message}"
        );
    }
}
