# collections Specification

## Purpose
Named, unordered sets of images the user keeps for themselves — favourites, a project, a
batch to upload — that are never tags and never leave the library, with one called Favorites
from the first open.

## Requirements

### Requirement: A collection is a named set of images, separate from tags
The app SHALL let the user create, rename and delete collections in the open library. A
collection SHALL have a name that is unique within the library once lower-cased with spaces as
underscores, and SHALL be identified by an id that does not change when it is renamed. An
image MAY be in any number of collections, and a collection MAY be empty. Membership SHALL NOT
be a tag: it SHALL NOT appear among an image's tags, SHALL NOT be sent to a booru, and SHALL
NOT be matched by a tag term. Deleting a collection SHALL remove its memberships and SHALL NOT
change any image otherwise.

#### Scenario: Create and rename
- **WHEN** the user creates a collection named `To upload` and later renames it `Queue`
- **THEN** the images in it are still in it, and nothing about those images changed

#### Scenario: A duplicate name
- **WHEN** a collection `Queue` exists and the user creates one named `queue`
- **THEN** the creation is refused with a reason naming the existing collection

#### Scenario: Not a tag
- **WHEN** an image is in the collection `Queue` and it is uploaded to a booru, or searched by the tag `queue`
- **THEN** the upload carries no tag for the collection, and the tag search does not match it on that account

#### Scenario: Delete
- **WHEN** the user deletes a collection holding forty images
- **THEN** the collection is gone, the forty images are still in the library with their tags and ratings, and they are in no fewer other collections than before

### Requirement: Every library has Favorites from the start
When a library is created, or opened for the first time by a version that has collections,
the app SHALL create one collection named `Favorites`, once. It SHALL be an ordinary
collection afterwards: the user MAY rename or delete it, and the app SHALL NOT create it again.

#### Scenario: A new library
- **WHEN** the user creates a library
- **THEN** it has one collection, `Favorites`, empty

#### Scenario: An upgraded library
- **WHEN** a library made by an older version is opened by this one
- **THEN** it gains `Favorites`, empty, and every image, tag and rating is as it was

#### Scenario: Deleted stays deleted
- **WHEN** the user deletes `Favorites` and reopens the library
- **THEN** there is no `Favorites`

### Requirement: An image is put into and taken out of collections from where it is shown
The app SHALL offer adding to and removing from any collection: for one image from the
inspector and from the tile's context menu, and for the whole selection from the selection
toolbar and from the context menu of a tile that is part of the selection. Adding an image
already in the collection SHALL change nothing; the action SHALL apply to every named image or
to none. Creating a new collection SHALL be offered from the same places. The inspector SHALL
show the collections the described image is in, each acting as a search term with the same
marking the tag list uses.

Every one of those menus SHALL stay within the window when the library has more collections
than fit below it, scrolling its list rather than extending off screen. Every menu and dialog
the inspector opens — the add menu, a tag's or a collection's context menu, the new-collection
dialog, the upload dialog — SHALL open and be usable when the inspector is shown inside the
full-size viewer, exactly as it is beside the grid.

A tile SHALL carry a mark when its image is in at least one collection, naming them on
hover, and no mark otherwise, so an image in no collection is told apart at a glance.

#### Scenario: From the tile
- **WHEN** the user right-clicks a tile that is not selected and adds it to `Favorites`
- **THEN** that one image is in `Favorites`, and the selection is unchanged

#### Scenario: From a selected tile
- **WHEN** twelve images are selected and the user right-clicks one of them and adds to `Favorites`
- **THEN** all twelve are in `Favorites`

#### Scenario: From the toolbar
- **WHEN** twelve images are selected, three of them already in `Queue`, and the user adds the selection to `Queue`
- **THEN** all twelve are in `Queue` and the selection stands

#### Scenario: Shown in the inspector
- **WHEN** the panel describes an image in `Favorites` and `Queue`
- **THEN** both names are shown, and acting on `Queue` puts `collection:queue` in the search with `Queue` marked as active

#### Scenario: Removed from the inspector
- **WHEN** the user removes the described image from `Queue` in the panel
- **THEN** the image is no longer in `Queue` and is still in `Favorites`

#### Scenario: A long menu
- **WHEN** the library has forty collections and the user opens the inspector's add menu near the bottom of the window
- **THEN** the menu ends inside the window and scrolls to the collections that did not fit

