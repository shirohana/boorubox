## Purpose

The library screen shows what is on its way — captures announced but not yet stored, imports
running or waiting — immediately and where the result will land, so the user can tell an
action registered before its image exists.

## ADDED Requirements

### Requirement: Work in flight is shown where the result will appear
The library screen SHALL show every capture the app has been told is coming and every import
run that is running or waiting, as tiles at the grid's tile size placed above the grid's first
row, and SHALL show a new one within the same interaction that started it, before any bytes
have arrived. The tiles SHALL be shown whether or not the library holds images and whether or
not the current search has results, and SHALL still be shown when the user leaves the library
screen and comes back while the work is going.

#### Scenario: Capture announced
- **WHEN** the app is told a capture is coming
- **THEN** a placeholder tile bearing the page's title appears above the grid at once, before the image is stored

#### Scenario: Capture stored
- **WHEN** an announced capture is stored
- **THEN** its placeholder is gone and the image is in the grid, without the user doing anything

#### Scenario: Capture withdrawn
- **WHEN** an announced capture is withdrawn or refused
- **THEN** its placeholder is gone and the grid is unchanged

#### Scenario: Empty library
- **WHEN** a capture is announced or an import started while the library holds no images
- **THEN** the tile is shown above the empty state

#### Scenario: Back from another screen
- **WHEN** the user opens settings while an import runs or a capture is pending and returns to the library
- **THEN** the tiles are still shown, with the import's progress moved on

### Requirement: Pending work is not library content
A pending capture SHALL NOT create an image, change any count, or appear in any search until
its bytes are stored. A pending capture that is neither stored nor withdrawn SHALL be dropped
from the screen after a bounded time rather than kept for ever.

#### Scenario: Browser closed mid-download
- **WHEN** a capture is announced and the browser is closed before its bytes are posted
- **THEN** no image and no count changes, and the placeholder goes away on its own once the bound is passed

#### Scenario: Searching while a capture is pending
- **WHEN** the user runs a search while a placeholder is shown
- **THEN** the results and the frame's image count do not include the pending capture

### Requirement: An import run is shown with its progress
A running import SHALL be shown as one tile with the number of files done of the total and
the number imported, moving as the run goes, or saying that the files are still being counted
before the total is known. A waiting import SHALL be shown as one tile saying it waits and how
many dropped or chosen items it holds. Each finished run SHALL leave its own result, kept
until dismissed.

#### Scenario: Large drop
- **WHEN** a folder of many files is dropped
- **THEN** a tile appears at once, first saying the files are being counted, then showing done of total and the imported count until the run finishes

#### Scenario: Two drops
- **WHEN** a second folder is dropped while the first is importing
- **THEN** a second tile appears saying it waits and how many items it holds, becomes the running tile when the first finishes, and each run leaves its own result

### Requirement: The window stays responsive while work is in flight
The library screen SHALL keep scrolling, repainting and answering input while captures are
being stored and while an import runs, however many are in flight. No action on the screen
SHALL wait for a whole import to finish; a search or a thumbnail asked for during an import
SHALL be answered between two of its files.

#### Scenario: Several captures storing at once
- **WHEN** three captures arrive within a second of each other
- **THEN** the grid scrolls and repaints throughout, and each image appears as it is stored

#### Scenario: Scrolling during an import
- **WHEN** the user scrolls the grid while a large import runs
- **THEN** thumbnails load as the rows come into view, and the import keeps going
