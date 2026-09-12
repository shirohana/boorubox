> Two implementing agents, disjoint file sets, able to run side by side. **A** owns
> `packages/app/src-tauri/**`; **B** owns `packages/shared/src/**` and
> `packages/app/src/**`. Group 3 is the lead's.
>
> The contract they share, pinned here so neither waits on the other:
> - Commands `import_pause`, `import_resume`, `import_cancel`. Each takes only
>   `State<'_, AppState>`, takes no argument, answers `Result<()>`, and is a silent no-op
>   when no import is running.
> - `ImportReport` gains `cancelled: bool` (serde camelCase leaves it `cancelled`), default
>   `false`. TypeScript: `cancelled: boolean` on `ImportReport` in `packages/shared`.
> - No new event. The webview knows the run is paused because it pressed Pause; Rust emits
>   `import:progress` exactly as it does today and stops emitting while parked.
> - B's wrappers in `src/lib/api/commands.ts`: `importPause()`, `importResume()`,
>   `importCancel()`, each `Promise<void>` invoking the command of the same name in snake
>   case with no argument.

## 1. Rust: the control handle, the checkpoints, the commands

- [x] 1.1 `packages/app/src-tauri/src/import.rs`: `ImportControl` per design D1 — a
      `Mutex<ControlState>` (`Running | Paused | Cancelled`) and a `Condvar`, with
      `pause()`, `resume()`, `cancel()`, and one method called between items that parks
      while paused and reports whether the run should carry on. Verify with tests that do
      not depend on timing luck: a control cancelled before the first check stops
      immediately; `resume()` wakes a thread parked in the check (assert the thread finishes,
      with a channel or a join, never a sleep-and-hope); a cancel delivered while parked
      wakes it and stops it.
- [x] 1.2 `import.rs`: `import_paths` takes `&ImportControl` and calls the checkpoint after
      `on_progress`, before the next candidate (design D2). The in-flight item finishes and
      is counted. Verify: a run cancelled after the first item reports exactly one item and
      leaves exactly one image in the library; `report.cancelled` is true; the files after
      the stop are neither imported nor reported.
- [x] 1.3 `packages/app/src-tauri/src/bundle.rs`: `import_bundle` takes the same
      `&ImportControl` and calls the same checkpoint in the same place. Verify on the
      existing fixture: a run cancelled after the first row reports one item and
      `cancelled: true`; re-running the same part afterwards skips what landed and imports
      the rest (this is the resume the spec promises, so assert it end to end).
- [x] 1.4 `packages/app/src-tauri/src/model.rs`: `ImportReport.cancelled: bool`, defaulting
      false, set by whatever returns a stopped run's report. Every existing construction site
      keeps compiling unchanged (`..Default::default()` or the field spelled out).
- [x] 1.5 `packages/app/src-tauri/src/lib.rs`: `AppState.import_control:
      Mutex<Option<Arc<ImportControl>>>` (design D4). Fresh handle installed when a run
      starts, slot cleared when it returns — including when it returns early or errors.
- [x] 1.6 `packages/app/src-tauri/src/commands.rs`: `import_pause`, `import_resume`,
      `import_cancel` as pinned above, locking `import_control` and never the library
      (design D5). `import_paths` and `import_bundle` install and clear the handle. Verify:
      the three commands are no-ops with nothing running; a cancel issued while a run holds
      the library returns without blocking (assert it returns while an import is mid-run,
      which is the whole point of D5); the two import commands register in the same
      `generate_handler!` list as the rest.
- [x] 1.7 Closing a library cancels a running import (design D7), so a paused run cannot
      hold the app open. Verify: with a run parked in the checkpoint, tearing the library
      down wakes it and the thread ends.
- [x] 1.8 `cargo test` and `cargo clippy --all-targets -- -D warnings` pass.

## 2. The webview: controls, the queue, and the two different re-runs

- [x] 2.1 `packages/shared/src/index.ts`: `cancelled: boolean` on `ImportReport`.
- [x] 2.2 `packages/app/src/lib/api/commands.ts`: `importPause`, `importResume`,
      `importCancel` wrappers as pinned above, with tests alongside the existing command
      tests.
- [x] 2.3 `packages/app/src/lib/api/imports.svelte.ts`: `pause()`, `resume()` and `cancel()`
      on `Imports`, and a paused flag the band can read. `cancel()` calls the command and
      then drops every run still `queued`, counting them (design D6). Verify with tests:
      cancelling with two runs queued leaves neither queued nor running and reports two
      discarded; pausing does not start a queued run; resuming after a pause does not
      double-start the running one. Remember vitest compiles runes server-side where
      `$state` does not proxy — never compare a `$state` field against an object literal.
- [x] 2.4 The pending-work band: Pause and Cancel on the running tile, Resume while paused,
      Cancel only on a waiting tile. A paused tile says it is paused rather than looking
      stalled. Derive anything read off a wholesale-reassigned store object with `$derived`,
      not an `$effect`.
- [x] 2.5 A cancelled run's result says it was cancelled, is not styled as an error, and
      carries the right sentence for its kind: a bundle result says selecting the same parts
      again carries on from where it stopped; a local result says importing the same files
      again imports them again, and offers no Resume. Verify both wordings with tests.
