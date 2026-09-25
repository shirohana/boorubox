## MODIFIED Requirements

### Requirement: A category is changed where the tag is shown
The context menu of a tag — in the sidebar's tag list and on the inspector's tag badges —
SHALL offer the five categories with the current one marked, each with its category's icon,
and choosing one SHALL change the tag's category everywhere it is shown, without touching any
image's tag set. The same menu, wherever it opens, SHALL offer opening the tag on Danbooru in
the system browser: the tag's wiki page for a tag of any category but artist, and the artist
search by name for an artist tag, since an artist's tag often differs from the name searched
(owner, 2026-09-25).

#### Scenario: From the sidebar
- **WHEN** the user opens the context menu of `azur_lane` in the sidebar and chooses Copyright
- **THEN** `azur_lane` is drawn in the copyright colour in the sidebar, in every inspector badge and in the suggestion list

#### Scenario: Open the wiki
- **WHEN** the user opens the context menu of the general tag `solo` and chooses "Open Danbooru wiki"
- **THEN** the system browser opens `https://danbooru.donmai.us/wiki_pages/solo`

#### Scenario: Search an artist
- **WHEN** the user opens the context menu of the artist tag `metaljelly` and chooses "Search artist on Danbooru"
- **THEN** the system browser opens Danbooru's artist search with `metaljelly` as the name to match

### Requirement: A tag can be pinned for one-click editing
The user SHALL be able to pin any tag from the tag's context menu in the sidebar and in the
inspector, and unpin it from the same menus and from the pinned chip's own context menu. A
pinned tag SHALL belong to exactly one group; groups SHALL be numbered from 1, SHALL have no
names, and a newly pinned tag SHALL land in group 1 (owner, 2026-09-24: bands to separate
drawing style, objects, clothes and a character's look, without naming them). The inspector
SHALL show every pinned tag, one row per group in group order, a thin separator between rows
and no label, and within a row in the app's one category order — artist, copyright,
character, general, meta — alphabetical within a category, as the sidebar lists tags, at the
top of its tag area in both of its placements, whether or not the described image carries it.
When there are two or more groups, each row SHALL carry a small dim `#n` hint at its right
edge, `n` being the number the row's "Move to #n" items use, so a tag can be moved to a group
without counting rows (owner, 2026-09-25, at eight groups); with one group no hint SHALL be
drawn, since there is nothing to move to.

A pinned tag's context menu SHALL offer, wherever the tag's menu opens: "New group above",
which inserts a group before the tag's own and moves the tag into it; "New group below",
which inserts one after and moves the tag into it — both only while the tag shares its group
with another, since for a tag alone in its group either would leave the groups exactly as
they were, and a control that does nothing is not shown (`app-frame`, owner 2026-09-24); and,
when there is more than one group, "Move to #n" for every group other than the tag's own. No group SHALL ever be empty: when a
move or an unpin empties a group, the groups after it SHALL close up and be renumbered. The
group number SHALL be kept with the tag in the library's own describing file, so a rebuild
restores the groups; a describing file written before groups existed SHALL restore every
pinned tag into group 1.

The strip SHALL hold tags only — a pinned collection's chip sits in the collections section
(`collections`) — and SHALL show nothing, no row and no heading, while no tag is pinned. A
pinned chip SHALL look like a control and not like one of the image's tags: a pill with a
pin mark, where the image's tags are plain text. Each chip SHALL show whether the described
image carries the tag, and one activation SHALL add the tag to the image when it does not and
remove it when it does, recording the image as changed. While the panel describes a
selection, each chip SHALL show whether all, some or none of the selected images carry the
tag; activating it SHALL add the tag to every selected image unless all carry it, in which
case it SHALL remove it from every one, asking first when more than one image would be
written. The selection panel's strip SHALL show the same groups and offer the same menu.

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
- **THEN** the rows read `1girl smile`, `sketch` — the new group took position 2, and the group `sketch` left, now empty, is gone

#### Scenario: Move to a group
- **WHEN** group 1 holds `1girl smile` and group 2 holds `sketch`, and the user chooses "Move to #2" on `smile`
- **THEN** the rows read `1girl` and `sketch smile`, and the menu on `smile` now offers only "Move to #1"

#### Scenario: An emptied group closes up
- **WHEN** three groups hold `1girl`, `smile`, `sketch` in turn and the user unpins `smile`
- **THEN** the strip shows two rows, `1girl` and `sketch`, and `sketch`'s menu offers "Move to #1"

#### Scenario: Only one group
- **WHEN** `1girl` and `smile` are pinned, both in group 1
- **THEN** either tag's menu offers "New group above" and "New group below" and no "Move to" item

#### Scenario: Alone in its group
- **WHEN** group 1 holds `1girl smile` and group 2 holds `sketch` alone
- **THEN** `sketch`'s menu offers "Move to #1" and neither "New group above" nor "New group below"

#### Scenario: Groups survive a rebuild
- **WHEN** `sketch` is pinned in group 2 and the library is rebuilt from its folder
- **THEN** `sketch` is in group 2 afterwards

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
- **WHEN** no tag is pinned and the collection `Cute` is
- **THEN** the strip is absent, and the collections section holds the one chip for `Cute`

#### Scenario: Nothing pinned
- **WHEN** no tag is pinned
- **THEN** the inspector draws no pinned row and no heading for one

#### Scenario: The hint appears with the second group
- **WHEN** `1girl` and `sketch` are pinned in one group and the user chooses "New group below" on `sketch`
- **THEN** the strip shows two rows, the first ending in `#1` and the second in `#2`, and `sketch`'s menu offers "Move to #1"

#### Scenario: One group has no hint
- **WHEN** every pinned tag is in group 1
- **THEN** the row shows no `#1`
