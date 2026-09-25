## MODIFIED Requirements

### Requirement: Rules are created, changed and deleted from settings
The app SHALL show the library's rules in its settings, listing for each its name, its
pattern, whether the pattern is a regular expression, the tags it adds, whether it is enabled
and whether it is invalid, and SHALL offer creating a rule, editing one, enabling or disabling
one without editing it, and deleting one. The listing SHALL fit the settings page's width
without a sideways scroll, one entry per rule with its fields stacked, so every control of a
rule is on screen at once (owner, 2026-09-25). A rule SHALL require a name and at least one tag. A
rule whose pattern is empty SHALL be shown as matching everything, because that is what it
does. Deleting a rule SHALL NOT change any image.

#### Scenario: Creating a rule
- **WHEN** the user fills in a name, a pattern and two tags and saves
- **THEN** the rule is listed and applies to the next image that enters the library

#### Scenario: A rule without tags
- **WHEN** the user saves a rule naming no tags
- **THEN** the save is refused with the reason and no rule is created

#### Scenario: Turning a rule off
- **WHEN** the user disables a rule
- **THEN** it stays in the list, is shown as disabled, and no longer applies to arriving images

#### Scenario: The pattern that matches everything
- **WHEN** a rule's pattern is empty
- **THEN** the list says so rather than showing an empty field

#### Scenario: Deleting
- **WHEN** the user deletes a rule that has tagged images
- **THEN** the rule is gone and every image it tagged keeps its tags

#### Scenario: Every control on screen
- **WHEN** the settings page is at its narrowest width and a rule has a long pattern and ten tags
- **THEN** its switch, edit and delete controls are visible without scrolling sideways, and the pattern and tags wrap
