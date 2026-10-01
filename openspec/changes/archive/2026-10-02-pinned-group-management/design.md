## Context

`tags.pinned_group INTEGER NOT NULL DEFAULT 0` (schema v11) is the whole of a group: `0` is
unpinned, `n ≥ 1` is group `n`, groups are dense from 1 and `tags::compact_groups` renumbers
after every write so no group is empty. `tags::place_pinned(library, name, PinTarget)` is the
one write (`Unpin`, `Group(n)`, `NewGroupAt(n)`), answering `Vec<TagEntry>`; so do
`set_category`, `set_note` and `vocabulary`, through four commands the webview's
`vocabulary.svelte.ts` replaces `entries` from. The store derives `pinnedGroups: string[][]`,
`pinned` (its flattening), `groupOf`, `groupCount`; `TagVocabularyMenuItems.svelte` draws
Pin/Unpin, "New group above/below" and "Move to #n"; `Inspector.svelte`'s `pinnedTagRows`
snippet draws one `<ul>` per group with a `#n` hint past one group, shared by its two strips.
`library.json` carries `tags: Option<Vec<TagEntry>>` with `pinnedGroup` per tag;
`recover::insert_vocabulary` restores it with "lowest non-zero group wins";
`artists::rename`'s merge carries the loser's group onto an unpinned winner. Schema was v17
when unit R landed (`stamp-order` took v17); this change's migration is v18.
`stamp-order` design D5 leaves `common/reorder.ts` (`moveItem`, `reorderable`) and `ReorderHandle.svelte` for this change to reuse.

The 2026-09-24 reading — "bands, not names; the number exists only in the menu" — was right
when the owner had three groups and wanted the alphabetical run broken up: a name would have
been a label on something self-evident, and moving a group was a need nobody had. It stopped
being right when the strip became a daily tool with many groups (owner, 2026-10-01): at that
size a band is found by what it is for, which is a name; a working set is arranged, which is
an order; and a reorganisation moves ten tags, which the one-tag menu makes ten round trips.
`menu-polish` D1 (2026-09-25) already bent the rule once, adding `#n` at eight groups. The
"minimal appearance" half of the decision stands: the strip gains a name where the number
was, and nothing else.

## Goals / Non-Goals

Goals: the proposal's. Non-goals: the proposal's.

## Decisions

**D1. Schema v18 (`MIGRATIONS.len()` when unit R landed): a `pinned_groups` table keyed by
position.** `CREATE TABLE pinned_groups (position INTEGER PRIMARY KEY, name TEXT NOT NULL
DEFAULT '', collapsed INTEGER NOT NULL DEFAULT 0 CHECK (collapsed IN (0, 1)))`, then
`INSERT INTO pinned_groups (position) SELECT DISTINCT pinned_group FROM tags WHERE
pinned_group > 0`. `tags.pinned_group` keeps its meaning — the group's position — so every
query, `PinTarget`, the wire row, `library.json`'s tag entries and the old-file reader stay
as they are. Keyed by position and not by a surrogate id because the position is already the
identity every existing write, menu item and describing file uses; an id would be a second
name for the same group and a join on every read of the strip. The invariant, held by every
write in this module: positions are dense from 1; every non-zero `tags.pinned_group` has a
row; a row with an empty name and no tag does not exist.

**D2. `compact_groups` closes up unnamed empty groups and keeps named ones.** It collects
the live positions — the union of `DISTINCT pinned_group` over tags and `position` over rows
whose `name != ''` — in ascending order, deletes every other row, and renumbers both tables
to `1..n` in one pass (ascending, so a move down never collides). A named group survives
empty because a name is work the user did and would have to redo, the same reason a general
tag with a note is never pruned (`tag-notes`); an unnamed empty group has nothing to lose and
closing it is what the strip has always done. A `Group(n)` or `NewGroupAt(n)` write inserts
the row it needs when none exists (`INSERT OR IGNORE`), and `NewGroupAt` shifts
`pinned_groups` rows at or past `n` up by one before it shifts the tags.

**D3. One write for a move of many: `place_pinned_many(library, names, target)`;
`place_pinned` is its one-name call.** Same transaction, same compaction, same answer. The
refusal for a name that is not a tag covers the whole list — nothing written — so a dialog
that moves ten tags never lands five. Command `move_pinned_tags(names, target)` beside
`set_tag_pinned_group`, which stays for the menu.

