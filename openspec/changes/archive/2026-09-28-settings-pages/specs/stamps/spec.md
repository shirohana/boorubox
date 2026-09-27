## MODIFIED Requirements

### Requirement: Stamps are kept with the library and managed on the settings screen
The user SHALL be able to create, edit and delete stamps, each with a name and a text, on the
Stamps page of Settings, and from the stamp bar. A stamp's text SHALL be checked as
it is saved, and a save with an invalid text SHALL be refused with the reason and the text
kept. Stamps SHALL be listed in the order they were created. They SHALL belong to the library,
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

