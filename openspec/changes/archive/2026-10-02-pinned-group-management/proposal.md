## Why

Pinned groups were built as bands (`pinned-tag-groups`, 2026-09-24): numbered, nameless,
moved one tag at a time, with naming, moving a group and bulk moves as non-goals. The owner
(2026-10-01): "I treated this feature as a mini helper, but now it growth. It becomes a day to
day use feature to me. Now I want to be able to name a group, rearrange groups, move many tags
from a group to another group or a new group." And: "I want the pinned tag groups keeps its
minimal appearance, but also supports these features I want", plus folding a group away.
Requirements §6 (the inspector as the casual door; Danbooru-style tagging).

## What Changes

- **A group is a thing of its own**: it has a position, an optional name and a folded state,
  kept with the library and restored by a rebuild. Tags still belong to exactly one group.
- **The strip stays as it is**, one row per group, a hairline between; the small dim `#n` at a
  row's right edge becomes the group's name when it has one, `#n` otherwise, and that label is
  the fold toggle: a folded row shows its label and the number of tags it hides. A single
  unnamed group has no label and does not fold, as today.
- **A named group survives being emptied**; an unnamed empty group closes up as today. A
  name is the user's work, and the precedent is a general tag with a note, which is never
  pruned (`tag-notes`).
- **A "Manage pinned groups…" dialog** from any pinned chip's menu: every group in order with
  its name editable, a drag handle and "Move up" / "Move down" to reorder groups, a checkbox
  on every tag, "Move selected to…" naming each other group and "New group…", "New group…" on
  its own, and "Delete group" on an empty group. The one-tag moves already on every pinned
  tag's menu stay as they are; the dialog is for the rest.
- "Move to #n" items in a tag's menu read the group's name when it has one.

## Capabilities

### Modified Capabilities

- `tag-vocabulary`: "A tag can be pinned for one-click editing" is rewritten around groups
  that have names, an order and a fold.
- `library-recovery`: the library's own file keeps the groups' names, order and folds.

## Non-goals

- Dragging a chip between rows in the strip itself; the strip is kept minimal on purpose and
  a chip's click already writes to the image.
- Renaming from the strip (double-click or inline): the dialog is one menu item away and
  keeps the strip free of editing state.
- Colours or icons for a group.
