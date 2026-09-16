## MODIFIED Requirements

### Requirement: The sidebar lists the collections with counts
Beside the results the app SHALL list every collection in the library with the number of
images of the current result in it, in name order; the collections the search names SHALL be
marked where they are and SHALL NOT move to the front. Each entry SHALL filter the search by
that collection on activation and take the term out again when it is already there, and SHALL
offer rename and delete on its context menu; deleting SHALL ask first, naming the collection
and how many images are in it. The section SHALL offer creating a collection.

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
