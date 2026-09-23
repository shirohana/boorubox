## MODIFIED Requirements

### Requirement: Categories can be hidden from the sidebar
The sidebar's tag list SHALL carry, on its heading's row at the right-hand edge, one toggle per tag category in the
app's one category order — artist, copyright, character, general, meta — each drawn in its
category's colour, so the row reads as the list's colour legend (owner, 2026-09-24: first drawn on
their own row under the heading; on the heading's row they cost no height and sit where the
section's controls are looked for). Acting on a toggle SHALL hide
that category's tags from the list, and acting on it again SHALL show them; a hidden
category's toggle SHALL be drawn dimmed, and each toggle SHALL name its category and state to
assistive technology as a pressed or unpressed button. A tag the search includes, excludes or
names in an or-group SHALL stay listed, in its marking, whatever its category, so a term can
always be taken out of the search from where it is shown. Which categories are hidden SHALL be
kept with the app's other preferences on this machine, not in the library, so the choice
survives a restart and is the same for every library opened on the machine; no category is
hidden until the user hides one. Hiding SHALL change only the sidebar's list: the counts, the
results, the inspector's tags, the tile footers and the tag editor SHALL show every category as
before.

#### Scenario: Hiding a category
- **WHEN** the result carries `kantoku` (artist) and `1girl` (general), and the user acts on the artist toggle
- **THEN** the list shows `1girl` and not `kantoku`, the artist toggle is dimmed, and the grid is unchanged

#### Scenario: Showing it again
- **WHEN** the artist category is hidden and the user acts on the artist toggle
- **THEN** `kantoku` is listed again in its place and the toggle is drawn at full strength

#### Scenario: A searched tag of a hidden category stays
- **WHEN** the artist category is hidden and the search reads `-kantoku` with `kantoku` an artist tag
- **THEN** `kantoku` is listed first, marked as excluded, and acting on it takes `-kantoku` out of the search

#### Scenario: Survives a restart
- **WHEN** the user hides the meta category and restarts the app
- **THEN** the meta toggle is dimmed and meta tags are left out of the list

#### Scenario: Every category hidden
- **WHEN** all five categories are hidden and the search reads `cat`
- **THEN** the list shows only `cat`, marked, and the five toggles stay on screen to show a category again

#### Scenario: Every category hidden, nothing searched
- **WHEN** all five categories are hidden, the search is empty and the result carries tags
- **THEN** the list shows no tags and says that the tags here are hidden, not that the results have none

#### Scenario: The toggles as the legend
- **WHEN** the sidebar is shown
- **THEN** the five toggles appear on the Tags heading's row, right-aligned, in the order artist, copyright, character, general, meta, each in the colour its category's tags are drawn in

#### Scenario: The inspector is not filtered
- **WHEN** the artist category is hidden and the user selects an image tagged `kantoku` (artist)
- **THEN** the inspector lists `kantoku` in the artist colour
