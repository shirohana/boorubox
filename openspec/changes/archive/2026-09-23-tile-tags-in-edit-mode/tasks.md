> One unit, agent F (Sonnet, `packages/app` webview only), running beside agent E
> (`TagSidebar`, `Inspector`, `categories.ts`, the tag-panel-polish archive and the
> tag-sidebar/tag-editing/tag-vocabulary main specs) — do not touch their files. Design
> D1–D2 decide every shape; do not re-decide them. Gate: `mise run check` green. Do not
> commit; do not tick a hand check.

## 1. Unit F — the footer and the row height (`packages/app`)

- [x] 1.1 `grid-window.ts`: `TAG_FOOTER` exported beside `GAP`; `gridWindow` takes
      `footer` and `rowHeight = tile + footer + GAP` (D2); `grid-window.test.ts` covers a
      footer of 0 and of `TAG_FOOTER` (row count, `imageTop`, list height). Verify:
      `pnpm --filter @boorubox/app test grid-window` passes.
- [x] 1.2 `ImageCard.svelte`: `showTags` and the footer per D1 (the trigger a column; the
      square wrapper keeps the image, the caption strip and the stamp overlay; the strip
      under it with `groupByCategory` + `CATEGORY_TEXT_CLASS`, "No tags" when empty); the
      tile's own comment about facts covering the image gains the footer's exception.
      `LibraryGrid.svelte`: `showTags` prop → `footer` for `gridWindow` and `showTags` on
      every card; `LibraryScreen.svelte`: `showTags={editMode}`. Verify: `mise run check`
      green.
      Hand check: press `E` — every tile grows a strip of coloured tags under the image,
      rows still align, scrolling stays exact at the bottom of a long result; click a tile
      with Cat active — its strip gains `cat animal` at once; `E` again — strips gone, rows
      back to square.
      Seen by the lead on the seeded scratch vault (smoke run 2026-09-23): footers under every
      tile, rows aligned, "No tags" where empty, the focus ring ending above the footer, the last
      row fully visible at the bottom; Cat then a click added `animal cat` to that footer at
      once; E removed the footers. The tags ran together with no space — fixed after the run
      (`{' '}` between spans), not re-seen. Right-click on a footer opened the tile menu; a plain
      click did nothing.

## 2. Unit G — the footer as a view setting (`packages/app`), folded into this change

> Owner, 2026-09-23, from the running app: the footer is useful outside edit mode too — a
> toggle for normal browsing, always on in edit mode. No key (the owner's rule: keys go to
> the most frequent day-to-day actions, after a discussion). Agent G (Sonnet), alone in the
> tree. Gate: `mise run check` green. Do not commit; do not tick a hand check. Folded into
> this change's commit: proposal, design and both spec copies (archived delta and
> `openspec/specs/stamps/spec.md`) are amended to the shipped shape, the non-goal reversed
> with the reason.

- [x] 2.1 The setting: `showTileTags: boolean` (default `false`) beside `gridTileSize` —
      `settings.rs` (key, default, setter), `commands.rs` + `lib.rs` (`set_show_tile_tags`,
      registered, a command test copied from the tile-size one), `packages/shared`
      `Settings.showTileTags`, `api/commands.ts` `setShowTileTags` with its invoke-shape
      test, `api/settings.svelte.ts` `setShowTileTags`. Verify: `cargo test settings::
      commands::` pass, `pnpm --filter @boorubox/app test commands settings` pass.
- [x] 2.2 `LibraryScreen.svelte`: `showTags` is `$derived(editMode || (settings.current?.
      showTileTags ?? false))` — derived, never an effect off the store object — and reaches
      the grid as before. In the toolbar's right-edge group, between the thumbnail-size
      `Slider` and the inspector button: a `Button size="icon-sm"` with the lucide `tags`
      icon, `variant` `secondary` while `showTags` else `ghost`, `aria-pressed={showTags}`,
      `aria-label="Show tags under thumbnails"`, `disabled={editMode}` with `title` "Tags
      under thumbnails" normally and "Tags under thumbnails (always on in edit mode)" while
      disabled; the click calls `settings.setShowTileTags(!current)` and surfaces a failure
      through `actionError` as the slider does. The inspector button's own comment ("the
      variant, not just aria-pressed") applies; write the toggle's comment in that voice.
      Verify: `mise run check` green.
      Hand check: the tags button beside the slider is plain; click — footers appear on every
      tile and the button fills; restart the app — still on; enter edit mode — the button is
      filled and disabled, footers stay; leave — back to the setting; click again — footers
      gone.
      Seen by the lead on the seeded scratch vault (smoke run 2026-09-23, third pass): the
      button plain between the slider and the inspector button; a click filled it and every
      tile grew a footer, tags separated by spaces; after Cmd+Q and a relaunch the button was
      still filled and `showTileTags: true` was in settings.json; in edit mode it read filled
      and disabled with the native tooltip "Tags under thumbnails (always on in edit mode)"
      showing on hover; leaving restored it; off again removed the footers, and edit mode
      forced them back with the setting off.
