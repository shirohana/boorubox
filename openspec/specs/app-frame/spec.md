# app-frame Specification

## Purpose
The frame every screen of the app is drawn in: a fixed set of regions, one inspector panel
that appears in two places, one keyboard map, and one theme setting. Later features fill named
slots in this frame instead of adding screens of their own.

## Requirements

### Requirement: The frame has fixed regions
While a library is open the app SHALL present four regions — a navigation sidebar, a toolbar,
a main content region, and an inspector panel — and SHALL keep the sidebar and the toolbar in
place across every screen reached from the navigation. The sidebar SHALL name the open library
and offer the actions that change which library is open.

On a screen with a result set the sidebar SHALL hold, from the top: the search fields, the
rating controls, the tag list, the order and grouping controls, then the library's note, the
navigation and the library footer. The tag list SHALL take whatever height the others leave and
SHALL scroll on its own; no other part of the sidebar SHALL scroll out of view. The search
fields SHALL use the sidebar's text size. The toolbar SHALL hold the sidebar toggle, the
thumbnail size, the inspector toggle and the row that changes with the screen: the selection's
actions while there is a selection, otherwise the screen's own action.

The sidebar SHALL collapse to an icon rail and expand again on request, and the window SHALL
be movable by dragging a surface that is not a control on every screen. A control that only
collapses or expands a region SHALL NOT present itself as a way to resize it: no region of the
frame is resizable by dragging, so no edge of one SHALL show a resize affordance. The shortcut
that focuses the tag search SHALL expand a collapsed sidebar first, so the field it reaches is
on screen.

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
- **WHEN** the pointer rests on the edge between the sidebar and the main content region
- **THEN** it is shown as something to click, not as something to drag, and clicking it collapses or expands the sidebar

#### Scenario: Full screen from the toolbar on Windows
- **WHEN** the app runs on Windows and the user presses the full-screen control, then presses it again
- **THEN** the window fills the screen with no title bar, the control shows the window is full screen, and the second press restores the window

#### Scenario: No second control on macOS
- **WHEN** the app runs on macOS
- **THEN** the toolbar shows no full-screen control, and the window's own green button is how full screen is entered

#### Scenario: The order of the sidebar
- **WHEN** the library screen is shown at a window height that fits everything
- **THEN** the sidebar reads, top to bottom: search, rating, tags, order and grouping, note, navigation, library

#### Scenario: A long tag list
- **WHEN** the current result carries more tags than the sidebar has room for
- **THEN** the tag list scrolls within its own space, and the search fields, the rating controls, the order and grouping controls and the note stay where they are

#### Scenario: Searching from a collapsed sidebar
- **WHEN** the sidebar is collapsed to its rail and the user presses the tag-search shortcut
- **THEN** the sidebar expands and the tag query field has the focus

#### Scenario: Room for the selection
- **WHEN** twenty images are selected at a window 1000 pixels wide
- **THEN** every selection action is visible in the toolbar without scrolling it

### Requirement: No control appears before it does something
The app SHALL NOT present a control, menu entry or navigation item that performs no action,
and SHALL NOT show a navigation item for a screen that does not exist. A region reserved for a
feature that is not built SHALL be absent, not empty.

#### Scenario: A feature that is not built
- **WHEN** a screen or action for a capability the app does not yet have would appear
- **THEN** nothing for it is rendered: no disabled control, no placeholder and no empty panel

#### Scenario: Read-only stands in for an editor
- **WHEN** a fact can be shown but not yet changed
- **THEN** it is shown as text with no editing affordance

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
| Viewer | `←` `→` | previous / next image in the current result order |
| Viewer | `↑` `↓` | the image one grid row before / after the one shown |
| Viewer | `i` | enter or leave inspect mode |
| Viewer | `Esc` `Space` | close the viewer, focusing the image it showed last |
| Anywhere | `F11` | enter or leave full screen |

The map SHALL be listed, read-only, on the settings screen.

A completed action in the inspector panel — a rating chosen, tags saved, a tag removed, a tag
or account acted on as a search term — SHALL hand the keyboard back to the region the panel
sits beside: the grid's current card when the panel is beside the grid, the viewer when the
panel is inside it. A failed save SHALL keep the focus in the editor so the text can be fixed.

#### Scenario: Typing a query
- **WHEN** the focus is in a search field and the user types `i` or presses an arrow key
- **THEN** the character is typed or the caret moves, and no shortcut fires

#### Scenario: A control that owns the key
- **WHEN** the focus is on a control whose own behaviour is bound to an arrow key and that key is pressed
- **THEN** only that control acts, and the binding of the region it sits in does not fire as well

#### Scenario: Leaving the search field
- **WHEN** the focus is in a search field and Escape is pressed
- **THEN** the field loses focus and the grid shortcuts are live again

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

### Requirement: Theme follows the system unless chosen
The app SHALL offer three appearance settings — follow the system, light, dark — SHALL persist
the choice outside the library folder, and SHALL apply it before the first frame is painted.
While set to follow the system it SHALL react to the operating system changing appearance
without a restart.

#### Scenario: First launch
- **WHEN** the app has never been given an appearance setting and the system is in dark mode
- **THEN** the app opens dark, with no flash of the light palette

#### Scenario: Chosen explicitly
- **WHEN** the user picks light while the system is dark
- **THEN** the app is light, and is still light after a restart

#### Scenario: System changes while running
- **WHEN** the setting is "follow the system" and the operating system switches to dark
- **THEN** the app switches to dark without a restart
