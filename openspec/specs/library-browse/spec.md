# library-browse Specification

## Purpose
The user browses the library as a grid, narrows it with Danbooru-style tag search, and views
images full size, with counts that make a migration verifiable.

## Requirements

### Requirement: Grid shows the library
The library UI SHALL show non-deleted images as a grid of thumbnails, newest capture first,
and SHALL stay responsive with ten thousand images.

#### Scenario: Open library
- **WHEN** a library with images is opened
- **THEN** thumbnails render in capture-time order, newest first, without loading every full image

### Requirement: Tag search uses the legacy query language
A search box SHALL accept the legacy extension's query syntax: space-separated tags are AND,
`a or b` is OR, `-tag` excludes, and the metatags `rating:`, `is:`, `tagcount:` and `account:`
filter as they do in the legacy viewer. Free text in `page title` and URLs SHALL be searchable.

#### Scenario: AND and NOT
- **WHEN** the query is `cat -dog`
- **THEN** only images tagged `cat` and not tagged `dog` are shown

#### Scenario: OR group
- **WHEN** the query is `cat or dog`
- **THEN** images tagged either `cat` or `dog` are shown

#### Scenario: Rating metatag
- **WHEN** the query is `rating:s,q`
- **THEN** only images rated `s` or `q` are shown

#### Scenario: Empty result
- **WHEN** no image matches
- **THEN** the grid shows an empty state naming the query, not a blank page

### Requirement: Lightbox
Activating a thumbnail SHALL open the full-size image with keyboard navigation to the previous
and next image in the current result order, and Escape SHALL close it.

#### Scenario: Navigate
- **WHEN** the lightbox is open and the right arrow is pressed
- **THEN** the next image in the current search result is shown

### Requirement: Per-source counts
The UI SHALL show the total image count and the count per source (`extension`, `local`,
`legacy-bundle`) for the whole library, independent of the current search.

#### Scenario: Counts after mixed ingest
- **WHEN** the library holds 3 extension captures and 2 local imports
- **THEN** the counts show total 5, extension 3, local 2, legacy-bundle 0
