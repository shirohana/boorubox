> Two implementing agents by file set. **A** owns `packages/app/src-tauri/**` and
> `packages/shared/src/**`; **B** owns `packages/app/src/**`. Group 3 is the lead's.
>
> **A runs first.** B's gate includes `pnpm typecheck`, and the type it compiles against is
> A's — group 2 cannot be green until group 1 has landed. The groups do not otherwise
> overlap, so B needs nothing from A but that file.
>
> The contract they share, pinned here so neither has to read the other's code:
> - Command `bundle_plan`, argument `files: Vec<String>` (the absolute paths the picker
>   returned, in any order), answers `Result<BundlePlan>`. It needs **no open library** and
>   writes nothing. It emits no event.
> - `BundlePlan { parts: Vec<BundlePartPlan>, total: u32 }` and
>   `BundlePartPlan { path: String, rows: Option<i64>, error: Option<String> }`, serde
>   camelCase, in `model.rs` beside `ImportReport`. `parts` is in the order the run will read
>   them, deduplicated, exactly as `import_bundle` orders them. Exactly one of `rows` and
>   `error` is `Some` on each part. `total` is the run's `total` — the same number
>   `import:progress` will carry — computed in Rust so the webview never has to know that an
>   unopenable part is worth one item.
> - TypeScript, in `packages/shared/src/index.ts`:
>   `interface BundlePartPlan { path: string, rows: number | null, error: string | null }` and
>   `interface BundlePlan { parts: BundlePartPlan[], total: number }`.
> - B's wrapper in `src/lib/api/commands.ts`: `bundlePlan(files: string[]): Promise<BundlePlan>`,
>   invoking `bundle_plan` with `{ files }`.
> - Nothing in this change touches `import_paths`, `import_bundle`'s row loop, the
>   `ImportControl` commands, `update-work.ts` or `UpdateDialog.svelte` (design D8).
>
> **Handoff to B.** Group 1 is landed as specified, no deviations. `bundle_plan` is registered
> in `lib.rs`'s `generate_handler!` next to `import_bundle`. `BundlePlan`/`BundlePartPlan` are
> exported from `packages/shared/src/index.ts` beside `ImportReport`, exactly the shape pinned
> above (`rows`/`error` as `number | null` / `string | null`, not optional — both keys are
> always present in the JSON, never omitted, since `serde` only skips them from `Some`/`None`
> on the Rust struct, not from the wire shape B's `commands.ts` wrapper sees). `cargo test`,
> `cargo clippy --all-targets -- -D warnings`, `pnpm --filter @boorubox/shared test` and
> `mise run lint` are all green on this half.

## 1. Rust and shared: the planning command

- [x] 1.1 `packages/app/src-tauri/src/bundle.rs`: extract the planning step `import_bundle`
      runs today — `sorted_parts` + `open_part` into `fn plan_parts(files: &[PathBuf]) ->
      Vec<PartPlan>`, and the `PartPlan::work` sum into `fn total_work(plans: &[PartPlan]) ->
      u32`. `import_bundle` calls both where it inlines them now, and its behaviour is
      unchanged (design D3: extraction, never a copy — two answers to "how much work is this"
      is the failure being designed out). Verify: every existing `bundle.rs` test still
      passes untouched.
- [x] 1.2 `packages/app/src-tauri/src/model.rs`: `BundlePlan` and `BundlePartPlan` exactly as
      pinned above, and `packages/app/src-tauri/src/bundle.rs`: `pub fn plan(files:
      &[PathBuf]) -> BundlePlan` built from `plan_parts` and `total_work`, mapping each
      `PartPlan` to its summary and dropping the connections it opened (design D3: the run
      re-plans; nothing is parked between the two commands). Verify on the existing fixture:
      the fixture part reports its real row count with `error: None`; a non-SQLite file among
      the picks reports `rows: None` with the same reason string `import_bundle` puts in that
      part's failed item; a file named twice appears once; parts come back in `part<N>` order
      whatever order they were passed in; `total` equals the `total` the first
      `import:progress` of a run over the same files carries (assert against a real run, not
      against a recomputed sum).
- [x] 1.3 `packages/app/src-tauri/src/commands.rs` + `lib.rs`: the `bundle_plan` command per
      the contract, registered in `generate_handler!`. Opening parts and counting rows is
      filesystem work, so it goes on a blocking thread — `tauri::async_runtime::spawn_blocking`
      directly, the way `booru_upload` reads its file, not `off_main_thread`, which exists to
      take the library this command never touches. Verify: the command answers with no
      library open; a plan over the fixture matches `bundle::plan` for the same files.
- [x] 1.4 `packages/shared/src/index.ts`: `BundlePartPlan` and `BundlePlan` as pinned,
      documented like their neighbours (what the record is for, not what the fields are
      named). Verify `pnpm --filter @boorubox/shared test` and the repo's lint pass.
- [x] 1.5 `cargo fmt`, `cargo test` and `cargo clippy --all-targets -- -D warnings` pass in
      `packages/app/src-tauri`.

## 2. Webview: the confirm step, Done, and the switch confirmation

- [x] 2.1 `packages/app/src/lib/api/commands.ts`: the `bundlePlan` wrapper per the contract,
      with the same mocked-invoke test its neighbours have in `commands.test.ts`.
- [x] 2.2 `packages/app/src/lib/api/bundle-pick.svelte.ts` (new, exported from `$lib/api`):
      the pending pick as a singleton (design D4) — the picked paths, the `BundlePlan`, a
      planning flag and an error; `pick(picker)` which plans what the picker returned and
      enqueues nothing; `confirm()` which calls `imports.enqueueBundle` with exactly the
      files it planned and clears itself; `discard()` which clears without enqueueing. A
      picker the user cancelled answers with no files and is not an error, and plans nothing.
      Verify with vitest: picking plans and does not enqueue; confirm enqueues once, with the
      picked files; discard enqueues nothing and leaves no pick; a failed `bundle_plan`
      leaves the error and no pick to confirm. Remember vitest compiles runes server-side —
      never compare a `$state` field against an object literal.
- [x] 2.3 `packages/app/src/lib/api/imports.svelte.ts`: `dismissBundleReport()`, clearing
      `latestBundleReport` and nothing else (design D5 — the band's `reports` list is a
      different audience), and `cancelAndSettle(): Promise<void>`, which cancels and resolves
      once `runs` is empty, resolving at once when it already is (design D6). Build it on the
      existing `onfinished` listener rather than a second notification path. Verify with
      vitest: `dismissBundleReport` leaves `reports` untouched; `cancelAndSettle` resolves
      only after the running run's report has landed, and resolves immediately with nothing
      running.
- [x] 2.4 `packages/app/src/lib/components/import/cancelled-import.ts`: the switch dialog's
      words — one pure function taking the running run's kind, how many runs wait behind it,
      and whether the library is being closed or swapped, answering the dialog's title,
      description and confirm label. It reuses `rerunNotice(kind)` for the re-run sentence,
      so the dialog and a cancelled report cannot claim different things about the same run
      (design D6); the queued sentence is this dialog's own, in the future tense the past-tense
      `queuedDiscardedNotice` cannot serve. Verify with vitest: both kinds' sentences; the
      queued clause present for one, for many, and absent for none; close and switch wordings.
- [x] 2.5 `packages/app/src/lib/api/library-switch.svelte.ts` (new, exported from `$lib/api`):
      the guard every swap path goes through (design D6) — `guard(action: () => Promise<void>)`
      runs `action` at once when `imports.runs` is empty, and otherwise parks it behind a
      pending question that snapshots the running run's kind and the number waiting, taken
      when the question is asked rather than read live while it is up. Confirming awaits
      `imports.cancelAndSettle()` and then runs the parked action; declining drops it and
      leaves the queue alone. Verify with vitest: nothing running runs the action with no
      question; a running import parks it; declining never runs it and leaves `runs` as they
      were; confirming cancels, empties the queue, and runs the action after — not before.
- [ ] 2.6 `packages/app/src/lib/components/frame/LibrarySwitchDialog.svelte` (new) and
      `packages/app/src/routes/+layout.svelte`: the dialog is the existing
      `$lib/components/common/ConfirmDialog.svelte`, fed by 2.4's words from the store's
      snapshot, mounted unconditionally in the layout the way `UpdateDialog` is — it must be
      reachable from `/start`, which renders outside the frame. It renders nothing while
      there is no pending question, and says the import is being stopped while
      `cancelAndSettle` is still waiting rather than looking stuck. `Hand check:` the dialog's
      appearance and wording in the running app.
- [x] 2.7 `packages/app/src/lib/components/frame/LibraryMenu.svelte` and
      `packages/app/src/routes/start/+page.svelte`: every action that swaps the library goes
      through `librarySwitch.guard` — the menu's Switch to, Choose folder and Close library,
      and the start screen's recent entries and its picker. The menu's existing `notes.flush()`
      stays where it is, inside the action the guard runs, so a declined question does not
      flush a note for a switch that did not happen. On the start screen the guard is expected
      to ask nothing under today's flow (design D6) and is wired anyway, for the reason given
      there.
- [ ] 2.8 `packages/app/src/routes/import/+page.svelte`: the confirm step. Picking no longer
      enqueues; the route shows `bundlePick`'s parts in run order with each part's row count
      or its open error, the plan's total, an Import button that confirms, and a Discard
      button. While a plan is being read the route says so. The running and paused states are
      unchanged. A finished report gains a Done button calling `imports.dismissBundleReport()`,
      after which the route is back to offering the picker with the counts block and the §9
      notice still below it. Derive anything read off a wholesale-reassigned store object with
      `$derived`, never an `$effect`. `Hand check:` pick two real bundle parts, read the
      counts, Discard, pick again, Import, then Done on the report.
- [x] 2.9 `pnpm -r test`, `pnpm typecheck` and `pnpm lint` pass.

> **Review (group 2).** Three deviations accepted as reported: `guard(action, run)` (the
> dialog's close/switch wording has to come from the caller), `LibrarySwitch`'s injectable
> `Imports`, and `/import` hiding the whole pick section while a report is up (task 2.8's
> "after which the route is back to offering the picker" says the same). Changed:
> `imports.svelte.ts` fires `onfinished` from `#pump` *after* the run leaves `runs`, not from
> `#execute`'s `finally` — `cancelAndSettle` resolved while the run it cancelled was still in
> the queue, and only microtask ordering kept a confirmed swap from running against a
> non-empty queue; `Imports#pickBundle` (and its `#pick` helper) deleted as the dead
> unconfirmed path — a picker that enqueues a bundle straight off is what this change removes.
> Tests added: a confirmed switch with two runs queued reads `imports.runs` *inside* the swap
> action and finds it empty, with only the first run ever started (spec "Confirming with work
> queued"), and `onfinished` fires with the finished run already off the queue.

## 3. Lead

- [x] 3.1 Review each group before the next agent starts; `mise run check` green on the
      merged result.
- [ ] 3.2 `Hand check:` on a real library — switch libraries mid-import from the menu,
      decline and confirm that the run is still going with its counts unbroken; confirm and
      check that the new library is open, the run's report says cancelled with honest counts,
      and no queued run started into the new folder. Left for the owner; an agent that cannot
      drive the owner's library does not tick this.
- [x] 3.3 Commit, leaving `main` releasable; do not tag.
