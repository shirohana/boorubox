> Two units in sequence, Sonnet each (retried on Opus if the gate fails): unit P (the pointer
> primitive and the Stamps table on it), then unit Q (the panel, its two doors, the label
> menu, the rename) after P has landed and after `tag-indicator-order` has landed (it edits
> `Inspector.svelte`'s tag list and `PinnedDot`; Q edits `Inspector.svelte`'s pinned
> snippets). Design D1–D10 decide every shape; do not re-decide them. One Opus review of the
> whole change. Gates: `pnpm lint && pnpm typecheck && pnpm --filter @boorubox/app test`
> per unit. No unit commits or ticks a hand check. The lead smokes the drag with a pointer
> drag helper afterwards; a jsdom drag test is wiring, not proof.

## 1. Unit P — the pointer primitive (`packages/app/src/lib/components/common`, `api/drag-drop.ts`)

- [x] 1.1 `common/pointer-drag.ts`: `pointerDrag(handle, handlers)` per D1 with a header
      comment saying why pointer events and not HTML5 drag (two lines, the rule, naming
      `api/drag-drop.ts`); `pointer-drag.test.ts` (jsdom): a move under 4px never starts; a
      move past it starts and reports coordinates; `pointerup` ends with `dropped: true`;
      `pointercancel` and Escape end with `dropped: false`; capture is taken and released.
- [x] 1.2 `common/reorder.ts`: `reorderable` rebuilt on `pointerDrag` per D1, same contract
      (`data-reorder-index`, `[data-reorder-handle]`, `data-reorder-drop`, `onmove(from,
      to)`); the HTML5 handlers removed; the header comment rewritten to the rule. `ReorderHandle.svelte`:
      `draggable` gone, `touch-action-none`. `reorder.svelte.test.ts` rewritten to drive
      pointer events with `setPointerCapture`/`releasePointerCapture`/`elementFromPoint`
      stubbed, asserting the marks and `onmove` at both ends of the list. `moveItem`,
      `dropIndex` and their tests unchanged.
- [x] 1.3 `api/drag-drop.ts` header: one line pointing at `pointer-drag.ts` per D10.
      `StampsTable.svelte` needs no change — confirm by reading it and by the gate.
- [x] 1.4 Gate green. Handoff: the primitive's exact signature and the stubs a consumer's
      test needs.
