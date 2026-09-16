## MODIFIED Requirements

### Requirement: Grouping by account offers the accounts as filters
While the results are grouped by X account the app SHALL show, beside the grid, every account
that any image of the current view names, each with the number of images the search would
return for that account if the search's own account terms were set aside — every other part
of the search honoured — so an account the search does not include is still listed with the
count it would give, and an account matching nothing is listed with zero. The accounts the
search includes or excludes SHALL come first, then the rest by count from highest, equal
counts by name. Each entry SHALL offer including the account in the search and excluding it,
SHALL show which of the two the current search does, and acting on an entry already included
or excluded SHALL take it out of the search; neither action SHALL disturb the rest of the
query. The list SHALL keep its entries on screen while a search runs, so acting on one does
not scroll it away. The list SHALL take its width from the grid, not from the inspector, and
SHALL be absent under any other grouping.

#### Scenario: The rail follows the grouping
- **WHEN** the result holds three images from `alice` and one from `bob`, and the user groups by X account
- **THEN** a list beside the grid reads `alice` 3, `bob` 1, and grouping by nothing removes it

#### Scenario: Include from the rail
- **WHEN** the user includes `alice` from the list
- **THEN** the search gains `account:alice`, the result narrows to her images, `alice` is marked and first, and `bob` is still listed with 1

#### Scenario: Exclude from the rail
- **WHEN** the user excludes `bob` from the list
- **THEN** the search gains `-account:bob`, his images leave the result, and `bob` is still listed, marked as excluded, with the 1 he would give

#### Scenario: Undo from the rail
- **WHEN** the search includes `alice` and the user acts on her entry
- **THEN** `account:alice` leaves the search and the rest of the query is unchanged

#### Scenario: A tag narrows the counts
- **WHEN** the search reads `cat`, tagged on two of `alice`'s images and none of `carol`'s
- **THEN** the list reads `alice` 2 and `carol` 0, and `carol` can still be included

#### Scenario: Clicking far down the list
- **WHEN** the list is scrolled and the user includes an account near its end
- **THEN** the list does not jump back to its top
