## MODIFIED Requirements

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
