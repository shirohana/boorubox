## MODIFIED Requirements

### Requirement: Edit mode applies the active stamp by a click
The library screen SHALL offer an edit mode, entered and left by a toolbar control — no key
for now: a key belongs to the most frequent day-to-day action, and a mode where a click writes
is not one (owner, 2026-09-23) — and shown plainly while it is on. Entering it SHALL show
a stamp bar above the grid holding a field first — for typing a stamp, or showing the active
one's text — and the saved stamps under it, and SHALL show under every thumbnail the
collections that image is in, each named after the bookmark mark the tile's corner already
uses, in plain text, first — and then the tags it carries, in the app's category order and
colours, the whole cut after three lines with the whole list shown over the row below while
the pointer is on the tile, so what a click would change and what it changed can be read
from the grid (owner, 2026-09-23; collections added 2026-09-24 — dropping an image into a
collection from the inspector's pinned chip needs its answer on the tile, and first because
the clamp cuts the tail, and an image is in a few collections and many tags); outside
the mode the footer follows a view setting the toolbar toggles, kept with the thumbnail size;
in the mode it is always shown. The active
stamp SHALL be whatever the field currently parses to: a blank field or text that does not
parse SHALL leave none active, the same as leaving the mode does. Activating a saved stamp
SHALL fill the field with its text; the field's text, one-off or not, SHALL be savable as a
named stamp at any time. While the mode is on and a stamp is active, a plain click on a
thumbnail SHALL apply the active stamp to that image and SHALL NOT open the viewer; the
thumbnail SHALL show the result at once without the search being re-run; a thumbnail under
the pointer SHALL show what a click there would apply; the modifier and shift clicks, the
checkbox and the keyboard SHALL select and move as they do outside the mode. While a
selection exists the bar SHALL offer applying the active stamp to the whole selection,
asking first when more than one image would be written and naming the count and the stamp,
after which the search SHALL be re-read as for any bulk edit. With no active stamp — the
field cleared, or never filled — a click SHALL do what it does outside the mode. The bar
SHALL say that there is no undo. Leaving the mode SHALL hide the bar, clear the field and
restore the ordinary click; the footers SHALL fall back to the view setting (on if it is on,
gone if it is off).

#### Scenario: Enter and stamp
- **WHEN** the user enters edit mode from the toolbar, activates the stamp Cat, and clicks three thumbnails
- **THEN** each of the three is tagged `cat` and `animal` the moment it is clicked, none opened in the viewer, and the grid still shows all three

#### Scenario: The tile says what it carries
- **WHEN** edit mode is on and a thumbnail's image is tagged `kantoku` (artist), `1girl` and `highres` (meta)
- **THEN** under the thumbnail the three read in that order, each in its category's colour, and after a click with Cat active the footer also reads `cat` and `animal`

#### Scenario: The tile says which collections it is in
- **WHEN** the footer is shown and a thumbnail's image is in the collections Cute and Queue and tagged `1girl`
- **THEN** under the thumbnail the footer reads the bookmark mark, `Cute`, `Queue`, then `1girl` in its category's colour; an image in no collection shows its tags alone

#### Scenario: A second click does not open
- **WHEN** edit mode is on with an active stamp and the user clicks the thumbnail that is already current
- **THEN** the stamp is applied again (changing nothing) and the viewer does not open

#### Scenario: What a click will do
- **WHEN** a stamp is active and the user hovers a thumbnail without clicking
- **THEN** the tile shows "Apply cat animal" over the image, naming exactly what a click there would apply

#### Scenario: Selecting while in the mode
- **WHEN** edit mode is on and the user shift-clicks a thumbnail
- **THEN** a range is selected and no stamp is applied

#### Scenario: Apply to the selection
- **WHEN** twelve images are selected, the active stamp is Cat, and the user chooses to apply it to the selection
- **THEN** the app asks, naming twelve and Cat, and on confirmation all twelve carry `cat` and `animal`

#### Scenario: A one-off stamp
- **WHEN** the user types `rating:g -tagme` into the bar's field
- **THEN** it is active at once, with no separate confirmation, and clicking a thumbnail rates it `g` and removes `tagme`

#### Scenario: A saved stamp fills the field
- **WHEN** the user clicks the saved stamp Cat
- **THEN** the field reads Cat's text, Cat's chip reads pressed, and clicking a thumbnail applies it

#### Scenario: Cancelling
- **WHEN** a stamp is active and the user clears the field
- **THEN** no stamp is active, no chip reads pressed, and a plain click on a thumbnail focuses it as it would outside the mode

#### Scenario: Saving the one-off
- **WHEN** the user chooses to save the one-off and names it Reviewed
- **THEN** Reviewed is a saved stamp with that text, listed in the bar and on the settings screen

#### Scenario: Leaving
- **WHEN** the user leaves edit mode from the toolbar
- **THEN** the bar is gone and a plain click on a thumbnail focuses it as before

#### Scenario: Leaving takes the footers
- **WHEN** the tags-under-thumbnails setting is off and the user leaves edit mode from the toolbar
- **THEN** no thumbnail shows a footer and the grid's rows are the height they were before the mode

#### Scenario: A footer while browsing
- **WHEN** the user is not in edit mode and turns on the tags-under-thumbnails toggle beside the thumbnail-size slider
- **THEN** every thumbnail gains its tag footer at once, the toggle reads pressed, and the setting is still on the next time the app opens
