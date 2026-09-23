## Why

Tagging a mixed batch — a parody set where every `cat` also needs `animal`, a folder to be
moved from `Uncategorized` into `Cute` and rated — costs, per image, a click into the editor,
typing, a save, a collection menu and a rating click. Danbooru's edit mode turns the same job
into "choose what to write once, then click the posts". The owner asked for that shape as
stamps: many saved edits, one active at a time, applied to one image by a click or to the
selection at once (2026-09-23). Requirements §6 (bulk ops, Danbooru-style tagging).

## What Changes

- **A stamp is a saved edit written in the tag language**: `cat animal -dog` adds and removes
  tags; `collection:cute -collection:uncategorized` moves between collections; `rating:g` sets
  the rating (never toggles it off); `artist:name` creates a tag under a category as
  `tag-vocabulary` describes. Stamps belong to the library, are managed on the settings
  screen beside the rules, and come back on a rebuild.
- **Edit mode**: a toggle in the toolbar (key `E` anywhere the grid's keys work) shows a
  stamp bar over the grid with the saved stamps, a field for a one-off stamp, and one active
  stamp. While the mode is on, a plain click on a thumbnail applies the active stamp to that
  image, and a button applies it to the selection, asking first when it is more than one
  image. Leaving the mode restores the ordinary click. There is no undo; the inverse stamp is
  the way back, and the bar says so.
- **One door for every multi-field edit**: adding and removing tags, moving between
  collections, setting the rating and creating categorised tags happen in one transaction per
  apply, for one image or for the selection. The bulk tag dialog writes through the same door.

## Capabilities

### New Capabilities

- `stamps`: the stamp language, saved stamps and their management, edit mode, applying to one
  image and to the selection, refusals.

### Modified Capabilities

- `selection`: the "entered deliberately" requirement gains the edit-mode exception: a plain
  click applies the active stamp instead of focusing only.
- `library-browse`: the Lightbox requirement's "a single click on a thumbnail that is not the
  current tile makes it current" gains the same exception; the second click does not open the
  viewer in edit mode.
- `app-frame`: the keyboard map gains `E`.
- `bulk-operations`: the bulk tag edit is stated as one apply of the same edit language.
- `library-recovery`: the library's own file describes the stamps; a rebuild restores them.

## Non-goals

- Undo.
- Reordering stamps by drag; they are listed in creation order.
- A stamp that clears the rating (`rating:none`): the owner's rule is that a rating stamp
  sets and never toggles, and clearing has its own control.
- Stamps that search (`is:`, `tagcount:`, `account:`, `or`): those tokens are refused as not
  an edit.
- Applying a stamp from the tile's context menu or the inspector outside edit mode. Edit mode
  is the one place a stamp is applied from; a second door doubles the surface for a feature
  the owner has not used yet.

## Impact

- Schema: one migration (planned as the one after `tag-vocabulary`'s by queue position; the
  design's sentence is amended to the real `MIGRATIONS.len()` when applied): a `stamps` table.
- `library.json` gains a `stamps` key; format stays 1.
- Rust: `tags.rs` (`apply_edit`, the one door; `bulk_update_tags` folded into it; the helper
  `stamp` renamed so the word is free), `stamps.rs` (new: list, upsert, delete), `model.rs`,
  `sidecar.rs`, `recover.rs`, `commands.rs`, `lib.rs`, `collections.rs` (membership writes
  callable inside a transaction).
- Shared: `TagEditSpec`, `Stamp`, `StampInput`.
- Webview: `domain/stamp.ts` (the grammar, pure), `api/stamps.svelte.ts`, `api/commands.ts`,
  `pending-write.ts` (the `edit` kind widened), `BulkTagDialog.svelte`, `LibraryScreen.svelte`
  (edit mode, the bar, the click routing), `LibraryGrid.svelte`, `ImageCard.svelte`, new
  `library/StampBar.svelte`, `stamps/StampsSection.svelte`, `StampForm.svelte`,
  `StampsTable.svelte`, `routes/settings/+page.svelte`, `keyboard.ts` (the map).
