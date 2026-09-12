## Why

On 2026-09-12 a library living in a Synology Drive bi-directional sync folder went
`database disk image is malformed` during a 200-row bundle import and would not reopen. The
sync client rewrites `library.sqlite` and its `-journal` underneath the app while
`store_image` commits once per image; sqlite.org/howtocorrupt calls the result a stale hot
journal. §10 already names this risk and §7 already names per-image sidecars as the recovery,
deferred on day one. Nothing about that deferral was wrong in June; what changed is that the
failure happened, to a real library, with no backup step the user could have taken instead.

Today the database is the only copy of every tag the user typed. Losing it loses the library
even though every image file is still sitting on disk, intact, in its bucket.

## What Changes

- **Every image gets a JSON sidecar beside it**, `images/<a1>/<b2>/<id>.json`, carrying its
  whole row plus its tags and its posts. Every write path that changes an image writes it in
  the same call that writes the row (§7's "costs a second write per edit", accepted).
- **The library's non-per-image state gets one file**, `library.json` at the library root:
  the auto-tag rules, the configured booru sites, and the library note. API keys stay in the
  OS credential store, exactly as `booru-sites` already requires; nothing secret is written.
- **Opening a library backfills what is missing.** The first open after this ships writes a
  sidecar for every row that has none, as background work, with the library usable while it
  runs. The pass is a file check, not a marker, so it also repairs a library whose sidecars
  were partly deleted or never finished.
- **Opening a library checks it.** `PRAGMA quick_check` on open, plus the corrupt-class
  errors the open path can raise; a library that fails is not opened and is not touched.
- **A damaged library can be rebuilt from its folder.** The bad file is moved aside as
  `library.sqlite.corrupt-<stamp>` — never deleted — a fresh database is built from the
  sidecars and `library.json`, and no image or thumbnail is decoded or rewritten. Offered on
  the start screen when a library will not open, and in Settings for one that opens.
- **The database becomes a rebuildable index.** It stays the only thing read from: nothing
  reads a sidecar except the rebuild.
- **This does not make two writers safe.** §7's "one machine writes at a time" is unchanged
  and is restated in the requirements: sidecars are recovery, not concurrency.

## Non-goals

- Concurrent writers, file locking, or merging two machines' divergent sidecars.
- Reading sidecars at runtime, or letting a hand-edited sidecar change the library without a
  rebuild. The database is what every read goes through.
- Making the sidecar the canonical record in the Eagle sense: the row is written first, the
  sidecar mirrors it, and a disagreement is settled by the database until the database is
  gone.
- Rebuilding from image files alone. A file with no sidecar stays a file with no sidecar.
- Restoring a library written by a newer build. A `SchemaTooNew` library is refused, as now,
  and is never rebuilt into a downgrade.
- Storing settings (`settings.json`, app config dir) or API keys (credential store) in the
  folder. §7's split stands.
- Backups, versioning or history of any kind. One current sidecar per image.

## Capabilities

### New Capabilities

- `library-recovery`: the library folder describes itself well enough to rebuild
  `library.sqlite` from it — the sidecar written beside every image, the library-level file,
  the check on open, and the rebuild.

### Modified Capabilities

- `library-folder`: "Layout is stable and self-contained" currently says metadata SHALL be
  stored **only** in `library.sqlite`. That is the sentence this change reverses.
- `pending-work`: the sidecar backfill is work in flight the library screen has to show, on
  the same terms as an import — visible, and never blocking a search.

## Impact

- `packages/app/src-tauri/src/sidecar.rs` (new) — the file format, the writer, the backfill.
- `packages/app/src-tauri/src/recover.rs` (new) — `quick_check`, moving the bad file aside,
  the rebuild.
- Write paths that gain a sidecar write: `ingest.rs` (`store_image`), `tags.rs`
  (`update_tags`, `set_rating`, `bulk_update_tags`, `bulk_set_rating`), `trash.rs`
  (`trash_images`, `restore_images`, `delete_forever`, `empty_trash`), `rules.rs`
  (`upsert`, `delete`, `import_json`, and the per-image writes of `run`), `notes.rs` (`set`),
  `booru/sites.rs` (`save`, `delete`), `booru/posts.rs` (`record`).
- `db.rs` — the check on open; `error.rs` — a corrupt-library variant; `library.rs` — the
  backfill started at open; `commands.rs` / `lib.rs` — the rebuild command and its event;
  `model.rs` + `packages/shared/src/index.ts` — the report and progress shapes.
- `packages/app/src/routes/start`, `packages/app/src/routes/settings`, the pending-work band.
- `docs/requirements.md` §7 and §10 (the deferral and its reversal), `docs/storage.md` (new,
  user-facing), `README.md` (a status paragraph).
- No new Rust or JS dependency: `serde_json` is already in the tree.
