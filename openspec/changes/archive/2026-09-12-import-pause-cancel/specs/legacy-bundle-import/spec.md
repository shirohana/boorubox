# legacy-bundle-import Specification (delta)

## ADDED Requirements

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
