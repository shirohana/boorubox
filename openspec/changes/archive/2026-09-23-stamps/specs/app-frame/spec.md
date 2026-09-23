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
| Viewer | `←` `→` | previous / next image in the current result order |
| Viewer | `↑` `↓` | the image one grid row before / after the one shown |
| Viewer | `i` | enter or leave inspect mode |
| Viewer | `Esc` `Space` | close the viewer, focusing the image it showed last |
| Anywhere | `F11` | enter or leave full screen |
| Library | `E` | enter or leave edit mode (`stamps`) |

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

#### Scenario: E toggles edit mode
- **WHEN** the library is on screen, no text field has the focus, and the user presses `E` twice
- **THEN** the stamp bar appears and then disappears
