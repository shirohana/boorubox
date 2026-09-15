> Depends on `library-folder`, `library-switching`, `app-shell`, archived. Two agents, serial:
> **R** owns group 1 (Rust, shared, `api/commands.ts`, `api/events.ts`), **T** owns group 2
> (webview). Gate per group in its tasks; `mise run check` for the change.

## 1. The launch path and the setting (agent R)

- [x] 1.1 `settings.rs` + `commands.rs` + `lib.rs` + shared + `api/commands.ts`:
      `open_last_on_launch` (default true, serde default), `set_open_last_on_launch`,
      `setOpenLastOnLaunch` (design D3). Verify: a settings test that an old file without the
      field reads as `true`, that the setter persists; typecheck passes.
- [x] 1.2 `lib.rs` + `AppState` + `model.rs` (`LibraryStatus.opening`) + `commands.rs`
      (`library_status`) + `api/events.ts` (`onLibraryOpened`): the launch open on a blocking
      thread with the flag and the event, skipped when the setting is off (design D1). Verify:
      a command test that `library_status` reports `opening` while the flag is set and `opened`
      false; the existing launch tests (missing, damaged, newer paths) still pass; `cargo test`,
      clippy, fmt, typecheck.
- [x] 1.3 Measure per design D4 on a copy of the owner's 25k vault
      (`~/Downloads/boorubox-vault/test-1` is the dev library; the 25k one is named in the
      backlog memory — if unavailable, measure on the largest library present and say so) and
      write the numbers into `design.md` Risks. Verify: the numbers are in the file.

## 2. The screen and the switch (agent T)

- [ ] 2.1 `components/common/OpeningScreen.svelte`, `routes/+layout.svelte` (the gate and the
      subscription), `api/library.svelte.ts` if it needs a field (design D2);
      `routes/settings/+page.svelte` (the switch, design D3). Verify: typecheck, lint, tests.
      Hand check: launch with the 25k vault remembered — the window appears at once naming the
      folder, then the grid; turn the switch off, relaunch — the start screen with the recent
      list, nothing reported missing, pick the vault — it opens; turn it on, relaunch — opens by
      itself; rename the remembered folder away, relaunch — the opening screen, then the start
      screen naming the missing path.

## 3. Change-level verification (owner)

- [ ] 3.1 `mise run check` green; the hand check passes on Windows.

## Handoff

Group 1 (agent R) is done; gate green (`cargo test` 565 passed/1 ignored, `cargo clippy
--all-targets -- -D warnings`, `cargo fmt`, `pnpm -r typecheck`, `pnpm lint`). What T needs
for group 2:

- **Status field:** `LibraryStatus.opening: string | null` (Rust `Option<String>`) —
  the remembered path while the launch-time open runs, `null`/`None` once it settles either
  way (success or failure). `opened` stays `false` for the whole time `opening` is set, except
  for a narrow, harmless window right at the end where `open_into_state` has already set the
  library (so a `library_status` poll could in principle read `opened: true` and `opening` still
  non-null for one poll) — the gate never treats that as a bug worth avoiding, since the layout
  only has to pick `opening` over the frame/start-screen when set, and `opened` over `opening`
  otherwise; either reading lands on the library UI.
- **Setting:** `AppSettings.openLastOnLaunch: boolean`, read via `appSettings()` /
  `app_settings`, written via `setOpenLastOnLaunch(value)` → `set_open_last_on_launch`
  (Rust `{ value: bool }`). Default `true`; an old settings file with no such key reads as
  `true`.
- **Event:** `LIBRARY_OPENED_EVENT = 'library:opened'`, wrapper `onLibraryOpened(handler: () =>
  void)` in `packages/app/src/lib/api/events.ts`. No payload — re-read `libraryStatus()` on
  it, the same as any other status-changing signal.
