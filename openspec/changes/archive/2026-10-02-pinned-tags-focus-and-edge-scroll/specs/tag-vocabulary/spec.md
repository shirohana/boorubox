## MODIFIED Requirements

### Requirement: A tag can be pinned for one-click editing
The user SHALL be able to pin any tag from the tag's context menu in the sidebar and in the
inspector, and unpin it from the same menus and from the pinned chip's own context menu. A
pinned tag SHALL belong to exactly one group. Groups SHALL be ordered from 1 and MAY carry a
name; a newly pinned tag SHALL land in group 1 (owner, 2026-09-24: bands to separate drawing
style, objects, clothes and a character's look; names, an order the user sets and moves of
many tags added 2026-10-01, when the strip had become a daily tool — the 2026-09-24 "bands,
not names" was right at three groups and stopped being right at many). The inspector SHALL
show every pinned tag, one row per group in group order, a thin separator between rows, and
within a row in the app's one category order — artist, copyright, character, general, meta —
alphabetical within a category, as the sidebar lists tags, at the top of its tag area in both
of its placements, whether or not the described image carries it.

Each row SHALL carry, at its right edge in small dim text, the group's name when it has one
and otherwise `#n`, `n` being the number the group's "Move to" items use; a single unnamed
group SHALL carry no label, since there is nothing to move to and nothing to name it by. The
label SHALL fold the row: a folded row shows the label and the number of tags it hides and no
chips, and a second activation unfolds it. Whether a group is folded SHALL be kept with the
group in the library (owner, 2026-10-01: a group is library data with an identity; the app's
settings file has none for it).

A pinned tag's context menu SHALL offer, wherever the tag's menu opens: "New group above",
which inserts a group before the tag's own and moves the tag into it; "New group below",
which inserts one after and moves the tag into it — both only while the tag shares its group
with another, since for a tag alone in its group either would leave the groups exactly as
they were, and a control that does nothing is not shown (`app-frame`, owner 2026-09-24); when
there is more than one group, "Move to <label>" for every group other than the tag's own,
the label being the group's name or `#n`; and "Manage pinned tags…", which opens the
panel below. The strip's group label SHALL offer "Manage pinned tags…" on its own context
menu, so a group with no tags is reachable (owner, 2026-10-02). No unnamed group SHALL ever be empty: when a move or an unpin empties an
unnamed group, the groups after it SHALL close up and be renumbered. A named group SHALL
survive being emptied, drawn as its label alone, because its name is the user's work (the
rule a general tag with a note already follows). The groups — each tag's group, and every
group's name, order and fold — SHALL be kept in the library's own describing file, so a
rebuild restores them; a describing file written before groups had names SHALL restore the
groups the tags name, unnamed and unfolded; one written before groups existed SHALL restore
every pinned tag into group 1.

The "Pinned tags" panel SHALL list every group in order, each with its name editable in
place and Move up / Move down buttons in its header, and its tags each with a checkbox, a
drag handle and an Unpin control carrying the pin mark the chip menu's Unpin uses; a tag
row's context menu SHALL offer Unpin, and opening it SHALL NOT change the row's checkbox.
Dragging a tag's handle onto another group SHALL move it there in one write, and when the
dragged tag is checked SHALL move every checked tag with it; the group under the pointer SHALL
be marked while the drag lasts, and a drag held near the top or bottom edge of the groups
SHALL scroll them so a group taller than the visible area can be reached (owner,
2026-10-02). The panel SHALL NOT focus a name field on its own: opening the dialog and every
write leave the focus where it was or on the groups, never in a field (owner, 2026-10-02). While any tag is checked the panel SHALL offer moving the
checked tags together to any other group or to a new group named on the spot, as one write
that either moves them all or none. Each group's header SHALL offer pinning a tag into that
group: a field with the editor's completion, where Enter pins the typed tag and keeps the
field for the next and Escape closes it; a name that is not a tag of the library SHALL be
refused with the reason and the text kept. The panel SHALL offer creating a named group, and
deleting a group only while the group is empty. A blank name SHALL be refused for a new group
and SHALL clear an existing group's name. The panel's bar — the move control, "New group…" and, in the dialog, Done — SHALL stay in
view while the groups scroll, and the dialog's header SHALL stay put, so a move is always one
reach away with many tags (owner, 2026-10-02). The panel SHALL be reachable as a dialog from the
menus above, which from inside the viewer SHALL appear above the viewer, and as the Settings
page "Pinned tags" (owner, 2026-10-02: the Stamps precedent — a casual door and a page).
Groups SHALL NOT be dragged; a drag inside this app is pointer-driven, since the window's
file dropper takes every HTML5 drag.

