## Context

See proposal.md for motivation. The state this design fits into:

- A capture's only wait is the image download (proposal, Non-goals). The extension's
  `captureAndDeliver` (`background/deliver.ts`) obtains the bytes first, then asks the page for
  its context, then mints the entry's id and `capturedAt`, writes the entry, and posts. The app
  hears nothing until the multipart `POST /captures` arrives with the bytes.
- `POST /captures` (`http/captures.rs`) answers 201 with the record for a new row, 200 for a
  known id, 4xx/5xx for a refusal, and calls `HttpState.on_stored` only on 201; `lib.rs` turns
  that into the `capture:stored` Tauri event (bridge-extension D16). The layout re-reads
  `LibraryStatus` on it and the library route re-runs its search.
- `Imports` (`components/library/imports.svelte.ts`) is constructed by the library route and
  runs one `import_paths` command at a time: `run` returns early and silently while `running`
  is true. `ImportMenu.svelte` disables the Import button during a run and shows the progress
  as `text-xs text-muted-foreground` in the toolbar. Drag-and-drop is subscribed by the route
  and not gated by the button, so a second drop during a run does nothing at all. Leaving the
  route mid-run unmounts the instance; the command keeps going but nothing shows it.
- `import:progress` (design D12 of Phase 1) is emitted per file with `done`, `total`,
  `imported`, `skipped`, `failed` and carries no run id. Rust counts the files before the first
  tick.
- The grid (`LibraryGrid.svelte`) is virtual over `results.total`: `gridWindow` turns the
  viewport width and the tile setting into columns and a row height, and every index in it is
  a search index the inspector, the lightbox and the keyboard map all share. Tiles are square.
- `capture:stored`'s payload is an `ImageRecord`, whose `width`/`height` come from decoding the
  bytes; the extension's `CaptureMeta` has neither.
- The app has store tests (`*.svelte.test.ts` under `lib/api`) and pure-module tests, no
  component tests. shadcn-svelte copy-ins are added with `pnpm dlx shadcn-svelte add <name>`;
  `skeleton` is in, `progress` is not.

## Goals / Non-Goals

**Goals:**

- The first visible change happens on the click, not on the bytes. Nothing in this design may
  put a network round trip, a decode or a search between the action and its placeholder.
- One surface, two kinds of work. A pending capture and an import run are tiles in the same
  band, shaped by the same tile setting, so "something is happening to my library" has one
  place to look.
- Nothing pending is ever mistaken for library content — by the grid, by a count, by a search,
  or by a later reader of the database.
- No new state on the Rust side and no new persistence. The transport gains two relays; policy
  about what pending work is and how long it is shown stays in the webview (CLAUDE.md:
  extraction in the extension, policy in the app, Rust owns storage).

**Non-Goals:**

- Feedback inside the extension beyond what it has (badge, notification, popup): the owner
  watches the app.
- Progress for a single capture (bytes downloaded of total). The extension's fetch does not
  expose it, and a one-second wait wants presence, not a bar.
- A cancel on either kind of tile (proposal, Non-goals).

## Decisions

**D1. A pending capture is announced by the extension and held as pending work, never as a
library row.** The owner closed this against a two-phase `POST /captures` that would create a
row before its bytes: `width` and `height` are unknown until the bytes are decoded and the grid
lays tiles out on them, so the row could not even be sized; and a browser closed mid-upload
would leave a permanent hole in the library that no user action put there. A row also leaks
into everything that reads `images` — counts, search, `GET /status`, `legacy-bundle-import`'s
reconciliation numbers. Pending work is a webview set (D3) and the database never hears of it.

**D2. Two new listener routes, both relays: `POST /captures/pending` and
`DELETE /captures/pending/{id}`.** The announcement's body is the `CaptureMeta` the extension
will send as the `meta` part — `id`, `imageUrl`, `pageUrl`, `pageTitle`, `capturedAt`, adapter
record — exactly, so there is no second type to mirror and the tile can name the site and the
page. The handler validates the JSON (an `id` is required, as for `meta`), refuses with 503
when no library is open (the capture it announces would get 503 too, and a tile for it would
promise an image that cannot arrive), stores nothing, and answers 202 after calling the event
callback. The withdrawal takes an optional `{ reason }` body, answers 204 whether or not the id
was ever announced — the app keeps no set to check against, and an idempotent 204 is what a
client retrying a lost response wants — and emits. The origin middleware already wraps every
route, so both are extension-only without new code. Rejected: a query parameter or a header for
the announcement (the meta is JSON already, and the same shape twice is one serde struct); a
single `POST /captures/pending` with a `status` field (a withdrawal is a different verb on the
same resource and reads as one).