- [x] 2.3 Docs: `proposal.md` — the non-goal "A footer outside edit mode, or a view toggle
      for one" is removed and "What Changes" gains the toggle, with the reason (owner found
      the footer useful for browsing, 2026-09-23); `design.md` gains D3 (the setting lives
      in settings.json beside the tile size because it is a display preference of this
      machine, not of the library; the button sits with the grid's display controls; edit
      mode forces it on and disables the button rather than hiding it, so the state stays
      readable); the stamps requirement in both spec copies: "the footer SHALL be absent
      outside the mode" becomes "outside the mode the footer follows a view setting the
      toolbar toggles, kept with the thumbnail size; in the mode it is always shown", plus a
      scenario "A footer while browsing". Verify: `openspec validate --specs` passes.

- [x] 2.4 Three lines and the whole list on hover (owner, 2026-09-23, after seeing a
      heavily tagged image cut short): `TAG_FOOTER` = 56, `line-clamp-3`, the strip's box
      absolute inside the fixed placeholder and expanded over the row below while hovered and
      overflowing (`markOverflow`). Verify: `mise run check` green.
      Hand check: an image with a dozen tags shows three lines with an ellipsis; hovering it
      shows every tag over the tile below on a background with a shadow; a tile with two tags
      shows no change on hover; leaving the hover collapses it; after a stamp adds tags the
      expansion follows.

## Handoff

Landed, both 1.1 and 1.2, `mise run check` green (includes lint, typecheck,
`pnpm -r test` — 54/54 app test files incl. `grid-window` with the 4 new footer
tests, clippy, and the Rust + web builds).

- `grid-window.ts`: `TAG_FOOTER = 40` exported beside `GAP`. `Viewport` gained
  `footer: number`; `gridWindow`'s `row` is now `columnWidth + footer + GAP`
  (the actual computed square edge, not the raw `tile` prop — the design's
  `tile + footer + GAP` is the concept, the code already derived the tile's
  real edge from the measured width before this change and still does).
  `sectionsOf`/`imageTop` needed no change, as D2 said: they already take
  `rowHeight` as a parameter.
- `grid-window.test.ts`: the shared `viewport` fixture now carries `footer:
  0`, so every existing test is already the footer-off case; one inline
  viewport literal (`narrow`, the column-dropping test) got the same field.
  New `describe('footer', …)` block: rowHeight/columns with footer on vs.
  off, `contentHeight` scaling, `imageTop` of the second row, and mounted
  tile-row count not growing for a taller row in the same viewport height.
- `ImageCard.svelte`: `ContextMenu.Trigger` is now `flex flex-col` instead of
  `aspect-square` itself; a new inner `<div class="relative aspect-square">`
  wraps everything that used to sit directly in the trigger — the three
  branches (loading / missing-file / the image button) and the trash+checkbox
  overlay span, which had to move inside it since its `absolute top-1
  right-1` positioning was relative to that square, not the taller column.
  `group/tile` stayed on the outer trigger; Tailwind's named-group selectors
  reach descendants at any depth, so `group-hover/tile:opacity-100` on the
  moved span still works unchanged. The footer is a sibling after the square,
  `showTags`-gated, fixed at `TAG_FOOTER` px via inline `style` (not a
  Tailwind height class) so it can never drift from the row math's constant.
  Tags: `groupByCategory(image.tags, (tag) => tag, vocabulary.categoryOf)`,
  same call shape as `Inspector.svelte`'s. "No tags" only once `image` has
  loaded and the group list is empty; blank (no text) while `image` is still
  `undefined`, so a loading row never flashes a wrong "No tags" before its
  page arrives. Tag-name spans carry a trailing literal space (`{name} `)
  inside the span rather than a `{' '}` mustache between them — ESLint's
  `svelte/no-useless-mustaches` rejects a mustache holding only a string
  literal, and a bare space *between* two `{#each}`-repeated inline elements
  risks Svelte trimming it as block-boundary whitespace; inside the span, on
  the same line, it is unambiguous.
- `LibraryGrid.svelte`: `showTags?: boolean` (default `false`), threaded into
  `gridWindow`'s `footer` and forwarded to every `ImageCard`.
- `LibraryScreen.svelte`: `showTags={editMode}` on `<LibraryGrid>` — the mode
  itself, same as the design's reasoning for `stampLabel` not gating on it.

