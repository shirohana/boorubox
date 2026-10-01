## MODIFIED Requirements

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

