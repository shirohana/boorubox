## ADDED Requirements

### Requirement: Settings is a set of pages with a nav
The settings screen SHALL be a set of pages, with a nav listing them at the left of the main
content region and the chosen page beside it. The nav SHALL list, in this order: General,
Library, Artists, Rules, Stamps, Booru, Keyboard, About, and SHALL show which page is open.
Each page SHALL have its own address, and navigating to that address from anywhere in the app
SHALL show that page with the nav, without passing through another page first. A page's
content SHALL be as wide as the settings screen's single column was; the nav SHALL NOT narrow
the app's own sidebar. When the main content region is too narrow for the nav and the page
side by side, the nav SHALL sit above the page instead. The nav SHALL stay in view while a long
page scrolls.

Opening Settings from the app's navigation SHALL show the settings page last shown on this
machine, surviving a restart, or General when none has been shown or the remembered one no
longer exists. The app's navigation SHALL show Settings as the current screen on every
settings page.

What each page holds is said by the capability that owns the fact; General holds the settings
set once and then left alone — how the app looks and how it listens for captures — because a
page holding a single field is not worth a place in the nav (owner, 2026-09-28).

#### Scenario: The pages in order
- **WHEN** the user opens Settings
- **THEN** the nav lists General, Library, Artists, Rules, Stamps, Booru, Keyboard, About, and the page shown is marked in it

#### Scenario: First visit
- **WHEN** Settings is opened on a machine where no settings page has been shown before
- **THEN** the General page is shown

#### Scenario: The last page is remembered
- **WHEN** the user shows the Artists page, leaves Settings, quits and restarts the app, and opens Settings again
- **THEN** the Artists page is shown

#### Scenario: A link to a page
- **WHEN** another screen of the app navigates straight to the Artists page's address
- **THEN** the Artists page is shown with the nav, and no other settings page was shown on the way

#### Scenario: The sidebar on a settings page
- **WHEN** any settings page is shown
- **THEN** Settings is the entry marked current in the app's sidebar, and no other entry is

#### Scenario: A long page
- **WHEN** the Artists page lists more entries than fit and the user scrolls to the end
- **THEN** the nav is still on screen, and no other settings section is below the list

#### Scenario: A narrow window
- **WHEN** the main content region is narrower than the nav and a page need side by side
- **THEN** the nav sits above the page, and nothing scrolls sideways

## MODIFIED Requirements

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

The map SHALL be listed, read-only, on the Keyboard page of Settings.

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
- **WHEN** the user opens the Keyboard page of Settings
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

