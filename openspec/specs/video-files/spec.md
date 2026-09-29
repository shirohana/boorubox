# video-files Specification

## Purpose
Which video files the library accepts and refuses, the facts a video records, the first-frame
thumbnail and the extractor the app carries to render it, type search, and the Size row.
Requirements §1 (the app is where the files live), §6 (local file import; the viewer).

## Requirements

### Requirement: Accepted video files
The library SHALL accept a video file through every door an image comes through — local
import, the capture listener, a legacy bundle — when its content is an MP4 container whose
video track is H.264 or HEVC, or a WebM container whose video track is VP8, VP9, H.264 or
HEVC. Acceptance SHALL be decided from the file's bytes, never its name. A file in either
container whose video track is any other codec, or that has no video track, SHALL be refused,
and the refusal SHALL name the codec by the container's own code for it (or the absence of a
video track) so the user can tell a refused file from a corrupt one. A refused file SHALL
leave no file, no part file and no row behind.

#### Scenario: An H.264 mp4 is imported
- **WHEN** an mp4 with an H.264 video track is dropped on the window
- **THEN** it is imported, stored under its own id with the extension `mp4` and the MIME type `video/mp4`

#### Scenario: An HEVC mp4 is imported
- **WHEN** an mp4 whose video track is HEVC, tagged `hvc1` or `hev1`, is dropped
- **THEN** it is imported with the extension `mp4`, the MIME type `video/mp4`, its dimensions and duration, and its tile shows its first frame

#### Scenario: A VP9 webm is imported
- **WHEN** a webm with a VP9 video track is dropped
- **THEN** it is imported with the extension `webm` and the MIME type `video/webm`

#### Scenario: A codec the app cannot show is refused by name
- **WHEN** an mp4 whose video track is MPEG-4 Part 2 (or AV1, or any codec outside the set) is dropped
- **THEN** it is skipped, and the import report's reason names that codec by its four-character code

#### Scenario: An audio-only file is refused
- **WHEN** an mp4 with an audio track and no video track is dropped
- **THEN** it is skipped, and the reason says the file has no video track

#### Scenario: The name does not decide
- **WHEN** a PNG renamed to `.mp4` is dropped
- **THEN** it is imported as the PNG it is

### Requirement: A video's facts
An accepted video SHALL record its pixel dimensions and its duration, read from the container
header without decoding a frame. The duration SHALL be part of the image's sidecar and SHALL
survive a rebuild of the index from the sidecars. An image (not a video) SHALL have no
duration.

#### Scenario: Dimensions and duration are stored
- **WHEN** a 1664×1024 mp4 lasting 8.03 s is imported
- **THEN** its record reads 1664 by 1024 and a duration of 8033 ms

#### Scenario: A rebuild keeps the duration
- **WHEN** the index is rebuilt from the sidecars
- **THEN** the video's duration is the same as before the rebuild

### Requirement: First-frame thumbnail
A video's thumbnail SHALL be its first frame, rendered at the same longest edge and in the same
format as an image's thumbnail, generated on the same occasions (after import, on demand when
the grid first asks, and by the regenerate pass). The app SHALL carry the frame extractor it
needs inside its own bundle on both platforms, so a machine with nothing else installed renders
the thumbnail. A failure to render the frame SHALL NOT fail the import: the video is stored and
its tile shows no preview, as an image whose thumbnail failed does.

#### Scenario: The grid shows the first frame
- **WHEN** a video is imported and the grid scrolls to its tile
- **THEN** the tile shows the video's first frame

#### Scenario: Regenerate re-renders posters
- **WHEN** the user runs the thumbnail regenerate pass
- **THEN** every video's thumbnail is rendered again, and one that cannot be rendered counts as failed in the pass's report

### Requirement: Search by type
`is:video` SHALL match every video; `is:mp4` and `is:webm` SHALL match by container. They SHALL
combine with the other `is:` values as those already do with each other.

#### Scenario: is:video
- **WHEN** the query is `is:video`
- **THEN** every mp4 and webm in the library matches and no image does

#### Scenario: is:webm beside is:png
- **WHEN** the query is `is:webm is:png`
- **THEN** every webm and every PNG matches

### Requirement: The Size row shows the duration
The inspector's Size row for a video SHALL append the duration, as minutes and seconds, to the
dimensions: `1.8 MB .mp4 (1664×1024, 0:08)`. An image's Size row SHALL be unchanged.

#### Scenario: A video's Size row
- **WHEN** the inspector shows an mp4 of 1,813,982 bytes, 1664×1024, 8033 ms
- **THEN** its Size row reads `1.8 MB .mp4 (1664×1024, 0:08)`
