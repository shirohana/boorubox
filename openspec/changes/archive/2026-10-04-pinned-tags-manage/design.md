## Context

`PinnedTagsPanel.svelte` (539 lines) is the one panel behind `PinnedTagsDialog.svelte` and
`routes/settings/pinned-tags/+page.svelte` (`pinned-tags-panel` D2: the panel never knows
which door it is in; the page passes `stickyBar`, the dialog passes `portalTo` and a `barEnd`
snippet holding Done). The groups region `[data-groups]` is `tabindex="-1"` and the panel's
resting focus (`pinned-tags-focus-and-edge-scroll` D1); `write()` runs every vocabulary write
and afterwards focuses the element that had focus if it is still connected and enabled, else
the region. Sections are `{#each groups as group, index (index)}` — keyed by index because a
group's identity is its position (`pinned-group-management` D5). A group's header is a
snippet rendered by one of two branches: inside a `ContextMenu.Trigger` (Delete group) while
the group is empty, bare otherwise. The header holds the name `Input`, Move up, Move down and
"+" (pin into this group). Each tag row is a `ContextMenu.Trigger` with a `ReorderHandle`,
a `Checkbox`, a name `button` that ticks on a primary click, and an Unpin button; the row's
menu holds Unpin alone. `TagVocabularyMenuItems.svelte` is the tag menu every other host
mounts (sidebar row, inspector badge, pinned chip): Edit artist…, Edit note…, the Danbooru
look-up, Pin/Unpin, New group above/below, Move to <label>, "Manage pinned tags…"
(whenever a group exists), Category; its three callbacks are required props and each host
mounts its own `TagNoteDialog`, `ArtistDialog` and `PinnedTagsDialog` unconditionally
(`tag-notes` D10). `PinnedGroup.collapsed` is library data (owner, 2026-10-01) written by
`vocabulary.setGroupCollapsed`; the inspector strip folds on it, the panel ignores it. Notes
are read by `vocabulary.noteOf(name)`; the Artists settings list shows one as text under the
tag, the sidebar, badges and chips as a hover glyph (`tag-notes`). Settings pages that hold
data — Artists, Rules, Stamps, Booru — open as read lists with a per-row Edit that opens one
form; there is no shared edit-mode mechanism and none is needed for one page.

## Decisions

**D1. The panel has two modes, owned by the panel and started by the host.** `let editing =
$state(startEditing)` with a new prop `startEditing?: boolean` (default `false`). The page
passes nothing and opens reading, the settings rule; the dialog passes `startEditing` — it is
opened by "Manage pinned tags…", which is the edit intent said out loud, and a dialog that
opened reading would make every management task two clicks longer. The toggle is a `Button`
in the bar, first and pushed left (`mr-auto`), `PencilIcon` + "Edit", `aria-pressed={editing}`
and `variant={editing ? 'default' : 'outline'}` — the stamp chip's pattern: a pressed control
whose look follows the state it toggles. Leaving edit mode closes what only it can use: the
pin field, the naming form, the ticks (`ticked = []`), the drafts and rename errors
(`clearPositionKeyed()`); the move `Select` and "New group…" are rendered only while editing.
Not a per-group Edit as Artists has per row: a group's edits (name, order, members) are the
whole panel's edits, and one toggle is what the owner asked for ("read mode by default", an
Edit step).

**D2. Read mode draws a group as its label and its tags as plain rows.** The header shows the
fold toggle (D4) with the label inside it (`vocabulary.labelOf(position)`), nothing else; a
row is `div[data-tag-row]` holding the name as coloured text (`CATEGORY_TEXT_CLASS`, not a
button — there is no tick to toggle) and the note (D7); no handle, no `Checkbox`, no Unpin
button, no "+", no arrows, no name field. The row keeps its `ContextMenu.Trigger` (D6).
`tagDrag` stays attached to the region: with no `[data-tag-handle]` in the DOM nothing can
start a drag, so the mode needs no second guard. The "No tags" line stays for an empty group
in both modes.

**D3. One header element per position, whatever the group holds.** The header snippet is
rendered once, inside one `ContextMenu.Root` whose `ContextMenu.Trigger` is
`disabled={!editing || group.tags.length > 0}`; the content holds "Delete group" as today.
The element the pressed arrow lives in then survives a move that swaps an empty group with a
full one at its index, and survives the mode toggle, so `write()`'s "still connected"
restore finds it. The cause of the jump (owner, 2026-10-04) was exactly the branch flip: the
destroyed header dropped focus to the body, the dialog's focus scope took the first name field
and scrolled to it, and on the page the restore focused a groups region taller than the
viewport, which scrolls the page to its top. The comment on the `Root` says why the trigger
is disabled rather than conditional.

**D4. The fold is the strip's fold.** A fold toggle `button` leads each header in both modes:
`ChevronDownIcon` open, `ChevronRightIcon` folded, `aria-expanded`, `aria-label="Fold
<label>"` / `"Unfold <label>"`; in read mode the label text sits inside it (D2), in edit
mode the name `Input` follows it. It writes `vocabulary.setGroupCollapsed(position,
!collapsed)` through `write()`, so a group folded on the Settings page is folded in the
strip: one meaning of "folded", kept with the group in the library by the owner's 2026-10-01
call. A folded section renders no tag list, no "No tags" line and no pin field (folding the
group whose pin field is open closes it), and shows `· <count>` after the label or the name
field in the strip's dim text; it stays a drop target and keeps its arrows and "+" in edit
mode.