- [ ] 1.5 Hand check (owner): on Settings → Stamps drag a stamp by its handle; the drop
      marker follows the pointer and the row lands where it was dropped, on macOS and on
      Windows.
      Seen (lead's smoke 2026-10-02): on Settings → Stamps a pointer drag of the first row's handle two rows down landed it third, in the table and the database (boorubox-vault/smoke-2026-10-01/r2-13-stamp-drag.png). Windows is the owner's.

### Handoff

- Signature: `pointerDrag(root: HTMLElement, handlers: { onstart?(x, y, handle), onmove?(x, y),
  onend?(x, y, dropped) }, options?: { selector?: string }) => { update(handlers), destroy() }`,
  exported with `THRESHOLD = 4` from `common/pointer-drag.ts`. Without `selector` the press
  arms on `root`; with it, only on a press inside a match (delegation), and `handle` is that
  match. `onend` fires once, only after a start; `dropped` is true for pointerup alone.
- It ends on pointerup/pointercancel/Escape itself and calls `releasePointerCapture`;
  `lostpointercapture` (blur) is the backstop. Resolve the target with
  `document.elementFromPoint(x, y)?.closest(...)`.
- jsdom stubs a consumer test needs: `setPointerCapture` and `releasePointerCapture` as `vi.fn()`
  on the element that gets captured (the matched handle, or `root`); `document.elementFromPoint`
  assigned a function returning the element the test says is under the pointer. Events are plain
  `new Event(type, { bubbles: true })` with `clientX`, `clientY`, `pointerId: 1`, `button: 0`
  assigned (jsdom has no `PointerEvent`); move more than 4px after pointerdown to start.
  `hasPointerCapture` need not exist. Escape is a `keydown` on `document`.
- Deviation: the handle uses Tailwind's `touch-none` (touch-action: none), the same class
  `SectionResizer` uses. `StampsTable.svelte` untouched; gate green.

## 2. Unit Q — the panel and its doors (`packages/app/src/lib/components/tags`, `library/Inspector.svelte`, `settings`, `routes/settings`)

- [x] 2.1 `tags/PinnedTagsPanel.svelte` per D2–D7: the body of today's
      `PinnedGroupsDialog.svelte` moved here, groups by header buttons (D3), tag rows with
      handle, checkbox, name button, Unpin (D5, D6), tag drag between groups through
      `pointerDrag` with the section marking and the drop rules (D4), the "+" and the pin
      field (D7), the bottom bar with "Move N selected to…" and "New group…". Header comment
      per D2.
- [x] 2.2 `tags/PinnedTagsDialog.svelte` (renamed from `PinnedGroupsDialog.svelte`, `git mv`)
      per D2: the shell only. `routes/settings/pinned-tags/+page.svelte` and `SETTINGS_PAGES`
      per D2; `settings-pages.test.ts` asserts Pinned tags follows Stamps.
- [x] 2.3 `tags/TagVocabularyMenuItems.svelte`: "Manage pinned tags…" per D8.
      `library/Inspector.svelte` `pinnedTagRows`: the label's context menu per D9, and the
      dialog import renamed; `TagSidebar.svelte`: the import renamed.
- [x] 2.4 `tags/PinnedTagsPanel.svelte.test.ts` (renamed from the dialog's test, mocked IPC,
      the real store): the existing cases kept; a right-click on a name does not tick (D5);
      Unpin on a row calls `set_tag_pinned_group` with `unpin` (D6); the "+" opens the field,
      Enter calls `set_tag_pinned_group` with `{ group: n }` and keeps the field, a refusal
      shows (D7); a drag of a ticked tag onto another section calls `move_pinned_tags` with
      every ticked name (D4, pointer events with the unit-P stubs); Move up/down buttons
      disabled at the ends (D3).
- [x] 2.5 Gate green. Handoff: anything the design did not foresee, and what the reviewer
      should read first.
- [ ] 2.6 Hand check (owner): right-click a tag row — the menu opens, the box is unchanged;
      drag a tag's handle onto another group — the group rings under the pointer and the tag
      lands there; tick three and drag one — all three move; Unpin at the row's end removes
      the chip from the strip; "+" on a group, type a tag, Enter — it is pinned there and the
      field stays; a refused name shows its reason; right-click a group label in the strip
      — "Manage pinned tags…"; Settings → Pinned tags shows the same panel after Stamps.
      Seen (lead's smoke 2026-10-02): right-click on `arknights` opened a menu with Unpin and the box stayed clear (boorubox-vault/smoke-2026-10-01/r2-07-row-menu.png); dragging `tag_later`'s handle onto Games moved it there (boorubox-vault/smoke-2026-10-01/r2-06-dragged.png); ticking `arknights` and `genshin_impact` and dragging one moved both and cleared the ticks (boorubox-vault/smoke-2026-10-01/r2-08-ticked-drag.png); Unpin on a row removed `genshin_impact` (boorubox-vault/smoke-2026-10-01/r2-10-unpinned.png); “+” on Games, `kani_biimu`, Enter pinned it there with the field still open (boorubox-vault/smoke-2026-10-01/r2-09-pinned-from-field.png); right-click on the strip's `#2` label offered Manage pinned tags… (boorubox-vault/smoke-2026-10-01/r2-04-label-menu.png) and opened the Pinned tags dialog (boorubox-vault/smoke-2026-10-01/r2-05-panel.png); Settings → Pinned tags sits after Stamps and shows the same panel (boorubox-vault/smoke-2026-10-01/r2-11-settings.png, boorubox-vault/smoke-2026-10-01/r2-12-page.png). Not driven: a refused name, the ring under the pointer (not captured mid-drag), a drop on New group…, the viewer. Nit seen: the right-click left the row's name text selected (boorubox-vault/smoke-2026-10-01/r2-07-row-menu.png).

### Handoff (Unit Q)

- Landed: `PinnedTagsPanel.svelte` (new; D2-D7), `PinnedTagsDialog.svelte` (git mv, shell only), the panel test (git mv, 959 -> 960 total with the Inspector label test), `routes/settings/pinned-tags/+page.svelte`, `SETTINGS_PAGES` + test, `TagVocabularyMenuItems` ("Manage pinned tags…"), `Inspector.svelte` (label `ContextMenu` per D9, import renamed), `TagSidebar.svelte` import. Gate green: 79 files, 960 tests.
- Deviations: (1) `TagInput` completes a typed word with a trailing space on the first Enter and only submits on the second, so the pin field treats a trailing space in `oninput` as the confirmation (one Enter pins; typing a space or accepting a suggestion pins too). (2) The drag handle is `ReorderHandle` inside a `span[data-tag-handle]`, since `common/` is not Q's to edit. (3) A group's context menu exists only on an empty group's header (Delete group); a non-empty group has no menu, so no empty menu opens. (4) The panel takes `portalTo` only; reset-on-open is by mount (Dialog content unmounts when closed), so the old `$effect` on `open` is gone.
- Panel test needs: `tag_suggestions` answered `[]` (the pin field's TagInput asks) and kept out of `calls`.
- Read first: `PinnedTagsPanel.svelte` `dropDrag`/`targetAt`/`tagDrag` (drop rules, D4), then `pinTyped`/`onAddInput` (deviation 1).

- Review fixes applied: 1-8. Escape in the pin field stops at the field's wrapper (1); every panel write goes through `write`, which clears the position-keyed state when the group count changed, and a pin re-opens the field in the group the tag landed in (2); `closeAdd` clears `addError` with `adding` (3); the trailing-space trigger is gone: Enter, after `TagInput`'s own handling, pins what the field holds, one pin at a time, text reset only if unchanged; Tab on a highlighted suggestion and a clicked suggestion only fill the field; `vocabulary.place` answers whether it landed (4); `pointerDrag` listens on `window` in capture, stops Escape once started, and ends at the last move (5); spec and `settings-pages.ts` cite 2026-10-02 (6); new panel tests for the New group… drop, a drop outside, compaction, Escape; `elementFromPoint` restored (7); `placeAndUntick` (8). This supersedes deviation (1) above.
