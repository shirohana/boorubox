## Why

With the results grouped by X account, the rail beside the grid lists the result's groups, so
including one account empties the rail of every other: the one control meant for moving
between accounts disappears on first use. The owner's ask (2026-09-17): the rail always shows
every account, the search's own first, the rest by how many they would give, zeros included.
Requirements §6 (the legacy viewer's account panel is the reference).

## What Changes

- The search counts gain a per-account list, filled while grouped by account: every account
  in the view, counted with the `account:` part of the search dropped and the rest honoured
  (the rating pills' rule), ordered largest first.
- The rail reads that list instead of the result's groups, marks the accounts the search
  names and lists them first, keeps zero-count accounts, and keeps its rows on screen while a
  search runs so its scroll position survives a click.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `sort-and-group`: the account list's membership, counts and order.

## Non-goals

- Changing the grid's own groups or their headings: those still describe the result as
  filtered.
- Counting accounts while not grouped by them.

## Impact

`packages/app/src-tauri/src/query.rs` and `model.rs` (a fourth field on the counts, one more
count query while grouped by account), `packages/shared/src/index.ts` (the type),
`AccountRail.svelte` and `LibraryScreen.svelte`. No schema change.
