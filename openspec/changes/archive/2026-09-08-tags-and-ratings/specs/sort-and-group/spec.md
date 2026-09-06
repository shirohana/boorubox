## Purpose

Choosing the order the results come back in, and dividing them into named groups — by the
account a capture came from, or by images that look like copies of each other.

## ADDED Requirements

### Requirement: The result order is chosen by the user
The app SHALL offer sorting the results by capture time, by the time the image was last
changed, by file size and by pixel area, ascending or descending, and SHALL default to newest
capture first. The chosen order SHALL apply to the whole result set, not to the images that
happen to be loaded, and SHALL be stable: paging through a result SHALL show every matching
image exactly once, including images the order cannot tell apart.

#### Scenario: Sorting a result larger than one screen
- **WHEN** the user sorts 5000 matching images by file size, largest first
- **THEN** the first image shown is the largest in the whole result, not the largest of those already loaded

#### Scenario: Ties do not repeat or hide an image
- **WHEN** the result holds images whose sort values are identical and the user scrolls through every one of them
- **THEN** each image appears exactly once

#### Scenario: Sorting by last change
- **WHEN** the user edits the tags of an old capture and sorts by last change, newest first
- **THEN** that image is first

#### Scenario: Default
- **WHEN** a library is opened and no order has been chosen
- **THEN** the results are newest capture first

### Requirement: Results can be grouped
The app SHALL offer grouping the results by nothing, by X account, or by duplicates, and SHALL
show each group under a heading naming it and how many images it holds. Grouping by X account
SHALL show only images whose page address names an account, one group per account, largest
group first and equal sizes by name. Grouping by duplicates SHALL show only images that share
their pixel dimensions and byte size with at least one other image in the result, one group
per such pair of values. Within a group the chosen sort SHALL still apply.

#### Scenario: Grouping by account
- **WHEN** the result holds three images from one account, one from another and two from pages that name no account, and the user groups by X account
- **THEN** two groups are shown, of three and of one, the three first, and the two other images are not shown

#### Scenario: Grouping by duplicates
- **WHEN** the result holds two images of identical dimensions and byte size, and three images each unique in that pair of values
- **THEN** one group of two is shown and the other three images are not

#### Scenario: The count in the heading is the whole group
- **WHEN** a group holds more images than the screen can show
- **THEN** its heading states the full number of images in it

#### Scenario: Leaving grouping
- **WHEN** the user groups by nothing again
- **THEN** every matching image is shown once, in the chosen sort order, with no headings

### Requirement: Groups are correct before every image is loaded
The headings, their counts and the position of each group SHALL be known from the search
itself, so a grouped result SHALL NOT reorganise itself as more of it is loaded. Scrolling to
a group far down a large result SHALL show that group's images under its own heading.

#### Scenario: Scrolling into a group
- **WHEN** a grouped result holds 40 groups and the user scrolls to the last one without visiting the ones between
- **THEN** its heading and count are already right and its images appear beneath it

### Requirement: Sort and grouping are remembered for the session only
The chosen sort and grouping SHALL survive moving between screens and running new searches
within one run of the app, and SHALL start again at the defaults the next time the app is
opened.

#### Scenario: Across a new search
- **WHEN** the user sorts by file size and then types a new search
- **THEN** the new result is sorted by file size

#### Scenario: Across a restart
- **WHEN** the user groups by duplicates and restarts the app
- **THEN** the results are ungrouped and newest capture first