**D4. Three more writes, each the position's: `rename_pinned_group(position, name)`,
`set_pinned_group_collapsed(position, collapsed)`, `move_pinned_group(from, to)`; one
create, `create_pinned_group(name)`; one delete, `delete_pinned_group(position)`.** `rename`
trims and stores; a blank name stores `''` and then compacts — an unnamed group that is also
empty closes, which the dialog shows as the group disappearing, honestly. `create` refuses a
blank name (an unnamed empty group would close before it was seen; `app-frame`'s "no control
appears before it does something" read from the other side) and appends at `n + 1`.
`move_pinned_group` renumbers tags and rows together: the moved group's members and row are
parked at `0`/a temporary, the groups between `from` and `to` shift by one, the parked ones
land at `to`. `delete` refuses a group that still has tags — the dialog offers it on an empty
group only — and compacts. Each refuses a position with no row. Every one answers the
vocabulary (D5) and rewrites `library.json`, as `place_pinned` does.

**D5. The vocabulary answer widens to `Vocabulary { tags: Vec<TagEntry>, groups:
Vec<PinnedGroup> }`, `PinnedGroup { name: String, collapsed: bool }` in position order.**
Every command that answers `Vec<TagEntry>` today (`tag_vocabulary`, `set_tag_category`,
`set_tag_pinned_group`, `set_tag_note`) answers `Vocabulary`, as do D3's and D4's. One answer
and not a second `pinned_groups_list` command: a compaction after any pinned write can delete
or renumber a group, so a store that read tags from one call and groups from another would
show a name on the wrong row for the length of a round trip. `groups[i]` describes position
`i + 1`; the position is the index, not a field, for the same reason `Stamp` carries no
position (`stamp-order` D2). The TS `Vocabulary` type lives in `packages/shared` beside
`TagEntry`.

**D6. `library.json` gains `pinnedGroups: Option<Vec<PinnedGroup>>` in position order;
`LIBRARY_VERSION` stays 1.** `None` is a file written before this change: a rebuild derives
the rows from the tags' groups as the migration does, every group unnamed and unfolded.
`Some` is restored verbatim (`INSERT` per index), then `insert_vocabulary` runs, then
`compact_groups` closes anything the two disagree on — an old reader's required fields are
untouched (`#[serde(default)]`, the `collections` reasoning in `sidecar.rs`).

**D7. The store: `groups: PinnedGroup[]` beside `entries`, replaced together from every
answer; `pinnedGroups` becomes `{ name, collapsed, tags }[]`.** `pinned`, `groupOf`,
`groupCount`, `isPinned` keep their shapes. `labelOf(position)` answers the name or `#n`
for the menu and the strip, so the two cannot disagree on what a group is called. `place`
stays; `placeMany(names, target)`, `renameGroup`, `setGroupCollapsed`, `moveGroup`,
`createGroup`, `deleteGroup` each call their command and replace both arrays, with
`setNote`'s "answer whether it landed" shape where a dialog has to stay open on a refusal
(rename, create).

**D8. The strip: the `#n` hint becomes the group's label and the fold toggle; nothing else
moves.** `pinnedTagRows` draws the label (`vocabulary.labelOf`) where `#n` is today, as a
`button` with `aria-expanded`, when the group is named or there is more than one group; a
single unnamed group has no label and no fold, exactly today's rule. A folded row shows the
label and `· <count>` in the same dim text and no chips; its `<ul>` is not rendered. The
toggle writes through `setGroupCollapsed`. The label is `text-[10px] text-muted-foreground`,
truncated at `max-w-[8rem]` with the full name in `title`. Layout: the label is the last item
of the row's own wrap container (`ml-auto order-last self-end`), so it takes the end of the
chips' last line when there is room and a line of its own only when there is not; the
absolute `pr-6` reservation goes, since a name is wider than `#n` and a fixed reservation
would either clip it or waste a column. The hand check judges it at one group, three and
eight.

**D9. "Move to #n" reads the label.** `TagVocabularyMenuItems` maps the other groups through
`vocabulary.labelOf`, so the item reads "Move to Clothes" or "Move to #3". "New group
above/below" stay. One item is added for a pinned tag, after the moves: "Manage pinned
groups…", which raises `onmanagegroups` — a required prop, the `oneditnote` shape, so
typecheck names every mount — and the host mounts one `PinnedGroupsDialog` (D10) the way it
mounts `TagNoteDialog`. The item is offered whenever a group exists, on pinned and unpinned
tags alike: a named empty group has no chip to open a menu from, so a menu on any tag is the
only way to reach the dialog that renames or deletes it.

**D10. `PinnedGroupsDialog.svelte` in `components/tags`, the `CollectionNameDialog` shell at
`sm:max-w-lg`.** One section per group in order: a `ReorderHandle` and the group's name as an
`Input` (saved on blur and Enter through `renameGroup`; a refusal shows under it and keeps the
text), a context menu with "Move up" / "Move down" (disabled at the ends) and, when the group
is empty, "Delete group"; under it the group's tags as `Checkbox` rows in the strip's order
and colours. A footer that appears while any tag is checked: "Move <n> selected to…" as a
`Select` listing every group but, when all checked tags share one group, that group, plus
"New group…"; and always "New group…", which asks for a name in a `CollectionNameDialog`-shaped
prompt and calls `createGroup` — or, from the Select, `createGroup` then `placeMany` into the
new last position, two writes because a name and a move are two commands and a combined one
would be a third door onto the same rows. Checked names are component state keyed by tag,
cleared after a move lands. Reordering groups uses `reorderable` on the sections list with
`moveGroup(from + 1, to + 1)`. Opened from inside the viewer the dialog portals to the
inspector's `portalTo`, as `TagNoteDialog` does.

**D11. Tests that pin the invariants.** Rust: the migration seeds rows from the tags; a
named empty group survives compaction and an unnamed one closes; `move_pinned_group` keeps
every tag with its group; `place_pinned_many` is all-or-nothing; `delete_pinned_group`
refuses a non-empty group; `library.json` round-trips names and folds; an old file rebuilds
unnamed groups from the tags; the `every_tags_column…` test stays green. Webview:
`pinnedGroups` carries names and folds and `labelOf` falls back to `#n`; the strip hides a
folded group's chips and shows its count (`Inspector.svelte.test.ts`); the menu reads a name.

## Risks / Trade-offs

- [Two tables that must agree] → D1's invariant is held in one module (`tags.rs`), every
  write ends in `compact_groups`, and a debug assertion in tests (`groups_are_consistent`)
  checks it after each.
- [A fold is library data and syncs between machines] → the owner's call (2026-10-01): the
  group is library data with an identity, and the settings file has none for it.
- [The label's layout in a wrapping row] → D8 names the two layouts; the hand check decides.
- [Archive order] → `stamp-order` archives first (its migration is v17; both touch
  `library-recovery`'s describing-file requirement); re-copy this change's
  `library-recovery` delta from the merged spec before archiving if the text drifted.

## Migration

Schema v18 (D1), automatic. `library.json` gains `pinnedGroups` on the next write; old files
read (D6).
