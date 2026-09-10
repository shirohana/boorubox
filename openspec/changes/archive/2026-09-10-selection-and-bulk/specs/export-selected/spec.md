## Purpose

Getting a handful of images back out of the library without copying the whole folder: the
selected originals, unaltered, in one zip file the user names and can hand to someone else.

## ADDED Requirements

### Requirement: The selection is written to a zip the user names
The app SHALL offer, while a selection exists, writing the selected images to a single zip file
at a path the user chooses through the system's save dialog. The archive SHALL contain the
original file of each selected image, byte for byte as the library holds it. Cancelling the
dialog SHALL write nothing.

Each entry SHALL be named by that image's identifier, followed by its tags (in the image's stored
order, space-separated, `/`, `\` and control characters replaced so a tag cannot open a
directory) and its file extension — the identifier first so two selected images can never collide
and the name stays stable, the tags after so an export is readable without opening the app. A name
longer than the filesystem's limit SHALL drop whole tags from the end rather than cut one short.
An image with no tags SHALL be named by its identifier and extension alone. Each entry's
last-modified time SHALL be the image's capture time, so exports carry a time axis to sort by; a
capture time the archive format cannot represent SHALL fall back to the format's own default
rather than failing the export.

Originally each entry was named by identifier and extension alone with no last-modified time set:
the identifier already ruled out a collision, which was reason enough on its own. Reversed once
the owner used the feature and reported a UUID-only name has no order and every entry read the
same unhelpful date — an export needs to be skimmed and sorted outside the app, not just unpacked
inside it.

#### Scenario: Exporting a selection
- **WHEN** twenty images, several of them tagged, are selected and the user picks a destination
- **THEN** a zip is written there holding those twenty files, each identical to the file in the
  library; a tagged image's entry is named by its identifier, then its tags, then its extension,
  an untagged one by its identifier and extension alone, and every entry's last-modified time is
  that image's capture time

#### Scenario: Cancelled
- **WHEN** the user dismisses the save dialog without choosing a path
- **THEN** no file is written and the selection is unchanged

#### Scenario: Beyond what is loaded
- **WHEN** the selection covers images no thumbnail has been drawn for
- **THEN** those images are in the archive too

### Requirement: A long export reports progress and finishes
While an export runs the app SHALL show how far it has got, SHALL keep the window responsive,
and SHALL NOT block a capture arriving from the extension while it writes.

#### Scenario: A large export
- **WHEN** the user exports several hundred images
- **THEN** progress is shown as the archive is written and the app stays usable

#### Scenario: A capture during an export
- **WHEN** an image is delivered by the extension while an export is running
- **THEN** the capture is stored as usual and the export completes

### Requirement: A missing file is reported, not fatal
An image whose file is no longer under the library folder SHALL be left out of the archive and
named in the result; the export SHALL still write every other selected image and SHALL say how
many were written.

#### Scenario: One file gone
- **WHEN** one of the selected images has been deleted from the library folder outside the app
- **THEN** the zip holds the rest, and the app reports how many were written and which image was missing

#### Scenario: Nothing left to write
- **WHEN** every selected image's file is missing
- **THEN** the app reports that nothing was written rather than leaving an empty archive unexplained
