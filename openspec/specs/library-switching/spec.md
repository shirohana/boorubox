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