#### Scenario: Adding from inside the viewer
- **WHEN** the full-size viewer is in inspect mode and the user opens the panel's add menu and chooses `Queue`
- **THEN** the menu opens over the viewer, the image is in `Queue`, and the viewer is still open on it

#### Scenario: The mark
- **WHEN** one image is in `Favorites` and `Queue` and another is in no collection
- **THEN** the first tile carries the mark, naming both on hover, and the second carries none

### Requirement: The sidebar lists the collections with counts
Beside the results the app SHALL list every collection in the library with the number of
images of the current result in it, in name order; the collections the search names SHALL be
marked where they are and SHALL NOT move to the front. Each entry SHALL filter the search by
that collection on activation and take the term out again when it is already there, SHALL
offer including it in the search and excluding it from the search as two separate controls
in the same form the tag list uses, and SHALL show which of the two the current search does.
Excluding a collection the search includes SHALL replace the inclusion, and the reverse;
neither action SHALL disturb the rest of the query. Each entry SHALL offer rename and delete
on its context menu; deleting SHALL ask first, naming the collection and how many images are in
it. The section SHALL offer creating a collection.

Each row SHALL be drawn exactly as a row of the tag list is — the same text size, the same
row height, the same spacing between rows — so the two lists read as one column (owner,
2026-09-24: the collection rows were taller and looser than the tags above them). The list
SHALL have no frame of its own around it.

The section SHALL fold away and unfold on request, and whether it is folded SHALL be kept
with the app's other preferences, so the choice survives a restart. Unfolded, its list SHALL
occupy a height the user can change by dragging the section's top edge — the edge it shares
with the tag list, dragged up for more room and down for less — kept for the session, and
SHALL scroll inside that height: a library with many collections SHALL NOT push the tag list
or the controls below out of their room. The handle SHALL be on the top edge and not the
bottom, because the section's bottom is pinned against the sections below it and a handle
there cannot be dragged past them (owner, 2026-09-24).

#### Scenario: Counts follow the search
- **WHEN** `Favorites` holds 300 images and the search is `cat`, matching 40 of them
- **THEN** `Favorites` is listed with 40

#### Scenario: Filter from the list
- **WHEN** the user activates `Favorites` in the list
- **THEN** the search reads `collection:favorites`, and activating it again removes the term

#### Scenario: Exclude from the list
- **WHEN** the search reads `cat` and the user excludes `Uncategorized` from the list
- **THEN** the search reads `cat -collection:uncategorized`, the row is marked as excluded, and images in `Uncategorized` leave the result

#### Scenario: Include replaces exclude
- **WHEN** the search reads `-collection:queue` and the user includes `Queue` from the list
- **THEN** the search reads `collection:queue` and nothing else changed

#### Scenario: An active collection stays in place
- **WHEN** the list reads `Art`, `Favorites`, `Queue` and the user activates `Queue`
- **THEN** `Queue` is marked active and the list still reads `Art`, `Favorites`, `Queue`

#### Scenario: Rename from the row
- **WHEN** the user opens the context menu of `Queue` and chooses rename
- **THEN** a dialog offers the new name, and no other control for it is drawn on the row

#### Scenario: Delete from the list
- **WHEN** the user chooses delete on `Queue`, holding 12 images
- **THEN** the app asks, naming `Queue` and 12, and only on confirmation deletes it

#### Scenario: Many collections
- **WHEN** the library has thirty collections
- **THEN** the section shows as many as fit the height it has, scrolls to the rest, and the tag list keeps its height

#### Scenario: Dragging the top edge
- **WHEN** the user drags the section's top edge upward by 100 pixels
- **THEN** the list is 100 pixels taller, the tag list above it 100 pixels shorter, and dragging it back down restores both

#### Scenario: Rows match the tag list
- **WHEN** the tag list and the collection list are both on screen
- **THEN** a collection row is the same height as a tag row, in the same text size, with the same spacing to the next row

#### Scenario: Folded across a restart
- **WHEN** the user folds the section and restarts the app
- **THEN** the section is folded

### Requirement: A collection survives a rebuild by its id
An image's describing file SHALL carry the ids of the collections it is in; the library-level
file SHALL carry every collection's id, name, times and whether it is pinned. A rebuild SHALL
restore the collections, their pins included, from the library-level file and the memberships
from the describing files. A membership naming a collection the library-level file does not
list SHALL come back under a collection named by that id, unpinned, so it can be renamed rather
than lost. Renaming, pinning or unpinning a collection SHALL rewrite the library-level file and
SHALL NOT rewrite any describing file.

