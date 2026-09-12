> Three implementing agents. **A** (group 1) and **B** (group 2) own
> `packages/app/src-tauri/**` and run **in order — B builds on A's `sidecar.rs`**. **C**
> (group 3) owns `packages/shared/src/**` and `packages/app/src/**` and runs alongside B.
> Group 4 is the lead's. `docs/requirements.md` §7 and §10 are already amended by this
> change's planning — no task touches them.
>
> The contract the three share, pinned here so nobody waits on anyone:
>
> - **Sidecar file**: `images/<a1>/<b2>/<id>.json`, pretty-printed, `None` fields omitted,
>   tags sorted. Keys: `version` (`1`), `id`, `ext`, `mime`, `size`, `width`, `height`,
>   `source`, `sourceRef`, `imageUrl`, `pageUrl`, `pageTitle`, `adapter`, `rating`, `tags`,
>   `capturedAt`, `fileModifiedAt`, `createdAt`, `updatedAt`, `deletedAt`, `posts`
>   (`[{ site, remoteId, postedAt }]`). No `file`, no `missing` (design D2).
> - **Library file**: `<library>/library.json`, keys `version` (`1`), `rules`, `booruSites`,
>   `note` (`{ content, updatedAt }`). Rule and site ids verbatim. No credential, ever.
> - **Command**: `rebuild_library(path: String) -> Result<RebuildReport>`. Closes the library
>   if that path is the one open; leaves the rebuilt path closed and a different open library
>   untouched — that library is unrelated to the rebuild, and closing it would tear down a
>   frame nobody asked about; the caller opens the rebuilt path afterwards through the
>   existing `open_library`.
> - **Events**: `library:rebuild` → `{ done: number, total: number }`;
>   `library:sidecars` → `{ done: number, total: number }`.
> - **`RebuildReport`** (serde camelCase both sides): `{ images: number, failed: number,
>   failures: { file: string, reason: string }[], keptAs: string, rules: number,
>   sites: number }`.
> - **`LibraryStatus`** gains `damagedPath: string | null` — set only when the remembered
>   library failed to open *because it is damaged* — and `newerPath: string | null`, set only
>   when it was written by a newer build; `missingPath` keeps every other case, and exactly
>   one of the three is ever non-null. A newer library needs its own slot rather than a
>   silence: with all three null the start screen falls through to "Choose a library folder"
>   and never names the folder.
> - **No new `MIGRATIONS` entry** and no schema version claimed (design D7). If one turns out
>   to be needed, its number is `MIGRATIONS.len()` in `db.rs` at the moment it is written,
>   never a number pinned here.

## 1. Rust: the sidecar format and every write path (agent A)

- [x] 1.1 `packages/app/src-tauri/src/sidecar.rs` (new): the `Sidecar` type with the pinned
      shape (design D1, D2), `path(paths, id)` beside `thumbs::thumbnail_path`'s precedent,
      `read(path)`, and `write(paths, sidecar)` going through `inbox/<id>.json.part` and a
      rename, with no fsync (design D6). Verify: a sidecar round-trips through
      `write`/`read` with every field intact including a `None` one; writing the same
      sidecar twice produces identical bytes; a `.part` left behind by a killed write is
      removed by `Library::sweep_inbox` (assert through `Library::open_or_create`).
- [x] 1.2 `sidecar.rs`: `write_for(paths, conn, ids)` — one `ingest::load_records` call for
      the whole slice, then one file per id — and `remove_for(paths, id)`. Verify: a slice of
      three ids costs one `load_records` call and leaves three files; an id with no row
      writes nothing rather than failing.
