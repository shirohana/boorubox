## MODIFIED Requirements

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
