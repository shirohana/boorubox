# legacy-bundle-import Specification (delta)

## ADDED Requirements

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
