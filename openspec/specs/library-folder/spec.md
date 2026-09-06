# library-folder Specification

## Purpose
The library folder is where every image and all metadata live as plain files, so the user can
back it up or move it by copying the folder.

## Requirements

### Requirement: First launch asks for a library folder
On launch without a remembered library, the app SHALL show a setup screen whose only action is
picking a folder, and SHALL NOT expose the library UI until a folder is chosen.

#### Scenario: No library remembered
- **WHEN** the app starts and no library path is stored
- **THEN** the setup screen is shown and the grid, search and import actions are unavailable

#### Scenario: Folder chosen
- **WHEN** the user picks a writable folder
- **THEN** the app creates `images/`, `inbox/` and `library.sqlite` inside it if missing, stores the path, and shows the library UI

### Requirement: Remembered library opens on launch
The app SHALL reopen the last library on launch without asking.

#### Scenario: Library present
- **WHEN** the app starts and the stored path exists and contains `library.sqlite`
- **THEN** the library UI opens on that folder

#### Scenario: Library missing
- **WHEN** the app starts and the stored path does not exist or is not readable
- **THEN** the setup screen is shown with the missing path named, and the stored path is kept until the user picks another

### Requirement: Layout is stable and self-contained
Files SHALL be stored as `images/<id>.<ext>`; metadata SHALL be stored only in `library.sqlite`
inside the same folder; uploads in progress SHALL live under `inbox/` until complete.

#### Scenario: Folder copied elsewhere
- **WHEN** the user copies the whole library folder to another location and picks it
- **THEN** every image and its metadata are shown identically

### Requirement: External file changes never crash the app
An image whose file was removed or renamed outside the app SHALL be shown as missing and
SHALL offer to drop its record; the rest of the library SHALL keep working.

#### Scenario: File removed externally
- **WHEN** an image file under `images/` is deleted outside the app
- **THEN** its card shows a missing state, the grid still renders, and the user can remove the record

#### Scenario: File restored
- **WHEN** a file previously marked missing reappears at its path
- **THEN** the image renders again and the missing state is cleared
