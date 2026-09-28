## Why

The inspector's facts use the app's storage vocabulary, not the reader's. "Source" names the
capture origin (`extension · x`, `local`), while everywhere else the user meets the word —
Danbooru, the upload dialog, the new `source:` filter — it means the page the image came
from, which the panel calls "Page". Three rows spell the file (Dimensions, Size, Type) and
three the time (Captured, Imported, File modified), for facts the spec itself says are seldom
read. Owner's ask, 2026-09-28, with the shapes settled in conversation: relabel to Danbooru's
vocabulary, fold the file rows into one line in Danbooru's form, fold the time rows into one
relative date with the absolute times on hover. Requirements §6 (the inspector shows an image's
facts); `app-frame` spec, "One inspector panel, two placements".

## What Changes

- **Origin** is the capture origin row (was Source). Values unchanged.
- **Source** is the page address (was Page), still editable in the facts form.
- **Source file** is the image address (was Image), still editable with it.
- **Size** is one row, `1.3 MB .jpg (1200×2200)`: bytes, the stored file's extension, pixel
  dimensions (was Dimensions, Size, Type).
- **Date** is one row showing how long ago the image was captured (`3 days ago`, `2 years
  ago`); hovering it shows the absolute captured, imported and file-modified times (was three
  rows of absolute times).
- ID stays last. The spec's list of facts and its "at least" sentence are amended.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `app-frame`: the names and shape of the inspector's facts rows.

## Non-goals

- Rewording the origin values (`extension · x`, `local`, `legacy-bundle · <id>`).
- A live-ticking relative time: the row is computed when the image is shown, as Danbooru's is.
- Changing which facts are editable or how the form works.
- Showing `sourceUrl` in the Source row: the row is the page address, editable; the filter's
  fallback to the image address is the filter's, and the Source file row shows that address.