**D3. The webview holds the pending set; Rust holds nothing.** `lib/api/pending.svelte.ts`
exports one `pendingCaptures` store, subscribed once by the root layout for the life of the
window, exactly as the layout already subscribes `capture:stored` for the sidebar count. It
adds on `capture:pending`, removes on `capture:stored` and `capture:withdrawn` by id, and
exposes the entries newest first. The layout subscribes because the library route mounts and
unmounts (a capture announced on `/settings` must be on the band when the user comes back) and
because a Tauri event emitted before any listener exists is simply lost. Rejected: a Rust-side
map with a `pending_captures` command to read it. It would buy one case — the webview reloaded
mid-capture, which happens in dev under HMR and nowhere else — at the price of a second copy of
the set, its own expiry, a lock, and a command; pending work lives for seconds and is UI state
by definition.

**D4. An announcement expires after 120 s if nothing settles it.** The extension cannot always
keep its promise: a worker killed during the download has no history entry yet (the entry is
written after the bytes, bridge-extension D4), so its startup sweep has no id to withdraw; a
browser quit mid-capture sends nothing at all. The store therefore times each entry out. The
bound has to outlast the slowest honest capture — the extension's own POST timeout is 30 s, and
a large Pixiv original on a slow line can take tens of seconds to download before it — while
not leaving a ghost tile for minutes; 120 s is the assumption, in one constant with this
argument beside it. A capture that does arrive after the bound still lands normally: the
`capture:stored` event refreshes the grid whether or not a tile was waiting.

