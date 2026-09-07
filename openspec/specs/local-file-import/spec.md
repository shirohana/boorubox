# local-file-import Specification

## Purpose
Plain image files already on disk become library images without going through a browser.

## Requirements

### Requirement: Drop files or folders to import
The app SHALL accept image files and folders dropped onto the library window, or chosen via a
file dialog, and SHALL import every decodable image found, recursing into folders.

#### Scenario: Mixed drop
- **WHEN** the user drops two image files and a folder containing three images and one text file
- **THEN** five images are imported, the text file is skipped, and the result names the skipped file

#### Scenario: Progress on a large drop
- **WHEN** more than 50 files are dropped
- **THEN** the UI stays responsive and shows a running imported count until done

### Requirement: Originals are never moved
Import SHALL copy the file into the library and SHALL NOT delete, move or modify the original.

#### Scenario: Import from a removable drive
- **WHEN** a file on another volume is imported
- **THEN** the original is unchanged and the library copy renders after the volume is unplugged

### Requirement: Metadata comes from the file
An imported image SHALL record the filename as title, the file's modification time as capture
time, its pixel dimensions, size and MIME type, and `source=local`.

#### Scenario: Metadata visible
- **WHEN** a file `cat.png` last modified 2026-01-02 is imported
- **THEN** its card shows the title `cat.png` and the capture time 2026-01-02

### Requirement: Duplicate content is imported as a new image
Phase 1 SHALL NOT deduplicate by content; each import creates a new id.

#### Scenario: Same file twice
- **WHEN** the same file is imported twice
- **THEN** two images exist, each with its own id

### Requirement: Imports started during a run are queued
An import started while another is running — by a drop or from the import menu — SHALL be
queued and run after it, in the order started, and SHALL NOT be refused or silently dropped.
The controls that start an import SHALL stay available while a run is going. Each run SHALL
produce its own result.

#### Scenario: Drop during a run
- **WHEN** files are dropped while an import is running
- **THEN** they are imported after the running import finishes, and both runs report their own result

#### Scenario: Menu during a run
- **WHEN** the user opens the import menu while an import is running
- **THEN** the menu's entries can be chosen and what they pick is queued

#### Scenario: Leaving the screen mid-run
- **WHEN** the user leaves the library screen while an import runs with another queued
- **THEN** both still run to completion and their results are shown on return
