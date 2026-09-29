//! What a file's bytes are: a still image the `image` crate decodes, or an mp4 / webm whose
//! video track the app can show (`video-files` design D1). One door, `probe`, for every caller
//! of `ingest::store_image`; nothing reads a file's extension.

use std::io::Cursor;

use crate::error::{AppError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Image,
    Video,
}

/// The facts `images` records about a file, read from the bytes alone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Probed {
    pub kind: Kind,
    pub ext: &'static str,
    pub mime: &'static str,
    pub width: u32,
    pub height: u32,
    /// Milliseconds; `None` for an image and for a container that states no duration.
    pub duration_ms: Option<i64>,
    /// The codec name a video's container gives its track; `None` for an image.
    pub codec: Option<&'static str>,
}

/// Matroska codec ids the two webviews and the bundled extractor decode.
const ACCEPTED_MATROSKA: &[&str] = &["V_VP8", "V_VP9", "V_MPEG4/ISO/AVC", "V_MPEGH/ISO/HEVC"];

const NO_VIDEO_TRACK: &str = "no video track";

/// Undecodable bytes are an error before anything is written, so a rejected capture leaves no
/// file, no part file and no row. An image is fully decoded (its dimensions come from the
/// pixels); a video is read from its header only.
pub fn probe(bytes: &[u8]) -> Result<Probed> {
    // The container signatures are stricter than the image crate's: an mp4 whose `ftyp` box is
    // exactly 256 bytes starts `00 00 01 00`, which `guess_format` reads as ICO.
    if is_mp4(bytes) {
        probe_mp4(bytes)
    } else if is_matroska(bytes) {
        probe_webm(bytes)
    } else {
        probe_image(bytes, image::guess_format(bytes)?)
    }
}

fn probe_image(bytes: &[u8], format: image::ImageFormat) -> Result<Probed> {
    let image = image::load_from_memory_with_format(bytes, format)?;
    Ok(Probed {
        kind: Kind::Image,
        ext: format.extensions_str().first().copied().unwrap_or("bin"),
        mime: format.to_mime_type(),
        width: image.width(),
        height: image.height(),
        duration_ms: None,
        codec: None,
    })
}

fn is_mp4(bytes: &[u8]) -> bool {
    bytes.get(4..8) == Some(b"ftyp")
}

fn is_matroska(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0x1A, 0x45, 0xDF, 0xA3])
}

fn unsupported_codec(name: &str) -> AppError {
    AppError::Unsupported(format!("video codec {name} is not supported"))
}

/// A QuickTime brand is still `mp4`: the webview plays it. Rotation is not applied: an mp4
/// with a rotation matrix reports its stored dimensions, not the displayed ones.
///
/// `mp4parse` names H.264, MPEG-4 Part 2, VP8, VP9, AV1 and H.263 sample entries and reports
/// every other one, HEVC's `hvc1`/`hev1` included, as `Unknown`.
///
/// FIXME: an HEVC mp4 (an iPhone recording) is refused as `unknown` although the sidecar and
/// both webviews decode it. The right shape is a parser that names the sample entry's fourcc
/// (the `mp4` crate reads `hvc1`/`hev1` but drops every entry it does not know, so a refusal
/// could not name its codec) or a targeted read of the first `stsd` entry's fourcc when
/// `mp4parse` says `Unknown`. Not built because the owner's clips are H.264 and they asked to
/// raise unsupported formats as they hit them (`video-files` design D1).
fn probe_mp4(bytes: &[u8]) -> Result<Probed> {
    use mp4parse::{CodecType, SampleEntry, TrackType};

    let context = mp4parse::read_mp4(&mut Cursor::new(bytes))
        .map_err(|error| AppError::Unsupported(format!("unreadable mp4 header: {error:?}")))?;
    let track = context
        .tracks
        .iter()
        .find(|track| track.track_type == TrackType::Video)
        .ok_or_else(|| AppError::Unsupported(NO_VIDEO_TRACK.to_string()))?;
    let entry = track
        .stsd
        .as_ref()
        .and_then(|stsd| stsd.descriptions.iter().next());
    let Some(SampleEntry::Video(video)) = entry else {
        return Err(unsupported_codec("unknown"));
    };
    let codec = match video.codec_type {
        CodecType::H264 => "avc1",
        CodecType::VP9 => "vp09",
        CodecType::MP4V => return Err(unsupported_codec("mp4v")),
        CodecType::AV1 => return Err(unsupported_codec("av01")),
        CodecType::VP8 => return Err(unsupported_codec("vp08")),
        CodecType::H263 => return Err(unsupported_codec("s263")),
        _ => return Err(unsupported_codec("unknown")),
    };
    let duration_ms = match (&track.duration, &track.timescale) {
        (Some(duration), Some(timescale)) if timescale.0 > 0 => {
            known_duration(scale_to_ms(duration.0 as u128, timescale.0 as u128))
        }
        _ => None,
    };
    let (width, height) = checked_dimensions(
        u64::from(video.width),
        u64::from(video.height),
        "unreadable mp4 header",
    )?;
    Ok(Probed {
        kind: Kind::Video,
        ext: "mp4",
        mime: "video/mp4",
        width,
        height,
        duration_ms,
        codec: Some(codec),
    })
}

