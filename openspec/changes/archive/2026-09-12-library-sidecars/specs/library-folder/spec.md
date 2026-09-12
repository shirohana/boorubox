# library-folder Specification (delta)

## MODIFIED Requirements

### Requirement: Layout is stable and self-contained
Files SHALL be stored as `images/<a1>/<b2>/<id>.<ext>` and thumbnails as
`.thumbs/<a1>/<b2>/<id>.jpg`, where `<a1>` and `<b2>` are the first two and the next two
characters of the id; uploads in progress SHALL live under `inbox/` until complete. A library
whose files sit directly under `images/` or `.thumbs/` from an earlier layout SHALL be moved
into that shape when it is opened, without changing any record.

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
- **THEN** its file is at `images/a1/b2/a1b2c3d4-….jpg`, the bucket directories having been created as needed, and its record's `file` field reads that relative path

#### Scenario: Flat library opened
- **WHEN** a library whose files sit directly under `images/` and `.thumbs/` is opened
- **THEN** every such file is at its bucketed path afterwards, every record still renders, and opening the library again moves nothing

#### Scenario: Metadata beside the image
- **WHEN** an image is stored or edited
- **THEN** the folder holds a file beside it describing it, and the library could be rebuilt from the folder with the database deleted
