## Why

The library takes still images only: an mp4 or webm dropped on the window is "skipped,
undecodable", though Danbooru — the site the tag system and the staging workflow descend from
— accepts both, and the owner's own material (the girlscreation `novel_script` clips: H.264
mp4, 1664×1024, ~8 s) has nowhere to live. Requirements §1 (the app is where the files live),
§6 (local file import is a source; a redesigned viewer with the lightbox as a feature to keep)
and the Danbooru-shaped tag search all assume "an image" means "a file Danbooru would take".
Owner, 2026-09-29: local import, playback in the viewer looping by default, muted; the
extension stays image-only; upload untouched.

## What Changes

- **mp4 and webm files import through every door** (local import, the capture listener, a
  legacy bundle if one ever carried one): sniffed by content, never by extension, and accepted
  only when the video track is one of H.264, HEVC, VP8 or VP9. Anything else is skipped with
  the codec named in the import report, so a file the app cannot show is refused at the door
  rather than stored as a black tile.
- **A video's facts come from its container header**: pixel dimensions and duration, with no
  decode. `duration_ms` is a new nullable column, carried by the sidecar and restored by a
  rebuild. One schema migration.
- **The grid tile shows the video's first frame as its thumbnail**, rendered by a minimal
  ffmpeg the app bundles as a Tauri sidecar: 3.2 MB per platform (measured 2026-09-29),
  decoders for exactly the four codecs above, JPEG out, LGPL, built from pinned source by a
  script the release workflow and the developer both run. The tile carries a duration badge.
- **The viewer plays a video** where it would show an image: looping, autoplaying, muted, with
  the platform's own controls for sound and seeking; fitted to the window like an image, no
  zoom. A file the machine's webview cannot decode shows a plain message instead of a blank.
- **Search:** `is:video`, `is:mp4`, `is:webm`. The inspector's Size row gains the duration.

## Capabilities

### New Capabilities

- `video-files`: which video files the library accepts and refuses, the facts a video records,
  the first-frame thumbnail and the bundled extractor that renders it, type search, the Size
  row.

### Modified Capabilities

- `local-file-import`: "Drop files or folders to import" accepts video files and names the
  codec of a refused one.
- `library-browse`: two added requirements — a video in the full-size view, and video tiles in
  the grid. The Lightbox requirement itself is unchanged: Space still closes the view, a click
  on the dark area still closes it; only the image-specific clauses (click-to-zoom, wheel,
  pinch) do not apply to a video.

## Non-goals

- Video capture from the bridge extension: its context menu is registered for images only,
  and a video cannot be read through the canvas path. Owner: image-only is enough. Noted so
  the "no Save entry on a video" question has its answer.
- Any change to booru upload. The stored MIME is sent as it is; Danbooru accepts mp4 and
  webm, and the owner tests it when they upload.
- Zoom, pan or click-to-zoom on a video; a poster frame other than the first; hover playback
  in the grid; sound on by default (owner: muted).
- AV1, AVIF, ugoira zips, GIF-to-video conversion, rotation metadata, audio-only files.
- Streaming reads on import and export: every door still reads the whole file into memory
  (a FIXME names it).

## Impact

- `packages/app/src-tauri`: `ingest` (probe replaces decode), `thumbs` (poster through the
  sidecar; one downscale-and-encode path for both kinds), `db` (migration), `model`, `sidecar`,
  `recover`, `import` (report reason), a new `media` module (sniff and header parse) and a new
  `ffmpeg` module (locate and run the sidecar). New crates: an mp4 header parser and a
  Matroska header parser. `tauri.conf.json` gains `bundle.externalBin`.
- `packages/shared`: `ImageRecord.durationMs`.
- `packages/app/src`: viewer, tile, inspector Size row, `tag-utils` `is:` values, `format`.
- Build and release: `scripts/build-ffmpeg.sh`, a `mise run ffmpeg` task, a sidecar step in
  `release.yml` and `windows-test-build.yml` (MSYS2 on Windows), a gitignored
  `packages/app/src-tauri/binaries/`. `cargo build` and `cargo test` need the sidecar binary
  present, so the script is a prerequisite of the gate.
- Bundle size: +3.2 MB per platform on today's 6.6 MB dmg and 5.1 MB Windows installer.
