## Purpose

Users of the legacy browser extension move their images into the app through its Phase 0
export bundle, with a report they can check against the browser before deleting anything.

## ADDED Requirements

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
