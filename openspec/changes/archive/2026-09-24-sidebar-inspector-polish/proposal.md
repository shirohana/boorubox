## Why

Nine browse-screen asks from the owner on 2026-09-24, from a day of use on a 2K monitor: the
thumbnail cap is too small, the inspector's sections are in the wrong frequency order, the
sidebar's Collections and Filter sections take more room than the Tags list they sit under,
the corner resize handle is hard to grab on a section pinned at the bottom, the sidebar's edge
toggles the panel by accident, and editing one image's tags needs a key. Requirements §6
(tagging flow, sidebars) and §9 (keyboard-first browsing). The keyboard rule of 2026-09-23
reserved `e` for exactly this ("keys go to the most frequent day-to-day actions… `e` is
reserved for editing one image's tags").

## What Changes

- **Tile cap 640px.** `GRID_TILE_MAX` in `packages/shared` and its Rust twin go from 360 to
  640; the toolbar slider and the Settings slider follow. The thumbnail edge that makes 640
  crisp is `one-level-buckets`' change, not this one.
- **Inspector sections in use order.** The title row stays first. Then Rating, Tags,
  Collections, the Upload button when a booru is configured, Info (the facts list and Posted),
  and Move to trash at the foot. The "No booru configured" hint moves to the very end.
- **Pinned tag chips act as search terms** from their context menu: "Search for this tag" and
  "Exclude from the search", the same two items the Tags list already has.
- **The sidebar's Filter selects** drop to the compact size.
- **The sidebar's edge rail is gone.** The toolbar toggle and ⌘B remain the two ways to
  collapse and expand.
- **Notes sits at the bottom** with the navigation on screens that have no result set to
  describe, instead of floating to the top.
- **Collection rows take the tag rows' size**: one shared row component, so the two lists
  cannot drift apart again.
- **Section heights are set by dragging the section's top edge**, for Collections and Notes,
  replacing the CSS corner handle. The border around the collection list goes.
- **`e` opens the tag editor** for the focused image, in the grid and in the viewer, opening
  the inspector first when it is hidden.

## Capabilities

### Modified Capabilities

- `app-frame`: no rail on the sidebar's edge; Notes and the navigation sit at the bottom
  together; `e` in the keyboard map; the collections and the note sized by dragging an edge.
- `collections`: the sidebar's collection rows take the tag rows' size; the list has no border
  and is sized by its top edge.
- `notes`: the note is sized by its top edge, not a corner handle.
- `library-browse`: the thumbnail cap; the inspector's section order.
- `tag-editing`: a pinned chip is a search term from its menu; `e` opens the editor.

## Non-goals

- Persisting section heights across launches (session-only, as today).
- A resizable sidebar or inspector width (`browse-polish` design D10 still stands for the
  frame's regions; this change resizes sections inside the sidebar).
- Any change to the selection panel's section order.
- The thumbnail edge and regeneration (`one-level-buckets`).
- Grouping pinned tags (`pinned-tag-groups`).
