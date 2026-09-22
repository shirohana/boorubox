## MODIFIED Requirements

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