- [x] 1.3 `sidecar.rs`: a test that reads `pragma_table_info('images')` and asserts every
      column is represented in `Sidecar`, with `missing` the single named exception (design
      D11's drift risk). This is the guard that makes the next migration fail the suite
      rather than the next rebuild.
- [x] 1.4 `sidecar.rs`: `write_library(paths, conn)` — the `library.json` of the pinned shape
      from `rules::list`, `booru::sites::list` and `notes::get`, same temp-then-rename
      (design D3). Verify: a library with two rules, one site and a note round-trips through
      `read_library`; the file contains no API key even when one is stored for the site
      (use `InMemoryCredentials`, assert the serialised text does not contain the key).
- [x] 1.5 `packages/app/src-tauri/src/ingest.rs`: `store_image` writes the sidecar after its
      transaction commits, and the `Ingested::Existing` early return ensures one too (design
      D4). A sidecar write failure fails the call. Verify: a stored image has its sidecar
      beside its file with the right tags and rating; deleting the sidecar and redelivering
      the same id writes it again and still reports `Existing` without a second file;
      an unwritable `images/` bucket makes `store_image` return `Err`.
- [x] 1.6 `packages/app/src-tauri/src/tags.rs`: `update_tags`, `set_rating`,
      `bulk_update_tags` and `bulk_set_rating` each rewrite the sidecars of the ids they
      touched, after the commit, through `sidecar::write_for` (design D4, D5). Verify: a tag
      edit and a rating change are both visible in the file afterwards; a bulk add over three
      ids rewrites all three; a `rating:e` typed in the tag box lands as `rating` in the
      file and not as a tag.
- [x] 1.7 `packages/app/src-tauri/src/trash.rs`: `trash_images` and `restore_images` rewrite
      the sidecars; `delete_forever` removes each sidecar **before** its image file and names
      one that will not go in `DeleteReport.files_left` (design D5). Verify: a trashed
      image's file records `deletedAt` and a restored one clears it, with the file never
      removed by either; `delete_forever` leaves no sidecar; `empty_trash` over three images
      leaves none of the three.
- [x] 1.8 `packages/app/src-tauri/src/booru/posts.rs`: `record` rewrites the image's sidecar
      after its commit. Verify: the file lists the post with its site, remote id and time.
- [x] 1.9 `packages/app/src-tauri/src/rules.rs`: `apply_rules_to_image` rewrites that one
      image's sidecar inside a run (design D5), and `upsert`, `delete` and `import_json`
      rewrite `library.json`. Verify: a run over three images where one matches rewrites
      exactly that one image's sidecar (compare mtimes or bytes of the other two); saving,
      disabling and deleting a rule are each visible in `library.json`.
- [x] 1.10 `packages/app/src-tauri/src/notes.rs` and `booru/sites.rs`: `notes::set`,
      `sites::save` and `sites::delete` rewrite `library.json`. Verify: the note's text and
      the site list are in the file after each; removing a site removes it from the file.
- [x] 1.11 `packages/app/src-tauri/src/maintenance.rs`: assert the opposite — a test that
      `refresh_missing_for` writes no sidecar (design D2), naming why in the test's own
      comment so a later reader does not "fix" it.
- [x] 1.12 `pub mod sidecar;` in `lib.rs`. `cargo fmt`, `cargo test` and
      `cargo clippy --all-targets -- -D warnings` all clean.

> **Handoff to B.** Group 1 landed as specified, with two deviations:
>
> - **`notes::set` now takes `&Library`, not `&Connection`** (`content: &str` unchanged) —
>   task 1.10 needs `paths` to rewrite `library.json`, and `notes.rs` had no way to reach
>   `LibraryPaths` from a bare connection. The one call site, `commands::note_set`, is updated
>   to `notes::set(library, &content)` — a one-line, mechanical change (no logic touched) made
>   to keep the gate green rather than left broken for you; flagging it since `commands.rs` is
>   your file from here. `notes::get` is untouched (`&Connection`, read-only, never writes
>   `library.json`).
> - **Task 1.5's "an unwritable `images/` bucket makes `store_image` return `Err`"** is tested
>   as `ingest::tests::a_sidecar_write_failure_fails_the_call_even_though_the_row_is_committed`:
>   rather than chmod-ing the bucket (which would fail the image's own write first, before
>   ever reaching the sidecar), it pre-creates a directory at the sidecar's own destination
>   path so `sidecar::write`'s rename onto it fails deterministically on every platform, after
>   the row has already committed. Same assertion the task asks for (`store_image` returns
>   `Err`, the row still lands); just a more direct way to isolate the sidecar's own failure.
>
> New in `sidecar.rs` for you to build on (`recover.rs`'s rebuild and `library.rs`'s backfill,
> task 2.4/2.7): `pub struct Sidecar` and `pub struct LibraryFile` (both pinned-shape,
> `PartialEq`/`Clone`/`Serialize`/`Deserialize`), `pub fn path(paths, id) -> PathBuf`,
> `pub fn library_path(paths) -> PathBuf`, `pub fn read(path: &Path) -> Result<Sidecar>`,
> `pub fn write(paths, sidecar: &Sidecar) -> Result<()>`,
> `pub fn write_for(paths, conn, ids: &[String]) -> Result<()>`,
> `pub fn remove_for(paths, id: &str) -> Result<()>`,
> `pub fn read_library(path: &Path) -> Result<LibraryFile>`,
> `pub fn write_library(paths, conn) -> Result<()>`. `Sidecar` implements `From<&ImageRecord>`.
> A malformed sidecar or library file fails `read`/`read_library` as `AppError::BadRequest`
> (nothing more specific existed on this half; add a variant in `error.rs` if the rebuild wants
> to tell that case apart from others in its failure report).
>
> Two existing tests elsewhere (not group 1's files, so left as found until you touch them)
> now count the sidecar among a bucket's files: none needed changing beyond what's already
> fixed here (`ingest.rs`, `trash.rs`), but a future test asserting a bucket holds exactly the
> image files it expects will need to account for one sidecar per image.
>
> `cargo fmt`, `cargo test` (471 passed), `cargo clippy --all-targets -- -D warnings` and
> `mise run lint` are all green on this half.

> **Review (group 1, Opus).** Three changes, no behaviour change to any write path:
>
> - **The column-coverage guard (1.3) now reads `Sidecar`'s own serialisation**, not a list
>   of field names kept beside it: a list is a second spelling of the struct, and the cheapest
>   way past a failure would have been to add the new column's name to the list — the drift
>   D11 asks this test to catch. Every `Option` is filled in before serialising, since a
>   `None` field is omitted from the file and would read as absent. Proved by adding a fake
>   `sha256` column to the pragma result: the test fails with
>   `images.sha256 has no representation in Sidecar`.
> - **`sidecar::write_one(paths, conn, id)` added** — `write_for` over a slice of one, spelled
>   once instead of at four call sites (`tags::update_tags`, `tags::set_rating`,
>   `booru::posts::record`, `rules::apply_rules_to_image`), each of which had its own copy of
>   the `slice::from_ref(&id.to_string())` trick and two of which had an identical private
>   `write_sidecar` helper. New in the API list above for B.
> - **`ingest::store_image` writes from the record it already holds** (`sidecar::write` +
>   `Sidecar::from`), on both outcomes, rather than re-reading the row it just wrote:
>   `write_for`'s three statements per image are pure cost on the one door a 25,000-image
>   bundle import comes through, and the sidecar cannot then disagree with the record the
>   caller is handed back.
>
> Task 1.4's round-trip test carried one rule where the Verify line says two; it now creates
> two. Everything else checked against D1–D7, D14 and the spec held: no sidecar written inside
> or before a commit, every D5 path present (including the `Existing` early return and
> `delete_forever` removing the sidecar before the image and naming a stuck one in
> `files_left`), no `file`/`missing` in the sidecar, no credential in `library.json`, no fsync,
> `.part` through `inbox/` that `sweep_inbox`'s `part` extension matches, and
> `refresh_missing_for` still writing none. `cargo fmt`, `cargo test` (471 passed) and
> `cargo clippy --all-targets -- -D warnings` are green.

## 2. Rust: detection, the backfill and the rebuild (agent B, after group 1)

- [x] 2.1 `packages/app/src-tauri/src/error.rs`: `AppError::LibraryCorrupt { path }`, and the
      mapping from rusqlite's corrupt-class codes (`DatabaseCorrupt`, `NotADatabase`) so the
      same message is given wherever the damage surfaces (design D8). Verify: a rusqlite
      error carrying each code converts to `LibraryCorrupt`; `SchemaTooNew` and `JournalMode`
      are unchanged.
- [x] 2.2 `packages/app/src-tauri/src/db.rs`: `PRAGMA quick_check(1)` in `open`, before the
      journal-mode check, failing with `LibraryCorrupt` (design D8). Verify: a database with
      bytes overwritten mid-file is refused with `LibraryCorrupt` and the file is left exactly
      as it was (assert its bytes afterwards); a healthy library still opens and every
      existing `db.rs` test still passes.
- [x] 2.3 `packages/app/src-tauri/src/recover.rs` (new): `move_aside(paths)` — the database
      and any `-journal` to `…​.corrupt-<now_ms>` in one step, never deleting either (design
      D10). Verify: both files move and both are still in the folder afterwards; a second
      call leaves the first pair untouched and makes a second pair.
- [x] 2.4 `recover.rs`: `rebuild(paths, on_progress) -> RebuildReport` per design D11 — build
      into `library.sqlite.rebuilding` through `db::open`, walk `images/**/*.json`, insert
      each row with its stored timestamps and `missing` computed from the image file's
      presence, link tags through `tags::link_tag`, insert posts, apply `library.json`, then
      `move_aside` and rename into place. Verify the round trip end to end: build a library
      with captures, hand tags, a rating, a trashed image, a recorded post, two rules, a site
      and a note; snapshot what `search`/`load_records`/`rules::list`/`notes::get` answer;
      corrupt the database; rebuild; assert the same answers, including a tag search and a
      full-text search (the FTS triggers, design D11).
- [x] 2.5 `recover.rs`: the failure paths. Verify: a truncated sidecar is counted in
      `failed`, named in `failures`, left on disk, and every other image still comes back; an
      image file with no sidecar creates no row and is not moved; a sidecar whose image file
      is gone comes back as a row with `missing` true; no image or thumbnail byte or mtime
      changes across a rebuild (compare a full tree snapshot); a stale `library.sqlite.rebuilding`
      from an interrupted run is overwritten rather than adopted.
- [x] 2.6 `packages/app/src-tauri/src/model.rs`: `RebuildReport`, `RebuildFailure` and the two
      progress payloads of the pinned shape, and `LibraryStatus.damagedPath`. Verify: the
      serde field names match the pinned contract exactly (a serialisation test, since this
      is the hand-mirrored half of `packages/shared`).
- [x] 2.7 `packages/app/src-tauri/src/library.rs` + `sidecar.rs`: the backfill —
      `SELECT id, ext FROM images`, a file check per row, write the ones missing, plus
      `library.json` when absent (design D7). It takes the library per image through
      `with_library` and stops when the open library's root is no longer the one it started
      for. Verify: a library whose sidecars were deleted gets exactly those written again and
      a second run writes nothing; a pass told to run against a library that has since been
      switched stops without writing into the new folder; a 500-row library's pass leaves
      every sidecar matching what `write_for` would have written.
- [x] 2.8 `packages/app/src-tauri/src/commands.rs`: start the backfill from `open_into_state`
      on a blocking thread, emitting `library:sidecars`; record the open failure in
      `AppState.open_failure` on both outcomes and report `damagedPath` from `status`
      (design D9). Verify: opening a library with missing sidecars emits progress and does not
      block the command; a damaged library leaves `damagedPath` set and `missingPath` null; a
      folder that is simply gone leaves `missingPath` set and `damagedPath` null; a
      `SchemaTooNew` library sets `newerPath` and neither of the others; opening a healthy
      library clears all three.
      Note: this line read "sets neither" until verification found the gap — a library from a
      newer build reported no path at all, so the start screen offered "Choose a library
      folder" without naming it, against `library-recovery`'s "A library from a newer build".
- [x] 2.9 `commands.rs` + `lib.rs`: the `rebuild_library` command of the pinned contract,
      emitting `library:rebuild`, registered in `generate_handler!`. Verify: rebuilding the
      open library closes it and leaves nothing open; rebuilding a path that is not open
      works with no library open at all; the progress event reaches a listener (assert with
      the mock app, as the import-progress tests do).
- [x] 2.10 `cargo fmt`, `cargo test`, `cargo clippy --all-targets -- -D warnings` clean.

> **Handoff to C / the lead.** Group 2 landed as specified. Group 3 had already landed by the
> time this note was written (`packages/shared`'s `RebuildReport`/`RebuildFailure`/`damagedPath`,
> `commands.ts`'s `rebuildLibrary(path)`, `events.ts`'s `library:rebuild`/`library:sidecars`) —
> checked against this half and everything agrees: the command name, its one `path` argument,
> `RebuildReport`'s field names including `keptAs`, and both event names all match verbatim.
> One thing worth naming for whoever reads this record: `packages/shared` mirrors both progress
> events as the existing generic `ExportProgress { done, total }` shape rather than two named
> types; `model.rs` here defines `RebuildProgress` and `SidecarsProgress` as two distinct structs
> (per task 2.6's "the two progress payloads") that happen to serialise to the identical shape.
> No wire mismatch either way — only the Rust-side type name differs from the TS-side type name
> for the same JSON.
>
> **Constraint from the webview reviewer, applied here:** the pending-work tile hides only once
> `done >= total` and shows for *any* `library:sidecars` event with `total > 0`, so `total` had
> to become "rows still lacking a sidecar" (after the file check) with **no event emitted at all**
> when that count is zero — not even a `(0, 0)` tick — or a library already in step would flash a
> tile on every open. `library::backfill_sidecars` now only calls `on_progress` when there is
> something to write; covered by `library::tests::a_library_already_in_step_emits_no_sidecars_event`
> and the "second pass" half of `a_library_whose_sidecars_were_deleted_gets_exactly_those_written_again`.
>
> `cargo fmt`, `cargo test` (497 passed) and `cargo clippy --all-targets -- -D warnings` are green
> on this half; `mise run lint` is green on the merged tree.

> **Review (group 2, Opus).** Six changes; detection, the backfill and the report shape held
> as built.
>
> - **The corrupt-class mapping reached only `db::open`.** Design D8 asks for it "wherever
>   they arrive — at open or mid-session", and mid-session every rusqlite error takes `?`'s
>   `From` road to `Db` before anyone can name the file. `AppError::or_corrupt(path)` is the
>   second door onto the same predicate, applied in `library::with_library_if_open` — the one
>   frame every command and every capture passes through that knows which file it was.
> - **A row the database would not take ended the whole rebuild.** Two sidecars carrying one
>   id — the "conflicted copy" D15 says this change *will* produce — made the second insert a
>   `UNIQUE` failure that aborted the run, and would abort it again on every retry: the one
>   library most needing a rebuild would be the one that could not have one. Each sidecar's
>   insert now runs in a savepoint and a refusal is counted and named like an unreadable file.
>   `walk_sidecars` orders shorter filenames first so the row comes from `<id>.json` rather
>   than from a copy (a copy's name can only be longer), which is the file every later write
>   path rewrites.
> - **`move_aside` failed on a folder whose `library.sqlite` was already gone** — the goal
>   sentence of this whole change — throwing away a rebuild that had already done its work,
>   every time. It now keeps aside what is there and reports `keptAs: ""` when there was
>   nothing; the journal goes aside either way, since a stale one left beside the database
>   being renamed into place is what SQLite would roll back into it. *For the lead:*
>   `RebuildStatus.svelte` prints "The previous database is kept as ." for that empty string —
>   a one-line `{#if report.keptAs}`, left to whoever owns the webview.
> - **`missing` was computed from `LibraryPaths::image_path`**, so a flat pre-bucket library
>   rebuilt every row as missing. D11 spells the check as "is `<id>.<ext>` beside this
>   sidecar"; it is now asked of the sidecar's own directory.
> - **An unreadable `library.json` was swallowed**, silently costing the rules, the sites and
>   the note while the report said zero. It is counted and named now; an *absent* one stays
>   silent, which is just a library from before this change.
> - **`rebuild_library` tore the library down without `cancel_running_import`**, the one thing
>   that function's own note says every such place does first: an import left running would
>   write the rest of its rows into the database the rebuild is about to move aside.
>
> Checked and left alone: `quick_check` before anything else touches the file and the refused
> open byte-identical; `SchemaTooNew`/`JournalMode` still distinct and reporting neither path;
> the rebuild through `db::open` for one schema definition, overwriting a stale `.rebuilding`,
> moving the old database aside only once the new one is complete; timestamps, rule and site
> ids, the note and `rating` restored verbatim; tags through `tags::link_tag`; FTS by trigger;
> chunked commits; nothing holding the mutex across an `.await`; a capture arriving mid-rebuild
> answered non-2xx, so it stays `failed` with its blob. One thing judged not worth changing:
> between a rebuild and the caller's reopen, `status` reports the remembered path as
> `missingPath` (`close_library` forgets the path, a rebuild must not) — both callers reopen
> at once and nothing polls status on a timer.
> `cargo fmt`, `cargo test` (504 passed) and `cargo clippy --all-targets -- -D warnings` green.

## 3. The webview: the damaged library, the rebuild and the catch-up tile (agent C)

- [x] 3.1 `packages/shared/src/index.ts`: `RebuildReport`, `RebuildFailure`, the two progress
      payloads, and `damagedPath: string | null` on `LibraryStatus`, matching the pinned
      contract. Verify: `pnpm typecheck` passes across both packages.
- [x] 3.2 `packages/app/src/lib/api/commands.ts`: a `rebuildLibrary(path)` wrapper invoking
      `rebuild_library`, with tests beside the existing command tests.
- [x] 3.3 `packages/app/src/lib/api/events.ts` + a small store for the rebuild: subscribe to
      `library:rebuild`, expose done/total and the report. Verify with tests; remember vitest
      compiles runes server-side, so never compare a `$state` field against an object literal.
- [ ] 3.4 `packages/app/src/routes/start/+page.svelte`: when `damagedPath` is set, the damaged
      state — the folder named, what a rebuild does, that the current database is kept — with
      a Rebuild action, progress while it runs, then the report (images restored, files that
      could not be read, the name the old database was kept under) and a button that opens the
      library. Verify with component tests: the damaged wording is shown for `damagedPath` and
      the missing wording for `missingPath`, never both; the report is shown before the
      library opens. Derive anything read off `library.status` with `$derived`, never an
      `$effect`.
      Hand check: no `@testing-library/svelte` (or any renderer) is installed anywhere in this
      repo and none was added (boundary: no new dependencies), so this was implemented but not
      verified by a component test — `rebuild.svelte.ts`'s own state machine is (see its test),
      the rendering and the wiring are not. Open a library whose `library.json`/database
      disagree (or stub `damagedPath` in `LibraryStatus`) and confirm: the damaged wording
      shows, never the missing wording; Rebuild shows progress, then the report (images
      restored, any unreadable files, the kept name); Open library opens it via the normal
      guard; a second damaged folder afterwards starts clean.
- [ ] 3.5 `packages/app/src/routes/settings`: a Library section with "Rebuild library index",
      behind a confirmation naming what is kept, calling the same wrapper and reopening the
      library afterwards. Verify: the control is absent with no library open; confirming
      rebuilds and reopens; declining does nothing.
      Hand check: same gap as 3.4 — no renderer to prove "absent with no library open" or the
      confirm/decline behaviour against real markup. `canRebuild` (`library.status?.opened`)
      and the confirm-dialog wiring are implemented; the manual check is opening Settings →
      Library, confirming the button and description are there, declining the dialog does
      nothing, and confirming rebuilds, reopens the same path and shows no leftover progress.
- [x] 3.6 The pending-work band: one tile for `library:sidecars` showing done of total, gone
      when the pass finishes, leaving no result to dismiss, and absent for a library already
      in step. Verify with tests alongside the existing band tests.
- [x] 3.7 `pnpm -r test`, `pnpm typecheck` and `pnpm lint` pass.

> **Handoff to the lead.** Group 3 landed as specified, with one deviation:
>
> - **Tasks 3.4 and 3.5's "component tests" do not exist.** No Svelte component renderer
>   (`@testing-library/svelte`, `vitest-browser-svelte`, anything) is installed in this repo —
>   checked before writing anything: zero `*.svelte` files are rendered by any existing test,
>   only `*.svelte.ts` stores are. Adding one would be a new dependency, out of bounds for this
>   agent. Both routes are implemented to spec (damaged-vs-missing wording, Rebuild → progress →
>   report → Open on `/start`; a confirmed, guarded "Rebuild library index" on `/settings`), and
>   everything below the template — `rebuild.svelte.ts`'s progress/report/error state machine,
>   `sidecars-backfill.svelte.ts`'s tile lifecycle, the two commands, the two events — is
>   covered by `vitest`. The markup and click-wiring on both routes are left as `Hand check:`
>   lines above rather than ticked, per this run's boundary against claiming a check nothing
>   ran.
>
> Also worth knowing, not a deviation: the two `{ done, total }` progress payloads
> (`library:rebuild`, `library:sidecars`) are not given their own named types in
> `packages/shared` — they reuse `ExportProgress`, the same way `rules:progress` already does
> in `events.ts`. TypeScript is structural, so this matches the pinned contract exactly; group
> 2 gave the Rust mirrors their own `RebuildProgress`/`SidecarsProgress` names (seen in
> `model.rs` at `cargo fmt --check` time), which is fine — nothing requires the two sides to
> share a type name, only a shape.
>
> `pnpm -r test` (555 passed across `shared`, `app`, `extension`), `pnpm typecheck` and
> `pnpm lint` are all green. `mise run lint`'s Rust half (`cargo fmt --check`) was still
> reporting group 2's in-progress files at the time of this handoff — not this group's to fix.

> **Review (group 3, Opus).** Four changes, all in the two routes and one new
> component; the stores, the events and the shared types held as built.
>
> - **A failed rebuild from Settings left the frame describing a closed library.**
>   `rebuild_library` leaves nothing open whether it worked or not (the pinned contract),
>   but `confirmRebuild` returned early on failure, so the sidebar, the counts and the grid
>   behind them went on showing a library Rust had closed and every command behind them
>   would have answered that none is open. The reopen now runs on both outcomes; a reopen
>   that also fails refreshes the status, which sends the layout's gate to `/start` — where
>   D9's `damagedPath`, recorded by that failed open, offers the rebuild again.
> - **The rebuild's report is now shown by both callers, and the store is no longer left
>   holding one.** Settings showed nothing at all after a rebuild — no count, no kept-aside
>   name, and no progress for the minutes a 25,000-image rebuild takes, which is the screen
>   `library-recovery`'s "rather than appear stalled" and "SHALL name the kept file where the
>   user can read it" both rule out. The report/progress markup moved out of
>   `/start` into `lib/components/common/RebuildStatus.svelte` and both routes render it.
>   Settings keeps its own copy and clears `rebuild.report`, so a folder met as damaged later
>   cannot open the start screen on some earlier rebuild's report; `/start` now clears it only
>   once the library actually opened, since a failed open would otherwise drop the kept-aside
>   name that cannot be asked for again.
> - **The start screen's other doors are shut while a rebuild runs** (`blocked`): the recent
>   entries, Choose folder and Forget were all live, and opening a library while the rebuild
>   moves a database aside and renames another onto it is the second writer this change
>   exists to keep out.
> - **The failures list is keyed by position and capped** (`max-h-32`, scrolled, like the
>   export report's): keyed on `file`, two failures naming one file would crash the panel
>   whose whole job is explaining the damage, and an uncapped list would push the screen off
>   itself. Rules and sites are shown in the report too, at zero as well — a folder that lost
>   `library.json` rebuilds every image and no rule, and this is where that is visible.
>
> Also: `ExportProgress`'s doc in `packages/shared` now names the four events that share it,
> so the next field added for the export's sake is not added blind; `ConfirmDialog`'s header
> comment counts its fifth caller. Reused `ExportProgress` for the two new payloads rather
> than declaring them in `packages/shared` (task 3.1): `rules:progress` set that precedent and
> two more identical interfaces would be a shape spelled three times — the doc above is the
> guard that makes the reuse safe. One coordination risk for group 2, not fixable here: the
> tile hides itself only on `done >= total`, so "a library already in step shows no tile at
> all" holds only if the backfill emits **nothing** for a library that needs no writes — a
> tick per row checked would put a tile on screen for a library in step.
> `pnpm test` (462), `pnpm -r test` (555), `pnpm typecheck` and `pnpm lint` green.

## 4. Lead

- [x] 4.1 Review each agent's unit before the next touches its files; `mise run check` green
      on the merged result.
- [x] 4.2 `docs/storage.md` (new, prose, the lead's own): what is in a library folder and why
      each part is there, what the describing files are for, what a rebuild does and what it
      cannot recover, when a cloud-synced folder is a fit (one machine at a time) and when it
      is not, and what to do when the app says a library is damaged. Written for the user, not
      the implementer; linked from the README.
- [x] 4.3 `README.md` (prose, the lead's own): a status paragraph — alpha, installers are
      unsigned on both platforms, libraries migrate forward on their own and this build adds
      the describing files on first open, where to report a problem.
- [ ] 4.4 Hand check: open the library in the owner's Synology Drive folder, let the backfill
      run, confirm the tile appears and the app stays usable; then use Settings → Rebuild
      library index on a copy of that folder and confirm the rebuilt library matches the
      original and the old database is still there under its kept name. Left for the owner —
      an agent that cannot reach that folder does not tick this.
- [x] 4.5 Commit, leaving `main` releasable; do not tag.
