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
An imported image SHALL record the filename as title, the moment it was imported as its
capture time, the file's own modification time as a fact of its own, its pixel dimensions,
size and MIME type, and `source=local`. Both times SHALL be visible in the inspector, named
apart, and the file's modification time SHALL be absent rather than invented when the
filesystem cannot report one. An image that never came from a file SHALL have no modification
time.

**Why the capture time changed**: the file's modification time was the capture time because
the import this app was designed around is a one-off bulk migration of an old folder, where
the mtimes are the only record of the order the pictures arrived in and using them preserves
it. It stopped being the right column once the app became the one in daily use: an image
imported today gets whatever time its file happens to carry, so it lands somewhere in the
middle of the default "newest capture first" grid instead of at the front, and the user has to
search for the thing they just added. A migration that loses its order costs one sort; an added
image landing at random costs a hunt every time. The order the mtimes carry is not lost — the
modification time is still read and still stored, now under its own name, where an ordering
that wants it can read it.

#### Scenario: Metadata visible
- **WHEN** a file `cat.png` last modified 2026-01-02 is imported on 2026-09-08
- **THEN** its card shows the title `cat.png`, its capture time is 2026-09-08, and the inspector shows the file's modification time as 2026-01-02

#### Scenario: A freshly imported file is at the front
- **WHEN** a file whose modification time is two years old is imported into a library sorted newest capture first
- **THEN** it is among the first tiles of the grid, not among the images captured two years ago

#### Scenario: No modification time to be had
- **WHEN** the filesystem cannot report a modification time for an imported file
- **THEN** the image is still imported with the import time as its capture time, and its modification time is shown as absent

#### Scenario: An image that is not a file import
- **WHEN** an image captured by the browser extension is inspected
- **THEN** its capture time is the time of the capture and it carries no file modification time

### Requirement: Duplicate content is imported as a new image
Phase 1 SHALL NOT deduplicate by content; each import creates a new id.

#### Scenario: Same file twice
- **WHEN** the same file is imported twice
- **THEN** two images exist, each with its own id

### Requirement: Imports started during a run are queued
An import started while another is running — by a drop or from the import menu — SHALL be
queued and run after it, in the order started, and SHALL NOT be refused or silently dropped.
The controls that start an import SHALL stay available while a run is going. Each run SHALL
produce its own result. Cancelling the running import SHALL discard the queue rather than
start the next run; a paused import SHALL hold the queue until it is resumed or cancelled.

#### Scenario: Drop during a run
- **WHEN** files are dropped while an import is running
- **THEN** they are imported after the running import finishes, and both runs report their own result

#### Scenario: Menu during a run
- **WHEN** the user opens the import menu while an import is running
- **THEN** the menu's entries can be chosen and what they pick is queued

#### Scenario: Leaving the screen mid-run
- **WHEN** the user leaves the library screen while an import runs with another queued
- **THEN** both still run to completion and their results are shown on return

#### Scenario: Cancelling with a queued run
- **WHEN** the user cancels the running import while another is queued
- **THEN** the queued run does not start and the result says it was discarded

### Requirement: The drop target answers external drags only
The app SHALL show its "drop to import" target only for a drag that carries files from outside
the app, and SHALL NOT show it while the user drags something inside the app's own window.
Dragging an image in the library SHALL NOT be offered as a way to import it.

#### Scenario: Dragging a thumbnail
- **WHEN** the user presses on a thumbnail in the grid and drags it across the window
- **THEN** no import overlay appears and nothing is imported

#### Scenario: Dragging a file in from outside
- **WHEN** the user drags an image file from the file manager over the window
- **THEN** the import overlay appears and releasing it imports the file

### Requirement: Re-running a cancelled local import imports everything again
A local file import mints a fresh id for every file, so a run started again over the same
files SHALL import them again rather than resume. Where a cancelled local import's result is
shown, the app SHALL say so in words, before the user chooses to run it again. The app SHALL
NOT offer a "Resume" for a local import.

**Why this is said out loud**: the other importer in the app resumes, because bundle rows
carry their own ids. A user who has learned that behaviour from a migration would reasonably
expect it here and end up with every file twice.

#### Scenario: Cancelled local import re-run
- **WHEN** a local import of 100 files is cancelled after 40 and the same folder is imported again
- **THEN** the 40 already imported are imported a second time, and the library holds them twice

#### Scenario: The warning is shown, not buried
- **WHEN** a local import's result says it was cancelled
- **THEN** the result says that importing the same files again will import them again, and offers no Resume
