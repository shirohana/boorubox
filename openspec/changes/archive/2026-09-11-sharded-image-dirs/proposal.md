## Why

A library keeps every image directly under `images/` and every thumbnail directly under
`.thumbs/`. The owner's legacy collection is already past 25,000 images, and the legacy bundle
import (`legacy-bundle-import`) moves it into the app in one go. A flat folder of that size is
fine for APFS and NTFS themselves (the caps are per volume) but not for what sits on top:
Finder and Explorer listings, cloud-sync clients, NTFS 8.3 short-name generation, and FAT32's
65,534 entries per folder on a removable drive. Every large-scale image store buckets by hash
prefix; Danbooru, which the owner runs, uses two levels (`a1/b2/`). Doing it before the bulk
import means no library ever has to be reshaped at size.

## What Changes

- **Sharded layout**: an image lives at `images/<a1>/<b2>/<id>.<ext>` and its thumbnail at
  `.thumbs/<a1>/<b2>/<id>.jpg`, where `a1` and `b2` are the first two and next two characters
  of the id. Both paths keep their one definition each (`LibraryPaths::image_path`,
  `thumbs::thumbnail_path`); the writers create the bucket on demand.
- **Relayout on open**: files sitting flat under `images/` or `.thumbs/` from a library made
  before this change are moved into their buckets when the library opens. Idempotent and a
  rename within one volume, so it costs nothing on a library already in shape.
- **The record carries its file**: `ImageRecord` gains `file`, the path relative to the library
  root, so the webview stops composing `images/<id>.<ext>` itself.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `library-folder`: the "Layout is stable and self-contained" requirement names the sharded
  path and the relayout of a flat library.

## Non-goals

- Validating ids at the capture door. An id is trusted as a file-name component today; the
  shard function must merely never fail on a short or odd id. A `FIXME` in `library.rs` names
  the right shape.
- Removing empty buckets after a permanent delete. Empty directories are harmless.

## Impact

- `packages/app/src-tauri`: `library.rs` (paths, relayout), `thumbs.rs`, `ingest.rs`
  (`row_to_record`, the rename into a bucket), `model.rs`; the tests that assert file paths.
- `packages/shared`: `ImageRecord.file`.
- `packages/app/src`: `assets.ts` uses the record's `file`.
- `openspec/specs/library-folder`: the layout requirement.
