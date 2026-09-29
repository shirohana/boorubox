//! The bundled frame extractor (`video-files` design D2): a minimal ffmpeg shipped as a Tauri
//! sidecar, spawned once per poster. It decodes h264, hevc, vp8 and vp9 and writes PNG; the
//! caller downscales and encodes the thumbnail, so an image and a video share one path.

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

/// The first frame of the video at `path`, decoded from the PNG the extractor writes to stdout.
///
/// FIXME: no timeout. A hung extractor blocks the calling thread forever; the library lock is
/// not held (thumbnails are generated with it released), so only that thread hangs. The right
/// shape is to spawn, wait on a helper thread or poll `try_wait`, and kill the child after N
/// seconds. Not built: no input is known to hang it.
///
/// FIXME: every call spawns a process, through the unbounded `spawn_blocking` pool, and a
/// failed poster is spawned again on every request. A wiped `.thumbs/` under a grid of video
/// tiles starts dozens of ffmpeg processes at once. The right shape is a small semaphore around
/// the spawn (and a remembered failure). Not built: the image path has the same shape today at
/// far lower cost per call.
pub fn first_frame(path: &Path) -> Result<DynamicImage> {
    let mut command = Command::new(locate()?);
    command
        .args(["-nostdin", "-loglevel", "error", "-i"])
        .arg(path)
        .args(["-frames:v", "1", "-f", "image2pipe", "-vcodec", "png", "-"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW: no console flashes over the app.
        command.creation_flags(0x0800_0000);
    }
    let output = command.spawn()?.wait_with_output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let last_line = stderr
            .lines()
            .rev()
            .find(|line| !line.trim().is_empty())
            .unwrap_or("no output");
        return Err(AppError::Io(std::io::Error::other(format!(
            "ffmpeg failed ({}): {last_line}",
            output.status
        ))));
    }
    Ok(image::load_from_memory(&output.stdout)?)
}

#[cfg(test)]
mod tests {
    use super::*;

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
