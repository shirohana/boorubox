## Context

`phase-1-app-mvp` shipped the storage schema, the ingest write path and the browse UI this
change plugs into; read its design.md first. The mapping below is D9 of that design, moved
here with the capability it belongs to when Phase 1 was archived without it, and amended
against a real bundle (D1, D4).

Local file import (`import.rs`, Phase 1 D8/D12/D13) is the model for the run shape: a
blocking function taking the library one item at a time, `import:progress` events, an
`ImportReport` of per-item outcomes, and the webview's `Imports` queue that outlives the route
which started it.

## The bundle, as exported today (format `1.0`)

A folder `image-storage-backup-<date>-<ms>/` holding `manifest.json`
(`{ exportedAt, totalImages, files, version: "1.0" }`) and the parts: `database.db` when
there are 200 images or fewer, else `database-part<N>of<M>.db`. Each part is a SQLite file
with one table:

```
images (id TEXT PRIMARY KEY, imageUrl, pageUrl, pageTitle, mimeType, fileSize, width,
        height, savedAt INTEGER, updatedAt INTEGER, tags TEXT, isDeleted INTEGER, rating TEXT,
        blob BLOB)
```

`id` is the extension's UUID v4. `tags` is a JSON array of strings (`["blue_archive"]`), not
a comma string. `rating` is `g`/`s`/`q`/`e` or null. `updatedAt` is the last metadata edit,
present on a small minority of rows. The writer is
`chrome-image-storage/src/storage/sqlite-import-export.ts`; the checked-in fixture
(`packages/app/src-tauri/fixtures/legacy-bundle/`) is four rows of the owner's 2026-09-10
export of 3,569 images, one of them with its blob replaced by four bytes.

## Goals / Non-Goals

**Goals:**

- Every row in a bundle ends in exactly one of imported / skipped / failed, and the user can
  reconcile that report against the browser before deleting anything.
- Import goes through the same `ingest::store_image` door as captures and local files, so
  there is one write path and one place idempotency lives.
- One part file is a valid import on its own, so a user can move a bundle over in pieces and
  a tester can import 200 images without waiting for 3,569.

**Non-Goals:**

- Re-deriving tags or ratings. Whatever the bundle carries is what is stored — `ingest`
  already skips the auto-tag rules for `source = LegacyBundle` for this reason.
- Two-way sync with the legacy extension. This is a one-time move.
- Reading the manifest, or anything beyond the `images` table.

## Decisions

**D1 (was `phase-1-app-mvp` D9, amended). Bundle rows map onto the schema as follows.**
`id` is kept as the app's id — the extension made it a UUID and the app is idempotent on it.
`blob` is the bytes `store_image` decodes; the bundle's `mimeType`, `fileSize`, `width` and
`height` are not trusted, the decode decides. `imageUrl`, `pageUrl`, `pageTitle` map to their
columns. `savedAt` becomes `captured_at`. `tags` is parsed as a JSON array — the plan said
"comma tags"; the real export writes `JSON.stringify(tags)` — and passed as `input.tags`;
`rating` is passed as-is. `source = legacy-bundle`, `source_ref` = the bundle folder's name
(the parent directory of the part file), `adapter = None`; the `account:` filter reads the
page URL, so timeline captures still match. `isDeleted = 1` becomes `deleted_at` = the
import time: `updatedAt` means "last edited", is present on 55 of 3,569 rows, and the trash
view orders by `deleted_at`, so the import's own moment is the one honest timestamp.
`updatedAt` is otherwise dropped; `created_at`/`updated_at` are the row's insertion as for
every other source. Every row ends in exactly one of imported / skipped (id already
present) / failed (reason), and the report is shown before the counts.

**D2. The mapping lives in one module with fixtures from a real bundle.** `bundle.rs` reads
parts and maps rows; nothing else knows the legacy column names. The fixture is real rows,
so a format change is a contained edit rather than a hunt.

**D3. `ingest::store_image` gains a `deleted_at` input rather than a second write path.**
Phase 1 left that field out deliberately — nothing needed it — and a trashed bundle item is
the first thing that does. A separate insert for bundle rows would fork the one door every
image enters by. Every existing caller passes `None`.

**D4. Format `1.0` is pinned now; the Phase 0 gate is lifted.** The proposal of 2026-09-06
gated this change on the legacy repo shipping Phase 0, which was right then: with no bundle
in hand there was no format to pin and no fixture. It stopped being right on 2026-09-10 when
the owner exported their real library with the extension as it is: the images are the whole
migration, the app has grown its own rules import and notes since, and the legacy repo has
not moved. If a later export adds rules, notes or settings, it bumps `version` and a later
change reads the new parts; this reader ignores everything but the `images` table and does
not read the manifest at all — the §9 comparison is against the old viewer's count, not the
manifest's.

**D5. The unit of import is a part file, not the folder.** The command takes a list of `.db`
paths; the webview picks them with a multi-select file dialog. Parts are imported in
natural order of their `part<N>` number (name order otherwise) and rows within a part in
`rowid` order, so a re-run resumes through skips in the same sequence. A part that cannot be
opened as SQLite, or has no `images` table, is one `failed` item naming the file, and the run
goes on. `total` for the first progress event is the sum of `COUNT(*)` over the parts, read
before any row.

**D6. One report type, one queue, one progress band.** The command answers with the existing
`ImportReport`; a bundle row's `ImportOutcome.path` is its `pageUrl` (the thing a user can
find in the old viewer), `id` is its id whether imported or skipped (the field's comment is
widened from "new image id when imported"), `reason` is the skip or failure text. The
webview's `Imports` queue gains a second run kind that calls `import_bundle`; `import:progress`
is reused unchanged. A second queue would mean two progress bands and two report lists for
one activity.

**D7. The `/import` route is where a bundle run starts and where its report is read.** The
route explains what to pick, offers the file picker, shows the running progress, and renders
the latest bundle report grouped by status — failed and skipped expanded, imported collapsed
behind its count, so a 3,569-row report is readable — then the total and `legacyBundle`
counts from `image_counts` with the §9 notice verbatim. The route is reachable from the
sidebar between Trash and Settings.

**D8. Each row holds the library once, and a thumbnail is warmed outside it (Phase 1 D13).**
Rows stream from the part's own read-only connection; the blob (up to a few megabytes) is
read, then `with_library` for the one `store_image`, then `warm_thumbnail`. Decoding is the
cost — the door decodes the full image for its dimensions — so 3,569 rows take minutes with
a progress bar, and a bundle of 25,000 takes proportionally longer. A header-only dimension
read in `ingest::decode` would help every source and is not this change's.

## Risks / Trade-offs

- [The bundle format changes after this design] → D2 keeps the mapping in one module with a
  fixture from a real bundle; D4 says a new version is a new change.
- [A huge bundle blocks the UI] → D8: the run is off the main thread, holds the library per
  row, and reports progress.
- [A part copied half-way] → SQLite refuses it; D5 reports the file as failed and imports the
  rest.

## Open Questions

- Whether a bundle carrying an id already in the library should ever overwrite rather than
  skip. Skip until someone hits a reason not to.
