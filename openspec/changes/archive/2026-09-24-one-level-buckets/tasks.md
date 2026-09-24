> Two units in sequence, Sonnet each (retried on Opus if the gate fails): unit R (Rust:
> the layout, the relayout, the edge, the pass, the command), then unit W (webview: the
> versioned URL, the store, the Settings button) after R lands. Design D1–D6 decide every
> shape; do not re-decide them. Gate for every unit: `mise run check` green. No unit ticks a
> hand check. Sibling changes in flight: `sidebar-inspector-polish` (unit A touches
> `model.rs` — `GRID_TILE_MAX` — and `packages/shared/src/index.ts`; unit C touches nothing
> here) and, later, `pinned-tag-groups` (`db.rs`, `tags.rs`, `commands.rs`, `model.rs`,
> shared). Edits to `commands.rs`, `lib.rs`, `model.rs`, `packages/shared/src/index.ts`,
> `api/commands.ts`, `api/events.ts`, `api/index.ts` are additive; rebase, never merge by
> hand. Never run the relayout against `boorubox-vault/test-1`: tests use tempdirs, and the
> smoke run uses a scratch copy.

## 1. Unit R — layout, relayout, edge, pass (`packages/app/src-tauri`)

- [x] 1.1 `library.rs`: `shard_dirs` per D1 with its doc comment; `relative_image_path` and
      `image_path` doc comments name `images/<a1>/<id>.<ext>`. Tests: the three `shard_dirs`
      tests rewritten for one level (`a` → `["a"]`, `abc` → `["ab"]`, a non-ASCII id slices by
      character); `a_flat_image_moves_into_its_bucket_on_open_and_the_record_still_loads`
      asserts the one-level path.
- [x] 1.2 `library.rs`: `relayout_to_one_level` per D2 (flat lift, then the `<a1>/<b2>/`
      lift, both directories), called where `relayout_to_buckets` was. Tests:
      `a_two_level_library_lifts_its_files_one_level_on_open` (image, sidecar and thumbnail
      at `a1/b2/`, all three at `a1/` after, both `b2` dirs gone, the record loads, a second
      open moves nothing); `a_file_already_at_its_destination_is_left_alone` (both copies
      survive, `b2` stays); `a_stray_part_under_b2_is_removed`; `a_non_empty_b2_is_left`
      (a subdirectory inside it). Verify: `cargo test library::` pass.
- [x] 1.3 `thumbs.rs`: `THUMB_EDGE = 768` (D3) with its doc comment amended (the open
      question is closed: the grid exists, 640 px tiles judged it); `thumbnail_path` doc
      comment names one level. `scales_the_longest_edge_to_the_thumb_edge` and
      `scales_a_tall_image_by_its_height` follow the constant, not a literal.
- [x] 1.4 `thumbs.rs`: `regenerate_all` per D4 with its doc comment (why per image off the
      lock, why the root check, why trashed rows are included). Tests:
      `regenerate_all_replaces_a_small_thumbnail_with_one_at_the_current_edge` (a 384 file
      written by hand, 768 after); `regenerate_all_counts_an_unreadable_image_and_continues`;
      `regenerate_all_stops_when_the_library_is_switched` (replace the `Mutex`'s library
      mid-pass from the progress closure, assert the pass returns short);
      `regenerate_all_reports_done_and_total`. Verify: `cargo test thumbs::` pass.
- [x] 1.5 `commands.rs` + `lib.rs` + `model.rs`: `THUMBS_PROGRESS_EVENT`, `ThumbsProgress`
      `{ done, total }`, `ThumbsReport { regenerated, failed }`, the `regenerate_thumbnails`
      command with the `AtomicBool` busy flag on `AppState` (D4); `thumbnail_path` answers
      `ThumbnailRef { path, version }` (D6) — `version` the file's mtime in ms, `0` if it
      cannot be read. Tests: `regenerate_thumbnails_refuses_a_second_run_while_one_runs`,
      `thumbnail_path_answers_the_file_s_version` (the shape of the existing
      `thumbnail_path` command test), both in `commands.rs`. Verify: `mise run check` green.

## 2. Unit W — the versioned URL, the store, the button (`packages/app`, `packages/shared`), after R

- [x] 2.1 `packages/shared`: `ThumbnailRef`, `ThumbsProgress`, `ThumbsReport`.
      `api/commands.ts`: `thumbnailPath(): Promise<ThumbnailRef>`, `regenerateThumbnails()`;
      `api/events.ts`: `onThumbsProgress`; invoke-shape tests in `commands.test.ts` and
      `events.test.ts` as the file does for its siblings.
- [x] 2.2 `api/assets.ts`: `thumbnailUrl` appends `?v=${version}` (D6) with the comment that
      says why; `thumbnail-cache.ts`: `forgetAll()`. Tests: `assets.test.ts` covers the
      query string.
