## 1. Shared contract (`packages/shared`)

- [x] 1.1 Define `CaptureMeta` (id, imageUrl, pageUrl, pageTitle, capturedAt, adapter?), `SiteAdapterRecord` (`{ site, fields }`), `StatusResponse`, `ImageRecord`, `ImageSource` and `ParsedTagSearch`; verify `pnpm typecheck` passes and the app imports them.

## 2. Storage (`packages/app/src-tauri`)

- [x] 2.1 Add rusqlite (`bundled`, `fts5`), uuid, serde, thiserror; verify `cargo check` passes.
- [x] 2.2 Migration runner on `PRAGMA user_version` applying schema v1 from design D2, with `journal_mode=DELETE` asserted; verify a unit test opens a temp DB and reads back the schema and journal mode.
- [x] 2.3 `Library` type: open/create folder layout (`images/`, `inbox/`, `.thumbs/`, `library.sqlite`), sweep stray `inbox/*.part`; verify unit tests on a temp dir.
- [x] 2.4 Image write path per design D4 (part file, fsync, rename, row insert in one transaction, idempotent on id); verify a test that inserts the same id twice yields one file and one row.
- [x] 2.5 Query compiler from `ParsedTagSearch` to SQL (AND, OR groups, exclusions, rating, is:, tagcount:, account:, free text via FTS5); verify unit tests per operator against a fixture DB.
- [x] 2.6 Per-source counts and missing-file detection (mark `missing`, clear when file returns); verify tests.
- [x] 2.7 Thumbnail generation with the `image` crate into `.thumbs/<id>.jpg` on ingest and on demand; verify a test produces a thumbnail with the configured edge size.

## 3. App shell and settings

- [x] 3.1 Add tauri-plugin-dialog and tauri-plugin-store; settings file with `libraryPath` and `port`; verify settings round-trip in a test.
- [x] 3.2 Tauri commands: `pick_library`, `open_library`, `library_status`, `search`, `image_counts`, `drop_image_record`, `thumbnail_path`; verify each has a happy-path integration test through `tauri::test` or a direct call.
- [x] 3.3 Grant the asset protocol scope for the chosen library at runtime; verify an image renders in the webview via `convertFileSrc`.

## 4. HTTP listener

- [x] 4.1 axum router started in `setup()` on `127.0.0.1:<port>`; failure surfaced to settings state; verify a test binds the port twice and the second reports the error.
- [x] 4.2 Origin layer per spec `capture-ingest`; verify tests for `chrome-extension://` accept, `https://` reject, missing header reject, `/status` exempt.
- [x] 4.3 `POST /captures` multipart handler wired to 2.4; verify tests for new capture 201, retry 200, missing part 400, undecodable 422.
- [x] 4.4 `GET /status` (200 with library, 503 without); verify tests.

## 5. Local file import

- [x] 5.1 Import command taking paths, recursing folders, filtering decodable images, copying with fresh ids and `source=local`, metadata from the file; runs on a blocking thread with progress events; verify a test on a fixture folder counts imported and skipped.
- [x] 5.2 Drop-zone and file dialog in the UI with a running count; verify manually with a folder of more than 50 files.

## 6. Frontend (`packages/app/src`)

- [x] 6.1 Lift `tag-utils`, `filters`, `grouping`, `navigation-math` and their tests from the legacy repo into `src/lib/domain/`; verify `pnpm test` passes in the app package.
- [x] 6.2 `src/lib/api/` wrappers for every command from 3.2; verify typecheck.
- [x] 6.3 `/setup` route: folder pick and missing-library state per spec `library-folder`; verify manually through both scenarios.

  The "Library missing" scenario needed a fix, not just the screen. `Library::open_or_create`
  ran `create_dir_all` with no existence check, so a remembered folder that was deleted,
  renamed or unmounted came back as a new empty library and the app reported it open. Opening
  is now two intents: `Library::open_or_create` for a folder the user just chose, and
  `Library::open_existing`, which requires `library.sqlite` to be present — the folder alone
  is not enough, because an unmounted volume or an unsynced cloud folder can leave an empty
  directory at the path. `setup()` passes `OpenMode::ExistingOnly`; only picking a folder
  creates one. Verified in the app: a vanished path leaves `/status` 503, creates nothing on
  disk, and shows "Your library folder is missing" naming the path; picking the real library
  from that screen recovers and the stored path is replaced.

- [x] 6.4 `/` route: virtualised thumbnail grid ordered newest first, per-source counts panel, missing-file card state with drop action; verify with a generated library of 10k rows that scrolling stays smooth.
- [x] 6.5 Search box wired to `parseTagSearch` then the `search` command; empty state; verify the four `library-browse` search scenarios manually and by unit tests on the parser.
- [x] 6.6 Lightbox with arrow navigation and Escape using `navigation-math`; verify manually.

## 7. Verification

- [x] 7.1 `mise run check` passes (lint, typecheck, tests, clippy, builds).
- [x] 7.2 End-to-end: start `mise run dev`, pick a folder, `curl` a multipart capture with `Origin: chrome-extension://test`, see it in the grid, retry the same id, confirm one image; drop a folder of files; confirm per-source counts.

  Verified in the running app on 2026-09-06: first launch with no
  remembered library shows `/setup`; the folder picker opens and picking a folder opens the
  library and writes `settings.json`; `GET /status` is 503 before and 200 after; a capture
  posted with `Origin: chrome-extension://test` is 201, the same id retried with different
  bytes is 200 with one image on disk, a web-page `Origin` and a missing `Origin` are both
  403, a missing part 400, undecodable bytes 422; the capture renders in the grid; importing
  a folder of 58 images and 2 non-images through the picker reports `imported 58 · skipped 2
  · failed 0` and names both skipped files; per-source counts read total 59 / extension 1 /
  local 58 / legacy-bundle 0 and do not move when the search does; free text finds one image
  by page title through FTS5; a no-match search shows the empty state naming the query; the
  lightbox opens, the right arrow advances (`2 of 59`), Escape closes it; deleting a file
  under `images/` flips the record to missing and shows the "File not found — Remove record…"
  card, and restoring the file clears it.

  Dropping files and folders on the window was verified by the owner on 2026-09-06, which
  closes the last gap; GIFs were accepted. The stored file is the bytes as delivered — ingest
  writes them through `inbox/<id>.part` and renames, never re-encoding — so an animated GIF
  keeps its animation. Its thumbnail is a still JPEG of the first frame, which is what the
  grid shows.
