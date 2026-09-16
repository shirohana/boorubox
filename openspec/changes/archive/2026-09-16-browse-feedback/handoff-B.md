# Handoff — agent B (group 2)

## 2.1 The stored fold (`collectionsCollapsed`)

Landed, copied line for line from `notesCollapsed`:

- `packages/shared/src/index.ts` — `AppSettings.collectionsCollapsed`.
- `packages/app/src-tauri/src/model.rs` — `AppSettings.collections_collapsed` field, and
  the `collectionsCollapsed` key added to `app_settings_crosses_the_wire_in_camel_case`.
- `packages/app/src-tauri/src/settings.rs` — `COLLECTIONS_COLLAPSED` key, field, default
  `false`, load, save, and the two tests the notes field has (round trip via the struct
  literal, and `a_file_without_the_collections_key_reads_as_expanded`).
- `packages/app/src-tauri/src/commands.rs` — `set_collections_collapsed`, and
  `the_collections_fold_reaches_the_state_and_the_store` mirroring the notes test.
- `packages/app/src-tauri/src/lib.rs` — registered.
- `packages/app/src/lib/api/commands.ts` — `setCollectionsCollapsed`.
- `packages/app/src/lib/api/settings.svelte.ts` — `Settings.setCollectionsCollapsed`.
- Tests: `commands.test.ts` (new `set_collections_collapsed` test, `collectionsCollapsed:
  false` added to every existing `AppSettings` literal) and `settings.svelte.test.ts`
  (field added to the `stored` fixture, a `setCollectionsCollapsed` round trip added to the
  write test).

No deviations. `cargo test` 567 passed, `cargo clippy -D warnings` clean, `cargo fmt` clean.

## 2.2 CollectionsSection.svelte

Landed per design D4:

- `Collapsible.Root` on `!settings.current?.collectionsCollapsed`, written through
  `settings.setCollectionsCollapsed`; the fold chevron sits in `Collapsible.Trigger`
  beside the "Collections" label, the "+" button stays a sibling outside the trigger (a
  button can't nest inside the trigger's own interactive element).
- The list is a `div.h-40.min-h-10.max-h-[50vh].resize-y.overflow-y-auto` wrapping the
  `<ul>`; `TagSidebar` above it is untouched and keeps the one `flex-1` region per D4, so
  growing the box shrinks the tag list only (component boundary — not directly verifiable
  from this file alone; see hand check).
- Rows: the `listed` derived (the active-first sort) and its comment are deleted; rows now
  render `counts` directly, in the name order Rust returns. Active/excluded marking is
  unchanged (still reads `activeTerms`), just no longer reorders.
- Each row is a `ContextMenu.Root`/`Trigger` (the `li`'s inner div, via the `child` snippet
  — the same pattern `ImageCard`'s tile trigger already uses) with `Content` offering
  Rename… and Delete…; the ellipsis `DropdownMenu` and its trigger button are gone.

No deviations from D4.

## 2.3 ImageCard.svelte — the crash and the mark

Landed per D6:

- The rating heading and its items (six ratings + "none") are wrapped in
  `ContextMenu.Group`, which is what bits-ui 2.19's `ContextMenu.GroupHeading` needs in
  context — this is the fix for the crash described in the design's Context section.
- A `BookmarkIcon` badge is added to the top-left badge group, shown when
  `image.collections.length > 0`, titled with the joined names from
  `collections.byId(id)?.name ?? id` (falls back to the bare id for a membership whose
  collection is gone, per D6's parenthetical).

**Deviation / could-not-write:** the jsdom DOM test in the task was attempted and then
removed. I wrote a test that used Svelte 5's `mount()`/`unmount()` to mount `ImageCard`
directly (there is no `@testing-library/svelte` in this repo, and no precedent anywhere in
the codebase of mounting a `.svelte` file in a test — every existing component-adjacent
test exercises a plain `.ts` module instead). It failed immediately with `Svelte error:
lifecycle_function_unavailable — mount(...) is not available on the server`: the `svelte`
package resolves to `src/index-server.js` under Vitest's default (SSR) module transform,
and only `svelte`'s `browser` export condition provides the client `mount`. Vitest does not
set `resolve.conditions: ['browser']` by default; enabling it needs a change to
`vite.config.ts` (`resolve.conditions` for the test run) and is genuinely shared
infrastructure — outside my file ownership for this task, used by every other package test,
and edited by nobody else's brief either. I did not make that change. The mount-based test
is deleted (nothing broken or half-working left behind); the crash fix itself is exercised
by `pnpm -r typecheck` (bits-ui's own types would reject a `GroupHeading` used wrong) and by
`pnpm lint`, but the actual "does bits-ui throw at runtime" question is only answered by the
hand check below, which the lead should still run.

Hand check (unticked, as instructed): right-click a tile — the menu opens with Rating,
Collections, Move to trash; pick `q` — the badge appears; add to Favorites — the tile gains
the mark, hover names it; a tile in no collection has no mark.

## 2.4 Menu heights

Landed per D5: `class="max-h-(--bits-floating-available-height) overflow-y-auto"` added to
`Inspector`'s `DropdownMenu.Content` (Add to…), `SelectionToolbar`'s outer
`DropdownMenu.Content` (Collection — not its inner per-collection `SubContent`, which is
the fixed two-item Add/Remove submenu, not the long list), and `ImageCard`'s
`ContextMenu.SubContent`. No deviations.

## Gate

- `cargo fmt` — clean (no diff).
- `cargo clippy --all-targets -- -D warnings` — clean.
- `cargo test` — 567 passed, 1 ignored, 0 failed.
- `pnpm -r typecheck` — shared, app (1156 files, 0 errors, 0 warnings), extension all done.
- `pnpm lint` — 0 errors, 0 warnings (ran `eslint --fix` once to settle Tailwind class
  order/wrapping, then hand-wrapped two long lines it couldn't auto-fix; re-ran clean).
- `pnpm --filter @boorubox/app test` — 531 passed across 46 files.

## Files touched

`packages/shared/src/index.ts`, `packages/app/src-tauri/src/{model,settings,commands,lib}.rs`,
`packages/app/src/lib/api/{commands,commands.test,settings.svelte,settings.svelte.test}.ts`,
`packages/app/src/lib/components/tags/CollectionsSection.svelte`,
`packages/app/src/lib/components/library/{ImageCard,Inspector,SelectionToolbar}.svelte`.

Not touched: anything under `src/lib/components/ui`, `tasks.md`, any file owned by agents
A/C/D.
