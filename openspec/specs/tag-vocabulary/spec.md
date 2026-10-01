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
SHALL offer the five categories with the current one marked, each with its category's icon,
and choosing one SHALL change the tag's category everywhere it is shown, without touching any
image's tag set. The same menu, wherever it opens, SHALL offer opening the tag on Danbooru in
the system browser: the tag's wiki page for a tag of any category but artist, and the artist
search by name for an artist tag, since an artist's tag often differs from the name searched
(owner, 2026-09-25).

#### Scenario: From the sidebar
- **WHEN** the user opens the context menu of `azur_lane` in the sidebar and chooses Copyright
- **THEN** `azur_lane` is drawn in the copyright colour in the sidebar, in every inspector badge and in the suggestion list

#### Scenario: Open the wiki
- **WHEN** the user opens the context menu of the general tag `solo` and chooses "Open Danbooru wiki"
- **THEN** the system browser opens `https://danbooru.donmai.us/wiki_pages/solo`

#### Scenario: Search an artist
- **WHEN** the user opens the context menu of the artist tag `metaljelly` and chooses "Search artist on Danbooru"
- **THEN** the system browser opens Danbooru's artist search with `metaljelly` as the name to match

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
inspector, and unpin it from the same menus and from the pinned chip's own context menu. A
pinned tag SHALL belong to exactly one group. Groups SHALL be ordered from 1 and MAY carry a
name; a newly pinned tag SHALL land in group 1 (owner, 2026-09-24: bands to separate drawing
style, objects, clothes and a character's look; names, an order the user sets and moves of
many tags added 2026-10-01, when the strip had become a daily tool — the 2026-09-24 "bands,
not names" was right at three groups and stopped being right at many). The inspector SHALL
show every pinned tag, one row per group in group order, a thin separator between rows, and
within a row in the app's one category order — artist, copyright, character, general, meta —
alphabetical within a category, as the sidebar lists tags, at the top of its tag area in both
of its placements, whether or not the described image carries it.

Each row SHALL carry, at its right edge in small dim text, the group's name when it has one
and otherwise `#n`, `n` being the number the group's "Move to" items use; a single unnamed
group SHALL carry no label, since there is nothing to move to and nothing to name it by. The
label SHALL fold the row: a folded row shows the label and the number of tags it hides and no
chips, and a second activation unfolds it. Whether a group is folded SHALL be kept with the
group in the library (owner, 2026-10-01: a group is library data with an identity; the app's
settings file has none for it).

A pinned tag's context menu SHALL offer, wherever the tag's menu opens: "New group above",
which inserts a group before the tag's own and moves the tag into it; "New group below",
which inserts one after and moves the tag into it — both only while the tag shares its group
with another, since for a tag alone in its group either would leave the groups exactly as
they were, and a control that does nothing is not shown (`app-frame`, owner 2026-09-24); when
there is more than one group, "Move to <label>" for every group other than the tag's own,
the label being the group's name or `#n`; and "Manage pinned groups…", which opens the
dialog below. No unnamed group SHALL ever be empty: when a move or an unpin empties an
unnamed group, the groups after it SHALL close up and be renumbered. A named group SHALL
survive being emptied, drawn as its label alone, because its name is the user's work (the
rule a general tag with a note already follows). The groups — each tag's group, and every
group's name, order and fold — SHALL be kept in the library's own describing file, so a
rebuild restores them; a describing file written before groups had names SHALL restore the
groups the tags name, unnamed and unfolded; one written before groups existed SHALL restore
every pinned tag into group 1.

The "Manage pinned groups…" dialog SHALL list every group in order, each with its name
editable in place, a handle to drag it to another position and "Move up" / "Move down", and
its tags each with a checkbox; while any tag is checked it SHALL offer moving the checked
tags together to any other group or to a new group named on the spot, as one write that
either moves them all or none; it SHALL offer creating a named group, and deleting a group
only while the group is empty. A blank name SHALL be refused for a new group and SHALL clear
an existing group's name. From inside the viewer the dialog SHALL appear above the viewer.

The strip SHALL hold tags only — a pinned collection's chip sits in the collections section
(`collections`) — and SHALL show nothing, no row and no heading, while no tag is pinned and no
group is named. A pinned chip SHALL look like a control and not like one of the image's tags:
a pill with a pin mark, where the image's tags are plain text. Each chip SHALL show whether
the described image carries the tag, and one activation SHALL add the tag to the image when
it does not and remove it when it does, recording the image as changed. While the panel
describes a selection, each chip SHALL show whether all, some or none of the selected images
carry the tag; activating it SHALL add the tag to every selected image unless all carry it,
in which case it SHALL remove it from every one, asking first when more than one image would
be written. The selection panel's strip SHALL show the same groups, the same labels and folds,
and offer the same menu.

#### Scenario: Pin from the sidebar
- **WHEN** the user opens the context menu of `tagme` in the sidebar and chooses Pin
- **THEN** `tagme` appears as a chip with a pin mark in group 1 at the top of the inspector's tag area

