## Context

See proposal.md — Why. What the code already gives this change:

- **One door in.** `ingest::store_image` is the only path that creates an image row, and it
  already does file-first-row-second: bytes to `inbox/<id>.part`, fsync, rename into
  `images/<a1>/<b2>/`, then the insert, and the file is unlinked if the insert fails. A
  sidecar is one more file with the same discipline.
- **One writer.** `SharedLibrary` is a `Mutex<Option<Library>>` around a single
  `rusqlite::Connection` (Phase 1 D1). Every write path is already serialised.
- **One place per file kind names its path.** `LibraryPaths::image_path`,
  `thumbs::thumbnail_path`. A third belongs beside them, in the module that owns the file
  kind, exactly as `thumbs.rs` owns the thumbnail's.
- **A one-time pass over the folder at open already exists.** `sharded-image-dirs` D4 put
  `relayout_to_buckets` in `Library::open_or_create` and argued that no schema version is
  involved, "the database does not describe the layout and never did". The same holds here.
- **Per-item locking is the established shape for long runs.** `import::import_paths`,
  `bundle::import_bundle` and `rules::run` take the library per item so a search is answered
  between two of them (Phase 1 D13).
- `load_records(conn, ids)` already returns the row, its tags and its posts in three
  statements however many ids are asked for — which is exactly a sidecar's content.

Constraints that shape everything below: the webview is reached only through Tauri commands
and events; `open_into_state` runs on the main thread, including from `setup` before the
window exists; the library mutex may never be held across an `.await`.

## Goals / Non-Goals

**Goals**

- A library folder from which `library.sqlite` can be deleted and rebuilt with nothing lost
  but the FTS index (which is itself rebuilt).
- One definition of the sidecar's content, shared by the writer and the rebuild.
- No user step: no backup button, no export before an import, no marker to tick.
- A rebuild that costs disk reads of small text files and nothing else — no decode, no
  rewrite of a single image byte.

**Non-Goals (design level; the proposal has the product-level ones)**

- Changing the schema. This change adds no `MIGRATIONS` entry (see D7) and therefore claims
  no schema version.
- Changing `ImageRecord` or the IPC contract for images. The sidecar is a second
  representation with its own type, on purpose (D2).
- A background verifier that compares sidecars against rows. The backfill writes what is
  missing; it never compares content.

## Decisions

### D1: One JSON file per image, beside the image, holding the whole row plus tags and posts

`images/<a1>/<b2>/<id>.json`, next to `images/<a1>/<b2>/<id>.<ext>`. The bucket is already
created by the image write, the stem is already the id, and a user who opens a bucket in
Finder sees the picture and the text that describes it together — which is the whole "the
folder is self-describing" claim from §7.

Content, taken from the `images` table (`db.rs`) plus the two relations `load_records` joins:

```json
{
  "version": 1,
  "id": "a1b2c3d4-…", "ext": "jpg", "mime": "image/jpeg",
  "size": 148213, "width": 1200, "height": 800,
  "source": "extension", "sourceRef": "x",
  "imageUrl": "…", "pageUrl": "…", "pageTitle": "…",
  "adapter": { "site": "x", "fields": { } },
  "rating": "s",
  "tags": ["artist:foo", "landscape"],
  "capturedAt": 1757000000000, "fileModifiedAt": null,
  "createdAt": 1757000000000, "updatedAt": 1757000000123,
  "deletedAt": null,
  "posts": [{ "site": "danbooru", "remoteId": "7412", "postedAt": 1757000000456 }]
}
```

`version` is the file format's own number, not `PRAGMA user_version`: the sidecar is now the
thing a future build has to be able to read, and a reader that cannot say which shape it is
holding has to guess. It is bumped only when the shape changes in a way a v1 reader would get
wrong.

Written pretty-printed, keys in a fixed order, tags sorted (which `load_records` already does
by `ORDER BY tags.name`), `None` fields omitted. Deterministic output means rewriting an
image whose facts did not change produces identical bytes, so a sync client has nothing to
upload; pretty-printing costs ~200 bytes per image against a folder that already holds the
images themselves, and buys a file a human can read in the folder they were told to trust.

*Alternative rejected:* one sidecar per bucket, or one large JSONL. Fewer files, but every
edit rewrites a file holding other images' data, which is both a bigger sync write and a
bigger thing to lose to one truncation.

### D2: The sidecar is its own type, not a serialised `ImageRecord`

