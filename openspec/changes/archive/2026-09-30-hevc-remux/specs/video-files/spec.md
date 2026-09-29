## MODIFIED Requirements

### Requirement: Accepted video files
The library SHALL accept a video file through every door an image comes through — local
import, the capture listener, a legacy bundle — when its content is an MP4 container whose
video track is H.264 or HEVC, or a WebM container whose video track is VP8, VP9, H.264 or
HEVC. Acceptance SHALL be decided from the file's bytes, never its name. An MP4 whose video
entry keeps its parameter sets in the stream (`hev1`, `avc3`) SHALL be stored as the same
stream rewritten with the parameter sets in the container (`hvc1`, `avc1`), without
re-encoding; the file on disk the user imported SHALL be untouched; a file the rewrite cannot
handle SHALL fail the import with the tool's own reason. A file in either container whose
video track is any other codec, or that has no video track, SHALL be refused, and the refusal
SHALL name the codec by the container's own code for it (or the absence of a video track) so
the user can tell a refused file from a corrupt one. A refused file SHALL leave no file, no
part file and no row behind.

#### Scenario: An H.264 mp4 is imported
- **WHEN** an mp4 with an H.264 video track is dropped on the window
- **THEN** it is imported, stored under its own id with the extension `mp4` and the MIME type `video/mp4`

#### Scenario: An HEVC mp4 is imported
- **WHEN** an mp4 whose video track is HEVC, tagged `hvc1` or `hev1`, is dropped
- **THEN** it is imported with the extension `mp4`, the MIME type `video/mp4`, its dimensions and duration, and its tile shows its first frame

#### Scenario: An hev1 mp4 is stored as hvc1
- **WHEN** an mp4 whose video entry is `hev1` is imported
- **THEN** the stored file's video entry is `hvc1`, its video stream is byte-identical, the record's codec reads `hvc1`, and the imported file on disk is unchanged

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
An accepted video SHALL record its pixel dimensions, its duration and its video codec, read
from the container header without decoding a frame. The duration and the codec SHALL be part
of the image's sidecar and SHALL survive a rebuild of the index from the sidecars. An image
(not a video) SHALL have no duration and no codec.

#### Scenario: Dimensions and duration are stored
- **WHEN** a 1664×1024 mp4 lasting 8.03 s is imported
- **THEN** its record reads 1664 by 1024 and a duration of 8033 ms

#### Scenario: The codec is stored
- **WHEN** an H.264 mp4 and a VP9 webm are imported
- **THEN** their records read the codecs `avc1` and `vp09`

#### Scenario: A rebuild keeps the duration
- **WHEN** the index is rebuilt from the sidecars
- **THEN** the video's duration and codec are the same as before the rebuild
