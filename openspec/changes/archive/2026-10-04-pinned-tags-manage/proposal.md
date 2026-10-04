## Why

Four things the owner asked for on 2026-10-02 and 2026-10-04 about the Pinned tags panel —
the one component behind the Settings page and the "Manage pinned tags…" dialog:

- Every Settings page opens read-only with an Edit step, except Pinned tags, which opens
  with live name fields, checkboxes, drag handles and Unpin buttons (owner, 2026-10-02:
  "every settings page should open read mode by default"). Artists, Rules, Stamps and Booru
  already open as read lists with a per-row Edit; General and Library are live controls with
  nothing to protect; Pinned tags is the one page left.
- A tag's row in the panel offers only Unpin, while the same tag's menu everywhere else offers
  the Danbooru look-up, Edit note…, New group above/below, Move to <group> and the category
  (owner, 2026-10-04). The row shows no note either; a management list is where a note is
  read.
- A group cannot be folded in the panel, though the strip folds it (owner, 2026-10-04:
  "support collapse pinned tag in the manage page"), and with many groups there is no way to
  reach one but scrolling (owner, 2026-10-04: "a small sticky block to show all the groups,
  and click to scroll to it").
- Moving a group with the arrow buttons sometimes scrolls the groups to the top, in the dialog
  and on the page (owner, 2026-10-04). The group sections are keyed by index and a group's
  header is rendered by one of two branches — wrapped in a context menu while the group is
  empty, bare otherwise — so a move that swaps an empty group with a full one at an index
  destroys the header holding the pressed arrow; focus falls to the body, the dialog's focus
  scope sends it to the first name field and scrolls to it, and on the page the panel's own
  restore focuses the groups region, which, taller than the viewport, scrolls the page to its
  top.

Requirements §6.

## What Changes

- **The panel has a read mode and an edit mode.** Read mode shows each group as its label
  with a fold toggle and its tags as plain rows with their notes and the tag's menu; edit mode
  is today's panel. An Edit toggle in the bar switches. The Settings page opens reading; the
  dialog, opened by "Manage pinned tags…", opens editing.
- **A tag row's menu is the tag's menu.** The row's context menu offers what the sidebar and
  inspector menus offer — Edit artist…, Edit note…, the Danbooru look-up, Unpin, New group
  above/below, Move to <group>, Category — minus "Manage pinned tags…", since the panel is
  where that leads. The row shows the tag's note after its name.
- **A group folds in the panel**, by the same fold the strip keeps with the group.
- **A group index above the groups**, one entry per group by its label; activating one scrolls
  that group into view. It stays in view on the Settings page as the bar does.
- **A move keeps the pressed arrow and never scrolls.** One header element per group
  position, whatever the group holds, and a focus restore that does not scroll.

## Capabilities

### Modified Capabilities

- `tag-vocabulary`: "A tag can be pinned for one-click editing" describes the panel's two
  modes, the row menu, the fold, the index and the move's focus rule; "A tag's note is shown
  where the tag is read" adds the panel's row, where the note is text.

## Non-goals

- Reordering tags within a group.
- Renaming or deleting a group from read mode.
- A read mode for General or Library (live controls with nothing to protect).
