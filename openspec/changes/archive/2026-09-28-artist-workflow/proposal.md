## Why

`artist-entries` (2026-09-25) made the correction stick, but only from one door: an artist
tag's "Rename artist…" in the inspector, which reads its URL from the image on screen. Adding
the second X account of an artist already corrected, or making an entry for an artist whose
derived name is already right, has no convenient path: Settings → Artists takes URLs typed by
hand and tags nothing already stored, and nothing on the inspector says which entry, if any,
owns the image's author. The owner wants account-to-artist tagging convenient, without a
migration of the library they did not ask for, and without touching `account:` search
(owner, 2026-09-28). Requirements §6 (Danbooru-style tagging), §11 (policy lives in the app).

## What Changes

- **"Edit artist…" replaces "Rename artist…"** and is offered on every artist tag's menu —
  the sidebar row, the pinned chip and the inspector badge. The dialog reads the entry's own
  URLs and the tag's carrier count, and, opened from an image, appends that image's profile
  URL when no entry owns it yet. Saving under the same name replaces the entry's URLs; saving
  under a new name renames as before.
- **An Artist row among the inspector's facts**, above Account, for any image whose record
  yields a profile URL (X, and Pixiv records carrying `userId`): the owning entry's tag as a
  chip that toggles it in the search, or "Create artist…" when no entry owns the URL.
- **Create artist…** makes the entry from the image's profile URL, named by default with the
  tag capture already derives, and tags every stored image from that profile with it, after
  saying how many that is and how many carry no artist tag at all.
- **Apply an entry to stored images, opt in**: the Edit artist dialog offers "Apply to
  existing images" (unchecked), and each entry in Settings → Artists gains "Apply…", closing
  the pass `artist-entries` left as a FIXME. Applying only adds the tag; nothing is removed.
- `account:` search and the Account row are unchanged; no image changes unless the user
  creates an artist or applies an entry.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `artist-entries`: the rename requirement becomes the edit requirement (every artist-tag
  menu, entry URLs, same-name save); the settings requirement gains apply; new requirements
  for the Artist row, Create artist, and applying an entry to stored images.
- `tag-editing`: the facts order around the Account row gains the Artist row above it; the
  Account row's behaviour is unchanged.

## Non-goals

- A global pass re-deriving every image's artist tag from the entries (owner, 2026-09-28):
  applying is per artist, chosen by the user.
- Removing or replacing a tag when applying: an image carrying the derived handle keeps it
  beside the entry's tag; renaming the derived tag is Edit artist's rename, as before.
- Any change to `account:` search, the Account row, or derivation at capture
  (`capture-ingest` is untouched).
- A profile URL for Pixiv captures stored before the record carried `userId`: they have no
  Artist row, and a rename by name still reaches them.
- Renaming tags of other categories (the `artist-entries` non-goal stands).

## Impact

- Rust: `artists.rs` gains `artist_preview`, `artist_match`, `apply_preview`, `apply`;
  `rename_preview` and its command go; four commands in `commands.rs`; wire types in
  `model.rs` and `packages/shared`; TS wrappers in `api/commands.ts`.
- Webview: `RenameArtistDialog.svelte` becomes `ArtistDialog.svelte` hosted by `Inspector` and
  `TagSidebar`; `TagVocabularyMenuItems` gains the item; `Inspector` gains the Artist row;
  `ArtistsSection` gains Apply and loses its FIXME.
- No schema change, no extension change.
