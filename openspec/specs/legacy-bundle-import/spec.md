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
