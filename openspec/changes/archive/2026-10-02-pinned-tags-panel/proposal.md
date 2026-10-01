## Why

The owner used the "Manage pinned groups…" dialog for a day (2026-10-02): dragging a group
"triggers the background dropper and fails to swap"; a right-click on a tag row ticks its box;
a tag cannot be pinned or unpinned from the dialog; a named empty group has no chip whose
menu could reach the dialog; and the dialog is about tags more than groups. Requirements §6
(the inspector as the casual door; Danbooru-style tagging).

## What Changes

- **"Pinned tags", not "Pinned groups".** The menu item reads "Manage pinned tags…", the
  dialog's title "Pinned tags", and the same panel gets a Settings page, "Pinned tags" after
  Stamps — the Stamps precedent: the bar is the casual door, the page is where you manage.
- **The strip's group label has a context menu** with "Manage pinned tags…", so a group with
  no tags is still reachable.
- **Drag is pointer-driven.** The window keeps Tauri's drag-drop on for import by drop, which
  swallows every HTML5 drag inside the webview on macOS; the `stamp-order` design's HTML5 drag
  never worked in the app. The reorder primitive is rebuilt on pointer events with the same
  row contract; the Stamps page keeps its handle on it. The archive's design records the
  reversal.
- **In the panel, tags drag between groups and groups move by buttons.** A tag row's handle
  drags it onto another group's section, which highlights under the pointer; dropping writes
  one move, and when the dragged tag is ticked every ticked tag goes with it. Groups have
  Move up / Move down buttons in their header and no drag handle.
- **A right-click never ticks.** The row's menu opens without touching its checkbox.
- **Unpin and pin from the panel.** Each tag row ends in an Unpin control carrying the
  pin-off icon the chip menu uses, and "Unpin" is in the row's menu; each group's header has
  a "+" that opens a tag field at the group's foot, with the editor's completion, Enter pins
  the typed tag into that group and keeps the field for the next, Escape closes it.

## Capabilities

### Modified Capabilities

- `tag-vocabulary`: "A tag can be pinned for one-click editing" — the panel, its doors and
  its controls.
- `app-frame`: "Settings is a set of pages with a nav" — Pinned tags after Stamps.
- `stamps`: unchanged in text; the drag the spec promises now works through the pointer
  primitive.

## Non-goals

- Dragging chips in the inspector's strip: the strip stays as it is.
- Dragging a group: Move up / Move down is enough for a list that is rarely longer than ten
  (owner, 2026-10-02).
- Pinning a name that is not yet a tag of the library: Rust refuses it as before, and the
  field's completion offers existing tags only.
