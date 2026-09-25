## Why

The owner's 2026-09-25 asks, all on the right-click menus and the pinned strip: with eight
pinned groups, "Move to #n" needs counting rows to find the target; a Danbooru-style tagger
sometimes needs a tag's wiki page; and the menus themselves read as one undifferentiated list —
items too large, group headings as dark as the items, the rating choices as text, the
destructive item unmarked. Requirements §6 (Danbooru-style tagging, the inspector as the
casual door) and §11's "ship the smallest thing that works".

## What Changes

- **A `#n` hint on each pinned group**, small and dim, at the row's right edge, only when there
  are two or more groups. Nothing else about the strip changes: no names, no empty groups, the
  number is the one "Move to #n" uses.
- **A Danbooru look-up item on every tag menu**: "Open Danbooru wiki" for a tag, "Search artist
  on Danbooru" for an artist tag (the artist's tag often differs from the name searched, so the
  artist search is the page that answers).
- **Denser menus**: smaller items, group headings smaller and dimmer than items, on every
  right-click and dropdown menu in the app, from one stylesheet block.
- **Icons on the well-known actions only**: categories, pin/unpin, trash, restore, edit, the
  external link. Not on every item (owner, 2026-09-25: "like the Apple, who adds icons for
  every menu items ... users feedback it's too messy").
- **The tile menu's rating as a row of coloured pills**, the current one filled, the same
  colours as the inspector's rating control.
- **"Move to trash" in the destructive colour**, still one click and no confirmation.
- **"Remove from this image" last** in the inspector's tag menu.

## Capabilities

### Modified Capabilities

- `tag-vocabulary`: the pinned strip's rows carry a `#n` hint when there are two or more
  groups; the tag menu offers a Danbooru look-up.
- `rating`: the thumbnail's context menu shows the five choices as coloured pills.

## Non-goals

- Icons on every menu item. A menu without an icon on some items is by design.
- Group names, drag-reordering, a "move to group" submenu. The hint is the whole fix.
- A configurable wiki host: the owner tags Danbooru-style and named `danbooru.donmai.us`; the
  self-hosted booru's wiki is not the reference.
- Confirming "Move to trash": the red is identity, not a warning (`trash` spec: one image moves
  without confirmation).
- Editing the shadcn copy-in files under `components/ui`: the density override lives in
  `app.css`, where the sidebar's overrides already live.
