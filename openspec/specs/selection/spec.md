# selection Specification

## Purpose
Which images the next action applies to. The user marks a set of images in the grid with the
pointer or the keyboard, sees how many are marked, and keeps that set while working; every bulk
action reads it and nothing else.

## Requirements

### Requirement: A selection is entered deliberately
Clicking a thumbnail without a modifier SHALL only focus it, as browsing already does, and SHALL
clear any selection. The app SHALL start or change a selection only through a deliberate act:
holding the platform's multi-select modifier while clicking a thumbnail, holding shift while
clicking one, the per-tile selection control, the keyboard bindings below, or the toolbar's
select-all action.

A multi-select click made while nothing is selected SHALL select the clicked thumbnail and also
the thumbnail the user was standing on — the one last clicked, or last reached with the arrow
keys — when that is a different thumbnail, so the two clicks that Finder and Explorer users make
to select two images select two images here. The per-tile selection control SHALL select only
its own image: a checkbox names exactly the image it is drawn on.

While edit mode is on with an active stamp (`stamps`), a plain click on a thumbnail SHALL apply
the stamp to that image instead of focusing it and clearing the selection; every other gesture
above SHALL select as it does outside the mode.

#### Scenario: Looking at an image
- **WHEN** the user clicks a thumbnail with no modifier held
- **THEN** that thumbnail is focused, its facts are shown, and nothing is selected

#### Scenario: Toggling one image
- **WHEN** images are already selected and the user holds the multi-select modifier and clicks a thumbnail
- **THEN** that image joins the selection, and clicking it the same way again removes it while the rest of the selection stays

#### Scenario: The second image
- **WHEN** nothing is selected, the user clicks thumbnail A with no modifier, then holds the multi-select modifier and clicks thumbnail B
- **THEN** A and B are both selected and the count reads two

#### Scenario: Arrowed to, then picked up
- **WHEN** nothing is selected, the focus was moved to thumbnail A with the arrow keys, and the user modifier-clicks thumbnail B
- **THEN** A and B are both selected

#### Scenario: The same thumbnail
- **WHEN** nothing is selected, the user clicks thumbnail A, then modifier-clicks A again
- **THEN** only A is selected

#### Scenario: The checkbox names one image
- **WHEN** nothing is selected, the user clicks thumbnail A, then ticks the selection control on thumbnail B
- **THEN** only B is selected

#### Scenario: A range
- **WHEN** the user clicks one thumbnail, then holds shift and clicks another
- **THEN** every image between the two in the current result order, both ends included, is selected and nothing else is

#### Scenario: Clearing by clicking away
- **WHEN** images are selected and the user clicks a thumbnail with no modifier
- **THEN** the selection is emptied and only the clicked thumbnail is focused

#### Scenario: A plain click in edit mode
- **WHEN** edit mode is on with an active stamp, three images are selected, and the user clicks a fourth thumbnail with no modifier
- **THEN** the stamp is applied to the fourth image and the three stay selected

### Requirement: The keyboard selects
The app SHALL extend the browsing keyboard map with the bindings below, SHALL NOT act on any of
them while the focus is in a text field or inside a dialog, and SHALL move the focused card by the
same rules browsing already uses, stopping at the edges. A shift movement SHALL extend the
selection from the card the user started it on: that card SHALL stay the anchor for as long as
shift movements continue, and only a movement or a click without shift SHALL move it.

| Where | Key | Action |
| --- | --- | --- |
| Grid | shift + `←` `→` `↑` `↓` | move the focus and select from the anchor to it |
| Grid | shift + `Home` `End` | move the focus to the first / last image and select from the anchor to it |
| Library screen | select-all shortcut | select every image in the current result |
| Library screen | `Esc` | clear the selection |

#### Scenario: Growing a selection with the keyboard
- **WHEN** a card is focused and the user presses shift with the right arrow twice
- **THEN** the focused card and the two after it are selected, and the focus is on the last of them

#### Scenario: Shrinking it again
- **WHEN** three images are selected by shift-arrow and the user presses shift with the left arrow
- **THEN** the selection is the two images between the anchor and the new focus, and the third is no longer selected

#### Scenario: A shift movement keeps its starting point
- **WHEN** the user shift-arrows three cards away from where they started and then shift-arrows back one
- **THEN** the selection runs from the card they started on to the card the focus is on now, and the card they started on is never dropped from it

#### Scenario: Selecting everything
- **WHEN** the user presses the select-all shortcut while the library is on screen and nothing is being typed into
- **THEN** every image matching the current search is selected, including the ones no thumbnail has been drawn for, whether or not a thumbnail was clicked first

