## Context

See proposal.md — Why. Three things about the code as it stands shape the approach:

- `bundle::import_bundle` already does the work a confirm step needs, and does it before it
  writes anything: `sorted_parts` puts the picked files in run order, `open_part` opens each
  one read-only and reads `COUNT(*)` or records why it could not, and `PartPlan::work` sums
  those into the run's `total` (`legacy-bundle-import` design D5). The confirm screen wants
  exactly that list and exactly that number.
- The `/import` route already survives being left and returned to, because the run and its
  report live in the `Imports` singleton rather than in the route (design D7 and D9).
- `import-pause-cancel` cancels a running import whenever the library is torn down or
  replaced — `close_library` and `open_into_state` both call `cancel_running_import` (design
  D7). That is the backstop this change puts a question in front of. Rust's cancel stops the
  run it knows about; the queue of waiting runs is in the webview (design D6), and nothing
  stops it today.

## Goals / Non-Goals

**Goals**

- The number on the confirm screen and the number the run reports are the same number,
  computed once.
- Nothing about the confirm step changes what a started run does. The run is the run that
  ships today.
- One place decides whether a library swap may proceed, so a future swap path cannot forget
  to ask.

**Non-Goals**

- A preview of the images, or any per-row reading before the run. Opening a part and counting
  its rows is metadata; decoding is the cost (`legacy-bundle-import` design D8).
- Holding the opened part connections between the confirm step and the run. See D3.
- Any change to how `import_paths`, the drop target or the import menu commit.

## Decisions

### D1: The confirm step is bundle-only; a drop still imports on release

A dropped folder and an import-menu pick keep importing immediately. A bundle pick does not.

The difference is not how much work each starts — a dropped folder can be larger than a
bundle — it is what the user was doing when they started it. A drop lands on the window
the user aimed at, with the files they dragged; there is nothing to read back that they did
not just hold in their hand. A bundle pick is a multi-select dialog over files named
`database-part7of18.db`, whose contents nobody can see and whose row counts nobody knows —
and picking them is step 3 of the one flow the requirements tell the user to *check* (§9
step 4, §2 guarantee 4). A dialog in front of the drop would cost the daily case a click to
serve a case that has no ambiguity in it.

Alternative considered: a setting, "confirm before importing". Rejected — it makes the
question a preference when it is a property of the two flows, and the answer is already
decided by which of them the user is in.

### D2: The row counts come from Rust, from the planning step the run itself runs

The webview cannot count rows in a SQLite part: `bundle.rs` is the only place that knows the
legacy schema (`legacy-bundle-import` design D2), the webview has no SQLite, and it has no
filesystem access to those paths beyond the strings the picker handed it.

But "Rust because the webview cannot" is the weaker half. The stronger half is that a count
computed any other way would be a *second* answer to "how much work is this", and the confirm
screen's whole job is to be the number the run will honour. Reading `COUNT(*)` through the
same `open_part` the run uses means a part the confirm screen says is unopenable is the same
part that will be one `failed` item, with the same reason text.

### D3: The planning step is extracted, not copied, and the run re-plans

`sorted_parts` + `open_part` + `PartPlan::work` become one named function over
`&[PathBuf]`. `import_bundle` calls it where it inlines those three today; the new command
calls it, maps each plan to a serializable record, and drops the connections it opened.

So each part is opened twice for a confirmed import: once to count, once to read. That is
deliberate, and the alternative — parking the confirm step's open connections in `AppState`
for the run to adopt — is rejected twice over. It would hold read handles on the user's
bundle for as long as a confirm screen sits unanswered, and it would make the run's `total`
depend on state from an earlier command rather than on the files as they are when it starts.
A second open costs one `COUNT(*)` per part.

The consequence is worth stating plainly for the next reader: **the preview is advisory**. If
a part is moved, replaced or unplugged between the preview and Import, the run reports it, not
the preview. That is the honest direction — the report is the record (§9 step 3), and it is
built from what the run actually found.

### D4: The pending pick is a store, not route state

The picked files and their plans live in a singleton beside `Imports`, not in
`/import/+page.svelte`.

Two reasons, in order of weight. First, it is the only shape a test can reach: the confirm
step's real behaviour is "picking plans and does not enqueue", "Import enqueues exactly what
was planned, in that order", "Discard enqueues nothing" — all statements about a store,
provable in vitest, where the same logic inside a route's `<script>` is provable only by
hand. Second, it matches what the route already does with the run and the report: leaving
for settings and coming back must not lose a plan that took seconds to read off a network
drive.

