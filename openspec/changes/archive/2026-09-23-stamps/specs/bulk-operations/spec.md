## MODIFIED Requirements

### Requirement: Tags are added and removed across the selection
The app SHALL offer, while a selection exists, one action that adds a set of tags to every
selected image and removes another set from every selected image. Adding a tag an image already
carries and removing one it does not carry SHALL both leave that image unchanged. The add list
SHALL read a category prefix as `tag-vocabulary` describes, refusing the whole edit on a
conflict. The edit SHALL apply to the whole selection or to none of it: no run SHALL leave
some selected images edited and others not. It SHALL be the same write a stamp makes, with
only its tag parts filled (`stamps`).

#### Scenario: Adding to many
- **WHEN** fifty images are selected and the user adds two tags
- **THEN** all fifty carry both tags afterwards, and images that already had one of them are unchanged by it

#### Scenario: Adding and removing in one pass
- **WHEN** the user gives both tags to add and tags to remove
- **THEN** each selected image ends with the added tags present and the removed tags absent

#### Scenario: A failure part-way
- **WHEN** the edit cannot be completed
- **THEN** no selected image is left changed, and the app says the edit did not apply

#### Scenario: Beyond what is loaded
- **WHEN** the selection covers images no thumbnail has been drawn for
- **THEN** those images are edited too

#### Scenario: A new artist across the selection
- **WHEN** no tag `kantoku` exists and the user adds `artist:kantoku` to fifty images
- **THEN** all fifty carry `kantoku` and it is an artist tag
