# stamps Specification

## Purpose
Stamps: edits written once in the tag language and applied to images by a click, and the edit
mode in which they are applied.

## Requirements

### Requirement: A stamp is an edit written in the tag language
A stamp SHALL be a text whose space-separated tokens each mean one of: `tag` add the tag,
`-tag` remove it, `collection:name` put the image in that collection, `-collection:name` take
it out, `rating:g|s|q|e` set the rating, and a category prefix (`artist:name` and the others
`tag-vocabulary` names) add the tag and create it under that category. A rating token SHALL
set the rating and never clear or toggle it; the last rating token wins. A token that is a
search-only metatag (`is:`, `tagcount:`, `gentags:`, `arttags:`, `chartags:`, `copytags:`,
`metatags:`, `account:`, `or`) or `-rating:` SHALL make the text invalid with a reason naming
the token; an empty text SHALL be invalid. A collection named that does not exist SHALL refuse
the apply with a reason naming it; nothing SHALL be created by a stamp except tags.

#### Scenario: Tags both ways
- **WHEN** the stamp `cat animal -dog` is applied to an image tagged `dog`, `cute`
- **THEN** the image is tagged `cat`, `animal`, `cute`

#### Scenario: Moving between collections
- **WHEN** the stamp `collection:cute -collection:uncategorized` is applied to an image in `Uncategorized`
- **THEN** the image is in `Cute` and not in `Uncategorized`

#### Scenario: A rating sets and stays
- **WHEN** the stamp `rating:g` is applied twice to an image rated `s`
- **THEN** the image is rated `g` after the first apply and still `g` after the second

#### Scenario: Not an edit
- **WHEN** the user saves a stamp reading `cat is:png`
- **THEN** the save is refused naming `is:png` as not an edit

#### Scenario: A category count is not an edit
- **WHEN** the user saves a stamp reading `touhou copytags:0`
- **THEN** the save is refused naming `copytags:0` as not an edit

#### Scenario: A collection that does not exist
- **WHEN** the stamp `collection:nope` is applied and no collection is named Nope
- **THEN** the apply is refused naming `nope`, and the image is unchanged

#### Scenario: A category conflict
- **WHEN** `cat` is a general tag and the stamp `artist:cat` is applied
- **THEN** the apply is refused with `tag-vocabulary`'s reason, and the image is unchanged

### Requirement: An apply is one write
Applying a stamp to one image or to a selection SHALL write every part of it — tags,
collections, rating, new categorised tags — in one operation that applies to every named image
or to none. A part that changes nothing on an image (adding a tag it has, removing it from a
collection it is not in) SHALL leave that image's other parts unaffected. An apply that adds or
removes a tag or sets the rating SHALL record the image as changed; one that only moves
between collections SHALL NOT, as the collection menus do not.

#### Scenario: Half of it would fail
- **WHEN** a stamp naming a collection that does not exist is applied to fifty images
- **THEN** no image of the fifty is changed and the reason is shown once

#### Scenario: Only a collection
- **WHEN** the stamp `collection:cute` is applied to an image
- **THEN** the image is in `Cute` and its last-changed time is unchanged

### Requirement: Stamps are kept with the library and managed on the settings screen
The user SHALL be able to create, edit and delete stamps, each with a name and a text, on the
Stamps page of Settings, and from the stamp bar. A stamp's text SHALL be checked as
it is saved, and a save with an invalid text SHALL be refused with the reason and the text
kept. Stamps SHALL be listed, on the page and in the bar alike, in an order the user sets: a new
stamp lands last, and the Stamps page SHALL offer moving a stamp by dragging its handle and by
"Move up" and "Move down" on its row (owner, 2026-10-01, reversing the 2026-09-23
non-goal: creation order was the order at four stamps and stopped being at a daily working
set). The bar SHALL follow the new order at once. A drag held near the top or bottom of the
page SHALL scroll it, so a long list can be dragged across (owner, 2026-10-02). They SHALL belong to the library,
not to the machine, and SHALL be described in the library's own file so a rebuild restores
them.

#### Scenario: Create from settings
- **WHEN** the user creates a stamp named Cat with the text `cat animal`
- **THEN** it is listed on the Stamps page and in the stamp bar, and the library's own file describes it

#### Scenario: Edit the text
- **WHEN** the user changes Cat's text to `cat animal -dog`
- **THEN** the next apply removes `dog` too

#### Scenario: Invalid on save
- **WHEN** the user saves a text reading `cat or dog`
- **THEN** the save is refused naming `or`, and the form keeps the text

#### Scenario: Drag to reorder
- **WHEN** the stamps are Cat, Dog, Bird and the user drags Bird's handle above Cat
- **THEN** the page and the bar both list Bird, Cat, Dog

#### Scenario: Move by the row's menu
- **WHEN** the stamps are Cat, Dog, Bird and the user chooses "Move down" on Cat
- **THEN** the order is Dog, Cat, Bird, and "Move up" on Cat restores Cat, Dog, Bird

#### Scenario: A new stamp lands last
- **WHEN** the stamps are Bird, Cat, Dog and the user saves a stamp Fish
- **THEN** the order is Bird, Cat, Dog, Fish

#### Scenario: The order survives a rebuild
- **WHEN** the order is Bird, Cat, Dog and the library is rebuilt from its folder
- **THEN** the order is Bird, Cat, Dog afterwards

#### Scenario: Drag past the page's edge
- **WHEN** the Stamps page is longer than the window and the user drags a stamp's handle to the bottom edge and holds it
- **THEN** the page scrolls until the target row is under the pointer, and letting go lands the stamp there

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
SHALL fill the field with its text, and activating the stamp whose text the field already
holds SHALL empty it — the pressed chip is a toggle (owner, 2026-10-01, reversing the
2026-09-23 reading that a chip only fills); the field SHALL also offer a clear control inside
it, at its end, while it holds text; the field's text, one-off or not, SHALL be savable as a
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

#### Scenario: The active stamp clears on a second click
- **WHEN** the stamp Cat is active and the user clicks its chip again
- **THEN** the field is empty, no stamp is active, and a click on a thumbnail opens the viewer

#### Scenario: Clearing a typed stamp
- **WHEN** the user has typed `cat -dog` into the field and clicks the clear control at its end
- **THEN** the field is empty and no stamp is active
