## Why

The owner's ask of 2026-09-12, kept as the last of the queue on 2026-09-15: with the 25,000-image
vault remembered, the window shows black at launch while the library opens. The cause is in
`lib.rs`: the remembered library is opened inside Tauri's `setup` hook, on the main thread,
before the window has painted anything; the webview's "Opening your library…" line exists but
cannot be drawn until `setup` returns. The owner wants both: a loading screen while the vault
opens, and the choice to land on the start screen and pick a vault instead — "a select-vault
screen at launch, plus a Settings option 'open the recent vault by default'". §6 says the folder
is "user-picked on first launch, remembered"; the spec `library-folder` says it "SHALL reopen the
last library on launch without asking". That stays the default; it becomes a setting.

**Depends on:** `library-folder`, `library-switching` (the start screen already lists recent
libraries, which is the picker), `app-shell` (the layout's gating), archived.

## What Changes

- **The remembered library opens off the main thread.** `setup` returns at once; the window
  paints; the library's status says it is opening, and the webview shows a screen naming the
  folder until the open settles, then the library UI or the start screen as today.
- **A setting, "Open the last library at launch", on by default.** Off, the app starts on the
  start screen with the recent list, nothing is opened until the user picks one, and nothing
  is reported as missing. The remembered path is kept either way.
- No change to what opening does, to the listener (still bound at launch), or to the start
  screen's own actions.

## Non-goals

- Making the open itself faster (measure first: the design records what is measured).
- A progress bar for the open: nothing inside it reports progress today.
- Changing the recent list's shape or the start screen's layout.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `library-folder`: "Remembered library opens on launch" — the open no longer blocks the
  window, a screen names the folder while it runs, and the automatic open is a setting.

## Impact

- `packages/app/src-tauri`: `lib.rs` (`setup`), `AppState` (an opening flag), `LibraryStatus`
  (`opening`), one event, `Settings` (`open_last_on_launch`) with its command; the shared
  mirrors; `api/events.ts`, `api/library.svelte.ts`, `routes/+layout.svelte`,
  `routes/settings/+page.svelte`.
