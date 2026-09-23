# tag-vocabulary Specification

## Purpose
The library's tag vocabulary: what a tag is beyond its name — its category and whether it is
pinned — how a category is given and changed, and how the vocabulary outlives the images that
carry it.

## Requirements

### Requirement: Every tag has one category
Every tag in the library SHALL belong to exactly one of five categories — artist, copyright,
character, general, meta — and SHALL be general unless given another. The category SHALL be a
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

### Requirement: A prefix creates a tag under a category
In every place tags are typed to be stored — the inspector's editor, the bulk edit's add
list, a rule's tags — a token of the form `artist:name`, `copyright:name`, `character:name`,
`meta:name` or `general:name` (case-insensitive prefix) SHALL tag the image `name` and, when
no tag named `name` exists, SHALL create it under that category. When `name` already exists
under the same category the token SHALL be accepted as the plain tag `name`. When `name`
already exists under a different category the whole save SHALL be refused with a reason that
names the tag and both categories, and nothing SHALL change. The prefix SHALL never change
the category of an existing tag. A rule applied at capture time SHALL NOT refuse the capture
over such a conflict: it SHALL link the existing tag as it is.

#### Scenario: Creating an artist
- **WHEN** no tag `kantoku` exists and the user saves `artist:kantoku 1girl`
- **THEN** the image carries `kantoku` and `1girl`, `kantoku` is an artist tag, and `1girl` is general

#### Scenario: An existing artist, tagged plainly
- **WHEN** `kantoku` is an artist tag and the user saves `kantoku`
- **THEN** the image carries `kantoku` and it is still an artist tag

#### Scenario: An existing artist, tagged with the prefix
- **WHEN** `kantoku` is an artist tag and the user saves `artist:kantoku`
- **THEN** the save is accepted exactly as `kantoku` would be

#### Scenario: A conflict
- **WHEN** `cat` is a general tag and the user saves `artist:cat 1girl`
- **THEN** the save is refused with a reason naming `cat`, that it is general, and that it cannot become an artist tag, and the image's tags are unchanged

#### Scenario: A conflict in a bulk edit
- **WHEN** `cat` is a general tag and the user adds `artist:cat` to a selection of fifty
- **THEN** the edit is refused with the same reason and no image of the fifty is changed

#### Scenario: A conflict in a rule at capture time
- **WHEN** `cat` is a general tag and a rule's tags read `artist:cat`, and a capture matches the rule
- **THEN** the capture succeeds carrying `cat`, and `cat` is still general

### Requirement: A category is changed where the tag is shown
The context menu of a tag — in the sidebar's tag list and on the inspector's tag badges —
SHALL offer the five categories with the current one marked, and choosing one SHALL change the
tag's category everywhere it is shown, without touching any image's tag set.

#### Scenario: From the sidebar
- **WHEN** the user opens the context menu of `azur_lane` in the sidebar and chooses Copyright
- **THEN** `azur_lane` is drawn in the copyright colour in the sidebar, in every inspector badge and in the suggestion list

### Requirement: Tags are coloured by category where they are read
Wherever a stored tag is displayed as text to be read — the inspector's tag list, the
sidebar's tag list, the editor's suggestion list — each tag SHALL be drawn in its category's
colour, one colour per category, five colours as Danbooru has them with blue for general
(owner, 2026-09-23), the same in every place and distinguishable in both themes. The marking
that says a tag is in the search SHALL remain visible on a coloured tag. Text inside an
editor SHALL NOT be coloured.

#### Scenario: A mixed panel
- **WHEN** the panel describes an image tagged `kantoku` (artist), `azur_lane` (copyright), `tashkent_(azur_lane)` (character), `highres` (meta) and `1girl`
- **THEN** the five carry five different colours, `1girl` blue, and the search marking is still visible on whichever of them the search names

#### Scenario: Plain in the editor
- **WHEN** the same image's editor is open
- **THEN** every tag in it is drawn in the editor's plain text colour

### Requirement: A tag can be pinned for one-click editing
The user SHALL be able to pin any tag from the tag's context menu in the sidebar and in the
inspector, and unpin it from the same menus and from the pinned chip's own context menu. The
inspector SHALL show every pinned tag, in the app's one category order — artist, copyright,
character, general, meta — alphabetical within a category, as the sidebar lists tags, at the
top of its tag area in both of its placements, whether or not the described image carries it
(owner, 2026-09-24: the strip was alphabetical alone, and reads better grouped like the left
panel). The strip SHALL hold tags only — a
pinned collection's chip sits in the collections section (`collections`) — and SHALL show
nothing, no row and no heading, while no tag is pinned. A pinned chip SHALL look like a
control and not like one of the image's
tags: a pill with a pin mark, where the image's tags are plain text. Each chip SHALL show
whether the described image carries the tag, and one activation SHALL add the tag to the
image when it does not and remove it when it does, recording the image as changed. While the
panel describes a selection, each chip SHALL show whether all, some or none of the selected
images carry the tag; activating it SHALL add the tag to every selected image unless all
carry it, in which case it SHALL remove it from every one, asking first when more than one
image would be written.

#### Scenario: Pin from the sidebar
- **WHEN** the user opens the context menu of `tagme` in the sidebar and chooses Pin
- **THEN** `tagme` appears as a chip with a pin mark at the top of the inspector's tag area

#### Scenario: Pinned tags in category order
- **WHEN** `tagme` (meta), `kantoku` (artist), `1girl` (general) and `azur_lane` (copyright) are pinned
- **THEN** the strip reads `kantoku`, `azur_lane`, `1girl`, `tagme`, each in its category's colour

#### Scenario: One click on, one click off
- **WHEN** `tagme` is pinned, the panel describes an image without it, and the user activates the chip, then activates it again
- **THEN** the image carries `tagme` after the first activation and not after the second, and its last-changed time moved each time

#### Scenario: Over a selection
- **WHEN** `tagme` is pinned, twelve images are selected of which four carry it, and the user activates the chip
- **THEN** the app asks, naming twelve, and on confirmation all twelve carry `tagme`

#### Scenario: All of them
- **WHEN** `tagme` is pinned, twelve images are selected and all carry it, and the user activates the chip and confirms
- **THEN** none of the twelve carries `tagme`

#### Scenario: Unpin from the chip
- **WHEN** the user opens the chip's context menu and chooses Unpin
- **THEN** the chip is gone and the tag is unchanged on every image

#### Scenario: Tags first, then collections
- **WHEN** the tags `tagme` and `wip` and the collection `Cute` are pinned
- **THEN** the strip reads `tagme`, `wip`, and `Cute`'s chip sits in the collections section, not in the strip

#### Scenario: Only a collection pinned
- **WHEN** no tag is pinned and the collection `Cute` is
- **THEN** the strip is absent, and the collections section holds the one chip for `Cute`

#### Scenario: Nothing pinned
- **WHEN** no tag is pinned
- **THEN** the inspector draws no pinned row and no heading for one

### Requirement: The vocabulary outlives its carriers
A tag that is not general, or is pinned, SHALL stay in the library's vocabulary when no image
carries it any more, keeping its category and its pin, and SHALL be offered by the editor's
suggestions. A general, unpinned tag SHALL leave the vocabulary with its last carrier, as it
does today.

#### Scenario: An artist's last image is trashed and deleted
- **WHEN** `kantoku` is an artist tag on one image and that image is deleted for good
- **THEN** `kantoku` is still an artist tag and is still suggested when `kan` is typed

#### Scenario: A general tag's last image
- **WHEN** `bench` is general and unpinned on one image and that tag is removed from it
- **THEN** `bench` is no longer suggested
