## MODIFIED Requirements

### Requirement: Drop files or folders to import
The app SHALL accept image and video files and folders dropped onto the library window, or
chosen via a file dialog, and SHALL import every decodable image and every accepted video
found (`video-files`), recursing into folders. A file that is neither SHALL be skipped and
named in the result; a video refused for its codec SHALL be named with the codec.

#### Scenario: Mixed drop
- **WHEN** the user drops two image files and a folder containing three images, one mp4 and one text file
- **THEN** six items are imported, the text file is skipped, and the result names the skipped file

#### Scenario: A refused video is named with its reason
- **WHEN** a folder holding one H.264 mp4 and one MPEG-4 Part 2 mp4 is dropped
- **THEN** one video is imported, the other is skipped, and the result names it with its codec

#### Scenario: Progress on a large drop
- **WHEN** more than 50 files are dropped
- **THEN** the UI stays responsive and shows a running imported count until done
