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
/// every other one as `Unknown` without its fourcc or dimensions. For those,
/// `first_video_sample_entry` answers the one question left (what is the entry's type): HEVC
/// (`hvc1`/`hev1`) is accepted with its dimensions from the track header, any other type is
/// refused by name. The walk is not a second parser: it never runs on a codec `mp4parse` names.
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
    let named = match entry {
        Some(SampleEntry::Video(video)) => {
            let codec = match video.codec_type {
                CodecType::H264 => Some("avc1"),
                CodecType::VP9 => Some("vp09"),
                CodecType::MP4V => return Err(unsupported_codec("mp4v")),
                CodecType::AV1 => return Err(unsupported_codec("av01")),
                CodecType::VP8 => return Err(unsupported_codec("vp08")),
                CodecType::H263 => return Err(unsupported_codec("s263")),
                _ => None,
            };
            codec.map(|codec| (codec, u64::from(video.width), u64::from(video.height)))
        }
        _ => None,
    };
    let (codec, width, height) = match named {
        Some(named) => named,
        None => unnamed_video_entry(bytes, track)?,
    };
    let duration_ms = match (&track.duration, &track.timescale) {
        (Some(duration), Some(timescale)) if timescale.0 > 0 => {
            known_duration(scale_to_ms(duration.0 as u128, timescale.0 as u128))
        }
        _ => None,
    };
    let (width, height) = checked_dimensions(width, height, "unreadable mp4 header")?;
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

/// The codec and pixel size of a video entry `mp4parse` left `Unknown`: the entry's fourcc from
/// the box walk, the size from the track header (16.16 fixed point).
fn unnamed_video_entry(bytes: &[u8], track: &mp4parse::Track) -> Result<(&'static str, u64, u64)> {
    let Some(fourcc) = first_video_sample_entry(bytes) else {
        return Err(unsupported_codec("unknown"));
    };
    let codec = match &fourcc {
        b"hvc1" => "hvc1",
        b"hev1" => "hev1",
        other => {
            let name: String = other
                .iter()
                .map(|&byte| {
                    if byte.is_ascii_graphic() {
                        char::from(byte)
                    } else {
                        '?'
                    }
                })
                .collect();
            return Err(unsupported_codec(&name));
        }
    };
    let (width, height) = track
        .tkhd
        .as_ref()
        .map_or((0, 0), |tkhd| (tkhd.width >> 16, tkhd.height >> 16));
    Ok((codec, u64::from(width), u64::from(height)))
}

/// The type of the first sample entry of the first `vide` track (`moov` > `trak` > `mdia` >
/// `minf` > `stbl` > `stsd`), or `None` on any box that is missing or does not fit its parent.
fn first_video_sample_entry(bytes: &[u8]) -> Option<[u8; 4]> {
    let moov = child(bytes, b"moov")?;
    let mdia = boxes(moov)
        .filter(|(kind, _)| kind == b"trak")
        .find_map(|(_, trak)| {
            let mdia = child(trak, b"mdia")?;
            // hdlr payload: version and flags, pre_defined, then the handler type.
            (child(mdia, b"hdlr")?.get(8..12)? == b"vide").then_some(mdia)
        })?;
    let stsd = child(child(child(mdia, b"minf")?, b"stbl")?, b"stsd")?;
    // stsd is a full box: version and flags, entry count, then the entries.
    let (kind, _, _) = next_box(stsd.get(8..)?)?;
    Some(kind)
}

/// The boxes laid end to end in `data`; iteration stops at the first one that does not fit.
fn boxes(mut data: &[u8]) -> impl Iterator<Item = ([u8; 4], &[u8])> {
    std::iter::from_fn(move || {
        let (kind, payload, rest) = next_box(data)?;
        data = rest;
        Some((kind, payload))
    })
}

fn child<'a>(data: &'a [u8], kind: &[u8; 4]) -> Option<&'a [u8]> {
    boxes(data)
        .find(|(k, _)| k == kind)
        .map(|(_, payload)| payload)
}

/// The first box in `data` as its type, payload and the bytes after it. Size 1 is followed by a
/// 64-bit size, size 0 runs to the end of `data`.
fn next_box(data: &[u8]) -> Option<([u8; 4], &[u8], &[u8])> {
    let size = u32::from_be_bytes(data.get(..4)?.try_into().ok()?);
    let kind: [u8; 4] = data.get(4..8)?.try_into().ok()?;
    let (header, total) = match size {
        0 => (8, data.len()),
        1 => (
            16,
            usize::try_from(u64::from_be_bytes(data.get(8..16)?.try_into().ok()?)).ok()?,
        ),
        size => (8, usize::try_from(size).ok()?),
    };
    (total >= header).then_some(())?;
    Some((kind, data.get(header..total)?, data.get(total..)?))
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

    fn hevc_fixture_is_probed(name: &str, codec: &str) {
        let probed = probe(&fixture(name)).unwrap();

        assert_eq!(probed.kind, Kind::Video);
        assert_eq!((probed.ext, probed.mime), ("mp4", "video/mp4"));
        assert_eq!((probed.width, probed.height), (16, 16));
        assert_eq!(probed.codec, Some(codec));
        assert_within_a_millisecond(probed.duration_ms);
    }

    #[test]
    fn hvc1_mp4_is_probed_as_hevc_video() {
        hevc_fixture_is_probed("hvc1.mp4", "hvc1");
    }

    #[test]
    fn hev1_mp4_is_probed_as_hevc_video() {
        hevc_fixture_is_probed("hev1.mp4", "hev1");
    }

    #[test]
    fn an_unknown_fourcc_is_named_in_the_refusal() {
        let mut bytes = fixture("hvc1.mp4");
        let entry = bytes.windows(4).rposition(|w| w == b"hvc1").unwrap();
        bytes[entry..entry + 4].copy_from_slice(b"zzzz");

        let error = probe(&bytes).unwrap_err();

        assert!(
            matches!(&error, AppError::Unsupported(reason) if reason == "video codec zzzz is not supported"),
            "unexpected error: {error:?}"
        );
    }

    #[test]
    fn a_non_ascii_fourcc_prints_as_question_marks() {
        let mut bytes = fixture("hvc1.mp4");
        let entry = bytes.windows(4).rposition(|w| w == b"hvc1").unwrap();
        bytes[entry..entry + 4].copy_from_slice(&[b'a', 0xFF, 0x01, b'b']);

        let error = probe(&bytes).unwrap_err();

        assert!(
            matches!(&error, AppError::Unsupported(reason) if reason == "video codec a??b is not supported"),
            "unexpected error: {error:?}"
        );
    }

    #[test]
    fn a_truncated_moov_is_refused_not_panicked() {
        let bytes = fixture("hvc1.mp4");
        let moov = bytes.windows(4).position(|w| w == b"moov").unwrap();

        for cut in [moov + 4, moov + 40, bytes.len() - 1] {
            let error = probe(&bytes[..cut]).unwrap_err();

            assert!(matches!(error, AppError::Unsupported(_)), "{error:?}");
        }
    }

    #[test]
    fn the_walk_answers_none_for_garbage() {
        assert_eq!(first_video_sample_entry(&[]), None);
        assert_eq!(first_video_sample_entry(&[0xFF; 64]), None);
        assert_eq!(first_video_sample_entry(b"\0\0\0\x01moov"), None);
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
