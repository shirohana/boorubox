# pending-work Specification (delta)

## ADDED Requirements

### Requirement: A running import can be paused and cancelled
The tile showing a running import SHALL offer Pause and Cancel. While paused it SHALL offer
Resume and SHALL say that the run is paused rather than appear stalled. Pause, Resume and
Cancel SHALL take effect between two items: the item in flight SHALL be allowed to finish and
SHALL be counted. A waiting import SHALL offer Cancel and SHALL NOT offer Pause, having
nothing in flight to hold.

A waiting import's Cancel SHALL remove only that run from the queue, leaving the running
import and every other waiting run undisturbed. This is a different action from Cancel on the
*running* tile, which stops the run and discards every import queued behind it (see "Cancel
discards the runs queued behind it" below) — the two controls share a label, not a meaning.

Neither control SHALL wait for the running import to reach its end: a Cancel pressed during a
run of tens of thousands of items SHALL be acknowledged within one item.

#### Scenario: Pausing a run
- **WHEN** the user presses Pause on a running import
- **THEN** the run stops advancing within one item, the tile says it is paused, and Resume is offered

#### Scenario: Resuming a paused run
- **WHEN** the user presses Resume on a paused import
- **THEN** the run continues from the item after the last one counted, with its counts unchanged

#### Scenario: Cancelling a long run
- **WHEN** the user presses Cancel during an import of many thousands of items
- **THEN** the run ends within one item rather than at the end of the run

#### Scenario: Cancelling a waiting import
- **WHEN** the user presses Cancel on a waiting import while another import is running and a third is queued behind it
- **THEN** only the waiting import pressed is removed, and the running import and the third continue undisturbed

### Requirement: A cancelled run leaves a result, not an error
A cancelled import SHALL leave the same kind of result a finished run leaves, marked
cancelled, carrying the counts of what was imported, skipped and failed before it stopped.
It SHALL NOT be presented as a failure, and the images already imported SHALL remain in the
library.

#### Scenario: Result of a cancelled run
- **WHEN** an import is cancelled after 400 of 3,000 items
- **THEN** its result says it was cancelled and reports the 400 by imported, skipped and failed, and those images are in the library

#### Scenario: Cancelling is not failing
- **WHEN** a cancelled run's result is shown
- **THEN** it is not styled or worded as an error, and it offers no Retry that would repeat the whole run silently

### Requirement: Cancel discards the runs queued behind it
Cancelling the *running* import SHALL also discard every import queued behind it, and the
cancelled run's own result SHALL say how many queued runs were discarded. Cancelling a waiting
import (see above) SHALL NOT discard anything but itself, and SHALL NOT count towards this
total. Pausing SHALL NOT start a queued run.

#### Scenario: Cancel with work queued
- **WHEN** the user cancels a running import while two more are queued
- **THEN** the running import stops, neither queued run starts, and the result says two queued runs were discarded

#### Scenario: Paused with work queued
- **WHEN** an import is paused while another is queued
- **THEN** the queued run does not start until the paused run is resumed or cancelled
