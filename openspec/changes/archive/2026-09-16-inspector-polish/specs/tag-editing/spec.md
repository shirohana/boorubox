## MODIFIED Requirements

### Requirement: Tags on screen are search terms
Every tag shown for an image SHALL be usable as a search term: acting on it SHALL add it to
the tag search, and acting on a tag the search already includes SHALL take it out again. The
app SHALL also offer excluding it, which SHALL add it as an exclusion instead. A tag added or
removed this way SHALL leave the rest of the query intact.

Each tag shown for an image SHALL show whether the search includes it or excludes it, in the
same marking the tag sidebar uses, so the list reads as a set of toggles.

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

#### Scenario: The inspected image stays current
- **WHEN** the panel describes an image tagged `cat` at row 40 of an empty search, and the user acts on `cat`
- **THEN** the search reads `cat`, the same image is current at its row in the new result, the grid has scrolled to it, and the panel still describes it

#### Scenario: The inspected image leaves the result
- **WHEN** the panel describes an image tagged `cat` and the user excludes `cat`
- **THEN** the search reads `-cat` and no image is current

## ADDED Requirements

### Requirement: An X account on screen is a search term
For an image whose page address names an X account, the inspector SHALL show that account's
handle as the first entry under the tag editor, marked apart from the tags (blue), on its own
row above them. Acting on it SHALL add `account:<handle>` to the tag search, and acting on it
while the search already names that account SHALL take the term out again; it SHALL show
whether the search includes or excludes it the way a tag does. The handle shown SHALL be the
one the search matches: derived by the same rule from the same page address, so the entry
never names an account the search cannot find. The entry SHALL be absent for an image whose
page address names no X account, and SHALL never be stored as a tag.

#### Scenario: Finding the same artist
- **WHEN** the panel shows an image captured from `https://x.com/alice/status/1` and the user acts on the account entry
- **THEN** the tag search reads `account:alice`, the result is every image whose page address names `alice`, and this image is still current

#### Scenario: Toggling off
- **WHEN** the search reads `cat account:alice` and the user acts on `alice`'s entry
- **THEN** the search reads `cat`

#### Scenario: Not an X page
- **WHEN** the panel shows an image captured from a Pixiv page or imported from a file
- **THEN** no account entry is shown

#### Scenario: X's own pages
- **WHEN** the panel shows an image whose page address is `https://x.com/home`
- **THEN** no account entry is shown
