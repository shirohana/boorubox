## MODIFIED Requirements

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

## ADDED Requirements

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