The strip SHALL hold tags only — a pinned collection's chip sits in the collections section
(`collections`) — and SHALL show nothing, no row and no heading, while no tag is pinned and no
group is named. A pinned chip SHALL look like a control and not like one of the image's tags:
a pill with a pin mark, where the image's tags are plain text. Each chip SHALL show whether
the described image carries the tag, and one activation SHALL add the tag to the image when
it does not and remove it when it does, recording the image as changed. While the panel
describes a selection, each chip SHALL show whether all, some or none of the selected images
carry the tag; activating it SHALL add the tag to every selected image unless all carry it,
in which case it SHALL remove it from every one, asking first when more than one image would
be written. The selection panel's strip SHALL show the same groups, the same labels and folds,
and offer the same menu.

#### Scenario: Pin from the sidebar
- **WHEN** the user opens the context menu of `tagme` in the sidebar and chooses Pin
- **THEN** `tagme` appears as a chip with a pin mark in group 1 at the top of the inspector's tag area

#### Scenario: Pinned tags in category order
- **WHEN** `tagme` (meta), `kantoku` (artist), `1girl` (general) and `azur_lane` (copyright) are pinned in one group
- **THEN** the row reads `kantoku`, `azur_lane`, `1girl`, `tagme`, each in its category's colour

#### Scenario: A new group below
- **WHEN** `1girl`, `smile` and `sketch` are pinned in group 1 and the user chooses "New group below" on `sketch`
- **THEN** the strip shows two rows, `1girl smile` and `sketch`, with a separator between them

#### Scenario: A new group above
- **WHEN** group 1 holds `1girl smile` and group 2 holds `sketch`, and the user chooses "New group above" on `sketch`
- **THEN** the rows read `1girl smile`, `sketch` — the new group took position 2, and the unnamed group `sketch` left, now empty, is gone

#### Scenario: Move to a group
- **WHEN** group 1 holds `1girl smile` and group 2 holds `sketch`, and the user chooses "Move to #2" on `smile`
- **THEN** the rows read `1girl` and `sketch smile`, and the menu on `smile` now offers only "Move to #1"

#### Scenario: An emptied group closes up
- **WHEN** three unnamed groups hold `1girl`, `smile`, `sketch` in turn and the user unpins `smile`
- **THEN** the strip shows two rows, `1girl` and `sketch`, and `sketch`'s menu offers "Move to #1"

#### Scenario: An emptied named group stays
- **WHEN** group 2 is named `Clothes` and holds `dress` alone, and the user unpins `dress`
- **THEN** the strip still shows a row labelled `Clothes` with no chips, and the panel offers deleting it

#### Scenario: Only one group
- **WHEN** `1girl` and `smile` are pinned, both in group 1
- **THEN** either tag's menu offers "New group above" and "New group below" and no "Move to" item

#### Scenario: Alone in its group
- **WHEN** group 1 holds `1girl smile` and group 2 holds `sketch` alone
- **THEN** `sketch`'s menu offers "Move to #1" and neither "New group above" nor "New group below"

#### Scenario: Groups survive a rebuild
- **WHEN** `sketch` is pinned in group 2, named `Style` and folded, and the library is rebuilt from its folder
- **THEN** `sketch` is in group 2 afterwards, the group is named `Style` and is folded

#### Scenario: A describing file from before names
- **WHEN** the library's describing file lists `sketch` in group 2 and carries no groups of its own, and the library is rebuilt from it
- **THEN** `sketch` is in group 2, which is unnamed and unfolded

#### Scenario: An old describing file
- **WHEN** the library's describing file was written before groups existed and lists `tagme` as pinned, and the library is rebuilt from it
- **THEN** `tagme` is pinned in group 1

#### Scenario: One click on, one click off
- **WHEN** `tagme` is pinned, the panel describes an image without it, and the user activates the chip, then activates it again
- **THEN** the image carries `tagme` after the first activation and not after the second, and its last-changed time moved each time

#### Scenario: Over a selection
- **WHEN** `tagme` is pinned, twelve images are selected of which four carry it, and the user activates the chip
- **THEN** the app asks, naming twelve, and on confirmation all twelve carry `tagme`

#### Scenario: All of them
- **WHEN** `tagme` is pinned, twelve images are selected and all carry it, and the user activates the chip and confirms
- **THEN** none of the twelve carries `tagme`

#### Scenario: Unpin from the chip
- **WHEN** the user opens the chip's context menu and chooses Unpin
- **THEN** the chip is gone and the tag is unchanged on every image

#### Scenario: Tags first, then collections
- **WHEN** the tags `tagme` and `wip` and the collection `Cute` are pinned
- **THEN** the strip reads `tagme`, `wip`, and `Cute`'s chip sits in the collections section, not in the strip

