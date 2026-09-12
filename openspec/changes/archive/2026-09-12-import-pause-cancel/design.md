## Context

Both importers already have the shape this change needs. `import::import_paths` and
`bundle::import_bundle` walk their work one item at a time, call a progress callback after
each, and take the library lock *inside* the item rather than around the loop (design D13).
So there is a point between two items where the run holds no lock and can be stopped
cheaply. Nothing has to be restructured; a check has to be added at that point.

The queue that holds runs waiting behind the running one lives in the webview
(`Imports` in `packages/app/src/lib/api/imports.svelte.ts`), not in Rust. Rust knows about
one run at a time.

## Goals / Non-Goals

**Goals**
- Stop a run within one item, from the UI, without quitting the app.
- Hold a run and continue it.
- A cancelled run reports what it did, as a report, not an error.

**Non-Goals**
- Rolling back what a cancelled run imported. Every item is committed as it goes and stays.
- Stopping mid-item. An item is the unit; `store_image` is what makes it atomic.
- Resuming a *local* import. Fresh ids each run make that a different feature, not this one.
- A pause that survives quitting the app.

## Decisions

### D1: A per-run control handle, parked on a condvar

`ImportControl` holds a `Mutex<ControlState>` (`Running | Paused | Cancelled`) and a
`Condvar`. Between items the run calls one method that returns whether to carry on, and
that method *blocks* on the condvar while the state is `Paused`.

A condvar rather than an `AtomicBool` polled in a sleep loop: a paused import must cost
nothing while it waits, and must wake the instant Resume is pressed rather than at the end
of a poll interval. An `AtomicBool` would be enough for cancel alone, and is not enough for
pause.

Parking the thread is safe precisely because of where the check sits: between items the run
holds no library lock, so a paused import blocks nothing else. Parking *inside* an item
would freeze every command behind the library mutex.

### D2: One checkpoint, shared by both importers

Both `import_paths` and `import_bundle` take `&ImportControl` and call the same method at
the same place — after `on_progress`, before the next item. Two importers with their own
idea of when a run may stop would drift the first time one of them grows a step.

The in-flight item finishes and is counted. A cancel observed between items therefore always
leaves the report and the library agreeing with each other, with no half-imported item
anywhere.

### D3: `cancelled` is a field on the report, not an `Err`

`ImportReport` gains `cancelled: bool`. The command still returns `Ok`. A cancelled run has
a real result the user asked to see — how far it got — and `AppError` carries no counts. The
existing rule that per-item failures are outcomes rather than errors already points this
way: `Err` is reserved for a run that could not start.

### D4: The handle lives in `AppState`, one slot, cleared when the run ends

`AppState` gains `import_control: Mutex<Option<Arc<ImportControl>>>`. The import command
installs a fresh handle before it starts and clears it when it returns, so:

- a Cancel pressed after a run has finished cancels nothing rather than the next run,
- the pause/resume/cancel commands are a no-op when nothing is running.

A handle per run, never one reused across runs, is what makes that true without the UI
having to time anything.

### D5: The control commands never touch the library mutex

`import_pause`, `import_resume` and `import_cancel` lock only `import_control`. This is the
whole point: the library mutex is held (per item) by the very run being cancelled, so a
cancel that waited for it would wait for the run it is trying to stop. A Cancel during a
25,000-row import has to be answered while that import is holding the library.

### D6: The webview owns the queue, so cancel clears it there

Rust cancels the run it knows about. `Imports.cancel()` calls the command and then drops
every `queued` run, counting them for the report it shows. Moving the queue into Rust to
make this one call is a larger change than the behaviour is worth, and the queue exists for
UI reasons (a drop during a run) that the backend has no opinion about.

### D7: A closing library cancels a running import

Closing the library or quitting with an import paused must not leave a parked thread holding
the app open. Whatever tears a library down cancels the control handle first, which wakes the
parked thread, which returns its report and exits. Without this, Pause is a way to hang the
app on quit.

## Risks / Trade-offs

- **A paused run holds no lock but does hold its place.** Queued imports wait behind a paused
  one indefinitely. Accepted, and specified: the band says it is paused, so the state is
  visible rather than mysterious. The alternative — starting a queued run while another is
  paused — would interleave two runs' progress events, which carry no run id.
- **Cancel is cooperative.** An item that itself takes a long time (one very large image)
  delays the stop by that item. Acceptable: the worst case is one decode, and the alternative
  is killing a thread mid-write.
- **`cancelled` on the report is a new field** that every existing consumer of `ImportReport`
  ignores safely, but the TypeScript type is shared, so `packages/shared` and both runtimes
  move together.
