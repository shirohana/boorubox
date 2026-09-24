## MODIFIED Requirements

### Requirement: Tags on screen are search terms
Every tag shown for an image SHALL be usable as a search term: acting on it SHALL add it to
the tag search, and acting on a tag the search already includes SHALL take it out again. The
app SHALL also offer excluding it, which SHALL add it as an exclusion instead. A tag added or
removed this way SHALL leave the rest of the query intact. A pinned tag's chip SHALL offer
the same two actions from its context menu, in both placements of the panel and over a
selection, since the chip is a tag on screen like any other (owner, 2026-09-24).

Each tag shown for an image SHALL be drawn as plain text in its category's colour, not as a
pill: a pill's own fill competes with the marking that says the tag is in the search (owner,
2026-09-23). Each SHALL show whether the search includes it or excludes it, in the one
marking the tag sidebar uses, so the list reads as a set of toggles; that marking SHALL be a
background on the tag's whole box, with no underline and no strike-through — an underline
makes English hard to read, and a tag already carrying its category's colour struck through
is one colour too many (owner, 2026-09-23). The marking SHALL sit beside the tag's category
colour, not replace it; an included tag SHALL also carry extra weight, the one change of
weight the marking makes, since a background alone is easy to miss as the only cue once the
underline is gone. The tags SHALL be shown grouped by category in the app's one category
order, alphabetical within a group.

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

#### Scenario: Searching from a pinned chip
- **WHEN** `tagme` is pinned, the search is empty, and the user opens the chip's context menu and chooses "Search for this tag"
- **THEN** the search reads `tagme`, and no image's tags changed

#### Scenario: Excluding from a pinned chip
- **WHEN** `tagme` is pinned, the search reads `cat`, and the user chooses "Exclude from the search" on the chip
- **THEN** the search reads `cat -tagme`

#### Scenario: The rest of the query survives
- **WHEN** the search reads `cat rating:s is:png` and the user adds `dog`
- **THEN** the search still carries the rating and file-type terms and now also `dog`

#### Scenario: The tag shows it is active
- **WHEN** the search reads `cat -dog` and the panel shows an image tagged `cat`, `dog` and `bird`
- **THEN** `cat` is marked as included, `dog` as excluded, and `bird` as neither; `dog` and `bird` are the same weight and `cat` is heavier

#### Scenario: Active and coloured
- **WHEN** the search reads `kantoku` and the panel shows an image tagged `kantoku` (artist)
- **THEN** the text carries both the included marking and the artist colour

#### Scenario: The inspected image stays current
- **WHEN** the panel describes an image tagged `cat` at row 40 of an empty search, and the user acts on `cat`
- **THEN** the search reads `cat`, the same image is current at its row in the new result, the grid has scrolled to it, and the panel still describes it

#### Scenario: The inspected image leaves the result
- **WHEN** the panel describes an image tagged `cat` and the user excludes `cat`
- **THEN** the search reads `-cat` and no image is current
