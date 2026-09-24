## Context

`tags.pinned INTEGER NOT NULL DEFAULT 0` (schema v7) is a flag; `TagEntry { name, category,
pinned: bool }` is the vocabulary row on the wire and in `library.json`'s `tags` key, restored
verbatim by `recover::insert_tags`. `tags::set_pinned(library, name, bool)` writes it and
answers the vocabulary; `commands::set_tag_pinned` exposes it. The webview's
`vocabulary.svelte.ts` derives `pinned: string[]` (category order then alphabetical, via
`groupByCategory` + `sortTags`) which `Inspector.svelte` draws twice through the `pinnedChip`
snippet (the single-image strip under Tags and the selection panel's strip) and flattens into
`selectionTagCounts`' names. `TagVocabularyMenuItems.svelte` carries Pin/Unpin and the
category radio; it is mounted on the sidebar row, the inspector badge and the pinned chip.
Schema is at v10 (`MIGRATIONS.len()`); `sidebar-inspector-polish` unit B moves the two
search items into the chip menu, above these items.

## Goals / Non-Goals

Goals: groups as the proposal states them. Non-goals: in the proposal.

## Decisions

**D1. The flag becomes the group number: schema v11 renames `pinned` to `pinned_group`.**
`ALTER TABLE tags RENAME COLUMN pinned TO pinned_group` — `0` = unpinned, `n ≥ 1` = group n.
Every pinned row today reads `1`, which is group 1: the owner's "at begin I only have one
default group #1", with no data patch. A rename rather than a second column, because one
column cannot disagree with itself about whether a tag is pinned. The v9 merge migration's
SQL names `pinned` and runs before v11 on any database that needs it, so it is untouched.
Every live SQL that reads `pinned` (`vocabulary`, `set_pinned`, the `LEFT JOIN` default rule
in `tags.rs`, `recover::insert_tags`) reads `pinned_group`. The real number is v11
(`MIGRATIONS.len()` when unit R landed) — the planned number held.

**D2. The wire row carries `pinnedGroup: number | null`.** `TagEntry { name, category,
pinned_group: Option<u32> }`, `None` for unpinned (stored as `0`), camelCase `pinnedGroup`.
The vocabulary's exceptions rule becomes `category != 'general' OR pinned_group > 0`. A
`library.json` written before this change carries `"pinned": true|false` and no
`pinnedGroup`: `TagEntry` deserialises with a custom `deserialize` that accepts either
key — `pinned: true` reads as `Some(1)`, `pinnedGroup: 0` and `false`/absent both as `None`
— so a rebuild from an old file restores every pin into group 1. `LIBRARY_VERSION` stays 1,
but not because "an old reader ignores `pinnedGroup`" — that line held only under the
assumption every field defaults when absent, the way `Collection.pinned` and `FactsEdit`'s
fields are declared. The build before this change never gave `TagEntry.pinned` that
`#[serde(default)]`; it derives `Deserialize` with `pinned: bool` required, so an old reader
finding no `pinned` key fails `read_library` outright (review finding on `aa38dda`), losing
rules, sites, note, collections and stamps along with the vocabulary — not the harmless skip
a defaulted field would take. `TagEntry`'s `Serialize` is hand-written, not derived, to keep
emitting `"pinned": <pinned_group.is_some()>` alongside `pinnedGroup`, so an old reader still
parses the file; a new reader reads `pinnedGroup` and the now-redundant `pinned` never
reaches it. `recover::insert_tags` writes the group; its merge
rule for a name that appears twice (`pinned` "only ever turns on") becomes "the lowest
non-zero group wins".

**D3. One Rust write: `place_pinned(library, name, target)`.** `target` is
`PinTarget::Unpin`, `PinTarget::Group(n)` (move into existing group n; `n` past the last
group means a new last group) or `PinTarget::NewGroupAt(n)` (insert an empty group at
position n — every tag in a group `≥ n` moves to `group + 1` — then place the tag in n).
After the write, `compact_groups(conn)` renumbers the distinct non-zero groups densely in
ascending order, so no empty group survives any operation, including the one that emptied
the tag's old group. The command is `set_tag_pinned_group(name, target)`, replacing
`set_tag_pinned`; it answers the vocabulary as before and writes `library.json` as before.
Refused for a name that is not a tag, as `set_pinned` was. Pin from a menu is `Group(1)`;
"New group above" on a tag in group g is `NewGroupAt(g)`; "New group below" is
`NewGroupAt(g + 1)`; "Move to #x" is `Group(x)`; Unpin is `Unpin`.

**D4. The store derives `pinnedGroups: string[][]`, and `pinned` is its flattening.**
`vocabulary.svelte.ts`: `pinnedGroups` groups entries by `pinnedGroup` ascending and sorts
each group by `groupByCategory` + `sortTags` as `pinned` did; `pinned = $derived(
pinnedGroups.flat())` stays for `selectionTagCounts` and any other flat reader, so the two
cannot disagree. `isPinned` reads `pinnedGroup !== null`; `groupOf(name): number | null`
and `groupCount` (`pinnedGroups.length`) serve the menu. `setPinned(name, bool)` becomes
`place(name, target)` with the same error handling; the menu's five actions are five
targets, not five methods.

**D5. One row per group, a hairline between.** `Inspector.svelte` lifts the `<ul>` of pinned
tag chips into a snippet `pinnedTagRows(fill, onactivate)` used by both strips: `{#each
vocabulary.pinnedGroups as group, index}` → a `<ul class="flex flex-wrap gap-1">` per group,
the second and later rows with `mt-1.5 border-t border-border pt-1.5`. No label, no number:
the owner wants bands, not names; the number exists only in the menu. The recommendation was
"rows with a separator" over "a wider gap"; the owner judges on screen and the alternative is
one class change.

**D6. The menu lives in `TagVocabularyMenuItems`, shown wherever the tag is pinned.** After
Unpin and before the category group, for a pinned tag: "New group above", "New group below"
(only while the tag shares its group — alone, `NewGroupAt` plus compaction lands back where
it started, and `app-frame`'s "no control appears before it does something" applies; owner,
2026-09-24, on the reviewer's note), then, when `groupCount > 1`, one "Move to #x" item per
group other than the tag's own, in order. Sidebar rows, inspector badges and chips all mount this component, so a pinned tag
offers the same moves from every menu it has — one component, no flag. Unit B of
`sidebar-inspector-polish` places its two search items above this component in the chip's
menu, which is unchanged here.

## Risks / Trade-offs

- [Renaming a column touches every query on it] → `grep pinned` in `src-tauri` is the
  checklist; the v9 migration is the one deliberate exception.
- [An old `library.json` after a rebuild] → covered by D2's dual-key read and a test.
- [The strip's look] → owner feedback decides between rows and gaps; one class.

## Migration

Schema v11 (D1), automatic. `library.json` gains `pinnedGroup` on the next write; old files
read (D2).
