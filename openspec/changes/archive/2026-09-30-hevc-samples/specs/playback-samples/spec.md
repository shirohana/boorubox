## Purpose

A derived H.264 copy of a video for playback on a machine whose browser engine cannot decode
the original, kept as a cache beside the thumbnails. Requirements §6 (the viewer), §7
(derived data beside the files).

## ADDED Requirements

### Requirement: A playback sample is made only where it is needed
A playback sample SHALL be made only when the full-size view is asked to play a video whose
codec the machine's browser engine reports it cannot decode. A machine whose engine decodes
the original SHALL never make one. The sample SHALL be an H.264 mp4 copy of the stored file,
about the size of its source, with its audio; the stored file, the tile thumbnail, export and
booru upload SHALL be unaffected.

#### Scenario: A Windows machine without an HEVC decoder
- **WHEN** the view opens an HEVC clip on a machine whose engine cannot decode HEVC
- **THEN** a sample is made once, and the view plays it now and on every later opening without converting again

#### Scenario: A machine that decodes HEVC
- **WHEN** the view opens the same clip on a machine whose engine decodes HEVC
- **THEN** the original plays and no sample is made

### Requirement: The sample is a cache
Samples SHALL live under `.samples/` in the library folder, bucketed like thumbnails, SHALL be
deleted with their record, SHALL be safe to delete by hand at any time, and SHALL be clearable
from Settings, which reports how many were removed and how much space they held. A rebuild
SHALL neither need nor restore them.

#### Scenario: Clearing samples
- **WHEN** the user clears playback samples in Settings
- **THEN** every sample is deleted, the count and size are shown, and the next opening of an undecodable clip converts it again
