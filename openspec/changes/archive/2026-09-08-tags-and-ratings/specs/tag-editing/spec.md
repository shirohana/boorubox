## Purpose

Giving one image its tags: an editor beside the image that suggests what the library already
uses, keeps the set clean and sorted, and turns any tag on screen into a search term.

## ADDED Requirements

### Requirement: The tags of one image can be edited
The app SHALL let the user change the tags of the image currently shown in the inspector,
from both of the inspector's placements, and SHALL store the result as the image's whole tag
set. Saving SHALL drop duplicates and blank entries, SHALL leave the set unordered but
present it in one order everywhere it is displayed, and SHALL record the image as changed at
that moment. An empty editor SHALL be a valid save that leaves the image with no tags. The
new tags SHALL be visible in the grid, the inspector and the tag list of the current results
without reopening the library.

#### Scenario: Adding tags
- **WHEN** the user types two tags into the editor of an image that has none and saves
- **THEN** the image carries both tags, they are shown in the same order in the inspector and everywhere else the image's tags appear, and its last-changed time is now

#### Scenario: The same tag twice
- **WHEN** the user saves an editor whose text names one tag twice
- **THEN** the image carries that tag once

#### Scenario: Clearing every tag
- **WHEN** the user empties the editor and saves
- **THEN** the image carries no tags and is still in the library

#### Scenario: Editing from the full-size viewer
- **WHEN** the inspector is shown beside the full-size image and its tags are edited there
- **THEN** the image is changed exactly as it would be from the grid, and the same editor is used

#### Scenario: An edit that names no image
- **WHEN** an edit is submitted for an image that is no longer in the library
- **THEN** the edit is refused with a reason and nothing else in the library changes

### Requirement: A rating written as a tag sets the rating
A tag of the form `rating:g`, `rating:s`, `rating:q` or `rating:e` SHALL set the image's
rating rather than being stored as a tag, matching what the legacy library did with the same
text. Any other tag beginning with `rating:` SHALL be stored as an ordinary tag.

#### Scenario: Rating typed among the tags
- **WHEN** the user saves the tags `cat rating:s`
- **THEN** the image carries the tag `cat`, no tag named `rating:s`, and its rating is `s`

#### Scenario: Not a rating
- **WHEN** the user saves a tag `rating:unknown`
- **THEN** it is stored as a tag and the image's rating is unchanged

### Requirement: The editor suggests tags the library already uses
While a tag is being typed the app SHALL offer tags already used in the library that begin
with what has been typed, most used first, excluding tags already present in the input. It
SHALL NOT offer anything while the token being typed is a metatag or the `or` operator of the
query language. Accepting a suggestion SHALL replace the token being typed, preserving a
leading `-` when the token has one.

#### Scenario: Prefix
- **WHEN** the library uses `cat`, `cathedral` and `dog`, and the user types `cat`
- **THEN** `cathedral` is offered and `dog` is not

#### Scenario: Already typed
- **WHEN** the input already holds `cat` and the user starts typing `cat` again
- **THEN** `cat` is not offered

#### Scenario: Inside a metatag
- **WHEN** the token being typed is `rating:` or `or`
- **THEN** no suggestions appear

#### Scenario: Accepting into an exclusion
- **WHEN** the token being typed is `-cathe` and the suggestion `cathedral` is accepted
- **THEN** the token becomes `-cathedral`

### Requirement: Confirming a tag takes two steps
Confirming SHALL first act on what is pending, then submit: while a suggestion is highlighted
the first confirmation SHALL accept it and finish the token — a space after the tag, or none
if one is already there — leaving the input ready for the next tag; with an unfinished token
and no highlighted suggestion the first confirmation SHALL finish that token the same way;
with nothing pending it SHALL submit the input. Dismissing the suggestions SHALL leave the
typed text untouched.

#### Scenario: Accept, then submit
- **WHEN** the user types a prefix that highlights a suggestion and confirms twice without typing anything else
- **THEN** the first confirmation inserts the suggested tag followed by a space, and the second submits the input

#### Scenario: A tag the library does not have
- **WHEN** the user types a tag no image carries and confirms twice
- **THEN** the typed tag is kept as written and the input is submitted

#### Scenario: Dismissing
- **WHEN** suggestions are showing and the user dismisses them
- **THEN** the list closes, the typed text is unchanged, and the next confirmation submits

### Requirement: A tag can be removed without retyping the set
The app SHALL offer a remove action on each tag shown for an image, and SHALL apply it to
that image alone.

#### Scenario: Remove one tag
- **WHEN** the user removes one tag of an image carrying three
- **THEN** that image carries the other two, no other image changes, and the image's last-changed time is now

### Requirement: Tags on screen are search terms
Every tag shown for an image SHALL be usable as a search term: acting on it SHALL add it to
the tag search, and acting on a tag the search already includes SHALL take it out again. The
app SHALL also offer excluding it, which SHALL add it as an exclusion instead. A tag added or
removed this way SHALL leave the rest of the query intact.

#### Scenario: Click to narrow
- **WHEN** the search is empty and the user acts on the tag `cat`
- **THEN** the tag search reads `cat` and the results are the images tagged `cat`

#### Scenario: Click again to widen
- **WHEN** the search reads `cat dog` and the user acts on `cat`
- **THEN** the search reads `dog`

#### Scenario: Exclude
- **WHEN** the search reads `cat` and the user excludes `dog`
- **THEN** the search reads `cat -dog`

#### Scenario: The rest of the query survives
- **WHEN** the search reads `cat rating:s is:png` and the user adds `dog`
- **THEN** the search still carries the rating and file-type terms and now also `dog`
