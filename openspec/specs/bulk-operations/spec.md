# bulk-operations Specification

## Purpose
One tag edit or one rating applied to every selected image at once, so a batch that arrived
together can be described in a single pass instead of one image at a time.

## Requirements

### Requirement: Tags are added and removed across the selection
The app SHALL offer, while a selection exists, one action that adds a set of tags to every
selected image and removes another set from every selected image. Adding a tag an image already
carries and removing one it does not carry SHALL both leave that image unchanged. The edit SHALL
apply to the whole selection or to none of it: no run SHALL leave some selected images edited and
others not.

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

### Requirement: The selection offers its own commonest tags for removal
The tag action SHALL offer the ten tags most often carried by the selected images, each with the
number of selected images carrying it, as one-click shortcuts that put that tag in the set to
remove. The counts SHALL be computed over the whole selection, not over the part currently on
screen, and SHALL be absent when no selected image has a tag.

#### Scenario: Frequencies over the whole selection
- **WHEN** a selection spans images the app has not loaded records for and the tag action is opened
- **THEN** the offered tags and counts describe every selected image

#### Scenario: Picking one
- **WHEN** the user clicks an offered tag
- **THEN** it appears in the set of tags to remove, and clicking it again takes it back out

#### Scenario: Nothing tagged
- **WHEN** no selected image carries any tag
- **THEN** no shortcuts are offered and the tag action still works for adding

### Requirement: A rating is set across the selection
The app SHALL offer, while a selection exists, setting every selected image to one rating or to
no rating at all. As with tags, the change SHALL apply to the whole selection or to none of it.

#### Scenario: Rating many
- **WHEN** twelve images are selected and the user picks a rating
- **THEN** all twelve carry it, whatever they carried before

#### Scenario: Clearing ratings
- **WHEN** the user chooses no rating for the selection
- **THEN** every selected image becomes unrated

#### Scenario: The selection survives
- **WHEN** a bulk rating has been applied
- **THEN** the same images are still selected, so a further action can follow

### Requirement: A bulk edit leaves the library consistent
After a bulk tag edit the app SHALL show the edited images with their new tags without the user
re-running the search, and SHALL NOT leave a tag in the library's vocabulary that no image
carries any more.

#### Scenario: The grid catches up
- **WHEN** a bulk tag edit finishes
- **THEN** the inspector and the tag list show the new tags for the selected images

#### Scenario: A tag removed from its last image
- **WHEN** a bulk removal takes the only remaining use of a tag out of the library
- **THEN** that tag stops being offered as a suggestion and stops appearing in tag counts
