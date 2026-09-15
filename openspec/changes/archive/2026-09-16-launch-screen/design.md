## Context

See proposal.md — Why. `open_remembered_library_and_listen(app)` runs in `setup`: it loads the
settings, calls `commands::open_into_state(…, ExistingOnly)` for the remembered path (which
opens the database, relayouts flat files if any, sweeps the inbox, grants the asset scope,
cancels a running import, stores the library and spawns the sidecar backfill), then
`block_on(rebind_listener)`. `LibraryStatus` has `opened`, `library_path` and the three
failure paths; the layout renders "Opening your library…" only while `library.status` is
still `null`, i.e. before `library_status` has answered once. Events go through `app.emit`
and `api/events.ts` `listen` wrappers; the layout already subscribes to three.

## Goals / Non-Goals

**Goals:** the window paints before the library opens; the webview knows the three states
(opening, opened, not opened) from one status; the setting is one field read in one place.

**Non-Goals:** progress inside the open; changing `open_into_state`.

## Decisions

### D1. The launch open runs on a blocking thread; the status says `opening`

`AppState.launch_opening: Mutex<Option<PathBuf>>` — `Some(path)` while the launch open runs.
`setup`: load settings; if a path is remembered and `open_last_on_launch`, set the flag and
`tauri::async_runtime::spawn_blocking` the `open_into_state` call, clearing the flag when it
returns (either way) and emitting `library:opened` (no payload — the webview re-reads the
status, which is the one truth). The listener rebind stays in `setup` as today (it is quick
and the extension's first capture should find the port bound). `library_status` reports
`opening: Option<String>` (the path) so the screen can name the folder; `opened` stays false
while opening. Every other reader of `state.library` already tolerates `None`.

*Alternative rejected:* showing the window late (`visible: false` until setup returns). That
hides the delay rather than the window; the owner wants to see the app is alive.

### D2. The webview: one more state in the layout's gate

`library.status.opening` → an `OpeningScreen` naming the folder (`libraryName(path)`, the
sidebar's helper), drawn instead of the frame or the start screen; the existing pre-answer
line becomes the same component with no name. The layout subscribes to `onLibraryOpened` and
calls `library.refresh()`. Because the status is re-read rather than carried in the event, a
capture arriving during the open, or a failure, is reported by the same path as today.

### D3. The setting

`Settings.open_last_on_launch: bool`, default `true` (`#[serde(default = "…")]` so an existing
settings file reads as on), `set_open_last_on_launch(value)` command and wrapper, a `Switch`
on /settings under the library section labelled "Open the last library at launch", with a line
saying it takes effect at the next launch. Off: `setup` skips the open, sets no flag, and the
status is the plain not-opened one — the start screen then shows the recent list, which is
the picker the owner asked for and already exists (`library-switching`).

### D4. What is slow is measured, not guessed

Task 1.3 times `Library::open_existing` and `open_into_state` on the owner's 25k vault copy
and writes the numbers into this file under Risks. If the open itself is under a second, the
black window was the unpainted webview alone and D1 is the whole fix; if not, the number says
where the next change looks (`grant_asset_scope`, the backfill's first query, the FTS).

## Risks / Trade-offs

- [A command arrives while the launch open holds the library mutex] → commands already wait on
  the same mutex; the webview shows the opening screen and issues none of them until the
  status says opened.
- [The open fails after the window is up] → the flag clears, the event fires, the status names
  `missing_path` / `damaged_path` / `newer_path` as today, and the start screen shows the reason.
- [Task 1.3 measurement] The owner's 25,000-image vault was not present under
  `~/Downloads/boorubox-vault/` for this run (only `test-1` at 58 images, `test-2` at 8,
  `test-3` at 200, and `import-verify-library` at 400 files / 351 live rows were there); the
  largest of those, `import-verify-library`, was copied to a scratch directory (never the
  owner's live folders) and timed there instead, release profile, on this machine:
  `Library::open_existing` 5.8ms, `open_into_state` 3.4ms for 351 images. Both are far under a
  second, so — at this size — the black window was the unpainted webview alone and D1 is the
  whole fix; the number says nothing about a 25,000-image vault, which is roughly 70x larger,
  and should be re-measured against one before this risk is called closed. Re-run with
  `LAUNCH_SCREEN_MEASURE_DIR=<path> cargo test --release -- --ignored --nocapture
  measure_launch_open_time` from `packages/app/src-tauri` (`commands.rs`,
  `measure_launch_open_time`).
- [Measured open time on the 25k vault: to be filled by task 1.3.]
