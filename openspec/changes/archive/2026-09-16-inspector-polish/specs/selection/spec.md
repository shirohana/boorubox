## MODIFIED Requirements

### Requirement: A selection belongs to one result set
The selection SHALL survive an action that changes the images it holds and the app re-reading
the same search, and SHALL be emptied when the search itself changes, so a count never describes
a result the user is no longer looking at.

When the search is changed by acting on a tag or an account shown in the inspector, the
selection SHALL be emptied as for any new search, but the image the panel was describing SHALL
remain the focused image, at its row in the new result, when it is in that result.

#### Scenario: After a bulk edit
- **WHEN** a bulk action is applied and the grid re-reads the current search
- **THEN** the same images are still selected, and the count is unchanged unless the edit removed images from the result

#### Scenario: A new search
- **WHEN** the user changes the tag query or the free-text query
- **THEN** the selection is emptied and the selection actions disappear

#### Scenario: A search from the inspector over a selection
- **WHEN** three images are selected, the panel describes the focused one, and the user acts on one of its tags
- **THEN** the selection is emptied, the selection actions disappear, and that image is the focused image in the new result