#### Scenario: Pinned tags in category order
- **WHEN** `tagme` (meta), `kantoku` (artist), `1girl` (general) and `azur_lane` (copyright) are pinned in one group
- **THEN** the row reads `kantoku`, `azur_lane`, `1girl`, `tagme`, each in its category's colour

#### Scenario: A new group below
- **WHEN** `1girl`, `smile` and `sketch` are pinned in group 1 and the user chooses "New group below" on `sketch`
- **THEN** the strip shows two rows, `1girl smile` and `sketch`, with a separator between them

#### Scenario: A new group above
- **WHEN** group 1 holds `1girl smile` and group 2 holds `sketch`, and the user chooses "New group above" on `sketch`
- **THEN** the rows read `1girl smile`, `sketch` — the new group took position 2, and the unnamed group `sketch` left, now empty, is gone

#### Scenario: Move to a group
- **WHEN** group 1 holds `1girl smile` and group 2 holds `sketch`, and the user chooses "Move to #2" on `smile`
- **THEN** the rows read `1girl` and `sketch smile`, and the menu on `smile` now offers only "Move to #1"

#### Scenario: An emptied group closes up
- **WHEN** three unnamed groups hold `1girl`, `smile`, `sketch` in turn and the user unpins `smile`
- **THEN** the strip shows two rows, `1girl` and `sketch`, and `sketch`'s menu offers "Move to #1"

#### Scenario: An emptied named group stays
- **WHEN** group 2 is named `Clothes` and holds `dress` alone, and the user unpins `dress`
- **THEN** the strip still shows a row labelled `Clothes` with no chips, and the dialog offers deleting it

#### Scenario: Only one group
- **WHEN** `1girl` and `smile` are pinned, both in group 1
- **THEN** either tag's menu offers "New group above" and "New group below" and no "Move to" item

#### Scenario: Alone in its group
- **WHEN** group 1 holds `1girl smile` and group 2 holds `sketch` alone
- **THEN** `sketch`'s menu offers "Move to #1" and neither "New group above" nor "New group below"

#### Scenario: Groups survive a rebuild
- **WHEN** `sketch` is pinned in group 2, named `Style` and folded, and the library is rebuilt from its folder
- **THEN** `sketch` is in group 2 afterwards, the group is named `Style` and is folded

#### Scenario: A describing file from before names
- **WHEN** the library's describing file lists `sketch` in group 2 and carries no groups of its own, and the library is rebuilt from it
- **THEN** `sketch` is in group 2, which is unnamed and unfolded

#### Scenario: An old describing file
- **WHEN** the library's describing file was written before groups existed and lists `tagme` as pinned, and the library is rebuilt from it
- **THEN** `tagme` is pinned in group 1

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
- **WHEN** no tag is pinned, no group is named, and the collection `Cute` is pinned
- **THEN** the strip is absent, and the collections section holds the one chip for `Cute`

#### Scenario: Nothing pinned
- **WHEN** no tag is pinned and no group is named
- **THEN** the inspector draws no pinned row and no heading for one

#### Scenario: The hint appears with the second group
- **WHEN** `1girl` and `sketch` are pinned in one unnamed group and the user chooses "New group below" on `sketch`
- **THEN** the strip shows two rows, the first ending in `#1` and the second in `#2`, and `sketch`'s menu offers "Move to #1"

#### Scenario: One group has no hint
- **WHEN** every pinned tag is in group 1 and the group has no name
- **THEN** the row shows no label and cannot be folded

#### Scenario: A name replaces the number
- **WHEN** group 2 is named `Clothes` in the dialog
- **THEN** its row ends in `Clothes` instead of `#2`, and a tag in group 1 offers "Move to Clothes"

#### Scenario: Fold a group
- **WHEN** group 2 is named `Clothes` and holds four tags, and the user activates its label
- **THEN** the row shows `Clothes · 4` and no chips, in both of the inspector's placements, and activating the label again shows the chips

#### Scenario: Reorder groups
- **WHEN** the groups are `Style`, `Clothes`, `Pose` and the user drags `Pose`'s handle above `Style` in the dialog
- **THEN** the strip's rows read `Pose`, `Style`, `Clothes`, every tag still in its own group

#### Scenario: Move many tags at once
- **WHEN** the user checks `dress`, `hat` and `boots` in group 1 and chooses "Move selected to… New group…", naming it `Clothes`
- **THEN** a new last group named `Clothes` holds the three, group 1 no longer does, and the checkboxes are clear

#### Scenario: A move of many is all or none
- **WHEN** one of the checked names is no longer a tag when the move is written
- **THEN** no tag moved, and the dialog shows the refusal

#### Scenario: Delete an empty group
- **WHEN** group 3, named `Old`, holds no tag and the user chooses "Delete group" on it
- **THEN** the group is gone and the groups after it close up; a group holding a tag offers no delete

