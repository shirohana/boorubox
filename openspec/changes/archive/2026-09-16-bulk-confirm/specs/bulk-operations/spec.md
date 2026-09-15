## MODIFIED Requirements

### Requirement: A rating is set across the selection
The app SHALL offer, while a selection exists, setting every selected image to one rating or to
no rating at all. As with tags, the change SHALL apply to the whole selection or to none of it.

Rating one selected image SHALL happen immediately. Rating two or more SHALL ask for
confirmation first, naming how many images and which rating they will be set to (or that their
rating will be cleared), because the ratings being replaced cannot be recovered afterwards.
Confirming SHALL apply the write; dismissing SHALL write nothing and SHALL leave the selection
as it was.

#### Scenario: Rating many
- **WHEN** twelve images are selected and the user picks a rating and confirms
- **THEN** all twelve carry it, whatever they carried before

#### Scenario: Clearing ratings
- **WHEN** the user chooses no rating for the selection and confirms
- **THEN** every selected image becomes unrated

#### Scenario: The selection survives
- **WHEN** a bulk rating has been applied
- **THEN** the same images are still selected, so a further action can follow

#### Scenario: One image needs no question
- **WHEN** one image is selected and the user picks a rating
- **THEN** it is written at once, with no confirmation

#### Scenario: Declining
- **WHEN** the whole library is selected, the user picks `g` by mistake and dismisses the confirmation
- **THEN** no image's rating changes and the selection is unchanged
