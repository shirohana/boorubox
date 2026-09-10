## Why

Editing tags and ratings one image at a time is the slow half of the job. The images that
arrive together — one artist, one thread, one import run — want the same tags, and the legacy
viewer's answer was a selection model with a bulk tag modal (docs/requirements.md §6: the
existing viewer is the reference for behaviours, "bulk ops" among them; §8 Phase 2 lists them
under Parity). `tags-and-ratings` gives the app a tag editor for one image; this change makes
it reach many.

The frame is already cut for it. `app-shell`'s slot map reserves "selection toolbar replacing
this row while a selection exists" in Toolbar · actions, "selection checkbox and selected ring"
in Grid · tile, and "thumbnails and count for a multi-selection" in Inspector · header, and its
design D9 says the focus index the grid already moves "is also what `selection-and-bulk`
extends into a selection model, so the interaction is built once".

The third piece is getting images back out. The legacy viewer's "dump selected" wrote a zip of
the chosen originals; it is the only way to hand a handful of pictures to someone without
copying the whole library folder (§6).

**Depends on:** `app-shell` (slot map, keyboard map, focus index, the grid and Inspector this
change extends) and `tags-and-ratings` (the tag input, the rating control, and the per-image
tag and rating commands whose bulk forms this change adds). It lands after both.

## What Changes

- **A selection model over the grid** (§6 "bulk ops"): Cmd/Ctrl+click toggles one card,
  Shift+click and Shift+arrows take a range from the anchor, Cmd/Ctrl+A takes the whole current
  result, Escape clears. A plain click still only focuses, exactly as `app-shell` shipped it, so
  looking at an image never starts a selection. The count is exact from the first keystroke even
  when the selection covers rows the app has not loaded.
- **A selection toolbar** in the toolbar's action row while a selection exists: the count,
  select all / clear, bulk tags, a bulk rating control, export selected.
- **The Inspector shows the selection**: one selected image is the panel that exists today; two
  or more become a header with the count and a thumbnail strip.
- **Bulk tag editing** (§6 Phase 2): one dialog that adds and removes tags across the selection,
  with quick-remove pills for the ten commonest tags in the selection and their counts — the
  legacy modal's shape, which is the one the owner works in.
- **Bulk rating** (§6 Phase 2): set or clear the rating of every selected image in one write.
- **Export selected** (§6): a zip of the selected originals, named by image id and extension,
  written by Rust to a path from the system save dialog, with progress while it runs and a
  report of any file that was gone.

## Capabilities

### New Capabilities

- `selection`: which images the next action applies to — pointer and keyboard selection over the
  grid, the count, select all and clear, and how a selection behaves when the result set under
  it changes.
- `bulk-operations`: applying one tag edit or one rating to every selected image, as a single
  all-or-nothing write, with the selection's own tag frequencies offered for removal.
- `export-selected`: writing the selected originals to a zip file the user names, reporting what
  was written and what was missing.

### Modified Capabilities

None. `library-browse`'s shipped requirements keep their behaviour: the grid still shows
non-deleted images newest first, a single click still focuses a thumbnail without opening it
(`app-shell` set that and this change does not touch it), and the lightbox is unchanged. A
selection is entered only with a modifier, so nothing already specified acts differently.

## Non-goals

- **Bulk delete.** There is no trash yet, and `drop_image_record` is Phase 1 D16's "the file is
  gone, forget the row" action, not a delete. Wiring it to a multi-selection would make "delete
  200 images" the one unrecoverable action in the app. `trash` adds **Move to trash** and
  **Restore** to this change's selection toolbar, and owns their specs.
- **Bulk anything else.** No bulk source edit, no bulk re-import, no bulk upload
  (`booru-upload` is per image, from the Inspector).
- **Selection outside the grid.** The lightbox shows one image and keeps its own navigation;
  it neither reads nor writes the selection.
- **A metadata file in the export zip.** The legacy dump carried `metadata.json` because the
  export was the only way data left the browser; here the library folder is the portable
  artefact (§6) and the zip is a hand-off of pictures.
- **Cancelling or resuming an export.** It reports progress and it finishes or errors.

## Impact

- `packages/shared`: `TagCount`, `ExportReport` and `ExportProgress`;
  `src-tauri/src/model.rs` mirrors them by hand (Phase 1 D11), so both change in one commit.
- `packages/app/src-tauri`: `commands.rs` gains `bulk_update_tags`, `bulk_set_rating`,
  `export_zip`, `selection_tag_counts` and `search_ids`; `query.rs` has its id-page query
  extracted so the selection resolves rows in the same order the grid pages them; new
  `export.rs`. New dependency `zip` (no default features — the entries are stored, not
  deflated). `capabilities/default.json` gains `dialog:allow-save`.
- `packages/app/src`: new `lib/api/selection.svelte.ts` (the store, with its own test),
  `lib/components/library/SelectionToolbar.svelte` and `BulkTagDialog.svelte`; `LibraryGrid`,
  `ImageCard` and `Inspector` gain selection awareness; the toolbar's action row swaps. New
  shadcn-svelte copy-ins `checkbox` and `progress`.
- No schema change and no migration: bulk tag writes go through the `tags` / `image_tags` tables
  Phase 1 D2 already ships, and the rating is a column on `images`. Schema v2 belongs to
  `bridge-extension`; the next version belongs to whichever change first needs one.
