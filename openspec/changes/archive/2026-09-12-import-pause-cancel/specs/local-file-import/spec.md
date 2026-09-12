# local-file-import Specification (delta)

## ADDED Requirements

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

## MODIFIED Requirements

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
