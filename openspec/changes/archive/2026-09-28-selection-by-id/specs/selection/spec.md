## MODIFIED Requirements

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

Whenever the app re-reads the same search — after a write, after a capture is stored, after an
import finishes — the selection SHALL hold the same images it held before, wherever their rows
now are; rows that arrived or moved under a selection SHALL NOT join it or push images out of
it. The focused image and the image a range extends from SHALL likewise stay the same images at
their new rows, and the inspector SHALL keep describing the image it described. A focused image
that is no longer in the result SHALL leave the focus at the row it occupied. A capture or an
import SHALL NOT scroll the grid.

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

#### Scenario: A capture lands above a range
- **WHEN** the result is in capture order, the first three images are selected with shift, and a capture is stored
- **THEN** the new image appears first and is not selected, the same three images are selected at rows two to four, and the count reads three

#### Scenario: A capture lands above the focus
- **WHEN** the fifth image is focused, the inspector describes it, and a capture is stored at the top of the result
- **THEN** the same image is focused at row six, the inspector still describes it, and the arrows move from there

#### Scenario: Everything selected when a capture lands
- **WHEN** every image of a result is selected and a capture matching the search is stored
- **THEN** the count is unchanged, the new image is not selected, and every image selected before still is

#### Scenario: A capture lands inside a range
- **WHEN** the result is ordered by size, a range of images is selected, and a capture whose size places it inside that range is stored
- **THEN** the new image is not selected and every image selected before still is

#### Scenario: An edit reorders the result
- **WHEN** the result is ordered by last update, the tenth image is focused, and a bulk edit changes other images
- **THEN** the edited images move to the top and the same image is still focused at its new row

#### Scenario: The focused image leaves the result
- **WHEN** the focused image is moved to the trash
- **THEN** the focus stays at the row it occupied, on the image that now fills it
