## MODIFIED Requirements

### Requirement: One inspector panel, two placements
The app SHALL show the facts of the current image in an inspector panel that is the same in
both of its placements: beside the main content region, and inside the full-size viewer. The
panel SHALL show at least the title, origin, page address, image address, pixel dimensions,
file size, file type, capture time and tags of that image. The rows SHALL be named for the
reader, in Danbooru's vocabulary: the capture origin is "Origin"; the page address is
"Source"; the image address is "Source file"; pixel dimensions, file size and file type are
one "Size" row reading `<size> .<extension> (<width>×<height>)`, the extension taken from
the stored file's name and omitted when it has none; the times are one "Date" row reading
how long ago the image was captured (`3 days ago`, `2 years ago`), and hovering it SHALL show
the absolute captured, imported and file-modified times. The title, the page address and the
image address SHALL be editable behind one edit action that turns those three rows into a form
with save and cancel; every other fact SHALL be read-only. Saving SHALL write all three, SHALL
record the image as changed at that moment, and SHALL show the new values everywhere the image
appears without re-running the search. An address that is not empty and is not an `http` or
`https` address SHALL be refused with a reason, leaving the form open with the typed text.
Cancelling SHALL discard the typed text. A completed save SHALL hand the keyboard back the way
the panel's other writes do.

For one image the panel SHALL read, top to bottom, in the order the owner uses them (2026-09-24):
the title row; the rating; the tags; the collections; the upload action, when a booru is
configured; the facts (origin, source, source file, size, date, id) and where the image
has been posted; the move-to-trash action, at the foot; and, only when no booru is configured,
the note that says so and where to add one, last of all. The rating sits first below the title
because its height never changes; the facts sit low because they are seldom read; the trash
action sits last because the tile's own trash control is the usual way there.

Beside the page address and beside the image address the panel SHALL offer an action that
opens that address in the system browser, present only when the address is an `http` or
`https` address. When the address cannot be handed to the browser the panel SHALL say so
where the action is, rather than doing nothing.

#### Scenario: Beside the grid
- **WHEN** a card in the grid is focused and the inspector is open
- **THEN** the panel shows that image's facts

#### Scenario: Inside the viewer
- **WHEN** the full-size viewer is in inspect mode
- **THEN** the same panel with the same fields is shown for the image being viewed

#### Scenario: The order of the panel
- **WHEN** the panel describes one image and a booru is configured
- **THEN** it reads, top to bottom: title, rating, tags, collections, upload, facts, move to trash

#### Scenario: No booru configured
- **WHEN** the panel describes one image and no booru is configured
- **THEN** no upload action is drawn between the collections and the facts, and the note that none is configured is the last thing in the panel, below move to trash

#### Scenario: Nothing focused
- **WHEN** no image is focused and the inspector is open
- **THEN** the panel says that no image is selected rather than showing empty fields

#### Scenario: Opening the page address
- **WHEN** the image has a page address and the user presses the open action beside it
- **THEN** the system browser opens that address and the app stays on the library screen

#### Scenario: An image with no page address
- **WHEN** the image came from a local file and has no page address
- **THEN** no open action is shown for it

#### Scenario: Giving a local import a title and a page
- **WHEN** the panel shows an image imported from a file, the user presses the edit action, types a title and `https://x.com/alice/status/9` as the page address, and saves
- **THEN** the rows show the title and the address, the image's last-changed time is now, the account entry shows `alice`, and a search for `account:alice` finds the image

#### Scenario: A bad address
- **WHEN** the user types `not a url` as the image address and saves
- **THEN** the save is refused, the reason is shown under the form, and the typed text is still there

#### Scenario: Cancel
- **WHEN** the user edits the title and cancels
- **THEN** the rows show the stored values and nothing was written

#### Scenario: Clearing an address
- **WHEN** the user empties the page address and saves
- **THEN** the image has no page address, its row shows none, and no account entry is shown

#### Scenario: The size row
- **WHEN** the panel shows a 1.3 MB JPEG of 1200 by 2200 pixels
- **THEN** its Size row reads `1.3 MB .jpg (1200×2200)`

#### Scenario: The date row
- **WHEN** the panel shows an image captured three days ago and the user hovers its Date row
- **THEN** the row reads `3 days ago` and the hover shows the captured, imported and file-modified times in full
