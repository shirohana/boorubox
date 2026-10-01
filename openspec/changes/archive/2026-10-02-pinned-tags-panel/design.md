## Context

`PinnedGroupsDialog.svelte` (287 lines) holds the dialog shell and its body: one section per
group, a `ReorderHandle` and an `Input` for the name, a `ContextMenu` on the section with
Move up / Move down / Delete group, checkbox rows per tag, a footer with "Move N selected
to…" (a `Select`), "New group…" and Done. `common/reorder.ts` is `moveItem`, `dropIndex` and
the `reorderable` action on HTML5 drag events (`dragstart` on `[data-reorder-handle]`,
`dragover`/`drop` on `[data-reorder-index]` rows, `data-reorder-drop` marks);
`ReorderHandle.svelte` is a `draggable` grip. `StampsTable.svelte` is its one other user.
`SectionResizer.svelte` is the app's pointer-event drag: `setPointerCapture` at pointer-down,
the gesture ends in `onlostpointercapture`. `api/drag-drop.ts` says why the window's Tauri
drag-drop stays on: an HTML5 `drop` hands over `File`s with no path, and import needs paths.
`settings-pages.ts` is the one list the nav, redirect and fallback read. `TagInput.svelte`
takes `suggest` and completes tag names. `TagVocabularyMenuItems.svelte` offers "Manage
pinned groups…" whenever a group exists, raising `onmanagegroups`; `Inspector.svelte` and
`TagSidebar.svelte` each mount one dialog. `Inspector.svelte`'s `pinnedTagRows` draws the
group label as a `button` that folds the row.

`stamp-order` design D5 chose native HTML5 drag "because the webview is WebKit and WebView2,
both of which drag a `draggable` element with the OS ghost image for free." True of the
engines, and false inside this window: Tauri's drag-drop handler, which import by drop needs,
takes every drag in the webview before the page sees `dragover` (the owner, 2026-10-02: the
drag "triggers the background dropper"). The jsdom test dispatched the events the handler
never lets through, and the lead's smoke drove the menus, not a drag. The decision stops
holding the moment the window has a file dropper, which it has had since `local-file-import`.

## Decisions

**D1. `reorderable` keeps its contract and is rebuilt on pointer events, over one new
primitive, `common/pointer-drag.ts`.** `pointerDrag(handle, { onstart, onmove(x, y),
onend(x, y, dropped) })`: pointer-down on the handle captures the pointer, a move past 4px
starts the drag (`onstart`), every move reports client coordinates, `lostpointercapture`
ends it — `dropped: false` on `pointercancel`, Escape or a capture lost to a blur, exactly
`SectionResizer`'s ending — and `touch-action: none` on the handle. The consumer resolves what
is under the pointer with `document.elementFromPoint(x, y)?.closest(selector)`, since
pointer capture routes every event to the handle. `reorderable` becomes: `pointerDrag` on
each `[data-reorder-handle]` inside `node` (attached by delegation at pointer-down), the row
under the pointer gets `data-reorder-drop` as before, `onmove(from, to)` on a drop, marks
cleared on an end without a drop. `moveItem` and `dropIndex` are unchanged. `ReorderHandle`
loses `draggable` and gains `touch-action-none`; `StampsTable` changes nothing. The tests
drive `pointerdown`/`pointermove`/`pointerup` with `setPointerCapture`, `releasePointerCapture`
and `elementFromPoint` stubbed on jsdom, and they are honest about it: what they prove is the
index math and the event wiring, and the hand check proves the drag.

**D2. The panel is one component, `tags/PinnedTagsPanel.svelte`; the dialog and the
settings page are two doors.** `PinnedTagsDialog.svelte` is the shell: `Dialog.Root`,
title "Pinned tags", a one-line description, the panel, a footer with Done. The page
`routes/settings/pinned-tags/+page.svelte` is a `section` with the heading "Pinned tags",
one paragraph saying what a pinned tag is and that the groups live with the library, and the
panel. `SETTINGS_PAGES` gains `{ slug: 'pinned-tags', label: 'Pinned tags', path:
'/settings/pinned-tags' }` after Stamps; `settings-pages.test.ts` asserts the order. The
panel's own bottom bar holds "Move N selected to…" and "New group…", so both doors offer the
same controls. The panel reads and writes the vocabulary store only.

