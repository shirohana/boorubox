# library-switching Specification (delta)

## ADDED Requirements

### Requirement: Swapping the library confirms before it stops an import
Every action that puts a different library behind the app — opening another from the switch
menu, picking a folder, choosing one on the start screen, and closing the open library —
SHALL ask the user first when an import is running or waiting. The question SHALL say that
the running import will be cancelled, that what it has already imported stays in the library
it imported into, and what running it again would cost for that kind of import: a bundle
import carries on from where it stopped, a local file import imports everything a second
time. When imports are waiting behind the running one, the question SHALL say how many will
be discarded.

Declining SHALL leave the import running, the queue intact and the open library unchanged.
Confirming SHALL stop the running import and discard the queue before the other library is
opened, so no waiting run is ever carried into a library it was not started for.

With no import running or waiting, these actions SHALL NOT ask anything.

#### Scenario: Switching during a migration
- **WHEN** the user opens another library from the switch menu while a bundle import is running
- **THEN** the app asks first, saying the run will be cancelled and that selecting the same parts again carries on from where it stopped

#### Scenario: Declining
- **WHEN** the user declines that question
- **THEN** the import is still running with its counts unbroken, and the same library is still open

#### Scenario: Confirming with work queued
- **WHEN** the user confirms a switch while one import runs and two wait behind it
- **THEN** the question said two waiting imports would be discarded, the running import stops with its own result, neither waiting import starts, and the new library is opened empty of them

#### Scenario: Closing during an import
- **WHEN** the user closes the library while a local file import is running
- **THEN** the app asks first and says that importing the same files again would import them again

#### Scenario: Nothing in flight
- **WHEN** the user switches or closes the library with no import running or waiting
- **THEN** the library changes at once, with no question asked
