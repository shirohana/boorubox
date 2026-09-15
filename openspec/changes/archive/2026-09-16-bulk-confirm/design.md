## Context

See proposal.md — Why. Today `SelectionToolbar.rate()` calls `bulkSetRating(await
selection.ids(), rating)` itself and refreshes. The library screen owns the one `ConfirmDialog`
and the `pendingWrite` state behind it; `trash-actions.ts` holds `PendingWrite` (`trash` |
`delete` | `empty`), `needsTrashConfirmation(count) = count > 1`, `confirmedCount` and
`confirmPrompt`, with tests in `trash-actions.test.ts`. `ConfirmDialog.svelte`'s header lists
the five questions it asks and says the bar for a sixth is high: a confirmation on an act the
user can undo in one click teaches them to dismiss confirmations.

## Goals / Non-Goals

**Goals:** one rule for "how many is many", one dialog, one place that knows what a confirmed
rating write refreshes.

**Non-Goals:** a second dialog component; confirming from the inspector or tile menu.

## Decisions

### D1. The rating write joins the pending-write machinery, which moves to a module named for it

`PendingWrite` gains `{ kind: 'rate', ids, rating: Rating | null }`. `PendingWrite`,
`confirmedCount`, `ConfirmPrompt`, `confirmPrompt` and the count rule move from
`trash-actions.ts` to `pending-write.ts` (tests with them); `trash-actions.ts` keeps only
`TrashActions`. `needsTrashConfirmation` becomes `needsConfirmation` — the rule ("two or more")
is the same and the argument is the same, so it is one function; the trash caller and the new
rating caller both read it.

*Alternative rejected:* a rating-specific dialog in the toolbar. It would be the same
sentence and the same `open`/`onclose`/`onconfirm` shape a second time, and the toolbar would
then have to know what a rating write refreshes.

### D2. The toolbar asks the screen; the screen asks the user or writes

`SelectionToolbar` gets a prop `rate: (ids: string[], rating: Rating | null) => void` and stops
importing `bulkSetRating`. The screen's implementation: `needsConfirmation(ids.length)` →
`pendingWrite = { kind: 'rate', ids, rating }`, else write now. `commit()` gains the `rate`
branch: `bulkSetRating(ids, rating)` then `results.refresh()` — the same two calls the toolbar
made, now beside the trash write that already lives there. The selection is resolved to ids
before the question is asked (the toolbar's `apply` already does this for trash), so a range
selection survives the dialog.

### D3. The prompt's words

Title: `Set 1,234 images to Sensitive?` / `Clear the rating of 1,234 images?`. Description:
"Their current ratings are replaced. There is no undo." Confirm label: `Set rating` / `Clear
rating`. `destructive: true`: the dialog's own rule is that the red button is for what cannot
be undone, and this cannot. Rating names come from the one place that already spells them for
the rating control (grep `Sensitive` before writing).

### D4. `ConfirmDialog`'s header comment is amended, not appended

The comment enumerates the questions and argues the bar. It now names six, and the argument for
the sixth is the trash's own: not the act's severity but its scale with no way back. The
"bar is high" sentence stays — it is still the rule for the seventh.

## Risks / Trade-offs

- [A user rating a hundred images one selection at a time meets the dialog every time] → the
  threshold is the trash's, chosen once; if it grates, raise it in `needsConfirmation` for both.
- [The toolbar's `RatingControl` shows no current value, so a dismissed question leaves nothing
  to reset] → nothing to do; noted so nobody adds a reset.
