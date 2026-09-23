## MODIFIED Requirements

### Requirement: The sidebar lists the tags of the current results with counts
While a library is open the app SHALL show, beside the results, every tag carried by an image
in the current result set with the number of images in that set carrying it, except the tags
of a category the user has hidden, as "Categories can be hidden from the sidebar" describes.
The counts SHALL describe the whole result set, not the part of it that has been loaded for
display, and SHALL change when the search changes. A tag the search includes or excludes SHALL
be listed even when no image in the result carries it, so the filter that produced an empty
result can be undone from where it is shown; a name the search uses that no tag in the library
has SHALL NOT be listed — it is not a tag, and a row for it offers actions that cannot succeed.
The list SHALL show the tags the search includes or excludes first, then the rest, each part
grouped by category in the app's one category order — artist, copyright, character, general,
meta — alphabetical within a group, with no group label: a count order reshuffles the list
with every search, and the owner reads the rest by colour and category rather than a labelled
section (2026-09-23, amended 2026-09-23 — the labels were noise once the colour already told
the story, and the search's own tags read better surfaced at the top than buried in their
category's place).

#### Scenario: Counts describe the whole result
- **WHEN** a search matches 500 images of which 300 are tagged `cat`, and the grid has drawn only the first screenful
- **THEN** the list shows `cat` with 300

#### Scenario: Counts follow the search
- **WHEN** the user narrows the search to `cat`
- **THEN** the counts are recomputed over the narrowed result, and tags that no longer occur in it are gone from the list

#### Scenario: An active tag with no matches left
- **WHEN** the search excludes `dog`, `dog` is a tag of the library, and no image in the result carries `dog`
- **THEN** `dog` is still listed, marked as excluded, with a count of zero

#### Scenario: A name no tag has
- **WHEN** the search reads `a b c` and no tag of the library is named `a`, `b` or `c`
- **THEN** the list shows no tags at all

#### Scenario: Order
- **WHEN** the result carries `1girl`, `kantoku` (artist), `azur_lane` (copyright), `highres` (meta) and `cat`, and the search reads `highres`
- **THEN** the list reads `highres` first (the search's own tag, marked), then `kantoku`, `azur_lane`, `1girl`, `cat` in category order, alphabetical within — no group labels

#### Scenario: The search's own tags surface first
- **WHEN** the result carries `kantoku` (artist) and `1girl`, and the search reads `1girl`
- **THEN** the list reads `1girl` first, marked, then `kantoku` — category order applies within each part, not across the two

## ADDED Requirements

### Requirement: Categories can be hidden from the sidebar
The sidebar's tag list SHALL carry, under its heading, one toggle per tag category in the
app's one category order — artist, copyright, character, general, meta — each drawn in its
category's colour, so the row reads as the list's colour legend. Acting on a toggle SHALL hide
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
- **THEN** the five toggles appear under the Tags heading in the order artist, copyright, character, general, meta, each in the colour its category's tags are drawn in

#### Scenario: The inspector is not filtered
- **WHEN** the artist category is hidden and the user selects an image tagged `kantoku` (artist)
- **THEN** the inspector lists `kantoku` in the artist colour
