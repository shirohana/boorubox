# library-recovery Specification

## Purpose
The library folder describes itself well enough that `library.sqlite` can be thrown away and
rebuilt from the files beside the images, so a sync client or a bad disk costs the user an
index rather than every tag they ever typed.

## Requirements

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
note, the collections with their ids, names and pins, the tag vocabulary — every tag that is
not general or is pinned, with its category and its pin — and the stamps. It SHALL be rewritten whenever any
of those change. It SHALL NOT contain any API key or other credential; those live in the
operating system's credential store (`booru-sites`) and SHALL NOT be written into the
library folder.

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

#### Scenario: A collection is pinned
- **WHEN** the user pins or unpins a collection
- **THEN** the library's own file lists the collections as they now stand, each saying whether it is pinned

#### Scenario: A stamp is saved
- **WHEN** the user creates, edits or deletes a stamp
- **THEN** the library's own file lists the stamps as they now stand

#### Scenario: A tag is categorised or pinned
- **WHEN** the user creates a tag under a category, changes a tag's category, or pins or unpins a tag
- **THEN** the library's own file lists the vocabulary as it now stands, and a general unpinned tag is not in it

### Requirement: A write that cannot be mirrored is a failed write
A write that stores the row but cannot write the file describing it SHALL be reported as a
failure to whoever asked for it, naming what went wrong. A capture SHALL NOT be answered with
success in that case. Retrying the write SHALL write the missing file rather than refusing as
a duplicate.

The app SHALL repair what it can on its own: opening a library SHALL write a describing file
for every image that has none, and the library-level file if it is missing, so a write that
failed, a run that was interrupted, and a folder whose files were partly deleted all come
back into step without the user doing anything.

#### Scenario: The folder cannot be written
- **WHEN** an edit's row is stored but its describing file cannot be written
- **THEN** the edit is reported as failed with the reason, and a capture in that position is not answered with success

#### Scenario: Retrying a capture that half-landed
- **WHEN** a capture whose image was stored but whose describing file was not is delivered again
- **THEN** the describing file is written and the delivery succeeds, without a second copy of the image

#### Scenario: A library from before this existed
- **WHEN** a library whose images have no describing files is opened
- **THEN** one is written for every image, the library is usable while that runs, and opening it again writes nothing

#### Scenario: Files deleted from the folder
- **WHEN** some describing files are deleted from the folder and the library is opened again
- **THEN** exactly the missing ones are written again

### Requirement: Opening a library checks it, and a damaged one is not opened
Opening a library SHALL check its database before use and SHALL refuse to open one that is
damaged, leaving every file in the folder exactly as it found it. A library that is damaged
SHALL be reported as damaged, distinctly from a library folder that is missing and from one
written by a newer version of the app, and the app SHALL say which folder it is.

#### Scenario: A damaged library on launch
- **WHEN** the app starts and the remembered library's database is damaged
- **THEN** the start screen says that library is damaged rather than missing, names the folder, and nothing in the folder has been changed

#### Scenario: A damaged library picked by hand
- **WHEN** the user picks a folder whose database is damaged
- **THEN** the library is not opened, the damage is reported, and no new library is created in that folder

#### Scenario: A library from a newer build
- **WHEN** the remembered library was written by a newer version of the app
- **THEN** it is reported as such and is not treated as damaged

### Requirement: A damaged database is kept, never deleted
Rebuilding SHALL move the existing database aside under a name that does not collide with any
other, in the library folder, and SHALL keep any journal file beside it in the same move. The
app SHALL NOT delete either, at any point, and SHALL name the kept file where the user can
read it.

#### Scenario: Rebuilding keeps the evidence
- **WHEN** a library is rebuilt
- **THEN** the previous database file is still in the folder under its kept name, and the result names that file

#### Scenario: Rebuilt twice
- **WHEN** a library that has already been rebuilt once is rebuilt again
- **THEN** the earlier kept file is still there and the second one sits beside it

### Requirement: A library can be rebuilt from its folder
The app SHALL be able to build a working library database from the files in the folder alone:
every image whose describing file can be read SHALL come back with its tags, rating, trash
state, times, source, posts and collections, and searchable exactly as before; the auto-tag
rules, the booru sites, the note, the collections with their pins, the tag vocabulary and the
stamps SHALL come back from the library-level file, a vocabulary tag with no carrier included.

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

#### Scenario: Pinned collections come back
- **WHEN** a library is rebuilt whose file lists `Cute` as pinned and `Queue` as not
- **THEN** `Cute` is pinned and `Queue` is not

#### Scenario: A file from before collection pins
- **WHEN** a library is rebuilt whose file lists collections with no word on whether they are pinned
- **THEN** every collection comes back, unpinned

#### Scenario: The vocabulary comes back
- **WHEN** a library is rebuilt whose file lists `kantoku` as an artist, `tagme` as pinned, and `azur_lane` as a copyright carried by no image
- **THEN** `kantoku` is an artist tag, `tagme` is pinned, and `azur_lane` is a copyright tag that the editor suggests

#### Scenario: A file from before the vocabulary
- **WHEN** a library is rebuilt whose file has no vocabulary key
- **THEN** every tag comes back general and unpinned

#### Scenario: Stamps come back
- **WHEN** a library is rebuilt whose file lists two stamps
- **THEN** both are back with their names and texts, in their order

### Requirement: A rebuild is offered where the damage is met, and only on request
The app SHALL offer to rebuild a library from the start screen when that library will not
open because it is damaged, and from settings for the library that is open. It SHALL NOT
rebuild without the user asking, and SHALL say, before they ask, that the existing database is
kept rather than replaced. After a rebuild from the start screen the user SHALL see what the
rebuild did before the library opens.

A rebuild SHALL NOT be offered for a library refused for any other reason, in particular one
written by a newer version of the app.

#### Scenario: Rebuilding from the start screen
- **WHEN** the start screen reports the library as damaged and the user asks to rebuild it
- **THEN** progress is shown while it runs, the result is shown when it finishes, and the library opens from there

#### Scenario: Rebuilding a library that opens
- **WHEN** the user asks from settings to rebuild the open library's index
- **THEN** they are told the current database will be kept aside, and on confirming the library is rebuilt and opened again

#### Scenario: Nothing happens unasked
- **WHEN** a damaged library is met on launch
- **THEN** no file is moved and no database is built until the user asks for it

### Requirement: Describing files are recovery, not concurrency
The app SHALL continue to require that one machine writes to a library at a time. Describing
files SHALL NOT be read to reconcile two machines' edits, SHALL NOT be merged, and SHALL NOT
be read at all except by a rebuild: every read the app answers comes from the database. A file
edited by hand SHALL have no effect until the library is rebuilt.

#### Scenario: A hand-edited file
- **WHEN** the user edits an image's describing file in a text editor and reopens the library
- **THEN** the library shows what the database holds, unchanged

#### Scenario: Two machines
- **WHEN** two machines write to the same synced library folder
- **THEN** the app makes no claim to have made that safe, and the documentation says so