#### Scenario: Only a collection pinned
- **WHEN** no tag is pinned, no group is named, and the collection `Cute` is pinned
- **THEN** the strip is absent, and the collections section holds the one chip for `Cute`

#### Scenario: Nothing pinned
- **WHEN** no tag is pinned and no group is named
- **THEN** the inspector draws no pinned row and no heading for one

#### Scenario: The hint appears with the second group
- **WHEN** `1girl` and `sketch` are pinned in one unnamed group and the user chooses "New group below" on `sketch`
- **THEN** the strip shows two rows, the first ending in `#1` and the second in `#2`, and `sketch`'s menu offers "Move to #1"

#### Scenario: One group has no hint
- **WHEN** every pinned tag is in group 1 and the group has no name
- **THEN** the row shows no label and cannot be folded

#### Scenario: A name replaces the number
- **WHEN** group 2 is named `Clothes` in the dialog
- **THEN** its row ends in `Clothes` instead of `#2`, and a tag in group 1 offers "Move to Clothes"

#### Scenario: Fold a group
- **WHEN** group 2 is named `Clothes` and holds four tags, and the user activates its label
- **THEN** the row shows `Clothes · 4` and no chips, in both of the inspector's placements, and activating the label again shows the chips

#### Scenario: Reorder groups
- **WHEN** the groups are `Style`, `Clothes`, `Pose` and the user presses Move up on `Pose` twice in the panel
- **THEN** the strip's rows read `Pose`, `Style`, `Clothes`, every tag still in its own group

#### Scenario: Move many tags at once
- **WHEN** the user checks `dress`, `hat` and `boots` in group 1 and chooses "Move selected to… New group…" in the panel, naming it `Clothes`
- **THEN** a new last group named `Clothes` holds the three, group 1 no longer does, and the checkboxes are clear

#### Scenario: A move of many is all or none
- **WHEN** one of the checked names is no longer a tag when the move is written
- **THEN** no tag moved, and the panel shows the refusal

#### Scenario: Delete an empty group
- **WHEN** group 3, named `Old`, holds no tag and the user chooses "Delete group" on it
- **THEN** the group is gone and the groups after it close up; a group holding a tag offers no delete

#### Scenario: The label's menu
- **WHEN** group 2 is named `Clothes` and holds no tag, and the user opens the context menu on its label in the strip
- **THEN** it offers "Manage pinned tags…", which opens the panel

#### Scenario: A right-click does not tick
- **WHEN** the user right-clicks the `dress` row in the panel
- **THEN** the row's menu opens, offering Unpin, and `dress`'s checkbox is as it was

#### Scenario: Drag a tag to another group
- **WHEN** the user drags `dress`'s handle from group 1 onto the group named `Clothes` and lets go
- **THEN** `dress` is in `Clothes`, and nothing else moved

#### Scenario: Drag carries the checked tags
- **WHEN** `dress`, `hat` and `boots` are checked and the user drags `hat`'s handle onto `Clothes`
- **THEN** all three are in `Clothes` and the checkboxes are clear

#### Scenario: Unpin from the panel
- **WHEN** the user activates the Unpin control at the end of `dress`'s row
- **THEN** `dress` is no longer pinned and its chip is gone from the strip

#### Scenario: Pin into a group from the panel
- **WHEN** the user presses the "+" in `Clothes`'s header, types `skirt`, which is a tag of the library, and presses Enter
- **THEN** `skirt` is pinned in `Clothes`, the field is empty and still open, and typing `zzz` and Enter is refused naming the tag

#### Scenario: The Settings page
- **WHEN** the user opens Settings → Pinned tags
- **THEN** the same panel is shown, with every group and control the dialog offers

#### Scenario: Many tags, the bar stays
- **WHEN** the panel holds more groups than the dialog can show and the user ticks a tag in the first group
- **THEN** "Move 1 selected to…" is in view at the dialog's foot without scrolling, and the groups scroll under the header and above the bar

#### Scenario: Focus after a write
- **WHEN** the user presses Unpin on a row, or Move up on a group
- **THEN** no name field gains the focus; it stays on the pressed control when it still exists, and otherwise rests on the groups

#### Scenario: Opening the dialog
- **WHEN** the user opens the Pinned tags dialog
- **THEN** no field is focused, and Escape closes the dialog

#### Scenario: Drag at the edge
- **WHEN** the groups are taller than the dialog shows and the user drags a tag to the bottom edge and holds it there
- **THEN** the groups scroll until the target group is under the pointer, which is then marked, and letting go moves the tag there

