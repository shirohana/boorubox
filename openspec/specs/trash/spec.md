# trash Specification

## Purpose
Deleting an image without losing it: a first act that takes the image out of the library and
keeps every byte of it, a place to see and search what has been deleted, a way back, and a
second, confirmed act that destroys it for good.

## Requirements

### Requirement: Deleting an image moves it to the trash
Every delete action the app offers on an image in the library SHALL move that image to the
trash rather than destroy anything. A trashed image SHALL disappear from browsing and from the
library's image counts, and its file, its thumbnail, its tags, its rating and every other fact
recorded about it SHALL be left untouched. The app SHALL record when the image was trashed.

Moving one image SHALL happen immediately, without confirmation. Moving two or more at once
SHALL ask for confirmation naming how many will move, whatever offered the action, and the
question SHALL NOT describe the move as permanent. Until it is answered, nothing SHALL be moved
and the selection SHALL be exactly what the user had made. An image the grid is showing SHALL
offer the move on the image itself, not only in its context menu.

#### Scenario: Deleting one image
- **WHEN** the user deletes an image from the grid
- **THEN** it leaves the grid, the library's image count drops by one, and the trash count rises by one

#### Scenario: Nothing on disk changes
- **WHEN** an image has been moved to the trash
- **THEN** its file is still in the library folder, byte for byte, and its thumbnail still exists

#### Scenario: It stops matching searches
- **WHEN** a search that matched a trashed image is run again while browsing the library
- **THEN** the image is not among the results

#### Scenario: Deleting a selection
- **WHEN** several images are selected and the user moves the selection to the trash
- **THEN** the app asks first, naming how many will move, and on confirming every selected image is trashed together, or none is and the app says the move did not apply

#### Scenario: Declining to move a selection
- **WHEN** the user dismisses that question
- **THEN** no image has moved and the same images are still selected

#### Scenario: Keyboard
- **WHEN** one image is focused in the grid, nothing is selected, and the user presses the delete key
- **THEN** that image moves to the trash straight away, with no question asked

#### Scenario: Keyboard over a selection
- **WHEN** images are selected and the user presses the delete key
- **THEN** the whole selection is what moves, and the app asks first, naming how many

#### Scenario: On the tile
- **WHEN** the pointer is over an image in the grid, or the image holds the keyboard focus
- **THEN** the tile itself offers moving that one image to the trash, and taking that offer moves it without asking and without changing which images are selected

#### Scenario: Typing
- **WHEN** the focus is in a text field and the user presses the delete key
- **THEN** the field's own editing behaviour applies and no image is trashed

### Requirement: The trash is a view of the library with a live count
The app SHALL offer the trash as a destination in its navigation, showing how many images are in
it, and that number SHALL be correct after every action that puts an image in or takes one out.
The trash SHALL be browsed with the same grid, search, full-size view and detail panel as the
library, showing only trashed images and never a library image.

#### Scenario: Reaching the trash
- **WHEN** the user opens the trash
- **THEN** every trashed image is shown and no image that is still in the library is

#### Scenario: The count follows the actions
- **WHEN** the user trashes three images and then restores one
- **THEN** the navigation shows the trash holding two more images than before

#### Scenario: Searching the trash
- **WHEN** a tag or free-text query is entered while browsing the trash
- **THEN** it narrows the trashed images by the same rules it narrows the library

#### Scenario: An empty trash
- **WHEN** the trash holds nothing
- **THEN** the trash view says so rather than showing a blank grid

#### Scenario: A trashed image whose file vanished
- **WHEN** the file behind a trashed image is removed outside the app
- **THEN** the trash still lists the image, shows it as missing, and the rest of the trash renders

### Requirement: Restoring returns an image to the library unchanged
The app SHALL offer restoring a trashed image, one at a time or as a selection, and a restored
image SHALL return to the library with the tags, rating, source and capture time it had before,
in the same place in the browsing order it would have held had it never been trashed.

#### Scenario: Restoring one
- **WHEN** the user restores a trashed image
- **THEN** it appears in the library again with its tags and rating intact, and leaves the trash

#### Scenario: Restoring asks nothing
- **WHEN** the user restores one trashed image or a selection of them
- **THEN** the restore happens straight away: it puts images back rather than taking them away

