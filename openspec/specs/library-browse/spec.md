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
filter as they do in the legacy viewer. A tag term SHALL match without regard to case: tag
names are stored lowercase (`tag-vocabulary`), and a term is lowercased before it is
matched, so `Cat` finds what `cat` finds. Free text in `page title` and URLs SHALL be
searchable. The metatag `collection:<name>` SHALL match images in the collection whose name,
lower-cased with spaces as underscores, is `<name>`; `-collection:<name>` SHALL exclude them;
a name no collection has SHALL match nothing.

#### Scenario: AND and NOT
- **WHEN** the query is `cat -dog`
- **THEN** only images tagged `cat` and not tagged `dog` are shown

#### Scenario: Case
- **WHEN** the query is `Cat -DOG`
- **THEN** the result is the same as for `cat -dog`, and the panels mark `cat` as included and `dog` as excluded

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
current without opening it. While edit mode is on with an active stamp (`stamps`), a
single click on any thumbnail, current or not, SHALL apply the stamp and SHALL NOT open the
view; the keyboard SHALL still open it. A press that the pointer is dragged away from before it is released
SHALL NOT open the view, whatever the tile was.

Opening SHALL move the keyboard focus into the full-size view, and the view SHALL own every
key it binds: an arrow pressed while it is open SHALL act on the view and SHALL NOT move the
focus in the grid behind it. The view SHALL offer keyboard navigation to the previous and the
next image in the current result order, and to the image one grid row before and after the one
shown, stepping by the number of tiles the grid is drawing per row. Escape, Space, and a click
on the dark area around the image SHALL close it, at any zoom. Tab SHALL reach only the view's
own controls; the view itself SHALL NOT be a stop in that order.

The view SHALL draw nothing over the image: no title, no position, no buttons. Moving,
inspecting and closing are the keys' and the dark area's. The image SHALL be fitted to the
whole of the view's space.

A single click on the image SHALL zoom it towards covering its space — the smaller of its two
dimensions filling the space's, so the image overflows along one axis only — but no further
than a ceiling the user sets, expressed as a multiple of the fit: the click's size is the cover
or the ceiling times the fit, whichever is smaller. A single click on the zoomed image SHALL
return it to the fit. An image whose cover size is not larger than its fit SHALL zoom to twice
the fit instead, so the click always visibly zooms, again no further than the ceiling. The
ceiling SHALL be adjustable on the settings screen between one and a quarter and three and a
half times the fit, in steps of a quarter, SHALL start at one and a half times the fit, and
SHALL survive a restart. The wheel's own ceiling is not
this one. The change of scale SHALL be animated briefly rather than cut. The mouse wheel over the image SHALL scale it in
proportion to how far the wheel turned, between the fit and a ceiling, so that a trackpad's
small movements zoom in small steps and a mouse notch in one visible step. A pinch on a trackpad
SHALL scale the image the same way. While the image is larger than its space, the pointer's
position across the middle third of that space, on each axis, SHALL choose which part of the
image is shown, edge to edge — a pointer outside that third rests at the nearer edge — so that
a small movement of the pointer reaches the far end of the image. The image's space SHALL be
the whole window: no gap SHALL be kept between the image and the window's edge, so that a
part of the image the edge cuts off reads as cut off rather than as the image's own edge.
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
- **WHEN** the view is open on the third image of a result and the right arrow is pressed
- **THEN** the fourth image is shown, and the grid behind the view has not moved its focus

#### Scenario: Not opened in edit mode
- **WHEN** edit mode is on with an active stamp and the user clicks the current thumbnail
- **THEN** the view does not open, and Enter on that card still opens it

#### Scenario: Opened from the keyboard
- **WHEN** a card is focused and Enter is pressed
- **THEN** the view opens on that image and the arrows move within the view

#### Scenario: A tag acted on inside the view
- **WHEN** the view is in inspect mode on an image tagged `cat` and the user acts on `cat` in the panel
- **THEN** the search reads `cat`, the view stays open on the same image, and the arrows now step through the `cat` result

#### Scenario: The viewed image leaves the result
- **WHEN** the view is on an image not tagged `dog` and the user includes `dog` from the panel of another term
- **THEN** the view closes and the grid shows the `dog` result

#### Scenario: A row at a time
- **WHEN** the grid is drawing five tiles per row, the view is on the seventh image, and the down arrow is pressed
- **THEN** the twelfth image is shown

#### Scenario: A row at a time at the edge
- **WHEN** the view is on the second image of a result and the up arrow is pressed
- **THEN** the view stays on the second image

#### Scenario: Click then open
- **WHEN** the user clicks a thumbnail that is not the current tile and then clicks it again
- **THEN** the first click makes it current and the second opens the view

#### Scenario: Click the current thumbnail
- **WHEN** the user clicks once on the thumbnail that is already the current tile
- **THEN** the view opens on it

#### Scenario: A press dragged away from
- **WHEN** the user presses on the current thumbnail, drags the pointer several pixels and releases
- **THEN** the view does not open

#### Scenario: Fit to the window
- **WHEN** a 4000 by 3000 image opens in a 1500 by 900 view
- **THEN** the whole image is visible, scaled down, with nothing drawn over it

#### Scenario: Inspect mode
- **WHEN** the view is open on the image alone and the user presses `i`
- **THEN** the inspector panel appears beside the image, describing it, and the view stays open

