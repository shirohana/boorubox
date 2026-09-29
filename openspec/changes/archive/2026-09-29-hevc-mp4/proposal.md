## Why

`video-files` accepts H.264 in an mp4 and refuses HEVC there as `video codec unknown`, because
the header parser in use has no HEVC entry (its design D1, amended at apply time). The owner has
H.265 clips on Windows and asked for them on 2026-09-29, AVI explicitly excluded. The bundled
extractor already decodes HEVC, so nothing changes in the sidecar; only the import door does.
Requirements §6 (local file import).

## What Changes

- **An mp4 whose video track is `hvc1` or `hev1` imports** with its dimensions from the track
  header, and gets a first-frame thumbnail like any other video.
- **A refused mp4 codec is named by its own four-character code** where the parser could not
  name it (`video codec xxxx is not supported`), never `unknown`, except for a file whose
  sample entry cannot be read at all.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `video-files`: "Accepted video files" — HEVC in an mp4 is accepted; the refusal names the
  fourcc.

## Non-goals

- AVI / MPEG-4 Part 2 (owner, 2026-09-29: not now; estimate recorded in the backlog).
- Transcoding; a playback guarantee on Windows (WebView2 plays HEVC only with Microsoft's HEVC
  Video Extensions installed — the viewer's "cannot be played here" message covers it).
- Rotation metadata.

## Impact

`packages/app/src-tauri/src/media.rs` only, plus two fixtures and tests. No bundle change.