- [x] 2.6 `pnpm -r test`, `pnpm typecheck` and `pnpm lint` pass.

## 3. Lead

- [x] 3.1 Review each agent's unit before the other's files are touched; `mise run check`
      green on the merged result.
- [ ] 3.2 Hand check: cancel a real bundle import part-way in the running app, confirm the
      band shows a cancelled result with honest counts, then re-select the same parts and
      confirm the run resumes rather than duplicating. Left for the owner — an agent that
      cannot drive the Windows box or the owner's library does not tick this.
- [x] 3.3 Commit, leaving `main` releasable; do not tag.

## Handoff (agent B, group 2)

Done: 2.1–2.6, plus the lead's post-review fixes below. Gate green (`pnpm -r test` 403/403
app + 1 shared + 93 extension, `pnpm typecheck`, `pnpm lint`, all clean). Not committed —
lead's call per 3.3.

- `packages/shared/src/index.ts`: `ImportReport.cancelled: boolean`, as pinned in the header.
- `packages/app/src/lib/api/commands.ts`: `importPause`/`importResume`/`importCancel`, each
  `invoke('import_*')` with no args, matching the pinned contract. Tests in `commands.test.ts`.
- `packages/app/src/lib/api/imports.svelte.ts`:
  - `Imports.paused: boolean` — one flag, not per-run, since only the run in front can ever be
    parked (design D1/D4: one control handle). `pause()`/`resume()` call the command and flip
    it; `#execute`'s `finally` also clears it, so a run that ends on its own (finished, failed,
    or cancelled) never leaves a stale "paused" behind for the next run.
  - **Split into `cancel()` (running tile: stop the run, drop the whole queue) and
    `dequeue(id)` (waiting tile: drop only that run)** — the lead's call on the ambiguity
    flagged in the first pass. `dequeue` never touches `#queuedDiscarded` or calls a command;
    nothing has started for a queued run. `ImportRunTile.svelte` and the `/import` route branch
    on `run.status`/`bundleRun.status` to call the right one. Spec updated:
    `specs/pending-work/spec.md`'s "A running import can be paused and cancelled" now says a
    waiting tile's Cancel removes only itself, with a new scenario, and "Cancel discards the
    runs queued behind it" now says explicitly that's the *running* tile's Cancel.
  - **`#queuedDiscarded` now resets in `#execute`'s `finally`, not just the success path** — a
    cancelled run whose command rejects no longer leaks its count onto the next, unrelated
    run's report. Same `finally` also clears the two request latches below.
  - **Latched replay for Cancel/Pause pressed while Rust's control slot is empty**: the window
    between issuing `import_paths`/`import_bundle` and Rust actually installing the handle
    (design D4) — which recurs between every two queued runs even though the tile already reads
    `running` — made a Cancel or Pause pressed right then a silent no-op. `#cancelRequested`/
    `#pauseRequested` latch the request and replay it once on the run's first `import:progress`
    tick, which can only fire after the handle exists (design D2: the checkpoint that would see
    it runs after `on_progress`). Both latches are cleared in `finally` too, so a request that
    never got a tick to replay on (the run ended some other way first) can't bleed into the
    next run.
  - Introduced `ImportReportEntry` (extends shared `ImportReport` with `kind` and
    `queuedDiscarded`), webview-only, exported from `imports.svelte.ts` and re-exported from
    `$lib/api`. Rust's `ImportReport` doesn't gain these two fields — the run kind and the
    queue are both webview-only concepts (design D6), so extending the shared type would leak
    UI state into the IPC contract. `Imports.reports` and `.latestBundleReport` are now
    `ImportReportEntry[]` / `ImportReportEntry | null`; `ImportReportCard.svelte` and the
    `/import` route were updated to match.
- `packages/app/src/lib/components/import/cancelled-import.ts` (new): `rerunNotice(kind)` (the
  two different re-run sentences — bundle resumes, local duplicates) and
  `queuedDiscardedNotice(count)`, both pure and tested in `cancelled-import.test.ts`. Used by
  `ImportReportCard.svelte` (both kinds) and the `/import` route's progress panel and report
  panel (bundle only).