`ImageRecord` is the hand-mirrored IPC contract with `packages/shared` (Phase 1 D11). Writing
it to disk would make every rename done for the webview's benefit a silent change to the
on-disk format of 25,000 files. `sidecar::Sidecar` is defined in `sidecar.rs` with its own
serde and its own `version`, and the rebuild reads *that*.

Two fields of `ImageRecord` are deliberately not in it:

- **`file`** — `images/<a1>/<b2>/<id>.<ext>` is a function of `id` and `ext`, both present.
  Storing it would be a second spelling of the layout that could disagree with
  `LibraryPaths::relative_image_path`.
- **`missing`** — it is a cache of the last time someone stat'd the file, written by
  `maintenance::refresh_missing_for` for whatever page of the grid was on screen. Storing it
  would make that pass a sidecar write path, so scrolling a synced library while its files are
  still downloading would rewrite thousands of sidecars to record a fact that is about to
  change back. The rebuild recomputes it instead, for free: it is already looking at the
  folder, so "is `<id>.<ext>` beside this sidecar" is a stat it was going to do anyway.

`maintenance::refresh_missing_for` is therefore **not** a sidecar write path. It is the only
write to `images` that is not.

### D3: One `library.json` at the library root for everything that is not per image

Rules, booru sites and the note are tens of rows between them, are always read and written
whole (`rules::export_json` already serialises the entire list), and are the only library-level
state in the database. One file:

```json
{ "version": 1, "rules": [ … ], "booruSites": [ … ], "note": { "content": "…", "updatedAt": 0 } }
```

It sits beside `library.sqlite` so the pairing is visible. Rule ids and site ids are written
verbatim and restored verbatim: `posts.site` holds a site id (`booru-upload` D2) and rule ids
travel in the export format (`auto-tag-rules` D10), so regenerating either on rebuild would
break a reference. Tag ids are *not* preserved — they are internal, and the rebuild recreates
them through `tags::link_tag`, the one place a `tags` row is made.

No API key, ever: `booru-sites` D7 puts those in the OS credential store keyed on
`<host>/<username>`, and a key in a synced folder is the one thing that would make this change
a security regression. `BooruSite` has no key field, so this is a property of the type, not a
rule someone has to remember.

*Alternative rejected:* a separate `note.md` a user could edit. It reads as an invitation, and
nothing would honour the edit until a rebuild — a trap. Keeping the note inside the app's own
file makes the folder's rule uniform: these files are the app's copy, not the app's input.

### D4: The sidecar is written after the row's transaction commits, in the same call

The commit is the last irreversible step of a write and it cannot be conditioned on a file
write. So there are two orders, and each leaves a window:

- *Sidecar first:* a failed commit leaves a sidecar for a row that does not exist. On rebuild
  it becomes a ghost row.
- *Commit first:* a failed sidecar write leaves a row with no recovery copy, and the call must
  still report failure even though the row landed.

Commit first wins because the second window **closes on its own**: the backfill (D7) writes a
sidecar for every row that has none, at every open. There is no equivalent repair for a ghost
row — nothing can tell it from a real one. And keeping file I/O out of the transaction matters
in its own right: a bulk edit over a whole selection would otherwise hold an open write
transaction, and the rollback journal, for as long as it takes to write thousands of files —
which is precisely the window in which the sync client did its damage.