- [x] 2.3 `api/thumbs-regenerate.svelte.ts` per D5 (`running`, `progress`, `report`,
      `error`, `start()`, `subscribe()`; `forgetAll()` on the last tick and on `start()`'s
      resolution; `reset()` on a library switch called from `frame/Sidebar.svelte`'s path
      effect beside `sidecarsBackfill.reset()`); exported from `api/index.ts`; subscribed
      from the layout beside `sidecarsBackfill`. Test `thumbs-regenerate.svelte.test.ts` the
      shape of `rebuild.svelte.test.ts`.
- [x] 2.4 `routes/settings/+page.svelte` Library section per D5: the button, the progress
      line, the report line. Verify: `mise run check` green.
- [ ] 2.5 Hand check (on a scratch copy of test-1, never the real one): opening the copy
      lifts every file one level and the grid renders; Regenerate thumbnails counts up,
      reports, and the grid's tiles sharpen without a restart at the 640 px tile; a second
      press while running is refused; switching libraries mid-pass stops it.

  Hand check: on a scratch copy of `boorubox-vault/test-1` (never the real folder), open it
  and confirm every file lifted one level and the grid renders; in Settings → Library, press
  Regenerate thumbnails and confirm the count climbs to the total, the report reads
  "Regenerated n", the grid's tiles sharpen without restarting the app; press the button again
  mid-pass and confirm it stays disabled/refused; open another library mid-pass and confirm
  the first library's pass stops (some old thumbnails remain) rather than erroring the app.
  Seen 2026-09-24 (smoke, scratch copy of test-1): before launch, `images/13/a6/<id>.jpg` and `.thumbs/13/a6/<id>.jpg`. After the first open, `images/13/<id>.jpg` and `.thumbs/13/<id>.jpg`, `find -mindepth 2 -type d` is empty for both, 113 files before and after, and the grid renders (/Users/shirohana/.claude/jobs/8dbef24a/tmp/01-grid-after-relayout.png). Regenerate thumbnails: the button reads Regenerating… and is disabled, and a second press did nothing. "0 of 55" climbed about 3 per 10s on the debug build (/Users/shirohana/.claude/jobs/8dbef24a/tmp/03-regen-running-crop.png, /Users/shirohana/.claude/jobs/8dbef24a/tmp/04-regen-report-crop.png). It ended with "Regenerated 54 — 1 could not be read" (/Users/shirohana/.claude/jobs/8dbef24a/tmp/05-regen-state-crop.png). The one failure is image b182c980…, whose blob is missing in test-1 itself (only its .json exists), so that failure is correct. The thumbnails went from 250×384 to 543×768. At 640px tiles, without a restart, the tiles are sharp (/Users/shirohana/.claude/jobs/8dbef24a/tmp/06-grid-640.png, /Users/shirohana/.claude/jobs/8dbef24a/tmp/06b-tile-640-detail.png). Not checked: switching libraries mid-pass.

## Handoff (unit W)

Landed on `packages/shared/src/index.ts`, `packages/app/src/lib/api/**`,
`packages/app/src/lib/components/library/thumbnail-cache.ts`,
`packages/app/src/routes/+layout.svelte`, `packages/app/src/lib/components/frame/Sidebar.svelte`
(one line: `thumbsRegenerate.reset()`), `packages/app/src/routes/settings/+page.svelte`.
`mise run check` green: 56 frontend test files / all passing (5 new in
`thumbs-regenerate.svelte.test.ts`, plus updated `commands.test.ts`, `events.test.ts`,
`assets.test.ts`), 729 Rust tests, clippy clean, lint/typecheck clean, both builds succeed.

Review fixes (Opus review of `0cbec82`): `thumbnail-cache.ts` is now
`thumbnail-cache.svelte.ts`, with a module-level `$state` `epoch` that `forgetAll()` bumps and
a `cacheEpoch()` getter; `ImageCard.svelte`'s and `SelectionThumbs.svelte`'s thumbnail effects
both call `cacheEpoch()` so a `forgetAll()` re-runs them for a card already on screen, and
`ImageCard`'s effect keeps the tile's current `src` (via a `lastPreviewId` closure var) until
the reload resolves rather than blanking the tile first. `thumbs-regenerate.svelte.ts` gained
a private `#runToken`/`#liveToken` pair: `reset()` bumps the token, `start()` returns early
while already running, and both `start()`'s own continuation and the progress-tick handler
check the token before writing, so a pass superseded by a library switch can no longer write
its report or a late tick into the newly-open library. Settings now shows
`thumbsRegenerate.error` beside the report line and gates the progress block on
`report || error || progress` rather than `running`, so a failed pass is visible on return and
no empty div sits on screen before the first tick.

Hand check: with a scratch library open, run Regenerate thumbnails, and mid-pass switch to
another library and back — confirm the grid's tiles (and the selection strip, if anything is
selected) still update to the new thumbnails once a second pass finishes, rather than staying
blank or on the stale image; also confirm a pass left running when you switch away and an
error from it doesn't retroactively show in the library you switched back to unless that pass
was actually still running there.

