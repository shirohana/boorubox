## ADDED Requirements

### Requirement: The start screen stands in for an open library
Whenever no library is open — on first launch, after the user closes one, and when the
remembered folder cannot be opened — the app SHALL show the start screen, and SHALL NOT expose
the library UI, the search or the import actions until a library is open. The start screen's
actions SHALL be picking a folder and opening one the app has opened before.

#### Scenario: No library remembered
- **WHEN** the app starts and no library path is stored
- **THEN** the start screen is shown and the grid, search and import actions are unavailable

#### Scenario: Folder chosen
- **WHEN** the user picks a writable folder
- **THEN** the app creates `images/`, `inbox/` and `library.sqlite` inside it if missing, stores the path, and shows the library UI

#### Scenario: Closed by the user
- **WHEN** the user closes the open library
- **THEN** the start screen is shown, and it is not described as a missing folder

## MODIFIED Requirements

### Requirement: Remembered library opens on launch
The app SHALL reopen the last library on launch without asking, unless the user closed it
deliberately, in which case there is no library to reopen.

#### Scenario: Library present
- **WHEN** the app starts and the stored path exists and contains `library.sqlite`
- **THEN** the library UI opens on that folder

#### Scenario: Library missing
- **WHEN** the app starts and the stored path does not exist or is not readable
- **THEN** the start screen is shown with the missing path named, and the stored path is kept until the user picks another

#### Scenario: Closed before quitting
- **WHEN** the user closed the library and then quit
- **THEN** the next launch shows the start screen with that folder offered as the most recent, and nothing is reported as missing

## REMOVED Requirements

### Requirement: First launch asks for a library folder
**Reason**: The screen it described is no longer reached only on a first launch: closing a
library and switching between libraries both land on it, and it now offers the recent list as
well as the folder picker. Its gate — no library open means no library UI — is kept verbatim
in "The start screen stands in for an open library", which replaces it.
**Migration**: None. Both scenarios of this requirement ("No library remembered", "Folder
chosen") are carried over unchanged into the replacing requirement.
