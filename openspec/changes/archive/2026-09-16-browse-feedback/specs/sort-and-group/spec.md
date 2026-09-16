## ADDED Requirements

### Requirement: Grouping by account offers the accounts as filters
While the results are grouped by X account the app SHALL show, beside the grid, every account
in the result with the number of its images, in the groups' own order — largest first, equal
sizes by name. Each entry SHALL offer including the account in the search and excluding it,
SHALL show which of the two the current search does, and acting on an entry already included
or excluded SHALL take it out of the search; neither action SHALL disturb the rest of the
query. The list SHALL take its width from the grid, not from the inspector, and SHALL be absent
under any other grouping.

#### Scenario: The rail follows the grouping
- **WHEN** the result holds three images from `alice` and one from `bob`, and the user groups by X account
- **THEN** a list beside the grid reads `alice` 3, `bob` 1, and grouping by nothing removes it

#### Scenario: Include from the rail
- **WHEN** the user includes `alice` from the list
- **THEN** the search gains `account:alice`, the result narrows to her images, and `alice` is marked as included

#### Scenario: Exclude from the rail
- **WHEN** the user excludes `bob` from the list
- **THEN** the search gains `-account:bob` and his images leave the result

#### Scenario: Undo from the rail
- **WHEN** the search includes `alice` and the user acts on her entry
- **THEN** `account:alice` leaves the search and the rest of the query is unchanged