#### Scenario: Inspect mode is remembered
- **WHEN** the user closes the view while it is in inspect mode and opens another image
- **THEN** the view opens in inspect mode

#### Scenario: Tab inside the view
- **WHEN** the view is in inspect mode and Tab is pressed repeatedly
- **THEN** the focus visits the panel's controls in turn and never leaves the view

#### Scenario: Dismiss by clicking beside the image
- **WHEN** the user clicks the dark area beside the image
- **THEN** the view closes

#### Scenario: Closing returns focus
- **WHEN** the view was opened from the fourth card and Escape is pressed
- **THEN** the fourth card is focused

#### Scenario: Closing after moving on
- **WHEN** the view was opened from the fourth card, the right arrow was pressed twice, and Escape is pressed
- **THEN** the sixth card is focused and scrolled into view

#### Scenario: The image has the whole view
- **WHEN** the view opens on a tall image
- **THEN** the image is fitted to the full height of the view, and nothing is drawn over it

#### Scenario: Showing and hiding the controls
- **WHEN** the user clicks once on the image and waits
- **THEN** nothing appears over the image: there are no controls to show, and the click has zoomed it

#### Scenario: A double click does not flash the controls
- **WHEN** the user clicks the image twice in quick succession
- **THEN** the image zooms to cover and returns to the fit, and nothing appears over it

#### Scenario: Zoom to natural size
- **WHEN** the ceiling is at its default of one and a half times the fit, a 4000 by 3000 image is fitted into a 1500 by 900 view, and the user clicks it once
- **THEN** the image grows, animated, until it is 1500 pixels wide and taller than the view — the cover, at one and a quarter times the fit, is under the ceiling — only the vertical direction can be panned, its sides reach the window's edges with no gap, and one more click returns it to the fit

#### Scenario: A portrait image stops at the ceiling
- **WHEN** the ceiling is at its default, a 1000 by 3000 image is fitted into a 1500 by 900 view, and the user clicks it once
- **THEN** the image grows to one and a half times the fit — 450 pixels wide by 1350 tall, not the 1500 by 4500 that would cover the width — with dark space beside it, and only the vertical direction can be panned

#### Scenario: A raised ceiling zooms the portrait image further
- **WHEN** the user has set the ceiling to three and a half times the fit, the top of its range, and clicks the same 1000 by 3000 image in the same view
- **THEN** the image is 1050 pixels wide and 3150 tall — still short of the 1500 that would cover the width, since that cover is five times the fit and out of the slider's reach

#### Scenario: A small image zooms to the ceiling
- **WHEN** the ceiling is at its default, a 400 by 300 image is fitted into a 1500 by 900 view — at its own size, never upscaled — and the user clicks it once
- **THEN** the image is shown at 600 by 450, the ceiling, rather than covering the view's width

#### Scenario: A wide image under the ceiling
- **WHEN** the ceiling is at its default, a 4000 by 1000 image is fitted into a 1500 by 900 view, and the user clicks it once
- **THEN** the image is 2250 pixels wide and 562 tall — one and a half times the fit, where the cover would have been 900 tall — and only the horizontal direction can be panned

#### Scenario: A wide image covers by height
- **WHEN** the user has set the ceiling to three times the fit, a 4000 by 1000 image is fitted into a 1500 by 900 view, and the user clicks it once
- **THEN** the image is 900 pixels tall and wider than the view — the cover, at two and two fifths times the fit, is under that ceiling — and only the horizontal direction can be panned

#### Scenario: The ceiling is set and kept
- **WHEN** the user moves the click zoom control on the settings screen to three times the fit and restarts the app
- **THEN** the next click in the viewer zooms no further than three times the fit

#### Scenario: The pointer pans
- **WHEN** the image is zoomed beyond its space horizontally, the space is 900 pixels wide, and the user moves the pointer from 300 pixels in to 600 pixels in
- **THEN** the shown part of the image moves from its left edge to its right edge, and moving further out either way changes nothing

#### Scenario: Wheel
- **WHEN** the image is at the fit and the user rolls a mouse wheel one notch up over it, then one notch down
- **THEN** the image grows by one visible step and shrinks back to the fit

#### Scenario: A trackpad scroll
- **WHEN** the image is at the fit and the user scrolls a trackpad slowly upward over it
- **THEN** the image grows smoothly by an amount proportional to the distance scrolled, not by one step per movement

#### Scenario: A pinch
- **WHEN** the user pinches outward on a trackpad over the image
- **THEN** the image grows, and pinching inward shrinks it back no smaller than the fit

#### Scenario: Closing while zoomed
- **WHEN** the image is zoomed to cover its space and the user presses Escape or Space
- **THEN** the view closes — there is no dark area left to click while it covers the window

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

### Requirement: The library screen keeps its search across screens
The tag query, the free-text query and the result they produced SHALL survive leaving the
library screen for another screen and coming back, within one run of the app; the trash SHALL
keep its own in the same way. Coming back SHALL show the result of the kept query re-run
against the library as it now is, since the other screen may have changed it. The selection
SHALL NOT survive: a return starts with nothing selected.

#### Scenario: A trip to the trash
- **WHEN** the library screen shows the result of `cat rating:s` and the user opens the trash, restores an image, and returns to the library
- **THEN** the fields still read `cat rating:s`, the result is that query's, and the restored image is in it if it matches

#### Scenario: A restart
- **WHEN** the user searches `cat` and restarts the app
- **THEN** the library opens with empty search fields
