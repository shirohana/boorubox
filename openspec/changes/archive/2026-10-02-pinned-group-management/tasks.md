> Two units in sequence, Sonnet each (retried on Opus if the gate fails): unit R (Rust +
> `packages/shared` + `packages/app/src/lib/api`), after `stamp-order` unit R has landed (it
> takes schema v17 and edits `db.rs`, `recover.rs`, `sidecar.rs`, `commands.rs`, `lib.rs`);
> then unit U (webview) after R, after `pinned-marker` (it edits `Inspector.svelte`,
> `FilterRow.svelte`, `TagSidebar.svelte`) and after `stamp-order` unit U (it leaves
> `common/reorder.ts` and `ReorderHandle.svelte` for this unit to reuse). Design D1–D11 decide
> every shape; do not re-decide them. The migration's number is whatever `MIGRATIONS.len()`
> is when unit R lands (planned v18; 16 today, 17 after `stamp-order`): amend design D1's
> sentence to the real number, never pin the planned one. Gates: unit R `mise run check` (it
> owns Rust); unit U `pnpm lint && pnpm typecheck && pnpm --filter @boorubox/app test`. No
> unit commits or ticks a hand check. Archive after `stamp-order` and `pinned-marker`: both
> deltas here were written as if those two had merged.

## 1. Unit R — the table, the writes, the wire (`packages/app/src-tauri`, `packages/shared`, `packages/app/src/lib/api`)

- [x] 1.1 `db.rs`: `SCHEMA_V18` per D1 with a doc comment naming
      `pinned-group-management` design D1, appended to `MIGRATIONS`. Test
      `a_v17_library_migrates_seeding_a_row_per_pinned_group`: tags in groups 1 and 3 (a
      v11-era database can carry a gap only before compaction; seed 1 and 2 and assert two
      rows if the migration follows a compaction — say which in the handoff),
      `user_version == MIGRATIONS.len()`.
