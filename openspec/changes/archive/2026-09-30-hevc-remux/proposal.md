## Why

An HEVC mp4 tagged `hev1` imports (`hevc-mp4`) but does not play on the Mac: Apple's decoder
path, which WebKit uses, plays HEVC only as `hvc1`, with the parameter sets in the container.
A lossless remux (`-c copy -tag:v hvc1`) takes 0.15 s on a 50 MB clip, keeps every byte of
video, and the remuxed clip plays in the app on the Mac (checked 2026-09-29). On Windows the
same clip plays audio only: Edge's engine, which the webview is, decodes HEVC only through
Microsoft's paid Store package, and the viewer shows a blank because no error event fires
(audio plays). The owner will not ask users to pay; a playback sample is planned separately
(`hevc-samples`, gated on the `PlatformHEVCDecoderSupport` experiment). This change makes the
Mac play `hev1` files and makes Windows say why it cannot. Requirements §6 (viewer).

## What Changes

- **An mp4 whose video entry is `hev1` (or `avc3`, H.264's in-band form) is remuxed at import
  to `hvc1` (`avc1`)** by the bundled sidecar, stream-copied, with `faststart`. The stored
  file is the remuxed one; the original on disk is untouched. A remux the sidecar cannot do
  fails the import with ffmpeg's own last line.
- **The record carries its video codec** (`codec` column, `hvc1` / `avc1` / `vp08` / `vp09`,
  null for an image), in the sidecar and restored by a rebuild. One schema migration.
- **The viewer asks the webview whether it decodes the record's codec before playing.** When
  it does not, the view shows a message naming the codec and, on Windows, that the system's
  browser engine has no HEVC decoder, instead of a blank or audio alone.
- The sidecar gains the mp4 muxer (measured: no size growth).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `video-files`: "Accepted video files" (the remux), "A video's facts" (the codec).
- `library-browse`: "A video in the full-size view" (the decoder check and its message).

## Non-goals

- Transcoding (`hevc-samples`, separate change, after the flag experiment).
- `faststart` for every imported mp4 (only remuxed files get it).
- Remuxing webm.

## Impact

`scripts/build-ffmpeg.sh` (mp4 muxer, two bitstream filters), `packages/app/src-tauri`
(`ffmpeg.rs` remux, `ingest.rs`, `media.rs`, `db.rs` v16, `model.rs`, `sidecar.rs`,
`recover.rs`), `packages/shared` (`codec`), `packages/app/src` (viewer). Bundle unchanged.
