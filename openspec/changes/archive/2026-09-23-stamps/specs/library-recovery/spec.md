## MODIFIED Requirements

### Requirement: The library's own settings are described by one file
The app SHALL keep one file in the library folder describing the library-level state a
rebuild would otherwise lose: the auto-tag rules, the configured booru sites, the library
note, the collections with their ids and names, the tag vocabulary — every tag that is
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

#### Scenario: A stamp is saved
- **WHEN** the user creates, edits or deletes a stamp
- **THEN** the library's own file lists the stamps as they now stand

#### Scenario: A tag is categorised or pinned
- **WHEN** the user creates a tag under a category, changes a tag's category, or pins or unpins a tag
- **THEN** the library's own file lists the vocabulary as it now stands, and a general unpinned tag is not in it

### Requirement: A library can be rebuilt from its folder
The app SHALL be able to build a working library database from the files in the folder alone:
every image whose describing file can be read SHALL come back with its tags, rating, trash
state, times, source, posts and collections, and searchable exactly as before; the auto-tag
rules, the booru sites, the note, the collections, the tag vocabulary and the stamps SHALL come
back from the library-level file, a vocabulary tag with no carrier included.

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

#### Scenario: The vocabulary comes back
- **WHEN** a library is rebuilt whose file lists `kantoku` as an artist, `tagme` as pinned, and `azur_lane` as a copyright carried by no image
- **THEN** `kantoku` is an artist tag, `tagme` is pinned, and `azur_lane` is a copyright tag that the editor suggests

#### Scenario: A file from before the vocabulary
- **WHEN** a library is rebuilt whose file has no vocabulary key
- **THEN** every tag comes back general and unpinned

#### Scenario: Stamps come back
- **WHEN** a library is rebuilt whose file lists two stamps
- **THEN** both are back with their names and texts, in their order