- **Deviations from the brief:**
  - `open_remembered_library_and_listen` now delegates the blocking-thread work to a new
    private `spawn_launch_open(handle, path)` in `lib.rs` (not named in the brief) — kept
    small and named rather than inlined, per repo `CLAUDE.md`'s "name the block".
  - Typecheck failures forced edits outside the stated file ownership: `AppSettings` gained a
    required field, which broke hand-written literals in
    `packages/app/src/lib/api/commands.test.ts` and `packages/app/src/lib/api/settings.svelte.test.ts`
    (added `openLastOnLaunch` to each, plus one new test each for the setter/event, mirroring
    the `notesCollapsed` precedent already in those files) and `packages/app/src/lib/api/events.test.ts`
    (one new test for `onLibraryOpened`). No file under `lib/components/**` was touched.
  - Task 1.3: the owner's 25,000-image vault was not present under
    `~/Downloads/boorubox-vault/` this run. Per the coordinator's scope correction, measured
    only inside that folder and the repo, on a scratch copy (never the owner's live folders,
    never `find`/`mdfind` outside those two paths). The largest library actually present was
    `import-verify-library` (400 files / 351 live rows), not `test-1` — numbers are in
    `design.md` Risks against that one, release profile: `open_existing` 5.8ms,
    `open_into_state` 3.4ms. Both are far under a second at this size, so D1 alone likely
    explains the black window at this scale, but the number does not speak for a vault ~70x
    larger — `design.md` says so and gives the re-run command
    (`commands.rs::tests::measure_launch_open_time`, `#[ignore]`, reads
    `LAUNCH_SCREEN_MEASURE_DIR`) for whoever has the real 25k vault.
- **Not touched:** `packages/app/src/lib/components/**` (T2's concurrent edits — left
  exactly as found), `routes/+layout.svelte`, `routes/settings/+page.svelte`,
  `api/library.svelte.ts`, `OpeningScreen.svelte` — all group 2.

### Group 2 (agent T) — done, gate green

`pnpm -r typecheck`, `pnpm lint`, `pnpm --filter @boorubox/app test` (515 passed) all green.
Landed: `components/common/OpeningScreen.svelte` (new — one component for both the pre-answer
screen and the named-folder screen, `path` prop optional); `routes/+layout.svelte` (the two new
gate branches, the redirect guard's `!opening` addition); `routes/settings/+page.svelte` (the
switch under Library, wired through a new `settings.setOpenLastOnLaunch`); `api/settings.svelte.ts`
+ its test (the missing wrapper method — R's handoff had the command in `commands.ts` but no
webview-store member yet, so this was still needed despite the field already being on
`AppSettings`).

- **The race (design D2):** `library.load()` no longer fires at the top of `<script>`. It now
  fires inside the same `$effect` that subscribes to `onLibraryOpened`, chained after the
  subscription's promise resolves (`onLibraryOpened(...).then(() => library.load())`). The
  subscription is registered — and only then is the status asked for — so a launch-time open
  that settles and emits between "subscribe" and "read status" cannot happen: the emit can only
  land after the listener exists. `settings.load()` stays at the top level; only `library.load()`
  needed moving, since `library:opened` is the only signal the opening screen depends on.
- **`api/library.svelte.ts` needed no member:** `LibraryStatus.opening` is read directly off
  `library.status` through a `$derived` in the layout (`repo CLAUDE.md`'s rule against reading a
  store field inside an `$effect`), the same way `libraryOpen` already was. No new method or
  field earned its place on the class.
- **Gate ordering in the opening screen (design D2):** `library.status.opening` is checked ahead
  of `onStart`/`libraryOpen` in the render gate. The handoff's noted race window (`opened: true`
  read for one poll before `opening` clears) means this ordering can show the opening screen one
  poll longer than strictly needed at the very end of an open, never the reverse — both readings
  still land on the library UI, so this is not treated as a bug.
- **Deviations:** none from the brief's file list. The `Switch` uses `aria-label` rather than a
  paired `<Label for>`, since the row already carries a visible heading (`<p>`) that is not an
  `<input>`'s label in the HTML sense — pairing them would have needed a second, redundant
  screen-reader-only copy of the same text.
- **Not touched:** `packages/app/src/lib/components/**` (left exactly as found through a
  concurrent edit mid-session — a `LibraryGrid.svelte`/`ImageCard.svelte` typecheck and lint
  failure appeared and cleared on its own between two gate runs, from that other agent's work,
  not group 2's).