The store holds the picked paths, the planned parts, whether a plan is in flight, and its
error; it exposes planning a pick, confirming it (which calls `imports.enqueueBundle` and
clears itself), and discarding it. It does not own the run — `Imports` still does.

### D5: Done clears the route's latest report, and nothing else

`Imports` already keeps two things: `reports`, the band's list, dismissed one at a time
(`pending-work` design D10), and `latestBundleReport`, the one report the `/import` route
reads (`legacy-bundle-import` design D7). Done clears the second only.

Clearing both would delete a card from the library band that the user may not have seen, on
a screen that never showed it — the band and the route are two audiences for one run, and
dismissing on one is not dismissing on the other. `dismissAll()` stays what it is: the
library-switch teardown, where every report describes a folder that is no longer open.

The counts block below stays rendered whether or not a report is up; it already does, for the
reason its comment gives, and the spec ("Counts shown") requires it.

### D6: One function is asked before any library swap, and confirming cancels from the webview

Every path that swaps the library goes through the same helper: it answers whether to ask,
and with what words, from `imports.runs`. The paths are the library menu's Switch to, Choose
folder and Close library, and the start screen's entries and its picker.

Confirming cancels from the webview *before* the swap command runs, rather than leaving the
swap to Rust's `cancel_running_import`. This matters beyond tidiness: Rust only knows the run
in flight, so today a switch confirmed with two runs queued would cancel the first and then
let the webview pump start the second **against the newly opened library**. The dialog
promises those waiting runs are discarded; cancelling from the webview is what makes the
promise true (`import-pause-cancel` design D6 — the queue is the webview's).

It also waits for the cancelled run to settle before swapping, which `cancel()` alone does
not: `cancel()` resolves when Rust has taken the signal, while the run's own report arrives
one item later. Without the wait, that report is added to the band *after* the library path
changed — after the sidebar's `dismissAll()` has run — leaving a card describing the old
library on the new one's screen. The wait is bounded by exactly what the cancel spec already
promises, one item, and it is what makes "the reports describe runs into the library that was
open" stay true. So `Imports` gains a cancel that resolves once its queue is empty, and the
swap paths use that one.

The words come from `cancelled-import.ts`, which already owns the two re-run sentences and
the discarded-queue sentence, so the dialog and the cancelled report cannot drift into
different claims about the same run. The dialog is the existing `ConfirmDialog` — a fifth
caller, and the bar for one is high by that component's own comment, but this question meets
it: the act destroys work in flight and cannot be undone by clicking again.

On the start screen the helper is asked and, under today's flow, always answers "no question"
— closing a library cancels its import before the redirect lands. It is wired there anyway
because the helper *is* the swap path, not as a guard against a case that exists: leaving one
door unwired is how the next door gets built without one.

### D7: Rust's cancel on a library swap stays exactly as it is

`close_library` and `open_into_state` keep calling `cancel_running_import`. The dialog is a
question asked in the webview; the backstop protects every path that does not pass through
it — startup restoring a remembered library, and anything added later. A confirmation is not
a mechanism.

### D8: `update-check` design D6 is untouched

An update still refuses outright while an import is running, saying what is in flight, with
no "cancel it and update anyway" offered. The two questions look alike and are not: an update
restarts the app unattended and the user cannot weigh what they would lose, while a library
switch is a deliberate act the user can price once the dialog tells them what it costs.
Nothing in this change edits `update-work.ts` or `UpdateDialog.svelte`.

## Risks / Trade-offs

- **A confirm step is a step the migration did not have** → It is one button on a flow run
  once or twice in a library's life, and it is the flow the requirements ask the user to check
  before deleting anything. The daily path (drop, menu) is untouched by D1.
- **Planning opens every part before the import does** → One `COUNT(*)` per part on a
  read-only connection that is then dropped. Measured against a run that decodes every image,
  it is not on the scale that matters; a bundle on a slow network drive pays it twice, and the
  screen says it is reading while it does.
- **The preview can go stale** (D3) → Accepted and specified: the run's report is the record,
  and it names what it actually found.
- **A fifth `ConfirmDialog` caller erodes the "confirmations get dismissed unread" argument**
  → It is the only one that stands in front of work in flight rather than in front of data at
  rest, and it appears only when there is a run to lose.
- **Confirming a switch now waits for the run to stop** (D6) → Bounded by one item, which is
  the bound the cancel spec already sets. The worst case is one large image's decode between
  the click and the new library appearing, and the confirming button says the import is being
  stopped while it waits rather than looking stuck.
