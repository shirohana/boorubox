## MODIFIED Requirements

### Requirement: Metadata comes from the file
An imported image SHALL record the filename as title, the moment it was imported as its
capture time, the file's own modification time as a fact of its own, its pixel dimensions,
size and MIME type, and `source=local`. Both times SHALL be visible in the inspector, named
apart, and the file's modification time SHALL be absent rather than invented when the
filesystem cannot report one. An image that never came from a file SHALL have no modification
time.

**Why the capture time changed**: the file's modification time was the capture time because
the import this app was designed around is a one-off bulk migration of an old folder, where
the mtimes are the only record of the order the pictures arrived in and using them preserves
it. It stopped being the right column once the app became the one in daily use: an image
imported today gets whatever time its file happens to carry, so it lands somewhere in the
middle of the default "newest capture first" grid instead of at the front, and the user has to
search for the thing they just added. A migration that loses its order costs one sort; an added
image landing at random costs a hunt every time. The order the mtimes carry is not lost — the
modification time is still read and still stored, now under its own name, where an ordering
that wants it can read it.

#### Scenario: Metadata visible
- **WHEN** a file `cat.png` last modified 2026-01-02 is imported on 2026-09-08
- **THEN** its card shows the title `cat.png`, its capture time is 2026-09-08, and the inspector shows the file's modification time as 2026-01-02

#### Scenario: A freshly imported file is at the front
- **WHEN** a file whose modification time is two years old is imported into a library sorted newest capture first
- **THEN** it is among the first tiles of the grid, not among the images captured two years ago

#### Scenario: No modification time to be had
- **WHEN** the filesystem cannot report a modification time for an imported file
- **THEN** the image is still imported with the import time as its capture time, and its modification time is shown as absent

#### Scenario: An image that is not a file import
- **WHEN** an image captured by the browser extension is inspected
- **THEN** its capture time is the time of the capture and it carries no file modification time

## ADDED Requirements

### Requirement: The drop target answers external drags only
The app SHALL show its "drop to import" target only for a drag that carries files from outside
the app, and SHALL NOT show it while the user drags something inside the app's own window.
Dragging an image in the library SHALL NOT be offered as a way to import it.

#### Scenario: Dragging a thumbnail
- **WHEN** the user presses on a thumbnail in the grid and drags it across the window
- **THEN** no import overlay appears and nothing is imported

#### Scenario: Dragging a file in from outside
- **WHEN** the user drags an image file from the file manager over the window
- **THEN** the import overlay appears and releasing it imports the file
