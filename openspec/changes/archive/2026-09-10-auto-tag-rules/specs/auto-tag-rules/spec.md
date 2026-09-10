## Purpose

Tagging an image from what its source already said about it: named rules that watch the title
and the context a capture carried, add their tags as the image enters the library, and can be
run again over images that are already there.

## ADDED Requirements

### Requirement: A rule names a pattern and the tags it adds
A rule SHALL consist of a name, a pattern, whether that pattern is a regular expression or a
plain substring, the tags it adds, and whether it is enabled. Rules SHALL be stored with the
library, so that copying the library folder carries its rules and opening a different library
shows that library's rules. A rule SHALL NOT remove a tag, change a title, or change anything
about an image other than adding tags — and, through those tags, a rating.

#### Scenario: A rule belongs to its library
- **WHEN** a rule is created in one library and a second library is opened
- **THEN** the second library shows no rules, and reopening the first shows the rule unchanged

#### Scenario: Copying the folder carries the rules
- **WHEN** a library folder holding rules is copied and the copy is opened
- **THEN** the copy holds the same rules

### Requirement: Rules apply as an image enters the library
Every image entering the library from a capture or from a local file import SHALL be matched
against the enabled rules before it is stored, and SHALL be stored carrying the tags of every
rule that matched, in addition to any tags its source supplied. A tag named by two rules SHALL
be carried once. A tag of the form `rating:g`, `rating:s`, `rating:q` or `rating:e` among a
rule's tags SHALL set the image's rating rather than being stored as a tag, exactly as the
same text typed into the tag editor does. A rule SHALL NOT overwrite a rating the source
supplied.

Delivery of an image whose id is already stored SHALL remain unchanged: rules SHALL NOT run
again for it, and it SHALL NOT gain tags from a retry.

An image imported from the legacy extension's export bundle SHALL NOT have rules applied to
it, so that what the bundle carries is what is stored.

#### Scenario: A capture arrives tagged
- **WHEN** an enabled rule adds `fanart` and an image is captured from a page the rule matches
- **THEN** the stored image carries `fanart` and it is visible without reopening the library

#### Scenario: A local import arrives tagged
- **WHEN** an enabled rule adds `scan` and a file whose name the rule matches is imported
- **THEN** the imported image carries `scan`

#### Scenario: Two rules naming the same tag
- **WHEN** two enabled rules both match the same incoming image and both add `fanart`
- **THEN** the image carries `fanart` once

#### Scenario: A rule setting a rating
- **WHEN** a matching rule's tags are `fanart` and `rating:s` and the source supplied no rating
- **THEN** the image carries the tag `fanart`, no tag `rating:s`, and its rating is `s`

#### Scenario: The source already said what the rating is
- **WHEN** a matching rule adds `rating:e` to an image whose source supplied the rating `s`
- **THEN** the image's rating stays `s`

#### Scenario: Retrying a delivery
- **WHEN** a capture is delivered again with an id already stored, after a new rule was added
- **THEN** the stored image is unchanged and gains no tags

### Requirement: A rule is matched against what the source said about the image
A rule's pattern SHALL be matched against the image's title, against the name of the site an
adapter record identifies, and against every text value the adapter record's fields carry,
including the entries of a field holding several values. A match against any one of them SHALL
be a match. For an image imported from a local file, the title is its filename, which is
therefore what the rule sees.

Field names the app has no meaning for SHALL be matched like any other, so an adapter that
starts sending a new field needs no change here. Web addresses that are not adapter fields —
the page address and the image address — SHALL NOT be matched, so that a short pattern does
not match inside an address by accident.

#### Scenario: Matching the title
- **WHEN** a rule's pattern is `commission` and a capture's page title contains it
- **THEN** the rule matches

#### Scenario: Matching an adapter field
- **WHEN** a rule's pattern is an account handle carried in the capture's adapter record, and the title does not contain it
- **THEN** the rule matches

#### Scenario: Matching the site
- **WHEN** a rule's pattern is `pixiv` and a capture's adapter record names that site
- **THEN** the rule matches

#### Scenario: A field the app does not know
- **WHEN** the adapter record carries a field the app has no meaning for and a rule's pattern occurs in its value
- **THEN** the rule matches

#### Scenario: No adapter record
- **WHEN** a capture carries no adapter record
- **THEN** rules are matched against its title alone and the image is stored either way

#### Scenario: Addresses are not matched
- **WHEN** a rule's pattern occurs only in the image's page address and in no title or adapter field
- **THEN** the rule does not match

### Requirement: Matching follows the rules the legacy library used
A disabled rule SHALL never match. A rule whose pattern is empty SHALL match every image. A
rule that is not a regular expression SHALL match when its pattern occurs anywhere in one of
the matched texts, ignoring letter case. A regular-expression rule SHALL match ignoring letter
case. Text that is absent SHALL be treated as empty rather than as a failure.

#### Scenario: Disabled
- **WHEN** a rule whose pattern matches is disabled and an image arrives
- **THEN** the image does not gain that rule's tags

