## MODIFIED Requirements

### Requirement: Grid shows the library
The library UI SHALL show non-deleted images as a grid of thumbnails, in the order and the
grouping the user has chosen — newest capture first, ungrouped, until one is chosen — and
SHALL stay responsive with ten thousand images. A grouping that admits only some of the
matching images SHALL be reflected in the grid and in the number of results it reports, so
that what is counted is what is shown. Each thumbnail SHALL be shown whole, scaled to fit its
tile without cropping, and the image SHALL be the tile: any text about it SHALL appear only
while the tile is hovered or focused. The tile the keyboard is on SHALL be marked as the
current tile in a way that is visible at a glance across the grid and distinct from the text
overlay, so that the current tile can be told from a hovered one. The tile size SHALL be
adjustable from the toolbar, and the setting SHALL survive a restart.

#### Scenario: Open library
- **WHEN** a library with images is opened
- **THEN** thumbnails render in capture-time order, newest first, without loading every full image

#### Scenario: A chosen order
- **WHEN** the user chooses a different order for the results
- **THEN** the grid redraws in that order from the first tile, still without loading every full image

#### Scenario: Mixed shapes
- **WHEN** the library holds both a tall and a wide image
- **THEN** both are shown complete inside equally sized tiles, neither cropped nor stretched

#### Scenario: Hovering a tile
- **WHEN** the pointer rests on a tile, or the tile is focused with the keyboard
- **THEN** its title, source and capture date appear over the image and disappear when it is left

#### Scenario: Finding the current tile
- **WHEN** the keyboard focus is on a tile somewhere in a full screen of thumbnails
- **THEN** that tile is marked so it can be found without hunting, and the marking is not the hover overlay

#### Scenario: Changing the tile size
- **WHEN** the user moves the thumbnail size control
- **THEN** the grid re-flows to the new size, keeps the images uncropped, still renders only the tiles near the viewport, and opens at that size after a restart

### Requirement: Lightbox
Activating a thumbnail — by double click, Enter, Space, or a single click on the thumbnail
that is already the current tile — SHALL open the full-size image scaled to fit the window
without cropping. A single click on a thumbnail that is not the current tile SHALL make it
current without opening it. A press that the pointer is dragged away from before it is released
SHALL NOT open the view, whatever the tile was.

Opening SHALL move the keyboard focus into the full-size view, and the view SHALL own every
key it binds: an arrow pressed while it is open SHALL act on the view and SHALL NOT move the
focus in the grid behind it. The view SHALL offer keyboard navigation to the previous and the
next image in the current result order, and to the image one grid row before and after the one
shown, stepping by the number of tiles the grid is drawing per row. Escape, Space, and a click
on the dark area around the image SHALL close it. Tab SHALL reach only the view's own
controls; the view itself SHALL NOT be a stop in that order.

The full-size view SHALL have two modes: the image alone, and the image beside the inspector
panel; the user SHALL be able to move between them without closing the view, and the mode the
view was left in SHALL be the mode it opens in next, until the app is restarted.

Closing SHALL return focus to the thumbnail of the image the view showed last — the one it was
opened from when it was not moved.

#### Scenario: Navigate
- **WHEN** the lightbox is open and the right arrow is pressed
- **THEN** the next image in the current search result is shown

#### Scenario: Opened from the keyboard
- **WHEN** a tile is focused, Space opens the full-size view, and an arrow key is pressed
- **THEN** the view moves to another image and the grid's focused tile is unchanged

#### Scenario: A row at a time
- **WHEN** the grid is showing five tiles per row and the down arrow is pressed in the full-size view
- **THEN** the image five places later in the result is shown

#### Scenario: A row at a time at the edge
- **WHEN** the up arrow is pressed in the full-size view while the image shown is in the first row of the grid
- **THEN** the view stays on an image and never on nothing

#### Scenario: Click then open
- **WHEN** the user clicks a thumbnail that is not the current one
- **THEN** the thumbnail becomes current and its facts are shown in the inspector, and the full-size view does not open

#### Scenario: Click the current thumbnail
- **WHEN** the user clicks the thumbnail that is already the current one
- **THEN** the full-size view opens on that image

#### Scenario: A press dragged away from
- **WHEN** the user presses on the current thumbnail, moves the pointer several pixels and releases it
- **THEN** the full-size view does not open

#### Scenario: Fit to the window
- **WHEN** an image larger than the window is opened
- **THEN** the whole image is visible, scaled down, with no part cropped and no scrollbars

#### Scenario: Inspect mode
- **WHEN** the inspect key is pressed while the full-size view is open
- **THEN** the inspector panel appears beside the image, the image is refitted to the space that is left, and pressing it again returns to the image alone

#### Scenario: Inspect mode is remembered
- **WHEN** the user leaves the full-size view with the inspector panel showing and opens another image
- **THEN** the view opens with the panel showing, and after a restart it opens on the image alone again

#### Scenario: Tab inside the view
- **WHEN** Tab and Shift-Tab are pressed while the full-size view is open
- **THEN** the focus moves between the view's controls and never onto the view as a whole

#### Scenario: Dismiss by clicking beside the image
- **WHEN** the user clicks the dark area around the image
- **THEN** the view closes, exactly as Escape closes it

#### Scenario: Closing returns focus
- **WHEN** the full-size view is closed without having moved to another image
- **THEN** focus returns to the thumbnail it was opened from, and the arrow keys move the grid focus again

#### Scenario: Closing after moving on
- **WHEN** the user moves to the next image in the full-size view and closes it
- **THEN** the grid focuses that image's thumbnail, scrolled into view, and the inspector shows it
