## MODIFIED Requirements

### Requirement: The note lives in a panel that can be folded away
The note SHALL be shown beside the results, below the list of tags, in a panel the user can
collapse and expand. Whether it is collapsed SHALL be remembered across restarts, because a
panel large enough to write in is large enough to be in the way of the tag list. The panel
SHALL be absent while no library is open, since there is no note to show.

Expanded, the note's height SHALL be changed by dragging the panel's top edge — up for more
room, down for less — kept for the session; the text area SHALL offer no resize handle of
its own, since its bottom is pinned against the navigation below it and a handle there cannot
be dragged past it (owner, 2026-09-24).

#### Scenario: Collapsing
- **WHEN** the user collapses the note panel and restarts the app
- **THEN** the panel is still collapsed, and expanding it shows the note unchanged

#### Scenario: Dragging the top edge
- **WHEN** the user drags the note panel's top edge upward by 80 pixels
- **THEN** the note is 80 pixels taller and the sections above it have 80 pixels less

#### Scenario: No library open
- **WHEN** no library is open
- **THEN** no note panel is shown
