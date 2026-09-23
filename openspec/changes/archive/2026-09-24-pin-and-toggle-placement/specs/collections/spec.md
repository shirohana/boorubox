## MODIFIED Requirements

### Requirement: A collection can be pinned for one-click membership
The user SHALL be able to pin any collection from the context menu of its row in the sidebar's
collections section and of its entry among the inspector's collections, and unpin it from the
same menus and from the pinned chip's own context menu. Whether a collection is pinned SHALL be
a property of the collection, the same for every image, and SHALL NOT change any image, the
collection's name, or its members.

The inspector SHALL show every pinned collection as a chip at the top of its collections
section, in both of its placements — the selection panel gains a collections section for them,
after its tag area — in collection name order, whether or not the described image is in it
(owner, 2026-09-24: the chips first sat in the pinned strip beside the pinned tags; a collection
chip among tag chips read as one more tag, and the section that lists the image's collections is
where a click that changes them is looked for). A pinned collection's chip SHALL be told apart
from a pinned tag's at a glance: it SHALL carry the mark the tiles use for collection
membership rather than a pin mark, and its text SHALL NOT be drawn in any tag category's
colour. It SHALL show the collection's current name.

Each chip SHALL show whether the described image is in the collection, and one activation
SHALL add the image to it when it is not and take it out when it is, without recording the
image as changed. While the panel describes a selection, each chip SHALL show whether all,
some or none of the selected images are in the collection; activating it SHALL add every
selected image unless all are in, in which case it SHALL take every one out, asking first when
more than one image would be written. A pinned collection SHALL NOT appear in the sidebar's tag
list. Deleting a pinned collection SHALL remove its chip.

#### Scenario: Pin from the sidebar
- **WHEN** the user opens the context menu of `Cute` in the collections section and chooses Pin
- **THEN** `Cute` appears as a chip with the collection mark at the top of the inspector's collections section, and the tag list shows no entry for it

#### Scenario: Pin from the inspector
- **WHEN** the panel describes an image in `Cute` and the user opens the context menu of `Cute` among its collections and chooses Pin
- **THEN** `Cute` appears as a chip at the top of the collections section, shown as containing the image, and its badge stays among the image's collections below

#### Scenario: One click in, one click out
- **WHEN** `Cute` is pinned, the panel describes an image not in it, and the user activates the chip, then activates it again
- **THEN** the image is in `Cute` after the first activation and not after the second, its last-changed time did not move, and nothing else about it changed

#### Scenario: Over a selection
- **WHEN** `Cute` is pinned, twelve images are selected of which four are in it, and the user activates the chip
- **THEN** the chip showed some of them in it, the app asks, naming twelve and `Cute`, and on confirmation all twelve are in `Cute`

#### Scenario: All of them
- **WHEN** `Cute` is pinned, twelve images are selected and all are in it, and the user activates the chip and confirms
- **THEN** none of the twelve is in `Cute`, and each is still in its other collections

#### Scenario: Told apart from a tag
- **WHEN** the tag `cute` and the collection `Cute` are both pinned
- **THEN** the tag's chip sits in the tag area's pinned strip with a pin mark in its category colour, and the collection's sits in the collections section with the collection mark in plain text

#### Scenario: Unpin from the chip
- **WHEN** the user opens the chip's context menu and chooses Unpin
- **THEN** the chip is gone and every image is in exactly the collections it was in

#### Scenario: A pinned collection is renamed
- **WHEN** `Cute` is pinned and renamed `Kawaii`
- **THEN** the chip reads `Kawaii` and is still pinned

#### Scenario: A pinned collection is deleted
- **WHEN** `Cute` is pinned and the user deletes it
- **THEN** its chip is gone, and the chip row is absent if no collection is pinned

### Requirement: A collection survives a rebuild by its id
An image's describing file SHALL carry the ids of the collections it is in; the library-level
file SHALL carry every collection's id, name, times and whether it is pinned. A rebuild SHALL
restore the collections, their pins included, from the library-level file and the memberships
from the describing files. A membership naming a collection the library-level file does not
list SHALL come back under a collection named by that id, unpinned, so it can be renamed rather
than lost. Renaming, pinning or unpinning a collection SHALL rewrite the library-level file and
SHALL NOT rewrite any describing file.

#### Scenario: Rename touches one file
- **WHEN** `Favorites` holding 25,000 images is renamed
- **THEN** the library-level file changes and no image's describing file does

#### Scenario: Pin touches one file
- **WHEN** `Favorites` holding 25,000 images is pinned
- **THEN** the library-level file says so and no image's describing file changes

#### Scenario: Rebuilt
- **WHEN** a library with three collections and their memberships is rebuilt
- **THEN** the three collections come back with their names and the same images in each

#### Scenario: Rebuilt pinned
- **WHEN** a library whose collection `Cute` is pinned is rebuilt
- **THEN** `Cute` comes back pinned, and its chip is at the top of the inspector's collections section

#### Scenario: Rebuilt with no library-level file
- **WHEN** the library-level file is missing and an image's describing file names a collection id
- **THEN** the image comes back in a collection named by that id, not pinned
