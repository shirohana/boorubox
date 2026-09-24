## Why

The owner pins many general tags, and the pinned strip lists them alphabetically as one run
(2026-09-24): "the tags are put randomly now, I just want to simply separate the tags" — by
drawing style, by what is in the picture, by clothes, by a character's look. Groups with no
names, only an order, are enough: the reader knows what each band is. Requirements §6
(Danbooru-style tagging, the inspector as the casual door).

## What Changes

- **A pinned tag belongs to one group.** Groups are numbered from 1 and have no names. A tag
  pinned from any menu lands in group 1. Within a group the strip keeps its order: category
  order first, alphabetical within a category.
- **The strip draws one row per group**, a thin separator between rows, no label.
- **A pinned tag's context menu moves it**: "New group above" and "New group below" insert a
  group beside the tag's own and move the tag into it; "Move to #x", one item per other
  existing group, moves it there. No "move up" or "move down": inserting is the operation the
  owner described (2026-09-24).
- **No empty groups.** When a move or an unpin empties a group, the groups after it close up.
- **The selection panel's pinned chips** show the same groups and offer the same menu.
- **The groups survive a rebuild**: `library.json` carries the group number with the tag.

## Capabilities

### Modified Capabilities

- `tag-vocabulary`: the pinned requirement gains groups; the vocabulary row carries a group
  number instead of a flag.

## Non-goals

- Naming a group, moving a whole group, or dragging chips between groups.
- Grouping pinned collections (their chips sit in the Collections section, one row).
- Any change to how a tag is pinned or unpinned, or to the sidebar's tag list.
