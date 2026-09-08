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

#### Scenario: Moving between screens
- **WHEN** the user moves from the library to the settings screen
- **THEN** the sidebar and its library name stay in place and the main content region changes

#### Scenario: No library open
- **WHEN** no library is open
- **THEN** the frame is not shown and the start screen occupies the window

#### Scenario: The edge of the sidebar
- **WHEN** the pointer rests on the edge between the sidebar and the main content region
- **THEN** it is shown as something to click, not as something to drag, and clicking it collapses or expands the sidebar

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

#### Scenario: Typing a query
- **WHEN** the focus is in a search field and the user types `i` or presses an arrow key
- **THEN** the character is typed or the caret moves, and no shortcut fires

#### Scenario: A control that owns the key
- **WHEN** the focus is on a control whose own behaviour is bound to an arrow key and that key is pressed
- **THEN** only that control acts, and the binding of the region it sits in does not fire as well

#### Scenario: Leaving the search field
- **WHEN** the focus is in a search field and Escape is pressed
- **THEN** the field loses focus and the grid shortcuts are live again

#### Scenario: Finding the shortcuts
- **WHEN** the user opens the settings screen
- **THEN** every binding in this table is listed with its keys and what it does

#### Scenario: Arrow at the edge
- **WHEN** the focused card is in the first row and the up arrow is pressed
- **THEN** the focus stays on that card and the grid does not scroll away
