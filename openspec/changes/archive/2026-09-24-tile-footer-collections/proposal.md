## Why

The tile footer lists an image's tags so a click's result can be read from the grid, and
`pinned-collections` adds a one-click way to drop an image into a collection from the
inspector. The answer to that click is on the tile only as the corner's bookmark mark, which
says "in some collection" and names them on hover. The owner asked for the footer to say which
(2026-09-24). Requirements §6 (Danbooru-style browsing, collections).

## What Changes

- **The footer names the image's collections**, before the tags: the bookmark mark the corner
  already uses, then each collection's name in plain text, then the tag groups as today. An
  image in no collection shows its tags alone. Same clamp, same hover expansion.
- The corner mark stays: the footer is off outside edit mode unless the view setting is on,
  and the mark is what says "in a collection" when it is off.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `stamps`: "Edit mode applies the active stamp by a click" — the footer under every thumbnail
  lists the collections first, then the tags; one scenario.

## Non-goals

- Acting on a collection from the footer (removing membership by a click there): the footer
  reads, the chip and the menus write.
- Colouring collections: every category hue is taken (`pinned-collections` design D7); the
  bookmark glyph and plain foreground text are the one look no tag has, and the same look the
  pinned chip uses, so "collection" reads the same on the tile and in the inspector.
- A footer height change: the footer is a fixed strip cut at three lines; collections take
  their share of those lines like any tag.

## Impact

- Webview only: `library/ImageCard.svelte` (the footer's first run, from the tile's existing
  `collectionNames`). No Rust, no shared type, no setting, no migration.