/// A zero duration is no duration: a fragmented mp4 keeps its length in the fragments and states
/// 0 in the track header, and a `0:00` badge would be a lie.
fn known_duration(duration_ms: i64) -> Option<i64> {
    (duration_ms > 0).then_some(duration_ms)
}

/// A video track's pixel size as `u32`; zero or beyond `u32` is a malformed header, never a
/// picture, so it is refused before anything is stored.
fn checked_dimensions(width: u64, height: u64, header: &str) -> Result<(u32, u32)> {
    match (u32::try_from(width), u32::try_from(height)) {
        (Ok(width @ 1..), Ok(height @ 1..)) => Ok((width, height)),
        _ => Err(AppError::Unsupported(format!(
            "{header}: zero-sized video track"
        ))),
    }
}

/// `units` at `units_per_second`, in milliseconds, rounded.
fn scale_to_ms(units: u128, units_per_second: u128) -> i64 {
    ((units * 1000 + units_per_second / 2) / units_per_second) as i64
}

/// The `matroska` crate skips the EBML header, so the DocType is found by its text: `webm`
/// sits in the first bytes of every webm, and a Matroska file that is not one is refused
/// because the platform webviews only promise webm.
fn probe_webm(bytes: &[u8]) -> Result<Probed> {
    use matroska::{Matroska, Settings};

    let head = &bytes[..bytes.len().min(64)];
    if !head.windows(4).any(|window| window == b"webm") {
        return Err(AppError::Unsupported(
            "matroska container is not supported, only webm".to_string(),
        ));
    }
    let matroska = Matroska::open(Cursor::new(bytes))
        .map_err(|error| AppError::Unsupported(format!("unreadable webm header: {error}")))?;
    let track = matroska
        .video_tracks()
        .next()
        .ok_or_else(|| AppError::Unsupported(NO_VIDEO_TRACK.to_string()))?;
    let Some(codec) = ACCEPTED_MATROSKA
        .iter()
        .copied()
        .find(|id| *id == track.codec_id)
    else {
        return Err(unsupported_codec(&track.codec_id));
    };
    let Settings::Video(video) = &track.settings else {
        return Err(AppError::Unsupported(NO_VIDEO_TRACK.to_string()));
    };
    let (width, height) = checked_dimensions(
        video.pixel_width,
        video.pixel_height,
        "unreadable webm header",
    )?;
    Ok(Probed {
        kind: Kind::Video,
        ext: "webm",
        mime: "video/webm",
        width,
        height,
        duration_ms: matroska
            .info
            .duration
            .and_then(|duration| known_duration(scale_to_ms(duration.as_nanos(), 1_000_000_000))),
        codec: Some(codec),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> Vec<u8> {
        std::fs::read(format!(
            "{}/fixtures/video/{name}",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap()
    }

    fn assert_within_a_millisecond(duration_ms: Option<i64>) {
        let duration = duration_ms.expect("a video fixture states its duration");
        assert!((199..=201).contains(&duration), "duration {duration} ms");
    }

    #[test]
    fn h264_mp4_is_probed_as_video() {
        let probed = probe(&fixture("h264.mp4")).unwrap();

        assert_eq!(probed.kind, Kind::Video);
        assert_eq!((probed.ext, probed.mime), ("mp4", "video/mp4"));
        assert_eq!((probed.width, probed.height), (16, 16));
        assert_eq!(probed.codec, Some("avc1"));
        assert_within_a_millisecond(probed.duration_ms);
    }

    #[test]
    fn vp9_webm_is_probed_as_video() {
        let probed = probe(&fixture("vp9.webm")).unwrap();

        assert_eq!(probed.kind, Kind::Video);
        assert_eq!((probed.ext, probed.mime), ("webm", "video/webm"));
        assert_eq!((probed.width, probed.height), (16, 16));
        assert_eq!(probed.codec, Some("V_VP9"));
        assert_within_a_millisecond(probed.duration_ms);
    }

    #[test]
    fn mpeg4_mp4_is_refused_by_codec_name() {
        let error = probe(&fixture("mpeg4.mp4")).unwrap_err();

        assert!(
            matches!(&error, AppError::Unsupported(reason) if reason == "video codec mp4v is not supported"),
            "unexpected error: {error:?}"
        );
    }

    #[test]
    fn audio_only_mp4_is_refused_for_no_video_track() {
        let error = probe(&fixture("audio-only.mp4")).unwrap_err();

        assert!(
            matches!(&error, AppError::Unsupported(reason) if reason == "no video track"),
            "unexpected error: {error:?}"
        );
    }

    #[test]
    fn a_png_named_mp4_is_a_png() {
        let probed = probe(&fixture("png-named.mp4")).unwrap();

        assert_eq!(probed.kind, Kind::Image);
        assert_eq!((probed.ext, probed.mime), ("png", "image/png"));
    }

    #[test]
    fn an_image_has_no_duration() {
        let probed = probe(&fixture("png-named.mp4")).unwrap();

        assert_eq!(probed.duration_ms, None);
        assert_eq!(probed.codec, None);
    }

    #[test]
    fn a_zero_or_oversized_dimension_is_refused() {
        for (width, height) in [(0, 16), (16, 0), (u64::from(u32::MAX) + 1, 16)] {
            for header in ["unreadable mp4 header", "unreadable webm header"] {
                let error = checked_dimensions(width, height, header).unwrap_err();

                assert!(
                    matches!(&error, AppError::Unsupported(reason) if *reason == format!("{header}: zero-sized video track")),
                    "unexpected error: {error:?}"
                );
            }
        }
    }

    #[test]
    fn a_zero_duration_is_no_duration() {
        assert_eq!(known_duration(0), None);
        assert_eq!(known_duration(200), Some(200));
    }

    #[test]
    fn an_mp4_whose_ftyp_box_looks_like_an_ico_is_still_an_mp4() {
        // A 256-byte `ftyp` box starts `00 00 01 00`, the ICO signature.
        let mut bytes = vec![0, 0, 1, 0];
        bytes.extend_from_slice(b"ftyp");

        let error = probe(&bytes).unwrap_err();

        assert!(matches!(error, AppError::Unsupported(_)), "{error:?}");
    }

    #[test]
    fn bytes_that_are_no_known_file_stay_a_decode_error() {
        let error = probe(b"just some text").unwrap_err();

        assert!(matches!(error, AppError::Decode(_)), "{error:?}");
    }
}
