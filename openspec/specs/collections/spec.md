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
than fit below it, scrolling its list rather than extending off screen.

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

The section SHALL fold away and unfold on request, and whether it is folded SHALL be kept
with the app's other preferences, so the choice survives a restart. Unfolded, its list SHALL
occupy a height the user can change by dragging, kept for the session, and SHALL scroll inside
that height: a library with many collections SHALL NOT push the tag list or the controls
below out of their room.

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

#### Scenario: Folded across a restart
- **WHEN** the user folds the section and restarts the app
- **THEN** the section is folded

### Requirement: A collection survives a rebuild by its id
An image's describing file SHALL carry the ids of the collections it is in; the library-level
file SHALL carry every collection's id, name and times. A rebuild SHALL restore the
collections from the library-level file and the memberships from the describing files. A
membership naming a collection the library-level file does not list SHALL come back under a
collection named by that id, so it can be renamed rather than lost. Renaming a collection SHALL
rewrite the library-level file and SHALL NOT rewrite any describing file.

#### Scenario: Rename touches one file
- **WHEN** `Favorites` holding 25,000 images is renamed
- **THEN** the library-level file changes and no image's describing file does

#### Scenario: Rebuilt
- **WHEN** a library with three collections and their memberships is rebuilt
- **THEN** the three collections come back with their names and the same images in each

#### Scenario: Rebuilt with no library-level file
- **WHEN** the library-level file is missing and an image's describing file names a collection id
- **THEN** the image comes back in a collection named by that id