#### Scenario: Empty pattern
- **WHEN** an enabled rule's pattern is empty
- **THEN** every image entering the library gains that rule's tags

#### Scenario: Case
- **WHEN** a rule's pattern is `Pixiv` and an image's title contains `pixiv`
- **THEN** the rule matches

#### Scenario: Regular expression
- **WHEN** a rule is marked as a regular expression with the pattern `^\[.+\]` and an image's title starts with a bracketed word
- **THEN** the rule matches

#### Scenario: Nothing to match against
- **WHEN** an image has no title and no adapter record and a rule's pattern is not empty
- **THEN** the rule does not match and the image is stored

### Requirement: A pattern the app cannot use makes the rule invalid, never an error
A rule marked as a regular expression whose pattern the app cannot interpret SHALL be reported
as invalid wherever rules are listed, with the reason, SHALL be skipped when images are
matched, and SHALL NOT stop any other rule from matching, stop an image from being stored, or
raise an error to the source that delivered it. Saving such a pattern from the rules editor
SHALL be refused with the reason, so a rule written in the app is usable by construction; a
rule that arrives by import SHALL be stored and shown as invalid rather than dropped.

#### Scenario: An invalid pattern skipped at ingest
- **WHEN** one rule's regular expression is unusable and another rule matches an incoming image
- **THEN** the image is stored carrying the second rule's tags and the delivery reports success

#### Scenario: Shown as invalid
- **WHEN** the rules list is shown and one rule's pattern is unusable
- **THEN** that rule is marked invalid with the reason, and its enabled state is unchanged

#### Scenario: Refused in the editor
- **WHEN** the user saves a rule marked as a regular expression whose pattern is unusable
- **THEN** the save is refused with the reason and no rule is created or changed

#### Scenario: Accepted on import
- **WHEN** an imported file carries a rule whose regular expression this app cannot interpret
- **THEN** the rule is imported and listed as invalid

### Requirement: Rules can be run over images already in the library
The app SHALL offer running the current rules over every image already in the library, and
SHALL report how many images were examined, how many changed, and for each rule how many
images it added tags to. A run SHALL only add tags: no tag SHALL be removed, and an image
already carrying a rule's tag SHALL be left unchanged by it. A run SHALL set a rating from a
rule's `rating:` tag only on an image that has no rating, so a rating given by hand survives.
Images in the trash SHALL NOT be examined. Invalid and disabled rules SHALL be skipped, and
the report SHALL name the invalid ones.

#### Scenario: Tagging what is already stored
- **WHEN** a rule is written and run over a library where 12 of 400 images match it
- **THEN** those 12 carry its tags, the report says 400 examined and 12 changed, and the rule's own count is 12

#### Scenario: Running twice
- **WHEN** the same run is performed a second time with no rule changed
- **THEN** the report says nothing changed and no image gains a duplicate tag

#### Scenario: A rating already given
- **WHEN** a rule adding `rating:e` matches an image the user rated `s` and one with no rating
- **THEN** the first stays `s` and the second becomes `e`

#### Scenario: Nothing is taken away
- **WHEN** a run is performed on images carrying tags no rule names
- **THEN** those tags are still there afterwards

#### Scenario: An invalid rule in the run
- **WHEN** a run is performed while one rule's pattern is unusable
- **THEN** the run completes, the other rules apply, and the report names the invalid rule

### Requirement: Rules move between libraries as a JSON file
The app SHALL export the library's rules as a JSON file and SHALL import rules from such a
file. The exported shape SHALL be the one the legacy extension wrote, so that a file exported
by either can be read by this app. An imported rule SHALL be given an identity of its own in
the receiving library, leaving the file's identities out of it. An imported rule whose name,
pattern, kind and set of tags match a rule the library already has SHALL be skipped, and the
result SHALL say how many were imported and how many were skipped. A file that is not a list
of rules SHALL be refused with a reason, leaving the library's rules untouched.

#### Scenario: Round trip
- **WHEN** rules are exported from one library and the file is imported into an empty one
- **THEN** the second library holds the same rules, with their own identities

#### Scenario: Importing the same file twice
- **WHEN** a file is imported and then imported again without changes
- **THEN** the second import adds nothing and reports every rule as skipped

#### Scenario: A rule that differs only by its tags
- **WHEN** an imported rule has the same name and pattern as an existing one but a different tag
- **THEN** it is imported as a second rule

#### Scenario: Order of tags does not make a new rule
- **WHEN** an imported rule names the same tags as an existing one in a different order
- **THEN** it is skipped as a duplicate

#### Scenario: A file that is not rules
- **WHEN** a file that is not a list of rules is imported
- **THEN** the import is refused with a reason and the library's rules are unchanged

### Requirement: Rules are created, changed and deleted from settings
The app SHALL show the library's rules in its settings, listing for each its name, its
pattern, whether the pattern is a regular expression, the tags it adds, whether it is enabled
and whether it is invalid, and SHALL offer creating a rule, editing one, enabling or disabling
one without editing it, and deleting one. A rule SHALL require a name and at least one tag. A
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
