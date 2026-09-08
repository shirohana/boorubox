## ADDED Requirements

### Requirement: The rating control is operated from the keyboard
Every one of the five choices SHALL be reachable from the keyboard, both by stepping through
them with Tab and Shift-Tab and by moving between them with the left and right arrow keys, and
the control SHALL show which one the keyboard is on. While the focus is inside the control,
those keys SHALL do nothing but move within it: no image SHALL change, and nothing behind the
control SHALL move.

#### Scenario: Stepping through the choices
- **WHEN** the focus is on one of the five choices and Tab, then Shift-Tab, is pressed
- **THEN** the focus moves to the next and back to the previous choice of the same control

#### Scenario: Arrows inside the control
- **WHEN** the focus is on one of the five choices in the full-size viewer and the right arrow is pressed
- **THEN** the focus moves to the next choice and the image being viewed is unchanged

#### Scenario: Choosing from the keyboard
- **WHEN** a choice is focused and confirmed with Enter or Space
- **THEN** the image takes that rating, exactly as clicking the choice would set it
