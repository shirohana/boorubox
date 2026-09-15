## MODIFIED Requirements

### Requirement: One inspector panel, two placements
The app SHALL show the facts of the current image in an inspector panel that is the same in
both of its placements: beside the main content region, and inside the full-size viewer. The
panel SHALL show at least the title, source, page address, image address, pixel dimensions,
file size, file type, capture time and tags of that image, and SHALL show them read-only.

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

