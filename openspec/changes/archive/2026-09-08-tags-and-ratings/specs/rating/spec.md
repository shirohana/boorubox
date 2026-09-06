## Purpose

Giving one image the safety rating the booru tag system is built around — general, sensitive,
questionable, explicit, or none — and showing it where the image is.

## ADDED Requirements

### Requirement: An image has one rating or none
The app SHALL let the user set the rating of the image shown in the inspector to `g`, `s`,
`q` or `e`, or clear it, and SHALL record the image as changed at that moment. The current
rating SHALL be visible in the control, and clearing SHALL leave the image matching the
unrated filter rather than any rating. The app SHALL refuse a rating that is not one of those
four values or none.

#### Scenario: Set a rating
- **WHEN** the user sets an unrated image to `s`
- **THEN** the image's rating is `s`, the control shows `s`, and its last-changed time is now

#### Scenario: Change a rating
- **WHEN** the user sets an image already rated `s` to `e`
- **THEN** the image's rating is `e` and it no longer matches a search for `rating:s`

#### Scenario: Clear a rating
- **WHEN** the user clears the rating of a rated image
- **THEN** the image has no rating and matches a search for `is:unrated`

#### Scenario: A value that is not a rating
- **WHEN** a rating other than `g`, `s`, `q`, `e` or none is submitted
- **THEN** it is refused with a reason and the image's rating is unchanged

### Requirement: The rating can be set without opening the image
The app SHALL offer the same five choices from a context menu on a thumbnail in the grid, so
a rating can be given to an image the user is not currently inspecting.

#### Scenario: From the grid
- **WHEN** the user opens the context menu of a thumbnail and picks `q`
- **THEN** that image is rated `q`, the grid shows it, and the inspector's current image is unchanged

### Requirement: The rating is visible on the thumbnail
A rated image's thumbnail SHALL carry a badge naming its rating, distinguishable between the
four ratings without reading it. An unrated image's thumbnail SHALL carry no badge, because a
badge for the absence of a rating is noise on the majority of a fresh library.

#### Scenario: Rated and unrated side by side
- **WHEN** the grid shows one image rated `e` and one unrated
- **THEN** the rated one carries an `e` badge and the unrated one carries none

#### Scenario: Badge follows the edit
- **WHEN** an image's rating is changed while its thumbnail is on screen
- **THEN** the badge changes with it, without re-running the search