**D5. The extension announces after asking the page and before fetching, and awaits the
announcement with a short timeout.** Order in `captureAndDeliver` becomes: mint `id` and
`capturedAt`, ask the page for its context (a DOM read, milliseconds, and it gives the
announcement the page's own title per bridge-extension D17), announce, then obtain the bytes,
then write the entry and post. The announcement is awaited, not fired and forgotten: the app
emits `capture:pending` before it answers, so an awaited 202 guarantees the pending event
precedes the stored one — un-awaited, a fast POST could overtake a slow announcement and leave a
tile that only the expiry clears. It is awaited under its own 2 s timeout, separate from the
30 s delivery timeout, and every outcome is ignored: the app not running is the ordinary
failure here, and the capture path already handles it when the POST fails. `capturedAt` moves
from "when the bytes arrived" to "when the user clicked", which is what the field means. A
retry never announces: the bytes are in hand and the stored event follows within the request.

**D6. `POST /captures` settles every readable id, through one callback with three variants.**
`HttpState.on_stored: Fn(&ImageRecord)` becomes `on_event: Fn(CaptureEvent)` with
`Pending(CaptureMeta)`, `Withdrawn(CaptureWithdrawn)` and `Stored(ImageRecord)`; `lib.rs` maps
each to its event name and the test recorder collects them. `captures.rs` emits `Stored` on a
new row (as today), and `Withdrawn` for an existing row (no reason) and for any refusal that
came after the meta was parsed (the refusal's own message as the reason) — so the tile clears
on the same answer the extension gets, and the extension needs no second request to clear it.
A request that failed before the id was known has nothing to settle. Rejected: making the
extension `DELETE` after a failed POST (cannot cover the app being the one that refused before
the extension reads the answer, and doubles the requests on the failure path); a separate
`capture:settled` event (three names for three facts is simpler than two names and a flag).

**D7. The band is its own component above the grid, not rows inside it.** `PendingBand.svelte`
renders above the scroll viewport (and above `EmptyState`, and above the search's "nothing
found"), only while it has something, and lays its tiles out with the same `gridWindow` module
the grid uses — same `EDGE`, same `GAP`, columns from the same width and tile — so its tiles
line up with the grid's first row and read as "the row that is about to exist". It sits outside
the scroll container, so it stays visible while the user scrolls. Rejected: ghost rows inside
`LibraryGrid`. Every index in the grid is a search index shared with the inspector, the
lightbox and the keyboard map; prepending rows would shift them all, and a placeholder inside a
filtered search would claim to match a query it has not been tested against. Rejected: a toast
or the sidebar. The owner watches the grid; a toast is where the eye is not.

**D8. Two tiles.** `PendingCaptureTile` is a square at the grid's tile size with a `skeleton`
pulse, the site name from the adapter record when there is one, and the page title truncated —
no image: loading `imageUrl` in the webview would be a second download of the same bytes, and
on a hotlink-protected host a failing one. `ImportRunTile` shows the run's state: "Looking
through what you dropped…" until the first tick, then a `progress` bar with `done` of `total`
and the imported count; a queued run shows "Waiting" and the number of dropped or chosen items
(paths, not files: the files are not counted until the run starts).

**D9. `Imports` moves to `lib/api/imports.svelte.ts` and becomes a singleton with a FIFO
queue.** `enqueue(paths)` appends a run `{ id, paths, status: 'queued' | 'running', progress }`
and starts the pump if it is idle; runs go one at a time, in order, each subscribing to
`import:progress` before its command as today. Sequential on purpose and in the webview on
purpose: `import_paths` serialises on the library mutex anyway, `import:progress` carries no run
id so two concurrent runs would interleave counts, and the brief's "no transport change" holds.
A singleton because a run must outlive the route (going to settings mid-import today loses the
only reference to it) and because the drop handler, the menu and the band all read one queue.
The route registers `imports.onfinished(listener)` — returning an unsubscribe, called in an
`$effect` — to refresh its search; the constructor callback goes. `run`'s silent early return
goes with it: `enqueue` with no paths is the one no-op, and it is not silent about a running
import because there is nothing to be silent about.

**D10. Reports become a list.** Each finished run's `ImportReport` is kept, newest first, until
dismissed one at a time; `ImportReportCard` renders the list. One slot replaced by the next run
would lose the first run's skipped-and-failed list — the only place those files are ever named
(app-shell D8) — the moment the second run finished.

**D11. The Import button is never disabled and the toolbar progress line is deleted.** The
owner's reading: once a run is queued the button must be available again, because imports
queue. The progress line beside the button was real feedback nobody saw; keeping it beside the
band would be the same number in two places.

**D12. No failed tile.** A withdrawn or refused capture's tile disappears. The reason travels
in `capture:withdrawn` (D6) because Rust has it for free, but rendering it is a second failure
surface competing with the extension's notification, popup entry and badge, which
`capture-delivery` already specifies and §2 guarantee 3 already rests on. If the owner wants the
reason in the app it is a tile state on top of this event, not a transport change.

**D13. No command waits for the library on the main thread, and nothing holds the library
longer than one image.** Found in the owner's manual pass (7.1): with three captures in flight
the window stopped scrolling. Tauri runs a synchronous command on the app's main thread, and on
macOS that thread is also the window's run loop; `search`, `thumbnail_path`, `library_status`,
`image_counts` and `drop_image_record` were all synchronous and all took the library mutex
through `with_library`, while a capture's `store_image` held that mutex on a blocking thread
for decode, write, fsync, insert and thumbnail (70–380 ms per image by the owner's earlier
numbers). Each `capture:stored` sent the webview straight back to `search`, which parked the
main thread behind the lock, and the webview could not repaint. `import_paths` was worse: it
took the lock once for the whole run, so a scroll that needed a thumbnail froze the window
until the last file was in — the archived `local-file-import` scenario "Progress on a large
drop" ("the UI stays responsive") was already violated, unseen, because the Import button used
to be disabled and nothing invited the user to touch the grid mid-run.

Three moves, in order of effect. First, one helper — `spawn_blocking` around `with_library`,
the shape `captures::store` already uses and argues for — and every command that touches the
library becomes `async fn` on it, so the main thread never waits on the mutex. Second,
`import::import_paths` takes the shared library and locks per file inside its loop, so a read
slips in between two files; one machine still writes at a time (§7), one file at a time.
Third, thumbnail generation leaves the lock in `store_image` and `thumbnail_path`: it needs
only the image path and the record, and it is the largest single piece of the hold. Rejected:
`#[tauri::command(async)]` on the synchronous functions alone — it moves the wait to a runtime
worker, which is better than the main thread but still parks an async thread on a
`std::sync::Mutex`, the exact hazard `captures::store` documents.

## Risks / Trade-offs

- [A tile vanishes on `capture:stored` a few milliseconds before the refreshed search puts the
  image in the grid] → accepted; the refresh is one local search command. Holding the tile until
  the route's refresh resolves would couple the layout-owned store to the route-owned results.
- [The webview reloaded mid-capture loses the set (D3)] → the image still arrives and
  `capture:stored` still refreshes; only the placeholder is missed, and only under dev HMR.
- [A capture slower than 120 s (D4) shows a tile that expires before the image lands] → the
  stored event still refreshes the grid; the user sees the image arrive without its
  placeholder, which is today's behaviour, not a regression.
- [The announcement's 2 s timeout (D5) adds up to 2 s to a capture while the app is wedged] →
  a wedged app also fails the 30 s POST; the capture ends `failed` and retryable either way.
- [Reordering `askThePage` before the bytes (D5) changes the tested control flow of
  `captureAndDeliver`] → the existing tests for "no content script still delivers" and "record
  reaches the meta on both paths" pin the invariants; the order test is added beside them.
- [Two agents touch `events.ts`-shaped contracts on both sides] → the event names and the
  `CaptureWithdrawn` type land first (tasks 1.1), and both agents build on that commit.

## Migration Plan

No schema, setting or stored-data change. The extension and the app ship together as they do
today; an older extension against this app simply never announces and captures land as before,
and this extension against an older app gets a 404 on the announcement, ignores it (D5), and
delivers as before.
