## Why

Two things the owner met in the Pinned tags dialog on 2026-10-02: after Move up, Move down or
Unpin the focus lands in the first group's name field, and the same field is focused when the
dialog opens; and a tag dragged to the edge of the groups does not scroll them, so a group
taller than the visible area cannot be reached by drag. Requirements §6.

## What Changes

- **No field is focused by the dialog.** Opening it focuses nothing editable; after a write
  the focus stays where it was when that control still exists, and otherwise rests on the
  groups region, never on a name field.
- **A drag near the top or bottom edge of the groups scrolls them**, faster the closer to
  the edge, until the pointer moves away or the drag ends.

## Capabilities

### Modified Capabilities

- `tag-vocabulary`: "A tag can be pinned for one-click editing" says the panel never takes a
  field's focus on its own and that a drag scrolls at the edges.

## Non-goals

- Keyboard drag.
