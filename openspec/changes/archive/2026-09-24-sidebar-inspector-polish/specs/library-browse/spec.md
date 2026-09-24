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
adjustable from the toolbar, from 120 to 640 pixels (owner, 2026-09-24: 360 was too small on
a 2K monitor), and the setting SHALL survive a restart.

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

#### Scenario: The largest tile
- **WHEN** the user moves the thumbnail size control to its end
- **THEN** the tiles are 640 pixels wide
