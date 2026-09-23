## MODIFIED Requirements

### Requirement: The tags of one image can be edited
The app SHALL let the user change the tags of the image currently shown in the inspector,
from both of the inspector's placements, and SHALL store the result as the image's whole tag
set. Saving SHALL drop duplicates and blank entries, SHALL lowercase every name (so `Cat` and
`cat` in one editor are one tag), SHALL leave the set unordered but present it in one order
everywhere it is displayed, and SHALL record the image as changed at that moment. An empty
editor SHALL be a valid save that leaves the image with no tags. The new tags SHALL be visible
in the grid, the inspector and the tag list of the current results without reopening the
library.

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
- **WHEN** the user opens the editor of an image tagged `cat` and `dog`
- **THEN** the field reads `cat dog ` with the caret free to type a third tag at once

#### Scenario: Read-only until opened
- **WHEN** the panel describes an image and the user has not used the edit action
- **THEN** the tags are shown as text and no field is on screen

#### Scenario: One line per category
- **WHEN** the user opens the editor of an image tagged `kantoku` (artist), `1girl`, `highres` (meta) and `azur_lane` (copyright)
- **THEN** the field reads `kantoku`, `azur_lane`, `highres`, `1girl ` on four lines, in that order

#### Scenario: A line does not categorise
- **WHEN** the user moves `1girl` onto the artist line and saves
- **THEN** `1girl` is still a general tag

#### Scenario: Adding tags
- **WHEN** the user opens the editor of an image tagged `cat`, types `dog`, and saves
- **THEN** the image carries `cat` and `dog`, the grid tile and the inspector show both, and the tag list of the results counts `dog` one higher

#### Scenario: The same tag twice
- **WHEN** the user saves `cat cat dog`
- **THEN** the image carries `cat` and `dog`

#### Scenario: Typed in capitals
- **WHEN** the user saves `Cat DOG`
- **THEN** the image carries `cat` and `dog`, and the editor reopens reading `cat dog `

#### Scenario: Clearing every tag
- **WHEN** the user empties the editor and saves
- **THEN** the image carries no tags and is still in the library

#### Scenario: Editing from the full-size viewer
- **WHEN** the user opens the inspector inside the full-size view, edits the tags and saves
- **THEN** the same result as from the grid's inspector

#### Scenario: An edit that names no image
- **WHEN** the panel describes no image
- **THEN** no tag editor is on screen
