## Why

`pinned-tags-focus-and-edge-scroll` scrolls the dialog's own groups box when a drag reaches
its edge; on the Settings page, which scrolls as a whole, and on the Stamps table, nothing
scrolls, so a long list cannot be dragged across (owner, 2026-10-02: "it needs to fix before
publish"). Requirements §6.

## What Changes

- **Every drag in the app scrolls whatever scrolls under the pointer** when the pointer is
  held near that box's top or bottom edge: the dialog's groups box, the Settings page's
  content column, the Stamps table's page. One helper in the drag primitive, used by the
  reorder action and the panel. No new drag is added.

## Capabilities

### Modified Capabilities

- `tag-vocabulary`: the edge-scroll sentence says "whatever scrolls", not the dialog.
- `stamps`: the Stamps page's drag scrolls the page at its edges.

## Non-goals

- A new drag anywhere.
