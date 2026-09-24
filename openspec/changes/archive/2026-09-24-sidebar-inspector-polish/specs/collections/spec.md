## MODIFIED Requirements

### Requirement: The sidebar lists the collections with counts
Beside the results the app SHALL list every collection in the library with the number of
images of the current result in it, in name order; the collections the search names SHALL be
marked where they are and SHALL NOT move to the front. Each entry SHALL filter the search by
that collection on activation and take the term out again when it is already there, SHALL
offer including it in the search and excluding it from the search as two separate controls
in the same form the tag list uses, and SHALL show which of the two the current search does.
Excluding a collection the search includes SHALL replace the inclusion, and the reverse;
neither action SHALL disturb the rest of the query. Each entry SHALL offer rename and delete
on its context menu; deleting SHALL ask first, naming the collection and how many images are in
it. The section SHALL offer creating a collection.

Each row SHALL be drawn exactly as a row of the tag list is — the same text size, the same
row height, the same spacing between rows — so the two lists read as one column (owner,
2026-09-24: the collection rows were taller and looser than the tags above them). The list
SHALL have no frame of its own around it.

The section SHALL fold away and unfold on request, and whether it is folded SHALL be kept
with the app's other preferences, so the choice survives a restart. Unfolded, its list SHALL
occupy a height the user can change by dragging the section's top edge — the edge it shares
with the tag list, dragged up for more room and down for less — kept for the session, and
SHALL scroll inside that height: a library with many collections SHALL NOT push the tag list
or the controls below out of their room. The handle SHALL be on the top edge and not the
bottom, because the section's bottom is pinned against the sections below it and a handle
there cannot be dragged past them (owner, 2026-09-24).

#### Scenario: Counts follow the search
- **WHEN** `Favorites` holds 300 images and the search is `cat`, matching 40 of them
- **THEN** `Favorites` is listed with 40

#### Scenario: Filter from the list
- **WHEN** the user activates `Favorites` in the list
- **THEN** the search reads `collection:favorites`, and activating it again removes the term

#### Scenario: Exclude from the list
- **WHEN** the search reads `cat` and the user excludes `Uncategorized` from the list
- **THEN** the search reads `cat -collection:uncategorized`, the row is marked as excluded, and images in `Uncategorized` leave the result

#### Scenario: Include replaces exclude
- **WHEN** the search reads `-collection:queue` and the user includes `Queue` from the list
- **THEN** the search reads `collection:queue` and nothing else changed

#### Scenario: An active collection stays in place
- **WHEN** the list reads `Art`, `Favorites`, `Queue` and the user activates `Queue`
- **THEN** `Queue` is marked active and the list still reads `Art`, `Favorites`, `Queue`

#### Scenario: Rename from the row
- **WHEN** the user opens the context menu of `Queue` and chooses rename
- **THEN** a dialog offers the new name, and no other control for it is drawn on the row

#### Scenario: Delete from the list
- **WHEN** the user chooses delete on `Queue`, holding 12 images
- **THEN** the app asks, naming `Queue` and 12, and only on confirmation deletes it

#### Scenario: Many collections
- **WHEN** the library has thirty collections
- **THEN** the section shows as many as fit the height it has, scrolls to the rest, and the tag list keeps its height

#### Scenario: Dragging the top edge
- **WHEN** the user drags the section's top edge upward by 100 pixels
- **THEN** the list is 100 pixels taller, the tag list above it 100 pixels shorter, and dragging it back down restores both

#### Scenario: Rows match the tag list
- **WHEN** the tag list and the collection list are both on screen
- **THEN** a collection row is the same height as a tag row, in the same text size, with the same spacing to the next row

#### Scenario: Folded across a restart
- **WHEN** the user folds the section and restarts the app
- **THEN** the section is folded
