## MODIFIED Requirements

### Requirement: The frame has fixed regions
While a library is open the app SHALL present four regions — a navigation sidebar, a toolbar,
a main content region, and an inspector panel — and SHALL keep the sidebar and the toolbar in
place across every screen reached from the navigation. The sidebar SHALL name the open library
and offer the actions that change which library is open.

On a screen with a result set the sidebar SHALL hold, from the top: the tag search field, the
rating controls, the tag list, the collections, the order and grouping controls, then the
library's note, the navigation and the library footer. The tag list SHALL take whatever height
the others leave and SHALL scroll on its own; the collections SHALL take the height the user
gives them by dragging their top edge and scroll inside it; no other part of the sidebar SHALL
scroll out of view. On a screen with no result set the note SHALL sit directly above the
navigation at the bottom of the sidebar, not at its top (owner, 2026-09-24). The tag search
field SHALL be a single line at the sidebar's text size and SHALL show the query syntax's
examples while it is empty. The order and grouping controls SHALL be drawn at the sidebar's
compact control size, no taller than a row of the tag list plus its padding.

The toolbar SHALL hold, left to right: the sidebar toggle; the free-text search field over
titles and addresses; the selection's actions, centred in the remaining width, while there is
a selection; then, held at the right edge whether or not there is a selection, the screen's
own action, the thumbnail size and the inspector toggle. A selection SHALL NOT move the
controls at either end.

The sidebar SHALL collapse to an icon rail and expand again on request — from the toolbar's
toggle and from the keyboard — and the window SHALL be movable by dragging a surface that is
not a control on every screen. The edge between the sidebar and the main content region SHALL
be plain: not a control that collapses the sidebar (owner, 2026-09-24: the click fired by
accident), and not a resize affordance — no region of the frame is resizable by dragging, so
no edge of one SHALL show one. The shortcut that focuses the tag search SHALL expand a
collapsed sidebar first, so the field it reaches is on screen.

Whether the inspector panel is shown SHALL be kept for the session: hiding it on the library
screen, visiting another screen and coming back SHALL find it still hidden.

On a platform whose window has no native full-screen control the toolbar SHALL end with one
that enters and leaves full screen and shows which state the window is in. On a platform whose
window draws its own full-screen control the toolbar SHALL NOT draw a second.

#### Scenario: Moving between screens
- **WHEN** the user moves from the library to the settings screen
- **THEN** the sidebar and its library name stay in place and the main content region changes

#### Scenario: No library open
- **WHEN** no library is open
- **THEN** the frame is not shown and the start screen occupies the window

#### Scenario: The edge of the sidebar
- **WHEN** the pointer rests on the edge between the sidebar and the main content region and clicks it
- **THEN** the pointer is the plain arrow, and nothing happens: the sidebar neither collapses nor resizes

#### Scenario: Full screen from the toolbar on Windows
- **WHEN** the app runs on Windows and the user presses the full-screen control, then presses it again
- **THEN** the window fills the screen with no title bar, the control shows the window is full screen, and the second press restores the window

#### Scenario: No second control on macOS
- **WHEN** the app runs on macOS
- **THEN** the toolbar shows no full-screen control, and the window's own green button is how full screen is entered

#### Scenario: The order of the sidebar
- **WHEN** the library screen is shown at a window height that fits everything
- **THEN** the sidebar reads, top to bottom: tag search, rating, tags, collections, order and grouping, note, navigation, library

#### Scenario: The note on the settings screen
- **WHEN** the settings screen is shown
- **THEN** the sidebar reads, top to bottom: empty space, note, navigation, library — the note directly above the navigation

#### Scenario: A long tag list
- **WHEN** the current result carries more tags than the sidebar has room for
- **THEN** the tag list scrolls within its own space, and the search field, the rating controls, the collections, the order and grouping controls and the note stay where they are

#### Scenario: The toolbar with nothing selected
- **WHEN** the library screen is shown and no image is selected
- **THEN** the sidebar toggle and the text search field sit at the left edge, and the import action, the thumbnail size and the inspector toggle sit at the right edge, with empty space between

#### Scenario: The toolbar with a selection
- **WHEN** twenty images are selected
- **THEN** the selection's actions appear in the middle of the toolbar, and the controls at both edges are where they were before the selection

#### Scenario: Searching from a collapsed sidebar
- **WHEN** the sidebar is collapsed to its rail and the user presses the tag-search shortcut
- **THEN** the sidebar expands and the tag query field has the focus

#### Scenario: Room for the selection
- **WHEN** twenty images are selected at a window 1000 pixels wide
- **THEN** every selection action is visible in the toolbar without scrolling it

#### Scenario: The inspector stays hidden
- **WHEN** the user hides the inspector on the library screen, opens the import screen and returns to the library
- **THEN** the inspector is still hidden

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

For one image the panel SHALL read, top to bottom, in the order the owner uses them (2026-09-24):
the title row; the rating; the tags; the collections; the upload action, when a booru is
configured; the facts (source, addresses, dimensions, size, type, times, id) and where the image
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

