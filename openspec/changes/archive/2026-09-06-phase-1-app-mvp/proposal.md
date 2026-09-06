## Why

Image storage inside the browser extension is fragile and caps what features are possible
(docs/requirements.md §1). Phase 1 is the app MVP: the smallest app that can receive
captures, hold a library on disk, and let the user browse and search it, so the legacy
extension can be frozen (§8, row "1 — App MVP").

## What Changes

- **Library folder** (§6): user picks a folder on first launch; the app remembers it and
  lays it out as `images/<id>.<ext>`, `library.sqlite`, `inbox/`. Missing or renamed files
  are shown as missing, never crash the app.
- **Capture ingest over localhost HTTP** (§5): listener on `127.0.0.1:47201`,
  `POST /captures` (multipart, idempotent by the caller's UUID), `GET /status`, `Origin`
  check for `chrome-extension://`.
- **Local file import** (§6): drop files or folders; metadata is filename, mtime,
  dimensions.
- **Browse and search** (§6): grid, Danbooru-style tag search using the lifted parser
  (AND, `or`, `-tag`, `rating:`, `is:`, `tagcount:`, `account:`), lightbox, and per-source
  counts so a migration can be verified (§2 guarantee 4, §9 step 4).
- **Metadata store** (§7): SQLite + FTS5, tags as rows, rollback journal, `posts` relation
  modelled from day one even though nothing writes it yet.

## Capabilities

### New Capabilities

- `library-folder`: choosing, remembering and laying out the library folder; tolerance of
  external file changes.
- `capture-ingest`: the localhost HTTP contract the bridge extension and any local source
  post to.
- `local-file-import`: importing plain image files and folders from disk.
- `library-browse`: grid, tag search, lightbox, per-source counts.

### Modified Capabilities

None. This is the first change in the repo.

## Non-goals

- Legacy bundle import. It was part of this change until the rest was finished and it was
  still gated on the legacy repo's Phase 0, which defines the bundle format and has not
  shipped; it moved to its own change, `legacy-bundle-import`, so a done phase could be
  archived without publishing a spec for behaviour nothing implements. The schema keeps
  `source=legacy-bundle` ready for it.
- The bridge extension itself (Phase 1b), tag and rating editing, bulk ops, auto-tag rules,
  trash UI, notes, booru upload (Phase 2), anything in Phase 3.
- Auto-launching the app when a capture arrives while it is closed (§5, "later nicety").
- JSON sidecars (§7, deferred). Code signing (§10).

## Impact

- `packages/app/src-tauri`: new crates rusqlite (bundled, FTS5), axum, tauri-plugin-dialog,
  image decoding; new Tauri commands; Rust owns filesystem, DB and listener.
- `packages/app/src`: lifted pure modules from the legacy repo (`tag-utils`, `filters`,
  `grouping`, `navigation-math`) with their tests; new SvelteKit routes and components.
- `packages/shared`: transport contract types (`/captures` JSON part, `/status` response,
  site-adapter record) used by the app now and the extension in Phase 1b.
- No dependency on any other repo: the one capability that needed the legacy repo's Phase 0
  is now the separate `legacy-bundle-import` change.
