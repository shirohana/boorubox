## ADDED Requirements

### Requirement: The facts form opens where the facts are
The edit action for the title and the two addresses ("One inspector panel, two placements")
SHALL be offered twice: in the title row, and in a heading row directly above the facts it
edits. Both SHALL be the same action, open the same form, move the keyboard to its first field
so the form is scrolled into view, and be hidden while the form is open. The facts sit low in
the panel, below a tag list that is often long, so the title row's action is a screen away from
what it opens (owner, 2026-09-28). Changing the image the panel describes SHALL close the form
and discard its text, as it closes the tag editor; a save of the same image SHALL NOT reopen
or reset it.

#### Scenario: From the facts
- **WHEN** the panel describes one image and the user presses the edit action in the heading row above the facts
- **THEN** the title and the two addresses become a form, the Title field has the keyboard, and neither edit action is shown

#### Scenario: From the title row still
- **WHEN** the user presses the edit action in the title row
- **THEN** the same form opens with the same field focused

#### Scenario: Another image while editing the facts
- **WHEN** the facts form is open for image A with a typed title and the user focuses image B
- **THEN** the panel shows image B's facts read-only, and image A's title is unchanged
