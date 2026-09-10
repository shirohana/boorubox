# library-folder Specification

## Purpose
The library folder is where every image and all metadata live as plain files, so the user can
back it up or move it by copying the folder.

## Requirements

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

### Requirement: Layout is stable and self-contained
Files SHALL be stored as `images/<a1>/<b2>/<id>.<ext>` and thumbnails as
`.thumbs/<a1>/<b2>/<id>.jpg`, where `<a1>` and `<b2>` are the first two and the next two
characters of the id; metadata SHALL be stored only in `library.sqlite` inside the same
folder; uploads in progress SHALL live under `inbox/` until complete. A library whose files
sit directly under `images/` or `.thumbs/` from an earlier layout SHALL be moved into that
shape when it is opened, without changing any record.

#### Scenario: Folder copied elsewhere
- **WHEN** the user copies the whole library folder to another location and picks it
- **THEN** every image and its metadata are shown identically

#### Scenario: Image stored
- **WHEN** an image with id `a1b2c3d4-…` and extension `jpg` is stored
- **THEN** its file is at `images/a1/b2/a1b2c3d4-….jpg`, the bucket directories having been created as needed, and its record's `file` field reads that relative path

#### Scenario: Flat library opened
- **WHEN** a library whose files sit directly under `images/` and `.thumbs/` is opened
- **THEN** every such file is at its bucketed path afterwards, every record still renders, and opening the library again moves nothing

### Requirement: External file changes never crash the app
An image whose file was removed or renamed outside the app SHALL be shown as missing and
SHALL offer to move its record to the trash, from where the record can be restored if the file
comes back; the rest of the library SHALL keep working. No action offered on a missing image
SHALL destroy its record outright.

#### Scenario: File removed externally
- **WHEN** an image file under `images/` is deleted outside the app
- **THEN** its card shows a missing state, the grid still renders, and the user can move the record to the trash

#### Scenario: File restored
- **WHEN** a file previously marked missing reappears at its path
- **THEN** the image renders again and the missing state is cleared

#### Scenario: File restored after the record was trashed
- **WHEN** the user moved a missing image's record to the trash and the file later reappears
- **THEN** restoring the record from the trash brings the image back into the library and it renders again