The call still fails: the requirement is "a write that cannot be mirrored is a failed write",
so the sidecar's error is the call's error, and the capture listener answers non-2xx (§5's "a
capture is `failed` until the app answers 2xx"). Retrying then repairs it, which needs one
extra line: `store_image`'s idempotent early return (`Ingested::Existing`) also ensures the
sidecar, so a redelivered capture that half-landed writes the missing file rather than
returning success and leaving the hole.

Bulk paths read once and write many: `sidecar::write_for(paths, conn, ids)` uses
`load_records` (three statements for any number of ids) and then writes one file per id, after
the commit, still inside the command's hold of the library.

### D5: Every write path, named

Per image — each writes its sidecar after its own commit:

| Path | Trigger |
|---|---|
| `ingest::store_image` | capture, local import, bundle import — one door, so three sources cost one call site |
| `tags::update_tags` | the tag editor (tags + a `rating:` metatag) |
| `tags::set_rating` | the rating control |
| `tags::bulk_update_tags` | bulk add/remove over a selection |
| `tags::bulk_set_rating` | bulk rating over a selection |
| `trash::trash_images` / `restore_images` | `deleted_at` set and cleared |
| `trash::delete_forever` | **removes** the sidecar (and `empty_trash` through it) |
| `booru::posts::record` | a successful upload's `posts` row and `updated_at` bump |
| `rules::apply_rules_to_image` | one image changed by a rules run, per image, inside the run |

Library-level — each rewrites `library.json` after its own commit:
`rules::upsert`, `rules::delete`, `rules::import_json`, `notes::set`, `booru::sites::save`,
`booru::sites::delete`.

Not a write path: `maintenance::refresh_missing_for` (D2).

`delete_forever` removes the sidecar **before** the image file, and a sidecar that will not go
is a failure named in `DeleteReport.files_left` rather than a shrug. A leftover image file is a
stray the rebuild ignores; a leftover sidecar resurrects a permanently deleted image on the
next rebuild, which is the one outcome this whole change must not produce.

### D6: Temp-then-rename through `inbox/`, without an fsync

`sidecar::write` writes `inbox/<id>.json.part`, closes it, and renames it onto the
destination — the same route `ingest::write_through_inbox` takes, which means the existing
`Library::sweep_inbox` already removes a `.part` a crash left behind (it matches on the
extension `part`, which `<id>.json.part` has). `library.json` goes through
`inbox/library.json.part` the same way. Rename within one filesystem is atomic, so no reader
ever sees a half-written sidecar — the spec's "a reader never sees a half-written one".

No `sync_all`, unlike the image bytes. The failure this change answers is another process
rewriting the database file, not a power cut: a sync client reads through the filesystem and
sees the renamed file immediately, fsync or not. An fsync per sidecar would make a bulk edit
over a large selection pay thousands of disk flushes — seconds to tens of seconds with the
library mutex held — to narrow a window the database's own commit fsync already covers for the
primary copy. The image bytes keep their fsync because they are the irreplaceable thing; a
sidecar lost to a power cut is rewritten by the next open's backfill.

### D7: The backfill is a file check at open, not a marker, and no migration

"Which rows lack a sidecar" is answered by asking the folder: `SELECT id, ext FROM images`,
then `path.exists()` per row, then write the ones that are missing. Cost on the owner's
library: one query, 25,000 stats (well under a second locally), and — once — 25,000 small
writes; every later open is the stats and nothing else.

A marker (a `MIGRATIONS` entry, or a file saying "sidecars done") is cheaper on the second
open and wrong in every other case: a run interrupted half way, a folder where some sidecars
were deleted, a library restored from a partial copy, or a sidecar write that failed under D4
all leave the marker set and the folder incomplete. The self-correcting check is what makes
D4's "the window closes on its own" true. This is the same argument `sharded-image-dirs` D4
made for the relayout: the database does not describe the folder, so the folder is what to
ask. Hence **no new `MIGRATIONS` entry and no schema version claimed by this change**.

Where it runs: **not** inside `Library::open_or_create`. `open_into_state` is called on the
main thread, from `setup` before the window exists, so a 25,000-file pass there would hold the
window back from appearing. It runs after the library is in state, on a blocking thread, taking
the library per image through `with_library` — Phase 1 D13's shape, so a search during it is
answered between two images. It stops when the open library's root is no longer the root it
started for, which is what makes a library switch mid-pass safe without a second control type
(`import-pause-cancel` D4's handle is about a user-visible run; this pass has no controls).

`library.json` is written by the same pass when it is absent.

### D8: `PRAGMA quick_check(1)` on open, plus the corrupt-class errors

`db::open` runs `PRAGMA quick_check(1)` before anything else touches the file. It reads every
page and verifies each b-tree; `integrity_check` additionally verifies every index entry
against its table, costs several times as much on every open, and the extra class of damage it
finds — an index disagreeing with its table — is repaired by the same rebuild anyway. `(1)`
stops at the first problem because one problem is the whole answer.

`quick_check` is not the only detector: a file so damaged that it is not a database at all
fails earlier, and some damage only surfaces on a later query. So `AppError` gains a
`LibraryCorrupt { path }` variant, and rusqlite's corrupt-class codes (`DatabaseCorrupt`,
`NotADatabase`) map to it wherever they arrive — at open or mid-session. One variant means one
message, and the message names the recovery. A mid-session corruption is reported and nothing
else: the rebuild is offered from the start screen and Settings (D12), and the next launch's
`quick_check` finds it anyway.

`SchemaTooNew` and `JournalMode` are untouched and stay distinct: a library from a newer build
is not damaged, and rebuilding it would silently downgrade it.

### D9: "Damaged" is recorded by the attempt, not derived from the folder

`LibraryStatus.missing_path` is derived — a remembered path with no library open is the path
that would not open — and Phase 1 D3 argued for that over a stored field nobody remembers to
clear. That derivation cannot tell damaged from missing, and the start screen must: "your
library folder is missing" is wrong and frightening for a folder that is sitting right there.

Re-deriving it would mean repeating the open (a `quick_check` per status poll), so instead
`AppState` gains `open_failure: Mutex<Option<(PathBuf, OpenFailureKind)>>`, written by
`open_into_state` on **both** outcomes — set on failure, cleared on success. It is written by
exactly the one function that can change the answer, which is the property the stored-field
objection is really about. `status` reports `damagedPath` when the recorded failure names the
remembered path and its kind is `Corrupt`, and `newerPath` when its kind is `SchemaTooNew`;
everything else keeps today's `missing_path`.

`newerPath` was not in this design as written: the `SchemaTooNew` kind was to report no path at
all, on the reasoning that a library from a newer build is neither missing nor damaged. That is
true, and it was the right half of the answer. It leaves out the other half — the start screen
keys its entire wording on which path is set, so reporting none of them drops a remembered
library onto "Choose a library folder" with its folder never named, which `library-recovery`'s
"SHALL say which folder it is" rules out. Three slots, at most one of them ever set.

### D10: The damaged file is moved aside, with its journal, under an epoch-millisecond stamp

`library.sqlite` → `library.sqlite.corrupt-<now_ms>`, and `library.sqlite-journal` →
`library.sqlite-journal.corrupt-<now_ms>` in the same step when it exists. Moving the database
and leaving the journal would be worse than doing nothing: a stale hot journal beside a fresh
database is how this library died in the first place, and SQLite would try to roll it back into
the new file.

The stamp is `db::now_ms()`. It adds no date-formatting dependency for a filename, it is
fixed-width and therefore sorts chronologically for the next two centuries, and two rebuilds
cannot collide inside one millisecond. The user is never asked to read it: the app names the
file in the result, and `docs/storage.md` explains what it is. Nothing deletes these, ever —
"we never claim safe to delete" (§2.4) applies to the user's own database most of all.

### D11: Rebuild builds a temp database and renames it into place

1. Close the connection if that library is open.
2. Build into `library.sqlite.rebuilding` through the ordinary `db::open` — so the fresh file
   gets every migration and every trigger from one definition, never a second schema.
3. Walk `images/**/*.json` with `read_dir`, one bucket at a time; for each sidecar, insert the
   row with its stored timestamps, `missing` computed from whether `<id>.<ext>` is beside it,
   link its tags through `tags::link_tag`, insert its posts. Read `library.json` for the rules,
   the sites and the note. Commit in chunks so memory stays flat.
4. Move the old database aside (D10), rename the temp file onto `library.sqlite`.

Building into a temp file rather than in place is what makes "leaves no half-built library
behind" true: an interrupted rebuild leaves the original still there and a `.rebuilding` file
that the next attempt overwrites. It is also why the move-aside happens last — until the new
file is complete there is no reason to disturb the old one.

The FTS index comes from the `images_fts` triggers firing on each insert, not a final
`rebuild_fts`: one path, already correct, and 25,000 rows is small.

Never opened, never decoded: an image file is stat'd (does it exist) and nothing more. A
sidecar that will not parse is counted, named in the report, and left on disk — the same rule
`row_to_record` already applies to a malformed `adapter_json`, because one bad document must
not take the library down with it. An image file with no sidecar is left alone: rebuilding a
row for it would invent metadata, and §2's "we never claim" spirit says do not.

*Note for whoever touches `relayout_flat_files` next:* it maps any flat file under `images/`
through `image_path(stem, ext)`, which for `ext = "json"` is exactly the sidecar's path, so a
flat sidecar from a pre-bucket library buckets correctly with no change. `image::guess_format`
never yields `json`, so no image can collide with a sidecar's name.

### D12: The rebuild is a command the user asks for, from the two places damage is met

One command, `rebuild_library(path) -> RebuildReport`. It closes the library if that path is
the one open, rebuilds, and leaves **the rebuilt path closed**; a different library that
happens to be open is untouched, because it is unrelated to the rebuild and closing it would
tear down a frame the user never asked about. The caller then opens the rebuilt path through
the existing `open_library`. Two callers, one path through Rust:

- The start screen, when `damagedPath` is set: "This library's index is damaged" with the
  folder named, what a rebuild does, and that the current file is kept. Progress while it runs,
  then the report, then a button that opens the library. The user reads what happened before
  the grid replaces it.
- Settings → Library → "Rebuild library index", behind a confirmation that says the current
  database is kept aside.

Manual rebuild earns its place beyond symmetry: a database can be *stale* rather than damaged
— a sync client resurrecting yesterday's `library.sqlite` next to today's sidecars passes
`quick_check` perfectly — and without this control that library has no way back. It is also
how the owner can exercise the whole path on the real Synology folder without having to corrupt
anything, which is the hand check this change ends on.

Never automatic: moving a user's database aside is not something to do while they are not
looking, and a `quick_check` failure met at launch may be a folder the sync client has not
finished writing.

### D13: Its own progress event, not `import:progress`

`import:progress` carries `{ done, total, imported, skipped, failed }` and the webview's
`Imports` singleton listens for it; borrowing it would put a phantom import tile on screen and
give three fields that mean nothing here. Two new events instead, because the two passes are
seen in different places:

- `library:rebuild` → `{ done, total }`, rendered on the start screen (or the Settings dialog),
  where no library is open.
- `library:sidecars` → `{ done, total }`, rendered as one tile in the pending-work band, where
  a library is open.

`RebuildReport { images, failed, failures: [{ file, reason }], keptAs, rules, sites }` is
mirrored in `packages/shared` by hand, like every other IPC type (Phase 1 D11).

### D14: The journal mode stays DELETE, and no journal mode would have prevented this

§7 fixed the rollback journal so a sync client sees one file, and `assert_rollback_journal`
reads the mode back rather than trusting the pragma. Nothing here changes it:

- **WAL** splits the library across `library.sqlite`, `-wal` and `-shm`. A client that copies
  one of three restores a torn database, and `-shm` is machine-local by design. It would make
  the sync case worse, not better.
- **TRUNCATE / PERSIST** leave a journal file permanently in the folder for the client to copy
  around; DELETE removes it after each commit, which narrows the window this library fell into.
- **MEMORY / OFF** remove crash atomicity outright to avoid a file that is not the problem: the
  damage came from another process rewriting files, which no journal mode prevents.

The answer to another process writing your database is a second copy of the data, not a
pragma. That is what this change is.

### D15: Nothing here makes two writers safe

Two machines writing one synced folder still corrupt it, and now also produce two divergent
sidecar sets the sync client turns into conflicted copies. The sidecars are read only by a
rebuild and are never merged (`library-recovery`'s last requirement). §7's "one machine writes
at a time" stands, is restated in `docs/requirements.md` §7 by this change, and is what
`docs/storage.md` tells the user in the same breath as "a synced folder is fine for one
machine".

## Risks / Trade-offs

- **A bulk edit over thousands of images now writes thousands of files while holding the
  library.** → The writes are outside the transaction (D4), so the journal window does not
  grow; the hold does. Accepted for now, and the shape is already there to fix it — the writes
  could move behind the same per-item lock an import uses. Not built: make it work first.
- **Twice the files in `images/`.** → Buckets were sized for 65,536 ways of splitting; two
  files per image in a bucket that holds one or two images is nothing. A sync client's file
  count doubles, which is the cost §7 priced as "a second write per edit".
- **A sidecar lost to a power cut (no fsync, D6).** → The next open's backfill writes it again.
  The row it mirrors is in a database that did fsync.
- **A ghost row from a sidecar written for a rolled-back write.** → Cannot happen under D4's
  ordering; the only sidecar written before a commit would be one under the rejected order.
- **`quick_check` on every open costs a full read of the database file.** → Tens of
  milliseconds for a library of this size, once per open, against a failure mode that costs the
  whole library. If it ever shows up on a cold network volume, the check is one call to make
  conditional — the rebuild does not depend on it.
- **A rebuild resurrects an image the user permanently deleted**, if its sidecar survived the
  delete. → D5 removes the sidecar first and fails the delete loudly if it cannot.
- **The sidecar format drifts from the schema** — a column added later and not added to
  `Sidecar` is a column the rebuild silently drops. → A test that asserts every column of
  `images` is represented in `Sidecar` (read `pragma_table_info('images')` and compare against
  the struct's fields), so the next migration fails the suite rather than the next rebuild.
- **Sidecars are inside the asset-protocol scope**, since `grant_asset_scope` allows `images/`
  recursively. → They hold nothing the webview cannot already ask a command for, and narrowing
  the scope per extension would be a new rule for no gain.

## Migration Plan

No schema migration (D7). The upgrade is: install, open the library, let the backfill run
once. A library opened by an older build afterwards ignores the sidecars entirely — they are
files it does not read — so downgrading costs nothing and loses nothing. Rollback is
uninstalling the new build.

## Open Questions

None that change the specs, the approach or the split. Two settled here for the record: the
sidecar is not fsynced (D6) and the backfill has no cancel control of its own (D7).
