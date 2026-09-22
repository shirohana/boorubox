> One Sonnet unit, webview only, files disjoint from every other change in flight. No design:
> the shapes are copies of the account rewriters and the tag row's buttons. Gate:
> `mise run check` green.

## 1. The rewriters and the row (`packages/app`)

- [x] 1.1 `domain/tag-utils.ts`: `addCollectionToQuery(query, slug)` and
      `excludeCollectionFromQuery(query, slug)` beside the account pair, on
      `rewriteMetatagList`, each removing the slug from the other list first (the spec's
      "include replaces exclude"). Tests in `tag-query.test.ts` copied from the account pair's:
      add to an empty query, add beside other terms, exclude replaces an inclusion, include
      replaces an exclusion, the rest of the query survives, an existing entry is not doubled.
      Verify: `pnpm --filter @boorubox/app test tag-query` passes.
- [x] 1.2 `components/tags/CollectionsSection.svelte`: the include and exclude buttons from
      `TagSidebar.svelte`'s row (same icons, sizes, `aria-label`s "Include {name}" / "Exclude
      {name}"), placed before the name button inside the context-menu trigger's child so the
      row's right-click still works; the name button unchanged. Verify: `mise run check` green.
      Hand check: the buttons draw on every collection row at the same size as the tag rows';
      `-` on `Uncategorized` puts `-collection:uncategorized` in the search and marks the row
      struck through; `+` on the same row flips it to `collection:uncategorized`; right-click on
      the row still offers rename and delete.
