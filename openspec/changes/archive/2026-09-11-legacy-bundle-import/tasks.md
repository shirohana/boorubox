> Lands after `sharded-image-dirs` (both edit `ingest.rs`). Two implementing agents: **A**
> owns group 1 (Rust; `packages/shared` needs nothing new — design D6 reuses `ImportReport`),
> **B** owns group 2 (`packages/app/src`). They share one contract, pinned here so they can
> run side by side: the Tauri command is `import_bundle`, its argument is `files: Vec<String>`
> (absolute paths of `.db` parts), it answers `ImportReport`, and it emits `import:progress`
> exactly as `import_paths` does. B's wrapper in `src/lib/api/commands.ts` is
> `importBundle(files: string[]): Promise<ImportReport>` invoking `import_bundle` with
> `{ files }`. Group 3 is the lead's.

## 1. Rust: the bundle reader and the command

- [x] 1.1 Pin the bundle format: `packages/app/src-tauri/fixtures/legacy-bundle/database.db`
      (four real rows: two live, one deleted with rating `e`, one with a four-byte blob) and
      its `manifest.json`, layout documented in design.md. Done by the lead, 2026-09-10.
- [x] 1.2 `packages/app/src-tauri/src/ingest.rs`: `IngestInput.deleted_at: Option<i64>`
      written into `images.deleted_at` by the one insert (design D3); every existing caller
      passes `None`; verify a test that a `Some` input stores a deleted row that a `Library`
      search does not return and a `Trash` search does.
- [x] 1.3 `packages/app/src-tauri/src/bundle.rs`: `import_bundle(library: &SharedLibrary,
      files: &[PathBuf], on_progress) -> Result<ImportReport>` per design D1, D5, D6, D8 —
      parts ordered by `part<N>` then name, rows by `rowid`, read-only connection per part,
      `total` counted first, one `with_library` per row, thumbnail warmed outside, `source_ref`
      the parent folder's name; verify tests on the fixture: clean import gives 3 imported /
      0 skipped / 1 failed with a decode reason and the library holds 3 images with
      `source = legacy-bundle`; re-import gives 3 skipped / 1 failed and still 3 images; the
      deleted row has `deleted_at` set and is absent from a `Library` search; the
      `honkai:_star_rail` row is found by that tag; the `e` row by `rating:e`; `captured_at`
      equals the row's `savedAt`; `path` is the page URL and `id` is set on skipped items too;
      a non-SQLite file among `files` is one failed item and the others still import; an
      unreadable `tags` value (test-built row) fails that row alone.
- [x] 1.4 `packages/app/src-tauri/src/commands.rs` + `lib.rs`: the `import_bundle` command
      (contract above) on `off_main_thread` emitting `IMPORT_PROGRESS_EVENT`, registered in the
      handler list; `model.rs`'s `ImportOutcome.id` comment widened per D6; verify `cargo test`
      and `cargo clippy` green.

## 2. Webview: the `/import` route and the bundle run

- [x] 2.1 `packages/app/src/lib/api/commands.ts`: `importBundle` per the contract, with the
      same mocked-invoke test the other wrappers have; `src/lib/api/dialog.ts`: a multi-select
      picker for `.db` files (`sqlite` / `db` filter); verify `commands.test.ts` covers the
      wrapper.
- [x] 2.2 `packages/app/src/lib/api/imports.svelte.ts`: a second run kind (design D6) — the
      queue entry records whether a run is paths or bundle files, `#execute` calls the matching
      command, and the store exposes the latest bundle report for the route (the library
      band's report list may keep showing bundle reports too; one list, no second queue);
      verify `imports.svelte.test.ts` covers a bundle run's progress, its report landing, and
      that a paths run and a bundle run go one after the other.
- [x] 2.3 `packages/app/src/routes/import/+page.svelte` (design D7): what to pick, the
      picker, the progress of a running bundle import, the latest bundle report grouped by
      status with failed and skipped open and imported collapsed behind its count, then the
      total and `legacyBundle` counts from `imageCounts` with the §9 notice text verbatim
      ("Compare these numbers and spot-check a few images before removing anything from the
      browser. This app cannot verify the browser's data for you."); no wording that says the
      browser data is safe to delete; `Sidebar.svelte` gains the entry between Trash and
      Settings; verify a component test that the report renders the three groups and the
      counts line, and `mise run lint` + `typecheck` green. Manual verification is group 3's.
      Note: this repo has no Svelte component-render test harness (no `@testing-library/svelte`
      or equivalent) — every existing "component test" in the codebase tests logic extracted to
      a plain `.ts` module beside the component (`group-label.ts`, `trash-actions.ts`, etc.).
      Followed that pattern: `src/lib/components/import/bundle-report.ts` extracts
      `groupBundleReport` (the three groups) and `bundleCountsLine`/`BUNDLE_MIGRATION_NOTICE`
      (the counts line and notice verbatim), tested in `bundle-report.test.ts`; the route
      renders them. Hand check: the owner opens `/import` and confirms the page actually
      renders the groups and counts line as the extracted logic says it should — group 3's
      task 3.2 covers this against a real bundle.

## 3. Lead: wiring, gate, live run

- [x] 3.1 `mise run check` green on the wired tree (2026-09-11: vitest 386, cargo 427, clippy clean, both builds).
- [x] 3.2 Live run through the drive-app probe against a fresh library
      (`~/Downloads/boorubox-vault/import-verify-library`) importing
      `database-part1of18.db` of the owner's export: 200 imported, the trash and grid counts
      match the part's `isDeleted` split, a re-run reports 200 skipped, `rating:e` and one tag
      count match the part, thumbnails render, timing noted here.
      Agent (2026-09-11, lead): part 1 → report "Imported — 200"; library 200 rows, 26 with
      `deleted_at` (part: 26 `isDeleted`), 21 `rating = e` (part: 21), 200 files under
      `images/<a1>/<b2>/`, 200 thumbnails; counts line "174 images total, 174 imported from
      the extension" (200 − 26 trashed), Trash badge 26. Re-run of part 1 → "Skipped — 200,
      Imported — 0", the library band shows the same report. Grid: thumbnails render; sidebar
      `blue_archive 52`, `E 1` equal the database's live counts; `rating:e` finds that one
      image. Timing: the debug binary decodes at ~4.3 s per image (200 in ~14 min, one core);
      the release binary imported part 2 (200 rows) in 36 s, ~0.18 s per image, so the whole
      3,569-row export is ~11 min and 25,000 images ~75 min. The cost is the full decode in
      `ingest::decode` plus the thumbnail decode (design D8); the dev profile does not
      optimise dependencies.
      Hand check: the owner opens `/import` and reads the report and notice.
