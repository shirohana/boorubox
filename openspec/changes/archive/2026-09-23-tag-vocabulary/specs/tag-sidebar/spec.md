## MODIFIED Requirements

### Requirement: A tag in the sidebar filters the results
Each listed tag SHALL offer including it in the search and excluding it from the search, and
SHALL show which of the two the current search does. Acting on a tag that is already included
or excluded SHALL remove it from the search. Neither action SHALL disturb the rest of the
query. Each listed tag SHALL be drawn in its category's colour, and SHALL offer on its context
menu pinning or unpinning it and choosing its category, as `tag-vocabulary` describes.

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
- **THEN** `kantoku` is listed in the artist colour and `1girl` in the ordinary text colour

#### Scenario: The row's menu
- **WHEN** the user right-clicks `kantoku` in the list
- **THEN** the menu offers Pin (or Unpin) and the five categories with Artist marked
