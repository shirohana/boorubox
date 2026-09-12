# library-switching Specification

## Purpose
The user keeps more than one library folder and moves between them without relaunching the
app. Exactly one is open at a time; the app remembers the ones it has opened and offers them
on the start screen.

## Requirements

### Requirement: The start screen lists recently opened libraries
With no library open, the app SHALL list the libraries it has opened before, most recent
first, each with its folder name and its full path, alongside the action that picks a folder
the app has not seen. Choosing an entry SHALL open that library.

#### Scenario: Nothing opened yet
- **WHEN** the app starts for the first time
- **THEN** the start screen offers only picking a folder, with no empty list and no placeholder rows

#### Scenario: Two libraries seen
- **WHEN** the user has opened folder A and then folder B
- **THEN** the start screen lists B first and A second, each with its name and path

#### Scenario: Opening from the list
- **WHEN** the user chooses an entry whose folder is intact
- **THEN** that library opens, the library screen is shown, and the entry moves to the front of the list

### Requirement: At most one library is open
Opening a library SHALL close the one that is open first, so the app never holds two libraries
at once. Everything reading the library — the screen, the counts and the capture listener —
SHALL follow the switch.

#### Scenario: Switching
- **WHEN** a library is open and the user opens another
- **THEN** the grid, the counts and the library name show the new library and nothing from the old one

#### Scenario: A capture arriving after a switch
- **WHEN** a capture is delivered after the user switched libraries
- **THEN** it is stored in the library that is now open

### Requirement: Closing a library returns to the start screen
The app SHALL offer closing the open library. After closing, the start screen SHALL be shown,
the closed folder SHALL be first in the recent list, and the next launch SHALL start on the
start screen rather than reopening that folder.

#### Scenario: Close
- **WHEN** the user closes the open library
- **THEN** the start screen is shown with that folder first in the recent list

#### Scenario: Launch after a close
- **WHEN** the app is restarted after a deliberate close
- **THEN** the start screen is shown and the folder is not reported as missing

### Requirement: The library folder can be revealed
The app SHALL offer to reveal the open library's folder in the operating system's file
manager, so the user can back it up or copy it (docs/requirements.md §6).

#### Scenario: Reveal
- **WHEN** the user chooses to reveal the library folder
- **THEN** the operating system's file manager opens showing that folder

### Requirement: The recent list is bounded, deduplicated and prunable
The recent list SHALL hold each folder at most once, SHALL keep only a bounded number of the
most recently opened folders, and SHALL offer removing an entry. An entry whose folder is no
longer readable SHALL be shown as unavailable and SHALL remain listed until it is removed, and
checking availability SHALL NOT delay the app's launch.

#### Scenario: Reopening a listed library
- **WHEN** a library already in the list is opened again
- **THEN** it appears once, at the front

#### Scenario: Folder gone
- **WHEN** a listed folder has been deleted, renamed, or is on a volume that is not mounted
- **THEN** the entry is shown as unavailable and choosing it reports that instead of creating a library there

#### Scenario: Forgetting an entry
- **WHEN** the user removes an entry from the recent list
- **THEN** it is no longer listed, and nothing inside that folder is changed

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
