## ADDED Requirements

### Requirement: A tag can carry a note
Any tag SHALL be able to carry a note: one short plain text, kept with the tag across every
image that carries it. The tag's context menu — in the sidebar's tag list, on the inspector's
tag badges and on the pinned chips — SHALL offer "Edit note…", which opens a dialog titled
"Tag note" naming the tag in its category's colour, with the note's current text prefilled for
editing, and Cancel and Save. Saving SHALL store the text as written, trimmed at both ends;
saving an empty or blank text SHALL remove the note. Saving the text unchanged SHALL write
nothing. Cancel SHALL write nothing. A note SHALL NOT change any image's tag set. Opened from
inside the viewer, the dialog SHALL appear above the viewer. A refusal SHALL be shown in the
dialog, which stays open with the text kept.

#### Scenario: Write a note
- **WHEN** the user opens the context menu of `sky` in the sidebar, chooses "Edit note…", types `whole background only` and saves
- **THEN** `sky` carries the note `whole background only` wherever it is shown, and no image's tags changed

#### Scenario: Clear a note
- **WHEN** `sky` carries a note and the user empties the dialog's text and saves
- **THEN** `sky` carries no note

#### Scenario: Unchanged text
- **WHEN** the user opens the dialog on `sky` and saves without changing the text
- **THEN** nothing is written

#### Scenario: From the viewer
- **WHEN** the viewer is open and the user chooses "Edit note…" on a tag in its inspector
- **THEN** the dialog appears above the viewer and can be typed into and saved

### Requirement: A tag's note is shown where the tag is read
Wherever a tag with a note is drawn as a sidebar row, an inspector badge or a pinned chip, a
small muted glyph SHALL follow its name, and hovering the glyph SHALL show the note, wrapped,
after a short delay. A tag without a note SHALL show no glyph. Tile footers SHALL NOT show a
note or a glyph.

#### Scenario: A noted tag in the sidebar
- **WHEN** `sky` carries a note and the results carry `sky` and `cloud`, which has none
- **THEN** the sidebar's `sky` row shows the glyph after its name, `cloud` shows none, and hovering the glyph shows the note

#### Scenario: A long note wraps
- **WHEN** a tag's note is three sentences long
- **THEN** hovering the glyph shows all of it wrapped across lines at a fixed width, not cut off on one line

#### Scenario: Pinned chip and badge
- **WHEN** a pinned tag carries a note and the inspector shows an image that carries it
- **THEN** both its pinned chip and its badge show the glyph, and hovering either shows the note

## MODIFIED Requirements

### Requirement: The vocabulary outlives its carriers
A tag that is not general, is pinned, or carries a note SHALL stay in the library's vocabulary
when no image carries it any more, keeping its category, its pin and its note, and SHALL be
offered by the editor's suggestions. A general, unpinned tag with no note SHALL leave the
vocabulary with its last carrier, as it does today; removing the note of a general, unpinned
tag no image carries SHALL remove the tag from the vocabulary.

#### Scenario: An artist's last image is trashed and deleted
- **WHEN** `kantoku` is an artist tag on one image and that image is deleted for good
- **THEN** `kantoku` is still an artist tag and is still suggested when `kan` is typed

#### Scenario: A general tag's last image
- **WHEN** `bench` is general and unpinned on one image and that tag is removed from it
- **THEN** `bench` is no longer suggested

#### Scenario: A noted general tag's last image
- **WHEN** `bench` is general, unpinned and carries a note, on one image, and that tag is removed from it
- **THEN** `bench` is still suggested when `ben` is typed, and still carries its note

#### Scenario: The note of an uncarried tag is removed
- **WHEN** `bench` is general, unpinned, carried by no image and carries a note, and the note is removed
- **THEN** `bench` is no longer suggested
