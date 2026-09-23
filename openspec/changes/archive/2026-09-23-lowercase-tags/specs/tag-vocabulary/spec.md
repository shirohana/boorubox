## MODIFIED Requirements

### Requirement: Every tag has one category
Every tag in the library SHALL belong to exactly one of five categories — artist, copyright,
character, meta, general — and SHALL be general unless given another. The category SHALL be a
property of the tag, shared by every image carrying it: changing it SHALL change it for every
image at once. A tag's name SHALL be unique across categories: the library SHALL NOT hold two
tags of the same name in different categories. A tag's name SHALL be lowercase: a name
reaching the library in any case — typed, in a stamp or a rule, with a capture, restored from
a describing file, imported — SHALL name the tag of its lowercase form, and be created under
that form when new, so two spellings of one name never make two tags. A library written
before this rule SHALL be brought under it when opened: rows differing only by case become
one, carried by every image either carried, keeping a non-general category over general and a
pin over none.

#### Scenario: Default
- **WHEN** a tag reaches the library with no category named — typed plainly, arrived with a capture, applied by a rule, restored from a describing file
- **THEN** it is general

#### Scenario: One name, one tag
- **WHEN** `cat` is a general tag and an artist named Cat is to be tagged
- **THEN** the artist is tagged under another name, such as `cat_(artist)`, and `cat` stays the general tag

#### Scenario: Typed in capitals
- **WHEN** `tagme` is a meta tag and the user saves `artist:Tagme` on an image
- **THEN** the save is refused as a category conflict naming `tagme`, exactly as `artist:tagme` is

#### Scenario: Created in capitals
- **WHEN** no tag `kantoku` exists and the user saves `artist:Kantoku`
- **THEN** the image carries `kantoku`, an artist tag, and the editor shows it lowercase

#### Scenario: An older library
- **WHEN** a library holding `Tagme` (artist, on image A) and `tagme` (meta, pinned, on image B) is opened
- **THEN** it holds one tag `tagme`, meta, pinned, carried by A and B
