## Why

On Windows the app's webview is Edge's engine, which decodes HEVC only through Microsoft's paid
Store package plus a hardware decoder; the owner's machine decodes HEVC in Chrome but not in
Edge, the free package is delisted, and Chromium's platform-HEVC switch passed to the webview
changed nothing (experiment build 36597272757, 2026-09-30). The owner will not ask users to
pay. So an HEVC clip that imports and plays on the Mac shows only a message on Windows
(`hevc-remux`). Owner's go, 2026-09-30: a derived H.264 playback copy, made only where the
engine cannot decode the original, with an explicit "converting, please wait"; doubling the
storage of an HEVC clip is accepted. Requirements §6 (the viewer), §7 (derived caches beside
the files).

## What Changes

- **A playback sample**: `.samples/<a1>/<id>.mp4`, an H.264 copy of a video, made by the
  bundled sidecar with the operating system's own H.264 encoder (Media Foundation on
  Windows, VideoToolbox on macOS) the first time the viewer needs it, kept as a cache like
  `.thumbs/`, deleted with the record and clearable from Settings.
- **The viewer converts instead of refusing**: when the engine says it cannot decode the
  record's codec, the view says it is converting the video because this machine's browser
  engine cannot decode it, shows progress, then plays the sample. A machine that decodes the
  original never makes a sample. The original file, the tile poster, export and upload are
  untouched.
- **The sidecar gains the two OS encoders, the AAC encoder and the mp4 audio decoders.**

## Capabilities

### New Capabilities

- `playback-samples`: when a sample is made, where it lives, what it is made from, when it
  is deleted, and that a machine that decodes the original never makes one.

### Modified Capabilities

- `library-browse`: "A video in the full-size view" — the converting state replaces the
  refusal message for a codec the engine cannot decode.

## Non-goals

- Transcoding at import; samples for webm (VP8/VP9 play in both engines); cancelling a
  conversion (a FIXME names it); a quality setting; playing the sample where the original
  plays.

## Impact

`scripts/build-ffmpeg.sh` and both workflows (encoder flags per platform); `packages/app/
src-tauri` (`samples.rs`, a `playback_sample` command with progress events, asset scope,
trash/drop, a `clear_samples` command); `packages/app/src` (viewer state, Settings button).
Sidecar grows by well under 1 MB (measured with more than this: 3.97 MB total).
