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
deliberately, in which case there is no library to reopen, or unless the user has turned the
automatic open off in settings, in which case the start screen is shown with the recent
libraries and nothing is opened until one is chosen. The remembered path SHALL be kept in
both cases.

The launch open SHALL NOT keep the window from appearing: while it runs the app SHALL show a
screen naming the folder being opened, and SHALL show the library UI, or the start screen
with the reason, once it settles.

#### Scenario: Library present
- **WHEN** the app starts and the stored path exists and contains `library.sqlite`
- **THEN** the library UI opens on that folder

#### Scenario: Library missing
- **WHEN** the app starts and the stored path does not exist or is not readable
- **THEN** the start screen is shown with the missing path named, and the stored path is kept until the user picks another

#### Scenario: Closed before quitting
- **WHEN** the user closed the library and then quit
- **THEN** the next launch shows the start screen with that folder offered as the most recent, and nothing is reported as missing

#### Scenario: A large library at launch
- **WHEN** the app starts with a library of 25,000 images remembered
- **THEN** the window appears at once showing that folder's name as opening, and the library UI replaces it when the open finishes

#### Scenario: Automatic open turned off
- **WHEN** the user has turned "Open the last library at launch" off and starts the app
- **THEN** the start screen is shown with the recent libraries, nothing is opened, nothing is reported as missing, and choosing one opens it

#### Scenario: The setting is where the other settings are
- **WHEN** the user opens the settings screen
- **THEN** "Open the last library at launch" is offered as a switch, on unless turned off, and changing it takes effect at the next launch

### Requirement: Layout is stable and self-contained
Files SHALL be stored as `images/<a1>/<id>.<ext>` and thumbnails as `.thumbs/<a1>/<id>.jpg`,
where `<a1>` is the first two characters of the id (owner, 2026-09-24: one level, settled
before a stable release); uploads in progress SHALL live under `inbox/` until complete. A
library whose files sit directly under `images/` or `.thumbs/`, or one level deeper than this
shape from the earlier two-level layout, SHALL be moved into this shape when it is opened,
without changing any record, and the emptied directories removed.

Metadata SHALL be held in `library.sqlite` inside the same folder, which is what every read
the app answers comes from, and SHALL also be written beside the images as plain files the
library can be rebuilt from (`library-recovery`): one per image next to its own file, and one
in the folder for the library's rules, sites and note. No metadata SHALL be held anywhere
outside the library folder, machine-local preferences and the operating system's credential
store excepted.

#### Scenario: Folder copied elsewhere
- **WHEN** the user copies the whole library folder to another location and picks it
- **THEN** every image and its metadata are shown identically

#### Scenario: Image stored
- **WHEN** an image with id `a1b2c3d4-…` and extension `jpg` is stored
- **THEN** its file is at `images/a1/a1b2c3d4-….jpg`, the bucket directory having been created as needed, and its record's `file` field reads that relative path

#### Scenario: Flat library opened
- **WHEN** a library whose files sit directly under `images/` and `.thumbs/` is opened
- **THEN** every such file is at its bucketed path afterwards, every record still renders, and opening the library again moves nothing

#### Scenario: Two-level library opened
- **WHEN** a library holding `images/a1/b2/a1b2….jpg`, `images/a1/b2/a1b2….json` and `.thumbs/a1/b2/a1b2….jpg` is opened
- **THEN** the three files are at `images/a1/a1b2….jpg`, `images/a1/a1b2….json` and `.thumbs/a1/a1b2….jpg`, the `b2` directories are gone, every record still renders, and opening the library again moves nothing

#### Scenario: A file that cannot move
- **WHEN** one file under `images/a1/b2/` cannot be renamed
- **THEN** every other file moves, the library opens, that file's directory stays, and the next open tries again

#### Scenario: Metadata beside the image
- **WHEN** an image is stored or edited
- **THEN** the folder holds a file beside it describing it, and the library could be rebuilt from the folder with the database deleted

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

### Requirement: Thumbnails are a cache the user can regenerate
A thumbnail SHALL be a derived file, rendered from the image at 768 pixels on its longest edge
and never larger than the image (owner, 2026-09-24: 384 was blurry at the new tile size on a
2K monitor). A missing thumbnail SHALL be rendered when it is first asked for. The settings
screen SHALL offer regenerating every thumbnail in the library, as a background pass that
shows its progress where it was started, reports how many it rendered and how many images it
could not read, keeps the library usable while it runs, stops when another library is
opened, and refuses to start while it is already running. Once the pass has finished, the
grid SHALL show the new thumbnails without a restart.

#### Scenario: Regenerate
- **WHEN** the library holds 300 images and the user presses Regenerate thumbnails
- **THEN** progress counts up to 300, the report says 300 regenerated, and every thumbnail on disk is now 768 pixels on its longest edge or the image's own size if smaller

#### Scenario: The grid follows
- **WHEN** the pass finishes while the library grid is on screen
- **THEN** the tiles show the new thumbnails without reopening the library

#### Scenario: Switching libraries mid-pass
- **WHEN** the user opens another library while the pass is running
- **THEN** the pass stops, and the library just left has some new thumbnails and some old

#### Scenario: An unreadable image
- **WHEN** one image's file is missing
- **THEN** the pass finishes, the report names one that could not be read, and every other thumbnail was regenerated
