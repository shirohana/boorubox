## Why

The app has no way to delete an image. `drop_image_record` forgets a row whose file is already
gone (Phase 1 D16) and is offered on nothing else; a picture the user does not want stays in the
grid forever, and the disk it occupies can only be reclaimed in Finder. docs/requirements.md §8
lists "trash" under Phase 2 Parity, and §6 names the legacy viewer as the reference for the
behaviour: two views, a per-image delete, restore, permanent delete, and Empty trash.

Deleting is also the one thing §2 constrains directly. Guarantee 2 — "nothing auto-deletes" —
and guarantee 4 — the app never claims data is safe to remove — mean a delete has to be an act
the user performs, twice, and never one the app performs on a timer. That is what a trash is:
the first act is free and reversible, the second is confirmed and final.

Phase 1 D2 already ships the column. `images.deleted_at` has been in schema v1 from the first
migration, `SearchRequest` carries `includeDeleted`, `query.rs` filters on it and Phase 1's tests
already exercise a deleted row — with a `#[cfg(test)]` helper standing in for "the action a trash
view eventually will" write. This change is that action.

**Depends on:** `app-shell` (the sidebar nav, the Inspector, the keyboard map and the slot map
that says where each control lands) and `selection-and-bulk` (the selection store and the
toolbar this change adds two buttons to; its proposal already names bulk delete as this change's
work). It lands after both.

## What Changes

- **Move to trash** (§8 Phase 2): a soft delete that writes `deleted_at`, hides the image from
  browsing and from the library's counts, and touches nothing on disk. Offered on the grid tile's
  context menu, in the Inspector's action row, on a selection, and on the `Delete` / `Backspace`
  keys.
- **A Trash view**: a sidebar entry with a live count, opening the same grid, search, lightbox
  and Inspector against the trashed rows instead of the library's. `SearchRequest`'s
  `includeDeleted` boolean becomes a `view` of `library` or `trash` — **BREAKING** for the
  IPC contract, which has one consumer inside this repo.
- **Restore**: puts an image back exactly as it was — same tags, same rating, same place in the
  order — from the tile, the Inspector or a selection.
- **Delete forever** (**BREAKING**, reverses Phase 1 D16): removes the row, the thumbnail **and
  the file under `images/`**. Confirmed, never bound to a key, and reachable only from the trash.
  `drop_image_record` is removed: a row whose file is already gone is exactly the case where
  "delete forever" has no file to unlink, so the two actions collapse into one.
- **Empty trash**: the same permanent delete over everything in the trash, behind a confirmation
  that names how many images it will destroy.
- **Nothing is purged automatically** (§2 guarantee 2): no retention window, no purge on quit, no
  purge on open. The trash holds what the user put in it until the user empties it.
- **The missing-file card's action changes** from "Remove record" to "Move to trash", so a file
  that was removed outside the app — and may come back — no longer costs an unrecoverable step.
- **Settings → Library gains a trashed count** beside the per-source counts, so "what the app
  holds" (§2 guarantee 4, §9 step 4) still includes what is in the trash while the numbers the
  user reconciles against the browser keep describing the library.

## Capabilities

### New Capabilities

- `trash`: deleting an image without losing it, browsing what has been deleted, restoring it,
  destroying it deliberately, and the guarantee that nothing leaves the trash on its own.

### Modified Capabilities

- `library-folder`: "External file changes never crash the app" — the action offered on an image
  whose file vanished becomes moving the record to the trash rather than dropping it, so a file
  that reappears has a row to reappear into.

`library-browse` is **not** modified. Its "Grid shows the library" requirement is about the
library view, which still shows non-deleted images newest first; the trash is a second view and
its behaviour is specified by the new capability. Its "Per-source counts" requirement is
unchanged on purpose: those numbers exclude trashed rows today, and §9 step 4 compares them
against a browser viewer whose own count excludes its trash, so making them agree would break the
one comparison they exist for (design D9).

## Non-goals

- **Undo, as a toast or otherwise.** Restore is the reversal, and it lives in one place. A
  transient "Undo" would be a second path holding its own copy of what was just deleted, and the
  Trash badge incrementing in the sidebar is the feedback that the action landed.
- **A retention policy, even one defaulting to off.** A setting whose only safe value is off is a
  trap one mis-click arms (design D8).
- **Deleting from the lightbox.** It shows one image and navigates its own way (`app-shell` D10);
  a delete key there would leave the viewer on an image that is no longer in the result.
- **Recovering a file after "Delete forever".** That is the point of the confirmation.
- **A "recently trashed" sort.** The trash uses the grid's ordering; ordering choices belong to
  `sort` in `SearchRequest` (design D10).
- **Deduplicating a re-import against a trashed row.** Local import assigns fresh ids (Phase 1
  D8); importing a file whose earlier copy is in the trash makes a new image, as it does today.

## Impact

- `packages/shared` + `packages/app/src-tauri/src/model.rs` (hand-mirrored, Phase 1 D11 — one
  commit, both files): `SearchRequest.includeDeleted` becomes `view: 'library' | 'trash'`; new
  `DeleteReport { deleted, filesLeft }`.
- `packages/app/src-tauri`: new `trash.rs` with `trash_images`, `restore_images`,
  `delete_forever`, `empty_trash`, `trash_count`; `maintenance::drop_image_record` folds into the
  permanent-delete path and the `drop_image_record` command is removed; `query.rs`'s deleted
  filter becomes a view switch. **No schema change and no migration** — `deleted_at` is Phase 1
  D2's schema v1, schema v2 (`images.adapter_json`) belongs to `bridge-extension`, and v3 is free
  for whichever Phase 2 change first needs one.
- `packages/app/src`: new route `/trash`; the library screen becomes one component taking a view
  (`routes/+page.svelte` and `routes/trash/+page.svelte` both render it); new
  `lib/api/trash.svelte.ts` holding the count; `SelectionToolbar`, the tile context menu, the
  Inspector's action row, `ImageCard`'s missing state and `LibraryGrid`'s keydown handler each
  gain trash actions; `lib/api/commands.ts` loses `dropImageRecord` and gains five wrappers.
- No new dependency, no new shadcn copy-in beyond what `app-shell` and `selection-and-bulk`
  already add (`dialog`, `dropdown-menu`, `badge`).