#### Scenario: Rename touches one file
- **WHEN** `Favorites` holding 25,000 images is renamed
- **THEN** the library-level file changes and no image's describing file does

#### Scenario: Pin touches one file
- **WHEN** `Favorites` holding 25,000 images is pinned
- **THEN** the library-level file says so and no image's describing file changes

#### Scenario: Rebuilt
- **WHEN** a library with three collections and their memberships is rebuilt
- **THEN** the three collections come back with their names and the same images in each

#### Scenario: Rebuilt pinned
- **WHEN** a library whose collection `Cute` is pinned is rebuilt
- **THEN** `Cute` comes back pinned, and its chip is at the top of the inspector's collections section

#### Scenario: Rebuilt with no library-level file
- **WHEN** the library-level file is missing and an image's describing file names a collection id
- **THEN** the image comes back in a collection named by that id, not pinned

### Requirement: A collection can be pinned for one-click membership
The user SHALL be able to pin any collection from the context menu of its row in the sidebar's
collections section and of its entry among the inspector's collections, and unpin it from the
same menus and from the pinned chip's own context menu. Whether a collection is pinned SHALL be
a property of the collection, the same for every image, and SHALL NOT change any image, the
collection's name, or its members.

The inspector SHALL show every pinned collection as a chip at the top of its collections
section, in both of its placements — the selection panel gains a collections section for them,
after its tag area — in collection name order, whether or not the described image is in it
(owner, 2026-09-24: the chips first sat in the pinned strip beside the pinned tags; a collection
chip among tag chips read as one more tag, and the section that lists the image's collections is
where a click that changes them is looked for). A pinned collection's chip SHALL be told apart
from a pinned tag's at a glance: it SHALL carry the mark the tiles use for collection
membership rather than a pin mark, and its text SHALL NOT be drawn in any tag category's
colour. It SHALL show the collection's current name.

Each chip SHALL show whether the described image is in the collection, and one activation
SHALL add the image to it when it is not and take it out when it is, without recording the
image as changed. While the panel describes a selection, each chip SHALL show whether all,
some or none of the selected images are in the collection; activating it SHALL add every
selected image unless all are in, in which case it SHALL take every one out, asking first when
more than one image would be written. A pinned collection SHALL NOT appear in the sidebar's tag
list. Deleting a pinned collection SHALL remove its chip.

#### Scenario: Pin from the sidebar
- **WHEN** the user opens the context menu of `Cute` in the collections section and chooses Pin
- **THEN** `Cute` appears as a chip with the collection mark at the top of the inspector's collections section, and the tag list shows no entry for it

#### Scenario: Pin from the inspector
- **WHEN** the panel describes an image in `Cute` and the user opens the context menu of `Cute` among its collections and chooses Pin
- **THEN** `Cute` appears as a chip at the top of the collections section, shown as containing the image, and its badge stays among the image's collections below

#### Scenario: One click in, one click out
- **WHEN** `Cute` is pinned, the panel describes an image not in it, and the user activates the chip, then activates it again
- **THEN** the image is in `Cute` after the first activation and not after the second, its last-changed time did not move, and nothing else about it changed

#### Scenario: Over a selection
- **WHEN** `Cute` is pinned, twelve images are selected of which four are in it, and the user activates the chip
- **THEN** the chip showed some of them in it, the app asks, naming twelve and `Cute`, and on confirmation all twelve are in `Cute`

#### Scenario: All of them
- **WHEN** `Cute` is pinned, twelve images are selected and all are in it, and the user activates the chip and confirms
- **THEN** none of the twelve is in `Cute`, and each is still in its other collections

#### Scenario: Told apart from a tag
- **WHEN** the tag `cute` and the collection `Cute` are both pinned
- **THEN** the tag's chip sits in the tag area's pinned strip with a pin mark in its category colour, and the collection's sits in the collections section with the collection mark in plain text

#### Scenario: Unpin from the chip
- **WHEN** the user opens the chip's context menu and chooses Unpin
- **THEN** the chip is gone and every image is in exactly the collections it was in

#### Scenario: A pinned collection is renamed
- **WHEN** `Cute` is pinned and renamed `Kawaii`
- **THEN** the chip reads `Kawaii` and is still pinned

#### Scenario: A pinned collection is deleted
- **WHEN** `Cute` is pinned and the user deletes it
- **THEN** its chip is gone, and the chip row is absent if no collection is pinned
