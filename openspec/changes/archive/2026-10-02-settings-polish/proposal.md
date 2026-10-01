## Why

Two small frame fixes the owner raised on 2026-10-01 from daily use: on Settings → Library the
action buttons land at the left when their row wraps, and in the sidebar the Collections
header folds only when its label is clicked, where the Notes header folds on its whole row.
Requirements §6 (the app's frame).

## What Changes

- **Settings → Library: an action button always sits at the right edge of its row.** Today each
  row is description left, button right, wrapping; when the description is too wide for one
  line the button drops to a second line and lands left. Wrapped or not, the button is
  right-aligned.
- **Sidebar → Collections: the fold toggle is the whole header row** except the "+" button at
  its right end, the same clickable area the Notes header gives.

## Capabilities

### Modified Capabilities

- `app-frame`: "Settings is a set of pages with a nav" gains the rule that an action row keeps
  its button at the right edge.
- `collections`: "The sidebar lists the collections with counts" says where the fold is
  clicked.

## Non-goals

- Any change to what the buttons do, or to the Notes header.
