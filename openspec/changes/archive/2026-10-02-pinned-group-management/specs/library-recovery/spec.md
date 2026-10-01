## MODIFIED Requirements

### Requirement: The library's own settings are described by one file
The app SHALL keep one file in the library folder describing the library-level state a
rebuild would otherwise lose: the auto-tag rules, the configured booru sites, the library
note, the collections with their ids, names and pins, the tag vocabulary — every tag that is
not general, is pinned or carries a note, with its category, its pin and its note — the
pinned groups with their names, order and folds, and the
stamps in their order. It SHALL be rewritten whenever any
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
- **THEN** the library's own file lists the vocabulary as it now stands, and a general unpinned tag with no note is not in it

#### Scenario: A tag's note is written
- **WHEN** the user writes, changes or removes a tag's note
- **THEN** the library's own file lists the vocabulary as it now stands, each noted tag with its note

#### Scenario: A group is named or folded
- **WHEN** the user names, reorders, folds or deletes a pinned group
- **THEN** the library's own file lists the groups as they now stand
