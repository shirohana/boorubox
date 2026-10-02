## Why

The owner asked (2026-10-02) for two category orders: the left panel keeps Danbooru's
sidebar order (artist, copyright, character, general, meta), and wherever the tags of one
image are read the order is artist, copyright, character, meta, general. Requirements §6
(the inspector and the grid as the doors to one image's tags).

## What Changes

- **A single-image order.** The inspector's tag list, the tile footer and the tag editor's
  lines read artist, copyright, character, meta, general.
- **The library order stays** for the tag sidebar's rows and category toggles, the pinned
  tag chips, and the category list in a tag's menu.
- Reverses `tag-panel-polish` design D1's "one order" (argument kept in this design's D1).

## Capabilities

### Modified Capabilities

- `tag-editing`: "The tags of one image can be edited" (editor line order) and "Tags on
  screen are search terms" (the inspector's tag order).
- `stamps`: "Edit mode applies the active stamp by a click" (the tile footer's order).

## Non-goals

- Any change to the sidebar, the pinned chips, the stamp chips or the menu's category list.
- The upload form's tag string (`sortTags`, alphabetical, not grouped by category).
