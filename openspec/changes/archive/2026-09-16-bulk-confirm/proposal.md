## Why

The owner's Windows pass (2026-09-15): "if the user select-all and pressed any `Rating:G` button
accidentally, there's no way for recovering". The selection toolbar's rating control writes
the whole selection on one click, with no question, and the app has no undo. Moving two or more
images to the trash already asks first, on exactly this argument (`trash` design D12, amended:
`Cmd A` is one keystroke away from the whole library, and the count is the fact the user is
missing at that moment). A rating overwrite is worse than a trash move — the trash move can be
restored, the old ratings are gone. §6 Phase 2 makes bulk ops a parity feature; parity with a
tool that can silently re-rate a library is not the reference.

**Depends on:** `selection-and-bulk` (the toolbar and `bulk_set_rating`), `trash` (the one
confirmation dialog and the pending-write shape), both archived and implemented. Lands after
`inspector-polish`, which touches `LibraryScreen.svelte` too.

## What Changes

- **Rating two or more selected images asks first**, naming how many and which rating (or
  "clear"), through the same dialog the trash uses. Confirming applies the whole rating write
  as today; dismissing writes nothing and keeps the selection. One selected image is rated
  without a question, as one image is trashed without one: the rule is the same, and lives in
  one function.
- **The bulk tag editor is unchanged.** It is already a dialog with an explicit button; its
  "Add" and "Remove" are typed, not one click away.

## Non-goals

- Undo. This is the cheap fence; undo is the real fix and stays a raised follow-up.
- Confirming single-image writes from the inspector or the tile menu.
- Any other bulk action's confirmation: trash already asks; export and restore are harmless.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `bulk-operations`: "A rating is set across the selection" gains the confirmation for two or
  more images.

## Impact

- `packages/app/src/lib/components/library/`: the pending-write machinery (`PendingWrite`,
  the count rule, `confirmPrompt`) moves out of `trash-actions.ts` into a module named for what
  it is, since a rating write joins it; `SelectionToolbar.svelte` stops writing the rating
  itself and hands `(ids, rating)` to the screen; `LibraryScreen.svelte` asks or writes;
  `ConfirmDialog.svelte`'s header comment records the sixth question and why it clears the bar.
- No Rust, schema or sidecar change.