### Requirement: The vocabulary outlives its carriers
A tag that is not general, is pinned, or carries a note SHALL stay in the library's vocabulary
when no image carries it any more, keeping its category, its pin and its note, and SHALL be
offered by the editor's suggestions. A general, unpinned tag with no note SHALL leave the
vocabulary with its last carrier, as it does today; removing the note of a general, unpinned
tag no image carries SHALL remove the tag from the vocabulary.

#### Scenario: An artist's last image is trashed and deleted
- **WHEN** `kantoku` is an artist tag on one image and that image is deleted for good
- **THEN** `kantoku` is still an artist tag and is still suggested when `kan` is typed

#### Scenario: A general tag's last image
- **WHEN** `bench` is general and unpinned on one image and that tag is removed from it
- **THEN** `bench` is no longer suggested

#### Scenario: A noted general tag's last image
- **WHEN** `bench` is general, unpinned and carries a note, on one image, and that tag is removed from it
- **THEN** `bench` is still suggested when `ben` is typed, and still carries its note

#### Scenario: The note of an uncarried tag is removed
- **WHEN** `bench` is general, unpinned, carried by no image and carries a note, and the note is removed
- **THEN** `bench` is no longer suggested

### Requirement: A tag can carry a note
Any tag SHALL be able to carry a note: one short plain text, kept with the tag across every
image that carries it. The tag's context menu — in the sidebar's tag list, on the inspector's
tag badges and on the pinned chips — SHALL offer "Edit note…", which opens a dialog titled
"Tag note" naming the tag in its category's colour, with the note's current text prefilled for
editing, and Cancel and Save. Saving SHALL store the text as written, trimmed at both ends;
saving an empty or blank text SHALL remove the note. Saving the text unchanged SHALL write
nothing. Cancel SHALL write nothing. A note SHALL NOT change any image's tag set. Opened from
inside the viewer, the dialog SHALL appear above the viewer. A refusal SHALL be shown in the
dialog, which stays open with the text kept.

#### Scenario: Write a note
- **WHEN** the user opens the context menu of `sky` in the sidebar, chooses "Edit note…", types `whole background only` and saves
- **THEN** `sky` carries the note `whole background only` wherever it is shown, and no image's tags changed

#### Scenario: Clear a note
- **WHEN** `sky` carries a note and the user empties the dialog's text and saves
- **THEN** `sky` carries no note

#### Scenario: Unchanged text
- **WHEN** the user opens the dialog on `sky` and saves without changing the text
- **THEN** nothing is written

#### Scenario: From the viewer
- **WHEN** the viewer is open and the user chooses "Edit note…" on a tag in its inspector
- **THEN** the dialog appears above the viewer and can be typed into and saved

### Requirement: A tag's note is shown where the tag is read
Wherever a tag with a note is drawn as a sidebar row, an inspector badge or a pinned chip, a
small muted glyph SHALL follow its name, and hovering the glyph SHALL show the note, wrapped,
after a short delay. A tag without a note SHALL show no glyph. Tile footers SHALL NOT show a
note or a glyph.

#### Scenario: A noted tag in the sidebar
- **WHEN** `sky` carries a note and the results carry `sky` and `cloud`, which has none
- **THEN** the sidebar's `sky` row shows the glyph after its name, `cloud` shows none, and hovering the glyph shows the note

#### Scenario: A long note wraps
- **WHEN** a tag's note is three sentences long
- **THEN** hovering the glyph shows all of it wrapped across lines at a fixed width, not cut off on one line

#### Scenario: Pinned chip and badge
- **WHEN** a pinned tag carries a note and the inspector shows an image that carries it
- **THEN** both its pinned chip and its badge show the glyph, and hovering either shows the note

### Requirement: A pinned tag is marked where it is read
Wherever a pinned tag is drawn as a row of the sidebar's tag list or as one of the described
image's own tags in the inspector — beside the grid and inside the viewer alike — a small
muted dot SHALL follow its name, no wider than the text's x-height, with no background, no
change of weight and no underline, so it cannot be mistaken for the search's marking (owner,
2026-10-01). A tag that is not pinned SHALL show no dot. The pinned chips, the tile footers,
the editor and the completion list SHALL NOT carry it.

#### Scenario: The dot in the sidebar
- **WHEN** `1girl` is pinned and `cat` is not, and both are in the results
- **THEN** the sidebar's `1girl` row shows a small muted dot after its name and `cat`'s row shows none

#### Scenario: The dot in the viewer
- **WHEN** `1girl` is pinned and the viewer shows an image carrying it
- **THEN** the viewer's panel lists `1girl` with the dot after it, and the dot carries no background or underline

#### Scenario: Active and pinned
- **WHEN** `1girl` is pinned and the search includes it
- **THEN** its sidebar row carries the search's tint and the dot, and the dot alone marks a pinned tag the search does not include
