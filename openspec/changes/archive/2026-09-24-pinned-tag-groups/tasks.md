> Two units in sequence, Sonnet each (retried on Opus if the gate fails): unit R (Rust +
> shared: the migration, the row, the write, the command), then unit W (webview: the store,
> the menu, the rows) after R lands and after `sidebar-inspector-polish` units B and D have
> landed (all three edit `Inspector.svelte`). Design D1–D6 decide every shape; do not
> re-decide them. Gate: `mise run check` green. No unit ticks a hand check. The migration's
> number is whatever `MIGRATIONS.len()` is when unit R lands (planned v11; 10 today): amend
> design D1's sentence to the real number, never pin the planned one. `one-level-buckets`
> unit R may be in flight on `commands.rs`, `lib.rs`, `model.rs`: its edits are additive
> (thumbnail types and a command); rebase, never merge by hand.

## 1. Unit R — column, row, write, command (`packages/app/src-tauri`, `packages/shared`)

- [x] 1.1 `db.rs`: `SCHEMA_V11` (the real number) per D1, appended to `MIGRATIONS`, with its
      doc comment. Test `a_v10_library_migrates_reading_every_pinned_tag_in_group_one` (the
      shape of the v7 test: a pinned row reads `pinned_group = 1`, an unpinned one `0`).
