## MODIFIED Requirements

### Requirement: The tags of one image can be edited
The app SHALL let the user change the tags of the image currently shown in the inspector,
from both of the inspector's placements, and SHALL store the result as the image's whole tag
set. Saving SHALL drop duplicates and blank entries, SHALL leave the set unordered but
present it in one order everywhere it is displayed, and SHALL record the image as changed at
that moment. An empty editor SHALL be a valid save that leaves the image with no tags. The
new tags SHALL be visible in the grid, the inspector and the tag list of the current results
without reopening the library.

The tag area SHALL be read-only until the user opens the editor with an edit action; saving
or cancelling SHALL close it again, and a refused save SHALL keep it open with the typed
text. The editor SHALL be a field that grows with its text, not a single line: an image
carries dozens of tags and a line that scrolls sideways shows a few of them. It SHALL open
with the image's tags on one line per category that has any, in the order artist, copyright,
character, meta, general, alphabetical within a line, and with a space after the last tag,
so a click that lands at the end is already on a new token — a click into the editor means a
tag is about to be added, and the owner had been typing that space by hand on every edit
(2026-09-12). The lines are presentation: a tag's category comes from the vocabulary and
SHALL NOT be changed by the line it is typed or moved to, and a line break SHALL count as a
space. The space is not a change: a save trims.

#### Scenario: Opening the editor to add a tag
- **WHEN** the user opens the editor of an image that already has tags and clicks at its end
- **THEN** the caret sits after a space and the next keystroke begins a new tag, and nothing is marked as changed until one is typed

#### Scenario: Read-only until opened
- **WHEN** the panel describes an image
- **THEN** its tags are shown as badges and no text field is drawn until the edit action is used

#### Scenario: One line per category
- **WHEN** the user opens the editor of an image tagged `1girl`, `kantoku` (artist), `azur_lane` (copyright), `highres` (meta) and `solo`
- **THEN** the editor reads `kantoku`, then `azur_lane`, then `highres`, then `1girl solo`, each on its own line

#### Scenario: A line does not categorise
- **WHEN** the user moves `solo` onto the artist line and saves
- **THEN** `solo` is still a general tag and the image's tag set is unchanged

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

### Requirement: The editor suggests tags the library already uses
While a tag is being typed the app SHALL offer tags already used in the library that begin
with what has been typed, most used first, excluding tags already present in the input, each
drawn in its category's colour. It SHALL NOT offer anything while the token being typed is a
metatag or the `or` operator of the query language, nor while it begins with a category
prefix. Accepting a suggestion SHALL replace the token being typed, preserving a leading `-`
when the token has one.

#### Scenario: Prefix
- **WHEN** the library uses `cat`, `cathedral` and `dog`, and the user types `cat`
- **THEN** `cathedral` is offered and `dog` is not

#### Scenario: Already typed
- **WHEN** the input already holds `cat` and the user starts typing `cat` again
- **THEN** `cat` is not offered

#### Scenario: Inside a metatag
- **WHEN** the token being typed is `rating:` or `or`
- **THEN** no suggestions appear

#### Scenario: Behind a category prefix
- **WHEN** the token being typed is `artist:kan`
- **THEN** no suggestions appear

#### Scenario: Accepting into an exclusion
- **WHEN** the token being typed is `-cathe` and the suggestion `cathedral` is accepted
- **THEN** the token becomes `-cathedral`

### Requirement: Tags on screen are search terms
Every tag shown for an image SHALL be usable as a search term: acting on it SHALL add it to
the tag search, and acting on a tag the search already includes SHALL take it out again. The
app SHALL also offer excluding it, which SHALL add it as an exclusion instead. A tag added or
removed this way SHALL leave the rest of the query intact.

Each tag shown for an image SHALL show whether the search includes it or excludes it, in the
same marking the tag sidebar uses, so the list reads as a set of toggles; that marking SHALL
sit beside the tag's category colour, not replace it. The tags SHALL be shown grouped by
category in the editor's order, alphabetical within a group.

Acting on a tag from the inspector SHALL keep the image the panel describes as the current
image: after the search re-runs, that image SHALL be current at whatever row it now occupies,
and the panel SHALL still describe it. Only when the image is no longer in the result SHALL the
screen fall back to no current image.

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

#### Scenario: The tag shows it is active
- **WHEN** the search reads `cat -dog` and the panel shows an image tagged `cat`, `dog` and `bird`
- **THEN** `cat` is marked as included, `dog` as excluded, and `bird` as neither

#### Scenario: Active and coloured
- **WHEN** the search reads `kantoku` and the panel shows an image tagged `kantoku` (artist)
- **THEN** the badge carries both the included marking and the artist colour

#### Scenario: The inspected image stays current
- **WHEN** the panel describes an image tagged `cat` at row 40 of an empty search, and the user acts on `cat`
- **THEN** the search reads `cat`, the same image is current at its row in the new result, the grid has scrolled to it, and the panel still describes it

#### Scenario: The inspected image leaves the result
- **WHEN** the panel describes an image tagged `cat` and the user excludes `cat`
- **THEN** the search reads `-cat` and no image is current
