## MODIFIED Requirements

### Requirement: A tag can be pinned for one-click editing
The user SHALL be able to pin any tag from the tag's context menu in the sidebar and in the
inspector, and unpin it from the same menus and from the pinned chip's own context menu. The
inspector SHALL show every pinned tag, in the app's one category order — artist, copyright,
character, general, meta — alphabetical within a category, as the sidebar lists tags, at the
top of its tag area in both of its placements, whether or not the described image carries it
(owner, 2026-09-24: the strip was alphabetical alone, and reads better grouped like the left
panel). The same strip SHALL hold the pinned
collections' chips (`collections`), after every pinned tag, and SHALL show nothing — no row and
no heading — while neither a tag nor a collection is pinned. A pinned chip SHALL look like a
control and not like one of the image's
tags: a pill with a pin mark, where the image's tags are plain text. Each chip SHALL show
whether the described image carries the tag, and one activation SHALL add the tag to the
image when it does not and remove it when it does, recording the image as changed. While the
panel describes a selection, each chip SHALL show whether all, some or none of the selected
images carry the tag; activating it SHALL add the tag to every selected image unless all
carry it, in which case it SHALL remove it from every one, asking first when more than one
image would be written.

#### Scenario: Pin from the sidebar
- **WHEN** the user opens the context menu of `tagme` in the sidebar and chooses Pin
- **THEN** `tagme` appears as a chip with a pin mark at the top of the inspector's tag area

#### Scenario: Pinned tags in category order
- **WHEN** `tagme` (meta), `kantoku` (artist), `1girl` (general) and `azur_lane` (copyright) are pinned
- **THEN** the strip reads `kantoku`, `azur_lane`, `1girl`, `tagme`, each in its category's colour, and the pinned collections follow

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
- **THEN** the strip reads `tagme`, `wip`, `Cute`, in that order

#### Scenario: Only a collection pinned
- **WHEN** no tag is pinned and the collection `Cute` is
- **THEN** the strip is drawn, holding the one chip for `Cute`

#### Scenario: Nothing pinned
- **WHEN** no tag and no collection is pinned
- **THEN** the inspector draws no pinned row and no heading for one
