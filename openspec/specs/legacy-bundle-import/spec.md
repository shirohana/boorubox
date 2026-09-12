# legacy-bundle-import Specification

## Purpose
Users of the legacy browser extension move their images into the app through its Phase 0
export bundle, with a report they can check against the browser before deleting anything.

## Requirements

### Requirement: Bundle import produces a per-item report
The app SHALL import a Phase 0 bundle and SHALL end with a report listing, per item, one of
`imported`, `skipped` (id already in the library) or `failed` with a reason. The report SHALL
be shown before the counts.

#### Scenario: Clean import
- **WHEN** a bundle with 100 items is imported into an empty library
- **THEN** the report shows 100 imported, 0 skipped, 0 failed and the library holds 100 images with `source=legacy-bundle`

#### Scenario: Re-import
- **WHEN** the same bundle is imported again
- **THEN** every item is `skipped` and the library still holds 100 images

#### Scenario: Corrupt item
- **WHEN** one item's image data cannot be decoded
- **THEN** that item is `failed` with the reason, the other items are imported, and the import does not abort

### Requirement: Trash state is preserved
Items marked deleted in the bundle SHALL be imported as deleted, not as live images and not
dropped.

#### Scenario: Trashed item
- **WHEN** a bundle item has `isDeleted` set
- **THEN** the image is stored with a deletion time and does not appear in the default grid

### Requirement: Tags and rating carry over
Each item's tags SHALL become tag rows on the image and its rating SHALL be kept.

#### Scenario: Tagged item
- **WHEN** an item has tags `a, b` and rating `s`
- **THEN** searching `a b rating:s` finds it

### Requirement: The app never claims the browser is safe to clear
After import the app SHALL show the total image count and the count with
`source=legacy-bundle`, together with the notice text from docs/requirements.md §9 step 4,
and SHALL NOT use wording that says the browser data is safe to delete.

#### Scenario: Counts shown
- **WHEN** an import finishes
- **THEN** the total and the legacy-bundle count are shown next to the report with the §9 notice

### Requirement: A cancelled bundle import resumes when run again
A bundle row keeps the id it had in the export, and a row whose id is already in the library
is skipped without being decoded. Selecting the same parts again after a cancelled run
SHALL therefore carry on from where it stopped rather than duplicate anything, and the app
SHALL say so where a cancelled bundle import's result is shown.

The parts SHALL be processed in a fixed order — by part number, then by row — so that the
prefix a re-run skips is the prefix the cancelled run imported, whatever order the file
picker handed the parts over in.

#### Scenario: Cancelled migration continued
- **WHEN** a 25,000-row bundle import is cancelled after 8,000 rows and the same parts are selected again
- **THEN** the first 8,000 rows are reported skipped as already in the library, the run continues from row 8,001, and nothing is imported twice

#### Scenario: Resuming is cheap
- **WHEN** a re-run passes over rows already in the library
- **THEN** those rows are skipped without decoding their images, so the skipped prefix costs far less than importing it did

#### Scenario: The user is told it resumes
- **WHEN** a bundle import's result says it was cancelled
- **THEN** the result says that selecting the same parts again will carry on from where it stopped

### Requirement: Picked bundle parts are shown and confirmed before anything is imported
Choosing bundle files SHALL NOT start an import. The app SHALL first show every part it
would read, in the order the run will read them, each named by its file and carrying either
the number of rows it holds or the reason it could not be opened, together with the total
number of items the run would produce. The user SHALL then choose between starting the
import and discarding the pick, and SHALL be able to choose more files instead; nothing
SHALL be written to the library until the import is chosen.

The counts shown SHALL be the counts the run itself will work through, so the total on this
screen and the total the run reports are the same number.

A part that cannot be opened SHALL be shown with its reason and SHALL NOT block the others:
the import may still be started, and that part becomes one failed item exactly as it does
today.

#### Scenario: Parts listed before the run
- **WHEN** the user picks `database-part2of18.db` and `database-part1of18.db`
- **THEN** part 1 is listed before part 2, each with its own row count and their sum, and no image has been imported

#### Scenario: Confirming the pick
- **WHEN** the user starts the import from that list
- **THEN** the run imports exactly the parts listed, in the order listed, and its total matches the total that was shown

#### Scenario: Discarding the pick
- **WHEN** the user discards the pick instead
- **THEN** nothing is imported, nothing is queued, and the screen is back to offering the file picker

#### Scenario: An unopenable part among good ones
- **WHEN** one picked file is not a readable bundle part
- **THEN** it is listed with the reason it could not be opened, the other parts still show their row counts, and starting the import is still offered

#### Scenario: Nothing picked
- **WHEN** the user cancels the file dialog
- **THEN** nothing is listed, nothing is imported, and the screen is unchanged

### Requirement: A finished bundle report is cleared when the user is done with it
After a bundle run's report has been read, the app SHALL offer to dismiss it, and dismissing
it SHALL return the screen to the state it starts in — offering the file picker, with no
report and no pick. The library count, the legacy-bundle count and the §9 notice SHALL stay
on the screen after the report is dismissed, since they are what the browser is compared
against and they outlive any one run.

#### Scenario: Done with the report
- **WHEN** the user dismisses a finished bundle report
- **THEN** the report is gone, the file picker is offered again, and nothing that was imported is affected

#### Scenario: The counts survive the report
- **WHEN** a bundle report is dismissed
- **THEN** the total and legacy-bundle counts and the notice are still shown