- `packages/app/src/lib/components/library/ImportRunTile.svelte`: running tile shows
  Pause + Cancel; paused shows "Paused" (+ Resume + Cancel, no progress bar since Rust stops
  emitting `import:progress` while parked, per the header's "no new event"); queued tile shows
  Cancel (now `dequeue`, not `cancel`) only. Reads `imports.paused` directly (a primitive
  `$state` field, not a nested field off a wholesale-reassigned object) via `$derived`, per the
  repo's `$effect` rule. `ImportReportCard.svelte`'s discarded-queue line is now gated on
  `report.cancelled` — an ordinary report can no longer claim a discard that belongs to a
  different, cancelled run.
- `routes/import/+page.svelte`: now has its own Pause/Resume/Cancel row and "Paused" state,
  gated on `bundleRun.status === 'running'` (a `queued` bundle run reflects `imports.paused`
  belonging to a different, running paths run otherwise — same trap `ImportRunTile` avoids) and
  surfaces `queuedDiscarded` in its report panel, gated on `cancelled` the same way.
- No `src-tauri/**` files touched.

## Handoff (agent A, group 1)

Done: 1.1–1.8, gate green (`cargo fmt`, `cargo test` 436/436, `cargo clippy --all-targets -- -D
warnings`, all clean). Not committed — lead's call per 3.3.

- `import.rs`: `ImportControl { state: Mutex<ControlState>, woken: Condvar }`,
  `ControlState::{Running,Paused,Cancelled}`. `pause()`/`resume()` are no-ops outside their
  expected starting state (`pause` on anything but `Running`, `resume` on anything but
  `Paused`) — in particular `pause()` never resurrects an already-`Cancelled` run.
  `checkpoint()` loops on the condvar so a spurious wake finds it still `Paused` and waits
  again. `import_paths` calls it once per iteration, right after `on_progress`, right before
  looping to the next candidate (design D2); on `false` it sets `report.cancelled = true` and
  `break`s, so the in-flight item's outcome is already counted by the time the checkpoint runs.
- `bundle.rs`: `import_bundle` takes `&ImportControl` too and calls the identical checkpoint in
  the identical place, both after a failed-part item and after each row — a `'parts:` loop
  label lets one `break 'parts` exit both the row loop and the part loop from either site.
  Resuming a cancelled bundle run needed no new code: `import_row`'s existing "id already in
  the library → skip, no decode" path is exactly the resume mechanism, so a re-run just
  re-scans every row from the top and the already-landed prefix comes back cheap and skipped.
- `model.rs`: `ImportReport.cancelled: bool` with `#[serde(default)]`; `derive(Default)` already
  gives every existing `ImportReport::default()`/`..Default::default()` site `false` for free —
  nothing else needed to change to keep compiling.
- `lib.rs`: `AppState.import_control: Mutex<Option<Arc<import::ImportControl>>>`, `None` in
  `Default`. `import_pause`/`import_resume`/`import_cancel` added to `generate_handler!`
  alongside `import_paths`/`import_bundle`.
- `commands.rs`: `install_import_control`/`clear_import_control` helpers wrap `import_paths`'s
  and `import_bundle`'s body — installed before `off_main_thread(...).await`, cleared
  unconditionally after (the `.await`'s `Result` is captured to a `let`, not returned early with
  `?`, specifically so the clear always runs — success, per-item failure, refusal, or a closed-
  library `Err` alike). `import_pause`/`import_resume`/`import_cancel` are plain sync `fn`s (no
  `async`, no `<R: Runtime>`) — they only ever lock `state.import_control`, clone the `Arc` out,
  and drop the guard before calling the control's own method, so they can never queue up behind
  the library mutex a running import holds (design D5).
- `close_library` (design D7): cancels whatever's in `import_control` before clearing
  `state.library`, waking a paused run so it can't hold the app open. Order between the cancel
  and the library-clear doesn't matter for correctness — a woken run breaks at the checkpoint
  and never touches the library again — but cancel-then-clear is what's there.
- Deviation from a literal reading of 1.6: that task's verification bullet says "the two import
  commands register in the same `generate_handler!` list as the rest" — `import_paths`/
  `import_bundle` were already registered before this change, so I read that as asking to
  confirm the *three new* commands land in the same list, which they do; noting the reading in
  case the intent was different.
- No `packages/shared/**` or `packages/app/src/**` files touched.
- Hand check: none owed from this group — 3.2 is explicitly the owner's.

### Addendum: review fixes (still group 1)

- **Blocker fixed.** `open_into_state` (the path `open_library`/`pick_library` and the library
  menu's "switch to" take) now calls the same `cancel_running_import` helper `close_library`
  uses, right after the new library opens and before `state.library` is swapped. A failed open
  leaves a running import alone — the cancel only runs once the new library is actually about
  to replace the old one. New test: `commands::tests::switching_the_library_wakes_a_paused_import_and_ends_it`
  pauses a run, switches libraries mid-run, and asserts both that the run ends with an honest
  one-item report and that the library it got switched to is left empty — proving it stopped
  rather than silently continuing into the new folder.
- **Should-fix fixed.** Added `import::is_last_item(report, total)`, shared by `import_paths` and
  `import_bundle`; both now skip the checkpoint call entirely once every item `total` promised
  has been counted, so a pause or cancel landing in that last instant neither parks a finished
  run nor marks its report `cancelled`. New tests:
  `import::tests::cancelling_exactly_when_the_last_item_finishes_does_not_mark_the_run_cancelled`
  and bundle.rs's row-loop equivalent.
- **Note addressed.** `install_import_control` now `debug_assert!`s the slot is `None` before
  installing; `clear_import_control` takes the `Arc` it installed and clears the slot only if it
  still holds that same handle (`Arc::ptr_eq`), so a future second caller can't silently steal or
  erase another run's control. No behavior change under the current single-caller invariant —
  covered indirectly by every existing pause/resume/cancel test still passing.
- Gate re-run clean: `cargo fmt`, `cargo test` (439/439), `cargo clippy --all-targets -- -D
  warnings`.