- Shared mirrors `ThumbnailRef { path, version }`, `ThumbsProgress { done, total }`,
  `ThumbsReport { regenerated, failed }` added to `packages/shared/src/index.ts`, matching
  `model.rs` field-for-field. `thumbnailPath()` now returns `ThumbnailRef`; added
  `regenerateThumbnails()` (no args) and `onThumbsProgress` (event `thumbs:progress`).
- `assets.ts`'s `thumbnailUrl` now does `convertFileSrc(path) + '?v=' + version` (D6).
  `thumbnail-cache.ts` gained `forgetAll()` (clears the id→url `Map`). Its own import of
  `thumbnailUrl` was changed from the `$lib/api` barrel to `$lib/api/assets` directly, to avoid
  a real import cycle: the barrel now also exports `thumbsRegenerate`, which imports
  `forgetAll` from `thumbnail-cache.ts`.
- New store `packages/app/src/lib/api/thumbs-regenerate.svelte.ts` (`ThumbsRegenerate` class,
  singleton `thumbsRegenerate`): `running`, `progress`, `report`, `error`, `start()`,
  `subscribe()`, `reset()`. One deliberate deviation from `Rebuild`'s shape (the stated visual
  precedent) worth flagging: `subscribe()` is **not** scoped inside `start()` the way
  `Rebuild.run()` owns its own `onRebuildProgress` listener. It is instead called once from the
  layout and kept for the life of the window, the same pattern as `sidecarsBackfill.subscribe`.
  Reason: D5/D6 need the thumbnail cache dropped "as soon as the last tick arrives" regardless
  of which screen is mounted, and a per-`start()` listener would miss a tick if Settings were
  ever left before the command settles. `forgetAll()` is called from two places for the reason
  written in the store's own comment: the progress handler on `done >= total` (fires as early
  as possible), and `start()`'s own `finally` (idempotent — the only drop a zero-image pass or
  a pass stopped mid-way by a library switch gets, since neither is guaranteed to reach a tick
  where `done >= total`).
- `frame/Sidebar.svelte`'s path effect now also calls `thumbsRegenerate.reset()` beside
  `sidecarsBackfill.reset()` — a minimal two-line edit (the import and the call), no other line
  in that file touched.
- Settings → Library: a "Regenerate thumbnails" outline button placed directly under the count
  `<dl>`/"In trash" line (before "Open the last library at launch"), disabled while running;
  below it, while running, "n of total" + a `Progress` bar, replaced once finished by
  "Regenerated n" (+ "— m could not be read" when `failed > 0`), cleared by the next run or a
  library switch. Kept inline in `+page.svelte` rather than extracted into a
  `RebuildStatus`-shaped component: the markup is small enough (no failures list, no `keptAs`)
  that a second component would just be indirection.
- Task 2.5 (hand check) intentionally left unticked; see the `Hand check:` line above it.

Unit R landed on `packages/app/src-tauri` only, `mise run check` green (Rust tests, clippy,
both builds). Final wire shapes unit W reads, all in `model.rs` with `#[serde(rename_all =
"camelCase")]`:

- `ThumbnailRef { path: String, version: i64 }` — `thumbnail_path(id) -> Result<ThumbnailRef>`
  now answers this instead of a bare path string; `version` is the thumbnail file's own
  modified time in milliseconds, `0` if it cannot be read. `assets.ts`'s `thumbnailUrl` is
  meant to append `?v=${version}` (D6).
- `ThumbsProgress { done: i64, total: i64 }` — payload of event `"thumbs:progress"`
  (`THUMBS_PROGRESS_EVENT` in `commands.rs`), emitted while `regenerate_thumbnails` runs.
- `ThumbsReport { regenerated: i64, failed: i64 }` — what the `regenerate_thumbnails` command
  resolves with.
- Command `regenerate_thumbnails()` — no arguments, spawns `thumbs::regenerate_all` against
  whichever library is open when the call starts. Refuses a second concurrent call with
  `AppError::Busy { reason }` (new `AppError` variant, serializes as a plain string like every
  other variant — the webview sees only the message, not a discriminant), read from
  `AppState.thumbs_regenerating: AtomicBool`.

Deviation from the design text: none in shape. One naming note — `library::all_image_ids` was
made `pub(crate)` (was private) so `thumbs::regenerate_all` could reuse the exact "every id,
trashed included" query `backfill_sidecars` already had, rather than a second copy of the same
one-line SQL.

`THUMB_EDGE` is `768` (was `384`); existing thumbnails on disk stay at their old size until a
user runs Regenerate — nothing migrates them automatically. `shard_dirs` is one level now
(`id[..2]`); `relayout_to_one_level` (was `relayout_to_buckets`) lifts both a flat library and
a two-level one into that shape on every open, idempotently.

Nothing in `packages/app/src/**` or `packages/shared` was touched. Task 2's own shared-package
mirror (`ThumbnailRef`, `ThumbsProgress`, `ThumbsReport`) still needs to be written by hand —
task 2.1's job, per the file's own FIXME on hand-mirrored types.