### Requirement: One keyboard map
The app SHALL bind the following keys, and SHALL NOT act on any of them while the focus is in
a text field, nor while the focus is in a control that acts on that key itself:

| Where | Key | Action |
| --- | --- | --- |
| Anywhere in the frame | `/` | move focus to the tag search field |
| Anywhere | `Cmd/Ctrl B` | collapse or expand the sidebar |
| Anywhere | `Cmd/Ctrl =` `-` `0` | zoom the whole app in, out, back to normal |
| Grid | `←` `→` `↑` `↓` | move the focused card, stopping at the edges |
| Grid | `Home` `End` | focus the first / last card |
| Grid | `Enter` `Space` | open the focused image in the full-size viewer |
| Grid | `i` | show or hide the inspector |
| Grid | `e` | edit the focused image's tags in the inspector, showing it first if hidden |
| Viewer | `←` `→` | previous / next image in the current result order |
| Viewer | `↑` `↓` | the image one grid row before / after the one shown |
| Viewer | `i` | enter or leave inspect mode |
| Viewer | `e` | edit the shown image's tags in the inspector, entering inspect mode first |
| Viewer | `Esc` `Space` | close the viewer, focusing the image it showed last |
| Anywhere | `F11` | enter or leave full screen |

`e` is the one key given to a single image's tag editing (owner's keyboard rule, 2026-09-23:
keys go to the most frequent day-to-day actions). It SHALL act only while the panel would
describe one image: with more than one image selected the panel describes the selection, and
the key SHALL do nothing.

The map SHALL be listed, read-only, on the settings screen.

A completed action in the inspector panel — a rating chosen, tags saved, a tag removed, a tag
or account acted on as a search term — SHALL hand the keyboard back to the region the panel
sits beside: the grid's current card when the panel is beside the grid, the viewer when the
panel is inside it. A failed save SHALL keep the focus in the editor so the text can be fixed.

A click anywhere on the library screen that leaves no control focused — on the inspector's
text or empty space, on the account rail, on the toolbar's band, on the sidebar between its
controls — SHALL likewise leave the grid's current card focused, so the grid's keys keep
working after it. A click that lands on a control that takes the focus (a text field, a menu, a
dialog) SHALL leave the focus there.

#### Scenario: Typing a query
- **WHEN** the focus is in a search field and the user types `i` or presses an arrow key
- **THEN** the character is typed or the caret moves, and no shortcut fires

#### Scenario: A control that owns the key
- **WHEN** the focus is on a control whose own behaviour is bound to an arrow key and that key is pressed
- **THEN** only that control acts, and the binding of the region it sits in does not fire as well

#### Scenario: Leaving the search field
- **WHEN** the focus is in a search field and Escape is pressed
- **THEN** the field loses focus and the grid shortcuts are live again

#### Scenario: Editing tags from the grid
- **WHEN** a card is current, the inspector is hidden, and the user presses `e`
- **THEN** the inspector opens beside the grid, its tag editor is open for that image, and the caret is at the end of its text

#### Scenario: Editing tags from the viewer
- **WHEN** the viewer shows an image in gallery mode and the user presses `e`
- **THEN** the viewer enters inspect mode with the tag editor open for that image and the caret at the end of its text

#### Scenario: `e` over a selection
- **WHEN** three images are selected and the user presses `e`
- **THEN** nothing changes: the panel keeps describing the selection

#### Scenario: `e` while typing
- **WHEN** the focus is in the tag search field and the user types `e`
- **THEN** the letter is typed and no editor opens

#### Scenario: Rating from the panel beside the grid
- **WHEN** a card is current, the user clicks a rating in the panel beside the grid, and then presses the right arrow
- **THEN** the rating is written and the grid's focus moves to the next card

#### Scenario: Saving tags from the panel beside the grid
- **WHEN** the user confirms the tag editor beside the grid with the keyboard and the save succeeds, then presses Space
- **THEN** the current image opens in the full-size viewer

#### Scenario: A save that fails
- **WHEN** the user confirms the tag editor and the save is refused
- **THEN** the focus stays in the editor and the reason is shown under it

#### Scenario: Finding the shortcuts
- **WHEN** the user opens the settings screen
- **THEN** every binding in this table is listed with its keys and what it does

#### Scenario: Arrow at the edge
- **WHEN** the focused card is in the first row and the up arrow is pressed
- **THEN** the focus stays on that card and the grid does not scroll away

#### Scenario: F11
- **WHEN** the library is on screen and F11 is pressed, then pressed again
- **THEN** the window fills the screen, and then returns to its previous size and place

#### Scenario: Clicking the panel's text
- **WHEN** a card is current and the user clicks the title text, the page address text or empty space in the panel beside the grid, then presses the right arrow
- **THEN** the grid's focus moves to the next card

#### Scenario: Clicking into the editor
- **WHEN** the user clicks into the panel's tag editor and presses the right arrow
- **THEN** the caret moves and the grid's focus does not
