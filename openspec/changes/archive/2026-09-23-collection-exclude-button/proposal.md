## Why

The search already reads `-collection:name` and the sidebar already marks an excluded
collection, but the only control on a collection row adds the positive term: to exclude one the
owner types the minus by hand (2026-09-23). The Tags section above has had include and exclude
buttons on every row since `tag-sidebar`. Requirements §6 (Danbooru-style tag search, sidebars).

## What Changes

- Every collection row in the sidebar gains the same two small buttons a tag row has: include
  (`collection:name`) and exclude (`-collection:name`). The name itself keeps toggling the
  positive term, as it does today.
- The query rewriters gain the collection twins of the account ones: add and exclude, built on
  the same metatag-list rewrite, so the rest of the query is untouched.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `collections`: "The sidebar lists the collections with counts" — each entry offers excluding
  the collection from the search as well as filtering by it; two scenarios.

## Non-goals

- A `-` toggle on the inspector's collection badges or the tile menu: those act on membership,
  not the search.
- Any change to how `-collection:` is compiled (`query.rs` already handles it).

## Impact

- `packages/app/src/lib/domain/tag-utils.ts` (`addCollectionToQuery`,
  `excludeCollectionFromQuery`) and `tag-query.test.ts`.
- `packages/app/src/lib/components/tags/CollectionsSection.svelte` (two buttons per row, the
  Tags section's markup).