Deviations from the task text: none in shape: D1/D2 were followed as
written. The one place the task's own formula (`tile + footer + GAP`) doesn't
literally appear in the code is `gridWindow` itself, where `tile` becomes
`columnWidth` (the tile's real, measured edge) — that indirection already
existed before this change (see `grid-window.ts`'s own comment on
`rowHeight`), so D2 is honoured in spirit and by the row's actual height,
not word-for-word in the source.

Not done, left for whoever owns it: the `Hand check:` under 1.2 (press `E` in
the running app and confirm the strip, the row alignment, the live stamp
update, and leaving the mode) — boundaries for this run excluded running the
dev server or the app.

Nothing owed to agent E: no file outside my list (`ImageCard.svelte`,
`LibraryGrid.svelte`, `grid-window.ts`, `grid-window.test.ts`,
`LibraryScreen.svelte`, `openspec/changes/tile-tags-in-edit-mode/**`) was
touched. `mise run format` (`eslint . --fix`, repo-wide) ran once to fix this
change's own template indentation; it may also have reformatted files E was
mid-edit on (`Inspector.svelte`, `TagSidebar.svelte`,
`components/tags/categories.ts`/`.test.ts`) — those diffs were already
substantial (real content, not just formatting) when checked, so nothing
looked clobbered, but worth E's own eye before trusting whitespace there.

### Unit G

Landed, 2.1–2.3, `mise run check` green (lint, typecheck, `pnpm -r test` — app
54/54 test files incl. the new `set_show_tile_tags`/`setShowTileTags` cases,
`cargo test` — 659 passed including the new `settings::`/`commands::` ones,
clippy, and the Rust + web builds) and `openspec validate --specs` passes
(29/29, `spec/stamps` included).

- `showTileTags: boolean` (default `false`), kept beside `gridTileSize`
  end to end: `settings.rs` (`SHOW_TILE_TAGS` key, `Settings.show_tile_tags`,
  `load_from`/`save_to`), `model.rs` (`AppSettings.show_tile_tags`),
  `commands.rs` (`set_show_tile_tags`, unclamped — a bare flag has no
  out-of-range value the way the tile size does), registered in `lib.rs`.
  New Rust tests: `settings::tests::a_file_without_the_show_tile_tags_key_
  reads_as_off` (absent-key default, same shape as the notes/collections
  ones) and `commands::tests::show_tile_tags_reaches_the_state_and_the_store`
  (the tile-size test's round trip, minus the clamp assertions it has
  nothing to prove). `packages/shared`'s `AppSettings.showTileTags`,
  `api/commands.ts`'s `setShowTileTags`, `api/settings.svelte.ts`'s
  `setShowTileTags` all follow the existing per-field pattern exactly
  (`setNotesCollapsed`'s shape). Every `AppSettings` object literal in
  `commands.test.ts` and `settings.svelte.test.ts` gained the field
  (compiler-forced, not optional); both files also gained a case exercising
  the new command.
- `LibraryScreen.svelte`: `showTags` is now `$derived(editMode ||
  (settings.current?.showTileTags ?? false))`, placed beside the existing
  `clickZoomCeiling` derivation and reusing its stated reason (a `$derived`
  of `settings.current`, not an `$effect` on it, so a change to one setting
  field doesn't re-run this on every unrelated write) rather than repeating
  the comment. The grid still receives `showTags` as before, now via
  shorthand (`{showTags}`) since it is no longer just `editMode` inline. A
  new `Button size="icon-sm"` sits between the thumbnail-size `Slider` and
  the inspector toggle: `tags` lucide icon, `variant` follows `showTags`,
  `aria-pressed={showTags}`, `disabled={editMode}`, `title` switches wording
  while disabled, `onclick` calls `settings.setShowTileTags(!showTags)` and
  routes a rejection through `actionError` like the slider's `onValueCommit`
  does. Passing `!showTags` rather than negating `settings.current?.
  showTileTags` directly is safe and simpler: the button is disabled
  whenever `editMode` makes the two diverge, so while it's clickable
  `showTags` already equals the setting.
- Docs folded into this change's own history: `proposal.md`'s non-goal
  removed, "What Changes" gained the toggle with the 2026-09-23 owner
  finding as the reason, `Impact` lists the new Rust/shared/API files.
  `design.md` gained D3, recording why "a footer outside edit mode" was
  right until the shipped footer turned out to answer browsing too, and why
  the button is disabled rather than hidden in edit mode (so "on because of
  the mode" stays visually distinct from "on because of the setting"). Both
  spec copies (archived delta and `openspec/specs/stamps/spec.md`, kept in
  lockstep): the requirement's footer sentence now reads "outside the mode
  the footer follows a view setting the toolbar toggles, kept with the
  thumbnail size; in the mode it is always shown"; the closing sentence
  ("Leaving the mode SHALL … remove the footers …") is reworded too — it
  would otherwise contradict the new setting, since leaving the mode no
  longer always removes the footer. "Leaving takes the footers" is now
  qualified to the setting-off case for the same reason, and a new scenario
  "A footer while browsing" covers the toggle itself.

Deviations from the task text: the two spec amendments beyond what 2.3 named
verbatim — the requirement's closing sentence and the "Leaving takes the
footers" scenario, both of which said the footer always leaves with the
mode, which stopped being true once the view setting exists. Left unfixed
they'd contradict the sentence 2.3 did ask for, so both were reworded in the
same spirit (setting-off is the case they describe) rather than left stale.

Not done: the `Hand check:` under 2.2 (the tags button beside the slider,
its click, restart persistence, edit-mode disable) — boundaries for this run
excluded running the dev server or the app.
