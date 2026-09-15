> Depends on `selection-and-bulk` and `trash`, archived; lands after `inspector-polish` on
> `LibraryScreen.svelte`. One implementing agent; no Rust. Gate: `pnpm -r typecheck`, `pnpm
> lint`, `pnpm --filter @boorubox/app test`, then `mise run check`.

## 1. The pending write and its words (agent A)

- [x] 1.1 `packages/app/src/lib/components/library/pending-write.ts` (+ `pending-write.test.ts`):
      move `PendingWrite`, `confirmedCount`, `ConfirmPrompt`, `confirmPrompt` and the count rule
      out of `trash-actions.ts`, renaming `needsTrashConfirmation` to `needsConfirmation`
      (design D1); add `{ kind: 'rate', ids, rating }` and its prompt (design D3). Verify: the
      moved tests pass unchanged apart from the import; new tests cover the rate title for a
      rating and for `null`, the label, `destructive === true`, and `needsConfirmation(1) ===
      false` / `(2) === true`.

## 2. The toolbar and the screen (agent A)

- [ ] 2.1 `SelectionToolbar.svelte`: prop `rate(ids, rating)`, `bulkSetRating` import gone;
      `LibraryScreen.svelte`: implements it per design D2, `commit()` gains the `rate` branch,
      imports move to `pending-write`. Verify: typecheck, lint and tests pass.
      Hand check: select all, click `g` — the dialog names the count and "General"; Cancel —
      nothing changes and the toolbar still shows the count; confirm — every image is `g` and the
      selection stands; select one image, click `s` — no dialog, the image is `s`; select three,
      choose "none" — the dialog says "Clear the rating".
- [x] 2.2 `ConfirmDialog.svelte`: the header comment names the sixth question and its argument
      (design D4). Verify: lint passes; the comment reads as a rule for the next editor, not a
      changelog.

## 3. Change-level verification (owner)

- [ ] 3.1 `mise run check` green; the hand check above passes on Windows.

## Handoff

- `LibraryScreen.svelte` already had a `rate(image, rating)` function (the tile menu's own
  single-image write, predating this change). Design D2's screen-side name `rate` collides with
  it, so the screen-side implementation is named `rateSelection` instead; only the prop on
  `SelectionToolbar` — the public surface D2 actually specifies — is named `rate`, wired as
  `rate={rateSelection}`.
- `trash-actions.test.ts` is deleted rather than left empty: every test it held moved to
  `pending-write.test.ts` with the rest of the machinery task 1.1 names, and `trash-actions.ts`
  now holds only the `TrashActions` interface, which has no runtime behaviour left to test.
- D3's title capitalizes the rating name (`Sensitive`), but the single existing source of rating
  names, `ratingLabel` in `$lib/domain/format.ts`, returns lowercase for its other callers
  (tooltips). Rather than add a second rating-name table, `confirmPrompt` calls `ratingLabel` and
  capitalizes the result locally — one source of truth for the word, one place that reads it as
  a title.
