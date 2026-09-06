# library-browse Specification

## Purpose
The user browses the library as a grid, narrows it with Danbooru-style tag search, and views
images full size, with counts that make a migration verifiable.

## Requirements

### Requirement: Grid shows the library
The library UI SHALL show non-deleted images as a grid of thumbnails, newest capture first,
and SHALL stay responsive with ten thousand images. Each thumbnail SHALL be shown whole,
scaled to fit its tile without cropping, and the image SHALL be the tile: any text about it
SHALL appear only while the tile is hovered or focused. The tile size SHALL be adjustable from
the toolbar, and the setting SHALL survive a restart.

#### Scenario: Open library
- **WHEN** a library with images is opened
- **THEN** thumbnails render in capture-time order, newest first, without loading every full image

#### Scenario: Mixed shapes
- **WHEN** the library holds both a tall and a wide image
- **THEN** both are shown complete inside equally sized tiles, neither cropped nor stretched

#### Scenario: Hovering a tile
- **WHEN** the pointer rests on a tile, or the tile is focused with the keyboard
- **THEN** its title, source and capture date appear over the image and disappear when it is left

#### Scenario: Changing the tile size
- **WHEN** the user moves the thumbnail size control
- **THEN** the grid re-flows to the new size, keeps the images uncropped, still renders only the tiles near the viewport, and opens at that size after a restart

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
Activating a thumbnail — by double click, Enter or Space — SHALL open the full-size image
scaled to fit the window without cropping, with keyboard navigation to the previous and next
image in the current result order, and Escape SHALL close it. A single click SHALL focus a
thumbnail without opening it. The full-size view SHALL have two modes: the image alone, and
the image beside the inspector panel; the user SHALL be able to move between them without
closing the view. Closing SHALL return focus to the thumbnail of the image the view showed
last — the one it was opened from when it was not moved.

#### Scenario: Navigate
- **WHEN** the lightbox is open and the right arrow is pressed
- **THEN** the next image in the current search result is shown

#### Scenario: Click then open
- **WHEN** the user clicks a thumbnail once
- **THEN** the thumbnail is focused and its facts are shown in the inspector, and the full-size view does not open

#### Scenario: Fit to the window
- **WHEN** an image larger than the window is opened
- **THEN** the whole image is visible, scaled down, with no part cropped and no scrollbars

#### Scenario: Inspect mode
- **WHEN** the inspect key is pressed while the full-size view is open
- **THEN** the inspector panel appears beside the image, the image is refitted to the space that is left, and pressing it again returns to the image alone

#### Scenario: Closing returns focus
- **WHEN** the full-size view is closed without having moved to another image
- **THEN** focus returns to the thumbnail it was opened from, and the arrow keys move the grid focus again

#### Scenario: Closing after moving on
- **WHEN** the user moves to the next image in the full-size view and closes it
- **THEN** the grid focuses that image's thumbnail, scrolled into view, and the inspector shows it

### Requirement: Per-source counts
The UI SHALL show the total image count and the count per source (`extension`, `local`,
`legacy-bundle`) for the whole library, independent of the current search. The per-source
counts SHALL live on a screen the user reaches from the library without running a search, and
SHALL NOT occupy the browsing screen; the total SHALL remain visible while browsing.

#### Scenario: Counts after mixed ingest
- **WHEN** the library holds 3 extension captures and 2 local imports
- **THEN** the counts show total 5, extension 3, local 2, legacy-bundle 0

#### Scenario: Counts do not follow the search
- **WHEN** a search is narrowing the grid to one image
- **THEN** the total shown while browsing and the per-source counts still describe the whole library

#### Scenario: Reaching the counts
- **WHEN** the user wants to reconcile the library against another source of the same images
- **THEN** the per-source counts are two clicks away from the grid, with no search involved
