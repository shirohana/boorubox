## MODIFIED Requirements

### Requirement: The sidebar lists the tags of the current results with counts
While a library is open the app SHALL show, beside the results, every tag carried by an image
in the current result set with the number of images in that set carrying it. The counts SHALL
describe the whole result set, not the part of it that has been loaded for display, and SHALL
change when the search changes. A tag the search includes or excludes SHALL be listed even
when no image in the result carries it, so the filter that produced an empty result can be
undone from where it is shown; a name the search uses that no tag in the library has SHALL
NOT be listed — it is not a tag, and a row for it offers actions that cannot succeed. The
list SHALL show the tags the search includes or excludes first, then the rest, each part
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

### Requirement: A tag in the sidebar filters the results
Each listed tag SHALL offer including it in the search and excluding it from the search, and
SHALL show which of the two the current search does, in the one marking the inspector's tags
use. Acting on a tag that is already included or excluded SHALL remove it from the search.
Neither action SHALL disturb the rest of the query. Each listed tag SHALL be drawn in its
category's colour, and SHALL offer on its context menu pinning or unpinning it and choosing
its category, as `tag-vocabulary` describes.

#### Scenario: Include
- **WHEN** the user includes `cat` from the list
- **THEN** the tag search gains `cat` and the results narrow to images tagged `cat`

#### Scenario: Exclude
- **WHEN** the user excludes `dog` from the list
- **THEN** the tag search gains `-dog` and images tagged `dog` leave the result

#### Scenario: Undo from the list
- **WHEN** the user acts on a tag the search already includes
- **THEN** that tag leaves the search and the rest of the query is unchanged

#### Scenario: Coloured rows
- **WHEN** the result carries `kantoku` (artist) and `1girl`
- **THEN** `kantoku` is listed in the artist colour and `1girl` in the general colour

#### Scenario: The row's menu
- **WHEN** the user right-clicks `kantoku` in the list
- **THEN** the menu offers Pin (or Unpin) and the five categories with Artist marked
