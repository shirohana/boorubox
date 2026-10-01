> One unit, Sonnet, retried on Opus if the gate fails; the lead checks both on screen. Design
> D1–D3 decide every shape. Owns `components/tags/PinnedTagsPanel.svelte` + its test and
> `PinnedTagsDialog.svelte`. Gate: `pnpm lint && pnpm typecheck && pnpm --filter
> @boorubox/app test`. Does not commit; does not tick the hand check.

## 1. Unit F — focus and edge scroll (`packages/app/src/lib/components/tags`)

- [ ] 1.1 `PinnedTagsPanel.svelte`: `tabindex="-1"` on `[data-groups]`; `write()` keeps or
      rests the focus per D1, with a comment saying why (the kit's focus scope would pick the
      first field). `PinnedTagsPanel.svelte.test.ts`: after a mocked Unpin removes the pressed
      button, `document.activeElement` is the groups region; after a Move down that keeps the
      button, it is still the button.
- [ ] 1.2 `PinnedTagsDialog.svelte`: `onOpenAutoFocus` per D2. Test in the panel's test file if
      the dialog can be mounted there, else a `PinnedTagsDialog.svelte.test.ts`: on open the
      active element is the groups region, not an input.
- [ ] 1.3 `PinnedTagsPanel.svelte`: edge auto-scroll per D3 in `tagDrag` (`onmove` records
      the pointer and arms the frame loop; `onend` stops it), the section under the pointer
      re-resolved after each step. Test: with the region's rect and `scrollTop` stubbed, a
      move inside the bottom band advances `scrollTop` on the next frame (`requestAnimationFrame`
      stubbed to run once), a move outside it does not, and `onend` stops the loop.
- [ ] 1.4 Gate green. Handoff: anything the design did not foresee.
- [ ] 1.5 Hand check (owner): open the dialog — no field is focused, Escape closes it; press
      Unpin and Move up — the focus never lands in a name field; with a group taller than
      the dialog, drag a tag to the bottom edge — the groups scroll, the target rings, the
      drop lands.
      Seen (lead, on screen 2026-10-02, window 560px tall): opening the dialog focused no field (boorubox-vault/smoke-2026-10-01/r4-01-open.png); Unpin on `daran_9` left the focus on the Unpin control that took its row's place, and Move down on group #1 on that arrow button — no name field either time (boorubox-vault/smoke-2026-10-01/r4-02-unpin.png, boorubox-vault/smoke-2026-10-01/r4-03-movedown.png); dragging `blue_archive` to the bottom band and holding for 2.5 s scrolled Games under the pointer and the release pinned it there, in the database (boorubox-vault/smoke-2026-10-01/r4-04-edge-drag.png). Escape on open was not pressed.