**D3. Groups move by two icon buttons in their header, after the name field: arrow up and
arrow down, disabled at the ends, `aria-label="Move <label> up"`.** The group's context menu
keeps Delete group (on an empty group) and nothing else: one control per action, the visible
one. No `ReorderHandle` on a group (owner, 2026-10-02: a short list is moved by buttons).

**D4. Tag rows drag to another group.** Each tag row leads with a `ReorderHandle`; the
panel's root carries `pointerDrag` by delegation on `[data-tag-handle]`. While dragging, the
section under the pointer (`[data-group-position]`) carries `data-drop-over` and draws a ring
(`ring-1 ring-primary`); the row being dragged is dimmed. On a drop onto another group:
`placeMany(names, { group: position })` where `names` is the ticked tags when the dragged tag
is among them, else the dragged tag alone — the tick is how "move these" is said, and a drag
on one of them is the gesture; onto its own group or outside every section, nothing. A drop
on the "New group…" button at the bottom opens the naming prompt with `move: true` for those
names, so a new group is one gesture away too.

**D5. A right-click never ticks.** The row is `div` with the handle, the `Checkbox`, a name
`button` (primary click toggles the tick; `onpointerdown` with `button !== 0` is not a
toggle), and the Unpin button; the `ContextMenu.Trigger` wraps that row. No `<label>` around
the checkbox: the label's own click-to-toggle is what fired on the press that opened the
menu. A test right-clicks the name and asserts the box stays unticked.

**D6. Unpin per row.** A ghost `icon-xs` button at the row's end, `PinOffIcon`, `aria-label=
"Unpin <tag>"`, and an "Unpin" item with the same icon in the row's menu; both call
`vocabulary.place(tag, 'unpin')`. A ticked tag that is unpinned drops out of the ticks by
the existing `selected` derivation.

**D7. Pin into a group from the panel.** A ghost `icon-xs` "+" (`PlusIcon`) in the group's
header, `aria-label="Pin a tag into <label>"`, opens one `TagInput` row at the group's foot
(`adding = position`, one at a time), `TagInput`'s default `suggest` (the library's tags), `label="Tag to pin"`, placeholder "Tag name". Enter: `place(name, { group: position
})`, then the field empties and stays for the next; a refusal (not a tag) shows under the
field from `vocabulary.error` and the text stays. Escape or blur with an empty field closes
it. The group's "No tags" line stays while it is empty.

**D8. Rename.** `TagVocabularyMenuItems`'s item reads "Manage pinned tags…"; the prop stays
`onmanagegroups` in name — a rename of a prop across four mounts for a label is churn with
no reader. The dialog file is renamed to `PinnedTagsDialog.svelte` and its test with it; the
spec's text follows.

**D9. The strip's label gets a context menu.** In `pinnedTagRows` the label `button` becomes
a `ContextMenu.Trigger` child; the content holds "Manage pinned tags…" raising the same
dialog the chip menus raise. A left click still folds. Both placements share the snippet,
so the viewer's strip has it too.

**D10. The `stamp-order` archive's D5 is amended in place** with two sentences: why HTML5
drag was right about the engines and wrong inside this window, and that `pinned-tags-panel`
D1 holds the pointer primitive. `api/drag-drop.ts`'s header already says the rule; it gains
one line pointing at `pointer-drag.ts` as what to use instead.

## Risks / Trade-offs

- [`elementFromPoint` under pointer capture] → it reads the hit-test tree, not the event
  route, so it answers the row under the pointer regardless of capture; the resizer does
  not need it, this does.
- [A drag that starts on a scrolling page] → the panel is inside a scrollable dialog; a drag
  near its edge does not auto-scroll. Accepted: ten groups fit.
- [Two doors, one panel] → the panel never knows which door it is in; the dialog's Done and
  the page's nav are the doors' own.
