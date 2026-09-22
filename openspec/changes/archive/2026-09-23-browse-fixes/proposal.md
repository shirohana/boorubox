## Why

Four defects the owner met while tagging (2026-09-23), each in a shipped behaviour that the
specs already describe correctly: a bulk edit leaves images in the selection that the result no
longer shows; a plain click followed by a modifier-click selects only the second image; the
viewer's inspector cannot open its collection menu; and a click on the inspector's text or empty
space leaves the grid's keys dead. Requirements §6 (browse, lightbox, bulk ops, keyboard nav).

## What Changes

- **The selection keeps only what the result still shows.** After every write that re-reads
  the search (bulk tags, bulk rating, a collection changed from the sidebar, trash and restore),
  ids the search no longer matches leave the selection, so the count, the thumbnail strip and
  the next bulk action all describe images on screen. Today only trash prunes, and by the ids it
  wrote.
- **A modifier-click picks up the card the user was standing on.** With nothing selected, a
  multi-select click on a second thumbnail selects both it and the thumbnail last clicked or
  arrowed to, the way a shift-click already includes both ends. The per-tile checkbox keeps
  selecting only its own image. A plain click still selects nothing.
- **Every overlay the inspector opens works inside the viewer.** The add-to-collection menu,
  the two context menus, the rename dialog and the upload dialog are drawn inside the viewer's
  own layer, as the tag suggestion list already is; today they open underneath it, unreachable.
- **A click that lands on nothing hands the keys back to the grid.** Clicking the inspector's
  title, address, empty space, or any other surface of the library screen that takes no focus of
  its own, leaves the grid's current card focused, so the arrows, Space and `i` keep working.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `selection`: the "Toggling one image" scenario gains the pickup of the anchored card; the
  "After a bulk edit" scenario is restated so that ids leaving the result leave the selection
  on every write that re-reads the search.
- `app-frame`: the keyboard map's hand-back paragraph widens from "a completed action in the
  inspector" to any click on the screen that leaves no control focused.
- `collections`: the "every one of those menus SHALL stay within the window" sentence gains
  "and SHALL open inside the full-size viewer when the inspector is shown there"; one scenario.

## Non-goals

- Undo. A bulk edit that filtered images out is not reverted; the selection just stops
  describing them.
- A plain click that selects (the legacy viewer's behaviour): `selection-and-bulk` D1 stands,
  and this change keeps its argument (see design D2).
- A screen-wide key dispatcher for the arrows: `app-shell` D14 rules it out; the fix hands the
  focus back instead.
- The single-image edit paths (`replace`, `replaceMany`), which deliberately keep an edited
  image on screen until the next search (`tags-and-ratings` D10). Only writes that re-read the
  search prune.

## Impact

- Rust: `query.rs` (`matching_ids` on the same plan `search_ids` uses), `commands.rs`,
  `lib.rs`.
- Webview: `api/commands.ts`, `api/selection.svelte.ts` (`keepMatching`, the anchor pickup, a
  `toggle` entry for the checkbox), `LibraryScreen.svelte` (one after-write path), `BulkTagDialog.svelte`,
  `ImageCard.svelte`, `LibraryGrid.svelte`, `Inspector.svelte`, `TagInput.svelte`,
  `CollectionNameDialog.svelte`, `UploadAction.svelte`, `UploadDialog.svelte`, two new pure
  modules (`lib/portal.ts`, `library/focus-handback.ts`) with tests.
- No schema, sidecar or settings change.
