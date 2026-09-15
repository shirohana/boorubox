## MODIFIED Requirements

### Requirement: The frame has fixed regions
While a library is open the app SHALL present four regions — a navigation sidebar, a toolbar,
a main content region, and an inspector panel — and SHALL keep the sidebar and the toolbar in
place across every screen reached from the navigation. The sidebar SHALL name the open library
and offer the actions that change which library is open.

On a screen with a result set the sidebar SHALL hold, from the top: the search fields, the
rating controls, the collections, the tag list, the order and grouping controls, then the
library's note, the navigation and the library footer. The tag list SHALL take whatever height the others leave and
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
- **THEN** the sidebar reads, top to bottom: search, rating, collections, tags, order and grouping, note, navigation, library

#### Scenario: A long tag list
- **WHEN** the current result carries more tags than the sidebar has room for
- **THEN** the tag list scrolls within its own space, and the search fields, the rating controls, the order and grouping controls and the note stay where they are

#### Scenario: Searching from a collapsed sidebar
- **WHEN** the sidebar is collapsed to its rail and the user presses the tag-search shortcut
- **THEN** the sidebar expands and the tag query field has the focus

#### Scenario: Room for the selection
- **WHEN** twenty images are selected at a window 1000 pixels wide
- **THEN** every selection action is visible in the toolbar without scrolling it
