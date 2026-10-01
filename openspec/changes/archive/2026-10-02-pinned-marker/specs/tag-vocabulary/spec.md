## ADDED Requirements

### Requirement: A pinned tag is marked where it is read
Wherever a pinned tag is drawn as a row of the sidebar's tag list or as one of the described
image's own tags in the inspector — beside the grid and inside the viewer alike — a small
muted dot SHALL follow its name, no wider than the text's x-height, with no background, no
change of weight and no underline, so it cannot be mistaken for the search's marking (owner,
2026-10-01). A tag that is not pinned SHALL show no dot. The pinned chips, the tile footers,
the editor and the completion list SHALL NOT carry it.

#### Scenario: The dot in the sidebar
- **WHEN** `1girl` is pinned and `cat` is not, and both are in the results
- **THEN** the sidebar's `1girl` row shows a small muted dot after its name and `cat`'s row shows none

#### Scenario: The dot in the viewer
- **WHEN** `1girl` is pinned and the viewer shows an image carrying it
- **THEN** the viewer's panel lists `1girl` with the dot after it, and the dot carries no background or underline

#### Scenario: Active and pinned
- **WHEN** `1girl` is pinned and the search includes it
- **THEN** its sidebar row carries the search's tint and the dot, and the dot alone marks a pinned tag the search does not include