#### Scenario: The shortcut under a dialog
- **WHEN** a dialog or the full-size viewer is open and the user presses the select-all shortcut
- **THEN** the selection is unchanged

#### Scenario: Escape clears
- **WHEN** images are selected and the user presses Escape with the library on screen, outside a text field, a dialog and the viewer
- **THEN** the selection is empty, and the focused card and the current search are unchanged

#### Scenario: Typing a query
- **WHEN** the focus is in a search field and the user presses the select-all shortcut or a shift-arrow
- **THEN** the field's own text selection behaves normally and no image is selected

### Requirement: A selection is visible and counted
A selected image SHALL be marked as such in the grid, and the app SHALL show how many images are
selected. The count SHALL be the true size of the selection even when part of it covers rows the
app has not yet drawn or loaded.

#### Scenario: Marked tiles
- **WHEN** an image is selected
- **THEN** its tile is visibly marked, and the mark disappears when it is deselected

#### Scenario: Counting beyond what is loaded
- **WHEN** the result holds ten thousand images and the user selects them all
- **THEN** the count shows ten thousand immediately, without the app first loading ten thousand records

#### Scenario: No selection
- **WHEN** nothing is selected
- **THEN** no count and no selection actions are shown

### Requirement: A selection belongs to one result set
The selection SHALL survive an action that changes the images it holds and the app re-reading
the same search, and SHALL be emptied when the search itself changes, so a count never describes
a result the user is no longer looking at.

Whenever a write makes the app re-read the current search — a bulk tag edit, a bulk rating, an
image moved to or out of the trash, a collection changed from the sidebar — every selected image
the re-read search no longer matches SHALL leave the selection, and every one it still matches
SHALL stay. The count, the selection's thumbnail strip and the next action over the selection
SHALL therefore describe only images the main area shows. An image edited from the inspector
alone, which stays on screen until the next search, stays selected with it.

When the search is changed by acting on a tag or an account shown in the inspector, the
selection SHALL be emptied as for any new search, but the image the panel was describing SHALL
remain the focused image, at its row in the new result, when it is in that result.

#### Scenario: After a bulk edit
- **WHEN** a bulk action is applied and the grid re-reads the current search
- **THEN** the same images are still selected, and the count is unchanged unless the edit removed images from the result

#### Scenario: Edited out of the result
- **WHEN** the search reads `tagme`, fifty images are selected, and a bulk edit removes `tagme` from twenty of them
- **THEN** those twenty leave the grid and the selection, the count reads thirty, and the thumbnail strip shows none of the twenty

#### Scenario: Rated out of the result
- **WHEN** the search reads `rating:s`, five images are selected, and the user rates the selection `q`
- **THEN** the grid is empty of them, nothing is selected, and the selection actions disappear

#### Scenario: A new search
- **WHEN** the user changes the tag query or the free-text query
- **THEN** the selection is emptied and the selection actions disappear

#### Scenario: A search from the inspector over a selection
- **WHEN** three images are selected, the panel describes the focused one, and the user acts on one of its tags
- **THEN** the selection is emptied, the selection actions disappear, and that image is the focused image in the new result

### Requirement: The inspector follows the selection
The inspector panel SHALL show the selection when there is one and the focused image otherwise.
With exactly one image selected it SHALL show that image's facts as it does for a focused image.
With two or more it SHALL show the count and thumbnails of images in the selection instead of one
image's fields. Each of those thumbnails SHALL open its image in the full-size viewer when it is
pressed, and SHALL offer a control that takes that one image out of the selection and leaves the
rest of the selection alone.

#### Scenario: One selected
- **WHEN** exactly one image is selected
- **THEN** the inspector shows that image's facts

#### Scenario: Several selected
- **WHEN** four images are selected
- **THEN** the inspector's header says four are selected and shows their thumbnails, and no single image's fields are shown

#### Scenario: Looking at one of them
- **WHEN** the user presses one of the thumbnails in the inspector's selection
- **THEN** that image opens in the full-size viewer, and the selection is unchanged

#### Scenario: Taking one back out
- **WHEN** the user uses a thumbnail's remove control
- **THEN** that image is no longer selected, every other selected image still is, and the count drops by one

#### Scenario: Down to one
- **WHEN** removing thumbnails leaves exactly one image selected
- **THEN** the inspector shows that image's facts, as it does for any single selected image

#### Scenario: Selection cleared
- **WHEN** the selection is emptied
- **THEN** the inspector returns to the focused image, or to its empty state if none is focused