- [x] 1.2 `model.rs`: `PinnedGroup { name: String, collapsed: bool }` (camelCase, derived
      serde, `#[serde(default)]` on both so an old file's partial row still reads) and
      `Vocabulary { tags: Vec<TagEntry>, groups: Vec<PinnedGroup> }` per D5, doc comments
      saying why the position is the index. `packages/shared/src/index.ts`: the same two
      types with doc lines; `TagEntry` unchanged.
- [x] 1.3 `tags.rs`: `vocabulary()` answers `Vocabulary` (groups from `pinned_groups ORDER
      BY position`); `compact_groups` per D2; `place_pinned_many` per D3 with `place_pinned`
      calling it; `rename_pinned_group`, `set_pinned_group_collapsed`, `move_pinned_group`,
      `create_pinned_group`, `delete_pinned_group` per D4, each ending in `compact_groups`
      and the `library.json` rewrite as `place_pinned` does; `set_category` and `set_note`
      answer `Vocabulary`. A test helper `groups_are_consistent(conn)` asserting D1's
      invariant, called at the end of every test that writes. Tests per D11:
      `a_named_empty_group_survives_compaction_and_an_unnamed_one_closes`;
      `new_group_at_shifts_rows_with_the_tags`; `move_pinned_group_keeps_every_tag_with_its_group`
      (forward and backward); `place_pinned_many_moves_all_or_none`;
      `create_pinned_group_refuses_a_blank_name_and_appends_otherwise`;
      `rename_to_blank_clears_the_name_and_an_empty_group_closes`;
      `delete_pinned_group_refuses_a_group_with_tags`; `a_position_with_no_row_is_refused`
      for rename, fold, move and delete; the existing pinned tests green with `Vocabulary`.
- [x] 1.4 `sidecar.rs`: `LibraryFile.pinned_groups: Option<Vec<PinnedGroup>>` per D6 with
      the `collections`-shaped doc; written from `vocabulary()`'s groups. `recover.rs`:
      restore per D6 (rows first, then `insert_vocabulary`, then `compact_groups`). Tests:
      `a_rebuild_restores_group_names_and_folds`; `a_library_file_without_pinned_groups_restores_unnamed_groups_from_the_tags`;
      `every_tags_column_but_id_is_represented_on_tag_entry` green;
      `an_old_reader_still_parses_a_file_with_pinned_groups` in the shape of the existing
      old-reader test. `artists.rs` `rename`'s group carry is unchanged and its test green.
- [x] 1.5 `commands.rs` + `lib.rs`: `tag_vocabulary`, `set_tag_category`,
      `set_tag_pinned_group`, `set_tag_note` answer `Vocabulary`; `move_pinned_tags(names,
      target)`, `rename_pinned_group(position, name)`, `set_pinned_group_collapsed(position,
      collapsed)`, `move_pinned_group(from, to)`, `create_pinned_group(name)`,
      `delete_pinned_group(position)`, registered; one command test in the shape of
      `set_tag_category_and_set_tag_pinned_group_reach_the_open_library_through_the_commands`.
      `packages/app/src/lib/api/commands.ts`: the wrappers, the four existing ones retyped to
      `Promise<Vocabulary>`; `commands.test.ts`: invoke tests. `vocabulary.svelte.ts` per D7
      with `vocabulary.svelte.test.ts` extended (`pinnedGroups` carries names and folds;
      `labelOf` falls back to `#n`; every setter replaces both arrays; `renameGroup` and
      `createGroup` answer whether they landed). `pnpm typecheck` names every other reader of
      the old `TagEntry[]` answer (`ArtistDialog.svelte`, `LibraryGrid.svelte`,
      `Inspector.svelte` reference the commands or the store) — fix the type, not the
      behaviour, and list each in the handoff.
- [x] 1.6 Gate `mise run check` green. Handoff: the real migration number, each refusal's
      wording, the exact `Vocabulary` JSON shape, and every file 1.5's typecheck named.

## 2. Unit U — the strip, the menu, the dialog (`packages/app/src/lib/components`)

- [x] 2.1 `library/Inspector.svelte` `pinnedTagRows` per D8: label, fold, count, the
      `vocabulary.pinnedGroups` shape from D7; the snippet's comment rewritten to D8's rule
      (no history). The strip's mount condition reads `vocabulary.pinnedGroups.length > 0`
      (a named empty group is a row). `Inspector.svelte.test.ts`: a folded group shows its
      label and count and no chips; a single unnamed group shows no label; a named single
      group shows its name.
- [x] 2.2 `tags/TagVocabularyMenuItems.svelte` per D9: labels through `labelOf`, the
      "Manage pinned groups…" item and the required `onmanagegroups` prop; every mount
      (`TagSidebar.svelte`, `Inspector.svelte`) threads it to one `PinnedGroupsDialog`, the
      `editingNote` shape, `Inspector`'s with `{portalTo}` and `onrelease`.
- [x] 2.3 `tags/PinnedGroupsDialog.svelte` per D10, reusing `common/reorder.ts` and
      `ReorderHandle.svelte` by `stamp-order` unit U's row contract. `PinnedGroupsDialog.svelte.test.ts`
      (jsdom, mocked store): checking two tags shows the move footer; "Move selected to" a
      group calls `placeMany` with both names and the target; a blank new-group name is
      refused in the dialog; "Delete group" is offered on an empty group only.
- [x] 2.4 Gate green. Handoff: which D8 layout landed and why, and anything the design did
      not foresee.
- [ ] 2.5 Hand check (owner): with three groups, name one from the dialog — the strip's row
      ends in the name, "Move to" items read it; fold it — the row shows `Name · n` in both
      inspector placements and in the viewer; drag a group above another in the dialog —
      the rows reorder with their tags; check three tags and move them to a new named group;
      unpin the last tag of a named group — its row stays as a label; delete it from the
      dialog. Rebuild library index keeps names, order and folds. At one unnamed group the
      strip is exactly as before. Judge D8's label layout at one, three and eight groups.
      Hand check: with VoiceOver, confirm each strip row still reads as a list (the `<ul>` is
      `display: contents`); if the role is lost, keep the `<ul>` as the flex container and lay
      the label out without `contents`.
      Seen (lead's smoke 2026-10-02 on a scratch copy of test-3, migrated v15 → v18 with three groups seeded): three rows ended in `#1` `#2` `#3`, labels on the chips' last line (boorubox-vault/smoke-2026-10-01/03-inspector.png); clicking `#2` folded it to `#2 · 3` and wrote `collapsed = 1` (boorubox-vault/smoke-2026-10-01/04-fold.png); the chip menu read Unpin, Move to #1, Move to #2, Manage pinned groups… (boorubox-vault/smoke-2026-10-01/05-chip-menu.png); the dialog listed the groups with handles, name fields and checkboxes (boorubox-vault/smoke-2026-10-01/06-dialog.png); naming group 2 `Games` on Enter wrote the name and the strip read `Games · 3`; checking two tags raised “Move 2 selected to…” whose options were #1, #3, New group… — Games omitted (boorubox-vault/smoke-2026-10-01/07-dialog-named-checked.png, boorubox-vault/smoke-2026-10-01/08-select.png); choosing #3 moved both, cleared the checks and updated the strip behind (boorubox-vault/smoke-2026-10-01/09-moved.png); the group's menu offered Move up and a disabled Move down at the end (boorubox-vault/smoke-2026-10-01/10-group-menu.png); Move up put Games third with its fold intact, the strip read `#1`, `#2`, `Games · 1` (boorubox-vault/smoke-2026-10-01/11-strip-after.png). Not driven: drag, New group…, Delete group, a named group emptied, a rebuild, the viewer placement, VoiceOver. The built D8 layout costs no extra line at three groups; eight groups and the fixed-column alternative are the owner's call.

## Handoff

Unit R (not committed). Gate `mise run check` green: Rust 916 passed / 1 ignored, app 932, extension 96, shared 1.

- Migration: `SCHEMA_V18`, `MIGRATIONS.len() == 18`. Design D1's heading and Context paragraph amended.
  The migration test seeds groups 1 and 3 and asserts rows 1 and 3: the migration is not followed
  by a compaction, and a gap cannot occur in a real v17 database (groups were always compacted).
- Refusals: unknown tag in `place_pinned_many` -> `NotFound("tag <name>")` (nothing written);
  position with no row (rename, fold, move either end, delete) -> `NotFound("pinned group <n>")`;
  blank `create_pinned_group` name -> `BadRequest("a group needs a name")`; delete of a group with
  tags -> `BadRequest("pinned group <n> still has tags")`.
- Wire: `Vocabulary` = `{ tags: TagEntry[], groups: { name: string, collapsed: boolean }[] }`;
  `groups[i]` is position `i + 1`; `TagEntry` unchanged. `library.json` gains
  `pinnedGroups: [{ name, collapsed }]` (LIBRARY_VERSION 1). Table `pinned_groups(position PK,
  name, collapsed)`.
- Commands: `tag_vocabulary`, `set_tag_category`, `set_tag_pinned_group`, `set_tag_note` answer
  `Vocabulary`; new `move_pinned_tags(names, target)`, `rename_pinned_group(position, name)`,
  `set_pinned_group_collapsed(position, collapsed)`, `move_pinned_group(from, to)`,
  `create_pinned_group(name)`, `delete_pinned_group(position)`.
- Store (`vocabulary.svelte.ts`): `entries`, `groups`, `pinnedGroups: { name, collapsed, tags }[]`,
  `labelOf(position)`, `groupCount`, `pinned`; `refresh(): Promise<void>`, `setCategory`,
  `place(name, target)`, `placeMany(names, target)`, `setGroupCollapsed(position, collapsed)`,
  `moveGroup(from, to)`, `deleteGroup(position)` all `Promise<void>`; `setNote`,
  `renameGroup(position, name)`, `createGroup(name)` -> `Promise<boolean>` (landed). A refusal sets
  `error` and leaves both arrays.
- Typecheck named / reads fixed (type or shape only): `Inspector.svelte` (`{#each group.tags}` in
  `pinnedTagRows`, no behaviour change), `TagVocabularyMenuItems.svelte`
  (`pinnedGroups[group - 1]?.tags.length`), `search.svelte.test.ts` (mock of `tag_vocabulary`
  answers `{ tags: [], groups: [] }`). `ArtistDialog.svelte` and `LibraryGrid.svelte` needed nothing.
- Deviation: `compact_groups` also creates the row for a group a tag names when none exists
  (rebuild from a file without `pinnedGroups`); `NewGroupAt(n)` clamps `n` to count + 1.

### Unit U (not committed)

Gate green: `pnpm lint` 0 errors (warnings in others' files only), typecheck 0 errors, app tests 940 passed (78 files).

- `Inspector.svelte`: `pinnedTagRows` per D8, strip mounts read `pinnedGroups.length > 0`, one `PinnedGroupsDialog` with `{portalTo}` and `onrelease`, `onmanagegroups` on both menu mounts. `Inspector.svelte.test.ts`: folded group (label, count, no chips), single unnamed (no label), single named.
- `TagVocabularyMenuItems.svelte`: "Move to" through `labelOf`, "Manage pinned groups…" after the moves (pinned tags only), required `onmanagegroups`. `TagSidebar.svelte`: dialog wiring.
- `tags/PinnedGroupsDialog.svelte` (+ test, 4 cases: footer appears on check, `placeMany` with both names and target via the real Select, blank name refused with no write, "Delete group" on an empty group only). New-group prompt is inline in the dialog, not a nested Dialog.
- D8 layout built: the row is the wrap container, the chips' `<ul>` is `contents` (role="list" kept explicit), the label button is its last item (`ml-auto order-last self-end`). Alternative: the label as a fixed trailing column (a `pr-*` reservation with the label absolutely positioned at the row's top-right), always on the first line, never sharing the last chip line; it costs a column of chip width at every group count.
- Not foreseen: `display: contents` on the `<ul>` relies on WebKit keeping the list role from `role="list"`; look at VoiceOver/real WebKit in the hand check. A blank new-group name is refused by Rust (`create_pinned_group`) and the dialog shows that answer; it carries no check of its own.

Review fixes applied: 1–11 (7 as a hand-check line).