- [x] 1.2 `model.rs` `TagEntry.pinned_group: Option<u32>` with the dual-key deserialiser per
      D2 and its doc comment; `packages/shared` `TagEntry.pinnedGroup: number | null`;
      `PinTarget` enum (`unpin` | `{ group: n }` | `{ newGroupAt: n }`, camelCase, tagged as
      `commands.ts`'s invoke test names it) in both. Tests in `model.rs`: the wire test for
      `TagEntry` (`pinnedGroup` present; `pinned: true` reads `Some(1)`; `pinned: false` and
      absent read `None`).
- [x] 1.3 `tags.rs`: every `pinned` read/write becomes `pinned_group` (`vocabulary`, the
      exceptions rule, the `LEFT JOIN` default); `place_pinned(library, name, target)` and
      `compact_groups(conn)` per D3 replacing `set_pinned`; `recover::insert_tags` writes the
      group with the lowest-non-zero merge rule. Tests (`tags.rs`): `pin_lands_in_group_one`,
      `new_group_at_shifts_the_groups_after_it`, `new_group_below_the_last_makes_a_new_last`,
      `move_to_a_group_past_the_last_makes_a_new_last`, `an_emptied_group_closes_up`
      (three groups, unpin the middle, the third is now 2),
      `place_pinned_on_an_unknown_tag_is_refused`, `vocabulary_lists_a_grouped_tag`;
      (`recover.rs`): `a_rebuild_restores_pinned_groups`,
      `an_old_library_file_restores_pins_into_group_one`. Verify: `cargo test tags::
      recover:: db:: model::` pass.
- [x] 1.4 `commands.rs` + `lib.rs`: `set_tag_pinned_group(name, target)` replacing
      `set_tag_pinned`, registered in its place; its test renamed and extended. Verify:
      `mise run check` green.

## Handoff (unit R)

Landed on `main`, uncommitted (unit R does not commit per its brief). Schema is now v11
(`MIGRATIONS.len() == 11`, `db.rs`'s `SCHEMA_V11`: `ALTER TABLE tags RENAME COLUMN pinned TO
pinned_group`); D1's sentence in `design.md` already named v11 as "planned", so no amendment
needed — the real number matched the plan.

Wire shapes, exactly as they cross IPC:
- `TagEntry { name: string, category: TagCategory, pinnedGroup: number | null }` — `null`
  unpinned, `n` (>= 1) group `n`. Rust: `pinned_group: Option<u32>`, hand-written
  `Deserialize` (D2) reads either `pinnedGroup` (new files) or the old `pinned: bool` key
  (`true` → `Some(1)`, `false`/absent → `None`), `pinnedGroup` winning when both are present.
  `Serialize` is derived — only `pinnedGroup` is ever written, never `pinned`.
- `PinTarget` crosses as `'unpin' | { group: number } | { newGroupAt: number }` — Rust's
  plain externally-tagged enum representation (`PinTarget::Unpin`, `PinTarget::Group(u32)`,
  `PinTarget::NewGroupAt(u32)`, `#[serde(rename_all = "camelCase")]`, no `tag`/`content`
  attribute needed): a unit variant serialises as its bare name, a one-field tuple variant as
  `{ variantName: value }`. No wrapper key, exactly the shape the brief asked for.
- Command: `set_tag_pinned_group(name: string, target: PinTarget) -> TagEntry[]`, replacing
  `set_tag_pinned`, registered in `lib.rs` in its old place.

Both types are mirrored by hand in `packages/shared/src/index.ts` (`TagEntry`, `PinTarget`)
next to the Rust ones in `model.rs` — no generator, per the file's own `FIXME`.

No deviation from D1–D3. One thing D3 leaves implicit that matters for unit W: `Group(n)`
clamps `n` to `[1, max_group + 1]` — a target past the last existing group always lands as
exactly one new last group, never the literal number asked for — and `compact_groups` runs
inside the same transaction as every write, so a group emptied by the same operation that
created a new one is already closed by the time the command answers. The menu's "Move to #x"
items should therefore only ever be built from the vocabulary's actual current group numbers
(`groupOf`/`groupCount` per D4), never a number typed or computed independently.

Gate: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo test` (739 passed, 0
failed) all green from `packages/app/src-tauri`. `pnpm --filter @boorubox/shared typecheck`
green (no `build` script in that package). `mise run check` from the repo root: `lint` green,
`typecheck` red only on `packages/app`, all 12 errors against `TagEntry.pinned` no longer
existing, in `src/lib/api/vocabulary.svelte.ts` (2), `src/lib/api/commands.test.ts` (3) and
`src/lib/api/vocabulary.svelte.test.ts` (7) — exactly the webview surface unit W owns. First
lines:
```
src/lib/api/vocabulary.svelte.ts 49:65 "Property 'pinned' does not exist on type 'TagEntry'."
src/lib/api/vocabulary.svelte.ts 62:44 "Property 'pinned' does not exist on type 'TagEntry'."
src/lib/api/commands.test.ts 507:74 "Object literal may only specify known properties, and 'pinned' does not exist in type 'TagEntry'."
```

Nothing in `packages/app/src/**` was touched, per the brief's boundary.

## 2. Unit W — store, menu, rows (`packages/app`), after R and after `sidebar-inspector-polish` B and D

- [x] 2.1 `api/commands.ts`: `setTagPinnedGroup(name, target)` replacing `setTagPinned`,
      invoke-shape test. `api/vocabulary.svelte.ts` per D4: `pinnedGroups`, `pinned` as its
      flattening, `isPinned`, `groupOf`, `groupCount`, `place(name, target)`. Tests in
      `vocabulary.svelte.test.ts`: `pinnedGroups orders groups then category then name`,
      `pinned is the groups flattened`, `groupOf reads the group and null for unpinned`, and
      the existing `pinned` test amended to the new shape.
- [x] 2.2 `tags/TagVocabularyMenuItems.svelte` per D6: Pin → `place(name, { group: 1 })`,
      Unpin → `place(name, 'unpin')`, then the two insert items and the "Move to #n" items
      for a pinned tag, with the comment that says why they are here and not on the chip.
- [x] 2.3 `Inspector.svelte` per D5: the `pinnedTagRows` snippet drawing one `<ul>` per
      group with the separator classes, used by the single-image strip and the selection
      strip; `selectionTagCounts` keeps reading `vocabulary.pinned`. Verify: `mise run check`
      green.
- [ ] 2.4 Hand check: pin three general tags; "New group below" on one makes a second row
      with a separator; "Move to #1" brings it back and the second row vanishes; "New group
      above" on a tag in #2 makes it #2 and the old #2 becomes #3; unpinning the only tag of
      #2 closes it; the same in the viewer and the selection panel; the sidebar row's menu
      offers the same moves for a pinned tag; a rebuild keeps the groups.

Hand check: not run by this unit (no dev server, per the brief's boundary). Verify by hand
per 2.4's steps above, in the viewer's single-image strip, the selection panel's strip, and
the sidebar row's own menu.
Seen 2026-09-24 (smoke, scratch copy of test-1): pinned blue_archive, he and has from the sidebar menu (/Users/shirohana/.claude/jobs/8dbef24a/tmp/13-three-pinned.png). "New group below" on he gave a second row under a hairline (/Users/shirohana/.claude/jobs/8dbef24a/tmp/15-two-groups.png, /Users/shirohana/.claude/jobs/8dbef24a/tmp/15-two-groups-fullres.png). "Move to #1" brought it back and the row went away (/Users/shirohana/.claude/jobs/8dbef24a/tmp/17-moved-back.png). With #2 = [has, he], "New group above" on he gave [blue_archive] / [he] / [has] (/Users/shirohana/.claude/jobs/8dbef24a/tmp/21-three-groups.png). Unpinning he closed its group (/Users/shirohana/.claude/jobs/8dbef24a/tmp/23-unpinned-group-closed.png). The viewer's inspect strip has the same menu and "Move to #1" / "New group below" work there (/Users/shirohana/.claude/jobs/8dbef24a/tmp/40-viewer-chip-menu.png, /Users/shirohana/.claude/jobs/8dbef24a/tmp/41c.png). The selection panel (two selected) shows the two groups with the separator and the same menu (/Users/shirohana/.claude/jobs/8dbef24a/tmp/46-selection-two.png, /Users/shirohana/.claude/jobs/8dbef24a/tmp/47c.png). The sidebar row menu for a pinned tag offers Unpin / New group above / New group below / Move to #1 (/Users/shirohana/.claude/jobs/8dbef24a/tmp/24-sidebar-pinned-menu.png). A menu opened on a sidebar row scrolled out of view came up as WebKit's native Back/Reload/Inspect Element menu, not the app menu (/Users/shirohana/.claude/jobs/8dbef24a/tmp/11.png). That is an artefact of AXShowMenu, not a user path. Not checked: whether a rebuild keeps the groups.

## Handoff (unit W)

Landed on `main`, uncommitted (unit W does not commit per its brief).

Store's public surface (`api/vocabulary.svelte.ts`, D4): `pinnedGroups: string[][]`
(group-ordered, each group `groupByCategory` + `sortTags`, group numbers taken from the
entries themselves rather than counted, so an already-compacted group is never
second-guessed here); `pinned = pinnedGroups.flat()` (unchanged reader contract — every
existing flat reader, `selectionTagCounts`'s names filter included, needed no change);
`isPinned(name)` = `groupOf(name) !== null`; `groupOf(name): number | null`; `groupCount =
pinnedGroups.length`; `place(name, target: PinTarget)` replacing `setPinned`, same
error-to-`error`-field handling as `setCategory`.

Menu (`TagVocabularyMenuItems.svelte`, D6), in order, for a pinned tag: Unpin
(`place(name, 'unpin')`) — Pin (`place(name, { group: 1 })`) when not pinned — "New group
above" (`place(name, { newGroupAt: group })`) — "New group below"
(`place(name, { newGroupAt: group + 1 })`) — "Move to #n" for every group but the tag's own
(`place(name, { group: n })`), `n` from `vocabulary.groupCount`, never computed
independently (R's handoff warning: `Group(n)` clamps past the last group).

`Inspector.svelte`: `pinnedTagRows(fill, onactivate)` snippet, one `<ul class="flex
flex-wrap gap-1">` per `vocabulary.pinnedGroups` entry, rows after the first also carrying
`mt-1.5 border-t border-border pt-1.5`. Both strips (the single-image Tags heading's ul and
the selection panel's ul) now render through it; the single-image call site lost its own
`mt-2` on the `<ul>` (the snippet's first row carries none) and gained a wrapping `<div
class="mt-2">` around the `{@render}` instead, since D5 fixes the snippet's signature to
`(fill, onactivate)` with no class parameter.

Deviation: none from D4–D6. One unplanned Rust-side change landed concurrently and
independently of this unit (visible in `git diff` on `model.rs`/`design.md`, not authored
here): `TagEntry`'s `Serialize` now also emits a redundant `"pinned": <bool>` field beside
`pinnedGroup`, for an old reader's benefit. The webview's `TagEntry` interface
(`pinnedGroup: number | null` only) needed no change for this — TS reads structurally, and
nothing here reads a `pinned` key off a `TagEntry` any more.

Gate: `pnpm --filter @boorubox/app test` — 709 passed. `pnpm --filter @boorubox/app
typecheck` — 0 errors (the `tag-input.test.ts` errors R's handoff left for the other unit
were gone by the time this ran — that unit landed concurrently). `npx eslint` from the repo
root on every file this unit touched — clean. Full `mise run check`: green end to end on a
clean single run, confirmed after a first run (started while a second, overlapping `mise run
check` was also running in this same repo) showed one Rust test failing —
`recover::tests::a_rebuild_compacts_the_groups_a_merge_left_with_a_gap`, "No such file or
directory" — which passed alone (`cargo test recover::tests::a_rebuild_compacts_the_groups_a_merge_left_with_a_gap`)
and is outside this unit's scope (`packages/app/src-tauri`) either way; treated as a fixture
race from the concurrent runs, not a real failure, and confirmed by the clean solo run.

What the reviewer should look at first: `TagVocabularyMenuItems.svelte`'s `otherGroups`
derivation and the three `place()` calls it drives — that's the one place a wrong group
number would show up as a silent no-op or a wrong move. Second,
`vocabulary.svelte.ts`'s `pinnedGroups` derivation — the dedupe-and-sort of group numbers
without a `Set` (an eslint rule the codebase already works around for `#byName`; done here
with `filter`+`indexOf` instead, to avoid re-justifying the same exception twice).
