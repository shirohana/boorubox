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

The sidebar SHALL collapse to an icon rail and expand again on request, and the window SHALL
be movable by dragging a surface that is not a control on every screen.

#### Scenario: Moving between screens
- **WHEN** the user moves from the library to the settings screen
- **THEN** the sidebar and its library name stay in place and the main content region changes

#### Scenario: No library open
- **WHEN** no library is open
- **THEN** the frame is not shown and the start screen occupies the window

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
file size, file type, capture time and tags of that image, and SHALL show them read-only.

#### Scenario: Beside the grid
- **WHEN** a card in the grid is focused and the inspector is open
- **THEN** the panel shows that image's facts

#### Scenario: Inside the viewer
- **WHEN** the full-size viewer is in inspect mode
- **THEN** the same panel with the same fields is shown for the image being viewed

#### Scenario: Nothing focused
- **WHEN** no image is focused and the inspector is open
- **THEN** the panel says that no image is selected rather than showing empty fields

### Requirement: One keyboard map
The app SHALL bind the following keys, and SHALL NOT act on any of them while the focus is in
a text field:

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
| Viewer | `i` | enter or leave inspect mode |
| Viewer | `Esc` `Space` | close the viewer, focusing the image it showed last |

The map SHALL be listed, read-only, on the settings screen.

#### Scenario: Typing a query
- **WHEN** the focus is in a search field and the user types `i` or presses an arrow key
- **THEN** the character is typed or the caret moves, and no shortcut fires

#### Scenario: Leaving the search field
- **WHEN** the focus is in a search field and Escape is pressed
- **THEN** the field loses focus and the grid shortcuts are live again

#### Scenario: Finding the shortcuts
- **WHEN** the user opens the settings screen
- **THEN** every binding in this table is listed with its keys and what it does

#### Scenario: Arrow at the edge
- **WHEN** the focused card is in the first row and the up arrow is pressed
- **THEN** the focus stays on that card and the grid does not scroll away

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
