## Why

Four small frictions the owner hit tagging on the real vault (2026-09-28). The sidebar's
include/exclude pair sits spaced like two unrelated controls, and looking a tag up on Danbooru
takes a right-click even though Danbooru's own tag list puts a `?` beside every tag. The facts
form's only pencil is in the title row, a long scroll above the facts it edits. And the tag
editor survives a change of image: open it, click another tile (or press an arrow in the
viewer, or let a capture shift the rows), save, and the first image's draft is written to the
second image — a silent overwrite of the wrong image's tags. Requirements §6 (Danbooru-style
tag search and sidebars, the webview owns UI only).

## What Changes

- **The row's controls are one cluster**: in the shared sidebar row (tags and collections) the
  include and exclude buttons sit together with no gap, and the row's gap separates the
  cluster from the name.
- **A `?` look-up on every tag row**, first in that cluster, opening the same Danbooru page
  the tag's context menu opens (the wiki for a tag, the artist search for an artist tag).
  Collection rows have none: a collection has no Danbooru page.
- **A second facts pencil** in a heading row over the facts list, the same action as the
  title row's pencil, which stays.
- **Changing the described image closes the tag editor** and discards its text, as it already
  closes the facts form. A write to the same image does not close it.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `tag-sidebar`: a tag row offers the Danbooru look-up; a collection row does not.
- `tag-editing`: the tag editor closes when the described image changes, and never writes to
  an image other than the one it was opened for.
- `app-frame`: the facts' edit action is offered in two places; a change of image discards the
  facts form (built, not yet written down).

## Non-goals

- A notice surface for a look-up the browser refused. The `?` drops the failure the way the
  menu item does, under the menu item's existing FIXME (`menu-polish` D2); the shared notice
  is its own change.
- A look-up for collections, account rows, or the inspector's tag badges (the badge's menu
  already has it; the owner asked for the sidebar row).
- Keeping the tag draft across images, or asking before discarding it: the draft belongs to
  the image it was typed for, and a question on every tile click is a cost the owner did not
  ask for.
- Moving or removing the title row's pencil.

## Impact

Webview only, `packages/app/src/lib`: `components/tags/FilterRow.svelte`,
`components/tags/TagSidebar.svelte`, `components/tags/TagVocabularyMenuItems.svelte`, a small
shared look-up helper beside them, `components/library/Inspector.svelte`, and a new component
test `components/library/Inspector.svelte.test.ts` (the package's first Svelte component
mount; `vite.config.ts` may gain Svelte's documented browser resolve condition under Vitest).
No Rust, no wire type, no schema change.
