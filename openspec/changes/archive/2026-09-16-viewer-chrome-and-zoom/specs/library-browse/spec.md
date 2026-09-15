## MODIFIED Requirements

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
