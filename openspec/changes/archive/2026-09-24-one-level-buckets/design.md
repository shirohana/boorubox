## Context

`library::shard_dirs(id)` is the one definition of the bucket rule, shared by
`LibraryPaths::image_path`, `LibraryPaths::relative_image_path` (the string `ImageRecord.file`
carries, computed at read — there is no `file` column), `thumbs::thumbnail_path` and, through
`image_path`, `sidecar::path`. `Library::open_or_create` and `open_existing` run
`relayout_to_buckets`, which lifts files sitting flat under `images/` and `.thumbs/` into their
buckets (`sharded-image-dirs` design D4). `recover::walk_sidecars` walks whatever tree is
under `images/`. `thumbs::THUMB_EDGE = 384`, "until the grid exists to judge it"
(`phase-1-app-mvp` design, Open Questions). The webview caches one thumbnail URL per id for
the window's life (`thumbnail-cache.ts`), built by `convertFileSrc` over the path
`thumbnail_path` answers; the browser caches the image itself by URL. A background pass with
progress and a stop on library switch exists in `library::backfill_sidecars`, driven from
`commands::open_and_start_backfill`; a user-started pass with progress and a report exists in
`rebuild_library` (`REBUILD_PROGRESS_EVENT`, `RebuildStatus.svelte`).

## Goals / Non-Goals

Goals: the new layout, a library that reaches it on open, the larger edge, a regeneration the
user starts and can watch. Non-goals: in the proposal.

## Decisions

**D1. One level: `<a1>` = the first two characters, nothing after it.** `shard_dirs` returns
`[id[..2]]` by character boundary, `a` → `["a"]`. This reverses `sharded-image-dirs` design
D1, which chose two levels because "two levels cost nothing more and never need a second
reshaping". That was true and is still true; what changed is the owner's judgement that a
flatter tree is worth one reshaping now, before any release is stable and while the library
is theirs alone (2026-09-24). One level is 256 buckets: about a hundred files per bucket at
today's 25,000 images and about a thousand at 256,000, well inside what every filesystem the
app runs on handles without slowing. The doc comments on `shard_dirs`, `relative_image_path`
and `thumbnail_path` name one level and this change.

**D2. The relayout lifts one level, on every open, and still lifts a flat library.**
`relayout_to_buckets` becomes `relayout_to_one_level`: for each of `images/` and `.thumbs/`
it (a) runs `relayout_flat_files` as today, so a pre-shard library still lands in its buckets,
then (b) for each directory entry `<a1>/` reads it, and for each subdirectory `<b2>/` inside
moves every regular file up to `<a1>/` (skipping a destination that exists, removing a stray
`.part` and the OS's own droppings — `.DS_Store`, `Thumbs.db` — swallowing every error per
file and per `<b2>/` read, as `relayout_flat_files` does — a sync client can evict a `<b2>/`
between the listing and the read), then removes `<b2>/` if it is empty. A `<b2>/` that is not
empty afterwards (a file that could not move, a stray directory) is left; the next open tries
again. `bucket.is_dir()` / `subdir.is_dir()` never stat: both directory listings hand back
`DirEntry::file_type()`, which the platform's `readdir` already carries. An already-migrated
library pays one `read_dir` of `images/`, one of `.thumbs/`, and one `read_dir` per bucket, no
stat per file — milliseconds for 256 near-empty bucket reads, and cheaper than a marker file
that could lie. The walk is one function over both directories with a closure for the
destination, as `relayout_flat_files` is; a file's destination is `<a1>/<name>` verbatim — the
walk never recomputes the bucket from the stem, so a file in the wrong bucket stays where it
is rather than being "fixed" into a path the record does not expect.

**D3. Thumbnail edge 768.** `THUMB_EDGE = 768`, quality unchanged at 82. The 640 px tile at
DPR 1 draws it at 0.83×, at DPR 2 at 1.67× — the latter is what a 360 px tile does from a
384 px file today, which the owner found fine on a Mac. `downscale` never upscales, so a
smaller source keeps its size. Existing thumbnails stay at 384 until regenerated (D4).

**D4. Regeneration is a user-started background pass, per image off the lock, with
progress, stopping on a library switch.** `thumbs::regenerate_all(shared: &Mutex<Option<
Library>>, root: &Path, progress: &mut dyn FnMut(done, total))`: under the lock, reads every
`(id, ext)` from `images` (trashed rows included — a trashed image keeps its thumbnail until
deleted forever) and the paths; releases. Then, for each record, the lock is taken again only
to check the open library's root is still `root` (else the pass returns) — never held through
the render — and the image is rendered off the lock, against the paths read at the start,
straight through `write_through_part` (never `ensure_thumbnail`, whose already-there check
would skip every image this pass exists to redo): its rename replaces `<id>.jpg` atomically,
so a render that fails leaves the old thumbnail rather than an earlier removal losing it. A
per-image error is swallowed into a count, and `(done, total)` is reported after each. The
shape is `backfill_sidecars`'s, and the same check stops it when the library is switched. A
Tauri command `regenerate_thumbnails` spawns it on `spawn_blocking` and emits
`THUMBS_PROGRESS_EVENT = "thumbs:progress"` with `{ done, total }`, resolving when the pass
ends with `{ regenerated, failed }`. Two passes at once are refused with a `Busy` error the
button reads as "already running"; an `AtomicBool` on `AppState` is the flag.

**D5. Settings → Library gets the button and the progress.** Under the counts: a
"Regenerate thumbnails" outline button, disabled while running, with a `Progress` bar and
"n of total" beside it while the pass runs, then "Regenerated n" (and "m could not be read"
when `failed > 0`) until the next run or a library switch. A new store
`api/thumbs-regenerate.svelte.ts` (`running`, `progress`, `report`, `start()`), the shape of
`rebuild.svelte.ts`, listening to the event from `events.ts`.

**D6. The grid shows the new files: the URL carries the file's version, and the cache is
dropped when the pass ends.** `thumbnail_path` answers `{ path, version }` where `version`
is the file's modified time in ms (`ThumbnailRef` in `model.rs` and shared), and `assets.ts`
builds `convertFileSrc(path) + '?v=' + version`; `thumbnail-cache.ts` gains `forgetAll()`,
called by the store when the pass reports done. A card drawn after that asks Rust again and
gets a new version, so the browser fetches the new file rather than serving the old bytes
under the old URL. Without the version the cache-drop alone would re-ask Rust and get the
same URL back, and the webview's image cache would keep the old thumbnail on screen.

## Risks / Trade-offs

- [A relayout over a cloud-synced folder] → every move is a rename inside the same volume; a
  file mid-sync that cannot be renamed is skipped and retried on the next open, as today.
- [Regeneration over 25,000 images takes tens of minutes] → it is off the lock, per image,
  and the grid keeps working; the pass stops on a switch and is idempotent, so it can be
  started again.
- [768 px thumbnails are roughly four times the bytes] → an estimate for the owner's library
  is 25,000 × ~80 KB ≈ 2 GB against ~0.6 GB today; accepted (2026-09-24).

## Migration

Automatic on open (D2). No schema change. `relative_image_path` changes what every record's
`file` reads, in step with where the file is, since both derive from `shard_dirs`.
