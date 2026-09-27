## Why

A capture that lands while a range is selected moves the selection onto other images. The
owner's repro (2026-09-28): select the first three tiles by range (`*123*456`), a capture is
stored and prepended, and the selection reads `*012*3456` where `0*123*456` was meant. The
focused card, the inspector's image and the open viewer slide by one row the same way. The
selection holds rows, not images: a range and the focus are indices into the result order, and
the capture's refresh resolves the range to ids against a database that already holds the new
row. `selection-and-bulk` D4 ("ids, not indices, are what survives an action") holds for an
action, which resolves before it writes; a refresh nobody asked for arrives after the write.
Requirements §6 (bulk ops, keyboard nav; the webview owns UI only).

## What Changes

- **A refresh keeps the images, not the rows.** Before any re-read of the current search that
  can move rows — a capture stored, an import finished, a write's re-read, an artist renamed
  from the viewer — the screen records which images the focus, the anchor and the open viewer
  are on, read from the rows it has loaded; after the re-read each goes back to that image's new
  row. An image that left the result keeps today's behaviour (the index stays where it was).
- **A range is pinned from the loaded rows.** A range whose rows are all loaded becomes ids from
  those rows, with no round trip to the database. A range reaching past the loaded rows, on a
  capture, follows the capture's own row: shifted when the capture landed before it, left alone
  when after it, and resolved minus the capture when it landed inside it. Select-all stays a
  range across a capture.
- **Captures refresh one at a time.** Captures stored while a refresh is running are handled
  together by the next one, so no refresh reads rows another has cleared.
- **The viewer's own inspector** refreshes through the same path after an artist rename,
  instead of a bare re-read.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `selection`: a selection, the focused image and the range's anchor stay on their images when
  the result is re-read under them.
- `library-browse`: the open viewer stays on its image when the result is re-read under it.

## Non-goals

- Changing the grid's index contract: `selection.focus` stays a row index, and every reader
  of it (`LibraryGrid`, the inspector, the viewer) keeps reading an index.
- The flash of empty rows while a refresh reloads (`SearchResults.#start` clears the rows
  before page 0 answers): pre-existing, and not a drift.
- Scrolling the grid or moving the DOM focus on a capture: a capture is not the user's
  gesture, so the grid does not jump to follow the card it keeps.
- An exact range across an import that reached past the loaded rows: the import reports no
  ids to shift by. A FIXME names the shape (design D5).
- Keeping the anchor across a write: `afterWrite` re-anchors at the focus through
  `grid.focusCard`, as today.
