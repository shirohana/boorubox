## Why

Two things the owner saw on 2026-10-02 after `pinned-tags-panel` and `tag-indicator-order`:
a pinned chip that carries a note glyph sits higher than its neighbours in the inspector's
strip (the earlier fix straightened the tag list under the strip, not the chips), and in the
Pinned tags dialog the "Move N selected to…" control scrolls out of view with many tags,
while the whole dialog scrolls and its scrollbar runs outside the box. Requirements §6.

## What Changes

- **The sidebar dot sits right after the name**, before the row's remaining space; today the
  stretched name pushes it to the right beside the glyph.
- **The pinned chips sit on one line**, with or without a note glyph.
- **The dialog's header and footer stay put; only the groups scroll**, with the scrollbar
  inside the box. The footer holds "Move N selected to…", "New group…" and Done on one row,
  so the move control is always in reach. The Settings page keeps the same bar at the bottom
  of the page, sticky while the page scrolls.

## Capabilities

### Modified Capabilities

- `tag-vocabulary`: "A tag can be pinned for one-click editing" says the panel's bar stays in
  view; "A tag's note is shown where the tag is read" says a chip with a glyph stays on the line;
  "A pinned tag is marked where it is read" says where the sidebar dot sits.

## Non-goals

- Any other change to the panel's controls.
