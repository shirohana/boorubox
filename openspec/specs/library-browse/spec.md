# library-browse Specification

## Purpose
The user browses the library as a grid, narrows it with Danbooru-style tag search, and views
images full size, with counts that make a migration verifiable.

## Requirements

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

### Requirement: Tag search uses the legacy query language
A search box SHALL accept the legacy extension's query syntax: space-separated tags are AND,
`a or b` is OR, `-tag` excludes, and the metatags `rating:`, `is:`, `tagcount:` and `account:`
filter as they do in the legacy viewer. Free text in `page title` and URLs SHALL be searchable.
The metatag `collection:<name>` SHALL match images in the collection whose name, lower-cased
with spaces as underscores, is `<name>`; `-collection:<name>` SHALL exclude them; a name no
collection has SHALL match nothing.

#### Scenario: AND and NOT
- **WHEN** the query is `cat -dog`
- **THEN** only images tagged `cat` and not tagged `dog` are shown

#### Scenario: OR group
- **WHEN** the query is `cat or dog`
- **THEN** images tagged either `cat` or `dog` are shown

#### Scenario: Rating metatag
- **WHEN** the query is `rating:s,q`
- **THEN** only images rated `s` or `q` are shown

#### Scenario: Empty result
- **WHEN** no image matches
- **THEN** the grid shows an empty state naming the query, not a blank page

#### Scenario: Collection metatag
- **WHEN** the query is `cat collection:my_favorites`
- **THEN** only images tagged `cat` that are in the collection named `My favorites` are shown

#### Scenario: Excluding a collection
- **WHEN** the query is `-collection:queue`
- **THEN** images in `Queue` are not shown and every other image is

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
on the dark area around the image SHALL close it, at any zoom. Tab SHALL reach only the view's
own controls; the view itself SHALL NOT be a stop in that order.

The view's controls — the title, the position in the result, previous, next, inspect and
close — SHALL be drawn over the image in one corner, small, and SHALL be hidden until asked
for: the image SHALL be fitted to the whole of the view's space, not to what the controls
leave. A single click on the image SHALL show the controls if hidden and hide them if shown,
taking effect only once the double-click interval has passed without a second click, so a
double click never shows them. Tab SHALL show them before moving the focus to one of them.

A double click on the image SHALL zoom it to its natural pixel size, or to twice its fitted
size when its natural size is not larger than the fit, and a double click on the zoomed image
SHALL return it to the fit. The mouse wheel over the image SHALL step the scale up and down
between the fit and a ceiling. While the image is larger than its space, the pointer's
position across that space SHALL choose which part of the image is shown, edge to edge, so
that a small movement near an edge reaches the far end of the image; and a margin of the dark
area SHALL remain around the image at every zoom, so a click beside it still closes the view.
Moving to another image SHALL return the zoom to the fit.

The full-size view SHALL have two modes: the image alone, and the image beside the inspector
panel; the user SHALL be able to move between them without closing the view, and the mode the
view was left in SHALL be the mode it opens in next, until the app is restarted.

A search rewritten from inside the view — a tag or account acted on in its inspector panel —
SHALL NOT close it: the view SHALL stay open on the same image at its row in the new result,
and its previous/next order SHALL be the new result's. The view SHALL close only when that
image is no longer in the result.

Closing SHALL return focus to the thumbnail of the image the view showed last — the one it was
opened from when it was not moved.

#### Scenario: Navigate
- **WHEN** the lightbox is open and the right arrow is pressed
- **THEN** the next image in the current search result is shown

#### Scenario: Opened from the keyboard
- **WHEN** a tile is focused, Space opens the full-size view, and an arrow key is pressed
- **THEN** the view moves to another image and the grid's focused tile is unchanged

#### Scenario: A tag acted on inside the view
- **WHEN** the view is in inspect mode showing an image tagged `cat`, and the user acts on `cat` in its panel
- **THEN** the search reads `cat`, the view is still open on the same image, and the right arrow moves to the next image tagged `cat`

#### Scenario: The viewed image leaves the result
- **WHEN** the view is showing an image tagged `cat` and the user excludes `cat` from its panel
- **THEN** the view closes and the grid shows the images without `cat`

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

#### Scenario: The image has the whole view
- **WHEN** the view opens on a tall image
- **THEN** the image is fitted to the full height of the view, and no controls take space above it

#### Scenario: Showing and hiding the controls
- **WHEN** the user clicks once on the image and waits
- **THEN** the controls appear in the corner over the image, and one more click hides them

#### Scenario: A double click does not flash the controls
- **WHEN** the user double-clicks the image with the controls hidden
- **THEN** the image zooms and the controls stay hidden

#### Scenario: Zoom to natural size
- **WHEN** a 4000 by 3000 image is fitted into a 1500 by 900 view and the user double-clicks it
- **THEN** the image is shown at 4000 by 3000 pixels, a margin of the dark area stays around the visible part, and a second double click returns it to the fit

#### Scenario: A small image zooms to twice the fit
- **WHEN** a 400 by 300 image is fitted into a 1500 by 900 view and the user double-clicks it
- **THEN** the image is shown at twice its fitted size

#### Scenario: The pointer pans
- **WHEN** the image is zoomed beyond its space and the user moves the pointer from the left edge of that space to the right edge
- **THEN** the shown part of the image moves from its left edge to its right edge

#### Scenario: Wheel
- **WHEN** the image is at the fit and the user rolls the wheel up over it, then rolls down past where it started
- **THEN** the image grows step by step, and shrinks back no smaller than the fit

#### Scenario: Closing while zoomed
- **WHEN** the image is zoomed and the user clicks the dark margin beside it, or presses Escape
- **THEN** the view closes

#### Scenario: Moving on resets the zoom
- **WHEN** the image is zoomed and the user presses the right arrow
- **THEN** the next image is shown at the fit

### Requirement: Per-source counts
The UI SHALL show the total image count and the count per source (`extension`, `local`,
`legacy-bundle`) for the whole library, independent of the current search. The per-source
counts SHALL live on a screen the user reaches from the library without running a search, and
SHALL NOT occupy the browsing screen; the total SHALL remain visible while browsing.

#### Scenario: Counts after mixed ingest
- **WHEN** the library holds 3 extension captures and 2 local imports
- **THEN** the counts show total 5, extension 3, local 2, legacy-bundle 0

#### Scenario: Counts do not follow the search
- **WHEN** a search is narrowing the grid to one image
- **THEN** the total shown while browsing and the per-source counts still describe the whole library

#### Scenario: Reaching the counts
- **WHEN** the user wants to reconcile the library against another source of the same images
- **THEN** the per-source counts are two clicks away from the grid, with no search involved
