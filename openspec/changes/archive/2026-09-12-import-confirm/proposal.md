## Why

The bundle import route starts the run the moment the file dialog closes. The owner ran the
real migration on Windows and hit the two consequences: a mis-picked part is already
importing before it can be read back, and a picker that quietly returned nothing — or a part
that will not open at all — is indistinguishable from one that is about to import 3,569 rows.
The migration is the one operation in the app where the user is meant to *check* before
committing (§9 step 4, §2 guarantee 4), and it is the only one that commits without asking.

Two smaller gaps from the same run: a finished bundle report has no way off the screen, so
the route never returns to the state it starts in; and `import-pause-cancel` made a library
switch cancel a running import silently — correct, but a 25,000-row migration is now thrown
away by a menu item the user reached for to do something else.

## What Changes

- **Picking bundle parts no longer imports.** The `/import` route shows the parts it will
  read, in run order, each with the number of rows it holds or the reason it could not be
  opened, and the total. An Import button starts the run; a Discard button throws the pick
  away (§9 step 3: the user reads before committing).
- Those row counts come from a **new Rust command** that runs the same part-planning step
  `import_bundle` already runs before its first row (`legacy-bundle-import` design D5), so
  the number on the confirm screen is the number the run will report as its total.
- **Local folder and file import stays immediate.** Dropping a folder on the window, or
  picking one from the import menu, imports as it does today (§6 "local file import").
  Dropping is already the decision.
- **Done on a finished bundle report** clears it and returns the route to its idle state. The
  counts and the §9 notice below it stay, as the spec requires, because they are what the
  browser is compared against and they outlive any one run.
- **A library switch that would cancel a running import asks first.** Every path that swaps
  the open library — the library menu's Switch to, Choose folder and Close library, and the
  start screen's entries and picker — asks when an import is running or queued, says the run
  will be cancelled, and says what re-running costs for that kind of run. Declining leaves
  everything running.

## Capabilities

### New Capabilities

None. This changes when three existing capabilities commit, not what the product does.

### Modified Capabilities

- `legacy-bundle-import`: picking parts shows what will be imported and waits for a second
  confirmation; a finished report is dismissed by the user.
- `library-switching`: a switch that would stop an import in flight is confirmed first.

## Non-goals

- **Changing when a local import commits.** A drop is an act the user aimed at the window;
  putting a dialog in front of it would make the common case slower to serve the rare one.
- **Rolling anything back.** A cancelled or declined run still keeps what it imported
  (`import-pause-cancel` design D3). Nothing here undoes an import.
- **Reading bundle rows before the run.** The confirm step opens each part and counts its
  rows; it does not decode an image, read a tag or build a preview. Anything beyond the count
  costs the same minutes the import itself costs (`legacy-bundle-import` design D8).
- **Revisiting `update-check` design D6.** An update is still refused outright while work is
  in flight, with no confirmation offered. That refusal protects an unattended restart; this
  change's confirmation protects a deliberate switch. They are different questions.
- **Making the pick survive a relaunch.** A pending pick is state of the session, not of the
  library.

## Impact

- `packages/app/src-tauri/src/bundle.rs` — the part-planning step becomes a named function
  with its own return type, called by both `import_bundle` and the new command.
- `packages/app/src-tauri/src/commands.rs`, `lib.rs` — one new command in the handler list.
- `packages/shared/src/index.ts` — the planned-part record the command answers with.
- `packages/app/src/lib/api/` — a wrapper for the command, a store holding the pending pick,
  and a way to clear the latest bundle report.
- `packages/app/src/routes/import/+page.svelte` — the confirm step, Import, Discard, Done.
- `packages/app/src/lib/components/frame/LibraryMenu.svelte`,
  `packages/app/src/routes/start/+page.svelte`,
  `packages/app/src/lib/components/import/cancelled-import.ts` — the switch confirmation and
  its wording.

No new dependencies. No schema change: nothing here touches the database.
