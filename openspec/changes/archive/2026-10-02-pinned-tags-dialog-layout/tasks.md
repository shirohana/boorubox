> Two units in parallel: unit L (the dialog and panel layout), Sonnet; unit C (the chip row),
> the lead, with the app on screen. Design D1–D3 decide every shape. Gate: `pnpm lint && pnpm
> typecheck && pnpm --filter @boorubox/app test`. No unit commits or ticks a hand check.

## 1. Unit L — the panel's scroll region and bar (`components/tags/PinnedTagsPanel.svelte`, `PinnedTagsDialog.svelte`, `routes/settings/pinned-tags/+page.svelte`)

- [x] 1.1 `PinnedTagsPanel.svelte` per D2: root `flex min-h-0 flex-col`, groups region
      `min-h-0 flex-1 overflow-y-auto pr-1`, the bar sticky with the page background,
      `barEnd?: Snippet` rendered after "New group…". The header comment says the host bounds
      the panel and why the bar is sticky.
- [x] 1.2 `PinnedTagsDialog.svelte` per D3: `Dialog.Content` is a bounded flex column, the
      panel `min-h-0 flex-1`, Done passed through `barEnd`, no `Dialog.Footer`.
- [x] 1.3 `PinnedTagsPanel.svelte.test.ts`: `barEnd` renders inside the bar; the bar is a
      sibling of the scroll region, not inside it.
- [x] 1.4 Gate green. Handoff: anything the design did not foresee.
- [ ] 1.5 Hand check (owner): with more groups than fit, tick a tag — "Move N selected to…"
      is at the dialog's foot with Done, the groups scroll under the title, the scrollbar
      runs inside the box; on Settings → Pinned tags the bar sticks to the page bottom.
      Seen (lead, on screen 2026-10-02, window 560px tall): the dialog's title stays, the groups scroll under it with the scrollbar inside the box, the bar holds New group… and Done (boorubox-vault/smoke-2026-10-01/r3-07.png); ticking `daran_9` put “Move 1 selected to…” in the bar (boorubox-vault/smoke-2026-10-01/r3-08-ticked.png); Settings → Pinned tags scrolled to its end shows the bar at the page bottom (boorubox-vault/smoke-2026-10-01/r3-09-page.png). Two fixes the unit's gate could not see, both recorded in design D2/D3: `Dialog.Content` is the kit's grid, so the bound is a row template, not `flex-col`; the bar is sticky only on the page, since WebKit misplaces a sticky box inside the dialog's transform (boorubox-vault/smoke-2026-10-01/r3-05-dialog.png shows the misplaced bar).

## 2. Unit C — the chip row and the sidebar dot (`components/library/Inspector.svelte`, `components/tags/FilterRow.svelte`)

- [x] 2.1 `class="flex"` on each chip's `li` per D1; checked on screen beside an unnoted chip.
- [x] 2.2 `FilterRow.svelte` per D4: the dot inside the name button after a `truncate` text
      span; `FilterRow.svelte.test.ts` asserts the dot is inside the name button and the glyph
      and count follow it. Checked on screen.
- [ ] 2.3 Hand check (owner): a noted pinned chip and an unnoted one share a line; a sidebar
      row reads name, dot right after it, the gap, then glyph and count at the end.
      Seen (lead, on screen 2026-10-02): `daran_9` with its note glyph and `blue_archive` share one line in the strip (boorubox-vault/smoke-2026-10-01/r3-02-strip3x.png, enlarged; boorubox-vault/smoke-2026-10-01/r3-01-strip3x.png is before, with the lifted chip); the sidebar row reads `daran_9`, dot, the gap, glyph, count (boorubox-vault/smoke-2026-10-01/r3-01.png).

## Handoff

Unit L landed (uncommitted): panel root `flex min-h-0 flex-1 flex-col`, groups region
`data-groups` (`min-h-0 flex-1 overflow-y-auto pr-1`), sticky `data-bar` with `barEnd`;
dialog is a flex column, Done via `barEnd`, no footer; test added. Deviation: the panel root
carries `flex-1` itself (no class prop, no wrapper div), harmless on the Settings page; the
Settings page needed no edit. Gate result: see the lead's report. Hand check 1.5 open.
