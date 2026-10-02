> One unit, done by the lead. Design D1–D3 decide every shape. Gate: `mise run check`.

## 1. Unit A — two orders (`packages/app/src/lib`)

- [x] 1.1 `domain/tag-categories.ts`: `IMAGE_CATEGORY_ORDER` per D1; `groupByCategory` takes
      `order` per D3; `sidebarRows` passes `CATEGORY_ORDER`; tests for both orders.
- [x] 1.2 `api/vocabulary.svelte.ts` (`pinnedGroups`) passes `CATEGORY_ORDER`;
      `domain/tag-input.ts` (`editorText`), `library/Inspector.svelte` (tag list) and
      `library/ImageCard.svelte` (footer) pass `IMAGE_CATEGORY_ORDER`; tests that pinned the
      old editor order updated.
- [x] 1.3 Gate green.
- [ ] 1.4 Hand check (owner): on an image with meta and general tags, the inspector's list,
      the tile footer and the editor's lines put meta before general; the sidebar and the
      pinned chips still put general before meta.
