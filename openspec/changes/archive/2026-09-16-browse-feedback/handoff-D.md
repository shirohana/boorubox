# Handoff — agent D, group 4 (tasks 4.1–4.3)

## 4.1 — `tag-utils.ts` + tests

Added `addAccountToQuery(query, handle)` and `excludeAccountFromQuery(query, handle)` in
`packages/app/src/lib/domain/tag-utils.ts`, right after `toggleAccountInQuery` and before
`toggleCollectionInQuery`. Both are written on the existing (unexported) `rewriteMetatagList`
helper, mirroring `addTagToQuery` / `excludeTagFromQuery`'s one-way shape: idempotent when the
handle is already on the side being asked for, and they drop the opposite side's entry first
(the excluded-stops-being-excluded rule `toggleAccountInQuery`'s comment already states — the
new pair's doc comments point at that comment rather than restating it).

`toggleAccountInQuery` itself was **not** touched; it already existed and is reused as-is by the
rail (see 4.2) for the "click the handle to take it out" gesture.

Tests live in `packages/app/src/lib/domain/tag-query.test.ts` (the file that actually holds
`tag-utils`'s tests — there is no `tag-utils.test.ts` in this repo; see Deviations), in new
`describe('addAccountToQuery')` / `describe('excludeAccountFromQuery')` blocks placed
immediately after `excludeTagFromQuery`'s block, beside the tag pair's as design D9 asks. Cases:
empty-query start, no-op when already on the asked-for side, dropping the opposite side instead
of contradicting it, and the rest of the query (a tag, `rating:s`) left untouched — for both
functions.

## 4.2 — `AccountRail.svelte`

New file: `packages/app/src/lib/components/library/AccountRail.svelte`.

Props (exactly as specified):
```ts
interface Props {
  groups: GroupSlice[]   // from '@boorubox/shared'
  tagQuery: string
  onquery: (next: string) => void
}
```

Shape copied from `TagSidebar.svelte`'s row (three buttons + count), fed by `GroupSlice.key`/
`.count` instead of `TagCount.name`/`.count`, rendered in the order `groups` is given —
never sorted, since Rust already orders largest-first:

- `+` button → `addAccountToQuery(tagQuery, key)` (include; no-op if already included)
- `−` button → `excludeAccountFromQuery(tagQuery, key)` (exclude; no-op if already excluded)
- the handle itself is a button → `toggleAccountInQuery(tagQuery, key)` (the existing helper) —
  this is what satisfies the spec's "acting on an entry already included or excluded SHALL take
  it out of the search": clicking the name of an active row removes it, clicking `+`/`−` again
  on an already-active row is a no-op, exactly like `TagSidebar`'s name-button/`toggleTagInQuery`
  pairing.
- marking via `activeTerms(tagQuery)`: included → `bg-sky-500/15 ... text-sky-700
  dark:text-sky-300` (blue — the same colour `Inspector.svelte`'s account row uses, per design
  D9, not `TagSidebar`'s emerald), excluded → `bg-destructive/10 text-destructive line-through`.

The component renders its own heading (`Accounts`) and `p-2` padding; it does **not** render the
outer `aside` sizing/border/scroll classes — those belong to `LibraryScreen` per design D9 and
task 4.4 (below), since the same rail markup could in principle be reused without them.

## 4.3 — snippet for the lead (task 4.4)

Import: `import AccountRail from '$lib/components/library/AccountRail.svelte'`

Paste between the grid column and the inspector's `aside`, gated on the group:

```svelte
{#if results.group === 'x-account'}
  <aside class="w-48 shrink-0 border-s border-border overflow-y-auto">
    <AccountRail
      groups={results.groups}
      tagQuery={tagQuery}
      onquery={(next) => searchKeeping(next, focused?.id)}
    />
  </aside>
{/if}
```

(`results` / `tagQuery` / `searchKeeping` / `focused` are whatever names task 1.4 lands with on
`LibraryScreen`'s side — this is the exact wiring shape, not tied to agent A's local variable
names if they differ from today's.)

## Gate

- `pnpm -r typecheck` — 0 errors (all packages)
- `pnpm lint` — clean (0 errors, 0 warnings)
- `pnpm --filter @boorubox/app test` — 46 files, 524 tests passed

## Deviations from the brief

- The brief's reading list and task 4.1 say `tag-utils.test.ts`; no such file exists in this
  repo. `tag-utils.ts`'s tests actually live in `packages/app/src/lib/domain/tag-query.test.ts`
  (confirmed: `toggleAccountInQuery`, `activeTerms`, etc. are all tested there already). New
  tests were added there instead, beside the tag pair's tests as design D9 directs.
- Nothing else deviates: props are exactly the three specified, file ownership was respected
  (no edits to `LibraryScreen.svelte`, `Inspector.svelte`, `Lightbox.svelte`, or `ui/`), no
  commit was made, tasks.md was not edited.
