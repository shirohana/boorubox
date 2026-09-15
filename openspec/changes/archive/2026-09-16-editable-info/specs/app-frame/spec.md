## MODIFIED Requirements

### Requirement: One inspector panel, two placements
The app SHALL show the facts of the current image in an inspector panel that is the same in
both of its placements: beside the main content region, and inside the full-size viewer. The
panel SHALL show at least the title, source, page address, image address, pixel dimensions,
file size, file type, capture time and tags of that image. The title, the page address and the
image address SHALL be editable behind one edit action that turns those three rows into a form
with save and cancel; every other fact SHALL be read-only. Saving SHALL write all three, SHALL
record the image as changed at that moment, and SHALL show the new values everywhere the image
appears without re-running the search. An address that is not empty and is not an `http` or
`https` address SHALL be refused with a reason, leaving the form open with the typed text.
Cancelling SHALL discard the typed text. A completed save SHALL hand the keyboard back the way
the panel's other writes do.

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

