## MODIFIED Requirements

### Requirement: Every image is described by a file beside it
The app SHALL keep, for every image it holds, a plain-text file beside that image's own file
describing everything the app knows about it that is not in the image itself: its identity,
its file's type and dimensions and size, where it came from, its tags, its rating, whether it
is in the trash, the times recorded against it, every booru post recorded for it, and the
collections it is in, by their ids. A trashed image SHALL keep its file; permanently deleting
an image SHALL remove it.

The file SHALL be written so that a reader never sees a half-written one, and SHALL be
written the same way for the same content, so that rewriting an image whose facts have not
changed produces the same bytes.

#### Scenario: An image arrives
- **WHEN** an image is captured, imported or dropped in
- **THEN** a file describing it sits beside it in the same folder before the call reports success

#### Scenario: A tag is typed
- **WHEN** the user adds, removes or replaces an image's tags, or sets or clears its rating
- **THEN** the file beside that image says so afterwards

#### Scenario: An image is trashed
- **WHEN** the user moves an image to the trash and later restores it
- **THEN** its file records it as trashed and then as restored, and is never removed by either

#### Scenario: An image is deleted for good
- **WHEN** the user permanently deletes an image
- **THEN** its describing file is removed with its image file and its thumbnail, and a rebuild afterwards does not bring the image back

#### Scenario: A post is recorded
- **WHEN** an image is uploaded to a booru and the post is recorded against it
- **THEN** the file beside the image names that post

#### Scenario: Put in a collection
- **WHEN** the user adds an image to a collection or takes it out of one
- **THEN** the file beside that image lists its collections' ids afterwards

### Requirement: The library's own settings are described by one file
The app SHALL keep one file in the library folder describing the library-level state a
rebuild would otherwise lose: the auto-tag rules, the configured booru sites, the library
note, and the collections with their ids and names. It SHALL be rewritten whenever any of
those change. It SHALL NOT contain any API key or
other credential; those live in the operating system's credential store (`booru-sites`) and
SHALL NOT be written into the library folder.

#### Scenario: A rule is saved
- **WHEN** the user creates, edits, deletes or imports an auto-tag rule
- **THEN** the library's own file says so afterwards

#### Scenario: A site is configured
- **WHEN** the user adds, edits or removes a booru site
- **THEN** the library's own file lists the sites as they now stand, and carries no API key

#### Scenario: The note is written
- **WHEN** the library note is saved
- **THEN** the library's own file carries its text

#### Scenario: A collection is renamed
- **WHEN** the user creates, renames or deletes a collection
- **THEN** the library's own file lists the collections as they now stand, by id and name

### Requirement: A library can be rebuilt from its folder
The app SHALL be able to build a working library database from the files in the folder alone:
every image whose describing file can be read SHALL come back with its tags, rating, trash
state, times, source, posts and collections, and searchable exactly as before; the auto-tag
rules, the booru sites, the note and the collections SHALL come back from the library-level
file.

Rebuilding SHALL NOT read, decode or rewrite any image or thumbnail — an image's dimensions
come from its describing file — and SHALL NOT remove any file under the folder. A describing
file that cannot be read SHALL be counted and named in the result and SHALL NOT stop the
rebuild. An image file with no describing file SHALL be left alone and SHALL NOT become a
record. An image whose describing file is there but whose image file is not SHALL come back
as a record shown missing, which `library-folder` already offers to trash.

A rebuild SHALL report what it did: how many images came back, how many files could not be
read and which, and the name the previous database was kept under. While it runs it SHALL
show its progress rather than appear stalled, and it SHALL leave no half-built library behind
if it is interrupted.

#### Scenario: Rebuilt from the folder
- **WHEN** a library of 25,000 images is rebuilt after its database is damaged
- **THEN** all 25,000 are back with their tags, ratings, trash state and posts, searchable by tag and by text, and no image file's bytes or modification time changed

#### Scenario: An unreadable describing file
- **WHEN** one image's describing file is truncated or is not valid
- **THEN** the rebuild finishes, its result counts and names that one file, and every other image comes back

#### Scenario: An image whose file is gone
- **WHEN** an image's describing file is present but its image file is not
- **THEN** its record comes back and is shown as missing

#### Scenario: A stray file in the folder
- **WHEN** the folder holds an image file that no describing file names
- **THEN** the rebuild leaves it where it is and creates no record for it

#### Scenario: Interrupted part-way
- **WHEN** a rebuild is interrupted before it finishes
- **THEN** the library is not left with a partly built database presented as complete, and rebuilding again works

#### Scenario: Collections come back
- **WHEN** a library whose images are in collections is rebuilt
- **THEN** every collection is back by its id and name, and every image is in the collections it was in