**D5. A focus restore never scrolls.** Both `focus()` calls in `write()` pass
`{ preventScroll: true }`: the restore is about where the next key goes, not about what is in
view, and the user's scroll position after pressing an arrow is where they pressed it. D3
removes the cause of the jump; D5 removes the mechanism, so a future branch flip cannot bring
it back.

**D6. The row's menu is `TagVocabularyMenuItems`.** The row's own "Unpin" item goes; the
shared component is mounted in the row's `ContextMenu.Content` with `onmanagegroups={null}`:
the prop's type widens to `(() => void) | null` and the item is not offered on `null`. Still
required, not optional — a host says "I am the manager" in so many words, and a mount that
forgets the prop still fails typecheck, the reason the three callbacks were made required. The
panel mounts one `TagNoteDialog` (`open`, `name`, `portalTo`, `onclose`) and one
`ArtistDialog` (`request = { mode: 'edit', tag, adapter: null, pageUrl: null }`, the
sidebar's shape, `portalTo`, `onsaved` doing what `ArtistDialog`'s doc comment asks of a host
with no search to re-run) unconditionally beside the menu, the `TagSidebar` pattern: the menu
content unmounts on select, so the dialog cannot live inside it. Inside the dialog door this
is a dialog over a dialog; both portal to `portalTo` or `body`, Escape closes the top one
first — the hand check watches it. The Unpin icon button at the row's end stays in edit mode
(`pinned-tags-panel` D6: one visible control per action), and the menu's Unpin is the same
write.

**D7. The note is text on the row.** When `vocabulary.noteOf(tag)` is not null, a `<span
class="min-w-0 flex-1 truncate text-muted-foreground" title={note}>` follows the name, in
both modes; the name loses `flex-1` and keeps `min-w-0 truncate`. Text and not the glyph the
sidebar and chips use: a management list is read for its notes, and a glyph per row is a hover
per row. Truncated to the line with the whole note in `title`, not wrapped as Artists wraps
it: the Artists list has one tag per entry, this list has every pinned tag. The owner judges
both on screen.

**D8. A group index above the groups.** When `groups.length > 1`, a `<nav aria-label="Groups"
class="flex flex-wrap gap-1">` precedes the groups region with one small ghost `Button` per
group reading its label, `onclick` scrolling that group's section
(`groupsRegion.querySelector('[data-group-position="n"]').scrollIntoView({ block: 'start' })`)
— `scrollIntoView` scrolls every scrolling ancestor, so the one call serves the dialog, where
the region scrolls, and the page, where the layout's scroller does. Hidden at one group: an
index of one entry is a control that does nothing (`app-frame`). The prop `stickyBar` becomes
`sticky` — "the host scrolls as a whole: pin the index to its top and the bar to its
bottom" — and the index gets `sticky top-0 z-10 bg-background pb-2` under it; the page
passes `sticky`. Not sticky in the dialog, for the reason the bar is not
(`pinned-tags-dialog-layout` D2): there it is already outside the scroll region.

**D9. Tests that pin the rules.** `PinnedTagsPanel.svelte.test.ts`: a mount with no props has
no `input`, no checkbox and no Unpin button and offers "Edit"; pressing Edit shows them and
pressing it again hides them and clears a tick; `startEditing` mounts editing; the fold toggle
calls `set_pinned_group_collapsed` and a folded group lists no tags and shows its count; Move
down on a full group next to an empty one leaves `document.activeElement` the very same
button node (`toBe`), and a spy on `HTMLElement.prototype.focus` sees `{ preventScroll: true
}`; a right-click on a row offers "Open Danbooru wiki", "Edit note…", "New group above",
"Move to Spare" and no "Manage pinned tags…"; choosing "Edit note…" opens a dialog in the
document; a noted tag's row shows the note text with the whole note in `title`; the index is
absent at one group, lists `Animals`, `#2`, `Spare` at three, and a click calls
`scrollIntoView` on that section (stubbed on `Element.prototype`); `sticky` puts the sticky
class on the index and the bar. `PinnedTagsDialog.svelte.test.ts`: the dialog mounts the
panel editing. `TagVocabularyMenuItems` gets its first test or the panel's test covers the
`null` case — the latter, since the component has no test file and the panel is its only
`null` host.

## Risks / Trade-offs

- [A fold on the Settings page folds the strip too] → by design (D4); if the owner wants a
  page-only fold it becomes view state and D4 is amended with why.
- [Nested dialogs from the dialog door] → the hand check opens Edit note… from inside the
  dialog and presses Escape twice.
- [The read-mode row has no primary action] → it has none to have; the menu is the door, as
  it is for a sidebar row.
- [`ContextMenu.Trigger disabled` still spreads data attributes onto the header] → harmless;
  the header is a plain `div`.
