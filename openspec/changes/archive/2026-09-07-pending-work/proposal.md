## Why

The app shows nothing between the action that adds a picture and the picture appearing. A
capture from the browser waits on the image download — about a second, and unavoidable, since
§5's transport needs the bytes before `POST /captures` can start — and a local import can run
for minutes; through both the grid stays exactly as it was. The owner cannot tell that a
right-click landed, and ran one import twice because nothing said the first was going. The
answer is feedback, not speed: the app should show the pending thing the moment it is asked
for, where the result will appear.

## What Changes

- **A work-in-flight band on the library screen.** Above the grid's first row, at the grid's
  tile size, one placeholder tile per capture the app has been told is coming and one tile per
  import run — running with its progress, or queued with what it holds. It is there whether the
  library is empty or full, and it stays while the user visits another screen and comes back.
- **The extension announces a capture before it fetches the bytes** (§4, §5): a new
  `POST /captures/pending` carrying the same JSON it will later send as the `meta` part, and a
  `DELETE /captures/pending/{id}` when neither capture route produced bytes. The app stores
  nothing for either; it relays them to the open window as events, and every answer it gives to
  `POST /captures` settles the announcement — a new row as `capture:stored`, anything else as
  withdrawn. A pending capture is held as pending work, never as a library row.
- **Imports queue** (§6, local file import). An import started while one is running — a drop,
  or the menu — is queued and runs after it, in order, each with its own report. The Import
  button is no longer disabled during a run, and a drop during a run no longer does nothing:
  this is a **change to existing behaviour**, not only an addition.
- **The toolbar's progress line goes.** The band is where progress lives; a second copy beside
  the Import button is the one the owner never saw.

## Capabilities

### New Capabilities

- `pending-work`: the library screen shows what is on its way — captures announced but not yet
  stored, imports running or waiting — immediately, in the place the result will land, without
  any of it counting as library content until it is stored.

### Modified Capabilities

- `capture-ingest`: two new listener routes (announce a coming capture, withdraw it), the window
  told of both, and every answer to `POST /captures` settling the announcement. The existing
  "A stored capture reaches the open window" requirement is restated so that "announced" there
  means the stored announcement and a request that stored nothing is settled as withdrawn.
- `capture-delivery`: the extension tells the app a capture is coming before it obtains the
  bytes, proceeds whether or not the app answered, withdraws when both routes fail, and never
  announces a retry.
- `local-file-import`: imports started during a run are queued, never refused or silently
  dropped, and the controls that start one stay available.

## Non-goals

- **Making a capture faster.** The wait is the download of the site's own bytes (measured
  2026-09-07: 867 ms for 0.8 MB); on X the page-canvas route is dead because the image host
  taints the canvas, so the background fetch is the only route and there is nothing left to
  optimise on it. Do not re-measure or re-open.
- **Deduplication, of imports or captures.** One artist may upload the same picture more than
  once, so duplicates are legitimate library content; `import_file` minting a fresh id per file
  stays. The legacy extension's answer was a *show duplicates* filter, and that would be its own
  later change.
- **A failed tile in the app.** Failure belongs to the extension (`capture-delivery`: the
  notification, the popup entry with its reason, the badge). The band shows only what is coming;
  a withdrawn capture's tile goes. The withdrawn event carries the reason anyway, so showing it
  later needs no transport change.
- **Pending work outside the library screen** — no sidebar count, nothing on `/settings`.
- **Cancelling a queued or running import.** A running one cannot stop without a transport
  change; a queued one is visible before the next drop, which is the mistake this change exists
  to prevent.
- **Persisting pending captures in Rust or on disk.** They live seconds; see design D3.

## Impact

- `packages/shared` + `packages/app/src-tauri/src/model.rs` (hand-mirrored, one commit):
  `CaptureWithdrawn { id, reason? }`. `CaptureMeta` is reused as the announcement body — no new
  type for it.
- `packages/app/src-tauri`: new `http/pending.rs` with the two handlers; `HttpState.on_stored`
  becomes one `on_event` callback over a `CaptureEvent` enum (pending / withdrawn / stored);
  `http/captures.rs` settles on every answer; `lib.rs` maps the three variants to
  `capture:pending`, `capture:withdrawn`, `capture:stored`. No schema change, no migration.
- `packages/extension`: `background/deliver.ts` mints the id at the click, asks the page first,
  announces, then captures; `delivery/client.ts` gains the announce and withdraw calls;
  `settings.ts` the endpoint. The existing `http://127.0.0.1/*` host permission covers it.
- `packages/app/src`: `lib/api/pending.svelte.ts` (the pending set, subscribed by the layout),
  `lib/api/imports.svelte.ts` (moved out of `components/library`, now a singleton with a queue),
  two subscriptions in `lib/api/events.ts`, new `components/library/PendingBand.svelte` with its
  two tile components, `ImportMenu.svelte` loses `disabled` and the progress line,
  `ImportReportCard.svelte` shows a list, `routes/+page.svelte` and `routes/+layout.svelte` wire
  them. One new shadcn-svelte copy-in: `progress`.
