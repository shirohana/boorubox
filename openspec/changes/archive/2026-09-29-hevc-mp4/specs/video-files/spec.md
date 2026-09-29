## MODIFIED Requirements

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

#### Scenario: HEVC in an mp4 is refused for now
- **WHEN** an mp4 whose video track is HEVC (`hvc1`) is dropped
- **THEN** it is imported (the refusal this scenario recorded under `video-files` is reversed by `hevc-mp4`; the main spec drops this scenario at archive time and keeps "An HEVC mp4 is imported")

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
