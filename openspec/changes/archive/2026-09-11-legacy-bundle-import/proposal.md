## Why

Users of the legacy browser extension need their images out of it and into the app before
that extension can be retired (docs/requirements.md §9). The route is the extension's export
bundle, and the app has to import it with a report the user can check against the browser
before deleting anything (§2 guarantee 4).

This was split out of `phase-1-app-mvp` and held open, gated on the legacy repo shipping
Phase 0 — the export that would add tag rules, notes and settings to today's bundle. The gate
is lifted without Phase 0 (design D4): the legacy repo has not moved since its requirements
doc, the images are the migration, and the app already has its own rules import and notes.
What the legacy extension exports today — version `1.0`: SQLite parts of 200 rows plus a
manifest — is the format this change pins.

## What Changes

- **Legacy bundle import** (§8 Phase 0, §9): read the export bundle's SQLite parts and map
  their rows onto the schema the app ships, with a per-item report (imported / skipped /
  failed + reason). Trashed items stay trashed.
- `ingest::store_image` accepts a deletion time, so a trashed bundle row goes through the
  same door as everything else (design D3).
- An `/import` route that picks the bundle's part files, shows progress and the report, then
  the total and legacy-bundle counts with the §9 notice text.

## Capabilities

### New Capabilities

- `legacy-bundle-import`: importing the legacy extension's export bundle with a per-item
  report.

### Modified Capabilities

None. The storage schema already models `source=legacy-bundle` and `deleted_at`.

## Non-goals

- Changing the bundle format. It is defined by the legacy repo and consumed here.
- Importing tag rules, notes or settings. A later bundle version may carry them (§8 Phase 0);
  this change reads only the `images` table and ignores anything else in the folder.
- Deleting anything from the browser. The app never claims the browser is safe to clear
  (§9 step 4).

## Impact

- `packages/app/src-tauri`: one bundle-reading module (`bundle.rs`) and one Tauri command,
  both feeding the existing `ingest::store_image` write path; `IngestInput.deleted_at`.
- `packages/app/src`: the `/import` route, a sidebar entry, and a second kind of run in the
  existing import queue.
- `packages/app/src-tauri/fixtures/legacy-bundle/`: four real rows of the owner's export,
  one with a corrupted blob, checked in as the format's fixture (design D2).
- Depends on `sharded-image-dirs` landing first: both edit `ingest.rs`, and the bulk import
  should land in the final folder layout.