#### Scenario: On the tile in the trash
- **WHEN** the pointer is over a trashed image in the grid, or it holds the keyboard focus
- **THEN** the tile itself offers restoring that one image, and permanent deletion is not offered there

#### Scenario: Restoring a selection
- **WHEN** several trashed images are selected and restored
- **THEN** every one of them returns to the library together, or none does and the app says the restore did not apply

#### Scenario: Searchable again
- **WHEN** a restored image carried a tag
- **THEN** searching the library for that tag matches it again

### Requirement: Permanent deletion is deliberate and removes the file
The app SHALL offer permanent deletion only from the trash, SHALL ask for confirmation naming
how many images will be destroyed, and SHALL NOT bind it to any key. Permanent deletion SHALL
remove the image's record, its thumbnail and its file from the library folder, and SHALL NOT be
reversible. When a file cannot be removed, the record SHALL still be gone and the app SHALL name
the file that was left behind rather than report success or fail the whole action.

#### Scenario: Deleting forever
- **WHEN** the user confirms permanent deletion of a trashed image
- **THEN** the image is gone from the trash, its record and thumbnail are gone, and its file is no longer in the library folder

#### Scenario: Declining the confirmation
- **WHEN** the user dismisses the confirmation
- **THEN** nothing is deleted and the trash is unchanged

#### Scenario: No key destroys anything
- **WHEN** an image in the trash is focused and the user presses the delete key
- **THEN** nothing is destroyed

#### Scenario: A record whose file is already gone
- **WHEN** a trashed image whose file was removed outside the app is permanently deleted
- **THEN** the record is removed, the action succeeds, and nothing is reported as left behind

#### Scenario: A file that will not go
- **WHEN** the file cannot be removed because something else is holding it
- **THEN** the record is still removed, and the app names the file's location so the user can remove it themselves

### Requirement: Emptying the trash removes everything in it
The app SHALL offer emptying the trash in one act, behind a confirmation naming how many images
it will destroy, with the same effect as permanently deleting each of them. It SHALL touch no
image outside the trash, and SHALL do nothing when the trash is empty.

#### Scenario: Emptying
- **WHEN** the trash holds seven images and the user confirms emptying it
- **THEN** the confirmation named seven, the trash is empty, all seven records and files are gone, and the library is unchanged

#### Scenario: Nothing to empty
- **WHEN** the trash is empty
- **THEN** emptying it is not offered as something that would do anything

### Requirement: Nothing leaves the trash on its own
The app SHALL NOT remove anything from the trash except when the user restores it or destroys it.
There SHALL be no retention period, no age at which an image is purged, and no cleanup on
startup, on quit, or when a library is opened or closed.

#### Scenario: Time passes
- **WHEN** an image has been in the trash for months and the app has been started many times
- **THEN** it is still in the trash with its file intact

#### Scenario: Across a restart
- **WHEN** the app is quit with images in the trash and started again
- **THEN** the trash holds exactly the same images and the count matches

#### Scenario: Across a library switch
- **WHEN** another library is opened and the first one is opened again
- **THEN** the first library's trash is unchanged

### Requirement: A trashed image is still held, and is counted as such
The app SHALL report the number of images in the trash alongside the counts that describe the
library, and the counts used to verify a migration SHALL keep describing images that are not in
the trash, so a number compared against another source is compared like for like.

#### Scenario: Reconciling a migration
- **WHEN** the library holds 100 images of which 5 are in the trash
- **THEN** the total and per-source counts report 95, and the trashed count reports 5 beside them

#### Scenario: Nothing is hidden
- **WHEN** the user looks for how many images the app holds in total
- **THEN** the trashed count is on the same screen as the per-source counts and in the navigation

### Requirement: The trash opens on what was trashed last

The trash view SHALL order its images by the time they were trashed, newest first, unless the
user chooses another order, and SHALL offer "Trashed last" and "Trashed first" as orders in the
trash view only.

#### Scenario: A slip with Backspace is the first tile in the trash

- **WHEN** the user trashes an image by mistake and opens the trash
- **THEN** that image is the first tile shown, whatever its capture time

#### Scenario: The library's orders do not gain the trash's

- **WHEN** the user opens the sort control on the library
- **THEN** no order by trash time is offered
