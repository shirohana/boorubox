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
