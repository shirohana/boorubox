## MODIFIED Requirements

### Requirement: The frame has fixed regions
While a library is open the app SHALL present four regions — a navigation sidebar, a toolbar,
a main content region, and an inspector panel — and SHALL keep the sidebar and the toolbar in
place across every screen reached from the navigation. The sidebar SHALL name the open library
and offer the actions that change which library is open.

The sidebar SHALL collapse to an icon rail and expand again on request, and the window SHALL
be movable by dragging a surface that is not a control on every screen. A control that only
collapses or expands a region SHALL NOT present itself as a way to resize it: no region of the
frame is resizable by dragging, so no edge of one SHALL show a resize affordance.

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
