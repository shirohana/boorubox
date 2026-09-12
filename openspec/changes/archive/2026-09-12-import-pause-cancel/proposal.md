## Why

An import cannot be stopped. The owner is about to migrate 25,000+ images from the legacy
extension — roughly 75 minutes in a release build — and today the only way out of a run that
is going wrong is to quit the app. Quitting works, but it is indistinguishable from a crash:
no report, no count, and no way to tell the user what did land.

The same gap makes every smaller mistake expensive. Drop the wrong folder and it imports to
the end.

## What Changes

- A running import can be **paused** and **resumed**, and can be **cancelled**.
- A cancelled run produces its report, marked cancelled, with the real counts of what was
  imported, skipped and failed before it stopped. Cancelling is not an error.
- Everything already imported stays imported. Cancelling never rolls back.
- Cancel ends the current run **and discards runs queued behind it**; the report says how
  many were dropped. A user stopping a migration does not want the next batch to start.
- Pause holds the current run; queued runs do not start while it is paused.
- The UI says plainly that re-running a cancelled **bundle** import resumes where it stopped,
  while re-running a cancelled **local file** import imports everything again. That asymmetry
  is real (bundle rows keep their own ids, local files mint fresh ones) and today it is
  recorded only in a design doc.
- Pause and cancel take effect **between items**, never mid-item. The item in flight finishes
  and is counted.

## Capabilities

### New Capabilities

None. This changes how two existing import capabilities behave and what the pending-work
band offers; it introduces no new area of the product.

### Modified Capabilities

- `pending-work`: the band that shows an import in flight gains pause/resume and cancel
  controls, and shows a cancelled run's outcome.
- `local-file-import`: a run can be cancelled; a cancelled run reports what it imported; the
  user is told that re-running imports everything again.
- `legacy-bundle-import`: a run can be cancelled; a cancelled run reports what it imported;
  the user is told that re-running resumes.

## Impact

- `packages/app/src-tauri/src/import.rs`, `bundle.rs` — the per-item loops gain a check.
- `packages/app/src-tauri/src/lib.rs` — `AppState` gains the run's control handle.
- `packages/app/src-tauri/src/commands.rs` — new commands to pause, resume and cancel; the
  two import commands honour them.
- `packages/app/src-tauri/src/model.rs` — `ImportReport` gains a cancelled marker.
- `packages/shared/src/index.ts` — the report type follows.
- `packages/app/src/lib/api/imports.svelte.ts` and the pending-work band — controls and the
  queue's behaviour on cancel.

No new dependencies. No migration: nothing about this touches the database.
