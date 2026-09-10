## Purpose

Where an image has already been posted, shown wherever the image is: on its grid tile and in the
Inspector, as a link to the post. This is the label half of the "posted to `<site>`" feature; the
search filter over the same records is a later phase.

## ADDED Requirements

### Requirement: An image shows where it has been posted
An image with a recorded post SHALL show, in the Inspector, the site it was posted to and the
post's identifier, as a link that opens that post on the booru. An image posted to more than one
site SHALL show one entry per site. An image with no recorded post SHALL show nothing about
posting.

#### Scenario: Posted image in the Inspector
- **WHEN** the Inspector shows an image with a recorded post
- **THEN** the site name and post identifier are shown, and following the link opens that post

#### Scenario: Posted to two sites
- **WHEN** an image has a record for two sites
- **THEN** both are listed

#### Scenario: Never posted
- **WHEN** the image has no recorded post
- **THEN** the Inspector shows no posting entry and no empty placeholder

### Requirement: The grid tile marks posted images
A grid tile SHALL carry a mark identifying the image as posted, without opening the Inspector,
and SHALL name the site the image was posted to when the mark is pointed at. The mark SHALL be
drawn whether or not the tile is hovered, and SHALL NOT obscure the thumbnail's subject.

#### Scenario: Scanning the grid
- **WHEN** the grid shows a mix of posted and unposted images
- **THEN** the posted ones are distinguishable at a glance

### Requirement: A site already posted to is not offered again
For a site an image already has a post record for, the app SHALL show the record instead of an
upload action. Sites the image has not been posted to SHALL still be offered.

#### Scenario: Only site already used
- **WHEN** the image has a post record for the only configured site
- **THEN** the Inspector shows the record and offers no upload action

#### Scenario: A second site is configured
- **WHEN** the image has a post record for one of two configured sites
- **THEN** the record is shown for the first and the upload action is offered for the second

### Requirement: A record outlives the site configuration
Removing a configured site SHALL NOT remove the post records made against it. A record whose site
is no longer configured SHALL still identify the site and the post, and MAY omit the link.

#### Scenario: Site removed after posting
- **WHEN** the user removes a site that images have been posted to
- **THEN** those images still show that they were posted, with the site and post identifier
