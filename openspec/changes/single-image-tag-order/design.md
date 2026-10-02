## Context

`domain/tag-categories.ts` holds one `CATEGORY_ORDER` and `groupByCategory` walks it. Five
callers group tags: the sidebar (`sidebarRows`), the pinned chips (`vocabulary.pinnedGroups`),
the inspector's tag list, the tile footer (`ImageCard`) and the editor (`editorText`). Two
more loop over `CATEGORY_ORDER` itself: the sidebar's category toggles and the menu's
category list.

## Decisions

### D1. Two orders: the library's and one image's

`CATEGORY_ORDER` stays `['artist', 'copyright', 'character', 'general', 'meta']` and is the
library order. A second constant, `IMAGE_CATEGORY_ORDER`, is
`['artist', 'copyright', 'character', 'meta', 'general']`: the order of the tags of one
image — the inspector's tag list, the tile footer, the editor's lines (owner, 2026-10-02).

*Reverses `tag-panel-polish` D1*, which made one order the app's only order: "two orders for
the same five words is a second source of truth for a fact a reader compares across panels",
and the editor's meta-before-general from `tag-vocabulary` had no argument recorded. That
was right while nobody had asked for a difference — the order was a lead's guess either way.
It stopped being right when the owner asked for two: the
comparison D1 guarded is now the owner's stated design, not drift. The guard against drift
moves to the code: two named constants in one module, each caller naming the one it means.

### D2. Which views take which order

One image's tags: the inspector's tag list, the tile footer, the editor's lines. Everything
else keeps the library order: the sidebar's rows and toggles, the pinned chips (a vocabulary
the user arranged, not one image's tags — owner, 2026-10-02), the menu's category list, the
stamp chips.

### D3. The order is an argument, never a default

`groupByCategory(items, nameOf, categoryOf, order)` takes the order as a required last
argument. A default would let a new single-image caller fall into the library order
silently; a required argument makes each call site say which it means.

## Risks / Trade-offs

- A reader comparing the sidebar with the inspector sees meta and general swap. Intended (D1).
