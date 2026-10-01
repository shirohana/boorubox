## Why

A pinned tag is only visible as a chip in the inspector's strip; in the sidebar's tag list and
in an image's own tags there is no way to tell a pinned tag from any other. The owner
(2026-10-01) wants to see it in both places, and in the viewer's panel, "but the style
shouldn't be too high-contrast to be misleading with active tags. As a reminder, no
underline" — and as little extra width as possible. Requirements §6 (Danbooru-style tagging,
the inspector as the casual door).

## What Changes

- **A small muted dot after a pinned tag's name** in the sidebar's tag rows and in the
  inspector's list of the described image's tags, in both inspector placements (beside the
  grid and inside the viewer). A tag that is not pinned shows nothing. The dot is a disc the
  size of the text's x-height, in the muted colour, with the row's own gap before it: no
  background, no weight, no underline, no glyph — the search marking stays the only tinted
  thing on a tag.
- The pinned strip's chips, tile footers, the editor and the completion list are unchanged.

## Capabilities

### Modified Capabilities

- `tag-vocabulary`: a new requirement, "A pinned tag is marked where it is read" — its own
  requirement rather than a sentence in the pinned one, so `pinned-group-management`, which
  rewrites that one, and this change never modify the same text.

## Non-goals

- Showing which group a tag is in from the row; the strip says that.
- A marker on tiles or in completion rows.
